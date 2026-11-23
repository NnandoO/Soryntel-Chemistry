//! Module 22 — Material Database (HSC Chemistry 10, Chapter 90, Figs. 1–3).
//!
//! Stores industrial materials (concentrates, fluxes, fuels, steels, slags, plastics, sludges,
//! and nested mixtures) defined by recursive `[SubMaterial]` + chemical species compositions,
//! physical/electrical/mechanical properties with `Auto Calculate` weighted estimation,
//! 6-term HSC polynomial `Cp Data` across temperature ranges, `Analyses`, `Data`, and
//! `User Properties`, plus JSON database persistence and CSV export.

use crate::species::{awdb_atomic_weight, parse_hsc_formula_elements};
use crate::{evaluate_species_si, ChemicalSpecies, SpeciesRepository};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// High-level material classification for catalog filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialCategory {
    Mixture,
    Concentrate,
    Flux,
    Fuel,
    Metal,
    Slag,
    Polymer,
    Sludge,
    Other,
}

impl MaterialCategory {
    pub const ALL: &'static [MaterialCategory] = &[
        MaterialCategory::Mixture,
        MaterialCategory::Concentrate,
        MaterialCategory::Flux,
        MaterialCategory::Fuel,
        MaterialCategory::Metal,
        MaterialCategory::Slag,
        MaterialCategory::Polymer,
        MaterialCategory::Sludge,
        MaterialCategory::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mixture => "Mixture",
            Self::Concentrate => "Concentrate",
            Self::Flux => "Flux",
            Self::Fuel => "Fuel",
            Self::Metal => "Metal",
            Self::Slag => "Slag",
            Self::Polymer => "Polymer",
            Self::Sludge => "Sludge",
            Self::Other => "Other",
        }
    }
}

/// One row in the left-hand `Composition` sheet (Fig. 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionEntry {
    /// Either a chemical species formula (e.g. `C`, `H2O`, `CuFeS2`) or a nested material
    /// wrapped in square brackets (e.g. `[Concentrate]`, `[Silica Flux]`, `[Steel]`).
    pub item: String,
    /// Mass fraction in wt-% (composition rows should sum to 100.000%).
    pub wt_pct: f64,
    pub reference: String,
}

impl CompositionEntry {
    pub fn new(item: impl Into<String>, wt_pct: f64) -> Self {
        Self {
            item: item.into(),
            wt_pct,
            reference: String::new(),
        }
    }

    pub fn submaterial_name(&self) -> Option<&str> {
        let t = self.item.trim();
        if t.starts_with('[') && t.ends_with(']') && t.len() >= 3 {
            Some(t[1..t.len() - 1].trim())
        } else {
            None
        }
    }
}

/// Laboratory / assay entry shown in the `Analyses` tab (Fig. 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisEntry {
    pub parameter: String,
    pub value: String,
    pub unit: String,
    pub method: String,
    pub reference: String,
}

/// One row in the `Properties` spreadsheet (Fig. 2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyField {
    pub value: String,
    pub reference: String,
    pub auto_calculate: bool,
}

impl PropertyField {
    pub fn manual(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            reference: String::new(),
            auto_calculate: false,
        }
    }

    pub fn auto_calc() -> Self {
        Self {
            value: String::new(),
            reference: String::new(),
            auto_calculate: true,
        }
    }
}

/// Complete standard physical property sheet matching Fig. 2 (`Heat & State`, `Electrical & Magnetic`, `Mechanical`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialProperties {
    // Heat & State
    pub state_at_25c: PropertyField,
    pub avg_molar_weight: PropertyField,
    pub density: PropertyField,
    pub melting_point: PropertyField,
    pub boiling_point: PropertyField,
    pub heat_capacity: PropertyField,
    pub enthalpy_25c: PropertyField,
    pub heat_of_fusion: PropertyField,
    pub heat_of_vaporization: PropertyField,
    // Electrical & Magnetic
    pub electrical_conductivity: PropertyField,
    pub mass_susceptibility: PropertyField,
    pub magnetic_permeability: PropertyField,
    // Mechanical
    pub hardness_hv: PropertyField,
    pub hardness_mohs: PropertyField,
    pub youngs_modulus: PropertyField,
}

/// One temperature range column in `Cp Data` (Fig. 3).
///
/// Polynomial form (J/(kg·K)):
/// `Cp(T) = A + B*T*1e-3 + C*T^-2*1e5 + D*T^2*1e-6 + E*T^-3*1e8 + F*T^3*1e-9`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialCpRange {
    pub t_min_k: f64,
    pub t_max_k: f64,
    pub phase: String,
    /// For range 1: `H(298.15 K)` in `kJ/kg`. For ranges >= 2: transition enthalpy `ΔH_tr` in `kJ/kg`.
    pub h_kj_kg: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    pub reference: String,
}

impl Default for MaterialCpRange {
    fn default() -> Self {
        Self {
            t_min_k: 298.15,
            t_max_k: 1000.0,
            phase: "s".into(),
            h_kj_kg: 0.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: String::new(),
        }
    }
}

impl MaterialCpRange {
    /// Evaluates specific heat capacity `Cp(T)` in `J/(kg·K)` using the exact 6-term HSC polynomial (Fig. 3).
    pub fn eval_cp_j_kg_k(&self, t_k: f64) -> f64 {
        let t = t_k.max(1.0);
        let t2 = t * t;
        let t3 = t2 * t;
        self.a
            + self.b * t * 1e-3
            + self.c * 1e5 / t2
            + self.d * t2 * 1e-6
            + self.e * 1e8 / t3
            + self.f * t3 * 1e-9
    }

    /// Analytically integrates `Cp(T) dT` from `t1` to `t2` (K) and returns `ΔH` in `kJ/kg`.
    pub fn integrate_h_kj_kg(&self, t1: f64, t2: f64) -> f64 {
        let a1 = t1.max(1.0);
        let a2 = t2.max(1.0);
        let dh_j_kg = self.a * (a2 - a1)
            + 0.5 * self.b * 1e-3 * (a2 * a2 - a1 * a1)
            - self.c * 1e5 * (1.0 / a2 - 1.0 / a1)
            + (self.d * 1e-6 / 3.0) * (a2.powi(3) - a1.powi(3))
            - 0.5 * self.e * 1e8 * (1.0 / (a2 * a2) - 1.0 / (a1 * a1))
            + 0.25 * self.f * 1e-9 * (a2.powi(4) - a1.powi(4));
        dh_j_kg / 1000.0
    }
}

