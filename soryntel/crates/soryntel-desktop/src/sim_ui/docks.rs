use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::model::*;
use soryntel_thermo::sim::presets::UNIT_ICON_GALLERY;
use soryntel_thermo::sim::solver::LogFilterLevel;
use crate::sim_ui::icons::draw_equipment_icon;
use crate::sim_ui::state::*;

fn prop_row_readonly(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(132.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(11.5).color(SIM_MUTED_TEXT));
            },
        );
        ui.label(RichText::new(value).size(11.5).color(SIM_MUTED_TEXT));
    });
}

fn prop_row_text(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(132.0, 19.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(11.5).color(SIM_TEXT_DARK));
            },
        );
        changed = ui
            .add_sized([ui.available_width().max(70.0), 18.0], egui::TextEdit::singleline(value))
            .changed();
    });
    changed
}

fn prop_row_rgb(ui: &mut egui::Ui, label: &str, rgb: &mut [u8; 3]) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(132.0, 19.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(11.5).color(SIM_TEXT_DARK));
            },
        );
        let mut c32 = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        if ui.color_edit_button_srgba(&mut c32).changed() {
            *rgb = [c32.r(), c32.g(), c32.b()];
        }
        ui.label(
            RichText::new(format!("{}; {}; {}", rgb[0], rgb[1], rgb[2]))
                .size(11.0)
                .color(SIM_TEXT_DARK),
        );
    });
}

pub fn render_right_docks(ui: &mut egui::Ui, state: &mut SimModuleState) {
    let total_h = ui.available_height();
    let top_h = if state.show_right_top_dock && state.show_right_bottom_dock {
        (total_h * 0.44).max(190.0)
    } else if state.show_right_top_dock {
        total_h
    } else {
        0.0
    };

    if state.show_right_top_dock {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, SIM_BORDER))
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.set_min_height(top_h - 6.0);
                ui.set_max_height(top_h - 6.0);
                render_right_top_dock(ui, state);
            });
    }

    if state.show_right_bottom_dock {
        ui.add_space(3.0);
        egui::Frame::none()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, SIM_BORDER))
            .inner_margin(4.0)
            .show(ui, |ui| {
                render_right_bottom_dock(ui, state);
            });
    }
}

fn render_right_top_dock(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(if state.right_top_tab == RightTopDockTab::Properties {
            "Properties"
        } else {
            "Inspector"
        }).strong().size(12.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(state.right_top_tab == RightTopDockTab::Inspector, "Inspector")
                .clicked()
            {
                state.right_top_tab = RightTopDockTab::Inspector;
            }
            if ui
                .selectable_label(state.right_top_tab == RightTopDockTab::Properties, "Properties")
                .clicked()
            {
                state.right_top_tab = RightTopDockTab::Properties;
            }
        });
    });
    ui.separator();

    match state.right_top_tab {
        RightTopDockTab::Properties => render_properties_pane(ui, state),
        RightTopDockTab::Inspector => render_inspector_pane(ui, state),
    }
}

