//! Module 22 — Material Database UI (HSC Chemistry 10, Chapter 90, Figs. 1–3).
//!
//! Full desktop editor implementing:
//! - Material catalog browser with search, category filter, New/Duplicate/Delete, JSON Save/Open & CSV Export
//! - Left editor (Fig. 1): `Composition` (recursive `[SubMaterial]` + Species + Elements wt-% breakdown) & `Analyses`
//! - Right editor (Figs. 2–3): `Common` (`General Info`, `Properties` with live `Auto Calculate`, `Data`, `User Properties`),
//!   `Cp Data` (6-term HSC polynomial ranges + Auto-Fit Cp from Species DB), and interactive `Cp Chart`.

use eframe::egui;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::material_db::{
    eval_material_cp_and_h, sample_cp_chart_points, AnalysisEntry, CompositionEntry,
    MaterialCategory, MaterialCpRange, MaterialDataRow, MaterialDatabase, MaterialRecord,
    PropertyField, ResolvedComposition, UserProperty,
};
use soryntel_thermo::SpeciesCatalogPort;

use crate::excel_grid::{header_cell, row_num_cell, widget_cell, EXCEL_GRID_BORDER};
use crate::ui_common::{draw_rotated_y_title, format_tick, nice_step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    Composition,
    Analyses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightTopTab {
    Common,
    CpData,
    CpChart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommonSubTab {
    GeneralInfo,
    Properties,
    Data,
    UserProperties,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileModalMode {
    None,
    OpenJson,
    SaveJson,
    ExportCsv,
}

pub struct MaterialDbModuleState {
    pub mat_db: MaterialDatabase,
    pub selected_idx: usize,
    catalog_search: String,
    catalog_category: Option<MaterialCategory>,
    show_catalog: bool,
    left_tab: LeftTab,
    right_top_tab: RightTopTab,
    common_sub_tab: CommonSubTab,

    // Active cell & formula bar trackers
    left_active_cell: (usize, usize),
    prop_active_cell: (usize, usize),
    cp_active_cell: (usize, usize),

    // Quick Cp(T) evaluator temperature in K
    eval_temp_str: String,

    // Species picker modal
    show_species_picker: bool,
    species_query: String,

    // File modal
    file_modal: FileModalMode,
    file_path_str: String,

    // Status feedback
    status_msg: Option<String>,
    status_err: Option<String>,
    initialized_auto_calc: bool,
}

impl Default for MaterialDbModuleState {
    fn default() -> Self {
        Self {
            mat_db: MaterialDatabase::with_default_catalog(),
            selected_idx: 0,
            catalog_search: String::new(),
            catalog_category: None,
            show_catalog: true,
            left_tab: LeftTab::Composition,
            right_top_tab: RightTopTab::Common,
            common_sub_tab: CommonSubTab::Properties,
            left_active_cell: (2, 0),
            prop_active_cell: (1, 2),
            cp_active_cell: (5, 1),
            eval_temp_str: "500".into(),
            show_species_picker: false,
            species_query: String::new(),
            file_modal: FileModalMode::None,
            file_path_str: "/tmp/hsc_materials.json".into(),
            status_msg: Some(
                "Material Database ready (Ch. 90). Edit Composition or Properties — Auto Calculate updates live."
                    .into(),
            ),
            status_err: None,
            initialized_auto_calc: false,
        }
    }
}

impl MaterialDbModuleState {
    fn ensure_valid_selection(&mut self) {
        if self.mat_db.materials.is_empty() {
            self.mat_db
                .materials
                .push(MaterialRecord::new_blank("New Material"));
        }
        if self.selected_idx >= self.mat_db.materials.len() {
            self.selected_idx = self.mat_db.materials.len() - 1;
        }
    }

    fn refresh_selected_auto_calc(&mut self, thermo_db: &ThermoDatabase) {
        self.ensure_valid_selection();
        let mut cloned = self.mat_db.materials[self.selected_idx].clone();
        self.mat_db.apply_auto_calculate(&mut cloned, thermo_db);
        self.mat_db.materials[self.selected_idx] = cloned;
    }

    fn refresh_all_auto_calc(&mut self, thermo_db: &ThermoDatabase) {
        for i in 0..self.mat_db.materials.len() {
            let mut cloned = self.mat_db.materials[i].clone();
            self.mat_db.apply_auto_calculate(&mut cloned, thermo_db);
            self.mat_db.materials[i] = cloned;
        }
    }

    fn create_new_material(&mut self, thermo_db: &ThermoDatabase) {
        let mut n = self.mat_db.materials.len() + 1;
        let mut name = format!("Material {n}");
        while self.mat_db.find_by_name(&name).is_some() {
            n += 1;
            name = format!("Material {n}");
        }
        let mut m = MaterialRecord::new_blank(name.clone());
        m.composition.push(CompositionEntry::new("SiO2", 100.0));
        self.mat_db.materials.push(m);
        self.selected_idx = self.mat_db.materials.len() - 1;
        self.refresh_selected_auto_calc(thermo_db);
        self.status_err = None;
        self.status_msg = Some(format!("Created new material '{name}'."));
    }

    fn duplicate_selected_material(&mut self, thermo_db: &ThermoDatabase) {
        self.ensure_valid_selection();
        let mut copy = self.mat_db.materials[self.selected_idx].clone();
        let base = copy.name.clone();
        let mut k = 1;
        let mut candidate = format!("{base} (Copy)");
        while self.mat_db.find_by_name(&candidate).is_some() {
            k += 1;
            candidate = format!("{base} (Copy {k})");
        }
        copy.name = candidate.clone();
        self.mat_db.materials.push(copy);
        self.selected_idx = self.mat_db.materials.len() - 1;
        self.refresh_selected_auto_calc(thermo_db);
        self.status_err = None;
        self.status_msg = Some(format!("Duplicated material as '{candidate}'."));
    }

    fn delete_selected_material(&mut self, thermo_db: &ThermoDatabase) {
        if self.mat_db.materials.len() <= 1 {
            self.status_err = Some("Cannot delete the last material in the database.".into());
            return;
        }
        let removed = self.mat_db.materials.remove(self.selected_idx);
        if self.selected_idx >= self.mat_db.materials.len() {
            self.selected_idx = self.mat_db.materials.len() - 1;
        }
        self.refresh_selected_auto_calc(thermo_db);
        self.status_err = None;
        self.status_msg = Some(format!("Deleted material '{}'.", removed.name));
    }

    fn normalize_selected(&mut self, thermo_db: &ThermoDatabase) {
        self.ensure_valid_selection();
        if self.mat_db.materials[self.selected_idx].normalize_composition() {
            self.refresh_selected_auto_calc(thermo_db);
            self.status_err = None;
            self.status_msg = Some("Composition normalized to 100.000 wt-%.".into());
        } else {
            self.status_err = Some("Add at least one positive composition entry to normalize.".into());
        }
    }

    fn auto_fit_cp_selected(&mut self, thermo_db: &ThermoDatabase) {
        self.ensure_valid_selection();
        let mat = self.mat_db.materials[self.selected_idx].clone();
        match self.mat_db.auto_fit_cp_from_composition(&mat, thermo_db) {
            Ok(range) => {
                self.mat_db.materials[self.selected_idx].cp_ranges = vec![range];
                self.right_top_tab = RightTopTab::CpData;
                self.status_err = None;
                self.status_msg = Some(
                    "Generated mass-weighted Cp polynomial (298.15–1000 K) from species database."
                        .into(),
                );
            }
            Err(e) => self.status_err = Some(e),
        }
    }

    fn sync_elements_to_analyses(&mut self) {
        self.ensure_valid_selection();
        let mat = self.mat_db.materials[self.selected_idx].clone();
        let res = self.mat_db.resolve_composition(&mat);
        let target = &mut self.mat_db.materials[self.selected_idx].analyses;
        for (el, wt) in res.elements {
            let param = format!("Element {el}");
            if let Some(existing) = target.iter_mut().find(|a| a.parameter == param) {
                existing.value = format!("{wt:.3}");
            } else {
                target.push(AnalysisEntry {
                    parameter: param,
                    value: format!("{wt:.3}"),
                    unit: "wt-%".into(),
                    method: "Composition Breakdown".into(),
                    reference: "Calculated".into(),
                });
            }
        }
        self.left_tab = LeftTab::Analyses;
        self.status_err = None;
        self.status_msg = Some("Synchronized calculated elemental assays into Analyses tab.".into());
    }

    fn save_json(&mut self, path: &str) {
        match self.mat_db.to_json() {
            Ok(json) => match std::fs::write(path, json) {
                Ok(()) => {
                    self.status_err = None;
                    self.status_msg = Some(format!(
                        "Saved {} materials to {path}",
                        self.mat_db.materials.len()
                    ));
                }
                Err(e) => self.status_err = Some(format!("Write error: {e}")),
            },
            Err(e) => self.status_err = Some(e),
        }
    }

    fn open_json(&mut self, path: &str, thermo_db: &ThermoDatabase) {
        match std::fs::read_to_string(path) {
            Ok(raw) => match MaterialDatabase::from_json(&raw) {
                Ok(db) if !db.materials.is_empty() => {
                    let count = db.materials.len();
                    self.mat_db = db;
                    self.selected_idx = 0;
                    self.refresh_all_auto_calc(thermo_db);
                    self.status_err = None;
                    self.status_msg = Some(format!("Loaded {count} materials from {path}"));
                }
                Ok(_) => self.status_err = Some("Database file contains 0 materials.".into()),
                Err(e) => self.status_err = Some(e),
            },
            Err(e) => self.status_err = Some(format!("Read error: {e}")),
        }
    }

    fn export_csv(&mut self, path: &str) {
        self.ensure_valid_selection();
        let mat = self.mat_db.materials[self.selected_idx].clone();
        let csv = self.mat_db.export_material_csv(&mat);
        match std::fs::write(path, csv) {
            Ok(()) => {
                self.status_err = None;
                self.status_msg = Some(format!("Exported '{}' to {path}", mat.name));
            }
            Err(e) => self.status_err = Some(format!("Export error: {e}")),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, thermo_db: &ThermoDatabase) {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = egui::Color32::from_rgb(242, 245, 250);
        vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(vis);

        self.ensure_valid_selection();
        if !self.initialized_auto_calc {
            self.refresh_all_auto_calc(thermo_db);
            self.initialized_auto_calc = true;
        }

        // Top Ribbon
        self.ui_ribbon(ctx, thermo_db);

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("matdb_footer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 238, 244))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Soryntel Material Database  |  Chapter 90  |  Materials: {}  |  Species DB: {}",
                            self.mat_db.materials.len(),
                            thermo_db.len()
                        ))
                        .size(10.5)
                        .color(egui::Color32::from_rgb(75, 85, 99)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(err) = &self.status_err {
                            ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                        } else if let Some(msg) = &self.status_msg {
                            ui.colored_label(egui::Color32::from_rgb(21, 128, 61), msg);
                        }
                    });
                });
            });

        // Optional Left Catalog Sidebar
        if self.show_catalog {
            self.ui_catalog_sidebar(ctx, thermo_db);
        }

        // Main Workspace: Left Composition/Analyses (Fig. 1) + Right Common/CpData/CpChart (Figs. 2–3)
        let mat_snapshot = self.mat_db.materials[self.selected_idx].clone();
        let resolved = self.mat_db.resolve_composition(&mat_snapshot);
        let mut needs_recalc = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 240, 246))
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                let avail_w = ui.available_width();
                let avail_h = ui.available_height();
                let left_w = (avail_w * 0.51).clamp(380.0, avail_w - 380.0).max(300.0);
                let right_w = (avail_w - left_w - 8.0).max(300.0);

                ui.horizontal_top(|ui| {
                    // LEFT PANE: Fig. 1 (Composition & Analyses)
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, avail_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            if self.ui_left_pane(ui, &resolved) {
                                needs_recalc = true;
                            }
                        },
                    );

                    ui.add_space(6.0);

                    // RIGHT PANE: Figs. 2 & 3 (Common / Cp Data / Cp Chart)
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_w, avail_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            if self.ui_right_pane(ui, thermo_db) {
                                needs_recalc = true;
                            }
                        },
                    );
                });
            });

        if needs_recalc {
            self.refresh_selected_auto_calc(thermo_db);
        }

        // Modals
        self.ui_species_picker_modal(ctx, thermo_db);
        self.ui_file_modal(ctx, thermo_db);
    }

    fn ui_ribbon(&mut self, ctx: &egui::Context, thermo_db: &ThermoDatabase) {
        let all_names: Vec<String> = self
            .mat_db
            .materials
            .iter()
            .map(|m| m.name.clone())
            .collect();
        let current_name = self.mat_db.materials[self.selected_idx].name.clone();

        egui::TopBottomPanel::top("matdb_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 240, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ribbon_group(ui, "Database", |ui| {
                        if ribbon_btn(ui, "☰", "Catalog", self.show_catalog) {
                            self.show_catalog = !self.show_catalog;
                        }
                        if ribbon_btn(ui, "📄", "New", false) {
                            self.create_new_material(thermo_db);
                        }
                        if ribbon_btn(ui, "⧉", "Clone", false) {
                            self.duplicate_selected_material(thermo_db);
                        }
                        if ribbon_btn(ui, "🗑", "Delete", false) {
                            self.delete_selected_material(thermo_db);
                        }
                        if ribbon_btn(ui, "📂", "Open DB", false) {
                            self.file_path_str = "/tmp/hsc_materials.json".into();
                            self.file_modal = FileModalMode::OpenJson;
                        }
                        if ribbon_btn(ui, "💾", "Save DB", false) {
                            self.file_path_str = "/tmp/hsc_materials.json".into();
                            self.file_modal = FileModalMode::SaveJson;
                        }
                        if ribbon_btn(ui, "⭳", "Export\nCSV", false) {
                            let safe = current_name.replace(' ', "_").to_lowercase();
                            self.file_path_str = format!("/tmp/{safe}.csv");
                            self.file_modal = FileModalMode::ExportCsv;
                        }
                    });

                    ribbon_group(ui, "Composition", |ui| {
                        if ribbon_btn(ui, "➕", "Add\nRow", false) {
                            self.mat_db.materials[self.selected_idx]
                                .composition
                                .push(CompositionEntry::new("", 0.0));
                            self.left_tab = LeftTab::Composition;
                        }

                        // Insert [Material] dropdown
                        let mat_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("🧱\n[Material] ▾")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(25, 35, 50)),
                            )
                            .min_size(egui::vec2(66.0, 42.0))
                            .fill(egui::Color32::from_rgb(248, 250, 253)),
                        );
                        let pop_id = ui.make_persistent_id("matdb_insert_submat_popup");
                        if mat_btn.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(pop_id));
                        }
                        egui::popup_below_widget(
                            ui,
                            pop_id,
                            &mat_btn,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(170.0);
                                ui.strong("Insert Sub-Material:");
                                ui.separator();
                                for name in &all_names {
                                    if name == &current_name {
                                        continue;
                                    }
                                    if ui.button(format!("[{name}]")).clicked() {
                                        self.mat_db.materials[self.selected_idx]
                                            .composition
                                            .push(CompositionEntry::new(format!("[{name}]"), 10.0));
                                        self.left_tab = LeftTab::Composition;
                                        self.refresh_selected_auto_calc(thermo_db);
                                        self.status_msg =
                                            Some(format!("Added '[{name}]' (10 wt-%) to composition."));
                                    }
                                }
                            },
                        );

                        if ribbon_btn(ui, "⚗", "Insert\nSpecies", false) {
                            self.show_species_picker = true;
                        }
                        if ribbon_btn(ui, "⚖", "Normalize\n100%", false) {
                            self.normalize_selected(thermo_db);
                        }
                        if ribbon_btn(ui, "↻", "Auto\nCalc", false) {
                            self.refresh_all_auto_calc(thermo_db);
                            self.status_err = None;
                            self.status_msg = Some("Recalculated all Auto Calculate properties.".into());
                        }
                    });

                    ribbon_group(ui, "Cp & Assays", |ui| {
                        if ribbon_btn(ui, "📈", "Auto-Fit\nCp", false) {
                            self.auto_fit_cp_selected(thermo_db);
                        }
                        if ribbon_btn(ui, "➕", "Add Cp\nRange", false) {
                            let ranges = &mut self.mat_db.materials[self.selected_idx].cp_ranges;
                            let next_tmin = ranges.last().map(|r| r.t_max_k).unwrap_or(298.15);
                            ranges.push(MaterialCpRange {
                                t_min_k: next_tmin,
                                t_max_k: next_tmin + 300.0,
                                phase: "s".into(),
                                ..Default::default()
                            });
                            self.right_top_tab = RightTopTab::CpData;
                        }
                        if ribbon_btn(ui, "📋", "Sync\nAssays", false) {
                            self.sync_elements_to_analyses();
                        }
                    });
                });
            });
    }

    fn ui_catalog_sidebar(&mut self, ctx: &egui::Context, thermo_db: &ThermoDatabase) {
        egui::SidePanel::left("matdb_catalog_panel")
            .default_width(215.0)
            .min_width(175.0)
            .max_width(320.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(246, 248, 252))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.strong("Materials Catalog");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 26.0, 22.0],
                        egui::TextEdit::singleline(&mut self.catalog_search)
                            .hint_text("Filter materials..."),
                    );
                    if ui.small_button("✖").clicked() {
                        self.catalog_search.clear();
                    }
                });
                ui.add_space(4.0);
                egui::ComboBox::from_id_salt("matdb_cat_filter")
                    .width(ui.available_width())
                    .selected_text(
                        self.catalog_category
                            .map(|c| c.label())
                            .unwrap_or("All Categories"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.catalog_category, None, "All Categories");
                        for &cat in MaterialCategory::ALL {
                            ui.selectable_value(&mut self.catalog_category, Some(cat), cat.label());
                        }
                    });
                ui.separator();

                let q = self.catalog_search.trim().to_ascii_lowercase();
                let mut clicked_idx: Option<usize> = None;

                egui::ScrollArea::vertical()
                    .id_salt("matdb_catalog_scroll")
                    .show(ui, |ui| {
                        for (idx, m) in self.mat_db.materials.iter().enumerate() {
                            if let Some(cat) = self.catalog_category {
                                if m.category != cat {
                                    continue;
                                }
                            }
                            if !q.is_empty()
                                && !m.name.to_ascii_lowercase().contains(&q)
                                && !m.code.to_ascii_lowercase().contains(&q)
                                && !m.description.to_ascii_lowercase().contains(&q)
                            {
                                continue;
                            }
                            let selected = idx == self.selected_idx;
                            let sum_wt: f64 = m
                                .composition
                                .iter()
                                .filter(|e| !e.item.trim().is_empty())
                                .map(|e| e.wt_pct)
                                .sum();
                            let ok_100 = (sum_wt - 100.0).abs() < 0.01;

                            let fill = if selected {
                                egui::Color32::from_rgb(219, 234, 254)
                            } else {
                                egui::Color32::WHITE
                            };
                            let border = if selected {
                                egui::Color32::from_rgb(37, 99, 235)
                            } else {
                                egui::Color32::from_rgb(214, 220, 230)
                            };
                            let resp = egui::Frame::none()
                                .fill(fill)
                                .stroke(egui::Stroke::new(if selected { 1.4 } else { 0.8 }, border))
                                .rounding(3.0)
                                .inner_margin(egui::Margin::symmetric(7.0, 5.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&m.name)
                                                .strong()
                                                .size(11.5)
                                                .color(egui::Color32::from_rgb(15, 23, 42)),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(m.category.label())
                                                .size(9.5)
                                                .color(egui::Color32::from_rgb(100, 116, 139)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let col = if ok_100 {
                                                    egui::Color32::from_rgb(21, 128, 61)
                                                } else {
                                                    egui::Color32::from_rgb(180, 83, 9)
                                                };
                                                ui.label(
                                                    egui::RichText::new(format!("{sum_wt:.1}%"))
                                                        .size(9.5)
                                                        .strong()
                                                        .color(col),
                                                );
                                            },
                                        );
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());
                            if resp.clicked() {
                                clicked_idx = Some(idx);
                            }
                            ui.add_space(3.0);
                        }
                    });

                if let Some(idx) = clicked_idx {
                    self.selected_idx = idx;
                    self.refresh_selected_auto_calc(thermo_db);
                }
            });
    }

    // ========================================================================
    // Left Pane: Composition & Analyses (Fig. 1)
    // ========================================================================

    fn ui_left_pane(&mut self, ui: &mut egui::Ui, resolved: &ResolvedComposition) -> bool {
        let mut changed = false;

        // Top tabs: Composition | Analyses (exact Fig. 1)
        ui.horizontal(|ui| {
            tab_btn(ui, "Composition", self.left_tab == LeftTab::Composition, || {
                self.left_tab = LeftTab::Composition;
            });
            tab_btn(ui, "Analyses", self.left_tab == LeftTab::Analyses, || {
                self.left_tab = LeftTab::Analyses;
            });
        });

        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(170, 178, 190)))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - 4.0).max(260.0));
                let mat_name = self.mat_db.materials[self.selected_idx].name.clone();

                // Header bar: "Material: Feed Material" (Fig. 1)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Material: {mat_name}"))
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(20, 28, 42)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = (resolved.composition_total_wt - 100.0).abs() < 0.01;
                        let col = if ok {
                            egui::Color32::from_rgb(21, 128, 61)
                        } else {
                            egui::Color32::from_rgb(185, 28, 28)
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "Total: {:.3}%",
                                resolved.composition_total_wt
                            ))
                            .strong()
                            .size(11.0)
                            .color(col),
                        );
                    });
                });

                match self.left_tab {
                    LeftTab::Composition => {
                        if self.ui_composition_sheet(ui, resolved) {
                            changed = true;
                        }
                    }
                    LeftTab::Analyses => {
                        self.ui_analyses_sheet(ui);
                    }
                }
            });

        changed
    }

    fn ui_composition_sheet(&mut self, ui: &mut egui::Ui, resolved: &ResolvedComposition) -> bool {
        let mut changed = false;
        let mat = &mut self.mat_db.materials[self.selected_idx];

        // Formula bar (Fig. 1: A7 | x ✓ fx | ...)
        let (ar, ac) = self.left_active_cell;
        let cref = cell_ref(ar, ac);
        let fx_preview = if ar >= 2 {
            let data_idx = ar - 2;
            match ac {
                0 => mat
                    .composition
                    .get(data_idx)
                    .map(|e| e.item.clone())
                    .unwrap_or_default(),
                1 => mat
                    .composition
                    .get(data_idx)
                    .map(|e| format!("{:.3}", e.wt_pct))
                    .unwrap_or_default(),
                2 => resolved
                    .species
                    .get(data_idx)
                    .map(|(s, _)| s.clone())
                    .unwrap_or_default(),
                3 => resolved
                    .species
                    .get(data_idx)
                    .map(|(_, w)| format!("{w:.3}"))
                    .unwrap_or_default(),
                4 => resolved
                    .elements
                    .get(data_idx)
                    .map(|(e, _)| e.clone())
                    .unwrap_or_default(),
                5 => resolved
                    .elements
                    .get(data_idx)
                    .map(|(_, w)| format!("{w:.3}"))
                    .unwrap_or_default(),
                6 => mat
                    .composition
                    .get(data_idx)
                    .map(|e| e.reference.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            }
        } else {
            String::new()
        };
        draw_formula_bar(ui, &cref, &fx_preview);
        ui.add_space(4.0);

        if !resolved.warnings.is_empty() {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(254, 243, 199))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(217, 119, 6)))
                .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                .show(ui, |ui| {
                    for w in &resolved.warnings {
                        ui.label(
                            egui::RichText::new(format!("⚠ {w}"))
                                .size(10.5)
                                .color(egui::Color32::from_rgb(146, 64, 14)),
                        );
                    }
                });
            ui.add_space(4.0);
        }

        // Exact Fig. 1 colors & columns
        let olive_hdr = egui::Color32::from_rgb(214, 227, 188);
        let readonly_gray = egui::Color32::from_rgb(218, 220, 224);
        let col_w = [112.0, 62.0, 82.0, 66.0, 62.0, 66.0, 96.0];
        let display_rows = mat
            .composition
            .len()
            .max(resolved.species.len())
            .max(resolved.elements.len())
            .max(15)
            + 1;
        let mut remove_idx: Option<usize> = None;

        egui::ScrollArea::both()
            .id_salt("matdb_comp_grid_scroll")
            .max_height((ui.available_height() - 10.0).max(200.0))
            .show(ui, |ui| {
                // Column letters A..G
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    for (c, &w) in col_w.iter().enumerate() {
                        header_cell(ui, &col_letter(c), w);
                    }
                });

                // Row 1: Olive header (Composition | wt-% | Species | wt-% | Elements | wt-% | Reference)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 26.0);
                    let headers = [
                        "Composition",
                        "wt-%",
                        "Species",
                        "wt-%",
                        "Elements",
                        "wt-%",
                        "Reference",
                    ];
                    for (c, h) in headers.iter().enumerate() {
                        widget_cell(ui, col_w[c], olive_hdr, false, |ui| {
                            ui.label(egui::RichText::new(*h).strong().size(11.0));
                        });
                    }
                });

                // Row 2: Total row (Fig. 1)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 2, 26.0);
                    let totals = [
                        "Total".to_string(),
                        format!("{:.3}", resolved.composition_total_wt),
                        String::new(),
                        format!("{:.3}", resolved.species_total_wt),
                        String::new(),
                        format!("{:.3}", resolved.elements_total_wt),
                        String::new(),
                    ];
                    for (c, txt) in totals.iter().enumerate() {
                        widget_cell(ui, col_w[c], olive_hdr, false, |ui| {
                            ui.label(egui::RichText::new(txt).strong().monospace().size(11.0));
                        });
                    }
                });

                // Rows 3..N
                for i in 0..display_rows {
                    let row_num = i + 3;
                    ui.horizontal(|ui| {
                        row_num_cell(ui, row_num, 26.0);

                        // Col A (Composition) & Col B (wt-%)
                        if i < mat.composition.len() {
                            let is_act_a = self.left_active_cell == (i + 2, 0);
                            let is_sub = mat.composition[i].submaterial_name().is_some();
                            let bg_a = if is_sub {
                                egui::Color32::from_rgb(244, 249, 255)
                            } else {
                                egui::Color32::WHITE
                            };
                            widget_cell(ui, col_w[0], bg_a, is_act_a, |ui| {
                                let r = ui.text_edit_singleline(&mut mat.composition[i].item);
                                if r.gained_focus() || r.clicked() {
                                    self.left_active_cell = (i + 2, 0);
                                }
                                if r.changed() {
                                    changed = true;
                                }
                            });

                            let is_act_b = self.left_active_cell == (i + 2, 1);
                            widget_cell(ui, col_w[1], egui::Color32::WHITE, is_act_b, |ui| {
                                let dv = ui.add(
                                    egui::DragValue::new(&mut mat.composition[i].wt_pct)
                                        .speed(0.2)
                                        .range(0.0..=1000.0)
                                        .fixed_decimals(3),
                                );
                                if dv.gained_focus() || dv.clicked() {
                                    self.left_active_cell = (i + 2, 1);
                                }
                                if dv.changed() {
                                    changed = true;
                                }
                            });
                        } else {
                            // Blank click-to-add row
                            let is_act_a = self.left_active_cell == (i + 2, 0);
                            widget_cell(ui, col_w[0], egui::Color32::WHITE, is_act_a, |ui| {
                                let mut tmp = String::new();
                                let r = ui.text_edit_singleline(&mut tmp);
                                if r.gained_focus() || r.clicked() {
                                    self.left_active_cell = (i + 2, 0);
                                }
                                if r.changed() && !tmp.trim().is_empty() {
                                    mat.composition.push(CompositionEntry::new(tmp, 0.0));
                                    changed = true;
                                }
                            });
                            widget_cell(ui, col_w[1], egui::Color32::WHITE, false, |_| {});
                        }

                        // Col C & D: Auto-expanded Species & wt-% (gray readonly block per Fig. 1)
                        let (sp_name, sp_wt) = resolved
                            .species
                            .get(i)
                            .map(|(s, w)| (s.as_str(), format!("{w:.3}")))
                            .unwrap_or(("", String::new()));
                        widget_cell(
                            ui,
                            col_w[2],
                            readonly_gray,
                            self.left_active_cell == (i + 2, 2),
                            |ui| {
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        egui::RichText::new(sp_name).size(11.0),
                                    ))
                                    .clicked()
                                {
                                    self.left_active_cell = (i + 2, 2);
                                }
                            },
                        );
                        widget_cell(
                            ui,
                            col_w[3],
                            readonly_gray,
                            self.left_active_cell == (i + 2, 3),
                            |ui| {
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        egui::RichText::new(sp_wt).monospace().size(11.0),
                                    ))
                                    .clicked()
                                {
                                    self.left_active_cell = (i + 2, 3);
                                }
                            },
                        );

                        // Col E & F: Auto-expanded Elements & wt-% (gray readonly block per Fig. 1)
                        let (el_name, el_wt) = resolved
                            .elements
                            .get(i)
                            .map(|(e, w)| (e.as_str(), format!("{w:.3}")))
                            .unwrap_or(("", String::new()));
                        widget_cell(
                            ui,
                            col_w[4],
                            readonly_gray,
                            self.left_active_cell == (i + 2, 4),
                            |ui| {
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        egui::RichText::new(el_name).size(11.0),
                                    ))
                                    .clicked()
                                {
                                    self.left_active_cell = (i + 2, 4);
                                }
                            },
                        );
                        widget_cell(
                            ui,
                            col_w[5],
                            readonly_gray,
                            self.left_active_cell == (i + 2, 5),
                            |ui| {
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        egui::RichText::new(el_wt).monospace().size(11.0),
                                    ))
                                    .clicked()
                                {
                                    self.left_active_cell = (i + 2, 5);
                                }
                            },
                        );

                        // Col G: Reference + optional row delete button
                        if i < mat.composition.len() {
                            widget_cell(
                                ui,
                                col_w[6],
                                egui::Color32::WHITE,
                                self.left_active_cell == (i + 2, 6),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        let r = ui.add_sized(
                                            [col_w[6] - 26.0, 18.0],
                                            egui::TextEdit::singleline(
                                                &mut mat.composition[i].reference,
                                            ),
                                        );
                                        if r.gained_focus() || r.clicked() {
                                            self.left_active_cell = (i + 2, 6);
                                        }
                                        if ui
                                            .small_button("✖")
                                            .on_hover_text("Remove row")
                                            .clicked()
                                        {
                                            remove_idx = Some(i);
                                        }
                                    });
                                },
                            );
                        } else {
                            widget_cell(ui, col_w[6], egui::Color32::WHITE, false, |_| {});
                        }
                    });
                }
            });

        if let Some(idx) = remove_idx {
            mat.composition.remove(idx);
            changed = true;
        }
        changed
    }

    fn ui_analyses_sheet(&mut self, ui: &mut egui::Ui) {
        let mat = &mut self.mat_db.materials[self.selected_idx];
        ui.horizontal(|ui| {
            if ui.button("➕ Add Analysis Row").clicked() {
                mat.analyses.push(AnalysisEntry {
                    parameter: "New Assay".into(),
                    value: String::new(),
                    unit: "wt-%".into(),
                    method: String::new(),
                    reference: String::new(),
                });
            }
            ui.label(
                egui::RichText::new("Laboratory assays, proximate/ultimate analyses, and moisture/PSD.")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(100, 116, 139)),
            );
        });
        ui.add_space(4.0);

        let olive_hdr = egui::Color32::from_rgb(214, 227, 188);
        let col_w = [145.0, 75.0, 62.0, 130.0, 105.0];
        let mut del_idx: Option<usize> = None;

        egui::ScrollArea::both()
            .id_salt("matdb_analyses_scroll")
            .max_height((ui.available_height() - 8.0).max(200.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    for (c, h) in ["Parameter", "Value", "Unit", "Method", "Reference"]
                        .iter()
                        .enumerate()
                    {
                        widget_cell(ui, col_w[c], olive_hdr, false, |ui| {
                            ui.label(egui::RichText::new(*h).strong().size(11.0));
                        });
                    }
                });

                for (i, a) in mat.analyses.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, i + 1, 26.0);
                        widget_cell(ui, col_w[0], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut a.parameter);
                        });
                        widget_cell(ui, col_w[1], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut a.value);
                        });
                        widget_cell(ui, col_w[2], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut a.unit);
                        });
                        widget_cell(ui, col_w[3], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut a.method);
                        });
                        widget_cell(ui, col_w[4], egui::Color32::WHITE, false, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [col_w[4] - 26.0, 18.0],
                                    egui::TextEdit::singleline(&mut a.reference),
                                );
                                if ui.small_button("✖").clicked() {
                                    del_idx = Some(i);
                                }
                            });
                        });
                    });
                }
            });

        if let Some(i) = del_idx {
            mat.analyses.remove(i);
        }
    }

    // ========================================================================
    // Right Pane: Common (Fig. 2) | Cp Data (Fig. 3 L) | Cp Chart (Fig. 3 R)
    // ========================================================================

    fn ui_right_pane(&mut self, ui: &mut egui::Ui, thermo_db: &ThermoDatabase) -> bool {
        let mut changed = false;

        // Top tabs: Common | Cp Data | Cp Chart (exact Figs. 2 & 3)
        ui.horizontal(|ui| {
            tab_btn(ui, "Common", self.right_top_tab == RightTopTab::Common, || {
                self.right_top_tab = RightTopTab::Common;
            });
            tab_btn(ui, "Cp Data", self.right_top_tab == RightTopTab::CpData, || {
                self.right_top_tab = RightTopTab::CpData;
            });
            tab_btn(ui, "Cp Chart", self.right_top_tab == RightTopTab::CpChart, || {
                self.right_top_tab = RightTopTab::CpChart;
            });
        });

        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(170, 178, 190)))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.set_min_height((ui.available_height() - 4.0).max(260.0));
                match self.right_top_tab {
                    RightTopTab::Common => {
                        if self.ui_common_tab(ui, thermo_db) {
                            changed = true;
                        }
                    }
                    RightTopTab::CpData => {
                        self.ui_cp_data_tab(ui);
                    }
                    RightTopTab::CpChart => {
                        self.ui_cp_chart_tab(ui);
                    }
                }
            });

        changed
    }

    fn ui_common_tab(&mut self, ui: &mut egui::Ui, _thermo_db: &ThermoDatabase) -> bool {
        let mut changed = false;

        let body_h = (ui.available_height() - 30.0).max(180.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), body_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| match self.common_sub_tab {
                CommonSubTab::GeneralInfo => {
                    self.ui_general_info_subtab(ui);
                }
                CommonSubTab::Properties => {
                    if self.ui_properties_sheet(ui) {
                        changed = true;
                    }
                }
                CommonSubTab::Data => {
                    self.ui_data_subtab(ui);
                }
                CommonSubTab::UserProperties => {
                    self.ui_user_properties_subtab(ui);
                }
            },
        );

        // Bottom workbook sheet tabs matching Fig. 2 exactly:
        // General Info | Properties | Data | User Properties
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⏮ ◀ ▶ ⏭")
                    .size(9.5)
                    .color(egui::Color32::from_rgb(120, 130, 145)),
            );
            ui.add_space(4.0);
            for (sub, label) in [
                (CommonSubTab::GeneralInfo, "General Info"),
                (CommonSubTab::Properties, "Properties"),
                (CommonSubTab::Data, "Data"),
                (CommonSubTab::UserProperties, "User Properties"),
            ] {
                let active = self.common_sub_tab == sub;
                let btn = egui::Button::new(
                    egui::RichText::new(label)
                        .size(11.0)
                        .strong()
                        .color(if active {
                            egui::Color32::from_rgb(15, 23, 42)
                        } else {
                            egui::Color32::from_rgb(90, 100, 115)
                        }),
                )
                .fill(if active {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(232, 236, 242)
                });
                if ui.add(btn).clicked() {
                    self.common_sub_tab = sub;
                }
            }
        });

        changed
    }

    fn ui_general_info_subtab(&mut self, ui: &mut egui::Ui) {
        let cur_name = self.mat_db.materials[self.selected_idx].name.clone();
        let used_in: Vec<String> = self
            .mat_db
            .materials
            .iter()
            .filter(|m| {
                m.name != cur_name
                    && m.composition.iter().any(|e| {
                        e.submaterial_name()
                            .map(|s| s.eq_ignore_ascii_case(&cur_name))
                            .unwrap_or(false)
                    })
            })
            .map(|m| m.name.clone())
            .collect();

        let mat = &mut self.mat_db.materials[self.selected_idx];
        egui::ScrollArea::vertical()
            .id_salt("matdb_geninfo_scroll")
            .show(ui, |ui| {
                egui::Grid::new("matdb_geninfo_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.strong("Material Name:");
                        ui.add_sized([280.0, 22.0], egui::TextEdit::singleline(&mut mat.name));
                        ui.end_row();

                        ui.strong("Material Code:");
                        ui.add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut mat.code));
                        ui.end_row();

                        ui.strong("Category:");
                        egui::ComboBox::from_id_salt("matdb_mat_cat")
                            .width(180.0)
                            .selected_text(mat.category.label())
                            .show_ui(ui, |ui| {
                                for &c in MaterialCategory::ALL {
                                    ui.selectable_value(&mut mat.category, c, c.label());
                                }
                            });
                        ui.end_row();

                        ui.strong("Supplier / Origin:");
                        ui.add_sized([280.0, 22.0], egui::TextEdit::singleline(&mut mat.supplier));
                        ui.end_row();

                        ui.strong("Last Updated:");
                        ui.add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut mat.updated_at));
                        ui.end_row();

                        ui.strong("Description:");
                        ui.add_sized(
                            [300.0, 68.0],
                            egui::TextEdit::multiline(&mut mat.description),
                        );
                        ui.end_row();

                        ui.strong("Referenced By:");
                        if used_in.is_empty() {
                            ui.label("Not referenced by other materials.");
                        } else {
                            ui.label(used_in.join(", "));
                        }
                        ui.end_row();
                    });
            });
    }

    fn ui_properties_sheet(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let mat = &mut self.mat_db.materials[self.selected_idx];

        let (ar, ac) = self.prop_active_cell;
        draw_formula_bar(ui, &cell_ref(ar, ac), "");
        ui.add_space(4.0);

        let section_bg = egui::Color32::from_rgb(252, 249, 212);
        let label_bg = egui::Color32::from_rgb(220, 222, 226);
        let auto_bg = egui::Color32::from_rgb(214, 226, 244);
        let col_w = [18.0, 148.0, 86.0, 64.0, 96.0, 86.0];

        egui::ScrollArea::both()
            .id_salt("matdb_props_grid_scroll")
            .show(ui, |ui| {
                // Column letters A..F
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    for (c, &w) in col_w.iter().enumerate() {
                        header_cell(ui, &col_letter(c), w);
                    }
                });

                // Section 1: Heat & State (Row 1)
                prop_section_row(ui, 1, "Heat & State", &col_w, section_bg);
                let p = &mut mat.properties;
                if prop_data_row(ui, 2, "State at 25 °C", &mut p.state_at_25c, "Text", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 3, "Average molar weight", &mut p.avg_molar_weight, "g/mol", true, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 4, "Density", &mut p.density, "kg/l", true, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 5, "Melting Point", &mut p.melting_point, "K", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 6, "Boiling Point", &mut p.boiling_point, "K", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 7, "Heat Capacity", &mut p.heat_capacity, "J/(kg*K)", true, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 8, "Enthalpy at 25 °C", &mut p.enthalpy_25c, "kJ/kg", true, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 9, "Heat of Fusion", &mut p.heat_of_fusion, "kJ/kg", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 10, "Heat of Vaporization", &mut p.heat_of_vaporization, "kJ/kg", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }

                // Section 2: Electrical & Magnetic (Row 11)
                prop_section_row(ui, 11, "Electrical & Magnetic", &col_w, section_bg);
                if prop_data_row(ui, 12, "Electrical Conductivity", &mut p.electrical_conductivity, "S/m", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 13, "Mass Susceptibility", &mut p.mass_susceptibility, "m3/kg", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 14, "Magnetic permeability", &mut p.magnetic_permeability, "H/m", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }

                // Section 3: Mechanical (Row 15)
                prop_section_row(ui, 15, "Mechanical", &col_w, section_bg);
                if prop_data_row(ui, 16, "Hardness (HV)", &mut p.hardness_hv, "HV", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 17, "Hardness (Mohs)", &mut p.hardness_mohs, "Mohs", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
                if prop_data_row(ui, 18, "Young's Modulus", &mut p.youngs_modulus, "MPa", false, &col_w, label_bg, auto_bg, &mut self.prop_active_cell) {
                    changed = true;
                }
            });

        changed
    }

    fn ui_data_subtab(&mut self, ui: &mut egui::Ui) {
        let mat = &mut self.mat_db.materials[self.selected_idx];
        ui.horizontal(|ui| {
            if ui.button("➕ Add Data Row").clicked() {
                mat.data_rows.push(MaterialDataRow {
                    parameter: "Measurement".into(),
                    temperature_k: "298.15".into(),
                    value: String::new(),
                    unit: String::new(),
                    reference: String::new(),
                });
            }
        });
        ui.add_space(4.0);
        let hdr_bg = egui::Color32::from_rgb(252, 249, 212);
        let col_w = [125.0, 68.0, 75.0, 65.0, 120.0];
        let mut del_idx = None;
        egui::ScrollArea::both()
            .id_salt("matdb_data_subtab_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    for (c, h) in ["Parameter", "T (K)", "Value", "Unit", "Reference"]
                        .iter()
                        .enumerate()
                    {
                        widget_cell(ui, col_w[c], hdr_bg, false, |ui| {
                            ui.label(egui::RichText::new(*h).strong().size(11.0));
                        });
                    }
                });
                for (i, d) in mat.data_rows.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, i + 1, 26.0);
                        widget_cell(ui, col_w[0], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut d.parameter);
                        });
                        widget_cell(ui, col_w[1], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut d.temperature_k);
                        });
                        widget_cell(ui, col_w[2], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut d.value);
                        });
                        widget_cell(ui, col_w[3], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut d.unit);
                        });
                        widget_cell(ui, col_w[4], egui::Color32::WHITE, false, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [col_w[4] - 26.0, 18.0],
                                    egui::TextEdit::singleline(&mut d.reference),
                                );
                                if ui.small_button("✖").clicked() {
                                    del_idx = Some(i);
                                }
                            });
                        });
                    });
                }
            });
        if let Some(i) = del_idx {
            mat.data_rows.remove(i);
        }
    }

    fn ui_user_properties_subtab(&mut self, ui: &mut egui::Ui) {
        let mat = &mut self.mat_db.materials[self.selected_idx];
        ui.horizontal(|ui| {
            if ui.button("➕ Add User Property").clicked() {
                mat.user_properties.push(UserProperty {
                    name: "Custom Property".into(),
                    value: String::new(),
                    unit: String::new(),
                    reference: String::new(),
                });
            }
            ui.label(
                egui::RichText::new("Create custom material properties for HSC Sim models (§90).")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(100, 116, 139)),
            );
        });
        ui.add_space(4.0);
        let hdr_bg = egui::Color32::from_rgb(252, 249, 212);
        let col_w = [155.0, 88.0, 72.0, 145.0];
        let mut del_idx = None;
        egui::ScrollArea::both()
            .id_salt("matdb_user_props_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    for (c, h) in ["Property Name", "Value", "Unit", "Reference"]
                        .iter()
                        .enumerate()
                    {
                        widget_cell(ui, col_w[c], hdr_bg, false, |ui| {
                            ui.label(egui::RichText::new(*h).strong().size(11.0));
                        });
                    }
                });
                for (i, up) in mat.user_properties.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, i + 1, 26.0);
                        widget_cell(ui, col_w[0], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut up.name);
                        });
                        widget_cell(ui, col_w[1], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut up.value);
                        });
                        widget_cell(ui, col_w[2], egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut up.unit);
                        });
                        widget_cell(ui, col_w[3], egui::Color32::WHITE, false, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [col_w[3] - 26.0, 18.0],
                                    egui::TextEdit::singleline(&mut up.reference),
                                );
                                if ui.small_button("✖").clicked() {
                                    del_idx = Some(i);
                                }
                            });
                        });
                    });
                }
            });
        if let Some(i) = del_idx {
            mat.user_properties.remove(i);
        }
    }

    fn ui_cp_data_tab(&mut self, ui: &mut egui::Ui) {
        // Exact Fig. 3 formula header
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(248, 250, 253))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    egui::RichText::new(
                        "Cp(T) = A + B·T·10⁻³ + C·T⁻²·10⁵ + D·T²·10⁻⁶ + E·T⁻³·10⁸ + F·T³·10⁻⁹",
                    )
                    .italics()
                    .strong()
                    .size(12.5)
                    .color(egui::Color32::from_rgb(20, 30, 50)),
                );
            });
        ui.add_space(4.0);

        let (ar, ac) = self.cp_active_cell;
        draw_formula_bar(ui, &cell_ref(ar, ac), "");
        ui.add_space(4.0);

        let mat = &mut self.mat_db.materials[self.selected_idx];
        let num_cols = mat.cp_ranges.len().max(5);
        let label_bg = egui::Color32::from_rgb(220, 222, 226);
        let label_w = 152.0;
        let data_w = 78.0;

        ui.horizontal(|ui| {
            if ui.button("➕ Add Range").clicked() {
                let next_t = mat.cp_ranges.last().map(|r| r.t_max_k).unwrap_or(298.15);
                mat.cp_ranges.push(MaterialCpRange {
                    t_min_k: next_t,
                    t_max_k: next_t + 250.0,
                    phase: "s".into(),
                    ..Default::default()
                });
            }
            if mat.cp_ranges.len() > 1 && ui.button("➖ Remove Last Range").clicked() {
                mat.cp_ranges.pop();
            }
            ui.separator();
            ui.label("Eval T (K):");
            ui.add_sized([58.0, 20.0], egui::TextEdit::singleline(&mut self.eval_temp_str));
            if let Ok(t_k) = self.eval_temp_str.trim().replace(',', ".").parse::<f64>() {
                if let Some((cp, h)) = eval_material_cp_and_h(mat, t_k) {
                    ui.label(
                        egui::RichText::new(format!("Cp = {cp:.3} J/(kg·K)   |   H = {h:.3} kJ/kg"))
                            .strong()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(29, 78, 216)),
                    );
                }
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::both()
            .id_salt("matdb_cp_grid_scroll")
            .show(ui, |ui| {
                // Header A, B, C, ...
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    header_cell(ui, "A", label_w);
                    for c in 0..num_cols {
                        header_cell(ui, &col_letter(c + 1), data_w);
                    }
                });

                // Row 1: Range 1..N
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 26.0);
                    widget_cell(ui, label_w, label_bg, false, |ui| {
                        ui.label(egui::RichText::new("Range").strong().size(11.0));
                    });
                    for c in 0..num_cols {
                        widget_cell(ui, data_w, label_bg, false, |ui| {
                            ui.label(egui::RichText::new(format!("{}", c + 1)).strong().size(11.0));
                        });
                    }
                });

                let row_specs: [(usize, &str); 11] = [
                    (2, "Tmin (K)"),
                    (3, "Tmax (K)"),
                    (4, "Phase"),
                    (5, "H kJ/kg"),
                    (6, "Cp coefficient A J/(kg*K)"),
                    (7, "Cp coefficient B"),
                    (8, "Cp coefficient C"),
                    (9, "Cp coefficient D"),
                    (10, "Cp coefficient E"),
                    (11, "Cp coefficient F"),
                    (12, "Reference"),
                ];

                for &(rnum, rlabel) in &row_specs {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, rnum, 26.0);
                        widget_cell(ui, label_w, label_bg, false, |ui| {
                            ui.label(egui::RichText::new(rlabel).strong().size(10.5));
                        });
                        for c in 0..num_cols {
                            let is_act = self.cp_active_cell == (rnum - 1, c + 1);
                            if c < mat.cp_ranges.len() {
                                let r = &mut mat.cp_ranges[c];
                                widget_cell(ui, data_w, egui::Color32::WHITE, is_act, |ui| {
                                    let resp = match rnum {
                                        2 => ui.add(
                                            egui::DragValue::new(&mut r.t_min_k)
                                                .speed(1.0)
                                                .fixed_decimals(3),
                                        ),
                                        3 => ui.add(
                                            egui::DragValue::new(&mut r.t_max_k)
                                                .speed(1.0)
                                                .fixed_decimals(3),
                                        ),
                                        4 => ui.text_edit_singleline(&mut r.phase),
                                        5 => ui.add(
                                            egui::DragValue::new(&mut r.h_kj_kg)
                                                .speed(1.0)
                                                .fixed_decimals(3),
                                        ),
                                        6 => ui.add(
                                            egui::DragValue::new(&mut r.a)
                                                .speed(1.0)
                                                .fixed_decimals(3),
                                        ),
                                        7 => ui.add(
                                            egui::DragValue::new(&mut r.b)
                                                .speed(0.5)
                                                .fixed_decimals(3),
                                        ),
                                        8 => ui.add(
                                            egui::DragValue::new(&mut r.c)
                                                .speed(0.5)
                                                .fixed_decimals(3),
                                        ),
                                        9 => ui.add(
                                            egui::DragValue::new(&mut r.d)
                                                .speed(0.5)
                                                .fixed_decimals(3),
                                        ),
                                        10 => ui.add(
                                            egui::DragValue::new(&mut r.e)
                                                .speed(0.5)
                                                .fixed_decimals(3),
                                        ),
                                        11 => ui.add(
                                            egui::DragValue::new(&mut r.f)
                                                .speed(0.5)
                                                .fixed_decimals(3),
                                        ),
                                        _ => ui.text_edit_singleline(&mut r.reference),
                                    };
                                    if resp.gained_focus() || resp.clicked() {
                                        self.cp_active_cell = (rnum - 1, c + 1);
                                    }
                                });
                            } else {
                                widget_cell(ui, data_w, egui::Color32::WHITE, false, |_| {});
                            }
                        }
                    });
                }
            });
    }

    fn ui_cp_chart_tab(&mut self, ui: &mut egui::Ui) {
        let mat = &self.mat_db.materials[self.selected_idx];
        let pts = sample_cp_chart_points(mat, 40);

        let avail = ui.available_size();
        let w = avail.x.max(260.0);
        let h = avail.y.max(240.0);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
        let p = ui.painter_at(rect);

        p.rect_filled(rect, 0.0, egui::Color32::WHITE);

        let plot = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 62.0, rect.top() + 18.0),
            egui::pos2(rect.right() - 18.0, rect.bottom() - 40.0),
        );
        if plot.width() < 40.0 || plot.height() < 40.0 {
            return;
        }

        p.rect_filled(plot, 0.0, egui::Color32::WHITE);
        p.rect_stroke(
            plot,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 76, 86)),
        );

        if pts.len() < 2 {
            p.text(
                plot.center(),
                egui::Align2::CENTER_CENTER,
                "Add at least one valid Cp Data range (Tmin < Tmax) to plot Cp(T).",
                egui::FontId::proportional(11.5),
                egui::Color32::GRAY,
            );
            return;
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(t, cp) in &pts {
            if t.is_finite() && cp.is_finite() {
                x_min = x_min.min(t);
                x_max = x_max.max(t);
                y_min = y_min.min(cp);
                y_max = y_max.max(cp);
            }
        }
        let x_span = (x_max - x_min).max(1.0);
        let y_span = (y_max - y_min).abs();
        let y_pad = if y_span < 1e-6 {
            (y_min.abs() * 0.1).max(5.0)
        } else {
            y_span * 0.15
        };
        let x0 = x_min;
        let x1 = x_max;
        let y0 = y_min - y_pad;
        let y1 = y_max + y_pad;

        let x_step = nice_step(x_span / 6.0);
        let y_step = nice_step((y1 - y0) / 6.0);

        let to_x = |t: f64| plot.left() + (((t - x0) / (x1 - x0)) as f32) * plot.width();
        let to_y = |cp: f64| plot.bottom() - (((cp - y0) / (y1 - y0)) as f32) * plot.height();

        // Horizontal gridlines & Y ticks
        let mut yv = (y0 / y_step).ceil() * y_step;
        let mut g = 0;
        while yv <= y1 + y_step * 0.01 && g < 30 {
            let py = to_y(yv);
            if py >= plot.top() - 1.0 && py <= plot.bottom() + 1.0 {
                p.line_segment(
                    [egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)],
                    egui::Stroke::new(0.7, egui::Color32::from_rgb(230, 234, 240)),
                );
                p.line_segment(
                    [egui::pos2(plot.left() - 4.0, py), egui::pos2(plot.left(), py)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                p.text(
                    egui::pos2(plot.left() - 7.0, py),
                    egui::Align2::RIGHT_CENTER,
                    format_tick(yv),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(40, 48, 60),
                );
            }
            yv += y_step;
            g += 1;
        }

        // Vertical gridlines & X ticks
        let mut xv = (x0 / x_step).ceil() * x_step;
        let mut xg = 0;
        while xv <= x1 + x_step * 0.01 && xg < 30 {
            let px = to_x(xv);
            if px >= plot.left() - 1.0 && px <= plot.right() + 1.0 {
                p.line_segment(
                    [egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())],
                    egui::Stroke::new(0.7, egui::Color32::from_rgb(230, 234, 240)),
                );
                p.line_segment(
                    [egui::pos2(px, plot.bottom()), egui::pos2(px, plot.bottom() + 4.0)],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );
                p.text(
                    egui::pos2(px, plot.bottom() + 7.0),
                    egui::Align2::CENTER_TOP,
                    format_tick(xv),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(40, 48, 60),
                );
            }
            xv += x_step;
            xg += 1;
        }

        // Axis titles matching Fig. 3 right exactly
        p.text(
            egui::pos2(plot.center().x, rect.bottom() - 11.0),
            egui::Align2::CENTER_CENTER,
            "Temperature (K)",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(30, 38, 50),
        );
        draw_rotated_y_title(
            &p,
            rect.left() + 14.0,
            plot.center().y,
            "Heat Capacity (J/(kg*K))",
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(30, 38, 50),
        );

        // Crisp black Cp(T) line (Fig. 3 right)
        let screen_pts: Vec<egui::Pos2> = pts
            .iter()
            .filter(|(t, cp)| t.is_finite() && cp.is_finite())
            .map(|&(t, cp)| egui::pos2(to_x(t), to_y(cp)))
            .collect();
        if screen_pts.len() >= 2 {
            p.add(egui::Shape::line(
                screen_pts,
                egui::Stroke::new(1.5, egui::Color32::BLACK),
            ));
        }

        // Interactive hover crosshair & live (T, Cp, H) readout
        if let Some(hp) = resp.hover_pos() {
            if plot.contains(hp) {
                let frac = ((hp.x - plot.left()) / plot.width()).clamp(0.0, 1.0) as f64;
                let t_hover = x0 + frac * (x1 - x0);
                if let Some((cp_h, h_h)) = eval_material_cp_and_h(mat, t_hover) {
                    let cx = to_x(t_hover);
                    let cy = to_y(cp_h).clamp(plot.top(), plot.bottom());
                    p.line_segment(
                        [egui::pos2(cx, plot.top()), egui::pos2(cx, plot.bottom())],
                        egui::Stroke::new(0.9, egui::Color32::from_rgb(148, 163, 184)),
                    );
                    p.circle_filled(
                        egui::pos2(cx, cy),
                        3.5,
                        egui::Color32::from_rgb(37, 99, 235),
                    );
                    let txt = format!(
                        "T = {t_hover:.1} K\nCp = {cp_h:.2} J/(kg·K)\nH = {h_h:.2} kJ/kg"
                    );
                    let box_rect = egui::Rect::from_min_size(
                        egui::pos2(plot.right() - 160.0, plot.top() + 10.0),
                        egui::vec2(150.0, 52.0),
                    );
                    p.rect_filled(
                        box_rect,
                        4.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 240),
                    );
                    p.rect_stroke(
                        box_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 130, 145)),
                    );
                    p.text(
                        box_rect.left_top() + egui::vec2(8.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        txt,
                        egui::FontId::monospace(10.0),
                        egui::Color32::from_rgb(20, 28, 42),
                    );
                }
            }
        }
    }

    // ========================================================================
    // Modals: Species Picker & Open/Save/Export
    // ========================================================================

    fn ui_species_picker_modal(&mut self, ctx: &egui::Context, thermo_db: &ThermoDatabase) {
        if !self.show_species_picker {
            return;
        }
        let mut open = self.show_species_picker;
        let mut chosen: Option<String> = None;

        egui::Window::new("Insert Species from Thermodynamic Database")
            .open(&mut open)
            .collapsible(false)
            .default_size([440.0, 360.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search formula/name:");
                    ui.add_sized(
                        [240.0, 22.0],
                        egui::TextEdit::singleline(&mut self.species_query)
                            .hint_text("e.g. CuFeS2, Fe2O3, SiO2, CaCO3..."),
                    );
                });
                ui.separator();

                let results = thermo_db.search_species(self.species_query.trim(), 80);
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        if results.is_empty() {
                            ui.label("No matching species found.");
                        }
                        for sp in results {
                            ui.horizontal(|ui| {
                                if ui.button("➕ Add").clicked() {
                                    chosen = Some(sp.formula.clone());
                                }
                                ui.label(egui::RichText::new(&sp.formula).strong().monospace());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "MW={:.2} g/mol, ρ={:.2} kg/l  {}",
                                        sp.molecular_weight, sp.density_g_cm3, sp.name
                                    ))
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(75, 85, 99)),
                                );
                            });
                        }
                    });
            });

        if let Some(formula) = chosen {
            self.mat_db.materials[self.selected_idx]
                .composition
                .push(CompositionEntry::new(formula.clone(), 5.0));
            self.left_tab = LeftTab::Composition;
            self.refresh_selected_auto_calc(thermo_db);
            self.status_err = None;
            self.status_msg = Some(format!("Added species '{formula}' (5 wt-%) to composition."));
            self.show_species_picker = false;
        } else {
            self.show_species_picker = open;
        }
    }

    fn ui_file_modal(&mut self, ctx: &egui::Context, thermo_db: &ThermoDatabase) {
        if self.file_modal == FileModalMode::None {
            return;
        }
        let mut open = true;
        let (title, action_lbl) = match self.file_modal {
            FileModalMode::OpenJson => ("Open Material Database (.json)", "Open"),
            FileModalMode::SaveJson => ("Save Material Database (.json)", "Save"),
            FileModalMode::ExportCsv => ("Export Active Material (.csv)", "Export"),
            FileModalMode::None => return,
        };
        let mut run = false;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("File path:");
                ui.add_sized(
                    [360.0, 22.0],
                    egui::TextEdit::singleline(&mut self.file_path_str),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(action_lbl).clicked() {
                        run = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.file_modal = FileModalMode::None;
                    }
                });
            });
        if run {
            let path = self.file_path_str.clone();
            match self.file_modal {
                FileModalMode::OpenJson => self.open_json(&path, thermo_db),
                FileModalMode::SaveJson => self.save_json(&path),
                FileModalMode::ExportCsv => self.export_csv(&path),
                FileModalMode::None => {}
            }
            self.file_modal = FileModalMode::None;
        } else if !open {
            self.file_modal = FileModalMode::None;
        }
    }
}

