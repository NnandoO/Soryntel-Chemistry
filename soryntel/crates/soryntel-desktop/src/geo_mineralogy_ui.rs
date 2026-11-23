//! Module 24 (Chapter 84) — HSC Geo: Mineralogical Calculations and Mineral Database UI.
//!
//! Renders all 6 Chapter 84 tools inside the unified HSC Geo window (`GeoLiberationModuleState`):
//! 1. `Database Viewer` (§84.2) — 5 sources, Filter builder, multi-keyword Find, View/Edit mode,
//!    Add/Remove records, Copy/Paste TSV with column identification report.
//! 2. `Find by Properties` (§84.3) — 3-state Periodic Table (Yellow MUST / Red MUST NOT / Neutral),
//!    HSC filter expression, 3-pane Results + General Properties + Elemental Composition Pie Chart.
//! 3. `Catalogue` (§84.4) — General Properties, List of Minerals + Filter, Variability Table,
//!    Variability strip chart, 2D/3D Scatter plot, and Elemental Composition Pie Chart.
//! 4. `Find by Composition` (§84.5) — WSSQ SEM/EDS grain identification, Add New Value dialog,
//!    Periodic Table selector, Oxide/Element toggle, ranked WSSQ match table.
//! 5. `Petrological` (§84.6) — Bruvann 7-sheet dataset, 12-row `Identify`, color-coded Periodic Table
//!    Method Priority (`Top/Up/Down/Bottom`), all 13 calculation routines, Spidergram, Excel `.xlsx`.
//! 6. `Modal` (§84.7) — Multi-round `LS/WLS/NNLS/WNNLS` + Remainder (`Sum = 100%`), 7 Result tabs,
//!    and Monte Carlo `Error Estimation` (`Regenerate CV-tables`, `Average ± 1 Sigma`, `Average`,
//!    `StdDev`, `Min-Max`, `CV`, `.xlsx` export).

