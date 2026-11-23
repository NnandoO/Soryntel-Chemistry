//! Module 24 (Chapter 84) — HSC Geo: Mineralogical Calculations and Mineral Database.
//!
//! Full backend engine for all 6 Chapter 84 tools inside the unified HSC Geo window:
//! 1. `Database Viewer` (§84.2) — 5 sources (`Mineral Chemistry`, `Phase Chemistry`, `Mineral`,
//!    `Element`, `Mineral Dissolution`), multi-condition `Filter`, multi-keyword `Find`,
//!    `View / Edit`, `Add / Remove`, `Copy / Paste` TSV with column identification report.
//! 2. `Find by Properties` (§84.3) — Periodic Table 3-state element filter (`Must Contain` /
//!    `Must Not Contain` / `Neutral`) + property filters + HSC filter expression generator.
//! 3. `Catalogue` (§84.4) — Full 3,812-mineral catalog, multi-location analyses, variability
//!    statistics, scatter datasets, and elemental pie composition.
//! 4. `Find by Composition` (§84.5) — SEM/EDS mineral grain identification via Weighted Sum
//!    of Squares `WSSQ = sum(((Database_E - Given_E) / SD_E)^2)` with Oxide/Element conversion.
//! 5. `Petrological` (§84.6) — Preloaded `Bruvann` 7-sheet dataset, automatic 12-row `Identify`,
//!    element method priority resolution, and all 13 routines (`ARRANGE`, `ANION`, `CATION`,
//!    `VF`, `SF`, `VSF`, `CIPW`, `REE_N`, `CUMNAME`, `TRC`, `TRACE_N`, `ELEMENT`, `OXIDE`).
//! 6. `Modal` (§84.7) — Multi-round Element-to-Mineral Conversion (`LS`, `WLS`, `NNLS`, `WNNLS`),
//!    remainder (`Sum = 100%`), normalization, SG, Distribution, Fraction, Residuals, Bulk Ch.,
//!    and Monte Carlo `Error Estimation` (`Average ± 1 Sigma`, `Average`, `StdDev`, `Min-Max`, `CV`).

use crate::elements::PERIODIC_ELEMENTS as ELEMENTS;
use crate::minerals_data::MINERALS_RAW;
use crate::spreadsheet_export::{
    build_xlsx, ExportCell, ExportCellStyle, ExportCellValue, ExportSheet,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn make_export_sheet(name: String, rows: &[Vec<String>]) -> ExportSheet {
    let mut cells = Vec::new();
    let max_cols = rows.iter().map(Vec::len).max().unwrap_or(1);
    for (r, row) in rows.iter().enumerate() {
        for (c, txt) in row.iter().enumerate() {
            let value = if r > 0 {
                if let Ok(num) = txt.trim().parse::<f64>() {
                    ExportCellValue::Number(num)
                } else {
                    ExportCellValue::Text(txt.clone())
                }
            } else {
                ExportCellValue::Text(txt.clone())
            };
            cells.push(ExportCell {
                row: r,
                col: c,
                value,
                style: if r == 0 { ExportCellStyle::Header } else { ExportCellStyle::Default },
            });
        }
    }
    ExportSheet {
        name,
        cells,
        col_widths: vec![13.0; max_cols],
    }
}

// ============================================================================
// Common geochemical helpers: element parsing, oxide conversion, NNLS, RNG
// ============================================================================

/// Parse `"S=32.903; Zn=67.097; "` into an ordered `element -> wt%` map.
pub fn parse_composition_map(raw: &str) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for part in raw.split(';') {
        let p = part.trim();
        if let Some((k, v)) = p.split_once('=') {
            if let Ok(val) = v.trim().parse::<f64>() {
                let sym = k.trim();
                if !sym.is_empty() && val.is_finite() {
                    map.insert(sym.to_string(), val);
                }
            }
        }
    }
    map
}

/// Format an ordered `element -> wt%` map back to HSC `"El=val; "` string.
pub fn format_composition_map(map: &BTreeMap<String, f64>) -> String {
    let mut s = String::new();
    for (el, val) in map {
        if *val > 1e-6 {
            s.push_str(&format!("{el}={val:.3}; "));
        }
    }
    s
}

