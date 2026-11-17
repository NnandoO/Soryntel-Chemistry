use approx::assert_relative_eq;
use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::{calculate, CalculationRequest};

#[test]
fn test_al2o3_at_0c() {
    let db = ThermoDatabase::with_open_defaults();
    let req = CalculationRequest {
        input: "Al2O3".into(),
        t_from: 0.0,
        t_to: 2000.0,
        t_step: 100.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
    };
    let resp = calculate(&req, &db).unwrap();
    let row0 = &resp.substance_rows[0];
    assert_relative_eq!(row0.cp, 72.164, epsilon = 0.05);
    assert_relative_eq!(row0.h, -1677.558, epsilon = 0.05);
    assert_relative_eq!(row0.s, 44.407, epsilon = 0.05);
    assert_relative_eq!(row0.g, -1689.688, epsilon = 0.05);
    assert_relative_eq!(resp.species_data[0].volume, 25.715, epsilon = 0.01);
}

#[test]
fn test_mg_vaporization_with_melting_transition() {
    let db = ThermoDatabase::with_open_defaults();
    let req = CalculationRequest {
        input: "Mg = Mg(g)".into(),
        t_from: 500.0,
        t_to: 1500.0,
        t_step: 50.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
    };
    let resp = calculate(&req, &db).unwrap();
    let r500 = resp
        .reaction_rows
        .iter()
        .find(|r| (r.temperature - 500.0).abs() < 1e-6)
        .unwrap();
    assert_relative_eq!(r500.delta_h, 143.884, epsilon = 0.05);
    assert_relative_eq!(r500.delta_s, 110.035, epsilon = 0.05);
    assert_relative_eq!(r500.delta_g, 58.811, epsilon = 0.05);

    let r650 = resp
        .reaction_rows
        .iter()
        .find(|r| (r.temperature - 650.0).abs() < 1e-6)
        .unwrap();
    assert_relative_eq!(r650.delta_h, 133.839, epsilon = 0.05);
    assert_relative_eq!(r650.delta_s, 99.002, epsilon = 0.05);
    assert_relative_eq!(r650.delta_g, 42.445, epsilon = 0.05);

    let r1100 = resp
        .reaction_rows
        .iter()
        .find(|r| (r.temperature - 1100.0).abs() < 1e-6)
        .unwrap();
    assert!(r1100.k_eq > 1.0);
    assert_relative_eq!(r1100.log10_k, 0.031, epsilon = 0.01);
}

fn load_full_test_db() -> Option<ThermoDatabase> {
    let mut db = ThermoDatabase::with_open_defaults();
    let p = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
    if std::path::Path::new(p).exists() {
        db.load_external_hsc_readonly(p, false).ok()?;
        Some(db)
    } else {
        None
    }
}

#[test]
fn test_butane_adiabatic_flame() {
    let Some(db) = load_full_test_db() else {
        return;
    };
    let mut wb = soryntel_thermo::balance_engine::preset_butane();
    soryntel_thermo::balance_engine::evaluate_workbook(&mut wb, &db).unwrap();
    let s = &wb.summaries[0];
    // Element balance exact
    for el in &s.elements {
        assert_relative_eq!(el.balance_kmol, 0.0, epsilon = 1e-6);
    }
    // BUTANE.OHE values
    assert_relative_eq!(s.in_totals.total_h, -126.148, epsilon = 0.5);
    assert_relative_eq!(s.out_totals.heat_content, 2485.724, max_relative = 0.002);
    assert_relative_eq!(s.out_totals.total_h, -297.427, max_relative = 0.01);
    let t_adiab = soryntel_thermo::balance_engine::solve_adiabatic_temperature(&wb, 0, &db).unwrap();
    assert_relative_eq!(t_adiab, 2121.948, epsilon = 5.0);
}

#[test]
fn test_cusmelt_heat_balance() {
    let Some(db) = load_full_test_db() else {
        return;
    };
    let mut wb = soryntel_thermo::balance_engine::preset_cusmelt();
    soryntel_thermo::balance_engine::evaluate_workbook(&mut wb, &db).unwrap();
    let s = &wb.summaries[0];
    for el in &s.elements {
        assert_relative_eq!(el.balance_kmol, 0.0, epsilon = 1e-6);
    }
    assert_relative_eq!(s.in_totals.total_h, -1195.369, max_relative = 0.003);
    assert_relative_eq!(s.out_totals.total_h, -1265.612, max_relative = 0.003);
    assert_relative_eq!(s.balance_totals.total_h, -70.243, epsilon = 5.0);
    let t_adiab = soryntel_thermo::balance_engine::solve_adiabatic_temperature(&wb, 0, &db).unwrap();
    assert_relative_eq!(t_adiab, 1398.243, epsilon = 15.0);
}