/// Free-form tabular data entry in the `Data` sub-tab (Fig. 2 bottom bar).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialDataRow {
    pub parameter: String,
    pub temperature_k: String,
    pub value: String,
    pub unit: String,
    pub reference: String,
}

/// User-defined custom property entry in `User Properties` (Fig. 2 bottom bar, §90).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserProperty {
    pub name: String,
    pub value: String,
    pub unit: String,
    pub reference: String,
}

/// Complete material record stored in `MaterialDatabase`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialRecord {
    pub name: String,
    pub code: String,
    pub category: MaterialCategory,
    pub supplier: String,
    pub description: String,
    pub updated_at: String,
    pub composition: Vec<CompositionEntry>,
    pub analyses: Vec<AnalysisEntry>,
    pub properties: MaterialProperties,
    pub cp_ranges: Vec<MaterialCpRange>,
    pub data_rows: Vec<MaterialDataRow>,
    pub user_properties: Vec<UserProperty>,
}

impl MaterialRecord {
    pub fn new_blank(name: impl Into<String>) -> Self {
        let name_s = name.into();
        let mut props = MaterialProperties::default();
        props.avg_molar_weight.auto_calculate = true;
        props.density.auto_calculate = true;
        props.enthalpy_25c.auto_calculate = true;
        Self {
            name: name_s,
            code: String::new(),
            category: MaterialCategory::Mixture,
            supplier: String::new(),
            description: String::new(),
            updated_at: "2026-11-22".into(),
            composition: Vec::new(),
            analyses: Vec::new(),
            properties: props,
            cp_ranges: vec![MaterialCpRange::default()],
            data_rows: Vec::new(),
            user_properties: Vec::new(),
        }
    }

    /// Scales all positive composition rows proportionally so their sum equals `100.000%`.
    pub fn normalize_composition(&mut self) -> bool {
        let sum: f64 = self
            .composition
            .iter()
            .filter(|e| !e.item.trim().is_empty() && e.wt_pct > 0.0)
            .map(|e| e.wt_pct)
            .sum();
        if sum <= 1e-12 {
            return false;
        }
        let factor = 100.0 / sum;
        for e in &mut self.composition {
            if !e.item.trim().is_empty() && e.wt_pct > 0.0 {
                e.wt_pct *= factor;
            }
        }
        true
    }
}

/// Output of recursively breaking down a material into species and elements (Fig. 1).
#[derive(Debug, Clone, Default)]
pub struct ResolvedComposition {
    pub composition_total_wt: f64,
    pub species: Vec<(String, f64)>,
    pub species_total_wt: f64,
    pub elements: Vec<(String, f64)>,
    pub elements_total_wt: f64,
    pub warnings: Vec<String>,
}

/// Weighted-average estimates for `Auto Calculate` properties (Fig. 2).
#[derive(Debug, Clone, Default)]
pub struct AutoCalcResult {
    pub avg_molar_weight_g_mol: Option<f64>,
    pub density_kg_l: Option<f64>,
    pub enthalpy_25c_kj_kg: Option<f64>,
    pub heat_capacity_25c_j_kg_k: Option<f64>,
    pub matched_species: usize,
    pub total_species: usize,
}

/// Collection of materials with recursive composition resolution, `Auto Calculate` physics,
/// Cp polynomial evaluation/fitting, and JSON/CSV persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDatabase {
    pub materials: Vec<MaterialRecord>,
}

impl Default for MaterialDatabase {
    fn default() -> Self {
        Self::with_default_catalog()
    }
}

impl MaterialDatabase {
    /// Builds the default catalog featuring the exact `Feed Material` + `[Concentrate]` +
    /// `[Silica Flux]` hierarchy from Chapter 90 Figs. 1–3, plus `[Steel]`, `[Bituminous Coal]`,
    /// `[Limestone]`, `[Fayalite Slag]`, and `[HDPE Plastic]`.
    pub fn with_default_catalog() -> Self {
        // 1. [Concentrate]: Cu-Fe sulfide concentrate whose 75% share + 20% [Silica Flux] + 3% C + 2% H2O
        //    reproduces the exact species & elemental breakdown in Fig. 1.
        let mut concentrate = MaterialRecord::new_blank("Concentrate");
        concentrate.code = "MAT-CONC-01".into();
        concentrate.category = MaterialCategory::Concentrate;
        concentrate.supplier = "Pyrometallurgical Mine".into();
        concentrate.description = "Copper-iron sulfide flotation concentrate (Ch. 90 Fig. 1).".into();
        concentrate.composition = vec![
            CompositionEntry::new("CuFeS2", 22.450667),
            CompositionEntry::new("Cu5FeS4", 1.754667),
            CompositionEntry::new("Cu2S", 11.140000),
            CompositionEntry::new("FeS", 12.570667),
            CompositionEntry::new("FeS2", 35.000000),
            CompositionEntry::new("ZnS", 1.750667),
            CompositionEntry::new("NiS", 0.878667),
            CompositionEntry::new("SiO2", 7.740000),
            CompositionEntry::new("MgO", 5.372000),
            CompositionEntry::new("Al2O3", 0.606667),
            CompositionEntry::new("Fe2O3", 0.605333),
            CompositionEntry::new("CaO", 0.130665),
        ];
        concentrate.properties.state_at_25c = PropertyField::manual("Solid powder");
        concentrate.cp_ranges = vec![MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 1000.0,
            phase: "s".into(),
            h_kj_kg: -2510.0,
            a: 495.0,
            b: 140.0,
            c: -45.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: "HSC Sulfide Concentrate".into(),
        }];

