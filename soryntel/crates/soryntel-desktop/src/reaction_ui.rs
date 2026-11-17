//! Module 1: Reaction Equations (`HSC - Reaction Equations`, Chapter 10, Figs. 1–10).
//!
//! Hexagonal UI Adapter: delegates all stoichiometric balancing, Kelley polynomial integration,
//! Criss-Cobble aqueous extrapolation, `.rea8` persistence, `.xlsx` / TSV exports, and vector SVG
//! rendering to `SoryntelApplication`.

use crate::excel_grid;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Text as PlotText};
use rust_i18n::t;
use soryntel_application::SoryntelApplication;
use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};
use soryntel_thermo::SpeciesCatalogPort;
use soryntel_thermo::{
    CalculationRequest, CalculationResponse, ReactionCollectedSheet, ReactionWorkbookFile,
    ReactionYAxis,
};

const DEFAULT_REA8_PATH: &str = "soryntel_reaction.rea8";
const DEFAULT_SVG_PATH: &str = "soryntel_reaction_chart.svg";

const SERIES_PALETTE: [egui::Color32; 6] = [
    egui::Color32::from_rgb(30, 64, 175),
    egui::Color32::from_rgb(22, 163, 74),
    egui::Color32::from_rgb(185, 28, 28),
    egui::Color32::from_rgb(147, 51, 234),
    egui::Color32::from_rgb(217, 119, 6),
    egui::Color32::from_rgb(14, 116, 144),
];

const SERIES_HEX: [&str; 6] = [
    "#1e40af", "#16a34a", "#b91c1c", "#9333ea", "#d97706", "#0e7490",
];

pub struct ReactionModuleState {
    image_clipboard: Option<arboard::Clipboard>,
    pub last_chart_size: egui::Vec2,
    pub compact_chart: bool,
    pub formula_input: String,
    pub smallest_coeff: f64,
    pub t_from: f64,
    pub t_to: f64,
    pub t_step: f64,
    pub temp_unit: TemperatureUnit,
    pub energy_unit: EnergyUnit,
    pub result_format: ResultFormat,
    pub show_transitions: bool,
    pub criss_cobble: bool,
    pub y_axis: ReactionYAxis,
    pub collect_data: bool,
    pub plot_all_data: bool,
    pub log_y_scale: bool,
    pub show_curve_labels: bool,
    pub show_grid: bool,
    pub line_width: f32,
    pub decimals: usize,
    pub status_msg: String,
    pub status_ok: bool,
    pub sheets: Vec<ReactionCollectedSheet>,
    pub active_sheet_idx: usize,
    pub selected_cell: Option<(usize, usize, String)>,
    pub show_detached_chart: bool,
    pub show_chart_toolbox: bool,
    pub show_print_modal: bool,
    pub show_help_modal: bool,
    pub show_file_modal: bool,
    pub file_modal_is_save: bool,
    pub file_path_buf: String,
    pub request_open_browser: bool,
    pub plot_reset_counter: u64,
}

impl ReactionModuleState {
    pub fn new(db: &dyn SpeciesCatalogPort) -> Self {
        let mut state = Self {
            image_clipboard: None,
            last_chart_size: egui::Vec2::ZERO,
            compact_chart: false,
            formula_input: "Al2O3".to_string(),
            smallest_coeff: 1.0,
            t_from: 0.0,
            t_to: 2000.0,
            t_step: 100.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Normal,
            show_transitions: true,
            criss_cobble: false,
            y_axis: ReactionYAxis::Cp,
            collect_data: false,
            plot_all_data: false,
            log_y_scale: false,
            show_curve_labels: true,
            show_grid: true,
            line_width: 2.0,
            decimals: 3,
            status_msg: "Listo.".to_string(),
            status_ok: true,
            sheets: Vec::new(),
            active_sheet_idx: 0,
            selected_cell: None,
            show_detached_chart: false,
            show_chart_toolbox: false,
            show_print_modal: false,
            show_help_modal: false,
            show_file_modal: false,
            file_modal_is_save: true,
            file_path_buf: crate::ui_common::temp_file_path(DEFAULT_REA8_PATH),
            request_open_browser: false,
            plot_reset_counter: 0,
        };
        state.run_calculate(db);
        state
    }

    pub fn active_sheet(&self) -> Option<&ReactionCollectedSheet> {
        self.sheets.get(self.active_sheet_idx)
    }

    pub fn active_response(&self) -> Option<&CalculationResponse> {
        self.active_sheet().map(|s| &s.response)
    }