fn render_properties_pane(ui: &mut egui::Ui, state: &mut SimModuleState) {
    match state.selected {
        SelectedObject::Unit(uid) => {
            ui.horizontal(|ui| {
                for (idx, lbl) in ["Process", "Drawings"].iter().enumerate() {
                    if ui.selectable_label(state.prop_subtab_selected == idx, *lbl).clicked() {
                        state.prop_subtab_selected = idx;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("prop_unit_scroll").show(ui, |ui| {
                let mut open_editor = false;
                if let Some(u) = state.project.units.iter_mut().find(|u| u.id == uid) {
                    if state.prop_subtab_selected == 0 {
                        // Fig. 63 right: Properties - Unit Process
                        egui::CollapsingHeader::new("General").default_open(true).show(ui, |ui| {
                            prop_row_readonly(ui, "Object Type", "Unit");
                            prop_row_text(ui, "NameID", &mut u.name_id);
                            prop_row_text(ui, "Number/Alias", &mut u.alias);
                            prop_row_readonly(ui, "Mode", &u.model_mode);
                            prop_row_readonly(ui, "Type", u.model_type.as_str());
                            prop_row_readonly(ui, "Model", &u.model_name);
                            prop_row_text(ui, "Technology", &mut u.technology);
                            prop_row_text(ui, "SubTechnology", &mut u.sub_technology);
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(Vec2::new(132.0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.label(RichText::new("Availability%").size(11.5));
                                });
                                ui.add(egui::DragValue::new(&mut u.availability_pct).range(0.0..=100.0).speed(0.5));
                            });
                        });
                        egui::CollapsingHeader::new("Location").default_open(true).show(ui, |ui| {
                            prop_row_readonly(ui, "Page", &format!("Page{}", u.page + 1));
                            prop_row_readonly(ui, "Layer", &format!("Layer {}", u.layer));
                        });
                        egui::CollapsingHeader::new("Visibility").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut u.visible_unit, "Unit");
                            ui.checkbox(&mut u.visible_name_label, "Name Label");
                            ui.checkbox(&mut u.show_title_inside, "Show Title Inside Unit");
                        });
                        ui.add_space(4.0);
                        if ui.button(crate::i18n::text("review.182")).clicked() {
                            open_editor = true;
                        }
                    } else {
                        // Fig. 63 left: Properties - Unit Drawings
                        egui::CollapsingHeader::new("Image").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut u.show_unit_image, "Show Unit Image");
                            prop_row_readonly(ui, "Unit Image", u.icon_key.as_deref().unwrap_or("(none)"));
                        });
                        egui::CollapsingHeader::new("Background").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut u.transparent_bg, "Transparent");
                        });
                        egui::CollapsingHeader::new("Drawing - InTitle").default_open(true).show(ui, |ui| {
                            prop_row_text(ui, "Font Name", &mut u.font_name);
                            ui.horizontal(|ui| {
                                ui.label("Font size:");
                                ui.add(egui::DragValue::new(&mut u.font_size).range(6.0..=24.0).speed(0.25));
                            });
                            ui.checkbox(&mut u.font_bold, "Font Bold");
                            ui.checkbox(&mut u.font_italic, "Font Italic");
                            prop_row_rgb(ui, "Font Color", &mut u.font_color);
                        });
                        egui::CollapsingHeader::new("Drawing - Line & Fill").default_open(true).show(ui, |ui| {
                            prop_row_rgb(ui, "Line Color", &mut u.line_color);
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("review.185"));
                                ui.add(egui::DragValue::new(&mut u.line_width).range(0.1..=3.0).speed(0.05));
                            });
                            prop_row_rgb(ui, "Fill Color 1", &mut u.fill_color_1);
                            prop_row_rgb(ui, "Fill Color 2", &mut u.fill_color_2);
                        });
                    }
                }
                if open_editor {
                    state.open_unit_editor(uid);
                }
            });
        }
        SelectedObject::Stream(sid) => {
            ui.horizontal(|ui| {
                for (idx, lbl) in ["Process", "Drawings"].iter().enumerate() {
                    if ui.selectable_label(state.prop_subtab_selected == idx, *lbl).clicked() {
                        state.prop_subtab_selected = idx;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("prop_stream_scroll").show(ui, |ui| {
                let mut open_src_dst = false;
                if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                    if state.prop_subtab_selected == 0 {
                        // Fig. 64 right: Stream Process
                        egui::CollapsingHeader::new("General").default_open(true).show(ui, |ui| {
                            prop_row_readonly(ui, "Object Type", "Stream");
                            prop_row_text(ui, "NameID", &mut s.name_id);
                            prop_row_text(ui, "Number/Alias", &mut s.alias);
                            prop_row_readonly(
                                ui,
                                "Connectivity",
                                match s.connectivity_kind() {
                                    StreamConnectivity::Input => "Input (Feed)",
                                    StreamConnectivity::Internal => "Internal",
                                    StreamConnectivity::Output => "Output (Product)",
                                },
                            );
                            ui.horizontal(|ui| {
                                ui.label("Mass Flow (t/h):");
                                ui.add(egui::DragValue::new(&mut s.mass_flow_t_h).speed(0.5));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Temperature (°C):");
                                ui.add(egui::DragValue::new(&mut s.temp_c).speed(0.5));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Pressure (bar):");
                                ui.add(egui::DragValue::new(&mut s.pressure_bar).speed(0.1));
                            });
                        });
                        egui::CollapsingHeader::new("Visibility").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut s.visible_stream, "Stream");
                            ui.checkbox(&mut s.visible_name_label, "Stream Name Label");
                            ui.checkbox(&mut s.visible_stream_table, "Stream Table");
                            ui.checkbox(&mut s.has_duplicates, "Has Duplicates Warning");
                        });
                        if ui.button(crate::i18n::text("review.183")).clicked() {
                            open_src_dst = true;
                        }
                    } else {
                        // Fig. 64 left: Stream Drawings
                        egui::CollapsingHeader::new("Drawing - Line").default_open(true).show(ui, |ui| {
                            prop_row_rgb(ui, "Line Color", &mut s.line_color);
                            ui.horizontal(|ui| {
                                ui.label(crate::i18n::text("review.185"));
                                ui.add(egui::DragValue::new(&mut s.line_width).range(0.1..=3.0).speed(0.05));
                            });
                        });
                        egui::CollapsingHeader::new("Drawing - Arrow").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut s.auto_arrow_update, "Automatic Arrow Update");
                            prop_row_rgb(ui, "Arrow Color", &mut s.arrow_color);
                            egui::ComboBox::from_label("End Style")
                                .selected_text(format!("{:?}", s.end_arrow_style))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut s.end_arrow_style, ArrowHeadStyle::None, "None");
                                    ui.selectable_value(&mut s.end_arrow_style, ArrowHeadStyle::Triangle, "Triangle");
                                });
                        });
                    }
                }
                if open_src_dst {
                    state.open_stream_src_dest(sid);
                }
            });
        }
        _ => {
            // Empty canvas selected: Process | KPI | Page | Arrowheads (Startup screenshot!)
            ui.horizontal(|ui| {
                for (idx, lbl) in ["Process", "KPI", "Page", "Arrowheads"].iter().enumerate() {
                    if ui.selectable_label(state.prop_subtab_idle == idx, *lbl).clicked() {
                        state.prop_subtab_idle = idx;
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("prop_idle_scroll").show(ui, |ui| {
                match state.prop_subtab_idle {
                    0 => {
                        egui::CollapsingHeader::new("Project").default_open(true).show(ui, |ui| {
                            prop_row_text(ui, "Project Name", &mut state.project.process_info.project_name);
                            prop_row_text(ui, "Project Path", &mut state.project.process_info.project_path);
                        });
                        egui::CollapsingHeader::new("Process").default_open(true).show(ui, |ui| {
                            prop_row_text(ui, "Name", &mut state.project.process_info.process_name);
                            prop_row_text(ui, "Technology", &mut state.project.process_info.technology);
                            prop_row_text(ui, "Sub Technology", &mut state.project.process_info.sub_technology);
                            prop_row_text(ui, "Plant Type", &mut state.project.process_info.plant_type);
                            prop_row_text(ui, "Products", &mut state.project.process_info.products);
                            prop_row_text(ui, "More Information", &mut state.project.process_info.more_information);
                        });
                    }
                    1 => {
                        for (k, u, v) in &mut state.project.process_info.kpi_rows {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(k.as_str()).strong().size(11.5));
                                ui.add(egui::TextEdit::singleline(v).desired_width(60.0));
                                ui.label(RichText::new(u.as_str()).size(11.0).color(SIM_MUTED_TEXT));
                            });
                        }
                        if ui.button("Edit All in Process Information...").clicked() {
                            state.process_info_tab = 1;
                            state.show_process_info = true;
                        }
                    }
                    2 => {
                        prop_row_rgb(ui, "Page Background Color", &mut state.project.canvas_settings.page_bg_rgb);
                        ui.checkbox(&mut state.project.canvas_settings.show_grid, "Show Grid");
                        prop_row_rgb(ui, "Grid Color", &mut state.project.canvas_settings.grid_color_rgb);
                        ui.horizontal(|ui| {
                            ui.label("Grid Width / Height:");
                            ui.add(egui::DragValue::new(&mut state.project.canvas_settings.grid_width).range(1.0..=20.0));
                            ui.add(egui::DragValue::new(&mut state.project.canvas_settings.grid_height).range(1.0..=20.0));
                        });
                    }
                    _ => {
                        prop_row_readonly(ui, "Default Feed Start", "Circle (Input)");
                        prop_row_readonly(ui, "Default End Arrow", "Triangle (Size 3)");
                        prop_row_readonly(ui, "Connectivity Rule", "Auto Input/Internal/Output");
                    }
                }
            });
        }
    }
}

