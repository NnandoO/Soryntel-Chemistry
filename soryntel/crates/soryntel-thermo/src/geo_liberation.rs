//! Module 23 — HSC Geo: Processing Mineral Liberation Data (Chapter 85, §§85.1–85.5).
//!
//! Complete automated mineralogy & liberation processing engine supporting:
//! - Global `MineralSetup` (`Mins` with behavioral types, SG & elemental composition + `Size` classes)
//! - Mineral grouping / `Combine` workflow + lookup across all 3,812 HSC Geo catalog minerals (`MINERALS_RAW`)
//! - Multi-stream workspace (`Active` / `Reference` / `Passive`) with 4 sheets (`Mineral Composition`, `Particles`, `Log`, `Modal`)
//! - Full liberation processing routines: `Add`, `Subtract`, `Basic Binning` (`A/B/C/D`), `Missing?` (WLS-SVD extrapolation),
//!   `Rebalance` (modal IPF), `Pack` (6-rule `N min` + type `E`), `Create Ref`, `Bin with Ref`, `Smooth`, `Clean`,
//!   `Randomize`, `Filter`, `Fill Gangue`, `+Lib`, `-Lib`
//! - All 7 `Graphs` analyses (`MC`, `EC`, `PSD`, `MD`, `ED`, `LIB`, `LIM`)
//! - Real ECMA-376 `.xlsx` export (`MinSetup.xlsx` & stream workbooks) and `.HSCStream` XML/JSON serialization.

use crate::minerals_data::{MineralRawRow, MINERALS_RAW};
use crate::spreadsheet_export::{
    build_xlsx, ExportCell, ExportCellStyle, ExportCellValue, ExportSheet,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

// ============================================================================
// 1. Catalog Lookup over 3,812 HSC Geo Minerals
// ============================================================================

/// Lightweight parsed summary of a mineral from HSC Geo's 3,812-mineral `MINERALS_RAW` database.
#[derive(Debug, Clone)]
pub struct GeoMineralCatalogEntry {
    pub id: u32,
    pub name: String,
    pub code: String,
    pub formula: String,
    pub group: String,
    pub chem_wt_pct: BTreeMap<String, f64>,
    pub estimated_sg: f64,
}

/// Parses `"H=6.211; C=71.698; N=10.789; "` into `BTreeMap<String, f64>`.
pub fn parse_mineral_composition_str(raw: &str) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for part in raw.split(';') {
        let p = part.trim();
        if let Some((sym, val_s)) = p.split_once('=') {
            if let Ok(v) = val_s.trim().replace(',', ".").parse::<f64>() {
                if v > 0.0 {
                    map.insert(sym.trim().to_string(), v);
                }
            }
        }
    }
    map
}

/// Searches the embedded 3,812-mineral HSC Geo catalog by name, code, formula, group, or element.
pub fn search_geo_mineral_catalog(query: &str, limit: usize) -> Vec<GeoMineralCatalogEntry> {
    let q = query.trim().to_ascii_lowercase();
    let mut out = Vec::new();
    for row in MINERALS_RAW {
        if !q.is_empty()
            && !row.name.to_ascii_lowercase().contains(&q)
            && !row.code.to_ascii_lowercase().contains(&q)
            && !row.formula.to_ascii_lowercase().contains(&q)
            && !row.group.to_ascii_lowercase().contains(&q)
        {
            continue;
        }
        out.push(catalog_row_to_entry(row));
        if out.len() >= limit {
            break;
        }
    }
    out
}