// ============================================================================
// UI Helper Widgets
// ============================================================================

fn ribbon_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 175))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(198, 206, 220)))
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(5.0, 3.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    add_contents(ui);
                });
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .size(9.5)
                            .color(egui::Color32::from_rgb(100, 112, 130)),
                    );
                });
            });
        });
}

fn ribbon_btn(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(format!("{icon}\n{label}"))
            .size(10.0)
            .color(egui::Color32::from_rgb(25, 35, 50)),
    )
    .min_size(egui::vec2(52.0, 42.0))
    .fill(if active {
        egui::Color32::from_rgb(191, 219, 254)
    } else {
        egui::Color32::from_rgb(248, 250, 253)
    });
    ui.add(btn).clicked()
}

fn tab_btn(ui: &mut egui::Ui, label: &str, active: bool, on_click: impl FnOnce()) {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(11.5)
            .strong()
            .color(if active {
                egui::Color32::from_rgb(15, 23, 42)
            } else {
                egui::Color32::from_rgb(85, 95, 110)
            }),
    )
    .fill(if active {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(226, 231, 240)
    });
    if ui.add(btn).clicked() {
        on_click();
    }
}

fn draw_formula_bar(ui: &mut egui::Ui, cref: &str, preview: &str) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(246, 248, 251))
        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
        .inner_margin(egui::Margin::symmetric(5.0, 2.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [44.0, 18.0],
                    egui::Label::new(egui::RichText::new(cref).strong().monospace().size(11.0)),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new("✖  ✓  fx")
                        .size(10.5)
                        .color(egui::Color32::from_rgb(100, 110, 125)),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(preview)
                        .monospace()
                        .size(11.0)
                        .color(egui::Color32::from_rgb(30, 41, 59)),
                );
            });
        });
}

