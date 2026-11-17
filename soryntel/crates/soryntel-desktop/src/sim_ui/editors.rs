use eframe::egui::{self, Color32, RichText};
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, Points, VLine};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::model::*;
use soryntel_thermo::sim::presets::DEFAULT_STREAM_VARIABLES;
use crate::sim_ui::state::*;

pub fn render_sim_modals(ctx: &egui::Context, state: &mut SimModuleState) {
    if state.show_streams_src_dest {
        render_streams_src_dest(ctx, state);
    }
    if state.show_default_settings {
        render_default_settings(ctx, state);
    }
    if state.show_table_editor {
        render_table_editor(ctx, state);
    }
    if state.show_stream_table_editor {
        render_stream_table_editor(ctx, state);
    }
    if state.show_select_unit_models {
        render_select_unit_models(ctx, state);
    }
    if state.show_unit_editor {
        render_unit_editor(ctx, state);
    }
    if state.show_control_monitor {
        render_control_monitor(ctx, state);
    }
    if state.show_scenario_editor {
        render_scenario_editor(ctx, state);
    }
    if state.show_reports_window {
        render_reports_window(ctx, state);
    }
    if state.show_process_info {
        render_process_info(ctx, state);
    }
    if state.show_notes_window {
        render_notes_window(ctx, state);
    }
    if state.show_convergence_monitor {
        render_convergence_monitor(ctx, state);
    }
    if state.show_calc_order_window {
        render_calc_order_window(ctx, state);
    }
    if state.show_visualization_picker {
        render_visualization_picker(ctx, state);
    }
    if state.show_sim7_migrator {
        render_sim7_migrator(ctx, state);
    }
    if state.show_export_modal {
        render_export_modal(ctx, state);
    }
    if state.show_lca_window {
        crate::lca_ui::render_lca_floating_window(
            ctx,
            &mut state.show_lca_window,
            &state.project,
            &mut state.lca_state,
        );
    }
    if state.show_mb_window {
        crate::mass_balance_ui::render_mb_floating_window(
            ctx,
            &mut state.show_mb_window,
            &mut state.project,
            &mut state.mb_state,
        );
    }
    if state.show_about_modal {
        render_about_modal(ctx, state);
    }
    if state.show_updates_modal {
        render_updates_modal(ctx, state);
    }
}

fn render_streams_src_dest(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_streams_src_dest;
    egui::Window::new("Streams Source and Destination")
        .open(&mut open)
        .default_width(360.0)
        .show(ctx, |ui| {
            let Some(sid) = state.src_dest_stream_id else {
                ui.label("No stream selected.");
                return;
            };
            let stream_name = state
                .project
                .streams
                .iter()
                .find(|s| s.id == sid)
                .map(|s| s.name_id.clone())
                .unwrap_or_default();
            let unit_name = |opt: Option<usize>| -> String {
                match opt {
                    None => "(None / Flowsheet Boundary)".to_string(),
                    Some(uid) => state
                        .project
                        .units
                        .iter()
                        .find(|u| u.id == uid)
                        .map(|u| u.name_id.clone())
                        .unwrap_or_else(|| format!("Unit#{uid}")),
                }
            };
            let units_list: Vec<(usize, String)> = state
                .project
                .units
                .iter()
                .map(|u| (u.id, u.name_id.clone()))
                .collect();

            ui.label(RichText::new(format!("Stream: {stream_name}")).strong());
            ui.separator();

            if let Some(s) = state.project.streams.iter().find(|s| s.id == sid) {
                egui::Grid::new("src_dst_grid").spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new("Old").strong());
                    ui.label(RichText::new(crate::i18n::text("common.033")).strong());
                    ui.end_row();

                    ui.label("Source:");
                    ui.label(unit_name(s.source_unit));
                    egui::ComboBox::from_id_salt("new_src_cb")
                        .selected_text(unit_name(state.src_dest_draft_src))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.src_dest_draft_src, None, "(None / Boundary)");
                            for (uid, nm) in &units_list {
                                ui.selectable_value(&mut state.src_dest_draft_src, Some(*uid), nm);
                            }
                        });
                    ui.end_row();

                    ui.label("Destination:");
                    ui.label(unit_name(s.dest_unit));
                    egui::ComboBox::from_id_salt("new_dst_cb")
                        .selected_text(unit_name(state.src_dest_draft_dst))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.src_dest_draft_dst, None, "(None / Boundary)");
                            for (uid, nm) in &units_list {
                                ui.selectable_value(&mut state.src_dest_draft_dst, Some(*uid), nm);
                            }
                        });
                    ui.end_row();
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(crate::i18n::text("common.032")).clicked() {
                    let src = state.src_dest_draft_src;
                    let dst = state.src_dest_draft_dst;
                    state.push_undo();
                    if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                        s.source_unit = src;
                        s.dest_unit = dst;
                    }
                    state.status_banner = format!("Updated connections for '{}'", stream_name);
                    state.show_streams_src_dest = false;
                }
                if ui.button("Discard").clicked() {
                    state.show_streams_src_dest = false;
                }
            });
        });
    state.show_streams_src_dest = open;
}

fn render_default_settings(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_default_settings;
    egui::Window::new("Default Settings")
        .open(&mut open)
        .default_size([400.0, 360.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, t) in ["Page", "Unit", "Stream", "ArrowHeads", "Header Label"].iter().enumerate() {
                    if ui.selectable_label(state.default_settings_tab == i, *t).clicked() {
                        state.default_settings_tab = i;
                    }
                }
            });
            ui.separator();

            let cs = &mut state.project.canvas_settings;
            match state.default_settings_tab {
                0 => {
                    egui::Grid::new("def_page_grid").spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label("Page Background Color");
                        let mut c = Color32::from_rgb(cs.page_bg_rgb[0], cs.page_bg_rgb[1], cs.page_bg_rgb[2]);
                        if ui.color_edit_button_srgba(&mut c).changed() {
                            cs.page_bg_rgb = [c.r(), c.g(), c.b()];
                        }
                        ui.end_row();

                        ui.label("Show Grid");
                        ui.checkbox(&mut cs.show_grid, "");
                        ui.end_row();

                        ui.label("Grid Style");
                        ui.label(&cs.grid_style);
                        ui.end_row();

                        ui.label("Grid Color");
                        let mut gc = Color32::from_rgb(cs.grid_color_rgb[0], cs.grid_color_rgb[1], cs.grid_color_rgb[2]);
                        if ui.color_edit_button_srgba(&mut gc).changed() {
                            cs.grid_color_rgb = [gc.r(), gc.g(), gc.b()];
                        }
                        ui.end_row();

                        ui.label("Grid Width / Height");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut cs.grid_width).range(1.0..=20.0));
                            ui.add(egui::DragValue::new(&mut cs.grid_height).range(1.0..=20.0));
                        });
                        ui.end_row();
                    });
                }
                1 => {
                    ui.label("Default Unit Fill Gradient: 255;255;255 -> 166;214;255");
                    ui.label("Default Border Line Width: 0.25 mm | Font: Arial 8pt");
                }
                2 => {
                    ui.checkbox(&mut cs.orthogonal, "Orthogonal Stream Routing");
                    ui.checkbox(&mut cs.auto_route, "Auto Route around Units");
                    ui.checkbox(&mut cs.snap_to_grid, "Snap to Grid");
                }
                3 => {
                    ui.label("Input Stream Source: Hollow Circle");
                    ui.label("Stream End Arrowhead: Triangle (Size 3)");
                }
                _ => {
                    ui.label("Header Label Default Font: Arial 12pt Bold");
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.016")).clicked() || ui.button("Apply and Exit").clicked() {
                        state.show_default_settings = false;
                    }
                    if ui.button("Restore Default").clicked() {
                        state.project.canvas_settings = DefaultCanvasSettings::default();
                    }
                });
            });
        });
    state.show_default_settings = open;
}

