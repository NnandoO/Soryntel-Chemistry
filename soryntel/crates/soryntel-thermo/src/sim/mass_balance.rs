//! Module 20: `HSC - Mass Balance` (`Mas`) — Data Reconciliation Engine.
//!
//! Clean-room implementation of Chapter 51 (`51-mass-balance.md`):
//! - Topology: units (`Size Transforming`), streams (`Solids/Slurry`, `Liquid/Water`, `Unknown`),
//!   variables (`SF`, `SC`, `A`, `M`, `SA`, `SP`, `Info`), size fractions (`Bulk` + sieves), datasets.
//! - Uncertainty models (`Table 3–4`): `Fixed Absolute`, `Fixed Relative`, `S-shape`,
//!   `Mineral Grade Based`, `Size Fraction (Whiten)` + sample quality multipliers (`1 / 1.5 / 3`).
//! - Pre-processing: sampling rounds (`Master Data` + empirical `SD`), `Stream by Equation`,
//!   `Sol-% to Water`, `Estimate Missing Values`, `Element to Mineral` (via HSC Geo NNLS),
//!   MATLAB-equivalent `Assays_scaled` (`Fig. 17–18`), and `Low Grade Weighting`.
//! - Two-stage reconciliation engine (`51.7`, Eqs. 1–29):
//!   - Stage 1: Iterative Element-Wise Weighted Total Least Squares (`EWTLS`) for total solids,
//!     water/liquid, and size-fraction sub-flows (`Unsized`, `Sized`, `Sized by Assay`).
//!   - Stage 2: Weighted Least Squares (`WLS`) with `LS`, `LS MinMax` (Chebyshev), `NNLS`,
//!     `CLS` (box bounds), and `LLS` (`x > SD`, unsized only), plus optional `Sum = 100`.
//! - Post-processing: posterior covariance `±CI`, `WSSQ`, `RMSD`, `Relative RMSD`,
//!   `Unit Balance` residuals, `Goodness`, recoveries, solid `SG`, volumetric flows,
//!   12 Chapter 51.6 validation checks, HSC7 `Analyses.xls` tab/CSV/TSV import, `.HSCMas` JSON,
//!   multi-sheet `.xlsx` export, and bidirectional `SimProject` synchronization.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use soryntel_solver::parse_term;

use crate::minerals_data::{MineralRawRow, MINERALS_RAW};
use crate::sim::model::SimProject;
use crate::species_converter::{
    solve_species_conversion, ConversionMethod, ConverterInputEntry, ConverterOutputSpec,
    SpeciesConverterRequest, SpeciesTypeFilter,
};
use crate::spreadsheet_export::{build_xlsx, ExportCell, ExportCellStyle, ExportCellValue, ExportSheet};
use crate::{ChemicalSpecies, SpeciesRepository};

// ---------------------------------------------------------------------------
// Core Enumerations (Chapter 51)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    Unknown,
    SolidsSlurry,
    LiquidWater,
}

impl StreamType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::SolidsSlurry => "Solids/Slurry",
            Self::LiquidWater => "Liquid/Water",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleQuality {
    Good,
    Moderate,
    Bad,
}

impl SampleQuality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Good => "Good",
            Self::Moderate => "Moderate",
            Self::Bad => "Bad",
        }
    }

    /// Table 4 multiplier on stream sampling error.
    pub fn multiplier(self) -> f64 {
        match self {
            Self::Good => 1.0,
            Self::Moderate => 1.5,
            Self::Bad => 3.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarTypeCode {
    SF,
    SC,
    A,
    M,
    SA,
    SP,
    Info,
}

impl VarTypeCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::SF => "SF",
            Self::SC => "SC",
            Self::A => "A",
            Self::M => "M",
            Self::SA => "SA",
            Self::SP => "SP",
            Self::Info => "Info",
        }
    }

    pub fn type_name(self) -> &'static str {
        match self {
            Self::SF => "Total Solids",
            Self::SC => "Solids Component",
            Self::A => "Assay of Solids Component",
            Self::M => "Mineral Assay",
            Self::SA => "Size Fraction Assay",
            Self::SP => "Solids Percentage",
            Self::Info => "Info",
        }
    }

    pub fn is_assay(self) -> bool {
        matches!(self, Self::A | Self::M)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementMethod {
    FixedAbsolute,
    FixedRelative,
    SShape,
    MineralGradeBased,
    SizeFractionWhiten,
}

impl MeasurementMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::FixedAbsolute => "Fixed Absolute",
            Self::FixedRelative => "Fixed Relative",
            Self::SShape => "S-shape",
            Self::MineralGradeBased => "Mineral Grade Based",
            Self::SizeFractionWhiten => "Size Fraction (Whiten)",
        }
    }

    /// Per 51.4.4, sampling error is added only to formula-based methods.
    pub fn uses_sampling_error(self) -> bool {
        matches!(
            self,
            Self::SShape | Self::MineralGradeBased | Self::SizeFractionWhiten
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreMeasStatus {
    Missing,
    Measured,
    Guesstimated,
    Fixed,
    Excluded,
}

impl PreMeasStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Missing => "Missing",
            Self::Measured => "Measured",
            Self::Guesstimated => "Guesstimated",
            Self::Fixed => "Fixed",
            Self::Excluded => "Excluded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostBalStatus {
    Balanced,
    Calculated,
    NonAvailable,
}

impl PostBalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::Calculated => "Calculated",
            Self::NonAvailable => "Non Available",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveMethod {
    LS,
    LsMinMax,
    NNLS,
    CLS,
    LLS,
}

impl SolveMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::LS => "LS",
            Self::LsMinMax => "LS, MinMax",
            Self::NNLS => "NNLS",
            Self::CLS => "CLS",
            Self::LLS => "LLS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizingMode {
    Unsized,
    Sized,
    SizedByAssay,
}

impl SizingMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unsized => "Unsized",
            Self::Sized => "Sized",
            Self::SizedByAssay => "Sized by Assay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceType {
    SolidsOnly,
    AssaysOnly,
    SolidsAndAssays,
    SolidsWaterAndAssays,
}

