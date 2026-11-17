//! Hexagonal Application Layer (Inbound Use-Case Ports).
//!
//! Depends strictly on domain crates (`soryntel-core`, `soryntel-solver`, `soryntel-thermo`)
//! and outbound port traits (`SpeciesRepository`, `SpeciesCatalogPort`) — never on concrete
//! database or UI adapters.

use soryntel_core::SoryntelError;
use soryntel_thermo::{
    aqua::{
        analyze_parameter_sets as domain_aqua_analyze_parameter_sets,
        calculate_aqua as domain_aqua_calculate,
        calculate_solution_density as domain_aqua_density,
        resolve_pitzer_catalog as domain_aqua_resolve_catalog,
        ActiveParameterSetsReport, AquaCalcResult, AquaCaseFile, DensityCalcResult, DensityInputRow,
        MergedPitzerCatalog, OwnPitzerDb,
    },
    aqua_pitzer_data::{builtin_main_pitzer_db as domain_aqua_main_pitzer_db, PitzerDatabase},
    balance::estimate_air_cooled_reactor_heat_loss as domain_estimate_air_cooled_heat_loss,
    balance_engine::{
        build_balance_export_sheets as domain_build_balance_export_sheets,
        build_linked_paste_stream as domain_build_linked_paste_stream,
        compute_stream_compositions as domain_compute_stream_compositions,
        convert_workbook_energy_unit as domain_convert_wb_energy_unit,
        convert_workbook_temperature_unit as domain_convert_wb_temp_unit,
        deserialize_bal8_workbook as domain_deserialize_bal8_workbook,
        evaluate_workbook as domain_evaluate_balance_workbook,
        iterate_target_rows as domain_iterate_balance_targets,
        preset_butane as domain_balance_preset_butane,
        preset_cuconv1 as domain_balance_preset_cuconv1,
        preset_cusmelt as domain_balance_preset_cusmelt,
        preset_fsf_process as domain_balance_preset_fsf_process,
        preset_h2o as domain_balance_preset_h2o,
        run_diagram_sweep as domain_run_balance_diagram_sweep,
        scale_stream_by_total_kg as domain_scale_stream_kg,
        scale_stream_by_total_kmol as domain_scale_stream_kmol,
        scale_stream_by_total_nm3 as domain_scale_stream_nm3,
        serialize_bal8_workbook as domain_serialize_bal8_workbook,
        set_species_amount_from_kg as domain_set_species_kg,
        set_species_amount_from_nm3 as domain_set_species_nm3,
        solve_adiabatic_temperature as domain_solve_adiabatic_temp, BalanceArea, BalanceWorkbook,
        DiagramResult as BalanceDiagramResult, SpeciesRow, StreamComposition, StreamGroup,
    },
    benson::{
        build_benson_chemical_species, estimate_benson, BensonEstimationResult, BensonParamsInput,
        MoleculeGraph,
    },
    diagrams::{calculate_diagram, DiagramRequest, DiagramResult},
    ehph::{calculate_ehph, EhPhRequest, EhPhResult},
    elements::{
        lookup_element_species as domain_lookup_element_species, ElementWorkbook,
        PeriodicDiagramData,
    },
    equilibrium::{
        solve_equilibrium_sweep, EquilibriumParams, EquilibriumSweepResult, EquilibriumSystem,
    },
    estimate::{evaluate_estimate_row, EstimateRow},
    excel_addins::{
        build_addinsample_export_sheet, build_minerals_export_sheet, build_streameq_export_sheet,
        build_webservice_formula, eval_addin_function, eval_formula_string, eval_streameq,
        lookup_mineral, migrate_legacy_formula, streameq_element_balance, AddInFunctionMeta,
        AddInUnits, ArgValue, CellEvalValue, MineralRecord, ADDIN_FUNCTION_CATALOG,
    },
    heat_loss::{solve_sheet, solve_workbook, HeatLossSheet, HeatLossWorkbook, SheetResult, WorkbookResult},
    phase_stability::{
        calculate_phase_stability, PhaseStabilityDiagram, PhaseStabilityError, PhaseStabilityInput,
    },
    sim::{
        lca::{
            builtin_gabi_flows as domain_sim_lca_gabi_flows,
            builtin_lcia_methods as domain_sim_lca_methods,
            builtin_openlca_flows as domain_sim_lca_openlca_flows,
            preset_beta_iron_process_lca as domain_sim_lca_preset_beta_iron,
            preset_classic_zn_tsl_fuming_lca as domain_sim_lca_preset_classic_zn,
            BetaLcaState, ClassicLcaState, LcaCatalogFlow, LciaMethodSpec,
        },
        mass_balance::{
            preset_grinding_circuit_mb as domain_mb_preset_grinding,
            preset_rougher_flotation_mb as domain_mb_preset_rougher,
            MassBalanceWorkbook, MbSolution, ValidationIssue,
        },
        model::{LogSeverity, ScenarioColumn, SimProject, UnitModelCatalogEntry},
        presets::{
            builtin_unit_model_catalog as domain_sim_model_catalog,
            empty_project as domain_sim_empty_project,
            metric_value_and_unit as domain_sim_metric_value,
            preset_40_1_model as domain_sim_preset_40_1,
            preset_concentrator_flotation as domain_sim_preset_concentrator,
            preset_hydro_example as domain_sim_preset_hydro,
            preset_iron_process as domain_sim_preset_iron_process,
            preset_recycle_loop as domain_sim_preset_recycle,
            preset_zn_tsl_fuming as domain_sim_preset_zn_tsl,
        },
        reports::{
            generate_sim_reports as domain_sim_generate_reports,
            report_sheet_to_csv as domain_sim_report_csv, SimReportSheet, SimReportWorkbook,
        },
        solver::{
            clear_internal_streams as domain_sim_clear_internal,
            compute_calculation_order as domain_sim_calc_order,
            empty_all_tanks as domain_sim_empty_tanks,
            evaluate_distribution_curves as domain_sim_eval_mc_curves,
            evaluate_global_balances as domain_sim_eval_balances,
            execute_scenario as domain_sim_execute_scenario,
            export_flowsheet_svg as domain_sim_export_svg,
            export_lca_json as domain_sim_export_lca_json,
            filter_log_message as domain_sim_filter_log,
            generate_control_sweep as domain_sim_control_sweep,
            rewrite_sim7_formula as domain_sim_rewrite_sim7,
            run_control_loops as domain_sim_run_controls,
            run_static_calculation as domain_sim_run_static,
            step_dynamic_simulation as domain_sim_step_dynamic, CalcOrderInfo, LogFilterLevel,
        },
    },
    species::{
        analyze_aqueous_scales, awdb_atomic_weight, color_spec, compute_molecular_weight,
        convert_cp_function, convert_gibbs_function, extrapolate_liquid_298, fit_cp_kelley,
        gibbs_delta_to_normal, parse_hsc_formula_elements, stoichiometry_matches,
        AqueousScaleSummary, CpDataPoint, CpFitMask, CpFitResult, GibbsCoeffsInput,
        GibbsConvertOutput, HscColorSpec,
    },
    species_converter::{
        fix_element_balance, initialize_input, solve_species_conversion, CalculatedElementRow,
        ConverterInputEntry, ConverterOutputSpec, SpeciesConverterError, SpeciesConverterRequest,
        SpeciesConverterResult,
    },
    spreadsheet_export::{
        build_libreoffice_oxt, build_ods, build_onlyoffice_plugin_zip, build_xlsx,
        generate_excel_wps_vba, ExportSheet,
    },
    units::{
        build_air_humidity_3d_surface as domain_air_3d_surface,
        build_air_humidity_grid_descending as domain_air_grid_descending,
        compute_air_humidity_point as domain_air_humidity_point, AirHumidityGrid,
        AirHumidityPoint, SiPrefix, UnitsWorkbook,
    },
    water::{
        calculate_water_points as domain_calculate_water_points,
        default_water_request as domain_default_water_request,
        water_diagram_curves as domain_water_diagram_curves, WaterCalcRequest, WaterCalcResult,
        WaterDiagramCurves,
    },
    are_responses_plot_compatible as domain_reaction_plot_compatible,
    build_reaction_export_sheets as domain_build_reaction_export_sheets, calculate,
    deserialize_rea8_workbook as domain_deserialize_rea8_workbook,
    extract_reaction_plot_points as domain_extract_reaction_plot_points,
    format_reaction_sheet_tsv as domain_format_reaction_sheet_tsv,
    render_reaction_chart_svg as domain_render_reaction_chart_svg,
    serialize_rea8_workbook as domain_serialize_rea8_workbook, CalculationRequest,
    CalculationResponse, ChemicalSpecies, ReactionCollectedSheet, ReactionWorkbookFile,
    ReactionYAxis, SpeciesCatalogPort,
};