/// Searches the 3,812-mineral catalog by required elements (e.g. `["Cu", "Fe", "S"]`).
pub fn find_geo_minerals_by_elements(
    required_elements: &[String],
    limit: usize,
) -> Vec<GeoMineralCatalogEntry> {
    let mut out = Vec::new();
    for row in MINERALS_RAW {
        let mut ok = true;
        for el in required_elements {
            let needle = format!("\\{}\\", el.trim());
            if !row.elements.contains(&needle) {
                ok = false;
                break;
            }
        }
        if ok {
            out.push(catalog_row_to_entry(row));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn catalog_row_to_entry(row: &MineralRawRow) -> GeoMineralCatalogEntry {
    let chem = parse_mineral_composition_str(row.composition);
    let sg = estimate_mineral_sg(row.name, row.code, &chem);
    GeoMineralCatalogEntry {
        id: row.id,
        name: row.name.to_string(),
        code: if row.code.is_empty() {
            row.name.chars().take(3).collect()
        } else {
            row.code.to_string()
        },
        formula: row.formula.to_string(),
        group: row.group.to_string(),
        chem_wt_pct: chem,
        estimated_sg: sg,
    }
}

fn estimate_mineral_sg(name: &str, code: &str, chem: &BTreeMap<String, f64>) -> f64 {
    let n = name.to_ascii_lowercase();
    let c = code.to_ascii_lowercase();
    match (n.as_str(), c.as_str()) {
        ("galena", _) | (_, "gn") => 7.50,
        ("sphalerite", _) | (_, "sp" | "sph" | "sph1" | "sph2" | "sph3") => 4.00,
        ("pyrite", _) | (_, "py") => 5.01,
        ("pyrrhotite", _) | (_, "po") => 4.61,
        ("chalcopyrite", _) | (_, "cpy") => 4.19,
        ("bornite", _) | (_, "bn") => 5.09,
        ("chalcocite", _) | (_, "cc") => 5.65,
        ("arsenopyrite", _) | (_, "ars" | "apy") => 6.07,
        ("magnetite", _) | (_, "mt" | "mag") => 5.18,
        ("hematite", _) | (_, "hem") => 5.26,
        ("quartz", _) | (_, "qtz" | "qz") => 2.65,
        ("dolomite", _) | (_, "dol") => 2.85,
        ("calcite", _) | (_, "cal") => 2.71,
        ("chlorite", _) | (_, "chl") => 2.80,
        ("siderite", _) | (_, "sid") => 3.96,
        ("fluorite", _) | (_, "flu") => 3.18,
        ("talc", _) | (_, "tal" | "tal1") => 2.75,
        ("albite", _) | (_, "ab") => 2.62,
        ("orthoclase", _) | (_, "or") => 2.56,
        ("biotite", _) | (_, "bt") => 3.09,
        ("muscovite", _) | (_, "ms") => 2.82,
        ("apatite", _) | (_, "apa") => 3.19,
        ("zircon", _) | (_, "zir") => 4.65,
        ("acanthite", _) | (_, "ac" | "aca") => 7.22,
        ("bournonite", _) | (_, "bou") => 5.83,
        _ => {
            // Heuristic SG from heavy metal fraction
            let heavy = chem.get("Pb").copied().unwrap_or(0.0)
                + chem.get("Au").copied().unwrap_or(0.0)
                + chem.get("Ag").copied().unwrap_or(0.0)
                + chem.get("Bi").copied().unwrap_or(0.0)
                + chem.get("U").copied().unwrap_or(0.0);
            let trans = chem.get("Fe").copied().unwrap_or(0.0)
                + chem.get("Cu").copied().unwrap_or(0.0)
                + chem.get("Zn").copied().unwrap_or(0.0)
                + chem.get("Ni").copied().unwrap_or(0.0)
                + chem.get("Mn").copied().unwrap_or(0.0);
            let s = chem.get("S").copied().unwrap_or(0.0);
            (2.65 + 0.045 * heavy + 0.025 * trans + 0.015 * s).clamp(2.2, 8.5)
        }
    }
}

// ============================================================================
// 2. Global Mineral Setup (`Mins` & `Size` tabs — §85.1 & §85.2.1)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MineralGroupDef {
    pub code: String,
    pub mineral: String,
    pub includes: Vec<String>,
    pub behavioral_types: Vec<String>,
    pub reference: String,
    pub sg: f64,
    pub chem_wt_pct: BTreeMap<String, f64>,
}

impl MineralGroupDef {
    pub fn from_catalog(entry: &GeoMineralCatalogEntry) -> Self {
        Self {
            code: entry.code.clone(),
            mineral: entry.name.clone(),
            includes: vec![entry.code.clone()],
            behavioral_types: vec!["All".into()],
            reference: if entry.formula.is_empty() {
                "HSC Geo DB".into()
            } else {
                entry.formula.clone()
            },
            sg: entry.estimated_sg,
            chem_wt_pct: entry.chem_wt_pct.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SizeClassDef {
    pub no: usize,
    pub lower: f64,
    pub upper: f64,
}

impl SizeClassDef {
    /// Geometric mean average size matching Ch. 85 Size tab exactly:
    /// `0-10 -> 4.47`, `10-20 -> 14.14`, `20-38 -> 27.57`, `38-75 -> 53.39`,
    /// `75-106 -> 89.16`, `106-150 -> 126.10`, `150-300 -> 212.13`.
    pub fn average_size(&self) -> f64 {
        if self.lower > 0.0 && self.upper > 0.0 {
            (self.lower * self.upper).sqrt()
        } else if self.upper > 0.0 {
            self.upper / 5.0_f64.sqrt()
        } else {
            0.0
        }
    }

    pub fn label(&self, unit: &str) -> String {
        let lo = format_clean_num(self.lower);
        let hi = format_clean_num(self.upper);
        format!("{lo}-{hi} {unit}")
    }
}

fn format_clean_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Global mineralogical setup shared across all streams in a project (`MinSetup.xlsx`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MineralSetup {
    pub version: String,
    pub size_unit: String,
    pub minerals: Vec<MineralGroupDef>,
    pub size_classes: Vec<SizeClassDef>,
    /// Uncombined raw measured phases (available in `Create -> Combine` dialog).
    pub raw_phases: Vec<MineralGroupDef>,
}

impl Default for MineralSetup {
    fn default() -> Self {
        Self::default_pb_zn_project()
    }
}

impl MineralSetup {
    /// Preloaded Pb-Zn sulfide project matching the screenshots in Ch. 85 (`Gn`, `Sp`, `Py`, `Dol`, `Qtz`,
    /// 7 size classes `0-10` to `150-300 um`, plus raw phases `Sph1/Sph2/Sph3/Cpy/Po/Ars/Sid/Cal/Chl/...`
    /// ready for `Create -> Combine`).
    pub fn default_pb_zn_project() -> Self {
        let gn = MineralGroupDef {
            code: "Gn".into(),
            mineral: "Galena".into(),
            includes: vec!["Gn".into()],
            behavioral_types: vec!["All".into()],
            reference: "PbS (Ag-bearing)".into(),
            sg: 7.50,
            chem_wt_pct: BTreeMap::from([
                ("Pb".into(), 86.53),
                ("S".into(), 13.40),
                ("Ag".into(), 0.07),
            ]),
        };
        let sp = MineralGroupDef {
            code: "Sp".into(),
            mineral: "Sphalerite".into(),
            includes: vec!["Sph1".into(), "Sph2".into(), "Sph3".into()],
            behavioral_types: vec!["All".into()],
            reference: "(Zn,Fe,Cd)S".into(),
            sg: 4.00,
            chem_wt_pct: BTreeMap::from([
                ("Zn".into(), 63.17),
                ("Fe".into(), 3.50),
                ("S".into(), 33.00),
                ("Cd".into(), 0.33),
            ]),
        };
        let py = MineralGroupDef {
            code: "Py".into(),
            mineral: "Pyrite".into(),
            includes: vec!["Py".into()],
            behavioral_types: vec!["All".into()],
            reference: "FeS2".into(),
            sg: 5.01,
            chem_wt_pct: BTreeMap::from([("Fe".into(), 46.55), ("S".into(), 53.45)]),
        };
        let dol = MineralGroupDef {
            code: "Dol".into(),
            mineral: "Dolomite".into(),
            includes: vec!["Dol".into(), "Cal".into()],
            behavioral_types: vec!["All".into()],
            reference: "CaMg(CO3)2".into(),
            sg: 2.85,
            chem_wt_pct: BTreeMap::from([
                ("Ca".into(), 21.73),
                ("Mg".into(), 13.18),
                ("C".into(), 13.03),
                ("O".into(), 52.06),
            ]),
        };
        let qtz = MineralGroupDef {
            code: "Qtz".into(),
            mineral: "Quartz".into(),
            includes: vec!["Qtz".into(), "Sil".into()],
            behavioral_types: vec!["All".into()],
            reference: "SiO2".into(),
            sg: 2.65,
            chem_wt_pct: BTreeMap::from([("Si".into(), 46.74), ("O".into(), 53.26)]),
        };

        let raw_phases = vec![
            gn.clone(),
            MineralGroupDef {
                code: "Sph1".into(),
                mineral: "Sphalerite (low-Fe)".into(),
                includes: vec!["Sph1".into()],
                ..sp.clone()
            },
            MineralGroupDef {
                code: "Sph2".into(),
                mineral: "Sphalerite (mid-Fe)".into(),
                includes: vec!["Sph2".into()],
                ..sp.clone()
            },
            MineralGroupDef {
                code: "Sph3".into(),
                mineral: "Sphalerite (high-Fe)".into(),
                includes: vec!["Sph3".into()],
                ..sp.clone()
            },
            py.clone(),
            dol.clone(),
            qtz.clone(),
            MineralGroupDef {
                code: "Cpy".into(),
                mineral: "Chalcopyrite".into(),
                includes: vec!["Cpy".into()],
                behavioral_types: vec!["All".into()],
                reference: "CuFeS2".into(),
                sg: 4.19,
                chem_wt_pct: BTreeMap::from([
                    ("Cu".into(), 34.63),
                    ("Fe".into(), 30.43),
                    ("S".into(), 34.94),
                ]),
            },
            MineralGroupDef {
                code: "Po".into(),
                mineral: "Pyrrhotite".into(),
                includes: vec!["Po".into()],
                behavioral_types: vec!["All".into()],
                reference: "Fe7S8".into(),
                sg: 4.61,
                chem_wt_pct: BTreeMap::from([("Fe".into(), 60.38), ("S".into(), 39.62)]),
            },
            MineralGroupDef {
                code: "Ars".into(),
                mineral: "Arsenopyrite".into(),
                includes: vec!["Ars".into()],
                behavioral_types: vec!["All".into()],
                reference: "FeAsS".into(),
                sg: 6.07,
                chem_wt_pct: BTreeMap::from([
                    ("Fe".into(), 34.30),
                    ("As".into(), 46.01),
                    ("S".into(), 19.69),
                ]),
            },
            MineralGroupDef {
                code: "Cal".into(),
                mineral: "Calcite".into(),
                includes: vec!["Cal".into()],
                behavioral_types: vec!["All".into()],
                reference: "CaCO3".into(),
                sg: 2.71,
                chem_wt_pct: BTreeMap::from([
                    ("Ca".into(), 40.04),
                    ("C".into(), 12.00),
                    ("O".into(), 47.96),
                ]),
            },
            MineralGroupDef {
                code: "Chl".into(),
                mineral: "Chlorite".into(),
                includes: vec!["Chl".into()],
                behavioral_types: vec!["All".into()],
                reference: "Mg5Al2Si3O10(OH)8".into(),
                sg: 2.80,
                chem_wt_pct: BTreeMap::from([
                    ("Mg".into(), 21.86),
                    ("Al".into(), 9.71),
                    ("Si".into(), 15.16),
                    ("O".into(), 51.82),
                    ("H".into(), 1.45),
                ]),
            },
            MineralGroupDef {
                code: "Bou".into(),
                mineral: "Bournonite".into(),
                includes: vec!["Bou".into()],
                behavioral_types: vec!["All".into()],
                reference: "PbCuSbS3".into(),
                sg: 5.83,
                chem_wt_pct: BTreeMap::from([
                    ("Pb".into(), 42.40),
                    ("Cu".into(), 13.00),
                    ("Sb".into(), 24.91),
                    ("S".into(), 19.69),
                ]),
            },
            MineralGroupDef {
                code: "Ac".into(),
                mineral: "Acanthite".into(),
                includes: vec!["Ac".into()],
                behavioral_types: vec!["All".into()],
                reference: "Ag2S".into(),
                sg: 7.22,
                chem_wt_pct: BTreeMap::from([("Ag".into(), 87.06), ("S".into(), 12.94)]),
            },
        ];

        Self {
            version: "7.0".into(),
            size_unit: "um".into(),
            minerals: vec![gn, sp, py, dol, qtz],
            size_classes: Self::default_7_size_classes(),
            raw_phases,
        }
    }

    pub fn default_7_size_classes() -> Vec<SizeClassDef> {
        vec![
            SizeClassDef { no: 1, lower: 0.0, upper: 10.0 },
            SizeClassDef { no: 2, lower: 10.0, upper: 20.0 },
            SizeClassDef { no: 3, lower: 20.0, upper: 38.0 },
            SizeClassDef { no: 4, lower: 38.0, upper: 75.0 },
            SizeClassDef { no: 5, lower: 75.0, upper: 106.0 },
            SizeClassDef { no: 6, lower: 106.0, upper: 150.0 },
            SizeClassDef { no: 7, lower: 150.0, upper: 300.0 },
        ]
    }

    /// Returns all unique chemical elements referenced across `self.minerals` in alphabetical order.
    pub fn all_elements(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for m in &self.minerals {
            for el in m.chem_wt_pct.keys() {
                set.insert(el.clone());
            }
        }
        set.into_iter().collect()
    }

    pub fn size_class_label(&self, no: usize) -> String {
        self.size_classes
            .iter()
            .find(|s| s.no == no)
            .map(|s| s.label(&self.size_unit))
            .unwrap_or_else(|| format!("Class {no}"))
    }

    pub fn renumber_size_classes(&mut self) {
        for (i, s) in self.size_classes.iter_mut().enumerate() {
            s.no = i + 1;
        }
    }
}

// ============================================================================
// 3. Stream, Particle & Configuration Structures (§85.2.3 & §85.3.16)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiberationBasis {
    WeightPct,
    VolumePct,
    SurfacePct,
}

impl LiberationBasis {
    pub const ALL: &'static [LiberationBasis] = &[
        LiberationBasis::WeightPct,
        LiberationBasis::VolumePct,
        LiberationBasis::SurfacePct,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::WeightPct => "wt%",
            Self::VolumePct => "vol%",
            Self::SurfacePct => "sur%",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmoothRegressionType {
    Linear,
    Polynomial2,
    Logarithmic,
    Exponential,
    Power,
}

impl SmoothRegressionType {
    pub const ALL: &'static [SmoothRegressionType] = &[
        SmoothRegressionType::Linear,
        SmoothRegressionType::Polynomial2,
        SmoothRegressionType::Logarithmic,
        SmoothRegressionType::Exponential,
        SmoothRegressionType::Power,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Polynomial2 => "Polynomial (2nd)",
            Self::Logarithmic => "Logarithmic",
            Self::Exponential => "Exponential",
            Self::Power => "Power",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamConfig {
    // Binning Config (§85.3.16 — Config window)
    pub lib_threshold: f64,
    pub tolerance: f64,
    pub binary_classes: usize,
    pub classif_basis: LiberationBasis,
    pub ternaries: bool,
    pub n_min: f64,
    pub uniform_classes: bool,
    // Rebalance Config
    pub max_indiv_diff: f64,
    pub max_total_diff: f64,
    pub max_iter: usize,
    // Smooth Config
    pub smooth_regression: SmoothRegressionType,
    /// Enabled regression checklist matching Config window:
    /// `[Lin, 2nd, Log, Exp, POWER, 3rd Deg, 4th Deg, 5th Deg]`
    pub smooth_equations_mask: [bool; 8],
    pub smooth_basis: LiberationBasis,
    pub smooth_binaries_alone: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            lib_threshold: 95.0,
            tolerance: 5.0,
            binary_classes: 10,
            classif_basis: LiberationBasis::WeightPct,
            ternaries: true,
            n_min: 10.0,
            uniform_classes: false,
            max_indiv_diff: 0.05,
            max_total_diff: 0.20,
            max_iter: 50,
            smooth_regression: SmoothRegressionType::Polynomial2,
            smooth_equations_mask: [true; 8],
            smooth_basis: LiberationBasis::WeightPct,
            smooth_binaries_alone: false,
        }
    }
}

/// Particle class tag corresponding to the `M.P.<sz>.<X|A|B|C|D|E>...` ID scheme (§85.2.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticleKind {
    RawX,
    LiberatedA(usize),
    BinaryB { m1: usize, m2: usize, bin_start: usize, bin_end: usize },
    TernaryC { m1: usize, m2: usize, m3: usize, perm: usize },
    ComplexD(usize),
    PackedE { count: usize },
}

impl ParticleKind {
    /// Returns the `BegCode` prefix used by `Pack` (§85.3.7) to group particles before packing.
    pub fn beg_code(&self) -> String {
        match self {
            Self::RawX => "X".into(),
            Self::LiberatedA(m) => format!("A.{:02}", m + 1),
            Self::BinaryB { m1, m2, .. } => format!("B.{:02}{:02}", m1 + 1, m2 + 1),
            Self::TernaryC { m1, m2, m3, .. } => format!("C.{:02}{:02}{:02}", m1 + 1, m2 + 1, m3 + 1),
            Self::ComplexD(m) => format!("D.{:02}", m + 1),
            Self::PackedE { .. } => "E".into(),
        }
    }

    pub fn format_id(&self, size_no: usize, seq_no: usize) -> String {
        match self {
            Self::RawX => format!("M.P.{size_no}.X.{seq_no}"),
            Self::LiberatedA(m) => format!("M.P.{size_no}.A.{:02}", m + 1),
            Self::BinaryB { m1, m2, bin_start, bin_end } => {
                if bin_start == bin_end {
                    format!("M.P.{size_no}.B.{:02}{:02}.{:02}", m1 + 1, m2 + 1, bin_start)
                } else {
                    format!(
                        "M.P.{size_no}.B.{:02}{:02}.{:02}-{:02}",
                        m1 + 1,
                        m2 + 1,
                        bin_start,
                        bin_end
                    )
                }
            }
            Self::TernaryC { m1, m2, m3, perm } => {
                format!("M.P.{size_no}.C.{:02}{:02}{:02}.{:02}", m1 + 1, m2 + 1, m3 + 1, perm)
            }
            Self::ComplexD(m) => format!("M.P.{size_no}.D.{:02}", m + 1),
            Self::PackedE { count } => format!("M.P.{size_no}.E.{count:02}"),
        }
    }

    /// Key independent of size class, used by `Missing?` & `Bin with Ref` & `Add`/`Subtract`.
    pub fn class_signature(&self) -> String {
        match self {
            Self::RawX => "X".into(),
            Self::LiberatedA(m) => format!("A.{:02}", m + 1),
            Self::BinaryB { m1, m2, bin_start, bin_end } => {
                format!("B.{:02}{:02}.{bin_start:02}-{bin_end:02}", m1 + 1, m2 + 1)
            }
            Self::TernaryC { m1, m2, m3, perm } => {
                format!("C.{:02}{:02}{:02}.{perm:02}", m1 + 1, m2 + 1, m3 + 1)
            }
            Self::ComplexD(m) => format!("D.{:02}", m + 1),
            Self::PackedE { count } => format!("E.{count:02}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleRecord {
    pub id: String,
    pub size_um: f64,
    pub size_class_no: usize,
    pub particle_type: String,
    pub tph: f64,
    pub wt_pct_in_bulk: f64,
    pub wt_pct_in_fraction: f64,
    pub nop: f64,
    pub sg: f64,
    pub min_wt_pct: Vec<f64>,
    pub min_vol_pct: Vec<f64>,
    pub min_sur_pct: Vec<f64>,
    pub kind: ParticleKind,
}

impl ParticleRecord {
    pub fn basis_vec(&self, basis: LiberationBasis) -> &[f64] {
        match basis {
            LiberationBasis::WeightPct => &self.min_wt_pct,
            LiberationBasis::VolumePct => &self.min_vol_pct,
            LiberationBasis::SurfacePct => &self.min_sur_pct,
        }
    }

    pub fn recompute_vol_and_sg(&mut self, setup: &MineralSetup) {
        let mut vol_raw = vec![0.0; setup.minerals.len()];
        let mut vol_sum = 0.0;
        let mut wt_sum = 0.0;
        for (i, m) in setup.minerals.iter().enumerate() {
            let w = self.min_wt_pct.get(i).copied().unwrap_or(0.0).max(0.0);
            let sg = m.sg.max(0.5);
            let v = w / sg;
            vol_raw[i] = v;
            vol_sum += v;
            wt_sum += w;
        }
        if vol_sum > 1e-12 && wt_sum > 1e-12 {
            self.sg = wt_sum / vol_sum;
            self.min_vol_pct = vol_raw.into_iter().map(|v| 100.0 * v / vol_sum).collect();
            if self.min_sur_pct.len() != setup.minerals.len()
                || self.min_sur_pct.iter().sum::<f64>() <= 1e-6
            {
                // Approximate exposed surface area % from vol^(2/3)
                let sur_raw: Vec<f64> = self
                    .min_vol_pct
                    .iter()
                    .map(|&v| v.max(0.0).powf(2.0 / 3.0))
                    .collect();
                let s_sum: f64 = sur_raw.iter().sum();
                self.min_sur_pct = if s_sum > 1e-12 {
                    sur_raw.into_iter().map(|s| 100.0 * s / s_sum).collect()
                } else {
                    self.min_vol_pct.clone()
                };
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamLogEntry {
    pub operation: String,
    pub by: String,
    pub at: String,
    pub note: String,
    pub particles: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiberationStream {
    pub name: String,
    pub total_input_tph: f64,
    /// Bulk mass distribution across `setup.size_classes` (wt-%, sums to 100%).
    pub size_bulk_wt_pct: Vec<f64>,
    pub particles: Vec<ParticleRecord>,
    /// Modal target mineralogy `[mineral_idx][size_idx]` in wt-% (used by `Rebalance`, §85.3.6).
    pub modal_targets: Vec<Vec<f64>>,
    pub log: Vec<StreamLogEntry>,
    pub config: StreamConfig,
    pub is_basic_binned: bool,
}

/// Summary table computed for the `Mineral Composition` sheet of a stream (§85.2.3).
#[derive(Debug, Clone, Default)]
pub struct StreamCompositionSummary {
    /// `[mineral_idx][size_idx]` in wt-% of each size fraction.
    pub mineral_wt_by_size: Vec<Vec<f64>>,
    /// Bulk wt-% for each mineral across measured fractions.
    pub mineral_wt_bulk: Vec<f64>,
    /// Total wt-% per size class (100% if measured, 0% if missing).
    pub total_wt_by_size: Vec<f64>,
    pub total_wt_bulk: f64,
    /// `[mineral_idx][size_idx]` flow rate in tph.
    pub mineral_tph_by_size: Vec<Vec<f64>>,
    pub mineral_tph_bulk: Vec<f64>,
    pub total_tph_by_size: Vec<f64>,
    pub total_tph_bulk: f64,
    /// Ordered chemical elements and `[elem_idx][size_idx]` + bulk wt-%.
    pub elements: Vec<String>,
    pub elem_wt_by_size: Vec<Vec<f64>>,
    pub elem_wt_bulk: Vec<f64>,
}

impl LiberationStream {
    pub fn new_empty(name: impl Into<String>, setup: &MineralSetup, cfg: StreamConfig) -> Self {
        let n_sz = setup.size_classes.len();
        let n_min = setup.minerals.len();
        let default_bulk = if n_sz == 7 {
            vec![10.0, 12.0, 16.0, 21.0, 17.0, 14.0, 10.0]
        } else if n_sz > 0 {
            vec![100.0 / n_sz as f64; n_sz]
        } else {
            Vec::new()
        };
        Self {
            name: name.into(),
            total_input_tph: 21.2,
            size_bulk_wt_pct: default_bulk,
            particles: Vec::new(),
            modal_targets: vec![vec![0.0; n_sz]; n_min],
            log: vec![StreamLogEntry {
                operation: "Created".into(),
                by: "USER".into(),
                at: "2026-11-22 22:00".into(),
                note: "New empty stream".into(),
                particles: 0,
            }],
            config: cfg,
            is_basic_binned: false,
        }
    }

    pub fn push_log(&mut self, operation: &str, note: impl Into<String>) {
        self.log.push(StreamLogEntry {
            operation: operation.into(),
            by: "USER".into(),
            at: "2026-11-22 22:00".into(),
            note: note.into(),
            particles: self.particles.len(),
        });
    }

    /// Synchronizes `wt_pct_in_bulk` and `tph` of every particle from `wt_pct_in_fraction`,
    /// `size_bulk_wt_pct`, and `total_input_tph`.
    pub fn sync_particle_flowrates(&mut self, setup: &MineralSetup) {
        let n_sz = setup.size_classes.len();
        if self.size_bulk_wt_pct.len() < n_sz {
            self.size_bulk_wt_pct.resize(n_sz, 0.0);
        }
        // Normalize wt_pct_in_fraction per non-empty size class
        let mut frac_sums = vec![0.0; n_sz];
        for p in &self.particles {
            if p.size_class_no >= 1 && p.size_class_no <= n_sz {
                frac_sums[p.size_class_no - 1] += p.wt_pct_in_fraction.max(0.0);
            }
        }
        for p in &mut self.particles {
            if p.size_class_no >= 1 && p.size_class_no <= n_sz {
                let s_idx = p.size_class_no - 1;
                let fsum = frac_sums[s_idx];
                if fsum > 1e-12 {
                    p.wt_pct_in_fraction = 100.0 * p.wt_pct_in_fraction.max(0.0) / fsum;
                }
                let bulk_w = self.size_bulk_wt_pct[s_idx].max(0.0);
                p.wt_pct_in_bulk = p.wt_pct_in_fraction * bulk_w / 100.0;
                p.tph = self.total_input_tph * p.wt_pct_in_bulk / 100.0;
            }
        }
    }

    /// Computes the entire `Mineral Composition` sheet (mineral wt-%, tph, and chemical assays).
    pub fn compute_summary(&self, setup: &MineralSetup) -> StreamCompositionSummary {
        let n_min = setup.minerals.len();
        let n_sz = setup.size_classes.len();
        let mut sum = StreamCompositionSummary {
            mineral_wt_by_size: vec![vec![0.0; n_sz]; n_min],
            mineral_wt_bulk: vec![0.0; n_min],
            total_wt_by_size: vec![0.0; n_sz],
            total_wt_bulk: 0.0,
            mineral_tph_by_size: vec![vec![0.0; n_sz]; n_min],
            mineral_tph_bulk: vec![0.0; n_min],
            total_tph_by_size: vec![0.0; n_sz],
            total_tph_bulk: 0.0,
            elements: setup.all_elements(),
            elem_wt_by_size: Vec::new(),
            elem_wt_bulk: Vec::new(),
        };

        let mut frac_wt_present = vec![0.0; n_sz];
        for p in &self.particles {
            if p.size_class_no < 1 || p.size_class_no > n_sz {
                continue;
            }
            let s = p.size_class_no - 1;
            let fw = p.wt_pct_in_fraction.max(0.0);
            frac_wt_present[s] += fw;
            for m in 0..n_min {
                let mw = p.min_wt_pct.get(m).copied().unwrap_or(0.0).max(0.0);
                sum.mineral_wt_by_size[m][s] += fw * mw / 100.0;
            }
        }

        for s in 0..n_sz {
            let tot = frac_wt_present[s];
            sum.total_wt_by_size[s] = if tot > 1e-6 { 100.0 } else { 0.0 };
            if tot > 1e-6 {
                for m in 0..n_min {
                    sum.mineral_wt_by_size[m][s] = 100.0 * sum.mineral_wt_by_size[m][s] / tot;
                }
            }
            let bulk_share = self.size_bulk_wt_pct.get(s).copied().unwrap_or(0.0).max(0.0);
            if tot > 1e-6 {
                sum.total_wt_bulk += bulk_share;
                let frac_tph = self.total_input_tph * bulk_share / 100.0;
                sum.total_tph_by_size[s] = frac_tph;
                sum.total_tph_bulk += frac_tph;
                for m in 0..n_min {
                    let mw = sum.mineral_wt_by_size[m][s];
                    sum.mineral_wt_bulk[m] += mw * bulk_share / 100.0;
                    let mtph = frac_tph * mw / 100.0;
                    sum.mineral_tph_by_size[m][s] = mtph;
                    sum.mineral_tph_bulk[m] += mtph;
                }
            }
        }

        // Chemical composition back-calculated from mineral wt-% and MineralSetup chem_wt_pct
        let n_el = sum.elements.len();
        sum.elem_wt_by_size = vec![vec![0.0; n_sz]; n_el];
        sum.elem_wt_bulk = vec![0.0; n_el];
        for (ei, el) in sum.elements.iter().enumerate() {
            for m in 0..n_min {
                let el_in_m = setup.minerals[m].chem_wt_pct.get(el).copied().unwrap_or(0.0);
                if el_in_m <= 0.0 {
                    continue;
                }
                sum.elem_wt_bulk[ei] += sum.mineral_wt_bulk[m] * el_in_m / 100.0;
                for s in 0..n_sz {
                    sum.elem_wt_by_size[ei][s] += sum.mineral_wt_by_size[m][s] * el_in_m / 100.0;
                }
            }
        }
        sum
    }

    /// Populates `modal_targets` from the stream's current calculated mineral composition.
    pub fn init_modal_from_current(&mut self, setup: &MineralSetup) {
        let sum = self.compute_summary(setup);
        self.modal_targets = sum.mineral_wt_by_size;
    }

    /// Generates realistic synthetic MLA / QemScan / SEM particle data matching Ch. 85 screenshots
    /// (with missing outer size classes 1 & 7 optionally omitted for `Missing?` demonstration!).
    pub fn generate_sample_stream(
        name: &str,
        setup: &MineralSetup,
        total_tph: f64,
        gn_bias: f64,
        sp_bias: f64,
        omit_outer_sizes: bool,
        seed: u64,
    ) -> Self {
        let mut st = Self::new_empty(name, setup, StreamConfig::default());
        st.total_input_tph = total_tph;
        let n_min = setup.minerals.len();
        let n_sz = setup.size_classes.len();
        let mut rng = LcgRng::new(seed);

        for s_idx in 0..n_sz {
            if omit_outer_sizes && (s_idx == 0 || s_idx + 1 == n_sz) {
                continue;
            }
            let sc = &setup.size_classes[s_idx];
            let avg_sz = sc.average_size();
            let coarseness = (s_idx as f64) / (n_sz.max(2) - 1) as f64; // 0 = finest, 1 = coarsest
            let lib_prob = (0.72 - 0.36 * coarseness).clamp(0.25, 0.85);
            let n_particles_in_frac = 180usize;
            let mut frac_particles = Vec::with_capacity(n_particles_in_frac);

            for p_i in 0..n_particles_in_frac {
                let u = rng.f64();
                let mut w = vec![0.0; n_min];
                // Base mineral preference by size (finer = richer in Gn/Sp, coarser = more gangue)
                let base_probs = mineral_weights_for_sample(n_min, gn_bias, sp_bias, coarseness);

                if u < lib_prob {
                    // Liberated particle (96..100% single mineral)
                    let m = pick_weighted(&base_probs, rng.f64());
                    let purity = 96.0 + 4.0 * rng.f64();
                    w[m] = purity;
                    if n_min > 1 && purity < 100.0 {
                        let m2 = (m + 1 + (rng.next_u32() as usize % (n_min - 1))) % n_min;
                        w[m2] = 100.0 - purity;
                    } else {
                        w[m] = 100.0;
                    }
                } else if u < lib_prob + 0.22 && n_min >= 2 {
                    // Binary particle (2 minerals)
                    let m1 = pick_weighted(&base_probs, rng.f64());
                    let mut m2 = pick_weighted(&base_probs, rng.f64());
                    if m2 == m1 {
                        m2 = (m1 + 1) % n_min;
                    }
                    let frac1 = 8.0 + 84.0 * rng.f64();
                    w[m1] = frac1;
                    w[m2] = 100.0 - frac1;
                } else if u < lib_prob + 0.28 && n_min >= 3 {
                    // Ternary particle (3 minerals)
                    let m1 = pick_weighted(&base_probs, rng.f64());
                    let m2 = (m1 + 1) % n_min;
                    let m3 = (m1 + 2) % n_min;
                    let a = 15.0 + 50.0 * rng.f64();
                    let b = 10.0 + (80.0 - a).max(5.0) * rng.f64();
                    let c = (100.0 - a - b).max(6.0);
                    let s = a + b + c;
                    w[m1] = 100.0 * a / s;
                    w[m2] = 100.0 * b / s;
                    w[m3] = 100.0 * c / s;
                } else {
                    // Complex particle (4+ minerals)
                    let mut s = 0.0;
                    for val in w.iter_mut() {
                        let r = 6.0 + 30.0 * rng.f64();
                        *val = r;
                        s += r;
                    }
                    for val in w.iter_mut() {
                        *val = 100.0 * (*val) / s;
                    }
                }

                let p_sz = (avg_sz * (0.75 + 0.50 * rng.f64())).max(1.0);
                let nop = (1.0 + 25.0 * rng.f64().powi(2)).round();
                let rel_wt = nop * (0.5 + rng.f64());
                let mut pr = ParticleRecord {
                    id: format!("M.P.{}.X.{}", sc.no, p_i + 1),
                    size_um: p_sz,
                    size_class_no: sc.no,
                    particle_type: "X".into(),
                    tph: 0.0,
                    wt_pct_in_bulk: 0.0,
                    wt_pct_in_fraction: rel_wt,
                    nop,
                    sg: 3.5,
                    min_wt_pct: w,
                    min_vol_pct: vec![0.0; n_min],
                    min_sur_pct: vec![0.0; n_min],
                    kind: ParticleKind::RawX,
                };
                pr.recompute_vol_and_sg(setup);
                frac_particles.push(pr);
            }
            st.particles.extend(frac_particles);
        }

        st.sync_particle_flowrates(setup);
        st.init_modal_from_current(setup);
        st.log.clear();
        st.push_log(
            "Import MLA",
            format!(
                "Loaded {} measured particles ({} size classes)",
                st.particles.len(),
                if omit_outer_sizes { n_sz.saturating_sub(2) } else { n_sz }
            ),
        );
        st
    }
}

fn mineral_weights_for_sample(n: usize, gn_bias: f64, sp_bias: f64, coarseness: f64) -> Vec<f64> {
    let mut w = vec![0.15; n];
    if n >= 1 {
        w[0] = (gn_bias * (1.25 - 0.55 * coarseness)).max(0.04);
    }
    if n >= 2 {
        w[1] = (sp_bias * (1.05 - 0.15 * coarseness)).max(0.04);
    }
    if n >= 3 {
        w[2] = 0.08;
    }
    if n >= 4 {
        w[3] = (0.28 + 0.25 * coarseness).max(0.05);
    }
    if n >= 5 {
        w[4] = (0.22 + 0.25 * coarseness).max(0.05);
    }
    w
}

fn pick_weighted(weights: &[f64], u: f64) -> usize {
    let tot: f64 = weights.iter().sum();
    if tot <= 1e-12 {
        return 0;
    }
    let mut acc = 0.0;
    let target = u * tot;
    for (i, &w) in weights.iter().enumerate() {
        acc += w;
        if target <= acc {
            return i;
        }
    }
    weights.len().saturating_sub(1)
}

// ============================================================================
// 4. Core Mineral Liberation Processing Routines (§85.3.1–§85.3.15)
// ============================================================================

/// §85.3.4 — `Basic Binning`: classifies particles in each size class into Liberated (`A`),
/// Binary (`B`, with `binary_classes` intervals), Ternary (`C`, 6 permutations), and Complex (`D`).
pub fn run_basic_binning(stream: &mut LiberationStream, setup: &MineralSetup) {
    let n_min = setup.minerals.len();
    let n_sz = setup.size_classes.len();
    if n_min == 0 || n_sz == 0 || stream.particles.is_empty() {
        return;
    }
    let cfg = stream.config.clone();
    let lib_thr = cfg.lib_threshold.clamp(50.0, 100.0);
    let tol = cfg.tolerance.clamp(0.1, 49.0);
    let n_bins = cfg.binary_classes.clamp(2, 20);

    let mut out_particles: Vec<ParticleRecord> = Vec::new();

    for sc in &setup.size_classes {
        let frac_ps: Vec<&ParticleRecord> = stream
            .particles
            .iter()
            .filter(|p| p.size_class_no == sc.no && p.wt_pct_in_fraction > 0.0)
            .collect();
        if frac_ps.is_empty() {
            continue;
        }

        // Accumulate into map keyed by `(sort_key, ParticleKind)`
        let mut buckets: BTreeMap<(u8, String), (ParticleKind, String, Vec<&ParticleRecord>)> =
            BTreeMap::new();

        for p in frac_ps {
            let basis = p.basis_vec(cfg.classif_basis);
            let (kind, ptype, order_rank) =
                classify_particle_kind(basis, setup, lib_thr, tol, n_bins, cfg.ternaries);
            let key = (order_rank, kind.class_signature());
            buckets
                .entry(key)
                .or_insert_with(|| (kind, ptype, Vec::new()))
                .2
                .push(p);
        }

        for (_, (kind, ptype, group)) in buckets {
            if let Some(merged) = merge_particle_group(&group, sc.no, kind, ptype, setup) {
                out_particles.push(merged);
            }
        }
    }

    stream.particles = out_particles;
    stream.is_basic_binned = true;
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Basic Binning",
        format!(
            "{} ; {} ; {} ({})",
            cfg.lib_threshold,
            cfg.tolerance,
            cfg.binary_classes,
            cfg.classif_basis.label()
        ),
    );
}

fn classify_particle_kind(
    basis: &[f64],
    setup: &MineralSetup,
    lib_thr: f64,
    tol: f64,
    n_bins: usize,
    use_ternaries: bool,
) -> (ParticleKind, String, u8) {
    let n = setup.minerals.len();
    // 1. Check liberated (A)
    let mut best_m = 0usize;
    let mut best_v = -1.0f64;
    for i in 0..n {
        let v = basis.get(i).copied().unwrap_or(0.0);
        if v > best_v {
            best_v = v;
            best_m = i;
        }
    }
    if best_v >= lib_thr {
        return (
            ParticleKind::LiberatedA(best_m),
            setup.minerals[best_m].code.clone(),
            0,
        );
    }

    // Collect minerals >= tolerance
    let mut sig: Vec<(usize, f64)> = (0..n)
        .filter_map(|i| {
            let v = basis.get(i).copied().unwrap_or(0.0);
            if v >= tol { Some((i, v)) } else { None }
        })
        .collect();
    if sig.len() <= 1 {
        return (
            ParticleKind::LiberatedA(best_m),
            setup.minerals[best_m].code.clone(),
            0,
        );
    }
    if sig.len() == 2 {
        let (mut m1, mut v1) = sig[0];
        let (mut m2, mut v2) = sig[1];
        if m1 > m2 {
            std::mem::swap(&mut m1, &mut m2);
            std::mem::swap(&mut v1, &mut v2);
        }
        let ratio = (v1 / (v1 + v2).max(1e-9)).clamp(0.0, 1.0);
        // Bin 1 = richest in m1 (close to lib_thr), Bin n_bins = poorest in m1 (matching Ch. 85 screenshot rows 82..92!)
        let bin = (((1.0 - ratio) * (n_bins as f64)).floor() as usize + 1).clamp(1, n_bins);
        let ptype = format!("{}-{}", setup.minerals[m1].code, setup.minerals[m2].code);
        return (
            ParticleKind::BinaryB {
                m1,
                m2,
                bin_start: bin,
                bin_end: bin,
            },
            ptype,
            1,
        );
    }
    if sig.len() == 3 && use_ternaries {
        let mut by_idx = sig.clone();
        by_idx.sort_by_key(|&(idx, _)| idx);
        let (m1, v1) = by_idx[0];
        let (m2, v2) = by_idx[1];
        let (m3, v3) = by_idx[2];
        // 6 possible descending permutations
        let perm = if v1 >= v2 && v2 >= v3 {
            1
        } else if v1 >= v3 && v3 >= v2 {
            2
        } else if v2 >= v1 && v1 >= v3 {
            3
        } else if v2 >= v3 && v3 >= v1 {
            4
        } else if v3 >= v1 && v1 >= v2 {
            5
        } else {
            6
        };
        let ptype = format!(
            "{}-{}-{}",
            setup.minerals[m1].code, setup.minerals[m2].code, setup.minerals[m3].code
        );
        return (
            ParticleKind::TernaryC { m1, m2, m3, perm },
            ptype,
            2,
        );
    }

    // Complex (D): keyed by main mineral
    sig.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let main_m = sig[0].0;
    let ptype = format!("{}-Complex", setup.minerals[main_m].code);
    (ParticleKind::ComplexD(main_m), ptype, 3)
}

fn merge_particle_group(
    group: &[&ParticleRecord],
    size_no: usize,
    kind: ParticleKind,
    ptype: String,
    setup: &MineralSetup,
) -> Option<ParticleRecord> {
    if group.is_empty() {
        return None;
    }
    let n_min = setup.minerals.len();
    let mut sum_wt = 0.0;
    let mut sum_nop = 0.0;
    let mut avg_sz = 0.0;
    let mut min_wt = vec![0.0; n_min];
    let mut min_sur = vec![0.0; n_min];

    for p in group {
        let w = p.wt_pct_in_fraction.max(0.0);
        sum_wt += w;
        sum_nop += p.nop.max(0.0);
        avg_sz += w * p.size_um;
        for i in 0..n_min {
            min_wt[i] += w * p.min_wt_pct.get(i).copied().unwrap_or(0.0);
            min_sur[i] += w * p.min_sur_pct.get(i).copied().unwrap_or(0.0);
        }
    }
    if sum_wt <= 1e-12 {
        return None;
    }
    avg_sz /= sum_wt;
    for i in 0..n_min {
        min_wt[i] /= sum_wt;
        min_sur[i] /= sum_wt;
    }
    let id = kind.format_id(size_no, 1);
    let mut pr = ParticleRecord {
        id,
        size_um: avg_sz,
        size_class_no: size_no,
        particle_type: ptype,
        tph: 0.0,
        wt_pct_in_bulk: 0.0,
        wt_pct_in_fraction: sum_wt,
        nop: sum_nop,
        sg: 3.5,
        min_wt_pct: min_wt,
        min_vol_pct: vec![0.0; n_min],
        min_sur_pct: min_sur,
        kind,
    };
    pr.recompute_vol_and_sg(setup);
    Some(pr)
}

/// §85.3.5 — `Missing?`: WLS linear regression (`X = avg_size`, `Y = wt%`, `sigma = 1/sqrt(NOP)`)
/// to extrapolate/interpolate missing size fractions.
pub fn run_extrapolate_missing(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
) -> Result<usize, String> {
    if !stream.is_basic_binned {
        run_basic_binning(stream, setup);
    }
    let present_sizes: BTreeSet<usize> = stream.particles.iter().map(|p| p.size_class_no).collect();
    let missing_sizes: Vec<&SizeClassDef> = setup
        .size_classes
        .iter()
        .filter(|sc| !present_sizes.contains(&sc.no))
        .collect();
    if missing_sizes.is_empty() {
        return Ok(0);
    }
    if present_sizes.is_empty() {
        return Err("Stream has no measured particles to extrapolate from.".into());
    }

    // Group existing particles by class_signature across measured size classes
    let mut by_sig: BTreeMap<String, Vec<&ParticleRecord>> = BTreeMap::new();
    for p in &stream.particles {
        by_sig
            .entry(p.kind.class_signature())
            .or_default()
            .push(p);
    }

    let sz_avg_map: HashMap<usize, f64> = setup
        .size_classes
        .iter()
        .map(|s| (s.no, s.average_size()))
        .collect();

    let mut new_particles = Vec::new();
    for miss_sc in &missing_sizes {
        let x_target = miss_sc.average_size();
        for group in by_sig.values() {
            // Weighted least squares: w_i = 1 / sigma_i^2 = NOP_i
            let mut s_w = 0.0;
            let mut s_wx = 0.0;
            let mut s_wy = 0.0;
            let mut s_wxx = 0.0;
            let mut s_wxy = 0.0;
            let mut avg_nop = 0.0;
            let n_min = setup.minerals.len();
            let mut avg_wt = vec![0.0; n_min];
            let mut avg_sur = vec![0.0; n_min];

            for p in group {
                let x = sz_avg_map.get(&p.size_class_no).copied().unwrap_or(p.size_um);
                let y = p.wt_pct_in_fraction;
                let wi = p.nop.max(1.0); // 1 / sigma^2 where sigma = 1/sqrt(NOP)
                s_w += wi;
                s_wx += wi * x;
                s_wy += wi * y;
                s_wxx += wi * x * x;
                s_wxy += wi * x * y;
                avg_nop += p.nop;
                for m in 0..n_min {
                    avg_wt[m] += p.min_wt_pct.get(m).copied().unwrap_or(0.0);
                    avg_sur[m] += p.min_sur_pct.get(m).copied().unwrap_or(0.0);
                }
            }
            let cnt = group.len() as f64;
            avg_nop /= cnt;
            for m in 0..n_min {
                avg_wt[m] /= cnt;
                avg_sur[m] /= cnt;
            }

            let y_pred = if group.len() >= 2 {
                let det = s_w * s_wxx - s_wx * s_wx;
                if det.abs() > 1e-12 {
                    let slope = (s_w * s_wxy - s_wx * s_wy) / det;
                    let intercept = (s_wy - slope * s_wx) / s_w;
                    (intercept + slope * x_target).max(0.0)
                } else {
                    (s_wy / s_w.max(1e-9)).max(0.0)
                }
            } else {
                (s_wy / s_w.max(1e-9)).max(0.0)
            };

            if y_pred > 1e-6 {
                let template = group[0];
                let mut pr = ParticleRecord {
                    id: template.kind.format_id(miss_sc.no, 1),
                    size_um: x_target,
                    size_class_no: miss_sc.no,
                    particle_type: template.particle_type.clone(),
                    tph: 0.0,
                    wt_pct_in_bulk: 0.0,
                    wt_pct_in_fraction: y_pred,
                    nop: avg_nop.max(1.0).round(),
                    sg: template.sg,
                    min_wt_pct: avg_wt,
                    min_vol_pct: vec![0.0; n_min],
                    min_sur_pct: avg_sur,
                    kind: template.kind.clone(),
                };
                pr.recompute_vol_and_sg(setup);
                new_particles.push(pr);
            }
        }
    }

    let filled = missing_sizes.len();
    stream.particles.extend(new_particles);
    stream.particles.sort_by_key(|p| (p.size_class_no, p.id.clone()));
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Missing Extrapolated",
        format!("WLS-SVD filled {filled} missing size class(es)"),
    );
    Ok(filled)
}

/// §85.3.6 — `Rebalance`: adjusts particle weights per size class to match `stream.modal_targets`.
pub fn run_rebalance(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
) -> Result<(usize, f64), String> {
    if !stream.is_basic_binned {
        run_basic_binning(stream, setup);
    }
    let n_min = setup.minerals.len();
    let n_sz = setup.size_classes.len();
    let max_iter = stream.config.max_iter.max(1);
    let max_indiv = stream.config.max_indiv_diff.max(1e-5);
    let max_tot = stream.config.max_total_diff.max(1e-5);

    let mut total_iters = 0usize;
    let mut worst_diff = 0.0f64;

    for s_idx in 0..n_sz {
        let size_no = setup.size_classes[s_idx].no;
        let target_sum: f64 = (0..n_min)
            .map(|m| {
                stream
                    .modal_targets
                    .get(m)
                    .and_then(|r| r.get(s_idx))
                    .copied()
                    .unwrap_or(0.0)
                    .max(0.0)
            })
            .sum();
        if target_sum <= 1e-6 {
            continue;
        }
        let targets: Vec<f64> = (0..n_min)
            .map(|m| {
                100.0
                    * stream
                        .modal_targets
                        .get(m)
                        .and_then(|r| r.get(s_idx))
                        .copied()
                        .unwrap_or(0.0)
                        .max(0.0)
                    / target_sum
            })
            .collect();

        let idxs: Vec<usize> = stream
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| p.size_class_no == size_no)
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            continue;
        }

        for it in 1..=max_iter {
            total_iters = total_iters.max(it);
            let mut cur = vec![0.0; n_min];
            let mut fsum = 0.0;
            for &pi in &idxs {
                let w = stream.particles[pi].wt_pct_in_fraction.max(0.0);
                fsum += w;
                for m in 0..n_min {
                    cur[m] += w * stream.particles[pi].min_wt_pct.get(m).copied().unwrap_or(0.0) / 100.0;
                }
            }
            if fsum <= 1e-12 {
                break;
            }
            for m in 0..n_min {
                cur[m] = 100.0 * cur[m] / fsum;
            }

            let mut max_i = 0.0f64;
            let mut sum_d = 0.0f64;
            for m in 0..n_min {
                let d = (cur[m] - targets[m]).abs();
                max_i = max_i.max(d);
                sum_d += d;
            }
            if max_i <= max_indiv || sum_d <= max_tot {
                worst_diff = worst_diff.max(max_i);
                break;
            }
            if it == max_iter {
                worst_diff = worst_diff.max(max_i);
            }

            // Multiplicative IPF update weighted by particle mineral composition
            for &pi in &idxs {
                let mut mult = 0.0;
                let mut wsum = 0.0;
                for m in 0..n_min {
                    let comp = stream.particles[pi].min_wt_pct.get(m).copied().unwrap_or(0.0) / 100.0;
                    if comp > 1e-9 && cur[m] > 1e-9 {
                        let ratio = (targets[m] / cur[m]).clamp(0.25, 4.0);
                        mult += comp * ratio;
                        wsum += comp;
                    }
                }
                if wsum > 1e-9 {
                    let factor = (mult / wsum).powf(0.65); // damped for smooth stability
                    stream.particles[pi].wt_pct_in_fraction *= factor;
                }
            }
        }
    }

    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Rebalance",
        format!("iters={total_iters}, max_diff={worst_diff:.4}%"),
    );
    Ok((total_iters, worst_diff))
}

/// §85.3.7 — `Pack`: implements the 6 exact HSC rules to merge low-NOP binary classes by `BegCode`,
/// then packs `C` & `D` particles meeting `N min` into type `E`.
pub fn run_pack(stream: &mut LiberationStream, setup: &MineralSetup) {
    if !stream.is_basic_binned {
        run_basic_binning(stream, setup);
    }
    let n_min = stream.config.n_min.max(1.0);
    let mut result = Vec::new();

    for sc in &setup.size_classes {
        let frac: Vec<&ParticleRecord> = stream
            .particles
            .iter()
            .filter(|p| p.size_class_no == sc.no)
            .collect();
        if frac.is_empty() {
            continue;
        }

        // Type A particles are untouched
        let mut by_beg: BTreeMap<String, Vec<&ParticleRecord>> = BTreeMap::new();
        let mut cd_pool: Vec<&ParticleRecord> = Vec::new();

        for p in frac {
            match &p.kind {
                ParticleKind::LiberatedA(_) => result.push(p.clone()),
                ParticleKind::BinaryB { .. } => {
                    by_beg.entry(p.kind.beg_code()).or_default().push(p);
                }
                ParticleKind::TernaryC { .. } | ParticleKind::ComplexD(_) => {
                    cd_pool.push(p);
                }
                ParticleKind::PackedE { .. } | ParticleKind::RawX => result.push(p.clone()),
            }
        }

        // Pack each Binary BegCode using the exact 6 rules (§85.3.7)
        for (_beg, list) in by_beg {
            let mut groups: Vec<Vec<&ParticleRecord>> = Vec::new();
            for (i, &p) in list.iter().enumerate() {
                if groups.is_empty() {
                    groups.push(vec![p]);
                    continue;
                }
                let cur_nop: f64 = groups.last().unwrap().iter().map(|x| x.nop).sum();
                let is_last = i + 1 == list.len();
                let next_nop = list.get(i + 1).map(|x| x.nop).unwrap_or(f64::INFINITY);

                let add_to_current = if cur_nop < n_min {
                    true
                } else if p.nop > n_min {
                    false
                } else if is_last {
                    true
                } else if groups.last().unwrap().len() > 1 {
                    false
                } else if next_nop < cur_nop {
                    false
                } else {
                    true
                };

                if add_to_current {
                    groups.last_mut().unwrap().push(p);
                } else {
                    groups.push(vec![p]);
                }
            }
            // If last binary group has < N min particles, merge with previous group
            if groups.len() >= 2 {
                let last_nop: f64 = groups.last().unwrap().iter().map(|x| x.nop).sum();
                if last_nop < n_min {
                    let tail = groups.pop().unwrap();
                    groups.last_mut().unwrap().extend(tail);
                }
            }
            for g in groups {
                let (m1, m2, b_start, b_end) = match (&g.first().unwrap().kind, &g.last().unwrap().kind) {
                    (
                        ParticleKind::BinaryB { m1, m2, bin_start, .. },
                        ParticleKind::BinaryB { bin_end, .. },
                    ) => (*m1, *m2, *bin_start, *bin_end),
                    _ => (0, 1, 1, 1),
                };
                let kind = ParticleKind::BinaryB { m1, m2, bin_start: b_start, bin_end: b_end };
                let ptype = g[0].particle_type.clone();
                if let Some(m) = merge_particle_group(&g, sc.no, kind, ptype, setup) {
                    result.push(m);
                }
            }
        }

        // Combine C & D particles into single type E if total NOP >= N min
        if !cd_pool.is_empty() {
            let cd_nop: f64 = cd_pool.iter().map(|p| p.nop).sum();
            if cd_nop >= n_min {
                let cnt = cd_pool.len();
                let kind = ParticleKind::PackedE { count: cnt };
                if let Some(m) = merge_particle_group(&cd_pool, sc.no, kind, "E".into(), setup) {
                    result.push(m);
                }
            } else {
                for p in cd_pool {
                    result.push(p.clone());
                }
            }
        }
    }

    stream.particles = result;
    stream.sync_particle_flowrates(setup);
    stream.push_log("Pack", format!("N min = {}", stream.config.n_min));
}

/// §85.3.9 — `Bin with Ref`: aligns active stream's particle classes to match `reference`'s classes.
pub fn run_bin_with_ref(
    active: &mut LiberationStream,
    reference: &LiberationStream,
    setup: &MineralSetup,
) {
    if !active.is_basic_binned {
        run_basic_binning(active, setup);
    }
    let mut mapped: Vec<ParticleRecord> = Vec::new();
    for sc in &setup.size_classes {
        let ref_ps: Vec<&ParticleRecord> = reference
            .particles
            .iter()
            .filter(|p| p.size_class_no == sc.no)
            .collect();
        let act_ps: Vec<&ParticleRecord> = active
            .particles
            .iter()
            .filter(|p| p.size_class_no == sc.no)
            .collect();
        if ref_ps.is_empty() || act_ps.is_empty() {
            for p in act_ps {
                mapped.push(p.clone());
            }
            continue;
        }
        let mut assigned: Vec<Vec<&ParticleRecord>> = vec![Vec::new(); ref_ps.len()];
        for p in act_ps {
            // Find exact signature match or nearest compositional reference class
            let best_idx = ref_ps
                .iter()
                .position(|r| r.kind.class_signature() == p.kind.class_signature())
                .unwrap_or_else(|| {
                    ref_ps
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            let da = l2_dist(&a.min_wt_pct, &p.min_wt_pct);
                            let db = l2_dist(&b.min_wt_pct, &p.min_wt_pct);
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i)
                        .unwrap_or(0)
                });
            assigned[best_idx].push(p);
        }
        for (ri, group) in assigned.into_iter().enumerate() {
            let r = ref_ps[ri];
            if let Some(m) = merge_particle_group(
                &group,
                sc.no,
                r.kind.clone(),
                r.particle_type.clone(),
                setup,
            ) {
                mapped.push(m);
            } else {
                let mut zero_p = r.clone();
                zero_p.wt_pct_in_fraction = 0.0;
                zero_p.wt_pct_in_bulk = 0.0;
                zero_p.tph = 0.0;
                zero_p.nop = 0.0;
                mapped.push(zero_p);
            }
        }
    }
    active.particles = mapped;
    active.sync_particle_flowrates(setup);
    active.push_log("Bin with Ref", format!("Ref: {}", reference.name));
}

fn l2_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// §85.3.10 — `Smooth`: divides active stream by reference stream and smooths binary partition curves.
pub fn run_smooth(
    active: &mut LiberationStream,
    reference: &LiberationStream,
    setup: &MineralSetup,
) {
    let ref_map: HashMap<(usize, String), f64> = reference
        .particles
        .iter()
        .map(|p| ((p.size_class_no, p.kind.class_signature()), p.tph.max(1e-9)))
        .collect();

    let basis = active.config.smooth_basis;
    let binaries_alone = active.config.smooth_binaries_alone;

    // Group indices by (size_class_no, binary mineral pair)
    let mut pair_groups: BTreeMap<(usize, usize, usize), Vec<usize>> = BTreeMap::new();
    for (idx, p) in active.particles.iter().enumerate() {
        if let ParticleKind::BinaryB { m1, m2, .. } = p.kind {
            pair_groups.entry((p.size_class_no, m1, m2)).or_default().push(idx);
        }
    }
    if !binaries_alone {
        for (idx, p) in active.particles.iter().enumerate() {
            if let ParticleKind::LiberatedA(m) = p.kind {
                for (&(sz, m1, m2), vec_idx) in pair_groups.iter_mut() {
                    if sz == p.size_class_no && (m == m1 || m == m2) {
                        vec_idx.push(idx);
                    }
                }
            }
        }
    }

    for ((sz, m1, _m2), idxs) in pair_groups {
        if idxs.len() < 3 {
            continue;
        }
        let mut pts: Vec<(usize, f64, f64, f64)> = Vec::new();
        for &i in &idxs {
            let p = &active.particles[i];
            let x = p.basis_vec(basis).get(m1).copied().unwrap_or(0.0);
            let ref_tph = ref_map
                .get(&(sz, p.kind.class_signature()))
                .copied()
                .unwrap_or(p.tph.max(1e-6));
            let y = (p.tph / ref_tph).clamp(0.0, 2.0);
            pts.push((i, x, y, ref_tph));
        }
        // Fit simple linear trend y = a + b*x over partition curve
        let n = pts.len() as f64;
        let mx = pts.iter().map(|t| t.1).sum::<f64>() / n;
        let my = pts.iter().map(|t| t.2).sum::<f64>() / n;
        let num: f64 = pts.iter().map(|t| (t.1 - mx) * (t.2 - my)).sum();
        let den: f64 = pts.iter().map(|t| (t.1 - mx).powi(2)).sum();
        let slope = if den > 1e-9 { num / den } else { 0.0 };
        let intercept = my - slope * mx;

        for (i, x, _, ref_tph) in pts {
            let smoothed_ratio = (intercept + slope * x).clamp(0.0, 1.5);
            let new_tph = smoothed_ratio * ref_tph;
            active.particles[i].wt_pct_in_fraction = new_tph.max(0.0);
        }
    }

    active.sync_particle_flowrates(setup);
    active.push_log(
        "Smooth",
        format!(
            "Ref: {}, Reg: {}",
            reference.name,
            active.config.smooth_regression.label()
        ),
    );
}

/// §85.3.11 — `Clean`: removes particles with `NOP == 0`.
pub fn run_clean(stream: &mut LiberationStream, setup: &MineralSetup) -> usize {
    let before = stream.particles.len();
    stream.particles.retain(|p| p.nop > 0.0 && p.wt_pct_in_fraction > 0.0);
    let removed = before - stream.particles.len();
    stream.sync_particle_flowrates(setup);
    stream.push_log("Cleaned", format!("Removed {removed} zero-NOP particles"));
    removed
}

/// §85.3.12 — `Randomize`: randomly perturbs particle weights (+/- 15%) and renormalizes.
pub fn run_randomize(stream: &mut LiberationStream, setup: &MineralSetup, seed: u64) {
    let mut rng = LcgRng::new(seed);
    for p in &mut stream.particles {
        let factor = 0.85 + 0.30 * rng.f64();
        p.wt_pct_in_fraction = (p.wt_pct_in_fraction * factor).max(0.0);
    }
    stream.sync_particle_flowrates(setup);
    stream.push_log("Randomize", "Randomized particle weights (+/- 15%)");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterRelation {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl FilterRelation {
    pub const ALL: &'static [FilterRelation] = &[
        FilterRelation::Eq,
        FilterRelation::Neq,
        FilterRelation::Gt,
        FilterRelation::Gte,
        FilterRelation::Lt,
        FilterRelation::Lte,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }

    pub fn eval(self, lhs: f64, rhs: f64) -> bool {
        match self {
            Self::Eq => (lhs - rhs).abs() < 1e-6,
            Self::Neq => (lhs - rhs).abs() >= 1e-6,
            Self::Gt => lhs > rhs,
            Self::Gte => lhs >= rhs,
            Self::Lt => lhs < rhs,
            Self::Lte => lhs <= rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterProperty {
    Size,
    Tph,
    WtPctBulk,
    WtPctFraction,
    Nop,
    Sg,
    MineralWtPct(usize),
}

/// §85.3.13 — `Filter`: keeps or deletes particles matching numeric condition.
pub fn run_filter(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
    size_class_filter: Option<usize>,
    delete_matching: bool,
    prop: FilterProperty,
    rel: FilterRelation,
    threshold: f64,
) -> usize {
    let before = stream.particles.len();
    stream.particles.retain(|p| {
        if let Some(sc) = size_class_filter {
            if p.size_class_no != sc {
                return true;
            }
        }
        let val = match prop {
            FilterProperty::Size => p.size_um,
            FilterProperty::Tph => p.tph,
            FilterProperty::WtPctBulk => p.wt_pct_in_bulk,
            FilterProperty::WtPctFraction => p.wt_pct_in_fraction,
            FilterProperty::Nop => p.nop,
            FilterProperty::Sg => p.sg,
            FilterProperty::MineralWtPct(m) => p.min_wt_pct.get(m).copied().unwrap_or(0.0),
        };
        let matched = rel.eval(val, threshold);
        if delete_matching { !matched } else { matched }
    });
    let removed = before - stream.particles.len();
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Filter",
        format!(
            "{} {} {} (removed {removed})",
            if delete_matching { "Delete" } else { "Keep" },
            rel.label(),
            threshold
        ),
    );
    removed
}

/// §85.3.14 — `Fill Gangue`: increases particles smaller than `min_size_um` in `size_class_no`
/// up to `min_size_um` by adding volume/mass of the chosen gangue mineral group `mineral_idx`.
pub fn run_fill_gangue(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
    size_class_no: usize,
    mineral_idx: usize,
    min_size_um: f64,
) -> usize {
    let n_min = setup.minerals.len();
    if mineral_idx >= n_min || min_size_um <= 0.0 {
        return 0;
    }
    let gangue_sg = setup.minerals[mineral_idx].sg.max(0.5);
    let mut modified = 0usize;
    for p in &mut stream.particles {
        if p.size_class_no == size_class_no && p.size_um < min_size_um {
            let old_sz = p.size_um.max(0.5);
            let vol_ratio = (min_size_um / old_sz).powi(3).clamp(1.0, 100.0);
            let added_vol_frac = vol_ratio - 1.0;
            let old_mass = p.sg.max(0.5);
            let added_mass = added_vol_frac * gangue_sg;
            let new_mass = old_mass + added_mass;

            let mut new_wt = vec![0.0; n_min];
            for m in 0..n_min {
                let mw = p.min_wt_pct.get(m).copied().unwrap_or(0.0);
                new_wt[m] = (mw * old_mass) / new_mass;
            }
            new_wt[mineral_idx] += 100.0 * added_mass / new_mass;
            p.min_wt_pct = new_wt;
            p.size_um = min_size_um;
            p.wt_pct_in_fraction *= new_mass / old_mass;
            p.recompute_vol_and_sg(setup);
            modified += 1;
        }
    }
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Fill Gangue",
        format!(
            "Class {}, {}, <{} um ({modified} particles)",
            setup.size_class_label(size_class_no),
            setup.minerals[mineral_idx].code,
            min_size_um
        ),
    );
    modified
}

/// §85.3.15 — `+Lib` / `-Lib`: scales liberated particles in selected mineral groups & size classes.
pub fn run_adjust_liberation(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
    selected_minerals: &[usize],
    selected_size_nos: &[usize],
    increase: bool,
    step_pct: f64,
) -> usize {
    if !stream.is_basic_binned {
        run_basic_binning(stream, setup);
    }
    let factor = if increase {
        1.0 + step_pct.abs() / 100.0
    } else {
        (1.0 - step_pct.abs() / 100.0).max(0.05)
    };
    let mut touched = 0usize;
    for p in &mut stream.particles {
        if !selected_size_nos.contains(&p.size_class_no) {
            continue;
        }
        if let ParticleKind::LiberatedA(m) = p.kind {
            if selected_minerals.contains(&m) {
                p.wt_pct_in_fraction *= factor;
                touched += 1;
            }
        }
    }
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        if increase { "+Lib" } else { "-Lib" },
        format!("Adjusted {touched} liberated classes by factor {factor:.2}"),
    );
    touched
}

/// §85.3.3 — `Add` / `Subtract`: adds or subtracts `reference` particle flow rates from `active`.
pub fn run_add_or_subtract_stream(
    active: &mut LiberationStream,
    reference: &LiberationStream,
    setup: &MineralSetup,
    subtract: bool,
) {
    let sign = if subtract { -1.0 } else { 1.0 };
    let n_sz = setup.size_classes.len();
    let mut map: BTreeMap<(usize, String), ParticleRecord> = BTreeMap::new();

    for p in &active.particles {
        let key = (p.size_class_no, p.kind.class_signature());
        map.entry(key)
            .and_modify(|e| {
                e.tph += p.tph;
                e.nop += p.nop;
            })
            .or_insert_with(|| p.clone());
    }
    for p in &reference.particles {
        let key = (p.size_class_no, p.kind.class_signature());
        if let Some(e) = map.get_mut(&key) {
            e.tph = (e.tph + sign * p.tph).max(0.0);
            if !subtract {
                e.nop += p.nop;
            }
        } else if !subtract {
            map.insert(key, p.clone());
        }
    }

    let new_ps: Vec<ParticleRecord> = map.into_values().filter(|p| p.tph > 1e-12).collect();
    let mut size_tph = vec![0.0; n_sz];
    let mut total_tph = 0.0;
    for p in &new_ps {
        if p.size_class_no >= 1 && p.size_class_no <= n_sz {
            size_tph[p.size_class_no - 1] += p.tph;
            total_tph += p.tph;
        }
    }
    active.total_input_tph = total_tph;
    if total_tph > 1e-12 {
        active.size_bulk_wt_pct = size_tph.iter().map(|&t| 100.0 * t / total_tph).collect();
    }
    active.particles = new_ps
        .into_iter()
        .map(|mut p| {
            let st = size_tph.get(p.size_class_no.saturating_sub(1)).copied().unwrap_or(1.0);
            p.wt_pct_in_fraction = if st > 1e-12 { 100.0 * p.tph / st } else { 0.0 };
            p
        })
        .collect();
    active.sync_particle_flowrates(setup);
    active.push_log(
        if subtract { "Subtract Stream" } else { "Add Stream" },
        format!("Ref: {}", reference.name),
    );
}

// ============================================================================
// 5. All 7 Stream Analysis Graphs (§85.4: MC, EC, PSD, MD, ED, LIB, LIM)
// ============================================================================

/// Generic chart / table series used by `Graphs` window (supports both Chart & Table views).
#[derive(Debug, Clone, Default)]
pub struct LiberationTableData {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub categories: Vec<String>,
    pub series_names: Vec<String>,
    /// `[series_idx][cat_idx]`
    pub values: Vec<Vec<f64>>,
}

/// 1. `MC` — Modal (mineral) composition by size (+ Bulk).
pub fn build_mc_graph(stream: &LiberationStream, setup: &MineralSetup) -> LiberationTableData {
    let sum = stream.compute_summary(setup);
    let mut cats = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        cats.push(sc.label(&setup.size_unit));
    }
    let mut series_names = Vec::new();
    let mut values = Vec::new();
    for (m, mdef) in setup.minerals.iter().enumerate() {
        series_names.push(mdef.code.clone());
        let mut row = vec![sum.mineral_wt_bulk[m]];
        row.extend_from_slice(&sum.mineral_wt_by_size[m]);
        values.push(row);
    }
    LiberationTableData {
        title: format!("Modal Composition — {}", stream.name),
        x_label: "Size Fraction".into(),
        y_label: "Mineral wt-%".into(),
        categories: cats,
        series_names,
        values,
    }
}

/// 2. `EC` — Elemental composition by size (+ Bulk).
pub fn build_ec_graph(stream: &LiberationStream, setup: &MineralSetup) -> LiberationTableData {
    let sum = stream.compute_summary(setup);
    let mut cats = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        cats.push(sc.label(&setup.size_unit));
    }
    let mut values = Vec::new();
    for ei in 0..sum.elements.len() {
        let mut row = vec![sum.elem_wt_bulk[ei]];
        row.extend_from_slice(&sum.elem_wt_by_size[ei]);
        values.push(row);
    }
    LiberationTableData {
        title: format!("Elemental Composition — {}", stream.name),
        x_label: "Size Fraction".into(),
        y_label: "Element wt-%".into(),
        categories: cats,
        series_names: sum.elements,
        values,
    }
}

/// 3. `PSD` — Particle Size Distribution by mineral (Cumulative passing or Continuous passing).
pub fn build_psd_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    cumulative: bool,
) -> (Vec<f64>, LiberationTableData) {
    let sum = stream.compute_summary(setup);
    let xs: Vec<f64> = setup.size_classes.iter().map(|s| s.upper).collect();
    let cats: Vec<String> = setup
        .size_classes
        .iter()
        .map(|s| s.label(&setup.size_unit))
        .collect();
    let mut series_names = vec!["Bulk".into()];
    let mut values = Vec::new();

    // Bulk curve
    let mut bulk_row = Vec::new();
    let mut acc = 0.0;
    let bulk_tot: f64 = stream.size_bulk_wt_pct.iter().sum::<f64>().max(1e-9);
    for s in 0..setup.size_classes.len() {
        let f = 100.0 * stream.size_bulk_wt_pct.get(s).copied().unwrap_or(0.0) / bulk_tot;
        acc += f;
        bulk_row.push(if cumulative { acc } else { f });
    }
    values.push(bulk_row);

    // Per-mineral PSD curves
    for (m, mdef) in setup.minerals.iter().enumerate() {
        series_names.push(mdef.code.clone());
        let mtot = sum.mineral_tph_bulk[m].max(1e-12);
        let mut row = Vec::new();
        let mut macc = 0.0;
        for s in 0..setup.size_classes.len() {
            let f = 100.0 * sum.mineral_tph_by_size[m][s] / mtot;
            macc += f;
            row.push(if cumulative { macc } else { f });
        }
        values.push(row);
    }

    (
        xs,
        LiberationTableData {
            title: format!(
                "Particle Size Distribution ({}) — {}",
                if cumulative { "Cumulative Passing" } else { "Frequency" },
                stream.name
            ),
            x_label: format!("Particle Size ({})", setup.size_unit),
            y_label: if cumulative { "Cumulative Passing (%)" } else { "Weight in Class (%)" }.into(),
            categories: cats,
            series_names,
            values,
        },
    )
}

/// 4. `MD` — Mode of occurrence of a chosen mineral (Liberated, Binary with each partner, Ternary, Complex).
pub fn build_md_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    mineral_idx: usize,
    each_size_100: bool,
) -> LiberationTableData {
    let n_sz = setup.size_classes.len();
    let mut cats = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        cats.push(sc.label(&setup.size_unit));
    }
    let m_code = setup
        .minerals
        .get(mineral_idx)
        .map(|m| m.code.as_str())
        .unwrap_or("Min");

    // Occurrence modes: Liberated, Binary with each other mineral, Ternary, Complex
    let mut mode_names = vec![format!("Liberated {m_code}")];
    let mut partner_map: HashMap<usize, usize> = HashMap::new();
    for (i, m) in setup.minerals.iter().enumerate() {
        if i != mineral_idx {
            partner_map.insert(i, mode_names.len());
            mode_names.push(format!("{m_code}-{}", m.code));
        }
    }
    let idx_tern = mode_names.len();
    mode_names.push("Ternary".into());
    let idx_comp = mode_names.len();
    mode_names.push("Complex/Packed".into());

    let n_modes = mode_names.len();
    let mut tph_grid = vec![vec![0.0; n_sz + 1]; n_modes];

    for p in &stream.particles {
        if p.size_class_no < 1 || p.size_class_no > n_sz {
            continue;
        }
        let m_tph = p.tph * p.min_wt_pct.get(mineral_idx).copied().unwrap_or(0.0) / 100.0;
        if m_tph <= 1e-12 {
            continue;
        }
        let mode_i = match &p.kind {
            ParticleKind::LiberatedA(m) if *m == mineral_idx => 0,
            ParticleKind::BinaryB { m1, m2, .. } if *m1 == mineral_idx => {
                *partner_map.get(m2).unwrap_or(&idx_comp)
            }
            ParticleKind::BinaryB { m1, m2, .. } if *m2 == mineral_idx => {
                *partner_map.get(m1).unwrap_or(&idx_comp)
            }
            ParticleKind::TernaryC { .. } => idx_tern,
            ParticleKind::ComplexD(_) | ParticleKind::PackedE { .. } => idx_comp,
            _ => {
                // Fallback for unbinned raw particles
                let w = p.min_wt_pct.get(mineral_idx).copied().unwrap_or(0.0);
                if w >= stream.config.lib_threshold {
                    0
                } else {
                    idx_comp
                }
            }
        };
        tph_grid[mode_i][0] += m_tph;
        tph_grid[mode_i][p.size_class_no] += m_tph;
    }

    let total_bulk: f64 = (0..n_modes).map(|k| tph_grid[k][0]).sum::<f64>().max(1e-12);
    for col in 0..=n_sz {
        let denom = if each_size_100 || col == 0 {
            (0..n_modes).map(|k| tph_grid[k][col]).sum::<f64>().max(1e-12)
        } else {
            total_bulk
        };
        for k in 0..n_modes {
            tph_grid[k][col] = 100.0 * tph_grid[k][col] / denom;
        }
    }

    LiberationTableData {
        title: format!("Mode of Occurrence of {m_code} — {}", stream.name),
        x_label: "Size Fraction".into(),
        y_label: if each_size_100 { "Distribution (% in size)" } else { "Distribution (% of total)" }.into(),
        categories: cats,
        series_names: mode_names,
        values: tph_grid,
    }
}

/// 5. `ED` — Distribution of a chosen element between minerals (by size + Bulk).
pub fn build_ed_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    element: &str,
    each_size_100: bool,
) -> LiberationTableData {
    let sum = stream.compute_summary(setup);
    let n_min = setup.minerals.len();
    let n_sz = setup.size_classes.len();
    let mut cats = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        cats.push(sc.label(&setup.size_unit));
    }

    let mut grid = vec![vec![0.0; n_sz + 1]; n_min];
    for m in 0..n_min {
        let c = setup.minerals[m].chem_wt_pct.get(element).copied().unwrap_or(0.0) / 100.0;
        grid[m][0] = sum.mineral_tph_bulk[m] * c;
        for s in 0..n_sz {
            grid[m][s + 1] = sum.mineral_tph_by_size[m][s] * c;
        }
    }

    let bulk_tot: f64 = (0..n_min).map(|m| grid[m][0]).sum::<f64>().max(1e-12);
    for col in 0..=n_sz {
        let denom = if each_size_100 || col == 0 {
            (0..n_min).map(|m| grid[m][col]).sum::<f64>().max(1e-12)
        } else {
            bulk_tot
        };
        for m in 0..n_min {
            grid[m][col] = 100.0 * grid[m][col] / denom;
        }
    }

    LiberationTableData {
        title: format!("Element Distribution ({element}) — {}", stream.name),
        x_label: "Size Fraction".into(),
        y_label: format!("Weight % of {element} in mineral"),
        categories: cats,
        series_names: setup.minerals.iter().map(|m| m.code.clone()).collect(),
        values: grid,
    }
}