fn render_inspector_pane(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        for (idx, lbl) in ["Phases", "Hydrometallurgy", "Stream Properties"].iter().enumerate() {
            if ui.selectable_label(state.inspector_subtab == idx, *lbl).clicked() {
                state.inspector_subtab = idx;
            }
        }
    });
    ui.separator();

    let stream = match state.selected {
        SelectedObject::Stream(sid) => state.project.streams.iter().find(|s| s.id == sid),
        _ => state.project.streams.first(),
    };

    egui::ScrollArea::vertical().id_salt("inspector_scroll").show(ui, |ui| {
        if let Some(s) = stream {
            ui.label(RichText::new(format!("Stream: {}  ({:.2} t/h)", s.name_id, s.mass_flow_t_h)).strong());
            ui.add_space(4.0);
            match state.inspector_subtab {
                0 => {
                    // Pastel Phase Headers (Fig. 66-67)
                    draw_phase_band(ui, "Gas Phase", Color32::from_rgb(246, 206, 196), &format!("{:.2} Nm3/h", s.gas_flow_nm3_h), s, StreamPhaseKind::Gas);
                    draw_phase_band(ui, "Water Phase", Color32::from_rgb(204, 232, 250), &format!("{:.2} t/h", s.liquid_flow_t_h), s, StreamPhaseKind::Water);
                    draw_phase_band(ui, "Pure Phase", Color32::from_rgb(242, 236, 206), &format!("{:.2} t/h", s.solids_flow_t_h), s, StreamPhaseKind::PureSolid);
                }
                1 => {
                    prop_row_readonly(ui, "Solids Weight %", &format!("{:.2} wt-%", s.pct_solids));
                    prop_row_readonly(ui, "Pulp Vol. Flow", &format!("{:.2} m3/h", s.pulp_vol_m3_h));
                    prop_row_readonly(ui, "Solids SG", &format!("{:.2} g/cm3", s.solids_sg));
                    prop_row_readonly(ui, "Liquid SG", &format!("{:.2} g/cm3", s.liquid_sg));
                    prop_row_readonly(ui, "Pulp SG", &format!("{:.2} g/cm3", s.pulp_sg));
                    prop_row_readonly(ui, "Cu wt-% / Rec%", &format!("{:.2}% / {:.2}%", s.cu_wt_pct, s.cu_rec_pct));
                    prop_row_readonly(ui, "P80 / P50", &format!("{:.1} / {:.1} um", s.p80_um, s.p50_um));
                }
                _ => {
                    prop_row_readonly(ui, "Temperature", &format!("{:.2} °C", s.temp_c));
                    prop_row_readonly(ui, "Pressure", &format!("{:.2} bar", s.pressure_bar));
                    prop_row_readonly(ui, "Total Enthalpy", &format!("{:.2} kW", s.enthalpy_kw));
                    prop_row_readonly(ui, "Total Exergy", &format!("{:.2} kW", s.exergy_kw));
                    prop_row_readonly(ui, "Cp", &format!("{:.3} kJ/(kg*K)", s.cp_kj_kg_k));
                }
            }
        } else {
            ui.label(RichText::new("Select any stream on the flowsheet to inspect phases and compositions.").color(SIM_MUTED_TEXT));
        }
    });
}

