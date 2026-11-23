//! Module 23 — HSC Geo: Processing Mineral Liberation Data UI (Chapter 85).
//!
//! Full desktop implementation matching the HSC Geo Mineral Liberation workflow:
//! - Left `Geo Tools` accordion (`Mineral Database`: Catalogue / Find by Properties / Find by Composition /
//!   Database Viewer; `Calculations`: Modal / Mineral Liberation / Petrological)
//! - Ribbon groups `Mineral Setup` (`New Project ▾`, `Open Project`, `Save`, `Remove Size Class`) and `Stream`
//!   (`New`, `Open ▾`, `Save ▾`, `Close`, `Close All`, `Add`, `Subtract`, `Rebalance`, `Basic Binning`, `Pack`,
//!   `Missing?`, `Create Ref`, `Bin with Ref`, `Smooth`, `Clean`, `Filter`, `Randomize`, `Fill Gangue`, `+Lib`,
//!   `-Lib`, `Active Stream`, `Reference Stream`, `Graphs`, `Config`, `Global Config`)
//! - `Mineral Setup` workbook (`Mins` / `Size` / `Particles`) + `Mineral Group` Combine modal + `Select from DB`
//! - Multi-stream tabs with right-click context menu (`Copy`, `Rename`, `Save XLS`, `Save XML`, `Set Active`, `Set Ref`, `Close`)
//! - 4 Stream sheets (`Mineral Composition`, `Particles`, `Log`, `Modal`) + `Graphs` window (7 chart/table modes).