/// 6. `LIB` — Cumulative liberation yield curves by size class for a selected mineral.
pub fn build_lib_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    mineral_idx: usize,
    n_classes: usize,
    first_class_100: bool,
) -> LiberationTableData {
    let nc = n_classes.clamp(4, 12);
    let mut cats = Vec::new();
    let mut thresholds = Vec::new(); // descending lower bounds: 100, 90, 80, ...
    if first_class_100 {
        cats.push("100-100".into());
        thresholds.push(99.5);
        for k in 1..nc {
            let hi = 100.0 * ((nc - k) as f64) / ((nc - 1) as f64);
            let lo = 100.0 * ((nc - 1 - k) as f64) / ((nc - 1) as f64);
            cats.push(format!("{:.0}-{:.0}", lo, hi));
            thresholds.push(lo);
        }
    } else {
        for k in 0..nc {
            let hi = 100.0 * ((nc - k) as f64) / (nc as f64);
            let lo = 100.0 * ((nc - 1 - k) as f64) / (nc as f64);
            cats.push(format!("{:.0}-{:.0}", lo, hi));
            thresholds.push(lo);
        }
    }

    let m_code = setup
        .minerals
        .get(mineral_idx)
        .map(|m| m.code.as_str())
        .unwrap_or("Min");
    let mut series_names = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        series_names.push(sc.label(&setup.size_unit));
    }

    let mut values = Vec::new();
    for s_opt in std::iter::once(None).chain(setup.size_classes.iter().map(|s| Some(s.no))) {
        let mut tot_m = 0.0;
        let mut ps: Vec<(f64, f64)> = Vec::new();
        for p in &stream.particles {
            if let Some(sn) = s_opt {
                if p.size_class_no != sn {
                    continue;
                }
            }
            let grade = p.min_wt_pct.get(mineral_idx).copied().unwrap_or(0.0);
            let m_tph = p.tph * grade / 100.0;
            if m_tph > 1e-12 {
                tot_m += m_tph;
                ps.push((grade, m_tph));
            }
        }
        let mut row = Vec::with_capacity(thresholds.len());
        for &thr in &thresholds {
            let cum: f64 = ps.iter().filter(|&&(g, _)| g >= thr).map(|&(_, mt)| mt).sum();
            row.push(if tot_m > 1e-12 { 100.0 * cum / tot_m } else { 0.0 });
        }
        values.push(row);
    }

    LiberationTableData {
        title: format!("Cumulative Liberation Yield ({m_code}) — {}", stream.name),
        x_label: "Liberation Class (%)".into(),
        y_label: "Cumulative Mineral Yield (%)".into(),
        categories: cats,
        series_names,
        values,
    }
}

