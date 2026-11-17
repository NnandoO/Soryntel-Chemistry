//! Module 06 — HSC Estimate Engine (H, S, Cp, ΔG Estimation & Kelley Polynomial Fitting).
//!
//! Implements Neumann-Kopp heat capacity estimation, Latimer ionic entropy,
//! inorganic ionic/oxide enthalpy & hydration corrections, Benson/Joback organic
//! group additivity, oxidation-state set enumeration, and 4-parameter Kelley Cp fitting.

use crate::balance::parse_hsc_species_elements;
use crate::{evaluate_species_si, ChemicalSpecies, KelleyCoefficients, Phase, SpeciesRepository, SpeciesThermoInterval};
use serde::{Deserialize, Serialize};
use soryntel_core::T_REF_K;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstimateSpeciesType {
    NotSpecified,
    Inorganic,
    Organic,
}

impl EstimateSpeciesType {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotSpecified => "Not Specified",
            Self::Inorganic => "Inorganic",
            Self::Organic => "Organic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganicGroup {
    None,
    Alkanes,
    Alkenes,
    Alkynes,
    Aromatics,
    Alcohols,
    Acids,
}

impl OrganicGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Alkanes => "ALKANES",
            Self::Alkenes => "ALKENES",
            Self::Alkynes => "ALKYNES",
            Self::Aromatics => "AROMATICS",
            Self::Alcohols => "ALCOHOLS",
            Self::Acids => "ACIDS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionalGroup {
    None,
    Ene,
    Diene,
    Triene,
    Yne,
    Hydroxyl,
    Carboxyl,
}

impl FunctionalGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Ene => "ene",
            Self::Diene => "diene",
            Self::Triene => "triene",
            Self::Yne => "yne",
            Self::Hydroxyl => "-OH",
            Self::Carboxyl => "-COOH",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub symbol: String,
    pub amount: f64,
    pub charge: i32,
    pub weight_pct: f64,
    pub atom_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRefValues {
    pub h_kj_mol: f64,
    pub s_j_molk: f64,
    pub cp_j_molk: f64,
    pub dg_kj_mol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateRow {
    pub formula: String,
    pub temp_c: f64,
    pub species_type: EstimateSpeciesType,
    pub organic_group: OrganicGroup,
    pub functional_group: FunctionalGroup,
    pub oxidation_set_idx: usize,
    pub available_sets: usize,
    pub mw: f64,
    pub est_h_kj_mol: Option<f64>,
    pub est_s_j_molk: Option<f64>,
    pub est_cp_j_molk: Option<f64>,
    pub est_dg_kj_mol: Option<f64>,
    pub db_vals: Option<DatabaseRefValues>,
    pub elements: Vec<ElementInfo>,
}

impl EstimateRow {
    pub fn new(formula: impl Into<String>) -> Self {
        Self {
            formula: formula.into(),
            temp_c: 25.0,
            species_type: EstimateSpeciesType::NotSpecified,
            organic_group: OrganicGroup::None,
            functional_group: FunctionalGroup::None,
            oxidation_set_idx: 0,
            available_sets: 1,
            mw: 0.0,
            est_h_kj_mol: None,
            est_s_j_molk: None,
            est_cp_j_molk: None,
            est_dg_kj_mol: None,
            db_vals: None,
            elements: Vec::new(),
        }
    }

    pub fn type_display(&self) -> String {
        match self.species_type {
            EstimateSpeciesType::NotSpecified => "Not Specified".to_string(),
            EstimateSpeciesType::Inorganic => "Inorganic".to_string(),
            EstimateSpeciesType::Organic => format!(
                "Organic, {}, {}",
                if self.organic_group == OrganicGroup::None {
                    "ALKANES"
                } else {
                    self.organic_group.label()
                },
                self.functional_group.label()
            ),
        }
    }
}

/// Atomic weights (g/mol) for common elements.
pub fn atomic_weight(sym: &str) -> f64 {
    match sym {
        "H" => 1.008,
        "Li" => 6.941,
        "Be" => 9.0122,
        "B" => 10.811,
        "C" => 12.011,
        "N" => 14.007,
        "O" => 15.999,
        "F" => 18.998,
        "Na" => 22.990,
        "Mg" => 24.305,
        "Al" => 26.982,
        "Si" => 28.086,
        "P" => 30.974,
        "S" => 32.065,
        "Cl" => 35.453,
        "K" => 39.098,
        "Ca" => 40.078,
        "Ti" => 47.867,
        "V" => 50.942,
        "Cr" => 51.996,
        "Mn" => 54.938,
        "Fe" => 55.845,
        "Co" => 58.933,
        "Ni" => 58.693,
        "Cu" => 63.546,
        "Zn" => 65.38,
        "As" => 74.922,
        "Se" => 78.96,
        "Br" => 79.904,
        "Sr" => 87.62,
        "Zr" => 91.224,
        "Mo" => 95.96,
        "Ag" => 107.868,
        "Cd" => 112.411,
        "Sn" => 118.710,
        "Sb" => 121.760,
        "I" => 126.904,
        "Ba" => 137.327,
        "W" => 183.84,
        "Pt" => 195.084,
        "Au" => 196.967,
        "Hg" => 200.59,
        "Pb" => 207.2,
        "Bi" => 208.98,
        _ => 50.0,
    }
}

/// Common oxidation states for elements (ordered by prevalence).
fn common_ox_states(sym: &str) -> &'static [i32] {
    match sym {
        "O" => &[-2],
        "H" => &[1, -1],
        "F" => &[-1],
        "Cl" | "Br" | "I" => &[-1, 1, 5, 7],
        "Li" | "Na" | "K" | "Ag" => &[1],
        "Be" | "Mg" | "Ca" | "Sr" | "Ba" | "Zn" | "Cd" => &[2],
        "Al" | "B" => &[3],
        "Si" | "Zr" => &[4],
        "C" => &[4, 2, -4],
        "N" => &[5, 3, -3],
        "P" => &[5, 3, -3],
        "S" => &[6, 4, -2],
        "Se" => &[4, 6, -2],
        "Fe" => &[3, 2, 4],
        "Mn" => &[2, 4, 3, 7, 6],
        "V" => &[5, 4, 3, 2],
        "Cr" => &[3, 6, 2],
        "Cu" => &[2, 1],
        "Co" | "Ni" => &[2, 3],
        "Ti" => &[4, 3],
        "Pb" | "Sn" => &[2, 4],
        "W" | "Mo" => &[6, 4],
        _ => &[2, 3, 1],
    }
}

