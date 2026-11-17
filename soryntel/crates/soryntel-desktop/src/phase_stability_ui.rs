use std::collections::BTreeSet;

use eframe::egui;
use soryntel_db::ThermoDatabase as SpeciesDb;
use soryntel_thermo::{
    phase_stability::{
        calculate_phase_stability, AxisVariable, PhaseStabilityDiagram, PhaseStabilityInput,
        TemperatureUnit,
    },
    Phase,
};

use crate::excel_grid::{
    header_cell, row_num_cell, section_cell, text_cell, EXCEL_ALT_ROW_BG, EXCEL_GRID_BORDER,
    EXCEL_READONLY_BG,
};
use crate::ui_common::{
    draw_dashed_line, draw_rotated_y_title, format_tick, is_organic_formula, nice_ticks,
    render_chart_tabs, ChartTabAction, ELEMENT_GRID_4COL,
};

/// Curated HSC-inspired region fill palette (Fig. 2: dark gray, sky blue, leaf green, etc.).
const REGION_PALETTE: &[egui::Color32] = &[
    egui::Color32::from_rgb(92, 92, 96),
    egui::Color32::from_rgb(24, 152, 214),
    egui::Color32::from_rgb(108, 178, 68),
    egui::Color32::from_rgb(232, 168, 68),
    egui::Color32::from_rgb(176, 128, 210),
    egui::Color32::from_rgb(220, 108, 108),
    egui::Color32::from_rgb(94, 198, 188),
    egui::Color32::from_rgb(196, 186, 86),
    egui::Color32::from_rgb(142, 166, 216),
    egui::Color32::from_rgb(190, 136, 108),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Chart(usize),
    Table,
}

pub struct PhaseStabilityModuleState {
    selected_elements: Vec<String>,
    include_liquids: bool,
    include_organics: bool,

    available_species: Vec<String>,
    selected_species: BTreeSet<String>,
    available_gases: Vec<String>,

    x_axis_choice: String,
    y_axis_choice: String,
    x_min_str: String,
    x_max_str: String,
    y_min_str: String,
    y_max_str: String,
    auto_scale: bool,

    temp_unit: TemperatureUnit,
    fixed_temp_str: String,
    fixed_gas_choice: String,
    fixed_gas_p_str: String,

    show_colors: bool,
    create_new_page: bool,

    charts: Vec<(String, PhaseStabilityDiagram)>,
    active_tab: RightTab,
    last_error: Option<String>,
}

impl PhaseStabilityModuleState {
    pub fn new(db: &SpeciesDb) -> Self {
        let mut state = Self {
            selected_elements: vec!["Cu".into(), "S".into(), "O".into()],
            include_liquids: true,
            include_organics: false,
            available_species: Vec::new(),
            selected_species: BTreeSet::new(),
            available_gases: Vec::new(),
            x_axis_choice: "Temperature".into(),
            y_axis_choice: "SO2(g)".into(),
            x_min_str: "0".into(),
            x_max_str: "1000".into(),
            y_min_str: "-5".into(),
            y_max_str: "2".into(),
            auto_scale: false,
            temp_unit: TemperatureUnit::Celsius,
            fixed_temp_str: "300".into(),
            fixed_gas_choice: "O2(g)".into(),
            fixed_gas_p_str: "0.1".into(),
            show_colors: true,
            create_new_page: true,
            charts: Vec::new(),
            active_tab: RightTab::Chart(0),
            last_error: None,
        };
        state.refresh_species_and_gases(db);
        state.run_calculate(db);
        state
    }

    fn apply_cu_s_o_preset(&mut self, db: &SpeciesDb) {
        self.selected_elements = vec!["Cu".into(), "S".into(), "O".into()];
        self.include_liquids = false;
        self.include_organics = false;
        self.refresh_species_and_gases(db);
        self.x_axis_choice = "Temperature".into();
        self.y_axis_choice = pick_gas(&self.available_gases, "SO2(g)");
        self.fixed_gas_choice = pick_gas(&self.available_gases, "O2(g)");
        self.fixed_gas_p_str = "0.1".into();
        self.x_min_str = "0".into();
        self.x_max_str = "1000".into();
        self.y_min_str = "-5".into();
        self.y_max_str = "2".into();
        self.auto_scale = false;
        self.temp_unit = TemperatureUnit::Celsius;
        self.show_colors = true;
        self.run_calculate(db);
    }

