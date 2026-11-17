//! Module 12 UI — Periodic Chart & Elements Database (`Element4`).
//!
//! Two integrated views matching HSC6 `Fig. 1`, `Fig. 2`, and `Fig. 3`:
//! - **Periodic Chart**: 18-column interactive Periodic Table colored by category,
//!   symbol color by standard state (`Gas`=red, `Liquid`=green, `Solid`=black),
//!   metalloid staircase divider, lanthanide/actinide connector tiles (`57-71`, `89-103`),
//!   proportional 3D-shaded property spheres (`Density`, etc.), bottom category/state legend,
//!   bottom toolbar (`-- Select property --`, `Clear`, `Print`, `Database`, `Show/Hide Legend`, `Show/Hide Data`),
//!   and docked right Excel-style element data sheet + double-click detail/Maindb6 popup.
//! - **Elements Database Spreadsheet**: `File/Edit/Insert/Delete/Format/Help` menu bar,
//!   Excel formula bar, editable `Property | Units | Diagram | H..Lr` grid with
//!   `Color RGB-number` swatch tinting, `Fix Left 3 Columns` freeze toggle, and `Exit`.

use eframe::egui;
use soryntel_application::SoryntelApplication;
use soryntel_thermo::{
    elements::{
        find_element_index_by_symbol, ElementCategory, ElementWorkbook, PeriodicDiagramData,
        StandardState, PERIODIC_ELEMENTS,
    },
    ChemicalSpecies, Phase, SpeciesCatalogPort,
};
use std::collections::HashMap;

