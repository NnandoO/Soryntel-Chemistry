//! Clean-room Desktop UI for Module 20: `HSC - Mass Balance` (`Mas`, Chapter 51).
//!
//! Clones the exact 4-zone interface of HSC Sim Mass Balance (`Figs. 1–28` + `51.5` ribbon):
//! 1. `Upper Buttons` — permanent `Main Menu` strip + context-sensitive tool groups.
//! 2. `Balancing Navigator` — collapsible left accordion tree with Salmon active selection.
//! 3. `Working Area` — spreadsheets with semantic color coding, Data Status pie charts,
//!    Review SD variance plots, `Balance Convergence` + `Parity Chart`, and `Reporting -> Charts`.
//! 4. `Properties` — right inspector for unit/stream/variable/selected-cell & algorithm summary.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, Points};
use serde::{Deserialize, Serialize};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::{
    mass_balance::{
        default_mineral_sg, evaluate_cell_uncertainty, search_minerals_catalog, BalanceType,
        CellKey, ChartPanelSpec, CheckSeverity, MassBalanceWorkbook, MbSolution, MbStream,
        MbVariable, MeasurementMethod, PostBalStatus, PreMeasStatus, SampleQuality, SizingMode,
        SolveMethod, StreamEquation, StreamType, ValidationIssue, VarTypeCode,
    },
    model::SimProject,
};

// Palette matching HSC Mass Balance official screenshots
const RIBBON_BG: Color32 = Color32::from_rgb(241, 244, 249);
const RIBBON_GROUP_BG: Color32 = Color32::from_rgb(248, 250, 252);
const RIBBON_BORDER: Color32 = Color32::from_rgb(198, 208, 220);
const NAV_BG: Color32 = Color32::from_rgb(246, 248, 251);
const NAV_SALMON_ACTIVE: Color32 = Color32::from_rgb(245, 168, 130); // Official HSC salmon highlight
const HDR_GRAY_BG: Color32 = Color32::from_rgb(232, 237, 244);
const ALT_ROW_BG: Color32 = Color32::from_rgb(249, 251, 253);
const GRID_LINE: Color32 = Color32::from_rgb(203, 213, 225);

const BLUE_TEXT: Color32 = Color32::from_rgb(29, 78, 216); // Estimated / Balanced / Auto-SD / ±CI
const GRAY_EXCLUDED: Color32 = Color32::from_rgb(148, 163, 184); // Excluded assays
const GREEN_EQ_BG: Color32 = Color32::from_rgb(187, 247, 208); // Stream by Equation green cell
const YELLOW_WARN_BG: Color32 = Color32::from_rgb(254, 240, 138); // Medium discrepancy heat
const RED_WARN_BG: Color32 = Color32::from_rgb(254, 202, 202); // Large discrepancy heat
const BAR_FILL: Color32 = Color32::from_rgb(96, 165, 250); // Recovery % Data Bars
const DARK_INK: Color32 = Color32::from_rgb(15, 23, 42);

const CHART_PALETTE: &[Color32] = &[
    Color32::from_rgb(59, 130, 246),
    Color32::from_rgb(234, 88, 12),
    Color32::from_rgb(22, 163, 74),
    Color32::from_rgb(168, 85, 247),
    Color32::from_rgb(225, 29, 72),
    Color32::from_rgb(13, 148, 136),
    Color32::from_rgb(202, 138, 4),
    Color32::from_rgb(71, 85, 105),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MbNavPage {
    Units,
    Streams,
    Variables,
    SizeFractions,
    DataSets,
    MeasurementData,
    DataStatus,
    ReviewSd,
    EditSdTable,
    Balance,
    ReportingResults,
    ReportingCharts,
}

impl MbNavPage {
    fn title(self) -> &'static str {
        match self {
            Self::Units => "Experimental Data > 1. Units",
            Self::Streams => "Experimental Data > 2. Streams",
            Self::Variables => "Experimental Data > 3. Variables",
            Self::SizeFractions => "Experimental Data > 4. Size Fractions",
            Self::DataSets => "Experimental Data > 5. Data Sets",
            Self::MeasurementData => "Experimental Data > 6. Measurement Data",
            Self::DataStatus => "Review Data > 1. Data Status",
            Self::ReviewSd => "Standard Deviations > 1. Review SD",
            Self::EditSdTable => "Standard Deviations > 2. Edit SD Table",
            Self::Balance => "Balancing > 1. Balance",
            Self::ReportingResults => "Reporting > 1. Results",
            Self::ReportingCharts => "Reporting > 2. Charts",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BalanceTab {
    Data,
    Details,
    Goodness,
    UnitBalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsTab {
    StreamSummary,
    SizeDistribution,
    PhysicalProperties,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextModalKind {
    PasteExperimental,
    PasteSdRsd,
    ImportRex,
    BackupJson,
    ExportPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecoveryDraftRow {
    stream_id: usize,
    stream_name: String,
    var_id: usize,
    var_name: String,
    rec_pct: f64,
}

pub struct MassBalanceWindowState {
    pub workbook: MassBalanceWorkbook,
    pub preset_label: String,
    active_page: MbNavPage,
    active_frac_idx: usize,
    active_round_view: Option<usize>, // None = Master Data, Some(r) = Sampling Round r+1

    // Layout & chrome state
    nav_open: bool,
    props_open: bool,
    nav_exp_open: bool,
    nav_rev_open: bool,
    nav_sd_open: bool,
    nav_bal_open: bool,
    nav_rep_open: bool,
    prop_sel_data_open: bool,
    prop_sel_vals_open: bool,
    prop_sel_bal_open: bool,
    prop_calc_settings_open: bool,
    prop_algo_params_open: bool,
    prop_summary_open: bool,

    // Selections
    selected_unit_id: Option<usize>,
    selected_stream_id: Option<usize>,
    selected_var_id: Option<usize>,
    selected_cell: Option<CellKey>,
    meas_table_locked: bool,
    sd_table_locked: bool,
    show_sg_column: bool,

    // View controls
    status_paint_tool: PreMeasStatus,
    balance_tab: BalanceTab,
    results_tab: ResultsTab,
    parity_log_scale: bool,
    results_filter_mode: usize, // 0=All Selected, 1=Feeds & Products Only
    show_chart_props_panel: bool,
    status_banner: String,

    // Inline editors / draft inputs
    new_sf_name: String,
    new_ds_name: String,
    new_var_name: String,
    new_var_unit: String,
    new_var_type: VarTypeCode,
    mineral_search_query: String,

    // Modals & Popovers
    show_low_grade_popover: bool,
    show_equations_modal: bool,
    eq_only_non_sru: bool,
    show_error_check_modal: bool,
    error_check_cache: Vec<ValidationIssue>,
    show_import_hsc7_modal: bool,
    import_hsc7_flowsheet_buf: String,
    import_hsc7_data_buf: String,
    import_hsc7_horizontal: bool,
    show_stream_eq_modal: bool,
    eq_target_stream_idx: usize,
    eq_source_stream_idx: usize,
    eq_factor_input: f64,
    show_mineral_matrix_modal: bool,
    show_element_picker_modal: bool,
    show_recoveries_modal: bool,
    recovery_drafts: Vec<RecoveryDraftRow>,
    show_text_modal: Option<TextModalKind>,
    text_modal_buf: String,
    text_modal_title: String,
    show_help_modal: bool,
}

impl Default for MassBalanceWindowState {
    fn default() -> Self {
        Self::with_preset_rougher().1
    }
}

impl MassBalanceWindowState {
    pub fn with_preset_rougher() -> (SimProject, Self) {
        let mut wb = SoryntelApplication::mb_preset_rougher();
        let sol = SoryntelApplication::mb_calculate(&mut wb, 0).ok();
        let banner = match &sol {
            Some(s) => format!(
                "Rougher Flotation preset ready — {} rounds, WSSQ={:.4}, RMSD={:.4}",
                s.convergence_rounds.len(),
                s.wssq,
                s.rmsd
            ),
            None => "Rougher Flotation preset loaded".to_string(),
        };
        let proj = soryntel_thermo::sim::presets::empty_project();
        let st = Self::from_workbook(wb, "Rougher Flotation (Figs. 3, 9–23)", banner);
        (proj, st)
    }

    pub fn with_preset_grinding() -> (SimProject, Self) {
        let mut wb = SoryntelApplication::mb_preset_grinding();
        let sol = SoryntelApplication::mb_calculate(&mut wb, 0).ok();
        let banner = match &sol {
            Some(s) => format!(
                "Grinding Survey preset ready — {} size fractions, WSSQ={:.4}",
                wb.size_fractions.len(),
                s.wssq
            ),
            None => "Grinding Survey preset loaded".to_string(),
        };
        let proj = soryntel_thermo::sim::presets::empty_project();
        let st = Self::from_workbook(wb, "3 Grinding Survey (Figs. 24–25)", banner);
        (proj, st)
    }

    fn from_workbook(wb: MassBalanceWorkbook, label: &str, banner: String) -> Self {
        let selected_unit_id = wb.units.first().map(|u| u.id);
        let selected_stream_id = wb.streams.first().map(|s| s.id);
        let selected_var_id = wb.variables.first().map(|v| v.id);
        let selected_cell = match (selected_stream_id, selected_var_id) {
            (Some(stream_id), Some(var_id)) => Some(CellKey {
                frac_idx: 0,
                stream_id,
                var_id,
            }),
            _ => None,
        };
        Self {
            workbook: wb,
            preset_label: label.to_string(),
            active_page: MbNavPage::Balance,
            active_frac_idx: 0,
            active_round_view: None,
            nav_open: true,
            props_open: true,
            nav_exp_open: true,
            nav_rev_open: true,
            nav_sd_open: true,
            nav_bal_open: true,
            nav_rep_open: true,
            prop_sel_data_open: true,
            prop_sel_vals_open: true,
            prop_sel_bal_open: true,
            prop_calc_settings_open: true,
            prop_algo_params_open: false,
            prop_summary_open: true,
            selected_unit_id,
            selected_stream_id,
            selected_var_id,
            selected_cell,
            meas_table_locked: false,
            sd_table_locked: false,
            show_sg_column: false,
            status_paint_tool: PreMeasStatus::Measured,
            balance_tab: BalanceTab::Data,
            results_tab: ResultsTab::StreamSummary,
            parity_log_scale: false,
            results_filter_mode: 0,
            show_chart_props_panel: false,
            status_banner: banner,
            new_sf_name: "75-150um".to_string(),
            new_ds_name: "Survey 2".to_string(),
            new_var_name: "Pb".to_string(),
            new_var_unit: "%".to_string(),
            new_var_type: VarTypeCode::A,
            mineral_search_query: String::new(),
            show_low_grade_popover: false,
            show_equations_modal: false,
            eq_only_non_sru: false,
            show_error_check_modal: false,
            error_check_cache: Vec::new(),
            show_import_hsc7_modal: false,
            import_hsc7_flowsheet_buf: "Units\tROM\tSAG Discharge\tCyclone UF\tCyclone OF\nSAG\t1\t-1\t1\t0\nCyclone\t0\t1\t-1\t-1".to_string(),
            import_hsc7_data_buf: "Stream->\tROM\tSAG Discharge\tCyclone UF\tCyclone OF\nSolids Flowrate t/h\t517.5\t1810\t1290\t517.5\nCu %\t1.2\t1.22\t1.3\t1.96".to_string(),
            import_hsc7_horizontal: false,
            show_stream_eq_modal: false,
            eq_target_stream_idx: 0,
            eq_source_stream_idx: 0,
            eq_factor_input: 1.0,
            show_mineral_matrix_modal: false,
            show_element_picker_modal: false,
            show_recoveries_modal: false,
            recovery_drafts: Vec::new(),
            show_text_modal: None,
            text_modal_buf: String::new(),
            text_modal_title: String::new(),
            show_help_modal: false,
        }
    }

    /// Build or refresh Mass Balance topology & streams from active HSC Sim `SimProject`.
    pub fn sync_from_project(&mut self, project: &SimProject) {
        if project.units.is_empty() && project.streams.is_empty() {
            return;
        }
        let mut wb = SoryntelApplication::mb_from_sim_project(project);
        let _ = SoryntelApplication::mb_calculate(&mut wb, 0);
        self.workbook = wb;
        self.preset_label = format!("Flowsheet: {}", project.process_info.process_name);
        self.active_frac_idx = 0;
        self.active_round_view = None;
        self.selected_unit_id = self.workbook.units.first().map(|u| u.id);
        self.selected_stream_id = self.workbook.streams.first().map(|s| s.id);
        self.selected_var_id = self.workbook.variables.first().map(|v| v.id);
        self.status_banner = format!(
            "Synced {} units & {} streams from Sim Flowsheet",
            self.workbook.units.len(),
            self.workbook.streams.len()
        );
    }

    fn run_calculate_active(&mut self) {
        let ds_idx = self.workbook.active_dataset;
        match SoryntelApplication::mb_calculate(&mut self.workbook, ds_idx) {
            Ok(sol) => {
                self.status_banner = format!(
                    "Balanced '{}': {} rounds (conv={}), WSSQ={:.4}, RMSD={:.4}, Rel.RMSD={:.4}%",
                    self.workbook.datasets[ds_idx].name,
                    sol.convergence_rounds.len(),
                    sol.converged,
                    sol.wssq,
                    sol.rmsd,
                    sol.rel_rmsd * 100.0
                );
            }
            Err(err) => {
                self.status_banner = format!("Balance error: {err}");
            }
        }
    }

    fn run_error_check(&mut self) {
        let ds_idx = self.workbook.active_dataset;
        self.error_check_cache = SoryntelApplication::mb_check_errors(&self.workbook, ds_idx);
        self.show_error_check_modal = true;
        self.status_banner = if self.error_check_cache.is_empty() {
            "Check for Errors: Everything seems to be OK!".to_string()
        } else {
            format!(
                "Check for Errors: {} issue(s) reported",
                self.error_check_cache.len()
            )
        };
    }

    fn open_text_modal(&mut self, kind: TextModalKind, title: &str, content: String) {
        self.show_text_modal = Some(kind);
        self.text_modal_title = title.to_string();
        self.text_modal_buf = content;
    }

    fn export_xlsx_to_tmp(&mut self) {
        match SoryntelApplication::mb_export_xlsx(&self.workbook) {
            Ok(bytes) => {
                let path = "/tmp/soryntel_mass_balance_results.xlsx";
                match crate::storage::write_atomic(path, &bytes) {
                    Ok(_) => {
                        self.status_banner = format!(
                            "Exported {} bytes multi-sheet .xlsx to {path}",
                            bytes.len()
                        );
                    }
                    Err(e) => {
                        self.status_banner = format!("Failed to write {path}: {e}");
                    }
                }
            }
            Err(e) => {
                self.status_banner = format!("Excel export error: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public Window / Standalone Entry Points
// ---------------------------------------------------------------------------

pub fn render_standalone_mb_module(
    ctx: &egui::Context,
    project: &mut SimProject,
    state: &mut MassBalanceWindowState,
) {
    egui::TopBottomPanel::top("standalone_mb_top_strip").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Soryntel · Module 20: HSC - Mass Balance (Mas)")
                    .strong()
                    .color(DARK_INK),
            );
            ui.separator();
            ui.label("Preset:");
            if ui.button("Rougher Flotation (Figs. 3–23)").clicked() {
                let (p, next) = MassBalanceWindowState::with_preset_rougher();
                *project = p;
                *state = next;
            }
            if ui.button("Grinding Survey (Figs. 24–25)").clicked() {
                let (p, next) = MassBalanceWindowState::with_preset_grinding();
                *project = p;
                *state = next;
            }
            ui.separator();
            ui.label(RichText::new(&state.preset_label).small().color(BLUE_TEXT));
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut dummy_open = true;
        render_mb_body(ui, ctx, Some(project), &mut dummy_open, state, false);
    });
}

pub fn render_mb_floating_window(
    ctx: &egui::Context,
    open: &mut bool,
    project: &mut SimProject,
    state: &mut MassBalanceWindowState,
) {
    egui::Window::new("Mass Balancing")
        .id(egui::Id::new("hsc_sim_mass_balance_window"))
        .open(open)
        .default_size([1280.0, 760.0])
        .min_size([920.0, 560.0])
        .resizable(true)
        .show(ctx, |ui| {
            // Note: `open` handled inside via Discard & Close button below
            let mut keep_open = true;
            render_mb_body(ui, ctx, Some(project), &mut keep_open, state, true);
            if !keep_open {
                ui.ctx().memory_mut(|m| {
                    m.data.insert_temp(egui::Id::new("mb_close_req"), true);
                });
            }
        });
    let close_req = ctx.memory_mut(|m| {
        let v = m
            .data
            .get_temp::<bool>(egui::Id::new("mb_close_req"))
            .unwrap_or(false);
        if v {
            m.data.insert_temp(egui::Id::new("mb_close_req"), false);
        }
        v
    });
    if close_req {
        *open = false;
    }
}

fn render_mb_body(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    mut project: Option<&mut SimProject>,
    keep_open: &mut bool,
    state: &mut MassBalanceWindowState,
    is_floating: bool,
) {
    // Zone 1: Upper Buttons contextual ribbon
    render_upper_ribbon(ui, &mut project, keep_open, state, is_floating);
    ui.add_space(3.0);

    // Remaining vertical height for the 3-pane workspace + status strip
    let avail = ui.available_size();
    let body_h = (avail.y - 26.0).max(260.0);

    ui.allocate_ui_with_layout(
        Vec2::new(avail.x, body_h),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            // Zone 2: Left Balancing Navigator
            render_left_navigator(ui, state, body_h);
            ui.add_space(3.0);

            // Calculate remaining width after optional Right Properties panel
            let props_w = if state.props_open { 278.0 } else { 26.0 };
            let center_w = (ui.available_width() - props_w - 6.0).max(300.0);

            // Zone 3: Central Working Area
            egui::Frame::none()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, RIBBON_BORDER))
                .rounding(2.0)
                .show(ui, |ui| {
                    ui.set_width(center_w);
                    ui.set_height(body_h);
                    render_working_area(ui, state, body_h);
                });

            ui.add_space(3.0);

            // Zone 4: Right Property Panel
            render_properties_panel(ui, state, body_h);
        },
    );

    // Bottom status bar
    egui::Frame::none()
        .fill(RIBBON_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(crate::i18n::text("common.101")).strong().size(11.0));
                ui.label(RichText::new(&state.status_banner).size(11.0).color(DARK_INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ds) = state.workbook.active_dataset() {
                        ui.label(
                            RichText::new(format!(
                                "Dataset: {} | Method: {} | Sizing: {} | Mode: {}",
                                ds.name,
                                ds.method.label(),
                                ds.sizing.label(),
                                ds.balance_type.label()
                            ))
                            .size(10.5)
                            .color(Color32::from_rgb(71, 85, 105)),
                        );
                    }
                });
            });
        });

    render_mb_modals(ctx, state);
}