/// Enumerate valid charge-neutral oxidation number assignments for up to 5 elements.
pub fn enumerate_oxidation_sets(elems: &BTreeMap<String, f64>) -> Vec<BTreeMap<String, i32>> {
    let keys: Vec<String> = elems.keys().cloned().collect();
    let mut results = Vec::new();
    if keys.is_empty() || keys.len() > 6 {
        return results;
    }

    fn backtrack(
        idx: usize,
        keys: &[String],
        elems: &BTreeMap<String, f64>,
        current: &mut BTreeMap<String, i32>,
        sum_charge: f64,
        out: &mut Vec<BTreeMap<String, i32>>,
    ) {
        if out.len() >= 8 {
            return;
        }
        if idx == keys.len() {
            if sum_charge.abs() < 1e-3 {
                out.push(current.clone());
            }
            return;
        }
        let sym = &keys[idx];
        let amt = elems[sym];
        for &ox in common_ox_states(sym) {
            current.insert(sym.clone(), ox);
            backtrack(
                idx + 1,
                keys,
                elems,
                current,
                sum_charge + amt * (ox as f64),
                out,
            );
        }
        current.remove(sym);
    }

    let mut cur = BTreeMap::new();
    backtrack(0, &keys, elems, &mut cur, 0.0, &mut results);
    results
}