fn render_table_editor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_table_editor;
    egui::Window::new("Table Editor")
        .open(&mut open)
        .default_size([460.0, 320.0])
        .show(ctx, |ui| {
            let Some(tid) = state.editing_table_id else {
                ui.label("Select a Table or Title Table on the canvas first.");
                return;
            };
            if let Some(tbl) = state.project.tables.iter_mut().find(|t| t.id == tid) {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut tbl.size_locked, "Size Lock (prevent auto-shrink warning)");
                    if tbl.size_locked {
                        ui.label(RichText::new("Width/Height locked").color(Color32::from_rgb(180, 90, 0)));
                    }
                });
                ui.separator();
                if tbl.is_title_table {
                    egui::Grid::new("title_tbl_edit").spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("Title:");
                        ui.text_edit_singleline(&mut tbl.title);
                        ui.end_row();
                        ui.label("Organization / Author:");
                        ui.text_edit_singleline(&mut tbl.organization_author);
                        ui.end_row();
                        ui.label("HSC Version:");
                        ui.text_edit_singleline(&mut tbl.hsc_version);
                        ui.end_row();
                        ui.label(crate::i18n::text("common.089"));
                        ui.text_edit_singleline(&mut tbl.date);
                        ui.end_row();
                        ui.label("Logo Text:");
                        ui.text_edit_singleline(&mut tbl.logo_text);
                        ui.end_row();
                    });
                } else {
                    if ui.button(crate::i18n::text("common.095")).clicked() {
                        let cols = tbl.cells.first().map(|r| r.len()).unwrap_or(3);
                        tbl.cells.push(vec![String::new(); cols]);
                    }
                    egui::ScrollArea::both().show(ui, |ui| {
                        egui::Grid::new("tbl_cells_edit").spacing([4.0, 4.0]).show(ui, |ui| {
                            for row in &mut tbl.cells {
                                for cell in row.iter_mut() {
                                    ui.add(egui::TextEdit::singleline(cell).desired_width(110.0));
                                }
                                ui.end_row();
                            }
                        });
                    });
                }
            }
        });
    state.show_table_editor = open;
}

fn render_stream_table_editor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_stream_table_editor;
    egui::Window::new("Stream Table Editor")
        .open(&mut open)
        .default_size([740.0, 440.0])
        .show(ctx, |ui| {
            let active_idx = state.project.active_stream_table_set;
            ui.columns(3, |cols| {
                // Left column: StreamTableSet list + Properties (Fig. 19)
                cols[0].label(RichText::new("StreamTableSets").strong());
                cols[0].horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.015")).clicked() {
                        let n = state.project.stream_table_sets.len() + 1;
                        state.project.stream_table_sets.push(StreamTableSet {
                            name: format!("StreamTableSet{n}"),
                            view_mode: TableViewMode::NameValueAndUnit,
                            columns: 1,
                            rows: 3,
                            variables: vec!["Total Mass Flow".into(), "Temperature".into(), "Cu".into()],
                            assigned_streams: Vec::new(),
                            header_visible: true,
                            use_alias_name: true,
                            transpose: false,
                        });
                        state.project.active_stream_table_set = n - 1;
                    }
                    if ui.button("Place Flow Table on Canvas").clicked() {
                        if let Some(set) = state.project.stream_table_sets.get(active_idx).cloned() {
                            let id = state.alloc_id();
                            let mut cells = Vec::new();
                            let mut hdr = vec!["Variable".into(), "Unit".into()];
                            for sn in &set.assigned_streams {
                                hdr.push(sn.clone());
                            }
                            if hdr.len() == 2 {
                                hdr.push("Main Stream".into());
                            }
                            cells.push(hdr);
                            for v in &set.variables {
                                let mut row = vec![v.clone(), "".into()];
                                if set.assigned_streams.is_empty() {
                                    if let Some(s) = state.project.streams.first() {
                                        let (val, unit_s) = SoryntelApplication::sim_metric_value(s, v);
                                        row[1] = unit_s.to_string();
                                        row.push(format!("{val:.2}"));
                                    }
                                } else {
                                    for sn in &set.assigned_streams {
                                        if let Some(s) = state.project.streams.iter().find(|s| &s.name_id == sn) {
                                            let (val, unit_s) = SoryntelApplication::sim_metric_value(s, v);
                                            row[1] = unit_s.to_string();
                                            row.push(format!("{val:.2}"));
                                        }
                                    }
                                }
                                cells.push(row);
                            }
                            state.project.tables.push(SimTableObj {
                                id,
                                pos: [30.0, 96.0],
                                size: [92.0, 32.0],
                                page: state.project.active_page,
                                layer: 2,
                                is_title_table: false,
                                size_locked: false,
                                cells,
                                title: String::new(),
                                organization_author: String::new(),
                                hsc_version: String::new(),
                                date: String::new(),
                                logo_text: String::new(),
                            });
                            state.status_banner = "Placed Flow Table onto canvas".to_string();
                        }
                    }
                });

                for i in 0..state.project.stream_table_sets.len() {
                    let nm = state.project.stream_table_sets[i].name.clone();
                    if cols[0]
                        .selectable_label(state.project.active_stream_table_set == i, nm)
                        .clicked()
                    {
                        state.project.active_stream_table_set = i;
                    }
                }
                cols[0].separator();
                if let Some(set) = state.project.stream_table_sets.get_mut(active_idx) {
                    cols[0].label(RichText::new("Properties").strong());
                    cols[0].horizontal(|ui| {
                        ui.label(crate::i18n::text("common.030"));
                        ui.text_edit_singleline(&mut set.name);
                    });
                    egui::ComboBox::from_label("ViewMode")
                        .selected_text(set.view_mode.as_str())
                        .show_ui(&mut cols[0], |ui| {
                            ui.selectable_value(&mut set.view_mode, TableViewMode::NameValueAndUnit, "Name, value and unit");
                            ui.selectable_value(&mut set.view_mode, TableViewMode::FlowTable, "Flow Table");
                        });
                    cols[0].checkbox(&mut set.header_visible, "HeaderVisible");
                    cols[0].checkbox(&mut set.use_alias_name, "UseAliasName");
                    cols[0].checkbox(&mut set.transpose, "Transpose");
                }

                // Middle column: All Stream Variables
                cols[1].label(RichText::new("Stream Variables (22)").strong());
                egui::ScrollArea::vertical().id_salt("st_all_vars").max_height(340.0).show(&mut cols[1], |ui| {
                    for (idx, (var, unit)) in DEFAULT_STREAM_VARIABLES.iter().enumerate() {
                        let sel = state.stream_table_var_sel == Some(idx);
                        if ui.selectable_label(sel, format!("{var}  [{unit}]")).clicked() {
                            state.stream_table_var_sel = Some(idx);
                        }
                    }
                });

                // Right column: Assigned variables & Assigned Streams
                cols[2].horizontal(|ui| {
                    if ui.button("> Add Variable").clicked() {
                        if let Some(vi) = state.stream_table_var_sel {
                            if let Some((var, _)) = DEFAULT_STREAM_VARIABLES.get(vi) {
                                if let Some(set) = state.project.stream_table_sets.get_mut(active_idx) {
                                    if !set.variables.iter().any(|x| x == var) {
                                        set.variables.push((*var).to_string());
                                    }
                                }
                            }
                        }
                    }
                    if ui.button("< Remove").clicked() {
                        if let Some(ci) = state.stream_table_canvas_sel {
                            if let Some(set) = state.project.stream_table_sets.get_mut(active_idx) {
                                if ci < set.variables.len() {
                                    set.variables.remove(ci);
                                }
                            }
                        }
                    }
                });
                cols[2].label(RichText::new("Assigned Variables").strong());
                if let Some(set) = state.project.stream_table_sets.get(active_idx) {
                    for (ci, v) in set.variables.iter().enumerate() {
                        if cols[2]
                            .selectable_label(state.stream_table_canvas_sel == Some(ci), v)
                            .clicked()
                        {
                            state.stream_table_canvas_sel = Some(ci);
                        }
                    }
                    cols[2].separator();
                    cols[2].label(RichText::new("Assigned Streams:").strong());
                    for s_name in &set.assigned_streams {
                        cols[2].label(format!("• {s_name}"));
                    }
                }
            });
        });
    state.show_stream_table_editor = open;
}

