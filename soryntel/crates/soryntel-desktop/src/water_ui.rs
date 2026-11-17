//! Module 13 UI — Water / Steam Pressure & Temperature Calculator (`PTCalc`).
//!
//! Layout and behavior matching HSC6 `Fig. 1`–`Fig. 4`:
//! - Classic menu bar: `File | Edit | Format | Show | Set | Help`
//! - Left Panel: `Water / Steam Calculator` (`Amount`, `MW`, `Data Limits`),
//!   5-column Excel table (`Label | Point 1 | Point 2 | Delta | Unit`) with
//!   red inputs / black auto-calculated saturation coordinates / soft yellow Delta H & S,
//!   `Exergy` Szargut rows & checkbox, and bottom `Exit | Triple Point | Critical Point` + `Outotec`.
//! - Right Panel: Interactive `P [bar]` (log10) vs `T [°C]` phase diagram with
//!   `Gas Steam`, `Liquid Water`, `Solid Ice`, `Triple point`, `Critical point`,
//!   drag-and-drop `[1]` (yellow active) and `[2]` (white inactive) markers, and red connector line.

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::{
    balance::{h2o_saturation_pressure_bar, H2O_PC_BAR, H2O_TC_K},
    water::{
        sat_pressure_gs_bar, sat_temp_gl_k, sat_temp_ls_k, WaterAmountUnit, WaterCalcRequest,
        WaterCalcResult, WaterDensityUnit, WaterDiagramCurves, WaterEnergyUnit, WaterPhaseMode,
        WaterPressureUnit, WaterTempUnit, H2O_MOL_WEIGHT, H2O_PT_BAR, H2O_TT_K,
    },
    evaluate_species_si, SpeciesCatalogPort,
};

