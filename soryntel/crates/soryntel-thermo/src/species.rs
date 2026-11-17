//! Module 16: Species & Thermochemical Databases (`HSC - Databases`) domain logic.
//!
//! Covers atomic weight & formula composition parsing (`AWDB4` / `MainDB6` conventions),
//! stoichiometry / `Fix Ratio` / `Organic` matching, liquid `X -> X(l)` 298.15 K extrapolation,
//! aqueous ion scales & Criss–Cobble analysis, Kelley OLS/QR `Fit Cp Data` with `N` masks,
//! external $C_p(T)$ expression evaluator & converter, $G(T) \to (H, S, A, B, C, D)$ analytical
//! conversion + `Delta to Normal`, and HSC color palette (`0..=53`).

use crate::{
    evaluate_species_si, species_converter, ChemicalSpecies, KelleyCoefficients, Phase,
    SpeciesMetadata, SpeciesRepository, SpeciesThermoInterval,
};
use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_CAL, R_JOULE, SoryntelError, T_REF_K};
use std::collections::BTreeMap;

/// Color palette entry (`0..=53`) from HSC Chemistry 6 Ch. 28.18.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HscColorSpec {
    pub code: i16,
    pub name: &'static str,
    pub rgb: [u8; 3],
}

/// Complete 54-color HSC 6 palette (`0..=53`).
pub const HSC_COLOR_PALETTE: [HscColorSpec; 54] = [
    HscColorSpec { code: 0, name: "No data available", rgb: [226, 232, 240] },
    HscColorSpec { code: 1, name: "Blue", rgb: [37, 99, 235] },
    HscColorSpec { code: 2, name: "Green", rgb: [22, 163, 74] },
    HscColorSpec { code: 3, name: "Cyan", rgb: [6, 182, 212] },
    HscColorSpec { code: 4, name: "Red", rgb: [220, 38, 38] },
    HscColorSpec { code: 5, name: "Magenta (blue-red)", rgb: [192, 38, 211] },
    HscColorSpec { code: 6, name: "Brown", rgb: [146, 64, 14] },
    HscColorSpec { code: 7, name: "White", rgb: [248, 250, 252] },
    HscColorSpec { code: 8, name: "Dark Grey", rgb: [71, 85, 105] },
    HscColorSpec { code: 9, name: "Light Blue", rgb: [96, 165, 250] },
    HscColorSpec { code: 10, name: "Light Green", rgb: [74, 222, 128] },
    HscColorSpec { code: 11, name: "Light Cyan", rgb: [103, 232, 249] },
    HscColorSpec { code: 12, name: "Light Red", rgb: [248, 113, 113] },
    HscColorSpec { code: 13, name: "Light Magenta", rgb: [232, 121, 249] },
    HscColorSpec { code: 14, name: "Light Yellow", rgb: [254, 240, 138] },
    HscColorSpec { code: 15, name: "Bright White", rgb: [255, 255, 255] },
    HscColorSpec { code: 16, name: "Colorless", rgb: [241, 245, 249] },
    HscColorSpec { code: 17, name: "Yellow", rgb: [234, 179, 8] },
    HscColorSpec { code: 18, name: "Grey", rgb: [148, 163, 184] },
    HscColorSpec { code: 19, name: "Black", rgb: [24, 24, 27] },
    HscColorSpec { code: 20, name: "Orange", rgb: [249, 115, 22] },
    HscColorSpec { code: 21, name: "Yellow-grey", rgb: [168, 162, 120] },
    HscColorSpec { code: 22, name: "Yellow-brown", rgb: [180, 130, 40] },
    HscColorSpec { code: 23, name: "Red-brown", rgb: [153, 52, 42] },
    HscColorSpec { code: 24, name: "Orange-yellow", rgb: [245, 158, 11] },
    HscColorSpec { code: 25, name: "Yellow-green", rgb: [132, 204, 22] },
    HscColorSpec { code: 26, name: "Black-brown", rgb: [68, 44, 29] },
    HscColorSpec { code: 27, name: "White brown", rgb: [224, 205, 182] },
    HscColorSpec { code: 28, name: "Orange-green", rgb: [180, 145, 45] },
    HscColorSpec { code: 29, name: "Blue-green", rgb: [13, 148, 136] },
    HscColorSpec { code: 30, name: "Dark-green", rgb: [20, 83, 45] },
    HscColorSpec { code: 31, name: "Blue-black", rgb: [30, 41, 59] },
    HscColorSpec { code: 32, name: "Red-orange", rgb: [234, 88, 12] },
    HscColorSpec { code: 33, name: "Yellow-orange", rgb: [251, 146, 60] },
    HscColorSpec { code: 34, name: "Green-yellow", rgb: [190, 242, 100] },
    HscColorSpec { code: 35, name: "Brown-black", rgb: [54, 34, 22] },
    HscColorSpec { code: 36, name: "Green-blue", rgb: [2, 132, 199] },
    HscColorSpec { code: 37, name: "Orange-red", rgb: [225, 55, 29] },
    HscColorSpec { code: 38, name: "Brown-yellow", rgb: [161, 118, 25] },
    HscColorSpec { code: 39, name: "Dark-red", rgb: [127, 29, 29] },
    HscColorSpec { code: 40, name: "Brown-violet", rgb: [107, 45, 86] },
    HscColorSpec { code: 41, name: "Red-violet", rgb: [162, 28, 108] },
    HscColorSpec { code: 42, name: "Violet-black", rgb: [49, 27, 71] },
    HscColorSpec { code: 43, name: "Yellow-black", rgb: [66, 60, 20] },
    HscColorSpec { code: 44, name: "Blue-white", rgb: [219, 234, 254] },
    HscColorSpec { code: 45, name: "Red-black", rgb: [69, 20, 25] },
    HscColorSpec { code: 46, name: "Green-black", rgb: [20, 45, 30] },
    HscColorSpec { code: 47, name: "Purple-black", rgb: [59, 23, 68] },
    HscColorSpec { code: 48, name: "Grey-brown", rgb: [120, 113, 108] },
    HscColorSpec { code: 49, name: "Brown-red", rgb: [148, 43, 43] },
    HscColorSpec { code: 50, name: "Gold-brown", rgb: [176, 128, 24] },
    HscColorSpec { code: 51, name: "Gold-yellow", rgb: [250, 204, 21] },
    HscColorSpec { code: 52, name: "Blue-grey", rgb: [100, 116, 139] },
    HscColorSpec { code: 53, name: "Orange-brown", rgb: [186, 92, 22] },
];

/// Return metadata/color specification for HSC color index `0..=53` (clamped safely).
pub fn color_spec(code: i16) -> &'static HscColorSpec {
    let idx = if (0..54).contains(&code) {
        code as usize
    } else {
        0
    };
    &HSC_COLOR_PALETTE[idx]
}

/// Originating database layer (`OwnDB` always takes calculation precedence over `MainDB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbSource {
    Main,
    Own,
    OpenDefault,
}

impl DbSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "MainDB",
            Self::Own => "OwnDB",
            Self::OpenDefault => "Built-in",
        }
    }
}

/// Element inclusion rule for `Search Mode` (`Some` = contains any; `All` = contains all).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementSearchMode {
    Some,
    All,
}