use eframe::egui;
use soryntel_thermo::elements::{ElementCategory, PERIODIC_ELEMENTS};
use soryntel_thermo::geo_liberation::{GeoMineralCatalogEntry, MineralGroupDef};
use soryntel_thermo::geo_mineralogy::{
    default_eds_criteria, filter_minerals_by_properties, find_minerals_by_composition_wssq,
    format_filter_expression, CompCriterion, CompUnit, CompositionMatch, ElementFilterState,
    FilterCondition, FilterLogic, FilterOp, GeoDatabaseStore, GeoDbSource, GeoMineralRecord,
    MineralChemRecord, ModalMineralDef, ModalProject, ModalRound, ModalSolveMethod, PasteReport,
    PetroGroup, PetroRoutine, PetrologicalProject, PropertySearchFilter, COMMON_OXIDES,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::excel_grid::{header_cell, row_num_cell, widget_cell, EXCEL_GRID_BORDER};
use crate::ui_common::{draw_rotated_y_title, format_tick, nice_step};

const CHART_PALETTE: [egui::Color32; 12] = [
    egui::Color32::from_rgb(76, 114, 176),
    egui::Color32::from_rgb(152, 78, 163),
    egui::Color32::from_rgb(85, 168, 104),
    egui::Color32::from_rgb(221, 132, 82),
    egui::Color32::from_rgb(196, 78, 82),
    egui::Color32::from_rgb(147, 120, 96),
    egui::Color32::from_rgb(218, 139, 195),
    egui::Color32::from_rgb(100, 181, 205),
    egui::Color32::from_rgb(204, 185, 116),
    egui::Color32::from_rgb(129, 114, 179),
    egui::Color32::from_rgb(56, 142, 60),
    egui::Color32::from_rgb(245, 124, 0),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PetroViewTab {
    Parameters,
    Settings,
    Result,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalTopTab {
    Calculation,
    ErrorEstimation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalCalcTab {
    InputData,
    Rules,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalResultSubTab {
    Modal,
    Distribution,
    Fraction,
    Residual,
    BulkChem,
    NotesStats,
    InitialValues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McResultSubTab {
    AvgSigma,
    Average,
    StdDev,
    MinMax,
    Cv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Geo84ExportKind {
    None,
    PetroXlsx,
    ModalXlsx,
}

/// Action returned from Chapter 84 views to `GeoLiberationModuleState` for cross-tool integration.
pub enum Geo84CrossAction {
    None,
    AddToMineralSetup(MineralGroupDef),
    JumpToCatalogue(String),
}

pub struct GeoMineralogyUiState {
    pub db: GeoDatabaseStore,

    // ---- 1. Database Viewer (§84.2, Images 3–11) ----
    pub db_source: GeoDbSource,
    pub db_edit_mode: bool,
    pub db_find_text: String,
    pub db_find_applied: String,
    pub db_show_filter: bool,
    pub db_filter_enabled: bool,
    pub db_filter_text_tab: bool,
    pub db_filter_logic: FilterLogic,
    pub db_col_filters: BTreeMap<String, (FilterOp, String)>,
    pub db_page: usize,
    pub db_active_cell_preview: String,
    pub db_selected_rows: BTreeSet<usize>,
    pub db_confirm_delete: bool,
    pub db_show_about: bool,
    pub db_show_help: bool,
    pub db_show_paste_modal: bool,
    pub db_paste_buf: String,
    pub db_paste_report: Option<PasteReport>,
    pub db_show_add_modal: bool,
    pub db_add_source: GeoDbSource,
    pub db_add_mineral_name: String,
    pub db_add_location: String,
    pub db_add_mineral_key: String,
    pub db_add_reference: String,
    pub db_add_sg: String,
    pub db_add_extra_fields: Vec<(String, String)>,

    // ---- 2. Find by Properties (§84.3, Images 12–14) ----
    pub prop_filter: PropertySearchFilter,
    pub prop_expr_text_tab: bool,
    pub prop_density_min_str: String,
    pub prop_density_max_str: String,
    pub prop_results: Vec<usize>,
    pub prop_selected_idx: Option<usize>,

    // ---- 3. Catalogue (§84.4, Image 15) ----
    pub cat_search: String,
    pub cat_elem_filter: String,
    pub cat_show_filter_popup: bool,
    pub cat_selected_mineral: String,
    pub cat_selected_record_id: Option<u32>,
    pub cat_show_general_props: bool,
    pub cat_show_variability: bool,
    pub cat_show_scatter: bool,
    pub cat_show_pie: bool,
    pub cat_scatter_x: String,
    pub cat_scatter_y: String,
    pub cat_scatter_z: String,
    pub cat_scatter_3d: bool,

    // ---- 4. Find by Composition (§84.5) ----
    pub comp_criteria: Vec<CompCriterion>,
    pub comp_top_n: usize,
    pub comp_matches: Vec<CompositionMatch>,
    pub comp_selected_match: Option<usize>,
    pub comp_show_add_modal: bool,
    pub comp_show_ptable: bool,
    pub comp_new_species: String,
    pub comp_new_is_oxide: bool,
    pub comp_new_unit: CompUnit,
    pub comp_new_val: f64,
    pub comp_new_sd: f64,

    // ---- 5. Petrological (§84.6) ----
    pub petro: PetrologicalProject,
    petro_tab: PetroViewTab,
    petro_active_sheet: usize,
    petro_active_result: usize,
    petro_selected_el: String,
    petro_selected_method_idx: usize,
    petro_show_import_modal: bool,
    petro_import_name: String,
    petro_import_method: String,
    petro_import_tsv: String,

    // ---- 6. Modal & Monte Carlo (§84.7) ----
    pub modal: ModalProject,
    modal_top_tab: ModalTopTab,
    modal_calc_tab: ModalCalcTab,
    modal_res_tab: ModalResultSubTab,
    modal_mc_tab: McResultSubTab,
    modal_active_round: usize,
    modal_selected_mineral_list: Option<usize>,
    modal_dist_element: String,
    modal_residual_relative: bool,
    modal_show_add_col_modal: bool,
    modal_show_select_min_modal: bool,
    modal_min_search: String,
    modal_mc_seed: u64,

    // ---- File export & status ----
    export_modal: Geo84ExportKind,
    export_path: String,
    pub status_msg: Option<String>,
    pub status_err: Option<String>,
}

impl Default for GeoMineralogyUiState {
    fn default() -> Self {
        let db = GeoDatabaseStore::load_default();
        let mut prop_filter = PropertySearchFilter::default();
        prop_filter
            .element_states
            .insert("Pb".into(), ElementFilterState::MustContain);
        prop_filter
            .element_states
            .insert("S".into(), ElementFilterState::MustContain);
        let prop_results = filter_minerals_by_properties(&db, &prop_filter);
        let prop_selected_idx = prop_results
            .iter()
            .copied()
            .find(|&i| db.minerals[i].name == "Galena")
            .or_else(|| prop_results.first().copied());

        let comp_criteria = default_eds_criteria();
        let comp_matches = find_minerals_by_composition_wssq(&db, &comp_criteria, 10);

        let petro = PetrologicalProject::default();
        let mut modal = ModalProject::default();
        modal.run_monte_carlo(42);

        Self {
            db,
            db_source: GeoDbSource::MineralChemistry,
            db_edit_mode: false,
            db_find_text: String::new(),
            db_find_applied: String::new(),
            db_show_filter: false,
            db_filter_enabled: true,
            db_filter_text_tab: false,
            db_filter_logic: FilterLogic::And,
            db_col_filters: BTreeMap::new(),
            db_page: 0,
            db_active_cell_preview: "Sphalerite".into(),
            db_selected_rows: BTreeSet::new(),
            db_confirm_delete: false,
            db_show_about: false,
            db_show_help: false,
            db_show_paste_modal: false,
            db_paste_buf: "Mineral Name\tLocation\tMineral Key\tFe %\tZn %\tS %\tCd %\tCuO %\tIn ppm\nSphalerite\tTest\ttest\t6.22\t59.9\t32.2\t0.01\t0.02\t200\nPyrite\tTest\ttest\t45\t\t55\t0.01\t\t100\n".into(),
            db_paste_report: None,
            db_show_add_modal: false,
            db_add_source: GeoDbSource::MineralChemistry,
            db_add_mineral_name: "Sphalerite".into(),
            db_add_location: "New Mine".into(),
            db_add_mineral_key: "Ref: Report 121".into(),
            db_add_reference: String::new(),
            db_add_sg: "4.05".into(),
            db_add_extra_fields: vec![
                ("Zn".into(), "64.0".into()),
                ("S".into(), "33.0".into()),
                ("Fe".into(), "3.0".into()),
            ],

            prop_filter,
            prop_expr_text_tab: true,
            prop_density_min_str: String::new(),
            prop_density_max_str: String::new(),
            prop_results,
            prop_selected_idx,

            cat_search: String::new(),
            cat_elem_filter: String::new(),
            cat_show_filter_popup: false,
            cat_selected_mineral: "Sphalerite".into(),
            cat_selected_record_id: None,
            cat_show_general_props: true,
            cat_show_variability: true,
            cat_show_scatter: true,
            cat_show_pie: true,
            cat_scatter_x: "Zn".into(),
            cat_scatter_y: "S".into(),
            cat_scatter_z: "Fe".into(),
            cat_scatter_3d: false,

            comp_criteria,
            comp_top_n: 10,
            comp_matches,
            comp_selected_match: Some(0),
            comp_show_add_modal: false,
            comp_show_ptable: false,
            comp_new_species: "Cu".into(),
            comp_new_is_oxide: false,
            comp_new_unit: CompUnit::WtPct,
            comp_new_val: 26.0,
            comp_new_sd: 1.0,

            petro,
            petro_tab: PetroViewTab::Parameters,
            petro_active_sheet: 4, // XRF
            petro_active_result: 0,
            petro_selected_el: "Ni".into(),
            petro_selected_method_idx: 0,
            petro_show_import_modal: false,
            petro_import_name: "CustomLab".into(),
            petro_import_method: "ICP".into(),
            petro_import_tsv: "SampleID\tCu_ppm\tNi_ppm\tS_%\nBR-01\t220\t2700\t0.30\nBR-02\t490\t2550\t0.65\n".into(),

            modal,
            modal_top_tab: ModalTopTab::Calculation,
            modal_calc_tab: ModalCalcTab::Rules,
            modal_res_tab: ModalResultSubTab::Modal,
            modal_mc_tab: McResultSubTab::AvgSigma,
            modal_active_round: 0,
            modal_selected_mineral_list: Some(0),
            modal_dist_element: "S".into(),
            modal_residual_relative: false,
            modal_show_add_col_modal: false,
            modal_show_select_min_modal: false,
            modal_min_search: String::new(),
            modal_mc_seed: 100,

            export_modal: Geo84ExportKind::None,
            export_path: "/tmp/hsc_geo_export.xlsx".into(),
            status_msg: None,
            status_err: None,
        }
    }
}

impl GeoMineralogyUiState {
    fn mineral_to_group_def(m: &GeoMineralRecord) -> MineralGroupDef {
        let entry = GeoMineralCatalogEntry {
            id: m.id,
            name: m.name.clone(),
            code: m.symbol.clone(),
            formula: m.formula.clone(),
            group: m.group.clone(),
            chem_wt_pct: m.composition.clone(),
            estimated_sg: m.density,
        };
        MineralGroupDef::from_catalog(&entry)
    }

    pub fn render_modals(&mut self, ctx: &egui::Context) {
        self.ui_db_paste_modal(ctx);
        self.ui_db_paste_report_dialog(ctx);
        self.ui_db_add_modal(ctx);
        self.ui_db_confirm_delete_modal(ctx);
        self.ui_db_about_help_modals(ctx);
        self.ui_comp_add_modal(ctx);
        self.ui_petro_import_modal(ctx);
        self.ui_modal_add_col_modal(ctx);
        self.ui_modal_select_min_modal(ctx);
        self.ui_export_modal(ctx);
    }

    fn active_db_conditions(&self) -> Vec<FilterCondition> {
        if !self.db_filter_enabled {
            return Vec::new();
        }
        self.db_col_filters
            .iter()
            .filter(|(_, (_, val))| !val.trim().is_empty())
            .map(|(field, (op, val))| FilterCondition {
                field: field.clone(),
                op: *op,
                value: val.clone(),
            })
            .collect()
    }

    // ========================================================================
    // 1. DATABASE VIEWER (§84.2 — Exact Images 3–11 Ribbon, Inline Filters,
    //    Right Filter Panel, Yellow Find Highlight, Orange Edit Mode)
    // ========================================================================

    pub fn ui_database_viewer(&mut self, ui: &mut egui::Ui) -> Geo84CrossAction {
        let mut action = Geo84CrossAction::None;
        let can_edit = self.db_source != GeoDbSource::Element;
        let conditions = self.active_db_conditions();
        let matching = self.db.query_rows(
            self.db_source,
            &conditions,
            self.db_filter_logic,
            &self.db_find_applied,
        );

        // Ctrl+C shortcut for Copy Data (Image 6)
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::C)) && !self.db_selected_rows.is_empty() {
            let rows_to_copy: Vec<usize> = self.db_selected_rows.iter().copied().collect();
            let tsv = self.db.export_tsv(self.db_source, &rows_to_copy);
            ui.ctx().copy_text(tsv);
            self.status_msg = Some(format!("Copied {} selected row(s) to clipboard (Ctrl+C).", rows_to_copy.len()));
        }

        // Top HSC Geo Ribbon: Clipboard | Database Table | Editing | Filter | Help (Images 3, 6, 8, 9)
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(236, 241, 248))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Group 1: Clipboard
                    hsc_ribbon_group(ui, "Clipboard", |ui| {
                        ui.vertical(|ui| {
                            if ui.add_sized([88.0, 22.0], egui::Button::new("📋 Copy Data")).on_hover_text("Copy selected or visible rows (Ctrl+C)").clicked() {
                                let copy_idxs: Vec<usize> = if self.db_selected_rows.is_empty() {
                                    matching[..matching.len().min(500)].to_vec()
                                } else {
                                    self.db_selected_rows.iter().copied().collect()
                                };
                                let tsv = self.db.export_tsv(self.db_source, &copy_idxs);
                                ui.ctx().copy_text(tsv);
                                self.status_msg = Some(format!("Copied {} row(s) from '{}' to clipboard.", copy_idxs.len(), self.db_source.label()));
                            }
                            if ui
                                .add_enabled(can_edit && self.db_edit_mode, egui::Button::new("📥 Paste Data").min_size(egui::vec2(88.0, 22.0)))
                                .on_disabled_hover_text("Enable 'View / Edit' first to Paste Data into the database")
                                .clicked()
                            {
                                self.db_show_paste_modal = true;
                            }
                        });
                    });

                    // Group 2: Database Table (scrollable listbox matching Image 3 & 8!)
                    hsc_ribbon_group(ui, "Database Table", |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(2.0))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("geo84_db_src_list")
                                    .max_height(46.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(126.0);
                                        for src in GeoDbSource::ALL {
                                            let sel = self.db_source == src;
                                            let txt = egui::RichText::new(src.label()).size(10.5);
                                            let txt = if sel { txt.strong() } else { txt };
                                            if ui.selectable_label(sel, txt).clicked() {
                                                self.db_source = src;
                                                self.db_page = 0;
                                                self.db_selected_rows.clear();
                                                self.db_col_filters.clear();
                                                if src == GeoDbSource::Element {
                                                    self.db_edit_mode = false;
                                                }
                                            }
                                        }
                                    });
                            });
                    });

                    // Group 3: Editing (View / Edit, Add Record, Remove Record (s))
                    hsc_ribbon_group(ui, "Editing", |ui| {
                        let ve_btn = egui::Button::new(
                            egui::RichText::new(if self.db_edit_mode { "✏\nView / Edit" } else { "📋\nView / Edit" })
                                .size(10.5)
                                .strong()
                                .color(if self.db_edit_mode { egui::Color32::from_rgb(154, 52, 18) } else { egui::Color32::from_rgb(30, 41, 59) }),
                        )
                        .fill(if self.db_edit_mode {
                            egui::Color32::from_rgb(254, 215, 170)
                        } else {
                            egui::Color32::from_rgb(248, 250, 252)
                        })
                        .min_size(egui::vec2(74.0, 46.0));
                        if ui.add_enabled(can_edit, ve_btn).clicked() {
                            self.db_edit_mode = !self.db_edit_mode;
                        }
                        ui.vertical(|ui| {
                            if ui
                                .add_enabled(can_edit && self.db_edit_mode, egui::Button::new("➕ Add Record").min_size(egui::vec2(114.0, 22.0)))
                                .on_disabled_hover_text("Press 'View / Edit' to add records")
                                .clicked()
                            {
                                self.db_add_source = self.db_source;
                                self.db_show_add_modal = true;
                            }
                            if ui
                                .add_enabled(
                                    can_edit && self.db_edit_mode && !self.db_selected_rows.is_empty(),
                                    egui::Button::new("🗑 Remove Record (s)").min_size(egui::vec2(114.0, 22.0)),
                                )
                                .on_disabled_hover_text("Enable 'View / Edit' and select row(s) to remove")
                                .clicked()
                            {
                                self.db_confirm_delete = true;
                            }
                        });
                    });

                    // Group 4: Filter
                    hsc_ribbon_group(ui, "Filter", |ui| {
                        let f_btn = egui::Button::new(egui::RichText::new("🔽\nFilter").size(10.5).strong())
                            .fill(if self.db_show_filter {
                                egui::Color32::from_rgb(219, 234, 254)
                            } else {
                                egui::Color32::from_rgb(248, 250, 252)
                            })
                            .min_size(egui::vec2(54.0, 46.0));
                        if ui.add(f_btn).clicked() {
                            self.db_show_filter = !self.db_show_filter;
                        }
                    });

                    // Far-right Group 5: Help (About, Help, Restore Defaults — Image 3 & 9)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        hsc_ribbon_group(ui, "Help", |ui| {
                            if ui.add_sized([60.0, 46.0], egui::Button::new(egui::RichText::new("↺\nRestore\nDefaults").size(9.5))).clicked() {
                                self.db = GeoDatabaseStore::load_default();
                                self.db_col_filters.clear();
                                self.db_find_text.clear();
                                self.db_find_applied.clear();
                                self.db_selected_rows.clear();
                                self.db_edit_mode = false;
                                self.status_msg = Some("Restored default HSC Geo database and filters.".into());
                            }
                            if ui.add_sized([46.0, 46.0], egui::Button::new(egui::RichText::new("❓\nHelp").size(10.0))).clicked() {
                                self.db_show_help = true;
                            }
                            if ui.add_sized([48.0, 46.0], egui::Button::new(egui::RichText::new("ℹ\nAbout").size(10.0))).clicked() {
                                self.db_show_about = true;
                            }
                        });
                    });
                });
            });

        ui.add_space(3.0);

        // Cell value preview strip + Search Bar (`Enter text to search...` | `Find` | `Clear` — Images 3–7!)
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("✏").color(egui::Color32::from_rgb(100, 116, 139)));
            ui.add_sized(
                [ui.available_width().max(200.0), 19.0],
                egui::TextEdit::singleline(&mut self.db_active_cell_preview).interactive(false),
            );
        });

        ui.horizontal(|ui| {
            let enter = ui
                .add_sized(
                    [260.0, 21.0],
                    egui::TextEdit::singleline(&mut self.db_find_text)
                        .hint_text("Enter text to search... (e.g. DHZ Biotite)"),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.add_sized([54.0, 21.0], egui::Button::new("Find")).clicked() || enter {
                self.db_find_applied = self.db_find_text.clone();
                self.db_page = 0;
            }
            if ui.add_sized([54.0, 21.0], egui::Button::new("Clear")).clicked() {
                self.db_find_text.clear();
                self.db_find_applied.clear();
                self.db_page = 0;
            }
            if self.db_source == GeoDbSource::Mineral && self.db_selected_rows.len() == 1 {
                let &ridx = self.db_selected_rows.iter().next().unwrap();
                if let Some(m) = self.db.minerals.get(ridx) {
                    ui.separator();
                    if ui.small_button(format!("➕ Add '{}' to Mineral Setup", m.name)).clicked() {
                        action = Geo84CrossAction::AddToMineralSetup(Self::mineral_to_group_def(m));
                    }
                    if ui.small_button("❖ Open in Catalogue").clicked() {
                        action = Geo84CrossAction::JumpToCatalogue(m.name.clone());
                    }
                }
            }
        });

        ui.add_space(3.0);

        // Main Split Area: Data Grid (left) + Optional Right-hand `Filter` Dock Panel (Images 4–6!)
        let cols = self.db.columns_for_source(self.db_source);
        let conditions = self.active_db_conditions();
        let matching = self.db.query_rows(
            self.db_source,
            &conditions,
            self.db_filter_logic,
            &self.db_find_applied,
        );
        let total = matching.len();
        let page_size = 60usize;
        let max_page = total.saturating_sub(1) / page_size;
        if self.db_page > max_page {
            self.db_page = max_page;
        }
        let start = self.db_page * page_size;
        let end = (start + page_size).min(total);

        let avail = ui.available_size();
        let filter_panel_w = if self.db_show_filter { 245.0f32 } else { 0.0 };
        let grid_w = (avail.x - filter_panel_w - if self.db_show_filter { 8.0 } else { 0.0 }).max(320.0);
        let grid_h = (avail.y - 48.0).max(180.0);

        let find_tokens: Vec<String> = self
            .db_find_applied
            .split_whitespace()
            .map(|s| s.to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        let col_widths: Vec<f32> = cols
            .iter()
            .map(|c| match c.as_str() {
                "Mineral ID" | "Id" | "Z" => 68.0,
                "Mineral Symbol" | "Symbol" | "SG" => 74.0,
                "Mineral Name" | "Name" | "Mineral" | "Phase" => 112.0,
                "Location" | "Mineral Key" | "Reference" => 126.0,
                "Formula" | "Formula / Desc" => 130.0,
                "Composition" | "Elements" | "Note" => 165.0,
                _ => 72.0,
            })
            .collect();

        let edit_bg = egui::Color32::from_rgb(252, 228, 200); // Exact HSC peach-orange Edit mode (Images 9–10!)

        ui.horizontal_top(|ui| {
            // Left: Database Grid with Inline Auto-Filter Row
            ui.vertical(|ui| {
                ui.set_min_size(egui::vec2(grid_w, grid_h));
                ui.set_max_size(egui::vec2(grid_w, grid_h));

                egui::ScrollArea::both()
                    .id_salt("geo84_db_table_scroll")
                    .max_width(grid_w)
                    .max_height(grid_h)
                    .show(ui, |ui| {
                        // Row 1: Column Headers
                        ui.horizontal(|ui| {
                            header_cell(ui, "", 28.0);
                            for (ci, h) in cols.iter().enumerate() {
                                header_cell(ui, h, col_widths[ci]);
                            }
                        });

                        // Row 2: Inline Auto-Filter Row (Image 4 & 5: clickable operator badge + text input!)
                        ui.horizontal(|ui| {
                            widget_cell(ui, 28.0, egui::Color32::from_rgb(245, 247, 250), false, |ui| {
                                ui.label(egui::RichText::new("🔽").size(9.5));
                            });
                            for (ci, h) in cols.iter().enumerate() {
                                let w = col_widths[ci];
                                let entry = self
                                    .db_col_filters
                                    .entry(h.clone())
                                    .or_insert_with(|| (FilterOp::Contains, String::new()));
                                widget_cell(ui, w, egui::Color32::WHITE, false, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        let badge_col = match entry.0 {
                                            FilterOp::Contains => egui::Color32::from_rgb(22, 163, 74),
                                            FilterOp::BeginsWith | FilterOp::EndsWith => egui::Color32::from_rgb(37, 99, 235),
                                            FilterOp::NotContains | FilterOp::NotEquals => egui::Color32::from_rgb(220, 38, 38),
                                            _ => egui::Color32::from_rgb(71, 85, 105),
                                        };
                                        let badge_resp = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(entry.0.short_badge())
                                                    .size(8.5)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(badge_col)
                                            .min_size(egui::vec2(22.0, 16.0)),
                                        );
                                        let popup_id = ui.make_persistent_id(format!("col_fop_{ci}_{h}"));
                                        if badge_resp.clicked() {
                                            ui.memory_mut(|m| m.toggle_popup(popup_id));
                                        }
                                        egui::popup_below_widget(
                                            ui,
                                            popup_id,
                                            &badge_resp,
                                            egui::PopupCloseBehavior::CloseOnClick,
                                            |ui| {
                                                ui.set_min_width(155.0);
                                                if ui.button("✖ Clear Filter").clicked() {
                                                    entry.1.clear();
                                                    self.db_page = 0;
                                                }
                                                ui.separator();
                                                for op in FilterOp::ALL {
                                                    if ui.selectable_label(entry.0 == op, format!("{:>4}  {}", op.short_badge(), op.label())).clicked() {
                                                        entry.0 = op;
                                                        self.db_page = 0;
                                                    }
                                                }
                                            },
                                        );
                                        if ui
                                            .add_sized(
                                                [(w - 30.0).max(24.0), 16.0],
                                                egui::TextEdit::singleline(&mut entry.1).frame(false),
                                            )
                                            .changed()
                                        {
                                            self.db_filter_enabled = true;
                                            self.db_page = 0;
                                        }
                                    });
                                });
                            }
                        });

                        // Data Rows
                        for &ridx in &matching[start..end] {
                            let selected = self.db_selected_rows.contains(&ridx);
                            let row_bg = if selected {
                                egui::Color32::from_rgb(219, 234, 254)
                            } else if self.db_edit_mode {
                                edit_bg
                            } else {
                                egui::Color32::WHITE
                            };

                            if self.db_edit_mode {
                                self.ui_db_editable_row(ui, ridx, &col_widths, row_bg);
                            } else {
                                let cells = self.db.row_cells(self.db_source, ridx);
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 28.0, row_bg, false, |ui| {
                                        let mut chk = selected;
                                        if ui.checkbox(&mut chk, "").changed() {
                                            if chk {
                                                self.db_selected_rows.insert(ridx);
                                            } else {
                                                self.db_selected_rows.remove(&ridx);
                                            }
                                        }
                                    });
                                    for (ci, val) in cells.iter().enumerate() {
                                        let is_hit = !find_tokens.is_empty()
                                            && find_tokens.iter().any(|t| val.to_ascii_lowercase().contains(t));
                                        let cell_bg = if is_hit {
                                            egui::Color32::from_rgb(254, 240, 138) // Yellow Find highlight (Image 7!)
                                        } else {
                                            row_bg
                                        };
                                        widget_cell(ui, col_widths[ci], cell_bg, false, |ui| {
                                            let resp = ui.add(
                                                egui::Label::new(egui::RichText::new(val).size(10.5))
                                                    .sense(egui::Sense::click()),
                                            );
                                            if resp.clicked() {
                                                let ctrl = ui.input(|i| i.modifiers.ctrl);
                                                if !ctrl {
                                                    self.db_selected_rows.clear();
                                                }
                                                self.db_selected_rows.insert(ridx);
                                                self.db_active_cell_preview = val.clone();
                                            }
                                        });
                                    }
                                });
                            }
                        }
                    });
            });

            // Right-Hand `Filter` Panel (`Visual` | `Text` + `Clear` — Images 4, 5, 6!)
            if self.db_show_filter {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(248, 250, 252))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(filter_panel_w, grid_h));
                        ui.set_max_size(egui::vec2(filter_panel_w, grid_h));
                        ui.horizontal(|ui| {
                            ui.strong("Filter");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✖").clicked() {
                                    self.db_show_filter = false;
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!self.db_filter_text_tab, "Visual").clicked() {
                                self.db_filter_text_tab = false;
                            }
                            if ui.selectable_label(self.db_filter_text_tab, "Text").clicked() {
                                self.db_filter_text_tab = true;
                            }
                        });
                        ui.separator();

                        let expr = format_filter_expression(&conditions, self.db_filter_logic);
                        if self.db_filter_text_tab {
                            ui.label(
                                egui::RichText::new(if expr.is_empty() { "(No active column filter)" } else { &expr })
                                    .monospace()
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(29, 78, 216)),
                            );
                        } else {
                            // Visual tree matching Image 4 & 5: clickable red And/Or root + condition pills
                            ui.horizontal(|ui| {
                                let logic_lbl = match self.db_filter_logic {
                                    FilterLogic::And => "And",
                                    FilterLogic::Or => "Or",
                                };
                                if ui
                                    .button(egui::RichText::new(logic_lbl).strong().color(egui::Color32::from_rgb(185, 28, 28)))
                                    .on_hover_text("Click to toggle And / Or")
                                    .clicked()
                                {
                                    self.db_filter_logic = match self.db_filter_logic {
                                        FilterLogic::And => FilterLogic::Or,
                                        FilterLogic::Or => FilterLogic::And,
                                    };
                                }
                                ui.label(egui::RichText::new("(click to switch And/Or)").size(9.5).color(egui::Color32::GRAY));
                            });
                            let mut to_clear = Vec::new();
                            egui::ScrollArea::vertical().id_salt("db_vis_filter").max_height(grid_h - 110.0).show(ui, |ui| {
                                if conditions.is_empty() {
                                    ui.label(egui::RichText::new("Type in the inline filter row under any column header.").size(10.0).color(egui::Color32::GRAY));
                                }
                                for c in &conditions {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new(format!("└ [{}]", c.field)).color(egui::Color32::from_rgb(37, 99, 235)).size(10.5));
                                        ui.label(egui::RichText::new(c.op.label()).color(egui::Color32::from_rgb(22, 163, 74)).size(10.5));
                                        ui.label(egui::RichText::new(&c.value).strong().size(10.5));
                                        if ui.small_button("⊗").clicked() {
                                            to_clear.push(c.field.clone());
                                        }
                                    });
                                }
                            });
                            for f in to_clear {
                                self.db_col_filters.remove(&f);
                            }
                        }

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                            if ui.add_sized([74.0, 22.0], egui::Button::new("Clear")).clicked() {
                                self.db_col_filters.clear();
                                self.db_page = 0;
                            }
                        });
                    });
            }
        });

        // Bottom Record Navigator + Active Filter Expression Strip (Image 5 & 6!)
        ui.horizontal(|ui| {
            if ui.add_enabled(self.db_page > 0, egui::Button::new("|◀").small()).clicked() {
                self.db_page = 0;
            }
            if ui.add_enabled(self.db_page > 0, egui::Button::new("◀").small()).clicked() {
                self.db_page -= 1;
            }
            ui.label(
                egui::RichText::new(format!(
                    "Record {}..{} of {total}  (Total in {}: {})",
                    if total == 0 { 0 } else { start + 1 },
                    end,
                    self.db_source.label(),
                    self.db.row_count(self.db_source)
                ))
                .size(10.5),
            );
            if ui.add_enabled(self.db_page < max_page, egui::Button::new("▶").small()).clicked() {
                self.db_page += 1;
            }
            if ui.add_enabled(self.db_page < max_page, egui::Button::new("▶|").small()).clicked() {
                self.db_page = max_page;
            }
        });

        let expr_bar = format_filter_expression(
            &self
                .db_col_filters
                .iter()
                .filter(|(_, (_, v))| !v.trim().is_empty())
                .map(|(f, (op, v))| FilterCondition { field: f.clone(), op: *op, value: v.clone() })
                .collect::<Vec<_>>(),
            self.db_filter_logic,
        );
        if !expr_bar.is_empty() {
            ui.horizontal(|ui| {
                if ui.small_button("✖").on_hover_text("Clear filter").clicked() {
                    self.db_col_filters.clear();
                }
                ui.checkbox(&mut self.db_filter_enabled, "");
                ui.label(egui::RichText::new(&expr_bar).monospace().size(10.0).color(egui::Color32::from_rgb(30, 58, 138)));
            });
        }

        action
    }

    fn ui_db_editable_row(&mut self, ui: &mut egui::Ui, ridx: usize, widths: &[f32], bg: egui::Color32) {
        let selected = self.db_selected_rows.contains(&ridx);
        ui.horizontal(|ui| {
            widget_cell(ui, 28.0, bg, false, |ui| {
                let mut chk = selected;
                if ui.checkbox(&mut chk, "").changed() {
                    if chk {
                        self.db_selected_rows.insert(ridx);
                    } else {
                        self.db_selected_rows.remove(&ridx);
                    }
                }
            });

            match self.db_source {
                GeoDbSource::MineralChemistry => {
                    let r = &mut self.db.mineral_chemistry[ridx];
                    widget_cell(ui, widths[0], bg, false, |ui| { ui.label(r.id.to_string()); });
                    widget_cell(ui, widths[1], bg, false, |ui| { ui.text_edit_singleline(&mut r.mineral_symbol); });
                    widget_cell(ui, widths[2], bg, false, |ui| { ui.text_edit_singleline(&mut r.mineral); });
                    widget_cell(ui, widths[3], bg, false, |ui| { ui.text_edit_singleline(&mut r.location); });
                    widget_cell(ui, widths[4], bg, false, |ui| { ui.text_edit_singleline(&mut r.mineral_key); });
                    widget_cell(ui, widths[5], bg, false, |ui| { ui.text_edit_singleline(&mut r.statistics); });
                    widget_cell(ui, widths[6], bg, false, |ui| { ui.text_edit_singleline(&mut r.reference); });
                    widget_cell(ui, widths[7], bg, false, |ui| { ui.text_edit_singleline(&mut r.sample_no); });
                    widget_cell(ui, widths[8], bg, false, |ui| { ui.text_edit_singleline(&mut r.note1); });
                    widget_cell(ui, widths[9], bg, false, |ui| { ui.text_edit_singleline(&mut r.note2); });
                    widget_cell(ui, widths[10], bg, false, |ui| { ui.text_edit_singleline(&mut r.note3); });
                    widget_cell(ui, widths[11], bg, false, |ui| { ui.text_edit_singleline(&mut r.database); });
                    let mut sg_val = r.sg.unwrap_or(0.0);
                    widget_cell(ui, widths[12], bg, false, |ui| {
                        if ui.add(egui::DragValue::new(&mut sg_val).speed(0.05).range(0.0..=25.0)).changed() {
                            r.sg = if sg_val > 0.0 { Some(sg_val) } else { None };
                        }
                    });
                    let els = [
                        "Cu", "Fe", "S", "Zn", "Pb", "Sb", "As", "Ag", "Ni", "Co", "Si", "O", "Mg",
                        "Ca", "Al", "Mn", "Cd", "Hg", "In", "Bi",
                    ];
                    for (ei, e) in els.iter().enumerate() {
                        let mut v = r.values.get(*e).copied().unwrap_or(0.0);
                        widget_cell(ui, widths[13 + ei], bg, false, |ui| {
                            if ui.add(egui::DragValue::new(&mut v).speed(0.05).range(0.0..=100.0)).changed() {
                                if v > 0.0 {
                                    r.values.insert((*e).to_string(), v);
                                } else {
                                    r.values.remove(*e);
                                }
                            }
                        });
                    }
                    let tot = r.total_wt();
                    widget_cell(ui, *widths.last().unwrap(), bg, false, |ui| {
                        ui.label(format!("{tot:.2}"));
                    });
                }
                GeoDbSource::Mineral => {
                    let m = &mut self.db.minerals[ridx];
                    widget_cell(ui, widths[0], bg, false, |ui| { ui.label(m.id.to_string()); });
                    widget_cell(ui, widths[1], bg, false, |ui| { ui.text_edit_singleline(&mut m.name); });
                    widget_cell(ui, widths[2], bg, false, |ui| { ui.text_edit_singleline(&mut m.symbol); });
                    widget_cell(ui, widths[3], bg, false, |ui| { ui.text_edit_singleline(&mut m.formula); });
                    widget_cell(ui, widths[4], bg, false, |ui| { ui.text_edit_singleline(&mut m.group); });
                    widget_cell(ui, widths[5], bg, false, |ui| {
                        ui.add(egui::DragValue::new(&mut m.density).speed(0.05).range(0.5..=25.0));
                    });
                    widget_cell(ui, widths[6], bg, false, |ui| { ui.text_edit_singleline(&mut m.color); });
                    widget_cell(ui, widths[7], bg, false, |ui| { ui.text_edit_singleline(&mut m.hardness); });
                    widget_cell(ui, widths[8], bg, false, |ui| { ui.text_edit_singleline(&mut m.luster); });
                    widget_cell(ui, widths[9], bg, false, |ui| { ui.label(&m.elements_str); });
                    widget_cell(ui, widths[10], bg, false, |ui| { ui.label(&m.composition_str); });
                }
                GeoDbSource::PhaseChemistry => {
                    let r = &mut self.db.phase_chemistry[ridx];
                    widget_cell(ui, widths[0], bg, false, |ui| { ui.label(r.id.to_string()); });
                    widget_cell(ui, widths[1], bg, false, |ui| { ui.text_edit_singleline(&mut r.phase_name); });
                    widget_cell(ui, widths[2], bg, false, |ui| { ui.text_edit_singleline(&mut r.process_type); });
                    widget_cell(ui, widths[3], bg, false, |ui| { ui.text_edit_singleline(&mut r.formula_or_desc); });
                    widget_cell(ui, widths[4], bg, false, |ui| {
                        ui.add(egui::DragValue::new(&mut r.density).speed(0.05).range(0.5..=20.0));
                    });
                    widget_cell(ui, widths[5], bg, false, |ui| { ui.text_edit_singleline(&mut r.reference); });
                    let els = ["Fe", "Cu", "S", "Si", "O", "Ca", "Zn", "Pb", "Ni", "Al", "Mg"];
                    let mut tot = 0.0;
                    for (ei, e) in els.iter().enumerate() {
                        let mut v = r.values.get(*e).copied().unwrap_or(0.0);
                        widget_cell(ui, widths[6 + ei], bg, false, |ui| {
                            if ui.add(egui::DragValue::new(&mut v).speed(0.1).range(0.0..=100.0)).changed() {
                                if v > 0.0 { r.values.insert((*e).to_string(), v); } else { r.values.remove(*e); }
                            }
                        });
                        tot += v;
                    }
                    widget_cell(ui, *widths.last().unwrap(), bg, false, |ui| { ui.label(format!("{tot:.2}")); });
                }
                GeoDbSource::MineralDissolution => {
                    let r = &mut self.db.mineral_dissolution[ridx];
                    widget_cell(ui, widths[0], bg, false, |ui| { ui.label(r.id.to_string()); });
                    widget_cell(ui, widths[1], bg, false, |ui| { ui.text_edit_singleline(&mut r.mineral); });
                    widget_cell(ui, widths[2], bg, false, |ui| { ui.text_edit_singleline(&mut r.formula); });
                    widget_cell(ui, widths[3], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.bromine_methanol).range(0.0..=100.0)); });
                    widget_cell(ui, widths[4], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.citric_acid).range(0.0..=100.0)); });
                    widget_cell(ui, widths[5], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.hcl_10pct).range(0.0..=100.0)); });
                    widget_cell(ui, widths[6], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.aqua_regia).range(0.0..=100.0)); });
                    widget_cell(ui, widths[7], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.h2o2_nh4oac).range(0.0..=100.0)); });
                    widget_cell(ui, widths[8], bg, false, |ui| { ui.add(egui::DragValue::new(&mut r.nacn).range(0.0..=100.0)); });
                    widget_cell(ui, widths[9], bg, false, |ui| { ui.text_edit_singleline(&mut r.reference); });
                    widget_cell(ui, widths[10], bg, false, |ui| { ui.text_edit_singleline(&mut r.note); });
                }
                GeoDbSource::Element => {}
            }
        });
    }

    fn remove_selected_db_rows(&mut self) {
        let mut idxs: Vec<usize> = self.db_selected_rows.iter().copied().collect();
        idxs.sort_unstable();
        let count = idxs.len();
        for i in idxs.into_iter().rev() {
            match self.db_source {
                GeoDbSource::MineralChemistry if i < self.db.mineral_chemistry.len() => {
                    self.db.mineral_chemistry.remove(i);
                }
                GeoDbSource::PhaseChemistry if i < self.db.phase_chemistry.len() => {
                    self.db.phase_chemistry.remove(i);
                }
                GeoDbSource::Mineral if i < self.db.minerals.len() => {
                    self.db.minerals.remove(i);
                }
                GeoDbSource::MineralDissolution if i < self.db.mineral_dissolution.len() => {
                    self.db.mineral_dissolution.remove(i);
                }
                _ => {}
            }
        }
        self.db_selected_rows.clear();
        self.status_msg = Some(format!("Removed {count} record(s) from {}.", self.db_source.label()));
    }

    // ========================================================================
    // 2. FIND BY PROPERTIES (§84.3 — 3-State Periodic Table + 3 Lower Panes)
    // ========================================================================

    pub fn ui_find_by_properties(&mut self, ui: &mut egui::Ui) -> Geo84CrossAction {
        let mut action = Geo84CrossAction::None;

        // Top Periodic Table (Family-colored like Images 13 & 14!) & Visual/Text Filter Controls
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(244, 247, 251))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::same(6.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Periodic Table");
                    ui.separator();
                    legend_chip(ui, egui::Color32::from_rgb(250, 232, 42), "1st Click: MUST contain");
                    legend_chip(ui, egui::Color32::from_rgb(235, 52, 52), "2nd Click: MUST NOT contain");
                    ui.separator();
                    if ui.small_button("Example 1: Cu,Cl,O !B..Bi").clicked() {
                        self.prop_filter.clear();
                        for e in ["Cu", "Cl", "O"] {
                            self.prop_filter.element_states.insert(e.into(), ElementFilterState::MustContain);
                        }
                        for e in ["B", "N", "Na", "Mg", "S", "K", "Ca", "Fe", "Zn", "Pb", "Bi"] {
                            self.prop_filter.element_states.insert(e.into(), ElementFilterState::MustNotContain);
                        }
                        self.apply_property_filter();
                    }
                    if ui.small_button("Example 2: Pb,S !Ag,As,Sb,Bi").clicked() {
                        self.prop_filter.clear();
                        for e in ["Pb", "S"] {
                            self.prop_filter.element_states.insert(e.into(), ElementFilterState::MustContain);
                        }
                        for e in ["Ag", "As", "Sb", "Bi"] {
                            self.prop_filter.element_states.insert(e.into(), ElementFilterState::MustNotContain);
                        }
                        self.apply_property_filter();
                    }
                    ui.separator();
                    ui.label("Name:");
                    if ui.add_sized([92.0, 19.0], egui::TextEdit::singleline(&mut self.prop_filter.name_contains)).changed() {
                        self.apply_property_filter();
                    }
                    ui.label("Group:");
                    if ui.add_sized([82.0, 19.0], egui::TextEdit::singleline(&mut self.prop_filter.group_contains)).changed() {
                        self.apply_property_filter();
                    }
                    ui.label("SG:");
                    if ui.add_sized([38.0, 19.0], egui::TextEdit::singleline(&mut self.prop_density_min_str).hint_text("min")).changed() {
                        self.prop_filter.density_min = self.prop_density_min_str.trim().parse().ok();
                        self.apply_property_filter();
                    }
                    if ui.add_sized([38.0, 19.0], egui::TextEdit::singleline(&mut self.prop_density_max_str).hint_text("max")).changed() {
                        self.prop_filter.density_max = self.prop_density_max_str.trim().parse().ok();
                        self.apply_property_filter();
                    }
                });

                ui.add_space(4.0);
                let mut changed = false;
                let states_snap = self.prop_filter.element_states.clone();
                render_periodic_table_grid(ui, "prop_ptable", 32.0, 21.0, |sym| {
                    match states_snap
                        .get(sym)
                        .copied()
                        .unwrap_or(ElementFilterState::Neutral)
                    {
                        ElementFilterState::MustContain => (
                            egui::Color32::from_rgb(252, 232, 60),
                            egui::Color32::BLACK,
                        ),
                        ElementFilterState::MustNotContain => (
                            egui::Color32::from_rgb(232, 54, 54),
                            egui::Color32::WHITE,
                        ),
                        ElementFilterState::Neutral => (
                            element_family_tint(sym),
                            egui::Color32::from_rgb(25, 32, 44),
                        ),
                    }
                }, |sym| {
                    let cur = self
                        .prop_filter
                        .element_states
                        .get(sym)
                        .copied()
                        .unwrap_or(ElementFilterState::Neutral);
                    let nxt = cur.cycle();
                    if nxt == ElementFilterState::Neutral {
                        self.prop_filter.element_states.remove(sym);
                    } else {
                        self.prop_filter.element_states.insert(sym.to_string(), nxt);
                    }
                    changed = true;
                });

                if changed {
                    self.apply_property_filter();
                }

                ui.add_space(4.0);
                // Visual | Text expression box + large Clear Filter / Apply Filter buttons (exact Images 13 & 14!)
                let expr = self.prop_filter.to_hsc_expression();
                ui.horizontal_top(|ui| {
                    let box_w = (ui.available_width() - 175.0).max(260.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!self.prop_expr_text_tab, "Visual").clicked() {
                                self.prop_expr_text_tab = false;
                            }
                            if ui.selectable_label(self.prop_expr_text_tab, "Text").clicked() {
                                self.prop_expr_text_tab = true;
                            }
                        });
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(5.0))
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(box_w, 40.0));
                                ui.set_max_size(egui::vec2(box_w, 54.0));
                                egui::ScrollArea::vertical().id_salt("prop_expr_scroll").show(ui, |ui| {
                                    if self.prop_expr_text_tab {
                                        ui.label(
                                            egui::RichText::new(if expr.is_empty() { "(All 3,812 minerals)" } else { &expr })
                                                .monospace()
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(29, 78, 216)),
                                        );
                                    } else {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(egui::RichText::new("And").strong().color(egui::Color32::from_rgb(185, 28, 28)));
                                            for (el, st) in &self.prop_filter.element_states {
                                                let (lbl, col) = match st {
                                                    ElementFilterState::MustContain => (format!("Contains({el})"), egui::Color32::from_rgb(22, 163, 74)),
                                                    ElementFilterState::MustNotContain => (format!("Not Contains({el})"), egui::Color32::from_rgb(220, 38, 38)),
                                                    ElementFilterState::Neutral => continue,
                                                };
                                                ui.label(egui::RichText::new(lbl).size(10.0).strong().color(col));
                                            }
                                        });
                                    }
                                });
                            });
                    });

                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([78.0, 44.0], egui::Button::new(egui::RichText::new("✖\nClear Filter").size(10.0))).clicked() {
                                self.prop_filter.clear();
                                self.prop_density_min_str.clear();
                                self.prop_density_max_str.clear();
                                self.apply_property_filter();
                            }
                            if ui.add_sized([78.0, 44.0], egui::Button::new(egui::RichText::new("🔽\nApply Filter").size(10.0).strong())).clicked() {
                                self.apply_property_filter();
                            }
                        });
                    });
                });
            });

        ui.add_space(4.0);

        // Lower 3-Panel Layout: Results | General Properties | Elemental Composition Pie Chart (§84.3)
        let avail_w = ui.available_width();
        let avail_h = ui.available_height().max(220.0);
        let left_w = (avail_w * 0.45).max(270.0);
        let mid_w = (avail_w * 0.28).max(200.0);
        let right_w = (avail_w - left_w - mid_w - 16.0).max(180.0);

        ui.horizontal_top(|ui| {
            // Left: Results Table (exact columns of Images 13 & 14: Id, Name, Symbol, Formula, Empirical Formula, Group)
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(left_w, avail_h));
                    ui.set_max_size(egui::vec2(left_w, avail_h));
                    ui.strong(format!("Results ({} matches)", self.prop_results.len()));
                    egui::ScrollArea::both()
                        .id_salt("prop_results_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Id", 44.0);
                                header_cell(ui, "Name", 118.0);
                                header_cell(ui, "Symbol", 52.0);
                                header_cell(ui, "Formula", 118.0);
                                header_cell(ui, "Empirical Formula", 104.0);
                                header_cell(ui, "Group", 82.0);
                            });
                            for &midx in self.prop_results.iter().take(250) {
                                let m = &self.db.minerals[midx];
                                let sel = self.prop_selected_idx == Some(midx);
                                let bg = if sel {
                                    egui::Color32::from_rgb(219, 234, 254)
                                } else {
                                    egui::Color32::WHITE
                                };
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 44.0, bg, false, |ui| { ui.label(m.id.to_string()); });
                                    widget_cell(ui, 118.0, bg, false, |ui| {
                                        if ui.selectable_label(sel, &m.name).clicked() {
                                            self.prop_selected_idx = Some(midx);
                                        }
                                    });
                                    widget_cell(ui, 52.0, bg, false, |ui| { ui.label(&m.symbol); });
                                    widget_cell(ui, 118.0, bg, false, |ui| {
                                        ui.label(egui::RichText::new(&m.formula).monospace().size(10.0));
                                    });
                                    widget_cell(ui, 104.0, bg, false, |ui| {
                                        ui.label(egui::RichText::new(&m.empirical_formula).monospace().size(10.0));
                                    });
                                    widget_cell(ui, 82.0, bg, false, |ui| { ui.label(&m.group); });
                                });
                            }
                        });
                });

            // Middle: General Properties
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(mid_w, avail_h));
                    ui.set_max_size(egui::vec2(mid_w, avail_h));
                    ui.strong("General Properties");
                    if let Some(midx) = self.prop_selected_idx.filter(|&i| i < self.db.minerals.len()) {
                        let m = self.db.minerals[midx].clone();
                        ui.horizontal(|ui| {
                            if ui.small_button("➕ Mineral Setup").clicked() {
                                action = Geo84CrossAction::AddToMineralSetup(Self::mineral_to_group_def(&m));
                            }
                            if ui.small_button("❖ Catalogue").clicked() {
                                action = Geo84CrossAction::JumpToCatalogue(m.name.clone());
                            }
                        });
                        render_general_properties_table(ui, &m, mid_w - 12.0);
                    } else {
                        ui.label("Select a mineral in Results.");
                    }
                });

            // Right: Elemental Composition Pie Chart
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(right_w, avail_h));
                    ui.set_max_size(egui::vec2(right_w, avail_h));
                    ui.strong("Elemental Composition");
                    if let Some(midx) = self.prop_selected_idx.filter(|&i| i < self.db.minerals.len()) {
                        let m = &self.db.minerals[midx];
                        draw_elemental_pie_chart(ui, &format!("{} (wt-%)", m.name), &m.composition);
                    }
                });
        });

        action
    }

    fn apply_property_filter(&mut self) {
        self.prop_results = filter_minerals_by_properties(&self.db, &self.prop_filter);
        self.prop_selected_idx = self.prop_results.first().copied();
    }

    // ========================================================================
    // 3. CATALOGUE (§84.4 — exact `Imagen pegada (16).png` layout!)
    // ========================================================================

    pub fn ui_catalogue(&mut self, ui: &mut egui::Ui) -> Geo84CrossAction {
        let mut action = Geo84CrossAction::None;

        // Top HSC Geo `Catalogue` Ribbon with 4 exact toggle buttons (Image 15!)
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(236, 241, 248))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    hsc_ribbon_group(ui, "Catalogue", |ui| {
                        for (flag, icon, lbl) in [
                            (&mut self.cat_show_general_props, "📋", "General\nProperties Table"),
                            (&mut self.cat_show_variability, "📊", "Variability\nChart"),
                            (&mut self.cat_show_scatter, "⁘", "Scatter\nPlot"),
                            (&mut self.cat_show_pie, "◔", "Composition\nChart"),
                        ] {
                            let btn = egui::Button::new(egui::RichText::new(format!("{icon}\n{lbl}")).size(9.5))
                                .fill(if *flag {
                                    egui::Color32::from_rgb(219, 234, 254)
                                } else {
                                    egui::Color32::from_rgb(248, 250, 252)
                                })
                                .min_size(egui::vec2(86.0, 46.0));
                            if ui.add(btn).clicked() {
                                *flag = !*flag;
                            }
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(m) = self
                            .db
                            .minerals
                            .iter()
                            .find(|x| x.name.eq_ignore_ascii_case(&self.cat_selected_mineral))
                            .cloned()
                        {
                            if ui.button("➕ Add to Modal").clicked() {
                                self.modal.add_mineral(ModalMineralDef {
                                    name: m.name.clone(),
                                    location: "[stoichiometric]".into(),
                                    formula: m.formula.clone(),
                                    sg: m.density,
                                    composition: m.composition.clone(),
                                });
                                self.status_msg = Some(format!("Added '{}' to Modal mineral list.", m.name));
                            }
                            if ui.button("➕ Add to Mineral Setup").clicked() {
                                action = Geo84CrossAction::AddToMineralSetup(Self::mineral_to_group_def(&m));
                            }
                        }
                    });
                });
            });
        ui.add_space(3.0);

        let selected_mineral = self
            .db
            .minerals
            .iter()
            .find(|x| x.name.eq_ignore_ascii_case(&self.cat_selected_mineral))
            .cloned()
            .or_else(|| self.db.minerals.first().cloned());

        let analyses = selected_mineral
            .as_ref()
            .map(|m| self.db.analyses_for_mineral(&m.name))
            .unwrap_or_default();

        // Collect all elements present across analyses of this mineral (ordered by max wt% descending)
        let mut el_max: BTreeMap<String, f64> = BTreeMap::new();
        for a in &analyses {
            for (k, &v) in &a.values {
                let entry = el_max.entry(k.clone()).or_insert(0.0);
                if v > *entry {
                    *entry = v;
                }
            }
        }
        let mut ordered_els: Vec<String> = el_max.keys().cloned().collect();
        ordered_els.sort_by(|a, b| el_max[b].total_cmp(&el_max[a]));

        if !ordered_els.is_empty() {
            if !ordered_els.contains(&self.cat_scatter_x) {
                self.cat_scatter_x = ordered_els[0].clone();
            }
            if !ordered_els.contains(&self.cat_scatter_y) {
                self.cat_scatter_y = ordered_els
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| ordered_els[0].clone());
            }
            if !ordered_els.contains(&self.cat_scatter_z) {
                self.cat_scatter_z = ordered_els
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| self.cat_scatter_y.clone());
            }
        }

        let avail = ui.available_size();
        let left_w = if self.cat_show_general_props { 225.0f32 } else { 0.0 };
        let right_w = (avail.x - left_w - if self.cat_show_general_props { 10.0 } else { 2.0 }).max(400.0);
        let any_chart = self.cat_show_variability || self.cat_show_scatter || self.cat_show_pie;
        let top_h = if any_chart { (avail.y * 0.45).max(190.0) } else { avail.y.max(260.0) };
        let bot_h = (avail.y - top_h - 10.0).max(200.0);

        ui.horizontal_top(|ui| {
            // Far-Left full-height pane: General Properties (toggled via Ribbon — Image 15!)
            if self.cat_show_general_props {
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(4.0))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(left_w, avail.y.max(380.0)));
                        ui.set_max_size(egui::vec2(left_w, avail.y.max(380.0)));
                        ui.strong("General Properties");
                        if let Some(m) = &selected_mineral {
                            render_general_properties_table(ui, m, left_w - 10.0);
                        }
                    });
            }

            // Right composite column: Top (List of Minerals + Variability Table) + Bottom (3 Charts)
            ui.vertical(|ui| {
                // Top Row: List of Minerals | Variability Table
                ui.horizontal_top(|ui| {
                    let list_w = 185.0f32;
                    let table_w = (right_w - list_w - 8.0).max(220.0);

                    // List of Minerals
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(4.0))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(list_w, top_h));
                            ui.set_max_size(egui::vec2(list_w, top_h));
                            ui.strong("List of Minerals");
                            if ui
                                .add_sized(
                                    [list_w - 10.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.cat_search).hint_text("Sphal..."),
                                )
                                .changed()
                            {
                                // Exact Image 15 behavior: auto-select first mineral matching prefix/text!
                                let q_now = self.cat_search.trim().to_ascii_lowercase();
                                if !q_now.is_empty() {
                                    if let Some(hit) = self
                                        .db
                                        .minerals
                                        .iter()
                                        .find(|m| m.name.to_ascii_lowercase().starts_with(&q_now))
                                        .or_else(|| {
                                            self.db
                                                .minerals
                                                .iter()
                                                .find(|m| m.name.to_ascii_lowercase().contains(&q_now))
                                        })
                                    {
                                        self.cat_selected_mineral = hit.name.clone();
                                        self.cat_selected_record_id = None;
                                    }
                                }
                            }
                            let q = self.cat_search.trim().to_ascii_lowercase();
                            let req_els: Vec<String> = self
                                .cat_elem_filter
                                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                                .filter(|s| !s.is_empty())
                                .map(str::to_string)
                                .collect();

                            let list_h = (top_h - 92.0).max(90.0);
                            egui::ScrollArea::vertical()
                                .id_salt("cat_mineral_list")
                                .max_height(list_h)
                                .show(ui, |ui| {
                                    let mut shown = 0usize;
                                    for m in &self.db.minerals {
                                        if !q.is_empty() && !m.name.to_ascii_lowercase().contains(&q) {
                                            continue;
                                        }
                                        if !req_els.is_empty()
                                            && !req_els.iter().all(|e| m.elements.contains(e))
                                        {
                                            continue;
                                        }
                                        let sel = m.name.eq_ignore_ascii_case(&self.cat_selected_mineral);
                                        if ui.selectable_label(sel, &m.name).clicked() {
                                            self.cat_selected_mineral = m.name.clone();
                                            self.cat_selected_record_id = None;
                                        }
                                        shown += 1;
                                        if shown >= 180 {
                                            break;
                                        }
                                    }
                                });

                            ui.horizontal(|ui| {
                                if ui.small_button("🔽 Filter").clicked() {
                                    self.cat_show_filter_popup = !self.cat_show_filter_popup;
                                }
                                if ui.small_button("✖ Clear filter").clicked() {
                                    self.cat_search.clear();
                                    self.cat_elem_filter.clear();
                                }
                            });
                            if self.cat_show_filter_popup {
                                ui.add_sized(
                                    [list_w - 10.0, 18.0],
                                    egui::TextEdit::singleline(&mut self.cat_elem_filter)
                                        .hint_text("Elements: Zn, S"),
                                );
                            }
                        });

                    // Variability Table (multi-location analyses)
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(4.0))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(table_w, top_h));
                            ui.set_max_size(egui::vec2(table_w, top_h));
                            ui.strong(format!("Variability Table (N={})", analyses.len()));
                            egui::ScrollArea::both()
                                .id_salt("cat_var_table")
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        header_cell(ui, "Id", 40.0);
                                        header_cell(ui, "Location", 115.0);
                                        header_cell(ui, "Reference", 95.0);
                                        header_cell(ui, "Note1", 95.0);
                                        header_cell(ui, "Database", 58.0);
                                        for el in ordered_els.iter().take(10) {
                                            header_cell(ui, &format!("{el} %"), 54.0);
                                        }
                                    });
                                    for a in &analyses {
                                        let sel = self.cat_selected_record_id == Some(a.id);
                                        let bg = if sel {
                                            egui::Color32::from_rgb(219, 234, 254)
                                        } else {
                                            egui::Color32::WHITE
                                        };
                                        ui.horizontal(|ui| {
                                            widget_cell(ui, 40.0, bg, false, |ui| {
                                                if ui.selectable_label(sel, a.id.to_string()).clicked() {
                                                    self.cat_selected_record_id = Some(a.id);
                                                }
                                            });
                                            widget_cell(ui, 115.0, bg, false, |ui| { ui.label(&a.location); });
                                            widget_cell(ui, 95.0, bg, false, |ui| { ui.label(&a.reference); });
                                            widget_cell(ui, 95.0, bg, false, |ui| { ui.label(&a.note1); });
                                            widget_cell(ui, 58.0, bg, false, |ui| { ui.label(&a.database); });
                                            for el in ordered_els.iter().take(10) {
                                                let txt = a
                                                    .values
                                                    .get(el)
                                                    .map(|v| format!("{v:.2}"))
                                                    .unwrap_or_default();
                                                widget_cell(ui, 54.0, bg, false, |ui| {
                                                    ui.label(egui::RichText::new(txt).monospace().size(10.0));
                                                });
                                            }
                                        });
                                    }
                                });
                        });
                });

                // Bottom Row: Variability Chart | Scatter Plot | Elemental Composition Pie
                if any_chart {
                    ui.add_space(4.0);
                    let count = self.cat_show_variability as usize
                        + self.cat_show_scatter as usize
                        + self.cat_show_pie as usize;
                    let panel_w = ((right_w - (count.saturating_sub(1) as f32) * 6.0) / count.max(1) as f32).max(160.0);

                    ui.horizontal_top(|ui| {
                        if self.cat_show_variability {
                            egui::Frame::none()
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                .inner_margin(egui::Margin::same(4.0))
                                .show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(panel_w, bot_h));
                                    ui.set_max_size(egui::vec2(panel_w, bot_h));
                                    let title = format!(
                                        "{} all records (N={})",
                                        self.cat_selected_mineral,
                                        analyses.len()
                                    );
                                    draw_variability_strip_chart(
                                        ui,
                                        &title,
                                        &ordered_els,
                                        &analyses,
                                        self.cat_selected_record_id,
                                    );
                                });
                        }

                        if self.cat_show_scatter {
                            egui::Frame::none()
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                .inner_margin(egui::Margin::same(4.0))
                                .show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(panel_w, bot_h));
                                    ui.set_max_size(egui::vec2(panel_w, bot_h));
                                    let title = format!(
                                        "{} Scatter (N={})",
                                        self.cat_selected_mineral,
                                        analyses.len()
                                    );
                                    if let Some(clicked_id) = draw_catalogue_scatter_chart(
                                        ui,
                                        &title,
                                        &analyses,
                                        &self.cat_scatter_x,
                                        &self.cat_scatter_y,
                                        if self.cat_scatter_3d {
                                            Some(self.cat_scatter_z.as_str())
                                        } else {
                                            None
                                        },
                                        self.cat_selected_record_id,
                                    ) {
                                        self.cat_selected_record_id = Some(clicked_id);
                                    }
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("X");
                                        egui::ComboBox::from_id_salt("cat_sc_x")
                                            .selected_text(format!("{} %", self.cat_scatter_x))
                                            .width(56.0)
                                            .show_ui(ui, |ui| {
                                                for el in &ordered_els {
                                                    ui.selectable_value(&mut self.cat_scatter_x, el.clone(), format!("{el} %"));
                                                }
                                            });
                                        ui.label("Y");
                                        egui::ComboBox::from_id_salt("cat_sc_y")
                                            .selected_text(format!("{} %", self.cat_scatter_y))
                                            .width(56.0)
                                            .show_ui(ui, |ui| {
                                                for el in &ordered_els {
                                                    ui.selectable_value(&mut self.cat_scatter_y, el.clone(), format!("{el} %"));
                                                }
                                            });
                                        ui.checkbox(&mut self.cat_scatter_3d, "3D");
                                        if self.cat_scatter_3d {
                                            egui::ComboBox::from_id_salt("cat_sc_z")
                                                .selected_text(format!("{} %", self.cat_scatter_z))
                                                .width(52.0)
                                                .show_ui(ui, |ui| {
                                                    for el in &ordered_els {
                                                        ui.selectable_value(&mut self.cat_scatter_z, el.clone(), format!("{el} %"));
                                                    }
                                                });
                                        }
                                    });
                                });
                        }

                        if self.cat_show_pie {
                            egui::Frame::none()
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                .inner_margin(egui::Margin::same(4.0))
                                .show(ui, |ui| {
                                    ui.set_min_size(egui::vec2(panel_w, bot_h));
                                    ui.set_max_size(egui::vec2(panel_w, bot_h));
                                    let comp = self
                                        .cat_selected_record_id
                                        .and_then(|id| analyses.iter().find(|a| a.id == id))
                                        .map(|a| a.values.clone())
                                        .or_else(|| selected_mineral.as_ref().map(|m| m.composition.clone()))
                                        .unwrap_or_default();
                                    draw_elemental_pie_chart(
                                        ui,
                                        &format!("{} (wt-%)", self.cat_selected_mineral),
                                        &comp,
                                    );
                                });
                        }
                    });
                }
            });
        });

        action
    }

    // ========================================================================
    // 4. FIND BY COMPOSITION (§84.5 — WSSQ Mineral Grain Identification)
    // ========================================================================

    pub fn ui_find_by_composition(&mut self, ui: &mut egui::Ui) -> Geo84CrossAction {
        let mut action = Geo84CrossAction::None;

        egui::Frame::none()
            .fill(egui::Color32::from_rgb(241, 245, 250))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("➕ Add Row").clicked() {
                        self.comp_show_add_modal = true;
                    }
                    if ui.selectable_label(self.comp_show_ptable, "⚛ Periodic Table").clicked() {
                        self.comp_show_ptable = !self.comp_show_ptable;
                    }
                    if ui.button("🧹 Clear").clicked() {
                        self.comp_criteria.clear();
                        self.comp_matches.clear();
                    }
                    if ui.button("📖 Load EDS Example (Tetrahedrite)").clicked() {
                        self.comp_criteria = default_eds_criteria();
                        self.comp_matches = find_minerals_by_composition_wssq(
                            &self.db,
                            &self.comp_criteria,
                            self.comp_top_n,
                        );
                        self.comp_selected_match = Some(0);
                    }
                    ui.separator();
                    ui.label("Top N:");
                    ui.add(egui::DragValue::new(&mut self.comp_top_n).range(1..=50));
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("🔍 Find").strong().color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(37, 99, 235)),
                        )
                        .clicked()
                    {
                        self.comp_matches = find_minerals_by_composition_wssq(
                            &self.db,
                            &self.comp_criteria,
                            self.comp_top_n,
                        );
                        self.comp_selected_match = if self.comp_matches.is_empty() { None } else { Some(0) };
                        self.status_msg = Some(format!(
                            "WSSQ search finished: top match '{}' (WSSQ = {:.3}).",
                            self.comp_matches.first().map(|m| m.mineral.as_str()).unwrap_or("-"),
                            self.comp_matches.first().map(|m| m.wssq).unwrap_or(0.0)
                        ));
                    }
                });
            });

        if self.comp_show_ptable {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(248, 250, 252))
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Click an element to add it to the EDS/microprobe criteria list:").size(10.5));
                    let existing: BTreeSet<String> =
                        self.comp_criteria.iter().map(|c| c.species.clone()).collect();
                    render_periodic_table_grid(
                        ui,
                        "comp_ptable",
                        30.0,
                        20.0,
                        |sym| {
                            if existing.contains(sym) {
                                (egui::Color32::from_rgb(59, 130, 246), egui::Color32::WHITE)
                            } else {
                                (egui::Color32::from_rgb(229, 234, 241), egui::Color32::from_rgb(30, 41, 59))
                            }
                        },
                        |sym| {
                            if !self.comp_criteria.iter().any(|c| c.species == sym) {
                                self.comp_criteria.push(CompCriterion {
                                    species: sym.to_string(),
                                    is_oxide: false,
                                    unit: CompUnit::WtPct,
                                    value: 10.0,
                                    sd: 1.0,
                                });
                            }
                        },
                    );
                });
        }

        ui.add_space(4.0);
        let avail_w = ui.available_width();
        let avail_h = ui.available_height().max(260.0);
        let left_w = (avail_w * 0.36).max(280.0);
        let right_w = (avail_w - left_w - 10.0).max(320.0);

        ui.horizontal_top(|ui| {
            // Left: Measured Composition Input Grid
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(left_w, avail_h));
                    ui.set_max_size(egui::vec2(left_w, avail_h));
                    ui.strong("Measured Analysis (SEM / EDS / EPMA)");
                    ui.label(
                        egui::RichText::new("WSSQ = Σ ((Database_E − Given_E) / SD_E)²")
                            .size(10.5)
                            .color(egui::Color32::from_rgb(71, 85, 105)),
                    );
                    ui.add_space(4.0);
                    let mut remove_idx = None;
                    egui::ScrollArea::vertical()
                        .id_salt("comp_criteria_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Element/Oxide", 86.0);
                                header_cell(ui, "Ox?", 34.0);
                                header_cell(ui, "Unit", 48.0);
                                header_cell(ui, "Value", 62.0);
                                header_cell(ui, "SD", 52.0);
                                header_cell(ui, "", 24.0);
                            });
                            for (i, c) in self.comp_criteria.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 86.0, egui::Color32::WHITE, false, |ui| {
                                        ui.text_edit_singleline(&mut c.species);
                                    });
                                    widget_cell(ui, 34.0, egui::Color32::WHITE, false, |ui| {
                                        ui.checkbox(&mut c.is_oxide, "");
                                    });
                                    widget_cell(ui, 48.0, egui::Color32::WHITE, false, |ui| {
                                        egui::ComboBox::from_id_salt(format!("cc_u_{i}"))
                                            .selected_text(c.unit.label())
                                            .width(42.0)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut c.unit, CompUnit::WtPct, "%");
                                                ui.selectable_value(&mut c.unit, CompUnit::Ppm, "ppm");
                                                ui.selectable_value(&mut c.unit, CompUnit::Ppb, "ppb");
                                            });
                                    });
                                    widget_cell(ui, 62.0, egui::Color32::WHITE, false, |ui| {
                                        ui.add(egui::DragValue::new(&mut c.value).speed(0.2).range(0.0..=100.0));
                                    });
                                    widget_cell(ui, 52.0, egui::Color32::WHITE, false, |ui| {
                                        ui.add(egui::DragValue::new(&mut c.sd).speed(0.1).range(0.01..=50.0));
                                    });
                                    widget_cell(ui, 24.0, egui::Color32::WHITE, false, |ui| {
                                        if ui.small_button("✖").clicked() {
                                            remove_idx = Some(i);
                                        }
                                    });
                                });
                            }
                        });
                    if let Some(ri) = remove_idx {
                        self.comp_criteria.remove(ri);
                    }
                    let sum_pct: f64 = self.comp_criteria.iter().map(|c| c.unit.to_wt_pct(c.value)).sum();
                    ui.separator();
                    ui.label(egui::RichText::new(format!("Sum of given components: {sum_pct:.2} wt%")).strong());
                });

            // Right: Ranked WSSQ Candidates
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(right_w, avail_h));
                    ui.set_max_size(egui::vec2(right_w, avail_h));
                    ui.horizontal(|ui| {
                        ui.strong(format!("Identified Candidates (Top {} by lowest WSSQ)", self.comp_matches.len()));
                        if let Some(mi) = self.comp_selected_match.filter(|&i| i < self.comp_matches.len()) {
                            let m_name = self.comp_matches[mi].mineral.clone();
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("❖ Open in Catalogue").clicked() {
                                    action = Geo84CrossAction::JumpToCatalogue(m_name.clone());
                                }
                                if ui.small_button("➕ Add to Modal").clicked() {
                                    let cm = &self.comp_matches[mi];
                                    self.modal.add_mineral(ModalMineralDef {
                                        name: cm.mineral.clone(),
                                        location: cm.location.clone(),
                                        formula: cm.formula.clone(),
                                        sg: 4.5,
                                        composition: cm.database_values.clone(),
                                    });
                                    self.status_msg = Some(format!("Added '{}' ({}) to Modal.", cm.mineral, cm.location));
                                }
                            });
                        }
                    });
                    ui.add_space(4.0);

                    let crit_els: Vec<String> = self.comp_criteria.iter().map(|c| c.species.clone()).collect();
                    egui::ScrollArea::both()
                        .id_salt("comp_matches_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "#", 28.0);
                                header_cell(ui, "Mineral", 118.0);
                                header_cell(ui, "Location", 112.0);
                                header_cell(ui, "WSSQ", 68.0);
                                header_cell(ui, "Formula", 118.0);
                                for el in &crit_els {
                                    header_cell(ui, &format!("{el} %"), 56.0);
                                }
                            });
                            for (i, m) in self.comp_matches.iter().enumerate() {
                                let sel = self.comp_selected_match == Some(i);
                                let bg = if sel {
                                    egui::Color32::from_rgb(219, 234, 254)
                                } else if i == 0 {
                                    egui::Color32::from_rgb(220, 252, 231)
                                } else {
                                    egui::Color32::WHITE
                                };
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 28.0, bg, false, |ui| { ui.label(format!("{}", i + 1)); });
                                    widget_cell(ui, 118.0, bg, false, |ui| {
                                        if ui.selectable_label(sel, &m.mineral).clicked() {
                                            self.comp_selected_match = Some(i);
                                        }
                                    });
                                    widget_cell(ui, 112.0, bg, false, |ui| { ui.label(&m.location); });
                                    widget_cell(ui, 68.0, bg, false, |ui| {
                                        ui.label(egui::RichText::new(format!("{:.3}", m.wssq)).monospace().strong());
                                    });
                                    widget_cell(ui, 118.0, bg, false, |ui| {
                                        ui.label(egui::RichText::new(&m.formula).monospace().size(10.0));
                                    });
                                    for el in &crit_els {
                                        let v = m.database_values.get(el).copied().unwrap_or(0.0);
                                        widget_cell(ui, 56.0, bg, false, |ui| {
                                            ui.label(egui::RichText::new(format!("{v:.2}")).monospace().size(10.0));
                                        });
                                    }
                                });
                            }
                        });
                });
        });

        action
    }

    // ========================================================================
    // 5. PETROLOGICAL CALCULATIONS (§84.6 — Bruvann + 13 Routines)
    // ========================================================================

    pub fn ui_petrological(&mut self, ui: &mut egui::Ui) {
        // Top Ribbon
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(241, 245, 250))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("📖 Load Bruvann Tutorial").clicked() {
                        self.petro = PetrologicalProject::default();
                        self.petro_active_sheet = 4;
                        self.status_msg = Some("Loaded Bruvann 7-sheet tutorial & calculated all routines.".into());
                    }
                    if ui.button("📥 Import Sheet (CSV/TSV)").clicked() {
                        self.petro_show_import_modal = true;
                    }
                    ui.separator();
                    if ui.button("🔍 Identify").clicked() {
                        self.petro.identify_all();
                        self.status_msg = Some("Identified all source columns & updated 12 system rows.".into());
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("▶ Calculate").strong().color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(22, 163, 74)),
                        )
                        .clicked()
                    {
                        self.petro.calculate();
                        self.petro_tab = PetroViewTab::Result;
                        self.status_msg = Some(format!(
                            "Calculated {} petrological result sheets.",
                            self.petro.results.len()
                        ));
                    }
                    if ui.button("💾 Save as Excel (.xlsx)").clicked() {
                        self.export_path = "/tmp/bruvann_petrological.xlsx".into();
                        self.export_modal = Geo84ExportKind::PetroXlsx;
                    }
                    ui.separator();
                    for (tab, lbl) in [
                        (PetroViewTab::Parameters, "Parameters (Input)"),
                        (PetroViewTab::Settings, "Settings"),
                        (PetroViewTab::Result, "Result"),
                        (PetroViewTab::Log, "Log"),
                    ] {
                        if ui.selectable_label(self.petro_tab == tab, egui::RichText::new(lbl).strong()).clicked() {
                            self.petro_tab = tab;
                        }
                    }
                });
            });

        ui.add_space(4.0);
        match self.petro_tab {
            PetroViewTab::Parameters => self.ui_petro_parameters_tab(ui),
            PetroViewTab::Settings => self.ui_petro_settings_tab(ui),
            PetroViewTab::Result => self.ui_petro_result_tab(ui),
            PetroViewTab::Log => {
                ui.strong("Petrological Calculation Log");
                egui::ScrollArea::vertical().id_salt("petro_log_scroll").show(ui, |ui| {
                    for line in &self.petro.log {
                        ui.label(egui::RichText::new(line).monospace().size(11.0));
                    }
                });
            }
        }
    }

    fn ui_petro_parameters_tab(&mut self, ui: &mut egui::Ui) {
        if self.petro.sheets.is_empty() {
            ui.label("No input sheets loaded.");
            return;
        }
        if self.petro_active_sheet >= self.petro.sheets.len() {
            self.petro_active_sheet = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, s) in self.petro.sheets.iter().enumerate() {
                let lbl = format!("{} ({})", s.name, s.rows.len());
                if ui.selectable_label(self.petro_active_sheet == i, lbl).clicked() {
                    self.petro_active_sheet = i;
                }
            }
        });
        ui.add_space(3.0);

        let sheet = &mut self.petro.sheets[self.petro_active_sheet];
        let sys_blue = egui::Color32::from_rgb(219, 234, 254);
        let sys_gray = egui::Color32::from_rgb(241, 245, 249);
        let sys_labels = [
            "HEADER", "ELEMENT", "METHOD", "UNIT", "TABLE", "FIELD", "TYPE", "LENGTH", "DIGITS",
            "INDEXED", "MULTIPLIER", "IDENTIFIED",
        ];

        egui::ScrollArea::both()
            .id_salt("petro_param_grid")
            .show(ui, |ui| {
                // 12 System rows (§84.6.2)
                for (si, lbl) in sys_labels.iter().enumerate() {
                    let bg = if si < 4 || si == 11 { sys_blue } else { sys_gray };
                    ui.horizontal(|ui| {
                        header_cell(ui, lbl, 86.0);
                        for col in &mut sheet.columns {
                            widget_cell(ui, 82.0, bg, false, |ui| match si {
                                0 => { ui.label(egui::RichText::new(&col.header).strong().size(10.0)); }
                                1 => { ui.text_edit_singleline(&mut col.element); }
                                2 => { ui.text_edit_singleline(&mut col.method); }
                                3 => { ui.text_edit_singleline(&mut col.unit); }
                                4 => { ui.label(egui::RichText::new(&col.table).size(10.0)); }
                                5 => { ui.label(egui::RichText::new(&col.field).size(10.0)); }
                                6 => { ui.label(egui::RichText::new(&col.data_type).size(10.0)); }
                                7 => { ui.label(egui::RichText::new(col.length.to_string()).size(10.0)); }
                                8 => { ui.label(egui::RichText::new(col.digits.to_string()).size(10.0)); }
                                9 => { ui.label(if col.indexed { "TRUE" } else { "FALSE" }); }
                                10 => { ui.label(egui::RichText::new(format!("{:.0e}", col.multiplier)).size(9.5)); }
                                11 => {
                                    ui.colored_label(
                                        if col.identified { egui::Color32::from_rgb(21, 128, 61) } else { egui::Color32::from_rgb(185, 28, 28) },
                                        if col.identified { "YES" } else { "NO" },
                                    );
                                }
                                _ => {}
                            });
                        }
                    });
                }

                // Editable sample data rows
                for (ri, row) in sheet.rows.iter_mut().enumerate() {
                    if row.len() < sheet.columns.len() {
                        row.resize(sheet.columns.len(), String::new());
                    }
                    ui.horizontal(|ui| {
                        row_num_cell(ui, ri + 13, 86.0);
                        for cell in row.iter_mut() {
                            widget_cell(ui, 82.0, egui::Color32::WHITE, false, |ui| {
                                ui.text_edit_singleline(cell);
                            });
                        }
                    });
                }
            });
    }

    fn ui_petro_settings_tab(&mut self, ui: &mut egui::Ui) {
        let avail_w = ui.available_width();
        let left_w = (avail_w * 0.36).max(270.0);
        let right_w = (avail_w - left_w - 10.0).max(340.0);

        ui.horizontal_top(|ui| {
            // Left: Data Sources + Group + 13 Calculation Routines
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_width(left_w);
                    ui.set_max_width(left_w);
                    egui::ScrollArea::vertical().id_salt("petro_set_left").show(ui, |ui| {
                        ui.strong("1. Active Data Sources");
                        ui.horizontal_wrapped(|ui| {
                            for s in &mut self.petro.sheets {
                                ui.checkbox(&mut s.enabled, &s.name);
                            }
                        });
                        ui.separator();

                        ui.strong("2. Calculation Group & Options");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.petro.group, PetroGroup::Rock, "ROCK");
                            ui.radio_value(&mut self.petro.group, PetroGroup::Mineral, "MINERAL");
                        });
                        ui.horizontal(|ui| {
                            ui.label("ANION O-basis:");
                            ui.add(egui::DragValue::new(&mut self.petro.anion_oxygens).speed(0.5).range(1.0..=32.0));
                            ui.label("Fe³⁺/FeT:");
                            ui.add(egui::DragValue::new(&mut self.petro.fe2o3_feot_ratio).speed(0.01).range(0.05..=0.50));
                        });
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.strong("3. Calculation Routines");
                            if ui.small_button("All").clicked() {
                                for r in PetroRoutine::ALL {
                                    self.petro.enabled_routines.insert(r);
                                }
                            }
                            if ui.small_button("None").clicked() {
                                self.petro.enabled_routines.clear();
                            }
                        });
                        for r in PetroRoutine::ALL {
                            let mut checked = self.petro.enabled_routines.contains(&r);
                            if ui
                                .checkbox(&mut checked, egui::RichText::new(r.code()).strong())
                                .on_hover_text(r.description())
                                .changed()
                            {
                                if checked {
                                    self.petro.enabled_routines.insert(r);
                                } else {
                                    self.petro.enabled_routines.remove(&r);
                                }
                            }
                            ui.label(egui::RichText::new(format!("   {}", r.description())).size(9.5).color(egui::Color32::GRAY));
                        }
                    });
                });

            // Right: Color-coded Periodic Table + Method Priority (`Top/Up/Down/Bottom`)
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_width(right_w);
                    ui.strong("4. Analytical Method Priority per Element");
                    ui.horizontal_wrapped(|ui| {
                        legend_chip(ui, egui::Color32::from_rgb(229, 234, 241), "0 methods");
                        legend_chip(ui, egui::Color32::from_rgb(134, 239, 172), "1 method");
                        legend_chip(ui, egui::Color32::from_rgb(96, 165, 250), "2+ methods (priority)");
                        legend_chip(ui, egui::Color32::from_rgb(249, 115, 22), "Selected");
                    });
                    ui.add_space(4.0);

                    let priorities = self.petro.method_priorities.clone();
                    let sel_el = self.petro_selected_el.clone();
                    render_periodic_table_grid(
                        ui,
                        "petro_ptable",
                        30.0,
                        21.0,
                        |sym| {
                            let n = priorities.get(sym).map(Vec::len).unwrap_or(0);
                            if sym == sel_el {
                                (egui::Color32::from_rgb(249, 115, 22), egui::Color32::WHITE)
                            } else if n >= 2 {
                                (egui::Color32::from_rgb(96, 165, 250), egui::Color32::WHITE)
                            } else if n == 1 {
                                (egui::Color32::from_rgb(134, 239, 172), egui::Color32::BLACK)
                            } else {
                                (egui::Color32::from_rgb(229, 234, 241), egui::Color32::from_rgb(100, 116, 139))
                            }
                        },
                        |sym| {
                            self.petro_selected_el = sym.to_string();
                            self.petro_selected_method_idx = 0;
                        },
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.strong(format!("Priority Order for Element: {}", self.petro_selected_el));
                    let methods = self
                        .petro
                        .method_priorities
                        .get(&self.petro_selected_el)
                        .cloned()
                        .unwrap_or_default();

                    if methods.is_empty() {
                        ui.label("No analytical methods detected for this element in active sheets.");
                    } else {
                        if self.petro_selected_method_idx >= methods.len() {
                            self.petro_selected_method_idx = 0;
                        }
                        ui.horizontal_top(|ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(248, 250, 252))
                                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(180.0);
                                    for (mi, m) in methods.iter().enumerate() {
                                        let sel = self.petro_selected_method_idx == mi;
                                        if ui.selectable_label(sel, format!("{}. {}", mi + 1, m)).clicked() {
                                            self.petro_selected_method_idx = mi;
                                        }
                                    }
                                });
                            ui.vertical(|ui| {
                                let el = self.petro_selected_el.clone();
                                if ui.add_sized([76.0, 22.0], egui::Button::new("⬆ Top")).clicked() {
                                    self.petro.reorder_method(&el, self.petro_selected_method_idx, i32::MIN);
                                    self.petro_selected_method_idx = 0;
                                }
                                if ui.add_sized([76.0, 22.0], egui::Button::new("▲ Up")).clicked() {
                                    self.petro.reorder_method(&el, self.petro_selected_method_idx, -1);
                                    self.petro_selected_method_idx = self.petro_selected_method_idx.saturating_sub(1);
                                }
                                if ui.add_sized([76.0, 22.0], egui::Button::new("▼ Down")).clicked() {
                                    self.petro.reorder_method(&el, self.petro_selected_method_idx, 1);
                                    if self.petro_selected_method_idx + 1 < methods.len() {
                                        self.petro_selected_method_idx += 1;
                                    }
                                }
                                if ui.add_sized([76.0, 22.0], egui::Button::new("⬇ Bottom")).clicked() {
                                    self.petro.reorder_method(&el, self.petro_selected_method_idx, i32::MAX);
                                    self.petro_selected_method_idx = methods.len().saturating_sub(1);
                                }
                            });
                        });
                    }
                });
        });
    }

    fn ui_petro_result_tab(&mut self, ui: &mut egui::Ui) {
        if self.petro.results.is_empty() {
            ui.label("No results calculated yet. Press '▶ Calculate'.");
            return;
        }
        if self.petro_active_result >= self.petro.results.len() {
            self.petro_active_result = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (i, r) in self.petro.results.iter().enumerate() {
                if ui.selectable_label(self.petro_active_result == i, r.routine.code()).clicked() {
                    self.petro_active_result = i;
                }
            }
        });
        ui.add_space(3.0);

        let res = &self.petro.results[self.petro_active_result];
        ui.strong(&res.title);

        let is_spider = matches!(res.routine, PetroRoutine::ReeN | PetroRoutine::TraceN);
        let table_max_h = if is_spider { 215.0 } else { ui.available_height().max(260.0) };

        egui::ScrollArea::both()
            .id_salt("petro_res_grid")
            .max_height(table_max_h)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 28.0);
                    for h in &res.headers {
                        header_cell(ui, h, if h == "Lithology" { 115.0 } else { 78.0 });
                    }
                });
                for (ri, row) in res.rows.iter().enumerate() {
                    let bg = if ri % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(248, 250, 252)
                    };
                    ui.horizontal(|ui| {
                        row_num_cell(ui, ri + 2, 28.0);
                        for (ci, val) in row.iter().enumerate() {
                            let w = if res.headers.get(ci).map(String::as_str) == Some("Lithology") { 115.0 } else { 78.0 };
                            widget_cell(ui, w, bg, false, |ui| {
                                ui.label(egui::RichText::new(val).monospace().size(10.5));
                            });
                        }
                    });
                }
            });

        // Log-Y Spidergram for REE_N and TRACE_N!
        if is_spider {
            ui.add_space(6.0);
            let n_el = if res.routine == PetroRoutine::ReeN { 14 } else { res.headers.len().saturating_sub(1) };
            let x_labels: Vec<String> = res.headers[1..(1 + n_el).min(res.headers.len())]
                .iter()
                .map(|s| s.split('_').next().unwrap_or(s).to_string())
                .collect();
            let mut series = Vec::new();
            for row in &res.rows {
                let sname = row.first().cloned().unwrap_or_default();
                let vals: Vec<f64> = row[1..(1 + x_labels.len()).min(row.len())]
                    .iter()
                    .map(|s| s.parse::<f64>().unwrap_or(0.0))
                    .collect();
                series.push((sname, vals));
            }
            draw_log_spidergram(ui, &res.title, &x_labels, &series);
        }
    }

    // ========================================================================
    // 6. MODAL CALCULATION & MONTE CARLO ERROR ESTIMATION (§84.7)
    // ========================================================================

    pub fn ui_modal(&mut self, ui: &mut egui::Ui) {
        // Top-level tabs: Calculation | Error Estimation (exact `Imagen pegada (44)` & `(52)`)
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(241, 245, 250))
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(
                            self.modal_top_tab == ModalTopTab::Calculation,
                            egui::RichText::new("Calculation").strong(),
                        )
                        .clicked()
                    {
                        self.modal_top_tab = ModalTopTab::Calculation;
                    }
                    if ui
                        .selectable_label(
                            self.modal_top_tab == ModalTopTab::ErrorEstimation,
                            egui::RichText::new("Error Estimation (Monte Carlo)").strong(),
                        )
                        .clicked()
                    {
                        self.modal_top_tab = ModalTopTab::ErrorEstimation;
                    }

                    ui.separator();
                    if self.modal_top_tab == ModalTopTab::Calculation {
                        for (t, lbl) in [
                            (ModalCalcTab::InputData, "1. Input Data"),
                            (ModalCalcTab::Rules, "2. Rules"),
                            (ModalCalcTab::Results, "3. Results"),
                        ] {
                            if ui.selectable_label(self.modal_calc_tab == t, lbl).clicked() {
                                self.modal_calc_tab = t;
                            }
                        }
                        ui.separator();
                        ui.checkbox(&mut self.modal.normalize_total, "Normalize 100%");
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("▶ Calculate").strong().color(egui::Color32::WHITE))
                                    .fill(egui::Color32::from_rgb(22, 163, 74)),
                            )
                            .clicked()
                        {
                            self.modal.calculate();
                            self.modal_calc_tab = ModalCalcTab::Results;
                            self.status_msg = Some(format!(
                                "Modal calculation completed for {} samples ({} minerals, {} rounds).",
                                self.modal.samples.len(),
                                self.modal.minerals.len(),
                                self.modal.rounds.len()
                            ));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("💾 Export .xlsx").clicked() {
                            self.export_path = "/tmp/hsc_geo_modal.xlsx".into();
                            self.export_modal = Geo84ExportKind::ModalXlsx;
                        }
                        if ui.button("📖 Cu-S Tutorial").clicked() {
                            self.modal = ModalProject::default();
                            self.modal.run_monte_carlo(42);
                            self.modal_active_round = 0;
                            self.status_msg = Some("Loaded §84.7 Cu-S Modal Tutorial (Ccp + Py + Qtz).".into());
                        }
                    });
                });
            });

        ui.add_space(4.0);
        match self.modal_top_tab {
            ModalTopTab::Calculation => match self.modal_calc_tab {
                ModalCalcTab::InputData => self.ui_modal_input_tab(ui),
                ModalCalcTab::Rules => self.ui_modal_rules_tab(ui),
                ModalCalcTab::Results => self.ui_modal_results_tab(ui),
            },
            ModalTopTab::ErrorEstimation => self.ui_modal_error_estimation_tab(ui),
        }
    }

    fn ui_modal_input_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("➕ Add Column (Elements / LOI / XRD)").clicked() {
                self.modal_show_add_col_modal = true;
            }
            if ui.button("➕ Add Sample").clicked() {
                let next_id = format!("{}", self.modal.samples.len() + 1);
                let values = self.modal.columns.iter().map(|c| (c.clone(), 0.0)).collect();
                self.modal.samples.push(soryntel_thermo::geo_mineralogy::ModalSampleRow { id: next_id, values });
                self.modal.regenerate_cv_tables();
            }
            if ui
                .add_enabled(self.modal.samples.len() > 1, egui::Button::new("➖ Remove Last Sample"))
                .clicked()
            {
                self.modal.samples.pop();
                self.modal.regenerate_cv_tables();
            }
        });
        ui.add_space(4.0);

        let mut remove_col: Option<String> = None;
        egui::ScrollArea::both().id_salt("modal_in_grid").show(ui, |ui| {
            ui.horizontal(|ui| {
                row_num_cell(ui, 1, 28.0);
                header_cell(ui, "Id", 80.0);
                for col in &self.modal.columns {
                    widget_cell(ui, 86.0, egui::Color32::from_rgb(232, 238, 247), false, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{col} %")).strong().size(10.5));
                            if self.modal.columns.len() > 1 && ui.small_button("✖").clicked() {
                                remove_col = Some(col.clone());
                            }
                        });
                    });
                }
            });
            for (si, samp) in self.modal.samples.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    row_num_cell(ui, si + 2, 28.0);
                    widget_cell(ui, 80.0, egui::Color32::WHITE, false, |ui| {
                        ui.text_edit_singleline(&mut samp.id);
                    });
                    for col in &self.modal.columns {
                        let entry = samp.values.entry(col.clone()).or_insert(0.0);
                        widget_cell(ui, 86.0, egui::Color32::WHITE, false, |ui| {
                            ui.add(egui::DragValue::new(entry).speed(0.05).range(0.0..=100.0));
                        });
                    }
                });
            }
        });
        if let Some(c) = remove_col {
            self.modal.remove_column(&c);
        }
    }

    fn ui_modal_rules_tab(&mut self, ui: &mut egui::Ui) {
        if self.modal.rounds.is_empty() {
            self.modal.rounds.push(ModalRound {
                name: "Round 1".into(),
                mineral_indices: Vec::new(),
                elements: Vec::new(),
                is_remainder: false,
                method: ModalSolveMethod::NonNegativeLeastSquares,
            });
        }
        if self.modal_active_round >= self.modal.rounds.len() {
            self.modal_active_round = self.modal.rounds.len() - 1;
        }

        let input_cols: BTreeSet<String> = self.modal.columns.iter().cloned().collect();
        let active_round_els: BTreeSet<String> = self.modal.rounds[self.modal_active_round]
            .elements
            .iter()
            .cloned()
            .collect();

        // Top section: Periodic Table (RED = active Round element!) + Element | Method Accepted list
        ui.horizontal_top(|ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(248, 250, 252))
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(4.0))
                .show(ui, |ui| {
                    render_periodic_table_grid(
                        ui,
                        "modal_rules_ptable",
                        28.0,
                        19.0,
                        |sym| {
                            if active_round_els.contains(sym) {
                                (egui::Color32::from_rgb(220, 38, 38), egui::Color32::WHITE)
                            } else if input_cols.contains(sym) {
                                (egui::Color32::from_rgb(59, 130, 246), egui::Color32::WHITE)
                            } else {
                                (egui::Color32::from_rgb(130, 136, 145), egui::Color32::WHITE)
                            }
                        },
                        |sym| {
                            if input_cols.contains(sym) {
                                let r = &mut self.modal.rounds[self.modal_active_round];
                                r.is_remainder = false;
                                if let Some(pos) = r.elements.iter().position(|e| e == sym) {
                                    r.elements.remove(pos);
                                } else {
                                    r.elements.push(sym.to_string());
                                }
                            }
                        },
                    );
                });

            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_width(180.0);
                    ui.strong("Available Input Columns");
                    ui.label(egui::RichText::new("Click to toggle in active Round:").size(9.5).color(egui::Color32::GRAY));
                    for col in self.modal.columns.clone() {
                        let used = self.modal.rounds[self.modal_active_round].elements.contains(&col);
                        if ui.selectable_label(used, format!("{col} %")).clicked() {
                            let r = &mut self.modal.rounds[self.modal_active_round];
                            r.is_remainder = false;
                            if used {
                                r.elements.retain(|e| e != &col);
                            } else {
                                r.elements.push(col);
                            }
                        }
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        for extra in ["LOI", "Sat", "H2O"] {
                            let has = self.modal.columns.iter().any(|c| c == extra);
                            if ui.selectable_label(has, extra).clicked() {
                                if has {
                                    self.modal.remove_column(extra);
                                } else {
                                    self.modal.add_column(extra.to_string());
                                }
                            }
                        }
                    });
                });
        });

        ui.add_space(6.0);

        // Bottom section: Minerals list (left) | Transfer buttons | Phases & Elements per Rounds (right)
        let avail_w = ui.available_width();
        let left_w = 240.0f32;
        let right_w = (avail_w - left_w - 62.0).max(320.0);

        ui.horizontal_top(|ui| {
            // Left: Minerals
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(left_w, 250.0));
                    ui.set_max_size(egui::vec2(left_w, 250.0));
                    ui.horizontal(|ui| {
                        ui.strong("Minerals");
                        if ui.button("🔍 Select Minerals").clicked() {
                            self.modal_show_select_min_modal = true;
                        }
                    });
                    egui::ScrollArea::vertical().id_salt("modal_min_list").show(ui, |ui| {
                        for (mi, m) in self.modal.minerals.iter().enumerate() {
                            let sel = self.modal_selected_mineral_list == Some(mi);
                            if ui.selectable_label(sel, m.display_name()).clicked() {
                                self.modal_selected_mineral_list = Some(mi);
                            }
                        }
                    });
                });

            // Middle transfer buttons (`>`, `>>`, `<`, `<<`, `-`) matching `Imagen pegada (44).png`!
            ui.vertical(|ui| {
                ui.add_space(28.0);
                if ui.add_sized([44.0, 22.0], egui::Button::new("▶")).on_hover_text("Add selected mineral to active Round").clicked() {
                    if let Some(mi) = self.modal_selected_mineral_list {
                        let r = &mut self.modal.rounds[self.modal_active_round];
                        if !r.mineral_indices.contains(&mi) {
                            r.mineral_indices.push(mi);
                        }
                    }
                }
                if ui.add_sized([44.0, 22.0], egui::Button::new("⏭")).on_hover_text("Add all minerals to active Round").clicked() {
                    let r = &mut self.modal.rounds[self.modal_active_round];
                    r.mineral_indices = (0..self.modal.minerals.len()).collect();
                }
                if ui.add_sized([44.0, 22.0], egui::Button::new("◀")).on_hover_text("Remove selected mineral from active Round").clicked() {
                    if let Some(mi) = self.modal_selected_mineral_list {
                        self.modal.rounds[self.modal_active_round]
                            .mineral_indices
                            .retain(|&x| x != mi);
                    }
                }
                if ui.add_sized([44.0, 22.0], egui::Button::new("⏮")).on_hover_text("Clear active Round phases").clicked() {
                    self.modal.rounds[self.modal_active_round].mineral_indices.clear();
                }
                if ui.add_sized([44.0, 22.0], egui::Button::new("🗑")).on_hover_text("Remove selected mineral from project").clicked() {
                    if let Some(mi) = self.modal_selected_mineral_list {
                        self.modal.remove_mineral(mi);
                        self.modal_selected_mineral_list = if self.modal.minerals.is_empty() { None } else { Some(0) };
                    }
                }
            });

            // Right: Phases and Elements per Rounds
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(right_w, 250.0));
                    ui.horizontal(|ui| {
                        ui.strong("Phases and Elements per Rounds");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("↺ Reset").clicked() {
                                self.modal.rounds = vec![ModalRound {
                                    name: "Round 1".into(),
                                    mineral_indices: Vec::new(),
                                    elements: Vec::new(),
                                    is_remainder: false,
                                    method: ModalSolveMethod::NonNegativeLeastSquares,
                                }];
                                self.modal_active_round = 0;
                            }
                            if ui
                                .add_enabled(self.modal.rounds.len() > 1, egui::Button::new("➖ Remove Round").small())
                                .clicked()
                            {
                                self.modal.rounds.remove(self.modal_active_round);
                                self.modal_active_round = self.modal_active_round.min(self.modal.rounds.len() - 1);
                            }
                            if ui.small_button("➕ Add Round").clicked() {
                                let n = self.modal.rounds.len() + 1;
                                self.modal.rounds.push(ModalRound {
                                    name: format!("Round {n}"),
                                    mineral_indices: Vec::new(),
                                    elements: Vec::new(),
                                    is_remainder: false,
                                    method: ModalSolveMethod::NonNegativeLeastSquares,
                                });
                                self.modal_active_round = self.modal.rounds.len() - 1;
                            }
                        });
                    });

                    // Round selector tabs
                    ui.horizontal(|ui| {
                        for (ri, r) in self.modal.rounds.iter().enumerate() {
                            if ui.selectable_label(self.modal_active_round == ri, &r.name).clicked() {
                                self.modal_active_round = ri;
                            }
                        }
                    });
                    ui.separator();

                    let round = &mut self.modal.rounds[self.modal_active_round];
                    let det_label = round.determination_label();

                    ui.horizontal_top(|ui| {
                        // Phases in Round
                        ui.vertical(|ui| {
                            ui.set_min_width(185.0);
                            ui.label(egui::RichText::new("Phases").strong().size(10.5));
                            let mut rm_phase = None;
                            for (pi, &mi) in round.mineral_indices.iter().enumerate() {
                                if let Some(m) = self.modal.minerals.get(mi) {
                                    ui.horizontal(|ui| {
                                        ui.label(m.display_name());
                                        if ui.small_button("✖").clicked() {
                                            rm_phase = Some(pi);
                                        }
                                    });
                                }
                            }
                            if let Some(pi) = rm_phase {
                                round.mineral_indices.remove(pi);
                            }
                        });
                        ui.separator();
                        // Elements / Sum=100% in Round
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Elements").strong().size(10.5));
                                if ui
                                    .selectable_label(round.is_remainder, "Σ  Set Remaining (Sum = 100%)")
                                    .clicked()
                                {
                                    round.is_remainder = !round.is_remainder;
                                }
                            });
                            if round.is_remainder {
                                ui.colored_label(
                                    egui::Color32::from_rgb(29, 78, 216),
                                    "Sum = 100%  (Remaining = 100 − other phases)",
                                );
                            } else {
                                let mut rm_el = None;
                                for (ei, el) in round.elements.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{el} %"));
                                        if ui.small_button("✖").clicked() {
                                            rm_el = Some(ei);
                                        }
                                    });
                                }
                                if let Some(ei) = rm_el {
                                    round.elements.remove(ei);
                                }
                            }
                        });
                    });

                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Method:").strong().size(10.5));
                        for m in ModalSolveMethod::ALL {
                            ui.radio_value(&mut round.method, m, m.label());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(239, 246, 255))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(147, 197, 253)))
                                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(det_label).strong().size(10.5));
                                });
                        });
                    });
                });
        });
    }

    fn ui_modal_results_tab(&mut self, ui: &mut egui::Ui) {
        if self.modal.results.is_empty() {
            ui.label("No results yet. Press '▶ Calculate'.");
            return;
        }
        // All 7 HSC Geo Modal Result sub-tabs (§84.7.1)
        ui.horizontal_wrapped(|ui| {
            for (t, lbl) in [
                (ModalResultSubTab::Modal, "Modal"),
                (ModalResultSubTab::Distribution, "Distribution"),
                (ModalResultSubTab::Fraction, "Fraction"),
                (ModalResultSubTab::Residual, "Residual"),
                (ModalResultSubTab::BulkChem, "Bulk Ch."),
                (ModalResultSubTab::NotesStats, "Notes / Statistics"),
                (ModalResultSubTab::InitialValues, "Initial values"),
            ] {
                if ui.selectable_label(self.modal_res_tab == t, egui::RichText::new(lbl).strong()).clicked() {
                    self.modal_res_tab = t;
                }
            }
        });
        ui.separator();

        match self.modal_res_tab {
            ModalResultSubTab::Modal => {
                egui::ScrollArea::both().id_salt("mres_modal").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        for m in &self.modal.minerals {
                            header_cell(ui, &format!("{} %", m.name), 104.0);
                        }
                        header_cell(ui, "Total %", 74.0);
                        header_cell(ui, "SG", 66.0);
                    });
                    for r in &self.modal.results {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| { ui.label(&r.sample_id); });
                            for &v in &r.mineral_wt {
                                widget_cell(ui, 104.0, egui::Color32::WHITE, false, |ui| {
                                    ui.label(egui::RichText::new(format!("{v:.3}")).monospace());
                                });
                            }
                            widget_cell(ui, 74.0, egui::Color32::from_rgb(240, 253, 244), false, |ui| {
                                ui.label(egui::RichText::new(format!("{:.2}", r.total_after_norm)).monospace().strong());
                            });
                            widget_cell(ui, 66.0, egui::Color32::from_rgb(239, 246, 255), false, |ui| {
                                ui.label(egui::RichText::new(format!("{:.3}", r.sg)).monospace().strong());
                            });
                        });
                    }
                });
            }
            ModalResultSubTab::Distribution => {
                let mut all_els: BTreeSet<String> = self.modal.columns.iter().cloned().collect();
                for m in &self.modal.minerals {
                    all_els.extend(m.composition.keys().cloned());
                }
                let els: Vec<String> = all_els.into_iter().collect();
                if !els.contains(&self.modal_dist_element) && !els.is_empty() {
                    self.modal_dist_element = els[0].clone();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Select Element for Deportment / Distribution (%):");
                    for e in &els {
                        if ui.selectable_label(&self.modal_dist_element == e, e).clicked() {
                            self.modal_dist_element = e.clone();
                        }
                    }
                });
                let dist = self.modal.element_distribution(&self.modal_dist_element);
                egui::ScrollArea::both().id_salt("mres_dist").max_height(175.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        for m in &self.modal.minerals {
                            header_cell(ui, &format!("{} %", m.name), 104.0);
                        }
                    });
                    for (si, row) in dist.iter().enumerate() {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| {
                                ui.label(&self.modal.results[si].sample_id);
                            });
                            for &v in row {
                                widget_cell(ui, 104.0, egui::Color32::WHITE, false, |ui| {
                                    ui.label(egui::RichText::new(format!("{v:.2}%")).monospace());
                                });
                            }
                        });
                    }
                });
                ui.add_space(6.0);
                let sample_labels: Vec<String> = self.modal.results.iter().map(|r| r.sample_id.clone()).collect();
                let min_labels: Vec<String> = self.modal.minerals.iter().map(|m| m.name.clone()).collect();
                draw_stacked_distribution_chart(
                    ui,
                    &format!("{} Distribution Across Minerals (%)", self.modal_dist_element),
                    &sample_labels,
                    &min_labels,
                    &dist,
                );
            }
            ModalResultSubTab::Fraction => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Select Phases to Normalize to 100%:");
                    for (mi, m) in self.modal.minerals.iter().enumerate() {
                        let mut chk = self.modal.fraction_selected_minerals.contains(&mi);
                        if ui.checkbox(&mut chk, &m.name).changed() {
                            if chk {
                                self.modal.fraction_selected_minerals.insert(mi);
                            } else {
                                self.modal.fraction_selected_minerals.remove(&mi);
                            }
                        }
                    }
                });
                let sel: Vec<usize> = if self.modal.fraction_selected_minerals.is_empty() {
                    (0..self.modal.minerals.len()).collect()
                } else {
                    self.modal.fraction_selected_minerals.iter().copied().collect()
                };
                let frac = self.modal.calculate_fraction();
                egui::ScrollArea::both().id_salt("mres_frac").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        for &mi in &sel {
                            header_cell(ui, &format!("{} %", self.modal.minerals[mi].name), 104.0);
                        }
                        header_cell(ui, "Sum %", 70.0);
                    });
                    for (sid, vals) in frac {
                        let s: f64 = vals.iter().sum();
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| { ui.label(&sid); });
                            for v in vals {
                                widget_cell(ui, 104.0, egui::Color32::WHITE, false, |ui| {
                                    ui.label(egui::RichText::new(format!("{v:.2}")).monospace());
                                });
                            }
                            widget_cell(ui, 70.0, egui::Color32::from_rgb(240, 253, 244), false, |ui| {
                                ui.label(egui::RichText::new(format!("{s:.2}")).monospace().strong());
                            });
                        });
                    }
                });
            }
            ModalResultSubTab::Residual => {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.modal_residual_relative, false, "Absolute Residual (Calc − Obs, wt%)");
                    ui.radio_value(&mut self.modal_residual_relative, true, "Relative Residual (%)");
                });
                egui::ScrollArea::both().id_salt("mres_resid").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        for c in &self.modal.columns {
                            header_cell(ui, c, 86.0);
                        }
                        header_cell(ui, "RMSE", 74.0);
                    });
                    for r in &self.modal.results {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| { ui.label(&r.sample_id); });
                            for c in &self.modal.columns {
                                let v = if self.modal_residual_relative {
                                    r.residual_rel_pct.get(c).copied().unwrap_or(0.0)
                                } else {
                                    r.residual_abs.get(c).copied().unwrap_or(0.0)
                                };
                                let bg = if v.abs() > (if self.modal_residual_relative { 5.0 } else { 0.25 }) {
                                    egui::Color32::from_rgb(254, 226, 226)
                                } else {
                                    egui::Color32::from_rgb(240, 253, 244)
                                };
                                widget_cell(ui, 86.0, bg, false, |ui| {
                                    ui.label(egui::RichText::new(format!("{v:+.4}")).monospace());
                                });
                            }
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| {
                                ui.label(egui::RichText::new(format!("{:.4}", r.rmse)).monospace());
                            });
                        });
                    }
                });
            }
            ModalResultSubTab::BulkChem => {
                let els: Vec<String> = self
                    .modal
                    .results
                    .first()
                    .map(|r| r.bulk_calc.keys().cloned().collect())
                    .unwrap_or_default();
                egui::ScrollArea::both().id_salt("mres_bulk").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        for e in &els {
                            header_cell(ui, &format!("{e} %"), 72.0);
                        }
                    });
                    for r in &self.modal.results {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| { ui.label(&r.sample_id); });
                            for e in &els {
                                let v = r.bulk_calc.get(e).copied().unwrap_or(0.0);
                                widget_cell(ui, 72.0, egui::Color32::WHITE, false, |ui| {
                                    ui.label(egui::RichText::new(format!("{v:.3}")).monospace());
                                });
                            }
                        });
                    }
                });
            }
            ModalResultSubTab::NotesStats => {
                egui::ScrollArea::both().id_salt("mres_notes").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Sample", 74.0);
                        header_cell(ui, "Total Raw %", 90.0);
                        header_cell(ui, "Total Final %", 90.0);
                        header_cell(ui, "SG", 70.0);
                        header_cell(ui, "RMSE", 76.0);
                        header_cell(ui, "Status / Note", 240.0);
                    });
                    for r in &self.modal.results {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::WHITE, false, |ui| { ui.label(&r.sample_id); });
                            widget_cell(ui, 90.0, egui::Color32::WHITE, false, |ui| { ui.label(format!("{:.3}", r.total_before_norm)); });
                            widget_cell(ui, 90.0, egui::Color32::WHITE, false, |ui| { ui.label(format!("{:.3}", r.total_after_norm)); });
                            widget_cell(ui, 70.0, egui::Color32::WHITE, false, |ui| { ui.label(format!("{:.3}", r.sg)); });
                            widget_cell(ui, 76.0, egui::Color32::WHITE, false, |ui| { ui.label(format!("{:.4}", r.rmse)); });
                            widget_cell(ui, 240.0, egui::Color32::WHITE, false, |ui| { ui.label(&r.note); });
                        });
                    }
                });
            }
            ModalResultSubTab::InitialValues => {
                ui.strong("Mineral Composition Matrix Used (wt%)");
                let els: Vec<String> = self.modal.mineral_rsd_elements.clone();
                egui::ScrollArea::both().id_salt("mres_init").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        header_cell(ui, "Element", 74.0);
                        for m in &self.modal.minerals {
                            header_cell(ui, &m.name, 102.0);
                        }
                    });
                    for el in &els {
                        ui.horizontal(|ui| {
                            widget_cell(ui, 74.0, egui::Color32::from_rgb(241, 245, 249), false, |ui| { ui.strong(el); });
                            for m in &self.modal.minerals {
                                let v = m.composition.get(el).copied().unwrap_or(0.0);
                                widget_cell(ui, 102.0, egui::Color32::WHITE, false, |ui| {
                                    ui.label(egui::RichText::new(if v > 0.0 { format!("{v:.3}") } else { String::new() }).monospace());
                                });
                            }
                        });
                    }
                });
            }
        }
    }

    fn ui_modal_error_estimation_tab(&mut self, ui: &mut egui::Ui) {
        // Exact `Imagen pegada (52).png` Error Estimation (Monte Carlo)!
        ui.horizontal(|ui| {
            ui.label("Calculation Rounds:");
            ui.add(egui::DragValue::new(&mut self.modal.mc_rounds).range(5..=2000));
            if ui.button("🔄 Regenerate CV-tables").clicked() {
                self.modal.regenerate_cv_tables();
                self.status_msg = Some("Regenerated sample (4% RSD) & mineral (1% RSD) CV-tables.".into());
            }
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶ Run Monte Carlo simulation")
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(37, 99, 235)),
                )
                .clicked()
            {
                self.modal_mc_seed = self.modal_mc_seed.wrapping_add(17);
                self.modal.run_monte_carlo(self.modal_mc_seed);
                self.status_msg = Some(format!(
                    "Monte Carlo simulation completed ({} iterations).",
                    self.modal.mc_rounds
                ));
            }
        });
        ui.add_space(4.0);

        let avail_w = ui.available_width();
        let left_w = (avail_w * 0.44).max(300.0);
        let right_w = (avail_w - left_w - 10.0).max(320.0);

        ui.horizontal_top(|ui| {
            // Left: Top Sheet (Sample Assay RSD %) + Bottom Sheet (Mineral Microprobe RSD %)
            ui.vertical(|ui| {
                ui.set_min_width(left_w);
                ui.set_max_width(left_w);
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(5.0))
                    .show(ui, |ui| {
                        ui.strong("Sample Chemical Analysis RSD (%)");
                        egui::ScrollArea::both().id_salt("mc_samp_rsd").max_height(155.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Id", 56.0);
                                for c in &self.modal.columns {
                                    header_cell(ui, &format!("{c} %"), 68.0);
                                }
                            });
                            for (si, s) in self.modal.samples.iter().enumerate() {
                                if si >= self.modal.sample_rsd.len() {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 56.0, egui::Color32::WHITE, false, |ui| { ui.label(&s.id); });
                                    for ci in 0..self.modal.columns.len() {
                                        if let Some(v) = self.modal.sample_rsd[si].get_mut(ci) {
                                            widget_cell(ui, 68.0, egui::Color32::WHITE, false, |ui| {
                                                ui.add(egui::DragValue::new(v).speed(0.1).range(0.0..=50.0));
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    });

                ui.add_space(6.0);
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(5.0))
                    .show(ui, |ui| {
                        ui.strong("Mineral Composition RSD (%)");
                        egui::ScrollArea::both().id_salt("mc_min_rsd").max_height(185.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Element", 64.0);
                                for m in &self.modal.minerals {
                                    header_cell(ui, &m.name, 88.0);
                                }
                            });
                            for (ei, el) in self.modal.mineral_rsd_elements.iter().enumerate() {
                                if ei >= self.modal.mineral_rsd.len() {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 64.0, egui::Color32::WHITE, false, |ui| { ui.label(el); });
                                    for mi in 0..self.modal.minerals.len() {
                                        if let Some(v) = self.modal.mineral_rsd[ei].get_mut(mi) {
                                            widget_cell(ui, 88.0, egui::Color32::WHITE, false, |ui| {
                                                ui.add(egui::DragValue::new(v).speed(0.1).range(0.0..=50.0));
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    });
            });

            // Right: 5 Monte Carlo Output Tabs (§84.7.2)
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_min_width(right_w);
                    ui.horizontal_wrapped(|ui| {
                        for (t, lbl) in [
                            (McResultSubTab::AvgSigma, "Average ± 1 Sigma"),
                            (McResultSubTab::Average, "Average"),
                            (McResultSubTab::StdDev, "StdDev"),
                            (McResultSubTab::MinMax, "Min-Max"),
                            (McResultSubTab::Cv, "CV (%)"),
                        ] {
                            if ui.selectable_label(self.modal_mc_tab == t, egui::RichText::new(lbl).strong()).clicked() {
                                self.modal_mc_tab = t;
                            }
                        }
                    });
                    ui.separator();

                    if let Some(mc) = &self.modal.mc_result {
                        ui.label(
                            egui::RichText::new(format!("Monte Carlo Results ({} iterations)", mc.iterations))
                                .size(10.5)
                                .color(egui::Color32::GRAY),
                        );
                        egui::ScrollArea::both().id_salt("mc_res_grid").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Sample", 64.0);
                                for m in &mc.mineral_names {
                                    header_cell(ui, m, 118.0);
                                }
                            });
                            for (si, sid) in mc.sample_ids.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    widget_cell(ui, 64.0, egui::Color32::WHITE, false, |ui| { ui.label(sid); });
                                    for mi in 0..mc.mineral_names.len() {
                                        let txt = match self.modal_mc_tab {
                                            McResultSubTab::AvgSigma => {
                                                format!("{:.2} ± {:.2}", mc.mean[si][mi], mc.stddev[si][mi])
                                            }
                                            McResultSubTab::Average => format!("{:.3}", mc.mean[si][mi]),
                                            McResultSubTab::StdDev => format!("{:.3}", mc.stddev[si][mi]),
                                            McResultSubTab::MinMax => {
                                                format!("{:.2} – {:.2}", mc.min_v[si][mi], mc.max_v[si][mi])
                                            }
                                            McResultSubTab::Cv => format!("{:.2}%", mc.cv_pct[si][mi]),
                                        };
                                        widget_cell(ui, 118.0, egui::Color32::WHITE, false, |ui| {
                                            ui.label(egui::RichText::new(txt).monospace());
                                        });
                                    }
                                });
                            }
                        });
                    } else {
                        ui.label("Click 'Run Monte Carlo simulation' to compute uncertainty bounds.");
                    }
                });
        });
    }

    // ========================================================================
    // Modals (Paste Data, Add Record, Add Composition Row, Import Petro Sheet,
    // Modal Add Column, Modal Select Minerals, Excel Export)
    // ========================================================================

    fn ui_db_paste_modal(&mut self, ctx: &egui::Context) {
        if !self.db_show_paste_modal {
            return;
        }
        let mut open = self.db_show_paste_modal;
        let mut do_paste = false;
        egui::Window::new(format!("Paste Data -> {}", self.db_source.label()))
            .open(&mut open)
            .default_size([520.0, 300.0])
            .show(ctx, |ui| {
                ui.label("Paste tab- or comma-separated table with header row:");
                egui::ScrollArea::vertical().max_height(210.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.db_paste_buf)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace),
                    );
                });
                ui.horizontal(|ui| {
                    if ui.button("📥 Import / Paste").clicked() {
                        do_paste = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.db_show_paste_modal = false;
                    }
                });
            });
        if do_paste {
            let rep = self.db.paste_tsv(self.db_source, &self.db_paste_buf);
            self.db_paste_report = Some(rep);
            open = false;
        }
        self.db_show_paste_modal = open;
    }

    fn ui_db_paste_report_dialog(&mut self, ctx: &egui::Context) {
        let Some(rep) = self.db_paste_report.clone() else {
            return;
        };
        let mut open = true;
        // Exact Image 11 modal: "HSC Geo — Columns identified: 7 of 9 / Rows added: 2 of 2 [OK]"
        egui::Window::new("HSC Geo")
            .id(egui::Id::new("hsc_geo_paste_report_dlg"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ℹ").size(28.0).color(egui::Color32::from_rgb(37, 99, 235)));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Columns identified: {} of {}",
                                rep.identified_cols, rep.total_cols
                            ))
                            .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Rows added: {} of {}",
                                rep.added_rows, rep.total_rows
                            ))
                            .strong(),
                        );
                        for n in rep.notes.iter().skip(1) {
                            ui.label(egui::RichText::new(n).size(10.0).color(egui::Color32::from_rgb(180, 83, 9)));
                        }
                    });
                });
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    if ui.add_sized([76.0, 24.0], egui::Button::new("OK")).clicked() {
                        self.db_paste_report = None;
                    }
                });
            });
        if !open {
            self.db_paste_report = None;
        }
    }

    fn ui_db_confirm_delete_modal(&mut self, ctx: &egui::Context) {
        if !self.db_confirm_delete {
            return;
        }
        let mut open = true;
        // Exact Image 9 modal: "HSC Geo — Are you sure? [Yes] [No] [Cancel]"
        egui::Window::new("HSC Geo")
            .id(egui::Id::new("hsc_geo_confirm_del_dlg"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("❓").size(26.0).color(egui::Color32::from_rgb(37, 99, 235)));
                    ui.label(
                        egui::RichText::new(format!(
                            "Are you sure? ({} row(s) selected)",
                            self.db_selected_rows.len()
                        ))
                        .strong(),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_sized([72.0, 24.0], egui::Button::new("Yes")).clicked() {
                        self.remove_selected_db_rows();
                        self.db_confirm_delete = false;
                    }
                    if ui.add_sized([72.0, 24.0], egui::Button::new("No")).clicked() {
                        self.db_confirm_delete = false;
                    }
                    if ui.add_sized([72.0, 24.0], egui::Button::new("Cancel")).clicked() {
                        self.db_confirm_delete = false;
                    }
                });
            });
        if !open {
            self.db_confirm_delete = false;
        }
    }

    fn ui_db_about_help_modals(&mut self, ctx: &egui::Context) {
        if self.db_show_about {
            let mut open = self.db_show_about;
            egui::Window::new("About HSC Geo")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.strong("Soryntel Chemistry — HSC Geo (Chapters 84 & 85)");
                    ui.label(format!("Embedded Mineral Catalog: {} IMA/HSC minerals", self.db.minerals.len()));
                    ui.label(format!("Multi-Locality Microprobe Analyses: {} records", self.db.mineral_chemistry.len()));
                    ui.label("Tools: Catalogue, Find by Properties, Find by Composition (WSSQ), Database Viewer, Modal + Monte Carlo, Petrological (13 routines), Mineral Liberation.");
                });
            self.db_show_about = open;
        }
        if self.db_show_help {
            let mut open = self.db_show_help;
            egui::Window::new("HSC Geo — Database Viewer Help")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("• Use the inline row below column headers to filter by Contains, Begins with, Equals, >, <.");
                    ui.label("• In the Search box, space-separated words search with OR logic (e.g. 'DHZ Biotite') and highlight matches in yellow.");
                    ui.label("• Press 'View / Edit' to unlock editing (orange rows), 'Add Record', 'Remove Record (s)', and 'Paste Data'.");
                    ui.label("• Select rows (Ctrl+Click for multi-select) and press 'Copy Data' or Ctrl+C to copy TSV.");
                });
            self.db_show_help = open;
        }
    }

    fn ui_db_add_modal(&mut self, ctx: &egui::Context) {
        if !self.db_show_add_modal {
            return;
        }
        let mut open = self.db_show_add_modal;
        let mut do_add = false;
        // Exact Image 10 modal: Source, Mineral Name (auto-fills Symbol), Property table + Add Field / Add Record / Cancel
        egui::Window::new("Add Record")
            .open(&mut open)
            .collapsible(false)
            .default_size([360.0, 290.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    egui::ComboBox::from_id_salt("add_rec_src")
                        .selected_text(self.db_add_source.label())
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for src in [
                                GeoDbSource::MineralChemistry,
                                GeoDbSource::PhaseChemistry,
                                GeoDbSource::Mineral,
                                GeoDbSource::MineralDissolution,
                            ] {
                                ui.selectable_value(&mut self.db_add_source, src, src.label());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Mineral Name:");
                    ui.add_sized([220.0, 20.0], egui::TextEdit::singleline(&mut self.db_add_mineral_name));
                });
                let auto_sym = self.db.lookup_mineral_symbol(&self.db_add_mineral_name);
                ui.label(
                    egui::RichText::new(format!("Auto-filled Mineral Symbol: {auto_sym}"))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(37, 99, 235)),
                );
                ui.separator();

                let mut rm_field = None;
                egui::ScrollArea::vertical().max_height(170.0).show(ui, |ui| {
                    egui::Grid::new("db_add_prop_grid").num_columns(3).striped(true).show(ui, |ui| {
                        ui.label("Mineral ID");
                        ui.label(format!("{} (auto)", self.db.next_chem_id()));
                        ui.label("");
                        ui.end_row();
                        ui.label("Location");
                        ui.text_edit_singleline(&mut self.db_add_location);
                        ui.label("");
                        ui.end_row();
                        ui.label("Mineral Key");
                        ui.text_edit_singleline(&mut self.db_add_mineral_key);
                        ui.label("");
                        ui.end_row();
                        ui.label("Reference");
                        ui.text_edit_singleline(&mut self.db_add_reference);
                        ui.label("");
                        ui.end_row();
                        ui.label("SG");
                        ui.text_edit_singleline(&mut self.db_add_sg);
                        ui.label("");
                        ui.end_row();
                        for (i, (k, v)) in self.db_add_extra_fields.iter_mut().enumerate() {
                            ui.add_sized([90.0, 19.0], egui::TextEdit::singleline(k).hint_text("Element (e.g. Cu)"));
                            ui.add_sized([110.0, 19.0], egui::TextEdit::singleline(v).hint_text("wt%"));
                            if ui.small_button("✖").clicked() {
                                rm_field = Some(i);
                            }
                            ui.end_row();
                        }
                    });
                });
                if let Some(ri) = rm_field {
                    self.db_add_extra_fields.remove(ri);
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("➕ Add Field").clicked() {
                        self.db_add_extra_fields.push(("Cu".into(), "0.0".into()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖ Cancel").clicked() {
                            self.db_show_add_modal = false;
                        }
                        if ui.button("➕ Add Record").clicked() {
                            do_add = true;
                        }
                    });
                });
            });

        if do_add && !self.db_add_mineral_name.trim().is_empty() {
            let mut comp = BTreeMap::new();
            for (k, v) in &self.db_add_extra_fields {
                if let Ok(num) = v.trim().replace(',', ".").parse::<f64>() {
                    if !k.trim().is_empty() && num > 0.0 {
                        comp.insert(k.trim().to_string(), num);
                    }
                }
            }
            let sg_opt = self.db_add_sg.trim().replace(',', ".").parse::<f64>().ok();
            let auto_sym = self.db.lookup_mineral_symbol(&self.db_add_mineral_name);
            match self.db_add_source {
                GeoDbSource::MineralChemistry => {
                    let id = self.db.next_chem_id();
                    self.db.mineral_chemistry.push(MineralChemRecord {
                        id,
                        mineral_symbol: auto_sym,
                        mineral: self.db_add_mineral_name.trim().into(),
                        location: self.db_add_location.trim().into(),
                        mineral_key: self.db_add_mineral_key.trim().into(),
                        statistics: String::new(),
                        reference: self.db_add_reference.trim().into(),
                        sample_no: format!("U-{id}"),
                        note1: String::new(),
                        note2: String::new(),
                        note3: self.db_add_mineral_key.trim().into(),
                        database: "User".into(),
                        sg: sg_opt,
                        values: comp,
                    });
                }
                GeoDbSource::Mineral => {
                    let id = self.db.next_mineral_id();
                    let els: BTreeSet<String> = comp.keys().cloned().collect();
                    let sg = sg_opt.unwrap_or_else(|| {
                        soryntel_thermo::geo_mineralogy::estimate_mineral_sg(&self.db_add_mineral_name, "", &comp)
                    });
                    self.db.minerals.push(GeoMineralRecord {
                        id,
                        name: self.db_add_mineral_name.trim().into(),
                        symbol: auto_sym,
                        formula: self.db_add_mineral_key.trim().into(),
                        empirical_formula: self.db_add_mineral_key.trim().into(),
                        group: self.db_add_location.trim().into(),
                        symbol_ref: "User".into(),
                        note: self.db_add_reference.trim().into(),
                        composition_str: soryntel_thermo::geo_mineralogy::format_composition_map(&comp),
                        composition: comp,
                        elements_str: soryntel_thermo::geo_mineralogy::format_elements_set(&els),
                        elements: els,
                        ore_mineral: String::new(),
                        density: sg,
                        color: String::new(),
                        hardness: String::new(),
                        luster: String::new(),
                        mag_susc: String::new(),
                        formula_base: String::new(),
                        date_of_input: "2026".into(),
                        inputted_by: "User".into(),
                    });
                }
                GeoDbSource::PhaseChemistry => {
                    let id = self.db.next_phase_id();
                    self.db.phase_chemistry.push(soryntel_thermo::geo_mineralogy::PhaseChemRecord {
                        id,
                        phase_name: self.db_add_mineral_name.trim().into(),
                        process_type: self.db_add_location.trim().into(),
                        formula_or_desc: self.db_add_mineral_key.trim().into(),
                        density: sg_opt.unwrap_or(4.0),
                        reference: self.db_add_reference.trim().into(),
                        values: comp,
                    });
                }
                GeoDbSource::MineralDissolution => {
                    let id = self.db.next_diss_id();
                    self.db.mineral_dissolution.push(soryntel_thermo::geo_mineralogy::MineralDissolutionRecord {
                        id,
                        mineral: self.db_add_mineral_name.trim().into(),
                        formula: self.db_add_mineral_key.trim().into(),
                        bromine_methanol: 0.0,
                        citric_acid: 0.0,
                        hcl_10pct: 0.0,
                        aqua_regia: 100.0,
                        h2o2_nh4oac: 0.0,
                        nacn: 0.0,
                        reference: self.db_add_reference.trim().into(),
                        note: self.db_add_location.trim().into(),
                    });
                }
                GeoDbSource::Element => {}
            }
            self.db_source = self.db_add_source;
            self.status_msg = Some(format!(
                "Added record '{}' to {} (auto-filled symbol).",
                self.db_add_mineral_name.trim(),
                self.db_add_source.label()
            ));
            open = false;
        }
        self.db_show_add_modal = open;
    }

    fn ui_comp_add_modal(&mut self, ctx: &egui::Context) {
        if !self.comp_show_add_modal {
            return;
        }
        let mut open = self.comp_show_add_modal;
        let mut ok = false;
        egui::Window::new("Add New Value")
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Select element (or oxide), its value and SD");
                ui.checkbox(&mut self.comp_new_is_oxide, "Oxide input (e.g. SiO2, FeO)");
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(if self.comp_new_is_oxide { "Oxide" } else { "Element" });
                        egui::ComboBox::from_id_salt("comp_add_el")
                            .selected_text(&self.comp_new_species)
                            .width(82.0)
                            .show_ui(ui, |ui| {
                                if self.comp_new_is_oxide {
                                    for &ox in COMMON_OXIDES {
                                        ui.selectable_value(&mut self.comp_new_species, ox.to_string(), ox);
                                    }
                                } else {
                                    for el in PERIODIC_ELEMENTS {
                                        ui.selectable_value(&mut self.comp_new_species, el.symbol.to_string(), format!("{} ({})", el.symbol, el.name));
                                    }
                                }
                            });
                    });
                    ui.vertical(|ui| {
                        ui.label("Proportion");
                        egui::ComboBox::from_id_salt("comp_add_unit")
                            .selected_text(self.comp_new_unit.label())
                            .width(64.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.comp_new_unit, CompUnit::WtPct, "%");
                                ui.selectable_value(&mut self.comp_new_unit, CompUnit::Ppm, "ppm");
                                ui.selectable_value(&mut self.comp_new_unit, CompUnit::Ppb, "ppb");
                            });
                    });
                    ui.vertical(|ui| {
                        ui.label("Value");
                        ui.add(egui::DragValue::new(&mut self.comp_new_val).speed(0.2).range(0.0..=1e6));
                    });
                    ui.vertical(|ui| {
                        ui.label("SD");
                        ui.add(egui::DragValue::new(&mut self.comp_new_sd).speed(0.1).range(0.01..=1e5));
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        ok = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.comp_show_add_modal = false;
                    }
                });
            });
        if ok {
            self.comp_criteria.push(CompCriterion {
                species: self.comp_new_species.clone(),
                is_oxide: self.comp_new_is_oxide,
                unit: self.comp_new_unit,
                value: self.comp_new_val,
                sd: self.comp_new_sd,
            });
            open = false;
        }
        self.comp_show_add_modal = open;
    }

    fn ui_petro_import_modal(&mut self, ctx: &egui::Context) {
        if !self.petro_show_import_modal {
            return;
        }
        let mut open = self.petro_show_import_modal;
        let mut do_import = false;
        egui::Window::new("Import Petrological Data Sheet (CSV / TSV)")
            .open(&mut open)
            .default_size([500.0, 310.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sheet Name:");
                    ui.text_edit_singleline(&mut self.petro_import_name);
                    ui.label("Method Code:");
                    ui.text_edit_singleline(&mut self.petro_import_method);
                });
                egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.petro_import_tsv)
                            .desired_width(f32::INFINITY)
                            .desired_rows(9)
                            .font(egui::TextStyle::Monospace),
                    );
                });
                if ui.button("📥 Import & Identify").clicked() {
                    do_import = true;
                }
            });
        if do_import {
            match self.petro.import_tsv_sheet(
                self.petro_import_name.clone(),
                self.petro_import_method.clone(),
                &self.petro_import_tsv,
            ) {
                Ok(n) => {
                    self.petro_active_sheet = self.petro.sheets.len().saturating_sub(1);
                    self.status_msg = Some(format!("Imported '{}' ({n} rows) & identified columns.", self.petro_import_name));
                    open = false;
                }
                Err(e) => self.status_err = Some(e),
            }
        }
        self.petro_show_import_modal = open;
    }

    fn ui_modal_add_col_modal(&mut self, ctx: &egui::Context) {
        if !self.modal_show_add_col_modal {
            return;
        }
        let mut open = self.modal_show_add_col_modal;
        egui::Window::new("Select Elements / Columns for Modal Input")
            .open(&mut open)
            .default_size([590.0, 320.0])
            .show(ctx, |ui| {
                ui.label("Click elements to add/remove as assay columns:");
                let cols: BTreeSet<String> = self.modal.columns.iter().cloned().collect();
                render_periodic_table_grid(
                    ui,
                    "modal_addcol_ptable",
                    29.0,
                    20.0,
                    |sym| {
                        if cols.contains(sym) {
                            (egui::Color32::from_rgb(37, 99, 235), egui::Color32::WHITE)
                        } else {
                            (egui::Color32::from_rgb(229, 234, 241), egui::Color32::from_rgb(30, 41, 59))
                        }
                    },
                    |sym| {
                        if cols.contains(sym) {
                            if self.modal.columns.len() > 1 {
                                self.modal.remove_column(sym);
                            }
                        } else {
                            self.modal.add_column(sym.to_string());
                        }
                    },
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("Special Columns:");
                    for sp in ["LOI", "Sat", "H2O"] {
                        let has = self.modal.columns.iter().any(|c| c == sp);
                        if ui.selectable_label(has, sp).clicked() {
                            if has {
                                self.modal.remove_column(sp);
                            } else {
                                self.modal.add_column(sp.to_string());
                            }
                        }
                    }
                    ui.separator();
                    ui.strong("XRD Mineral Column:");
                    for m in self.modal.minerals.clone() {
                        let has = self.modal.columns.iter().any(|c| c == &m.name);
                        if ui.selectable_label(has, &m.name).clicked() {
                            if has {
                                self.modal.remove_column(&m.name);
                            } else {
                                self.modal.add_column(m.name);
                            }
                        }
                    }
                });
            });
        self.modal_show_add_col_modal = open;
    }

    fn ui_modal_select_min_modal(&mut self, ctx: &egui::Context) {
        if !self.modal_show_select_min_modal {
            return;
        }
        let mut open = self.modal_show_select_min_modal;
        egui::Window::new("Select Minerals from HSC Geo Database")
            .open(&mut open)
            .default_size([520.0, 380.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add_sized(
                        [280.0, 20.0],
                        egui::TextEdit::singleline(&mut self.modal_min_search)
                            .hint_text("Chalcopyrite, Galena, Sphalerite, Biotite..."),
                    );
                });
                ui.separator();
                let q = self.modal_min_search.trim().to_ascii_lowercase();
                let mut add_def: Option<ModalMineralDef> = None;
                egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
                    let mut count = 0usize;
                    for m in &self.db.minerals {
                        if m.composition.is_empty() {
                            continue;
                        }
                        if !q.is_empty()
                            && !m.name.to_ascii_lowercase().contains(&q)
                            && !m.formula.to_ascii_lowercase().contains(&q)
                        {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("➕ Add").clicked() {
                                add_def = Some(ModalMineralDef {
                                    name: m.name.clone(),
                                    location: "[stoichiometric]".into(),
                                    formula: m.formula.clone(),
                                    sg: m.density,
                                    composition: m.composition.clone(),
                                });
                            }
                            ui.strong(&m.name);
                            ui.label(egui::RichText::new(&m.formula).monospace().size(10.5));
                            ui.label(egui::RichText::new(format!("SG={:.2}", m.density)).size(10.0).color(egui::Color32::GRAY));
                        });
                        count += 1;
                        if count >= 120 {
                            break;
                        }
                    }
                });
                if let Some(d) = add_def {
                    let n = d.name.clone();
                    self.modal.add_mineral(d);
                    self.status_msg = Some(format!("Added '{n}' to Modal phases."));
                }
            });
        self.modal_show_select_min_modal = open;
    }

    fn ui_export_modal(&mut self, ctx: &egui::Context) {
        if self.export_modal == Geo84ExportKind::None {
            return;
        }
        let mut open = true;
        let mut save_now = false;
        let title = match self.export_modal {
            Geo84ExportKind::PetroXlsx => "Save Petrological Results as Excel (.xlsx)",
            Geo84ExportKind::ModalXlsx => "Save Modal Results as Excel (.xlsx)",
            Geo84ExportKind::None => "",
        };
        egui::Window::new(title).open(&mut open).collapsible(false).show(ctx, |ui| {
            ui.label("Output .xlsx file path:");
            ui.add_sized([380.0, 22.0], egui::TextEdit::singleline(&mut self.export_path));
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() {
                    save_now = true;
                }
                if ui.button("Cancel").clicked() {
                    self.export_modal = Geo84ExportKind::None;
                }
            });
        });
        if save_now {
            let bytes = match self.export_modal {
                Geo84ExportKind::PetroXlsx => self.petro.export_results_xlsx(),
                Geo84ExportKind::ModalXlsx => self.modal.export_xlsx(),
                Geo84ExportKind::None => Err("Nothing to export".into()),
            };
            match bytes.and_then(|b| std::fs::write(&self.export_path, b).map_err(|e| e.to_string())) {
                Ok(()) => {
                    self.status_err = None;
                    self.status_msg = Some(format!("Saved Excel workbook to {}.", self.export_path));
                    self.export_modal = Geo84ExportKind::None;
                }
                Err(e) => self.status_err = Some(e),
            }
        }
        if !open {
            self.export_modal = Geo84ExportKind::None;
        }
    }
}

