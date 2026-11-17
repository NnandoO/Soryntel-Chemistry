//! Outer solvers for Module 4:
//! - Parameter sweeps (0, 1, 2 independent variables; linear & log steps)
//! - Adiabatic equilibrium (`N,H,P`) via Brent/bisection over [273.15 K, 20000 K]
//! - Constant volume (`N,P,V`)
//! - Target Calculations (`TargetSpec`)
//! - Cp extrapolation `Warnings` collector

use crate::equilibrium::gem_solver::solve_single_point;
use crate::equilibrium::model::{
    AxisRange, EquilibriumParams, EquilibriumPointResult, EquilibriumSweepResult,
    EquilibriumSystem, ExtrapolationWarning, StepMode, SweepVariable, TargetResultSummary,
    ThermodynamicMode,
};
use crate::SpeciesRepository;
use soryntel_core::SoryntelError;

/// Generates `steps` values along `AxisRange` (linear or logarithmic).
pub fn generate_axis_values(range: AxisRange, count: usize) -> Vec<f64> {
    let n = count.max(1);
    if n == 1 {
        return vec![range.initial];
    }
    let mut vals = Vec::with_capacity(n);
    if range.log_scale && range.initial > 0.0 && range.final_val > 0.0 {
        let l0 = range.initial.ln();
        let l1 = range.final_val.ln();
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            vals.push((l0 + t * (l1 - l0)).exp());
        }
    } else {
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            vals.push(range.initial + t * (range.final_val - range.initial));
        }
    }
    vals
}

/// Check sweep sizes before allocating either axis or the result matrix.
fn validate_sweep(params: &EquilibriumParams) -> Result<(), SoryntelError> {
    let error = |message: &str| SoryntelError::ParseError(message.into());
    if params.num_indep_vars > 2 {
        return Err(error("El equilibrio admite como máximo dos variables independientes"));
    }
    if params.num_indep_vars == 0 { return Ok(()); }
    let axis = |var| -> Result<usize, SoryntelError> {
        let range = match var {
            SweepVariable::TemperatureC => Some(params.temp_range),
            SweepVariable::PressureBar => Some(params.pressure_range),
            SweepVariable::EnthalpyMj => Some(params.enthalpy_range),
            SweepVariable::VolumeM3 => Some(params.volume_range),
            SweepVariable::DischargeLevelKmolE => Some(params.discharge_range),
            _ => None,
        };
        let count = if let Some(r) = range {
            if !r.initial.is_finite() || !r.final_val.is_finite()
                || !(r.final_val - r.initial).is_finite()
                || (r.log_scale && (r.initial <= 0.0 || r.final_val <= 0.0)) {
                return Err(error("El eje requiere límites finitos y positivos para escala logarítmica"));
            }
            r.steps
        } else if var == SweepVariable::XAmount { params.x_steps } else { params.z_steps };
        if count == 0 || count > 100_001 {
            return Err(error("El eje debe contener entre 1 y 100001 puntos"));
        }
        Ok(count)
    };
    let primary = axis(params.primary_var)?;
    let secondary = if params.num_indep_vars == 2 {
        let var = params.secondary_var.ok_or_else(|| error("Falta la segunda variable independiente"))?;
        if var == params.primary_var { return Err(error("Las variables independientes deben ser distintas")); }
        axis(var)?
    } else { 1 };
    if primary.checked_mul(secondary).filter(|&n| n <= 100_001).is_none() {
        return Err(error("El barrido completo de equilibrio supera los 100001 puntos"));
    }
    Ok(())
}