impl BalanceType {
    pub fn label(self) -> &'static str {
        match self {
            Self::SolidsOnly => "Solids only",
            Self::AssaysOnly => "Assays only",
            Self::SolidsAndAssays => "Solids and Assays",
            Self::SolidsWaterAndAssays => "Solids, Water and Assays",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckSeverity {
    Error,
    Notification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: usize,
    pub severity: CheckSeverity,
    pub title: String,
    pub detail: String,
}

// ---------------------------------------------------------------------------
// Domain Entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbUnit {
    pub id: usize,
    pub name: String,
    pub selected: bool,
    pub size_transforming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbStream {
    pub id: usize,
    pub name: String,
    pub selected: bool,
    pub stream_type: StreamType,
    pub flows_enabled: bool,
    pub assays_enabled: bool,
    pub sampling_device: String,
    pub sampling_sd: f64,
    pub sample_quality: SampleQuality,
    pub source_unit: Option<usize>,
    pub dest_unit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbVariable {
    pub id: usize,
    pub name: String,
    pub unit: String,
    pub type_code: VarTypeCode,
    pub selected: bool,
    pub method: MeasurementMethod,
    pub param_a: f64,
    pub param_b: f64,
    pub param_c: f64,
    pub low_grade_weighted: bool,
    pub mineral_formula: Option<String>,
    pub mineral_sg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEquation {
    pub target_stream_id: usize,
    pub source_stream_id: usize,
    pub factor: f64,
}

/// Key identifying a single cell in a dataset grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CellKey {
    pub frac_idx: usize,
    pub stream_id: usize,
    pub var_id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbDataset {
    pub name: String,
    pub selected_for_batch: bool,
    pub values: BTreeMap<CellKey, f64>,
    pub estimated_cells: BTreeSet<CellKey>,
    pub meas_status_override: BTreeMap<CellKey, PreMeasStatus>,
    pub sd_override: BTreeMap<CellKey, f64>,
    pub min_bounds: BTreeMap<CellKey, f64>,
    pub max_bounds: BTreeMap<CellKey, f64>,
    pub rounds: Vec<BTreeMap<CellKey, f64>>,

    // Frozen results (Freeze Balanced)
    pub frozen_values: BTreeMap<CellKey, f64>,
    pub frozen_bal_status: BTreeMap<CellKey, PostBalStatus>,

    // Per-dataset algorithm / calculation settings (Fig. 15–16)
    pub method: SolveMethod,
    pub sizing: SizingMode,
    pub balance_type: BalanceType,
    pub mineral_sum_100: bool,
    pub assay_sum_100: bool,
    pub scaling: bool,
    pub max_iter: usize,
    pub balance_tolerance: f64,
    pub null_sd_estimate: f64,
    pub low_grade_coeff: f64,
    pub show_data_bars: bool,
    pub feed_stream_id: Option<usize>,
    pub liquid_sg: f64,
    pub recovered_for_model_fit: BTreeMap<(usize, usize), f64>, // (stream_id, var_id) -> Rec%
}

impl Default for MbDataset {
    fn default() -> Self {
        Self {
            name: "Data Set 1".to_string(),
            selected_for_batch: true,
            values: BTreeMap::new(),
            estimated_cells: BTreeSet::new(),
            meas_status_override: BTreeMap::new(),
            sd_override: BTreeMap::new(),
            min_bounds: BTreeMap::new(),
            max_bounds: BTreeMap::new(),
            rounds: Vec::new(),
            frozen_values: BTreeMap::new(),
            frozen_bal_status: BTreeMap::new(),
            method: SolveMethod::LS,
            sizing: SizingMode::Unsized,
            balance_type: BalanceType::SolidsAndAssays,
            mineral_sum_100: false,
            assay_sum_100: false,
            scaling: true,
            max_iter: 1000,
            balance_tolerance: 0.001,
            null_sd_estimate: 0.001,
            low_grade_coeff: 0.7,
            show_data_bars: true,
            feed_stream_id: None,
            liquid_sg: 1.0,
            recovered_for_model_fit: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationCell {
    pub key: CellKey,
    pub measured: Option<f64>,
    pub balanced: Option<f64>,
    pub ci: Option<f64>,
    pub diff: Option<f64>,
    pub rel_diff_pct: Option<f64>,
    pub min_bound: Option<f64>,
    pub max_bound: Option<f64>,
    pub rsd_pct: Option<f64>,
    pub sd_eff: Option<f64>,
    pub sd_meas: Option<f64>,
    pub sd_sampling: f64,
    pub recovery_pct: Option<f64>,
    pub pre_status: PreMeasStatus,
    pub bal_status: PostBalStatus,
    pub is_frozen: bool,
    pub is_estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodnessVarRow {
    pub var_id: usize,
    pub name: String,
    pub unit: String,
    pub wssq: f64,
    pub diff_tot: f64,
    pub rel_diff_pct: f64,
    pub avg_sd: f64,
    pub avg_rsd_pct: f64,
    pub rmsd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodnessStreamRow {
    pub stream_id: usize,
    pub name: String,
    pub sum_wssq: f64,
    pub diff_sum: f64,
    pub rel_diff_sum: f64,
    pub rel_diff_avg: f64,
    pub rmsd: f64,
    pub relative_rmsd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitBalanceRow {
    pub unit_id: usize,
    pub unit_name: String,
    pub equation_text: String,
    pub input_stream_names: Vec<String>,
    pub output_stream_names: Vec<String>,
    pub var_labels: Vec<String>,
    pub inputs_mass: Vec<Vec<f64>>,
    pub outputs_mass: Vec<Vec<f64>>,
    pub residual_balance: Vec<f64>,
    pub tolerance_error: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbSolution {
    pub dataset_idx: usize,
    pub cells: BTreeMap<CellKey, ReconciliationCell>,
    pub convergence_rounds: Vec<(usize, f64)>, // (round, iterated_tolerance)
    pub converged: bool,
    pub num_meas: usize,
    pub num_bal: usize,
    pub num_zeros: usize,
    pub num_negatives: usize,
    pub wssq: f64,
    pub rmsd: f64,
    pub rel_rmsd: f64,
    pub equations_text: Vec<String>,
    pub goodness_vars: Vec<GoodnessVarRow>,
    pub goodness_streams: Vec<GoodnessStreamRow>,
    pub unit_balances: Vec<UnitBalanceRow>,
    pub sg_by_stream: BTreeMap<usize, f64>,
    pub vol_flow_by_stream: BTreeMap<usize, f64>,
    pub solved_water_by_stream: BTreeMap<usize, f64>,
    pub last_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPanelSpec {
    pub title: String,
    pub show_total_solids: bool,
    pub show_stream_composition: bool,
    pub selected_var_ids: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassBalanceWorkbook {
    pub units: Vec<MbUnit>,
    pub streams: Vec<MbStream>,
    pub variables: Vec<MbVariable>,
    pub size_fractions: Vec<String>, // Index 0 is "Bulk" (cannot be removed)
    pub datasets: Vec<MbDataset>,
    pub active_dataset: usize,
    pub stream_equations: Vec<StreamEquation>,
    pub balancing_of_minerals: bool,
    pub next_id: usize,
    pub solutions: Vec<Option<MbSolution>>,
    pub chart_panels: Vec<ChartPanelSpec>,
}

impl Default for MassBalanceWorkbook {
    fn default() -> Self {
        preset_rougher_flotation_mb()
    }
}

// ---------------------------------------------------------------------------
// Uncertainty & Scaling Helpers (`Table 3–4`, `Figs. 17–18`)
// ---------------------------------------------------------------------------

/// Compute analytical SD, RSD-% and effective combined SD (Eqs. Table 3 & Table 4).
pub fn evaluate_cell_uncertainty(
    value: Option<f64>,
    var: &MbVariable,
    stream: &MbStream,
    pre_status: PreMeasStatus,
    override_sd: Option<f64>,
    null_sd_estimate: f64,
    low_grade_coeff: f64,
) -> (f64, f64, f64, f64) {
    // Returns: (sd_meas, rsd_pct, sd_sampling_scaled, sd_eff_for_solver)
    if let Some(ov) = override_sd {
        let ov_pos = ov.max(null_sd_estimate.max(1e-9));
        let rsd = value
            .map(|v| if v.abs() > 1e-12 { 100.0 * ov_pos / v.abs() } else { 0.0 })
            .unwrap_or(0.0);
        return (ov_pos, rsd, 0.0, ov_pos);
    }

    let x = value.unwrap_or(0.0).abs();
    let sd_meas = match var.method {
        MeasurementMethod::FixedAbsolute => var.param_a.abs().max(null_sd_estimate),
        MeasurementMethod::FixedRelative => {
            if value.is_some() {
                ((var.param_a.abs() / 100.0) * x).max(null_sd_estimate)
            } else {
                var.param_a.abs().max(null_sd_estimate)
            }
        }
        MeasurementMethod::SShape => {
            let a_term = (var.param_a.abs() / 100.0) * x;
            let b_floor = var.param_b.abs().max(1e-6);
            let raw = (b_floor * b_floor + a_term * a_term).sqrt();
            let capped = if var.param_c > 1e-9 { raw.min(var.param_c) } else { raw };
            capped.max(null_sd_estimate)
        }
        MeasurementMethod::MineralGradeBased => {
            if x > 1e-9 {
                let rsd = var.param_a.abs() * x.powf(-0.5);
                ((rsd / 100.0) * x).max(null_sd_estimate)
            } else {
                null_sd_estimate
            }
        }
        MeasurementMethod::SizeFractionWhiten => {
            if x <= 10.0 {
                (0.1 + x / 10.0).max(null_sd_estimate)
            } else {
                1.0_f64.max(null_sd_estimate)
            }
        }
    };

    let sd_sampling = if var.method.uses_sampling_error() {
        stream.sampling_sd.abs() * stream.sample_quality.multiplier()
    } else {
        0.0
    };

    let mut sd_eff = (sd_meas * sd_meas + sd_sampling * sd_sampling).sqrt().max(null_sd_estimate);
    if pre_status == PreMeasStatus::Guesstimated {
        sd_eff = sd_eff.max(x.abs().max(1.0) * 0.5);
    }

    // Apply Low Grade Weighting multiplier if enabled on variable
    if var.low_grade_weighted && low_grade_coeff > 0.0 {
        let factor = 1.0 + low_grade_coeff.clamp(0.0, 1.5) * 4.0;
        sd_eff = (sd_eff / factor).max(1e-9);
    }

    let rsd_pct = if x > 1e-12 {
        100.0 * (sd_meas * sd_meas + sd_sampling * sd_sampling).sqrt() / x
    } else if var.method == MeasurementMethod::FixedRelative {
        var.param_a.abs()
    } else {
        0.0
    };

    (sd_meas, rsd_pct, sd_sampling, sd_eff)
}

/// Apply exact Matlab-notation scaling (`Figs. 17–18`):
/// `S = repmat(AssaysSD(indexOfInput, :), size(AssaysSD, 1), 1);`
/// `Assays_scaled = Assays ./ (100 * S);`
pub fn apply_assay_scaling(
    assays: &[Vec<f64>],
    assays_sd: &[Vec<f64>],
    index_of_input: usize,
) -> Vec<Vec<f64>> {
    let n_rows = assays.len();
    let n_cols = assays.first().map(|r| r.len()).unwrap_or(0);
    if n_rows == 0 || n_cols == 0 || index_of_input >= assays_sd.len() {
        return assays.to_vec();
    }
    let ref_sd = &assays_sd[index_of_input];
    let mut scaled = vec![vec![0.0; n_cols]; n_rows];
    for i in 0..n_rows {
        for j in 0..n_cols {
            let denom = 100.0 * ref_sd.get(j).copied().unwrap_or(1.0).abs().max(1e-12);
            scaled[i][j] = assays[i][j] / denom;
        }
    }
    scaled
}

// ---------------------------------------------------------------------------
// Dense Linear Algebra & Equality-Constrained Solvers (Shared Clean-Room Core)
// ---------------------------------------------------------------------------

/// Solve square linear system `A x = b` with partial pivoting and mild regularization fallback.
fn solve_dense_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, String> {
    let n = b.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    for col in 0..n {
        let mut piv_row = col;
        let mut piv_val = a[col][col].abs();
        for r in (col + 1)..n {
            let v = a[r][col].abs();
            if v > piv_val {
                piv_val = v;
                piv_row = r;
            }
        }
        if piv_val < 1e-13 {
            a[col][col] += 1e-10;
        }
        if piv_row != col {
            a.swap(col, piv_row);
            b.swap(col, piv_row);
        }
        let diag = a[col][col];
        if diag.abs() < 1e-15 {
            return Err("Singular normal matrix in balance solver".into());
        }
        for r in (col + 1)..n {
            let f = a[r][col] / diag;
            if f.abs() > 0.0 {
                for k in col..n {
                    a[r][k] -= f * a[col][k];
                }
                b[r] -= f * b[col];
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }
    Ok(x)
}

/// Invert SPD matrix by solving `M X = I`.
fn invert_spd_matrix(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let k = m.len();
    if k == 0 {
        return Some(Vec::new());
    }
    let mut inv = vec![vec![0.0; k]; k];
    for c in 0..k {
        let mut e = vec![0.0; k];
        e[c] = 1.0;
        let col_sol = solve_dense_linear(m.to_vec(), e).ok()?;
        for r in 0..k {
            inv[r][c] = col_sol[r];
        }
    }
    Some(inv)
}

/// Extract linearly independent equality constraints via Gaussian RREF on `[C | d]`
/// and compute particular solution `x0` (minimum-norm) + orthonormal nullspace basis `Z`.
fn build_constraint_subspace(
    c_rows: &[Vec<f64>],
    d_vals: &[f64],
    n_vars: usize,
) -> Result<(Vec<f64>, Vec<Vec<f64>>), String> {
    if n_vars == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut aug: Vec<Vec<f64>> = Vec::new();
    for (row, &rhs) in c_rows.iter().zip(d_vals.iter()) {
        if row.len() != n_vars {
            continue;
        }
        let mut r = row.clone();
        r.push(rhs);
        aug.push(r);
    }

    let m = aug.len();
    let mut pivot_cols = Vec::new();
    let mut r_idx = 0;
    for col in 0..n_vars {
        if r_idx >= m {
            break;
        }
        let mut best_r = r_idx;
        let mut best_v = aug[r_idx][col].abs();
        for rr in (r_idx + 1)..m {
            if aug[rr][col].abs() > best_v {
                best_v = aug[rr][col].abs();
                best_r = rr;
            }
        }
        if best_v < 1e-11 {
            continue;
        }
        aug.swap(r_idx, best_r);
        let div = aug[r_idx][col];
        for k in col..=n_vars {
            aug[r_idx][k] /= div;
        }
        for rr in 0..m {
            if rr != r_idx && aug[rr][col].abs() > 1e-13 {
                let fac = aug[rr][col];
                for k in col..=n_vars {
                    aug[rr][k] -= fac * aug[r_idx][k];
                }
            }
        }
        pivot_cols.push(col);
        r_idx += 1;
    }

    // Check feasibility of dependent rows (0 = rhs?)
    for rr in r_idx..m {
        if aug[rr][n_vars].abs() > 1e-4 {
            return Err("Conflicting linear equality constraints in balance topology".into());
        }
    }

    let free_cols: Vec<usize> = (0..n_vars).filter(|c| !pivot_cols.contains(c)).collect();

    // Particular solution with free variables = 0
    let mut x0 = vec![0.0; n_vars];
    for (row_i, &p_col) in pivot_cols.iter().enumerate() {
        x0[p_col] = aug[row_i][n_vars];
    }

    // Build nullspace columns (one per free variable), then orthonormalize via Modified Gram-Schmidt
    let mut z_cols: Vec<Vec<f64>> = Vec::with_capacity(free_cols.len());
    for &f_col in &free_cols {
        let mut v = vec![0.0; n_vars];
        v[f_col] = 1.0;
        for (row_i, &p_col) in pivot_cols.iter().enumerate() {
            v[p_col] = -aug[row_i][f_col];
        }
        for prev in &z_cols {
            let dot: f64 = v.iter().zip(prev.iter()).map(|(a, b)| a * b).sum();
            for j in 0..n_vars {
                v[j] -= dot * prev[j];
            }
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for j in 0..n_vars {
                v[j] /= norm;
            }
            z_cols.push(v);
        }
    }

    Ok((x0, z_cols))
}

/// Solve `min_x 0.5 * x^T Q x - f^T x` subject to `C x = d`, plus posterior diag variance of `(H)^-1`.
/// Here `Q` is SPD/PSD quadratic matrix and `f` is linear term.
fn solve_equality_qp(
    q_mat: &[Vec<f64>],
    f_vec: &[f64],
    c_rows: &[Vec<f64>],
    d_vals: &[f64],
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let n = f_vec.len();
    let (x0, z_cols) = build_constraint_subspace(c_rows, d_vals, n)?;
    let k = z_cols.len();
    if k == 0 {
        return Ok((x0, vec![0.0; n]));
    }

    // Reduced Hessian H_red = Z^T Q Z (k x k) and gradient g_red = Z^T (f - Q x0)
    let mut q_x0 = vec![0.0; n];
    for i in 0..n {
        let mut s = 0.0;
        for j in 0..n {
            s += q_mat[i][j] * x0[j];
        }
        q_x0[i] = s;
    }

    let mut q_z = vec![vec![0.0; n]; k];
    for a in 0..k {
        for i in 0..n {
            let mut s = 0.0;
            for j in 0..n {
                s += q_mat[i][j] * z_cols[a][j];
            }
            q_z[a][i] = s;
        }
    }

    let mut h_red = vec![vec![0.0; k]; k];
    let mut g_red = vec![0.0; k];
    for a in 0..k {
        let mut g = 0.0;
        for i in 0..n {
            g += z_cols[a][i] * (f_vec[i] - q_x0[i]);
        }
        g_red[a] = g;
        for b in a..k {
            let mut h = 0.0;
            for i in 0..n {
                h += z_cols[a][i] * q_z[b][i];
            }
            h_red[a][b] = h;
            h_red[b][a] = h;
        }
        if h_red[a][a] < 1e-12 {
            h_red[a][a] += 1e-10;
        }
    }

    let y = solve_dense_linear(h_red.clone(), g_red)?;
    let mut x = x0;
    for a in 0..k {
        for i in 0..n {
            x[i] += z_cols[a][i] * y[a];
        }
    }

    // Posterior marginal variance diag(Z H_red^{-1} Z^T)
    let mut post_sd = vec![0.0; n];
    if let Some(h_inv) = invert_spd_matrix(&h_red) {
        for i in 0..n {
            let mut var = 0.0;
            for a in 0..k {
                let za = z_cols[a][i];
                if za.abs() < 1e-15 {
                    continue;
                }
                for b in 0..k {
                    var += za * h_inv[a][b] * z_cols[b][i];
                }
            }
            post_sd[i] = var.max(0.0).sqrt();
        }
    }

    Ok((x, post_sd))
}

/// Active-set Equality + Box Constrained QP (`min 0.5 x^T Q x - f^T x` s.t. `C x = d, lb <= x <= ub`).
fn solve_constrained_qp(
    q_mat: &[Vec<f64>],
    f_vec: &[f64],
    c_rows: &[Vec<f64>],
    d_vals: &[f64],
    lb: &[Option<f64>],
    ub: &[Option<f64>],
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let n = f_vec.len();
    let has_bounds = lb.iter().any(|b| b.is_some()) || ub.iter().any(|b| b.is_some());
    if !has_bounds {
        return solve_equality_qp(q_mat, f_vec, c_rows, d_vals);
    }

    // Build initial feasible estimate inside [lb, ub] intersecting C x = d via Dykstra/projection
    let (mut x_feas, _) = solve_equality_qp(q_mat, f_vec, c_rows, d_vals)?;
    let clamp_vec = |v: &mut [f64]| {
        for i in 0..n {
            if let Some(l) = lb[i] {
                if v[i] < l {
                    v[i] = l;
                }
            }
            if let Some(u) = ub[i] {
                if v[i] > u {
                    v[i] = u;
                }
            }
        }
    };

    let any_viol = x_feas.iter().enumerate().any(|(i, &val)| {
        lb[i].is_some_and(|l| val < l - 1e-8) || ub[i].is_some_and(|u| val > u + 1e-8)
    });
    if !any_viol {
        return solve_equality_qp(q_mat, f_vec, c_rows, d_vals);
    }

    // Project initial point onto intersection {C x = d} ∩ [lb, ub]
    let eye: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut r = vec![0.0; n];
            r[i] = 1.0;
            r
        })
        .collect();
    for _ in 0..40 {
        clamp_vec(&mut x_feas);
        if let Ok((proj, _)) = solve_equality_qp(&eye, &x_feas, c_rows, d_vals) {
            let diff: f64 = proj
                .iter()
                .zip(x_feas.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max);
            x_feas = proj;
            if diff < 1e-8 {
                break;
            }
        } else {
            break;
        }
    }
    clamp_vec(&mut x_feas);

    // Track active bounds: idx -> (is_upper, value)
    let mut active: BTreeMap<usize, (bool, f64)> = BTreeMap::new();
    for i in 0..n {
        if let Some(l) = lb[i] {
            if (x_feas[i] - l).abs() < 1e-7 {
                active.insert(i, (false, l));
            }
        }
        if let Some(u) = ub[i] {
            if (x_feas[i] - u).abs() < 1e-7 {
                active.entry(i).or_insert((true, u));
            }
        }
    }

    let mut post_sd = vec![0.0; n];
    let max_steps = 4 * n + 20;
    for _ in 0..max_steps {
        let mut c_ext = c_rows.to_vec();
        let mut d_ext = d_vals.to_vec();
        for (&idx, &(_, val)) in &active {
            let mut r = vec![0.0; n];
            r[idx] = 1.0;
            c_ext.push(r);
            d_ext.push(val);
        }

        let (x_cand, sd_cand) = match solve_equality_qp(q_mat, f_vec, &c_ext, &d_ext) {
            Ok(res) => res,
            Err(_) => break,
        };
        post_sd = sd_cand;

        // Check feasibility along step p = x_cand - x_feas
        let mut blocking_alpha = 1.0_f64;
        let mut blocking_bound: Option<(usize, bool, f64)> = None;
        for i in 0..n {
            if active.contains_key(&i) {
                continue;
            }
            let p = x_cand[i] - x_feas[i];
            if let Some(l) = lb[i] {
                if x_cand[i] < l - 1e-9 && p < -1e-12 {
                    let alpha = ((l - x_feas[i]) / p).clamp(0.0, 1.0);
                    if alpha < blocking_alpha {
                        blocking_alpha = alpha;
                        blocking_bound = Some((i, false, l));
                    }
                }
            }
            if let Some(u) = ub[i] {
                if x_cand[i] > u + 1e-9 && p > 1e-12 {
                    let alpha = ((u - x_feas[i]) / p).clamp(0.0, 1.0);
                    if alpha < blocking_alpha {
                        blocking_alpha = alpha;
                        blocking_bound = Some((i, true, u));
                    }
                }
            }
        }

        if let Some((b_idx, is_up, b_val)) = blocking_bound {
            for i in 0..n {
                x_feas[i] += blocking_alpha * (x_cand[i] - x_feas[i]);
            }
            x_feas[b_idx] = b_val;
            active.insert(b_idx, (is_up, b_val));
        } else {
            x_feas = x_cand;
            // Check KKT sign condition on active bounds via gradient g = Q x - f
            let mut grad = vec![0.0; n];
            for i in 0..n {
                let mut g = -f_vec[i];
                for j in 0..n {
                    g += q_mat[i][j] * x_feas[j];
                }
                grad[i] = g;
            }
            // Test dropping most favorable active bound (only if freeing it strictly lowers objective)
            let mut best_drop: Option<(usize, f64)> = None;
            for (&idx, &(is_up, _)) in &active {
                // Approximate multiplier test with feasible direction via sign of gradient vs unconstrained pull
                let viol = if is_up { grad[idx] } else { -grad[idx] };
                if viol > 1e-7 && best_drop.is_none_or(|(_, bv)| viol > bv) {
                    best_drop = Some((idx, viol));
                }
            }
            if let Some((drop_idx, _)) = best_drop {
                let backup = active.remove(&drop_idx);
                // Verify dropping actually moves strictly into feasible region; otherwise keep it active
                let mut c_test = c_rows.to_vec();
                let mut d_test = d_vals.to_vec();
                for (&idx, &(_, val)) in &active {
                    let mut r = vec![0.0; n];
                    r[idx] = 1.0;
                    c_test.push(r);
                    d_test.push(val);
                }
                if let Ok((x_try, _)) = solve_equality_qp(q_mat, f_vec, &c_test, &d_test) {
                    let move_dir = x_try[drop_idx] - x_feas[drop_idx];
                    if let Some((is_up, _)) = backup {
                        let enters_interior =
                            (!is_up && move_dir > 1e-10) || (is_up && move_dir < -1e-10);
                        if !enters_interior {
                            active.insert(drop_idx, (is_up, if is_up { ub[drop_idx].unwrap() } else { lb[drop_idx].unwrap() }));
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }
    }

    Ok((x_feas, post_sd))
}

/// Minimax Chebyshev norm solver (Lawson IRLS approximating Barrodale & Phillips 1975).
fn solve_minimax_wls(
    a_meas: &[Option<f64>],
    sd: &[f64],
    c_rows: &[Vec<f64>],
    d_vals: &[f64],
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let n = a_meas.len();
    let mut lawson_w = vec![1.0; n];
    let mut last_x = vec![0.0; n];
    let mut last_sd = vec![0.0; n];

    for _ in 0..16 {
        let mut q_mat = vec![vec![0.0; n]; n];
        let mut f_vec = vec![0.0; n];
        for i in 0..n {
            if let Some(a) = a_meas[i] {
                let s = sd[i].max(1e-6);
                let w = lawson_w[i] / (s * s);
                q_mat[i][i] = w;
                f_vec[i] = w * a;
            } else {
                q_mat[i][i] = 1e-12;
            }
        }
        let (x, post_sd) = solve_equality_qp(&q_mat, &f_vec, c_rows, d_vals)?;
        last_x = x;
        last_sd = post_sd;

        let mut errs = vec![0.0; n];
        let mut max_err = 0.0_f64;
        for i in 0..n {
            if let Some(a) = a_meas[i] {
                let e = (last_x[i] - a).abs() / sd[i].max(1e-6);
                errs[i] = e;
                max_err = max_err.max(e);
            }
        }
        if max_err < 1e-12 {
            break;
        }
        let mut sum_w = 0.0;
        for i in 0..n {
            if a_meas[i].is_some() {
                lawson_w[i] *= errs[i].max(1e-4);
                sum_w += lawson_w[i];
            }
        }
        if sum_w > 0.0 {
            let scale = n as f64 / sum_w;
            for i in 0..n {
                lawson_w[i] *= scale;
            }
        }
    }

    Ok((last_x, last_sd))
}

// ---------------------------------------------------------------------------
// Validation Rules (`51.6` Messages 1..12)
// ---------------------------------------------------------------------------

impl MassBalanceWorkbook {
    pub fn active_dataset(&self) -> Option<&MbDataset> {
        self.datasets.get(self.active_dataset)
    }

    pub fn active_dataset_mut(&mut self) -> Option<&mut MbDataset> {
        self.datasets.get_mut(self.active_dataset)
    }

    /// Generate human-readable balance equations (`Fig. 11`).
    pub fn build_equations_text(&self, only_non_sru: bool) -> Vec<String> {
        let mut out = Vec::new();
        for u in &self.units {
            if !u.selected || (only_non_sru && u.size_transforming) {
                continue;
            }
            let ins: Vec<String> = self
                .streams
                .iter()
                .filter(|s| s.selected && s.dest_unit == Some(u.id))
                .map(|s| s.name.clone())
                .collect();
            let outs: Vec<String> = self
                .streams
                .iter()
                .filter(|s| s.selected && s.source_unit == Some(u.id))
                .map(|s| s.name.clone())
                .collect();
            if !ins.is_empty() || !outs.is_empty() {
                out.push(format!("{} = {}", ins.join(" + "), outs.join(" + ")));
            }
        }
        for eq in &self.stream_equations {
            let tgt = self.streams.iter().find(|s| s.id == eq.target_stream_id).map(|s| s.name.as_str()).unwrap_or("?");
            let src = self.streams.iter().find(|s| s.id == eq.source_stream_id).map(|s| s.name.as_str()).unwrap_or("?");
            out.push(format!("{tgt} = {} * {src}", eq.factor));
        }
        out
    }

    /// Evaluate effective pre-balance measurement status for a cell.
    pub fn cell_pre_status(&self, ds_idx: usize, key: CellKey) -> PreMeasStatus {
        let ds = match self.datasets.get(ds_idx) {
            Some(d) => d,
            None => return PreMeasStatus::Missing,
        };
        let var_excluded_by_minerals = self.balancing_of_minerals
            && self
                .variables
                .iter()
                .find(|v| v.id == key.var_id)
                .is_some_and(|v| v.type_code == VarTypeCode::A);
        if var_excluded_by_minerals {
            return PreMeasStatus::Excluded;
        }
        if ds.frozen_values.contains_key(&key) {
            return PreMeasStatus::Fixed;
        }
        if let Some(&ov) = ds.meas_status_override.get(&key) {
            return ov;
        }
        if ds.estimated_cells.contains(&key) {
            return PreMeasStatus::Guesstimated;
        }
        if ds.values.contains_key(&key) {
            PreMeasStatus::Measured
        } else {
            PreMeasStatus::Missing
        }
    }

    /// Automatic stream type detection (`Detect Stream Types` in `2. Streams`).
    pub fn detect_stream_types(&mut self) {
        let ds_idx = self.active_dataset;
        let Some(ds) = self.datasets.get(ds_idx).cloned() else {
            return;
        };
        let sf_var = self.variables.iter().find(|v| v.selected && v.type_code == VarTypeCode::SF).map(|v| v.id);
        let water_var = self.variables.iter().find(|v| v.selected && v.type_code == VarTypeCode::SC).map(|v| v.id);

        for s in &mut self.streams {
            if s.stream_type != StreamType::Unknown {
                continue;
            }
            let name_lc = s.name.to_lowercase();
            if name_lc.contains("water") || name_lc.contains("sump") || name_lc.contains("wash") {
                let sf_zero = sf_var
                    .and_then(|vid| ds.values.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: vid }))
                    .is_none_or(|&v| v.abs() < 1e-9);
                let has_w = water_var
                    .and_then(|vid| ds.values.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: vid }))
                    .is_some_and(|&v| v > 1e-9);
                if has_w && sf_zero {
                    s.stream_type = StreamType::LiquidWater;
                    continue;
                }
            }
            s.stream_type = StreamType::SolidsSlurry;
        }
    }

    /// Unselect streams without any data (`Unselect Streams with no Data`).
    pub fn unselect_streams_without_data(&mut self) {
        let ds_idx = self.active_dataset;
        let Some(ds) = self.datasets.get(ds_idx) else {
            return;
        };
        for s in &mut self.streams {
            let has_any = ds.values.keys().any(|k| k.stream_id == s.id);
            if !has_any {
                s.selected = false;
            }
        }
    }

    /// Implement all `12` exact Error Check Rules (`51.6`).
    pub fn check_errors(&self, dataset_idx: usize) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let active_units: Vec<&MbUnit> = self.units.iter().filter(|u| u.selected).collect();
        let active_streams: Vec<&MbStream> = self.streams.iter().filter(|s| s.selected).collect();

        // 1. Units: No selected units found
        if active_units.is_empty() {
            issues.push(ValidationIssue {
                code: 1,
                severity: CheckSeverity::Error,
                title: "Units: No selected units found".into(),
                detail: "At least one unit should be selected".into(),
            });
        }
        // 2. Streams: No selected streams found
        if active_streams.is_empty() {
            issues.push(ValidationIssue {
                code: 2,
                severity: CheckSeverity::Error,
                title: "Streams: No selected streams found".into(),
                detail: "Some of the streams should be selected".into(),
            });
        }
        // 3 & 4. Units with no input or no output streams
        let mut missing_in = false;
        let mut missing_out = false;
        for u in &active_units {
            let has_in = active_streams.iter().any(|s| s.dest_unit == Some(u.id));
            let has_out = active_streams.iter().any(|s| s.source_unit == Some(u.id));
            if !has_in {
                missing_in = true;
            }
            if !has_out {
                missing_out = true;
            }
        }
        if missing_in {
            issues.push(ValidationIssue {
                code: 3,
                severity: CheckSeverity::Error,
                title: "Units: No input stream".into(),
                detail: "Found a unit with no input streams".into(),
            });
        }
        if missing_out {
            issues.push(ValidationIssue {
                code: 4,
                severity: CheckSeverity::Error,
                title: "Units: No output stream".into(),
                detail: "Found a unit with no output streams".into(),
            });
        }
        // 5. Streams: Source and destination missing
        if active_streams
            .iter()
            .any(|s| s.source_unit.is_none() && s.dest_unit.is_none())
        {
            issues.push(ValidationIssue {
                code: 5,
                severity: CheckSeverity::Error,
                title: "Streams: Source and destination missing".into(),
                detail: "Found a stream with no source and destination".into(),
            });
        }

        let ds = self.datasets.get(dataset_idx);
        // 6. Data: No data found
        if ds.is_none_or(|d| d.values.is_empty() && d.frozen_values.is_empty()) {
            issues.push(ValidationIssue {
                code: 6,
                severity: CheckSeverity::Error,
                title: "Data: No data found".into(),
                detail: "Add data in the measurement data view".into(),
            });
        }
        // 7. Variables: No selected assays found (when balance requires assays)
        let needs_assays = ds.is_none_or(|d| d.balance_type != BalanceType::SolidsOnly);
        let has_selected_assay = self
            .variables
            .iter()
            .any(|v| v.selected && v.type_code.is_assay());
        if needs_assays && !has_selected_assay {
            issues.push(ValidationIssue {
                code: 7,
                severity: CheckSeverity::Error,
                title: "Variables: No selected assays found".into(),
                detail: "There should be at least one selected variable of the Mineral Assay or Solids Component Assay type".into(),
            });
        }
        // 8 & 9. Stream Types unknown
        if !active_streams.is_empty() {
            let unknown_cnt = active_streams
                .iter()
                .filter(|s| s.stream_type == StreamType::Unknown)
                .count();
            if unknown_cnt == active_streams.len() {
                issues.push(ValidationIssue {
                    code: 8,
                    severity: CheckSeverity::Error,
                    title: "Streams: All stream types unknown".into(),
                    detail: "Go to Streams and press Detect Stream Types button or set the stream types manually".into(),
                });
            } else if unknown_cnt > 0 {
                issues.push(ValidationIssue {
                    code: 9,
                    severity: CheckSeverity::Error,
                    title: ".9 Stream Types: Unknown found".into(),
                    detail: "Go to Streams and press Detect Stream Types button or set the stream types manually".into(),
                });
            }
        }

        // 10. Units: No combined units found (checks if unit graph has active streams & at least 1 solids flow anchor)
        if let Some(d) = ds {
            let sf_vars: Vec<usize> = self
                .variables
                .iter()
                .filter(|v| v.selected && v.type_code == VarTypeCode::SF)
                .map(|v| v.id)
                .collect();
            let has_flow_anchor = d.values.iter().any(|(k, _)| sf_vars.contains(&k.var_id))
                || d.frozen_values.iter().any(|(k, _)| sf_vars.contains(&k.var_id));
            if !active_units.is_empty() && !active_streams.is_empty() && !d.values.is_empty() && !has_flow_anchor {
                issues.push(ValidationIssue {
                    code: 10,
                    severity: CheckSeverity::Error,
                    title: "Units: No combined units found".into(),
                    detail: "There can be several reasons why combined units cannot be formed. There may be too many missing measurements or there may be errors in the flowsheet.".into(),
                });
            }
            // 11. Notification: Min or Max values detected -> CLS recommended
            if (!d.min_bounds.is_empty() || !d.max_bounds.is_empty()) && d.method != SolveMethod::CLS {
                issues.push(ValidationIssue {
                    code: 11,
                    severity: CheckSeverity::Notification,
                    title: "Notification: Min or Max values detected: It is recommended to use the CLS method".into(),
                    detail: "Set Method to CLS to enforce defined Min/Max bounds.".into(),
                });
            }
            // 12. Notification: Method LLS not available for sized solution
            if d.method == SolveMethod::LLS && d.sizing != SizingMode::Unsized {
                issues.push(ValidationIssue {
                    code: 12,
                    severity: CheckSeverity::Notification,
                    title: "Notification: Method LLS not available for sized solution: It is recommended to use LS, NNLS, or CLS method".into(),
                    detail: "Select LS, NNLS, or CLS when Balancing Sized or Sized by Assay.".into(),
                });
            }
        }

        issues
    }
}

// ---------------------------------------------------------------------------
// Two-Stage EWTLS + WLS Solver Engine (`51.7`, Eqs. 1–29)
// ---------------------------------------------------------------------------

impl MassBalanceWorkbook {
    /// Run data reconciliation on a single dataset (updating `self.solutions[dataset_idx]`).
    pub fn calculate_dataset(&mut self, dataset_idx: usize) -> Result<MbSolution, String> {
        let issues = self.check_errors(dataset_idx);
        if let Some(first_err) = issues.iter().find(|i| i.severity == CheckSeverity::Error) {
            return Err(format!("{} — {}", first_err.title, first_err.detail));
        }
        let ds = self
            .datasets
            .get(dataset_idx)
            .cloned()
            .ok_or_else(|| "Dataset not found".to_string())?;

        if ds.method == SolveMethod::LLS && ds.sizing != SizingMode::Unsized {
            return Err(
                "Method LLS is not available for Sized or Sized by Assay modes (select LS, NNLS, or CLS)"
                    .into(),
            );
        }

        let active_units: Vec<MbUnit> = self.units.iter().filter(|u| u.selected).cloned().collect();
        let active_streams: Vec<MbStream> = self.streams.iter().filter(|s| s.selected).cloned().collect();
        let n_f = active_streams.len();
        let n_u = active_units.len();
        if n_f == 0 || n_u == 0 {
            return Err("No active streams or units selected".into());
        }

        // Stream index map
        let stream_pos: BTreeMap<usize, usize> =
            active_streams.iter().enumerate().map(|(i, s)| (s.id, i)).collect();

        // Full incidence matrix E (n_u x n_f): +1 input into unit, -1 output from unit
        let mut e_full = vec![vec![0.0_f64; n_f]; n_u];
        for (ui, u) in active_units.iter().enumerate() {
            for (fi, s) in active_streams.iter().enumerate() {
                let mut val = 0.0;
                if s.dest_unit == Some(u.id) {
                    val += 1.0;
                }
                if s.source_unit == Some(u.id) {
                    val -= 1.0;
                }
                e_full[ui][fi] = val;
            }
        }

        let feed_idx = self.resolve_feed_idx(&active_streams, &ds);
        let sf_var = self
            .variables
            .iter()
            .find(|v| v.selected && v.type_code == VarTypeCode::SF)
            .cloned();
        let water_var = self
            .variables
            .iter()
            .find(|v| v.selected && v.type_code == VarTypeCode::SC)
            .cloned();
        let sp_var = self
            .variables
            .iter()
            .find(|v| v.selected && v.type_code == VarTypeCode::SP)
            .cloned();

        // Active assay variables (excluding element variables `A` if Balancing of Minerals is ON)
        let active_assays: Vec<MbVariable> = self
            .variables
            .iter()
            .filter(|v| {
                if !v.selected {
                    return false;
                }
                match v.type_code {
                    VarTypeCode::A => !self.balancing_of_minerals,
                    VarTypeCode::M => true,
                    _ => false,
                }
            })
            .cloned()
            .collect();

        // Ensure sized modes resolve/verify bulk first
        match ds.sizing {
            SizingMode::Unsized => self.solve_unsized(
                dataset_idx,
                &ds,
                &active_units,
                &active_streams,
                &stream_pos,
                &e_full,
                feed_idx,
                sf_var.as_ref(),
                water_var.as_ref(),
                sp_var.as_ref(),
                &active_assays,
            ),
            SizingMode::Sized => self.solve_sized(
                dataset_idx,
                &ds,
                &active_units,
                &active_streams,
                &stream_pos,
                &e_full,
                feed_idx,
                sf_var.as_ref(),
                &active_assays,
            ),
            SizingMode::SizedByAssay => self.solve_sized_by_assay(
                dataset_idx,
                &ds,
                &active_units,
                &active_streams,
                &stream_pos,
                &e_full,
                feed_idx,
                sf_var.as_ref(),
                &active_assays,
            ),
        }
    }

    pub fn calculate_all_batch(&mut self) -> Vec<Result<MbSolution, String>> {
        let mut out = Vec::with_capacity(self.datasets.len());
        for idx in 0..self.datasets.len() {
            if self.datasets[idx].selected_for_batch {
                out.push(self.calculate_dataset(idx));
            }
        }
        out
    }

    fn resolve_feed_idx(&self, active_streams: &[MbStream], ds: &MbDataset) -> usize {
        if let Some(fid) = ds.feed_stream_id {
            if let Some(p) = active_streams.iter().position(|s| s.id == fid) {
                return p;
            }
        }
        // First selected boundary input stream whose source is None (external feed)
        if let Some(p) = active_streams
            .iter()
            .position(|s| s.source_unit.is_none() && s.stream_type == StreamType::SolidsSlurry)
        {
            return p;
        }
        active_streams
            .iter()
            .position(|s| s.source_unit.is_none())
            .unwrap_or(0)
    }

    fn effective_cell_value(&self, ds: &MbDataset, key: CellKey) -> (Option<f64>, PreMeasStatus) {
        let var_excluded = self.balancing_of_minerals
            && self
                .variables
                .iter()
                .find(|v| v.id == key.var_id)
                .is_some_and(|v| v.type_code == VarTypeCode::A);
        if var_excluded {
            return (ds.values.get(&key).copied(), PreMeasStatus::Excluded);
        }
        if let Some(&fv) = ds.frozen_values.get(&key) {
            return (Some(fv), PreMeasStatus::Fixed);
        }
        let status = ds
            .meas_status_override
            .get(&key)
            .copied()
            .unwrap_or_else(|| {
                if ds.estimated_cells.contains(&key) {
                    PreMeasStatus::Guesstimated
                } else if ds.values.contains_key(&key) {
                    PreMeasStatus::Measured
                } else {
                    PreMeasStatus::Missing
                }
            });
        if matches!(status, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
            return (ds.values.get(&key).copied(), status);
        }
        (ds.values.get(&key).copied(), status)
    }

    fn collect_bounds(
        &self,
        ds: &MbDataset,
        method: SolveMethod,
        keys: &[CellKey],
        sds: &[f64],
    ) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
        let mut lb = vec![None; keys.len()];
        let mut ub = vec![None; keys.len()];
        for (i, &k) in keys.iter().enumerate() {
            match method {
                SolveMethod::NNLS => lb[i] = Some(0.0),
                SolveMethod::CLS => {
                    lb[i] = ds.min_bounds.get(&k).copied();
                    ub[i] = ds.max_bounds.get(&k).copied();
                }
                SolveMethod::LLS => {
                    lb[i] = Some(sds[i] * (1.0 + 1e-6));
                }
                SolveMethod::LS | SolveMethod::LsMinMax => {}
            }
        }
        (lb, ub)
    }

    /// Solve Stage 1a: Bulk Total Solids flow rates `F` via iterative EWTLS (`Eqs. 2–4`).
    #[allow(clippy::too_many_arguments)]
    fn solve_bulk_flows_ewtls(
        &self,
        ds: &MbDataset,
        frac_idx: usize,
        active_streams: &[MbStream],
        stream_pos: &BTreeMap<usize, usize>,
        e_full: &[Vec<f64>],
        feed_idx: usize,
        sf_var: Option<&MbVariable>,
        extra_gm_cols: &[(Vec<Option<f64>>, Vec<f64>, Vec<bool>)], // (GM, SD_G, stream_has_assay)
        lb_f: &[Option<f64>],
        ub_f: &[Option<f64>],
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<(usize, f64)>, bool), String> {
        let n_f = active_streams.len();
        let n_u = e_full.len();

        let mut fm = vec![None; n_f];
        let mut s_f = vec![ds.null_sd_estimate.max(1e-3); n_f];
        let mut is_fixed_f = vec![false; n_f];
        let mut c_rows = e_full.to_vec();
        let mut d_vals = vec![0.0_f64; n_u];

        // Liquid/Water streams have zero total solids
        for (fi, s) in active_streams.iter().enumerate() {
            if s.stream_type == StreamType::LiquidWater {
                let mut row = vec![0.0; n_f];
                row[fi] = 1.0;
                c_rows.push(row);
                d_vals.push(0.0);
                is_fixed_f[fi] = true;
                continue;
            }
            if let Some(sfv) = sf_var {
                let key = CellKey { frac_idx, stream_id: s.id, var_id: sfv.id };
                let (val_opt, st) = self.effective_cell_value(ds, key);
                let ov = ds.sd_override.get(&key).copied();
                let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                    val_opt,
                    sfv,
                    s,
                    st,
                    ov,
                    ds.null_sd_estimate,
                    ds.low_grade_coeff,
                );
                s_f[fi] = sd_eff.max(ds.null_sd_estimate);
                if st == PreMeasStatus::Fixed {
                    if let Some(v) = val_opt {
                        let mut row = vec![0.0; n_f];
                        row[fi] = 1.0;
                        c_rows.push(row);
                        d_vals.push(v);
                        is_fixed_f[fi] = true;
                        fm[fi] = Some(v);
                    }
                } else if s.flows_enabled
                    && !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded)
                {
                    if let Some(v) = val_opt {
                        fm[fi] = Some(v);
                    }
                }
            }
        }

        // Stream by Equation linear constraints: F_tgt - factor * F_src = 0
        for eq in &self.stream_equations {
            if let (Some(&ti), Some(&si)) = (stream_pos.get(&eq.target_stream_id), stream_pos.get(&eq.source_stream_id)) {
                let mut row = vec![0.0; n_f];
                row[ti] = 1.0;
                row[si] = -eq.factor;
                c_rows.push(row);
                d_vals.push(0.0);
            }
        }

        // Apply optional MATLAB-notation assay scaling (`Fig. 17–18`)
        let mut gm_scaled_cols: Vec<(Vec<Option<f64>>, Vec<f64>)> = Vec::with_capacity(extra_gm_cols.len());
        if ds.scaling && !extra_gm_cols.is_empty() {
            let gm_dense: Vec<Vec<f64>> = (0..n_f)
                .map(|i| extra_gm_cols.iter().map(|c| c.0[i].unwrap_or(0.0)).collect())
                .collect();
            let sd_dense: Vec<Vec<f64>> = (0..n_f)
                .map(|i| extra_gm_cols.iter().map(|c| c.1[i]).collect())
                .collect();
            let gm_s = apply_assay_scaling(&gm_dense, &sd_dense, feed_idx);
            let sd_s = apply_assay_scaling(&sd_dense, &sd_dense, feed_idx);
            for (c_idx, col) in extra_gm_cols.iter().enumerate() {
                let mut g_col = vec![None; n_f];
                let mut s_col = vec![1.0; n_f];
                for i in 0..n_f {
                    if col.0[i].is_some() && col.2[i] {
                        g_col[i] = Some(gm_s[i][c_idx]);
                        s_col[i] = sd_s[i][c_idx].max(1e-9);
                    }
                }
                gm_scaled_cols.push((g_col, s_col));
            }
        } else {
            for col in extra_gm_cols {
                let g_col = col
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| if col.2[i] { v } else { None })
                    .collect();
                gm_scaled_cols.push((g_col, col.1.clone()));
            }
        }

        // Initial flow seed: uniform positive scale matching average measured flow
        let avg_flow = {
            let vals: Vec<f64> = fm.iter().flatten().copied().filter(|&v| v > 0.0).collect();
            if vals.is_empty() {
                100.0
            } else {
                vals.iter().sum::<f64>() / vals.len() as f64
            }
        };
        let mut f_cur = vec![avg_flow; n_f];
        for i in 0..n_f {
            if let Some(v) = fm[i] {
                f_cur[i] = v;
            }
        }
        // Project initial guess onto conservation manifold
        let eye: Vec<Vec<f64>> = (0..n_f)
            .map(|i| {
                let mut r = vec![0.0; n_f];
                r[i] = 1.0;
                r
            })
            .collect();
        if let Ok((proj, _)) = solve_equality_qp(&eye, &f_cur, &c_rows, &d_vals) {
            f_cur = proj;
        }

        let mut history = Vec::new();
        let mut post_sd = vec![0.0; n_f];
        let mut converged = false;

        let max_it = ds.max_iter.clamp(1, 2000);
        for iter in 0..max_it {
            let mut q_mat = vec![vec![0.0_f64; n_f]; n_f];
            let mut f_vec = vec![0.0_f64; n_f];

            for i in 0..n_f {
                if let Some(meas) = fm[i] {
                    if !is_fixed_f[i] {
                        let w = 1.0 / (s_f[i] * s_f[i]);
                        q_mat[i][i] += w;
                        f_vec[i] += w * meas;
                    }
                } else {
                    q_mat[i][i] += 1e-12;
                }
            }

            // Element-Wise Weighted Total Least Squares terms per unit and assay/fraction column:
            // r_{u,c}(F) = sum_j e_{u,j} * GM_{j,c} * F_j, variance = sum_j (e_{u,j} * s_{G,j,c} * F_cur_j)^2
            for u in 0..n_u {
                let mut unit_complete_assay = true;
                for i in 0..n_f {
                    if e_full[u][i].abs() > 0.5 && active_streams[i].stream_type != StreamType::LiquidWater {
                        // check any active column exists on this unit
                        if !gm_scaled_cols.iter().any(|c| c.0[i].is_some()) {
                            unit_complete_assay = false;
                            break;
                        }
                    }
                }
                if !unit_complete_assay {
                    continue;
                }
                for (g_col, sg_col) in &gm_scaled_cols {
                    // Only include equation if every non-water attached stream has this assay measured
                    let all_present = (0..n_f).all(|i| {
                        e_full[u][i].abs() < 0.5
                            || active_streams[i].stream_type == StreamType::LiquidWater
                            || g_col[i].is_some()
                    });
                    if !all_present {
                        continue;
                    }
                    let mut var_u = (ds.null_sd_estimate * 0.01).powi(2);
                    let mut coeffs = vec![0.0; n_f];
                    for i in 0..n_f {
                        if e_full[u][i].abs() > 0.5 {
                            let c_i = e_full[u][i] * g_col[i].unwrap_or(0.0);
                            coeffs[i] = c_i;
                            let s_term = e_full[u][i] * sg_col[i] * f_cur[i].abs().max(1.0);
                            var_u += s_term * s_term;
                        }
                    }
                    let w_u = 1.0 / var_u.max(1e-18);
                    for i in 0..n_f {
                        if coeffs[i].abs() < 1e-15 {
                            continue;
                        }
                        for j in 0..n_f {
                            if coeffs[j].abs() > 1e-15 {
                                q_mat[i][j] += w_u * coeffs[i] * coeffs[j];
                            }
                        }
                    }
                }
            }

            let (f_next, sd_next) = if ds.method == SolveMethod::LsMinMax {
                // Chebyshev fallback around current IRLS quadratic approximation
                solve_constrained_qp(&q_mat, &f_vec, &c_rows, &d_vals, lb_f, ub_f)?
            } else {
                solve_constrained_qp(&q_mat, &f_vec, &c_rows, &d_vals, lb_f, ub_f)?
            };

            let max_f = f_next.iter().map(|v| v.abs()).fold(1.0, f64::max);
            let tol_r = f_next
                .iter()
                .zip(f_cur.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max)
                / max_f;

            // Damping for large step oscillations
            for i in 0..n_f {
                f_cur[i] = f_next[i];
            }
            post_sd = sd_next;
            history.push((iter + 1, tol_r.max(1e-16)));
            if iter >= 1 && tol_r <= ds.balance_tolerance {
                converged = true;
                break;
            }
        }

        if history.is_empty() {
            history.push((1, 0.0));
            converged = true;
        }
        Ok((f_cur, post_sd, history, converged))
    }

    /// Solve Stage 1b: Water flow rates `W` using solved `F` + `WM` + `%Solids` (`Eqs. 5–7`).
    fn solve_water_flows_wls(
        &self,
        ds: &MbDataset,
        frac_idx: usize,
        active_streams: &[MbStream],
        e_full: &[Vec<f64>],
        f_bal: &[f64],
        water_var: Option<&MbVariable>,
        sp_var: Option<&MbVariable>,
        method: SolveMethod,
    ) -> Result<(Vec<f64>, Vec<f64>), String> {
        let n_f = active_streams.len();
        let mut q_mat = vec![vec![0.0_f64; n_f]; n_f];
        let mut f_vec = vec![0.0_f64; n_f];
        let mut c_rows = e_full.to_vec();
        let mut d_vals = vec![0.0; e_full.len()];
        let mut s_w_bounds = vec![ds.null_sd_estimate; n_f];

        for (fi, s) in active_streams.iter().enumerate() {
            // Water direct measurement (Eq. 6)
            if let Some(wv) = water_var {
                let k = CellKey { frac_idx, stream_id: s.id, var_id: wv.id };
                let (v_opt, st) = self.effective_cell_value(ds, k);
                let ov = ds.sd_override.get(&k).copied();
                let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                    v_opt,
                    wv,
                    s,
                    st,
                    ov,
                    ds.null_sd_estimate,
                    ds.low_grade_coeff,
                );
                s_w_bounds[fi] = sd_eff;
                if st == PreMeasStatus::Fixed {
                    if let Some(v) = v_opt {
                        let mut r = vec![0.0; n_f];
                        r[fi] = 1.0;
                        c_rows.push(r);
                        d_vals.push(v);
                    }
                } else if !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                    if let Some(v) = v_opt {
                        let w = 1.0 / (sd_eff * sd_eff);
                        q_mat[fi][fi] += w;
                        f_vec[fi] += w * v;
                    }
                }
            }

            // % Solids constraint linearized: (PcM/100)*W_i ≈ (1 - PcM/100)*F_i (`Eq. 7`)
            if let Some(sv) = sp_var {
                let k = CellKey { frac_idx, stream_id: s.id, var_id: sv.id };
                let (pc_opt, st) = self.effective_cell_value(ds, k);
                if !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                    if let Some(pcm) = pc_opt {
                        if pcm > 1e-4 && pcm < 99.9999 {
                            let ov = ds.sd_override.get(&k).copied();
                            let (_, _, _, sd_pc) = evaluate_cell_uncertainty(
                                Some(pcm),
                                sv,
                                s,
                                st,
                                ov,
                                ds.null_sd_estimate,
                                ds.low_grade_coeff,
                            );
                            let p = pcm / 100.0;
                            let target_w = f_bal[fi].abs() * (1.0 - p) / p;
                            // Error propagation dW / d(Pc) = F / (100 * p^2)
                            let dw_sd = (f_bal[fi].abs() * sd_pc / (100.0 * p * p)).max(ds.null_sd_estimate);
                            let w = 1.0 / (dw_sd * dw_sd);
                            q_mat[fi][fi] += w;
                            f_vec[fi] += w * target_w;
                        }
                    }
                }
            }

            if q_mat[fi][fi] < 1e-12 {
                q_mat[fi][fi] += 1e-12;
            }
        }

        let lb: Vec<Option<f64>> = (0..n_f)
            .map(|i| match method {
                SolveMethod::NNLS | SolveMethod::CLS => Some(0.0),
                SolveMethod::LLS => Some(s_w_bounds[i] * (1.0 + 1e-6)),
                _ => None,
            })
            .collect();
        let ub = vec![None; n_f];
        solve_constrained_qp(&q_mat, &f_vec, &c_rows, &d_vals, &lb, &ub)
    }

    /// Solve Stage 2: Bulk Assays (`Eqs. 8–11`) or Single Component WLS (`B * g = 0`).
    fn solve_bulk_assays_wls(
        &self,
        ds: &MbDataset,
        frac_idx: usize,
        active_streams: &[MbStream],
        e_full: &[Vec<f64>],
        f_bal: &[f64],
        assays: &[MbVariable],
        sum_100: bool,
    ) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>), String> {
        let n_f = active_streams.len();
        let n_e = assays.len();
        if n_e == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        // B_{unit, flow} = e_{flow}^{unit} * F_{flow, tot} (`Eq. 10`)
        let n_u_eff = if sum_100 && e_full.len() > 1 {
            e_full.len() - 1
        } else {
            e_full.len()
        };
        let mut b_mat = vec![vec![0.0_f64; n_f]; n_u_eff];
        for u in 0..n_u_eff {
            for i in 0..n_f {
                b_mat[u][i] = e_full[u][i] * f_bal[i];
            }
        }

        let total_vars = n_f * n_e;
        let mut a_meas = vec![None; total_vars];
        let mut s_vec = vec![ds.null_sd_estimate.max(1e-6); total_vars];
        let mut q_mat = vec![vec![0.0_f64; total_vars]; total_vars];
        let mut f_vec = vec![0.0_f64; total_vars];
        let mut c_rows = Vec::new();
        let mut d_vals = Vec::new();
        let mut lb = vec![None; total_vars];
        let mut ub = vec![None; total_vars];

        // Fill per-assay objective and per-unit mass conservation constraints `B * g_c = 0`
        for (c_idx, var) in assays.iter().enumerate() {
            let offset = c_idx * n_f;
            for (fi, s) in active_streams.iter().enumerate() {
                let k = CellKey { frac_idx, stream_id: s.id, var_id: var.id };
                let (val_opt, st) = self.effective_cell_value(ds, k);
                let ov = ds.sd_override.get(&k).copied();
                let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                    val_opt,
                    var,
                    s,
                    st,
                    ov,
                    ds.null_sd_estimate,
                    ds.low_grade_coeff,
                );
                let idx = offset + fi;
                s_vec[idx] = sd_eff.max(1e-6);

                if s.stream_type == StreamType::LiquidWater {
                    let mut r = vec![0.0; total_vars];
                    r[idx] = 1.0;
                    c_rows.push(r);
                    d_vals.push(0.0);
                    continue;
                }

                match ds.method {
                    SolveMethod::NNLS => lb[idx] = Some(0.0),
                    SolveMethod::CLS => {
                        lb[idx] = ds.min_bounds.get(&k).copied();
                        ub[idx] = ds.max_bounds.get(&k).copied();
                    }
                    SolveMethod::LLS => lb[idx] = Some(s_vec[idx] * (1.0 + 1e-6)),
                    SolveMethod::LS | SolveMethod::LsMinMax => {}
                }

                if st == PreMeasStatus::Fixed {
                    if let Some(v) = val_opt {
                        let mut r = vec![0.0; total_vars];
                        r[idx] = 1.0;
                        c_rows.push(r);
                        d_vals.push(v);
                        a_meas[idx] = Some(v);
                    }
                } else if s.assays_enabled && !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                    if let Some(v) = val_opt {
                        let w = 1.0 / (s_vec[idx] * s_vec[idx]);
                        q_mat[idx][idx] = w;
                        f_vec[idx] = w * v;
                        a_meas[idx] = Some(v);
                    } else {
                        q_mat[idx][idx] = 1e-12;
                    }
                } else {
                    q_mat[idx][idx] = 1e-12;
                }
            }

            // Unit conservation for assay component c: B * G[:, c] = 0
            for b_row in &b_mat {
                let mut r = vec![0.0; total_vars];
                for fi in 0..n_f {
                    r[offset + fi] = b_row[fi];
                }
                c_rows.push(r);
                d_vals.push(0.0);
            }
        }

        // Optional sum = 100 constraint (`Eq. 11`)
        if sum_100 {
            for (fi, s) in active_streams.iter().enumerate() {
                if s.stream_type == StreamType::LiquidWater {
                    continue;
                }
                let mut r = vec![0.0; total_vars];
                for c_idx in 0..n_e {
                    r[c_idx * n_f + fi] = 1.0;
                }
                c_rows.push(r);
                d_vals.push(100.0);
            }
        }

        // Can solve faster per-component if not coupled by sum_100, else solve coupled
        let (vec_g, vec_sd) = if !sum_100 {
            let mut g_all = vec![0.0; total_vars];
            let mut sd_all = vec![0.0; total_vars];
            for c_idx in 0..n_e {
                let off = c_idx * n_f;
                let q_sub: Vec<Vec<f64>> = (0..n_f)
                    .map(|i| {
                        let mut r = vec![0.0; n_f];
                        r[i] = q_mat[off + i][off + i];
                        r
                    })
                    .collect();
                let f_sub: Vec<f64> = f_vec[off..(off + n_f)].to_vec();
                let mut c_sub = b_mat.clone();
                let mut d_sub = vec![0.0; b_mat.len()];
                // copy stream-local fixed/water rows for component c_idx
                for (r_full, &rhs_full) in c_rows.iter().zip(d_vals.iter()) {
                    let has_only_c = r_full[off..(off + n_f)].iter().any(|&v| v.abs() > 1e-15)
                        && (0..total_vars)
                            .all(|j| (j >= off && j < off + n_f) || r_full[j].abs() < 1e-15);
                    let is_unit_eq = b_mat.iter().any(|br| {
                        (0..n_f).all(|j| (br[j] - r_full[off + j]).abs() < 1e-12)
                    });
                    if has_only_c && !is_unit_eq {
                        c_sub.push(r_full[off..(off + n_f)].to_vec());
                        d_sub.push(rhs_full);
                    }
                }
                let (gx, gsd) = if ds.method == SolveMethod::LsMinMax {
                    solve_minimax_wls(
                        &a_meas[off..(off + n_f)],
                        &s_vec[off..(off + n_f)],
                        &c_sub,
                        &d_sub,
                    )?
                } else {
                    solve_constrained_qp(
                        &q_sub,
                        &f_sub,
                        &c_sub,
                        &d_sub,
                        &lb[off..(off + n_f)],
                        &ub[off..(off + n_f)],
                    )?
                };
                g_all[off..(off + n_f)].copy_from_slice(&gx);
                sd_all[off..(off + n_f)].copy_from_slice(&gsd);
            }
            (g_all, sd_all)
        } else if ds.method == SolveMethod::LsMinMax {
            solve_minimax_wls(&a_meas, &s_vec, &c_rows, &d_vals)?
        } else {
            solve_constrained_qp(&q_mat, &f_vec, &c_rows, &d_vals, &lb, &ub)?
        };

        let mut g_mat = vec![vec![0.0; n_e]; n_f];
        let mut g_sd = vec![vec![0.0; n_e]; n_f];
        for c_idx in 0..n_e {
            for fi in 0..n_f {
                g_mat[fi][c_idx] = vec_g[c_idx * n_f + fi];
                g_sd[fi][c_idx] = vec_sd[c_idx * n_f + fi];
            }
        }
        Ok((g_mat, g_sd))
    }

    // -----------------------------------------------------------------------
    // Mode Drivers: Unsized (`51.7.1`), Sized (`51.7.2`), SizedByAssay (`51.7.3`)
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn solve_unsized(
        &mut self,
        dataset_idx: usize,
        ds: &MbDataset,
        active_units: &[MbUnit],
        active_streams: &[MbStream],
        stream_pos: &BTreeMap<usize, usize>,
        e_full: &[Vec<f64>],
        feed_idx: usize,
        sf_var: Option<&MbVariable>,
        water_var: Option<&MbVariable>,
        sp_var: Option<&MbVariable>,
        active_assays: &[MbVariable],
    ) -> Result<MbSolution, String> {
        let n_f = active_streams.len();
        let frac_idx = 0;

        // Collect assay matrices GM & SD_GM for Stage 1 EWTLS
        let mut extra_gm = Vec::new();
        if ds.balance_type != BalanceType::SolidsOnly {
            for var in active_assays {
                let mut g_col = vec![None; n_f];
                let mut s_col = vec![ds.null_sd_estimate; n_f];
                let mut has_col = vec![false; n_f];
                for (fi, s) in active_streams.iter().enumerate() {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: var.id };
                    let (v_opt, st) = self.effective_cell_value(ds, k);
                    let ov = ds.sd_override.get(&k).copied();
                    let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                        v_opt,
                        var,
                        s,
                        st,
                        ov,
                        ds.null_sd_estimate,
                        ds.low_grade_coeff,
                    );
                    s_col[fi] = sd_eff;
                    if s.assays_enabled && !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                        g_col[fi] = v_opt;
                        has_col[fi] = v_opt.is_some();
                    }
                }
                extra_gm.push((g_col, s_col, has_col));
            }
        }