// ---------------------------------------------------------------------------
// Zone 1: Upper Buttons Contextual Ribbon
// ---------------------------------------------------------------------------

fn ribbon_group(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(RIBBON_GROUP_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| add_contents(ui));
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(label)
                            .size(9.5)
                            .color(Color32::from_rgb(100, 116, 139)),
                    );
                });
            });
        });
}

fn rbtn(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let fill = if active {
        Color32::from_rgb(191, 219, 254)
    } else {
        Color32::WHITE
    };
    let stroke = if active {
        Stroke::new(1.0, Color32::from_rgb(59, 130, 246))
    } else {
        Stroke::new(1.0, RIBBON_BORDER)
    };
    ui.add(
        egui::Button::new(RichText::new(text).size(10.5).color(DARK_INK))
            .fill(fill)
            .stroke(stroke)
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

fn render_upper_ribbon(
    ui: &mut egui::Ui,
    project: &mut Option<&mut SimProject>,
    keep_open: &mut bool,
    state: &mut MassBalanceWindowState,
    is_floating: bool,
) {
    egui::Frame::none()
        .fill(RIBBON_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .id_salt("mb_upper_ribbon_scroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Permanent Group 1: Main Menu
                        ribbon_group(ui, "Main Menu", |ui| {
                            ui.menu_button(RichText::new("Save to\nFlowsheet ▾").size(10.5), |ui| {
                                if ui.button("Save Balanced Flows, Grades & Recoveries").clicked() {
                                    if let Some(proj) = project.as_deref_mut() {
                                        let n = SoryntelApplication::mb_save_to_sim_project(
                                            &state.workbook,
                                            proj,
                                        );
                                        state.status_banner = format!(
                                            "Saved mass balance to Flowsheet ({n} streams updated; Mass Balance = YES)"
                                        );
                                    } else {
                                        state.status_banner =
                                            "No parent Flowsheet attached (use inside Sim -> Tools or export .HSCMas)".into();
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Save Recoveries / Properties Info Only").clicked() {
                                    if let Some(proj) = project.as_deref_mut() {
                                        let n = SoryntelApplication::mb_save_to_sim_project(
                                            &state.workbook,
                                            proj,
                                        );
                                        state.status_banner = format!(
                                            "Synced {n} stream recoveries to Flowsheet"
                                        );
                                    }
                                    ui.close_menu();
                                }
                            });

                            let close_label = if is_floating {
                                "Discard &\nClose"
                            } else {
                                "Reload\nPreset"
                            };
                            if rbtn(ui, close_label, false).clicked() {
                                if is_floating {
                                    *keep_open = false;
                                } else {
                                    let (_, fresh) = MassBalanceWindowState::with_preset_rougher();
                                    *state = fresh;
                                }
                            }

                            ui.menu_button(RichText::new("Backup ▾").size(10.5), |ui| {
                                if ui.button("Export Backup (.HSCMas JSON)").clicked() {
                                    let json = serde_json::to_string_pretty(&state.workbook)
                                        .unwrap_or_else(|e| format!("Serialize error: {e}"));
                                    state.open_text_modal(
                                        TextModalKind::BackupJson,
                                        "Mass Balance Backup (.HSCMas JSON)",
                                        json,
                                    );
                                    ui.close_menu();
                                }
                                if ui.button("Restore from .HSCMas JSON...").clicked() {
                                    state.open_text_modal(
                                        TextModalKind::BackupJson,
                                        "Restore .HSCMas JSON Backup",
                                        String::new(),
                                    );
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Load Benchmark: Rougher Flotation").clicked() {
                                    let (_, next) = MassBalanceWindowState::with_preset_rougher();
                                    *state = next;
                                    ui.close_menu();
                                }
                                if ui.button("Load Benchmark: Grinding Survey").clicked() {
                                    let (_, next) = MassBalanceWindowState::with_preset_grinding();
                                    *state = next;
                                    ui.close_menu();
                                }
                            });

                            if rbtn(ui, "Import\nHSC7 Data", false).clicked() {
                                state.show_import_hsc7_modal = true;
                            }
                            if rbtn(ui, "Import\nReX Data", false).clicked() {
                                state.open_text_modal(
                                    TextModalKind::ImportRex,
                                    "Import HSC ReX / Tab-Delimited Mineral Assay Data",
                                    "Stream\tCcp\tPy\tSp\tQtz\nFeed\t2.93\t73.50\t2.93\t20.66".into(),
                                );
                            }
                            if rbtn(ui, "Check for\nErrors", false)
                                .on_hover_text("Validate all 12 Chapter 51.6 rules")
                                .clicked()
                            {
                                state.run_error_check();
                            }
                            if rbtn(ui, "Help", false).clicked() {
                                state.show_help_modal = true;
                            }
                        });

                        ui.add_space(4.0);

                        // Context-sensitive groups depending on active_page
                        match state.active_page {
                            MbNavPage::Units => {
                                ribbon_group(ui, "Units", |ui| {
                                    if rbtn(ui, "Select\nAll", false).clicked() {
                                        for u in &mut state.workbook.units {
                                            u.selected = true;
                                        }
                                    }
                                    if rbtn(ui, "Unselect\nAll", false).clicked() {
                                        for u in &mut state.workbook.units {
                                            u.selected = false;
                                        }
                                    }
                                });
                                ribbon_group(ui, "Data Tools", |ui| {
                                    if rbtn(ui, "Balance\nEquations", false).clicked() {
                                        state.show_equations_modal = true;
                                    }
                                });
                            }
                            MbNavPage::Streams => {
                                ribbon_group(ui, "Streams", |ui| {
                                    if rbtn(ui, "Detect\nStream Types", false).clicked() {
                                        state.workbook.detect_stream_types();
                                        state.status_banner =
                                            "Auto-detected Solids/Slurry vs Liquid/Water stream types"
                                                .into();
                                    }
                                    if rbtn(ui, "Unselect Streams\nwith no Data", false).clicked() {
                                        state.workbook.unselect_streams_without_data();
                                        state.status_banner =
                                            "Unselected streams that have no measurements".into();
                                    }
                                    if rbtn(ui, "Select\nAll", false).clicked() {
                                        for s in &mut state.workbook.streams {
                                            s.selected = true;
                                        }
                                    }
                                    if rbtn(ui, "Stream by\nEquation...", false).clicked() {
                                        state.show_stream_eq_modal = true;
                                    }
                                });
                            }
                            MbNavPage::Variables => {
                                ribbon_group(ui, "Variables", |ui| {
                                    if rbtn(ui, "Add\nVariable", false).clicked() {
                                        let id = state.workbook.next_id;
                                        state.workbook.next_id += 1;
                                        state.workbook.variables.push(MbVariable {
                                            id,
                                            name: state.new_var_name.clone(),
                                            unit: state.new_var_unit.clone(),
                                            type_code: state.new_var_type,
                                            selected: true,
                                            method: MeasurementMethod::FixedRelative,
                                            param_a: 5.0,
                                            param_b: 0.0,
                                            param_c: 0.0,
                                            low_grade_weighted: false,
                                            mineral_formula: None,
                                            mineral_sg: None,
                                        });
                                        state.selected_var_id = Some(id);
                                    }
                                    let active_min = state.workbook.balancing_of_minerals;
                                    if rbtn(ui, "Balancing\nof Minerals", active_min)
                                        .on_hover_text(
                                            "If pressed down, solids elemental assays (A) are excluded & colored gray",
                                        )
                                        .clicked()
                                    {
                                        state.workbook.balancing_of_minerals = !active_min;
                                        state.status_banner = if state.workbook.balancing_of_minerals
                                        {
                                            "Balancing of Minerals ON: Elemental assays (A) excluded"
                                                .into()
                                        } else {
                                            "Balancing of Minerals OFF: Elemental assays active"
                                                .into()
                                        };
                                    }
                                    if rbtn(ui, "View Mineral\nMatrix", false).clicked() {
                                        state.show_mineral_matrix_modal = true;
                                    }
                                    if rbtn(ui, "Elements...", false).clicked() {
                                        state.show_element_picker_modal = true;
                                    }
                                });
                            }
                            MbNavPage::SizeFractions => {
                                ribbon_group(ui, "Size Fractions", |ui| {
                                    if rbtn(ui, "Add Size\nFraction", false).clicked() {
                                        let name = state.new_sf_name.trim().to_string();
                                        if !name.is_empty()
                                            && !state.workbook.size_fractions.contains(&name)
                                        {
                                            state.workbook.size_fractions.push(name);
                                        }
                                    }
                                    if rbtn(ui, "Remove Last\nSieve", false).clicked() {
                                        if state.workbook.size_fractions.len() > 1 {
                                            state.workbook.size_fractions.pop();
                                            state.active_frac_idx = state
                                                .active_frac_idx
                                                .min(state.workbook.size_fractions.len() - 1);
                                        }
                                    }
                                    if rbtn(ui, "Add Standard\n4-Sieve Series", false).clicked() {
                                        for s in ["-53um", "53-150um", "150-600um", "+600um"] {
                                            if !state
                                                .workbook
                                                .size_fractions
                                                .iter()
                                                .any(|x| x == s)
                                            {
                                                state.workbook.size_fractions.push(s.to_string());
                                            }
                                        }
                                    }
                                });
                            }
                            MbNavPage::DataSets => {
                                ribbon_group(ui, "Data Sets", |ui| {
                                    if rbtn(ui, "Add\nData Set", false).clicked() {
                                        let mut clone = state
                                            .workbook
                                            .active_dataset()
                                            .cloned()
                                            .unwrap_or_default();
                                        clone.name = if state.new_ds_name.trim().is_empty() {
                                            format!("Data Set {}", state.workbook.datasets.len() + 1)
                                        } else {
                                            state.new_ds_name.trim().to_string()
                                        };
                                        state.workbook.datasets.push(clone);
                                        state.workbook.solutions.push(None);
                                        state.workbook.active_dataset =
                                            state.workbook.datasets.len() - 1;
                                    }
                                    if rbtn(ui, "Select All\nfor Batch", false).clicked() {
                                        for d in &mut state.workbook.datasets {
                                            d.selected_for_batch = true;
                                        }
                                    }
                                    if rbtn(ui, "Batch\nRun", false).clicked() {
                                        let res = SoryntelApplication::mb_calculate_batch(
                                            &mut state.workbook,
                                        );
                                        let ok_cnt = res.iter().filter(|r| r.is_ok()).count();
                                        state.status_banner = format!(
                                            "Batch Run complete: {ok_cnt}/{} dataset(s) converged",
                                            res.len()
                                        );
                                    }
                                });
                            }
                            MbNavPage::MeasurementData => {
                                ribbon_group(ui, "Table", |ui| {
                                    ui.menu_button(RichText::new(crate::i18n::text("common.102")).size(10.5), |ui| {
                                        if ui.button("Copy Whole Table (TSV)").clicked() {
                                            let txt = build_measurement_tsv(
                                                &state.workbook,
                                                state.active_frac_idx,
                                            );
                                            ui.ctx().copy_text(txt.clone());
                                            state.open_text_modal(
                                                TextModalKind::ExportPreview,
                                                "Copied Measurement Table (TSV)",
                                                txt,
                                            );
                                            ui.close_menu();
                                        }
                                        if ui.button("Copy Empty Template").clicked() {
                                            let mut out = String::from("Stream");
                                            for v in
                                                state.workbook.variables.iter().filter(|v| v.selected)
                                            {
                                                out.push('\t');
                                                out.push_str(&v.name);
                                            }
                                            for s in
                                                state.workbook.streams.iter().filter(|s| s.selected)
                                            {
                                                out.push('\n');
                                                out.push_str(&s.name);
                                            }
                                            ui.ctx().copy_text(out.clone());
                                            state.open_text_modal(
                                                TextModalKind::ExportPreview,
                                                "Measurement Template (TSV)",
                                                out,
                                            );
                                            ui.close_menu();
                                        }
                                    });
                                    if rbtn(ui, "Paste\nExp. Data", false).clicked() {
                                        state.open_text_modal(
                                            TextModalKind::PasteExperimental,
                                            "Paste Experimental Data (Tab / Semicolon / CSV)",
                                            "Stream\tMass\tCu\tFe\nFeed\t\t1.013\t36.3\nRC1\t464\t23.6\t29.2"
                                                .into(),
                                        );
                                    }
                                    if rbtn(
                                        ui,
                                        if state.meas_table_locked {
                                            "Unlock\nTable"
                                        } else {
                                            "Lock\nTable"
                                        },
                                        state.meas_table_locked,
                                    )
                                    .clicked()
                                    {
                                        state.meas_table_locked = !state.meas_table_locked;
                                    }
                                });

                                ribbon_group(ui, "Data Tools", |ui| {
                                    if rbtn(ui, "Element to\nMineral...", false)
                                        .on_hover_text("Convert elemental assays (A) to minerals (M) via HSC Geo NNLS")
                                        .clicked()
                                    {
                                        match state.workbook.convert_elements_to_minerals(false) {
                                            Ok(n) => {
                                                state.status_banner = format!(
                                                    "Converted elemental assays to minerals ({n} cells written)"
                                                );
                                            }
                                            Err(e) => {
                                                state.status_banner =
                                                    format!("Element to Mineral: {e}");
                                            }
                                        }
                                    }
                                    if rbtn(ui, "View Mineral\nMatrix", false).clicked() {
                                        state.show_mineral_matrix_modal = true;
                                    }
                                    if rbtn(ui, "Sol-% to\nWater", false).clicked() {
                                        let n = state.workbook.convert_sol_pct_to_water();
                                        state.status_banner = format!(
                                            "Converted %Solids to Water flowrates on {n} stream(s)"
                                        );
                                    }
                                    if rbtn(ui, "Show\nSG", state.show_sg_column).clicked() {
                                        state.show_sg_column = !state.show_sg_column;
                                    }
                                    if rbtn(ui, "Estimate\nMissing Values", false).clicked() {
                                        let n = state.workbook.estimate_missing_values();
                                        state.status_banner = format!(
                                            "Estimated {n} missing cell(s) (shown in blue Guesstimated text)"
                                        );
                                    }
                                    if rbtn(ui, "Clear Est.\nValues", false).clicked() {
                                        state.workbook.clear_estimated_values();
                                        state.status_banner =
                                            "Cleared all estimated (Guesstimated) cells".into();
                                    }
                                    if rbtn(ui, "Enter\nRecoveries", false).clicked() {
                                        state.recovery_drafts.clear();
                                        let ds_idx = state.workbook.active_dataset;
                                        if let Some(ds) = state.workbook.datasets.get(ds_idx) {
                                            for s in
                                                state.workbook.streams.iter().filter(|s| s.selected)
                                            {
                                                for v in state
                                                    .workbook
                                                    .variables
                                                    .iter()
                                                    .filter(|v| v.selected && v.type_code.is_assay())
                                                {
                                                    let cur = ds
                                                        .recovered_for_model_fit
                                                        .get(&(s.id, v.id))
                                                        .copied()
                                                        .or_else(|| {
                                                            state
                                                                .workbook
                                                                .solutions
                                                                .get(ds_idx)
                                                                .and_then(|o| o.as_ref())
                                                                .and_then(|sol| {
                                                                    sol.cells
                                                                        .get(&CellKey {
                                                                            frac_idx: 0,
                                                                            stream_id: s.id,
                                                                            var_id: v.id,
                                                                        })
                                                                        .and_then(|c| c.recovery_pct)
                                                                })
                                                        })
                                                        .unwrap_or(0.0);
                                                    state.recovery_drafts.push(RecoveryDraftRow {
                                                        stream_id: s.id,
                                                        stream_name: s.name.clone(),
                                                        var_id: v.id,
                                                        var_name: v.name.clone(),
                                                        rec_pct: cur,
                                                    });
                                                }
                                            }
                                        }
                                        state.show_recoveries_modal = true;
                                    }
                                });

                                ribbon_group(ui, "Sampling Rounds", |ui| {
                                    if rbtn(ui, "Add\nRound", false).clicked() {
                                        let idx = state.workbook.active_dataset;
                                        if let Some(ds) = state.workbook.datasets.get_mut(idx) {
                                            let base = ds.values.clone();
                                            ds.rounds.push(base);
                                            state.active_round_view = Some(ds.rounds.len() - 1);
                                            state.status_banner = format!(
                                                "Added Sampling Round #{}",
                                                ds.rounds.len()
                                            );
                                        }
                                    }
                                    if rbtn(ui, "Remove\nRound", false).clicked() {
                                        let idx = state.workbook.active_dataset;
                                        if let Some(ds) = state.workbook.datasets.get_mut(idx) {
                                            if !ds.rounds.is_empty() {
                                                ds.rounds.pop();
                                                state.active_round_view = None;
                                            }
                                        }
                                    }
                                    if rbtn(ui, "Calculate\nMaster Data", false).clicked() {
                                        let n = state.workbook.calculate_master_data_from_rounds();
                                        state.active_round_view = None;
                                        state.status_banner = format!(
                                            "Averaged {n} master cell(s) from sampling rounds"
                                        );
                                    }
                                });
                            }
                            MbNavPage::DataStatus => {
                                ribbon_group(ui, "Data Status", |ui| {
                                    for st in [
                                        PreMeasStatus::Measured,
                                        PreMeasStatus::Fixed,
                                        PreMeasStatus::Guesstimated,
                                        PreMeasStatus::Missing,
                                        PreMeasStatus::Excluded,
                                    ] {
                                        if rbtn(ui, st.label(), state.status_paint_tool == st)
                                            .clicked()
                                        {
                                            state.status_paint_tool = st;
                                        }
                                    }
                                    if rbtn(ui, "Reset Meas.\nStatus", false).clicked() {
                                        state.workbook.reset_measurement_statuses();
                                        state.status_banner =
                                            "Reset all manual measurement status overrides".into();
                                    }
                                    if rbtn(ui, "Refresh\nCharts", false).clicked() {
                                        state.run_calculate_active();
                                    }
                                });
                            }
                            MbNavPage::ReviewSd | MbNavPage::EditSdTable => {
                                ribbon_group(ui, "Table", |ui| {
                                    if rbtn(ui, "Paste SD or\nRSD Data", false).clicked() {
                                        state.open_text_modal(
                                            TextModalKind::PasteSdRsd,
                                            "Paste SD Override Data (TSV)",
                                            "Stream\tCu\tFe\nFeed\t0.05\t1.5".into(),
                                        );
                                    }
                                    if rbtn(ui, "Clear SD\nOverrides", false).clicked() {
                                        if let Some(ds) = state.workbook.active_dataset_mut() {
                                            ds.sd_override.clear();
                                        }
                                    }
                                    if rbtn(
                                        ui,
                                        if state.sd_table_locked {
                                            "Unlock SD\nTable"
                                        } else {
                                            "Lock SD\nTable"
                                        },
                                        state.sd_table_locked,
                                    )
                                    .clicked()
                                    {
                                        state.sd_table_locked = !state.sd_table_locked;
                                    }
                                });
                                ribbon_group(ui, "Sampling Rounds", |ui| {
                                    if rbtn(ui, "Calculate SD\nFrom Data", false).clicked() {
                                        let n = state.workbook.calculate_sd_from_rounds();
                                        state.status_banner = format!(
                                            "Computed empirical SD from sampling rounds for {n} cell(s)"
                                        );
                                    }
                                });
                            }
                            MbNavPage::Balance => {
                                ribbon_group(ui, "Settings", |ui| {
                                    let ds_idx = state.workbook.active_dataset;
                                    if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                        if rbtn(ui, "Mineral\nSum=100", ds.mineral_sum_100)
                                            .clicked()
                                        {
                                            ds.mineral_sum_100 = !ds.mineral_sum_100;
                                        }
                                        if rbtn(ui, "Assay\nSum=100", ds.assay_sum_100).clicked()
                                        {
                                            ds.assay_sum_100 = !ds.assay_sum_100;
                                        }
                                        if rbtn(
                                            ui,
                                            "Low Grade\nWeighting ▾",
                                            state.show_low_grade_popover,
                                        )
                                        .clicked()
                                        {
                                            state.show_low_grade_popover =
                                                !state.show_low_grade_popover;
                                        }
                                        ui.menu_button(
                                            RichText::new(format!(
                                                "Method:\n{} ▾",
                                                ds.method.label()
                                            ))
                                            .size(10.5),
                                            |ui| {
                                                for m in [
                                                    SolveMethod::LS,
                                                    SolveMethod::LsMinMax,
                                                    SolveMethod::NNLS,
                                                    SolveMethod::CLS,
                                                    SolveMethod::LLS,
                                                ] {
                                                    if ui
                                                        .selectable_label(
                                                            ds.method == m,
                                                            m.label(),
                                                        )
                                                        .clicked()
                                                    {
                                                        ds.method = m;
                                                        ui.close_menu();
                                                    }
                                                }
                                            },
                                        );
                                    }
                                });

                                ribbon_group(ui, "Run Balancing", |ui| {
                                    let ds_idx = state.workbook.active_dataset;
                                    if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                        ui.menu_button(
                                            RichText::new(format!(
                                                "Sizing:\n{} ▾",
                                                ds.sizing.label()
                                            ))
                                            .size(10.5),
                                            |ui| {
                                                for sz in [
                                                    SizingMode::Unsized,
                                                    SizingMode::Sized,
                                                    SizingMode::SizedByAssay,
                                                ] {
                                                    if ui
                                                        .selectable_label(
                                                            ds.sizing == sz,
                                                            sz.label(),
                                                        )
                                                        .clicked()
                                                    {
                                                        ds.sizing = sz;
                                                        ui.close_menu();
                                                    }
                                                }
                                            },
                                        );
                                        ui.menu_button(
                                            RichText::new(format!(
                                                "Balance Type:\n{} ▾",
                                                ds.balance_type.label()
                                            ))
                                            .size(10.5),
                                            |ui| {
                                                for bt in [
                                                    BalanceType::SolidsOnly,
                                                    BalanceType::AssaysOnly,
                                                    BalanceType::SolidsAndAssays,
                                                    BalanceType::SolidsWaterAndAssays,
                                                ] {
                                                    if ui
                                                        .selectable_label(
                                                            ds.balance_type == bt,
                                                            bt.label(),
                                                        )
                                                        .clicked()
                                                    {
                                                        ds.balance_type = bt;
                                                        ui.close_menu();
                                                    }
                                                }
                                            },
                                        );
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("⚖ Calculate")
                                                    .strong()
                                                    .size(11.0)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(Color32::from_rgb(37, 99, 235))
                                            .min_size(Vec2::new(82.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        state.run_calculate_active();
                                    }
                                    if rbtn(ui, "Freeze\nBalanced", false).clicked() {
                                        let n = state.workbook.freeze_balanced();
                                        state.status_banner = format!(
                                            "Froze {n} balanced value(s) as Fixed measurements"
                                        );
                                    }
                                    if rbtn(ui, "Clear & Unfreeze\nBalanced", false).clicked() {
                                        state.workbook.clear_and_unfreeze_balanced();
                                        state.status_banner =
                                            "Cleared solution & unfroze frozen measurements".into();
                                    }
                                    ui.menu_button(RichText::new("Batch\nRun ▾").size(10.5), |ui| {
                                        if ui.button("Run All Selected Data Sets").clicked() {
                                            let res = SoryntelApplication::mb_calculate_batch(
                                                &mut state.workbook,
                                            );
                                            let ok_cnt = res.iter().filter(|r| r.is_ok()).count();
                                            state.status_banner = format!(
                                                "Batch Run: {ok_cnt}/{} dataset(s) solved",
                                                res.len()
                                            );
                                            ui.close_menu();
                                        }
                                    });
                                });

                                ribbon_group(ui, "Data Tools", |ui| {
                                    if rbtn(ui, "Balanced Elements\nto Minerals...", false)
                                        .clicked()
                                    {
                                        match state.workbook.convert_elements_to_minerals(true) {
                                            Ok(n) => {
                                                state.status_banner = format!(
                                                    "Converted balanced elements to {n} mineral cell(s)"
                                                );
                                            }
                                            Err(e) => {
                                                state.status_banner =
                                                    format!("Balanced Elem->Min: {e}");
                                            }
                                        }
                                    }
                                    if rbtn(ui, "Balance\nEquations", false).clicked() {
                                        state.show_equations_modal = true;
                                    }
                                    if let Some(ds) = state.workbook.active_dataset_mut() {
                                        if rbtn(ui, "Show Data\nBars", ds.show_data_bars).clicked()
                                        {
                                            ds.show_data_bars = !ds.show_data_bars;
                                        }
                                    }
                                });
                            }
                            MbNavPage::ReportingResults => {
                                ribbon_group(ui, "Results", |ui| {
                                    ui.menu_button(RichText::new(crate::i18n::text("common.102")).size(10.5), |ui| {
                                        if ui.button("Copy Stream Summary (TSV)").clicked() {
                                            let tsv = state.workbook.export_summary_tsv();
                                            ui.ctx().copy_text(tsv.clone());
                                            state.open_text_modal(
                                                TextModalKind::ExportPreview,
                                                "Stream Summary (TSV)",
                                                tsv,
                                            );
                                            ui.close_menu();
                                        }
                                    });
                                    ui.menu_button(
                                        RichText::new("Calculate Vol.\nFlows ▾").size(10.5),
                                        |ui| {
                                            if let Some(ds) = state.workbook.active_dataset_mut() {
                                                ui.horizontal(|ui| {
                                                    ui.label("Liquid SG:");
                                                    ui.add(
                                                        egui::DragValue::new(&mut ds.liquid_sg)
                                                            .speed(0.01)
                                                            .range(0.5..=3.0),
                                                    );
                                                });
                                            }
                                            if ui.button("Recalculate SG & m3/h").clicked() {
                                                state.run_calculate_active();
                                                state.results_tab = ResultsTab::PhysicalProperties;
                                                ui.close_menu();
                                            }
                                        },
                                    );
                                    if rbtn(ui, "Calculate All\nMineral Elements", false).clicked()
                                    {
                                        let n = state.workbook.calculate_all_mineral_elements();
                                        state.status_banner = format!(
                                            "Back-calculated elemental assays from minerals ({n} cells)"
                                        );
                                    }
                                    ui.menu_button(
                                        RichText::new("Stream\nTables ▾").size(10.5),
                                        |ui| {
                                            if ui
                                                .selectable_label(
                                                    state.results_filter_mode == 0,
                                                    "All Selected Streams",
                                                )
                                                .clicked()
                                            {
                                                state.results_filter_mode = 0;
                                                ui.close_menu();
                                            }
                                            if ui
                                                .selectable_label(
                                                    state.results_filter_mode == 1,
                                                    "External Feeds & Final Products",
                                                )
                                                .clicked()
                                            {
                                                state.results_filter_mode = 1;
                                                ui.close_menu();
                                            }
                                        },
                                    );
                                    if rbtn(ui, "Export to\nExcel (.xlsx)", false).clicked() {
                                        state.export_xlsx_to_tmp();
                                    }
                                    if rbtn(ui, "Save Stream\nAs...", false).clicked() {
                                        let tsv = state.workbook.export_summary_tsv();
                                        state.open_text_modal(
                                            TextModalKind::ExportPreview,
                                            "Stream Export Preview (TSV)",
                                            tsv,
                                        );
                                    }
                                });
                            }
                            MbNavPage::ReportingCharts => {
                                ribbon_group(ui, "Charts", |ui| {
                                    if rbtn(ui, "Add New\nPanel", false).clicked() {
                                        let n = state.workbook.chart_panels.len() + 1;
                                        state.workbook.chart_panels.push(ChartPanelSpec {
                                            title: format!("Custom Panel {n}"),
                                            show_total_solids: true,
                                            show_stream_composition: false,
                                            selected_var_ids: state
                                                .workbook
                                                .variables
                                                .iter()
                                                .filter(|v| v.selected && v.type_code.is_assay())
                                                .take(3)
                                                .map(|v| v.id)
                                                .collect(),
                                        });
                                    }
                                    if rbtn(ui, "Add Bar\nChart", false).clicked() {
                                        state.workbook.chart_panels.push(ChartPanelSpec {
                                            title: "Assay Recovery Comparison".into(),
                                            show_total_solids: true,
                                            show_stream_composition: false,
                                            selected_var_ids: state
                                                .workbook
                                                .variables
                                                .iter()
                                                .filter(|v| v.selected)
                                                .map(|v| v.id)
                                                .collect(),
                                        });
                                    }
                                    ui.menu_button(
                                        RichText::new("Add Default\nCharts ▾").size(10.5),
                                        |ui| {
                                            if ui.button("1. Total Solids Bar Chart").clicked() {
                                                state.workbook.chart_panels.push(ChartPanelSpec {
                                                    title: "Total Solids".into(),
                                                    show_total_solids: true,
                                                    show_stream_composition: false,
                                                    selected_var_ids: Vec::new(),
                                                });
                                                ui.close_menu();
                                            }
                                            if ui.button("2. Stream Composition (100% Stacked)")
                                                .clicked()
                                            {
                                                state.workbook.chart_panels.push(ChartPanelSpec {
                                                    title: "Stream Composition".into(),
                                                    show_total_solids: false,
                                                    show_stream_composition: true,
                                                    selected_var_ids: Vec::new(),
                                                });
                                                ui.close_menu();
                                            }
                                        },
                                    );
                                    if rbtn(ui, "Chart\nProperties", state.show_chart_props_panel)
                                        .clicked()
                                    {
                                        state.show_chart_props_panel =
                                            !state.show_chart_props_panel;
                                    }
                                    if rbtn(ui, "Reset", false).clicked() {
                                        state.workbook.chart_panels = vec![
                                            ChartPanelSpec {
                                                title: "Total Solids".into(),
                                                show_total_solids: true,
                                                show_stream_composition: false,
                                                selected_var_ids: Vec::new(),
                                            },
                                            ChartPanelSpec {
                                                title: "Stream Composition".into(),
                                                show_total_solids: false,
                                                show_stream_composition: true,
                                                selected_var_ids: Vec::new(),
                                            },
                                        ];
                                    }
                                });
                            }
                        }
                    });
                });
        });
}