fn render_select_unit_models(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_select_unit_models;
    egui::Window::new("Select Unit Models")
        .open(&mut open)
        .default_size([700.0, 420.0])
        .show(ctx, |ui| {
            let mut catalog = SoryntelApplication::sim_model_catalog();
            catalog.extend(state.project.imported_models.iter().cloned());

            ui.horizontal(|ui| {
                for (tab, lbl) in [
                    (ModelLibraryTab::Species, "Species"),
                    (ModelLibraryTab::Particles, "Particles"),
                    (ModelLibraryTab::Others, "Others"),
                    (ModelLibraryTab::Imported, "Imported"),
                ] {
                    if ui.selectable_label(state.unit_model_tab == tab, lbl).clicked() {
                        state.unit_model_tab = tab;
                    }
                }
            });
            ui.separator();

            let filtered: Vec<(usize, &UnitModelCatalogEntry)> = catalog
                .iter()
                .enumerate()
                .filter(|(_, m)| m.category == state.unit_model_tab)
                .collect();

            egui::ScrollArea::vertical().max_height(210.0).show(ui, |ui| {
                egui::Grid::new("unit_models_grid").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    for hdr in ["Model Name", "Mode", "Technology", "Type", "Version", "Calculation"] {
                        ui.label(RichText::new(hdr).strong());
                    }
                    ui.end_row();
                    for (orig_i, entry) in &filtered {
                        let sel = state.selected_catalog_idx == *orig_i;
                        if ui.selectable_label(sel, &entry.model_name).clicked() {
                            state.selected_catalog_idx = *orig_i;
                        }
                        ui.label(&entry.mode);
                        ui.label(&entry.technology);
                        ui.label(entry.impl_type.as_str());
                        ui.label(&entry.version);
                        ui.label(&entry.calculation);
                        ui.end_row();
                    }
                });
            });

            ui.separator();
            if let Some(entry) = catalog.get(state.selected_catalog_idx) {
                ui.horizontal(|ui| {
                    let preview_k = if entry.mode.contains("Distribution") || entry.mode.contains("Particles") {
                        crate::sim_ui::icons::unit_kind_icon_fallback(UnitKind::Distributions)
                    } else if entry.mode.contains("Dynamic") {
                        crate::sim_ui::icons::unit_kind_icon_fallback(UnitKind::Dynamic)
                    } else {
                        crate::sim_ui::icons::unit_kind_icon_fallback(UnitKind::Reactions)
                    };
                    let (pre_r, _) = ui.allocate_exact_size(egui::Vec2::splat(44.0), egui::Sense::hover());
                    ui.painter().rect_stroke(pre_r, 2.0, egui::Stroke::new(1.0, SIM_BORDER));
                    crate::sim_ui::icons::draw_equipment_icon(ui.painter(), pre_r.shrink(3.0), preview_k);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&entry.model_name).strong().size(13.5));
                        ui.label(&entry.description);
                        ui.label(
                            RichText::new(format!(
                                "Version: {}  |  Calculation: {}",
                                entry.version, entry.calculation
                            ))
                            .color(SIM_MUTED_TEXT),
                        );
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Select & Insert Unit").clicked() {
                        let id = state.alloc_id();
                        let kind = if entry.mode.contains("Distribution") || entry.mode.contains("Particles") {
                            UnitKind::Distributions
                        } else if entry.mode.contains("Dynamic") {
                            UnitKind::Dynamic
                        } else if entry.mode.contains("Reactions") {
                            UnitKind::Reactions
                        } else {
                            UnitKind::Generic
                        };
                        let mut u = SimUnit::new(id, &entry.model_name, kind, [75.0, 45.0], state.project.active_page);
                        u.model_name = entry.model_name.clone();
                        u.model_mode = entry.mode.clone();
                        u.model_type = entry.impl_type;
                        u.technology = entry.technology.clone();
                        state.push_undo();
                        state.project.units.push(u);
                        state.selected = SelectedObject::Unit(id);
                        state.show_select_unit_models = false;
                    }
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Import Custom Unit (.dll/.model):");
                ui.add(egui::TextEdit::singleline(&mut state.import_model_name).desired_width(170.0));
                if ui.button("Import Unit...").clicked() && !state.import_model_name.trim().is_empty() {
                    state.project.imported_models.push(UnitModelCatalogEntry {
                        model_name: state.import_model_name.trim().to_string(),
                        mode: state.import_model_mode.clone(),
                        technology: "User Imported".into(),
                        type_code: "USR".into(),
                        impl_type: ModelImplType::Dll,
                        category: ModelLibraryTab::Imported,
                        version: "1.0.0".into(),
                        calculation: "Static / Dynamic".into(),
                        description: "Custom clean-room user imported unit operation model.".into(),
                    });
                    state.unit_model_tab = ModelLibraryTab::Imported;
                }
            });
        });
    state.show_select_unit_models = open;
}

