//! Module 08: H, S, C and G Diagrams (Thermochemical Data Diagrams & Ellingham).
//!
//! Evaluates 8 temperature-dependent thermochemical diagram types:
//! `H Total`, `H Heat Content (latent)`, `S Entropy`, `Cp Heat Capacity`,
//! `G Gibbs Energy`, `DH`, `DS`, and `DG (Ellingham)`.
//! Supports element balancing (`Balance Element Amount`), per-mol or per-kg
//! scaling, and solid vs. extrapolated curve segmentation.

use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_JOULE, T_REF_K};
use soryntel_solver::parse_term;

use crate::{
    balance::species_chemical_exergy_kj_mol, evaluate_species_si, sum_element_references,
    ChemicalSpecies, SpeciesRepository,
};

/// Nine thermochemical diagram types supported by HSC6 §19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramType {
    HTotal,
    HHeatContent,
    SEntropy,
    CpHeatCapacity,
    GGibbsEnergy,
    DeltaH,
    DeltaS,
    DeltaGEllingham,
    ExExergy,
}

impl DiagramType {
    pub const ALL: &'static [DiagramType] = &[
        DiagramType::HTotal,
        DiagramType::HHeatContent,
        DiagramType::SEntropy,
        DiagramType::CpHeatCapacity,
        DiagramType::GGibbsEnergy,
        DiagramType::DeltaH,
        DiagramType::DeltaS,
        DiagramType::DeltaGEllingham,
        DiagramType::ExExergy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DiagramType::HTotal => "H Enthalpy (total)",
            DiagramType::HHeatContent => "H Heat Content",
            DiagramType::SEntropy => "S Entropy",
            DiagramType::CpHeatCapacity => "Cp Heat Capacity",
            DiagramType::GGibbsEnergy => "G Gibbs Energy",
            DiagramType::DeltaH => "Delta H (DH)",
            DiagramType::DeltaS => "Delta S (DS)",
            DiagramType::DeltaGEllingham => "Delta G (Ellingham)",
            DiagramType::ExExergy => "Ex Exergy (total)",
        }
    }

    pub fn is_entropy_or_cp(self) -> bool {
        matches!(
            self,
            DiagramType::SEntropy | DiagramType::CpHeatCapacity | DiagramType::DeltaS
        )
    }
}

/// Energy unit for diagram Y-axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramEnergyUnit {
    KiloJoules,
    KiloCalories,
}

impl DiagramEnergyUnit {
    pub fn y_unit_label(self, dtype: DiagramType, per_kg: bool) -> &'static str {
        let mass_or_mol = if per_kg { "kg" } else { "mol" };
        match (self, dtype.is_entropy_or_cp()) {
            (DiagramEnergyUnit::KiloJoules, false) => {
                if per_kg { "kJ/kg" } else { "kJ/mol" }
            }
            (DiagramEnergyUnit::KiloCalories, false) => {
                if per_kg { "kcal/kg" } else { "kcal/mol" }
            }
            (DiagramEnergyUnit::KiloJoules, true) => {
                if per_kg { "J/(kg·K)" } else { "J/(mol·K)" }
            }
            (DiagramEnergyUnit::KiloCalories, true) => {
                let _ = mass_or_mol;
                if per_kg { "cal/(kg·K)" } else { "cal/(mol·K)" }
            }
        }
    }
}

/// Input species row in `X-data`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramSpeciesInput {
    pub formula: String,
    pub coeff: f64,
    #[serde(default)]
    pub row_index: Option<usize>,
    #[serde(default)]
    pub species_override: Option<ChemicalSpecies>,
}

/// Request for computing a thermochemical data diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramRequest {
    pub species: Vec<DiagramSpeciesInput>,
    pub diagram_type: DiagramType,
    pub t_min_k: f64,
    pub t_max_k: f64,
    pub step_k: f64,
    pub energy_unit: DiagramEnergyUnit,
    pub per_kg: bool,
}

/// Single evaluated point on a species curve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DiagramPoint {
    pub t_k: f64,
    pub y_val: f64,
    pub extrapolated: bool,
}