/// Parse `"\Zn\Fe\S\"` into a `BTreeSet<String>`.
pub fn parse_elements_set(raw: &str) -> BTreeSet<String> {
    raw.split('\\')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Format a set of element symbols into HSC `"\El1\El2\"` notation.
pub fn format_elements_set(els: &BTreeSet<String>) -> String {
    if els.is_empty() {
        return String::new();
    }
    let mut out = String::from("\\");
    for e in els {
        out.push_str(e);
        out.push('\\');
    }
    out
}

/// Oxide -> (cation symbol, element-wt / oxide-wt conversion factor).
pub fn oxide_to_element_factor(oxide: &str) -> Option<(&'static str, f64)> {
    match oxide.trim() {
        "SiO2" => Some(("Si", 28.0855 / 60.0843)),
        "TiO2" => Some(("Ti", 47.867 / 79.8658)),
        "Al2O3" => Some(("Al", (2.0 * 26.98154) / 101.9613)),
        "Fe2O3" | "Fe2O3T" => Some(("Fe", (2.0 * 55.845) / 159.6882)),
        "FeO" | "FeOT" => Some(("Fe", 55.845 / 71.8444)),
        "MnO" => Some(("Mn", 54.93804 / 70.9374)),
        "MgO" => Some(("Mg", 24.305 / 40.3044)),
        "CaO" => Some(("Ca", 40.078 / 56.0774)),
        "Na2O" => Some(("Na", (2.0 * 22.98977) / 61.9789)),
        "K2O" => Some(("K", (2.0 * 39.0983) / 94.196)),
        "P2O5" => Some(("P", (2.0 * 30.97376) / 141.9445)),
        "Cr2O3" => Some(("Cr", (2.0 * 51.9961) / 151.9904)),
        "NiO" => Some(("Ni", 58.6934 / 74.6928)),
        "CuO" => Some(("Cu", 63.546 / 79.5454)),
        "ZnO" => Some(("Zn", 65.38 / 81.3794)),
        "PbO" => Some(("Pb", 207.2 / 223.1994)),
        "SO3" => Some(("S", 32.065 / 80.0632)),
        "As2O3" => Some(("As", (2.0 * 74.9216) / 197.8414)),
        "Sb2O3" => Some(("Sb", (2.0 * 121.76) / 291.5182)),
        "Ag2O" => Some(("Ag", (2.0 * 107.8682) / 231.7358)),
        "SnO2" => Some(("Sn", 118.71 / 150.7088)),
        "WO3" => Some(("W", 183.84 / 231.8382)),
        "ZrO2" => Some(("Zr", 91.224 / 123.2228)),
        "BaO" => Some(("Ba", 137.327 / 153.3264)),
        "SrO" => Some(("Sr", 87.62 / 103.6194)),
        "V2O5" => Some(("V", (2.0 * 50.9415) / 181.88)),
        "CoO" => Some(("Co", 58.9332 / 74.9326)),
        "MoO3" => Some(("Mo", 95.95 / 143.9482)),
        "Bi2O3" => Some(("Bi", (2.0 * 208.9804) / 465.959)),
        "CO2" => Some(("C", 12.0107 / 44.0095)),
        "H2O" | "H2O+" | "H2O-" => Some(("H", (2.0 * 1.00794) / 18.01528)),
        _ => None,
    }
}

/// Element -> canonical oxide name & oxide/element factor.
pub fn element_to_oxide_factor(el: &str) -> Option<(&'static str, f64)> {
    match el.trim() {
        "Si" => Some(("SiO2", 60.0843 / 28.0855)),
        "Ti" => Some(("TiO2", 79.8658 / 47.867)),
        "Al" => Some(("Al2O3", 101.9613 / (2.0 * 26.98154))),
        "Fe" => Some(("Fe2O3", 159.6882 / (2.0 * 55.845))),
        "Mn" => Some(("MnO", 70.9374 / 54.93804)),
        "Mg" => Some(("MgO", 40.3044 / 24.305)),
        "Ca" => Some(("CaO", 56.0774 / 40.078)),
        "Na" => Some(("Na2O", 61.9789 / (2.0 * 22.98977))),
        "K" => Some(("K2O", 94.196 / (2.0 * 39.0983))),
        "P" => Some(("P2O5", 141.9445 / (2.0 * 30.97376))),
        "Cr" => Some(("Cr2O3", 151.9904 / (2.0 * 51.9961))),
        "Ni" => Some(("NiO", 74.6928 / 58.6934)),
        "Cu" => Some(("CuO", 79.5454 / 63.546)),
        "Zn" => Some(("ZnO", 81.3794 / 65.38)),
        "Pb" => Some(("PbO", 223.1994 / 207.2)),
        "C" => Some(("CO2", 44.0095 / 12.0107)),
        "H" => Some(("H2O", 18.01528 / (2.0 * 1.00794))),
        _ => None,
    }
}

/// All supported oxide names for dropdowns.
pub const COMMON_OXIDES: &[&str] = &[
    "SiO2", "TiO2", "Al2O3", "Fe2O3", "FeO", "MnO", "MgO", "CaO", "Na2O", "K2O", "P2O5", "Cr2O3",
    "NiO", "CuO", "ZnO", "PbO", "SO3", "CO2", "H2O",
];

/// Estimate realistic Specific Gravity (g/cm³) for common minerals or from chemistry.
pub fn estimate_mineral_sg(name: &str, group: &str, comp: &BTreeMap<String, f64>) -> f64 {
    let n = name.to_ascii_lowercase();
    match n.as_str() {
        "quartz" | "cristobalite" | "tridymite" => return 2.65,
        "chalcopyrite" => return 4.20,
        "pyrite" => return 5.01,
        "pyrrhotite" => return 4.61,
        "pentlandite" => return 4.80,
        "galena" => return 7.58,
        "sphalerite" => return 4.05,
        "bornite" => return 5.08,
        "chalcocite" => return 5.65,
        "covellite" => return 4.68,
        "tetrahedrite" => return 4.95,
        "tennantite" => return 4.65,
        "freibergite" => return 5.10,
        "arsenopyrite" => return 6.07,
        "molybdenite" => return 4.70,
        "magnetite" => return 5.18,
        "hematite" => return 5.26,
        "ilmenite" => return 4.72,
        "chromite" => return 4.60,
        "rutile" => return 4.23,
        "cassiterite" => return 6.99,
        "calcite" => return 2.71,
        "dolomite" => return 2.85,
        "siderite" => return 3.96,
        "albite" => return 2.62,
        "anorthite" => return 2.76,
        "orthoclase" | "microcline" | "k-feldspar" => return 2.56,
        "plagioclase" => return 2.68,
        "biotite" => return 3.09,
        "muscovite" => return 2.83,
        "chlorite" => return 2.80,
        "hornblende" | "actinolite" => return 3.20,
        "olivine" | "forsterite" => return 3.32,
        "fayalite" => return 4.39,
        "enstatite" | "orthopyroxene" => return 3.28,
        "diopside" | "augite" | "clinopyroxene" => return 3.35,
        "talc" => return 2.75,
        "serpentine" | "lizardite" | "antigorite" => return 2.55,
        "apatite" | "fluorapatite" => return 3.19,
        "barite" => return 4.48,
        "fluorite" => return 3.18,
        "gypsum" => return 2.32,
        "gold" => return 19.30,
        "silver" => return 10.50,
        "copper" => return 8.94,
        _ => {}
    }
    let g = group.to_ascii_lowercase();
    if g.contains("feldspar") {
        return 2.65;
    }
    if g.contains("pyroxene") || g.contains("amphibole") {
        return 3.25;
    }
    if g.contains("mica") {
        return 2.90;
    }
    if g.contains("garnet") {
        return 3.85;
    }
    if g.contains("spinel") {
        return 4.50;
    }
    if g.contains("carbonate") {
        return 2.85;
    }
    // Chemistry-based rough SG estimator
    let heavy = comp.get("Pb").copied().unwrap_or(0.0) * 0.065
        + comp.get("Au").copied().unwrap_or(0.0) * 0.16
        + comp.get("W").copied().unwrap_or(0.0) * 0.06
        + comp.get("Bi").copied().unwrap_or(0.0) * 0.055
        + comp.get("Hg").copied().unwrap_or(0.0) * 0.07
        + comp.get("Ag").copied().unwrap_or(0.0) * 0.05
        + comp.get("U").copied().unwrap_or(0.0) * 0.06
        + comp.get("Sn").copied().unwrap_or(0.0) * 0.04
        + comp.get("Fe").copied().unwrap_or(0.0) * 0.025
        + comp.get("Cu").copied().unwrap_or(0.0) * 0.025
        + comp.get("Zn").copied().unwrap_or(0.0) * 0.020
        + comp.get("Ni").copied().unwrap_or(0.0) * 0.026
        + comp.get("Sb").copied().unwrap_or(0.0) * 0.028
        + comp.get("As").copied().unwrap_or(0.0) * 0.025
        + comp.get("Ba").copied().unwrap_or(0.0) * 0.022;
    (2.60 + heavy).clamp(2.10, 12.0)
}

/// Deterministic fast PRNG (xorshift64*) with Box-Muller standard normal sampler.
#[derive(Debug, Clone)]
struct FastRng {
    state: u64,
}

impl FastRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().clamp(1e-12, 1.0 - 1e-12);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Solve small dense linear system `M * x = rhs` via Gaussian elimination with partial pivoting
/// and tiny Tikhonov regularization for numerical robustness.
fn solve_linear_system(mut m: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
    let n = rhs.len();
    if n == 0 {
        return Vec::new();
    }
    for i in 0..n {
        m[i][i] += 1e-12;
    }
    for col in 0..n {
        let mut pivot = col;
        let mut max_abs = m[col][col].abs();
        for (r, row) in m.iter().enumerate().skip(col + 1) {
            if row[col].abs() > max_abs {
                max_abs = row[col].abs();
                pivot = r;
            }
        }
        if max_abs < 1e-14 {
            continue;
        }
        if pivot != col {
            m.swap(col, pivot);
            rhs.swap(col, pivot);
        }
        let diag = m[col][col];
        for r in (col + 1)..n {
            let factor = m[r][col] / diag;
            if factor.abs() < 1e-18 {
                continue;
            }
            for c in col..n {
                m[r][c] -= factor * m[col][c];
            }
            rhs[r] -= factor * rhs[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = rhs[i];
        for c in (i + 1)..n {
            s -= m[i][c] * x[c];
        }
        let diag = m[i][i];
        x[i] = if diag.abs() > 1e-14 { s / diag } else { 0.0 };
    }
    x
}

/// Weighted Ordinary Least Squares: minimize `|| diag(sqrt(w)) * (A*x - b) ||_2^2`.
fn solve_wls(a: &[Vec<f64>], b: &[f64], w: &[f64]) -> Vec<f64> {
    let m = a.len();
    if m == 0 {
        return Vec::new();
    }
    let n = a[0].len();
    if n == 0 {
        return Vec::new();
    }
    let mut ata = vec![vec![0.0; n]; n];
    let mut atb = vec![0.0; n];
    for i in 0..m {
        let wi = w[i].max(0.0);
        for j in 0..n {
            let aij = a[i][j];
            atb[j] += wi * aij * b[i];
            for k in 0..n {
                ata[j][k] += wi * aij * a[i][k];
            }
        }
    }
    solve_linear_system(ata, atb)
}

/// Lawson-Hanson Active-Set Weighted Non-Negative Least Squares (`x >= 0`).
pub fn solve_wnnls(a: &[Vec<f64>], b: &[f64], w: &[f64]) -> Vec<f64> {
    let m = a.len();
    if m == 0 {
        return Vec::new();
    }
    let n = a[0].len();
    if n == 0 {
        return Vec::new();
    }
    let mut x = vec![0.0; n];
    let mut passive = vec![false; n];
    let max_iter = (n * 6).max(30);

    for _ in 0..max_iter {
        // Negative gradient g = A^T W (b - A x)
        let mut r = vec![0.0; m];
        for i in 0..m {
            let mut ax = 0.0;
            for j in 0..n {
                ax += a[i][j] * x[j];
            }
            r[i] = b[i] - ax;
        }
        let mut best_j = None;
        let mut best_g = 1e-9;
        for j in 0..n {
            if passive[j] {
                continue;
            }
            let mut g = 0.0;
            for i in 0..m {
                g += a[i][j] * w[i] * r[i];
            }
            if g > best_g {
                best_g = g;
                best_j = Some(j);
            }
        }
        let Some(enter) = best_j else {
            break;
        };
        passive[enter] = true;

        // Inner loop: ensure feasibility x >= 0
        for _ in 0..max_iter {
            let p_cols: Vec<usize> = (0..n).filter(|&j| passive[j]).collect();
            if p_cols.is_empty() {
                break;
            }
            let a_sub: Vec<Vec<f64>> = (0..m)
                .map(|i| p_cols.iter().map(|&j| a[i][j]).collect())
                .collect();
            let s_sub = solve_wls(&a_sub, b, w);
            let mut s = vec![0.0; n];
            for (&col, &val) in p_cols.iter().zip(s_sub.iter()) {
                s[col] = val;
            }
            if p_cols.iter().all(|&j| s[j] > 0.0) {
                x = s;
                break;
            }
            // Step toward boundary
            let mut alpha = f64::INFINITY;
            for &j in &p_cols {
                if s[j] <= 0.0 {
                    let denom = x[j] - s[j];
                    if denom > 1e-15 {
                        alpha = alpha.min(x[j] / denom);
                    }
                }
            }
            if !alpha.is_finite() || alpha <= 0.0 {
                for &j in &p_cols {
                    if s[j] <= 0.0 {
                        x[j] = 0.0;
                        passive[j] = false;
                    }
                }
                break;
            }
            for j in 0..n {
                x[j] += alpha * (s[j] - x[j]);
                if passive[j] && x[j] <= 1e-11 {
                    x[j] = 0.0;
                    passive[j] = false;
                }
            }
        }
    }
    for v in &mut x {
        if *v < 0.0 || !v.is_finite() {
            *v = 0.0;
        }
    }
    x
}

// ============================================================================
// 1. Mineral Database Store (§84.2 Database Viewer, §84.3 Properties, §84.4 Catalogue)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeoDbSource {
    MineralChemistry,
    PhaseChemistry,
    Mineral,
    Element,
    MineralDissolution,
}

impl GeoDbSource {
    pub const ALL: [GeoDbSource; 5] = [
        Self::MineralChemistry,
        Self::PhaseChemistry,
        Self::Mineral,
        Self::Element,
        Self::MineralDissolution,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MineralChemistry => "Mineral Chemistry",
            Self::PhaseChemistry => "Phase Chemistry",
            Self::Mineral => "Mineral",
            Self::Element => "Element",
            Self::MineralDissolution => "Mineral Dissolution",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoMineralRecord {
    pub id: u32,
    pub name: String,
    pub symbol: String,
    pub formula: String,
    pub empirical_formula: String,
    pub group: String,
    pub symbol_ref: String,
    pub note: String,
    pub composition_str: String,
    pub composition: BTreeMap<String, f64>,
    pub elements_str: String,
    pub elements: BTreeSet<String>,
    pub ore_mineral: String,
    pub density: f64,
    pub color: String,
    pub hardness: String,
    pub luster: String,
    pub mag_susc: String,
    pub formula_base: String,
    pub date_of_input: String,
    pub inputted_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineralChemRecord {
    pub id: u32,
    pub mineral_symbol: String,
    pub mineral: String,
    pub location: String,
    pub mineral_key: String,
    pub statistics: String,
    pub reference: String,
    pub sample_no: String,
    pub note1: String,
    pub note2: String,
    pub note3: String,
    pub database: String,
    pub sg: Option<f64>,
    pub values: BTreeMap<String, f64>,
}

impl MineralChemRecord {
    pub fn total_wt(&self) -> f64 {
        self.values.values().copied().sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseChemRecord {
    pub id: u32,
    pub phase_name: String,
    pub process_type: String,
    pub formula_or_desc: String,
    pub density: f64,
    pub reference: String,
    pub values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineralDissolutionRecord {
    pub id: u32,
    pub mineral: String,
    pub formula: String,
    pub bromine_methanol: f64,
    pub citric_acid: f64,
    pub hcl_10pct: f64,
    pub aqua_regia: f64,
    pub h2o2_nh4oac: f64,
    pub nacn: f64,
    pub reference: String,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    BeginsWith,
    EndsWith,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
}

impl FilterOp {
    pub const ALL: [FilterOp; 10] = [
        Self::Equals,
        Self::NotEquals,
        Self::Contains,
        Self::NotContains,
        Self::BeginsWith,
        Self::EndsWith,
        Self::GreaterThan,
        Self::GreaterOrEqual,
        Self::LessThan,
        Self::LessOrEqual,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Equals => "Equals",
            Self::NotEquals => "Does not equal",
            Self::Contains => "Contains",
            Self::NotContains => "Does not contain",
            Self::BeginsWith => "Begins with",
            Self::EndsWith => "Ends with",
            Self::GreaterThan => "Is greater than",
            Self::GreaterOrEqual => "Is greater than or equal to",
            Self::LessThan => "Is less than",
            Self::LessOrEqual => "Is less than or equal to",
        }
    }

    pub fn short_badge(self) -> &'static str {
        match self {
            Self::Equals => "=",
            Self::NotEquals => "≠",
            Self::Contains => "ABC",
            Self::NotContains => "!ABC",
            Self::BeginsWith => "A*",
            Self::EndsWith => "*C",
            Self::GreaterThan => ">",
            Self::GreaterOrEqual => "≥",
            Self::LessThan => "<",
            Self::LessOrEqual => "≤",
        }
    }

    pub fn format_clause(self, field: &str, value: &str) -> String {
        let v = value.trim();
        match self {
            Self::Equals => format!("[{field}] = '{v}'"),
            Self::NotEquals => format!("[{field}] <> '{v}'"),
            Self::Contains => format!("Contains([{field}], '{v}')"),
            Self::NotContains => format!("Not Contains([{field}], '{v}')"),
            Self::BeginsWith => format!("Starts with([{field}], '{v}')"),
            Self::EndsWith => format!("Ends with([{field}], '{v}')"),
            Self::GreaterThan => format!("[{field}] > {v}"),
            Self::GreaterOrEqual => format!("[{field}] >= {v}"),
            Self::LessThan => format!("[{field}] < {v}"),
            Self::LessOrEqual => format!("[{field}] <= {v}"),
        }
    }
}

pub fn format_filter_expression(conditions: &[FilterCondition], logic: FilterLogic) -> String {
    let joiner = match logic {
        FilterLogic::And => " And ",
        FilterLogic::Or => " Or ",
    };
    conditions
        .iter()
        .filter(|c| !c.value.trim().is_empty())
        .map(|c| c.op.format_clause(&c.field, &c.value))
        .collect::<Vec<_>>()
        .join(joiner)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterLogic {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub field: String,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteReport {
    pub identified_cols: usize,
    pub total_cols: usize,
    pub added_rows: usize,
    pub total_rows: usize,
    pub notes: Vec<String>,
}

/// Complete in-memory editable HSC Geo Mineral Database (§84.2–§84.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoDatabaseStore {
    pub minerals: Vec<GeoMineralRecord>,
    pub mineral_chemistry: Vec<MineralChemRecord>,
    pub phase_chemistry: Vec<PhaseChemRecord>,
    pub mineral_dissolution: Vec<MineralDissolutionRecord>,
}

impl Default for GeoDatabaseStore {
    fn default() -> Self {
        Self::load_default()
    }
}

impl GeoDatabaseStore {
    pub fn load_default() -> Self {
        let minerals = build_default_minerals();
        let mineral_chemistry = build_default_mineral_chemistry(&minerals);
        let phase_chemistry = build_default_phase_chemistry();
        let mineral_dissolution = build_default_mineral_dissolution();
        Self {
            minerals,
            mineral_chemistry,
            phase_chemistry,
            mineral_dissolution,
        }
    }

    pub fn next_mineral_id(&self) -> u32 {
        self.minerals.iter().map(|m| m.id).max().unwrap_or(0) + 1
    }

    pub fn next_chem_id(&self) -> u32 {
        self.mineral_chemistry.iter().map(|r| r.id).max().unwrap_or(0) + 1
    }

    pub fn next_phase_id(&self) -> u32 {
        self.phase_chemistry.iter().map(|r| r.id).max().unwrap_or(0) + 1
    }

    pub fn next_diss_id(&self) -> u32 {
        self.mineral_dissolution.iter().map(|r| r.id).max().unwrap_or(0) + 1
    }

    /// Look up the IMA/HSC mineral symbol (e.g. `Sphalerite` -> `Sp`) from the `Mineral` table.
    pub fn lookup_mineral_symbol(&self, mineral_name: &str) -> String {
        self.minerals
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(mineral_name.trim()))
            .map(|m| m.symbol.clone())
            .unwrap_or_else(|| mineral_name.trim().chars().take(3).collect())
    }

    /// Columns for each `GeoDbSource` in Database Viewer (matching HSC Geo Ch. 84 Figs. 3–10).
    pub fn columns_for_source(&self, source: GeoDbSource) -> Vec<String> {
        match source {
            GeoDbSource::MineralChemistry => vec![
                "Mineral ID", "Mineral Symbol", "Mineral Name", "Location", "Mineral Key",
                "Statistics", "Reference", "Sample No", "Note 1", "Note 2", "Note 3", "Database",
                "SG", "Cu", "Fe", "S", "Zn", "Pb", "Sb", "As", "Ag", "Ni", "Co", "Si", "O", "Mg",
                "Ca", "Al", "Mn", "Cd", "Hg", "In", "Bi", "Total",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            GeoDbSource::PhaseChemistry => vec![
                "Id", "Phase", "Process", "Formula / Desc", "Density", "Reference",
                "Fe", "Cu", "S", "Si", "O", "Ca", "Zn", "Pb", "Ni", "Al", "Mg", "Total",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            GeoDbSource::Mineral => vec![
                "Id", "Name", "Symbol", "Formula", "Group", "Density", "Color", "Hardness",
                "Luster", "Elements", "Composition",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            GeoDbSource::Element => vec![
                "Z", "Symbol", "Name", "AtomicWeight", "Category", "Density", "MeltingK",
                "BoilingK", "Electronegativity", "OxStates", "Crystal",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            GeoDbSource::MineralDissolution => vec![
                "Id", "Mineral", "Formula", "Br-MeOH %", "Citric %", "HCl 10% %", "Aqua Regia %",
                "H2O2-NH4OAc %", "NaCN %", "Reference", "Note",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    /// Read cell value as string for filtering, Find, and copy/export.
    pub fn row_cells(&self, source: GeoDbSource, row_idx: usize) -> Vec<String> {
        match source {
            GeoDbSource::MineralChemistry => {
                let r = &self.mineral_chemistry[row_idx];
                let els = [
                    "Cu", "Fe", "S", "Zn", "Pb", "Sb", "As", "Ag", "Ni", "Co", "Si", "O", "Mg",
                    "Ca", "Al", "Mn", "Cd", "Hg", "In", "Bi",
                ];
                let mut out = vec![
                    r.id.to_string(),
                    r.mineral_symbol.clone(),
                    r.mineral.clone(),
                    r.location.clone(),
                    r.mineral_key.clone(),
                    r.statistics.clone(),
                    r.reference.clone(),
                    r.sample_no.clone(),
                    r.note1.clone(),
                    r.note2.clone(),
                    r.note3.clone(),
                    r.database.clone(),
                    r.sg.map(|v| format!("{v:.2}")).unwrap_or_default(),
                ];
                for e in els {
                    out.push(
                        r.values
                            .get(e)
                            .map(|v| format!("{v:.3}"))
                            .unwrap_or_default(),
                    );
                }
                out.push(format!("{:.2}", r.total_wt()));
                out
            }
            GeoDbSource::PhaseChemistry => {
                let r = &self.phase_chemistry[row_idx];
                let els = ["Fe", "Cu", "S", "Si", "O", "Ca", "Zn", "Pb", "Ni", "Al", "Mg"];
                let mut out = vec![
                    r.id.to_string(),
                    r.phase_name.clone(),
                    r.process_type.clone(),
                    r.formula_or_desc.clone(),
                    format!("{:.2}", r.density),
                    r.reference.clone(),
                ];
                let mut tot = 0.0;
                for e in els {
                    if let Some(v) = r.values.get(e) {
                        tot += *v;
                        out.push(format!("{v:.2}"));
                    } else {
                        out.push(String::new());
                    }
                }
                out.push(format!("{tot:.2}"));
                out
            }
            GeoDbSource::Mineral => {
                let m = &self.minerals[row_idx];
                vec![
                    m.id.to_string(),
                    m.name.clone(),
                    m.symbol.clone(),
                    m.formula.clone(),
                    m.group.clone(),
                    format!("{:.2}", m.density),
                    m.color.clone(),
                    m.hardness.clone(),
                    m.luster.clone(),
                    m.elements_str.clone(),
                    m.composition_str.clone(),
                ]
            }
            GeoDbSource::Element => {
                let e = &ELEMENTS[row_idx];
                vec![
                    e.z.to_string(),
                    e.symbol.to_string(),
                    e.name.to_string(),
                    format!("{:.4}", e.atomic_weight),
                    format!("{:?}", e.category),
                    e.density.map(|v| format!("{v:.3}")).unwrap_or_default(),
                    e.melting_k.map(|v| format!("{v:.1}")).unwrap_or_default(),
                    e.boiling_k.map(|v| format!("{v:.1}")).unwrap_or_default(),
                    e.electronegativity.map(|v| format!("{v:.2}")).unwrap_or_default(),
                    e.ox_states.to_string(),
                    e.crystal.to_string(),
                ]
            }
            GeoDbSource::MineralDissolution => {
                let r = &self.mineral_dissolution[row_idx];
                vec![
                    r.id.to_string(),
                    r.mineral.clone(),
                    r.formula.clone(),
                    format!("{:.1}", r.bromine_methanol),
                    format!("{:.1}", r.citric_acid),
                    format!("{:.1}", r.hcl_10pct),
                    format!("{:.1}", r.aqua_regia),
                    format!("{:.1}", r.h2o2_nh4oac),
                    format!("{:.1}", r.nacn),
                    r.reference.clone(),
                    r.note.clone(),
                ]
            }
        }
    }

    pub fn row_count(&self, source: GeoDbSource) -> usize {
        match source {
            GeoDbSource::MineralChemistry => self.mineral_chemistry.len(),
            GeoDbSource::PhaseChemistry => self.phase_chemistry.len(),
            GeoDbSource::Mineral => self.minerals.len(),
            GeoDbSource::Element => ELEMENTS.len(),
            GeoDbSource::MineralDissolution => self.mineral_dissolution.len(),
        }
    }

    /// Filter + multi-keyword Find across a source table, returning matching row indices.
    pub fn query_rows(
        &self,
        source: GeoDbSource,
        conditions: &[FilterCondition],
        logic: FilterLogic,
        find_text: &str,
    ) -> Vec<usize> {
        let cols = self.columns_for_source(source);
        let n = self.row_count(source);
        let keywords: Vec<String> = find_text
            .split_whitespace()
            .map(|w| w.to_ascii_lowercase())
            .filter(|w| !w.is_empty())
            .collect();
        let active_conds: Vec<(usize, &FilterCondition)> = conditions
            .iter()
            .filter(|c| !c.value.trim().is_empty())
            .filter_map(|c| {
                cols.iter()
                    .position(|col| col.eq_ignore_ascii_case(&c.field))
                    .map(|idx| (idx, c))
            })
            .collect();

        let mut out = Vec::new();
        for r in 0..n {
            let cells = self.row_cells(source, r);
            if !active_conds.is_empty() {
                let eval = |col_idx: usize, cond: &FilterCondition| -> bool {
                    let cell = cells.get(col_idx).map(String::as_str).unwrap_or("");
                    let cv = cell.to_ascii_lowercase();
                    let target = cond.value.trim().to_ascii_lowercase();
                    match cond.op {
                        FilterOp::Contains => cv.contains(&target),
                        FilterOp::NotContains => !cv.contains(&target),
                        FilterOp::BeginsWith => cv.starts_with(&target),
                        FilterOp::EndsWith => cv.ends_with(&target),
                        FilterOp::Equals => cv == target,
                        FilterOp::NotEquals => cv != target,
                        FilterOp::GreaterThan => {
                            let a = cell.trim().parse::<f64>().unwrap_or(f64::NAN);
                            let b = cond.value.trim().parse::<f64>().unwrap_or(f64::NAN);
                            a.is_finite() && b.is_finite() && a > b
                        }
                        FilterOp::GreaterOrEqual => {
                            let a = cell.trim().parse::<f64>().unwrap_or(f64::NAN);
                            let b = cond.value.trim().parse::<f64>().unwrap_or(f64::NAN);
                            a.is_finite() && b.is_finite() && a >= b
                        }
                        FilterOp::LessThan => {
                            let a = cell.trim().parse::<f64>().unwrap_or(f64::NAN);
                            let b = cond.value.trim().parse::<f64>().unwrap_or(f64::NAN);
                            a.is_finite() && b.is_finite() && a < b
                        }
                        FilterOp::LessOrEqual => {
                            let a = cell.trim().parse::<f64>().unwrap_or(f64::NAN);
                            let b = cond.value.trim().parse::<f64>().unwrap_or(f64::NAN);
                            a.is_finite() && b.is_finite() && a <= b
                        }
                    }
                };
                let ok = match logic {
                    FilterLogic::And => active_conds.iter().all(|&(ci, c)| eval(ci, c)),
                    FilterLogic::Or => active_conds.iter().any(|&(ci, c)| eval(ci, c)),
                };
                if !ok {
                    continue;
                }
            }
            if !keywords.is_empty() {
                // Exact HSC Geo §84.2 Find behavior (Fig. 7): space-separated tokens match ANY (OR)
                let joined = cells.join(" ").to_ascii_lowercase();
                if !keywords.iter().any(|kw| joined.contains(kw)) {
                    continue;
                }
            }
            out.push(r);
        }
        out
    }

    /// Format filtered rows as TSV for clipboard Copy Data.
    pub fn export_tsv(&self, source: GeoDbSource, row_indices: &[usize]) -> String {
        let mut lines = Vec::with_capacity(row_indices.len() + 1);
        lines.push(self.columns_for_source(source).join("\t"));
        for &r in row_indices {
            lines.push(self.row_cells(source, r).join("\t"));
        }
        lines.join("\n")
    }

    /// Paste TSV/CSV data into `MineralChemistry`, `Mineral`, `PhaseChemistry`, or `MineralDissolution`
    /// (§84.2.2 Paste Data with column identification summary!).
    pub fn paste_tsv(&mut self, source: GeoDbSource, text: &str) -> PasteReport {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .collect();
        if lines.len() < 2 {
            return PasteReport {
                identified_cols: 0,
                total_cols: 0,
                added_rows: 0,
                total_rows: 0,
                notes: vec!["Provide a header row plus at least one data row (tab- or comma-separated).".into()],
            };
        }
        let delim = if lines[0].contains('\t') { '\t' } else { ',' };
        let headers: Vec<String> = lines[0].split(delim).map(|s| s.trim().to_string()).collect();
        let total_cols = headers.len();
        let total_rows = lines.len() - 1;
        let known_els: BTreeSet<&str> = ELEMENTS.iter().map(|e| e.symbol).collect();

        match source {
            GeoDbSource::MineralChemistry => {
                // Exact §84.2.2 Paste Data header identification rules (Image 11):
                // Metadata fields + elemental wt% columns (`Fe`, `Fe %`, `Zn %`, `S %`, `Cd %`).
                // Oxides (`CuO %`) and non-% units (`In ppm`) are flagged as unidentified in Mineral Chemistry.
                let is_valid_el_pct = |h: &str| -> Option<String> {
                    let parts: Vec<&str> = h.split_whitespace().collect();
                    let sym = parts.first().copied().unwrap_or("");
                    if !known_els.contains(sym) {
                        return None;
                    }
                    if parts.len() == 1 || parts[1] == "%" || parts[1].eq_ignore_ascii_case("wt%") || parts[1].eq_ignore_ascii_case("wt-%") {
                        Some(sym.to_string())
                    } else {
                        None
                    }
                };
                let is_meta_col = |hl: &str| {
                    matches!(
                        hl,
                        "mineral" | "mineral name" | "name" | "mineral symbol" | "symbol"
                            | "location" | "mineral key" | "statistics" | "reference"
                            | "sample" | "sample no" | "note" | "note1" | "note 1"
                            | "note2" | "note 2" | "note3" | "note 3" | "database" | "sg"
                    )
                };
                let mut identified_cols = 0usize;
                let mut unidentified = Vec::new();
                for h in &headers {
                    let hl = h.to_ascii_lowercase();
                    if is_meta_col(hl.as_str()) || is_valid_el_pct(h).is_some() {
                        identified_cols += 1;
                    } else {
                        unidentified.push(h.clone());
                    }
                }
                let mut added_rows = 0usize;
                for line in &lines[1..] {
                    let cells: Vec<&str> = line.split(delim).map(str::trim).collect();
                    let mut rec = MineralChemRecord {
                        id: self.next_chem_id(),
                        mineral_symbol: String::new(),
                        mineral: String::new(),
                        location: "User Import".into(),
                        mineral_key: String::new(),
                        statistics: String::new(),
                        reference: "Pasted".into(),
                        sample_no: String::new(),
                        note1: String::new(),
                        note2: String::new(),
                        note3: String::new(),
                        database: "User".into(),
                        sg: None,
                        values: BTreeMap::new(),
                    };
                    for (ci, h) in headers.iter().enumerate() {
                        let val = cells.get(ci).copied().unwrap_or("");
                        if val.is_empty() {
                            continue;
                        }
                        let hl = h.to_ascii_lowercase();
                        match hl.as_str() {
                            "mineral" | "mineral name" | "name" => rec.mineral = val.to_string(),
                            "mineral symbol" | "symbol" => rec.mineral_symbol = val.to_string(),
                            "location" => rec.location = val.to_string(),
                            "mineral key" => rec.mineral_key = val.to_string(),
                            "statistics" => rec.statistics = val.to_string(),
                            "reference" => rec.reference = val.to_string(),
                            "sample" | "sample no" => rec.sample_no = val.to_string(),
                            "note" | "note1" | "note 1" => rec.note1 = val.to_string(),
                            "note2" | "note 2" => rec.note2 = val.to_string(),
                            "note3" | "note 3" => rec.note3 = val.to_string(),
                            "database" => rec.database = val.to_string(),
                            "sg" => rec.sg = val.replace(',', ".").parse().ok(),
                            _ => {
                                if let Some(sym) = is_valid_el_pct(h) {
                                    if let Ok(num) = val.replace(',', ".").parse::<f64>() {
                                        *rec.values.entry(sym).or_insert(0.0) += num;
                                    }
                                }
                            }
                        }
                    }
                    if !rec.mineral.is_empty() && !rec.values.is_empty() {
                        if rec.mineral_symbol.is_empty() {
                            rec.mineral_symbol = self.lookup_mineral_symbol(&rec.mineral);
                        }
                        if rec.sg.is_none() {
                            rec.sg = Some(estimate_mineral_sg(&rec.mineral, "", &rec.values));
                        }
                        self.mineral_chemistry.push(rec);
                        added_rows += 1;
                    }
                }
                let mut notes = vec![format!(
                    "Columns identified: {identified_cols} of {total_cols}\nRows added: {added_rows} of {total_rows}"
                )];
                if !unidentified.is_empty() {
                    notes.push(format!("Unidentified columns (skipped): {}", unidentified.join(", ")));
                }
                PasteReport {
                    identified_cols,
                    total_cols,
                    added_rows,
                    total_rows,
                    notes,
                }
            }
            GeoDbSource::Mineral => {
                let mut identified_cols = 0usize;
                for h in &headers {
                    if matches!(
                        h.to_ascii_lowercase().as_str(),
                        "name" | "mineral" | "symbol" | "code" | "formula" | "group" | "density" | "sg" | "color" | "hardness" | "luster" | "composition"
                    ) {
                        identified_cols += 1;
                    }
                }
                let mut added_rows = 0usize;
                for line in &lines[1..] {
                    let cells: Vec<&str> = line.split(delim).map(str::trim).collect();
                    let mut rec = GeoMineralRecord {
                        id: self.next_mineral_id(),
                        name: String::new(),
                        symbol: String::new(),
                        formula: String::new(),
                        empirical_formula: String::new(),
                        group: String::new(),
                        symbol_ref: "User".into(),
                        note: "Pasted".into(),
                        composition_str: String::new(),
                        composition: BTreeMap::new(),
                        elements_str: String::new(),
                        elements: BTreeSet::new(),
                        ore_mineral: String::new(),
                        density: 3.0,
                        color: String::new(),
                        hardness: String::new(),
                        luster: String::new(),
                        mag_susc: String::new(),
                        formula_base: String::new(),
                        date_of_input: "2026".into(),
                        inputted_by: "User".into(),
                    };
                    for (ci, h) in headers.iter().enumerate() {
                        let val = cells.get(ci).copied().unwrap_or("");
                        match h.to_ascii_lowercase().as_str() {
                            "name" | "mineral" => rec.name = val.to_string(),
                            "symbol" | "code" => rec.symbol = val.to_string(),
                            "formula" => rec.formula = val.to_string(),
                            "group" => rec.group = val.to_string(),
                            "density" | "sg" => {
                                if let Ok(v) = val.parse::<f64>() {
                                    rec.density = v;
                                }
                            }
                            "color" => rec.color = val.to_string(),
                            "hardness" => rec.hardness = val.to_string(),
                            "luster" => rec.luster = val.to_string(),
                            "composition" => {
                                rec.composition_str = val.to_string();
                                rec.composition = parse_composition_map(val);
                                rec.elements = rec.composition.keys().cloned().collect();
                                rec.elements_str = format_elements_set(&rec.elements);
                            }
                            _ => {}
                        }
                    }
                    if !rec.name.is_empty() {
                        if rec.symbol.is_empty() {
                            rec.symbol = rec.name.chars().take(3).collect();
                        }
                        self.minerals.push(rec);
                        added_rows += 1;
                    }
                }
                PasteReport {
                    identified_cols,
                    total_cols,
                    added_rows,
                    total_rows,
                    notes: vec![format!(
                        "{identified_cols} out of {total_cols} columns identified; {added_rows} out of {total_rows} minerals added."
                    )],
                }
            }
            GeoDbSource::PhaseChemistry => {
                let mut added_rows = 0usize;
                for line in &lines[1..] {
                    let cells: Vec<&str> = line.split(delim).map(str::trim).collect();
                    let name = cells.first().copied().unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let mut values = BTreeMap::new();
                    for (ci, h) in headers.iter().enumerate() {
                        if known_els.contains(h.as_str()) {
                            if let Some(Ok(v)) = cells.get(ci).map(|s| s.parse::<f64>()) {
                                values.insert(h.clone(), v);
                            }
                        }
                    }
                    self.phase_chemistry.push(PhaseChemRecord {
                        id: self.next_phase_id(),
                        phase_name: name.to_string(),
                        process_type: cells.get(1).copied().unwrap_or("User").to_string(),
                        formula_or_desc: cells.get(2).copied().unwrap_or("").to_string(),
                        density: cells.get(3).and_then(|s| s.parse().ok()).unwrap_or(3.5),
                        reference: "Pasted".into(),
                        values,
                    });
                    added_rows += 1;
                }
                PasteReport {
                    identified_cols: total_cols,
                    total_cols,
                    added_rows,
                    total_rows,
                    notes: vec![format!("Added {added_rows} rows to Phase Chemistry.")],
                }
            }
            GeoDbSource::MineralDissolution => {
                let mut added_rows = 0usize;
                for line in &lines[1..] {
                    let cells: Vec<&str> = line.split(delim).map(str::trim).collect();
                    let name = cells.first().copied().unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let pf = |i: usize| cells.get(i).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    self.mineral_dissolution.push(MineralDissolutionRecord {
                        id: self.next_diss_id(),
                        mineral: name.to_string(),
                        formula: cells.get(1).copied().unwrap_or("").to_string(),
                        bromine_methanol: pf(2),
                        citric_acid: pf(3),
                        hcl_10pct: pf(4),
                        aqua_regia: pf(5),
                        h2o2_nh4oac: pf(6),
                        nacn: pf(7),
                        reference: "Pasted".into(),
                        note: String::new(),
                    });
                    added_rows += 1;
                }
                PasteReport {
                    identified_cols: total_cols,
                    total_cols,
                    added_rows,
                    total_rows,
                    notes: vec![format!("Added {added_rows} rows to Mineral Dissolution.")],
                }
            }
            GeoDbSource::Element => PasteReport {
                identified_cols: 0,
                total_cols,
                added_rows: 0,
                total_rows,
                notes: vec!["Element table is read-only IUPAC reference data.".into()],
            },
        }
    }

    /// Return all `MineralChemistry` analyses for a mineral name, always ensuring a `[stoichiometric]`
    /// entry exists from the `Mineral` table when composition data is available.
    pub fn analyses_for_mineral(&self, mineral_name: &str) -> Vec<MineralChemRecord> {
        let mut out: Vec<MineralChemRecord> = self
            .mineral_chemistry
            .iter()
            .filter(|r| r.mineral.eq_ignore_ascii_case(mineral_name))
            .cloned()
            .collect();
        let has_stoich = out
            .iter()
            .any(|r| r.location.eq_ignore_ascii_case("[stoichiometric]"));
        if !has_stoich {
            if let Some(m) = self
                .minerals
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(mineral_name))
            {
                if !m.composition.is_empty() {
                    out.insert(
                        0,
                        MineralChemRecord {
                            id: m.id,
                            mineral_symbol: m.symbol.clone(),
                            mineral: m.name.clone(),
                            location: "[stoichiometric]".into(),
                            mineral_key: "HSC Formula".into(),
                            statistics: String::new(),
                            reference: "HSC Formula".into(),
                            sample_no: String::new(),
                            note1: String::new(),
                            note2: String::new(),
                            note3: m.formula.clone(),
                            database: "HSC".into(),
                            sg: Some(m.density),
                            values: m.composition.clone(),
                        },
                    );
                }
            }
        }
        out
    }
}

// ============================================================================
// 2. Find by Properties (§84.3)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementFilterState {
    Neutral,
    MustContain,
    MustNotContain,
}

impl ElementFilterState {
    pub fn cycle(self) -> Self {
        match self {
            Self::Neutral => Self::MustContain,
            Self::MustContain => Self::MustNotContain,
            Self::MustNotContain => Self::Neutral,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertySearchFilter {
    pub element_states: BTreeMap<String, ElementFilterState>,
    pub group_contains: String,
    pub name_contains: String,
    pub density_min: Option<f64>,
    pub density_max: Option<f64>,
}

impl PropertySearchFilter {
    /// Generate the exact HSC Geo text filter expression (§84.3).
    pub fn to_hsc_expression(&self) -> String {
        let mut parts = Vec::new();
        for (el, state) in &self.element_states {
            match state {
                ElementFilterState::MustContain => {
                    parts.push(format!("Contains([Elements], '\\{el}\\')"));
                }
                ElementFilterState::MustNotContain => {
                    parts.push(format!("Not Contains([Elements], '\\{el}\\')"));
                }
                ElementFilterState::Neutral => {}
            }
        }
        if !self.group_contains.trim().is_empty() {
            parts.push(format!("Contains([Group], '{}')", self.group_contains.trim()));
        }
        if !self.name_contains.trim().is_empty() {
            parts.push(format!("Contains([Name], '{}')", self.name_contains.trim()));
        }
        if let Some(lo) = self.density_min {
            parts.push(format!("[Density] >= {lo:.2}"));
        }
        if let Some(hi) = self.density_max {
            parts.push(format!("[Density] <= {hi:.2}"));
        }
        parts.join(" And ")
    }

    pub fn clear(&mut self) {
        self.element_states.clear();
        self.group_contains.clear();
        self.name_contains.clear();
        self.density_min = None;
        self.density_max = None;
    }
}

/// Filter all minerals in `db` by Periodic Table + physical property criteria (§84.3).
pub fn filter_minerals_by_properties(
    db: &GeoDatabaseStore,
    filter: &PropertySearchFilter,
) -> Vec<usize> {
    let required: Vec<&str> = filter
        .element_states
        .iter()
        .filter(|(_, &st)| st == ElementFilterState::MustContain)
        .map(|(k, _)| k.as_str())
        .collect();
    let excluded: Vec<&str> = filter
        .element_states
        .iter()
        .filter(|(_, &st)| st == ElementFilterState::MustNotContain)
        .map(|(k, _)| k.as_str())
        .collect();
    let grp = filter.group_contains.trim().to_ascii_lowercase();
    let nam = filter.name_contains.trim().to_ascii_lowercase();

    db.minerals
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            if !required.iter().all(|e| m.elements.contains(*e)) {
                return false;
            }
            if excluded.iter().any(|e| m.elements.contains(*e)) {
                return false;
            }
            if !grp.is_empty() && !m.group.to_ascii_lowercase().contains(&grp) {
                return false;
            }
            if !nam.is_empty() && !m.name.to_ascii_lowercase().contains(&nam) {
                return false;
            }
            if let Some(lo) = filter.density_min {
                if m.density < lo {
                    return false;
                }
            }
            if let Some(hi) = filter.density_max {
                if m.density > hi {
                    return false;
                }
            }
            true
        })
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// 3. Find by Composition (§84.5 — WSSQ mineral grain identification)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompUnit {
    WtPct,
    Ppm,
    Ppb,
}

impl CompUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::WtPct => "%",
            Self::Ppm => "ppm",
            Self::Ppb => "ppb",
        }
    }

    pub fn to_wt_pct(self, v: f64) -> f64 {
        match self {
            Self::WtPct => v,
            Self::Ppm => v * 1e-4,
            Self::Ppb => v * 1e-7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompCriterion {
    pub species: String,
    pub is_oxide: bool,
    pub unit: CompUnit,
    pub value: f64,
    pub sd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionMatch {
    pub mineral: String,
    pub location: String,
    pub formula: String,
    pub wssq: f64,
    pub database_values: BTreeMap<String, f64>,
    pub total_wt: f64,
}

/// Default §84.5 EDS grain example (`26% Cu, 22% Sb, 27% S, 5% Fe, 20% Ag` -> Tetrahedrite).
pub fn default_eds_criteria() -> Vec<CompCriterion> {
    vec![
        CompCriterion { species: "Cu".into(), is_oxide: false, unit: CompUnit::WtPct, value: 26.0, sd: 1.0 },
        CompCriterion { species: "Sb".into(), is_oxide: false, unit: CompUnit::WtPct, value: 22.0, sd: 1.0 },
        CompCriterion { species: "S".into(), is_oxide: false, unit: CompUnit::WtPct, value: 27.0, sd: 1.0 },
        CompCriterion { species: "Fe".into(), is_oxide: false, unit: CompUnit::WtPct, value: 5.0, sd: 1.0 },
        CompCriterion { species: "Ag".into(), is_oxide: false, unit: CompUnit::WtPct, value: 20.0, sd: 1.0 },
    ]
}

/// Identify candidate minerals via Weighted Sum of Squares (§84.5):
/// `WSSQ = sum_E ((Database_E - Given_E) / SD_E)^2`.
pub fn find_minerals_by_composition_wssq(
    db: &GeoDatabaseStore,
    criteria: &[CompCriterion],
    top_n: usize,
) -> Vec<CompositionMatch> {
    if criteria.is_empty() {
        return Vec::new();
    }
    // Convert all criteria to elemental wt%
    let mut targets: Vec<(String, f64, f64)> = Vec::new();
    for c in criteria {
        let val_pct = c.unit.to_wt_pct(c.value);
        let sd_pct = c.unit.to_wt_pct(c.sd.abs().max(1e-4)).max(1e-4);
        if c.is_oxide {
            if let Some((el, fac)) = oxide_to_element_factor(&c.species) {
                targets.push((el.to_string(), val_pct * fac, (sd_pct * fac).max(1e-4)));
                continue;
            }
        }
        targets.push((c.species.trim().to_string(), val_pct, sd_pct));
    }

    let formula_lookup: BTreeMap<String, String> = db
        .minerals
        .iter()
        .map(|m| (m.name.to_ascii_lowercase(), m.formula.clone()))
        .collect();

    let mut matches: Vec<CompositionMatch> = Vec::new();
    let mut seen_stoich: BTreeSet<String> = BTreeSet::new();

    // 1) Score all real MineralChemistry records
    for rec in &db.mineral_chemistry {
        if rec.values.is_empty() {
            continue;
        }
        if rec.location.eq_ignore_ascii_case("[stoichiometric]") {
            seen_stoich.insert(rec.mineral.to_ascii_lowercase());
        }
        let wssq = compute_wssq(&rec.values, &targets);
        matches.push(CompositionMatch {
            mineral: rec.mineral.clone(),
            location: rec.location.clone(),
            formula: formula_lookup
                .get(&rec.mineral.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default(),
            wssq,
            database_values: rec.values.clone(),
            total_wt: rec.total_wt(),
        });
    }

    // 2) Score all 3,812 stoichiometric Mineral records
    for m in &db.minerals {
        if m.composition.is_empty() || seen_stoich.contains(&m.name.to_ascii_lowercase()) {
            continue;
        }
        let wssq = compute_wssq(&m.composition, &targets);
        matches.push(CompositionMatch {
            mineral: m.name.clone(),
            location: "[stoichiometric]".into(),
            formula: m.formula.clone(),
            wssq,
            database_values: m.composition.clone(),
            total_wt: m.composition.values().copied().sum(),
        });
    }

    matches.sort_by(|a, b| a.wssq.total_cmp(&b.wssq));
    matches.truncate(top_n.max(1));
    matches
}

fn compute_wssq(db_vals: &BTreeMap<String, f64>, targets: &[(String, f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for (el, given, sd) in targets {
        let db_v = db_vals.get(el).copied().unwrap_or(0.0);
        let d = (db_v - given) / sd;
        sum += d * d;
    }
    sum
}

// ============================================================================
// 4. Petrological Calculations (§84.6 — Bruvann + 13 routines)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PetroGroup {
    Rock,
    Mineral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PetroRoutine {
    Arrange,
    Anion,
    Cation,
    Vf,
    Sf,
    Vsf,
    Cipw,
    ReeN,
    Cumname,
    Trc,
    TraceN,
    Element,
    Oxide,
}

impl PetroRoutine {
    pub const ALL: [PetroRoutine; 13] = [
        Self::Arrange,
        Self::Anion,
        Self::Cation,
        Self::Vf,
        Self::Sf,
        Self::Vsf,
        Self::Cipw,
        Self::ReeN,
        Self::Cumname,
        Self::Trc,
        Self::TraceN,
        Self::Element,
        Self::Oxide,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Arrange => "ARRANGE",
            Self::Anion => "ANION",
            Self::Cation => "CATION",
            Self::Vf => "VF",
            Self::Sf => "SF",
            Self::Vsf => "VSF",
            Self::Cipw => "CIPW",
            Self::ReeN => "REE_N",
            Self::Cumname => "CUMNAME",
            Self::Trc => "TRC",
            Self::TraceN => "TRACE_N",
            Self::Element => "ELEMENT",
            Self::Oxide => "OXIDE",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Arrange => "Arrange multi-source sample data using method priority",
            Self::Anion => "Anion-based mineral formula recalculation (O-normalized)",
            Self::Cation => "Cation-normalized mineral formula & end-members",
            Self::Vf => "Volatile-free silicate normalization (to 100%)",
            Self::Sf => "Sulfide fraction (Ccp, Pn, Cpn, Sp, Ga, Mb, Apy, Po, Py) & 100% tenors",
            Self::Vsf => "Volatile- and sulfide-free silicate composition",
            Self::Cipw => "CIPW normative mineralogy (Q, Or, Ab, An, Di, Hy, Ol, Mt, Il, Ap, Cm)",
            Self::ReeN => "Chondrite-normalized REE (McDonough & Sun 1995) + Eu/Eu*",
            Self::Cumname => "4-part Cumulus rock naming (o/b/a/p/m/c/t/$ + A/M/O + C/R)",
            Self::Trc => "Trace-element geochemical ratios (Cu/Ni, Cu/Pd, Pd/Ir, Mg#)",
            Self::TraceN => "Primitive Mantle-normalized multi-element spidergram",
            Self::Element => "Convert major oxides to elemental wt%",
            Self::Oxide => "Convert major elements to oxide wt%",
        }
    }
}

/// One column's 12 system rows produced by `Identify` (§84.6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedColumnMeta {
    pub header: String,
    pub element: String,
    pub method: String,
    pub unit: String,
    pub table: String,
    pub field: String,
    pub data_type: String,
    pub length: usize,
    pub digits: usize,
    pub indexed: bool,
    pub multiplier: f64,
    pub identified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetroSourceSheet {
    pub name: String,
    pub enabled: bool,
    pub method_code: String,
    pub columns: Vec<IdentifiedColumnMeta>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetroResultSheet {
    pub routine: PetroRoutine,
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetrologicalProject {
    pub name: String,
    pub sheets: Vec<PetroSourceSheet>,
    pub identified: bool,
    pub group: PetroGroup,
    pub anion_oxygens: f64,
    pub fe2o3_feot_ratio: f64,
    pub enabled_routines: BTreeSet<PetroRoutine>,
    /// Element -> ordered list of analytical method codes (highest priority first).
    pub method_priorities: BTreeMap<String, Vec<String>>,
    pub results: Vec<PetroResultSheet>,
    pub log: Vec<String>,
}

impl Default for PetrologicalProject {
    fn default() -> Self {
        let mut proj = Self::bruvann_tutorial();
        proj.identify_all();
        proj.calculate();
        proj
    }
}

impl PetrologicalProject {
    /// Preloaded **Bruvann** 7-sheet tutorial dataset (`Header`, `LOI`, `LECO`, `BM`, `XRF`, `PGE`, `REE`)
    /// matching §84.6.1–§84.6.4.
    pub fn bruvann_tutorial() -> Self {
        let header = PetroSourceSheet {
            name: "Header".into(),
            enabled: true,
            method_code: "HEADER".into(),
            columns: make_raw_cols(&["SampleID", "Drillhole", "From_m", "To_m", "Lithology"]),
            rows: vec![
                vec!["BR-01", "BH-101", "12.0", "14.0", "Dunite"].into_iter().map(String::from).collect(),
                vec!["BR-02", "BH-101", "28.0", "30.0", "Harzburgite"].into_iter().map(String::from).collect(),
                vec!["BR-03", "BH-101", "45.0", "47.0", "Olivine Norite"].into_iter().map(String::from).collect(),
                vec!["BR-04", "BH-102", "62.0", "64.0", "Norite (dissem.)"].into_iter().map(String::from).collect(),
                vec!["BR-05", "BH-102", "80.0", "82.0", "Gabbronorite"].into_iter().map(String::from).collect(),
                vec!["BR-06", "BH-103", "95.0", "97.0", "Pyroxenite"].into_iter().map(String::from).collect(),
                vec!["BR-07", "BH-103", "112.0", "114.0", "Net-textured Ore"].into_iter().map(String::from).collect(),
                vec!["BR-08", "BH-104", "130.0", "132.0", "Massive Sulfide"].into_iter().map(String::from).collect(),
            ],
        };
        let loi = PetroSourceSheet {
            name: "LOI".into(),
            enabled: true,
            method_code: "LOI".into(),
            columns: make_raw_cols(&["SampleID", "LOI_%"]),
            rows: vec![
                vec!["BR-01", "4.20"],
                vec!["BR-02", "2.10"],
                vec!["BR-03", "1.15"],
                vec!["BR-04", "0.90"],
                vec!["BR-05", "0.85"],
                vec!["BR-06", "1.40"],
                vec!["BR-07", "1.80"],
                vec!["BR-08", "2.60"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };
        let leco = PetroSourceSheet {
            name: "LECO".into(),
            enabled: true,
            method_code: "LECO".into(),
            columns: make_raw_cols(&["SampleID", "S_%", "C_%"]),
            rows: vec![
                vec!["BR-01", "0.28", "0.08"],
                vec!["BR-02", "0.62", "0.05"],
                vec!["BR-03", "1.15", "0.04"],
                vec!["BR-04", "2.40", "0.03"],
                vec!["BR-05", "0.45", "0.04"],
                vec!["BR-06", "1.85", "0.06"],
                vec!["BR-07", "8.60", "0.05"],
                vec!["BR-08", "24.50", "0.07"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };
        // Bromine-Methanol (BM) selective sulfide leach — key for Po/Py split in SF routine!
        let bm = PetroSourceSheet {
            name: "BM".into(),
            enabled: true,
            method_code: "BM".into(),
            columns: make_raw_cols(&["SampleID", "Fe_BM_%", "Ni_BM_ppm", "Cu_BM_ppm", "Co_BM_ppm", "Zn_BM_ppm", "Pb_BM_ppm"]),
            rows: vec![
                vec!["BR-01", "0.42", "950", "210", "55", "40", "8"],
                vec!["BR-02", "0.96", "1850", "480", "95", "55", "10"],
                vec!["BR-03", "1.78", "3100", "980", "140", "68", "12"],
                vec!["BR-04", "3.60", "6400", "2150", "270", "92", "18"],
                vec!["BR-05", "0.62", "820", "440", "45", "60", "15"],
                vec!["BR-06", "2.75", "4200", "1600", "210", "85", "14"],
                vec!["BR-07", "12.40", "21500", "7800", "860", "180", "35"],
                vec!["BR-08", "34.20", "62000", "22400", "2350", "390", "80"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };
        let xrf = PetroSourceSheet {
            name: "XRF".into(),
            enabled: true,
            method_code: "XRF".into(),
            columns: make_raw_cols(&[
                "SampleID", "SiO2_%", "TiO2_%", "Al2O3_%", "Fe2O3T_%", "MnO_%", "MgO_%",
                "CaO_%", "Na2O_%", "K2O_%", "P2O5_%", "Cr_ppm", "Ni_ppm", "Cu_ppm", "Zr_ppm", "Sr_ppm",
            ]),
            rows: vec![
                vec!["BR-01", "38.60", "0.18", "1.95", "12.80", "0.18", "40.20", "1.20", "0.12", "0.04", "0.02", "5400", "2850", "230", "14", "22"],
                vec!["BR-02", "44.20", "0.24", "4.10", "11.90", "0.17", "32.40", "3.15", "0.35", "0.09", "0.03", "4100", "2600", "510", "20", "55"],
                vec!["BR-03", "48.10", "0.35", "11.80", "10.60", "0.15", "18.90", "6.80", "1.15", "0.22", "0.05", "1850", "3450", "1020", "32", "195"],
                vec!["BR-04", "49.50", "0.48", "14.20", "12.40", "0.14", "11.60", "7.45", "1.65", "0.31", "0.07", "920", "6700", "2220", "45", "280"],
                vec!["BR-05", "50.80", "0.62", "15.90", "9.10", "0.15", "9.40", "10.60", "1.95", "0.38", "0.09", "540", "980", "460", "58", "340"],
                vec!["BR-06", "51.60", "0.44", "5.20", "13.10", "0.21", "21.80", "4.60", "0.42", "0.11", "0.04", "2950", "4500", "1680", "28", "82"],
                vec!["BR-07", "35.40", "0.38", "7.80", "26.20", "0.12", "12.40", "4.80", "0.85", "0.18", "0.05", "1100", "22100", "8050", "26", "140"],
                vec!["BR-08", "14.20", "0.22", "3.10", "54.80", "0.06", "4.80", "1.90", "0.30", "0.08", "0.02", "450", "63500", "23100", "12", "55"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };
        let pge = PetroSourceSheet {
            name: "PGE".into(),
            enabled: true,
            method_code: "PGE".into(),
            columns: make_raw_cols(&["SampleID", "Pt_ppb", "Pd_ppb", "Au_ppb", "Ir_ppb", "Rh_ppb", "Ru_ppb", "Se_ppm", "As_ppm"]),
            rows: vec![
                vec!["BR-01", "8", "10", "3", "1.8", "0.9", "3.2", "1.2", "1.5"],
                vec!["BR-02", "18", "24", "7", "2.4", "1.6", "4.5", "2.8", "2.0"],
                vec!["BR-03", "35", "48", "14", "3.5", "2.8", "6.2", "5.4", "3.5"],
                vec!["BR-04", "78", "112", "32", "6.0", "5.4", "11.0", "11.5", "6.0"],
                vec!["BR-05", "12", "15", "5", "0.8", "0.6", "1.4", "1.8", "1.2"],
                vec!["BR-06", "52", "68", "21", "4.8", "3.9", "8.4", "8.2", "4.0"],
                vec!["BR-07", "245", "380", "115", "18.5", "16.0", "32.0", "38.0", "18.0"],
                vec!["BR-08", "690", "1120", "310", "52.0", "46.0", "88.0", "108.0", "45.0"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };
        let ree = PetroSourceSheet {
            name: "REE".into(),
            enabled: true,
            method_code: "ICP-MS".into(),
            columns: make_raw_cols(&[
                "SampleID", "La_ppm", "Ce_ppm", "Pr_ppm", "Nd_ppm", "Sm_ppm", "Eu_ppm",
                "Gd_ppm", "Tb_ppm", "Dy_ppm", "Ho_ppm", "Er_ppm", "Tm_ppm", "Yb_ppm", "Lu_ppm",
                "Th_ppm", "Nb_ppm", "Y_ppm", "Rb_ppm", "Ba_ppm",
            ]),
            rows: vec![
                vec!["BR-01", "1.4", "3.2", "0.42", "1.9", "0.48", "0.16", "0.52", "0.09", "0.56", "0.12", "0.34", "0.05", "0.33", "0.05", "0.18", "0.6", "3.2", "1.5", "18"],
                vec!["BR-02", "2.1", "4.8", "0.64", "2.8", "0.70", "0.24", "0.76", "0.13", "0.80", "0.17", "0.48", "0.07", "0.46", "0.07", "0.28", "0.9", "4.8", "2.8", "32"],
                vec!["BR-03", "4.8", "10.6", "1.35", "5.8", "1.35", "0.54", "1.40", "0.23", "1.42", "0.30", "0.84", "0.12", "0.78", "0.12", "0.65", "1.8", "8.4", "6.2", "88"],
                vec!["BR-04", "6.4", "14.2", "1.80", "7.6", "1.75", "0.72", "1.82", "0.30", "1.85", "0.39", "1.10", "0.16", "1.02", "0.15", "0.92", "2.4", "11.0", "8.8", "125"],
                vec!["BR-05", "7.8", "17.4", "2.20", "9.4", "2.18", "0.88", "2.28", "0.38", "2.32", "0.48", "1.36", "0.20", "1.28", "0.19", "1.15", "3.1", "13.6", "11.2", "160"],
                vec!["BR-06", "3.2", "7.4", "0.98", "4.4", "1.12", "0.34", "1.22", "0.21", "1.32", "0.28", "0.80", "0.12", "0.76", "0.11", "0.42", "1.2", "7.5", "3.5", "48"],
                vec!["BR-07", "3.6", "8.0", "1.02", "4.5", "1.05", "0.42", "1.10", "0.18", "1.12", "0.24", "0.68", "0.10", "0.64", "0.10", "0.50", "1.4", "6.6", "4.8", "68"],
                vec!["BR-08", "1.8", "4.0", "0.52", "2.2", "0.52", "0.22", "0.55", "0.09", "0.56", "0.12", "0.34", "0.05", "0.32", "0.05", "0.24", "0.7", "3.2", "2.1", "30"],
            ]
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect(),
        };

        let mut enabled_routines = BTreeSet::new();
        for r in [
            PetroRoutine::Arrange,
            PetroRoutine::Vf,
            PetroRoutine::Sf,
            PetroRoutine::Vsf,
            PetroRoutine::Cipw,
            PetroRoutine::ReeN,
            PetroRoutine::Cumname,
            PetroRoutine::Trc,
            PetroRoutine::TraceN,
            PetroRoutine::Element,
            PetroRoutine::Oxide,
        ] {
            enabled_routines.insert(r);
        }

        Self {
            name: "Bruvann (HSC Tutorial)".into(),
            sheets: vec![header, loi, leco, bm, xrf, pge, ree],
            identified: false,
            group: PetroGroup::Rock,
            anion_oxygens: 6.0,
            fe2o3_feot_ratio: 0.15,
            enabled_routines,
            method_priorities: BTreeMap::new(),
            results: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Run `Identify` on all source sheets (§84.6.2), generating the 12 system rows and
    /// building default Method Priority lists per element.
    pub fn identify_all(&mut self) {
        self.log.clear();
        let known_els: BTreeSet<&str> = ELEMENTS.iter().map(|e| e.symbol).collect();
        let mut methods_by_el: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for sheet in &mut self.sheets {
            let mut id_count = 0usize;
            for col in &mut sheet.columns {
                let raw = col.header.trim();
                let parts: Vec<&str> = raw.split('_').collect();
                let token0 = parts.first().copied().unwrap_or("");
                let last = parts.last().copied().unwrap_or("");

                col.table = sheet.name.clone();
                col.method = sheet.method_code.clone();

                // Detect unit
                let (unit, mult) = match last.to_ascii_lowercase().as_str() {
                    "%" | "wt%" | "pct" => ("%".to_string(), 1.0),
                    "ppm" => ("ppm".to_string(), 1e-4),
                    "ppb" => ("ppb".to_string(), 1e-7),
                    "m" => ("m".to_string(), 1.0),
                    _ => (String::new(), 1.0),
                };

                // Detect method override in middle token (e.g., Fe_BM_%)
                if parts.len() >= 3 && !parts[1].is_empty() {
                    col.method = parts[1].to_ascii_uppercase();
                }

                let tl = token0.to_ascii_lowercase();
                if matches!(tl.as_str(), "sampleid" | "sample" | "id" | "sample_id") {
                    col.element = "SAMPLE".into();
                    col.field = "SampleID".into();
                    col.unit = String::new();
                    col.data_type = "Text".into();
                    col.length = 16;
                    col.digits = 0;
                    col.indexed = true;
                    col.multiplier = 1.0;
                    col.identified = true;
                    id_count += 1;
                } else if matches!(tl.as_str(), "loi" | "h2o" | "co2") {
                    col.element = token0.to_ascii_uppercase();
                    col.field = col.element.clone();
                    col.unit = if unit.is_empty() { "%".into() } else { unit };
                    col.data_type = "Number".into();
                    col.length = 8;
                    col.digits = 2;
                    col.multiplier = mult;
                    col.identified = true;
                    id_count += 1;
                } else if oxide_to_element_factor(token0).is_some() {
                    let (el, _) = oxide_to_element_factor(token0).unwrap();
                    col.element = el.to_string();
                    col.field = token0.to_string();
                    col.unit = if unit.is_empty() { "%".into() } else { unit };
                    col.data_type = "Number".into();
                    col.length = 8;
                    col.digits = 3;
                    col.multiplier = mult;
                    col.identified = true;
                    id_count += 1;
                    let list = methods_by_el.entry(el.to_string()).or_default();
                    if !list.contains(&col.method) {
                        list.push(col.method.clone());
                    }
                } else if known_els.contains(token0) {
                    let empty_unit = unit.is_empty();
                    col.element = token0.to_string();
                    col.field = token0.to_string();
                    col.unit = if empty_unit { "ppm".into() } else { unit };
                    col.data_type = "Number".into();
                    col.length = 8;
                    col.digits = if col.unit == "%" { 3 } else { 2 };
                    col.multiplier = if empty_unit { 1e-4 } else { mult };
                    col.identified = true;
                    id_count += 1;
                    let list = methods_by_el.entry(token0.to_string()).or_default();
                    if !list.contains(&col.method) {
                        list.push(col.method.clone());
                    }
                } else {
                    col.element = String::new();
                    col.field = raw.to_string();
                    col.unit = unit;
                    col.data_type = if col.unit == "m" { "Number".into() } else { "Text".into() };
                    col.length = 24;
                    col.digits = if col.unit == "m" { 2 } else { 0 };
                    col.multiplier = 1.0;
                    col.identified = matches!(tl.as_str(), "drillhole" | "from" | "to" | "lithology" | "rock" | "mineral");
                    if col.identified {
                        id_count += 1;
                    }
                }
            }
            self.log.push(format!(
                "Identify [{}]: {}/{} columns identified (method {}).",
                sheet.name,
                id_count,
                sheet.columns.len(),
                sheet.method_code
            ));
        }

        // Preserve user-reordered priorities while inserting newly discovered methods
        for (el, methods) in methods_by_el {
            let entry = self.method_priorities.entry(el.clone()).or_default();
            for m in &methods {
                if !entry.contains(m) {
                    entry.push(m.clone());
                }
            }
            // Prefer XRF for major Fe, LECO for S, BM for Ni/Cu/Co/Zn/Pb sulfide metals if freshly created
            if el == "Fe" && entry.contains(&"XRF".to_string()) {
                move_method_to_front(entry, "XRF");
            } else if el == "S" && entry.contains(&"LECO".to_string()) {
                move_method_to_front(entry, "LECO");
            } else if matches!(el.as_str(), "Ni" | "Cu" | "Co") && entry.contains(&"BM".to_string()) {
                move_method_to_front(entry, "BM");
            }
        }
        self.identified = true;
    }

    /// Move a method inside `method_priorities[el]` (`delta = -1` Up, `+1` Down, `i32::MIN` Top, `i32::MAX` Bottom).
    pub fn reorder_method(&mut self, element: &str, method_idx: usize, action: i32) {
        let Some(list) = self.method_priorities.get_mut(element) else {
            return;
        };
        if method_idx >= list.len() {
            return;
        }
        if action == i32::MIN {
            let m = list.remove(method_idx);
            list.insert(0, m);
        } else if action == i32::MAX {
            let m = list.remove(method_idx);
            list.push(m);
        } else if action < 0 && method_idx > 0 {
            list.swap(method_idx, method_idx - 1);
        } else if action > 0 && method_idx + 1 < list.len() {
            list.swap(method_idx, method_idx + 1);
        }
    }

    /// Import a TSV/CSV sheet into the Petrological workbook.
    pub fn import_tsv_sheet(&mut self, name: String, method_code: String, text: &str) -> Result<usize, String> {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.trim().is_empty())
            .collect();
        if lines.len() < 2 {
            return Err("Need a header line and at least one sample row.".into());
        }
        let delim = if lines[0].contains('\t') { '\t' } else { ',' };
        let headers: Vec<&str> = lines[0].split(delim).map(str::trim).collect();
        let rows: Vec<Vec<String>> = lines[1..]
            .iter()
            .map(|l| l.split(delim).map(|s| s.trim().to_string()).collect())
            .collect();
        let n_rows = rows.len();
        self.sheets.push(PetroSourceSheet {
            name,
            enabled: true,
            method_code,
            columns: make_raw_cols(&headers),
            rows,
        });
        self.identify_all();
        Ok(n_rows)
    }

    /// Execute all checked calculation routines (§84.6.4) and populate `self.results` and `self.log`.
    pub fn calculate(&mut self) {
        if !self.identified {
            self.identify_all();
        }
        self.results.clear();
        self.log.push(format!("--- Starting Petrological Calculation ({:?}) ---", self.group));

        // Step 1: Merge all active sheets by SampleID into structured sample records
        let samples = self.merge_samples();
        self.log.push(format!(
            "Merged {} unique samples across {} enabled sheets.",
            samples.len(),
            self.sheets.iter().filter(|s| s.enabled).count()
        ));
        if samples.is_empty() {
            self.log.push("WARNING: No samples found with SampleID.".into());
            return;
        }

        // Compute core derived quantities once per sample so routines can cross-use SF / VSF / CIPW
        let sf_rows: Vec<SfSampleResult> = samples.iter().map(compute_sf_sample).collect();
        let vsf_rows: Vec<BTreeMap<String, f64>> = samples
            .iter()
            .zip(sf_rows.iter())
            .map(|(s, sf)| compute_vsf_oxides(s, sf))
            .collect();
        let cipw_rows: Vec<CipwResult> = vsf_rows
            .iter()
            .map(|ox| compute_cipw_norm(ox, self.fe2o3_feot_ratio))
            .collect();

        for &routine in &PetroRoutine::ALL {
            if !self.enabled_routines.contains(&routine) {
                continue;
            }
            let sheet = match routine {
                PetroRoutine::Arrange => build_arrange_sheet(&samples),
                PetroRoutine::Anion => build_anion_sheet(&samples, self.anion_oxygens),
                PetroRoutine::Cation => build_cation_sheet(&samples),
                PetroRoutine::Vf => build_vf_sheet(&samples),
                PetroRoutine::Sf => build_sf_sheet(&samples, &sf_rows),
                PetroRoutine::Vsf => build_vsf_sheet(&samples, &vsf_rows),
                PetroRoutine::Cipw => build_cipw_sheet(&samples, &cipw_rows),
                PetroRoutine::ReeN => build_ree_norm_sheet(&samples),
                PetroRoutine::Cumname => build_cumname_sheet(&samples, &cipw_rows, &sf_rows),
                PetroRoutine::Trc => build_trc_sheet(&samples),
                PetroRoutine::TraceN => build_trace_norm_sheet(&samples),
                PetroRoutine::Element => build_element_sheet(&samples),
                PetroRoutine::Oxide => build_oxide_sheet(&samples),
            };
            self.log.push(format!(
                "Completed {}: {} samples x {} columns.",
                routine.code(),
                sheet.rows.len(),
                sheet.headers.len()
            ));
            self.results.push(sheet);
        }
    }

    fn merge_samples(&self) -> Vec<MergedPetroSample> {
        let mut order: Vec<String> = Vec::new();
        let mut map: BTreeMap<String, MergedPetroSample> = BTreeMap::new();

        for sheet in &self.sheets {
            if !sheet.enabled {
                continue;
            }
            let Some(id_col) = sheet
                .columns
                .iter()
                .position(|c| c.element == "SAMPLE" || c.field.eq_ignore_ascii_case("SampleID"))
            else {
                continue;
            };

            for row in &sheet.rows {
                let sid = row.get(id_col).map(|s| s.trim()).unwrap_or("");
                if sid.is_empty() {
                    continue;
                }
                if !map.contains_key(sid) {
                    order.push(sid.to_string());
                    map.insert(sid.to_string(), MergedPetroSample::new(sid.to_string()));
                }
                let sample = map.get_mut(sid).unwrap();

                for (ci, col) in sheet.columns.iter().enumerate() {
                    if ci == id_col {
                        continue;
                    }
                    let raw_val = row.get(ci).map(|s| s.trim()).unwrap_or("");
                    if raw_val.is_empty() {
                        continue;
                    }
                    if col.data_type == "Text" {
                        sample.meta.insert(col.field.clone(), raw_val.to_string());
                        continue;
                    }
                    let Ok(num) = raw_val.replace(',', ".").parse::<f64>() else {
                        continue;
                    };
                    if col.element == "LOI" {
                        sample.loi_pct = Some(num * col.multiplier);
                        continue;
                    }
                    // Keep BM-specific sulfide leach values separately for exact SF Po/Py solver
                    if col.method == "BM" {
                        sample
                            .bm_wt_pct
                            .insert(col.element.clone(), num * col.multiplier);
                    }
                    // Store oxide value directly if oxide column
                    if oxide_to_element_factor(&col.field).is_some() {
                        let ox_key = if col.field == "Fe2O3T" {
                            "Fe2O3"
                        } else {
                            col.field.as_str()
                        };
                        sample
                            .oxides_by_method
                            .entry(ox_key.to_string())
                            .or_default()
                            .insert(col.method.clone(), num * col.multiplier);
                        if let Some((el, fac)) = oxide_to_element_factor(&col.field) {
                            sample
                                .elements_by_method
                                .entry(el.to_string())
                                .or_default()
                                .insert(col.method.clone(), num * col.multiplier * fac);
                        }
                    } else if !col.element.is_empty() {
                        sample
                            .elements_by_method
                            .entry(col.element.clone())
                            .or_default()
                            .insert(col.method.clone(), num * col.multiplier);
                    }
                }
            }
        }

        // Resolve final preferred value per element & oxide according to `method_priorities`
        let mut out = Vec::with_capacity(order.len());
        for sid in order {
            let mut s = map.remove(&sid).unwrap();
            let all_els: Vec<String> = s.elements_by_method.keys().cloned().collect();
            for el in all_els {
                if let Some(m_map) = s.elements_by_method.get(&el) {
                    let chosen = pick_by_priority(m_map, self.method_priorities.get(&el));
                    if let Some((val, method)) = chosen {
                        s.el_wt_pct.insert(el.clone(), val);
                        s.el_source.insert(el, method);
                    }
                }
            }
            let all_ox: Vec<String> = s.oxides_by_method.keys().cloned().collect();
            for ox in all_ox {
                if let Some(m_map) = s.oxides_by_method.get(&ox) {
                    let el_key = oxide_to_element_factor(&ox).map(|(e, _)| e.to_string());
                    let prio = el_key.as_ref().and_then(|e| self.method_priorities.get(e));
                    if let Some((val, _)) = pick_by_priority(m_map, prio) {
                        s.oxides_wt_pct.insert(ox, val);
                    }
                }
            }
            out.push(s);
        }
        out
    }

    /// Export all Petrological result sheets to multi-sheet Excel `.xlsx` (§84.6.5).
    pub fn export_results_xlsx(&self) -> Result<Vec<u8>, String> {
        let mut sheets: Vec<ExportSheet> = Vec::new();
        for res in &self.results {
            let mut rows = Vec::with_capacity(res.rows.len() + 1);
            rows.push(res.headers.clone());
            rows.extend(res.rows.clone());
            sheets.push(make_export_sheet(res.routine.code().to_string(), &rows));
        }
        if sheets.is_empty() {
            return Err("No calculated results to export. Press Calculate first.".into());
        }
        Ok(build_xlsx(&sheets))
    }
}

fn make_raw_cols(headers: &[&str]) -> Vec<IdentifiedColumnMeta> {
    headers
        .iter()
        .map(|h| IdentifiedColumnMeta {
            header: (*h).to_string(),
            element: String::new(),
            method: String::new(),
            unit: String::new(),
            table: String::new(),
            field: (*h).to_string(),
            data_type: "Text".into(),
            length: 12,
            digits: 2,
            indexed: false,
            multiplier: 1.0,
            identified: false,
        })
        .collect()
}

fn move_method_to_front(list: &mut Vec<String>, m: &str) {
    if let Some(pos) = list.iter().position(|x| x == m) {
        let val = list.remove(pos);
        list.insert(0, val);
    }
}

fn pick_by_priority(
    m_map: &BTreeMap<String, f64>,
    prio: Option<&Vec<String>>,
) -> Option<(f64, String)> {
    if let Some(order) = prio {
        for m in order {
            if let Some(&v) = m_map.get(m) {
                return Some((v, m.clone()));
            }
        }
    }
    m_map.iter().next().map(|(k, &v)| (v, k.clone()))
}

#[derive(Debug, Clone)]
struct MergedPetroSample {
    sample_id: String,
    meta: BTreeMap<String, String>,
    loi_pct: Option<f64>,
    oxides_by_method: BTreeMap<String, BTreeMap<String, f64>>,
    elements_by_method: BTreeMap<String, BTreeMap<String, f64>>,
    bm_wt_pct: BTreeMap<String, f64>,
    oxides_wt_pct: BTreeMap<String, f64>,
    el_wt_pct: BTreeMap<String, f64>,
    el_source: BTreeMap<String, String>,
}

impl MergedPetroSample {
    fn new(sample_id: String) -> Self {
        Self {
            sample_id,
            meta: BTreeMap::new(),
            loi_pct: None,
            oxides_by_method: BTreeMap::new(),
            elements_by_method: BTreeMap::new(),
            bm_wt_pct: BTreeMap::new(),
            oxides_wt_pct: BTreeMap::new(),
            el_wt_pct: BTreeMap::new(),
            el_source: BTreeMap::new(),
        }
    }

    fn ox(&self, k: &str) -> f64 {
        self.oxides_wt_pct.get(k).copied().unwrap_or(0.0)
    }

    fn el(&self, k: &str) -> f64 {
        self.el_wt_pct.get(k).copied().unwrap_or(0.0)
    }

    fn el_ppm(&self, k: &str) -> f64 {
        self.el(k) * 1e4
    }

    fn el_ppb(&self, k: &str) -> f64 {
        self.el(k) * 1e7
    }
}

#[derive(Debug, Clone)]
struct SfSampleResult {
    ccp: f64,
    pn: f64,
    cpn: f64,
    sp: f64,
    ga: f64,
    mb: f64,
    apy: f64,
    po: f64,
    py: f64,
    total_sf: f64,
    fe_in_sulfides: f64,
    s_in_sulfides: f64,
    ni_100s: f64,
    cu_100s: f64,
    co_100s: f64,
    pt_100s_ppb: f64,
    pd_100s_ppb: f64,
    au_100s_ppb: f64,
}

/// Exact §84.6.4 p. 32 Sulfide Fraction (SF) calculation.
fn compute_sf_sample(s: &MergedPetroSample) -> SfSampleResult {
    let s_tot = s.el("S");
    let cu = s.bm_wt_pct.get("Cu").copied().unwrap_or_else(|| s.el("Cu"));
    let ni = s.bm_wt_pct.get("Ni").copied().unwrap_or_else(|| s.el("Ni"));
    let co = s.bm_wt_pct.get("Co").copied().unwrap_or_else(|| s.el("Co"));
    let zn = s.bm_wt_pct.get("Zn").copied().unwrap_or_else(|| s.el("Zn"));
    let pb = s.bm_wt_pct.get("Pb").copied().unwrap_or_else(|| s.el("Pb"));
    let mo = s.bm_wt_pct.get("Mo").copied().unwrap_or_else(|| s.el("Mo"));
    let as_v = s.bm_wt_pct.get("As").copied().unwrap_or_else(|| s.el("As"));

    let mut ccp = 100.0 * cu / 34.63;
    let mut pn = 100.0 * ni / 36.16;
    let mut cpn = 100.0 * co / 67.40;
    let mut sp = 100.0 * zn / 67.10;
    let mut ga = 100.0 * pb / 86.60;
    let mut mb = 100.0 * mo / 59.94;
    let mut apy = 100.0 * as_v / 46.01;

    // Sulfur & Fe consumed by the 7 base sulfides (stoichiometric wt% in each mineral)
    let s_used = ccp * 0.3494
        + pn * 0.3312
        + cpn * 0.3260
        + sp * 0.3290
        + ga * 0.1340
        + mb * 0.4006
        + apy * 0.1969;
    let fe_used = ccp * 0.3043 + pn * 0.3072 + apy * 0.3430;

    let s_res = s_tot - s_used;
    let (po, py) = if s_tot <= 1e-6 {
        ccp = 0.0;
        pn = 0.0;
        cpn = 0.0;
        sp = 0.0;
        ga = 0.0;
        mb = 0.0;
        apy = 0.0;
        (0.0, 0.0)
    } else if s_res < 0.0 {
        // Per §84.6.4 p. 32: if residual S < 0, zero other sulfides and put all S in Po
        ccp = 0.0;
        pn = 0.0;
        cpn = 0.0;
        sp = 0.0;
        ga = 0.0;
        mb = 0.0;
        apy = 0.0;
        (100.0 * s_tot / 36.48, 0.0)
    } else if let Some(&fe_bm) = s.bm_wt_pct.get("Fe") {
        let fe_res = (fe_bm - fe_used).max(0.0);
        // 2x2 NNLS: Po (36.48% S, 63.52% Fe_BM) & Py (53.45% S, 0.07*46.55% Fe_BM soluble!)
        let a = vec![vec![0.3648, 0.5345], vec![0.6352, 0.07 * 0.4655]];
        let b = vec![s_res, fe_res];
        let w = vec![1.0, 1.0];
        let sol = solve_wnnls(&a, &b, &w);
        (sol[0], sol[1])
    } else {
        (100.0 * s_res / 36.48, 0.0)
    };

    // Trace metals included in sulfide fraction
    let trace_sum: f64 = [
        "Os", "Ir", "Ru", "Rh", "Pt", "Pd", "Au", "Re", "Ag", "Cd", "Sn", "Sb", "Bi", "Se", "Te",
        "Hg",
    ]
    .iter()
    .map(|e| s.el(e))
    .sum();

    let total_sf = ccp + pn + cpn + sp + ga + mb + apy + po + py + trace_sum;
    let fe_in_sulfides =
        ccp * 0.3043 + pn * 0.3072 + apy * 0.3430 + po * 0.6352 + py * 0.4655;
    let s_in_sulfides =
        ccp * 0.3494 + pn * 0.3312 + cpn * 0.3260 + sp * 0.3290 + ga * 0.1340 + mb * 0.4006
            + apy * 0.1969
            + po * 0.3648
            + py * 0.5345;

    let scale = if total_sf > 1e-6 { 100.0 / total_sf } else { 0.0 };
    SfSampleResult {
        ccp,
        pn,
        cpn,
        sp,
        ga,
        mb,
        apy,
        po,
        py,
        total_sf,
        fe_in_sulfides,
        s_in_sulfides,
        ni_100s: ni * scale,
        cu_100s: cu * scale,
        co_100s: co * scale,
        pt_100s_ppb: s.el_ppb("Pt") * scale,
        pd_100s_ppb: s.el_ppb("Pd") * scale,
        au_100s_ppb: s.el_ppb("Au") * scale,
    }
}

const MAJOR_OXIDES: &[&str] = &[
    "SiO2", "TiO2", "Al2O3", "Fe2O3", "MnO", "MgO", "CaO", "Na2O", "K2O", "P2O5", "Cr2O3",
];

fn compute_vsf_oxides(s: &MergedPetroSample, sf: &SfSampleResult) -> BTreeMap<String, f64> {
    let mut raw = BTreeMap::new();
    for &ox in MAJOR_OXIDES {
        let mut v = s.ox(ox);
        if ox == "Cr2O3" && v == 0.0 {
            v = s.el("Cr") * (151.9904 / (2.0 * 51.9961));
        }
        if ox == "Fe2O3" {
            // Subtract sulfide-hosted Fe (converted to Fe2O3 equivalent)
            let fe2o3_sulf = sf.fe_in_sulfides * (159.6882 / (2.0 * 55.845));
            v = (v - fe2o3_sulf).max(0.0);
        }
        raw.insert(ox.to_string(), v);
    }
    let tot: f64 = raw.values().sum();
    if tot > 1e-6 {
        for v in raw.values_mut() {
            *v = *v * 100.0 / tot;
        }
    }
    raw
}

#[derive(Debug, Clone, Default)]
struct CipwResult {
    q: f64,
    or: f64,
    ab: f64,
    an: f64,
    ne: f64,
    cor: f64,
    di: f64,
    hy: f64,
    ol: f64,
    mt: f64,
    il: f64,
    ap: f64,
    cm: f64,
    mg_num: f64,
    an_pct: f64,
    total: f64,
}

/// Standard CIPW Norm calculation from volatile- and sulfide-free major oxides.
fn compute_cipw_norm(ox: &BTreeMap<String, f64>, fe3_frac: f64) -> CipwResult {
    let g = |k: &str| ox.get(k).copied().unwrap_or(0.0);
    let fe2o3_t = g("Fe2O3");
    let fe_tot_mol = (fe2o3_t / 159.6882) * 2.0;
    let fe3_mol = fe_tot_mol * fe3_frac.clamp(0.05, 0.5);
    let fe2_mol = fe_tot_mol - fe3_mol;

    let mut si = g("SiO2") / 60.0843;
    let mut ti = g("TiO2") / 79.8658;
    let mut al = g("Al2O3") / 101.9613;
    let mut fe3 = fe3_mol * 0.5; // mol Fe2O3
    let mut fe2 = fe2_mol + g("MnO") / 70.9374; // mol FeO + MnO
    let mut mg = g("MgO") / 40.3044;
    let mut ca = g("CaO") / 56.0774;
    let mut na = g("Na2O") / 61.9789;
    let mut k = g("K2O") / 94.196;
    let p = g("P2O5") / 141.9445;
    let cr = g("Cr2O3") / 151.9904;

    let mg_num = if mg + fe2 > 1e-9 {
        100.0 * mg / (mg + fe2)
    } else {
        0.0
    };

    // 1. Apatite: 3.33 CaO . P2O5
    let ap_m = p.min(ca / 3.33);
    ca = (ca - 3.33 * ap_m).max(0.0);

    // 2. Chromite: FeO . Cr2O3
    let cm_m = cr.min(fe2);
    fe2 = (fe2 - cm_m).max(0.0);

    // 3. Ilmenite: FeO . TiO2
    let il_m = ti.min(fe2);
    ti -= il_m;
    fe2 = (fe2 - il_m).max(0.0);
    let _ = ti;

    // 4. Orthoclase: K2O . Al2O3 . 6 SiO2
    let or_m = k.min(al);
    k -= or_m;
    al -= or_m;
    let _ = k;

    // 5. Albite: Na2O . Al2O3 . 6 SiO2
    let mut ab_m = na.min(al);
    na -= ab_m;
    al -= ab_m;
    let _ = na;

    // 6. Anorthite: CaO . Al2O3 . 2 SiO2
    let an_m = al.min(ca);
    al -= an_m;
    ca -= an_m;

    // 7. Corundum: excess Al2O3
    let cor_m = al.max(0.0);

    // 8. Magnetite: FeO . Fe2O3
    let mt_m = fe3.min(fe2);
    fe3 -= mt_m;
    fe2 -= mt_m;
    let _ = fe3;

    // 9. Diopside: CaO . (Mg,Fe)O . 2 SiO2
    let fm_tot = mg + fe2;
    let mg_ratio = if fm_tot > 1e-9 { mg / fm_tot } else { 0.5 };
    let di_m = ca.min(fm_tot);
    ca -= di_m;
    let _ = ca;
    mg = (mg - di_m * mg_ratio).max(0.0);
    fe2 = (fe2 - di_m * (1.0 - mg_ratio)).max(0.0);

    // 10. Provisional Hypersthene: (Mg,Fe)O . SiO2
    let mut hy_m = mg + fe2;

    // 11. Silica balance
    let si_needed = 6.0 * or_m + 6.0 * ab_m + 2.0 * an_m + 2.0 * di_m + hy_m;
    let mut q_m = 0.0;
    let mut ol_m = 0.0;
    let mut ne_m = 0.0;

    if si >= si_needed {
        q_m = si - si_needed;
    } else {
        let mut def = si_needed - si;
        // Convert Hy -> 0.5 Ol (saves 0.5 SiO2 per mol Hy)
        let hy_to_ol = (2.0 * def).min(hy_m);
        hy_m -= hy_to_ol;
        ol_m = 0.5 * hy_to_ol;
        def -= 0.5 * hy_to_ol;
        // Convert Ab -> Ne (saves 4 SiO2 per mol Ab)
        if def > 1e-9 && ab_m > 0.0 {
            let ab_to_ne = (def / 4.0).min(ab_m);
            ab_m -= ab_to_ne;
            ne_m = ab_to_ne;
            def -= 4.0 * ab_to_ne;
        }
        let _ = def;
    }
    si -= si_needed;
    let _ = si;

    // Convert moles to wt% using molecular weights
    let mw_di = 56.0774 + mg_ratio * 40.3044 + (1.0 - mg_ratio) * 71.8444 + 2.0 * 60.0843;
    let mw_hy = mg_ratio * 40.3044 + (1.0 - mg_ratio) * 71.8444 + 60.0843;
    let mw_ol = 2.0 * (mg_ratio * 40.3044 + (1.0 - mg_ratio) * 71.8444) + 60.0843;

    let q = q_m * 60.0843;
    let or = or_m * 556.663; // 2 * KAlSi3O8
    let ab = ab_m * 524.446; // 2 * NaAlSi3O8
    let an = an_m * 278.207;
    let ne = ne_m * 284.109;
    let cor = cor_m * 101.9613;
    let di = di_m * mw_di;
    let hy = hy_m * mw_hy;
    let ol = ol_m * mw_ol;
    let mt = mt_m * 231.533;
    let il = il_m * 151.710;
    let ap = ap_m * 328.86;
    let cm = cm_m * 223.835;

    let total = q + or + ab + an + ne + cor + di + hy + ol + mt + il + ap + cm;
    let an_pct = if an + ab > 1e-6 { 100.0 * an / (an + ab) } else { 0.0 };

    CipwResult {
        q,
        or,
        ab,
        an,
        ne,
        cor,
        di,
        hy,
        ol,
        mt,
        il,
        ap,
        cm,
        mg_num,
        an_pct,
        total,
    }
}

/// Exact §84.6.4 p. 33 4-part Cumulus Naming (`o/b/a/p/m/c/t/$` + `A/M/O` + `C/R` + intercumulus).
pub fn compute_cumulus_name(
    ol: f64,
    opx: f64,
    cpx: f64,
    plag: f64,
    mt: f64,
    cm: f64,
    il: f64,
    sulf: f64,
) -> String {
    let candidates = [
        ("o", ol, 10.0),
        ("b", opx, 25.0),
        ("a", cpx, 25.0),
        ("p", plag, 25.0),
        ("m", mt, 10.0),
        ("c", cm, 0.6),
        ("t", il, 1.0),
        ("$", sulf, 0.3),
    ];
    let mut cum: Vec<(&str, f64)> = candidates
        .iter()
        .filter(|&&(_, v, thr)| v > thr)
        .map(|&(sym, v, _)| (sym, v))
        .collect();
    cum.sort_by(|a, b| b.1.total_cmp(&a.1));

    let part1: String = cum.iter().map(|(s, _)| *s).collect();
    let cum_sum: f64 = cum.iter().map(|(_, v)| *v).sum();

    let part2 = if cum_sum > 93.0 {
        "A"
    } else if cum_sum > 75.0 {
        "M"
    } else if cum_sum > 50.0 {
        "O"
    } else {
        ""
    };
    let part3 = if cum_sum > 50.0 { "C" } else { "R" };

    let mut inter: Vec<(&str, f64)> = [("b", opx), ("a", cpx), ("p", plag)]
        .into_iter()
        .filter(|&(_, v)| v >= 10.0 && v <= 25.0)
        .collect();
    inter.sort_by(|a, b| b.1.total_cmp(&a.1));
    let part4: String = inter.iter().map(|(s, _)| *s).collect();

    if part4.is_empty() {
        format!("{part1}{part2}{part3}")
    } else {
        format!("{part1}{part2}{part3}-{part4}")
    }
}

fn build_arrange_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let headers: Vec<String> = vec![
        "SampleID", "Lithology", "SiO2 %", "TiO2 %", "Al2O3 %", "Fe2O3 %", "MnO %", "MgO %",
        "CaO %", "Na2O %", "K2O %", "P2O5 %", "LOI %", "S %", "Ni ppm", "Cu ppm", "Co ppm",
        "Cr ppm", "Pt ppb", "Pd ppb", "Au ppb",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let rows = samples
        .iter()
        .map(|s| {
            vec![
                s.sample_id.clone(),
                s.meta.get("Lithology").cloned().unwrap_or_default(),
                fmt2(s.ox("SiO2")),
                fmt2(s.ox("TiO2")),
                fmt2(s.ox("Al2O3")),
                fmt2(s.ox("Fe2O3")),
                fmt2(s.ox("MnO")),
                fmt2(s.ox("MgO")),
                fmt2(s.ox("CaO")),
                fmt2(s.ox("Na2O")),
                fmt2(s.ox("K2O")),
                fmt2(s.ox("P2O5")),
                fmt2(s.loi_pct.unwrap_or(0.0)),
                fmt2(s.el("S")),
                fmt1(s.el_ppm("Ni")),
                fmt1(s.el_ppm("Cu")),
                fmt1(s.el_ppm("Co")),
                fmt1(s.el_ppm("Cr")),
                fmt1(s.el_ppb("Pt")),
                fmt1(s.el_ppb("Pd")),
                fmt1(s.el_ppb("Au")),
            ]
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Arrange,
        title: "ARRANGE — Priority-Merged Master Geochemistry".into(),
        headers,
        rows,
    }
}

fn build_vf_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(MAJOR_OXIDES.iter().map(|o| format!("{o}_VF %")));
    headers.push("Total %".into());
    let rows = samples
        .iter()
        .map(|s| {
            let vals: Vec<f64> = MAJOR_OXIDES
                .iter()
                .map(|&o| {
                    if o == "Cr2O3" && s.ox(o) == 0.0 {
                        s.el("Cr") * (151.9904 / (2.0 * 51.9961))
                    } else {
                        s.ox(o)
                    }
                })
                .collect();
            let tot: f64 = vals.iter().sum();
            let scale = if tot > 1e-6 { 100.0 / tot } else { 0.0 };
            let mut r = vec![s.sample_id.clone()];
            for v in vals {
                r.push(fmt2(v * scale));
            }
            r.push(fmt2(if tot > 1e-6 { 100.0 } else { 0.0 }));
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Vf,
        title: "VF — Volatile-Free Silicate Composition (100%)".into(),
        headers,
        rows,
    }
}

fn build_sf_sheet(samples: &[MergedPetroSample], sf_rows: &[SfSampleResult]) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "Ccp %", "Pn %", "Cpn %", "Sp %", "Ga %", "Mb %", "Apy %", "Po %", "Py %",
        "Total SF %", "S_sulf %", "Ni_100S %", "Cu_100S %", "Co_100S %", "Pt_100S ppb", "Pd_100S ppb",
        "Au_100S ppb",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let rows = samples
        .iter()
        .zip(sf_rows.iter())
        .map(|(s, sf)| {
            vec![
                s.sample_id.clone(),
                fmt3(sf.ccp),
                fmt3(sf.pn),
                fmt3(sf.cpn),
                fmt3(sf.sp),
                fmt3(sf.ga),
                fmt3(sf.mb),
                fmt3(sf.apy),
                fmt3(sf.po),
                fmt3(sf.py),
                fmt2(sf.total_sf),
                fmt2(sf.s_in_sulfides),
                fmt2(sf.ni_100s),
                fmt2(sf.cu_100s),
                fmt2(sf.co_100s),
                fmt1(sf.pt_100s_ppb),
                fmt1(sf.pd_100s_ppb),
                fmt1(sf.au_100s_ppb),
            ]
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Sf,
        title: "SF — Sulfide Fraction & 100% Sulfide Tenors (§84.6.4)".into(),
        headers,
        rows,
    }
}

fn build_vsf_sheet(samples: &[MergedPetroSample], vsf_rows: &[BTreeMap<String, f64>]) -> PetroResultSheet {
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(MAJOR_OXIDES.iter().map(|o| format!("{o}_VSF %")));
    headers.push("Total %".into());
    let rows = samples
        .iter()
        .zip(vsf_rows.iter())
        .map(|(s, m)| {
            let mut r = vec![s.sample_id.clone()];
            let mut tot = 0.0;
            for &o in MAJOR_OXIDES {
                let v = m.get(o).copied().unwrap_or(0.0);
                tot += v;
                r.push(fmt2(v));
            }
            r.push(fmt2(tot));
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Vsf,
        title: "VSF — Volatile- & Sulfide-Free Silicate Composition".into(),
        headers,
        rows,
    }
}

fn build_cipw_sheet(samples: &[MergedPetroSample], cipw: &[CipwResult]) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "Q %", "Or %", "Ab %", "An %", "Ne %", "C %", "Di %", "Hy %", "Ol %",
        "Mt %", "Il %", "Ap %", "Cm %", "Mg#", "An%", "Total %",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let rows = samples
        .iter()
        .zip(cipw.iter())
        .map(|(s, c)| {
            vec![
                s.sample_id.clone(),
                fmt2(c.q),
                fmt2(c.or),
                fmt2(c.ab),
                fmt2(c.an),
                fmt2(c.ne),
                fmt2(c.cor),
                fmt2(c.di),
                fmt2(c.hy),
                fmt2(c.ol),
                fmt2(c.mt),
                fmt2(c.il),
                fmt2(c.ap),
                fmt2(c.cm),
                fmt1(c.mg_num),
                fmt1(c.an_pct),
                fmt2(c.total),
            ]
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Cipw,
        title: "CIPW — Normative Mineralogy".into(),
        headers,
        rows,
    }
}

/// CI Chondrite values (ppm, McDonough & Sun 1995).
const CI_CHONDRITE_REE: &[(&str, f64)] = &[
    ("La", 0.237), ("Ce", 0.613), ("Pr", 0.0928), ("Nd", 0.457), ("Sm", 0.148),
    ("Eu", 0.0563), ("Gd", 0.199), ("Tb", 0.0361), ("Dy", 0.246), ("Ho", 0.0546),
    ("Er", 0.160), ("Tm", 0.0247), ("Yb", 0.161), ("Lu", 0.0246),
];

fn build_ree_norm_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(CI_CHONDRITE_REE.iter().map(|(e, _)| format!("{e}_N")));
    headers.extend(["Eu/Eu*".into(), "(La/Yb)N".into(), "(La/Sm)N".into()]);
    let rows = samples
        .iter()
        .map(|s| {
            let mut r = vec![s.sample_id.clone()];
            let mut norm = BTreeMap::new();
            for &(el, ci) in CI_CHONDRITE_REE {
                let v = s.el_ppm(el) / ci;
                norm.insert(el, v);
                r.push(fmt2(v));
            }
            let sm = norm["Sm"];
            let gd = norm["Gd"];
            let eu_star = if sm * gd > 1e-12 { norm["Eu"] / (sm * gd).sqrt() } else { 0.0 };
            let la_yb = if norm["Yb"] > 1e-9 { norm["La"] / norm["Yb"] } else { 0.0 };
            let la_sm = if sm > 1e-9 { norm["La"] / sm } else { 0.0 };
            r.push(fmt2(eu_star));
            r.push(fmt2(la_yb));
            r.push(fmt2(la_sm));
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::ReeN,
        title: "REE_N — Chondrite-Normalized REE (McDonough & Sun 1995)".into(),
        headers,
        rows,
    }
}

fn build_cumname_sheet(
    samples: &[MergedPetroSample],
    cipw: &[CipwResult],
    sf_rows: &[SfSampleResult],
) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "Lithology", "Ol %", "Opx %", "Cpx %", "Plag %", "Mt %", "Cm %", "Il %",
        "Sulf %", "CUMNAME",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let rows = samples
        .iter()
        .zip(cipw.iter())
        .zip(sf_rows.iter())
        .map(|((s, c), sf)| {
            let plag = c.ab + c.an;
            let name = compute_cumulus_name(c.ol, c.hy, c.di, plag, c.mt, c.cm, c.il, sf.total_sf);
            vec![
                s.sample_id.clone(),
                s.meta.get("Lithology").cloned().unwrap_or_default(),
                fmt1(c.ol),
                fmt1(c.hy),
                fmt1(c.di),
                fmt1(plag),
                fmt1(c.mt),
                fmt2(c.cm),
                fmt2(c.il),
                fmt2(sf.total_sf),
                name,
            ]
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Cumname,
        title: "CUMNAME — 4-Part Cumulus Rock Naming (§84.6.4)".into(),
        headers,
        rows,
    }
}

fn build_trc_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "Cu/Ni", "Cu/Pd (x10^3)", "Pd/Ir", "Pt/Pd", "S/Se", "Ni/Cr", "Zr/Y", "Th/Nb",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let ratio = |a: f64, b: f64| if b > 1e-9 { a / b } else { 0.0 };
    let rows = samples
        .iter()
        .map(|s| {
            vec![
                s.sample_id.clone(),
                fmt3(ratio(s.el_ppm("Cu"), s.el_ppm("Ni"))),
                fmt2(ratio(s.el_ppm("Cu"), s.el_ppb("Pd")) * 1.0), // ppm / ppb = x10^3!
                fmt2(ratio(s.el_ppb("Pd"), s.el_ppb("Ir"))),
                fmt2(ratio(s.el_ppb("Pt"), s.el_ppb("Pd"))),
                fmt1(ratio(s.el_ppm("S"), s.el_ppm("Se"))),
                fmt2(ratio(s.el_ppm("Ni"), s.el_ppm("Cr"))),
                fmt2(ratio(s.el_ppm("Zr"), s.el_ppm("Y"))),
                fmt3(ratio(s.el_ppm("Th"), s.el_ppm("Nb"))),
            ]
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Trc,
        title: "TRC — Magmatic & Trace-Element Geochemical Ratios".into(),
        headers,
        rows,
    }
}

/// Primitive Mantle normalization constants (ppm, McDonough & Sun 1995).
const PM_TRACE: &[(&str, f64)] = &[
    ("Rb", 0.60), ("Ba", 6.60), ("Th", 0.0795), ("Nb", 0.658), ("La", 0.648), ("Ce", 1.675),
    ("Sr", 19.9), ("Nd", 1.250), ("Zr", 10.5), ("Sm", 0.406), ("Eu", 0.154), ("Ti", 1205.0),
    ("Gd", 0.544), ("Dy", 0.674), ("Y", 4.30), ("Yb", 0.441), ("Lu", 0.0675),
];

fn build_trace_norm_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(PM_TRACE.iter().map(|(e, _)| format!("{e}_PM")));
    let rows = samples
        .iter()
        .map(|s| {
            let mut r = vec![s.sample_id.clone()];
            for &(el, pm) in PM_TRACE {
                r.push(fmt2(s.el_ppm(el) / pm));
            }
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::TraceN,
        title: "TRACE_N — Primitive Mantle-Normalized Multi-Element Table".into(),
        headers,
        rows,
    }
}

fn build_anion_sheet(samples: &[MergedPetroSample], target_o: f64) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "O_basis", "Si_apfu", "Ti_apfu", "Al_apfu", "Fe_apfu", "Mn_apfu", "Mg_apfu",
        "Ca_apfu", "Na_apfu", "K_apfu", "Sum_Cat",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let specs = [
        ("SiO2", 60.0843, 1.0, 2.0),
        ("TiO2", 79.8658, 1.0, 2.0),
        ("Al2O3", 101.9613, 2.0, 3.0),
        ("Fe2O3", 159.6882, 2.0, 3.0),
        ("MnO", 70.9374, 1.0, 1.0),
        ("MgO", 40.3044, 1.0, 1.0),
        ("CaO", 56.0774, 1.0, 1.0),
        ("Na2O", 61.9789, 2.0, 1.0),
        ("K2O", 94.196, 2.0, 1.0),
    ];
    let rows = samples
        .iter()
        .map(|s| {
            let o_tot: f64 = specs.iter().map(|(ox, mw, _, o_n)| (s.ox(ox) / mw) * o_n).sum();
            let fac = if o_tot > 1e-9 { target_o / o_tot } else { 0.0 };
            let mut r = vec![s.sample_id.clone(), fmt1(target_o)];
            let mut cat_sum = 0.0;
            for (ox, mw, c_n, _) in specs {
                let apfu = (s.ox(ox) / mw) * c_n * fac;
                cat_sum += apfu;
                r.push(fmt3(apfu));
            }
            r.push(fmt3(cat_sum));
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Anion,
        title: format!("ANION — Oxygen-Normalized Formula ({target_o:.1} O)"),
        headers,
        rows,
    }
}

fn build_cation_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let headers = vec![
        "SampleID", "Si %cat", "Ti %cat", "Al %cat", "Fe %cat", "Mg %cat", "Ca %cat", "Na %cat",
        "K %cat", "Mg#",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let specs = [
        ("SiO2", 60.0843, 1.0),
        ("TiO2", 79.8658, 1.0),
        ("Al2O3", 101.9613, 2.0),
        ("Fe2O3", 159.6882, 2.0),
        ("MgO", 40.3044, 1.0),
        ("CaO", 56.0774, 1.0),
        ("Na2O", 61.9789, 2.0),
        ("K2O", 94.196, 2.0),
    ];
    let rows = samples
        .iter()
        .map(|s| {
            let cats: Vec<f64> = specs.iter().map(|(ox, mw, c_n)| (s.ox(ox) / mw) * c_n).collect();
            let sum: f64 = cats.iter().sum();
            let sc = if sum > 1e-9 { 100.0 / sum } else { 0.0 };
            let mg_num = if cats[4] + cats[3] > 1e-9 {
                100.0 * cats[4] / (cats[4] + cats[3])
            } else {
                0.0
            };
            let mut r = vec![s.sample_id.clone()];
            for c in cats {
                r.push(fmt2(c * sc));
            }
            r.push(fmt1(mg_num));
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Cation,
        title: "CATION — Cation-Normalized Proportions (%)".into(),
        headers,
        rows,
    }
}

fn build_element_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let els = ["Si", "Ti", "Al", "Fe", "Mn", "Mg", "Ca", "Na", "K", "P", "Cr", "S"];
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(els.iter().map(|e| format!("{e} %")));
    let rows = samples
        .iter()
        .map(|s| {
            let mut r = vec![s.sample_id.clone()];
            for e in els {
                r.push(fmt3(s.el(e)));
            }
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Element,
        title: "ELEMENT — Major Elements (wt%)".into(),
        headers,
        rows,
    }
}

fn build_oxide_sheet(samples: &[MergedPetroSample]) -> PetroResultSheet {
    let mut headers = vec!["SampleID".to_string()];
    headers.extend(MAJOR_OXIDES.iter().map(|o| format!("{o} %")));
    let rows = samples
        .iter()
        .map(|s| {
            let mut r = vec![s.sample_id.clone()];
            for &ox in MAJOR_OXIDES {
                let v = if s.ox(ox) > 0.0 {
                    s.ox(ox)
                } else if let Some((el, fac)) = oxide_to_element_factor(ox) {
                    s.el(el) / fac
                } else {
                    0.0
                };
                r.push(fmt2(v));
            }
            r
        })
        .collect();
    PetroResultSheet {
        routine: PetroRoutine::Oxide,
        title: "OXIDE — Major Oxides (wt%)".into(),
        headers,
        rows,
    }
}

fn fmt1(v: f64) -> String { format!("{v:.1}") }
fn fmt2(v: f64) -> String { format!("{v:.2}") }
fn fmt3(v: f64) -> String { format!("{v:.3}") }

// ============================================================================
// 5. Modal Calculation & Monte Carlo Error Estimation (§84.7)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModalSolveMethod {
    LeastSquares,
    WeightedLeastSquares,
    NonNegativeLeastSquares,
    WeightedNnls,
}

impl ModalSolveMethod {
    pub const ALL: [ModalSolveMethod; 4] = [
        Self::LeastSquares,
        Self::WeightedLeastSquares,
        Self::NonNegativeLeastSquares,
        Self::WeightedNnls,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LeastSquares => "Least Squares (LS)",
            Self::WeightedLeastSquares => "Weighted Least Squares (WLS)",
            Self::NonNegativeLeastSquares => "Non-negative Least Squares (NNLS)",
            Self::WeightedNnls => "Weighted non-negative Least Squares (WNNLS)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalMineralDef {
    pub name: String,
    pub location: String,
    pub formula: String,
    pub sg: f64,
    pub composition: BTreeMap<String, f64>, // element -> wt%
}

impl ModalMineralDef {
    pub fn display_name(&self) -> String {
        if self.location.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.location)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalRound {
    pub name: String,
    pub mineral_indices: Vec<usize>,
    pub elements: Vec<String>,
    pub is_remainder: bool,
    pub method: ModalSolveMethod,
}

impl ModalRound {
    pub fn determination_label(&self) -> String {
        let n_m = self.mineral_indices.len();
        let n_e = if self.is_remainder { 1 } else { self.elements.len() };
        if n_m == 0 {
            return "No Phases".into();
        }
        if n_e == n_m {
            format!("Exactly Determined {n_e}/{n_m}")
        } else if n_e > n_m {
            format!("Overdetermined {n_e}/{n_m}")
        } else {
            format!("Underdetermined {n_e}/{n_m}")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalSampleRow {
    pub id: String,
    pub values: BTreeMap<String, f64>, // element/column -> wt%
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalSampleResult {
    pub sample_id: String,
    pub mineral_wt: Vec<f64>,
    pub total_before_norm: f64,
    pub total_after_norm: f64,
    pub sg: f64,
    pub bulk_calc: BTreeMap<String, f64>,
    pub residual_abs: BTreeMap<String, f64>,
    pub residual_rel_pct: BTreeMap<String, f64>,
    pub rmse: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalMonteCarloResult {
    pub iterations: usize,
    pub sample_ids: Vec<String>,
    pub mineral_names: Vec<String>,
    pub mean: Vec<Vec<f64>>,
    pub stddev: Vec<Vec<f64>>,
    pub min_v: Vec<Vec<f64>>,
    pub max_v: Vec<Vec<f64>>,
    pub cv_pct: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalProject {
    pub columns: Vec<String>, // e.g., ["Cu", "S"]
    pub samples: Vec<ModalSampleRow>,
    pub minerals: Vec<ModalMineralDef>,
    pub rounds: Vec<ModalRound>,
    pub normalize_total: bool,
    pub fraction_selected_minerals: BTreeSet<usize>,
    pub results: Vec<ModalSampleResult>,
    // Error Estimation (Monte Carlo §84.7.2)
    pub mc_rounds: usize,
    pub sample_rsd: Vec<Vec<f64>>,  // [sample_idx][col_idx] % RSD (default 4%)
    pub mineral_rsd: Vec<Vec<f64>>, // [el_idx][mineral_idx] % RSD (default 1%)
    pub mineral_rsd_elements: Vec<String>,
    pub mc_result: Option<ModalMonteCarloResult>,
}

impl Default for ModalProject {
    fn default() -> Self {
        let mut p = Self::cu_s_tutorial();
        p.regenerate_cv_tables();
        p.calculate();
        p
    }
}

impl ModalProject {
    /// Exact §84.7 Cu-S Tutorial (`Chalcopyrite`, `Pyrite`, `Quartz`, 4 samples, 2 Rounds).
    pub fn cu_s_tutorial() -> Self {
        let columns = vec!["Cu".into(), "S".into()];
        let samples = vec![
            ModalSampleRow {
                id: "1".into(),
                values: [("Cu".into(), 0.82), ("S".into(), 2.55)].into_iter().collect(),
            },
            ModalSampleRow {
                id: "2".into(),
                values: [("Cu".into(), 1.45), ("S".into(), 4.10)].into_iter().collect(),
            },
            ModalSampleRow {
                id: "3".into(),
                values: [("Cu".into(), 0.35), ("S".into(), 1.90)].into_iter().collect(),
            },
            ModalSampleRow {
                id: "4".into(),
                values: [("Cu".into(), 2.10), ("S".into(), 5.20)].into_iter().collect(),
            },
        ];
        let minerals = vec![
            ModalMineralDef {
                name: "Chalcopyrite".into(),
                location: "[stoichiometric]".into(),
                formula: "CuFeS2".into(),
                sg: 4.20,
                composition: [
                    ("Cu".into(), 34.629),
                    ("Fe".into(), 30.430),
                    ("S".into(), 34.941),
                ]
                .into_iter()
                .collect(),
            },
            ModalMineralDef {
                name: "Pyrite".into(),
                location: "[stoichiometric]".into(),
                formula: "FeS2".into(),
                sg: 5.01,
                composition: [("Fe".into(), 46.550), ("S".into(), 53.450)]
                    .into_iter()
                    .collect(),
            },
            ModalMineralDef {
                name: "Quartz".into(),
                location: "[stoichiometric]".into(),
                formula: "SiO2".into(),
                sg: 2.65,
                composition: [("Si".into(), 46.743), ("O".into(), 53.257)]
                    .into_iter()
                    .collect(),
            },
        ];
        let rounds = vec![
            ModalRound {
                name: "Round 1".into(),
                mineral_indices: vec![0, 1],
                elements: vec!["Cu".into(), "S".into()],
                is_remainder: false,
                method: ModalSolveMethod::NonNegativeLeastSquares,
            },
            ModalRound {
                name: "Round 2".into(),
                mineral_indices: vec![2],
                elements: Vec::new(),
                is_remainder: true,
                method: ModalSolveMethod::LeastSquares,
            },
        ];
        let mut fraction_selected_minerals = BTreeSet::new();
        fraction_selected_minerals.insert(0);
        fraction_selected_minerals.insert(1);

        Self {
            columns,
            samples,
            minerals,
            rounds,
            normalize_total: false,
            fraction_selected_minerals,
            results: Vec::new(),
            mc_rounds: 50,
            sample_rsd: Vec::new(),
            mineral_rsd: Vec::new(),
            mineral_rsd_elements: Vec::new(),
            mc_result: None,
        }
    }

    pub fn add_column(&mut self, col: String) {
        if !self.columns.contains(&col) {
            self.columns.push(col.clone());
            for s in &mut self.samples {
                s.values.entry(col.clone()).or_insert(0.0);
            }
            self.regenerate_cv_tables();
        }
    }

    pub fn remove_column(&mut self, col: &str) {
        self.columns.retain(|c| c != col);
        for s in &mut self.samples {
            s.values.remove(col);
        }
        for r in &mut self.rounds {
            r.elements.retain(|e| e != col);
        }
        self.regenerate_cv_tables();
    }

    pub fn add_mineral(&mut self, m: ModalMineralDef) {
        if !self
            .minerals
            .iter()
            .any(|x| x.name == m.name && x.location == m.location)
        {
            self.minerals.push(m);
            self.regenerate_cv_tables();
        }
    }

    pub fn remove_mineral(&mut self, idx: usize) {
        if idx >= self.minerals.len() {
            return;
        }
        self.minerals.remove(idx);
        for r in &mut self.rounds {
            r.mineral_indices.retain(|&i| i != idx);
            for i in &mut r.mineral_indices {
                if *i > idx {
                    *i -= 1;
                }
            }
        }
        self.fraction_selected_minerals.clear();
        self.regenerate_cv_tables();
    }

    /// Solve a single sample given mineral compositions (used by both `calculate` and Monte Carlo).
    fn solve_sample_with_comps(
        &self,
        sample_id: &str,
        assays: &BTreeMap<String, f64>,
        min_comps: &[BTreeMap<String, f64>],
    ) -> ModalSampleResult {
        let n_min = self.minerals.len();
        let mut x = vec![0.0; n_min];
        let mut assigned = vec![false; n_min];
        let mut remaining_b = assays.clone();

        for round in &self.rounds {
            let m_idxs: Vec<usize> = round
                .mineral_indices
                .iter()
                .copied()
                .filter(|&i| i < n_min)
                .collect();
            if m_idxs.is_empty() {
                continue;
            }
            if round.is_remainder {
                let used_sum: f64 = x.iter().copied().sum();
                let rem = (100.0 - used_sum).max(0.0);
                let share = rem / (m_idxs.len() as f64);
                for &mi in &m_idxs {
                    x[mi] = share;
                    assigned[mi] = true;
                }
                continue;
            }
            if round.elements.is_empty() {
                continue;
            }

            let m = round.elements.len();
            let n = m_idxs.len();
            let mut a = vec![vec![0.0; n]; m];
            let mut b = vec![0.0; m];
            let mut w = vec![1.0; m];

            for (ei, el) in round.elements.iter().enumerate() {
                let target = remaining_b.get(el).copied().unwrap_or(0.0);
                b[ei] = target;
                let obs_orig = assays.get(el).copied().unwrap_or(0.0).abs();
                w[ei] = match round.method {
                    ModalSolveMethod::WeightedLeastSquares | ModalSolveMethod::WeightedNnls => {
                        1.0 / obs_orig.max(0.05).powi(2)
                    }
                    ModalSolveMethod::LeastSquares | ModalSolveMethod::NonNegativeLeastSquares => 1.0,
                };
                for (col_j, &mi) in m_idxs.iter().enumerate() {
                    // Support both element columns AND XRD mineral columns (`[Mineral]`)
                    if el == &self.minerals[mi].name {
                        a[ei][col_j] = 1.0;
                    } else {
                        a[ei][col_j] = min_comps[mi].get(el).copied().unwrap_or(0.0) / 100.0;
                    }
                }
            }

            let sol = match round.method {
                ModalSolveMethod::LeastSquares | ModalSolveMethod::WeightedLeastSquares => {
                    solve_wls(&a, &b, &w)
                }
                ModalSolveMethod::NonNegativeLeastSquares | ModalSolveMethod::WeightedNnls => {
                    solve_wnnls(&a, &b, &w)
                }
            };

            for (&mi, &val) in m_idxs.iter().zip(sol.iter()) {
                let v = if val.is_finite() { val } else { 0.0 };
                x[mi] = v;
                assigned[mi] = true;
                // Subtract this mineral's elemental contribution from remaining_b for subsequent rounds
                for (el, &comp_pct) in &min_comps[mi] {
                    if let Some(rem) = remaining_b.get_mut(el) {
                        *rem -= v * (comp_pct / 100.0);
                    }
                }
            }
        }

        let total_before_norm: f64 = x.iter().copied().sum();
        if self.normalize_total && total_before_norm > 1e-9 {
            for v in &mut x {
                *v *= 100.0 / total_before_norm;
            }
        }
        let total_after_norm: f64 = x.iter().copied().sum();

        // Specific Gravity per §84.7 p. 39: SG = sum(x_i) / sum(x_i / SG_i) (ignoring missing mass)
        let mut vol_sum = 0.0;
        let mut mass_pos = 0.0;
        for (mi, &w_i) in x.iter().enumerate() {
            if w_i > 0.0 && self.minerals[mi].sg > 0.1 {
                mass_pos += w_i;
                vol_sum += w_i / self.minerals[mi].sg;
            }
        }
        let sg = if vol_sum > 1e-9 { mass_pos / vol_sum } else { 0.0 };

        // Back-calculated bulk elemental chemistry
        let mut all_els: BTreeSet<String> = self.columns.iter().cloned().collect();
        for comp in min_comps {
            all_els.extend(comp.keys().cloned());
        }
        let mut bulk_calc = BTreeMap::new();
        for el in &all_els {
            let mut sum = 0.0;
            for (mi, &w_i) in x.iter().enumerate() {
                sum += w_i * min_comps[mi].get(el).copied().unwrap_or(0.0) / 100.0;
            }
            bulk_calc.insert(el.clone(), sum);
        }

        // Residuals for input assay columns
        let mut residual_abs = BTreeMap::new();
        let mut residual_rel_pct = BTreeMap::new();
        let mut sq_err = 0.0;
        let mut n_err = 0usize;
        for col in &self.columns {
            let obs = assays.get(col).copied().unwrap_or(0.0);
            let calc = bulk_calc.get(col).copied().unwrap_or(0.0);
            let abs_r = calc - obs;
            let rel_r = if obs.abs() > 1e-9 { 100.0 * abs_r / obs } else { 0.0 };
            residual_abs.insert(col.clone(), abs_r);
            residual_rel_pct.insert(col.clone(), rel_r);
            sq_err += abs_r * abs_r;
            n_err += 1;
        }
        let rmse = if n_err > 0 { (sq_err / n_err as f64).sqrt() } else { 0.0 };

        let note = if total_before_norm > 105.0 {
            "Warning: Unnormalized total > 105%".into()
        } else if total_before_norm < 90.0 && !self.rounds.iter().any(|r| r.is_remainder) {
            "Note: Total < 90% (missing gangue phase?)".into()
        } else {
            "OK".into()
        };

        ModalSampleResult {
            sample_id: sample_id.to_string(),
            mineral_wt: x,
            total_before_norm,
            total_after_norm,
            sg,
            bulk_calc,
            residual_abs,
            residual_rel_pct,
            rmse,
            note,
        }
    }

    /// Run Modal Calculation across all samples (§84.7.1).
    pub fn calculate(&mut self) {
        let comps: Vec<BTreeMap<String, f64>> =
            self.minerals.iter().map(|m| m.composition.clone()).collect();
        self.results = self
            .samples
            .iter()
            .map(|s| self.solve_sample_with_comps(&s.id, &s.values, &comps))
            .collect();
    }

    /// Compute elemental Distribution (% of total `element` hosted in each mineral) per sample.
    pub fn element_distribution(&self, element: &str) -> Vec<Vec<f64>> {
        self.results
            .iter()
            .map(|res| {
                let contribs: Vec<f64> = res
                    .mineral_wt
                    .iter()
                    .enumerate()
                    .map(|(mi, &w)| w * self.minerals[mi].composition.get(element).copied().unwrap_or(0.0) / 100.0)
                    .collect();
                let tot: f64 = contribs.iter().sum();
                if tot > 1e-12 {
                    contribs.into_iter().map(|c| 100.0 * c / tot).collect()
                } else {
                    vec![0.0; self.minerals.len()]
                }
            })
            .collect()
    }

    /// Compute normalized `Fraction` (selected subset of minerals normalized to 100%).
    pub fn calculate_fraction(&self) -> Vec<(String, Vec<f64>)> {
        let sel: Vec<usize> = if self.fraction_selected_minerals.is_empty() {
            (0..self.minerals.len()).collect()
        } else {
            self.fraction_selected_minerals
                .iter()
                .copied()
                .filter(|&i| i < self.minerals.len())
                .collect()
        };
        self.results
            .iter()
            .map(|res| {
                let vals: Vec<f64> = sel.iter().map(|&i| res.mineral_wt[i].max(0.0)).collect();
                let sum: f64 = vals.iter().sum();
                let norm = if sum > 1e-9 {
                    vals.into_iter().map(|v| 100.0 * v / sum).collect()
                } else {
                    vec![0.0; sel.len()]
                };
                (res.sample_id.clone(), norm)
            })
            .collect()
    }

    /// Regenerate default CV-tables for Monte Carlo Error Estimation (§84.7.2, `Imagen pegada (52).png`).
    pub fn regenerate_cv_tables(&mut self) {
        self.sample_rsd = vec![vec![4.0; self.columns.len()]; self.samples.len()];
        let mut el_set = BTreeSet::new();
        for m in &self.minerals {
            for (k, &v) in &m.composition {
                if v > 1e-6 {
                    el_set.insert(k.clone());
                }
            }
        }
        self.mineral_rsd_elements = el_set.into_iter().collect();
        self.mineral_rsd = self
            .mineral_rsd_elements
            .iter()
            .map(|el| {
                self.minerals
                    .iter()
                    .map(|m| if m.composition.get(el).copied().unwrap_or(0.0) > 1e-6 { 1.0 } else { 0.0 })
                    .collect()
            })
            .collect();
    }

    /// Execute Monte Carlo Error Estimation (§84.7.2).
    pub fn run_monte_carlo(&mut self, seed: u64) {
        if self.sample_rsd.len() != self.samples.len()
            || self.sample_rsd.first().map(Vec::len).unwrap_or(0) != self.columns.len()
            || self.mineral_rsd.len() != self.mineral_rsd_elements.len()
        {
            self.regenerate_cv_tables();
        }
        let n_iter = self.mc_rounds.clamp(5, 2000);
        let n_samp = self.samples.len();
        let n_min = self.minerals.len();
        if n_samp == 0 || n_min == 0 {
            self.mc_result = None;
            return;
        }

        let mut rng = FastRng::new(seed);
        let mut draws: Vec<Vec<Vec<f64>>> = vec![vec![Vec::with_capacity(n_iter); n_min]; n_samp];

        for _ in 0..n_iter {
            // Perturb mineral compositions
            let mut p_comps: Vec<BTreeMap<String, f64>> =
                self.minerals.iter().map(|m| m.composition.clone()).collect();
            for (ei, el) in self.mineral_rsd_elements.iter().enumerate() {
                for (mi, comp) in p_comps.iter_mut().enumerate() {
                    let rsd = self
                        .mineral_rsd
                        .get(ei)
                        .and_then(|r| r.get(mi))
                        .copied()
                        .unwrap_or(0.0);
                    if rsd > 0.0 {
                        if let Some(val) = comp.get_mut(el) {
                            let z = rng.next_normal();
                            *val = (*val * (1.0 + (rsd / 100.0) * z)).max(0.0);
                        }
                    }
                }
            }

            // Perturb each sample's assays and re-solve
            for (si, samp) in self.samples.iter().enumerate() {
                let mut p_assay = samp.values.clone();
                for (ci, col) in self.columns.iter().enumerate() {
                    let rsd = self
                        .sample_rsd
                        .get(si)
                        .and_then(|r| r.get(ci))
                        .copied()
                        .unwrap_or(0.0);
                    if rsd > 0.0 {
                        if let Some(val) = p_assay.get_mut(col) {
                            let z = rng.next_normal();
                            *val = (*val * (1.0 + (rsd / 100.0) * z)).max(0.0);
                        }
                    }
                }
                let res = self.solve_sample_with_comps(&samp.id, &p_assay, &p_comps);
                for mi in 0..n_min {
                    draws[si][mi].push(res.mineral_wt[mi]);
                }
            }
        }

        let mut mean = vec![vec![0.0; n_min]; n_samp];
        let mut stddev = vec![vec![0.0; n_min]; n_samp];
        let mut min_v = vec![vec![0.0; n_min]; n_samp];
        let mut max_v = vec![vec![0.0; n_min]; n_samp];
        let mut cv_pct = vec![vec![0.0; n_min]; n_samp];

        for si in 0..n_samp {
            for mi in 0..n_min {
                let v = &draws[si][mi];
                let m = v.iter().sum::<f64>() / (v.len() as f64);
                let var = if v.len() > 1 {
                    v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / ((v.len() - 1) as f64)
                } else {
                    0.0
                };
                let sd = var.sqrt();
                let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                mean[si][mi] = m;
                stddev[si][mi] = sd;
                min_v[si][mi] = lo;
                max_v[si][mi] = hi;
                cv_pct[si][mi] = if m.abs() > 1e-9 { 100.0 * sd / m.abs() } else { 0.0 };
            }
        }

        self.mc_result = Some(ModalMonteCarloResult {
            iterations: n_iter,
            sample_ids: self.samples.iter().map(|s| s.id.clone()).collect(),
            mineral_names: self.minerals.iter().map(|m| m.name.clone()).collect(),
            mean,
            stddev,
            min_v,
            max_v,
            cv_pct,
        });
    }

    /// Export Modal Results (+ Monte Carlo if calculated) to `.xlsx`.
    pub fn export_xlsx(&self) -> Result<Vec<u8>, String> {
        if self.results.is_empty() {
            return Err("Run Calculate before exporting.".into());
        }
        let mut sheets = Vec::new();

        // Modal sheet
        let mut h_modal = vec!["Sample".to_string()];
        h_modal.extend(self.minerals.iter().map(|m| format!("{} %", m.name)));
        h_modal.extend(["Total %".into(), "SG".into()]);
        let mut r_modal = vec![h_modal];
        for r in &self.results {
            let mut row = vec![r.sample_id.clone()];
            row.extend(r.mineral_wt.iter().map(|v| format!("{v:.3}")));
            row.push(format!("{:.2}", r.total_after_norm));
            row.push(format!("{:.3}", r.sg));
            r_modal.push(row);
        }
        sheets.push(make_export_sheet("Modal".into(), &r_modal));

        // Residual sheet
        let mut h_res = vec!["Sample".to_string()];
        for c in &self.columns {
            h_res.push(format!("{c} Abs"));
            h_res.push(format!("{c} Rel%"));
        }
        let mut r_res = vec![h_res];
        for r in &self.results {
            let mut row = vec![r.sample_id.clone()];
            for c in &self.columns {
                row.push(format!("{:.4}", r.residual_abs.get(c).copied().unwrap_or(0.0)));
                row.push(format!("{:.2}", r.residual_rel_pct.get(c).copied().unwrap_or(0.0)));
            }
            r_res.push(row);
        }
        sheets.push(make_export_sheet("Residual".into(), &r_res));

        // Monte Carlo Average ± 1 Sigma if present
        if let Some(mc) = &self.mc_result {
            let mut h_mc = vec!["Sample".to_string()];
            h_mc.extend(mc.mineral_names.iter().cloned());
            let mut r_mc = vec![h_mc];
            for (si, sid) in mc.sample_ids.iter().enumerate() {
                let mut row = vec![sid.clone()];
                for mi in 0..mc.mineral_names.len() {
                    row.push(format!("{:.2} ± {:.2}", mc.mean[si][mi], mc.stddev[si][mi]));
                }
                r_mc.push(row);
            }
            sheets.push(make_export_sheet("MC_Avg_1Sigma".into(), &r_mc));
        }

        Ok(build_xlsx(&sheets))
    }
}

// ============================================================================
// Default Curated Reference Datasets (3,812 Minerals + Multi-Location Microprobe + Phases + Dissolution)
// ============================================================================

fn build_default_minerals() -> Vec<GeoMineralRecord> {
    MINERALS_RAW
        .iter()
        .map(|row| {
            let comp = parse_composition_map(row.composition);
            let els = parse_elements_set(row.elements);
            let sg = estimate_mineral_sg(row.name, row.group, &comp);
            let (color, hardness, luster, ore) = mineral_physical_defaults(row.name);
            GeoMineralRecord {
                id: row.id,
                name: row.name.to_string(),
                symbol: row.code.to_string(),
                formula: row.formula.to_string(),
                empirical_formula: row.formula.to_string(),
                group: row.group.to_string(),
                symbol_ref: "IMA".into(),
                note: String::new(),
                composition_str: row.composition.to_string(),
                composition: comp,
                elements_str: row.elements.to_string(),
                elements: els,
                ore_mineral: ore.into(),
                density: sg,
                color: color.into(),
                hardness: hardness.into(),
                luster: luster.into(),
                mag_susc: String::new(),
                formula_base: String::new(),
                date_of_input: "HSC10".into(),
                inputted_by: "HSC Geo".into(),
            }
        })
        .collect()
}

fn mineral_physical_defaults(name: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match name {
        "Sphalerite" => ("Brown - Yellow", "3.5...4", "Resinous - Greasy", "Zn"),
        "Galena" => ("Lead gray", "2.5", "Metallic", "Pb"),
        "Chalcopyrite" => ("Brass yellow", "3.5...4", "Metallic", "Cu"),
        "Pyrite" => ("Pale brass yellow", "6...6.5", "Metallic", "Fe, S"),
        "Pyrrhotite" => ("Bronze yellow", "3.5...4.5", "Metallic", "Fe, S"),
        "Pentlandite" => ("Light bronze yellow", "3.5...4", "Metallic", "Ni"),
        "Tetrahedrite" => ("Steel gray - Iron black", "3...4.5", "Metallic", "Cu, Sb, Ag"),
        "Quartz" => ("Colorless - White", "7", "Vitreous", ""),
        "Magnetite" => ("Iron black", "5.5...6.5", "Metallic", "Fe"),
        "Hematite" => ("Steel gray - Red", "5.5...6.5", "Metallic - Earthy", "Fe"),
        "Calcite" => ("White - Colorless", "3", "Vitreous", ""),
        "Dolomite" => ("White - Pink", "3.5...4", "Vitreous - Pearly", ""),
        "Biotite" => ("Dark brown - Black", "2.5...3", "Pearly - Submetallic", ""),
        "Albite" => ("White - Colorless", "6...6.5", "Vitreous", ""),
        "Bornite" => ("Bronze - Iridescent purple", "3", "Metallic", "Cu"),
        "Arsenopyrite" => ("Silver white", "5.5...6", "Metallic", "As"),
        "Molybdenite" => ("Lead gray", "1...1.5", "Metallic", "Mo"),
        "Chromite" => ("Brownish black", "5.5", "Submetallic", "Cr"),
        "Ilmenite" => ("Iron black", "5...6", "Metallic - Submetallic", "Ti"),
        _ => ("", "", "", ""),
    }
}

fn mk_chem(
    id: u32,
    sym: &str,
    mineral: &str,
    location: &str,
    key: &str,
    reference: &str,
    note1: &str,
    sg: f64,
    pairs: &[(&str, f64)],
) -> MineralChemRecord {
    MineralChemRecord {
        id,
        mineral_symbol: sym.into(),
        mineral: mineral.into(),
        location: location.into(),
        mineral_key: key.into(),
        statistics: String::new(),
        reference: reference.into(),
        sample_no: format!("S-{id}"),
        note1: note1.into(),
        note2: String::new(),
        note3: key.into(),
        database: "HSC".into(),
        sg: Some(sg),
        values: pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
    }
}

fn build_default_mineral_chemistry(minerals: &[GeoMineralRecord]) -> Vec<MineralChemRecord> {
    let sym_map: BTreeMap<String, (String, f64)> = minerals
        .iter()
        .map(|m| (m.name.to_ascii_lowercase(), (m.symbol.clone(), m.density)))
        .collect();
    let mut recs = Vec::new();
    let mut id = 100u32;
    let mut push = |m: &str, loc: &str, key: &str, refr: &str, note: &str, vals: &[(&str, f64)]| {
        let (sym, sg) = sym_map
            .get(&m.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| (m.chars().take(3).collect(), 4.0));
        recs.push(mk_chem(id, &sym, m, loc, key, refr, note, sg, vals));
        id += 1;
    };

    // Sphalerite — rich multi-locality dataset (including Image 4/5 "Location Begins with S" examples!)
    push("Sphalerite", "[stoichiometric]", "ZnS", "HSC Formula", "ZnS", &[("Zn", 67.097), ("S", 32.903)]);
    let sphal_locs = [
        ("Neves Corvo", "Graca 1990", "Graca 1990", "Hg,Sn trace", 58.2, 33.4, 6.9, 0.35, 0.12, 0.92, 0.18, 0.08, 0.42),
        ("Sweetwater, Viburnum Trend", "Viets et al. 1992", "Viets et al.", "Brown orange", 65.4, 32.9, 0.65, 0.55, 0.04, 0.10, 0.03, 0.02, 0.04),
        ("Stoeker mine, C. Missouri", "Viets et al. 1992", "Viets et al.", "Black-banded", 57.9, 33.1, 7.8, 0.62, 0.35, 0.18, 0.05, 0.03, 0.09),
        ("Stoeker mine, C. Missouri", "Viets et al. 1992", "Viets et al.", "Yellow", 65.6, 32.8, 0.85, 0.42, 0.05, 0.08, 0.02, 0.01, 0.03),
        ("Stoeker mine, C. Missouri", "Viets et al. 1992", "Viets et al.", "Brown", 61.5, 33.0, 4.4, 0.54, 0.19, 0.12, 0.04, 0.02, 0.06),
        ("Sonoro", "Sonoro, HOM", "Anthony et al.", "Vein", 63.2, 32.9, 2.8, 0.48, 0.12, 0.11, 0.04, 0.02, 0.05),
        ("S. Martin", "S. Martin", "IMMSA", "avg.", 59.4, 33.2, 6.1, 0.38, 0.25, 0.21, 0.06, 0.03, 0.08),
        ("S. Barbara", "S. Barbara", "IMMSA", "avg.", 58.8, 33.1, 6.7, 0.41, 0.28, 0.19, 0.05, 0.03, 0.07),
        ("Copperhead Mine", "Viets et al. 1992", "Viets et al.", "Black-banded", 56.8, 33.0, 8.8, 0.48, 0.65, 0.14, 0.06, 0.04, 0.15),
        ("Copperhead Mine", "Viets et al. 1992", "Viets et al.", "Yellow", 65.1, 32.8, 1.1, 0.52, 0.08, 0.12, 0.04, 0.02, 0.05),
        ("Copperhead Mine", "Viets et al. 1992", "Viets et al.", "Orange", 63.4, 33.2, 2.5, 0.61, 0.14, 0.09, 0.05, 0.03, 0.07),
        ("Jackpot Mine", "Viets et al. 1992", "Viets et al.", "Black-banded", 54.0, 32.5, 11.4, 0.95, 0.82, 0.20, 0.08, 0.05, 0.12),
        ("Jackpot Mine", "Viets et al. 1992", "Viets et al.", "Orange", 62.1, 32.6, 4.1, 0.72, 0.25, 0.15, 0.04, 0.02, 0.08),
        ("Lucky Dog Mine", "Viets et al. 1992", "Viets et al.", "Pale yellow", 66.2, 32.9, 0.45, 0.31, 0.05, 0.06, 0.02, 0.01, 0.03),
        ("Lucky Dog Mine", "Viets et al. 1992", "Viets et al.", "Red brown", 58.6, 33.8, 6.4, 0.68, 0.38, 0.16, 0.06, 0.04, 0.10),
        ("Madison Mine", "Viets et al. 1992", "Viets et al.", "Black-banded", 55.3, 34.0, 9.2, 0.85, 0.42, 0.18, 0.09, 0.05, 0.11),
        ("Madison Mine", "Viets et al. 1992", "Viets et al.", "Orange", 64.5, 33.1, 1.8, 0.44, 0.11, 0.08, 0.03, 0.02, 0.04),
        ("Monte Cristo", "Viets et al. 1992", "Viets et al.", "Brown", 59.9, 32.2, 6.1, 1.15, 0.44, 0.19, 0.05, 0.03, 0.09),
        ("Pyhasalmi", "Deep Ore", "Outotec", "VMS ore", 57.5, 33.7, 7.6, 0.28, 0.22, 0.35, 0.12, 0.06, 0.18),
        ("Outokumpu", "Cu-Co-Zn", "Outotec", "Cu-Co-Zn", 56.1, 33.2, 9.4, 0.22, 0.30, 0.42, 0.22, 0.08, 0.24),
        ("Kidd Creek", "Cook 2009", "Cook 2009", "In-rich", 58.1, 33.3, 5.9, 0.40, 0.15, 0.85, 0.14, 0.05, 1.15),
        ("Broken Hill", "Cook 2009", "Cook 2009", "Mn-Fe-rich", 52.0, 31.4, 11.8, 0.32, 4.10, 0.18, 0.10, 0.04, 0.08),
        ("Balmat", "Cook 2009", "Cook 2009", "Marmatite", 53.9, 31.9, 10.6, 0.55, 2.40, 0.24, 0.08, 0.03, 0.06),
        ("Santander", "Cook 2009", "Cook 2009", "Honey blende", 66.6, 32.8, 0.25, 0.18, 0.03, 0.05, 0.02, 0.01, 0.02),
        ("Red Dog", "Kelley 2004", "Kelley 2004", "SEDEX", 64.9, 32.8, 1.4, 0.62, 0.09, 0.11, 0.04, 0.02, 0.06),
        ("Freiberg", "Seifert 2008", "Seifert 2008", "Ag-In-bearing", 55.8, 31.5, 9.8, 0.78, 0.55, 0.64, 0.25, 0.12, 0.68),
        ("Creede", "Barton 1977", "Barton 1977", "Epithermal", 61.2, 33.2, 3.8, 1.35, 0.28, 0.14, 0.06, 0.02, 0.09),
        ("Mount Isa", "Perkins 1997", "Perkins 1997", "Fine-grained", 59.1, 33.5, 6.2, 0.45, 0.31, 0.26, 0.08, 0.04, 0.12),
        ("Tara Navan", "Anderson 1998", "Anderson 1998", "Pale", 65.8, 32.9, 0.75, 0.38, 0.06, 0.08, 0.03, 0.01, 0.04),
    ];
    for (loc, key, refr, note, zn, s, fe, cd, mn, cu, hg, ag, ind) in sphal_locs {
        push(
            "Sphalerite",
            loc,
            key,
            refr,
            note,
            &[
                ("Zn", zn), ("S", s), ("Fe", fe), ("Cd", cd), ("Mn", mn),
                ("Cu", cu), ("Hg", hg), ("Ag", ag), ("In", ind),
            ],
        );
    }

    // Tetrahedrite — including Ag-rich Freibergite variants matching §84.5 EDS example (26% Cu, 22% Sb, 27% S, 5% Fe, 20% Ag)!
    push("Tetrahedrite", "Freiberg (Ag-rich)", "Moelo 2008", "Moelo 2008", "Ag-tetrahedrite", &[("Cu", 26.2), ("Sb", 22.3), ("S", 26.7), ("Fe", 4.9), ("Ag", 19.6), ("Zn", 0.3)]);
    push("Tetrahedrite", "Keno Hill", "Lynch 1989", "Lynch 1989", "Freibergite var.", &[("Cu", 24.8), ("Sb", 23.1), ("S", 25.4), ("Fe", 5.2), ("Ag", 20.5), ("Zn", 1.0)]);
    push("Tetrahedrite", "Coeur d'Alene", "Sack 2002", "Sack 2002", "Ag-bearing", &[("Cu", 27.6), ("Sb", 22.8), ("S", 25.9), ("Fe", 4.6), ("Ag", 17.8), ("Zn", 1.3)]);
    push("Tetrahedrite", "Boliden", "Wagner 2005", "Wagner 2005", "Cu-Sb-Fe", &[("Cu", 38.4), ("Sb", 27.2), ("S", 25.1), ("Fe", 4.2), ("Zn", 3.1), ("Ag", 1.8)]);
    push("Tetrahedrite", "Potosi", "Miller 1990", "Miller 1990", "Zn-rich", &[("Cu", 34.5), ("Sb", 25.8), ("S", 24.8), ("Fe", 2.9), ("Zn", 4.8), ("Ag", 6.9)]);
    push("Tetrahedrite", "Casapalca", "Wu & Petersen", "Wu & Petersen", "As-bearing", &[("Cu", 37.2), ("Sb", 22.4), ("As", 4.1), ("S", 25.6), ("Fe", 3.5), ("Zn", 3.8), ("Ag", 3.4)]);

    // Chalcopyrite
    push("Chalcopyrite", "[stoichiometric]", "CuFeS2", "HSC Formula", "CuFeS2", &[("Cu", 34.629), ("Fe", 30.430), ("S", 34.941)]);
    push("Chalcopyrite", "Pyhasalmi", "Outotec", "Outotec", "Massive ore", &[("Cu", 34.4), ("Fe", 30.6), ("S", 34.8), ("Zn", 0.15)]);
    push("Chalcopyrite", "Sudbury", "Naldrett", "Naldrett", "Ni-Cu ore", &[("Cu", 34.5), ("Fe", 30.5), ("S", 34.7), ("Ni", 0.18)]);
    push("Chalcopyrite", "Escondida", "Porphyry", "Porphyry", "Hypogene", &[("Cu", 34.7), ("Fe", 30.3), ("S", 34.9), ("Ag", 0.04)]);
    push("Chalcopyrite", "Olympic Dam", "IOCG", "IOCG", "Bornite assoc.", &[("Cu", 34.8), ("Fe", 30.2), ("S", 34.8), ("Co", 0.06)]);

    // Pyrite
    push("Pyrite", "[stoichiometric]", "FeS2", "HSC Formula", "FeS2", &[("Fe", 46.550), ("S", 53.450)]);
    push("Pyrite", "Rio Tinto", "VMS", "VMS", "As-bearing", &[("Fe", 46.1), ("S", 53.0), ("As", 0.65), ("Co", 0.12)]);
    push("Pyrite", "Pyhasalmi", "Outotec", "Outotec", "Co-bearing", &[("Fe", 46.2), ("S", 53.2), ("Co", 0.35), ("Ni", 0.10)]);
    push("Pyrite", "Carlin", "Epithermal", "Epithermal", "Arsenian", &[("Fe", 45.4), ("S", 51.8), ("As", 2.40), ("Cu", 0.18)]);

    // Galena
    push("Galena", "[stoichiometric]", "PbS", "HSC Formula", "PbS", &[("Pb", 86.599), ("S", 13.401)]);
    push("Galena", "Broken Hill", "Ag-Bi bearing", "Anthony et al.", "Metamorphosed", &[("Pb", 86.1), ("S", 13.3), ("Ag", 0.32), ("Bi", 0.18)]);
    push("Galena", "Freiberg", "Ag-Sb bearing", "Anthony et al.", "Vein", &[("Pb", 85.8), ("S", 13.4), ("Ag", 0.45), ("Sb", 0.30)]);
    push("Galena", "Laisvall", "Sandstone Pb", "Anthony et al.", "Pure", &[("Pb", 86.5), ("S", 13.4), ("Ag", 0.05)]);

    // Pentlandite & Pyrrhotite
    push("Pentlandite", "Sudbury", "Naldrett", "Naldrett", "(Fe,Ni)9S8", &[("Ni", 35.8), ("Fe", 30.4), ("S", 33.1), ("Co", 0.7)]);
    push("Pentlandite", "Bruvann", "Boyd 1983", "Boyd 1983", "Norite-hosted", &[("Ni", 36.2), ("Fe", 29.8), ("S", 33.0), ("Co", 1.0)]);
    push("Pentlandite", "Kambalda", "Komatiite", "Komatiite", "High-Ni", &[("Ni", 37.4), ("Fe", 28.9), ("S", 32.9), ("Co", 0.8)]);
    push("Pyrrhotite", "Sudbury", "Monoclinic", "Naldrett", "Fe7S8", &[("Fe", 60.4), ("S", 39.0), ("Ni", 0.55)]);
    push("Pyrrhotite", "Bruvann", "Hexagonal", "Boyd 1983", "Fe9S10", &[("Fe", 61.5), ("S", 38.0), ("Ni", 0.42)]);

    // Quartz, Biotite (DHZ Vol 3 — matching Image 7!), Vesuvianite/Olivine (Bushveld — matching Image 7!), Magnetite, Calcite
    push("Quartz", "[stoichiometric]", "SiO2", "HSC Formula", "SiO2", &[("Si", 46.743), ("O", 53.257)]);
    push("Quartz", "Outokumpu", "Vein", "Outotec", "Trace Al", &[("Si", 46.65), ("O", 53.20), ("Al", 0.08), ("Fe", 0.05)]);
    push("Biotite", "S. California Batholith", "DHZ, Vol:3, p.60", "Deer et al.", "In norite", &[("Si", 18.2), ("Al", 7.8), ("Fe", 11.4), ("Mg", 10.6), ("K", 7.6), ("O", 41.8)]);
    push("Biotite", "Liruei, Northern Nigeria", "DHZ, Vol:3, p.60", "Deer et al.", "In granite", &[("Si", 16.8), ("Al", 8.4), ("Fe", 18.6), ("Mg", 5.2), ("K", 7.5), ("O", 41.2)]);
    push("Biotite", "Glen Esk, Scotland", "DHZ, Vol:3, p.60", "Deer et al.", "Schist", &[("Si", 17.5), ("Al", 8.1), ("Fe", 15.2), ("Mg", 7.8), ("K", 7.4), ("O", 41.5)]);
    push("Vesuvianite", "Bushveld, S. Africa", "DHZ, Vol:1A", "Deer et al.", "Skarn", &[("Ca", 25.4), ("Al", 9.8), ("Si", 17.2), ("Fe", 3.4), ("Mg", 1.8), ("O", 42.1)]);
    push("Olivine", "Western Bushveld Complex, S. Africa", "1/Table2", "3614", "Ultramafic", &[("Si", 18.7), ("Mg", 27.6), ("Fe", 11.0), ("Ni", 0.28), ("O", 42.4)]);
    push("Olivine", "Bruvann Fo84", "Boyd 1983", "Boyd 1983", "Dunite", &[("Si", 18.6), ("Mg", 27.2), ("Fe", 11.4), ("Ni", 0.25), ("O", 42.5)]);
    push("Olivine", "San Carlos Fo90", "Standard", "Standard", "Mantle", &[("Si", 19.1), ("Mg", 29.8), ("Fe", 7.6), ("Ni", 0.30), ("O", 43.2)]);
    push("Magnetite", "[stoichiometric]", "Fe3O4", "HSC Formula", "Fe3O4", &[("Fe", 72.36), ("O", 27.64)]);
    push("Magnetite", "Kiruna", "IOA", "Anthony et al.", "Ti-poor", &[("Fe", 71.8), ("O", 27.6), ("Mn", 0.12)]);
    push("Calcite", "[stoichiometric]", "CaCO3", "HSC Formula", "CaCO3", &[("Ca", 40.043), ("O", 47.957)]);

    recs
}

fn build_default_phase_chemistry() -> Vec<PhaseChemRecord> {
    let mk = |id: u32, name: &str, proc: &str, desc: &str, sg: f64, vals: &[(&str, f64)]| PhaseChemRecord {
        id,
        phase_name: name.into(),
        process_type: proc.into(),
        formula_or_desc: desc.into(),
        density: sg,
        reference: "HSC Metallurgy".into(),
        values: vals.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
    };
    vec![
        mk(1, "Fayalite Slag", "Cu Smelting", "Fe2SiO4-rich", 3.95, &[("Fe", 44.5), ("Si", 16.8), ("O", 31.2), ("Ca", 2.4), ("Al", 2.6), ("Cu", 0.8), ("S", 0.7)]),
        mk(2, "Copper Matte (62% Cu)", "Flash Smelting", "Cu2S-FeS", 5.20, &[("Cu", 62.0), ("Fe", 13.5), ("S", 22.8), ("O", 0.9), ("Ni", 0.4)]),
        mk(3, "White Metal", "Cu Converting", "Cu2S", 5.60, &[("Cu", 78.5), ("S", 20.1), ("Fe", 1.1), ("O", 0.3)]),
        mk(4, "Blister Copper", "Cu Converting", "Cu (98.8%)", 8.85, &[("Cu", 98.8), ("S", 0.3), ("O", 0.6), ("Fe", 0.1)]),
        mk(5, "Calcium Ferrite Slag", "Mitsubishi", "CaO-FeOx", 4.10, &[("Fe", 48.2), ("Ca", 14.5), ("O", 30.8), ("Cu", 4.2), ("Si", 1.5)]),
        mk(6, "Zinc Ferrite Calcine", "Zn Roasting", "ZnFe2O4", 5.33, &[("Zn", 27.1), ("Fe", 46.3), ("O", 26.6)]),
        mk(7, "Lead Bullion", "Pb Smelting", "Pb-Ag-Cu", 11.20, &[("Pb", 97.4), ("Cu", 1.2), ("Fe", 0.2), ("S", 0.3)]),
        mk(8, "Ferronickel Alloy", "Ni RKEF", "Fe-Ni (25% Ni)", 8.10, &[("Fe", 73.5), ("Ni", 25.2), ("Si", 0.6), ("S", 0.05)]),
        mk(9, "Blast Furnace Slag", "Ironmaking", "CaO-SiO2-Al2O3-MgO", 2.90, &[("Ca", 29.3), ("Si", 16.8), ("Al", 6.4), ("Mg", 4.8), ("O", 41.2), ("S", 1.2)]),
        mk(10, "Nickel Matte", "Ni Smelting", "Ni3S2-Cu2S-FeS", 5.35, &[("Ni", 46.0), ("Cu", 16.5), ("Fe", 12.2), ("S", 24.8)]),
    ]
}

fn build_default_mineral_dissolution() -> Vec<MineralDissolutionRecord> {
    let mk = |id: u32, m: &str, f: &str, bm: f64, cit: f64, hcl: f64, ar: f64, hp: f64, cn: f64, note: &str| {
        MineralDissolutionRecord {
            id,
            mineral: m.into(),
            formula: f.into(),
            bromine_methanol: bm,
            citric_acid: cit,
            hcl_10pct: hcl,
            aqua_regia: ar,
            h2o2_nh4oac: hp,
            nacn: cn,
            reference: "HSC Geo / Outotec".into(),
            note: note.into(),
        }
    };
    vec![
        mk(1, "Pyrrhotite", "Fe1-xS", 100.0, 4.0, 95.0, 100.0, 98.0, 8.0, "Complete in Br-MeOH & HCl"),
        mk(2, "Pentlandite", "(Fe,Ni)9S8", 100.0, 1.0, 18.0, 100.0, 96.0, 2.0, "Selective Ni sulfide leach"),
        mk(3, "Chalcopyrite", "CuFeS2", 98.0, 0.5, 2.0, 100.0, 15.0, 6.0, "Refractory to NaCN & HCl"),
        mk(4, "Pyrite", "FeS2", 7.0, 0.0, 1.0, 100.0, 4.0, 1.0, "Only ~7% soluble in Br-MeOH (SF rule!)"),
        mk(5, "Sphalerite", "(Zn,Fe)S", 99.0, 1.0, 85.0, 100.0, 90.0, 4.0, "Soluble in Br-MeOH & hot HCl"),
        mk(6, "Galena", "PbS", 99.0, 2.0, 75.0, 100.0, 92.0, 3.0, "High solubility in acid/oxidants"),
        mk(7, "Bornite", "Cu5FeS4", 100.0, 1.5, 10.0, 100.0, 85.0, 70.0, "High NaCN solubility"),
        mk(8, "Chalcocite", "Cu2S", 100.0, 2.0, 12.0, 100.0, 92.0, 90.0, "Very soluble in NaCN"),
        mk(9, "Malachite", "Cu2CO3(OH)2", 12.0, 98.0, 100.0, 100.0, 20.0, 95.0, "Oxide Cu: selective in Citric Acid"),
        mk(10, "Chrysocolla", "Cu2-xAlxH2-xSi2O5(OH)4", 5.0, 82.0, 95.0, 98.0, 10.0, 18.0, "Silicate Cu"),
        mk(11, "Magnetite", "Fe3O4", 0.5, 1.0, 45.0, 98.0, 0.5, 0.0, "Insoluble in Br-MeOH"),
        mk(12, "Olivine", "(Mg,Fe)2SiO4", 0.2, 2.0, 35.0, 80.0, 0.2, 0.0, "Silicate Ni does NOT dissolve in Br-MeOH"),
        mk(13, "Quartz", "SiO2", 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, "Refractory gangue"),
        mk(14, "Calcite", "CaCO3", 2.0, 95.0, 100.0, 100.0, 5.0, 0.0, "Rapid carbonate dissolution"),
    ]
}

// ============================================================================
// Unit Tests (verifying all Ch. 84 examples & math)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_by_properties_pb_s_and_expression() {
        let db = GeoDatabaseStore::load_default();
        assert!(db.minerals.len() > 3800);
        let mut f = PropertySearchFilter::default();
        f.element_states.insert("Pb".into(), ElementFilterState::MustContain);
        f.element_states.insert("S".into(), ElementFilterState::MustContain);
        f.element_states.insert("Ag".into(), ElementFilterState::MustNotContain);

        let expr = f.to_hsc_expression();
        assert!(expr.contains("Contains([Elements], '\\Pb\\')"));
        assert!(expr.contains("Not Contains([Elements], '\\Ag\\')"));

        let hits = filter_minerals_by_properties(&db, &f);
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|&i| db.minerals[i].name == "Galena"));
        assert!(hits.iter().all(|&i| !db.minerals[i].elements.contains("Ag")));
    }

    #[test]
    fn find_by_composition_wssq_identifies_tetrahedrite() {
        let db = GeoDatabaseStore::load_default();
        let criteria = default_eds_criteria();
        let res = find_minerals_by_composition_wssq(&db, &criteria, 10);
        assert_eq!(res.len(), 10);
        assert_eq!(res[0].mineral, "Tetrahedrite");
        assert!(res[0].wssq < 1.0, "Expected very low WSSQ for Ag-tetrahedrite, got {}", res[0].wssq);
    }

    #[test]
    fn database_viewer_filter_and_paste_tsv() {
        let mut db = GeoDatabaseStore::load_default();
        let before = db.mineral_chemistry.len();
        // Exact Image 11 paste example: 7 of 9 columns identified (CuO % and In ppm skipped), 2 of 2 rows added!
        let tsv = "Mineral Name\tLocation\tMineral Key\tFe %\tZn %\tS %\tCd %\tCuO %\tIn ppm\nSphalerite\tTest\ttest\t6.22\t59.9\t32.2\t0.01\t0.02\t200\nPyrite\tTest\ttest\t45\t\t55\t0.01\t\t100\n";
        let rep = db.paste_tsv(GeoDbSource::MineralChemistry, tsv);
        assert_eq!(rep.identified_cols, 7);
        assert_eq!(rep.total_cols, 9);
        assert_eq!(rep.added_rows, 2);
        assert_eq!(rep.total_rows, 2);
        assert_eq!(db.mineral_chemistry.len(), before + 2);
        assert_eq!(db.mineral_chemistry[before].mineral_symbol, "Sp");

        // Exact Image 4/5 filter: Mineral Name Contains Sphalerite AND Location Begins with S
        let conds = vec![
            FilterCondition {
                field: "Mineral Name".into(),
                op: FilterOp::Contains,
                value: "Sphalerite".into(),
            },
            FilterCondition {
                field: "Location".into(),
                op: FilterOp::BeginsWith,
                value: "S".into(),
            },
        ];
        let rows = db.query_rows(GeoDbSource::MineralChemistry, &conds, FilterLogic::And, "");
        assert!(rows.len() >= 8);
        let expr = format_filter_expression(&conds, FilterLogic::And);
        assert!(expr.contains("Starts with([Location], 'S')"));

        // Exact Image 7 Find OR search: "DHZ Biotite" matches DHZ OR Biotite
        let or_rows = db.query_rows(GeoDbSource::MineralChemistry, &[], FilterLogic::And, "DHZ Bushveld");
        assert!(or_rows.len() >= 4);
    }

    #[test]
    fn petrological_bruvann_identify_sf_cipw_cumname_and_xlsx() {
        let proj = PetrologicalProject::default();
        assert!(proj.identified);
        assert!(proj.results.len() >= 10);

        // Verify SF & CUMNAME sheets exist and have 8 Bruvann samples
        let sf = proj.results.iter().find(|r| r.routine == PetroRoutine::Sf).unwrap();
        assert_eq!(sf.rows.len(), 8);
        // Massive sulfide BR-08 should have > 50% total SF
        let br08_sf: f64 = sf.rows[7][10].parse().unwrap();
        assert!(br08_sf > 55.0, "BR-08 massive sulfide SF should exceed 55%, got {br08_sf}");

        let cum = proj.results.iter().find(|r| r.routine == PetroRoutine::Cumname).unwrap();
        assert_eq!(cum.rows.len(), 8);
        // Dunite BR-01 should start with 'o' (olivine cumulus)
        assert!(cum.rows[0][10].starts_with('o'), "Expected BR-01 dunite cumulus to start with 'o', got {}", cum.rows[0][10]);

        let xlsx = proj.export_results_xlsx().unwrap();
        assert!(xlsx.len() > 2000);
    }

    #[test]
    fn modal_cu_s_tutorial_and_monte_carlo() {
        let mut modal = ModalProject::default();
        assert_eq!(modal.results.len(), 4);

        // Sample 1: Cu=0.82%, S=2.55% -> Ccp = 0.82/0.34629 = 2.368%, S_ccp = 0.827%, Py = (2.55-0.827)/0.5345 = 3.223%, Qtz = 94.409%
        let r0 = &modal.results[0];
        assert!((r0.mineral_wt[0] - 2.368).abs() < 0.05, "Ccp got {}", r0.mineral_wt[0]);
        assert!((r0.mineral_wt[1] - 3.223).abs() < 0.05, "Py got {}", r0.mineral_wt[1]);
        assert!((r0.mineral_wt[2] - 94.409).abs() < 0.08, "Qtz got {}", r0.mineral_wt[2]);
        assert!((r0.total_after_norm - 100.0).abs() < 1e-4);
        assert!(r0.sg > 2.65 && r0.sg < 2.90);

        // Monte Carlo Error Estimation
        modal.mc_rounds = 60;
        modal.run_monte_carlo(42);
        let mc = modal.mc_result.as_ref().unwrap();
        assert_eq!(mc.iterations, 60);
        assert!((mc.mean[0][0] - 2.37).abs() < 0.25);
        assert!(mc.stddev[0][0] > 0.01);
        assert!(mc.cv_pct[0][0] > 1.0 && mc.cv_pct[0][0] < 12.0);

        let xlsx = modal.export_xlsx().unwrap();
        assert!(xlsx.len() > 1500);
    }
}
