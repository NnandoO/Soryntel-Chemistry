//! Clean-room Desktop UI for Module 19: `HSC - Sim LCA` (`LCA Evaluation` 49.3 & `LCA` 49.4 Beta).
//!
//! Keeps all physical calculations, normalization, EcoSpold/JSON-LD/XLSX generation, and LCIA
//! evaluation delegated to `soryntel_thermo::sim::lca` via `SoryntelApplication`.

use eframe::egui::{self, Color32, RichText, Stroke};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::{
    lca::{
        deterministic_uuid, BetaLcaMode, BetaLcaState, BetaStreamRow, ClassicLcaState,
        ClassicProcessInfo, LcaCatalogFlow, LcaDatabaseKind, LcaFlowType, LcaFlowUnitKind,
        LcaGroup, LciaMethodSpec,
    },
    model::SimProject,
};
use std::collections::{BTreeMap, BTreeSet};

// Palette matching Chapter 49 screenshots
const RIBBON_BG: Color32 = Color32::from_rgb(245, 247, 250);
const RIBBON_BORDER: Color32 = Color32::from_rgb(203, 213, 225);
const EXCEL_HDR_GRAY: Color32 = Color32::from_rgb(236, 239, 244);
const EXCEL_GRID_LINE: Color32 = Color32::from_rgb(191, 203, 217);
const BLUE_HEADER_FILL: Color32 = Color32::from_rgb(91, 155, 213); // #5B9BD5
const BLUE_STREAM_FILL: Color32 = Color32::from_rgb(155, 194, 230); // #9BC2E6
const BLUE_SUB_FILL: Color32 = Color32::from_rgb(221, 235, 247); // #DDEBF7
const YELLOW_MATCH_FILL: Color32 = Color32::from_rgb(254, 240, 138); // Yellow Find highlight (Fig. 24)
const ACTIVE_BTN_FILL: Color32 = Color32::from_rgb(219, 234, 254);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcaUiMode {
    Classic,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassicTab {
    Inputs,
    Outputs,
    ManualInputs,
    ManualOutputs,
    Indicators,
    Normalized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BetaSheetTab {
    Inputs,
    Outputs,
    Results,
    Custom(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BetaDockTab {
    Flows,
    ImpactMethods,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BetaRowSelection {
    pub is_output: bool,
    pub stream_idx: usize,
    pub species_idx: Option<usize>,
    pub excel_cell_ref: [char; 2], // Column letter + digit display cache
    pub excel_row_num: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassicRowSelection {
    pub tab: ClassicTab,
    pub idx: usize,
}

pub struct LcaWindowState {
    pub ui_mode: LcaUiMode,

    // Classic state (49.3)
    pub classic: ClassicLcaState,
    pub classic_tab: ClassicTab,
    pub classic_show_mapping: bool,
    pub classic_mapping_dirty: bool,
    pub classic_pending_tab: Option<ClassicTab>,
    pub classic_show_apply_modal: bool,
    pub classic_selection: Option<ClassicRowSelection>,
    pub classic_filter_unit: Option<LcaFlowUnitKind>,
    pub classic_search_selected_cat: bool,
    pub classic_selected_category: Option<String>,
    pub classic_keyword: String,
    pub classic_selected_flow_id: Option<String>,
    pub classic_expanded_cats: BTreeSet<String>,
    pub classic_new_indicator: String,

    // Classic modals
    pub show_process_info_modal: bool,
    pub process_info_draft: ClassicProcessInfo,
    pub show_openlca_db_modal: bool,
    pub openlca_zip_path_input: String,
    pub show_export_modal: bool,
    pub export_modal_title: String,
    pub export_modal_body: String,
    pub export_last_saved_path: Option<String>,

    // Beta state (49.4)
    pub beta: BetaLcaState,
    pub beta_sheet: BetaSheetTab,
    pub beta_dock_tab: BetaDockTab,
    pub beta_selection: Option<BetaRowSelection>,
    pub beta_formula_text: String,
    pub beta_cell_ref_label: String,
    pub beta_find_query: String,
    pub beta_active_find: String,
    pub beta_selected_flow_id: Option<String>,
    pub beta_selected_method_cat: Option<(String, String)>,
    pub beta_expanded_folders: BTreeSet<String>,

    // Beta modals
    pub beta_error_modal: Option<String>,
    pub show_rename_sheet_modal: bool,
    pub rename_sheet_input: String,
    pub rename_sheet_creating_new: bool,
    pub show_ext_import_modal: bool,
    pub ext_selected_tables: BTreeSet<usize>,
    pub show_synth_processes_modal: bool,
    pub show_help_modal: bool,

    // Shared toast / status line
    pub status_message: String,
}

impl Default for LcaWindowState {
    fn default() -> Self {
        let mut expanded_cats = BTreeSet::new();
        expanded_cats.insert("Resources".to_string());
        expanded_cats.insert("Emissions to air".to_string());
        expanded_cats.insert("Others".to_string());
        expanded_cats.insert("Valuable substances".to_string());

        let mut expanded_folders = BTreeSet::new();
        expanded_folders.insert("Flows".to_string());
        expanded_folders.insert("C:Manufacturing".to_string());
        expanded_folders.insert("D:Electricity, gas, steam and air conditioning supply".to_string());
        expanded_folders.insert("Elementary flows".to_string());
        expanded_folders.insert("recipe_midpoint_h".to_string());

        Self {
            ui_mode: LcaUiMode::Classic,
            classic: ClassicLcaState::default(),
            classic_tab: ClassicTab::Inputs,
            classic_show_mapping: false,
            classic_mapping_dirty: false,
            classic_pending_tab: None,
            classic_show_apply_modal: false,
            classic_selection: None,
            classic_filter_unit: None,
            classic_search_selected_cat: false,
            classic_selected_category: None,
            classic_keyword: String::new(),
            classic_selected_flow_id: None,
            classic_expanded_cats: expanded_cats,
            classic_new_indicator: "SO2(g)".to_string(),
            show_process_info_modal: false,
            process_info_draft: ClassicProcessInfo::default(),
            show_openlca_db_modal: false,
            openlca_zip_path_input: String::new(),
            show_export_modal: false,
            export_modal_title: String::new(),
            export_modal_body: String::new(),
            export_last_saved_path: None,
            beta: BetaLcaState::default(),
            beta_sheet: BetaSheetTab::Inputs,
            beta_dock_tab: BetaDockTab::Flows,
            beta_selection: None,
            beta_formula_text: String::new(),
            beta_cell_ref_label: "A1".to_string(),
            beta_find_query: String::new(),
            beta_active_find: String::new(),
            beta_selected_flow_id: None,
            beta_selected_method_cat: None,
            beta_expanded_folders: expanded_folders,
            beta_error_modal: None,
            show_rename_sheet_modal: false,
            rename_sheet_input: String::new(),
            rename_sheet_creating_new: false,
            show_ext_import_modal: false,
            ext_selected_tables: BTreeSet::new(),
            show_synth_processes_modal: false,
            show_help_modal: false,
            status_message: "Ready".to_string(),
        }
    }
}

impl LcaWindowState {
    pub fn with_preset_classic_zn() -> (SimProject, Self) {
        let (proj, classic) = SoryntelApplication::sim_lca_preset_classic_zn();
        let mut st = Self::default();
        st.ui_mode = LcaUiMode::Classic;
        st.classic = classic;
        st.beta.sync_from_project(&proj);
        st.status_message = "Loaded Zn TSL Fuming benchmark (Ch. 49.3)".to_string();
        (proj, st)
    }

    pub fn with_preset_beta_iron() -> (SimProject, Self) {
        let (proj, beta) = SoryntelApplication::sim_lca_preset_beta_iron();
        let mut st = Self::default();
        st.ui_mode = LcaUiMode::Beta;
        st.classic.sync_from_project(&proj);
        st.beta = beta;
        st.beta_sheet = BetaSheetTab::Results;
        st.status_message = "Loaded Iron Process LCIA benchmark (Ch. 49.4)".to_string();
        (proj, st)
    }

    pub fn sync_from_project(&mut self, project: &SimProject) {
        SoryntelApplication::sim_lca_classic_sync(&mut self.classic, project);
        SoryntelApplication::sim_lca_beta_sync(&mut self.beta, project);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Standalone Module Entry Point (Launcher --module lca)
// ─────────────────────────────────────────────────────────────────────────────

pub fn render_standalone_lca_module(
    ctx: &egui::Context,
    project: &mut SimProject,
    state: &mut LcaWindowState,
) {
    egui::TopBottomPanel::top("standalone_lca_top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Soryntel · Module 19: HSC Sim LCA")
                    .strong()
                    .color(Color32::from_rgb(15, 23, 42)),
            );
            ui.separator();
            ui.label("Mode:");
            if ui
                .selectable_label(state.ui_mode == LcaUiMode::Classic, "Classic (LCA Evaluation 49.3)")
                .clicked()
            {
                state.ui_mode = LcaUiMode::Classic;
            }
            if ui
                .selectable_label(state.ui_mode == LcaUiMode::Beta, "Beta (openLCA IPC / LCIA 49.4)")
                .clicked()
            {
                state.ui_mode = LcaUiMode::Beta;
            }
            ui.separator();
            ui.label("Load Benchmark:");
            if ui.button("Zn TSL Fuming (49.3)").clicked() {
                let (p, next_state) = LcaWindowState::with_preset_classic_zn();
                *project = p;
                *state = next_state;
            }
            if ui.button("Iron Process (49.4)").clicked() {
                let (p, next_state) = LcaWindowState::with_preset_beta_iron();
                *project = p;
                *state = next_state;
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        render_lca_body(ui, ctx, project, state);
    });
}

/// Modal/floating window entrypoint invoked inside `HSC Sim -> Tools -> LCA Evaluation` / `(beta)`.
pub fn render_lca_floating_window(
    ctx: &egui::Context,
    open: &mut bool,
    project: &SimProject,
    state: &mut LcaWindowState,
) {
    let title = match state.ui_mode {
        LcaUiMode::Classic => "LCA Evaluation",
        LcaUiMode::Beta => "LCA",
    };
    egui::Window::new(title)
        .id(egui::Id::new("hsc_sim_lca_main_window"))
        .open(open)
        .default_size([1180.0, 690.0])
        .min_size([860.0, 500.0])
        .resizable(true)
        .show(ctx, |ui| {
            render_lca_body(ui, ctx, project, state);
        });
}

/// Shared inner canvas used by both standalone Launcher window and `HSC Sim -> Tools` floating window.
pub fn render_lca_body(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    project: &SimProject,
    state: &mut LcaWindowState,
) {
    ui.colored_label(Color32::from_rgb(180, 90, 0), soryntel_thermo::sim::lca::BUILTIN_LCA_NOTICE);
    match state.ui_mode {
        LcaUiMode::Classic => render_classic_lca(ui, ctx, project, state),
        LcaUiMode::Beta => render_beta_lca(ui, ctx, project, state),
    }
    render_shared_lca_modals(ctx, project, state);
}

// ─────────────────────────────────────────────────────────────────────────────
// Part A Renderer: Classic `LCA Evaluation` (`49.3 Figs. 9–31`)
// ─────────────────────────────────────────────────────────────────────────────

fn ribbon_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(RIBBON_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    add_contents(ui);
                });
                ui.add_space(2.0);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(title)
                            .size(10.0)
                            .color(Color32::from_rgb(71, 85, 105)),
                    );
                });
            });
        });
}

fn ribbon_tile_button(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, enabled: bool) -> egui::Response {
    let text = if icon.is_empty() {
        label.to_string()
    } else {
        format!("{icon}\n{label}")
    };
    let fill = if !enabled {
        Color32::from_rgb(241, 245, 249)
    } else if active {
        ACTIVE_BTN_FILL
    } else {
        Color32::WHITE
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).size(10.5))
            .fill(fill)
            .min_size(egui::vec2(54.0, 44.0)),
    )
}