/// Evaluated curve for one species.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramSeries {
    pub label: String,
    pub formula: String,
    pub coeff: f64,
    #[serde(default)]
    pub row_index: Option<usize>,
    pub t_min_db_k: f64,
    pub t_max_db_k: f64,
    pub points: Vec<DiagramPoint>,
}

/// Complete diagram calculation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    pub diagram_type: DiagramType,
    pub y_unit: String,
    pub series: Vec<DiagramSeries>,
    pub warnings: Vec<String>,
}

/// Computes the balanced stoichiometric coefficient `target_atoms / nu_element`
/// for `formula` with respect to `element` (used for Ellingham `DG` diagrams).
pub fn balanced_coeff_for_element(formula: &str, element: &str, target_atoms: f64) -> Option<f64> {
    let term = parse_term(formula).ok()?;
    let count = *term.elements.get(element)?;
    if count <= 0.0 || target_atoms <= 0.0 {
        return None;
    }
    Some(target_atoms / count)
}

/// Evaluates all requested species over `[t_min_k, t_max_k]` with step `step_k`.
pub fn calculate_diagram(
    req: &DiagramRequest,
    repo: &dyn SpeciesRepository,
) -> Result<DiagramResult, String> {
    if req.t_max_k <= req.t_min_k {
        return Err("T Max must be greater than T Min.".to_string());
    }
    let step = req.step_k.max(1.0);
    let mut temps = Vec::new();
    let mut t = req.t_min_k;
    while t <= req.t_max_k + 1e-6 {
        temps.push(t.max(1.0));
        t += step;
    }

    let mut series = Vec::new();
    let mut warnings = Vec::new();
    let need_ref = matches!(
        req.diagram_type,
        DiagramType::DeltaH | DiagramType::DeltaS | DiagramType::DeltaGEllingham
    );
    let need_ref_298 = matches!(
        req.diagram_type,
        DiagramType::HHeatContent | DiagramType::ExExergy
    );

    for item in &req.species {
        let f_trim = item.formula.trim();
        if f_trim.is_empty() {
            continue;
        }
        let Some(sp) = item
            .species_override
            .clone()
            .or_else(|| repo.get_species(f_trim))
        else {
            warnings.push(format!("Species '{f_trim}' not found in database."));
            continue;
        };
        let term = parse_term(&sp.formula).map_err(|e| format!("{f_trim}: {e}"))?;
        let (t_min_db_k, t_max_db_k) = species_db_temp_range(&sp);

        // Reference H(298.15 K), S(298.15 K), and standard chemical exergy for H Heat Content / Exergy.
        let pt_298 = if need_ref_298 {
            evaluate_species_si(&sp, T_REF_K, false, false)
                .map(|(p, _)| p)
                .ok()
        } else {
            None
        };
        let h_298_j = pt_298.as_ref().map(|p| p.h).unwrap_or(0.0);
        let s_298_j_k = pt_298.as_ref().map(|p| p.s).unwrap_or(0.0);
        let ex_chem_j = if req.diagram_type == DiagramType::ExExergy {
            species_chemical_exergy_kj_mol(repo, &sp) * 1000.0
        } else {
            0.0
        };

        let scale_mass = if req.per_kg {
            // 1 kg = 1000 g / M (mol)
            1000.0 / sp.molecular_weight.max(1e-6)
        } else {
            item.coeff
        };

        let mut points = Vec::with_capacity(temps.len());
        for &tk in &temps {
            let Ok((pt, _)) = evaluate_species_si(&sp, tk, false, false) else {
                continue;
            };
            let (h_ref, s_ref, g_ref) = if need_ref {
                let (h, s, g, _) = sum_element_references(&term, tk, false, repo)
                    .unwrap_or((0.0, 0.0, 0.0, 0.0));
                (h, s, g)
            } else {
                (0.0, 0.0, 0.0)
            };

            let val_si = match req.diagram_type {
                DiagramType::HTotal => pt.h,
                DiagramType::HHeatContent => pt.h - h_298_j,
                DiagramType::SEntropy => pt.s,
                DiagramType::CpHeatCapacity => pt.cp,
                DiagramType::GGibbsEnergy => pt.g,
                DiagramType::DeltaH => pt.h - h_ref,
                DiagramType::DeltaS => pt.s - s_ref,
                DiagramType::DeltaGEllingham => pt.g - g_ref,
                DiagramType::ExExergy => {
                    ex_chem_j + (pt.h - h_298_j) - T_REF_K * (pt.s - s_298_j_k)
                }
            };

            let y_converted = match (req.energy_unit, req.diagram_type.is_entropy_or_cp()) {
                (DiagramEnergyUnit::KiloJoules, false) => (val_si * scale_mass) / 1000.0,
                (DiagramEnergyUnit::KiloCalories, false) => {
                    (val_si * scale_mass) / (1000.0 * CAL_TO_JOULE)
                }
                (DiagramEnergyUnit::KiloJoules, true) => val_si * scale_mass,
                (DiagramEnergyUnit::KiloCalories, true) => (val_si * scale_mass) / CAL_TO_JOULE,
            };

            points.push(DiagramPoint {
                t_k: tk,
                y_val: y_converted,
                extrapolated: pt.extrapolated_from_k.is_some(),
            });
        }

        let label = if (item.coeff - 1.0).abs() < 1e-3 || req.per_kg {
            sp.formula.clone()
        } else {
            format!("{:.2} {}", item.coeff, sp.formula)
        };

        series.push(DiagramSeries {
            label,
            formula: sp.formula,
            coeff: item.coeff,
            row_index: item.row_index,
            t_min_db_k,
            t_max_db_k,
            points,
        });
    }

    Ok(DiagramResult {
        diagram_type: req.diagram_type,
        y_unit: req
            .energy_unit
            .y_unit_label(req.diagram_type, req.per_kg)
            .to_string(),
        series,
        warnings,
    })
}