use eframe::egui;
use soryntel_thermo::geo_liberation::{
    build_ec_graph, build_ed_graph, build_lib_graph, build_lib_xy_graph, build_lim_graph,
    build_mc_graph, build_md_graph, build_psd_graph, export_minsetup_xlsx, export_stream_xlsx,
    import_particles_csv_into_size_class, run_add_or_subtract_stream, run_adjust_liberation,
    run_basic_binning, run_bin_with_ref, run_clean, run_extrapolate_missing, run_fill_gangue,
    run_filter, run_pack, run_randomize, run_rebalance, run_smooth, search_geo_mineral_catalog,
    FilterProperty, FilterRelation, GeoMineralCatalogEntry, HscStreamPackage, LiberationBasis,
    LiberationStream, LiberationTableData, MineralGroupDef, MineralSetup, SizeClassDef,
    StreamConfig,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::excel_grid::{header_cell, row_num_cell, widget_cell, EXCEL_GRID_BORDER};
use crate::geo_mineralogy_ui::{Geo84CrossAction, GeoMineralogyUiState};
use crate::ui_common::{draw_rotated_y_title, format_tick, nice_step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeoToolView {
    Catalogue,
    FindByProperties,
    FindByComposition,
    DatabaseViewer,
    ModalCalc,
    MineralLiberation,
    Petrological,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveWorkspaceTab {
    MineralSetup,
    Stream(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupSubTab {
    Mins,
    Size,
    ParticlesPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamSubTab {
    MineralComposition,
    Particles,
    Log,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphKind {
    Mc,
    Ec,
    Psd,
    Md,
    Ed,
    Lib,
    Lim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileModalAction {
    None,
    SaveMinSetupXlsx,
    OpenProjectJson,
    SaveStreamXlsx(usize),
    SaveStreamHsc(usize),
    OpenStreamHsc,
}

/// Interactive state for the `Mineral Group` (`Create -> Combine`) dialog (Screenshots 3 & 4).
struct CombineModalState {
    open: bool,
    existing: Vec<MineralGroupDef>,
    existing_selected: BTreeSet<usize>,
    new_groups: Vec<MineralGroupDef>,
    new_selected: Option<usize>,
    rename_open: bool,
    rename_buf: String,
    info_open: bool,
    info_group_idx: Option<usize>,
}

impl Default for CombineModalState {
    fn default() -> Self {
        Self {
            open: false,
            existing: Vec::new(),
            existing_selected: BTreeSet::new(),
            new_groups: Vec::new(),
            new_selected: None,
            rename_open: false,
            rename_buf: String::new(),
            info_open: false,
            info_group_idx: None,
        }
    }
}

pub struct GeoLiberationModuleState {
    active_geo_tool: GeoToolView,
    pub setup: MineralSetup,
    pub streams: Vec<LiberationStream>,
    active_workspace_tab: ActiveWorkspaceTab,
    setup_sub_tab: SetupSubTab,
    stream_sub_tab: StreamSubTab,
    pub active_stream_idx: Option<usize>,
    pub ref_stream_idx: Option<usize>,
    global_config: StreamConfig,

    // Particle table basis display
    particle_comp_basis: LiberationBasis,
    particle_page: usize,

    // Full Chapter 84 Mineralogical Calculations & Mineral Database state
    pub mineralogy: GeoMineralogyUiState,
    geo_profile: String,

    // Modals
    combine: CombineModalState,
    show_select_db_modal: bool,
    select_db_query: String,

    // Size class import modal (Screenshot 6)
    show_import_size_modal: bool,
    import_source_label: String,
    import_size_selected: usize,
    import_csv_path: String,

    // Rename stream modal
    rename_stream_idx: Option<usize>,
    rename_stream_buf: String,

    // Config & Global Config modal (§85.3.16)
    show_config_modal: bool,
    config_is_global: bool,
    config_backup: Option<StreamConfig>,
    show_binning_tree_modal: bool,
    tree_backup: Option<StreamConfig>,

    // Prerequisite Basic Binning confirmation dialog (Missing? / Rebalance)
    prereq_binning_action: Option<&'static str>,

    // Filter modal (§85.3.13 — Process Filter)
    show_filter_modal: bool,
    filter_by_size_fraction: bool,
    filter_size_no: usize,
    filter_delete_matching: bool,
    filter_prop_idx: usize,
    filter_rel: FilterRelation,
    filter_val_str: String,

    // Fill Gangue 3-step wizard (§85.3.14)
    show_fill_gangue_modal: bool,
    fg_step: u8,
    fg_size_no: usize,
    fg_mineral_idx: usize,
    fg_min_size_str: String,

    // +Lib / -Lib 2-step `Select many` wizard (§85.3.15)
    show_lib_adjust_modal: bool,
    lib_step: u8,
    lib_adjust_increase: bool,
    lib_selected_minerals: BTreeSet<usize>,
    lib_selected_sizes: BTreeSet<usize>,
    lib_step_pct_str: String,

    // Stream Properties / Graphs window (§85.4)
    show_graphs_modal: bool,
    graph_kind: GraphKind,
    graph_as_table: bool,
    graph_mineral_idx: usize,
    graph_element_idx: usize,
    graph_psd_cumulative: bool,
    graph_psd_log_x: bool,
    graph_each_size_100: bool,
    graph_lib_classes: usize,
    graph_lib_first_100: bool,
    graph_lim_by_element: bool,
    graph_lim_grade_on_x: bool,
    graph_y_auto_zoom: bool,
    graph_show_legend: bool,
    graph_hidden_series: BTreeSet<usize>,

    // File modal
    file_modal: FileModalAction,
    file_path_str: String,

    // Randomize seed counter & status
    rng_counter: u64,
    status_msg: Option<String>,
    status_err: Option<String>,
}

impl Default for GeoLiberationModuleState {
    fn default() -> Self {
        let setup = MineralSetup::default_pb_zn_project();
        // Preload realistic Feed (with outer classes 1 & 7 omitted so bulk=80% matching Ch. 85 screenshots!),
        // Concentrate, and Tail streams so every button (Basic Binning, Missing?, Rebalance, Add/Subtract, Graphs)
        // works immediately out-of-the-box!
        let feed = LiberationStream::generate_sample_stream("Feed", &setup, 21.2, 0.34, 0.28, true, 101);
        let conc = LiberationStream::generate_sample_stream("Concentrate", &setup, 6.4, 0.68, 0.22, false, 202);
        let tail = LiberationStream::generate_sample_stream("Tail", &setup, 14.8, 0.08, 0.14, false, 303);

        Self {
            active_geo_tool: GeoToolView::MineralLiberation,
            setup,
            streams: vec![feed, conc, tail],
            active_workspace_tab: ActiveWorkspaceTab::Stream(0),
            setup_sub_tab: SetupSubTab::Mins,
            stream_sub_tab: StreamSubTab::MineralComposition,
            active_stream_idx: Some(0),
            ref_stream_idx: Some(0),
            global_config: StreamConfig::default(),
            particle_comp_basis: LiberationBasis::WeightPct,
            particle_page: 0,
            mineralogy: GeoMineralogyUiState::default(),
            geo_profile: "HSC 9".into(),
            combine: CombineModalState::default(),
            show_select_db_modal: false,
            select_db_query: String::new(),
            show_import_size_modal: false,
            import_source_label: "MLA".into(),
            import_size_selected: 2,
            import_csv_path: String::new(),
            rename_stream_idx: None,
            rename_stream_buf: String::new(),
            show_config_modal: false,
            config_is_global: false,
            config_backup: None,
            show_binning_tree_modal: false,
            tree_backup: None,
            prereq_binning_action: None,
            show_filter_modal: false,
            filter_by_size_fraction: false,
            filter_size_no: 2,
            filter_delete_matching: true,
            filter_prop_idx: 0, // Size (matching Process Filter screenshot)
            filter_rel: FilterRelation::Gte,
            filter_val_str: "50".into(),
            show_fill_gangue_modal: false,
            fg_step: 0,
            fg_size_no: 2,
            fg_mineral_idx: 4,
            fg_min_size_str: "15".into(),
            show_lib_adjust_modal: false,
            lib_step: 0,
            lib_adjust_increase: true,
            lib_selected_minerals: BTreeSet::from([0, 1]),
            lib_selected_sizes: BTreeSet::from([2, 3, 4]),
            lib_step_pct_str: "15".into(),
            show_graphs_modal: false,
            graph_kind: GraphKind::Mc,
            graph_as_table: false,
            graph_mineral_idx: 0,
            graph_element_idx: 0,
            graph_psd_cumulative: true,
            graph_psd_log_x: true,
            graph_each_size_100: true,
            graph_lib_classes: 10,
            graph_lib_first_100: true,
            graph_lim_by_element: false,
            graph_lim_grade_on_x: true,
            graph_y_auto_zoom: false,
            graph_show_legend: true,
            graph_hidden_series: BTreeSet::new(),
            file_modal: FileModalAction::None,
            file_path_str: "/tmp/MinSetup.xlsx".into(),
            rng_counter: 1,
            status_msg: Some(
                "HSC Geo Mineral Liberation ready (Feed / Concentrate / Tail preloaded). Click Basic Binning, Missing?, Rebalance, or Graphs."
                    .into(),
            ),
            status_err: None,
        }
    }
}

impl GeoLiberationModuleState {
    fn clamp_indices(&mut self) {
        if let Some(i) = self.active_stream_idx {
            if i >= self.streams.len() {
                self.active_stream_idx = if self.streams.is_empty() { None } else { Some(0) };
            }
        }
        if let Some(i) = self.ref_stream_idx {
            if i >= self.streams.len() {
                self.ref_stream_idx = self.active_stream_idx;
            }
        }
        if let ActiveWorkspaceTab::Stream(i) = self.active_workspace_tab {
            if i >= self.streams.len() {
                self.active_workspace_tab = if let Some(a) = self.active_stream_idx {
                    ActiveWorkspaceTab::Stream(a)
                } else {
                    ActiveWorkspaceTab::MineralSetup
                };
            }
        }
    }

    fn open_combine_modal(&mut self) {
        self.combine.existing = if self.setup.raw_phases.is_empty() {
            self.setup.minerals.clone()
        } else {
            self.setup.raw_phases.clone()
        };
        self.combine.existing_selected.clear();
        self.combine.new_groups.clear();
        self.combine.new_selected = None;
        self.combine.rename_open = false;
        self.combine.info_open = false;
        self.combine.open = true;
    }

    fn create_new_stream(&mut self) {
        let mut n = self.streams.len() + 1;
        let mut name = format!("Stream {n}");
        while self.streams.iter().any(|s| s.name == name) {
            n += 1;
            name = format!("Stream {n}");
        }
        let st = LiberationStream::new_empty(name.clone(), &self.setup, self.global_config.clone());
        self.streams.push(st);
        let idx = self.streams.len() - 1;
        self.active_stream_idx = Some(idx);
        if self.ref_stream_idx.is_none() {
            self.ref_stream_idx = Some(idx);
        }
        self.active_workspace_tab = ActiveWorkspaceTab::Stream(idx);
        self.status_err = None;
        self.status_msg = Some(format!("Created new stream '{name}'. Use Open to load size fractions."));
    }

    fn copy_stream(&mut self, idx: usize) {
        if idx >= self.streams.len() {
            return;
        }
        let mut copy = self.streams[idx].clone();
        copy.name = format!("Copy of {}", copy.name);
        copy.push_log("Copied", format!("Copied from {}", self.streams[idx].name));
        self.streams.push(copy);
        let new_idx = self.streams.len() - 1;
        self.active_stream_idx = Some(new_idx);
        self.active_workspace_tab = ActiveWorkspaceTab::Stream(new_idx);
        self.status_err = None;
        self.status_msg = Some(format!("Created '{}'.", self.streams[new_idx].name));
    }

    fn close_stream(&mut self, idx: usize) {
        if idx >= self.streams.len() {
            return;
        }
        let removed = self.streams.remove(idx);
        if self.active_stream_idx == Some(idx) {
            self.active_stream_idx = if self.streams.is_empty() { None } else { Some(0) };
        } else if let Some(a) = self.active_stream_idx {
            if a > idx {
                self.active_stream_idx = Some(a - 1);
            }
        }
        if self.ref_stream_idx == Some(idx) {
            self.ref_stream_idx = self.active_stream_idx;
        } else if let Some(r) = self.ref_stream_idx {
            if r > idx {
                self.ref_stream_idx = Some(r - 1);
            }
        }
        self.clamp_indices();
        self.status_msg = Some(format!("Closed stream '{}'.", removed.name));
    }

    fn import_project_from_sem(&mut self, source: &str) {
        self.setup = MineralSetup::default_pb_zn_project();
        self.streams.clear();
        let st = LiberationStream::generate_sample_stream(
            &format!("{source} Stream 1"),
            &self.setup,
            21.2,
            0.30,
            0.26,
            true,
            777 + self.rng_counter,
        );
        self.rng_counter += 1;
        self.streams.push(st);
        self.active_stream_idx = Some(0);
        self.ref_stream_idx = Some(0);
        self.active_workspace_tab = ActiveWorkspaceTab::MineralSetup;
        self.setup_sub_tab = SetupSubTab::Mins;
        self.status_err = None;
        self.status_msg = Some(format!(
            "Imported {source} project ({} minerals, {} size classes). Use 'New Project -> Combine' to simplify groups.",
            self.setup.minerals.len(),
            self.setup.size_classes.len()
        ));
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = egui::Color32::from_rgb(238, 242, 248);
        vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(vis);

        self.clamp_indices();

        // Top Ribbon (Mineral Setup + Stream groups when in Mineral Liberation)
        if self.active_geo_tool == GeoToolView::MineralLiberation {
            self.ui_ribbon(ctx);
        }

        // Pull any status message from Ch. 84 mineralogy views into the unified status bar
        if let Some(m) = self.mineralogy.status_msg.take() {
            self.status_err = None;
            self.status_msg = Some(m);
        }
        if let Some(e) = self.mineralogy.status_err.take() {
            self.status_err = Some(e);
        }

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("geo_lib_footer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 238, 244))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tool_lbl = match self.active_geo_tool {
                        GeoToolView::Catalogue => "Catalogue (Ch. 84.4)",
                        GeoToolView::FindByProperties => "Find by Properties (Ch. 84.3)",
                        GeoToolView::FindByComposition => "Find by Composition (Ch. 84.5)",
                        GeoToolView::DatabaseViewer => "Database Viewer (Ch. 84.2)",
                        GeoToolView::ModalCalc => "Modal & Monte Carlo (Ch. 84.7)",
                        GeoToolView::Petrological => "Petrological Calculations (Ch. 84.6)",
                        GeoToolView::MineralLiberation => "Mineral Liberation (Ch. 85)",
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "HSC Geo — {tool_lbl}  |  DB Minerals: {}  |  Setup Minerals: {}",
                            self.mineralogy.db.minerals.len(),
                            self.setup.minerals.len()
                        ))
                        .size(10.5)
                        .color(egui::Color32::from_rgb(70, 80, 95)),
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

        // Left Navigation Panel: Geo Tools (Screenshot 1)
        self.ui_left_geo_tools(ctx);

        // Central Workspace
        let mut cross_action = Geo84CrossAction::None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| match self.active_geo_tool {
                GeoToolView::MineralLiberation => self.ui_mineral_liberation_workspace(ui),
                GeoToolView::Catalogue => cross_action = self.mineralogy.ui_catalogue(ui),
                GeoToolView::FindByProperties => cross_action = self.mineralogy.ui_find_by_properties(ui),
                GeoToolView::FindByComposition => cross_action = self.mineralogy.ui_find_by_composition(ui),
                GeoToolView::DatabaseViewer => cross_action = self.mineralogy.ui_database_viewer(ui),
                GeoToolView::ModalCalc => self.mineralogy.ui_modal(ui),
                GeoToolView::Petrological => self.mineralogy.ui_petrological(ui),
            });

        match cross_action {
            Geo84CrossAction::None => {}
            Geo84CrossAction::AddToMineralSetup(mdef) => {
                let name = mdef.mineral.clone();
                let code = mdef.code.clone();
                self.setup.minerals.push(mdef);
                self.status_err = None;
                self.status_msg = Some(format!("Added '{name}' ({code}) to Mineral Liberation Setup."));
            }
            Geo84CrossAction::JumpToCatalogue(name) => {
                self.mineralogy.cat_selected_mineral = name;
                self.mineralogy.cat_selected_record_id = None;
                self.active_geo_tool = GeoToolView::Catalogue;
            }
        }

        // All interactive modals (Ch. 85 + Ch. 84)
        self.ui_combine_modal(ctx);
        self.ui_select_from_db_modal(ctx);
        self.ui_import_size_class_modal(ctx);
        self.ui_rename_stream_modal(ctx);
        self.ui_prereq_binning_modal(ctx);
        self.ui_config_modal(ctx);
        self.ui_binning_tree_modal(ctx);
        self.ui_filter_modal(ctx);
        self.ui_fill_gangue_modal(ctx);
        self.ui_lib_adjust_modal(ctx);
        self.ui_graphs_modal(ctx);
        self.ui_file_modal(ctx);
        self.mineralogy.render_modals(ctx);
    }

    // ========================================================================
    // Left `Geo Tools` Sidebar (Screenshot 1)
    // ========================================================================

    fn ui_left_geo_tools(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("geo_tools_sidebar")
            .default_width(172.0)
            .min_width(150.0)
            .max_width(220.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Geo Tools")
                        .strong()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 110, 125)),
                );
                ui.add_space(4.0);

                egui::CollapsingHeader::new(egui::RichText::new("Mineral Database").strong().size(11.5))
                    .default_open(true)
                    .show(ui, |ui| {
                        geo_nav_btn(ui, "❖", "Catalogue", self.active_geo_tool == GeoToolView::Catalogue, || {
                            self.active_geo_tool = GeoToolView::Catalogue;
                        });
                        geo_nav_btn(ui, "🔍", "Find by Properties", self.active_geo_tool == GeoToolView::FindByProperties, || {
                            self.active_geo_tool = GeoToolView::FindByProperties;
                        });
                        geo_nav_btn(ui, "🧪", "Find by Composition", self.active_geo_tool == GeoToolView::FindByComposition, || {
                            self.active_geo_tool = GeoToolView::FindByComposition;
                        });
                        geo_nav_btn(ui, "📚", "Database Viewer", self.active_geo_tool == GeoToolView::DatabaseViewer, || {
                            self.active_geo_tool = GeoToolView::DatabaseViewer;
                        });
                    });

                ui.add_space(6.0);
                egui::CollapsingHeader::new(egui::RichText::new("Calculations").strong().size(11.5))
                    .default_open(true)
                    .show(ui, |ui| {
                        geo_nav_btn(ui, "📊", "Modal", self.active_geo_tool == GeoToolView::ModalCalc, || {
                            self.active_geo_tool = GeoToolView::ModalCalc;
                        });
                        geo_nav_btn(ui, "⚙", "Mineral Liberation", self.active_geo_tool == GeoToolView::MineralLiberation, || {
                            self.active_geo_tool = GeoToolView::MineralLiberation;
                        });
                        geo_nav_btn(ui, "🪨", "Petrological", self.active_geo_tool == GeoToolView::Petrological, || {
                            self.active_geo_tool = GeoToolView::Petrological;
                        });
                    });

                // Bottom Profile selector (exact HSC Geo sidebar: Images 3, 6, 9!)
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("geo_sidebar_profile")
                            .selected_text(&self.geo_profile)
                            .width(96.0)
                            .show_ui(ui, |ui| {
                                for p in ["HSC 9", "HSC 10", "Custom"] {
                                    if ui.selectable_value(&mut self.geo_profile, p.to_string(), p).clicked() {
                                        self.status_msg = Some(format!("Switched HSC Geo layout profile to '{p}'."));
                                    }
                                }
                            });
                        if ui
                            .add_sized([28.0, 22.0], egui::Button::new("⊞"))
                            .on_hover_text("Reset Geo panel layout defaults")
                            .clicked()
                        {
                            self.mineralogy.cat_show_general_props = true;
                            self.mineralogy.cat_show_variability = true;
                            self.mineralogy.cat_show_scatter = true;
                            self.mineralogy.cat_show_pie = true;
                            self.status_msg = Some("Reset HSC Geo layout panels to default.".into());
                        }
                    });
                    ui.label(egui::RichText::new("Profile:").size(10.0).color(egui::Color32::from_rgb(90, 100, 115)));
                    ui.separator();
                });
            });
    }

    // ========================================================================
    // Top Ribbon (`Mineral Setup` & `Stream` — Screenshots 2, 6, 7, 11, 14)
    // ========================================================================

    fn ui_ribbon(&mut self, ctx: &egui::Context) {
        let has_active = self.active_stream_idx.is_some();
        let has_ref = self.ref_stream_idx.is_some();

        egui::TopBottomPanel::top("geo_lib_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 240, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // GROUP 1: Mineral Setup (Screenshots 2, 3, 6)
                    ribbon_group(ui, "Mineral Setup", |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let np_btn = ui.add(
                                egui::Button::new(egui::RichText::new("✨ New Project ▾").size(10.5))
                                    .min_size(egui::vec2(106.0, 21.0))
                                    .fill(egui::Color32::from_rgb(248, 250, 253)),
                            );
                            let np_id = ui.make_persistent_id("geo_new_project_popup");
                            if np_btn.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(np_id));
                            }
                            egui::popup_below_widget(
                                ui,
                                np_id,
                                &np_btn,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| {
                                    ui.set_min_width(155.0);
                                    for src in ["MLA", "QemScan", "Inca", "Aztec", "Zeiss", "3D CT"] {
                                        if ui.button(format!("📂 Import {src}")).clicked() {
                                            self.import_project_from_sem(src);
                                            self.active_geo_tool = GeoToolView::MineralLiberation;
                                        }
                                    }
                                    ui.separator();
                                    if ui.button("📚 Select from DB").clicked() {
                                        self.show_select_db_modal = true;
                                        self.active_geo_tool = GeoToolView::MineralLiberation;
                                    }
                                    if ui.button("🔀 Combine").clicked() {
                                        self.open_combine_modal();
                                        self.active_geo_tool = GeoToolView::MineralLiberation;
                                    }
                                },
                            );

                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("📂 Open Project").size(10.5))
                                        .min_size(egui::vec2(106.0, 21.0))
                                        .fill(egui::Color32::from_rgb(248, 250, 253)),
                                )
                                .clicked()
                            {
                                self.file_path_str = "/tmp/hsc_geo_stream.hscstream".into();
                                self.file_modal = FileModalAction::OpenProjectJson;
                            }
                        });

                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("💾 Save").size(10.5))
                                        .min_size(egui::vec2(110.0, 21.0))
                                        .fill(egui::Color32::from_rgb(248, 250, 253)),
                                )
                                .on_hover_text("Save mineral set-up (MinSetup.xlsx)")
                                .clicked()
                            {
                                self.file_path_str = "/tmp/MinSetup.xlsx".into();
                                self.file_modal = FileModalAction::SaveMinSetupXlsx;
                            }
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("⊖ Remove Size Class").size(10.0))
                                        .min_size(egui::vec2(110.0, 21.0))
                                        .fill(egui::Color32::from_rgb(248, 250, 253)),
                                )
                                .clicked()
                            {
                                if self.setup.size_classes.len() > 1 {
                                    let removed = self.setup.size_classes.pop().unwrap();
                                    self.setup.renumber_size_classes();
                                    for st in &mut self.streams {
                                        st.particles.retain(|p| p.size_class_no != removed.no);
                                        st.size_bulk_wt_pct.truncate(self.setup.size_classes.len());
                                        st.sync_particle_flowrates(&self.setup);
                                    }
                                    self.status_msg = Some(format!(
                                        "Removed size class '{}'.",
                                        removed.label(&self.setup.size_unit)
                                    ));
                                }
                            }
                        });
                    });

                    // GROUP 2: Stream (Screenshots 6, 7, 11, 12, 13, 14)
                    ribbon_group(ui, "Stream", |ui| {
                        // New / Open ▾
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if small_rib_btn(ui, "✳ New", 68.0) {
                                self.create_new_stream();
                                self.active_geo_tool = GeoToolView::MineralLiberation;
                            }
                            let open_btn = ui.add(
                                egui::Button::new(egui::RichText::new("📂 Open ▾").size(10.0))
                                    .min_size(egui::vec2(68.0, 20.0))
                                    .fill(egui::Color32::from_rgb(248, 250, 253)),
                            );
                            let open_id = ui.make_persistent_id("geo_open_stream_popup");
                            if open_btn.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(open_id));
                            }
                            egui::popup_below_widget(
                                ui,
                                open_id,
                                &open_btn,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| {
                                    ui.set_min_width(148.0);
                                    if ui.button("📂 Open HSCStream").clicked() {
                                        self.file_path_str = "/tmp/Feed.hscstream".into();
                                        self.file_modal = FileModalAction::OpenStreamHsc;
                                    }
                                    ui.separator();
                                    for src in [
                                        "Read MLA",
                                        "Open XLS",
                                        "Import QemScan",
                                        "Import Inca",
                                        "Import Aztec",
                                        "Import Zeiss",
                                        "Import 3D CT",
                                    ] {
                                        if ui.button(format!("📄 {src}")).clicked() {
                                            if self.active_stream_idx.is_none() {
                                                self.create_new_stream();
                                            }
                                            self.import_source_label = src.to_string();
                                            self.import_size_selected =
                                                self.setup.size_classes.get(1).map(|s| s.no).unwrap_or(1);
                                            self.show_import_size_modal = true;
                                        }
                                    }
                                },
                            );
                        });

                        // Save ▾ / Close / Close All
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let save_btn = ui.add_enabled(
                                has_active,
                                egui::Button::new(egui::RichText::new("💾 Save ▾").size(10.0))
                                    .min_size(egui::vec2(68.0, 20.0))
                                    .fill(egui::Color32::from_rgb(248, 250, 253)),
                            );
                            let save_id = ui.make_persistent_id("geo_save_stream_popup");
                            if save_btn.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(save_id));
                            }
                            egui::popup_below_widget(
                                ui,
                                save_id,
                                &save_btn,
                                egui::PopupCloseBehavior::CloseOnClick,
                                |ui| {
                                    ui.set_min_width(140.0);
                                    if let Some(aidx) = self.active_stream_idx {
                                        let sname = self.streams[aidx].name.replace(' ', "_");
                                        if ui.button("💾 Save XLS (.xlsx)").clicked() {
                                            self.file_path_str = format!("/tmp/{sname}.xlsx");
                                            self.file_modal = FileModalAction::SaveStreamXlsx(aidx);
                                        }
                                        if ui.button("💾 Save HSCStream").clicked() {
                                            self.file_path_str = format!("/tmp/{sname}.hscstream");
                                            self.file_modal = FileModalAction::SaveStreamHsc(aidx);
                                        }
                                    }
                                },
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("Close").size(9.5))).clicked() {
                                    if let Some(aidx) = self.active_stream_idx {
                                        self.close_stream(aidx);
                                    }
                                }
                                if ui.add_enabled(!self.streams.is_empty(), egui::Button::new(egui::RichText::new("All").size(9.5))).on_hover_text("Close All Streams").clicked() {
                                    self.streams.clear();
                                    self.active_stream_idx = None;
                                    self.ref_stream_idx = None;
                                    self.active_workspace_tab = ActiveWorkspaceTab::MineralSetup;
                                }
                            });
                        });

                        // Add / Subtract
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if ui.add_enabled(has_active && has_ref, egui::Button::new(egui::RichText::new("⊕ Add").size(10.0)).min_size(egui::vec2(72.0, 20.0))).clicked() {
                                if let (Some(a), Some(r)) = (self.active_stream_idx, self.ref_stream_idx) {
                                    let ref_clone = self.streams[r].clone();
                                    run_add_or_subtract_stream(&mut self.streams[a], &ref_clone, &self.setup, false);
                                    self.status_msg = Some(format!("Added '{}' into '{}'.", ref_clone.name, self.streams[a].name));
                                }
                            }
                            if ui.add_enabled(has_active && has_ref, egui::Button::new(egui::RichText::new("⊖ Subtract").size(10.0)).min_size(egui::vec2(72.0, 20.0))).clicked() {
                                if let (Some(a), Some(r)) = (self.active_stream_idx, self.ref_stream_idx) {
                                    let ref_clone = self.streams[r].clone();
                                    run_add_or_subtract_stream(&mut self.streams[a], &ref_clone, &self.setup, true);
                                    self.status_msg = Some(format!("Subtracted '{}' from '{}'.", ref_clone.name, self.streams[a].name));
                                }
                            }
                        });

                        // Rebalance
                        if ui
                            .add_enabled(
                                has_active,
                                egui::Button::new(egui::RichText::new("⚖\nRebalance").size(10.0))
                                    .min_size(egui::vec2(62.0, 42.0))
                                    .fill(egui::Color32::from_rgb(248, 250, 253)),
                            )
                            .on_hover_text("Adjust particle mass proportions to match Modal sheet")
                            .clicked()
                        {
                            if let Some(a) = self.active_stream_idx {
                                if !self.streams[a].is_basic_binned {
                                    self.prereq_binning_action = Some("rebalance");
                                } else {
                                    match run_rebalance(&mut self.streams[a], &self.setup) {
                                        Ok((it, diff)) => {
                                            self.status_err = None;
                                            self.status_msg = Some(format!(
                                                "Rebalance complete ({it} iters, max diff {diff:.3}%)."
                                            ));
                                        }
                                        Err(e) => self.status_err = Some(e),
                                    }
                                }
                            }
                        }

                        // Basic Binning / Pack / Missing?
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            if ui
                                .add_enabled(has_active, egui::Button::new(egui::RichText::new("🧩 Basic Binning").size(9.5)).min_size(egui::vec2(96.0, 14.0)))
                                .on_hover_text("Apply basic binning of particles")
                                .clicked()
                            {
                                if let Some(a) = self.active_stream_idx {
                                    run_basic_binning(&mut self.streams[a], &self.setup);
                                    self.status_err = None;
                                    self.status_msg = Some(format!(
                                        "Basic Binning complete: {} particle classes.",
                                        self.streams[a].particles.len()
                                    ));
                                }
                            }
                            if ui
                                .add_enabled(has_active, egui::Button::new(egui::RichText::new("📦 Pack").size(9.5)).min_size(egui::vec2(96.0, 14.0)))
                                .on_hover_text("Combine binary/C/D classes until N Min is reached")
                                .clicked()
                            {
                                if let Some(a) = self.active_stream_idx {
                                    run_pack(&mut self.streams[a], &self.setup);
                                    self.status_msg = Some(format!(
                                        "Packed particles: {} classes remaining.",
                                        self.streams[a].particles.len()
                                    ));
                                }
                            }
                            if ui
                                .add_enabled(has_active, egui::Button::new(egui::RichText::new("❓ Missing?").size(9.5)).min_size(egui::vec2(96.0, 14.0)))
                                .on_hover_text("Defines the particles for undefined size classes")
                                .clicked()
                            {
                                if let Some(a) = self.active_stream_idx {
                                    if !self.streams[a].is_basic_binned {
                                        self.prereq_binning_action = Some("missing");
                                    } else {
                                        match run_extrapolate_missing(&mut self.streams[a], &self.setup) {
                                            Ok(0) => self.status_msg = Some("All size classes already have particles.".into()),
                                            Ok(n) => self.status_msg = Some(format!("Extrapolated {n} missing size class(es) via WLS-SVD.")),
                                            Err(e) => self.status_err = Some(e),
                                        }
                                    }
                                }
                            }
                        });

                        // Create Ref / Bin with Ref
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("◉ Create Ref").size(9.5)).min_size(egui::vec2(84.0, 20.0))).clicked() {
                                if let Some(a) = self.active_stream_idx {
                                    let mut ref_st = self.streams[a].clone();
                                    ref_st.name = format!("{} (Ref)", ref_st.name);
                                    run_pack(&mut ref_st, &self.setup);
                                    self.streams.push(ref_st);
                                    let r_idx = self.streams.len() - 1;
                                    self.ref_stream_idx = Some(r_idx);
                                    self.status_msg = Some(format!("Created packed Reference Stream '{}'.", self.streams[r_idx].name));
                                }
                            }
                            if ui.add_enabled(has_active && has_ref, egui::Button::new(egui::RichText::new("⌖ Bin with Ref").size(9.5)).min_size(egui::vec2(84.0, 20.0))).clicked() {
                                if let (Some(a), Some(r)) = (self.active_stream_idx, self.ref_stream_idx) {
                                    let ref_clone = self.streams[r].clone();
                                    run_bin_with_ref(&mut self.streams[a], &ref_clone, &self.setup);
                                    self.status_msg = Some(format!("Binned '{}' using classes from '{}'.", self.streams[a].name, ref_clone.name));
                                }
                            }
                        });

                        // Smooth / Clean / Filter
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            if ui.add_enabled(has_active && has_ref, egui::Button::new(egui::RichText::new("〰 Smooth").size(9.5)).min_size(egui::vec2(68.0, 14.0))).clicked() {
                                if let (Some(a), Some(r)) = (self.active_stream_idx, self.ref_stream_idx) {
                                    let ref_clone = self.streams[r].clone();
                                    run_smooth(&mut self.streams[a], &ref_clone, &self.setup);
                                    self.status_msg = Some("Smoothed binary partition curves against Reference.".into());
                                }
                            }
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("C Clean").size(9.5)).min_size(egui::vec2(68.0, 14.0))).clicked() {
                                if let Some(a) = self.active_stream_idx {
                                    let n = run_clean(&mut self.streams[a], &self.setup);
                                    self.status_msg = Some(format!("Clean removed {n} zero-NOP particles."));
                                }
                            }
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("⧩ Filter").size(9.5)).min_size(egui::vec2(68.0, 14.0))).clicked() {
                                self.show_filter_modal = true;
                            }
                        });

                        // Randomize / Fill Gangue / +Lib / -Lib
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("🎲 Randomize").size(9.5)).min_size(egui::vec2(84.0, 14.0))).clicked() {
                                if let Some(a) = self.active_stream_idx {
                                    self.rng_counter += 1;
                                    run_randomize(&mut self.streams[a], &self.setup, 999 + self.rng_counter);
                                    self.status_msg = Some("Randomized particle weights (+/- 15%).".into());
                                }
                            }
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("🧱 Fill Gangue").size(9.5)).min_size(egui::vec2(84.0, 14.0))).clicked() {
                                self.fg_step = 0;
                                self.show_fill_gangue_modal = true;
                            }
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("+Lib").size(9.5)).min_size(egui::vec2(40.0, 14.0))).clicked() {
                                    self.lib_step = 0;
                                    self.lib_adjust_increase = true;
                                    self.show_lib_adjust_modal = true;
                                }
                                if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("-Lib").size(9.5)).min_size(egui::vec2(40.0, 14.0))).clicked() {
                                    self.lib_step = 0;
                                    self.lib_adjust_increase = false;
                                    self.show_lib_adjust_modal = true;
                                }
                            });
                        });

                        // Active Stream & Reference Stream ComboBoxes (Screenshot 12)
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Active Stream").size(10.0));
                                let act_name = self
                                    .active_stream_idx
                                    .and_then(|i| self.streams.get(i))
                                    .map(|s| s.name.clone())
                                    .unwrap_or_else(|| "-".into());
                                egui::ComboBox::from_id_salt("geo_active_stream_cb")
                                    .width(96.0)
                                    .selected_text(act_name)
                                    .show_ui(ui, |ui| {
                                        for (i, s) in self.streams.iter().enumerate() {
                                            if ui.selectable_label(self.active_stream_idx == Some(i), &s.name).clicked() {
                                                self.active_stream_idx = Some(i);
                                                self.active_workspace_tab = ActiveWorkspaceTab::Stream(i);
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Ref. Stream   ").size(10.0));
                                let ref_name = self
                                    .ref_stream_idx
                                    .and_then(|i| self.streams.get(i))
                                    .map(|s| s.name.clone())
                                    .unwrap_or_else(|| "-".into());
                                egui::ComboBox::from_id_salt("geo_ref_stream_cb")
                                    .width(96.0)
                                    .selected_text(ref_name)
                                    .show_ui(ui, |ui| {
                                        for (i, s) in self.streams.iter().enumerate() {
                                            if ui.selectable_label(self.ref_stream_idx == Some(i), &s.name).clicked() {
                                                self.ref_stream_idx = Some(i);
                                            }
                                        }
                                    });
                            });
                        });

                        // Graphs / Config / Global Config (Screenshot 11 right)
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("📈 Graphs").size(9.5)).min_size(egui::vec2(82.0, 14.0))).clicked() {
                                self.show_graphs_modal = true;
                            }
                            if ui.add_enabled(has_active, egui::Button::new(egui::RichText::new("⚙ Config").size(9.5)).min_size(egui::vec2(82.0, 14.0))).clicked() {
                                self.config_is_global = false;
                                self.config_backup = self.active_stream_idx.map(|a| self.streams[a].config.clone());
                                self.show_config_modal = true;
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("🌐 Global Config").size(9.5)).min_size(egui::vec2(82.0, 14.0))).clicked() {
                                self.config_is_global = true;
                                self.config_backup = Some(self.global_config.clone());
                                self.show_config_modal = true;
                            }
                        });
                    });
                });
            });
    }

    // ========================================================================
    // Main `Mineral Liberation` Workspace (Tabs + Setup/Stream Sheets)
    // ========================================================================

    fn ui_mineral_liberation_workspace(&mut self, ui: &mut egui::Ui) {
        // Document Tabs Strip: Mineral Setup | Feed (Active, Reference) × | Concentrate × | Tail ×
        let mut close_req: Option<usize> = None;
        let mut ctx_action: Option<(usize, &'static str)> = None;

        ui.horizontal_wrapped(|ui| {
            let setup_act = self.active_workspace_tab == ActiveWorkspaceTab::MineralSetup;
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Mineral Setup").strong().size(11.5))
                        .fill(if setup_act {
                            egui::Color32::from_rgb(219, 234, 254)
                        } else {
                            egui::Color32::from_rgb(232, 236, 242)
                        }),
                )
                .clicked()
            {
                self.active_workspace_tab = ActiveWorkspaceTab::MineralSetup;
            }

            for i in 0..self.streams.len() {
                let is_a = self.active_stream_idx == Some(i);
                let is_r = self.ref_stream_idx == Some(i);
                let suffix = match (is_a, is_r) {
                    (true, true) => " (Active, Reference)",
                    (true, false) => " (Active)",
                    (false, true) => " (Reference)",
                    (false, false) => "",
                };
                let label = format!("{}{suffix}", self.streams[i].name);
                let tab_open = self.active_workspace_tab == ActiveWorkspaceTab::Stream(i);

                egui::Frame::none()
                    .fill(if tab_open {
                        egui::Color32::from_rgb(219, 234, 254)
                    } else {
                        egui::Color32::from_rgb(236, 240, 246)
                    })
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .rounding(3.0)
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let resp = ui.add(
                                egui::Label::new(egui::RichText::new(label).strong().size(11.0))
                                    .sense(egui::Sense::click()),
                            );
                            if resp.clicked() {
                                self.active_workspace_tab = ActiveWorkspaceTab::Stream(i);
                                self.active_stream_idx = Some(i);
                            }
                            // Right-click context menu matching Screenshot 13!
                            resp.context_menu(|ui| {
                                if ui.button("📋 Copy").clicked() {
                                    ctx_action = Some((i, "copy"));
                                    ui.close_menu();
                                }
                                if ui.button("✏ Rename").clicked() {
                                    ctx_action = Some((i, "rename"));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("💾 Save XLS").clicked() {
                                    ctx_action = Some((i, "save_xls"));
                                    ui.close_menu();
                                }
                                if ui.button("💾 Save XML (.HSCStream)").clicked() {
                                    ctx_action = Some((i, "save_xml"));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("✔ Set Active Stream").clicked() {
                                    ctx_action = Some((i, "set_active"));
                                    ui.close_menu();
                                }
                                if ui.button("◎ Set Reference Stream").clicked() {
                                    ctx_action = Some((i, "set_ref"));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("✖ Close").clicked() {
                                    ctx_action = Some((i, "close"));
                                    ui.close_menu();
                                }
                            });

                            if ui.small_button("×").on_hover_text("Close stream tab").clicked() {
                                close_req = Some(i);
                            }
                        });
                    });
            }
        });

        if let Some((idx, act)) = ctx_action {
            match act {
                "copy" => self.copy_stream(idx),
                "rename" => {
                    self.rename_stream_idx = Some(idx);
                    self.rename_stream_buf = self.streams[idx].name.clone();
                }
                "save_xls" => {
                    let sname = self.streams[idx].name.replace(' ', "_");
                    self.file_path_str = format!("/tmp/{sname}.xlsx");
                    self.file_modal = FileModalAction::SaveStreamXlsx(idx);
                }
                "save_xml" => {
                    let sname = self.streams[idx].name.replace(' ', "_");
                    self.file_path_str = format!("/tmp/{sname}.hscstream");
                    self.file_modal = FileModalAction::SaveStreamHsc(idx);
                }
                "set_active" => {
                    self.active_stream_idx = Some(idx);
                    self.active_workspace_tab = ActiveWorkspaceTab::Stream(idx);
                }
                "set_ref" => {
                    self.ref_stream_idx = Some(idx);
                }
                "close" => close_req = Some(idx),
                _ => {}
            }
        }
        if let Some(ci) = close_req {
            self.close_stream(ci);
        }

        ui.separator();

        match self.active_workspace_tab {
            ActiveWorkspaceTab::MineralSetup => self.ui_mineral_setup_sheets(ui),
            ActiveWorkspaceTab::Stream(idx) if idx < self.streams.len() => {
                self.ui_stream_sheets(ui, idx);
            }
            _ => {
                ui.label("No stream open. Click 'New' or 'Open' in the Stream ribbon group.");
            }
        }
    }

    // ========================================================================
    // `Mineral Setup` Sheets (`Mins` | `Size` | `Particles` — Screenshots 2 & 5)
    // ========================================================================

    fn ui_mineral_setup_sheets(&mut self, ui: &mut egui::Ui) {
        let body_h = (ui.available_height() - 30.0).max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), body_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| match self.setup_sub_tab {
                SetupSubTab::Mins => self.ui_setup_mins_sheet(ui),
                SetupSubTab::Size => self.ui_setup_size_sheet(ui),
                SetupSubTab::ParticlesPreview => {
                    ui.strong("Global Particle Classification Defaults");
                    ui.label(format!(
                        "Lib. Threshold: {}%   |   Tolerance: {}%   |   Binary classes: {}   |   Basis: {}   |   N min: {}",
                        self.global_config.lib_threshold,
                        self.global_config.tolerance,
                        self.global_config.binary_classes,
                        self.global_config.classif_basis.label(),
                        self.global_config.n_min
                    ));
                    ui.add_space(6.0);
                    if ui.button("⚙ Open Global Config...").clicked() {
                        self.config_is_global = true;
                        self.show_config_modal = true;
                    }
                    if ui.button("🌲 Preview Binning Particle Tree...").clicked() {
                        self.show_binning_tree_modal = true;
                    }
                }
            },
        );

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⏮ ◀ ▶ ⏭").size(9.5).color(egui::Color32::GRAY));
            for (st, lbl) in [
                (SetupSubTab::Mins, "Mins"),
                (SetupSubTab::Size, "Size"),
                (SetupSubTab::ParticlesPreview, "Particles"),
            ] {
                let active = self.setup_sub_tab == st;
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(lbl).strong().size(11.0))
                            .fill(if active { egui::Color32::WHITE } else { egui::Color32::from_rgb(230, 234, 242) }),
                    )
                    .clicked()
                {
                    self.setup_sub_tab = st;
                }
            }
        });
    }

    fn ui_setup_mins_sheet(&mut self, ui: &mut egui::Ui) {
        let teal_hdr = egui::Color32::from_rgb(0, 112, 160);
        let gray_row = egui::Color32::from_rgb(198, 200, 204);
        let yellow_row = egui::Color32::from_rgb(245, 240, 185);
        let green_note = egui::Color32::from_rgb(165, 205, 115);

        let elems = self.setup.all_elements();
        let n_min = self.setup.minerals.len();
        let label_w = 185.0;
        let col_w = 96.0;

        ui.horizontal(|ui| {
            if ui.button("📚 Select from DB").clicked() {
                self.show_select_db_modal = true;
            }
            if ui.button("🔀 Combine Minerals").clicked() {
                self.open_combine_modal();
            }
            if ui.button("➕ Add Blank Mineral").clicked() {
                let n = self.setup.minerals.len() + 1;
                self.setup.minerals.push(MineralGroupDef {
                    code: format!("M{n}"),
                    mineral: format!("Mineral {n}"),
                    includes: vec![format!("M{n}")],
                    behavioral_types: vec!["All".into()],
                    reference: String::new(),
                    sg: 3.0,
                    chem_wt_pct: BTreeMap::new(),
                });
            }
        });
        ui.add_space(4.0);

        egui::ScrollArea::both()
            .id_salt("geo_setup_mins_scroll")
            .show(ui, |ui| {
                // Header letters A, B, C, ...
                ui.horizontal(|ui| {
                    row_num_cell(ui, 0, 26.0);
                    header_cell(ui, "A", label_w);
                    for m in 0..n_min.max(4) {
                        header_cell(ui, &col_letter(m + 1), col_w);
                    }
                });

                // Row 1: Title + Version + Green help note (exact Screenshot 2)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 26.0);
                    widget_cell(ui, label_w + col_w * 2.0, egui::Color32::WHITE, false, |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "HSC Sim Mineral Based Model Set-up File     Version {}",
                                self.setup.version
                            ))
                            .italics()
                            .size(11.0),
                        );
                    });
                    widget_cell(ui, col_w * 2.0, green_note, false, |ui| {
                        ui.label(
                            egui::RichText::new("FF=Fast floating, SF=Slow floating, NF=Non-floating")
                                .italics()
                                .size(10.0),
                        );
                    });
                });

                // Row 5: No. of minerals
                ui.horizontal(|ui| {
                    row_num_cell(ui, 5, 26.0);
                    widget_cell(ui, label_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new("No. of minerals").strong().size(11.0));
                    });
                    widget_cell(ui, col_w, egui::Color32::from_rgb(218, 222, 228), true, |ui| {
                        ui.label(egui::RichText::new(format!("{n_min}")).strong().monospace());
                    });
                });

                // Row 6: Teal header MineralNo
                ui.horizontal(|ui| {
                    row_num_cell(ui, 6, 26.0);
                    widget_cell(ui, label_w, teal_hdr, false, |ui| {
                        ui.label(egui::RichText::new("MineralNo").strong().color(egui::Color32::WHITE));
                    });
                    for m in 0..n_min {
                        widget_cell(ui, col_w, teal_hdr, false, |ui| {
                            ui.label(egui::RichText::new(format!("{}", m + 1)).strong().color(egui::Color32::WHITE));
                        });
                    }
                });

                // Row 7: Code
                ui.horizontal(|ui| {
                    row_num_cell(ui, 7, 26.0);
                    widget_cell(ui, label_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new("    Code").color(egui::Color32::from_rgb(0, 102, 153)));
                    });
                    for m in &mut self.setup.minerals {
                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut m.code);
                        });
                    }
                });

                // Row 8: Mineral
                ui.horizontal(|ui| {
                    row_num_cell(ui, 8, 26.0);
                    widget_cell(ui, label_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new("    Mineral").color(egui::Color32::from_rgb(0, 102, 153)));
                    });
                    for m in &mut self.setup.minerals {
                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                            ui.text_edit_singleline(&mut m.mineral);
                        });
                    }
                });

                // Row 9: Includes
                ui.horizontal(|ui| {
                    row_num_cell(ui, 9, 26.0);
                    widget_cell(ui, label_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new("    Includes").color(egui::Color32::from_rgb(0, 102, 153)));
                    });
                    for m in &self.setup.minerals {
                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                            ui.label(egui::RichText::new(m.includes.join(",")).size(10.5));
                        });
                    }
                });

                // Row 10: No of behavioral types (gray block per Screenshot 2)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 10, 26.0);
                    widget_cell(ui, label_w, gray_row, false, |ui| {
                        ui.label(egui::RichText::new("No of behavioral types:").strong().size(10.5));
                    });
                    for m in &self.setup.minerals {
                        widget_cell(ui, col_w, gray_row, false, |ui| {
                            ui.label(format!("{}", m.behavioral_types.len()));
                        });
                    }
                });

                // Row 14: Reference (yellow bar per Screenshot 2)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 14, 26.0);
                    widget_cell(ui, label_w, yellow_row, false, |ui| {
                        ui.label(egui::RichText::new("Reference").strong().size(10.5));
                    });
                    for m in &mut self.setup.minerals {
                        widget_cell(ui, col_w, yellow_row, false, |ui| {
                            ui.text_edit_singleline(&mut m.reference);
                        });
                    }
                });

                // Row 15: SG (Specific gravity)
                ui.horizontal(|ui| {
                    row_num_cell(ui, 15, 26.0);
                    widget_cell(ui, label_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new("SG  (Specific gravity)").strong().size(10.5));
                    });
                    for m in &mut self.setup.minerals {
                        widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                            ui.add(egui::DragValue::new(&mut m.sg).speed(0.05).range(0.5..=20.0).fixed_decimals(2));
                        });
                    }
                });

                // Row 16: Teal header Chemical composition, wt.%
                ui.horizontal(|ui| {
                    row_num_cell(ui, 16, 26.0);
                    widget_cell(ui, label_w, teal_hdr, false, |ui| {
                        ui.label(egui::RichText::new("Chemical composition, wt.%").strong().color(egui::Color32::WHITE));
                    });
                    for _ in 0..n_min {
                        widget_cell(ui, col_w, teal_hdr, false, |_| {});
                    }
                });

                // Element rows (editable chemical wt-% per mineral!)
                for (ei, el) in elems.iter().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, 17 + ei, 26.0);
                        widget_cell(ui, label_w, egui::Color32::from_rgb(245, 247, 250), false, |ui| {
                            ui.label(egui::RichText::new(format!("{el} %")).strong().size(10.5));
                        });
                        for m in &mut self.setup.minerals {
                            let val = m.chem_wt_pct.entry(el.clone()).or_insert(0.0);
                            widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                                ui.add(egui::DragValue::new(val).speed(0.1).range(0.0..=100.0).fixed_decimals(3));
                            });
                        }
                    });
                }
            });
    }

    fn ui_setup_size_sheet(&mut self, ui: &mut egui::Ui) {
        // Exact Screenshot 5 layout!
        ui.label(
            egui::RichText::new(format!(
                "HSC Sim Mineral Based Model Size Set-up          Version {}",
                self.setup.version
            ))
            .italics()
            .size(12.0),
        );
        ui.label(egui::RichText::new("Size classes").strong().size(13.0));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.strong("Unit:");
            ui.add_sized([72.0, 22.0], egui::TextEdit::singleline(&mut self.setup.size_unit));
            ui.add_space(16.0);
            ui.strong(format!("Total: {}", self.setup.size_classes.len()));
            ui.add_space(16.0);
            if ui.button("➕ Add Size Class").clicked() {
                let last_hi = self.setup.size_classes.last().map(|s| s.upper).unwrap_or(0.0);
                let next_no = self.setup.size_classes.len() + 1;
                self.setup.size_classes.push(SizeClassDef {
                    no: next_no,
                    lower: last_hi,
                    upper: (last_hi * 1.5).max(last_hi + 50.0),
                });
                for st in &mut self.streams {
                    st.size_bulk_wt_pct.push(0.0);
                    for row in &mut st.modal_targets {
                        row.push(0.0);
                    }
                }
            }
        });
        ui.add_space(6.0);

        let blue_no = egui::Color32::from_rgb(0, 0, 230);
        let cream_hdr = egui::Color32::from_rgb(245, 245, 195);
        let gray_cell = egui::Color32::from_rgb(214, 216, 220);

        ui.horizontal(|ui| {
            widget_cell(ui, 38.0, blue_no, false, |ui| {
                ui.label(egui::RichText::new("No").strong().color(egui::Color32::WHITE));
            });
            for (lbl, w) in [("Lower", 82.0), ("Upper", 82.0), ("Average Size", 104.0), ("Label", 118.0)] {
                widget_cell(ui, w, cream_hdr, false, |ui| {
                    ui.label(egui::RichText::new(lbl).strong());
                });
            }
        });

        let unit = self.setup.size_unit.clone();
        for sc in &mut self.setup.size_classes {
            ui.horizontal(|ui| {
                widget_cell(ui, 38.0, blue_no, false, |ui| {
                    ui.label(egui::RichText::new(format!("{}", sc.no)).strong().color(egui::Color32::WHITE));
                });
                widget_cell(ui, 82.0, gray_cell, false, |ui| {
                    ui.add(egui::DragValue::new(&mut sc.lower).speed(1.0).range(0.0..=100000.0));
                });
                widget_cell(ui, 82.0, gray_cell, false, |ui| {
                    ui.add(egui::DragValue::new(&mut sc.upper).speed(1.0).range(0.0..=100000.0));
                });
                widget_cell(ui, 104.0, gray_cell, false, |ui| {
                    ui.label(egui::RichText::new(format!("{:.2}", sc.average_size())).monospace());
                });
                widget_cell(ui, 118.0, gray_cell, false, |ui| {
                    ui.label(sc.label(&unit));
                });
            });
        }
    }

    // ========================================================================
    // Stream 4 Sheets: Mineral Composition | Particles | Log | Modal
    // ========================================================================

    fn ui_stream_sheets(&mut self, ui: &mut egui::Ui, idx: usize) {
        let body_h = (ui.available_height() - 30.0).max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), body_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| match self.stream_sub_tab {
                StreamSubTab::MineralComposition => self.ui_stream_mineral_comp_sheet(ui, idx),
                StreamSubTab::Particles => self.ui_stream_particles_sheet(ui, idx),
                StreamSubTab::Log => self.ui_stream_log_sheet(ui, idx),
                StreamSubTab::Modal => self.ui_stream_modal_sheet(ui, idx),
            },
        );

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⏮ ◀ ▶ ⏭").size(9.5).color(egui::Color32::GRAY));
            for (st, lbl) in [
                (StreamSubTab::MineralComposition, "Mineral Composition"),
                (StreamSubTab::Particles, "Particles"),
                (StreamSubTab::Log, "Log"),
                (StreamSubTab::Modal, "Modal"),
            ] {
                let active = self.stream_sub_tab == st;
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new(lbl).strong().size(11.0))
                            .fill(if active { egui::Color32::WHITE } else { egui::Color32::from_rgb(230, 234, 242) }),
                    )
                    .clicked()
                {
                    self.stream_sub_tab = st;
                }
            }
        });
    }

    fn ui_stream_mineral_comp_sheet(&mut self, ui: &mut egui::Ui, idx: usize) {
        let mut changed = false;
        let sum = self.streams[idx].compute_summary(&self.setup);
        let blue_bar = egui::Color32::from_rgb(0, 0, 225);
        let gray_row = egui::Color32::from_rgb(206, 208, 214);
        let yellow_chem = egui::Color32::from_rgb(245, 245, 115);

        let n_sz = self.setup.size_classes.len();
        let lbl_w = 132.0;
        let num_w = 88.0;

        egui::ScrollArea::both()
            .id_salt("geo_stream_mc_scroll")
            .show(ui, |ui| {
                // Header Stream & Total input (C3, C4 — exact Screenshot 8)
                ui.horizontal(|ui| {
                    ui.strong("Stream:");
                    ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut self.streams[idx].name));
                    ui.add_space(12.0);
                    ui.strong("Total input:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.streams[idx].total_input_tph)
                                .speed(0.2)
                                .range(0.0..=1e6)
                                .fixed_decimals(2),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.strong("tph");
                });
                ui.add_space(4.0);

                // Row 6: Blue SizeClassNo header
                ui.horizontal(|ui| {
                    row_num_cell(ui, 6, 26.0);
                    widget_cell(ui, lbl_w, blue_bar, false, |ui| {
                        ui.label(egui::RichText::new("SizeClassNo").strong().color(egui::Color32::WHITE));
                    });
                    widget_cell(ui, num_w, blue_bar, false, |ui| {
                        ui.label(egui::RichText::new("Bulk").strong().color(egui::Color32::WHITE));
                    });
                    for sc in &self.setup.size_classes {
                        widget_cell(ui, num_w, blue_bar, false, |ui| {
                            ui.label(egui::RichText::new(format!("{}", sc.no)).strong().color(egui::Color32::WHITE));
                        });
                    }
                });

                // Row 7: Name
                ui.horizontal(|ui| {
                    row_num_cell(ui, 7, 26.0);
                    widget_cell(ui, lbl_w, egui::Color32::WHITE, false, |ui| {
                        ui.label("Name");
                    });
                    widget_cell(ui, num_w, egui::Color32::WHITE, false, |_| {});
                    for sc in &self.setup.size_classes {
                        widget_cell(ui, num_w, egui::Color32::WHITE, false, |ui| {
                            ui.label(sc.label(&self.setup.size_unit));
                        });
                    }
                });

                // Row 8: Wt.% of bulk (editable!)
                let bulk_sum: f64 = self.streams[idx].size_bulk_wt_pct.iter().sum();
                ui.horizontal(|ui| {
                    row_num_cell(ui, 8, 26.0);
                    widget_cell(ui, lbl_w, egui::Color32::WHITE, false, |ui| {
                        ui.label("Wt.% of bulk");
                    });
                    widget_cell(ui, num_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(format!("{bulk_sum:.1}")).monospace());
                    });
                    for s in 0..n_sz {
                        if s >= self.streams[idx].size_bulk_wt_pct.len() {
                            self.streams[idx].size_bulk_wt_pct.push(0.0);
                        }
                        widget_cell(ui, num_w, egui::Color32::WHITE, false, |ui| {
                            if ui
                                .add(
                                    egui::DragValue::new(&mut self.streams[idx].size_bulk_wt_pct[s])
                                        .speed(0.5)
                                        .range(0.0..=100.0)
                                        .fixed_decimals(2),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        });
                    }
                });

                // Section: Mineral composition of the fraction (calculated from particles), wt.%
                ui.label(egui::RichText::new("Mineral composition of the fraction (calculated from particles), wt.%").strong());
                for (m, mdef) in self.setup.minerals.iter().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, 10 + m, 26.0);
                        widget_cell(ui, lbl_w, gray_row, false, |ui| {
                            ui.label(format!("{} wt%", mdef.code));
                        });
                        widget_cell(ui, num_w, gray_row, false, |ui| {
                            ui.label(egui::RichText::new(format!("{:.4}", sum.mineral_wt_bulk[m])).monospace().size(10.5));
                        });
                        for s in 0..n_sz {
                            widget_cell(ui, num_w, gray_row, false, |ui| {
                                let v = sum.mineral_wt_by_size[m][s];
                                let txt = if sum.total_wt_by_size[s] > 0.0 {
                                    format!("{v:.4}")
                                } else {
                                    String::new()
                                };
                                ui.label(egui::RichText::new(txt).monospace().size(10.5));
                            });
                        }
                    });
                }
                ui.horizontal(|ui| {
                    row_num_cell(ui, 10 + self.setup.minerals.len(), 26.0);
                    widget_cell(ui, lbl_w, gray_row, false, |ui| {
                        ui.label(egui::RichText::new("Total wt.%").strong());
                    });
                    widget_cell(ui, num_w, gray_row, false, |ui| {
                        ui.label(egui::RichText::new(format!("{:.1}", sum.total_wt_bulk)).strong().monospace());
                    });
                    for s in 0..n_sz {
                        widget_cell(ui, num_w, gray_row, false, |ui| {
                            let txt = if sum.total_wt_by_size[s] > 0.0 { "100".into() } else { String::new() };
                            ui.label(egui::RichText::new(txt).strong().monospace());
                        });
                    }
                });

                ui.add_space(4.0);
                ui.label(egui::RichText::new("Minerals tph").strong());
                for (m, mdef) in self.setup.minerals.iter().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, 17 + m, 26.0);
                        widget_cell(ui, lbl_w, gray_row, false, |ui| {
                            ui.label(format!("{} tph", mdef.code));
                        });
                        widget_cell(ui, num_w, gray_row, false, |ui| {
                            ui.label(egui::RichText::new(format!("{:.4}", sum.mineral_tph_bulk[m])).monospace().size(10.5));
                        });
                        for s in 0..n_sz {
                            widget_cell(ui, num_w, gray_row, false, |ui| {
                                let txt = if sum.total_wt_by_size[s] > 0.0 {
                                    format!("{:.4}", sum.mineral_tph_by_size[m][s])
                                } else {
                                    String::new()
                                };
                                ui.label(egui::RichText::new(txt).monospace().size(10.5));
                            });
                        }
                    });
                }
                ui.horizontal(|ui| {
                    row_num_cell(ui, 17 + self.setup.minerals.len(), 26.0);
                    widget_cell(ui, lbl_w, gray_row, false, |ui| {
                        ui.label(egui::RichText::new("Total tph").strong());
                    });
                    widget_cell(ui, num_w, gray_row, false, |ui| {
                        ui.label(egui::RichText::new(format!("{:.3}", sum.total_tph_bulk)).strong().monospace());
                    });
                    for s in 0..n_sz {
                        widget_cell(ui, num_w, gray_row, false, |ui| {
                            let txt = if sum.total_wt_by_size[s] > 0.0 {
                                format!("{:.3}", sum.total_tph_by_size[s])
                            } else {
                                String::new()
                            };
                            ui.label(egui::RichText::new(txt).strong().monospace());
                        });
                    }
                });

                // Yellow Chemical composition block (exact Screenshot 8 bottom!)
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Chemical composition of the fraction (wt.%, ppm, g/t)").strong());
                for (ei, el) in sum.elements.iter().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, 22 + ei, 26.0);
                        widget_cell(ui, lbl_w, yellow_chem, false, |ui| {
                            ui.label(format!("{el} %"));
                        });
                        widget_cell(ui, num_w, yellow_chem, false, |ui| {
                            ui.label(egui::RichText::new(format!("{:.5}", sum.elem_wt_bulk[ei])).monospace().size(10.5));
                        });
                        for s in 0..n_sz {
                            widget_cell(ui, num_w, yellow_chem, false, |ui| {
                                let txt = if sum.total_wt_by_size[s] > 0.0 {
                                    format!("{:.5}", sum.elem_wt_by_size[ei][s])
                                } else {
                                    String::new()
                                };
                                ui.label(egui::RichText::new(txt).monospace().size(10.5));
                            });
                        }
                    });
                }
            });

        if changed {
            self.streams[idx].sync_particle_flowrates(&self.setup);
        }
    }

    fn ui_stream_particles_sheet(&mut self, ui: &mut egui::Ui, idx: usize) {
        let st = &self.streams[idx];
        let total_rows = st.particles.len();
        let page_size = 150usize;
        let max_page = total_rows.saturating_sub(1) / page_size;
        if self.particle_page > max_page {
            self.particle_page = max_page;
        }
        let start = self.particle_page * page_size;
        let end = (start + page_size).min(total_rows);

        ui.horizontal(|ui| {
            ui.strong(format!("Particle data — {total_rows} rows"));
            ui.separator();
            ui.label("Mineral columns:");
            for &b in LiberationBasis::ALL {
                if ui.selectable_label(self.particle_comp_basis == b, b.label()).clicked() {
                    self.particle_comp_basis = b;
                }
            }
            ui.separator();
            if ui.add_enabled(self.particle_page > 0, egui::Button::new("◀ Prev")).clicked() {
                self.particle_page -= 1;
            }
            ui.label(format!("Rows {}..{} of {total_rows}", start + 1, end));
            if ui.add_enabled(self.particle_page < max_page, egui::Button::new("Next ▶")).clicked() {
                self.particle_page += 1;
            }
        });
        ui.add_space(4.0);

        let blue_hdr = egui::Color32::from_rgb(0, 0, 225);
        let cols: [(&str, f32); 10] = [
            ("ID", 118.0),
            ("Size", 68.0),
            ("SizeClassNo", 68.0),
            ("SizeClassName", 84.0),
            ("ParticleType", 82.0),
            ("tph", 72.0),
            ("Wt%(InBulk)", 78.0),
            ("Wt%(InFraction)", 88.0),
            ("NOP", 52.0),
            ("SG", 54.0),
        ];

        egui::ScrollArea::both()
            .id_salt("geo_particles_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 6, 30.0);
                    for &(h, w) in &cols {
                        widget_cell(ui, w, blue_hdr, false, |ui| {
                            ui.label(egui::RichText::new(h).strong().size(10.0).color(egui::Color32::WHITE));
                        });
                    }
                    for m in &self.setup.minerals {
                        widget_cell(ui, 72.0, blue_hdr, false, |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} {}", m.code, self.particle_comp_basis.label()))
                                    .strong()
                                    .size(10.0)
                                    .color(egui::Color32::WHITE),
                            );
                        });
                    }
                });

                for (i, p) in st.particles[start..end].iter().enumerate() {
                    let bg = if i % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(247, 249, 252)
                    };
                    let comp = p.basis_vec(self.particle_comp_basis);
                    ui.horizontal(|ui| {
                        row_num_cell(ui, 7 + start + i, 30.0);
                        let vals = [
                            p.id.clone(),
                            format!("{:.3}", p.size_um),
                            format!("{}", p.size_class_no),
                            self.setup.size_class_label(p.size_class_no),
                            p.particle_type.clone(),
                            format!("{:.4}", p.tph),
                            format!("{:.4}", p.wt_pct_in_bulk),
                            format!("{:.4}", p.wt_pct_in_fraction),
                            format!("{:.0}", p.nop),
                            format!("{:.3}", p.sg),
                        ];
                        for (c, v) in vals.iter().enumerate() {
                            widget_cell(ui, cols[c].1, bg, false, |ui| {
                                ui.label(egui::RichText::new(v).monospace().size(10.0));
                            });
                        }
                        for m in 0..self.setup.minerals.len() {
                            let mv = comp.get(m).copied().unwrap_or(0.0);
                            widget_cell(ui, 72.0, bg, false, |ui| {
                                ui.label(egui::RichText::new(format!("{mv:.2}")).monospace().size(10.0));
                            });
                        }
                    });
                }
            });
    }

    fn ui_stream_log_sheet(&mut self, ui: &mut egui::Ui, idx: usize) {
        // Exact Screenshot 10: Operation | By | At | Note | Particles
        let cols = [
            ("Operation", 140.0),
            ("By", 82.0),
            ("At", 130.0),
            ("Note", 240.0),
            ("Particles", 78.0),
        ];
        egui::ScrollArea::both()
            .id_salt("geo_log_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 26.0);
                    for &(h, w) in &cols {
                        header_cell(ui, h, w);
                    }
                });
                for (i, e) in self.streams[idx].log.iter().enumerate() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, i + 2, 26.0);
                        widget_cell(ui, cols[0].1, egui::Color32::WHITE, false, |ui| {
                            ui.label(&e.operation);
                        });
                        widget_cell(ui, cols[1].1, egui::Color32::WHITE, false, |ui| {
                            ui.label(&e.by);
                        });
                        widget_cell(ui, cols[2].1, egui::Color32::WHITE, false, |ui| {
                            ui.label(&e.at);
                        });
                        widget_cell(ui, cols[3].1, egui::Color32::WHITE, false, |ui| {
                            ui.label(&e.note);
                        });
                        widget_cell(ui, cols[4].1, egui::Color32::WHITE, false, |ui| {
                            ui.label(egui::RichText::new(format!("{}", e.particles)).monospace());
                        });
                    });
                }
            });
    }

    fn ui_stream_modal_sheet(&mut self, ui: &mut egui::Ui, idx: usize) {
        // Exact Screenshot 11: Mineral × Size Class editable target table for Rebalance
        ui.horizontal(|ui| {
            if ui.button("📋 Copy Current Composition -> Modal").clicked() {
                self.streams[idx].init_modal_from_current(&self.setup);
                self.status_msg = Some("Copied current calculated mineralogy into Modal targets.".into());
            }
            if ui.button("⚖ Rebalance Stream to Modal").clicked() {
                match run_rebalance(&mut self.streams[idx], &self.setup) {
                    Ok((it, diff)) => {
                        self.status_err = None;
                        self.status_msg = Some(format!("Rebalance finished ({it} iters, max diff {diff:.3}%)."));
                    }
                    Err(e) => self.status_err = Some(e),
                }
            }
        });
        ui.add_space(4.0);

        let n_min = self.setup.minerals.len();
        let n_sz = self.setup.size_classes.len();
        if self.streams[idx].modal_targets.len() < n_min {
            self.streams[idx].modal_targets.resize(n_min, vec![0.0; n_sz]);
        }
        for row in &mut self.streams[idx].modal_targets {
            if row.len() < n_sz {
                row.resize(n_sz, 0.0);
            }
        }

        egui::ScrollArea::both()
            .id_salt("geo_modal_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, 26.0);
                    header_cell(ui, "Mineral", 92.0);
                    for sc in &self.setup.size_classes {
                        header_cell(ui, &sc.label(&self.setup.size_unit), 88.0);
                    }
                });
                for m in 0..n_min {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, m + 2, 26.0);
                        widget_cell(ui, 92.0, egui::Color32::from_rgb(240, 243, 248), false, |ui| {
                            ui.label(egui::RichText::new(&self.setup.minerals[m].code).strong());
                        });
                        for s in 0..n_sz {
                            widget_cell(ui, 88.0, egui::Color32::WHITE, false, |ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.streams[idx].modal_targets[m][s])
                                        .speed(0.25)
                                        .range(0.0..=100.0)
                                        .fixed_decimals(2),
                                );
                            });
                        }
                    });
                }
            });
    }

    // ========================================================================
    // Modals: Combine (`Mineral Group`), Select from DB, Size class, Config,
    // Filter, Fill Gangue, +Lib/-Lib, Graphs, File Save/Open
    // ========================================================================

    fn ui_combine_modal(&mut self, ctx: &egui::Context) {
        if !self.combine.open {
            return;
        }
        let mut open = self.combine.open;
        let mut apply_done = false;

        egui::Window::new("Mineral Group")
            .open(&mut open)
            .collapsible(false)
            .default_size([470.0, 420.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // Left list: Minerals/Groups Existing
                    ui.vertical(|ui| {
                        ui.label("Minerals/Groups Existing:");
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("comb_existing_scroll")
                                    .max_height(310.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(155.0);
                                        for (i, m) in self.combine.existing.iter().enumerate() {
                                            let sel = self.combine.existing_selected.contains(&i);
                                            if ui.selectable_label(sel, &m.code).clicked() {
                                                if sel {
                                                    self.combine.existing_selected.remove(&i);
                                                } else {
                                                    self.combine.existing_selected.insert(i);
                                                }
                                            }
                                        }
                                    });
                            });
                    });

                    // Middle 5 Arrow Buttons (exact Screenshots 3 & 4!)
                    ui.vertical_centered(|ui| {
                        ui.add_space(72.0);
                        // > move selected as individual new groups
                        if ui.add_sized([54.0, 24.0], egui::Button::new("▶")).on_hover_text("Move selected mineral(s) as new group(s)").clicked() {
                            let idxs: Vec<usize> = self.combine.existing_selected.iter().copied().collect();
                            for &i in idxs.iter().rev() {
                                let m = self.combine.existing.remove(i);
                                self.combine.new_groups.push(m);
                                self.combine.new_selected = Some(self.combine.new_groups.len() - 1);
                            }
                            self.combine.existing_selected.clear();
                        }
                        // >> move all
                        if ui.add_sized([54.0, 24.0], egui::Button::new("⏩")).on_hover_text("Move all minerals to right").clicked() {
                            self.combine.new_groups.append(&mut self.combine.existing);
                            self.combine.existing_selected.clear();
                            if !self.combine.new_groups.is_empty() {
                                self.combine.new_selected = Some(0);
                            }
                        }
                        // >| merge selected into selected right-side group!
                        if ui.add_sized([54.0, 24.0], egui::Button::new("⏭")).on_hover_text("Combine selected mineral(s) into highlighted group on right").clicked() {
                            if let Some(target_i) = self.combine.new_selected {
                                if target_i < self.combine.new_groups.len() {
                                    let idxs: Vec<usize> = self.combine.existing_selected.iter().copied().collect();
                                    for &i in idxs.iter().rev() {
                                        let m = self.combine.existing.remove(i);
                                        let tgt = &mut self.combine.new_groups[target_i];
                                        for inc in m.includes {
                                            if !tgt.includes.contains(&inc) {
                                                tgt.includes.push(inc);
                                            }
                                        }
                                        tgt.sg = 0.5 * (tgt.sg + m.sg);
                                    }
                                    self.combine.existing_selected.clear();
                                }
                            }
                        }
                        ui.add_space(18.0);
                        // < return selected group to left
                        if ui.add_sized([54.0, 24.0], egui::Button::new("◀")).on_hover_text("Return selected group to left").clicked() {
                            if let Some(ti) = self.combine.new_selected {
                                if ti < self.combine.new_groups.len() {
                                    let m = self.combine.new_groups.remove(ti);
                                    self.combine.existing.push(m);
                                    self.combine.new_selected = if self.combine.new_groups.is_empty() {
                                        None
                                    } else {
                                        Some(ti.min(self.combine.new_groups.len() - 1))
                                    };
                                }
                            }
                        }
                        // << return all to left
                        if ui.add_sized([54.0, 24.0], egui::Button::new("⏪")).on_hover_text("Return all groups to left").clicked() {
                            self.combine.existing.append(&mut self.combine.new_groups);
                            self.combine.new_selected = None;
                        }
                    });

                    // Right list: New Groups + Rename + Ok / Cancel (exact Screenshot 4)
                    ui.vertical(|ui| {
                        ui.label("New Groups (double-click to inspect):");
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("comb_new_scroll")
                                    .max_height(260.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(175.0);
                                        for (i, g) in self.combine.new_groups.iter().enumerate() {
                                            let sel = self.combine.new_selected == Some(i);
                                            let resp = ui.selectable_label(sel, &g.code);
                                            if resp.clicked() {
                                                self.combine.new_selected = Some(i);
                                            }
                                            if resp.double_clicked() {
                                                self.combine.info_group_idx = Some(i);
                                                self.combine.info_open = true;
                                            }
                                        }
                                    });
                            });
                        ui.add_space(4.0);
                        if ui.add_sized([175.0, 24.0], egui::Button::new("✏ Rename")).clicked() {
                            if let Some(ti) = self.combine.new_selected {
                                if ti < self.combine.new_groups.len() {
                                    self.combine.rename_buf = self.combine.new_groups[ti].code.clone();
                                    self.combine.rename_open = true;
                                }
                            }
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([84.0, 26.0], egui::Button::new("✖ Cancel")).clicked() {
                                self.combine.open = false;
                            }
                            if ui.add_sized([84.0, 26.0], egui::Button::new("✔ Ok")).clicked() {
                                apply_done = true;
                            }
                        });
                    });
                });
            });

        // Sub-popup: Rename Mineral Group (exact Screenshot 4)
        if self.combine.rename_open {
            let mut r_open = true;
            egui::Window::new("Rename Mineral Group")
                .open(&mut r_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let cur = self
                        .combine
                        .new_selected
                        .and_then(|i| self.combine.new_groups.get(i))
                        .map(|g| g.code.clone())
                        .unwrap_or_default();
                    ui.label(format!("New name for {cur}"));
                    ui.add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut self.combine.rename_buf));
                    ui.horizontal(|ui| {
                        if ui.button("Ok").clicked() {
                            if let Some(ti) = self.combine.new_selected {
                                if let Some(g) = self.combine.new_groups.get_mut(ti) {
                                    if !self.combine.rename_buf.trim().is_empty() {
                                        g.code = self.combine.rename_buf.trim().to_string();
                                    }
                                }
                            }
                            self.combine.rename_open = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.combine.rename_open = false;
                        }
                    });
                });
            if !r_open {
                self.combine.rename_open = false;
            }
        }

        // Sub-popup: Mineral group information on double click (exact Screenshot 4)
        if self.combine.info_open {
            let mut i_open = true;
            egui::Window::new("Mineral group information")
                .open(&mut i_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if let Some(gi) = self.combine.info_group_idx {
                        if let Some(g) = self.combine.new_groups.get(gi) {
                            ui.label("Minerals Group:");
                            ui.strong(&g.code);
                            ui.separator();
                            ui.label("Includes Mineral:");
                            ui.label(g.includes.join(","));
                        }
                    }
                    ui.add_space(6.0);
                    if ui.button("Ok").clicked() {
                        self.combine.info_open = false;
                    }
                });
            if !i_open {
                self.combine.info_open = false;
            }
        }

        if apply_done {
            if !self.combine.new_groups.is_empty() {
                self.setup.minerals = self.combine.new_groups.clone();
                self.active_workspace_tab = ActiveWorkspaceTab::MineralSetup;
                self.setup_sub_tab = SetupSubTab::Mins;
                self.status_msg = Some(format!(
                    "Updated Mineral Setup with {} combined mineral group(s).",
                    self.setup.minerals.len()
                ));
            }
            self.combine.open = false;
        } else if !open {
            self.combine.open = false;
        }
    }

    fn ui_select_from_db_modal(&mut self, ctx: &egui::Context) {
        if !self.show_select_db_modal {
            return;
        }
        let mut open = true;
        let mut added: Option<GeoMineralCatalogEntry> = None;
        egui::Window::new("Select Minerals from HSC Geo DB (3,812 minerals)")
            .open(&mut open)
            .collapsible(false)
            .default_size([460.0, 360.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add_sized([280.0, 22.0], egui::TextEdit::singleline(&mut self.select_db_query).hint_text("Galena, Quartz, Chalcopyrite..."));
                });
                ui.separator();
                let res = search_geo_mineral_catalog(&self.select_db_query, 80);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for r in res {
                        ui.horizontal(|ui| {
                            if ui.small_button("➕ Add").clicked() {
                                added = Some(r.clone());
                            }
                            ui.label(egui::RichText::new(&r.code).strong().monospace());
                            ui.label(&r.name);
                            ui.label(egui::RichText::new(&r.formula).monospace().size(10.5));
                        });
                    }
                });
            });
        if let Some(e) = added {
            self.setup.minerals.push(MineralGroupDef::from_catalog(&e));
            self.status_msg = Some(format!("Added '{}' ({}) to Mineral Setup.", e.name, e.code));
        }
        if !open {
            self.show_select_db_modal = false;
        }
    }

    fn ui_import_size_class_modal(&mut self, ctx: &egui::Context) {
        if !self.show_import_size_modal {
            return;
        }
        let mut open = true;
        let mut do_ok = false;
        egui::Window::new("Size class")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Select one and press OK, cancel to exit without selection.");
                ui.label(egui::RichText::new(format!("Source: {}", self.import_source_label)).strong());
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .show(ui, |ui| {
                        ui.set_min_width(230.0);
                        for sc in &self.setup.size_classes {
                            let lbl = sc.label(&self.setup.size_unit);
                            if ui.selectable_label(self.import_size_selected == sc.no, lbl).clicked() {
                                self.import_size_selected = sc.no;
                            }
                        }
                    });
                ui.add_space(6.0);
                ui.label("Optional CSV/TSV file path (leave blank to generate sample fraction):");
                ui.add_sized([280.0, 22.0], egui::TextEdit::singleline(&mut self.import_csv_path));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        do_ok = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_import_size_modal = false;
                    }
                });
            });
        if do_ok {
            if let Some(aidx) = self.active_stream_idx {
                let sz_no = self.import_size_selected;
                let path = self.import_csv_path.trim().to_string();
                if !path.is_empty() {
                    match std::fs::read_to_string(&path) {
                        Ok(raw) => match import_particles_csv_into_size_class(
                            &mut self.streams[aidx],
                            &self.setup,
                            sz_no,
                            &raw,
                        ) {
                            Ok(n) => {
                                self.status_err = None;
                                self.status_msg = Some(format!(
                                    "Imported {n} particles into {}.",
                                    self.setup.size_class_label(sz_no)
                                ));
                            }
                            Err(e) => self.status_err = Some(e),
                        },
                        Err(e) => self.status_err = Some(format!("File read error: {e}")),
                    }
                } else {
                    // Generate measured SEM fraction for that size class
                    self.rng_counter += 1;
                    let sample = LiberationStream::generate_sample_stream(
                        "tmp",
                        &self.setup,
                        21.2,
                        0.32,
                        0.26,
                        false,
                        400 + self.rng_counter,
                    );
                    let frac_ps: Vec<_> = sample
                        .particles
                        .into_iter()
                        .filter(|p| p.size_class_no == sz_no)
                        .collect();
                    let n = frac_ps.len();
                    self.streams[aidx].particles.retain(|p| p.size_class_no != sz_no);
                    self.streams[aidx].particles.extend(frac_ps);
                    self.streams[aidx].is_basic_binned = false;
                    self.streams[aidx].sync_particle_flowrates(&self.setup);
                    self.streams[aidx].push_log(
                        &self.import_source_label,
                        format!("Loaded {n} particles into {}", self.setup.size_class_label(sz_no)),
                    );
                    self.status_err = None;
                    self.status_msg = Some(format!(
                        "Loaded {n} measured particles into {}.",
                        self.setup.size_class_label(sz_no)
                    ));
                }
            }
            self.show_import_size_modal = false;
        } else if !open {
            self.show_import_size_modal = false;
        }
    }

    fn ui_rename_stream_modal(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.rename_stream_idx else {
            return;
        };
        let mut open = true;
        egui::Window::new("Rename Stream")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("New stream name:");
                ui.add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut self.rename_stream_buf));
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        if idx < self.streams.len() && !self.rename_stream_buf.trim().is_empty() {
                            self.streams[idx].name = self.rename_stream_buf.trim().to_string();
                        }
                        self.rename_stream_idx = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.rename_stream_idx = None;
                    }
                });
            });
        if !open {
            self.rename_stream_idx = None;
        }
    }

    fn ui_prereq_binning_modal(&mut self, ctx: &egui::Context) {
        let Some(action) = self.prereq_binning_action else {
            return;
        };
        let mut open = true;
        let mut run_now = false;
        let act_label = if action == "missing" {
            "Missing? (Extrapolate Size Classes)"
        } else {
            "Rebalance"
        };
        egui::Window::new("Basic Binning Required")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Basic Binning is a prerequisite for {act_label}.\nDo you want to run Basic Binning first and then proceed?"
                ));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("✔ Yes, Run Basic Binning").clicked() {
                        run_now = true;
                    }
                    if ui.button("✖ Cancel").clicked() {
                        self.prereq_binning_action = None;
                    }
                });
            });
        if run_now {
            if let Some(a) = self.active_stream_idx {
                run_basic_binning(&mut self.streams[a], &self.setup);
                if action == "missing" {
                    match run_extrapolate_missing(&mut self.streams[a], &self.setup) {
                        Ok(0) => self.status_msg = Some("Basic Binning done. All size classes already have particles.".into()),
                        Ok(n) => self.status_msg = Some(format!("Basic Binning + Missing? extrapolated {n} size class(es) via WLS-SVD.")),
                        Err(e) => self.status_err = Some(e),
                    }
                } else {
                    match run_rebalance(&mut self.streams[a], &self.setup) {
                        Ok((it, diff)) => {
                            self.status_err = None;
                            self.status_msg = Some(format!("Basic Binning + Rebalance complete ({it} iters, max diff {diff:.3}%)."));
                        }
                        Err(e) => self.status_err = Some(e),
                    }
                }
            }
            self.prereq_binning_action = None;
        } else if !open {
            self.prereq_binning_action = None;
        }
    }

    fn ui_config_modal(&mut self, ctx: &egui::Context) {
        if !self.show_config_modal {
            return;
        }
        let mut open = true;
        let title = if self.config_is_global { "Global Config" } else { "Config" };
        let cfg = if self.config_is_global {
            &mut self.global_config
        } else if let Some(a) = self.active_stream_idx {
            &mut self.streams[a].config
        } else {
            &mut self.global_config
        };

        let mut reset_default = false;
        let mut open_tree = false;
        let mut cancel_changes = false;

        let eq_labels = [
            "Lin", "2nd", "Log", "Exp", "POWER", "3rd Deg", "4th Deg", "5th Deg",
        ];

        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size([430.0, 410.0])
            .show(ctx, |ui| {
                // Exact 2-column layout from Batch 2 Screenshot 5!
                ui.horizontal_top(|ui| {
                    // LEFT COLUMN: Bins group + List button
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(245, 246, 249))
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.set_min_width(185.0);
                            ui.horizontal(|ui| {
                                ui.strong("Bins");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("List  ▶").on_hover_text("Inspect particle classes tree").clicked() {
                                        open_tree = true;
                                    }
                                });
                            });
                            ui.separator();
                            egui::Grid::new("geo_cfg_bins_l").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                                ui.label("Lib threshold:");
                                ui.add(egui::DragValue::new(&mut cfg.lib_threshold).range(50.0..=100.0).speed(0.5));
                                ui.end_row();
                                ui.label("Tolerance:");
                                ui.add(egui::DragValue::new(&mut cfg.tolerance).range(0.5..=45.0).speed(0.5));
                                ui.end_row();
                                ui.label("Binary classes:");
                                ui.add(egui::DragValue::new(&mut cfg.binary_classes).range(2..=20));
                                ui.end_row();
                            });
                            ui.checkbox(&mut cfg.ternaries, "Ternaries");
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(166.0);
                                    ui.label(egui::RichText::new("Classif. basis").strong().size(10.5));
                                    ui.radio_value(&mut cfg.classif_basis, LiberationBasis::WeightPct, "Wt%");
                                    ui.radio_value(&mut cfg.classif_basis, LiberationBasis::VolumePct, "Vol%");
                                    ui.radio_value(&mut cfg.classif_basis, LiberationBasis::SurfacePct, "Surface%");
                                });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("N Min:");
                                ui.add(egui::DragValue::new(&mut cfg.n_min).range(1.0..=500.0));
                            });
                            ui.checkbox(&mut cfg.uniform_classes, "Uniform classes");
                        });

                    ui.add_space(6.0);

                    // RIGHT COLUMN: Rebalance + Smooth
                    ui.vertical(|ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(245, 246, 249))
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_min_width(198.0);
                                ui.strong("Rebalance");
                                ui.separator();
                                egui::Grid::new("geo_cfg_reb_r").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                                    ui.label("Max Iter:");
                                    ui.add(egui::DragValue::new(&mut cfg.max_iter).range(5..=2000));
                                    ui.end_row();
                                    ui.label("Max total DIFF:");
                                    ui.add(egui::DragValue::new(&mut cfg.max_total_diff).speed(0.05).range(0.0..=20.0));
                                    ui.end_row();
                                    ui.label("Max indiv DIFF:");
                                    ui.add(egui::DragValue::new(&mut cfg.max_indiv_diff).speed(0.01).range(0.0..=10.0));
                                    ui.end_row();
                                });
                            });

                        ui.add_space(6.0);

                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(245, 246, 249))
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_min_width(198.0);
                                ui.strong("Smooth");
                                ui.separator();
                                ui.checkbox(&mut cfg.smooth_binaries_alone, "Equations applied in smoothing");
                                egui::Frame::none()
                                    .fill(egui::Color32::WHITE)
                                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                    .inner_margin(egui::Margin::same(4.0))
                                    .show(ui, |ui| {
                                        egui::ScrollArea::vertical()
                                            .id_salt("geo_cfg_smooth_eqs")
                                            .max_height(106.0)
                                            .show(ui, |ui| {
                                                ui.set_min_width(180.0);
                                                for (i, lbl) in eq_labels.iter().enumerate() {
                                                    ui.checkbox(&mut cfg.smooth_equations_mask[i], *lbl);
                                                }
                                            });
                                    });
                                ui.add_space(4.0);
                                egui::Frame::none()
                                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                    .inner_margin(egui::Margin::same(6.0))
                                    .show(ui, |ui| {
                                        ui.set_min_width(180.0);
                                        ui.label(egui::RichText::new("Smooth basis").strong().size(10.5));
                                        ui.radio_value(&mut cfg.smooth_basis, LiberationBasis::WeightPct, "Wt%");
                                        ui.radio_value(&mut cfg.smooth_basis, LiberationBasis::VolumePct, "Vol%");
                                        ui.radio_value(&mut cfg.smooth_basis, LiberationBasis::SurfacePct, "Surface%");
                                    });
                            });
                    });
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("↺ Reset to default").clicked() {
                        reset_default = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([84.0, 26.0], egui::Button::new("✖ Cancel")).clicked() {
                            cancel_changes = true;
                        }
                        if ui.add_sized([84.0, 26.0], egui::Button::new("✔ OK")).clicked() {
                            self.config_backup = None;
                            self.show_config_modal = false;
                        }
                    });
                });
            });

        if reset_default {
            if self.config_is_global {
                self.global_config = StreamConfig::default();
            } else if let Some(a) = self.active_stream_idx {
                self.streams[a].config = self.global_config.clone();
            }
        }
        if open_tree {
            self.tree_backup = if self.config_is_global {
                Some(self.global_config.clone())
            } else {
                self.active_stream_idx.map(|a| self.streams[a].config.clone())
            };
            self.show_binning_tree_modal = true;
        }
        if cancel_changes || !open {
            if let Some(bak) = self.config_backup.take() {
                if self.config_is_global {
                    self.global_config = bak;
                } else if let Some(a) = self.active_stream_idx {
                    self.streams[a].config = bak;
                }
            }
            self.show_config_modal = false;
        }
    }

    fn ui_binning_tree_modal(&mut self, ctx: &egui::Context) {
        if !self.show_binning_tree_modal {
            return;
        }
        let mut open = true;
        let mut cancel_tree = false;
        let cfg = if self.config_is_global {
            &mut self.global_config
        } else if let Some(a) = self.active_stream_idx {
            &mut self.streams[a].config
        } else {
            &mut self.global_config
        };

        let n_m = self.setup.minerals.len();
        let n_sz = self.setup.size_classes.len();
        let n_pairs = n_m * n_m.saturating_sub(1) / 2;
        let n_triplets = if cfg.ternaries && n_m >= 3 {
            n_m * (n_m - 1) * (n_m - 2) / 6
        } else {
            0
        };
        let n_lib_total = n_sz * n_m;
        let n_bin_total = n_sz * n_pairs * cfg.binary_classes;
        let n_tern_total = n_sz * n_triplets * 6;
        let n_comp_total = n_sz * n_m;
        let grand_total = n_lib_total + n_bin_total + n_tern_total + n_comp_total;

        // Exact Batch 2 Screenshot 6 (`Particles` window with left tree & right Bins + Regenerate)!
        egui::Window::new("Particles")
            .open(&mut open)
            .collapsible(false)
            .default_size([520.0, 390.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // Left Tree Pane
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("geo_particles_tree_scroll")
                                .max_height(340.0)
                                .show(ui, |ui| {
                                    ui.set_min_width(305.0);
                                    egui::CollapsingHeader::new(format!("Minerals = {n_m}"))
                                        .default_open(false)
                                        .show(ui, |ui| {
                                            for (i, m) in self.setup.minerals.iter().enumerate() {
                                                ui.label(format!("{} = {} ({})", i + 1, m.code, m.mineral));
                                            }
                                        });
                                    egui::CollapsingHeader::new(format!("Size classes = {n_sz}"))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            for sc in &self.setup.size_classes {
                                                ui.label(format!("{} = {:.0}  ({})", sc.no, sc.lower, sc.label(&self.setup.size_unit)));
                                            }
                                        });
                                    egui::CollapsingHeader::new(format!("Particles classes = {grand_total}"))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            egui::CollapsingHeader::new(format!(
                                                "Liberated = ( {n_sz} * {n_m} ) = {n_lib_total}"
                                            ))
                                            .show(ui, |ui| {
                                                for (i, m) in self.setup.minerals.iter().enumerate() {
                                                    ui.label(format!("A.{:02} — {} (>= {:.1}%)", i + 1, m.code, cfg.lib_threshold));
                                                }
                                            });
                                            egui::CollapsingHeader::new(format!(
                                                "Binaries = ( {n_sz} x {n_pairs} x {} ) = {n_bin_total}",
                                                cfg.binary_classes
                                            ))
                                            .default_open(true)
                                            .show(ui, |ui| {
                                                let nb = cfg.binary_classes.max(1);
                                                let step = cfg.lib_threshold / nb as f64;
                                                for b in 0..nb {
                                                    let lo = b as f64 * step;
                                                    let hi = (b + 1) as f64 * step;
                                                    ui.label(format!("{})  {lo:.1} - {hi:.1}", b + 1));
                                                }
                                            });
                                            ui.label(format!("Ternaries = {n_tern_total}"));
                                            ui.label(format!("Complex = {n_comp_total}"));
                                        });
                                });
                        });

                    ui.add_space(8.0);

                    // Right Controls Pane (`Bins` + `Regenerate` + `Ok` / `Cancel`)
                    ui.vertical(|ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(245, 246, 249))
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_min_width(168.0);
                                ui.strong("Bins");
                                ui.separator();
                                egui::Grid::new("geo_tree_bins_g").num_columns(2).spacing([6.0, 5.0]).show(ui, |ui| {
                                    ui.label("Lib threshold:");
                                    ui.add(egui::DragValue::new(&mut cfg.lib_threshold).range(50.0..=100.0).speed(0.5));
                                    ui.end_row();
                                    ui.label("Tolerance:");
                                    ui.add(egui::DragValue::new(&mut cfg.tolerance).range(0.5..=45.0).speed(0.5));
                                    ui.end_row();
                                    ui.label("Binary classes:");
                                    ui.add(egui::DragValue::new(&mut cfg.binary_classes).range(2..=20));
                                    ui.end_row();
                                });
                                ui.checkbox(&mut cfg.ternaries, "Ternaries");
                                ui.add_space(6.0);
                                if ui.add_sized([152.0, 26.0], egui::Button::new("🔄 Regenerate")).clicked() {
                                    // Tree recalculates live on every change; clicking gives immediate visual feedback
                                }
                            });
                        ui.add_space(100.0);
                        if ui.add_sized([168.0, 26.0], egui::Button::new("✔ Ok")).clicked() {
                            self.tree_backup = None;
                            self.show_binning_tree_modal = false;
                        }
                        ui.add_space(4.0);
                        if ui.add_sized([168.0, 26.0], egui::Button::new("✖ Cancel")).clicked() {
                            cancel_tree = true;
                        }
                    });
                });
            });

        if cancel_tree || !open {
            if let Some(bak) = self.tree_backup.take() {
                if self.config_is_global {
                    self.global_config = bak;
                } else if let Some(a) = self.active_stream_idx {
                    self.streams[a].config = bak;
                }
            }
            self.show_binning_tree_modal = false;
        }
    }

    fn ui_filter_modal(&mut self, ctx: &egui::Context) {
        if !self.show_filter_modal {
            return;
        }
        let mut open = true;
        let mut do_apply = false;

        // Exact Batch 2 Screenshot 3 (`Process Filter`)!
        egui::Window::new("Process Filter")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 249))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.strong("Process:");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.filter_by_size_fraction, false, "All particles");
                            ui.radio_value(&mut self.filter_by_size_fraction, true, "Size Fraction");
                            ui.add_enabled_ui(self.filter_by_size_fraction, |ui| {
                                egui::ComboBox::from_id_salt("geo_flt_sz_cb")
                                    .width(120.0)
                                    .selected_text(self.setup.size_class_label(self.filter_size_no))
                                    .show_ui(ui, |ui| {
                                        for sc in &self.setup.size_classes {
                                            ui.selectable_value(
                                                &mut self.filter_size_no,
                                                sc.no,
                                                sc.label(&self.setup.size_unit),
                                            );
                                        }
                                    });
                            });
                        });
                    });

                ui.add_space(6.0);
                ui.horizontal_top(|ui| {
                    // Left: Delete / Leave + "all particles where" + Property combo
                    ui.vertical(|ui| {
                        ui.radio_value(&mut self.filter_delete_matching, true, "Delete");
                        ui.radio_value(&mut self.filter_delete_matching, false, "Leave");
                        ui.add_space(10.0);
                        ui.label("all particles where");
                        let mut prop_names = vec![
                            "Size".to_string(),
                            "tph".into(),
                            "Wt%(InBulk)".into(),
                            "Wt%(InFraction)".into(),
                            "NOP".into(),
                            "SG".into(),
                        ];
                        for m in &self.setup.minerals {
                            prop_names.push(format!("{} wt%", m.code));
                        }
                        let sel_name = prop_names
                            .get(self.filter_prop_idx)
                            .cloned()
                            .unwrap_or_else(|| "Size".into());
                        egui::ComboBox::from_id_salt("geo_flt_prop")
                            .width(135.0)
                            .selected_text(sel_name)
                            .show_ui(ui, |ui| {
                                for (i, n) in prop_names.iter().enumerate() {
                                    ui.selectable_value(&mut self.filter_prop_idx, i, n);
                                }
                            });
                    });

                    ui.add_space(10.0);

                    // Center: white box with 6 vertical relation radios (=, <>, >, >=, <, <=)
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for &r in FilterRelation::ALL {
                                    ui.radio_value(&mut self.filter_rel, r, r.label());
                                }
                            });
                        });

                    ui.add_space(10.0);

                    // Right: numeric value box at top, Apply + Close at bottom
                    ui.vertical(|ui| {
                        ui.add_sized([132.0, 22.0], egui::TextEdit::singleline(&mut self.filter_val_str));
                        ui.add_space(80.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([62.0, 24.0], egui::Button::new("Apply")).clicked() {
                                do_apply = true;
                            }
                            if ui.add_sized([62.0, 24.0], egui::Button::new("Close")).clicked() {
                                self.show_filter_modal = false;
                            }
                        });
                    });
                });
            });

        if do_apply {
            if let Some(a) = self.active_stream_idx {
                let thr = self.filter_val_str.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0);
                let prop = match self.filter_prop_idx {
                    0 => FilterProperty::Size,
                    1 => FilterProperty::Tph,
                    2 => FilterProperty::WtPctBulk,
                    3 => FilterProperty::WtPctFraction,
                    4 => FilterProperty::Nop,
                    5 => FilterProperty::Sg,
                    k => FilterProperty::MineralWtPct(k - 6),
                };
                let sz_opt = if self.filter_by_size_fraction {
                    Some(self.filter_size_no)
                } else {
                    None
                };
                let rem = run_filter(
                    &mut self.streams[a],
                    &self.setup,
                    sz_opt,
                    self.filter_delete_matching,
                    prop,
                    self.filter_rel,
                    thr,
                );
                self.status_msg = Some(format!("Process Filter applied: removed {rem} particle(s)."));
            }
            self.show_filter_modal = false;
        } else if !open {
            self.show_filter_modal = false;
        }
    }

    fn ui_fill_gangue_modal(&mut self, ctx: &egui::Context) {
        if !self.show_fill_gangue_modal {
            return;
        }
        let mut open = true;
        let mut run_fg = false;

        // Exact Batch 2 Screenshot 4 3-step sequence: Size class -> Mineral group -> Min particle size!
        match self.fg_step {
            0 => {
                egui::Window::new("Size class")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Select one and press OK, cancel to exit without selection.");
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(4.0))
                            .show(ui, |ui| {
                                ui.set_min_width(220.0);
                                ui.set_min_height(150.0);
                                for sc in &self.setup.size_classes {
                                    let lbl = sc.label(&self.setup.size_unit);
                                    if ui.selectable_label(self.fg_size_no == sc.no, lbl).clicked() {
                                        self.fg_size_no = sc.no;
                                    }
                                }
                            });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Ok")).clicked() {
                                self.fg_step = 1;
                            }
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Cancel")).clicked() {
                                self.show_fill_gangue_modal = false;
                            }
                        });
                    });
            }
            1 => {
                egui::Window::new("Mineral group")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Select mineral group to fill gangue and press OK:");
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(4.0))
                            .show(ui, |ui| {
                                ui.set_min_width(220.0);
                                ui.set_min_height(140.0);
                                for (i, m) in self.setup.minerals.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.fg_mineral_idx == i, format!("{} ({})", m.code, m.mineral))
                                        .clicked()
                                    {
                                        self.fg_mineral_idx = i;
                                    }
                                }
                            });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Ok")).clicked() {
                                self.fg_step = 2;
                            }
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Cancel")).clicked() {
                                self.show_fill_gangue_modal = false;
                            }
                        });
                    });
            }
            _ => {
                let m_code = self
                    .setup
                    .minerals
                    .get(self.fg_mineral_idx)
                    .map(|m| m.code.as_str())
                    .unwrap_or("Gangue");
                egui::Window::new("Min particle size")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Give minimum particle size of the fraction (if particle is smaller it will be increased to the minimum size with {m_code})"
                        ));
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_sized([120.0, 24.0], egui::TextEdit::singleline(&mut self.fg_min_size_str));
                            ui.add_space(20.0);
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Ok")).clicked() {
                                run_fg = true;
                            }
                            if ui.add_sized([74.0, 24.0], egui::Button::new("Cancel")).clicked() {
                                self.show_fill_gangue_modal = false;
                            }
                        });
                    });
            }
        }

        if run_fg {
            if let Some(a) = self.active_stream_idx {
                let min_sz = self.fg_min_size_str.trim().replace(',', ".").parse::<f64>().unwrap_or(10.0);
                let n = run_fill_gangue(
                    &mut self.streams[a],
                    &self.setup,
                    self.fg_size_no,
                    self.fg_mineral_idx,
                    min_sz,
                );
                self.status_msg = Some(format!("Fill Gangue increased {n} particle(s) up to {min_sz} um."));
            }
            self.show_fill_gangue_modal = false;
            self.fg_step = 0;
        } else if !open {
            self.show_fill_gangue_modal = false;
            self.fg_step = 0;
        }
    }

    fn ui_lib_adjust_modal(&mut self, ctx: &egui::Context) {
        if !self.show_lib_adjust_modal {
            return;
        }
        let mut open = true;
        let mut do_ok = false;

        // Exact Batch 2 Screenshot 5: two `Select many` windows (1: Mineral groups -> 2: Size classes)
        let title = if self.lib_step == 0 {
            "Select many — Mineral Groups"
        } else {
            "Select many — Size Classes"
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_min_width(185.0);
                            ui.set_min_height(180.0);
                            if self.lib_step == 0 {
                                for (i, m) in self.setup.minerals.iter().enumerate() {
                                    let mut checked = self.lib_selected_minerals.contains(&i);
                                    if ui.checkbox(&mut checked, &m.code).changed() {
                                        if checked {
                                            self.lib_selected_minerals.insert(i);
                                        } else {
                                            self.lib_selected_minerals.remove(&i);
                                        }
                                    }
                                }
                            } else {
                                for sc in &self.setup.size_classes {
                                    let mut checked = self.lib_selected_sizes.contains(&sc.no);
                                    if ui.checkbox(&mut checked, sc.label(&self.setup.size_unit)).changed() {
                                        if checked {
                                            self.lib_selected_sizes.insert(sc.no);
                                        } else {
                                            self.lib_selected_sizes.remove(&sc.no);
                                        }
                                    }
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        if ui.add_sized([82.0, 24.0], egui::Button::new("Sel All")).clicked() {
                            if self.lib_step == 0 {
                                self.lib_selected_minerals = (0..self.setup.minerals.len()).collect();
                            } else {
                                self.lib_selected_sizes = self.setup.size_classes.iter().map(|s| s.no).collect();
                            }
                        }
                        if ui.add_sized([82.0, 24.0], egui::Button::new("Sel None")).clicked() {
                            if self.lib_step == 0 {
                                self.lib_selected_minerals.clear();
                            } else {
                                self.lib_selected_sizes.clear();
                            }
                        }
                        ui.add_space(68.0);
                        if self.lib_step == 1 {
                            ui.label("Step (%):");
                            ui.add_sized([82.0, 20.0], egui::TextEdit::singleline(&mut self.lib_step_pct_str));
                        }
                        if ui.add_sized([82.0, 24.0], egui::Button::new("Cancel")).clicked() {
                            self.show_lib_adjust_modal = false;
                            self.lib_step = 0;
                        }
                        if ui.add_sized([82.0, 24.0], egui::Button::new("OK")).clicked() {
                            if self.lib_step == 0 {
                                self.lib_step = 1;
                            } else {
                                do_ok = true;
                            }
                        }
                    });
                });
            });

        if do_ok {
            if let Some(a) = self.active_stream_idx {
                let mins: Vec<usize> = self.lib_selected_minerals.iter().copied().collect();
                let szs: Vec<usize> = self.lib_selected_sizes.iter().copied().collect();
                let step = self.lib_step_pct_str.trim().replace(',', ".").parse::<f64>().unwrap_or(15.0);
                let n = run_adjust_liberation(
                    &mut self.streams[a],
                    &self.setup,
                    &mins,
                    &szs,
                    self.lib_adjust_increase,
                    step,
                );
                self.status_msg = Some(format!(
                    "{} adjusted {n} liberated class(es).",
                    if self.lib_adjust_increase { "+Lib" } else { "-Lib" }
                ));
            }
            self.show_lib_adjust_modal = false;
            self.lib_step = 0;
        } else if !open {
            self.show_lib_adjust_modal = false;
            self.lib_step = 0;
        }
    }

    fn ui_graphs_modal(&mut self, ctx: &egui::Context) {
        if !self.show_graphs_modal {
            return;
        }
        let Some(aidx) = self.active_stream_idx else {
            self.show_graphs_modal = false;
            return;
        };
        let mut open = true;
        let elems = self.setup.all_elements();
        if self.graph_mineral_idx >= self.setup.minerals.len() {
            self.graph_mineral_idx = 0;
        }
        if self.graph_element_idx >= elems.len() {
            self.graph_element_idx = 0;
        }

        // Exact Batch 2 Screenshots 2, 7, 8, 9, 10 (`Stream Properties` window)!
        egui::Window::new("Stream Properties")
            .open(&mut open)
            .default_size([920.0, 610.0])
            .show(ctx, |ui| {
                // Top Menu Bar: File | Edit | Stream ▾ (exact Screenshot 7!)
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(242, 245, 250))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.menu_button("File", |ui| {
                                if ui.button("💾 Export Stream Excel (.xlsx)").clicked() {
                                    let sname = self.streams[aidx].name.replace(' ', "_");
                                    self.file_path_str = format!("/tmp/{sname}.xlsx");
                                    self.file_modal = FileModalAction::SaveStreamXlsx(aidx);
                                    ui.close_menu();
                                }
                                if ui.button("Close Window").clicked() {
                                    self.show_graphs_modal = false;
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Edit", |ui| {
                                if ui.button("Reset Visible Series").clicked() {
                                    self.graph_hidden_series.clear();
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Stream ▾", |ui| {
                                ui.set_min_width(220.0);
                                ui.horizontal(|ui| {
                                    ui.label("Stream       ");
                                    let aname = self.streams[aidx].name.clone();
                                    egui::ComboBox::from_id_salt("sp_stream_menu_act")
                                        .width(130.0)
                                        .selected_text(aname)
                                        .show_ui(ui, |ui| {
                                            for (i, s) in self.streams.iter().enumerate() {
                                                if ui.selectable_label(self.active_stream_idx == Some(i), &s.name).clicked() {
                                                    self.active_stream_idx = Some(i);
                                                    self.active_workspace_tab = ActiveWorkspaceTab::Stream(i);
                                                }
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Ref Stream ");
                                    let rname = self
                                        .ref_stream_idx
                                        .and_then(|i| self.streams.get(i))
                                        .map(|s| s.name.clone())
                                        .unwrap_or_else(|| "-".into());
                                    egui::ComboBox::from_id_salt("sp_stream_menu_ref")
                                        .width(130.0)
                                        .selected_text(rname)
                                        .show_ui(ui, |ui| {
                                            for (i, s) in self.streams.iter().enumerate() {
                                                if ui.selectable_label(self.ref_stream_idx == Some(i), &s.name).clicked() {
                                                    self.ref_stream_idx = Some(i);
                                                }
                                            }
                                        });
                                });
                            });

                            ui.separator();
                            // Context-specific options bar right next to Stream menu
                            match self.graph_kind {
                                GraphKind::Psd => {
                                    ui.selectable_value(&mut self.graph_psd_cumulative, true, "Cumulative passing");
                                    ui.selectable_value(&mut self.graph_psd_cumulative, false, "Continuous passing");
                                }
                                GraphKind::Md => {
                                    let m_lbl = self.setup.minerals.get(self.graph_mineral_idx).map(|m| m.code.as_str()).unwrap_or("-");
                                    ui.label("Mineral:");
                                    egui::ComboBox::from_id_salt("geo_g_md_min").width(74.0).selected_text(m_lbl).show_ui(ui, |ui| {
                                        for (i, m) in self.setup.minerals.iter().enumerate() {
                                            if ui.selectable_value(&mut self.graph_mineral_idx, i, &m.code).clicked() {
                                                self.graph_hidden_series.clear();
                                            }
                                        }
                                    });
                                    ui.selectable_value(&mut self.graph_each_size_100, true, "Each size 100%");
                                    ui.selectable_value(&mut self.graph_each_size_100, false, "Total 100%");
                                }
                                GraphKind::Ed => {
                                    let el_lbl = elems.get(self.graph_element_idx).map(|s| s.as_str()).unwrap_or("-");
                                    ui.label("Element:");
                                    egui::ComboBox::from_id_salt("geo_g_ed_el").width(64.0).selected_text(el_lbl).show_ui(ui, |ui| {
                                        for (i, el) in elems.iter().enumerate() {
                                            ui.selectable_value(&mut self.graph_element_idx, i, el);
                                        }
                                    });
                                    ui.selectable_value(&mut self.graph_each_size_100, true, "Each size 100%");
                                    ui.selectable_value(&mut self.graph_each_size_100, false, "Total 100%");
                                }
                                GraphKind::Lib => {
                                    let m_lbl = self.setup.minerals.get(self.graph_mineral_idx).map(|m| m.code.as_str()).unwrap_or("-");
                                    ui.label("Mineral:");
                                    egui::ComboBox::from_id_salt("geo_g_lib_min").width(74.0).selected_text(m_lbl).show_ui(ui, |ui| {
                                        for (i, m) in self.setup.minerals.iter().enumerate() {
                                            ui.selectable_value(&mut self.graph_mineral_idx, i, &m.code);
                                        }
                                    });
                                    ui.label("Classes:");
                                    ui.add(egui::DragValue::new(&mut self.graph_lib_classes).range(4..=15));
                                    ui.checkbox(&mut self.graph_lib_first_100, "First class 100-100");
                                }
                                GraphKind::Lim => {
                                    if self.graph_lim_by_element {
                                        let el_lbl = elems.get(self.graph_element_idx).map(|s| s.as_str()).unwrap_or("-");
                                        ui.label("Element:");
                                        egui::ComboBox::from_id_salt("geo_g_lim_el").width(64.0).selected_text(el_lbl).show_ui(ui, |ui| {
                                            for (i, el) in elems.iter().enumerate() {
                                                ui.selectable_value(&mut self.graph_element_idx, i, el);
                                            }
                                        });
                                    } else {
                                        let m_lbl = self.setup.minerals.get(self.graph_mineral_idx).map(|m| m.code.as_str()).unwrap_or("-");
                                        ui.label("Mineral:");
                                        egui::ComboBox::from_id_salt("geo_g_lim_min").width(74.0).selected_text(m_lbl).show_ui(ui, |ui| {
                                            for (i, m) in self.setup.minerals.iter().enumerate() {
                                                ui.selectable_value(&mut self.graph_mineral_idx, i, &m.code);
                                            }
                                        });
                                    }
                                    ui.checkbox(&mut self.graph_lim_grade_on_x, "Grade on X-axis");
                                }
                                _ => {}
                            }
                        });
                    });

                ui.add_space(4.0);

                // Main Split: Left Vertical Control Sidebar (exact Screenshots 2 & 8!) + Right Chart/Table Canvas
                let cur_idx = self.active_stream_idx.unwrap_or(aidx).min(self.streams.len() - 1);
                let avail_h = ui.available_height().max(420.0);

                ui.horizontal_top(|ui| {
                    // LEFT SIDEBAR: --Show-- / -Analysis- / --Chart--
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(243, 245, 248))
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(6.0))
                        .show(ui, |ui| {
                            ui.set_min_width(96.0);
                            ui.set_min_height(avail_h - 6.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 3.0;
                                ui.label(egui::RichText::new("--Show--").size(10.0).color(egui::Color32::GRAY));
                                if sp_side_btn(ui, "Table", self.graph_as_table) {
                                    self.graph_as_table = true;
                                }
                                if sp_side_btn(ui, "Chart", !self.graph_as_table) {
                                    self.graph_as_table = false;
                                }

                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("-Analysis-").size(10.0).color(egui::Color32::GRAY));
                                for (lbl, gk, lim_el) in [
                                    (">MC", GraphKind::Mc, false),
                                    (">EC", GraphKind::Ec, false),
                                    (">PSD", GraphKind::Psd, false),
                                    (">MD", GraphKind::Md, false),
                                    (">ED", GraphKind::Ed, false),
                                    (">LIB", GraphKind::Lib, false),
                                    (">LIM(m)", GraphKind::Lim, false),
                                    (">LIM(e)", GraphKind::Lim, true),
                                ] {
                                    let is_act = self.graph_kind == gk
                                        && (gk != GraphKind::Lim || self.graph_lim_by_element == lim_el);
                                    if sp_side_btn(ui, lbl, is_act) {
                                        if self.graph_kind != gk || (gk == GraphKind::Lim && self.graph_lim_by_element != lim_el) {
                                            self.graph_hidden_series.clear();
                                        }
                                        self.graph_kind = gk;
                                        if gk == GraphKind::Lim {
                                            self.graph_lim_by_element = lim_el;
                                        }
                                    }
                                }

                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("--Chart--").size(10.0).color(egui::Color32::GRAY));
                                if sp_side_btn(ui, "-X-Axis", self.graph_psd_log_x) {
                                    if self.graph_kind == GraphKind::Psd {
                                        self.graph_psd_log_x = !self.graph_psd_log_x;
                                    } else if self.graph_kind == GraphKind::Lim {
                                        self.graph_lim_grade_on_x = !self.graph_lim_grade_on_x;
                                    }
                                }
                                if sp_side_btn(ui, "-Y-Axis", self.graph_y_auto_zoom) {
                                    self.graph_y_auto_zoom = !self.graph_y_auto_zoom;
                                }
                                if sp_side_btn(ui, "Legend", self.graph_show_legend) {
                                    self.graph_show_legend = !self.graph_show_legend;
                                }
                            });
                        });

                    ui.add_space(6.0);

                    // RIGHT CANVAS: Chart or Table
                    ui.vertical(|ui| {
                        let st = &self.streams[cur_idx];
                        match self.graph_kind {
                            GraphKind::Mc => {
                                let td = build_mc_graph(st, &self.setup);
                                if self.graph_as_table {
                                    draw_table_data(ui, &td);
                                } else {
                                    draw_hsc_stacked_bar_chart(
                                        ui,
                                        &td,
                                        "Weight % of mineral",
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Ec => {
                                let td = build_ec_graph(st, &self.setup);
                                if self.graph_as_table {
                                    draw_table_data(ui, &td);
                                } else {
                                    // Batch 2 Screenshot 9 top: Chemical Composition is a STACKED bar chart!
                                    draw_hsc_stacked_bar_chart(
                                        ui,
                                        &td,
                                        "Weight %",
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Psd => {
                                if self.graph_as_table {
                                    draw_psd_full_table(ui, st, &self.setup);
                                } else {
                                    let (xs, td) = build_psd_graph(st, &self.setup, self.graph_psd_cumulative);
                                    draw_hsc_psd_chart(
                                        ui,
                                        &xs,
                                        &td,
                                        self.graph_psd_log_x,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Md => {
                                let td = build_md_graph(st, &self.setup, self.graph_mineral_idx, self.graph_each_size_100);
                                if self.graph_as_table {
                                    draw_table_data(ui, &td);
                                } else {
                                    draw_hsc_stacked_bar_chart(
                                        ui,
                                        &td,
                                        "Weight % of mineral in particle type",
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Ed => {
                                let el = elems.get(self.graph_element_idx).map(|s| s.as_str()).unwrap_or("S");
                                let td = build_ed_graph(st, &self.setup, el, self.graph_each_size_100);
                                if self.graph_as_table {
                                    draw_table_data(ui, &td);
                                } else {
                                    draw_hsc_stacked_bar_chart(
                                        ui,
                                        &td,
                                        "Weight % of element in mineral",
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Lib => {
                                if self.graph_as_table {
                                    let td = build_lib_graph(
                                        st,
                                        &self.setup,
                                        self.graph_mineral_idx,
                                        self.graph_lib_classes,
                                        self.graph_lib_first_100,
                                    );
                                    draw_table_data(ui, &td);
                                } else {
                                    // Batch 2 Screenshot 10 center: continuous XY 0..100 Cumulative liberation
                                    let (title, x_lbl, names, curves) = build_lib_xy_graph(
                                        st,
                                        &self.setup,
                                        self.graph_mineral_idx,
                                        self.graph_lib_classes,
                                        self.graph_lib_first_100,
                                    );
                                    draw_hsc_xy_curves_chart(
                                        ui,
                                        &title,
                                        &x_lbl,
                                        "Cumulative yield, %",
                                        &names,
                                        &curves,
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                            GraphKind::Lim => {
                                let el_opt = if self.graph_lim_by_element {
                                    elems.get(self.graph_element_idx).map(|s| s.as_str())
                                } else {
                                    None
                                };
                                let (title, names, raw_curves) =
                                    build_lim_graph(st, &self.setup, Some(self.graph_mineral_idx), el_opt);
                                if self.graph_as_table {
                                    egui::ScrollArea::both().show(ui, |ui| {
                                        ui.strong(&title);
                                        for (si, name) in names.iter().enumerate() {
                                            if let Some(pts) = raw_curves.get(si) {
                                                if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                                                    ui.label(format!(
                                                        "{name}: Grade {:.2}% @ Rec {:.1}%  ->  Grade {:.2}% @ Rec {:.1}%",
                                                        first.1, first.0, last.1, last.0
                                                    ));
                                                }
                                            }
                                        }
                                    });
                                } else {
                                    // Batch 2 Screenshot 10 bottom: X = Grade %, Y = Recovery % (by default)
                                    let target_sym = el_opt.unwrap_or_else(|| {
                                        self.setup
                                            .minerals
                                            .get(self.graph_mineral_idx)
                                            .map(|m| m.code.as_str())
                                            .unwrap_or("Min")
                                    });
                                    let (x_lbl, y_lbl, oriented): (String, String, Vec<Vec<(f64, f64)>>) =
                                        if self.graph_lim_grade_on_x {
                                            (
                                                format!("{target_sym} grade, %"),
                                                format!("{target_sym} recovery, %"),
                                                raw_curves
                                                    .iter()
                                                    .map(|c| c.iter().map(|&(r, g)| (g, r)).collect())
                                                    .collect(),
                                            )
                                        } else {
                                            (
                                                format!("{target_sym} recovery, %"),
                                                format!("{target_sym} grade, %"),
                                                raw_curves,
                                            )
                                        };
                                    draw_hsc_xy_curves_chart(
                                        ui,
                                        &title,
                                        &x_lbl,
                                        &y_lbl,
                                        &names,
                                        &oriented,
                                        self.graph_y_auto_zoom,
                                        self.graph_show_legend,
                                        &mut self.graph_hidden_series,
                                    );
                                }
                            }
                        }
                    });
                });
            });

        if !open {
            self.show_graphs_modal = false;
        }
    }

    fn ui_file_modal(&mut self, ctx: &egui::Context) {
        if self.file_modal == FileModalAction::None {
            return;
        }
        let mut open = true;
        let title = match self.file_modal {
            FileModalAction::SaveMinSetupXlsx => "Save Mineral Setup (MinSetup.xlsx)",
            FileModalAction::OpenProjectJson | FileModalAction::OpenStreamHsc => {
                "Open HSCStream / Project (.hscstream)"
            }
            FileModalAction::SaveStreamXlsx(_) => "Save Stream as Excel (.xlsx)",
            FileModalAction::SaveStreamHsc(_) => "Save Stream as HSCStream (.hscstream)",
            FileModalAction::None => return,
        };
        let mut run = false;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("File path:");
                ui.add_sized([360.0, 22.0], egui::TextEdit::singleline(&mut self.file_path_str));
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        run = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.file_modal = FileModalAction::None;
                    }
                });
            });
        if run {
            let path = self.file_path_str.clone();
            match self.file_modal {
                FileModalAction::SaveMinSetupXlsx => {
                    let bytes = export_minsetup_xlsx(&self.setup);
                    match std::fs::write(&path, bytes) {
                        Ok(()) => {
                            self.status_err = None;
                            self.status_msg = Some(format!("Saved MinSetup workbook to {path}"));
                        }
                        Err(e) => self.status_err = Some(format!("Save error: {e}")),
                    }
                }
                FileModalAction::SaveStreamXlsx(idx) if idx < self.streams.len() => {
                    let bytes = export_stream_xlsx(&self.streams[idx], &self.setup);
                    match std::fs::write(&path, bytes) {
                        Ok(()) => {
                            self.status_err = None;
                            self.status_msg = Some(format!("Saved stream Excel workbook to {path}"));
                        }
                        Err(e) => self.status_err = Some(format!("Save error: {e}")),
                    }
                }
                FileModalAction::SaveStreamHsc(idx) if idx < self.streams.len() => {
                    let pkg = HscStreamPackage {
                        setup: self.setup.clone(),
                        stream: self.streams[idx].clone(),
                    };
                    match pkg.to_json().and_then(|j| std::fs::write(&path, j).map_err(|e| e.to_string())) {
                        Ok(()) => {
                            self.status_err = None;
                            self.status_msg = Some(format!("Saved HSCStream package to {path}"));
                        }
                        Err(e) => self.status_err = Some(format!("Save error: {e}")),
                    }
                }
                FileModalAction::OpenProjectJson | FileModalAction::OpenStreamHsc => {
                    match std::fs::read_to_string(&path).and_then(|s| {
                        HscStreamPackage::from_json(&s)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    }) {
                        Ok(pkg) => {
                            self.setup = pkg.setup;
                            self.streams.push(pkg.stream);
                            let ni = self.streams.len() - 1;
                            self.active_stream_idx = Some(ni);
                            self.active_workspace_tab = ActiveWorkspaceTab::Stream(ni);
                            self.status_err = None;
                            self.status_msg = Some(format!("Opened HSCStream from {path}"));
                        }
                        Err(e) => self.status_err = Some(format!("Open error: {e}")),
                    }
                }
                _ => {}
            }
            self.file_modal = FileModalAction::None;
        } else if !open {
            self.file_modal = FileModalAction::None;
        }
    }
}

// ============================================================================
// Graph & Widget Helpers (Exact HSC Geo Stream Properties Palette & Layout)
// ============================================================================

/// Authentic HSC Geo / Outotec chart color palette (Batch 2 Screenshots 2, 8, 9, 10):
/// Green (Liberated/Gn), Sky Blue (Binary/Sp), Charcoal Gray (Dol/Py), Olive/Khaki,
/// Orange, Deep Red, Dark Teal, Slate Blue, Magenta, Amber.
const PALETTE: [egui::Color32; 10] = [
    egui::Color32::from_rgb(104, 168, 54),  // Green (Gn / Liberated)
    egui::Color32::from_rgb(0, 149, 217),   // Cyan-Blue (Sp / Binary with Sp)
    egui::Color32::from_rgb(85, 85, 85),    // Charcoal Gray (Py / Binary with Dol)
    egui::Color32::from_rgb(156, 147, 80),  // Olive/Khaki (Dol / Ternary)
    egui::Color32::from_rgb(230, 138, 0),   // Orange (Others / Qtz)
    egui::Color32::from_rgb(204, 24, 24),   // Deep Red
    egui::Color32::from_rgb(14, 116, 144),  // Dark Teal
    egui::Color32::from_rgb(59, 95, 168),   // Steel Blue
    egui::Color32::from_rgb(147, 51, 234),  // Purple
    egui::Color32::from_rgb(217, 119, 6),   // Amber
];

fn sp_side_btn(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .size(10.5)
                .color(egui::Color32::from_rgb(20, 28, 42)),
        )
        .min_size(egui::vec2(84.0, 22.0))
        .fill(if active {
            egui::Color32::from_rgb(196, 220, 250)
        } else {
            egui::Color32::from_rgb(245, 247, 250)
        }),
    )
    .clicked()
}

fn draw_legend_checkbox_bar(
    ui: &mut egui::Ui,
    names: &[String],
    hidden: &mut BTreeSet<usize>,
) {
    egui::Frame::none()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 128, 140)))
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (si, name) in names.iter().enumerate() {
                    let mut visible = !hidden.contains(&si);
                    let col = PALETTE[si % PALETTE.len()];
                    if ui.checkbox(&mut visible, "").changed() {
                        if visible {
                            hidden.remove(&si);
                        } else {
                            hidden.insert(si);
                        }
                    }
                    ui.colored_label(col, "■");
                    ui.label(egui::RichText::new(name).size(10.5));
                    ui.add_space(6.0);
                }
            });
        });
}

