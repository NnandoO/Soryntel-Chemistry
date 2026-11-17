use soryntel_application::SoryntelApplication as App;
use soryntel_core::{EnergyUnit, ResultFormat, SoryntelError, TemperatureUnit};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::{
    equilibrium::{EquilibriumParams, EquilibriumSystem},
    CalculationRequest,
};

fn request() -> CalculationRequest {
    CalculationRequest {
        input: "CO(g)".into(),
        t_from: 25.0,
        t_to: 25.0,
        t_step: 1.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
    }
}

#[test]
fn reaction_rejects_nonfinite_and_excessive_grids() {
    let db = ThermoDatabase::with_open_defaults();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for field in 0..3 {
            let mut req = request();
            match field {
                0 => req.t_from = value,
                1 => req.t_to = value,
                _ => req.t_step = value,
            }
            assert!(matches!(
                App::calculate_reaction(&db, &req),
                Err(SoryntelError::InvalidRange { .. })
            ));
        }
    }
    let mut req = request();
    req.t_to = 1000.0;
    req.t_step = 1e-300;
    assert!(matches!(
        App::calculate_reaction(&db, &req),
        Err(SoryntelError::InvalidRange { .. })
    ));
}

#[test]
fn equilibrium_rejects_invalid_temperature_and_pressure_before_solving() {
    let db = ThermoDatabase::with_open_defaults();
    let sys = EquilibriumSystem {
        title: "Validación".into(),
        phases: vec![],
    };
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut params = EquilibriumParams::default();
        params.num_indep_vars = 0;
        params.pressure_range.initial = value;
        assert!(matches!(
            App::solve_equilibrium_sweep(&db, &sys, &params),
            Err(SoryntelError::InvalidPressure(_))
        ));
        params.pressure_range.initial = 1.0;
        params.temp_range.initial = value - 273.15;
        assert!(matches!(
            App::solve_equilibrium_sweep(&db, &sys, &params),
            Err(SoryntelError::InvalidTemperature(_))
        ));
    }
}

#[test]
fn species_gibbs_identity_and_temperature_units_are_consistent() {
    let db = ThermoDatabase::with_open_defaults();
    let mut req = request();
    let c = App::calculate_reaction(&db, &req).unwrap();
    req.temp_unit = TemperatureUnit::Kelvin;
    req.t_from = 298.15;
    req.t_to = 298.15;
    let k = App::calculate_reaction(&db, &req).unwrap();
    let a = &c.substance_rows[0];
    let b = &k.substance_rows[0];
    assert!((a.h - b.h).abs() < 1e-10);
    assert!((a.g - (a.h - 298.15 * a.s / 1000.0)).abs() < 1e-9);
    assert!(a.cp.is_finite() && a.cp > 0.0);
}

#[test]
fn heat_loss_rejects_corrupt_gas_property_tables() {
    use soryntel_thermo::heat_loss::{model::ColumnType, presets::preset_smelting1};
    for table in [
        vec![(500.0, f64::NAN)],
        vec![(1000.0, 0.2), (500.0, 0.1)],
        vec![(500.0, 0.1), (500.0, 0.2)],
    ] {
        let mut wb = preset_smelting1();
        let column = wb.sheets[0]
            .columns
            .iter_mut()
            .find(|c| c.column_type == ColumnType::Surface)
            .unwrap();
        column.hr_fixed = false;
        column.iterative_gas_eps_table = table;
        assert!(matches!(
            App::solve_heat_loss_workbook(&wb),
            Err(SoryntelError::HeatLossError(_))
        ));
    }
}

#[test]
fn water_rejects_invalid_requests_at_application_boundary() {
    let db = ThermoDatabase::with_open_defaults();
    for value in [f64::NAN, f64::INFINITY, -1.0] {
        let mut req = App::default_water_request();
        req.amount = value;
        assert!(App::try_calculate_water(&db, &req).is_err());
        req.amount = 1.0;
        req.point1.pressure = value;
        assert!(App::try_calculate_water(&db, &req).is_err());
    }
    let mut req = App::default_water_request();
    req.point1.temp = -300.0;
    assert!(App::try_calculate_water(&db, &req).is_err());
    req.point1.temp = 25.0;
    req.point2.mixture_pct = 101.0;
    assert!(App::try_calculate_water(&db, &req).is_err());
}

#[test]
fn saturation_pressure_matches_nist_table_1() {
    // NISTIR 5078 table 1 (IAPWS-95), MPa; correlation checked to 0.05%.
    // https://www.nist.gov/document/nistir5078-tab1pdf
    use soryntel_thermo::water::WaterPhaseMode;
    let db = ThermoDatabase::with_open_defaults();
    for (temp_c, pressure_mpa) in [(10.0, 0.0012282), (25.0, 0.0031699), (50.0, 0.012352)] {
        let mut req = App::default_water_request();
        req.point1.temp = temp_c;
        req.point1.phase = WaterPhaseMode::SatGasLiquid;
        let res = App::try_calculate_water(&db, &req).unwrap();
        let actual_mpa = res.point1.pressure_bar / 10.0;
        assert!((actual_mpa / pressure_mpa - 1.0).abs() < 5e-4, "T={temp_c}: {actual_mpa}");
    }
}