use crate::excel_grid::{EXCEL_GRID_BORDER, EXCEL_HEADER_BORDER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWaterPoint {
    Point1,
    Point2,
}

pub struct WaterModuleState {
    pub req: WaterCalcRequest,
    pub result: WaterCalcResult,
    pub curves: WaterDiagramCurves,
    pub active_point: ActiveWaterPoint,
    pub show_line: bool,
    pub show_exergy: bool,
    pub show_ice_polymorphs: bool,
    pub decimal_places: usize,

    // Text buffers for smooth spreadsheet typing without cursor fighting
    pub amount_buf: String,
    pub t1_buf: String,
    pub p1_buf: String,
    pub mix1_buf: String,
    pub t2_buf: String,
    pub p2_buf: String,
    pub mix2_buf: String,

    // Optional Fig. 5 Cp vs T comparison modal + About modal
    pub show_cp_diagram_modal: bool,
    pub show_about_modal: bool,
    pub status_msg: String,
    pub input_error: Option<String>,
}

impl WaterModuleState {
    pub fn new(catalog: &dyn SpeciesCatalogPort) -> Self {
        let req = SoryntelApplication::default_water_request();
        let curves = SoryntelApplication::water_phase_curves();
        let result = SoryntelApplication::calculate_water(catalog, &req);
        let mut s = Self {
            amount_buf: format_clean(req.amount, 1),
            t1_buf: String::new(),
            p1_buf: String::new(),
            mix1_buf: String::new(),
            t2_buf: String::new(),
            p2_buf: String::new(),
            mix2_buf: String::new(),
            req,
            result,
            curves,
            active_point: ActiveWaterPoint::Point1,
            show_line: true,
            show_exergy: true,
            show_ice_polymorphs: true,
            decimal_places: 3,
            show_cp_diagram_modal: false,
            show_about_modal: false,
            input_error: None,
            status_msg: "Pulse o arrastre [1]/[2] en el diagrama p-T o edite las celdas de la izquierda."
                .to_string(),
        };
        s.sync_input_buffers();
        s
    }

    fn recalculate(&mut self, catalog: &dyn SpeciesCatalogPort) {
        match SoryntelApplication::try_calculate_water(catalog, &self.req) {
            Ok(result) => { self.result = result; self.input_error = None; }
            Err(error) => { self.input_error = Some(format!("{error}. Se conserva el último resultado válido.")); return; }
        }
        // Write back effective auto-resolved P or T when saturation is selected
        if self.result.point1.temp_is_auto {
            self.req.point1.temp = self.result.point1.temp_display;
        }
        if self.result.point1.pressure_is_auto {
            self.req.point1.pressure = self.result.point1.pressure_display;
        }
        if self.result.point2.temp_is_auto {
            self.req.point2.temp = self.result.point2.temp_display;
        }
        if self.result.point2.pressure_is_auto {
            self.req.point2.pressure = self.result.point2.pressure_display;
        }
        self.req.point1.phase = self.result.point1.resolved_phase;
        self.req.point2.phase = self.result.point2.resolved_phase;
        self.sync_input_buffers();
    }

    fn sync_input_buffers(&mut self) {
        let dp = self.decimal_places;
        self.t1_buf = format!("{:.*}", dp, self.result.point1.temp_display);
        self.p1_buf = format!("{:.*}", dp, self.result.point1.pressure_display);
        self.mix1_buf = format!("{:.*}", dp, self.req.point1.mixture_pct);
        self.t2_buf = format!("{:.*}", dp, self.result.point2.temp_display);
        self.p2_buf = format!("{:.*}", dp, self.result.point2.pressure_display);
        self.mix2_buf = format!("{:.*}", dp, self.req.point2.mixture_pct);
    }

    fn recalculate_preserving_focus(
        &mut self,
        ctx: &egui::Context,
        catalog: &dyn SpeciesCatalogPort,
    ) {
        match SoryntelApplication::try_calculate_water(catalog, &self.req) {
            Ok(result) => { self.result = result; self.input_error = None; }
            Err(error) => { self.input_error = Some(format!("{error}. Se conserva el último resultado válido.")); return; }
        }
        if self.result.point1.temp_is_auto {
            self.req.point1.temp = self.result.point1.temp_display;
        }
        if self.result.point1.pressure_is_auto {
            self.req.point1.pressure = self.result.point1.pressure_display;
        }
        if self.result.point2.temp_is_auto {
            self.req.point2.temp = self.result.point2.temp_display;
        }
        if self.result.point2.pressure_is_auto {
            self.req.point2.pressure = self.result.point2.pressure_display;
        }
        self.req.point1.phase = self.result.point1.resolved_phase;
        self.req.point2.phase = self.result.point2.resolved_phase;

        let dp = self.decimal_places;
        let focused = |id: &str| ctx.memory(|m| m.has_focus(egui::Id::new(id)));
        if !focused("wt_t1") {
            self.t1_buf = format!("{:.*}", dp, self.result.point1.temp_display);
        }
        if !focused("wt_p1") {
            self.p1_buf = format!("{:.*}", dp, self.result.point1.pressure_display);
        }
        if !focused("wt_mix1") {
            self.mix1_buf = format!("{:.*}", dp, self.req.point1.mixture_pct);
        }
        if !focused("wt_t2") {
            self.t2_buf = format!("{:.*}", dp, self.result.point2.temp_display);
        }
        if !focused("wt_p2") {
            self.p2_buf = format!("{:.*}", dp, self.result.point2.pressure_display);
        }
        if !focused("wt_mix2") {
            self.mix2_buf = format!("{:.*}", dp, self.req.point2.mixture_pct);
        }
    }

    fn set_temp_unit(&mut self, new_unit: WaterTempUnit, catalog: &dyn SpeciesCatalogPort) {
        if self.req.temp_unit == new_unit {
            return;
        }
        let t1_k = self.req.temp_unit.to_kelvin(self.req.point1.temp);
        let t2_k = self.req.temp_unit.to_kelvin(self.req.point2.temp);
        self.req.temp_unit = new_unit;
        self.req.point1.temp = new_unit.from_kelvin(t1_k);
        self.req.point2.temp = new_unit.from_kelvin(t2_k);
        self.recalculate(catalog);
    }

    fn set_pressure_unit(
        &mut self,
        new_unit: WaterPressureUnit,
        catalog: &dyn SpeciesCatalogPort,
    ) {
        if self.req.pressure_unit == new_unit {
            return;
        }
        let p1_b = self.req.pressure_unit.to_bar(self.req.point1.pressure);
        let p2_b = self.req.pressure_unit.to_bar(self.req.point2.pressure);
        self.req.pressure_unit = new_unit;
        self.req.point1.pressure = new_unit.from_bar(p1_b);
        self.req.point2.pressure = new_unit.from_bar(p2_b);
        self.recalculate(catalog);
    }

    fn set_active_to_triple_point(&mut self, catalog: &dyn SpeciesCatalogPort) {
        let t_disp = self.req.temp_unit.from_kelvin(H2O_TT_K);
        let p_disp = self.req.pressure_unit.from_bar(H2O_PT_BAR);
        let target = match self.active_point {
            ActiveWaterPoint::Point1 => &mut self.req.point1,
            ActiveWaterPoint::Point2 => &mut self.req.point2,
        };
        target.temp = t_disp;
        target.pressure = p_disp;
        target.phase = WaterPhaseMode::Liquid;
        target.auto_phase = false;
        self.recalculate(catalog);
    }

    fn set_active_to_critical_point(&mut self, catalog: &dyn SpeciesCatalogPort) {
        let t_disp = self.req.temp_unit.from_kelvin(H2O_TC_K);
        let p_disp = self.req.pressure_unit.from_bar(H2O_PC_BAR);
        let target = match self.active_point {
            ActiveWaterPoint::Point1 => &mut self.req.point1,
            ActiveWaterPoint::Point2 => &mut self.req.point2,
        };
        target.temp = t_disp;
        target.pressure = p_disp;
        target.phase = WaterPhaseMode::Liquid;
        target.auto_phase = false;
        self.recalculate(catalog);
    }

    fn load_example_preset(&mut self, id: u8, catalog: &dyn SpeciesCatalogPort) {
        self.req.temp_unit = WaterTempUnit::Celsius;
        self.req.pressure_unit = WaterPressureUnit::Bar;
        self.req.density_unit = WaterDensityUnit::KgPerM3;
        self.req.amount_unit = WaterAmountUnit::Kg;
        match id {
            1 => {
                // Fig. 1: 180 °C Gas -> Triple Point Liquid
                self.req = SoryntelApplication::default_water_request();
                self.amount_buf = "1.0".into();
                self.active_point = ActiveWaterPoint::Point1;
                self.show_line = true;
            }
            2 => {
                // Fig. 2: Ice Melting at 500 bar (0% -> 90% liquid)
                self.req.amount = 1.0;
                self.amount_buf = "1.0".into();
                self.req.energy_unit = WaterEnergyUnit::KJ;
                self.req.point1.pressure = 500.0;
                self.req.point1.phase = WaterPhaseMode::SatLiquidSolid;
                self.req.point1.mixture_pct = 0.0;
                self.req.point1.auto_phase = false;
                self.req.point2.pressure = 500.0;
                self.req.point2.phase = WaterPhaseMode::SatLiquidSolid;
                self.req.point2.mixture_pct = 90.0;
                self.req.point2.auto_phase = false;
                self.active_point = ActiveWaterPoint::Point1;
            }
            3 => {
                // Fig. 3: Boiler Evaporation at 285 °C, 500000 kg, MWh
                self.req.amount = 500_000.0;
                self.amount_buf = "500000".into();
                self.req.energy_unit = WaterEnergyUnit::MWh;
                self.req.point1.temp = 285.0;
                self.req.point1.phase = WaterPhaseMode::SatGasLiquid;
                self.req.point1.mixture_pct = 0.0;
                self.req.point1.auto_phase = false;
                self.req.point2.temp = 285.0;
                self.req.point2.phase = WaterPhaseMode::SatGasLiquid;
                self.req.point2.mixture_pct = 10.0;
                self.req.point2.auto_phase = false;
                self.active_point = ActiveWaterPoint::Point1;
            }
            _ => {
                // Fig. 4: Steam Turbine 480 °C, 40 bar -> 30 °C Sat.(g,l) 91%
                self.req.amount = 10.0;
                self.amount_buf = "10".into();
                self.req.energy_unit = WaterEnergyUnit::KJ;
                self.req.point1.temp = 480.0;
                self.req.point1.pressure = 40.0;
                self.req.point1.phase = WaterPhaseMode::Gas;
                self.req.point1.auto_phase = false;
                self.req.point2.temp = 30.0;
                self.req.point2.phase = WaterPhaseMode::SatGasLiquid;
                self.req.point2.mixture_pct = 91.0;
                self.req.point2.auto_phase = false;
                self.active_point = ActiveWaterPoint::Point1;
                self.show_line = true;
            }
        }
        self.recalculate(catalog);
    }

    fn export_csv(&mut self) {
        if self.input_error.is_some() {
            self.status_msg = "Corrija los datos antes de exportar.".into();
            return;
        }
        let dp = self.decimal_places;
        let tu = self.req.temp_unit.label();
        let pu = self.req.pressure_unit.label();
        let eu = self.req.energy_unit.label();
        let su = self.req.energy_unit.entropy_label(self.req.temp_unit);
        let cpu = self.req.energy_unit.cp_label(self.req.amount_unit);
        let du = self.req.density_unit.label();
        let r = &self.result;
        let csv = format!(
            "Property,Point 1,Point 2,Delta,Unit\n\
             Temp.,{:.dp$},{:.dp$},{:.dp$},{tu}\n\
             Pressure,{:.dp$},{:.dp$},{:.dp$},{pu}\n\
             Phase,{},{},,\n\
             H,{:.dp$},{:.dp$},{:.dp$},{eu}\n\
             S,{:.dp$},{:.dp$},{:.dp$},{su}\n\
             H specific,{:.dp$},{:.dp$},{:.dp$},{eu}\n\
             G,{:.dp$},{:.dp$},{:.dp$},{eu}\n\
             Cp,{:.dp$},{:.dp$},{:.dp$},{cpu}\n\
             Density,{:.dp$},{:.dp$},{:.dp$},{du}\n\
             Exergy Ph,{:.dp$},{:.dp$},{:.dp$},{eu}\n\
             Exergy Ch,{:.dp$},{:.dp$},{:.dp$},{eu}\n\
             Exergy Tot,{:.dp$},{:.dp$},{:.dp$},{eu}\n",
            r.point1.temp_display,
            r.point2.temp_display,
            r.delta_temp,
            r.point1.pressure_display,
            r.point2.pressure_display,
            r.delta_pressure,
            r.point1.resolved_phase.label(),
            r.point2.resolved_phase.label(),
            r.point1.h,
            r.point2.h,
            r.delta_h,
            r.point1.s,
            r.point2.s,
            r.delta_s,
            r.point1.h_specific,
            r.point2.h_specific,
            r.delta_h_specific,
            r.point1.g,
            r.point2.g,
            r.delta_g,
            r.point1.cp,
            r.point2.cp,
            r.delta_cp,
            r.point1.density,
            r.point2.density,
            r.delta_density,
            r.point1.exergy_ph,
            r.point2.exergy_ph,
            r.delta_exergy_ph,
            r.point1.exergy_ch,
            r.point2.exergy_ch,
            r.delta_exergy_ch,
            r.point1.exergy_tot,
            r.point2.exergy_tot,
            r.delta_exergy_tot,
        );
        let path = crate::ui_common::temp_file_path("soryntel_water_calc.csv");
        match crate::storage::write_atomic(&path, csv) {
            Ok(_) => self.status_msg = format!("Saved Water calculation CSV to {path}"),
            Err(e) => self.status_msg = format!("CSV export error: {e}"),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, catalog: &dyn SpeciesCatalogPort) {
        egui::TopBottomPanel::top("water_model_notice").show(ctx, |ui| {
            ui.label("Modelo aproximado de agua y vapor. La correlación de saturación procede de IAPWS; las demás propiedades no implementan IF97 completo.");
        });
        // 1. Classic Top Menu Bar: File | Edit | Format | Show | Set | Help (Fig. 1)
        egui::TopBottomPanel::top("water_menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(238, 240, 244))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 2.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("Preset 1: 180 °C Steam → Triple Point (Fig. 1)").clicked() {
                            self.load_example_preset(1, catalog);
                            ui.close_menu();
                        }
                        if ui.button("Preset 2: Ice Melting at 500 bar (Fig. 2)").clicked() {
                            self.load_example_preset(2, catalog);
                            ui.close_menu();
                        }
                        if ui.button("Preset 3: Boiler Evaporation 285 °C (Fig. 3)").clicked() {
                            self.load_example_preset(3, catalog);
                            ui.close_menu();
                        }
                        if ui.button("Preset 4: Steam Turbine 480 °C → 30 °C (Fig. 4)").clicked() {
                            self.load_example_preset(4, catalog);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export CSV (/tmp/soryntel_water_calc.csv)").clicked() {
                            self.export_csv();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.006"), |ui| {
                        if ui.button("Point 2 = Triple Point (Steam Table Ref)").clicked() {
                            self.active_point = ActiveWaterPoint::Point2;
                            self.set_active_to_triple_point(catalog);
                            ui.close_menu();
                        }
                        if ui.button("Point 1 = Triple Point").clicked() {
                            self.active_point = ActiveWaterPoint::Point1;
                            self.set_active_to_triple_point(catalog);
                            ui.close_menu();
                        }
                        if ui.button("Swap Point 1 ↔ Point 2").clicked() {
                            std::mem::swap(&mut self.req.point1, &mut self.req.point2);
                            self.recalculate(catalog);
                            ui.close_menu();
                        }
                        if let Some(iso) = &self.result.isentropic_summary.clone() {
                            if let Some(mix) = iso.target_mixture_pct {
                                if ui
                                    .button(format!("Apply Isentropic Point 2 Mixture = {mix:.2}%"))
                                    .clicked()
                                {
                                    self.req.point2.mixture_pct = mix;
                                    self.recalculate(catalog);
                                    ui.close_menu();
                                }
                            }
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.019"), |ui| {
                        for dp in [2usize, 3, 4] {
                            if ui
                                .selectable_label(
                                    self.decimal_places == dp,
                                    format!("{dp} Decimal Places"),
                                )
                                .clicked()
                            {
                                self.decimal_places = dp;
                                self.sync_input_buffers();
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.103"), |ui| {
                        ui.checkbox(&mut self.show_line, "Line between Point 1 and Point 2");
                        ui.checkbox(&mut self.show_exergy, "Exergy Rows (Ph / Ch / Tot)");
                        ui.checkbox(
                            &mut self.show_ice_polymorphs,
                            "High-Pressure Ice Polymorph Lines",
                        );
                        ui.separator();
                        if ui.button("Cp vs T Steam Curves (Fig. 5)...").clicked() {
                            self.show_cp_diagram_modal = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.104"), |ui| {
                        if ui
                            .selectable_label(
                                self.active_point == ActiveWaterPoint::Point1,
                                "Active Point = Point 1",
                            )
                            .clicked()
                        {
                            self.active_point = ActiveWaterPoint::Point1;
                            ui.close_menu();
                        }
                        if ui
                            .selectable_label(
                                self.active_point == ActiveWaterPoint::Point2,
                                "Active Point = Point 2",
                            )
                            .clicked()
                        {
                            self.active_point = ActiveWaterPoint::Point2;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button(crate::i18n::text("common.105")).clicked() {
                            self.show_about_modal = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // 2. Bottom Outotec Footer Bar (Fig. 1)
        egui::TopBottomPanel::bottom("water_footer_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 238, 242))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Soryntel")
                            .strong()
                            .size(15.0)
                            .color(egui::Color32::from_rgb(198, 52, 40)),
                    );
                    ui.separator();
                    let note = self.input_error.as_deref().or(self
                        .result
                        .point1
                        .warning
                        .as_deref()
                        .or(self.result.point2.warning.as_deref()))
                        .unwrap_or(&self.status_msg);
                    ui.label(
                        egui::RichText::new(note)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(72, 78, 90)),
                    );
                });
            });

        // 3. Left Worksheet Panel (~58% width clamped cleanly, scrollable if viewport is short)
        let left_w = (ctx.screen_rect().width() * 0.56).clamp(430.0, 610.0);
        egui::SidePanel::left("water_left_worksheet")
            .exact_width(left_w)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 241, 245))
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .id_salt("water_left_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_left_worksheet(ui, catalog);
                    });
            });

        // 4. Right Interactive P-T Phase Diagram (Fig. 1-4)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 241, 245))
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                self.render_right_pt_diagram(ui, catalog);
            });

        self.render_modals(ctx, catalog);
    }

    fn render_left_worksheet(&mut self, ui: &mut egui::Ui, catalog: &dyn SpeciesCatalogPort) {
        let mut needs_recalc = false;

        // Top Header Card matching Fig. 1
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(242, 243, 246))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 210, 218)))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(crate::i18n::text("review.095"))
                                    .strong()
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(26, 28, 34)),
                            );
                            ui.add_space(28.0);
                            ui.label(
                                egui::RichText::new("H2O")
                                    .strong()
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(122, 152, 188)),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(crate::i18n::text("common.017")).size(12.0));
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.amount_buf)
                                    .desired_width(82.0)
                                    .horizontal_align(egui::Align::Center)
                                    .text_color(egui::Color32::from_rgb(190, 20, 20)),
                            );
                            if resp.changed() {
                                if let Ok(v) = self.amount_buf.trim().parse::<f64>() {
                                    if v > 0.0 {
                                        self.req.amount = v;
                                        needs_recalc = true;
                                    }
                                }
                            }
                            egui::ComboBox::from_id_salt("water_amt_unit")
                                .width(64.0)
                                .selected_text(self.req.amount_unit.label())
                                .show_ui(ui, |ui| {
                                    for u in [WaterAmountUnit::Kg, WaterAmountUnit::Kmol] {
                                        if ui
                                            .selectable_value(
                                                &mut self.req.amount_unit,
                                                u,
                                                u.label(),
                                            )
                                            .clicked()
                                        {
                                            needs_recalc = true;
                                        }
                                    }
                                });
                        });
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("MW").strong().size(12.0));
                            ui.add_space(22.0);
                            let mut mw_s = format!("{H2O_MOL_WEIGHT:.5}");
                            ui.add_enabled(
                                false,
                                egui::TextEdit::singleline(&mut mw_s)
                                    .desired_width(82.0)
                                    .horizontal_align(egui::Align::Center),
                            );
                            ui.label(egui::RichText::new("kg/kmol").strong().size(11.5));
                        });
                    });

                    ui.add_space(12.0);
                    // Data Limits Frame (Fig. 1)
                    egui::Frame::none()
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 210, 218)))
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(crate::i18n::text("review.096"))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(56, 60, 68)),
                            );
                            let t_lim = match self.req.temp_unit {
                                WaterTempUnit::Celsius => "T = -270.15 ... 2000 °C",
                                WaterTempUnit::Kelvin => "T = 3.00 ... 2273.15 K",
                            };
                            let p_lim = match self.req.pressure_unit {
                                WaterPressureUnit::Bar => "P = 0.001 ... 1000 bar",
                                WaterPressureUnit::MPa => "P = 0.0001 ... 100 MPa",
                            };
                            ui.label(egui::RichText::new(t_lim).size(11.5));
                            ui.label(egui::RichText::new(p_lim).size(11.5));
                        });
                });
            });

        ui.add_space(8.0);

        // Main 5-Column Worksheet Grid: Label | Point 1 | Point 2 | Delta | Unit
        let w_lbl = 98.0;
        let w_val = 104.0;
        let w_unit = 92.0;
        let dp = self.decimal_places;

        let hdr_bg = egui::Color32::from_rgb(248, 248, 250);
        let input_bg = egui::Color32::WHITE;
        let calc_bg = egui::Color32::from_rgb(233, 232, 223); // warm beige-grey HSC6 Fig. 1 result cells
        let yellow_bg = egui::Color32::from_rgb(254, 252, 224); // soft cream yellow Delta H/S
        let red_txt = egui::Color32::from_rgb(190, 24, 24);
        let black_txt = egui::Color32::from_rgb(20, 22, 26);

        let r_h = 24.0;

        // Header Row
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(ui, "", w_lbl, r_h, hdr_bg, black_txt, true, true, true);
            let p1_bg = if self.active_point == ActiveWaterPoint::Point1 {
                egui::Color32::from_rgb(254, 249, 195)
            } else {
                hdr_bg
            };
            let p2_bg = if self.active_point == ActiveWaterPoint::Point2 {
                egui::Color32::from_rgb(254, 249, 195)
            } else {
                hdr_bg
            };
            if draw_cell_btn(ui, "Point 1", w_val, r_h, p1_bg, black_txt).clicked() {
                self.active_point = ActiveWaterPoint::Point1;
            }
            if draw_cell_btn(ui, "Point 2", w_val, r_h, p2_bg, black_txt).clicked() {
                self.active_point = ActiveWaterPoint::Point2;
            }
            draw_cell_text(ui, "Delta", w_val, r_h, hdr_bg, black_txt, true, true, true);
            draw_cell_text(ui, "Unit", w_unit, r_h, hdr_bg, black_txt, true, true, true);
        });

        // Row 1: Temp.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(ui, "Temp.", w_lbl, r_h, hdr_bg, black_txt, true, true, false);
            let c1 = if self.result.point1.temp_is_auto {
                black_txt
            } else {
                red_txt
            };
            let bg1 = if self.result.point1.temp_is_auto {
                calc_bg
            } else {
                input_bg
            };
            if draw_edit_cell(
                ui,
                "wt_t1",
                &mut self.t1_buf,
                w_val,
                r_h,
                bg1,
                c1,
                !self.result.point1.temp_is_auto,
            ) {
                if let Ok(v) = self.t1_buf.trim().parse::<f64>() {
                    self.req.point1.temp = v;
                    needs_recalc = true;
                }
            }
            let c2 = if self.result.point2.temp_is_auto {
                black_txt
            } else {
                red_txt
            };
            let bg2 = if self.result.point2.temp_is_auto {
                calc_bg
            } else {
                input_bg
            };
            if draw_edit_cell(
                ui,
                "wt_t2",
                &mut self.t2_buf,
                w_val,
                r_h,
                bg2,
                c2,
                !self.result.point2.temp_is_auto,
            ) {
                if let Ok(v) = self.t2_buf.trim().parse::<f64>() {
                    self.req.point2.temp = v;
                    needs_recalc = true;
                }
            }
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.delta_temp),
                w_val,
                r_h,
                hdr_bg,
                black_txt,
                false,
                false,
                true,
            );
            egui::Frame::none()
                .fill(hdr_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_unit - 2.0, r_h - 2.0));
                    let mut u = self.req.temp_unit;
                    egui::ComboBox::from_id_salt("wt_t_unit")
                        .width(w_unit - 6.0)
                        .selected_text(u.label())
                        .show_ui(ui, |ui| {
                            for opt in [WaterTempUnit::Celsius, WaterTempUnit::Kelvin] {
                                if ui.selectable_value(&mut u, opt, opt.label()).clicked() {
                                    self.set_temp_unit(opt, catalog);
                                }
                            }
                        });
                });
        });

        // Row 2: Pressure
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(
                ui,
                "Pressure",
                w_lbl,
                r_h,
                hdr_bg,
                black_txt,
                true,
                true,
                false,
            );
            let p_c1 = if self.result.point1.pressure_is_auto {
                black_txt
            } else {
                red_txt
            };
            let p_bg1 = if self.result.point1.pressure_is_auto {
                calc_bg
            } else {
                input_bg
            };
            if draw_edit_cell(
                ui,
                "wt_p1",
                &mut self.p1_buf,
                w_val,
                r_h,
                p_bg1,
                p_c1,
                true,
            ) {
                if let Ok(v) = self.p1_buf.trim().parse::<f64>() {
                    if v > 0.0 {
                        if self.req.point1.phase == WaterPhaseMode::SatGasLiquid {
                            let p_bar = self.req.pressure_unit.to_bar(v);
                            let t_k = sat_temp_gl_k(p_bar);
                            self.req.point1.temp = self.req.temp_unit.from_kelvin(t_k);
                        }
                        self.req.point1.pressure = v;
                        needs_recalc = true;
                    }
                }
            }
            let p_c2 = if self.result.point2.pressure_is_auto {
                black_txt
            } else {
                red_txt
            };
            let p_bg2 = if self.result.point2.pressure_is_auto {
                calc_bg
            } else {
                input_bg
            };
            if draw_edit_cell(
                ui,
                "wt_p2",
                &mut self.p2_buf,
                w_val,
                r_h,
                p_bg2,
                p_c2,
                true,
            ) {
                if let Ok(v) = self.p2_buf.trim().parse::<f64>() {
                    if v > 0.0 {
                        if self.req.point2.phase == WaterPhaseMode::SatGasLiquid {
                            let p_bar = self.req.pressure_unit.to_bar(v);
                            let t_k = sat_temp_gl_k(p_bar);
                            self.req.point2.temp = self.req.temp_unit.from_kelvin(t_k);
                        }
                        self.req.point2.pressure = v;
                        needs_recalc = true;
                    }
                }
            }
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.delta_pressure),
                w_val,
                r_h,
                hdr_bg,
                black_txt,
                false,
                false,
                true,
            );
            egui::Frame::none()
                .fill(hdr_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_unit - 2.0, r_h - 2.0));
                    let mut u = self.req.pressure_unit;
                    egui::ComboBox::from_id_salt("wt_p_unit")
                        .width(w_unit - 6.0)
                        .selected_text(u.label())
                        .show_ui(ui, |ui| {
                            for opt in [WaterPressureUnit::Bar, WaterPressureUnit::MPa] {
                                if ui.selectable_value(&mut u, opt, opt.label()).clicked() {
                                    self.set_pressure_unit(opt, catalog);
                                }
                            }
                        });
                });
        });

        // Row 3: Phase
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(ui, "Phase", w_lbl, r_h, hdr_bg, black_txt, true, true, false);
            egui::Frame::none()
                .fill(input_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_val - 2.0, r_h - 2.0));
                    egui::ComboBox::from_id_salt("wt_ph1")
                        .width(w_val - 6.0)
                        .selected_text(self.req.point1.phase.label())
                        .show_ui(ui, |ui| {
                            for ph in WaterPhaseMode::ALL {
                                if ui
                                    .selectable_value(&mut self.req.point1.phase, ph, ph.label())
                                    .clicked()
                                {
                                    self.req.point1.auto_phase = false;
                                    needs_recalc = true;
                                }
                            }
                        });
                });
            egui::Frame::none()
                .fill(input_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_val - 2.0, r_h - 2.0));
                    egui::ComboBox::from_id_salt("wt_ph2")
                        .width(w_val - 6.0)
                        .selected_text(self.req.point2.phase.label())
                        .show_ui(ui, |ui| {
                            for ph in WaterPhaseMode::ALL {
                                if ui
                                    .selectable_value(&mut self.req.point2.phase, ph, ph.label())
                                    .clicked()
                                {
                                    self.req.point2.auto_phase = false;
                                    needs_recalc = true;
                                }
                            }
                        });
                });
            draw_cell_text(ui, "", w_val, r_h, calc_bg, black_txt, false, false, false);
            draw_cell_text(ui, "", w_unit, r_h, calc_bg, black_txt, false, false, false);
        });

        // Row 4: Mixture % (editable red only when phase is saturated)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(
                ui,
                "Mixture %",
                w_lbl,
                r_h,
                hdr_bg,
                black_txt,
                true,
                true,
                false,
            );
            if self.result.point1.resolved_phase.is_saturated() {
                if draw_edit_cell(
                    ui,
                    "wt_mix1",
                    &mut self.mix1_buf,
                    w_val,
                    r_h,
                    input_bg,
                    red_txt,
                    true,
                ) {
                    if let Ok(v) = self.mix1_buf.trim().parse::<f64>() {
                        self.req.point1.mixture_pct = v.clamp(0.0, 100.0);
                        needs_recalc = true;
                    }
                }
            } else {
                draw_cell_text(ui, "", w_val, r_h, input_bg, black_txt, false, false, false);
            }
            if self.result.point2.resolved_phase.is_saturated() {
                if draw_edit_cell(
                    ui,
                    "wt_mix2",
                    &mut self.mix2_buf,
                    w_val,
                    r_h,
                    input_bg,
                    red_txt,
                    true,
                ) {
                    if let Ok(v) = self.mix2_buf.trim().parse::<f64>() {
                        self.req.point2.mixture_pct = v.clamp(0.0, 100.0);
                        needs_recalc = true;
                    }
                }
            } else {
                draw_cell_text(ui, "", w_val, r_h, input_bg, black_txt, false, false, false);
            }
            draw_cell_text(ui, "", w_val, r_h, calc_bg, black_txt, false, false, false);
            draw_cell_text(ui, "", w_unit, r_h, calc_bg, black_txt, false, false, false);
        });

        // Dark divider bar under Mixture % matching Fig. 1
        let (sep_rect, _) =
            ui.allocate_exact_size(egui::vec2(w_lbl + 3.0 * w_val + w_unit, 2.0), egui::Sense::hover());
        ui.painter().rect_filled(sep_rect, 0.0, black_txt);

        // Result Rows
        let eu_lbl = self.req.energy_unit.label().to_string();
        let su_lbl = self.req.energy_unit.entropy_label(self.req.temp_unit);
        let cpu_lbl = self.req.energy_unit.cp_label(self.req.amount_unit);

        // Row 5: H (with Unit ComboBox)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(ui, "H", w_lbl, r_h, hdr_bg, black_txt, true, true, false);
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.point1.h),
                w_val,
                r_h,
                calc_bg,
                black_txt,
                false,
                false,
                true,
            );
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.point2.h),
                w_val,
                r_h,
                calc_bg,
                black_txt,
                false,
                false,
                true,
            );
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.delta_h),
                w_val,
                r_h,
                yellow_bg,
                black_txt,
                true,
                false,
                true,
            );
            egui::Frame::none()
                .fill(hdr_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_unit - 2.0, r_h - 2.0));
                    egui::ComboBox::from_id_salt("wt_e_unit")
                        .width(w_unit - 6.0)
                        .selected_text(&eu_lbl)
                        .show_ui(ui, |ui| {
                            for opt in WaterEnergyUnit::ALL {
                                if ui
                                    .selectable_value(
                                        &mut self.req.energy_unit,
                                        opt,
                                        opt.label(),
                                    )
                                    .clicked()
                                {
                                    needs_recalc = true;
                                }
                            }
                        });
                });
        });

        // Row 6: S (yellow Delta bold)
        draw_result_row(
            ui,
            "S",
            self.result.point1.s,
            self.result.point2.s,
            self.result.delta_s,
            &su_lbl,
            dp,
            w_lbl,
            w_val,
            w_unit,
            r_h,
            hdr_bg,
            calc_bg,
            yellow_bg,
            true,
        );

        // Rows 7..9: H specific, G, Cp
        draw_result_row(
            ui,
            "H specific",
            self.result.point1.h_specific,
            self.result.point2.h_specific,
            self.result.delta_h_specific,
            &eu_lbl,
            dp,
            w_lbl,
            w_val,
            w_unit,
            r_h,
            hdr_bg,
            calc_bg,
            calc_bg,
            false,
        );
        draw_result_row(
            ui,
            "G",
            self.result.point1.g,
            self.result.point2.g,
            self.result.delta_g,
            &eu_lbl,
            dp,
            w_lbl,
            w_val,
            w_unit,
            r_h,
            hdr_bg,
            calc_bg,
            calc_bg,
            false,
        );
        draw_result_row(
            ui,
            "Cp",
            self.result.point1.cp,
            self.result.point2.cp,
            self.result.delta_cp,
            &cpu_lbl,
            dp,
            w_lbl,
            w_val,
            w_unit,
            r_h,
            hdr_bg,
            calc_bg,
            calc_bg,
            false,
        );

        // Row 10: Density (with Unit ComboBox)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_cell_text(
                ui,
                "Density",
                w_lbl,
                r_h,
                hdr_bg,
                black_txt,
                true,
                true,
                false,
            );
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.point1.density),
                w_val,
                r_h,
                calc_bg,
                black_txt,
                false,
                false,
                true,
            );
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.point2.density),
                w_val,
                r_h,
                calc_bg,
                black_txt,
                false,
                false,
                true,
            );
            draw_cell_text(
                ui,
                &format!("{:.*}", dp, self.result.delta_density),
                w_val,
                r_h,
                calc_bg,
                black_txt,
                false,
                false,
                true,
            );
            egui::Frame::none()
                .fill(hdr_bg)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(w_unit - 2.0, r_h - 2.0));
                    egui::ComboBox::from_id_salt("wt_rho_unit")
                        .width(w_unit - 6.0)
                        .selected_text(self.req.density_unit.label())
                        .show_ui(ui, |ui| {
                            for opt in [WaterDensityUnit::KgPerM3, WaterDensityUnit::GPerCm3] {
                                if ui
                                    .selectable_value(
                                        &mut self.req.density_unit,
                                        opt,
                                        opt.label(),
                                    )
                                    .clicked()
                                {
                                    needs_recalc = true;
                                }
                            }
                        });
                });
        });

        // Rows 11..13: Exergy Ph, Exergy Ch, Exergy Tot
        if self.show_exergy {
            draw_result_row(
                ui,
                "Exergy Ph",
                self.result.point1.exergy_ph,
                self.result.point2.exergy_ph,
                self.result.delta_exergy_ph,
                &eu_lbl,
                dp,
                w_lbl,
                w_val,
                w_unit,
                r_h,
                hdr_bg,
                calc_bg,
                calc_bg,
                false,
            );
            draw_result_row(
                ui,
                "Exergy Ch",
                self.result.point1.exergy_ch,
                self.result.point2.exergy_ch,
                self.result.delta_exergy_ch,
                &eu_lbl,
                dp,
                w_lbl,
                w_val,
                w_unit,
                r_h,
                hdr_bg,
                calc_bg,
                calc_bg,
                false,
            );
            draw_result_row(
                ui,
                "Exergy Tot",
                self.result.point1.exergy_tot,
                self.result.point2.exergy_tot,
                self.result.delta_exergy_tot,
                &eu_lbl,
                dp,
                w_lbl,
                w_val,
                w_unit,
                r_h,
                hdr_bg,
                calc_bg,
                calc_bg,
                false,
            );
        }

        ui.add_space(6.0);
        // Exergy Checkbox & Note (Fig. 1)
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.show_exergy,
                egui::RichText::new(crate::i18n::text("review.097")).strong().size(12.0),
            );
            ui.label(
                egui::RichText::new(
                    crate::i18n::text("review.098"),
                )
                .size(10.5)
                .color(egui::Color32::from_rgb(56, 60, 68)),
            );
        });

        if let Some(iso) = &self.result.isentropic_summary {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(232, 244, 252))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(147, 197, 253)))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} = {:.3} (ΔHs = {:.3} {eu_lbl}, mix_s = {:.2}%)",
                            iso.mode_label,
                            iso.efficiency,
                            iso.dh_isentropic,
                            iso.target_mixture_pct.unwrap_or(100.0)
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(20, 60, 120)),
                    );
                });
        }

        ui.add_space(8.0);
        // Bottom 3 Action Buttons matching Fig. 1: Exit | Triple Point | Critical Point
        let btn_w = ((w_lbl + 3.0 * w_val + w_unit - 20.0) / 3.0).max(95.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 4.0);
            if hsc_action_btn(ui, "Exit", btn_w).clicked() {
                self.load_example_preset(1, catalog);
                self.status_msg = "Reset to default Triple Point Steam Table view.".to_string();
            }
            if hsc_action_btn(ui, "Triple Point", btn_w).clicked() {
                self.set_active_to_triple_point(catalog);
            }
            if hsc_action_btn(ui, "Critical Point", btn_w).clicked() {
                self.set_active_to_critical_point(catalog);
            }
        });

        if needs_recalc {
            self.recalculate_preserving_focus(ui.ctx(), catalog);
        }
    }

    fn render_right_pt_diagram(&mut self, ui: &mut egui::Ui, catalog: &dyn SpeciesCatalogPort) {
        let avail = ui.available_size();
        if avail.x < 160.0 || avail.y < 160.0 {
            ui.label(crate::i18n::text("review.099"));
            return;
        }

        let (outer_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
        let painter = ui.painter().with_clip_rect(outer_rect);

        // Chart margins for Y axis (`-100..500 °C`) on left and X axis (`0.001..1000+ bar`) at bottom
        let left_m = 44.0f32;
        let right_m = 14.0f32;
        let top_m = 16.0f32;
        let bot_m = 30.0f32;
        let plot_rect = egui::Rect::from_min_max(
            egui::pos2(outer_rect.left() + left_m, outer_rect.top() + top_m),
            egui::pos2(outer_rect.right() - right_m, outer_rect.bottom() - bot_m),
        );
        if !plot_rect.is_positive() {
            return;
        }

        // Axis bounds matching HSC6 Fig. 1–4: X = log10(P_bar) in [-3.0, 4.35] (0.001..22400 bar), Y = T_C in [-100, 500]
        let logp_min = -3.0_f64;
        let logp_max = 4.35_f64;
        let t_min = -100.0_f64;
        let t_max = 500.0_f64;

        let to_screen = |p_bar: f64, t_c: f64| -> egui::Pos2 {
            let lp = p_bar.clamp(1e-3, 22_000.0).log10();
            let nx = ((lp - logp_min) / (logp_max - logp_min)) as f32;
            let ny = ((t_c.clamp(t_min, t_max) - t_min) / (t_max - t_min)) as f32;
            egui::pos2(
                plot_rect.left() + nx * plot_rect.width(),
                plot_rect.bottom() - ny * plot_rect.height(),
            )
        };
        let to_world = |pos: egui::Pos2| -> (f64, f64) {
            let nx = ((pos.x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0) as f64;
            let ny = ((plot_rect.bottom() - pos.y) / plot_rect.height()).clamp(0.0, 1.0) as f64;
            let lp = logp_min + nx * (logp_max - logp_min);
            let p_bar = 10_f64.powf(lp).clamp(0.001, 1000.0);
            let t_c = t_min + ny * (t_max - t_min);
            (p_bar, t_c)
        };

        // 1. Background Phase Shading Polygons inside plot_rect (clipped strictly to plot_rect)
        let pp = painter.with_clip_rect(plot_rect);
        // Base sky-blue fill for Gas/Steam
        pp.rect_filled(plot_rect, 0.0, egui::Color32::from_rgb(200, 240, 248));

        // Supercritical / >Pc soft band on the right
        let x_pc = to_screen(H2O_PC_BAR, 0.0).x;
        pp.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x_pc, plot_rect.top()),
                plot_rect.right_bottom(),
            ),
            0.0,
            egui::Color32::from_rgb(218, 238, 250),
        );

        // Liquid Water horizontal convex strips: between Sat.(g,l) on left and high-P melting/right border
        let liq_fill = egui::Color32::from_rgb(94, 186, 242);
        let right_liq_edge = |t_c: f64| -> f64 {
            if t_c < -22.0 {
                2085.0
            } else if t_c < 95.0 {
                // Smooth HP liquidus boundary (~2085 at -22 °C to ~22000 at 95 °C)
                let f = ((t_c + 22.0) / 117.0).clamp(0.0, 1.0);
                2085.0 * (22_000.0 / 2085.0_f64).powf(f)
            } else {
                22_000.0
            }
        };
        for pair in self.curves.sat_gl.windows(2) {
            let [p0, t0] = pair[0];
            let [p1, t1] = pair[1];
            let q = vec![
                to_screen(p0, t0),
                to_screen(right_liq_edge(t0), t0),
                to_screen(right_liq_edge(t1), t1),
                to_screen(p1, t1),
            ];
            pp.add(egui::Shape::convex_polygon(q, liq_fill, egui::Stroke::NONE));
        }
        // Small triangular/trapezoid wedge between Sat.(l,s) and T = 0.01 °C
        for pair in self.curves.sat_ls.windows(2) {
            let [p0, t0] = pair[0];
            let [p1, t1] = pair[1];
            let r0 = right_liq_edge(t0).max(p0);
            let r1 = right_liq_edge(t1).max(p1);
            let q = vec![
                to_screen(p0, t0),
                to_screen(r0, t0),
                to_screen(r1, t1),
                to_screen(p1, t1),
            ];
            pp.add(egui::Shape::convex_polygon(q, liq_fill, egui::Stroke::NONE));
        }

        // Solid Ice vertical convex strips below Sat.(g,s) and Sat.(l,s) down to T = -100 °C
        let ice_fill = egui::Color32::from_rgb(244, 247, 252);
        for pair in self.curves.sat_gs.windows(2) {
            let [p0, t0] = pair[0];
            let [p1, t1] = pair[1];
            let q = vec![
                to_screen(p0, t0),
                to_screen(p1, t1),
                to_screen(p1, -100.0),
                to_screen(p0, -100.0),
            ];
            pp.add(egui::Shape::convex_polygon(q, ice_fill, egui::Stroke::NONE));
        }
        for pair in self.curves.sat_ls.windows(2) {
            let [p0, t0] = pair[0];
            let [p1, t1] = pair[1];
            let q = vec![
                to_screen(p0, t0),
                to_screen(p1, t1),
                to_screen(p1, -100.0),
                to_screen(p0, -100.0),
            ];
            pp.add(egui::Shape::convex_polygon(q, ice_fill, egui::Stroke::NONE));
        }
        // High-pressure ice region > 2085 bar up to HP liquidus
        let hp_liquidus_poly = [
            [2085.0, -22.0],
            [3500.0, -17.0],
            [6324.0, 0.16],
            [10_000.0, 32.0],
            [22_000.0, 95.0],
        ];
        for pair in hp_liquidus_poly.windows(2) {
            let [p0, t0] = pair[0];
            let [p1, t1] = pair[1];
            let q = vec![
                to_screen(p0, t0),
                to_screen(p1, t1),
                to_screen(p1, -100.0),
                to_screen(p0, -100.0),
            ];
            pp.add(egui::Shape::convex_polygon(q, ice_fill, egui::Stroke::NONE));
        }

        // 2. Gridlines (log P decades 0.001..1000 & T steps 0..400 °C)
        let grid_stroke = egui::Stroke::new(0.8, egui::Color32::from_rgba_unmultiplied(90, 110, 130, 65));
        for exp in -3..=3 {
            let p_val = 10_f64.powi(exp);
            let x = to_screen(p_val, 0.0).x;
            pp.line_segment(
                [egui::pos2(x, plot_rect.top()), egui::pos2(x, plot_rect.bottom())],
                grid_stroke,
            );
        }
        for t_tick in [0, 100, 200, 300, 400] {
            let y = to_screen(1.0, t_tick as f64).y;
            pp.line_segment(
                [egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y)],
                grid_stroke,
            );
        }

        // 3. Draw Solid Saturation Boundary Curves in crisp dark ink (Fig. 1-4)
        let curve_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(18, 22, 28));
        draw_polyline(&pp, &self.curves.sat_gs, &to_screen, curve_stroke);
        draw_polyline(&pp, &self.curves.sat_gl, &to_screen, curve_stroke);
        draw_polyline(&pp, &self.curves.sat_ls, &to_screen, curve_stroke);

        // High-pressure Ice III / V / VI / VII schematic polymorph boundaries (>2085 bar, matching Fig. 1-4)
        if self.show_ice_polymorphs {
            let hp_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(24, 28, 36));
            let hp_liquidus = [
                [2085.0, -22.0],
                [3500.0, -17.0],
                [6324.0, 0.16],
                [10_000.0, 32.0],
                [22_000.0, 95.0],
            ];
            draw_polyline(&pp, &hp_liquidus, &to_screen, hp_stroke);
            // Vertical sub-solidus ice polymorph dividers
            draw_polyline(&pp, &[[2085.0, -22.0], [2085.0, -100.0]], &to_screen, hp_stroke);
            draw_polyline(&pp, &[[3500.0, -17.0], [3200.0, -100.0]], &to_screen, hp_stroke);
            draw_polyline(&pp, &[[6324.0, 0.16], [6200.0, -100.0]], &to_screen, hp_stroke);
            draw_polyline(
                &pp,
                &[[22_000.0, 95.0], [22_000.0, 320.0]],
                &to_screen,
                hp_stroke,
            );
        }

        // 4. Phase Region Callouts & Triple/Critical Point Markers (Fig. 1-4)
        let tp_pos = to_screen(H2O_PT_BAR, 0.01);
        let cp_pos = to_screen(H2O_PC_BAR, 373.946);
        draw_callout_box(
            &pp,
            to_screen(0.18, 295.0),
            "Gas\nSteam",
            egui::Color32::from_rgba_unmultiplied(242, 251, 253, 215),
        );
        draw_callout_box(
            &pp,
            to_screen(220.0, 125.0),
            "Liquid\nWater",
            egui::Color32::from_rgba_unmultiplied(240, 248, 255, 225),
        );
        draw_callout_box(
            &pp,
            to_screen(3.5, -52.0),
            "Solid Ice",
            egui::Color32::WHITE,
        );
        draw_callout_box(
            &pp,
            tp_pos + egui::vec2(-6.0, -40.0),
            "Triple\npoint",
            egui::Color32::from_rgba_unmultiplied(238, 250, 255, 220),
        );
        draw_callout_box(
            &pp,
            cp_pos + egui::vec2(-2.0, -42.0),
            "Critical\npoint",
            egui::Color32::from_rgba_unmultiplied(238, 250, 255, 220),
        );

        // Red circles at Triple Point & Critical Point
        for pt in [tp_pos, cp_pos] {
            pp.circle_filled(pt, 4.5, egui::Color32::from_rgb(212, 36, 36));
            pp.circle_stroke(pt, 4.5, egui::Stroke::new(1.1, egui::Color32::BLACK));
        }

        // 5. Point 1 & Point 2 Markers + Red Line
        let p1_pos = to_screen(
            self.result.point1.pressure_bar,
            self.result.point1.temp_k - 273.15,
        );
        let p2_pos = to_screen(
            self.result.point2.pressure_bar,
            self.result.point2.temp_k - 273.15,
        );
        if self.show_line {
            pp.line_segment(
                [p1_pos, p2_pos],
                egui::Stroke::new(1.8, egui::Color32::from_rgb(208, 32, 32)),
            );
        }

        // Draw [1] and [2] marker badges + handle click selection / canvas drag-and-drop
        let box1 = marker_badge_rect(p1_pos, egui::vec2(18.0, 12.0));
        let box2 = marker_badge_rect(p2_pos, egui::vec2(18.0, 18.0));
        let r_b1 = ui.interact(box1, egui::Id::new("water_badge_1"), egui::Sense::click());
        let r_b2 = ui.interact(box2, egui::Id::new("water_badge_2"), egui::Sense::click());

        let plot_resp = ui.interact(
            plot_rect,
            egui::Id::new("water_pt_canvas"),
            egui::Sense::click_and_drag(),
        );
        if r_b1.clicked() {
            self.active_point = ActiveWaterPoint::Point1;
        } else if r_b2.clicked() {
            self.active_point = ActiveWaterPoint::Point2;
        } else if plot_resp.clicked() || plot_resp.dragged() {
            if let Some(pointer) = plot_resp.interact_pointer_pos() {
                let (p_bar, t_c) = to_world(pointer);
                let target = match self.active_point {
                    ActiveWaterPoint::Point1 => &mut self.req.point1,
                    ActiveWaterPoint::Point2 => &mut self.req.point2,
                };
                match target.phase {
                    WaterPhaseMode::SatGasLiquid => {
                        let tc_clamped = t_c.clamp(0.01, H2O_TC_K - 273.15);
                        target.temp = self.req.temp_unit.from_kelvin(tc_clamped + 273.15);
                        target.pressure = self
                            .req
                            .pressure_unit
                            .from_bar(h2o_saturation_pressure_bar(tc_clamped + 273.15));
                    }
                    WaterPhaseMode::SatGasSolid => {
                        let tc_clamped = t_c.clamp(-100.0, 0.01);
                        target.temp = self.req.temp_unit.from_kelvin(tc_clamped + 273.15);
                        target.pressure = self
                            .req
                            .pressure_unit
                            .from_bar(sat_pressure_gs_bar(tc_clamped + 273.15));
                    }
                    WaterPhaseMode::SatLiquidSolid => {
                        let p_clamped = p_bar.clamp(H2O_PT_BAR, 1000.0);
                        target.pressure = self.req.pressure_unit.from_bar(p_clamped);
                        target.temp = self
                            .req
                            .temp_unit
                            .from_kelvin(sat_temp_ls_k(p_clamped));
                    }
                    _ => {
                        target.temp = self.req.temp_unit.from_kelvin(t_c + 273.15);
                        target.pressure = self.req.pressure_unit.from_bar(p_bar);
                        target.auto_phase = true;
                    }
                }
                self.recalculate(catalog);
            }
        }

        draw_point_marker(
            &pp,
            p1_pos,
            box1,
            "1",
            self.active_point == ActiveWaterPoint::Point1,
        );
        draw_point_marker(
            &pp,
            p2_pos,
            box2,
            "2",
            self.active_point == ActiveWaterPoint::Point2,
        );

        // 6. Outer Border and Axis Ticks / Labels
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(130, 140, 156)),
        );

        // Y-axis ticks (-100..500 °C)
        for t_tick in [-100, 0, 100, 200, 300, 400, 500] {
            let y = to_screen(1.0, t_tick as f64).y;
            let disp = if self.req.temp_unit == WaterTempUnit::Kelvin {
                format!("{}", t_tick + 273)
            } else {
                format!("{t_tick}")
            };
            painter.text(
                egui::pos2(plot_rect.left() - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                disp,
                egui::FontId::proportional(10.5),
                egui::Color32::from_rgb(28, 32, 38),
            );
        }
        painter.text(
            egui::pos2(plot_rect.left() - 6.0, to_screen(1.0, 445.0).y),
            egui::Align2::RIGHT_CENTER,
            format!("T\n[{}]", self.req.temp_unit.label()),
            egui::FontId::proportional(10.5),
            egui::Color32::from_rgb(28, 32, 38),
        );

        // X-axis ticks (0.001..1000 bar)
        for (exp, lbl) in [
            (-3, "0.001"),
            (-2, "0.01"),
            (-1, "0.1"),
            (0, "1"),
            (1, "10"),
            (2, "100"),
            (3, "1000"),
        ] {
            let p_b = 10_f64.powi(exp);
            let x = to_screen(p_b, 0.0).x;
            let txt = if self.req.pressure_unit == WaterPressureUnit::MPa {
                format_clean(p_b * 0.1, 4)
            } else {
                lbl.to_string()
            };
            painter.text(
                egui::pos2(x, plot_rect.bottom() + 6.0),
                egui::Align2::CENTER_TOP,
                txt,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(28, 32, 38),
            );
        }
        painter.text(
            egui::pos2(plot_rect.right() - 2.0, plot_rect.bottom() + 6.0),
            egui::Align2::RIGHT_TOP,
            format!("P [{}]", self.req.pressure_unit.label()),
            egui::FontId::proportional(10.5),
            egui::Color32::from_rgb(28, 32, 38),
        );
    }

    fn render_modals(&mut self, ctx: &egui::Context, catalog: &dyn SpeciesCatalogPort) {
        if self.show_cp_diagram_modal {
            let mut open = true;
            egui::Window::new(crate::i18n::text("review.100"))
                .open(&mut open)
                .default_width(640.0)
                .default_height(420.0)
                .show(ctx, |ui| {
                    ui.label("Pressure-corrected steam Cp functions vs ideal-gas H2O(g) [J/(kg·K)] from Maindb6 & Water EOS:");
                    Plot::new("water_cp_fig5_plot")
                        .legend(Legend::default())
                        .x_axis_label(crate::i18n::text("common.090"))
                        .y_axis_label("Cp [J/(kg·K)]")
                        .show(ui, |plot_ui| {
                            let specs = [
                                ("H2O(g)", egui::Color32::from_rgb(20, 90, 40)),
                                ("H2O(0.1barg)", egui::Color32::from_rgb(28, 38, 120)),
                                ("H2O(1barg)", egui::Color32::from_rgb(165, 35, 165)),
                                ("H2O(5barg)", egui::Color32::from_rgb(185, 25, 25)),
                                ("H2O(10barg)", egui::Color32::from_rgb(0, 170, 195)),
                                ("H2O(20barg)", egui::Color32::from_rgb(35, 185, 45)),
                                ("H2O(30barg)", egui::Color32::from_rgb(25, 40, 180)),
                            ];
                            for (name, color) in specs {
                                if let Some(sp) = catalog.get_species(name) {
                                    let pts: PlotPoints = (0..=70)
                                        .filter_map(|i| {
                                            let tc = i as f64 * 5.0;
                                            let tk = tc + 273.15;
                                            evaluate_species_si(&sp, tk, false, false)
                                                .ok()
                                                .map(|(pt, _)| [tc, pt.cp / 0.01801528])
                                        })
                                        .collect();
                                    plot_ui.line(Line::new(pts).name(name).color(color).width(1.8));
                                }
                            }
                        });
                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        self.show_cp_diagram_modal = false;
                    }
                });
            if !open {
                self.show_cp_diagram_modal = false;
            }
        }

        if self.show_about_modal {
            let mut open = true;
            egui::Window::new(crate::i18n::text("review.101"))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(crate::i18n::text("review.102"));
                    ui.label("• Límites numéricos: T = -270,15 a 2000 °C y P = 0,001 a 1000 bar. Estos límites no garantizan exactitud científica.");
                    ui.label("• Fases: sólido (s), líquido (l), gas (g), mezclas saturadas (g,l), (g,s), (l,s) y fluido supercrítico.");
                    ui.label("• Punto triple: 0,01 °C y 0,006117 bar. Punto crítico: 373,946 °C y 220,64 bar.");
                    ui.label("• Modelo de exergía de Szargut con referencia a 25 °C y 1 bar.");
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        self.show_about_modal = false;
                    }
                });
            if !open {
                self.show_about_modal = false;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_result_row(
    ui: &mut egui::Ui,
    label: &str,
    v1: f64,
    v2: f64,
    dv: f64,
    unit: &str,
    dp: usize,
    w_lbl: f32,
    w_val: f32,
    w_unit: f32,
    r_h: f32,
    hdr_bg: egui::Color32,
    calc_bg: egui::Color32,
    delta_bg: egui::Color32,
    delta_bold: bool,
) {
    let fg = egui::Color32::from_rgb(20, 22, 26);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        draw_cell_text(ui, label, w_lbl, r_h, hdr_bg, fg, true, true, false);
        draw_cell_text(
            ui,
            &format!("{:.*}", dp, v1),
            w_val,
            r_h,
            calc_bg,
            fg,
            false,
            false,
            true,
        );
        draw_cell_text(
            ui,
            &format!("{:.*}", dp, v2),
            w_val,
            r_h,
            calc_bg,
            fg,
            false,
            false,
            true,
        );
        draw_cell_text(
            ui,
            &format!("{:.*}", dp, dv),
            w_val,
            r_h,
            delta_bg,
            fg,
            delta_bold,
            false,
            true,
        );
        draw_cell_text(ui, unit, w_unit, r_h, hdr_bg, fg, false, false, false);
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_cell_text(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
    right: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
        let clipped = p.with_clip_rect(rect.shrink2(egui::vec2(4.0, 1.0)));
        let _ = bold;
        let (pos, align) = if center {
            (rect.center(), egui::Align2::CENTER_CENTER)
        } else if right {
            (
                rect.right_center() - egui::vec2(6.0, 0.0),
                egui::Align2::RIGHT_CENTER,
            )
        } else {
            (
                rect.left_center() + egui::vec2(6.0, 0.0),
                egui::Align2::LEFT_CENTER,
            )
        };
        clipped.text(pos, align, text, egui::FontId::proportional(11.5), fg);
    }
}

fn draw_cell_btn(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.0),
            fg,
        );
    }
    resp.on_hover_text(crate::i18n::text("review.103"))
}