    pub fn run_balance(&mut self) {
        match SoryntelApplication::balance_reaction_equation(
            &self.formula_input,
            self.smallest_coeff,
        ) {
            Ok((balanced, _)) => {
                self.formula_input = balanced.clone();
                self.status_msg = format!("Balanceado: {balanced}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error de balance: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn run_calculate(&mut self, db: &dyn SpeciesCatalogPort) {
        let req = CalculationRequest {
            input: self.formula_input.clone(),
            t_from: self.t_from,
            t_to: self.t_to,
            t_step: self.t_step,
            temp_unit: self.temp_unit,
            energy_unit: self.energy_unit,
            result_format: self.result_format,
            show_transitions: self.show_transitions,
            criss_cobble: self.criss_cobble,
        };
        match SoryntelApplication::calculate_reaction(db, &req) {
            Ok(resp) => {
                if resp.is_single_substance {
                    if !matches!(
                        self.y_axis,
                        ReactionYAxis::Cp | ReactionYAxis::H | ReactionYAxis::S | ReactionYAxis::G
                    ) {
                        self.y_axis = ReactionYAxis::Cp;
                    }
                } else if matches!(self.y_axis, ReactionYAxis::Cp) {
                    self.y_axis = if resp.has_electrochemical_potential {
                        ReactionYAxis::EVsShe
                    } else {
                        ReactionYAxis::K
                    };
                }

                self.status_msg = format!("Cálculo completado: {}", resp.normalized_input);
                self.status_ok = true;

                if self.collect_data && !self.sheets.is_empty() {
                    let next_num = self.sheets.len() + 1;
                    self.sheets.push(ReactionCollectedSheet {
                        name: format!("S{next_num}"),
                        response: resp,
                        visible_in_plot: true,
                        amount_scale: 1.0,
                    });
                    self.active_sheet_idx = self.sheets.len() - 1;
                } else if self.sheets.is_empty() {
                    self.sheets.push(ReactionCollectedSheet {
                        name: "S1".to_string(),
                        response: resp,
                        visible_in_plot: true,
                        amount_scale: 1.0,
                    });
                    self.active_sheet_idx = 0;
                } else if let Some(sheet) = self.sheets.get_mut(self.active_sheet_idx) {
                    sheet.response = resp;
                    sheet.visible_in_plot = true;
                    sheet.amount_scale = 1.0;
                }
            }
            Err(e) => {
                self.status_msg = format!("Error: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn load_preset(
        &mut self,
        db: &dyn SpeciesCatalogPort,
        eq: &str,
        f: f64,
        t: f64,
        s: f64,
        target_y: ReactionYAxis,
        log_y: bool,
    ) {
        self.formula_input = eq.to_string();
        self.t_from = f;
        self.t_to = t;
        self.t_step = s;
        self.y_axis = target_y;
        self.log_y_scale = log_y;
        self.run_calculate(db);
    }

    /// Reproduce Fig. 6 from Manual Ch. 10.3: Multi-reaction vapor pressure comparison (`Mg`, `Zn`, `Cd`).
    pub fn load_fig6_multi_vapor_preset(&mut self, db: &dyn SpeciesCatalogPort) {
        self.sheets.clear();
        self.temp_unit = TemperatureUnit::Celsius;
        self.energy_unit = EnergyUnit::Kilojoules;
        self.t_from = 500.0;
        self.t_to = 1500.0;
        self.t_step = 50.0;
        self.y_axis = ReactionYAxis::K;
        self.log_y_scale = false;
        self.collect_data = true;
        self.plot_all_data = true;

        for eq in ["Mg = Mg(g)", "Zn = Zn(g)", "Cd = Cd(g)"] {
            self.formula_input = eq.to_string();
            self.run_calculate(db);
        }
        self.active_sheet_idx = 0;
        self.status_msg = "Ejemplo de vaporización de Mg, Zn y Cd cargado en tres hojas".into();
        self.status_ok = true;
    }

    pub fn save_rea8(&mut self, path: &str) {
        let wb = ReactionWorkbookFile {
            version: "HSC-REA8-10.0".to_string(),
            formula_input: self.formula_input.clone(),
            smallest_coeff: self.smallest_coeff,
            t_from: self.t_from,
            t_to: self.t_to,
            t_step: self.t_step,
            temp_unit: self.temp_unit,
            energy_unit: self.energy_unit,
            result_format: self.result_format,
            show_transitions: self.show_transitions,
            criss_cobble: self.criss_cobble,
            y_axis: self.y_axis,
            collect_data: self.collect_data,
            plot_all_data: self.plot_all_data,
            log_y_scale: self.log_y_scale,
            decimals: self.decimals,
            sheets: self.sheets.clone(),
            active_sheet_idx: self.active_sheet_idx,
        };
        match SoryntelApplication::serialize_rea8_workbook(&wb).and_then(|json| {
            crate::storage::write_atomic(path, json)
                .map_err(|e| soryntel_core::SoryntelError::DatabaseError(e.to_string()))
        }) {
            Ok(_) => {
                self.status_msg = format!("Libro guardado en {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error guardando .rea8: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn load_rea8(&mut self, path: &str) {
        match std::fs::read_to_string(path)
            .map_err(|e| soryntel_core::SoryntelError::DatabaseError(e.to_string()))
            .and_then(|raw| SoryntelApplication::deserialize_rea8_workbook(&raw))
        {
            Ok(wb) => {
                self.formula_input = wb.formula_input;
                self.smallest_coeff = wb.smallest_coeff;
                self.t_from = wb.t_from;
                self.t_to = wb.t_to;
                self.t_step = wb.t_step;
                self.temp_unit = wb.temp_unit;
                self.energy_unit = wb.energy_unit;
                self.result_format = wb.result_format;
                self.show_transitions = wb.show_transitions;
                self.criss_cobble = wb.criss_cobble;
                self.y_axis = wb.y_axis;
                self.collect_data = wb.collect_data;
                self.plot_all_data = wb.plot_all_data;
                self.log_y_scale = wb.log_y_scale;
                self.decimals = wb.decimals.clamp(1, 8);
                if !wb.sheets.is_empty() {
                    self.active_sheet_idx = wb.active_sheet_idx.min(wb.sheets.len() - 1);
                    self.sheets = wb.sheets;
                }
                self.status_msg = format!("Libro cargado desde {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error abriendo .rea8: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn export_excel_xlsx(&mut self, path: &str) {
        if self.sheets.is_empty() {
            return;
        }
        let bytes = SoryntelApplication::export_reaction_workbook_xlsx(&self.sheets);
        match crate::storage::write_atomic(path, bytes) {
            Ok(_) => {
                self.status_msg = format!(
                    "Exportado Excel (.xlsx) con {} hoja(s) en {path}",
                    self.sheets.len()
                );
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error exportando Excel (.xlsx): {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn export_active_csv(&mut self) {
        self.choose_export("csv");
    }

    fn choose_workbook(&mut self, save: bool) {
        let title = if save {
            t!("reaction.save")
        } else {
            t!("reaction.open")
        };
        if let Some(path) =
            crate::reaction_actions::choose_file(save, &title, "rea8", "reacciones.rea8")
        {
            self.file_path_buf = path.to_string_lossy().into_owned();
            let path = self.file_path_buf.clone();
            if save {
                self.save_rea8(&path);
            } else {
                self.load_rea8(&path);
            }
        }
    }

    fn choose_export(&mut self, extension: &str) {
        if self.active_response().is_none() {
            self.status_msg = t!("reaction.no_results").into_owned();
            self.status_ok = false;
            return;
        }
        let title = t!("reaction.export");
        let Some(path) = crate::reaction_actions::choose_file(
            true,
            &title,
            extension,
            &format!("reacciones.{extension}"),
        ) else {
            return;
        };
        let path = path.to_string_lossy().into_owned();
        match extension {
            "xlsx" => self.export_excel_xlsx(&path),
            "svg" => self.save_chart_svg(&path),
            "csv" => {
                let tsv = SoryntelApplication::format_reaction_sheet_tsv(
                    self.active_sheet().unwrap(),
                    self.decimals,
                );
                let result = crate::storage::write_atomic(
                    &path,
                    crate::reaction_actions::csv_from_tsv(&tsv),
                );
                self.status_ok = result.is_ok();
                self.status_msg = match result {
                    Ok(()) => format!("CSV guardado en {path}"),
                    Err(e) => format!("Error al guardar CSV: {e}"),
                };
            }
            "png" => {
                let result = self
                    .build_chart_svg()
                    .ok_or_else(|| "No hay gráfico".to_string())
                    .and_then(|svg| crate::reaction_actions::render_svg(&svg))
                    .and_then(|pixels| pixels.encode_png().map_err(|e| e.to_string()))
                    .and_then(|png| {
                        crate::storage::write_atomic(&path, png).map_err(|e| e.to_string())
                    });
                self.status_ok = result.is_ok();
                self.status_msg = match result {
                    Ok(()) => format!("Imagen guardada en {path}"),
                    Err(e) => format!("Error al guardar PNG: {e}"),
                };
            }
            _ => unreachable!("extensión interna"),
        }
    }

    fn copy_chart_image(&mut self) {
        let result = (|| -> Result<(), String> {
            let svg = self
                .build_chart_svg()
                .ok_or_else(|| t!("reaction.no_results").into_owned())?;
            let pixels = crate::reaction_actions::render_svg(&svg)?;
            if self.image_clipboard.is_none() {
                self.image_clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            self.image_clipboard
                .as_mut()
                .unwrap()
                .set_image(arboard::ImageData {
                    width: pixels.width() as usize,
                    height: pixels.height() as usize,
                    bytes: std::borrow::Cow::Owned(crate::reaction_actions::rgba(&pixels)),
                })
                .map_err(|e| e.to_string())
        })();
        self.status_ok = result.is_ok();
        self.status_msg = match result {
            Ok(()) => t!("reaction.image_copied").into_owned(),
            Err(e) => format!("No se pudo copiar la imagen: {e}"),
        };
    }

    pub fn copy_active_sheet_to_clipboard(&mut self, ctx: &egui::Context) {
        let Some(sheet) = self.active_sheet() else {
            return;
        };
        let tsv = SoryntelApplication::format_reaction_sheet_tsv(sheet, self.decimals);
        ctx.copy_text(tsv);
        self.status_msg = format!(
            "Hoja {} copiada al portapapeles (formato TSV para Excel)",
            sheet.name
        );
        self.status_ok = true;
    }

    pub fn build_chart_svg(&self) -> Option<String> {
        let primary = self.active_response()?;
        let title = self.y_axis.chart_title();
        let x_label = format!("T ({})", primary.temp_unit_label);
        let y_label = self.y_axis.axis_label(primary, self.log_y_scale);

        let mut series = Vec::new();
        if self.plot_all_data && self.sheets.len() > 1 {
            for (idx, sheet) in self.sheets.iter().enumerate() {
                if !sheet.visible_in_plot
                    || !SoryntelApplication::are_reaction_sheets_plot_compatible(
                        primary,
                        &sheet.response,
                    )
                {
                    continue;
                }
                let pts = SoryntelApplication::extract_reaction_plot_points(
                    &sheet.response,
                    self.y_axis,
                    self.log_y_scale,
                );
                let hex = SERIES_HEX[idx % SERIES_HEX.len()];
                series.push((sheet.response.normalized_input.clone(), pts, hex));
            }
        } else {
            let pts = SoryntelApplication::extract_reaction_plot_points(
                primary,
                self.y_axis,
                self.log_y_scale,
            );
            series.push((primary.normalized_input.clone(), pts, SERIES_HEX[0]));
        }

        Some(SoryntelApplication::render_reaction_chart_svg(
            title,
            &x_label,
            &y_label,
            &series,
            self.show_grid,
            self.show_curve_labels,
        ))
    }

    pub fn save_chart_svg(&mut self, path: &str) {
        let Some(svg) = self.build_chart_svg() else {
            return;
        };
        match crate::storage::write_atomic(path, svg) {
            Ok(_) => {
                self.status_msg = format!("Gráfico vectorial guardado en {path}");
                self.status_ok = true;
            }
            Err(e) => {
                self.status_msg = format!("Error guardando SVG: {e}");
                self.status_ok = false;
            }
        }
    }

    pub fn copy_chart_vector(&mut self, ctx: &egui::Context) {
        if let Some(svg) = self.build_chart_svg() {
            ctx.copy_text(svg.clone());
            let path = crate::ui_common::temp_file_path(DEFAULT_SVG_PATH);
            if let Err(error) = crate::storage::write_atomic(&path, &svg) {
                self.status_msg = format!("Gráfico copiado; no se pudo guardar en {path}: {error}");
                self.status_ok = false;
                return;
            }
            self.status_msg =
                format!("Gráfico vectorial (SVG) copiado al portapapeles y guardado en {path}");
            self.status_ok = true;
        }
    }

    pub fn copy_chart_series_data(&mut self, ctx: &egui::Context) {
        let Some(primary) = self.active_response() else {
            return;
        };
        let pts = SoryntelApplication::extract_reaction_plot_points(
            primary,
            self.y_axis,
            self.log_y_scale,
        );
        let mut out = format!(
            "T ({})\t{}\n",
            primary.temp_unit_label,
            self.y_axis.axis_label(primary, self.log_y_scale)
        );
        for [x, y] in pts {
            out.push_str(&format!("{x:.3}\t{y:.6}\n"));
        }
        ctx.copy_text(out);
        self.status_msg = "Datos de la serie del gráfico copiados al portapapeles".into();
        self.status_ok = true;
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &dyn SpeciesCatalogPort) {
        let yellow_bg = excel_grid::EXCEL_YELLOW_BG;

        // Keyboard shortcut Ctrl+C for copying selected cell or sheet
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
            if let Some((_, _, ref val)) = self.selected_cell {
                ctx.copy_text(val.clone());
                self.status_msg = format!("Celda copiada: {val}");
                self.status_ok = true;
            }
        }

        // 1. Top Menu Bar (Fig. 1: File | Edit | Format | Sheet | Options | Help)
        egui::TopBottomPanel::top("reaction_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(t!("common.020"), |ui| {
                    if ui.button(t!("review.000")).clicked() {
                        self.file_modal_is_save = false;
                        self.file_path_buf = crate::ui_common::temp_file_path(DEFAULT_REA8_PATH);
                        self.show_file_modal = true;
                        ui.close_menu();
                    }
                    if ui.button(t!("review.001")).clicked() {
                        self.file_modal_is_save = true;
                        self.file_path_buf = crate::ui_common::temp_file_path(DEFAULT_REA8_PATH);
                        self.show_file_modal = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(t!("common.038"))
                        .on_hover_text(t!("reaction.choose_destination"))
                        .clicked()
                    {
                        self.choose_export("xlsx");
                        ui.close_menu();
                    }
                    if ui
                        .button(t!("review.002"))
                        .on_hover_text(t!("reaction.choose_destination"))
                        .clicked()
                    {
                        self.export_active_csv();
                        ui.close_menu();
                    }
                    if ui
                        .button(t!("review.003"))
                        .on_hover_text(t!("reaction.choose_destination"))
                        .clicked()
                    {
                        self.choose_export("svg");
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t!("reaction.save_png")).clicked() {
                        self.choose_export("png");
                        ui.close_menu();
                    }
                    if ui.button(t!("review.004")).clicked() {
                        self.request_open_browser = true;
                        ui.close_menu();
                    }
                    if ui.button(t!("review.005")).clicked() {
                        self.show_print_modal = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button(t!("common.021"), |ui| {
                    if ui.button(t!("review.006")).clicked() {
                        if let Some((_, _, ref val)) = self.selected_cell {
                            ctx.copy_text(val.clone());
                            self.status_msg = format!("Celda copiada: {val}");
                            self.status_ok = true;
                        }
                        ui.close_menu();
                    }
                    if ui.button(t!("review.007")).clicked() {
                        self.copy_active_sheet_to_clipboard(ctx);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t!("review.008")).clicked() {
                        self.copy_chart_vector(ctx);
                        ui.close_menu();
                    }
                    if ui.button(t!("review.009")).clicked() {
                        self.copy_chart_series_data(ctx);
                        ui.close_menu();
                    }
                });

                ui.menu_button(t!("common.042"), |ui| {
                    ui.label(egui::RichText::new(t!("review.010")).strong());
                    for dec in [2usize, 3, 4, 6] {
                        if ui
                            .radio_value(&mut self.decimals, dec, format!("{dec} decimals"))
                            .clicked()
                        {
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(t!("review.011")).strong());
                    let mut changed = false;
                    changed |= ui
                        .radio_value(
                            &mut self.result_format,
                            ResultFormat::Normal,
                            "Normal (H, S, Cp y G de la especie)",
                        )
                        .changed();
                    changed |= ui
                        .radio_value(
                            &mut self.result_format,
                            ResultFormat::Delta,
                            "Delta (formación: ΔHf, ΔSf, ΔGf)",
                        )
                        .changed();
                    if changed {
                        self.run_calculate(db);
                        ui.close_menu();
                    }
                });

                ui.menu_button(t!("review.012"), |ui| {
                    if ui.button(t!("review.013")).clicked() {
                        let prev = self.collect_data;
                        self.collect_data = true;
                        self.run_calculate(db);
                        self.collect_data = prev;
                        ui.close_menu();
                    }
                    if ui.button(t!("review.014")).clicked() {
                        self.copy_active_sheet_to_clipboard(ctx);
                        ui.close_menu();
                    }
                    if self.sheets.len() > 1 && ui.button(t!("common.051")).clicked() {
                        self.sheets.remove(self.active_sheet_idx);
                        if self.active_sheet_idx >= self.sheets.len() {
                            self.active_sheet_idx = self.sheets.len() - 1;
                        }
                        ui.close_menu();
                    }
                    if ui.button(t!("review.015")).clicked() {
                        if let Some(first) = self.sheets.first().cloned() {
                            self.sheets = vec![first];
                            self.active_sheet_idx = 0;
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button(t!("review.016"), |ui| {
                    let mut recalc = false;
                    ui.label(egui::RichText::new(t!("common.052")).strong());
                    recalc |= ui
                        .radio_value(
                            &mut self.temp_unit,
                            TemperatureUnit::Celsius,
                            "°C (Celsius)",
                        )
                        .changed();
                    recalc |= ui
                        .radio_value(&mut self.temp_unit, TemperatureUnit::Kelvin, "K (Kelvin)")
                        .changed();
                    recalc |= ui
                        .radio_value(
                            &mut self.temp_unit,
                            TemperatureUnit::Fahrenheit,
                            "°F (Fahrenheit)",
                        )
                        .changed();
                    ui.separator();
                    ui.label(egui::RichText::new(t!("review.017")).strong());
                    recalc |= ui
                        .radio_value(&mut self.energy_unit, EnergyUnit::Kilojoules, "kJ")
                        .changed();
                    recalc |= ui
                        .radio_value(&mut self.energy_unit, EnergyUnit::Joules, "J")
                        .changed();
                    recalc |= ui
                        .radio_value(&mut self.energy_unit, EnergyUnit::Kilocalories, "kcal")
                        .changed();
                    recalc |= ui
                        .radio_value(&mut self.energy_unit, EnergyUnit::Calories, "cal")
                        .changed();
                    ui.separator();
                    ui.label(egui::RichText::new(t!("review.018")).strong());
                    recalc |= ui
                        .radio_value(
                            &mut self.result_format,
                            ResultFormat::Normal,
                            "Normal (referencia Barin/JANAF)",
                        )
                        .changed();
                    recalc |= ui
                        .radio_value(
                            &mut self.result_format,
                            ResultFormat::Delta,
                            "Delta (formación: NBS/Pankratz)",
                        )
                        .changed();
                    ui.separator();
                    recalc |= ui
                        .checkbox(&mut self.show_transitions, "Mostrar transiciones de fase")
                        .changed();
                    recalc |= ui
                        .checkbox(&mut self.criss_cobble, "Extrapolación de Criss–Cobble")
                        .changed();
                    ui.separator();
                    if ui.button(t!("review.019")).clicked() {
                        self.temp_unit = TemperatureUnit::Celsius;
                        self.t_from = 25.0;
                        self.t_to = 300.0;
                        self.t_step = 25.0;
                        recalc = true;
                        ui.close_menu();
                    }
                    if recalc {
                        self.run_calculate(db);
                    }
                });

                ui.menu_button(t!("common.022"), |ui| {
                    if ui.button(t!("review.020")).clicked() {
                        self.show_help_modal = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("🗄 Maindb6: {} especies", db.species_count()))
                            .small()
                            .color(egui::Color32::from_rgb(71, 85, 105)),
                    );
                });
            });
        });

        // 2. Icon Toolbar Strip (Quick actions + Format/Unit pills)
        egui::TopBottomPanel::top("reaction_icon_toolbar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(248, 250, 252))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(226, 232, 240),
                    ))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(t!("common.009"))
                        .on_hover_text("Abrir archivo .rea8 (File -> Open)")
                        .clicked()
                    {
                        self.file_modal_is_save = false;
                        self.file_path_buf = crate::ui_common::temp_file_path(DEFAULT_REA8_PATH);
                        self.show_file_modal = true;
                    }
                    if ui
                        .button("💾 Guardar .rea8")
                        .on_hover_text("Guardar libro .rea8 (Archivo → Guardar como)")
                        .clicked()
                    {
                        self.choose_workbook(true);
                    }
                    if ui
                        .button("📊 Excel .xlsx")
                        .on_hover_text("Exportar todas las hojas a MS Excel (.xlsx)")
                        .clicked()
                    {
                        self.choose_export("xlsx");
                    }
                    if ui
                        .button(t!("review.021"))
                        .on_hover_text("Copiar hoja activa al portapapeles (TSV para Excel)")
                        .clicked()
                    {
                        self.copy_active_sheet_to_clipboard(ctx);
                    }
                    if ui
                        .button(t!("review.022"))
                        .on_hover_text("Vista previa de impresión / reporte")
                        .clicked()
                    {
                        self.show_print_modal = true;
                    }
                    ui.separator();
                    if ui
                        .button(t!("review.023"))
                        .on_hover_text("Balancear coeficientes estequiométricos y carga iónica")
                        .clicked()
                    {
                        self.run_balance();
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(t!("review.024"))
                                    .strong()
                                    .color(egui::Color32::from_rgb(30, 58, 138)),
                            )
                            .fill(egui::Color32::from_rgb(219, 234, 254)),
                        )
                        .on_hover_text("Ejecutar cálculo termodinámico (Enter)")
                        .clicked()
                    {
                        self.run_calculate(db);
                    }
                    if ui
                        .button(t!("review.025"))
                        .on_hover_text("Abrir navegador de base de datos (Maindb6 / OwnDB6)")
                        .clicked()
                    {
                        self.request_open_browser = true;
                    }
                    ui.separator();

                    // Quick Format Toggle (Normal vs Delta)
                    ui.label(egui::RichText::new(t!("review.026")).small());
                    let is_norm = self.result_format == ResultFormat::Normal;
                    if ui.selectable_label(is_norm, "Normal").clicked() && !is_norm {
                        self.result_format = ResultFormat::Normal;
                        self.run_calculate(db);
                    }
                    if ui
                        .selectable_label(!is_norm, "Delta (Δf)")
                        .on_hover_text("Funciones de formación ΔHf, ΔSf, ΔGf a partir de elementos")
                        .clicked()
                        && is_norm
                    {
                        self.result_format = ResultFormat::Delta;
                        self.run_calculate(db);
                    }

                    ui.separator();
                    ui.label(egui::RichText::new(t!("common.024")).small());
                    for u in [TemperatureUnit::Celsius, TemperatureUnit::Kelvin] {
                        if ui
                            .selectable_label(self.temp_unit == u, u.label())
                            .clicked()
                            && self.temp_unit != u
                        {
                            self.temp_unit = u;
                            self.run_calculate(db);
                        }
                    }
                    for eu in [EnergyUnit::Kilojoules, EnergyUnit::Kilocalories] {
                        if ui
                            .selectable_label(self.energy_unit == eu, eu.energy_label())
                            .clicked()
                            && self.energy_unit != eu
                        {
                            self.energy_unit = eu;
                            self.run_calculate(db);
                        }
                    }
                });
            });

        // Keep controls scrollable so small windows retain space for results.
        egui::TopBottomPanel::top("reaction_top_controls").show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("reaction_controls_scroll")
                .max_height((ctx.screen_rect().height() * 0.32).clamp(90.0, 280.0))
                .show(ui, |ui| {
                    if ui.available_width() >= 860.0 {
                        ui.columns(2, |cols| {
                            self.render_specifications_box(&mut cols[0], db, yellow_bg);
                            self.render_chart_controls_box(&mut cols[1], ctx, db, yellow_bg);
                        });
                    } else {
                        self.render_specifications_box(ui, db, yellow_bg);
                        egui::CollapsingHeader::new("Opciones del gráfico").show(ui, |ui| {
                            self.render_chart_controls_box(ui, ctx, db, yellow_bg);
                        });
                    }
                });
        });

        // 4. Bottom Status Bar (with Outotec/Soryntel badge matching Fig. 1)
        egui::TopBottomPanel::bottom("reaction_status_bar").show(ctx, |ui| {
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
                ui.colored_label(color, &self.status_msg);
                if let Some((r, c, ref val)) = self.selected_cell {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Celda {}{}: {}",
                                excel_grid::col_letter(c),
                                r,
                                val
                            ))
                            .small()
                            .monospace(),
                        );
                    });
                }
            });
        });

        // 5. Main Central Split (Responsive: Left Excel Tables | Right Chart + Fig. 2 Tools)
        egui::CentralPanel::default().show(ctx, |ui| {
            let avail_w = ui.available_width();
            if avail_w >= 880.0 {
                ui.columns(2, |cols| {
                    self.render_left_spreadsheet_pane(&mut cols[0], db);
                    self.render_right_chart_pane(&mut cols[1], ctx);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.compact_chart, false, "Datos y cantidades");
                    ui.selectable_value(&mut self.compact_chart, true, "Gráfico");
                });
                if self.compact_chart {
                    self.render_right_chart_pane(ui, ctx);
                } else {
                    self.render_left_spreadsheet_pane(ui, db);
                }
            }
        });

        // 6. Modals & Detached Windows
        self.render_detached_chart_window(ctx);
        self.render_file_modal(ctx);
        self.render_print_report_modal(ctx);
        self.render_help_modal(ctx);
    }

    fn render_specifications_box(
        &mut self,
        ui: &mut egui::Ui,
        db: &dyn SpeciesCatalogPort,
        yellow_bg: egui::Color32,
    ) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("📋 Parámetros");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(t!("review.023"))
                        .on_hover_text("Resolver coeficientes desconocidos (Ch. 10.3)")
                        .clicked()
                    {
                        self.run_balance();
                    }
                    ui.add(
                        egui::DragValue::new(&mut self.smallest_coeff)
                            .speed(0.1)
                            .range(0.01..=1000.0),
                    );
                    ui.label(t!("review.027"));
                });
            });
            ui.label(
                egui::RichText::new(t!("review.028"))
                    .size(11.5)
                    .color(egui::Color32::from_rgb(51, 65, 85)),
            );

            let resp = ui
                .scope(|ui| {
                    ui.visuals_mut().extreme_bg_color = yellow_bg;
                    ui.add(
                        egui::TextEdit::singleline(&mut self.formula_input)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    )
                })
                .inner;
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.run_calculate(db);
            }

            let t_lbl = self.temp_unit.label();
            ui.horizontal_wrapped(|ui| {
                ui.label(t!("common.005"));
                ui.label(format!("Desde ({t_lbl})"));
                ui.add(egui::DragValue::new(&mut self.t_from).speed(5.0));
                ui.label(format!("Hasta ({t_lbl})"));
                ui.add(egui::DragValue::new(&mut self.t_to).speed(5.0));
                ui.label(format!("Paso ({t_lbl})"));
                ui.add(
                    egui::DragValue::new(&mut self.t_step)
                        .speed(1.0)
                        .range(0.1..=5000.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(t!("review.024")).strong())
                                .fill(egui::Color32::from_rgb(219, 234, 254)),
                        )
                        .clicked()
                    {
                        self.run_calculate(db);
                    }
                    if ui.button(t!("review.029")).clicked() {
                        self.request_open_browser = true;
                    }
                });
            });
        });
    }

    fn render_chart_controls_box(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        db: &dyn SpeciesCatalogPort,
        yellow_bg: egui::Color32,
    ) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("📈 Gráfico");
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(t!("review.030"));
                    if let Some(res) = self.active_response().cloned() {
                        let is_delta = res.result_format == ResultFormat::Delta;
                        let selected_lbl = self.y_axis.axis_label(&res, self.log_y_scale);
                        egui::Frame::none().fill(yellow_bg).show(ui, |ui| {
                            egui::ComboBox::from_id_salt("reaction_y_axis_combo")
                                .selected_text(selected_lbl)
                                .width(185.0)
                                .show_ui(ui, |ui| {
                                    if res.is_single_substance {
                                        for kind in [
                                            ReactionYAxis::Cp,
                                            ReactionYAxis::H,
                                            ReactionYAxis::S,
                                            ReactionYAxis::G,
                                        ] {
                                            let lbl = kind.axis_label(&res, false);
                                            ui.selectable_value(&mut self.y_axis, kind, lbl);
                                        }
                                    } else {
                                        for kind in [
                                            ReactionYAxis::LogK,
                                            ReactionYAxis::K,
                                            ReactionYAxis::H,
                                            ReactionYAxis::S,
                                            ReactionYAxis::G,
                                        ] {
                                            let lbl = kind.short_label(false, is_delta);
                                            ui.selectable_value(&mut self.y_axis, kind, lbl);
                                        }
                                        if res.has_electrochemical_potential {
                                            ui.selectable_value(
                                                &mut self.y_axis,
                                                ReactionYAxis::EVsShe,
                                                "E vs. SHE (V)",
                                            );
                                        }
                                    }
                                });
                        });
                    }
                });

                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.collect_data, "Recopilar datos");
                    ui.checkbox(&mut self.plot_all_data, "Representar todas las hojas");
                });
                ui.vertical(|ui| {
                    ui.checkbox(&mut self.log_y_scale, "Escala logarítmica Y (para K)");
                    ui.checkbox(&mut self.show_curve_labels, "Etiquetas de curvas");
                });
            });

            // Buttons matching Fig. 1: Open in new Window | Copy Vector | Copy Raster + Presets
            ui.horizontal_wrapped(|ui| {
                if ui.button(t!("review.031")).clicked() {
                    self.show_detached_chart = true;
                }
                if ui
                    .button(t!("review.032"))
                    .on_hover_text("Copiar gráfico como SVG vectorial al portapapeles")
                    .clicked()
                {
                    self.copy_chart_vector(ctx);
                }
                if ui
                    .button(t!("review.033"))
                    .on_hover_text(t!("reaction.copy_image"))
                    .clicked()
                {
                    self.copy_chart_image();
                }
                if ui.button(t!("reaction.save_png")).clicked() {
                    self.choose_export("png");
                }
                ui.separator();
                ui.label(egui::RichText::new(t!("common.045")).small());
                if ui.small_button("Al2O3 (Fig.1)").clicked() {
                    self.load_preset(db, "Al2O3", 0.0, 2000.0, 100.0, ReactionYAxis::Cp, false);
                }
                if ui.small_button("Mond (Fig.3-4)").clicked() {
                    self.load_preset(
                        db,
                        "Ni + 4CO(g) = Ni(CO)4(g)",
                        70.0,
                        160.0,
                        10.0,
                        ReactionYAxis::K,
                        true,
                    );
                }
                if ui.small_button("Mg=Mg(g) (Fig.5)").clicked() {
                    self.load_preset(
                        db,
                        "Mg = Mg(g)",
                        500.0,
                        1500.0,
                        50.0,
                        ReactionYAxis::K,
                        false,
                    );
                }
                if ui.small_button("Vap 3x (Fig.6)").clicked() {
                    self.load_fig6_multi_vapor_preset(db);
                }
                if ui.small_button("FeS2 Redox (Fig.7-9)").clicked() {
                    self.load_preset(
                        db,
                        "2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-",
                        0.0,
                        175.0,
                        25.0,
                        ReactionYAxis::EVsShe,
                        false,
                    );
                }
            });
        });
    }

    fn render_left_spreadsheet_pane(&mut self, ui: &mut egui::Ui, db: &dyn SpeciesCatalogPort) {
        let Some(sheet_clone) = self.active_sheet().cloned() else {
            return;
        };
        let res = &sheet_clone.response;
        let dec = self.decimals.clamp(1, 8);
        let mut new_scale = sheet_clone.amount_scale;

        egui::Frame::group(ui.style()).show(ui, |ui| {
            let total_h = ui.available_height().max(320.0);
            let rxn_table_h = (total_h * 0.56).clamp(180.0, 520.0);
            let sp_table_h = (total_h * 0.28).clamp(115.0, 240.0);

            // Upper Excel Grid: Formula/Reaction Banner + Reaction Data (Figs. 1, 5, 9)
            egui::ScrollArea::both()
                .id_salt("reaction_main_table_scroll")
                .max_height(rxn_table_h)
                .show(ui, |ui| {
                    let dark_txt = egui::Color32::from_rgb(24, 24, 27);
                    let num_cols = if res.is_single_substance {
                        6usize
                    } else if res.has_electrochemical_potential {
                        7usize
                    } else {
                        6usize
                    };
                    let col_w = |c: usize| -> f32 {
                        if res.is_single_substance && c == 5 {
                            165.0
                        } else {
                            94.0
                        }
                    };

                    egui::Grid::new("reaction_main_excel_grid")
                        .spacing([0.0, 0.0])
                        .show(ui, |ui| {
                            // Column Letters Row (A, B, C...)
                            excel_grid::header_cell(ui, "", 34.0);
                            for c in 0..num_cols {
                                excel_grid::header_cell(ui, &excel_grid::col_letter(c), col_w(c));
                            }
                            ui.end_row();

                            // Row 1: Formula / Reaction Equation header banner (Figs. 1, 5, 9)
                            excel_grid::row_num_cell(ui, 1, 34.0);
                            let banner_lbl = if res.is_single_substance {
                                "Fórmula"
                            } else {
                                "Ecuación de reacción"
                            };
                            for c in 0..num_cols {
                                let txt = if c == 2 { banner_lbl } else { "" };
                                excel_grid::section_cell(ui, txt, col_w(c), true);
                            }
                            ui.end_row();

                            // Row 2: Formula / Equation string
                            excel_grid::row_num_cell(ui, 2, 34.0);
                            for c in 0..num_cols {
                                let txt = if c == 2 { &res.normalized_input } else { "" };
                                excel_grid::section_cell(ui, txt, col_w(c), true);
                            }
                            ui.end_row();

                            // Row 4: Reaction Data section title
                            excel_grid::row_num_cell(ui, 4, 34.0);
                            excel_grid::section_cell(ui, "Datos de reacción", col_w(0), false);
                            for c in 1..num_cols {
                                excel_grid::section_cell(ui, "", col_w(c), false);
                            }
                            ui.end_row();

                            if res.is_single_substance {
                                let is_delta = res.result_format == ResultFormat::Delta;
                                // Row 5: Property names
                                excel_grid::row_num_cell(ui, 5, 34.0);
                                excel_grid::section_cell(ui, "T", col_w(0), true);
                                excel_grid::section_cell(
                                    ui,
                                    if is_delta { "ΔCp" } else { "Cp" },
                                    col_w(1),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    if is_delta { "ΔH" } else { "H" },
                                    col_w(2),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    if is_delta { "ΔS" } else { "S" },
                                    col_w(3),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    if is_delta { "ΔG" } else { "G" },
                                    col_w(4),
                                    true,
                                );
                                excel_grid::section_cell(ui, "Referencia", col_w(5), true);
                                ui.end_row();

                                // Row 6: Units row (Fig. 1)
                                excel_grid::row_num_cell(ui, 6, 34.0);
                                excel_grid::section_cell(ui, &res.temp_unit_label, col_w(0), true);
                                excel_grid::section_cell(
                                    ui,
                                    &res.entropy_unit_label,
                                    col_w(1),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    &format!("{}/mol", res.energy_unit_label),
                                    col_w(2),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    &res.entropy_unit_label,
                                    col_w(3),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    &format!("{}/mol", res.energy_unit_label),
                                    col_w(4),
                                    true,
                                );
                                excel_grid::section_cell(ui, "", col_w(5), true);
                                ui.end_row();

                                for (idx, r) in res.substance_rows.iter().enumerate() {
                                    let row_num = idx + 7;
                                    let bg = if r.is_transition {
                                        egui::Color32::from_rgb(254, 249, 195)
                                    } else if idx % 2 == 0 {
                                        egui::Color32::WHITE
                                    } else {
                                        excel_grid::EXCEL_ALT_ROW_BG
                                    };
                                    excel_grid::row_num_cell(ui, row_num, 34.0);
                                    let vals = [
                                        format!("{:.dec$}", r.temperature, dec = dec),
                                        format!("{:.dec$}", r.cp, dec = dec),
                                        format!("{:.dec$}", r.h, dec = dec),
                                        format!("{:.dec$}", r.s, dec = dec),
                                        format!("{:.dec$}", r.g, dec = dec),
                                        r.reference.clone(),
                                    ];
                                    for (c, v) in vals.iter().enumerate() {
                                        let sel = self
                                            .selected_cell
                                            .as_ref()
                                            .is_some_and(|(sr, sc, _)| *sr == row_num && *sc == c);
                                        if excel_grid::text_cell(
                                            ui,
                                            v,
                                            col_w(c),
                                            bg,
                                            dark_txt,
                                            c < 5,
                                            false,
                                            sel,
                                        )
                                        .clicked()
                                        {
                                            self.selected_cell = Some((row_num, c, v.clone()));
                                        }
                                    }
                                    ui.end_row();
                                }
                            } else {
                                // Reaction Equation two-row header (Figs. 5 & 9)
                                excel_grid::row_num_cell(ui, 5, 34.0);
                                excel_grid::section_cell(ui, "T", col_w(0), true);
                                excel_grid::section_cell(ui, "ΔH", col_w(1), true);
                                excel_grid::section_cell(ui, "ΔS", col_w(2), true);
                                excel_grid::section_cell(ui, "ΔG", col_w(3), true);
                                excel_grid::section_cell(ui, "K", col_w(4), true);
                                excel_grid::section_cell(ui, "Log K", col_w(5), true);
                                if res.has_electrochemical_potential {
                                    excel_grid::section_cell(ui, "E vs. SHE", col_w(6), true);
                                }
                                ui.end_row();

                                excel_grid::row_num_cell(ui, 6, 34.0);
                                excel_grid::section_cell(ui, &res.temp_unit_label, col_w(0), true);
                                excel_grid::section_cell(
                                    ui,
                                    &res.energy_unit_label,
                                    col_w(1),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    &res.reaction_entropy_unit_label,
                                    col_w(2),
                                    true,
                                );
                                excel_grid::section_cell(
                                    ui,
                                    &res.energy_unit_label,
                                    col_w(3),
                                    true,
                                );
                                excel_grid::section_cell(ui, "", col_w(4), true);
                                excel_grid::section_cell(ui, "", col_w(5), true);
                                if res.has_electrochemical_potential {
                                    excel_grid::section_cell(ui, "V", col_w(6), true);
                                }
                                ui.end_row();

                                for (idx, r) in res.reaction_rows.iter().enumerate() {
                                    let row_num = idx + 7;
                                    let bg = if idx % 2 == 0 {
                                        egui::Color32::WHITE
                                    } else {
                                        excel_grid::EXCEL_ALT_ROW_BG
                                    };
                                    excel_grid::row_num_cell(ui, row_num, 34.0);
                                    let mut vals = vec![
                                        format!("{:.dec$}", r.temperature, dec = dec),
                                        format!("{:.dec$}", r.delta_h, dec = dec),
                                        format!("{:.dec$}", r.delta_s, dec = dec),
                                        format!("{:.dec$}", r.delta_g, dec = dec),
                                        format!("{:.3E}", r.k_eq),
                                        format!("{:.dec$}", r.log10_k, dec = dec),
                                    ];
                                    if let Some(ev) = r.e_vs_she_v {
                                        vals.push(format!("{:.dec$}", ev, dec = dec));
                                    }
                                    for (c, v) in vals.iter().enumerate() {
                                        let sel = self
                                            .selected_cell
                                            .as_ref()
                                            .is_some_and(|(sr, sc, _)| *sr == row_num && *sc == c);
                                        if excel_grid::text_cell(
                                            ui,
                                            v,
                                            col_w(c),
                                            bg,
                                            dark_txt,
                                            true,
                                            false,
                                            sel,
                                        )
                                        .clicked()
                                        {
                                            self.selected_cell = Some((row_num, c, v.clone()));
                                        }
                                    }
                                    ui.end_row();
                                }
                            }
                        });
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.strong("🔬 Datos de especies");
                ui.label(
                    egui::RichText::new(
                        "(edite mol o g en las cantidades de entrada para ajustar la escala)",
                    )
                    .small()
                    .color(egui::Color32::from_rgb(100, 116, 139)),
                );
                if (new_scale - 1.0).abs() > 1e-6 && ui.small_button(t!("review.034")).clicked() {
                    new_scale = 1.0;
                }
            });

            // Lower Excel Grid: Species Data (Figs. 1, 5, 9 with 2-row header & Reactant/Product double separator)
            egui::ScrollArea::both()
                .id_salt("reaction_species_table_scroll")
                .max_height(sp_table_h)
                .show(ui, |ui| {
                    let dark_txt = egui::Color32::from_rgb(24, 24, 27);
                    let widths = [98.0, 76.0, 78.0, 72.0, 82.0, 78.0, 48.0, 118.0];
                    egui::Grid::new("reaction_species_excel_grid")
                        .spacing([0.0, 0.0])
                        .show(ui, |ui| {
                            // Top header row: Formula | M | Conc. | Input Amounts (col 3..6) | Extrapolated
                            excel_grid::row_num_cell(ui, 1, 34.0);
                            excel_grid::section_cell(ui, "Fórmula", widths[0], false);
                            excel_grid::section_cell(ui, "M", widths[1], true);
                            excel_grid::section_cell(ui, "Conc.", widths[2], true);
                            excel_grid::section_cell(ui, "Entrada", widths[3], true);
                            excel_grid::section_cell(ui, "Cantidades", widths[4], true);
                            excel_grid::section_cell(ui, "", widths[5], true);
                            excel_grid::section_cell(ui, "", widths[6], true);
                            excel_grid::section_cell(ui, "Extrapolación", widths[7], true);
                            ui.end_row();

                            // Second header row: units
                            excel_grid::row_num_cell(ui, 2, 34.0);
                            excel_grid::section_cell(ui, "", widths[0], false);
                            excel_grid::section_cell(ui, "g/mol", widths[1], true);
                            excel_grid::section_cell(ui, "wt-%", widths[2], true);
                            excel_grid::section_cell(ui, "mol", widths[3], true);
                            excel_grid::section_cell(ui, "g", widths[4], true);
                            excel_grid::section_cell(ui, "Volumen", widths[5], true);
                            excel_grid::section_cell(ui, "Unidad", widths[6], true);
                            excel_grid::section_cell(ui, "Desde T (K)", widths[7], true);
                            ui.end_row();

                            let mut prev_side: Option<&str> = None;
                            for (idx, s) in res.species_data.iter().enumerate() {
                                // Double line divider between Reactants and Products (Figs. 5 & 9)
                                if let Some(ps) = prev_side {
                                    if ps != s.side {
                                        let (sep_r, _) = ui.allocate_exact_size(
                                            egui::vec2(34.0, 4.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(
                                            sep_r,
                                            0.0,
                                            excel_grid::EXCEL_HEADER_BORDER,
                                        );
                                        for &w in &widths {
                                            let (r, _) = ui.allocate_exact_size(
                                                egui::vec2(w, 4.0),
                                                egui::Sense::hover(),
                                            );
                                            let p = ui.painter();
                                            p.rect_filled(r, 0.0, egui::Color32::WHITE);
                                            p.line_segment(
                                                [
                                                    egui::pos2(r.min.x, r.min.y + 1.0),
                                                    egui::pos2(r.max.x, r.min.y + 1.0),
                                                ],
                                                egui::Stroke::new(
                                                    1.2,
                                                    egui::Color32::from_rgb(71, 85, 105),
                                                ),
                                            );
                                            p.line_segment(
                                                [
                                                    egui::pos2(r.min.x, r.max.y - 1.0),
                                                    egui::pos2(r.max.x, r.max.y - 1.0),
                                                ],
                                                egui::Stroke::new(
                                                    1.2,
                                                    egui::Color32::from_rgb(71, 85, 105),
                                                ),
                                            );
                                        }
                                        ui.end_row();
                                    }
                                }
                                prev_side = Some(&s.side);

                                let bg = if idx % 2 == 0 {
                                    egui::Color32::WHITE
                                } else {
                                    excel_grid::EXCEL_ALT_ROW_BG
                                };
                                excel_grid::row_num_cell(ui, idx + 3, 34.0);
                                excel_grid::text_cell(
                                    ui, &s.formula, widths[0], bg, dark_txt, false, true, false,
                                );
                                excel_grid::text_cell(
                                    ui,
                                    &format!("{:.3}", s.molar_mass),
                                    widths[1],
                                    bg,
                                    dark_txt,
                                    true,
                                    false,
                                    false,
                                );
                                excel_grid::text_cell(
                                    ui,
                                    &format!("{:.3}", s.conc_wt_pct),
                                    widths[2],
                                    bg,
                                    dark_txt,
                                    true,
                                    false,
                                    false,
                                );

                                // Interactive `mol` cell
                                let mut mol_val = s.mol * new_scale;
                                excel_grid::widget_cell(ui, widths[3], bg, false, |ui| {
                                    let resp = ui.add(
                                        egui::DragValue::new(&mut mol_val)
                                            .speed(0.05)
                                            .range(0.0001..=1e9)
                                            .fixed_decimals(3),
                                    );
                                    if resp.changed() && s.mol > 1e-12 {
                                        new_scale = (mol_val / s.mol).max(1e-6);
                                    }
                                });

                                // Interactive `g` cell
                                let mut g_val = s.mass_g * new_scale;
                                excel_grid::widget_cell(ui, widths[4], bg, false, |ui| {
                                    let resp = ui.add(
                                        egui::DragValue::new(&mut g_val)
                                            .speed(0.5)
                                            .range(0.0..=1e12)
                                            .fixed_decimals(3),
                                    );
                                    if resp.changed() && s.mass_g > 1e-12 {
                                        new_scale = (g_val / s.mass_g).max(1e-6);
                                    }
                                });

                                excel_grid::text_cell(
                                    ui,
                                    &format!("{:.3}", s.volume * new_scale),
                                    widths[5],
                                    bg,
                                    dark_txt,
                                    true,
                                    false,
                                    false,
                                );
                                excel_grid::text_cell(
                                    ui,
                                    &s.volume_unit,
                                    widths[6],
                                    bg,
                                    dark_txt,
                                    false,
                                    false,
                                    false,
                                );
                                let ext = s
                                    .extrapolated_from_k
                                    .map(|k| format!("{k:.2}"))
                                    .unwrap_or_else(|| "-".into());
                                excel_grid::text_cell(
                                    ui, &ext, widths[7], bg, dark_txt, true, false, false,
                                );
                                ui.end_row();
                            }
                        });
                });

            ui.separator();
            // Bottom Excel Sheet Tabs Bar (`|< < > >|  S1  S2 ... +` matching Fig. 1)
            ui.horizontal(|ui| {
                if ui
                    .small_button("|<")
                    .on_hover_text("Primera hoja")
                    .clicked()
                {
                    self.active_sheet_idx = 0;
                }
                if ui
                    .small_button("<")
                    .on_hover_text("Hoja anterior")
                    .clicked()
                    && self.active_sheet_idx > 0
                {
                    self.active_sheet_idx -= 1;
                }
                if ui
                    .small_button(">")
                    .on_hover_text("Siguiente hoja")
                    .clicked()
                    && self.active_sheet_idx + 1 < self.sheets.len()
                {
                    self.active_sheet_idx += 1;
                }
                if ui.small_button(">|").on_hover_text("Última hoja").clicked()
                    && !self.sheets.is_empty()
                {
                    self.active_sheet_idx = self.sheets.len() - 1;
                }
                ui.separator();
                for (idx, sheet) in self.sheets.iter().enumerate() {
                    let selected = idx == self.active_sheet_idx;
                    if ui.selectable_label(selected, &sheet.name).clicked() {
                        self.active_sheet_idx = idx;
                    }
                }
                if ui
                    .small_button("➕")
                    .on_hover_text("Colectar cálculo actual en una nueva hoja")
                    .clicked()
                {
                    let prev = self.collect_data;
                    self.collect_data = true;
                    self.run_calculate(db);
                    self.collect_data = prev;
                }
            });
        });

        if let Some(sheet) = self.sheets.get_mut(self.active_sheet_idx) {
            sheet.amount_scale = new_scale;
        }
    }

    fn render_right_chart_pane(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            // Check compatibility when Plot All Data is enabled (Ch. 10.1 Step 4)
            let mut incompatible_names = Vec::new();
            if self.plot_all_data && self.sheets.len() > 1 {
                if let Some(primary) = self.active_response().cloned() {
                    for s in &self.sheets {
                        if s.visible_in_plot
                            && !SoryntelApplication::are_reaction_sheets_plot_compatible(
                                &primary,
                                &s.response,
                            )
                        {
                            incompatible_names.push(s.name.clone());
                        }
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(t!("review.035")).strong().size(11.5));
                    for sheet in &mut self.sheets {
                        ui.checkbox(
                            &mut sheet.visible_in_plot,
                            format!("{}: {}", sheet.name, sheet.response.normalized_input),
                        );
                    }
                });
                if !incompatible_names.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(180, 83, 9),
                        format!(
                            "⚠ Ch. 10.1: Hojas {:?} omitidas por tener distinto modo (sustancia vs reacción), unidades o formato Normal/Delta.",
                            incompatible_names
                        ),
                    );
                }
            }

            if self.show_chart_toolbox {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(241, 245, 249))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong("🎨 Herramientas del gráfico:");
                            ui.checkbox(&mut self.show_grid, "Cuadrícula");
                            ui.checkbox(&mut self.show_curve_labels, "Etiquetas de curvas");
                            ui.checkbox(&mut self.log_y_scale, "Log10(K)");
                            ui.label(t!("review.036"));
                            ui.add(
                                egui::DragValue::new(&mut self.line_width)
                                    .speed(0.1)
                                    .range(1.0..=5.0),
                            );
                        });
                    });
            }

            // Horizontal layout: Left Vertical Chart Tools Strip (Fig. 2) | Right Plot Canvas
            ui.horizontal_wrapped(|ui| {
                self.render_fig2_chart_tools_strip(ui, ctx);
            });
            self.render_chart(ui, "thermo_plot_main");
        });
    }

    /// Vertical 6-icon Chart Tools bar matching Fig. 2 (`Save, Copy, Print, Clone, Redraw, Toolbox`).
    fn render_fig2_chart_tools_strip(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(248, 250, 252))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(214, 220, 229),
            ))
            .rounding(5.0)
            .inner_margin(egui::Margin::symmetric(4.0, 6.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if chart_tool_icon_btn(ui, "💾", "Guardar gráfico vectorial SVG (cap. 10.2)")
                        .clicked()
                    {
                        self.choose_export("svg");
                    }
                    if chart_tool_icon_btn(ui, "📋", "Copiar gráfico o datos al portapapeles")
                        .clicked()
                    {
                        self.copy_chart_vector(ctx);
                    }
                    if chart_tool_icon_btn(ui, "📄", "Imprimir o ver informe").clicked() {
                        self.show_print_modal = true;
                    }
                    if chart_tool_icon_btn(ui, "📑", "Duplicar gráfico en ventana independiente")
                        .clicked()
                    {
                        self.show_detached_chart = true;
                    }
                    if chart_tool_icon_btn(ui, "🔄", "Redibujar y restablecer ejes").clicked() {
                        self.plot_reset_counter = self.plot_reset_counter.wrapping_add(1);
                        self.status_msg = "Gráfico redibujado y reescalado".into();
                        self.status_ok = true;
                    }
                    if chart_tool_icon_btn(
                        ui,
                        "🎨",
                        "Editar cuadrícula, etiquetas y estilo de línea",
                    )
                    .clicked()
                    {
                        self.show_chart_toolbox = !self.show_chart_toolbox;
                    }
                });
            });
    }

    pub fn render_chart(&mut self, ui: &mut egui::Ui, plot_id: &str) {
        let title = self.y_axis.chart_title();
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .size(14.5)
                    .color(egui::Color32::from_rgb(15, 23, 42)),
            );
        });

        let Some(primary) = self.active_response().cloned() else {
            return;
        };
        let x_label = format!("T ({})", primary.temp_unit_label);
        let y_label = self.y_axis.axis_label(&primary, self.log_y_scale);

        let full_id = format!("{plot_id}_{}", self.plot_reset_counter);
        let plot = Plot::new(full_id)
            .width(ui.available_width().max(1.0))
            .height(ui.available_height().clamp(320.0, 1200.0))
            .x_axis_label(x_label)
            .y_axis_label(y_label)
            .show_grid(self.show_grid)
            .legend(Legend::default());

        let show_labels = self.show_curve_labels;
        let line_w = self.line_width;

        let response = plot.show(ui, |plot_ui| {
            if self.plot_all_data && self.sheets.len() > 1 {
                let total_sheets = self.sheets.len();
                for (idx, sheet) in self.sheets.iter().enumerate() {
                    if !sheet.visible_in_plot
                        || !SoryntelApplication::are_reaction_sheets_plot_compatible(
                            &primary,
                            &sheet.response,
                        )
                    {
                        continue;
                    }
                    let raw_pts = SoryntelApplication::extract_reaction_plot_points(
                        &sheet.response,
                        self.y_axis,
                        self.log_y_scale,
                    );
                    let color = SERIES_PALETTE[idx % SERIES_PALETTE.len()];
                    plot_ui.line(
                        Line::new(PlotPoints::from(raw_pts.clone()))
                            .name(&sheet.response.normalized_input)
                            .color(color)
                            .width(line_w),
                    );
                    if show_labels && !raw_pts.is_empty() {
                        let mid = ((raw_pts.len() * (idx + 1)) / (total_sheets + 1))
                            .min(raw_pts.len() - 1);
                        let [mx, my] = raw_pts[mid];
                        plot_ui.text(
                            PlotText::new(
                                [mx, my].into(),
                                egui::RichText::new(&sheet.response.normalized_input)
                                    .strong()
                                    .size(11.5)
                                    .color(color),
                            )
                            .anchor(egui::Align2::LEFT_BOTTOM),
                        );
                    }
                }
            } else {
                let raw_pts = SoryntelApplication::extract_reaction_plot_points(
                    &primary,
                    self.y_axis,
                    self.log_y_scale,
                );
                let color = SERIES_PALETTE[0];
                plot_ui.line(
                    Line::new(PlotPoints::from(raw_pts.clone()))
                        .name(&primary.normalized_input)
                        .color(color)
                        .width(line_w),
                );
                if show_labels && !raw_pts.is_empty() {
                    let mid = (raw_pts.len() / 3).min(raw_pts.len() - 1);
                    let [mx, my] = raw_pts[mid];
                    plot_ui.text(
                        PlotText::new(
                            [mx, my].into(),
                            egui::RichText::new(&primary.normalized_input)
                                .strong()
                                .size(12.0)
                                .color(color),
                        )
                        .anchor(egui::Align2::LEFT_BOTTOM),
                    );
                }
            }
        });
        self.last_chart_size = response.response.rect.size();
    }

    fn render_detached_chart_window(&mut self, ctx: &egui::Context) {
        if !self.show_detached_chart {
            return;
        }
        let mut open = self.show_detached_chart;
        egui::Window::new(t!("review.037"))
            .open(&mut open)
            .default_size([800.0, 540.0])
            .min_size([360.0, 320.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(t!("review.038")).clicked() {
                        self.choose_export("svg");
                    }
                    if ui.button(t!("review.032")).clicked() {
                        self.copy_chart_vector(ctx);
                    }
                    if ui.button(t!("review.039")).clicked() {
                        self.export_active_csv();
                    }
                    ui.checkbox(&mut self.log_y_scale, "Escala logarítmica Y (para K)");
                    ui.checkbox(&mut self.plot_all_data, "Representar todas las hojas");
                    ui.checkbox(&mut self.show_curve_labels, "Etiquetas de curvas");
                });
                ui.separator();
                self.render_chart(ui, "thermo_plot_detached");
            });
        self.show_detached_chart = open;
    }

    fn render_file_modal(&mut self, _ctx: &egui::Context) {
        if self.show_file_modal {
            self.show_file_modal = false;
            self.choose_workbook(self.file_modal_is_save);
        }
    }

    fn render_print_report_modal(&mut self, ctx: &egui::Context) {
        if !self.show_print_modal {
            return;
        }
        let mut open = self.show_print_modal;
        let report_txt = self
            .active_sheet()
            .map(|s| SoryntelApplication::format_reaction_sheet_tsv(s, self.decimals))
            .unwrap_or_default();
        egui::Window::new(t!("review.041"))
            .open(&mut open)
            .default_size([680.0, 480.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(t!("review.042")).clicked() {
                        ctx.copy_text(report_txt.clone());
                        self.status_msg = "Reporte copiado al portapapeles".into();
                        self.status_ok = true;
                    }
                    if ui.button(t!("review.043")).clicked() {
                        self.choose_export("xlsx");
                    }
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut report_txt.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        self.show_print_modal = open;
    }

    fn render_help_modal(&mut self, ctx: &egui::Context) {
        if !self.show_help_modal {
            return;
        }
        let mut open = self.show_help_modal;
        egui::Window::new(t!("review.044"))
            .open(&mut open)
            .default_size([560.0, 420.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(t!("review.045"));
                    ui.label("• Una sustancia (10.1): introduzca una fórmula, p. ej., Al2O3, Fe o SO4(-2a), para tabular y representar Cp, H, S y G frente a la temperatura.");
                    ui.label("• Formatos Normal y Delta: Normal muestra H(T), S(T), Cp(T) y G(T) de la especie. Delta resta los elementos de referencia para calcular propiedades de formación e incluye el balance de carga de iones acuosos mediante H2(g)/H(+a).");
                    ui.label("• Extrapolación de Criss–Cobble: estima Cp de iones acuosos por encima de 25 °C cuando los coeficientes B, C y D de Kelley son cero. Intervalo recomendado: 25–300 °C con paso de 25 °C. Revise el alcance aproximado del modelo.");
                    ui.label("• Ecuaciones de reacción (10.3): introduzca una reacción química o una semirreacción iónica con e- y pulse Balancear para obtener los coeficientes estequiométricos.");
                    ui.label("• Interpretación (10.4): K > 1 favorece productos en términos de estados estándar; ΔH < 0 indica reacción exotérmica. Compruebe la convención de signo del potencial frente al electrodo estándar de hidrógeno.");
                    ui.label("• Plot All Data: Superimposes multiple collected sheets (S1..Sn) when they share the same calculation mode, units, and format.");
                });
            });
        self.show_help_modal = open;
    }
}