        // 2. [Silica Flux]: 20% contribution to Feed Material
        let mut silica_flux = MaterialRecord::new_blank("Silica Flux");
        silica_flux.code = "MAT-FLUX-01".into();
        silica_flux.category = MaterialCategory::Flux;
        silica_flux.supplier = "Quarry Quartz".into();
        silica_flux.description = "Silica smelting flux (90% SiO2, 5% Al2O3, 5% CaO).".into();
        silica_flux.composition = vec![
            CompositionEntry::new("SiO2", 90.000),
            CompositionEntry::new("Al2O3", 5.000),
            CompositionEntry::new("CaO", 5.000),
        ];
        silica_flux.properties.state_at_25c = PropertyField::manual("Solid granular");
        silica_flux.properties.melting_point = PropertyField::manual("1986");
        silica_flux.cp_ranges = vec![MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 1200.0,
            phase: "s".into(),
            h_kj_kg: -15030.0,
            a: 740.0,
            b: 310.0,
            c: -180.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: "Quartz Flux".into(),
        }];

        // 3. Feed Material (exact Chapter 90 Figs. 1, 2, 3)
        let mut feed = MaterialRecord::new_blank("Feed Material");
        feed.code = "MAT-FEED-01".into();
        feed.category = MaterialCategory::Mixture;
        feed.supplier = "Smelter Blend".into();
        feed.description =
            "Flash smelting furnace feed blend (75% [Concentrate] + 20% [Silica Flux] + 3% C + 2% H2O).".into();
        feed.composition = vec![
            CompositionEntry::new("[Concentrate]", 75.000),
            CompositionEntry::new("[Silica Flux]", 20.000),
            CompositionEntry::new("C", 3.000),
            CompositionEntry::new("H2O", 2.000),
        ];
        feed.analyses = vec![
            AnalysisEntry {
                parameter: "Moisture (H2O)".into(),
                value: "2.000".into(),
                unit: "wt-%".into(),
                method: "Oven drying 105 °C".into(),
                reference: "Lab Assay".into(),
            },
            AnalysisEntry {
                parameter: "Loss on Ignition (LOI)".into(),
                value: "24.8".into(),
                unit: "wt-%".into(),
                method: "TGA 950 °C".into(),
                reference: "Lab Assay".into(),
            },
            AnalysisEntry {
                parameter: "P80 Particle Size".into(),
                value: "75".into(),
                unit: "um".into(),
                method: "Laser diffraction".into(),
                reference: "Mill Spec".into(),
            },
        ];
        // Fig. 2: Auto Calculate = TRUE on Average molar weight, Density, Enthalpy at 25 °C
        feed.properties.avg_molar_weight = PropertyField::auto_calc();
        feed.properties.density = PropertyField::auto_calc();
        feed.properties.enthalpy_25c = PropertyField::auto_calc();
        // Fig. 3 exact two-range Cp polynomial data
        feed.cp_ranges = vec![
            MaterialCpRange {
                t_min_k: 298.150,
                t_max_k: 750.000,
                phase: "s".into(),
                h_kj_kg: -5661.375,
                a: 250.000,
                b: 55.000,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "HSC Ch. 90 Fig. 3".into(),
            },
            MaterialCpRange {
                t_min_k: 750.000,
                t_max_k: 1000.000,
                phase: "s".into(),
                h_kj_kg: 0.000,
                a: 280.000,
                b: 0.000,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "HSC Ch. 90 Fig. 3".into(),
            },
        ];
        feed.data_rows = vec![
            MaterialDataRow {
                parameter: "Bulk Density".into(),
                temperature_k: "298.15".into(),
                value: "2.15".into(),
                unit: "kg/l".into(),
                reference: "Plant measurement".into(),
            },
            MaterialDataRow {
                parameter: "Angle of Repose".into(),
                temperature_k: "298.15".into(),
                value: "36.5".into(),
                unit: "deg".into(),
                reference: "Plant measurement".into(),
            },
        ];
        feed.user_properties = vec![
            UserProperty {
                name: "Moisture Target".into(),
                value: "< 2.5".into(),
                unit: "wt-%".into(),
                reference: "Dryer spec".into(),
            },
            UserProperty {
                name: "Cu/S Mass Ratio".into(),
                value: "0.512".into(),
                unit: "-".into(),
                reference: "Calculated".into(),
            },
        ];

        // 4. [Steel] (explicitly cited in §90.1: "e.g. [Steel]")
        let mut steel = MaterialRecord::new_blank("Steel");
        steel.code = "MAT-STL-01".into();
        steel.category = MaterialCategory::Metal;
        steel.supplier = "Structural Mill".into();
        steel.description = "Low-carbon structural steel (AISI 1020 equivalent).".into();
        steel.composition = vec![
            CompositionEntry::new("Fe", 98.400),
            CompositionEntry::new("Mn", 1.000),
            CompositionEntry::new("Si", 0.350),
            CompositionEntry::new("C", 0.250),
        ];
        steel.properties.state_at_25c = PropertyField::manual("Solid");
        steel.properties.melting_point = PropertyField::manual("1795");
        steel.properties.electrical_conductivity = PropertyField::manual("6.2e6");
        steel.properties.hardness_hv = PropertyField::manual("145");
        steel.properties.youngs_modulus = PropertyField::manual("205000");
        steel.cp_ranges = vec![
            MaterialCpRange {
                t_min_k: 298.15,
                t_max_k: 1000.0,
                phase: "s".into(),
                h_kj_kg: 0.0,
                a: 425.0,
                b: 285.0,
                c: -12.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "Ferritic Steel".into(),
            },
            MaterialCpRange {
                t_min_k: 1000.0,
                t_max_k: 1795.0,
                phase: "s".into(),
                h_kj_kg: 18.5,
                a: 580.0,
                b: 60.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "Austenitic Steel".into(),
            },
        ];

        // 5. [Bituminous Coal]
        let mut coal = MaterialRecord::new_blank("Bituminous Coal");
        coal.code = "MAT-FUEL-01".into();
        coal.category = MaterialCategory::Fuel;
        coal.supplier = "Thermal Coal Mine".into();
        coal.description = "Pulverized metallurgical coal with mineral ash and moisture.".into();
        coal.composition = vec![
            CompositionEntry::new("C", 74.000),
            CompositionEntry::new("H2O", 6.000),
            CompositionEntry::new("SiO2", 11.000),
            CompositionEntry::new("Al2O3", 5.000),
            CompositionEntry::new("Fe2O3", 2.500),
            CompositionEntry::new("FeS2", 1.500),
        ];
        coal.properties.state_at_25c = PropertyField::manual("Solid");
        coal.analyses = vec![
            AnalysisEntry {
                parameter: "Gross Calorific Value (HHV)".into(),
                value: "28.4".into(),
                unit: "MJ/kg".into(),
                method: "Bomb calorimeter".into(),
                reference: "ISO 1928".into(),
            },
            AnalysisEntry {
                parameter: "Ash Content".into(),
                value: "20.0".into(),
                unit: "wt-%".into(),
                method: "Proximate".into(),
                reference: "ISO 1171".into(),
            },
        ];
        coal.cp_ranges = vec![MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 900.0,
            phase: "s".into(),
            h_kj_kg: -3610.0,
            a: 1050.0,
            b: 720.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: "Merrick Coal Model".into(),
        }];

        // 6. [Limestone]
        let mut limestone = MaterialRecord::new_blank("Limestone");
        limestone.code = "MAT-FLUX-02".into();
        limestone.category = MaterialCategory::Flux;
        limestone.supplier = "Carbonate Quarry".into();
        limestone.description = "Calcining limestone flux (94% CaCO3).".into();
        limestone.composition = vec![
            CompositionEntry::new("CaCO3", 94.000),
            CompositionEntry::new("MgO", 3.000),
            CompositionEntry::new("SiO2", 2.000),
            CompositionEntry::new("H2O", 1.000),
        ];
        limestone.properties.state_at_25c = PropertyField::manual("Solid");
        limestone.properties.hardness_mohs = PropertyField::manual("3.0");
        limestone.cp_ranges = vec![MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 1170.0,
            phase: "s".into(),
            h_kj_kg: -12250.0,
            a: 820.0,
            b: 490.0,
            c: -110.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: "Calcite Flux".into(),
        }];

        // 7. [Fayalite Slag]
        let mut slag = MaterialRecord::new_blank("Fayalite Slag");
        slag.code = "MAT-SLAG-01".into();
        slag.category = MaterialCategory::Slag;
        slag.supplier = "Smelter Tap".into();
        slag.description = "Copper smelting fayalite slag (Fe2SiO4-rich silicate).".into();
        slag.composition = vec![
            CompositionEntry::new("Fe2SiO4", 68.000),
            CompositionEntry::new("SiO2", 15.000),
            CompositionEntry::new("CaO", 8.000),
            CompositionEntry::new("Al2O3", 6.000),
            CompositionEntry::new("MgO", 3.000),
        ];
        slag.properties.state_at_25c = PropertyField::manual("Solid glassy");
        slag.properties.melting_point = PropertyField::manual("1478");
        slag.cp_ranges = vec![
            MaterialCpRange {
                t_min_k: 298.15,
                t_max_k: 1478.0,
                phase: "s".into(),
                h_kj_kg: -9560.0,
                a: 760.0,
                b: 220.0,
                c: -140.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "Slag Atlas".into(),
            },
            MaterialCpRange {
                t_min_k: 1478.0,
                t_max_k: 1900.0,
                phase: "l".into(),
                h_kj_kg: 380.0,
                a: 1140.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
                e: 0.0,
                f: 0.0,
                reference: "Slag Atlas".into(),
            },
        ];

        // 8. [HDPE Plastic]
        let mut hdpe = MaterialRecord::new_blank("HDPE Plastic");
        hdpe.code = "MAT-POLY-01".into();
        hdpe.category = MaterialCategory::Polymer;
        hdpe.supplier = "Polymer Recycling".into();
        hdpe.description = "High-density polyethylene (C2H4)n feedstock.".into();
        hdpe.composition = vec![
            CompositionEntry::new("C2H4", 99.500),
            CompositionEntry::new("TiO2", 0.500),
        ];
        hdpe.properties.state_at_25c = PropertyField::manual("Solid polymer");
        hdpe.properties.density = PropertyField::manual("0.955");
        hdpe.properties.melting_point = PropertyField::manual("405");
        hdpe.properties.youngs_modulus = PropertyField::manual("1050");
        hdpe.cp_ranges = vec![MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 405.0,
            phase: "s".into(),
            h_kj_kg: -1850.0,
            a: 1850.0,
            b: 1900.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            reference: "Polymer Handbook".into(),
        }];

        Self {
            materials: vec![
                feed,
                concentrate,
                silica_flux,
                steel,
                coal,
                limestone,
                slag,
                hdpe,
            ],
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<&MaterialRecord> {
        let needle = name.trim();
        self.materials
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(needle))
    }

    /// Recursively expands `[SubMaterial]` entries into flat species wt-% and elemental wt-% (Fig. 1).
    pub fn resolve_composition(&self, material: &MaterialRecord) -> ResolvedComposition {
        let mut res = ResolvedComposition::default();
        res.composition_total_wt = material
            .composition
            .iter()
            .filter(|e| !e.item.trim().is_empty())
            .map(|e| e.wt_pct)
            .sum();

        let mut species_order: Vec<String> = Vec::new();
        let mut species_map: HashMap<String, f64> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(material.name.to_ascii_lowercase());

        self.expand_entries(
            &material.composition,
            1.0,
            &mut visited,
            &mut species_order,
            &mut species_map,
            &mut res.warnings,
        );

        for sp in species_order {
            if let Some(&wt) = species_map.get(&sp) {
                if wt.abs() > 1e-12 {
                    res.species_total_wt += wt;
                    res.species.push((sp, wt));
                }
            }
        }

        // Break down expanded species to elemental wt-% preserving chemical appearance order
        let mut elem_order: Vec<String> = Vec::new();
        let mut elem_map: HashMap<String, f64> = HashMap::new();

        for (formula, wt_pct) in &res.species {
            match parse_hsc_formula_elements(formula) {
                Ok((counts, _charge)) if !counts.is_empty() => {
                    // Use formula appearance order so CuFeS2 registers Cu, Fe, S in that order (matching Fig. 1)
                    let ordered_syms = extract_element_appearance_order(formula, &counts);
                    let mut molar_mass = 0.0;
                    for (sym, &stoich) in &counts {
                        let aw = awdb_atomic_weight(sym).unwrap_or(0.0);
                        molar_mass += aw * stoich;
                    }
                    if molar_mass > 1e-9 {
                        for sym in ordered_syms {
                            if let Some(&stoich) = counts.get(&sym) {
                                let aw = awdb_atomic_weight(&sym).unwrap_or(0.0);
                                let el_wt = wt_pct * (aw * stoich / molar_mass);
                                if !elem_map.contains_key(&sym) {
                                    elem_order.push(sym.clone());
                                }
                                *elem_map.entry(sym).or_insert(0.0) += el_wt;
                            }
                        }
                    } else {
                        res.warnings
                            .push(format!("Zero molar mass for species '{formula}'."));
                    }
                }
                _ => {
                    res.warnings
                        .push(format!("Could not parse elemental formula for '{formula}'."));
                }
            }
        }

        for el in elem_order {
            if let Some(&wt) = elem_map.get(&el) {
                if wt.abs() > 1e-12 {
                    res.elements_total_wt += wt;
                    res.elements.push((el, wt));
                }
            }
        }

        if (res.composition_total_wt - 100.0).abs() > 0.005 && !material.composition.is_empty() {
            res.warnings.push(format!(
                "Composition total is {:.3}% (should add up to 100.000%).",
                res.composition_total_wt
            ));
        }

        res
    }

    fn expand_entries(
        &self,
        entries: &[CompositionEntry],
        scale: f64,
        visited: &mut HashSet<String>,
        species_order: &mut Vec<String>,
        species_map: &mut HashMap<String, f64>,
        warnings: &mut Vec<String>,
    ) {
        for entry in entries {
            let trimmed = entry.item.trim();
            if trimmed.is_empty() || entry.wt_pct.abs() <= 1e-12 {
                continue;
            }
            if let Some(sub_name) = entry.submaterial_name() {
                let key = sub_name.to_ascii_lowercase();
                if visited.contains(&key) {
                    warnings.push(format!(
                        "Circular material reference detected for '[{sub_name}]'."
                    ));
                    continue;
                }
                if let Some(sub_mat) = self.find_by_name(sub_name) {
                    visited.insert(key.clone());
                    let sub_sum: f64 = sub_mat
                        .composition
                        .iter()
                        .filter(|e| !e.item.trim().is_empty())
                        .map(|e| e.wt_pct)
                        .sum();
                    let norm = if sub_sum > 1e-12 { 100.0 / sub_sum } else { 1.0 };
                    let child_scale = scale * (entry.wt_pct / 100.0) * norm;
                    self.expand_entries(
                        &sub_mat.composition,
                        child_scale,
                        visited,
                        species_order,
                        species_map,
                        warnings,
                    );
                    visited.remove(&key);
                } else {
                    warnings.push(format!(
                        "Referenced material '[{sub_name}]' not found in Material Database."
                    ));
                }
            } else {
                let contrib = entry.wt_pct * scale;
                if !species_map.contains_key(trimmed) {
                    species_order.push(trimmed.to_string());
                }
                *species_map.entry(trimmed.to_string()).or_insert(0.0) += contrib;
            }
        }
    }

    /// Computes weighted-average `Auto Calculate` properties (Fig. 2) from the expanded species
    /// using `ThermoDatabase` (with NIST/Barin mineral fallback when species are absent or lack density).
    pub fn auto_calculate_properties(
        &self,
        material: &MaterialRecord,
        repo: &dyn SpeciesRepository,
    ) -> AutoCalcResult {
        let resolved = self.resolve_composition(material);
        let mut out = AutoCalcResult {
            total_species: resolved.species.len(),
            ..Default::default()
        };
        if resolved.species.is_empty() {
            return out;
        }

        let mut mw_num = 0.0;
        let mut mw_den = 0.0;

        let mut dens_num = 0.0; // mass-weighted average matching HSC Ch. 90 §90.1 ("weighted average of the property")
        let mut dens_w_sum = 0.0;

        let mut h_num = 0.0; // sum(w_i * h_i_kj_kg)
        let mut h_w_sum = 0.0;

        let mut cp_num = 0.0; // sum(w_i * cp_i_j_kg_k)
        let mut cp_w_sum = 0.0;

        for (formula, wt_pct) in &resolved.species {
            let w = *wt_pct;
            if w <= 0.0 {
                continue;
            }
            let props = lookup_species_props(formula, repo);
            if props.matched {
                out.matched_species += 1;
            }
            if let Some(mw) = props.molar_weight_g_mol {
                if mw > 1e-6 {
                    mw_num += w * mw;
                    mw_den += w;
                }
            }
            if let Some(rho) = props.density_kg_l {
                if rho > 1e-6 {
                    dens_num += w * rho;
                    dens_w_sum += w;
                }
            }
            if let Some(h_kj_kg) = props.h298_kj_kg {
                h_num += w * h_kj_kg;
                h_w_sum += w;
            }
            if let Some(cp_j_kg_k) = props.cp298_j_kg_k {
                cp_num += w * cp_j_kg_k;
                cp_w_sum += w;
            }
        }

        if mw_den > 1e-9 {
            out.avg_molar_weight_g_mol = Some(mw_num / mw_den);
        }
        if dens_w_sum > 1e-9 {
            out.density_kg_l = Some(dens_num / dens_w_sum);
        }
        if h_w_sum > 1e-9 {
            out.enthalpy_25c_kj_kg = Some(h_num / h_w_sum);
        }
        if cp_w_sum > 1e-9 {
            out.heat_capacity_25c_j_kg_k = Some(cp_num / cp_w_sum);
        }
        out
    }

    /// Refreshes all `Auto Calculate == TRUE` fields of `material` in place.
    pub fn apply_auto_calculate(
        &self,
        material: &mut MaterialRecord,
        repo: &dyn SpeciesRepository,
    ) -> AutoCalcResult {
        let calc = self.auto_calculate_properties(material, repo);
        if material.properties.avg_molar_weight.auto_calculate {
            material.properties.avg_molar_weight.value = calc
                .avg_molar_weight_g_mol
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default();
        }
        if material.properties.density.auto_calculate {
            material.properties.density.value = calc
                .density_kg_l
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default();
        }
        if material.properties.enthalpy_25c.auto_calculate {
            material.properties.enthalpy_25c.value = calc
                .enthalpy_25c_kj_kg
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default();
        }
        if material.properties.heat_capacity.auto_calculate {
            let cp_val = calc
                .heat_capacity_25c_j_kg_k
                .or_else(|| eval_material_cp_and_h(material, 298.15).map(|(cp, _)| cp));
            material.properties.heat_capacity.value = cp_val
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default();
        }
        calc
    }

    /// Generates a mass-weighted Kelley 4-term `Cp Data` range (`298.15..1000 K`) directly from
    /// the resolved species in `ThermoDatabase` (`J/(kg·K)` and `H298` in `kJ/kg`).
    pub fn auto_fit_cp_from_composition(
        &self,
        material: &MaterialRecord,
        repo: &dyn SpeciesRepository,
    ) -> Result<MaterialCpRange, String> {
        let resolved = self.resolve_composition(material);
        if resolved.species.is_empty() {
            return Err("Material composition is empty.".into());
        }
        let mut w_tot = 0.0;
        let mut a = 0.0;
        let mut b = 0.0;
        let mut c = 0.0;
        let mut d = 0.0;
        let mut h298 = 0.0;

        for (formula, wt) in &resolved.species {
            if *wt <= 0.0 {
                continue;
            }
            if let Some(sp) = find_species_in_repo(formula, repo) {
                if let Some(iv) = sp.intervals.first() {
                    let mw = sp.molecular_weight.max(1e-6);
                    let to_kg = 1000.0 / mw; // J/(mol·K) -> J/(kg·K)
                    a += wt * iv.kelley.a * to_kg;
                    b += wt * iv.kelley.b * to_kg;
                    c += wt * iv.kelley.c * to_kg;
                    d += wt * iv.kelley.d * to_kg;
                    h298 += wt * (iv.h_or_delta_h_tr / mw); // (J/mol)/(g/mol) = kJ/kg
                    w_tot += wt;
                    continue;
                }
            }
            let props = lookup_species_props(formula, repo);
            if let Some(cp) = props.cp298_j_kg_k {
                a += wt * cp;
                h298 += wt * props.h298_kj_kg.unwrap_or(0.0);
                w_tot += wt;
            }
        }
        if w_tot <= 1e-9 {
            return Err("No species thermodynamic Cp data found for this composition.".into());
        }
        Ok(MaterialCpRange {
            t_min_k: 298.15,
            t_max_k: 1000.0,
            phase: "s".into(),
            h_kj_kg: h298 / w_tot,
            a: a / w_tot,
            b: b / w_tot,
            c: c / w_tot,
            d: d / w_tot,
            e: 0.0,
            f: 0.0,
            reference: "Auto-fitted from Species DB".into(),
        })
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("JSON serialize error: {e}"))
    }

    pub fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| format!("JSON parse error: {e}"))
    }

    /// Exports one material (Composition, Species, Elements, Properties, Cp Data) as clean CSV.
    pub fn export_material_csv(&self, material: &MaterialRecord) -> String {
        let res = self.resolve_composition(material);
        let mut out = String::new();
        out.push_str(&format!("Material,{}\n", material.name));
        out.push_str(&format!(
            "Code,{},Category,{}\n\n",
            material.code,
            material.category.label()
        ));
        out.push_str("Composition,wt-%,Species,wt-%,Elements,wt-%,Reference\n");
        out.push_str(&format!(
            "Total,{:.3},,{:.3},,{:.3},\n",
            res.composition_total_wt, res.species_total_wt, res.elements_total_wt
        ));
        let nrows = material
            .composition
            .len()
            .max(res.species.len())
            .max(res.elements.len());
        for i in 0..nrows {
            let (c_item, c_wt, c_ref) = material
                .composition
                .get(i)
                .map(|e| (e.item.as_str(), format!("{:.3}", e.wt_pct), e.reference.as_str()))
                .unwrap_or(("", String::new(), ""));
            let (s_item, s_wt) = res
                .species
                .get(i)
                .map(|(s, w)| (s.as_str(), format!("{w:.3}")))
                .unwrap_or(("", String::new()));
            let (e_item, e_wt) = res
                .elements
                .get(i)
                .map(|(e, w)| (e.as_str(), format!("{w:.3}")))
                .unwrap_or(("", String::new()));
            out.push_str(&format!("{c_item},{c_wt},{s_item},{s_wt},{e_item},{e_wt},{c_ref}\n"));
        }

        out.push_str("\nProperty,Value,Unit,Reference,Auto Calculate\n");
        let p = &material.properties;
        let rows: [(&str, &PropertyField, &str); 15] = [
            ("State at 25 °C", &p.state_at_25c, "Text"),
            ("Average molar weight", &p.avg_molar_weight, "g/mol"),
            ("Density", &p.density, "kg/l"),
            ("Melting Point", &p.melting_point, "K"),
            ("Boiling Point", &p.boiling_point, "K"),
            ("Heat Capacity", &p.heat_capacity, "J/(kg*K)"),
            ("Enthalpy at 25 °C", &p.enthalpy_25c, "kJ/kg"),
            ("Heat of Fusion", &p.heat_of_fusion, "kJ/kg"),
            ("Heat of Vaporization", &p.heat_of_vaporization, "kJ/kg"),
            ("Electrical Conductivity", &p.electrical_conductivity, "S/m"),
            ("Mass Susceptibility", &p.mass_susceptibility, "m3/kg"),
            ("Magnetic permeability", &p.magnetic_permeability, "H/m"),
            ("Hardness (HV)", &p.hardness_hv, "HV"),
            ("Hardness (Mohs)", &p.hardness_mohs, "Mohs"),
            ("Young's Modulus", &p.youngs_modulus, "MPa"),
        ];
        for (name, field, unit) in rows {
            let ac = if field.auto_calculate { "TRUE" } else { "" };
            out.push_str(&format!("{name},{},{unit},{},{ac}\n", field.value, field.reference));
        }

        out.push_str("\nRange,Tmin (K),Tmax (K),Phase,H (kJ/kg),A,B,C,D,E,F,Reference\n");
        for (idx, r) in material.cp_ranges.iter().enumerate() {
            out.push_str(&format!(
                "{},{:.3},{:.3},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{}\n",
                idx + 1,
                r.t_min_k,
                r.t_max_k,
                r.phase,
                r.h_kj_kg,
                r.a,
                r.b,
                r.c,
                r.d,
                r.e,
                r.f,
                r.reference
            ));
        }
        out
    }
}