/// Solid-state Neumann-Kopp / Kopp atomic Cp(298.15 K) in J/(mol·K) and temperature slope dCp/dT.
fn element_kopp_cp_params(sym: &str) -> (f64, f64) {
    match sym {
        "H" => (9.6, 0.012),
        "C" => (8.5, 0.014),
        "N" => (12.6, 0.010),
        "O" => (16.7, 0.011),
        "F" => (20.9, 0.010),
        "Cl" => (25.1, 0.010),
        "Br" => (26.0, 0.010),
        "I" => (26.8, 0.010),
        "S" => (22.6, 0.012),
        "P" => (23.8, 0.012),
        "Si" => (20.0, 0.010),
        "B" => (11.1, 0.015),
        "Li" => (24.8, 0.012),
        "Na" => (28.2, 0.012),
        "K" => (29.6, 0.012),
        "Mg" => (24.9, 0.010),
        "Ca" => (25.9, 0.010),
        "Ba" => (28.1, 0.010),
        "Al" => (24.2, 0.011),
        "Fe" => (25.1, 0.012),
        "Cu" => (24.4, 0.008),
        "Zn" => (25.4, 0.010),
        "Ag" => (25.4, 0.008),
        "Mn" => (26.3, 0.010),
        "V" => (24.9, 0.009),
        "W" => (24.3, 0.006),
        "Pb" => (26.8, 0.009),
        _ => (25.5, 0.010),
    }
}

/// Latimer (1951) / Kubaschewski ionic entropy contribution (J/(mol·K)) at 298.15 K.
fn latimer_element_entropy(sym: &str, charge: i32) -> f64 {
    if sym == "O" {
        return 10.5;
    }
    if sym == "H" {
        return 8.5;
    }
    if sym == "Cl" && charge < 0 {
        return 41.8;
    }
    if sym == "Br" && charge < 0 {
        return 54.4;
    }
    if sym == "I" && charge < 0 {
        return 61.1;
    }
    if sym == "S" && charge < 0 {
        return 34.3;
    }
    if sym == "S" && charge > 0 {
        return 18.0;
    }
    if sym == "C" {
        return 10.0;
    }
    if sym == "Si" {
        return 14.0;
    }
    if sym == "N" {
        return 15.0;
    }
    // Latimer formula for cations: S_cat ≈ 1.5 * R * ln(AW) - 0.94 * |z| * 4.184 + 12.0
    let aw = atomic_weight(sym).max(2.0);
    (12.47 * aw.ln() - 3.93 * (charge.abs() as f64) + 8.0).max(12.0)
}

/// Approximate standard enthalpy contribution per element/oxide in inorganic solids (kJ/mol).
fn inorganic_enthalpy_contrib(sym: &str, ox: i32) -> f64 {
    match (sym, ox) {
        ("O", _) => -180.0,
        ("H", 1) => -55.0,
        ("F", -1) => -310.0,
        ("Cl", -1) => -165.0,
        ("Br", -1) => -130.0,
        ("I", -1) => -95.0,
        ("P", 5) => -520.0,
        ("P", 3) => -340.0,
        ("Se", 4) => -110.0,
        ("Se", 6) => -160.0,
        ("Zr", 4) => -740.0,
        ("S", -2) => -65.0,
        ("S", 6) => -410.0,
        ("S", 4) => -290.0,
        ("C", 4) => -360.0,
        ("C", 2) => -210.0,
        ("Si", 4) => -520.0,
        ("Al", 3) => -510.0,
        ("Fe", 3) => -160.0,
        ("Fe", 2) => -95.0,
        ("Mn", 2) => -210.0,
        ("Mn", 4) => -220.0,
        ("V", 5) => -420.0,
        ("V", 4) => -360.0,
        ("W", 6) => -490.0,
        ("Ca", 2) => -450.0,
        ("Mg", 2) => -420.0,
        ("Ba", 2) => -445.0,
        ("Na", 1) => -220.0,
        ("K", 1) => -235.0,
        ("Li", 1) => -230.0,
        ("Ag", 1) => 38.0,
        ("Cu", 2) => 15.0,
        ("Cu", 1) => -10.0,
        ("Zn", 2) => -175.0,
        ("Pb", 2) => -50.0,
        _ => -120.0 * (ox.max(0) as f64 / 2.0),
    }
}

fn is_gas_formula(formula: &str) -> bool {
    let f = formula.trim();
    f.ends_with("(g)") || f.ends_with("(G)")
}

fn is_aqueous_formula(formula: &str) -> bool {
    let f = formula.trim();
    f.ends_with("a)") || f.ends_with("(a)") || f.ends_with("(ia)")
}