/// Target column for `Find Key Word`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeywordSearchField {
    Any,
    Formula,
    StructuralFormula,
    ChemicalName,
    CommonName,
    CasNumber,
}

/// Complete phase + organic filter matching HSC6 Ch. 28.5 (`Search Mode` + `List Names`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseFilterFlags {
    pub solid: bool,
    pub liquid: bool,
    pub gas: bool,
    pub aqua_ions: bool,
    pub aqua_neutrals: bool,
    /// When `false`, excludes `ai` (`(ia)`) neutral electrolytes.
    pub include_ai: bool,
    /// When `false`, excludes `ao` (`(a)`) undissociated aqueous species.
    pub include_ao: bool,
    /// When `false`, skips organic molecules (`H > 0` and `C > 2` per Ch. 28.5).
    pub organic: bool,
    /// Optional explicit carbon atom counts (`4, 6, 7` etc.; empty = unrestricted).
    pub carbon_counts: Vec<u32>,
}

impl Default for PhaseFilterFlags {
    fn default() -> Self {
        Self {
            solid: true,
            liquid: true,
            gas: true,
            aqua_ions: true,
            aqua_neutrals: true,
            include_ai: true,
            include_ao: true,
            organic: true,
            carbon_counts: Vec::new(),
        }
    }
}

impl PhaseFilterFlags {
    pub fn matches_phase(&self, phase: Phase, is_ai_electrolyte: bool) -> bool {
        match phase {
            Phase::Solid => self.solid,
            Phase::Liquid => self.liquid,
            Phase::Gas => self.gas,
            Phase::AqueousIon => self.aqua_ions,
            Phase::AqueousNeutral => {
                if !self.aqua_neutrals {
                    return false;
                }
                if is_ai_electrolyte {
                    self.include_ai
                } else {
                    self.include_ao
                }
            }
        }
    }

    pub fn matches_composition(&self, elements: &BTreeMap<String, f64>) -> bool {
        let c = elements.get("C").copied().unwrap_or(0.0);
        let h = elements.get("H").copied().unwrap_or(0.0);
        if !self.organic && h > 1e-9 && c > 2.0 + 1e-9 {
            return false;
        }
        if !self.carbon_counts.is_empty() {
            let matches_c = self
                .carbon_counts
                .iter()
                .any(|&target| (c - target as f64).abs() < 1e-6);
            if !matches_c {
                return false;
            }
        }
        true
    }
}

/// Complete database dataset block (used when a formula has multiple literature datasets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDataset {
    pub source: DbSource,
    pub species: ChemicalSpecies,
    pub metadata: SpeciesMetadata,
    pub dataset_index: usize,
    pub raw_phase_code: String,
}

/// Summary entry for browsing, listing, and resolving species metadata cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesCatalogEntry {
    pub formula: String,
    pub source: DbSource,
    pub species: ChemicalSpecies,
    pub metadata: SpeciesMetadata,
    pub dataset_count: usize,
    pub raw_phase_code: String,
}

/// Duplicate conflict emitted during `Merge HSC1 -> HSC2` when `Show Duplicates` is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMergeConflict {
    pub formula: String,
    pub source_dataset: SpeciesDataset,
    pub target_dataset: SpeciesDataset,
}

/// Merge resolution decision for duplicate species.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeConflictDecision {
    TakeSource,
    TakeTarget,
}

/// Atomic weight lookup unifying IUPAC/HSC `AWDB4.HSC` elements including `D` and `e-`.
pub fn awdb_atomic_weight(symbol: &str) -> Option<f64> {
    match symbol {
        "D" => Some(2.0159),
        "e-" => Some(0.0005487),
        _ => species_converter::atomic_weight(symbol),
    }
}

/// True if HSC's Ch. 28.5 organic rule triggers (`H > 0` and `C > 2`).
pub fn is_organic_hsc(elements: &BTreeMap<String, f64>) -> bool {
    let c = elements.get("C").copied().unwrap_or(0.0);
    let h = elements.get("H").copied().unwrap_or(0.0);
    h > 1e-9 && c > 2.0 + 1e-9
}

/// Parse carbon count limits typed as `"4, 6, 7"` or `"4 6 7"` (max 9 counts per Ch. 28.5).
pub fn parse_carbon_limits(raw: &str) -> Vec<u32> {
    raw.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok())
        .take(9)
        .collect()
}

/// Parse an HSC database formula into elemental counts and ionic charge.
///
/// Handles:
/// - Leading/internal `*` adducts (`*2FeO*SiO2`, `CuSO4*5H2O`, `Na2O*Al2O3*6SiO2`)
/// - Aqueous/gas/liquid/solid/polymorph/pressure suffixes (`(g)`, `(l)`, `(s)`, `(a)`, `(ia)`, `(+2a)`, `(-2a)`, `(A)`, `(B)`, `(C)`, `(100bar)`)
/// - Square brackets (`[Fe(CN)6](-4a)`)
/// - Free electron `e-` / `e(-)`
pub fn parse_hsc_formula_elements(formula: &str) -> Result<(BTreeMap<String, f64>, i32), SoryntelError> {
    let trimmed = formula.trim();
    if trimmed.is_empty() {
        return Err(SoryntelError::ParseError("Formula cannot be empty".into()));
    }
    if trimmed == "e-" || trimmed == "e(-)" {
        return Ok((BTreeMap::new(), -1));
    }
    if trimmed.starts_with('*') {
        // Fast path avoiding `soryntel_solver::parse_term` treating leading '*' awkwardly if any
        return parse_core_formula_and_suffix(trimmed);
    }
    parse_core_formula_and_suffix(trimmed)
}

fn parse_core_formula_and_suffix(trimmed: &str) -> Result<(BTreeMap<String, f64>, i32), SoryntelError> {
    let (core, charge) = strip_hsc_trailing_suffix(trimmed);
    let normalized = core.replace('[', "(").replace(']', ")").replace('·', "*");
    let elements = parse_adduct_parts(&normalized)?;
    if elements.is_empty() {
        return Err(SoryntelError::ParseError(format!(
            "No chemical elements found in formula '{trimmed}'"
        )));
    }
    Ok((elements, charge))
}

fn strip_hsc_trailing_suffix(formula: &str) -> (&str, i32) {
    if !formula.ends_with(')') {
        return (formula, 0);
    }
    let Some(open_idx) = formula.rfind('(') else {
        return (formula, 0);
    };
    let suf = &formula[open_idx + 1..formula.len() - 1];
    if let Some(z) = parse_hsc_suffix_tag(suf) {
        return (&formula[..open_idx], z);
    }
    (formula, 0)
}

fn parse_hsc_suffix_tag(suf: &str) -> Option<i32> {
    match suf {
        "s" | "l" | "L" | "g" | "G" | "a" | "ia" | "ao" | "ai" | "A" | "B" | "C" | "D"
        | "E" | "F" | "I" | "II" | "III" | "IV" => return Some(0),
        _ => {}
    }
    // High-pressure / thermal labels in Maindb6 like Ar(100bar), C2H6(90bar), etc.
    if suf.ends_with("bar") || suf.ends_with("atm") {
        return Some(0);
    }
    let trimmed = suf.trim_end_matches(|c| c == 'a' || c == 'g' || c == 'l');
    if trimmed == "+" {
        return Some(1);
    }
    if trimmed == "-" {
        return Some(-1);
    }
    if (trimmed.starts_with('+') || trimmed.starts_with('-')) && trimmed.len() >= 2 {
        if let Ok(z) = trimmed.parse::<i32>() {
            return Some(z);
        }
    }
    None
}