fn draw_table_data(ui: &mut egui::Ui, td: &LiberationTableData) {
    ui.strong(&td.title);
    egui::ScrollArea::both().show(ui, |ui| {
        ui.horizontal(|ui| {
            header_cell(ui, "Series", 125.0);
            for cat in &td.categories {
                header_cell(ui, cat, 82.0);
            }
        });
        for (si, name) in td.series_names.iter().enumerate() {
            ui.horizontal(|ui| {
                widget_cell(ui, 125.0, egui::Color32::from_rgb(242, 244, 248), false, |ui| {
                    ui.label(egui::RichText::new(name).strong().size(10.5));
                });
                for &v in &td.values[si] {
                    widget_cell(ui, 82.0, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(format!("{v:.2}")).monospace().size(10.5));
                    });
                }
            });
        }
    });
}

/// Exact Batch 2 Screenshot 8 Table layout for PSD:
/// `Particle Size Distribution` (`Fraction`, `Bottom size`, `Top size`, `AVG size`),
/// `Distribution` (`Solids wt%`, minerals), and `Cumulative distribution` (`Solids wt%`, minerals).
fn draw_psd_full_table(ui: &mut egui::Ui, stream: &LiberationStream, setup: &MineralSetup) {
    let (_xs, dist_td) = build_psd_graph(stream, setup, false);
    let (_xs2, cum_td) = build_psd_graph(stream, setup, true);
    let n_sz = setup.size_classes.len();
    let lbl_w = 115.0;
    let col_w = 78.0;

    egui::ScrollArea::both().id_salt("geo_psd_table_scroll").show(ui, |ui| {
        ui.horizontal(|ui| {
            row_num_cell(ui, 0, 26.0);
            header_cell(ui, "A", 20.0);
            header_cell(ui, "B", lbl_w);
            for s in 0..n_sz {
                header_cell(ui, &col_letter(s + 2), col_w);
            }
        });

        ui.label(egui::RichText::new("Particle Size Distribution").strong().size(11.5));
        // Fraction / Bottom / Top / AVG
        let meta_rows: [(&str, Vec<String>); 4] = [
            ("Fraction", setup.size_classes.iter().map(|s| s.label(&setup.size_unit)).collect()),
            ("Bottom size", setup.size_classes.iter().map(|s| format!("{:.2}", s.lower)).collect()),
            ("Top size", setup.size_classes.iter().map(|s| format!("{:.2}", s.upper)).collect()),
            ("AVG size", setup.size_classes.iter().map(|s| format!("{:.2}", s.average_size())).collect()),
        ];
        for (ri, (lbl, vals)) in meta_rows.iter().enumerate() {
            ui.horizontal(|ui| {
                row_num_cell(ui, 2 + ri, 26.0);
                widget_cell(ui, 20.0, egui::Color32::WHITE, false, |_| {});
                widget_cell(ui, lbl_w, egui::Color32::from_rgb(245, 247, 250), false, |ui| {
                    ui.label(*lbl);
                });
                for v in vals {
                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(v).monospace().size(10.5));
                    });
                }
            });
        }

        ui.add_space(6.0);
        ui.label(egui::RichText::new("Distribution").strong().size(11.5));
        for (si, name) in dist_td.series_names.iter().enumerate() {
            let display_name = if si == 0 { "Solids wt%" } else { name.as_str() };
            ui.horizontal(|ui| {
                row_num_cell(ui, 7 + si, 26.0);
                widget_cell(ui, 20.0, egui::Color32::WHITE, false, |_| {});
                widget_cell(ui, lbl_w, egui::Color32::from_rgb(245, 247, 250), false, |ui| {
                    ui.label(display_name);
                });
                for &v in &dist_td.values[si] {
                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(format!("{v:.2}")).monospace().size(10.5));
                    });
                }
            });
        }

        ui.add_space(6.0);
        ui.label(egui::RichText::new("Cumulative distribution").strong().size(11.5));
        for (si, name) in cum_td.series_names.iter().enumerate() {
            let display_name = if si == 0 { "Solids wt%" } else { name.as_str() };
            ui.horizontal(|ui| {
                row_num_cell(ui, 18 + si, 26.0);
                widget_cell(ui, 20.0, egui::Color32::WHITE, false, |_| {});
                widget_cell(ui, lbl_w, egui::Color32::from_rgb(245, 247, 250), false, |ui| {
                    ui.label(display_name);
                });
                for &v in &cum_td.values[si] {
                    widget_cell(ui, col_w, egui::Color32::WHITE, false, |ui| {
                        ui.label(egui::RichText::new(format!("{v:.2}")).monospace().size(10.5));
                    });
                }
            });
        }
    });
}