fn synthetic_equilibrium(gamma: &str) -> soryntel_thermo::equilibrium::EquilibriumSystem {
    use soryntel_thermo::{ChemicalSpecies, Phase, SpeciesThermoInterval, KelleyCoefficients};
    use soryntel_thermo::equilibrium::*;
    let species = ["A", "B"].iter().enumerate().map(|(i, name)| EquilibriumSpeciesInput {
        formula: name.to_string(), feed_temp_c: 25.0,
        activity_coeff_expr: if i == 0 { "1.0" } else { gamma }.into(),
        fixed_activity: None, discharge_coeff: 0.0, amount_unit: EquilibriumAmountUnit::Mol,
        initial_amount: 1.0, add_x_amount: 0.0, add_z_amount: 0.0, remove_percent: 0.0, excluded: false,
        stoichiometry: [("H".into(), 2.0)].into_iter().collect(),
        species_data: Some(ChemicalSpecies {
            formula: name.to_string(), name: "Especie sintética de contraste analítico".into(),
            phase: Phase::Gas, molecular_weight: 2.0, density_g_cm3: 0.0, charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 200.0, t_max: 1000.0, phase: Phase::Gas,
                kelley: KelleyCoefficients { a: 30.0, b: 0.0, c: 0.0, d: 0.0 },
                h_or_delta_h_tr: if i == 0 { 0.0 } else { soryntel_core::R_JOULE * 298.15 * 2.0_f64.ln() },
                s_or_delta_s_tr: 100.0, reference: "Caso analítico sintético A ↔ B, ΔG = RT ln(2)".into(),
            }],
        }),
    }).collect();
    EquilibriumSystem { title: "Contraste analítico".into(), phases: vec![EquilibriumPhase {
        name: "GAS".into(), model: SolutionModel::IdealMixture, is_gas_phase: true,
        electrode_role: String::new(), capacitance_farads: 0.0, species,
    }] }
}

#[test]
fn equilibrium_matches_analytic_mass_action_and_element_conservation() {
    let db = ThermoDatabase::with_open_defaults();
    let mut params = EquilibriumParams::default();
    params.num_indep_vars = 0;
    params.temp_range.initial = 25.0;
    for pressure in [0.1, 1.0, 10.0] {
        params.pressure_range.initial = pressure;
        let res = App::solve_equilibrium_sweep(&db, &synthetic_equilibrium("1.0"), &params).unwrap();
        let pt = &res.points[0];
        assert!(pt.converged);
        let a = pt.species[0].equil_amount_mol;
        let b = pt.species[1].equil_amount_mol;
        assert!((a + b - 2.0).abs() < 1e-8);
        assert!((b / a - 0.5).abs() < 1e-7);
    }
}

#[test]
fn nonideal_equilibrium_does_not_claim_convergence_before_activity_iteration() {
    let db = ThermoDatabase::with_open_defaults();
    let mut params = EquilibriumParams::default();
    params.num_indep_vars = 0;
    params.temp_range.initial = 25.0;
    params.ac_steps = 1;
    let sys = synthetic_equilibrium("2.0");
    let first = App::solve_equilibrium_sweep(&db, &sys, &params).unwrap();
    assert!(!first.points[0].converged);
    params.ac_steps = 40;
    let res = App::solve_equilibrium_sweep(&db, &sys, &params).unwrap();
    let pt = &res.points[0];
    assert!(pt.converged);
    assert!((pt.species[1].equil_amount_mol / pt.species[0].equil_amount_mol - 0.25).abs() < 1e-7);
}

#[test]
fn benson_rejects_unknown_groups_instead_of_fabricating_properties() {
    let db = ThermoDatabase::with_open_defaults();
    let molecule = App::parse_benson_smiles("C").unwrap();
    let groups = vec![("grupo-sin-parametros".into(), 1)];
    let error = App::estimate_benson_properties(&db, &molecule, Some(&groups), None).unwrap_err();
    assert!(error.contains("grupo-sin-parametros"));
    assert!(App::estimate_benson_properties(&db, &molecule, None, None).is_ok());
}

#[test]
fn equilibrium_rejects_invalid_and_oversized_sweeps_before_allocation() {
    use soryntel_thermo::equilibrium::SweepVariable;
    let db = ThermoDatabase::with_open_defaults();
    let sys = synthetic_equilibrium("1.0");
    let mut params = EquilibriumParams::default();
    params.temp_range.steps = usize::MAX;
    assert!(App::solve_equilibrium_sweep(&db, &sys, &params).is_err());
    params.temp_range.steps = 1000;
    params.num_indep_vars = 2;
    params.secondary_var = Some(SweepVariable::PressureBar);
    params.pressure_range.steps = 1000;
    assert!(App::solve_equilibrium_sweep(&db, &sys, &params).is_err());
    params.num_indep_vars = 1;
    params.temp_range.steps = 2;
    params.temp_range.log_scale = true;
    params.temp_range.initial = 0.0;
    assert!(App::solve_equilibrium_sweep(&db, &sys, &params).is_err());
    params.temp_range.log_scale = false;
    params.temp_range.initial = f64::NAN;
    assert!(App::solve_equilibrium_sweep(&db, &sys, &params).is_err());
}