/// Executes a full 0D, 1D, or 2D equilibrium sweep (including `N,H,P`, `N,P,V`, `Target`, and `Cell Mode`).
pub fn solve_equilibrium_sweep<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
) -> Result<EquilibriumSweepResult, SoryntelError> {
    validate_sweep(params)?;
    if params.target.enabled && params.num_indep_vars == 1 {
        let summary = solve_target_calculation(system, params, repo)?;
        return Ok(EquilibriumSweepResult {
            points: vec![summary.point.clone()],
            target_summary: Some(summary),
        });
    }

    if params.num_indep_vars == 0 {
        let pt = solve_state_point(
            system,
            params,
            repo,
            0,
            params.temp_range.initial,
            params.pressure_range.initial,
            0.0,
            None,
            None,
            params.enthalpy_range.initial,
            params.volume_range.initial,
            params.discharge_range.initial,
        )?;
        return Ok(EquilibriumSweepResult {
            points: vec![pt],
            target_summary: None,
        });
    }

    let primary_vals = axis_values_for_var(params, params.primary_var);
    let secondary_vals: Vec<Option<f64>> = if params.num_indep_vars == 2 {
        if let Some(sec) = params.secondary_var {
            axis_values_for_var(params, sec).into_iter().map(Some).collect()
        } else {
            vec![None]
        }
    } else {
        vec![None]
    };

    let mut points = Vec::with_capacity(primary_vals.len() * secondary_vals.len());
    let mut step_counter = 1usize;

    // Track Remove % carry-over state if StepMode == RemovePercent
    let mut carry_inputs_mol: Option<Vec<f64>> = None;

    for &z_opt in &secondary_vals {
        for (i_idx, &x_val) in primary_vals.iter().enumerate() {
            let mut t_c = params.temp_range.initial;
            let mut p_bar = params.pressure_range.initial;
            let mut h_mj = params.enthalpy_range.initial;
            let mut v_m3 = params.volume_range.initial;
            let mut q_kmol_e = params.discharge_range.initial;
            let mut x_frac = 0.0;
            let mut z_frac = 0.0;

            let n_x = primary_vals.len().max(2);
            let frac_x = i_idx as f64 / (n_x - 1) as f64;

            match params.primary_var {
                SweepVariable::TemperatureC => t_c = x_val,
                SweepVariable::PressureBar => p_bar = x_val,
                SweepVariable::EnthalpyMj => h_mj = x_val,
                SweepVariable::VolumeM3 => v_m3 = x_val,
                SweepVariable::DischargeLevelKmolE => q_kmol_e = x_val,
                SweepVariable::XAmount => x_frac = x_val,
                SweepVariable::ZAmount => z_frac = x_val,
            }
            if let (Some(sec_var), Some(z_val)) = (params.secondary_var, z_opt) {
                match sec_var {
                    SweepVariable::TemperatureC => t_c = z_val,
                    SweepVariable::PressureBar => p_bar = z_val,
                    SweepVariable::EnthalpyMj => h_mj = z_val,
                    SweepVariable::VolumeM3 => v_m3 = z_val,
                    SweepVariable::DischargeLevelKmolE => q_kmol_e = z_val,
                    SweepVariable::XAmount => x_frac = z_val,
                    SweepVariable::ZAmount => z_frac = z_val,
                }
            }

            // Compute effective input amounts if XAmount / ZAmount / RemovePercent
            let override_inputs = build_step_inputs(
                system,
                params,
                repo,
                frac_x,
                x_frac,
                z_frac,
                carry_inputs_mol.as_deref(),
            );

            let pt = solve_state_point(
                system,
                params,
                repo,
                step_counter,
                t_c,
                p_bar,
                x_val,
                z_opt,
                override_inputs.as_deref(),
                h_mj,
                v_m3,
                q_kmol_e,
            )?;

            if matches!(
                params.step_mode,
                StepMode::RemovePercent
                    | StepMode::RemovePercentFinalAmount
                    | StepMode::ReactorStandard
                    | StepMode::ReactorSheetsView
            ) {
                let total_sp_count: usize = system.phases.iter().map(|p| p.species.len()).sum();
                let mut next_carry = vec![0.0; total_sp_count];
                let mut flat_idx = 0usize;
                for (p_idx, phase) in system.phases.iter().enumerate() {
                    for sp in &phase.species {
                        let rem_frac = (sp.remove_percent / 100.0).clamp(0.0, 1.0);
                        let eq_mol = pt
                            .species
                            .iter()
                            .find(|s| s.phase_idx == p_idx && s.formula == sp.formula)
                            .map(|s| s.equil_amount_mol)
                            .unwrap_or(0.0);
                        next_carry[flat_idx] = eq_mol * (1.0 - rem_frac);
                        flat_idx += 1;
                    }
                }
                carry_inputs_mol = Some(next_carry);
            }

            points.push(pt);
            step_counter += 1;
        }
    }

    Ok(EquilibriumSweepResult {
        points,
        target_summary: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn solve_state_point<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    step_index: usize,
    t_c: f64,
    p_bar: f64,
    x_value: f64,
    z_value: Option<f64>,
    override_inputs_mol: Option<&[f64]>,
    target_h_mj: f64,
    target_v_m3: f64,
    discharge_kmol_e: f64,
) -> Result<EquilibriumPointResult, SoryntelError> {
    match params.thermo_mode {
        ThermodynamicMode::Ntp | ThermodynamicMode::Ntpq => solve_single_point(
            system,
            params,
            repo,
            t_c + 273.15,
            p_bar,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
            discharge_kmol_e,
        ),
        ThermodynamicMode::Nhp => solve_adiabatic_nhp(
            system,
            params,
            repo,
            p_bar,
            target_h_mj,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
        ),
        ThermodynamicMode::Npv => solve_constant_volume_npv(
            system,
            params,
            repo,
            t_c + 273.15,
            p_bar,
            target_v_m3,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
        ),
    }
}

/// Solves Adiabatic Equilibrium (`N,H,P`): find `T_eq in [200 K, 20000 K]` such that
/// `ReactionEnthalpy(T_eq) == target_h_mj` (or `H_eq(T_eq) - H_in == target_h_mj`).
#[allow(clippy::too_many_arguments)]
fn solve_adiabatic_nhp<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    p_bar: f64,
    target_h_mj: f64,
    step_index: usize,
    x_value: f64,
    z_value: Option<f64>,
    override_inputs_mol: Option<&[f64]>,
) -> Result<EquilibriumPointResult, SoryntelError> {
    let target_kj = target_h_mj * 1000.0;
    let mut t_lo = 200.0f64;
    let mut t_hi = 20000.0f64;

    let pt_lo = solve_single_point(
        system,
        params,
        repo,
        t_lo,
        p_bar,
        step_index,
        x_value,
        z_value,
        override_inputs_mol,
        0.0,
    )?;
    let f_lo = pt_lo.reaction_enthalpy_kj - target_kj;
    if f_lo.abs() < 1e-3 {
        return Ok(pt_lo);
    }

    let mut best_pt = pt_lo;
    for _ in 0..48 {
        let t_mid = 0.5 * (t_lo + t_hi);
        let pt_mid = solve_single_point(
            system,
            params,
            repo,
            t_mid,
            p_bar,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
            0.0,
        )?;
        let f_mid = pt_mid.reaction_enthalpy_kj - target_kj;
        best_pt = pt_mid;
        if f_mid.abs() < 1e-3 || (t_hi - t_lo) < 1e-3 {
            break;
        }
        if (f_lo < 0.0 && f_mid < 0.0) || (f_lo > 0.0 && f_mid > 0.0) {
            t_lo = t_mid;
        } else {
            t_hi = t_mid;
        }
    }
    Ok(best_pt)
}

/// Solves Constant Volume (`N,P,V`): bisection on Pressure (when T is fixed/stepped)
/// or on Temperature (when P is stepped/fixed and T is the dependent variable).
#[allow(clippy::too_many_arguments)]
fn solve_constant_volume_npv<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    t_k: f64,
    p_bar: f64,
    target_v_m3: f64,
    step_index: usize,
    x_value: f64,
    z_value: Option<f64>,
    override_inputs_mol: Option<&[f64]>,
) -> Result<EquilibriumPointResult, SoryntelError> {
    // Section 13.4.3 (`Use as base volume` + `Initial Pressure`):
    let mut eff_target_v = target_v_m3.max(1e-9);
    if params.use_as_base_volume {
        let p_init = params.initial_pressure_bar.max(1e-6);
        let pt_ref = solve_single_point(
            system,
            params,
            repo,
            t_k,
            p_init,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
            0.0,
        )?;
        eff_target_v = (pt_ref.volume_m3 + target_v_m3).max(1e-9);
    }

    let solve_for_temp = params.primary_var == SweepVariable::PressureBar
        || params.secondary_var == Some(SweepVariable::PressureBar);

    if solve_for_temp {
        let p_fixed = p_bar.max(1e-6);
        let mut t_lo = 200.0f64;
        let mut t_hi = 20000.0f64;
        let mut best_pt = solve_single_point(
            system,
            params,
            repo,
            t_k,
            p_fixed,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
            0.0,
        )?;
        for _ in 0..45 {
            let t_mid = 0.5 * (t_lo + t_hi);
            let pt = solve_single_point(
                system,
                params,
                repo,
                t_mid,
                p_fixed,
                step_index,
                x_value,
                z_value,
                override_inputs_mol,
                0.0,
            )?;
            let err = pt.volume_m3 - eff_target_v;
            best_pt = pt;
            if err.abs() < 1e-6 * eff_target_v.max(1e-6) || (t_hi - t_lo) < 1e-3 {
                break;
            }
            if err < 0.0 {
                t_lo = t_mid;
            } else {
                t_hi = t_mid;
            }
        }
        return Ok(best_pt);
    }

    let mut p_lo = 1e-4f64;
    let mut p_hi = 1e4f64;
    let mut best_pt = solve_single_point(
        system,
        params,
        repo,
        t_k,
        p_bar.max(1.0),
        step_index,
        x_value,
        z_value,
        override_inputs_mol,
        0.0,
    )?;
    for _ in 0..45 {
        let p_mid = (p_lo * p_hi).sqrt();
        let pt = solve_single_point(
            system,
            params,
            repo,
            t_k,
            p_mid,
            step_index,
            x_value,
            z_value,
            override_inputs_mol,
            0.0,
        )?;
        let err = pt.volume_m3 - eff_target_v;
        best_pt = pt;
        if err.abs() < 1e-6 * eff_target_v.max(1e-6) {
            break;
        }
        if err > 0.0 {
            p_lo = p_mid;
        } else {
            p_hi = p_mid;
        }
    }
    Ok(best_pt)
}