        // Stage 1a: Solids flows F
        let (f_bal, f_post_sd, conv_rounds, converged) = if ds.balance_type == BalanceType::AssaysOnly {
            // Require measured/fixed SF on every balanced solids flow (`Table 1`)
            let mut f_fix = vec![0.0; n_f];
            if let Some(sfv) = sf_var {
                for (fi, s) in active_streams.iter().enumerate() {
                    if s.stream_type == StreamType::LiquidWater {
                        continue;
                    }
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sfv.id };
                    let (v_opt, _) = self.effective_cell_value(ds, k);
                    f_fix[fi] = v_opt.ok_or_else(|| {
                        format!("Assays only balance requires Total Solids flow rate for stream '{}'", s.name)
                    })?;
                }
            }
            (f_fix, vec![0.0; n_f], vec![(1, 0.0)], true)
        } else {
            let flow_keys: Vec<CellKey> = active_streams
                .iter()
                .map(|s| CellKey {
                    frac_idx,
                    stream_id: s.id,
                    var_id: sf_var.map(|v| v.id).unwrap_or(0),
                })
                .collect();
            let flow_sds: Vec<f64> = active_streams
                .iter()
                .map(|s| {
                    sf_var.map_or(ds.null_sd_estimate, |v| {
                        let k = CellKey { frac_idx, stream_id: s.id, var_id: v.id };
                        let (vo, st) = self.effective_cell_value(ds, k);
                        evaluate_cell_uncertainty(
                            vo, v, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                        ).3
                    })
                })
                .collect();
            let (lb_f, ub_f) = self.collect_bounds(ds, ds.method, &flow_keys, &flow_sds);
            self.solve_bulk_flows_ewtls(
                ds, frac_idx, active_streams, stream_pos, e_full, feed_idx, sf_var, &extra_gm, &lb_f, &ub_f,
            )?
        };

        // Stage 1b: Optional Water flows (`Solids, Water and Assays`)
        let (w_bal, _w_sd) = if ds.balance_type == BalanceType::SolidsWaterAndAssays {
            self.solve_water_flows_wls(
                ds, frac_idx, active_streams, e_full, &f_bal, water_var, sp_var, ds.method,
            )?
        } else {
            (vec![0.0; n_f], vec![0.0; n_f])
        };

        // Stage 2: Assays
        let sum_100 = (ds.assay_sum_100 && !self.balancing_of_minerals)
            || (ds.mineral_sum_100 && self.balancing_of_minerals);
        let (g_mat, g_sd) = if ds.balance_type == BalanceType::SolidsOnly || active_assays.is_empty() {
            (vec![vec![0.0; active_assays.len()]; n_f], vec![vec![0.0; active_assays.len()]; n_f])
        } else {
            self.solve_bulk_assays_wls(
                ds, frac_idx, active_streams, e_full, &f_bal, active_assays, sum_100,
            )?
        };

        let mut sol = self.assemble_solution(
            dataset_idx,
            ds,
            active_units,
            active_streams,
            e_full,
            feed_idx,
            frac_idx,
            sf_var,
            water_var,
            sp_var,
            active_assays,
            &f_bal,
            &f_post_sd,
            &w_bal,
            &g_mat,
            &g_sd,
            conv_rounds,
            converged,
        );
        self.store_solution(dataset_idx, &sol);
        if sol.last_message.is_empty() {
            sol.last_message = "Balanced".into();
        }
        Ok(sol)
    }

    /// Solve `Sized` mode (`51.7.2`, Eqs. 12–18): uses bulk `F` + size fraction `SA` across non-SRUs.
    #[allow(clippy::too_many_arguments)]
    fn solve_sized(
        &mut self,
        dataset_idx: usize,
        ds: &MbDataset,
        active_units: &[MbUnit],
        active_streams: &[MbStream],
        stream_pos: &BTreeMap<usize, usize>,
        e_full: &[Vec<f64>],
        feed_idx: usize,
        sf_var: Option<&MbVariable>,
        active_assays: &[MbVariable],
    ) -> Result<MbSolution, String> {
        let n_f = active_streams.len();
        let sa_var = self
            .variables
            .iter()
            .find(|v| v.selected && v.type_code == VarTypeCode::SA)
            .cloned();
        let n_sieves = self.size_fractions.len().saturating_sub(1);

        // Extra GM columns include bulk assays + non-SRU fraction% measurements (`Eqs. 13, 15`)
        let mut extra_gm = Vec::new();
        for var in active_assays {
            let mut g_col = vec![None; n_f];
            let mut s_col = vec![ds.null_sd_estimate; n_f];
            let mut has_col = vec![false; n_f];
            for (fi, s) in active_streams.iter().enumerate() {
                let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: var.id };
                let (v_opt, st) = self.effective_cell_value(ds, k);
                let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                    v_opt, var, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                );
                s_col[fi] = sd_eff;
                if s.assays_enabled && !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                    g_col[fi] = v_opt;
                    has_col[fi] = v_opt.is_some();
                }
            }
            extra_gm.push((g_col, s_col, has_col));
        }

        if let Some(sav) = &sa_var {
            for frac_idx in 1..self.size_fractions.len() {
                let mut m_col = vec![None; n_f];
                let mut s_col = vec![ds.null_sd_estimate; n_f];
                let mut has_col = vec![false; n_f];
                for (fi, s) in active_streams.iter().enumerate() {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sav.id };
                    let (v_opt, st) = self.effective_cell_value(ds, k);
                    let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                        v_opt, sav, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                    );
                    s_col[fi] = sd_eff;
                    if !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                        m_col[fi] = v_opt;
                        has_col[fi] = v_opt.is_some();
                    }
                }
                extra_gm.push((m_col, s_col, has_col));
            }
        }

        let flow_keys: Vec<CellKey> = active_streams
            .iter()
            .map(|s| CellKey {
                frac_idx: 0,
                stream_id: s.id,
                var_id: sf_var.map(|v| v.id).unwrap_or(0),
            })
            .collect();
        let (lb_f, ub_f) = self.collect_bounds(ds, ds.method, &flow_keys, &vec![0.0; n_f]);
        let (f_bal, f_post_sd, conv_rounds, converged) = self.solve_bulk_flows_ewtls(
            ds, 0, active_streams, stream_pos, e_full, feed_idx, sf_var, &extra_gm, &lb_f, &ub_f,
        )?;

        // Stage 2: Size fraction %m solved across non-SRU units (`Eqs. 16–18`)
        let non_sru_e: Vec<Vec<f64>> = active_units
            .iter()
            .enumerate()
            .filter(|(_, u)| !u.size_transforming)
            .map(|(ui, _)| e_full[ui].clone())
            .collect();

        let (m_mat, m_sd) = if let (Some(sav), true) = (&sa_var, n_sieves > 0) {
            let sieve_vars: Vec<MbVariable> = (0..n_sieves).map(|_| sav.clone()).collect();
            // Solve coupled M_{flow, subflow} using same block WLS engine with sum_100 = true (`Eq. 18`)
            let n_u_eff = non_sru_e.len().saturating_sub(1).max(1);
            let mut b_non_sru = vec![vec![0.0; n_f]; n_u_eff.min(non_sru_e.len())];
            for u in 0..b_non_sru.len() {
                for i in 0..n_f {
                    b_non_sru[u][i] = non_sru_e[u][i] * f_bal[i];
                }
            }
            let total = n_f * n_sieves;
            let mut q_m = vec![vec![0.0; total]; total];
            let mut f_m = vec![0.0; total];
            let mut c_m = Vec::new();
            let mut d_m = Vec::new();
            let mut lb_m = vec![None; total];
            let mut ub_m = vec![None; total];

            for sf_i in 0..n_sieves {
                let frac_idx = sf_i + 1;
                let off = sf_i * n_f;
                for (fi, s) in active_streams.iter().enumerate() {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sav.id };
                    let (vo, st) = self.effective_cell_value(ds, k);
                    let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                        vo, sav, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                    );
                    let idx = off + fi;
                    if ds.method == SolveMethod::NNLS {
                        lb_m[idx] = Some(0.0);
                    } else if ds.method == SolveMethod::CLS {
                        lb_m[idx] = ds.min_bounds.get(&k).copied().or(Some(0.0));
                        ub_m[idx] = ds.max_bounds.get(&k).copied().or(Some(100.0));
                    }
                    if st == PreMeasStatus::Fixed {
                        if let Some(v) = vo {
                            let mut r = vec![0.0; total];
                            r[idx] = 1.0;
                            c_m.push(r);
                            d_m.push(v);
                        }
                    } else if !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                        if let Some(v) = vo {
                            let w = 1.0 / (sd_eff * sd_eff);
                            q_m[idx][idx] = w;
                            f_m[idx] = w * v;
                        } else {
                            q_m[idx][idx] = 1e-12;
                        }
                    } else {
                        q_m[idx][idx] = 1e-12;
                    }
                }
                for br in &b_non_sru {
                    let mut r = vec![0.0; total];
                    for fi in 0..n_f {
                        r[off + fi] = br[fi];
                    }
                    c_m.push(r);
                    d_m.push(0.0);
                }
            }
            for fi in 0..n_f {
                if active_streams[fi].stream_type == StreamType::LiquidWater {
                    continue;
                }
                let mut r = vec![0.0; total];
                for sf_i in 0..n_sieves {
                    r[sf_i * n_f + fi] = 1.0;
                }
                c_m.push(r);
                d_m.push(100.0);
            }
            let _ = sieve_vars;
            solve_constrained_qp(&q_m, &f_m, &c_m, &d_m, &lb_m, &ub_m)?
        } else {
            (Vec::new(), Vec::new())
        };

        // Stage 3: Bulk assays (`Eqs. 8–11`)
        let sum_100 = (ds.assay_sum_100 && !self.balancing_of_minerals)
            || (ds.mineral_sum_100 && self.balancing_of_minerals);
        let (g_mat, g_sd) = self.solve_bulk_assays_wls(
            ds, 0, active_streams, e_full, &f_bal, active_assays, sum_100,
        )?;

        let water_var = self.variables.iter().find(|v| v.selected && v.type_code == VarTypeCode::SC);
        let sp_var = self.variables.iter().find(|v| v.selected && v.type_code == VarTypeCode::SP);
        let w_bal = vec![0.0; n_f];

        let mut sol = self.assemble_solution(
            dataset_idx, ds, active_units, active_streams, e_full, feed_idx, 0, sf_var, water_var,
            sp_var, active_assays, &f_bal, &f_post_sd, &w_bal, &g_mat, &g_sd, conv_rounds, converged,
        );

        // Attach size fraction cells `1..size_fractions.len()`
        if let (Some(sav), true) = (&sa_var, n_sieves > 0) {
            for sf_i in 0..n_sieves {
                let frac_idx = sf_i + 1;
                for (fi, s) in active_streams.iter().enumerate() {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sav.id };
                    let idx = sf_i * n_f + fi;
                    let bal = m_mat[idx];
                    let sd_post = m_sd[idx];
                    let (meas, pre_st) = self.effective_cell_value(ds, k);
                    let diff = meas.map(|m| m - bal);
                    let rel_diff = meas.and_then(|m| if m.abs() > 1e-12 { Some(100.0 * (m - bal).abs() / m.abs()) } else { None });
                    let (sd_meas, rsd_pct, sd_samp, _) = evaluate_cell_uncertainty(
                        meas, sav, s, pre_st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                    );
                    sol.cells.insert(
                        k,
                        ReconciliationCell {
                            key: k,
                            measured: meas,
                            balanced: Some(bal),
                            ci: Some(1.96 * sd_post),
                            diff,
                            rel_diff_pct: rel_diff,
                            min_bound: ds.min_bounds.get(&k).copied(),
                            max_bound: ds.max_bounds.get(&k).copied(),
                            rsd_pct: Some(rsd_pct),
                            sd_eff: Some(sd_meas),
                            sd_meas: Some(sd_meas),
                            sd_sampling: sd_samp,
                            recovery_pct: None,
                            pre_status: pre_st,
                            bal_status: if meas.is_some() { PostBalStatus::Balanced } else { PostBalStatus::Calculated },
                            is_frozen: ds.frozen_values.contains_key(&k),
                            is_estimated: ds.estimated_cells.contains(&k),
                        },
                    );
                }
            }
        }
        self.store_solution(dataset_idx, &sol);
        Ok(sol)
    }

    /// Solve `Sized by Assay` (`51.7.3`, Eqs. 19–29): uses solved bulk totals, then solves subflows + assays per fraction.
    #[allow(clippy::too_many_arguments)]
    fn solve_sized_by_assay(
        &mut self,
        dataset_idx: usize,
        ds: &MbDataset,
        active_units: &[MbUnit],
        active_streams: &[MbStream],
        stream_pos: &BTreeMap<usize, usize>,
        e_full: &[Vec<f64>],
        feed_idx: usize,
        sf_var: Option<&MbVariable>,
        active_assays: &[MbVariable],
    ) -> Result<MbSolution, String> {
        // Prerequisite: solve bulk (`Unsized` / `Sized` totals `F_tot` and `G_tot`)
        let bulk_sol = self.solve_sized(
            dataset_idx, ds, active_units, active_streams, stream_pos, e_full, feed_idx, sf_var, active_assays,
        )?;
        let n_f = active_streams.len();
        let n_s = self.size_fractions.len().saturating_sub(1);
        if n_s == 0 {
            return Ok(bulk_sol);
        }

        let mut f_tot = vec![0.0; n_f];
        if let Some(sfv) = sf_var {
            for (fi, s) in active_streams.iter().enumerate() {
                let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: sfv.id };
                f_tot[fi] = bulk_sol.cells.get(&k).and_then(|c| c.balanced).unwrap_or(0.0);
            }
        }
        let mut g_tot = vec![vec![0.0; active_assays.len()]; n_f];
        for (fi, s) in active_streams.iter().enumerate() {
            for (ci, av) in active_assays.iter().enumerate() {
                let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: av.id };
                g_tot[fi][ci] = bulk_sol.cells.get(&k).and_then(|c| c.balanced).unwrap_or(0.0);
            }
        }

        // Solve size sub-flows `F_{flow, subflow}` across non-SRUs (`Eqs. 19–23`)
        let sa_var = self.variables.iter().find(|v| v.selected && v.type_code == VarTypeCode::SA).cloned();
        let non_sru_indices: Vec<usize> = active_units
            .iter()
            .enumerate()
            .filter(|(_, u)| !u.size_transforming)
            .map(|(i, _)| i)
            .collect();
        let total_sub = n_f * n_s;
        let mut q_sub = vec![vec![0.0; total_sub]; total_sub];
        let mut f_sub = vec![0.0; total_sub];
        let mut c_sub = Vec::new();
        let mut d_sub = Vec::new();
        let mut lb_sub = vec![None; total_sub];
        let ub_sub = vec![None; total_sub];

        for sf_i in 0..n_s {
            let frac_idx = sf_i + 1;
            let off = sf_i * n_f;
            for (fi, s) in active_streams.iter().enumerate() {
                let idx = off + fi;
                if ds.method == SolveMethod::NNLS || ds.method == SolveMethod::CLS {
                    lb_sub[idx] = Some(0.0);
                }
                if let Some(sav) = &sa_var {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sav.id };
                    let (mm_opt, st) = self.effective_cell_value(ds, k);
                    if !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                        if let Some(mm) = mm_opt {
                            let f_m = mm * f_tot[fi] / 100.0;
                            let (_, _, _, sd_m) = evaluate_cell_uncertainty(
                                Some(mm), sav, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                            );
                            let sd_fsub = (sd_m * f_tot[fi].abs() / 100.0).max(ds.null_sd_estimate);
                            let w = 1.0 / (sd_fsub * sd_fsub);
                            q_sub[idx][idx] = w;
                            f_sub[idx] = w * f_m;
                        }
                    }
                }
                if q_sub[idx][idx] < 1e-12 {
                    q_sub[idx][idx] = 1e-12;
                }
            }
            // Eq. 19: conservation per non-SRU (up to non_sru - 1 if > 1)
            let take_u = if non_sru_indices.len() > 1 { non_sru_indices.len() - 1 } else { non_sru_indices.len() };
            for &ui in non_sru_indices.iter().take(take_u) {
                let mut r = vec![0.0; total_sub];
                for fi in 0..n_f {
                    r[off + fi] = e_full[ui][fi];
                }
                c_sub.push(r);
                d_sub.push(0.0);
            }
        }
        // Eq. 20: sum_{subflow} F_{flow, subflow} = F_{flow, tot}
        for fi in 0..n_f {
            let mut r = vec![0.0; total_sub];
            for sf_i in 0..n_s {
                r[sf_i * n_f + fi] = 1.0;
            }
            c_sub.push(r);
            d_sub.push(f_tot[fi]);
        }

        let (f_sub_bal, f_sub_sd) = solve_constrained_qp(&q_sub, &f_sub, &c_sub, &d_sub, &lb_sub, &ub_sub)?;

        // Stage 2: Sized fraction assays (`Eqs. 24–29`) using block-diagonal B (`Eq. 26`) + head grade closure (`Eq. 28`)
        let n_e = active_assays.len();
        let total_ga = n_f * n_s * n_e;
        let sum_100 = (ds.assay_sum_100 && !self.balancing_of_minerals)
            || (ds.mineral_sum_100 && self.balancing_of_minerals);
        let mut out_sol = bulk_sol;

        if n_e > 0 {
            let mut q_ga = vec![vec![0.0; total_ga]; total_ga];
            let mut f_ga = vec![0.0; total_ga];
            let mut c_ga = Vec::new();
            let mut d_ga = Vec::new();
            let mut lb_ga = vec![None; total_ga];
            let mut ub_ga = vec![None; total_ga];

            let idx_of = |c_idx: usize, sf_i: usize, fi: usize| -> usize {
                (c_idx * n_s + sf_i) * n_f + fi
            };

            for (c_idx, var) in active_assays.iter().enumerate() {
                for sf_i in 0..n_s {
                    let frac_idx = sf_i + 1;
                    for (fi, s) in active_streams.iter().enumerate() {
                        let k = CellKey { frac_idx, stream_id: s.id, var_id: var.id };
                        let (vo, st) = self.effective_cell_value(ds, k);
                        let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                            vo, var, s, st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                        );
                        let gi = idx_of(c_idx, sf_i, fi);
                        if ds.method == SolveMethod::NNLS {
                            lb_ga[gi] = Some(0.0);
                        } else if ds.method == SolveMethod::CLS {
                            lb_ga[gi] = ds.min_bounds.get(&k).copied();
                            ub_ga[gi] = ds.max_bounds.get(&k).copied();
                        }
                        if st == PreMeasStatus::Fixed {
                            if let Some(v) = vo {
                                let mut r = vec![0.0; total_ga];
                                r[gi] = 1.0;
                                c_ga.push(r);
                                d_ga.push(v);
                            }
                        } else if s.assays_enabled && !matches!(st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                            if let Some(v) = vo {
                                let w = 1.0 / (sd_eff * sd_eff);
                                q_ga[gi][gi] = w;
                                f_ga[gi] = w * v;
                            } else {
                                q_ga[gi][gi] = 1e-12;
                            }
                        } else {
                            q_ga[gi][gi] = 1e-12;
                        }
                    }

                    // Eq. 25–27: B_subflow * G_subflow = 0 for subflows 0..N_S-1 across non-SRUs
                    if sf_i < n_s.saturating_sub(1) || n_s == 1 {
                        let take_u = if sum_100 && non_sru_indices.len() > 1 {
                            non_sru_indices.len() - 1
                        } else {
                            non_sru_indices.len()
                        };
                        for &ui in non_sru_indices.iter().take(take_u) {
                            let mut r = vec![0.0; total_ga];
                            for fi in 0..n_f {
                                r[idx_of(c_idx, sf_i, fi)] = e_full[ui][fi] * f_sub_bal[sf_i * n_f + fi];
                            }
                            c_ga.push(r);
                            d_ga.push(0.0);
                        }
                    }
                }

                // Eq. 28: sum_{subflow} F_{flow,subflow} * G_{flow,c,subflow} = F_{flow,tot} * G_{flow,tot,c}
                for fi in 0..n_f {
                    if f_tot[fi].abs() < 1e-12 {
                        continue;
                    }
                    let mut r = vec![0.0; total_ga];
                    for sf_i in 0..n_s {
                        r[idx_of(c_idx, sf_i, fi)] = f_sub_bal[sf_i * n_f + fi];
                    }
                    c_ga.push(r);
                    d_ga.push(f_tot[fi] * g_tot[fi][c_idx]);
                }
            }

            // Eq. 29: sum_{element} G = 100
            if sum_100 {
                for sf_i in 0..n_s {
                    for (fi, s) in active_streams.iter().enumerate() {
                        if s.stream_type == StreamType::LiquidWater {
                            continue;
                        }
                        let mut r = vec![0.0; total_ga];
                        for c_idx in 0..n_e {
                            r[idx_of(c_idx, sf_i, fi)] = 1.0;
                        }
                        c_ga.push(r);
                        d_ga.push(100.0);
                    }
                }
            }

            let (ga_bal, ga_sd) = solve_constrained_qp(&q_ga, &f_ga, &c_ga, &d_ga, &lb_ga, &ub_ga)?;
            for (c_idx, var) in active_assays.iter().enumerate() {
                for sf_i in 0..n_s {
                    let frac_idx = sf_i + 1;
                    for (fi, s) in active_streams.iter().enumerate() {
                        let k = CellKey { frac_idx, stream_id: s.id, var_id: var.id };
                        let gi = idx_of(c_idx, sf_i, fi);
                        let bal = ga_bal[gi];
                        let (meas, pre_st) = self.effective_cell_value(ds, k);
                        let (sd_meas, rsd_pct, sd_samp, _) = evaluate_cell_uncertainty(
                            meas, var, s, pre_st, ds.sd_override.get(&k).copied(), ds.null_sd_estimate, ds.low_grade_coeff,
                        );
                        let diff = meas.map(|m| m - bal);
                        let rel_diff = meas.and_then(|m| if m.abs() > 1e-12 { Some(100.0 * (m - bal).abs() / m.abs()) } else { None });
                        out_sol.cells.insert(
                            k,
                            ReconciliationCell {
                                key: k,
                                measured: meas,
                                balanced: Some(bal),
                                ci: Some(1.96 * ga_sd[gi]),
                                diff,
                                rel_diff_pct: rel_diff,
                                min_bound: ds.min_bounds.get(&k).copied(),
                                max_bound: ds.max_bounds.get(&k).copied(),
                                rsd_pct: Some(rsd_pct),
                                sd_eff: Some(sd_meas),
                                sd_meas: Some(sd_meas),
                                sd_sampling: sd_samp,
                                recovery_pct: None,
                                pre_status: pre_st,
                                bal_status: if meas.is_some() { PostBalStatus::Balanced } else { PostBalStatus::Calculated },
                                is_frozen: ds.frozen_values.contains_key(&k),
                                is_estimated: ds.estimated_cells.contains(&k),
                            },
                        );
                    }
                }
            }
        }

        // Also update subflow SF records if SF variable exists
        if let Some(sfv) = sf_var {
            for sf_i in 0..n_s {
                let frac_idx = sf_i + 1;
                for (fi, s) in active_streams.iter().enumerate() {
                    let k = CellKey { frac_idx, stream_id: s.id, var_id: sfv.id };
                    let val = f_sub_bal[sf_i * n_f + fi];
                    out_sol.cells.insert(
                        k,
                        ReconciliationCell {
                            key: k,
                            measured: None,
                            balanced: Some(val),
                            ci: Some(1.96 * f_sub_sd[sf_i * n_f + fi]),
                            diff: None,
                            rel_diff_pct: None,
                            min_bound: None,
                            max_bound: None,
                            rsd_pct: None,
                            sd_eff: Some(f_sub_sd[sf_i * n_f + fi]),
                            sd_meas: None,
                            sd_sampling: 0.0,
                            recovery_pct: None,
                            pre_status: PreMeasStatus::Missing,
                            bal_status: PostBalStatus::Calculated,
                            is_frozen: false,
                            is_estimated: false,
                        },
                    );
                }
            }
        }

        self.store_solution(dataset_idx, &out_sol);
        Ok(out_sol)
    }

    #[allow(clippy::too_many_arguments)]
    fn assemble_solution(
        &self,
        dataset_idx: usize,
        ds: &MbDataset,
        active_units: &[MbUnit],
        active_streams: &[MbStream],
        e_full: &[Vec<f64>],
        feed_idx: usize,
        frac_idx: usize,
        sf_var: Option<&MbVariable>,
        water_var: Option<&MbVariable>,
        sp_var: Option<&MbVariable>,
        active_assays: &[MbVariable],
        f_bal: &[f64],
        f_post_sd: &[f64],
        w_bal: &[f64],
        g_mat: &[Vec<f64>],
        g_sd: &[Vec<f64>],
        convergence_rounds: Vec<(usize, f64)>,
        converged: bool,
    ) -> MbSolution {
        let n_f = active_streams.len();
        let mut cells = BTreeMap::new();
        let feed_f = f_bal.get(feed_idx).copied().unwrap_or(0.0).max(1e-12);

        let mut num_meas = 0usize;
        let mut num_bal = 0usize;
        let mut num_zeros = 0usize;
        let mut num_negatives = 0usize;
        let mut wssq_total = 0.0;
        let mut sum_sq_diff = 0.0;
        let mut sum_sq_rel = 0.0;
        let mut goodness_vars = Vec::new();

        let mut push_cell = |var: &MbVariable, fi: usize, bal_val: f64, sd_post: f64, rec_pct: Option<f64>| {
            let s = &active_streams[fi];
            let key = CellKey { frac_idx, stream_id: s.id, var_id: var.id };
            let (meas_opt, pre_st) = self.effective_cell_value(ds, key);
            let ov = ds.sd_override.get(&key).copied();
            let (sd_meas, rsd_pct, sd_samp, sd_eff) = evaluate_cell_uncertainty(
                meas_opt, var, s, pre_st, ov, ds.null_sd_estimate, ds.low_grade_coeff,
            );
            let active_meas = if !matches!(pre_st, PreMeasStatus::Missing | PreMeasStatus::Excluded) {
                meas_opt
            } else {
                None
            };
            let diff = active_meas.map(|m| m - bal_val);
            let rel_diff_pct = active_meas.and_then(|m| {
                if m.abs() > 1e-12 {
                    Some(100.0 * (m - bal_val).abs() / m.abs())
                } else {
                    None
                }
            });
            num_bal += 1;
            if bal_val.abs() < 1e-12 {
                num_zeros += 1;
            }
            if bal_val < -1e-9 {
                num_negatives += 1;
            }
            if let (Some(m), Some(d)) = (active_meas, diff) {
                num_meas += 1;
                wssq_total += (d * d) / (sd_eff * sd_eff);
                sum_sq_diff += d * d;
                if let Some(rd) = rel_diff_pct {
                    sum_sq_rel += rd * rd;
                }
                let _ = m;
            }

            let bal_status = if active_meas.is_some() {
                PostBalStatus::Balanced
            } else {
                PostBalStatus::Calculated
            };

            cells.insert(
                key,
                ReconciliationCell {
                    key,
                    measured: meas_opt,
                    balanced: Some(bal_val),
                    ci: Some(1.96 * sd_post),
                    diff,
                    rel_diff_pct,
                    min_bound: ds.min_bounds.get(&key).copied(),
                    max_bound: ds.max_bounds.get(&key).copied(),
                    rsd_pct: Some(rsd_pct),
                    sd_eff: Some(sd_eff),
                    sd_meas: Some(sd_meas),
                    sd_sampling: sd_samp,
                    recovery_pct: rec_pct,
                    pre_status: pre_st,
                    bal_status,
                    is_frozen: ds.frozen_values.contains_key(&key),
                    is_estimated: ds.estimated_cells.contains(&key),
                },
            );
        };

        if let Some(sfv) = sf_var {
            for fi in 0..n_f {
                push_cell(sfv, fi, f_bal[fi], f_post_sd[fi], Some(100.0 * f_bal[fi] / feed_f));
            }
        }
        if ds.balance_type == BalanceType::SolidsWaterAndAssays {
            if let Some(wv) = water_var {
                let feed_w = w_bal.get(feed_idx).copied().unwrap_or(0.0).max(1e-12);
                for fi in 0..n_f {
                    push_cell(wv, fi, w_bal[fi], 0.0, Some(100.0 * w_bal[fi] / feed_w));
                }
            }
            if let Some(sv) = sp_var {
                for fi in 0..n_f {
                    let tot = f_bal[fi] + w_bal[fi];
                    let pc = if tot.abs() > 1e-12 { 100.0 * f_bal[fi] / tot } else { 0.0 };
                    push_cell(sv, fi, pc, 0.0, None);
                }
            }
        }

        for (ci, av) in active_assays.iter().enumerate() {
            let feed_comp = (feed_f * g_mat[feed_idx][ci]).abs().max(1e-12);
            for fi in 0..n_f {
                let g_val = g_mat[fi][ci];
                let comp_mass = f_bal[fi] * g_val;
                let rec = Some(100.0 * comp_mass / feed_comp);
                push_cell(av, fi, g_val, g_sd[fi][ci], rec);
            }
        }

        // Goodness summary per variable (`Fig. 21` top)
        let mut all_vars: Vec<&MbVariable> = Vec::new();
        if let Some(sfv) = sf_var {
            all_vars.push(sfv);
        }
        for av in active_assays {
            all_vars.push(av);
        }
        for v in all_vars {
            let mut v_wssq = 0.0;
            let mut v_diff = 0.0;
            let mut v_rel = 0.0;
            let mut v_sd = 0.0;
            let mut v_rsd = 0.0;
            let mut v_sq = 0.0;
            let mut cnt = 0usize;
            for s in active_streams {
                if let Some(c) = cells.get(&CellKey { frac_idx, stream_id: s.id, var_id: v.id }) {
                    if let (Some(d), Some(sd)) = (c.diff, c.sd_eff) {
                        v_wssq += (d * d) / (sd * sd);
                        v_diff += d.abs();
                        v_rel += c.rel_diff_pct.unwrap_or(0.0);
                        v_sd += sd;
                        v_rsd += c.rsd_pct.unwrap_or(0.0);
                        v_sq += d * d;
                        cnt += 1;
                    }
                }
            }
            if cnt > 0 {
                goodness_vars.push(GoodnessVarRow {
                    var_id: v.id,
                    name: v.name.clone(),
                    unit: v.unit.clone(),
                    wssq: v_wssq,
                    diff_tot: v_diff,
                    rel_diff_pct: v_rel / cnt as f64,
                    avg_sd: v_sd / cnt as f64,
                    avg_rsd_pct: v_rsd / cnt as f64,
                    rmsd: (v_sq / cnt as f64).sqrt(),
                });
            }
        }

        // Goodness summary per stream (`Fig. 21` bottom)
        let mut goodness_streams = Vec::new();
        for s in active_streams {
            let mut s_wssq = 0.0;
            let mut s_diff = 0.0;
            let mut s_rel = 0.0;
            let mut s_sq = 0.0;
            let mut s_sq_rel = 0.0;
            let mut cnt = 0usize;
            for c in cells.values().filter(|c| c.key.stream_id == s.id) {
                if let (Some(d), Some(sd)) = (c.diff, c.sd_eff) {
                    s_wssq += (d * d) / (sd * sd);
                    s_diff += d.abs();
                    let rd = c.rel_diff_pct.unwrap_or(0.0);
                    s_rel += rd;
                    s_sq += d * d;
                    s_sq_rel += rd * rd;
                    cnt += 1;
                }
            }
            goodness_streams.push(GoodnessStreamRow {
                stream_id: s.id,
                name: s.name.clone(),
                sum_wssq: s_wssq,
                diff_sum: s_diff,
                rel_diff_sum: s_rel,
                rel_diff_avg: if cnt > 0 { s_rel / cnt as f64 } else { 0.0 },
                rmsd: if cnt > 0 { (s_sq / cnt as f64).sqrt() } else { 0.0 },
                relative_rmsd: if cnt > 0 { (s_sq_rel / cnt as f64).sqrt() } else { 0.0 },
            });
        }

        // Unit Balance (`Fig. 22`)
        let mut unit_balances = Vec::new();
        for (ui, u) in active_units.iter().enumerate() {
            let in_indices: Vec<usize> = (0..n_f).filter(|&fi| e_full[ui][fi] > 0.5).collect();
            let out_indices: Vec<usize> = (0..n_f).filter(|&fi| e_full[ui][fi] < -0.5).collect();
            let in_names: Vec<String> = in_indices.iter().map(|&i| active_streams[i].name.clone()).collect();
            let out_names: Vec<String> = out_indices.iter().map(|&i| active_streams[i].name.clone()).collect();
            let mut var_labels = Vec::new();
            if let Some(sfv) = sf_var {
                var_labels.push(format!("{} {}", sfv.name, sfv.unit));
            }
            for av in active_assays {
                let comp_u = sf_var.map(|v| v.unit.as_str()).unwrap_or("g");
                var_labels.push(format!("{} {}", av.name, comp_u));
            }

            let eval_stream_comps = |fi: usize| -> Vec<f64> {
                let mut v = vec![f_bal[fi]];
                for ci in 0..active_assays.len() {
                    v.push(f_bal[fi] * g_mat[fi][ci] / 100.0);
                }
                v
            };

            let inputs_mass: Vec<Vec<f64>> = in_indices.iter().map(|&i| eval_stream_comps(i)).collect();
            let outputs_mass: Vec<Vec<f64>> = out_indices.iter().map(|&i| eval_stream_comps(i)).collect();
            let n_c = var_labels.len();
            let mut res_bal = vec![0.0; n_c];
            let mut tol_err = vec![0.0; n_c];
            for c in 0..n_c {
                let sum_in: f64 = inputs_mass.iter().map(|r| r[c]).sum();
                let sum_out: f64 = outputs_mass.iter().map(|r| r[c]).sum();
                res_bal[c] = sum_in - sum_out;
                tol_err[c] = if sum_in.abs() > 1e-12 {
                    (sum_in - sum_out).abs() / sum_in.abs()
                } else {
                    0.0
                };
            }
            unit_balances.push(UnitBalanceRow {
                unit_id: u.id,
                unit_name: u.name.clone(),
                equation_text: format!("{} = {}", in_names.join(" + "), out_names.join(" + ")),
                input_stream_names: in_names,
                output_stream_names: out_names,
                var_labels,
                inputs_mass,
                outputs_mass,
                residual_balance: res_bal,
                tolerance_error: tol_err,
            });
        }

        // Solid SG + Volumetric flows (`Show SG`, `Calculate Volumetric Flows`)
        let mut sg_by_stream = BTreeMap::new();
        let mut vol_by_stream = BTreeMap::new();
        let mut water_by_stream = BTreeMap::new();
        for (fi, s) in active_streams.iter().enumerate() {
            water_by_stream.insert(s.id, w_bal[fi]);
            let mut inv_sg = 0.0;
            let mut min_sum = 0.0;
            for (ci, av) in active_assays.iter().enumerate() {
                if av.type_code == VarTypeCode::M {
                    let g = g_mat[fi][ci].max(0.0);
                    let rho = av.mineral_sg.unwrap_or_else(|| default_mineral_sg(&av.name));
                    inv_sg += g / rho.max(0.1);
                    min_sum += g;
                }
            }
            let solid_sg = if min_sum > 1e-6 && inv_sg > 1e-9 {
                min_sum / inv_sg
            } else {
                3.5
            };
            sg_by_stream.insert(s.id, solid_sg);
            let vol = f_bal[fi] / solid_sg + w_bal[fi] / ds.liquid_sg.max(0.1);
            vol_by_stream.insert(s.id, vol);
        }

        let rmsd = if num_meas > 0 { (sum_sq_diff / num_meas as f64).sqrt() } else { 0.0 };
        let rel_rmsd = if num_meas > 0 { (sum_sq_rel / num_meas as f64).sqrt() } else { 0.0 };

        MbSolution {
            dataset_idx,
            cells,
            convergence_rounds,
            converged,
            num_meas,
            num_bal,
            num_zeros,
            num_negatives,
            wssq: wssq_total,
            rmsd,
            rel_rmsd,
            equations_text: self.build_equations_text(ds.sizing != SizingMode::Unsized),
            goodness_vars,
            goodness_streams,
            unit_balances,
            sg_by_stream,
            vol_flow_by_stream: vol_by_stream,
            solved_water_by_stream: water_by_stream,
            last_message: String::new(),
        }
    }

    fn store_solution(&mut self, dataset_idx: usize, sol: &MbSolution) {
        if self.solutions.len() <= dataset_idx {
            self.solutions.resize(dataset_idx + 1, None);
        }
        self.solutions[dataset_idx] = Some(sol.clone());
    }
}