// ---------------------------------------------------------------------------
// Zone 2: Left Collapsible Balancing Navigator
// ---------------------------------------------------------------------------

fn nav_accordion(
    ui: &mut egui::Ui,
    open: &mut bool,
    title: &str,
    items: &[(&str, MbNavPage)],
    active_page: &mut MbNavPage,
) {
    let arrow = if *open { "▼" } else { "▶" };
    if ui
        .add(
            egui::Button::new(
                RichText::new(format!("{arrow}  {title}"))
                    .strong()
                    .size(11.5)
                    .color(DARK_INK),
            )
            .fill(HDR_GRAY_BG)
            .min_size(Vec2::new(ui.available_width(), 22.0)),
        )
        .clicked()
    {
        *open = !*open;
    }
    if *open {
        for &(label, page) in items {
            let is_active = *active_page == page;
            let fill = if is_active {
                NAV_SALMON_ACTIVE
            } else {
                Color32::TRANSPARENT
            };
            let txt = if is_active {
                RichText::new(label).strong().size(11.5).color(DARK_INK)
            } else {
                RichText::new(label).size(11.5).color(Color32::from_rgb(30, 41, 59))
            };
            if ui
                .add(
                    egui::Button::new(txt)
                        .fill(fill)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(ui.available_width(), 21.0)),
                )
                .clicked()
            {
                *active_page = page;
            }
        }
    }
}

fn render_left_navigator(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, body_h: f32) {
    if !state.nav_open {
        egui::Frame::none()
            .fill(NAV_BG)
            .stroke(Stroke::new(1.0, RIBBON_BORDER))
            .show(ui, |ui| {
                ui.set_width(26.0);
                ui.set_height(body_h);
                if ui.button("▶").on_hover_text("Expand Balancing Navigator").clicked() {
                    state.nav_open = true;
                }
            });
        return;
    }

    let u_sel = state.workbook.units.iter().filter(|u| u.selected).count();
    let u_tot = state.workbook.units.len();
    let s_sel = state.workbook.streams.iter().filter(|s| s.selected).count();
    let s_tot = state.workbook.streams.len();
    let v_sel = state.workbook.variables.iter().filter(|v| v.selected).count();
    let v_tot = state.workbook.variables.len();
    let sf_tot = state.workbook.size_fractions.len();
    let ds_tot = state.workbook.datasets.len();

    egui::Frame::none()
        .fill(NAV_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_width(220.0);
            ui.set_height(body_h);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("  Balancing Navigator")
                        .strong()
                        .size(11.5)
                        .color(DARK_INK),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("◀").on_hover_text("Collapse Navigator").clicked() {
                        state.nav_open = false;
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("mb_left_nav_scroll")
                .show(ui, |ui| {
                    let l_units = format!("1. Units ({u_sel} / {u_tot})");
                    let l_streams = format!("2. Streams ({s_sel} / {s_tot})");
                    let l_vars = format!("3. Variables ({v_sel} / {v_tot})");
                    let l_sfs = format!("4. Size Fractions ({sf_tot})");
                    let l_dss = format!("5. Data Sets ({ds_tot})");

                    nav_accordion(
                        ui,
                        &mut state.nav_exp_open,
                        "Experimental Data",
                        &[
                            (&l_units, MbNavPage::Units),
                            (&l_streams, MbNavPage::Streams),
                            (&l_vars, MbNavPage::Variables),
                            (&l_sfs, MbNavPage::SizeFractions),
                            (&l_dss, MbNavPage::DataSets),
                            ("6. Measurement Data", MbNavPage::MeasurementData),
                        ],
                        &mut state.active_page,
                    );
                    ui.add_space(4.0);
                    nav_accordion(
                        ui,
                        &mut state.nav_rev_open,
                        "Review Data",
                        &[("1. Data Status", MbNavPage::DataStatus)],
                        &mut state.active_page,
                    );
                    ui.add_space(4.0);
                    nav_accordion(
                        ui,
                        &mut state.nav_sd_open,
                        "Standard Deviations",
                        &[
                            ("1. Review SD", MbNavPage::ReviewSd),
                            ("2. Edit SD Table", MbNavPage::EditSdTable),
                        ],
                        &mut state.active_page,
                    );
                    ui.add_space(4.0);
                    nav_accordion(
                        ui,
                        &mut state.nav_bal_open,
                        "Balancing",
                        &[("1. Balance", MbNavPage::Balance)],
                        &mut state.active_page,
                    );
                    ui.add_space(4.0);
                    nav_accordion(
                        ui,
                        &mut state.nav_rep_open,
                        "Reporting",
                        &[
                            ("1. Results", MbNavPage::ReportingResults),
                            ("2. Charts", MbNavPage::ReportingCharts),
                        ],
                        &mut state.active_page,
                    );
                });
        });
}