fn parse_adduct_parts(core: &str) -> Result<BTreeMap<String, f64>, SoryntelError> {
    let mut total = BTreeMap::new();
    for part in core.split('*') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut num_end = 0;
        let mut seen_dot = false;
        for (i, c) in part.char_indices() {
            if c.is_ascii_digit() || (c == '.' && !seen_dot && i > 0) {
                if c == '.' {
                    seen_dot = true;
                }
                num_end = i + 1;
            } else {
                break;
            }
        }
        let mult: f64 = if num_end > 0 {
            part[..num_end].parse().unwrap_or(1.0)
        } else {
            1.0
        };
        let sub = parse_nested_group(&part[num_end..])?;
        for (el, count) in sub {
            *total.entry(el).or_insert(0.0) += count * mult;
        }
    }
    Ok(total)
}

fn parse_nested_group(s: &str) -> Result<BTreeMap<String, f64>, SoryntelError> {
    let bytes = s.as_bytes();
    let mut stack: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new()];
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'(' {
            stack.push(BTreeMap::new());
            i += 1;
        } else if b == b')' {
            if stack.len() <= 1 {
                return Err(SoryntelError::ParseError(format!("Unmatched ')' in '{s}'")));
            }
            let group = stack.pop().unwrap();
            i += 1;
            let (mult, next_i) = read_stoich_number(s, i);
            i = next_i;
            let top = stack.last_mut().unwrap();
            for (el, cnt) in group {
                *top.entry(el).or_insert(0.0) += cnt * mult;
            }
        } else if b.is_ascii_uppercase() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            let sym = &s[start..i];
            let (mult, next_i) = read_stoich_number(s, i);
            i = next_i;
            let top = stack.last_mut().unwrap();
            *top.entry(sym.to_string()).or_insert(0.0) += mult;
        } else if b == b'+' || b == b'-' || b == b' ' {
            i += 1;
        } else {
            return Err(SoryntelError::ParseError(format!(
                "Unexpected character '{}' in formula '{s}'",
                b as char
            )));
        }
    }

    if stack.len() != 1 {
        return Err(SoryntelError::ParseError(format!("Unclosed '(' in '{s}'")));
    }
    Ok(stack.pop().unwrap())
}

fn read_stoich_number(s: &str, start: usize) -> (f64, usize) {
    let bytes = s.as_bytes();
    let mut i = start;
    let mut seen_dot = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            i += 1;
        } else if bytes[i] == b'.' && !seen_dot && i > start {
            seen_dot = true;
            i += 1;
        } else {
            break;
        }
    }
    if i == start {
        (1.0, start)
    } else {
        (s[start..i].parse::<f64>().unwrap_or(1.0), i)
    }
}

/// Calculate formula molecular weight (g/mol), element composition map, and formal charge.
pub fn compute_molecular_weight(
    formula: &str,
) -> Result<(f64, BTreeMap<String, f64>, i32), SoryntelError> {
    if formula.trim() == "e-" || formula.trim() == "e(-)" {
        return Ok((awdb_atomic_weight("e-").unwrap_or(0.0005487), BTreeMap::new(), -1));
    }
    let (elements, charge) = parse_hsc_formula_elements(formula)?;
    let mut mw = 0.0;
    for (el, &count) in &elements {
        let aw = awdb_atomic_weight(el).ok_or_else(|| {
            SoryntelError::ParseError(format!("Unknown chemical element '{el}' in '{formula}'"))
        })?;
        mw += count * aw;
    }
    // Ionic mass adjustment for electron mass
    mw -= (charge as f64) * awdb_atomic_weight("e-").unwrap_or(0.0005487);
    Ok((mw.max(0.0), elements, charge))
}