/// Evaluates `Cp(T)` in `J/(kg·K)` and `H(T)` in `kJ/kg` from a material's `cp_ranges`.
pub fn eval_material_cp_and_h(material: &MaterialRecord, t_k: f64) -> Option<(f64, f64)> {
    let valid: Vec<&MaterialCpRange> = material
        .cp_ranges
        .iter()
        .filter(|r| r.t_max_k > r.t_min_k && r.t_min_k > 0.0)
        .collect();
    if valid.is_empty() {
        return None;
    }

    let first = valid[0];
    let mut h_acc = first.h_kj_kg;

    if t_k < first.t_min_k {
        h_acc += first.integrate_h_kj_kg(first.t_min_k, t_k);
        return Some((first.eval_cp_j_kg_k(t_k), h_acc));
    }

    for (idx, r) in valid.iter().enumerate() {
        if idx > 0 {
            h_acc += r.h_kj_kg; // transition enthalpy at t_min
        }
        if t_k <= r.t_max_k || idx + 1 == valid.len() {
            h_acc += r.integrate_h_kj_kg(r.t_min_k, t_k);
            return Some((r.eval_cp_j_kg_k(t_k), h_acc));
        }
        h_acc += r.integrate_h_kj_kg(r.t_min_k, r.t_max_k);
    }
    None
}