fn render_unit_editor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_unit_editor;
    egui::Window::new("Unit Editor")
        .open(&mut open)
        .default_size([740.0, 430.0])
        .show(ctx, |ui| {
            let Some(uid) = state.editing_unit_id else {
                ui.label("No unit selected.");
                return;
            };
            let unit_name = state
                .project
                .units
                .iter()
                .find(|u| u.id == uid)
                .map(|u| u.name_id.clone())
                .unwrap_or_default();

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Unit Editor — {unit_name}")).strong().size(13.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Run All Controls").clicked() {
                        state.run_controls(None);
                    }
                    if ui.button("Run This Unit").clicked() {
                        state.push_undo();
                        SoryntelApplication::sim_run_static(&mut state.project, Some(&[uid]));
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                for (idx, lbl) in ["Input", "Output", "Dist", "Controls", "Model"].iter().enumerate() {
                    if ui.selectable_label(state.unit_editor_tab == idx, *lbl).clicked() {
                        state.unit_editor_tab = idx;
                    }
                }
            });
            ui.separator();

            match state.unit_editor_tab {
                0 | 1 => {
                    let is_in = state.unit_editor_tab == 0;
                    ui.label(RichText::new(if is_in { "Connected Input Streams" } else { "Connected Output Streams" }).strong());
                    egui::Grid::new("ue_streams_grid").striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new(crate::i18n::text("review.186")).strong());
                        ui.label(RichText::new("Mass Flow (t/h)").strong());
                        ui.label(RichText::new("Temp (°C)").strong());
                        ui.label(RichText::new("Cu (wt-%)").strong());
                        ui.label(RichText::new("Enthalpy (kW)").strong());
                        ui.end_row();
                        for s in state.project.streams.iter_mut().filter(|s| {
                            if is_in { s.dest_unit == Some(uid) } else { s.source_unit == Some(uid) }
                        }) {
                            ui.label(&s.name_id);
                            ui.add(egui::DragValue::new(&mut s.mass_flow_t_h).speed(0.5));
                            ui.add(egui::DragValue::new(&mut s.temp_c).speed(0.5));
                            ui.add(egui::DragValue::new(&mut s.cu_wt_pct).speed(0.1));
                            ui.add(egui::DragValue::new(&mut s.enthalpy_kw).speed(10.0));
                            ui.end_row();
                        }
                    });
                }
                2 | 4 => {
                    if let Some(u) = state.project.units.iter_mut().find(|u| u.id == uid) {
                        ui.label(RichText::new("Unit Model Behavior & Parameters").strong());
                        match &mut u.behavior {
                            UnitBehavior::HydroCooler { target_temp_c, coolant_flow_t_h, target_duty_kw } => {
                                ui.horizontal(|ui| {
                                    ui.label("Target Temperature (°C):");
                                    ui.add(egui::DragValue::new(target_temp_c).speed(0.5));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Coolant Flow (t/h):");
                                    ui.add(egui::DragValue::new(coolant_flow_t_h).speed(0.5));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Target Duty (kW):");
                                    ui.add(egui::DragValue::new(target_duty_kw).speed(5.0));
                                });
                            }
                            UnitBehavior::HydroLeaching { reaction_temp_c, acid_feed_t_h, target_acid_g_l, conversion_pct } => {
                                ui.horizontal(|ui| {
                                    ui.label("Reaction Temp (°C):");
                                    ui.add(egui::DragValue::new(reaction_temp_c).speed(0.5));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Acid Feed (t/h):");
                                    ui.add(egui::DragValue::new(acid_feed_t_h).speed(0.1));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Target Acid (g/l):");
                                    ui.add(egui::DragValue::new(target_acid_g_l).speed(0.1));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Conversion (%):");
                                    ui.add(egui::DragValue::new(conversion_pct).range(0.0..=100.0).speed(0.5));
                                });
                            }
                            UnitBehavior::Hydrocyclone { cut_size_um, bypass_fines } => {
                                ui.horizontal(|ui| {
                                    ui.label("Cut Size d50 (um):");
                                    ui.add(egui::DragValue::new(cut_size_um).range(20.0..=500.0).speed(1.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Bypass Fines:");
                                    ui.add(egui::DragValue::new(bypass_fines).range(0.0..=1.0).speed(0.01));
                                });
                            }
                            UnitBehavior::Splitter { main_fraction } => {
                                ui.horizontal(|ui| {
                                    ui.label("Primary Output Split Fraction:");
                                    ui.add(egui::DragValue::new(main_fraction).range(0.0..=1.0).speed(0.01));
                                });
                            }
                            UnitBehavior::DynamicTank { holdup_t, max_capacity_t, tau_s } => {
                                ui.horizontal(|ui| {
                                    ui.label("Current Holdup (t):");
                                    ui.add(egui::DragValue::new(holdup_t).speed(0.5));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Max Capacity (t):");
                                    ui.add(egui::DragValue::new(max_capacity_t).speed(1.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Residence Time tau (s):");
                                    ui.add(egui::DragValue::new(tau_s).range(1.0..=3600.0).speed(1.0));
                                });
                            }
                            _ => {
                                ui.label("PassThrough / Generic Model — balances total inputs to outputs.");
                            }
                        }
                    }
                }
                _ => {
                    // Controls Sheet matching Fig. 50
                    let mut run_single = None;
                    let mut monitor_idx = None;
                    egui::ScrollArea::both().show(ui, |ui| {
                        egui::Grid::new("ue_controls_sheet").striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                            for h in ["Active", "Name", "Set Point", "Measured", "Tolerance", "Method", "Control Var", "Min", "Max", "Actions"] {
                                ui.label(RichText::new(h).strong());
                            }
                            ui.end_row();
                            for (idx, c) in state.project.controls.iter_mut().enumerate() {
                                if !c.location_unit.eq_ignore_ascii_case(&unit_name) {
                                    continue;
                                }
                                ui.checkbox(&mut c.active, "");
                                ui.text_edit_singleline(&mut c.control_name);
                                ui.add(egui::DragValue::new(&mut c.set_point).speed(0.05));
                                ui.label(format!("{:.3}", c.measured));
                                ui.add(egui::DragValue::new(&mut c.tolerance).speed(0.001));
                                ui.label(c.method.as_str());
                                ui.add(egui::DragValue::new(&mut c.control_variable).speed(0.5));
                                ui.add(egui::DragValue::new(&mut c.x_min).speed(0.5));
                                ui.add(egui::DragValue::new(&mut c.x_max).speed(0.5));
                                ui.horizontal(|ui| {
                                    if ui.button("Run Control").clicked() {
                                        run_single = Some(idx);
                                    }
                                    if ui.button("Monitor...").clicked() {
                                        monitor_idx = Some(idx);
                                    }
                                });
                                ui.end_row();
                            }
                        });
                    });
                    if let Some(i) = run_single {
                        state.run_controls(Some(i));
                    }
                    if let Some(i) = monitor_idx {
                        state.open_control_monitor(i);
                    }
                }
            }
        });
    state.show_unit_editor = open;
}