use crate::excel_grid::{
    col_letter, EXCEL_ACTIVE_BORDER, EXCEL_GRID_BORDER, EXCEL_HEADER_BG, EXCEL_HEADER_BORDER,
    EXCEL_READONLY_BG, EXCEL_SECTION_BG,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementsViewMode {
    PeriodicChart,
    DatabaseSpreadsheet,
}

#[derive(Debug, Clone)]
pub struct CellPos {
    /// 0 = Header sub-row (Property / Units / Diagram / Symbols), 1..=rows.len() = property row index + 1
    pub row: usize,
    /// 0 = Property, 1 = Units, 2 = Diagram, 3.. = element index + 3
    pub col: usize,
}

pub struct ElementsModuleState {
    pub view_mode: ElementsViewMode,
    pub workbook: ElementWorkbook,
    pub selected_element_idx: usize, // 0..102 (defaults to 0 = H)
    pub selected_diagram_row: Option<usize>,
    pub diagram_cache: Option<PeriodicDiagramData>,
    pub show_legend: bool,
    pub show_data: bool,

    // Spreadsheet state
    pub active_cell: CellPos,
    pub formula_buf: String,
    pub fix_left_3_cols: bool,
    pub col_widths_base: [f32; 3], // Property, Units, Diagram
    pub elem_col_width: f32,

    // Modals & status
    pub double_click_modal_idx: Option<usize>,
    pub modal_db_species: Vec<ChemicalSpecies>,
    pub show_insert_row_modal: bool,
    pub new_row_name: String,
    pub new_row_units: String,
    pub new_row_diagram: bool,
    pub show_about_modal: bool,
    pub status_msg: String,
}

impl Default for ElementsModuleState {
    fn default() -> Self {
        let workbook = SoryntelApplication::default_elements_workbook();
        let mut s = Self {
            view_mode: ElementsViewMode::PeriodicChart,
            workbook,
            selected_element_idx: 0, // Hydrogen ('H'), matching Fig. 1
            selected_diagram_row: None,
            diagram_cache: None,
            show_legend: false,
            show_data: true,
            active_cell: CellPos { row: 1, col: 0 }, // A2 ("Atomic Number"), matching Fig. 2
            formula_buf: String::new(),
            fix_left_3_cols: true,
            col_widths_base: [185.0, 74.0, 56.0],
            elem_col_width: 68.0,
            double_click_modal_idx: None,
            modal_db_species: Vec::new(),
            show_insert_row_modal: false,
            new_row_name: String::new(),
            new_row_units: String::new(),
            new_row_diagram: true,
            show_about_modal: false,
            status_msg: "Ready — Click an element to inspect properties, double-click for exact value & DB species.".to_string(),
        };
        s.sync_formula_from_active_cell();
        s
    }
}

impl ElementsModuleState {
    fn recompute_diagram(&mut self) {
        self.diagram_cache = match self.selected_diagram_row {
            Some(r) => match SoryntelApplication::compute_elements_diagram(&self.workbook, r) {
                Ok(d) => {
                    self.status_msg = format!(
                        "{} ({}) — {} elements plotted | Max: {:.4} | Min: {:.4}",
                        d.property_name, d.units, d.valid_count, d.max_val, d.min_val
                    );
                    Some(d)
                }
                Err(e) => {
                    self.status_msg = format!("Diagram error: {e}");
                    None
                }
            },
            None => None,
        };
    }

    fn clear_diagram(&mut self) {
        self.selected_diagram_row = None;
        self.diagram_cache = None;
        self.status_msg = "Property diagram cleared.".to_string();
    }

    fn cell_value(&self, row: usize, col: usize) -> String {
        if row == 0 {
            return match col {
                0 => "Property".to_string(),
                1 => "Units".to_string(),
                2 => "Diagram".to_string(),
                c => self
                    .workbook
                    .element_symbols
                    .get(c - 3)
                    .cloned()
                    .unwrap_or_default(),
            };
        }
        let Some(r) = self.workbook.rows.get(row - 1) else {
            return String::new();
        };
        match col {
            0 => r.property.clone(),
            1 => r.units.clone(),
            2 => if r.diagram { "TRUE" } else { "FALSE" }.to_string(),
            c => r.values.get(c - 3).cloned().unwrap_or_default(),
        }
    }

    fn set_cell_value(&mut self, row: usize, col: usize, val: String) {
        if row == 0 {
            return;
        }
        let prop_idx = row - 1;
        let Some(r) = self.workbook.rows.get_mut(prop_idx) else {
            return;
        };
        match col {
            0 => r.property = val,
            1 => r.units = val,
            2 => {
                let u = val.trim().to_ascii_uppercase();
                r.diagram = matches!(u.as_str(), "TRUE" | "1" | "YES");
            }
            c => {
                if let Some(slot) = r.values.get_mut(c - 3) {
                    *slot = val;
                }
            }
        }
        if self.selected_diagram_row == Some(prop_idx) {
            self.recompute_diagram();
        }
    }

    fn sync_formula_from_active_cell(&mut self) {
        self.formula_buf = self.cell_value(self.active_cell.row, self.active_cell.col);
    }

    fn select_spreadsheet_cell(&mut self, row: usize, col: usize) {
        self.active_cell = CellPos { row, col };
        self.sync_formula_from_active_cell();
    }

    fn open_element_modal(&mut self, idx: usize, catalog: &dyn SpeciesCatalogPort) {
        self.selected_element_idx = idx;
        self.double_click_modal_idx = Some(idx);
        if let Some(spec) = PERIODIC_ELEMENTS.get(idx) {
            self.modal_db_species = SoryntelApplication::lookup_element_species(catalog, spec.symbol);
        } else {
            self.modal_db_species.clear();
        }
    }

    fn export_csv(&mut self) {
        let csv = self.workbook.to_csv();
        let path = "/tmp/soryntel_elements_export.csv";
        match crate::storage::write_atomic(path, csv) {
            Ok(_) => {
                self.status_msg = format!("Exported Elements workbook to {path}");
            }
            Err(e) => {
                self.status_msg = format!("CSV export failed: {e}");
            }
        }
    }

    fn print_periodic_svg(&mut self) {
        let path = "/tmp/soryntel_periodic_chart.svg";
        let title = self
            .diagram_cache
            .as_ref()
            .map(|d| format!("Periodic Chart — {}", d.property_name))
            .unwrap_or_else(|| "Periodic Chart — Elements".to_string());
        let mut svg = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1260" height="720" font-family="sans-serif">"#,
        );
        svg.push_str(r##"<rect width="100%" height="100%" fill="#f4f6f9"/>"##);
        svg.push_str(&format!(
            r##"<text x="30" y="28" font-size="18" font-weight="bold" fill="#18222d">{}</text>"##,
            xml_escape(&title)
        ));
        let cell_w = 64.0f32;
        let cell_h = 56.0f32;
        let gap = 3.0f32;
        let ox = 28.0f32;
        let oy = 46.0f32;
        for (idx, el) in PERIODIC_ELEMENTS.iter().enumerate() {
            let y_extra = if el.grid_row >= 7 { 16.0 } else { 0.0 };
            let x = ox + el.grid_col as f32 * (cell_w + gap);
            let y = oy + el.grid_row as f32 * (cell_h + gap) + y_extra;
            let fill = category_rgb_hex(el.category);
            let sym_col = state_hex(el.state);
            svg.push_str(&format!(
                r##"<rect x="{x:.1}" y="{y:.1}" width="{cell_w:.1}" height="{cell_h:.1}" fill="{fill}" stroke="#9aa3b2" stroke-width="1"/>"##
            ));
            svg.push_str(&format!(
                r##"<text x="{:.1}" y="{:.1}" font-size="15" font-weight="bold" fill="{sym_col}">{}</text>"##,
                x + 5.0,
                y + 18.0,
                xml_escape(el.symbol)
            ));
            svg.push_str(&format!(
                r##"<text x="{:.1}" y="{:.1}" font-size="10" text-anchor="end" fill="#222">{}</text>"##,
                x + cell_w - 4.0,
                y + 13.0,
                el.z
            ));
            svg.push_str(&format!(
                r##"<text x="{:.1}" y="{:.1}" font-size="8.5" fill="#222">{}</text>"##,
                x + 4.0,
                y + 38.0,
                xml_escape(el.name)
            ));
            if let Some(diag) = &self.diagram_cache {
                if let Some(pt) = diag.points.get(idx) {
                    if pt.norm_diameter > 0.0 {
                        let r = 0.5 * cell_w.min(cell_h) * 0.88 * pt.norm_diameter as f32;
                        let cx = x + cell_w * 0.5;
                        let cy = y + cell_h * 0.5;
                        let sc = if pt.is_negative { "#d23030" } else { "#2d46d2" };
                        svg.push_str(&format!(
                            r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{sc}" fill-opacity="0.58" stroke="#1b2a78" stroke-width="0.8"/>"##
                        ));
                    }
                }
            }
        }
        svg.push_str("</svg>");
        match crate::storage::write_atomic(path, svg) {
            Ok(_) => {
                self.status_msg = format!("Printable Periodic Chart SVG saved to {path}");
            }
            Err(e) => {
                self.status_msg = format!("Print SVG export failed: {e}");
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, catalog: &dyn SpeciesCatalogPort) {
        match self.view_mode {
            ElementsViewMode::PeriodicChart => self.render_periodic_chart_view(ctx, catalog),
            ElementsViewMode::DatabaseSpreadsheet => self.render_spreadsheet_view(ctx),
        }
        self.render_modals(ctx);
    }

    fn render_periodic_chart_view(
        &mut self,
        ctx: &egui::Context,
        catalog: &dyn SpeciesCatalogPort,
    ) {
        let diag_map: HashMap<usize, (f64, bool, Option<f64>)> = self
            .diagram_cache
            .as_ref()
            .map(|d| {
                d.points
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i, (p.norm_diameter, p.is_negative, p.raw_value)))
                    .collect()
            })
            .unwrap_or_default();

        // Bottom Toolbar & Optional Legend Bar inside BottomPanel so center fills remaining area cleanly
        egui::TopBottomPanel::bottom("elements_periodic_bottom")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(237, 239, 243))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0)),
            )
            .show(ctx, |ui| {
                if self.show_legend {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 4.0);
                        for cat in ElementCategory::LEGEND_ORDER {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(26.0, 22.0),
                                egui::Sense::hover(),
                            );
                            if ui.is_rect_visible(rect) {
                                let p = ui.painter();
                                p.rect_filled(rect, 2.0, category_color(cat));
                                p.rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 174, 184)),
                                );
                            }
                            ui.label(
                                egui::RichText::new(cat.label().replace('\n', " "))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(24, 28, 34)),
                            );
                        }
                        ui.separator();
                        state_legend_chip(ui, "Br", state_color(StandardState::Liquid), "Liquid");
                        state_legend_chip(ui, "C", state_color(StandardState::Solid), "Solid");
                        state_legend_chip(ui, "He", state_color(StandardState::Gas), "Gas");
                    });
                    ui.add_space(4.0);
                }

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

                    let selected_label = self
                        .selected_diagram_row
                        .and_then(|idx| self.workbook.rows.get(idx))
                        .map(|r| r.property.as_str())
                        .unwrap_or("-- Select property --");

                    let mut next_selection = self.selected_diagram_row;
                    egui::ComboBox::from_id_salt("elements_property_combo")
                        .width(210.0)
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.selected_diagram_row.is_none(),
                                    "-- Select property --",
                                )
                                .clicked()
                            {
                                next_selection = None;
                            }
                            for idx in self.workbook.diagram_property_indices() {
                                if let Some(row) = self.workbook.rows.get(idx) {
                                    let lbl = if row.units.is_empty() {
                                        row.property.clone()
                                    } else {
                                        format!("{} ({})", row.property, row.units)
                                    };
                                    if ui
                                        .selectable_label(
                                            self.selected_diagram_row == Some(idx),
                                            lbl,
                                        )
                                        .clicked()
                                    {
                                        next_selection = Some(idx);
                                    }
                                }
                            }
                        });

                    if next_selection != self.selected_diagram_row {
                        self.selected_diagram_row = next_selection;
                        self.recompute_diagram();
                    }

                    if hsc_toolbar_button(ui, "Clear", 74.0, false).clicked() {
                        self.clear_diagram();
                    }
                    if hsc_toolbar_button(ui, "Print", 74.0, false).clicked() {
                        self.print_periodic_svg();
                    }
                    if hsc_toolbar_button(ui, "Database", 84.0, false).clicked() {
                        self.view_mode = ElementsViewMode::DatabaseSpreadsheet;
                        self.sync_formula_from_active_cell();
                    }
                    let legend_lbl = if self.show_legend {
                        "Hide Legend"
                    } else {
                        "Show Legend"
                    };
                    if hsc_toolbar_button(ui, legend_lbl, 94.0, self.show_legend).clicked() {
                        self.show_legend = !self.show_legend;
                    }
                    let data_lbl = if self.show_data {
                        "Hide Data"
                    } else {
                        "Show Data"
                    };
                    if hsc_toolbar_button(ui, data_lbl, 86.0, self.show_data).clicked() {
                        self.show_data = !self.show_data;
                    }

                    ui.separator();
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(72, 78, 88)),
                    );
                });
            });

        // Docked Right Data Panel (Fig. 1)
        if self.show_data {
            let panel_w = (ctx.screen_rect().width() * 0.27).clamp(220.0, 325.0);
            egui::SidePanel::right("elements_right_data_panel")
                .exact_width(panel_w)
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                        .inner_margin(egui::Margin::same(3.0)),
                )
                .show(ctx, |ui| {
                    self.render_right_data_sheet(ui, panel_w - 8.0);
                });
        }

        // Main Central Periodic Chart Canvas
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(241, 243, 246))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                let avail = ui.available_size();
                if avail.x < 180.0 || avail.y < 140.0 {
                    ui.label("Resize window larger to display Periodic Chart.");
                    return;
                }

                // Frame box like HSC6 Fig. 1 / Fig. 3
                let (canvas_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
                let painter = ui.painter().with_clip_rect(canvas_rect);
                painter.rect_filled(
                    canvas_rect,
                    2.0,
                    egui::Color32::from_rgb(248, 249, 251),
                );
                painter.rect_stroke(
                    canvas_rect,
                    2.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(196, 202, 212)),
                );

                let pad = 8.0f32;
                let inner = canvas_rect.shrink(pad);
                let cols = 18.0f32;
                let gap = (inner.width() * 0.003).clamp(1.5, 4.0);
                let header_h = (inner.height() * 0.048).clamp(16.0, 24.0);
                let f_block_gap = (inner.height() * 0.032).clamp(6.0, 18.0);

                let max_cell_w = (inner.width() - (cols - 1.0) * gap) / cols;
                let max_cell_h =
                    (inner.height() - header_h - 6.0 - 8.0 * gap - f_block_gap) / 9.0;
                // Maintain HSC-like tile proportions (~1 : 1.08 aspect or square-ish)
                let cell_w = max_cell_w.min(max_cell_h * 1.08).max(16.0);
                let cell_h = (cell_w / 0.98).min(max_cell_h).max(16.0);

                let total_w = cols * cell_w + (cols - 1.0) * gap;
                let total_h = header_h + 6.0 + 9.0 * cell_h + 8.0 * gap + f_block_gap;
                let origin_x = inner.left() + ((inner.width() - total_w) * 0.5).max(0.0);
                let origin_y = inner.top() + ((inner.height() - total_h) * 0.5).max(0.0);

                let cell_rect = |row: usize, col: usize| -> egui::Rect {
                    let extra_y = if row >= 7 { f_block_gap } else { 0.0 };
                    let x = origin_x + col as f32 * (cell_w + gap);
                    let y = origin_y + header_h + 6.0 + row as f32 * (cell_h + gap) + extra_y;
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_w, cell_h))
                };

                // 1. Group column headers 1..18
                for c in 0..18usize {
                    let hr = egui::Rect::from_min_size(
                        egui::pos2(origin_x + c as f32 * (cell_w + gap), origin_y),
                        egui::vec2(cell_w, header_h),
                    );
                    painter.rect_filled(hr, 1.5, egui::Color32::from_rgb(218, 222, 228));
                    painter.rect_stroke(
                        hr,
                        1.5,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(172, 178, 188)),
                    );
                    painter.text(
                        hr.center(),
                        egui::Align2::CENTER_CENTER,
                        (c + 1).to_string(),
                        egui::FontId::proportional((header_h * 0.55).clamp(9.0, 12.5)),
                        egui::Color32::from_rgb(30, 34, 42),
                    );
                }

                // 2. Lanthanide & Actinide connector lines + placeholder tiles in Col 2 (Group 3)
                let la_tile = cell_rect(5, 2);
                let ac_tile = cell_rect(6, 2);
                let la_row_start = cell_rect(7, 3);
                let ac_row_start = cell_rect(8, 3);

                // Warm beige connector path from 57-71 tile to Lanthanides row
                let connector_stroke =
                    egui::Stroke::new((gap * 1.4).clamp(3.0, 5.0), egui::Color32::from_rgb(225, 214, 206));
                painter.line_segment(
                    [
                        egui::pos2(la_tile.center().x, la_tile.bottom()),
                        egui::pos2(la_tile.center().x, la_row_start.center().y),
                    ],
                    connector_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(la_tile.center().x, la_row_start.center().y),
                        egui::pos2(la_row_start.left(), la_row_start.center().y),
                    ],
                    connector_stroke,
                );

                // Lavender connector path from 89-103 tile to Actinides row
                let ac_connector_stroke =
                    egui::Stroke::new((gap * 1.4).clamp(3.0, 5.0), egui::Color32::from_rgb(176, 178, 222));
                painter.line_segment(
                    [
                        egui::pos2(ac_tile.center().x, ac_tile.bottom()),
                        egui::pos2(ac_tile.center().x, ac_row_start.center().y),
                    ],
                    ac_connector_stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(ac_tile.center().x, ac_row_start.center().y),
                        egui::pos2(ac_row_start.left(), ac_row_start.center().y),
                    ],
                    ac_connector_stroke,
                );

                // Placeholder tiles 57-71 & 89-103
                draw_range_tile(&painter, la_tile, "57-\n71", egui::Color32::from_rgb(234, 224, 218));
                draw_range_tile(&painter, ac_tile, "89-\n103", egui::Color32::from_rgb(194, 196, 236));

                // 3. Draw all 103 Element Tiles
                for (idx, el) in PERIODIC_ELEMENTS.iter().enumerate() {
                    let rect = cell_rect(el.grid_row, el.grid_col);
                    let resp = ui.interact(
                        rect,
                        egui::Id::new(("periodic_el", el.z)),
                        egui::Sense::click(),
                    );
                    if resp.clicked() {
                        self.selected_element_idx = idx;
                    }
                    if resp.double_clicked() {
                        self.open_element_modal(idx, catalog);
                    }

                    let is_selected = self.selected_element_idx == idx;
                    let base_bg = category_color(el.category);
                    let bg = if resp.hovered() && !is_selected {
                        lighten(base_bg, 14)
                    } else {
                        base_bg
                    };

                    painter.rect_filled(rect, 1.5, bg);
                    // Subtle HSC6 top highlight bevel
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            rect.min + egui::vec2(1.0, 1.0),
                            egui::pos2(rect.max.x - 1.0, rect.min.y + rect.height() * 0.25),
                        ),
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 55),
                    );

                    // Proportional sphere overlay (Fig. 3) drawn translucent inside the tile
                    if let Some(&(norm_d, is_neg, _raw_val)) = diag_map.get(&idx) {
                        if norm_d > 0.0 {
                            let radius = (0.5 * cell_w.min(cell_h) * 0.90 * norm_d as f32)
                                .max(2.5)
                                .min(cell_w.min(cell_h) * 0.48);
                            let center = rect.center();
                            let fill = if is_neg {
                                egui::Color32::from_rgba_unmultiplied(212, 46, 46, 165)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(46, 70, 212, 165)
                            };
                            let rim = if is_neg {
                                egui::Color32::from_rgb(138, 22, 22)
                            } else {
                                egui::Color32::from_rgb(28, 42, 138)
                            };
                            painter.circle_filled(center, radius, fill);
                            if radius >= 5.0 {
                                // Upper-left specular reflection for 3D glossy look (Fig. 3)
                                painter.circle_filled(
                                    center - egui::vec2(radius * 0.28, radius * 0.28),
                                    radius * 0.34,
                                    egui::Color32::from_rgba_unmultiplied(185, 210, 255, 110),
                                );
                            }
                            painter.circle_stroke(center, radius, egui::Stroke::new(0.8, rim));
                        }
                    }

                    let border = if is_selected {
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(20, 48, 128))
                    } else {
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(162, 170, 182))
                    };
                    painter.rect_stroke(rect, 1.5, border);

                    // Tile text clipped inside rect
                    let tp = painter.with_clip_rect(rect.shrink(1.5));
                    let sym_font = (cell_h * 0.33).clamp(9.0, 15.5);
                    tp.text(
                        rect.left_top() + egui::vec2(3.0, 2.0),
                        egui::Align2::LEFT_TOP,
                        el.symbol,
                        egui::FontId::proportional(sym_font),
                        state_color(el.state),
                    );
                    if cell_w >= 26.0 {
                        tp.text(
                            rect.right_top() + egui::vec2(-3.0, 2.0),
                            egui::Align2::RIGHT_TOP,
                            el.z.to_string(),
                            egui::FontId::proportional((cell_h * 0.19).clamp(7.0, 10.0)),
                            egui::Color32::from_rgb(28, 32, 40),
                        );
                    }
                    if cell_w >= 34.0 && cell_h >= 34.0 {
                        let sub_font = (cell_h * 0.145).clamp(6.8, 8.8);
                        tp.text(
                            rect.left_bottom() + egui::vec2(2.5, -(sub_font + 3.0)),
                            egui::Align2::LEFT_BOTTOM,
                            el.name,
                            egui::FontId::proportional(sub_font),
                            egui::Color32::from_rgb(24, 26, 30),
                        );
                        tp.text(
                            rect.left_bottom() + egui::vec2(2.5, -1.5),
                            egui::Align2::LEFT_BOTTOM,
                            format!("{:.4}", el.atomic_weight)
                                .trim_end_matches('0')
                                .trim_end_matches('.'),
                            egui::FontId::proportional(sub_font * 0.95),
                            egui::Color32::from_rgb(40, 44, 52),
                        );
                    }

                    // Rich hover tooltip matching active diagram property
                    if resp.hovered() {
                        let extra = if let Some(&(_, _, Some(rv))) = diag_map.get(&idx) {
                            let pname = self
                                .diagram_cache
                                .as_ref()
                                .map(|d| d.property_name.as_str())
                                .unwrap_or("");
                            let punits = self
                                .diagram_cache
                                .as_ref()
                                .map(|d| d.units.as_str())
                                .unwrap_or("");
                            format!("\n{pname}: {rv} {punits}")
                        } else {
                            String::new()
                        };
                        resp.on_hover_text(format!(
                            "{} ({}) — Z = {}, Ar = {:.4}{extra}\n(Click: Data Sheet | Double-Click: Details & DB)",
                            el.name, el.symbol, el.z, el.atomic_weight
                        ));
                    }
                }

                // 4. Stepped Metalloid Staircase Line (Fig. 1 & Fig. 3)
                let half_gap = gap * 0.5;
                let h_mid = |top_r: usize, col: usize| -> f32 {
                    cell_rect(top_r, col).bottom() + half_gap
                };
                let v_mid = |row: usize, left_c: usize| -> f32 {
                    cell_rect(row, left_c).right() + half_gap
                };
                let x_b_left = cell_rect(1, 12).left() - half_gap;
                let x_12_13 = v_mid(2, 12);
                let x_13_14 = v_mid(3, 13);
                let x_14_15 = v_mid(4, 14);
                let x_15_16 = v_mid(5, 15);
                let y_1_2 = h_mid(1, 12);
                let y_2_3 = h_mid(2, 13);
                let y_3_4 = h_mid(3, 14);
                let y_4_5 = h_mid(4, 15);
                let y_5_bot = cell_rect(5, 15).bottom() + half_gap;

                let stair_pts = [
                    egui::pos2(x_b_left, y_1_2),
                    egui::pos2(x_12_13, y_1_2),
                    egui::pos2(x_12_13, y_2_3),
                    egui::pos2(x_13_14, y_2_3),
                    egui::pos2(x_13_14, y_3_4),
                    egui::pos2(x_14_15, y_3_4),
                    egui::pos2(x_14_15, y_4_5),
                    egui::pos2(x_15_16, y_4_5),
                    egui::pos2(x_15_16, y_5_bot),
                ];
                for pair in stair_pts.windows(2) {
                    painter.line_segment(
                        [pair[0], pair[1]],
                        egui::Stroke::new(3.2, egui::Color32::from_rgb(132, 138, 148)),
                    );
                    painter.line_segment(
                        [pair[0], pair[1]],
                        egui::Stroke::new(1.2, egui::Color32::from_rgb(245, 247, 250)),
                    );
                }
            });
    }

    fn render_right_data_sheet(&self, ui: &mut egui::Ui, avail_w: f32) {
        let el_idx = self.selected_element_idx.min(self.workbook.element_symbols.len() - 1);
        let sym = &self.workbook.element_symbols[el_idx];

        let w_prop = (avail_w * 0.56).clamp(110.0, 190.0);
        let w_unit = (avail_w * 0.21).clamp(44.0, 74.0);
        let w_val = (avail_w - w_prop - w_unit).max(48.0);
        let row_h = 19.5f32;

        // Header row: Property | Units | <Symbol> (Fig. 1)
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            draw_sheet_cell(
                ui,
                "Property",
                w_prop,
                row_h,
                EXCEL_HEADER_BG,
                egui::Color32::from_rgb(28, 32, 38),
                true,
                true,
            );
            draw_sheet_cell(
                ui,
                "Units",
                w_unit,
                row_h,
                EXCEL_HEADER_BG,
                egui::Color32::from_rgb(28, 32, 38),
                true,
                true,
            );
            draw_sheet_cell(
                ui,
                sym,
                w_val,
                row_h,
                EXCEL_HEADER_BG,
                egui::Color32::from_rgb(20, 48, 128),
                true,
                true,
            );
        });

        egui::ScrollArea::vertical()
            .id_salt("elements_right_sheet_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                for (r_idx, row) in self.workbook.rows.iter().enumerate() {
                    let val = row.values.get(el_idx).map(String::as_str).unwrap_or("-");
                    let is_active_diag = self.selected_diagram_row == Some(r_idx);
                    let row_bg = if is_active_diag {
                        egui::Color32::from_rgb(235, 243, 255)
                    } else if r_idx % 2 == 1 {
                        egui::Color32::from_rgb(250, 251, 253)
                    } else {
                        egui::Color32::WHITE
                    };
                    let val_bg = if row.property == "Color RGB-number" {
                        parse_vb6_rgb(val).unwrap_or(row_bg)
                    } else {
                        row_bg
                    };
                    let val_fg = contrasting_text_color(val_bg);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        draw_sheet_cell(
                            ui,
                            &row.property,
                            w_prop,
                            row_h,
                            row_bg,
                            egui::Color32::from_rgb(24, 24, 27),
                            true,
                            false,
                        );
                        draw_sheet_cell(
                            ui,
                            &row.units,
                            w_unit,
                            row_h,
                            row_bg,
                            egui::Color32::from_rgb(48, 52, 60),
                            true,
                            false,
                        );
                        draw_sheet_cell(
                            ui,
                            val,
                            w_val,
                            row_h,
                            val_bg,
                            val_fg,
                            false,
                            true,
                        );
                    });
                }
            });
    }

    fn render_spreadsheet_view(&mut self, ctx: &egui::Context) {
        // Top Classic Menu Bar: File | Edit | Insert | Delete | Format | Help (Fig. 2)
        egui::TopBottomPanel::top("elements_spreadsheet_menu")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(242, 244, 247))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("Periodic Chart (Return)").clicked() {
                            self.view_mode = ElementsViewMode::PeriodicChart;
                            ui.close_menu();
                        }
                        if ui.button("Plot Selected Row as Diagram").clicked() {
                            if self.active_cell.row > 0 {
                                let p_idx = self.active_cell.row - 1;
                                if let Some(r) = self.workbook.rows.get_mut(p_idx) {
                                    r.diagram = true;
                                }
                                self.selected_diagram_row = Some(p_idx);
                                self.recompute_diagram();
                                self.view_mode = ElementsViewMode::PeriodicChart;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export CSV (/tmp/soryntel_elements_export.csv)").clicked() {
                            self.export_csv();
                            ui.close_menu();
                        }
                        if ui.button("Reset Default Element4 Database").clicked() {
                            self.workbook = SoryntelApplication::default_elements_workbook();
                            self.recompute_diagram();
                            self.sync_formula_from_active_cell();
                            self.status_msg = "Reset workbook to default Element4 (103 elements).".into();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.006"), |ui| {
                        if ui.button("Copy Active Cell").clicked() {
                            let val = self.cell_value(self.active_cell.row, self.active_cell.col);
                            ui.ctx().copy_text(val);
                            ui.close_menu();
                        }
                        if ui.button("Toggle Diagram TRUE/FALSE on Row").clicked() {
                            if self.active_cell.row > 0 {
                                if let Some(r) = self.workbook.rows.get_mut(self.active_cell.row - 1) {
                                    r.diagram = !r.diagram;
                                }
                                self.sync_formula_from_active_cell();
                            }
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.010"), |ui| {
                        if ui.button("Insert Custom Property Row...").clicked() {
                            self.new_row_name = "Custom Property".into();
                            self.new_row_units.clear();
                            self.new_row_diagram = true;
                            self.show_insert_row_modal = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.013"), |ui| {
                        if ui.button("Delete Active Row").clicked() {
                            if self.active_cell.row > 0 && self.workbook.rows.len() > 1 {
                                let del_idx = self.active_cell.row - 1;
                                self.workbook.rows.remove(del_idx);
                                if self.selected_diagram_row == Some(del_idx) {
                                    self.clear_diagram();
                                } else if let Some(sel) = self.selected_diagram_row {
                                    if sel > del_idx {
                                        self.selected_diagram_row = Some(sel - 1);
                                    }
                                }
                                self.active_cell.row = self.active_cell.row.min(self.workbook.rows.len());
                                self.sync_formula_from_active_cell();
                            }
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.019"), |ui| {
                        if ui
                            .checkbox(&mut self.fix_left_3_cols, "Fix Left 3 Columns (Property | Units | Diagram)")
                            .clicked()
                        {
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button("About Elements Module").clicked() {
                            self.show_about_modal = true;
                            ui.close_menu();
                        }
                    });
                });

                ui.add_space(2.0);
                // Excel Formula Bar: [ A2 v ] [ cell value ]
                ui.horizontal(|ui| {
                    let addr = format!(
                        "{}{}",
                        col_letter(self.active_cell.col),
                        self.active_cell.row + 1
                    );
                    let (addr_rect, _) =
                        ui.allocate_exact_size(egui::vec2(72.0, 22.0), egui::Sense::hover());
                    if ui.is_rect_visible(addr_rect) {
                        let p = ui.painter();
                        p.rect_filled(addr_rect, 1.0, egui::Color32::WHITE);
                        p.rect_stroke(addr_rect, 1.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
                        p.text(
                            addr_rect.left_center() + egui::vec2(6.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            addr,
                            egui::FontId::proportional(11.5),
                            egui::Color32::from_rgb(24, 24, 27),
                        );
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.formula_buf)
                            .desired_width(ui.available_width() - 95.0)
                            .font(egui::FontId::proportional(12.0)),
                    );
                    if resp.changed() {
                        self.set_cell_value(
                            self.active_cell.row,
                            self.active_cell.col,
                            self.formula_buf.clone(),
                        );
                    }
                    if ui.button("Periodic Chart").clicked() {
                        if self.active_cell.row > 0 {
                            let idx = self.active_cell.row - 1;
                            if self.workbook.rows.get(idx).is_some_and(|r| r.diagram) {
                                self.selected_diagram_row = Some(idx);
                                self.recompute_diagram();
                            }
                        }
                        self.view_mode = ElementsViewMode::PeriodicChart;
                    }
                });
            });

        // Bottom Bar: Outotec | Exit | Fix Left 3 Columns (Fig. 2)
        egui::TopBottomPanel::bottom("elements_spreadsheet_footer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 238, 242))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Outotec")
                            .strong()
                            .size(14.0)
                            .color(egui::Color32::from_rgb(198, 52, 40)),
                    );
                    ui.separator();
                    if hsc_toolbar_button(ui, "Exit", 64.0, false).clicked() {
                        self.view_mode = ElementsViewMode::PeriodicChart;
                    }
                    if hsc_toolbar_button(ui, "Fix Left 3 Columns", 124.0, self.fix_left_3_cols)
                        .clicked()
                    {
                        self.fix_left_3_cols = !self.fix_left_3_cols;
                    }
                    ui.separator();
                    if hsc_toolbar_button(ui, "Show on Periodic Chart", 148.0, false).clicked() {
                        if self.active_cell.row > 0 {
                            let idx = self.active_cell.row - 1;
                            if let Some(r) = self.workbook.rows.get_mut(idx) {
                                r.diagram = true;
                            }
                            self.selected_diagram_row = Some(idx);
                            self.recompute_diagram();
                        }
                        self.view_mode = ElementsViewMode::PeriodicChart;
                    }
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(80, 84, 94)),
                    );
                });
            });

        // Main Excel Grid Area
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                let row_h = 20.5f32;
                let rn_w = 34.0f32;
                let [w_prop, w_unit, w_diag] = self.col_widths_base;
                let w_el = self.elem_col_width;
                let n_elems = self.workbook.element_symbols.len();
                let total_rows = 1 + self.workbook.rows.len();

                if self.fix_left_3_cols {
                    // Frozen Left 3 Columns + Horizontally Scrollable Elements, synchronized vertically!
                    egui::ScrollArea::vertical()
                        .id_salt("elements_excel_vscroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                // Left frozen pane (Row# + A:Property + B:Units + C:Diagram)
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                    // Column letters A, B, C
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                        draw_excel_hdr(ui, "", rn_w, row_h, false);
                                        draw_excel_hdr(
                                            ui,
                                            "A",
                                            w_prop,
                                            row_h,
                                            self.active_cell.col == 0,
                                        );
                                        draw_excel_hdr(
                                            ui,
                                            "B",
                                            w_unit,
                                            row_h,
                                            self.active_cell.col == 1,
                                        );
                                        draw_excel_hdr(
                                            ui,
                                            "C",
                                            w_diag,
                                            row_h,
                                            self.active_cell.col == 2,
                                        );
                                    });

                                    for r in 0..total_rows {
                                        let is_hdr = r == 0;
                                        let is_color_row = r > 0
                                            && self.workbook.rows[r - 1].property
                                                == "Color RGB-number";
                                        let _ = is_color_row;
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                                            draw_excel_hdr(
                                                ui,
                                                &(r + 1).to_string(),
                                                rn_w,
                                                row_h,
                                                self.active_cell.row == r,
                                            );
                                            let widths = [w_prop, w_unit, w_diag];
                                            for c in 0..3usize {
                                                let txt = self.cell_value(r, c);
                                                let is_sel = self.active_cell.row == r
                                                    && self.active_cell.col == c;
                                                let bg = if is_hdr {
                                                    EXCEL_SECTION_BG
                                                } else if c == 0 || c == 1 {
                                                    EXCEL_READONLY_BG
                                                } else {
                                                    egui::Color32::WHITE
                                                };
                                                let center = is_hdr || c == 2;
                                                let resp = draw_grid_cell_interactive(
                                                    ui,
                                                    &txt,
                                                    widths[c],
                                                    row_h,
                                                    bg,
                                                    egui::Color32::from_rgb(20, 22, 26),
                                                    true,
                                                    center,
                                                    is_sel,
                                                );
                                                if resp.clicked() {
                                                    self.select_spreadsheet_cell(r, c);
                                                    if !is_hdr && c == 2 {
                                                        // Toggle Diagram TRUE/FALSE directly on click
                                                        if let Some(prop) =
                                                            self.workbook.rows.get_mut(r - 1)
                                                        {
                                                            prop.diagram = !prop.diagram;
                                                            self.sync_formula_from_active_cell();
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                });

                                // Right horizontally scrollable pane (Columns D.. : H..Lr)
                                egui::ScrollArea::horizontal()
                                    .id_salt("elements_excel_hscroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let total_el_w = n_elems as f32 * w_el;
                                        let total_el_h = (total_rows + 1) as f32 * row_h;
                                        let (grid_rect, resp) = ui.allocate_exact_size(
                                            egui::vec2(total_el_w, total_el_h),
                                            egui::Sense::click(),
                                        );
                                        if resp.clicked() {
                                            if let Some(pos) = resp.interact_pointer_pos() {
                                                let col_off = ((pos.x - grid_rect.left()) / w_el)
                                                    .floor() as usize;
                                                let row_idx = ((pos.y - grid_rect.top()) / row_h)
                                                    .floor() as usize;
                                                if row_idx > 0
                                                    && row_idx <= total_rows
                                                    && col_off < n_elems
                                                {
                                                    self.select_spreadsheet_cell(
                                                        row_idx - 1,
                                                        col_off + 3,
                                                    );
                                                }
                                            }
                                        }
                                        let clip = ui.clip_rect().intersect(grid_rect);
                                        if clip.is_positive() {
                                            let first_col = ((clip.left() - grid_rect.left()) / w_el)
                                                .floor()
                                                .max(0.0) as usize;
                                            let last_col = (((clip.right() - grid_rect.left())
                                                / w_el)
                                                .ceil() as usize)
                                                .min(n_elems);
                                            let p = ui.painter().with_clip_rect(clip);

                                            // Column letter headers D..DB
                                            for c_off in first_col..last_col {
                                                let c = c_off + 3;
                                                let x = grid_rect.left() + c_off as f32 * w_el;
                                                let rct = egui::Rect::from_min_size(
                                                    egui::pos2(x, grid_rect.top()),
                                                    egui::vec2(w_el, row_h),
                                                );
                                                paint_excel_hdr_rect(
                                                    &p,
                                                    rct,
                                                    &col_letter(c),
                                                    self.active_cell.col == c,
                                                );
                                            }

                                            // Rows 0..total_rows for visible columns
                                            for r in 0..total_rows {
                                                let y = grid_rect.top() + (r + 1) as f32 * row_h;
                                                if y + row_h < clip.top() || y > clip.bottom() {
                                                    continue;
                                                }
                                                let is_hdr = r == 0;
                                                let is_color_row = r > 0
                                                    && self.workbook.rows[r - 1].property
                                                        == "Color RGB-number";
                                                let is_symbol_row = r > 0
                                                    && self.workbook.rows[r - 1].property == "Symbol";

                                                for c_off in first_col..last_col {
                                                    let c = c_off + 3;
                                                    let x = grid_rect.left() + c_off as f32 * w_el;
                                                    let rct = egui::Rect::from_min_size(
                                                        egui::pos2(x, y),
                                                        egui::vec2(w_el, row_h),
                                                    );
                                                    let txt = self.cell_value(r, c);
                                                    let is_sel = self.active_cell.row == r
                                                        && self.active_cell.col == c;
                                                    let bg = if is_hdr {
                                                        EXCEL_SECTION_BG
                                                    } else if is_color_row {
                                                        parse_vb6_rgb(&txt).unwrap_or(egui::Color32::WHITE)
                                                    } else {
                                                        egui::Color32::WHITE
                                                    };
                                                    let fg = if is_color_row {
                                                        contrasting_text_color(bg)
                                                    } else {
                                                        egui::Color32::from_rgb(20, 22, 26)
                                                    };
                                                    let bold = is_hdr || is_symbol_row;
                                                    let center = is_hdr;
                                                    paint_grid_cell(
                                                        &p, rct, &txt, bg, fg, bold, center, is_sel,
                                                    );
                                                }
                                            }
                                        }
                                    });
                            });
                        });
                } else {
                    // Unfrozen 2D scroll over all columns A..DB
                    egui::ScrollArea::both()
                        .id_salt("elements_excel_both_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let total_w = rn_w + w_prop + w_unit + w_diag + n_elems as f32 * w_el;
                            let total_h = (total_rows + 1) as f32 * row_h;
                            let (grid_rect, resp) = ui.allocate_exact_size(
                                egui::vec2(total_w, total_h),
                                egui::Sense::click(),
                            );
                            let col_x_and_w = |col: usize| -> (f32, f32) {
                                match col {
                                    0 => (grid_rect.left() + rn_w, w_prop),
                                    1 => (grid_rect.left() + rn_w + w_prop, w_unit),
                                    2 => (grid_rect.left() + rn_w + w_prop + w_unit, w_diag),
                                    c => (
                                        grid_rect.left()
                                            + rn_w
                                            + w_prop
                                            + w_unit
                                            + w_diag
                                            + (c - 3) as f32 * w_el,
                                        w_el,
                                    ),
                                }
                            };
                            if resp.clicked() {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    let row_idx =
                                        ((pos.y - grid_rect.top()) / row_h).floor() as usize;
                                    let dx = pos.x - grid_rect.left() - rn_w;
                                    let col_opt = if dx < 0.0 {
                                        None
                                    } else if dx < w_prop {
                                        Some(0)
                                    } else if dx < w_prop + w_unit {
                                        Some(1)
                                    } else if dx < w_prop + w_unit + w_diag {
                                        Some(2)
                                    } else {
                                        let c = 3 + ((dx - w_prop - w_unit - w_diag) / w_el).floor() as usize;
                                        if c < 3 + n_elems { Some(c) } else { None }
                                    };
                                    if row_idx > 0 && row_idx <= total_rows {
                                        if let Some(col) = col_opt {
                                            self.select_spreadsheet_cell(row_idx - 1, col);
                                        }
                                    }
                                }
                            }

                            let clip = ui.clip_rect().intersect(grid_rect);
                            if clip.is_positive() {
                                let p = ui.painter().with_clip_rect(clip);
                                // Row 0 column letters
                                for c in 0..(3 + n_elems) {
                                    let (x, w) = col_x_and_w(c);
                                    if x + w < clip.left() || x > clip.right() {
                                        continue;
                                    }
                                    paint_excel_hdr_rect(
                                        &p,
                                        egui::Rect::from_min_size(
                                            egui::pos2(x, grid_rect.top()),
                                            egui::vec2(w, row_h),
                                        ),
                                        &col_letter(c),
                                        self.active_cell.col == c,
                                    );
                                }
                                for r in 0..total_rows {
                                    let y = grid_rect.top() + (r + 1) as f32 * row_h;
                                    if y + row_h < clip.top() || y > clip.bottom() {
                                        continue;
                                    }
                                    paint_excel_hdr_rect(
                                        &p,
                                        egui::Rect::from_min_size(
                                            egui::pos2(grid_rect.left(), y),
                                            egui::vec2(rn_w, row_h),
                                        ),
                                        &(r + 1).to_string(),
                                        self.active_cell.row == r,
                                    );
                                    let is_hdr = r == 0;
                                    let is_color_row = r > 0
                                        && self.workbook.rows[r - 1].property == "Color RGB-number";
                                    for c in 0..(3 + n_elems) {
                                        let (x, w) = col_x_and_w(c);
                                        if x + w < clip.left() || x > clip.right() {
                                            continue;
                                        }
                                        let rct = egui::Rect::from_min_size(
                                            egui::pos2(x, y),
                                            egui::vec2(w, row_h),
                                        );
                                        let txt = self.cell_value(r, c);
                                        let is_sel =
                                            self.active_cell.row == r && self.active_cell.col == c;
                                        let bg = if is_hdr {
                                            EXCEL_SECTION_BG
                                        } else if c < 2 {
                                            EXCEL_READONLY_BG
                                        } else if is_color_row {
                                            parse_vb6_rgb(&txt).unwrap_or(egui::Color32::WHITE)
                                        } else {
                                            egui::Color32::WHITE
                                        };
                                        let fg = if is_color_row && c >= 3 {
                                            contrasting_text_color(bg)
                                        } else {
                                            egui::Color32::from_rgb(20, 22, 26)
                                        };
                                        paint_grid_cell(
                                            &p,
                                            rct,
                                            &txt,
                                            bg,
                                            fg,
                                            is_hdr || c < 2,
                                            is_hdr || c == 2,
                                            is_sel,
                                        );
                                    }
                                }
                            }
                        });
                }
            });
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        // Double-click element detail & Maindb6 lookup modal
        if let Some(el_idx) = self.double_click_modal_idx {
            if let Some(spec) = PERIODIC_ELEMENTS.get(el_idx) {
                let mut open = true;
                egui::Window::new(format!("{} ({}) — Z = {}", spec.name, spec.symbol, spec.z))
                    .open(&mut open)
                    .collapsible(false)
                    .default_width(500.0)
                    .max_height(520.0)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            let (chip, _) = ui.allocate_exact_size(
                                egui::vec2(54.0, 54.0),
                                egui::Sense::hover(),
                            );
                            if ui.is_rect_visible(chip) {
                                let p = ui.painter();
                                p.rect_filled(chip, 4.0, category_color(spec.category));
                                p.rect_stroke(
                                    chip,
                                    4.0,
                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 90, 110)),
                                );
                                p.text(
                                    chip.center() - egui::vec2(0.0, 6.0),
                                    egui::Align2::CENTER_CENTER,
                                    spec.symbol,
                                    egui::FontId::proportional(20.0),
                                    state_color(spec.state),
                                );
                                p.text(
                                    chip.center_bottom() - egui::vec2(0.0, 4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("Z={}", spec.z),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::BLACK,
                                );
                            }
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} — {}",
                                        spec.name,
                                        spec.category.label().replace('\n', " ")
                                    ))
                                    .strong()
                                    .size(15.0),
                                );
                                ui.label(format!(
                                    "Atomic Weight: {:.5} g/mol | Config: {} | Crystal: {}",
                                    spec.atomic_weight, spec.electron_config, spec.crystal
                                ));
                                if let Some(diag_row) = self.selected_diagram_row {
                                    if let Some(row) = self.workbook.rows.get(diag_row) {
                                        let v = row
                                            .values
                                            .get(el_idx)
                                            .map(String::as_str)
                                            .unwrap_or("-");
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Active Diagram — {}: {} {}",
                                                row.property, v, row.units
                                            ))
                                            .strong()
                                            .color(egui::Color32::from_rgb(24, 62, 165)),
                                        );
                                    }
                                }
                            });
                        });

                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "HSC Maindb6 Species Containing {} ({} found)",
                                spec.symbol,
                                self.modal_db_species.len()
                            ))
                            .strong(),
                        );

                        egui::ScrollArea::vertical()
                            .id_salt("elements_modal_db_scroll")
                            .max_height(240.0)
                            .show(ui, |ui| {
                                if self.modal_db_species.is_empty() {
                                    ui.label("No matching element-containing entries loaded in Maindb6.");
                                } else {
                                    egui::Grid::new("elements_modal_db_grid")
                                        .striped(true)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                            ui.strong("Formula");
                                            ui.strong("Phase");
                                            ui.strong("H°298 (kJ/mol)");
                                            ui.strong("S°298 (J/mol·K)");
                                            ui.strong("MW (g/mol)");
                                            ui.end_row();
                                            for sp in self.modal_db_species.iter().take(40) {
                                                let (h298, s298) = sp
                                                    .intervals
                                                    .first()
                                                    .map(|iv| {
                                                        (
                                                            format!("{:.2}", iv.h_or_delta_h_tr / 1000.0),
                                                            format!("{:.2}", iv.s_or_delta_s_tr),
                                                        )
                                                    })
                                                    .unwrap_or_else(|| ("-".into(), "-".into()));
                                                ui.label(&sp.formula);
                                                ui.label(phase_short(sp.phase));
                                                ui.label(h298);
                                                ui.label(s298);
                                                ui.label(format!("{:.3}", sp.molecular_weight));
                                                ui.end_row();
                                            }
                                        });
                                }
                            });

                        ui.add_space(6.0);
                        if ui.button(crate::i18n::text("common.004")).clicked() {
                            self.double_click_modal_idx = None;
                        }
                    });
                if !open {
                    self.double_click_modal_idx = None;
                }
            }
        }

        // Insert Custom Property Row modal
        if self.show_insert_row_modal {
            let mut open = true;
            egui::Window::new("Insert Custom Element Property Row")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("elements_insert_row_grid")
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Property Name:");
                            ui.text_edit_singleline(&mut self.new_row_name);
                            ui.end_row();
                            ui.label(crate::i18n::text("common.024"));
                            ui.text_edit_singleline(&mut self.new_row_units);
                            ui.end_row();
                            ui.label("Diagram Plottable:");
                            ui.checkbox(&mut self.new_row_diagram, "Enable spheres on Periodic Chart");
                            ui.end_row();
                        });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::text("common.077")).clicked() {
                            let name = if self.new_row_name.trim().is_empty() {
                                "Custom Property".to_string()
                            } else {
                                self.new_row_name.trim().to_string()
                            };
                            self.workbook.add_custom_row(
                                name.clone(),
                                self.new_row_units.trim().to_string(),
                                self.new_row_diagram,
                            );
                            let new_row_num = self.workbook.rows.len();
                            self.select_spreadsheet_cell(new_row_num, 0);
                            self.status_msg = format!("Inserted custom row '{name}'.");
                            self.show_insert_row_modal = false;
                        }
                        if ui.button(crate::i18n::text("common.000")).clicked() {
                            self.show_insert_row_modal = false;
                        }
                    });
                });
            if !open {
                self.show_insert_row_modal = false;
            }
        }

        // About modal
        if self.show_about_modal {
            let mut open = true;
            egui::Window::new("About Elements Module (Element4)")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Soryntel / HSC Elements Module — Z = 1..=103 (H..Lr)");
                    ui.label("• Periodic Chart with category coloring, state colors, and proportional 3D-shaded property spheres.");
                    ui.label("• Interactive Elements spreadsheet with Fix Left 3 Columns, Diagram toggles, VB6 RGB color swatches, custom rows, and CSV export.");
                    ui.label("• Integrated with Maindb6 species catalog lookup (double-click any element).");
                    if let Some(idx) = find_element_index_by_symbol("Os") {
                        let _ = idx;
                    }
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        self.show_about_modal = false;
                    }
                });
            if !open {
                self.show_about_modal = false;
            }
        }
    }
}