fn draw_phase_band(
    ui: &mut egui::Ui,
    title: &str,
    bg: Color32,
    summary: &str,
    stream: &SimStream,
    kind: StreamPhaseKind,
) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(0.8, SIM_BORDER))
        .inner_margin(3.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong().size(11.5).color(Color32::BLACK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(summary).size(11.0).color(Color32::BLACK));
                });
            });
        });
    let matching: Vec<_> = stream.species_flows.iter().filter(|sp| sp.phase == kind).collect();
    for sp in matching {
        ui.horizontal(|ui| {
            ui.label(format!("  {}  ->  {:.2} t/h  ({:.1} kW)", sp.formula, sp.mass_flow_t_h, sp.enthalpy_kw));
        });
    }
}

fn render_right_bottom_dock(ui: &mut egui::Ui, state: &mut SimModuleState) {
    let tab_title = match state.right_bottom_tab {
        RightBottomTab::Icons => "Icons",
        RightBottomTab::Items => "Items",
        RightBottomTab::Names => "Names",
        RightBottomTab::Visibility => "Visibility",
        RightBottomTab::Controls => "Controls",
        RightBottomTab::CellRef => "Cell Ref.",
        RightBottomTab::Log => "Log",
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tab_title).strong().size(12.0));
    });
    ui.horizontal_wrapped(|ui| {
        for (tab, lbl) in [
            (RightBottomTab::Icons, "Icons"),
            (RightBottomTab::Items, "Items"),
            (RightBottomTab::Names, "Names"),
            (RightBottomTab::Visibility, "Visibility"),
            (RightBottomTab::Controls, "Controls"),
            (RightBottomTab::CellRef, "Cell Ref."),
            (RightBottomTab::Log, "Log"),
        ] {
            if ui.selectable_label(state.right_bottom_tab == tab, lbl).clicked() {
                state.right_bottom_tab = tab;
            }
        }
    });
    ui.separator();

    match state.right_bottom_tab {
        RightBottomTab::Icons => render_icons_tab(ui, state),
        RightBottomTab::Items => render_items_tab(ui, state),
        RightBottomTab::Names => render_names_tab(ui, state),
        RightBottomTab::Visibility => render_visibility_tab(ui, state),
        RightBottomTab::Controls => render_controls_tab(ui, state),
        RightBottomTab::CellRef => render_cell_ref_tab(ui, state),
        RightBottomTab::Log => render_log_tab(ui, state),
    }
}