/// Compare candidate elemental stoichiometry against `target`.
///
/// - `fix_ratio = false`: Exact element counts match regardless of order (`SiO4FeFe` == `*2FeO*SiO2`).
/// - `fix_ratio = true`: Same element set with positive constant scaling factor (`AsS` matches `As2S2`, `As3S3`).
pub fn stoichiometry_matches(
    candidate: &BTreeMap<String, f64>,
    target: &BTreeMap<String, f64>,
    fix_ratio: bool,
) -> bool {
    if target.is_empty() || candidate.len() != target.len() {
        return false;
    }
    let mut ratio: Option<f64> = None;
    for (el, &t_cnt) in target {
        let Some(&c_cnt) = candidate.get(el) else {
            return false;
        };
        if t_cnt <= 1e-12 || c_cnt <= 1e-12 {
            return false;
        }
        if !fix_ratio {
            if (c_cnt - t_cnt).abs() > 1e-5 {
                return false;
            }
        } else {
            let r = c_cnt / t_cnt;
            match ratio {
                None => ratio = Some(r),
                Some(r0) => {
                    if (r - r0).abs() > 1e-4 * r0.max(1.0) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Canonical descending-elements sort key (`Al2O3`, `C`, `Cu2S`, etc.) matching HSC6 `Sort`.
pub fn formula_sort_key(formula: &str) -> (String, String) {
    let stripped = formula.trim().trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    (stripped.to_string(), formula.to_string())
}

/// Extrapolate separate liquid species `X(l)` back to `298.15 K` from a contiguous solid-liquid record.
///
/// Returns `(H_l_298_J_per_mol, S_l_298_J_per_mol_K)` per `MATHEMATICAL_SPECIFICATION.md §3`.
pub fn extrapolate_liquid_298(
    solid_intervals: &[SpeciesThermoInterval],
    t_melt_k: f64,
    delta_h_fus_j: f64,
    liquid_kelley: KelleyCoefficients,
) -> Result<(f64, f64), SoryntelError> {
    if t_melt_k <= 0.0 || solid_intervals.is_empty() {
        return Err(SoryntelError::BalanceError(
            "Valid solid intervals and positive melting temperature required for liquid extrapolation".into(),
        ));
    }
    let h_ref = solid_intervals[0].h_or_delta_h_tr;
    let s_ref = solid_intervals[0].s_or_delta_s_tr;

    let mut h_solid_tm = h_ref;
    let mut s_solid_tm = s_ref;
    for (idx, iv) in solid_intervals.iter().enumerate() {
        if idx > 0 {
            h_solid_tm += iv.h_or_delta_h_tr;
            s_solid_tm += iv.s_or_delta_s_tr;
        }
        let t_lo = iv.t_min.max(T_REF_K);
        let t_hi = iv.t_max.min(t_melt_k);
        if t_hi > t_lo {
            h_solid_tm += iv.kelley.integrate_h(t_lo, t_hi);
            s_solid_tm += iv.kelley.integrate_s(t_lo, t_hi);
        }
    }

    let delta_s_fus = delta_h_fus_j / t_melt_k;
    let h_liq_tm = h_solid_tm + delta_h_fus_j;
    let s_liq_tm = s_solid_tm + delta_s_fus;

    let h_liq_298 = h_liq_tm - liquid_kelley.integrate_h(T_REF_K, t_melt_k);
    let s_liq_298 = s_liq_tm - liquid_kelley.integrate_s(T_REF_K, t_melt_k);
    Ok((h_liq_298, s_liq_298))
}

/// Aqueous entropy scale conversion & Criss–Cobble applicability diagnostics (Ch. 28.3–28.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqueousScaleSummary {
    pub formula: String,
    pub charge: i32,
    /// Conventional molal H+ = 0 entropy at 298.15 K (`J/(mol·K)`).
    pub s_conv_298_j: f64,
    /// Third-law absolute entropy scale `S_abs = S_conv - 5.0 * 4.184 * z` (`J/(mol·K)`).
    pub s_abs_298_j: f64,
    /// Dilution mixing entropy contribution `R * ln(1 / 55.51) ≈ -33.392 J/(mol·K)` (`-7.981 cal/(mol·K)`).
    pub delta_s_mix_j: f64,
    /// Pure-ion mole-fraction scale entropy `S_conv - (-delta_s_mix)` (`J/(mol·K)`).
    pub s_pure_ion_298_j: f64,
    /// True iff `Phase == AqueousIon` and `B = C = D = 0` on its sole interval.
    pub criss_cobble_active: bool,
    /// Criss–Cobble `Cp(573.15 K)` in `J/(mol·K)` when active.
    pub cp_at_300c_j: Option<f64>,
}

/// Analyze aqueous ion scales and conditionally evaluate Criss–Cobble at `300 °C` (`573.15 K`).
pub fn analyze_aqueous_scales(species: &ChemicalSpecies) -> Option<AqueousScaleSummary> {
    let iv = species.intervals.first()?;
    let z = species.charge;
    let s_conv = iv.s_or_delta_s_tr;
    let s_abs = s_conv - 5.0 * CAL_TO_JOULE * (z as f64);
    let delta_s_mix_j = R_CAL * (1.0_f64 / 55.51).ln() * CAL_TO_JOULE;
    let s_pure_ion = s_conv + delta_s_mix_j;
    let criss_cobble_active = species.phase == Phase::AqueousIon
        && species.intervals.len() == 1
        && !iv.kelley.has_temp_terms();
    let cp_at_300c_j = if criss_cobble_active {
        evaluate_species_si(species, 573.15, true, false)
            .ok()
            .map(|(pt, _)| pt.cp)
    } else {
        None
    };

    Some(AqueousScaleSummary {
        formula: species.formula.clone(),
        charge: z,
        s_conv_298_j: s_conv,
        s_abs_298_j: s_abs,
        delta_s_mix_j,
        s_pure_ion_298_j: s_pure_ion,
        criss_cobble_active,
        cp_at_300c_j,
    })
}

/// Experimental heat-capacity node `(T [K], Cp [J/(mol·K)])`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpDataPoint {
    pub t_k: f64,
    pub cp_j: f64,
}

/// Parameter lock mask for `Fit Cp Data` (`true` = free/optimize `Y`, `false` = locked `N`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpFitMask {
    pub fit_a: bool,
    pub fit_b: bool,
    pub fit_c: bool,
    pub fit_d: bool,
}

impl Default for CpFitMask {
    fn default() -> Self {
        Self {
            fit_a: true,
            fit_b: true,
            fit_c: true,
            fit_d: true,
        }
    }
}

/// Result of fitting Maier–Kelley coefficients via Householder QR least squares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpFitResult {
    /// Fitted Kelley coefficients in SI `J/(mol·K)`.
    pub kelley_si: KelleyCoefficients,
    pub fitted_cp_j: Vec<f64>,
    pub rel_error_pct: Vec<f64>,
    pub rmse_j: f64,
    pub max_abs_error_pct: f64,
}

/// Fit Kelley polynomial $C_p(T) = A + B\cdot 10^{-3}T + C\cdot 10^5 T^{-2} + D\cdot 10^{-6}T^2$
/// to experimental points with arbitrary `N` parameter locking (`locked_si`).
pub fn fit_cp_kelley(
    points: &[CpDataPoint],
    mask: CpFitMask,
    locked_si: KelleyCoefficients,
) -> Result<CpFitResult, SoryntelError> {
    if points.is_empty() {
        return Err(SoryntelError::BalanceError("No Cp data points provided".into()));
    }
    for p in points {
        if p.t_k <= 0.0 || !p.t_k.is_finite() || !p.cp_j.is_finite() {
            return Err(SoryntelError::InvalidTemperature(p.t_k));
        }
    }

    let flags = [mask.fit_a, mask.fit_b, mask.fit_c, mask.fit_d];
    let free_indices: Vec<usize> = flags
        .iter()
        .enumerate()
        .filter_map(|(idx, &f)| if f { Some(idx) } else { None })
        .collect();

    if points.len() < free_indices.len() {
        return Err(SoryntelError::BalanceError(format!(
            "Need at least {} data points to fit {} unlocked Kelley parameters",
            free_indices.len(),
            free_indices.len()
        )));
    }

    let locked_vals = [locked_si.a, locked_si.b, locked_si.c, locked_si.d];
    let mut final_vals = [0.0_f64; 4];
    for (i, &is_free) in flags.iter().enumerate() {
        if !is_free {
            final_vals[i] = locked_vals[i];
        }
    }

    if !free_indices.is_empty() {
        let m = points.len();
        let p = free_indices.len();
        let mut design = vec![vec![0.0_f64; p]; m];
        let mut rhs = vec![0.0_f64; m];

        for (r, pt) in points.iter().enumerate() {
            let basis = kelley_basis(pt.t_k);
            let mut y = pt.cp_j;
            for (i, &is_free) in flags.iter().enumerate() {
                if !is_free {
                    y -= final_vals[i] * basis[i];
                }
            }
            rhs[r] = y;
            for (c, &b_idx) in free_indices.iter().enumerate() {
                design[r][c] = basis[b_idx];
            }
        }

        let solved = solve_least_squares_qr(&mut design, &mut rhs)?;
        for (c, &b_idx) in free_indices.iter().enumerate() {
            final_vals[b_idx] = solved[c];
        }
    }

    let kelley_si = KelleyCoefficients {
        a: final_vals[0],
        b: final_vals[1],
        c: final_vals[2],
        d: final_vals[3],
    };

    let mut fitted_cp_j = Vec::with_capacity(points.len());
    let mut rel_error_pct = Vec::with_capacity(points.len());
    let mut sse = 0.0_f64;
    let mut max_abs_error_pct = 0.0_f64;

    for pt in points {
        let fit_val = kelley_si.cp(pt.t_k);
        let diff = fit_val - pt.cp_j;
        sse += diff * diff;
        let err_pct = if pt.cp_j.abs() > 1e-12 {
            (diff / pt.cp_j) * 100.0
        } else {
            0.0
        };
        max_abs_error_pct = max_abs_error_pct.max(err_pct.abs());
        fitted_cp_j.push(fit_val);
        rel_error_pct.push(err_pct);
    }

    Ok(CpFitResult {
        kelley_si,
        fitted_cp_j,
        rel_error_pct,
        rmse_j: (sse / points.len() as f64).sqrt(),
        max_abs_error_pct,
    })
}

#[inline]
fn kelley_basis(t_k: f64) -> [f64; 4] {
    [1.0, 1e-3 * t_k, 1e5 / (t_k * t_k), 1e-6 * t_k * t_k]
}

/// Householder QR linear least squares solver for small dense $m \times n$ ($n \le 4$) systems.
fn solve_least_squares_qr(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<Vec<f64>, SoryntelError> {
    let m = a.len();
    let n = a[0].len();
    for k in 0..n {
        let mut norm_sq = 0.0_f64;
        for row in k..m {
            norm_sq += a[row][k] * a[row][k];
        }
        let norm = norm_sq.sqrt();
        if norm < 1e-15 || !norm.is_finite() {
            return Err(SoryntelError::BalanceError(
                "Rank-deficient Cp regression matrix: lock more parameters or widen temperature span"
                    .into(),
            ));
        }
        let alpha = if a[k][k] >= 0.0 { -norm } else { norm };
        let mut v = vec![0.0_f64; m - k];
        v[0] = a[k][k] - alpha;
        for i in 1..(m - k) {
            v[i] = a[k + i][k];
        }
        let v_dot: f64 = v.iter().map(|x| x * x).sum();
        if v_dot < 1e-30 {
            continue;
        }
        // Apply reflector to remaining columns k..n and rhs
        for col in k..n {
            let dot: f64 = (0..(m - k)).map(|i| v[i] * a[k + i][col]).sum();
            let scale = 2.0 * dot / v_dot;
            for i in 0..(m - k) {
                a[k + i][col] -= scale * v[i];
            }
        }
        let dot_b: f64 = (0..(m - k)).map(|i| v[i] * b[k + i]).sum();
        let scale_b = 2.0 * dot_b / v_dot;
        for i in 0..(m - k) {
            b[k + i] -= scale_b * v[i];
        }
    }

    let mut x = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for col in (row + 1)..n {
            sum -= a[row][col] * x[col];
        }
        let diag = a[row][row];
        if diag.abs() < 1e-15 {
            return Err(SoryntelError::BalanceError(
                "Singular upper-triangular R in Cp QR solver".into(),
            ));
        }
        x[row] = sum / diag;
    }
    Ok(x)
}

/// Built-in literature $C_p(T)$ expression templates for `Convert Cp Functions` (`28.13`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalCpTemplate {
    MaierKelleyStandard,
    ShomateNist,
    Nasa7,
    Dippr100,
    Custom,
}