// ---------------------------------------------------------------------------
// Data Tools, Mineral Conversions, Rounds, Import/Export & Sim Sync
// ---------------------------------------------------------------------------

pub fn default_mineral_sg(name: &str) -> f64 {
    let lc = name.to_lowercase();
    if lc.contains("ccp") || lc.contains("chalcopyrite") {
        4.19
    } else if lc.contains("py") && !lc.contains("pyrox") {
        5.01
    } else if lc.contains("sp") || lc.contains("sphalerite") {
        4.05
    } else if lc.contains("qtz") || lc.contains("quartz") {
        2.65
    } else if lc.contains("gn") || lc.contains("galena") {
        7.50
    } else if lc.contains("mag") || lc.contains("magnetite") {
        5.15
    } else if lc.contains("hem") || lc.contains("hematite") {
        5.26
    } else {
        3.20
    }
}

/// Resolve stoichiometric composition weights of a mineral by code or name in `MINERALS_RAW` or formula.
pub fn lookup_mineral_formula(query: &str) -> Option<(String, String)> {
    let q = query.trim().to_lowercase();
    for m in MINERALS_RAW {
        if m.code.to_lowercase() == q || m.name.to_lowercase() == q {
            if !m.formula.is_empty() {
                return Some((m.name.to_string(), m.formula.to_string()));
            }
        }
    }
    None
}

