use approx::{assert_abs_diff_eq, assert_relative_eq};
use soryntel_core::BalanceEnergyUnit;
use soryntel_thermo::heat_loss::convection::evaluate_convection;
use soryntel_thermo::heat_loss::diagram::{
    generate_optimum_insulation_diagram, generate_temperature_profile, OptimumInsulationSpec,
};
use soryntel_thermo::heat_loss::gas_radiation::{
    calculate_gas_particle_radiation, calculate_gas_radiation, GasRadiationInput,
    OpticalDepthGeometry, ParticleCloudInput,
};
use soryntel_thermo::heat_loss::model::{
    CalculationMode, ConvectionGeometry, ConvectionRegime,
};
use soryntel_thermo::heat_loss::presets::{
    builtin_fluids, preset_hotwatertank, preset_oil_furnace_gas, preset_radiator,
    preset_simple_case, preset_smelting1,
};
use soryntel_thermo::heat_loss::radiation::validate_emissivity;
use soryntel_thermo::heat_loss::solver::{
    heat_flow_w_to_balance_energy_per_hour, solve_sheet, solve_workbook,
};
use soryntel_thermo::heat_loss::target::{solve_target, TargetGoal, TargetSpec, TargetVariable};

#[test]
fn test_simple_case_heat_flow_and_inverse_profile() {
    let wb = preset_simple_case();
    let res = solve_workbook(&wb).expect("SimpleCase should converge");
    assert_relative_eq!(res.total_heat_flow_w, 735.0, max_relative = 1e-4);

    let cols = &res.sheet_results[0].columns;
    assert_abs_diff_eq!(cols[0].t_right_k - 273.15, 418.0, epsilon = 0.05);
    assert_abs_diff_eq!(cols[1].t_right_k - 273.15, 50.63, epsilon = 0.05);

    // Steady-state invariance across all columns
    for c in cols {
        assert_relative_eq!(c.heat_flow_w, 735.0, max_relative = 1e-4);
    }

    // Temperature profile mode (1 fixed T + fixed Q = 735 W) must recover 20.0 °C
    let mut wb_prof = wb.clone();
    wb_prof.sheets[0].mode = CalculationMode::TemperatureProfile;
    wb_prof.sheets[0].fixed_heat_flow_w = 735.0;
    wb_prof.sheets[0].columns[2].t_right_fixed = false;
    let res_prof = solve_sheet(&wb_prof.sheets[0]).expect("TemperatureProfile should converge");
    assert_abs_diff_eq!(
        res_prof.columns.last().unwrap().t_right_k - 273.15,
        20.0,
        epsilon = 0.02
    );
}

#[test]
fn test_smelting1_fig01_exact_parity() {
    let wb = preset_smelting1();
    let res = solve_workbook(&wb).expect("Smelting1 should converge");
    assert_abs_diff_eq!(res.total_heat_flow_w / 1000.0, 4.251, epsilon = 0.005);

    let cols = &res.sheet_results[0].columns;
    assert_abs_diff_eq!(cols[0].t_right_k - 273.15, 1170.5, epsilon = 1.0);
    assert_abs_diff_eq!(cols[1].t_right_k - 273.15, 21.8, epsilon = 0.6);
    assert_abs_diff_eq!(cols[0].thermal_resistance_k_w * 1000.0, 6.944, epsilon = 0.01);
    assert_abs_diff_eq!(cols[2].thermal_resistance_k_w * 1000.0, 0.417, epsilon = 0.01);
}

#[test]
fn test_radiator_fig02_and_target_solver_fig11() {
    let wb = preset_radiator();
    let res = solve_workbook(&wb).expect("Radiator should converge");
    assert_abs_diff_eq!(res.total_heat_flow_w / 1000.0, 0.174, epsilon = 0.025);

    // Target: find Width to achieve 0.50 kW (500 W)
    let target = solve_target(
        &wb,
        &TargetSpec {
            sheet_index: 0,
            variable: TargetVariable::Width,
            goal: TargetGoal::TotalHeatFlowW(500.0),
            min_val: 0.5,
            max_val: 15.0,
            max_iterations: 60,
            tolerance: 1e-4,
        },
    )
    .expect("Target solver should find Radiator width");

    assert_abs_diff_eq!(target.achieved_goal_value, 500.0, epsilon = 0.1);
    assert!(target.solution_value > 4.5 && target.solution_value < 7.0);
}

