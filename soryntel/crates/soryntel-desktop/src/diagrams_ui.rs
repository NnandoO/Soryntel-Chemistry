use eframe::egui;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::{
    diagrams::{
        balanced_coeff_for_element, calculate_diagram, species_db_temp_range, DiagramEnergyUnit,
        DiagramRequest, DiagramResult, DiagramSpeciesInput, DiagramType,
    },
    ChemicalSpecies, SpeciesRepository,
};

use crate::excel_grid::{
    header_cell, row_num_cell, text_cell, widget_cell, EXCEL_ALT_ROW_BG, EXCEL_GRID_BORDER,
    EXCEL_READONLY_BG,
};
use crate::ui_common::{draw_dashed_line, draw_rotated_y_title, nice_step};

const DIAG_ELEMENTS: &[&str] = &[
    "Ac", "Be", "Cm", "Fe", "Ho", "Mn", "Os", "Ra", "Si",
    "Ag", "Bi", "Co", "Fm", "I", "Mo", "P", "Rb", "Sm",
    "Al", "Bk", "Cr", "Fr", "In", "N", "Pa", "Re", "Sn",
    "Am", "Br", "Cs", "Ga", "Ir", "Na", "Pb", "Rh", "Sr",
    "Ar", "C", "Cu", "Gd", "K", "Nb", "Pd", "Rn", "Ta",
    "As", "Ca", "Dy", "Ge", "Kr", "Nd", "Pm", "Ru", "Tb",
    "At", "Cd", "Er", "H", "La", "Ne", "Po", "S", "Tc",
    "Au", "Ce", "Es", "He", "Li", "Ni", "Pr", "Sb", "Te",
    "B", "Cf", "Eu", "Hf", "Lu", "Np", "Pt", "Sc", "Th",
    "Ba", "Cl", "F", "Hg", "Mg", "O", "Pu", "Se", "Ti",
    "Be", "Co", "Fr", "Ir", "Nb", "Pm", "S", "Te", "Zn",
    "Br", "Cr", "Ga", "K", "Nd", "Po", "Sb", "Th", "Zr",
];