fn render_classic_lca(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    project: &SimProject,
    state: &mut LcaWindowState,
) {
    // 1. Classic 5-Group Ribbon (`Database`, `Streams`, `Tools`, `Export`, `Info`)
    ui.horizontal_wrapped(|ui| {
        ribbon_group(ui, "Database", |ui| {
            if ribbon_tile_button(
                ui,
                "🌐",
                "GaBi",
                state.classic.active_db == LcaDatabaseKind::GaBi,
                true,
            )
            .clicked()
            {
                state.classic.active_db = LcaDatabaseKind::GaBi;
                state.classic.refresh_indicators();
                state.status_message = "Switched Classic DB to GaBi".to_string();
            }
            let olca_active = state.classic.active_db == LcaDatabaseKind::OpenLca;
            ui.menu_button(
                RichText::new("LCA\nOpenLCA ▾")
                    .size(10.5)
                    .color(if olca_active {
                        Color32::from_rgb(29, 78, 216)
                    } else {
                        Color32::BLACK
                    }),
                |ui| {
                    ui.label(
                        RichText::new(format!("Active: {}", state.classic.active_openlca_db_path))
                            .small()
                            .color(Color32::DARK_GRAY),
                    );
                    ui.separator();
                    if ui.button("Use Built-in OpenLCA Catalog").clicked() {
                        state.classic.active_db = LcaDatabaseKind::OpenLca;
                        state.classic.refresh_indicators();
                        state.status_message = "Switched Classic DB to Built-in OpenLCA".to_string();
                        ui.close_menu();
                    }
                    if ui.button("Set Active Database...").clicked() {
                        state.show_openlca_db_modal = true;
                        ui.close_menu();
                    }
                },
            );
        });

        ribbon_group(ui, "Streams", |ui| {
            let mut next_tab: Option<ClassicTab> = None;
            for (icon, label, tab) in [
                ("⬇", "Input", ClassicTab::Inputs),
                ("⬆", "Output", ClassicTab::Outputs),
                ("⬇M", "Manual\nInput", ClassicTab::ManualInputs),
                ("⬆M", "Manual\nOutput", ClassicTab::ManualOutputs),
                ("⬆I", "Indicator", ClassicTab::Indicators),
            ] {
                if ribbon_tile_button(ui, icon, label, state.classic_tab == tab, true).clicked() {
                    next_tab = Some(tab);
                }
            }
            if let Some(target) = next_tab {
                if state.classic_show_mapping && state.classic_mapping_dirty {
                    state.classic_pending_tab = Some(target);
                    state.classic_show_apply_modal = true;
                } else {
                    state.classic_tab = target;
                }
            }
        });

        ribbon_group(ui, "Tools", |ui| {
            if ribbon_tile_button(ui, "⇄", "Mapping", state.classic_show_mapping, true).clicked() {
                if state.classic_show_mapping && state.classic_mapping_dirty {
                    state.classic_pending_tab = None;
                    state.classic_show_apply_modal = true;
                } else {
                    state.classic_show_mapping = !state.classic_show_mapping;
                }
            }
            if ribbon_tile_button(ui, "▤", "Normalize", state.classic_tab == ClassicTab::Normalized, true).clicked() {
                match SoryntelApplication::sim_lca_classic_normalize(&mut state.classic) {
                    Ok(()) => {
                        state.classic_tab = ClassicTab::Normalized;
                        state.status_message = format!(
                            "Normalized {} unique flows to 1 kg Main Product",
                            state.classic.normalized.len()
                        );
                    }
                    Err(e) => {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
            if ribbon_tile_button(ui, "⚙", "Process\nInfo", false, true).clicked() {
                state.process_info_draft = state.classic.process_info.clone();
                state.show_process_info_modal = true;
            }
        });

        ribbon_group(ui, "Export", |ui| {
            let is_gabi = state.classic.active_db == LcaDatabaseKind::GaBi;
            if ribbon_tile_button(ui, "💾", "To EcoSpold\nv1.0 (GaBi)", false, is_gabi).clicked() {
                match SoryntelApplication::sim_lca_classic_ecospold_xml(&mut state.classic) {
                    Ok(xml) => {
                        let out_path = "/tmp/soryntel_ecospold_v1.xml";
                        let saved = crate::storage::write_atomic(out_path, xml.as_bytes()).is_ok();
                        state.export_last_saved_path = saved.then(|| out_path.to_string());
                        state.export_modal_title = "EcoSpold v1.0 export succeeded!".to_string();
                        state.export_modal_body = xml;
                        state.show_export_modal = true;
                        state.classic_tab = ClassicTab::Normalized;
                    }
                    Err(e) => state.beta_error_modal = Some(e),
                }
            }
            let olca_enabled = !is_gabi;
            ui.add_enabled_ui(olca_enabled, |ui| {
                ui.menu_button(RichText::new("💾\nTo JSON-LD\n(OpenLCA) ▾").size(10.5), |ui| {
                    for action in ["To Active Database", "To Empty File..."] {
                        if ui.button(action).clicked() {
                            match SoryntelApplication::sim_lca_classic_jsonld_zip(&mut state.classic) {
                                Ok(bytes) => {
                                    let out_path = "/tmp/soryntel_openlca_jsonld.zip";
                                    let saved = crate::storage::write_atomic(out_path, &bytes).is_ok();
                                    state.export_last_saved_path = saved.then(|| out_path.to_string());
                                    state.export_modal_title = "JSON-LD export succeeded!".to_string();
                                    state.export_modal_body = format!(
                                        "Export target: {action}\nArchive size: {} bytes\nSaved: {out_path}\nProcess category: HSC\nNormalized flows exported: {}",
                                        bytes.len(),
                                        state.classic.normalized.len()
                                    );
                                    state.show_export_modal = true;
                                    state.classic_tab = ClassicTab::Normalized;
                                }
                                Err(e) => state.beta_error_modal = Some(e),
                            }
                            ui.close_menu();
                        }
                    }
                });
            });
            if ribbon_tile_button(ui, "XLS", "To Excel", false, true).clicked() {
                match SoryntelApplication::sim_lca_classic_xlsx(&mut state.classic) {
                    Ok(bytes) => {
                        let out_path = "/tmp/soryntel_lca_classic.xlsx";
                        let saved = crate::storage::write_atomic(out_path, &bytes).is_ok();
                        state.export_last_saved_path = saved.then(|| out_path.to_string());
                        state.export_modal_title = "Excel Export Succeeded".to_string();
                        state.export_modal_body = format!(
                            "Exported 6 sheets (Input Streams, Output Streams, Manual Input Streams, Manual Output Streams, Indicator Streams, Normalized Streams) ({} bytes) to {out_path}.",
                            bytes.len()
                        );
                        state.show_export_modal = true;
                    }
                    Err(e) => state.beta_error_modal = Some(e),
                }
            }
        });

        ribbon_group(ui, "Info", |ui| {
            if ribbon_tile_button(ui, "❓", "Help", false, true).clicked() {
                state.show_help_modal = true;
            }
            if ribbon_tile_button(ui, "⟳", "Sync", false, true)
                .on_hover_text("Synchronize boundary streams from active HSC Sim flowsheet")
                .clicked()
            {
                state.sync_from_project(project);
                state.status_message = "Synced boundary streams from flowsheet".to_string();
            }
        });
    });

    ui.separator();

    // 2. Main Split Area (Left Tabs+Grid ~65% when Mapping active, Right Mapping Browser ~35%)
    let avail = ui.available_size();
    let left_w = if state.classic_show_mapping {
        (avail.x * 0.63).max(460.0)
    } else {
        avail.x
    };

    ui.horizontal_top(|ui| {
        ui.allocate_ui(egui::vec2(left_w, avail.y - 24.0), |ui| {
            ui.vertical(|ui| {
                // Sheet Tabs (matching Fig. 9–17)
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (ClassicTab::Inputs, "Inputs"),
                        (ClassicTab::Outputs, "Outputs"),
                        (ClassicTab::ManualInputs, "Manual Inputs"),
                        (ClassicTab::ManualOutputs, "Manual Outputs"),
                        (ClassicTab::Indicators, "Indicators"),
                        (ClassicTab::Normalized, "Normalized Flows"),
                    ] {
                        if ui.selectable_label(state.classic_tab == tab, label).clicked() {
                            if state.classic_show_mapping && state.classic_mapping_dirty {
                                state.classic_pending_tab = Some(tab);
                                state.classic_show_apply_modal = true;
                            } else {
                                if tab == ClassicTab::Normalized && !state.classic.normalized_valid {
                                    let _ = state.classic.normalize();
                                }
                                state.classic_tab = tab;
                            }
                        }
                    }
                });
                ui.separator();

                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    match state.classic_tab {
                        ClassicTab::Inputs => render_classic_streams_table(ui, state, false, false),
                        ClassicTab::Outputs => render_classic_streams_table(ui, state, true, false),
                        ClassicTab::ManualInputs => render_classic_streams_table(ui, state, false, true),
                        ClassicTab::ManualOutputs => render_classic_streams_table(ui, state, true, true),
                        ClassicTab::Indicators => render_classic_indicators_table(ui, state),
                        ClassicTab::Normalized => render_classic_normalized_table(ui, state),
                    }
                });
            });
        });

        if state.classic_show_mapping {
            ui.separator();
            ui.vertical(|ui| {
                render_classic_mapping_dock(ui, state);
            });
        }
    });

    ui.separator();
    ui.label(RichText::new(&state.status_message).small().color(Color32::DARK_GRAY));
}