fn detect_species_type(formula: &str, elems: &BTreeMap<String, f64>) -> EstimateSpeciesType {
    if elems.contains_key("C") && elems.contains_key("H") && !elems.contains_key("Na") && !elems.contains_key("K") {
        // Organic hydrocarbon/derivative check (unless simple bicarbonate/formate)
        if !formula.contains("CO3") && !formula.contains("HCO3") && !formula.contains("COOH") {
            return EstimateSpeciesType::Organic;
        }
    }
    EstimateSpeciesType::Inorganic
}

/// Split hydration water `*nH2O` if present and return `(base_formula, n_h2o)`.
fn split_hydrate(formula: &str) -> (&str, f64) {
    if let Some(idx) = formula.find("*H2O") {
        return (&formula[..idx], 1.0);
    }
    if let Some(idx) = formula.find('*') {
        let rest = &formula[idx + 1..];
        if let Some(h_idx) = rest.find("H2O") {
            let coeff_str = rest[..h_idx].trim();
            let n = if coeff_str.is_empty() {
                1.0
            } else {
                coeff_str.parse::<f64>().unwrap_or(1.0)
            };
            return (&formula[..idx], n);
        }
    }
    (formula, 0.0)
}

/// Evaluate thermodynamic estimate for a formula at `temp_c`.
pub fn evaluate_estimate_row(row: &mut EstimateRow, repo: &dyn SpeciesRepository) {
    let formula = row.formula.trim().to_string();
    if formula.is_empty() {
        row.mw = 0.0;
        row.est_h_kj_mol = None;
        row.est_s_j_molk = None;
        row.est_cp_j_molk = None;
        row.est_dg_kj_mol = None;
        row.db_vals = None;
        row.elements.clear();
        return;
    }

    let Ok(elems) = parse_hsc_species_elements(&formula) else {
        row.est_h_kj_mol = None;
        row.est_s_j_molk = None;
        row.est_cp_j_molk = None;
        row.est_dg_kj_mol = None;
        row.db_vals = None;
        row.elements.clear();
        return;
    };

    let mw: f64 = elems.iter().map(|(s, &n)| n * atomic_weight(s)).sum();
    let total_atoms: f64 = elems.values().sum();
    row.mw = mw;

    let ox_sets = enumerate_oxidation_sets(&elems);
    row.available_sets = ox_sets.len().max(1);
    if row.oxidation_set_idx >= row.available_sets {
        row.oxidation_set_idx = 0;
    }
    let active_ox = ox_sets.get(row.oxidation_set_idx);

    row.elements = elems
        .iter()
        .map(|(sym, &amt)| {
            let charge = active_ox.and_then(|m| m.get(sym).copied()).unwrap_or(0);
            let wt = if mw > 0.0 {
                100.0 * amt * atomic_weight(sym) / mw
            } else {
                0.0
            };
            let at = if total_atoms > 0.0 {
                100.0 * amt / total_atoms
            } else {
                0.0
            };
            ElementInfo {
                symbol: sym.clone(),
                amount: amt,
                charge,
                weight_pct: wt,
                atom_pct: at,
            }
        })
        .collect();

    let effective_type = match row.species_type {
        EstimateSpeciesType::NotSpecified => detect_species_type(&formula, &elems),
        other => other,
    };

    let t_k = (row.temp_c + 273.15).max(1.0);
    let is_gas = is_gas_formula(&formula);
    let is_aq = is_aqueous_formula(&formula);

    let (h298_kj, s298_j, cp298_j, dcp_dt) = if effective_type == EstimateSpeciesType::Organic {
        // Benson / Joback-Reid group estimation for hydrocarbons & simple organics
        let n_c = elems.get("C").copied().unwrap_or(0.0);
        let n_h = elems.get("H").copied().unwrap_or(0.0);
        let n_o = elems.get("O").copied().unwrap_or(0.0);

        let is_alkene = row.organic_group == OrganicGroup::Alkenes
            || row.functional_group == FunctionalGroup::Ene
            || (n_c >= 2.0 && (n_h - 2.0 * n_c).abs() < 0.1 && n_o == 0.0);
        let is_alkyne = row.organic_group == OrganicGroup::Alkynes
            || row.functional_group == FunctionalGroup::Yne;

        let (hf_est, s_est, cp_est) = if is_alkene && (n_c - 2.0).abs() < 0.1 && (n_h - 4.0).abs() < 0.1 {
            // 2 * (=CH2) Benson/Joback
            (60.52, 226.78, 40.47)
        } else if is_alkene {
            let hf = 52.0 - 20.6 * (n_c - 2.0).max(0.0);
            let s = 220.0 + 39.0 * (n_c - 2.0).max(0.0);
            let cp = 42.0 + 23.0 * (n_c - 2.0).max(0.0);
            (hf, s, cp)
        } else if is_alkyne {
            let hf = 226.7 - 20.6 * (n_c - 2.0).max(0.0);
            let s = 201.0 + 39.0 * (n_c - 2.0).max(0.0);
            let cp = 44.0 + 22.5 * (n_c - 2.0).max(0.0);
            (hf, s, cp)
        } else {
            // Alkanes CnH(2n+2): -CH3 (-42.2 kJ, 127.2 J/K, 25.9 Cp) + -CH2- (-20.6 kJ, 39.4 J/K, 22.8 Cp)
            let n_ch3 = if n_c >= 2.0 { 2.0 } else { 1.0 };
            let n_ch2 = (n_c - n_ch3).max(0.0);
            let hf = n_ch3 * (-42.2) + n_ch2 * (-22.08) - n_o * 165.0;
            let s = 186.0 + n_c * 33.6 + n_o * 20.0;
            let cp = n_ch3 * 25.9 + n_ch2 * 22.72 + n_o * 18.0;
            (hf, s, cp)
        };
        (hf_est, s_est, cp_est, cp_est * 0.0018)
    } else {
        // Inorganic estimation (Neumann-Kopp Cp, Latimer S, Oxides/Ionic H, + hydrate correction)
        let (_base, n_h2o) = split_hydrate(&formula);
        let mut h_sum = 0.0;
        let mut s_sum = 0.0;
        let mut cp_sum = 0.0;
        let mut slope_sum = 0.0;

        for (sym, &amt) in &elems {
            let ox = active_ox
                .and_then(|m| m.get(sym).copied())
                .unwrap_or_else(|| common_ox_states(sym)[0]);
            h_sum += amt * inorganic_enthalpy_contrib(sym, ox);
            s_sum += amt * latimer_element_entropy(sym, ox);
            let (cp_e, sl_e) = element_kopp_cp_params(sym);
            cp_sum += amt * cp_e;
            slope_sum += amt * sl_e;
        }

        // Hydrate crystal water adjustment
        if n_h2o > 0.0 {
            // H2O in crystal contributes ~39.3 J/(mol*K) to S and ~40.5 J/(mol*K) to Cp
            s_sum += n_h2o * 4.5;
            cp_sum += n_h2o * 4.5;
            h_sum -= n_h2o * 25.0;
        }

        // Gas phase adjustment (translational + rotational + vibrational entropy)
        if is_gas {
            let n_atoms = total_atoms.max(1.0);
            h_sum += 145.0;
            // Sackur-Tetrode + rotational/vibrational scaling for gas species
            let s_trans = 115.0 + 12.47 * mw.max(2.0).ln();
            let s_rot_vib = if n_atoms <= 1.1 {
                0.0
            } else if n_atoms <= 2.1 {
                45.0
            } else {
                62.0 + 18.0 * (n_atoms - 2.0).min(12.0)
            };
            s_sum = s_trans + s_rot_vib;
            cp_sum = if n_atoms <= 1.1 {
                20.79
            } else if n_atoms <= 2.1 {
                29.1 + 4.0 * (mw / 80.0).min(1.5)
            } else {
                (cp_sum * 0.80).max(33.0)
            };
        } else if is_aq {
            // Aqueous ion / neutral species hydration contribution
            h_sum -= 45.0;
            s_sum = (s_sum * 0.85).max(-120.0);
            cp_sum = (cp_sum * 0.75).max(20.0);
        }

        (h_sum, s_sum, cp_sum, slope_sum)
    };

    // Integrate Cp(T) from 298.15 K to t_k for H(T) and S(T)
    let cp_t = (cp298_j + dcp_dt * (t_k - T_REF_K)).max(4.0);
    let dh_kj = 0.5 * (cp298_j + cp_t) * (t_k - T_REF_K) / 1000.0;
    let ds_j = if (t_k - T_REF_K).abs() > 1e-4 {
        0.5 * (cp298_j + cp_t) * (t_k / T_REF_K).ln()
    } else {
        0.0
    };
    let h_t_kj = h298_kj + dh_kj;
    let s_t_j = s298_j + ds_j;

    // ΔG estimate consistent with HSC Estimate (ΔGf°(298) ≈ Hf° - T*(S° - Σ S°_elem))
    let s_elem_ref: f64 = elems
        .iter()
        .map(|(sym, &amt)| {
            let s_e = match sym.as_str() {
                "O" => 102.57,
                "H" => 65.34,
                "N" => 95.81,
                "Cl" => 111.54,
                "F" => 101.39,
                "S" => 32.05,
                "C" => 5.74,
                "Si" => 18.81,
                "Fe" => 27.28,
                "Cu" => 33.15,
                "Ag" => 42.55,
                "Mg" => 32.67,
                "Ca" => 41.59,
                "Ba" => 62.42,
                "Al" => 28.30,
                "K" => 64.68,
                "Na" => 51.30,
                "Mn" => 32.01,
                "V" => 28.91,
                "W" => 32.64,
                _ => 35.0,
            };
            amt * s_e
        })
        .sum();
    let dg_t_kj = h_t_kj - t_k * (s_t_j - s_elem_ref) / 1000.0;

    row.est_h_kj_mol = Some(h_t_kj);
    row.est_s_j_molk = Some(s_t_j);
    row.est_cp_j_molk = Some(cp_t);
    row.est_dg_kj_mol = Some(dg_t_kj);

    // Lookup experimental Database values in Maindb6 if available
    if let Some(sp) = repo.get_species(&formula) {
        if let Ok((pt, _)) = evaluate_species_si(&sp, t_k, false, false) {
            let dg_db = (pt.h / 1000.0) - t_k * (pt.s - s_elem_ref) / 1000.0;
            row.db_vals = Some(DatabaseRefValues {
                h_kj_mol: pt.h / 1000.0,
                s_j_molk: pt.s,
                cp_j_molk: pt.cp,
                dg_kj_mol: dg_db,
            });
        } else {
            row.db_vals = None;
        }
    } else {
        row.db_vals = None;
    }
}