/// Evaluates a single value `v` of `params.primary_var` for Target Calculations (`13.4.6`).
fn eval_target_point_at<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    v: f64,
    v_min: f64,
    v_max: f64,
) -> Result<EquilibriumPointResult, SoryntelError> {
    let mut t_c = params.temp_range.initial;
    let mut p_bar = params.pressure_range.initial;
    let mut h_mj = params.enthalpy_range.initial;
    let mut v_m3 = params.volume_range.initial;
    let mut q_kmol_e = params.discharge_range.initial;
    let mut x_idx = 0.0;
    let mut z_idx = 0.0;

    let span = (v_max - v_min).abs().max(1e-12);
    let frac_x = ((v - v_min) / span).clamp(0.0, 1.0);

    match params.primary_var {
        SweepVariable::TemperatureC => t_c = v,
        SweepVariable::PressureBar => p_bar = v,
        SweepVariable::EnthalpyMj => h_mj = v,
        SweepVariable::VolumeM3 => v_m3 = v,
        SweepVariable::DischargeLevelKmolE => q_kmol_e = v,
        SweepVariable::XAmount => x_idx = v,
        SweepVariable::ZAmount => z_idx = v,
    }
    let ov = build_step_inputs(system, params, repo, frac_x, x_idx, z_idx, None);
    solve_state_point(
        system,
        params,
        repo,
        1,
        t_c,
        p_bar,
        v,
        None,
        ov.as_deref(),
        h_mj,
        v_m3,
        q_kmol_e,
    )
}