impl ExternalCpTemplate {
    pub fn formula_string(&self) -> &'static str {
        match self {
            Self::MaierKelleyStandard => "A + B*T + C/T^2 + D*T^2",
            Self::ShomateNist => "A + B*(T/1000) + C*(T/1000)^2 + D*(T/1000)^3 + E/(T/1000)^2",
            Self::Nasa7 => "R*(A + B*T + C*T^2 + D*T^3 + E*T^4)",
            Self::Dippr100 => "A + B*T + C*T^2 + D*T^3 + E*T^4",
            Self::Custom => "A + B*T",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::MaierKelleyStandard => "Maier-Kelley (A + B*T + C/T^2 + D*T^2)",
            Self::ShomateNist => "NIST Shomate (A..E, T/1000)",
            Self::Nasa7 => "NASA 7-term Polynomial (R*(A..E))",
            Self::Dippr100 => "DIPPR 100 Polynomial (A..E)",
            Self::Custom => "Custom Expression",
        }
    }
}

/// Resolve an external $C_p$ unit string (e.g. `"J/(kmol*K)"`, `"cal/(mol*K)"`, `"BTU/(lbmol*F)"`)
/// to its multiplier into SI `J/(mol·K)`.
pub fn cp_unit_to_si_factor(unit: &str) -> Result<f64, SoryntelError> {
    let norm = unit.trim().to_ascii_lowercase().replace(' ', "");
    match norm.as_str() {
        "j/(mol*k)" | "j/(mol·k)" | "j/mol*k" | "j/mol/k" => Ok(1.0),
        "j/(kmol*k)" | "j/(kmol·k)" | "j/kmol*k" | "j/kmol/k" => Ok(1e-3),
        "kj/(mol*k)" | "kj/(mol·k)" | "kj/mol*k" | "kj/mol/k" => Ok(1000.0),
        "kj/(kmol*k)" | "kj/(kmol·k)" | "kj/kmol*k" | "kj/kmol/k" => Ok(1.0),
        "cal/(mol*k)" | "cal/(mol·k)" | "cal/mol*k" | "cal/mol/k" => Ok(CAL_TO_JOULE),
        "cal/(kmol*k)" | "cal/kmol*k" => Ok(CAL_TO_JOULE * 1e-3),
        "kcal/(mol*k)" | "kcal/(mol·k)" | "kcal/mol*k" | "kcal/mol/k" => Ok(CAL_TO_JOULE * 1000.0),
        "btu/(lbmol*f)" | "btu/(lbmol*r)" | "btu/lbmol*f" => Ok(CAL_TO_JOULE),
        _ => Err(SoryntelError::ParseError(format!(
            "Unsupported Cp unit '{unit}' (supported: J/(mol*K), J/(kmol*K), kJ/(mol*K), cal/(mol*K), kcal/(mol*K), Btu/(lbmol*F))"
        ))),
    }
}

/// Evaluate a user or template $C_p(T)$ mathematical expression with variables `T`, `A..F`, `R`.
pub fn eval_cp_expression(expr: &str, t_k: f64, params: &[f64; 6]) -> Result<f64, SoryntelError> {
    let tokens = tokenize_expr(expr)?;
    let mut parser = ExprParser {
        tokens: &tokens,
        pos: 0,
        t_k,
        params,
    };
    let val = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(SoryntelError::ParseError(format!(
            "Unexpected trailing token in Cp expression '{expr}'"
        )));
    }
    if !val.is_finite() {
        return Err(SoryntelError::BalanceError(format!(
            "Non-finite Cp expression evaluation at T = {t_k} K"
        )));
    }
    Ok(val)
}

#[derive(Debug, Clone, PartialEq)]
enum ExprTok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize_expr(expr: &str) -> Result<Vec<ExprTok>, SoryntelError> {
    let bytes = expr.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match b {
            b'+' => { toks.push(ExprTok::Plus); i += 1; }
            b'-' => { toks.push(ExprTok::Minus); i += 1; }
            b'*' => { toks.push(ExprTok::Star); i += 1; }
            b'/' => { toks.push(ExprTok::Slash); i += 1; }
            b'^' => { toks.push(ExprTok::Caret); i += 1; }
            b'(' => { toks.push(ExprTok::LParen); i += 1; }
            b')' => { toks.push(ExprTok::RParen); i += 1; }
            _ if b.is_ascii_digit() || b == b'.' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                        i += 1;
                    }
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let val: f64 = expr[start..i].parse().map_err(|_| {
                    SoryntelError::ParseError(format!("Invalid number '{}'", &expr[start..i]))
                })?;
                toks.push(ExprTok::Num(val));
            }
            _ if b.is_ascii_alphabetic() || b == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                toks.push(ExprTok::Ident(expr[start..i].to_ascii_uppercase()));
            }
            _ => {
                return Err(SoryntelError::ParseError(format!(
                    "Invalid character '{}' in Cp formula",
                    b as char
                )));
            }
        }
    }
    Ok(toks)
}