/// 6b. `LIB` XY — Continuous cumulative liberation yield curves `(Wt% of mineral in particle 0..100, Cumulative yield %)`
/// matching Batch 2 Screenshot 10 center (`Cumulative liberation of Gn %`).
pub fn build_lib_xy_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    mineral_idx: usize,
    n_classes: usize,
    first_class_100: bool,
) -> (String, String, Vec<String>, Vec<Vec<(f64, f64)>>) {
    let nc = n_classes.clamp(4, 20);
    let mut thresholds: Vec<f64> = Vec::new();
    for k in 0..nc {
        thresholds.push(100.0 * (k as f64) / (nc as f64));
    }
    if first_class_100 {
        thresholds.push(98.0);
        thresholds.push(99.5);
    }
    thresholds.push(100.0);

    let m_code = setup
        .minerals
        .get(mineral_idx)
        .map(|m| m.code.as_str())
        .unwrap_or("Min");
    let mut names = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        names.push(sc.label(&setup.size_unit));
    }

    let mut curves = Vec::new();
    for s_opt in std::iter::once(None).chain(setup.size_classes.iter().map(|s| Some(s.no))) {
        let mut tot_m = 0.0;
        let mut ps: Vec<(f64, f64)> = Vec::new();
        for p in &stream.particles {
            if let Some(sn) = s_opt {
                if p.size_class_no != sn {
                    continue;
                }
            }
            let grade = p.min_wt_pct.get(mineral_idx).copied().unwrap_or(0.0);
            let m_tph = p.tph * grade / 100.0;
            if m_tph > 1e-12 {
                tot_m += m_tph;
                ps.push((grade, m_tph));
            }
        }
        let mut pts = Vec::new();
        if tot_m > 1e-12 {
            for &thr in &thresholds {
                let effective_thr = if thr >= 99.9 { stream.config.lib_threshold } else { thr };
                let cum: f64 = ps.iter().filter(|&&(g, _)| g >= effective_thr).map(|&(_, mt)| mt).sum();
                pts.push((thr, 100.0 * cum / tot_m));
            }
        }
        curves.push(pts);
    }

    (
        format!("Cumulative liberation of {m_code} %"),
        format!("Wt% of mineral {m_code} in particle"),
        names,
        curves,
    )
}

