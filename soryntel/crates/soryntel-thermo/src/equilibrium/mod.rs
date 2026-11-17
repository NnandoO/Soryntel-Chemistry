//! Module 4: Equilibrium Calculations (Gibbs Energy Minimization).
//!
//! Implements:
//! - White, Johnson & Dantzig (1958) multi-phase Gibbs minimization + active-set pure phases
//! - Redlich-Kister (`Ga-As`, `Al-Zn`), aqueous Debye-Huckel (`Aqua` + 55.508435 molality conversion),
//!   and Column-C formula activity coefficients
//! - Parameter sweeps (0D/1D/2D), Adiabatic (`N,H,P`), Constant Volume (`N,P,V`),
//!   Target Calculations, Infinite Phase, Fixed Activity, Remove %, Cell Mode (`N,T,P,Q`),
//!   and Cp extrapolation `Warnings`.

pub mod activity;
pub mod gem_solver;
pub mod model;
pub mod outer_solvers;

pub use gem_solver::solve_single_point;
pub use model::*;
pub use outer_solvers::{collect_cp_warnings, generate_axis_values, solve_equilibrium_sweep};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equilibrium::activity::{
        apply_al_zn_redlich_kister, eval_column_c_formula, redlich_kister_binary, R_HSC_APP_A,
    };
    use crate::{ChemicalSpecies, KelleyCoefficients, Phase, SpeciesRepository, SpeciesThermoInterval};
    use approx::assert_relative_eq;
    use soryntel_core::{CAL_TO_JOULE, T_REF_K};
    use std::collections::BTreeMap;

    struct DummyRepo(BTreeMap<String, ChemicalSpecies>);
    impl SpeciesRepository for DummyRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get(formula).cloned()
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            self.0.get(element_symbol).cloned()
        }
    }

    fn make_const_species(
        formula: &str,
        phase: Phase,
        hf_cal_mol: f64,
        s_cal_mol_k: f64,
        cp_cal_mol_k: f64,
    ) -> ChemicalSpecies {
        ChemicalSpecies {
            formula: formula.to_string(),
            name: formula.to_string(),
            phase,
            molecular_weight: 28.0,
            density_g_cm3: 1.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 200.0,
                t_max: 3000.0,
                phase,
                kelley: KelleyCoefficients {
                    a: cp_cal_mol_k * CAL_TO_JOULE,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: hf_cal_mol * CAL_TO_JOULE,
                s_or_delta_s_tr: s_cal_mol_k * CAL_TO_JOULE,
                reference: "Test".to_string(),
            }],
        }
    }

    #[test]
    fn test_redlich_kister_ga_as_appendix_b() {
        let t_k = 1000.0;
        let k0 = -25503.6 - 4.3109 * t_k;
        let k1 = 5174.7;
        let rt = R_HSC_APP_A * t_k;
        let (ln_f_ga, ln_f_as) = redlich_kister_binary(0.5, 0.5, &[k0, k1], rt);
        // At x=0.5, (x_i - x_j) = 0 so n=0 gives k0 * 0.25 / RT, n=1 gives +/- k1 * 0.25 / RT
        let expected_ga = (0.25 * k0 + 0.25 * k1) / rt;
        let expected_as = (0.25 * k0 - 0.25 * k1) / rt;
        assert_relative_eq!(ln_f_ga, expected_ga, epsilon = 1e-12);
        assert_relative_eq!(ln_f_as, expected_as, epsilon = 1e-12);
    }

    #[test]
    fn test_column_c_formula_eval() {
        let mut x_map = BTreeMap::new();
        x_map.insert(24, 0.2);
        x_map.insert(25, 0.1);
        let val = eval_column_c_formula("=EXP(8495/T - 2.653)", 1000.0, &x_map).unwrap();
        assert_relative_eq!(val, (8495.0 / 1000.0 - 2.653f64).exp(), epsilon = 1e-10);
        let ln_f_val = eval_column_c_formula("Ln(f) = 0.69 + 56.8*X_24 + 5.45*X_25", 1000.0, &x_map).unwrap();
        assert_relative_eq!(ln_f_val, (0.69 + 56.8 * 0.2 + 5.45 * 0.1f64).exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_wjd_ammonia_equilibrium_mass_balance() {
        let mut map = BTreeMap::new();
        // N2(g), H2(g), NH3(g) realistic thermochemical constants at 298.15 K
        map.insert("N2(g)".into(), make_const_species("N2(g)", Phase::Gas, 0.0, 45.77, 6.96));
        map.insert("H2(g)".into(), make_const_species("H2(g)", Phase::Gas, 0.0, 31.21, 6.89));
        map.insert("NH3(g)".into(), make_const_species("NH3(g)", Phase::Gas, -10970.0, 46.05, 8.52));
        let repo = DummyRepo(map);

        let sys = EquilibriumSystem {
            title: "NH3".into(),
            phases: vec![EquilibriumPhase {
                name: "GAS".into(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: true,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: vec![
                    EquilibriumSpeciesInput {
                        formula: "N2(g)".into(),
                        feed_temp_c: 25.0,
                        activity_coeff_expr: "1.0".into(),
                        fixed_activity: None,
                        discharge_coeff: 0.0,
                        amount_unit: EquilibriumAmountUnit::Mol,
                        initial_amount: 1.0,
                        add_x_amount: 0.0,
                        add_z_amount: 0.0,
                        remove_percent: 0.0,
                        excluded: false,
                        species_data: None,
                        stoichiometry: BTreeMap::new(),
                    },
                    EquilibriumSpeciesInput {
                        formula: "H2(g)".into(),
                        feed_temp_c: 25.0,
                        activity_coeff_expr: "1.0".into(),
                        fixed_activity: None,
                        discharge_coeff: 0.0,
                        amount_unit: EquilibriumAmountUnit::Mol,
                        initial_amount: 3.0,
                        add_x_amount: 0.0,
                        add_z_amount: 0.0,
                        remove_percent: 0.0,
                        excluded: false,
                        species_data: None,
                        stoichiometry: BTreeMap::new(),
                    },
                    EquilibriumSpeciesInput {
                        formula: "NH3(g)".into(),
                        feed_temp_c: 25.0,
                        activity_coeff_expr: "1.0".into(),
                        fixed_activity: None,
                        discharge_coeff: 0.0,
                        amount_unit: EquilibriumAmountUnit::Mol,
                        initial_amount: 0.0,
                        add_x_amount: 0.0,
                        add_z_amount: 0.0,
                        remove_percent: 0.0,
                        excluded: false,
                        species_data: None,
                        stoichiometry: BTreeMap::new(),
                    },
                ],
            }],
        };

        let params = EquilibriumParams {
            num_indep_vars: 0,
            temp_range: AxisRange {
                initial: 25.0,
                final_val: 25.0,
                steps: 1,
                log_scale: false,
            },
            pressure_range: AxisRange {
                initial: 1.0,
                final_val: 1.0,
                steps: 1,
                log_scale: false,
            },
            ..Default::default()
        };

        let pt = solve_single_point(&sys, &params, &repo, T_REF_K, 1.0, 0, 25.0, None, None, 0.0).unwrap();
        assert!(pt.converged, "WJD should converge");
        assert!(pt.max_mass_balance_err < 1e-10, "Mass balance error: {}", pt.max_mass_balance_err);
        // At 298.15 K, 1 N2 + 3 H2 -> almost complete conversion to ~2 mol NH3
        let n_nh3 = pt.species[2].equil_amount_mol;
        assert!(n_nh3 > 1.9, "Expected >1.9 mol NH3 at 298 K, got {}", n_nh3);
    }

    #[test]
    fn test_pure_phase_active_set_caco3_decomposition() {
        let mut map = BTreeMap::new();
        map.insert("N2(g)".into(), make_const_species("N2(g)", Phase::Gas, 0.0, 45.77, 6.96));
        map.insert("CO2(g)".into(), make_const_species("CO2(g)", Phase::Gas, -94051.0, 51.07, 8.87));
        map.insert("CaO".into(), make_const_species("CaO", Phase::Solid, -151790.0, 9.50, 10.23));
        map.insert("CaCO3".into(), make_const_species("CaCO3", Phase::Solid, -288450.0, 22.20, 19.57));
        let repo = DummyRepo(map);

        let sp_in = |f: &str, amt: f64| EquilibriumSpeciesInput {
            formula: f.into(),
            feed_temp_c: 25.0,
            activity_coeff_expr: "1.0".into(),
            fixed_activity: None,
            discharge_coeff: 0.0,
            amount_unit: EquilibriumAmountUnit::Mol,
            initial_amount: amt,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        };

        let sys = EquilibriumSystem {
            title: "CaCO3".into(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".into(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("N2(g)", 0.1), sp_in("CO2(g)", 0.0)],
                },
                EquilibriumPhase {
                    name: "SOLID".into(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("CaO", 0.0), sp_in("CaCO3", 1.0)],
                },
            ],
        };

        let params = EquilibriumParams::default();
        // At 298.15 K: CaCO3 stays solid (~1.0 mol)
        let pt_low = solve_single_point(&sys, &params, &repo, 298.15, 1.0, 0, 25.0, None, None, 0.0).unwrap();
        assert!(pt_low.max_mass_balance_err < 1e-10);
        assert_relative_eq!(pt_low.species[3].equil_amount_mol, 1.0, epsilon = 1e-4);

        // At 1300 K: CaCO3 decomposes to CaO + CO2(g)
        let pt_high = solve_single_point(&sys, &params, &repo, 1300.0, 1.0, 0, 1026.85, None, None, 0.0).unwrap();
        assert!(pt_high.max_mass_balance_err < 1e-10);
        assert_relative_eq!(pt_high.species[2].equil_amount_mol, 1.0, epsilon = 1e-4);
        assert_relative_eq!(pt_high.species[1].equil_amount_mol, 1.0, epsilon = 1e-4);
    }

    #[test]
    fn test_redlich_kister_al_zn_appendix_c_all_modes() {
        let t_k = 800.0;
        let rt = R_HSC_APP_A * t_k;

        // 1. Liquid: Al(l), Zn(l)
        let mut ln_g_liq = [0.0; 2];
        apply_al_zn_redlich_kister(&["Al(l)".into(), "Zn(l)".into()], &[0.4, 0.6], t_k, &mut ln_g_liq);
        let (exp_al_l, exp_zn_l) = redlich_kister_binary(
            0.4,
            0.6,
            &[10483.5 - 4.74442 * t_k, -210.4 + 0.85139 * t_k],
            rt,
        );
        assert_relative_eq!(ln_g_liq[0], exp_al_l, epsilon = 1e-12);
        assert_relative_eq!(ln_g_liq[1], exp_zn_l, epsilon = 1e-12);

        // 2. Solid FCC: Al(FCC), Zn(FCC)
        let mut ln_g_fcc = [0.0; 2];
        apply_al_zn_redlich_kister(
            &["Al(FCC)".into(), "Zn(FCC)".into()],
            &[0.7, 0.3],
            t_k,
            &mut ln_g_fcc,
        );
        let (exp_al_fcc, exp_zn_fcc) = redlich_kister_binary(
            0.7,
            0.3,
            &[11014.8 - 3.71812 * t_k, 1546.9 + 1.60179 * t_k, -308.7],
            rt,
        );
        assert_relative_eq!(ln_g_fcc[0], exp_al_fcc, epsilon = 1e-12);
        assert_relative_eq!(ln_g_fcc[1], exp_zn_fcc, epsilon = 1e-12);

        // 3. Solid HCP: Al(HCP), Zn(HCP)
        let mut ln_g_hcp = [0.0; 2];
        apply_al_zn_redlich_kister(
            &["Al(HCP)".into(), "Zn(HCP)".into()],
            &[0.1, 0.9],
            t_k,
            &mut ln_g_hcp,
        );
        let (exp_al_hcp, exp_zn_hcp) =
            redlich_kister_binary(0.1, 0.9, &[17047.8 - 10.37525 * t_k], rt);
        assert_relative_eq!(ln_g_hcp[0], exp_al_hcp, epsilon = 1e-12);
        assert_relative_eq!(ln_g_hcp[1], exp_zn_hcp, epsilon = 1e-12);
    }

    #[test]
    fn test_fixed_activity_co2_dissolution_zero_initial_c() {
        let mut map = BTreeMap::new();
        map.insert("N2(g)".into(), make_const_species("N2(g)", Phase::Gas, 0.0, 45.77, 6.96));
        map.insert("O2(g)".into(), make_const_species("O2(g)", Phase::Gas, 0.0, 49.00, 7.02));
        map.insert("CO2(g)".into(), make_const_species("CO2(g)", Phase::Gas, -94051.0, 51.07, 8.87));
        map.insert("H2O".into(), make_const_species("H2O", Phase::AqueousNeutral, -68315.0, 16.71, 18.0));
        map.insert("CO2(a)".into(), make_const_species("CO2(a)", Phase::AqueousNeutral, -98900.0, 28.1, 35.0));
        let repo = DummyRepo(map);

        let sp_in = |f: &str, amt: f64, fa: Option<f64>| EquilibriumSpeciesInput {
            formula: f.into(),
            feed_temp_c: 25.0,
            activity_coeff_expr: "1.0".into(),
            fixed_activity: fa,
            discharge_coeff: 0.0,
            amount_unit: EquilibriumAmountUnit::Kmol,
            initial_amount: amt,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        };

        let sys = EquilibriumSystem {
            title: "CO2 Fixed Activity".into(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".into(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        sp_in("N2(g)", 0.79, None),
                        sp_in("O2(g)", 0.21, None),
                        sp_in("CO2(g)", 0.0, Some(0.0003)),
                    ],
                },
                EquilibriumPhase {
                    name: "AQUA".into(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("H2O", 100.0, None), sp_in("CO2(a)", 0.0, None)],
                },
            ],
        };

        let params = EquilibriumParams {
            calc_mode: CalculationMode::FixedActivity,
            ..Default::default()
        };

        let pt = solve_single_point(&sys, &params, &repo, 298.15, 1.0, 0, 25.0, None, None, 0.0).unwrap();
        let co2_g = &pt.species[2];
        let co2_a = &pt.species[4];
        assert_relative_eq!(co2_g.mole_fraction, 0.0003, epsilon = 1e-6);
        assert_relative_eq!(co2_g.activity, 0.0003, epsilon = 1e-6);
        assert!(co2_a.equil_amount_mol > 0.0, "CO2(a) should dissolve from fixed-activity CO2(g) buffer");
    }

    #[test]
    fn test_infinite_phase_rescaling_and_cell_mode_voltage() {
        let mut map = BTreeMap::new();
        map.insert("Pb".into(), make_const_species("Pb", Phase::Solid, 0.0, 15.49, 6.4));
        map.insert("PbO2".into(), make_const_species("PbO2", Phase::Solid, -66300.0, 16.4, 14.8));
        map.insert("PbSO4".into(), make_const_species("PbSO4", Phase::Solid, -219870.0, 35.5, 24.7));
        map.insert("H2O".into(), make_const_species("H2O", Phase::AqueousNeutral, -68315.0, 16.71, 18.0));
        map.insert("H2SO4(a)".into(), make_const_species("H2SO4(a)", Phase::AqueousNeutral, -217320.0, 4.8, 20.0));
        let repo = DummyRepo(map);

        let sp_in = |f: &str, amt: f64, dis: f64| EquilibriumSpeciesInput {
            formula: f.into(),
            feed_temp_c: 25.0,
            activity_coeff_expr: "1.0".into(),
            fixed_activity: None,
            discharge_coeff: dis,
            amount_unit: EquilibriumAmountUnit::Kmol,
            initial_amount: amt,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        };

        let sys = EquilibriumSystem {
            title: "Lead-Acid Cell".into(),
            phases: vec![
                EquilibriumPhase {
                    name: "ANODE".into(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: "anode".into(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("Pb", 5.0, -2.0)],
                },
                EquilibriumPhase {
                    name: "CATHODE".into(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: "cathode".into(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("PbO2", 5.0, 0.0)],
                },
                EquilibriumPhase {
                    name: "ELECTROLYTE".into(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("H2O", 50.0, 0.0), sp_in("H2SO4(a)", 10.0, 0.0)],
                },
                EquilibriumPhase {
                    name: "SOLID_PRODUCT".into(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![sp_in("PbSO4", 0.1, 0.0)],
                },
            ],
        };

        let params = EquilibriumParams {
            calc_mode: CalculationMode::CellMode,
            thermo_mode: ThermodynamicMode::Ntpq,
            ..Default::default()
        };

        // At Q = 2.0 kmol e-, 1.0 kmol of Pb should oxidize (5.0 -> 4.0 kmol) and cell voltage ~2.0 V
        let pt = solve_single_point(&sys, &params, &repo, 298.15, 1.0, 1, 2.0, None, None, 2.0).unwrap();
        assert_relative_eq!(pt.species[0].equil_amount_mol / 1000.0, 4.0, epsilon = 1e-4);
        let v = pt.cell_voltage_v.expect("Cell voltage should be present");
        assert!(v > 1.8 && v < 2.3, "Expected lead-acid voltage ~2.04 V, got {}", v);
    }
}