fn render_classic_streams_table(
    ui: &mut egui::Ui,
    state: &mut LcaWindowState,
    is_output: bool,
    is_manual: bool,
) {
    let tab = match (is_output, is_manual) {
        (false, false) => ClassicTab::Inputs,
        (true, false) => ClassicTab::Outputs,
        (false, true) => ClassicTab::ManualInputs,
        (true, true) => ClassicTab::ManualOutputs,
    };
    let title = match (is_output, is_manual) {
        (false, false) => "Input Streams",
        (true, false) => "Output Streams",
        (false, true) => "Manual Input Streams",
        (true, true) => "Manual Output Streams",
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().size(13.0));
    });
    ui.add_space(4.0);

    let group_options = if is_output {
        LcaGroup::output_options()
    } else {
        LcaGroup::input_options()
    };

    let len = match (is_output, is_manual) {
        (false, false) => state.classic.inputs.len(),
        (true, false) => state.classic.outputs.len(),
        (false, true) => state.classic.manual_inputs.len(),
        (true, true) => state.classic.manual_outputs.len(),
    };

    let mut set_main_idx: Option<(usize, bool)> = None;
    let mut delete_manual_idx: Option<usize> = None;

    egui::Grid::new(format!("classic_table_{title}"))
        .striped(true)
        .min_col_width(50.0)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            // Header row
            ui.label("");
            ui.label(RichText::new("Stream Name").strong());
            if !is_manual {
                ui.label(RichText::new(crate::i18n::text("common.096")).strong());
            }
            ui.label(RichText::new(crate::i18n::text("common.017")).strong());
            ui.label(RichText::new(crate::i18n::text("common.036")).strong());
            ui.label(RichText::new("Use Exergy").strong());
            ui.label(RichText::new(crate::i18n::text("common.097")).strong());
            ui.label(RichText::new(crate::i18n::text("common.098")).strong());
            ui.label(RichText::new("Main Product").strong());
            if state.classic_show_mapping {
                ui.label(RichText::new(crate::i18n::text("common.037")).strong());
            } else if is_manual {
                ui.label(RichText::new(crate::i18n::text("common.013")).strong());
            }
            ui.end_row();

            for i in 0..len {
                let is_selected = state.classic_selection == Some(ClassicRowSelection { tab, idx: i });
                let row = match (is_output, is_manual) {
                    (false, false) => &mut state.classic.inputs[i],
                    (true, false) => &mut state.classic.outputs[i],
                    (false, true) => &mut state.classic.manual_inputs[i],
                    (true, true) => &mut state.classic.manual_outputs[i],
                };

                if !is_manual {
                    let expand_lbl = if row.expanded { "⊟" } else { "⊞" };
                    if ui.small_button(expand_lbl).clicked() {
                        row.expanded = !row.expanded;
                    }
                } else {
                    ui.label("•");
                }

                if is_manual {
                    ui.add(egui::TextEdit::singleline(&mut row.stream_name).desired_width(130.0));
                } else {
                    let prefix = if is_selected { "▶ " } else { "" };
                    if ui
                        .selectable_label(is_selected, format!("{prefix}{}", row.stream_name))
                        .clicked()
                    {
                        state.classic_selection = Some(ClassicRowSelection { tab, idx: i });
                    }
                    ui.label(&row.unit_name);
                }

                if is_manual {
                    let mut val = row.mass_flow_t_h;
                    if ui
                        .add(egui::DragValue::new(&mut val).speed(1.0).max_decimals(2))
                        .changed()
                    {
                        row.mass_flow_t_h = val;
                        state.classic.normalized_valid = false;
                    }
                    egui::ComboBox::from_id_salt(("classic_man_unit", is_output, i))
                        .selected_text(if row.is_energy_only { "kWh" } else { "kg" })
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(!row.is_energy_only, "kg").clicked() {
                                row.is_energy_only = false;
                            }
                            if ui.selectable_label(row.is_energy_only, "kWh").clicked() {
                                row.is_energy_only = true;
                            }
                        });
                } else {
                    ui.label(format!("{:.2}", row.converted_amount()));
                    ui.label(row.converted_unit());
                }

                let mut use_ex = row.use_exergy;
                if ui.checkbox(&mut use_ex, "").changed() {
                    row.use_exergy = use_ex;
                    state.classic.normalized_valid = false;
                }

                if ui
                    .selectable_label(is_selected, &row.lca_equivalent)
                    .on_hover_text("Select row then choose a Flow in the right Mapping dock")
                    .clicked()
                {
                    state.classic_selection = Some(ClassicRowSelection { tab, idx: i });
                    if !state.classic_show_mapping {
                        state.classic_show_mapping = true;
                    }
                }

                let mut grp = row.lca_group;
                egui::ComboBox::from_id_salt(("classic_grp", title, i))
                    .selected_text(grp.as_str())
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for &opt in group_options {
                            if ui.selectable_value(&mut grp, opt, opt.as_str()).changed() {
                                row.lca_group = grp;
                                state.classic.normalized_valid = false;
                                if is_output {
                                    // Will refresh indicators cleanly below
                                    state.classic_mapping_dirty = true;
                                }
                            }
                        }
                    });

                let mut mp = row.main_product;
                if ui.checkbox(&mut mp, "").changed() {
                    set_main_idx = Some((i, mp));
                }

                if state.classic_show_mapping {
                    if ui
                        .button(RichText::new("⊗").color(Color32::from_rgb(185, 28, 28)))
                        .on_hover_text("Clear LCA Equivalent")
                        .clicked()
                    {
                        row.lca_equivalent = "No Mapping".to_string();
                        row.lca_flow_id = None;
                        state.classic_mapping_dirty = true;
                        state.classic.normalized_valid = false;
                    }
                } else if is_manual {
                    if ui
                        .button(RichText::new("✖").color(Color32::from_rgb(185, 28, 28)))
                        .on_hover_text("Delete manual stream")
                        .clicked()
                    {
                        delete_manual_idx = Some(i);
                    }
                }
                ui.end_row();

                // Expanded read-only thermodynamic & species detail card (Fig. 9)
                if row.expanded && !is_manual {
                    ui.label("");
                    ui.label(
                        RichText::new(format!(
                            "Mass: {:.2} t/h | T: {:.1} °C | P: {:.2} bar\nEnthalpy: {:.2} kW | Exergy: {:.2} kW\nVolume: {:.2} Nm3 | Solids: {:.2} t/h",
                            row.mass_flow_t_h,
                            row.temp_c,
                            row.pressure_bar,
                            row.enthalpy_kw,
                            row.exergy_kw,
                            row.total_volume_nm3,
                            row.total_solids_t_h,
                        ))
                        .small()
                        .color(Color32::from_rgb(51, 65, 85)),
                    );
                    let sp_summary: String = row
                        .species
                        .iter()
                        .map(|s| format!("{} ({:.3} t/h)", s.formula, s.mass_flow_t_h))
                        .collect::<Vec<_>>()
                        .join(", ");
                    ui.label(RichText::new(sp_summary).small().color(Color32::from_rgb(71, 85, 105)));
                    ui.end_row();
                }
            }
        });

    if let Some((idx, checked)) = set_main_idx {
        state.classic.set_main_product(is_output, is_manual, idx, checked);
    }
    if let Some(idx) = delete_manual_idx {
        state.classic.delete_manual_stream(is_output, idx);
    }

    if is_manual {
        ui.add_space(8.0);
        let btn_lbl = if is_output {
            "Add new output stream"
        } else {
            "Add new input stream"
        };
        if ui
            .add(egui::Button::new(btn_lbl).min_size(egui::vec2(ui.available_width().min(560.0), 26.0)))
            .clicked()
        {
            state.classic.add_manual_stream(is_output);
        }
    }
}

fn render_classic_indicators_table(ui: &mut egui::Ui, state: &mut LcaWindowState) {
    ui.label(RichText::new("Indicator Streams (Wildcards: *, ?, #)").strong().size(13.0));
    ui.add_space(4.0);

    let mut delete_idx: Option<usize> = None;
    let mut toggle_counted: Option<(usize, usize, bool)> = None;
    let mut refresh_needed = false;

    egui::Grid::new("classic_indicators_grid")
        .striped(true)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label(RichText::new("Stream Name / Pattern").strong());
            ui.label(RichText::new(crate::i18n::text("common.017")).strong());
            ui.label(RichText::new(crate::i18n::text("common.036")).strong());
            ui.label(RichText::new(crate::i18n::text("common.097")).strong());
            ui.label(RichText::new(crate::i18n::text("common.098")).strong());
            ui.label(RichText::new("Action").strong());
            ui.end_row();

            for i in 0..state.classic.indicators.len() {
                let is_sel = state.classic_selection == Some(ClassicRowSelection { tab: ClassicTab::Indicators, idx: i });
                let total_amt = state.classic.indicators[i].total_counted_amount();
                let ind = &mut state.classic.indicators[i];

                if ui.small_button(if ind.expanded { "⊟" } else { "⊞" }).clicked() {
                    ind.expanded = !ind.expanded;
                }
                if ui.add(egui::TextEdit::singleline(&mut ind.pattern).desired_width(110.0)).changed() {
                    refresh_needed = true;
                }
                ui.label(format!("{total_amt:.2}"));
                ui.label(&ind.unit);
                if ui.selectable_label(is_sel, &ind.lca_equivalent).clicked() {
                    state.classic_selection = Some(ClassicRowSelection { tab: ClassicTab::Indicators, idx: i });
                    state.classic_show_mapping = true;
                }
                egui::ComboBox::from_id_salt(("classic_ind_grp", i))
                    .selected_text(ind.lca_group.as_str())
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for &opt in LcaGroup::output_options() {
                            if ui.selectable_value(&mut ind.lca_group, opt, opt.as_str()).changed() {
                                state.classic.normalized_valid = false;
                            }
                        }
                    });

                if state.classic_show_mapping {
                    if ui.button(RichText::new("⊗").color(Color32::from_rgb(185, 28, 28))).clicked() {
                        ind.lca_equivalent = "No Mapping".to_string();
                        ind.lca_flow_id = None;
                        state.classic_mapping_dirty = true;
                    }
                } else if ui.button(RichText::new("✖").color(Color32::from_rgb(185, 28, 28))).clicked() {
                    delete_idx = Some(i);
                }
                ui.end_row();

                if ind.expanded {
                    for (si, src) in ind.sources.iter().enumerate() {
                        ui.label("");
                        ui.label(RichText::new(format!("↳ {} [{}]", src.stream_name, src.species_formula)).small());
                        ui.label(RichText::new(format!("{:.2}", src.amount_kg)).small());
                        ui.label(RichText::new("kg").small());
                        let mut c = src.counted;
                        if ui.checkbox(&mut c, "Counted").changed() {
                            toggle_counted = Some((i, si, c));
                        }
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    }
                }
            }
        });

    if let Some((i, si, c)) = toggle_counted {
        state.classic.set_counted(i, si, c);
    }
    if let Some(i) = delete_idx {
        state.classic.indicators.remove(i);
        state.classic.normalized_valid = false;
    }
    if refresh_needed {
        state.classic.refresh_indicators();
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.classic_new_indicator).desired_width(140.0));
        if ui.button("Add new indicator").clicked() {
            let pat = state.classic_new_indicator.trim().to_string();
            if !pat.is_empty() {
                state.classic.add_indicator(pat);
            }
        }
    });
}

fn render_classic_normalized_table(ui: &mut egui::Ui, state: &mut LcaWindowState) {
    ui.label(RichText::new("Normalized Flows (per 1 kg Main Product)").strong().size(13.0));
    ui.add_space(4.0);
    if state.classic.normalized.is_empty() {
        ui.label("Click Normalize in the Ribbon after marking one Main Product stream.");
        return;
    }

    egui::Grid::new("classic_normalized_grid")
        .striped(true)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            for h in [
                "Direction",
                "Flow Name",
                "Group Name",
                "Category",
                "Sub Category",
                "Amount",
                "Normalized Amount",
                "Unit",
            ] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();

            for n in &state.classic.normalized {
                ui.label(&n.direction);
                ui.label(&n.flow_name);
                ui.label(&n.group_name);
                ui.label(&n.category);
                ui.label(&n.sub_category);
                ui.label(format!("{:.2}", n.amount));
                ui.label(RichText::new(format!("{:.2}", n.normalized_amount)).strong());
                ui.label(&n.unit);
                ui.end_row();
            }
        });
}