/// Generates piecewise `(T_K, Cp_J_kg_K)` curve segments for `Cp Chart` (Fig. 3).
/// Adjacent ranges emit both left and right endpoint values so phase/range steps render sharply.
pub fn sample_cp_chart_points(material: &MaterialRecord, samples_per_range: usize) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    let n = samples_per_range.max(8);
    for r in &material.cp_ranges {
        if r.t_max_k <= r.t_min_k || r.t_min_k <= 0.0 {
            continue;
        }
        for i in 0..=n {
            let frac = (i as f64) / (n as f64);
            let t = r.t_min_k + frac * (r.t_max_k - r.t_min_k);
            pts.push((t, r.eval_cp_j_kg_k(t)));
        }
    }
    pts
}

// ============================================================================
// Internal Species Lookup & Fallback Reference Table (NIST / Barin / HSC)
// ============================================================================

struct SpeciesQuickProps {
    matched: bool,
    molar_weight_g_mol: Option<f64>,
    density_kg_l: Option<f64>,
    h298_kj_kg: Option<f64>,
    cp298_j_kg_k: Option<f64>,
}

fn find_species_in_repo(formula: &str, repo: &dyn SpeciesRepository) -> Option<ChemicalSpecies> {
    let f = formula.trim();
    if let Some(sp) = repo.get_species(f) {
        return Some(sp);
    }
    if f.eq_ignore_ascii_case("H2O") {
        if let Some(sp) = repo.get_species("H2O(l)") {
            return Some(sp);
        }
    }
    if !f.contains('(') {
        for suffix in ["(s)", "(l)", "(g)"] {
            let cand = format!("{f}{suffix}");
            if let Some(sp) = repo.get_species(&cand) {
                return Some(sp);
            }
        }
    }
    None
}