fn render_icons_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("icons_cat")
            .width(120.0)
            .selected_text(&state.icons_filter_category)
            .show_ui(ui, |ui| {
                for opt in ["Units by Type", "All", "Hydro", "Particles", "Pyro"] {
                    ui.selectable_value(&mut state.icons_filter_category, opt.to_string(), opt);
                }
            });
        ui.add_sized(
            [ui.available_width().max(80.0), 20.0],
            egui::TextEdit::singleline(&mut state.icons_search).hint_text("Search for units"),
        );
    });
    ui.add_space(4.0);

    let thumb = state.icons_thumb_size;
    let q = state.icons_search.to_lowercase();
    let cat = state.icons_filter_category.as_str();
    let matches: Vec<(usize, &soryntel_thermo::sim::presets::UnitIconMeta)> = UNIT_ICON_GALLERY
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            (cat == "Units by Type" || cat == "All" || m.category == cat)
                && (q.is_empty() || m.label.to_lowercase().contains(&q) || m.key.contains(&q))
        })
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("icons_gallery_scroll")
        .max_height((ui.available_height() - 28.0).max(120.0))
        .show(ui, |ui| {
            let cols = ((ui.available_width() / (thumb + 8.0)).floor() as usize).clamp(3, 6);
            egui::Grid::new("icons_gallery_grid").spacing([6.0, 6.0]).show(ui, |ui| {
                for (cell_i, (orig_idx, meta)) in matches.iter().enumerate() {
                    let selected = state.active_tool == ActiveTool::PlaceGalleryIcon(*orig_idx);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(thumb), egui::Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        if selected { Color32::from_rgb(218, 234, 255) } else { Color32::from_rgb(248, 250, 252) },
                    );
                    ui.painter().rect_stroke(
                        rect,
                        2.0,
                        if selected {
                            Stroke::new(1.5, SIM_ACCENT_BLUE)
                        } else {
                            Stroke::new(0.8, SIM_BORDER)
                        },
                    );
                    draw_equipment_icon(ui.painter(), rect.shrink(3.0), meta.key);

                    if resp.clicked() {
                        state.active_tool = ActiveTool::PlaceGalleryIcon(*orig_idx);
                        state.status_banner = format!("Click on flowsheet canvas to place '{}'", meta.label);
                    }
                    if resp.double_clicked() {
                        let id = state.alloc_id();
                        let mut u = SimUnit::new(id, meta.label, meta.default_kind, [65.0, 40.0], state.project.active_page);
                        u.icon_key = Some(meta.key.to_string());
                        state.push_undo();
                        state.project.units.push(u);
                        state.selected = SelectedObject::Unit(id);
                    }
                    resp.on_hover_text(format!("{} ({})\nClick then click canvas, or double-click to insert", meta.label, meta.category));
                    if (cell_i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }
            });
        });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add(egui::Slider::new(&mut state.icons_thumb_size, 36.0..=68.0).show_value(false));
    });
}