/// Solves Target Calculations (`TargetSpec`) by scanning/bisecting `primary_var`
/// to reach `target_amount_kmol` of `target_species`.
fn solve_target_calculation<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
) -> Result<TargetResultSummary, SoryntelError> {
    let vals = axis_values_for_var(params, params.primary_var);
    let v_min = vals.first().copied().unwrap_or(0.0);
    let v_max = vals.last().copied().unwrap_or(1.0);
    let target_mol = params.target.target_amount_kmol * params.input_scale_to_mol;
    let mut prev_val = v_min;
    let mut prev_err = f64::NAN;
    let mut best_point: Option<EquilibriumPointResult> = None;
    let mut best_var = prev_val;
    let mut best_abs = f64::INFINITY;

    for &v in &vals {
        let pt = eval_target_point_at(system, params, repo, v, v_min, v_max)?;
        let cur_mol = pt
            .species
            .iter()
            .find(|s| s.formula.eq_ignore_ascii_case(params.target.target_species.trim()))
            .map(|s| s.equil_amount_mol)
            .unwrap_or(0.0);
        let err = cur_mol - target_mol;
        if err.abs() < best_abs {
            best_abs = err.abs();
            best_point = Some(pt.clone());
            best_var = v;
        }
        if !prev_err.is_nan() && ((prev_err < 0.0 && err > 0.0) || (prev_err > 0.0 && err < 0.0)) {
            // Refine with bisection on [prev_val, v]
            let mut a = prev_val;
            let mut b = v;
            let mut fa = prev_err;
            for _ in 0..24 {
                let mid = 0.5 * (a + b);
                let pmid = eval_target_point_at(system, params, repo, mid, v_min, v_max)?;
                let cm = pmid
                    .species
                    .iter()
                    .find(|s| s.formula.eq_ignore_ascii_case(params.target.target_species.trim()))
                    .map(|s| s.equil_amount_mol)
                    .unwrap_or(0.0);
                let fm = cm - target_mol;
                best_point = Some(pmid);
                best_var = mid;
                if fm.abs() < 1e-6 * target_mol.max(1e-6) {
                    break;
                }
                if (fa < 0.0 && fm < 0.0) || (fa > 0.0 && fm > 0.0) {
                    a = mid;
                    fa = fm;
                } else {
                    b = mid;
                }
            }
            break;
        }
        prev_val = v;
        prev_err = err;
    }

    let point = best_point.ok_or_else(|| {
        SoryntelError::BalanceError("Target calculation failed to evaluate any points.".to_string())
    })?;
    let achieved_kmol = point
        .species
        .iter()
        .find(|s| s.formula.eq_ignore_ascii_case(params.target.target_species.trim()))
        .map(|s| s.equil_amount_mol / params.input_scale_to_mol.max(1e-12))
        .unwrap_or(0.0);

    Ok(TargetResultSummary {
        target_species: params.target.target_species.clone(),
        sought_value_kmol: params.target.target_amount_kmol,
        achieved_value_kmol: achieved_kmol,
        variable_name: params.primary_var.label().to_string(),
        solved_variable_value: best_var,
        point,
    })
}