fn render_classic_mapping_dock(ui: &mut egui::Ui, state: &mut LcaWindowState) {
    let flows = state.classic.available_flows();
    ui.horizontal(|ui| {
        ui.label("Unit type:");
        let sel_lbl = state
            .classic_filter_unit
            .map(|u| u.as_str())
            .unwrap_or("All");
        egui::ComboBox::from_id_salt("classic_unit_filter")
            .selected_text(sel_lbl)
            .width(120.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.classic_filter_unit, None, "All");
                for u in [
                    LcaFlowUnitKind::Mass,
                    LcaFlowUnitKind::Energy,
                    LcaFlowUnitKind::Volume,
                    LcaFlowUnitKind::Area,
                ] {
                    ui.selectable_value(&mut state.classic_filter_unit, Some(u), u.as_str());
                }
            });
        ui.checkbox(&mut state.classic_search_selected_cat, "Search from selected category");
    });

    ui.horizontal(|ui| {
        ui.label("Keyword Search:");
        ui.add(egui::TextEdit::singleline(&mut state.classic_keyword).desired_width(170.0));
    });
    ui.separator();

    // Group flows by root category
    let mut by_cat: BTreeMap<String, Vec<LcaCatalogFlow>> = BTreeMap::new();
    for f in &flows {
        let root = f
            .category_path
            .first()
            .cloned()
            .unwrap_or_else(|| "Flows".to_string());
        by_cat.entry(root).or_default().push(f.clone());
    }

    let kw = state.classic_keyword.trim().to_ascii_lowercase();
    let filtered: Vec<LcaCatalogFlow> = flows
        .iter()
        .filter(|f| {
            let unit_ok = state
                .classic_filter_unit
                .map(|u| f.unit_kind == u)
                .unwrap_or(true);
            let cat_ok = !state.classic_search_selected_cat
                || state
                    .classic_selected_category
                    .as_ref()
                    .map(|c| f.category_path.iter().any(|p| p == c))
                    .unwrap_or(true);
            let kw_ok = kw.is_empty()
                || f.name.to_ascii_lowercase().contains(&kw)
                || (kw.contains("scrap") && f.name.to_ascii_lowercase().contains("srap"));
            unit_ok && cat_ok && kw_ok
        })
        .cloned()
        .collect();

    let dock_h = (ui.available_height() - 165.0).max(160.0);
    ui.columns(2, |cols| {
        cols[0].label(RichText::new("Flows").strong());
        egui::ScrollArea::vertical()
            .id_salt("classic_cat_tree")
            .max_height(dock_h)
            .show(&mut cols[0], |ui| {
                for (cat, items) in &by_cat {
                    let is_open = state.classic_expanded_cats.contains(cat);
                    ui.horizontal(|ui| {
                        if ui.small_button(if is_open { "−" } else { "+" }).clicked() {
                            if is_open {
                                state.classic_expanded_cats.remove(cat);
                            } else {
                                state.classic_expanded_cats.insert(cat.clone());
                            }
                        }
                        let sel = state.classic_selected_category.as_ref() == Some(cat);
                        if ui.selectable_label(sel, cat).clicked() {
                            state.classic_selected_category = Some(cat.clone());
                        }
                    });
                    if is_open {
                        for item in items {
                            let is_sel = state.classic_selected_flow_id.as_deref() == Some(&item.id);
                            if ui
                                .selectable_label(is_sel, format!("    • {}", item.name))
                                .clicked()
                            {
                                state.classic_selected_flow_id = Some(item.id.clone());
                            }
                        }
                    }
                }
            });

        cols[1].label(RichText::new("Matching Flows (Double-click / Assign)").strong());
        egui::ScrollArea::vertical()
            .id_salt("classic_search_hits")
            .max_height(dock_h)
            .show(&mut cols[1], |ui| {
                for item in &filtered {
                    let is_sel = state.classic_selected_flow_id.as_deref() == Some(&item.id);
                    let resp = ui.selectable_label(is_sel, &item.name);
                    if resp.clicked() {
                        state.classic_selected_flow_id = Some(item.id.clone());
                    }
                    if resp.double_clicked() {
                        assign_classic_flow(state, item);
                    }
                }
            });
    });

    ui.separator();
    // Bottom Metadata Inspector (`49.3 Fig. 15`)
    let active_flow = state
        .classic_selected_flow_id
        .as_deref()
        .and_then(|id| flows.iter().find(|f| f.id == id));
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                active_flow.is_some() && state.classic_selection.is_some(),
                egui::Button::new("Assign Flow to Selected Row"),
            )
            .clicked()
        {
            if let Some(f) = active_flow {
                assign_classic_flow(state, f);
            }
        }
    });
    if let Some(f) = active_flow {
        ui.label(RichText::new(format!("Name: {}", f.name)).small());
        ui.label(RichText::new(format!("Category: \\{}", f.category_string())).small());
        ui.label(RichText::new(format!("Unit: {}", f.unit_label)).small());
        ui.label(RichText::new(format!("Reference Quantity: {}", f.reference_quantity)).small());
        ui.label(RichText::new(format!("Energy Quantity: {}", f.energy_quantity)).small());
        ui.label(RichText::new(format!("Conversion Factor: {:.2}", f.conversion_factor)).small());
    } else {
        ui.label(
            RichText::new("Select a flow above and assign it to the selected stream/indicator row.")
                .small()
                .color(Color32::GRAY),
        );
    }
}

fn assign_classic_flow(state: &mut LcaWindowState, flow: &LcaCatalogFlow) {
    let Some(sel) = state.classic_selection else {
        state.status_message = "Select a stream or indicator row on the left first.".to_string();
        return;
    };
    match sel.tab {
        ClassicTab::Inputs => {
            if let Some(r) = state.classic.inputs.get_mut(sel.idx) {
                r.lca_equivalent = flow.name.clone();
                r.lca_flow_id = Some(flow.id.clone());
                r.lca_group = flow.gabi_group_hint;
            }
        }
        ClassicTab::Outputs => {
            if let Some(r) = state.classic.outputs.get_mut(sel.idx) {
                r.lca_equivalent = flow.name.clone();
                r.lca_flow_id = Some(flow.id.clone());
                r.lca_group = flow.gabi_group_hint;
            }
        }
        ClassicTab::ManualInputs => {
            if let Some(r) = state.classic.manual_inputs.get_mut(sel.idx) {
                r.lca_equivalent = flow.name.clone();
                r.lca_flow_id = Some(flow.id.clone());
                r.lca_group = flow.gabi_group_hint;
            }
        }
        ClassicTab::ManualOutputs => {
            if let Some(r) = state.classic.manual_outputs.get_mut(sel.idx) {
                r.lca_equivalent = flow.name.clone();
                r.lca_flow_id = Some(flow.id.clone());
                r.lca_group = flow.gabi_group_hint;
            }
        }
        ClassicTab::Indicators => {
            if let Some(ind) = state.classic.indicators.get_mut(sel.idx) {
                ind.lca_equivalent = flow.name.clone();
                ind.lca_flow_id = Some(flow.id.clone());
                ind.lca_group = LcaGroup::ToNature;
            }
        }
        ClassicTab::Normalized => {}
    }
    state.classic_mapping_dirty = true;
    state.classic.normalized_valid = false;
    state.status_message = format!("Mapped row to '{}'", flow.name);
}

// ─────────────────────────────────────────────────────────────────────────────
// Part B Renderer: Beta IPC / LCIA `LCA` (`49.4 Figs. 7–40`)
// ─────────────────────────────────────────────────────────────────────────────