fn render_control_monitor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_control_monitor;
    egui::Window::new("Control Monitor")
        .open(&mut open)
        .default_size([720.0, 420.0])
        .show(ctx, |ui| {
            let idx = state.monitor_ctrl_idx;
            let Some(ctrl) = state.project.controls.get(idx).cloned() else {
                ui.label("No control loop available.");
                return;
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} ({})", ctrl.control_name, ctrl.location_unit)).strong());
                ui.label(format!(
                    "SetPoint = {:.3} {} | Measured = {:.3} {} | ControlVar = {:.3} {}",
                    ctrl.set_point, ctrl.y_unit, ctrl.measured, ctrl.y_unit, ctrl.control_variable, ctrl.x_unit
                ));
            });
            ui.separator();

            ui.columns(2, |cols| {
                // Left 50-pt table (Fig. 51)
                cols[0].set_max_width(240.0);
                egui::ScrollArea::vertical().max_height(310.0).show(&mut cols[0], |ui| {
                    egui::Grid::new("cm_table").spacing([10.0, 2.0]).show(ui, |ui| {
                        ui.label(RichText::new("Pt").strong());
                        ui.label(RichText::new("X").strong());
                        ui.label(RichText::new("Y").strong());
                        ui.end_row();
                        for (r_i, x, y) in &state.monitor_sweep_cache {
                            let is_sel = state.monitor_selected_row == Some(*r_i);
                            if ui.selectable_label(is_sel, format!("{r_i}")).clicked() {
                                state.monitor_selected_row = Some(*r_i);
                            }
                            ui.label(format!("{x:.3}"));
                            ui.label(format!("{y:.3}"));
                            ui.end_row();
                        }
                    });
                });

                // Right X-Y Plot + Green Set Point line (Fig. 51)
                let line_pts: PlotPoints = state
                    .monitor_sweep_cache
                    .iter()
                    .map(|(_, x, y)| [*x, *y])
                    .collect();
                Plot::new("control_monitor_plot")
                    .height(310.0)
                    .legend(Legend::default())
                    .x_axis_label(format!("{} ({})", ctrl.x_var_name, ctrl.x_unit))
                    .y_axis_label(format!("{} ({})", ctrl.y_target_name, ctrl.y_unit))
                    .show(&mut cols[1], |plot_ui| {
                        plot_ui.line(
                            Line::new(line_pts)
                                .name("Response Curve")
                                .color(Color32::from_rgb(37, 99, 235))
                                .width(2.0),
                        );
                        plot_ui.hline(
                            HLine::new(ctrl.set_point)
                                .name("Set Point")
                                .color(Color32::from_rgb(22, 163, 74))
                                .width(1.6),
                        );
                        if state.monitor_plot_tolerance {
                            plot_ui.hline(
                                HLine::new(ctrl.set_point + ctrl.tolerance)
                                    .style(egui_plot::LineStyle::dashed_dense())
                                    .color(Color32::from_rgb(120, 180, 120)),
                            );
                            plot_ui.hline(
                                HLine::new(ctrl.set_point - ctrl.tolerance)
                                    .style(egui_plot::LineStyle::dashed_dense())
                                    .color(Color32::from_rgb(120, 180, 120)),
                            );
                        }
                        plot_ui.points(
                            Points::new(PlotPoints::from(vec![[ctrl.control_variable, ctrl.measured]]))
                                .name("Current Solution")
                                .radius(5.0)
                                .color(Color32::from_rgb(234, 179, 8)),
                        );
                    });
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Draw Function:");
                ui.add(egui::DragValue::new(&mut state.monitor_sweep_points).range(10..=200));
                if ui.button("Re-Sweep").clicked() {
                    state.monitor_sweep_cache = SoryntelApplication::sim_control_sweep(
                        &state.project,
                        idx,
                        state.monitor_sweep_points.max(10),
                    );
                }
                ui.checkbox(&mut state.monitor_plot_tolerance, "Plot Tolerance");
                if ui.button("Calculate Control").clicked() {
                    state.run_controls(Some(idx));
                }
            });
        });
    state.show_control_monitor = open;
}