#[test]
fn test_hotwatertank_fig04_cylindrical_areas_and_optimum_insulation() {
    let wb = preset_hotwatertank();
    let res = solve_workbook(&wb).expect("Hotwatertank should converge");

    // Check Wall cylindrical areas from Fig. 4: 37.699 -> 37.699 -> 37.919 -> 40.156 m^2
    let wall_cols = &res.sheet_results[0].columns;
    assert_abs_diff_eq!(wall_cols[0].surface_area_m2, 37.699, epsilon = 0.005);
    assert_abs_diff_eq!(wall_cols[1].surface_area_m2, 37.699, epsilon = 0.005);
    assert_abs_diff_eq!(wall_cols[2].surface_area_m2, 37.919, epsilon = 0.005);
    assert_abs_diff_eq!(wall_cols[3].surface_area_m2, 40.156, epsilon = 0.005);

    // Total heat flow across Wall + Roof + Bottom ~ 1.378 kW
    assert_abs_diff_eq!(res.total_heat_flow_w / 1000.0, 1.378, epsilon = 0.08);

    // Temperature profile diagram extraction
    let (_, profiles) = generate_temperature_profile(&wb).unwrap();
    assert_eq!(profiles.len(), 3);

    // Optimum insulation thickness curve (Fig. 14)
    let (pts, best) = generate_optimum_insulation_diagram(
        &wb,
        &OptimumInsulationSpec {
            sheet_index: 0,
            insulation_col_index: 2,
            min_thickness_m: 0.02,
            max_thickness_m: 0.60,
            steps: 29,
            operating_hours_per_year: 8760.0,
            energy_cost_per_kwh: 0.08,
            insulation_cost_per_m3: 250.0,
            amortization_years: 5.0,
        },
    )
    .expect("Optimum insulation sweep should succeed");
    assert_eq!(pts.len(), 30);
    assert!(best.thickness_m > 0.10 && best.thickness_m < 0.55);
}

#[test]
fn test_gas_and_particle_radiation_ewbm() {
    let comp = preset_oil_furnace_gas();
    let l = OpticalDepthGeometry::Cylinder {
        height_m: 3.0,
        diameter_m: 1.5,
    }
    .optical_depth_m()
    .unwrap();
    assert_abs_diff_eq!(l, 0.73 * 1.5, epsilon = 1e-6);

    // Cube 1x4x7 extrapolation example from Section 12.4.3 (exact = 1.35)
    let l_cube = OpticalDepthGeometry::Cube {
        a_m: 1.0,
        b_m: 4.0,
        c_m: 7.0,
    }
    .optical_depth_m()
    .unwrap();
    assert_abs_diff_eq!(l_cube, 1.35, epsilon = 1e-6);

    let gas_in = GasRadiationInput {
        t_gas_k: 1473.15,
        t_surface_k: 773.15,
        pressure_bar: 1.0,
        optical_depth_m: l,
        surface_emissivity: 0.85,
        composition: comp,
    };
    let gas_res = calculate_gas_radiation(&gas_in).unwrap();
    assert!(gas_res.gas_emissivity > 0.05 && gas_res.gas_emissivity < 0.85);
    assert!(gas_res.gas_absorptivity > 0.05 && gas_res.gas_absorptivity < 0.99);
    assert!(gas_res.heat_flux_w_m2 > 0.0);
    assert!(!gas_res.bands.is_empty());

    let particle = ParticleCloudInput {
        particle_emissivity: 0.8,
        number_density_m3: 5e8,
        cloud_thickness_m: l,
        size_distribution: vec![(50e-6, 1.0)],
    };
    let (combined, d_s, eps_c, hr) =
        calculate_gas_particle_radiation(&gas_in, &particle).unwrap();
    assert_abs_diff_eq!(d_s, 50e-6, epsilon = 1e-12);
    assert!(eps_c > 0.0 && eps_c < 1.0);
    assert!(combined.gas_emissivity >= gas_res.gas_emissivity);
    assert!(hr > 0.0);
}