// ============================================================================
// Reusable Widgets & Custom Charts (Periodic Table, Properties, Pie, Variability,
// Scatter, Spidergram, Stacked Distribution Bar Chart)
// ============================================================================

fn hsc_ribbon_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 190))
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
                            .size(9.0)
                            .color(egui::Color32::from_rgb(100, 112, 130)),
                    );
                });
            });
        });
}

fn element_family_tint(sym: &str) -> egui::Color32 {
    let cat = PERIODIC_ELEMENTS
        .iter()
        .find(|e| e.symbol == sym)
        .map(|e| e.category);
    match cat {
        Some(ElementCategory::Alkali) => egui::Color32::from_rgb(245, 190, 125),
        Some(ElementCategory::Alkaline) => egui::Color32::from_rgb(248, 245, 175),
        Some(ElementCategory::TransitionMetal) => egui::Color32::from_rgb(245, 218, 218),
        Some(ElementCategory::PostTransitionMetal) => egui::Color32::from_rgb(165, 196, 245),
        Some(ElementCategory::Metalloid) => egui::Color32::from_rgb(168, 226, 185),
        Some(ElementCategory::NonMetal) => egui::Color32::from_rgb(198, 188, 242),
        Some(ElementCategory::Halogen) => egui::Color32::from_rgb(218, 216, 172),
        Some(ElementCategory::NobleGas) => egui::Color32::from_rgb(155, 212, 248),
        Some(ElementCategory::Lanthanide) => egui::Color32::from_rgb(248, 216, 178),
        Some(ElementCategory::Actinide) => egui::Color32::from_rgb(210, 220, 238),
        None => egui::Color32::from_rgb(229, 234, 241),
    }
}

