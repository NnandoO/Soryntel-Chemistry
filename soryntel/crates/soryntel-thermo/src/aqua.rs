//! Module 17: HSC Aqua (`Aqueous Solutions`) — Pitzer / Harvie / Davies electrolyte engine,
//! Osmotic / Colligative / pH (`Osmcalc`, `Colligcalc`, `CPPitzer`, `Istrength`, `pHcalc`),
//! `OwnDB` management, `*.aqu8` persistence, and Laliberté–Cooper (2004) `Density Calculator`.

use crate::aqua_density_data::{DensitySaltEntry, DENSITY_SALTS};
use crate::aqua_pitzer_data::{
    BinaryPitzerEntry, LambdaPitzerEntry, PitzerDatabase, PitzerTempCurve, PsiPitzerEntry,
    ThetaPitzerEntry, ValidityRange,
};
use crate::species::{compute_molecular_weight, parse_hsc_formula_elements};
use crate::{evaluate_species_si, SpeciesCatalogPort};
use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_JOULE};
use std::collections::{BTreeMap, BTreeSet};

/// 1000 / 18.01528 mol H2O per kg water.
pub const MOL_H2O_PER_KG: f64 = 55.508_435_061_791_99;
pub const MW_H2O_G_MOL: f64 = 18.015_28;
pub const MW_H2O_KG_MOL: f64 = 0.018_015_28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AquaMethod {
    #[default]
    Harvie,
    Pitzer,
    Davies,
}

impl AquaMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Harvie => "Harvie",
            Self::Pitzer => "Pitzer",
            Self::Davies => "Davies",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Harvie => "Harvie modification: binary (β0, β1, β2, Cφ), same-sign (θ, Φ), ternary (Ψ) and neutral-ion (λ) parameters.",
            Self::Pitzer => "Pitzer binary model: omits ternary (θ, Φ, Ψ) terms; uses binary and neutral-ion parameters only.",
            Self::Davies => "Extended Debye-Hückel (Eq. 15 'Davies'): disregards all Pitzer interaction parameters.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AquaTempUnit {
    #[default]
    Celsius,
    Kelvin,
}

impl AquaTempUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Kelvin => "K",
        }
    }

    pub fn to_c(self, v: f64) -> f64 {
        match self {
            Self::Celsius => v,
            Self::Kelvin => v - 273.15,
        }
    }

    pub fn to_k(self, v: f64) -> f64 {
        match self {
            Self::Celsius => v + 273.15,
            Self::Kelvin => v,
        }
    }

    pub fn from_c(self, t_c: f64) -> f64 {
        match self {
            Self::Celsius => t_c,
            Self::Kelvin => t_c + 273.15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AquaEnergyUnit {
    #[default]
    Kcal,
    KJ,
}

impl AquaEnergyUnit {
    pub fn h_mol_label(self) -> &'static str {
        match self {
            Self::Kcal => "kcal/mol",
            Self::KJ => "kJ/mol",
        }
    }

    pub fn h_kg_label(self) -> &'static str {
        match self {
            Self::Kcal => "kcal/kg",
            Self::KJ => "kJ/kg",
        }
    }

    pub fn cp_mol_label(self) -> &'static str {
        match self {
            Self::Kcal => "cal/(mol*K)",
            Self::KJ => "J/(mol*K)",
        }
    }

    pub fn cp_kg_label(self) -> &'static str {
        match self {
            Self::Kcal => "kcal/(kg*K)",
            Self::KJ => "kJ/(kg*K)",
        }
    }

    pub fn h_from_j_mol(self, j_per_mol: f64) -> f64 {
        match self {
            Self::Kcal => j_per_mol / (1000.0 * CAL_TO_JOULE),
            Self::KJ => j_per_mol / 1000.0,
        }
    }

    pub fn cp_from_j_mol_k(self, j_per_mol_k: f64) -> f64 {
        match self {
            Self::Kcal => j_per_mol_k / CAL_TO_JOULE,
            Self::KJ => j_per_mol_k,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AquaPressUnit {
    #[default]
    Pa,
    KPa,
    Bar,
    Atm,
}

impl AquaPressUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pa => "Pa",
            Self::KPa => "kPa",
            Self::Bar => "bar",
            Self::Atm => "atm",
        }
    }

    pub fn from_pa(self, pa: f64) -> f64 {
        match self {
            Self::Pa => pa,
            Self::KPa => pa / 1000.0,
            Self::Bar => pa / 100_000.0,
            Self::Atm => pa / 101_325.0,
        }
    }
}

