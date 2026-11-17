use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Vec2};
use soryntel_thermo::sim::model::UnitKind;

/// Render the 15 schematic equipment icons inside the `Icons` dock gallery and on unit shapes (Fig. 65).
pub fn draw_equipment_icon(painter: &Painter, rect: Rect, icon_key: &str) {
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    let metal_dark = Color32::from_rgb(76, 86, 98);
    let metal_light = Color32::from_rgb(208, 214, 222);
    let metal_mid = Color32::from_rgb(164, 174, 188);
    let steel_stroke = Stroke::new(1.0, Color32::from_rgb(52, 60, 72));
    let accent = Color32::from_rgb(90, 128, 172);

    match icon_key {
        "column_tall" | "packed_tower" => {
            let body = Rect::from_center_size(c + Vec2::new(0.0, h * 0.03), Vec2::new(w * 0.30, h * 0.72));
            painter.rect_filled(body, 4.0, metal_light);
            painter.rect_stroke(body, 4.0, steel_stroke);
            // shading band & nozzle
            let band = Rect::from_min_max(
                Pos2::new(body.left() + body.width() * 0.55, body.top() + 2.0),
                Pos2::new(body.right() - 1.5, body.bottom() - 2.0),
            );
            painter.rect_filled(band, 1.0, metal_mid);
            painter.line_segment(
                [Pos2::new(body.left() - w * 0.14, c.y - h * 0.14), Pos2::new(body.left(), c.y - h * 0.14)],
                steel_stroke,
            );
            painter.line_segment(
                [Pos2::new(body.right(), c.y + h * 0.16), Pos2::new(body.right() + w * 0.14, c.y + h * 0.16)],
                steel_stroke,
            );
            if icon_key == "packed_tower" {
                for dy in [-0.14_f32, 0.0, 0.14] {
                    painter.line_segment(
                        [Pos2::new(body.left() + 2.0, c.y + h * dy), Pos2::new(body.right() - 2.0, c.y + h * dy)],
                        Stroke::new(0.8, metal_dark),
                    );
                }
            }
        }
        "autoclave" | "rotary_kiln" => {
            let body = if icon_key == "rotary_kiln" {
                Rect::from_center_size(c + Vec2::new(0.0, h * 0.04), Vec2::new(w * 0.72, h * 0.28))
            } else {
                Rect::from_center_size(c + Vec2::new(0.0, h * 0.04), Vec2::new(w * 0.42, h * 0.56))
            };
            painter.rect_filled(body, 6.0, metal_light);
            painter.rect_stroke(body, 6.0, steel_stroke);
            painter.rect_filled(
                Rect::from_center_size(c - Vec2::new(0.0, h * 0.26), Vec2::new(w * 0.16, h * 0.14)),
                2.0,
                metal_dark,
            );
            painter.line_segment(
                [Pos2::new(body.left() + 4.0, body.bottom()), Pos2::new(body.left() + 4.0, rect.bottom() - 4.0)],
                Stroke::new(1.5, metal_dark),
            );
            painter.line_segment(
                [Pos2::new(body.right() - 4.0, body.bottom()), Pos2::new(body.right() - 4.0, rect.bottom() - 4.0)],
                Stroke::new(1.5, metal_dark),
            );
        }
        "sx_mixer_settler" | "sx_bank" | "leach_train" => {
            let stages = if icon_key == "sx_mixer_settler" { 3 } else { 4 };
            let step = (w * 0.78) / stages as f32;
            for i in 0..stages {
                let x = rect.left() + w * 0.11 + i as f32 * step;
                let top_r = Rect::from_min_size(Pos2::new(x + 1.0, c.y - h * 0.14), Vec2::new(step - 3.0, h * 0.34));
                painter.rect_filled(top_r, 1.5, metal_light);
                painter.rect_stroke(top_r, 1.5, steel_stroke);
                let chimney = Rect::from_min_size(Pos2::new(x + step * 0.3, c.y - h * 0.28), Vec2::new(step * 0.4, h * 0.14));
                painter.rect_filled(chimney, 1.0, metal_mid);
                painter.rect_stroke(chimney, 1.0, steel_stroke);
            }
            painter.line_segment(
                [Pos2::new(rect.left() + 4.0, c.y + h * 0.22), Pos2::new(rect.right() - 4.0, c.y + h * 0.22)],
                Stroke::new(1.3, metal_dark),
            );
        }
        "thickener" | "tankcell_200" => {
            let tank = Rect::from_center_size(c + Vec2::new(0.0, h * 0.06), Vec2::new(w * 0.62, h * 0.42));
            painter.rect_filled(tank, 2.0, metal_light);
            painter.rect_stroke(tank, 2.0, steel_stroke);
            // Bridge / motor atop tank
            painter.line_segment(
                [Pos2::new(tank.left() - 2.0, tank.top() - 4.0), Pos2::new(tank.right() + 2.0, tank.top() - 4.0)],
                Stroke::new(1.4, metal_dark),
            );
            painter.rect_filled(
                Rect::from_center_size(Pos2::new(c.x, tank.top() - 8.0), Vec2::new(w * 0.16, h * 0.14)),
                1.0,
                accent,
            );
            // Vertical baffles
            for frac in [-0.15_f32, 0.0, 0.15] {
                painter.line_segment(
                    [Pos2::new(c.x + w * frac, tank.top()), Pos2::new(c.x + w * frac, tank.bottom())],
                    Stroke::new(0.7, metal_dark),
                );
            }
        }
        "hydrocyclone" => {
            // Cylindrical feed box + conical lower apex
            let top_box = Rect::from_center_size(c - Vec2::new(0.0, h * 0.18), Vec2::new(w * 0.30, h * 0.20));
            painter.rect_filled(top_box, 1.0, metal_light);
            painter.rect_stroke(top_box, 1.0, steel_stroke);
            let pts = vec![
                Pos2::new(top_box.left(), top_box.bottom()),
                Pos2::new(top_box.right(), top_box.bottom()),
                Pos2::new(c.x + 2.5, rect.bottom() - 5.0),
                Pos2::new(c.x - 2.5, rect.bottom() - 5.0),
            ];
            painter.add(egui::Shape::convex_polygon(pts, metal_mid, steel_stroke));
            // Overflow pipe top
            painter.rect_filled(
                Rect::from_center_size(Pos2::new(c.x, top_box.top() - 4.0), Vec2::new(w * 0.10, 7.0)),
                0.5,
                metal_dark,
            );
        }
        "conditioner" | "reactor_tank" => {
            let vessel = Rect::from_center_size(c + Vec2::new(0.0, h * 0.05), Vec2::new(w * 0.48, h * 0.54));
            painter.rect_filled(vessel, 3.0, metal_light);
            painter.rect_stroke(vessel, 3.0, steel_stroke);
            // Impeller shaft & blades
            painter.line_segment(
                [Pos2::new(c.x, rect.top() + 5.0), Pos2::new(c.x, vessel.bottom() - 5.0)],
                Stroke::new(1.5, metal_dark),
            );
            painter.line_segment(
                [Pos2::new(c.x - w * 0.14, vessel.bottom() - 8.0), Pos2::new(c.x + w * 0.14, vessel.bottom() - 8.0)],
                Stroke::new(1.8, metal_dark),
            );
            painter.rect_filled(
                Rect::from_center_size(Pos2::new(c.x, rect.top() + 5.0), Vec2::new(w * 0.18, 6.0)),
                1.0,
                accent,
            );
        }
        _ => {
            // flotation_bank / filter_plant / dewatering: detailed plant frame
            let base = Rect::from_center_size(c + Vec2::new(0.0, h * 0.08), Vec2::new(w * 0.72, h * 0.44));
            painter.rect_filled(base, 2.0, metal_light);
            painter.rect_stroke(base, 2.0, steel_stroke);
            for i in 0..3 {
                let x = base.left() + 4.0 + i as f32 * (base.width() / 3.0);
                let cell = Rect::from_min_size(Pos2::new(x, base.top() - 5.0), Vec2::new(base.width() / 3.8, 7.0));
                painter.rect_filled(cell, 1.0, accent);
                painter.rect_stroke(cell, 1.0, steel_stroke);
            }
        }
    }
}