fn legend_chip(ui: &mut egui::Ui, col: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, col);
        ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(0.8, egui::Color32::from_gray(120)));
        ui.label(egui::RichText::new(label).size(10.0));
    });
}

fn render_periodic_table_grid(
    ui: &mut egui::Ui,
    id: &str,
    cell_w: f32,
    cell_h: f32,
    mut color_fn: impl FnMut(&str) -> (egui::Color32, egui::Color32),
    mut on_click: impl FnMut(&str),
) {
    let mut grid: [[Option<&'static str>; 18]; 9] = [[None; 18]; 9];
    for e in PERIODIC_ELEMENTS {
        if e.grid_row < 9 && e.grid_col < 18 {
            grid[e.grid_row][e.grid_col] = Some(e.symbol);
        }
    }
    ui.push_id(id, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
        for (r, row) in grid.iter().enumerate() {
            if r == 7 {
                ui.add_space(3.0);
            }
            ui.horizontal(|ui| {
                for sym_opt in row {
                    if let Some(sym) = sym_opt {
                        let (bg, fg) = color_fn(sym);
                        let btn = egui::Button::new(egui::RichText::new(*sym).size(10.0).strong().color(fg))
                            .fill(bg)
                            .min_size(egui::vec2(cell_w, cell_h))
                            .rounding(2.0);
                        if ui.add(btn).clicked() {
                            on_click(sym);
                        }
                    } else {
                        ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::hover());
                    }
                }
            });
        }
    });
}

