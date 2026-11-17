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

/// Atomic weights (g/mol) for elements H..U.
pub fn atomic_weight(sym: &str) -> f64 {
    match sym {
        "H" => 1.008,
        "He" => 4.0026,
        "Li" => 6.941,
        "Be" => 9.0122,
        "B" => 10.811,
        "C" => 12.011,
        "N" => 14.007,
        "O" => 15.999,
        "F" => 18.998,
        "Ne" => 20.180,
        "Na" => 22.990,
        "Mg" => 24.305,
        "Al" => 26.982,
        "Si" => 28.086,
        "P" => 30.974,
        "S" => 32.065,
        "Cl" => 35.453,
        "Ar" => 39.948,
        "K" => 39.098,
        "Ca" => 40.078,
        "Sc" => 44.956,
        "Ti" => 47.867,
        "V" => 50.942,
        "Cr" => 51.996,
        "Mn" => 54.938,
        "Fe" => 55.845,
        "Co" => 58.933,
        "Ni" => 58.693,
        "Cu" => 63.546,
        "Zn" => 65.38,
        "Ga" => 69.723,
        "Ge" => 72.64,
        "As" => 74.922,
        "Se" => 78.96,
        "Br" => 79.904,
        "Kr" => 83.798,
        "Rb" => 85.468,
        "Sr" => 87.62,
        "Y" => 88.906,
        "Zr" => 91.224,
        "Nb" => 92.906,
        "Mo" => 95.96,
        "Ru" => 101.07,
        "Rh" => 102.906,
        "Pd" => 106.42,
        "Ag" => 107.868,
        "Cd" => 112.411,
        "In" => 114.818,
        "Sn" => 118.710,
        "Sb" => 121.760,
        "Te" => 127.60,
        "I" => 126.904,
        "Xe" => 131.293,
        "Cs" => 132.905,
        "Ba" => 137.327,
        "La" => 138.905,
        "Ce" => 140.116,
        "Nd" => 144.242,
        "Hf" => 178.49,
        "Ta" => 180.948,
        "W" => 183.84,
        "Re" => 186.207,
        "Os" => 190.23,
        "Ir" => 192.217,
        "Pt" => 195.084,
        "Au" => 196.967,
        "Hg" => 200.59,
        "Tl" => 204.383,
        "Pb" => 207.2,
        "Bi" => 208.98,
        "Th" => 232.038,
        "U" => 238.029,
        _ => 50.0,
    }
}

/// Common oxidation states for elements (ordered by prevalence).
fn common_ox_states(sym: &str) -> &'static [i32] {
    match sym {
        "He" | "Ne" | "Ar" | "Kr" | "Xe" => &[0],
        "O" => &[-2],
        "H" => &[1, -1],
        "F" => &[-1],
        "Cl" | "Br" | "I" => &[-1, 1, 5, 7, 3],
        "Li" | "Na" | "K" | "Rb" | "Cs" | "Ag" => &[1],
        "Be" | "Mg" | "Ca" | "Sr" | "Ba" | "Zn" | "Cd" | "Hg" => &[2],
        "Al" | "B" | "Ga" | "In" | "Sc" | "Y" | "La" => &[3],
        "Si" | "Ti" | "Zr" | "Hf" | "Ge" => &[4, 3],
        "C" => &[4, 2, -4],
        "N" => &[5, 3, -3],
        "P" | "As" | "Sb" | "Bi" => &[5, 3, -3],
        "S" | "Se" | "Te" => &[6, 4, -2],
        "Fe" => &[3, 2, 4, 6],
        "Mn" => &[2, 4, 3, 7, 6],
        "V" => &[5, 4, 3, 2],
        "Nb" | "Ta" => &[5, 4],
        "Cr" => &[3, 6, 2],
        "Cu" => &[2, 1],
        "Co" | "Ni" => &[2, 3],
        "Pb" | "Sn" => &[2, 4],
        "W" | "Mo" => &[6, 4],
        "Re" => &[7, 4, 6],
        "Pt" | "Pd" | "Ru" | "Rh" | "Ir" | "Os" => &[2, 4, 3],
        "Au" => &[3, 1],
        "Tl" => &[1, 3],
        "Ce" => &[3, 4],
        "Th" => &[4],
        "U" => &[6, 4],
        _ => &[2, 3, 1],
    }
}

/// Enumerate valid charge-neutral oxidation number assignments for up to 6 elements.
pub fn enumerate_oxidation_sets(elems: &BTreeMap<String, f64>) -> Vec<BTreeMap<String, i32>> {
    enumerate_oxidation_sets_with_charge(elems, 0)
}

