use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::sim::model::*;
use soryntel_thermo::sim::presets::{UNIT_ICON_GALLERY};
use crate::sim_ui::icons::draw_equipment_icon;
use crate::sim_ui::state::*;

const MM_TO_PX_BASE: f32 = 3.8;

fn to_color32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

fn snap(val: f32, enabled: bool, step: f32) -> f32 {
    if !enabled || step <= 0.0 {
        val
    } else {
        (val / step).round() * step
    }
}

fn mm_to_screen(mm: [f32; 2], origin: Pos2, pan_mm: [f32; 2], scale: f32) -> Pos2 {
    Pos2::new(
        origin.x + (mm[0] - pan_mm[0]) * scale,
        origin.y + (mm[1] - pan_mm[1]) * scale,
    )
}

fn screen_to_mm(pos: Pos2, origin: Pos2, pan_mm: [f32; 2], scale: f32) -> [f32; 2] {
    [
        (pos.x - origin.x) / scale + pan_mm[0],
        (pos.y - origin.y) / scale + pan_mm[1],
    ]
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn compute_stream_points_mm(
    s: &SimStream,
    units: &[SimUnit],
    orthogonal: bool,
) -> Vec<[f32; 2]> {
    let start = if let Some(uid) = s.source_unit {
        if let Some(u) = units.iter().find(|u| u.id == uid) {
            [u.pos[0] + u.size[0], u.pos[1] + u.size[1] * 0.5]
        } else {
            s.source_anchor
        }
    } else {
        s.source_anchor
    };

    let end = if let Some(uid) = s.dest_unit {
        if let Some(u) = units.iter().find(|u| u.id == uid) {
            // Recycle backwards if source is right of dest
            if start[0] > u.pos[0] + u.size[0] * 0.5 {
                [u.pos[0] + u.size[0] * 0.5, u.pos[1] + u.size[1]]
            } else {
                [u.pos[0], u.pos[1] + u.size[1] * 0.5]
            }
        } else {
            s.dest_anchor
        }
    } else {
        s.dest_anchor
    };

    let mut pts = vec![start];
    if !s.waypoints.is_empty() {
        pts.extend(s.waypoints.iter().copied());
    } else if orthogonal {
        if (start[1] - end[1]).abs() > 1.5 {
            let mid_x = 0.5 * (start[0] + end[0]);
            if end[0] < start[0] - 4.0 {
                // Recycle loop route underneath units
                let drop_y = start[1].max(end[1]) + 22.0;
                pts.push([start[0], drop_y]);
                pts.push([end[0], drop_y]);
            } else {
                pts.push([mid_x, start[1]]);
                pts.push([mid_x, end[1]]);
            }
        }
    }
    pts.push(end);
    pts
}

fn draw_gradient_rect(painter: &egui::Painter, rect: Rect, c1: Color32, c2: Color32) {
    let mut mesh = Mesh::default();
    let idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_top(),
        uv: egui::epaint::WHITE_UV,
        color: c1,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_top(),
        uv: egui::epaint::WHITE_UV,
        color: c1.linear_multiply(0.95),
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: c2,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: c2.linear_multiply(0.97),
    });
    mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
    painter.add(Shape::mesh(mesh));
}

fn draw_arrowhead(
    painter: &egui::Painter,
    from: Pos2,
    tip: Pos2,
    style: ArrowHeadStyle,
    size_px: f32,
    color: Color32,
) {
    let dir_v = tip - from;
    let len = dir_v.length();
    if len < 1e-3 {
        return;
    }
    let dir = dir_v / len;
    let perp = Vec2::new(-dir.y, dir.x);
    match style {
        ArrowHeadStyle::None => {}
        ArrowHeadStyle::Triangle => {
            let p1 = tip - dir * size_px + perp * (size_px * 0.55);
            let p2 = tip - dir * size_px - perp * (size_px * 0.55);
            painter.add(Shape::convex_polygon(
                vec![tip, p1, p2],
                Color32::WHITE,
                Stroke::new(1.1, color),
            ));
        }
    }
}

fn unit_at_mm(units: &[SimUnit], page: usize, mm: [f32; 2]) -> Option<usize> {
    units
        .iter()
        .rev()
        .find(|u| {
            u.page == page
                && u.visible_unit
                && mm[0] >= u.pos[0]
                && mm[0] <= u.pos[0] + u.size[0]
                && mm[1] >= u.pos[1]
                && mm[1] <= u.pos[1] + u.size[1]
        })
        .map(|u| u.id)
}

