pub mod lca;
pub mod mass_balance;
pub mod model;
pub mod presets;
pub mod reports;
pub mod solver;

#[cfg(test)]
mod tests {
    use super::model::{LogSeverity, MonteCarloDist};
    use super::presets::{
        preset_concentrator_flotation, preset_hydro_example, preset_recycle_loop,
    };
    use super::reports::{generate_sim_reports, report_sheet_to_csv};
    use super::solver::{
        compute_calculation_order, empty_all_tanks, evaluate_distribution_curves,
        execute_scenario, export_flowsheet_svg, export_lca_json, filter_log_message,
        generate_control_sweep, rewrite_sim7_formula, run_control_loops, run_static_calculation,
        safediv, step_dynamic_simulation, LogFilterLevel,
    };

    #[test]
    fn test_recycle_loop_tarjan_and_convergence() {
        let mut proj = preset_recycle_loop();
        let order = compute_calculation_order(&proj);
        assert!(
            !order.tear_stream_ids.is_empty(),
            "Recycle loop preset must identify at least one tear stream"
        );
        assert_eq!(order.unit_order_ids.len(), proj.units.len());

        run_static_calculation(&mut proj, None);
        let last = proj
            .convergence_history
            .last()
            .expect("convergence history populated");
        assert!(
            last.converged,
            "Recycle loop must converge (MB={}%, rounds={})",
            last.mb_pct,
            proj.convergence_history.len()
        );
        assert!(last.mb_pct < proj.calc_settings.tolerance_pct);
    }

    #[test]
    fn test_hydro_example_static_and_controls() {
        let mut proj = preset_hydro_example();
        run_control_loops(&mut proj, None);
        run_static_calculation(&mut proj, None);

        let last = proj.convergence_history.last().expect("history exists");
        assert!(last.converged);
        assert!(last.mb_pct < 0.1);

        // Both Cooler (Heat bal) and Leaching (Acid conc.) controls should stay close to setpoints
        assert_eq!(proj.controls.len(), 2);
        for ctrl in &proj.controls {
            let err = (ctrl.measured - ctrl.set_point).abs();
            assert!(
                err <= 0.25,
                "Control '{}' err={} exceeded threshold",
                ctrl.control_name,
                err
            );
        }

        // Control sweep should produce 50 points spanning [x_min, x_max]
        let sweep = generate_control_sweep(&proj, 1, 50);
        assert_eq!(sweep.len(), 50);
        assert!(sweep.first().unwrap().0 < sweep.last().unwrap().0);
    }

    #[test]
    fn test_scenario_concentrator_deterministic_and_monte_carlo() {
        let mut proj = preset_concentrator_flotation();
        // Deterministic mode: Monte Carlo OFF
        proj.scenarios[0].columns[0].monte_carlo_on = false;
        execute_scenario(&mut proj, 0, 42);

        let rows = &proj.scenarios[0].values;
        assert_eq!(rows.len(), 10);
        // Run 1: d50=50 -> Cu Rec ~= 53.10
        assert!((rows[0][0] - 50.0).abs() < 1e-6);
        assert!((rows[0][1] - 53.10).abs() < 0.05, "got {}", rows[0][1]);
        // Run 6: d50=175 -> peak Cu Rec ~= 66.04
        assert!((rows[5][0] - 175.0).abs() < 1e-6);
        assert!((rows[5][1] - 66.04).abs() < 0.05, "got {}", rows[5][1]);
        // Run 10: d50=350 -> Cu Rec ~= 64.80
        assert!((rows[9][0] - 350.0).abs() < 1e-6);
        assert!((rows[9][1] - 64.80).abs() < 0.05, "got {}", rows[9][1]);

        // Monte Carlo Normal mode [150, 250] + Round to Nearest = 5.0
        proj.scenarios[0].columns[0].monte_carlo_on = true;
        proj.scenarios[0].columns[0].dist_type = MonteCarloDist::Normal;
        proj.scenarios[0].columns[0].mc_min = 150.0;
        proj.scenarios[0].columns[0].mc_max = 250.0;
        proj.scenarios[0].columns[0].mc_mean = 185.0;
        proj.scenarios[0].columns[0].mc_sd = 20.21;
        proj.scenarios[0].columns[0].round_to_nearest = Some(5.0);
        execute_scenario(&mut proj, 0, 12345);

        for r in &proj.scenarios[0].values {
            let x = r[0];
            assert!((150.0..=250.0).contains(&x), "MC sample {x} out of [150,250]");
            let rem = (x / 5.0).fract().abs();
            assert!(rem < 1e-6 || (1.0 - rem) < 1e-6, "Not rounded to nearest 5: {x}");
        }

        // Distribution PDF/CDF curve sanity
        let curves = evaluate_distribution_curves(&proj.scenarios[0].columns[0], 41);
        assert_eq!(curves.0.len(), 41);
        assert_eq!(curves.1.len(), 41);
        assert!((curves.1.last().unwrap()[1] - 1.0).abs() < 0.02);
    }

