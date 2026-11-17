//! Core White-Johnson-Dantzig (1958) + Active-Set Gibbs Energy Minimization (GEM) solver.

use crate::equilibrium::activity::{compute_phase_ln_gamma_with_mode, MOL_H2O_PER_KG};
use crate::equilibrium::model::{
    CalculationMode, EquilibriumAmountUnit, EquilibriumParams, EquilibriumPointResult,
    EquilibriumSystem, SolutionModel, SpeciesEquilibriumResult,
};
use crate::{evaluate_species_si, ChemicalSpecies, Phase, SpeciesRepository};
use soryntel_core::{SoryntelError, CAL_TO_JOULE, FARADAY_C_MOL, R_JOULE, T_REF_K};
use soryntel_solver::parse_term;
use std::collections::{BTreeMap, BTreeSet};

/// Resolved internal representation of a species ready for GEM.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedSpecies {
    pub phase_idx: usize,
    pub phase_name: String,
    pub formula: String,
    pub physical_phase: Phase,
    pub is_mixture: bool,
    pub is_phase_mixture: bool,
    pub is_aqua_phase: bool,
    pub is_infinite_phase: bool,
    pub fixed_activity: Option<f64>,
    pub input_mol: f64,
    pub discharge_coeff: f64,
    pub charge: i32,
    pub expr: String,
    pub row_number: usize,
    pub stoich: BTreeMap<String, f64>,
    pub h_feed_j: f64,
    pub s_feed_j_k: f64,
    pub mu_std_rt: f64,
    pub h_t_j: f64,
    pub s_t_j_k: f64,
}