pub fn search_minerals_catalog(query: &str, limit: usize) -> Vec<&'static MineralRawRow> {
    let q = query.trim().to_lowercase();
    MINERALS_RAW
        .iter()
        .filter(|m| {
            !m.formula.is_empty()
                && (q.is_empty()
                    || m.name.to_lowercase().contains(&q)
                    || m.code.to_lowercase().contains(&q)
                    || m.formula.to_lowercase().contains(&q))
        })
        .take(limit)
        .collect()
}

impl MassBalanceWorkbook {
    /// Convert elemental assays (`A`) to mineral grades (`M`) per stream using NNLS (`species_converter.rs`).
    pub fn convert_elements_to_minerals(&mut self, use_balanced_values: bool) -> Result<usize, String> {
        let ds_idx = self.active_dataset;
        let elem_vars: Vec<MbVariable> = self
            .variables
            .iter()
            .filter(|v| v.selected && v.type_code == VarTypeCode::A)
            .cloned()
            .collect();
        let min_vars: Vec<MbVariable> = self
            .variables
            .iter()
            .filter(|v| v.selected && v.type_code == VarTypeCode::M)
            .cloned()
            .collect();
        if elem_vars.is_empty() || min_vars.is_empty() {
            return Err("Select both elemental assay (A) and mineral assay (M) variables".into());
        }

        let mut outputs = Vec::new();
        for mv in &min_vars {
            let formula = mv
                .mineral_formula
                .clone()
                .or_else(|| lookup_mineral_formula(&mv.name).map(|(_, f)| f))
                .unwrap_or_else(|| mv.name.clone());
            // Verify parseable formula
            if parse_term(&formula).is_err() {
                return Err(format!("Cannot parse mineral formula '{}' for '{}'", formula, mv.name));
            }
            outputs.push(ConverterOutputSpec {
                formula,
                target_wt_pct: None,
                weight_coeff: 1.0,
            });
        }

        let sol_opt = if use_balanced_values {
            self.solutions.get(ds_idx).and_then(|s| s.clone())
        } else {
            None
        };

        let mut updated = 0usize;
        let streams_snapshot = self.streams.clone();
        let ds = self
            .datasets
            .get_mut(ds_idx)
            .ok_or_else(|| "Active dataset not found".to_string())?;

        for s in &streams_snapshot {
            if !s.selected || s.stream_type == StreamType::LiquidWater {
                continue;
            }
            let mut inputs = Vec::new();
            for ev in &elem_vars {
                let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: ev.id };
                let val_opt = sol_opt
                    .as_ref()
                    .and_then(|sol| sol.cells.get(&k).and_then(|c| c.balanced))
                    .or_else(|| ds.values.get(&k).copied());
                if let Some(v) = val_opt {
                    inputs.push(ConverterInputEntry {
                        formula: ev.name.clone(),
                        wt_pct: v,
                    });
                }
            }
            if inputs.is_empty() {
                continue;
            }
            struct MineralEmptyRepo;
            impl SpeciesRepository for MineralEmptyRepo {
                fn get_species(&self, _formula: &str) -> Option<ChemicalSpecies> {
                    None
                }
                fn get_reference_element(&self, _element_symbol: &str) -> Option<ChemicalSpecies> {
                    None
                }
            }
            let req = SpeciesConverterRequest {
                inputs,
                outputs: outputs.clone(),
                method: ConversionMethod::Method1,
                exact_o: false,
                exact_h: false,
                set_norm_limit: false,
                norm_limit: 100.0,
                small_meas_limit: 0.0,
                element_balance_limit_pct: 5.0,
                stability_type: SpeciesTypeFilter::None,
                stability_weighting_coef: 1.0,
            };
            if let Ok(res) = solve_species_conversion(&MineralEmptyRepo, &req) {
                for (mi, mv) in min_vars.iter().enumerate() {
                    if let Some(row) = res.output_rows.get(mi) {
                        let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: mv.id };
                        ds.values.insert(k, row.calculated_wt_pct);
                        updated += 1;
                    }
                }
            }
        }
        Ok(updated)
    }

    /// Back-calculate element grades from mineral values (and append missing elements if requested).
    pub fn calculate_all_mineral_elements(&mut self) -> usize {
        let min_vars: Vec<MbVariable> = self
            .variables
            .iter()
            .filter(|v| v.selected && v.type_code == VarTypeCode::M)
            .cloned()
            .collect();
        if min_vars.is_empty() {
            return 0;
        }
        let mut parsed_mins = Vec::new();
        let mut elem_set = BTreeSet::new();
        for mv in &min_vars {
            let formula = mv
                .mineral_formula
                .clone()
                .or_else(|| lookup_mineral_formula(&mv.name).map(|(_, f)| f))
                .unwrap_or_else(|| mv.name.clone());
            if let Ok(pt) = parse_term(&formula) {
                // Convert atom counts to element wt%
                let mw: f64 = pt.elements.iter().map(|(e, &n)| atomic_wt(e) * n).sum();
                if mw > 1e-6 {
                    let wt_map: BTreeMap<String, f64> = pt
                        .elements
                        .iter()
                        .map(|(e, &n)| {
                            elem_set.insert(e.clone());
                            (e.clone(), 100.0 * atomic_wt(e) * n / mw)
                        })
                        .collect();
                    parsed_mins.push((mv.id, wt_map));
                }
            }
        }

        // Ensure element variables exist for each discovered element
        let mut elem_var_ids = Vec::new();
        for elem in elem_set {
            if let Some(ev) = self
                .variables
                .iter()
                .find(|v| v.type_code == VarTypeCode::A && v.name.eq_ignore_ascii_case(&elem))
            {
                elem_var_ids.push((elem, ev.id));
            } else {
                let id = self.next_id;
                self.next_id += 1;
                self.variables.push(MbVariable {
                    id,
                    name: elem.clone(),
                    unit: "%".into(),
                    type_code: VarTypeCode::A,
                    selected: true,
                    method: MeasurementMethod::FixedRelative,
                    param_a: 5.0,
                    param_b: 0.0,
                    param_c: 0.0,
                    low_grade_weighted: false,
                    mineral_formula: None,
                    mineral_sg: None,
                });
                elem_var_ids.push((elem, id));
            }
        }

        let ds_idx = self.active_dataset;
        let sol_opt = self.solutions.get(ds_idx).and_then(|s| s.clone());
        let streams = self.streams.clone();
        let mut count = 0usize;
        if let Some(ds) = self.datasets.get_mut(ds_idx) {
            for s in &streams {
                if !s.selected {
                    continue;
                }
                for (elem, eid) in &elem_var_ids {
                    let mut tot = 0.0;
                    let mut any = false;
                    for (mid, wt_map) in &parsed_mins {
                        let mk = CellKey { frac_idx: 0, stream_id: s.id, var_id: *mid };
                        let mv_opt = sol_opt
                            .as_ref()
                            .and_then(|sol| sol.cells.get(&mk).and_then(|c| c.balanced))
                            .or_else(|| ds.values.get(&mk).copied());
                        if let Some(mv) = mv_opt {
                            tot += (mv / 100.0) * wt_map.get(elem).copied().unwrap_or(0.0);
                            any = true;
                        }
                    }
                    if any {
                        ds.values.insert(
                            CellKey { frac_idx: 0, stream_id: s.id, var_id: *eid },
                            tot,
                        );
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// `Sol-% to Water`: derives Water (`SC`) values from paired (`SF`, `SP`).
    pub fn convert_sol_pct_to_water(&mut self) -> usize {
        let sf_id = self.variables.iter().find(|v| v.type_code == VarTypeCode::SF).map(|v| v.id);
        let sp_id = self.variables.iter().find(|v| v.type_code == VarTypeCode::SP).map(|v| v.id);
        let Some((sf_id, sp_id)) = sf_id.zip(sp_id) else {
            return 0;
        };
        let water_id = if let Some(w) = self.variables.iter().find(|v| v.type_code == VarTypeCode::SC) {
            w.id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.variables.push(MbVariable {
                id,
                name: "Water".into(),
                unit: "t/h".into(),
                type_code: VarTypeCode::SC,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            });
            id
        };
        let ds_idx = self.active_dataset;
        let streams = self.streams.clone();
        let mut count = 0;
        if let Some(ds) = self.datasets.get_mut(ds_idx) {
            for s in &streams {
                let f = ds.values.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: sf_id }).copied();
                let pc = ds.values.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: sp_id }).copied();
                if let (Some(f_v), Some(pc_v)) = (f, pc) {
                    if pc_v > 1e-4 && pc_v < 100.0 {
                        let w_v = f_v * (100.0 - pc_v) / pc_v;
                        ds.values.insert(
                            CellKey { frac_idx: 0, stream_id: s.id, var_id: water_id },
                            w_v,
                        );
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Propagate/interpolate missing measurements (`Estimate Missing Values`, blue font).
    pub fn estimate_missing_values(&mut self) -> usize {
        let ds_idx = self.active_dataset;
        let units = self.units.clone();
        let streams = self.streams.clone();
        let vars = self.variables.clone();
        let Some(ds) = self.datasets.get_mut(ds_idx) else {
            return 0;
        };
        let mut count = 0;
        // Estimate SF via simple additive propagation across units where possible (passes = 6)
        if let Some(sfv) = vars.iter().find(|v| v.selected && v.type_code == VarTypeCode::SF) {
            for _ in 0..6 {
                for u in units.iter().filter(|u| u.selected) {
                    let ins: Vec<usize> = streams.iter().filter(|s| s.selected && s.dest_unit == Some(u.id)).map(|s| s.id).collect();
                    let outs: Vec<usize> = streams.iter().filter(|s| s.selected && s.source_unit == Some(u.id)).map(|s| s.id).collect();
                    let in_missing: Vec<usize> = ins.iter().copied().filter(|&sid| !ds.values.contains_key(&CellKey { frac_idx: 0, stream_id: sid, var_id: sfv.id })).collect();
                    let out_missing: Vec<usize> = outs.iter().copied().filter(|&sid| !ds.values.contains_key(&CellKey { frac_idx: 0, stream_id: sid, var_id: sfv.id })).collect();
                    let sum_in: f64 = ins.iter().filter_map(|&sid| ds.values.get(&CellKey { frac_idx: 0, stream_id: sid, var_id: sfv.id })).sum();
                    let sum_out: f64 = outs.iter().filter_map(|&sid| ds.values.get(&CellKey { frac_idx: 0, stream_id: sid, var_id: sfv.id })).sum();

                    if in_missing.len() == 1 && out_missing.is_empty() {
                        let k = CellKey { frac_idx: 0, stream_id: in_missing[0], var_id: sfv.id };
                        ds.values.insert(k, (sum_out - sum_in).max(0.0));
                        ds.estimated_cells.insert(k);
                        count += 1;
                    } else if out_missing.len() == 1 && in_missing.is_empty() {
                        let k = CellKey { frac_idx: 0, stream_id: out_missing[0], var_id: sfv.id };
                        ds.values.insert(k, (sum_in - sum_out).max(0.0));
                        ds.estimated_cells.insert(k);
                        count += 1;
                    }
                }
            }
        }
        // Estimate remaining missing assays by flow-weighted unit balance or average
        for v in vars.iter().filter(|v| v.selected && v.type_code.is_assay()) {
            let avg: Option<f64> = {
                let vals: Vec<f64> = streams
                    .iter()
                    .filter(|s| s.selected && s.stream_type != StreamType::LiquidWater)
                    .filter_map(|s| ds.values.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: v.id }).copied())
                    .collect();
                if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) }
            };
            if let Some(a) = avg {
                for s in streams.iter().filter(|s| s.selected && s.stream_type != StreamType::LiquidWater) {
                    let k = CellKey { frac_idx: 0, stream_id: s.id, var_id: v.id };
                    if !ds.values.contains_key(&k) {
                        ds.values.insert(k, a);
                        ds.estimated_cells.insert(k);
                        count += 1;
                    }
                }
            }
        }
        count
    }

    pub fn clear_estimated_values(&mut self) {
        if let Some(ds) = self.datasets.get_mut(self.active_dataset) {
            let keys: Vec<CellKey> = ds.estimated_cells.iter().copied().collect();
            for k in keys {
                ds.values.remove(&k);
            }
            ds.estimated_cells.clear();
        }
    }

    /// `Calculate Master Data` from sampling rounds (mean across all rounds per cell).
    pub fn calculate_master_data_from_rounds(&mut self) -> usize {
        let Some(ds) = self.datasets.get_mut(self.active_dataset) else {
            return 0;
        };
        if ds.rounds.is_empty() {
            return 0;
        }
        let mut acc: BTreeMap<CellKey, Vec<f64>> = BTreeMap::new();
        for r in &ds.rounds {
            for (&k, &v) in r {
                acc.entry(k).or_default().push(v);
            }
        }
        let mut updated = 0;
        for (k, vals) in acc {
            if !vals.is_empty() {
                ds.values.insert(k, vals.iter().sum::<f64>() / vals.len() as f64);
                ds.estimated_cells.remove(&k);
                updated += 1;
            }
        }
        updated
    }

    /// `Calculate SD From Data`: sample standard deviation across sampling rounds (`n >= 2`).
    pub fn calculate_sd_from_rounds(&mut self) -> usize {
        let Some(ds) = self.datasets.get_mut(self.active_dataset) else {
            return 0;
        };
        if ds.rounds.len() < 2 {
            return 0;
        }
        let mut acc: BTreeMap<CellKey, Vec<f64>> = BTreeMap::new();
        for r in &ds.rounds {
            for (&k, &v) in r {
                acc.entry(k).or_default().push(v);
            }
        }
        let mut updated = 0;
        for (k, vals) in acc {
            if vals.len() >= 2 {
                let n = vals.len() as f64;
                let mean = vals.iter().sum::<f64>() / n;
                let var = vals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
                ds.sd_override.insert(k, var.sqrt().max(ds.null_sd_estimate));
                updated += 1;
            }
        }
        updated
    }

    /// Freeze balanced results (`Freeze Balanced`).
    pub fn freeze_balanced(&mut self) -> usize {
        let ds_idx = self.active_dataset;
        let Some(sol) = self.solutions.get(ds_idx).and_then(|s| s.clone()) else {
            return 0;
        };
        let Some(ds) = self.datasets.get_mut(ds_idx) else {
            return 0;
        };
        let mut cnt = 0;
        for (k, cell) in sol.cells {
            if let Some(b) = cell.balanced {
                ds.frozen_values.insert(k, b);
                ds.frozen_bal_status.insert(k, cell.bal_status);
                cnt += 1;
            }
        }
        cnt
    }

    pub fn clear_and_unfreeze_balanced(&mut self) {
        let ds_idx = self.active_dataset;
        if let Some(ds) = self.datasets.get_mut(ds_idx) {
            ds.frozen_values.clear();
            ds.frozen_bal_status.clear();
        }
        if let Some(slot) = self.solutions.get_mut(ds_idx) {
            *slot = None;
        }
    }

    pub fn reset_measurement_statuses(&mut self) {
        if let Some(ds) = self.datasets.get_mut(self.active_dataset) {
            ds.meas_status_override.clear();
        }
    }

    /// Paste tab/comma-separated experimental data (autodetects horizontal vs vertical headers).
    pub fn paste_experimental_tsv(&mut self, text: &str, frac_idx: usize) -> usize {
        let rows: Vec<Vec<String>> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let sep = if l.contains('\t') { '\t' } else { ',' };
                l.split(sep).map(|c| c.trim().to_string()).collect()
            })
            .collect();
        if rows.len() < 2 || rows[0].len() < 2 {
            return 0;
        }

        let col_streams = rows[0][1..]
            .iter()
            .filter(|h| self.streams.iter().any(|s| s.name.eq_ignore_ascii_case(h)))
            .count();
        let row_streams = rows[1..]
            .iter()
            .filter(|r| !r.is_empty() && self.streams.iter().any(|s| s.name.eq_ignore_ascii_case(&r[0])))
            .count();

        let match_var_id = |hdr: &str| -> Option<usize> {
            let h = hdr.to_lowercase();
            self.variables
                .iter()
                .find(|v| {
                    v.name.to_lowercase() == h
                        || format!("{} {}", v.name, v.unit).to_lowercase() == h
                        || format!("{}%", v.name).to_lowercase() == h
                })
                .map(|v| v.id)
        };

        let mut inserted = 0usize;
        let ds_idx = self.active_dataset;
        let streams = self.streams.clone();
        let Some(ds) = self.datasets.get_mut(ds_idx) else {
            return 0;
        };

        if row_streams >= col_streams {
            // Horizontal: rows = streams, cols = variables
            let col_vars: Vec<Option<usize>> = rows[0].iter().map(|h| match_var_id(h)).collect();
            for r in &rows[1..] {
                let Some(s) = streams.iter().find(|s| s.name.eq_ignore_ascii_case(&r[0])) else {
                    continue;
                };
                for (ci, cell_str) in r.iter().enumerate() {
                    if ci == 0 {
                        continue;
                    }
                    if let (Some(vid), Ok(val)) = (col_vars.get(ci).copied().flatten(), cell_str.replace(',', ".").parse::<f64>()) {
                        let k = CellKey { frac_idx, stream_id: s.id, var_id: vid };
                        ds.values.insert(k, val);
                        ds.estimated_cells.remove(&k);
                        inserted += 1;
                    }
                }
            }
        } else {
            // Vertical: cols = streams, rows = variables
            let col_streams_id: Vec<Option<usize>> = rows[0]
                .iter()
                .map(|h| streams.iter().find(|s| s.name.eq_ignore_ascii_case(h)).map(|s| s.id))
                .collect();
            for r in &rows[1..] {
                let Some(vid) = match_var_id(&r[0]) else {
                    continue;
                };
                for (ci, cell_str) in r.iter().enumerate() {
                    if ci == 0 {
                        continue;
                    }
                    if let (Some(sid), Ok(val)) = (col_streams_id.get(ci).copied().flatten(), cell_str.replace(',', ".").parse::<f64>()) {
                        let k = CellKey { frac_idx, stream_id: sid, var_id: vid };
                        ds.values.insert(k, val);
                        ds.estimated_cells.remove(&k);
                        inserted += 1;
                    }
                }
            }
        }
        inserted
    }

    /// Import legacy HSC7 `Flowsheet` incidence CSV/TSV (`A1=Units`, `+1/-1/0` `Fig. 24`) + vertical/horizontal analyses (`Fig. 25–26`).
    pub fn import_hsc7_text(
        &mut self,
        flowsheet_csv: Option<&str>,
        data_csv: &str,
        horizontal: bool,
    ) -> Result<usize, String> {
        // Parse optional Flowsheet incidence sheet (`Units` x `Streams`, +1 = input to unit, -1 = output from unit)
        let mut parsed_units: Vec<(String, usize)> = Vec::new();
        let mut parsed_streams: BTreeMap<String, (Option<usize>, Option<usize>)> = BTreeMap::new();
        let mut id_counter = 1usize;

        if let Some(fs) = flowsheet_csv {
            let mat: Vec<Vec<String>> = fs
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    let sep = if l.contains('\t') { '\t' } else { ',' };
                    l.split(sep).map(|c| c.trim().to_string()).collect()
                })
                .collect();
            if mat.len() >= 2 && mat[0].len() >= 2 {
                let s_names = &mat[0][1..];
                for s in s_names {
                    if !s.is_empty() {
                        parsed_streams.entry(s.clone()).or_insert((None, None));
                    }
                }
                for row in &mat[1..] {
                    if row.is_empty() || row[0].is_empty() {
                        continue;
                    }
                    let u_name = row[0].clone();
                    let u_id = id_counter;
                    id_counter += 1;
                    parsed_units.push((u_name, u_id));
                    for (ci, s_name) in s_names.iter().enumerate() {
                        if let Some(cell) = row.get(ci + 1) {
                            let val = cell.parse::<i32>().unwrap_or(0);
                            if let Some(entry) = parsed_streams.get_mut(s_name) {
                                if val > 0 {
                                    entry.1 = Some(u_id); // dest_unit (+1 enters unit)
                                } else if val < 0 {
                                    entry.0 = Some(u_id); // source_unit (-1 exits unit)
                                }
                            }
                        }
                    }
                }
            }
        }

        // Parse data table
        let rows: Vec<Vec<String>> = data_csv
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let sep = if l.contains('\t') { '\t' } else { ',' };
                l.split(sep).map(|c| c.trim().to_string()).collect()
            })
            .collect();
        if rows.len() < 2 {
            return Err("HSC7 import requires at least a header row and one data row".into());
        }

        let mut units_by_name: BTreeMap<String, usize> =
            parsed_units.iter().map(|(n, id)| (n.clone(), *id)).collect();
        let mut get_or_add_unit = |name: &str, units: &mut Vec<(String, usize)>, cnt: &mut usize| -> Option<usize> {
            let trimmed = name.trim();
            if trimmed.is_empty() || trimmed == "?" {
                return None;
            }
            if let Some(&id) = units_by_name.get(trimmed) {
                return Some(id);
            }
            let id = *cnt;
            *cnt += 1;
            units_by_name.insert(trimmed.to_string(), id);
            units.push((trimmed.to_string(), id));
            Some(id)
        };

        let mut new_vars: Vec<MbVariable> = Vec::new();
        let mut new_fracs: Vec<String> = vec!["Bulk".into()];
        let mut new_vals: BTreeMap<(String, usize, usize), f64> = BTreeMap::new(); // (stream_name, frac_idx, var_id)

        if !horizontal {
            // Vertical HSC7 (`Fig. 25`): Row 0 = `Stream-> | S1 | S2 ...`, optional Row 1 `Source`, Row 2 `Destination`, then variables/sizes
            let s_headers = &rows[0][1..];
            let mut src_row: Option<&Vec<String>> = None;
            let mut dst_row: Option<&Vec<String>> = None;
            let mut data_start = 1;
            for (ri, r) in rows.iter().enumerate().skip(1) {
                let tag = r.first().map(|s| s.to_lowercase()).unwrap_or_default();
                if tag == "source" {
                    src_row = Some(r);
                    data_start = ri + 1;
                } else if tag == "destination" {
                    dst_row = Some(r);
                    data_start = ri + 1;
                } else if tag.starts_with("mass%") {
                    data_start = ri + 1;
                } else {
                    break;
                }
            }
            for (ci, s_name) in s_headers.iter().enumerate() {
                if s_name.is_empty() {
                    continue;
                }
                let src = src_row.and_then(|r| r.get(ci + 1)).and_then(|n| get_or_add_unit(n, &mut parsed_units, &mut id_counter));
                let dst = dst_row.and_then(|r| r.get(ci + 1)).and_then(|n| get_or_add_unit(n, &mut parsed_units, &mut id_counter));
                let entry = parsed_streams.entry(s_name.clone()).or_insert((None, None));
                if src.is_some() {
                    entry.0 = src;
                }
                if dst.is_some() {
                    entry.1 = dst;
                }
            }

            let mut sa_var_id: Option<usize> = None;
            for r in &rows[data_start..] {
                let raw_name = r[0].trim();
                if raw_name.is_empty() {
                    continue;
                }
                let lc = raw_name.to_lowercase();
                // Detect sieve size row e.g. "0-53um" or "+75um"
                if lc.contains("um") && (lc.contains('-') || lc.starts_with('+') || lc.starts_with('-')) {
                    let sav_id = match sa_var_id {
                        Some(id) => id,
                        None => {
                            let id = id_counter;
                            id_counter += 1;
                            new_vars.push(MbVariable {
                                id,
                                name: "Weight Retained".into(),
                                unit: "%".into(),
                                type_code: VarTypeCode::SA,
                                selected: true,
                                method: MeasurementMethod::SizeFractionWhiten,
                                param_a: 0.0,
                                param_b: 0.0,
                                param_c: 0.0,
                                low_grade_weighted: false,
                                mineral_formula: None,
                                mineral_sg: None,
                            });
                            sa_var_id = Some(id);
                            id
                        }
                    };
                    new_fracs.push(raw_name.to_string());
                    let frac_idx = new_fracs.len() - 1;
                    for (ci, s_name) in s_headers.iter().enumerate() {
                        if let Some(Ok(val)) = r.get(ci + 1).map(|c| c.parse::<f64>()) {
                            new_vals.insert((s_name.clone(), frac_idx, sav_id), val);
                        }
                    }
                    continue;
                }

                let (tc, unit_str) = if lc.contains("solids flow") || lc == "mass" || lc.starts_with("total solids") {
                    (VarTypeCode::SF, if lc.contains("t/h") { "t/h" } else { "g" })
                } else if lc.contains("water") {
                    (VarTypeCode::SC, "t/h")
                } else if lc.contains("%solids") || lc.contains("% solids") {
                    (VarTypeCode::SP, "%")
                } else {
                    (VarTypeCode::A, "%")
                };
                let vid = id_counter;
                id_counter += 1;
                let clean_name = raw_name
                    .split_whitespace()
                    .next()
                    .unwrap_or(raw_name)
                    .to_string();
                new_vars.push(MbVariable {
                    id: vid,
                    name: clean_name,
                    unit: unit_str.into(),
                    type_code: tc,
                    selected: true,
                    method: MeasurementMethod::FixedRelative,
                    param_a: 5.0,
                    param_b: 0.0,
                    param_c: 0.0,
                    low_grade_weighted: false,
                    mineral_formula: None,
                    mineral_sg: None,
                });
                for (ci, s_name) in s_headers.iter().enumerate() {
                    if let Some(Ok(val)) = r.get(ci + 1).map(|c| c.parse::<f64>()) {
                        new_vals.insert((s_name.clone(), 0, vid), val);
                    }
                }
            }
        } else {
            // Horizontal (`Fig. 26`): `Stream | Source | Destination | Solids Recovery% | Total Solids t/h | FractionNo | Fraction name | Fraction m% | Note | ...assays...`
            let headers = &rows[0];
            let find_col = |needle: &str| headers.iter().position(|h| h.to_lowercase().contains(needle));
            let col_src = find_col("source");
            let col_dst = find_col("destination");
            let col_ts = find_col("total solids");
            let col_frac_name = find_col("fraction name");
            let col_frac_m = find_col("fraction m%");

            let sf_id = if col_ts.is_some() {
                let id = id_counter;
                id_counter += 1;
                new_vars.push(MbVariable {
                    id,
                    name: "Total Solids".into(),
                    unit: "t/h".into(),
                    type_code: VarTypeCode::SF,
                    selected: true,
                    method: MeasurementMethod::FixedRelative,
                    param_a: 5.0,
                    param_b: 0.0,
                    param_c: 0.0,
                    low_grade_weighted: false,
                    mineral_formula: None,
                    mineral_sg: None,
                });
                Some(id)
            } else {
                None
            };

            let mut assay_cols = Vec::new();
            for (ci, h) in headers.iter().enumerate() {
                let lc = h.to_lowercase();
                if Some(ci) == Some(0)
                    || Some(ci) == col_src
                    || Some(ci) == col_dst
                    || Some(ci) == col_ts
                    || Some(ci) == col_frac_name
                    || Some(ci) == col_frac_m
                    || lc.contains("recovery")
                    || lc.contains("fractionno")
                    || lc == "note"
                    || lc == "-"
                    || h.trim().is_empty()
                {
                    continue;
                }
                let vid = id_counter;
                id_counter += 1;
                let cname = h.split_whitespace().next().unwrap_or(h).to_string();
                new_vars.push(MbVariable {
                    id: vid,
                    name: cname,
                    unit: "%".into(),
                    type_code: VarTypeCode::A,
                    selected: true,
                    method: MeasurementMethod::FixedRelative,
                    param_a: 5.0,
                    param_b: 0.0,
                    param_c: 0.0,
                    low_grade_weighted: false,
                    mineral_formula: None,
                    mineral_sg: None,
                });
                assay_cols.push((ci, vid));
            }

            let mut sa_vid = None;
            for r in &rows[1..] {
                let s_name = r[0].trim().to_string();
                if s_name.is_empty() {
                    continue;
                }
                let src = col_src.and_then(|c| r.get(c)).and_then(|n| get_or_add_unit(n, &mut parsed_units, &mut id_counter));
                let dst = col_dst.and_then(|c| r.get(c)).and_then(|n| get_or_add_unit(n, &mut parsed_units, &mut id_counter));
                let entry = parsed_streams.entry(s_name.clone()).or_insert((None, None));
                if src.is_some() {
                    entry.0 = src;
                }
                if dst.is_some() {
                    entry.1 = dst;
                }
                let frac_name = col_frac_name.and_then(|c| r.get(c)).map(|s| s.trim()).unwrap_or("Bulk");
                let frac_idx = if frac_name.eq_ignore_ascii_case("Bulk") || frac_name.is_empty() {
                    0
                } else if let Some(pos) = new_fracs.iter().position(|f| f.eq_ignore_ascii_case(frac_name)) {
                    pos
                } else {
                    new_fracs.push(frac_name.to_string());
                    new_fracs.len() - 1
                };
                if let (Some(ts_c), Some(sfv)) = (col_ts, sf_id) {
                    if let Some(Ok(v)) = r.get(ts_c).map(|s| s.parse::<f64>()) {
                        new_vals.insert((s_name.clone(), frac_idx, sfv), v);
                    }
                }
                if frac_idx > 0 {
                    if let Some(fm_c) = col_frac_m {
                        if let Some(Ok(v)) = r.get(fm_c).map(|s| s.parse::<f64>()) {
                            let id = *sa_vid.get_or_insert_with(|| {
                                let id = id_counter;
                                id_counter += 1;
                                new_vars.push(MbVariable {
                                    id,
                                    name: "Weight Retained".into(),
                                    unit: "%".into(),
                                    type_code: VarTypeCode::SA,
                                    selected: true,
                                    method: MeasurementMethod::SizeFractionWhiten,
                                    param_a: 0.0,
                                    param_b: 0.0,
                                    param_c: 0.0,
                                    low_grade_weighted: false,
                                    mineral_formula: None,
                                    mineral_sg: None,
                                });
                                id
                            });
                            new_vals.insert((s_name.clone(), frac_idx, id), v);
                        }
                    }
                }
                for &(ci, vid) in &assay_cols {
                    if let Some(Ok(v)) = r.get(ci).map(|s| s.parse::<f64>()) {
                        new_vals.insert((s_name.clone(), frac_idx, vid), v);
                    }
                }
            }
        }

        // Populate workbook
        self.units = parsed_units
            .into_iter()
            .map(|(name, id)| {
                let lc = name.to_lowercase();
                let is_mill = lc.contains("sag") || lc.contains("mill") || lc.contains("crush");
                MbUnit { id, name, selected: true, size_transforming: is_mill }
            })
            .collect();

        let mut stream_id_by_name = BTreeMap::new();
        self.streams.clear();
        for (s_name, (src, dst)) in parsed_streams {
            let sid = id_counter;
            id_counter += 1;
            stream_id_by_name.insert(s_name.clone(), sid);
            let is_water = s_name.to_lowercase().contains("water");
            self.streams.push(MbStream {
                id: sid,
                name: s_name,
                selected: true,
                stream_type: if is_water { StreamType::LiquidWater } else { StreamType::SolidsSlurry },
                flows_enabled: true,
                assays_enabled: !is_water,
                sampling_device: "Default".into(),
                sampling_sd: 0.0,
                sample_quality: SampleQuality::Good,
                source_unit: src,
                dest_unit: dst,
            });
        }

        self.variables = new_vars;
        self.size_fractions = new_fracs;
        self.next_id = id_counter + 1;

        let mut ds = MbDataset::default();
        ds.name = "HSC7 Imported".into();
        let imported_cnt = new_vals.len();
        for ((s_name, fi, vid), val) in new_vals {
            if let Some(&sid) = stream_id_by_name.get(&s_name) {
                ds.values.insert(CellKey { frac_idx: fi, stream_id: sid, var_id: vid }, val);
            }
        }
        if self.size_fractions.len() > 1 {
            ds.sizing = SizingMode::Sized;
        }
        self.datasets = vec![ds];
        self.active_dataset = 0;
        self.solutions = vec![None];
        self.stream_equations.clear();
        Ok(imported_cnt)
    }

    /// Synchronize units, streams & initial flow/assay seed from an HSC Sim `SimProject`.
    pub fn from_sim_project(proj: &SimProject) -> Self {
        let mut units: Vec<MbUnit> = proj
            .units
            .iter()
            .map(|u| {
                let lc = u.name_id.to_lowercase();
                MbUnit {
                    id: u.id,
                    name: u.name_id.clone(),
                    selected: true,
                    size_transforming: lc.contains("mill") || lc.contains("sag") || lc.contains("crush"),
                }
            })
            .collect();
        if units.is_empty() {
            return Self::default();
        }
        let max_uid = units.iter().map(|u| u.id).max().unwrap_or(0);
        let max_sid = proj.streams.iter().map(|s| s.id).max().unwrap_or(0);

        let streams: Vec<MbStream> = proj
            .streams
            .iter()
            .map(|s| MbStream {
                id: s.id,
                name: s.name_id.clone(),
                selected: true,
                stream_type: if s.solids_flow_t_h.abs() < 1e-9 && s.liquid_flow_t_h > 1e-6 {
                    StreamType::LiquidWater
                } else {
                    StreamType::SolidsSlurry
                },
                flows_enabled: s.source_unit.is_some() && s.dest_unit.is_none() || s.mass_flow_t_h > 0.0,
                assays_enabled: true,
                sampling_device: "Default".into(),
                sampling_sd: 0.0,
                sample_quality: SampleQuality::Good,
                source_unit: s.source_unit,
                dest_unit: s.dest_unit,
            })
            .collect();

        let base_id = max_uid.max(max_sid) + 100;
        let variables = vec![
            MbVariable {
                id: base_id,
                name: "Solids Flow".into(),
                unit: "t/h".into(),
                type_code: VarTypeCode::SF,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            },
            MbVariable {
                id: base_id + 1,
                name: "Water".into(),
                unit: "t/h".into(),
                type_code: VarTypeCode::SC,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            },
            MbVariable {
                id: base_id + 2,
                name: "Cu".into(),
                unit: "%".into(),
                type_code: VarTypeCode::A,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            },
            MbVariable {
                id: base_id + 3,
                name: "Fe".into(),
                unit: "%".into(),
                type_code: VarTypeCode::A,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            },
            MbVariable {
                id: base_id + 4,
                name: "S".into(),
                unit: "%".into(),
                type_code: VarTypeCode::A,
                selected: true,
                method: MeasurementMethod::FixedRelative,
                param_a: 5.0,
                param_b: 0.0,
                param_c: 0.0,
                low_grade_weighted: false,
                mineral_formula: None,
                mineral_sg: None,
            },
        ];

        let mut ds = MbDataset::default();
        ds.name = "Sim Survey 1".into();
        for s in &proj.streams {
            if s.solids_flow_t_h > 0.0 || s.mass_flow_t_h > 0.0 {
                let sf = if s.solids_flow_t_h > 0.0 { s.solids_flow_t_h } else { s.mass_flow_t_h };
                ds.values.insert(CellKey { frac_idx: 0, stream_id: s.id, var_id: base_id }, sf);
            }
            if s.liquid_flow_t_h > 0.0 {
                ds.values.insert(CellKey { frac_idx: 0, stream_id: s.id, var_id: base_id + 1 }, s.liquid_flow_t_h);
            }
            if s.cu_wt_pct > 0.0 {
                ds.values.insert(CellKey { frac_idx: 0, stream_id: s.id, var_id: base_id + 2 }, s.cu_wt_pct);
            }
            if s.fe_wt_pct > 0.0 {
                ds.values.insert(CellKey { frac_idx: 0, stream_id: s.id, var_id: base_id + 3 }, s.fe_wt_pct);
            }
            if s.s_wt_pct > 0.0 {
                ds.values.insert(CellKey { frac_idx: 0, stream_id: s.id, var_id: base_id + 4 }, s.s_wt_pct);
            }
        }
        units.sort_by_key(|u| u.id);

        Self {
            units,
            streams,
            variables,
            size_fractions: vec!["Bulk".into()],
            datasets: vec![ds],
            active_dataset: 0,
            stream_equations: Vec::new(),
            balancing_of_minerals: false,
            next_id: base_id + 10,
            solutions: vec![None],
            chart_panels: default_chart_panels(),
        }
    }

    /// Write balanced flows, grades & recoveries back into HSC Sim `SimProject` (`Save to Flowsheet`).
    pub fn save_to_sim_project(&self, proj: &mut SimProject) -> usize {
        let ds_idx = self.active_dataset;
        let sol_opt = self.solutions.get(ds_idx).and_then(|s| s.as_ref());
        let Some(ds) = self.datasets.get(ds_idx) else {
            return 0;
        };
        let find_var = |name_match: &str| {
            self.variables
                .iter()
                .find(|v| v.name.to_lowercase() == name_match || v.type_code == VarTypeCode::SF && name_match == "sf")
                .map(|v| v.id)
        };
        let sf_vid = self.variables.iter().find(|v| v.type_code == VarTypeCode::SF).map(|v| v.id);
        let water_vid = self.variables.iter().find(|v| v.type_code == VarTypeCode::SC).map(|v| v.id);
        let cu_vid = find_var("cu");
        let fe_vid = find_var("fe");
        let s_vid = find_var("s");

        let mut updated = 0usize;
        for sim_s in &mut proj.streams {
            let Some(mb_s) = self.streams.iter().find(|s| s.id == sim_s.id || s.name.eq_ignore_ascii_case(&sim_s.name_id)) else {
                continue;
            };
            if let Some(sol) = sol_opt {
                if let Some(sfv) = sf_vid {
                    if let Some(c) = sol.cells.get(&CellKey { frac_idx: 0, stream_id: mb_s.id, var_id: sfv }) {
                        if let Some(b) = c.balanced {
                            sim_s.solids_flow_t_h = b;
                            sim_s.mass_flow_t_h = b + sim_s.liquid_flow_t_h;
                        }
                        if let Some(rec) = c.recovery_pct {
                            sim_s.solids_rec_pct = rec;
                        }
                    }
                }
                if let Some(wv) = water_vid {
                    if let Some(&w_b) = sol.solved_water_by_stream.get(&mb_s.id) {
                        if w_b > 0.0 {
                            sim_s.liquid_flow_t_h = w_b;
                            sim_s.mass_flow_t_h = sim_s.solids_flow_t_h + w_b;
                        }
                    }
                    let _ = wv;
                }
                if let Some(vid) = cu_vid {
                    if let Some(c) = sol.cells.get(&CellKey { frac_idx: 0, stream_id: mb_s.id, var_id: vid }) {
                        if let Some(b) = c.balanced {
                            sim_s.cu_wt_pct = b;
                        }
                        if let Some(rec) = c.recovery_pct {
                            sim_s.cu_rec_pct = rec;
                        }
                    }
                }
                if let Some(vid) = fe_vid {
                    if let Some(c) = sol.cells.get(&CellKey { frac_idx: 0, stream_id: mb_s.id, var_id: vid }) {
                        if let Some(b) = c.balanced {
                            sim_s.fe_wt_pct = b;
                        }
                    }
                }
                if let Some(vid) = s_vid {
                    if let Some(c) = sol.cells.get(&CellKey { frac_idx: 0, stream_id: mb_s.id, var_id: vid }) {
                        if let Some(b) = c.balanced {
                            sim_s.s_wt_pct = b;
                        }
                    }
                }
                updated += 1;
            }
            // Also transfer any Model Fit measured recoveries entered via `Enter Recoveries`
            if let Some(vid) = cu_vid {
                if let Some(&rec) = ds.recovered_for_model_fit.get(&(mb_s.id, vid)) {
                    sim_s.cu_rec_pct = rec;
                }
            }
        }
        if updated > 0 {
            proj.process_info.mass_balance = "YES".to_string();
        }
        updated
    }

    /// Generate `.xlsx` multi-sheet Excel workbook (`Stream Summary`, `Goodness`, `Unit Balance`, `Balance Details`).
    pub fn export_results_xlsx(&self) -> Result<Vec<u8>, String> {
        let sol = self
            .solutions
            .get(self.active_dataset)
            .and_then(|s| s.as_ref())
            .ok_or_else(|| "Calculate the mass balance before exporting to Excel".to_string())?;
        let active_streams: Vec<&MbStream> = self.streams.iter().filter(|s| s.selected).collect();
        let active_vars: Vec<&MbVariable> = self.variables.iter().filter(|v| v.selected).collect();

        // 1. Stream Summary
        let mut s_cells = vec![ExportCell {
            row: 0,
            col: 0,
            value: ExportCellValue::Text("Stream".into()),
            style: ExportCellStyle::Header,
        }];
        let mut col = 1;
        for v in &active_vars {
            for suffix in ["Meas.", "Bal.", "Rec %"] {
                s_cells.push(ExportCell {
                    row: 0,
                    col,
                    value: ExportCellValue::Text(format!("{} {} {}", v.name, v.unit, suffix)),
                    style: ExportCellStyle::Header,
                });
                col += 1;
            }
        }
        for (ri, s) in active_streams.iter().enumerate() {
            let row = ri + 1;
            s_cells.push(ExportCell {
                row,
                col: 0,
                value: ExportCellValue::Text(s.name.clone()),
                style: ExportCellStyle::BoldBox,
            });
            let mut c = 1;
            for v in &active_vars {
                let cell = sol.cells.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: v.id });
                for val_opt in [
                    cell.and_then(|x| x.measured),
                    cell.and_then(|x| x.balanced),
                    cell.and_then(|x| x.recovery_pct),
                ] {
                    s_cells.push(ExportCell {
                        row,
                        col: c,
                        value: val_opt.map(ExportCellValue::Number).unwrap_or(ExportCellValue::Empty),
                        style: ExportCellStyle::Default,
                    });
                    c += 1;
                }
            }
        }
        let sheet_summary = ExportSheet {
            name: "Stream Summary".into(),
            cells: s_cells,
            col_widths: vec![16.0; col],
        };

        // 2. Goodness
        let mut g_cells = Vec::new();
        for (ci, hdr) in ["Variable", "WSSQ", "Diff.Tot", "Rel.Dif", "AVG SD", "AVG RSD", "RMSD"].iter().enumerate() {
            g_cells.push(ExportCell {
                row: 0,
                col: ci,
                value: ExportCellValue::Text((*hdr).into()),
                style: ExportCellStyle::Header,
            });
        }
        for (ri, gr) in sol.goodness_vars.iter().enumerate() {
            let r = ri + 1;
            g_cells.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text(gr.name.clone()), style: ExportCellStyle::BoldBox });
            for (ci, v) in [gr.wssq, gr.diff_tot, gr.rel_diff_pct, gr.avg_sd, gr.avg_rsd_pct, gr.rmsd].iter().enumerate() {
                g_cells.push(ExportCell { row: r, col: ci + 1, value: ExportCellValue::Number(*v), style: ExportCellStyle::Default });
            }
        }
        let sheet_goodness = ExportSheet {
            name: "Goodness".into(),
            cells: g_cells,
            col_widths: vec![16.0; 7],
        };

        // 3. Unit Balance
        let mut u_cells = Vec::new();
        u_cells.push(ExportCell { row: 0, col: 0, value: ExportCellValue::Text("Units".into()), style: ExportCellStyle::Header });
        u_cells.push(ExportCell { row: 0, col: 1, value: ExportCellValue::Text("Equations".into()), style: ExportCellStyle::Header });
        let mut r = 1;
        for ub in &sol.unit_balances {
            u_cells.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text(ub.unit_name.clone()), style: ExportCellStyle::BoldBox });
            u_cells.push(ExportCell { row: r, col: 1, value: ExportCellValue::Text(ub.equation_text.clone()), style: ExportCellStyle::Default });
            for (ci, &val) in ub.residual_balance.iter().enumerate() {
                u_cells.push(ExportCell { row: r, col: ci + 2, value: ExportCellValue::Number(val), style: ExportCellStyle::Teal });
            }
            r += 1;
        }
        let sheet_unit = ExportSheet {
            name: "Unit Balance".into(),
            cells: u_cells,
            col_widths: vec![18.0; 8],
        };

        Ok(build_xlsx(&[sheet_summary, sheet_goodness, sheet_unit]))
    }

    pub fn export_summary_tsv(&self) -> String {
        let Some(sol) = self.solutions.get(self.active_dataset).and_then(|s| s.as_ref()) else {
            return String::new();
        };
        let active_streams: Vec<&MbStream> = self.streams.iter().filter(|s| s.selected).collect();
        let active_vars: Vec<&MbVariable> = self.variables.iter().filter(|v| v.selected).collect();
        let mut lines = Vec::new();
        let mut hdr = vec!["Stream".to_string()];
        for v in &active_vars {
            hdr.push(format!("{} {} Meas.", v.name, v.unit));
            hdr.push(format!("{} {} Bal.", v.name, v.unit));
            hdr.push(format!("{} Rec %", v.name));
        }
        lines.push(hdr.join("\t"));
        for s in active_streams {
            let mut cols = vec![s.name.clone()];
            for v in &active_vars {
                let c = sol.cells.get(&CellKey { frac_idx: 0, stream_id: s.id, var_id: v.id });
                cols.push(c.and_then(|x| x.measured).map(|v| format!("{v:.4}")).unwrap_or_default());
                cols.push(c.and_then(|x| x.balanced).map(|v| format!("{v:.4}")).unwrap_or_default());
                cols.push(c.and_then(|x| x.recovery_pct).map(|v| format!("{v:.2}")).unwrap_or_default());
            }
            lines.push(cols.join("\t"));
        }
        lines.join("\n")
    }
}

