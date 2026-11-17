//! Diagram generation: spatial temperature profiles T(x), parametric sweeps,
//! and optimum insulation thickness economic analysis (Figs. 3, 12, 13, 14).

use super::model::HeatLossWorkbook;
use super::solver::solve_workbook;
use super::target::{apply_variable, TargetVariable};
use serde::{Deserialize, Serialize};
use soryntel_core::SoryntelError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureProfileSeries {
    pub sheet_name: String,
    /// Pairs `(cumulative_thickness_m, temperature_k)`.
    pub points: Vec<(f64, f64)>,
}

/// Extract continuous spatial temperature profiles for each calculated sheet (Fig. 3).
pub fn generate_temperature_profile(
    wb: &HeatLossWorkbook,
) -> Result<(f64, Vec<TemperatureProfileSeries>), SoryntelError> {
    let res = solve_workbook(wb)?;
    let mut series = Vec::with_capacity(res.sheet_results.len());
    for sheet in res.sheet_results {
        let mut pts = Vec::new();
        for col in sheet.columns {
            for p in col.profile_points {
                if pts.last().map(|last: &(f64, f64)| (last.0 - p.0).abs() > 1e-9 || (last.1 - p.1).abs() > 1e-6).unwrap_or(true) {
                    pts.push(p);
                }
            }
        }
        series.push(TemperatureProfileSeries {
            sheet_name: sheet.sheet_name,
            points: pts,
        });
    }
    Ok((res.total_heat_flow_w, series))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SweepParameter {
    LeftBoundaryTempK,
    RightBoundaryTempK,
    LayerThickness(usize),
    Width,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepPoint {
    pub x: f64,
    pub total_heat_flow_w: f64,
    pub active_sheet_heat_flow_w: f64,
    pub outer_surface_temp_k: f64,
}

/// Generate a 1D parametric sweep diagram (Fig. 12 & Fig. 13).
pub fn generate_parameter_sweep(
    base: &HeatLossWorkbook,
    sheet_index: usize,
    param: SweepParameter,
    min_x: f64,
    max_x: f64,
    steps: usize,
) -> Result<Vec<SweepPoint>, SoryntelError> {
    if steps == 0 || max_x <= min_x {
        return Err(SoryntelError::HeatLossError(
            "Invalid diagram sweep range or step count".into(),
        ));
    }
    let var = match param {
        SweepParameter::LeftBoundaryTempK => TargetVariable::LeftBoundaryTempK,
        SweepParameter::RightBoundaryTempK => TargetVariable::RightBoundaryTempK,
        SweepParameter::LayerThickness(c) => TargetVariable::LayerThickness(c),
        SweepParameter::Width => TargetVariable::Width,
    };

    let mut out = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let x = min_x + (max_x - min_x) * (i as f64) / (steps as f64);
        let mut wb = base.clone();
        apply_variable(&mut wb, sheet_index, var, x);
        let res = solve_workbook(&wb)?;
        let active = res
            .sheet_results
            .get(sheet_index)
            .or_else(|| res.sheet_results.first())
            .ok_or_else(|| SoryntelError::HeatLossError("Missing sheet result".into()))?;
        let outer_t = active
            .columns
            .last()
            .map(|c| c.t_left_k)
            .unwrap_or(0.0);
        out.push(SweepPoint {
            x,
            total_heat_flow_w: res.total_heat_flow_w,
            active_sheet_heat_flow_w: active.heat_flow_w,
            outer_surface_temp_k: outer_t,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OptimumInsulationSpec {
    pub sheet_index: usize,
    pub insulation_col_index: usize,
    pub min_thickness_m: f64,
    pub max_thickness_m: f64,
    pub steps: usize,
    /// Annual operating hours (default 8760 h/yr).
    pub operating_hours_per_year: f64,
    /// Energy cost in currency/kWh.
    pub energy_cost_per_kwh: f64,
    /// Installed insulation cost in currency/m^3.
    pub insulation_cost_per_m3: f64,
    /// Capital amortization period in years.
    pub amortization_years: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OptimumInsulationPoint {
    pub thickness_m: f64,
    pub heat_loss_w: f64,
    pub annual_energy_cost: f64,
    pub annual_insulation_cost: f64,
    pub total_annual_cost: f64,
}

/// Economic optimum insulation thickness curve (Fig. 14).
pub fn generate_optimum_insulation_diagram(
    base: &HeatLossWorkbook,
    spec: &OptimumInsulationSpec,
) -> Result<(Vec<OptimumInsulationPoint>, OptimumInsulationPoint), SoryntelError> {
    if spec.steps == 0 || spec.max_thickness_m <= spec.min_thickness_m {
        return Err(SoryntelError::HeatLossError(
            "Invalid thickness range for optimum insulation diagram".into(),
        ));
    }

    let mut points = Vec::with_capacity(spec.steps + 1);
    let mut best: Option<OptimumInsulationPoint> = None;

    for i in 0..=spec.steps {
        let thick = spec.min_thickness_m
            + (spec.max_thickness_m - spec.min_thickness_m) * (i as f64) / (spec.steps as f64);
        let mut wb = base.clone();
        apply_variable(
            &mut wb,
            spec.sheet_index,
            TargetVariable::LayerThickness(spec.insulation_col_index),
            thick,
        );
        let res = solve_workbook(&wb)?;
        let active_sheet = &wb.sheets[spec.sheet_index];
        let mut d_before = 0.0;
        for col in active_sheet.columns.iter().take(spec.insulation_col_index) {
            d_before += col.thickness_m;
        }
        let area_ins = active_sheet.geometry.surface_area_at_thickness(d_before)?;
        let ins_volume_m3 = area_ins * thick;

        let annual_energy_cost =
            (res.total_heat_flow_w / 1000.0) * spec.operating_hours_per_year * spec.energy_cost_per_kwh;
        let annual_insulation_cost =
            (ins_volume_m3 * spec.insulation_cost_per_m3) / spec.amortization_years.max(1.0);
        let total = annual_energy_cost + annual_insulation_cost;

        let pt = OptimumInsulationPoint {
            thickness_m: thick,
            heat_loss_w: res.total_heat_flow_w,
            annual_energy_cost,
            annual_insulation_cost,
            total_annual_cost: total,
        };
        if best.map(|b| pt.total_annual_cost < b.total_annual_cost).unwrap_or(true) {
            best = Some(pt);
        }
        points.push(pt);
    }

    Ok((points, best.unwrap()))
}
