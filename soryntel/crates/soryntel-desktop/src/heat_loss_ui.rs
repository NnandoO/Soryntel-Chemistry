//! Module 3 Desktop UI: Heat Loss / Heat Transfer Calculations (`12-heat-loss.md`, Figs. 1–14)
//! and Material Database (`90-material-database.md`, Figs. 1–3).

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, section_cell, text_cell, widget_cell,
    EXCEL_READONLY_BG, EXCEL_SECTION_BG, EXCEL_YELLOW_BG,
};
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use soryntel_application::SoryntelApplication;
use soryntel_core::{HeatFlowUnit, LengthUnit, TemperatureUnit};
use soryntel_thermo::heat_loss::convection::ConvectionDiagnostics;
use soryntel_thermo::heat_loss::diagram::{
    OptimumInsulationPoint, OptimumInsulationSpec, SweepParameter, SweepPoint,
    TemperatureProfileSeries,
};
use soryntel_thermo::heat_loss::gas_radiation::{
    GasRadiationInput, GasRadiationResult, OpticalDepthGeometry,
    ParticleSuspensionDetailedResult, ParticleSuspensionSpec,
};
use soryntel_thermo::heat_loss::material_db::{
    MaterialCompositionRow, MaterialCpRange, MaterialDatabase, MaterialUserProperty,
};
use soryntel_thermo::heat_loss::model::{
    CalculationMode, ColumnType, ConvectionFunctionPreset, ConvectionGeometry, ConvectionRegime,
    FluidPropertyTable, Geometry, HeatLossColumn, HeatLossWorkbook, SolidConductivityData,
    SurfaceEmissivityData, WorkbookResult,
};
use soryntel_thermo::heat_loss::presets::{
    builtin_convection_functions, builtin_emissivities, builtin_fluids, builtin_solids,
    preset_hotwatertank, preset_oil_furnace, preset_oil_furnace_gas, preset_radiator,
    preset_simple_case, preset_smelting1,
};
use soryntel_thermo::heat_loss::target::{TargetGoal, TargetSpec, TargetVariable};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagramKind {
    TemperatureProfile,
    ParameterSweep,
    OptimumInsulation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConvectionDbTab {
    Table,
    Function,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GasBandTab {
    Gas,
    Wall,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterialLeftTab {
    Composition,
    Analyses,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterialRightTopTab {
    Common,
    CpData,
    CpChart,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterialRightBottomTab {
    GeneralInfo,
    Properties,
    Data,
    UserProperties,
}

#[derive(Clone)]
pub(crate) struct TargetDialogRow {
    pub(crate) var: TargetVariable,
    pub(crate) test_val: f64,
    pub(crate) min_val: f64,
    pub(crate) max_val: f64,
    pub(crate) target_name: String,
    pub(crate) target_val_kw: f64,
    pub(crate) max_iters: usize,
}

pub struct HeatLossModuleState {
    pub wb: HeatLossWorkbook,
    pub result: Option<WorkbookResult>,
    pub selected_col: usize,
    pub show_advanced_rows: bool,
    pub solids: Vec<SolidConductivityData>,
    pub fluids: Vec<FluidPropertyTable>,
    pub conv_functions: Vec<ConvectionFunctionPreset>,
    pub emissivities: Vec<SurfaceEmissivityData>,
    pub material_db: MaterialDatabase,
    pub status_msg: String,
    pub status_ok: bool,
    pub file_path_input: String,
    // 1. Conduction DB (Fig. 5)
    pub show_cond_picker: bool,
    pub cond_search: String,
    pub cond_selected_idx: usize,
    // 2. Convection DB (Figs. 6-7)
    pub show_conv_calc: bool,
    pub conv_tab: ConvectionDbTab,
    pub conv_diag: Option<ConvectionDiagnostics>,
    pub conv_selected_fluid: usize,
    pub conv_selected_func: usize,
    pub conv_test_speed: f64,
    pub conv_find_query: String,
    // 3. Surface Radiation DB (Fig. 8)
    pub show_rad_picker: bool,
    pub rad_search: String,
    pub rad_selected_idx: usize,
    // 4. Gas Mixture Radiation (Fig. 9)
    pub show_gas_rad: bool,
    pub gas_band_tab: GasBandTab,
    pub gas_input: GasRadiationInput,
    pub gas_result: Option<(GasRadiationResult, f64, f64)>,
    // 5. Particle Suspension Radiation (Fig. 10)
    pub show_particle_rad: bool,
    pub show_particle_image: bool,
    pub particle_spec: ParticleSuspensionSpec,
    pub particle_detailed_res: Option<ParticleSuspensionDetailedResult>,
    // 6. Target Dialog (Fig. 11)
    pub show_target: bool,
    pub target_rows: Vec<TargetDialogRow>,
    pub selected_target_row: usize,
    pub target_msg: String,
    // 7. Diagrams (Figs. 3, 12, 13, 14)
    pub show_diagram: bool,
    pub diagram_show_table: bool,
    pub diagram_show_grid: bool,
    pub diagram_auto_scale: bool,
    pub diagram_x_min: f64,
    pub diagram_x_max: f64,
    pub diagram_y_min: f64,
    pub diagram_y_max: f64,
    pub diagram_apply_bounds: bool,
    pub show_shape_draw: bool,
    pub diagram_kind: DiagramKind,
    pub profile_series: Vec<TemperatureProfileSeries>,
    pub profile_total_w: f64,
    pub sweep_param: SweepParameter,
    pub sweep_min: f64,
    pub sweep_max: f64,
    pub sweep_steps: usize,
    pub sweep_points: Vec<SweepPoint>,
    pub opt_spec: OptimumInsulationSpec,
    pub opt_points: Vec<OptimumInsulationPoint>,
    pub opt_best: Option<OptimumInsulationPoint>,
    // 8. Material Database (Ch. 90, Figs. 1-3)
    pub show_material_db: bool,
    pub selected_material_idx: usize,
    pub mat_left_tab: MaterialLeftTab,
    pub mat_right_top_tab: MaterialRightTopTab,
    pub mat_right_btm_tab: MaterialRightBottomTab,
    // Export to Module 2
    pub export_target_area: usize,
    pub pending_export_w: Option<f64>,
}

impl HeatLossModuleState {
    pub fn new() -> Self {
        let wb = preset_hotwatertank();
        let mut particle_spec = ParticleSuspensionSpec::default();
        let particle_detailed_res =
            SoryntelApplication::evaluate_heat_loss_particle_suspension_detailed(&particle_spec)
                .ok();
        if let Some(pr) = &particle_detailed_res {
            let _ = pr;
        }
        let _ = &mut particle_spec;

        let mut state = Self {
            wb,
            result: None,
            selected_col: 0,
            show_advanced_rows: false,
            solids: builtin_solids(),
            fluids: builtin_fluids(),
            conv_functions: builtin_convection_functions(),
            emissivities: builtin_emissivities(),
            material_db: MaterialDatabase::with_hsc_defaults(),
            status_msg: "Listo (Hotwatertank.htr8 cargado).".into(),
            status_ok: true,
            file_path_input: "Hotwatertank.htr8".into(),
            show_cond_picker: false,
            cond_search: String::new(),
            cond_selected_idx: 0,
            show_conv_calc: false,
            conv_tab: ConvectionDbTab::Table,
            conv_diag: None,
            conv_selected_fluid: 0,
            conv_selected_func: 0,
            conv_test_speed: 0.01,
            conv_find_query: String::new(),
            show_rad_picker: false,
            rad_search: String::new(),
            rad_selected_idx: 0,
            show_gas_rad: false,
            gas_band_tab: GasBandTab::Gas,
            gas_input: GasRadiationInput {
                t_gas_k: 232.0 + 273.15,
                t_surface_k: 23.0 + 273.15,
                pressure_bar: 1.0,
                optical_depth_m: 0.450,
                surface_emissivity: 1.0,
                composition: soryntel_thermo::heat_loss::GasComposition {
                    x_h2o: 0.05,
                    x_co2: 0.10,
                    x_co: 0.0,
                    x_no: 0.0,
                    x_so2: 0.0,
                    x_ch4: 0.0,
                },
            },
            gas_result: None,
            show_particle_rad: false,
            show_particle_image: false,
            particle_spec,
            particle_detailed_res,
            show_target: false,
            target_rows: vec![TargetDialogRow {
                var: TargetVariable::Width,
                test_val: 2.00,
                min_val: 0.50,
                max_val: 10.00,
                target_name: "Heat flow".into(),
                target_val_kw: 0.50,
                max_iters: 10,
            }],
            selected_target_row: 0,
            target_msg: String::new(),
            show_diagram: false,
            diagram_show_table: false,
            diagram_show_grid: true,
            diagram_auto_scale: true,
            diagram_x_min: 0.0,
            diagram_x_max: 60.0,
            diagram_y_min: 20.0,
            diagram_y_max: 60.0,
            diagram_apply_bounds: false,
            show_shape_draw: false,
            diagram_kind: DiagramKind::TemperatureProfile,
            profile_series: Vec::new(),
            profile_total_w: 0.0,
            sweep_param: SweepParameter::LeftBoundaryTempK,
            sweep_min: 303.15,
            sweep_max: 363.15,
            sweep_steps: 12,
            sweep_points: Vec::new(),
            opt_spec: OptimumInsulationSpec {
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
            opt_points: Vec::new(),
            opt_best: None,
            show_material_db: false,
            selected_material_idx: 0,
            mat_left_tab: MaterialLeftTab::Composition,
            mat_right_top_tab: MaterialRightTopTab::Common,
            mat_right_btm_tab: MaterialRightBottomTab::Properties,
            export_target_area: 0,
            pending_export_w: None,
        };
        if let Ok(gr) = SoryntelApplication::calculate_heat_loss_gas_radiation(&state.gas_input) {
            state.gas_result = Some((gr, 0.0, 0.0));
        }
        state.recalculate();
        state
    }

    pub fn recalculate(&mut self) {
        match SoryntelApplication::solve_heat_loss_workbook(&self.wb) {
            Ok(res) => {
                let mut warn_count = 0usize;
                for sr in &res.sheet_results {
                    warn_count += sr.warnings.len();
                }
                let q_str = format!(
                    "Total Heat Flow: {:.3} {}",
                    res.total_heat_flow_w * self.wb.heat_flow_unit.from_watts_factor(),
                    self.wb.heat_flow_unit.label()
                );
                if warn_count > 0 {
                    self.status_msg = format!("⚠ {q_str} ({warn_count} advertencia(s) Tmin/Tmax)");
                } else {
                    self.status_msg = format!("✔ {q_str}");
                }
                self.status_ok = true;
                self.result = Some(res);
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn save_htr8(&mut self, path: &str) {
        match SoryntelApplication::serialize_htr8_workbook(&self.wb) {
            Ok(json) => match crate::storage::write_atomic(path, json) {
                Ok(_) => {
                    self.status_msg = format!("✔ Guardado en {path}");
                    self.status_ok = true;
                }
                Err(e) => {
                    self.status_msg = format!("Error guardando {path}: {e}");
                    self.status_ok = false;
                }
            },
            Err(e) => {
                self.status_msg = e;
                self.status_ok = false;
            }
        }
    }

    pub fn load_htr8(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(raw) => match SoryntelApplication::deserialize_htr8_workbook(&raw) {
                Ok(wb) => {
                    self.wb = wb;
                    self.selected_col = 0;
                    self.recalculate();
                    self.status_msg = format!("📂 Cargado {path}");
                    self.status_ok = true;
                }
                Err(e) => {
                    self.status_msg = e;
                    self.status_ok = false;
                }
            },
            Err(e) => {
                self.status_msg = format!("Error leyendo {path}: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn export_xlsx(&mut self, path: &str) {
        let bytes =
            SoryntelApplication::export_heat_loss_workbook_xlsx(&self.wb, self.result.as_ref());
        match crate::storage::write_atomic(path, bytes) {
            Ok(_) => {
                self.status_msg = format!("📊 Exportado Excel a {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error exportando {path}: {e}");
                self.status_ok = false;
            }
        }
    }

    fn load_preset(&mut self, wb: HeatLossWorkbook) {
        self.file_path_input = wb.title.clone();
        self.wb = wb;
        self.selected_col = 0;
        self.recalculate();
    }

    pub fn ui(&mut self, ctx: &egui::Context, balance_area_count: usize) {
        let yellow_bg = egui::Color32::from_rgb(254, 249, 195);
        let red_fix = egui::Color32::from_rgb(185, 28, 28);

        // Top HSC Menu Bar + Quick Icon Toolbar (Figs. 1, 2, 4)
        egui::TopBottomPanel::top("hl_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(crate::i18n::text("common.020"), |ui| {
                    if ui.button(crate::i18n::text("review.140")).clicked() {
                        let p = self.file_path_input.clone();
                        self.load_htr8(&p);
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.141")).clicked() {
                        let p = self.file_path_input.clone();
                        self.save_htr8(&p);
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.142")).clicked() {
                        let p = format!("{}.xlsx", self.file_path_input.trim_end_matches(".htr8"));
                        self.export_xlsx(&p);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("HSC Examples (Ch. 12.2.1)").strong().small());
                    if ui.button("🔥 Smelting1.htr8 (Fig. 1 — 1200/1400/1600°C)").clicked() {
                        self.load_preset(preset_smelting1());
                        ui.close_menu();
                    }
                    if ui.button("🔥 Radiator.htr8 (Fig. 2 — 40/50/60°C)").clicked() {
                        self.load_preset(preset_radiator());
                        ui.close_menu();
                    }
                    if ui.button("💧 Hotwatertank.htr8 (Fig. 4 — 1.378 kW)").clicked() {
                        self.load_preset(preset_hotwatertank());
                        ui.close_menu();
                    }
                    if ui.button("📦 SimpleCase.htr8 (735 W)").clicked() {
                        self.load_preset(preset_simple_case());
                        ui.close_menu();
                    }
                    if ui.button("🏭 OilFurnace.htr8 (Internal Gas Radiation)").clicked() {
                        self.load_preset(preset_oil_furnace());
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.021"), |ui| {
                    if ui.button(crate::i18n::text("review.143")).clicked() {
                        self.recalculate();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📦 Conduction Database (HeatCond.XLS)...").clicked() {
                        self.show_cond_picker = true;
                        ui.close_menu();
                    }
                    if ui.button("💨 Convection Database (HeatConv.XLS)...").clicked() {
                        self.show_conv_calc = true;
                        ui.close_menu();
                    }
                    if ui.button("☀ Radiation Database (HeatRadi.XLS)...").clicked() {
                        self.show_rad_picker = true;
                        ui.close_menu();
                    }
                    if ui.button("🗄 Material Database (Ch. 90)...").clicked() {
                        self.show_material_db = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.039"), |ui| {
                    if ui
                        .checkbox(
                            &mut self.show_advanced_rows,
                            "📋 Advanced View (Rows 38..96+, Fig. 4)",
                        )
                        .changed()
                    {
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📦 Conduction Database (Fig. 5)...").clicked() {
                        self.show_cond_picker = true;
                        ui.close_menu();
                    }
                    if ui.button("💨 Convection Calculator (Figs. 6-7)...").clicked() {
                        self.show_conv_calc = true;
                        ui.close_menu();
                    }
                    if ui.button("☀ Surface Radiation (Fig. 8)...").clicked() {
                        self.show_rad_picker = true;
                        ui.close_menu();
                    }
                    if ui.button("💨 Gas Mixture Radiation (Fig. 9)...").clicked() {
                        self.show_gas_rad = true;
                        ui.close_menu();
                    }
                    if ui.button("✨ Particle Suspension Radiation (Fig. 10)...").clicked() {
                        self.show_particle_rad = true;
                        ui.close_menu();
                    }
                    if ui.button("🗄 Material Database (Ch. 90, Figs. 1-3)...").clicked() {
                        self.show_material_db = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.040"), |ui| {
                    let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                    if ui.button(crate::i18n::text("review.144")).clicked() {
                        self.wb.sheets[s_idx].columns.push(HeatLossColumn {
                            column_type: ColumnType::Surface,
                            layer_material: "Air (gas)".into(),
                            ..Default::default()
                        });
                        self.recalculate();
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.145")).clicked() {
                        self.wb.sheets[s_idx].columns.push(HeatLossColumn::default());
                        self.recalculate();
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.146")).clicked() {
                        self.wb.sheets[s_idx].columns.push(HeatLossColumn {
                            column_type: ColumnType::Enclosure,
                            layer_material: "Air (gas)".into(),
                            thickness_m: 0.05,
                            hc_fixed: false,
                            hr_fixed: false,
                            fluid_data: Some(self.fluids[0].clone()),
                            convection_geometry: ConvectionGeometry::VerticalEnclosure,
                            ..Default::default()
                        });
                        self.recalculate();
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.147")).clicked() {
                        self.wb.sheets[s_idx].columns.push(HeatLossColumn {
                            column_type: ColumnType::LayerContact,
                            layer_material: "Contact resistance".into(),
                            contact_resistance_k_w: 0.005,
                            ..Default::default()
                        });
                        self.recalculate();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📑 Own / Calculation Sheet").clicked() {
                        let n = self.wb.sheets.len() + 1;
                        let mut new_sheet = self.wb.sheets[0].clone();
                        new_sheet.name = format!("Sheet {n}");
                        self.wb.sheets.push(new_sheet);
                        self.wb.active_sheet = self.wb.sheets.len() - 1;
                        self.recalculate();
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.041"), |ui| {
                    let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                    if ui.button(crate::i18n::text("review.148")).clicked()
                        && self.wb.sheets[s_idx].columns.len() > 1
                        && self.selected_col < self.wb.sheets[s_idx].columns.len()
                    {
                        self.wb.sheets[s_idx].columns.remove(self.selected_col);
                        self.selected_col =
                            self.selected_col.min(self.wb.sheets[s_idx].columns.len() - 1);
                        self.recalculate();
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("common.051")).clicked() && self.wb.sheets.len() > 1 {
                        self.wb.sheets.remove(s_idx);
                        self.wb.active_sheet = s_idx.min(self.wb.sheets.len() - 1);
                        self.recalculate();
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("review.149"), |ui| {
                    ui.label(crate::i18n::text("common.052"));
                    ui.radio_value(&mut self.wb.temp_unit, TemperatureUnit::Celsius, "°C");
                    ui.radio_value(&mut self.wb.temp_unit, TemperatureUnit::Kelvin, "K");
                    ui.separator();
                    ui.label(crate::i18n::text("review.150"));
                    for lu in [
                        LengthUnit::Meter,
                        LengthUnit::Centimeter,
                        LengthUnit::Millimeter,
                    ] {
                        ui.radio_value(&mut self.wb.length_unit, lu, lu.label());
                    }
                    ui.separator();
                    ui.label(crate::i18n::text("review.151"));
                    for u in [
                        HeatFlowUnit::Watt,
                        HeatFlowUnit::Kilowatt,
                        HeatFlowUnit::Megawatt,
                        HeatFlowUnit::KcalPerHour,
                        HeatFlowUnit::BtuPerHour,
                    ] {
                        ui.radio_value(&mut self.wb.heat_flow_unit, u, u.label());
                    }
                });
                ui.menu_button(crate::i18n::text("common.008"), |ui| {
                    let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                    if ui.button(crate::i18n::text("review.152")).clicked() {
                        self.wb.sheets[s_idx].mode = CalculationMode::HeatFlow;
                        self.recalculate();
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.153")).clicked() {
                        self.wb.sheets[s_idx].mode = CalculationMode::TemperatureProfile;
                        self.recalculate();
                        ui.close_menu();
                    }
                });
                if ui.button(crate::i18n::text("common.044")).clicked() {
                    self.show_target = true;
                }
                if ui.button(crate::i18n::text("common.012")).clicked() {
                    self.refresh_diagram();
                    self.diagram_show_table = false;
                    self.show_diagram = true;
                }
                if ui.button(crate::i18n::text("review.154")).clicked() {
                    self.show_material_db = true;
                }
                ui.menu_button(crate::i18n::text("common.022"), |ui| {
                    ui.label("HSC Heat Loss & Material Database (Ch. 12 & Ch. 90)");
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📤 Send Heat Loss -> Module 2").clicked() {
                        if let Some(res) = &self.result {
                            self.pending_export_w = Some(res.total_heat_flow_w);
                        }
                    }
                    if balance_area_count > 0 {
                        egui::ComboBox::from_id_salt("hl_export_area")
                            .width(80.0)
                            .selected_text(format!("Area {}", self.export_target_area + 1))
                            .show_ui(ui, |ui| {
                                for idx in 0..balance_area_count {
                                    ui.selectable_value(
                                        &mut self.export_target_area,
                                        idx,
                                        format!("Area {}", idx + 1),
                                    );
                                }
                            });
                    }
                    ui.separator();
                    if ui.small_button("🏭 OilFurnace").clicked() {
                        self.load_preset(preset_oil_furnace());
                    }
                    if ui.small_button("📦 SimpleCase").clicked() {
                        self.load_preset(preset_simple_case());
                    }
                    if ui.small_button("💧 Hotwatertank").clicked() {
                        self.load_preset(preset_hotwatertank());
                    }
                    if ui.small_button("🔥 Radiator").clicked() {
                        self.load_preset(preset_radiator());
                    }
                    if ui.small_button("🔥 Smelting1").clicked() {
                        self.load_preset(preset_smelting1());
                    }
                });
            });

            ui.separator();
            // Quick Icon Action Bar + Context-Sensitive "Get Data for Column" Frame (Ch. 12.1 step 4, Fig. 4 item 5)
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.file_path_input)
                        .desired_width(145.0)
                        .hint_text("File.htr8"),
                );
                if ui.button(crate::i18n::text("common.009")).on_hover_text("Abrir archivo .htr8").clicked() {
                    let p = self.file_path_input.clone();
                    self.load_htr8(&p);
                }
                if ui.button(crate::i18n::text("common.014")).on_hover_text("Guardar archivo .htr8").clicked() {
                    let p = self.file_path_input.clone();
                    self.save_htr8(&p);
                }
                if ui.button("📊 .xlsx").on_hover_text("Exportar a Excel").clicked() {
                    let p = format!("{}.xlsx", self.file_path_input.trim_end_matches(".htr8"));
                    self.export_xlsx(&p);
                }
                ui.separator();

                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                let n_cols = self.wb.sheets[s_idx].columns.len();
                let active_ct = self
                    .wb
                    .sheets[s_idx]
                    .columns
                    .get(self.selected_col)
                    .map(|c| c.column_type)
                    .unwrap_or(ColumnType::SolidLayer);
                let has_right_solid_or_contact = self
                    .wb
                    .sheets[s_idx]
                    .columns
                    .get(self.selected_col + 1)
                    .map(|c| matches!(c.column_type, ColumnType::SolidLayer | ColumnType::LayerContact))
                    .unwrap_or(false);
                let adj_enclosure = (self.selected_col > 0
                    && self.wb.sheets[s_idx]
                        .columns
                        .get(self.selected_col - 1)
                        .map(|c| c.column_type == ColumnType::Enclosure)
                        .unwrap_or(false))
                    || (self.selected_col + 1 < n_cols
                        && self.wb.sheets[s_idx]
                            .columns
                            .get(self.selected_col + 1)
                            .map(|c| c.column_type == ColumnType::Enclosure)
                            .unwrap_or(false));

                ui.label(egui::RichText::new(crate::i18n::text("review.155")).strong().small());
                if ui
                    .add_enabled(
                        active_ct == ColumnType::SolidLayer,
                        egui::Button::new("📦 Conduction"),
                    )
                    .on_hover_text("Base de datos de conducción térmica (HeatCond.XLS, Fig. 5)")
                    .clicked()
                {
                    self.show_cond_picker = true;
                }
                if ui
                    .add_enabled(
                        matches!(active_ct, ColumnType::Surface | ColumnType::Enclosure),
                        egui::Button::new("💨 Convection"),
                    )
                    .on_hover_text("Base de datos de convección Table/Function (Figs. 6-7)")
                    .clicked()
                {
                    self.show_conv_calc = true;
                }
                if ui
                    .add_enabled(
                        active_ct == ColumnType::Surface
                            || (active_ct == ColumnType::SolidLayer && adj_enclosure),
                        egui::Button::new("☀ Radiation (surface)"),
                    )
                    .on_hover_text("Base de datos de emisividad superficial (Fig. 8)")
                    .clicked()
                {
                    self.show_rad_picker = true;
                }
                if ui
                    .add_enabled(
                        active_ct == ColumnType::Surface && (has_right_solid_or_contact || n_cols >= 1),
                        egui::Button::new("💨 Radiation (gas)"),
                    )
                    .on_hover_text("Calculadora de radiación de mezcla de gases EWBM (Fig. 9)")
                    .clicked()
                {
                    self.show_gas_rad = true;
                }
                if ui
                    .add_enabled(
                        active_ct == ColumnType::Surface && (has_right_solid_or_contact || n_cols >= 1),
                        egui::Button::new("✨ Radiation (particles)"),
                    )
                    .on_hover_text("Calculadora de radiación de suspensión de partículas (Fig. 10)")
                    .clicked()
                {
                    self.show_particle_rad = true;
                }
                ui.separator();
                ui.checkbox(&mut self.show_advanced_rows, "📋 Advanced Rows");
                if ui
                    .button("🗄 Material DB (Ch. 90)")
                    .on_hover_text("Abrir módulo Material Database (Figs. 1-3)")
                    .clicked()
                {
                    self.show_material_db = true;
                }
            });
        });

        // Bottom 3-block panel (matching Figs. 1, 2, 4)
        egui::TopBottomPanel::bottom("hl_bottom_panel")
            .resizable(true)
            .default_height(170.0)
            .min_height(95.0)
            .max_height(420.0)
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.columns(3, |cols| {
                        // Block 1: Total Heat Flow & Shape and Dimensions (Figs. 1, 2, 4)
                        cols[0].group(|ui| {
                            let total_w = self
                                .result
                                .as_ref()
                                .map(|r| r.total_heat_flow_w)
                                .unwrap_or(0.0);
                            let q_disp = total_w * self.wb.heat_flow_unit.from_watts_factor();
                            ui.horizontal(|ui| {
                                ui.strong("TOTAL HEAT FLOW:");
                                egui::Frame::none()
                                    .fill(yellow_bg)
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(180, 180, 180),
                                    ))
                                    .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.strong(format!("{q_disp:.3}"));
                                    });
                                egui::ComboBox::from_id_salt("hl_q_unit_btm")
                                    .width(58.0)
                                    .selected_text(self.wb.heat_flow_unit.label())
                                    .show_ui(ui, |ui| {
                                        for u in [
                                            HeatFlowUnit::Watt,
                                            HeatFlowUnit::Kilowatt,
                                            HeatFlowUnit::Megawatt,
                                            HeatFlowUnit::KcalPerHour,
                                            HeatFlowUnit::BtuPerHour,
                                        ] {
                                            ui.selectable_value(
                                                &mut self.wb.heat_flow_unit,
                                                u,
                                                u.label(),
                                            );
                                        }
                                    });
                            });
                            ui.separator();
                            ui.label(egui::RichText::new(crate::i18n::text("review.156")).strong().small());
                            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                            let mut changed = false;
                            let mut pending_geom: Option<Geometry> = None;
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let geom = &self.wb.sheets[s_idx].geometry;
                                    let is_wall = matches!(geom, Geometry::Wall { .. });
                                    let is_cube = matches!(geom, Geometry::Cube { .. });
                                    let is_cyl = matches!(geom, Geometry::Cylinder { .. });
                                    let is_sph = matches!(geom, Geometry::Sphere { .. });
                                    if ui.radio(is_wall, "Wall").clicked() && !is_wall {
                                        pending_geom = Some(Geometry::Wall {
                                            width_m: 1.0,
                                            length_m: 1.0,
                                        });
                                    }
                                    if ui.radio(is_cube, "Cube").clicked() && !is_cube {
                                        pending_geom = Some(Geometry::Cube {
                                            width_m: 1.0,
                                            length_m: 1.0,
                                            height_m: 1.0,
                                        });
                                    }
                                    if ui.radio(is_cyl, "Cylinder").clicked() && !is_cyl {
                                        pending_geom = Some(Geometry::Cylinder {
                                            height_m: 4.0,
                                            diameter_m: 3.0,
                                        });
                                    }
                                    if ui.radio(is_sph, "Sphere").clicked() && !is_sph {
                                        pending_geom = Some(Geometry::Sphere { diameter_m: 2.0 });
                                    }
                                });
                                ui.add_space(6.0);
                                if ui.button(crate::i18n::text("review.157")).clicked() {
                                    self.show_shape_draw = true;
                                }
                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    let geom = &mut self.wb.sheets[s_idx].geometry;
                                    match geom {
                                        Geometry::Wall { width_m, length_m } => {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.053"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(width_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.054"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(length_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                        }
                                        Geometry::Cube {
                                            width_m,
                                            length_m,
                                            height_m,
                                        } => {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.053"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(width_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.054"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(length_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.055"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(height_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                        }
                                        Geometry::Cylinder {
                                            height_m,
                                            diameter_m,
                                        } => {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.055"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(height_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.056"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(diameter_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                        }
                                        Geometry::Sphere { diameter_m } => {
                                            ui.horizontal(|ui| {
                                                ui.label(crate::i18n::text("common.056"));
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(diameter_m)
                                                            .speed(0.05)
                                                            .range(0.01..=1000.0)
                                                            .fixed_decimals(2),
                                                    )
                                                    .changed();
                                                ui.label("m");
                                            });
                                        }
                                    }
                                });
                            });
                            if let Some(ng) = pending_geom {
                                SoryntelApplication::apply_heat_loss_geometry_transition(
                                    &mut self.wb,
                                    s_idx,
                                    ng,
                                );
                                changed = true;
                            }
                            if changed {
                                self.recalculate();
                            }
                        });

                        // Block 2: Calculate (Figs. 1, 2, 4)
                        cols[1].group(|ui| {
                            ui.label(egui::RichText::new(crate::i18n::text("common.011")).strong().small());
                            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                            let is_tp =
                                self.wb.sheets[s_idx].mode == CalculationMode::TemperatureProfile;
                            let bw = ui.available_width().min(185.0);
                            ui.vertical_centered(|ui| {
                                if ui
                                    .add_sized(
                                        [bw, 22.0],
                                        egui::Button::new("🔥 Temperature profile").selected(is_tp),
                                    )
                                    .clicked()
                                {
                                    self.wb.sheets[s_idx].mode =
                                        CalculationMode::TemperatureProfile;
                                    self.recalculate();
                                }
                                if ui
                                    .add_sized(
                                        [bw, 22.0],
                                        egui::Button::new("🔥 Heat flow").selected(!is_tp),
                                    )
                                    .clicked()
                                {
                                    self.wb.sheets[s_idx].mode = CalculationMode::HeatFlow;
                                    self.recalculate();
                                }
                            });
                            if is_tp {
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("review.158"));
                                    if ui
                                        .add(
                                            egui::DragValue::new(
                                                &mut self.wb.sheets[s_idx].fixed_heat_flow_w,
                                            )
                                            .speed(10.0),
                                        )
                                        .changed()
                                    {
                                        self.recalculate();
                                    }
                                });
                            }
                            // Visual [RED] HOT => COLD [BLUE] indicator (Figs. 1, 2, 4)
                            ui.vertical_centered(|ui| {
                                ui.horizontal(|ui| {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(220, 38, 38))
                                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new("  ").small());
                                        });
                                    egui::Frame::none()
                                        .fill(egui::Color32::WHITE)
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(200, 200, 200),
                                        ))
                                        .inner_margin(egui::Margin::symmetric(10.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(crate::i18n::text("review.159")).strong().small(),
                                            );
                                        });
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(37, 99, 235))
                                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new("  ").small());
                                        });
                                });
                            });
                            let mut all = self.wb.calculate_all_sheets;
                            if ui.radio(all, "Calculate all sheets").clicked() {
                                all = true;
                            }
                            if ui.radio(!all, "Calculate active sheet").clicked() {
                                all = false;
                            }
                            if all != self.wb.calculate_all_sheets {
                                self.wb.calculate_all_sheets = all;
                                self.recalculate();
                            }
                        });

                        // Block 3: Options & Calculators (Figs. 1, 2, 4)
                        cols[2].group(|ui| {
                            ui.label(egui::RichText::new(crate::i18n::text("common.109")).strong().small());
                            let bw = ui.available_width().min(170.0);
                            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                            let is_cur_fixed = self
                                .wb
                                .sheets[s_idx]
                                .columns
                                .get(self.selected_col)
                                .map(|c| match c.column_type {
                                    ColumnType::SolidLayer => c.k_fixed,
                                    ColumnType::Surface | ColumnType::Enclosure => c.hc_fixed,
                                    ColumnType::LayerContact => true,
                                })
                                .unwrap_or(false);
                            let fix_caption = if is_cur_fixed {
                                "📌 Unfix Value"
                            } else {
                                "📌 Fix Value"
                            };
                            ui.vertical_centered(|ui| {
                                if ui.add_sized([bw, 22.0], egui::Button::new(fix_caption)).clicked()
                                {
                                    if let Some(col) =
                                        self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                    {
                                        match col.column_type {
                                            ColumnType::SolidLayer => col.k_fixed = !col.k_fixed,
                                            ColumnType::Surface | ColumnType::Enclosure => {
                                                col.hc_fixed = !col.hc_fixed
                                            }
                                            ColumnType::LayerContact => {}
                                        }
                                        self.recalculate();
                                    }
                                }
                                if ui
                                    .add_sized([bw, 22.0], egui::Button::new("📋 Diagram Table"))
                                    .clicked()
                                {
                                    self.refresh_diagram();
                                    self.diagram_show_table = true;
                                    self.show_diagram = true;
                                }
                                if ui
                                    .add_sized([bw, 22.0], egui::Button::new("📈 Draw Diagram"))
                                    .clicked()
                                {
                                    self.refresh_diagram();
                                    self.diagram_show_table = false;
                                    self.show_diagram = true;
                                }
                            });
                            ui.separator();
                            let status_color = if self.status_ok {
                                egui::Color32::DARK_GREEN
                            } else {
                                red_fix
                            };
                            ui.colored_label(status_color, &self.status_msg);
                        });
                    });
                });
            });

        // Central Spreadsheet Grid + Excel Bottom Sheet Tabs (Figs. 1, 2, 4)
        egui::CentralPanel::default().show(ctx, |ui| {
            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
            let t_unit = self.wb.temp_unit;
            let q_unit = self.wb.heat_flow_unit;
            let l_unit = self.wb.length_unit;

            let sheet_res = self.result.as_ref().and_then(|r| {
                r.sheet_results
                    .iter()
                    .find(|sr| sr.sheet_name == self.wb.sheets[s_idx].name)
                    .or_else(|| r.sheet_results.first())
                    .cloned()
            });

            if let Some(sr) = &sheet_res {
                if !sr.warnings.is_empty() {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(254, 243, 199))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(217, 119, 6)))
                        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                        .show(ui, |ui| {
                            for w in &sr.warnings {
                                ui.colored_label(
                                    egui::Color32::from_rgb(146, 64, 14),
                                    format!("⚠ {w}"),
                                );
                            }
                        });
                }
            }

            let mut changed = false;
            let dark = egui::Color32::from_rgb(24, 24, 27);
            let lbl_w = 190.0;
            let unit_w = 78.0;
            let col_w = 132.0;

            let tab_bar_h = 28.0;
            let grid_h = (ui.available_height() - tab_bar_h - 6.0).max(160.0);

            egui::ScrollArea::both()
                .id_salt("hl_main_sheet_scroll")
                .max_height(grid_h)
                .show(ui, |ui| {
                    let n_cols = self.wb.sheets[s_idx].columns.len();
                    let display_cols = n_cols.max(8);
                    egui::Grid::new("hl_spreadsheet_grid")
                        .spacing([0.0, 0.0])
                        .show(ui, |ui| {
                            // Row 0: Excel Column Letters (A, B, C..J)
                            header_cell(ui, "", 32.0);
                            header_cell(ui, "A", lbl_w);
                            header_cell(ui, "B", unit_w);
                            for i in 0..display_cols {
                                header_cell(ui, &col_letter(i + 2), col_w);
                            }
                            ui.end_row();

                            // Row 1: Layer Type | SET
                            row_num_cell(ui, 1, 32.0);
                            section_cell(ui, "Layer Type", lbl_w, false);
                            section_cell(ui, "", unit_w, true);
                            for i in 0..display_cols {
                                let is_sel = i < n_cols && self.selected_col == i;
                                let resp = text_cell(
                                    ui,
                                    if is_sel { "SET" } else { "" },
                                    col_w,
                                    if is_sel {
                                        EXCEL_YELLOW_BG
                                    } else {
                                        EXCEL_READONLY_BG
                                    },
                                    red_fix,
                                    false,
                                    true,
                                    is_sel,
                                );
                                if i < n_cols && resp.clicked() {
                                    self.selected_col = i;
                                }
                            }
                            ui.end_row();

                            // Row 2: Layer Number (1..8)
                            row_num_cell(ui, 2, 32.0);
                            text_cell(
                                ui,
                                "Layer Number",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(ui, "", unit_w, EXCEL_READONLY_BG, dark, true, false, false);
                            for i in 0..display_cols {
                                let selected = i < n_cols && self.selected_col == i;
                                let resp = text_cell(
                                    ui,
                                    &format!("{}", i + 1),
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    true,
                                    selected,
                                );
                                if i < n_cols && resp.clicked() {
                                    self.selected_col = i;
                                }
                            }
                            ui.end_row();

                            // Row 3: Layer Type
                            row_num_cell(ui, 3, 32.0);
                            text_cell(
                                ui,
                                "Layer Type",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(ui, "", unit_w, EXCEL_READONLY_BG, dark, true, false, false);
                            for i in 0..n_cols {
                                let is_sel = self.selected_col == i;
                                widget_cell(ui, col_w, EXCEL_READONLY_BG, is_sel, |ui| {
                                    let ct = &mut self.wb.sheets[s_idx].columns[i].column_type;
                                    egui::ComboBox::from_id_salt(format!("ct_{s_idx}_{i}"))
                                        .width(col_w - 14.0)
                                        .selected_text(ct.label())
                                        .show_ui(ui, |ui| {
                                            for opt in [
                                                ColumnType::Surface,
                                                ColumnType::SolidLayer,
                                                ColumnType::Enclosure,
                                                ColumnType::LayerContact,
                                            ] {
                                                if ui
                                                    .selectable_value(ct, opt, opt.label())
                                                    .changed()
                                                {
                                                    self.selected_col = i;
                                                    changed = true;
                                                }
                                            }
                                        });
                                });
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 4: Layer Material
                            row_num_cell(ui, 4, 32.0);
                            text_cell(
                                ui,
                                "Layer Material",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(ui, "", unit_w, EXCEL_READONLY_BG, dark, true, false, false);
                            for i in 0..n_cols {
                                let is_sel = self.selected_col == i;
                                widget_cell(ui, col_w, egui::Color32::WHITE, is_sel, |ui| {
                                    let r = ui.text_edit_singleline(
                                        &mut self.wb.sheets[s_idx].columns[i].layer_material,
                                    );
                                    if r.clicked() {
                                        self.selected_col = i;
                                    }
                                    changed |= r.changed();
                                });
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 5: Surface Material
                            row_num_cell(ui, 5, 32.0);
                            text_cell(
                                ui,
                                "Surface Material",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(ui, "", unit_w, EXCEL_READONLY_BG, dark, true, false, false);
                            for i in 0..n_cols {
                                let is_sel = self.selected_col == i;
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if matches!(
                                    col.column_type,
                                    ColumnType::Surface | ColumnType::Enclosure
                                ) {
                                    widget_cell(ui, col_w, egui::Color32::WHITE, is_sel, |ui| {
                                        changed |= ui
                                            .text_edit_singleline(&mut col.surface_material)
                                            .changed();
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 7: Layer Specifications
                            row_num_cell(ui, 7, 32.0);
                            section_cell(ui, "Layer Specifications", lbl_w, false);
                            section_cell(ui, "", unit_w, true);
                            for _ in 0..display_cols {
                                section_cell(ui, "", col_w, true);
                            }
                            ui.end_row();

                            // Row 8: Temperature (left edge)
                            row_num_cell(ui, 8, 32.0);
                            text_cell(
                                ui,
                                "Temperature (left edge)",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            widget_cell(ui, unit_w, EXCEL_READONLY_BG, false, |ui| {
                                egui::ComboBox::from_id_salt("hl_t_unit_inline")
                                    .width(unit_w - 10.0)
                                    .selected_text(self.wb.temp_unit.label())
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_value(
                                                &mut self.wb.temp_unit,
                                                TemperatureUnit::Celsius,
                                                "°C",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if ui
                                            .selectable_value(
                                                &mut self.wb.temp_unit,
                                                TemperatureUnit::Kelvin,
                                                "K",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    });
                            });
                            for i in 0..n_cols {
                                let calc_t = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.t_left_k);
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                let bg = if col.t_left_fixed {
                                    egui::Color32::WHITE
                                } else {
                                    EXCEL_READONLY_BG
                                };
                                widget_cell(ui, col_w, bg, false, |ui| {
                                    let mut t_disp = t_unit.from_kelvin(if col.t_left_fixed {
                                        col.t_left_k
                                    } else {
                                        calc_t.unwrap_or(col.t_left_k)
                                    });
                                    let resp = ui.add_enabled(
                                        col.t_left_fixed,
                                        egui::DragValue::new(&mut t_disp)
                                            .speed(1.0)
                                            .fixed_decimals(2),
                                    );
                                    if resp.changed() {
                                        col.t_left_k = t_unit.to_kelvin(t_disp);
                                        changed = true;
                                    }
                                    let fix_lbl = if col.t_left_fixed {
                                        egui::RichText::new("FIX").color(red_fix).small()
                                    } else {
                                        egui::RichText::new("auto").small()
                                    };
                                    if ui.button(fix_lbl).clicked() {
                                        col.t_left_fixed = !col.t_left_fixed;
                                        changed = true;
                                    }
                                });
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 9: Temperature (right edge)
                            row_num_cell(ui, 9, 32.0);
                            text_cell(
                                ui,
                                "Temperature (right edge)",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(
                                ui,
                                t_unit.label(),
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let calc_t = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.t_right_k);
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                let bg = if col.t_right_fixed {
                                    egui::Color32::WHITE
                                } else {
                                    EXCEL_READONLY_BG
                                };
                                widget_cell(ui, col_w, bg, false, |ui| {
                                    let mut t_disp = t_unit.from_kelvin(if col.t_right_fixed {
                                        col.t_right_k
                                    } else {
                                        calc_t.unwrap_or(col.t_right_k)
                                    });
                                    let resp = ui.add_enabled(
                                        col.t_right_fixed,
                                        egui::DragValue::new(&mut t_disp)
                                            .speed(1.0)
                                            .fixed_decimals(2),
                                    );
                                    if resp.changed() {
                                        col.t_right_k = t_unit.to_kelvin(t_disp);
                                        changed = true;
                                    }
                                    let fix_lbl = if col.t_right_fixed {
                                        egui::RichText::new("FIX").color(red_fix).small()
                                    } else {
                                        egui::RichText::new("auto").small()
                                    };
                                    if ui.button(fix_lbl).clicked() {
                                        col.t_right_fixed = !col.t_right_fixed;
                                        changed = true;
                                    }
                                });
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 10: Thickness
                            row_num_cell(ui, 10, 32.0);
                            text_cell(
                                ui,
                                "Thickness",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            widget_cell(ui, unit_w, EXCEL_READONLY_BG, false, |ui| {
                                egui::ComboBox::from_id_salt("hl_l_unit_inline")
                                    .width(unit_w - 10.0)
                                    .selected_text(self.wb.length_unit.label())
                                    .show_ui(ui, |ui| {
                                        for lu in [
                                            LengthUnit::Meter,
                                            LengthUnit::Centimeter,
                                            LengthUnit::Millimeter,
                                        ] {
                                            if ui
                                                .selectable_value(
                                                    &mut self.wb.length_unit,
                                                    lu,
                                                    lu.label(),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        }
                                    });
                            });
                            for i in 0..n_cols {
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if matches!(
                                    col.column_type,
                                    ColumnType::SolidLayer | ColumnType::Enclosure
                                ) {
                                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                        let mut d_disp =
                                            col.thickness_m * l_unit.from_meters_factor();
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut d_disp)
                                                    .speed(0.002)
                                                    .range(0.0001..=1000.0)
                                                    .fixed_decimals(3),
                                            )
                                            .changed()
                                        {
                                            col.thickness_m = d_disp * l_unit.to_meters_factor();
                                            changed = true;
                                        }
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 11: Calculation Grid
                            row_num_cell(ui, 11, 32.0);
                            text_cell(
                                ui,
                                "Calculation Grid",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            text_cell(ui, "", unit_w, EXCEL_READONLY_BG, dark, false, false, false);
                            for i in 0..n_cols {
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if col.column_type == ColumnType::SolidLayer {
                                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut col.calculation_grid)
                                                    .range(1..=100),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 12: Average conductivity (k)
                            row_num_cell(ui, 12, 32.0);
                            text_cell(
                                ui,
                                "Average conductivity (k)",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                "W/(m·K)",
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let calc_k = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.k_avg_w_m_k);
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if col.column_type == ColumnType::SolidLayer {
                                    let bg = if col.k_fixed {
                                        egui::Color32::WHITE
                                    } else {
                                        EXCEL_READONLY_BG
                                    };
                                    widget_cell(ui, col_w, bg, false, |ui| {
                                        if col.k_fixed {
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut col.k_w_m_k)
                                                        .speed(0.05)
                                                        .range(1e-4..=1000.0)
                                                        .fixed_decimals(3),
                                                )
                                                .changed();
                                        } else {
                                            ui.monospace(format!(
                                                "{:.3}",
                                                calc_k.unwrap_or(col.k_w_m_k)
                                            ));
                                        }
                                        let lbl = if col.k_fixed {
                                            egui::RichText::new("FIX").color(red_fix).small()
                                        } else {
                                            egui::RichText::new("DB").small()
                                        };
                                        if ui.button(lbl).clicked() {
                                            col.k_fixed = !col.k_fixed;
                                            changed = true;
                                        }
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 13: Convection coefficient (hc)
                            row_num_cell(ui, 13, 32.0);
                            text_cell(
                                ui,
                                "Convection coefficient (hc)",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                "W/(m²K)",
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let calc_hc = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.hc_w_m2_k);
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if matches!(
                                    col.column_type,
                                    ColumnType::Surface | ColumnType::Enclosure
                                ) {
                                    let bg = if col.hc_fixed {
                                        egui::Color32::WHITE
                                    } else {
                                        EXCEL_READONLY_BG
                                    };
                                    widget_cell(ui, col_w, bg, false, |ui| {
                                        if col.hc_fixed {
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut col.hc_w_m2_k)
                                                        .speed(0.5)
                                                        .range(0.0..=50000.0)
                                                        .fixed_decimals(3),
                                                )
                                                .changed();
                                        } else {
                                            ui.monospace(format!(
                                                "{:.3}",
                                                calc_hc.unwrap_or(col.hc_w_m2_k)
                                            ));
                                        }
                                        let lbl = if col.hc_fixed {
                                            egui::RichText::new("FIX").color(red_fix).small()
                                        } else {
                                            egui::RichText::new("auto").small()
                                        };
                                        if ui.button(lbl).clicked() {
                                            col.hc_fixed = !col.hc_fixed;
                                            changed = true;
                                        }
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 14: Radiation coefficient (hr)
                            row_num_cell(ui, 14, 32.0);
                            text_cell(
                                ui,
                                "Radiation coefficient (hr)",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                "W/(m²K)",
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let calc_hr = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.hr_w_m2_k);
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if matches!(
                                    col.column_type,
                                    ColumnType::Surface | ColumnType::Enclosure
                                ) {
                                    let bg = if col.hr_fixed {
                                        egui::Color32::WHITE
                                    } else {
                                        EXCEL_READONLY_BG
                                    };
                                    widget_cell(ui, col_w, bg, false, |ui| {
                                        if col.hr_fixed {
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut col.hr_w_m2_k)
                                                        .speed(0.1)
                                                        .range(0.0..=1000.0)
                                                        .fixed_decimals(3),
                                                )
                                                .changed();
                                        } else {
                                            ui.monospace(format!(
                                                "{:.3}",
                                                calc_hr.unwrap_or(col.hr_w_m2_k)
                                            ));
                                        }
                                        let lbl = if col.hr_fixed {
                                            egui::RichText::new("FIX").color(red_fix).small()
                                        } else {
                                            egui::RichText::new("auto").small()
                                        };
                                        if ui.button(lbl).clicked() {
                                            col.hr_fixed = !col.hr_fixed;
                                            changed = true;
                                        }
                                    });
                                } else {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 15: Thermal Resistance
                            row_num_cell(ui, 15, 32.0);
                            text_cell(
                                ui,
                                "Thermal Resistance",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                "°C/kW",
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let col = &mut self.wb.sheets[s_idx].columns[i];
                                if col.column_type == ColumnType::LayerContact {
                                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                        let mut r_ckw = col.contact_resistance_k_w * 1000.0;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut r_ckw)
                                                    .speed(0.1)
                                                    .range(1e-4..=1e6)
                                                    .fixed_decimals(3),
                                            )
                                            .changed()
                                        {
                                            col.contact_resistance_k_w = r_ckw / 1000.0;
                                            changed = true;
                                        }
                                        ui.colored_label(red_fix, "FIX");
                                    });
                                } else {
                                    let r = sheet_res
                                        .as_ref()
                                        .and_then(|sr| sr.columns.get(i))
                                        .map(|c| c.thermal_resistance_k_w * 1000.0)
                                        .unwrap_or(0.0);
                                    text_cell(
                                        ui,
                                        &format!("{r:.3}"),
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 17: Results
                            row_num_cell(ui, 17, 32.0);
                            section_cell(ui, "Results", lbl_w, false);
                            section_cell(ui, "", unit_w, true);
                            for _ in 0..display_cols {
                                section_cell(ui, "", col_w, true);
                            }
                            ui.end_row();

                            // Row 18: Heat Flow
                            row_num_cell(ui, 18, 32.0);
                            text_cell(
                                ui,
                                "Heat Flow",
                                lbl_w,
                                EXCEL_SECTION_BG,
                                dark,
                                false,
                                true,
                                false,
                            );
                            widget_cell(ui, unit_w, EXCEL_SECTION_BG, false, |ui| {
                                egui::ComboBox::from_id_salt("hl_q_unit_inline")
                                    .width(unit_w - 10.0)
                                    .selected_text(self.wb.heat_flow_unit.label())
                                    .show_ui(ui, |ui| {
                                        for u in [
                                            HeatFlowUnit::Watt,
                                            HeatFlowUnit::Kilowatt,
                                            HeatFlowUnit::Megawatt,
                                            HeatFlowUnit::KcalPerHour,
                                            HeatFlowUnit::BtuPerHour,
                                        ] {
                                            if ui
                                                .selectable_value(
                                                    &mut self.wb.heat_flow_unit,
                                                    u,
                                                    u.label(),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        }
                                    });
                            });
                            for i in 0..n_cols {
                                let q = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.heat_flow_w * q_unit.from_watts_factor())
                                    .unwrap_or(0.0);
                                text_cell(
                                    ui,
                                    &format!("{q:.3}"),
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    true,
                                    false,
                                );
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 19: Heat Flux
                            row_num_cell(ui, 19, 32.0);
                            text_cell(
                                ui,
                                "Heat Flux",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                q_unit.flux_label(),
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let qf = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.heat_flux_w_m2 * q_unit.from_watts_factor())
                                    .unwrap_or(0.0);
                                text_cell(
                                    ui,
                                    &format!("{qf:.3}"),
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 20: Surface Area
                            row_num_cell(ui, 20, 32.0);
                            text_cell(
                                ui,
                                "Surface Area",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                "m²",
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let a = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.surface_area_m2)
                                    .unwrap_or(0.0);
                                text_cell(
                                    ui,
                                    &format!("{a:.3}"),
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Row 21: Cumulative Thickness
                            row_num_cell(ui, 21, 32.0);
                            text_cell(
                                ui,
                                "Cumulative Thickness",
                                lbl_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            text_cell(
                                ui,
                                l_unit.label(),
                                unit_w,
                                EXCEL_READONLY_BG,
                                dark,
                                false,
                                false,
                                false,
                            );
                            for i in 0..n_cols {
                                let ct = sheet_res
                                    .as_ref()
                                    .and_then(|sr| sr.columns.get(i))
                                    .map(|c| c.cumulative_thickness_m * l_unit.from_meters_factor())
                                    .unwrap_or(0.0);
                                text_cell(
                                    ui,
                                    &format!("{ct:.3}"),
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            for _ in n_cols..display_cols {
                                text_cell(
                                    ui,
                                    "",
                                    col_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            ui.end_row();

                            // Optional Advanced / Detailed Rows (Rows 38, 50, 61-62, 69, 92-93, 96+, Fig. 4 item 4 & 8)
                            if self.show_advanced_rows {
                                // Row 38: Fluid Speed
                                row_num_cell(ui, 38, 32.0);
                                text_cell(
                                    ui,
                                    "Speed (forced convection)",
                                    lbl_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    "m/s",
                                    unit_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                for i in 0..n_cols {
                                    let col = &mut self.wb.sheets[s_idx].columns[i];
                                    if col.column_type == ColumnType::Surface {
                                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut col.fluid_speed_m_s)
                                                        .speed(0.1)
                                                        .range(0.0..=500.0)
                                                        .fixed_decimals(2),
                                                )
                                                .changed()
                                            {
                                                col.convection_regime = if col.fluid_speed_m_s > 0.0
                                                {
                                                    ConvectionRegime::Forced
                                                } else {
                                                    ConvectionRegime::Free
                                                };
                                                changed = true;
                                            }
                                        });
                                    } else {
                                        text_cell(
                                            ui,
                                            "",
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                }
                                for _ in n_cols..display_cols {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                ui.end_row();

                                // Row 50: Surface Emissivity
                                row_num_cell(ui, 50, 32.0);
                                text_cell(
                                    ui,
                                    "Surface Emissivity (εs)",
                                    lbl_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    "none",
                                    unit_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                for i in 0..n_cols {
                                    let col = &mut self.wb.sheets[s_idx].columns[i];
                                    if matches!(
                                        col.column_type,
                                        ColumnType::Surface | ColumnType::Enclosure
                                    ) {
                                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut col.emissivity)
                                                        .speed(0.01)
                                                        .range(0.0..=1.0)
                                                        .fixed_decimals(3),
                                                )
                                                .changed();
                                        });
                                    } else {
                                        text_cell(
                                            ui,
                                            "",
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                }
                                for _ in n_cols..display_cols {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                ui.end_row();

                                // Row 61: Gas Emissivity
                                row_num_cell(ui, 61, 32.0);
                                text_cell(
                                    ui,
                                    "Gas Emissivity (εg)",
                                    lbl_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    "none",
                                    unit_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                for i in 0..n_cols {
                                    let col = &mut self.wb.sheets[s_idx].columns[i];
                                    let txt = col
                                        .gas_emissivity
                                        .map(|v| format!("{v:.4}"))
                                        .unwrap_or_default();
                                    text_cell(
                                        ui,
                                        &txt,
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        red_fix,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                for _ in n_cols..display_cols {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                ui.end_row();

                                // Row 62: Gas Absorptivity
                                row_num_cell(ui, 62, 32.0);
                                text_cell(
                                    ui,
                                    "Gas Absorptivity (αg)",
                                    lbl_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    "none",
                                    unit_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                for i in 0..n_cols {
                                    let col = &mut self.wb.sheets[s_idx].columns[i];
                                    let txt = col
                                        .gas_absorptivity
                                        .map(|v| format!("{v:.4}"))
                                        .unwrap_or_default();
                                    text_cell(
                                        ui,
                                        &txt,
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        red_fix,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                for _ in n_cols..display_cols {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                ui.end_row();

                                // Row 69: Particle Cloud Emissivity
                                row_num_cell(ui, 69, 32.0);
                                text_cell(
                                    ui,
                                    "Particle Cloud Emissivity (εc)",
                                    lbl_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    "none",
                                    unit_w,
                                    EXCEL_READONLY_BG,
                                    dark,
                                    false,
                                    false,
                                    false,
                                );
                                for i in 0..n_cols {
                                    let col = &mut self.wb.sheets[s_idx].columns[i];
                                    let txt = col
                                        .particle_cloud_emissivity
                                        .map(|v| format!("{v:.4}"))
                                        .unwrap_or_default();
                                    text_cell(
                                        ui,
                                        &txt,
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        red_fix,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                for _ in n_cols..display_cols {
                                    text_cell(
                                        ui,
                                        "",
                                        col_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                ui.end_row();

                                // Rows 92-93: Min / Max Temperature (K)
                                for (r_num, r_lbl, is_min) in [
                                    (92, "Min Temperature (Tmin)", true),
                                    (93, "Max Temperature (Tmax)", false),
                                ] {
                                    row_num_cell(ui, r_num, 32.0);
                                    text_cell(
                                        ui,
                                        r_lbl,
                                        lbl_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        false,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        "K",
                                        unit_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        false,
                                        false,
                                        false,
                                    );
                                    for i in 0..n_cols {
                                        let col = &self.wb.sheets[s_idx].columns[i];
                                        let db_range = col
                                            .solid_data
                                            .as_ref()
                                            .and_then(|s| s.t_range_k())
                                            .or_else(|| {
                                                col.fluid_data.as_ref().and_then(|f| {
                                                    Some((
                                                        f.points.first()?.t_k,
                                                        f.points.last()?.t_k,
                                                    ))
                                                })
                                            });
                                        let val = if is_min {
                                            col.t_min_k.or_else(|| db_range.map(|(lo, _)| lo))
                                        } else {
                                            col.t_max_k.or_else(|| db_range.map(|(_, hi)| hi))
                                        };
                                        let s_val = val
                                            .map(|v| format!("{v:.2}"))
                                            .unwrap_or_else(|| "N/A".into());
                                        text_cell(
                                            ui,
                                            &s_val,
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                    for _ in n_cols..display_cols {
                                        text_cell(
                                            ui,
                                            "",
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                    ui.end_row();
                                }

                                // Rows 96..101: Conduction / Convection Data (Fig. 4 item 8)
                                for pt_idx in 0..4 {
                                    let row_no = 96 + pt_idx * 2;
                                    row_num_cell(ui, row_no, 32.0);
                                    let lbl = if pt_idx == 0 {
                                        "Conduction / Convection Data"
                                    } else {
                                        ""
                                    };
                                    text_cell(
                                        ui,
                                        lbl,
                                        lbl_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        false,
                                        pt_idx == 0,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        if pt_idx % 2 == 0 { "T (K)" } else { "k / ρ" },
                                        unit_w,
                                        EXCEL_READONLY_BG,
                                        dark,
                                        false,
                                        false,
                                        false,
                                    );
                                    for i in 0..n_cols {
                                        let col = &self.wb.sheets[s_idx].columns[i];
                                        let txt = if let Some(sd) = &col.solid_data {
                                            sd.k_table
                                                .get(pt_idx)
                                                .map(|(t, k)| format!("{t:.1} K: {k:.3}"))
                                                .unwrap_or_default()
                                        } else if let Some(fd) = &col.fluid_data {
                                            fd.points
                                                .get(pt_idx)
                                                .map(|p| format!("{:.1} K: k={:.4}", p.t_k, p.k_w_m_k))
                                                .unwrap_or_default()
                                        } else {
                                            String::new()
                                        };
                                        text_cell(
                                            ui,
                                            &txt,
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                    for _ in n_cols..display_cols {
                                        text_cell(
                                            ui,
                                            "",
                                            col_w,
                                            EXCEL_READONLY_BG,
                                            dark,
                                            true,
                                            false,
                                            false,
                                        );
                                    }
                                    ui.end_row();
                                }
                            }
                        });
                });

            ui.separator();
            // Excel-style Bottom Sheet Tabs (Figs. 1, 2, 4)
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(232, 236, 242))
                .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for idx in 0..self.wb.sheets.len() {
                            let active = idx == self.wb.active_sheet;
                            let fill = if active {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(220, 226, 235)
                            };
                            let text_col = if active {
                                egui::Color32::from_rgb(15, 23, 42)
                            } else {
                                egui::Color32::from_rgb(71, 85, 105)
                            };
                            let stroke = if active {
                                egui::Stroke::new(1.2, egui::Color32::from_rgb(29, 78, 216))
                            } else {
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 198, 210))
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(&self.wb.sheets[idx].name)
                                            .size(11.5)
                                            .strong()
                                            .color(text_col),
                                    )
                                    .fill(fill)
                                    .stroke(stroke)
                                    .rounding(3.0),
                                )
                                .clicked()
                            {
                                self.wb.active_sheet = idx;
                                self.recalculate();
                            }
                        }
                        if ui.small_button(crate::i18n::text("review.160")).clicked() {
                            let n = self.wb.sheets.len() + 1;
                            let mut new_sheet = self.wb.sheets[0].clone();
                            new_sheet.name = format!("Sheet {n}");
                            self.wb.sheets.push(new_sheet);
                            self.wb.active_sheet = self.wb.sheets.len() - 1;
                            self.recalculate();
                        }
                    });
                });

            if changed {
                self.recalculate();
            }
        });

        self.render_modals(ctx);
    }

    fn refresh_diagram(&mut self) {
        match self.diagram_kind {
            DiagramKind::TemperatureProfile => {
                if let Ok((tot, ser)) =
                    SoryntelApplication::generate_heat_loss_temperature_profile(&self.wb)
                {
                    self.profile_total_w = tot;
                    self.profile_series = ser;
                }
            }
            DiagramKind::ParameterSweep => {
                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                if let Ok(pts) = SoryntelApplication::generate_heat_loss_parameter_sweep(
                    &self.wb,
                    s_idx,
                    self.sweep_param,
                    self.sweep_min,
                    self.sweep_max,
                    self.sweep_steps,
                ) {
                    self.sweep_points = pts;
                }
            }
            DiagramKind::OptimumInsulation => {
                if let Ok((pts, best)) = SoryntelApplication::generate_heat_loss_optimum_insulation(
                    &self.wb,
                    &self.opt_spec,
                ) {
                    self.opt_points = pts;
                    self.opt_best = Some(best);
                }
            }
        }
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        let red_fix = egui::Color32::from_rgb(185, 28, 28);
        let yellow_bg = egui::Color32::from_rgb(254, 249, 195);

        // 1. Conduction Database Window (Fig. 5 — HeatCond.XLS)
        if self.show_cond_picker {
            let mut open = true;
            egui::Window::new("📦 Conduction C:\\HSC8\\Databases\\HeatCond.XLS (Fig. 5)")
                .open(&mut open)
                .default_size([720.0, 440.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Active Column: {}", self.selected_col + 1));
                        ui.separator();
                        ui.label(crate::i18n::text("common.025"));
                        ui.text_edit_singleline(&mut self.cond_search);
                        if ui.button(crate::i18n::text("review.161")).clicked() {
                            self.solids.push(SolidConductivityData {
                                name: format!("Custom Refractory {}", self.solids.len() + 1),
                                category: "Custom".into(),
                                density_kg_m3: Some(2200.0),
                                k_table: vec![(300.0, 1.5), (800.0, 1.8), (1300.0, 2.1)],
                                reference: "User".into(),
                            });
                            self.cond_selected_idx = self.solids.len() - 1;
                        }
                    });
                    ui.separator();

                    let q = self.cond_search.trim().to_lowercase();
                    egui::ScrollArea::both().max_height(310.0).show(ui, |ui| {
                        egui::Grid::new("heatcond_xls_grid")
                            .striped(true)
                            .spacing([8.0, 3.0])
                            .show(ui, |ui| {
                                ui.strong("Thermal Conduction Database");
                                ui.strong("Property");
                                ui.strong("Units");
                                for t_idx in 1..=6 {
                                    ui.strong(format!("T{t_idx}"));
                                }
                                ui.strong("Action");
                                ui.end_row();

                                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                for (idx, solid) in self.solids.iter().enumerate() {
                                    if !q.is_empty()
                                        && !solid.name.to_lowercase().contains(&q)
                                        && !solid.category.to_lowercase().contains(&q)
                                    {
                                        continue;
                                    }
                                    let is_sel = self.cond_selected_idx == idx;
                                    // Row A: Temperature (K)
                                    if ui.selectable_label(is_sel, &solid.name).clicked() {
                                        self.cond_selected_idx = idx;
                                    }
                                    ui.label(crate::i18n::text("common.005"));
                                    ui.label("K");
                                    for k_i in 0..6 {
                                        let txt = solid
                                            .k_table
                                            .get(k_i)
                                            .map(|(t, _)| format!("{t:.1}"))
                                            .unwrap_or_default();
                                        ui.monospace(txt);
                                    }
                                    if ui.small_button(crate::i18n::text("common.057")).clicked() {
                                        self.cond_selected_idx = idx;
                                        if let Some(col) =
                                            self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                        {
                                            col.layer_material = solid.name.clone();
                                            col.solid_data = Some(solid.clone());
                                            col.k_fixed = false;
                                        }
                                    }
                                    ui.end_row();

                                    // Row B: Thermal conductivity (W/m K)
                                    ui.label(
                                        egui::RichText::new(format!("({})", solid.category))
                                            .small()
                                            .weak(),
                                    );
                                    ui.label(crate::i18n::text("review.162"));
                                    ui.label("W/m K");
                                    for k_i in 0..6 {
                                        let txt = solid
                                            .k_table
                                            .get(k_i)
                                            .map(|(_, k)| format!("{k:.2}"))
                                            .unwrap_or_default();
                                        ui.monospace(txt);
                                    }
                                    ui.label("");
                                    ui.end_row();
                                }
                            });
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::text("review.163")).clicked() {
                            if let Some(solid) = self.solids.get(self.cond_selected_idx).cloned() {
                                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                if let Some(col) =
                                    self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                {
                                    col.layer_material = solid.name.clone();
                                    col.solid_data = Some(solid);
                                    col.k_fixed = false;
                                }
                                self.recalculate();
                            }
                        }
                    });
                });
            if !open {
                self.show_cond_picker = false;
                self.recalculate();
            }
        }

        // 2. Convection Database & Calculator Window (Figs. 6-7 — Table & Function tabs)
        if self.show_conv_calc {
            let mut open = true;
            egui::Window::new("💨 Convection Database & Calculator (Figs. 6-7)")
                .open(&mut open)
                .default_size([760.0, 520.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.conv_tab,
                            ConvectionDbTab::Table,
                            "📋 Table (Fig. 6)",
                        );
                        ui.selectable_value(
                            &mut self.conv_tab,
                            ConvectionDbTab::Function,
                            "ƒ(v) Function (Fig. 7)",
                        );
                        ui.separator();
                        ui.label(crate::i18n::text("common.025"));
                        ui.text_edit_singleline(&mut self.conv_find_query);
                    });
                    ui.separator();

                    match self.conv_tab {
                        ConvectionDbTab::Table => {
                            egui::ScrollArea::both().max_height(195.0).show(ui, |ui| {
                                egui::Grid::new("conv_table_sheet_grid")
                                    .striped(true)
                                    .spacing([8.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.strong("Fluid");
                                        ui.strong("Property");
                                        ui.strong("Units");
                                        for i in 1..=6 {
                                            ui.strong(format!("T{i}"));
                                        }
                                        ui.end_row();

                                        for (f_idx, fl) in self.fluids.iter().enumerate() {
                                            if !self.conv_find_query.is_empty()
                                                && !fl
                                                    .name
                                                    .to_lowercase()
                                                    .contains(&self.conv_find_query.to_lowercase())
                                            {
                                                continue;
                                            }
                                            let is_sel = self.conv_selected_fluid == f_idx;
                                            if ui.selectable_label(is_sel, &fl.name).clicked() {
                                                self.conv_selected_fluid = f_idx;
                                            }
                                            ui.label(crate::i18n::text("common.005"));
                                            ui.label("K");
                                            for p_i in 0..6 {
                                                ui.monospace(
                                                    fl.points
                                                        .get(p_i)
                                                        .map(|p| format!("{:.1}", p.t_k))
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            ui.end_row();

                                            ui.label(egui::RichText::new(&fl.reference).small().weak());
                                            ui.label(crate::i18n::text("common.128"));
                                            ui.label("kg/m³");
                                            for p_i in 0..6 {
                                                ui.monospace(
                                                    fl.points
                                                        .get(p_i)
                                                        .map(|p| format!("{:.4}", p.density_kg_m3))
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            ui.end_row();

                                            ui.label("");
                                            ui.label(crate::i18n::text("review.164"));
                                            ui.label("W/mK");
                                            for p_i in 0..6 {
                                                ui.monospace(
                                                    fl.points
                                                        .get(p_i)
                                                        .map(|p| format!("{:.2}", p.k_w_m_k * 1e3))
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            ui.end_row();

                                            ui.label("");
                                            ui.label(crate::i18n::text("review.165"));
                                            ui.label(crate::i18n::text("review.166"));
                                            for p_i in 0..6 {
                                                ui.monospace(
                                                    fl.points
                                                        .get(p_i)
                                                        .map(|p| format!("{:.3}", p.prandtl()))
                                                        .unwrap_or_default(),
                                                );
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                        ConvectionDbTab::Function => {
                            // Fig. 7: Function hc(v) = A v^a + B v^b + C v^c + ...
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("review.167"));
                                ui.add(
                                    egui::DragValue::new(&mut self.conv_test_speed)
                                        .speed(0.05)
                                        .range(0.0..=100.0)
                                        .fixed_decimals(3),
                                );
                            });
                            egui::ScrollArea::both().max_height(175.0).show(ui, |ui| {
                                egui::Grid::new("conv_function_sheet_grid")
                                    .striped(true)
                                    .spacing([10.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.strong("Thermal Convection Function");
                                        ui.strong("hc(v) [W/(m²K)]");
                                        ui.strong("v [m/s]");
                                        ui.strong("A / a");
                                        ui.strong("B / b");
                                        ui.strong("C / c");
                                        ui.strong("Action");
                                        ui.end_row();

                                        let s_idx =
                                            self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                        for (fn_i, fp) in self.conv_functions.iter().enumerate() {
                                            let hc_val =
                                                SoryntelApplication::evaluate_heat_loss_hc_function(
                                                    &fp.terms,
                                                    self.conv_test_speed,
                                                );
                                            let is_sel = self.conv_selected_func == fn_i;
                                            if ui
                                                .selectable_label(
                                                    is_sel,
                                                    format!("{} {}", fp.name, fp.subtitle),
                                                )
                                                .clicked()
                                            {
                                                self.conv_selected_func = fn_i;
                                            }
                                            ui.strong(format!("{hc_val:.1}"));
                                            ui.monospace(format!("{:.2}", self.conv_test_speed));
                                            for t_k in 0..3 {
                                                let txt = fp
                                                    .terms
                                                    .get(t_k)
                                                    .map(|t| {
                                                        format!("{:.0} (v^{:.0})", t.coeff, t.exponent)
                                                    })
                                                    .unwrap_or_else(|| "0".into());
                                                ui.monospace(txt);
                                            }
                                            if ui.small_button(crate::i18n::text("review.168")).clicked() {
                                                self.conv_selected_func = fn_i;
                                                if let Some(col) = self.wb.sheets[s_idx]
                                                    .columns
                                                    .get_mut(self.selected_col)
                                                {
                                                    col.layer_material = fp.name.clone();
                                                    col.custom_hc_terms = fp.terms.clone();
                                                    col.hc_w_m2_k = hc_val;
                                                    col.hc_fixed = false;
                                                }
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                    }

                    ui.separator();
                    let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                    if let Some(col) = self.wb.sheets[s_idx].columns.get_mut(self.selected_col) {
                        ui.columns(2, |cols| {
                            cols[0].group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("review.169"));
                                    egui::ComboBox::from_id_salt("conv_fluid_sel")
                                        .selected_text(&self.fluids[self.conv_selected_fluid].name)
                                        .show_ui(ui, |ui| {
                                            for (i, fl) in self.fluids.iter().enumerate() {
                                                ui.selectable_value(
                                                    &mut self.conv_selected_fluid,
                                                    i,
                                                    &fl.name,
                                                );
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.radio_value(
                                        &mut col.convection_regime,
                                        ConvectionRegime::Free,
                                        "Free Convection",
                                    );
                                    ui.radio_value(
                                        &mut col.convection_regime,
                                        ConvectionRegime::Forced,
                                        "Forced Convection",
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("review.170"));
                                    egui::ComboBox::from_id_salt("conv_geom_sel")
                                        .selected_text(col.convection_geometry.label())
                                        .show_ui(ui, |ui| {
                                            for g in [
                                                ConvectionGeometry::VerticalWall,
                                                ConvectionGeometry::HorizontalPlateHotUp,
                                                ConvectionGeometry::HorizontalPlateHotDown,
                                                ConvectionGeometry::HorizontalCylinder,
                                                ConvectionGeometry::VerticalCylinder,
                                                ConvectionGeometry::Sphere,
                                                ConvectionGeometry::VerticalEnclosure,
                                                ConvectionGeometry::HorizontalEnclosure,
                                                ConvectionGeometry::ConcentricCylinders,
                                                ConvectionGeometry::ConcentricSpheres,
                                            ] {
                                                ui.selectable_value(
                                                    &mut col.convection_geometry,
                                                    g,
                                                    g.label(),
                                                );
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("review.171"));
                                    ui.add(
                                        egui::DragValue::new(&mut col.fluid_speed_m_s)
                                            .speed(0.1)
                                            .range(0.0..=200.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    if ui.button(crate::i18n::text("common.008")).clicked() {
                                        let fluid = self.fluids[self.conv_selected_fluid].clone();
                                        if let Ok(diag) =
                                            SoryntelApplication::evaluate_heat_loss_convection(
                                                &fluid,
                                                col.convection_regime,
                                                col.convection_geometry,
                                                col.t_left_k,
                                                col.t_right_k,
                                                1.0,
                                                None,
                                                col.fluid_speed_m_s,
                                                None,
                                            )
                                        {
                                            self.conv_diag = Some(diag);
                                        }
                                    }
                                    if ui.button(crate::i18n::text("review.172")).clicked() {
                                        let fluid = self.fluids[self.conv_selected_fluid].clone();
                                        col.layer_material = fluid.name.clone();
                                        col.fluid_data = Some(fluid);
                                        col.custom_hc_terms.clear();
                                        col.hc_fixed = false;
                                    }
                                    if ui.button(crate::i18n::text("common.026")).clicked() {
                                        let fluid = self.fluids[self.conv_selected_fluid].clone();
                                        if let Ok(diag) =
                                            SoryntelApplication::evaluate_heat_loss_convection(
                                                &fluid,
                                                col.convection_regime,
                                                col.convection_geometry,
                                                col.t_left_k,
                                                col.t_right_k,
                                                1.0,
                                                None,
                                                col.fluid_speed_m_s,
                                                None,
                                            )
                                        {
                                            self.conv_diag = Some(diag);
                                            col.layer_material = fluid.name.clone();
                                            col.hc_w_m2_k = diag.hc_w_m2_k;
                                            col.hc_fixed = true;
                                        }
                                    }
                                });
                            });

                            cols[1].group(|ui| {
                                ui.strong("RESULTS (Fig. 6)");
                                if let Some(d) = self.conv_diag {
                                    egui::Grid::new("conv_results_8row_grid")
                                        .striped(true)
                                        .show(ui, |ui| {
                                            ui.label(crate::i18n::text("review.173"));
                                            ui.label("m");
                                            ui.monospace(format!("{:.3}", d.char_length_m));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.174"));
                                            ui.label("");
                                            ui.monospace(format!("{:.3}", d.aspect_ratio_h_l));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.175"));
                                            ui.label("°C");
                                            ui.monospace(format!("{:.2}", d.t_film_k - 273.15));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.176"));
                                            ui.label("");
                                            ui.monospace(format!("{:.3}", d.nusselt));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.177"));
                                            ui.label("");
                                            ui.monospace(format!("{:.3e}", d.rayleigh));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.178"));
                                            ui.label("");
                                            ui.monospace(format!("{:.3e}", d.grashof));
                                            ui.end_row();
                                            ui.label(crate::i18n::text("review.179"));
                                            ui.label("");
                                            ui.monospace(format!("{:.3e}", d.reynolds));
                                            ui.end_row();
                                            ui.label("8 Re*Pr");
                                            ui.label("");
                                            ui.monospace(format!("{:.3e}", d.peclet_re_pr));
                                            ui.end_row();
                                            ui.strong("9 hc (Convection coeff.)");
                                            ui.strong("W/(m²K)");
                                            ui.strong(format!("{:.3}", d.hc_w_m2_k));
                                            ui.end_row();
                                        });
                                } else {
                                    ui.label("Pulse Calculate para evaluar los números adimensionales.");
                                }
                            });
                        });
                    }
                });
            if !open {
                self.show_conv_calc = false;
                self.recalculate();
            }
        }

        // 3. Surface Radiation Database Window (Fig. 8 — HeatRadi.XLS)
        if self.show_rad_picker {
            let mut open = true;
            egui::Window::new("☀ Surface Radiation C:\\HSC8\\Databases\\HeatRadi.XLS (Fig. 8)")
                .open(&mut open)
                .default_size([620.0, 400.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("(h)=hemispherical emissivity, (n)=normal emissivity");
                        ui.separator();
                        ui.label(crate::i18n::text("common.025"));
                        ui.text_edit_singleline(&mut self.rad_search);
                    });
                    ui.separator();
                    let q = self.rad_search.trim().to_lowercase();
                    egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
                        egui::Grid::new("heatradi_xls_grid")
                            .striped(true)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong("Thermal Radiation Database");
                                ui.strong("Type");
                                ui.strong("Range");
                                ui.strong("Emissivity (ε)");
                                ui.strong("Action");
                                ui.end_row();

                                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                for (idx, em) in self.emissivities.iter().enumerate() {
                                    if !q.is_empty() && !em.name.to_lowercase().contains(&q) {
                                        continue;
                                    }
                                    if ui
                                        .selectable_label(self.rad_selected_idx == idx, &em.name)
                                        .clicked()
                                    {
                                        self.rad_selected_idx = idx;
                                    }
                                    ui.label(&em.measurement_type);
                                    ui.label(&em.temp_range_desc);
                                    ui.monospace(format!("{:.3}", em.emissivity));
                                    if ui.small_button(crate::i18n::text("common.057")).clicked() {
                                        self.rad_selected_idx = idx;
                                        if let Some(col) =
                                            self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                        {
                                            col.surface_material = em.name.clone();
                                            col.emissivity = em.emissivity;
                                            col.hr_fixed = false;
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                });
            if !open {
                self.show_rad_picker = false;
                self.recalculate();
            }
        }

        // 4. Gas Mixture Radiation Calculator (Fig. 9)
        if self.show_gas_rad {
            let mut open = true;
            egui::Window::new("💨 Gas Mixture Radiation Calculator (Fig. 9)")
                .open(&mut open)
                .default_size([760.0, 490.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.columns(2, |cols| {
                        // Top Left: User Input table (Species | vol-% in red font)
                        cols[0].group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(37, 99, 235),
                                    egui::RichText::new(crate::i18n::text("common.058")).strong(),
                                );
                            });
                            let comp = &mut self.gas_input.composition;
                            egui::Grid::new("gas_rad_user_input_grid")
                                .striped(true)
                                .spacing([16.0, 4.0])
                                .show(ui, |ui| {
                                    ui.strong("Species");
                                    ui.strong("vol-%");
                                    ui.end_row();

                                    for (sp_lbl, frac_ref) in [
                                        ("H2O(g)", &mut comp.x_h2o),
                                        ("CO2(g)", &mut comp.x_co2),
                                        ("CO(g)", &mut comp.x_co),
                                        ("NO(g)", &mut comp.x_no),
                                        ("SO2(g)", &mut comp.x_so2),
                                        ("CH4(g)", &mut comp.x_ch4),
                                    ] {
                                        ui.label(sp_lbl);
                                        let mut pct = *frac_ref * 100.0;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut pct)
                                                    .speed(0.2)
                                                    .range(0.0..=100.0)
                                                    .fixed_decimals(3),
                                            )
                                            .changed()
                                        {
                                            *frac_ref = (pct / 100.0).clamp(0.0, 1.0);
                                        }
                                        ui.end_row();
                                    }
                                });
                            ui.add_space(4.0);
                            if ui.small_button("Load OilFurnace Flue Gas Preset").clicked() {
                                self.gas_input.composition = preset_oil_furnace_gas();
                                self.gas_input.t_gas_k = 1200.0 + 273.15;
                                self.gas_input.t_surface_k = 500.0 + 273.15;
                            }
                        });

                        // Top Right: RESULTS table (Nyy-Low | Nyy-Upp | F-Low | F-Upp | DF | SumDF) + Gas/Wall tabs
                        cols[1].group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(37, 99, 235),
                                    egui::RichText::new(crate::i18n::text("review.180")).strong(),
                                );
                            });
                            egui::ScrollArea::vertical().max_height(155.0).show(ui, |ui| {
                                egui::Grid::new("gas_rad_bands_grid")
                                    .striped(true)
                                    .spacing([8.0, 2.0])
                                    .show(ui, |ui| {
                                        ui.strong("Nyy-Low");
                                        ui.strong("Nyy-Upp");
                                        ui.strong("F-Low");
                                        ui.strong("F-Upp");
                                        ui.strong("DF");
                                        ui.strong("SumDF");
                                        ui.end_row();

                                        if let Some((res, _, _)) = &self.gas_result {
                                            let active_bands = match self.gas_band_tab {
                                                GasBandTab::Gas => &res.bands,
                                                GasBandTab::Wall => &res.wall_bands,
                                            };
                                            let mut sum_df = 0.0;
                                            for b in active_bands {
                                                sum_df += b.df;
                                                ui.monospace(format!("{:.0}", b.nyy_low_cm_1));
                                                ui.monospace(format!("{:.0}", b.nyy_upp_cm_1));
                                                ui.monospace(format!("{:.4}", b.f_low));
                                                ui.monospace(format!("{:.4}", b.f_upp));
                                                ui.monospace(format!("{:.4}", b.df));
                                                ui.monospace(format!("{:.4}", sum_df));
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.gas_band_tab, GasBandTab::Gas, "Gas");
                                ui.selectable_value(
                                    &mut self.gas_band_tab,
                                    GasBandTab::Wall,
                                    "Wall",
                                );
                            });
                        });
                    });

                    ui.separator();
                    ui.columns(2, |cols| {
                        // Bottom Left: Gas Properties & Surface Properties
                        cols[0].group(|ui| {
                            ui.strong("Gas Properties");
                            let mut tg_c = self.gas_input.t_gas_k - 273.15;
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("common.059"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut tg_c)
                                            .speed(5.0)
                                            .range(-50.0..=3000.0),
                                    )
                                    .changed()
                                {
                                    self.gas_input.t_gas_k = tg_c + 273.15;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("review.181"));
                                ui.add(
                                    egui::DragValue::new(&mut self.gas_input.pressure_bar)
                                        .speed(0.1)
                                        .range(0.05..=30.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Optical Depth [m]:");
                                ui.add(
                                    egui::DragValue::new(&mut self.gas_input.optical_depth_m)
                                        .speed(0.02)
                                        .range(0.01..=50.0)
                                        .fixed_decimals(3),
                                );
                                if ui
                                    .small_button("Auto L from Sheet")
                                    .on_hover_text("Calcular L según la geometría de la hoja (Tabla 1)")
                                    .clicked()
                                {
                                    let s_idx =
                                        self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                    let opt_g = match self.wb.sheets[s_idx].geometry {
                                        Geometry::Sphere { diameter_m } => {
                                            OpticalDepthGeometry::Sphere { diameter_m }
                                        }
                                        Geometry::Cylinder {
                                            height_m,
                                            diameter_m,
                                        } => OpticalDepthGeometry::Cylinder {
                                            height_m,
                                            diameter_m,
                                        },
                                        Geometry::Cube {
                                            width_m,
                                            length_m,
                                            height_m,
                                        } => OpticalDepthGeometry::Cube {
                                            a_m: width_m,
                                            b_m: length_m,
                                            c_m: height_m,
                                        },
                                        Geometry::Wall { width_m, length_m } => {
                                            OpticalDepthGeometry::Manual(
                                                0.6 * width_m.min(length_m),
                                            )
                                        }
                                    };
                                    if let Ok(l) = opt_g.optical_depth_m() {
                                        self.gas_input.optical_depth_m = l;
                                    }
                                }
                            });
                            ui.add_space(4.0);
                            ui.strong("Surface Properties");
                            let mut ts_c = self.gas_input.t_surface_k - 273.15;
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("common.059"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut ts_c)
                                            .speed(5.0)
                                            .range(-50.0..=2800.0),
                                    )
                                    .changed()
                                {
                                    self.gas_input.t_surface_k = ts_c + 273.15;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Emissivity:");
                                ui.add(
                                    egui::DragValue::new(&mut self.gas_input.surface_emissivity)
                                        .speed(0.02)
                                        .range(0.0..=1.0),
                                );
                            });
                        });

                        // Bottom Right: Highlighted outputs + Calculate / Select at fixed T / Select iterative
                        cols[1].group(|ui| {
                            if let Some((res, _, _)) = &self.gas_result {
                                egui::Grid::new("gas_rad_summary_grid")
                                    .spacing([10.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.strong("Gas Emissivity at Gas Temperature");
                                        egui::Frame::none()
                                            .fill(yellow_bg)
                                            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.monospace(format!("{:.4}", res.gas_emissivity));
                                            });
                                        ui.end_row();

                                        ui.strong("Gas Absorptivity at Surface Temperature");
                                        egui::Frame::none()
                                            .fill(yellow_bg)
                                            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.monospace(format!(
                                                    "{:.4}",
                                                    res.gas_absorptivity
                                                ));
                                            });
                                        ui.end_row();

                                        ui.strong("Heat Flux [kW/m²]");
                                        ui.monospace(format!("{:.2}", res.heat_flux_w_m2 / 1000.0));
                                        ui.end_row();
                                    });
                            }
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.button(crate::i18n::text("common.008")).clicked() {
                                    if let Ok(res) =
                                        SoryntelApplication::calculate_heat_loss_gas_radiation(
                                            &self.gas_input,
                                        )
                                    {
                                        self.particle_spec.gas_emissivity_at_gas_temp =
                                            res.gas_emissivity;
                                        self.particle_spec.gas_absorptivity_at_surface_temp =
                                            res.gas_absorptivity;
                                        self.gas_result = Some((res, 0.0, 0.0));
                                    }
                                }
                                if ui
                                    .button(crate::i18n::text("common.026"))
                                    .on_hover_text("Transferir T_gas, T_surf, ε_g y α_g fijos a la columna activa")
                                    .clicked()
                                {
                                    if let Some((res, _, _)) = &self.gas_result {
                                        let s_idx =
                                            self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                        if let Some(col) = self.wb.sheets[s_idx]
                                            .columns
                                            .get_mut(self.selected_col)
                                        {
                                            col.t_left_k = self.gas_input.t_gas_k;
                                            col.t_right_k = self.gas_input.t_surface_k;
                                            col.emissivity = self.gas_input.surface_emissivity;
                                            col.gas_emissivity = Some(res.gas_emissivity);
                                            col.gas_absorptivity = Some(res.gas_absorptivity);
                                            col.hr_w_m2_k = res.hr_w_m2_k;
                                            col.hr_fixed = false;
                                        }
                                        self.recalculate();
                                    }
                                }
                                if ui
                                    .button(crate::i18n::text("common.060"))
                                    .on_hover_text("Iterar emisividad entre 0 °C y 3000 °C cada 50 °C (Ch. 12.3.7)")
                                    .clicked()
                                {
                                    let tbl = SoryntelApplication::generate_heat_loss_iterative_gas_radiation_table(
                                        self.gas_input.pressure_bar,
                                        self.gas_input.optical_depth_m,
                                        &self.gas_input.composition,
                                    );
                                    let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                    if let Some(col) =
                                        self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                    {
                                        col.emissivity = self.gas_input.surface_emissivity;
                                        col.iterative_gas_eps_table = tbl;
                                        col.hr_fixed = false;
                                    }
                                    self.recalculate();
                                }
                            });
                        });
                    });
                });
            if !open {
                self.show_gas_rad = false;
            }
        }

        // 5. Particle Suspension Radiation Calculator (Fig. 10)
        if self.show_particle_rad {
            let mut open = true;
            egui::Window::new("✨ Particle Suspension Radiation Calculator (Fig. 10)")
                .open(&mut open)
                .default_size([780.0, 560.0])
                .resizable(true)
                .show(ctx, |ui| {
                    let mut p_changed = false;
                    ui.columns(2, |cols| {
                        // Left Top: 8-row User Input table (Fig. 10)
                        cols[0].group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(37, 99, 235),
                                    egui::RichText::new(crate::i18n::text("common.058")).strong(),
                                );
                            });
                            let ps = &mut self.particle_spec;
                            egui::Grid::new("part_rad_input_grid")
                                .striped(true)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    ui.strong("Property");
                                    ui.strong("Unit");
                                    ui.strong("Value");
                                    ui.end_row();

                                    for (lbl, unit, val_ref, dec) in [
                                        ("Gas Amount", "Nm³/h", &mut ps.gas_amount_nm3_h, 0usize),
                                        ("Particle Amount", "t/h", &mut ps.particle_amount_t_h, 1),
                                        ("Particle Density", "g/ml", &mut ps.particle_density_g_ml, 2),
                                        ("Particle Diameter", "mm", &mut ps.particle_diameter_mm, 3),
                                        ("Particle Emissivity", "", &mut ps.particle_emissivity, 3),
                                        ("Cube Width & Height", "m", &mut ps.cube_width_height_m, 3),
                                        ("Cube Thickness", "m", &mut ps.cube_thickness_m, 3),
                                        ("Tube Diameter", "m", &mut ps.tube_diameter_m, 3),
                                    ] {
                                        ui.label(lbl);
                                        ui.label(unit);
                                        p_changed |= ui
                                            .add(
                                                egui::DragValue::new(val_ref)
                                                    .speed(0.05)
                                                    .range(0.0001..=1e7)
                                                    .fixed_decimals(dec),
                                            )
                                            .changed();
                                        ui.end_row();
                                    }
                                });

                            ui.separator();
                            ui.strong("Gas & Surface Properties");
                            let mut tg_c = ps.t_gas_k - 273.15;
                            ui.horizontal(|ui| {
                                ui.label("Gas Temp [°C]:");
                                if ui.add(egui::DragValue::new(&mut tg_c).speed(5.0)).changed() {
                                    ps.t_gas_k = tg_c + 273.15;
                                    p_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Gas ε at Tg:");
                                p_changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut ps.gas_emissivity_at_gas_temp)
                                            .speed(0.01)
                                            .range(0.0..=1.0),
                                    )
                                    .changed();
                                ui.label("Gas α at Ts:");
                                p_changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut ps.gas_absorptivity_at_surface_temp,
                                        )
                                        .speed(0.01)
                                        .range(0.0..=1.0),
                                    )
                                    .changed();
                            });
                            let mut ts_c = ps.t_surface_k - 273.15;
                            ui.horizontal(|ui| {
                                ui.label("Surface Temp [°C]:");
                                if ui.add(egui::DragValue::new(&mut ts_c).speed(5.0)).changed() {
                                    ps.t_surface_k = ts_c + 273.15;
                                    p_changed = true;
                                }
                                ui.label("Surface ε:");
                                p_changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut ps.surface_emissivity)
                                            .speed(0.01)
                                            .range(0.0..=1.0),
                                    )
                                    .changed();
                            });
                        });

                        // Right Top: 16-row Results table (Fig. 10)
                        cols[1].group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.colored_label(
                                    egui::Color32::from_rgb(37, 99, 235),
                                    egui::RichText::new(crate::i18n::text("common.108")).strong(),
                                );
                            });
                            if p_changed || self.particle_detailed_res.is_none() {
                                self.particle_detailed_res =
                                    SoryntelApplication::evaluate_heat_loss_particle_suspension_detailed(
                                        &self.particle_spec,
                                    )
                                    .ok();
                            }
                            if let Some(pr) = &self.particle_detailed_res {
                                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                                    egui::Grid::new("part_rad_results_16row_grid")
                                        .striped(true)
                                        .spacing([12.0, 3.0])
                                        .show(ui, |ui| {
                                            ui.strong("Property");
                                            ui.strong("Unit");
                                            ui.strong("Value");
                                            ui.end_row();

                                            for (lbl, unit, val_str) in [
                                                ("Gas", "m³/h", format!("{:.1}", pr.gas_flow_m3_h)),
                                                ("Particles", "m³/h", format!("{:.3}", pr.particle_flow_m3_h)),
                                                ("Gas + Particles", "m³/h", format!("{:.1}", pr.total_flow_m3_h)),
                                                ("Particles", "g/m³", format!("{:.2}", pr.particles_g_m3)),
                                                ("Particles", "ml/m³", format!("{:.2}", pr.particles_ml_m3)),
                                                ("Particles", "vol-%", format!("{:.4}", pr.particles_vol_pct)),
                                                ("Particle Radius", "mm", format!("{:.4}", pr.particle_radius_mm)),
                                                ("P.Cross-sectional Area", "mm²", format!("{:.5}", pr.particle_cross_section_mm2)),
                                                ("Particle Volume", "mm³", format!("{:.6e}", pr.particle_volume_mm3)),
                                                ("Particle Weight", "g", format!("{:.6e}", pr.particle_weight_g)),
                                                ("Particles/m³", "", format!("{:.3e}", pr.particles_per_m3)),
                                                ("Cube Volume", "m³", format!("{:.6e}", pr.cube_volume_m3)),
                                                ("Particles in Cube", "", format!("{:.2}", pr.particles_in_cube)),
                                                ("Particle Distances", "mm", format!("{:.3}", pr.particle_distance_mm)),
                                                ("Gas Speed", "m³/sec", format!("{:.3}", pr.gas_speed_m3_s)),
                                                ("Gas Speed in Tube", "m/sec", format!("{:.3}", pr.gas_speed_in_tube_m_s)),
                                            ] {
                                                ui.label(lbl);
                                                ui.label(unit);
                                                ui.monospace(val_str);
                                                ui.end_row();
                                            }
                                        });
                                });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.strong("Particle cloud Emissivity:");
                                    egui::Frame::none()
                                        .fill(yellow_bg)
                                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.monospace(format!("{:.4}", pr.cloud_emissivity));
                                        });
                                    ui.strong("Heat Flux [kW/m²]:");
                                    ui.monospace(format!("{:.2}", pr.heat_flux_kw_m2));
                                });
                            }
                        });
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("🖼 Image (Cloud Schematic)").clicked() {
                            self.show_particle_image = !self.show_particle_image;
                        }
                        if ui.button(crate::i18n::text("common.008")).clicked() {
                            self.particle_detailed_res =
                                SoryntelApplication::evaluate_heat_loss_particle_suspension_detailed(
                                    &self.particle_spec,
                                )
                                .ok();
                        }
                        if ui.button(crate::i18n::text("common.026")).clicked() {
                            if let Some(pr) = &self.particle_detailed_res {
                                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                if let Some(col) =
                                    self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                {
                                    col.particle_cloud_emissivity = Some(pr.cloud_emissivity);
                                    col.gas_emissivity =
                                        Some(self.particle_spec.gas_emissivity_at_gas_temp);
                                    col.gas_absorptivity =
                                        Some(self.particle_spec.gas_absorptivity_at_surface_temp);
                                    col.hr_w_m2_k = pr.hr_w_m2_k;
                                    col.hr_fixed = false;
                                }
                                self.recalculate();
                            }
                        }
                        if ui.button(crate::i18n::text("common.060")).clicked() {
                            if let Some(pr) = &self.particle_detailed_res {
                                let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                                if let Some(col) =
                                    self.wb.sheets[s_idx].columns.get_mut(self.selected_col)
                                {
                                    col.particle_cloud_emissivity = Some(pr.cloud_emissivity);
                                    col.hr_fixed = false;
                                }
                                self.recalculate();
                            }
                        }
                    });

                    if self.show_particle_image {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(320.0, 90.0), egui::Sense::hover());
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 41, 59));
                        for idx in 0..28 {
                            let fx = ((idx * 37) % 100) as f32 / 100.0;
                            let fy = ((idx * 61) % 100) as f32 / 100.0;
                            let pt = egui::pos2(
                                rect.left() + 12.0 + fx * (rect.width() - 24.0),
                                rect.top() + 10.0 + fy * (rect.height() - 20.0),
                            );
                            p.circle_filled(pt, 3.0, egui::Color32::from_rgb(251, 146, 60));
                        }
                    }
                });
            if !open {
                self.show_particle_rad = false;
            }
        }

        // 6. Target Dialog Window (Fig. 11 — Goal Seek Table)
        if self.show_target {
            let mut open = true;
            egui::Window::new("🎯 Target Dialog (Fig. 11)")
                .open(&mut open)
                .default_size([720.0, 360.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.columns(2, |cols| {
                        cols[0].group(|ui| {
                            ui.strong("Target Calculation Sheet (Goal seek)");
                            egui::ScrollArea::both().max_height(210.0).show(ui, |ui| {
                                egui::Grid::new("target_dialog_sheet_grid")
                                    .striped(true)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.strong("#");
                                        ui.strong("Variable Name");
                                        ui.strong("Test value");
                                        ui.strong("Min");
                                        ui.strong("Max");
                                        ui.strong("Target");
                                        ui.strong("Value (kW)");
                                        ui.strong("Max Iter");
                                        ui.end_row();

                                        for (r_idx, row) in self.target_rows.iter_mut().enumerate()
                                        {
                                            let is_sel = self.selected_target_row == r_idx;
                                            if ui
                                                .selectable_label(is_sel, format!("{}", r_idx + 1))
                                                .clicked()
                                            {
                                                self.selected_target_row = r_idx;
                                            }
                                            egui::ComboBox::from_id_salt(format!("trow_v_{r_idx}"))
                                                .width(120.0)
                                                .selected_text(row.var.label())
                                                .show_ui(ui, |ui| {
                                                    for v in [
                                                        TargetVariable::Width,
                                                        TargetVariable::Length,
                                                        TargetVariable::Height,
                                                        TargetVariable::Diameter,
                                                        TargetVariable::LayerThickness(
                                                            self.selected_col,
                                                        ),
                                                        TargetVariable::LeftBoundaryTempK,
                                                        TargetVariable::RightBoundaryTempK,
                                                    ] {
                                                        ui.selectable_value(
                                                            &mut row.var,
                                                            v,
                                                            v.label(),
                                                        );
                                                    }
                                                });
                                            ui.colored_label(
                                                red_fix,
                                                format!("{:.3}", row.test_val),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut row.min_val).speed(0.1),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut row.max_val).speed(0.5),
                                            );
                                            ui.label(&row.target_name);
                                            egui::Frame::none()
                                                .fill(yellow_bg)
                                                .show(ui, |ui| {
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut row.target_val_kw,
                                                        )
                                                        .speed(0.05),
                                                    );
                                                });
                                            ui.add(
                                                egui::DragValue::new(&mut row.max_iters)
                                                    .range(5..=200),
                                            );
                                            ui.end_row();
                                        }
                                    });
                            });
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button(crate::i18n::text("common.046")).clicked() {
                                    self.target_rows.push(TargetDialogRow {
                                        var: TargetVariable::LayerThickness(self.selected_col),
                                        test_val: 0.10,
                                        min_val: 0.005,
                                        max_val: 2.0,
                                        target_name: "Heat flow".into(),
                                        target_val_kw: 0.50,
                                        max_iters: 20,
                                    });
                                }
                                if ui.button(crate::i18n::text("common.047")).clicked() && self.target_rows.len() > 1 {
                                    let idx =
                                        self.selected_target_row.min(self.target_rows.len() - 1);
                                    self.target_rows.remove(idx);
                                    self.selected_target_row =
                                        self.selected_target_row.min(self.target_rows.len() - 1);
                                }
                            });
                        });

                        cols[1].group(|ui| {
                            ui.strong("Iteration Controls (Fig. 11)");
                            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
                            if ui.button("▶ Iterate Selected Rows").clicked() {
                                let r_idx =
                                    self.selected_target_row.min(self.target_rows.len() - 1);
                                let row = self.target_rows[r_idx].clone();
                                let spec = TargetSpec {
                                    sheet_index: s_idx,
                                    variable: row.var,
                                    goal: TargetGoal::TotalHeatFlowW(row.target_val_kw * 1000.0),
                                    min_val: row.min_val,
                                    max_val: row.max_val,
                                    max_iterations: row.max_iters.max(20),
                                    tolerance: 1e-4,
                                };
                                match SoryntelApplication::solve_heat_loss_target(&self.wb, &spec) {
                                    Ok(tr) => {
                                        self.target_rows[r_idx].test_val = tr.solution_value;
                                        soryntel_thermo::heat_loss::target::apply_variable(
                                            &mut self.wb,
                                            s_idx,
                                            row.var,
                                            tr.solution_value,
                                        );
                                        self.recalculate();
                                        self.target_msg = format!(
                                            "✔ Converged in {} iters: {} = {:.4} (Q = {:.3} kW)",
                                            tr.iterations,
                                            row.var.label(),
                                            tr.solution_value,
                                            tr.achieved_goal_value / 1000.0
                                        );
                                    }
                                    Err(e) => {
                                        self.target_msg = format!("Error: {e}");
                                    }
                                }
                            }
                            if ui.button(crate::i18n::text("common.048")).clicked() {
                                for r_idx in 0..self.target_rows.len() {
                                    let row = self.target_rows[r_idx].clone();
                                    let spec = TargetSpec {
                                        sheet_index: s_idx,
                                        variable: row.var,
                                        goal: TargetGoal::TotalHeatFlowW(
                                            row.target_val_kw * 1000.0,
                                        ),
                                        min_val: row.min_val,
                                        max_val: row.max_val,
                                        max_iterations: row.max_iters.max(20),
                                        tolerance: 1e-4,
                                    };
                                    if let Ok(tr) =
                                        SoryntelApplication::solve_heat_loss_target(&self.wb, &spec)
                                    {
                                        self.target_rows[r_idx].test_val = tr.solution_value;
                                        soryntel_thermo::heat_loss::target::apply_variable(
                                            &mut self.wb,
                                            s_idx,
                                            row.var,
                                            tr.solution_value,
                                        );
                                    }
                                }
                                self.recalculate();
                            }
                            if !self.target_msg.is_empty() {
                                ui.separator();
                                ui.label(&self.target_msg);
                            }
                        });
                    });
                });
            if !open {
                self.show_target = false;
            }
        }

        // 7. HSC Diagrams Window (Figs. 3, 12, 13, 14)
        if self.show_diagram {
            let mut open = true;
            egui::Window::new("📈 HSC Diagrams — Heat Loss (Figs. 3, 12-14)")
                .open(&mut open)
                .default_size([760.0, 500.0])
                .min_size([380.0, 240.0])
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.diagram_kind == DiagramKind::TemperatureProfile,
                                "🔥 Temperature Profile T(x) (Fig. 3)",
                            )
                            .clicked()
                        {
                            self.diagram_kind = DiagramKind::TemperatureProfile;
                            self.refresh_diagram();
                        }
                        if ui
                            .selectable_label(
                                self.diagram_kind == DiagramKind::ParameterSweep,
                                "📈 Parameter Sweep (Figs. 12-13)",
                            )
                            .clicked()
                        {
                            self.diagram_kind = DiagramKind::ParameterSweep;
                            self.refresh_diagram();
                        }
                        if ui
                            .selectable_label(
                                self.diagram_kind == DiagramKind::OptimumInsulation,
                                "💰 Optimum Insulation Thickness (Fig. 14)",
                            )
                            .clicked()
                        {
                            self.diagram_kind = DiagramKind::OptimumInsulation;
                            self.refresh_diagram();
                        }
                        ui.separator();
                        ui.checkbox(&mut self.diagram_show_table, "📋 Data Table");
                        ui.checkbox(&mut self.diagram_show_grid, "Grid");
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .checkbox(&mut self.diagram_auto_scale, "Auto Scale Axes")
                            .changed()
                        {
                            self.diagram_apply_bounds = !self.diagram_auto_scale;
                        }
                        let mut edited = false;
                        ui.label("X Min:");
                        edited |= ui
                            .add(egui::DragValue::new(&mut self.diagram_x_min).speed(1.0))
                            .changed();
                        ui.label("X Max:");
                        edited |= ui
                            .add(egui::DragValue::new(&mut self.diagram_x_max).speed(1.0))
                            .changed();
                        ui.label("Y Min:");
                        edited |= ui
                            .add(egui::DragValue::new(&mut self.diagram_y_min).speed(1.0))
                            .changed();
                        ui.label("Y Max:");
                        edited |= ui
                            .add(egui::DragValue::new(&mut self.diagram_y_max).speed(1.0))
                            .changed();
                        if edited || ui.small_button(crate::i18n::text("common.050")).clicked() {
                            self.diagram_auto_scale = false;
                            self.diagram_apply_bounds = true;
                        }
                    });
                    ui.separator();

                    let apply_b = self.diagram_apply_bounds
                        && !self.diagram_auto_scale
                        && self.diagram_x_max > self.diagram_x_min
                        && self.diagram_y_max > self.diagram_y_min;
                    let (bx0, bx1, by0, by1) = (
                        self.diagram_x_min,
                        self.diagram_x_max,
                        self.diagram_y_min,
                        self.diagram_y_max,
                    );

                    let plot_w = ui.available_width().max(240.0);
                    let plot_h = (ui.available_height()
                        - if self.diagram_show_table { 145.0 } else { 28.0 })
                    .max(160.0);

                    match self.diagram_kind {
                        DiagramKind::TemperatureProfile => {
                            ui.strong(format!(
                                "Total Heat flow: {:.3} kW",
                                self.profile_total_w / 1000.0
                            ));
                            Plot::new("hl_profile_plot")
                                .width(plot_w)
                                .height(plot_h)
                                .legend(Legend::default())
                                .show_grid(self.diagram_show_grid)
                                .x_axis_label("Distance mm")
                                .y_axis_label("°C")
                                .show(ui, |plot_ui| {
                                    if apply_b {
                                        plot_ui.set_plot_bounds(
                                            egui_plot::PlotBounds::from_min_max(
                                                [bx0, by0],
                                                [bx1, by1],
                                            ),
                                        );
                                    }
                                    for ser in &self.profile_series {
                                        let pts: PlotPoints = ser
                                            .points
                                            .iter()
                                            .map(|&(x, t_k)| [x * 1000.0, t_k - 273.15])
                                            .collect();
                                        plot_ui.line(Line::new(pts).name(&ser.sheet_name).width(2.2));
                                    }
                                });
                        }
                        DiagramKind::ParameterSweep => {
                            ui.horizontal(|ui| {
                                ui.label("MIN (K):");
                                ui.add(egui::DragValue::new(&mut self.sweep_min).speed(5.0));
                                ui.label("MAX (K):");
                                ui.add(egui::DragValue::new(&mut self.sweep_max).speed(5.0));
                                ui.label("Steps:");
                                ui.add(
                                    egui::DragValue::new(&mut self.sweep_steps).range(2..=100),
                                );
                                if ui.button("🔄 Update").clicked() {
                                    self.refresh_diagram();
                                }
                            });
                            Plot::new("hl_sweep_plot")
                                .width(plot_w)
                                .height(plot_h)
                                .legend(Legend::default())
                                .show_grid(self.diagram_show_grid)
                                .x_axis_label("Temperature (left edge) °C")
                                .y_axis_label("kW")
                                .show(ui, |plot_ui| {
                                    if apply_b {
                                        plot_ui.set_plot_bounds(
                                            egui_plot::PlotBounds::from_min_max(
                                                [bx0, by0],
                                                [bx1, by1],
                                            ),
                                        );
                                    }
                                    let pts: PlotPoints = self
                                        .sweep_points
                                        .iter()
                                        .map(|p| [p.x - 273.15, p.total_heat_flow_w / 1000.0])
                                        .collect();
                                    plot_ui.line(Line::new(pts).name("Heat flow").width(2.2));
                                });
                        }
                        DiagramKind::OptimumInsulation => {
                            if let Some(best) = self.opt_best {
                                ui.strong(format!(
                                    "Optimum thickness: {:.1} cm (Total annual cost: {:.2})",
                                    best.thickness_m * 100.0,
                                    best.total_annual_cost
                                ));
                            }
                            Plot::new("hl_opt_ins_plot")
                                .width(plot_w)
                                .height(plot_h)
                                .legend(Legend::default())
                                .show_grid(self.diagram_show_grid)
                                .x_axis_label("Thickness (cm)")
                                .y_axis_label("Annual Cost")
                                .show(ui, |plot_ui| {
                                    if apply_b {
                                        plot_ui.set_plot_bounds(
                                            egui_plot::PlotBounds::from_min_max(
                                                [bx0, by0],
                                                [bx1, by1],
                                            ),
                                        );
                                    }
                                    let tot: PlotPoints = self
                                        .opt_points
                                        .iter()
                                        .map(|p| [p.thickness_m * 100.0, p.total_annual_cost])
                                        .collect();
                                    let ene: PlotPoints = self
                                        .opt_points
                                        .iter()
                                        .map(|p| [p.thickness_m * 100.0, p.annual_energy_cost])
                                        .collect();
                                    let ins: PlotPoints = self
                                        .opt_points
                                        .iter()
                                        .map(|p| [p.thickness_m * 100.0, p.annual_insulation_cost])
                                        .collect();
                                    plot_ui.line(Line::new(tot).name("Total Cost").width(2.2));
                                    plot_ui.line(Line::new(ene).name("Energy Loss Cost"));
                                    plot_ui.line(Line::new(ins).name("Insulation Cost"));
                                });
                        }
                    }
                    if apply_b {
                        self.diagram_apply_bounds = false;
                    }

                    if self.diagram_show_table {
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            egui::Grid::new("hl_diag_data_tbl").striped(true).show(ui, |ui| {
                                ui.strong("Series / Point");
                                ui.strong("X");
                                ui.strong("Y");
                                ui.end_row();
                                for ser in &self.profile_series {
                                    for &(x, t_k) in &ser.points {
                                        ui.label(&ser.sheet_name);
                                        ui.monospace(format!("{:.2} mm", x * 1000.0));
                                        ui.monospace(format!("{:.2} °C", t_k - 273.15));
                                        ui.end_row();
                                    }
                                }
                            });
                        });
                    }
                });
            if !open {
                self.show_diagram = false;
            }
        }

        // 8. Material Database Window (Ch. 90, Figs. 1-3)
        self.render_material_database_modal(ctx);

        // Shape Preview Modal (triggered by "Draw" button in Shape and Dimensions)
        if self.show_shape_draw {
            let mut open = self.show_shape_draw;
            let s_idx = self.wb.active_sheet.min(self.wb.sheets.len() - 1);
            let geom = self.wb.sheets[s_idx].geometry;
            egui::Window::new("📐 Shape and Dimensions — Schematic")
                .open(&mut open)
                .default_size([360.0, 260.0])
                .show(ctx, |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(320.0, 170.0), egui::Sense::hover());
                    let p = ui.painter_at(rect);
                    p.rect_filled(rect, 4.0, egui::Color32::WHITE);
                    let stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(37, 99, 235));
                    let fill = egui::Color32::from_rgb(219, 234, 254);
                    let c = rect.center();
                    match geom {
                        Geometry::Wall { width_m, length_m } => {
                            let r = egui::Rect::from_center_size(c, egui::vec2(180.0, 90.0));
                            p.rect_filled(r, 0.0, fill);
                            p.rect_stroke(r, 0.0, stroke);
                            ui.label(format!(
                                "Wall: Width = {width_m:.2} m, Length = {length_m:.2} m, Area = {:.3} m²",
                                width_m * length_m
                            ));
                        }
                        Geometry::Cube {
                            width_m,
                            length_m,
                            height_m,
                        } => {
                            let front = egui::Rect::from_center_size(
                                c + egui::vec2(-12.0, 12.0),
                                egui::vec2(120.0, 80.0),
                            );
                            let back = front.translate(egui::vec2(28.0, -24.0));
                            p.rect_filled(back, 0.0, fill);
                            p.rect_stroke(back, 0.0, stroke);
                            p.rect_filled(front, 0.0, fill);
                            p.rect_stroke(front, 0.0, stroke);
                            ui.label(format!(
                                "Cube: {width_m:.2} × {length_m:.2} × {height_m:.2} m"
                            ));
                        }
                        Geometry::Cylinder {
                            height_m,
                            diameter_m,
                        } => {
                            let r = egui::Rect::from_center_size(c, egui::vec2(100.0, 110.0));
                            p.rect_filled(r, 12.0, fill);
                            p.rect_stroke(r, 12.0, stroke);
                            ui.label(format!(
                                "Cylinder: Height = {height_m:.2} m, Diameter = {diameter_m:.2} m"
                            ));
                        }
                        Geometry::Sphere { diameter_m } => {
                            p.circle_filled(c, 55.0, fill);
                            p.circle_stroke(c, 55.0, stroke);
                            ui.label(format!(
                                "Sphere: Diameter = {diameter_m:.2} m, Area = {:.3} m²",
                                std::f64::consts::PI * diameter_m * diameter_m
                            ));
                        }
                    }
                });
            self.show_shape_draw = open;
        }
    }

    fn render_material_database_modal(&mut self, ctx: &egui::Context) {
        if !self.show_material_db {
            return;
        }
        let mut open = true;
        egui::Window::new("🗄 HSC Material Database (Ch. 90, Figs. 1–3)")
            .open(&mut open)
            .default_size([980.0, 560.0])
            .resizable(true)
            .show(ctx, |ui| {
                let m_idx = self
                    .selected_material_idx
                    .min(self.material_db.materials.len().saturating_sub(1));
                self.selected_material_idx = m_idx;

                ui.horizontal(|ui| {
                    ui.strong("Material:");
                    egui::ComboBox::from_id_salt("mat_db_selector")
                        .width(220.0)
                        .selected_text(&self.material_db.materials[m_idx].name)
                        .show_ui(ui, |ui| {
                            for (i, m) in self.material_db.materials.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.selected_material_idx,
                                    i,
                                    format!("{} ({})", m.name, m.category),
                                );
                            }
                        });
                    if ui.button("➕ New Material").clicked() {
                        let mut clone_m = self.material_db.materials[0].clone();
                        clone_m.name = format!("New Material {}", self.material_db.materials.len() + 1);
                        self.material_db.materials.push(clone_m);
                        self.selected_material_idx = self.material_db.materials.len() - 1;
                    }
                    if ui.button("⚡ Auto Calculate Properties").clicked() {
                        let _ = SoryntelApplication::recalculate_material_auto_properties(
                            &mut self.material_db,
                            self.selected_material_idx,
                        );
                    }
                    if ui
                        .button("📦 Send to Conduction DB")
                        .on_hover_text("Agregar este material a la base de datos de conducción térmica")
                        .clicked()
                    {
                        let m = &self.material_db.materials[self.selected_material_idx];
                        self.solids.push(SolidConductivityData {
                            name: m.name.clone(),
                            category: m.category.clone(),
                            density_kg_m3: Some(4062.0),
                            k_table: vec![(300.0, 1.45), (600.0, 1.65), (1000.0, 1.90)],
                            reference: "Material DB (Ch. 90)".into(),
                        });
                        self.status_msg =
                            format!("Material '{}' añadido a Conduction DB.", m.name);
                    }
                });
                ui.separator();

                let mut comp_changed = false;
                let resolved = SoryntelApplication::resolve_material_composition(
                    &self.material_db,
                    self.selected_material_idx,
                )
                .ok();

                ui.columns(2, |cols| {
                    // Left Pane: Composition | Analyses tabs (Fig. 1)
                    cols[0].group(|ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.mat_left_tab,
                                MaterialLeftTab::Composition,
                                "Composition (Fig. 1)",
                            );
                            ui.selectable_value(
                                &mut self.mat_left_tab,
                                MaterialLeftTab::Analyses,
                                "Analyses",
                            );
                        });
                        ui.separator();
                        let mat_name = self.material_db.materials[self.selected_material_idx]
                            .name
                            .clone();
                        ui.strong(format!("Material: {mat_name}"));

                        match self.mat_left_tab {
                            MaterialLeftTab::Composition => {
                                let (comp_sum, sp_list, el_list) =
                                    resolved.clone().unwrap_or_default();
                                let sp_sum: f64 = sp_list.iter().map(|s| s.wt_pct).sum();
                                let el_sum: f64 = el_list.iter().map(|e| e.wt_pct).sum();
                                let max_rows = self.material_db.materials
                                    [self.selected_material_idx]
                                    .composition
                                    .len()
                                    .max(sp_list.len())
                                    .max(el_list.len())
                                    .max(6);

                                egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
                                    egui::Grid::new("mat_db_fig1_comp_grid")
                                        .striped(true)
                                        .spacing([8.0, 3.0])
                                        .show(ui, |ui| {
                                            // Row 1 Header (greenish header bar as in Fig. 1)
                                            ui.strong("Composition");
                                            ui.strong("wt-%");
                                            ui.strong("Species");
                                            ui.strong("wt-%");
                                            ui.strong("Elements");
                                            ui.strong("wt-%");
                                            ui.strong("Reference");
                                            ui.end_row();

                                            // Row 2 Totals: Total | 100.000 | | 100.000 | | 100.000
                                            ui.strong("Total");
                                            ui.strong(format!("{comp_sum:.3}"));
                                            ui.label("");
                                            ui.strong(format!("{sp_sum:.3}"));
                                            ui.label("");
                                            ui.strong(format!("{el_sum:.3}"));
                                            ui.label("");
                                            ui.end_row();

                                            let mat = &mut self.material_db.materials
                                                [self.selected_material_idx];
                                            for r_i in 0..max_rows {
                                                if let Some(c_row) = mat.composition.get_mut(r_i) {
                                                    comp_changed |= ui
                                                        .add(
                                                            egui::TextEdit::singleline(
                                                                &mut c_row.component,
                                                            )
                                                            .desired_width(95.0),
                                                        )
                                                        .changed();
                                                    comp_changed |= ui
                                                        .add(
                                                            egui::DragValue::new(&mut c_row.wt_pct)
                                                                .speed(0.5)
                                                                .range(0.0..=100.0)
                                                                .fixed_decimals(3),
                                                        )
                                                        .changed();
                                                } else {
                                                    ui.label("");
                                                    ui.label("");
                                                }

                                                if let Some(sp) = sp_list.get(r_i) {
                                                    ui.label(&sp.formula);
                                                    ui.monospace(format!("{:.3}", sp.wt_pct));
                                                } else {
                                                    ui.label("");
                                                    ui.label("");
                                                }

                                                if let Some(el) = el_list.get(r_i) {
                                                    ui.label(&el.element);
                                                    ui.monospace(format!("{:.3}", el.wt_pct));
                                                } else {
                                                    ui.label("");
                                                    ui.label("");
                                                }

                                                if let Some(c_row) = mat.composition.get_mut(r_i) {
                                                    ui.add(
                                                        egui::TextEdit::singleline(
                                                            &mut c_row.reference,
                                                        )
                                                        .desired_width(65.0),
                                                    );
                                                } else {
                                                    ui.label("");
                                                }
                                                ui.end_row();
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    if ui.small_button("➕ Add Component").clicked() {
                                        self.material_db.materials[self.selected_material_idx]
                                            .composition
                                            .push(MaterialCompositionRow {
                                                component: "SiO2".into(),
                                                wt_pct: 0.0,
                                                reference: String::new(),
                                            });
                                        comp_changed = true;
                                    }
                                    if ui.small_button("⚖ Normalize 100%").clicked() {
                                        let mat = &mut self.material_db.materials
                                            [self.selected_material_idx];
                                        let s: f64 =
                                            mat.composition.iter().map(|x| x.wt_pct).sum();
                                        if s > 1e-9 {
                                            for x in &mut mat.composition {
                                                x.wt_pct = 100.0 * x.wt_pct / s;
                                            }
                                            comp_changed = true;
                                        }
                                    }
                                });
                            }
                            MaterialLeftTab::Analyses => {
                                let mat = &mut self.material_db.materials
                                    [self.selected_material_idx];
                                egui::Grid::new("mat_analyses_grid")
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.strong("Analysis Parameter");
                                        ui.strong("Value");
                                        ui.strong("Unit");
                                        ui.end_row();
                                        for (k, v, u) in &mut mat.analyses_notes {
                                            ui.text_edit_singleline(k);
                                            ui.text_edit_singleline(v);
                                            ui.text_edit_singleline(u);
                                            ui.end_row();
                                        }
                                    });
                            }
                        }
                    });

                    // Right Pane: Common | Cp Data | Cp Chart (Figs. 2-3)
                    cols[1].group(|ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.mat_right_top_tab,
                                MaterialRightTopTab::Common,
                                "Common (Fig. 2)",
                            );
                            ui.selectable_value(
                                &mut self.mat_right_top_tab,
                                MaterialRightTopTab::CpData,
                                "Cp Data (Fig. 3)",
                            );
                            ui.selectable_value(
                                &mut self.mat_right_top_tab,
                                MaterialRightTopTab::CpChart,
                                "Cp Chart (Fig. 3)",
                            );
                        });
                        ui.separator();

                        match self.mat_right_top_tab {
                            MaterialRightTopTab::Common => {
                                match self.mat_right_btm_tab {
                                    MaterialRightBottomTab::UserProperties => {
                                        let mat = &mut self.material_db.materials
                                            [self.selected_material_idx];
                                        egui::ScrollArea::vertical().max_height(335.0).show(
                                            ui,
                                            |ui| {
                                                egui::Grid::new("mat_user_props_grid")
                                                    .striped(true)
                                                    .show(ui, |ui| {
                                                        ui.strong("User Property");
                                                        ui.strong("Value");
                                                        ui.strong("Unit");
                                                        ui.strong("Reference");
                                                        ui.end_row();
                                                        for up in &mut mat.user_properties {
                                                            ui.text_edit_singleline(&mut up.name);
                                                            ui.text_edit_singleline(&mut up.value);
                                                            ui.text_edit_singleline(&mut up.unit);
                                                            ui.text_edit_singleline(
                                                                &mut up.reference,
                                                            );
                                                            ui.end_row();
                                                        }
                                                    });
                                                if ui.small_button("➕ Add User Property").clicked()
                                                {
                                                    mat.user_properties.push(MaterialUserProperty {
                                                        name: "Custom Property".into(),
                                                        value: "0.0".into(),
                                                        unit: "-".into(),
                                                        reference: String::new(),
                                                    });
                                                }
                                            },
                                        );
                                    }
                                    _ => {
                                        // Fig. 2 Properties table (Heat & State, Electrical & Magnetic, Mechanical)
                                        let mut auto_toggled = false;
                                        egui::ScrollArea::vertical().max_height(335.0).show(
                                            ui,
                                            |ui| {
                                                let mat = &mut self.material_db.materials
                                                    [self.selected_material_idx];
                                                let mut cur_sec = String::new();
                                                egui::Grid::new("mat_fig2_props_grid")
                                                    .striped(true)
                                                    .spacing([10.0, 3.0])
                                                    .show(ui, |ui| {
                                                        for prop in &mut mat.properties {
                                                            if prop.section != cur_sec {
                                                                cur_sec = prop.section.clone();
                                                                ui.strong(&cur_sec);
                                                                ui.strong("Value");
                                                                ui.strong("Unit");
                                                                ui.strong("Reference");
                                                                ui.strong("Auto Calculate");
                                                                ui.end_row();
                                                            }
                                                            ui.label(&prop.name);
                                                            ui.add(
                                                                egui::TextEdit::singleline(
                                                                    &mut prop.value_text,
                                                                )
                                                                .desired_width(85.0),
                                                            );
                                                            ui.label(&prop.unit);
                                                            ui.add(
                                                                egui::TextEdit::singleline(
                                                                    &mut prop.reference,
                                                                )
                                                                .desired_width(70.0),
                                                            );
                                                            if prop.supports_auto_calc {
                                                                let lbl = if prop.auto_calculate {
                                                                    "TRUE"
                                                                } else {
                                                                    "FALSE"
                                                                };
                                                                if ui
                                                                    .selectable_label(
                                                                        prop.auto_calculate,
                                                                        lbl,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    prop.auto_calculate =
                                                                        !prop.auto_calculate;
                                                                    auto_toggled = true;
                                                                }
                                                            } else {
                                                                ui.label("");
                                                            }
                                                            ui.end_row();
                                                        }
                                                    });
                                            },
                                        );
                                        if auto_toggled {
                                            let _ = SoryntelApplication::recalculate_material_auto_properties(
                                                &mut self.material_db,
                                                self.selected_material_idx,
                                            );
                                        }
                                    }
                                }

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.selectable_value(
                                        &mut self.mat_right_btm_tab,
                                        MaterialRightBottomTab::GeneralInfo,
                                        "General Info",
                                    );
                                    ui.selectable_value(
                                        &mut self.mat_right_btm_tab,
                                        MaterialRightBottomTab::Properties,
                                        "Properties",
                                    );
                                    ui.selectable_value(
                                        &mut self.mat_right_btm_tab,
                                        MaterialRightBottomTab::Data,
                                        "Data",
                                    );
                                    ui.selectable_value(
                                        &mut self.mat_right_btm_tab,
                                        MaterialRightBottomTab::UserProperties,
                                        "User Properties",
                                    );
                                });
                            }
                            MaterialRightTopTab::CpData => {
                                // Fig. 3 Left: Cp(T) = A + B*T*10^-3 + C*T^-2*10^5 + D*T^2*10^-6 + E*T^-3*10^8 + F*T^3*10^-9
                                ui.monospace(
                                    "Cp(T) = A + B*T*10^-3 + C*T^-2*10^5 + D*T^2*10^-6 + E*T^-3*10^8 + F*T^3*10^-9",
                                );
                                ui.separator();
                                let mat = &mut self.material_db.materials
                                    [self.selected_material_idx];
                                egui::Grid::new("mat_fig3_cp_data_grid")
                                    .striped(true)
                                    .spacing([10.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.strong("Range");
                                        for idx in 0..mat.cp_ranges.len() {
                                            ui.strong(format!("{}", idx + 1));
                                        }
                                        ui.end_row();

                                        ui.label("Tmin (K)");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.t_min_k)
                                                    .speed(5.0)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label("Tmax (K)");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.t_max_k)
                                                    .speed(5.0)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label(crate::i18n::text("common.132"));
                                        for r in &mut mat.cp_ranges {
                                            ui.text_edit_singleline(&mut r.phase);
                                        }
                                        ui.end_row();

                                        ui.label("H kJ/kg");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.h_kj_kg)
                                                    .speed(10.0)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label("Cp coefficient A J/(kg*K)");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.a)
                                                    .speed(1.0)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label(crate::i18n::text("common.061"));
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.b)
                                                    .speed(0.5)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label(crate::i18n::text("common.062"));
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.c)
                                                    .speed(0.5)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label(crate::i18n::text("common.063"));
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.d)
                                                    .speed(0.5)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label("Cp coefficient E");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.e)
                                                    .speed(0.5)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();

                                        ui.label("Cp coefficient F");
                                        for r in &mut mat.cp_ranges {
                                            ui.add(
                                                egui::DragValue::new(&mut r.f)
                                                    .speed(0.5)
                                                    .fixed_decimals(3),
                                            );
                                        }
                                        ui.end_row();
                                    });
                                if ui.small_button("➕ Add Temperature Range").clicked() {
                                    let last_tmax = mat
                                        .cp_ranges
                                        .last()
                                        .map(|r| r.t_max_k)
                                        .unwrap_or(1000.0);
                                    mat.cp_ranges.push(MaterialCpRange {
                                        t_min_k: last_tmax,
                                        t_max_k: last_tmax + 500.0,
                                        phase: "l".into(),
                                        h_kj_kg: 0.0,
                                        a: 300.0,
                                        b: 0.0,
                                        c: 0.0,
                                        d: 0.0,
                                        e: 0.0,
                                        f: 0.0,
                                        reference: "User".into(),
                                    });
                                }
                            }
                            MaterialRightTopTab::CpChart => {
                                // Fig. 3 Right: Heat Capacity (J/(kg*K)) vs Temperature (K)
                                let pts_vec = SoryntelApplication::sample_material_cp_curve(
                                    &self.material_db,
                                    self.selected_material_idx,
                                    25,
                                );
                                Plot::new("mat_db_fig3_cp_plot")
                                    .legend(Legend::default())
                                    .height(340.0)
                                    .x_axis_label("Temperature (K)")
                                    .y_axis_label("Heat Capacity (J/(kg*K))")
                                    .show(ui, |plot_ui| {
                                        let pts: PlotPoints =
                                            pts_vec.into_iter().map(|(t, cp)| [t, cp]).collect();
                                        plot_ui.line(Line::new(pts).name("Cp(T)").width(2.0));
                                    });
                            }
                        }
                    });
                });

                if comp_changed {
                    let _ = SoryntelApplication::recalculate_material_auto_properties(
                        &mut self.material_db,
                        self.selected_material_idx,
                    );
                }
            });
        if !open {
            self.show_material_db = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_loss_ui_renders_all_modals_and_material_db_and_htr8_roundtrip() {
        let ctx = egui::Context::default();
        let mut state = HeatLossModuleState::new();
        assert!(state.status_ok);

        // Open all modals and Material Database to verify headless rendering
        state.show_advanced_rows = true;
        state.show_cond_picker = true;
        state.show_conv_calc = true;
        state.show_rad_picker = true;
        state.show_gas_rad = true;
        state.show_particle_rad = true;
        state.show_target = true;
        state.show_diagram = true;
        state.show_material_db = true;

        let mut raw_input = egui::RawInput::default();
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        ));
        let _ = ctx.run(raw_input, |ctx| {
            state.ui(ctx, 2);
        });

        // Test .htr8 save/load roundtrip
        let tmp_htr8 = "/tmp/soryntel_test_heatloss.htr8";
        state.save_htr8(tmp_htr8);
        assert!(state.status_ok);
        state.load_htr8(tmp_htr8);
        assert!(state.status_ok);
        let _ = std::fs::remove_file(tmp_htr8);
    }
}