struct ExprParser<'a> {
    tokens: &'a [ExprTok],
    pos: usize,
    t_k: f64,
    params: &'a [f64; 6],
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&ExprTok> {
        self.tokens.get(self.pos)
    }

    fn parse_expr(&mut self) -> Result<f64, SoryntelError> {
        let mut lhs = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprTok::Plus => { self.pos += 1; lhs += self.parse_term()?; }
                ExprTok::Minus => { self.pos += 1; lhs -= self.parse_term()?; }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<f64, SoryntelError> {
        let mut lhs = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprTok::Star => { self.pos += 1; lhs *= self.parse_power()?; }
                ExprTok::Slash => {
                    self.pos += 1;
                    let d = self.parse_power()?;
                    lhs /= d;
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_power(&mut self) -> Result<f64, SoryntelError> {
        let base = self.parse_unary()?;
        if let Some(ExprTok::Caret) = self.peek() {
            self.pos += 1;
            let exp = self.parse_power()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, SoryntelError> {
        match self.peek() {
            Some(ExprTok::Plus) => { self.pos += 1; self.parse_unary() }
            Some(ExprTok::Minus) => { self.pos += 1; Ok(-self.parse_unary()?) }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, SoryntelError> {
        let tok = self.peek().cloned().ok_or_else(|| {
            SoryntelError::ParseError("Unexpected end of Cp formula expression".into())
        })?;
        self.pos += 1;
        match tok {
            ExprTok::Num(v) => Ok(v),
            ExprTok::LParen => {
                let v = self.parse_expr()?;
                match self.peek() {
                    Some(ExprTok::RParen) => { self.pos += 1; Ok(v) }
                    _ => Err(SoryntelError::ParseError("Missing closing ')' in Cp formula".into())),
                }
            }
            ExprTok::Ident(name) => {
                if let Some(ExprTok::LParen) = self.peek() {
                    self.pos += 1;
                    let arg = self.parse_expr()?;
                    match self.peek() {
                        Some(ExprTok::RParen) => self.pos += 1,
                        _ => return Err(SoryntelError::ParseError("Missing ')' after function call".into())),
                    }
                    return match name.as_str() {
                        "LN" => Ok(arg.ln()),
                        "LOG" | "LOG10" => Ok(arg.log10()),
                        "EXP" => Ok(arg.exp()),
                        "SQRT" => Ok(arg.sqrt()),
                        "SIN" => Ok(arg.sin()),
                        "COS" => Ok(arg.cos()),
                        "TANH" => Ok(arg.tanh()),
                        "SINH" => Ok(arg.sinh()),
                        "COSH" => Ok(arg.cosh()),
                        "ABS" => Ok(arg.abs()),
                        _ => Err(SoryntelError::ParseError(format!("Unknown function '{name}'"))),
                    };
                }
                match name.as_str() {
                    "T" => Ok(self.t_k),
                    "A" => Ok(self.params[0]),
                    "B" => Ok(self.params[1]),
                    "C" => Ok(self.params[2]),
                    "D" => Ok(self.params[3]),
                    "E" => Ok(self.params[4]),
                    "F" => Ok(self.params[5]),
                    "R" => Ok(R_JOULE),
                    _ => Err(SoryntelError::ParseError(format!("Unknown variable '{name}'"))),
                }
            }
            _ => Err(SoryntelError::ParseError("Invalid token in Cp formula".into())),
        }
    }
}

/// Convert an external $C_p(T)$ function on `[t1_k, t2_k]` into Kelley `(A, B, C, D)` in SI `J/(mol·K)`.
///
/// Follows HSC6 Ch. 28.13: divides `[T1, T2]` into 10 uniform subintervals (11 sample nodes) and fits Kelley OLS.
pub fn convert_cp_function(
    expr: &str,
    unit: &str,
    t1_k: f64,
    t2_k: f64,
    params: &[f64; 6],
    mask: CpFitMask,
    locked_si: KelleyCoefficients,
) -> Result<(Vec<CpDataPoint>, CpFitResult), SoryntelError> {
    if t1_k <= 0.0 || t2_k <= t1_k {
        return Err(SoryntelError::InvalidTemperature(t1_k));
    }
    let scale = cp_unit_to_si_factor(unit)?;
    let mut points = Vec::with_capacity(11);
    for i in 0..=10 {
        let t_k = t1_k + (i as f64 / 10.0) * (t2_k - t1_k);
        let raw_cp = eval_cp_expression(expr, t_k, params)?;
        points.push(CpDataPoint {
            t_k,
            cp_j: raw_cp * scale,
        });
    }
    let fit = fit_cp_kelley(&points, mask, locked_si)?;
    Ok((points, fit))
}

/// Input coefficients for `Convert G(T) -> H, S, Cp` (`28.14`):
///
/// $G(T) = h + s\cdot 10^{-3}T + a\cdot 10^{-3}T\ln T + b\cdot 10^{-6}T^2 + c\,T^{-1} + d\cdot 10^{-9}T^3$
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GibbsCoeffsInput {
    pub h: f64,
    pub s: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub is_calories: bool,
    pub t_ref_k: f64,
}

impl Default for GibbsCoeffsInput {
    fn default() -> Self {
        Self {
            h: 0.0,
            s: 0.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            is_calories: true,
            t_ref_k: T_REF_K,
        }
    }
}

/// Output values of `Convert G(T)` in **SI units** (`J/mol`, `J/(mol·K)`) alongside raw unit factor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GibbsConvertOutput {
    pub h_ref_j: f64,
    pub s_ref_j: f64,
    pub g_ref_j: f64,
    pub kelley_si: KelleyCoefficients,
}

/// Convert $G(T)$ coefficients analytically into `(H(T_ref), S(T_ref), G(T_ref), A, B, C, D)` in SI.
pub fn convert_gibbs_function(input: &GibbsCoeffsInput) -> Result<GibbsConvertOutput, SoryntelError> {
    let t0 = input.t_ref_k;
    if t0 <= 0.0 || !t0.is_finite() {
        return Err(SoryntelError::InvalidTemperature(t0));
    }
    let e_mult = if input.is_calories { CAL_TO_JOULE } else { 1.0 };

    // Scaled polynomial terms in display energy units (kcal or kJ -> *1e3 to J/cal at the end)
    let h0 = input.h;
    let s0 = input.s * 1e-3;
    let a0 = input.a * 1e-3;
    let b0 = input.b * 1e-6;
    let c0 = input.c;
    let d0 = input.d * 1e-9;

    // Exact thermodynamic derivatives at T0:
    // H(T) = h - a0*T - b0*T^2 + 2*c0*T^-1 - 2*d0*T^3   [kcal or kJ]
    let h_disp = h0 - a0 * t0 - b0 * t0 * t0 + 2.0 * c0 / t0 - 2.0 * d0 * t0 * t0 * t0;
    // S(T) = (-s0 - a0*(1 + ln T) - 2*b0*T + c0*T^-2 - 3*d0*T^2) * 1e3  [cal or J]
    let s_disp =
        (-s0 - a0 * (1.0 + t0.ln()) - 2.0 * b0 * t0 + c0 / (t0 * t0) - 3.0 * d0 * t0 * t0)
            * 1000.0;
    let g_disp = h_disp - t0 * (s_disp / 1000.0);

    // Exact HSC Kelley comparison:
    // A = -a, B = -2*b, C = -0.02*c, D = -0.006*d
    let kelley_si = KelleyCoefficients {
        a: (-input.a) * e_mult,
        b: (-2.0 * input.b) * e_mult,
        c: (-0.02 * input.c) * e_mult,
        d: (-0.006 * input.d) * e_mult,
    };

    Ok(GibbsConvertOutput {
        h_ref_j: h_disp * 1000.0 * e_mult,
        s_ref_j: s_disp * e_mult,
        g_ref_j: g_disp * 1000.0 * e_mult,
        kelley_si,
    })
}

/// Apply HSC6 `Delta to Normal` (`28.14`): adds reference-element `(H, S, A, B, C, D)` multiplied by
/// the elemental stoichiometry $\nu_e$ of `target_formula`.
pub fn gibbs_delta_to_normal(
    repo: &dyn SpeciesRepository,
    target_formula: &str,
    delta_out: &GibbsConvertOutput,
    t_ref_k: f64,
) -> Result<GibbsConvertOutput, SoryntelError> {
    let (elements, _charge) = parse_hsc_formula_elements(target_formula)?;
    let mut sum_h = delta_out.h_ref_j;
    let mut sum_s = delta_out.s_ref_j;
    let mut sum_k = delta_out.kelley_si;

    for (el, &nu) in &elements {
        let ref_sp = repo.get_reference_element(el).ok_or_else(|| {
            SoryntelError::SpeciesNotFound(format!("Reference element for '{el}'"))
        })?;
        let (ref_elems, _) = parse_hsc_formula_elements(&ref_sp.formula)?;
        let ref_atoms = ref_elems.get(el).copied().unwrap_or(1.0).max(1.0);
        let factor = nu / ref_atoms;

        let (pt, _) = evaluate_species_si(&ref_sp, t_ref_k, false, false)?;
        let iv = ref_sp
            .intervals
            .iter()
            .find(|r| r.t_min <= t_ref_k + 1e-3 && t_ref_k - 1e-3 <= r.t_max)
            .unwrap_or(&ref_sp.intervals[0]);

        sum_h += factor * pt.h;
        sum_s += factor * pt.s;
        sum_k.a += factor * iv.kelley.a;
        sum_k.b += factor * iv.kelley.b;
        sum_k.c += factor * iv.kelley.c;
        sum_k.d += factor * iv.kelley.d;
    }

    Ok(GibbsConvertOutput {
        h_ref_j: sum_h,
        s_ref_j: sum_s,
        g_ref_j: sum_h - t_ref_k * sum_s,
        kelley_si: sum_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpeciesThermoInterval;
    use approx::assert_relative_eq;
    use std::collections::HashMap;

    struct MiniRepo(HashMap<String, ChemicalSpecies>);
    impl SpeciesRepository for MiniRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get(formula).cloned()
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            self.0.get(element_symbol).cloned()
        }
    }

    fn si_interval(
        t_min: f64,
        t_max: f64,
        phase: Phase,
        h_kcal: f64,
        s_cal: f64,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    ) -> SpeciesThermoInterval {
        SpeciesThermoInterval {
            t_min,
            t_max,
            phase,
            kelley: KelleyCoefficients {
                a: a * CAL_TO_JOULE,
                b: b * CAL_TO_JOULE,
                c: c * CAL_TO_JOULE,
                d: d * CAL_TO_JOULE,
            },
            h_or_delta_h_tr: h_kcal * 1000.0 * CAL_TO_JOULE,
            s_or_delta_s_tr: s_cal * CAL_TO_JOULE,
            reference: "Test".into(),
        }
    }

    #[test]
    fn test_s3_s4_stoichiometry_and_organic() {
        let (target, _) = parse_hsc_formula_elements("SiO4FeFe").unwrap();
        let (fayalite, _) = parse_hsc_formula_elements("*2FeO*SiO2").unwrap();
        let (beta, _) = parse_hsc_formula_elements("Fe2SiO4(B)").unwrap();
        assert!(stoichiometry_matches(&fayalite, &target, false));
        assert!(stoichiometry_matches(&beta, &target, false));

        let (ass, _) = parse_hsc_formula_elements("AsS").unwrap();
        let (as2s2, _) = parse_hsc_formula_elements("As2S2").unwrap();
        let (as4s4l, _) = parse_hsc_formula_elements("As4S4(l)").unwrap();
        assert!(!stoichiometry_matches(&as2s2, &ass, false));
        assert!(stoichiometry_matches(&as2s2, &ass, true));
        assert!(stoichiometry_matches(&as4s4l, &ass, true));

        let (propane, _) = parse_hsc_formula_elements("C3H8(g)").unwrap();
        let (ethylene, _) = parse_hsc_formula_elements("C2H4(g)").unwrap();
        assert!(is_organic_hsc(&propane));
        assert!(!is_organic_hsc(&ethylene));

        let mut flags = PhaseFilterFlags::default();
        flags.organic = false;
        assert!(!flags.matches_composition(&propane));
        assert!(flags.matches_composition(&ethylene));
    }

    #[test]
    fn test_s5_cu2s_transitions_and_liquid_extrapolation() {
        // S5: Cu2S transition entropies (920/376, 290/717, 2300/1402) and X->X(l) continuity
        let solids = vec![
            si_interval(298.15, 376.0, Phase::Solid, -19.000, 28.900, 12.7719, 18.2741, -0.0279, 0.5869),
            si_interval(376.0, 717.0, Phase::Solid, 0.920, 920.0 / 376.0, 26.8020, -7.4028, -0.0110, 0.0352),
            si_interval(717.0, 1402.0, Phase::Solid, 0.290, 290.0 / 717.0, 20.3200, 0.0, 0.0, 0.0),
        ];
        assert_relative_eq!(920.0 / 376.0, 2.447, epsilon = 1e-3);
        assert_relative_eq!(290.0 / 717.0, 0.404, epsilon = 1e-3);
        assert_relative_eq!(2300.0 / 1402.0, 1.641, epsilon = 1e-3);

        let liq_k = KelleyCoefficients {
            a: 20.0 * CAL_TO_JOULE,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        };
        let (h_l_298, s_l_298) =
            extrapolate_liquid_298(&solids, 1402.0, 2.300 * 1000.0 * CAL_TO_JOULE, liq_k).unwrap();

        let sp_liq = ChemicalSpecies {
            formula: "Cu2S(l)".into(),
            name: "Copper(I) sulfide liquid".into(),
            phase: Phase::Liquid,
            molecular_weight: 159.152,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 298.15,
                t_max: 2000.0,
                phase: Phase::Liquid,
                kelley: liq_k,
                h_or_delta_h_tr: h_l_298,
                s_or_delta_s_tr: s_l_298,
                reference: "Test".into(),
            }],
        };
        let mut all_ivs = solids;
        all_ivs.push(si_interval(1402.0, 2000.0, Phase::Liquid, 2.300, 2300.0 / 1402.0, 20.0, 0.0, 0.0, 0.0));
        let sp_full = ChemicalSpecies {
            formula: "Cu2S".into(),
            name: "Copper(I) sulfide".into(),
            phase: Phase::Solid,
            molecular_weight: 159.152,
            density_g_cm3: 5.6,
            charge: 0,
            intervals: all_ivs,
        };
        let (pt_full, _) = evaluate_species_si(&sp_full, 1500.0, false, false).unwrap();
        let (pt_liq, _) = evaluate_species_si(&sp_liq, 1500.0, false, false).unwrap();
        assert_relative_eq!(pt_full.h, pt_liq.h, epsilon = 1e-4);
        assert_relative_eq!(pt_full.s, pt_liq.s, epsilon = 1e-4);
        assert_relative_eq!(pt_full.g, pt_liq.g, epsilon = 1e-4);
    }

    #[test]
    fn test_s6_naf_fit_cp_data_and_mask() {
        let true_k = KelleyCoefficients {
            a: 11.166 * CAL_TO_JOULE,
            b: 1.652 * CAL_TO_JOULE,
            c: -0.594 * CAL_TO_JOULE,
            d: 1.546 * CAL_TO_JOULE,
        };
        let mut pts = Vec::new();
        for t in (300..=1200).step_by(100) {
            let t_k = t as f64;
            pts.push(CpDataPoint {
                t_k,
                cp_j: true_k.cp(t_k),
            });
        }

        let res_all = fit_cp_kelley(&pts, CpFitMask::default(), KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 }).unwrap();
        assert!(res_all.max_abs_error_pct < 1e-6);
        assert_relative_eq!(res_all.kelley_si.a / CAL_TO_JOULE, 11.166, epsilon = 1e-6);
        assert_relative_eq!(res_all.kelley_si.b / CAL_TO_JOULE, 1.652, epsilon = 1e-6);
        assert_relative_eq!(res_all.kelley_si.c / CAL_TO_JOULE, -0.594, epsilon = 1e-6);
        assert_relative_eq!(res_all.kelley_si.d / CAL_TO_JOULE, 1.546, epsilon = 1e-6);

        let mask_no_c = CpFitMask {
            fit_a: true,
            fit_b: true,
            fit_c: false,
            fit_d: true,
        };
        let res_no_c = fit_cp_kelley(&pts, mask_no_c, KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 }).unwrap();
        assert_relative_eq!(res_no_c.kelley_si.c, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn test_s7_cu2s_gibbs_conversion_and_delta_to_normal() {
        let input = GibbsCoeffsInput {
            h: -14.394,
            s: -86.625,
            a: 13.574,
            b: -17.817,
            c: -185.3,
            d: 0.0,
            is_calories: true,
            t_ref_k: 298.15,
        };
        let out = convert_gibbs_function(&input).unwrap();
        assert_relative_eq!(out.h_ref_j / (1000.0 * CAL_TO_JOULE), -18.100, epsilon = 0.002);
        assert_relative_eq!(out.s_ref_j / CAL_TO_JOULE, 4.252, epsilon = 0.002);
        assert_relative_eq!(out.g_ref_j / (1000.0 * CAL_TO_JOULE), -19.368, epsilon = 0.002);
        assert_relative_eq!(out.kelley_si.a / CAL_TO_JOULE, -13.574, epsilon = 1e-4);
        assert_relative_eq!(out.kelley_si.b / CAL_TO_JOULE, 35.634, epsilon = 1e-4);
        assert_relative_eq!(out.kelley_si.c / CAL_TO_JOULE, 3.706, epsilon = 1e-4);
        assert_relative_eq!(out.kelley_si.d / CAL_TO_JOULE, 0.0, epsilon = 1e-6);

        let mut map = HashMap::new();
        map.insert(
            "Cu".into(),
            ChemicalSpecies {
                formula: "Cu".into(),
                name: "Copper".into(),
                phase: Phase::Solid,
                molecular_weight: 63.546,
                density_g_cm3: 8.96,
                charge: 0,
                intervals: vec![si_interval(298.15, 1358.0, Phase::Solid, 0.0, 7.923, 5.41, 1.50, 0.0, 0.0)],
            },
        );
        map.insert(
            "S".into(),
            ChemicalSpecies {
                formula: "S".into(),
                name: "Sulfur".into(),
                phase: Phase::Solid,
                molecular_weight: 32.06,
                density_g_cm3: 2.07,
                charge: 0,
                intervals: vec![si_interval(298.15, 368.3, Phase::Solid, 0.0, 7.668, 3.58, 6.24, 0.0, 0.0)],
            },
        );
        let repo = MiniRepo(map);
        let normal = gibbs_delta_to_normal(&repo, "Cu2S", &out, 298.15).unwrap();
        assert_relative_eq!(normal.h_ref_j / (1000.0 * CAL_TO_JOULE), -18.100, epsilon = 0.002);
        assert_relative_eq!(normal.s_ref_j / CAL_TO_JOULE, 27.766, epsilon = 0.005);
    }

    #[test]
    fn test_s8_aqueous_scales_and_criss_cobble() {
        let sp = ChemicalSpecies {
            formula: "Mg(+2a)".into(),
            name: "Magnesium ion".into(),
            phase: Phase::AqueousIon,
            molecular_weight: 24.304,
            density_g_cm3: 0.0,
            charge: 2,
            intervals: vec![si_interval(298.15, 573.15, Phase::AqueousIon, -111.367, -33.0, -20.0, 0.0, 0.0, 0.0)],
        };
        let summary = analyze_aqueous_scales(&sp).unwrap();
        assert!(summary.criss_cobble_active);
        assert_relative_eq!(summary.s_abs_298_j / CAL_TO_JOULE, -43.0, epsilon = 1e-6);
        assert_relative_eq!(summary.delta_s_mix_j / CAL_TO_JOULE, -7.981, epsilon = 0.01);
        assert!(summary.cp_at_300c_j.is_some());
    }

    #[test]
    fn test_convert_cp_function_shomate() {
        let mut params = [0.0; 6];
        // Test exact linear identity in J/(kmol*K): 25000 + 10*T  =>  25 + 10*1e-3*T J/(mol*K)
        params[0] = 25000.0;
        params[1] = 10000.0;
        let (pts, fit) = convert_cp_function(
            "A + B*(T/1000)",
            "J/(kmol*K)",
            300.0,
            1300.0,
            &params,
            CpFitMask::default(),
            KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 },
        )
        .unwrap();
        assert_eq!(pts.len(), 11);
        assert_relative_eq!(fit.kelley_si.a, 25.0, epsilon = 1e-6);
        assert_relative_eq!(fit.kelley_si.b, 10.0, epsilon = 1e-6);
        assert!(fit.max_abs_error_pct < 1e-6);
    }
}