// ---------------------------------------------------------------------------
// Zone 3: Central Working Area
// ---------------------------------------------------------------------------

fn render_working_area(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, body_h: f32) {
    // Top breadcrumb & active Size Fraction / Data Set strip
    egui::Frame::none()
        .fill(HDR_GRAY_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .inner_margin(egui::Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(state.active_page.title()).strong().size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Size fraction switcher
                    if state.workbook.size_fractions.len() > 1 {
                        let cur_sf = state
                            .workbook
                            .size_fractions
                            .get(state.active_frac_idx)
                            .cloned()
                            .unwrap_or_else(|| "Bulk".into());
                        egui::ComboBox::from_id_salt("mb_hdr_sf_combo")
                            .width(120.0)
                            .selected_text(format!("Fraction: {cur_sf}"))
                            .show_ui(ui, |ui| {
                                for (i, f) in state.workbook.size_fractions.iter().enumerate() {
                                    ui.selectable_value(&mut state.active_frac_idx, i, f);
                                }
                            });
                    }
                    // Data Set switcher
                    if !state.workbook.datasets.is_empty() {
                        let ds_name = state
                            .workbook
                            .datasets
                            .get(state.workbook.active_dataset)
                            .map(|d| d.name.clone())
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt("mb_hdr_ds_combo")
                            .width(145.0)
                            .selected_text(ds_name)
                            .show_ui(ui, |ui| {
                                for (i, d) in state.workbook.datasets.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut state.workbook.active_dataset,
                                        i,
                                        &d.name,
                                    );
                                }
                            });
                    }
                });
            });
        });

    let inner_h = (body_h - 32.0).max(180.0);
    match state.active_page {
        MbNavPage::Units => render_units_page(ui, state, inner_h),
        MbNavPage::Streams => render_streams_page(ui, state, inner_h),
        MbNavPage::Variables => render_variables_page(ui, state, inner_h),
        MbNavPage::SizeFractions => render_size_fractions_page(ui, state, inner_h),
        MbNavPage::DataSets => render_datasets_page(ui, state, inner_h),
        MbNavPage::MeasurementData => render_measurement_data_page(ui, state, inner_h),
        MbNavPage::DataStatus => render_data_status_page(ui, state, inner_h),
        MbNavPage::ReviewSd => render_review_sd_page(ui, state, inner_h),
        MbNavPage::EditSdTable => render_edit_sd_page(ui, state, inner_h),
        MbNavPage::Balance => render_balance_page(ui, state, inner_h),
        MbNavPage::ReportingResults => render_reporting_results_page(ui, state, inner_h),
        MbNavPage::ReportingCharts => render_reporting_charts_page(ui, state, inner_h),
    }
}

fn cell_hdr(ui: &mut egui::Ui, label: &str, w: f32) {
    ui.allocate_ui(Vec2::new(w, 22.0), |ui| {
        egui::Frame::none()
            .fill(HDR_GRAY_BG)
            .stroke(Stroke::new(0.5, GRID_LINE))
            .inner_margin(egui::Margin::symmetric(5.0, 3.0))
            .show(ui, |ui| {
                ui.set_min_width(w - 10.0);
                ui.label(RichText::new(label).strong().size(11.0).color(DARK_INK));
            });
    });
}

fn render_units_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        egui::Grid::new("mb_units_grid")
            .striped(true)
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Unit Name", 190.0);
                cell_hdr(ui, "Selected", 75.0);
                cell_hdr(ui, "Size Transforming", 130.0);
                cell_hdr(ui, "Input Streams", 200.0);
                cell_hdr(ui, "Output Streams", 200.0);
                ui.end_row();

                let streams = state.workbook.streams.clone();
                for u in &mut state.workbook.units {
                    let is_sel = state.selected_unit_id == Some(u.id);
                    if ui
                        .selectable_label(is_sel, RichText::new(&u.name).size(11.5))
                        .clicked()
                    {
                        state.selected_unit_id = Some(u.id);
                    }
                    ui.checkbox(&mut u.selected, "");
                    ui.checkbox(&mut u.size_transforming, "");
                    let ins: Vec<&str> = streams
                        .iter()
                        .filter(|s| s.dest_unit == Some(u.id))
                        .map(|s| s.name.as_str())
                        .collect();
                    let outs: Vec<&str> = streams
                        .iter()
                        .filter(|s| s.source_unit == Some(u.id))
                        .map(|s| s.name.as_str())
                        .collect();
                    ui.label(RichText::new(ins.join(", ")).size(11.0));
                    ui.label(RichText::new(outs.join(", ")).size(11.0));
                    ui.end_row();
                }
            });
    });
}

fn render_streams_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let unit_names: Vec<(usize, String)> = state
        .workbook
        .units
        .iter()
        .map(|u| (u.id, u.name.clone()))
        .collect();
    let unit_label = |id: Option<usize>| -> String {
        id.and_then(|uid| unit_names.iter().find(|x| x.0 == uid).map(|x| x.1.clone()))
            .unwrap_or_else(|| "External".to_string())
    };
    let eqs = state.workbook.stream_equations.clone();
    let stream_names: Vec<(usize, String)> = state
        .workbook
        .streams
        .iter()
        .map(|s| (s.id, s.name.clone()))
        .collect();

    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        egui::Grid::new("mb_streams_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream Name", 130.0);
                cell_hdr(ui, "Sel.", 40.0);
                cell_hdr(ui, "Stream Type", 110.0);
                cell_hdr(ui, "Source", 105.0);
                cell_hdr(ui, "Dest.", 105.0);
                cell_hdr(ui, "Flows", 48.0);
                cell_hdr(ui, "Assays", 48.0);
                cell_hdr(ui, "Sampling SD (%)", 95.0);
                cell_hdr(ui, "Sample Quality", 105.0);
                cell_hdr(ui, "Equation", 130.0);
                ui.end_row();

                for s in &mut state.workbook.streams {
                    let is_sel = state.selected_stream_id == Some(s.id);
                    if ui
                        .selectable_label(is_sel, RichText::new(&s.name).size(11.0))
                        .clicked()
                    {
                        state.selected_stream_id = Some(s.id);
                    }
                    ui.checkbox(&mut s.selected, "");
                    egui::ComboBox::from_id_salt(("stype_c", s.id))
                        .width(98.0)
                        .selected_text(s.stream_type.label())
                        .show_ui(ui, |ui| {
                            for t in [
                                StreamType::SolidsSlurry,
                                StreamType::LiquidWater,
                                StreamType::Unknown,
                            ] {
                                ui.selectable_value(&mut s.stream_type, t, t.label());
                            }
                        });
                    ui.label(RichText::new(unit_label(s.source_unit)).size(11.0));
                    ui.label(RichText::new(unit_label(s.dest_unit)).size(11.0));
                    ui.checkbox(&mut s.flows_enabled, "");
                    ui.checkbox(&mut s.assays_enabled, "");
                    ui.add(
                        egui::DragValue::new(&mut s.sampling_sd)
                            .speed(0.05)
                            .range(0.0..=50.0),
                    );
                    egui::ComboBox::from_id_salt(("squal_c", s.id))
                        .width(92.0)
                        .selected_text(s.sample_quality.label())
                        .show_ui(ui, |ui| {
                            for q in [
                                SampleQuality::Good,
                                SampleQuality::Moderate,
                                SampleQuality::Bad,
                            ] {
                                ui.selectable_value(&mut s.sample_quality, q, q.label());
                            }
                        });
                    if let Some(eq) = eqs.iter().find(|e| e.target_stream_id == s.id) {
                        let src_name = stream_names
                            .iter()
                            .find(|x| x.0 == eq.source_stream_id)
                            .map(|x| x.1.as_str())
                            .unwrap_or("?");
                        egui::Frame::none().fill(GREEN_EQ_BG).show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{} * {}", eq.factor, src_name))
                                    .size(10.5)
                                    .color(DARK_INK),
                            );
                        });
                    } else {
                        ui.label(RichText::new("-").color(GRAY_EXCLUDED));
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_variables_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    ui.horizontal(|ui| {
        ui.label("Quick Add:");
        ui.add(egui::TextEdit::singleline(&mut state.new_var_name).desired_width(80.0));
        ui.add(egui::TextEdit::singleline(&mut state.new_var_unit).desired_width(45.0));
        egui::ComboBox::from_id_salt("mb_new_var_type")
            .width(90.0)
            .selected_text(state.new_var_type.code())
            .show_ui(ui, |ui| {
                for t in [
                    VarTypeCode::SF,
                    VarTypeCode::SC,
                    VarTypeCode::A,
                    VarTypeCode::M,
                    VarTypeCode::SA,
                    VarTypeCode::SP,
                    VarTypeCode::Info,
                ] {
                    ui.selectable_value(&mut state.new_var_type, t, t.type_name());
                }
            });
    });
    ui.separator();

    egui::ScrollArea::both().max_height(h - 32.0).show(ui, |ui| {
        egui::Grid::new("mb_vars_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Name", 130.0);
                cell_hdr(ui, "Unit", 50.0);
                cell_hdr(ui, "Code", 55.0);
                cell_hdr(ui, "Sel.", 40.0);
                cell_hdr(ui, "Measurement Error Method", 165.0);
                cell_hdr(ui, "Param a", 65.0);
                cell_hdr(ui, "Param b", 65.0);
                cell_hdr(ui, "Param c", 65.0);
                cell_hdr(ui, "LowGr.", 52.0);
                cell_hdr(ui, "Formula / SG", 120.0);
                ui.end_row();

                for v in &mut state.workbook.variables {
                    let excluded =
                        state.workbook.balancing_of_minerals && v.type_code == VarTypeCode::A;
                    let is_sel = state.selected_var_id == Some(v.id);
                    let col = if excluded { GRAY_EXCLUDED } else { DARK_INK };
                    if ui
                        .selectable_label(is_sel, RichText::new(&v.name).size(11.0).color(col))
                        .clicked()
                    {
                        state.selected_var_id = Some(v.id);
                    }
                    ui.label(RichText::new(&v.unit).size(11.0).color(col));
                    ui.label(RichText::new(v.type_code.code()).size(11.0).color(col));
                    ui.checkbox(&mut v.selected, "");
                    egui::ComboBox::from_id_salt(("vmethod_c", v.id))
                        .width(155.0)
                        .selected_text(v.method.label())
                        .show_ui(ui, |ui| {
                            for m in [
                                MeasurementMethod::FixedAbsolute,
                                MeasurementMethod::FixedRelative,
                                MeasurementMethod::SShape,
                                MeasurementMethod::MineralGradeBased,
                                MeasurementMethod::SizeFractionWhiten,
                            ] {
                                ui.selectable_value(&mut v.method, m, m.label());
                            }
                        });
                    ui.add(egui::DragValue::new(&mut v.param_a).speed(0.05));
                    ui.add(egui::DragValue::new(&mut v.param_b).speed(0.01));
                    ui.add(egui::DragValue::new(&mut v.param_c).speed(0.05));
                    ui.checkbox(&mut v.low_grade_weighted, "");
                    let f_txt = match (&v.mineral_formula, v.mineral_sg) {
                        (Some(f), Some(sg)) => format!("{f} ({sg:.2})"),
                        (Some(f), None) => f.clone(),
                        _ => "-".to_string(),
                    };
                    ui.label(RichText::new(f_txt).size(10.5).color(col));
                    ui.end_row();
                }
            });
    });
}

fn render_size_fractions_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    ui.horizontal(|ui| {
        ui.label("New Size Fraction Label:");
        ui.add(egui::TextEdit::singleline(&mut state.new_sf_name).desired_width(140.0));
        ui.label(
            RichText::new("Index 0 ('Bulk') is reserved and cannot be deleted.")
                .small()
                .color(Color32::from_rgb(100, 116, 139)),
        );
    });
    ui.separator();
    egui::ScrollArea::vertical().max_height(h - 32.0).show(ui, |ui| {
        egui::Grid::new("mb_sf_grid").striped(true).show(ui, |ui| {
            cell_hdr(ui, "#", 40.0);
            cell_hdr(ui, "Size Fraction Label", 220.0);
            cell_hdr(ui, "Role", 180.0);
            ui.end_row();
            for (i, label) in state.workbook.size_fractions.iter().enumerate() {
                ui.label(format!("{i}"));
                if ui
                    .selectable_label(state.active_frac_idx == i, RichText::new(label).strong())
                    .clicked()
                {
                    state.active_frac_idx = i;
                }
                ui.label(if i == 0 {
                    "Bulk Unsized Master"
                } else {
                    "PSD Sieve Sub-fraction"
                });
                ui.end_row();
            }
        });
    });
}

fn render_datasets_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    ui.horizontal(|ui| {
        ui.label("New Data Set Name:");
        ui.add(egui::TextEdit::singleline(&mut state.new_ds_name).desired_width(150.0));
    });
    ui.separator();
    egui::ScrollArea::both().max_height(h - 32.0).show(ui, |ui| {
        egui::Grid::new("mb_ds_grid").striped(true).show(ui, |ui| {
            cell_hdr(ui, "Active", 55.0);
            cell_hdr(ui, "Batch", 50.0);
            cell_hdr(ui, "Data Set Name", 180.0);
            cell_hdr(ui, "Method", 95.0);
            cell_hdr(ui, "Sizing", 110.0);
            cell_hdr(ui, "Balance Type", 160.0);
            cell_hdr(ui, "Solved WSSQ", 90.0);
            ui.end_row();

            for (i, ds) in state.workbook.datasets.iter_mut().enumerate() {
                let is_active = state.workbook.active_dataset == i;
                if ui.radio(is_active, "").clicked() {
                    state.workbook.active_dataset = i;
                }
                ui.checkbox(&mut ds.selected_for_batch, "");
                ui.add(egui::TextEdit::singleline(&mut ds.name).desired_width(170.0));
                ui.label(ds.method.label());
                ui.label(ds.sizing.label());
                ui.label(ds.balance_type.label());
                let wssq = state
                    .workbook
                    .solutions
                    .get(i)
                    .and_then(|o| o.as_ref())
                    .map(|s| format!("{:.4}", s.wssq))
                    .unwrap_or_else(|| "-".into());
                ui.label(RichText::new(wssq).color(BLUE_TEXT));
                ui.end_row();
            }
        });
    });
}