fn render_beta_lca(
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    project: &SimProject,
    state: &mut LcaWindowState,
) {
    let is_custom = matches!(state.beta_sheet, BetaSheetTab::Custom(_));
    let can_convert_to_ind = state
        .beta_selection
        .and_then(|s| {
            let list = if s.is_output { &state.beta.outputs } else { &state.beta.inputs };
            list.get(s.stream_idx).map(|r| {
                s.species_idx.is_none()
                    && !r.is_manual
                    && !r.converted_to_indicators
                    && r.flow_mapping.is_none()
                    && !r.species.is_empty()
            })
        })
        .unwrap_or(false);
    let can_convert_back = state
        .beta_selection
        .and_then(|s| {
            let list = if s.is_output { &state.beta.outputs } else { &state.beta.inputs };
            list.get(s.stream_idx).map(|r| r.converted_to_indicators)
        })
        .unwrap_or(false);

    // 1. 8-Group Beta Ribbon (`Modes`, `Streams`, `Custom Sheets`, `External Sheet`, `Mapping`, `Calculation Methods`, `Results`, `Info`)
    ui.horizontal_wrapped(|ui| {
        ribbon_group(ui, "Modes", |ui| {
            ui.menu_button(
                RichText::new("LCA\nopenLCA ▾").size(10.5).strong(),
                |ui| {
                    if ui
                        .selectable_label(state.beta.mode == BetaLcaMode::UnitDatabase, "Unit database")
                        .clicked()
                    {
                        state.beta.mode = BetaLcaMode::UnitDatabase;
                        let _ = state.beta.probe_openlca_ipc();
                        state.status_message = state.beta.ipc_status_note.clone();
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(state.beta.mode == BetaLcaMode::LciDatabase, "LCI database")
                        .clicked()
                    {
                        state.beta.mode = BetaLcaMode::LciDatabase;
                        let _ = state.beta.probe_openlca_ipc();
                        state.status_message = state.beta.ipc_status_note.clone();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("IPC:");
                        ui.add(egui::TextEdit::singleline(&mut state.beta.ipc_endpoint).desired_width(110.0));
                    });
                    if ui.button("Connect / Probe openLCA IPC").clicked() {
                        let _ = state.beta.probe_openlca_ipc();
                        state.status_message = state.beta.ipc_status_note.clone();
                        ui.close_menu();
                    }
                },
            );
        });

        ribbon_group(ui, "Streams", |ui| {
            let io_tab = matches!(state.beta_sheet, BetaSheetTab::Inputs | BetaSheetTab::Outputs);
            if ribbon_tile_button(ui, "⊕", "Add Manual\nStream", false, io_tab).clicked() {
                let is_out = matches!(state.beta_sheet, BetaSheetTab::Outputs);
                let idx = state.beta.add_manual_stream(is_out);
                state.beta_selection = Some(BetaRowSelection {
                    is_output: is_out,
                    stream_idx: idx,
                    species_idx: None,
                    excel_cell_ref: ['B', '5'],
                    excel_row_num: 5 + idx,
                });
            }
            if ribbon_tile_button(ui, "⊖", "Delete Manual\nStream", false, io_tab).clicked() {
                let res = if let Some(sel) = state.beta_selection {
                    state.beta.delete_manual_stream(sel.is_output, sel.stream_idx)
                } else {
                    Err("Manual stream is not selected".to_string())
                };
                if let Err(e) = res {
                    state.beta_error_modal = Some(e);
                } else {
                    state.beta_selection = None;
                }
            }
            let eye_lbl = if state.beta.show_stream_content {
                "Hide Stream\nContent"
            } else {
                "Show Stream\nContent"
            };
            if ribbon_tile_button(ui, "👁", eye_lbl, !state.beta.show_stream_content, io_tab).clicked() {
                state.beta.show_stream_content = !state.beta.show_stream_content;
            }
            if ribbon_tile_button(ui, "⇄", "Convert Stream\nto Indicators", false, can_convert_to_ind).clicked() {
                if let Some(sel) = state.beta_selection {
                    if let Err(e) = state.beta.convert_stream_to_indicators(sel.is_output, sel.stream_idx) {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
            if ribbon_tile_button(ui, "↩", "Convert Indicators\nback to Stream", false, can_convert_back).clicked() {
                if let Some(sel) = state.beta_selection {
                    if let Err(e) = state.beta.convert_indicators_back_to_stream(sel.is_output, sel.stream_idx) {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
        });

        ribbon_group(ui, "Custom Sheets", |ui| {
            if ribbon_tile_button(ui, "📄+", "Add Custom\nSheet", false, true).clicked() {
                state.rename_sheet_input = format!("Custom {}", state.beta.custom_sheets.len() + 1);
                state.rename_sheet_creating_new = true;
                state.show_rename_sheet_modal = true;
            }
            if ribbon_tile_button(ui, "📄−", "Delete Custom\nSheet", false, is_custom).clicked() {
                if let BetaSheetTab::Custom(idx) = state.beta_sheet {
                    state.beta.delete_custom_sheet(idx);
                    state.beta_sheet = BetaSheetTab::Inputs;
                }
            }
            let can_rename = match state.beta_sheet {
                BetaSheetTab::Custom(idx) => state
                    .beta
                    .custom_sheets
                    .get(idx)
                    .map(|s| !s.is_saved_snapshot && !s.is_external_import)
                    .unwrap_or(false),
                _ => false,
            };
            if ribbon_tile_button(ui, "✏", "Rename Custom\nSheet", false, can_rename).clicked() {
                if let BetaSheetTab::Custom(idx) = state.beta_sheet {
                    state.rename_sheet_input = state.beta.custom_sheets[idx].name.clone();
                    state.rename_sheet_creating_new = false;
                    state.show_rename_sheet_modal = true;
                }
            }
        });

        ribbon_group(ui, "External Sheet", |ui| {
            if ribbon_tile_button(ui, "📥", "Import External\nSheet", false, true).clicked() {
                state.ext_selected_tables.clear();
                state.show_ext_import_modal = true;
            }
            if ribbon_tile_button(ui, "XLS", "Export to\nExcel", false, true).clicked() {
                let bytes = SoryntelApplication::sim_lca_beta_xlsx(&state.beta);
                let out_path = "/tmp/LCA_exports.xlsx";
                let _ = crate::storage::write_atomic(out_path, &bytes);
                state.export_last_saved_path = Some(out_path.to_string());
                state.export_modal_title = "Export to Excel Succeeded".to_string();
                state.export_modal_body = format!(
                    "Saved workbook to {out_path} ({} bytes) including Inputs, Outputs, Results, and {} custom/saved/external sheets.",
                    bytes.len(),
                    state.beta.custom_sheets.len()
                );
                state.show_export_modal = true;
            }
        });

        ribbon_group(ui, "Mapping", |ui| {
            let can_add_map = state.beta_selection.is_some() && state.beta_selected_flow_id.is_some();
            if ribbon_tile_button(ui, "⊕", "Add\nMapping", false, can_add_map).clicked() {
                if let (Some(sel), Some(fid)) = (state.beta_selection, state.beta_selected_flow_id.clone()) {
                    if let Err(e) = state
                        .beta
                        .add_mapping(sel.is_output, sel.stream_idx, sel.species_idx, &fid)
                    {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
            if ribbon_tile_button(ui, "⊖", "Delete\nMapping", false, state.beta_selection.is_some()).clicked() {
                if let Some(sel) = state.beta_selection {
                    state.beta.delete_mapping(sel.is_output, sel.stream_idx, sel.species_idx);
                }
            }
        });

        ribbon_group(ui, "Calculation Methods", |ui| {
            if ribbon_tile_button(ui, "⊕", "Add Impact\nCategory", false, state.beta_selected_method_cat.is_some())
                .clicked()
            {
                if let Some((mid, cid)) = state.beta_selected_method_cat.clone() {
                    if let Err(e) = state.beta.add_impact_category(&mid, &cid) {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
            if ribbon_tile_button(
                ui,
                "⊖",
                "Delete Impact\nCategory",
                false,
                !state.beta.selected_categories.is_empty(),
            )
            .clicked()
            {
                if let Some((_, cid)) = state.beta_selected_method_cat.clone() {
                    state.beta.delete_impact_category(&cid);
                } else if let Some(last) = state.beta.selected_categories.last().cloned() {
                    state.beta.delete_impact_category(&last.id);
                }
            }
        });

        ribbon_group(ui, "Results", |ui| {
            let eye_res = if state.beta.show_result_content {
                "Hide Result\nContent"
            } else {
                "Show Result\nContent"
            };
            if ribbon_tile_button(ui, "👁", eye_res, !state.beta.show_result_content, true).clicked() {
                state.beta.show_result_content = !state.beta.show_result_content;
            }
            if ribbon_tile_button(ui, "📊", "Calculate\nResults", false, true).clicked() {
                match SoryntelApplication::sim_lca_beta_calculate(&mut state.beta) {
                    Ok(()) => {
                        state.beta_sheet = BetaSheetTab::Results;
                        state.status_message = format!(
                            "Calculated {} LCIA category results via {}",
                            state.beta.results.len(),
                            state.beta.ipc_status_note
                        );
                    }
                    Err(e) => state.beta_error_modal = Some(e),
                }
            }
            if ribbon_tile_button(
                ui,
                "💾+",
                "Save Results\nto Custom sheet",
                false,
                !state.beta.results.is_empty(),
            )
            .clicked()
            {
                match state.beta.save_results_to_custom_sheet() {
                    Ok(nm) => state.status_message = format!("Saved frozen snapshot '{nm}'"),
                    Err(e) => state.beta_error_modal = Some(e),
                }
            }
        });

        ribbon_group(ui, "Info", |ui| {
            if ribbon_tile_button(ui, "❓", "Help", false, true).clicked() {
                state.show_help_modal = true;
            }
            if ribbon_tile_button(ui, "⚙", "Unit Proc.", false, !state.beta.synthetic_processes.is_empty())
                .on_hover_text("Inspect openLCA Synthetic Unit Processes (1.00000 kg / 3.6 MJ)")
                .clicked()
            {
                state.show_synth_processes_modal = true;
            }
        });
    });

    ui.separator();

    // 2. Excel-style Formula / Cell Reference Bar (`[CellRef ▼] [...]`)
    ui.horizontal(|ui| {
        ui.add(
            egui::Button::new(format!(" {} ▾ ", state.beta_cell_ref_label))
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(180, 190, 204))),
        );
        ui.add(
            egui::TextEdit::singleline(&mut state.beta_formula_text)
                .desired_width(ui.available_width() - 10.0),
        );
    });
    ui.separator();

    // 3. Split Grid (~66%) & Right Dock `Flows and Methods` (~34%)
    let avail = ui.available_size();
    let grid_w = (avail.x * 0.65).max(480.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui(egui::vec2(grid_w, avail.y - 46.0), |ui| {
            ui.vertical(|ui| {
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    match state.beta_sheet.clone() {
                        BetaSheetTab::Inputs => render_beta_io_excel_grid(ui, state, false),
                        BetaSheetTab::Outputs => render_beta_io_excel_grid(ui, state, true),
                        BetaSheetTab::Results => render_beta_results_excel_grid(ui, state),
                        BetaSheetTab::Custom(idx) => render_beta_custom_grid(ui, state, idx),
                    }
                });
            });
        });

        ui.separator();
        ui.vertical(|ui| {
            render_beta_right_dock(ui, state);
        });
    });

    // Bottom Sheet Tabs (`◄ ► | Inputs | Outputs | Results | Saved... | Custom...`)
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new("◄ ► │").small().color(Color32::GRAY));
        for (tab, lbl) in [
            (BetaSheetTab::Inputs, "Inputs"),
            (BetaSheetTab::Outputs, "Outputs"),
            (BetaSheetTab::Results, "Results"),
        ] {
            if ui.selectable_label(state.beta_sheet == tab, lbl).clicked() {
                state.beta_sheet = tab;
            }
        }
        for i in 0..state.beta.custom_sheets.len() {
            let lbl = state.beta.custom_sheets[i].name.clone();
            if ui
                .selectable_label(state.beta_sheet == BetaSheetTab::Custom(i), lbl)
                .clicked()
            {
                state.beta_sheet = BetaSheetTab::Custom(i);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Sync from Flowsheet").clicked() {
                state.sync_from_project(project);
                state.status_message = "Synced live boundary streams from flowsheet".to_string();
            }
            ui.label(RichText::new(&state.beta.ipc_status_note).small().color(Color32::GRAY));
        });
    });
}

fn excel_header_row(ui: &mut egui::Ui, letters: &[&str]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let corner = egui::Frame::none()
            .fill(EXCEL_HDR_GRAY)
            .stroke(Stroke::new(0.8, EXCEL_GRID_LINE))
            .inner_margin(egui::Margin::symmetric(4.0, 2.0));
        corner.show(ui, |ui| {
            ui.set_width(26.0);
            ui.label("");
        });
        for (&ltr, &w) in letters
            .iter()
            .zip([135.0_f32, 82.0, 70.0, 52.0, 155.0, 175.0, 185.0, 62.0].iter().cycle())
        {
            corner.show(ui, |ui| {
                ui.set_width(w);
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new(ltr).size(11.0).color(Color32::from_rgb(51, 65, 85)));
                });
            });
        }
    });
}