/// Nomographic Ellingham scales (`log10 pO2`, `CO/CO2`, `H2/H2O`) at `(t_k, y_val)`
/// implementing Ch. 19 Appendix 1 Equations (10)–(21) for `o_atoms_balanced` O atoms (`2.0` -> 1 mol O2(g)).
#[derive(Debug, Clone, Copy)]
pub struct EllinghamScales {
    pub log10_po2: f64,
    pub log10_co_co2: Option<f64>,
    pub log10_h2_h2o: Option<f64>,
}

/// Converts a plot `(T_K, ΔG)` value in `unit` back to `log10(pO2)` and `CO/CO2`, `H2/H2O` ratios.
pub fn ellingham_scales_at(
    t_k: f64,
    y_val: f64,
    o_atoms_balanced: f64,
    unit: DiagramEnergyUnit,
    repo: &dyn SpeciesRepository,
) -> EllinghamScales {
    let dg_j = match unit {
        DiagramEnergyUnit::KiloJoules => y_val * 1000.0,
        DiagramEnergyUnit::KiloCalories => y_val * 1000.0 * CAL_TO_JOULE,
    };
    let n_o2 = (o_atoms_balanced * 0.5).max(1e-6);
    let denom = n_o2 * R_JOULE * t_k.max(1.0) * std::f64::consts::LN_10;
    let log10_po2 = dg_j / denom;

    // 2 CO(g) + O2(g) = 2 CO2(g) -> scaled to n_o2 moles of O2(g): ΔG° = n_o2 * (2*g(CO2) - 2*g(CO) - g(O2))
    let log10_co_co2 = (|| {
        let co = repo.get_species("CO(g)")?;
        let co2 = repo.get_species("CO2(g)")?;
        let o2 = repo.get_species("O2(g)")?;
        let (pt_co, _) = evaluate_species_si(&co, t_k.max(1.0), false, false).ok()?;
        let (pt_co2, _) = evaluate_species_si(&co2, t_k.max(1.0), false, false).ok()?;
        let (pt_o2, _) = evaluate_species_si(&o2, t_k.max(1.0), false, false).ok()?;
        let dg_2co_j = n_o2 * (2.0 * pt_co2.g - 2.0 * pt_co.g - pt_o2.g);
        Some((dg_2co_j - dg_j) / (2.0 * denom))
    })();

    // 2 H2(g) + O2(g) = 2 H2O(g) -> scaled to n_o2 moles of O2(g)
    let log10_h2_h2o = (|| {
        let h2 = repo.get_species("H2(g)")?;
        let h2o = repo.get_species("H2O(g)")?;
        let o2 = repo.get_species("O2(g)")?;
        let (pt_h2, _) = evaluate_species_si(&h2, t_k.max(1.0), false, false).ok()?;
        let (pt_h2o, _) = evaluate_species_si(&h2o, t_k.max(1.0), false, false).ok()?;
        let (pt_o2, _) = evaluate_species_si(&o2, t_k.max(1.0), false, false).ok()?;
        let dg_2h2_j = n_o2 * (2.0 * pt_h2o.g - 2.0 * pt_h2.g - pt_o2.g);
        Some((dg_2h2_j - dg_j) / (2.0 * denom))
    })();

    EllinghamScales {
        log10_po2,
        log10_co_co2,
        log10_h2_h2o,
    }
}