/// Inbound Application Service orchestrating all 20 thermodynamic modules over an outbound
/// `SpeciesCatalogPort` adapter.
pub struct SoryntelApplication;

impl SoryntelApplication {
    pub fn balance_reaction_equation(
        raw: &str,
        smallest_coeff: f64,
    ) -> Result<(String, soryntel_solver::ParsedReaction), SoryntelError> {
        soryntel_solver::balance_reaction(raw, smallest_coeff)
    }

    pub fn calculate_reaction(
        catalog: &dyn SpeciesCatalogPort,
        req: &CalculationRequest,
    ) -> Result<CalculationResponse, SoryntelError> {
        calculate(req, catalog)
    }

    pub fn are_reaction_sheets_plot_compatible(
        primary: &CalculationResponse,
        candidate: &CalculationResponse,
    ) -> bool {
        domain_reaction_plot_compatible(primary, candidate)
    }

    pub fn extract_reaction_plot_points(
        res: &CalculationResponse,
        y_axis: ReactionYAxis,
        log_y_scale: bool,
    ) -> Vec<[f64; 2]> {
        domain_extract_reaction_plot_points(res, y_axis, log_y_scale)
    }

    pub fn format_reaction_sheet_tsv(sheet: &ReactionCollectedSheet, decimals: usize) -> String {
        domain_format_reaction_sheet_tsv(sheet, decimals)
    }

    pub fn export_reaction_workbook_xlsx(sheets: &[ReactionCollectedSheet]) -> Vec<u8> {
        let export_sheets = domain_build_reaction_export_sheets(sheets);
        build_xlsx(&export_sheets)
    }

    pub fn render_reaction_chart_svg(
        title: &str,
        x_label: &str,
        y_label: &str,
        series: &[(String, Vec<[f64; 2]>, &str)],
        show_grid: bool,
        show_curve_labels: bool,
    ) -> String {
        domain_render_reaction_chart_svg(title, x_label, y_label, series, show_grid, show_curve_labels)
    }