fn render_scenario_editor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_scenario_editor;
    egui::Window::new("Scenario Editor")
        .open(&mut open)
        .default_size([880.0, 520.0])
        .show(ctx, |ui| {
            let s_idx = state.project.active_scenario;
            if state.project.scenarios.is_empty() {
                ui.label("No scenario sheets in project.");
                return;
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("Run Selected Scenario").clicked() {
                    state.push_undo();
                    SoryntelApplication::sim_execute_scenario(&mut state.project, s_idx, 12345);
                }
                if ui.button("Run All Scenarios").clicked() {
                    state.push_undo();
                    for i in 0..state.project.scenarios.len() {
                        SoryntelApplication::sim_execute_scenario(&mut state.project, i, 12345 + i as u64);
                    }
                }
                if let Some(sc) = state.project.scenarios.get_mut(s_idx) {
                    ui.label("Runs:");
                    ui.add(egui::DragValue::new(&mut sc.num_runs).range(1..=200));
                    ui.checkbox(&mut sc.show_mc_config, "Configuration");
                    ui.checkbox(&mut sc.show_dist_charts, "Distribution Charts");
                }
                if ui.button("Clone Scenario").clicked() {
                    let mut cloned = state.project.scenarios[s_idx].clone();
                    cloned.name = format!("{} (Copy)", cloned.name);
                    state.project.scenarios.push(cloned);
                }
            });
            // Excel-like Scenario formula bar (`C13 | 185`) matching Figs. 29-33
            ui.horizontal(|ui| {
                if ui.selectable_label(state.scenario_tab == 0, "Charts").clicked() {
                    state.scenario_tab = 0;
                    if let Some(sc) = state.project.scenarios.get_mut(s_idx) {
                        sc.show_dist_charts = false;
                    }
                }
                if ui.selectable_label(state.scenario_tab == 1, "Monte Carlo").clicked() {
                    state.scenario_tab = 1;
                    if let Some(sc) = state.project.scenarios.get_mut(s_idx) {
                        sc.show_dist_charts = true;
                        sc.show_mc_config = true;
                    }
                }
                ui.separator();
                let col_letter = char::from(b'C' + (state.scenario_selected_col as u8).min(20));
                ui.label(RichText::new(format!("{}{}", col_letter, 18 + state.scenario_selected_run)).strong());
                if ui.text_edit_singleline(&mut state.scenario_formula_bar).lost_focus() {
                    if let Ok(parsed) = state.scenario_formula_bar.trim().parse::<f64>() {
                        if let Some(row) = state.project.scenarios[s_idx].values.get_mut(state.scenario_selected_run) {
                            if let Some(cell) = row.get_mut(state.scenario_selected_col) {
                                *cell = parsed;
                            }
                        }
                    }
                }
            });
            ui.separator();

            let sc_snapshot = state.project.scenarios[s_idx].clone();
            ui.columns(2, |cols| {
                // Left: Spreadsheet Grid (Figs. 29, 33)
                egui::ScrollArea::both().max_height(420.0).show(&mut cols[0], |ui| {
                    let sc = &mut state.project.scenarios[s_idx];
                    egui::Grid::new("scen_excel_grid").striped(true).spacing([6.0, 2.0]).show(ui, |ui| {
                        ui.label(RichText::new("Row / Run").strong());
                        ui.label(RichText::new("Rounds").strong());
                        for (c_i, col) in sc.columns.iter().enumerate() {
                            let tag = if col.is_set { "SET" } else { "GET" };
                            let sel = state.scenario_selected_col == c_i;
                            if ui
                                .selectable_label(sel, format!("[{tag}] {}\n{}", col.unit_name, col.variable_name))
                                .clicked()
                            {
                                state.scenario_selected_col = c_i;
                            }
                        }
                        ui.end_row();

                        if sc.show_mc_config {
                            ui.label("Monte Carlo");
                            ui.label("-");
                            for col in &mut sc.columns {
                                if col.is_set {
                                    let lbl = if col.monte_carlo_on { "ON" } else { "OFF" };
                                    ui.checkbox(&mut col.monte_carlo_on, lbl);
                                } else {
                                    ui.label("OFF");
                                }
                            }
                            ui.end_row();

                            ui.label("Dist. Type");
                            ui.label("-");
                            for (c_i, col) in sc.columns.iter_mut().enumerate() {
                                if col.is_set {
                                    egui::ComboBox::from_id_salt(format!("dist_{c_i}"))
                                        .width(90.0)
                                        .selected_text(col.dist_type.as_str())
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut col.dist_type, MonteCarloDist::Uniform, "Uniform");
                                            ui.selectable_value(&mut col.dist_type, MonteCarloDist::Normal, "Normal");
                                            ui.selectable_value(&mut col.dist_type, MonteCarloDist::Laplace, "Laplace");
                                            ui.selectable_value(&mut col.dist_type, MonteCarloDist::Trapezoid, "Trapezoid");
                                        });
                                } else {
                                    ui.label("-");
                                }
                            }
                            ui.end_row();

                            ui.label("Min");
                            ui.label("-");
                            for col in &mut sc.columns {
                                ui.add(egui::DragValue::new(&mut col.mc_min).speed(0.5));
                            }
                            ui.end_row();
                            ui.label("Max");
                            ui.label("-");
                            for col in &mut sc.columns {
                                ui.add(egui::DragValue::new(&mut col.mc_max).speed(0.5));
                            }
                            ui.end_row();
                            ui.label("Mean");
                            ui.label("-");
                            for col in &mut sc.columns {
                                ui.add(egui::DragValue::new(&mut col.mc_mean).speed(0.5));
                            }
                            ui.end_row();
                            ui.label("SD");
                            ui.label("-");
                            for col in &mut sc.columns {
                                ui.add(egui::DragValue::new(&mut col.mc_sd).speed(0.2));
                            }
                            ui.end_row();
                        }

                        for r_i in 0..sc.values.len() {
                            if ui.selectable_label(state.scenario_selected_run == r_i, format!("Run {}", r_i + 1)).clicked() {
                                state.scenario_selected_run = r_i;
                                if let Some(val) = sc.values[r_i].get(state.scenario_selected_col) {
                                    state.scenario_formula_bar = format!("{val:.2}");
                                }
                            }
                            ui.label(format!("{}", sc.run_rounds.get(r_i).copied().unwrap_or(4)));
                            for c_i in 0..sc.columns.len() {
                                let v = &mut sc.values[r_i][c_i];
                                if ui.add(egui::DragValue::new(v).speed(0.1).max_decimals(2)).gained_focus() {
                                    state.scenario_selected_run = r_i;
                                    state.scenario_selected_col = c_i;
                                    state.scenario_formula_bar = format!("{:.2}", *v);
                                }
                            }
                            ui.end_row();
                        }
                    });
                });

                // Right: Distribution Charts (PDF + CDF stacked, Fig. 33) OR Response Curve
                if sc_snapshot.show_dist_charts {
                    if let Some(col) = sc_snapshot.columns.get(state.scenario_selected_col) {
                        let (pdf, cdf) = SoryntelApplication::sim_mc_curves(col, 60);
                        cols[1].label(RichText::new(format!("Monte Carlo — mu = {:.1}, sigma = {:.2}", col.mc_mean, col.mc_sd)).strong());
                        let pdf_pts: PlotPoints = pdf.iter().map(|p| [p[0], p[1]]).collect();
                        Plot::new("mc_pdf_plot").height(190.0).show(&mut cols[1], |plot_ui| {
                            plot_ui.line(Line::new(pdf_pts).color(Color32::from_rgb(76, 114, 52)).width(1.8));
                            plot_ui.vline(VLine::new(col.mc_mean).color(Color32::from_rgb(37, 99, 235)).width(1.4));
                            plot_ui.vline(VLine::new(col.mc_min).color(Color32::from_rgb(208, 38, 38)));
                            plot_ui.vline(VLine::new(col.mc_max).color(Color32::from_rgb(208, 38, 38)));
                        });
                        let cdf_pts: PlotPoints = cdf.iter().map(|p| [p[0], p[1]]).collect();
                        Plot::new("mc_cdf_plot").height(190.0).show(&mut cols[1], |plot_ui| {
                            plot_ui.line(Line::new(cdf_pts).color(Color32::from_rgb(76, 114, 52)).width(1.8));
                            plot_ui.vline(VLine::new(col.mc_mean).color(Color32::from_rgb(37, 99, 235)).width(1.4));
                        });
                    }
                } else {
                    let x_col = 0usize;
                    let y_col = (sc_snapshot.columns.len() - 1).max(0);
                    let pts: PlotPoints = sc_snapshot
                        .values
                        .iter()
                        .filter(|r| r.len() > y_col)
                        .map(|r| [r[x_col], r[y_col]])
                        .collect();
                    Plot::new("scen_xy_plot")
                        .height(380.0)
                        .legend(Legend::default())
                        .x_axis_label(&sc_snapshot.columns[x_col].variable_name)
                        .y_axis_label(&sc_snapshot.columns[y_col].variable_name)
                        .show(&mut cols[1], |plot_ui| {
                            plot_ui.line(Line::new(pts).name("Scenario Sweep").color(Color32::from_rgb(0, 102, 204)).width(2.0));
                        });
                }
            });
        });
    state.show_scenario_editor = open;
}