fn render_items_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        if ui.button("Navigate to Diagram").clicked() {
            match state.selected {
                SelectedObject::Unit(uid) => {
                    if let Some(u) = state.project.units.iter().find(|u| u.id == uid) {
                        state.project.active_page = u.page;
                        state.pan_mm = [u.pos[0] - 40.0, u.pos[1] - 30.0];
                    }
                }
                SelectedObject::Stream(sid) => {
                    if let Some(s) = state.project.streams.iter().find(|s| s.id == sid) {
                        state.project.active_page = s.page;
                        state.pan_mm = [s.source_anchor[0] - 40.0, s.source_anchor[1] - 30.0];
                    }
                }
                _ => {}
            }
        }
        ui.add_sized([ui.available_width().max(70.0), 19.0], egui::TextEdit::singleline(&mut state.items_search));
    });
    ui.separator();

    let q = state.items_search.to_lowercase();
    egui::ScrollArea::vertical()
        .id_salt("items_tree_scroll")
        .max_height((ui.available_height() - 26.0).max(120.0))
        .show(ui, |ui| {
            let title = if state.project.process_info.project_name.is_empty() {
                "Process"
            } else {
                state.project.process_info.project_name.as_str()
            };
            egui::CollapsingHeader::new(title).default_open(true).show(ui, |ui| {
                egui::CollapsingHeader::new(format!("Units ({})", state.project.units.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        let units: Vec<(usize, String, &'static str)> = state
                            .project
                            .units
                            .iter()
                            .filter(|u| q.is_empty() || u.name_id.to_lowercase().contains(&q))
                            .map(|u| (u.id, u.name_id.clone(), u.kind.badge()))
                            .collect();
                        for (uid, nm, badge) in units {
                            let sel = state.selected == SelectedObject::Unit(uid);
                            if ui.selectable_label(sel, format!("[{badge}]  {nm}")).clicked() {
                                state.selected = SelectedObject::Unit(uid);
                            }
                        }
                    });
                egui::CollapsingHeader::new(format!("Streams ({})", state.project.streams.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        let streams: Vec<(usize, String)> = state
                            .project
                            .streams
                            .iter()
                            .filter(|s| q.is_empty() || s.name_id.to_lowercase().contains(&q))
                            .map(|s| (s.id, s.name_id.clone()))
                            .collect();
                        for (sid, nm) in streams {
                            let sel = state.selected == SelectedObject::Stream(sid);
                            if ui.selectable_label(sel, format!("  ->  {nm}")).clicked() {
                                state.selected = SelectedObject::Stream(sid);
                            }
                        }
                    });
            });
        });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(format!(
            "Layers: {}    |    Units: {}    |    Streams: {}",
            state.project.groups.len(),
            state.project.units.len(),
            state.project.streams.len()
        ));
    });
}

fn render_names_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.add(egui::TextEdit::singleline(&mut state.names_search).hint_text("Filter names..."));
    ui.separator();
    let q = state.names_search.to_lowercase();
    egui::ScrollArea::vertical().id_salt("names_scroll").show(ui, |ui| {
        egui::Grid::new("names_grid").striped(true).show(ui, |ui| {
            ui.label(RichText::new("Type").strong());
            ui.label(RichText::new("NameID").strong());
            ui.label(RichText::new("Alias").strong());
            ui.end_row();
            for u in &mut state.project.units {
                if !q.is_empty() && !u.name_id.to_lowercase().contains(&q) {
                    continue;
                }
                ui.label(u.kind.badge());
                ui.add(egui::TextEdit::singleline(&mut u.name_id).desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut u.alias).desired_width(90.0));
                ui.end_row();
            }
            for s in &mut state.project.streams {
                if !q.is_empty() && !s.name_id.to_lowercase().contains(&q) {
                    continue;
                }
                ui.label(crate::i18n::text("review.186"));
                ui.add(egui::TextEdit::singleline(&mut s.name_id).desired_width(110.0));
                ui.add(egui::TextEdit::singleline(&mut s.alias).desired_width(90.0));
                ui.end_row();
            }
        });
    });
}