    pub fn serialize_rea8_workbook(wb: &ReactionWorkbookFile) -> Result<String, SoryntelError> {
        domain_serialize_rea8_workbook(wb)
    }

    pub fn deserialize_rea8_workbook(raw: &str) -> Result<ReactionWorkbookFile, SoryntelError> {
        domain_deserialize_rea8_workbook(raw)
    }

    // -----------------------------------------------------------------------
    // Module 2: Heat and Material Balance (Ch. 11)
    // -----------------------------------------------------------------------

    pub fn balance_preset_cuconv1() -> BalanceWorkbook {
        domain_balance_preset_cuconv1()
    }

    pub fn balance_preset_butane() -> BalanceWorkbook {
        domain_balance_preset_butane()
    }

    pub fn balance_preset_cusmelt() -> BalanceWorkbook {
        domain_balance_preset_cusmelt()
    }

    pub fn balance_preset_h2o() -> BalanceWorkbook {
        domain_balance_preset_h2o()
    }

    pub fn balance_preset_fsf_process() -> BalanceWorkbook {
        domain_balance_preset_fsf_process()
    }

    pub fn balance_preset_ch4_cooling_water() -> BalanceWorkbook {
        soryntel_thermo::balance_engine::preset_ch4_cooling_water()
    }

    pub fn evaluate_balance_workbook(
        catalog: &dyn SpeciesCatalogPort,
        wb: &mut BalanceWorkbook,
    ) -> Result<(), SoryntelError> {
        domain_evaluate_balance_workbook(wb, catalog)
    }

    pub fn solve_balance_adiabatic_temperature(
        catalog: &dyn SpeciesCatalogPort,
        wb: &BalanceWorkbook,
        area_idx: usize,
    ) -> Result<f64, SoryntelError> {
        domain_solve_adiabatic_temp(wb, area_idx, catalog)
    }

    pub fn iterate_balance_target_rows(
        catalog: &dyn SpeciesCatalogPort,
        wb: &mut BalanceWorkbook,
        row_indices: &[usize],
    ) -> Result<(), SoryntelError> {
        domain_iterate_balance_targets(wb, row_indices, catalog)
    }

    pub fn run_balance_diagram_sweep(
        catalog: &dyn SpeciesCatalogPort,
        wb: &BalanceWorkbook,
    ) -> Result<BalanceDiagramResult, SoryntelError> {
        domain_run_balance_diagram_sweep(wb, catalog)
    }

    pub fn compute_balance_stream_compositions(
        area: &BalanceArea,
        is_out: bool,
    ) -> Vec<StreamComposition> {
        domain_compute_stream_compositions(area, is_out)
    }

    pub fn set_balance_species_kg(sp: &mut SpeciesRow, new_kg: f64) {
        domain_set_species_kg(sp, new_kg);
    }

    pub fn set_balance_species_nm3(
        catalog: &dyn SpeciesCatalogPort,
        sp: &mut SpeciesRow,
        new_nm3: f64,
    ) {
        domain_set_species_nm3(sp, new_nm3, catalog);
    }

    pub fn scale_balance_stream_kmol(stream: &mut StreamGroup, new_total_kmol: f64) {
        domain_scale_stream_kmol(stream, new_total_kmol);
    }

    pub fn scale_balance_stream_kg(stream: &mut StreamGroup, new_total_kg: f64) {
        domain_scale_stream_kg(stream, new_total_kg);
    }

    pub fn scale_balance_stream_nm3(stream: &mut StreamGroup, new_total_nm3: f64) {
        domain_scale_stream_nm3(stream, new_total_nm3);
    }

    pub fn convert_balance_temperature_unit(
        wb: &mut BalanceWorkbook,
        new_unit: soryntel_core::TemperatureUnit,
    ) {
        domain_convert_wb_temp_unit(wb, new_unit);
    }

    pub fn convert_balance_energy_unit(
        wb: &mut BalanceWorkbook,
        new_unit: soryntel_core::BalanceEnergyUnit,
    ) {
        domain_convert_wb_energy_unit(wb, new_unit);
    }

    pub fn build_balance_linked_paste_stream(
        wb: &BalanceWorkbook,
        src_area_idx: usize,
        src_is_out: bool,
        src_stream_idx: usize,
    ) -> Option<StreamGroup> {
        domain_build_linked_paste_stream(wb, src_area_idx, src_is_out, src_stream_idx)
    }

    pub fn estimate_reactor_natural_convection_loss(
        area_m2: f64,
        t_surface_c: f64,
        t_room_c: f64,
        unit: soryntel_core::BalanceEnergyUnit,
    ) -> f64 {
        domain_estimate_air_cooled_heat_loss(area_m2, t_surface_c, t_room_c, unit)
    }

    pub fn export_balance_workbook_xlsx(wb: &BalanceWorkbook) -> Vec<u8> {
        let sheets = domain_build_balance_export_sheets(wb);
        build_xlsx(&sheets)
    }

    pub fn serialize_bal8_workbook(wb: &BalanceWorkbook) -> Result<String, SoryntelError> {
        domain_serialize_bal8_workbook(wb)
    }

    pub fn deserialize_bal8_workbook(raw: &str) -> Result<BalanceWorkbook, SoryntelError> {
        domain_deserialize_bal8_workbook(raw)
    }