fn atomic_wt(elem: &str) -> f64 {
    match elem {
        "Cu" => 63.546,
        "Fe" => 55.845,
        "S" => 32.065,
        "Zn" => 65.38,
        "Pb" => 207.2,
        "Si" => 28.085,
        "O" => 15.999,
        "H" => 1.008,
        "C" => 12.011,
        "Ca" => 40.078,
        "Al" => 26.982,
        "Mg" => 24.305,
        "As" => 74.922,
        "Ag" => 107.868,
        "Au" => 196.967,
        "Ni" => 58.693,
        "Co" => 58.933,
        _ => 1.0,
    }
}

fn default_chart_panels() -> Vec<ChartPanelSpec> {
    vec![
        ChartPanelSpec {
            title: "Total Solids".into(),
            show_total_solids: true,
            show_stream_composition: false,
            selected_var_ids: Vec::new(),
        },
        ChartPanelSpec {
            title: "Stream Composition".into(),
            show_total_solids: false,
            show_stream_composition: true,
            selected_var_ids: Vec::new(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Built-In Presets (Chapter 51 Official Examples: Figs. 3, 9, 24–25)
// ---------------------------------------------------------------------------

/// Flotation rougher circuit (`Figs. 3, 9, 13–23`: Conditioner + Rougher 1..5).
pub fn preset_rougher_flotation_mb() -> MassBalanceWorkbook {
    let units = vec![
        MbUnit { id: 1, name: "Conditioner".into(), selected: true, size_transforming: false },
        MbUnit { id: 2, name: "Rougher 1".into(), selected: true, size_transforming: false },
        MbUnit { id: 3, name: "Rougher 2".into(), selected: true, size_transforming: false },
        MbUnit { id: 4, name: "Rougher 3".into(), selected: true, size_transforming: false },
        MbUnit { id: 5, name: "Rougher 4".into(), selected: true, size_transforming: false },
        MbUnit { id: 6, name: "Rougher 5".into(), selected: true, size_transforming: false },
    ];

    let mk_s = |id: usize, name: &str, src: Option<usize>, dst: Option<usize>, flows: bool, assays: bool| MbStream {
        id,
        name: name.into(),
        selected: true,
        stream_type: StreamType::SolidsSlurry,
        flows_enabled: flows,
        assays_enabled: assays,
        sampling_device: "Default".into(),
        sampling_sd: 0.0,
        sample_quality: SampleQuality::Good,
        source_unit: src,
        dest_unit: dst,
    };

    let streams = vec![
        mk_s(10, "Feed", None, Some(1), false, true),
        mk_s(11, "RT1", Some(2), Some(3), false, false),
        mk_s(12, "RT2", Some(3), Some(4), false, false),
        mk_s(13, "RT3", Some(4), Some(5), false, false),
        mk_s(14, "RT4", Some(5), Some(6), false, false),
        mk_s(15, "RT5", Some(6), None, true, true),
        mk_s(16, "RC1", Some(2), None, true, true),
        mk_s(17, "RC2", Some(3), None, true, true),
        mk_s(18, "RC3", Some(4), None, true, true),
        mk_s(19, "RC4", Some(5), None, true, true),
        mk_s(20, "RC5", Some(6), None, true, true),
        mk_s(21, "RC Feed", Some(1), Some(2), false, false),
    ];

    let mk_v = |id: usize, name: &str, unit: &str, tc: VarTypeCode, mm: MeasurementMethod, a: f64, b: f64, c: f64, f: Option<&str>, sg: Option<f64>| MbVariable {
        id,
        name: name.into(),
        unit: unit.into(),
        type_code: tc,
        selected: true,
        method: mm,
        param_a: a,
        param_b: b,
        param_c: c,
        low_grade_weighted: false,
        mineral_formula: f.map(str::to_string),
        mineral_sg: sg,
    };

    let variables = vec![
        mk_v(30, "Mass", "g", VarTypeCode::SF, MeasurementMethod::FixedRelative, 5.0, 0.0, 0.0, None, None),
        mk_v(31, "Cu", "%", VarTypeCode::A, MeasurementMethod::FixedRelative, 5.0, 0.0, 0.0, None, None),
        mk_v(32, "Fe", "%", VarTypeCode::A, MeasurementMethod::FixedRelative, 5.0, 0.0, 0.0, None, None),
        mk_v(33, "S", "%", VarTypeCode::A, MeasurementMethod::FixedRelative, 5.0, 0.0, 0.0, None, None),
        mk_v(34, "Zn", "%", VarTypeCode::A, MeasurementMethod::FixedRelative, 5.0, 0.0, 0.0, None, None),
        mk_v(35, "Ccp", "%", VarTypeCode::M, MeasurementMethod::SShape, 5.0, 0.01, 2.0, Some("CuFeS2"), Some(4.19)),
        mk_v(36, "Py", "%", VarTypeCode::M, MeasurementMethod::SShape, 5.0, 0.01, 5.0, Some("FeS2"), Some(5.01)),
        mk_v(37, "Sp", "%", VarTypeCode::M, MeasurementMethod::SShape, 5.0, 0.01, 2.0, Some("ZnS"), Some(4.05)),
        mk_v(38, "Qtz", "%", VarTypeCode::M, MeasurementMethod::SShape, 5.0, 0.01, 5.0, Some("SiO2"), Some(2.65)),
    ];

    let mut ds = MbDataset::default();
    ds.name = "Data Set 1".into();
    ds.feed_stream_id = Some(10);

    // Exact observed black values from Fig. 9 & Fig. 12 & Fig. 15
    let rows: &[(usize, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>)] = &[
        // sid, Mass, Cu%, Fe%, S%, Zn%
        (10, None, Some(1.013), Some(36.3), Some(41.3), Some(1.963)),
        (15, Some(13899.0), Some(0.0820), Some(30.20), Some(35.6), Some(1.960)),
        (16, Some(464.0), Some(23.60), Some(29.20), Some(34.4), Some(5.78)),
        (17, Some(118.4), Some(17.00), Some(28.90), Some(34.9), Some(8.56)),
        (18, Some(166.7), Some(12.50), Some(28.60), Some(34.8), Some(8.45)),
        (19, Some(207.6), Some(3.56), Some(31.20), Some(35.9), Some(6.07)),
        (20, Some(241.1), Some(0.560), Some(31.8), Some(36.4), Some(3.44)),
    ];
    for &(sid, m, cu, fe, s, zn) in rows {
        for (vid, vopt) in [(30, m), (31, cu), (32, fe), (33, s), (34, zn)] {
            if let Some(v) = vopt {
                ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: vid }, v);
            }
        }
    }

    // Approximate modal minerals matching Figs. 12–15 (`Feed` + `RC1..5` + `RT5`)
    let min_rows: &[(usize, f64, f64, f64, f64)] = &[
        // sid, Ccp%, Py%, Sp%, Qtz%
        (10, 2.927, 73.5, 2.926, 20.66),
        (15, 0.237, 66.2, 2.92, 30.6),
        (16, 68.16, 15.2, 8.61, 8.0),
        (17, 49.10, 25.0, 12.75, 13.1),
        (18, 36.10, 33.8, 12.59, 17.5),
        (19, 10.28, 54.9, 9.05, 25.8),
        (20, 1.62, 63.0, 5.13, 30.2),
    ];
    for &(sid, ccp, py, sp, qtz) in min_rows {
        for (vid, val) in [(35, ccp), (36, py), (37, sp), (38, qtz)] {
            ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: vid }, val);
        }
    }

    MassBalanceWorkbook {
        units,
        streams,
        variables,
        size_fractions: vec!["Bulk".into()],
        datasets: vec![ds],
        active_dataset: 0,
        stream_equations: Vec::new(),
        balancing_of_minerals: false,
        next_id: 50,
        solutions: vec![None],
        chart_panels: default_chart_panels(),
    }
}

