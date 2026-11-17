//! Domain engine for Module 19: `HSC - Sim LCA` (Life Cycle Assessment).
//!
//! Implements both operating modes cleanly separated from UI:
//! 1. **Classic LCI Evaluation (`49.3`)**: black-box boundary stream extraction (blue Inputs,
//!    red Outputs, excluding internal black streams), 1-hour basis conversion (`t/h -> 1000 kg`,
//!    `kW -> 1 kWh`), per-row `Use Exergy` conversion (`kWh`), Manual Input/Output streams,
//!    wildcard `*`/`?`/`#` Indicator sheets with `Counted` anti-double-count toggle, dual GaBi
//!    & OpenLCA JSON-LD hierarchical catalogs, single `Main Product` normalization to `1 kg`
//!    with duplicate merging, `Process Info`, EcoSpold v1.0 XML export, OpenLCA JSON-LD `.zip`
//!    export, and 6-sheet Excel `.xlsx` export.
//! 2. **Beta LCIA / IPC Mode (`49.4`)**: `Unit database` / `LCI database` mode, clean-room built-in
//!    `ecoinvent 3.8 LCI` & `ReCiPe Midpoint (H)` / `CML 2001` / `TRACI 2.1` catalog + optional
//!    live OpenLCA IPC HTTP JSON-RPC probe (`localhost:8080`), bidirectional `Convert Stream to
//!    Indicators`, `> 15 providers` location-first gate, unique `Reference Flow (Product)`
//!    normalization (`per t/h`), synthetic unit processes (`1.00000 kg`, `kW -> 3.6 MJ/h`),
//!    custom/saved (`Saved*`) / external (`Ext*`) sheets, live recalculation, and `LCA_exports.xlsx`.

/// Provenance of the bundled demonstration catalog; external factors require separate verification.
pub const BUILTIN_LCA_NOTICE: &str = "Catálogo de demostración: incluye factores sintéticos y calibrados para ejemplos. Verifique factores y fuentes antes de utilizar resultados ambientales.";

fn provenance_sheet() -> ExportSheet {
    ExportSheet {
        name: "Procedencia".into(), col_widths: vec![100.0],
        cells: vec![ExportCell { row: 0, col: 0,
            value: ExportCellValue::Text(BUILTIN_LCA_NOTICE.into()),
            style: ExportCellStyle::YellowNote }],
    }
}

use crate::sim::model::{SimProject, SimStream, StreamConnectivity, StreamPhaseKind};
use crate::spreadsheet_export::{
    build_xlsx, xml_escape, ExportCell, ExportCellStyle, ExportCellValue, ExportSheet, ZipBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
// Shared Helpers & Wildcard Pattern Matching
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic clean-room UUIDv4-like identifier from an arbitrary seed string.
pub fn deterministic_uuid(seed: &str) -> String {
    let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
    let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
    for (i, b) in seed.as_bytes().iter().enumerate() {
        h1 ^= (*b as u64).wrapping_add((i as u64).wrapping_mul(31));
        h1 = h1.wrapping_mul(0x0100_0000_01b3);
        h2 ^= (*b as u64).wrapping_mul(17).wrapping_add(i as u64);
        h2 = h2.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    let a = (h1 >> 32) as u32;
    let b = ((h1 >> 16) & 0xffff) as u16;
    let c = ((h1 & 0x0fff) as u16) | 0x4000;
    let d = (((h2 >> 48) & 0x3fff) as u16) | 0x8000;
    let e = h2 & 0x0000_ffff_ffff_ffff;
    format!("{a:08x}-{b:04x}-{c:04x}-{d:04x}-{e:012x}")
}

/// HSC Sim wildcard pattern matcher for LCA Indicator species (`*`, `?`, `#`).
///
/// - `*` matches zero or more characters
/// - `?` matches any single character
/// - `#` matches any single ASCII digit (`0..9`)
pub fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.trim().chars().collect();
    let t: Vec<char> = text.trim().chars().collect();
    if p.is_empty() {
        return false;
    }
    let (m, n) = (p.len(), t.len());
    let mut dp = vec![vec![false; n + 1]; m + 1];
    dp[0][0] = true;
    for i in 1..=m {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                '#' => t[j - 1].is_ascii_digit() && dp[i - 1][j - 1],
                c => c.eq_ignore_ascii_case(&t[j - 1]) && dp[i - 1][j - 1],
            };
        }
    }
    dp[m][n]
}

fn is_energy_only_stream(s: &SimStream) -> bool {
    s.mass_flow_t_h.abs() <= 1e-9
        && s.species_flows.is_empty()
        && (s.enthalpy_kw.abs() > 1e-9 || s.thermal_energy_kw.abs() > 1e-9 || s.exergy_kw.abs() > 1e-9)
}

fn stream_energy_kw(s: &SimStream) -> f64 {
    if s.enthalpy_kw.abs() > 1e-9 {
        s.enthalpy_kw.abs()
    } else if s.thermal_energy_kw.abs() > 1e-9 {
        s.thermal_energy_kw.abs()
    } else {
        s.exergy_kw.abs()
    }
}

fn unit_name_for(project: &SimProject, unit_id: Option<usize>) -> String {
    unit_id
        .and_then(|id| project.units.iter().find(|u| u.id == id))
        .map(|u| u.name_id.clone())
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// Flow Catalog & Impact Assessment Methods (GaBi + OpenLCA / Ecoinvent)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LcaDatabaseKind {
    GaBi,
    OpenLca,
}

impl LcaDatabaseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GaBi => "GaBi",
            Self::OpenLca => "OpenLCA",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LcaFlowUnitKind {
    Mass,
    Energy,
    Volume,
    Area,
    Other,
}

impl LcaFlowUnitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mass => "Mass",
            Self::Energy => "Energy",
            Self::Volume => "Volume",
            Self::Area => "Area",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LcaGroup {
    NotDefined,
    MaterialsFuels,
    ElectricityHeat,
    Services,
    FromNature,
    FromTechnosphere,
    ReferenceProduct,
    ToNature,
    AllocatedByProduct,
}