    fn apply_ti_cl_o_preset(&mut self, db: &SpeciesDb) {
        self.selected_elements = vec!["Ti".into(), "Cl".into(), "O".into()];
        self.include_liquids = true;
        self.include_organics = false;
        self.refresh_species_and_gases(db);
        self.x_axis_choice = pick_gas(&self.available_gases, "Cl2(g)");
        self.y_axis_choice = pick_gas(&self.available_gases, "O2(g)");
        self.fixed_temp_str = "300".into();
        self.temp_unit = TemperatureUnit::Celsius;
        self.auto_scale = true;
        self.show_colors = true;
        self.run_calculate(db);
    }

    fn apply_v_cl_o_preset(&mut self, db: &SpeciesDb) {
        self.selected_elements = vec!["V".into(), "Cl".into(), "O".into()];
        self.include_liquids = true;
        self.include_organics = false;
        self.refresh_species_and_gases(db);
        self.x_axis_choice = pick_gas(&self.available_gases, "Cl2(g)");
        self.y_axis_choice = pick_gas(&self.available_gases, "O2(g)");
        self.x_min_str = "-40.00".into();
        self.x_max_str = "0.00".into();
        self.y_min_str = "-40.00".into();
        self.y_max_str = "0.00".into();
        self.fixed_temp_str = "500".into();
        self.temp_unit = TemperatureUnit::Celsius;
        self.auto_scale = false;
        self.show_colors = true;
        self.run_calculate(db);
    }

    fn refresh_species_and_gases(&mut self, db: &SpeciesDb) {
        self.available_species.clear();
        self.selected_species.clear();
        self.available_gases.clear();

        if self.selected_elements.len() != 3 {
            return;
        }

        let condensed = db.search_by_elements(
            &self.selected_elements,
            false,
            self.include_liquids,
            true,
            false,
            false,
            500,
        );
        for sp in condensed {
            if !self.include_organics && is_organic_formula(&sp.formula) {
                continue;
            }
            if !self.available_species.contains(&sp.formula) {
                self.selected_species.insert(sp.formula.clone());
                self.available_species.push(sp.formula);
            }
        }

        let gases = db.search_by_elements(
            &self.selected_elements,
            true,
            false,
            false,
            false,
            false,
            300,
        );
        for sp in gases {
            if sp.phase == Phase::Gas && !self.available_gases.contains(&sp.formula) {
                self.available_gases.push(sp.formula);
            }
        }
        self.available_gases.sort();

        if self.x_axis_choice != "Temperature"
            && !self.available_gases.contains(&self.x_axis_choice)
        {
            self.x_axis_choice = "Temperature".into();
        }
        if self.y_axis_choice != "Temperature"
            && !self.available_gases.contains(&self.y_axis_choice)
        {
            self.y_axis_choice = self
                .available_gases
                .first()
                .cloned()
                .unwrap_or_else(|| "O2(g)".into());
        }
        if !self.available_gases.contains(&self.fixed_gas_choice) {
            self.fixed_gas_choice = self
                .available_gases
                .iter()
                .find(|g| *g != &self.x_axis_choice && *g != &self.y_axis_choice)
                .cloned()
                .or_else(|| self.available_gases.first().cloned())
                .unwrap_or_else(|| "O2(g)".into());
        }
    }

    fn is_pp_mode(&self) -> bool {
        self.x_axis_choice != "Temperature" && self.y_axis_choice != "Temperature"
    }

    fn parse_axis(&self, choice: &str) -> AxisVariable {
        if choice == "Temperature" {
            AxisVariable::Temperature
        } else {
            AxisVariable::Gas(choice.to_string())
        }
    }