/// Generate a `(T_K, Cp)` curve for a species row over `[t_min_k, t_max_k]` in `steps` intervals.
pub fn generate_cp_curve(
    row: &EstimateRow,
    repo: &dyn SpeciesRepository,
    t_min_k: f64,
    t_max_k: f64,
    steps: usize,
) -> Vec<(f64, f64)> {
    let steps = steps.max(4);
    let mut pts = Vec::with_capacity(steps + 1);
    let dt = (t_max_k - t_min_k) / (steps as f64);

    for i in 0..=steps {
        let t = t_min_k + (i as f64) * dt;
        let mut tmp = row.clone();
        tmp.temp_c = t - 273.15;
        evaluate_estimate_row(&mut tmp, repo);
        let cp = tmp.est_cp_j_molk.unwrap_or(40.0);
        pts.push((t, cp));
    }
    pts
}

/// Fit 4-parameter Kelley polynomial `Cp(T) = A + B*1e-3*T + C*1e5*T^-2 + D*1e-6*T^2`
/// by solving the 4x4 normal equations via Gaussian elimination with partial pivoting.
pub fn fit_kelley_cp(points: &[(f64, f64)]) -> KelleyCoefficients {
    if points.len() < 4 {
        let avg = if points.is_empty() {
            40.0
        } else {
            points.iter().map(|p| p.1).sum::<f64>() / (points.len() as f64)
        };
        return KelleyCoefficients {
            a: avg,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        };
    }

    let mut ata = [[0.0_f64; 4]; 4];
    let mut aty = [0.0_f64; 4];

    for &(t, cp) in points {
        let t_safe = t.max(10.0);
        let x = [
            1.0,
            1.0e-3 * t_safe,
            1.0e5 / (t_safe * t_safe),
            1.0e-6 * t_safe * t_safe,
        ];
        for r in 0..4 {
            aty[r] += x[r] * cp;
            for c in 0..4 {
                ata[r][c] += x[r] * x[c];
            }
        }
    }

    // Solve 4x4 system with partial pivoting
    let mut aug = [[0.0_f64; 5]; 4];
    for r in 0..4 {
        for c in 0..4 {
            aug[r][c] = ata[r][c];
        }
        aug[r][4] = aty[r];
    }

    for col in 0..4 {
        let mut piv = col;
        let mut best = aug[col][col].abs();
        for r in (col + 1)..4 {
            if aug[r][col].abs() > best {
                best = aug[r][col].abs();
                piv = r;
            }
        }
        if piv != col {
            aug.swap(col, piv);
        }
        let diag = aug[col][col];
        if diag.abs() < 1e-14 {
            continue;
        }
        for c in col..5 {
            aug[col][c] /= diag;
        }
        for r in 0..4 {
            if r == col {
                continue;
            }
            let factor = aug[r][col];
            for c in col..5 {
                aug[r][c] -= factor * aug[col][c];
            }
        }
    }

    KelleyCoefficients {
        a: aug[0][4],
        b: aug[1][4],
        c: aug[2][4],
        d: aug[3][4],
    }
}