/// Grinding circuit (`Figs. 24–25`: SAG [Size Transforming] + Cyclone + Process Waters with size fractions).
pub fn preset_grinding_circuit_mb() -> MassBalanceWorkbook {
    let units = vec![
        MbUnit { id: 1, name: "SAG".into(), selected: true, size_transforming: true },
        MbUnit { id: 2, name: "Cyclone".into(), selected: true, size_transforming: false },
        MbUnit { id: 3, name: "Process Waters".into(), selected: false, size_transforming: false },
    ];
    let mk_s = |id: usize, name: &str, src: Option<usize>, dst: Option<usize>, st: StreamType| MbStream {
        id,
        name: name.into(),
        selected: true,
        stream_type: st,
        flows_enabled: true,
        assays_enabled: st != StreamType::LiquidWater,
        sampling_device: "Default".into(),
        sampling_sd: 0.0,
        sample_quality: SampleQuality::Good,
        source_unit: src,
        dest_unit: dst,
    };
    let streams = vec![
        mk_s(10, "ROM", None, Some(1), StreamType::SolidsSlurry),
        mk_s(11, "SAG Discharge", Some(1), Some(2), StreamType::SolidsSlurry),
        mk_s(12, "Cyclone UF", Some(2), Some(1), StreamType::SolidsSlurry),
        mk_s(13, "Cyclone OF", Some(2), None, StreamType::SolidsSlurry),
        mk_s(14, "Mill Water", Some(3), Some(1), StreamType::LiquidWater),
        mk_s(15, "Mill Sump Water", Some(3), Some(2), StreamType::LiquidWater),
    ];
    let variables = vec![
        MbVariable {
            id: 30, name: "Solids Flowrate".into(), unit: "t/h".into(), type_code: VarTypeCode::SF,
            selected: true, method: MeasurementMethod::FixedRelative, param_a: 5.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        },
        MbVariable {
            id: 31, name: "Water".into(), unit: "t/h".into(), type_code: VarTypeCode::SC,
            selected: true, method: MeasurementMethod::FixedRelative, param_a: 5.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        },
        MbVariable {
            id: 32, name: "%Solids".into(), unit: "%".into(), type_code: VarTypeCode::SP,
            selected: true, method: MeasurementMethod::FixedRelative, param_a: 3.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        },
        MbVariable {
            id: 33, name: "Cu".into(), unit: "%".into(), type_code: VarTypeCode::A,
            selected: true, method: MeasurementMethod::FixedRelative, param_a: 5.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        },
        MbVariable {
            id: 34, name: "Weight Retained".into(), unit: "%".into(), type_code: VarTypeCode::SA,
            selected: true, method: MeasurementMethod::SizeFractionWhiten, param_a: 0.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        },
    ];
    let size_fractions = vec![
        "Bulk".into(),
        "0-53um".into(),
        "53-150um".into(),
        "150-600um".into(),
        "600-2360um".into(),
        "2360-20000um".into(),
    ];

    let mut ds = MbDataset::default();
    ds.name = "3 Grinding Survey".into();
    ds.sizing = SizingMode::Sized;
    ds.feed_stream_id = Some(10);

    // Fig. 25 values (agrupadas exactamente en 5 fracciones compatibles sum = 100%)
    let bulk = [
        (10, 517.5, 20.0, 96.6, 1.20),
        (11, 1810.0, 600.0, 75.7, 1.22),
        (12, 1290.0, 520.0, 71.2, 1.30),
        (13, 517.5, 600.0, 44.6, 1.96),
        (14, 0.0, 180.0, 0.0, 0.0),
        (15, 0.0, 200.0, 0.0, 0.0),
    ];
    for (sid, sf, w, sp, cu) in bulk {
        ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: 30 }, sf);
        ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: 31 }, w);
        ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: 32 }, sp);
        if sf > 0.0 {
            ds.values.insert(CellKey { frac_idx: 0, stream_id: sid, var_id: 33 }, cu);
        }
    }
    let psd = [
        (10, [9.7, 6.4, 12.4, 17.2, 54.3]),
        (11, [18.6, 13.4, 36.8, 19.7, 11.5]),
        (12, [11.1, 9.0, 39.7, 27.1, 13.1]),
        (13, [35.9, 25.2, 34.7, 4.1, 0.1]),
    ];
    for (sid, vals) in psd {
        for (fi_off, val) in vals.iter().enumerate() {
            ds.values.insert(CellKey { frac_idx: fi_off + 1, stream_id: sid, var_id: 34 }, *val);
        }
    }

    MassBalanceWorkbook {
        units,
        streams,
        variables,
        size_fractions,
        datasets: vec![ds],
        active_dataset: 0,
        stream_equations: Vec::new(),
        balancing_of_minerals: false,
        next_id: 60,
        solutions: vec![None],
        chart_panels: default_chart_panels(),
    }
}