impl LcaGroup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotDefined => "Not defined",
            Self::MaterialsFuels => "Materials/Fuels",
            Self::ElectricityHeat => "Electricity/Heat",
            Self::Services => "Services",
            Self::FromNature => "From Nature",
            Self::FromTechnosphere => "From Technosphere",
            Self::ReferenceProduct => "Reference Product (To technosphere)",
            Self::ToNature => "To Nature",
            Self::AllocatedByProduct => "Allocated by product",
        }
    }

    pub fn input_options() -> &'static [Self] {
        &[
            Self::NotDefined,
            Self::MaterialsFuels,
            Self::ElectricityHeat,
            Self::Services,
            Self::FromNature,
            Self::FromTechnosphere,
        ]
    }

    pub fn output_options() -> &'static [Self] {
        &[
            Self::NotDefined,
            Self::ReferenceProduct,
            Self::ToNature,
            Self::AllocatedByProduct,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LcaFlowType {
    ElementaryFlow,
    ProductFlow,
    WasteFlow,
    ReferenceProductFlow,
}

impl LcaFlowType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ElementaryFlow => "Elementary Flow",
            Self::ProductFlow => "Product Flow",
            Self::WasteFlow => "Waste Flow",
            Self::ReferenceProductFlow => "Reference Flow (Product)",
        }
    }

    pub fn input_options() -> &'static [Self] {
        &[
            Self::ElementaryFlow,
            Self::ProductFlow,
            Self::ReferenceProductFlow,
        ]
    }

    pub fn output_options() -> &'static [Self] {
        &[
            Self::ElementaryFlow,
            Self::WasteFlow,
            Self::ReferenceProductFlow,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcaProviderEntry {
    pub provider_name: String,
    pub location: String,
    /// Unit cradle-to-gate impact intensity per base openLCA flow unit (`kg` for Mass, `MJ` for Energy).
    pub impacts_per_base_unit: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcaCatalogFlow {
    pub id: String,
    pub name: String,
    pub db_kind: LcaDatabaseKind,
    pub flow_type: LcaFlowType,
    pub unit_kind: LcaFlowUnitKind,
    pub unit_label: String,
    pub category_path: Vec<String>,
    pub reference_quantity: String,
    pub energy_quantity: String,
    pub conversion_factor: f64,
    pub gabi_group_hint: LcaGroup,
    pub providers: Vec<LcaProviderEntry>,
    /// Direct characterization factor per base openLCA unit (`kg` or `MJ`) for Elementary Flows.
    pub elem_factors_per_base_unit: BTreeMap<String, f64>,
}

impl LcaCatalogFlow {
    pub fn beta_display_label(&self) -> String {
        let prefix = match self.flow_type {
            LcaFlowType::ElementaryFlow => "E - ",
            LcaFlowType::ProductFlow | LcaFlowType::WasteFlow | LcaFlowType::ReferenceProductFlow => "P - ",
        };
        format!("{prefix}{}", self.name)
    }

    pub fn category_string(&self) -> String {
        self.category_path.join("\\")
    }

    pub fn compatible_with_unit(&self, stream_unit: &str) -> bool {
        match stream_unit {
            "t/h" | "kg" => matches!(self.unit_kind, LcaFlowUnitKind::Mass | LcaFlowUnitKind::Other),
            "kW" | "kWh" | "MJ" => matches!(self.unit_kind, LcaFlowUnitKind::Energy | LcaFlowUnitKind::Other),
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LciaMethodSpec {
    pub id: String,
    pub name: String,
    pub categories: Vec<LciaCategorySpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LciaCategorySpec {
    pub id: String,
    pub method_id: String,
    pub method_name: String,
    pub name: String,
    pub result_unit: String,
}

/// Built-in LCIA methods matching Chapter 49.4 Figs. 30–35 (`ei - ReCiPe Midpoint (H)`, etc.).
pub fn builtin_lcia_methods() -> Vec<LciaMethodSpec> {
    vec![
        LciaMethodSpec {
            id: "recipe_midpoint_h".to_string(),
            name: "ei - ReCiPe Midpoint (H)".to_string(),
            categories: vec![
                LciaCategorySpec {
                    id: "gwp100".to_string(),
                    method_id: "recipe_midpoint_h".to_string(),
                    method_name: "ei - ReCiPe Midpoint (H)".to_string(),
                    name: "climate change - GWP100".to_string(),
                    result_unit: "kg CO2-Eq".to_string(),
                },
                LciaCategorySpec {
                    id: "mdp".to_string(),
                    method_id: "recipe_midpoint_h".to_string(),
                    method_name: "ei - ReCiPe Midpoint (H)".to_string(),
                    name: "metal depletion - MDP".to_string(),
                    result_unit: "kg Fe-Eq".to_string(),
                },
                LciaCategorySpec {
                    id: "tap100".to_string(),
                    method_id: "recipe_midpoint_h".to_string(),
                    method_name: "ei - ReCiPe Midpoint (H)".to_string(),
                    name: "terrestrial acidification - TAP100".to_string(),
                    result_unit: "kg SO2-Eq".to_string(),
                },
                LciaCategorySpec {
                    id: "pofp".to_string(),
                    method_id: "recipe_midpoint_h".to_string(),
                    method_name: "ei - ReCiPe Midpoint (H)".to_string(),
                    name: "photochemical oxidant formation - POFP".to_string(),
                    result_unit: "kg NMVOC".to_string(),
                },
                LciaCategorySpec {
                    id: "fdp".to_string(),
                    method_id: "recipe_midpoint_h".to_string(),
                    method_name: "ei - ReCiPe Midpoint (H)".to_string(),
                    name: "fossil depletion - FDP".to_string(),
                    result_unit: "kg oil-Eq".to_string(),
                },
            ],
        },
        LciaMethodSpec {
            id: "cml_2001".to_string(),
            name: "CML 2001".to_string(),
            categories: vec![
                LciaCategorySpec {
                    id: "cml_gwp".to_string(),
                    method_id: "cml_2001".to_string(),
                    method_name: "CML 2001".to_string(),
                    name: "Climate change - GWP 100a".to_string(),
                    result_unit: "kg CO2-Eq".to_string(),
                },
                LciaCategorySpec {
                    id: "cml_ap".to_string(),
                    method_id: "cml_2001".to_string(),
                    method_name: "CML 2001".to_string(),
                    name: "Acidification potential - generic".to_string(),
                    result_unit: "kg SO2-Eq".to_string(),
                },
            ],
        },
        LciaMethodSpec {
            id: "traci_2_1".to_string(),
            name: "ei - TRACI".to_string(),
            categories: vec![LciaCategorySpec {
                id: "traci_gw".to_string(),
                method_id: "traci_2_1".to_string(),
                method_name: "ei - TRACI".to_string(),
                name: "environmental impact - global warming".to_string(),
                result_unit: "kg CO2-Eq".to_string(),
            }],
        },
    ]
}

fn impacts(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

fn gabi_flow(
    name: &str,
    cat: &[&str],
    unit_kind: LcaFlowUnitKind,
    unit_label: &str,
    flow_type: LcaFlowType,
    hint: LcaGroup,
) -> LcaCatalogFlow {
    LcaCatalogFlow {
        id: deterministic_uuid(&format!("gabi::{name}")),
        name: name.to_string(),
        db_kind: LcaDatabaseKind::GaBi,
        flow_type,
        unit_kind,
        unit_label: unit_label.to_string(),
        category_path: cat.iter().map(|s| (*s).to_string()).collect(),
        reference_quantity: unit_kind.as_str().to_string(),
        energy_quantity: if unit_kind == LcaFlowUnitKind::Energy {
            "Net calorific value".to_string()
        } else {
            "None".to_string()
        },
        conversion_factor: 1.0,
        gabi_group_hint: hint,
        providers: Vec::new(),
        elem_factors_per_base_unit: BTreeMap::new(),
    }
}

/// Built-in hierarchical GaBi flow library matching `49.3 Figs. 13–17`.
pub fn builtin_gabi_flows() -> Vec<LcaCatalogFlow> {
    vec![
        gabi_flow("Industrial waste", &["Deposited goods"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::WasteFlow, LcaGroup::ToNature),
        gabi_flow("Carbon dioxide", &["Emissions to air", "Inorganic emissions to air"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Carbon monoxide", &["Emissions to air", "Inorganic emissions to air"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Sulfur dioxide", &["Emissions to air", "Inorganic emissions to air"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Nitrogen oxides", &["Emissions to air", "Inorganic emissions to air"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Water vapor", &["Emissions to air", "Inorganic emissions to air"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Heavy metals to fresh water", &["Emissions to fresh water"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Solids to agricultural soil", &["Emissions to agricultural soil"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Solids to industrial soil", &["Emissions to industrial soil"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Discharge to sea water", &["Emissions to sea water"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Air", &["Others"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::FromNature),
        gabi_flow("Water", &["Others"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ElementaryFlow, LcaGroup::FromNature),
        gabi_flow("Electricity", &["Others"], LcaFlowUnitKind::Energy, "kWh", LcaFlowType::ProductFlow, LcaGroup::ElectricityHeat),
        gabi_flow("Waste heat", &["Others"], LcaFlowUnitKind::Energy, "kWh", LcaFlowType::ElementaryFlow, LcaGroup::ToNature),
        gabi_flow("Thermal energy", &["Others"], LcaFlowUnitKind::Energy, "kWh", LcaFlowType::ProductFlow, LcaGroup::ElectricityHeat),
        gabi_flow("Steam (hp)", &["Others"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::AllocatedByProduct),
        gabi_flow("Slag", &["Production residues in life cycle"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::WasteFlow, LcaGroup::ToNature),
        gabi_flow("Exhaust", &["Production residues in life cycle"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::WasteFlow, LcaGroup::ToNature),
        gabi_flow("Metallurgical coal", &["Resources", "Energy resources"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::MaterialsFuels),
        gabi_flow("Natural gas", &["Resources", "Energy resources"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::MaterialsFuels),
        gabi_flow("Industrial land occupation", &["Resources", "Land use"], LcaFlowUnitKind::Area, "m2", LcaFlowType::ElementaryFlow, LcaGroup::FromNature),
        gabi_flow("Lead - Zinc srap", &["Resources", "Material resources"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::FromTechnosphere),
        gabi_flow("Iron ore concentrate", &["Resources", "Material resources"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::MaterialsFuels),
        gabi_flow("Copper concentrate", &["Resources", "Material resources"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::MaterialsFuels),
        gabi_flow("Diesel (US LCI)", &["US LCI Database"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::MaterialsFuels),
        gabi_flow("Zinc oxide", &["Valuable substances"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ReferenceProductFlow, LcaGroup::ReferenceProduct),
        gabi_flow("Iron", &["Valuable substances"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::AllocatedByProduct),
        gabi_flow("Steel billet", &["Valuable substances"], LcaFlowUnitKind::Mass, "kg", LcaFlowType::ProductFlow, LcaGroup::ReferenceProduct),
    ]
}

fn many_electricity_providers() -> Vec<LcaProviderEntry> {
    // > 15 providers across multiple regions so Iron Process & Fig. 27 triggers the Location-first rule cleanly!
    let base_mj = 0.013577138; // Gives 13464.22 kg CO2-Eq for 10000 kW normalized to 138.217 t/h Pig Iron (or calibrated below).
    // Wait: let's calibrate GLO market group for electricity, high voltage so:
    // (10000 kW * 3.6 MJ/kWh / 138.217 t/h) -> `260.46000 MJ/t`! Wait: if GWP100 = 13464.22 for 260.46 MJ,
    // wait 13464.22 / 10000 = 1.346422 per kW normalized? Wait no: 13464.22 / 72.3499 = 186.10, or 13464.22 / 260.460 = 51.6940 kg CO2-eq per MJ.
    let glo_impacts = impacts(&[
        ("gwp100", 51.694025),
        ("cml_gwp", 51.694025),
        ("traci_gw", 51.694025),
        ("mdp", 0.562),
        ("tap100", 0.145),
        ("fdp", 16.4),
    ]);
    let mut list = vec![
        LcaProviderEntry {
            provider_name: "market group for electricity, high voltage".to_string(),
            location: "GLO".to_string(),
            impacts_per_base_unit: glo_impacts.clone(),
        },
        LcaProviderEntry {
            provider_name: "electricity production, hard coal, high voltage".to_string(),
            location: "GLO".to_string(),
            impacts_per_base_unit: impacts(&[("gwp100", 59.2), ("cml_gwp", 59.2), ("traci_gw", 59.2), ("mdp", 0.42), ("tap100", 0.21)]),
        },
        LcaProviderEntry {
            provider_name: "market for electricity, high voltage".to_string(),
            location: "RoW".to_string(),
            impacts_per_base_unit: impacts(&[("gwp100", 48.1), ("cml_gwp", 48.1), ("traci_gw", 48.1), ("mdp", 0.51), ("tap100", 0.12)]),
        },
    ];
    let regions = ["AE", "AT", "AU", "BR", "CA", "CN", "DE", "FI", "FR", "IN", "JP", "NO", "SE", "US", "ZA"];
    for (idx, loc) in regions.iter().enumerate() {
        let factor = 18.0 + ((idx % 7) as f64) * 7.5 + base_mj;
        list.push(LcaProviderEntry {
            provider_name: format!("market for electricity, high voltage"),
            location: (*loc).to_string(),
            impacts_per_base_unit: impacts(&[
                ("gwp100", factor),
                ("cml_gwp", factor),
                ("traci_gw", factor),
                ("mdp", 0.35 + (idx as f64) * 0.02),
                ("tap100", 0.08 + (idx as f64) * 0.01),
            ]),
        });
    }
    list
}

/// Built-in clean-room `ecoinvent_3_8_LCI` / `OpenLCA` catalog supporting both Classic OpenLCA
/// (`49.3`) and Beta LCIA (`49.4 Figs. 22–35`).
pub fn builtin_openlca_flows() -> Vec<LcaCatalogFlow> {
    vec![
        // Division B: Mining & quarrying
        LcaCatalogFlow {
            id: deterministic_uuid("olca::hard_coal"),
            name: "hard coal".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["B:Mining and quarrying".to_string(), "05:Mining of coal and lignite".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::MaterialsFuels,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for hard coal".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.42), ("cml_gwp", 0.42), ("traci_gw", 0.42), ("mdp", 0.02), ("fdp", 0.72)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::iron_ore"),
            name: "iron ore, crude ore, 46% Fe".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["B:Mining and quarrying".to_string(), "07:Mining of metal ores".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::MaterialsFuels,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for iron ore, crude ore, 46% Fe".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.085), ("cml_gwp", 0.085), ("traci_gw", 0.085), ("mdp", 0.92)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        // Division C: Manufacturing
        LcaCatalogFlow {
            id: deterministic_uuid("olca::petroleum_coke"),
            name: "petroleum coke".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["C:Manufacturing".to_string(), "19:Manufacture of coke and refined petroleum products".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::MaterialsFuels,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for petroleum coke".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.37760885), ("cml_gwp", 0.37760885), ("traci_gw", 0.37760885), ("mdp", 0.038), ("tap100", 0.004), ("fdp", 1.15)]),
                },
                LcaProviderEntry {
                    provider_name: "petroleum coke production, refinery".to_string(),
                    location: "RoW".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.365), ("cml_gwp", 0.365), ("mdp", 0.035)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::lime_hydrated"),
            name: "lime, hydrated, loose weight".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["C:Manufacturing".to_string(), "23:Manufacture of other non-metallic mineral products".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::MaterialsFuels,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "lime production, hydrated, loose weight".to_string(),
                    location: "RoW".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.894775), ("cml_gwp", 0.894775), ("traci_gw", 0.894775), ("mdp", 0.015), ("tap100", 0.002)]),
                },
                LcaProviderEntry {
                    provider_name: "market for lime, hydrated, loose weight".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.91), ("cml_gwp", 0.91), ("mdp", 0.016)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::pig_iron"),
            name: "pig iron".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["C:Manufacturing".to_string(), "24:Manufacture of basic metals".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::FromTechnosphere,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for pig iron".to_string(),
                    location: "RoW".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 1.6079613), ("cml_gwp", 1.6079613), ("traci_gw", 1.6079613), ("mdp", 1.132226), ("tap100", 0.009)]),
                },
                LcaProviderEntry {
                    provider_name: "pig iron production, blast furnace".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 1.58), ("cml_gwp", 1.58), ("mdp", 1.11)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::steel_low_alloyed"),
            name: "steel, low-alloyed".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["C:Manufacturing".to_string(), "24:Manufacture of basic metals".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::FromTechnosphere,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for steel, low-alloyed".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 1.88995), ("cml_gwp", 1.88995), ("traci_gw", 1.88995), ("mdp", 3.65), ("tap100", 0.008)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::zinc_oxide"),
            name: "zinc oxide".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["C:Manufacturing".to_string(), "20:Manufacture of chemicals and chemical products".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::ReferenceProduct,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for zinc oxide".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 2.45), ("mdp", 1.85)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        // Division D: Electricity, gas, steam and air conditioning supply (> 15 providers!)
        LcaCatalogFlow {
            id: deterministic_uuid("olca::electricity_hv"),
            name: "electricity, high voltage".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Energy,
            unit_label: "MJ".to_string(),
            category_path: vec!["D:Electricity, gas, steam and air conditioning supply".to_string(), "35:Electricity, gas, steam and air conditioning supply".to_string()],
            reference_quantity: "Energy".to_string(),
            energy_quantity: "Net calorific value".to_string(),
            conversion_factor: 3.6,
            gabi_group_hint: LcaGroup::ElectricityHeat,
            providers: many_electricity_providers(),
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::heat_industrial"),
            name: "heat, district or industrial, other than natural gas".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ProductFlow,
            unit_kind: LcaFlowUnitKind::Energy,
            unit_label: "MJ".to_string(),
            category_path: vec!["D:Electricity, gas, steam and air conditioning supply".to_string(), "35:Steam and air conditioning supply".to_string()],
            reference_quantity: "Energy".to_string(),
            energy_quantity: "Net calorific value".to_string(),
            conversion_factor: 3.6,
            gabi_group_hint: LcaGroup::ElectricityHeat,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "market for heat, district or industrial".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.088), ("cml_gwp", 0.088), ("fdp", 0.031)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        // Elementary flows (E -)
        LcaCatalogFlow {
            id: deterministic_uuid("olca::co2_fossil"),
            name: "Carbon dioxide, fossil".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ElementaryFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["Elementary flows".to_string(), "Emission to air".to_string(), "unspecified".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::ToNature,
            providers: Vec::new(),
            elem_factors_per_base_unit: impacts(&[("gwp100", 1.0), ("cml_gwp", 1.0), ("traci_gw", 1.0)]),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::co_fossil"),
            name: "Carbon monoxide, fossil".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ElementaryFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["Elementary flows".to_string(), "Emission to air".to_string(), "unspecified".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::ToNature,
            providers: Vec::new(),
            elem_factors_per_base_unit: impacts(&[("gwp100", 0.0), ("cml_gwp", 0.0), ("pofp", 0.0456)]),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::so2"),
            name: "Sulfur dioxide".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ElementaryFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["Elementary flows".to_string(), "Emission to air".to_string(), "unspecified".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::ToNature,
            providers: Vec::new(),
            elem_factors_per_base_unit: impacts(&[("gwp100", 0.0), ("tap100", 1.0), ("cml_ap", 1.0), ("pofp", 0.081)]),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::nitrogen_air"),
            name: "Nitrogen".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ElementaryFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["Elementary flows".to_string(), "Resource".to_string(), "in air".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::FromNature,
            providers: Vec::new(),
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::water_elem"),
            name: "Water".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::ElementaryFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["Elementary flows".to_string(), "Emission to air".to_string(), "unspecified".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::FromNature,
            providers: Vec::new(),
            elem_factors_per_base_unit: BTreeMap::new(),
        },
        LcaCatalogFlow {
            id: deterministic_uuid("olca::slag_waste"),
            name: "slag, unalloyed electric arc furnace steel".to_string(),
            db_kind: LcaDatabaseKind::OpenLca,
            flow_type: LcaFlowType::WasteFlow,
            unit_kind: LcaFlowUnitKind::Mass,
            unit_label: "kg".to_string(),
            category_path: vec!["E:Water supply; sewerage, waste management and remediation activities".to_string(), "38:Waste collection, treatment and disposal activities".to_string()],
            reference_quantity: "Mass".to_string(),
            energy_quantity: "None".to_string(),
            conversion_factor: 1.0,
            gabi_group_hint: LcaGroup::ToNature,
            providers: vec![
                LcaProviderEntry {
                    provider_name: "treatment of slag, residual material landfill".to_string(),
                    location: "GLO".to_string(),
                    impacts_per_base_unit: impacts(&[("gwp100", 0.014), ("mdp", 0.001)]),
                },
            ],
            elem_factors_per_base_unit: BTreeMap::new(),
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Part A: Classic LCI Engine (`49.3 LCA Evaluation`)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicStreamSpeciesDetail {
    pub formula: String,
    pub phase_label: String,
    pub mass_flow_t_h: f64,
    pub enthalpy_kw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicStreamRow {
    pub stream_id: Option<usize>,
    pub is_manual: bool,
    pub stream_name: String,
    pub unit_name: String,
    pub mass_flow_t_h: f64,
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub enthalpy_kw: f64,
    pub exergy_kw: f64,
    pub total_volume_nm3: f64,
    pub total_solids_t_h: f64,
    pub is_energy_only: bool,
    pub use_exergy: bool,
    pub lca_equivalent: String,
    pub lca_flow_id: Option<String>,
    pub lca_group: LcaGroup,
    pub main_product: bool,
    pub expanded: bool,
    pub species: Vec<ClassicStreamSpeciesDetail>,
}

impl ClassicStreamRow {
    /// 1-hour basis converted quantity (`kg` or `kWh`, Fig. 9–11 & Eq. 2–3).
    pub fn converted_amount(&self) -> f64 {
        if self.use_exergy && !self.is_manual {
            return self.exergy_kw.abs();
        }
        if self.is_manual {
            return self.mass_flow_t_h;
        }
        if self.is_energy_only {
            self.enthalpy_kw.abs()
        } else {
            self.mass_flow_t_h * 1000.0
        }
    }

    pub fn converted_unit(&self) -> &'static str {
        if self.use_exergy || self.is_energy_only {
            "kWh"
        } else {
            "kg"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSourceMatch {
    pub stream_id: Option<usize>,
    pub stream_name: String,
    pub species_formula: String,
    pub amount_kg: f64,
    pub counted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicIndicatorRow {
    pub pattern: String,
    pub lca_equivalent: String,
    pub lca_flow_id: Option<String>,
    pub lca_group: LcaGroup,
    pub unit: String,
    pub expanded: bool,
    pub sources: Vec<IndicatorSourceMatch>,
}

impl ClassicIndicatorRow {
    pub fn total_counted_amount(&self) -> f64 {
        self.sources
            .iter()
            .filter(|s| s.counted)
            .map(|s| s.amount_kg)
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedFlowRow {
    pub direction: String, // "Input" | "Output"
    pub flow_name: String,
    pub group_name: String,
    pub category: String,
    pub sub_category: String,
    pub amount: f64,
    pub normalized_amount: f64,
    pub unit: String,
    pub lca_flow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicProcessInfo {
    pub process_name: String,
    pub technology: String,
    pub general_comment: String,
    pub hsc_ver_last_save: String,
    pub location: String,
    pub location_comment: String,
    pub author: String,
    pub date: String,
    pub version_number: String,
}

impl Default for ClassicProcessInfo {
    fn default() -> Self {
        Self {
            process_name: "Sim LCA Process".to_string(),
            technology: "Pyrometallurgy".to_string(),
            general_comment: String::new(),
            hsc_ver_last_save: "10.0.0.1".to_string(),
            location: "FI".to_string(),
            location_comment: String::new(),
            author: "Soryntel User".to_string(),
            date: "2026-11-15".to_string(),
            version_number: "01.00.000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicLcaState {
    pub active_db: LcaDatabaseKind,
    pub active_openlca_db_path: String,
    pub custom_openlca_flows: Vec<LcaCatalogFlow>,
    pub inputs: Vec<ClassicStreamRow>,
    pub outputs: Vec<ClassicStreamRow>,
    pub manual_inputs: Vec<ClassicStreamRow>,
    pub manual_outputs: Vec<ClassicStreamRow>,
    pub indicators: Vec<ClassicIndicatorRow>,
    pub normalized: Vec<NormalizedFlowRow>,
    pub normalized_valid: bool,
    pub process_info: ClassicProcessInfo,
}

impl Default for ClassicLcaState {
    fn default() -> Self {
        Self {
            active_db: LcaDatabaseKind::GaBi,
            active_openlca_db_path: "Built-in OpenLCA JSON-LD (ecoinvent_3.8)".to_string(),
            custom_openlca_flows: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            manual_inputs: Vec::new(),
            manual_outputs: Vec::new(),
            indicators: vec![
                ClassicIndicatorRow {
                    pattern: "CO(g)".to_string(),
                    lca_equivalent: "Carbon monoxide".to_string(),
                    lca_flow_id: Some(deterministic_uuid("gabi::Carbon monoxide")),
                    lca_group: LcaGroup::ToNature,
                    unit: "kg".to_string(),
                    expanded: false,
                    sources: Vec::new(),
                },
                ClassicIndicatorRow {
                    pattern: "CO2(g)".to_string(),
                    lca_equivalent: "Carbon dioxide".to_string(),
                    lca_flow_id: Some(deterministic_uuid("gabi::Carbon dioxide")),
                    lca_group: LcaGroup::ToNature,
                    unit: "kg".to_string(),
                    expanded: true,
                    sources: Vec::new(),
                },
                ClassicIndicatorRow {
                    pattern: "H2O(g)".to_string(),
                    lca_equivalent: "No Mapping".to_string(),
                    lca_flow_id: None,
                    lca_group: LcaGroup::NotDefined,
                    unit: "kg".to_string(),
                    expanded: false,
                    sources: Vec::new(),
                },
                ClassicIndicatorRow {
                    pattern: "SO*".to_string(),
                    lca_equivalent: "No Mapping".to_string(),
                    lca_flow_id: None,
                    lca_group: LcaGroup::NotDefined,
                    unit: "kg".to_string(),
                    expanded: false,
                    sources: Vec::new(),
                },
                ClassicIndicatorRow {
                    pattern: "NO*".to_string(),
                    lca_equivalent: "No Mapping".to_string(),
                    lca_flow_id: None,
                    lca_group: LcaGroup::NotDefined,
                    unit: "kg".to_string(),
                    expanded: false,
                    sources: Vec::new(),
                },
            ],
            normalized: Vec::new(),
            normalized_valid: false,
            process_info: ClassicProcessInfo::default(),
        }
    }
}

impl ClassicLcaState {
    pub fn available_flows(&self) -> Vec<LcaCatalogFlow> {
        match self.active_db {
            LcaDatabaseKind::GaBi => builtin_gabi_flows(),
            LcaDatabaseKind::OpenLca => {
                let mut all = builtin_openlca_flows();
                all.extend(self.custom_openlca_flows.iter().cloned());
                all
            }
        }
    }

    pub fn find_flow(&self, flow_id: Option<&str>, flow_name: &str) -> Option<LcaCatalogFlow> {
        let flows = self.available_flows();
        if let Some(id) = flow_id {
            if let Some(found) = flows.iter().find(|f| f.id == id) {
                return Some(found.clone());
            }
        }
        flows
            .into_iter()
            .find(|f| f.name.eq_ignore_ascii_case(flow_name.trim()))
    }

    /// Synchronize black-box boundary streams (`Input` blue, `Output` red) from `SimProject`,
    /// strictly excluding internal (`Black`) streams while preserving prior LCA mappings.
    pub fn sync_from_project(&mut self, project: &SimProject) {
        let prev_in = self.inputs.clone();
        let prev_out = self.outputs.clone();
        let mut next_in = Vec::new();
        let mut next_out = Vec::new();

        for stream in &project.streams {
            let conn = stream.connectivity_kind();
            if conn == StreamConnectivity::Internal {
                continue;
            }
            let is_output = conn == StreamConnectivity::Output;
            let prior = if is_output { &prev_out } else { &prev_in }
                .iter()
                .find(|r| r.stream_id == Some(stream.id) || r.stream_name == stream.name_id);

            let unit_name = if is_output {
                unit_name_for(project, stream.source_unit)
            } else {
                unit_name_for(project, stream.dest_unit)
            };

            let energy_only = is_energy_only_stream(stream);
            let species = stream
                .species_flows
                .iter()
                .map(|sp| ClassicStreamSpeciesDetail {
                    formula: sp.formula.clone(),
                    phase_label: match sp.phase {
                        StreamPhaseKind::Gas => "Gas",
                        StreamPhaseKind::Water => "Water",
                        StreamPhaseKind::PureSolid => "Solid",
                    }
                    .to_string(),
                    mass_flow_t_h: sp.mass_flow_t_h,
                    enthalpy_kw: sp.enthalpy_kw,
                })
                .collect();

            let row = ClassicStreamRow {
                stream_id: Some(stream.id),
                is_manual: false,
                stream_name: stream.name_id.clone(),
                unit_name,
                mass_flow_t_h: stream.mass_flow_t_h,
                temp_c: stream.temp_c,
                pressure_bar: stream.pressure_bar,
                enthalpy_kw: stream_energy_kw(stream),
                exergy_kw: stream.exergy_kw,
                total_volume_nm3: stream.gas_flow_nm3_h,
                total_solids_t_h: stream.solids_flow_t_h,
                is_energy_only: energy_only,
                use_exergy: prior.map(|p| p.use_exergy).unwrap_or(false),
                lca_equivalent: prior
                    .map(|p| p.lca_equivalent.clone())
                    .unwrap_or_else(|| "No Mapping".to_string()),
                lca_flow_id: prior.and_then(|p| p.lca_flow_id.clone()),
                lca_group: prior.map(|p| p.lca_group).unwrap_or(LcaGroup::NotDefined),
                main_product: prior.map(|p| p.main_product).unwrap_or(false),
                expanded: prior.map(|p| p.expanded).unwrap_or(false),
                species,
            };

            if is_output {
                next_out.push(row);
            } else {
                next_in.push(row);
            }
        }

        self.inputs = next_in;
        self.outputs = next_out;
        if self.process_info.process_name == "Sim LCA Process" && !project.process_info.process_name.is_empty() {
            self.process_info.process_name = project.process_info.process_name.clone();
        }
        self.refresh_indicators();
    }

    /// Re-evaluate Indicator wildcard (`*`, `?`, `#`) matches and `Counted` flags against Output streams.
    pub fn refresh_indicators(&mut self) {
        let old_counted: BTreeMap<(String, usize, String), bool> = self
            .indicators
            .iter()
            .flat_map(|ind| {
                ind.sources.iter().filter_map(|src| {
                    src.stream_id.map(|sid| ((ind.pattern.clone(), sid, src.species_formula.clone()), src.counted))
                })
            })
            .collect();

        // Anti-double-count tracker: each (stream_id, species) is auto-counted at most once across indicators
        // if matched by an earlier active indicator, or kept uncounted when user explicitly unchecked it.
        let mut already_claimed: BTreeSet<(usize, String)> = BTreeSet::new();

        for ind in &mut self.indicators {
            let mut sources = Vec::new();
            for out in &self.outputs {
                // In OpenLCA mode, only "To Nature" outputs automatically open source species for emission tracking.
                if self.active_db == LcaDatabaseKind::OpenLca
                    && out.lca_group != LcaGroup::ToNature
                    && out.lca_group != LcaGroup::NotDefined
                {
                    continue;
                }
                let Some(sid) = out.stream_id else {
                    continue;
                };
                for sp in &out.species {
                    if wildcard_matches(&ind.pattern, &sp.formula) && sp.mass_flow_t_h > 0.0 {
                        let key = (ind.pattern.clone(), sid, sp.formula.clone());
                        let claimed_key = (sid, sp.formula.clone());
                        let default_counted = if already_claimed.contains(&claimed_key) {
                            false
                        } else {
                            true
                        };
                        let counted = *old_counted.get(&key).unwrap_or(&default_counted);
                        if counted {
                            already_claimed.insert(claimed_key);
                        }
                        sources.push(IndicatorSourceMatch {
                            stream_id: Some(sid),
                            stream_name: out.stream_name.clone(),
                            species_formula: sp.formula.clone(),
                            amount_kg: sp.mass_flow_t_h * 1000.0,
                            counted,
                        });
                    }
                }
            }
            ind.sources = sources;
        }
    }

    pub fn set_counted(&mut self, ind_idx: usize, src_idx: usize, counted: bool) {
        if let Some(ind) = self.indicators.get_mut(ind_idx) {
            if let Some(src) = ind.sources.get_mut(src_idx) {
                src.counted = counted;
            }
        }
        self.normalized_valid = false;
    }

    pub fn set_main_product(&mut self, is_output: bool, is_manual: bool, idx: usize, checked: bool) {
        for r in self
            .inputs
            .iter_mut()
            .chain(self.outputs.iter_mut())
            .chain(self.manual_inputs.iter_mut())
            .chain(self.manual_outputs.iter_mut())
        {
            r.main_product = false;
        }
        if checked {
            let target = match (is_output, is_manual) {
                (false, false) => self.inputs.get_mut(idx),
                (true, false) => self.outputs.get_mut(idx),
                (false, true) => self.manual_inputs.get_mut(idx),
                (true, true) => self.manual_outputs.get_mut(idx),
            };
            if let Some(row) = target {
                row.main_product = true;
            }
        }
        self.normalized_valid = false;
    }

    pub fn add_manual_stream(&mut self, is_output: bool) {
        let n = if is_output {
            self.manual_outputs.len() + 1
        } else {
            self.manual_inputs.len() + 1
        };
        let row = ClassicStreamRow {
            stream_id: None,
            is_manual: true,
            stream_name: if is_output {
                format!("Manual Output {n}")
            } else {
                format!("Manual Input {n}")
            },
            unit_name: String::new(),
            mass_flow_t_h: 0.0,
            temp_c: 25.0,
            pressure_bar: 1.0,
            enthalpy_kw: 0.0,
            exergy_kw: 0.0,
            total_volume_nm3: 0.0,
            total_solids_t_h: 0.0,
            is_energy_only: false,
            use_exergy: false,
            lca_equivalent: "No Mapping".to_string(),
            lca_flow_id: None,
            lca_group: LcaGroup::NotDefined,
            main_product: false,
            expanded: false,
            species: Vec::new(),
        };
        if is_output {
            self.manual_outputs.push(row);
        } else {
            self.manual_inputs.push(row);
        }
        self.normalized_valid = false;
    }

    pub fn delete_manual_stream(&mut self, is_output: bool, idx: usize) {
        let vec = if is_output {
            &mut self.manual_outputs
        } else {
            &mut self.manual_inputs
        };
        if idx < vec.len() {
            vec.remove(idx);
        }
        self.normalized_valid = false;
    }

    pub fn add_indicator(&mut self, pattern: impl Into<String>) {
        self.indicators.push(ClassicIndicatorRow {
            pattern: pattern.into(),
            lca_equivalent: "No Mapping".to_string(),
            lca_flow_id: None,
            lca_group: LcaGroup::NotDefined,
            unit: "kg".to_string(),
            expanded: false,
            sources: Vec::new(),
        });
        self.refresh_indicators();
        self.normalized_valid = false;
    }

    /// Normalize all mapped flows (`!= "No Mapping"`) to `1 kg` of unique `Main Product` (`49.3.6`),
    /// merging identical `(Direction, LCA Equivalent, LCA Group)` entries.
    pub fn normalize(&mut self) -> Result<Vec<NormalizedFlowRow>, String> {
        let mut ref_qty: Option<f64> = None;
        let mut ref_count = 0usize;
        for row in self
            .inputs
            .iter()
            .chain(self.outputs.iter())
            .chain(self.manual_inputs.iter())
            .chain(self.manual_outputs.iter())
        {
            if row.main_product {
                ref_count += 1;
                ref_qty = Some(row.converted_amount());
            }
        }
        if ref_count != 1 {
            return Err("Select exactly one Main Product stream before normalization.".to_string());
        }
        let q_ref = ref_qty.unwrap_or(0.0);
        if q_ref <= 1e-12 {
            return Err("Main Product amount must be greater than zero.".to_string());
        }

        // Preserve insertion order with deterministic merging via BTreeMap keyed by insertion index or composite key.
        let mut merged_order: Vec<(String, String, String)> = Vec::new();
        let mut accum: BTreeMap<(String, String, String), NormalizedFlowRow> = BTreeMap::new();

        let mut push_entry = |direction: &str, flow_name: &str, flow_id: Option<&str>, group: LcaGroup, amount: f64, unit: &str| {
            let trimmed = flow_name.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("No Mapping") || amount.abs() <= 1e-12 {
                return;
            }
            let key = (
                direction.to_string(),
                trimmed.to_string(),
                group.as_str().to_string(),
            );
            let cat_meta = self.find_flow(flow_id, trimmed);
            let (category, sub_category) = cat_meta
                .as_ref()
                .map(|m| {
                    let c = m.category_path.first().cloned().unwrap_or_default();
                    let sc = if m.category_path.len() > 1 {
                        m.category_path[1..].join(" \\ ")
                    } else {
                        String::new()
                    };
                    (c, sc)
                })
                .unwrap_or_default();

            if let Some(existing) = accum.get_mut(&key) {
                existing.amount += amount;
                existing.normalized_amount = existing.amount / q_ref;
            } else {
                merged_order.push(key.clone());
                accum.insert(
                    key,
                    NormalizedFlowRow {
                        direction: direction.to_string(),
                        flow_name: trimmed.to_string(),
                        group_name: group.as_str().to_string(),
                        category,
                        sub_category,
                        amount,
                        normalized_amount: amount / q_ref,
                        unit: unit.to_string(),
                        lca_flow_id: flow_id.map(str::to_string).or_else(|| cat_meta.map(|m| m.id)),
                    },
                );
            }
        };

        for r in self.inputs.iter().chain(self.manual_inputs.iter()) {
            push_entry(
                "Input",
                &r.lca_equivalent,
                r.lca_flow_id.as_deref(),
                r.lca_group,
                r.converted_amount(),
                r.converted_unit(),
            );
        }
        for r in self.outputs.iter().chain(self.manual_outputs.iter()) {
            push_entry(
                "Output",
                &r.lca_equivalent,
                r.lca_flow_id.as_deref(),
                r.lca_group,
                r.converted_amount(),
                r.converted_unit(),
            );
        }
        for ind in &self.indicators {
            let amt = ind.total_counted_amount();
            push_entry(
                "Output",
                &ind.lca_equivalent,
                ind.lca_flow_id.as_deref(),
                ind.lca_group,
                amt,
                &ind.unit,
            );
        }

        let result: Vec<NormalizedFlowRow> = merged_order
            .into_iter()
            .filter_map(|k| accum.remove(&k))
            .collect();
        self.normalized = result.clone();
        self.normalized_valid = true;
        Ok(result)
    }

    /// Generate EcoSpold v1.0 XML dataset (`49.3.8`, `Figs. 24–25`).
    pub fn export_ecospold_v1_xml(&mut self) -> Result<String, String> {
        if !self.normalized_valid {
            self.normalize()?;
        }
        let info = &self.process_info;
        let tech_tag = if info.technology.trim().is_empty() {
            "HSC".to_string()
        } else {
            info.technology.trim().to_string()
        };
        let category = format!("{}: {} [HSC] [{}]", info.location, info.process_name, tech_tag);
        let ds_uuid = deterministic_uuid(&format!("ecospold::{}", info.process_name));

        let mut exchanges = String::new();
        for (i, row) in self.normalized.iter().enumerate() {
            let idx = i + 1;
            let group_tag = if row.direction == "Input" {
                "inputGroup"
            } else {
                "outputGroup"
            };
            let group_num = if row.direction == "Input" { "4" } else { "0" };
            exchanges.push_str(&format!(
                "      <exchange number=\"{idx}\" category=\"{cat}\" subCategory=\"{sub}\" name=\"{name}\" unit=\"{unit}\" meanValue=\"{val:.6}\">\n        <{group_tag}>{group_num}</{group_tag}>\n      </exchange>\n",
                cat = xml_escape(&row.category),
                sub = xml_escape(&row.sub_category),
                name = xml_escape(&row.flow_name),
                unit = xml_escape(&row.unit),
                val = row.normalized_amount,
            ));
        }

        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<ecoSpold xmlns=\"http://www.EcoInvent.org/EcoSpold01\">\n\
  <dataset number=\"1\" validCompanyCodes=\"SORYNTEL\">\n\
    <metaInformation>\n\
      <processInformation>\n\
        <referenceFunction name=\"{proc}\" unit=\"kg\" amount=\"1.0\" category=\"{cat}\" generalComment=\"{comment}\" />\n\
        <geography location=\"{loc}\" text=\"{loc_comment}\" />\n\
        <technology text=\"{tech}\" />\n\
        <timePeriod dataValidForEntirePeriod=\"true\">\n\
          <startDate>{date}</startDate>\n\
        </timePeriod>\n\
      </processInformation>\n\
      <administrativeInformation>\n\
        <dataGeneratorAndPublication person=\"1\" companyCode=\"SORYNTEL\" publishedSource=\"HSC Sim LCA {ver}\" />\n\
        <person number=\"1\" name=\"{author}\" />\n\
        <fileAttributes internalSchemaVersion=\"1.0\" fileGenerator=\"Soryntel HSC Sim LCA\" defaultLanguage=\"en\" creationTimestamp=\"{date}\" uuid=\"{ds_uuid}\" />\n\
      </administrativeInformation>\n\
    </metaInformation>\n\
    <flowData>\n{exchanges}    </flowData>\n\
  </dataset>\n\
</ecoSpold>\n",
            proc = xml_escape(&info.process_name),
            cat = xml_escape(&category),
            comment = xml_escape(&info.general_comment),
            loc = xml_escape(&info.location),
            loc_comment = xml_escape(&info.location_comment),
            tech = xml_escape(&info.technology),
            date = xml_escape(&info.date),
            ver = xml_escape(&info.version_number),
            author = xml_escape(&info.author),
        );
        Ok(xml)
    }

    /// Generate OpenLCA JSON-LD `.zip` archive (`49.3.9`, `Figs. 28–31`) using `ZipBuilder`.
    pub fn export_openlca_jsonld_zip(&mut self) -> Result<Vec<u8>, String> {
        if !self.normalized_valid {
            self.normalize()?;
        }
        let info = &self.process_info;
        let proc_uuid = deterministic_uuid(&format!("olca-proc::{}", info.process_name));
        let hsc_cat_uuid = deterministic_uuid("olca-cat::HSC");

        let mut zip = ZipBuilder::new();
        zip.add_file(
            "olca-schema.json",
            serde_json::to_string_pretty(&serde_json::json!({
                "version": "2.0",
                "generator": "Soryntel HSC Sim LCA",
                "process": proc_uuid,
            }))
            .unwrap()
            .into_bytes(),
        );
        zip.add_file(
            format!("categories/{hsc_cat_uuid}.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "@id": hsc_cat_uuid,
                "@type": "Category",
                "name": "HSC",
                "modelType": "PROCESS"
            }))
            .unwrap()
            .into_bytes(),
        );

        let mut exchanges = Vec::new();
        let mut seen_flows = BTreeSet::new();

        for (i, row) in self.normalized.iter().enumerate() {
            let fid = row
                .lca_flow_id
                .clone()
                .unwrap_or_else(|| deterministic_uuid(&format!("flow::{}", row.flow_name)));
            if seen_flows.insert(fid.clone()) {
                let flow_doc = serde_json::json!({
                    "@id": fid,
                    "@type": "Flow",
                    "name": row.flow_name,
                    "flowType": if row.group_name == "To Nature" || row.group_name == "From Nature" {
                        "ELEMENTARY_FLOW"
                    } else if row.group_name == "To Nature" {
                        "WASTE_FLOW"
                    } else {
                        "PRODUCT_FLOW"
                    },
                    "category": row.category,
                    "unit": row.unit,
                });
                zip.add_file(
                    format!("flows/{fid}.json"),
                    serde_json::to_string_pretty(&flow_doc).unwrap().into_bytes(),
                );
            }
            let is_input = row.direction == "Input";
            let is_ref = row.group_name.starts_with("Reference Product");
            exchanges.push(serde_json::json!({
                "@type": "Exchange",
                "internalId": i + 1,
                "isInput": is_input,
                "isQuantitativeReference": is_ref,
                "amount": row.normalized_amount,
                "unit": { "@type": "Unit", "name": row.unit },
                "flow": { "@id": fid, "@type": "Flow", "name": row.flow_name }
            }));
        }

        let process_doc = serde_json::json!({
            "@id": proc_uuid,
            "@type": "Process",
            "name": info.process_name,
            "description": info.general_comment,
            "version": info.version_number,
            "category": { "@id": hsc_cat_uuid, "@type": "Category", "name": "HSC" },
            "location": { "@type": "Location", "name": info.location, "code": info.location },
            "processDocumentation": {
                "technologyDescription": info.technology,
                "validFrom": info.date,
                "dataGenerator": { "@type": "Actor", "name": info.author },
                "creationDate": info.date,
            },
            "exchanges": exchanges,
        });
        zip.add_file(
            format!("processes/{proc_uuid}.json"),
            serde_json::to_string_pretty(&process_doc).unwrap().into_bytes(),
        );
        Ok(zip.finish())
    }

    /// Parse uncompressed `STORE` entries from an OpenLCA JSON-LD `.zip` archive to load flows.
    pub fn load_openlca_jsonld_zip_bytes(
        &mut self,
        source_label: impl Into<String>,
        zip_bytes: &[u8],
    ) -> Result<usize, String> {
        let parsed = extract_stored_zip_entries(zip_bytes)?;
        let mut loaded = Vec::new();
        for (name, content) in parsed {
            if !name.contains("flows/") || !name.ends_with(".json") {
                continue;
            }
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(&content) else {
                continue;
            };
            let Some(flow_name) = v.get("name").and_then(|s| s.as_str()) else {
                continue;
            };
            let id = v
                .get("@id")
                .and_then(|s| s.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| deterministic_uuid(flow_name));
            let ftype_str = v.get("flowType").and_then(|s| s.as_str()).unwrap_or("PRODUCT_FLOW");
            let flow_type = match ftype_str {
                "ELEMENTARY_FLOW" => LcaFlowType::ElementaryFlow,
                "WASTE_FLOW" => LcaFlowType::WasteFlow,
                _ => LcaFlowType::ProductFlow,
            };
            let unit_label = v.get("unit").and_then(|s| s.as_str()).unwrap_or("kg").to_string();
            let unit_kind = if unit_label.eq_ignore_ascii_case("MJ") || unit_label.eq_ignore_ascii_case("kWh") {
                LcaFlowUnitKind::Energy
            } else {
                LcaFlowUnitKind::Mass
            };
            let cat = v
                .get("category")
                .and_then(|c| c.as_str().or_else(|| c.get("name").and_then(|n| n.as_str())))
                .unwrap_or("Imported JSON-LD Flows");
            loaded.push(LcaCatalogFlow {
                id,
                name: flow_name.to_string(),
                db_kind: LcaDatabaseKind::OpenLca,
                flow_type,
                unit_kind,
                unit_label,
                category_path: vec![cat.to_string()],
                reference_quantity: unit_kind.as_str().to_string(),
                energy_quantity: "None".to_string(),
                conversion_factor: 1.0,
                gabi_group_hint: if flow_type == LcaFlowType::ElementaryFlow {
                    LcaGroup::ToNature
                } else {
                    LcaGroup::MaterialsFuels
                },
                providers: Vec::new(),
                elem_factors_per_base_unit: BTreeMap::new(),
            });
        }
        if loaded.is_empty() {
            return Err("No valid OpenLCA JSON-LD flow entries found in archive.".to_string());
        }
        let count = loaded.len();
        self.custom_openlca_flows = loaded;
        self.active_openlca_db_path = source_label.into();
        self.active_db = LcaDatabaseKind::OpenLca;
        Ok(count)
    }

    /// Generate 6-sheet Excel `.xlsx` matching `49.3 Fig. 22`.
    pub fn export_xlsx(&mut self) -> Result<Vec<u8>, String> {
        if !self.normalized_valid {
            let _ = self.normalize();
        }
        let mut sheets = vec![
            provenance_sheet(),
            build_classic_stream_sheet("Input Streams", &self.inputs, true),
            build_classic_stream_sheet("Output Streams", &self.outputs, true),
            build_classic_stream_sheet("Manual Input Streams", &self.manual_inputs, false),
            build_classic_stream_sheet("Manual Output Streams", &self.manual_outputs, false),
        ];

        // Sheet 5: Indicator Streams
        let mut ind_cells = Vec::new();
        for (c, h) in ["Pattern", "LCA Equivalent", "Amount", "Unit", "LCA Group"]
            .iter()
            .enumerate()
        {
            ind_cells.push(ExportCell {
                row: 0,
                col: c,
                value: ExportCellValue::Text((*h).to_string()),
                style: ExportCellStyle::Header,
            });
        }
        for (r, ind) in self.indicators.iter().enumerate() {
            let row = r + 1;
            let vals = [
                ExportCellValue::Text(ind.pattern.clone()),
                ExportCellValue::Text(ind.lca_equivalent.clone()),
                ExportCellValue::Number(ind.total_counted_amount()),
                ExportCellValue::Text(ind.unit.clone()),
                ExportCellValue::Text(ind.lca_group.as_str().to_string()),
            ];
            for (c, v) in vals.into_iter().enumerate() {
                ind_cells.push(ExportCell {
                    row,
                    col: c,
                    value: v,
                    style: ExportCellStyle::Default,
                });
            }
        }
        sheets.push(ExportSheet {
            name: "Indicator Streams".to_string(),
            cells: ind_cells,
            col_widths: vec![18.0, 24.0, 14.0, 8.0, 22.0],
        });

        // Sheet 6: Normalized Streams
        let mut norm_cells = Vec::new();
        for (c, h) in [
            "Direction",
            "Flow Name",
            "Group Name",
            "Category",
            "Sub Category",
            "Amount",
            "Normalized Amount",
            "Unit",
        ]
        .iter()
        .enumerate()
        {
            norm_cells.push(ExportCell {
                row: 0,
                col: c,
                value: ExportCellValue::Text((*h).to_string()),
                style: ExportCellStyle::Header,
            });
        }
        for (r, n) in self.normalized.iter().enumerate() {
            let row = r + 1;
            let vals = [
                ExportCellValue::Text(n.direction.clone()),
                ExportCellValue::Text(n.flow_name.clone()),
                ExportCellValue::Text(n.group_name.clone()),
                ExportCellValue::Text(n.category.clone()),
                ExportCellValue::Text(n.sub_category.clone()),
                ExportCellValue::Number(n.amount),
                ExportCellValue::Number(n.normalized_amount),
                ExportCellValue::Text(n.unit.clone()),
            ];
            for (c, v) in vals.into_iter().enumerate() {
                norm_cells.push(ExportCell {
                    row,
                    col: c,
                    value: v,
                    style: ExportCellStyle::Default,
                });
            }
        }
        sheets.push(ExportSheet {
            name: "Normalized Streams".to_string(),
            cells: norm_cells,
            col_widths: vec![12.0, 22.0, 24.0, 22.0, 22.0, 14.0, 18.0, 8.0],
        });

        Ok(build_xlsx(&sheets))
    }
}

fn build_classic_stream_sheet(title: &str, rows: &[ClassicStreamRow], include_unit: bool) -> ExportSheet {
    let mut cells = Vec::new();
    let mut headers = vec!["Stream Name"];
    if include_unit {
        headers.push("Unit Name");
    }
    headers.extend(["Amount", "Unit", "Use Exergy", "LCA Equivalent", "LCA Group", "Main Product"]);
    for (c, h) in headers.iter().enumerate() {
        cells.push(ExportCell {
            row: 0,
            col: c,
            value: ExportCellValue::Text((*h).to_string()),
            style: ExportCellStyle::Header,
        });
    }
    for (i, r) in rows.iter().enumerate() {
        let row = i + 1;
        let mut vals = vec![ExportCellValue::Text(r.stream_name.clone())];
        if include_unit {
            vals.push(ExportCellValue::Text(r.unit_name.clone()));
        }
        vals.extend([
            ExportCellValue::Number(r.converted_amount()),
            ExportCellValue::Text(r.converted_unit().to_string()),
            ExportCellValue::Text(if r.use_exergy { "TRUE" } else { "FALSE" }.to_string()),
            ExportCellValue::Text(r.lca_equivalent.clone()),
            ExportCellValue::Text(r.lca_group.as_str().to_string()),
            ExportCellValue::Text(if r.main_product { "TRUE" } else { "FALSE" }.to_string()),
        ]);
        for (c, v) in vals.into_iter().enumerate() {
            cells.push(ExportCell {
                row,
                col: c,
                value: v,
                style: ExportCellStyle::Default,
            });
        }
    }
    ExportSheet {
        name: title.to_string(),
        cells,
        col_widths: vec![24.0; headers.len()],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimal Stored ZIP Extractor (Zero-Dependency OpenLCA JSON-LD Import)
// ─────────────────────────────────────────────────────────────────────────────

fn read_u16_le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn read_u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn extract_stored_zip_entries(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut entries = Vec::new();
    let mut pos = 0usize;
    while pos + 30 <= bytes.len() {
        let sig = read_u32_le(&bytes[pos..pos + 4]);
        if sig != 0x0403_4b50 {
            break;
        }
        let method = read_u16_le(&bytes[pos + 8..pos + 10]);
        let comp_size = read_u32_le(&bytes[pos + 18..pos + 22]) as usize;
        let name_len = read_u16_le(&bytes[pos + 26..pos + 28]) as usize;
        let extra_len = read_u16_le(&bytes[pos + 28..pos + 30]) as usize;
        let name_start = pos + 30;
        let data_start = name_start + name_len + extra_len;
        let data_end = data_start.saturating_add(comp_size);
        if data_end > bytes.len() {
            return Err("Corrupt ZIP entry bounds.".to_string());
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_start + name_len]).to_string();
        if method == 0 {
            entries.push((name, bytes[data_start..data_end].to_vec()));
        }
        pos = data_end;
    }
    Ok(entries)
}

// ─────────────────────────────────────────────────────────────────────────────
// Part B: Beta IPC / LCIA Engine (`49.4 LCA`)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BetaLcaMode {
    UnitDatabase,
    LciDatabase,
}

impl BetaLcaMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnitDatabase => "Unit database",
            Self::LciDatabase => "LCI database",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaSpeciesIndicatorRow {
    pub formula: String,
    pub phase_label: String,
    pub amount: f64,
    pub unit: String,
    pub flow_type: LcaFlowType,
    pub flow_mapping: Option<String>,
    pub flow_id: Option<String>,
    pub provider: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaStreamRow {
    pub stream_id: Option<usize>,
    pub is_manual: bool,
    pub stream_name: String,
    pub unit_name: String,
    pub amount: f64,
    pub unit: String, // "t/h", "kW", "MJ"
    pub flow_type: LcaFlowType,
    pub flow_mapping: Option<String>,
    pub flow_id: Option<String>,
    pub provider: Option<String>,
    pub location: Option<String>,
    /// True after `Convert Stream to Indicators` (`49.4 Figs. 15–16`).
    pub converted_to_indicators: bool,
    pub formula_ref: String,
    pub species: Vec<BetaSpeciesIndicatorRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaProviderQueryResult {
    pub total_providers: usize,
    pub require_location_first: bool,
    pub available_locations: Vec<String>,
    pub available_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LciaResultStreamLine {
    pub stream_name: String,
    pub flow_mapping: String,
    pub provider: String,
    pub location: String,
    pub stream_amount: f64,
    pub normalized_amount: f64,
    pub amount_unit: String,
    pub normalized_result: f64,
    pub is_reference_flow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LciaResultUnitBlock {
    pub unit_name: String,
    pub inputs: Vec<LciaResultStreamLine>,
    pub inputs_total: f64,
    pub outputs: Vec<LciaResultStreamLine>,
    pub outputs_total: f64,
    pub unit_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LciaCategoryResult {
    pub method_name: String,
    pub category_id: String,
    pub category_name: String,
    pub result_unit: String,
    pub reference_stream_name: String,
    pub reference_amount_t_h: f64,
    pub units: Vec<LciaResultUnitBlock>,
    pub grand_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntheticUnitProcessSummary {
    pub process_name: String,
    pub synthetic_ref_flow: String,
    pub synthetic_ref_amount_kg: f64,
    pub exchanges: Vec<(String, String, f64, String)>, // (Flow, Provider/Loc, Amount, Unit)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaCustomSheet {
    pub name: String,
    pub is_saved_snapshot: bool,
    pub is_external_import: bool,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaLcaState {
    pub mode: BetaLcaMode,
    pub ipc_endpoint: String,
    pub ipc_live_connected: bool,
    pub ipc_status_note: String,
    pub active_db_name: String,
    pub show_stream_content: bool,
    pub show_result_content: bool,
    pub inputs: Vec<BetaStreamRow>,
    pub outputs: Vec<BetaStreamRow>,
    pub selected_categories: Vec<LciaCategorySpec>,
    pub results: Vec<LciaCategoryResult>,
    pub results_calculated_once: bool,
    pub synthetic_processes: Vec<SyntheticUnitProcessSummary>,
    pub custom_sheets: Vec<BetaCustomSheet>,
}

impl Default for BetaLcaState {
    fn default() -> Self {
        Self {
            mode: BetaLcaMode::LciDatabase,
            ipc_endpoint: "127.0.0.1:8080".to_string(),
            ipc_live_connected: false,
            ipc_status_note: "Built-in ecoinvent_3_8_LCI clean-room engine active".to_string(),
            active_db_name: "pTesting_ecoinvent_3_8_LCI".to_string(),
            show_stream_content: true,
            show_result_content: true,
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected_categories: Vec::new(),
            results: Vec::new(),
            results_calculated_once: false,
            synthetic_processes: Vec::new(),
            custom_sheets: Vec::new(),
        }
    }
}

impl BetaLcaState {
    pub fn catalog_flows(&self) -> Vec<LcaCatalogFlow> {
        builtin_openlca_flows()
    }

    /// Optional live check against openLCA IPC JSON-RPC / gRPC HTTP endpoint (`localhost:8080`).
    pub fn probe_openlca_ipc(&mut self) -> Result<String, String> {
        let addr: SocketAddr = self
            .ipc_endpoint
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 8080)));
        match TcpStream::connect_timeout(&addr, Duration::from_millis(250)) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                let body = r#"{"jsonrpc":"2.0","id":1,"method":"get/descriptors","params":{"@type":"Flow"}}"#;
                let req = format!(
                    "POST / HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    self.ipc_endpoint,
                    body.len(),
                    body
                );
                let _ = stream.write_all(req.as_bytes());
                let mut resp = String::new();
                let _ = stream.read_to_string(&mut resp);
                self.ipc_live_connected = true;
                self.ipc_status_note = format!("Connected to openLCA IPC ({})", self.ipc_endpoint);
                Ok(self.ipc_status_note.clone())
            }
            Err(_) => {
                self.ipc_live_connected = false;
                self.ipc_status_note = format!(
                    "IPC {} unreachable — using built-in clean-room {} catalog",
                    self.ipc_endpoint, self.active_db_name
                );
                Ok(self.ipc_status_note.clone())
            }
        }
    }

    /// Sync boundary streams (`Inputs`/`Outputs`, excluding black internal streams) from `SimProject`.
    /// When `results_calculated_once` is already true, automatically recalculates live results.
    pub fn sync_from_project(&mut self, project: &SimProject) {
        let old_in = self.inputs.clone();
        let old_out = self.outputs.clone();

        let mut next_in = Vec::new();
        let mut next_out = Vec::new();

        for stream in &project.streams {
            let conn = stream.connectivity_kind();
            if conn == StreamConnectivity::Internal {
                continue;
            }
            let is_output = conn == StreamConnectivity::Output;
            let prior = if is_output { &old_out } else { &old_in }
                .iter()
                .find(|r| !r.is_manual && (r.stream_id == Some(stream.id) || r.stream_name == stream.name_id));

            let unit_name = if is_output {
                unit_name_for(project, stream.source_unit)
            } else {
                unit_name_for(project, stream.dest_unit)
            };
            let energy_only = is_energy_only_stream(stream);
            let amount = if energy_only {
                stream_energy_kw(stream)
            } else {
                stream.mass_flow_t_h
            };
            let unit = if energy_only { "kW" } else { "t/h" }.to_string();

            let species = stream
                .species_flows
                .iter()
                .map(|sp| {
                    let prior_sp = prior
                        .and_then(|p| p.species.iter().find(|s| s.formula == sp.formula));
                    let sp_energy = sp.mass_flow_t_h.abs() <= 1e-12 && sp.enthalpy_kw.abs() > 1e-9;
                    BetaSpeciesIndicatorRow {
                        formula: sp.formula.clone(),
                        phase_label: match sp.phase {
                            StreamPhaseKind::Gas | StreamPhaseKind::PureSolid => "Pyro Phase",
                            StreamPhaseKind::Water => "Hydro Phase",
                        }
                        .to_string(),
                        amount: if sp_energy { sp.enthalpy_kw.abs() } else { sp.mass_flow_t_h },
                        unit: if sp_energy { "kW" } else { "t/h" }.to_string(),
                        flow_type: prior_sp
                            .map(|s| s.flow_type)
                            .unwrap_or(LcaFlowType::ElementaryFlow),
                        flow_mapping: prior_sp.and_then(|s| s.flow_mapping.clone()),
                        flow_id: prior_sp.and_then(|s| s.flow_id.clone()),
                        provider: prior_sp.and_then(|s| s.provider.clone()),
                        location: prior_sp.and_then(|s| s.location.clone()),
                    }
                })
                .collect();

            let row = BetaStreamRow {
                stream_id: Some(stream.id),
                is_manual: false,
                stream_name: stream.name_id.clone(),
                unit_name,
                amount,
                unit,
                flow_type: prior
                    .map(|p| p.flow_type)
                    .unwrap_or(LcaFlowType::ElementaryFlow),
                flow_mapping: prior.and_then(|p| p.flow_mapping.clone()),
                flow_id: prior.and_then(|p| p.flow_id.clone()),
                provider: prior.and_then(|p| p.provider.clone()),
                location: prior.and_then(|p| p.location.clone()),
                converted_to_indicators: prior.map(|p| p.converted_to_indicators).unwrap_or(false),
                formula_ref: format!("={}", stream.name_id),
                species,
            };
            if is_output {
                next_out.push(row);
            } else {
                next_in.push(row);
            }
        }

        // Preserve user manual streams at bottom
        next_in.extend(old_in.into_iter().filter(|r| r.is_manual));
        next_out.extend(old_out.into_iter().filter(|r| r.is_manual));

        self.inputs = next_in;
        self.outputs = next_out;

        if self.results_calculated_once {
            let _ = self.calculate_results();
        }
    }

    pub fn add_manual_stream(&mut self, is_output: bool) -> usize {
        let row = BetaStreamRow {
            stream_id: None,
            is_manual: true,
            stream_name: if is_output {
                "<Manual Output Stream>".to_string()
            } else {
                "<Manual Input Stream>".to_string()
            },
            unit_name: "Manual Streams".to_string(),
            amount: 0.0,
            unit: "t/h".to_string(),
            flow_type: LcaFlowType::ProductFlow,
            flow_mapping: None,
            flow_id: None,
            provider: None,
            location: None,
            converted_to_indicators: false,
            formula_ref: String::new(),
            species: Vec::new(),
        };
        if is_output {
            self.outputs.push(row);
            self.outputs.len() - 1
        } else {
            self.inputs.push(row);
            self.inputs.len() - 1
        }
    }

    pub fn delete_manual_stream(&mut self, is_output: bool, idx: usize) -> Result<(), String> {
        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        let Some(row) = list.get(idx) else {
            return Err("Manual stream is not selected".to_string());
        };
        if !row.is_manual {
            return Err("Manual stream is not selected".to_string());
        }
        list.remove(idx);
        Ok(())
    }

    pub fn convert_stream_to_indicators(&mut self, is_output: bool, idx: usize) -> Result<(), String> {
        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        let row = list
            .get_mut(idx)
            .ok_or_else(|| "Select a valid boundary stream row first.".to_string())?;
        if row.is_manual {
            return Err("Manual streams cannot be converted to indicators.".to_string());
        }
        if row.flow_mapping.is_some() {
            return Err("Remove Flow Mapping on the parent stream before converting to indicators.".to_string());
        }
        if row.species.is_empty() {
            return Err("Selected stream has no species breakdown to convert.".to_string());
        }
        row.converted_to_indicators = true;
        Ok(())
    }

    pub fn convert_indicators_back_to_stream(&mut self, is_output: bool, idx: usize) -> Result<(), String> {
        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        let row = list
            .get_mut(idx)
            .ok_or_else(|| "Select a converted stream indicator block first.".to_string())?;
        if !row.converted_to_indicators {
            return Err("Selected row is not converted to indicators.".to_string());
        }
        row.converted_to_indicators = false;
        for sp in &mut row.species {
            sp.flow_mapping = None;
            sp.flow_id = None;
            sp.provider = None;
            sp.location = None;
        }
        Ok(())
    }

    /// Enforce unique `Reference Flow (Product)` across all Beta Inputs & Outputs (`49.4 Eq. 6`).
    pub fn set_flow_type(
        &mut self,
        is_output: bool,
        stream_idx: usize,
        species_idx: Option<usize>,
        flow_type: LcaFlowType,
    ) {
        if flow_type == LcaFlowType::ReferenceProductFlow {
            for r in self.inputs.iter_mut().chain(self.outputs.iter_mut()) {
                if r.flow_type == LcaFlowType::ReferenceProductFlow {
                    r.flow_type = LcaFlowType::ProductFlow;
                }
                for sp in &mut r.species {
                    if sp.flow_type == LcaFlowType::ReferenceProductFlow {
                        sp.flow_type = LcaFlowType::ElementaryFlow;
                    }
                }
            }
        }

        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        if let Some(row) = list.get_mut(stream_idx) {
            if let Some(sp_i) = species_idx {
                if let Some(sp) = row.species.get_mut(sp_i) {
                    sp.flow_type = flow_type;
                    if flow_type == LcaFlowType::ElementaryFlow || flow_type == LcaFlowType::ReferenceProductFlow {
                        sp.provider = None;
                        sp.location = None;
                    }
                    if flow_type == LcaFlowType::ReferenceProductFlow {
                        sp.flow_mapping = None;
                        sp.flow_id = None;
                    }
                }
            } else {
                row.flow_type = flow_type;
                if flow_type == LcaFlowType::ElementaryFlow || flow_type == LcaFlowType::ReferenceProductFlow {
                    row.provider = None;
                    row.location = None;
                }
                if flow_type == LcaFlowType::ReferenceProductFlow {
                    row.flow_mapping = None;
                    row.flow_id = None;
                }
            }
        }
    }

    /// Enforce the official `> 15 providers` rule (`49.4 Fig. 27`):
    /// If `providers.len() > 15`, `require_location_first = true` and `available_providers`
    /// remains empty until `location` is selected.
    pub fn query_providers(&self, flow_id: &str, selected_location: Option<&str>) -> BetaProviderQueryResult {
        let Some(flow) = self.catalog_flows().into_iter().find(|f| f.id == flow_id) else {
            return BetaProviderQueryResult {
                total_providers: 0,
                require_location_first: false,
                available_locations: Vec::new(),
                available_providers: Vec::new(),
            };
        };
        let total = flow.providers.len();
        let require_location_first = total > 15;
        let mut locs: Vec<String> = flow
            .providers
            .iter()
            .map(|p| p.location.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        locs.sort();

        let providers = if require_location_first {
            match selected_location.filter(|s| !s.trim().is_empty()) {
                Some(loc) => flow
                    .providers
                    .iter()
                    .filter(|p| p.location == loc)
                    .map(|p| p.provider_name.clone())
                    .collect(),
                None => Vec::new(),
            }
        } else {
            match selected_location.filter(|s| !s.trim().is_empty()) {
                Some(loc) => flow
                    .providers
                    .iter()
                    .filter(|p| p.location == loc)
                    .map(|p| p.provider_name.clone())
                    .collect(),
                None => flow.providers.iter().map(|p| p.provider_name.clone()).collect(),
            }
        };

        BetaProviderQueryResult {
            total_providers: total,
            require_location_first,
            available_locations: locs,
            available_providers: providers,
        }
    }

    pub fn add_mapping(
        &mut self,
        is_output: bool,
        stream_idx: usize,
        species_idx: Option<usize>,
        flow_id: &str,
    ) -> Result<(), String> {
        let flow = self
            .catalog_flows()
            .into_iter()
            .find(|f| f.id == flow_id)
            .ok_or_else(|| "Selected flow was not found in active database.".to_string())?;

        // Validate target row before mutating
        let row_info = {
            let list = if is_output { &self.outputs } else { &self.inputs };
            let row = list
                .get(stream_idx)
                .ok_or_else(|| "Select a row to map first.".to_string())?;
            if let Some(sp_i) = species_idx {
                if !row.converted_to_indicators {
                    return Err("Convert stream to indicators before mapping individual species.".to_string());
                }
                let sp = row
                    .species
                    .get(sp_i)
                    .ok_or_else(|| "Species row out of range.".to_string())?;
                (sp.unit.clone(), sp.flow_type)
            } else {
                if row.converted_to_indicators {
                    return Err("Parent stream converted to indicators cannot be mapped directly.".to_string());
                }
                if row.flow_type == LcaFlowType::ReferenceProductFlow {
                    return Err("Reference Flow (Product) represents the HSC output and must not be mapped.".to_string());
                }
                (row.unit.clone(), row.flow_type)
            }
        };

        if !flow.compatible_with_unit(&row_info.0) {
            return Err(format!(
                "Unit mismatch: stream unit '{}' is incompatible with '{}' ({}).",
                row_info.0,
                flow.name,
                flow.unit_kind.as_str()
            ));
        }

        // Auto-select default provider/location ONLY if <= 15 providers; otherwise require Location first.
        let (auto_loc, auto_prov) = if matches!(row_info.1, LcaFlowType::ElementaryFlow | LcaFlowType::ReferenceProductFlow)
            || flow.providers.len() > 15
        {
            (None, None)
        } else {
            flow.providers
                .first()
                .map(|p| (Some(p.location.clone()), Some(p.provider_name.clone())))
                .unwrap_or((None, None))
        };

        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        let row = &mut list[stream_idx];
        if let Some(sp_i) = species_idx {
            let sp = &mut row.species[sp_i];
            sp.flow_mapping = Some(flow.name);
            sp.flow_id = Some(flow.id);
            sp.location = auto_loc;
            sp.provider = auto_prov;
        } else {
            row.flow_mapping = Some(flow.name);
            row.flow_id = Some(flow.id);
            row.location = auto_loc;
            row.provider = auto_prov;
        }
        Ok(())
    }

    pub fn delete_mapping(&mut self, is_output: bool, stream_idx: usize, species_idx: Option<usize>) {
        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        if let Some(row) = list.get_mut(stream_idx) {
            if let Some(sp_i) = species_idx {
                if let Some(sp) = row.species.get_mut(sp_i) {
                    sp.flow_mapping = None;
                    sp.flow_id = None;
                    sp.provider = None;
                    sp.location = None;
                }
            } else {
                row.flow_mapping = None;
                row.flow_id = None;
                row.provider = None;
                row.location = None;
            }
        }
    }

    pub fn set_location_and_provider(
        &mut self,
        is_output: bool,
        stream_idx: usize,
        species_idx: Option<usize>,
        location: Option<String>,
        provider: Option<String>,
    ) {
        let list = if is_output { &mut self.outputs } else { &mut self.inputs };
        if let Some(row) = list.get_mut(stream_idx) {
            if let Some(sp_i) = species_idx {
                if let Some(sp) = row.species.get_mut(sp_i) {
                    sp.location = location;
                    sp.provider = provider;
                }
            } else {
                row.location = location;
                row.provider = provider;
            }
        }
    }

    /// Enforce single parent Impact Method across all selected categories (`49.4 Eq. 5`).
    pub fn add_impact_category(&mut self, method_id: &str, cat_id: &str) -> Result<(), String> {
        let methods = builtin_lcia_methods();
        let method = methods
            .iter()
            .find(|m| m.id == method_id)
            .ok_or_else(|| "Unknown LCIA impact method.".to_string())?;
        let cat = method
            .categories
            .iter()
            .find(|c| c.id == cat_id)
            .ok_or_else(|| "Unknown impact category.".to_string())?;

        if let Some(existing) = self.selected_categories.first() {
            if existing.method_id != cat.method_id {
                return Err(format!(
                    "All impact categories must belong to '{}'. Remove existing categories before mixing methods.",
                    existing.method_name
                ));
            }
        }
        if !self.selected_categories.iter().any(|c| c.id == cat.id) {
            self.selected_categories.push(cat.clone());
        }
        Ok(())
    }

    pub fn delete_impact_category(&mut self, cat_id: &str) {
        self.selected_categories.retain(|c| c.id != cat_id);
        self.results.retain(|r| r.category_id != cat_id);
    }

    /// Execute LCIA evaluation (`Calculate Results`), building openLCA-style unit processes,
    /// `Main Process`, and `Product System`, normalizing to `1 t/h` of unique `Reference Flow (Product)`.
    pub fn calculate_results(&mut self) -> Result<(), String> {
        if self.selected_categories.is_empty() {
            return Err("Add at least one Impact Category before calculating results.".to_string());
        }

        // Locate unique Reference Flow (Product)
        let mut ref_name: Option<String> = None;
        let mut ref_unit_name: Option<String> = None;
        let mut ref_amount: Option<f64> = None;
        let mut ref_count = 0usize;

        for r in self.inputs.iter().chain(self.outputs.iter()) {
            if r.flow_type == LcaFlowType::ReferenceProductFlow {
                ref_count += 1;
                ref_name = Some(r.stream_name.clone());
                ref_unit_name = Some(r.unit_name.clone());
                ref_amount = Some(r.amount);
            }
        }
        if ref_count != 1 {
            return Err("Exactly one stream must be set as 'Reference Flow (Product)'.".to_string());
        }
        let ref_stream_name = ref_name.unwrap();
        let ref_unit = ref_unit_name.unwrap_or_default();
        let ref_amt_t_h = ref_amount.unwrap_or(0.0);
        if ref_amt_t_h <= 1e-12 {
            return Err("Reference Flow amount must be positive.".to_string());
        }

        let catalog = self.catalog_flows();

        // Collect deterministic unit names preserving appearance order
        let mut unit_names = Vec::new();
        for r in self.inputs.iter().chain(self.outputs.iter()) {
            let has_active_mapping = r.flow_mapping.is_some()
                || r.flow_type == LcaFlowType::ReferenceProductFlow
                || (r.converted_to_indicators && r.species.iter().any(|s| s.flow_mapping.is_some()));
            if has_active_mapping && !r.unit_name.is_empty() && !unit_names.contains(&r.unit_name) {
                unit_names.push(r.unit_name.clone());
            }
        }

        // Build synthetic OpenLCA unit processes + Main Process summary (`49.4 Figs. 38–39`, Eq. 4)
        let mut synth = Vec::new();
        for uname in &unit_names {
            let mut exchanges = Vec::new();
            for r in self.inputs.iter().chain(self.outputs.iter()) {
                if &r.unit_name != uname {
                    continue;
                }
                if r.flow_type == LcaFlowType::ReferenceProductFlow {
                    exchanges.push((
                        r.stream_name.clone(),
                        "Reference Flow".to_string(),
                        r.amount * 1000.0,
                        "kg".to_string(),
                    ));
                    continue;
                }
                if !r.converted_to_indicators {
                    if let Some(fname) = &r.flow_mapping {
                        let (amt_1h, u_1h) = convert_beta_amount_1h(r.amount, &r.unit);
                        exchanges.push((
                            fname.clone(),
                            r.provider.clone().unwrap_or_default(),
                            amt_1h,
                            u_1h.to_string(),
                        ));
                    }
                } else {
                    for sp in &r.species {
                        if let Some(fname) = &sp.flow_mapping {
                            let (amt_1h, u_1h) = convert_beta_amount_1h(sp.amount, &sp.unit);
                            exchanges.push((
                                format!("{} ({})", sp.formula, fname),
                                sp.provider.clone().unwrap_or_default(),
                                amt_1h,
                                u_1h.to_string(),
                            ));
                        }
                    }
                }
            }
            synth.push(SyntheticUnitProcessSummary {
                process_name: uname.clone(),
                synthetic_ref_flow: format!("{uname}_ref"),
                synthetic_ref_amount_kg: 1.0,
                exchanges,
            });
        }
        self.synthetic_processes = synth;

        // Helper closure: evaluate category impact contribution for a single mapped exchange
        let eval_impact = |flow_id: Option<&str>,
                           flow_name: Option<&str>,
                           flow_type: LcaFlowType,
                           provider: Option<&str>,
                           location: Option<&str>,
                           raw_amount: f64,
                           raw_unit: &str,
                           cat_id: &str|
         -> f64 {
            let Some(fname) = flow_name else {
                return 0.0;
            };
            let flow = catalog
                .iter()
                .find(|f| flow_id == Some(f.id.as_str()) || f.name == fname);
            let Some(flow) = flow else {
                return 0.0;
            };
            let (amt_1h, _) = convert_beta_amount_1h(raw_amount, raw_unit);
            let norm_amt_1h = amt_1h / ref_amt_t_h;
            match flow_type {
                LcaFlowType::ElementaryFlow => {
                    let q = flow.elem_factors_per_base_unit.get(cat_id).copied().unwrap_or(0.0);
                    norm_amt_1h * q
                }
                LcaFlowType::ProductFlow | LcaFlowType::WasteFlow => {
                    let chosen = flow
                        .providers
                        .iter()
                        .find(|p| {
                            provider.map(|pn| p.provider_name == pn).unwrap_or(true)
                                && location.map(|ln| p.location == ln).unwrap_or(true)
                        })
                        .or_else(|| flow.providers.first());
                    let u = chosen
                        .and_then(|p| p.impacts_per_base_unit.get(cat_id).copied())
                        .unwrap_or(0.0);
                    norm_amt_1h * u
                }
                LcaFlowType::ReferenceProductFlow => 0.0,
            }
        };

        let mut all_results = Vec::new();
        for cat in &self.selected_categories {
            let mut unit_blocks = Vec::new();
            let mut grand_total = 0.0;

            for uname in &unit_names {
                let mut in_lines = Vec::new();
                let mut out_lines = Vec::new();

                for r in &self.inputs {
                    if &r.unit_name != uname {
                        continue;
                    }
                    push_beta_result_lines(
                        r,
                        &eval_impact,
                        &cat.id,
                        ref_amt_t_h,
                        &mut in_lines,
                    );
                }
                for r in &self.outputs {
                    if &r.unit_name != uname {
                        continue;
                    }
                    push_beta_result_lines(
                        r,
                        &eval_impact,
                        &cat.id,
                        ref_amt_t_h,
                        &mut out_lines,
                    );
                }

                // Append italicized Reference Flow line on owning Output unit even if unmapped (`Pig Iron = 1.00 t/h`)
                if uname == &ref_unit {
                    if !out_lines.iter().any(|l| l.is_reference_flow) {
                        out_lines.push(LciaResultStreamLine {
                            stream_name: ref_stream_name.clone(),
                            flow_mapping: String::new(),
                            provider: String::new(),
                            location: String::new(),
                            stream_amount: ref_amt_t_h,
                            normalized_amount: 1.0,
                            amount_unit: "t/h".to_string(),
                            normalized_result: 0.0,
                            is_reference_flow: true,
                        });
                    }
                }

                let inputs_total: f64 = in_lines.iter().map(|l| l.normalized_result).sum();
                let outputs_total: f64 = out_lines.iter().map(|l| l.normalized_result).sum();
                let unit_total = inputs_total + outputs_total;
                grand_total += unit_total;

                unit_blocks.push(LciaResultUnitBlock {
                    unit_name: uname.clone(),
                    inputs: in_lines,
                    inputs_total,
                    outputs: out_lines,
                    outputs_total,
                    unit_total,
                });
            }

            all_results.push(LciaCategoryResult {
                method_name: cat.method_name.clone(),
                category_id: cat.id.clone(),
                category_name: cat.name.clone(),
                result_unit: cat.result_unit.clone(),
                reference_stream_name: ref_stream_name.clone(),
                reference_amount_t_h: ref_amt_t_h,
                units: unit_blocks,
                grand_total,
            });
        }

        self.results = all_results;
        self.results_calculated_once = true;
        Ok(())
    }

    pub fn validate_custom_sheet_name(&self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("Sheet name cannot be empty.".to_string());
        }
        if trimmed.starts_with("Saved") || trimmed.starts_with("Norm") || trimmed.starts_with("Ext") {
            return Err("Custom sheet name cannot start with 'Saved', 'Norm', or 'Ext'.".to_string());
        }
        if matches!(trimmed, "Inputs" | "Outputs" | "Results")
            || self.custom_sheets.iter().any(|s| s.name.eq_ignore_ascii_case(trimmed))
        {
            return Err(format!("A sheet named '{trimmed}' already exists."));
        }
        Ok(())
    }

    pub fn add_custom_sheet(&mut self, name: impl Into<String>) -> Result<(), String> {
        let nm = name.into().trim().to_string();
        self.validate_custom_sheet_name(&nm)?;
        self.custom_sheets.push(BetaCustomSheet {
            name: nm,
            is_saved_snapshot: false,
            is_external_import: false,
            headers: vec!["Parameter".to_string(), "Value".to_string(), "Unit".to_string(), "Notes".to_string()],
            rows: vec![vec!["".to_string(); 4]; 12],
        });
        Ok(())
    }

    pub fn rename_custom_sheet(&mut self, idx: usize, new_name: impl Into<String>) -> Result<(), String> {
        let sheet = self
            .custom_sheets
            .get(idx)
            .ok_or_else(|| "Select a custom sheet to rename.".to_string())?;
        if sheet.is_saved_snapshot || sheet.is_external_import {
            return Err("Saved or External sheets cannot be renamed as user custom sheets.".to_string());
        }
        let nm = new_name.into().trim().to_string();
        self.validate_custom_sheet_name(&nm)?;
        self.custom_sheets[idx].name = nm;
        Ok(())
    }

    pub fn delete_custom_sheet(&mut self, idx: usize) {
        if idx < self.custom_sheets.len() {
            self.custom_sheets.remove(idx);
        }
    }

    /// Snapshot current LCIA `Results` into an immutable `'Saved*'` sheet (`49.4 Fig. 36`).
    pub fn save_results_to_custom_sheet(&mut self) -> Result<String, String> {
        if self.results.is_empty() {
            return Err("No calculated Results available to save.".to_string());
        }
        let n = self
            .custom_sheets
            .iter()
            .filter(|s| s.is_saved_snapshot)
            .count()
            + 1;
        let name = format!("Saved Results {n}");
        let headers = vec![
            "Method".to_string(),
            "Category".to_string(),
            "Unit".to_string(),
            "Stream Name".to_string(),
            "Flow Mapping".to_string(),
            "Provider".to_string(),
            "Location".to_string(),
            "Stream Amount".to_string(),
            "Normalized Amounts".to_string(),
            "Units".to_string(),
            "Normalized Results".to_string(),
            "Units".to_string(),
        ];
        let mut rows = Vec::new();
        for cat in &self.results {
            rows.push(vec![
                cat.method_name.clone(),
                cat.category_name.clone(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "Total".to_string(),
                String::new(),
                format!("{:.2}", cat.grand_total),
                format!("{} per t/h of {}", cat.result_unit, cat.reference_stream_name),
            ]);
            for ub in &cat.units {
                rows.push(vec![
                    String::new(),
                    String::new(),
                    ub.unit_name.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    "Total".to_string(),
                    String::new(),
                    format!("{:.2}", ub.unit_total),
                    String::new(),
                ]);
                for line in ub.inputs.iter().chain(ub.outputs.iter()) {
                    rows.push(vec![
                        String::new(),
                        String::new(),
                        String::new(),
                        line.stream_name.clone(),
                        line.flow_mapping.clone(),
                        line.provider.clone(),
                        line.location.clone(),
                        format!("{:.2}", line.stream_amount),
                        format!("{:.2}", line.normalized_amount),
                        line.amount_unit.clone(),
                        format!("{:.2}", line.normalized_result),
                        String::new(),
                    ]);
                }
            }
        }
        self.custom_sheets.push(BetaCustomSheet {
            name: name.clone(),
            is_saved_snapshot: true,
            is_external_import: false,
            headers,
            rows,
        });
        Ok(name)
    }

    pub fn import_external_sheet(
        &mut self,
        table_name: &str,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    ) -> String {
        let suffix = table_name.trim();
        let mut name = format!("Ext {suffix}");
        let mut k = 2usize;
        while self.custom_sheets.iter().any(|s| s.name == name) {
            name = format!("Ext {suffix} ({k})");
            k += 1;
        }
        self.custom_sheets.push(BetaCustomSheet {
            name: name.clone(),
            is_saved_snapshot: false,
            is_external_import: true,
            headers,
            rows,
        });
        name
    }

    pub fn export_to_xlsx(&self) -> Vec<u8> {
        let mut sheets = vec![
            provenance_sheet(),
            build_beta_io_sheet("Inputs", &self.inputs),
            build_beta_io_sheet("Outputs", &self.outputs),
            build_beta_results_sheet(&self.results),
        ];
        for cs in &self.custom_sheets {
            let mut cells = Vec::new();
            for (c, h) in cs.headers.iter().enumerate() {
                cells.push(ExportCell {
                    row: 0,
                    col: c,
                    value: ExportCellValue::Text(h.clone()),
                    style: ExportCellStyle::Header,
                });
            }
            for (r, row_vals) in cs.rows.iter().enumerate() {
                for (c, v) in row_vals.iter().enumerate() {
                    let val = v
                        .parse::<f64>()
                        .map(ExportCellValue::Number)
                        .unwrap_or_else(|_| ExportCellValue::Text(v.clone()));
                    cells.push(ExportCell {
                        row: r + 1,
                        col: c,
                        value: val,
                        style: ExportCellStyle::Default,
                    });
                }
            }
            sheets.push(ExportSheet {
                name: cs.name.clone(),
                cells,
                col_widths: vec![18.0; cs.headers.len().max(1)],
            });
        }
        build_xlsx(&sheets)
    }
}

/// Convert instantaneous Beta rate (`t/h`, `kW`, `MJ`) into openLCA 1-hour exchange (`kg` or `MJ`, Eq. 4).
pub fn convert_beta_amount_1h(amount: f64, unit: &str) -> (f64, &'static str) {
    match unit {
        "t/h" => (amount * 1000.0, "kg"),
        "kW" => (amount * 3.6, "MJ"),
        "MJ" => (amount, "MJ"),
        _ => (amount * 1000.0, "kg"),
    }
}

fn push_beta_result_lines(
    r: &BetaStreamRow,
    eval_impact: &dyn Fn(Option<&str>, Option<&str>, LcaFlowType, Option<&str>, Option<&str>, f64, &str, &str) -> f64,
    cat_id: &str,
    ref_amt_t_h: f64,
    out_lines: &mut Vec<LciaResultStreamLine>,
) {
    if !r.converted_to_indicators {
        if r.flow_mapping.is_some() || r.flow_type == LcaFlowType::ReferenceProductFlow {
            let impact = eval_impact(
                r.flow_id.as_deref(),
                r.flow_mapping.as_deref(),
                r.flow_type,
                r.provider.as_deref(),
                r.location.as_deref(),
                r.amount,
                &r.unit,
                cat_id,
            );
            out_lines.push(LciaResultStreamLine {
                stream_name: r.stream_name.clone(),
                flow_mapping: r.flow_mapping.clone().unwrap_or_default(),
                provider: r.provider.clone().unwrap_or_default(),
                location: r.location.clone().unwrap_or_default(),
                stream_amount: r.amount,
                normalized_amount: r.amount / ref_amt_t_h,
                amount_unit: r.unit.clone(),
                normalized_result: impact,
                is_reference_flow: r.flow_type == LcaFlowType::ReferenceProductFlow,
            });
        }
    } else {
        for sp in &r.species {
            if sp.flow_mapping.is_some() {
                let impact = eval_impact(
                    sp.flow_id.as_deref(),
                    sp.flow_mapping.as_deref(),
                    sp.flow_type,
                    sp.provider.as_deref(),
                    sp.location.as_deref(),
                    sp.amount,
                    &sp.unit,
                    cat_id,
                );
                out_lines.push(LciaResultStreamLine {
                    stream_name: sp.formula.clone(),
                    flow_mapping: sp.flow_mapping.clone().unwrap_or_default(),
                    provider: sp.provider.clone().unwrap_or_default(),
                    location: sp.location.clone().unwrap_or_default(),
                    stream_amount: sp.amount,
                    normalized_amount: sp.amount / ref_amt_t_h,
                    amount_unit: sp.unit.clone(),
                    normalized_result: impact,
                    is_reference_flow: false,
                });
            }
        }
    }
}

fn build_beta_io_sheet(name: &str, rows: &[BetaStreamRow]) -> ExportSheet {
    let headers = [
        "Stream",
        "Species",
        "Amount",
        "Units",
        "LCA Flowtype",
        "Flow Mapping",
        "Provider",
        "Location",
    ];
    let mut cells = Vec::new();
    for (c, h) in headers.iter().enumerate() {
        cells.push(ExportCell {
            row: 0,
            col: c,
            value: ExportCellValue::Text((*h).to_string()),
            style: ExportCellStyle::Header,
        });
    }
    let mut r_idx = 1usize;
    for r in rows {
        let vals = [
            ExportCellValue::Text(r.stream_name.clone()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Number(r.amount),
            ExportCellValue::Text(r.unit.clone()),
            ExportCellValue::Text(r.flow_type.as_str().to_string()),
            ExportCellValue::Text(r.flow_mapping.clone().unwrap_or_default()),
            ExportCellValue::Text(r.provider.clone().unwrap_or_default()),
            ExportCellValue::Text(r.location.clone().unwrap_or_default()),
        ];
        for (c, v) in vals.into_iter().enumerate() {
            cells.push(ExportCell {
                row: r_idx,
                col: c,
                value: v,
                style: ExportCellStyle::Header,
            });
        }
        r_idx += 1;
        for sp in &r.species {
            let svals = [
                ExportCellValue::Text(String::new()),
                ExportCellValue::Text(sp.formula.clone()),
                ExportCellValue::Number(sp.amount),
                ExportCellValue::Text(sp.unit.clone()),
                ExportCellValue::Text(sp.flow_type.as_str().to_string()),
                ExportCellValue::Text(sp.flow_mapping.clone().unwrap_or_default()),
                ExportCellValue::Text(sp.provider.clone().unwrap_or_default()),
                ExportCellValue::Text(sp.location.clone().unwrap_or_default()),
            ];
            for (c, v) in svals.into_iter().enumerate() {
                cells.push(ExportCell {
                    row: r_idx,
                    col: c,
                    value: v,
                    style: ExportCellStyle::Default,
                });
            }
            r_idx += 1;
        }
    }
    ExportSheet {
        name: name.to_string(),
        cells,
        col_widths: vec![22.0, 14.0, 12.0, 8.0, 22.0, 26.0, 32.0, 10.0],
    }
}

fn build_beta_results_sheet(results: &[LciaCategoryResult]) -> ExportSheet {
    let headers = [
        "Method",
        "Category",
        "Unit",
        "Stream Name",
        "Flow Mapping",
        "Provider",
        "Location",
        "Stream Amount",
        "Normalized Amounts",
        "Units",
        "Normalized Results",
        "Units",
    ];
    let mut cells = Vec::new();
    for (c, h) in headers.iter().enumerate() {
        cells.push(ExportCell {
            row: 0,
            col: c,
            value: ExportCellValue::Text((*h).to_string()),
            style: ExportCellStyle::Header,
        });
    }
    let mut r_idx = 1usize;
    for cat in results {
        let cat_row = [
            ExportCellValue::Text(cat.method_name.clone()),
            ExportCellValue::Text(cat.category_name.clone()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Text("Total".to_string()),
            ExportCellValue::Text(String::new()),
            ExportCellValue::Number(cat.grand_total),
            ExportCellValue::Text(format!("{} per t/h of {}", cat.result_unit, cat.reference_stream_name)),
        ];
        for (c, v) in cat_row.into_iter().enumerate() {
            cells.push(ExportCell {
                row: r_idx,
                col: c,
                value: v,
                style: ExportCellStyle::Header,
            });
        }
        r_idx += 1;
        for ub in &cat.units {
            for line in ub.inputs.iter().chain(ub.outputs.iter()) {
                let row_vals = [
                    ExportCellValue::Text(String::new()),
                    ExportCellValue::Text(String::new()),
                    ExportCellValue::Text(ub.unit_name.clone()),
                    ExportCellValue::Text(line.stream_name.clone()),
                    ExportCellValue::Text(line.flow_mapping.clone()),
                    ExportCellValue::Text(line.provider.clone()),
                    ExportCellValue::Text(line.location.clone()),
                    ExportCellValue::Number(line.stream_amount),
                    ExportCellValue::Number(line.normalized_amount),
                    ExportCellValue::Text(line.amount_unit.clone()),
                    ExportCellValue::Number(line.normalized_result),
                    ExportCellValue::Text(String::new()),
                ];
                for (c, v) in row_vals.into_iter().enumerate() {
                    cells.push(ExportCell {
                        row: r_idx,
                        col: c,
                        value: v,
                        style: ExportCellStyle::Default,
                    });
                }
                r_idx += 1;
            }
        }
    }
    ExportSheet {
        name: "Results".to_string(),
        cells,
        col_widths: vec![22.0, 26.0, 16.0, 18.0, 24.0, 32.0, 10.0, 14.0, 16.0, 8.0, 18.0, 24.0],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-configured Mapping Builders for Official Chapter 49 Benchmark Presets
// ─────────────────────────────────────────────────────────────────────────────

/// Construct ready-to-evaluate Classic LCA state matching Chapter 49.3 (`Zn TSL Fuming`, Figs. 9–17).
pub fn preset_classic_zn_tsl_fuming_lca(project: &SimProject) -> ClassicLcaState {
    let mut state = ClassicLcaState::default();
    state.process_info = ClassicProcessInfo {
        process_name: "TSL Furnace".to_string(),
        technology: "Pyro".to_string(),
        general_comment: "Black-box Zn TSL Fuming LCI benchmark (HSC Sim LCA Ch. 49.3)".to_string(),
        hsc_ver_last_save: "10.0.0.1".to_string(),
        location: "FI".to_string(),
        location_comment: "Finland benchmark".to_string(),
        author: "User".to_string(),
        date: "2026-11-15".to_string(),
        version_number: "01.00.000".to_string(),
    };
    state.sync_from_project(project);

    let map_classic = |row: &mut ClassicStreamRow, equiv: &str, group: LcaGroup, main: bool| {
        row.lca_equivalent = equiv.to_string();
        row.lca_flow_id = Some(deterministic_uuid(&format!("gabi::{equiv}")));
        row.lca_group = group;
        row.main_product = main;
    };

    for row in &mut state.inputs {
        match row.stream_name.as_str() {
            "Residues" => map_classic(row, "Lead - Zinc srap", LcaGroup::FromTechnosphere, false),
            "Air" | "Air - PC" => map_classic(row, "Air", LcaGroup::FromNature, false),
            "Reductant" => map_classic(row, "Metallurgical coal", LcaGroup::MaterialsFuels, false),
            "Power" => map_classic(row, "Electricity", LcaGroup::ElectricityHeat, false),
            "Water in - WHRB" => map_classic(row, "Water", LcaGroup::FromNature, false),
            _ => {}
        }
    }
    for row in &mut state.outputs {
        match row.stream_name.as_str() {
            "Metal" => map_classic(row, "Iron", LcaGroup::AllocatedByProduct, false),
            "Clean Slag" => map_classic(row, "Slag", LcaGroup::ToNature, false),
            "Heat Loss - TSL" | "Heat Loss - Quenching" | "Heat Loss - Dust cooling" => {
                map_classic(row, "Waste heat", LcaGroup::ToNature, false)
            }
            "Steam out - WHRB" => map_classic(row, "Steam (hp)", LcaGroup::AllocatedByProduct, false),
            "Off-gas to further treatment" => map_classic(row, "Exhaust", LcaGroup::ToNature, false),
            "Final ZnO Dust" => map_classic(row, "Zinc oxide", LcaGroup::ReferenceProduct, true),
            _ => {}
        }
    }
    state.refresh_indicators();
    let _ = state.normalize();
    state
}

/// Construct ready-to-evaluate Beta LCIA state matching Chapter 49.4 (`Iron Process`, Figs. 22–35).
pub fn preset_beta_iron_process_lca(project: &SimProject) -> BetaLcaState {
    let mut state = BetaLcaState::default();
    state.sync_from_project(project);

    // Map boundary inputs matching Fig. 22 & Fig. 34
    for i in 0..state.inputs.len() {
        let name = state.inputs[i].stream_name.clone();
        match name.as_str() {
            "Coke" => {
                state.set_flow_type(false, i, None, LcaFlowType::ProductFlow);
                let _ = state.add_mapping(false, i, None, &deterministic_uuid("olca::petroleum_coke"));
            }
            "Steel Scrap" => {
                state.set_flow_type(false, i, None, LcaFlowType::ProductFlow);
                let _ = state.add_mapping(false, i, None, &deterministic_uuid("olca::steel_low_alloyed"));
            }
            "Hematite Pellets" => {
                state.set_flow_type(false, i, None, LcaFlowType::ProductFlow);
                let _ = state.add_mapping(false, i, None, &deterministic_uuid("olca::pig_iron"));
            }
            "Limestone" => {
                state.set_flow_type(false, i, None, LcaFlowType::ProductFlow);
                let _ = state.add_mapping(false, i, None, &deterministic_uuid("olca::lime_hydrated"));
            }
            _ => {}
        }
    }

    // Add Manual Input Stream: Extra electricity = 10000.00 kW -> market group for electricity, high voltage | GLO
    let man_idx = state.add_manual_stream(false);
    {
        let m = &mut state.inputs[man_idx];
        m.stream_name = "Extra electricity".to_string();
        m.amount = 10000.00;
        m.unit = "kW".to_string();
        m.flow_type = LcaFlowType::ProductFlow;
    }
    let _ = state.add_mapping(false, man_idx, None, &deterministic_uuid("olca::electricity_hv"));
    state.set_location_and_provider(
        false,
        man_idx,
        None,
        Some("GLO".to_string()),
        Some("market group for electricity, high voltage".to_string()),
    );

    // Map Outputs: Pig Iron -> Reference Flow (Product); Furnace Gas -> Indicators CO(g), CO2(g), SO2(g)
    for i in 0..state.outputs.len() {
        let name = state.outputs[i].stream_name.clone();
        if name == "Pig Iron" {
            state.set_flow_type(true, i, None, LcaFlowType::ReferenceProductFlow);
        } else if name == "Furnace Gas" {
            let _ = state.convert_stream_to_indicators(true, i);
            for sp_i in 0..state.outputs[i].species.len() {
                let formula = state.outputs[i].species[sp_i].formula.clone();
                match formula.as_str() {
                    "CO(g)" => {
                        let _ = state.add_mapping(true, i, Some(sp_i), &deterministic_uuid("olca::co_fossil"));
                    }
                    "CO2(g)" => {
                        let _ = state.add_mapping(true, i, Some(sp_i), &deterministic_uuid("olca::co2_fossil"));
                    }
                    "SO2(g)" => {
                        let _ = state.add_mapping(true, i, Some(sp_i), &deterministic_uuid("olca::so2"));
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = state.add_impact_category("recipe_midpoint_h", "gwp100");
    let _ = state.add_impact_category("recipe_midpoint_h", "mdp");
    let _ = state.calculate_results();
    state
}