/// Build a `ChemicalSpecies` record from an estimated row + fitted Kelley coefficients.
pub fn build_estimated_species(
    row: &EstimateRow,
    t_min_k: f64,
    t_max_k: f64,
    kelley: KelleyCoefficients,
) -> ChemicalSpecies {
    let phase = if is_gas_formula(&row.formula) {
        Phase::Gas
    } else if row.formula.ends_with("(l)") {
        Phase::Liquid
    } else {
        Phase::Solid
    };

    ChemicalSpecies {
        formula: row.formula.clone(),
        name: format!("{} (Estimated)", row.formula),
        phase,
        molecular_weight: row.mw,
        density_g_cm3: if phase == Phase::Gas { 0.001 } else { 3.0 },
        charge: 0,
        intervals: vec![SpeciesThermoInterval {
            t_min: t_min_k,
            t_max: t_max_k,
            phase,
            kelley,
            h_or_delta_h_tr: row.est_h_kj_mol.unwrap_or(0.0) * 1000.0,
            s_or_delta_s_tr: row.est_s_j_molk.unwrap_or(50.0),
            reference: "HSC Estimate".to_string(),
        }],
    }
}

#[derive(Debug, Clone, Default)]
pub struct EstimateParityStats {
    pub species_count: usize,
    pub max_error: f64,
    pub max_error_species: String,
    pub mean_error: f64,
    pub median_error: f64,
    pub rmse: f64,
}