fn lookup_species_props(formula: &str, repo: &dyn SpeciesRepository) -> SpeciesQuickProps {
    let fb = fallback_mineral_props(formula);
    let parsed_mw = formula_molar_weight(formula);

    if let Some(sp) = find_species_in_repo(formula, repo) {
        let mw = if sp.molecular_weight > 1e-6 {
            Some(sp.molecular_weight)
        } else {
            parsed_mw
        };
        let dens = if sp.density_g_cm3 > 1e-6 {
            Some(sp.density_g_cm3)
        } else {
            fb.map(|x| x.0)
        };
        let (h298_kj_kg, cp298_j_kg_k) = match evaluate_species_si(&sp, 298.15, false, false) {
            Ok((pt, _)) => {
                let m = mw.unwrap_or(1.0).max(1e-6);
                // pt.h is J/mol -> (J/mol)/(g/mol) = kJ/kg
                // pt.cp is J/(mol·K) -> * 1000 / (g/mol) = J/(kg·K)
                (Some(pt.h / m), Some(pt.cp * 1000.0 / m))
            }
            Err(_) => (
                fb.map(|x| x.1),
                fb.map(|x| x.2),
            ),
        };
        return SpeciesQuickProps {
            matched: true,
            molar_weight_g_mol: mw,
            density_kg_l: dens,
            h298_kj_kg,
            cp298_j_kg_k,
        };
    }

    if let Some((rho, h_kj_kg, cp_j_kg_k)) = fb {
        return SpeciesQuickProps {
            matched: true,
            molar_weight_g_mol: parsed_mw,
            density_kg_l: Some(rho),
            h298_kj_kg: Some(h_kj_kg),
            cp298_j_kg_k: Some(cp_j_kg_k),
        };
    }

    SpeciesQuickProps {
        matched: false,
        molar_weight_g_mol: parsed_mw,
        density_kg_l: None,
        h298_kj_kg: None,
        cp298_j_kg_k: None,
    }
}

