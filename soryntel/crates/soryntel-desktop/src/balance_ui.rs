//! Module 2 Desktop UI: Heat and Material Balance Calculator (`HSC - Heat and Material Balance`, Chapter 11, Figs. 1–14).
//!
//! Hexagonal UI Adapter: all thermodynamic calculations, cyclic formula evaluation, adiabatic Brent
//! root-finding, Target Goal-Seek (`F8`), 1D Diagram sweeps, unit conversions, stream scaling,
//! and `.bal8` / `.xlsx` persistence are delegated to `SoryntelApplication`.

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, section_cell, text_cell, widget_cell,
    EXCEL_ALT_ROW_BG, EXCEL_READONLY_BG, EXCEL_SECTION_BG, EXCEL_YELLOW_BG,
};
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Points, Text as PlotText};
use soryntel_application::SoryntelApplication;
use soryntel_core::{BalanceEnergyUnit, TemperatureUnit};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::balance_engine::{
    BalanceArea, BalanceUserSheet, BalanceWorkbook, DiagramResult, SpeciesRow, StreamGroup,
    TargetRow,
};

const DEFAULT_BAL8_PATH: &str = "/tmp/soryntel_balance.bal8";
const DEFAULT_BAL_XLSX_PATH: &str = "/tmp/soryntel_balance_workbook.xlsx";

#[derive(Clone, PartialEq, Eq)]
pub enum ActiveSheetTab {
    Balance,
    In(usize),
    Out(usize),
    InPct(usize),
    OutPct(usize),
    User(usize),
    Target,
}

pub struct BalanceModuleState {
    pub wb: BalanceWorkbook,
    pub active_area: usize,
    pub active_tab: ActiveSheetTab,
    pub selected_stream_idx: Option<usize>,
    pub selected_row_idx: Option<usize>,
    pub selected_user_cell: Option<(usize, usize)>,
    pub formula_bar_ref: String,
    pub formula_bar_text: String,
    pub copy_mode_whole_row: bool,
    pub copied_stream_ref: Option<(usize, bool, usize)>, // (area_idx, is_out, stream_idx)
    pub show_tools_sidebar: bool,
    pub status_msg: String,
    pub status_ok: bool,
    // Modals
    pub temp_bal_modal: Option<f64>,
    pub show_target_dialog: bool,
    pub selected_target_rows: Vec<bool>,
    pub show_diagram_settings: bool,
    pub diagram_result: Option<DiagramResult>,
    pub show_diagram_window: bool,
    pub diagram_reset_counter: u64,
    pub diagram_auto_scale: bool,
    pub diagram_x_min: f64,
    pub diagram_x_max: f64,
    pub diagram_y_min: f64,
    pub diagram_y_max: f64,
    pub diagram_apply_bounds: bool,
    // Heat Flow Estimator Modal (Ch. 11.1 Eq. 1)
    pub show_heat_flow_modal: bool,
    pub hf_area_m2: f64,
    pub hf_t_surface_c: f64,
    pub hf_t_room_c: f64,
    // File Save/Open Modal (.bal8)
    pub show_file_modal: bool,
    pub file_modal_is_save: bool,
    pub file_path_buf: String,
    // Add species modal
    pub show_add_species: bool,
    pub add_to_out: bool,
    pub add_species_query: String,
    pub add_species_kmol: f64,
    // Help modal
    pub show_help_modal: bool,
}

impl BalanceModuleState {
    /// Inject steady-state heat loss (in Watts) from Module 3 into the specified BalanceArea.
    pub fn set_area_heat_loss_w(&mut self, area_idx: usize, q_w: f64, db: &ThermoDatabase) {
        if let Some(area) = self.wb.areas.get_mut(area_idx) {
            let energy_per_h = SoryntelApplication::heat_flow_w_to_balance_energy_per_hour(
                q_w,
                self.wb.energy_unit,
            );
            area.heat_losses = energy_per_h;
            let _ = SoryntelApplication::evaluate_balance_workbook(db, &mut self.wb);
            self.status_msg = format!(
                "Importado desde Módulo 3 -> Area {}: {:.3} {}/h ({:.3} kW)",
                area_idx + 1,
                energy_per_h,
                self.wb.energy_unit.label(),
                q_w / 1000.0
            );
            self.status_ok = true;
        }
    }

    pub fn new(db: &ThermoDatabase) -> Self {
        let mut wb = SoryntelApplication::balance_preset_cuconv1();
        let _ = SoryntelApplication::evaluate_balance_workbook(db, &mut wb);
        let n_targets = wb.target_rows.len();
        Self {
            wb,
            active_area: 0,
            active_tab: ActiveSheetTab::In(0),
            selected_stream_idx: Some(0),
            selected_row_idx: Some(0),
            selected_user_cell: None,
            formula_bar_ref: "A2".into(),
            formula_bar_text: "GAS:".into(),
            copy_mode_whole_row: true,
            copied_stream_ref: None,
            show_tools_sidebar: true,
            status_msg: "CUCONV1 cargado.".into(),
            status_ok: true,
            temp_bal_modal: None,
            show_target_dialog: false,
            selected_target_rows: vec![true; n_targets],
            show_diagram_settings: false,
            diagram_result: None,
            show_diagram_window: false,
            diagram_reset_counter: 0,
            diagram_auto_scale: true,
            diagram_x_min: 0.0,
            diagram_x_max: 100.0,
            diagram_y_min: 0.0,
            diagram_y_max: 1400.0,
            diagram_apply_bounds: false,
            show_heat_flow_modal: false,
            hf_area_m2: 12.0,
            hf_t_surface_c: 180.0,
            hf_t_room_c: 25.0,
            show_file_modal: false,
            file_modal_is_save: true,
            file_path_buf: DEFAULT_BAL8_PATH.to_string(),
            show_add_species: false,
            add_to_out: false,
            add_species_query: String::new(),
            add_species_kmol: 1.0,
            show_help_modal: false,
        }
    }

