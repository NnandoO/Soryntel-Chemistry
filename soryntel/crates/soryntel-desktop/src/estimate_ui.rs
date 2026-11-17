//! Module 06 — HSC Estimate UI (Fig. 1–5 + Cp Range & HSC Cp Data workflow).

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, text_cell, widget_cell, EXCEL_GRID_BORDER,
    EXCEL_HEADER_BORDER, EXCEL_READONLY_BG,
};
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::estimate::{
    build_estimated_species, classic_hsc_estimate_rows, compute_parity_stats, default_fig1_rows,
    evaluate_estimate_row, fit_kelley_cp, generate_cp_curve, EstimateRow, EstimateSpeciesType,
    FunctionalGroup, OrganicGroup,
};
use soryntel_thermo::{KelleyCoefficients, Phase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatePlotMode {
    Parity,
    Comparison,
    Differences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimateScatterScope {
    SheetRows,
    GasDb,
    AqueousDb,
    CondensedDb,
    AllDb,
}

pub struct EstimateModuleState {
    pub rows: Vec<EstimateRow>,
    pub selected_row: Option<usize>,
    pub show_type_column: bool,
    pub freeze_col_a: bool,
    pub freeze_title_rows: bool,
    pub use_kelvin: bool,
    pub per_kg: bool,
    pub show_db_picker: bool,
    pub db_search: String,
    pub show_graphics_win: bool,
    pub plot_mode: EstimatePlotMode,
    pub scatter_scope: EstimateScatterScope,
    pub cached_db_scope: Option<EstimateScatterScope>,
    pub cached_db_rows: Vec<EstimateRow>,
    pub active_stat_prop: usize, // 0=H, 1=S, 2=Cp, 3=dG
    pub show_cp_range_dialog: bool,
    pub cp_tmin_k: f64,
    pub cp_tmax_k: f64,
    pub cp_steps: usize,
    pub show_cp_data_win: bool,
    pub cp_points: Vec<(f64, f64)>,
    pub cp_kelley: KelleyCoefficients,
    pub cp_target_row: usize,
    pub show_help_modal: bool,
    pub quick_species: String,
    pub quick_temp: f64,
    pub quick_is_kelvin: bool,
    pub status_msg: String,
}

impl EstimateModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let mut rows = default_fig1_rows();
        for r in &mut rows {
            evaluate_estimate_row(r, db);
        }
        Self {
            rows,
            selected_row: Some(0),
            show_type_column: true,
            freeze_col_a: false,
            freeze_title_rows: true,
            use_kelvin: false,
            per_kg: false,
            show_db_picker: false,
            db_search: String::new(),
            show_graphics_win: false,
            plot_mode: EstimatePlotMode::Parity,
            scatter_scope: EstimateScatterScope::SheetRows,
            cached_db_scope: None,
            cached_db_rows: Vec::new(),
            active_stat_prop: 1,
            show_cp_range_dialog: false,
            cp_tmin_k: 298.15,
            cp_tmax_k: 600.0,
            cp_steps: 50,
            show_cp_data_win: false,
            cp_points: Vec::new(),
            cp_kelley: KelleyCoefficients {
                a: 0.0,
                b: 0.0,
                c: 0.0,
                d: 0.0,
            },
            cp_target_row: 0,
            show_help_modal: false,
            quick_species: "POP".to_string(),
            quick_temp: 25.0,
            quick_is_kelvin: false,
            status_msg: "Listo · HSC Estimate (Neumann-Kopp, Latimer, Benson/Joback)".to_string(),
        }
    }

    fn recalc_all(&mut self, db: &ThermoDatabase) {
        for r in &mut self.rows {
            evaluate_estimate_row(r, db);
        }
    }

    fn open_cp_data_for_row(&mut self, row_idx: usize, db: &ThermoDatabase) {
        if let Some(row) = self.rows.get(row_idx) {
            self.cp_target_row = row_idx;
            self.cp_points =
                generate_cp_curve(row, db, self.cp_tmin_k, self.cp_tmax_k, self.cp_steps);
            self.cp_kelley = fit_kelley_cp(&self.cp_points);
            self.show_cp_data_win = true;
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        // Ensure at least one empty trailing input row
        if self
            .rows
            .last()
            .map(|r| !r.formula.trim().is_empty())
            .unwrap_or(true)
        {
            self.rows.push(EstimateRow::new(""));
        }

        // Top Ribbon Toolbar (Main | Estimation Settings | Units | Tools | Help)
        egui::TopBottomPanel::top("estimate_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 240, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Main: File
                    ui.vertical(|ui| {
                        ui.menu_button(
                            egui::RichText::new("📄\nFile ▾").size(11.5),
                            |ui| {
                                if ui.button("New (Blank Sheet)").clicked() {
                                    self.rows = vec![EstimateRow::new("")];
                                    self.selected_row = Some(0);
                                    self.status_msg = "Hoja nueva".to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 1 Example").clicked() {
                                    self.rows = default_fig1_rows();
                                    self.recalc_all(db);
                                    self.selected_row = Some(0);
                                    self.status_msg = "Ejemplo Fig. 1 cargado".to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Classic Example (Na2SO4, MgO*Al2O3, POP)").clicked() {
                                    self.rows = classic_hsc_estimate_rows();
                                    self.recalc_all(db);
                                    self.selected_row = Some(0);
                                    self.status_msg =
                                        "Ejemplo clásico H, S, Cp Estimate cargado (11 especies)"
                                            .to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 2 Example (C2H4(g))").clicked() {
                                    let mut r = EstimateRow::new("C2H4(g)");
                                    r.species_type = EstimateSpeciesType::Organic;
                                    r.organic_group = OrganicGroup::Alkenes;
                                    r.functional_group = FunctionalGroup::None;
                                    evaluate_estimate_row(&mut r, db);
                                    self.rows = vec![r, EstimateRow::new("")];
                                    self.selected_row = Some(0);
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 3 Example (FeVO4)").clicked() {
                                    let mut r = EstimateRow::new("FeVO4");
                                    evaluate_estimate_row(&mut r, db);
                                    self.rows = vec![r, EstimateRow::new("")];
                                    self.selected_row = Some(0);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Exit").clicked() {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            },
                        );
                        ui.label(egui::RichText::new("Main").small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Estimation Settings
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let sel = self.selected_row.unwrap_or(0);
                            if let Some(row) = self.rows.get_mut(sel) {
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Species Type").small());
                                    let mut changed = false;
                                    egui::ComboBox::from_id_salt("est_species_type")
                                        .width(105.0)
                                        .selected_text(row.species_type.label())
                                        .show_ui(ui, |ui| {
                                            changed |= ui
                                                .selectable_value(
                                                    &mut row.species_type,
                                                    EstimateSpeciesType::NotSpecified,
                                                    "Not Specified",
                                                )
                                                .changed();
                                            changed |= ui
                                                .selectable_value(
                                                    &mut row.species_type,
                                                    EstimateSpeciesType::Inorganic,
                                                    "Inorganic",
                                                )
                                                .changed();
                                            changed |= ui
                                                .selectable_value(
                                                    &mut row.species_type,
                                                    EstimateSpeciesType::Organic,
                                                    "Organic",
                                                )
                                                .changed();
                                        });
                                    if changed {
                                        evaluate_estimate_row(row, db);
                                    }
                                });

                                let is_org = row.species_type == EstimateSpeciesType::Organic;
                                ui.add_enabled_ui(is_org, |ui| {
                                    ui.vertical(|ui| {
                                        let mut changed = false;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Organic Group").small());
                                            egui::ComboBox::from_id_salt("est_org_group")
                                                .width(96.0)
                                                .selected_text(row.organic_group.label())
                                                .show_ui(ui, |ui| {
                                                    for g in [
                                                        OrganicGroup::Alkanes,
                                                        OrganicGroup::Alkenes,
                                                        OrganicGroup::Alkynes,
                                                        OrganicGroup::Aromatics,
                                                        OrganicGroup::Alcohols,
                                                        OrganicGroup::Acids,
                                                    ] {
                                                        changed |= ui
                                                            .selectable_value(
                                                                &mut row.organic_group,
                                                                g,
                                                                g.label(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Functional Group").small(),
                                            );
                                            egui::ComboBox::from_id_salt("est_func_group")
                                                .width(96.0)
                                                .selected_text(row.functional_group.label())
                                                .show_ui(ui, |ui| {
                                                    for fg in [
                                                        FunctionalGroup::None,
                                                        FunctionalGroup::Ene,
                                                        FunctionalGroup::Diene,
                                                        FunctionalGroup::Triene,
                                                        FunctionalGroup::Yne,
                                                        FunctionalGroup::Hydroxyl,
                                                        FunctionalGroup::Carboxyl,
                                                    ] {
                                                        changed |= ui
                                                            .selectable_value(
                                                                &mut row.functional_group,
                                                                fg,
                                                                fg.label(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                        });
                                        if changed {
                                            evaluate_estimate_row(row, db);
                                        }
                                    });
                                });
                            }

                            ui.toggle_value(&mut self.show_type_column, "Show Type\nColumn");
                            ui.toggle_value(&mut self.freeze_col_a, "Freeze\nColumn A");
                            ui.toggle_value(&mut self.freeze_title_rows, "Freeze\nTitle Rows");
                        });
                        ui.label(
                            egui::RichText::new("Estimation Settings")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    });

                    ui.separator();

                    // Units
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!self.use_kelvin, "°C\nCelsius").clicked() {
                                self.use_kelvin = false;
                            }
                            if ui.selectable_label(self.use_kelvin, "K\nKelvin").clicked() {
                                self.use_kelvin = true;
                            }
                            if ui.selectable_label(!self.per_kg, "n\nMoles").clicked() {
                                self.per_kg = false;
                            }
                            if ui.selectable_label(self.per_kg, "1kg\nKilograms").clicked() {
                                self.per_kg = true;
                            }
                        });
                        ui.label(egui::RichText::new("Units").small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Tools
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.menu_button("📊\nGraphics ▾", |ui| {
                                if ui.button("Parity Chart (Sheet)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::SheetRows;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Database Scatter (Gas Species)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::GasDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Database Scatter (Aqueous Species)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::AqueousDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Database Scatter (All Maindb6)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::AllDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Comparison").clicked() {
                                    self.plot_mode = EstimatePlotMode::Comparison;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Differences").clicked() {
                                    self.plot_mode = EstimatePlotMode::Differences;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Cp Estimation").clicked() {
                                    let idx = self.selected_row.unwrap_or(0);
                                    self.open_cp_data_for_row(idx, db);
                                    ui.close_menu();
                                }
                            });
                            if ui.button("🗄\nBrowse HSC\nDatabase").clicked() {
                                self.show_db_picker = true;
                            }
                            ui.menu_button("💾\nSave to HSC\nDatabase ▾", |ui| {
                                if ui.button("Cp at 25°C").clicked() {
                                    let idx = self.selected_row.unwrap_or(0);
                                    if let Some(row) = self.rows.get(idx) {
                                        if !row.formula.trim().is_empty() {
                                            let cp = row.est_cp_j_molk.unwrap_or(40.0);
                                            let sp = build_estimated_species(
                                                row,
                                                298.15,
                                                298.15,
                                                KelleyCoefficients {
                                                    a: cp,
                                                    b: 0.0,
                                                    c: 0.0,
                                                    d: 0.0,
                                                },
                                            );
                                            let f = sp.formula.clone();
                                            db.insert_species(sp);
                                            self.recalc_all(db);
                                            self.status_msg = format!(
                                                "Especie '{f}' guardada en sesión (Cp 25°C)"
                                            );
                                        }
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Cp Range...").clicked() {
                                    self.cp_target_row = self.selected_row.unwrap_or(0);
                                    self.show_cp_range_dialog = true;
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.label(egui::RichText::new("Tools").small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Help
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("❓\nHelp").clicked() {
                                self.show_help_modal = true;
                            }
                        });
                        ui.label(egui::RichText::new("Help").small().color(egui::Color32::GRAY));
                    });
                });
            });

        // Quick Estimate + Formula Bar (matching classic H, S, Cp Estimate + Fig. 1)
        egui::TopBottomPanel::top("estimate_formula_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let row_num = self.selected_row.unwrap_or(0) + 4;
                    ui.label(egui::RichText::new(format!("A{row_num}")).monospace().strong());
                    ui.separator();
                    if let Some(idx) = self.selected_row {
                        if let Some(row) = self.rows.get_mut(idx) {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut row.formula)
                                        .desired_width(220.0),
                                )
                                .changed()
                            {
                                evaluate_estimate_row(row, db);
                            }
                        }
                    }
                    ui.separator();
                    ui.label("Species:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.quick_species).desired_width(120.0),
                    );
                    ui.label("Temperature:");
                    ui.add(egui::DragValue::new(&mut self.quick_temp).speed(1.0).fixed_decimals(2));
                    if ui.checkbox(&mut (!self.quick_is_kelvin).clone(), "°C").clicked() {
                        if self.quick_is_kelvin {
                            self.quick_temp -= 273.15;
                            self.quick_is_kelvin = false;
                        }
                    }
                    if ui.checkbox(&mut self.quick_is_kelvin.clone(), "K").clicked() {
                        if !self.quick_is_kelvin {
                            self.quick_temp += 273.15;
                            self.quick_is_kelvin = true;
                        }
                    }
                    if ui.button("Estimate").clicked() && !self.quick_species.trim().is_empty() {
                        let mut r = EstimateRow::new(self.quick_species.trim());
                        r.temp_c = if self.quick_is_kelvin {
                            self.quick_temp - 273.15
                        } else {
                            self.quick_temp
                        };
                        evaluate_estimate_row(&mut r, db);
                        let insert_pos = self
                            .rows
                            .iter()
                            .position(|x| x.formula.trim().is_empty())
                            .unwrap_or(self.rows.len());
                        self.rows.insert(insert_pos, r);
                        self.selected_row = Some(insert_pos);
                        self.status_msg = format!("Estimado '{}'", self.quick_species.trim());
                    }
                });
            });

        // Bottom Sheet Tab Bar
        egui::TopBottomPanel::bottom("estimate_bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 237, 240))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("|◀  ◀  ▶  ▶|")
                            .size(10.5)
                            .color(egui::Color32::from_rgb(107, 114, 128)),
                    );
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                        .inner_margin(egui::Margin::symmetric(10.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Sheet1").strong().size(11.5));
                        });
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .small()
                            .color(egui::Color32::from_rgb(75, 85, 99)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Soryntel Estimate Engine")
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(185, 28, 28)),
                        );
                    });
                });
            });

        // Main Spreadsheet Area
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                self.render_spreadsheet(ui, db);
            });

        // Subwindows
        if self.show_db_picker {
            self.render_db_picker(ctx, db);
        }
        if self.show_graphics_win {
            self.render_graphics_window(ctx, db);
        }
        if self.show_cp_range_dialog {
            self.render_cp_range_dialog(ctx, db);
        }
        if self.show_cp_data_win {
            self.render_cp_data_window(ctx, db);
        }
        if self.show_help_modal {
            egui::Window::new("Help — HSC Estimate")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("• Escribe una fórmula química en la columna A (ej. FeVO4, MgSO4*2H2O, C8H18(g)).");
                    ui.label("• Compara automáticamente Estimate vs Maindb6 (Database).");
                    ui.label("• Las columnas N–AB muestran la información elemental (Amount, Charge, Weight-%, Atom-%).");
                    ui.label("• Usa Graphics -> Parity Chart o Save to HSC Database -> Cp Range para ajustar Kelley.");
                    ui.add_space(8.0);
                    if ui.button("Cerrar").clicked() {
                        self.show_help_modal = false;
                    }
                });
        }
    }

    fn render_spreadsheet(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        // Columns A..M + N..AB (3 Element Info blocks of 5 cols = 15 cols)
        let mut col_widths = vec![
            142.0, // A: Chemical Formula
            72.0,  // B: Temp
            if self.show_type_column { 148.0 } else { 0.0 }, // C: Species Type
            102.0, // D: Selected set of oxidation numbers
            68.0,  // E: MW
            78.0,  // F: H Est
            78.0,  // G: H DB
            76.0,  // H: S Est
            76.0,  // I: S DB
            74.0,  // J: Cp Est
            74.0,  // K: Cp DB
            78.0,  // L: dG Est
            78.0,  // M: dG DB
        ];
        // N..AB: 3 Element Information blocks (Elem, Amount, Charge, Weight-%, Atom-%)
        for _ in 0..3 {
            col_widths.extend_from_slice(&[54.0, 56.0, 54.0, 64.0, 62.0]);
        }

        let row_num_w = 30.0;
        let header_bg = egui::Color32::from_rgb(156, 160, 166);
        let temp_lbl = if self.use_kelvin { "Temp K" } else { "Temp °C" };
        let t_ref_hdr = if self.use_kelvin { "298.15 K" } else { "25 °C" };
        let mass_unit = if self.per_kg { "kg" } else { "mol" };

        let mut recalc_idx: Option<usize> = None;
        let mut insert_at: Option<usize> = None;
        let mut delete_at: Option<usize> = None;
        let mut open_cp_at: Option<usize> = None;
        let mut open_graph_mode: Option<EstimatePlotMode> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // Excel Column Letters (A .. AB)
                ui.horizontal(|ui| {
                    header_cell(ui, "", row_num_w);
                    for (idx, w) in col_widths.iter().enumerate() {
                        if *w > 0.0 {
                            header_cell(ui, &col_letter(idx), *w);
                        }
                    }
                });

                // Header Row 1
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, row_num_w);
                    let r1 = [
                        "",
                        "",
                        "",
                        "Selected set",
                        "MW",
                        &format!("H ({t_ref_hdr})"),
                        "",
                        &format!("S ({t_ref_hdr})"),
                        "",
                        "Cp",
                        "",
                        &format!("ΔG ({t_ref_hdr})"),
                        "",
                        "Element Information",
                        "",
                        "",
                        "",
                        "",
                        "Element Information",
                        "",
                        "",
                        "",
                        "",
                        "Element Information",
                        "",
                        "",
                        "",
                        "",
                    ];
                    for (idx, txt) in r1.iter().enumerate() {
                        if col_widths[idx] > 0.0 {
                            text_cell(
                                ui,
                                txt,
                                col_widths[idx],
                                header_bg,
                                egui::Color32::from_rgb(20, 20, 22),
                                false,
                                true,
                                false,
                            );
                        }
                    }
                });

                // Header Row 2
                ui.horizontal(|ui| {
                    row_num_cell(ui, 2, row_num_w);
                    let r2 = [
                        "Chemical Formula",
                        temp_lbl,
                        "Species Type",
                        "of oxidation",
                        "",
                        &format!("kJ/{mass_unit}"),
                        "",
                        &format!("J/{mass_unit}*K"),
                        "",
                        &format!("J/{mass_unit}*K"),
                        "",
                        &format!("kJ/{mass_unit}"),
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                    ];
                    for (idx, txt) in r2.iter().enumerate() {
                        if col_widths[idx] > 0.0 {
                            text_cell(
                                ui,
                                txt,
                                col_widths[idx],
                                header_bg,
                                egui::Color32::from_rgb(20, 20, 22),
                                false,
                                true,
                                false,
                            );
                        }
                    }
                });

                // Header Row 3
                ui.horizontal(|ui| {
                    row_num_cell(ui, 3, row_num_w);
                    let r3 = [
                        "",
                        "",
                        "",
                        "numbers",
                        "g/mol",
                        "Estimate",
                        "Database",
                        "Estimate",
                        "Database",
                        "Estimate",
                        "Database",
                        "Estimate",
                        "Database",
                        "Elem1",
                        "Amount",
                        "Charge",
                        "Weight-%",
                        "Atom-%",
                        "Elem2",
                        "Amount",
                        "Charge",
                        "Weight-%",
                        "Atom-%",
                        "Elem3",
                        "Amount",
                        "Charge",
                        "Weight-%",
                        "Atom-%",
                    ];
                    for (idx, txt) in r3.iter().enumerate() {
                        if col_widths[idx] > 0.0 {
                            text_cell(
                                ui,
                                txt,
                                col_widths[idx],
                                header_bg,
                                egui::Color32::from_rgb(20, 20, 22),
                                false,
                                true,
                                false,
                            );
                        }
                    }
                });

                // Data Rows (starting at Excel row 4)
                let n_rows = self.rows.len();
                for r_idx in 0..n_rows {
                    let excel_r = r_idx + 4;
                    let is_sel = self.selected_row == Some(r_idx);

                    ui.horizontal(|ui| {
                        row_num_cell(ui, excel_r, row_num_w);

                        // A: Chemical Formula (editable)
                        widget_cell(ui, col_widths[0], egui::Color32::WHITE, is_sel, |ui| {
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.rows[r_idx].formula)
                                    .desired_width(col_widths[0] - 8.0),
                            );
                            if resp.gained_focus() {
                                self.selected_row = Some(r_idx);
                            }
                            if resp.changed() {
                                recalc_idx = Some(r_idx);
                            }
                            resp.context_menu(|ui| {
                                if ui.button("Insert Row").clicked() {
                                    insert_at = Some(r_idx);
                                    ui.close_menu();
                                }
                                if ui.button("Delete Row").clicked() {
                                    delete_at = Some(r_idx);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Browse HSC Database...").clicked() {
                                    self.show_db_picker = true;
                                    ui.close_menu();
                                }
                                if ui.button("Graphics -> Parity Chart").clicked() {
                                    open_graph_mode = Some(EstimatePlotMode::Parity);
                                    ui.close_menu();
                                }
                                if ui.button("Save to HSC Database -> Cp Range").clicked() {
                                    open_cp_at = Some(r_idx);
                                    ui.close_menu();
                                }
                            });
                        });

                        // B: Temp (editable)
                        let mut t_disp = if self.use_kelvin {
                            self.rows[r_idx].temp_c + 273.15
                        } else {
                            self.rows[r_idx].temp_c
                        };
                        widget_cell(ui, col_widths[1], egui::Color32::WHITE, false, |ui| {
                            if !self.rows[r_idx].formula.trim().is_empty()
                                && ui
                                    .add(
                                        egui::DragValue::new(&mut t_disp)
                                            .speed(1.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                            {
                                self.rows[r_idx].temp_c = if self.use_kelvin {
                                    t_disp - 273.15
                                } else {
                                    t_disp
                                };
                                recalc_idx = Some(r_idx);
                            }
                        });

                        // C: Species Type (toggleable)
                        if col_widths[2] > 0.0 {
                            let type_txt = if self.rows[r_idx].formula.trim().is_empty() {
                                String::new()
                            } else {
                                self.rows[r_idx].type_display()
                            };
                            if text_cell(
                                ui,
                                &type_txt,
                                col_widths[2],
                                EXCEL_READONLY_BG,
                                egui::Color32::from_rgb(24, 24, 27),
                                false,
                                false,
                                false,
                            )
                            .clicked()
                            {
                                self.selected_row = Some(r_idx);
                            }
                        }

                        // D: Selected set of oxidation numbers
                        let sets = self.rows[r_idx].available_sets;
                        if !self.rows[r_idx].formula.trim().is_empty() && sets > 1 {
                            widget_cell(ui, col_widths[3], egui::Color32::WHITE, false, |ui| {
                                let cur_set = self.rows[r_idx].oxidation_set_idx + 1;
                                let mut chosen = self.rows[r_idx].oxidation_set_idx;
                                egui::ComboBox::from_id_salt(format!("ox_set_{r_idx}"))
                                    .width(col_widths[3] - 10.0)
                                    .selected_text(format!("Set {cur_set}"))
                                    .show_ui(ui, |ui| {
                                        for s in 0..sets {
                                            ui.selectable_value(
                                                &mut chosen,
                                                s,
                                                format!("Set {}", s + 1),
                                            );
                                        }
                                    });
                                if chosen != self.rows[r_idx].oxidation_set_idx {
                                    self.rows[r_idx].oxidation_set_idx = chosen;
                                    recalc_idx = Some(r_idx);
                                }
                            });
                        } else {
                            text_cell(
                                ui,
                                "",
                                col_widths[3],
                                EXCEL_READONLY_BG,
                                egui::Color32::BLACK,
                                false,
                                false,
                                false,
                            );
                        }

                        // E..M: MW, H(Est/DB), S(Est/DB), Cp(Est/DB), ΔG(Est/DB)
                        let row = &self.rows[r_idx];
                        let has_f = !row.formula.trim().is_empty();
                        let scale = if self.per_kg && row.mw > 0.0 {
                            1000.0 / row.mw
                        } else {
                            1.0
                        };

                        let fmt_opt = |v: Option<f64>| -> (String, egui::Color32) {
                            match v {
                                Some(val) if has_f => {
                                    (format!("{:.2}", val * scale), egui::Color32::from_rgb(17, 24, 39))
                                }
                                _ if has_f => ("N/A".to_string(), egui::Color32::from_rgb(37, 99, 235)),
                                _ => (String::new(), egui::Color32::BLACK),
                            }
                        };

                        let mw_str = if has_f {
                            format!("{:.2}", row.mw)
                        } else {
                            String::new()
                        };
                        text_cell(
                            ui,
                            &mw_str,
                            col_widths[4],
                            EXCEL_READONLY_BG,
                            egui::Color32::from_rgb(17, 24, 39),
                            true,
                            false,
                            false,
                        );

                        let pairs = [
                            fmt_opt(row.est_h_kj_mol),
                            fmt_opt(row.db_vals.as_ref().map(|d| d.h_kj_mol)),
                            fmt_opt(row.est_s_j_molk),
                            fmt_opt(row.db_vals.as_ref().map(|d| d.s_j_molk)),
                            fmt_opt(row.est_cp_j_molk),
                            fmt_opt(row.db_vals.as_ref().map(|d| d.cp_j_molk)),
                            fmt_opt(row.est_dg_kj_mol),
                            fmt_opt(row.db_vals.as_ref().map(|d| d.dg_kj_mol)),
                        ];
                        for (offset, (txt, col)) in pairs.iter().enumerate() {
                            text_cell(
                                ui,
                                txt,
                                col_widths[5 + offset],
                                EXCEL_READONLY_BG,
                                *col,
                                true,
                                false,
                                false,
                            );
                        }

                        // N..AB: Element Information (up to 3 elements)
                        for e_idx in 0..3 {
                            let base_col = 13 + e_idx * 5;
                            if has_f {
                                if let Some(el) = row.elements.get(e_idx) {
                                    text_cell(
                                        ui,
                                        &el.symbol,
                                        col_widths[base_col],
                                        EXCEL_READONLY_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        false,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        &format!("{:.0}", el.amount),
                                        col_widths[base_col + 1],
                                        EXCEL_READONLY_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        true,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        &format!("{}", el.charge),
                                        col_widths[base_col + 2],
                                        EXCEL_READONLY_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        true,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        &format!("{:.2}", el.weight_pct),
                                        col_widths[base_col + 3],
                                        EXCEL_READONLY_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        true,
                                        false,
                                        false,
                                    );
                                    text_cell(
                                        ui,
                                        &format!("{:.2}", el.atom_pct),
                                        col_widths[base_col + 4],
                                        EXCEL_READONLY_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        true,
                                        false,
                                        false,
                                    );
                                    continue;
                                }
                            }
                            for k in 0..5 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[base_col + k],
                                    EXCEL_READONLY_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                        }
                    });
                }

                // Trailing empty rows up to row 24
                let mut extra_r = n_rows + 4;
                while extra_r <= 24 {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, extra_r, row_num_w);
                        for (idx, w) in col_widths.iter().enumerate() {
                            if *w > 0.0 {
                                let bg = if idx <= 1 {
                                    egui::Color32::WHITE
                                } else {
                                    EXCEL_READONLY_BG
                                };
                                text_cell(
                                    ui,
                                    "",
                                    *w,
                                    bg,
                                    egui::Color32::BLACK,
                                    false,
                                    false,
                                    false,
                                );
                            }
                        }
                    });
                    extra_r += 1;
                }
            });

        if let Some(idx) = recalc_idx {
            if let Some(row) = self.rows.get_mut(idx) {
                evaluate_estimate_row(row, db);
            }
        }
        if let Some(idx) = insert_at {
            self.rows.insert(idx, EstimateRow::new(""));
        }
        if let Some(idx) = delete_at {
            if self.rows.len() > 1 {
                self.rows.remove(idx);
            }
        }
        if let Some(mode) = open_graph_mode {
            self.plot_mode = mode;
            self.show_graphics_win = true;
        }
        if let Some(idx) = open_cp_at {
            self.cp_target_row = idx;
            self.show_cp_range_dialog = true;
        }
    }

    fn render_db_picker(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_db_picker;
        let mut insert_formula: Option<String> = None;

        egui::Window::new("Browse HSC Database — Insert Species")
            .open(&mut open)
            .default_size([480.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Buscar fórmula:");
                    ui.text_edit_singleline(&mut self.db_search);
                });
                ui.separator();

                let matches = db.search(self.db_search.trim(), 150);
                egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
                    for sp in matches {
                        ui.horizontal(|ui| {
                            if ui.button("Insert").clicked() {
                                insert_formula = Some(sp.formula.clone());
                            }
                            ui.label(egui::RichText::new(&sp.formula).strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:?} · {:.2} g/mol",
                                    sp.phase, sp.molecular_weight
                                ))
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                        });
                    }
                });
            });

        self.show_db_picker = open;
        if let Some(f) = insert_formula {
            let mut r = EstimateRow::new(f);
            evaluate_estimate_row(&mut r, db);
            let idx = self.selected_row.unwrap_or(0).min(self.rows.len());
            self.rows.insert(idx, r);
            self.selected_row = Some(idx);
        }
    }

    fn ensure_db_scatter_cache(&mut self, db: &ThermoDatabase) {
        if self.scatter_scope == EstimateScatterScope::SheetRows {
            return;
        }
        if self.cached_db_scope == Some(self.scatter_scope) {
            return;
        }
        let species_list = db.search("", 20_000);
        let mut evaluated = Vec::new();
        for sp in species_list {
            if sp.formula == "e-" || sp.formula == "e(-)" {
                continue;
            }
            let keep = match self.scatter_scope {
                EstimateScatterScope::SheetRows => false,
                EstimateScatterScope::GasDb => sp.phase == Phase::Gas,
                EstimateScatterScope::AqueousDb => {
                    sp.phase == Phase::AqueousIon || sp.phase == Phase::AqueousNeutral
                }
                EstimateScatterScope::CondensedDb => {
                    sp.phase == Phase::Solid || sp.phase == Phase::Liquid
                }
                EstimateScatterScope::AllDb => true,
            };
            if !keep {
                continue;
            }
            let mut row = EstimateRow::new(sp.formula);
            evaluate_estimate_row(&mut row, db);
            if row.mw > 0.0 && row.db_vals.is_some() {
                evaluated.push(row);
            }
        }
        self.cached_db_rows = evaluated;
        self.cached_db_scope = Some(self.scatter_scope);
    }

    fn render_graphics_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        self.ensure_db_scatter_cache(db);
        let mut open = self.show_graphics_win;
        let mut close_clicked = false;
        egui::Window::new("HSC Estimate — Graphics")
            .open(&mut open)
            .default_size([860.0, 560.0])
            .show(ctx, |ui| {
                // Top Fig. 5 toolbar
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.plot_mode, EstimatePlotMode::Parity, "📈 Parity");
                    ui.selectable_value(
                        &mut self.plot_mode,
                        EstimatePlotMode::Comparison,
                        "📊 Comparison",
                    );
                    ui.selectable_value(
                        &mut self.plot_mode,
                        EstimatePlotMode::Differences,
                        "📉 Differences",
                    );
                    ui.separator();
                    ui.label("Scope:");
                    ui.selectable_value(
                        &mut self.scatter_scope,
                        EstimateScatterScope::SheetRows,
                        "Sheet",
                    );
                    ui.selectable_value(
                        &mut self.scatter_scope,
                        EstimateScatterScope::GasDb,
                        "Gas (DB)",
                    );
                    ui.selectable_value(
                        &mut self.scatter_scope,
                        EstimateScatterScope::AqueousDb,
                        "Aqueous (DB)",
                    );
                    ui.selectable_value(
                        &mut self.scatter_scope,
                        EstimateScatterScope::CondensedDb,
                        "Condensed (DB)",
                    );
                    ui.selectable_value(
                        &mut self.scatter_scope,
                        EstimateScatterScope::AllDb,
                        "All DB",
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖ Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.separator();

                let source_rows: &[EstimateRow] =
                    if self.scatter_scope == EstimateScatterScope::SheetRows {
                        &self.rows
                    } else {
                        &self.cached_db_rows
                    };

                let mut h_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut s_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut cp_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut dg_pairs: Vec<(&str, f64, f64)> = Vec::new();

                for r in source_rows {
                    if r.formula.trim().is_empty() {
                        continue;
                    }
                    if let Some(ref dbv) = r.db_vals {
                        if let Some(v) = r.est_h_kj_mol {
                            h_pairs.push((&r.formula, v, dbv.h_kj_mol));
                        }
                        if let Some(v) = r.est_s_j_molk {
                            s_pairs.push((&r.formula, v, dbv.s_j_molk));
                        }
                        if let Some(v) = r.est_cp_j_molk {
                            cp_pairs.push((&r.formula, v, dbv.cp_j_molk));
                        }
                        if let Some(v) = r.est_dg_kj_mol {
                            dg_pairs.push((&r.formula, v, dbv.dg_kj_mol));
                        }
                    }
                }

                let active_pairs = match self.active_stat_prop {
                    0 => &h_pairs,
                    1 => &s_pairs,
                    2 => &cp_pairs,
                    _ => &dg_pairs,
                };
                let active_name = match self.active_stat_prop {
                    0 => "H - Enthalpy",
                    1 => "S - Entropy",
                    2 => "Cp - Heat Capacity",
                    _ => "ΔG - Gibbs Energy",
                };
                let stats = compute_parity_stats(active_pairs);
                let is_large = self.scatter_scope != EstimateScatterScope::SheetRows;
                let pt_color = if is_large {
                    egui::Color32::from_rgb(220, 38, 38)
                } else {
                    egui::Color32::from_rgb(37, 99, 235)
                };
                let pt_radius = if is_large { 1.8 } else { 4.2 };

                let total_w = ui.available_width();
                let prop_w = 232.0;
                let charts_w = (total_w - prop_w - 14.0).max(460.0);
                let card_w = (charts_w - 10.0) * 0.5;
                let mut clicked_prop: Option<usize> = None;

                ui.horizontal_top(|ui| {
                    // Left 2x2 Chart Grid (Fig. 5)
                    ui.vertical(|ui| {
                        ui.set_width(charts_w);
                        let mut draw_card = |ui: &mut egui::Ui,
                                             prop_idx: usize,
                                             id: &str,
                                             header_title: &str,
                                             plot_title: &str,
                                             pairs: &[(&str, f64, f64)]| {
                            let is_active = self.active_stat_prop == prop_idx;
                            let border_col = if is_active {
                                egui::Color32::from_rgb(219, 39, 119)
                            } else {
                                EXCEL_HEADER_BORDER
                            };
                            let frame_resp = egui::Frame::none()
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(if is_active { 1.8 } else { 1.0 }, border_col))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    ui.set_width(card_w - 14.0);
                                    if ui
                                        .add(
                                            egui::Label::new(
                                                egui::RichText::new(header_title)
                                                    .strong()
                                                    .size(11.5)
                                                    .color(if is_active {
                                                        egui::Color32::from_rgb(157, 23, 77)
                                                    } else {
                                                        egui::Color32::from_rgb(55, 65, 81)
                                                    }),
                                            )
                                            .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        clicked_prop = Some(prop_idx);
                                    }
                                    ui.label(
                                        egui::RichText::new(plot_title)
                                            .strong()
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(17, 24, 39)),
                                    );
                                    let mut plot = Plot::new(id)
                                        .height(205.0)
                                        .x_axis_label(if is_large {
                                            "Experimental"
                                        } else {
                                            "Database"
                                        })
                                        .y_axis_label(if is_large {
                                            "Estimated"
                                        } else {
                                            "Estimate"
                                        });
                                    if self.plot_mode == EstimatePlotMode::Parity {
                                        plot = plot.data_aspect(1.0);
                                    }
                                    plot.show(ui, |plot_ui| {
                                        if !pairs.is_empty() {
                                            let min_v = pairs
                                                .iter()
                                                .flat_map(|p| [p.1, p.2])
                                                .fold(f64::INFINITY, f64::min);
                                            let max_v = pairs
                                                .iter()
                                                .flat_map(|p| [p.1, p.2])
                                                .fold(f64::NEG_INFINITY, f64::max);
                                            match self.plot_mode {
                                                EstimatePlotMode::Parity => {
                                                    plot_ui.line(
                                                        Line::new(PlotPoints::from(vec![
                                                            [min_v, min_v],
                                                            [max_v, max_v],
                                                        ]))
                                                        .color(egui::Color32::from_rgb(
                                                            22, 163, 74,
                                                        )),
                                                    );
                                                    let pts: Vec<[f64; 2]> = pairs
                                                        .iter()
                                                        .map(|p| [p.2, p.1])
                                                        .collect();
                                                    plot_ui.points(
                                                        Points::new(PlotPoints::from(pts))
                                                            .radius(pt_radius)
                                                            .color(pt_color),
                                                    );
                                                }
                                                EstimatePlotMode::Comparison => {
                                                    let est_pts: Vec<[f64; 2]> = pairs
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, p)| [i as f64, p.1])
                                                        .collect();
                                                    let db_pts: Vec<[f64; 2]> = pairs
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, p)| [i as f64, p.2])
                                                        .collect();
                                                    plot_ui.points(
                                                        Points::new(PlotPoints::from(est_pts))
                                                            .radius(pt_radius)
                                                            .name("Estimate"),
                                                    );
                                                    plot_ui.points(
                                                        Points::new(PlotPoints::from(db_pts))
                                                            .radius(pt_radius)
                                                            .name("Database"),
                                                    );
                                                }
                                                EstimatePlotMode::Differences => {
                                                    let diff_pts: Vec<[f64; 2]> = pairs
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, p)| [i as f64, p.1 - p.2])
                                                        .collect();
                                                    plot_ui.points(
                                                        Points::new(PlotPoints::from(diff_pts))
                                                            .radius(pt_radius)
                                                            .color(pt_color),
                                                    );
                                                }
                                            }
                                        }
                                    });
                                });
                            if frame_resp.response.clicked() {
                                clicked_prop = Some(prop_idx);
                            }
                        };

                        ui.horizontal(|ui| {
                            draw_card(
                                ui,
                                0,
                                "est_plot_h",
                                "H - Enthalpy",
                                "H (25 °C) - kJ/mol",
                                &h_pairs,
                            );
                            draw_card(
                                ui,
                                1,
                                "est_plot_s",
                                "S - Entropy",
                                "S (25 °C) - J/mol*K",
                                &s_pairs,
                            );
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            draw_card(
                                ui,
                                2,
                                "est_plot_cp",
                                "Cp - Heat Capacity",
                                "Cp - J/mol*K",
                                &cp_pairs,
                            );
                            draw_card(
                                ui,
                                3,
                                "est_plot_dg",
                                "ΔG - Gibbs Energy",
                                "ΔG (25 °C) - kJ/mol",
                                &dg_pairs,
                            );
                        });
                    });

                    // Right Chart Properties / Statistics Panel (Fig. 5)
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(245, 246, 248))
                        .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.set_width(prop_w - 16.0);
                            ui.label(egui::RichText::new("Chart Properties").strong().size(12.5));
                            ui.separator();
                            ui.label(egui::RichText::new("Chart").strong().small());
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                ui.label(egui::RichText::new(active_name).strong());
                            });
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Statistics").strong().small());
                            egui::Grid::new("est_stats_grid")
                                .num_columns(2)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Species Count");
                                    ui.label(
                                        egui::RichText::new(format!("{}", stats.species_count))
                                            .strong(),
                                    );
                                    ui.end_row();
                                    ui.label("Maximum Error");
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", stats.max_error))
                                            .strong(),
                                    );
                                    ui.end_row();
                                    ui.label("Max Error Species");
                                    ui.label(
                                        egui::RichText::new(&stats.max_error_species).strong(),
                                    );
                                    ui.end_row();
                                    ui.label("Mean Error");
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", stats.mean_error))
                                            .strong(),
                                    );
                                    ui.end_row();
                                    ui.label("Median Error");
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", stats.median_error))
                                            .strong(),
                                    );
                                    ui.end_row();
                                    ui.label("RMSE");
                                    ui.label(
                                        egui::RichText::new(format!("{:.3}", stats.rmse)).strong(),
                                    );
                                    ui.end_row();
                                });
                        });
                });

                if let Some(p) = clicked_prop {
                    self.active_stat_prop = p;
                }
            });
        self.show_graphics_win = open && !close_clicked;
    }

    fn render_cp_range_dialog(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_cp_range_dialog;
        let mut run_fit = false;
        let mut close_now = false;

        let sp_name = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.formula.clone())
            .unwrap_or_default();

        egui::Window::new("Cp Range Data")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Parameters").strong());
                    egui::Grid::new("cp_range_params").num_columns(2).show(ui, |ui| {
                        ui.label("Species");
                        ui.label(egui::RichText::new(&sp_name).strong());
                        ui.end_row();
                        ui.label("Tmin (K)");
                        ui.add(egui::DragValue::new(&mut self.cp_tmin_k).speed(1.0));
                        ui.end_row();
                        ui.label("Tmax (K)");
                        ui.add(egui::DragValue::new(&mut self.cp_tmax_k).speed(1.0));
                        ui.end_row();
                        ui.label("Steps");
                        ui.add(egui::DragValue::new(&mut self.cp_steps).range(5..=500));
                        ui.end_row();
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        run_fit = true;
                        close_now = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close_now = true;
                    }
                });
            });

        self.show_cp_range_dialog = open && !close_now;
        if run_fit {
            self.open_cp_data_for_row(self.cp_target_row, db);
        }
    }

    fn render_cp_data_window(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        let mut open = self.show_cp_data_win;
        let mut do_update_species = false;
        let mut do_refit = false;

        let sp_name = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.formula.clone())
            .unwrap_or_default();
        let sp_mw = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.mw)
            .unwrap_or(0.0);

        egui::Window::new("HSC Cp Data")
            .open(&mut open)
            .default_size([840.0, 520.0])
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    // Left: Experimental / Estimate table
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Species: {sp_name}   ·   MW: {sp_mw:.3} g/mol"
                            ))
                            .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                            egui::Grid::new("hsc_cp_data_table")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("T (K)").strong());
                                    ui.label(egui::RichText::new("Cp Estimate").strong());
                                    ui.label(egui::RichText::new("Cp Fitting").strong());
                                    ui.label(egui::RichText::new("Error").strong());
                                    ui.end_row();

                                    for &(t, cp_est) in &self.cp_points {
                                        let cp_fit = self.cp_kelley.cp(t);
                                        ui.label(format!("{t:.2}"));
                                        ui.label(format!("{cp_est:.3}"));
                                        ui.label(format!("{cp_fit:.3}"));
                                        ui.label(format!("{:.3}", cp_fit - cp_est));
                                        ui.end_row();
                                    }
                                });
                        });
                    });

                    // Right: Fitted Kelley coefficients + Chart
                    cols[1].vertical(|ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Cp(T) = A + B*T*10^-3 + C*T^-2*10^5 + D*T^2*10^-6",
                                    )
                                    .monospace()
                                    .small(),
                                );
                                if ui.button("Update Species").clicked() {
                                    do_update_species = true;
                                }
                            });
                            ui.separator();
                            egui::Grid::new("hsc_cp_fitted_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Tmin (K)");
                                    ui.label(format!("{:.2}", self.cp_tmin_k));
                                    ui.end_row();
                                    ui.label("Tmax (K)");
                                    ui.label(format!("{:.2}", self.cp_tmax_k));
                                    ui.end_row();
                                    ui.label("Cp coefficient A J/(mol*K)");
                                    ui.label(format!("{:.3}", self.cp_kelley.a));
                                    ui.end_row();
                                    ui.label("Cp coefficient B");
                                    ui.label(format!("{:.3}", self.cp_kelley.b));
                                    ui.end_row();
                                    ui.label("Cp coefficient C");
                                    ui.label(format!("{:.3}", self.cp_kelley.c));
                                    ui.end_row();
                                    ui.label("Cp coefficient D");
                                    ui.label(format!("{:.3}", self.cp_kelley.d));
                                    ui.end_row();
                                });
                            ui.horizontal(|ui| {
                                if ui.button("Fit Cp").clicked() {
                                    do_refit = true;
                                }
                                if ui.button("Estimate Cp").clicked() {
                                    do_refit = true;
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Cp Heat Capacity").strong());
                            Plot::new("hsc_cp_fit_plot")
                                .height(220.0)
                                .legend(Legend::default())
                                .show(ui, |plot_ui| {
                                    let est_pts: Vec<[f64; 2]> =
                                        self.cp_points.iter().map(|&(t, c)| [t, c]).collect();
                                    let fit_pts: Vec<[f64; 2]> = self
                                        .cp_points
                                        .iter()
                                        .map(|&(t, _)| [t, self.cp_kelley.cp(t)])
                                        .collect();
                                    plot_ui.line(
                                        Line::new(PlotPoints::from(est_pts))
                                            .name("Estimated")
                                            .color(egui::Color32::from_rgb(22, 163, 74)),
                                    );
                                    plot_ui.line(
                                        Line::new(PlotPoints::from(fit_pts))
                                            .name("Fitted")
                                            .color(egui::Color32::from_rgb(37, 99, 235)),
                                    );
                                });
                        });
                    });
                });
            });

        self.show_cp_data_win = open;
        if do_refit {
            self.cp_kelley = fit_kelley_cp(&self.cp_points);
        }
        if do_update_species {
            if let Some(row) = self.rows.get(self.cp_target_row) {
                if !row.formula.trim().is_empty() {
                    let sp = build_estimated_species(
                        row,
                        self.cp_tmin_k,
                        self.cp_tmax_k,
                        self.cp_kelley,
                    );
                    let f = sp.formula.clone();
                    db.insert_species(sp);
                    self.recalc_all(db);
                    self.status_msg =
                        format!("Especie '{f}' actualizada con coeficientes Kelley en sesión");
                }
            }
        }
    }
}