/// Stacked bar chart with in-bar labels and interactive checkbox legend (exact Batch 2 Screenshots 2, 8, 9, 10!).
fn draw_hsc_stacked_bar_chart(
    ui: &mut egui::Ui,
    td: &LiberationTableData,
    y_title: &str,
    auto_zoom_y: bool,
    show_legend: bool,
    hidden: &mut BTreeSet<usize>,
) {
    ui.vertical_centered(|ui| {
        ui.strong(&td.title);
    });

    let avail = ui.available_size();
    let leg_reserve = if show_legend { 42.0 } else { 8.0 };
    let chart_h = (avail.y - leg_reserve).max(250.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(320.0), chart_h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 56.0, rect.top() + 10.0),
        egui::pos2(rect.right() - 14.0, rect.bottom() - 42.0),
    );
    if plot.width() < 40.0 || plot.height() < 40.0 {
        return;
    }
    p.rect_filled(plot, 0.0, egui::Color32::WHITE);
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

    let n_cat = td.categories.len().max(1);
    let mut max_stack = if auto_zoom_y { 1.0f64 } else { 100.0f64 };
    for c in 0..n_cat {
        let s: f64 = td
            .values
            .iter()
            .enumerate()
            .filter(|(si, _)| !hidden.contains(si))
            .map(|(_, r)| r.get(c).copied().unwrap_or(0.0).max(0.0))
            .sum();
        max_stack = max_stack.max(s);
    }
    if auto_zoom_y {
        max_stack = (max_stack * 1.05).max(1.0);
    }

    let y_step = nice_step(max_stack / 10.0);
    let mut yv = 0.0;
    let mut guard = 0;
    while yv <= max_stack + y_step * 0.01 && guard < 40 {
        let py = plot.bottom() - ((yv / max_stack) as f32) * plot.height();
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_rgb(230, 234, 240)));
        p.line_segment([egui::pos2(plot.left() - 4.0, py), egui::pos2(plot.left(), py)], egui::Stroke::new(1.0, egui::Color32::BLACK));
        p.text(egui::pos2(plot.left() - 6.0, py), egui::Align2::RIGHT_CENTER, format_tick(yv), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
        yv += y_step;
        guard += 1;
    }

    draw_rotated_y_title(&p, rect.left() + 14.0, plot.center().y, y_title, egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));
    p.text(egui::pos2(plot.center().x, rect.bottom() - 8.0), egui::Align2::CENTER_CENTER, "Fraction", egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));

    let slot_w = plot.width() / n_cat as f32;
    let bar_w = (slot_w * 0.80).max(16.0);
    for (c, cat) in td.categories.iter().enumerate() {
        let cx = plot.left() + (c as f32 + 0.5) * slot_w;
        let mut acc_y = 0.0;
        for (si, row) in td.values.iter().enumerate() {
            if hidden.contains(&si) {
                continue;
            }
            let v = row.get(c).copied().unwrap_or(0.0).max(0.0);
            if v <= 1e-6 {
                continue;
            }
            let y_bot = plot.bottom() - ((acc_y / max_stack) as f32) * plot.height();
            let y_top = plot.bottom() - (((acc_y + v) / max_stack) as f32) * plot.height();
            acc_y += v;
            let bar_r = egui::Rect::from_min_max(egui::pos2(cx - bar_w * 0.5, y_top), egui::pos2(cx + bar_w * 0.5, y_bot));
            p.rect_filled(bar_r, 0.0, PALETTE[si % PALETTE.len()]);
            p.rect_stroke(bar_r, 0.0, egui::Stroke::new(0.8, egui::Color32::from_rgb(35, 40, 48)));

            // In-segment white label with dark shadow matching HSC Geo Screenshots 2, 8, 9, 10!
            if bar_r.height() >= 15.0 {
                let lbl = &td.series_names[si];
                let pos = egui::pos2(cx, bar_r.top() + 9.0);
                p.text(pos + egui::vec2(1.0, 1.0), egui::Align2::CENTER_CENTER, lbl, egui::FontId::proportional(9.5), egui::Color32::from_rgba_unmultiplied(0, 0, 0, 175));
                p.text(pos, egui::Align2::CENTER_CENTER, lbl, egui::FontId::proportional(9.5), egui::Color32::WHITE);
            }
        }
        p.text(egui::pos2(cx, plot.bottom() + 8.0), egui::Align2::CENTER_TOP, cat, egui::FontId::proportional(9.0), egui::Color32::from_rgb(35, 42, 54));
    }

    if show_legend {
        ui.add_space(4.0);
        draw_legend_checkbox_bar(ui, &td.series_names, hidden);
    }
}