pub fn render_flowsheet_canvas(ui: &mut egui::Ui, state: &mut SimModuleState) {
    let available = ui.available_rect_before_wrap();
    if available.width() < 40.0 || available.height() < 40.0 {
        return;
    }

    let ruler_h = if state.show_rulers { 18.0 } else { 0.0 };
    let ruler_w = if state.show_rulers { 20.0 } else { 0.0 };
    let scale = MM_TO_PX_BASE * (state.zoom_pct / 100.0).clamp(0.25, 3.0);

    let canvas_rect = Rect::from_min_max(
        Pos2::new(available.left() + ruler_w, available.top() + ruler_h),
        available.max,
    );
    let origin = Pos2::new(canvas_rect.left() + 18.0, canvas_rect.top() + 12.0);
    let painter = ui.painter_at(available);

    // Paint page background
    let page_bg = to_color32(state.project.canvas_settings.page_bg_rgb);
    painter.rect_filled(canvas_rect, 0.0, page_bg);

    // Canvas pointer interaction
    let resp = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
    if let Some(hover_pos) = resp.hover_pos() {
        state.cursor_mm = screen_to_mm(hover_pos, origin, state.pan_mm, scale);
    }

    // Mouse wheel zoom with Ctrl or Alt
    if resp.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        let zoom_mod = ui.input(|i| i.modifiers.ctrl);
        if zoom_mod && scroll_delta.abs() > 0.5 {
            state.zoom_pct = (state.zoom_pct + scroll_delta * 0.15).clamp(25.0, 250.0);
        }
    }

    // Draw grid points (Fig. 15 style)
    if state.project.canvas_settings.show_grid {
        let grid_c = to_color32(state.project.canvas_settings.grid_color_rgb);
        let gw = state.project.canvas_settings.grid_width.max(1.0);
        let gh = state.project.canvas_settings.grid_height.max(1.0);
        let mut step_mm_x = gw;
        let mut step_mm_y = gh;
        while step_mm_x * scale < 7.0 {
            step_mm_x *= 2.0;
        }
        while step_mm_y * scale < 7.0 {
            step_mm_y *= 2.0;
        }
        let min_mm = screen_to_mm(canvas_rect.min, origin, state.pan_mm, scale);
        let max_mm = screen_to_mm(canvas_rect.max, origin, state.pan_mm, scale);
        let start_x = (min_mm[0] / step_mm_x).floor() * step_mm_x;
        let start_y = (min_mm[1] / step_mm_y).floor() * step_mm_y;
        let mut x = start_x;
        let canvas_painter = ui.painter_at(canvas_rect);
        while x <= max_mm[0] {
            let sx = origin.x + (x - state.pan_mm[0]) * scale;
            let mut y = start_y;
            while y <= max_mm[1] {
                let sy = origin.y + (y - state.pan_mm[1]) * scale;
                canvas_painter.circle_filled(Pos2::new(sx, sy), 0.75, grid_c);
                y += step_mm_y;
            }
            x += step_mm_x;
        }
    }

    let cp = ui.painter_at(canvas_rect);
    let page = state.project.active_page;
    let orthogonal = state.project.canvas_settings.orthogonal;
    let vis = &state.project.visualization;
    let calc_order = if state.highlight_calc_order_on_canvas {
        Some(SoryntelApplication::sim_calc_order(&state.project))
    } else {
        None
    };

    // Build Layer & Group visibility lookup
    let is_layer_visible = |layer_idx: usize| -> bool {
        state
            .project
            .layers
            .get(layer_idx)
            .map(|l| l.visible)
            .unwrap_or(true)
    };
    let is_unit_group_visible = |uid: usize| -> bool {
        !state
            .project
            .groups
            .iter()
            .any(|g| !g.visible && g.unit_ids.contains(&uid))
    };

    // ------------------------------------------------------------------
    // 1. Render Streams (bottom layer behind units for clean arrows)
    // ------------------------------------------------------------------
    let mut stream_midpoints: Vec<(usize, Pos2, [f32; 2], [f32; 2])> = Vec::new();
    let max_metric_val: f64 = state
        .project
        .streams
        .iter()
        .filter(|s| s.page == page)
        .map(|s| {
            let (mut v, _) = SoryntelApplication::sim_metric_value(s, &vis.variable);
            if vis.use_absolute_values {
                v = v.abs();
            }
            v * 10f64.powi(vis.multiplier_pow10)
        })
        .fold(1.0_f64, f64::max);

    for s in &state.project.streams {
        if s.page != page || !s.visible_stream || !is_layer_visible(s.layer) {
            continue;
        }
        let pts_mm = compute_stream_points_mm(s, &state.project.units, orthogonal);
        let pts_px: Vec<Pos2> = pts_mm
            .iter()
            .map(|p| mm_to_screen(*p, origin, state.pan_mm, scale))
            .collect();
        if pts_px.len() < 2 {
            continue;
        }

        let is_selected = state.selected == SelectedObject::Stream(s.id)
            || state.multi_selected_streams.contains(&s.id);

        let (mut metric_val, metric_unit) =
            SoryntelApplication::sim_metric_value(s, &vis.variable);
        if vis.use_absolute_values {
            metric_val = metric_val.abs();
        }
        let scaled_metric = metric_val * 10f64.powi(vis.multiplier_pow10);

        let stroke_color = if vis.connectivity_mode {
            match s.connectivity_kind() {
                StreamConnectivity::Input => Color32::from_rgb(0, 102, 204),
                StreamConnectivity::Internal => Color32::BLACK,
                StreamConnectivity::Output => Color32::from_rgb(208, 28, 28),
            }
        } else if vis.active && vis.variable != "Select visualization" {
            let frac = if (vis.range_max - vis.range_min).abs() > 1e-6 {
                ((scaled_metric - vis.range_min) / (vis.range_max - vis.range_min)).clamp(0.0, 1.0)
            } else {
                (scaled_metric / max_metric_val).clamp(0.0, 1.0)
            };
            match vis.color_mode {
                SankeyColorMode::FlowsheetStreamColors => to_color32(s.line_color),
                SankeyColorMode::PositiveAndNegativeColors => {
                    if metric_val >= 0.0 {
                        to_color32(vis.color_1)
                    } else {
                        to_color32(vis.color_2)
                    }
                }
                SankeyColorMode::RangeColors => {
                    // Green -> Yellow -> Red gradient
                    if frac < 0.5 {
                        let t = (frac * 2.0) as f32;
                        Color32::from_rgb((34.0 + t * 200.0) as u8, 178, 52)
                    } else {
                        let t = ((frac - 0.5) * 2.0) as f32;
                        Color32::from_rgb(228, (178.0 - t * 140.0) as u8, 38)
                    }
                }
            }
        } else {
            to_color32(s.line_color)
        };

        let stroke_w = if vis.active && vis.visualize_stream_line_width && vis.variable != "Select visualization" {
            let frac = (scaled_metric / max_metric_val).clamp(0.08, 1.0) as f32;
            1.4 + frac * (vis.stream_width_max * 2.8)
        } else {
            (s.line_width * 4.2).clamp(1.2, 4.0)
        };

        if is_selected {
            for w in pts_px.windows(2) {
                cp.line_segment([w[0], w[1]], Stroke::new(stroke_w + 3.0, Color32::from_rgb(180, 212, 255)));
            }
        }
        for w in pts_px.windows(2) {
            cp.line_segment([w[0], w[1]], Stroke::new(stroke_w, stroke_color));
        }

        // Start & End arrows
        let p0 = pts_px[0];
        let p1 = pts_px[1];
        if s.source_unit.is_none() {
            cp.circle_filled(p0, 3.2, Color32::WHITE);
            cp.circle_stroke(p0, 3.2, Stroke::new(1.2, stroke_color));
        }
        let pn = pts_px[pts_px.len() - 1];
        let p_prev = pts_px[pts_px.len() - 2];
        draw_arrowhead(
            &cp,
            p_prev,
            pn,
            s.end_arrow_style,
            (s.end_arrow_size as f32 + 2.5) * (state.zoom_pct / 100.0).clamp(0.7, 1.3),
            stroke_color,
        );

        let mid_idx = pts_px.len() / 2;
        let mid = if pts_px.len() == 2 {
            Pos2::new(0.5 * (p0.x + p1.x), 0.5 * (p0.y + p1.y))
        } else {
            pts_px[mid_idx - 1].lerp(pts_px[mid_idx], 0.5)
        };
        stream_midpoints.push((s.id, mid, *pts_mm.first().unwrap(), *pts_mm.last().unwrap()));

        // Stream Name + Value Labels
        if s.visible_name_label {
            cp.text(
                mid - Vec2::new(0.0, 9.0),
                egui::Align2::CENTER_CENTER,
                &s.name_id,
                egui::FontId::proportional(11.0 * (scale / MM_TO_PX_BASE).clamp(0.7, 1.25)),
                Color32::from_rgb(22, 26, 32),
            );
        }
        if vis.active && vis.variable != "Select visualization" {
            let lbl = if vis.show_measurement_units {
                format!("{:.1} {}", metric_val, metric_unit)
            } else {
                format!("{:.1}", metric_val)
            };
            let pill = Rect::from_center_size(mid + Vec2::new(0.0, 9.0), Vec2::new(66.0, 15.0));
            cp.rect_filled(pill, 2.0, Color32::from_rgba_unmultiplied(255, 255, 232, 230));
            cp.rect_stroke(pill, 2.0, Stroke::new(0.7, Color32::from_rgb(140, 150, 165)));
            cp.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                lbl,
                egui::FontId::proportional(10.0),
                Color32::BLACK,
            );
        }

        // Duplicate name warning badge (Fig. 64 / section 40.3.2)
        if s.has_duplicates {
            cp.text(
                mid - Vec2::new(0.0, 20.0),
                egui::Align2::CENTER_CENTER,
                "Duplicate Name!",
                egui::FontId::proportional(9.5),
                Color32::from_rgb(200, 30, 30),
            );
        }

        // Optional attached Stream Table preview on canvas (Fig. 20)
        if s.visible_stream_table {
            let tbl_pos = mid + Vec2::new(-38.0, 14.0);
            let st_rect = Rect::from_min_size(tbl_pos, Vec2::new(98.0, 52.0));
            cp.rect_filled(st_rect, 1.0, Color32::from_rgba_unmultiplied(250, 252, 255, 240));
            cp.rect_stroke(st_rect, 1.0, Stroke::new(0.9, SIM_BORDER));
            cp.rect_filled(
                Rect::from_min_size(tbl_pos, Vec2::new(98.0, 15.0)),
                0.0,
                Color32::from_rgb(220, 230, 244),
            );
            cp.text(
                Pos2::new(tbl_pos.x + 4.0, tbl_pos.y + 7.5),
                egui::Align2::LEFT_CENTER,
                &s.name_id,
                egui::FontId::proportional(9.5),
                SIM_TEXT_DARK,
            );
            cp.text(
                Pos2::new(tbl_pos.x + 4.0, tbl_pos.y + 23.0),
                egui::Align2::LEFT_CENTER,
                format!("Mass: {:.1} t/h", s.mass_flow_t_h),
                egui::FontId::proportional(9.0),
                SIM_TEXT_DARK,
            );
            cp.text(
                Pos2::new(tbl_pos.x + 4.0, tbl_pos.y + 34.0),
                egui::Align2::LEFT_CENTER,
                format!("T: {:.1} °C  |  P: {:.1} bar", s.temp_c, s.pressure_bar),
                egui::FontId::proportional(8.5),
                SIM_MUTED_TEXT,
            );
            cp.text(
                Pos2::new(tbl_pos.x + 4.0, tbl_pos.y + 45.0),
                egui::Align2::LEFT_CENTER,
                format!("Cu: {:.2}%  Sol: {:.1}%", s.cu_wt_pct, s.pct_solids),
                egui::FontId::proportional(8.5),
                SIM_MUTED_TEXT,
            );
        }

        // Blue Handle Squares on Selected Stream (Fig. 3)
        if state.selected == SelectedObject::Stream(s.id) {
            let h1 = Rect::from_center_size(p0, Vec2::splat(8.0));
            let h2 = Rect::from_center_size(pn, Vec2::splat(8.0));
            cp.rect_filled(h1, 1.0, Color32::from_rgb(30, 120, 235));
            cp.rect_stroke(h1, 1.0, Stroke::new(1.0, Color32::WHITE));
            cp.rect_filled(h2, 1.0, Color32::from_rgb(30, 120, 235));
            cp.rect_stroke(h2, 1.0, Stroke::new(1.0, Color32::WHITE));
        }
    }

    // ------------------------------------------------------------------
    // 2. Render Units (Fig. 2 gradient boxes / equipment icons)
    // ------------------------------------------------------------------
    for u in &state.project.units {
        if u.page != page || !u.visible_unit || !is_layer_visible(u.layer) || !is_unit_group_visible(u.id) {
            continue;
        }
        let tl = mm_to_screen(u.pos, origin, state.pan_mm, scale);
        let br = mm_to_screen(
            [u.pos[0] + u.size[0], u.pos[1] + u.size[1]],
            origin,
            state.pan_mm,
            scale,
        );
        let urect = Rect::from_min_max(tl, br);
        let is_selected = state.selected == SelectedObject::Unit(u.id)
            || state.multi_selected_units.contains(&u.id);

        if !u.transparent_bg {
            draw_gradient_rect(&cp, urect, to_color32(u.fill_color_1), to_color32(u.fill_color_2));
        }

        if u.show_unit_image {
            if let Some(ref icon_k) = u.icon_key {
                draw_equipment_icon(&cp, urect.shrink(4.0), icon_k);
            }
        }

        let border_stroke = if is_selected {
            Stroke::new(2.0, Color32::from_rgb(16, 108, 224))
        } else {
            Stroke::new((u.line_width * 3.5).clamp(0.9, 2.4), to_color32(u.line_color))
        };
        cp.rect_stroke(urect, 1.5, border_stroke);

        // Selection handles around selected unit
        if is_selected {
            for handle_pt in [
                urect.left_top(),
                urect.right_top(),
                urect.left_bottom(),
                urect.right_bottom(),
            ] {
                let hr = Rect::from_center_size(handle_pt, Vec2::splat(6.0));
                cp.rect_filled(hr, 0.5, Color32::from_rgb(16, 108, 224));
                cp.rect_stroke(hr, 0.5, Stroke::new(0.8, Color32::WHITE));
            }
        }

        // Small unit-type corner pill `[U]` / `[R]` / `[D]` / `[τ]`
        if u.icon_key.is_none() {
            let badge_r = Rect::from_min_size(urect.left_top() + Vec2::splat(3.0), Vec2::splat(14.0));
            cp.rect_filled(badge_r, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 190));
            cp.text(
                badge_r.center(),
                egui::Align2::CENTER_CENTER,
                u.kind.badge(),
                egui::FontId::proportional(9.5),
                Color32::from_rgb(48, 68, 96),
            );
        }

        // Calculation Order overlay badge (Fig. 40)
        if let Some(ref info) = calc_order {
            if let Some(pos_idx) = info.unit_order_ids.iter().position(|id| *id == u.id) {
                let badge = Rect::from_center_size(
                    Pos2::new(urect.right() - 8.0, urect.top() + 8.0),
                    Vec2::new(22.0, 15.0),
                );
                let col = match pos_idx % 3 {
                    0 => Color32::from_rgb(34, 139, 34),
                    1 => Color32::from_rgb(217, 119, 6),
                    _ => Color32::from_rgb(124, 58, 237),
                };
                cp.rect_filled(badge, 3.0, col);
                cp.text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("#{}", pos_idx + 1),
                    egui::FontId::proportional(9.5),
                    Color32::WHITE,
                );
            }
        }

        // Unit name label (outside above or centered inside)
        if u.visible_name_label {
            let label_pos = if u.show_title_inside {
                urect.center()
            } else {
                Pos2::new(urect.center().x, urect.top() - 9.0)
            };
            cp.text(
                label_pos,
                egui::Align2::CENTER_CENTER,
                &u.name_id,
                egui::FontId::proportional((u.font_size + 3.0) * (scale / MM_TO_PX_BASE).clamp(0.75, 1.25)),
                to_color32(u.font_color),
            );
        }
    }

    // ------------------------------------------------------------------
    // 3. Render Annotations & Tables (`Table`, `Title Table`, `Text`, `Shape`)
    // ------------------------------------------------------------------
    for ann in &state.project.annotations {
        if ann.page != page || !is_layer_visible(ann.layer) {
            continue;
        }
        let tl = mm_to_screen(ann.pos, origin, state.pan_mm, scale);
        let br = mm_to_screen(
            [ann.pos[0] + ann.size[0], ann.pos[1] + ann.size[1]],
            origin,
            state.pan_mm,
            scale,
        );
        let arect = Rect::from_min_max(tl, br);
        let sel_stroke = if state.selected == SelectedObject::Annotation(ann.id) {
            Stroke::new(1.8, SIM_ACCENT_BLUE)
        } else {
            Stroke::new(1.0, to_color32(ann.line_color))
        };

        match ann.kind {
            AnnotationKind::HeaderLabel | AnnotationKind::TextLabel => {
                cp.text(
                    arect.left_top(),
                    egui::Align2::LEFT_TOP,
                    &ann.text,
                    egui::FontId::proportional(
                        (ann.font_size + if matches!(ann.kind, AnnotationKind::HeaderLabel) { 6.0 } else { 2.5 })
                            * (scale / MM_TO_PX_BASE).clamp(0.75, 1.3),
                    ),
                    to_color32(ann.text_color),
                );
                if state.selected == SelectedObject::Annotation(ann.id) {
                    cp.rect_stroke(arect, 1.0, sel_stroke);
                }
            }
            AnnotationKind::MultilineBox | AnnotationKind::Rectangle | AnnotationKind::RoundedRect => {
                let r = if matches!(ann.kind, AnnotationKind::RoundedRect) { 6.0 } else { 1.0 };
                cp.rect_filled(arect, r, to_color32(ann.fill_color));
                cp.rect_stroke(arect, r, sel_stroke);
                if !ann.text.is_empty() {
                    cp.text(
                        arect.left_top() + Vec2::splat(4.0),
                        egui::Align2::LEFT_TOP,
                        &ann.text,
                        egui::FontId::proportional(11.0),
                        to_color32(ann.text_color),
                    );
                }
            }
            AnnotationKind::Ellipse => {
                cp.circle_filled(arect.center(), arect.width().min(arect.height()) * 0.5, to_color32(ann.fill_color));
                cp.circle_stroke(arect.center(), arect.width().min(arect.height()) * 0.5, sel_stroke);
            }
            AnnotationKind::Line => {
                cp.line_segment([arect.left_top(), arect.right_bottom()], sel_stroke);
            }
            AnnotationKind::Triangle | AnnotationKind::Polygon => {
                let pts = vec![
                    Pos2::new(arect.center().x, arect.top()),
                    arect.right_bottom(),
                    arect.left_bottom(),
                ];
                cp.add(Shape::convex_polygon(pts, to_color32(ann.fill_color), sel_stroke));
            }
            AnnotationKind::PictureBox | AnnotationKind::Arc => {
                cp.rect_filled(arect, 2.0, Color32::from_rgb(245, 248, 252));
                cp.rect_stroke(arect, 2.0, sel_stroke);
                cp.text(
                    arect.center(),
                    egui::Align2::CENTER_CENTER,
                    if ann.text.is_empty() { "Image" } else { &ann.text },
                    egui::FontId::proportional(10.5),
                    SIM_MUTED_TEXT,
                );
            }
        }
    }

    for tbl in &state.project.tables {
        if tbl.page != page || !is_layer_visible(tbl.layer) {
            continue;
        }
        let tl = mm_to_screen(tbl.pos, origin, state.pan_mm, scale);
        let br = mm_to_screen(
            [tbl.pos[0] + tbl.size[0], tbl.pos[1] + tbl.size[1]],
            origin,
            state.pan_mm,
            scale,
        );
        let trect = Rect::from_min_max(tl, br);
        let sel = state.selected == SelectedObject::Table(tbl.id);

        cp.rect_filled(trect, 1.0, Color32::WHITE);
        cp.rect_stroke(
            trect,
            1.0,
            if sel {
                Stroke::new(2.0, SIM_ACCENT_BLUE)
            } else {
                Stroke::new(1.1, Color32::from_rgb(56, 64, 76))
            },
        );

        if tbl.is_title_table {
            // Official Title Table 4-row engineering header with red `Logo` box (Fig. 8)
            let logo_w = trect.width() * 0.25;
            let logo_rect = Rect::from_min_max(trect.min, Pos2::new(trect.left() + logo_w, trect.bottom()));
            cp.rect_stroke(logo_rect, 0.0, Stroke::new(0.8, Color32::GRAY));
            cp.text(
                logo_rect.center(),
                egui::Align2::CENTER_CENTER,
                &tbl.logo_text,
                egui::FontId::proportional(14.0),
                Color32::from_rgb(208, 30, 30),
            );
            let right_x = logo_rect.right();
            let rw = trect.right() - right_x;
            let rh = trect.height() / 4.0;
            let lines = [
                tbl.title.as_str(),
                tbl.organization_author.as_str(),
                tbl.hsc_version.as_str(),
                tbl.date.as_str(),
            ];
            for (idx, txt) in lines.iter().enumerate() {
                let row_r = Rect::from_min_size(Pos2::new(right_x, trect.top() + idx as f32 * rh), Vec2::new(rw, rh));
                cp.rect_stroke(row_r, 0.0, Stroke::new(0.6, Color32::from_rgb(160, 168, 178)));
                cp.text(
                    Pos2::new(row_r.left() + 6.0, row_r.center().y),
                    egui::Align2::LEFT_CENTER,
                    *txt,
                    egui::FontId::proportional(if idx == 0 { 11.5 } else { 10.0 }),
                    SIM_TEXT_DARK,
                );
            }
        } else {
            // Regular spreadsheet table on canvas (Fig. 7 / Stream Flow Table Fig. 21-23)
            let n_rows = tbl.cells.len().max(1);
            let n_cols = tbl.cells.first().map(|r| r.len()).unwrap_or(1).max(1);
            let rh = trect.height() / n_rows as f32;
            let cw = trect.width() / n_cols as f32;
            for (r_idx, row) in tbl.cells.iter().enumerate() {
                for (c_idx, cell) in row.iter().enumerate() {
                    let cr = Rect::from_min_size(
                        Pos2::new(trect.left() + c_idx as f32 * cw, trect.top() + r_idx as f32 * rh),
                        Vec2::new(cw, rh),
                    );
                    if r_idx == 0 {
                        cp.rect_filled(cr, 0.0, Color32::from_rgb(232, 238, 248));
                    }
                    cp.rect_stroke(cr, 0.0, Stroke::new(0.6, Color32::from_rgb(175, 182, 192)));
                    cp.text(
                        cr.center(),
                        egui::Align2::CENTER_CENTER,
                        cell,
                        egui::FontId::proportional(10.0),
                        SIM_TEXT_DARK,
                    );
                }
            }
        }
    }

    // Pending stream rubber-band line when `AddStream` is active
    if let (ActiveTool::AddStream, Some(start_mm)) = (state.active_tool, state.pending_stream_start_mm) {
        let p0 = mm_to_screen(start_mm, origin, state.pan_mm, scale);
        let p1 = mm_to_screen(state.cursor_mm, origin, state.pan_mm, scale);
        cp.line_segment([p0, p1], Stroke::new(1.5, SIM_ACCENT_BLUE));
    }

    // ------------------------------------------------------------------
    // 4. Drawing / Entire Process Lock Inner Border (Fig. 58!)
    // ------------------------------------------------------------------
    match state.project.lock_state {
        LockState::Unlocked => {}
        LockState::DrawingLocked => {
            cp.rect_stroke(canvas_rect.shrink(1.5), 0.0, Stroke::new(3.0, Color32::from_rgb(0, 178, 112)));
        }
        LockState::EntireProcessLocked => {
            cp.rect_stroke(canvas_rect.shrink(1.5), 0.0, Stroke::new(3.0, Color32::from_rgb(212, 34, 34)));
        }
    }

    // ------------------------------------------------------------------
    // 5. Render Millimeter Rulers Top & Left + Red Hairline Markers
    // ------------------------------------------------------------------
    if state.show_rulers {
        let top_ruler = Rect::from_min_max(
            Pos2::new(available.left() + ruler_w, available.top()),
            Pos2::new(available.right(), available.top() + ruler_h),
        );
        let left_ruler = Rect::from_min_max(
            Pos2::new(available.left(), available.top() + ruler_h),
            Pos2::new(available.left() + ruler_w, available.bottom()),
        );
        let corner_box = Rect::from_min_size(available.min, Vec2::new(ruler_w, ruler_h));

        painter.rect_filled(corner_box, 0.0, SIM_HEADER_BG);
        painter.rect_stroke(corner_box, 0.0, Stroke::new(0.8, SIM_BORDER));
        painter.circle_stroke(corner_box.center(), 4.5, Stroke::new(1.0, Color32::from_rgb(80, 120, 40)));

        painter.rect_filled(top_ruler, 0.0, SIM_RULER_BG);
        painter.rect_filled(left_ruler, 0.0, SIM_RULER_BG);

        let mm_start_x = (((top_ruler.left() - origin.x) / scale + state.pan_mm[0]) / 2.0).floor() as i32 * 2;
        let mm_end_x = (((top_ruler.right() - origin.x) / scale + state.pan_mm[0]) / 2.0).ceil() as i32 * 2;
        for mm in (mm_start_x..=mm_end_x).step_by(2) {
            let sx = origin.x + (mm as f32 - state.pan_mm[0]) * scale;
            if sx < top_ruler.left() || sx > top_ruler.right() {
                continue;
            }
            let tick_h = if mm % 10 == 0 {
                8.0
            } else if mm % 5 == 0 {
                5.0
            } else {
                3.0
            };
            painter.line_segment(
                [Pos2::new(sx, top_ruler.bottom() - tick_h), Pos2::new(sx, top_ruler.bottom())],
                Stroke::new(0.7, SIM_MUTED_TEXT),
            );
            if mm % 10 == 0 {
                painter.text(
                    Pos2::new(sx + 2.0, top_ruler.top() + 5.5),
                    egui::Align2::LEFT_CENTER,
                    format!("{mm}"),
                    egui::FontId::proportional(9.0),
                    SIM_TEXT_DARK,
                );
            }
        }
        // Red cursor tick on top ruler
        let cur_sx = origin.x + (state.cursor_mm[0] - state.pan_mm[0]) * scale;
        if cur_sx >= top_ruler.left() && cur_sx <= top_ruler.right() {
            painter.line_segment(
                [Pos2::new(cur_sx, top_ruler.top()), Pos2::new(cur_sx, top_ruler.bottom())],
                Stroke::new(1.2, Color32::from_rgb(220, 38, 38)),
            );
        }

        let mm_start_y = (((left_ruler.top() - origin.y) / scale + state.pan_mm[1]) / 2.0).floor() as i32 * 2;
        let mm_end_y = (((left_ruler.bottom() - origin.y) / scale + state.pan_mm[1]) / 2.0).ceil() as i32 * 2;
        for mm in (mm_start_y..=mm_end_y).step_by(2) {
            let sy = origin.y + (mm as f32 - state.pan_mm[1]) * scale;
            if sy < left_ruler.top() || sy > left_ruler.bottom() {
                continue;
            }
            let tick_w = if mm % 10 == 0 {
                8.0
            } else if mm % 5 == 0 {
                5.0
            } else {
                3.0
            };
            painter.line_segment(
                [Pos2::new(left_ruler.right() - tick_w, sy), Pos2::new(left_ruler.right(), sy)],
                Stroke::new(0.7, SIM_MUTED_TEXT),
            );
            if mm % 10 == 0 {
                painter.text(
                    Pos2::new(left_ruler.left() + 2.0, sy + 1.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{mm}"),
                    egui::FontId::proportional(8.5),
                    SIM_TEXT_DARK,
                );
            }
        }
        // Red cursor tick on left ruler
        let cur_sy = origin.y + (state.cursor_mm[1] - state.pan_mm[1]) * scale;
        if cur_sy >= left_ruler.top() && cur_sy <= left_ruler.bottom() {
            painter.line_segment(
                [Pos2::new(left_ruler.left(), cur_sy), Pos2::new(left_ruler.right(), cur_sy)],
                Stroke::new(1.2, Color32::from_rgb(220, 38, 38)),
            );
        }
        painter.line_segment(
            [top_ruler.left_bottom(), top_ruler.right_bottom()],
            Stroke::new(0.9, SIM_BORDER),
        );
        painter.line_segment(
            [left_ruler.right_top(), left_ruler.right_bottom()],
            Stroke::new(0.9, SIM_BORDER),
        );
    }

    // ------------------------------------------------------------------
    // 6. Pointer Event Handling (Select, Drag, Connect, Create, Edit)
    // ------------------------------------------------------------------
    let lock = state.project.lock_state;
    let snap_on = state.project.canvas_settings.snap_to_grid;
    let gw = state.project.canvas_settings.grid_width;
    let gh = state.project.canvas_settings.grid_height;

    if resp.drag_started() && state.active_tool == ActiveTool::Select && lock == LockState::Unlocked {
        if let Some(pos) = resp.interact_pointer_pos() {
            let click_mm = screen_to_mm(pos, origin, state.pan_mm, scale);
            // Check stream blue handles first if a stream is selected (Fig. 3!)
            state.dragging_stream_handle = None;
            if let SelectedObject::Stream(sid) = state.selected {
                if let Some((_id, _mid, start_mm, end_mm)) =
                    stream_midpoints.iter().find(|(id, ..)| *id == sid)
                {
                    let p0 = mm_to_screen(*start_mm, origin, state.pan_mm, scale);
                    let pn = mm_to_screen(*end_mm, origin, state.pan_mm, scale);
                    if pos.distance(p0) < 12.0 {
                        state.dragging_stream_handle = Some((sid, false));
                    } else if pos.distance(pn) < 12.0 {
                        state.dragging_stream_handle = Some((sid, true));
                    }
                }
            }
            if state.dragging_stream_handle.is_none() {
                if let Some(uid) = unit_at_mm(&state.project.units, page, click_mm) {
                    state.selected = SelectedObject::Unit(uid);
                    state.dragging_obj = true;
                }
            }
        }
    }

    if resp.dragged() {
        let delta_px = resp.drag_delta();
        let delta_mm = [delta_px.x / scale, delta_px.y / scale];
        let middle_pan = ui.input(|i| i.pointer.middle_down());
        if state.active_tool == ActiveTool::Pan || middle_pan {
            state.pan_mm[0] -= delta_mm[0];
            state.pan_mm[1] -= delta_mm[1];
        } else if let Some((sid, is_dest)) = state.dragging_stream_handle {
            if let Some(pointer_pos) = resp.interact_pointer_pos() {
                let ptr_mm = screen_to_mm(pointer_pos, origin, state.pan_mm, scale);
                let snapped = [snap(ptr_mm[0], snap_on, gw), snap(ptr_mm[1], snap_on, gh)];
                if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                    if is_dest {
                        s.dest_anchor = snapped;
                    } else {
                        s.source_anchor = snapped;
                    }
                }
            }
        } else if state.dragging_obj && lock == LockState::Unlocked {
            match state.selected {
                SelectedObject::Unit(uid) => {
                    if let Some(u) = state.project.units.iter_mut().find(|u| u.id == uid) {
                        u.pos[0] += delta_mm[0];
                        u.pos[1] += delta_mm[1];
                    }
                }
                SelectedObject::Table(tid) => {
                    if let Some(t) = state.project.tables.iter_mut().find(|t| t.id == tid) {
                        t.pos[0] += delta_mm[0];
                        t.pos[1] += delta_mm[1];
                    }
                }
                SelectedObject::Annotation(aid) => {
                    if let Some(a) = state.project.annotations.iter_mut().find(|a| a.id == aid) {
                        a.pos[0] += delta_mm[0];
                        a.pos[1] += delta_mm[1];
                    }
                }
                _ => {}
            }
        }
    }

    if resp.drag_stopped() {
        if let Some((sid, is_dest)) = state.dragging_stream_handle.take() {
            let hovered_uid = unit_at_mm(&state.project.units, page, state.cursor_mm);
            if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                if is_dest {
                    s.dest_unit = hovered_uid;
                } else {
                    s.source_unit = hovered_uid;
                }
            }
            state.open_stream_src_dest(sid);
        }
        if state.dragging_obj && snap_on {
            if let SelectedObject::Unit(uid) = state.selected {
                if let Some(u) = state.project.units.iter_mut().find(|u| u.id == uid) {
                    u.pos[0] = snap(u.pos[0], true, gw);
                    u.pos[1] = snap(u.pos[1], true, gh);
                }
            }
        }
        state.dragging_obj = false;
    }

    if resp.clicked() || resp.double_clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let mm = screen_to_mm(pos, origin, state.pan_mm, scale);
            let snapped = [snap(mm[0], snap_on, gw), snap(mm[1], snap_on, gh)];

            if state.active_tool != ActiveTool::Select
                && state.active_tool != ActiveTool::Pan
                && lock == LockState::Unlocked
            {
                handle_tool_placement(state, snapped, mm, page);
            } else {
                // Selection hit-test: Units -> Tables -> Annotations -> Streams
                if let Some(uid) = unit_at_mm(&state.project.units, page, mm) {
                    if ui.input(|i| i.modifiers.ctrl) {
                        if let Some(idx) = state.multi_selected_units.iter().position(|x| *x == uid) {
                            state.multi_selected_units.remove(idx);
                        } else {
                            state.multi_selected_units.push(uid);
                        }
                    } else {
                        state.multi_selected_units.clear();
                    }
                    state.selected = SelectedObject::Unit(uid);
                    if resp.double_clicked() {
                        state.open_unit_editor(uid);
                    }
                } else if let Some(tid) = state
                    .project
                    .tables
                    .iter()
                    .rev()
                    .find(|t| {
                        t.page == page
                            && mm[0] >= t.pos[0]
                            && mm[0] <= t.pos[0] + t.size[0]
                            && mm[1] >= t.pos[1]
                            && mm[1] <= t.pos[1] + t.size[1]
                    })
                    .map(|t| t.id)
                {
                    state.selected = SelectedObject::Table(tid);
                    if resp.double_clicked() {
                        state.editing_table_id = Some(tid);
                        state.show_table_editor = true;
                    }
                } else if let Some(aid) = state
                    .project
                    .annotations
                    .iter()
                    .rev()
                    .find(|a| {
                        a.page == page
                            && mm[0] >= a.pos[0]
                            && mm[0] <= a.pos[0] + a.size[0]
                            && mm[1] >= a.pos[1]
                            && mm[1] <= a.pos[1] + a.size[1]
                    })
                    .map(|a| a.id)
                {
                    state.selected = SelectedObject::Annotation(aid);
                } else {
                    // Hit-test stream segments
                    let mut hit_sid = None;
                    for s in state.project.streams.iter().filter(|s| s.page == page && s.visible_stream) {
                        let pts_px: Vec<Pos2> =
                            compute_stream_points_mm(s, &state.project.units, orthogonal)
                                .into_iter()
                                .map(|p| mm_to_screen(p, origin, state.pan_mm, scale))
                                .collect();
                        if pts_px.windows(2).any(|w| dist_to_segment(pos, w[0], w[1]) < 8.0) {
                            hit_sid = Some(s.id);
                            break;
                        }
                    }
                    if let Some(sid) = hit_sid {
                        state.selected = SelectedObject::Stream(sid);
                        if resp.double_clicked() {
                            state.open_stream_src_dest(sid);
                        }
                    } else {
                        state.selected = SelectedObject::None;
                        state.multi_selected_units.clear();
                    }
                }
            }
        }
    }

    // Canvas Right-Click Context Menu
    resp.context_menu(|ui| {
        match state.selected {
            SelectedObject::Unit(uid) => {
                if ui.button(crate::i18n::text("review.182")).clicked() {
                    state.open_unit_editor(uid);
                    ui.close_menu();
                }
                if ui.button("Select Unit Model...").clicked() {
                    state.show_select_unit_models = true;
                    ui.close_menu();
                }
                if ui.button("Calculate Selected Unit (Ctrl+F9)").clicked() {
                    state.run_static(true);
                    ui.close_menu();
                }
                if ui.button("Rename Unit").clicked() {
                    if let Some(u) = state.project.units.iter().find(|u| u.id == uid) {
                        state.inline_rename_target = Some(SelectedObject::Unit(uid));
                        state.inline_rename_buf = u.name_id.clone();
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete Unit (Del)").clicked() {
                    state.delete_selected();
                    ui.close_menu();
                }
            }
            SelectedObject::Stream(sid) => {
                if ui.button(crate::i18n::text("review.183")).clicked() {
                    state.open_stream_src_dest(sid);
                    ui.close_menu();
                }
                if ui.button("Toggle Attached Stream Table").clicked() {
                    if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                        s.visible_stream_table = !s.visible_stream_table;
                    }
                    ui.close_menu();
                }
                if ui.button("Add to Active Stream Table").clicked() {
                    if let Some(s_name) = state.project.streams.iter().find(|s| s.id == sid).map(|s| s.name_id.clone()) {
                        let idx = state.project.active_stream_table_set;
                        if let Some(set) = state.project.stream_table_sets.get_mut(idx) {
                            if !set.assigned_streams.contains(&s_name) {
                                set.assigned_streams.push(s_name);
                            }
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Rename Stream").clicked() {
                    if let Some(s) = state.project.streams.iter().find(|s| s.id == sid) {
                        state.inline_rename_target = Some(SelectedObject::Stream(sid));
                        state.inline_rename_buf = s.name_id.clone();
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete Stream (Del)").clicked() {
                    state.delete_selected();
                    ui.close_menu();
                }
            }
            SelectedObject::Table(tid) => {
                if ui.button("Edit Table (Table Editor)...").clicked() {
                    state.editing_table_id = Some(tid);
                    state.show_table_editor = true;
                    ui.close_menu();
                }
                if ui.button("Delete Table").clicked() {
                    state.delete_selected();
                    ui.close_menu();
                }
            }
            _ => {
                if ui.button("New Generic Unit Here").clicked() {
                    let id = state.alloc_id();
                    let name = format!("Unit {id}");
                    state.push_undo();
                    state.project.units.push(SimUnit::new(id, name, UnitKind::Generic, state.cursor_mm, page));
                    state.selected = SelectedObject::Unit(id);
                    ui.close_menu();
                }
                if ui.button(crate::i18n::text("review.184")).clicked() {
                    state.show_default_settings = true;
                    ui.close_menu();
                }
                if ui.button("Reset Zoom & Pan (100%)").clicked() {
                    state.zoom_pct = 100.0;
                    state.pan_mm = [-12.0, -4.0];
                    ui.close_menu();
                }
            }
        }
    });

    // Floating Inline Rename Modal Pill when triggered
    if let Some(target) = state.inline_rename_target {
        egui::Window::new("Rename")
            .collapsible(false)
            .resizable(false)
            .default_width(240.0)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("New NameID:");
                    let r = ui.text_edit_singleline(&mut state.inline_rename_buf);
                    r.request_focus();
                });
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.032")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let new_name = state.inline_rename_buf.trim().to_string();
                        if !new_name.is_empty() {
                            state.push_undo();
                            match target {
                                SelectedObject::Unit(uid) => {
                                    if let Some(u) = state.project.units.iter_mut().find(|u| u.id == uid) {
                                        u.name_id = new_name;
                                    }
                                }
                                SelectedObject::Stream(sid) => {
                                    let dup = state.project.streams.iter().any(|s| s.id != sid && s.name_id == new_name);
                                    if let Some(s) = state.project.streams.iter_mut().find(|s| s.id == sid) {
                                        s.name_id = new_name;
                                        s.has_duplicates = dup;
                                    }
                                }
                                _ => {}
                            }
                        }
                        state.inline_rename_target = None;
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        state.inline_rename_target = None;
                    }
                });
            });
    }
}