    pub fn heat_flow_w_to_balance_energy_per_hour(
        q_watts: f64,
        unit: soryntel_core::BalanceEnergyUnit,
    ) -> f64 {
        soryntel_thermo::heat_loss::heat_flow_w_to_balance_energy_per_hour(q_watts, unit)
    }

    pub fn solve_heat_loss_sheet(sheet: &HeatLossSheet) -> Result<SheetResult, SoryntelError> {
        solve_sheet(sheet)
    }

    pub fn solve_heat_loss_workbook(wb: &HeatLossWorkbook) -> Result<WorkbookResult, SoryntelError> {
        solve_workbook(wb)
    }

    pub fn solve_heat_loss_target(
        wb: &HeatLossWorkbook,
        spec: &soryntel_thermo::heat_loss::TargetSpec,
    ) -> Result<soryntel_thermo::heat_loss::TargetResult, SoryntelError> {
        soryntel_thermo::heat_loss::solve_target(wb, spec)
    }

    pub fn generate_heat_loss_temperature_profile(
        wb: &HeatLossWorkbook,
    ) -> Result<
        (
            f64,
            Vec<soryntel_thermo::heat_loss::TemperatureProfileSeries>,
        ),
        SoryntelError,
    > {
        soryntel_thermo::heat_loss::generate_temperature_profile(wb)
    }

    pub fn generate_heat_loss_parameter_sweep(
        wb: &HeatLossWorkbook,
        sheet_index: usize,
        param: soryntel_thermo::heat_loss::SweepParameter,
        min_x: f64,
        max_x: f64,
        steps: usize,
    ) -> Result<Vec<soryntel_thermo::heat_loss::SweepPoint>, SoryntelError> {
        soryntel_thermo::heat_loss::generate_parameter_sweep(wb, sheet_index, param, min_x, max_x, steps)
    }

    pub fn generate_heat_loss_optimum_insulation(
        wb: &HeatLossWorkbook,
        spec: &soryntel_thermo::heat_loss::OptimumInsulationSpec,
    ) -> Result<
        (
            Vec<soryntel_thermo::heat_loss::OptimumInsulationPoint>,
            soryntel_thermo::heat_loss::OptimumInsulationPoint,
        ),
        SoryntelError,
    > {
        soryntel_thermo::heat_loss::generate_optimum_insulation_diagram(wb, spec)
    }

    pub fn evaluate_heat_loss_convection(
        fluid: &soryntel_thermo::heat_loss::FluidPropertyTable,
        regime: soryntel_thermo::heat_loss::ConvectionRegime,
        geom: soryntel_thermo::heat_loss::ConvectionGeometry,
        t_surface_k: f64,
        t_ambient_k: f64,
        char_length_m: f64,
        enclosure_height_m: Option<f64>,
        speed_m_s: f64,
        outer_diameter_m: Option<f64>,
    ) -> Result<soryntel_thermo::heat_loss::convection::ConvectionDiagnostics, SoryntelError> {
        soryntel_thermo::heat_loss::convection::evaluate_convection(
            fluid,
            regime,
            geom,
            t_surface_k,
            t_ambient_k,
            char_length_m,
            enclosure_height_m,
            speed_m_s,
            outer_diameter_m,
        )
    }

    pub fn evaluate_heat_loss_hc_function(
        terms: &[soryntel_thermo::heat_loss::ConvectionFunctionTerm],
        speed_m_s: f64,
    ) -> f64 {
        soryntel_thermo::heat_loss::convection::evaluate_hc_function(terms, speed_m_s)
    }

    pub fn calculate_heat_loss_gas_radiation(
        input: &soryntel_thermo::heat_loss::GasRadiationInput,
    ) -> Result<soryntel_thermo::heat_loss::GasRadiationResult, SoryntelError> {
        soryntel_thermo::heat_loss::calculate_gas_radiation(input)
    }

    pub fn calculate_heat_loss_gas_particle_radiation(
        gas_input: &soryntel_thermo::heat_loss::GasRadiationInput,
        particle_input: &soryntel_thermo::heat_loss::ParticleCloudInput,
    ) -> Result<(soryntel_thermo::heat_loss::GasRadiationResult, f64, f64, f64), SoryntelError> {
        soryntel_thermo::heat_loss::calculate_gas_particle_radiation(gas_input, particle_input)
    }

    pub fn evaluate_heat_loss_particle_suspension_detailed(
        spec: &soryntel_thermo::heat_loss::ParticleSuspensionSpec,
    ) -> Result<soryntel_thermo::heat_loss::ParticleSuspensionDetailedResult, SoryntelError> {
        soryntel_thermo::heat_loss::evaluate_particle_suspension_detailed(spec)
    }

    pub fn generate_heat_loss_iterative_gas_radiation_table(
        pressure_bar: f64,
        optical_depth_m: f64,
        comp: &soryntel_thermo::heat_loss::GasComposition,
    ) -> Vec<(f64, f64)> {
        soryntel_thermo::heat_loss::generate_iterative_gas_radiation_table(
            pressure_bar,
            optical_depth_m,
            comp,
        )
    }

    pub fn apply_heat_loss_geometry_transition(
        wb: &mut HeatLossWorkbook,
        sheet_idx: usize,
        new_geom: soryntel_thermo::heat_loss::Geometry,
    ) {
        soryntel_thermo::heat_loss::apply_geometry_transition(wb, sheet_idx, new_geom);
    }

    pub fn serialize_htr8_workbook(wb: &HeatLossWorkbook) -> Result<String, String> {
        soryntel_thermo::heat_loss::serialize_htr8_workbook(wb)
    }