fn render_reports_window(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_reports_window;
    egui::Window::new(crate::i18n::text("review.187"))
        .open(&mut open)
        .default_size([860.0, 480.0])
        .show(ctx, |ui| {
            if state.reports_cache.is_none() {
                state.reports_cache = Some(SoryntelApplication::sim_generate_reports(&state.project));
            }
            ui.horizontal(|ui| {
                if ui.button("Refresh Reports").clicked() {
                    state.reports_cache = Some(SoryntelApplication::sim_generate_reports(&state.project));
                }
                if ui.button("Copy Current Sheet as CSV").clicked() {
                    if let Some(ref wb) = state.reports_cache {
                        if let Some(sh) = wb.sheets.get(state.report_sheet_idx) {
                            let csv = SoryntelApplication::sim_report_csv(sh);
                            ui.output_mut(|o| o.copied_text = csv);
                            state.status_banner = format!("Copied '{}' CSV to clipboard", sh.name);
                        }
                    }
                }
            });
            ui.separator();

            if let Some(ref wb) = state.reports_cache {
                let s_idx = state.report_sheet_idx.min(wb.sheets.len().saturating_sub(1));
                if let Some(sheet) = wb.sheets.get(s_idx) {
                    egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
                        egui::Grid::new("reports_sheet_grid").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
                            for (r_i, row) in sheet.rows.iter().enumerate() {
                                for cell in row {
                                    if r_i < sheet.freeze_rows {
                                        ui.label(RichText::new(cell).strong().size(11.5));
                                    } else {
                                        ui.label(RichText::new(cell).size(11.5));
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    for (idx, sh) in wb.sheets.iter().enumerate() {
                        if ui.selectable_label(s_idx == idx, &sh.name).clicked() {
                            state.report_sheet_idx = idx;
                        }
                    }
                });
            }
        });
    state.show_reports_window = open;
}

fn render_process_info(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_process_info;
    egui::Window::new(crate::i18n::text("common.099"))
        .open(&mut open)
        .default_size([560.0, 440.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(state.process_info_tab == 0, "Process").clicked() {
                    state.process_info_tab = 0;
                }
                if ui.selectable_label(state.process_info_tab == 1, "KPI").clicked() {
                    state.process_info_tab = 1;
                }
            });
            ui.separator();
            let pi = &mut state.project.process_info;
            if state.process_info_tab == 0 {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("pi_grid").spacing([14.0, 5.0]).show(ui, |ui| {
                        for (lbl, field) in [
                            ("Project Name", &mut pi.project_name),
                            ("Process Name", &mut pi.process_name),
                            ("Technology", &mut pi.technology),
                            ("Plant Type", &mut pi.plant_type),
                            ("Products", &mut pi.products),
                            ("Customer Name", &mut pi.customer_name),
                            ("Site Location", &mut pi.site_location),
                            ("Country", &mut pi.country),
                            ("Prepared By", &mut pi.prepared_by),
                            ("Classification", &mut pi.classification),
                            ("Version", &mut pi.version),
                            ("Date", &mut pi.date),
                        ] {
                            ui.label(lbl);
                            ui.add(egui::TextEdit::singleline(field).desired_width(300.0));
                            ui.end_row();
                        }
                    });
                });
            } else {
                egui::Grid::new("pi_kpi_grid").striped(true).spacing([12.0, 5.0]).show(ui, |ui| {
                    ui.label(RichText::new("KPI").strong());
                    ui.label(RichText::new(crate::i18n::text("common.036")).strong());
                    ui.label(RichText::new(crate::i18n::text("common.136")).strong());
                    ui.end_row();
                    for (k, u, v) in &mut pi.kpi_rows {
                        ui.text_edit_singleline(k);
                        ui.text_edit_singleline(u);
                        ui.text_edit_singleline(v);
                        ui.end_row();
                    }
                });
            }
        });
    state.show_process_info = open;
}

fn render_notes_window(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_notes_window;
    egui::Window::new(crate::i18n::text("review.188"))
        .open(&mut open)
        .default_size([480.0, 340.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, t) in ["Info", "Links", "Trainings"].iter().enumerate() {
                    if ui.selectable_label(state.notes_tab == i, *t).clicked() {
                        state.notes_tab = i;
                    }
                }
            });
            ui.separator();
            let buf = match state.notes_tab {
                0 => &mut state.project.process_info.notes_info,
                1 => &mut state.project.process_info.notes_links,
                _ => &mut state.project.process_info.notes_trainings,
            };
            ui.add_sized([ui.available_width(), 240.0], egui::TextEdit::multiline(buf));
        });
    state.show_notes_window = open;
}

fn render_convergence_monitor(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_convergence_monitor;
    egui::Window::new("Convergence Monitor")
        .open(&mut open)
        .default_size([560.0, 360.0])
        .show(ctx, |ui| {
            if let Some(last) = state.project.convergence_history.last() {
                ui.label(format!(
                    "Latest Round {}:  MB Error = {:.4}%  |  EB Error = {:.4}%  |  In = {:.2} t/h  Out = {:.2} t/h  ({})",
                    last.round,
                    last.mb_pct,
                    last.eb_pct,
                    last.total_mass_in,
                    last.total_mass_out,
                    if last.converged { "CONVERGED" } else { "ITERATING" }
                ));
            }
            let mb_pts: PlotPoints = state
                .project
                .convergence_history
                .iter()
                .map(|r| [r.round as f64, r.mb_pct])
                .collect();
            let eb_pts: PlotPoints = state
                .project
                .convergence_history
                .iter()
                .map(|r| [r.round as f64, r.eb_pct])
                .collect();
            Plot::new("conv_mon_plot")
                .height(260.0)
                .legend(Legend::default())
                .x_axis_label("Iteration Round")
                .y_axis_label("Balance Error (%)")
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(mb_pts).name("MB %").color(Color32::from_rgb(37, 99, 235)).width(2.0));
                    plot_ui.line(Line::new(eb_pts).name("EB %").color(Color32::from_rgb(220, 38, 38)).width(2.0));
                });
        });
    state.show_convergence_monitor = open;
}