    pub fn load_preset(&mut self, mut wb: BalanceWorkbook, db: &ThermoDatabase) {
        match SoryntelApplication::evaluate_balance_workbook(db, &mut wb) {
            Ok(()) => {
                self.selected_target_rows = vec![true; wb.target_rows.len()];
                self.status_msg = format!(
                    "Cargado: {} ({} iter, conv={})",
                    wb.title, wb.circular_iterations, wb.circular_converged
                );
                self.status_ok = true;
                self.wb = wb;
                self.active_area = 0;
                self.active_tab = ActiveSheetTab::In(0);
                self.selected_stream_idx = Some(0);
                self.selected_row_idx = Some(0);
            }
            Err(e) => {
                self.status_msg = format!("Error evaluando preset: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn load_ch4_cooling_preset(&mut self, db: &ThermoDatabase) {
        self.load_preset(SoryntelApplication::balance_preset_ch4_cooling_water(), db);
        self.active_tab = ActiveSheetTab::Out(0);
        if let Ok(res) = SoryntelApplication::run_balance_diagram_sweep(db, &self.wb) {
            self.diagram_auto_scale = false;
            self.diagram_x_min = 1.0;
            self.diagram_x_max = 12.0;
            self.diagram_y_min = 200.0;
            self.diagram_y_max = 1400.0;
            self.diagram_apply_bounds = true;
            self.diagram_reset_counter = self.diagram_reset_counter.wrapping_add(1);
            self.diagram_result = Some(res);
            self.show_diagram_window = true;
        }
    }

    pub fn recalculate(&mut self, db: &ThermoDatabase) {
        match SoryntelApplication::evaluate_balance_workbook(db, &mut self.wb) {
            Ok(()) => {
                self.status_msg = format!(
                    "Balance recalculado ({} iteraciones, convergencia={}).",
                    self.wb.circular_iterations, self.wb.circular_converged
                );
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn save_bal8(&mut self, path: &str) {
        match SoryntelApplication::serialize_bal8_workbook(&self.wb).and_then(|json| {
            crate::storage::write_atomic(path, json)
                .map_err(|e| soryntel_core::SoryntelError::DatabaseError(e.to_string()))
        }) {
            Ok(_) => {
                self.status_msg = format!("Workbook guardado en {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error guardando .bal8: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn load_bal8(&mut self, path: &str, db: &ThermoDatabase) {
        match std::fs::read_to_string(path)
            .map_err(|e| soryntel_core::SoryntelError::DatabaseError(e.to_string()))
            .and_then(|raw| SoryntelApplication::deserialize_bal8_workbook(&raw))
        {
            Ok(wb) => {
                self.load_preset(wb, db);
                self.status_msg = format!("Workbook cargado desde {path}");
            }
            Err(e) => {
                self.status_msg = format!("Error abriendo .bal8: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn export_xlsx(&mut self, path: &str) {
        let bytes = SoryntelApplication::export_balance_workbook_xlsx(&self.wb);
        match crate::storage::write_atomic(path, bytes) {
            Ok(_) => {
                self.status_msg = format!("Exportado Excel (.xlsx) en {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error exportando .xlsx: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn run_selected_targets(&mut self, db: &ThermoDatabase, all: bool) {
        if self.selected_target_rows.len() != self.wb.target_rows.len() {
            self.selected_target_rows
                .resize(self.wb.target_rows.len(), true);
        }
        let indices: Vec<usize> = if all {
            (0..self.wb.target_rows.len()).collect()
        } else {
            self.selected_target_rows
                .iter()
                .enumerate()
                .filter_map(|(i, &s)| if s { Some(i) } else { None })
                .collect()
        };
        match SoryntelApplication::iterate_balance_target_rows(db, &mut self.wb, &indices) {
            Ok(()) => {
                self.status_msg = format!(
                    "Target iteration completada ({} fila(s) resueltas).",
                    indices.len()
                );
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Target error: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn copy_selected_stream(&mut self) {
        let (area_idx, is_out) = match self.active_tab {
            ActiveSheetTab::In(i) => (i, false),
            ActiveSheetTab::Out(i) => (i, true),
            _ => (self.active_area, false),
        };
        if let Some(s_idx) = self.selected_stream_idx {
            self.copied_stream_ref = Some((area_idx, is_out, s_idx));
            self.status_msg = format!(
                "Stream #{} de {}{} copiado (usa Edit -> Paste Stream en otra hoja INx/OUTx).",
                s_idx + 1,
                if is_out { "OUT" } else { "IN" },
                area_idx + 1
            );
            self.status_ok = true;
        }
    }

    pub fn paste_linked_stream(&mut self, db: &ThermoDatabase) {
        let Some((src_area, src_is_out, src_s_idx)) = self.copied_stream_ref else {
            self.status_msg = "Primero selecciona un Stream y usa Edit -> Copy Stream.".into();
            self.status_ok = false;
            return;
        };
        let Some(pasted) = SoryntelApplication::build_balance_linked_paste_stream(
            &self.wb, src_area, src_is_out, src_s_idx,
        ) else {
            return;
        };
        let (dst_area, dst_is_out) = match self.active_tab {
            ActiveSheetTab::In(i) => (i, false),
            ActiveSheetTab::Out(i) => (i, true),
            _ => (self.active_area, false),
        };
        if let Some(area) = self.wb.areas.get_mut(dst_area) {
            let streams = if dst_is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            streams.push(pasted);
            self.selected_stream_idx = Some(streams.len() - 1);
            self.selected_row_idx = None;
        }
        self.recalculate(db);
        self.status_msg = format!(
            "Stream vinculado pegado en {}{} con referencias dinámicas ='{}'!D<row>.",
            if dst_is_out { "OUT" } else { "IN" },
            dst_area + 1,
            format!("{}{}", if src_is_out { "OUT" } else { "IN" }, src_area + 1)
        );
        self.status_ok = true;
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        // F8 shortcut for Target Iteration (Ch. 11.7 Step 8)
        if ctx.input(|i| i.key_pressed(egui::Key::F8)) {
            self.run_selected_targets(db, false);
        }

        // 1. Classic HSC Menu Bar with Semantic Icons (File | Edit | View | Insert | Delete | Format | Units | Calculate | Target | Diagram | Help)
        egui::TopBottomPanel::top("bal_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(crate::i18n::text("common.020"), |ui| {
                    if ui.button(crate::i18n::text("review.104")).clicked() {
                        self.file_modal_is_save = false;
                        self.file_path_buf = DEFAULT_BAL8_PATH.to_string();
                        self.show_file_modal = true;
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.105")).clicked() {
                        self.file_modal_is_save = true;
                        self.file_path_buf = DEFAULT_BAL8_PATH.to_string();
                        self.show_file_modal = true;
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("common.038")).clicked() {
                        self.export_xlsx(DEFAULT_BAL_XLSX_PATH);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Built-in HSC Examples (Ch. 11.11):").strong());
                    if ui
                        .button("🔥 CH4_Cooling.bal8 (CH4 Burner + Cooling Water Diagram)")
                        .clicked()
                    {
                        self.load_ch4_cooling_preset(db);
                        ui.close_menu();
                    }
                    if ui.button("📘 CUCONV1.bal8 (Copper Converter, Figs. 1–5)").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_cuconv1(), db);
                        ui.close_menu();
                    }
                    if ui.button("🔥 BUTANE.bal8 (Adiabatic Flame, Fig. 2)").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_butane(), db);
                        ui.close_menu();
                    }
                    if ui.button("🏭 CUSMELT.bal8 (Copper Smelting)").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_cusmelt(), db);
                        ui.close_menu();
                    }
                    if ui.button("💧 H2O.bal8 (Steam Evaporator)").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_h2o(), db);
                        ui.close_menu();
                    }
                    if ui.button("⛓ FSF_process.bal8 (5-Area Flash Smelting, Figs. 10–14)").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_fsf_process(), db);
                        ui.close_menu();
                    }
                });

                ui.menu_button(crate::i18n::text("common.021"), |ui| {
                    if ui.button(crate::i18n::text("review.106")).clicked() {
                        self.recalculate(db);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("📋 Copy Stream (Ch. 11.9)").clicked() {
                        self.copy_selected_stream();
                        ui.close_menu();
                    }
                    if ui
                        .button("📌 Paste Stream (Linked ='OUTx'!Dy)")
                        .clicked()
                    {
                        self.paste_linked_stream(db);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.copy_mode_whole_row, "Copy Mode (Whole Row - Ch. 11.1)");
                });

                ui.menu_button(crate::i18n::text("common.039"), |ui| {
                    if ui.button("📊 BALANCE Sheet").clicked() {
                        self.active_tab = ActiveSheetTab::Balance;
                        ui.close_menu();
                    }
                    if ui.button("📥 Active IN Sheet").clicked() {
                        self.active_tab = ActiveSheetTab::In(self.active_area);
                        ui.close_menu();
                    }
                    if ui.button("📤 Active OUT Sheet").clicked() {
                        self.active_tab = ActiveSheetTab::Out(self.active_area);
                        ui.close_menu();
                    }
                    if ui.button("🎯 Target Sheet").clicked() {
                        self.active_tab = ActiveSheetTab::Target;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.show_tools_sidebar, "🔧 Show Right Tools Sidebar");
                });

                ui.menu_button(crate::i18n::text("common.040"), |ui| {
                    if ui.button("➕ Balance Area (INx + OUTx)").clicked() {
                        let n = self.wb.areas.len() + 1;
                        self.wb.areas.push(BalanceArea {
                            name: format!("Balance Area {n}"),
                            in_streams: vec![StreamGroup::new("STREAM 1:", 25.0, vec![])],
                            extra_heat: 0.0,
                            out_streams: vec![StreamGroup::new("STREAM 1:", 25.0, vec![])],
                            heat_losses: 0.0,
                        });
                        self.recalculate(db);
                        ui.close_menu();
                    }
                    if ui.button("🌊 Stream (Substance Group)").clicked() {
                        self.insert_stream_in_active(db);
                        ui.close_menu();
                    }
                    if ui.button("🔬 Row (Species from Database)").clicked() {
                        self.add_to_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
                        self.show_add_species = true;
                        ui.close_menu();
                    }
                    if ui.button("🔥 Heat Flow / Air-Cooled Loss (Eq. 1)...").clicked() {
                        self.show_heat_flow_modal = true;
                        ui.close_menu();
                    }
                    if ui.button("📄 Sheet (Custom Excel Worksheet)").clicked() {
                        let idx = self.wb.user_sheets.len() + 1;
                        self.wb.user_sheets.push(BalanceUserSheet {
                            name: format!("Sheet{idx}"),
                            ..Default::default()
                        });
                        self.active_tab = ActiveSheetTab::User(self.wb.user_sheets.len() - 1);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .checkbox(
                            &mut self.wb.h2o_pressure_corr,
                            "💧 Pressure correction H2O (Ch. 11.1 Step 16)",
                        )
                        .changed()
                    {
                        self.recalculate(db);
                    }
                });

                ui.menu_button(crate::i18n::text("common.041"), |ui| {
                    if ui.button(crate::i18n::text("review.109")).clicked() && self.wb.areas.len() > 1 {
                        self.wb.areas.remove(self.active_area);
                        self.active_area = self.active_area.min(self.wb.areas.len() - 1);
                        self.active_tab = ActiveSheetTab::In(self.active_area);
                        self.recalculate(db);
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.110")).clicked() {
                        self.delete_selected_stream(db);
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.111")).clicked() {
                        self.delete_selected_row(db);
                        ui.close_menu();
                    }
                    if let ActiveSheetTab::User(u_idx) = self.active_tab {
                        if ui.button(crate::i18n::text("review.112")).clicked()
                            && u_idx < self.wb.user_sheets.len()
                        {
                            self.wb.user_sheets.remove(u_idx);
                            self.active_tab = ActiveSheetTab::In(self.active_area);
                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button(crate::i18n::text("common.042"), |ui| {
                    ui.checkbox(
                        &mut self.wb.red_font_shield,
                        "🛡 Red Font Shield (Ch. 11.6 & 11.10)",
                    );
                });

                ui.menu_button(crate::i18n::text("review.113"), |ui| {
                    ui.label(egui::RichText::new(crate::i18n::text("common.007")).strong());
                    for u in [TemperatureUnit::Celsius, TemperatureUnit::Kelvin] {
                        let sel = self.wb.temp_unit == u;
                        if ui.radio(sel, u.label()).clicked() && !sel {
                            SoryntelApplication::convert_balance_temperature_unit(&mut self.wb, u);
                            self.recalculate(db);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(crate::i18n::text("common.043")).strong());
                    for eu in [
                        BalanceEnergyUnit::Mcal,
                        BalanceEnergyUnit::MJ,
                        BalanceEnergyUnit::Kwh,
                    ] {
                        let sel = self.wb.energy_unit == eu;
                        if ui.radio(sel, eu.label()).clicked() && !sel {
                            SoryntelApplication::convert_balance_energy_unit(&mut self.wb, eu);
                            self.recalculate(db);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui
                        .checkbox(&mut self.wb.h2o_pressure_corr, "💧 Pressure correction H2O")
                        .changed()
                    {
                        self.recalculate(db);
                    }
                });

                ui.menu_button(crate::i18n::text("common.008"), |ui| {
                    if ui.button(crate::i18n::text("review.114")).clicked() {
                        self.recalculate(db);
                        ui.close_menu();
                    }
                    if ui.button("🔥 Temperature Balance (Adiabatic)...").clicked() {
                        self.trigger_temp_balance(db);
                        ui.close_menu();
                    }
                    if ui.button("📊 Stream Compositions (In-% / Out-%)").clicked() {
                        self.active_tab = ActiveSheetTab::OutPct(self.active_area);
                        ui.close_menu();
                    }
                    if ui.button(crate::i18n::text("review.115")).clicked() {
                        self.active_tab = ActiveSheetTab::Balance;
                        ui.close_menu();
                    }
                });

                if ui.button(crate::i18n::text("common.044")).clicked() {
                    self.show_target_dialog = true;
                }
                if ui.button(crate::i18n::text("common.012")).clicked() {
                    self.show_diagram_settings = true;
                }
                if ui.button(crate::i18n::text("common.022")).clicked() {
                    self.show_help_modal = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("🔥 CH4_Cooling")
                        .on_hover_text("Quemador CH4(g) + Agua de Enfriamiento (Diagram 1..12 Nm³)")
                        .clicked()
                    {
                        self.load_ch4_cooling_preset(db);
                    }
                    if ui.small_button("⛓ FSF_process").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_fsf_process(), db);
                    }
                    if ui.small_button("💧 H2O").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_h2o(), db);
                    }
                    if ui.small_button("🏭 CUSMELT").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_cusmelt(), db);
                    }
                    if ui.small_button("🔥 BUTANE").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_butane(), db);
                    }
                    if ui.small_button("📘 CUCONV1").clicked() {
                        self.load_preset(SoryntelApplication::balance_preset_cuconv1(), db);
                    }
                    ui.label(egui::RichText::new(crate::i18n::text("common.045")).small());
                });
            });

            // Quick Action Icon Toolbar + Excel Formula Bar (Figs. 1, 4, 11, 12)
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .small_button(crate::i18n::text("common.009"))
                    .on_hover_text("Abrir archivo .bal8")
                    .clicked()
                {
                    self.file_modal_is_save = false;
                    self.show_file_modal = true;
                }
                if ui
                    .small_button("💾 Save .bal8")
                    .on_hover_text("Guardar workbook en .bal8")
                    .clicked()
                {
                    self.save_bal8(DEFAULT_BAL8_PATH);
                }
                if ui
                    .small_button("📊 Excel .xlsx")
                    .on_hover_text("Exportar todas las hojas a MS Excel (.xlsx)")
                    .clicked()
                {
                    self.export_xlsx(DEFAULT_BAL_XLSX_PATH);
                }
                ui.separator();
                if ui
                    .small_button("🔄 ReCalc")
                    .on_hover_text("Recalcular todas las áreas y referencias circulares")
                    .clicked()
                {
                    self.recalculate(db);
                }
                if ui
                    .small_button("🔥 Temp. Balance")
                    .on_hover_text("Calcular temperatura adiabática de productos (ΔH = 0)")
                    .clicked()
                {
                    self.trigger_temp_balance(db);
                }
                if ui
                    .small_button("🎯 Iterate (F8)")
                    .on_hover_text("Iterar filas activas de Target (F8)")
                    .clicked()
                {
                    self.run_selected_targets(db, false);
                }
                if ui
                    .small_button(crate::i18n::text("common.012"))
                    .on_hover_text("Abrir configuración de Diagrama paramétrico")
                    .clicked()
                {
                    self.show_diagram_settings = true;
                }
                if ui
                    .small_button("🔥 CH4_Cooling (Diagram)")
                    .on_hover_text("Cargar ejemplo CH4 Burner + Cooling Water y abrir gráfica Diagram (1..12 Nm³)")
                    .clicked()
                {
                    self.load_ch4_cooling_preset(db);
                }
                ui.separator();
                if ui
                    .small_button(crate::i18n::text("review.107"))
                    .on_hover_text("Copiar corriente seleccionada")
                    .clicked()
                {
                    self.copy_selected_stream();
                }
                if ui
                    .small_button(crate::i18n::text("review.108"))
                    .on_hover_text("Pegar corriente con fórmulas vinculadas ='OUTx'!Dy")
                    .clicked()
                {
                    self.paste_linked_stream(db);
                }
                if ui
                    .selectable_label(self.wb.red_font_shield, "🛡 Red Shield")
                    .on_hover_text("Proteger celdas: solo permitir edición en celdas de entrada rojas")
                    .clicked()
                {
                    self.wb.red_font_shield = !self.wb.red_font_shield;
                }
                if ui
                    .selectable_label(self.show_tools_sidebar, "🔧 Tools")
                    .on_hover_text("Mostrar/ocultar panel lateral derecho de herramientas")
                    .clicked()
                {
                    self.show_tools_sidebar = !self.show_tools_sidebar;
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 186, 196)))
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.set_min_width(86.0);
                        ui.monospace(&self.formula_bar_ref);
                    });
                ui.label(
                    egui::RichText::new("fx")
                        .strong()
                        .italics()
                        .color(egui::Color32::from_rgb(71, 85, 105)),
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.formula_bar_text)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.apply_formula_bar_edit(db);
                }
            });
        });

        // 2. Bottom Status Bar (with Outotec/Soryntel badge matching Fig. 1)
        egui::TopBottomPanel::bottom("bal_status_bar").show(ctx, |ui| {
            let color = if self.status_ok {
                egui::Color32::from_rgb(21, 128, 61)
            } else {
                egui::Color32::from_rgb(185, 28, 28)
            };
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Soryntel")
                        .strong()
                        .color(egui::Color32::from_rgb(185, 28, 28)),
                );
                ui.separator();
                ui.strong(&self.wb.title);
                ui.separator();
                ui.colored_label(color, &self.status_msg);
            });
        });

        // 3. Right Tools Sidebar (Figs. 1, 3, 10-14 — Responsive & Collapsible)
        let screen_w = ctx.screen_rect().width();
        if self.show_tools_sidebar && screen_w >= 680.0 {
            egui::SidePanel::right("bal_tools_panel")
                .resizable(true)
                .default_width(232.0)
                .min_width(185.0)
                .max_width(400.0)
                .show(ctx, |ui| {
                    self.render_tools_sidebar(ui, db);
                });
        }

        // 4. Bottom Docked Panels: Element Balance (left) | Total Balance (right)
        egui::TopBottomPanel::bottom("bal_bottom_panels")
            .resizable(true)
            .default_height(172.0)
            .min_height(105.0)
            .max_height(400.0)
            .show(ctx, |ui| {
                if ui.available_width() >= 760.0 {
                    ui.columns(2, |cols| {
                        self.render_element_balance_panel(&mut cols[0]);
                        self.render_total_balance_panel(&mut cols[1]);
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("bal_bottom_compact_scroll")
                        .show(ui, |ui| {
                            self.render_element_balance_panel(ui);
                            ui.add_space(4.0);
                            self.render_total_balance_panel(ui);
                        });
                }
            });

        // 5. Central Panel: Active Sheet Grid + Excel Bottom Sheet Tabs (Figs. 1, 3-6, 10-14)
        egui::CentralPanel::default().show(ctx, |ui| {
            let tab_bar_h = 30.0;
            let grid_h = (ui.available_height() - tab_bar_h - 6.0).max(110.0);

            egui::ScrollArea::both()
                .id_salt("bal_main_sheet_scroll")
                .max_height(grid_h)
                .show(ui, |ui| match self.active_tab.clone() {
                    ActiveSheetTab::Balance => self.render_balance_sheet(ui),
                    ActiveSheetTab::In(idx) => self.render_species_sheet(ui, idx, false, db),
                    ActiveSheetTab::Out(idx) => self.render_species_sheet(ui, idx, true, db),
                    ActiveSheetTab::InPct(idx) => self.render_pct_sheet(ui, idx, false),
                    ActiveSheetTab::OutPct(idx) => self.render_pct_sheet(ui, idx, true),
                    ActiveSheetTab::User(u_idx) => self.render_user_sheet(ui, u_idx, db),
                    ActiveSheetTab::Target => self.render_target_sheet_inline(ui, db),
                });

            ui.separator();
            self.render_bottom_sheet_tabs(ui, db);
        });

        // 6. Modals
        self.render_modals(ctx, db);
    }

    fn render_tools_sidebar(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::ScrollArea::both().show(ui, |ui| {
            ui.label(
                egui::RichText::new(crate::i18n::text("review.116"))
                    .strong()
                    .size(13.0)
                    .color(egui::Color32::from_rgb(30, 41, 59)),
            );
            ui.separator();

            // Balance Area selector
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(crate::i18n::text("review.117")).strong().small());
                ui.label(egui::RichText::new(crate::i18n::text("review.118")).small());
                let cur_name = self
                    .wb
                    .areas
                    .get(self.active_area)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("bal_area_combo")
                    .selected_text(cur_name)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for (i, a) in self.wb.areas.iter().enumerate() {
                            if ui
                                .selectable_value(&mut self.active_area, i, &a.name)
                                .clicked()
                            {
                                self.active_tab = ActiveSheetTab::In(i);
                            }
                        }
                    });
            });

            ui.add_space(4.0);

            // Get Species from Database
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(crate::i18n::text("review.119"))
                        .strong()
                        .small(),
                );
                ui.columns(2, |cols| {
                    if cols[0]
                        .add_sized(
                            [cols[0].available_width(), 24.0],
                            egui::Button::new("📥 Input"),
                        )
                        .clicked()
                    {
                        self.add_to_out = false;
                        self.show_add_species = true;
                    }
                    if cols[1]
                        .add_sized(
                            [cols[1].available_width(), 24.0],
                            egui::Button::new("📤 Output"),
                        )
                        .clicked()
                    {
                        self.add_to_out = true;
                        self.show_add_species = true;
                    }
                });
            });

            ui.add_space(4.0);

            // Insert / Delete (exact HSC button layout with semantic icons)
            ui.columns(2, |cols| {
                egui::Frame::group(cols[0].style()).show(&mut cols[0], |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(crate::i18n::text("common.010")).strong().small());
                    let bw = ui.available_width();
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("➕ Balance Area"))
                        .clicked()
                    {
                        let n = self.wb.areas.len() + 1;
                        self.wb.areas.push(BalanceArea {
                            name: format!("Balance Area {n}"),
                            in_streams: vec![StreamGroup::new("STREAM 1:", 25.0, vec![])],
                            extra_heat: 0.0,
                            out_streams: vec![StreamGroup::new("STREAM 1:", 25.0, vec![])],
                            heat_losses: 0.0,
                        });
                        self.recalculate(db);
                    }
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🌊 Stream"))
                        .clicked()
                    {
                        self.insert_stream_in_active(db);
                    }
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🔬 Row"))
                        .clicked()
                    {
                        self.add_to_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
                        self.show_add_species = true;
                    }
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🔥 Heat Flow"))
                        .clicked()
                    {
                        self.show_heat_flow_modal = true;
                    }
                });
                egui::Frame::group(cols[1].style()).show(&mut cols[1], |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(crate::i18n::text("common.013")).strong().small());
                    let bw = ui.available_width();
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🗑 Balance Area"))
                        .clicked()
                        && self.wb.areas.len() > 1
                    {
                        self.wb.areas.remove(self.active_area);
                        self.active_area = self.active_area.min(self.wb.areas.len() - 1);
                        self.active_tab = ActiveSheetTab::In(self.active_area);
                        self.recalculate(db);
                    }
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🗑 Selected Stream"))
                        .clicked()
                    {
                        self.delete_selected_stream(db);
                    }
                    if ui
                        .add_sized([bw, 22.0], egui::Button::new("🗑 Selected Row"))
                        .clicked()
                    {
                        self.delete_selected_row(db);
                    }
                });
            });

            ui.add_space(4.0);

            // Measure Units (with automatic numeric/formula conversion per Ch. 11.1 Step 3)
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(crate::i18n::text("review.120")).strong().small());
                let mut new_t_unit = None;
                let mut new_e_unit = None;
                ui.columns(2, |cols| {
                    cols[0].label(egui::RichText::new(crate::i18n::text("common.007")).small());
                    for u in [TemperatureUnit::Celsius, TemperatureUnit::Kelvin] {
                        if cols[0].radio(self.wb.temp_unit == u, u.label()).clicked()
                            && self.wb.temp_unit != u
                        {
                            new_t_unit = Some(u);
                        }
                    }

                    cols[1].label(egui::RichText::new(crate::i18n::text("common.043")).small());
                    for eu in [
                        BalanceEnergyUnit::Mcal,
                        BalanceEnergyUnit::MJ,
                        BalanceEnergyUnit::Kwh,
                    ] {
                        if cols[1]
                            .radio(self.wb.energy_unit == eu, eu.label())
                            .clicked()
                            && self.wb.energy_unit != eu
                        {
                            new_e_unit = Some(eu);
                        }
                    }
                });
                let mut changed = false;
                if let Some(u) = new_t_unit {
                    SoryntelApplication::convert_balance_temperature_unit(&mut self.wb, u);
                    changed = true;
                }
                if let Some(eu) = new_e_unit {
                    SoryntelApplication::convert_balance_energy_unit(&mut self.wb, eu);
                    changed = true;
                }
                changed |= ui
                    .checkbox(&mut self.wb.h2o_pressure_corr, "💧 H2O Press. Corr.")
                    .changed();
                if changed {
                    self.recalculate(db);
                }
            });

            ui.add_space(4.0);

            // Functions
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(crate::i18n::text("review.121")).strong().small());
                ui.columns(2, |cols| {
                    if cols[0]
                        .add_sized(
                            [cols[0].available_width(), 24.0],
                            egui::Button::new("🔥 Temp. Balance"),
                        )
                        .clicked()
                    {
                        self.trigger_temp_balance(db);
                    }
                    if cols[1]
                        .add_sized(
                            [cols[1].available_width(), 24.0],
                            egui::Button::new("📑 Summary"),
                        )
                        .clicked()
                    {
                        self.active_tab = ActiveSheetTab::OutPct(self.active_area);
                    }
                });
            });
        });
    }

    fn render_element_balance_panel(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("⚖ Element Balance");
            egui::ScrollArea::both()
                .id_salt("el_bal_scroll")
                .show(ui, |ui| {
                    if let Some(sum) = self.wb.summaries.get(self.active_area) {
                        let dark = egui::Color32::from_rgb(24, 24, 27);
                        let red = egui::Color32::from_rgb(185, 28, 28);
                        let w = [85.0, 92.0, 92.0, 92.0];
                        egui::Grid::new("el_bal_grid")
                            .spacing([0.0, 0.0])
                            .show(ui, |ui| {
                                section_cell(ui, "Element", w[0], true);
                                section_cell(ui, "IN", w[1], true);
                                section_cell(ui, "OUT", w[2], true);
                                section_cell(ui, "Balance", w[3], true);
                                ui.end_row();
                                section_cell(ui, "", w[0], false);
                                section_cell(ui, "kmol", w[1], true);
                                section_cell(ui, "kmol", w[2], true);
                                section_cell(ui, "kmol", w[3], true);
                                ui.end_row();
                                for (idx, el) in sum.elements.iter().enumerate() {
                                    let bg = if idx % 2 == 0 {
                                        egui::Color32::WHITE
                                    } else {
                                        EXCEL_ALT_ROW_BG
                                    };
                                    text_cell(ui, &el.element, w[0], bg, dark, false, true, false);
                                    text_cell(
                                        ui,
                                        &format!("{:.3}", el.in_kmol),
                                        w[1],
                                        bg,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        &format!("{:.3}", el.out_kmol),
                                        w[2],
                                        bg,
                                        dark,
                                        true,
                                        false,
                                        false,
                                    );
                                    let imbal = el.balance_kmol.abs() > 1e-3;
                                    text_cell(
                                        ui,
                                        &format!("{:.3}", el.balance_kmol),
                                        w[3],
                                        bg,
                                        if imbal { red } else { dark },
                                        true,
                                        imbal,
                                        false,
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                });
        });
    }

    fn render_total_balance_panel(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("∑ Total Balance");
            let e_lbl = self.wb.energy_unit.label();
            egui::ScrollArea::both()
                .id_salt("tot_bal_scroll")
                .max_height(132.0)
                .show(ui, |ui| {
                    let dark = egui::Color32::from_rgb(24, 24, 27);
                    let w = [92.0, 68.0, 68.0, 68.0, 92.0, 78.0, 78.0];
                    egui::Grid::new("tot_bal_grid")
                        .spacing([0.0, 0.0])
                        .show(ui, |ui| {
                            section_cell(ui, "BALANCE", w[0], true);
                            section_cell(ui, "Amount", w[1], true);
                            section_cell(ui, "Amount", w[2], true);
                            section_cell(ui, "Amount", w[3], true);
                            section_cell(ui, "Heat Content", w[4], true);
                            section_cell(ui, "Total H", w[5], true);
                            section_cell(ui, "Exergy", w[6], true);
                            ui.end_row();
                            section_cell(ui, "AREAS", w[0], true);
                            section_cell(ui, "kmol", w[1], true);
                            section_cell(ui, "kg", w[2], true);
                            section_cell(ui, "Nm3", w[3], true);
                            section_cell(ui, e_lbl, w[4], true);
                            section_cell(ui, e_lbl, w[5], true);
                            section_cell(ui, e_lbl, w[6], true);
                            ui.end_row();
                            for (i, sum) in self.wb.summaries.iter().enumerate() {
                                let bg = if i % 2 == 0 {
                                    egui::Color32::WHITE
                                } else {
                                    EXCEL_ALT_ROW_BG
                                };
                                text_cell(
                                    ui,
                                    &format!("Balance ({})", i + 1),
                                    w[0],
                                    EXCEL_SECTION_BG,
                                    dark,
                                    false,
                                    true,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.kmol),
                                    w[1],
                                    bg,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.kg),
                                    w[2],
                                    EXCEL_YELLOW_BG,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.nm3),
                                    w[3],
                                    bg,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.heat_content),
                                    w[4],
                                    bg,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.total_h),
                                    w[5],
                                    EXCEL_YELLOW_BG,
                                    dark,
                                    true,
                                    true,
                                    false,
                                );
                                text_cell(
                                    ui,
                                    &format!("{:.3}", sum.balance_totals.ex_tot),
                                    w[6],
                                    bg,
                                    dark,
                                    true,
                                    false,
                                    false,
                                );
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn render_bottom_sheet_tabs(&mut self, ui: &mut egui::Ui, _db: &ThermoDatabase) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(232, 236, 242))
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Navigation arrows `|< < > >|` matching Figs. 1, 3–6, 10–14
                    if ui.small_button("|<").on_hover_text("Ir a BALANCE").clicked() {
                        self.active_tab = ActiveSheetTab::Balance;
                    }
                    if ui.small_button("<").on_hover_text("Área anterior").clicked()
                        && self.active_area > 0
                    {
                        self.active_area -= 1;
                        self.active_tab = ActiveSheetTab::In(self.active_area);
                    }
                    if ui.small_button(">").on_hover_text("Área siguiente").clicked()
                        && self.active_area + 1 < self.wb.areas.len()
                    {
                        self.active_area += 1;
                        self.active_tab = ActiveSheetTab::In(self.active_area);
                    }
                    if ui.small_button(">|").on_hover_text("Ir a Target").clicked() {
                        self.active_tab = ActiveSheetTab::Target;
                    }
                    ui.separator();

                    let sheet_tab = |ui: &mut egui::Ui, active: bool, label: &str| -> bool {
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
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new(label)
                                    .size(11.5)
                                    .strong()
                                    .color(text_col),
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .rounding(3.0),
                        )
                        .clicked()
                    };

                    if sheet_tab(ui, self.active_tab == ActiveSheetTab::Balance, "📊 BALANCE") {
                        self.active_tab = ActiveSheetTab::Balance;
                    }
                    for i in 0..self.wb.areas.len() {
                        let n = i + 1;
                        if sheet_tab(
                            ui,
                            self.active_tab == ActiveSheetTab::In(i),
                            &format!("📥 IN{n}"),
                        ) {
                            self.active_area = i;
                            self.active_tab = ActiveSheetTab::In(i);
                        }
                        if sheet_tab(
                            ui,
                            self.active_tab == ActiveSheetTab::Out(i),
                            &format!("📤 OUT{n}"),
                        ) {
                            self.active_area = i;
                            self.active_tab = ActiveSheetTab::Out(i);
                        }
                    }
                    let n = self.active_area + 1;
                    if sheet_tab(
                        ui,
                        self.active_tab == ActiveSheetTab::InPct(self.active_area),
                        &format!("📈 In{n}-%"),
                    ) {
                        self.active_tab = ActiveSheetTab::InPct(self.active_area);
                    }
                    if sheet_tab(
                        ui,
                        self.active_tab == ActiveSheetTab::OutPct(self.active_area),
                        &format!("📈 Out{n}-%"),
                    ) {
                        self.active_tab = ActiveSheetTab::OutPct(self.active_area);
                    }
                    for (u_idx, us) in self.wb.user_sheets.iter().enumerate() {
                        if sheet_tab(
                            ui,
                            self.active_tab == ActiveSheetTab::User(u_idx),
                            &format!("📄 {}", us.name),
                        ) {
                            self.active_tab = ActiveSheetTab::User(u_idx);
                        }
                    }
                    if sheet_tab(ui, self.active_tab == ActiveSheetTab::Target, "🎯 Target") {
                        self.active_tab = ActiveSheetTab::Target;
                    }
                });
            });
    }

    fn render_balance_sheet(&self, ui: &mut egui::Ui) {
        let e_lbl = self.wb.energy_unit.label();
        let dark = egui::Color32::from_rgb(24, 24, 27);
        let green = egui::Color32::from_rgb(21, 128, 61);
        let w = [105.0, 88.0, 88.0, 88.0, 105.0, 95.0, 85.0, 85.0, 90.0, 90.0];
        egui::Grid::new("bal_summary_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                header_cell(ui, "", 32.0);
                for (c, &cw) in w.iter().enumerate() {
                    header_cell(ui, &col_letter(c), cw);
                }
                ui.end_row();

                row_num_cell(ui, 1, 32.0);
                for (c, h) in [
                    "BALANCE",
                    "Amount kmol",
                    "Amount kg",
                    "Amount Nm3",
                    &format!("Heat Content {e_lbl}"),
                    &format!("Total H {e_lbl}"),
                    &format!("T*S {e_lbl}"),
                    &format!("G {e_lbl}"),
                    &format!("Ex Phys {e_lbl}"),
                    &format!("Exergy {e_lbl}"),
                ]
                .iter()
                .enumerate()
                {
                    text_cell(ui, h, w[c], EXCEL_SECTION_BG, green, true, true, false);
                }
                ui.end_row();

                let mut r_num = 2usize;
                for (i, sum) in self.wb.summaries.iter().enumerate() {
                    let n = i + 1;
                    for (lbl, tot, bold) in [
                        (format!("IN{n}"), &sum.in_totals, false),
                        (format!("OUT{n}"), &sum.out_totals, false),
                        (format!("BALANCE"), &sum.balance_totals, true),
                    ] {
                        let bg = if bold {
                            EXCEL_YELLOW_BG
                        } else {
                            egui::Color32::WHITE
                        };
                        row_num_cell(ui, r_num, 32.0);
                        text_cell(
                            ui,
                            &lbl,
                            w[0],
                            if bold { EXCEL_SECTION_BG } else { bg },
                            dark,
                            false,
                            bold,
                            false,
                        );
                        text_cell(ui, &format!("{:.3}", tot.kmol), w[1], bg, dark, true, bold, false);
                        text_cell(ui, &format!("{:.3}", tot.kg), w[2], bg, dark, true, bold, false);
                        text_cell(ui, &format!("{:.3}", tot.nm3), w[3], bg, dark, true, bold, false);
                        text_cell(
                            ui,
                            &format!("{:.2}", tot.heat_content),
                            w[4],
                            bg,
                            dark,
                            true,
                            bold,
                            false,
                        );
                        text_cell(ui, &format!("{:.2}", tot.total_h), w[5], bg, dark, true, bold, false);
                        text_cell(ui, &format!("{:.3}", tot.ts), w[6], bg, dark, true, bold, false);
                        text_cell(ui, &format!("{:.3}", tot.g), w[7], bg, dark, true, bold, false);
                        text_cell(
                            ui,
                            &format!("{:.3}", tot.ex_phys),
                            w[8],
                            bg,
                            dark,
                            true,
                            bold,
                            false,
                        );
                        text_cell(ui, &format!("{:.3}", tot.ex_tot), w[9], bg, dark, true, bold, false);
                        ui.end_row();
                        r_num += 1;
                    }
                    if i + 1 < self.wb.summaries.len() {
                        row_num_cell(ui, r_num, 32.0);
                        for cw in &w {
                            text_cell(ui, "", *cw, egui::Color32::WHITE, dark, false, false, false);
                        }
                        ui.end_row();
                        r_num += 1;
                    }
                }
            });
    }

    fn render_species_sheet(
        &mut self,
        ui: &mut egui::Ui,
        area_idx: usize,
        is_out: bool,
        db: &ThermoDatabase,
    ) {
        let t_lbl = self.wb.temp_unit.label();
        let e_lbl = self.wb.energy_unit.label();
        let h2o_corr = self.wb.h2o_pressure_corr;
        let mut need_recalc = false;
        let mut clicked_cell: Option<(String, String, usize, Option<usize>)> = None;

        let Some(area) = self.wb.areas.get_mut(area_idx) else {
            return;
        };
        let streams = if is_out {
            &mut area.out_streams
        } else {
            &mut area.in_streams
        };
        let header_title = if is_out {
            format!("OUTPUT SPECIES ({})", area_idx + 1)
        } else {
            format!("INPUT SPECIES ({})", area_idx + 1)
        };
        let sheet_prefix = if is_out {
            format!("OUT{}", area_idx + 1)
        } else {
            format!("IN{}", area_idx + 1)
        };

        let w = [138.0, 82.0, 78.0, 88.0, 88.0, 86.0, 95.0, 90.0, 95.0, 95.0, 72.0];
        let dark = egui::Color32::from_rgb(24, 24, 27);
        let blue_stream = egui::Color32::from_rgb(24, 48, 120);

        egui::Grid::new("bal_species_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                // Row 0: Excel Column Letters (A..K)
                header_cell(ui, "", 32.0);
                for (c, &cw) in w.iter().enumerate() {
                    header_cell(ui, &col_letter(c), cw);
                }
                ui.end_row();

                // Row 1: Column Headers
                row_num_cell(ui, 1, 32.0);
                section_cell(ui, &header_title, w[0], true);
                section_cell(ui, &format!("Temper. {t_lbl}"), w[1], true);
                section_cell(ui, "Pressure bar", w[2], true);
                section_cell(ui, "Amount kmol", w[3], true);
                section_cell(ui, "Amount kg", w[4], true);
                section_cell(ui, "Amount Nm³", w[5], true);
                section_cell(ui, &format!("Heat Content {e_lbl}"), w[6], true);
                section_cell(ui, &format!("Total H {e_lbl}"), w[7], true);
                section_cell(ui, &format!("Heat Cont {e_lbl}/kmol"), w[8], true);
                section_cell(ui, &format!("Tot H {e_lbl}/kmol"), w[9], true);
                section_cell(ui, "MW", w[10], true);
                ui.end_row();

                let mut row_num = 2usize;
                for (s_idx, st) in streams.iter_mut().enumerate() {
                    let st_sel = self.selected_stream_idx == Some(s_idx)
                        && self.selected_row_idx.is_none();
                    row_num_cell(ui, row_num, 32.0);
                    if text_cell(
                        ui,
                        &st.name,
                        w[0],
                        EXCEL_SECTION_BG,
                        blue_stream,
                        false,
                        true,
                        st_sel,
                    )
                    .clicked()
                    {
                        clicked_cell = Some((
                            format!("{sheet_prefix}!A{row_num}"),
                            st.name.clone(),
                            s_idx,
                            None,
                        ));
                    }

                    // Stream Temperature (affects all species in stream per Ch. 11.3 Step 2)
                    widget_cell(ui, w[1], EXCEL_SECTION_BG, false, |ui| {
                        let mut t_stream = st.stream_temp_display;
                        if ui
                            .add(egui::DragValue::new(&mut t_stream).speed(1.0).fixed_decimals(3))
                            .changed()
                        {
                            st.stream_temp_display = t_stream;
                            for sp in &mut st.species {
                                sp.temp_display = t_stream;
                            }
                            need_recalc = true;
                        }
                    });
                    // Stream Pressure bar (editable on Stream row: updates all species in stream)
                    widget_cell(ui, w[2], EXCEL_SECTION_BG, false, |ui| {
                        let mut p_stream = st
                            .species
                            .first()
                            .map(|sp| sp.pressure_bar)
                            .unwrap_or(1.0);
                        if ui
                            .add(
                                egui::DragValue::new(&mut p_stream)
                                    .speed(0.05)
                                    .range(0.001..=1000.0)
                                    .fixed_decimals(3),
                            )
                            .on_hover_text("Editar presión (bar) de todo el grupo de corriente")
                            .changed()
                        {
                            for sp in &mut st.species {
                                sp.pressure_bar = p_stream;
                            }
                            need_recalc = true;
                        }
                    });

                    // Stream Total kmol (editable per Ch. 11.3 Step 3: scales species proportionally)
                    widget_cell(ui, w[3], EXCEL_SECTION_BG, false, |ui| {
                        let mut s_kmol = st.sum_kmol;
                        if ui
                            .add(
                                egui::DragValue::new(&mut s_kmol)
                                    .speed(0.05)
                                    .range(0.0..=1e9)
                                    .fixed_decimals(3),
                            )
                            .on_hover_text("Editar total kmol del grupo (escala especies proporcionalmente)")
                            .changed()
                        {
                            SoryntelApplication::scale_balance_stream_kmol(st, s_kmol);
                            need_recalc = true;
                        }
                    });

                    // Stream Total kg (editable per Ch. 11.3 Step 3)
                    widget_cell(ui, w[4], EXCEL_SECTION_BG, false, |ui| {
                        let mut s_kg = st.sum_kg;
                        if ui
                            .add(
                                egui::DragValue::new(&mut s_kg)
                                    .speed(1.0)
                                    .range(0.0..=1e12)
                                    .fixed_decimals(3),
                            )
                            .on_hover_text("Editar total kg del grupo (escala especies proporcionalmente)")
                            .changed()
                        {
                            SoryntelApplication::scale_balance_stream_kg(st, s_kg);
                            need_recalc = true;
                        }
                    });

                    // Stream Total Nm3 (editable per Ch. 11.3 Step 3)
                    widget_cell(ui, w[5], EXCEL_SECTION_BG, false, |ui| {
                        let mut s_nm3 = st.sum_nm3;
                        if ui
                            .add(
                                egui::DragValue::new(&mut s_nm3)
                                    .speed(0.5)
                                    .range(0.0..=1e12)
                                    .fixed_decimals(3),
                            )
                            .on_hover_text("Editar total Nm³ del grupo (escala especies proporcionalmente)")
                            .changed()
                        {
                            SoryntelApplication::scale_balance_stream_nm3(st, s_nm3);
                            need_recalc = true;
                        }
                    });

                    text_cell(
                        ui,
                        &format!("{:.2}", st.sum_heat_content),
                        w[6],
                        EXCEL_SECTION_BG,
                        dark,
                        true,
                        true,
                        false,
                    );
                    text_cell(
                        ui,
                        &format!("{:.2}", st.sum_total_h),
                        w[7],
                        EXCEL_SECTION_BG,
                        dark,
                        true,
                        true,
                        false,
                    );
                    text_cell(ui, "", w[8], EXCEL_READONLY_BG, dark, true, false, false);
                    text_cell(ui, "", w[9], EXCEL_READONLY_BG, dark, true, false, false);
                    text_cell(ui, "", w[10], EXCEL_READONLY_BG, dark, true, false, false);
                    ui.end_row();
                    row_num += 1;

                    // Species rows
                    for (r_idx, sp) in st.species.iter_mut().enumerate() {
                        let is_sel = self.selected_stream_idx == Some(s_idx)
                            && self.selected_row_idx == Some(r_idx);
                        let sp_color = if sp.missing_in_db {
                            egui::Color32::RED
                        } else {
                            dark
                        };
                        row_num_cell(ui, row_num, 32.0);
                        if text_cell(
                            ui,
                            &sp.formula,
                            w[0],
                            egui::Color32::WHITE,
                            sp_color,
                            false,
                            false,
                            is_sel,
                        )
                        .clicked()
                        {
                            let fx = sp
                                .amount_formula
                                .clone()
                                .unwrap_or_else(|| format!("{:.3}", sp.amount_kmol));
                            clicked_cell = Some((
                                format!("{sheet_prefix}!D{row_num}"),
                                fx,
                                s_idx,
                                Some(r_idx),
                            ));
                        }

                        widget_cell(ui, w[1], egui::Color32::WHITE, false, |ui| {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut sp.temp_display)
                                        .speed(1.0)
                                        .fixed_decimals(3),
                                )
                                .changed()
                            {
                                need_recalc = true;
                            }
                        });

                        // Pressure bar column C: always editable for any species (0.001 .. 1000 bar)
                        let _ = h2o_corr;
                        widget_cell(ui, w[2], egui::Color32::WHITE, false, |ui| {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut sp.pressure_bar)
                                        .speed(0.05)
                                        .range(0.001..=1000.0)
                                        .fixed_decimals(3),
                                )
                                .on_hover_text("Presión de la especie (bar)")
                                .changed()
                            {
                                need_recalc = true;
                            }
                        });

                        // Amount kmol (Col D)
                        if let Some(ref f) = sp.amount_formula {
                            if text_cell(
                                ui,
                                &format!("{:.3}", sp.amount_kmol),
                                w[3],
                                EXCEL_YELLOW_BG,
                                dark,
                                true,
                                false,
                                is_sel,
                            )
                            .on_hover_text(f)
                            .clicked()
                            {
                                clicked_cell = Some((
                                    format!("{sheet_prefix}!D{row_num}"),
                                    f.clone(),
                                    s_idx,
                                    Some(r_idx),
                                ));
                            }
                        } else {
                            widget_cell(ui, w[3], egui::Color32::WHITE, is_sel, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut sp.amount_kmol)
                                            .speed(0.01)
                                            .range(0.0..=1e9)
                                            .fixed_decimals(3),
                                    )
                                    .changed()
                                {
                                    need_recalc = true;
                                }
                            });
                        }

                        // Amount kg (Col E — interactive per Ch. 11.1 Steps 1–2 & Ch. 11.3)
                        let mut kg_val = sp.amount_kg;
                        widget_cell(ui, w[4], egui::Color32::WHITE, false, |ui| {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut kg_val)
                                        .speed(0.5)
                                        .range(0.0..=1e12)
                                        .fixed_decimals(3),
                                )
                                .changed()
                            {
                                SoryntelApplication::set_balance_species_kg(sp, kg_val);
                                need_recalc = true;
                            }
                        });

                        // Amount Nm3 (Col F — interactive per Ch. 11.1 Steps 1–2 & Ch. 11.3)
                        let mut nm3_val = sp.amount_nm3;
                        widget_cell(ui, w[5], egui::Color32::WHITE, false, |ui| {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut nm3_val)
                                        .speed(0.1)
                                        .range(0.0..=1e12)
                                        .fixed_decimals(3),
                                )
                                .changed()
                            {
                                SoryntelApplication::set_balance_species_nm3(db, sp, nm3_val);
                                need_recalc = true;
                            }
                        });

                        text_cell(
                            ui,
                            &format!("{:.2}", sp.heat_content),
                            w[6],
                            EXCEL_READONLY_BG,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.2}", sp.total_h),
                            w[7],
                            EXCEL_READONLY_BG,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.3}", sp.hc_per_kmol),
                            w[8],
                            egui::Color32::WHITE,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.3}", sp.tot_h_per_kmol),
                            w[9],
                            egui::Color32::WHITE,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.3}", sp.mw),
                            w[10],
                            egui::Color32::WHITE,
                            dark,
                            true,
                            false,
                            false,
                        );
                        ui.end_row();
                        row_num += 1;
                    }
                }

                // EXTRA HEAT / HEAT LOSSES row in green (Figs. 1 & 3)
                let green = egui::Color32::from_rgb(21, 128, 61);
                row_num_cell(ui, row_num, 32.0);
                text_cell(
                    ui,
                    if is_out { "HEAT LOSSES:" } else { "EXTRA HEAT:" },
                    w[0],
                    egui::Color32::WHITE,
                    green,
                    false,
                    true,
                    false,
                );
                for cw in &w[1..7] {
                    text_cell(ui, "", *cw, EXCEL_READONLY_BG, dark, true, false, false);
                }
                widget_cell(ui, w[7], EXCEL_READONLY_BG, false, |ui| {
                    let val = if is_out {
                        &mut area.heat_losses
                    } else {
                        &mut area.extra_heat
                    };
                    if ui
                        .add(egui::DragValue::new(val).speed(0.5).fixed_decimals(2))
                        .changed()
                    {
                        need_recalc = true;
                    }
                });
                for cw in &w[8..] {
                    text_cell(ui, "", *cw, egui::Color32::WHITE, dark, true, false, false);
                }
                ui.end_row();
                row_num += 1;

                while row_num <= 16 {
                    row_num_cell(ui, row_num, 32.0);
                    for cw in &w {
                        text_cell(ui, "", *cw, egui::Color32::WHITE, dark, false, false, false);
                    }
                    ui.end_row();
                    row_num += 1;
                }
            });

        if let Some((cref, ftxt, s_idx, r_idx)) = clicked_cell {
            self.formula_bar_ref = cref;
            self.formula_bar_text = ftxt;
            self.selected_stream_idx = Some(s_idx);
            self.selected_row_idx = r_idx;
        }
        if need_recalc {
            self.recalculate(db);
        }
    }

    fn render_pct_sheet(&self, ui: &mut egui::Ui, area_idx: usize, is_out: bool) {
        let Some(area) = self.wb.areas.get(area_idx) else {
            return;
        };
        let comps = SoryntelApplication::compute_balance_stream_compositions(area, is_out);
        let t_lbl = self.wb.temp_unit.label();
        let title = if is_out {
            format!("OUT{}\nCompositions", area_idx + 1)
        } else {
            format!("IN{}\nCompositions", area_idx + 1)
        };

        let dark = egui::Color32::from_rgb(24, 24, 27);
        let w = [140.0, 90.0, 82.0, 95.0, 95.0, 60.0, 100.0, 100.0];
        egui::Grid::new("bal_pct_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                header_cell(ui, "", 32.0);
                for (c, &cw) in w.iter().enumerate() {
                    header_cell(ui, &col_letter(c), cw);
                }
                ui.end_row();

                row_num_cell(ui, 1, 32.0);
                section_cell(ui, &title.replace('\n', " "), w[0], true);
                section_cell(ui, &format!("Temper. {t_lbl}"), w[1], true);
                section_cell(ui, "Pressure bar", w[2], true);
                section_cell(ui, "Amount kmol", w[3], true);
                section_cell(ui, "Amount kg", w[4], true);
                section_cell(ui, "", w[5], true);
                section_cell(ui, "Content mol-%", w[6], true);
                section_cell(ui, "Content wt-%", w[7], true);
                ui.end_row();

                let mut r_num = 2usize;
                for sc in comps {
                    row_num_cell(ui, r_num, 32.0);
                    text_cell(
                        ui,
                        &sc.stream_name,
                        w[0],
                        EXCEL_SECTION_BG,
                        dark,
                        false,
                        true,
                        false,
                    );
                    text_cell(
                        ui,
                        &format!("{:.3}", sc.temp_display),
                        w[1],
                        EXCEL_SECTION_BG,
                        dark,
                        true,
                        true,
                        false,
                    );
                    text_cell(ui, "", w[2], EXCEL_SECTION_BG, dark, true, false, false);
                    text_cell(
                        ui,
                        &format!("{:.3}", sc.total_kmol),
                        w[3],
                        EXCEL_SECTION_BG,
                        dark,
                        true,
                        true,
                        false,
                    );
                    text_cell(
                        ui,
                        &format!("{:.3}", sc.total_kg),
                        w[4],
                        EXCEL_SECTION_BG,
                        dark,
                        true,
                        true,
                        false,
                    );
                    text_cell(ui, "", w[5], EXCEL_SECTION_BG, dark, true, false, false);
                    text_cell(ui, "100.00", w[6], EXCEL_SECTION_BG, dark, true, true, false);
                    text_cell(ui, "100.00", w[7], EXCEL_SECTION_BG, dark, true, true, false);
                    ui.end_row();
                    r_num += 1;

                    for (idx, el) in sc.elements.iter().enumerate() {
                        let bg = if idx % 2 == 0 {
                            egui::Color32::WHITE
                        } else {
                            EXCEL_ALT_ROW_BG
                        };
                        row_num_cell(ui, r_num, 32.0);
                        text_cell(ui, &el.element, w[0], bg, dark, false, false, false);
                        text_cell(ui, "", w[1], bg, dark, true, false, false);
                        text_cell(ui, "", w[2], bg, dark, true, false, false);
                        text_cell(
                            ui,
                            &format!("{:.3}", el.kmol),
                            w[3],
                            bg,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.3}", el.kg),
                            w[4],
                            bg,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(ui, "", w[5], bg, dark, true, false, false);
                        text_cell(
                            ui,
                            &format!("{:.3}", el.mol_pct),
                            w[6],
                            bg,
                            dark,
                            true,
                            false,
                            false,
                        );
                        text_cell(
                            ui,
                            &format!("{:.3}", el.wt_pct),
                            w[7],
                            bg,
                            dark,
                            true,
                            false,
                            false,
                        );
                        ui.end_row();
                        r_num += 1;
                    }
                }
            });
    }

    /// Render a custom user sheet (`FEED` Fig. 6, `Process flow` Fig. 13, `Analysis` Fig. 14) with Red Font Shield support.
    fn render_user_sheet(&mut self, ui: &mut egui::Ui, u_idx: usize, db: &ThermoDatabase) {
        let shield = self.wb.red_font_shield;
        let Some(us) = self.wb.user_sheets.get_mut(u_idx) else {
            return;
        };
        let sheet_name = us.name.clone();
        let dark = egui::Color32::from_rgb(24, 24, 27);
        let red_col = egui::Color32::from_rgb(185, 28, 28);
        let mut changed = false;
        let mut clicked_ref: Option<(usize, usize, String)> = None;

        egui::Grid::new(format!("bal_user_sheet_{u_idx}"))
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                header_cell(ui, "", 32.0);
                for c in 0..10 {
                    let cw = if c == 0 { 145.0 } else { 90.0 };
                    header_cell(ui, &col_letter(c), cw);
                }
                ui.end_row();

                for r in 0..18 {
                    row_num_cell(ui, r + 1, 32.0);
                    for c in 0..10 {
                        let cw = if c == 0 { 145.0 } else { 90.0 };
                        let is_red = us.red_cells.contains(&(r, c));
                        let sel = self.selected_user_cell == Some((r, c));
                        let val = us.cells.get(&(r, c)).cloned().unwrap_or_default();
                        let can_edit = !shield || is_red;
                        if is_red && can_edit {
                            let mut buf = val.clone();
                            widget_cell(ui, cw, egui::Color32::WHITE, sel, |ui| {
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut buf)
                                        .text_color(red_col)
                                        .desired_width(cw - 8.0),
                                );
                                if resp.changed() {
                                    us.cells.insert((r, c), buf);
                                    changed = true;
                                }
                                if resp.clicked() {
                                    clicked_ref = Some((r, c, val.clone()));
                                }
                            });
                        } else {
                            let col = if is_red { red_col } else { dark };
                            let bold = r == 0 || c == 0;
                            if text_cell(
                                ui,
                                &val,
                                cw,
                                egui::Color32::WHITE,
                                col,
                                val.parse::<f64>().is_ok(),
                                bold,
                                sel,
                            )
                            .clicked()
                            {
                                clicked_ref = Some((r, c, val));
                            }
                        }
                    }
                    ui.end_row();
                }
            });

        if let Some((r, c, val)) = clicked_ref {
            self.selected_user_cell = Some((r, c));
            self.formula_bar_ref = format!("{}!{}{}", sheet_name, col_letter(c), r + 1);
            self.formula_bar_text = val;
        }
        if changed {
            self.recalculate(db);
        }
    }

    /// Render the `Target` sheet inline in the CentralPanel matching Fig. 7.
    fn render_target_sheet_inline(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let salmon = egui::Color32::from_rgb(245, 200, 160);
        let dark = egui::Color32::from_rgb(24, 24, 27);
        let red_txt = egui::Color32::from_rgb(185, 28, 28);
        let w = [38.0, 130.0, 115.0, 88.0, 78.0, 78.0, 110.0, 105.0, 82.0, 82.0, 70.0];

        if self.selected_target_rows.len() != self.wb.target_rows.len() {
            self.selected_target_rows
                .resize(self.wb.target_rows.len(), true);
        }

        let mut run_sel = false;
        let mut run_all = false;

        ui.horizontal_wrapped(|ui| {
            if ui.button(crate::i18n::text("common.046")).clicked() {
                self.wb.target_rows.push(TargetRow {
                    name: "Var".into(),
                    var_cell_ref: "IN1!D11".into(),
                    test_value: 0.0,
                    min_val: 0.0,
                    max_val: 100.0,
                    target_name: "HEAT BAL".into(),
                    target_cell_ref: "BALANCE1!H4".into(),
                    current_target_val: 0.0,
                    goal_value: 0.0,
                    max_iterations: 10,
                });
                self.selected_target_rows.push(true);
            }
            if ui.button(crate::i18n::text("common.047")).clicked() && !self.wb.target_rows.is_empty() {
                self.wb.target_rows.pop();
                self.selected_target_rows.pop();
            }
            ui.separator();
            if ui
                .button(crate::i18n::text("review.122"))
                .on_hover_text("Usar la celda activa de la barra de fórmulas como Variable")
                .clicked()
            {
                if let Some(last) = self.wb.target_rows.last_mut() {
                    last.var_cell_ref = self.formula_bar_ref.clone();
                }
            }
            if ui
                .button(crate::i18n::text("review.123"))
                .on_hover_text("Usar BALANCE1!H4 o la celda activa como Target")
                .clicked()
            {
                if let Some(last) = self.wb.target_rows.last_mut() {
                    last.target_cell_ref = format!("BALANCE{}!H4", self.active_area + 1);
                }
            }
            ui.separator();
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(crate::i18n::text("review.124")).strong())
                        .fill(egui::Color32::from_rgb(219, 234, 254)),
                )
                .clicked()
            {
                run_sel = true;
            }
            if ui.button(crate::i18n::text("common.048")).clicked() {
                run_all = true;
            }
        });
        ui.add_space(4.0);

        egui::Grid::new("bal_target_inline_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                row_num_cell(ui, 1, 32.0);
                text_cell(
                    ui,
                    "Target Calculation Sheet (Goal seek)",
                    w[0] + w[1],
                    egui::Color32::WHITE,
                    dark,
                    false,
                    true,
                    false,
                );
                for &cw in &w[2..] {
                    text_cell(ui, "", cw, egui::Color32::WHITE, dark, false, false, false);
                }
                ui.end_row();

                row_num_cell(ui, 2, 32.0);
                section_cell(ui, "Sel", w[0], true);
                text_cell(ui, "Variable", w[1], salmon, dark, false, true, false);
                for &cw in &w[2..6] {
                    text_cell(ui, "", cw, salmon, dark, false, false, false);
                }
                text_cell(ui, "Target", w[6], salmon, dark, false, true, false);
                text_cell(ui, "", w[7], salmon, dark, false, false, false);
                text_cell(ui, "", w[8], salmon, dark, false, false, false);
                text_cell(ui, "", w[9], salmon, dark, false, false, false);
                text_cell(ui, "Iteration", w[10], salmon, dark, true, true, false);
                ui.end_row();

                row_num_cell(ui, 3, 32.0);
                for (c, lbl) in [
                    "",
                    "Name",
                    "Cell Ref.",
                    "Test value",
                    "Min",
                    "Max",
                    "Name",
                    "Cell Ref.",
                    "Current",
                    "Value",
                    "Max",
                ]
                .iter()
                .enumerate()
                {
                    text_cell(ui, lbl, w[c], salmon, dark, c >= 3 && c != 6, true, false);
                }
                ui.end_row();

                for (i, row) in self.wb.target_rows.iter_mut().enumerate() {
                    row_num_cell(ui, i + 4, 32.0);
                    widget_cell(ui, w[0], egui::Color32::WHITE, false, |ui| {
                        ui.checkbox(&mut self.selected_target_rows[i], "");
                    });
                    widget_cell(ui, w[1], salmon, false, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut row.name).desired_width(w[1] - 8.0));
                    });
                    widget_cell(ui, w[2], egui::Color32::WHITE, false, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut row.var_cell_ref)
                                .desired_width(w[2] - 8.0),
                        );
                    });
                    text_cell(
                        ui,
                        &format!("{:.3}", row.test_value),
                        w[3],
                        egui::Color32::WHITE,
                        red_txt,
                        true,
                        true,
                        false,
                    );
                    widget_cell(ui, w[4], egui::Color32::WHITE, false, |ui| {
                        ui.add(egui::DragValue::new(&mut row.min_val).speed(0.5));
                    });
                    widget_cell(ui, w[5], egui::Color32::WHITE, false, |ui| {
                        ui.add(egui::DragValue::new(&mut row.max_val).speed(0.5));
                    });
                    widget_cell(ui, w[6], salmon, false, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut row.target_name)
                                .desired_width(w[6] - 8.0),
                        );
                    });
                    widget_cell(ui, w[7], egui::Color32::WHITE, false, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut row.target_cell_ref)
                                .desired_width(w[7] - 8.0),
                        );
                    });
                    text_cell(
                        ui,
                        &format!("{:.3}", row.current_target_val),
                        w[8],
                        egui::Color32::WHITE,
                        dark,
                        true,
                        false,
                        false,
                    );
                    widget_cell(ui, w[9], EXCEL_YELLOW_BG, false, |ui| {
                        ui.add(egui::DragValue::new(&mut row.goal_value).speed(0.1));
                    });
                    widget_cell(ui, w[10], egui::Color32::WHITE, false, |ui| {
                        ui.add(egui::DragValue::new(&mut row.max_iterations).range(1..=100));
                    });
                    ui.end_row();
                }
            });

        if run_sel {
            self.run_selected_targets(db, false);
        }
        if run_all {
            self.run_selected_targets(db, true);
        }
    }

    fn render_modals(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        // 1. Temperature Balance Modal (Fig. 2)
        if let Some(t_adiab) = self.temp_bal_modal {
            let mut close = false;
            let mut apply = false;
            egui::Window::new(crate::i18n::text("review.125"))
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Temperature of products = {:.1} {}\n(when Heat Balance = 0)",
                            t_adiab,
                            self.wb.temp_unit.label()
                        ))
                        .size(13.5),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::text("review.126")).clicked() {
                            apply = true;
                            close = true;
                        }
                        if ui.button(crate::i18n::text("common.023")).clicked() {
                            close = true;
                        }
                    });
                });
            if apply {
                if let Some(area) = self.wb.areas.get_mut(self.active_area) {
                    for st in &mut area.out_streams {
                        st.stream_temp_display = t_adiab;
                        for sp in &mut st.species {
                            sp.temp_display = t_adiab;
                        }
                    }
                }
                self.recalculate(db);
            }
            if close {
                self.temp_bal_modal = None;
            }
        }

        // 2. Target Dialog Window (Fig. 7)
        if self.show_target_dialog {
            let mut open = self.show_target_dialog;
            egui::Window::new("🎯 Target Dialog — Target Calculation Sheet (Goal seek)")
                .open(&mut open)
                .default_size([880.0, 360.0])
                .show(ctx, |ui| {
                    self.render_target_sheet_inline(ui, db);
                });
            self.show_target_dialog = open;
        }

        // 3. Diagram Settings Modal (Figs. 8 & 9)
        if self.show_diagram_settings {
            let mut open = self.show_diagram_settings;
            let mut run_plot = false;
            egui::Window::new(crate::i18n::text("review.127"))
                .open(&mut open)
                .default_size([560.0, 320.0])
                .show(ctx, |ui| {
                    let cfg = &mut self.wb.diagram_config;
                    ui.strong("X-Axis Range:");
                    ui.horizontal(|ui| {
                        ui.label("MIN:");
                        ui.add(egui::DragValue::new(&mut cfg.x_min).speed(1.0));
                        ui.label("MAX:");
                        ui.add(egui::DragValue::new(&mut cfg.x_max).speed(1.0));
                        ui.label(crate::i18n::text("review.128"));
                        ui.add(
                            egui::DragValue::new(&mut cfg.x_step)
                                .speed(0.1)
                                .range(0.001..=10000.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("review.129"));
                        ui.text_edit_singleline(&mut cfg.title);
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("review.130"));
                        ui.text_edit_singleline(&mut cfg.x_cell_ref);
                        ui.label(crate::i18n::text("common.049"));
                        ui.text_edit_singleline(&mut cfg.x_label);
                    });
                    if let Some(y1) = cfg.y_series.first_mut() {
                        ui.horizontal(|ui| {
                            ui.label(crate::i18n::text("review.131"));
                            ui.text_edit_singleline(&mut y1.cell_ref);
                            ui.label(crate::i18n::text("common.049"));
                            ui.text_edit_singleline(&mut y1.label);
                            ui.label(crate::i18n::text("common.024"));
                            ui.text_edit_singleline(&mut y1.units);
                        });
                    }
                    ui.checkbox(
                        &mut cfg.use_target_iteration,
                        "Target iteration before every calculation step (Fig. 9)",
                    );
                    ui.separator();
                    ui.strong("Plot Axis Scale (X & Y Range):");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.diagram_auto_scale, "Auto Scale Axes");
                        if !self.diagram_auto_scale {
                            ui.label("X Min:");
                            ui.add(egui::DragValue::new(&mut self.diagram_x_min).speed(1.0));
                            ui.label("X Max:");
                            ui.add(egui::DragValue::new(&mut self.diagram_x_max).speed(1.0));
                            ui.label("Y Min:");
                            ui.add(egui::DragValue::new(&mut self.diagram_y_min).speed(10.0));
                            ui.label("Y Max:");
                            ui.add(egui::DragValue::new(&mut self.diagram_y_max).speed(10.0));
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("📌 Set X cell (from Formula Bar)").clicked() {
                            self.wb.diagram_config.x_cell_ref = self.formula_bar_ref.clone();
                        }
                        if ui.button(crate::i18n::text("common.012")).clicked() {
                            run_plot = true;
                        }
                    });
                });
            self.show_diagram_settings = open;
            if run_plot {
                match SoryntelApplication::run_balance_diagram_sweep(db, &self.wb) {
                    Ok(res) => {
                        if self.diagram_auto_scale {
                            self.diagram_x_min = self.wb.diagram_config.x_min;
                            self.diagram_x_max = self.wb.diagram_config.x_max;
                            if let Some((_, _, ys)) = res.series.first() {
                                let y_lo = ys.iter().copied().fold(f64::INFINITY, f64::min);
                                let y_hi = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                                if y_lo.is_finite() && y_hi.is_finite() {
                                    let pad = ((y_hi - y_lo).abs() * 0.08).max(1.0);
                                    self.diagram_y_min = (y_lo - pad).floor();
                                    self.diagram_y_max = (y_hi + pad).ceil();
                                }
                            }
                        }
                        self.diagram_apply_bounds = !self.diagram_auto_scale;
                        self.diagram_reset_counter = self.diagram_reset_counter.wrapping_add(1);
                        self.diagram_result = Some(res);
                        self.show_diagram_window = true;
                        self.status_msg = "Diagrama generado.".into();
                        self.status_ok = true;
                    }
                    Err(e) => {
                        self.status_msg = format!("Error generando diagrama: {e}");
                        self.status_ok = false;
                    }
                }
            }
        }

        // 4. Diagram Plot Window with Top Drawing Bar, X/Y Axis Range Controls & Left 5-Icon Strip (Figs. 8 & 9)
        if self.show_diagram_window {
            let mut open = self.show_diagram_window;
            let mut rerun_sweep_inside = false;
            if let Some(res) = self.diagram_result.clone() {
                egui::Window::new(format!("📈 Diagram — {}", res.title))
                    .open(&mut open)
                    .default_size([720.0, 500.0])
                    .min_size([380.0, 240.0])
                    .resizable(true)
                    .show(ctx, |ui| {
                        // Top Drawing Toolbar matching Figs. 8 & 9
                        ui.horizontal_wrapped(|ui| {
                            for lbl in ["↖", "╱", "↘", "▭", "◯", "A", "🎨"] {
                                let _ = ui.small_button(lbl);
                            }
                            ui.separator();
                            ui.strong(&res.title);
                            ui.separator();
                            if ui.small_button(crate::i18n::text("review.132")).clicked() {
                                self.show_diagram_settings = true;
                            }
                        });
                        // Interactive Sweep X-Range & Plot X/Y Axis Range Bar
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("Sweep X:");
                            ui.label("Min");
                            ui.add(
                                egui::DragValue::new(&mut self.wb.diagram_config.x_min)
                                    .speed(0.5),
                            );
                            ui.label("Max");
                            ui.add(
                                egui::DragValue::new(&mut self.wb.diagram_config.x_max)
                                    .speed(0.5),
                            );
                            ui.label(crate::i18n::text("review.133"));
                            ui.add(
                                egui::DragValue::new(&mut self.wb.diagram_config.x_step)
                                    .speed(0.1)
                                    .range(0.001..=10000.0),
                            );
                            if ui
                                .small_button(crate::i18n::text("review.134"))
                                .on_hover_text("Recalcular barrido con el nuevo rango X")
                                .clicked()
                            {
                                rerun_sweep_inside = true;
                            }
                            ui.separator();
                            if ui
                                .checkbox(&mut self.diagram_auto_scale, "Auto Scale")
                                .changed()
                            {
                                self.diagram_apply_bounds = !self.diagram_auto_scale;
                                self.diagram_reset_counter =
                                    self.diagram_reset_counter.wrapping_add(1);
                            }
                            let mut bounds_edited = false;
                            ui.label("X:");
                            bounds_edited |= ui
                                .add(egui::DragValue::new(&mut self.diagram_x_min).speed(0.5))
                                .changed();
                            ui.label("..");
                            bounds_edited |= ui
                                .add(egui::DragValue::new(&mut self.diagram_x_max).speed(0.5))
                                .changed();
                            ui.label("Y:");
                            bounds_edited |= ui
                                .add(egui::DragValue::new(&mut self.diagram_y_min).speed(10.0))
                                .changed();
                            ui.label("..");
                            bounds_edited |= ui
                                .add(egui::DragValue::new(&mut self.diagram_y_max).speed(10.0))
                                .changed();
                            if bounds_edited
                                || ui
                                    .small_button(crate::i18n::text("common.050"))
                                    .on_hover_text("Aplicar límites de los ejes X e Y")
                                    .clicked()
                            {
                                self.diagram_auto_scale = false;
                                self.diagram_apply_bounds = true;
                                self.diagram_reset_counter =
                                    self.diagram_reset_counter.wrapping_add(1);
                            }
                        });
                        ui.separator();

                        ui.horizontal(|ui| {
                            // Left vertical 5-icon strip (Save, Copy, Print, Clone, Redraw — Figs. 8 & 9)
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(248, 250, 252))
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgb(214, 220, 229),
                                ))
                                .rounding(4.0)
                                .inner_margin(egui::Margin::symmetric(4.0, 6.0))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        if ui
                                            .button("💾")
                                            .on_hover_text("Exportar datos del diagrama")
                                            .clicked()
                                        {
                                            self.export_xlsx(DEFAULT_BAL_XLSX_PATH);
                                        }
                                        if ui
                                            .button("📋")
                                            .on_hover_text("Copiar serie al portapapeles")
                                            .clicked()
                                        {
                                            let mut tsv = format!("X ({})\tY\n", res.x_label);
                                            if let Some((_, _, ys)) = res.series.first() {
                                                for (&x, &y) in res.x_values.iter().zip(ys) {
                                                    tsv.push_str(&format!("{x:.3}\t{y:.3}\n"));
                                                }
                                            }
                                            ctx.copy_text(tsv);
                                        }
                                        if ui
                                            .button("🔄")
                                            .on_hover_text("Reescalar ejes")
                                            .clicked()
                                        {
                                            self.diagram_apply_bounds = !self.diagram_auto_scale;
                                            self.diagram_reset_counter =
                                                self.diagram_reset_counter.wrapping_add(1);
                                        }
                                    });
                                });

                            let y_axis_lbl = res
                                .series
                                .first()
                                .map(|(l, u, _)| format!("{l} {u}"))
                                .unwrap_or_default();
                            let mut plot = Plot::new(format!(
                                "balance_diagram_plot_{}",
                                self.diagram_reset_counter
                            ))
                            .width(ui.available_width().max(240.0))
                            .height(ui.available_height().max(160.0))
                            .x_axis_label(&res.x_label)
                            .y_axis_label(y_axis_lbl)
                            .legend(Legend::default());
                            if !self.diagram_auto_scale
                                && self.diagram_x_max > self.diagram_x_min
                                && self.diagram_y_max > self.diagram_y_min
                            {
                                plot = plot
                                    .include_x(self.diagram_x_min)
                                    .include_x(self.diagram_x_max)
                                    .include_y(self.diagram_y_min)
                                    .include_y(self.diagram_y_max);
                            }
                            if self.diagram_reset_counter > 0 && self.diagram_auto_scale {
                                plot = plot.reset();
                            }
                            let apply_bounds_now = self.diagram_apply_bounds
                                && !self.diagram_auto_scale
                                && self.diagram_x_max > self.diagram_x_min
                                && self.diagram_y_max > self.diagram_y_min;
                            let (bx0, bx1, by0, by1) = (
                                self.diagram_x_min,
                                self.diagram_x_max,
                                self.diagram_y_min,
                                self.diagram_y_max,
                            );
                            plot.show(ui, |plot_ui| {
                                if apply_bounds_now {
                                    plot_ui.set_plot_bounds(
                                        egui_plot::PlotBounds::from_min_max(
                                            [bx0, by0],
                                            [bx1, by1],
                                        ),
                                    );
                                }
                                let green_curve = egui::Color32::from_rgb(77, 124, 43);
                                for (lbl, units, ys) in &res.series {
                                    let raw: Vec<[f64; 2]> = res
                                        .x_values
                                        .iter()
                                        .zip(ys.iter())
                                        .map(|(&x, &y)| [x, y])
                                        .collect();
                                    plot_ui.line(
                                        Line::new(PlotPoints::from(raw.clone()))
                                            .name(if units.is_empty() {
                                                lbl.clone()
                                            } else {
                                                format!("{lbl} ({units})")
                                            })
                                            .color(green_curve)
                                            .width(2.5),
                                    );
                                    plot_ui.points(
                                        Points::new(PlotPoints::from(raw.clone()))
                                            .color(green_curve)
                                            .radius(3.0),
                                    );
                                    if let Some(&[x0, y0]) = raw.first() {
                                        plot_ui.text(
                                            PlotText::new(
                                                [x0, y0].into(),
                                                egui::RichText::new(lbl)
                                                    .strong()
                                                    .size(12.0)
                                                    .color(green_curve),
                                            )
                                            .anchor(egui::Align2::LEFT_BOTTOM),
                                        );
                                    }
                                }
                            });
                            if apply_bounds_now {
                                self.diagram_apply_bounds = false;
                            }
                        });
                    });
            }
            if rerun_sweep_inside {
                if let Ok(res) = SoryntelApplication::run_balance_diagram_sweep(db, &self.wb) {
                    if !self.diagram_auto_scale {
                        self.diagram_x_min = self.wb.diagram_config.x_min;
                        self.diagram_x_max = self.wb.diagram_config.x_max;
                        self.diagram_apply_bounds = true;
                    }
                    self.diagram_reset_counter = self.diagram_reset_counter.wrapping_add(1);
                    self.diagram_result = Some(res);
                }
            }
            self.show_diagram_window = open;
        }

        // 5. Heat Flow / Air-Cooled Reactor Loss Estimator Modal (Ch. 11.1 Step 4 Eq. 1)
        if self.show_heat_flow_modal {
            let mut open = self.show_heat_flow_modal;
            let e_unit = self.wb.energy_unit;
            let h_loss_est = SoryntelApplication::estimate_reactor_natural_convection_loss(
                self.hf_area_m2,
                self.hf_t_surface_c,
                self.hf_t_room_c,
                e_unit,
            );
            let mut apply_to_area = false;
            egui::Window::new("🔥 Heat Loss Estimator (Ch. 11.1 Eq. 1 — Air-Cooled Reactor)")
                .open(&mut open)
                .collapsible(false)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label("H_loss [kcal/h] = (6.8 + 0.046 * T2) * (T2 - T1) * A");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("review.135"));
                        ui.add(
                            egui::DragValue::new(&mut self.hf_area_m2)
                                .speed(0.5)
                                .range(0.0..=10000.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("review.136"));
                        ui.add(egui::DragValue::new(&mut self.hf_t_surface_c).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("review.137"));
                        ui.add(egui::DragValue::new(&mut self.hf_t_room_c).speed(1.0));
                    });
                    ui.separator();
                    ui.strong(format!(
                        "Estimated Heat Loss = {:.3} {}/h",
                        h_loss_est,
                        e_unit.label()
                    ));
                    ui.add_space(6.0);
                    if ui
                        .button(format!(
                            "✔ Inject into OUT{} HEAT LOSSES",
                            self.active_area + 1
                        ))
                        .clicked()
                    {
                        apply_to_area = true;
                    }
                });
            if apply_to_area {
                if let Some(area) = self.wb.areas.get_mut(self.active_area) {
                    area.heat_losses = h_loss_est;
                }
                self.recalculate(db);
                open = false;
            }
            self.show_heat_flow_modal = open;
        }

        // 6. File Save / Open Modal (.bal8)
        if self.show_file_modal {
            let mut open = self.show_file_modal;
            let title = if self.file_modal_is_save {
                "💾 Save Workbook As (.bal8)"
            } else {
                "📂 Open Workbook (.bal8)"
            };
            let mut do_save = false;
            let mut do_open = false;
            egui::Window::new(title)
                .open(&mut open)
                .collapsible(false)
                .default_width(440.0)
                .show(ctx, |ui| {
                    ui.label(crate::i18n::text("review.138"));
                    ui.text_edit_singleline(&mut self.file_path_buf);
                    ui.add_space(6.0);
                    if self.file_modal_is_save {
                        if ui.button(crate::i18n::text("common.014")).clicked() {
                            do_save = true;
                        }
                    } else if ui.button(crate::i18n::text("common.009")).clicked() {
                        do_open = true;
                    }
                });
            if do_save {
                let p = self.file_path_buf.clone();
                self.save_bal8(&p);
                open = false;
            }
            if do_open {
                let p = self.file_path_buf.clone();
                self.load_bal8(&p, db);
                open = false;
            }
            self.show_file_modal = open;
        }

        // 7. Add Species to Stream Modal
        if self.show_add_species {
            let mut open = self.show_add_species;
            let mut chosen: Option<String> = None;
            let target_lbl = if self.add_to_out { "Output" } else { "Input" };
            egui::Window::new(format!("🔬 Get Species from Database ({target_lbl})"))
                .open(&mut open)
                .default_size([480.0, 380.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Buscar especie:");
                        ui.text_edit_singleline(&mut self.add_species_query);
                        ui.label("kmol:");
                        ui.add(
                            egui::DragValue::new(&mut self.add_species_kmol)
                                .speed(0.1)
                                .range(0.0..=1e6),
                        );
                    });
                    ui.separator();
                    let list = db.search(&self.add_species_query, 80);
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        egui::Grid::new("bal_add_sp_grid")
                            .striped(true)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong("Formula");
                                ui.strong("Name");
                                ui.strong("MW");
                                ui.strong("Action");
                                ui.end_row();
                                for sp in list {
                                    ui.monospace(&sp.formula);
                                    ui.label(&sp.name);
                                    ui.monospace(format!("{:.3}", sp.molecular_weight));
                                    if ui.small_button(crate::i18n::text("review.139")).clicked() {
                                        chosen = Some(sp.formula.clone());
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                });
            self.show_add_species = open;
            if let Some(formula) = chosen {
                if let Some(area) = self.wb.areas.get_mut(self.active_area) {
                    let streams = if self.add_to_out {
                        &mut area.out_streams
                    } else {
                        &mut area.in_streams
                    };
                    if streams.is_empty() {
                        streams.push(StreamGroup::new("STREAM 1:", 25.0, vec![]));
                    }
                    let s_idx = self.selected_stream_idx.unwrap_or(0).min(streams.len() - 1);
                    let t_st = streams[s_idx].stream_temp_display;
                    streams[s_idx]
                        .species
                        .push(SpeciesRow::new(formula, t_st, self.add_species_kmol));
                }
                self.show_add_species = false;
                self.recalculate(db);
            }
        }

        // 8. Help Modal (Ch. 11)
        if self.show_help_modal {
            let mut open = self.show_help_modal;
            egui::Window::new("❓ Help — Heat and Material Balance Module (Chapter 11)")
                .open(&mut open)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("11. Heat and Material Balance Module");
                        ui.label("• Balance Areas (11.1, 11.9): Each Balance Area consists of an INx and OUTx sheet pair collected automatically into the BALANCE summary sheet.");
                        ui.label("• Streams & Amounts (11.3): Edit amounts in kmol, kg, or Nm³ either on individual species rows or directly on the Stream header row (which scales all species in the group proportionally).");
                        ui.label("• Formulae & Defined Element Names (11.4): Use Excel-type cell references (e.g. =0.938*IN1!D7 or ='OUT4'!D12) and auto-defined element names (InCu, OutFe, InCa2, etc.).");
                        ui.label("• Target Dialog & F8 (11.7): Specify variable cells and target cells (e.g. BALANCE1!H4 = 0) and press F8 or Iterate Selected Rows.");
                        ui.label("• Copy / Paste Stream (11.9): Copy an OUTx stream and Paste Stream into INy to create linked recycle/return streams.");
                    });
                });
            self.show_help_modal = open;
        }
    }

    fn trigger_temp_balance(&mut self, db: &ThermoDatabase) {
        match SoryntelApplication::solve_balance_adiabatic_temperature(
            db,
            &self.wb,
            self.active_area,
        ) {
            Ok(t) => {
                self.temp_bal_modal = Some(t);
            }
            Err(e) => {
                self.status_msg = format!("Error Temp. Balance: {e}");
                self.status_ok = false;
            }
        }
    }

    fn insert_stream_in_active(&mut self, db: &ThermoDatabase) {
        let is_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
        if let Some(area) = self.wb.areas.get_mut(self.active_area) {
            let streams = if is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            let n = streams.len() + 1;
            streams.push(StreamGroup::new(format!("STREAM {n}:"), 25.0, vec![]));
            self.selected_stream_idx = Some(streams.len() - 1);
            self.selected_row_idx = None;
        }
        self.recalculate(db);
    }

    fn delete_selected_stream(&mut self, db: &ThermoDatabase) {
        let is_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
        if let (Some(area), Some(s_idx)) =
            (self.wb.areas.get_mut(self.active_area), self.selected_stream_idx)
        {
            let streams = if is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            if streams.len() > 1 && s_idx < streams.len() {
                streams.remove(s_idx);
                self.selected_stream_idx = Some(s_idx.min(streams.len() - 1));
                self.selected_row_idx = None;
            }
        }
        self.recalculate(db);
    }

    fn delete_selected_row(&mut self, db: &ThermoDatabase) {
        let is_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
        if let (Some(area), Some(s_idx), Some(r_idx)) = (
            self.wb.areas.get_mut(self.active_area),
            self.selected_stream_idx,
            self.selected_row_idx,
        ) {
            let streams = if is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            if let Some(st) = streams.get_mut(s_idx) {
                if r_idx < st.species.len() {
                    st.species.remove(r_idx);
                    self.selected_row_idx = None;
                }
            }
        }
        self.recalculate(db);
    }

    fn apply_formula_bar_edit(&mut self, db: &ThermoDatabase) {
        if let ActiveSheetTab::User(u_idx) = self.active_tab {
            if let (Some(us), Some((r, c))) =
                (self.wb.user_sheets.get_mut(u_idx), self.selected_user_cell)
            {
                if !self.wb.red_font_shield || us.red_cells.contains(&(r, c)) {
                    us.cells
                        .insert((r, c), self.formula_bar_text.trim().to_string());
                    self.recalculate(db);
                }
            }
            return;
        }

        let is_out = matches!(self.active_tab, ActiveSheetTab::Out(_));
        if let (Some(area), Some(s_idx)) =
            (self.wb.areas.get_mut(self.active_area), self.selected_stream_idx)
        {
            let streams = if is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            if let Some(r_idx) = self.selected_row_idx {
                if let Some(sp) = streams.get_mut(s_idx).and_then(|s| s.species.get_mut(r_idx)) {
                    let txt = self.formula_bar_text.trim().to_string();
                    if txt.starts_with('=') {
                        sp.amount_formula = Some(txt);
                    } else if let Ok(v) = txt.parse::<f64>() {
                        sp.amount_formula = None;
                        sp.amount_kmol = v.max(0.0);
                    }
                }
            } else if let Some(st) = streams.get_mut(s_idx) {
                let txt = self.formula_bar_text.trim().to_string();
                if !txt.is_empty() {
                    st.name = txt;
                }
            }
        }
        self.recalculate(db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_ui_renders_all_tabs_and_supports_paste_stream_and_bal8_roundtrip() {
        let db = ThermoDatabase::with_open_defaults();
        let mut state = BalanceModuleState::new(&db);
        let ctx = egui::Context::default();

        // Render IN1, OUT1, BALANCE, FEED (User(0)), and Target tabs in wide & compact viewports
        for tab in [
            ActiveSheetTab::In(0),
            ActiveSheetTab::Out(0),
            ActiveSheetTab::Balance,
            ActiveSheetTab::User(0),
            ActiveSheetTab::Target,
        ] {
            state.active_tab = tab;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1380.0, 840.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                state.ui(ctx, &db);
            });
        }

        // Test Copy Stream from OUT1 and Paste Stream into IN1
        state.active_tab = ActiveSheetTab::Out(0);
        state.selected_stream_idx = Some(0);
        state.copy_selected_stream();
        let orig_in_streams = state.wb.areas[0].in_streams.len();
        state.active_tab = ActiveSheetTab::In(0);
        state.paste_linked_stream(&db);
        assert_eq!(state.wb.areas[0].in_streams.len(), orig_in_streams + 1);

        // Test .bal8 save/load roundtrip
        let tmp_bal8 = "/tmp/soryntel_test_balance.bal8";
        state.save_bal8(tmp_bal8);
        assert!(state.status_ok);
        state.load_bal8(tmp_bal8, &db);
        assert!(state.status_ok);
    }
}