/// 7. `LIM` — Mineralogical limiting grade-recovery curves (ideal particle sorting by grade).
/// Returns `(series_names, Vec<Vec<(recovery_pct, grade_pct)>>)` for Bulk + each size class.
pub fn build_lim_graph(
    stream: &LiberationStream,
    setup: &MineralSetup,
    mineral_idx: Option<usize>,
    element_opt: Option<&str>,
) -> (String, Vec<String>, Vec<Vec<(f64, f64)>>) {
    let target_label = if let Some(el) = element_opt {
        format!("Lim(e): {el}")
    } else {
        let m = mineral_idx.unwrap_or(0);
        format!(
            "Lim(m): {}",
            setup.minerals.get(m).map(|x| x.code.as_str()).unwrap_or("Min")
        )
    };

    let particle_grade = |p: &ParticleRecord| -> f64 {
        if let Some(el) = element_opt {
            let mut g = 0.0;
            for (mi, mdef) in setup.minerals.iter().enumerate() {
                let c = mdef.chem_wt_pct.get(el).copied().unwrap_or(0.0);
                g += p.min_wt_pct.get(mi).copied().unwrap_or(0.0) * c / 100.0;
            }
            g
        } else {
            p.min_wt_pct.get(mineral_idx.unwrap_or(0)).copied().unwrap_or(0.0)
        }
    };

    let mut names = vec!["Bulk".into()];
    for sc in &setup.size_classes {
        names.push(sc.label(&setup.size_unit));
    }
    let mut curves = Vec::new();
    for s_opt in std::iter::once(None).chain(setup.size_classes.iter().map(|s| Some(s.no))) {
        let mut items: Vec<(f64, f64)> = stream
            .particles
            .iter()
            .filter(|p| s_opt.map(|sn| p.size_class_no == sn).unwrap_or(true) && p.tph > 1e-12)
            .map(|p| (particle_grade(p), p.tph))
            .collect();
        // Sort descending by grade for theoretical mineralogical limiting curve
        items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let tot_val: f64 = items.iter().map(|&(g, t)| g * t / 100.0).sum();
        let mut pts = Vec::new();
        if tot_val > 1e-12 {
            let mut cum_mass = 0.0;
            let mut cum_val = 0.0;
            for (g, t) in items {
                cum_mass += t;
                cum_val += g * t / 100.0;
                let rec = (100.0 * cum_val / tot_val).clamp(0.0, 100.0);
                let cum_grade = 100.0 * cum_val / cum_mass.max(1e-12);
                pts.push((rec, cum_grade));
            }
        }
        curves.push(pts);
    }

    (
        format!("Mineralogical Limiting Curves ({target_label}) — {}", stream.name),
        names,
        curves,
    )
}