fn render_general_properties_table(ui: &mut egui::Ui, m: &GeoMineralRecord, total_w: f32) {
    let k_w = (total_w * 0.44).max(76.0);
    let v_w = (total_w - k_w - 6.0).max(80.0);
    let rows = [
        ("Name", m.name.as_str()),
        ("Symbol", m.symbol.as_str()),
        ("Formula", m.formula.as_str()),
        ("Empirical", m.empirical_formula.as_str()),
        ("Group", m.group.as_str()),
        ("Symbol Ref", m.symbol_ref.as_str()),
        ("Note", m.note.as_str()),
        ("Composition", m.composition_str.as_str()),
        ("Elements", m.elements_str.as_str()),
        ("Ore Mineral", m.ore_mineral.as_str()),
        ("Color", m.color.as_str()),
        ("Hardness", m.hardness.as_str()),
        ("Luster", m.luster.as_str()),
        ("Date Of Input", m.date_of_input.as_str()),
        ("Inputted By", m.inputted_by.as_str()),
    ];
    let density_str = format!("{:.2}", m.density);
    egui::ScrollArea::vertical().id_salt(format!("gp_{}", m.id)).show(ui, |ui| {
        for (i, (k, v)) in rows.iter().enumerate() {
            if i == 10 {
                ui.horizontal(|ui| {
                    widget_cell(ui, k_w, egui::Color32::from_rgb(241, 245, 249), false, |ui| {
                        ui.label(egui::RichText::new("Density").size(10.0));
                    });
                    widget_cell(ui, v_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(&density_str).monospace().size(10.0));
                    });
                });
            }
            ui.horizontal(|ui| {
                widget_cell(ui, k_w, egui::Color32::from_rgb(241, 245, 249), false, |ui| {
                    ui.label(egui::RichText::new(*k).size(10.0));
                });
                widget_cell(ui, v_w, egui::Color32::WHITE, false, |ui| {
                    ui.label(egui::RichText::new(*v).size(10.0));
                });
            });
        }
    });
}