#[allow(clippy::too_many_arguments)]
fn draw_edit_cell(
    ui: &mut egui::Ui,
    id_salt: &str,
    buf: &mut String,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    enabled: bool,
) -> bool {
    let mut changed = false;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter()
        .rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(1.0)));
    let te = egui::TextEdit::singleline(buf)
        .id(egui::Id::new(id_salt))
        .desired_width(w - 4.0)
        .frame(false)
        .horizontal_align(egui::Align::RIGHT)
        .text_color(fg)
        .font(egui::FontId::proportional(11.5));
    let resp = child.add_enabled(enabled, te);
    if resp.changed() {
        changed = true;
    }
    changed
}

fn hsc_action_btn(ui: &mut egui::Ui, label: &str, w: f32) -> egui::Response {
    ui.add_sized(
        [w, 26.0],
        egui::Button::new(
            egui::RichText::new(label)
                .size(12.0)
                .color(egui::Color32::from_rgb(20, 24, 30)),
        )
        .fill(egui::Color32::from_rgb(232, 235, 241))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 175, 186)))
        .rounding(2.5),
    )
}

fn draw_polyline<F>(
    painter: &egui::Painter,
    pts: &[[f64; 2]],
    to_screen: &F,
    stroke: egui::Stroke,
) where
    F: Fn(f64, f64) -> egui::Pos2,
{
    let screen_pts: Vec<egui::Pos2> = pts.iter().map(|&[p, tc]| to_screen(p, tc)).collect();
    for pair in screen_pts.windows(2) {
        painter.line_segment([pair[0], pair[1]], stroke);
    }
}