#[test]
fn test_cuconv1_formula_and_element_balance() {
    let Some(db) = load_full_test_db() else {
        return;
    };
    let mut wb = soryntel_thermo::balance_engine::preset_cuconv1();
    soryntel_thermo::balance_engine::evaluate_workbook(&mut wb, &db).unwrap();
    let s = &wb.summaries[0];
    // Formula `=0.938*(InCu-D13)/2` -> 0.469 kmol Cu2S(l)
    assert_relative_eq!(
        wb.areas[0].out_streams[2].species[0].amount_kmol,
        0.469,
        epsilon = 1e-4
    );
    // Element balance: Cu = 1.000, Fe = 0.540, N = 6.930
    let cu = s.elements.iter().find(|e| e.element == "Cu").unwrap();
    assert_relative_eq!(cu.in_kmol, 1.000, epsilon = 1e-4);
    assert_relative_eq!(cu.balance_kmol, 0.0, epsilon = 1e-4);
    assert_relative_eq!(s.balance_totals.kg, 0.0, epsilon = 0.1);
}

#[test]
fn test_h2o_evaporator_pressure_corr() {
    let Some(db) = load_full_test_db() else {
        return;
    };
    let mut wb = soryntel_thermo::balance_engine::preset_h2o();
    soryntel_thermo::balance_engine::evaluate_workbook(&mut wb, &db).unwrap();
    let s = &wb.summaries[0];
    assert_relative_eq!(s.balance_totals.total_h, 0.0, epsilon = 0.05);
}

#[test]
fn test_s1_maindb6_and_ref_decode() {
    let hsc_path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
    if !std::path::Path::new(hsc_path).exists() {
        return;
    }
    let mut db = ThermoDatabase::with_open_defaults();
    let (unique_sp, total_recs) = db.load_main_hsc(hsc_path).unwrap();
    assert_eq!(total_recs, 36_537);
    assert_eq!(unique_sp, 21_089); // 21,089 chemical formulas + deleted/sentinel '0' tail slots in Maindb6.HSC

    let summary = db.status_summary();
    assert_eq!(summary.references_count, 948);

    use soryntel_thermo::{SpeciesCatalogPort, SpeciesRepository};
    let ac = db.get_species("Ac").unwrap();
    let ac_meta = db.get_species_metadata("Ac").unwrap();
    assert_eq!(ac_meta.cas_number, "7440-34-8");
    assert_eq!(ac_meta.color_code, 15);
    assert_eq!(ac_meta.reliability_class, 1);
    assert_eq!(ac.intervals.len(), 2);

    let cu2s = db.get_species("Cu2S").unwrap();
    let cu2s_meta = db.get_species_metadata("Cu2S").unwrap();
    assert_eq!(cu2s_meta.cas_number, "22205-45-4");
    assert_eq!(cu2s_meta.color_code, 31);
    assert_eq!(cu2s.intervals.len(), 4);

    let mg_sets = db.get_all_datasets_for(soryntel_thermo::species::DbSource::Main, "Mg(+2a)");
    assert!(mg_sets.len() >= 2, "Alternative Mg(+2a) datasets should be retained");

    let cites = db.lookup_citations(&cu2s.intervals[0].reference);
    assert!(!cites.is_empty());

    // S8: MgSO4(ia) vs Mg(+2a) + SO4(-2a)
    let mg = db.get_species("Mg(+2a)").unwrap();
    let so4 = db.get_species("SO4(-2a)").unwrap();
    let mgso4 = db.get_species("MgSO4(ia)").unwrap();
    let h_mg = mg.intervals[0].h_or_delta_h_tr / (1000.0 * soryntel_core::CAL_TO_JOULE);
    let h_so4 = so4.intervals[0].h_or_delta_h_tr / (1000.0 * soryntel_core::CAL_TO_JOULE);
    let h_ia = mgso4.intervals[0].h_or_delta_h_tr / (1000.0 * soryntel_core::CAL_TO_JOULE);
    assert!(((h_mg + h_so4) - h_ia).abs() < 0.5);
}