fn draw_elemental_pie_chart(ui: &mut egui::Ui, title: &str, comp: &BTreeMap<String, f64>) {
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(title).strong().size(11.0));
    });
    let mut items: Vec<(String, f64)> = comp
        .iter()
        .filter(|(_, &v)| v > 0.05)
        .map(|(k, &v)| (k.clone(), v))
        .collect();
    items.sort_by(|a, b| b.1.total_cmp(&a.1));
    let total: f64 = items.iter().map(|(_, v)| *v).sum();
    if total <= 1e-6 {
        ui.centered_and_justified(|ui| ui.label("No composition data"));
        return;
    }

    let avail = ui.available_size();
    let sz = egui::vec2(avail.x.max(150.0), (avail.y - 24.0).max(140.0));
    let (rect, _) = ui.allocate_exact_size(sz, egui::Sense::hover());
    let p = ui.painter_at(rect);
    let center = rect.center();
    let radius = (rect.width().min(rect.height()) * 0.36).clamp(32.0, 95.0);

    let mut angle = -std::f32::consts::FRAC_PI_2;
    for (i, (el, val)) in items.iter().enumerate() {
        let frac = (*val / total) as f32;
        let sweep = frac * std::f32::consts::TAU;
        let segs = ((sweep * 24.0) as usize).clamp(4, 48);
        let mut pts = Vec::with_capacity(segs + 2);
        pts.push(center);
        for s in 0..=segs {
            let a = angle + sweep * (s as f32 / segs as f32);
            pts.push(egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin()));
        }
        let col = CHART_PALETTE[i % CHART_PALETTE.len()];
        p.add(egui::Shape::convex_polygon(pts, col, egui::Stroke::new(0.8, egui::Color32::WHITE)));

        if frac >= 0.04 {
            let mid = angle + sweep * 0.5;
            let lr = radius + 16.0;
            let lp = egui::pos2(center.x + lr * mid.cos(), center.y + lr * mid.sin());
            p.text(
                lp,
                egui::Align2::CENTER_CENTER,
                format!("{el} ({val:.1}%)"),
                egui::FontId::proportional(9.5),
                egui::Color32::from_rgb(30, 41, 59),
            );
        }
        angle += sweep;
    }
}