fn render_measurement_data_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let ds_idx = state.workbook.active_dataset;
    let round_cnt = state
        .workbook
        .datasets
        .get(ds_idx)
        .map(|d| d.rounds.len())
        .unwrap_or(0);

    // Master Data + Sampling Round tabs
    ui.horizontal(|ui| {
        if ui
            .selectable_label(state.active_round_view.is_none(), "Master Data")
            .clicked()
        {
            state.active_round_view = None;
        }
        for r in 0..round_cnt {
            if ui
                .selectable_label(
                    state.active_round_view == Some(r),
                    format!("Round {}", r + 1),
                )
                .clicked()
            {
                state.active_round_view = Some(r);
            }
        }
        if state.meas_table_locked {
            ui.label(RichText::new("🔒 LOCKED").color(Color32::from_rgb(185, 28, 28)));
        }
    });
    ui.separator();

    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();
    let eqs = state.workbook.stream_equations.clone();
    let sg_map = state
        .workbook
        .solutions
        .get(ds_idx)
        .and_then(|o| o.as_ref())
        .map(|s| s.sg_by_stream.clone())
        .unwrap_or_default();

    egui::ScrollArea::both().max_height(h - 32.0).show(ui, |ui| {
        egui::Grid::new("mb_meas_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream \\ Var", 130.0);
                for v in &vars {
                    let hdr = format!("{} ({})", v.name, v.unit);
                    cell_hdr(ui, &hdr, 84.0);
                }
                if state.show_sg_column {
                    cell_hdr(ui, "SG (t/m3)", 70.0);
                }
                ui.end_row();

                for s in &streams {
                    let eq_match = eqs.iter().find(|e| e.target_stream_id == s.id);
                    ui.label(RichText::new(&s.name).strong().size(11.0));
                    for v in &vars {
                        let k = CellKey {
                            frac_idx: state.active_frac_idx,
                            stream_id: s.id,
                            var_id: v.id,
                        };
                        let pre_st = state.workbook.cell_pre_status(ds_idx, k);
                        let is_eq_cell = eq_match.is_some();

                        let mut val_str = {
                            let ds = &state.workbook.datasets[ds_idx];
                            let opt = match state.active_round_view {
                                Some(r) => ds.rounds.get(r).and_then(|m| m.get(&k)).copied(),
                                None => ds.values.get(&k).copied(),
                            };
                            opt.map(|x| format!("{x:.4}"))
                                .unwrap_or_default()
                        };

                        let text_col = match pre_st {
                            PreMeasStatus::Guesstimated => BLUE_TEXT,
                            PreMeasStatus::Excluded => GRAY_EXCLUDED,
                            PreMeasStatus::Fixed => Color32::from_rgb(180, 83, 9),
                            _ => DARK_INK,
                        };
                        let bg_col = if is_eq_cell {
                            GREEN_EQ_BG
                        } else if state.selected_cell == Some(k) {
                            Color32::from_rgb(219, 234, 254)
                        } else {
                            Color32::WHITE
                        };

                        let resp = egui::Frame::none().fill(bg_col).show(ui, |ui| {
                            let te = egui::TextEdit::singleline(&mut val_str)
                                .desired_width(74.0)
                                .text_color(text_col)
                                .interactive(!state.meas_table_locked && pre_st != PreMeasStatus::Excluded);
                            ui.add(te)
                        });
                        if resp.inner.clicked() {
                            state.selected_cell = Some(k);
                        }
                        if resp.inner.changed() {
                            if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                let target_map = match state.active_round_view {
                                    Some(r) => ds.rounds.get_mut(r),
                                    None => Some(&mut ds.values),
                                };
                                if let Some(map) = target_map {
                                    if val_str.trim().is_empty() {
                                        map.remove(&k);
                                    } else if let Ok(vnum) = val_str.trim().replace(',', ".").parse::<f64>() {
                                        map.insert(k, vnum);
                                        ds.estimated_cells.remove(&k);
                                    }
                                }
                            }
                        }
                        if let Some(eq) = eq_match {
                            let src_name = streams
                                .iter()
                                .find(|x| x.id == eq.source_stream_id)
                                .map(|x| x.name.as_str())
                                .unwrap_or("?");
                            resp.response.on_hover_text(format!(
                                "Balanced value will be by equation value.\nEquation = {} * {}",
                                eq.factor, src_name
                            ));
                        }
                    }
                    if state.show_sg_column {
                        let sg = sg_map.get(&s.id).copied().unwrap_or(3.5);
                        ui.label(RichText::new(format!("{sg:.2}")).size(11.0).color(BLUE_TEXT));
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_data_status_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let ds_idx = state.workbook.active_dataset;
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    // Tallies for two Chapter 51 pie charts (`Measured Data` and `Balanced Data`)
    let mut cnt_meas = 0usize;
    let mut cnt_miss = 0usize;
    let mut cnt_guess = 0usize;
    let mut cnt_fixed = 0usize;
    let mut cnt_excl = 0usize;

    let mut cnt_bal = 0usize;
    let mut cnt_calc = 0usize;
    let mut cnt_na = 0usize;

    let sol_opt = state
        .workbook
        .solutions
        .get(ds_idx)
        .and_then(|o| o.as_ref())
        .cloned();

    for s in &streams {
        for v in &vars {
            let k = CellKey {
                frac_idx: state.active_frac_idx,
                stream_id: s.id,
                var_id: v.id,
            };
            match state.workbook.cell_pre_status(ds_idx, k) {
                PreMeasStatus::Measured => cnt_meas += 1,
                PreMeasStatus::Missing => cnt_miss += 1,
                PreMeasStatus::Guesstimated => cnt_guess += 1,
                PreMeasStatus::Fixed => cnt_fixed += 1,
                PreMeasStatus::Excluded => cnt_excl += 1,
            }
            let bal_st = sol_opt
                .as_ref()
                .and_then(|sol| sol.cells.get(&k))
                .map(|c| c.bal_status)
                .unwrap_or(PostBalStatus::NonAvailable);
            match bal_st {
                PostBalStatus::Balanced => cnt_bal += 1,
                PostBalStatus::Calculated => cnt_calc += 1,
                PostBalStatus::NonAvailable => cnt_na += 1,
            }
        }
    }

    ui.columns(2, |cols| {
        render_pie_chart_card(
            &mut cols[0],
            "Measured Data",
            &[
                ("Measured", cnt_meas, Color32::from_rgb(34, 197, 94)),
                ("Missing", cnt_miss, Color32::from_rgb(203, 213, 225)),
                ("Guesstimated", cnt_guess, Color32::from_rgb(59, 130, 246)),
                ("Fixed", cnt_fixed, Color32::from_rgb(245, 158, 11)),
                ("Excluded", cnt_excl, Color32::from_rgb(100, 116, 139)),
            ],
        );
        render_pie_chart_card(
            &mut cols[1],
            "Balanced Data",
            &[
                ("Balanced", cnt_bal, Color32::from_rgb(37, 99, 235)),
                ("Calculated", cnt_calc, Color32::from_rgb(16, 185, 129)),
                ("Non Available", cnt_na, Color32::from_rgb(226, 232, 240)),
            ],
        );
    });

    ui.separator();
    ui.label(
        RichText::new(format!(
            "Click any cell below to apply Data Status = '{}' (change active painter tool in Ribbon)",
            state.status_paint_tool.label()
        ))
        .small()
        .color(Color32::from_rgb(71, 85, 105)),
    );

    egui::ScrollArea::both()
        .max_height((h - 175.0).max(120.0))
        .show(ui, |ui| {
            egui::Grid::new("mb_status_matrix_grid")
                .striped(false)
                .spacing([3.0, 2.0])
                .show(ui, |ui| {
                    cell_hdr(ui, "Stream \\ Var", 125.0);
                    for v in &vars {
                        cell_hdr(ui, &v.name, 78.0);
                    }
                    ui.end_row();
                    for s in &streams {
                        ui.label(RichText::new(&s.name).strong().size(11.0));
                        for v in &vars {
                            let k = CellKey {
                                frac_idx: state.active_frac_idx,
                                stream_id: s.id,
                                var_id: v.id,
                            };
                            let st = state.workbook.cell_pre_status(ds_idx, k);
                            let fill = match st {
                                PreMeasStatus::Measured => Color32::from_rgb(220, 252, 231),
                                PreMeasStatus::Missing => Color32::from_rgb(241, 245, 249),
                                PreMeasStatus::Guesstimated => Color32::from_rgb(219, 234, 254),
                                PreMeasStatus::Fixed => Color32::from_rgb(254, 243, 199),
                                PreMeasStatus::Excluded => Color32::from_rgb(226, 232, 240),
                            };
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(st.label()).size(10.0))
                                        .fill(fill)
                                        .min_size(Vec2::new(76.0, 20.0)),
                                )
                                .clicked()
                            {
                                state.selected_cell = Some(k);
                                if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                    ds.meas_status_override
                                        .insert(k, state.status_paint_tool);
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
        });
}

fn render_pie_chart_card(ui: &mut egui::Ui, title: &str, slices: &[(&str, usize, Color32)]) {
    egui::Frame::none()
        .fill(ALT_ROW_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().size(12.0));
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(112.0, 112.0), egui::Sense::hover());
                let painter = ui.painter();
                let center = rect.center();
                let radius = 48.0;
                let total: usize = slices.iter().map(|s| s.1).sum();
                if total == 0 {
                    painter.circle_filled(center, radius, Color32::LIGHT_GRAY);
                } else {
                    let mut angle_acc = -std::f32::consts::FRAC_PI_2;
                    for &(_, cnt, color) in slices {
                        if cnt == 0 {
                            continue;
                        }
                        let frac = cnt as f32 / total as f32;
                        let sweep = frac * std::f32::consts::TAU;
                        let steps = ((sweep * 24.0).ceil() as usize).max(3);
                        let mut pts = Vec::with_capacity(steps + 2);
                        pts.push(center);
                        for i in 0..=steps {
                            let a = angle_acc + sweep * (i as f32 / steps as f32);
                            pts.push(Pos2::new(
                                center.x + radius * a.cos(),
                                center.y + radius * a.sin(),
                            ));
                        }
                        painter.add(egui::Shape::convex_polygon(
                            pts,
                            color,
                            Stroke::new(1.0, Color32::WHITE),
                        ));
                        angle_acc += sweep;
                    }
                }
                ui.vertical(|ui| {
                    for &(label, cnt, color) in slices {
                        let pct = if total > 0 {
                            100.0 * cnt as f64 / total as f64
                        } else {
                            0.0
                        };
                        ui.horizontal(|ui| {
                            let (sq, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(sq, Rounding::same(2.0), color);
                            ui.label(RichText::new(format!("{label}: {cnt} ({pct:.0}%)")).size(10.5));
                        });
                    }
                });
            });
        });
}

fn render_review_sd_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let ds_idx = state.workbook.active_dataset;
    let ds = state.workbook.datasets[ds_idx].clone();
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        egui::Grid::new("mb_review_sd_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream", 115.0);
                cell_hdr(ui, "Variable", 95.0);
                cell_hdr(ui, "Measured", 80.0);
                cell_hdr(ui, "RSD (%)", 75.0);
                cell_hdr(ui, "SD Meas.", 80.0);
                cell_hdr(ui, "SD Sampling", 82.0);
                cell_hdr(ui, "SD Eff.", 80.0);
                cell_hdr(ui, "Model Formula", 130.0);
                ui.end_row();

                for s in &streams {
                    for v in &vars {
                        let k = CellKey {
                            frac_idx: state.active_frac_idx,
                            stream_id: s.id,
                            var_id: v.id,
                        };
                        let st = state.workbook.cell_pre_status(ds_idx, k);
                        let val = ds.values.get(&k).copied();
                        let ov = ds.sd_override.get(&k).copied();
                        let (sd_m, rsd, sd_s, sd_e) = evaluate_cell_uncertainty(
                            val,
                            v,
                            s,
                            st,
                            ov,
                            ds.null_sd_estimate,
                            ds.low_grade_coeff,
                        );
                        ui.label(RichText::new(&s.name).size(11.0));
                        ui.label(RichText::new(&v.name).size(11.0));
                        ui.label(
                            val.map(|x| format!("{x:.4}"))
                                .unwrap_or_else(|| "-".into()),
                        );
                        ui.label(RichText::new(format!("{rsd:.2}%")).size(11.0).color(BLUE_TEXT));
                        ui.label(RichText::new(format!("{sd_m:.4}")).size(11.0).color(BLUE_TEXT));
                        ui.label(RichText::new(format!("{sd_s:.4}")).size(11.0));
                        ui.label(RichText::new(format!("{sd_e:.4}")).strong().size(11.0).color(BLUE_TEXT));
                        ui.label(RichText::new(v.method.label()).size(10.5));
                        ui.end_row();
                    }
                }
            });
    });
}

fn render_edit_sd_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let ds_idx = state.workbook.active_dataset;
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    if state.sd_table_locked {
        ui.label(RichText::new("🔒 SD Table is Locked").color(Color32::from_rgb(185, 28, 28)));
    }
    egui::ScrollArea::both().max_height(h - 20.0).show(ui, |ui| {
        egui::Grid::new("mb_edit_sd_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream \\ SD(Var)", 130.0);
                for v in &vars {
                    cell_hdr(ui, &format!("{} SD", v.name), 84.0);
                }
                ui.end_row();

                for s in &streams {
                    ui.label(RichText::new(&s.name).strong().size(11.0));
                    for v in &vars {
                        let k = CellKey {
                            frac_idx: state.active_frac_idx,
                            stream_id: s.id,
                            var_id: v.id,
                        };
                        let pre_st = state.workbook.cell_pre_status(ds_idx, k);
                        let (auto_sd, is_ov, mut edit_str) = {
                            let ds = &state.workbook.datasets[ds_idx];
                            let val = ds.values.get(&k).copied();
                            let ov = ds.sd_override.get(&k).copied();
                            let (_, _, _, sd_eff) = evaluate_cell_uncertainty(
                                val,
                                v,
                                s,
                                pre_st,
                                ov,
                                ds.null_sd_estimate,
                                ds.low_grade_coeff,
                            );
                            (sd_eff, ov.is_some(), format!("{sd_eff:.4}"))
                        };
                        let _ = auto_sd;
                        let te = egui::TextEdit::singleline(&mut edit_str)
                            .desired_width(74.0)
                            .text_color(if is_ov { DARK_INK } else { BLUE_TEXT })
                            .interactive(!state.sd_table_locked);
                        let resp = ui.add(te);
                        if resp.clicked() {
                            state.selected_cell = Some(k);
                        }
                        if resp.changed() {
                            if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                if edit_str.trim().is_empty() {
                                    ds.sd_override.remove(&k);
                                } else if let Ok(vnum) = edit_str.trim().replace(',', ".").parse::<f64>() {
                                    ds.sd_override.insert(k, vnum.max(1e-6));
                                }
                            }
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_balance_page(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, h: f32) {
    let ds_idx = state.workbook.active_dataset;
    let sol_opt = state
        .workbook
        .solutions
        .get(ds_idx)
        .and_then(|o| o.as_ref())
        .cloned();

    // Inner Balance subtabs: Data | Details | Goodness | Unit Balance
    ui.horizontal(|ui| {
        for (tab, label) in [
            (BalanceTab::Data, "Data"),
            (BalanceTab::Details, "Details (Diff / ±CI / WSSQ)"),
            (BalanceTab::Goodness, "Goodness"),
            (BalanceTab::UnitBalance, "Unit Balance"),
        ] {
            if ui.selectable_label(state.balance_tab == tab, label).clicked() {
                state.balance_tab = tab;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut state.parity_log_scale, "Log Parity");
        });
    });
    ui.separator();

    let top_h = (h * 0.56).max(180.0);
    let bot_h = (h - top_h - 40.0).max(140.0);

    // Top half: Reconciliation spreadsheet
    match state.balance_tab {
        BalanceTab::Data => render_balance_data_grid(ui, state, sol_opt.as_ref(), top_h),
        BalanceTab::Details => render_balance_details_grid(ui, state, sol_opt.as_ref(), top_h),
        BalanceTab::Goodness => render_balance_goodness_grid(ui, sol_opt.as_ref(), top_h),
        BalanceTab::UnitBalance => render_balance_unit_grid(ui, sol_opt.as_ref(), top_h),
    }

    ui.separator();

    // Bottom half: Balance Convergence (semilog Y) + Parity Chart (`Obs.` vs `Est.`)
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), bot_h),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            let half_w = ((ui.available_width() - 8.0) * 0.5).max(160.0);
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(RichText::new("Balance Convergence").strong().size(11.5));
                render_convergence_plot(
                    ui,
                    &state.workbook.datasets[ds_idx].balance_tolerance,
                    sol_opt.as_ref(),
                    (bot_h - 22.0).max(110.0),
                );
            });
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.set_width(half_w);
                ui.label(RichText::new("Parity Chart (Obs. vs Est.)").strong().size(11.5));
                render_parity_plot(
                    ui,
                    state.parity_log_scale,
                    state.active_frac_idx,
                    sol_opt.as_ref(),
                    (bot_h - 22.0).max(110.0),
                );
            });
        },
    );
}