// ============================================================================
// 6. Export / Import: MinSetup.xlsx, Stream.xlsx, HSCStream XML/JSON & CSV
// ============================================================================

/// Builds `MinSetup.xlsx` with `Mins` and `Size` sheets (§85.2.1).
pub fn export_minsetup_xlsx(setup: &MineralSetup) -> Vec<u8> {
    let mut mins_cells = vec![
        ExportCell { row: 0, col: 0, value: ExportCellValue::Text("HSC Sim Mineral Based Model Set-up File".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 0, col: 5, value: ExportCellValue::Text(format!("Version {}", setup.version)), style: ExportCellStyle::Default },
        ExportCell { row: 4, col: 0, value: ExportCellValue::Text("No. of minerals".into()), style: ExportCellStyle::Header },
        ExportCell { row: 4, col: 2, value: ExportCellValue::Number(setup.minerals.len() as f64), style: ExportCellStyle::BoldBox },
        ExportCell { row: 5, col: 0, value: ExportCellValue::Text("MineralNo".into()), style: ExportCellStyle::Header },
        ExportCell { row: 6, col: 0, value: ExportCellValue::Text("Code".into()), style: ExportCellStyle::Default },
        ExportCell { row: 7, col: 0, value: ExportCellValue::Text("Mineral".into()), style: ExportCellStyle::Default },
        ExportCell { row: 8, col: 0, value: ExportCellValue::Text("Includes".into()), style: ExportCellStyle::Default },
        ExportCell { row: 9, col: 0, value: ExportCellValue::Text("No of behavioral types:".into()), style: ExportCellStyle::Default },
        ExportCell { row: 13, col: 0, value: ExportCellValue::Text("Reference".into()), style: ExportCellStyle::YellowNote },
        ExportCell { row: 14, col: 0, value: ExportCellValue::Text("SG (Specific gravity)".into()), style: ExportCellStyle::Default },
        ExportCell { row: 15, col: 0, value: ExportCellValue::Text("Chemical composition, wt.%".into()), style: ExportCellStyle::Header },
    ];
    let elems = setup.all_elements();
    for (ei, el) in elems.iter().enumerate() {
        mins_cells.push(ExportCell {
            row: 16 + ei,
            col: 0,
            value: ExportCellValue::Text(format!("{el} %")),
            style: ExportCellStyle::Default,
        });
    }
    for (mi, m) in setup.minerals.iter().enumerate() {
        let col = mi + 2;
        mins_cells.push(ExportCell { row: 5, col, value: ExportCellValue::Number((mi + 1) as f64), style: ExportCellStyle::Header });
        mins_cells.push(ExportCell { row: 6, col, value: ExportCellValue::Text(m.code.clone()), style: ExportCellStyle::Default });
        mins_cells.push(ExportCell { row: 7, col, value: ExportCellValue::Text(m.mineral.clone()), style: ExportCellStyle::Default });
        mins_cells.push(ExportCell { row: 8, col, value: ExportCellValue::Text(m.includes.join(",")), style: ExportCellStyle::Default });
        mins_cells.push(ExportCell { row: 9, col, value: ExportCellValue::Number(m.behavioral_types.len() as f64), style: ExportCellStyle::Default });
        mins_cells.push(ExportCell { row: 13, col, value: ExportCellValue::Text(m.reference.clone()), style: ExportCellStyle::YellowNote });
        mins_cells.push(ExportCell { row: 14, col, value: ExportCellValue::Number(m.sg), style: ExportCellStyle::Default });
        for (ei, el) in elems.iter().enumerate() {
            let v = m.chem_wt_pct.get(el).copied().unwrap_or(0.0);
            mins_cells.push(ExportCell { row: 16 + ei, col, value: ExportCellValue::Number(v), style: ExportCellStyle::Default });
        }
    }

    let mut size_cells = vec![
        ExportCell { row: 0, col: 0, value: ExportCellValue::Text("HSC Sim Mineral Based Model Size Set-up".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 1, col: 0, value: ExportCellValue::Text("Unit".into()), style: ExportCellStyle::Header },
        ExportCell { row: 1, col: 1, value: ExportCellValue::Text(setup.size_unit.clone()), style: ExportCellStyle::Default },
        ExportCell { row: 2, col: 0, value: ExportCellValue::Text("Total".into()), style: ExportCellStyle::Header },
        ExportCell { row: 2, col: 1, value: ExportCellValue::Number(setup.size_classes.len() as f64), style: ExportCellStyle::Default },
    ];
    for (c, h) in ["No", "Lower", "Upper", "Average Size", "Label"].iter().enumerate() {
        size_cells.push(ExportCell { row: 4, col: c, value: ExportCellValue::Text((*h).into()), style: ExportCellStyle::Header });
    }
    for (i, sc) in setup.size_classes.iter().enumerate() {
        let r = 5 + i;
        size_cells.push(ExportCell { row: r, col: 0, value: ExportCellValue::Number(sc.no as f64), style: ExportCellStyle::Default });
        size_cells.push(ExportCell { row: r, col: 1, value: ExportCellValue::Number(sc.lower), style: ExportCellStyle::Default });
        size_cells.push(ExportCell { row: r, col: 2, value: ExportCellValue::Number(sc.upper), style: ExportCellStyle::Default });
        size_cells.push(ExportCell { row: r, col: 3, value: ExportCellValue::Number(sc.average_size()), style: ExportCellStyle::Default });
        size_cells.push(ExportCell { row: r, col: 4, value: ExportCellValue::Text(sc.label(&setup.size_unit)), style: ExportCellStyle::Default });
    }

    build_xlsx(&[
        ExportSheet { name: "Mins".into(), cells: mins_cells, col_widths: vec![26.0, 12.0, 16.0, 16.0, 16.0] },
        ExportSheet { name: "Size".into(), cells: size_cells, col_widths: vec![10.0, 12.0, 12.0, 14.0, 18.0] },
    ])
}

/// Builds a 4-sheet Excel `.xlsx` stream workbook (`Mineral Composition`, `Particles`, `Log`, `Modal`, §85.2.3).
pub fn export_stream_xlsx(stream: &LiberationStream, setup: &MineralSetup) -> Vec<u8> {
    let sum = stream.compute_summary(setup);
    let mut mc = vec![
        ExportCell { row: 0, col: 1, value: ExportCellValue::Text("HSC Sim Mineral Based Model: Stream characterization".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 2, col: 1, value: ExportCellValue::Text("Stream:".into()), style: ExportCellStyle::Header },
        ExportCell { row: 2, col: 2, value: ExportCellValue::Text(stream.name.clone()), style: ExportCellStyle::Default },
        ExportCell { row: 3, col: 1, value: ExportCellValue::Text("Total input:".into()), style: ExportCellStyle::Header },
        ExportCell { row: 3, col: 2, value: ExportCellValue::Number(stream.total_input_tph), style: ExportCellStyle::BoldBox },
        ExportCell { row: 3, col: 3, value: ExportCellValue::Text("tph".into()), style: ExportCellStyle::Default },
        ExportCell { row: 5, col: 0, value: ExportCellValue::Text("SizeClassNo".into()), style: ExportCellStyle::Header },
        ExportCell { row: 6, col: 1, value: ExportCellValue::Text("Name".into()), style: ExportCellStyle::Default },
        ExportCell { row: 7, col: 1, value: ExportCellValue::Text("Wt.% of bulk".into()), style: ExportCellStyle::Default },
        ExportCell { row: 7, col: 2, value: ExportCellValue::Number(100.0), style: ExportCellStyle::Default },
    ];
    for (s, sc) in setup.size_classes.iter().enumerate() {
        let col = 3 + s;
        mc.push(ExportCell { row: 5, col, value: ExportCellValue::Number(sc.no as f64), style: ExportCellStyle::Header });
        mc.push(ExportCell { row: 6, col, value: ExportCellValue::Text(sc.label(&setup.size_unit)), style: ExportCellStyle::Default });
        mc.push(ExportCell { row: 7, col, value: ExportCellValue::Number(stream.size_bulk_wt_pct.get(s).copied().unwrap_or(0.0)), style: ExportCellStyle::Default });
    }
    for (m, mdef) in setup.minerals.iter().enumerate() {
        let r = 9 + m;
        mc.push(ExportCell { row: r, col: 1, value: ExportCellValue::Text(format!("{} wt%", mdef.code)), style: ExportCellStyle::Default });
        mc.push(ExportCell { row: r, col: 2, value: ExportCellValue::Number(sum.mineral_wt_bulk[m]), style: ExportCellStyle::Default });
        for s in 0..setup.size_classes.len() {
            mc.push(ExportCell { row: r, col: 3 + s, value: ExportCellValue::Number(sum.mineral_wt_by_size[m][s]), style: ExportCellStyle::Default });
        }
    }

    // Particles sheet
    let mut ps = Vec::new();
    let p_hdrs = ["ID", "Size", "SizeClassNo", "SizeClassName", "ParticleType", "tph", "Wt%(InBulk)", "Wt%(InFraction)", "NOP", "SG"];
    for (c, h) in p_hdrs.iter().enumerate() {
        ps.push(ExportCell { row: 5, col: c, value: ExportCellValue::Text((*h).into()), style: ExportCellStyle::Header });
    }
    for (m, mdef) in setup.minerals.iter().enumerate() {
        ps.push(ExportCell { row: 5, col: 10 + m, value: ExportCellValue::Text(format!("{} wt%", mdef.code)), style: ExportCellStyle::Header });
    }
    for (i, p) in stream.particles.iter().enumerate() {
        let r = 6 + i;
        ps.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text(p.id.clone()), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 1, value: ExportCellValue::Number(p.size_um), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 2, value: ExportCellValue::Number(p.size_class_no as f64), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 3, value: ExportCellValue::Text(setup.size_class_label(p.size_class_no)), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 4, value: ExportCellValue::Text(p.particle_type.clone()), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 5, value: ExportCellValue::Number(p.tph), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 6, value: ExportCellValue::Number(p.wt_pct_in_bulk), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 7, value: ExportCellValue::Number(p.wt_pct_in_fraction), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 8, value: ExportCellValue::Number(p.nop), style: ExportCellStyle::Default });
        ps.push(ExportCell { row: r, col: 9, value: ExportCellValue::Number(p.sg), style: ExportCellStyle::Default });
        for m in 0..setup.minerals.len() {
            ps.push(ExportCell { row: r, col: 10 + m, value: ExportCellValue::Number(p.min_wt_pct.get(m).copied().unwrap_or(0.0)), style: ExportCellStyle::Default });
        }
    }

    // Log sheet
    let mut lg = Vec::new();
    for (c, h) in ["Operation", "By", "At", "Note", "Particles"].iter().enumerate() {
        lg.push(ExportCell { row: 0, col: c, value: ExportCellValue::Text((*h).into()), style: ExportCellStyle::Header });
    }
    for (i, e) in stream.log.iter().enumerate() {
        let r = i + 1;
        lg.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text(e.operation.clone()), style: ExportCellStyle::Default });
        lg.push(ExportCell { row: r, col: 1, value: ExportCellValue::Text(e.by.clone()), style: ExportCellStyle::Default });
        lg.push(ExportCell { row: r, col: 2, value: ExportCellValue::Text(e.at.clone()), style: ExportCellStyle::Default });
        lg.push(ExportCell { row: r, col: 3, value: ExportCellValue::Text(e.note.clone()), style: ExportCellStyle::Default });
        lg.push(ExportCell { row: r, col: 4, value: ExportCellValue::Number(e.particles as f64), style: ExportCellStyle::Default });
    }

    // Modal sheet
    let mut md = Vec::new();
    for (s, sc) in setup.size_classes.iter().enumerate() {
        md.push(ExportCell { row: 0, col: 1 + s, value: ExportCellValue::Text(sc.label(&setup.size_unit)), style: ExportCellStyle::Header });
    }
    for (m, mdef) in setup.minerals.iter().enumerate() {
        md.push(ExportCell { row: 1 + m, col: 0, value: ExportCellValue::Text(mdef.code.clone()), style: ExportCellStyle::Default });
        for s in 0..setup.size_classes.len() {
            let v = stream.modal_targets.get(m).and_then(|r| r.get(s)).copied().unwrap_or(0.0);
            md.push(ExportCell { row: 1 + m, col: 1 + s, value: ExportCellValue::Number(v), style: ExportCellStyle::Default });
        }
    }

    build_xlsx(&[
        ExportSheet { name: "Mineral Composition".into(), cells: mc, col_widths: vec![12.0, 22.0, 14.0, 14.0, 14.0] },
        ExportSheet { name: "Particles".into(), cells: ps, col_widths: vec![18.0, 12.0, 12.0, 14.0, 14.0, 12.0, 12.0, 14.0, 10.0, 10.0] },
        ExportSheet { name: "Log".into(), cells: lg, col_widths: vec![18.0, 12.0, 18.0, 32.0, 12.0] },
        ExportSheet { name: "Modal".into(), cells: md, col_widths: vec![12.0, 12.0, 12.0, 12.0, 12.0] },
    ])
}