/// Compute parity statistics (`Species Count`, `Max Error`, `Mean`, `Median`, `RMSE`) for Fig. 5.
pub fn compute_parity_stats(pairs: &[(&str, f64, f64)]) -> EstimateParityStats {
    if pairs.is_empty() {
        return EstimateParityStats::default();
    }
    let mut abs_errs: Vec<(f64, &str)> = pairs
        .iter()
        .map(|&(name, est, db)| ((est - db).abs(), name))
        .collect();
    abs_errs.sort_by(|a, b| a.0.total_cmp(&b.0));

    let n = abs_errs.len();
    let max_pair = abs_errs[n - 1];
    let mean = abs_errs.iter().map(|x| x.0).sum::<f64>() / (n as f64);
    let median = if n % 2 == 1 {
        abs_errs[n / 2].0
    } else {
        0.5 * (abs_errs[n / 2 - 1].0 + abs_errs[n / 2].0)
    };
    let mse = abs_errs.iter().map(|x| x.0 * x.0).sum::<f64>() / (n as f64);

    EstimateParityStats {
        species_count: n,
        max_error: max_pair.0,
        max_error_species: max_pair.1.to_string(),
        mean_error: mean,
        median_error: median,
        rmse: mse.sqrt(),
    }
}

/// Classic `H, S, Cp Estimate` showcase rows (`Na2SO4`, `AlPO4`, `PbCl2(g)`, `ZrSiO4`, `MgO*Al2O3`, ...).
pub fn classic_hsc_estimate_rows() -> Vec<EstimateRow> {
    [
        "Na2SO4",
        "AlPO4",
        "PbCl2(g)",
        "ZrSiO4",
        "MgO*Al2O3",
        "LiOH(g)",
        "SeO2(g)",
        "KOP",
        "CaCO3",
        "CaO*Al2O3*2SiO2",
        "POP",
    ]
    .into_iter()
    .map(EstimateRow::new)
    .collect()
}