fn render_beta_io_excel_grid(ui: &mut egui::Ui, state: &mut LcaWindowState, is_output: bool) {
    let cols_w = [135.0, 82.0, 70.0, 52.0, 155.0, 175.0, 185.0, 62.0];
    let col_letters = ["B", "C", "D", "E", "F", "G", "H", "I"];
    excel_header_row(ui, &col_letters);

    let paint_band = |ui: &mut egui::Ui, row_num: usize, fill: Color32, col_idx: usize, add_fn: Box<dyn FnOnce(&mut egui::Ui)>| {
        if col_idx == 0 {
            egui::Frame::none()
                .fill(EXCEL_HDR_GRAY)
                .stroke(Stroke::new(0.6, EXCEL_GRID_LINE))
                .inner_margin(egui::Margin::symmetric(4.0, 3.0))
                .show(ui, |ui| {
                    ui.set_width(26.0);
                    ui.label(RichText::new(row_num.to_string()).size(10.5));
                });
        }
        egui::Frame::none()
            .fill(fill)
            .stroke(Stroke::new(0.6, EXCEL_GRID_LINE))
            .inner_margin(egui::Margin::symmetric(4.0, 3.0))
            .show(ui, |ui| {
                ui.set_width(cols_w[col_idx]);
                add_fn(ui);
            });
    };

    // Rows 1–3 solid blue header (`Stream | | Amount | Units | LCA Flowtype | Flow Mapping | Provider | Location`)
    let headers = ["Stream", "", "Amount", "Units", "LCA Flowtype", "Flow Mapping", "Provider", "Location"];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (ci, h) in headers.iter().enumerate() {
            let text = (*h).to_string();
            paint_band(ui, 2, BLUE_HEADER_FILL, ci, Box::new(move |ui| {
                ui.label(RichText::new(text).strong().size(11.0).color(Color32::WHITE));
            }));
        }
    });

    let rows_len = if is_output {
        state.beta.outputs.len()
    } else {
        state.beta.inputs.len()
    };

    let flow_type_opts = if is_output {
        LcaFlowType::output_options()
    } else {
        LcaFlowType::input_options()
    };

    let mut excel_row_cursor = 5usize;
    let mut deferred_flowtype: Option<(usize, Option<usize>, LcaFlowType)> = None;
    let mut deferred_loc_prov: Option<(usize, Option<usize>, Option<String>, Option<String>)> = None;

    for i in 0..rows_len {
        let mut row_snapshot: BetaStreamRow = if is_output {
            state.beta.outputs[i].clone()
        } else {
            state.beta.inputs[i].clone()
        };

        let is_selected = state.beta_selection.map(|s| s.is_output == is_output && s.stream_idx == i && s.species_idx.is_none()).unwrap_or(false);
        let row_fill = if is_selected {
            Color32::from_rgb(134, 179, 224)
        } else {
            BLUE_STREAM_FILL
        };
        let cur_excel_row = excel_row_cursor;

        // Main stream row (medium blue bold, white Flow Mapping cell)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Col B: Stream
            let name_click = paint_cell_box(ui, cur_excel_row, cols_w[0], row_fill, true, |ui| {
                if row_snapshot.is_manual {
                    if ui
                        .add(egui::TextEdit::singleline(&mut row_snapshot.stream_name).desired_width(cols_w[0] - 8.0))
                        .changed()
                    {
                        if is_output {
                            state.beta.outputs[i].stream_name = row_snapshot.stream_name.clone();
                        } else {
                            state.beta.inputs[i].stream_name = row_snapshot.stream_name.clone();
                        }
                    }
                } else {
                    let resp = ui.selectable_label(is_selected, RichText::new(&row_snapshot.stream_name).strong().size(11.0));
                    if resp.clicked() {
                        select_beta_cell(state, is_output, i, None, 'B', cur_excel_row, row_snapshot.stream_name.clone());
                    }
                }
            });
            if name_click {
                select_beta_cell(state, is_output, i, None, 'B', cur_excel_row, row_snapshot.stream_name.clone());
            }

            // Col C: (blank on parent)
            paint_cell_box(ui, cur_excel_row, cols_w[1], row_fill, false, |ui| {
                ui.label(RichText::new(&row_snapshot.unit_name).size(10.0).color(Color32::from_rgb(51, 65, 85)));
            });

            // Col D: Amount
            paint_cell_box(ui, cur_excel_row, cols_w[2], row_fill, false, |ui| {
                if row_snapshot.is_manual {
                    let mut val = row_snapshot.amount;
                    if ui.add(egui::DragValue::new(&mut val).speed(10.0).max_decimals(2)).changed() {
                        if is_output {
                            state.beta.outputs[i].amount = val;
                        } else {
                            state.beta.inputs[i].amount = val;
                        }
                        if state.beta.results_calculated_once {
                            let _ = state.beta.calculate_results();
                        }
                    }
                } else {
                    ui.label(RichText::new(format!("{:.2}", row_snapshot.amount)).strong().size(11.0));
                }
            });

            // Col E: Units (`t/h`, `kW`, `MJ` editable on manual)
            paint_cell_box(ui, cur_excel_row, cols_w[3], row_fill, false, |ui| {
                if row_snapshot.is_manual {
                    let mut u = row_snapshot.unit.clone();
                    egui::ComboBox::from_id_salt(("beta_man_u", is_output, i))
                        .selected_text(&u)
                        .width(cols_w[3] - 6.0)
                        .show_ui(ui, |ui| {
                            for opt in ["t/h", "kW", "MJ"] {
                                if ui.selectable_label(u == opt, opt).clicked() {
                                    u = opt.to_string();
                                    if is_output {
                                        state.beta.outputs[i].unit = u.clone();
                                    } else {
                                        state.beta.inputs[i].unit = u.clone();
                                    }
                                }
                            }
                        });
                } else {
                    ui.label(RichText::new(&row_snapshot.unit).strong().size(11.0));
                }
            });

            // Col F: LCA Flowtype
            paint_cell_box(ui, cur_excel_row, cols_w[4], row_fill, false, |ui| {
                let mut ft = row_snapshot.flow_type;
                egui::ComboBox::from_id_salt(("beta_ft", is_output, i))
                    .selected_text(ft.as_str())
                    .width(cols_w[4] - 8.0)
                    .show_ui(ui, |ui| {
                        for &opt in flow_type_opts {
                            if ui.selectable_value(&mut ft, opt, opt.as_str()).changed() {
                                deferred_flowtype = Some((i, None, ft));
                            }
                        }
                    });
            });

            // Col G: White `Flow Mapping` cell (`Fig. 9`)
            let map_text = row_snapshot.flow_mapping.clone().unwrap_or_default();
            paint_cell_box(ui, cur_excel_row, cols_w[5], Color32::WHITE, false, |ui| {
                let resp = ui.selectable_label(
                    is_selected,
                    RichText::new(if map_text.is_empty() { " " } else { &map_text }).size(11.0),
                );
                if resp.clicked() {
                    select_beta_cell(state, is_output, i, None, 'G', cur_excel_row, map_text.clone());
                }
            });

            // Col H & I: Provider & Location (disabled on Elementary/Reference; >15 rule enforces Location first!)
            let needs_provider = matches!(row_snapshot.flow_type, LcaFlowType::ProductFlow | LcaFlowType::WasteFlow)
                && row_snapshot.flow_id.is_some();
            let q_opt = row_snapshot
                .flow_id
                .as_deref()
                .map(|fid| state.beta.query_providers(fid, row_snapshot.location.as_deref()));

            paint_cell_box(ui, cur_excel_row, cols_w[6], row_fill, false, |ui| {
                if needs_provider {
                    if let Some(ref q) = q_opt {
                        if q.require_location_first && row_snapshot.location.is_none() {
                            ui.label(
                                RichText::new("Select Location first (>15)")
                                    .italics()
                                    .size(10.0)
                                    .color(Color32::from_rgb(127, 29, 29)),
                            );
                        } else {
                            let cur_p = row_snapshot.provider.clone().unwrap_or_else(|| "No provider".to_string());
                            egui::ComboBox::from_id_salt(("beta_prov", is_output, i))
                                .selected_text(&cur_p)
                                .width(cols_w[6] - 8.0)
                                .show_ui(ui, |ui| {
                                    for p in &q.available_providers {
                                        if ui.selectable_label(&cur_p == p, p).clicked() {
                                            deferred_loc_prov = Some((
                                                i,
                                                None,
                                                row_snapshot.location.clone(),
                                                Some(p.clone()),
                                            ));
                                        }
                                    }
                                });
                        }
                    }
                }
            });

            paint_cell_box(ui, cur_excel_row, cols_w[7], row_fill, false, |ui| {
                if needs_provider {
                    if let Some(ref q) = q_opt {
                        let cur_l = row_snapshot.location.clone().unwrap_or_else(|| "Loc".to_string());
                        egui::ComboBox::from_id_salt(("beta_loc", is_output, i))
                            .selected_text(&cur_l)
                            .width(cols_w[7] - 6.0)
                            .show_ui(ui, |ui| {
                                for loc in &q.available_locations {
                                    if ui.selectable_label(&cur_l == loc, loc).clicked() {
                                        let first_p = state
                                            .beta
                                            .query_providers(row_snapshot.flow_id.as_deref().unwrap_or(""), Some(loc))
                                            .available_providers
                                            .first()
                                            .cloned();
                                        deferred_loc_prov = Some((i, None, Some(loc.clone()), first_p));
                                    }
                                }
                            });
                    }
                }
            });
        });

        excel_row_cursor += 1;

        // Phase + Species sub-rows (hidden visually when `Hide Stream Content`, while preserving original Excel row numbering `Fig. 10`!)
        let sp_count = row_snapshot.species.len();
        if state.beta.show_stream_content && !row_snapshot.species.is_empty() {
            // Phase header row (`Pyro Phase`)
            let phase_lbl = row_snapshot
                .species
                .first()
                .map(|s| s.phase_label.as_str())
                .unwrap_or("Pyro Phase");
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                paint_cell_box(ui, excel_row_cursor, cols_w[0], BLUE_SUB_FILL, true, |ui| {
                    ui.label(RichText::new(phase_lbl).italics().size(10.5));
                });
                paint_cell_box(ui, excel_row_cursor, cols_w[1], BLUE_SUB_FILL, false, |_ui| {});
                paint_cell_box(ui, excel_row_cursor, cols_w[2], BLUE_SUB_FILL, false, |ui| {
                    ui.label(RichText::new(format!("{:.2}", row_snapshot.amount)).size(10.5));
                });
                for ci in 3..8 {
                    paint_cell_box(ui, excel_row_cursor, cols_w[ci], BLUE_SUB_FILL, false, |_ui| {});
                }
            });
            excel_row_cursor += 1;

            for (sp_i, sp) in row_snapshot.species.iter().enumerate() {
                let sp_selected = state
                    .beta_selection
                    .map(|s| s.is_output == is_output && s.stream_idx == i && s.species_idx == Some(sp_i))
                    .unwrap_or(false);
                let sp_fill = if sp_selected {
                    Color32::from_rgb(191, 219, 254)
                } else {
                    BLUE_SUB_FILL
                };
                let sp_row_num = excel_row_cursor;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    paint_cell_box(ui, sp_row_num, cols_w[0], sp_fill, true, |_ui| {});
                    paint_cell_box(ui, sp_row_num, cols_w[1], sp_fill, false, |ui| {
                        let resp = ui.selectable_label(sp_selected, RichText::new(&sp.formula).size(10.5));
                        if resp.clicked() {
                            select_beta_cell(state, is_output, i, Some(sp_i), 'C', sp_row_num, sp.formula.clone());
                        }
                    });
                    paint_cell_box(ui, sp_row_num, cols_w[2], sp_fill, false, |ui| {
                        ui.label(RichText::new(format!("{:.2}", sp.amount)).size(10.5));
                    });
                    paint_cell_box(ui, sp_row_num, cols_w[3], sp_fill, false, |ui| {
                        ui.label(RichText::new(&sp.unit).size(10.0));
                    });
                    paint_cell_box(ui, sp_row_num, cols_w[4], sp_fill, false, |ui| {
                        if row_snapshot.converted_to_indicators {
                            let mut ft = sp.flow_type;
                            egui::ComboBox::from_id_salt(("beta_sp_ft", is_output, i, sp_i))
                                .selected_text(ft.as_str())
                                .width(cols_w[4] - 8.0)
                                .show_ui(ui, |ui| {
                                    for &opt in flow_type_opts {
                                        if ui.selectable_value(&mut ft, opt, opt.as_str()).changed() {
                                            deferred_flowtype = Some((i, Some(sp_i), ft));
                                        }
                                    }
                                });
                        }
                    });
                    paint_cell_box(
                        ui,
                        sp_row_num,
                        cols_w[5],
                        if row_snapshot.converted_to_indicators { Color32::WHITE } else { sp_fill },
                        false,
                        |ui| {
                            if row_snapshot.converted_to_indicators {
                                let txt = sp.flow_mapping.as_deref().unwrap_or("");
                                if ui
                                    .selectable_label(sp_selected, RichText::new(if txt.is_empty() { " " } else { txt }).size(10.5))
                                    .clicked()
                                {
                                    select_beta_cell(state, is_output, i, Some(sp_i), 'G', sp_row_num, txt.to_string());
                                }
                            }
                        },
                    );
                    paint_cell_box(ui, sp_row_num, cols_w[6], sp_fill, false, |ui| {
                        ui.label(RichText::new(sp.provider.as_deref().unwrap_or("")).size(10.0));
                    });
                    paint_cell_box(ui, sp_row_num, cols_w[7], sp_fill, false, |ui| {
                        ui.label(RichText::new(sp.location.as_deref().unwrap_or("")).size(10.0));
                    });
                });
                excel_row_cursor += 1;
            }
        } else if !row_snapshot.species.is_empty() {
            // Preserve exact original Excel row numbering when hidden (`Fig. 10`)!
            excel_row_cursor += 1 + sp_count;
        }
    }

    if let Some((si, sp, ft)) = deferred_flowtype {
        state.beta.set_flow_type(is_output, si, sp, ft);
    }
    if let Some((si, sp, loc, prov)) = deferred_loc_prov {
        state.beta.set_location_and_provider(is_output, si, sp, loc, prov);
    }
}

fn paint_cell_box(
    ui: &mut egui::Ui,
    row_num: usize,
    width: f32,
    fill: Color32,
    draw_row_hdr: bool,
    add_fn: impl FnOnce(&mut egui::Ui),
) -> bool {
    if draw_row_hdr {
        egui::Frame::none()
            .fill(EXCEL_HDR_GRAY)
            .stroke(Stroke::new(0.6, EXCEL_GRID_LINE))
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.set_width(26.0);
                ui.label(RichText::new(row_num.to_string()).size(10.0));
            });
    }
    let resp = egui::Frame::none()
        .fill(fill)
        .stroke(Stroke::new(0.6, EXCEL_GRID_LINE))
        .inner_margin(egui::Margin::symmetric(4.0, 2.0))
        .show(ui, |ui| {
            ui.set_width(width);
            add_fn(ui);
        });
    resp.response.clicked()
}

fn select_beta_cell(
    state: &mut LcaWindowState,
    is_output: bool,
    stream_idx: usize,
    species_idx: Option<usize>,
    col_letter: char,
    excel_row: usize,
    text: String,
) {
    state.beta_selection = Some(BetaRowSelection {
        is_output,
        stream_idx,
        species_idx,
        excel_cell_ref: [col_letter, '5'],
        excel_row_num: excel_row,
    });
    state.beta_cell_ref_label = format!("{col_letter}{excel_row}");
    state.beta_formula_text = text;
}