fn axis_values_for_var(params: &EquilibriumParams, var: SweepVariable) -> Vec<f64> {
    match var {
        SweepVariable::TemperatureC => {
            generate_axis_values(params.temp_range, params.temp_range.steps)
        }
        SweepVariable::PressureBar => {
            generate_axis_values(params.pressure_range, params.pressure_range.steps)
        }
        SweepVariable::EnthalpyMj => {
            generate_axis_values(params.enthalpy_range, params.enthalpy_range.steps)
        }
        SweepVariable::VolumeM3 => {
            generate_axis_values(params.volume_range, params.volume_range.steps)
        }
        SweepVariable::DischargeLevelKmolE => {
            generate_axis_values(params.discharge_range, params.discharge_range.steps)
        }
        SweepVariable::XAmount => {
            let n = params.x_steps.max(1);
            (0..n).map(|i| i as f64).collect()
        }
        SweepVariable::ZAmount => {
            let n = params.z_steps.max(1);
            (0..n).map(|i| i as f64).collect()
        }
    }
}

fn build_step_inputs<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    frac_x: f64,
    x_step_idx: f64,
    z_step_idx: f64,
    carry_mol: Option<&[f64]>,
) -> Option<Vec<f64>> {
    let needs_override = params.primary_var == SweepVariable::XAmount
        || params.secondary_var == Some(SweepVariable::ZAmount)
        || matches!(
            params.step_mode,
            StepMode::RemovePercent
                | StepMode::RemovePercentFinalAmount
                | StepMode::ReactorStandard
                | StepMode::ReactorSheetsView
        );
    if !needs_override {
        return None;
    }
    let mut out = Vec::new();
    let mut flat = 0usize;
    for phase in &system.phases {
        for sp in &phase.species {
            let mw = sp
                .species_data
                .as_ref()
                .map(|d| d.molecular_weight)
                .or_else(|| repo.get_species(sp.formula.trim()).map(|d| d.molecular_weight))
                .unwrap_or(1.0);
            let scale = match sp.amount_unit {
                crate::equilibrium::model::EquilibriumAmountUnit::Kmol => params.input_scale_to_mol,
                crate::equilibrium::model::EquilibriumAmountUnit::Mol => 1.0,
                crate::equilibrium::model::EquilibriumAmountUnit::Kg => 1000.0 / mw.max(1e-6),
                crate::equilibrium::model::EquilibriumAmountUnit::Nm3 => {
                    1000.0 / soryntel_core::STP_MOLAR_VOL_L
                }
                crate::equilibrium::model::EquilibriumAmountUnit::GramPerLiter => 1.0 / mw.max(1e-6),
                crate::equilibrium::model::EquilibriumAmountUnit::MolPerLiter => 1.0,
            };
            let init_mol = sp.initial_amount * scale;
            let base = if let Some(c) = carry_mol {
                *c.get(flat).unwrap_or(&0.0)
            } else {
                init_mol
            };
            let add_x = sp.add_x_amount * scale;
            let add_z = sp.add_z_amount * scale;
            let val = match params.step_mode {
                StepMode::AddAmount => base + x_step_idx * add_x + z_step_idx * add_z,
                StepMode::FinalAmount => init_mol + frac_x * (add_x - init_mol),
                StepMode::RemovePercent
                | StepMode::ReactorStandard
                | StepMode::ReactorSheetsView => {
                    if carry_mol.is_some() {
                        base + add_x
                    } else {
                        base + x_step_idx * add_x + z_step_idx * add_z
                    }
                }
                StepMode::RemovePercentFinalAmount => {
                    let n_steps = (params.x_steps.max(2) - 1) as f64;
                    let step_delta = (add_x - init_mol).max(0.0) / n_steps;
                    if carry_mol.is_some() {
                        base + step_delta
                    } else {
                        base
                    }
                }
            };
            out.push(val.max(0.0));
            flat += 1;
        }
    }
    Some(out)
}