fn draw_callout_box(painter: &egui::Painter, center: egui::Pos2, text: &str, bg: egui::Color32) {
    let lines: Vec<&str> = text.lines().collect();
    let max_chars = lines.iter().map(|l| l.len()).max().unwrap_or(4);
    let w = (max_chars as f32 * 6.8 + 14.0).max(46.0);
    let h = lines.len() as f32 * 14.0 + 8.0;
    let rect = egui::Rect::from_center_size(center, egui::vec2(w, h));
    painter.rect_filled(rect, 3.0, bg);
    painter.rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(0.8, egui::Color32::from_rgba_unmultiplied(120, 140, 160, 120)),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(11.5),
        egui::Color32::from_rgb(16, 68, 112),
    );
}

fn marker_badge_rect(dot: egui::Pos2, offset: egui::Vec2) -> egui::Rect {
    egui::Rect::from_center_size(dot + offset, egui::vec2(20.0, 20.0))
}

fn draw_point_marker(
    painter: &egui::Painter,
    dot: egui::Pos2,
    badge: egui::Rect,
    label: &str,
    active: bool,
) {
    painter.circle_filled(dot, 3.2, egui::Color32::from_rgb(180, 24, 24));
    painter.circle_stroke(dot, 3.2, egui::Stroke::new(1.0, egui::Color32::BLACK));
    let bg = if active {
        egui::Color32::from_rgb(254, 240, 138) // yellow active box (Fig. 1-4)
    } else {
        egui::Color32::WHITE // white inactive box (Fig. 1-4)
    };
    painter.rect_filled(badge, 1.5, bg);
    painter.rect_stroke(
        badge,
        1.5,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 118, 130)),
    );
    painter.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.5),
        egui::Color32::from_rgb(20, 22, 26),
    );
}

fn format_clean(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}