const CURVE_COLORS: &[(u8, u8, u8)] = &[
    (95, 135, 65),
    (22, 110, 125),
    (45, 45, 55),
    (115, 110, 80),
    (185, 115, 35),
    (30, 55, 105),
    (37, 99, 235),
    (14, 150, 180),
    (0, 128, 128),
    (155, 45, 95),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataSheetTab {
    XData,
    YData,
}

#[derive(Debug, Clone)]
struct DiagramRow {
    formula: String,
    coeff_str: String,
    t_min_str: String,
    t_max_str: String,
    visible: bool,
}

impl DiagramRow {
    fn new(formula: &str, coeff: f64, db: &ThermoDatabase, in_kelvin: bool) -> Self {
        let mut r = Self {
            formula: formula.to_string(),
            coeff_str: format!("{coeff:.2}"),
            t_min_str: String::new(),
            t_max_str: String::new(),
            visible: true,
        };
        r.refresh_db_range(db, in_kelvin);
        r
    }

    fn empty() -> Self {
        Self {
            formula: String::new(),
            coeff_str: "1.00".to_string(),
            t_min_str: String::new(),
            t_max_str: String::new(),
            visible: true,
        }
    }

    fn refresh_db_range(&mut self, db: &ThermoDatabase, in_kelvin: bool) {
        if let Some(sp) = db.get_species(self.formula.trim()) {
            let (tmin_k, tmax_k) = species_db_temp_range(&sp);
            let off = if in_kelvin { 0.0 } else { 273.15 };
            self.t_min_str = format!("{:.2}", tmin_k - off);
            self.t_max_str = format!("{:.2}", tmax_k - off);
        } else {
            self.t_min_str.clear();
            self.t_max_str.clear();
        }
    }

    fn coeff(&self) -> f64 {
        self.coeff_str.trim().parse::<f64>().unwrap_or(1.0).max(1e-6)
    }
}

pub struct DiagramsModuleState {
    rows: Vec<DiagramRow>,
    active_row: usize,
    sheet_tab: DataSheetTab,
    diagram_type: DiagramType,
    t_min_str: String,
    t_max_str: String,
    t_step_str: String,
    temp_in_kelvin: bool,
    y_min_str: String,
    y_max_str: String,
    y_auto: bool,
    energy_unit: DiagramEnergyUnit,
    per_kg: bool,
    balance_element: String,
    balance_amount_str: String,
    result: Option<DiagramResult>,
    status_error: Option<String>,
    show_db_browser: bool,
    db_query: String,
    db_matches: Vec<ChemicalSpecies>,
}

impl DiagramsModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let initial = vec![
            ("Cu2O", 2.0),
            ("FeO", 2.0),
            ("Fe2O3", 2.0 / 3.0),
            ("ZnO", 2.0),
            ("Cr2O3", 2.0 / 3.0),
            ("MgO", 2.0),
            ("CO(g)", 2.0),
            ("CO2(g)", 1.0),
            ("H2O(g)", 2.0),
        ];
        let mut rows: Vec<DiagramRow> = initial
            .into_iter()
            .map(|(f, c)| DiagramRow::new(f, c, db, false))
            .collect();
        for _ in 0..8 {
            rows.push(DiagramRow::empty());
        }

        let mut st = Self {
            rows,
            active_row: 0,
            sheet_tab: DataSheetTab::XData,
            diagram_type: DiagramType::DeltaGEllingham,
            t_min_str: "0.00".to_string(),
            t_max_str: "2000.00".to_string(),
            t_step_str: "50.00".to_string(),
            temp_in_kelvin: false,
            y_min_str: "-1200.00".to_string(),
            y_max_str: "0.00".to_string(),
            y_auto: true,
            energy_unit: DiagramEnergyUnit::KiloJoules,
            per_kg: false,
            balance_element: "O".to_string(),
            balance_amount_str: "2.00".to_string(),
            result: None,
            status_error: None,
            show_db_browser: false,
            db_query: "Fe".to_string(),
            db_matches: db.search("Fe", 40),
        };
        st.recalculate(db);
        st
    }

    fn recalculate(&mut self, db: &ThermoDatabase) {
        let t_min_val: f64 = self.t_min_str.trim().parse().unwrap_or(0.0);
        let t_max_val: f64 = self.t_max_str.trim().parse().unwrap_or(2000.0);
        let step_val: f64 = self.t_step_str.trim().parse().unwrap_or(50.0);
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };

        let species: Vec<DiagramSpeciesInput> = self
            .rows
            .iter()
            .filter(|r| !r.formula.trim().is_empty())
            .map(|r| DiagramSpeciesInput {
                formula: r.formula.trim().to_string(),
                coeff: r.coeff(),
            })
            .collect();

        let req = DiagramRequest {
            species,
            diagram_type: self.diagram_type,
            t_min_k: (t_min_val + off).max(1.0),
            t_max_k: (t_max_val + off).max(2.0),
            step_k: step_val.max(1.0),
            energy_unit: self.energy_unit,
            per_kg: self.per_kg,
        };

        match calculate_diagram(&req, db) {
            Ok(res) => {
                self.status_error = if res.warnings.is_empty() {
                    None
                } else {
                    Some(res.warnings.join(" | "))
                };
                if self.y_auto {
                    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
                    for (idx, s) in res.series.iter().enumerate() {
                        let vis = self.rows.get(idx).map(|r| r.visible).unwrap_or(true);
                        if !vis {
                            continue;
                        }
                        for p in &s.points {
                            ymin = ymin.min(p.y_val);
                            ymax = ymax.max(p.y_val);
                        }
                    }
                    if ymin.is_finite() && ymax.is_finite() {
                        let pad = ((ymax - ymin) * 0.06).max(1.0);
                        self.y_min_str = format!("{:.2}", ymin - pad);
                        self.y_max_str = format!("{:.2}", ymax + pad);
                    }
                }
                self.result = Some(res);
            }
            Err(e) => {
                self.status_error = Some(e);
            }
        }
    }

    fn balance_element_amounts(&mut self) {
        let target: f64 = self.balance_amount_str.trim().parse().unwrap_or(2.0);
        if target <= 0.0 || self.balance_element.is_empty() {
            return;
        }
        for r in &mut self.rows {
            if r.formula.trim().is_empty() {
                continue;
            }
            if let Some(c) = balanced_coeff_for_element(r.formula.trim(), &self.balance_element, target) {
                r.coeff_str = format!("{c:.2}");
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = egui::Color32::from_rgb(240, 242, 245);
        vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(vis);

        // Top menu bar
        egui::TopBottomPanel::top("diag_top_menu")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 240, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Thermochemical Data Diagrams");
                    ui.separator();
                    let active_f = self
                        .rows
                        .get(self.active_row)
                        .map(|r| r.formula.as_str())
                        .unwrap_or("");
                    ui.label("Active species:");
                    ui.label(egui::RichText::new(active_f).strong());
                    if let Some(err) = &self.status_error {
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                    }
                });
            });

        // 1. Left Resizable Spreadsheet Panel (X-data / Y-data)
        egui::SidePanel::left("diag_left_sheet")
            .resizable(true)
            .default_width(355.0)
            .min_width(180.0)
            .max_width(640.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.sheet_tab == DataSheetTab::XData, "X-data")
                        .clicked()
                    {
                        self.sheet_tab = DataSheetTab::XData;
                    }
                    if ui
                        .selectable_label(self.sheet_tab == DataSheetTab::YData, "Y-data")
                        .clicked()
                    {
                        self.sheet_tab = DataSheetTab::YData;
                    }
                    if ui.small_button("+ Row").clicked() {
                        self.rows.push(DiagramRow::empty());
                    }
                });
                ui.separator();

                egui::ScrollArea::both()
                    .id_salt("diag_sheet_scroll")
                    .show(ui, |ui| match self.sheet_tab {
                        DataSheetTab::XData => self.ui_xdata_grid(ui, db),
                        DataSheetTab::YData => self.ui_ydata_grid(ui),
                    });
            });

        // 2. Center Resizable Controls Panel
        egui::SidePanel::left("diag_center_controls")
            .resizable(true)
            .default_width(245.0)
            .min_width(175.0)
            .max_width(420.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 245))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.label(egui::RichText::new("Diagram Type").strong().size(11.5));
                    egui::ComboBox::from_id_salt("diag_type_combo")
                        .width(ui.available_width().max(160.0) - 8.0)
                        .selected_text(self.diagram_type.label())
                        .show_ui(ui, |ui| {
                            for &dt in DiagramType::ALL {
                                if ui
                                    .selectable_value(&mut self.diagram_type, dt, dt.label())
                                    .changed()
                                {
                                    self.recalculate(db);
                                }
                            }
                        });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("X-axis Range (Temperature)")
                                .strong()
                                .size(11.0),
                        );
                        egui::Grid::new("diag_x_range_grid")
                            .spacing(egui::vec2(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label("MIN");
                                ui.label("MAX");
                                ui.label("STEP");
                                ui.end_row();
                                ui.add_sized(
                                    [58.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_min_str),
                                );
                                ui.add_sized(
                                    [58.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_max_str),
                                );
                                ui.add_sized(
                                    [52.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_step_str),
                                );
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            if ui.radio(!self.temp_in_kelvin, "°C").clicked() && self.temp_in_kelvin {
                                self.temp_in_kelvin = false;
                                if let Ok(v) = self.t_min_str.trim().parse::<f64>() {
                                    self.t_min_str = format!("{:.2}", v - 273.15);
                                }
                                if let Ok(v) = self.t_max_str.trim().parse::<f64>() {
                                    self.t_max_str = format!("{:.2}", v - 273.15);
                                }
                                for r in &mut self.rows {
                                    r.refresh_db_range(db, false);
                                }
                                self.recalculate(db);
                            }
                            if ui.radio(self.temp_in_kelvin, "K").clicked() && !self.temp_in_kelvin {
                                self.temp_in_kelvin = true;
                                if let Ok(v) = self.t_min_str.trim().parse::<f64>() {
                                    self.t_min_str = format!("{:.2}", v + 273.15);
                                }
                                if let Ok(v) = self.t_max_str.trim().parse::<f64>() {
                                    self.t_max_str = format!("{:.2}", v + 273.15);
                                }
                                for r in &mut self.rows {
                                    r.refresh_db_range(db, true);
                                }
                                self.recalculate(db);
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Y-axis Range").strong().size(11.0));
                        egui::Grid::new("diag_y_range_grid")
                            .spacing(egui::vec2(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label("MIN");
                                ui.label("MAX");
                                ui.label("");
                                ui.end_row();
                                ui.add_enabled(
                                    !self.y_auto,
                                    egui::TextEdit::singleline(&mut self.y_min_str)
                                        .desired_width(60.0),
                                );
                                ui.add_enabled(
                                    !self.y_auto,
                                    egui::TextEdit::singleline(&mut self.y_max_str)
                                        .desired_width(60.0),
                                );
                                if ui.checkbox(&mut self.y_auto, "Auto").changed() && self.y_auto {
                                    self.recalculate(db);
                                }
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            if ui
                                .radio(
                                    self.energy_unit == DiagramEnergyUnit::KiloCalories,
                                    "kcal/mol",
                                )
                                .clicked()
                            {
                                self.energy_unit = DiagramEnergyUnit::KiloCalories;
                                self.recalculate(db);
                            }
                            if ui
                                .radio(
                                    self.energy_unit == DiagramEnergyUnit::KiloJoules,
                                    "kJ/mol",
                                )
                                .clicked()
                            {
                                self.energy_unit = DiagramEnergyUnit::KiloJoules;
                                self.recalculate(db);
                            }
                            if ui.checkbox(&mut self.per_kg, "kg").changed() {
                                self.recalculate(db);
                            }
                        });
                    });

                    ui.add_space(4.0);
                    // Balance Element Amount + Element grid (Figs. 1, 2, 3)
                    ui.horizontal(|ui| {
                        if ui.button("Balance Element Amount").clicked() {
                            self.balance_element_amounts();
                            self.recalculate(db);
                        }
                        ui.add_sized(
                            [52.0, 20.0],
                            egui::TextEdit::singleline(&mut self.balance_amount_str),
                        );
                    });

                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(3.0))
                        .show(ui, |ui| {
                            egui::Grid::new("diag_elem_grid_9col")
                                .spacing(egui::vec2(1.5, 1.5))
                                .show(ui, |ui| {
                                    for (idx, &sym) in DIAG_ELEMENTS.iter().enumerate() {
                                        let sel = self.balance_element == sym;
                                        let btn = egui::Button::new(
                                            egui::RichText::new(sym).size(10.0),
                                        )
                                        .min_size(egui::vec2(21.0, 16.0))
                                        .fill(if sel {
                                            egui::Color32::from_rgb(191, 219, 254)
                                        } else {
                                            egui::Color32::WHITE
                                        });
                                        if ui.add(btn).clicked() {
                                            self.balance_element = sym.to_string();
                                        }
                                        if (idx + 1) % 9 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    ui.add_space(6.0);
                    if ui
                        .add_sized([ui.available_width().max(150.0), 24.0], egui::Button::new("Browse Database"))
                        .clicked()
                    {
                        self.show_db_browser = true;
                    }
                    ui.add_space(2.0);
                    if ui
                        .add_sized([ui.available_width().max(150.0), 26.0], egui::Button::new("Diagram"))
                        .clicked()
                    {
                        self.recalculate(db);
                    }
                });
            });

        // 3. Right Central Chart Panel
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .id_salt("diag_chart_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.ui_chart(ui);
                    });
            });

        if self.show_db_browser {
            self.ui_db_browser(ctx, db);
        }
    }

    fn ui_xdata_grid(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let t_unit = if self.temp_in_kelvin { "K" } else { "°C" };
        ui.horizontal(|ui| {
            row_num_cell(ui, 0, 28.0);
            header_cell(ui, "A", 110.0);
            header_cell(ui, "B", 58.0);
            header_cell(ui, "C", 74.0);
            header_cell(ui, "D", 74.0);
        });
        ui.horizontal(|ui| {
            row_num_cell(ui, 1, 28.0);
            header_cell(ui, "Species", 110.0);
            header_cell(ui, "Coef.", 58.0);
            header_cell(ui, &format!("TMin ({t_unit})"), 74.0);
            header_cell(ui, &format!("TMax ({t_unit})"), 74.0);
        });

        for idx in 0..self.rows.len() {
            let bg = if idx % 2 == 0 {
                egui::Color32::WHITE
            } else {
                EXCEL_ALT_ROW_BG
            };
            let is_active = self.active_row == idx;
            ui.horizontal(|ui| {
                row_num_cell(ui, idx + 2, 28.0);
                widget_cell(ui, 110.0, bg, is_active, |ui| {
                    let r = ui.text_edit_singleline(&mut self.rows[idx].formula);
                    if r.gained_focus() {
                        self.active_row = idx;
                    }
                    if r.lost_focus() {
                        let tk = self.temp_in_kelvin;
                        self.rows[idx].refresh_db_range(db, tk);
                    }
                });
                widget_cell(ui, 58.0, bg, false, |ui| {
                    ui.text_edit_singleline(&mut self.rows[idx].coeff_str);
                });
                text_cell(
                    ui,
                    &self.rows[idx].t_min_str,
                    74.0,
                    EXCEL_READONLY_BG,
                    egui::Color32::BLACK,
                    true,
                    true,
                    false,
                );
                text_cell(
                    ui,
                    &self.rows[idx].t_max_str,
                    74.0,
                    EXCEL_READONLY_BG,
                    egui::Color32::BLACK,
                    true,
                    true,
                    false,
                );
            });
        }
    }

    fn ui_ydata_grid(&self, ui: &mut egui::Ui) {
        let Some(res) = &self.result else {
            ui.label("Click Diagram to compute Y-data.");
            return;
        };
        let t_unit = if self.temp_in_kelvin { "K" } else { "°C" };
        ui.horizontal(|ui| {
            header_cell(ui, &format!("T ({t_unit})"), 72.0);
            for s in &res.series {
                header_cell(ui, &s.label, 88.0);
            }
        });
        let n_pts = res.series.first().map(|s| s.points.len()).unwrap_or(0);
        for pi in 0..n_pts {
            let t_disp = res.series[0].points[pi].t_k - if self.temp_in_kelvin { 0.0 } else { 273.15 };
            let bg = if pi % 2 == 0 {
                egui::Color32::WHITE
            } else {
                EXCEL_ALT_ROW_BG
            };
            ui.horizontal(|ui| {
                text_cell(ui, &format!("{t_disp:.1}"), 72.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                for s in &res.series {
                    let val_s = s
                        .points
                        .get(pi)
                        .map(|p| format!("{:.2}", p.y_val))
                        .unwrap_or_default();
                    text_cell(ui, &val_s, 88.0, bg, egui::Color32::BLACK, true, true, false);
                }
            });
        }
    }

    fn ui_chart(&mut self, ui: &mut egui::Ui) {
        let Some(res) = self.result.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label("Click Diagram to plot.");
            });
            return;
        };

        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(res.diagram_type.label())
                    .size(14.5)
                    .strong()
                    .color(egui::Color32::BLACK),
            );
        });

        let avail = ui.available_size();
        let plot_w = (avail.x - 12.0).max(240.0);
        let plot_h = (avail.y - 26.0).max(200.0);
        let (outer_rect, resp) =
            ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::hover());
        let painter = ui.painter_at(outer_rect);

        let margin_l = 68.0;
        let margin_r = 22.0;
        let margin_t = 10.0;
        let margin_b = 36.0;
        let plot_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(margin_l, margin_t),
            outer_rect.max - egui::vec2(margin_r, margin_b),
        );

        let t_min: f64 = self.t_min_str.trim().parse().unwrap_or(0.0);
        let t_max: f64 = self.t_max_str.trim().parse::<f64>().unwrap_or(2000.0).max(t_min + 1.0);
        let y_min: f64 = self.y_min_str.trim().parse().unwrap_or(-1200.0);
        let y_max: f64 = self.y_max_str.trim().parse::<f64>().unwrap_or(0.0).max(y_min + 1.0);
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };

        let x_to_px = |t_disp: f64| -> f32 {
            plot_rect.left() + (((t_disp - t_min) / (t_max - t_min)) as f32) * plot_rect.width()
        };
        let y_to_px = |y_val: f64| -> f32 {
            plot_rect.bottom() - (((y_val - y_min) / (y_max - y_min)) as f32) * plot_rect.height()
        };

        painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

        // Dotted gridlines
        let grid_col = egui::Color32::from_rgb(205, 210, 218);
        let x_step = nice_step((t_max - t_min) / 8.0);
        let mut gx = (t_min / x_step).ceil() * x_step;
        while gx <= t_max + 1e-6 {
            let x = x_to_px(gx);
            draw_dashed_line(&painter, egui::pos2(x, plot_rect.top()), egui::pos2(x, plot_rect.bottom()), 2.0, 3.0, egui::Stroke::new(0.8, grid_col));
            painter.text(
                egui::pos2(x, plot_rect.bottom() + 12.0),
                egui::Align2::CENTER_CENTER,
                format!("{gx:.0}"),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            gx += x_step;
        }

        let y_step = nice_step((y_max - y_min) / 8.0);
        let mut gy = (y_min / y_step).ceil() * y_step;
        while gy <= y_max + 1e-6 {
            let y = y_to_px(gy);
            draw_dashed_line(&painter, egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y), 2.0, 3.0, egui::Stroke::new(0.8, grid_col));
            painter.text(
                egui::pos2(plot_rect.left() - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{gy:.0}"),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            gy += y_step;
        }

        let clipped = painter.with_clip_rect(plot_rect);

        // Plot each visible series (solid in DB range, dashed when extrapolated)
        for (idx, s) in res.series.iter().enumerate() {
            let visible = self.rows.get(idx).map(|r| r.visible).unwrap_or(true);
            if !visible || s.points.len() < 2 {
                continue;
            }
            let (r, g, b) = CURVE_COLORS[idx % CURVE_COLORS.len()];
            let col = egui::Color32::from_rgb(r, g, b);
            let stroke = egui::Stroke::new(1.5, col);

            for win in s.points.windows(2) {
                let p0 = egui::pos2(x_to_px(win[0].t_k - off), y_to_px(win[0].y_val));
                let p1 = egui::pos2(x_to_px(win[1].t_k - off), y_to_px(win[1].y_val));
                if win[0].extrapolated || win[1].extrapolated {
                    draw_dashed_line(&clipped, p0, p1, 4.0, 3.0, stroke);
                } else {
                    clipped.line_segment([p0, p1], stroke);
                }
            }

            // Label near 70% along curve (offset slightly by series index to reduce overlap)
            let lbl_idx = ((s.points.len() * (55 + (idx % 4) * 10)) / 100).min(s.points.len() - 1);
            let lp = s.points[lbl_idx];
            let lpos = egui::pos2(
                x_to_px(lp.t_k - off),
                y_to_px(lp.y_val) - 10.0,
            );
            clipped.text(
                lpos,
                egui::Align2::CENTER_CENTER,
                &s.label,
                egui::FontId::proportional(11.5),
                col,
            );
        }

        painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.3, egui::Color32::BLACK));
        let x_lbl = if self.temp_in_kelvin {
            "Temperature (K)"
        } else {
            "Temperature (°C)"
        };
        painter.text(
            egui::pos2(plot_rect.center().x, plot_rect.bottom() + 25.0),
            egui::Align2::CENTER_CENTER,
            x_lbl,
            egui::FontId::proportional(12.0),
            egui::Color32::BLACK,
        );
        draw_rotated_y_title(
            &painter,
            outer_rect.left() + 12.0,
            plot_rect.center().y,
            &res.y_unit,
            egui::FontId::proportional(11.5),
            egui::Color32::BLACK,
        );

        // Hover inspector
        if let Some(pos) = resp.hover_pos() {
            if plot_rect.contains(pos) {
                let fx = (pos.x - plot_rect.left()) / plot_rect.width();
                let fy = (plot_rect.bottom() - pos.y) / plot_rect.height();
                let t_hov = t_min + fx as f64 * (t_max - t_min);
                let y_hov = y_min + fy as f64 * (y_max - y_min);
                resp.on_hover_text(format!("T: {t_hov:.1}\nY: {y_hov:.2} {}", res.y_unit));
            }
        }

        // Interactive Legend strip below chart
        ui.horizontal_wrapped(|ui| {
            for (idx, s) in res.series.iter().enumerate() {
                if let Some(row) = self.rows.get_mut(idx) {
                    let (r, g, b) = CURVE_COLORS[idx % CURVE_COLORS.len()];
                    ui.checkbox(
                        &mut row.visible,
                        egui::RichText::new(&s.label)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(r, g, b)),
                    );
                }
            }
        });
    }

    fn ui_db_browser(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_db_browser;
        egui::Window::new("Browse Database — Add Species")
            .open(&mut open)
            .resizable(true)
            .default_size([420.0, 380.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    if ui.text_edit_singleline(&mut self.db_query).changed() {
                        self.db_matches = db.search(&self.db_query, 50);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for sp in self.db_matches.clone() {
                        ui.horizontal(|ui| {
                            if ui.small_button("+ Add").clicked() {
                                let tk = self.temp_in_kelvin;
                                let row = DiagramRow::new(&sp.formula, 1.0, db, tk);
                                if let Some(empty) =
                                    self.rows.iter_mut().find(|r| r.formula.trim().is_empty())
                                {
                                    *empty = row;
                                } else {
                                    self.rows.push(row);
                                }
                            }
                            ui.strong(&sp.formula);
                            ui.label(&sp.name);
                        });
                    }
                });
            });
        if !open {
            self.show_db_browser = false;
        }
    }
}