    pub fn deserialize_htr8_workbook(raw: &str) -> Result<HeatLossWorkbook, String> {
        soryntel_thermo::heat_loss::deserialize_htr8_workbook(raw)
    }

    pub fn export_heat_loss_workbook_xlsx(
        wb: &HeatLossWorkbook,
        res: Option<&WorkbookResult>,
    ) -> Vec<u8> {
        let sheets = soryntel_thermo::heat_loss::build_heat_loss_export_sheets(wb, res);
        build_xlsx(&sheets)
    }

    pub fn resolve_material_composition(
        db: &soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
    ) -> Result<
        (
            f64,
            Vec<soryntel_thermo::heat_loss::ResolvedSpeciesWt>,
            Vec<soryntel_thermo::heat_loss::ResolvedElementWt>,
        ),
        SoryntelError,
    > {
        db.resolve_composition(material_idx)
    }

    pub fn recalculate_material_auto_properties(
        db: &mut soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
    ) -> Result<(), SoryntelError> {
        db.recalculate_auto_properties(material_idx)
    }

    pub fn sample_material_cp_curve(
        db: &soryntel_thermo::heat_loss::MaterialDatabase,
        material_idx: usize,
        points_per_range: usize,
    ) -> Vec<(f64, f64)> {
        db.sample_cp_curve(material_idx, points_per_range)
    }

    pub fn solve_equilibrium_sweep<R: SpeciesCatalogPort>(
        catalog: &R,
        sys: &EquilibriumSystem,
        params: &EquilibriumParams,
    ) -> Result<EquilibriumSweepResult, SoryntelError> {
        solve_equilibrium_sweep(sys, params, catalog)
    }

    pub fn evaluate_estimate(catalog: &dyn SpeciesCatalogPort, row: &mut EstimateRow) {
        evaluate_estimate_row(row, catalog);
    }

    pub fn calculate_ehph(
        catalog: &dyn SpeciesCatalogPort,
        req: &EhPhRequest,
    ) -> Result<EhPhResult, String> {
        calculate_ehph(req, Some(catalog))
    }

    pub fn calculate_diagram(
        catalog: &dyn SpeciesCatalogPort,
        req: &DiagramRequest,
    ) -> Result<DiagramResult, String> {
        calculate_diagram(req, catalog)
    }

    pub fn calculate_phase_stability(
        catalog: &dyn SpeciesCatalogPort,
        input: &PhaseStabilityInput,
    ) -> Result<PhaseStabilityDiagram, PhaseStabilityError> {
        calculate_phase_stability(catalog, input)
    }

    pub fn initialize_species_converter(
        entries: &[ConverterInputEntry],
    ) -> Result<(Vec<CalculatedElementRow>, f64), SpeciesConverterError> {
        initialize_input(entries)
    }

    pub fn fix_species_balance(
        inputs: &[ConverterInputEntry],
        outputs: &mut Vec<ConverterOutputSpec>,
    ) -> Result<(), SpeciesConverterError> {
        fix_element_balance(inputs, outputs)
    }

    pub fn convert_species(
        catalog: &dyn SpeciesCatalogPort,
        req: &SpeciesConverterRequest,
    ) -> Result<SpeciesConverterResult, SpeciesConverterError> {
        solve_species_conversion(catalog, req)
    }

    pub fn parse_benson_smiles(smiles: &str) -> Result<MoleculeGraph, String> {
        MoleculeGraph::from_smiles(smiles)
    }

    pub fn estimate_benson_properties(
        catalog: &dyn SpeciesCatalogPort,
        mol: &MoleculeGraph,
        custom_groups: Option<&[(String, usize)]>,
        custom_params: Option<&BensonParamsInput>,
    ) -> Result<BensonEstimationResult, String> {
        estimate_benson(mol, custom_groups, custom_params, Some(catalog))
    }

    pub fn save_benson_species(
        catalog: &mut dyn SpeciesCatalogPort,
        est: &BensonEstimationResult,
        phase: soryntel_thermo::Phase,
        custom_formula: &str,
    ) -> ChemicalSpecies {
        let sp = build_benson_chemical_species(est, phase, custom_formula);
        catalog.insert_or_update_species(sp.clone());
        sp
    }

    pub fn default_elements_workbook() -> ElementWorkbook {
        ElementWorkbook::default_workbook()
    }

    pub fn compute_elements_diagram(
        wb: &ElementWorkbook,
        row_idx: usize,
    ) -> Result<PeriodicDiagramData, String> {
        wb.compute_diagram(row_idx)
    }

    pub fn lookup_element_species(
        catalog: &dyn SpeciesCatalogPort,
        symbol: &str,
    ) -> Vec<ChemicalSpecies> {
        domain_lookup_element_species(catalog, symbol)
    }

    pub fn default_water_request() -> WaterCalcRequest {
        domain_default_water_request()
    }

    pub fn calculate_water(
        catalog: &dyn SpeciesCatalogPort,
        req: &WaterCalcRequest,
    ) -> WaterCalcResult {
        domain_calculate_water_points(req, Some(catalog))
    }

    /// Checked entry point for interactive and external requests.
    pub fn try_calculate_water(
        catalog: &dyn SpeciesCatalogPort,
        req: &WaterCalcRequest,
    ) -> Result<WaterCalcResult, SoryntelError> {
        soryntel_thermo::water::validate_water_request(req)?;
        Ok(domain_calculate_water_points(req, Some(catalog)))
    }