fn handle_tool_placement(state: &mut SimModuleState, snapped: [f32; 2], raw_mm: [f32; 2], page: usize) {
    match state.active_tool {
        ActiveTool::AddUnit(kind) => {
            state.push_undo();
            let id = state.alloc_id();
            let name = format!("Unit {id}");
            let unit = SimUnit::new(id, name, kind, snapped, page);
            state.project.units.push(unit);
            state.selected = SelectedObject::Unit(id);
            state.status_banner = format!("Added Unit #{id}");
        }
        ActiveTool::PlaceGalleryIcon(icon_idx) => {
            if let Some(meta) = UNIT_ICON_GALLERY.get(icon_idx) {
                state.push_undo();
                let id = state.alloc_id();
                let mut u = SimUnit::new(id, format!("{} {id}", meta.label), meta.default_kind, snapped, page);
                u.icon_key = Some(meta.key.to_string());
                state.project.units.push(u);
                state.selected = SelectedObject::Unit(id);
                state.status_banner = format!("Placed '{}' on canvas", meta.label);
            }
        }
        ActiveTool::AddStream => {
            let hit_uid = unit_at_mm(&state.project.units, page, raw_mm);
            if state.pending_stream_start_mm.is_none() {
                state.pending_stream_start_mm = Some(snapped);
                state.pending_stream_start_unit = hit_uid;
                state.status_banner = "Select destination point or unit to finish stream...".to_string();
                return;
            } else {
                let start_mm = state.pending_stream_start_mm.take().unwrap();
                let src_u = state.pending_stream_start_unit.take();
                let dst_u = hit_uid.filter(|d| Some(*d) != src_u);
                state.push_undo();
                let id = state.alloc_id();
                let mut stream = SimStream::new(id, format!("Stream {id}"), src_u, dst_u, start_mm, snapped, page);
                stream.mass_flow_t_h = 50.0;
                stream.enthalpy_kw = -1000.0;
                state.project.streams.push(stream);
                state.selected = SelectedObject::Stream(id);
                state.status_banner = format!("Created Stream #{id}");
            }
        }
        ActiveTool::AddTable => {
            state.push_undo();
            let id = state.alloc_id();
            state.project.tables.push(SimTableObj {
                id,
                pos: snapped,
                size: [72.0, 26.0],
                page,
                layer: 2,
                is_title_table: false,
                size_locked: false,
                cells: vec![
                    vec!["Stream".into(), "Mass (t/h)".into(), "Cu (%)".into()],
                    vec!["Feed".into(), "100.0".into(), "1.20".into()],
                    vec!["Product".into(), "100.0".into(), "1.20".into()],
                ],
                title: String::new(),
                organization_author: String::new(),
                hsc_version: String::new(),
                date: String::new(),
                logo_text: String::new(),
            });
            state.selected = SelectedObject::Table(id);
        }
        ActiveTool::AddTitleTable => {
            state.push_undo();
            let id = state.alloc_id();
            state.project.tables.push(SimTableObj {
                id,
                pos: snapped,
                size: [76.0, 28.0],
                page,
                layer: 2,
                is_title_table: true,
                size_locked: false,
                cells: Vec::new(),
                title: state.project.process_info.process_name.clone(),
                organization_author: "Soryntel / Process Engineering".into(),
                hsc_version: "10.3.1.0".into(),
                date: state.project.process_info.date.clone(),
                logo_text: "Logo".into(),
            });
            state.selected = SelectedObject::Table(id);
        }
        ActiveTool::AddHeaderLabel | ActiveTool::AddTextLabel | ActiveTool::AddTextBox
        | ActiveTool::AddPictureBox | ActiveTool::AddLine | ActiveTool::AddRect
        | ActiveTool::AddEllipse | ActiveTool::AddTriangle | ActiveTool::AddPolygon => {
            state.push_undo();
            let id = state.alloc_id();
            let (kind, text, size) = match state.active_tool {
                ActiveTool::AddHeaderLabel => (AnnotationKind::HeaderLabel, "Section Title", [52.0, 12.0]),
                ActiveTool::AddTextLabel => (AnnotationKind::TextLabel, "Process Label", [40.0, 10.0]),
                ActiveTool::AddTextBox => (AnnotationKind::MultilineBox, "Operating note:\nCheck balance", [54.0, 20.0]),
                ActiveTool::AddPictureBox => (AnnotationKind::PictureBox, "Flowsheet Logo", [38.0, 24.0]),
                ActiveTool::AddLine => (AnnotationKind::Line, "", [45.0, 20.0]),
                ActiveTool::AddRect => (AnnotationKind::Rectangle, "", [38.0, 22.0]),
                ActiveTool::AddEllipse => (AnnotationKind::Ellipse, "", [28.0, 28.0]),
                ActiveTool::AddTriangle => (AnnotationKind::Triangle, "", [28.0, 24.0]),
                _ => (AnnotationKind::Polygon, "", [32.0, 24.0]),
            };
            state.project.annotations.push(SimAnnotation {
                id,
                kind,
                pos: snapped,
                size,
                page,
                layer: 2,
                text: text.to_string(),
                font_name: state.selected_font_name.clone(),
                font_size: 9.0,
                bold: matches!(kind, AnnotationKind::HeaderLabel),
                italic: false,
                text_color: [0, 0, 0],
                fill_color: [248, 250, 254],
                line_color: [60, 68, 82],
                line_width: 0.3,
            });
            state.selected = SelectedObject::Annotation(id);
        }
        ActiveTool::Select | ActiveTool::Pan => {}
    }

    if !state.project.canvas_settings.persist_tool {
        state.active_tool = ActiveTool::Select;
    }
}