/// PSD chart with true logarithmic X-axis (`10.0 .. 100.0 .. 300.0`) matching Batch 2 Screenshot 9 center!
fn draw_hsc_psd_chart(
    ui: &mut egui::Ui,
    xs: &[f64],
    td: &LiberationTableData,
    log_x: bool,
    show_legend: bool,
    hidden: &mut BTreeSet<usize>,
) {
    ui.vertical_centered(|ui| {
        ui.strong(&td.title);
    });
    let avail = ui.available_size();
    let leg_reserve = if show_legend { 42.0 } else { 8.0 };
    let chart_h = (avail.y - leg_reserve).max(250.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(320.0), chart_h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 56.0, rect.top() + 10.0),
        egui::pos2(rect.right() - 16.0, rect.bottom() - 40.0),
    );
    if plot.width() < 40.0 || plot.height() < 40.0 || xs.is_empty() {
        return;
    }
    p.rect_filled(plot, 0.0, egui::Color32::WHITE);
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

    let x_min = xs.iter().copied().fold(f64::INFINITY, f64::min).max(1.0);
    let x_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(x_min + 1.0);
    let lx0 = x_min.log10();
    let lx1 = x_max.log10().max(lx0 + 0.1);

    let to_x = |x: f64| -> f32 {
        let frac = if log_x {
            ((x.max(x_min).log10() - lx0) / (lx1 - lx0)).clamp(0.0, 1.0)
        } else {
            ((x - x_min) / (x_max - x_min)).clamp(0.0, 1.0)
        };
        plot.left() + (frac as f32) * plot.width()
    };
    let y_max = 110.0f64;
    let to_y = |y: f64| plot.bottom() - ((y.clamp(0.0, y_max) / y_max) as f32) * plot.height();

    // Horizontal grid & Y ticks (0..110 step 10)
    for k in 0..=11 {
        let yv = k as f64 * 10.0;
        let py = to_y(yv);
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_rgb(232, 236, 242)));
        p.line_segment([egui::pos2(plot.left() - 4.0, py), egui::pos2(plot.left(), py)], egui::Stroke::new(1.0, egui::Color32::BLACK));
        p.text(egui::pos2(plot.left() - 6.0, py), egui::Align2::RIGHT_CENTER, format!("{yv:.0}"), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
    }

    // X ticks (decade markers for log scale: 10.0, 100.0, 1000.0)
    if log_x {
        for &xv in &[1.0, 10.0, 100.0, 1000.0] {
            if xv >= x_min * 0.99 && xv <= x_max * 1.01 {
                let px = to_x(xv);
                p.line_segment([egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())], egui::Stroke::new(0.7, egui::Color32::from_rgb(225, 230, 238)));
                p.line_segment([egui::pos2(px, plot.bottom()), egui::pos2(px, plot.bottom() + 4.0)], egui::Stroke::new(1.0, egui::Color32::BLACK));
                p.text(egui::pos2(px, plot.bottom() + 7.0), egui::Align2::CENTER_TOP, format!("{xv:.1}"), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
            }
        }
    } else {
        let x_step = nice_step((x_max - x_min) / 5.0);
        let mut xv = (x_min / x_step).ceil() * x_step;
        while xv <= x_max + 1e-6 {
            let px = to_x(xv);
            p.line_segment([egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())], egui::Stroke::new(0.6, egui::Color32::from_rgb(232, 236, 242)));
            p.text(egui::pos2(px, plot.bottom() + 7.0), egui::Align2::CENTER_TOP, format_tick(xv), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
            xv += x_step;
        }
    }

    draw_rotated_y_title(&p, rect.left() + 14.0, plot.center().y, &td.y_label, egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));
    p.text(egui::pos2(plot.center().x, rect.bottom() - 8.0), egui::Align2::CENTER_CENTER, if log_x { "Size (log)" } else { "Size" }, egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));

    let mut legend_names = td.series_names.clone();
    if let Some(first) = legend_names.first_mut() {
        *first = "Solids wt%".into();
    }

    for (si, row) in td.values.iter().enumerate() {
        if hidden.contains(&si) {
            continue;
        }
        let col = PALETTE[si % PALETTE.len()];
        let pts: Vec<egui::Pos2> = xs
            .iter()
            .zip(row.iter())
            .map(|(&x, &y)| egui::pos2(to_x(x), to_y(y)))
            .collect();
        if pts.len() >= 2 {
            p.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.8, col)));
        }
        for pt in pts {
            p.circle_filled(pt, 3.3, col);
        }
    }

    if show_legend {
        ui.add_space(4.0);
        draw_legend_checkbox_bar(ui, &legend_names, hidden);
    }
}

