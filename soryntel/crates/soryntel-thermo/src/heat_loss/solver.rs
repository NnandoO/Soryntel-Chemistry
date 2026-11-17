//! Coupled 1D steady-state thermal network solver (`HeatFlow` and `TemperatureProfile`)
//! and workbook aggregator + Module 2 (Heat & Material Balance) energy export.

use super::conduction::evaluate_solid_layer;
use super::convection::{evaluate_convection, evaluate_hc_function};
use super::model::{
    CalculationMode, ColumnResult, ColumnType, HeatLossColumn, HeatLossSheet, HeatLossWorkbook,
    SheetResult, WorkbookResult,
};
use super::radiation::{enclosure_radiation_hr, surface_radiation_hr, validate_emissivity};
use soryntel_core::{BalanceEnergyUnit, SoryntelError, CAL_TO_JOULE};

const MAX_OUTER_ITER: usize = 120;
const TEMP_TOL_K: f64 = 1e-6;

/// Evaluate a single column's thermal resistance, effective coefficients, and internal profile
/// given its left and right edge temperatures `t_left_k` and `t_right_k`.
fn evaluate_column_resistance(
    sheet: &HeatLossSheet,
    col: &HeatLossColumn,
    d_in_m: f64,
    t_left_k: f64,
    t_right_k: f64,
) -> Result<(f64, f64, f64, f64, f64, f64, Vec<(f64, f64)>), SoryntelError> {
    let area = sheet.geometry.surface_area_at_thickness(d_in_m)?;
    match col.column_type {
        ColumnType::SolidLayer => {
            let cond = evaluate_solid_layer(
                &sheet.geometry,
                d_in_m,
                col.thickness_m,
                col.calculation_grid,
                t_left_k,
                t_right_k,
                col.k_fixed,
                col.k_w_m_k,
                col.solid_data.as_ref(),
            )?;
            Ok((
                cond.resistance_k_w,
                cond.k_avg_w_m_k,
                0.0,
                0.0,
                area,
                d_in_m + col.thickness_m,
                cond.profile_points,
            ))
        }
        ColumnType::Surface => {
            let hc = if col.hc_fixed {
                col.hc_w_m2_k
            } else if !col.custom_hc_terms.is_empty() {
                evaluate_hc_function(&col.custom_hc_terms, col.fluid_speed_m_s)
            } else if let Some(fluid) = &col.fluid_data {
                let l = col
                    .characteristic_length_override_m
                    .unwrap_or_else(|| sheet.geometry.characteristic_length(d_in_m));
                // Identify which side is wall surface vs bulk ambient
                // If left < right or left > right: for internal film (col 0), right edge is wall; for external film, left edge is wall.
                let diag = evaluate_convection(
                    fluid,
                    col.convection_regime,
                    col.convection_geometry,
                    t_left_k,
                    t_right_k,
                    l,
                    None,
                    col.fluid_speed_m_s,
                    None,
                )?;
                diag.hc_w_m2_k
            } else {
                col.hc_w_m2_k
            };

            let hr = if col.hr_fixed {
                col.hr_w_m2_k
            } else if !col.iterative_gas_eps_table.is_empty() || col.gas_emissivity.is_some() {
                // Internal gas or gas+particle radiation (Ch. 12.1 step 4, Eqs. 45 & 53)
                let eps_s = validate_emissivity(col.emissivity)?;
                let (t_g, t_s) = if t_left_k >= t_right_k {
                    (t_left_k, t_right_k)
                } else {
                    (t_right_k, t_left_k)
                };
                let eps_g = if !col.iterative_gas_eps_table.is_empty() {
                    super::model::interpolate_sorted_table(
                        &col.iterative_gas_eps_table,
                        t_g,
                        "Iterative gas emissivity",
                    )?
                } else {
                    col.gas_emissivity.unwrap_or(0.0)
                };
                let alpha_g = if !col.iterative_gas_eps_table.is_empty() {
                    let e_ts = super::model::interpolate_sorted_table(
                        &col.iterative_gas_eps_table,
                        t_s,
                        "Iterative gas absorptivity",
                    )?;
                    (e_ts * (t_g / t_s.max(1.0)).sqrt()).clamp(0.0, 0.999)
                } else {
                    col.gas_absorptivity.unwrap_or(eps_g)
                };
                let dt = (t_g - t_s).abs();
                if dt < 1e-6 {
                    0.0
                } else if let Some(eps_c) = col.particle_cloud_emissivity {
                    let eps_gc = eps_g + eps_c - eps_g * eps_c;
                    let alpha_gc = alpha_g + eps_c - alpha_g * eps_c;
                    let denom = (alpha_gc + eps_s - alpha_gc * eps_s).max(1e-12);
                    let q_flux = (eps_s / denom)
                        * soryntel_core::STEFAN_BOLTZMANN
                        * (eps_gc * t_g.powi(4) - alpha_gc * t_s.powi(4));
                    (q_flux / dt).max(0.0)
                } else {
                    let q_flux = 0.5
                        * (eps_s + 1.0)
                        * soryntel_core::STEFAN_BOLTZMANN
                        * (eps_g * t_g.powi(4) - alpha_g * t_s.powi(4));
                    (q_flux / dt).max(0.0)
                }
            } else {
                validate_emissivity(col.emissivity)?;
                surface_radiation_hr(col.emissivity, t_left_k, t_right_k)?
            };

            let h_total = (hc + hr).max(1e-12);
            let r = 1.0 / (h_total * area);
            Ok((
                r,
                0.0,
                hc,
                hr,
                area,
                d_in_m,
                vec![(d_in_m, t_left_k), (d_in_m, t_right_k)],
            ))
        }
        ColumnType::Enclosure => {
            if col.thickness_m <= 0.0 {
                return Err(SoryntelError::HeatLossError(
                    "Enclosure thickness must be > 0".into(),
                ));
            }
            let hc = if col.hc_fixed {
                col.hc_w_m2_k
            } else if let Some(fluid) = &col.fluid_data {
                let h_enc = sheet.geometry.characteristic_length(d_in_m);
                let diag = evaluate_convection(
                    fluid,
                    col.convection_regime,
                    col.convection_geometry,
                    t_left_k,
                    t_right_k,
                    col.thickness_m,
                    Some(h_enc),
                    0.0,
                    None,
                )?;
                diag.hc_w_m2_k
            } else {
                col.hc_w_m2_k
            };

            let hr = if col.hr_fixed {
                col.hr_w_m2_k
            } else {
                enclosure_radiation_hr(
                    &sheet.geometry,
                    d_in_m,
                    d_in_m + col.thickness_m,
                    col.emissivity,
                    col.emissivity_opposite,
                    t_left_k,
                    t_right_k,
                )?
            };

            let a_mean = {
                let a_out = sheet
                    .geometry
                    .surface_area_at_thickness(d_in_m + col.thickness_m)?;
                0.5 * (area + a_out)
            };
            let h_total = (hc + hr).max(1e-12);
            let r = 1.0 / (h_total * a_mean);
            Ok((
                r,
                0.0,
                hc,
                hr,
                area,
                d_in_m + col.thickness_m,
                vec![
                    (d_in_m, t_left_k),
                    (d_in_m + col.thickness_m, t_right_k),
                ],
            ))
        }
        ColumnType::LayerContact => {
            let r = col.contact_resistance_k_w.max(1e-12);
            Ok((
                r,
                0.0,
                0.0,
                0.0,
                area,
                d_in_m,
                vec![(d_in_m, t_left_k), (d_in_m, t_right_k)],
            ))
        }
    }
}

