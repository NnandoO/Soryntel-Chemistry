use chem_addin_core::{
    bal, mw, parse_species, wtp, ChemAddinError, ChemEngine, PhaseTag, R_JOULE,
};

#[test]
fn units_session_and_conversions_work() {
    let mut engine = ChemEngine::in_memory().expect("in-memory sqlite db");
    assert_eq!(engine.energy_u(), "kJ");
    assert_eq!(engine.temp_u(), "C");
    assert_eq!(engine.pres_u(), "bar");

    assert!((engine.units.temp_to_kelvin(25.0).unwrap() - 298.15).abs() < 1e-9);
    assert!((engine.units.pressure_to_bar(1.0).unwrap() - 1.0).abs() < 1e-9);

    engine.set_energy_u("kcal").unwrap();
    engine.set_temp_u("K").unwrap();
    engine.set_pres_u("atm").unwrap();
    assert_eq!(engine.energy_u(), "kcal");
    assert_eq!(engine.temp_u(), "K");
    assert_eq!(engine.pres_u(), "atm");
    assert!((engine.units.temp_to_kelvin(298.15).unwrap() - 298.15).abs() < 1e-9);
    assert!((engine.units.pressure_to_bar(1.0).unwrap() - 1.01325).abs() < 1e-9);

    assert!(matches!(
        engine.units.temp_to_kelvin(0.0),
        Err(ChemAddinError::InvalidTemperature { .. })
    ));
    assert!(matches!(
        engine.set_energy_u("eV"),
        Err(ChemAddinError::InvalidUnit(_))
    ));
}

#[test]
fn parser_and_mw_handle_phases_parentheses_hydrates_and_ions() {
    let co2 = parse_species("CO2(g)").unwrap();
    assert_eq!(co2.phase, PhaseTag::Gas);
    assert_eq!(co2.composition["C"], 1.0);
    assert_eq!(co2.composition["O"], 2.0);
    assert!((mw("CO2(g)").unwrap() - 44.009).abs() < 1e-4);

    assert!((mw("H2O(l)").unwrap() - 18.015).abs() < 1e-4);
    assert!((mw("Fe2(SO4)3").unwrap() - 399.867).abs() < 1e-2);
    assert!((mw("CuSO4*5H2O").unwrap() - 249.677).abs() < 1e-2);
    assert!((mw("Ca3[Fe(CN)6]2").unwrap() - 544.138).abs() < 1e-2);

    let sulfate = parse_species("SO4(-2)(aq)").unwrap();
    assert_eq!(sulfate.phase, PhaseTag::Aqueous);
    assert_eq!(sulfate.charge, -2);
    assert_eq!(sulfate.composition["S"], 1.0);
    assert_eq!(sulfate.composition["O"], 4.0);

    let engine = ChemEngine::in_memory().unwrap();
    assert!((engine.mw("Pyrite").unwrap() - 119.965).abs() < 1e-3);

    assert!(matches!(mw(""), Err(ChemAddinError::InvalidFormula(_))));
    assert!(matches!(mw("Fe(OH"), Err(ChemAddinError::InvalidFormula(_))));
    assert!(matches!(mw("XxO2"), Err(ChemAddinError::UnknownElement(_))));
}

#[test]
fn wtp_computes_exact_stoichiometric_mass_fractions() {
    // wtp("V", "VO") = 50.9415 / (50.9415 + 15.999) * 100
    let v_in_vo = wtp("V", "VO").unwrap();
    assert!((v_in_vo - 76.09967).abs() < 1e-3, "got {v_in_vo}");

    let fe_pyrite = wtp("Fe", "FeS2").unwrap();
    let s_pyrite = wtp("S", "FeS2").unwrap();
    assert!((fe_pyrite + s_pyrite - 100.0).abs() < 1e-9);
    assert!((fe_pyrite - 46.551).abs() < 1e-2);

    // Zinc ferrite spinel moiety wt%
    let zno_pct = wtp("ZnO", "ZnO*Fe2O3").unwrap();
    assert!((zno_pct - (81.379 / (81.379 + 159.687) * 100.0)).abs() < 1e-2);

    // Hydrate water wt%
    let h2o_pct = wtp("H2O", "CuSO4*5H2O").unwrap();
    assert!((h2o_pct - (5.0 * 18.015 / 249.677 * 100.0)).abs() < 1e-2);

    // Absent element returns 0.0
    assert_eq!(wtp("Cu", "FeS2").unwrap(), 0.0);
}

#[test]
fn bal_balances_chemical_and_redox_equations_via_nalgebra_svd() {
    assert_eq!(bal("C + O2(g) = CO(g)", 1.0).unwrap(), "2 C + O2(g) = 2 CO(g)");
    assert_eq!(bal("C + O2(g) = CO(g)", 0.5).unwrap(), "C + 0.5 O2(g) = CO(g)");
    assert_eq!(
        bal("FeS2 + O2(g) = Fe2O3 + SO2(g)", 2.0).unwrap(),
        "4 FeS2 + 11 O2(g) = 2 Fe2O3 + 8 SO2(g)"
    );
    assert_eq!(
        bal("Fe+2 + MnO4(-1) + H+ = Fe+3 + Mn+2 + H2O(l)", 1.0).unwrap(),
        "5 Fe+2 + MnO4(-1) + 8 H+ = 5 Fe+3 + Mn+2 + 4 H2O(l)"
    );

    // Underdetermined system must fail cleanly.
    assert!(matches!(
        bal("C + O2(g) = CO(g) + CO2(g)", 1.0),
        Err(ChemAddinError::UnbalanceableReaction(_))
    ));
    // Impossible reaction (element missing on one side) must fail cleanly.
    assert!(matches!(
        bal("Fe + O2(g) = CO2(g)", 1.0),
        Err(ChemAddinError::UnbalanceableReaction(_))
    ));
}