fn synthetic_hydrogen(formula: &str, h: f64, entropy: f64, cp: f64) -> soryntel_thermo::ChemicalSpecies {
    use soryntel_thermo::{ChemicalSpecies, Phase, SpeciesThermoInterval, KelleyCoefficients};
    ChemicalSpecies {
        formula: formula.into(), name: "Caso analítico".into(), phase: Phase::Gas,
        molecular_weight: if formula == "H(g)" { 1.0 } else { 2.0 }, density_g_cm3: 0.0, charge: 0,
        intervals: vec![SpeciesThermoInterval {
            t_min: 100.0, t_max: 2000.0, phase: Phase::Gas,
            kelley: KelleyCoefficients { a: cp, b: 0.0, c: 0.0, d: 0.0 },
            h_or_delta_h_tr: h, s_or_delta_s_tr: entropy, reference: "Solución analítica sintética".into(),
        }],
    }
}

#[test]
fn module1_reaction_matches_analytic_h_s_g_and_log_k_in_both_energy_systems() {
    let mut db = ThermoDatabase::with_open_defaults();
    db.insert_species(synthetic_hydrogen("H2(g)", 0.0, 100.0, 30.0));
    db.insert_species(synthetic_hydrogen("H(g)", 10000.0, 60.0, 20.0));
    let mut req = request();
    req.input = "H2(g) = 2H(g)".into();
    req.t_from = 226.85;
    req.t_to = 226.85;
    let expected_h = 20000.0 + 10.0 * (500.0 - 298.15);
    let expected_s = 20.0 + 10.0 * (500.0_f64 / 298.15).ln();
    let expected_g = expected_h - 500.0 * expected_s;
    for unit in [EnergyUnit::Kilojoules, EnergyUnit::Kilocalories] {
        req.energy_unit = unit;
        let response = App::calculate_reaction(&db, &req).unwrap();
        let row = &response.reaction_rows[0];
        assert!((row.delta_h - expected_h * unit.from_joules_factor()).abs() < 1e-9);
        assert!((row.delta_s - expected_s * unit.entropy_from_joules_factor()).abs() < 1e-9);
        assert!((row.delta_g - expected_g * unit.from_joules_factor()).abs() < 1e-9);
        assert!((row.log10_k + expected_g / (soryntel_core::R_JOULE * 500.0 * 10.0_f64.ln())).abs() < 1e-10);
    }
}

#[test]
fn module1_integrates_up_down_and_across_phase_transition() {
    let mut db = ThermoDatabase::with_open_defaults();
    let mut sp = synthetic_hydrogen("H2(g)", 0.0, 100.0, 30.0);
    sp.intervals[0].t_max = 500.0;
    let mut next = sp.intervals[0].clone();
    next.t_min = 500.0; next.t_max = 1000.0;
    next.kelley.a = 40.0; next.h_or_delta_h_tr = 6000.0; next.s_or_delta_s_tr = 12.0;
    sp.intervals.push(next);
    db.insert_species(sp);
    for t in [200.0, 298.15, 600.0] {
        let mut req = request(); req.input = "H2(g)".into(); req.temp_unit = TemperatureUnit::Kelvin;
        req.t_from = t; req.t_to = t;
        let result = App::calculate_reaction(&db, &req).unwrap();
        let row = &result.substance_rows[0];
        let h = if t < 500.0 {30.0 * (t - 298.15)} else {30.0 * (500.0-298.15) + 6000.0 + 40.0 * (t-500.0)};
        assert!((row.h * 1000.0 - h).abs() < 1e-8);
    }
}

#[test]
fn module1_delta_requires_reference_element_instead_of_returning_partial_formation() {
    let mut db = ThermoDatabase::with_open_defaults();
    db.insert_species(synthetic_hydrogen("Xe2(g)", 0.0, 100.0, 30.0));
    let mut req = request(); req.input = "Xe2(g)".into(); req.result_format = ResultFormat::Delta;
    let error = App::calculate_reaction(&db, &req).unwrap_err();
    assert!(matches!(error, SoryntelError::SpeciesNotFound(_)));
}

#[test]
fn module1_co_reference_agrees_with_nist_codate_and_chase() {
    // NIST WebBook CO, https://webbook.nist.gov/cgi/cbook.cgi?ID=C630080&Mask=1
    // Published Hf = -110.53 +/- 0.17 kJ/mol; S(1 bar) = 197.660 +/- 0.004 J/(mol K).
    let db = ThermoDatabase::with_open_defaults();
    let response = App::calculate_reaction(&db, &request()).unwrap();
    let row = &response.substance_rows[0];
    assert!((row.h + 110.53).abs() <= 0.17);
    assert!((row.s - 197.660).abs() <= 0.004);
}