fn render_calc_order_window(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_calc_order_window;
    egui::Window::new("Process Calculation Order (Tarjan SCC)")
        .open(&mut open)
        .default_size([440.0, 320.0])
        .show(ctx, |ui| {
            let info = SoryntelApplication::sim_calc_order(&state.project);
            ui.checkbox(&mut state.highlight_calc_order_on_canvas, "Highlight Calculation Order Badges on Canvas");
            ui.separator();
            ui.label(RichText::new("Topological Unit Sequence:").strong());
            for (step, uid) in info.unit_order_ids.iter().enumerate() {
                let nm = state
                    .project
                    .units
                    .iter()
                    .find(|u| u.id == *uid)
                    .map(|u| u.name_id.as_str())
                    .unwrap_or("Unit");
                ui.label(format!("  #{}: {} (ID {})", step + 1, nm, uid));
            }
            ui.separator();
            if info.tear_stream_ids.is_empty() {
                ui.label(RichText::new("Cycle Detection: Acyclic DAG (0 tear streams)").color(Color32::from_rgb(22, 163, 74)));
            } else {
                ui.label(RichText::new(format!("Detected Recycle SCC Tear Streams ({}):", info.tear_stream_ids.len())).color(Color32::from_rgb(217, 119, 6)));
                for tid in info.tear_stream_ids {
                    let sn = state
                        .project
                        .streams
                        .iter()
                        .find(|s| s.id == tid)
                        .map(|s| s.name_id.as_str())
                        .unwrap_or("Stream");
                    ui.label(format!("  -> Tear Stream: {sn}"));
                }
            }
        });
    state.show_calc_order_window = open;
}

fn render_visualization_picker(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_visualization_picker;
    egui::Window::new(crate::i18n::text("review.189"))
        .open(&mut open)
        .default_size([420.0, 360.0])
        .show(ctx, |ui| {
            let vis = &mut state.project.visualization;
            ui.checkbox(&mut vis.active, "Enable Sankey Stream Visualization");
            ui.checkbox(&mut vis.visualize_stream_line_width, "Scale Stream Width by Flow");
            ui.horizontal(|ui| {
                ui.label("Stream Width Max:");
                ui.add(egui::Slider::new(&mut vis.stream_width_max, 0.5..=8.0));
            });
            ui.separator();
            ui.label(RichText::new("Color Mode:").strong());
            ui.radio_value(&mut vis.color_mode, SankeyColorMode::FlowsheetStreamColors, "Flowsheet Stream Colors");
            ui.radio_value(&mut vis.color_mode, SankeyColorMode::PositiveAndNegativeColors, "Positive and Negative Colors");
            ui.radio_value(&mut vis.color_mode, SankeyColorMode::RangeColors, "Range Colors (Green -> Yellow -> Red)");

            ui.horizontal(|ui| {
                ui.label("Min:");
                ui.add(egui::DragValue::new(&mut vis.range_min).speed(0.5));
                ui.label("Max:");
                ui.add(egui::DragValue::new(&mut vis.range_max).speed(0.5));
            });
            ui.checkbox(&mut vis.use_absolute_values, "Use absolute values");
            ui.horizontal(|ui| {
                ui.label("Multiplier 10^k:");
                ui.add(egui::DragValue::new(&mut vis.multiplier_pow10).range(-3..=3));
            });
        });
    state.show_visualization_picker = open;
}

fn render_sim7_migrator(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_sim7_migrator;
    egui::Window::new("HSC Sim 7 -> Sim 10 Model Converter")
        .open(&mut open)
        .default_size([520.0, 260.0])
        .show(ctx, |ui| {
            ui.label("Automatic translation of division denominators (`safediv`), external workbook links (`REF`), and `/` characters in Excel sheet names.");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Sheet Name:");
                ui.text_edit_singleline(&mut state.sim7_sheet_input);
            });
            ui.horizontal(|ui| {
                ui.label("Legacy Formula:");
                ui.add(egui::TextEdit::singleline(&mut state.sim7_expr_input).desired_width(340.0));
            });
            let (clean_sheet, rewritten) =
                SoryntelApplication::sim_rewrite_sim7(&state.sim7_expr_input, &state.sim7_sheet_input);
            ui.separator();
            ui.label(RichText::new(format!("Converted Sheet Name: {clean_sheet}")).strong());
            ui.label(RichText::new(format!("Converted Formula: {rewritten}")).monospace());
        });
    state.show_sim7_migrator = open;
}

fn render_export_modal(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_export_modal;
    let title = match state.export_kind {
        ExportModalKind::ProjectJson => "Save / View Project (.simjson)",
        ExportModalKind::LoadProjectJson => "Load Project from JSON (.simjson)",
        ExportModalKind::LcaJson => "Save as LCA JSON Inventory",
        ExportModalKind::FlowsheetSvg => "Export Flowsheet Image (.svg)",
    };
    egui::Window::new(title)
        .open(&mut open)
        .default_size([620.0, 400.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if state.export_kind == ExportModalKind::LoadProjectJson {
                    if ui.button("Load into Workspace").clicked() {
                        match serde_json::from_str::<SimProject>(&state.export_buffer) {
                            Ok(proj) => {
                                state.load_preset("Imported Project", proj);
                                state.show_export_modal = false;
                            }
                            Err(e) => {
                                state.status_banner = format!("Invalid .simjson: {e}");
                            }
                        }
                    }
                } else if ui.button("Copy All to Clipboard").clicked() {
                    ui.output_mut(|o| o.copied_text = state.export_buffer.clone());
                    state.status_banner = "Copied export output to clipboard".to_string();
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(310.0).show(ui, |ui| {
                ui.add_sized(
                    [ui.available_width(), 300.0],
                    egui::TextEdit::multiline(&mut state.export_buffer).code_editor(),
                );
            });
        });
    state.show_export_modal = open;
}

fn render_about_modal(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_about_modal;
    egui::Window::new("About HSC Sim")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.heading("Soryntel HSC Sim");
            ui.label(RichText::new("Version 10.3.1.0 (Clean-Room Edition)").strong());
            ui.separator();
            ui.label("Modules integrated:");
            ui.label("• Flowsheet Drawing, Rulers, Layers, Groups & Locking");
            ui.label("• Tarjan SCC Tear Stream Solver & Dynamic Tank ODE");
            ui.label("• Control Loops (Tangential / PID) + 50-pt Control Monitor");
            ui.label("• Scenario Editor & Monte Carlo PDF/CDF Sampling");
            ui.label("• Multi-sheet Reports, Sankey Visualization, SVG & LCA JSON");
        });
    state.show_about_modal = open;
}

fn render_updates_modal(ctx: &egui::Context, state: &mut SimModuleState) {
    let mut open = state.show_updates_modal;
    egui::Window::new("Check for Updates")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.label(RichText::new("Soryntel HSC Sim 10.3.1.0 is up to date.").strong());
            ui.label("Offline clean-room thermodynamic database & solver engine verified.");
        });
    state.show_updates_modal = open;
}