/// Default Fig. 1 showcase rows (`AgCl`, `FeMnO4`, `Ba(OH)2`, `CaAl2SiO6`, `C8H18(g)`, `MgSO4`, ...).
pub fn default_fig1_rows() -> Vec<EstimateRow> {
    let mut rows = vec![
        EstimateRow::new("AgCl"),
        EstimateRow::new("FeMnO4"),
        EstimateRow::new("Ba(OH)2"),
        EstimateRow::new("CaAl2SiO6"),
        {
            let mut r = EstimateRow::new("C8H18(g)");
            r.species_type = EstimateSpeciesType::Organic;
            r.organic_group = OrganicGroup::Alkanes;
            r
        },
        EstimateRow::new("MgSO4"),
        EstimateRow::new("MgSO4*2H2O"),
        EstimateRow::new("K2O"),
        EstimateRow::new("K2SO4"),
        EstimateRow::new("KHCO3"),
        EstimateRow::new("NaCOOH"),
        EstimateRow::new("NaCOOH(g)"),
    ];
    for r in &mut rows {
        if r.species_type == EstimateSpeciesType::NotSpecified
            && matches!(
                r.formula.as_str(),
                "AgCl" | "FeMnO4" | "Ba(OH)2" | "CaAl2SiO6" | "MgSO4" | "MgSO4*2H2O" | "K2O" | "K2SO4"
            )
        {
            r.species_type = EstimateSpeciesType::Inorganic;
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyRepo;
    impl SpeciesRepository for EmptyRepo {
        fn get_species(&self, _formula: &str) -> Option<ChemicalSpecies> {
            None
        }
        fn get_reference_element(&self, _symbol: &str) -> Option<ChemicalSpecies> {
            None
        }
    }

    #[test]
    fn fevo4_oxidation_sets_and_element_info() {
        let mut r = EstimateRow::new("FeVO4");
        evaluate_estimate_row(&mut r, &EmptyRepo);
        assert!(r.available_sets >= 1);
        assert!((r.mw - 170.783).abs() < 0.05);
        assert_eq!(r.elements.len(), 3);
    }

    #[test]
    fn mixed_oxide_and_pop_molecular_weights() {
        let mut r1 = EstimateRow::new("CaO*Al2O3*2SiO2");
        evaluate_estimate_row(&mut r1, &EmptyRepo);
        assert!((r1.mw - 278.21).abs() < 0.1);

        let mut r2 = EstimateRow::new("POP");
        evaluate_estimate_row(&mut r2, &EmptyRepo);
        assert!((r2.mw - 77.95).abs() < 0.1);
    }

    #[test]
    fn kelley_fit_recovers_exact_coefficients() {
        let target = KelleyCoefficients {
            a: 193.371,
            b: -104.887,
            c: -44.022,
            d: 99.354,
        };
        let pts: Vec<(f64, f64)> = (0..50)
            .map(|i| {
                let t = 298.15 + (i as f64) * 6.0;
                (t, target.cp(t))
            })
            .collect();
        let fitted = fit_kelley_cp(&pts);
        assert!((fitted.a - target.a).abs() < 1e-4);
        assert!((fitted.b - target.b).abs() < 1e-4);
        assert!((fitted.c - target.c).abs() < 1e-4);
        assert!((fitted.d - target.d).abs() < 1e-4);
    }

    #[test]
    fn parity_stats_match_definition() {
        let stats = compute_parity_stats(&[("A", 10.0, 12.0), ("B", 25.0, 20.0), ("C", 5.0, 5.0)]);
        assert_eq!(stats.species_count, 3);
        assert!((stats.max_error - 5.0).abs() < 1e-9);
        assert_eq!(stats.max_error_species, "B");
        assert!((stats.median_error - 2.0).abs() < 1e-9);
    }
}