fn hsc_toolbar_button(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    active: bool,
) -> egui::Response {
    let fill = if active {
        egui::Color32::from_rgb(209, 224, 248)
    } else {
        egui::Color32::from_rgb(232, 236, 242)
    };
    let stroke = if active {
        egui::Stroke::new(1.2, egui::Color32::from_rgb(56, 102, 186))
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 175, 186))
    };
    ui.add_sized(
        [width, 24.0],
        egui::Button::new(
            egui::RichText::new(text)
                .size(11.5)
                .color(egui::Color32::from_rgb(24, 28, 36)),
        )
        .fill(fill)
        .stroke(stroke)
        .rounding(2.5),
    )
}

fn state_legend_chip(ui: &mut egui::Ui, sym: &str, sym_color: egui::Color32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 2.0, egui::Color32::WHITE);
        p.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(156, 162, 172)),
        );
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            sym,
            egui::FontId::proportional(11.5),
            sym_color,
        );
    }
    ui.label(
        egui::RichText::new(label)
            .size(11.0)
            .color(egui::Color32::from_rgb(24, 28, 34)),
    );
}

fn draw_range_tile(painter: &egui::Painter, rect: egui::Rect, label: &str, fill: egui::Color32) {
    painter.rect_filled(rect, 1.5, fill);
    painter.rect_stroke(
        rect,
        1.5,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(162, 170, 182)),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional((rect.height() * 0.25).clamp(8.0, 11.0)),
        egui::Color32::from_rgb(35, 38, 46),
    );
}