#[test]
fn thermo_species_h_s_g_k_and_sqlite_seeds_are_consistent() {
    let mut engine = ChemEngine::in_memory().unwrap();

    // Standard 298.15 K (25 C) seed identity checks
    assert!((engine.h("H2O", 25.0).unwrap() - (-285.830)).abs() < 1e-6);
    assert!((engine.h("H2O(l)", 25.0).unwrap() - (-285.830)).abs() < 1e-6);
    assert!((engine.h("H2O(g)", 25.0).unwrap() - (-241.826)).abs() < 1e-6);
    assert!((engine.s("H2O(l)", 25.0).unwrap() - 69.950).abs() < 1e-6);
    assert!((engine.h("CO2(g)", 25.0).unwrap() - (-393.510)).abs() < 1e-6);
    assert!((engine.h("CO(g)", 25.0).unwrap() - (-110.530)).abs() < 1e-6);
    assert!((engine.h("C", 25.0).unwrap()).abs() < 1e-9);
    assert!((engine.h("O2(g)", 25.0).unwrap()).abs() < 1e-9);
    assert!((engine.h("Fe", 25.0).unwrap()).abs() < 1e-9);
    assert!((engine.h("S", 25.0).unwrap()).abs() < 1e-9);
    assert!((engine.h("FeS", 25.0).unwrap() - (-101.670)).abs() < 1e-6);
    assert!((engine.h("Pyrite", 25.0).unwrap() - (-171.540)).abs() < 1e-6);

    // G(T) = H(T) - T(K)*S(T)/1000
    let t_c = 100.0;
    let h_co2 = engine.h("CO2(g)", t_c).unwrap();
    let s_co2 = engine.s("CO2(g)", t_c).unwrap();
    let g_co2 = engine.g("CO2(g)", t_c).unwrap();
    assert!((h_co2 - (-390.56)).abs() < 0.1, "CO2 H(100C) = {h_co2}");
    assert!((g_co2 - (h_co2 - (t_c + 273.15) * s_co2 / 1000.0)).abs() < 1e-9);

    // Piecewise phase-transition Cp integration (Fe at 1000 C spans alpha -> beta -> gamma)
    let h_fe_1000 = engine.h("Fe", 1000.0).unwrap();
    assert!(h_fe_1000 > 30.0 && h_fe_1000 < 45.0, "Fe H(1000C) = {h_fe_1000}");

    // Reaction auto-detection & K = exp(-dG / RT)
    let rxn = "C + CO2(g) = 2 CO(g)";
    let dg_kj = engine.g(rxn, 800.0).unwrap();
    let k_800 = engine.k(rxn, 800.0).unwrap();
    let expected_k = (-(dg_kj * 1000.0) / (R_JOULE * (800.0 + 273.15))).exp();
    assert!((k_800 - expected_k).abs() / expected_k < 1e-12);
    assert!(k_800 > 1.0, "Boudouard equilibrium favors CO above ~700 C");

    // Non-reaction passed to K() returns InvalidReaction
    assert!(matches!(
        engine.k("CO2(g)", 25.0),
        Err(ChemAddinError::InvalidReaction(_))
    ));

    // Unit conversion consistency: switch to kcal and Kelvin
    engine.set_energy_u("kcal").unwrap();
    engine.set_temp_u("K").unwrap();
    let h_co2_kcal = engine.h("CO2(g)", 373.15).unwrap();
    let s_co2_cal = engine.s("CO2(g)", 373.15).unwrap();
    let g_co2_kcal = engine.g("CO2(g)", 373.15).unwrap();
    assert!((h_co2_kcal - h_co2 / 4.184).abs() < 1e-9);
    assert!((s_co2_cal - s_co2 / 4.184).abs() < 1e-9);
    assert!((g_co2_kcal - g_co2 / 4.184).abs() < 1e-9);

    // Equilibrium constant K is dimensionless and invariant to unit switches
    let k_800_after_switch = engine.k(rxn, 1073.15).unwrap();
    assert!((k_800_after_switch - k_800).abs() / k_800 < 1e-12);
}

#[test]
fn sqlite_persistent_file_roundtrip_works() {
    let path = std::env::temp_dir().join(format!(
        "chem_addin_core_test_{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    {
        let engine = ChemEngine::open_sqlite(&path).unwrap();
        assert!((engine.h("Pyrite", 25.0).unwrap() - (-171.540)).abs() < 1e-6);
    }
    {
        let reopened = ChemEngine::open_sqlite(&path).unwrap();
        assert!((reopened.h("FeS2", 25.0).unwrap() - (-171.540)).abs() < 1e-6);
    }

    let _ = std::fs::remove_file(&path);
}