#[test]
fn test_convection_limits_and_validations() {
    let air = &builtin_fluids()[0];
    // Sphere conductive limit: Ra -> 0 => Nu -> 2
    let diag = evaluate_convection(
        air,
        ConvectionRegime::Free,
        ConvectionGeometry::Sphere,
        300.0,
        300.0,
        0.1,
        None,
        0.0,
        None,
    )
    .unwrap();
    assert_abs_diff_eq!(diag.nusselt, 2.0, epsilon = 1e-6);

    // Horizontal enclosure conductive limit: Ra <= 1708 => Nu = 1
    let diag_enc = evaluate_convection(
        air,
        ConvectionRegime::Free,
        ConvectionGeometry::HorizontalEnclosure,
        300.1,
        300.0,
        0.005,
        Some(1.0),
        0.0,
        None,
    )
    .unwrap();
    assert!(diag_enc.rayleigh <= 1708.0);
    assert_abs_diff_eq!(diag_enc.nusselt, 1.0, epsilon = 1e-9);

    // Emissivity range validation
    assert!(validate_emissivity(1.2).is_err());
    assert!(validate_emissivity(-0.1).is_err());

    // Balance unit conversion check: 1000 W = 3.6 MJ/h = 1.0 kWh/h
    assert_abs_diff_eq!(
        heat_flow_w_to_balance_energy_per_hour(1000.0, BalanceEnergyUnit::MJ),
        3.6,
        epsilon = 1e-9
    );
    assert_abs_diff_eq!(
        heat_flow_w_to_balance_energy_per_hour(1000.0, BalanceEnergyUnit::Kwh),
        1.0,
        epsilon = 1e-9
    );
}

#[test]
fn test_material_database_fig1_to_fig3_and_particle_suspension_fig10() {
    use soryntel_thermo::heat_loss::gas_radiation::{
        evaluate_particle_suspension_detailed, generate_iterative_gas_radiation_table,
        ParticleSuspensionSpec,
    };
    use soryntel_thermo::heat_loss::material_db::MaterialDatabase;
    use soryntel_thermo::heat_loss::model::{deserialize_htr8_workbook, serialize_htr8_workbook};

    // 1. Material Database (Ch. 90 Figs. 1-3)
    let mut mdb = MaterialDatabase::with_hsc_defaults();
    let (comp_sum, species, elements) = mdb.resolve_composition(0).expect("Resolve Feed Material");
    assert_abs_diff_eq!(comp_sum, 100.0, epsilon = 1e-6);
    assert_eq!(species.len(), 14);
    let cufes2 = species.iter().find(|s| s.formula == "CuFeS2").unwrap();
    assert_abs_diff_eq!(cufes2.wt_pct, 16.838, epsilon = 0.01);
    let sio2 = species.iter().find(|s| s.formula == "SiO2").unwrap();
    assert_abs_diff_eq!(sio2.wt_pct, 23.805, epsilon = 0.01);
    let cu_el = elements.iter().find(|e| e.element == "Cu").unwrap();
    assert_abs_diff_eq!(cu_el.wt_pct, 13.335, epsilon = 0.05);

    mdb.recalculate_auto_properties(0).unwrap();
    let props = &mdb.materials[0].properties;
    let mw_str = &props.iter().find(|p| p.name == "Average molar weight").unwrap().value_text;
    assert_eq!(mw_str, "111.931");
    let rho_str = &props.iter().find(|p| p.name == "Density").unwrap().value_text;
    assert_eq!(rho_str, "4.062");
    let h_str = &props.iter().find(|p| p.name == "Enthalpy at 25 °C").unwrap().value_text;
    assert_eq!(h_str, "-5661.374");

    let cp_pts = mdb.sample_cp_curve(0, 10);
    assert!(!cp_pts.is_empty());

    // 2. Particle Suspension Detailed Calculator (Fig. 10)
    let p_spec = ParticleSuspensionSpec::default();
    let p_res = evaluate_particle_suspension_detailed(&p_spec).unwrap();
    assert!(p_res.gas_flow_m3_h > 24100.0);
    assert!(p_res.cloud_emissivity > 0.0 && p_res.cloud_emissivity <= 1.0);
    assert!(p_res.particles_per_m3 > 0.0);

    // 3. Iterative gas radiation table (0..3000 C step 50 C -> 61 points)
    let iter_tbl =
        generate_iterative_gas_radiation_table(1.0, 1.095, &preset_oil_furnace_gas());
    assert_eq!(iter_tbl.len(), 61);

    // 4. .htr8 roundtrip serialization
    let wb = preset_hotwatertank();
    let raw = serialize_htr8_workbook(&wb).unwrap();
    let wb2 = deserialize_htr8_workbook(&raw).unwrap();
    assert_eq!(wb2.sheets.len(), 3);
}