fn draw_sheet_cell(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_grid_cell(ui.painter(), rect, text, bg, fg, bold, center, false);
    }
}

fn draw_excel_hdr(ui: &mut egui::Ui, text: &str, w: f32, h: f32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_excel_hdr_rect(ui.painter(), rect, text, active);
    }
}

fn paint_excel_hdr_rect(painter: &egui::Painter, rect: egui::Rect, text: &str, active: bool) {
    let bg = if active {
        egui::Color32::from_rgb(212, 224, 242)
    } else {
        EXCEL_HEADER_BG
    };
    painter.rect_filled(rect, 0.0, bg);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(34, 38, 44),
    );
}

fn draw_grid_cell_interactive(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
    selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        paint_grid_cell(ui.painter(), rect, text, bg, fg, bold, center, selected);
    }
    resp
}

fn paint_grid_cell(
    painter: &egui::Painter,
    rect: egui::Rect,
    text: &str,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
    selected: bool,
) {
    painter.rect_filled(rect, 0.0, bg);
    let stroke = if selected {
        egui::Stroke::new(1.6, EXCEL_ACTIVE_BORDER)
    } else {
        egui::Stroke::new(1.0, EXCEL_GRID_BORDER)
    };
    painter.rect_stroke(rect, 0.0, stroke);
    let clipped = painter.with_clip_rect(rect.shrink2(egui::vec2(3.0, 1.0)));
    let _ = bold;
    let (pos, align) = if center {
        (rect.center(), egui::Align2::CENTER_CENTER)
    } else {
        (
            rect.left_center() + egui::vec2(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
        )
    };
    clipped.text(pos, align, text, egui::FontId::proportional(11.0), fg);
}

fn category_color(cat: ElementCategory) -> egui::Color32 {
    match cat {
        ElementCategory::Alkali => egui::Color32::from_rgb(236, 179, 102),
        ElementCategory::Alkaline => egui::Color32::from_rgb(244, 232, 134),
        ElementCategory::TransitionMetal => egui::Color32::from_rgb(244, 218, 224),
        ElementCategory::PostTransitionMetal => egui::Color32::from_rgb(128, 198, 240),
        ElementCategory::Metalloid => egui::Color32::from_rgb(132, 206, 132),
        ElementCategory::NonMetal => egui::Color32::from_rgb(174, 182, 238),
        ElementCategory::Halogen => egui::Color32::from_rgb(216, 232, 148),
        ElementCategory::NobleGas => egui::Color32::from_rgb(138, 222, 244),
        ElementCategory::Lanthanide => egui::Color32::from_rgb(234, 224, 218),
        ElementCategory::Actinide => egui::Color32::from_rgb(194, 196, 236),
    }
}

fn category_rgb_hex(cat: ElementCategory) -> &'static str {
    let c = category_color(cat);
    match (c.r(), c.g(), c.b()) {
        (236, 179, 102) => "#ecb366",
        (244, 232, 134) => "#f4e886",
        (244, 218, 224) => "#f4dae0",
        (128, 198, 240) => "#80c6f0",
        (132, 206, 132) => "#84ce84",
        (174, 182, 238) => "#aeb6ee",
        (216, 232, 148) => "#d8e894",
        (138, 222, 244) => "#8adef4",
        (234, 224, 218) => "#eae0da",
        _ => "#c2c4ec",
    }
}

