pub mod canvas;
pub mod docks;
pub mod editors;
pub mod icons;
pub mod state;

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::model::*;
use soryntel_thermo::sim::presets::VISUALIZATION_METRICS;

pub use state::SimModuleState;
use state::*;

fn tb_icon_btn(ui: &mut egui::Ui, glyph: &str, active: bool, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::click());
    if active || resp.hovered() {
        ui.painter().rect_filled(
            rect,
            2.0,
            if active {
                Color32::from_rgb(206, 224, 248)
            } else {
                Color32::from_rgb(225, 232, 242)
            },
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(0.8, if active { SIM_ACCENT_BLUE } else { SIM_BORDER }),
        );
    }
    icons::draw_toolbar_glyph(ui.painter(), rect, glyph, active);
    resp.on_hover_text(tip)
}

fn tb_text_btn(ui: &mut egui::Ui, label: &str, active: bool, tip: &str) -> egui::Response {
    let mut txt = RichText::new(label).size(11.5);
    if active {
        txt = txt.strong().color(SIM_ACCENT_BLUE);
    } else {
        txt = txt.color(SIM_TEXT_DARK);
    }
    ui.selectable_label(active, txt).on_hover_text(tip)
}

impl SimModuleState {
    pub fn ui(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
        if self.dynamic_running && self.project.lock_state != LockState::EntireProcessLocked {
            SoryntelApplication::sim_step_dynamic(&mut self.project, 1);
            ctx.request_repaint();
        }

        // 1. Top Menu Bar & 15 Top Toolbars
        egui::TopBottomPanel::top("sim_top_panel")
            .frame(egui::Frame::none().fill(SIM_WIN_BG))
            .show(ctx, |ui| {
                self.render_menu_bar(ui);
                if self.show_top_toolbars {
                    self.render_top_toolbars(ui);
                }
            });

        // 2. Bottom Status Bar (Zoom, Auto Backup, Auto Route, Orthogonal, Persist Tool, Snap to Grid, Coords)
        egui::TopBottomPanel::bottom("sim_status_panel")
            .frame(
                egui::Frame::none()
                    .fill(SIM_WIN_BG)
                    .stroke(Stroke::new(1.0, SIM_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 3.0)),
            )
            .show(ctx, |ui| {
                self.render_status_bar(ui);
            });

        // 3. Left Vertical Toolbar (12 toggleable groups; hides drawing tools when Drawing Locked - Fig. 58!)
        if self.show_left_toolbars {
            egui::SidePanel::left("sim_left_toolbar")
                .resizable(false)
                .exact_width(30.0)
                .frame(
                    egui::Frame::none()
                        .fill(SIM_TOOLBAR_BG)
                        .stroke(Stroke::new(0.8, SIM_BORDER))
                        .inner_margin(2.0),
                )
                .show(ctx, |ui| {
                    self.render_left_toolbar(ui);
                });
        }

        // 4. Right Docking Area (`Properties` / `Inspector` + `Icons`..`Log`)
        if self.show_right_top_dock || self.show_right_bottom_dock {
            egui::SidePanel::right("sim_right_docks")
                .resizable(true)
                .default_width(370.0)
                .min_width(250.0)
                .frame(egui::Frame::none().fill(SIM_WIN_BG).inner_margin(3.0))
                .show(ctx, |ui| {
                    docks::render_right_docks(ui, self);
                });
        }

        // 5. Center Flowsheet Canvas + Bottom Page Navigation Tabs (`Page1 | Page2 | Page3 | +`)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::WHITE))
            .show(ctx, |ui| {
                let page_bar_h = 24.0;
                let total = ui.available_rect_before_wrap();
                let canvas_area = egui::Rect::from_min_max(
                    total.min,
                    egui::Pos2::new(total.right(), (total.bottom() - page_bar_h).max(total.top() + 40.0)),
                );
                let mut canvas_ui = ui.new_child(egui::UiBuilder::new().max_rect(canvas_area));
                canvas::render_flowsheet_canvas(&mut canvas_ui, self);

                ui.allocate_space(Vec2::new(total.width(), (canvas_area.height()).max(0.0)));
                self.render_page_tabs(ui);
            });

        // 6. All Secondary Modal Windows & Editors
        editors::render_sim_modals(ctx, self);
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                self.undo();
            } else if i.modifiers.ctrl && i.key_pressed(egui::Key::Y) {
                self.redo();
            } else if i.key_pressed(egui::Key::Delete) {
                self.delete_selected();
            } else if i.key_pressed(egui::Key::F5) {
                self.run_static(false);
            } else if i.key_pressed(egui::Key::Escape) {
                self.active_tool = ActiveTool::Select;
                self.pending_stream_start_mm = None;
                self.pending_stream_start_unit = None;
            }
        });
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button(crate::i18n::text("common.001"), |ui| {
                if ui.button(crate::i18n::text("review.190")).clicked() {
                    self.load_preset("Empty Flowsheet", soryntel_thermo::sim::presets::empty_project());
                    ui.close_menu();
                }
                ui.menu_button("Open Reference Project...", |ui| {
                    for (label, proj) in SoryntelApplication::sim_presets() {
                        if ui.button(label).clicked() {
                            self.load_preset(label, proj);
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button("Open / Import .simjson...").clicked() {
                    self.export_kind = ExportModalKind::LoadProjectJson;
                    self.export_buffer.clear();
                    self.show_export_modal = true;
                    ui.close_menu();
                }
                if ui.button("Save Project (.simjson)...").clicked() {
                    self.export_kind = ExportModalKind::ProjectJson;
                    self.export_buffer = serde_json::to_string_pretty(&self.project).unwrap_or_default();
                    self.show_export_modal = true;
                    ui.close_menu();
                }
                if ui.button("Save as LCA JSON...").clicked() {
                    self.export_kind = ExportModalKind::LcaJson;
                    self.export_buffer = SoryntelApplication::sim_export_lca_json(&self.project)
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    self.show_export_modal = true;
                    ui.close_menu();
                }
                if ui.button("Export Flowsheet Image (.svg)...").clicked() {
                    self.export_kind = ExportModalKind::FlowsheetSvg;
                    self.export_buffer = SoryntelApplication::sim_export_svg(&self.project);
                    self.show_export_modal = true;
                    ui.close_menu();
                }
            });

            ui.menu_button(crate::i18n::text("common.028"), |ui| {
                ui.menu_button("Toolbars", |ui| {
                    ui.checkbox(&mut self.show_top_toolbars, "Show Top Toolbars Ribbon");
                    ui.checkbox(&mut self.show_left_toolbars, "Show Left Toolbars");
                    ui.separator();
                    ui.label(RichText::new("Top Toolbars (15):").strong());
                    ui.checkbox(&mut self.top_tb.project, "Project");
                    ui.checkbox(&mut self.top_tb.clipboard_undo, "Clipboard & Undo");
                    ui.checkbox(&mut self.top_tb.calculations, "Calculations");
                    ui.checkbox(&mut self.top_tb.sim_calculation, "Sim Calculation");
                    ui.checkbox(&mut self.top_tb.dynamic_simulation, "Dynamic Simulation");
                    ui.checkbox(&mut self.top_tb.dynamic_progress, "Dynamic ProgressBar");
                    ui.checkbox(&mut self.top_tb.font, "Font");
                    ui.checkbox(&mut self.top_tb.text_formatting, "Text Formatting");
                    ui.checkbox(&mut self.top_tb.layout, "Layout");
                    ui.checkbox(&mut self.top_tb.visualization, "Visualization");
                    ui.checkbox(&mut self.top_tb.dynamic_collect, "Dynamic Collect");
                    ui.checkbox(&mut self.top_tb.navigation, "Navigation");
                    ui.checkbox(&mut self.top_tb.report, "Report");
                    ui.checkbox(&mut self.top_tb.search, "Search");
                    ui.checkbox(&mut self.top_tb.info, "Info");
                });
                ui.menu_button("Docking Panes", |ui| {
                    ui.checkbox(&mut self.show_right_top_dock, "Properties / Inspector");
                    ui.checkbox(&mut self.show_right_bottom_dock, "Bottom Tool Tabs (Icons/Items/Log)");
                });
                ui.separator();
                ui.checkbox(&mut self.show_rulers, "Show Millimeter Rulers");
                ui.checkbox(&mut self.project.canvas_settings.show_grid, "Show Grid");
                if ui.button("Zoom 100%").clicked() {
                    self.zoom_pct = 100.0;
                    self.pan_mm = [-12.0, -4.0];
                    ui.close_menu();
                }
            });

            ui.menu_button("Select", |ui| {
                if ui.button("Select All Units").clicked() {
                    self.multi_selected_units = self.project.units.iter().map(|u| u.id).collect();
                    ui.close_menu();
                }
                if ui.button("Select All Streams").clicked() {
                    self.multi_selected_streams = self.project.streams.iter().map(|s| s.id).collect();
                    ui.close_menu();
                }
                if ui.button("Clear Selection").clicked() {
                    self.selected = SelectedObject::None;
                    self.multi_selected_units.clear();
                    self.multi_selected_streams.clear();
                    ui.close_menu();
                }
            });

            ui.menu_button("Flowsheet", |ui| {
                if ui.button(crate::i18n::text("review.184")).clicked() {
                    self.show_default_settings = true;
                    ui.close_menu();
                }
                if ui.button("Stream Table Editor...").clicked() {
                    self.show_stream_table_editor = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Lock Drawing (Green Frame)").clicked() {
                    self.project.lock_state = LockState::DrawingLocked;
                    self.active_tool = ActiveTool::Select;
                    ui.close_menu();
                }
                if ui.button("Lock Entire Process (Red Frame)").clicked() {
                    self.project.lock_state = LockState::EntireProcessLocked;
                    self.dynamic_running = false;
                    self.active_tool = ActiveTool::Select;
                    ui.close_menu();
                }
                if ui.button("Unlock").clicked() {
                    self.project.lock_state = LockState::Unlocked;
                    ui.close_menu();
                }
            });

            ui.menu_button(crate::i18n::text("common.018"), |ui| {
                if ui.button("Select Unit Models...").clicked() {
                    self.show_select_unit_models = true;
                    ui.close_menu();
                }
                if ui.button("Unit Editor...").clicked() {
                    if self.editing_unit_id.is_none() {
                        self.editing_unit_id = self.project.units.first().map(|u| u.id);
                    }
                    self.show_unit_editor = true;
                    ui.close_menu();
                }
                if ui.button("Scenario Editor...").clicked() {
                    self.show_scenario_editor = true;
                    ui.close_menu();
                }
                if ui.button("Reports...").clicked() {
                    self.reports_cache = Some(SoryntelApplication::sim_generate_reports(&self.project));
                    self.show_reports_window = true;
                    ui.close_menu();
                }
                if ui.button("Control Monitor...").clicked() {
                    self.open_control_monitor(0);
                    ui.close_menu();
                }
                if ui.button("Visualization Color Picker...").clicked() {
                    self.show_visualization_picker = true;
                    ui.close_menu();
                }
                if ui.button("Process Information...").clicked() {
                    self.show_process_info = true;
                    ui.close_menu();
                }
                if ui.button("Notes...").clicked() {
                    self.show_notes_window = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Mass Balancing...").clicked() {
                    self.mb_state.sync_from_project(&self.project);
                    self.show_mb_window = true;
                    ui.close_menu();
                }
                if ui.button("LCA Evaluation...").clicked() {
                    self.lca_state.ui_mode = crate::lca_ui::LcaUiMode::Classic;
                    self.lca_state.sync_from_project(&self.project);
                    self.show_lca_window = true;
                    ui.close_menu();
                }
                if ui.button("LCA Evaluation (beta)...").clicked() {
                    self.lca_state.ui_mode = crate::lca_ui::LcaUiMode::Beta;
                    self.lca_state.sync_from_project(&self.project);
                    self.show_lca_window = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("HSC Sim 7 -> Sim 10 Model Converter...").clicked() {
                    self.show_sim7_migrator = true;
                    ui.close_menu();
                }
            });

            ui.menu_button(crate::i18n::text("common.011"), |ui| {
                if ui.button("Calculate Active Flowsheet (F5)").clicked() {
                    self.run_static(false);
                    ui.close_menu();
                }
                if ui.button("Calculate Selected Units").clicked() {
                    self.run_static(true);
                    ui.close_menu();
                }
                if ui.button("Run All Active Controls").clicked() {
                    self.run_controls(None);
                    ui.close_menu();
                }
                if ui.button("Clear Internal Streams").clicked() {
                    self.push_undo();
                    SoryntelApplication::sim_clear_internal(&mut self.project);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Start / Pause Dynamic Simulation").clicked() {
                    self.dynamic_running = !self.dynamic_running;
                    ui.close_menu();
                }
                if ui.button("Step Dynamic +10s").clicked() {
                    SoryntelApplication::sim_step_dynamic(&mut self.project, 10);
                    ui.close_menu();
                }
                if ui.button("Empty All Tanks").clicked() {
                    SoryntelApplication::sim_empty_tanks(&mut self.project);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Convergence Monitor...").clicked() {
                    self.show_convergence_monitor = true;
                    ui.close_menu();
                }
                if ui.button("Calculation Order...").clicked() {
                    self.show_calc_order_window = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("Diagnostics", |ui| {
                if ui.button("Open Log Tab").clicked() {
                    self.right_bottom_tab = RightBottomTab::Log;
                    ui.close_menu();
                }
                if ui.button("Global Mass & Enthalpy Balance Check").clicked() {
                    let round = SoryntelApplication::sim_global_balances(&self.project);
                    self.status_banner = format!(
                        "Global Balance Check: MB={:.4}%, EB={:.4}%",
                        round.mb_pct, round.eb_pct
                    );
                    ui.close_menu();
                }
            });

            ui.menu_button(crate::i18n::text("common.003"), |ui| {
                if ui.button("Check for Updates...").clicked() {
                    self.show_updates_modal = true;
                    ui.close_menu();
                }
                if ui.button("About HSC Sim...").clicked() {
                    self.show_about_modal = true;
                    ui.close_menu();
                }
            });

            // Quick Demo Project shortcut pill right on menu bar so user can inspect immediately!
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("quick_preset_cb")
                    .selected_text(format!("Demo: {}", self.active_preset_name))
                    .show_ui(ui, |ui| {
                        for (label, proj) in SoryntelApplication::sim_presets() {
                            if ui.selectable_label(self.active_preset_name == label, label).clicked() {
                                self.load_preset(label, proj);
                            }
                        }
                    });
            });
        });
    }

    fn render_top_toolbars(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(SIM_TOOLBAR_BG)
            .stroke(Stroke::new(0.8, SIM_BORDER))
            .inner_margin(egui::Margin::symmetric(5.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if self.top_tb.project {
                        if ui.button(crate::i18n::text("common.033")).on_hover_text(crate::i18n::text("review.190")).clicked() {
                            self.load_preset("Empty Flowsheet", soryntel_thermo::sim::presets::empty_project());
                        }
                        if ui.button(crate::i18n::text("common.087")).on_hover_text("Open / Import .simjson").clicked() {
                            self.export_kind = ExportModalKind::LoadProjectJson;
                            self.export_buffer.clear();
                            self.show_export_modal = true;
                        }
                        if ui.button(crate::i18n::text("common.034")).on_hover_text("Save Project (.simjson)").clicked() {
                            self.export_kind = ExportModalKind::ProjectJson;
                            self.export_buffer = serde_json::to_string_pretty(&self.project).unwrap_or_default();
                            self.show_export_modal = true;
                        }
                        ui.separator();
                    }

                    if self.top_tb.clipboard_undo {
                        if ui.button(crate::i18n::text("common.029")).on_hover_text("Copy Selected Unit").clicked() {
                            if let SelectedObject::Unit(uid) = self.selected {
                                self.clipboard_unit = self.project.units.iter().find(|u| u.id == uid).cloned();
                                self.status_banner = "Copied unit to clipboard".to_string();
                            }
                        }
                        if ui
                            .add_enabled(self.clipboard_unit.is_some(), egui::Button::new("Paste"))
                            .on_hover_text("Paste Unit onto Flowsheet")
                            .clicked()
                        {
                            if let Some(mut copy) = self.clipboard_unit.clone() {
                                self.push_undo();
                                let nid = self.alloc_id();
                                copy.id = nid;
                                copy.name_id = format!("{} Copy", copy.name_id);
                                copy.pos[0] += 14.0;
                                copy.pos[1] += 10.0;
                                copy.page = self.project.active_page;
                                self.project.units.push(copy);
                                self.selected = SelectedObject::Unit(nid);
                            }
                        }
                        if ui.button(crate::i18n::text("common.121")).clicked() {
                            self.undo();
                        }
                        if ui.button(crate::i18n::text("common.122")).clicked() {
                            self.redo();
                        }
                        ui.separator();
                    }

                    if self.top_tb.visualization {
                        if tb_icon_btn(
                            ui,
                            "connectivity",
                            self.project.visualization.connectivity_mode,
                            "Stream Connectivity Visualization (Blue=Input, Black=Internal, Red=Output)",
                        )
                        .clicked()
                        {
                            self.project.visualization.connectivity_mode =
                                !self.project.visualization.connectivity_mode;
                        }
                        egui::ComboBox::from_id_salt("vis_metric_cb")
                            .width(142.0)
                            .selected_text(&self.project.visualization.variable)
                            .show_ui(ui, |ui| {
                                for m in VISUALIZATION_METRICS {
                                    if ui.selectable_value(
                                        &mut self.project.visualization.variable,
                                        (*m).to_string(),
                                        *m,
                                    )
                                    .changed()
                                    {
                                        self.project.visualization.active = *m != "Select visualization";
                                    }
                                }
                            });
                        if ui.button("Colors").on_hover_text(crate::i18n::text("review.189")).clicked() {
                            self.show_visualization_picker = true;
                        }
                        ui.separator();
                    }

                    if self.top_tb.sim_calculation {
                        if tb_icon_btn(ui, "play", false, "Calculate Active Flowsheet (F5)").clicked() {
                            self.run_static(false);
                        }
                        ui.add(
                            egui::DragValue::new(&mut self.calc_rounds_input)
                                .range(1..=500)
                                .speed(1.0),
                        )
                        .on_hover_text("Max convergence rounds");
                        if tb_icon_btn(
                            ui,
                            "calc_order",
                            self.highlight_calc_order_on_canvas,
                            "Toggle Process Calculation Order Overlay",
                        )
                        .clicked()
                        {
                            self.highlight_calc_order_on_canvas = !self.highlight_calc_order_on_canvas;
                        }
                        ui.separator();
                    }

                    if self.top_tb.dynamic_simulation {
                        if tb_icon_btn(
                            ui,
                            "play",
                            self.dynamic_running,
                            "Run / Resume Dynamic Simulation",
                        )
                        .clicked()
                        {
                            self.dynamic_running = !self.dynamic_running;
                        }
                        if tb_icon_btn(ui, "pause", false, "Pause Dynamic Simulation").clicked() {
                            self.dynamic_running = false;
                        }
                        if tb_icon_btn(ui, "stop", false, "Stop & Empty Tanks").clicked() {
                            self.dynamic_running = false;
                            SoryntelApplication::sim_empty_tanks(&mut self.project);
                        }
                    }

                    if self.top_tb.dynamic_progress {
                        let mins = (self.project.dynamic_time_s / 60.0).floor() as usize;
                        let secs = (self.project.dynamic_time_s % 60.0).floor() as usize;
                        let frac = ((self.project.dynamic_time_s / self.project.calc_settings.end_time_s.max(1.0)) as f32).clamp(0.05, 1.0);
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(112.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(208, 232, 198));
                        let fill_r = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * frac, rect.height()));
                        ui.painter().rect_filled(fill_r, 2.0, Color32::from_rgb(138, 196, 114));
                        ui.painter().rect_stroke(rect, 2.0, Stroke::new(0.8, SIM_BORDER));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{mins} min {secs} s"),
                            egui::FontId::proportional(11.0),
                            Color32::BLACK,
                        );
                        ui.separator();
                    }

                    if self.top_tb.font {
                        egui::ComboBox::from_id_salt("font_combo")
                            .width(74.0)
                            .selected_text(&self.selected_font_name)
                            .show_ui(ui, |ui| {
                                for f in ["Arial", "Calibri", "Consolas", "Segoe UI"] {
                                    ui.selectable_value(&mut self.selected_font_name, f.to_string(), f);
                                }
                            });
                    }

                    if self.top_tb.report {
                        if ui.button(crate::i18n::text("review.187")).clicked() {
                            self.reports_cache = Some(SoryntelApplication::sim_generate_reports(&self.project));
                            self.show_reports_window = true;
                        }
                        if ui.button("Scenarios").clicked() {
                            self.show_scenario_editor = true;
                        }
                        if ui.button("Stream Tables").clicked() {
                            self.show_stream_table_editor = true;
                        }
                        if ui.button("Models").clicked() {
                            self.show_select_unit_models = true;
                        }
                    }

                    if self.top_tb.search {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.toolbar_search_text)
                                .desired_width(92.0)
                                .hint_text("Find unit..."),
                        );
                        if resp.changed() && !self.toolbar_search_text.trim().is_empty() {
                            let q = self.toolbar_search_text.to_lowercase();
                            if let Some(u) = self.project.units.iter().find(|u| u.name_id.to_lowercase().contains(&q)) {
                                self.selected = SelectedObject::Unit(u.id);
                                self.project.active_page = u.page;
                                self.pan_mm = [u.pos[0] - 40.0, u.pos[1] - 30.0];
                            } else if let Some(s) = self.project.streams.iter().find(|s| s.name_id.to_lowercase().contains(&q)) {
                                self.selected = SelectedObject::Stream(s.id);
                                self.project.active_page = s.page;
                            }
                        }
                    }

                    if self.top_tb.info {
                        if ui.button("Info").clicked() {
                            self.show_process_info = true;
                        }
                        if ui.button(crate::i18n::text("review.188")).clicked() {
                            self.show_notes_window = true;
                        }
                    }
                });
            });
    }

    fn render_left_toolbar(&mut self, ui: &mut egui::Ui) {
        let locked = self.project.lock_state != LockState::Unlocked;
        ui.vertical_centered(|ui| {
            if self.left_tb.select {
                if tb_icon_btn(ui, "select", self.active_tool == ActiveTool::Select, "Select Tool (Esc)").clicked() {
                    self.active_tool = ActiveTool::Select;
                }
                if tb_icon_btn(ui, "pan", self.active_tool == ActiveTool::Pan, "Pan Tool (drag canvas)").clicked() {
                    self.active_tool = ActiveTool::Pan;
                }
                ui.separator();
            }

            // Fig. 58: Drawing lock hides Units / Streams / Annotations drawing buttons!
            if !locked {
                if self.left_tb.units {
                    for (glyph, kind, tip) in [
                        ("unit_u", UnitKind::Generic, "Add Generic Unit [U]"),
                        ("unit_r", UnitKind::Reactions, "Add Reactions (Hydro) Unit [R]"),
                        ("unit_d", UnitKind::Distributions, "Add Distributions (Pyro/Particles) Unit [D]"),
                        ("unit_tau", UnitKind::Dynamic, "Add Dynamic Unit [τ]"),
                    ] {
                        if tb_icon_btn(ui, glyph, self.active_tool == ActiveTool::AddUnit(kind), tip).clicked() {
                            self.active_tool = ActiveTool::AddUnit(kind);
                        }
                    }
                    ui.separator();
                }

                if self.left_tb.streams {
                    if tb_icon_btn(ui, "stream", self.active_tool == ActiveTool::AddStream, "Draw Orthogonal Stream").clicked() {
                        self.active_tool = ActiveTool::AddStream;
                    }
                    ui.separator();
                }

                if self.left_tb.text {
                    if tb_text_btn(ui, "H", self.active_tool == ActiveTool::AddHeaderLabel, "Add Header Label").clicked() {
                        self.active_tool = ActiveTool::AddHeaderLabel;
                    }
                    if tb_text_btn(ui, "Aa", self.active_tool == ActiveTool::AddTextLabel, "Add Text Label").clicked() {
                        self.active_tool = ActiveTool::AddTextLabel;
                    }
                    if tb_text_btn(ui, "[T]", self.active_tool == ActiveTool::AddTextBox, "Add Multiline TextBox").clicked() {
                        self.active_tool = ActiveTool::AddTextBox;
                    }
                    ui.separator();
                }

                if self.left_tb.picture {
                    if tb_text_btn(ui, "Img", self.active_tool == ActiveTool::AddPictureBox, "Insert Picture Box").clicked() {
                        self.active_tool = ActiveTool::AddPictureBox;
                    }
                    ui.separator();
                }

                if self.left_tb.table {
                    if tb_icon_btn(ui, "table", self.active_tool == ActiveTool::AddTable, "Insert Flowsheet Table").clicked() {
                        self.active_tool = ActiveTool::AddTable;
                    }
                    if tb_text_btn(ui, "Hdr", self.active_tool == ActiveTool::AddTitleTable, "Insert Title Table (w/ Logo)").clicked() {
                        self.active_tool = ActiveTool::AddTitleTable;
                    }
                    ui.separator();
                }

                if self.left_tb.shape {
                    if tb_text_btn(ui, "/", self.active_tool == ActiveTool::AddLine, "Draw Line").clicked() {
                        self.active_tool = ActiveTool::AddLine;
                    }
                    if tb_text_btn(ui, "[]", self.active_tool == ActiveTool::AddRect, "Draw Rectangle").clicked() {
                        self.active_tool = ActiveTool::AddRect;
                    }
                    if tb_text_btn(ui, "()", self.active_tool == ActiveTool::AddEllipse, "Draw Ellipse/Circle").clicked() {
                        self.active_tool = ActiveTool::AddEllipse;
                    }
                    if tb_text_btn(ui, "^", self.active_tool == ActiveTool::AddTriangle, "Draw Triangle").clicked() {
                        self.active_tool = ActiveTool::AddTriangle;
                    }
                    if tb_text_btn(ui, "<>", self.active_tool == ActiveTool::AddPolygon, "Draw Polygon").clicked() {
                        self.active_tool = ActiveTool::AddPolygon;
                    }
                    ui.separator();
                }

                if self.left_tb.size {
                    if tb_text_btn(ui, "Sz+", false, "Grow Selected Unit (+4 mm)").clicked() {
                        if let SelectedObject::Unit(uid) = self.selected {
                            if let Some(u) = self.project.units.iter_mut().find(|u| u.id == uid) {
                                u.size[0] += 4.0;
                                u.size[1] += 4.0;
                            }
                        }
                    }
                }
                if self.left_tb.rotate_flip {
                    if tb_text_btn(ui, "Rot", false, "Swap Selected Unit Width/Height").clicked() {
                        if let SelectedObject::Unit(uid) = self.selected {
                            if let Some(u) = self.project.units.iter_mut().find(|u| u.id == uid) {
                                u.size.swap(0, 1);
                            }
                        }
                    }
                }
                if self.left_tb.depth {
                    if tb_text_btn(ui, "Top", false, "Bring Selected Unit to Front").clicked() {
                        if let SelectedObject::Unit(uid) = self.selected {
                            if let Some(idx) = self.project.units.iter().position(|u| u.id == uid) {
                                let u = self.project.units.remove(idx);
                                self.project.units.push(u);
                            }
                        }
                    }
                }
                if self.left_tb.alignment {
                    if tb_text_btn(ui, "Aln", false, "Snap Selected Unit to Grid").clicked() {
                        let gw = self.project.canvas_settings.grid_width.max(1.0);
                        let gh = self.project.canvas_settings.grid_height.max(1.0);
                        if let SelectedObject::Unit(uid) = self.selected {
                            if let Some(u) = self.project.units.iter_mut().find(|u| u.id == uid) {
                                u.pos[0] = (u.pos[0] / gw).round() * gw;
                                u.pos[1] = (u.pos[1] / gh).round() * gh;
                            }
                        }
                    }
                    ui.separator();
                }
            }

            if self.left_tb.lock {
                if tb_icon_btn(
                    ui,
                    "lock",
                    self.project.lock_state != LockState::Unlocked,
                    "Cycle Lock State: Unlocked -> Lock Drawing (Green) -> Lock Entire Process (Red)",
                )
                .clicked()
                {
                    self.project.lock_state = match self.project.lock_state {
                        LockState::Unlocked => LockState::DrawingLocked,
                        LockState::DrawingLocked => LockState::EntireProcessLocked,
                        LockState::EntireProcessLocked => LockState::Unlocked,
                    };
                    if self.project.lock_state != LockState::Unlocked {
                        self.active_tool = ActiveTool::Select;
                    }
                }
            }
        });
    }

    fn render_page_tabs(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(SIM_HEADER_BG)
            .stroke(Stroke::new(0.8, SIM_BORDER))
            .inner_margin(egui::Margin::symmetric(4.0, 2.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("|<").clicked() {
                        self.project.active_page = 0;
                    }
                    if ui.button("<").clicked() && self.project.active_page > 0 {
                        self.project.active_page -= 1;
                    }
                    if ui.button(">").clicked() && self.project.active_page + 1 < self.project.pages.len() {
                        self.project.active_page += 1;
                    }
                    if ui.button(">|").clicked() && !self.project.pages.is_empty() {
                        self.project.active_page = self.project.pages.len() - 1;
                    }
                    if ui.button("+").on_hover_text("Add Flowsheet Page").clicked() {
                        let n = self.project.pages.len() + 1;
                        self.project.pages.push(SimPage {
                            name: format!("Page{n}"),
                            visible: true,
                        });
                        self.project.active_page = n - 1;
                    }
                    ui.separator();
                    let pages_len = self.project.pages.len();
                    for i in 0..pages_len {
                        let visible = self.project.pages[i].visible;
                        if !visible {
                            continue;
                        }
                        let sel = self.project.active_page == i;
                        if ui.selectable_label(sel, &self.project.pages[i].name).clicked() {
                            self.project.active_page = i;
                        }
                    }
                });
            });
    }

    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match self.project.lock_state {
                LockState::Unlocked => {}
                LockState::DrawingLocked => {
                    ui.label(RichText::new("Drawing Locked").strong().color(Color32::from_rgb(200, 24, 24)));
                    ui.separator();
                }
                LockState::EntireProcessLocked => {
                    ui.label(RichText::new("Process Locked").strong().color(Color32::from_rgb(200, 24, 24)));
                    ui.separator();
                }
            }

            ui.label(format!("{:.0}%", self.zoom_pct));
            ui.add(egui::Slider::new(&mut self.zoom_pct, 25.0..=250.0).show_value(false).custom_formatter(|v, _| format!("{v:.0}%")));
            ui.separator();

            let backup_lbl = match self.project.canvas_settings.auto_backup_minutes {
                None => "Auto Backup [OFF]",
                Some(15) => "Auto Backup [15m]",
                Some(30) => "Auto Backup [30m]",
                Some(_) => "Auto Backup [60m]",
            };
            if ui.button(backup_lbl).clicked() {
                self.project.canvas_settings.auto_backup_minutes =
                    match self.project.canvas_settings.auto_backup_minutes {
                        None => Some(15),
                        Some(15) => Some(30),
                        Some(30) => Some(60),
                        Some(_) => None,
                    };
            }
            ui.separator();

            ui.checkbox(&mut self.project.canvas_settings.auto_route, "Auto Route");
            ui.checkbox(&mut self.project.canvas_settings.orthogonal, "Orthogonal");
            ui.checkbox(&mut self.project.canvas_settings.persist_tool, "Persist Tool");
            ui.checkbox(&mut self.project.canvas_settings.snap_to_grid, "Snap to Grid");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{:.0}, {:.0}", self.cursor_mm[0], self.cursor_mm[1]));
                ui.separator();
                ui.label(RichText::new(&self.status_banner).size(11.0).color(SIM_MUTED_TEXT));
            });
        });
    }
}