fn render_beta_results_excel_grid(ui: &mut egui::Ui, state: &mut LcaWindowState) {
    if state.beta.results.is_empty() {
        ui.label("Select Impact Categories on the right and click 'Calculate Results' in the Ribbon.");
        return;
    }

    let widths = [125.0, 145.0, 105.0, 110.0, 145.0, 195.0, 52.0, 78.0, 82.0, 48.0, 86.0, 145.0];
    let headers = [
        "Method",
        "Category",
        "Unit",
        "Stream Name",
        "Flow Mapping",
        "Provider",
        "Location",
        "Stream Amount",
        "Normalized Amounts",
        "Units",
        "Normalized Results",
        "Units",
    ];

    egui::Grid::new("beta_results_grid")
        .striped(false)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for (h, w) in headers.iter().zip(widths.iter()) {
                egui::Frame::none()
                    .fill(BLUE_HEADER_FILL)
                    .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_width(*w);
                        ui.label(RichText::new(*h).strong().size(10.5).color(Color32::WHITE));
                    });
            }
            ui.end_row();

            for cat in &state.beta.results {
                // Thick black border separator before each category (`Fig. 35`)
                let cat_cells = [
                    cat.method_name.clone(),
                    cat.category_name.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    "Total".to_string(),
                    String::new(),
                    format!("{:.2}", cat.grand_total),
                    format!("{} per t/h of {}", cat.result_unit, cat.reference_stream_name),
                ];
                for (c, w) in cat_cells.iter().zip(widths.iter()) {
                    egui::Frame::none()
                        .fill(BLUE_STREAM_FILL)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 41, 59)))
                        .inner_margin(egui::Margin::symmetric(4.0, 3.0))
                        .show(ui, |ui| {
                            ui.set_width(*w);
                            ui.label(RichText::new(c).strong().size(10.5));
                        });
                }
                ui.end_row();

                if !state.beta.show_result_content {
                    continue;
                }

                for ub in &cat.units {
                    let u_cells = [
                        "".to_string(),
                        "".to_string(),
                        ub.unit_name.clone(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "Total".to_string(),
                        "".to_string(),
                        format!("{:.2}", ub.unit_total),
                        "".to_string(),
                    ];
                    for (c, w) in u_cells.iter().zip(widths.iter()) {
                        egui::Frame::none()
                            .fill(BLUE_SUB_FILL)
                            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                            .show(ui, |ui| {
                                ui.set_width(*w);
                                ui.label(RichText::new(c).strong().size(10.5));
                            });
                    }
                    ui.end_row();

                    for (dir_lbl, lines, dir_total) in [
                        ("Inputs", &ub.inputs, ub.inputs_total),
                        ("Outputs", &ub.outputs, ub.outputs_total),
                    ] {
                        if lines.is_empty() {
                            continue;
                        }
                        let d_cells = [
                            "".to_string(),
                            "".to_string(),
                            "".to_string(),
                            dir_lbl.to_string(),
                            "".to_string(),
                            "".to_string(),
                            "".to_string(),
                            "".to_string(),
                            "Total".to_string(),
                            "".to_string(),
                            format!("{dir_total:.2}"),
                            "".to_string(),
                        ];
                        for (ci, (c, w)) in d_cells.iter().zip(widths.iter()).enumerate() {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(245, 249, 253))
                                .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                .show(ui, |ui| {
                                    ui.set_width(*w);
                                    let mut rt = RichText::new(c).size(10.5);
                                    if ci == 3 {
                                        rt = rt.italics().strong();
                                    } else {
                                        rt = rt.strong();
                                    }
                                    ui.label(rt);
                                });
                        }
                        ui.end_row();

                        for ln in lines {
                            let l_cells = [
                                "".to_string(),
                                "".to_string(),
                                "".to_string(),
                                ln.stream_name.clone(),
                                ln.flow_mapping.clone(),
                                ln.provider.clone(),
                                ln.location.clone(),
                                format!("{:.2}", ln.stream_amount),
                                format!("{:.2}", ln.normalized_amount),
                                ln.amount_unit.clone(),
                                if ln.is_reference_flow {
                                    String::new()
                                } else {
                                    format!("{:.2}", ln.normalized_result)
                                },
                                "".to_string(),
                            ];
                            for (c, w) in l_cells.iter().zip(widths.iter()) {
                                egui::Frame::none()
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(0.4, Color32::from_rgb(226, 232, 240)))
                                    .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.set_width(*w);
                                        let rt = if ln.is_reference_flow {
                                            RichText::new(c).italics().size(10.5)
                                        } else {
                                            RichText::new(c).size(10.5)
                                        };
                                        ui.label(rt);
                                    });
                            }
                            ui.end_row();
                        }
                    }
                }
            }
        });
}