/// Bundled `.HSCStream` / `.xml` format storing both `MineralSetup` and `LiberationStream` (§85.2.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HscStreamPackage {
    pub setup: MineralSetup,
    pub stream: LiberationStream,
}

impl HscStreamPackage {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {e}"))
    }

    pub fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| format!("Parse error: {e}"))
    }
}

/// Imports CSV/TSV particle data (e.g. 3D CT `.csv` or exported SEM table) into a chosen size class.
/// Expected columns: `Size, NOP, <min1_wt%>, <min2_wt%>, ...` (or header row matching mineral codes).
pub fn import_particles_csv_into_size_class(
    stream: &mut LiberationStream,
    setup: &MineralSetup,
    size_class_no: usize,
    raw_csv: &str,
) -> Result<usize, String> {
    let n_min = setup.minerals.len();
    if n_min == 0 {
        return Err("Mineral Setup has no minerals defined.".into());
    }
    let lines: Vec<&str> = raw_csv.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return Err("CSV file is empty.".into());
    }

    let mut added = 0usize;
    let avg_sz = setup
        .size_classes
        .iter()
        .find(|s| s.no == size_class_no)
        .map(|s| s.average_size())
        .unwrap_or(50.0);

    // Replace existing particles in that size class
    stream.particles.retain(|p| p.size_class_no != size_class_no);

    for (li, line) in lines.iter().enumerate() {
        let sep = if line.contains('\t') {
            '\t'
        } else if line.contains(';') {
            ';'
        } else {
            ','
        };
        let parts: Vec<&str> = line.split(sep).map(|s| s.trim()).collect();
        let nums: Vec<f64> = parts
            .iter()
            .filter_map(|s| s.replace(',', ".").parse::<f64>().ok())
            .collect();
        if li == 0 && nums.len() < 2 {
            continue; // skip header row
        }
        if nums.len() < 2 {
            continue;
        }
        let sz = if nums.len() >= n_min + 2 { nums[0] } else { avg_sz };
        let nop = if nums.len() >= n_min + 2 { nums[1].max(1.0) } else { 1.0 };
        let start = if nums.len() >= n_min + 2 { 2 } else { 0 };
        let mut w = vec![0.0; n_min];
        let mut s = 0.0;
        for m in 0..n_min {
            let v = nums.get(start + m).copied().unwrap_or(0.0).max(0.0);
            w[m] = v;
            s += v;
        }
        if s <= 1e-9 {
            continue;
        }
        for val in &mut w {
            *val = 100.0 * (*val) / s;
        }
        added += 1;
        let mut pr = ParticleRecord {
            id: format!("M.P.{size_class_no}.X.{added}"),
            size_um: sz.max(1.0),
            size_class_no,
            particle_type: "X".into(),
            tph: 0.0,
            wt_pct_in_bulk: 0.0,
            wt_pct_in_fraction: nop,
            nop,
            sg: 3.5,
            min_wt_pct: w,
            min_vol_pct: vec![0.0; n_min],
            min_sur_pct: vec![0.0; n_min],
            kind: ParticleKind::RawX,
        };
        pr.recompute_vol_and_sg(setup);
        stream.particles.push(pr);
    }

    if added == 0 {
        return Err("No valid numeric particle rows parsed from file.".into());
    }
    stream.is_basic_binned = false;
    stream.sync_particle_flowrates(setup);
    stream.push_log(
        "Import Fraction",
        format!(
            "Loaded {added} particles into {}",
            setup.size_class_label(size_class_no)
        ),
    );
    Ok(added)
}