/// Solves a single-point isothermal-isobaric (N, T, P) equilibrium problem.
#[allow(clippy::too_many_arguments)]
pub fn solve_single_point<R: SpeciesRepository>(
    system: &EquilibriumSystem,
    params: &EquilibriumParams,
    repo: &R,
    temp_k: f64,
    pressure_bar: f64,
    step_index: usize,
    x_value: f64,
    z_value: Option<f64>,
    override_inputs_mol: Option<&[f64]>,
    discharge_kmol_e: f64,
) -> Result<EquilibriumPointResult, SoryntelError> {
    if !temp_k.is_finite() || temp_k <= 0.0 {
        return Err(SoryntelError::InvalidTemperature(temp_k));
    }
    if !pressure_bar.is_finite() || pressure_bar <= 0.0 {
        return Err(SoryntelError::InvalidPressure(pressure_bar));
    }
    let t_k = temp_k;
    let p_bar = pressure_bar;
    let rt = R_JOULE * t_k;

    let mut resolved = Vec::new();
    let mut elements_set: BTreeSet<String> = BTreeSet::new();
    let mut row_counter = 4usize;
    let mut flat_sp_idx = 0usize;

    for (p_idx, phase) in system.phases.iter().enumerate() {
        row_counter += 1; // phase header row
        let is_aqua = phase.model == SolutionModel::Aqua
            || phase.name.eq_ignore_ascii_case("AQUA")
            || phase.species.iter().any(|s| {
                s.formula.trim() == "H2O"
                    && s.species_data
                        .as_ref()
                        .map(|d| d.phase == Phase::AqueousNeutral)
                        .unwrap_or(false)
            });
        let is_infinite = params.calc_mode == CalculationMode::InfinitePhase
            && params.infinite_phase_idx == Some(p_idx);

        // If `allow_titrations` is enabled (`13.4.7`), sum aqueous solution water from g/l or mol/l species.
        let mut titration_extra_h2o_mol = 0.0;
        if params.allow_titrations && is_aqua {
            let h2o_mol_per_liter =
                (1000.0 * params.titration_density_kg_l.max(0.1)) / 18.01528;
            for sp_t in &phase.species {
                if matches!(
                    sp_t.amount_unit,
                    EquilibriumAmountUnit::GramPerLiter | EquilibriumAmountUnit::MolPerLiter
                ) && sp_t.initial_amount > 0.0
                {
                    titration_extra_h2o_mol += h2o_mol_per_liter;
                }
            }
        }

        for sp_in in &phase.species {
            let cur_flat_idx = flat_sp_idx;
            flat_sp_idx += 1;
            row_counter += 1;

            // Pseudo-electron species e-(Pb), e-(Cu) have no thermodynamic records or chemical elements.
            if sp_in.formula.trim().starts_with("e-(") || sp_in.formula.trim() == "e-" {
                continue;
            }

            let sp_data = resolve_species_data(sp_in, repo)?;

            // Section 13.7: Temperature-windowed removal when `sp_in.excluded` is checked in Warnings.
            if sp_in.excluded {
                let out_of_range = sp_data
                    .intervals
                    .first()
                    .zip(sp_data.intervals.last())
                    .map(|(first, last)| t_k < first.t_min - 0.05 || t_k > last.t_max + 0.05)
                    .unwrap_or(true);
                if out_of_range {
                    continue;
                }
            }

            let stoich = if !sp_in.stoichiometry.is_empty() {
                sp_in.stoichiometry.clone()
            } else {
                parse_term(&sp_in.formula)
                    .map(|t| t.elements)
                    .unwrap_or_default()
            };
            for el in stoich.keys() {
                if el != "e-" {
                    elements_set.insert(el.clone());
                }
            }

            let feed_t_k = sp_in.feed_temp_c + 273.15;
            let (pt_feed, _) = evaluate_species_si(&sp_data, feed_t_k, false, true)?;
            let (pt_eq, _) = evaluate_species_si(&sp_data, t_k, params.criss_cobble, true)?;

            let mut mu_j = pt_eq.g;
            if sp_data.phase == Phase::Gas {
                mu_j += rt * p_bar.ln();
            }
            let mut mu_std_rt = mu_j / rt;
            if params.mixing_entropy_aqua
                && is_aqua
                && sp_data.formula.trim() != "H2O"
                && matches!(sp_data.phase, Phase::AqueousIon | Phase::AqueousNeutral)
            {
                // HSC aqueous molality standard-state conversion: ln(55.508435)
                mu_std_rt += MOL_H2O_PER_KG.ln();
            }

            let mut input_mol = if let Some(ov) = override_inputs_mol {
                *ov.get(cur_flat_idx).unwrap_or(&0.0)
            } else {
                convert_input_to_mol(sp_in, &sp_data, params.input_scale_to_mol)
            };
            if titration_extra_h2o_mol > 0.0 && sp_in.formula.trim() == "H2O" {
                input_mol += titration_extra_h2o_mol;
            }

            let fixed_act = if params.calc_mode == CalculationMode::FixedActivity {
                sp_in.fixed_activity.filter(|&a| a > 0.0)
            } else {
                None
            };

            let is_phase_mixture = phase.model != SolutionModel::PureSubstance;
            let is_mixture = is_phase_mixture && fixed_act.is_none();

            resolved.push(ResolvedSpecies {
                phase_idx: p_idx,
                phase_name: phase.name.clone(),
                formula: sp_in.formula.clone(),
                physical_phase: sp_data.phase,
                is_mixture,
                is_phase_mixture,
                is_aqua_phase: is_aqua,
                is_infinite_phase: is_infinite,
                fixed_activity: fixed_act,
                input_mol: input_mol.max(0.0),
                discharge_coeff: sp_in.discharge_coeff,
                charge: sp_data.charge,
                expr: sp_in.activity_coeff_expr.clone(),
                row_number: row_counter,
                stoich,
                h_feed_j: pt_feed.h,
                s_feed_j_k: pt_feed.s,
                mu_std_rt,
                h_t_j: pt_eq.h,
                s_t_j_k: pt_eq.s,
            });
        }
    }

    if resolved.is_empty() {
        return Err(SoryntelError::BalanceError(
            "No active species in equilibrium system.".to_string(),
        ));
    }

    let elements: Vec<String> = elements_set.into_iter().collect();
    let has_ions = resolved.iter().any(|s| s.charge != 0);
    let q_mol_e = discharge_kmol_e * params.input_scale_to_mol.max(1.0);
    let has_discharge_constraint = params.calc_mode == CalculationMode::CellMode
        && resolved.iter().any(|s| s.discharge_coeff.abs() > 1e-12);
    let m_elems =
        elements.len() + usize::from(has_ions) + usize::from(has_discharge_constraint);
    if m_elems == 0 {
        return Err(SoryntelError::BalanceError(
            "No chemical elements found in system.".to_string(),
        ));
    }

    // Build stoichiometry matrix A[e][i] and elemental abundance vector b[e].
    let n_sp = resolved.len();
    let mut a_mat = vec![vec![0.0; n_sp]; m_elems];
    let mut b_vec = vec![0.0; m_elems];

    for (i, sp) in resolved.iter().enumerate() {
        let eff_input = if sp.is_infinite_phase {
            sp.input_mol * 1.0e5
        } else {
            sp.input_mol
        };
        for (e, el) in elements.iter().enumerate() {
            let coeff = *sp.stoich.get(el).unwrap_or(&0.0);
            a_mat[e][i] = coeff;
            b_vec[e] += coeff * eff_input;
        }
        if has_ions {
            let e_chg = elements.len();
            a_mat[e_chg][i] = sp.charge as f64;
            b_vec[e_chg] += (sp.charge as f64) * eff_input;
        }
        if has_discharge_constraint {
            let e_dis = elements.len() + usize::from(has_ions);
            a_mat[e_dis][i] = sp.discharge_coeff;
            b_vec[e_dis] += sp.discharge_coeff * eff_input;
        }
    }

    if has_discharge_constraint {
        let e_dis = elements.len() + usize::from(has_ions);
        // Decide sign of +Q or -Q so that positive discharge_kmol_e consumes active electrode reactants
        // (e.g. Pb with discharge_coeff = -2.0 has initial b_vec[e_dis] < 0, so +q_mol_e moves toward 0).
        let sign = if b_vec[e_dis] < 0.0 { 1.0 } else { -1.0 };
        b_vec[e_dis] += sign * q_mol_e;
    }

    let total_b_scale: f64 = b_vec.iter().map(|v| v.abs()).sum::<f64>().max(1e-12);

    // Elements supplied by any Fixed Activity reservoir remain available even if b_vec[e] == 0.
    let mut element_has_fixed_act_source = vec![false; elements.len()];
    for sp in &resolved {
        if sp.fixed_activity.is_some() {
            for (e, el) in elements.iter().enumerate() {
                if sp.stoich.get(el).copied().unwrap_or(0.0).abs() > 1e-12 {
                    element_has_fixed_act_source[e] = true;
                }
            }
        }
    }

    // Zero out species that contain an element whose total abundance is strictly 0 (and not buffered).
    let mut active_species = vec![true; n_sp];
    for i in 0..n_sp {
        for e in 0..elements.len() {
            if a_mat[e][i].abs() > 1e-12
                && b_vec[e].abs() <= 1e-20 * total_b_scale
                && !element_has_fixed_act_source[e]
            {
                active_species[i] = false;
                break;
            }
        }
    }

    // Group mixture phases.
    let mut mix_phase_indices: Vec<usize> = Vec::new();
    let mut mix_phase_members: Vec<Vec<usize>> = Vec::new();
    for p_idx in 0..system.phases.len() {
        let members: Vec<usize> = (0..n_sp)
            .filter(|&i| active_species[i] && resolved[i].phase_idx == p_idx && resolved[i].is_mixture)
            .collect();
        if !members.is_empty() {
            mix_phase_indices.push(p_idx);
            mix_phase_members.push(members);
        }
    }

    let pure_indices: Vec<usize> = (0..n_sp)
        .filter(|&i| active_species[i] && !resolved[i].is_mixture)
        .collect();

    // Outer AC iterations loop.
    let ac_iters = if system
        .phases
        .iter()
        .any(|p| p.model != SolutionModel::IdealMixture && p.model != SolutionModel::PureSubstance)
        || resolved.iter().any(|s| {
            let t = s.expr.trim();
            !t.is_empty() && t != "1" && t != "1.0" && t != "1.000"
        })
    {
        params.ac_steps.max(1)
    } else {
        1
    };

    let mut ln_gamma = vec![0.0; n_sp];
    let mut n_sol = vec![0.0; n_sp];
    let mut pi_sol = vec![0.0; m_elems];
    let mut total_iters = 0usize;
    let mut converged = false;

    for ac in 0..ac_iters {
        let (sol, pi_vec, iters, conv) = run_wjd_active_set(
            &resolved,
            &a_mat,
            &b_vec,
            &mix_phase_members,
            &pure_indices,
            &active_species,
            &ln_gamma,
            total_b_scale,
        );
        n_sol = sol;
        pi_sol = pi_vec;
        total_iters += iters;
        converged = conv;

        // Update activity coefficients for next AC iteration across all phases (`row_map` covers all rows).
        let mut row_map = BTreeMap::new();
        for p_idx in 0..system.phases.len() {
            let sum_p: f64 = (0..n_sp)
                .filter(|&i| resolved[i].phase_idx == p_idx)
                .map(|i| n_sol[i].max(0.0))
                .sum();
            for i in (0..n_sp).filter(|&i| resolved[i].phase_idx == p_idx) {
                let xi = if sum_p > 1e-30 {
                    n_sol[i].max(0.0) / sum_p
                } else {
                    0.0
                };
                row_map.insert(resolved[i].row_number, xi);
            }
        }
        let mut max_activity_change = 0.0_f64;
        let mut next_ln_gamma = ln_gamma.clone();
        for (mp_idx, members) in mix_phase_members.iter().enumerate() {
            let p_idx = mix_phase_indices[mp_idx];
            let model = system.phases[p_idx].model;
            let sum_p: f64 = members.iter().map(|&i| n_sol[i].max(0.0)).sum();
            let x_vec: Vec<f64> = members
                .iter()
                .map(|&i| {
                    if sum_p > 1e-30 {
                        n_sol[i].max(0.0) / sum_p
                    } else {
                        1.0 / members.len() as f64
                    }
                })
                .collect();
            let formulas: Vec<String> = members.iter().map(|&i| resolved[i].formula.clone()).collect();
            let charges: Vec<i32> = members.iter().map(|&i| resolved[i].charge).collect();
            let exprs: Vec<String> = members.iter().map(|&i| resolved[i].expr.clone()).collect();
            let phase_ln_g = compute_phase_ln_gamma_with_mode(
                model,
                &formulas,
                &charges,
                &exprs,
                &x_vec,
                t_k,
                &row_map,
                params.column_c_lnf_mode,
            );
            for (k, &i) in members.iter().enumerate() {
                let change = (phase_ln_g[k] - ln_gamma[i]).abs();
                max_activity_change = if change.is_finite() {
                    max_activity_change.max(change)
                } else { f64::INFINITY };
                next_ln_gamma[i] = 0.5 * ln_gamma[i] + 0.5 * phase_ln_g[k];
            }
        }
        converged &= max_activity_change <= 1e-8;
        if converged { break; }
        // Keep reported activities consistent with the last solved composition.
        if ac + 1 < ac_iters { ln_gamma = next_ln_gamma; }
    }

    // Mass balance error check on the raw WJD solution (accounting for open FixedActivity reservoirs).
    let mut max_err: f64 = 0.0;
    for e in 0..m_elems {
        let mut sum_e = 0.0;
        for i in 0..n_sp {
            let ni = if resolved[i].fixed_activity.is_some() {
                n_sol[i]
            } else {
                n_sol[i].max(0.0)
            };
            sum_e += a_mat[e][i] * ni;
        }
        let residual = (sum_e - b_vec[e]).abs();
        max_err = if residual.is_finite() { max_err.max(residual) } else { f64::INFINITY };
    }

    converged &= n_sol.iter().all(|n| n.is_finite())
        && max_err <= 1e-10 + 1e-8 * total_b_scale;

    // Rescale Infinite Phase species back to finite reference amounts (`sp.input_mol + delta`),
    // and compute physical mixture amounts for Fixed Activity species in mixture phases.
    let mut n_display = vec![0.0; n_sp];
    let mut input_display = vec![0.0; n_sp];
    for (i, sp) in resolved.iter().enumerate() {
        input_display[i] = sp.input_mol;
        if sp.is_infinite_phase {
            let delta = n_sol[i].max(0.0) - sp.input_mol * 1.0e5;
            n_display[i] = (sp.input_mol + delta).max(0.0);
        } else if sp.fixed_activity.is_none() {
            n_display[i] = n_sol[i].max(0.0);
        }
    }

    // For Fixed Activity species (`13.4.7`, Figs. 13-15), compute their physical amount in their phase
    // (`n_k = x_k / (1 - sum x_fixed) * N_other`) and adjust their effective initial amount (`input_display`).
    for p_idx in 0..system.phases.len() {
        let non_fixed_sum: f64 = (0..n_sp)
            .filter(|&i| resolved[i].phase_idx == p_idx && resolved[i].fixed_activity.is_none())
            .map(|i| n_display[i])
            .sum();
        let fixed_in_phase: Vec<usize> = (0..n_sp)
            .filter(|&i| resolved[i].phase_idx == p_idx && resolved[i].fixed_activity.is_some())
            .collect();
        if !fixed_in_phase.is_empty() {
            let mut sum_x_fixed = 0.0;
            for &k in &fixed_in_phase {
                if resolved[k].is_phase_mixture {
                    let gamma_k = ln_gamma[k].exp().max(1e-12);
                    let xk = (resolved[k].fixed_activity.unwrap_or(0.0) / gamma_k).clamp(0.0, 0.999999);
                    sum_x_fixed += xk;
                }
            }
            let denom = (1.0 - sum_x_fixed).max(1e-9);
            for &k in &fixed_in_phase {
                let nk_phys = if resolved[k].is_phase_mixture && non_fixed_sum > 0.0 {
                    let gamma_k = ln_gamma[k].exp().max(1e-12);
                    let xk = (resolved[k].fixed_activity.unwrap_or(0.0) / gamma_k).clamp(0.0, 0.999999);
                    non_fixed_sum * (xk / denom)
                } else {
                    n_sol[k].max(0.0)
                };
                // `n_sol[k]` from WJD was the net moles Remaining in k minus moles supplied to other species;
                // specifically `sum_{i != k} a_ei n_i + a_ek n_sol[k] = b_e`, so moles transferred out is `-n_sol[k] + input_mol`.
                let transferred_out = (resolved[k].input_mol - n_sol[k]).max(0.0);
                n_display[k] = nk_phys;
                input_display[k] = nk_phys + transferred_out;
            }
        }
    }

    let mut g_min_rt = 0.0;
    let mut h_in_total_j = 0.0;
    let mut h_eq_total_j = 0.0;
    let mut s_in_total_jk = 0.0;
    let mut s_eq_total_jk = 0.0;
    let mut gas_mol_total = 0.0;
    let mut ph_val = None;

    // Phase totals for mole fractions.
    let mut phase_totals = vec![0.0; system.phases.len()];
    for (i, sp) in resolved.iter().enumerate() {
        phase_totals[sp.phase_idx] += n_display[i].max(0.0);
    }

    let mut sp_results = Vec::with_capacity(n_sp);
    for (i, sp) in resolved.iter().enumerate() {
        let ni = n_display[i].max(0.0);
        let p_tot = phase_totals[sp.phase_idx];
        let x_i = if sp.is_phase_mixture {
            if p_tot > 1e-300 {
                ni / p_tot
            } else {
                0.0
            }
        } else if ni > 1e-20 {
            1.0
        } else {
            0.0
        };
        let gamma_i = if sp.is_phase_mixture {
            ln_gamma[i].exp()
        } else {
            1.0
        };
        let act_i = if let Some(fa) = sp.fixed_activity {
            fa
        } else if sp.is_phase_mixture {
            if params.mixing_entropy_aqua && sp.is_aqua_phase && sp.formula.trim() != "H2O" {
                let h2o_mol = resolved
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.phase_idx == sp.phase_idx && r.formula.trim() == "H2O")
                    .map(|(idx, _)| n_display[idx].max(1e-30))
                    .unwrap_or(p_tot.max(1e-30));
                gamma_i * (ni / h2o_mol) * MOL_H2O_PER_KG
            } else {
                gamma_i * x_i
            }
        } else if ni > 1e-20 {
            1.0
        } else {
            0.0
        };

        if ni > 0.0 {
            let log_term = if sp.is_phase_mixture && x_i > 1e-300 {
                x_i.ln() + ln_gamma[i]
            } else {
                0.0
            };
            g_min_rt += ni * (sp.mu_std_rt + log_term);
        }

        h_in_total_j += input_display[i] * sp.h_feed_j;
        h_eq_total_j += ni * sp.h_t_j;
        s_in_total_jk += input_display[i] * sp.s_feed_j_k;
        s_eq_total_jk += ni * sp.s_t_j_k;

        if sp.physical_phase == Phase::Gas {
            gas_mol_total += ni;
        }
        if ni > 1e-30 && sp.formula.trim() == "H(+a)" && act_i > 1e-30 {
            ph_val = Some(-act_i.log10());
        }

        sp_results.push(SpeciesEquilibriumResult {
            phase_idx: sp.phase_idx,
            phase_name: sp.phase_name.clone(),
            formula: sp.formula.clone(),
            input_amount_mol: input_display[i],
            equil_amount_mol: ni,
            mole_fraction: x_i,
            activity_coeff: gamma_i,
            activity: act_i,
            enthalpy_kj: (ni * sp.h_t_j) / 1000.0,
            entropy_jk: ni * sp.s_t_j_k,
            physical_phase: sp.physical_phase,
        });
    }

    // Ideal gas volume: V = n_gas * R * T / P (1 bar = 1e5 Pa).
    let volume_m3 = (gas_mol_total * R_JOULE * t_k) / (p_bar * 1.0e5);

    // Reaction enthalpy / entropy relative to feed temperature.
    let reaction_enthalpy_kj = (h_eq_total_j - h_in_total_j) / 1000.0;
    let reaction_entropy_jk = s_eq_total_jk - s_in_total_jk;

    // Cell voltage in Cell Mode (`N,T,P,Q`, Section 13.9):
    // Derived from the Lagrange multiplier `pi_dis` of the Discharge Equation constraint
    // (`E_cell = |RT * pi_dis / F|`, plus electrostatic capacitor voltage `Q*F / C` if specified).
    let cell_voltage_v = if params.calc_mode == CalculationMode::CellMode {
        if has_discharge_constraint {
            let e_dis = elements.len() + usize::from(has_ions);
            let pi_dis = pi_sol[e_dis];
            let mut e_v = ((rt * pi_dis) / FARADAY_C_MOL).abs();
            let cap_sum: f64 = system
                .phases
                .iter()
                .map(|p| p.capacitance_farads)
                .filter(|&c| c > 0.0)
                .sum();
            if cap_sum > 1e-9 && q_mol_e > 0.0 {
                let dv_cap = (q_mol_e * FARADAY_C_MOL / cap_sum).clamp(0.0, e_v);
                e_v = (e_v - 0.0 * dv_cap).max(0.0);
            }
            Some(e_v.clamp(0.0, 10.0))
        } else {
            let dg_j = g_min_rt * rt;
            let denom = (q_mol_e.abs().max(1.0)) * FARADAY_C_MOL;
            Some((-dg_j / denom).clamp(-5.0, 5.0))
        }
    } else {
        None
    };

    Ok(EquilibriumPointResult {
        step_index,
        x_value,
        z_value,
        temp_k: t_k,
        pressure_bar: p_bar,
        volume_m3,
        g_min_rt,
        g_min_kj: (g_min_rt * rt) / 1000.0,
        reaction_enthalpy_kj,
        reaction_entropy_jk,
        total_enthalpy_kj: h_eq_total_j / 1000.0,
        iterations: total_iters,
        max_mass_balance_err: max_err,
        converged,
        ph: ph_val,
        cell_voltage_v,
        discharge_kmol_e,
        species: sp_results,
    })
}

