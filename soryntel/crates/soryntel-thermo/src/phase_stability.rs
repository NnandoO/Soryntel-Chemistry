use std::collections::{BTreeMap, HashMap};

use soryntel_core::{CAL_TO_JOULE, R_JOULE};
use soryntel_solver::parse_term;

use crate::{
    evaluate_species_si, sum_element_references, ChemicalSpecies, Phase, SpeciesRepository,
};

const LN_10: f64 = std::f64::consts::LN_10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisVariable {
    Temperature,
    Gas(String),
}

impl AxisVariable {
    pub fn label(&self, temp_unit: TemperatureUnit) -> String {
        match self {
            AxisVariable::Temperature => match temp_unit {
                TemperatureUnit::Celsius => "T / °C".to_string(),
                TemperatureUnit::Kelvin => "T / K".to_string(),
            },
            AxisVariable::Gas(g) => format!("log p{g}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureUnit {
    Celsius,
    Kelvin,
}

impl TemperatureUnit {
    pub fn to_kelvin(self, val: f64) -> f64 {
        match self {
            TemperatureUnit::Celsius => val + 273.15,
            TemperatureUnit::Kelvin => val,
        }
    }

    pub fn from_kelvin(self, k: f64) -> f64 {
        match self {
            TemperatureUnit::Celsius => k - 273.15,
            TemperatureUnit::Kelvin => k,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaseStabilityInput {
    pub elements: [String; 3],
    pub selected_species: Vec<String>,
    pub x_axis: AxisVariable,
    pub y_axis: AxisVariable,
    pub temp_unit: TemperatureUnit,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub auto_scale: bool,
    pub fixed_temperature_k: f64,
    pub fixed_gas: Option<(String, f64)>,
    pub include_liquids: bool,
    pub include_organics: bool,
    pub nx: usize,
    pub ny: usize,
}

#[derive(Debug, Clone)]
pub struct PhaseStabilitySpeciesRow {
    pub formula: String,
    pub phase: Phase,
    pub elem_counts: [f64; 3],
    pub delta_g_kcal_mol: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct StabilityBoundarySegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[derive(Debug, Clone)]
pub struct StabilityRegionLabel {
    pub formula: String,
    pub x: f64,
    pub y: f64,
    pub area_fraction: f64,
}

#[derive(Debug, Clone)]
pub struct StabilityLegendEntry {
    pub formula: String,
    pub phase_id: usize,
}

#[derive(Debug, Clone)]
pub struct PhaseStabilityDiagram {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub footer: Option<String>,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub nx: usize,
    pub ny: usize,
    pub grid_phase_ids: Vec<usize>,
    pub legend: Vec<StabilityLegendEntry>,
    pub boundaries: Vec<StabilityBoundarySegment>,
    pub labels: Vec<StabilityRegionLabel>,
    pub table_rows: Vec<PhaseStabilitySpeciesRow>,
    pub temperature_k: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseStabilityError {
    DuplicateAxisVariable,
    MissingGasSpecies(String),
    CollinearGasMatrix { g1: String, g2: String },
    InvalidRange,
    NoCondensedSpecies,
    InvalidFixedVariable,
}

impl std::fmt::Display for PhaseStabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateAxisVariable => {
                write!(f, "X, Y, and fixed variables must all be distinct.")
            }
            Self::MissingGasSpecies(g) => {
                write!(f, "Reference gas species '{g}' was not found in database.")
            }
            Self::CollinearGasMatrix { g1, g2 } => {
                write!(
                    f,
                    "Reference gases '{g1}' and '{g2}' are linearly dependent or contain the base metal."
                )
            }
            Self::InvalidRange => write!(f, "Axis minimum must be strictly less than maximum."),
            Self::NoCondensedSpecies => {
                write!(f, "No valid condensed species found for the selected system.")
            }
            Self::InvalidFixedVariable => {
                write!(
                    f,
                    "Fixed partial pressure must be positive (> 0 bar) and temperature > 0 K."
                )
            }
        }
    }
}

impl std::error::Error for PhaseStabilityError {}

struct CondensedCandidate {
    species: ChemicalSpecies,
    elem_counts: [f64; 3],
    m_count: f64,
    alpha: f64,
    beta: f64,
}

enum EvalMode<'a> {
    PP {
        t_k: f64,
        g1: &'a ChemicalSpecies,
        g2: &'a ChemicalSpecies,
    },
    TP {
        temp_unit: TemperatureUnit,
        x_is_temp: bool,
        g_axis: &'a ChemicalSpecies,
        g_fixed: &'a ChemicalSpecies,
        log_p_fixed: f64,
    },
}

fn element_vector(formula: &str, elements: &[String; 3]) -> Option<[f64; 3]> {
    let parsed = parse_term(formula).ok()?;
    let mut vec = [0.0_f64; 3];
    for (el, count) in &parsed.elements {
        if let Some(pos) = elements.iter().position(|e| e == el) {
            vec[pos] += *count;
        } else {
            return None;
        }
    }
    Some(vec)
}

fn invert_2x2(c: [[f64; 2]; 2]) -> Option<[[f64; 2]; 2]> {
    let det = c[0][0] * c[1][1] - c[0][1] * c[1][0];
    if det.abs() < 1e-9 {
        return None;
    }
    Some([
        [c[1][1] / det, -c[0][1] / det],
        [-c[1][0] / det, c[0][0] / det],
    ])
}

fn is_organic(elements_map: &BTreeMap<String, f64>) -> bool {
    elements_map.contains_key("C") && elements_map.contains_key("H")
}

fn find_gas_species(
    repo: &dyn SpeciesRepository,
    formula: &str,
) -> Result<ChemicalSpecies, PhaseStabilityError> {
    repo.get_species(formula)
        .filter(|s| s.phase == Phase::Gas)
        .ok_or_else(|| PhaseStabilityError::MissingGasSpecies(formula.to_string()))
}

fn g_si(sp: &ChemicalSpecies, t_k: f64) -> Option<f64> {
    evaluate_species_si(sp, t_k, false, false)
        .ok()
        .map(|(pt, _)| pt.g)
}

fn eval_winner(
    x: f64,
    y: f64,
    mode: &EvalMode<'_>,
    candidates: &[CondensedCandidate],
) -> usize {
    let (t_k, g1_sp, log_p1, g2_sp, log_p2) = match mode {
        EvalMode::PP { t_k, g1, g2 } => (*t_k, *g1, x, *g2, y),
        EvalMode::TP {
            temp_unit,
            x_is_temp,
            g_axis,
            g_fixed,
            log_p_fixed,
        } => {
            if *x_is_temp {
                (
                    temp_unit.to_kelvin(x).max(1.0),
                    *g_fixed,
                    *log_p_fixed,
                    *g_axis,
                    y,
                )
            } else {
                (
                    temp_unit.to_kelvin(y).max(1.0),
                    *g_fixed,
                    *log_p_fixed,
                    *g_axis,
                    x,
                )
            }
        }
    };

    let rt_ln10 = R_JOULE * t_k * LN_10;
    let mu_1 = g_si(g1_sp, t_k).unwrap_or(0.0) + rt_ln10 * log_p1;
    let mu_2 = g_si(g2_sp, t_k).unwrap_or(0.0) + rt_ln10 * log_p2;

    let mut best_idx = 0usize;
    let mut best_g = f64::INFINITY;
    for (idx, cand) in candidates.iter().enumerate() {
        let Some(g_cond) = g_si(&cand.species, t_k) else {
            continue;
        };
        let g_norm = (g_cond - cand.alpha * mu_1 - cand.beta * mu_2) / cand.m_count;
        if g_norm < best_g {
            best_g = g_norm;
            best_idx = idx;
        }
    }
    best_idx
}

fn bisect_boundary(
    mut x0: f64,
    mut y0: f64,
    mut x1: f64,
    mut y1: f64,
    p0: usize,
    mode: &EvalMode<'_>,
    candidates: &[CondensedCandidate],
) -> (f64, f64) {
    for _ in 0..10 {
        let xm = 0.5 * (x0 + x1);
        let ym = 0.5 * (y0 + y1);
        let pm = eval_winner(xm, ym, mode, candidates);
        if pm == p0 {
            x0 = xm;
            y0 = ym;
        } else {
            x1 = xm;
            y1 = ym;
        }
    }
    (0.5 * (x0 + x1), 0.5 * (y0 + y1))
}

fn auto_scale_pp_bounds(
    mode: &EvalMode<'_>,
    candidates: &[CondensedCandidate],
) -> (f64, f64, f64, f64) {
    let coarse_n = 72usize;
    let lo = -140.0_f64;
    let hi = 20.0_f64;
    let dx = (hi - lo) / (coarse_n - 1) as f64;

    let mut grid = vec![0usize; coarse_n * coarse_n];
    for iy in 0..coarse_n {
        let y = lo + iy as f64 * dx;
        for ix in 0..coarse_n {
            let x = lo + ix as f64 * dx;
            grid[iy * coarse_n + ix] = eval_winner(x, y, mode, candidates);
        }
    }

    let mut min_tx = f64::INFINITY;
    let mut max_tx = f64::NEG_INFINITY;
    let mut min_ty = f64::INFINITY;
    let mut max_ty = f64::NEG_INFINITY;

    for iy in 0..coarse_n {
        for ix in 0..coarse_n {
            let p = grid[iy * coarse_n + ix];
            let x = lo + ix as f64 * dx;
            let y = lo + iy as f64 * dx;
            if ix + 1 < coarse_n && grid[iy * coarse_n + (ix + 1)] != p {
                min_tx = min_tx.min(x);
                max_tx = max_tx.max(x + dx);
                min_ty = min_ty.min(y);
                max_ty = max_ty.max(y);
            }
            if iy + 1 < coarse_n && grid[(iy + 1) * coarse_n + ix] != p {
                min_tx = min_tx.min(x);
                max_tx = max_tx.max(x);
                min_ty = min_ty.min(y);
                max_ty = max_ty.max(y + dx);
            }
        }
    }

    if !min_tx.is_finite() {
        return (-45.0, 5.0, -80.0, 5.0);
    }

    let round5_down = |v: f64| ((v - 5.0) / 5.0).floor() * 5.0;
    let round5_up = |v: f64| ((v + 5.0) / 5.0).ceil() * 5.0;

    let x_min = round5_down(min_tx).max(-140.0);
    let x_max = round5_up(max_tx).min(20.0);
    let y_min = round5_down(min_ty).max(-140.0);
    let y_max = round5_up(max_ty).min(20.0);

    (
        x_min,
        if x_max > x_min { x_max } else { x_min + 20.0 },
        y_min,
        if y_max > y_min { y_max } else { y_min + 20.0 },
    )
}

pub fn calculate_phase_stability(
    repo: &dyn SpeciesRepository,
    input: &PhaseStabilityInput,
) -> Result<PhaseStabilityDiagram, PhaseStabilityError> {
    if input.x_axis == input.y_axis {
        return Err(PhaseStabilityError::DuplicateAxisVariable);
    }

    let (g1_name, g2_name, is_pp, x_is_temp, fixed_p_bar) = match (&input.x_axis, &input.y_axis) {
        (AxisVariable::Gas(gx), AxisVariable::Gas(gy)) => {
            if input.fixed_temperature_k <= 0.0 {
                return Err(PhaseStabilityError::InvalidFixedVariable);
            }
            (gx.clone(), gy.clone(), true, false, 1.0)
        }
        (AxisVariable::Temperature, AxisVariable::Gas(gy)) => {
            let (gf, pf) = input
                .fixed_gas
                .as_ref()
                .ok_or(PhaseStabilityError::InvalidFixedVariable)?;
            if gf == gy || *pf <= 0.0 {
                return Err(if gf == gy {
                    PhaseStabilityError::DuplicateAxisVariable
                } else {
                    PhaseStabilityError::InvalidFixedVariable
                });
            }
            (gf.clone(), gy.clone(), false, true, *pf)
        }
        (AxisVariable::Gas(gx), AxisVariable::Temperature) => {
            let (gf, pf) = input
                .fixed_gas
                .as_ref()
                .ok_or(PhaseStabilityError::InvalidFixedVariable)?;
            if gf == gx || *pf <= 0.0 {
                return Err(if gf == gx {
                    PhaseStabilityError::DuplicateAxisVariable
                } else {
                    PhaseStabilityError::InvalidFixedVariable
                });
            }
            (gf.clone(), gx.clone(), false, false, *pf)
        }
        (AxisVariable::Temperature, AxisVariable::Temperature) => {
            return Err(PhaseStabilityError::DuplicateAxisVariable);
        }
    };

    let g1_sp = find_gas_species(repo, &g1_name)?;
    let g2_sp = find_gas_species(repo, &g2_name)?;

    let v1 = element_vector(&g1_sp.formula, &input.elements).ok_or_else(|| {
        PhaseStabilityError::CollinearGasMatrix {
            g1: g1_name.clone(),
            g2: g2_name.clone(),
        }
    })?;
    let v2 = element_vector(&g2_sp.formula, &input.elements).ok_or_else(|| {
        PhaseStabilityError::CollinearGasMatrix {
            g1: g1_name.clone(),
            g2: g2_name.clone(),
        }
    })?;

    // Find the metal/base element M absent from both reference gases.
    let mut chosen: Option<(usize, usize, usize, [[f64; 2]; 2])> = None;
    for m_idx in 0..3 {
        if v1[m_idx].abs() < 1e-12 && v2[m_idx].abs() < 1e-12 {
            let others: Vec<usize> = (0..3).filter(|&i| i != m_idx).collect();
            let a_idx = others[0];
            let b_idx = others[1];
            let c = [[v1[a_idx], v2[a_idx]], [v1[b_idx], v2[b_idx]]];
            if let Some(c_inv) = invert_2x2(c) {
                chosen = Some((m_idx, a_idx, b_idx, c_inv));
                break;
            }
        }
    }

    let (m_idx, a_idx, b_idx, c_inv) = chosen.ok_or(PhaseStabilityError::CollinearGasMatrix {
        g1: g1_name.clone(),
        g2: g2_name.clone(),
    })?;

    let mut candidates = Vec::new();
    let mut seen_formulas = HashMap::new();
    for formula in &input.selected_species {
        let Some(sp) = repo.get_species(formula) else {
            continue;
        };
        if sp.phase != Phase::Solid && !(input.include_liquids && sp.phase == Phase::Liquid) {
            continue;
        }
        if seen_formulas.contains_key(&sp.formula) {
            continue;
        }
        let Ok(parsed) = parse_term(&sp.formula) else {
            continue;
        };
        if !input.include_organics && is_organic(&parsed.elements) {
            continue;
        }
        let Some(counts) = element_vector(&sp.formula, &input.elements) else {
            continue;
        };
        let m_count = counts[m_idx];
        if m_count <= 1e-12 {
            continue;
        }
        let alpha = c_inv[0][0] * counts[a_idx] + c_inv[0][1] * counts[b_idx];
        let beta = c_inv[1][0] * counts[a_idx] + c_inv[1][1] * counts[b_idx];
        seen_formulas.insert(sp.formula.clone(), true);
        candidates.push(CondensedCandidate {
            species: sp,
            elem_counts: counts,
            m_count,
            alpha,
            beta,
        });
    }

    if candidates.is_empty() {
        return Err(PhaseStabilityError::NoCondensedSpecies);
    }

    let mode = if is_pp {
        EvalMode::PP {
            t_k: input.fixed_temperature_k,
            g1: &g1_sp,
            g2: &g2_sp,
        }
    } else {
        EvalMode::TP {
            temp_unit: input.temp_unit,
            x_is_temp,
            g_axis: &g2_sp,
            g_fixed: &g1_sp,
            log_p_fixed: fixed_p_bar.log10(),
        }
    };

    let (x_min, x_max, y_min, y_max) = if is_pp && input.auto_scale {
        auto_scale_pp_bounds(&mode, &candidates)
    } else {
        if input.x_min >= input.x_max || input.y_min >= input.y_max {
            return Err(PhaseStabilityError::InvalidRange);
        }
        (input.x_min, input.x_max, input.y_min, input.y_max)
    };

    let nx = input.nx.max(32);
    let ny = input.ny.max(32);
    let dx = (x_max - x_min) / (nx - 1) as f64;
    let dy = (y_max - y_min) / (ny - 1) as f64;

    let mut raw_grid = vec![0usize; nx * ny];
    for iy in 0..ny {
        let y = y_min + iy as f64 * dy;
        for ix in 0..nx {
            let x = x_min + ix as f64 * dx;
            raw_grid[iy * nx + ix] = eval_winner(x, y, &mode, &candidates);
        }
    }

    // Remap active candidate indices to compact 0..K legend IDs.
    let mut cand_to_legend: HashMap<usize, usize> = HashMap::new();
    let mut legend = Vec::new();
    let mut grid_phase_ids = vec![0usize; nx * ny];
    let mut counts_per_phase: Vec<(usize, f64, f64)> = Vec::new();

    for iy in 0..ny {
        let y = y_min + iy as f64 * dy;
        for ix in 0..nx {
            let x = x_min + ix as f64 * dx;
            let raw = raw_grid[iy * nx + ix];
            let pid = match cand_to_legend.get(&raw) {
                Some(&id) => id,
                None => {
                    let id = legend.len();
                    cand_to_legend.insert(raw, id);
                    legend.push(StabilityLegendEntry {
                        formula: candidates[raw].species.formula.clone(),
                        phase_id: id,
                    });
                    counts_per_phase.push((0, 0.0, 0.0));
                    id
                }
            };
            grid_phase_ids[iy * nx + ix] = pid;
            counts_per_phase[pid].0 += 1;
            counts_per_phase[pid].1 += x;
            counts_per_phase[pid].2 += y;
        }
    }

    // Extract refined boundaries between cells.
    let mut boundaries = Vec::new();
    for iy in 0..ny - 1 {
        for ix in 0..nx - 1 {
            let p00 = raw_grid[iy * nx + ix];
            let p10 = raw_grid[iy * nx + (ix + 1)];
            let p01 = raw_grid[(iy + 1) * nx + ix];
            let p11 = raw_grid[(iy + 1) * nx + (ix + 1)];

            if p00 == p10 && p00 == p01 && p00 == p11 {
                continue;
            }

            let x0 = x_min + ix as f64 * dx;
            let x1 = x0 + dx;
            let y0 = y_min + iy as f64 * dy;
            let y1 = y0 + dy;

            let mut crossings = Vec::with_capacity(4);
            if p00 != p10 {
                crossings.push(bisect_boundary(x0, y0, x1, y0, p00, &mode, &candidates));
            }
            if p10 != p11 {
                crossings.push(bisect_boundary(x1, y0, x1, y1, p10, &mode, &candidates));
            }
            if p01 != p11 {
                crossings.push(bisect_boundary(x0, y1, x1, y1, p01, &mode, &candidates));
            }
            if p00 != p01 {
                crossings.push(bisect_boundary(x0, y0, x0, y1, p00, &mode, &candidates));
            }

            if crossings.len() == 2 {
                boundaries.push(StabilityBoundarySegment {
                    x0: crossings[0].0,
                    y0: crossings[0].1,
                    x1: crossings[1].0,
                    y1: crossings[1].1,
                });
            } else if crossings.len() >= 3 {
                let cx = crossings.iter().map(|p| p.0).sum::<f64>() / crossings.len() as f64;
                let cy = crossings.iter().map(|p| p.1).sum::<f64>() / crossings.len() as f64;
                for pt in crossings {
                    boundaries.push(StabilityBoundarySegment {
                        x0: pt.0,
                        y0: pt.1,
                        x1: cx,
                        y1: cy,
                    });
                }
            }
        }
    }

    let total_cells = (nx * ny) as f64;
    let mut labels = Vec::new();
    for (pid, entry) in legend.iter().enumerate() {
        let (cnt, sum_x, sum_y) = counts_per_phase[pid];
        if cnt == 0 {
            continue;
        }
        let cx = sum_x / cnt as f64;
        let cy = sum_y / cnt as f64;
        let mut best_x = cx;
        let mut best_y = cy;
        let mut best_d2 = f64::INFINITY;
        for iy in 0..ny {
            let y = y_min + iy as f64 * dy;
            for ix in 0..nx {
                if grid_phase_ids[iy * nx + ix] == pid {
                    let x = x_min + ix as f64 * dx;
                    let d2 = (x - cx).powi(2) + (y - cy).powi(2);
                    if d2 < best_d2 {
                        best_d2 = d2;
                        best_x = x;
                        best_y = y;
                    }
                }
            }
        }
        labels.push(StabilityRegionLabel {
            formula: entry.formula.clone(),
            x: best_x,
            y: best_y,
            area_fraction: cnt as f64 / total_cells,
        });
    }

    let ref_t_k = if is_pp {
        input.fixed_temperature_k
    } else {
        input.temp_unit.to_kelvin(0.5 * (x_min + x_max)).max(298.15)
    };

    let mut table_rows = Vec::with_capacity(candidates.len());
    for cand in &candidates {
        let g_abs_j = g_si(&cand.species, ref_t_k).unwrap_or(0.0);
        let g_elem_j = parse_term(&cand.species.formula)
            .ok()
            .and_then(|term| sum_element_references(&term, ref_t_k, false, repo).ok())
            .map(|(_, _, g, _)| g)
            .unwrap_or(0.0);
        let delta_g_kcal = (g_abs_j - g_elem_j) / (1000.0 * CAL_TO_JOULE);
        table_rows.push(PhaseStabilitySpeciesRow {
            formula: cand.species.formula.clone(),
            phase: cand.species.phase,
            elem_counts: cand.elem_counts,
            delta_g_kcal_mol: delta_g_kcal,
        });
    }

    let sys_label = format!(
        "{}-{}-{}",
        input.elements[0], input.elements[1], input.elements[2]
    );
    let (title, footer) = if is_pp {
        let t_disp = match input.temp_unit {
            TemperatureUnit::Celsius => format!("{:.0} °C", input.fixed_temperature_k - 273.15),
            TemperatureUnit::Kelvin => format!("{:.2} K", input.fixed_temperature_k),
        };
        (
            format!("Predominance Diagram for {sys_label} System"),
            Some(format!("Constant value: Temperature = {t_disp}")),
        )
    } else {
        (
            format!("Predominance Diagram for {sys_label} System"),
            Some(format!("Constant value: p{g1_name} = {fixed_p_bar} bar")),
        )
    };

    Ok(PhaseStabilityDiagram {
        title,
        x_label: input.x_axis.label(input.temp_unit),
        y_label: input.y_axis.label(input.temp_unit),
        footer,
        x_min,
        x_max,
        y_min,
        y_max,
        nx,
        ny,
        grid_phase_ids,
        legend,
        boundaries,
        labels,
        table_rows,
        temperature_k: ref_t_k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KelleyCoefficients, SpeciesThermoInterval};

    struct MockRepo {
        species: Vec<ChemicalSpecies>,
    }

    impl MockRepo {
        fn formulas(&self) -> Vec<String> {
            self.species.iter().map(|s| s.formula.clone()).collect()
        }
    }

    impl SpeciesRepository for MockRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.species.iter().find(|s| s.formula == formula).cloned()
        }

        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            match element_symbol {
                "O" => self.get_species("O2(g)"),
                "Cl" => self.get_species("Cl2(g)"),
                _ => self.get_species(element_symbol),
            }
        }
    }

    fn const_species(
        formula: &str,
        phase: Phase,
        h298_kcal: f64,
        s298_cal: f64,
    ) -> ChemicalSpecies {
        ChemicalSpecies {
            formula: formula.to_string(),
            name: formula.to_string(),
            phase,
            molecular_weight: 1.0,
            density_g_cm3: 1.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 298.15,
                t_max: 3000.0,
                phase,
                kelley: KelleyCoefficients {
                    a: 0.0,
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

    fn cu_s_o_repo() -> MockRepo {
        MockRepo {
            species: vec![
                const_species("O2(g)", Phase::Gas, 0.0, 49.003),
                const_species("SO2(g)", Phase::Gas, -70.944, 59.30),
                const_species("Cu", Phase::Solid, 0.0, 7.923),
                const_species("Cu2O", Phase::Solid, -40.80, 22.08),
                const_species("CuO", Phase::Solid, -37.30, 10.19),
                const_species("Cu2S", Phase::Solid, -19.00, 28.90),
                const_species("CuSO4", Phase::Solid, -184.37, 26.10),
                const_species("CuO*CuSO4", Phase::Solid, -220.20, 37.60),
            ],
        }
    }

    fn ti_cl_o_repo() -> MockRepo {
        MockRepo {
            species: vec![
                const_species("Cl2(g)", Phase::Gas, 0.0, 53.29),
                const_species("O2(g)", Phase::Gas, 0.0, 49.003),
                const_species("Ti", Phase::Solid, 0.0, 7.34),
                const_species("TiCl2", Phase::Solid, -123.0, 20.88),
                const_species("TiCl3", Phase::Solid, -172.5, 33.4),
                const_species("TiO", Phase::Solid, -129.7, 8.31),
                const_species("TiO2", Phase::Solid, -225.8, 12.04),
                const_species("TiClO", Phase::Solid, -182.0, 18.0),
            ],
        }
    }

    #[test]
    fn cu_s_o_tp_diagram_shows_cuso4_and_cuo() {
        let repo = cu_s_o_repo();
        let input = PhaseStabilityInput {
            elements: ["Cu".into(), "S".into(), "O".into()],
            selected_species: repo.formulas(),
            x_axis: AxisVariable::Temperature,
            y_axis: AxisVariable::Gas("SO2(g)".into()),
            temp_unit: TemperatureUnit::Celsius,
            x_min: 0.0,
            x_max: 1000.0,
            y_min: -5.0,
            y_max: 2.0,
            auto_scale: false,
            fixed_temperature_k: 298.15,
            fixed_gas: Some(("O2(g)".into(), 0.1)),
            include_liquids: false,
            include_organics: false,
            nx: 60,
            ny: 60,
        };

        let diagram = calculate_phase_stability(&repo, &input).expect("Cu-S-O T-p should succeed");
        let phases: Vec<&str> = diagram.legend.iter().map(|e| e.formula.as_str()).collect();
        assert!(phases.contains(&"CuSO4"), "Expected CuSO4 in {phases:?}");
        assert!(phases.contains(&"CuO"), "Expected CuO in {phases:?}");
        assert!(!diagram.boundaries.is_empty());
        assert!(diagram
            .footer
            .as_deref()
            .unwrap_or("")
            .contains("pO2(g) = 0.1 bar"));
    }

    #[test]
    fn ti_cl_o_pp_diagram_at_300c_autoscale() {
        let repo = ti_cl_o_repo();
        let input = PhaseStabilityInput {
            elements: ["Ti".into(), "Cl".into(), "O".into()],
            selected_species: repo.formulas(),
            x_axis: AxisVariable::Gas("Cl2(g)".into()),
            y_axis: AxisVariable::Gas("O2(g)".into()),
            temp_unit: TemperatureUnit::Celsius,
            x_min: -45.0,
            x_max: -5.0,
            y_min: -100.0,
            y_max: -20.0,
            auto_scale: true,
            fixed_temperature_k: 573.15,
            fixed_gas: None,
            include_liquids: false,
            include_organics: false,
            nx: 60,
            ny: 60,
        };

        let diagram = calculate_phase_stability(&repo, &input).expect("Ti-Cl-O p-p should succeed");
        let phases: Vec<&str> = diagram.legend.iter().map(|e| e.formula.as_str()).collect();
        assert!(phases.contains(&"Ti"));
        assert!(phases.contains(&"TiO2"));
        assert!(phases.contains(&"TiCl2") || phases.contains(&"TiCl3"));
        assert!(!diagram.table_rows.is_empty());
    }

    #[test]
    fn rejects_collinear_or_metal_containing_gases() {
        let mut repo = cu_s_o_repo();
        repo.species
            .push(const_species("O(g)", Phase::Gas, 59.55, 38.47));

        let input = PhaseStabilityInput {
            elements: ["Cu".into(), "S".into(), "O".into()],
            selected_species: repo.formulas(),
            x_axis: AxisVariable::Gas("O2(g)".into()),
            y_axis: AxisVariable::Gas("O(g)".into()),
            temp_unit: TemperatureUnit::Celsius,
            x_min: -20.0,
            x_max: 5.0,
            y_min: -20.0,
            y_max: 5.0,
            auto_scale: false,
            fixed_temperature_k: 573.15,
            fixed_gas: None,
            include_liquids: false,
            include_organics: false,
            nx: 32,
            ny: 32,
        };

        let err = calculate_phase_stability(&repo, &input).unwrap_err();
        assert!(matches!(err, PhaseStabilityError::CollinearGasMatrix { .. }));
    }
}