fn prop_section_row(
    ui: &mut egui::Ui,
    row_num: usize,
    section_title: &str,
    col_w: &[f32; 6],
    bg: egui::Color32,
) {
    ui.horizontal(|ui| {
        row_num_cell(ui, row_num, 26.0);
        widget_cell(ui, col_w[0] + col_w[1], bg, false, |ui| {
            ui.label(egui::RichText::new(section_title).strong().size(11.0));
        });
        for (idx, lbl) in ["Value", "Unit", "Reference", "Auto Calculate"]
            .iter()
            .enumerate()
        {
            widget_cell(ui, col_w[idx + 2], bg, false, |ui| {
                ui.label(egui::RichText::new(*lbl).strong().size(11.0));
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn prop_data_row(
    ui: &mut egui::Ui,
    row_num: usize,
    name: &str,
    field: &mut PropertyField,
    unit: &str,
    supports_auto: bool,
    col_w: &[f32; 6],
    label_bg: egui::Color32,
    auto_bg: egui::Color32,
    active_cell: &mut (usize, usize),
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        row_num_cell(ui, row_num, 26.0);
        widget_cell(ui, col_w[0], label_bg, false, |_| {});
        widget_cell(ui, col_w[1], label_bg, false, |ui| {
            ui.label(egui::RichText::new(name).size(11.0));
        });

        let val_bg = if supports_auto && field.auto_calculate {
            auto_bg
        } else {
            egui::Color32::WHITE
        };
        let is_act_val = *active_cell == (row_num - 1, 2);
        widget_cell(ui, col_w[2], val_bg, is_act_val, |ui| {
            if supports_auto && field.auto_calculate {
                let r = ui.add_sized(
                    [col_w[2] - 8.0, 18.0],
                    egui::SelectableLabel::new(
                        false,
                        egui::RichText::new(&field.value)
                            .monospace()
                            .size(11.0)
                            .color(egui::Color32::from_rgb(15, 23, 42)),
                    ),
                );
                if r.clicked() {
                    *active_cell = (row_num - 1, 2);
                }
                r.on_hover_text("Auto-calculated from Composition (click TRUE in column F to edit manually)");
            } else {
                let r = ui.text_edit_singleline(&mut field.value);
                if r.gained_focus() || r.clicked() {
                    *active_cell = (row_num - 1, 2);
                }
            }
        });

        widget_cell(ui, col_w[3], egui::Color32::WHITE, false, |ui| {
            ui.label(egui::RichText::new(unit).size(11.0));
        });

        let is_act_ref = *active_cell == (row_num - 1, 4);
        widget_cell(ui, col_w[4], egui::Color32::WHITE, is_act_ref, |ui| {
            let r = ui.text_edit_singleline(&mut field.reference);
            if r.gained_focus() || r.clicked() {
                *active_cell = (row_num - 1, 4);
            }
        });

        widget_cell(ui, col_w[5], egui::Color32::WHITE, false, |ui| {
            if supports_auto {
                let lbl = if field.auto_calculate { "TRUE" } else { "" };
                let r = ui.add_sized(
                    [col_w[5] - 8.0, 18.0],
                    egui::SelectableLabel::new(
                        field.auto_calculate,
                        egui::RichText::new(lbl).monospace().size(11.0),
                    ),
                );
                if r.clicked() {
                    field.auto_calculate = !field.auto_calculate;
                    changed = true;
                }
                r.on_hover_text("Click to toggle Auto Calculate (TRUE / manual)");
            }
        });
    });
    changed
}

fn col_letter(idx: usize) -> String {
    let mut n = idx + 1;
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

fn cell_ref(row: usize, col: usize) -> String {
    format!("{}{}", col_letter(col), row + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_db_ui_auto_calc_normalize_and_json_roundtrip() {
        let thermo = ThermoDatabase::with_open_defaults();
        let mut st = MaterialDbModuleState::default();
        st.refresh_all_auto_calc(&thermo);

        // Feed Material auto-calculated properties must be populated
        let feed = &st.mat_db.materials[0];
        assert_eq!(feed.name, "Feed Material");
        assert!(!feed.properties.avg_molar_weight.value.is_empty());
        assert!(!feed.properties.density.value.is_empty());
        assert!(!feed.properties.enthalpy_25c.value.is_empty());

        // Create, normalize, sync assays, and JSON roundtrip
        st.create_new_material(&thermo);
        st.mat_db.materials[st.selected_idx]
            .composition
            .push(CompositionEntry::new("[Steel]", 50.0));
        st.normalize_selected(&thermo);
        let res = st
            .mat_db
            .resolve_composition(&st.mat_db.materials[st.selected_idx]);
        assert!((res.composition_total_wt - 100.0).abs() < 1e-3);

        st.sync_elements_to_analyses();
        assert!(!st.mat_db.materials[st.selected_idx].analyses.is_empty());

        let tmp = format!("/tmp/test_matdb_{}.json", std::process::id());
        st.save_json(&tmp);
        assert!(st.status_err.is_none());
        let expected_n = st.mat_db.materials.len();
        st.open_json(&tmp, &thermo);
        let _ = std::fs::remove_file(&tmp);
        assert!(st.status_err.is_none());
        assert_eq!(st.mat_db.materials.len(), expected_n);
    }
}