    pub fn water_phase_curves() -> WaterDiagramCurves {
        domain_water_diagram_curves()
    }

    pub fn default_units_workbook() -> UnitsWorkbook {
        UnitsWorkbook::default_hsc6()
    }

    pub fn convert_unit_value(
        wb: &UnitsWorkbook,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        unit2_idx: usize,
        prefix2: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Result<f64, String> {
        wb.convert_value(
            qty_idx,
            unit1_idx,
            prefix1,
            unit2_idx,
            prefix2,
            value,
            delta_temp_mode,
        )
    }

    pub fn convert_unit_all_equivalents(
        wb: &UnitsWorkbook,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Vec<(String, String, Result<f64, String>)> {
        wb.convert_all_equivalents(qty_idx, unit1_idx, prefix1, value, delta_temp_mode)
    }

    pub fn compute_air_humidity_point(
        temp_c: f64,
        rh_pct: f64,
        total_p_bar: f64,
    ) -> Result<AirHumidityPoint, String> {
        domain_air_humidity_point(temp_c, rh_pct, total_p_bar)
    }

    pub fn air_humidity_grid_descending() -> AirHumidityGrid {
        domain_air_grid_descending()
    }

    pub fn air_humidity_3d_surface() -> AirHumidityGrid {
        domain_air_3d_surface()
    }

    pub fn search_species(
        catalog: &dyn SpeciesCatalogPort,
        query: &str,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species(query, limit)
    }

    pub fn search_species_by_elements(
        catalog: &dyn SpeciesCatalogPort,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        catalog.search_species_by_elements(
            elements,
            include_gas,
            include_liquid,
            include_solid,
            include_aqua_ions,
            include_aqua_neutrals,
            limit,
        )
    }

    pub fn species_count(catalog: &dyn SpeciesCatalogPort) -> usize {
        catalog.species_count()
    }

    pub fn addin_catalog() -> &'static [AddInFunctionMeta] {
        ADDIN_FUNCTION_CATALOG
    }

    pub fn eval_addin_call(
        catalog: &dyn SpeciesCatalogPort,
        units: &mut AddInUnits,
        fn_name: &str,
        args: &[ArgValue],
    ) -> CellEvalValue {
        eval_addin_function(catalog, units, fn_name, args)
    }

    pub fn eval_addin_formula<F>(
        catalog: &dyn SpeciesCatalogPort,
        units: &mut AddInUnits,
        formula: &str,
        lookup: F,
    ) -> CellEvalValue
    where
        F: FnMut(usize, usize) -> String,
    {
        eval_formula_string(catalog, units, formula, lookup)
    }

    pub fn eval_addin_streameq(
        catalog: &dyn SpeciesCatalogPort,
        units: &AddInUnits,
        species: &[String],
        inputs: &[String],
        t_val: f64,
        p_val: f64,
    ) -> CellEvalValue {
        eval_streameq(catalog, units, species, inputs, t_val, p_val)
    }

    pub fn lookup_addin_mineral(query: &str) -> Option<MineralRecord> {
        lookup_mineral(query)
    }

    pub fn migrate_addin_formula(raw: &str) -> (String, Vec<String>) {
        migrate_legacy_formula(raw)
    }

    pub fn addin_webservice_formula(port: u16, fn_name: &str, args: &[&str]) -> String {
        build_webservice_formula(port, fn_name, args)
    }

    pub fn addin_streameq_sheet(
        catalog: &dyn SpeciesCatalogPort,
        units: &AddInUnits,
        species: &[String],
        inputs: &[String],
        t_val: f64,
        p_val: f64,
    ) -> ExportSheet {
        build_streameq_export_sheet(catalog, units, species, inputs, t_val, p_val)
    }

    pub fn addin_minerals_sheet() -> ExportSheet {
        build_minerals_export_sheet()
    }

    pub fn addin_sample_sheet(catalog: &dyn SpeciesCatalogPort, units: &AddInUnits) -> ExportSheet {
        build_addinsample_export_sheet(catalog, units)
    }

    pub fn addin_streameq_balance(
        species: &[String],
        inputs: &[String],
        outputs: &[CellEvalValue],
    ) -> Vec<(String, f64, f64)> {
        streameq_element_balance(species, inputs, outputs)
    }

    pub fn export_workbook_xlsx(sheets: &[ExportSheet]) -> Vec<u8> {
        build_xlsx(sheets)
    }

    pub fn export_workbook_ods(sheets: &[ExportSheet]) -> Vec<u8> {
        build_ods(sheets)
    }

    pub fn export_libreoffice_oxt(port: u16) -> Vec<u8> {
        build_libreoffice_oxt(port)
    }

    pub fn export_onlyoffice_plugin(port: u16) -> Vec<u8> {
        build_onlyoffice_plugin_zip(port)
    }

    pub fn export_excel_wps_vba(port: u16) -> String {
        generate_excel_wps_vba(port)
    }

    // ---- Module 16: Species & Thermochemical Databases (`HSC - Databases`) ----

    pub fn species_compute_mw(
        formula: &str,
    ) -> Result<(f64, std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        compute_molecular_weight(formula)
    }

    pub fn species_parse_elements(
        formula: &str,
    ) -> Result<(std::collections::BTreeMap<String, f64>, i32), SoryntelError> {
        parse_hsc_formula_elements(formula)
    }

    pub fn species_fit_cp(
        points: &[CpDataPoint],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<CpFitResult, SoryntelError> {
        fit_cp_kelley(points, mask, locked_si)
    }

    pub fn species_convert_cp_function(
        expr: &str,
        unit: &str,
        t1_k: f64,
        t2_k: f64,
        params: &[f64; 6],
        mask: CpFitMask,
        locked_si: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(Vec<CpDataPoint>, CpFitResult), SoryntelError> {
        convert_cp_function(expr, unit, t1_k, t2_k, params, mask, locked_si)
    }

    pub fn species_convert_gibbs(
        input: &GibbsCoeffsInput,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        convert_gibbs_function(input)
    }

    pub fn species_gibbs_delta_to_normal(
        catalog: &dyn SpeciesCatalogPort,
        target_formula: &str,
        delta_out: &GibbsConvertOutput,
        t_ref_k: f64,
    ) -> Result<GibbsConvertOutput, SoryntelError> {
        gibbs_delta_to_normal(catalog, target_formula, delta_out, t_ref_k)
    }

    pub fn species_extrapolate_liquid_298(
        solid_intervals: &[soryntel_thermo::SpeciesThermoInterval],
        t_melt_k: f64,
        delta_h_fus_j: f64,
        liquid_kelley: soryntel_thermo::KelleyCoefficients,
    ) -> Result<(f64, f64), SoryntelError> {
        extrapolate_liquid_298(solid_intervals, t_melt_k, delta_h_fus_j, liquid_kelley)
    }

    pub fn species_analyze_aqueous(species: &ChemicalSpecies) -> Option<AqueousScaleSummary> {
        analyze_aqueous_scales(species)
    }

    pub fn species_color_spec(code: i16) -> &'static HscColorSpec {
        color_spec(code)
    }

    pub fn species_awdb_weight(symbol: &str) -> Option<f64> {
        awdb_atomic_weight(symbol)
    }

    pub fn species_stoichiometry_matches(
        candidate: &std::collections::BTreeMap<String, f64>,
        target: &std::collections::BTreeMap<String, f64>,
        fix_ratio: bool,
    ) -> bool {
        stoichiometry_matches(candidate, target, fix_ratio)
    }

    pub fn aqua_main_pitzer_db() -> PitzerDatabase {
        domain_aqua_main_pitzer_db()
    }

    pub fn aqua_calculate(
        catalog: &dyn SpeciesCatalogPort,
        req: &AquaCaseFile,
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
    ) -> AquaCalcResult {
        domain_aqua_calculate(req, catalog, main_db, own_db, excluded_keys)
    }

    pub fn aqua_analyze_params(
        req: &AquaCaseFile,
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
    ) -> ActiveParameterSetsReport {
        domain_aqua_analyze_parameter_sets(req, main_db, own_db, excluded_keys)
    }

    pub fn aqua_resolve_catalog(
        main_db: &PitzerDatabase,
        own_db: &OwnPitzerDb,
        excluded_keys: &std::collections::BTreeSet<String>,
        use_own_db: bool,
    ) -> MergedPitzerCatalog {
        domain_aqua_resolve_catalog(main_db, own_db, excluded_keys, use_own_db)
    }

    pub fn aqua_density(rows: &[DensityInputRow], t_c: f64) -> DensityCalcResult {
        domain_aqua_density(rows, t_c)
    }

    // ------------------------------------------------------------------
    // Module 18: HSC Sim Common Tools Use Cases
    // ------------------------------------------------------------------
    pub fn sim_presets() -> Vec<(&'static str, SimProject)> {
        vec![
            ("Empty Flowsheet", domain_sim_empty_project()),
            ("40.1_model (Simple 2-Unit)", domain_sim_preset_40_1()),
            ("Hydro_example (Leaching + Cooler)", domain_sim_preset_hydro()),
            ("Concentrator_Flotation (Scenario)", domain_sim_preset_concentrator()),
            ("Recycle_Loop (Tarjan SCC)", domain_sim_preset_recycle()),
            ("Zn_TSL_Fuming (Sim LCA 49.3)", domain_sim_preset_zn_tsl()),
            ("Iron_Process (Sim LCA 49.4)", domain_sim_preset_iron_process()),
        ]
    }

    pub fn sim_model_catalog() -> Vec<UnitModelCatalogEntry> {
        domain_sim_model_catalog()
    }

    pub fn sim_calc_order(project: &SimProject) -> CalcOrderInfo {
        domain_sim_calc_order(project)
    }

    pub fn sim_run_static(project: &mut SimProject, selected_unit_ids: Option<&[usize]>) {
        domain_sim_run_static(project, selected_unit_ids);
    }

    pub fn sim_clear_internal(project: &mut SimProject) {
        domain_sim_clear_internal(project);
    }

    pub fn sim_empty_tanks(project: &mut SimProject) {
        domain_sim_empty_tanks(project);
    }

    pub fn sim_step_dynamic(project: &mut SimProject, steps: usize) {
        domain_sim_step_dynamic(project, steps);
    }

    pub fn sim_run_controls(project: &mut SimProject, only_idx: Option<usize>) {
        domain_sim_run_controls(project, only_idx);
    }

    pub fn sim_control_sweep(project: &SimProject, ctrl_idx: usize, steps: usize) -> Vec<(usize, f64, f64)> {
        domain_sim_control_sweep(project, ctrl_idx, steps)
    }

    pub fn sim_execute_scenario(project: &mut SimProject, scenario_idx: usize, seed: u64) {
        domain_sim_execute_scenario(project, scenario_idx, seed);
    }

    pub fn sim_mc_curves(col: &ScenarioColumn, points: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        domain_sim_eval_mc_curves(col, points)
    }

    pub fn sim_generate_reports(project: &SimProject) -> SimReportWorkbook {
        domain_sim_generate_reports(project)
    }

    pub fn sim_report_csv(sheet: &SimReportSheet) -> String {
        domain_sim_report_csv(sheet)
    }

    pub fn sim_filter_log(msg: &str, severity: LogSeverity, level: LogFilterLevel, query: &str) -> bool {
        domain_sim_filter_log(msg, severity, level, query)
    }

    pub fn sim_export_lca_json(project: &SimProject) -> Result<String, String> {
        domain_sim_export_lca_json(project)
    }

    pub fn sim_export_svg(project: &SimProject) -> String {
        domain_sim_export_svg(project)
    }

    pub fn sim_rewrite_sim7(expr: &str, sheet_name: &str) -> (String, String) {
        domain_sim_rewrite_sim7(expr, sheet_name)
    }

    pub fn sim_metric_value(stream: &soryntel_thermo::sim::model::SimStream, metric: &str) -> (f64, &'static str) {
        domain_sim_metric_value(stream, metric)
    }

    pub fn sim_global_balances(project: &SimProject) -> soryntel_thermo::sim::model::ConvergenceRound {
        domain_sim_eval_balances(project)
    }

    // ------------------------------------------------------------------
    // Module 19: HSC Sim LCA (Classic 49.3 & Beta IPC / LCIA 49.4)
    // ------------------------------------------------------------------
    pub fn sim_lca_gabi_flows() -> Vec<LcaCatalogFlow> {
        domain_sim_lca_gabi_flows()
    }

    pub fn sim_lca_openlca_flows() -> Vec<LcaCatalogFlow> {
        domain_sim_lca_openlca_flows()
    }

    pub fn sim_lca_methods() -> Vec<LciaMethodSpec> {
        domain_sim_lca_methods()
    }

    pub fn sim_lca_classic_sync(state: &mut ClassicLcaState, project: &SimProject) {
        state.sync_from_project(project);
    }

    pub fn sim_lca_classic_normalize(state: &mut ClassicLcaState) -> Result<(), String> {
        state.normalize().map(|_| ())
    }

    pub fn sim_lca_classic_ecospold_xml(state: &mut ClassicLcaState) -> Result<String, String> {
        state.export_ecospold_v1_xml()
    }

    pub fn sim_lca_classic_jsonld_zip(state: &mut ClassicLcaState) -> Result<Vec<u8>, String> {
        state.export_openlca_jsonld_zip()
    }

    pub fn sim_lca_classic_xlsx(state: &mut ClassicLcaState) -> Result<Vec<u8>, String> {
        state.export_xlsx()
    }

    pub fn sim_lca_beta_sync(state: &mut BetaLcaState, project: &SimProject) {
        state.sync_from_project(project);
    }

    pub fn sim_lca_beta_calculate(state: &mut BetaLcaState) -> Result<(), String> {
        state.calculate_results()
    }

    pub fn sim_lca_beta_xlsx(state: &BetaLcaState) -> Vec<u8> {
        state.export_to_xlsx()
    }

    pub fn sim_lca_preset_classic_zn() -> (SimProject, ClassicLcaState) {
        let p = domain_sim_preset_zn_tsl();
        let l = domain_sim_lca_preset_classic_zn(&p);
        (p, l)
    }

    pub fn sim_lca_preset_beta_iron() -> (SimProject, BetaLcaState) {
        let p = domain_sim_preset_iron_process();
        let l = domain_sim_lca_preset_beta_iron(&p);
        (p, l)
    }

    pub fn mb_preset_rougher() -> MassBalanceWorkbook {
        domain_mb_preset_rougher()
    }

    pub fn mb_preset_grinding() -> MassBalanceWorkbook {
        domain_mb_preset_grinding()
    }

    pub fn mb_from_sim_project(project: &SimProject) -> MassBalanceWorkbook {
        MassBalanceWorkbook::from_sim_project(project)
    }

    pub fn mb_save_to_sim_project(wb: &MassBalanceWorkbook, project: &mut SimProject) -> usize {
        wb.save_to_sim_project(project)
    }

    pub fn mb_check_errors(wb: &MassBalanceWorkbook, dataset_idx: usize) -> Vec<ValidationIssue> {
        wb.check_errors(dataset_idx)
    }

    pub fn mb_calculate(
        wb: &mut MassBalanceWorkbook,
        dataset_idx: usize,
    ) -> Result<MbSolution, String> {
        wb.calculate_dataset(dataset_idx)
    }

    pub fn mb_calculate_batch(wb: &mut MassBalanceWorkbook) -> Vec<Result<MbSolution, String>> {
        wb.calculate_all_batch()
    }

    pub fn mb_import_hsc7(
        wb: &mut MassBalanceWorkbook,
        flowsheet_csv: Option<&str>,
        data_csv: &str,
        horizontal: bool,
    ) -> Result<usize, String> {
        wb.import_hsc7_text(flowsheet_csv, data_csv, horizontal)
    }

    pub fn mb_export_xlsx(wb: &MassBalanceWorkbook) -> Result<Vec<u8>, String> {
        wb.export_results_xlsx()
    }
}