fn render_balance_data_grid(
    ui: &mut egui::Ui,
    state: &mut MassBalanceWindowState,
    sol_opt: Option<&MbSolution>,
    h: f32,
) {
    let ds_idx = state.workbook.active_dataset;
    let ds = state.workbook.datasets[ds_idx].clone();
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();
    let eqs = state.workbook.stream_equations.clone();

    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        egui::Grid::new("mb_bal_data_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream", 115.0);
                for v in &vars {
                    cell_hdr(ui, &format!("{} ({})\nObs | Bal | Rec%", v.name, v.unit), 156.0);
                }
                ui.end_row();

                for s in &streams {
                    let has_eq = eqs.iter().any(|e| e.target_stream_id == s.id);
                    ui.label(RichText::new(&s.name).strong().size(11.0));
                    for v in &vars {
                        let k = CellKey {
                            frac_idx: state.active_frac_idx,
                            stream_id: s.id,
                            var_id: v.id,
                        };
                        let pre_st = state.workbook.cell_pre_status(ds_idx, k);
                        let cell_opt = sol_opt.and_then(|sol| sol.cells.get(&k));
                        let obs_opt = ds.values.get(&k).copied();
                        let bal_opt = cell_opt.and_then(|c| c.balanced);
                        let rec_opt = cell_opt.and_then(|c| c.recovery_pct);

                        let is_sel = state.selected_cell == Some(k);
                        let fill = if has_eq {
                            GREEN_EQ_BG
                        } else if is_sel {
                            Color32::from_rgb(219, 234, 254)
                        } else {
                            Color32::TRANSPARENT
                        };

                        let resp = ui.allocate_ui(Vec2::new(156.0, 20.0), |ui| {
                            let (rect, click_resp) = ui.allocate_exact_size(
                                Vec2::new(156.0, 20.0),
                                egui::Sense::click(),
                            );
                            let painter = ui.painter();
                            if fill != Color32::TRANSPARENT {
                                painter.rect_filled(rect, Rounding::ZERO, fill);
                            }
                            if ds.show_data_bars {
                                if let Some(rec) = rec_opt {
                                    let frac = (rec / 100.0).clamp(0.0, 1.0) as f32;
                                    let bar_w = (46.0 * frac).max(1.0);
                                    let bar_rect = Rect::from_min_size(
                                        Pos2::new(rect.right() - 48.0, rect.top() + 3.0),
                                        Vec2::new(bar_w, rect.height() - 6.0),
                                    );
                                    painter.rect_filled(
                                        bar_rect,
                                        Rounding::same(1.0),
                                        Color32::from_rgba_unmultiplied(96, 165, 250, 90),
                                    );
                                    painter.rect_stroke(
                                        bar_rect,
                                        Rounding::same(1.0),
                                        Stroke::new(0.6, BAR_FILL),
                                    );
                                }
                            }
                            let obs_txt = obs_opt
                                .map(|x| format!("{x:.2}"))
                                .unwrap_or_else(|| "-".into());
                            let bal_txt = bal_opt
                                .map(|x| format!("{x:.2}"))
                                .unwrap_or_else(|| "-".into());
                            let rec_txt = rec_opt
                                .map(|x| format!("{x:.1}%"))
                                .unwrap_or_else(|| "-".into());
                            let obs_col = if pre_st == PreMeasStatus::Excluded {
                                GRAY_EXCLUDED
                            } else {
                                DARK_INK
                            };
                            painter.text(
                                Pos2::new(rect.left() + 4.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                obs_txt,
                                FontId::proportional(10.5),
                                obs_col,
                            );
                            painter.text(
                                Pos2::new(rect.left() + 56.0, rect.center().y),
                                Align2::LEFT_CENTER,
                                bal_txt,
                                FontId::proportional(10.5),
                                if pre_st == PreMeasStatus::Excluded {
                                    GRAY_EXCLUDED
                                } else {
                                    BLUE_TEXT
                                },
                            );
                            painter.text(
                                Pos2::new(rect.right() - 4.0, rect.center().y),
                                Align2::RIGHT_CENTER,
                                rec_txt,
                                FontId::proportional(10.0),
                                DARK_INK,
                            );
                            click_resp
                        });
                        if resp.inner.clicked() {
                            state.selected_cell = Some(k);
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_balance_details_grid(
    ui: &mut egui::Ui,
    state: &mut MassBalanceWindowState,
    sol_opt: Option<&MbSolution>,
    h: f32,
) {
    let Some(sol) = sol_opt else {
        ui.label("Run Calculate to inspect Balance Details.");
        return;
    };
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        egui::Grid::new("mb_bal_details_grid")
            .striped(true)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                cell_hdr(ui, "Stream", 105.0);
                cell_hdr(ui, "Variable", 85.0);
                cell_hdr(ui, "Meas.", 70.0);
                cell_hdr(ui, "Bal.", 70.0);
                cell_hdr(ui, "±CI", 65.0);
                cell_hdr(ui, "Diff", 68.0);
                cell_hdr(ui, "Rel. Diff %", 75.0);
                cell_hdr(ui, "SD Eff.", 68.0);
                cell_hdr(ui, "Rec. %", 64.0);
                cell_hdr(ui, "Status", 82.0);
                ui.end_row();

                for s in &streams {
                    for v in &vars {
                        let k = CellKey {
                            frac_idx: state.active_frac_idx,
                            stream_id: s.id,
                            var_id: v.id,
                        };
                        let Some(c) = sol.cells.get(&k) else {
                            continue;
                        };
                        if ui
                            .selectable_label(
                                state.selected_cell == Some(k),
                                RichText::new(&s.name).size(11.0),
                            )
                            .clicked()
                        {
                            state.selected_cell = Some(k);
                        }
                        ui.label(RichText::new(&v.name).size(11.0));
                        ui.label(
                            c.measured
                                .map(|x| format!("{x:.3}"))
                                .unwrap_or_else(|| "-".into()),
                        );
                        ui.label(RichText::new(
                            c.balanced
                                .map(|x| format!("{x:.3}"))
                                .unwrap_or_else(|| "-".into()),
                        )
                        .color(BLUE_TEXT));
                        ui.label(RichText::new(
                            c.ci.map(|x| format!("±{x:.3}"))
                                .unwrap_or_else(|| "-".into()),
                        )
                        .color(BLUE_TEXT));
                        ui.label(
                            c.diff
                                .map(|x| format!("{x:.3}"))
                                .unwrap_or_else(|| "-".into()),
                        );
                        let rel_abs = c.rel_diff_pct.unwrap_or(0.0).abs();
                        let bg = if rel_abs > 15.0 {
                            RED_WARN_BG
                        } else if rel_abs > 5.0 {
                            YELLOW_WARN_BG
                        } else {
                            Color32::TRANSPARENT
                        };
                        egui::Frame::none().fill(bg).show(ui, |ui| {
                            ui.label(
                                c.rel_diff_pct
                                    .map(|x| format!("{x:.2}%"))
                                    .unwrap_or_else(|| "-".into()),
                            );
                        });
                        ui.label(
                            c.sd_eff
                                .map(|x| format!("{x:.4}"))
                                .unwrap_or_else(|| "-".into()),
                        );
                        ui.label(
                            c.recovery_pct
                                .map(|x| format!("{x:.2}%"))
                                .unwrap_or_else(|| "-".into()),
                        );
                        ui.label(RichText::new(c.bal_status.label()).size(10.5));
                        ui.end_row();
                    }
                }
            });
    });
}

fn render_balance_goodness_grid(ui: &mut egui::Ui, sol_opt: Option<&MbSolution>, h: f32) {
    let Some(sol) = sol_opt else {
        ui.label("Run Calculate to view Goodness statistics.");
        return;
    };
    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        ui.label(RichText::new("Goodness by Variable").strong().size(11.5));
        egui::Grid::new("mb_good_vars_grid")
            .striped(true)
            .show(ui, |ui| {
                cell_hdr(ui, "Variable", 110.0);
                cell_hdr(ui, "WSSQ", 80.0);
                cell_hdr(ui, "Diff Tot", 80.0);
                cell_hdr(ui, "Rel Diff %", 80.0);
                cell_hdr(ui, "Avg SD", 80.0);
                cell_hdr(ui, "Avg RSD %", 80.0);
                cell_hdr(ui, "RMSD", 80.0);
                ui.end_row();
                for gv in &sol.goodness_vars {
                    ui.label(&gv.name);
                    ui.label(RichText::new(format!("{:.4}", gv.wssq)).color(BLUE_TEXT));
                    ui.label(format!("{:.4}", gv.diff_tot));
                    ui.label(format!("{:.2}%", gv.rel_diff_pct));
                    ui.label(format!("{:.4}", gv.avg_sd));
                    ui.label(format!("{:.2}%", gv.avg_rsd_pct));
                    ui.label(format!("{:.4}", gv.rmsd));
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
        ui.label(RichText::new("Goodness by Stream").strong().size(11.5));
        egui::Grid::new("mb_good_streams_grid")
            .striped(true)
            .show(ui, |ui| {
                cell_hdr(ui, "Stream", 110.0);
                cell_hdr(ui, "Sum WSSQ", 85.0);
                cell_hdr(ui, "Diff Sum", 85.0);
                cell_hdr(ui, "Rel Diff Avg", 90.0);
                cell_hdr(ui, "RMSD", 80.0);
                cell_hdr(ui, "Rel. RMSD", 80.0);
                ui.end_row();
                for gs in &sol.goodness_streams {
                    ui.label(&gs.name);
                    ui.label(RichText::new(format!("{:.4}", gs.sum_wssq)).color(BLUE_TEXT));
                    ui.label(format!("{:.4}", gs.diff_sum));
                    ui.label(format!("{:.2}%", gs.rel_diff_avg));
                    ui.label(format!("{:.4}", gs.rmsd));
                    ui.label(format!("{:.4}", gs.relative_rmsd));
                    ui.end_row();
                }
            });
    });
}

fn render_balance_unit_grid(ui: &mut egui::Ui, sol_opt: Option<&MbSolution>, h: f32) {
    let Some(sol) = sol_opt else {
        ui.label("Run Calculate to view Unit Balance closures.");
        return;
    };
    egui::ScrollArea::both().max_height(h).show(ui, |ui| {
        for ub in &sol.unit_balances {
            ui.label(
                RichText::new(format!("Unit: {}   [{}]", ub.unit_name, ub.equation_text))
                    .strong()
                    .size(11.5),
            );
            egui::Grid::new(("mb_ub_grid", ub.unit_id))
                .striped(true)
                .show(ui, |ui| {
                    cell_hdr(ui, "Component", 100.0);
                    cell_hdr(ui, "Sum Inputs", 90.0);
                    cell_hdr(ui, "Sum Outputs", 90.0);
                    cell_hdr(ui, "Residual (In-Out)", 110.0);
                    cell_hdr(ui, "Rel. Error", 85.0);
                    ui.end_row();
                    for (ci, lbl) in ub.var_labels.iter().enumerate() {
                        let s_in: f64 = ub
                            .inputs_mass
                            .iter()
                            .filter_map(|r| r.get(ci))
                            .sum();
                        let s_out: f64 = ub
                            .outputs_mass
                            .iter()
                            .filter_map(|r| r.get(ci))
                            .sum();
                        let res = ub.residual_balance.get(ci).copied().unwrap_or(0.0);
                        let err = ub.tolerance_error.get(ci).copied().unwrap_or(0.0);
                        ui.label(lbl);
                        ui.label(format!("{s_in:.4}"));
                        ui.label(format!("{s_out:.4}"));
                        ui.label(RichText::new(format!("{res:.2e}")).color(BLUE_TEXT));
                        ui.label(format!("{err:.2e}"));
                        ui.end_row();
                    }
                });
            ui.add_space(6.0);
        }
    });
}

fn render_convergence_plot(
    ui: &mut egui::Ui,
    tol: &f64,
    sol_opt: Option<&MbSolution>,
    height: f32,
) {
    let tol_val = tol.max(1e-12);
    let mut pts = Vec::new();
    if let Some(sol) = sol_opt {
        for &(round, val) in &sol.convergence_rounds {
            pts.push([round as f64, val.max(1e-16).log10()]);
        }
    }
    Plot::new("mb_convergence_plot")
        .height(height)
        .legend(Legend::default())
        .x_axis_label("Iteration")
        .y_axis_label("log10(Iterated Tolerance)")
        .show(ui, |plot_ui| {
            plot_ui.hline(
                HLine::new(tol_val.log10())
                    .name("Tolerance")
                    .color(Color32::from_rgb(220, 38, 38))
                    .style(egui_plot::LineStyle::dashed_dense()),
            );
            if !pts.is_empty() {
                plot_ui.line(
                    Line::new(PlotPoints::from(pts.clone()))
                        .name("Step Δ")
                        .color(Color32::from_rgb(37, 99, 235))
                        .width(2.0),
                );
                plot_ui.points(
                    Points::new(PlotPoints::from(pts))
                        .color(Color32::from_rgb(37, 99, 235))
                        .radius(3.5),
                );
            }
        });
}

fn render_parity_plot(
    ui: &mut egui::Ui,
    log_scale: bool,
    frac_idx: usize,
    sol_opt: Option<&MbSolution>,
    height: f32,
) {
    let mut pts = Vec::new();
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;

    if let Some(sol) = sol_opt {
        for (k, c) in &sol.cells {
            if k.frac_idx != frac_idx || c.pre_status == PreMeasStatus::Excluded {
                continue;
            }
            if let (Some(m), Some(b)) = (c.measured, c.balanced) {
                if log_scale {
                    if m > 1e-9 && b > 1e-9 {
                        let xm = m.log10();
                        let yb = b.log10();
                        min_v = min_v.min(xm).min(yb);
                        max_v = max_v.max(xm).max(yb);
                        pts.push([xm, yb]);
                    }
                } else {
                    min_v = min_v.min(m).min(b);
                    max_v = max_v.max(m).max(b);
                    pts.push([m, b]);
                }
            }
        }
    }

    Plot::new("mb_parity_plot")
        .height(height)
        .legend(Legend::default())
        .x_axis_label(if log_scale { "log10(Measured)" } else { "Measured" })
        .y_axis_label(if log_scale { "log10(Balanced)" } else { "Balanced" })
        .show(ui, |plot_ui| {
            if !pts.is_empty() && min_v.is_finite() && max_v.is_finite() {
                plot_ui.line(
                    Line::new(PlotPoints::from(vec![[min_v, min_v], [max_v, max_v]]))
                        .name("1:1 Parity")
                        .color(Color32::from_rgb(100, 116, 139))
                        .style(egui_plot::LineStyle::dashed_dense()),
                );
                plot_ui.points(
                    Points::new(PlotPoints::from(pts))
                        .name("Cells")
                        .color(Color32::from_rgb(22, 163, 74))
                        .radius(3.8),
                );
            }
        });
}

fn render_reporting_results_page(
    ui: &mut egui::Ui,
    state: &mut MassBalanceWindowState,
    h: f32,
) {
    let ds_idx = state.workbook.active_dataset;
    let Some(sol) = state
        .workbook
        .solutions
        .get(ds_idx)
        .and_then(|o| o.as_ref())
        .cloned()
    else {
        ui.label("No Mass Balance solution yet. Switch to Balancing -> 1. Balance and press Calculate.");
        return;
    };

    ui.horizontal(|ui| {
        for (tab, label) in [
            (ResultsTab::StreamSummary, "Stream Summary"),
            (ResultsTab::SizeDistribution, "Size Distribution"),
            (ResultsTab::PhysicalProperties, "SG & Volumetric Flows"),
        ] {
            if ui.selectable_label(state.results_tab == tab, label).clicked() {
                state.results_tab = tab;
            }
        }
    });
    ui.separator();

    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| {
            s.selected
                && (state.results_filter_mode == 0 || s.source_unit.is_none() || s.dest_unit.is_none())
        })
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    egui::ScrollArea::both().max_height(h - 32.0).show(ui, |ui| {
        match state.results_tab {
            ResultsTab::StreamSummary => {
                egui::Grid::new("mb_res_summary_grid").striped(true).show(ui, |ui| {
                    cell_hdr(ui, "Stream", 120.0);
                    for v in &vars {
                        cell_hdr(ui, &format!("{} ({})", v.name, v.unit), 95.0);
                    }
                    ui.end_row();
                    for s in &streams {
                        ui.label(RichText::new(&s.name).strong().size(11.0));
                        for v in &vars {
                            let k = CellKey {
                                frac_idx: 0,
                                stream_id: s.id,
                                var_id: v.id,
                            };
                            let val = sol
                                .cells
                                .get(&k)
                                .and_then(|c| c.balanced)
                                .map(|b| format!("{b:.3}"))
                                .unwrap_or_else(|| "-".into());
                            ui.label(RichText::new(val).color(BLUE_TEXT).size(11.0));
                        }
                        ui.end_row();
                    }
                });
            }
            ResultsTab::SizeDistribution => {
                let sa_var = vars.iter().find(|v| v.type_code == VarTypeCode::SA).cloned();
                let sf_var = vars.iter().find(|v| v.type_code == VarTypeCode::SF).cloned();
                if let Some(target_v) = sa_var.or(sf_var) {
                    egui::Grid::new("mb_res_psd_grid").striped(true).show(ui, |ui| {
                        cell_hdr(ui, "Size Fraction", 130.0);
                        for s in &streams {
                            cell_hdr(ui, &s.name, 95.0);
                        }
                        ui.end_row();
                        for (fi, fname) in state.workbook.size_fractions.iter().enumerate() {
                            ui.label(RichText::new(fname).strong());
                            for s in &streams {
                                let k = CellKey {
                                    frac_idx: fi,
                                    stream_id: s.id,
                                    var_id: target_v.id,
                                };
                                let txt = sol
                                    .cells
                                    .get(&k)
                                    .and_then(|c| c.balanced)
                                    .map(|b| format!("{b:.3}"))
                                    .unwrap_or_else(|| "-".into());
                                ui.label(RichText::new(txt).color(BLUE_TEXT));
                            }
                            ui.end_row();
                        }
                    });
                } else {
                    ui.label("No Size Fraction (SA) or Solids Flow (SF) variable present.");
                }
            }
            ResultsTab::PhysicalProperties => {
                egui::Grid::new("mb_res_phys_grid").striped(true).show(ui, |ui| {
                    cell_hdr(ui, "Stream", 130.0);
                    cell_hdr(ui, "Type", 105.0);
                    cell_hdr(ui, "Solids Flow", 95.0);
                    cell_hdr(ui, "Water Flow", 95.0);
                    cell_hdr(ui, "Solids SG", 85.0);
                    cell_hdr(ui, "Vol. Flow (m3/h)", 115.0);
                    ui.end_row();
                    let sf_id = vars.iter().find(|v| v.type_code == VarTypeCode::SF).map(|v| v.id);
                    for s in &streams {
                        let f_b = sf_id
                            .and_then(|vid| {
                                sol.cells.get(&CellKey {
                                    frac_idx: 0,
                                    stream_id: s.id,
                                    var_id: vid,
                                })
                            })
                            .and_then(|c| c.balanced)
                            .unwrap_or(0.0);
                        let w_b = sol.solved_water_by_stream.get(&s.id).copied().unwrap_or(0.0);
                        let sg = sol.sg_by_stream.get(&s.id).copied().unwrap_or(3.5);
                        let vol = sol.vol_flow_by_stream.get(&s.id).copied().unwrap_or(0.0);
                        ui.label(RichText::new(&s.name).strong());
                        ui.label(s.stream_type.label());
                        ui.label(RichText::new(format!("{f_b:.3}")).color(BLUE_TEXT));
                        ui.label(RichText::new(format!("{w_b:.3}")).color(BLUE_TEXT));
                        ui.label(format!("{sg:.3}"));
                        ui.label(RichText::new(format!("{vol:.3}")).strong().color(BLUE_TEXT));
                        ui.end_row();
                    }
                });
            }
        }
    });
}

fn render_reporting_charts_page(
    ui: &mut egui::Ui,
    state: &mut MassBalanceWindowState,
    h: f32,
) {
    let ds_idx = state.workbook.active_dataset;
    let sol_opt = state
        .workbook
        .solutions
        .get(ds_idx)
        .and_then(|o| o.as_ref())
        .cloned();
    let streams: Vec<MbStream> = state
        .workbook
        .streams
        .iter()
        .filter(|s| s.selected)
        .cloned()
        .collect();
    let vars: Vec<MbVariable> = state
        .workbook
        .variables
        .iter()
        .filter(|v| v.selected)
        .cloned()
        .collect();

    egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
        if state.show_chart_props_panel {
            egui::Frame::none()
                .fill(ALT_ROW_BG)
                .stroke(Stroke::new(1.0, RIBBON_BORDER))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("Chart Panel Properties").strong());
                    for (pi, panel) in state.workbook.chart_panels.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("#{}:", pi + 1));
                            ui.add(egui::TextEdit::singleline(&mut panel.title).desired_width(140.0));
                            ui.checkbox(&mut panel.show_total_solids, "Solids Flow Bar");
                            ui.checkbox(&mut panel.show_stream_composition, "100% Composition Bar");
                        });
                    }
                });
            ui.add_space(6.0);
        }

        for (pi, panel) in state.workbook.chart_panels.iter().enumerate() {
            egui::Frame::none()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, RIBBON_BORDER))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("Panel {}: {}", pi + 1, panel.title)).strong());
                    if panel.show_total_solids {
                        render_solids_bar_chart(ui, &streams, &vars, sol_opt.as_ref());
                    }
                    if panel.show_stream_composition {
                        render_stacked_comp_chart(ui, &streams, &vars, sol_opt.as_ref());
                    }
                });
            ui.add_space(6.0);
        }
    });
}