    fn run_calculate(&mut self, db: &SpeciesDb) {
        self.last_error = None;
        if self.selected_elements.len() != 3 {
            self.last_error = Some("Select exactly 3 elements.".into());
            return;
        }
        if self.selected_species.is_empty() {
            self.last_error = Some("Select at least one condensed species.".into());
            return;
        }

        let is_pp = self.is_pp_mode();
        let x_min = self.x_min_str.trim().parse::<f64>().unwrap_or(-45.0);
        let x_max = self.x_max_str.trim().parse::<f64>().unwrap_or(5.0);
        let y_min = self.y_min_str.trim().parse::<f64>().unwrap_or(-80.0);
        let y_max = self.y_max_str.trim().parse::<f64>().unwrap_or(5.0);

        let fixed_temp_val = match self.fixed_temp_str.trim().parse::<f64>() {
            Ok(v) => v,
            Err(_) if is_pp => {
                self.last_error = Some("Invalid fixed temperature.".into());
                return;
            }
            Err(_) => 300.0,
        };
        let fixed_temperature_k = self.temp_unit.to_kelvin(fixed_temp_val);

        let fixed_gas = if is_pp {
            None
        } else {
            let p_val = match self.fixed_gas_p_str.trim().parse::<f64>() {
                Ok(v) if v > 0.0 => v,
                _ => {
                    self.last_error =
                        Some("Fixed gas partial pressure must be a positive number (bar).".into());
                    return;
                }
            };
            Some((self.fixed_gas_choice.clone(), p_val))
        };

        let input = PhaseStabilityInput {
            elements: [
                self.selected_elements[0].clone(),
                self.selected_elements[1].clone(),
                self.selected_elements[2].clone(),
            ],
            selected_species: self.selected_species.iter().cloned().collect(),
            x_axis: self.parse_axis(&self.x_axis_choice),
            y_axis: self.parse_axis(&self.y_axis_choice),
            temp_unit: self.temp_unit,
            x_min,
            x_max,
            y_min,
            y_max,
            auto_scale: self.auto_scale,
            fixed_temperature_k,
            fixed_gas,
            include_liquids: self.include_liquids,
            include_organics: self.include_organics,
            nx: 120,
            ny: 120,
        };

        match calculate_phase_stability(db, &input) {
            Ok(diagram) => {
                if is_pp && self.auto_scale {
                    self.x_min_str = format!("{:.1}", diagram.x_min);
                    self.x_max_str = format!("{:.1}", diagram.x_max);
                    self.y_min_str = format!("{:.1}", diagram.y_min);
                    self.y_max_str = format!("{:.1}", diagram.y_max);
                }
                if self.create_new_page || self.charts.is_empty() {
                    let idx = self.charts.len();
                    let label = format!("Chart {}", idx + 1);
                    self.charts.push((label, diagram));
                    self.active_tab = RightTab::Chart(idx);
                } else {
                    let target = match self.active_tab {
                        RightTab::Chart(i) if i < self.charts.len() => i,
                        _ => self.charts.len() - 1,
                    };
                    self.charts[target].1 = diagram;
                    self.active_tab = RightTab::Chart(target);
                }
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &SpeciesDb) {
        egui::SidePanel::left("phase_stability_params")
            .resizable(true)
            .default_width(336.0)
            .min_width(250.0)
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_left_panel(ui, db);
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_right_panel(ui);
        });
    }