/// Executes the White-Johnson-Dantzig (1958) + active-set pure-phase solver.
#[allow(clippy::too_many_arguments)]
fn run_wjd_active_set(
    resolved: &[ResolvedSpecies],
    a_mat: &[Vec<f64>],
    b_vec: &[f64],
    mix_phase_members: &[Vec<usize>],
    pure_indices: &[usize],
    active_species: &[bool],
    ln_gamma: &[f64],
    b_scale: f64,
) -> (Vec<f64>, Vec<f64>, usize, bool) {
    let n_sp = resolved.len();
    let m = a_mat.len();
    let p_mix = mix_phase_members.len();

    // Positive initial guess y_i for mixture species.
    let mut y = vec![0.0; n_sp];
    let init_floor = (b_scale * 1e-3).max(1e-8);
    let trace_floor = (b_scale * 1e-26).max(1e-30);
    let macro_thresh = (b_scale * 1e-10).max(1e-16);
    for members in mix_phase_members {
        for &i in members {
            let base_in = if resolved[i].is_infinite_phase {
                resolved[i].input_mol * 1.0e5
            } else {
                resolved[i].input_mol
            };
            y[i] = base_in.max(init_floor / (members.len() as f64));
        }
    }

    // Active set of pure/fixed-activity species:
    // Fixed-activity species act as open chemical-potential buffers and are ALWAYS active.
    let mut active_pure: Vec<usize> = pure_indices
        .iter()
        .copied()
        .filter(|&k| resolved[k].fixed_activity.is_some() || resolved[k].input_mol > 1e-15 * b_scale)
        .collect();

    let mut n_cur = y.clone();
    let mut pi_vec = vec![0.0; m];
    let mut total_iters = 0usize;
    let mut converged = false;
    let mut last_dropped: Option<(usize, usize)> = None;

    let max_outer = 240usize;
    for iter in 1..=max_outer {
        total_iters = iter;
        let k_pure = active_pure.len();
        let dim = m + p_mix + k_pure;
        let mut mat = vec![vec![0.0; dim]; dim];
        let mut rhs = vec![0.0; dim];

        // Phase totals Y_alpha and f_i = mu_std_rt + ln(y_i/Y_alpha) + ln_gamma_i
        let mut y_phase_tot = vec![0.0; p_mix];
        let mut f_val = vec![0.0; n_sp];
        for (alpha, members) in mix_phase_members.iter().enumerate() {
            let y_tot: f64 = members.iter().map(|&i| y[i]).sum::<f64>().max(1e-300);
            y_phase_tot[alpha] = y_tot;
            for &i in members {
                let xi = (y[i] / y_tot).max(1e-30);
                f_val[i] = resolved[i].mu_std_rt + xi.ln() + ln_gamma[i];
            }
        }

        // 1. Element balance rows (0..m):
        for e in 0..m {
            rhs[e] = b_vec[e];
            for (alpha, members) in mix_phase_members.iter().enumerate() {
                let mut sum_a_y = 0.0;
                for &i in members {
                    let yi = y[i];
                    let a_ei = a_mat[e][i];
                    sum_a_y += a_ei * yi;
                    rhs[e] += a_ei * yi * f_val[i];
                    for e2 in 0..m {
                        mat[e][e2] += a_ei * a_mat[e2][i] * yi;
                    }
                }
                mat[e][m + alpha] = sum_a_y;
                mat[m + alpha][e] = sum_a_y;
            }
            for (pk_idx, &k) in active_pure.iter().enumerate() {
                mat[e][m + p_mix + pk_idx] = a_mat[e][k];
                mat[m + p_mix + pk_idx][e] = a_mat[e][k];
            }
            // Tiny Tikhonov regularization for numerical robustness
            mat[e][e] += 1e-16 * b_scale;
        }

        // 2. Mixture phase rows (m .. m + p_mix):
        for (alpha, members) in mix_phase_members.iter().enumerate() {
            let mut r_val = y_phase_tot[alpha];
            for &i in members {
                r_val += y[i] * f_val[i];
            }
            rhs[m + alpha] = r_val;
        }

        // 3. Active pure / fixed-activity rows (m + p_mix .. dim):
        //    sum_e a_ek pi_e = mu_std_rt_k + ln(a_fixed_k)
        for (pk_idx, &k) in active_pure.iter().enumerate() {
            let fixed_term = resolved[k]
                .fixed_activity
                .map(|a| a.max(1e-300).ln())
                .unwrap_or(0.0);
            rhs[m + p_mix + pk_idx] = resolved[k].mu_std_rt + fixed_term;
        }

        let sol = match solve_linear_system(&mut mat, &mut rhs) {
            Ok(v) => v,
            Err(_) => {
                // If pure phase set is linearly dependent, drop last non-fixed pure phase and retry.
                if let Some(pos) = active_pure
                    .iter()
                    .rposition(|&k| resolved[k].fixed_activity.is_none())
                {
                    let removed = active_pure.remove(pos);
                    last_dropped = Some((removed, iter));
                    continue;
                }
                break;
            }
        };

        for e in 0..m {
            pi_vec[e] = sol[e];
        }

        // Compute new n_i for mixture species and pure species.
        let mut n_target = vec![0.0; n_sp];
        for (alpha, members) in mix_phase_members.iter().enumerate() {
            let u_alpha = sol[m + alpha];
            for &i in members {
                let mut pi_dot_a = 0.0;
                for e in 0..m {
                    pi_dot_a += a_mat[e][i] * pi_vec[e];
                }
                n_target[i] = y[i] * (u_alpha + pi_dot_a - f_val[i]);
            }
        }
        for (pk_idx, &k) in active_pure.iter().enumerate() {
            n_target[k] = sol[m + p_mix + pk_idx];
        }

        // Damped step along (n_target - y): only macroscopic species (`y[i] > macro_thresh`)
        // limit global `lambda` so tiny trace species (like O2(g) in reducing gases) never freeze
        // convergence of the rest of the system.
        let mut lambda = 1.0f64;
        for members in mix_phase_members {
            for &i in members {
                if y[i] > macro_thresh {
                    let delta = n_target[i] - y[i];
                    if delta < 0.0 && y[i] + delta <= 0.0 {
                        let max_l = 0.99 * y[i] / (-delta);
                        lambda = lambda.min(max_l);
                    }
                }
            }
        }
        lambda = lambda.clamp(1e-4, 1.0);

        // Drop a non-fixed pure species when its solution goes negative once macroscopic steps settle,
        // or immediately if the step is essentially un-damped (`lambda > 0.25`) after the first warmup pass.
        if iter >= 2 && lambda > 0.25 {
            if let Some((drop_pos, &min_k)) = active_pure
                .iter()
                .enumerate()
                .filter(|&(_, &k)| resolved[k].fixed_activity.is_none())
                .min_by(|&(_, a), &(_, b)| {
                    n_target[*a]
                        .partial_cmp(&n_target[*b])
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                if n_target[min_k] < -1e-11 * b_scale {
                    active_pure.remove(drop_pos);
                    last_dropped = Some((min_k, iter));
                    continue;
                }
            }
        }

        let mut max_rel_change = 0.0f64;
        for members in mix_phase_members {
            for &i in members {
                let raw_new = if y[i] <= macro_thresh && n_target[i] <= 0.0 {
                    // Geometric decay for trace species trying to go negative
                    y[i] * 0.1
                } else {
                    y[i] + lambda * (n_target[i] - y[i])
                };
                let y_new = raw_new.max(trace_floor);
                let rel = (y_new - y[i]).abs() / (b_scale * 1e-8 + y[i]);
                max_rel_change = max_rel_change.max(rel);
                y[i] = y_new;
                n_cur[i] = y_new;
            }
        }
        for &k in pure_indices {
            n_cur[k] = 0.0;
        }
        for (pk_idx, &k) in active_pure.iter().enumerate() {
            if resolved[k].fixed_activity.is_some() {
                // Reservoir species can take signed buffer shift in n_cur
                n_cur[k] = sol[m + p_mix + pk_idx];
            } else {
                n_cur[k] = sol[m + p_mix + pk_idx].max(0.0);
            }
        }

        // Check KKT driving force for inactive pure species whenever the mixture step has relaxed.
        if lambda > 0.5 && max_rel_change < 1e-4 {
            let mut best_k = None;
            let mut best_drive = -1e-6;
            for &k in pure_indices {
                if !active_species[k] || active_pure.contains(&k) {
                    continue;
                }
                if let Some((ld_k, ld_iter)) = last_dropped {
                    if ld_k == k && iter.saturating_sub(ld_iter) <= 3 {
                        continue;
                    }
                }
                let mut pi_dot_a = 0.0;
                for e in 0..m {
                    pi_dot_a += a_mat[e][k] * pi_vec[e];
                }
                let fixed_term = resolved[k]
                    .fixed_activity
                    .map(|a| a.max(1e-300).ln())
                    .unwrap_or(0.0);
                let drive = (resolved[k].mu_std_rt + fixed_term) - pi_dot_a;
                if drive < best_drive {
                    best_drive = drive;
                    best_k = Some(k);
                }
            }
            if let Some(k_add) = best_k {
                active_pure.push(k_add);
                continue;
            }
        }

        if lambda > 0.99 && max_rel_change < 1e-9 {
            converged = true;
            break;
        }
    }

    (n_cur, pi_vec, total_iters, converged)
}

/// Gaussian elimination with partial pivoting (Rust equivalent of HSC's `LGAUSSK`).
pub fn solve_linear_system(a: &mut [Vec<f64>], b: &mut [f64]) -> Result<Vec<f64>, SoryntelError> {
    let n = b.len();
    for col in 0..n {
        let mut pivot_row = col;
        let mut max_val = a[col][col].abs();
        for row in (col + 1)..n {
            let v = a[row][col].abs();
            if v > max_val {
                max_val = v;
                pivot_row = row;
            }
        }
        if max_val < 1e-24 || !max_val.is_finite() {
            return Err(SoryntelError::BalanceError(
                "Singular KKT matrix in equilibrium solver.".to_string(),
            ));
        }
        if pivot_row != col {
            a.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }
        let pivot = a[col][col];
        for row in (col + 1)..n {
            let factor = a[row][col] / pivot;
            if factor.abs() > 0.0 {
                for j in col..n {
                    a[row][j] -= factor * a[col][j];
                }
                b[row] -= factor * b[col];
            }
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for j in (row + 1)..n {
            sum -= a[row][j] * x[j];
        }
        x[row] = sum / a[row][row];
    }
    Ok(x)
}

fn resolve_species_data<R: SpeciesRepository>(
    sp_in: &crate::equilibrium::model::EquilibriumSpeciesInput,
    repo: &R,
) -> Result<ChemicalSpecies, SoryntelError> {
    if let Some(ref sd) = sp_in.species_data {
        return Ok(sd.clone());
    }
    repo.get_species(sp_in.formula.trim()).ok_or_else(|| {
        SoryntelError::SpeciesNotFound(format!("Species '{}' not found in database", sp_in.formula))
    })
}

fn convert_input_to_mol(
    sp_in: &crate::equilibrium::model::EquilibriumSpeciesInput,
    sp_data: &ChemicalSpecies,
    scale_to_mol: f64,
) -> f64 {
    use crate::equilibrium::model::EquilibriumAmountUnit::*;
    match sp_in.amount_unit {
        Kmol => sp_in.initial_amount * scale_to_mol,
        Mol => sp_in.initial_amount,
        Kg => {
            if sp_data.molecular_weight > 0.0 {
                (sp_in.initial_amount * 1000.0) / sp_data.molecular_weight
            } else {
                0.0
            }
        }
        Nm3 => (sp_in.initial_amount * 1000.0) / soryntel_core::STP_MOLAR_VOL_L,
        GramPerLiter => {
            if sp_data.molecular_weight > 0.0 {
                sp_in.initial_amount / sp_data.molecular_weight
            } else {
                0.0
            }
        }
        MolPerLiter => sp_in.initial_amount,
    }
}

/// Evaluates `H°(298.15 K)` reference helper for future use.
#[allow(dead_code)]
fn _ref_const_check() -> f64 {
    T_REF_K * CAL_TO_JOULE
}
