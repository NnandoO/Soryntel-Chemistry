//! Shared Excel / Formula One style spreadsheet cell helpers for Modules 1, 2, and 3.

use eframe::egui;

pub const EXCEL_HEADER_BG: egui::Color32 = egui::Color32::from_rgb(240, 242, 245);
pub const EXCEL_SECTION_BG: egui::Color32 = egui::Color32::from_rgb(228, 225, 215);
pub const EXCEL_GRID_BORDER: egui::Color32 = egui::Color32::from_rgb(208, 213, 221);
pub const EXCEL_HEADER_BORDER: egui::Color32 = egui::Color32::from_rgb(176, 182, 192);
pub const EXCEL_ACTIVE_BORDER: egui::Color32 = egui::Color32::from_rgb(24, 24, 27);
pub const EXCEL_ALT_ROW_BG: egui::Color32 = egui::Color32::from_rgb(249, 250, 252);
pub const EXCEL_YELLOW_BG: egui::Color32 = egui::Color32::from_rgb(254, 249, 195);
pub const EXCEL_READONLY_BG: egui::Color32 = egui::Color32::from_rgb(236, 234, 226);

pub const CELL_H: f32 = 22.0;

/// Convert zero-based column index to Excel column letters (`0 -> A`, `25 -> Z`, `26 -> AA`).
pub fn col_letter(mut idx: usize) -> String {
    let mut s = String::new();
    loop {
        let rem = idx % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    s
}

/// Top-left corner or Excel column letter header cell (`A`, `B`, `C`, ...).
pub fn header_cell(ui: &mut egui::Ui, text: &str, width: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, CELL_H), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, EXCEL_HEADER_BG);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(11.5),
            egui::Color32::from_rgb(40, 40, 40),
        );
    }
}

/// Left row-number cell (`1`, `2`, `3`, ...).
pub fn row_num_cell(ui: &mut egui::Ui, row_num: usize, width: f32) {
    header_cell(ui, &row_num.to_string(), width);
}

/// Section / table column header cell (beige/gray HSC style).
pub fn section_cell(ui: &mut egui::Ui, text: &str, width: f32, center: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, CELL_H), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, EXCEL_SECTION_BG);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
        let (pos, align) = if center {
            (rect.center(), egui::Align2::CENTER_CENTER)
        } else {
            (rect.left_center() + egui::vec2(6.0, 0.0), egui::Align2::LEFT_CENTER)
        };
        p.text(
            pos,
            align,
            text,
            egui::FontId::proportional(11.5),
            egui::Color32::from_rgb(24, 24, 27),
        );
    }
}

/// Draw a bordered Excel text/number cell and return its `Response` for click selection.
pub fn text_cell(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    bg: egui::Color32,
    text_color: egui::Color32,
    right_align: bool,
    bold: bool,
    selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, CELL_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, bg);
        let stroke = if selected {
            egui::Stroke::new(2.0, EXCEL_ACTIVE_BORDER)
        } else {
            egui::Stroke::new(1.0, EXCEL_GRID_BORDER)
        };
        p.rect_stroke(rect, 0.0, stroke);
        if selected {
            let handle = egui::Rect::from_min_size(
                rect.right_bottom() - egui::vec2(4.0, 4.0),
                egui::vec2(4.0, 4.0),
            );
            p.rect_filled(handle, 0.0, EXCEL_ACTIVE_BORDER);
        }
        let font = if right_align {
            egui::FontId::monospace(11.5)
        } else if bold {
            egui::FontId::proportional(11.8)
        } else {
            egui::FontId::proportional(11.5)
        };
        let (pos, align) = if right_align {
            (rect.right_center() - egui::vec2(6.0, 0.0), egui::Align2::RIGHT_CENTER)
        } else {
            (rect.left_center() + egui::vec2(6.0, 0.0), egui::Align2::LEFT_CENTER)
        };
        p.text(pos, align, text, font, text_color);
    }
    resp
}

/// Interactive container cell with flat Excel border & background, suitable for `DragValue`, `TextEdit`, or `ComboBox`.
pub fn widget_cell(
    ui: &mut egui::Ui,
    width: f32,
    bg: egui::Color32,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, CELL_H), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, bg);
        let stroke = if selected {
            egui::Stroke::new(2.0, EXCEL_ACTIVE_BORDER)
        } else {
            egui::Stroke::new(1.0, EXCEL_GRID_BORDER)
        };
        p.rect_stroke(rect, 0.0, stroke);
        if selected {
            let handle = egui::Rect::from_min_size(
                rect.right_bottom() - egui::vec2(4.0, 4.0),
                egui::vec2(4.0, 4.0),
            );
            p.rect_filled(handle, 0.0, EXCEL_ACTIVE_BORDER);
        }
    }

    let inner = rect.shrink2(egui::vec2(3.0, 1.0));
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let vis = &mut child_ui.visuals_mut().widgets;
    vis.inactive.bg_fill = egui::Color32::TRANSPARENT;
    vis.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    vis.inactive.bg_stroke = egui::Stroke::NONE;
    vis.inactive.rounding = egui::Rounding::ZERO;
    vis.hovered.bg_fill = egui::Color32::from_rgb(239, 246, 255);
    vis.hovered.weak_bg_fill = egui::Color32::from_rgb(239, 246, 255);
    vis.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(147, 197, 253));
    vis.hovered.rounding = egui::Rounding::ZERO;
    vis.active.bg_fill = egui::Color32::WHITE;
    vis.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(37, 99, 235));
    vis.active.rounding = egui::Rounding::ZERO;
    add_contents(&mut child_ui);
}
