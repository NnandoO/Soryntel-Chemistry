use eframe::egui;
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::model::*;
use soryntel_thermo::sim::presets::empty_project;
use soryntel_thermo::sim::reports::SimReportWorkbook;
use soryntel_thermo::sim::solver::LogFilterLevel;

pub const SIM_WIN_BG: egui::Color32 = egui::Color32::from_rgb(240, 240, 240);
pub const SIM_TOOLBAR_BG: egui::Color32 = egui::Color32::from_rgb(236, 238, 243);
pub const SIM_BORDER: egui::Color32 = egui::Color32::from_rgb(172, 178, 190);
pub const SIM_HEADER_BG: egui::Color32 = egui::Color32::from_rgb(225, 230, 238);
pub const SIM_RULER_BG: egui::Color32 = egui::Color32::from_rgb(248, 249, 252);
pub const SIM_ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(0, 102, 204);
pub const SIM_TEXT_DARK: egui::Color32 = egui::Color32::from_rgb(28, 32, 38);
pub const SIM_MUTED_TEXT: egui::Color32 = egui::Color32::from_rgb(95, 104, 118);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Select,
    Pan,
    AddUnit(UnitKind),
    AddStream,
    AddHeaderLabel,
    AddTextLabel,
    AddTextBox,
    AddPictureBox,
    AddTable,
    AddTitleTable,
    AddLine,
    AddRect,
    AddEllipse,
    AddTriangle,
    AddPolygon,
    PlaceGalleryIcon(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedObject {
    None,
    Unit(usize),
    Stream(usize),
    Table(usize),
    Annotation(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightTopDockTab {
    Properties,
    Inspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightBottomTab {
    Icons,
    Items,
    Names,
    Visibility,
    Controls,
    CellRef,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportModalKind {
    ProjectJson,
    LoadProjectJson,
    LcaJson,
    FlowsheetSvg,
}

#[derive(Debug, Clone)]
pub struct TopToolbarVisibility {
    pub project: bool,
    pub clipboard_undo: bool,
    pub calculations: bool,
    pub sim_calculation: bool,
    pub dynamic_simulation: bool,
    pub dynamic_progress: bool,
    pub font: bool,
    pub text_formatting: bool,
    pub layout: bool,
    pub visualization: bool,
    pub dynamic_collect: bool,
    pub navigation: bool,
    pub report: bool,
    pub search: bool,
    pub info: bool,
}

impl Default for TopToolbarVisibility {
    fn default() -> Self {
        Self {
            project: true,
            clipboard_undo: true,
            calculations: true,
            sim_calculation: true,
            dynamic_simulation: true,
            dynamic_progress: true,
            font: true,
            text_formatting: true,
            layout: true,
            visualization: true,
            dynamic_collect: true,
            navigation: true,
            report: true,
            search: true,
            info: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LeftToolbarVisibility {
    pub select: bool,
    pub units: bool,
    pub streams: bool,
    pub text: bool,
    pub picture: bool,
    pub table: bool,
    pub shape: bool,
    pub size: bool,
    pub rotate_flip: bool,
    pub depth: bool,
    pub alignment: bool,
    pub lock: bool,
}

impl Default for LeftToolbarVisibility {
    fn default() -> Self {
        Self {
            select: true,
            units: true,
            streams: true,
            text: true,
            picture: true,
            table: true,
            shape: true,
            size: true,
            rotate_flip: true,
            depth: true,
            alignment: true,
            lock: true,
        }
    }
}

pub struct SimModuleState {
    pub project: SimProject,
    pub active_preset_name: String,
    pub undo_stack: Vec<SimProject>,
    pub redo_stack: Vec<SimProject>,
    pub clipboard_unit: Option<SimUnit>,

    // Navigation & Canvas View
    pub zoom_pct: f32,
    pub pan_mm: [f32; 2],
    pub cursor_mm: [f32; 2],
    pub active_tool: ActiveTool,
    pub pending_stream_start_mm: Option<[f32; 2]>,
    pub pending_stream_start_unit: Option<usize>,

    // Selection & Inline Editing
    pub selected: SelectedObject,
    pub multi_selected_units: Vec<usize>,
    pub multi_selected_streams: Vec<usize>,
    pub inline_rename_target: Option<SelectedObject>,
    pub inline_rename_buf: String,
    pub dragging_obj: bool,
    pub dragging_stream_handle: Option<(usize, bool)>, // (stream_id, is_dest_handle)

    // Toolbars & Docking Panes Visibility (View menu)
    pub show_top_toolbars: bool,
    pub show_left_toolbars: bool,
    pub show_right_top_dock: bool,
    pub show_right_bottom_dock: bool,
    pub show_rulers: bool,
    pub top_tb: TopToolbarVisibility,
    pub left_tb: LeftToolbarVisibility,

    pub right_top_tab: RightTopDockTab,
    pub prop_subtab_idle: usize,      // 0:Process, 1:KPI, 2:Page, 3:Arrowheads
    pub prop_subtab_selected: usize,  // 0:Process, 1:Drawings
    pub inspector_subtab: usize,      // 0:Phases, 1:Hydrometallurgy, 2:Stream Properties

    pub right_bottom_tab: RightBottomTab,
    pub icons_filter_category: String,
    pub icons_search: String,
    pub icons_thumb_size: f32,
    pub items_search: String,
    pub names_search: String,
    pub visibility_subtab: usize, // 0:Layers, 1:Groups, 2:Pages
    pub new_layer_name: String,
    pub new_group_name: String,
    pub new_page_name: String,
    pub controls_search: String,
    pub cell_ref_search: String,
    pub log_filter_level: LogFilterLevel,
    pub log_search_query: String,

    // Top bar transient UI fields
    pub calc_rounds_input: usize,
    pub selected_font_name: String,
    pub toolbar_search_text: String,
    pub dynamic_running: bool,
    pub highlight_calc_order_on_canvas: bool,
    pub status_banner: String,

    // Secondary Modals & Popups
    pub show_streams_src_dest: bool,
    pub src_dest_stream_id: Option<usize>,
    pub src_dest_draft_src: Option<usize>,
    pub src_dest_draft_dst: Option<usize>,

    pub show_default_settings: bool,
    pub default_settings_tab: usize,

    pub show_table_editor: bool,
    pub editing_table_id: Option<usize>,

    pub show_stream_table_editor: bool,
    pub stream_table_var_sel: Option<usize>,
    pub stream_table_canvas_sel: Option<usize>,

    pub show_select_unit_models: bool,
    pub unit_model_tab: ModelLibraryTab,
    pub selected_catalog_idx: usize,
    pub import_model_name: String,
    pub import_model_mode: String,

    pub show_unit_editor: bool,
    pub editing_unit_id: Option<usize>,
    pub unit_editor_tab: usize, // 0:Input, 1:Output, 2:Dist, 3:Controls, 4:Model

    pub show_control_monitor: bool,
    pub monitor_ctrl_idx: usize,
    pub monitor_sweep_points: usize,
    pub monitor_sweep_cache: Vec<(usize, f64, f64)>,
    pub monitor_selected_row: Option<usize>,
    pub monitor_plot_tolerance: bool,

    pub show_scenario_editor: bool,
    pub scenario_tab: usize, // 0:Charts, 1:Monte Carlo
    pub scenario_selected_col: usize,
    pub scenario_selected_run: usize,
    pub scenario_formula_bar: String,

    pub show_reports_window: bool,
    pub report_sheet_idx: usize,
    pub reports_cache: Option<SimReportWorkbook>,

    pub show_process_info: bool,
    pub process_info_tab: usize,

    pub show_notes_window: bool,
    pub notes_tab: usize,

    pub show_convergence_monitor: bool,
    pub show_calc_order_window: bool,
    pub show_visualization_picker: bool,

    pub show_sim7_migrator: bool,
    pub sim7_expr_input: String,
    pub sim7_sheet_input: String,

    pub show_export_modal: bool,
    pub export_kind: ExportModalKind,
    pub export_buffer: String,

    pub show_lca_window: bool,
    pub lca_state: crate::lca_ui::LcaWindowState,

    pub show_mb_window: bool,
    pub mb_state: crate::mass_balance_ui::MassBalanceWindowState,

    pub show_about_modal: bool,
    pub show_updates_modal: bool,
}

impl Default for SimModuleState {
    fn default() -> Self {
        let project = empty_project();
        Self {
            project,
            active_preset_name: "Empty Flowsheet (New)".to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard_unit: None,
            zoom_pct: 100.0,
            pan_mm: [-12.0, -4.0],
            cursor_mm: [83.0, 48.0],
            active_tool: ActiveTool::Select,
            pending_stream_start_mm: None,
            pending_stream_start_unit: None,
            selected: SelectedObject::None,
            multi_selected_units: Vec::new(),
            multi_selected_streams: Vec::new(),
            inline_rename_target: None,
            inline_rename_buf: String::new(),
            dragging_obj: false,
            dragging_stream_handle: None,
            show_top_toolbars: true,
            show_left_toolbars: true,
            show_right_top_dock: true,
            show_right_bottom_dock: true,
            show_rulers: true,
            top_tb: TopToolbarVisibility::default(),
            left_tb: LeftToolbarVisibility::default(),
            right_top_tab: RightTopDockTab::Properties,
            prop_subtab_idle: 0,
            prop_subtab_selected: 0,
            inspector_subtab: 0,
            right_bottom_tab: RightBottomTab::Icons,
            icons_filter_category: "Units by Type".to_string(),
            icons_search: String::new(),
            icons_thumb_size: 46.0,
            items_search: String::new(),
            names_search: String::new(),
            visibility_subtab: 0,
            new_layer_name: "Custom Layer".to_string(),
            new_group_name: "Section 1".to_string(),
            new_page_name: "Page4".to_string(),
            controls_search: String::new(),
            cell_ref_search: String::new(),
            log_filter_level: LogFilterLevel::All,
            log_search_query: String::new(),
            calc_rounds_input: 50,
            selected_font_name: "Arial".to_string(),
            toolbar_search_text: String::new(),
            dynamic_running: false,
            highlight_calc_order_on_canvas: false,
            status_banner: "Ready — Empty HSC Sim Workspace (Select preset from File -> Load Demo Project)".to_string(),
            show_streams_src_dest: false,
            src_dest_stream_id: None,
            src_dest_draft_src: None,
            src_dest_draft_dst: None,
            show_default_settings: false,
            default_settings_tab: 0,
            show_table_editor: false,
            editing_table_id: None,
            show_stream_table_editor: false,
            stream_table_var_sel: None,
            stream_table_canvas_sel: None,
            show_select_unit_models: false,
            unit_model_tab: ModelLibraryTab::Species,
            selected_catalog_idx: 0,
            import_model_name: "Custom Hydro Reactor".to_string(),
            import_model_mode: "Reactions (Hydro)".to_string(),
            show_unit_editor: false,
            editing_unit_id: None,
            unit_editor_tab: 0,
            show_control_monitor: false,
            monitor_ctrl_idx: 0,
            monitor_sweep_points: 50,
            monitor_sweep_cache: Vec::new(),
            monitor_selected_row: None,
            monitor_plot_tolerance: true,
            show_scenario_editor: false,
            scenario_tab: 0,
            scenario_selected_col: 0,
            scenario_selected_run: 0,
            scenario_formula_bar: "185".to_string(),
            show_reports_window: false,
            report_sheet_idx: 0,
            reports_cache: None,
            show_process_info: false,
            process_info_tab: 0,
            show_notes_window: false,
            notes_tab: 0,
            show_convergence_monitor: false,
            show_calc_order_window: false,
            show_visualization_picker: false,
            show_sim7_migrator: false,
            sim7_expr_input: "=[Book1.xls]Feed!C5 / Feed!B1".to_string(),
            sim7_sheet_input: "Leaching/Feed".to_string(),
            show_export_modal: false,
            export_kind: ExportModalKind::ProjectJson,
            export_buffer: String::new(),
            show_lca_window: false,
            lca_state: crate::lca_ui::LcaWindowState::default(),
            show_mb_window: false,
            mb_state: crate::mass_balance_ui::MassBalanceWindowState::default(),
            show_about_modal: false,
            show_updates_modal: false,
        }
    }
}

impl SimModuleState {
    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.project.clone());
        if self.undo_stack.len() > 30 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.project.clone());
            self.project = prev;
            self.status_banner = "Undo applied".to_string();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.project.clone());
            self.project = next;
            self.status_banner = "Redo applied".to_string();
        }
    }

    pub fn load_preset(&mut self, name: &str, project: SimProject) {
        self.push_undo();
        self.calc_rounds_input = project.calc_settings.max_rounds;
        self.project = project;
        self.active_preset_name = name.to_string();
        self.selected = SelectedObject::None;
        self.multi_selected_units.clear();
        self.multi_selected_streams.clear();
        self.pan_mm = [-14.0, -4.0];
        self.zoom_pct = 100.0;
        self.reports_cache = Some(SoryntelApplication::sim_generate_reports(&self.project));
        self.lca_state.sync_from_project(&self.project);
        self.mb_state.sync_from_project(&self.project);
        self.status_banner = format!(
            "Loaded project '{}' ({} units, {} streams)",
            name,
            self.project.units.len(),
            self.project.streams.len()
        );
    }

    pub fn run_static(&mut self, only_selected: bool) {
        if self.project.lock_state == LockState::EntireProcessLocked {
            self.status_banner = "Blocked: Entire Process is Locked".to_string();
            return;
        }
        self.push_undo();
        self.project.calc_settings.max_rounds = self.calc_rounds_input.max(1);
        let sel_ids = if only_selected {
            let mut ids = self.multi_selected_units.clone();
            if let SelectedObject::Unit(id) = self.selected {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            if ids.is_empty() { None } else { Some(ids) }
        } else {
            None
        };
        SoryntelApplication::sim_run_static(&mut self.project, sel_ids.as_deref());
        self.reports_cache = Some(SoryntelApplication::sim_generate_reports(&self.project));
        self.lca_state.sync_from_project(&self.project);
        if let Some(last) = self.project.convergence_history.last() {
            self.status_banner = format!(
                "Calculation round {}: MB={:.3}%, EB={:.3}% ({})",
                last.round,
                last.mb_pct,
                last.eb_pct,
                if last.converged { "Converged" } else { "Stopped at max rounds" }
            );
        }
    }

    pub fn run_controls(&mut self, idx: Option<usize>) {
        if self.project.lock_state == LockState::EntireProcessLocked {
            self.status_banner = "Blocked: Entire Process is Locked".to_string();
            return;
        }
        self.push_undo();
        SoryntelApplication::sim_run_controls(&mut self.project, idx);
        SoryntelApplication::sim_run_static(&mut self.project, None);
        self.reports_cache = Some(SoryntelApplication::sim_generate_reports(&self.project));
        self.lca_state.sync_from_project(&self.project);
        self.status_banner = match idx {
            Some(i) => format!("Executed Control loop #{}", i + 1),
            None => "Executed all active Control loops".to_string(),
        };
    }

    pub fn open_stream_src_dest(&mut self, stream_id: usize) {
        if let Some(s) = self.project.streams.iter().find(|s| s.id == stream_id) {
            self.src_dest_stream_id = Some(stream_id);
            self.src_dest_draft_src = s.source_unit;
            self.src_dest_draft_dst = s.dest_unit;
            self.show_streams_src_dest = true;
        }
    }

    pub fn open_unit_editor(&mut self, unit_id: usize) {
        self.editing_unit_id = Some(unit_id);
        self.unit_editor_tab = 0;
        self.show_unit_editor = true;
    }

    pub fn open_control_monitor(&mut self, ctrl_idx: usize) {
        if ctrl_idx < self.project.controls.len() {
            self.monitor_ctrl_idx = ctrl_idx;
            self.monitor_sweep_cache = SoryntelApplication::sim_control_sweep(
                &self.project,
                ctrl_idx,
                self.monitor_sweep_points.max(10),
            );
            self.show_control_monitor = true;
        }
    }

    pub fn delete_selected(&mut self) {
        if self.project.lock_state != LockState::Unlocked {
            self.status_banner = "Cannot delete objects while Drawing or Process is Locked".to_string();
            return;
        }
        self.push_undo();
        match self.selected {
            SelectedObject::Unit(uid) => {
                self.project.units.retain(|u| u.id != uid);
                for s in &mut self.project.streams {
                    if s.source_unit == Some(uid) {
                        s.source_unit = None;
                    }
                    if s.dest_unit == Some(uid) {
                        s.dest_unit = None;
                    }
                }
                self.status_banner = format!("Deleted unit #{uid}");
            }
            SelectedObject::Stream(sid) => {
                self.project.streams.retain(|s| s.id != sid);
                self.status_banner = format!("Deleted stream #{sid}");
            }
            SelectedObject::Table(tid) => {
                self.project.tables.retain(|t| t.id != tid);
                self.status_banner = format!("Deleted table #{tid}");
            }
            SelectedObject::Annotation(aid) => {
                self.project.annotations.retain(|a| a.id != aid);
                self.status_banner = format!("Deleted annotation #{aid}");
            }
            SelectedObject::None => {}
        }
        self.selected = SelectedObject::None;
    }

    pub fn alloc_id(&mut self) -> usize {
        let id = self.project.next_id;
        self.project.next_id += 1;
        id
    }
}