/// Multi-series XY curve chart with circular markers and interactive checkbox legend (exact Batch 2 Screenshot 10: LIB & LIM!).
#[allow(clippy::too_many_arguments)]
fn draw_hsc_xy_curves_chart(
    ui: &mut egui::Ui,
    title: &str,
    x_label: &str,
    y_label: &str,
    names: &[String],
    curves: &[Vec<(f64, f64)>],
    auto_zoom_y: bool,
    show_legend: bool,
    hidden: &mut BTreeSet<usize>,
) {
    ui.vertical_centered(|ui| {
        ui.strong(title);
    });
    let avail = ui.available_size();
    let leg_reserve = if show_legend { 42.0 } else { 8.0 };
    let chart_h = (avail.y - leg_reserve).max(250.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(320.0), chart_h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::WHITE);

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 56.0, rect.top() + 10.0),
        egui::pos2(rect.right() - 16.0, rect.bottom() - 40.0),
    );
    if plot.width() < 40.0 || plot.height() < 40.0 {
        return;
    }
    p.rect_filled(plot, 0.0, egui::Color32::WHITE);
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

    let mut x_max = 100.0f64;
    let mut y_min = if auto_zoom_y { f64::INFINITY } else { 0.0 };
    let mut y_max = 100.0f64;
    for (si, curve) in curves.iter().enumerate() {
        if hidden.contains(&si) {
            continue;
        }
        for &(x, y) in curve {
            if x.is_finite() && y.is_finite() {
                x_max = x_max.max(x);
                if auto_zoom_y {
                    y_min = y_min.min(y);
                }
                y_max = y_max.max(y);
            }
        }
    }
    if !y_min.is_finite() || y_min >= y_max - 1.0 {
        y_min = 0.0;
    } else if auto_zoom_y {
        y_min = (y_min - 5.0).max(0.0);
    }
    let y_span = (y_max - y_min).max(1.0);

    let to_x = |x: f64| plot.left() + ((x.clamp(0.0, x_max) / x_max) as f32) * plot.width();
    let to_y = |y: f64| plot.bottom() - (((y - y_min).clamp(0.0, y_span) / y_span) as f32) * plot.height();

    // Grid & ticks
    let x_step = nice_step(x_max / 5.0);
    let mut xv = 0.0;
    let mut xg = 0;
    while xv <= x_max + 1e-6 && xg < 30 {
        let px = to_x(xv);
        p.line_segment([egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())], egui::Stroke::new(0.6, egui::Color32::from_rgb(232, 236, 242)));
        p.line_segment([egui::pos2(px, plot.bottom()), egui::pos2(px, plot.bottom() + 4.0)], egui::Stroke::new(1.0, egui::Color32::BLACK));
        p.text(egui::pos2(px, plot.bottom() + 7.0), egui::Align2::CENTER_TOP, format_tick(xv), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
        xv += x_step;
        xg += 1;
    }
    let y_step = nice_step(y_span / 8.0);
    let mut yv = (y_min / y_step).ceil() * y_step;
    let mut yg = 0;
    while yv <= y_max + y_step * 0.01 && yg < 30 {
        let py = to_y(yv);
        p.line_segment([egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)], egui::Stroke::new(0.6, egui::Color32::from_rgb(232, 236, 242)));
        p.line_segment([egui::pos2(plot.left() - 4.0, py), egui::pos2(plot.left(), py)], egui::Stroke::new(1.0, egui::Color32::BLACK));
        p.text(egui::pos2(plot.left() - 6.0, py), egui::Align2::RIGHT_CENTER, format_tick(yv), egui::FontId::proportional(9.5), egui::Color32::from_rgb(35, 42, 54));
        yv += y_step;
        yg += 1;
    }

    draw_rotated_y_title(&p, rect.left() + 14.0, plot.center().y, y_label, egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));
    p.text(egui::pos2(plot.center().x, rect.bottom() - 8.0), egui::Align2::CENTER_CENTER, x_label, egui::FontId::proportional(10.5), egui::Color32::from_rgb(30, 38, 50));

    for (si, curve) in curves.iter().enumerate() {
        if hidden.contains(&si) || curve.len() < 2 {
            continue;
        }
        let col = PALETTE[si % PALETTE.len()];
        let pts: Vec<egui::Pos2> = curve.iter().map(|&(x, y)| egui::pos2(to_x(x), to_y(y))).collect();
        p.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.6, col)));
        // Draw circular markers (decimated if very dense like LIM)
        let stride = (pts.len() / 24).max(1);
        for (pi, &pt) in pts.iter().enumerate() {
            if pi % stride == 0 || pi + 1 == pts.len() {
                p.circle_filled(pt, 3.0, col);
            }
        }
    }

    if show_legend {
        ui.add_space(4.0);
        draw_legend_checkbox_bar(ui, names, hidden);
    }
}