// ---------------------------------------------------------------------------
// Comprehensive Verification Tests (`MB-01 .. MB-10`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mb01_mb05_rougher_flotation_ewtls_wls_closure() {
        let mut wb = preset_rougher_flotation_mb();
        let issues = wb.check_errors(0);
        assert!(
            issues.iter().all(|i| i.severity != CheckSeverity::Error),
            "Unexpected error checks: {:?}",
            issues
        );
        let sol = wb.calculate_dataset(0).expect("rougher reconciliation");
        assert!(sol.converged);
        // Check unit closures < 1e-9 (`Fig. 22`)
        for ub in &sol.unit_balances {
            for (ci, &res) in ub.residual_balance.iter().enumerate() {
                assert!(
                    res.abs() < 1e-8,
                    "Unit '{}' comp {} closure error too high: {}",
                    ub.unit_name,
                    ci,
                    res
                );
            }
        }
        // Check Feed ≈ sum(RC1..5) + RT5 ≈ 15097 (`Fig. 20`)
        let feed_mass = sol
            .cells
            .get(&CellKey { frac_idx: 0, stream_id: 10, var_id: 30 })
            .and_then(|c| c.balanced)
            .unwrap();
        assert!((feed_mass - 15097.0).abs() < 5.0, "Feed mass was {feed_mass}");

        // Sum recoveries across tails & concentrates == 100%
        let rec_sum: f64 = [15_usize, 16, 17, 18, 19, 20]
            .iter()
            .map(|&sid| {
                sol.cells
                    .get(&CellKey { frac_idx: 0, stream_id: sid, var_id: 30 })
                    .and_then(|c| c.recovery_pct)
                    .unwrap_or(0.0)
            })
            .sum();
        assert!((rec_sum - 100.0).abs() < 1e-5, "Rec sum = {rec_sum}");
    }

    #[test]
    fn test_mb02_stream_by_equation() {
        let mut wb = preset_rougher_flotation_mb();
        // Set RC5 Mass = 0.5 * RC1 Mass
        wb.stream_equations.push(StreamEquation {
            target_stream_id: 20,
            source_stream_id: 16,
            factor: 0.5,
        });
        let sol = wb.calculate_dataset(0).expect("stream equation solve");
        let f_rc5 = sol.cells[&CellKey { frac_idx: 0, stream_id: 20, var_id: 30 }].balanced.unwrap();
        let f_rc1 = sol.cells[&CellKey { frac_idx: 0, stream_id: 16, var_id: 30 }].balanced.unwrap();
        assert!((f_rc5 - 0.5 * f_rc1).abs() < 1e-8, "f_rc5={f_rc5}, f_rc1={f_rc1}");
    }

    #[test]
    fn test_mb03_error_models_table3_table4() {
        let s_bad = MbStream {
            id: 1, name: "S".into(), selected: true, stream_type: StreamType::SolidsSlurry,
            flows_enabled: true, assays_enabled: true, sampling_device: "Sampler".into(),
            sampling_sd: 0.1, sample_quality: SampleQuality::Bad, source_unit: None, dest_unit: Some(1),
        };
        let v_rel = MbVariable {
            id: 1, name: "Cu".into(), unit: "%".into(), type_code: VarTypeCode::A, selected: true,
            method: MeasurementMethod::FixedRelative, param_a: 5.0, param_b: 0.0, param_c: 0.0,
            low_grade_weighted: false, mineral_formula: None, mineral_sg: None,
        };
        let (sd_m, rsd, sd_s, sd_e) = evaluate_cell_uncertainty(Some(10.0), &v_rel, &s_bad, PreMeasStatus::Measured, None, 0.001, 0.0);
        assert!((sd_m - 0.5).abs() < 1e-9);
        assert!((rsd - 5.0).abs() < 1e-9);
        assert_eq!(sd_s, 0.0, "FixedRelative must NOT include sampling error");
        assert!((sd_e - 0.5).abs() < 1e-9);

        // Whiten PSD
        let v_wh = MbVariable {
            method: MeasurementMethod::SizeFractionWhiten,
            ..v_rel.clone()
        };
        let (sd5, _, sd_samp5, _) = evaluate_cell_uncertainty(Some(5.0), &v_wh, &s_bad, PreMeasStatus::Measured, None, 0.001, 0.0);
        assert!((sd5 - 0.6).abs() < 1e-9);
        assert!((sd_samp5 - 0.3).abs() < 1e-9, "Bad quality = 3 * 0.1 = 0.3");
        let (sd15, _, _, _) = evaluate_cell_uncertainty(Some(15.0), &v_wh, &s_bad, PreMeasStatus::Measured, None, 0.001, 0.0);
        assert!((sd15 - 1.0).abs() < 1e-9);

        // Mineral grade based: RSD% = a * x^-0.5 -> a=5, x=4 => RSD=2.5%, SD=0.10
        let v_mg = MbVariable { method: MeasurementMethod::MineralGradeBased, param_a: 5.0, ..v_rel };
        let (sd_mg, _, _, _) = evaluate_cell_uncertainty(Some(4.0), &v_mg, &s_bad, PreMeasStatus::Measured, None, 0.001, 0.0);
        assert!((sd_mg - 0.10).abs() < 1e-9);
    }

    #[test]
    fn test_mb04_matlab_scaling_fig18_exact() {
        let assays = vec![
            vec![1.3360, 4.9610],
            vec![4.1833, 2.8075],
            vec![41.0950, 3.4228],
        ];
        let assays_sd = vec![
            vec![0.1282, 0.1488],
            vec![0.3684, 0.3553], // indexOfInput = 1 (0-based -> 2nd stream in Fig. 17)
            vec![5.5495, 0.9885],
        ];
        let scaled = apply_assay_scaling(&assays, &assays_sd, 1);
        assert!((scaled[0][0] - 0.0363).abs() < 1e-4, "got {}", scaled[0][0]);
        assert!((scaled[0][1] - 0.1396).abs() < 1e-4, "got {}", scaled[0][1]);
        assert!((scaled[2][0] - 1.1155).abs() < 1e-4, "got {}", scaled[2][0]);
    }

    #[test]
    fn test_mb06_nnls_cls_lls_and_sum100() {
        let mut wb = preset_rougher_flotation_mb();
        wb.balancing_of_minerals = true;
        wb.datasets[0].mineral_sum_100 = true;
        wb.datasets[0].method = SolveMethod::CLS;
        // Bound Ccp on RC1 to [65.0, 69.0]
        wb.datasets[0].min_bounds.insert(CellKey { frac_idx: 0, stream_id: 16, var_id: 35 }, 65.0);
        wb.datasets[0].max_bounds.insert(CellKey { frac_idx: 0, stream_id: 16, var_id: 35 }, 69.0);
        let sol = wb.calculate_dataset(0).expect("CLS + MineralSum=100");
        let ccp_rc1 = sol.cells[&CellKey { frac_idx: 0, stream_id: 16, var_id: 35 }].balanced.unwrap();
        assert!((64.99..=69.01).contains(&ccp_rc1), "CLS violated: {ccp_rc1}");

        // Check sum of minerals = 100 on all streams
        for s in &wb.streams {
            let sum_m: f64 = [35_usize, 36, 37, 38]
                .iter()
                .map(|&vid| sol.cells[&CellKey { frac_idx: 0, stream_id: s.id, var_id: vid }].balanced.unwrap())
                .sum();
            assert!((sum_m - 100.0).abs() < 1e-6, "Stream '{}' min sum = {sum_m}", s.name);
        }
    }

    #[test]
    fn test_mb07_mb08_grinding_water_and_sized_non_sru() {
        let mut wb = preset_grinding_circuit_mb();
        // Verify Sized mode excludes SAG (Size Transforming) from PSD conservation while enforcing Cyclone PSD & bulk conservation
        let sol = wb.calculate_dataset(0).expect("sized grinding");
        assert!(sol.converged);
        // Cyclone PSD conservation across sieves (`Eq. 17`) and Sum = 100 (`Eq. 18`)
        let f_sag_d = sol.cells[&CellKey { frac_idx: 0, stream_id: 11, var_id: 30 }].balanced.unwrap();
        let f_cuf = sol.cells[&CellKey { frac_idx: 0, stream_id: 12, var_id: 30 }].balanced.unwrap();
        let f_cof = sol.cells[&CellKey { frac_idx: 0, stream_id: 13, var_id: 30 }].balanced.unwrap();
        assert!((f_sag_d - (f_cuf + f_cof)).abs() < 1e-6);

        for sid in [10_usize, 11, 12, 13] {
            let sum_sa: f64 = (1..wb.size_fractions.len())
                .map(|fi| sol.cells[&CellKey { frac_idx: fi, stream_id: sid, var_id: 34 }].balanced.unwrap())
                .sum();
            assert!((sum_sa - 100.0).abs() < 1e-5, "Stream {sid} sieve sum = {sum_sa}");
        }
        // Water balance
        wb.datasets[0].sizing = SizingMode::Unsized;
        wb.datasets[0].balance_type = BalanceType::SolidsWaterAndAssays;
        let sol_w = wb.calculate_dataset(0).expect("water + %solids balance");
        let pc_cof = sol_w.cells[&CellKey { frac_idx: 0, stream_id: 13, var_id: 32 }].balanced.unwrap();
        let f_cof2 = sol_w.cells[&CellKey { frac_idx: 0, stream_id: 13, var_id: 30 }].balanced.unwrap();
        let w_cof2 = sol_w.cells[&CellKey { frac_idx: 0, stream_id: 13, var_id: 31 }].balanced.unwrap();
        assert!((pc_cof - 100.0 * f_cof2 / (f_cof2 + w_cof2)).abs() < 1e-6);
    }

    #[test]
    fn test_mb09_mb10_sized_by_assay_errors_and_hsc7_import() {
        let mut wb = preset_grinding_circuit_mb();
        // Select LLS + Sized -> Rule 12 error/notification
        wb.datasets[0].method = SolveMethod::LLS;
        let issues = wb.check_errors(0);
        assert!(issues.iter().any(|i| i.code == 12 && i.title.contains("Method LLS not available")));
        assert!(wb.calculate_dataset(0).is_err());

        // Test SizedByAssay
        wb.datasets[0].method = SolveMethod::NNLS;
        wb.datasets[0].sizing = SizingMode::SizedByAssay;
        let sol_sba = wb.calculate_dataset(0).expect("sized by assay solve");
        let f_cof_tot = sol_sba.cells[&CellKey { frac_idx: 0, stream_id: 13, var_id: 30 }].balanced.unwrap();
        let sub_sum: f64 = (1..wb.size_fractions.len())
            .map(|fi| sol_sba.cells[&CellKey { frac_idx: fi, stream_id: 13, var_id: 30 }].balanced.unwrap())
            .sum();
        assert!((sub_sum - f_cof_tot).abs() < 1e-6, "Eq 20 failed: {sub_sum} vs {f_cof_tot}");

        // Test HSC7 import from Fig. 24 + 25 text
        let fs_csv = "Units\tROM\tSAG Discharge\nSAG\t1\t-1\nCyclone\t0\t1";
        let data_csv = "Stream->\tROM\tSAG Discharge\nSource\t?\tSAG\nDestination\tSAG\tCyclone\nSolids Flowrate t/h\t517.5\t1810\nCu %\t1.2\t1.22";
        let mut wb2 = MassBalanceWorkbook::default();
        let imported = wb2.import_hsc7_text(Some(fs_csv), data_csv, false).expect("hsc7 import");
        assert!(imported >= 4);
        assert_eq!(wb2.units.len(), 2);
    }
}
