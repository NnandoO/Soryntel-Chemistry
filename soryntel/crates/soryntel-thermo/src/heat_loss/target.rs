//! Target Dialog 1D root finder (Brent's method) for Module 3.

use super::model::{Geometry, HeatLossWorkbook};
use super::solver::solve_workbook;
use serde::{Deserialize, Serialize};
use soryntel_core::SoryntelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetVariable {
    Width,
    Length,
    Height,
    Diameter,
    LayerThickness(usize),
    LeftBoundaryTempK,
    RightBoundaryTempK,
}

impl TargetVariable {
    pub fn label(self) -> &'static str {
        match self {
            Self::Width => "Width (m)",
            Self::Length => "Length (m)",
            Self::Height => "Height (m)",
            Self::Diameter => "Diameter (m)",
            Self::LayerThickness(_) => "Layer Thickness (m)",
            Self::LeftBoundaryTempK => "Left Boundary Temp (K)",
            Self::RightBoundaryTempK => "Right Boundary Temp (K)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TargetGoal {
    TotalHeatFlowW(f64),
    ActiveSheetHeatFlowW(f64),
    OuterSurfaceTempK(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    pub sheet_index: usize,
    pub variable: TargetVariable,
    pub goal: TargetGoal,
    pub min_val: f64,
    pub max_val: f64,
    pub max_iterations: usize,
    pub tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetResult {
    pub solution_value: f64,
    pub achieved_goal_value: f64,
    pub iterations: usize,
}

pub fn apply_variable(wb: &mut HeatLossWorkbook, sheet_idx: usize, var: TargetVariable, val: f64) {
    let sheet = &mut wb.sheets[sheet_idx];
    match var {
        TargetVariable::Width => match &mut sheet.geometry {
            Geometry::Wall { width_m, .. } | Geometry::Cube { width_m, .. } => *width_m = val,
            _ => {}
        },
        TargetVariable::Length => match &mut sheet.geometry {
            Geometry::Wall { length_m, .. } | Geometry::Cube { length_m, .. } => *length_m = val,
            _ => {}
        },
        TargetVariable::Height => match &mut sheet.geometry {
            Geometry::Cube { height_m, .. } | Geometry::Cylinder { height_m, .. } => {
                *height_m = val
            }
            _ => {}
        },
        TargetVariable::Diameter => match &mut sheet.geometry {
            Geometry::Cylinder { diameter_m, .. } | Geometry::Sphere { diameter_m } => {
                *diameter_m = val
            }
            _ => {}
        },
        TargetVariable::LayerThickness(col_idx) => {
            if let Some(col) = sheet.columns.get_mut(col_idx) {
                col.thickness_m = val;
            }
        }
        TargetVariable::LeftBoundaryTempK => {
            if let Some(col) = sheet.columns.first_mut() {
                col.t_left_k = val;
            }
        }
        TargetVariable::RightBoundaryTempK => {
            if let Some(col) = sheet.columns.last_mut() {
                col.t_right_k = val;
            }
        }
    }
}

fn evaluate_objective(
    base: &HeatLossWorkbook,
    spec: &TargetSpec,
    x: f64,
) -> Result<(f64, f64), SoryntelError> {
    let mut wb = base.clone();
    apply_variable(&mut wb, spec.sheet_index, spec.variable, x);
    let res = solve_workbook(&wb)?;
    let (actual, target) = match spec.goal {
        TargetGoal::TotalHeatFlowW(t) => (res.total_heat_flow_w, t),
        TargetGoal::ActiveSheetHeatFlowW(t) => {
            let s = res
                .sheet_results
                .get(spec.sheet_index)
                .or_else(|| res.sheet_results.first())
                .ok_or_else(|| SoryntelError::HeatLossError("Missing sheet result".into()))?;
            (s.heat_flow_w, t)
        }
        TargetGoal::OuterSurfaceTempK(t) => {
            let s = res
                .sheet_results
                .get(spec.sheet_index)
                .or_else(|| res.sheet_results.first())
                .ok_or_else(|| SoryntelError::HeatLossError("Missing sheet result".into()))?;
            let outer_t = s
                .columns
                .last()
                .map(|c| c.t_left_k)
                .unwrap_or(0.0);
            (outer_t, t)
        }
    };
    Ok((actual - target, actual))
}

/// Solve 1D target equation using Brent's root-finding method within `[min_val, max_val]`.
pub fn solve_target(
    base: &HeatLossWorkbook,
    spec: &TargetSpec,
) -> Result<TargetResult, SoryntelError> {
    if spec.sheet_index >= base.sheets.len() {
        return Err(SoryntelError::HeatLossError(
            "Target sheet index out of bounds".into(),
        ));
    }
    let mut a = spec.min_val;
    let mut b = spec.max_val;
    if a >= b {
        return Err(SoryntelError::TargetNotBracketed(
            "Min value must be < Max value".into(),
        ));
    }

    let (mut fa, actual_a) = evaluate_objective(base, spec, a)?;
    if fa.abs() <= spec.tolerance {
        return Ok(TargetResult {
            solution_value: a,
            achieved_goal_value: actual_a,
            iterations: 0,
        });
    }
    let (mut fb, actual_b) = evaluate_objective(base, spec, b)?;
    if fb.abs() <= spec.tolerance {
        return Ok(TargetResult {
            solution_value: b,
            achieved_goal_value: actual_b,
            iterations: 0,
        });
    }

    if fa * fb > 0.0 {
        return Err(SoryntelError::TargetNotBracketed(
            "Iteration did not converge, check Variable min/max bounds.".into(),
        ));
    }

    let mut c = a;
    let mut fc = fa;
    let mut d = b - a;
    let mut e = d;
    let max_iter = spec.max_iterations.max(20);

    for iter in 1..=max_iter {
        if fb * fc > 0.0 {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }

        let tol1 = 2.0 * f64::EPSILON * b.abs() + 0.5 * spec.tolerance.max(1e-12);
        let xm = 0.5 * (c - b);
        if xm.abs() <= tol1 || fb.abs() <= spec.tolerance {
            let (_, actual) = evaluate_objective(base, spec, b)?;
            return Ok(TargetResult {
                solution_value: b,
                achieved_goal_value: actual,
                iterations: iter,
            });
        }

        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (p, q) = if (a - c).abs() < 1e-15 {
                (2.0 * xm * s, 1.0 - s)
            } else {
                let q1 = fa / fc;
                let r1 = fb / fc;
                (
                    s * (2.0 * xm * q1 * (q1 - r1) - (b - a) * (r1 - 1.0)),
                    (q1 - 1.0) * (r1 - 1.0) * (s - 1.0),
                )
            };
            let (p, q) = if p > 0.0 { (p, -q) } else { (-p, q) };
            let s_val = e;
            e = d;
            if 2.0 * p < (3.0 * xm * q - (tol1 * q).abs()).min((s_val * q).abs()) {
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }

        a = b;
        fa = fb;
        if d.abs() > tol1 {
            b += d;
        } else {
            b += if xm >= 0.0 { tol1 } else { -tol1 };
        }
        let (fb_new, _) = evaluate_objective(base, spec, b)?;
        fb = fb_new;
    }

    let (_, actual) = evaluate_objective(base, spec, b)?;
    Ok(TargetResult {
        solution_value: b,
        achieved_goal_value: actual,
        iterations: max_iter,
    })
}