    fn render_left_panel(&mut self, ui: &mut egui::Ui, db: &SpeciesDb) {
        ui.horizontal(|ui| {
            ui.strong("Diagram Parameters");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("V-Cl-O").clicked() {
                    self.apply_v_cl_o_preset(db);
                }
                if ui.small_button("Ti-Cl-O").clicked() {
                    self.apply_ti_cl_o_preset(db);
                }
                if ui.small_button("Cu-S-O").clicked() {
                    self.apply_cu_s_o_preset(db);
                }
            });
        });
        ui.separator();

        // Top 2-column selector: Select 3 Elements | Select Species
        ui.columns(2, |cols| {
            // Left: Select 3 Elements (4-column grid)
            cols[0].label("Select 3 Elements");
            egui::Frame::group(cols[0].style())
                .fill(egui::Color32::WHITE)
                .inner_margin(egui::Margin::same(4.0))
                .show(&mut cols[0], |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("ps_elem_scroll")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::Grid::new("ps_elem_grid")
                                .spacing([2.0, 1.0])
                                .min_col_width(28.0)
                                .show(ui, |ui| {
                                    for (idx, &sym) in ELEMENT_GRID_4COL.iter().enumerate() {
                                        let is_sel =
                                            self.selected_elements.iter().any(|e| e == sym);
                                        let btn = egui::Button::new(
                                            egui::RichText::new(sym).size(11.0).color(if is_sel {
                                                egui::Color32::from_rgb(10, 45, 110)
                                            } else {
                                                egui::Color32::BLACK
                                            }),
                                        )
                                        .fill(if is_sel {
                                            egui::Color32::from_rgb(198, 222, 250)
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .frame(is_sel);
                                        if ui.add_sized([30.0, 16.0], btn).clicked() {
                                            if let Some(pos) =
                                                self.selected_elements.iter().position(|e| e == sym)
                                            {
                                                self.selected_elements.remove(pos);
                                            } else {
                                                if self.selected_elements.len() >= 3 {
                                                    self.selected_elements.remove(0);
                                                }
                                                self.selected_elements.push(sym.to_string());
                                            }
                                            self.refresh_species_and_gases(db);
                                        }
                                        if idx % 4 == 3 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                });

            cols[0].horizontal(|ui| {
                let mut changed = false;
                changed |= ui.checkbox(&mut self.include_liquids, "Liquids").changed();
                changed |= ui.checkbox(&mut self.include_organics, "Organics").changed();
                if changed {
                    self.refresh_species_and_gases(db);
                }
            });

            // Right: Select Species
            cols[1].label(crate::i18n::text("common.083"));
            egui::Frame::group(cols[1].style())
                .fill(egui::Color32::WHITE)
                .inner_margin(egui::Margin::same(4.0))
                .show(&mut cols[1], |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("ps_species_scroll")
                        .max_height(250.0)
                        .min_scrolled_height(220.0)
                        .show(ui, |ui| {
                            ui.set_min_width(120.0);
                            for sp in &self.available_species {
                                let selected = self.selected_species.contains(sp);
                                if ui.selectable_label(selected, sp).clicked() {
                                    if selected {
                                        self.selected_species.remove(sp);
                                    } else {
                                        self.selected_species.insert(sp.clone());
                                    }
                                }
                            }
                        });
                });

            let all_selected = !self.available_species.is_empty()
                && self.selected_species.len() == self.available_species.len();
            let toggle_label = if all_selected { "Clear All" } else { "Select All" };
            if cols[1]
                .add_sized([cols[1].available_width(), 22.0], egui::Button::new(toggle_label))
                .clicked()
            {
                if all_selected {
                    self.selected_species.clear();
                } else {
                    self.selected_species = self.available_species.iter().cloned().collect();
                }
            }
        });

        ui.add_space(6.0);

        // Diagram parameters box
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.strong("Diagram parameters");
                ui.add_space(4.0);

                let mut axis_options = vec!["Temperature".to_string()];
                axis_options.extend(self.available_gases.iter().cloned());

                let is_pp = self.is_pp_mode();
                let disable_minmax = is_pp && self.auto_scale;

                ui.columns(2, |cols| {
                    cols[0].label("X-axis variable");
                    egui::ComboBox::from_id_salt("ps_x_axis")
                        .width(cols[0].available_width() - 4.0)
                        .selected_text(&self.x_axis_choice)
                        .show_ui(&mut cols[0], |ui| {
                            for opt in &axis_options {
                                ui.selectable_value(&mut self.x_axis_choice, opt.clone(), opt);
                            }
                        });
                    let x_sub = self.parse_axis(&self.x_axis_choice).label(self.temp_unit);
                    cols[0].small(x_sub);
                    cols[0].horizontal(|ui| {
                        ui.label("Min");
                        ui.add_enabled(
                            !disable_minmax,
                            egui::TextEdit::singleline(&mut self.x_min_str).desired_width(58.0),
                        );
                    });
                    cols[0].horizontal(|ui| {
                        ui.label("Max");
                        ui.add_enabled(
                            !disable_minmax,
                            egui::TextEdit::singleline(&mut self.x_max_str).desired_width(58.0),
                        );
                    });

                    cols[1].label("Y-axis variable");
                    egui::ComboBox::from_id_salt("ps_y_axis")
                        .width(cols[1].available_width() - 4.0)
                        .selected_text(&self.y_axis_choice)
                        .show_ui(&mut cols[1], |ui| {
                            for opt in &axis_options {
                                ui.selectable_value(&mut self.y_axis_choice, opt.clone(), opt);
                            }
                        });
                    let y_sub = self.parse_axis(&self.y_axis_choice).label(self.temp_unit);
                    cols[1].small(y_sub);
                    cols[1].horizontal(|ui| {
                        ui.label("Min");
                        ui.add_enabled(
                            !disable_minmax,
                            egui::TextEdit::singleline(&mut self.y_min_str).desired_width(58.0),
                        );
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("Max");
                        ui.add_enabled(
                            !disable_minmax,
                            egui::TextEdit::singleline(&mut self.y_max_str).desired_width(58.0),
                        );
                    });
                });

                if is_pp {
                    ui.checkbox(&mut self.auto_scale, "Auto scale");
                }

                ui.add_space(4.0);
                ui.label("Fixed variable");
                if is_pp {
                    ui.horizontal(|ui| {
                        ui.add_enabled(false, egui::Button::new("Temperature"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fixed_temp_str)
                                .desired_width(64.0),
                        );
                        self.render_temp_unit_button(ui);
                    });
                } else {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("ps_fixed_gas")
                            .width(86.0)
                            .selected_text(&self.fixed_gas_choice)
                            .show_ui(ui, |ui| {
                                for g in &self.available_gases {
                                    ui.selectable_value(&mut self.fixed_gas_choice, g.clone(), g);
                                }
                            });
                        ui.add(
                            egui::TextEdit::singleline(&mut self.fixed_gas_p_str)
                                .desired_width(62.0),
                        );
                        ui.label("bar");
                        self.render_temp_unit_button(ui);
                    });
                }
            });

        ui.add_space(6.0);
        ui.checkbox(&mut self.show_colors, "Show colors");
        ui.checkbox(&mut self.create_new_page, "Create new page");

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .add_sized([110.0, 26.0], egui::Button::new("Show table"))
                .clicked()
            {
                if self.charts.is_empty() {
                    self.run_calculate(db);
                }
                self.active_tab = RightTab::Table;
            }
            if ui
                .add_sized([150.0, 26.0], egui::Button::new("Draw Diagram"))
                .clicked()
            {
                self.run_calculate(db);
            }
        });

        if let Some(err) = &self.last_error {
            ui.add_space(6.0);
            ui.colored_label(egui::Color32::from_rgb(180, 30, 30), err);
        }
    }

    fn render_temp_unit_button(&mut self, ui: &mut egui::Ui) {
        let label = match self.temp_unit {
            TemperatureUnit::Celsius => "°C",
            TemperatureUnit::Kelvin => "K",
        };
        if ui.add_sized([36.0, 20.0], egui::Button::new(label)).clicked() {
            let old = self.temp_unit;
            let new = match old {
                TemperatureUnit::Celsius => TemperatureUnit::Kelvin,
                TemperatureUnit::Kelvin => TemperatureUnit::Celsius,
            };
            self.temp_unit = new;
            if let Ok(v) = self.fixed_temp_str.trim().parse::<f64>() {
                let k = old.to_kelvin(v);
                self.fixed_temp_str = format!("{:.2}", new.from_kelvin(k));
            }
            if self.x_axis_choice == "Temperature" {
                if let Ok(v) = self.x_min_str.trim().parse::<f64>() {
                    self.x_min_str = format!("{:.2}", new.from_kelvin(old.to_kelvin(v)));
                }
                if let Ok(v) = self.x_max_str.trim().parse::<f64>() {
                    self.x_max_str = format!("{:.2}", new.from_kelvin(old.to_kelvin(v)));
                }
            }
            if self.y_axis_choice == "Temperature" {
                if let Ok(v) = self.y_min_str.trim().parse::<f64>() {
                    self.y_min_str = format!("{:.2}", new.from_kelvin(old.to_kelvin(v)));
                }
                if let Ok(v) = self.y_max_str.trim().parse::<f64>() {
                    self.y_max_str = format!("{:.2}", new.from_kelvin(old.to_kelvin(v)));
                }
            }
        }
    }

    fn render_right_panel(&mut self, ui: &mut egui::Ui) {
        let names: Vec<&str> = self.charts.iter().map(|(n, _)| n.as_str()).collect();
        let active_chart = match self.active_tab {
            RightTab::Chart(i) => Some(i),
            RightTab::Table => None,
        };
        match render_chart_tabs(ui, &names, active_chart, self.active_tab == RightTab::Table) {
            ChartTabAction::SelectChart(i) => self.active_tab = RightTab::Chart(i),
            ChartTabAction::CloseChart(i) => {
                self.charts.remove(i);
                self.active_tab = RightTab::Chart(i.min(self.charts.len().saturating_sub(1)));
            }
            ChartTabAction::SelectTable => self.active_tab = RightTab::Table,
            ChartTabAction::None => {}
        }
        ui.separator();

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.active_tab {
                RightTab::Chart(idx) => {
                    if let Some((_, diagram)) = self.charts.get(idx) {
                        self.render_chart(ui, diagram);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Click 'Draw Diagram' to compute a phase stability diagram.");
                        });
                    }
                }
                RightTab::Table => {
                    if let Some((_, diagram)) = self.charts.last() {
                        self.render_table(ui, diagram);
                    } else {
                        ui.label("No calculated diagram yet.");
                    }
                }
            });
    }

    fn render_chart(&self, ui: &mut egui::Ui, diagram: &PhaseStabilityDiagram) {
        let avail = ui.available_size();
        let canvas_w = avail.x.max(520.0);
        let canvas_h = avail.y.max(400.0);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(canvas_w, canvas_h), egui::Sense::hover());

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::WHITE);

        let margin_l = 70.0;
        let margin_r = 24.0;
        let margin_t = 38.0;
        let margin_b = if diagram.footer.is_some() { 64.0 } else { 48.0 };

        let plot = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + margin_l, rect.min.y + margin_t),
            egui::pos2(rect.max.x - margin_r, rect.max.y - margin_b),
        );
        if plot.width() <= 20.0 || plot.height() <= 20.0 {
            return;
        }

        // Title
        painter.text(
            egui::pos2(plot.center().x, rect.min.y + 18.0),
            egui::Align2::CENTER_CENTER,
            &diagram.title,
            egui::FontId::proportional(15.0),
            egui::Color32::BLACK,
        );

        let map_x = |x: f64| -> f32 {
            let t = (x - diagram.x_min) / (diagram.x_max - diagram.x_min);
            plot.min.x + t as f32 * plot.width()
        };
        let map_y = |y: f64| -> f32 {
            let t = (y - diagram.y_min) / (diagram.y_max - diagram.y_min);
            plot.max.y - t as f32 * plot.height()
        };

        // Colored region cells when Show colors is enabled
        if self.show_colors {
            let cell_w = plot.width() / (diagram.nx - 1) as f32 + 0.6;
            let cell_h = plot.height() / (diagram.ny - 1) as f32 + 0.6;
            let dx = (diagram.x_max - diagram.x_min) / (diagram.nx - 1) as f64;
            let dy = (diagram.y_max - diagram.y_min) / (diagram.ny - 1) as f64;
            for iy in 0..diagram.ny {
                let y = diagram.y_min + iy as f64 * dy;
                let py = map_y(y);
                for ix in 0..diagram.nx {
                    let x = diagram.x_min + ix as f64 * dx;
                    let px = map_x(x);
                    let pid = diagram.grid_phase_ids[iy * diagram.nx + ix];
                    let color = REGION_PALETTE[pid % REGION_PALETTE.len()];
                    let cell_rect = egui::Rect::from_center_size(
                        egui::pos2(px, py),
                        egui::vec2(cell_w, cell_h),
                    )
                    .intersect(plot);
                    painter.rect_filled(cell_rect, 0.0, color);
                }
            }
        }

        // Dotted gridlines & ticks
        let grid_color = if self.show_colors {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 65)
        } else {
            egui::Color32::from_gray(205)
        };
        let x_ticks = nice_ticks(diagram.x_min, diagram.x_max, 6);
        let y_ticks = nice_ticks(diagram.y_min, diagram.y_max, 8);

        let grid_stroke = egui::Stroke::new(0.9, grid_color);
        for &xt in &x_ticks {
            let px = map_x(xt);
            draw_dashed_line(
                &painter,
                egui::pos2(px, plot.min.y),
                egui::pos2(px, plot.max.y),
                2.5,
                3.5,
                grid_stroke,
            );
            painter.line_segment(
                [egui::pos2(px, plot.max.y), egui::pos2(px, plot.max.y + 4.0)],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            painter.text(
                egui::pos2(px, plot.max.y + 7.0),
                egui::Align2::CENTER_TOP,
                format_tick(xt),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
        }

        for &yt in &y_ticks {
            let py = map_y(yt);
            draw_dashed_line(
                &painter,
                egui::pos2(plot.min.x, py),
                egui::pos2(plot.max.x, py),
                2.5,
                3.5,
                grid_stroke,
            );
            painter.line_segment(
                [egui::pos2(plot.min.x - 4.0, py), egui::pos2(plot.min.x, py)],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            painter.text(
                egui::pos2(plot.min.x - 7.0, py),
                egui::Align2::RIGHT_CENTER,
                format_tick(yt),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
        }

        // Phase boundaries
        let plot_painter = painter.with_clip_rect(plot);
        for seg in &diagram.boundaries {
            plot_painter.line_segment(
                [
                    egui::pos2(map_x(seg.x0), map_y(seg.y0)),
                    egui::pos2(map_x(seg.x1), map_y(seg.y1)),
                ],
                egui::Stroke::new(1.5, egui::Color32::BLACK),
            );
        }

        // Phase labels
        for lbl in &diagram.labels {
            let px = map_x(lbl.x).clamp(plot.min.x + 28.0, plot.max.x - 28.0);
            let py = map_y(lbl.y).clamp(plot.min.y + 12.0, plot.max.y - 12.0);
            plot_painter.text(
                egui::pos2(px, py),
                egui::Align2::CENTER_CENTER,
                &lbl.formula,
                egui::FontId::proportional(13.0),
                egui::Color32::BLACK,
            );
        }

        // Plot border
        painter.rect_stroke(plot, 0.0, egui::Stroke::new(1.4, egui::Color32::BLACK));

        // Axis titles
        painter.text(
            egui::pos2(plot.center().x, plot.max.y + 24.0),
            egui::Align2::CENTER_TOP,
            &diagram.x_label,
            egui::FontId::proportional(12.5),
            egui::Color32::BLACK,
        );
        if let Some(footer) = &diagram.footer {
            painter.text(
                egui::pos2(plot.min.x, plot.max.y + 42.0),
                egui::Align2::LEFT_TOP,
                footer,
                egui::FontId::proportional(12.0),
                egui::Color32::BLACK,
            );
        }

        // Rotated Y-axis title
        draw_rotated_y_title(
            &painter,
            rect.min.x + 14.0,
            plot.center().y,
            &diagram.y_label,
            egui::FontId::proportional(12.5),
            egui::Color32::BLACK,
        );

        // Hover readout
        if let Some(pos) = response.hover_pos() {
            if plot.contains(pos) {
                let fx = ((pos.x - plot.min.x) / plot.width()) as f64;
                let fy = ((plot.max.y - pos.y) / plot.height()) as f64;
                let xv = diagram.x_min + fx * (diagram.x_max - diagram.x_min);
                let yv = diagram.y_min + fy * (diagram.y_max - diagram.y_min);
                let ix = ((fx * (diagram.nx - 1) as f64).round() as usize).min(diagram.nx - 1);
                let iy = ((fy * (diagram.ny - 1) as f64).round() as usize).min(diagram.ny - 1);
                let pid = diagram.grid_phase_ids[iy * diagram.nx + ix];
                let phase_name = diagram
                    .legend
                    .get(pid)
                    .map(|e| e.formula.as_str())
                    .unwrap_or("?");
                painter.text(
                    egui::pos2(plot.max.x - 6.0, plot.min.y + 6.0),
                    egui::Align2::RIGHT_TOP,
                    format!(
                        "{}: {:.2} | {}: {:.2} | Phase: {}",
                        diagram.x_label, xv, diagram.y_label, yv, phase_name
                    ),
                    egui::FontId::monospace(11.0),
                    egui::Color32::BLACK,
                );
            }
        }
    }

    fn render_table(&self, ui: &mut egui::Ui, diagram: &PhaseStabilityDiagram) {
        let e0 = self
            .selected_elements
            .first()
            .map(|s| s.as_str())
            .unwrap_or("E1");
        let e1 = self
            .selected_elements
            .get(1)
            .map(|s| s.as_str())
            .unwrap_or("E2");
        let e2 = self
            .selected_elements
            .get(2)
            .map(|s| s.as_str())
            .unwrap_or("E3");

        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .show(ui, |ui| {
                egui::Grid::new("ps_table_values_grid")
                    .spacing([0.0, 0.0])
                    .show(ui, |ui| {
                        let tc = |ui: &mut egui::Ui, txt: &str, w: f32, bg: egui::Color32, r: bool| {
                            text_cell(ui, txt, w, bg, egui::Color32::BLACK, r, false, false);
                        };

                        // Column letters header
                        header_cell(ui, "", 36.0);
                        header_cell(ui, "A", 32.0);
                        header_cell(ui, "B", 130.0);
                        header_cell(ui, "C", 95.0);
                        header_cell(ui, "D", 95.0);
                        header_cell(ui, "E", 95.0);
                        header_cell(ui, "F", 120.0);
                        ui.end_row();

                        // Row 1: Title
                        row_num_cell(ui, 1, 36.0);
                        section_cell(ui, "", 32.0, true);
                        section_cell(ui, &diagram.title, 130.0, false);
                        section_cell(ui, "", 95.0, false);
                        section_cell(ui, "", 95.0, false);
                        section_cell(ui, "", 95.0, false);
                        section_cell(ui, "", 120.0, false);
                        ui.end_row();

                        let meta_rows: [(&str, String, &str); 7] = [
                            ("Species", diagram.table_rows.len().to_string(), ""),
                            ("Elements", "3".into(), ""),
                            (
                                "Liquids",
                                if self.include_liquids { "TRUE" } else { "FALSE" }.into(),
                                "",
                            ),
                            (
                                "Organics",
                                if self.include_organics { "TRUE" } else { "FALSE" }.into(),
                                "",
                            ),
                            ("Temperature", format!("{:.3}", diagram.temperature_k), "K"),
                            (
                                "Show Colors",
                                if self.show_colors { "TRUE" } else { "FALSE" }.into(),
                                "",
                            ),
                            (
                                "Auto Scale",
                                if self.auto_scale { "TRUE" } else { "FALSE" }.into(),
                                "",
                            ),
                        ];

                        for (idx, (k, v, unit)) in meta_rows.iter().enumerate() {
                            row_num_cell(ui, idx + 3, 36.0);
                            tc(ui, "", 32.0, EXCEL_READONLY_BG, false);
                            tc(ui, k, 130.0, egui::Color32::WHITE, false);
                            tc(ui, v, 95.0, egui::Color32::WHITE, true);
                            tc(ui, unit, 95.0, egui::Color32::WHITE, false);
                            tc(ui, "", 95.0, egui::Color32::WHITE, false);
                            tc(ui, "", 120.0, egui::Color32::WHITE, false);
                            ui.end_row();
                        }

                        // Axis range rows
                        let axis_rows = [
                            (&diagram.x_label, diagram.x_min, diagram.x_max),
                            (&diagram.y_label, diagram.y_min, diagram.y_max),
                        ];
                        for (idx, (lbl, mn, mx)) in axis_rows.iter().enumerate() {
                            row_num_cell(ui, 10 + idx, 36.0);
                            tc(ui, "", 32.0, EXCEL_READONLY_BG, false);
                            tc(ui, lbl, 130.0, egui::Color32::WHITE, false);
                            tc(ui, "Min", 95.0, egui::Color32::WHITE, false);
                            tc(ui, &format!("{mn:.3}"), 95.0, egui::Color32::WHITE, true);
                            tc(ui, "Max", 95.0, egui::Color32::WHITE, false);
                            tc(ui, &format!("{mx:.3}"), 120.0, egui::Color32::WHITE, true);
                            ui.end_row();
                        }

                        // Table header
                        row_num_cell(ui, 12, 36.0);
                        section_cell(ui, "#", 32.0, true);
                        section_cell(ui, "Species", 130.0, false);
                        section_cell(ui, e0, 95.0, true);
                        section_cell(ui, e1, 95.0, true);
                        section_cell(ui, e2, 95.0, true);
                        section_cell(ui, "DeltaG, kcal/mol", 120.0, true);
                        ui.end_row();

                        // Species rows
                        for (idx, row) in diagram.table_rows.iter().enumerate() {
                            let bg = if idx % 2 == 1 {
                                EXCEL_ALT_ROW_BG
                            } else {
                                egui::Color32::WHITE
                            };
                            row_num_cell(ui, 13 + idx, 36.0);
                            tc(ui, &(idx + 1).to_string(), 32.0, bg, true);
                            tc(ui, &row.formula, 130.0, bg, false);
                            tc(ui, &format!("{:.3}", row.elem_counts[0]), 95.0, bg, true);
                            tc(ui, &format!("{:.3}", row.elem_counts[1]), 95.0, bg, true);
                            tc(ui, &format!("{:.3}", row.elem_counts[2]), 95.0, bg, true);
                            tc(ui, &format!("{:.3}", row.delta_g_kcal_mol), 120.0, bg, true);
                            ui.end_row();
                        }
                    });
            });
    }
}

fn pick_gas(gases: &[String], preferred: &str) -> String {
    gases
        .iter()
        .find(|g| *g == preferred)
        .cloned()
        .or_else(|| gases.first().cloned())
        .unwrap_or_else(|| preferred.to_string())
}