fn chart_tool_icon_btn(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    ui.add_sized(
        [28.0, 28.0],
        egui::Button::new(egui::RichText::new(icon).size(15.0))
            .fill(egui::Color32::WHITE)
            .rounding(4.0),
    )
    .on_hover_text(tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_db::ThermoDatabase;

    #[test]
    fn compact_reaction_ui_keeps_data_and_chart_accessible() {
        let db = ThermoDatabase::with_open_defaults();
        let mut state = ReactionModuleState::new(&db);
        let ctx = egui::Context::default();
        for size in [
            egui::vec2(480.0, 360.0),
            egui::vec2(360.0, 640.0),
            egui::vec2(1340.0, 820.0),
        ] {
            for chart in [false, true] {
                state.compact_chart = chart;
                for _ in 0..2 {
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |ctx| state.ui(ctx, &db),
                    );
                    assert!(!output.shapes.is_empty());
                }
                assert!(state.status_ok);
                assert!(state.active_response().is_some());
                if chart {
                    assert!(state.last_chart_size.y >= 240.0, "ventana={size:?}, lienzo={:?}", state.last_chart_size);
                    assert!(state.last_chart_size.x > 200.0);
                }
            }
        }
    }

    #[test]
    fn reaction_ui_renders_wide_and_compact_and_supports_rea8_and_fig6_preset() {
        let db = ThermoDatabase::with_open_defaults();
        let mut state = ReactionModuleState::new(&db);
        let ctx = egui::Context::default();

        // 1. Render in wide desktop viewport (1340x820)
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1340.0, 820.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                state.ui(ctx, &db);
            });
        }
        assert!(state.status_ok);
        assert_eq!(state.sheets.len(), 1);

        // 2. Load Fig. 6 3-sheet multi-reaction preset and render in compact viewport (680x520)
        state.load_fig6_multi_vapor_preset(&db);
        assert_eq!(state.sheets.len(), 3);
        assert!(state.plot_all_data);
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(680.0, 520.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                state.ui(ctx, &db);
            });
        }

        // 3. Test .rea8 save/load roundtrip and SVG chart generation
        let tmp_rea8 = "/tmp/soryntel_test_roundtrip.rea8";
        state.save_rea8(tmp_rea8);
        assert!(state.status_ok);
        state.sheets.clear();
        state.load_rea8(tmp_rea8);
        assert_eq!(state.sheets.len(), 3);
        let svg = state.build_chart_svg().expect("chart svg");
        assert!(svg.contains("<svg") && svg.contains("Mg = Mg(g)"));
    }
}