#[test]
fn test_s2_owndb_binary_roundtrip_and_precedence() {
    use soryntel_thermo::species::{DbSource, MergeConflictDecision, PhaseFilterFlags};
    use soryntel_thermo::{
        ChemicalSpecies, KelleyCoefficients, Phase, SpeciesMetadata, SpeciesRepository,
    };

    let tmp_dir = std::env::temp_dir().join("soryntel_m16_s2_test");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let own_path = tmp_dir.join("OWN_TEST.HSC");

    let mut db = ThermoDatabase::with_open_defaults();
    let main_hsc = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
    if std::path::Path::new(main_hsc).exists() {
        db.load_main_hsc(main_hsc).unwrap();
    }

    // Save modified FeS2 in OwnDB (-42.500 kcal/mol instead of -40.989 kcal/mol)
    let fes2 = ChemicalSpecies {
        formula: "FeS2".into(),
        name: "Iron(II) disulfide CUSTOM".into(),
        phase: Phase::Solid,
        molecular_weight: 119.967,
        density_g_cm3: 5.02,
        charge: 0,
        intervals: vec![soryntel_thermo::SpeciesThermoInterval {
            t_min: 298.15,
            t_max: 1500.0,
            phase: Phase::Solid,
            kelley: KelleyCoefficients {
                a: 17.301 * soryntel_core::CAL_TO_JOULE,
                b: 2.115 * soryntel_core::CAL_TO_JOULE,
                c: -2.731 * soryntel_core::CAL_TO_JOULE,
                d: 0.0,
            },
            h_or_delta_h_tr: -42.500 * 1000.0 * soryntel_core::CAL_TO_JOULE,
            s_or_delta_s_tr: 12.651 * soryntel_core::CAL_TO_JOULE,
            reference: "OwnLab 26".into(),
        }],
    };
    let meta = SpeciesMetadata {
        structural_formula: "FeS2".into(),
        common_name: "Pyrite Custom".into(),
        cas_number: "1309-36-0".into(),
        melting_point_k: 1440.0,
        boiling_point_k: 0.0,
        color_code: 51,
        solubility: "-".into(),
        reliability_class: 1,
    };
    db.upsert_own_dataset(fes2, meta.clone(), "s".into());
    assert_eq!(db.resolved_source_of("FeS2"), Some(DbSource::Own));
    let resolved = db.get_species("FeS2").unwrap();
    assert_relative_eq!(
        resolved.intervals[0].h_or_delta_h_tr / (1000.0 * soryntel_core::CAL_TO_JOULE),
        -42.500,
        epsilon = 1e-4
    );

    db.save_own_hsc(&own_path).unwrap();
    assert!(tmp_dir.join("OWN_TEST.CAT").exists());

    let mut db2 = ThermoDatabase::with_open_defaults();
    db2.load_own_hsc(&own_path).unwrap();
    let re = db2.get_species("FeS2").unwrap();
    assert_eq!(re.name, "Iron(II) disulfide CUSTOM");
    assert_relative_eq!(
        re.intervals[0].h_or_delta_h_tr / (1000.0 * soryntel_core::CAL_TO_JOULE),
        -42.500,
        epsilon = 1e-4
    );

    // CSV + Legacy + Merge round-trip
    let csv = db2.export_csv(Some(DbSource::Own));
    assert!(csv.contains("FeS2") && csv.contains("OwnLab 26"));
    let legacy_out = tmp_dir.join("OWN_LEGACY_UP.HSC");
    ThermoDatabase::convert_legacy_to_hsc6(&own_path, &legacy_out).unwrap();
    let merge_dst = tmp_dir.join("MERGE_DST.HSC");
    let empty_dec = std::collections::BTreeMap::new();
    let (merged_n, conflicts) =
        ThermoDatabase::merge_hsc_files(&own_path, &merge_dst, &empty_dec, false).unwrap();
    assert!(merged_n >= 1 && conflicts.is_empty());

    let mut keep_target = std::collections::BTreeMap::new();
    keep_target.insert("FeS2".to_string(), MergeConflictDecision::TakeTarget);
    let (_, conflicts2) =
        ThermoDatabase::merge_hsc_files(&own_path, &merge_dst, &keep_target, true).unwrap();
    assert_eq!(conflicts2.len(), 1);

    // Delete reverts OwnDB
    assert!(db2.delete_own_species("FeS2"));
    assert!(db2
        .search_advanced(
            "FeS2",
            soryntel_thermo::species::KeywordSearchField::Formula,
            Some(DbSource::Own),
            &PhaseFilterFlags::default(),
            10
        )
        .is_empty());

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_s3_s4_maindb6_stoichiometry_and_organic() {
    let hsc_path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
    if !std::path::Path::new(hsc_path).exists() {
        return;
    }
    let mut db = ThermoDatabase::with_open_defaults();
    db.load_main_hsc(hsc_path).unwrap();

    let filters = soryntel_thermo::species::PhaseFilterFlags::default();
    let fay = db.search_stoichiometry("SiO4FeFe", false, None, &filters, 50).unwrap();
    assert!(fay.iter().any(|e| e.formula == "*2FeO*SiO2"));
    assert!(fay.iter().any(|e| e.formula == "Fe2SiO4(B)"));

    let ass = db.search_stoichiometry("AsS", true, None, &filters, 50).unwrap();
    assert!(ass.iter().any(|e| e.formula == "AsS"));
    assert!(ass.iter().any(|e| e.formula == "As2S2" || e.formula == "As4S4"));

    // S4: Organic filter with C, H, O
    let elems = vec!["C".into(), "H".into(), "O".into()];
    let mut no_org = filters.clone();
    no_org.organic = false;
    let res_no_org = db.search_elements_filtered(
        &elems,
        soryntel_thermo::species::ElementSearchMode::Some,
        None,
        &no_org,
        5000,
    );
    assert!(!res_no_org.iter().any(|e| e.formula == "C3H8(g)"));
    assert!(res_no_org.iter().any(|e| e.formula == "CO2(g)"));
    assert!(res_no_org.iter().any(|e| e.formula == "C2H4(g)"));

    let mut only_c2 = filters.clone();
    only_c2.organic = true;
    only_c2.carbon_counts = vec![2];
    let res_c2 = db.search_elements_filtered(
        &elems,
        soryntel_thermo::species::ElementSearchMode::Some,
        None,
        &only_c2,
        200,
    );
    assert!(!res_c2.is_empty());
    assert!(res_c2.iter().any(|e| e.formula == "C2H4(g)"));
    assert!(!res_c2.iter().any(|e| e.formula == "C3H8(g)"));
}