/// Variability strip chart matching `Imagen pegada (16).png` (green horizontal dashes per analysis!).
fn draw_variability_strip_chart(
    ui: &mut egui::Ui,
    title: &str,
    ordered_els: &[String],
    analyses: &[MineralChemRecord],
    selected_id: Option<u32>,
) {
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(title).strong().size(11.0));
    });
    let els: Vec<&String> = ordered_els.iter().take(12).collect();
    if els.is_empty() || analyses.is_empty() {
        ui.label("No analyses available.");
        return;
    }

    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(180.0), (avail.y - 4.0).max(160.0)), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 42.0, rect.top() + 8.0),
        egui::pos2(rect.right() - 10.0, rect.bottom() - 26.0),
    );
    if plot.width() < 30.0 || plot.height() < 30.0 {
        return;
    }
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

    let mut y_max = 10.0f64;
    for a in analyses {
        for el in &els {
            if let Some(&v) = a.values.get(el.as_str()) {
                y_max = y_max.max(v);
            }
        }
    }
    y_max = (y_max * 1.08).min(100.0);
    let y_step = nice_step(y_max / 6.0);
    let to_y = |v: f64| plot.bottom() - ((v.clamp(0.0, y_max) / y_max) as f32) * plot.height();

    let mut yv = 0.0;
    let mut g = 0;
    while yv <= y_max + 1e-6 && g < 20 {
        let py = to_y(yv);
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_gray(230)));
        p.text(egui::pos2(plot.left() - 4.0, py), egui::Align2::RIGHT_CENTER, format_tick(yv), egui::FontId::proportional(9.0), egui::Color32::BLACK);
        yv += y_step;
        g += 1;
    }
    draw_rotated_y_title(&p, rect.left() + 10.0, plot.center().y, "wt %", egui::FontId::proportional(9.5), egui::Color32::BLACK);

    let slot_w = plot.width() / (els.len() as f32);
    let tick_half = (slot_w * 0.34).min(11.0);
    for (ei, el) in els.iter().enumerate() {
        let cx = plot.left() + (ei as f32 + 0.5) * slot_w;
        p.text(egui::pos2(cx, plot.bottom() + 4.0), egui::Align2::CENTER_TOP, el.as_str(), egui::FontId::proportional(9.0), egui::Color32::BLACK);
        for a in analyses {
            if let Some(&v) = a.values.get(el.as_str()) {
                let py = to_y(v);
                let is_sel = selected_id == Some(a.id);
                let col = if is_sel {
                    egui::Color32::from_rgb(220, 38, 38)
                } else {
                    egui::Color32::from_rgb(77, 130, 56)
                };
                p.line_segment(
                    [egui::pos2(cx - tick_half, py), egui::pos2(cx + tick_half, py)],
                    egui::Stroke::new(if is_sel { 2.4 } else { 1.3 }, col),
                );
            }
        }
    }
}