fn render_beta_custom_grid(ui: &mut egui::Ui, state: &mut LcaWindowState, idx: usize) {
    let Some(sheet) = state.beta.custom_sheets.get_mut(idx) else {
        ui.label("Sheet not found.");
        return;
    };
    ui.label(RichText::new(&sheet.name).strong().size(13.0));
    ui.add_space(4.0);
    let editable = !sheet.is_saved_snapshot && !sheet.is_external_import;

    egui::Grid::new(("beta_custom_sheet", idx))
        .striped(true)
        .spacing([6.0, 3.0])
        .show(ui, |ui| {
            for h in &sheet.headers {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for r in &mut sheet.rows {
                for cell in r.iter_mut() {
                    if editable {
                        ui.add(egui::TextEdit::singleline(cell).desired_width(120.0));
                    } else {
                        ui.label(&*cell);
                    }
                }
                ui.end_row();
            }
        });
}

fn render_beta_right_dock(ui: &mut egui::Ui, state: &mut LcaWindowState) {
    ui.label(RichText::new("Flows and Methods").strong());
    ui.horizontal(|ui| {
        if ui
            .selectable_label(state.beta_dock_tab == BetaDockTab::Flows, "Flows")
            .clicked()
        {
            state.beta_dock_tab = BetaDockTab::Flows;
        }
        if ui
            .selectable_label(
                state.beta_dock_tab == BetaDockTab::ImpactMethods,
                "Impact Methods and Categories",
            )
            .clicked()
        {
            state.beta_dock_tab = BetaDockTab::ImpactMethods;
        }
    });
    ui.separator();

    // Shared `Enter text to search...` + Find + Clear (`Fig. 24`)
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.beta_find_query)
                .hint_text("Enter text to search...")
                .desired_width(175.0),
        );
        if ui.button("Find").clicked() {
            state.beta_active_find = state.beta_find_query.trim().to_ascii_lowercase();
        }
        if ui.button(crate::i18n::text("common.037")).clicked() {
            state.beta_find_query.clear();
            state.beta_active_find.clear();
        }
    });
    ui.separator();

    match state.beta_dock_tab {
        BetaDockTab::Flows => {
            let root_title = format!("Flows ({})", state.beta.active_db_name);
            ui.label(RichText::new(root_title).strong().size(11.0));
            let flows = state.beta.catalog_flows();
            let mut by_div: BTreeMap<String, Vec<LcaCatalogFlow>> = BTreeMap::new();
            for f in flows {
                let div = f
                    .category_path
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Other".to_string());
                by_div.entry(div).or_default().push(f);
            }

            let find_q = state.beta_active_find.clone();
            let mut double_clicked_flow: Option<String> = None;

            egui::ScrollArea::vertical()
                .id_salt("beta_flows_tree")
                .show(ui, |ui| {
                    for (div, items) in &by_div {
                        let any_match = !find_q.is_empty()
                            && items.iter().any(|it| it.name.to_ascii_lowercase().contains(&find_q));
                        let is_open = state.beta_expanded_folders.contains(div) || any_match;
                        ui.horizontal(|ui| {
                            if ui.small_button(if is_open { "−" } else { "+" }).clicked() {
                                if is_open {
                                    state.beta_expanded_folders.remove(div);
                                } else {
                                    state.beta_expanded_folders.insert(div.clone());
                                }
                            }
                            let mut lbl = RichText::new(div).size(11.0);
                            if any_match {
                                lbl = lbl.background_color(YELLOW_MATCH_FILL);
                            }
                            ui.label(lbl);
                        });
                        if is_open {
                            for item in items {
                                let matches_q = !find_q.is_empty()
                                    && item.name.to_ascii_lowercase().contains(&find_q);
                                let selected = state.beta_selected_flow_id.as_deref() == Some(&item.id);
                                let mut txt = RichText::new(format!("    {}", item.beta_display_label())).size(11.0);
                                if matches_q {
                                    txt = txt.background_color(YELLOW_MATCH_FILL);
                                }
                                let resp = ui.selectable_label(selected, txt);
                                if resp.clicked() {
                                    state.beta_selected_flow_id = Some(item.id.clone());
                                }
                                if resp.double_clicked() {
                                    state.beta_selected_flow_id = Some(item.id.clone());
                                    double_clicked_flow = Some(item.id.clone());
                                }
                            }
                        }
                    }
                });

            if let Some(fid) = double_clicked_flow {
                if let Some(sel) = state.beta_selection {
                    if let Err(e) = state
                        .beta
                        .add_mapping(sel.is_output, sel.stream_idx, sel.species_idx, &fid)
                    {
                        state.beta_error_modal = Some(e);
                    }
                }
            }
        }
        BetaDockTab::ImpactMethods => {
            let methods: Vec<LciaMethodSpec> = SoryntelApplication::sim_lca_methods();
            let find_q = state.beta_active_find.clone();
            let mut double_add: Option<(String, String)> = None;

            egui::ScrollArea::vertical()
                .id_salt("beta_methods_tree")
                .show(ui, |ui| {
                    for m in &methods {
                        let any_m = !find_q.is_empty()
                            && (m.name.to_ascii_lowercase().contains(&find_q)
                                || m.categories
                                    .iter()
                                    .any(|c| c.name.to_ascii_lowercase().contains(&find_q)));
                        let is_open = state.beta_expanded_folders.contains(&m.id) || any_m;
                        ui.horizontal(|ui| {
                            if ui.small_button(if is_open { "−" } else { "+" }).clicked() {
                                if is_open {
                                    state.beta_expanded_folders.remove(&m.id);
                                } else {
                                    state.beta_expanded_folders.insert(m.id.clone());
                                }
                            }
                            ui.label(RichText::new(&m.name).strong().size(11.0));
                        });
                        if is_open {
                            for cat in &m.categories {
                                let matches_q = !find_q.is_empty()
                                    && cat.name.to_ascii_lowercase().contains(&find_q);
                                let already_added = state
                                    .beta
                                    .selected_categories
                                    .iter()
                                    .any(|c| c.id == cat.id);
                                let selected = state.beta_selected_method_cat.as_ref()
                                    == Some(&(m.id.clone(), cat.id.clone()));
                                let marker = if already_added { "✔ " } else { "" };
                                let mut txt = RichText::new(format!("    {marker}{}", cat.name)).size(11.0);
                                if matches_q {
                                    txt = txt.background_color(YELLOW_MATCH_FILL);
                                }
                                let resp = ui.selectable_label(selected, txt);
                                if resp.clicked() {
                                    state.beta_selected_method_cat = Some((m.id.clone(), cat.id.clone()));
                                }
                                if resp.double_clicked() {
                                    double_add = Some((m.id.clone(), cat.id.clone()));
                                }
                            }
                        }
                    }
                });

            if let Some((mid, cid)) = double_add {
                if let Err(e) = state.beta.add_impact_category(&mid, &cid) {
                    state.beta_error_modal = Some(e);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Modals (Apply Mapping Changes, Process Info, Set Active DB, Flowsheet Tables, Rename, Synthetic Unit Processes, Help)
// ─────────────────────────────────────────────────────────────────────────────

fn render_shared_lca_modals(ctx: &egui::Context, project: &SimProject, state: &mut LcaWindowState) {
    // 1. "Question: Apply mapping changes ?" (`49.3 Fig. 16`)
    if state.classic_show_apply_modal {
        egui::Window::new("Question")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Apply mapping changes ?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        state.classic.refresh_indicators();
                        state.classic_mapping_dirty = false;
                        state.classic_show_apply_modal = false;
                        if let Some(next_tab) = state.classic_pending_tab.take() {
                            state.classic_tab = next_tab;
                        } else {
                            state.classic_show_mapping = false;
                        }
                    }
                    if ui.button("No").clicked() {
                        state.classic_mapping_dirty = false;
                        state.classic_show_apply_modal = false;
                        if let Some(next_tab) = state.classic_pending_tab.take() {
                            state.classic_tab = next_tab;
                        } else {
                            state.classic_show_mapping = false;
                        }
                    }
                });
            });
    }

    // 2. Process Information (`49.3 Fig. 18`)
    if state.show_process_info_modal {
        egui::Window::new(crate::i18n::text("common.099"))
            .collapsible(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new("Process").strong());
                    ui.horizontal(|ui| {
                        ui.label("Process Name:");
                        ui.text_edit_singleline(&mut state.process_info_draft.process_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Technology:   ");
                        ui.text_edit_singleline(&mut state.process_info_draft.technology);
                    });
                    ui.label("General Comment:");
                    ui.add(egui::TextEdit::multiline(&mut state.process_info_draft.general_comment).desired_rows(2));
                    ui.horizontal(|ui| {
                        ui.label("HSC Ver. (Last save):");
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut state.process_info_draft.hsc_ver_last_save));
                    });
                });
                ui.add_space(6.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new("Location").strong());
                    ui.horizontal(|ui| {
                        ui.label("Location:");
                        egui::ComboBox::from_id_salt("lca_proc_loc")
                            .selected_text(&state.process_info_draft.location)
                            .show_ui(ui, |ui| {
                                for c in ["FI", "DE", "EU-28", "GLO", "RoW", "US", "CN", "SE"] {
                                    ui.selectable_value(
                                        &mut state.process_info_draft.location,
                                        c.to_string(),
                                        c,
                                    );
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Location Comment:");
                        ui.text_edit_singleline(&mut state.process_info_draft.location_comment);
                    });
                });
                ui.add_space(6.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new("Developer").strong());
                    ui.horizontal(|ui| {
                        ui.label("Author:        ");
                        ui.text_edit_singleline(&mut state.process_info_draft.author);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Date:          ");
                        ui.text_edit_singleline(&mut state.process_info_draft.date);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Version Number:");
                        ui.text_edit_singleline(&mut state.process_info_draft.version_number);
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        state.classic.process_info = state.process_info_draft.clone();
                        state.show_process_info_modal = false;
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        state.show_process_info_modal = false;
                    }
                });
            });
    }

    // 3. Set Active OpenLCA Database (.zip JSON-LD loader) (`49.3.1`)
    if state.show_openlca_db_modal {
        egui::Window::new("Set Active Database (OpenLCA JSON-LD)")
            .collapsible(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label("Enter path to an uncompressed OpenLCA JSON-LD .zip archive:");
                ui.text_edit_singleline(&mut state.openlca_zip_path_input);
                ui.horizontal(|ui| {
                    if ui.button("Load JSON-LD .zip").clicked() {
                        match std::fs::read(&state.openlca_zip_path_input) {
                            Ok(bytes) => {
                                let p = state.openlca_zip_path_input.clone();
                                match state.classic.load_openlca_jsonld_zip_bytes(p, &bytes) {
                                    Ok(n) => {
                                        state.status_message = format!("Loaded {n} OpenLCA flows from JSON-LD archive");
                                        state.show_openlca_db_modal = false;
                                    }
                                    Err(e) => state.beta_error_modal = Some(e),
                                }
                            }
                            Err(e) => state.beta_error_modal = Some(format!("Failed to read file: {e}")),
                        }
                    }
                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        state.show_openlca_db_modal = false;
                    }
                });
            });
    }

    // 4. Rename / Add Custom Sheet (`49.4 Figs. 18–19`)
    if state.show_rename_sheet_modal {
        let title = if state.rename_sheet_creating_new {
            "Add Custom Sheet"
        } else {
            "Rename Sheet"
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Custom Sheet Name (cannot start with 'Saved', 'Norm', 'Ext'):");
                ui.text_edit_singleline(&mut state.rename_sheet_input);
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        let name = state.rename_sheet_input.clone();
                        let res = if state.rename_sheet_creating_new {
                            let r = state.beta.add_custom_sheet(name);
                            if r.is_ok() {
                                state.beta_sheet = BetaSheetTab::Custom(state.beta.custom_sheets.len() - 1);
                            }
                            r
                        } else if let BetaSheetTab::Custom(idx) = state.beta_sheet {
                            state.beta.rename_custom_sheet(idx, name)
                        } else {
                            Err("No custom sheet active.".to_string())
                        };
                        match res {
                            Ok(()) => state.show_rename_sheet_modal = false,
                            Err(e) => state.beta_error_modal = Some(e),
                        }
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        state.show_rename_sheet_modal = false;
                    }
                });
            });
    }

    // 5. External Sheet Importer (`Flowsheet Tables` modal, `49.4 Fig. 20`)
    if state.show_ext_import_modal {
        egui::Window::new("Flowsheet Tables")
            .collapsible(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✔ Apply and close").color(Color32::WHITE))
                                .fill(Color32::from_rgb(22, 163, 74)),
                        )
                        .clicked()
                    {
                        for &idx in &state.ext_selected_tables {
                            if let Some(st) = project.stream_table_sets.get(idx) {
                                let headers = vec![
                                    "Stream".to_string(),
                                    "Mass Flow (t/h)".to_string(),
                                    "Temp (°C)".to_string(),
                                    "Enthalpy (kW)".to_string(),
                                ];
                                let rows: Vec<Vec<String>> = project
                                    .streams
                                    .iter()
                                    .filter(|s| st.assigned_streams.is_empty() || st.assigned_streams.contains(&s.name_id))
                                    .map(|s| {
                                        vec![
                                            s.name_id.clone(),
                                            format!("{:.2}", s.mass_flow_t_h),
                                            format!("{:.1}", s.temp_c),
                                            format!("{:.2}", s.enthalpy_kw),
                                        ]
                                    })
                                    .collect();
                                let sheet_nm = state.beta.import_external_sheet(&st.name, headers, rows);
                                state.beta_sheet = BetaSheetTab::Custom(state.beta.custom_sheets.len() - 1);
                                state.status_message = format!("Imported external sheet '{sheet_nm}'");
                            }
                        }
                        if state.ext_selected_tables.is_empty() {
                            let sheet_nm = state.beta.import_external_sheet(
                                "Flowsheet Summary",
                                vec!["Stream".to_string(), "Mass (t/h)".to_string()],
                                project
                                    .streams
                                    .iter()
                                    .map(|s| vec![s.name_id.clone(), format!("{:.2}", s.mass_flow_t_h)])
                                    .collect(),
                            );
                            state.beta_sheet = BetaSheetTab::Custom(state.beta.custom_sheets.len() - 1);
                            state.status_message = format!("Imported '{sheet_nm}'");
                        }
                        state.show_ext_import_modal = false;
                    }
                    if ui.button("✖ Cancel").clicked() {
                        state.show_ext_import_modal = false;
                    }
                });
                ui.separator();
                if project.stream_table_sets.is_empty() {
                    ui.label("No custom Stream Tables defined — click Apply and close to import Flowsheet Summary.");
                } else {
                    for (idx, tbl) in project.stream_table_sets.iter().enumerate() {
                        let mut chk = state.ext_selected_tables.contains(&idx);
                        if ui
                            .checkbox(&mut chk, format!("{} ({})", tbl.name, tbl.view_mode.as_str()))
                            .changed()
                        {
                            if chk {
                                state.ext_selected_tables.insert(idx);
                            } else {
                                state.ext_selected_tables.remove(&idx);
                            }
                        }
                    }
                }
            });
    }

    // 6. Export Confirmation & Preview Modal (`Figs. 19, 21`)
    if state.show_export_modal {
        let mut open = state.show_export_modal;
        egui::Window::new(&state.export_modal_title)
            .open(&mut open)
            .default_size([620.0, 400.0])
            .show(ctx, |ui| {
                if let Some(p) = &state.export_last_saved_path {
                    ui.label(RichText::new(format!("Written to disk: {p}")).strong().color(Color32::from_rgb(22, 101, 52)));
                }
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.100")).clicked() {
                        ctx.copy_text(state.export_modal_body.clone());
                    }
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        state.show_export_modal = false;
                    }
                });
                ui.separator();
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut state.export_modal_body)
                            .code_editor()
                            .desired_width(590.0)
                            .desired_rows(16),
                    );
                });
            });
        state.show_export_modal = state.show_export_modal && open;
    }

    // 7. Synthetic OpenLCA Unit Processes Inspector (`49.4 Figs. 38–40`)
    if state.show_synth_processes_modal {
        let mut open = state.show_synth_processes_modal;
        egui::Window::new("openLCA Internal Unit Processes & Main Process (1.00000 kg / 3.6 MJ)")
            .open(&mut open)
            .default_size([640.0, 380.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for proc in &state.beta.synthetic_processes {
                        ui.label(
                            RichText::new(format!(
                                "{} (Synthetic Ref Flow: {} = {:.5} kg)",
                                proc.process_name, proc.synthetic_ref_flow, proc.synthetic_ref_amount_kg
                            ))
                            .strong(),
                        );
                        for (flow, prov, amt, unit) in &proc.exchanges {
                            ui.label(format!("    • {flow} [{prov}]: {amt:.5} {unit}"));
                        }
                        ui.add_space(4.0);
                    }
                });
            });
        state.show_synth_processes_modal = open;
    }

    // 8. Error Popup (`Fig. 12: "Manual stream is not selected"`, etc.)
    if let Some(msg) = state.beta_error_modal.clone() {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new(msg).color(Color32::from_rgb(185, 28, 28)));
                ui.add_space(6.0);
                if ui.button(crate::i18n::text("common.002")).clicked() {
                    state.beta_error_modal = None;
                }
            });
    }

    // 9. Quick Help Modal
    if state.show_help_modal {
        let mut open = state.show_help_modal;
        egui::Window::new("HSC Sim LCA — Quick Guide (Ch. 49)")
            .open(&mut open)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Classic Mode (49.3)").strong());
                ui.label("• Extracts blue Input & red Output boundary streams (excludes black internal streams).");
                ui.label("• 1-hour basis: t/h -> 1000 kg, kW -> 1 kWh (or Use Exergy -> exergy kWh).");
                ui.label("• Indicators support *, ?, # wildcards and per-source Counted check to prevent double counting.");
                ui.label("• Mark 1 Main Product -> Normalize -> export EcoSpold v1.0 XML, OpenLCA JSON-LD .zip, or 6-sheet Excel.");
                ui.add_space(6.0);
                ui.label(RichText::new("Beta IPC / LCIA Mode (49.4)").strong());
                ui.label("• Select a flow row -> assign LCA Flowtype -> map to P -/E - flows -> pick Location first when > 15 providers.");
                ui.label("• Mark 1 Reference Flow (Product) -> add Impact Categories from 1 parent Method -> Calculate Results.");
                ui.label("• Live updates automatically when flowsheet values change.");
            });
        state.show_help_modal = open;
    }

    // Keep unused deterministic_uuid helper referenced if needed? Already used cleanly in domain.
    let _ = deterministic_uuid;
}