fn render_visibility_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        for (idx, lbl) in ["Layers", "Groups", "Pages"].iter().enumerate() {
            if ui.selectable_label(state.visibility_subtab == idx, *lbl).clicked() {
                state.visibility_subtab = idx;
            }
        }
    });
    ui.separator();

    match state.visibility_subtab {
        0 => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.new_layer_name).desired_width(115.0));
                if ui.button(crate::i18n::text("common.015")).clicked() && !state.new_layer_name.trim().is_empty() {
                    state.project.layers.push(SimLayer {
                        name: state.new_layer_name.trim().to_string(),
                        visible: true,
                    });
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("vis_layers").show(ui, |ui| {
                for layer in &mut state.project.layers {
                    ui.checkbox(&mut layer.visible, &layer.name);
                }
            });
        }
        1 => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.new_group_name).desired_width(115.0));
                if ui.button(crate::i18n::text("common.015")).clicked() && !state.new_group_name.trim().is_empty() {
                    let unit_ids = state.multi_selected_units.clone();
                    state.project.groups.push(SimGroup {
                        name: state.new_group_name.trim().to_string(),
                        visible: true,
                        unit_ids,
                        stream_ids: Vec::new(),
                    });
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("vis_groups").show(ui, |ui| {
                for g in &mut state.project.groups {
                    ui.checkbox(&mut g.visible, format!("{} ({} units)", g.name, g.unit_ids.len()));
                }
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.new_page_name).desired_width(115.0));
                if ui.button("Add Page").clicked() && !state.new_page_name.trim().is_empty() {
                    state.project.pages.push(SimPage {
                        name: state.new_page_name.trim().to_string(),
                        visible: true,
                    });
                }
            });
            ui.separator();
            for p in &mut state.project.pages {
                ui.checkbox(&mut p.visible, &p.name);
            }
        }
    }
}

fn render_controls_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal_wrapped(|ui| {
        ui.add_sized([115.0, 19.0], egui::TextEdit::singleline(&mut state.controls_search).hint_text("Search Controls"));
        if ui.button("Control ON").clicked() {
            for c in &mut state.project.controls {
                c.active = true;
            }
        }
        if ui.button("Control OFF").clicked() {
            for c in &mut state.project.controls {
                c.active = false;
            }
        }
        if ui.button("Run Controls").clicked() {
            state.run_controls(None);
        }
    });
    ui.separator();

    let q = state.controls_search.to_lowercase();
    let mut open_idx = None;
    egui::ScrollArea::both().id_salt("controls_table_scroll").show(ui, |ui| {
        egui::Grid::new("controls_grid").striped(false).spacing([8.0, 3.0]).show(ui, |ui| {
            for hdr in ["Control Name", "Location", "Active", "Set Point", "Measured", "Control Variable", "Type"] {
                ui.label(RichText::new(hdr).strong().size(11.0));
            }
            ui.end_row();
            for (idx, c) in state.project.controls.iter_mut().enumerate() {
                if !q.is_empty()
                    && !c.control_name.to_lowercase().contains(&q)
                    && !c.location_unit.to_lowercase().contains(&q)
                {
                    continue;
                }
                let in_tol = (c.measured - c.set_point).abs() <= c.tolerance.max(0.05);
                let pill_bg = if idx == 0 || !in_tol {
                    Color32::from_rgb(248, 212, 212) // pink row (Fig. 62)
                } else {
                    Color32::from_rgb(220, 240, 214) // mint green row (Fig. 62)
                };
                egui::Frame::none().fill(pill_bg).inner_margin(2.0).show(ui, |ui| {
                    if ui.selectable_label(false, &c.control_name).clicked() {
                        open_idx = Some(idx);
                    }
                });
                ui.label(&c.location_unit);
                let mut active = c.active;
                if ui.selectable_label(active, if active { "ON" } else { "OFF" }).clicked() {
                    active = !active;
                    c.active = active;
                }
                ui.add(egui::DragValue::new(&mut c.set_point).speed(0.05));
                ui.label(format!("{:.3}", c.measured));
                ui.label(format!("{:.3}", c.control_variable));
                ui.label(&c.control_type);
                ui.end_row();
            }
        });
    });
    if let Some(idx) = open_idx {
        state.open_control_monitor(idx);
    }
}