/// Editable row in user's `OwnDB` Pitzer Binary sheet (`Fig. 8`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnBinaryRow {
    pub cation: String,
    pub anion: String,
    pub show_t_params: bool,
    pub min_t_c: Option<f64>,
    pub max_t_c: Option<f64>,
    pub min_m: Option<f64>,
    pub max_m: Option<f64>,
    pub min_i: Option<f64>,
    pub max_i: Option<f64>,
    pub beta0: PitzerTempCurve,
    pub beta1: PitzerTempCurve,
    pub beta2: PitzerTempCurve,
    pub c_phi: PitzerTempCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnThetaRow {
    pub ion_i: String,
    pub ion_j: String,
    pub show_t_params: bool,
    pub min_t_c: Option<f64>,
    pub max_t_c: Option<f64>,
    pub min_m: Option<f64>,
    pub max_m: Option<f64>,
    pub min_i: Option<f64>,
    pub max_i: Option<f64>,
    pub theta: PitzerTempCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnPsiRow {
    pub ion1: String,
    pub ion2: String,
    pub ion3: String,
    pub show_t_params: bool,
    pub min_t_c: Option<f64>,
    pub max_t_c: Option<f64>,
    pub min_m: Option<f64>,
    pub max_m: Option<f64>,
    pub min_i: Option<f64>,
    pub max_i: Option<f64>,
    pub psi: PitzerTempCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnLambdaRow {
    pub neutral: String,
    pub ion: String,
    pub show_t_params: bool,
    pub min_t_c: Option<f64>,
    pub max_t_c: Option<f64>,
    pub min_m: Option<f64>,
    pub max_m: Option<f64>,
    pub min_i: Option<f64>,
    pub max_i: Option<f64>,
    pub lambda: PitzerTempCurve,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OwnPitzerDb {
    pub binaries: Vec<OwnBinaryRow>,
    pub thetas: Vec<OwnThetaRow>,
    pub psis: Vec<OwnPsiRow>,
    pub lambdas: Vec<OwnLambdaRow>,
}

impl OwnPitzerDb {
    /// Enforce HSC manual Sec. 36.6 rule: read rows sequentially until the first invalid row.
    pub fn valid_binaries_prefix(&self) -> Vec<BinaryPitzerEntry> {
        let mut out = Vec::new();
        for r in &self.binaries {
            let (Some(z1), Some(z2)) = (
                parse_ion_charge_tag(r.cation.trim()),
                parse_ion_charge_tag(r.anion.trim()),
            ) else {
                break;
            };
            if z1 <= 0 || z2 >= 0 {
                break;
            }
            out.push(BinaryPitzerEntry {
                cation: r.cation.trim().to_string(),
                anion: r.anion.trim().to_string(),
                beta0: mask_curve_if_const(r.beta0, r.show_t_params),
                beta1: mask_curve_if_const(r.beta1, r.show_t_params),
                beta2: mask_curve_if_const(r.beta2, r.show_t_params),
                c_phi: mask_curve_if_const(r.c_phi, r.show_t_params),
                range: ValidityRange {
                    min_t_c: r.min_t_c,
                    max_t_c: r.max_t_c,
                    min_m: r.min_m,
                    max_m: r.max_m,
                    min_i: r.min_i,
                    max_i: r.max_i,
                },
            });
        }
        out
    }

    pub fn valid_thetas_prefix(&self) -> Vec<ThetaPitzerEntry> {
        let mut out = Vec::new();
        for r in &self.thetas {
            let (Some(z1), Some(z2)) = (
                parse_ion_charge_tag(r.ion_i.trim()),
                parse_ion_charge_tag(r.ion_j.trim()),
            ) else {
                break;
            };
            if z1 == 0 || z2 == 0 || (z1 > 0) != (z2 > 0) {
                break;
            }
            out.push(ThetaPitzerEntry {
                ion_i: r.ion_i.trim().to_string(),
                ion_j: r.ion_j.trim().to_string(),
                theta: mask_curve_if_const(r.theta, r.show_t_params),
                range: ValidityRange {
                    min_t_c: r.min_t_c,
                    max_t_c: r.max_t_c,
                    min_m: r.min_m,
                    max_m: r.max_m,
                    min_i: r.min_i,
                    max_i: r.max_i,
                },
            });
        }
        out
    }

    pub fn valid_psis_prefix(&self) -> Vec<PsiPitzerEntry> {
        let mut out = Vec::new();
        for r in &self.psis {
            if r.ion1.trim().is_empty() || r.ion2.trim().is_empty() || r.ion3.trim().is_empty() {
                break;
            }
            if parse_ion_charge_tag(r.ion1.trim()).is_none()
                || parse_ion_charge_tag(r.ion2.trim()).is_none()
                || parse_ion_charge_tag(r.ion3.trim()).is_none()
            {
                break;
            }
            out.push(PsiPitzerEntry {
                ion1: r.ion1.trim().to_string(),
                ion2: r.ion2.trim().to_string(),
                ion3: r.ion3.trim().to_string(),
                psi: mask_curve_if_const(r.psi, r.show_t_params),
                range: ValidityRange {
                    min_t_c: r.min_t_c,
                    max_t_c: r.max_t_c,
                    min_m: r.min_m,
                    max_m: r.max_m,
                    min_i: r.min_i,
                    max_i: r.max_i,
                },
            });
        }
        out
    }

    pub fn valid_lambdas_prefix(&self) -> Vec<LambdaPitzerEntry> {
        let mut out = Vec::new();
        for r in &self.lambdas {
            if r.neutral.trim().is_empty() || r.ion.trim().is_empty() {
                break;
            }
            let z_n = parse_ion_charge_tag(r.neutral.trim());
            let z_i = parse_ion_charge_tag(r.ion.trim());
            if z_n != Some(0) || matches!(z_i, None | Some(0)) {
                break;
            }
            out.push(LambdaPitzerEntry {
                neutral: r.neutral.trim().to_string(),
                ion: r.ion.trim().to_string(),
                lambda: mask_curve_if_const(r.lambda, r.show_t_params),
                range: ValidityRange {
                    min_t_c: r.min_t_c,
                    max_t_c: r.max_t_c,
                    min_m: r.min_m,
                    max_m: r.max_m,
                    min_i: r.min_i,
                    max_i: r.max_i,
                },
            });
        }
        out
    }
}

fn mask_curve_if_const(mut c: PitzerTempCurve, show_t: bool) -> PitzerTempCurve {
    if !show_t {
        c.b = 0.0;
        c.c = 0.0;
        c.d = 0.0;
        c.e = 0.0;
        c.f = 0.0;
    }
    c
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitzerSourceColor {
    MainBlack,
    OwnBlue,
    BothGreen,
    ExcludedRed,
}

#[derive(Debug, Clone)]
pub struct MergedBinaryMeta {
    pub key: String,
    pub entry: BinaryPitzerEntry,
    pub color: PitzerSourceColor,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct MergedThetaMeta {
    pub key: String,
    pub entry: ThetaPitzerEntry,
    pub color: PitzerSourceColor,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct MergedPsiMeta {
    pub key: String,
    pub entry: PsiPitzerEntry,
    pub color: PitzerSourceColor,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct MergedLambdaMeta {
    pub key: String,
    pub entry: LambdaPitzerEntry,
    pub color: PitzerSourceColor,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct MergedPitzerCatalog {
    pub binaries: Vec<MergedBinaryMeta>,
    pub thetas: Vec<MergedThetaMeta>,
    pub psis: Vec<MergedPsiMeta>,
    pub lambdas: Vec<MergedLambdaMeta>,
}

pub fn norm_pair_key(a: &str, b: &str) -> String {
    let ka = a.trim().to_string();
    let kb = b.trim().to_string();
    if ka <= kb {
        format!("{ka}|{kb}")
    } else {
        format!("{kb}|{ka}")
    }
}

pub fn norm_triple_key(a: &str, b: &str, c: &str) -> String {
    let mut v = [a.trim(), b.trim(), c.trim()];
    v.sort_unstable();
    format!("{}|{}|{}", v[0], v[1], v[2])
}

/// Resolves `OwnDB > MainDB` merged catalog (`use_own_db_visible` controls whether OwnDB entries
/// are surfaced in the `Main Database` list; per HSC Sec. 36.6, saved `OwnDB` overrides in calc regardless).
pub fn resolve_pitzer_catalog(
    main_db: &PitzerDatabase,
    own_db: &OwnPitzerDb,
    excluded_keys: &BTreeSet<String>,
    place_own_top: bool,
) -> MergedPitzerCatalog {
    let own_bins = own_db.valid_binaries_prefix();
    let own_thetas = own_db.valid_thetas_prefix();
    let own_psis = own_db.valid_psis_prefix();
    let own_lams = own_db.valid_lambdas_prefix();

    let mut binaries = Vec::new();
    let mut seen_bin = BTreeSet::new();
    let main_bin_keys: BTreeSet<String> = main_db
        .binaries
        .iter()
        .map(|b| format!("B:{}|{}", b.cation, b.anion))
        .collect();
    for b in &own_bins {
        let key = format!("B:{}|{}", b.cation, b.anion);
        seen_bin.insert(key.clone());
        let active = !excluded_keys.contains(&key);
        let color = if !active {
            PitzerSourceColor::ExcludedRed
        } else if main_bin_keys.contains(&key) {
            PitzerSourceColor::BothGreen
        } else {
            PitzerSourceColor::OwnBlue
        };
        binaries.push(MergedBinaryMeta {
            key,
            entry: b.clone(),
            color,
            active,
        });
    }
    for b in &main_db.binaries {
        let key = format!("B:{}|{}", b.cation, b.anion);
        if seen_bin.contains(&key) {
            continue;
        }
        let active = !excluded_keys.contains(&key);
        let color = if active {
            PitzerSourceColor::MainBlack
        } else {
            PitzerSourceColor::ExcludedRed
        };
        binaries.push(MergedBinaryMeta {
            key,
            entry: b.clone(),
            color,
            active,
        });
    }
    if !place_own_top {
        binaries.sort_by(|x, y| {
            x.entry
                .cation
                .cmp(&y.entry.cation)
                .then(x.entry.anion.cmp(&y.entry.anion))
        });
    }

    let mut thetas = Vec::new();
    let mut seen_th = BTreeSet::new();
    let main_th_keys: BTreeSet<String> = main_db
        .thetas
        .iter()
        .map(|t| format!("T:{}", norm_pair_key(&t.ion_i, &t.ion_j)))
        .collect();
    for t in &own_thetas {
        let key = format!("T:{}", norm_pair_key(&t.ion_i, &t.ion_j));
        seen_th.insert(key.clone());
        let active = !excluded_keys.contains(&key);
        let color = if !active {
            PitzerSourceColor::ExcludedRed
        } else if main_th_keys.contains(&key) {
            PitzerSourceColor::BothGreen
        } else {
            PitzerSourceColor::OwnBlue
        };
        thetas.push(MergedThetaMeta {
            key,
            entry: t.clone(),
            color,
            active,
        });
    }
    for t in &main_db.thetas {
        let key = format!("T:{}", norm_pair_key(&t.ion_i, &t.ion_j));
        if seen_th.contains(&key) {
            continue;
        }
        let active = !excluded_keys.contains(&key);
        thetas.push(MergedThetaMeta {
            key,
            entry: t.clone(),
            color: if active {
                PitzerSourceColor::MainBlack
            } else {
                PitzerSourceColor::ExcludedRed
            },
            active,
        });
    }

    let mut psis = Vec::new();
    let mut seen_psi = BTreeSet::new();
    let main_psi_keys: BTreeSet<String> = main_db
        .psis
        .iter()
        .map(|p| format!("P:{}", norm_triple_key(&p.ion1, &p.ion2, &p.ion3)))
        .collect();
    for p in &own_psis {
        let key = format!("P:{}", norm_triple_key(&p.ion1, &p.ion2, &p.ion3));
        seen_psi.insert(key.clone());
        let active = !excluded_keys.contains(&key);
        let color = if !active {
            PitzerSourceColor::ExcludedRed
        } else if main_psi_keys.contains(&key) {
            PitzerSourceColor::BothGreen
        } else {
            PitzerSourceColor::OwnBlue
        };
        psis.push(MergedPsiMeta {
            key,
            entry: p.clone(),
            color,
            active,
        });
    }
    for p in &main_db.psis {
        let key = format!("P:{}", norm_triple_key(&p.ion1, &p.ion2, &p.ion3));
        if seen_psi.contains(&key) {
            continue;
        }
        let active = !excluded_keys.contains(&key);
        psis.push(MergedPsiMeta {
            key,
            entry: p.clone(),
            color: if active {
                PitzerSourceColor::MainBlack
            } else {
                PitzerSourceColor::ExcludedRed
            },
            active,
        });
    }

    let mut lambdas = Vec::new();
    let mut seen_lam = BTreeSet::new();
    let main_lam_keys: BTreeSet<String> = main_db
        .lambdas
        .iter()
        .map(|l| format!("L:{}|{}", l.neutral, l.ion))
        .collect();
    for l in &own_lams {
        let key = format!("L:{}|{}", l.neutral, l.ion);
        seen_lam.insert(key.clone());
        let active = !excluded_keys.contains(&key);
        let color = if !active {
            PitzerSourceColor::ExcludedRed
        } else if main_lam_keys.contains(&key) {
            PitzerSourceColor::BothGreen
        } else {
            PitzerSourceColor::OwnBlue
        };
        lambdas.push(MergedLambdaMeta {
            key,
            entry: l.clone(),
            color,
            active,
        });
    }
    for l in &main_db.lambdas {
        let key = format!("L:{}|{}", l.neutral, l.ion);
        if seen_lam.contains(&key) {
            continue;
        }
        let active = !excluded_keys.contains(&key);
        lambdas.push(MergedLambdaMeta {
            key,
            entry: l.clone(),
            color: if active {
                PitzerSourceColor::MainBlack
            } else {
                PitzerSourceColor::ExcludedRed
            },
            active,
        });
    }

    MergedPitzerCatalog {
        binaries,
        thetas,
        psis,
        lambdas,
    }
}

/// Parses charge `z` from an HSC Aqua formula string (e.g., `H(+a) -> Some(1)`, `SO4(-2a) -> Some(-2)`, `CO2(a) -> Some(0)`, `H2O -> Some(0)`).
pub fn parse_ion_charge_tag(formula: &str) -> Option<i32> {
    let s = formula.trim();
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("H2O") {
        return Some(0);
    }
    if let Some((_, charge)) = parse_hsc_formula_elements(s).ok() {
        return Some(charge);
    }
    // Lightweight fallback parser for suffixes like `(+a)`, `(-2a)`, `(2+a)`
    if let Some(open) = s.rfind('(') {
        if let Some(close) = s[open..].find(')') {
            let tag = &s[open + 1..open + close];
            if tag.ends_with('a') {
                let inner = &tag[..tag.len() - 1];
                if inner.is_empty() {
                    return Some(0);
                }
                if inner == "+" {
                    return Some(1);
                }
                if inner == "-" {
                    return Some(-1);
                }
                if let Some(rest) = inner.strip_prefix('+') {
                    return Some(rest.parse::<i32>().unwrap_or(1));
                }
                if let Some(rest) = inner.strip_prefix('-') {
                    return Some(-rest.parse::<i32>().unwrap_or(1));
                }
                if let Some(rest) = inner.strip_suffix('+') {
                    return Some(rest.parse::<i32>().unwrap_or(1));
                }
                if let Some(rest) = inner.strip_suffix('-') {
                    return Some(-rest.parse::<i32>().unwrap_or(1));
                }
            }
        }
    }
    Some(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AquaSpeciesInput {
    pub formula: String,
    pub temp: f64,
    pub amount_kmol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AquaCaseFile {
    pub species: Vec<AquaSpeciesInput>,
    pub method: AquaMethod,
    pub temp_unit: AquaTempUnit,
    pub energy_unit: AquaEnergyUnit,
    pub press_unit: AquaPressUnit,
    pub use_own_db: bool,
}

impl Default for AquaCaseFile {
    fn default() -> Self {
        Self::preset_na_s_h2o_fig3()
    }
}

impl AquaCaseFile {
    /// Preset from HSC 10 Aqua manual Fig. 3, Fig. 4 & Fig. 8 (`Na-S-H2O` system at 40 °C).
    pub fn preset_na_s_h2o_fig3() -> Self {
        let rows = [
            ("H2O", 40.0, 55.50844),
            ("H(+a)", 40.0, 4.05e-8),
            ("HS(-a)", 40.0, 5.28e-126),
            ("HS2(-a)", 40.0, 3.34e-221),
            ("HSO3(-a)", 40.0, 1.49e-42),
            ("HSO4(-a)", 40.0, 7.29e-7),
            ("HSO5(-a)", 40.0, 1.37e-28),
            ("HS2O3(-a)", 40.0, 3.76e-135),
            ("Na(+a)", 40.0, 1.92),
            ("NaSO4(-a)", 40.0, 1.63),
            ("OH(-a)", 40.0, 7.69e-7),
            ("S(-2a)", 40.0, 4.67e-131),
            ("SO3(-2a)", 40.0, 1.0e-35),
            ("SO4(-2a)", 40.0, 0.145),
            ("S2O3(-2a)", 40.0, 1.0e-90),
        ];
        Self {
            species: rows
                .into_iter()
                .map(|(f, t, a)| AquaSpeciesInput {
                    formula: f.to_string(),
                    temp: t,
                    amount_kmol: a,
                })
                .collect(),
            method: AquaMethod::Harvie,
            temp_unit: AquaTempUnit::Celsius,
            energy_unit: AquaEnergyUnit::Kcal,
            press_unit: AquaPressUnit::Pa,
            use_own_db: false,
        }
    }

    /// Millero seawater recipe (`S = 35`) from Table 1 & Fig. 10 (Clegg & Whitfield, p. 413).
    pub fn preset_millero_seawater() -> Self {
        let rows = [
            ("H2O", 25.0, 55.50844e-3),
            ("Br(-a)", 25.0, 8.700e-7),
            ("Ca(+2a)", 25.0, 1.064e-5),
            ("Cl(-a)", 25.0, 5.658e-4),
            ("F(-a)", 25.0, 6.000e-8),
            ("HCO3(-a)", 25.0, 2.410e-6),
            ("K(+a)", 25.0, 1.058e-5),
            ("Mg(+2a)", 25.0, 5.519e-5),
            ("Na(+a)", 25.0, 4.853e-4),
            ("SO4(-2a)", 25.0, 2.927e-5),
            ("Sr(+2a)", 25.0, 9.000e-8),
        ];
        Self {
            species: rows
                .into_iter()
                .map(|(f, t, a)| AquaSpeciesInput {
                    formula: f.to_string(),
                    temp: t,
                    amount_kmol: a,
                })
                .collect(),
            method: AquaMethod::Harvie,
            temp_unit: AquaTempUnit::Celsius,
            energy_unit: AquaEnergyUnit::Kcal,
            press_unit: AquaPressUnit::Pa,
            use_own_db: false,
        }
    }

    /// Simple NaCl–HCl–H2O electrolyte benchmark.
    pub fn preset_nacl_hcl() -> Self {
        Self {
            species: vec![
                AquaSpeciesInput { formula: "H2O".into(), temp: 25.0, amount_kmol: 55.50844 },
                AquaSpeciesInput { formula: "H(+a)".into(), temp: 25.0, amount_kmol: 0.10 },
                AquaSpeciesInput { formula: "Na(+a)".into(), temp: 25.0, amount_kmol: 1.00 },
                AquaSpeciesInput { formula: "Cl(-a)".into(), temp: 25.0, amount_kmol: 1.10 },
            ],
            method: AquaMethod::Harvie,
            temp_unit: AquaTempUnit::Celsius,
            energy_unit: AquaEnergyUnit::Kcal,
            press_unit: AquaPressUnit::Pa,
            use_own_db: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AquaSpeciesResult {
    pub formula: String,
    pub temp_disp: f64,
    pub amount_kmol: f64,
    pub mol_pct: f64,
    pub amount_g: f64,
    pub h_ideal_mol: f64,
    pub h_est_mol: f64,
    pub cp_ideal_mol: f64,
    pub cp_est_mol: f64,
    pub ac_molar: f64,
    pub ac_molal: f64,
    pub molality: f64,
    pub mw_g_mol: f64,
    pub charge: i32,
    pub elements: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct AquaPhaseSummary {
    pub temp_disp: f64,
    pub total_kmol: f64,
    pub total_mol_pct: f64,
    pub total_g: f64,
    pub h_ideal_mol: f64,
    pub h_est_mol: f64,
    pub cp_ideal_mol: f64,
    pub cp_est_mol: f64,
    pub h_ideal_kg: f64,
    pub h_est_kg: f64,
    pub cp_ideal_kg: f64,
    pub cp_est_kg: f64,
    pub osmotic_coeff: f64,
    pub water_activity: f64,
    pub h2o_vapour_press: f64,
    pub sol_vapour_press: f64,
    pub ionic_strength: f64,
    pub ph: Option<f64>,
    pub rel_humidity_pct: f64,
    pub fp_c: f64,
    pub bp_c: f64,
    pub fp_aw25_c: f64,
    pub bp_aw25_c: f64,
    pub osmotic_pressure: f64,
}

#[derive(Debug, Clone)]
pub struct AquaCalcResult {
    pub summary: AquaPhaseSummary,
    pub species: Vec<AquaSpeciesResult>,
    pub element_columns: Vec<String>,
    pub warnings: Vec<String>,
}

/// Appendix A Eq. (A1): Pure water vapor pressure `p0` in Pascals (`Pa`) from `T_C` (°C).
pub fn pure_water_vapor_pressure_pa(t_c: f64) -> f64 {
    let t = t_c.clamp(0.0, 374.0);
    let kpa = if t < 160.0 {
        let coeffs = [
            -3.104_329_543_993_91e-21,
            1.009_594_111_006_62e-18,
            -5.880_990_242_074_25e-17,
            -3.184_767_544_574_04e-14,
            6.556_124_569_127_02e-12,
            2.255_759_383_501_16e-9,
            2.999_988_014_821_07e-7,
            2.578_492_589_527_55e-5,
            1.437_763_581_890_44e-3,
            4.517_300_653_520_71e-2,
            0.599_030_194_123_347,
        ];
        coeffs.iter().fold(0.0_f64, |acc, &c| acc * t + c)
    } else {
        let coeffs = [
            1.119_828_606_307_43e-18,
            -4.049_205_195_969_97e-15,
            6.082_904_575_313_76e-12,
            -5.128_979_928_127_43e-9,
            2.724_083_116_927_4e-6,
            -9.596_960_500_042_19e-4,
            0.228_254_919_514_224,
            -36.311_462_374_836,
            3706.728_287_939_79,
            -219_658.499_462_442,
            5_746_593.578_740_1,
        ];
        coeffs.iter().fold(0.0_f64, |acc, &c| acc * t + c)
    };
    (kpa * 1000.0).max(0.0)
}

/// Eq. (27): McCutcheon et al. (1993) pure water density in `kg/m^3` (`T_C` in °C).
pub fn water_density_mccutcheon_kg_m3(t_c: f64) -> f64 {
    let t = t_c.clamp(-10.0, 200.0);
    1000.0 * (1.0 - ((t + 288.9414) * (t - 3.9863).powi(2)) / (508_929.2 * (t + 68.12963)))
}

/// Kell (1975) water density in `kg/m^3` (`Laliberté & Cooper 2004 Eq. 6`).
pub fn water_density_kell_kg_m3(t_c: f64) -> f64 {
    let t = t_c.clamp(-10.0, 200.0);
    let num = ((((-2.8054253e-10 * t + 1.0556302e-7) * t - 4.6170461e-5) * t - 0.0079870401)
        * t
        + 16.945176)
        * t
        + 999.83952;
    num / (1.0 + 0.01687985 * t)
}

/// Pure water static dielectric constant (Bradley & Pitzer 1979 correlation, `T` in K).
fn water_dielectric_constant(t_k: f64) -> f64 {
    let t = t_k.clamp(260.0, 620.0);
    let u1 = 3.4279e2 * (-5.0866e-3 * t + 9.4690e-7 * t * t).exp();
    u1.max(5.0)
}

/// Debye-Hückel osmotic slope `A_phi(T)` in `(kg/mol)^0.5` (`0.39148` at 298.15 K).
pub fn pitzer_a_phi(t_k: f64) -> f64 {
    let t = t_k.clamp(260.0, 600.0);
    let t_c = t - 273.15;
    let rho_g_cm3 = (water_density_kell_kg_m3(t_c) / 1000.0).max(0.3);
    let eps = water_dielectric_constant(t);
    // Exact SI/Pitzer relation calibrated at 298.15 K (rho=0.997045, eps=78.384 -> A_phi=0.39148)
    1.400_608e6 * rho_g_cm3.sqrt() * (eps * t).powf(-1.5)
}

fn pitzer_g(x: f64) -> f64 {
    if x.abs() < 1e-6 {
        1.0 - 0.25 * x + x * x / 15.0
    } else {
        2.0 * (1.0 - (1.0 + x) * (-x).exp()) / (x * x)
    }
}

/// Pitzer `g'(x)` as defined in Pitzer (1973/1991): `d g(x)/dx * x` = `-2 [1 - (1 + x + x^2/2) exp(-x)] / x^2`.
fn pitzer_g_prime(x: f64) -> f64 {
    if x.abs() < 1e-5 {
        -x / 3.0 + x * x / 4.0
    } else {
        -2.0 * (1.0 - (1.0 + x + 0.5 * x * x) * (-x).exp()) / (x * x)
    }
}

/// Pitzer (1975) higher-order electrostatic asymmetry integral J(x) and J'(x) for unsymmetrical mixing `z_i != z_j`.
fn pitzer_j_and_prime(x: f64) -> (f64, f64) {
    if x <= 1e-12 {
        return (0.0, 0.0);
    }
    if x >= 1.0 {
        let xp = 1.0 / (4.0 + 4.581 * x.powf(-0.7237) * (-0.012 * x.powf(0.528)).exp());
        let j = x * xp;
        // Numerical central derivative wrt x for J'(x) = dJ/dx cleanly bounded
        let dx = x * 1e-4;
        let xp2 = (x + dx)
            / (4.0 + 4.581 * (x + dx).powf(-0.7237) * (-0.012 * (x + dx).powf(0.528)).exp());
        let xp1 = (x - dx)
            / (4.0 + 4.581 * (x - dx).powf(-0.7237) * (-0.012 * (x - dx).powf(0.528)).exp());
        return (j, (xp2 - xp1) / (2.0 * dx));
    }
    let b0 = 4.118;
    let b1 = 7.247;
    let b2 = -4.408;
    let b3 = 1.837;
    let b4 = -0.251;
    let z = x.powf(0.2);
    let q = (b0 + z * (b1 + z * (b2 + z * (b3 + z * b4)))).max(1e-6);
    let dq_dz = b1 + z * (2.0 * b2 + z * (3.0 * b3 + z * 4.0 * b4));
    let denom = 4.0 + (4.0 / 3.0) * x.powf(-2.0 / 3.0) + x.powf(-1.0 / 3.0) / q;
    let j = x / denom;
    let d_denom_dx = (-8.0 / 9.0) * x.powf(-5.0 / 3.0)
        - (1.0 / 3.0) * x.powf(-4.0 / 3.0) / q
        - x.powf(-1.0 / 3.0) * (0.2 * x.powf(-0.8) * dq_dz) / (q * q);
    let jp = (denom - x * d_denom_dx) / (denom * denom);
    (j, jp)
}

fn higher_order_etheta(zi: i32, zj: i32, ionic_strength: f64, a_phi: f64) -> (f64, f64) {
    if zi == zj || (zi > 0) != (zj > 0) || ionic_strength <= 1e-12 {
        return (0.0, 0.0);
    }
    let sqrt_i = ionic_strength.sqrt();
    let fac = 6.0 * a_phi * sqrt_i;
    let x_ij = (zi as f64) * (zj as f64) * fac;
    let x_ii = (zi * zi) as f64 * fac;
    let x_jj = (zj * zj) as f64 * fac;
    let (j_ij, jp_ij) = pitzer_j_and_prime(x_ij);
    let (j_ii, jp_ii) = pitzer_j_and_prime(x_ii);
    let (j_jj, jp_jj) = pitzer_j_and_prime(x_jj);
    let e_theta = ((zi as f64) * (zj as f64) / (4.0 * ionic_strength)) * (j_ij - 0.5 * (j_ii + j_jj));
    let e_theta_p = -e_theta / ionic_strength
        + ((zi as f64) * (zj as f64) / (8.0 * ionic_strength * ionic_strength))
            * (x_ij * jp_ij - 0.5 * (x_ii * jp_ii + x_jj * jp_jj));
    (e_theta, e_theta_p)
}

#[derive(Clone)]
struct ResolvedIon {
    idx: usize,
    charge: i32,
    molality: f64,
}

/// Evaluates `(ln_gamma_molal_vec, phi_osmotic)` for all rows at temperature `t_k` using `method`
/// and resolved active catalog.
fn eval_activity_and_osmotic(
    charges: &[i32],
    molalities: &[f64],
    formulas: &[String],
    t_k: f64,
    method: AquaMethod,
    catalog: &MergedPitzerCatalog,
) -> (Vec<f64>, f64) {
    let n = formulas.len();
    let mut ln_g = vec![0.0; n];
    let a_phi = pitzer_a_phi(t_k);
    let b = 1.2_f64;

    let mut ionic_strength = 0.0;
    let mut sum_solute_m = 0.0;
    let mut z_big = 0.0;
    let mut cations = Vec::new();
    let mut anions = Vec::new();
    let mut neutrals = Vec::new();

    for i in 0..n {
        if formulas[i].trim().eq_ignore_ascii_case("H2O") {
            continue;
        }
        let m = molalities[i].max(0.0);
        let z = charges[i];
        sum_solute_m += m;
        if z > 0 {
            ionic_strength += 0.5 * m * (z * z) as f64;
            z_big += m * (z.abs() as f64);
            cations.push(ResolvedIon { idx: i, charge: z, molality: m });
        } else if z < 0 {
            ionic_strength += 0.5 * m * (z * z) as f64;
            z_big += m * (z.abs() as f64);
            anions.push(ResolvedIon { idx: i, charge: z, molality: m });
        } else {
            neutrals.push(ResolvedIon { idx: i, charge: 0, molality: m });
        }
    }

    if sum_solute_m <= 1e-20 || ionic_strength <= 1e-20 {
        return (ln_g, 1.0);
    }

    let sqrt_i = ionic_strength.sqrt();
    let i_eff = ionic_strength.max(1e-12);
    let f_dh = -a_phi * (sqrt_i / (1.0 + b * sqrt_i) + (2.0 / b) * (1.0 + b * sqrt_i).ln());

    if method == AquaMethod::Davies {
        for i in 0..n {
            if charges[i] != 0 {
                let z2 = (charges[i] * charges[i]) as f64;
                ln_g[i] = z2 * f_dh;
            }
        }
        let phi = 1.0 - (2.0 * a_phi * ionic_strength.powf(1.5) / (1.0 + b * sqrt_i)) / sum_solute_m.max(1e-20);
        return (ln_g, phi);
    }

    // Lookup helpers
    let find_bin = |cat: &str, an: &str| -> Option<&BinaryPitzerEntry> {
        catalog
            .binaries
            .iter()
            .find(|m| m.active && m.entry.cation == cat && m.entry.anion == an)
            .map(|m| &m.entry)
    };
    let find_th = |a: &str, bb: &str| -> Option<&ThetaPitzerEntry> {
        let k = norm_pair_key(a, bb);
        catalog
            .thetas
            .iter()
            .find(|m| m.active && norm_pair_key(&m.entry.ion_i, &m.entry.ion_j) == k)
            .map(|m| &m.entry)
    };
    let find_psi = |a: &str, bb: &str, c: &str| -> Option<&PsiPitzerEntry> {
        let k = norm_triple_key(a, bb, c);
        catalog
            .psis
            .iter()
            .find(|m| m.active && norm_triple_key(&m.entry.ion1, &m.entry.ion2, &m.entry.ion3) == k)
            .map(|m| &m.entry)
    };
    let find_lam = |neu: &str, ion: &str| -> Option<f64> {
        catalog
            .lambdas
            .iter()
            .find(|m| m.active && m.entry.neutral == neu && m.entry.ion == ion)
            .map(|m| m.entry.lambda.eval(t_k))
    };

    // Binary B_ca, B'_ca, B^phi_ca, C_ca
    let mut f_term = f_dh;
    let mut c_a_c_sum = 0.0;
    let mut phi_sum = -a_phi * ionic_strength.powf(1.5) / (1.0 + b * sqrt_i);

    let nc = cations.len();
    let na = anions.len();
    let mut b_ca = vec![vec![0.0; na]; nc];
    let mut bp_ca = vec![vec![0.0; na]; nc];
    let mut c_ca = vec![vec![0.0; na]; nc];

    for (ci, cat) in cations.iter().enumerate() {
        for (ai, an) in anions.iter().enumerate() {
            if let Some(ent) = find_bin(&formulas[cat.idx], &formulas[an.idx]) {
                let b0 = ent.beta0.eval(t_k);
                let b1 = ent.beta1.eval(t_k);
                let b2 = ent.beta2.eval(t_k);
                let cp = ent.c_phi.eval(t_k);
                let is_2_2 = cat.charge.abs() >= 2 && an.charge.abs() >= 2;
                let alpha1 = if is_2_2 { 1.4 } else { 2.0 };
                let alpha2 = 12.0;

                let x1 = alpha1 * sqrt_i;
                let x2 = alpha2 * sqrt_i;
                let b_val = b0 + b1 * pitzer_g(x1) + if ent.beta2.defined { b2 * pitzer_g(x2) } else { 0.0 };
                let bp_val = (b1 * pitzer_g_prime(x1) + if ent.beta2.defined { b2 * pitzer_g_prime(x2) } else { 0.0 }) / i_eff;
                let b_phi = b0 + b1 * (-x1).exp() + if ent.beta2.defined { b2 * (-x2).exp() } else { 0.0 };
                let z_prod = ((cat.charge * an.charge).abs() as f64).sqrt();
                let c_val = cp / (2.0 * z_prod.max(1.0));

                b_ca[ci][ai] = b_val;
                bp_ca[ci][ai] = bp_val;
                c_ca[ci][ai] = c_val;

                let mc_ma = cat.molality * an.molality;
                f_term += mc_ma * bp_val;
                c_a_c_sum += mc_ma * c_val;
                phi_sum += mc_ma * (b_phi + z_big * c_val);
            }
        }
    }

    // Harvie same-sign theta/Phi + Etheta/Etheta' terms
    let mut phi_cc = vec![vec![0.0; nc]; nc];
    let mut phi_aa = vec![vec![0.0; na]; na];
    if method == AquaMethod::Harvie {
        for ci in 0..nc {
            for cj in (ci + 1)..nc {
                let c1 = &cations[ci];
                let c2 = &cations[cj];
                let th = find_th(&formulas[c1.idx], &formulas[c2.idx])
                    .map(|e| e.theta.eval(t_k))
                    .unwrap_or(0.0);
                let (et, etp) = higher_order_etheta(c1.charge, c2.charge, i_eff, a_phi);
                let phi_ij = th + et;
                let phip_ij = etp;
                let phiphi_ij = phi_ij + i_eff * phip_ij;
                phi_cc[ci][cj] = phi_ij;
                phi_cc[cj][ci] = phi_ij;
                f_term += c1.molality * c2.molality * phip_ij;
                phi_sum += c1.molality * c2.molality * phiphi_ij;
            }
        }
        for ai in 0..na {
            for aj in (ai + 1)..na {
                let a1 = &anions[ai];
                let a2 = &anions[aj];
                let th = find_th(&formulas[a1.idx], &formulas[a2.idx])
                    .map(|e| e.theta.eval(t_k))
                    .unwrap_or(0.0);
                let (et, etp) = higher_order_etheta(a1.charge, a2.charge, i_eff, a_phi);
                let phi_ij = th + et;
                let phip_ij = etp;
                let phiphi_ij = phi_ij + i_eff * phip_ij;
                phi_aa[ai][aj] = phi_ij;
                phi_aa[aj][ai] = phi_ij;
                f_term += a1.molality * a2.molality * phip_ij;
                phi_sum += a1.molality * a2.molality * phiphi_ij;
            }
        }
        // Triple Psi terms in osmotic sum
        for ci in 0..nc {
            for cj in (ci + 1)..nc {
                for ai in 0..na {
                    if let Some(p) = find_psi(&formulas[cations[ci].idx], &formulas[cations[cj].idx], &formulas[anions[ai].idx]) {
                        phi_sum += cations[ci].molality * cations[cj].molality * anions[ai].molality * p.psi.eval(t_k);
                    }
                }
            }
        }
        for ai in 0..na {
            for aj in (ai + 1)..na {
                for ci in 0..nc {
                    if let Some(p) = find_psi(&formulas[anions[ai].idx], &formulas[anions[aj].idx], &formulas[cations[ci].idx]) {
                        phi_sum += anions[ai].molality * anions[aj].molality * cations[ci].molality * p.psi.eval(t_k);
                    }
                }
            }
        }
    }

    // Neutral-ion Lambda sums (included in Pitzer and Harvie per Eqs. 16-21)
    for neu in &neutrals {
        for ion in cations.iter().chain(anions.iter()) {
            if let Some(lam) = find_lam(&formulas[neu.idx], &formulas[ion.idx]) {
                phi_sum += neu.molality * ion.molality * lam;
            }
        }
    }

    // Cations ln gamma_{+,M}
    for (ci, cat) in cations.iter().enumerate() {
        let z = cat.charge as f64;
        let mut val = z * z * f_term;
        for (ai, an) in anions.iter().enumerate() {
            val += an.molality * (2.0 * b_ca[ci][ai] + z_big * c_ca[ci][ai]);
        }
        if method == AquaMethod::Harvie {
            for (cj, cat2) in cations.iter().enumerate() {
                if cj == ci {
                    continue;
                }
                let mut psi_sum = 0.0;
                for an in &anions {
                    if let Some(p) = find_psi(&formulas[cat.idx], &formulas[cat2.idx], &formulas[an.idx]) {
                        psi_sum += an.molality * p.psi.eval(t_k);
                    }
                }
                val += cat2.molality * (2.0 * phi_cc[ci][cj] + psi_sum);
            }
            for ai in 0..na {
                for aj in (ai + 1)..na {
                    if let Some(p) = find_psi(&formulas[cat.idx], &formulas[anions[ai].idx], &formulas[anions[aj].idx]) {
                        val += anions[ai].molality * anions[aj].molality * p.psi.eval(t_k);
                    }
                }
            }
        }
        val += z.abs() * c_a_c_sum;
        for neu in &neutrals {
            if let Some(lam) = find_lam(&formulas[neu.idx], &formulas[cat.idx]) {
                val += 2.0 * neu.molality * lam;
            }
        }
        ln_g[cat.idx] = val.clamp(-25.0, 25.0);
    }

    // Anions ln gamma_{-,X}
    for (ai, an) in anions.iter().enumerate() {
        let z = an.charge as f64;
        let mut val = z * z * f_term;
        for (ci, cat) in cations.iter().enumerate() {
            val += cat.molality * (2.0 * b_ca[ci][ai] + z_big * c_ca[ci][ai]);
        }
        if method == AquaMethod::Harvie {
            for (aj, an2) in anions.iter().enumerate() {
                if aj == ai {
                    continue;
                }
                let mut psi_sum = 0.0;
                for cat in &cations {
                    if let Some(p) = find_psi(&formulas[an.idx], &formulas[an2.idx], &formulas[cat.idx]) {
                        psi_sum += cat.molality * p.psi.eval(t_k);
                    }
                }
                val += an2.molality * (2.0 * phi_aa[ai][aj] + psi_sum);
            }
            for ci in 0..nc {
                for cj in (ci + 1)..nc {
                    if let Some(p) = find_psi(&formulas[cations[ci].idx], &formulas[cations[cj].idx], &formulas[an.idx]) {
                        val += cations[ci].molality * cations[cj].molality * p.psi.eval(t_k);
                    }
                }
            }
        }
        val += z.abs() * c_a_c_sum;
        for neu in &neutrals {
            if let Some(lam) = find_lam(&formulas[neu.idx], &formulas[an.idx]) {
                val += 2.0 * neu.molality * lam;
            }
        }
        ln_g[an.idx] = val.clamp(-25.0, 25.0);
    }

    // Neutrals ln gamma_n
    for neu in &neutrals {
        let mut val = 0.0;
        for ion in cations.iter().chain(anions.iter()) {
            if let Some(lam) = find_lam(&formulas[neu.idx], &formulas[ion.idx]) {
                val += 2.0 * ion.molality * lam;
            }
        }
        ln_g[neu.idx] = val.clamp(-25.0, 25.0);
    }

    let phi = 1.0 + (2.0 / sum_solute_m.max(1e-20)) * phi_sum;
    (ln_g, phi.clamp(0.05, 5.0))
}

/// Colligcalc Eqs. (34)-(35): Computes `(FP °C, BP °C)` from water activity at TF (273.15 K) and TB (373.15 K).
fn colligcalc_fp_bp(
    charges: &[i32],
    molalities: &[f64],
    formulas: &[String],
    sum_m: f64,
    method: AquaMethod,
    catalog: &MergedPitzerCatalog,
) -> (f64, f64) {
    // Pure water fusion & vaporization constants
    let t_f = 273.15_f64;
    let dh_fus = 6008.0_f64; // J/mol
    let dcp_fus = 38.0_f64; // J/(mol*K) (Cliq - Cice)
    let t_b = 373.15_f64;
    let dh_vap = 40_656.0_f64; // J/mol
    let dcp_vap = -41.9_f64; // J/(mol*K) (Cvap - Cliq)

    let (_, phi_f) = eval_activity_and_osmotic(charges, molalities, formulas, t_f, method, catalog);
    let ln_aw_f = -(MW_H2O_G_MOL / 1000.0) * sum_m * phi_f;
    let (_, phi_b) = eval_activity_and_osmotic(charges, molalities, formulas, t_b, method, catalog);
    let ln_aw_b = -(MW_H2O_G_MOL / 1000.0) * sum_m * phi_b;

    let theta_f = quadratic_colligative_delta(t_f, dh_fus, dcp_fus, ln_aw_f);
    let theta_b = quadratic_colligative_delta(t_b, dh_vap, dcp_vap, ln_aw_b);
    (-theta_f, 100.0 + theta_b)
}

/// Eqs. (34) & (35) common quadratic solver evaluated directly from `ln(a_liq)`.
fn quadratic_colligative_delta(t0: f64, dh0: f64, dcp: f64, ln_aw: f64) -> f64 {
    if ln_aw.abs() < 1e-12 {
        return 0.0;
    }
    let rad = (2.0 * dcp * t0 * t0 * R_JOULE * ln_aw + dh0 * dh0).max(0.0);
    let num = dh0 - 2.0 * R_JOULE * t0 * ln_aw - rad.sqrt();
    let den = 2.0 * (dh0 / t0 + 0.5 * dcp - R_JOULE * ln_aw);
    if den.abs() < 1e-12 {
        0.0
    } else {
        (num / den).max(0.0)
    }
}

/// Evaluates all thermodynamic, osmotic, colligative and electrochemical outputs for `AquaCaseFile`.
pub fn calculate_aqua(
    req: &AquaCaseFile,
    thermo_catalog: &dyn SpeciesCatalogPort,
    main_db: &PitzerDatabase,
    own_db: &OwnPitzerDb,
    excluded_keys: &BTreeSet<String>,
) -> AquaCalcResult {
    let mut warnings = Vec::new();
    let catalog = resolve_pitzer_catalog(main_db, own_db, excluded_keys, req.use_own_db);

    let h2o_idx = req
        .species
        .iter()
        .position(|s| s.formula.trim().eq_ignore_ascii_case("H2O"))
        .unwrap_or(0);
    let n_h2o_kmol = req
        .species
        .get(h2o_idx)
        .map(|s| s.amount_kmol.max(1e-12))
        .unwrap_or(55.50844);

    let t_c_sys = req
        .species
        .get(h2o_idx)
        .map(|s| req.temp_unit.to_c(s.temp))
        .unwrap_or(25.0);
    let t_k_sys = t_c_sys + 273.15;

    let n = req.species.len();
    let formulas: Vec<String> = req.species.iter().map(|s| s.formula.trim().to_string()).collect();
    let mut charges = Vec::with_capacity(n);
    let mut molalities = Vec::with_capacity(n);
    let mut mws = Vec::with_capacity(n);
    let mut elements_list = Vec::with_capacity(n);
    let mut element_set = BTreeSet::new();

    for (idx, sp) in req.species.iter().enumerate() {
        let f = sp.formula.trim();
        let mut elems = BTreeMap::new();
        let mut z = parse_ion_charge_tag(f).unwrap_or(0);
        let mw = if f.eq_ignore_ascii_case("H2O") {
            elems.insert("H".into(), 2.0);
            elems.insert("O".into(), 1.0);
            z = 0;
            MW_H2O_G_MOL
        } else if let Ok((parsed_mw, parsed_elems, parsed_z)) = compute_molecular_weight(f) {
            elems = parsed_elems;
            z = parsed_z;
            if z != 0 {
                elems.insert("e-".into(), (-z) as f64);
            }
            parsed_mw
        } else {
            if z != 0 {
                elems.insert("e-".into(), (-z) as f64);
            }
            0.0
        };
        for k in elems.keys() {
            if k != "e-" {
                element_set.insert(k.clone());
            }
        }
        let m = if idx == h2o_idx {
            MOL_H2O_PER_KG
        } else {
            MOL_H2O_PER_KG * (sp.amount_kmol.max(0.0) / n_h2o_kmol)
        };
        charges.push(z);
        molalities.push(m);
        mws.push(mw);
        elements_list.push(elems);
    }

    let mut element_columns: Vec<String> = element_set.into_iter().collect();
    if elements_list.iter().any(|m| m.contains_key("e-")) {
        element_columns.push("e-".to_string());
    }

    // Activity coefficients & first/second T-derivatives via centered difference (Gibbs-Helmholtz)
    let dt = 0.05_f64;
    let (ln_g_0, phi_0) =
        eval_activity_and_osmotic(&charges, &molalities, &formulas, t_k_sys, req.method, &catalog);
    let (ln_g_p, phi_p) = eval_activity_and_osmotic(
        &charges,
        &molalities,
        &formulas,
        t_k_sys + dt,
        req.method,
        &catalog,
    );
    let (ln_g_m, phi_m) = eval_activity_and_osmotic(
        &charges,
        &molalities,
        &formulas,
        (t_k_sys - dt).max(260.0),
        req.method,
        &catalog,
    );

    let total_kmol: f64 = req.species.iter().map(|s| s.amount_kmol.max(0.0)).sum();
    let x_h2o = (n_h2o_kmol / total_kmol.max(1e-20)).clamp(1e-9, 1.0);

    let sum_solute_m: f64 = molalities
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != h2o_idx)
        .map(|(_, &m)| m)
        .sum();
    let ionic_strength: f64 = molalities
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != h2o_idx && charges[*i] != 0)
        .map(|(i, &m)| 0.5 * m * (charges[i] * charges[i]) as f64)
        .sum();

    // Check charge balance & non-dissociation hint (manual Sec. 36.3 & 36.8)
    let net_charge: f64 = req
        .species
        .iter()
        .enumerate()
        .map(|(i, s)| s.amount_kmol * charges[i] as f64)
        .sum();
    if net_charge.abs() > 1e-4 {
        warnings.push(format!(
            "Charge imbalance detected: net charge = {net_charge:.4e} kmol e-; verify electroneutrality."
        ));
    }

    // Evaluate H2O excess enthalpy L_w and Cp_w via ln(a_w) = -(Mw/1000)*(sum m)*phi
    let d_phi_dt = (phi_p - phi_m) / (2.0 * dt);
    let d2_phi_dt2 = (phi_p - 2.0 * phi_0 + phi_m) / (dt * dt);
    let ln_aw_factor = -(MW_H2O_G_MOL / 1000.0) * sum_solute_m;
    let d_ln_aw_dt = ln_aw_factor * d_phi_dt;
    let d2_ln_aw_dt2 = ln_aw_factor * d2_phi_dt2;
    let h_ex_h2o_j_mol = -R_JOULE * t_k_sys * t_k_sys * d_ln_aw_dt;
    let cp_ex_h2o_j_mol_k =
        -R_JOULE * (2.0 * t_k_sys * d_ln_aw_dt + t_k_sys * t_k_sys * d2_ln_aw_dt2);

    let mut total_g = 0.0;
    let mut sum_h_id_j = 0.0;
    let mut sum_h_est_j = 0.0;
    let mut sum_cp_id_j_k = 0.0;
    let mut sum_cp_est_j_k = 0.0;
    let mut species_out = Vec::with_capacity(n);

    for i in 0..n {
        let f = &formulas[i];
        let sp_in = &req.species[i];
        let t_k_sp = req.temp_unit.to_c(sp_in.temp) + 273.15;

        // Lookup ideal H(T), Cp(T) from Maindb6/OwnDB species database
        let (h_id_j_mol, cp_id_j_mol_k) = thermo_catalog
            .get_species(f)
            .and_then(|s| evaluate_species_si(&s, t_k_sp, true, false).ok())
            .map(|(pt, _)| (pt.h, pt.cp))
            .unwrap_or_else(|| {
                if i == h2o_idx {
                    let dt298 = t_k_sp - 298.15;
                    (-285_830.0 + 75.35 * dt298, 75.35)
                } else {
                    (0.0, 0.0)
                }
            });

        let (h_ex_j_mol, cp_ex_j_mol_k, ac_molal, ac_molar) = if i == h2o_idx {
            let aw = (ln_aw_factor * phi_0).exp();
            // Manual displays water AC ~ 1 / osmotic_coeff or aw/x(H2O)
            let ac_w = (aw / x_h2o).clamp(0.1, 2.0);
            (h_ex_h2o_j_mol, cp_ex_h2o_j_mol_k, ac_w, ac_w)
        } else {
            let d_lng_dt = (ln_g_p[i] - ln_g_m[i]) / (2.0 * dt);
            let d2_lng_dt2 = (ln_g_p[i] - 2.0 * ln_g_0[i] + ln_g_m[i]) / (dt * dt);
            let h_ex = -R_JOULE * t_k_sys * t_k_sys * d_lng_dt;
            let cp_ex = -R_JOULE * (2.0 * t_k_sys * d_lng_dt + t_k_sys * t_k_sys * d2_lng_dt2);
            let gm = ln_g_0[i].exp();
            let gx = gm / x_h2o; // Eq. (37)
            (h_ex, cp_ex, gm, gx)
        };

        let h_est_j_mol = h_id_j_mol + h_ex_j_mol;
        let cp_est_j_mol_k = cp_id_j_mol_k + cp_ex_j_mol_k;
        let moles = sp_in.amount_kmol.max(0.0) * 1000.0;
        let amt_g = moles * mws[i];
        total_g += amt_g;
        sum_h_id_j += moles * h_id_j_mol;
        sum_h_est_j += moles * h_est_j_mol;
        sum_cp_id_j_k += moles * cp_id_j_mol_k;
        sum_cp_est_j_k += moles * cp_est_j_mol_k;

        species_out.push(AquaSpeciesResult {
            formula: f.clone(),
            temp_disp: sp_in.temp,
            amount_kmol: sp_in.amount_kmol,
            mol_pct: (sp_in.amount_kmol.max(0.0) / total_kmol.max(1e-20)) * 100.0,
            amount_g: amt_g,
            h_ideal_mol: req.energy_unit.h_from_j_mol(h_id_j_mol),
            h_est_mol: req.energy_unit.h_from_j_mol(h_est_j_mol),
            cp_ideal_mol: req.energy_unit.cp_from_j_mol_k(cp_id_j_mol_k),
            cp_est_mol: req.energy_unit.cp_from_j_mol_k(cp_est_j_mol_k),
            ac_molar,
            ac_molal,
            molality: molalities[i],
            mw_g_mol: mws[i],
            charge: charges[i],
            elements: elements_list[i].clone(),
        });
    }

    // Osmcalc Eqs. (23)-(27) & Colligcalc Eqs. (34)-(35)
    let water_activity = (ln_aw_factor * phi_0).exp().clamp(1e-9, 1.5);
    let p0_pa = pure_water_vapor_pressure_pa(t_c_sys);
    let p_sol_pa = water_activity * p0_pa;
    let rh_pct = (p_sol_pa / p0_pa.max(1e-9)) * 100.0;
    let rho_w = water_density_mccutcheon_kg_m3(t_c_sys);
    let osm_press_pa = -(R_JOULE * t_k_sys * rho_w / MW_H2O_KG_MOL) * water_activity.ln();

    let (fp_c, bp_c) =
        colligcalc_fp_bp(&charges, &molalities, &formulas, sum_solute_m, req.method, &catalog);
    let (_, phi_25) =
        eval_activity_and_osmotic(&charges, &molalities, &formulas, 298.15, req.method, &catalog);
    let ln_aw_25 = ln_aw_factor * phi_25;
    let fp_aw25_c = -quadratic_colligative_delta(273.15, 6008.0, 38.0, ln_aw_25);
    let bp_aw25_c = 100.0 + quadratic_colligative_delta(373.15, 40_656.0, -41.9, ln_aw_25);

    // pHcalc Eq. (38): only if H(+a) is present with positive molality
    let ph = formulas
        .iter()
        .position(|f| f == "H(+a)")
        .and_then(|idx| {
            let a_h = species_out[idx].ac_molal * species_out[idx].molality;
            if a_h > 0.0 {
                Some(-a_h.log10())
            } else {
                None
            }
        });

    let total_mol = (total_kmol * 1000.0).max(1e-12);
    let total_kg = (total_g / 1000.0).max(1e-12);
    let h_id_phase_mol = req.energy_unit.h_from_j_mol(sum_h_id_j / total_mol);
    let h_est_phase_mol = req.energy_unit.h_from_j_mol(sum_h_est_j / total_mol);
    let cp_id_phase_mol = req.energy_unit.cp_from_j_mol_k(sum_cp_id_j_k / total_mol);
    let cp_est_phase_mol = req.energy_unit.cp_from_j_mol_k(sum_cp_est_j_k / total_mol);

    // Phase kcal/kg and kcal/(kg*K) (note manual headers Fig. 3-4: kcal/kg and kcal/(kg*K))
    let h_id_kg = req.energy_unit.h_from_j_mol(sum_h_id_j / total_kg);
    let h_est_kg = req.energy_unit.h_from_j_mol(sum_h_est_j / total_kg);
    let cp_id_kg = req.energy_unit.h_from_j_mol(sum_cp_id_j_k / total_kg);
    let cp_est_kg = req.energy_unit.h_from_j_mol(sum_cp_est_j_k / total_kg);

    AquaCalcResult {
        summary: AquaPhaseSummary {
            temp_disp: req.temp_unit.from_c(t_c_sys),
            total_kmol,
            total_mol_pct: 100.0,
            total_g,
            h_ideal_mol: h_id_phase_mol,
            h_est_mol: h_est_phase_mol,
            cp_ideal_mol: cp_id_phase_mol,
            cp_est_mol: cp_est_phase_mol,
            h_ideal_kg: h_id_kg,
            h_est_kg: h_est_kg,
            cp_ideal_kg: cp_id_kg,
            cp_est_kg: cp_est_kg,
            osmotic_coeff: phi_0,
            water_activity,
            h2o_vapour_press: req.press_unit.from_pa(p0_pa),
            sol_vapour_press: req.press_unit.from_pa(p_sol_pa),
            ionic_strength,
            ph,
            rel_humidity_pct: rh_pct,
            fp_c: req.temp_unit.from_c(fp_c),
            bp_c: req.temp_unit.from_c(bp_c),
            fp_aw25_c: req.temp_unit.from_c(fp_aw25_c),
            bp_aw25_c: req.temp_unit.from_c(bp_aw25_c),
            osmotic_pressure: req.press_unit.from_pa(osm_press_pa),
        },
        species: species_out,
        element_columns,
        warnings,
    }
}

/// Active/Missing parameter report (`Parameter Sets` window, Fig. 6).
#[derive(Debug, Clone)]
pub struct ActiveParameterSetsReport {
    pub active_binaries: Vec<MergedBinaryMeta>,
    pub active_thetas: Vec<MergedThetaMeta>,
    pub active_psis: Vec<MergedPsiMeta>,
    pub active_lambdas: Vec<MergedLambdaMeta>,
    pub missing_binaries: Vec<(String, String)>,
    pub missing_thetas: Vec<(String, String)>,
    pub missing_psis: Vec<(String, String, String)>,
    pub missing_lambdas: Vec<(String, String)>,
}

pub fn analyze_parameter_sets(
    req: &AquaCaseFile,
    main_db: &PitzerDatabase,
    own_db: &OwnPitzerDb,
    excluded_keys: &BTreeSet<String>,
) -> ActiveParameterSetsReport {
    let catalog = resolve_pitzer_catalog(main_db, own_db, excluded_keys, req.use_own_db);
    let mut cats = Vec::new();
    let mut ans = Vec::new();
    let mut neus = Vec::new();
    for sp in &req.species {
        let f = sp.formula.trim().to_string();
        if f.eq_ignore_ascii_case("H2O") {
            continue;
        }
        match parse_ion_charge_tag(&f) {
            Some(z) if z > 0 => cats.push(f),
            Some(z) if z < 0 => ans.push(f),
            Some(0) => neus.push(f),
            _ => {}
        }
    }

    let mut active_binaries = Vec::new();
    let mut missing_binaries = Vec::new();
    for c in &cats {
        for a in &ans {
            if let Some(m) = catalog
                .binaries
                .iter()
                .find(|x| x.active && &x.entry.cation == c && &x.entry.anion == a)
            {
                active_binaries.push(m.clone());
            } else if req.method != AquaMethod::Davies {
                missing_binaries.push((c.clone(), a.clone()));
            }
        }
    }

    let mut active_thetas = Vec::new();
    let mut missing_thetas = Vec::new();
    if req.method == AquaMethod::Harvie {
        for i in 0..cats.len() {
            for j in (i + 1)..cats.len() {
                let k = norm_pair_key(&cats[i], &cats[j]);
                if let Some(m) = catalog
                    .thetas
                    .iter()
                    .find(|x| x.active && norm_pair_key(&x.entry.ion_i, &x.entry.ion_j) == k)
                {
                    active_thetas.push(m.clone());
                } else {
                    missing_thetas.push((cats[i].clone(), cats[j].clone()));
                }
            }
        }
        for i in 0..ans.len() {
            for j in (i + 1)..ans.len() {
                let k = norm_pair_key(&ans[i], &ans[j]);
                if let Some(m) = catalog
                    .thetas
                    .iter()
                    .find(|x| x.active && norm_pair_key(&x.entry.ion_i, &x.entry.ion_j) == k)
                {
                    active_thetas.push(m.clone());
                } else {
                    missing_thetas.push((ans[i].clone(), ans[j].clone()));
                }
            }
        }
    }

    let mut active_psis = Vec::new();
    let mut missing_psis = Vec::new();
    if req.method == AquaMethod::Harvie {
        for i in 0..cats.len() {
            for j in (i + 1)..cats.len() {
                for a in &ans {
                    let k = norm_triple_key(&cats[i], &cats[j], a);
                    if let Some(m) = catalog.psis.iter().find(|x| {
                        x.active && norm_triple_key(&x.entry.ion1, &x.entry.ion2, &x.entry.ion3) == k
                    }) {
                        active_psis.push(m.clone());
                    } else {
                        missing_psis.push((cats[i].clone(), cats[j].clone(), a.clone()));
                    }
                }
            }
        }
        for i in 0..ans.len() {
            for j in (i + 1)..ans.len() {
                for c in &cats {
                    let k = norm_triple_key(&ans[i], &ans[j], c);
                    if let Some(m) = catalog.psis.iter().find(|x| {
                        x.active && norm_triple_key(&x.entry.ion1, &x.entry.ion2, &x.entry.ion3) == k
                    }) {
                        active_psis.push(m.clone());
                    } else {
                        missing_psis.push((ans[i].clone(), ans[j].clone(), c.clone()));
                    }
                }
            }
        }
    }

    let mut active_lambdas = Vec::new();
    let mut missing_lambdas = Vec::new();
    if req.method != AquaMethod::Davies {
        for neu in &neus {
            for ion in cats.iter().chain(ans.iter()) {
                if let Some(m) = catalog
                    .lambdas
                    .iter()
                    .find(|x| x.active && &x.entry.neutral == neu && &x.entry.ion == ion)
                {
                    active_lambdas.push(m.clone());
                } else {
                    missing_lambdas.push((neu.clone(), ion.clone()));
                }
            }
        }
    }

    ActiveParameterSetsReport {
        active_binaries,
        active_thetas,
        active_psis,
        active_lambdas,
        missing_binaries,
        missing_thetas,
        missing_psis,
        missing_lambdas,
    }
}

/// Selected component input for `Density Calculator` (Fig. 7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityInputRow {
    pub component: String,
    pub mass_fraction: f64,
}

#[derive(Debug, Clone)]
pub struct DensityRowViolation {
    pub component: String,
    pub mass_fraction: f64,
    pub mass_violation: Option<String>,
    pub temp_violation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DensityCalcResult {
    pub h2o_density_kg_m3: f64,
    pub solution_density_kg_m3: f64,
    pub h2o_mass_fraction: f64,
    pub rows: Vec<DensityRowViolation>,
    pub global_error: Option<String>,
}

/// Laliberté & Cooper (2004) multi-component density calculation matching Fig. 7 and HSC6 `Aqueous Solution Density.xls`.
pub fn calculate_solution_density(rows: &[DensityInputRow], t_c: f64) -> DensityCalcResult {
    let rho_w = water_density_kell_kg_m3(t_c);
    let mut w_tot = 0.0;
    for r in rows {
        if r.mass_fraction < 0.0 || !r.mass_fraction.is_finite() {
            return DensityCalcResult {
                h2o_density_kg_m3: rho_w,
                solution_density_kg_m3: f64::NAN,
                h2o_mass_fraction: 0.0,
                rows: Vec::new(),
                global_error: Some("Mass fractions must be non-negative.".into()),
            };
        }
        w_tot += r.mass_fraction;
    }
    if w_tot > 1.0 + 1e-9 {
        return DensityCalcResult {
            h2o_density_kg_m3: rho_w,
            solution_density_kg_m3: f64::NAN,
            h2o_mass_fraction: 1.0 - w_tot,
            rows: Vec::new(),
            global_error: Some("Total solute mass fraction exceeds 1.0.".into()),
        };
    }
    let w_h2o = (1.0 - w_tot).max(0.0);
    let mut inv_rho = w_h2o / rho_w.max(1.0);
    let mut row_evals = Vec::with_capacity(rows.len());

    for r in rows {
        let comp = r.component.trim();
        let entry_opt = find_density_salt(comp);
        let mut mass_violation = None;
        let mut temp_violation = None;

        if let Some(ent) = entry_opt {
            if let Some(wmin) = ent.w_min {
                if r.mass_fraction > 0.0 && r.mass_fraction < wmin {
                    mass_violation = Some(format!("< Min ({wmin:.4})"));
                }
            }
            if let Some(wmax) = ent.w_max {
                if w_tot > wmax {
                    mass_violation = Some(format!("> Max ({wmax:.4})"));
                }
            }
            if let Some(tmin) = ent.t_min_c {
                if t_c < tmin {
                    temp_violation = Some(format!("< Min ({tmin:.1} °C)"));
                }
            }
            if let Some(tmax) = ent.t_max_c {
                if t_c > tmax {
                    temp_violation = Some(format!("> Max ({tmax:.1} °C)"));
                }
            }
            let num = (ent.c0 * w_tot + ent.c1) * (1e-6 * (t_c + ent.c4).powi(2)).exp();
            let den = w_tot + ent.c2 + ent.c3 * t_c;
            let rho_app = if den.abs() > 1e-12 {
                (num / den).max(100.0)
            } else {
                2600.0
            };
            inv_rho += r.mass_fraction / rho_app;
        } else {
            mass_violation = Some("Salt not in density DB".into());
            inv_rho += r.mass_fraction / 2600.0;
        }

        row_evals.push(DensityRowViolation {
            component: r.component.clone(),
            mass_fraction: r.mass_fraction,
            mass_violation,
            temp_violation,
        });
    }

    DensityCalcResult {
        h2o_density_kg_m3: rho_w,
        solution_density_kg_m3: 1.0 / inv_rho.max(1e-9),
        h2o_mass_fraction: w_h2o,
        rows: row_evals,
        global_error: None,
    }
}

pub fn find_density_salt(component: &str) -> Option<&'static DensitySaltEntry> {
    let key = component.trim();
    DENSITY_SALTS
        .iter()
        .find(|e| e.component.eq_ignore_ascii_case(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aqua_pitzer_data::builtin_main_pitzer_db;
    use crate::{ChemicalSpecies, KelleyCoefficients, Phase, SpeciesThermoInterval};
    use approx::assert_relative_eq;
    use std::collections::HashMap;

    struct MockRepo(HashMap<String, ChemicalSpecies>);
    impl crate::SpeciesRepository for MockRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get(formula).cloned()
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            self.0.get(element_symbol).cloned()
        }
    }
    impl SpeciesCatalogPort for MockRepo {
        fn search_species(&self, _query: &str, _limit: usize) -> Vec<ChemicalSpecies> {
            self.0.values().cloned().collect()
        }
        fn search_species_by_elements(
            &self,
            _elements: &[String],
            _include_gas: bool,
            _include_liquid: bool,
            _include_solid: bool,
            _include_aqua_ions: bool,
            _include_aqua_neutrals: bool,
            _limit: usize,
        ) -> Vec<ChemicalSpecies> {
            self.0.values().cloned().collect()
        }
        fn insert_or_update_species(&mut self, sp: ChemicalSpecies) {
            self.0.insert(sp.formula.clone(), sp);
        }
        fn species_count(&self) -> usize {
            self.0.len()
        }
    }

    fn make_aq(formula: &str, h_kcal: f64, cp_cal: f64) -> ChemicalSpecies {
        let (_, _, z) = compute_molecular_weight(formula).unwrap_or((18.0, BTreeMap::new(), 0));
        ChemicalSpecies {
            formula: formula.into(),
            name: formula.into(),
            phase: if z == 0 {
                Phase::AqueousNeutral
            } else {
                Phase::AqueousIon
            },
            molecular_weight: 18.015,
            density_g_cm3: 1.0,
            charge: z,
            intervals: vec![SpeciesThermoInterval {
                t_min: 250.0,
                t_max: 400.0,
                phase: Phase::AqueousNeutral,
                kelley: KelleyCoefficients {
                    a: cp_cal * CAL_TO_JOULE,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: h_kcal * 1000.0 * CAL_TO_JOULE,
                s_or_delta_s_tr: 10.0 * CAL_TO_JOULE,
                reference: "Test".into(),
            }],
        }
    }

    #[test]
    fn test_p0_appendix_a_and_fig4() {
        assert_relative_eq!(pure_water_vapor_pressure_pa(40.0), 7377.08, epsilon = 0.02);
        assert_relative_eq!(pure_water_vapor_pressure_pa(100.0), 101_318.8, epsilon = 5.0);
        assert_relative_eq!(water_density_mccutcheon_kg_m3(40.0), 992.25, epsilon = 0.05);
    }

    #[test]
    fn test_laliberte_cooper_density_fig7() {
        let rows = vec![
            DensityInputRow {
                component: "ZnSO4".into(),
                mass_fraction: 0.149,
            },
            DensityInputRow {
                component: "H2SO4".into(),
                mass_fraction: 0.048,
            },
        ];
        let res = calculate_solution_density(&rows, 25.0);
        assert!(res.global_error.is_none());
        assert_relative_eq!(res.solution_density_kg_m3, 1204.37, epsilon = 0.25);
        assert!(res.rows.iter().all(|r| r.mass_violation.is_none()));
    }

    #[test]
    fn test_pure_water_limit_and_na_s_h2o_case() {
        let repo = MockRepo(HashMap::from([
            ("H2O".into(), make_aq("H2O", -68.315, 18.01)),
            ("H(+a)".into(), make_aq("H(+a)", 0.0, 0.0)),
            ("Na(+a)".into(), make_aq("Na(+a)", -57.433, 10.77)),
            ("NaSO4(-a)".into(), make_aq("NaSO4(-a)", -273.31, 21.21)),
            ("SO4(-2a)".into(), make_aq("SO4(-2a)", -217.40, -63.3)),
            ("OH(-a)".into(), make_aq("OH(-a)", -55.42, -27.29)),
        ]));
        let main_db = builtin_main_pitzer_db();
        let own_db = OwnPitzerDb::default();
        let excluded = BTreeSet::new();

        // A1: ultra-dilute limit
        let mut pure = AquaCaseFile::preset_nacl_hcl();
        pure.species[1].amount_kmol = 1e-10;
        pure.species[2].amount_kmol = 1e-10;
        pure.species[3].amount_kmol = 2e-10;
        let res_pure = calculate_aqua(&pure, &repo, &main_db, &own_db, &excluded);
        assert_relative_eq!(res_pure.summary.osmotic_coeff, 1.0, epsilon = 1e-4);
        assert_relative_eq!(res_pure.species[1].ac_molal, 1.0, epsilon = 1e-3);

        // A2: Na-S-H2O at 40 C (Figs. 3-4)
        let req = AquaCaseFile::preset_na_s_h2o_fig3();
        let res = calculate_aqua(&req, &repo, &main_db, &own_db, &excluded);
        assert_relative_eq!(res.summary.ionic_strength, 2.065, epsilon = 0.02);
        assert_relative_eq!(res.summary.h2o_vapour_press, 7377.08, epsilon = 0.05);
        assert!(res.summary.ph.unwrap_or(0.0) > 7.4 && res.summary.ph.unwrap_or(0.0) < 8.1);
        assert!(res.summary.fp_c < 0.0 && res.summary.bp_c > 100.0);
        // Eq. (37): gamma_x = gamma_m / x(H2O)
        let na = res.species.iter().find(|s| s.formula == "Na(+a)").unwrap();
        let x_w = res.species[0].mol_pct / 100.0;
        assert_relative_eq!(na.ac_molar, na.ac_molal / x_w, epsilon = 1e-9);
    }

    #[test]
    fn test_seawater_millero_and_methods_switch() {
        let repo = MockRepo(HashMap::new());
        let main_db = builtin_main_pitzer_db();
        let own_db = OwnPitzerDb::default();
        let excluded = BTreeSet::new();

        let mut req = AquaCaseFile::preset_millero_seawater();
        req.method = AquaMethod::Harvie;
        let r_harvie = calculate_aqua(&req, &repo, &main_db, &own_db, &excluded);
        let na = r_harvie.species.iter().find(|s| s.formula == "Na(+a)").unwrap();
        let cl = r_harvie.species.iter().find(|s| s.formula == "Cl(-a)").unwrap();
        let mg = r_harvie.species.iter().find(|s| s.formula == "Mg(+2a)").unwrap();
        let so4 = r_harvie.species.iter().find(|s| s.formula == "SO4(-2a)").unwrap();
        assert!((na.ac_molal - 0.65).abs() < 0.05, "na={}", na.ac_molal);
        assert!((cl.ac_molal - 0.69).abs() < 0.05, "cl={}", cl.ac_molal);
        assert!((mg.ac_molal - 0.247).abs() < 0.08, "mg={}", mg.ac_molal);
        assert!((so4.ac_molal - 0.125).abs() < 0.06, "so4={}", so4.ac_molal);

        req.method = AquaMethod::Pitzer;
        let r_pitzer = calculate_aqua(&req, &repo, &main_db, &own_db, &excluded);
        req.method = AquaMethod::Davies;
        let r_davies = calculate_aqua(&req, &repo, &main_db, &own_db, &excluded);
        assert!(
            (r_harvie.species[2].ac_molal - r_davies.species[2].ac_molal).abs() > 1e-3
                && (r_pitzer.species[2].ac_molal - r_davies.species[2].ac_molal).abs() > 1e-3
        );
    }

    #[test]
    fn test_owndb_precedence_and_stop_on_invalid_row() {
        let main_db = builtin_main_pitzer_db();
        let own_db = OwnPitzerDb {
            binaries: vec![
                OwnBinaryRow {
                    cation: "Ag(+a)".into(),
                    anion: "Cl(-a)".into(),
                    show_t_params: true,
                    min_t_c: Some(25.0),
                    max_t_c: Some(55.0),
                    min_m: Some(0.0),
                    max_m: Some(2.0),
                    min_i: None,
                    max_i: None,
                    beta0: PitzerTempCurve::full(2.0, 0.0, 5.0, 0.0, 0.0, 0.0),
                    beta1: PitzerTempCurve::EMPTY,
                    beta2: PitzerTempCurve::EMPTY,
                    c_phi: PitzerTempCurve::EMPTY,
                },
                OwnBinaryRow {
                    cation: "INVALID_ROW".into(),
                    anion: "".into(),
                    show_t_params: false,
                    min_t_c: None,
                    max_t_c: None,
                    min_m: None,
                    max_m: None,
                    min_i: None,
                    max_i: None,
                    beta0: PitzerTempCurve::constant(1.0),
                    beta1: PitzerTempCurve::EMPTY,
                    beta2: PitzerTempCurve::EMPTY,
                    c_phi: PitzerTempCurve::EMPTY,
                },
                OwnBinaryRow {
                    cation: "Al(+3a)".into(),
                    anion: "Cl(-a)".into(),
                    show_t_params: false,
                    min_t_c: Some(25.0),
                    max_t_c: Some(55.0),
                    min_m: None,
                    max_m: None,
                    min_i: Some(0.0),
                    max_i: Some(5.0),
                    beta0: PitzerTempCurve::constant(3.0),
                    beta1: PitzerTempCurve::EMPTY,
                    beta2: PitzerTempCurve::EMPTY,
                    c_phi: PitzerTempCurve::EMPTY,
                },
            ],
            ..Default::default()
        };
        let mut excluded = BTreeSet::new();
        excluded.insert("B:Al(+3a)|NO3(-a)".into());
        let cat = resolve_pitzer_catalog(&main_db, &own_db, &excluded, true);
        // Should stop at INVALID_ROW -> only Ag(+a)|Cl(-a) loaded from OwnDB (Blue)
        assert_eq!(cat.binaries[0].key, "B:Ag(+a)|Cl(-a)");
        assert_eq!(cat.binaries[0].color, PitzerSourceColor::OwnBlue);
        // Al(+3a)|NO3(-a) is Red
        let alno3 = cat.binaries.iter().find(|x| x.key == "B:Al(+3a)|NO3(-a)").unwrap();
        assert_eq!(alno3.color, PitzerSourceColor::ExcludedRed);
    }
}