fn state_color(state: StandardState) -> egui::Color32 {
    match state {
        StandardState::Gas => egui::Color32::from_rgb(190, 24, 24),
        StandardState::Liquid => egui::Color32::from_rgb(20, 135, 45),
        StandardState::Solid => egui::Color32::from_rgb(18, 22, 28),
    }
}

fn state_hex(state: StandardState) -> &'static str {
    match state {
        StandardState::Gas => "#be1818",
        StandardState::Liquid => "#14872d",
        StandardState::Solid => "#12161c",
    }
}

fn lighten(c: egui::Color32, delta: u8) -> egui::Color32 {
    egui::Color32::from_rgb(
        c.r().saturating_add(delta),
        c.g().saturating_add(delta),
        c.b().saturating_add(delta),
    )
}

/// VB6 stores `RGB(r,g,b)` in little-endian BGR integer `r + 256*g + 65536*b`.
fn parse_vb6_rgb(s: &str) -> Option<egui::Color32> {
    let v: u32 = s.trim().parse().ok()?;
    if v > 16_777_215 {
        return None;
    }
    let r = (v & 0xFF) as u8;
    let g = ((v >> 8) & 0xFF) as u8;
    let b = ((v >> 16) & 0xFF) as u8;
    Some(egui::Color32::from_rgb(r, g, b))
}

fn contrasting_text_color(bg: egui::Color32) -> egui::Color32 {
    let luma = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if luma < 125.0 {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_rgb(18, 20, 24)
    }
}

fn phase_short(p: Phase) -> &'static str {
    match p {
        Phase::Solid => "(s)",
        Phase::Liquid => "(l)",
        Phase::Gas => "(g)",
        Phase::AqueousIon => "(a)",
        Phase::AqueousNeutral => "(ia)",
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