fn render_cell_ref_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        ui.add_sized([150.0, 19.0], egui::TextEdit::singleline(&mut state.cell_ref_search).hint_text("Search Cell Refs"));
        if ui.button("Sort").clicked() {
            state.project.cell_references.sort_by(|a, b| a.ref_name.cmp(&b.ref_name));
        }
    });
    ui.separator();
    let q = state.cell_ref_search.to_lowercase();
    let mut nav_unit: Option<String> = None;
    egui::ScrollArea::vertical().id_salt("cellref_scroll").show(ui, |ui| {
        egui::Grid::new("cellref_grid").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
            ui.label(RichText::new(crate::i18n::text("common.096")).strong());
            ui.label(RichText::new("Cell Reference Name").strong());
            ui.label(RichText::new(crate::i18n::text("common.136")).strong());
            ui.end_row();
            for cr in &state.project.cell_references {
                if !q.is_empty()
                    && !cr.ref_name.to_lowercase().contains(&q)
                    && !cr.unit_name.to_lowercase().contains(&q)
                {
                    continue;
                }
                ui.label(&cr.unit_name);
                if ui.selectable_label(false, &cr.ref_name).clicked() {
                    nav_unit = Some(cr.unit_name.clone());
                }
                ui.label(format!("{:.6}", cr.value));
                ui.end_row();
            }
        });
    });
    if let Some(unm) = nav_unit {
        if let Some(u) = state.project.units.iter().find(|u| u.name_id == unm) {
            let id = u.id;
            state.selected = SelectedObject::Unit(id);
            state.open_unit_editor(id);
        }
    }
}

fn render_log_tab(ui: &mut egui::Ui, state: &mut SimModuleState) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(state.log_filter_level == LogFilterLevel::All, "All")
            .clicked()
        {
            state.log_filter_level = LogFilterLevel::All;
        }
        if ui
            .selectable_label(
                state.log_filter_level == LogFilterLevel::WarningsAndErrors,
                "Warnings & Errors",
            )
            .clicked()
        {
            state.log_filter_level = LogFilterLevel::WarningsAndErrors;
        }
        if ui
            .selectable_label(state.log_filter_level == LogFilterLevel::Errors, "Errors")
            .clicked()
        {
            state.log_filter_level = LogFilterLevel::Errors;
        }
        if ui.button(crate::i18n::text("common.037")).clicked() {
            state.project.logs.clear();
        }
    });
    ui.add_sized(
        [ui.available_width().max(120.0), 20.0],
        egui::TextEdit::singleline(&mut state.log_search_query)
            .hint_text("Enter text to search. Supports AND, OR and NOT operators."),
    );
    ui.separator();

    egui::ScrollArea::vertical().id_salt("log_tree_scroll").show(ui, |ui| {
        for entry in &state.project.logs {
            let show = SoryntelApplication::sim_filter_log(
                &entry.message,
                entry.severity,
                state.log_filter_level,
                &state.log_search_query,
            );
            if !show {
                continue;
            }
            let (badge, col) = match entry.severity {
                LogSeverity::Info => ("[i]", Color32::from_rgb(37, 99, 235)),
                LogSeverity::Success => ("[ok]", Color32::from_rgb(22, 163, 74)),
                LogSeverity::Warning => ("[!]", Color32::from_rgb(217, 119, 6)),
                LogSeverity::Error => ("[X]", Color32::from_rgb(220, 38, 38)),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(badge).strong().color(col));
                ui.label(format!("{}  ({:.2} s)", entry.message, entry.duration_s));
            });
            for ch in &entry.children {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(format!("|- {}  ({:.2} s)", ch.message, ch.duration_s));
                });
            }
        }
    });
}