/// Interactive Scatter plot matching `Imagen pegada (16).png` (click point to select record!).
fn draw_catalogue_scatter_chart(
    ui: &mut egui::Ui,
    title: &str,
    analyses: &[MineralChemRecord],
    x_el: &str,
    y_el: &str,
    z_el: Option<&str>,
    selected_id: Option<u32>,
) -> Option<u32> {
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(title).strong().size(11.0));
    });
    let avail = ui.available_size();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(avail.x.max(170.0), (avail.y - 28.0).max(135.0)),
        egui::Sense::click(),
    );
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 44.0, rect.top() + 8.0),
        egui::pos2(rect.right() - 10.0, rect.bottom() - 26.0),
    );
    if plot.width() < 30.0 || plot.height() < 30.0 {
        return None;
    }
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

    let pts: Vec<(u32, f64, f64, f64)> = analyses
        .iter()
        .filter_map(|a| {
            let x = a.values.get(x_el).copied()?;
            let y = a.values.get(y_el).copied()?;
            let z = z_el.and_then(|ze| a.values.get(ze).copied()).unwrap_or(0.0);
            Some((a.id, x, y, z))
        })
        .collect();
    if pts.is_empty() {
        p.text(plot.center(), egui::Align2::CENTER_CENTER, "No paired X/Y data", egui::FontId::proportional(10.0), egui::Color32::GRAY);
        return None;
    }

    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut zmax = 1e-6f64;
    for &(_, x, y, z) in &pts {
        xmin = xmin.min(x);
        xmax = xmax.max(x);
        ymin = ymin.min(y);
        ymax = ymax.max(y);
        zmax = zmax.max(z);
    }
    if (xmax - xmin).abs() < 1e-6 { xmin -= 1.0; xmax += 1.0; }
    if (ymax - ymin).abs() < 1e-6 { ymin -= 1.0; ymax += 1.0; }
    let xpad = (xmax - xmin) * 0.08;
    let ypad = (ymax - ymin) * 0.08;
    xmin -= xpad; xmax += xpad; ymin -= ypad; ymax += ypad;

    let to_x = |x: f64| plot.left() + (((x - xmin) / (xmax - xmin)) as f32) * plot.width();
    let to_y = |y: f64| plot.bottom() - (((y - ymin) / (ymax - ymin)) as f32) * plot.height();

    // Gridlines
    let xs = nice_step((xmax - xmin) / 4.0);
    let mut xv = (xmin / xs).ceil() * xs;
    let mut c = 0;
    while xv <= xmax && c < 12 {
        let px = to_x(xv);
        p.line_segment([egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())], egui::Stroke::new(0.6, egui::Color32::from_gray(232)));
        p.text(egui::pos2(px, plot.bottom() + 3.0), egui::Align2::CENTER_TOP, format_tick(xv), egui::FontId::proportional(8.5), egui::Color32::BLACK);
        xv += xs;
        c += 1;
    }
    let ys = nice_step((ymax - ymin) / 5.0);
    let mut yv = (ymin / ys).ceil() * ys;
    c = 0;
    while yv <= ymax && c < 12 {
        let py = to_y(yv);
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_gray(232)));
        p.text(egui::pos2(plot.left() - 4.0, py), egui::Align2::RIGHT_CENTER, format_tick(yv), egui::FontId::proportional(8.5), egui::Color32::BLACK);
        yv += ys;
        c += 1;
    }
    p.text(egui::pos2(plot.center().x, rect.bottom() - 6.0), egui::Align2::CENTER_CENTER, format!("{x_el} %"), egui::FontId::proportional(9.5), egui::Color32::BLACK);
    draw_rotated_y_title(&p, rect.left() + 10.0, plot.center().y, &format!("{y_el} %"), egui::FontId::proportional(9.5), egui::Color32::BLACK);

    let click_pos = if resp.clicked() { resp.interact_pointer_pos() } else { None };
    let mut clicked_id = None;
    let mut best_dist = 14.0f32;

    for &(id, x, y, z) in &pts {
        let pos = egui::pos2(to_x(x), to_y(y));
        let is_sel = selected_id == Some(id);
        let rad = if z_el.is_some() { 3.0 + (z / zmax) as f32 * 4.5 } else { 3.6 };
        let col = if is_sel {
            egui::Color32::from_rgb(220, 38, 38)
        } else {
            egui::Color32::from_rgb(77, 130, 56)
        };
        p.circle_filled(pos, rad, col);
        if is_sel {
            p.circle_stroke(pos, rad + 2.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
        }
        if let Some(cp) = click_pos {
            let d = cp.distance(pos);
            if d < best_dist {
                best_dist = d;
                clicked_id = Some(id);
            }
        }
    }
    clicked_id
}

fn draw_log_spidergram(
    ui: &mut egui::Ui,
    title: &str,
    x_labels: &[String],
    series: &[(String, Vec<f64>)],
) {
    if x_labels.len() < 2 || series.is_empty() {
        return;
    }
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail.x.max(320.0), avail.y.max(190.0).min(270.0)),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);
    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 48.0, rect.top() + 18.0),
        egui::pos2(rect.right() - 14.0, rect.bottom() - 28.0),
    );
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
    p.text(egui::pos2(plot.center().x, rect.top() + 8.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(10.5), egui::Color32::BLACK);

    let (mut lmin, mut lmax) = (-1.0f64, 2.0f64);
    for (_, vals) in series {
        for &v in vals {
            if v > 1e-4 {
                let lv = v.log10();
                lmin = lmin.min(lv.floor());
                lmax = lmax.max(lv.ceil());
            }
        }
    }
    let to_x = |i: usize| plot.left() + (i as f32 / (x_labels.len() - 1) as f32) * plot.width();
    let to_y = |v: f64| {
        let lv = v.max(10f64.powf(lmin)).log10().clamp(lmin, lmax);
        plot.bottom() - (((lv - lmin) / (lmax - lmin).max(1.0)) as f32) * plot.height()
    };

    let mut dec = lmin as i32;
    while dec <= lmax as i32 {
        let py = to_y(10f64.powi(dec));
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_gray(230)));
        p.text(egui::pos2(plot.left() - 4.0, py), egui::Align2::RIGHT_CENTER, format!("{}", 10f64.powi(dec)), egui::FontId::proportional(8.5), egui::Color32::BLACK);
        dec += 1;
    }
    for (i, lbl) in x_labels.iter().enumerate() {
        let px = to_x(i);
        p.line_segment([egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())], egui::Stroke::new(0.5, egui::Color32::from_gray(238)));
        p.text(egui::pos2(px, plot.bottom() + 4.0), egui::Align2::CENTER_TOP, lbl, egui::FontId::proportional(9.0), egui::Color32::BLACK);
    }

    for (si, (_, vals)) in series.iter().enumerate() {
        let col = CHART_PALETTE[si % CHART_PALETTE.len()];
        let pts: Vec<egui::Pos2> = vals
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v > 1e-4)
            .map(|(i, &v)| egui::pos2(to_x(i), to_y(v)))
            .collect();
        if pts.len() >= 2 {
            p.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.5, col)));
        }
        for pt in pts {
            p.circle_filled(pt, 2.5, col);
        }
    }
}

fn draw_stacked_distribution_chart(
    ui: &mut egui::Ui,
    title: &str,
    samples: &[String],
    minerals: &[String],
    dist: &[Vec<f64>],
) {
    if samples.is_empty() || minerals.is_empty() {
        return;
    }
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail.x.max(280.0), avail.y.max(170.0).min(240.0)),
        egui::Sense::hover(),
    );
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);
    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 46.0, rect.top() + 20.0),
        egui::pos2(rect.right() - 12.0, rect.bottom() - 28.0),
    );
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
    p.text(egui::pos2(plot.center().x, rect.top() + 9.0), egui::Align2::CENTER_CENTER, title, egui::FontId::proportional(10.5), egui::Color32::BLACK);

    let to_y = |v: f64| plot.bottom() - ((v.clamp(0.0, 100.0) / 100.0) as f32) * plot.height();
    for pct in [0.0, 25.0, 50.0, 75.0, 100.0] {
        let py = to_y(pct);
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_gray(232)));
        p.text(egui::pos2(plot.left() - 4.0, py), egui::Align2::RIGHT_CENTER, format!("{pct:.0}%"), egui::FontId::proportional(8.5), egui::Color32::BLACK);
    }

    let slot_w = plot.width() / (samples.len() as f32);
    let bar_w = (slot_w * 0.56).min(64.0);
    for (si, sid) in samples.iter().enumerate() {
        let cx = plot.left() + (si as f32 + 0.5) * slot_w;
        p.text(egui::pos2(cx, plot.bottom() + 4.0), egui::Align2::CENTER_TOP, sid, egui::FontId::proportional(9.5), egui::Color32::BLACK);
        let mut cum = 0.0;
        for (mi, &val) in dist[si].iter().enumerate() {
            if val <= 0.05 {
                continue;
            }
            let y0 = to_y(cum);
            let y1 = to_y(cum + val);
            let r = egui::Rect::from_min_max(egui::pos2(cx - bar_w * 0.5, y1), egui::pos2(cx + bar_w * 0.5, y0));
            let col = CHART_PALETTE[mi % CHART_PALETTE.len()];
            p.rect_filled(r, 0.0, col);
            p.rect_stroke(r, 0.0, egui::Stroke::new(0.6, egui::Color32::WHITE));
            if val >= 10.0 {
                p.text(r.center(), egui::Align2::CENTER_CENTER, format!("{} {:.0}%", minerals[mi], val), egui::FontId::proportional(8.5), egui::Color32::WHITE);
            }
            cum += val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_mineralogy_ui_default_state_preloads_all_six_tools() {
        let st = GeoMineralogyUiState::default();
        assert!(st.db.minerals.len() > 3800);
        assert!(!st.prop_results.is_empty());
        assert_eq!(st.comp_matches[0].mineral, "Tetrahedrite");
        assert!(st.petro.identified);
        assert!(st.petro.results.len() >= 10);
        assert_eq!(st.modal.results.len(), 4);
        assert!(st.modal.mc_result.is_some());
    }
}