fn render_solids_bar_chart(
    ui: &mut egui::Ui,
    streams: &[MbStream],
    vars: &[MbVariable],
    sol_opt: Option<&MbSolution>,
) {
    let Some(sol) = sol_opt else {
        ui.label("Run Calculate to populate Total Solids bar chart.");
        return;
    };
    let Some(sfv) = vars.iter().find(|v| v.type_code == VarTypeCode::SF) else {
        ui.label("No Solids Flow (SF) variable.");
        return;
    };

    let pairs: Vec<(&str, f64)> = streams
        .iter()
        .map(|s| {
            let v = sol
                .cells
                .get(&CellKey {
                    frac_idx: 0,
                    stream_id: s.id,
                    var_id: sfv.id,
                })
                .and_then(|c| c.balanced)
                .unwrap_or(0.0);
            (s.name.as_str(), v.max(0.0))
        })
        .collect();
    let max_val = pairs.iter().map(|p| p.1).fold(1.0, f64::max);
    let w = ui.available_width().max(260.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 165.0), egui::Sense::hover());
    let painter = ui.painter();
    let plot_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 45.0, rect.top() + 12.0),
        Pos2::new(rect.right() - 12.0, rect.bottom() - 28.0),
    );
    painter.rect_stroke(plot_rect, Rounding::ZERO, Stroke::new(1.0, GRID_LINE));

    let n = pairs.len().max(1);
    let slot_w = plot_rect.width() / n as f32;
    let bar_w = (slot_w * 0.58).max(8.0);
    for (i, &(name, val)) in pairs.iter().enumerate() {
        let frac = (val / max_val) as f32;
        let bh = plot_rect.height() * frac;
        let cx = plot_rect.left() + slot_w * (i as f32 + 0.5);
        let r = Rect::from_min_max(
            Pos2::new(cx - bar_w * 0.5, plot_rect.bottom() - bh),
            Pos2::new(cx + bar_w * 0.5, plot_rect.bottom()),
        );
        painter.rect_filled(r, Rounding::same(2.0), CHART_PALETTE[0]);
        painter.text(
            Pos2::new(cx, r.top() - 7.0),
            Align2::CENTER_CENTER,
            format!("{val:.1}"),
            FontId::proportional(9.5),
            DARK_INK,
        );
        painter.text(
            Pos2::new(cx, plot_rect.bottom() + 12.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(9.5),
            DARK_INK,
        );
    }
}

fn render_stacked_comp_chart(
    ui: &mut egui::Ui,
    streams: &[MbStream],
    vars: &[MbVariable],
    sol_opt: Option<&MbSolution>,
) {
    let Some(sol) = sol_opt else {
        ui.label("Run Calculate to populate Stream Composition chart.");
        return;
    };
    let min_vars: Vec<&MbVariable> = vars.iter().filter(|v| v.type_code == VarTypeCode::M).collect();
    let assays: Vec<&MbVariable> = if !min_vars.is_empty() {
        min_vars
    } else {
        vars.iter().filter(|v| v.type_code == VarTypeCode::A).collect()
    };
    if assays.is_empty() {
        ui.label("No Mineral or Elemental Assay variables selected.");
        return;
    }

    ui.horizontal_wrapped(|ui| {
        for (i, v) in assays.iter().enumerate() {
            let col = CHART_PALETTE[i % CHART_PALETTE.len()];
            let (sq, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(sq, Rounding::same(2.0), col);
            ui.label(RichText::new(&v.name).size(10.5));
        }
    });

    let row_h = 24.0;
    let total_h = (streams.len() as f32 * row_h + 12.0).max(60.0);
    let w = ui.available_width().max(260.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, total_h), egui::Sense::hover());
    let painter = ui.painter();
    for (si, s) in streams.iter().enumerate() {
        let y = rect.top() + 4.0 + si as f32 * row_h;
        painter.text(
            Pos2::new(rect.left() + 4.0, y + 9.0),
            Align2::LEFT_CENTER,
            &s.name,
            FontId::proportional(10.5),
            DARK_INK,
        );
        let vals: Vec<f64> = assays
            .iter()
            .map(|v| {
                sol.cells
                    .get(&CellKey {
                        frac_idx: 0,
                        stream_id: s.id,
                        var_id: v.id,
                    })
                    .and_then(|c| c.balanced)
                    .unwrap_or(0.0)
                    .max(0.0)
            })
            .collect();
        let tot: f64 = vals.iter().sum::<f64>().max(1e-9);
        let bar_x0 = rect.left() + 105.0;
        let bar_w = (rect.right() - bar_x0 - 12.0).max(80.0);
        let mut x_acc = bar_x0;
        for (vi, &v) in vals.iter().enumerate() {
            let seg_w = bar_w * (v / tot) as f32;
            if seg_w > 0.5 {
                let seg_r = Rect::from_min_size(Pos2::new(x_acc, y + 2.0), Vec2::new(seg_w, 16.0));
                painter.rect_filled(seg_r, Rounding::ZERO, CHART_PALETTE[vi % CHART_PALETTE.len()]);
                if seg_w > 26.0 {
                    painter.text(
                        seg_r.center(),
                        Align2::CENTER_CENTER,
                        format!("{:.1}%", 100.0 * v / tot),
                        FontId::proportional(9.0),
                        Color32::WHITE,
                    );
                }
                x_acc += seg_w;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Zone 4: Right Properties Inspector (`Fig. 15–16`)
// ---------------------------------------------------------------------------

fn prop_row(ui: &mut egui::Ui, label: &str, value: &str, blue: bool) {
    ui.label(RichText::new(label).size(10.5).color(Color32::from_rgb(71, 85, 105)));
    ui.label(
        RichText::new(value)
            .size(10.5)
            .color(if blue { BLUE_TEXT } else { DARK_INK }),
    );
    ui.end_row();
}

fn render_properties_panel(ui: &mut egui::Ui, state: &mut MassBalanceWindowState, body_h: f32) {
    if !state.props_open {
        egui::Frame::none()
            .fill(NAV_BG)
            .stroke(Stroke::new(1.0, RIBBON_BORDER))
            .show(ui, |ui| {
                ui.set_width(26.0);
                ui.set_height(body_h);
                if ui.button("◀").on_hover_text("Expand Properties").clicked() {
                    state.props_open = true;
                }
            });
        return;
    }

    let ds_idx = state.workbook.active_dataset;
    egui::Frame::none()
        .fill(NAV_BG)
        .stroke(Stroke::new(1.0, RIBBON_BORDER))
        .rounding(2.0)
        .show(ui, |ui| {
            ui.set_width(276.0);
            ui.set_height(body_h);
            ui.horizontal(|ui| {
                ui.label(RichText::new("  Properties").strong().size(11.5).color(DARK_INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("▶").on_hover_text("Collapse Properties").clicked() {
                        state.props_open = false;
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("mb_right_props_scroll")
                .show(ui, |ui| {
                    // Context object properties if on setup tabs
                    match state.active_page {
                        MbNavPage::Units => {
                            if let Some(uid) = state.selected_unit_id {
                                if let Some(u) = state.workbook.units.iter_mut().find(|u| u.id == uid) {
                                    ui.label(RichText::new("Selected Unit").strong().size(11.0));
                                    egui::Grid::new("prop_u_g").num_columns(2).show(ui, |ui| {
                                        prop_row(ui, "Unit Name:", &u.name, false);
                                        ui.label("Selected:");
                                        ui.checkbox(&mut u.selected, "");
                                        ui.end_row();
                                        ui.label("Size Transforming:");
                                        ui.checkbox(&mut u.size_transforming, "");
                                        ui.end_row();
                                    });
                                }
                            }
                            return;
                        }
                        MbNavPage::Variables => {
                            if let Some(vid) = state.selected_var_id {
                                if let Some(v) =
                                    state.workbook.variables.iter_mut().find(|v| v.id == vid)
                                {
                                    ui.label(RichText::new("Selected Variable").strong().size(11.0));
                                    egui::Grid::new("prop_v_g").num_columns(2).show(ui, |ui| {
                                        prop_row(ui, "Name:", &v.name, false);
                                        prop_row(ui, "Type:", v.type_code.type_name(), false);
                                        ui.label("Mineral SG:");
                                        let mut sg = v.mineral_sg.unwrap_or_else(|| default_mineral_sg(&v.name));
                                        if ui.add(egui::DragValue::new(&mut sg).speed(0.01)).changed() {
                                            v.mineral_sg = Some(sg);
                                        }
                                        ui.end_row();
                                    });
                                }
                            }
                            return;
                        }
                        _ => {}
                    }

                    // Selected Cell inspector (`Data`, `Values`, `Balanced Values` from Fig. 15)
                    if let Some(k) = state.selected_cell {
                        let s_name = state
                            .workbook
                            .streams
                            .iter()
                            .find(|s| s.id == k.stream_id)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "?".into());
                        let v_opt = state.workbook.variables.iter().find(|v| v.id == k.var_id).cloned();
                        let s_opt = state.workbook.streams.iter().find(|s| s.id == k.stream_id).cloned();
                        let f_name = state
                            .workbook
                            .size_fractions
                            .get(k.frac_idx)
                            .cloned()
                            .unwrap_or_else(|| "Bulk".into());
                        let pre_st = state.workbook.cell_pre_status(ds_idx, k);
                        let sol_cell = state
                            .workbook
                            .solutions
                            .get(ds_idx)
                            .and_then(|o| o.as_ref())
                            .and_then(|s| s.cells.get(&k).cloned());

                        egui::CollapsingHeader::new("Data (Selected Cell)")
                            .default_open(state.prop_sel_data_open)
                            .show(ui, |ui| {
                                egui::Grid::new("prop_sel_data_g").num_columns(2).show(ui, |ui| {
                                    prop_row(ui, "Stream:", &s_name, false);
                                    prop_row(
                                        ui,
                                        "Variable:",
                                        v_opt.as_ref().map(|v| v.name.as_str()).unwrap_or("?"),
                                        false,
                                    );
                                    prop_row(ui, "Fraction:", &f_name, false);
                                    ui.label(RichText::new(crate::i18n::text("common.101")).size(10.5));
                                    let mut next_st = pre_st;
                                    egui::ComboBox::from_id_salt("prop_cell_st")
                                        .width(120.0)
                                        .selected_text(pre_st.label())
                                        .show_ui(ui, |ui| {
                                            for st in [
                                                PreMeasStatus::Measured,
                                                PreMeasStatus::Fixed,
                                                PreMeasStatus::Guesstimated,
                                                PreMeasStatus::Missing,
                                                PreMeasStatus::Excluded,
                                            ] {
                                                ui.selectable_value(&mut next_st, st, st.label());
                                            }
                                        });
                                    if next_st != pre_st {
                                        if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                            ds.meas_status_override.insert(k, next_st);
                                        }
                                    }
                                    ui.end_row();
                                });
                            });

                        egui::CollapsingHeader::new("Values")
                            .default_open(state.prop_sel_vals_open)
                            .show(ui, |ui| {
                                if let (Some(v), Some(s)) = (&v_opt, &s_opt) {
                                    let ds = &state.workbook.datasets[ds_idx];
                                    let val = ds.values.get(&k).copied();
                                    let ov = ds.sd_override.get(&k).copied();
                                    let (sd_m, rsd, sd_s, sd_e) = evaluate_cell_uncertainty(
                                        val,
                                        v,
                                        s,
                                        pre_st,
                                        ov,
                                        ds.null_sd_estimate,
                                        ds.low_grade_coeff,
                                    );
                                    egui::Grid::new("prop_sel_vals_g").num_columns(2).show(ui, |ui| {
                                        prop_row(
                                            ui,
                                            "Measured:",
                                            &val.map(|x| format!("{x:.4}")).unwrap_or_else(|| "-".into()),
                                            false,
                                        );
                                        prop_row(ui, "RSD:", &format!("{rsd:.2} %"), true);
                                        prop_row(ui, "SD Meas.:", &format!("{sd_m:.4}"), true);
                                        prop_row(ui, "SD Sampling:", &format!("{sd_s:.4}"), false);
                                        prop_row(ui, "SD Eff.:", &format!("{sd_e:.4}"), true);
                                    });
                                }
                            });

                        egui::CollapsingHeader::new("Balanced Values")
                            .default_open(state.prop_sel_bal_open)
                            .show(ui, |ui| {
                                if let Some(c) = &sol_cell {
                                    egui::Grid::new("prop_sel_bal_g").num_columns(2).show(ui, |ui| {
                                        prop_row(
                                            ui,
                                            "Balanced:",
                                            &c.balanced.map(|x| format!("{x:.4}")).unwrap_or_else(|| "-".into()),
                                            true,
                                        );
                                        prop_row(
                                            ui,
                                            "CI (±):",
                                            &c.ci.map(|x| format!("{x:.4}")).unwrap_or_else(|| "-".into()),
                                            true,
                                        );
                                        prop_row(
                                            ui,
                                            "Diff:",
                                            &c.diff.map(|x| format!("{x:.4}")).unwrap_or_else(|| "-".into()),
                                            false,
                                        );
                                        prop_row(
                                            ui,
                                            "Rel. Diff:",
                                            &c.rel_diff_pct
                                                .map(|x| format!("{x:.2} %"))
                                                .unwrap_or_else(|| "-".into()),
                                            false,
                                        );
                                        prop_row(
                                            ui,
                                            "Recovery:",
                                            &c.recovery_pct
                                                .map(|x| format!("{x:.2} %"))
                                                .unwrap_or_else(|| "-".into()),
                                            false,
                                        );
                                        prop_row(ui, "Status:", c.bal_status.label(), false);
                                    });
                                } else {
                                    ui.label(RichText::new("Not calculated yet").small());
                                }
                            });
                    }

                    ui.separator();

                    // Calculation Settings (`Fig. 15`)
                    let stream_list: Vec<(usize, String)> = state
                        .workbook
                        .streams
                        .iter()
                        .filter(|s| s.selected)
                        .map(|s| (s.id, s.name.clone()))
                        .collect();

                    egui::CollapsingHeader::new("Calculation Settings")
                        .default_open(state.prop_calc_settings_open)
                        .show(ui, |ui| {
                            if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                egui::Grid::new("prop_calc_set_g").num_columns(2).show(ui, |ui| {
                                    ui.label(RichText::new("Feed Stream:").size(10.5));
                                    let cur_f = ds
                                        .feed_stream_id
                                        .and_then(|id| stream_list.iter().find(|x| x.0 == id).map(|x| x.1.clone()))
                                        .unwrap_or_else(|| "Auto".into());
                                    egui::ComboBox::from_id_salt("prop_feed_stream")
                                        .width(120.0)
                                        .selected_text(cur_f)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut ds.feed_stream_id, None, "Auto");
                                            for (sid, sname) in &stream_list {
                                                ui.selectable_value(&mut ds.feed_stream_id, Some(*sid), sname);
                                            }
                                        });
                                    ui.end_row();
                                    ui.label(RichText::new("Scaling:").size(10.5));
                                    ui.checkbox(&mut ds.scaling, "Matlab Fig. 17–18");
                                    ui.end_row();
                                    ui.label(RichText::new("Mineral Sum=100:").size(10.5));
                                    ui.checkbox(&mut ds.mineral_sum_100, "");
                                    ui.end_row();
                                    ui.label(RichText::new("Assay Sum=100:").size(10.5));
                                    ui.checkbox(&mut ds.assay_sum_100, "");
                                    ui.end_row();
                                    prop_row(ui, "Balance Type:", ds.balance_type.label(), false);
                                    prop_row(ui, "Method:", ds.method.label(), false);
                                    prop_row(ui, "Sizing:", ds.sizing.label(), false);
                                });
                            }
                        });

                    // Algorithm Parameters (`Fig. 15`)
                    egui::CollapsingHeader::new("Algorithm Parameters")
                        .default_open(state.prop_algo_params_open)
                        .show(ui, |ui| {
                            if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                                egui::Grid::new("prop_algo_p_g").num_columns(2).show(ui, |ui| {
                                    ui.label(RichText::new("Max Iter:").size(10.5));
                                    ui.add(egui::DragValue::new(&mut ds.max_iter).range(1..=10000));
                                    ui.end_row();
                                    ui.label(RichText::new("Balance Tol.:").size(10.5));
                                    ui.add(
                                        egui::DragValue::new(&mut ds.balance_tolerance)
                                            .speed(0.0001)
                                            .range(1e-8..=1.0),
                                    );
                                    ui.end_row();
                                    ui.label(RichText::new("Null SD Est.:").size(10.5));
                                    ui.add(
                                        egui::DragValue::new(&mut ds.null_sd_estimate)
                                            .speed(0.0005)
                                            .range(1e-6..=1.0),
                                    );
                                    ui.end_row();
                                    ui.label(RichText::new("Low Grade Coeff.:").size(10.5));
                                    ui.add(
                                        egui::DragValue::new(&mut ds.low_grade_coeff)
                                            .speed(0.02)
                                            .range(0.05..=2.0),
                                    );
                                    ui.end_row();
                                });
                            }
                        });

                    // Balancing Summary (`Fig. 15`)
                    egui::CollapsingHeader::new("Balancing Summary")
                        .default_open(state.prop_summary_open)
                        .show(ui, |ui| {
                            if let Some(sol) = state
                                .workbook
                                .solutions
                                .get(ds_idx)
                                .and_then(|o| o.as_ref())
                            {
                                egui::Grid::new("prop_bal_sum_g").num_columns(2).show(ui, |ui| {
                                    prop_row(ui, "Number of Meas.:", &sol.num_meas.to_string(), true);
                                    prop_row(ui, "Number of Bal.:", &sol.num_bal.to_string(), true);
                                    prop_row(ui, "Number of zeros:", &sol.num_zeros.to_string(), true);
                                    prop_row(ui, "Number of negatives:", &sol.num_negatives.to_string(), true);
                                    prop_row(ui, "WSSQ:", &format!("{:.4}", sol.wssq), true);
                                    prop_row(ui, "RMSD:", &format!("{:.4}", sol.rmsd), true);
                                    prop_row(
                                        ui,
                                        "Relative RMSD:",
                                        &format!("{:.4} %", sol.rel_rmsd * 100.0),
                                        true,
                                    );
                                });
                            } else {
                                ui.label(RichText::new("No summary yet (press Calculate)").small());
                            }
                        });
                });
        });
}

// ---------------------------------------------------------------------------
// Modals & Popovers (`Low Grade Weighting`, `Balance Equations`, `Error Check`,
// `Import HSC7`, `Stream by Equation`, `Mineral Matrix`, `Element Picker`,
// `Enter Recoveries`, `Paste TSV`, `Backup .HSCMas`)
// ---------------------------------------------------------------------------

fn render_mb_modals(ctx: &egui::Context, state: &mut MassBalanceWindowState) {
    // 1. Low Grade Weighting Popover
    if state.show_low_grade_popover {
        let mut open = state.show_low_grade_popover;
        let ds_idx = state.workbook.active_dataset;
        let mut trigger_calc = false;
        egui::Window::new("Low Grade Weighting")
            .open(&mut open)
            .default_width(290.0)
            .show(ctx, |ui| {
                if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                    ui.horizontal(|ui| {
                        ui.label("Coefficient (SD multiplier factor):");
                        ui.add(
                            egui::DragValue::new(&mut ds.low_grade_coeff)
                                .speed(0.02)
                                .range(0.05..=2.0),
                        );
                    });
                }
                ui.separator();
                ui.label(RichText::new("Apply to Variables:").strong());
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for v in &mut state.workbook.variables {
                        if v.selected && v.type_code.is_assay() {
                            ui.checkbox(&mut v.low_grade_weighted, format!("{} ({})", v.name, v.unit));
                        }
                    }
                });
                ui.separator();
                if ui.button("Apply Coefficient & Recalculate").clicked() {
                    trigger_calc = true;
                }
            });
        state.show_low_grade_popover = open;
        if trigger_calc {
            state.run_calculate_active();
        }
    }

    // 2. Mass Balance Equations Modal
    if state.show_equations_modal {
        let mut open = state.show_equations_modal;
        egui::Window::new("Mass Balance Equations")
            .open(&mut open)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.checkbox(
                    &mut state.eq_only_non_sru,
                    "Only Non-Size Transforming Units (Sieved PSD conservation)",
                );
                ui.separator();
                let eqs = state.workbook.build_equations_text(state.eq_only_non_sru);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    if eqs.is_empty() {
                        ui.label("No active unit mass balance equations.");
                    } else {
                        for line in &eqs {
                            ui.label(RichText::new(line).monospace().size(11.5));
                        }
                    }
                });
            });
        state.show_equations_modal = open;
    }

    // 3. Check for Errors Modal (all 12 Chapter 51.6 rules)
    if state.show_error_check_modal {
        let mut open = state.show_error_check_modal;
        egui::Window::new("Mass Balance Error Check")
            .open(&mut open)
            .default_width(540.0)
            .show(ctx, |ui| {
                if state.error_check_cache.is_empty() {
                    ui.label(
                        RichText::new("✔ Everything seems to be OK!")
                            .strong()
                            .size(14.0)
                            .color(Color32::from_rgb(22, 163, 74)),
                    );
                    ui.label("All 12 Chapter 51.6 pre-balance validation rules passed.");
                } else {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for iss in &state.error_check_cache {
                            let (badge, col) = match iss.severity {
                                CheckSeverity::Error => ("[ERROR]", Color32::from_rgb(185, 28, 28)),
                                CheckSeverity::Notification => ("[NOTICE]", Color32::from_rgb(180, 83, 9)),
                            };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{badge} #{}", iss.code)).strong().color(col));
                                ui.label(RichText::new(&iss.title).strong());
                            });
                            ui.label(RichText::new(&iss.detail).small());
                            ui.separator();
                        }
                    });
                }
            });
        state.show_error_check_modal = open;
    }

    // 4. Import HSC7 Data Modal (Figs. 24–26)
    if state.show_import_hsc7_modal {
        let mut open = state.show_import_hsc7_modal;
        let mut close_now = false;
        egui::Window::new("Import HSC7 Mass Balance Data")
            .open(&mut open)
            .default_width(580.0)
            .show(ctx, |ui| {
                ui.label("Paste HSC7 'Flowsheet' incidence matrix (+1/-1/0, optional) and 'Analyses.xls' data:");
                ui.checkbox(&mut state.import_hsc7_horizontal, "Horizontal Layout (streams in rows)");
                ui.label(RichText::new("Flowsheet Matrix (optional):").strong());
                ui.add(
                    egui::TextEdit::multiline(&mut state.import_hsc7_flowsheet_buf)
                        .desired_rows(4)
                        .desired_width(540.0)
                        .code_editor(),
                );
                ui.label(RichText::new("Analyses Data (TSV/CSV):").strong());
                ui.add(
                    egui::TextEdit::multiline(&mut state.import_hsc7_data_buf)
                        .desired_rows(6)
                        .desired_width(540.0)
                        .code_editor(),
                );
                if ui.button("Import & Parse HSC7 Data").clicked() {
                    let fs_opt = if state.import_hsc7_flowsheet_buf.trim().is_empty() {
                        None
                    } else {
                        Some(state.import_hsc7_flowsheet_buf.as_str())
                    };
                    match SoryntelApplication::mb_import_hsc7(
                        &mut state.workbook,
                        fs_opt,
                        &state.import_hsc7_data_buf,
                        state.import_hsc7_horizontal,
                    ) {
                        Ok(n) => {
                            state.selected_unit_id = state.workbook.units.first().map(|u| u.id);
                            state.selected_stream_id = state.workbook.streams.first().map(|s| s.id);
                            state.selected_var_id = state.workbook.variables.first().map(|v| v.id);
                            state.status_banner = format!("Imported {n} HSC7 measurement cell(s)");
                            close_now = true;
                        }
                        Err(e) => {
                            state.status_banner = format!("HSC7 import failed: {e}");
                        }
                    }
                }
            });
        state.show_import_hsc7_modal = open && !close_now;
    }

    // 5. Stream by Equation Modal
    if state.show_stream_eq_modal {
        let mut open = state.show_stream_eq_modal;
        let streams = state.workbook.streams.clone();
        egui::Window::new("Stream by Equation (Fig. 12)")
            .open(&mut open)
            .default_width(420.0)
            .show(ctx, |ui| {
                if streams.is_empty() {
                    ui.label("No streams defined.");
                } else {
                    state.eq_target_stream_idx = state.eq_target_stream_idx.min(streams.len() - 1);
                    state.eq_source_stream_idx = state.eq_source_stream_idx.min(streams.len() - 1);
                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        egui::ComboBox::from_id_salt("mb_eq_tgt")
                            .selected_text(&streams[state.eq_target_stream_idx].name)
                            .show_ui(ui, |ui| {
                                for (i, s) in streams.iter().enumerate() {
                                    ui.selectable_value(&mut state.eq_target_stream_idx, i, &s.name);
                                }
                            });
                        ui.label(" = ");
                        ui.add(egui::DragValue::new(&mut state.eq_factor_input).speed(0.05));
                        ui.label(" * ");
                        egui::ComboBox::from_id_salt("mb_eq_src")
                            .selected_text(&streams[state.eq_source_stream_idx].name)
                            .show_ui(ui, |ui| {
                                for (i, s) in streams.iter().enumerate() {
                                    ui.selectable_value(&mut state.eq_source_stream_idx, i, &s.name);
                                }
                            });
                    });
                    if ui.button("Add Stream Equation").clicked() {
                        let tgt = streams[state.eq_target_stream_idx].id;
                        let src = streams[state.eq_source_stream_idx].id;
                        state
                            .workbook
                            .stream_equations
                            .retain(|e| e.target_stream_id != tgt);
                        state.workbook.stream_equations.push(StreamEquation {
                            target_stream_id: tgt,
                            source_stream_id: src,
                            factor: state.eq_factor_input,
                        });
                    }
                    ui.separator();
                    let mut remove_idx = None;
                    for (i, eq) in state.workbook.stream_equations.iter().enumerate() {
                        let tname = streams
                            .iter()
                            .find(|s| s.id == eq.target_stream_id)
                            .map(|s| s.name.as_str())
                            .unwrap_or("?");
                        let sname = streams
                            .iter()
                            .find(|s| s.id == eq.source_stream_id)
                            .map(|s| s.name.as_str())
                            .unwrap_or("?");
                        ui.horizontal(|ui| {
                            ui.label(format!("{tname} = {} * {sname}", eq.factor));
                            if ui.button(crate::i18n::text("common.013")).clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    }
                    if let Some(ri) = remove_idx {
                        state.workbook.stream_equations.remove(ri);
                    }
                }
            });
        state.show_stream_eq_modal = open;
    }

    // 6. Mineral Matrix Viewer (`MINERALS_RAW` from HSC Geo catalog)
    if state.show_mineral_matrix_modal {
        let mut open = state.show_mineral_matrix_modal;
        egui::Window::new("View Mineral Matrix (HSC Geo Catalog)")
            .open(&mut open)
            .default_width(540.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search Mineral:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.mineral_search_query)
                            .desired_width(190.0),
                    );
                });
                ui.separator();
                let matches = search_minerals_catalog(&state.mineral_search_query, 35);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    egui::Grid::new("mb_mineral_cat_grid").striped(true).show(ui, |ui| {
                        cell_hdr(ui, "Code", 75.0);
                        cell_hdr(ui, "Mineral", 150.0);
                        cell_hdr(ui, "Formula", 150.0);
                        cell_hdr(ui, "Action", 90.0);
                        ui.end_row();
                        for m in matches {
                            ui.label(m.code);
                            ui.label(m.name);
                            ui.label(RichText::new(m.formula).color(BLUE_TEXT));
                            if ui.button("+ Add Var").clicked() {
                                let id = state.workbook.next_id;
                                state.workbook.next_id += 1;
                                state.workbook.variables.push(MbVariable {
                                    id,
                                    name: if m.code.is_empty() {
                                        m.name.to_string()
                                    } else {
                                        m.code.to_string()
                                    },
                                    unit: "%".into(),
                                    type_code: VarTypeCode::M,
                                    selected: true,
                                    method: MeasurementMethod::SShape,
                                    param_a: 5.0,
                                    param_b: 0.01,
                                    param_c: 5.0,
                                    low_grade_weighted: false,
                                    mineral_formula: Some(m.formula.to_string()),
                                    mineral_sg: Some(default_mineral_sg(m.name)),
                                });
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        state.show_mineral_matrix_modal = open;
    }

    // 7. Quick Element Picker
    if state.show_element_picker_modal {
        let mut open = state.show_element_picker_modal;
        egui::Window::new("Add Assay Elements")
            .open(&mut open)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for el in [
                        "Cu", "Fe", "S", "Zn", "Pb", "Si", "Al", "Ca", "Mg", "As", "Ag", "Au",
                        "Ni", "Co", "Mo", "Mn",
                    ] {
                        let exists = state
                            .workbook
                            .variables
                            .iter()
                            .any(|v| v.name.eq_ignore_ascii_case(el));
                        if ui.selectable_label(exists, el).clicked() && !exists {
                            let id = state.workbook.next_id;
                            state.workbook.next_id += 1;
                            state.workbook.variables.push(MbVariable {
                                id,
                                name: el.to_string(),
                                unit: "%".into(),
                                type_code: VarTypeCode::A,
                                selected: true,
                                method: MeasurementMethod::FixedRelative,
                                param_a: 5.0,
                                param_b: 0.0,
                                param_c: 0.0,
                                low_grade_weighted: false,
                                mineral_formula: None,
                                mineral_sg: None,
                            });
                        }
                    }
                });
            });
        state.show_element_picker_modal = open;
    }

    // 8. Enter Recoveries Modal (`Enter Recoveries`)
    if state.show_recoveries_modal {
        let mut open = state.show_recoveries_modal;
        let mut close_now = false;
        egui::Window::new("Enter Recoveries for Model Fit")
            .open(&mut open)
            .default_width(420.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    egui::Grid::new("mb_enter_rec_grid").striped(true).show(ui, |ui| {
                        cell_hdr(ui, "Stream", 120.0);
                        cell_hdr(ui, "Assay", 90.0);
                        cell_hdr(ui, "Recovery (%)", 110.0);
                        ui.end_row();
                        for row in &mut state.recovery_drafts {
                            ui.label(&row.stream_name);
                            ui.label(&row.var_name);
                            ui.add(
                                egui::DragValue::new(&mut row.rec_pct)
                                    .speed(0.1)
                                    .range(0.0..=100.0),
                            );
                            ui.end_row();
                        }
                    });
                });
                ui.separator();
                if ui.button("Apply Recoveries to Active Dataset").clicked() {
                    let ds_idx = state.workbook.active_dataset;
                    if let Some(ds) = state.workbook.datasets.get_mut(ds_idx) {
                        for row in &state.recovery_drafts {
                            ds.recovered_for_model_fit
                                .insert((row.stream_id, row.var_id), row.rec_pct);
                        }
                        state.status_banner = format!(
                            "Stored {} model-fit recovery value(s)",
                            state.recovery_drafts.len()
                        );
                    }
                    close_now = true;
                }
            });
        state.show_recoveries_modal = open && !close_now;
    }

    // 9. Text / Paste / Backup Modal
    if let Some(kind) = state.show_text_modal {
        let mut open = true;
        let title = state.text_modal_title.clone();
        egui::Window::new(title)
            .open(&mut open)
            .default_width(580.0)
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.text_modal_buf)
                        .desired_rows(12)
                        .desired_width(545.0)
                        .code_editor(),
                );
                ui.horizontal(|ui| {
                    match kind {
                        TextModalKind::PasteExperimental | TextModalKind::ImportRex => {
                            if ui.button("Paste into Active Size Fraction").clicked() {
                                let n = state
                                    .workbook
                                    .paste_experimental_tsv(&state.text_modal_buf, state.active_frac_idx);
                                state.status_banner = format!("Smart-pasted {n} cell(s)");
                                state.show_text_modal = None;
                            }
                        }
                        TextModalKind::PasteSdRsd => {
                            if ui.button("Apply SD Overrides").clicked() {
                                let n = paste_sd_tsv(&mut state.workbook, &state.text_modal_buf, state.active_frac_idx);
                                state.status_banner = format!("Applied {n} SD override(s)");
                                state.show_text_modal = None;
                            }
                        }
                        TextModalKind::BackupJson => {
                            if ui.button("Load JSON as Active Workbook").clicked() {
                                match serde_json::from_str::<MassBalanceWorkbook>(&state.text_modal_buf) {
                                    Ok(wb) => {
                                        state.workbook = wb;
                                        state.status_banner = "Restored .HSCMas JSON backup".into();
                                        state.show_text_modal = None;
                                    }
                                    Err(e) => {
                                        state.status_banner = format!("Invalid .HSCMas JSON: {e}");
                                    }
                                }
                            }
                        }
                        TextModalKind::ExportPreview => {
                            if ui.button(crate::i18n::text("common.100")).clicked() {
                                ctx.copy_text(state.text_modal_buf.clone());
                            }
                        }
                    }
                });
            });
        if !open {
            state.show_text_modal = None;
        }
    }

    // 10. Chapter 51 Help Modal
    if state.show_help_modal {
        let mut open = state.show_help_modal;
        egui::Window::new("HSC Mass Balance · Chapter 51 Reference")
            .open(&mut open)
            .default_width(470.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Two-Stage Reconciliation Engine (Eqs. 1–29)").strong());
                ui.label("• Stage 1: Iterative EWTLS (Total Solids, Water + %Solids, Sieve PSD fractions).");
                ui.label("• Stage 2: Weighted Least Squares (LS, LS MinMax, NNLS, CLS, LLS) with optional Sum = 100.");
                ui.separator();
                ui.label(RichText::new("Semantic Color Legend").strong());
                ui.label("• Black: Measured values");
                ui.label(RichText::new("• Blue: Estimated / Auto-SD / Balanced / ±CI").color(BLUE_TEXT));
                ui.label(RichText::new("• Gray: Excluded assays (Balancing of Minerals ON)").color(GRAY_EXCLUDED));
                ui.label("• Green Cell: Stream by Equation target stream");
            });
        state.show_help_modal = open;
    }
}