/// Collects Cp extrapolation warnings for the `Warnings` dialog (Figs. 35–38).
pub fn collect_cp_warnings<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
) -> Vec<ExtrapolationWarning> {
    let (t1_c, t2_c) = if params.thermo_mode == ThermodynamicMode::Nhp
        || params.thermo_mode == ThermodynamicMode::Npv
    {
        (-273.15, 19726.85)
    } else if params.num_indep_vars > 0 && params.primary_var == SweepVariable::TemperatureC {
        (
            params.temp_range.initial.min(params.temp_range.final_val),
            params.temp_range.initial.max(params.temp_range.final_val),
        )
    } else {
        (params.temp_range.initial, params.temp_range.initial)
    };

    let mut warnings = Vec::new();
    for (p_idx, phase) in system.phases.iter().enumerate() {
        for (s_idx, sp) in phase.species.iter().enumerate() {
            let sd = sp
                .species_data
                .clone()
                .or_else(|| repo.get_species(sp.formula.trim()));
            if let Some(data) = sd {
                if let (Some(first), Some(last)) = (data.intervals.first(), data.intervals.last()) {
                    let avail_min_c = first.t_min - 273.15;
                    let avail_max_c = last.t_max - 273.15;
                    let low = t1_c < avail_min_c - 0.05;
                    let high = t2_c > avail_max_c + 0.05;
                    if low || high {
                        warnings.push(ExtrapolationWarning {
                            phase_idx: p_idx,
                            species_idx: s_idx,
                            formula: sp.formula.clone(),
                            selected_t1_c: t1_c,
                            selected_t2_c: t2_c,
                            available_t_min_c: avail_min_c,
                            available_t_max_c: avail_max_c,
                            reference: first.reference.clone(),
                            feed_temp_c: sp.feed_temp_c,
                            out_of_range_low: low,
                            out_of_range_high: high,
                            suggested_remove: high && sp.initial_amount <= 0.0,
                        });
                    }
                }
            }
        }
    }
    warnings
}