/// Solve a single `HeatLossSheet` in either `HeatFlow` or `TemperatureProfile` mode.
pub fn solve_sheet(sheet: &HeatLossSheet) -> Result<SheetResult, SoryntelError> {
    let n = sheet.columns.len();
    if n == 0 {
        return Err(SoryntelError::HeatLossError(
            "Sheet must contain at least one layer/surface column".into(),
        ));
    }

    // Cumulative thickness before each column
    let mut d_in = Vec::with_capacity(n);
    let mut acc = 0.0;
    for col in &sheet.columns {
        d_in.push(acc);
        if matches!(col.column_type, ColumnType::SolidLayer | ColumnType::Enclosure) {
            acc += col.thickness_m.max(0.0);
        }
    }

    match sheet.mode {
        CalculationMode::HeatFlow => solve_heat_flow_mode(sheet, &d_in),
        CalculationMode::TemperatureProfile => solve_temperature_profile_mode(sheet, &d_in),
    }
}

fn solve_heat_flow_mode(sheet: &HeatLossSheet, d_in: &[f64]) -> Result<SheetResult, SoryntelError> {
    let n = sheet.columns.len();
    // Node temperatures T[0..=n]. Collect fixed node indices.
    let mut fixed_nodes: Vec<(usize, f64)> = Vec::new();
    if sheet.columns[0].t_left_fixed {
        fixed_nodes.push((0, sheet.columns[0].t_left_k));
    }
    for i in 0..n {
        if sheet.columns[i].t_right_fixed {
            fixed_nodes.push((i + 1, sheet.columns[i].t_right_k));
        } else if i + 1 < n && sheet.columns[i + 1].t_left_fixed {
            fixed_nodes.push((i + 1, sheet.columns[i + 1].t_left_k));
        }
    }

    // Fallback: if user didn't explicitly toggle Fix flags on ends, use boundary T_left(0) & T_right(n-1)
    let (idx_a, t_a, idx_b, t_b) = if fixed_nodes.len() >= 2 {
        let first = fixed_nodes[0];
        let last = fixed_nodes[fixed_nodes.len() - 1];
        (first.0, first.1, last.0, last.1)
    } else {
        (0, sheet.columns[0].t_left_k, n, sheet.columns[n - 1].t_right_k)
    };

    if idx_a == idx_b {
        return Err(SoryntelError::HeatLossError(
            "Heat flow mode requires two distinct fixed temperature nodes".into(),
        ));
    }

    // Initialize node temperatures linearly between (idx_a, t_a) and (idx_b, t_b)
    let mut temps = vec![0.0_f64; n + 1];
    let slope = (t_b - t_a) / ((idx_b - idx_a) as f64);
    for k in 0..=n {
        temps[k] = (t_a + slope * ((k as isize - idx_a as isize) as f64)).max(1.0);
    }

    let mut converged = false;
    let mut iters = 0;
    let mut q_w = 0.0;
    let mut col_cache = Vec::with_capacity(n);

    for iter in 1..=MAX_OUTER_ITER {
        iters = iter;
        col_cache.clear();
        for i in 0..n {
            let res = evaluate_column_resistance(
                sheet,
                &sheet.columns[i],
                d_in[i],
                temps[i],
                temps[i + 1],
            )?;
            col_cache.push(res);
        }

        let r_between: f64 = col_cache[idx_a..idx_b].iter().map(|c| c.0).sum();
        if r_between <= 1e-15 {
            return Err(SoryntelError::HeatLossError(
                "Total thermal resistance between fixed nodes is zero".into(),
            ));
        }
        q_w = (t_a - t_b) / r_between;

        // Compute new node temperatures propagating outward from idx_a
        let mut new_temps = temps.clone();
        new_temps[idx_a] = t_a;
        for i in idx_a..n {
            new_temps[i + 1] = (new_temps[i] - q_w * col_cache[i].0).max(1.0);
        }
        for i in (0..idx_a).rev() {
            new_temps[i] = (new_temps[i + 1] + q_w * col_cache[i].0).max(1.0);
        }
        new_temps[idx_b] = t_b;

        let max_diff = temps
            .iter()
            .zip(new_temps.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);

        // Adaptive damping for T^4 radiation & convection regime transitions
        let omega = if iter < 8 { 0.6 } else { 0.5 };
        for k in 0..=n {
            if k != idx_a && k != idx_b {
                temps[k] = omega * new_temps[k] + (1.0 - omega) * temps[k];
            }
        }

        if max_diff < TEMP_TOL_K {
            converged = true;
            break;
        }
    }

    build_sheet_result(sheet, d_in, &temps, q_w, iters, converged)
}