    #[test]
    fn test_dynamic_tank_empty_and_step() {
        let mut proj = preset_hydro_example();
        empty_all_tanks(&mut proj);
        assert_eq!(proj.dynamic_time_s, 0.0);
        step_dynamic_simulation(&mut proj, 5);
        assert!(proj.dynamic_time_s > 0.0);
        assert!(!proj.logs.is_empty());
    }

    #[test]
    fn test_boolean_log_filter_sim7_rewrite_and_exports() {
        assert_eq!(safediv(10.0, 0.0), 0.0);
        assert_eq!(safediv(10.0, 2.0), 5.0);

        assert!(filter_log_message(
            "Leaching balance warning on AcidFeed",
            LogSeverity::Warning,
            LogFilterLevel::WarningsAndErrors,
            "leaching AND acidfeed NOT cooler"
        ));
        assert!(!filter_log_message(
            "Leaching info ok",
            LogSeverity::Info,
            LogFilterLevel::Errors,
            "leaching"
        ));

        let (clean_sheet, rewritten) = rewrite_sim7_formula("=[Book1.xls]Sheet!C5 / B1", "Feed/A");
        assert_eq!(clean_sheet, "Feed_A");
        assert!(rewritten.contains("safediv"));
        assert!(rewritten.contains("REF["));

        let proj = preset_hydro_example();
        let wb = generate_sim_reports(&proj);
        assert_eq!(wb.sheets.len(), 4);
        assert_eq!(wb.sheets[0].name, "Unit Balance - General");
        assert_eq!(wb.sheets[1].name, "Stream Balance - Hydro");
        assert_eq!(wb.sheets[2].name, "Streams - Properties");
        assert_eq!(wb.sheets[3].name, "Flowsheet Tables");

        let csv = report_sheet_to_csv(&wb.sheets[2]);
        assert!(csv.contains("FeS"));
        assert!(csv.contains("Solution"));

        let lca = export_lca_json(&proj).expect("valid LCA json");
        assert!(lca.contains("Hydro_example"));
        assert!(lca.contains("elementary_inputs"));
        assert!(lca.contains("elementary_outputs"));

        let svg = export_flowsheet_svg(&proj);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Leaching"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_lca_classic_zn_tsl_fuming_normalization_and_exports() {
        use crate::sim::lca::*;
        use crate::sim::presets::preset_zn_tsl_fuming;
        let proj = preset_zn_tsl_fuming();
        let mut lca = preset_classic_zn_tsl_fuming_lca(&proj);

        // Black-box boundary exclusion: 6 inputs + 8 outputs (no internal streams)
        assert_eq!(lca.inputs.len(), 6);
        assert_eq!(lca.outputs.len(), 8);

        // Hourly conversion check
        let residues = lca.inputs.iter().find(|r| r.stream_name == "Residues").unwrap();
        assert!((residues.converted_amount() - 100_000.0).abs() < 1e-6);
        assert_eq!(residues.converted_unit(), "kg");
        let power = lca.inputs.iter().find(|r| r.stream_name == "Power").unwrap();
        assert!((power.converted_amount() - 63_020.50).abs() < 1e-6);
        assert_eq!(power.converted_unit(), "kWh");

        // Indicator CO2(g) wildcard match = 15951.35 kg
        let co2_ind = lca.indicators.iter().find(|i| i.pattern == "CO2(g)").unwrap();
        assert!((co2_ind.total_counted_amount() - 15_951.35).abs() < 1e-4);

        // Normalized per 1 kg Zinc oxide (8012.99 kg):
        // Merged Air (34590.74 + 1.29) = 34592.03 -> 4.32 kg
        let air_norm = lca.normalized.iter().find(|n| n.flow_name == "Air").unwrap();
        assert!((air_norm.amount - 34_592.03).abs() < 1e-2);
        assert!((air_norm.normalized_amount - 4.3170).abs() < 1e-3);

        // Merged Waste heat (6302.05 + 3327.22 + 441.09) = 10070.36 kWh -> 1.26 kWh
        let heat_norm = lca.normalized.iter().find(|n| n.flow_name == "Waste heat").unwrap();
        assert!((heat_norm.amount - 10_070.36).abs() < 1e-2);
        assert!((heat_norm.normalized_amount - 1.2568).abs() < 1e-3);

        // Zinc oxide = 1.00 kg
        let zno_norm = lca.normalized.iter().find(|n| n.flow_name == "Zinc oxide").unwrap();
        assert!((zno_norm.normalized_amount - 1.0).abs() < 1e-9);

        // Use Exergy switch on Residues -> switches amount to 30074.13 kWh
        lca.inputs[0].use_exergy = true;
        assert!((lca.inputs[0].converted_amount() - 30_074.13).abs() < 1e-6);
        assert_eq!(lca.inputs[0].converted_unit(), "kWh");
        lca.inputs[0].use_exergy = false;

        // EcoSpold v1.0 XML & OpenLCA JSON-LD zip + XLSX export
        let xml = lca.export_ecospold_v1_xml().expect("ecospold export");
        assert!(xml.contains("FI: TSL Furnace [HSC] [Pyro]"));
        assert!(xml.contains("Zinc oxide"));

        let zip_bytes = lca.export_openlca_jsonld_zip().expect("json-ld zip");
        assert!(zip_bytes.starts_with(b"PK\x03\x04"));
        let mut lca_import = ClassicLcaState::default();
        let loaded = lca_import
            .load_openlca_jsonld_zip_bytes("test.zip", &zip_bytes)
            .expect("round-trip stored zip");
        assert!(loaded >= 8);

        let xlsx = lca.export_xlsx().expect("6-sheet xlsx");
        assert!(xlsx.starts_with(b"PK\x03\x04"));
    }