fn geo_nav_btn(ui: &mut egui::Ui, icon: &str, label: &str, active: bool, on_click: impl FnOnce()) {
    let txt = egui::RichText::new(format!("{icon}  {label}"))
        .size(11.0)
        .color(if active {
            egui::Color32::from_rgb(29, 78, 216)
        } else {
            egui::Color32::from_rgb(35, 45, 60)
        });
    let txt = if active { txt.underline().strong() } else { txt };
    if ui.add(egui::SelectableLabel::new(active, txt)).clicked() {
        on_click();
    }
}

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

fn small_rib_btn(ui: &mut egui::Ui, label: &str, width: f32) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(10.0))
            .min_size(egui::vec2(width, 20.0))
            .fill(egui::Color32::from_rgb(248, 250, 253)),
    )
    .clicked()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_liberation_ui_stream_lifecycle_and_combine_work() {
        let mut st = GeoLiberationModuleState::default();
        assert_eq!(st.streams.len(), 3);
        assert_eq!(st.setup.size_classes.len(), 7);

        // Copy & close stream
        st.copy_stream(1);
        assert_eq!(st.streams.len(), 4);
        assert_eq!(st.streams[3].name, "Copy of Concentrate");
        st.close_stream(3);
        assert_eq!(st.streams.len(), 3);

        // Run Basic Binning + Missing on Feed
        run_basic_binning(&mut st.streams[0], &st.setup);
        let filled = run_extrapolate_missing(&mut st.streams[0], &st.setup).unwrap();
        assert_eq!(filled, 2);
        let sum = st.streams[0].compute_summary(&st.setup);
        assert!((sum.total_wt_bulk - 100.0).abs() < 1e-2);
    }
}