/// Evaluates `ΔG°(T)` along a constant `pO2 = 10^log10_po2` line in `unit` (Eq. 11).
pub fn ellingham_po2_isoline_val(
    t_k: f64,
    log10_po2: f64,
    o_atoms_balanced: f64,
    unit: DiagramEnergyUnit,
) -> f64 {
    let n_o2 = (o_atoms_balanced * 0.5).max(1e-6);
    let dg_j = n_o2 * R_JOULE * t_k.max(1.0) * std::f64::consts::LN_10 * log10_po2;
    match unit {
        DiagramEnergyUnit::KiloJoules => dg_j / 1000.0,
        DiagramEnergyUnit::KiloCalories => dg_j / (1000.0 * CAL_TO_JOULE),
    }
}

/// Helper to look up `TMin` / `TMax` in Kelvin for a formula in `repo`.
pub fn species_db_temp_range(sp: &ChemicalSpecies) -> (f64, f64) {
    let t_min = sp
        .intervals
        .iter()
        .map(|iv| iv.t_min)
        .fold(f64::INFINITY, f64::min);
    let t_max = sp
        .intervals
        .iter()
        .map(|iv| iv.t_max)
        .fold(0.0_f64, f64::max);
    (t_min, t_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KelleyCoefficients, Phase, SpeciesThermoInterval};
    use std::collections::HashMap;

    struct MockRepo(HashMap<String, ChemicalSpecies>);
    impl SpeciesRepository for MockRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get(formula).cloned()
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            let key = match element_symbol {
                "O" => "O2(g)",
                "C" => "C",
                _ => element_symbol,
            };
            self.0.get(key).cloned()
        }
    }

    fn sp_const(formula: &str, mw: f64, h298_kcal: f64, s298_cal: f64, cp_a: f64, t_max: f64) -> ChemicalSpecies {
        ChemicalSpecies {
            formula: formula.to_string(),
            name: formula.to_string(),
            phase: Phase::Solid,
            molecular_weight: mw,
            density_g_cm3: 1.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 298.15,
                t_max,
                phase: Phase::Solid,
                kelley: KelleyCoefficients {
                    a: cp_a,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: h298_kcal * 1000.0 * CAL_TO_JOULE,
                s_or_delta_s_tr: s298_cal * CAL_TO_JOULE,
                reference: "test".to_string(),
            }],
        }
    }

    #[test]
    fn balance_element_amount_oxides_for_o2() {
        assert!((balanced_coeff_for_element("Fe2O3", "O", 2.0).unwrap() - 2.0 / 3.0).abs() < 1e-6);
        assert!((balanced_coeff_for_element("FeO", "O", 2.0).unwrap() - 2.0).abs() < 1e-6);
        assert!((balanced_coeff_for_element("CO2(g)", "O", 2.0).unwrap() - 1.0).abs() < 1e-6);
        assert!((balanced_coeff_for_element("MgO", "O", 2.0).unwrap() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn heat_content_is_zero_at_298k_and_detects_extrapolation() {
        let mut map = HashMap::new();
        map.insert("FeO".to_string(), sp_const("FeO", 71.84, -63.5, 14.5, 50.0, 1000.0));
        let repo = MockRepo(map);
        let req = DiagramRequest {
            species: vec![DiagramSpeciesInput {
                formula: "FeO".to_string(),
                coeff: 1.0,
                row_index: Some(0),
                species_override: None,
            }],
            diagram_type: DiagramType::HHeatContent,
            t_min_k: 298.15,
            t_max_k: 1298.15,
            step_k: 500.0,
            energy_unit: DiagramEnergyUnit::KiloJoules,
            per_kg: false,
        };
        let res = calculate_diagram(&req, &repo).unwrap();
        let pts = &res.series[0].points;
        assert!(pts[0].y_val.abs() < 1e-6);
        assert!(!pts[0].extrapolated);
        assert!(!pts[1].extrapolated);
        assert!(pts[2].extrapolated);
        assert!((pts[2].y_val - 50.0).abs() < 1e-4);
        assert_eq!(res.series[0].row_index, Some(0));
    }

    #[test]
    fn ellingham_mgo_more_stable_than_feo() {
        let mut map = HashMap::new();
        map.insert("O2(g)".to_string(), sp_const("O2(g)", 32.0, 0.0, 49.0, 30.0, 3000.0));
        map.insert("Mg".to_string(), sp_const("Mg", 24.3, 0.0, 7.8, 25.0, 3000.0));
        map.insert("Fe".to_string(), sp_const("Fe", 55.85, 0.0, 6.5, 25.0, 3000.0));
        map.insert("MgO".to_string(), sp_const("MgO", 40.3, -143.8, 6.4, 45.0, 3000.0));
        map.insert("FeO".to_string(), sp_const("FeO", 71.84, -63.5, 14.5, 50.0, 3000.0));
        let repo = MockRepo(map);
        let req = DiagramRequest {
            species: vec![
                DiagramSpeciesInput {
                    formula: "MgO".to_string(),
                    coeff: 2.0,
                    row_index: Some(0),
                    species_override: None,
                },
                DiagramSpeciesInput {
                    formula: "FeO".to_string(),
                    coeff: 2.0,
                    row_index: Some(1),
                    species_override: None,
                },
            ],
            diagram_type: DiagramType::DeltaGEllingham,
            t_min_k: 298.15,
            t_max_k: 1298.15,
            step_k: 500.0,
            energy_unit: DiagramEnergyUnit::KiloJoules,
            per_kg: false,
        };
        let res = calculate_diagram(&req, &repo).unwrap();
        assert!(res.series[0].points[0].y_val < res.series[1].points[0].y_val);
    }

    #[test]
    fn ex_exergy_diagram_and_ellingham_fig6_scale_parity() {
        let mut map = HashMap::new();
        map.insert("Fe".to_string(), sp_const("Fe", 55.85, 0.0, 6.5, 25.0, 3000.0));
        map.insert("CO(g)".to_string(), sp_const("CO(g)", 28.01, -26.42, 47.2, 29.1, 3000.0));
        map.insert("CO2(g)".to_string(), sp_const("CO2(g)", 44.01, -94.05, 51.07, 37.1, 3000.0));
        map.insert("O2(g)".to_string(), sp_const("O2(g)", 32.0, 0.0, 49.0, 30.0, 3000.0));
        let repo = MockRepo(map);

        // Ch. 19 line 17: Ex Exergy (total) increases cleanly above T0 = 298.15 K
        let req = DiagramRequest {
            species: vec![DiagramSpeciesInput {
                formula: "Fe".to_string(),
                coeff: 1.0,
                row_index: Some(0),
                species_override: None,
            }],
            diagram_type: DiagramType::ExExergy,
            t_min_k: 298.15,
            t_max_k: 1298.15,
            step_k: 500.0,
            energy_unit: DiagramEnergyUnit::KiloJoules,
            per_kg: false,
        };
        let res = calculate_diagram(&req, &repo).unwrap();
        assert_eq!(res.y_unit, "kJ/mol");
        let pts = &res.series[0].points;
        assert!(pts[2].y_val > pts[0].y_val + 10.0);

        // Ch. 19 Fig. 6 check: 2 Fe + O2(g) = 2 FeO at 1000 °C -> ΔG° = -363.052 kJ => log10 pO2 = -14.896
        let sc = ellingham_scales_at(1273.15, -363.052, 2.0, DiagramEnergyUnit::KiloJoules, &repo);
        assert!((sc.log10_po2 - (-14.896)).abs() < 0.002);
        let dg_back = ellingham_po2_isoline_val(1273.15, sc.log10_po2, 2.0, DiagramEnergyUnit::KiloJoules);
        assert!((dg_back - (-363.052)).abs() < 1e-6);
    }
}