fn solve_temperature_profile_mode(
    sheet: &HeatLossSheet,
    d_in: &[f64],
) -> Result<SheetResult, SoryntelError> {
    let n = sheet.columns.len();
    let q_w = sheet.fixed_heat_flow_w;

    // Locate the fixed temperature node (default node 0 if none marked)
    let mut anchor = (0usize, sheet.columns[0].t_left_k);
    if sheet.columns[0].t_left_fixed {
        anchor = (0, sheet.columns[0].t_left_k);
    } else {
        for i in 0..n {
            if sheet.columns[i].t_right_fixed {
                anchor = (i + 1, sheet.columns[i].t_right_k);
                break;
            } else if i + 1 < n && sheet.columns[i + 1].t_left_fixed {
                anchor = (i + 1, sheet.columns[i + 1].t_left_k);
                break;
            }
        }
    }

    let mut temps = vec![anchor.1; n + 1];
    let mut total_iters = 0;

    // Forward propagation from anchor -> n
    for i in anchor.0..n {
        let t_l = temps[i];
        let mut t_r = (t_l - 10.0).max(1.0);
        for _ in 0..60 {
            total_iters += 1;
            let (r, _, _, _, _, _, _) =
                evaluate_column_resistance(sheet, &sheet.columns[i], d_in[i], t_l, t_r)?;
            let t_r_new = (t_l - q_w * r).max(1.0);
            let diff = (t_r_new - t_r).abs();
            t_r = 0.6 * t_r_new + 0.4 * t_r;
            if diff < TEMP_TOL_K {
                break;
            }
        }
        temps[i + 1] = t_r;
    }

    // Backward propagation from anchor -> 0
    for i in (0..anchor.0).rev() {
        let t_r = temps[i + 1];
        let mut t_l = (t_r + 10.0).max(1.0);
        for _ in 0..60 {
            total_iters += 1;
            let (r, _, _, _, _, _, _) =
                evaluate_column_resistance(sheet, &sheet.columns[i], d_in[i], t_l, t_r)?;
            let t_l_new = (t_r + q_w * r).max(1.0);
            let diff = (t_l_new - t_l).abs();
            t_l = 0.6 * t_l_new + 0.4 * t_l;
            if diff < TEMP_TOL_K {
                break;
            }
        }
        temps[i] = t_l;
    }

    build_sheet_result(sheet, d_in, &temps, q_w, total_iters, true)
}