/// Fallback `(density_kg_l, h298_kj_kg, cp298_j_kg_k)` from standard NIST/Barin/HSC tables
/// for industrial minerals, oxides, sulfides, metals, and fuels when absent from open defaults.
fn fallback_mineral_props(formula: &str) -> Option<(f64, f64, f64)> {
    let key = formula.trim();
    match key {
        "CuFeS2" => Some((4.19, -1062.0, 521.0)),
        "Cu5FeS4" => Some((5.09, -739.0, 487.0)),
        "Cu2S" => Some((5.60, -500.0, 480.0)),
        "FeS" => Some((4.74, -1138.0, 575.0)),
        "FeS2" => Some((5.01, -1429.0, 518.0)),
        "ZnS" => Some((4.09, -2114.0, 472.0)),
        "NiS" => Some((5.35, -904.0, 520.0)),
        "SiO2" => Some((2.65, -15157.0, 742.0)),
        "MgO" => Some((3.58, -14926.0, 924.0)),
        "Al2O3" => Some((3.97, -16435.0, 775.0)),
        "Fe2O3" => Some((5.24, -5161.0, 650.0)),
        "CaO" => Some((3.34, -11325.0, 750.0)),
        "CaCO3" => Some((2.71, -12065.0, 834.0)),
        "Fe2SiO4" => Some((4.39, -7255.0, 652.0)),
        "TiO2" => Some((4.23, -11820.0, 690.0)),
        "C" => Some((2.26, 0.0, 710.0)),
        "H2O" | "H2O(l)" => Some((0.997, -15866.0, 4182.0)),
        "Fe" => Some((7.87, 0.0, 449.0)),
        "Mn" => Some((7.44, 0.0, 479.0)),
        "Si" => Some((2.33, 0.0, 705.0)),
        "Cr" => Some((7.19, 0.0, 449.0)),
        "Ni" => Some((8.91, 0.0, 444.0)),
        "Cu" => Some((8.96, 0.0, 385.0)),
        "C2H4" => Some((0.95, -1850.0, 1850.0)),
        _ => None,
    }
}