/// Enumerate valid oxidation number assignments matching total formula charge `net_charge`.
pub fn enumerate_oxidation_sets_with_charge(
    elems: &BTreeMap<String, f64>,
    net_charge: i32,
) -> Vec<BTreeMap<String, i32>> {
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
        target_charge: f64,
        out: &mut Vec<BTreeMap<String, i32>>,
    ) {
        if out.len() >= 8 {
            return;
        }
        if idx == keys.len() {
            if (sum_charge - target_charge).abs() < 1e-3 {
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
                target_charge,
                out,
            );
        }
        current.remove(sym);
    }

    let mut cur = BTreeMap::new();
    backtrack(0, &keys, elems, &mut cur, 0.0, net_charge as f64, &mut results);
    results
}

/// Solid-state Neumann-Kopp / Kopp atomic Cp(298.15 K) in J/(mol·K) and temperature slope dCp/dT.
fn element_kopp_cp_params(sym: &str) -> (f64, f64) {
    match sym {
        "H" => (9.6, 0.012),
        "Be" => (9.2, 0.014),
        "B" => (11.1, 0.015),
        "C" => (8.5, 0.014),
        "N" => (12.6, 0.010),
        "O" => (16.7, 0.011),
        "F" => (20.9, 0.010),
        "Cl" => (25.1, 0.010),
        "Br" => (26.0, 0.010),
        "I" => (26.8, 0.010),
        "S" => (22.6, 0.012),
        "Se" => (25.0, 0.010),
        "Te" => (25.7, 0.009),
        "P" => (23.8, 0.012),
        "As" => (24.6, 0.009),
        "Sb" => (25.2, 0.008),
        "Bi" => (25.5, 0.008),
        "Si" => (20.0, 0.010),
        "Ge" => (23.3, 0.009),
        "Sn" => (26.9, 0.009),
        "Pb" => (26.8, 0.009),
        "Li" => (21.5, 0.012),
        "Na" => (28.2, 0.012),
        "K" => (29.6, 0.012),
        "Rb" => (31.1, 0.011),
        "Cs" => (32.2, 0.011),
        "Mg" => (22.5, 0.010),
        "Ca" => (25.9, 0.010),
        "Sr" => (26.4, 0.010),
        "Ba" => (28.1, 0.010),
        "Al" => (24.2, 0.011),
        "Ga" => (25.9, 0.010),
        "In" => (26.7, 0.009),
        "Ti" => (25.0, 0.010),
        "Zr" => (25.4, 0.009),
        "Hf" => (25.7, 0.008),
        "V" => (24.9, 0.009),
        "Nb" | "Ta" => (25.0, 0.008),
        "Cr" => (23.3, 0.010),
        "Mo" => (24.1, 0.007),
        "W" => (24.3, 0.006),
        "Mn" => (26.3, 0.010),
        "Fe" => (25.1, 0.012),
        "Co" => (24.8, 0.010),
        "Ni" => (26.1, 0.010),
        "Cu" => (24.4, 0.008),
        "Zn" => (25.4, 0.010),
        "Cd" => (26.0, 0.009),
        "Hg" => (27.9, 0.008),
        "Ag" => (25.4, 0.008),
        "Au" | "Pt" | "Pd" => (25.8, 0.007),
        "La" | "Ce" => (27.0, 0.008),
        "Th" | "U" => (27.5, 0.008),
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
    if sym == "F" && charge < 0 {
        return 22.0;
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
    if sym == "N" || sym == "P" {
        return 15.0;
    }
    // Latimer formula for cations: S_cat ≈ 1.5 * R * ln(AW) - 0.94 * |z| * 4.184 + 12.0
    let aw = atomic_weight(sym).max(2.0);
    (12.47 * aw.ln() - 3.93 * (charge.abs() as f64) + 8.0).max(10.0)
}

/// Standard element molar entropy at 298.15 K (J/(mol·K)) per gram-atom for ΔGf° calculations.
fn element_std_entropy_298(sym: &str) -> f64 {
    match sym {
        "H" => 65.34,
        "He" => 126.15,
        "Li" => 29.12,
        "Be" => 9.50,
        "B" => 5.90,
        "C" => 5.74,
        "N" => 95.81,
        "O" => 102.57,
        "F" => 101.39,
        "Ne" => 146.33,
        "Na" => 51.30,
        "Mg" => 32.67,
        "Al" => 28.30,
        "Si" => 18.81,
        "P" => 41.09,
        "S" => 32.05,
        "Cl" => 111.54,
        "Ar" => 154.84,
        "K" => 64.68,
        "Ca" => 41.59,
        "Sc" => 34.64,
        "Ti" => 30.72,
        "V" => 28.91,
        "Cr" => 23.77,
        "Mn" => 32.01,
        "Fe" => 27.28,
        "Co" => 30.04,
        "Ni" => 29.87,
        "Cu" => 33.15,
        "Zn" => 41.63,
        "Ga" => 40.88,
        "Ge" => 31.09,
        "As" => 35.10,
        "Se" => 42.44,
        "Br" => 76.11,
        "Rb" => 76.78,
        "Sr" => 55.70,
        "Y" => 44.43,
        "Zr" => 38.99,
        "Nb" => 36.40,
        "Mo" => 28.66,
        "Ru" => 28.53,
        "Rh" => 31.51,
        "Pd" => 37.57,
        "Ag" => 42.55,
        "Cd" => 51.80,
        "In" => 57.82,
        "Sn" => 51.18,
        "Sb" => 45.69,
        "Te" => 49.71,
        "I" => 58.07,
        "Cs" => 85.23,
        "Ba" => 62.42,
        "La" => 56.90,
        "Ce" => 72.00,
        "Hf" => 43.56,
        "Ta" => 41.51,
        "W" => 32.64,
        "Re" => 36.86,
        "Pt" => 41.63,
        "Au" => 47.40,
        "Hg" => 75.90,
        "Tl" => 64.18,
        "Pb" => 64.80,
        "Bi" => 56.74,
        "Th" => 51.80,
        "U" => 50.20,
        _ => 35.0,
    }
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
        ("P", 5) => -460.0,
        ("P", 3) => -320.0,
        ("As", 5) => -290.0,
        ("As", 3) => -175.0,
        ("Se", 4) => -110.0,
        ("Se", 6) => -160.0,
        ("B", 3) => -385.0,
        ("Ti", 4) => -580.0,
        ("Zr", 4) => -740.0,
        ("Hf", 4) => -760.0,
        ("S", -2) => -65.0,
        ("S", 6) => -210.0,
        ("S", 4) => -165.0,
        ("C", 4) => -260.0,
        ("C", 2) => -160.0,
        ("N", 5) => -120.0,
        ("Si", 4) => -550.0,
        ("Al", 3) => -555.0,
        ("Cr", 3) => -320.0,
        ("Cr", 6) => -310.0,
        ("Fe", 3) => -160.0,
        ("Fe", 2) => -95.0,
        ("Mn", 2) => -210.0,
        ("Mn", 4) => -220.0,
        ("V", 5) => -420.0,
        ("V", 4) => -360.0,
        ("Nb", 5) => -490.0,
        ("Ta", 5) => -525.0,
        ("Mo", 6) => -390.0,
        ("W", 6) => -480.0,
        ("Be", 2) => -425.0,
        ("Mg", 2) => -420.0,
        ("Ca", 2) => -450.0,
        ("Sr", 2) => -415.0,
        ("Ba", 2) => -370.0,
        ("Li", 1) => -208.0,
        ("Na", 1) => -120.0,
        ("K", 1) => -92.0,
        ("Rb", 1) => -85.0,
        ("Cs", 1) => -82.0,
        ("Ag", 1) => 38.0,
        ("Cu", 2) => 15.0,
        ("Cu", 1) => -10.0,
        ("Co", 2) => -65.0,
        ("Ni", 2) => -65.0,
        ("Zn", 2) => -175.0,
        ("Cd", 2) => -90.0,
        ("Sn", 4) => -220.0,
        ("Sn", 2) => -110.0,
        ("Pb", 2) => -50.0,
        ("Bi", 3) => -45.0,
        ("La", 3) => -620.0,
        ("Ce", 4) => -690.0,
        ("U", 4) => -720.0,
        _ => -120.0 * (ox.max(0) as f64 / 2.0),
    }
}

/// Detect phase and formal net charge from trailing HSC species-type parentheses `(g)`, `(M)`, `(-a)`, etc.
fn parse_formula_metadata(formula: &str) -> (&str, i32, Option<Phase>) {
    let f = formula.trim();
    if f == "e-" || f == "e(-)" {
        return (f, -1, Some(Phase::AqueousIon));
    }
    if let Some(open) = f.rfind('(') {
        if f.ends_with(')') && open < f.len() - 2 {
            let suf = &f[open + 1..f.len() - 1];
            match suf {
                "g" | "G" => return (&f[..open], 0, Some(Phase::Gas)),
                "l" | "L" => return (&f[..open], 0, Some(Phase::Liquid)),
                "s" | "D" | "M" | "R" | "Q" | "A" | "T" | "E" | "II" | "III" => {
                    return (&f[..open], 0, Some(Phase::Solid))
                }
                "ia" | "ao" | "ai" => return (&f[..open], 0, Some(Phase::AqueousNeutral)),
                "a" | "aq" => return (&f[..open], 0, Some(Phase::AqueousNeutral)),
                "+" | "+a" => return (&f[..open], 1, Some(Phase::AqueousIon)),
                "-" | "-a" => return (&f[..open], -1, Some(Phase::AqueousIon)),
                "+g" => return (&f[..open], 1, Some(Phase::Gas)),
                "-g" => return (&f[..open], -1, Some(Phase::Gas)),
                _ => {
                    let is_gas = suf.ends_with('g');
                    let trimmed = suf.trim_end_matches(['a', 'g']);
                    if let Some(rest) = trimmed.strip_prefix('+') {
                        let z = if rest.is_empty() { 1 } else { rest.parse().unwrap_or(1) };
                        let ph = if is_gas { Phase::Gas } else { Phase::AqueousIon };
                        return (&f[..open], z, Some(ph));
                    }
                    if let Some(rest) = trimmed.strip_prefix('-') {
                        let z = if rest.is_empty() { 1 } else { rest.parse().unwrap_or(1) };
                        let ph = if is_gas { Phase::Gas } else { Phase::AqueousIon };
                        return (&f[..open], -z, Some(ph));
                    }
                    // High-pressure labels (e.g. Ar(100bar)) and HSC organic isomer+phase tags
                    // like C6H6(BZEg), C8H18(OCTl), C8H18(224Tg), C8H18(2233TMB)
                    if suf.ends_with("bar") || suf.ends_with("atm") {
                        return (&f[..open], 0, Some(Phase::Gas));
                    }
                    let (iso_core, iso_phase) = if let Some(b) = suf.strip_suffix(['g', 'G']) {
                        (b, Phase::Gas)
                    } else if let Some(b) = suf.strip_suffix(['l', 'L']) {
                        (b, Phase::Liquid)
                    } else if let Some(b) = suf.strip_suffix('a') {
                        (b, Phase::AqueousNeutral)
                    } else if let Some(b) = suf.strip_suffix('s') {
                        (b, Phase::Solid)
                    } else {
                        (suf, Phase::Solid)
                    };
                    if iso_core.len() >= 3
                        && iso_core != "COOH"
                        && iso_core
                            .bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
                    {
                        return (&f[..open], 0, Some(iso_phase));
                    }
                }
            }
        }
    }
    (f, 0, None)
}

/// Parse elemental composition specifically following HSC Estimate Section 16.3 rules.
pub fn parse_estimate_formula(formula: &str) -> Result<(BTreeMap<String, f64>, i32, Option<Phase>), String> {
    let trimmed = formula.trim();
    if trimmed.is_empty() {
        return Err("Empty formula".into());
    }
    // Reject HSC-invalid pattern `*(...)` such as `AlO*(OH)` (§16.3)
    if trimmed.contains("*(") {
        return Err(format!("Invalid '*(' syntax in '{trimmed}'"));
    }
    let (core, charge, phase_opt) = parse_formula_metadata(trimmed);
    let elems = parse_hsc_species_elements(core).map_err(|e| e.to_string())?;
    Ok((elems, charge, phase_opt))
}

fn is_gas_formula(formula: &str) -> bool {
    parse_formula_metadata(formula).2 == Some(Phase::Gas)
}

fn is_liquid_formula(formula: &str) -> bool {
    parse_formula_metadata(formula).2 == Some(Phase::Liquid)
}

fn is_aqueous_formula(formula: &str) -> bool {
    matches!(
        parse_formula_metadata(formula).2,
        Some(Phase::AqueousIon | Phase::AqueousNeutral)
    )
}

fn detect_species_type(formula: &str, elems: &BTreeMap<String, f64>) -> EstimateSpeciesType {
    let has_metal = elems.keys().any(|k| {
        !matches!(
            k.as_str(),
            "C" | "H" | "O" | "N" | "S" | "P" | "F" | "Cl" | "Br" | "I" | "Si" | "B"
        )
    });
    if !has_metal && elems.contains_key("C") && elems.contains_key("H") {
        if !formula.contains("CO3") && !formula.contains("HCO3") {
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

/// Estimate organic 298.15 K `(Hf°, S°, Cp°, vib_theta_K)` using Benson / Joback group additivity.
fn estimate_organic_298(
    elems: &BTreeMap<String, f64>,
    org_group: OrganicGroup,
    func_group: FunctionalGroup,
    is_liq: bool,
) -> (f64, f64, f64) {
    let n_c = elems.get("C").copied().unwrap_or(0.0);
    let n_h = elems.get("H").copied().unwrap_or(0.0);
    let n_o = elems.get("O").copied().unwrap_or(0.0);

    let (mut hf, mut s, mut cp) =
        if org_group == OrganicGroup::Aromatics || (org_group == OrganicGroup::None && n_c >= 6.0 && n_h <= n_c + 1.0) {
            let n_subst = (n_c - 6.0).max(0.0);
            (82.9 - 32.0 * n_subst, 269.2 + 39.0 * n_subst, 82.4 + 26.0 * n_subst)
        } else if org_group == OrganicGroup::Acids || func_group == FunctionalGroup::Carboxyl {
            let n_alkyl = (n_c - 1.0).max(0.0);
            (-378.6 - 20.6 * n_alkyl, 248.9 + 39.0 * n_alkyl, 45.2 + 22.8 * n_alkyl)
        } else if org_group == OrganicGroup::Alcohols || func_group == FunctionalGroup::Hydroxyl {
            let n_ch2 = (n_c - 1.0).max(0.0);
            (-201.0 - 20.6 * n_ch2, 239.9 + 39.5 * n_ch2, 44.1 + 22.8 * n_ch2)
        } else if matches!(func_group, FunctionalGroup::Triene) {
            let n_ex = (n_c - 6.0).max(0.0);
            (168.0 - 20.6 * n_ex, 315.0 + 39.0 * n_ex, 102.0 + 23.0 * n_ex)
        } else if matches!(func_group, FunctionalGroup::Diene) {
            let n_ex = (n_c - 4.0).max(0.0);
            (110.0 - 20.6 * n_ex, 278.7 + 39.0 * n_ex, 79.5 + 23.0 * n_ex)
        } else if org_group == OrganicGroup::Alkenes
            || func_group == FunctionalGroup::Ene
            || (n_c >= 2.0 && (n_h - 2.0 * n_c).abs() < 0.1 && n_o == 0.0)
        {
            if (n_c - 2.0).abs() < 0.1 && (n_h - 4.0).abs() < 0.1 {
                (60.52, 226.78, 40.47)
            } else {
                let n_ex = (n_c - 2.0).max(0.0);
                (52.0 - 20.6 * n_ex, 220.0 + 39.0 * n_ex, 42.0 + 23.0 * n_ex)
            }
        } else if org_group == OrganicGroup::Alkynes || func_group == FunctionalGroup::Yne {
            let n_ex = (n_c - 2.0).max(0.0);
            (226.7 - 20.6 * n_ex, 201.0 + 39.0 * n_ex, 44.0 + 22.5 * n_ex)
        } else {
            // Alkanes & oxygenated aliphatic organics
            let n_ch3 = if n_c >= 2.0 { 2.0 } else { 1.0 };
            let n_ch2 = (n_c - n_ch3).max(0.0);
            (
                n_ch3 * (-42.2) + n_ch2 * (-22.08) - n_o * 165.0,
                186.0 + n_c * 33.6 + n_o * 20.0,
                n_ch3 * 25.9 + n_ch2 * 22.72 + n_o * 18.0,
            )
        };

    if is_liq {
        let dh_vap = 26.0 + 3.8 * n_c.min(12.0);
        hf -= dh_vap;
        s = (s - 95.0).max(40.0);
        cp += 24.0 + 2.5 * n_c.min(12.0);
    }
    (hf, s, cp)
}

/// Evaluate Einstein-Kelley curved Cp(T) anchored identically at `(298.15 K, cp298_j)`.
fn eval_einstein_cp(t_k: f64, cp298_j: f64, dcp_298: f64, cp_inf_boost: f64, is_monatomic_gas: bool) -> f64 {
    if is_monatomic_gas {
        return cp298_j;
    }
    let einstein = |x: f64| -> f64 {
        let xs = x.clamp(1e-3, 40.0);
        let sh = (0.5 * xs).sinh();
        let val = (0.5 * xs / sh).powi(2);
        val
    };
    let theta_k = 620.0;
    let f298 = einstein(theta_k / T_REF_K);
    let ft = einstein(theta_k / t_k.max(10.0));
    let curved = cp298_j + cp_inf_boost * (ft - f298) / (1.0 - f298).max(0.15);
    let high_t_lin = dcp_298 * 0.45 * (t_k - T_REF_K);
    (curved + high_t_lin).max(4.0)
}

/// Integrate Cp(T) from `T_REF_K` to `t_k` using 16-point Simpson quadrature for exact `H(T), S(T)`.
fn integrate_cp_simpson<F>(t_k: f64, cp_fn: F) -> (f64, f64)
where
    F: Fn(f64) -> f64,
{
    if (t_k - T_REF_K).abs() < 1e-6 {
        return (0.0, 0.0);
    }
    let n = 16usize;
    let h = (t_k - T_REF_K) / (n as f64);
    let mut sum_h = 0.0;
    let mut sum_s = 0.0;
    for i in 0..=n {
        let t = T_REF_K + (i as f64) * h;
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        let c = cp_fn(t);
        sum_h += w * c;
        sum_s += w * c / t.max(1.0);
    }
    (sum_h * h / 3000.0, sum_s * h / 3.0)
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

    let Ok((elems, net_charge, _)) = parse_estimate_formula(&formula) else {
        row.mw = 0.0;
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

    let ox_sets = enumerate_oxidation_sets_with_charge(&elems, net_charge);
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
    let is_liq = is_liquid_formula(&formula);
    let is_aq = is_aqueous_formula(&formula);
    let is_monatomic_gas = is_gas && total_atoms <= 1.1;

    let (h298_kj, s298_j, cp298_j, dcp_dt, cp_inf_boost) =
        if effective_type == EstimateSpeciesType::Organic {
            let (hf, s, cp) = estimate_organic_298(
                &elems,
                row.organic_group,
                row.functional_group,
                is_liq,
            );
            (hf, s, cp, cp * 0.0018, cp * 0.42)
        } else {
            let (_base, n_h2o) = split_hydrate(&formula);
            let mut h_sum = 0.0;
            let mut s_sum = 0.0;
            let mut cp_sum = 0.0;
            let mut slope_sum = 0.0;
            let mut alkali_mol = 0.0;
            let mut has_halide_or_oxoanion = false;
            let n_o = elems.get("O").copied().unwrap_or(0.0);

            for (sym, &amt) in &elems {
                let ox = active_ox
                    .and_then(|m| m.get(sym).copied())
                    .unwrap_or_else(|| common_ox_states(sym)[0]);
                h_sum += amt * inorganic_enthalpy_contrib(sym, ox);
                s_sum += amt * latimer_element_entropy(sym, ox);
                let (cp_e, sl_e) = element_kopp_cp_params(sym);
                cp_sum += amt * cp_e;
                slope_sum += amt * sl_e;

                if matches!(sym.as_str(), "Li" | "Na" | "K" | "Rb" | "Cs") {
                    alkali_mol += amt;
                }
                if matches!(sym.as_str(), "F" | "Cl" | "Br" | "I")
                    || (matches!(sym.as_str(), "S" | "N" | "P" | "W" | "Mo" | "Cr")
                        && ox >= 5
                        && n_o > 0.0)
                {
                    has_halide_or_oxoanion = true;
                }
            }

            // Alkali halide / oxoanion stabilization (M+ next to Cl-/F- or SO4/CO3/NO3/WO4)
            if alkali_mol > 0.0 && has_halide_or_oxoanion {
                h_sum -= 130.0 * alkali_mol;
            }
            // High-valence covalent oxoanion slightly tightens O vibrational Cp
            if elems.keys().any(|s| matches!(s.as_str(), "S" | "C" | "N" | "P")) && n_o >= 3.0 {
                cp_sum -= 2.4 * n_o.min(8.0);
            }

            if n_h2o > 0.0 {
                s_sum += n_h2o * 4.5;
                cp_sum += n_h2o * 4.5;
                h_sum -= n_h2o * 25.0;
            }

            let mut inf_boost = cp_sum * 0.28;
            if is_gas {
                let n_atoms = total_atoms.max(1.0);
                h_sum += 145.0;
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
                slope_sum = if is_monatomic_gas { 0.0 } else { cp_sum * 0.00025 };
                inf_boost = if is_monatomic_gas { 0.0 } else { cp_sum * 0.36 };
            } else if is_liq {
                h_sum += 14.0;
                s_sum += 18.0;
                cp_sum *= 1.12;
            } else if is_aq {
                h_sum -= 45.0;
                s_sum = (s_sum * 0.85).max(-120.0);
                cp_sum = (cp_sum * 0.75).max(20.0);
            }

            (h_sum, s_sum, cp_sum, slope_sum, inf_boost)
        };

    let cp_fn = |tk: f64| eval_einstein_cp(tk, cp298_j, dcp_dt, cp_inf_boost, is_monatomic_gas);
    let cp_t = cp_fn(t_k);
    let (dh_kj, ds_j) = integrate_cp_simpson(t_k, cp_fn);
    let h_t_kj = h298_kj + dh_kj;
    let s_t_j = s298_j + ds_j;

    // ΔG° formation at T using Maindb6 reference elements when available, else CODATA fallback
    let s_elem_ref: f64 = elems
        .iter()
        .map(|(sym, &amt)| {
            let s_e = repo
                .get_reference_element(sym)
                .and_then(|ref_sp| {
                    let ok = evaluate_species_si(&ref_sp, t_k, false, false).ok()?;
                    let ref_elems = parse_hsc_species_elements(&ref_sp.formula).ok()?;
                    let denom = ref_elems.get(sym).copied().unwrap_or(1.0).max(1e-9);
                    Some(ok.0.s / denom)
                })
                .unwrap_or_else(|| element_std_entropy_298(sym));
            amt * s_e
        })
        .sum();
    let dg_t_kj = h_t_kj - t_k * (s_t_j - s_elem_ref) / 1000.0;

    row.est_h_kj_mol = Some(h_t_kj);
    row.est_s_j_molk = Some(s_t_j);
    row.est_cp_j_molk = Some(cp_t);
    row.est_dg_kj_mol = Some(dg_t_kj);

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
/// Always anchors interval enthalpy/entropy at `T_REF_K = 298.15 K` regardless of sheet Temp.
pub fn build_estimated_species(
    row: &EstimateRow,
    t_min_k: f64,
    t_max_k: f64,
    kelley: KelleyCoefficients,
) -> ChemicalSpecies {
    build_estimated_species_with_intervals(row, vec![(t_min_k, t_max_k, kelley)], "HSC Estimate")
}

/// Build a `ChemicalSpecies` record with one or more Kelley intervals anchored at `298.15 K`.
pub fn build_estimated_species_with_intervals(
    row: &EstimateRow,
    ranges: Vec<(f64, f64, KelleyCoefficients)>,
    reference: &str,
) -> ChemicalSpecies {
    struct NoopRepo;
    impl SpeciesRepository for NoopRepo {
        fn get_species(&self, _formula: &str) -> Option<ChemicalSpecies> {
            None
        }
        fn get_reference_element(&self, _symbol: &str) -> Option<ChemicalSpecies> {
            None
        }
    }

    let (_, charge, detected_phase) = parse_formula_metadata(&row.formula);
    let phase = detected_phase.unwrap_or(Phase::Solid);

    let mut ref_row = row.clone();
    ref_row.temp_c = 25.0;
    evaluate_estimate_row(&mut ref_row, &NoopRepo);
    let h298_j = ref_row.est_h_kj_mol.unwrap_or(0.0) * 1000.0;
    let s298_j = ref_row.est_s_j_molk.unwrap_or(50.0);

    let intervals = ranges
        .into_iter()
        .enumerate()
        .map(|(idx, (t1, t2, kelley))| {
            let t_lo = t1.max(1.0);
            let t_hi = t2.max(t_lo + 1.0);
            SpeciesThermoInterval {
                t_min: t_lo,
                t_max: t_hi,
                phase,
                kelley,
                h_or_delta_h_tr: if idx == 0 { h298_j } else { 0.0 },
                s_or_delta_s_tr: if idx == 0 { s298_j } else { 0.0 },
                reference: reference.to_string(),
            }
        })
        .collect();

    ChemicalSpecies {
        formula: row.formula.trim().to_string(),
        name: format!("{} (Estimated)", row.formula.trim()),
        phase,
        molecular_weight: ref_row.mw,
        density_g_cm3: match phase {
            Phase::Gas => 0.001,
            Phase::Liquid | Phase::AqueousIon | Phase::AqueousNeutral => 1.0,
            Phase::Solid => 3.0,
        },
        charge,
        intervals,
    }
}

/// Fig. 5 oxides & sulfates selection example (`Li2O..CaO`).
pub fn fig5_oxides_rows() -> Vec<EstimateRow> {
    ["Li2O", "Na2O", "Na2SO4", "K2O", "K2SO4", "BeO", "MgO", "CaO"]
        .into_iter()
        .map(EstimateRow::new)
        .collect()
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

    #[test]
    fn section_16_formula_rules_and_ions() {
        // FeS2(M) strips (M) polymorph instead of creating element "M"
        let (elems, chg, ph) = parse_estimate_formula("FeS2(M)").unwrap();
        assert_eq!(elems.len(), 2);
        assert_eq!(chg, 0);
        assert_eq!(ph, Some(Phase::Solid));

        // C(D) strips (D)
        let (elems_cd, _, _) = parse_estimate_formula("C(D)").unwrap();
        assert_eq!(elems_cd.get("C"), Some(&1.0));

        // OH(-a) assigns net charge -1 and valid oxidation charges O=-2, H=+1
        let mut oh = EstimateRow::new("OH(-a)");
        evaluate_estimate_row(&mut oh, &EmptyRepo);
        assert_eq!(oh.elements.len(), 2);
        let h_chg = oh.elements.iter().find(|e| e.symbol == "H").unwrap().charge;
        let o_chg = oh.elements.iter().find(|e| e.symbol == "O").unwrap().charge;
        assert_eq!(h_chg + o_chg, -1);

        // Fractional non-stoichiometry Fe0.998O & nested groups
        let mut feo = EstimateRow::new("Fe0.998O");
        evaluate_estimate_row(&mut feo, &EmptyRepo);
        assert!((feo.mw - (0.998 * 55.845 + 15.999)).abs() < 0.05);

        let (et2o, _, _) = parse_estimate_formula("(C2H5)2O").unwrap();
        assert_eq!(et2o.get("C"), Some(&4.0));
        assert_eq!(et2o.get("H"), Some(&10.0));
        assert_eq!(et2o.get("O"), Some(&1.0));

        // Invalid *(...) pattern rejected per Section 16.3
        assert!(parse_estimate_formula("AlO*(OH)").is_err());

        // HSC6 organic isomer+phase suffixes like C6H6(BZEg) and C8H18(224Tg)
        let (bz, bz_chg, bz_ph) = parse_estimate_formula("C6H6(BZEg)").unwrap();
        assert_eq!(bz.get("C"), Some(&6.0));
        assert_eq!(bz.get("H"), Some(&6.0));
        assert_eq!(bz_chg, 0);
        assert_eq!(bz_ph, Some(Phase::Gas));

        let (oct, _, oct_ph) = parse_estimate_formula("C8H18(224Tg)").unwrap();
        assert_eq!(oct.get("C"), Some(&8.0));
        assert_eq!(oct.get("H"), Some(&18.0));
        assert_eq!(oct_ph, Some(Phase::Gas));
    }

    #[test]
    fn nonlinear_cp_curve_and_organic_groups_and_build_ref_temp() {
        // Na2WO4(g) Cp curve over 298.15..600 K has real 4-parameter Kelley curvature
        let mut nw = EstimateRow::new("Na2WO4(g)");
        evaluate_estimate_row(&mut nw, &EmptyRepo);
        assert!((nw.mw - 293.816).abs() < 0.1);
        let pts = generate_cp_curve(&nw, &EmptyRepo, 298.15, 600.0, 50);
        assert!(pts.last().unwrap().1 > pts.first().unwrap().1 + 10.0);
        let kelley = fit_kelley_cp(&pts);
        assert!(kelley.c.abs() > 0.1, "Expected non-zero Kelley C curvature, got {:?}", kelley);

        // Evaluating at high T must NOT corrupt saved 298.15 K enthalpy in build_estimated_species
        let mut nw_hot = nw.clone();
        nw_hot.temp_c = 600.0;
        evaluate_estimate_row(&mut nw_hot, &EmptyRepo);
        let sp = build_estimated_species(&nw_hot, 298.15, 600.0, kelley);
        assert!((sp.intervals[0].h_or_delta_h_tr / 1000.0 - nw.est_h_kj_mol.unwrap()).abs() < 1e-6);
        assert_eq!(sp.phase, Phase::Gas);

        // Aromatics / Alcohols / Acids produce distinct group-additive values
        let mut benz = EstimateRow::new("C6H6(g)");
        benz.species_type = EstimateSpeciesType::Organic;
        benz.organic_group = OrganicGroup::Aromatics;
        evaluate_estimate_row(&mut benz, &EmptyRepo);
        assert!(benz.est_h_kj_mol.unwrap() > 50.0);

        let mut etoh = EstimateRow::new("C2H5OH(g)");
        etoh.species_type = EstimateSpeciesType::Organic;
        etoh.organic_group = OrganicGroup::Alcohols;
        evaluate_estimate_row(&mut etoh, &EmptyRepo);
        assert!(etoh.est_h_kj_mol.unwrap() < -200.0);

        // Metal carboxylates (NaCOOH) stay Inorganic while CH3COOH(g) auto-detects as Organic
        let elems_na: BTreeMap<String, f64> =
            [("Na", 1.0), ("C", 1.0), ("O", 2.0), ("H", 1.0)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
        assert_eq!(detect_species_type("NaCOOH", &elems_na), EstimateSpeciesType::Inorganic);
        let elems_ac: BTreeMap<String, f64> =
            [("C", 2.0), ("H", 4.0), ("O", 2.0)]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
        assert_eq!(detect_species_type("CH3COOH(g)", &elems_ac), EstimateSpeciesType::Organic);
    }
}