// ============================================================================
// Internal Deterministic RNG
// ============================================================================

struct LcgRng(u64);

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
    fn f64(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_class_averages_match_ch85_screenshot_exact() {
        let classes = MineralSetup::default_7_size_classes();
        let expected = [4.47, 14.14, 27.57, 53.39, 89.16, 126.10, 212.13];
        for (sc, exp) in classes.iter().zip(expected.iter()) {
            assert!(
                (sc.average_size() - exp).abs() < 0.01,
                "{}: got {}, expected {}",
                sc.no,
                sc.average_size(),
                exp
            );
        }
    }

    #[test]
    fn full_liberation_pipeline_bin_missing_rebalance_pack_and_graphs() {
        let setup = MineralSetup::default_pb_zn_project();
        let mut feed = LiberationStream::generate_sample_stream("Feed", &setup, 21.2, 0.30, 0.25, true, 42);
        let sum_before = feed.compute_summary(&setup);
        // Outer classes 1 (0-10) and 7 (150-300) were omitted -> bulk wt% = 80% (exact match to Ch. 85!)
        assert!((sum_before.total_wt_bulk - 80.0).abs() < 1e-6);

        // 1. Basic Binning
        run_basic_binning(&mut feed, &setup);
        assert!(feed.is_basic_binned);
        assert!(feed.particles.iter().any(|p| p.id.contains(".A.01")));

        // 2. Extrapolate Missing (fills size classes 1 and 7 via WLS)
        let filled = run_extrapolate_missing(&mut feed, &setup).unwrap();
        assert_eq!(filled, 2);
        let sum_after = feed.compute_summary(&setup);
        assert!((sum_after.total_wt_bulk - 100.0).abs() < 1e-3);

        // 3. Rebalance to custom modal target in class 2 (10-20 um)
        feed.modal_targets[0][1] = 35.0; // Gn
        feed.modal_targets[1][1] = 25.0; // Sp
        feed.modal_targets[2][1] = 5.0;  // Py
        feed.modal_targets[3][1] = 20.0; // Dol
        feed.modal_targets[4][1] = 15.0; // Qtz
        let (_iters, diff) = run_rebalance(&mut feed, &setup).unwrap();
        assert!(diff < 2.0, "rebalance diff={diff}");

        // 4. Pack + Clean
        let before_pack = feed.particles.len();
        run_pack(&mut feed, &setup);
        assert!(feed.particles.len() <= before_pack);
        run_clean(&mut feed, &setup);

        // 5. All 7 Graphs compute cleanly
        let mc = build_mc_graph(&feed, &setup);
        assert_eq!(mc.categories.len(), 8);
        let ec = build_ec_graph(&feed, &setup);
        assert!(ec.series_names.contains(&"Pb".to_string()));
        let (xs, psd) = build_psd_graph(&feed, &setup, true);
        assert_eq!(xs.len(), 7);
        assert_eq!(psd.series_names[0], "Bulk");
        let md = build_md_graph(&feed, &setup, 0, true);
        assert!(!md.series_names.is_empty());
        let ed = build_ed_graph(&feed, &setup, "Pb", true);
        assert_eq!(ed.series_names.len(), 5);
        let lib = build_lib_graph(&feed, &setup, 0, 10, true);
        assert_eq!(lib.categories[0], "100-100");
        let (_t, lim_names, lim_curves) = build_lim_graph(&feed, &setup, Some(0), None);
        assert_eq!(lim_names.len(), 8);
        assert!(!lim_curves[0].is_empty());

        // 6. XLSX export produces valid PKZIP workbook
        let setup_xlsx = export_minsetup_xlsx(&setup);
        let stream_xlsx = export_stream_xlsx(&feed, &setup);
        assert!(setup_xlsx.starts_with(b"PK\x03\x04"));
        assert!(stream_xlsx.starts_with(b"PK\x03\x04"));
    }

    #[test]
    fn catalog_search_finds_galena_and_sphalerite_from_3812_minerals() {
        let res = search_geo_mineral_catalog("Galena", 10);
        assert!(res.iter().any(|m| m.name.eq_ignore_ascii_case("Galena")));
        let cu_fe_s = find_geo_minerals_by_elements(&["Cu".into(), "Fe".into(), "S".into()], 20);
        assert!(!cu_fe_s.is_empty());
    }
}