    #[test]
    fn test_lca_beta_iron_process_lcia_and_rules() {
        use crate::sim::lca::*;
        use crate::sim::presets::preset_iron_process;
        let mut proj = preset_iron_process();
        let mut beta = preset_beta_iron_process_lca(&proj);

        assert_eq!(beta.results.len(), 2);
        let gwp = &beta.results[0];
        let mdp = &beta.results[1];

        // Verify Ch. 49.4 Figs. 34-35 exact totals
        assert!((gwp.grand_total - 16_883.27).abs() < 0.05, "got {}", gwp.grand_total);
        assert!((mdp.grand_total - 1_851.14).abs() < 0.05, "got {}", mdp.grand_total);
        assert_eq!(gwp.units.len(), 2); // Blast Furnace + Manual Streams
        let bf = &gwp.units[0];
        assert!((bf.inputs_total - 2_685.82).abs() < 0.05);
        assert!((bf.outputs_total - 733.22).abs() < 0.05);
        assert!((bf.unit_total - 3_419.04).abs() < 0.05);

        // Verify 1.00000 kg synthetic processes & 3.6 MJ conversion
        let man_proc = beta
            .synthetic_processes
            .iter()
            .find(|p| p.process_name == "Manual Streams")
            .unwrap();
        assert!((man_proc.synthetic_ref_amount_kg - 1.0).abs() < 1e-9);
        assert!((man_proc.exchanges[0].2 - 36_000.0).abs() < 1e-6);

        // > 15 providers rule on electricity_hv (Fig. 27)
        let q_no_loc = beta.query_providers(&deterministic_uuid("olca::electricity_hv"), None);
        assert!(q_no_loc.total_providers > 15);
        assert!(q_no_loc.require_location_first);
        assert!(q_no_loc.available_providers.is_empty());
        let q_glo = beta.query_providers(&deterministic_uuid("olca::electricity_hv"), Some("GLO"));
        assert!(!q_glo.available_providers.is_empty());

        // Manual stream deletion error when non-manual selected (Fig. 12)
        assert_eq!(
            beta.delete_manual_stream(false, 0).unwrap_err(),
            "Manual stream is not selected"
        );

        // Single impact method restriction
        assert!(beta.add_impact_category("cml_2001", "cml_gwp").is_err());

        // Custom sheet naming prohibitions & Saved snapshot
        assert!(beta.add_custom_sheet("SavedTest").is_err());
        assert!(beta.add_custom_sheet("NormSheet").is_err());
        assert!(beta.add_custom_sheet("ExtData").is_err());
        assert!(beta.add_custom_sheet("Custom Mass Sheet").is_ok());
        let saved_name = beta.save_results_to_custom_sheet().expect("save snapshot");
        assert!(saved_name.starts_with("Saved"));

        // Live flowsheet update scales LCIA results proportionally without recalculating setup
        proj.streams.iter_mut().find(|s| s.name_id == "Coke").unwrap().mass_flow_t_h = 100.0;
        beta.sync_from_project(&proj);
        assert!(beta.results[0].grand_total > 17_000.0);

        let xlsx = beta.export_to_xlsx();
        assert!(xlsx.starts_with(b"PK\x03\x04"));
    }

    #[test]
    fn test_lca_wildcard_patterns() {
        use crate::sim::lca::wildcard_matches;
        assert!(wildcard_matches("CO2(g)", "CO2(g)"));
        assert!(wildcard_matches("SO*", "SO2(g)"));
        assert!(wildcard_matches("SO#", "SO2"));
        assert!(!wildcard_matches("SO#", "SOx"));
        assert!(wildcard_matches("NO?", "NO2"));
        assert!(!wildcard_matches("NO?", "NO2(g)"));
    }
}