fn build_sheet_result(
    sheet: &HeatLossSheet,
    d_in: &[f64],
    temps: &[f64],
    q_w: f64,
    iterations: usize,
    converged: bool,
) -> Result<SheetResult, SoryntelError> {
    let n = sheet.columns.len();
    let mut columns = Vec::with_capacity(n);
    let mut warnings = Vec::new();
    if !converged {
        warnings.push("Iteration did not converge, returning results from last iteration.".into());
    }

    for i in 0..n {
        let col = &sheet.columns[i];
        let (r, k_avg, hc, hr, area, cum_thick, profile) = evaluate_column_resistance(
            sheet,
            col,
            d_in[i],
            temps[i],
            temps[i + 1],
        )?;
        // Check rows 92-93 material temperature limits (Ch. 12.3.2 item 4)
        let db_range = col
            .solid_data
            .as_ref()
            .and_then(|s| s.t_range_k())
            .or_else(|| {
                col.fluid_data.as_ref().and_then(|f| {
                    Some((f.points.first()?.t_k, f.points.last()?.t_k))
                })
            });
        let t_min_lim = col.t_min_k.or_else(|| db_range.map(|(lo, _)| lo));
        let t_max_lim = col.t_max_k.or_else(|| db_range.map(|(_, hi)| hi));
        let t_lo = temps[i].min(temps[i + 1]);
        let t_hi = temps[i].max(temps[i + 1]);
        if let Some(min_k) = t_min_lim {
            if t_lo < min_k - 0.5 {
                warnings.push(format!(
                    "Layer {} ({}) temperature {:.1} K is below Tmin ({:.1} K).",
                    i + 1,
                    col.layer_material,
                    t_lo,
                    min_k
                ));
            }
        }
        if let Some(max_k) = t_max_lim {
            if t_hi > max_k + 0.5 {
                warnings.push(format!(
                    "Layer {} ({}) temperature {:.1} K exceeds Tmax ({:.1} K).",
                    i + 1,
                    col.layer_material,
                    t_hi,
                    max_k
                ));
            }
        }
        columns.push(ColumnResult {
            column_index: i,
            t_left_k: temps[i],
            t_right_k: temps[i + 1],
            k_avg_w_m_k: k_avg,
            hc_w_m2_k: hc,
            hr_w_m2_k: hr,
            thermal_resistance_k_w: r,
            heat_flow_w: q_w,
            heat_flux_w_m2: q_w / area.max(1e-12),
            surface_area_m2: area,
            cumulative_thickness_m: cum_thick,
            profile_points: profile,
        });
    }

    Ok(SheetResult {
        sheet_name: sheet.name.clone(),
        heat_flow_w: q_w,
        iterations,
        converged,
        columns,
        warnings,
    })
}

/// Solve either the active sheet or all sheets in a `HeatLossWorkbook`.
pub fn solve_workbook(wb: &HeatLossWorkbook) -> Result<WorkbookResult, SoryntelError> {
    if wb.sheets.is_empty() {
        return Err(SoryntelError::HeatLossError(
            "Workbook contains no sheets".into(),
        ));
    }

    let mut sheet_results = Vec::new();
    if wb.calculate_all_sheets {
        for sheet in &wb.sheets {
            sheet_results.push(solve_sheet(sheet)?);
        }
    } else {
        let idx = wb.active_sheet.min(wb.sheets.len() - 1);
        sheet_results.push(solve_sheet(&wb.sheets[idx])?);
    }

    let total_heat_flow_w: f64 = sheet_results.iter().map(|s| s.heat_flow_w).sum();

    Ok(WorkbookResult {
        total_heat_flow_w,
        sheet_results,
    })
}

/// Convert a steady-state heat loss in Watts (J/s) to Module 2 `BalanceEnergyUnit` per hour
/// (negative sign by convention for heat losses leaving the balance area, or caller adjusts sign).
pub fn heat_flow_w_to_balance_energy_per_hour(q_w: f64, unit: BalanceEnergyUnit) -> f64 {
    let mj_per_h = q_w * 3600.0 * 1e-6;
    match unit {
        BalanceEnergyUnit::MJ => mj_per_h,
        BalanceEnergyUnit::Mcal => mj_per_h / CAL_TO_JOULE,
        BalanceEnergyUnit::Kwh => mj_per_h / 3.6,
    }
}