fn formula_molar_weight(formula: &str) -> Option<f64> {
    let (counts, _) = parse_hsc_formula_elements(formula).ok()?;
    if counts.is_empty() {
        return None;
    }
    let mut mw = 0.0;
    for (sym, stoich) in counts {
        mw += awdb_atomic_weight(&sym)? * stoich;
    }
    if mw > 1e-6 {
        Some(mw)
    } else {
        None
    }
}

fn extract_element_appearance_order(
    formula: &str,
    counts: &std::collections::BTreeMap<String, f64>,
) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = formula.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_uppercase() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            let sym = &formula[start..i];
            if counts.contains_key(sym) && !out.iter().any(|s| s == sym) {
                out.push(sym.to_string());
            }
        } else {
            i += 1;
        }
    }
    for k in counts.keys() {
        if !out.contains(k) {
            out.push(k.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyRepo;
    impl SpeciesRepository for EmptyRepo {
        fn get_species(&self, _formula: &str) -> Option<ChemicalSpecies> {
            None
        }
        fn get_reference_element(&self, _element_symbol: &str) -> Option<ChemicalSpecies> {
            None
        }
    }

    #[test]
    fn feed_material_matches_ch90_fig1_fig2_fig3() {
        let db = MaterialDatabase::with_default_catalog();
        let feed = db.find_by_name("Feed Material").unwrap();
        let res = db.resolve_composition(feed);

        assert!((res.composition_total_wt - 100.0).abs() < 1e-3);
        assert!((res.species_total_wt - 100.0).abs() < 1e-2);
        assert!((res.elements_total_wt - 100.0).abs() < 1e-2);
        assert_eq!(res.species.len(), 14);

        let sp_map: HashMap<&str, f64> =
            res.species.iter().map(|(s, v)| (s.as_str(), *v)).collect();
        assert!((sp_map["CuFeS2"] - 16.838).abs() < 0.01);
        assert!((sp_map["FeS2"] - 26.250).abs() < 0.01);
        assert!((sp_map["SiO2"] - 23.805).abs() < 0.01);
        assert!((sp_map["C"] - 3.000).abs() < 0.01);
        assert!((sp_map["H2O"] - 2.000).abs() < 0.01);

        let el_map: HashMap<&str, f64> =
            res.elements.iter().map(|(e, v)| (e.as_str(), *v)).collect();
        assert!((el_map["Cu"] - 13.335).abs() < 0.05, "Cu={}", el_map["Cu"]);
        assert!((el_map["Fe"] - 23.795).abs() < 0.05, "Fe={}", el_map["Fe"]);
        assert!((el_map["S"] - 26.038).abs() < 0.05, "S={}", el_map["S"]);
        assert!((el_map["Si"] - 11.127).abs() < 0.05, "Si={}", el_map["Si"]);
        assert!((el_map["O"] - 17.188).abs() < 0.08, "O={}", el_map["O"]);

        // Fig. 2 Auto Calculate check
        let auto = db.auto_calculate_properties(feed, &EmptyRepo);
        let mw = auto.avg_molar_weight_g_mol.unwrap();
        let rho = auto.density_kg_l.unwrap();
        let h25 = auto.enthalpy_25c_kj_kg.unwrap();
        assert!((mw - 111.931).abs() < 0.2, "mw={mw}");
        assert!((rho - 4.062).abs() < 0.25, "rho={rho}");
        assert!((h25 - (-5661.374)).abs() < 25.0, "h25={h25}");

        // Fig. 3 Cp Data two-range evaluation & step discontinuity at 750 K
        let cp_298 = feed.cp_ranges[0].eval_cp_j_kg_k(298.15);
        let cp_750_left = feed.cp_ranges[0].eval_cp_j_kg_k(750.0);
        let cp_750_right = feed.cp_ranges[1].eval_cp_j_kg_k(750.0);
        assert!((cp_298 - 266.39825).abs() < 1e-4, "cp_298={cp_298}");
        assert!((cp_750_left - 291.25).abs() < 1e-6, "cp_750_left={cp_750_left}");
        assert!((cp_750_right - 280.0).abs() < 1e-6, "cp_750_right={cp_750_right}");
    }

    #[test]
    fn circular_material_reference_is_caught_safely() {
        let mut db = MaterialDatabase { materials: vec![] };
        let mut a = MaterialRecord::new_blank("A");
        a.composition.push(CompositionEntry::new("[B]", 100.0));
        let mut b = MaterialRecord::new_blank("B");
        b.composition.push(CompositionEntry::new("[A]", 100.0));
        db.materials.push(a);
        db.materials.push(b);

        let res = db.resolve_composition(&db.materials[0]);
        assert!(res.warnings.iter().any(|w| w.contains("Circular")));
    }

    #[test]
    fn json_roundtrip_and_csv_export_work() {
        let db = MaterialDatabase::with_default_catalog();
        let json = db.to_json().unwrap();
        let db2 = MaterialDatabase::from_json(&json).unwrap();
        assert_eq!(db.materials.len(), db2.materials.len());
        let csv = db2.export_material_csv(&db2.materials[0]);
        assert!(csv.contains("Feed Material"));
        assert!(csv.contains("[Concentrate]"));
        assert!(csv.contains("CuFeS2"));
    }
}