fn build_measurement_tsv(wb: &MassBalanceWorkbook, frac_idx: usize) -> String {
    let vars: Vec<&MbVariable> = wb.variables.iter().filter(|v| v.selected).collect();
    let streams: Vec<&MbStream> = wb.streams.iter().filter(|s| s.selected).collect();
    let Some(ds) = wb.active_dataset() else {
        return String::new();
    };
    let mut out = String::from("Stream");
    for v in &vars {
        out.push('\t');
        out.push_str(&v.name);
    }
    for s in &streams {
        out.push('\n');
        out.push_str(&s.name);
        for v in &vars {
            out.push('\t');
            if let Some(val) = ds.values.get(&CellKey {
                frac_idx,
                stream_id: s.id,
                var_id: v.id,
            }) {
                out.push_str(&format!("{val}"));
            }
        }
    }
    out
}

fn paste_sd_tsv(wb: &mut MassBalanceWorkbook, text: &str, frac_idx: usize) -> usize {
    let lines: Vec<Vec<String>> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect())
        .collect();
    if lines.len() < 2 {
        return 0;
    }
    let streams = wb.streams.clone();
    let vars = wb.variables.clone();
    let Some(ds) = wb.active_dataset_mut() else {
        return 0;
    };
    let col_vids: Vec<Option<usize>> = lines[0]
        .iter()
        .map(|h| {
            vars.iter()
                .find(|v| v.name.eq_ignore_ascii_case(h))
                .map(|v| v.id)
        })
        .collect();
    let mut n = 0usize;
    for r in &lines[1..] {
        let Some(s) = streams.iter().find(|s| s.name.eq_ignore_ascii_case(&r[0])) else {
            continue;
        };
        for (ci, c) in r.iter().enumerate().skip(1) {
            if let (Some(vid), Ok(val)) = (
                col_vids.get(ci).copied().flatten(),
                c.replace(',', ".").parse::<f64>(),
            ) {
                ds.sd_override.insert(
                    CellKey {
                        frac_idx,
                        stream_id: s.id,
                        var_id: vid,
                    },
                    val.max(1e-6),
                );
                n += 1;
            }
        }
    }
    n
}