/// Crisp vector glyph inside small toolbar square buttons.
pub fn draw_toolbar_glyph(painter: &Painter, rect: Rect, kind: &str, active: bool) {
    let c = rect.center();
    let ink = if active { Color32::from_rgb(12, 64, 136) } else { Color32::from_rgb(48, 56, 66) };
    let stroke = Stroke::new(1.2, ink);

    match kind {
        "select" => {
            let pts = vec![
                Pos2::new(c.x - 4.0, c.y - 6.0),
                Pos2::new(c.x + 5.0, c.y + 1.0),
                Pos2::new(c.x + 0.5, c.y + 1.5),
                Pos2::new(c.x + 2.5, c.y + 6.0),
                Pos2::new(c.x - 1.0, c.y + 6.5),
                Pos2::new(c.x - 2.5, c.y + 2.0),
                Pos2::new(c.x - 4.0, c.y + 4.5),
            ];
            painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(70, 82, 96), stroke));
        }
        "pan" => {
            painter.circle_stroke(c, 4.5, stroke);
            painter.line_segment([c - Vec2::new(6.0, 0.0), c + Vec2::new(6.0, 0.0)], stroke);
            painter.line_segment([c - Vec2::new(0.0, 6.0), c + Vec2::new(0.0, 6.0)], stroke);
        }
        "unit_u" | "unit_r" | "unit_d" | "unit_tau" => {
            let box_r = Rect::from_center_size(c, Vec2::new(13.0, 12.0));
            let fill = match kind {
                "unit_r" => Color32::from_rgb(198, 224, 255),
                "unit_d" => Color32::from_rgb(248, 210, 210),
                "unit_tau" => Color32::from_rgb(220, 226, 232),
                _ => Color32::from_rgb(235, 244, 255),
            };
            painter.rect_filled(box_r, 1.5, fill);
            painter.rect_stroke(box_r, 1.5, stroke);
            let txt = match kind {
                "unit_u" => "U",
                "unit_r" => "R",
                "unit_d" => "D",
                _ => "τ",
            };
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                txt,
                egui::FontId::proportional(10.0),
                ink,
            );
        }
        "stream" => {
            let p1 = Pos2::new(c.x - 6.0, c.y + 4.0);
            let p2 = Pos2::new(c.x, c.y + 4.0);
            let p3 = Pos2::new(c.x, c.y - 4.0);
            let p4 = Pos2::new(c.x + 6.0, c.y - 4.0);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.circle_filled(p1, 2.0, Color32::BLACK);
            painter.circle_stroke(p4, 2.0, stroke);
        }
        "play" => {
            let pts = vec![
                Pos2::new(c.x - 4.0, c.y - 6.0),
                Pos2::new(c.x + 6.0, c.y),
                Pos2::new(c.x - 4.0, c.y + 6.0),
            ];
            painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(38, 122, 224), Stroke::NONE));
        }
        "pause" => {
            painter.rect_filled(Rect::from_center_size(c - Vec2::new(3.0, 0.0), Vec2::new(3.0, 10.0)), 0.0, Color32::from_rgb(200, 120, 20));
            painter.rect_filled(Rect::from_center_size(c + Vec2::new(3.0, 0.0), Vec2::new(3.0, 10.0)), 0.0, Color32::from_rgb(200, 120, 20));
        }
        "stop" => {
            painter.rect_filled(Rect::from_center_size(c, Vec2::new(9.0, 9.0)), 1.0, Color32::from_rgb(190, 42, 42));
        }
        "calc_order" => {
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "1→2",
                egui::FontId::proportional(9.5),
                Color32::from_rgb(16, 128, 64),
            );
        }
        "connectivity" => {
            painter.line_segment([c - Vec2::new(6.0, 3.0), c + Vec2::new(6.0, 3.0)], Stroke::new(2.0, Color32::from_rgb(24, 112, 210)));
            painter.line_segment([c - Vec2::new(6.0, -3.0), c + Vec2::new(6.0, -3.0)], Stroke::new(2.0, Color32::from_rgb(208, 32, 32)));
        }
        "table" => {
            let r = Rect::from_center_size(c, Vec2::new(13.0, 10.0));
            painter.rect_filled(r, 0.0, Color32::WHITE);
            painter.rect_stroke(r, 0.0, stroke);
            painter.line_segment([Pos2::new(r.left(), c.y), Pos2::new(r.right(), c.y)], Stroke::new(0.8, ink));
            painter.line_segment([Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom())], Stroke::new(0.8, ink));
        }
        "lock" => {
            let body = Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(9.0, 7.0));
            painter.rect_filled(body, 1.0, Color32::from_rgb(220, 168, 42));
            painter.rect_stroke(body, 1.0, Stroke::new(1.0, Color32::from_rgb(120, 80, 16)));
            painter.circle_stroke(c - Vec2::new(0.0, 3.0), 3.0, stroke);
        }
        _ => {}
    }
}

pub fn unit_kind_icon_fallback(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Generic => "tankcell_200",
        UnitKind::Reactions => "leach_train",
        UnitKind::Distributions => "hydrocyclone",
        UnitKind::Dynamic => "reactor_tank",
    }
}
