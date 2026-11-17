//! Module 14 UI — `Measure Units` (`UNITS4.UNI`) Matching HSC6 Screenshots (`Fig. 1`, `Fig. 2`, `Fig. 3`).
//!
//! Features:
//! - Top Menu Bar (`File | Edit | Help`)
//! - Top Dual GroupBoxes: `Unit Conversion` (90 quantities, 4 inline Prefix1/Unit1/Prefix2/Unit2
//!   combos, `Clear Sheet`) & `Sampler and Graphs` (`Sampler`, `Water in Air`, `Water in Air 3D`)
//! - Excel Formula Bar (`A7 | 22`, `A71 | Energy, work`, etc.)
//! - 6 Workbook Sheets: `Convert`, `Units` (270 rows in 3-row blocks), `Constants` (56 rows),
//!   `Mesh` (15 rows + sieve interpolator), `Air` (dry air + 77x10 humidity matrix), `H2O` (88 rows)
//! - Bottom Excel Sheet Tab Bar (`|< < > >| Convert / Units / Constants / Mesh / Air / H2O`) + `Outotec` footer
//! - `WaterInAir 3D` interactive golden wireframe surface modal (`Fig. 3`).

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::units::{
    AirHumidityGrid, AirHumidityPoint, ConvertRow, SiPrefix, UnitsWorkbook, STD_ATM_BAR,
};

use crate::excel_grid::{
    col_letter, EXCEL_ACTIVE_BORDER, EXCEL_GRID_BORDER, EXCEL_HEADER_BG, EXCEL_HEADER_BORDER,
    EXCEL_SECTION_BG,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsSheetTab {
    Convert,
    Units,
    Constants,
    Mesh,
    Air,
    H2O,
}

impl UnitsSheetTab {
    pub const ALL: [UnitsSheetTab; 6] = [
        Self::Convert,
        Self::Units,
        Self::Constants,
        Self::Mesh,
        Self::Air,
        Self::H2O,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Convert => "Convert",
            Self::Units => "Units",
            Self::Constants => "Constants",
            Self::Mesh => "Mesh",
            Self::Air => "Air",
            Self::H2O => "H2O",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Air3DMetric {
    VolPct,
    GNm3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertColSide {
    Data1,
    Data2,
}

pub struct UnitsModuleState {
    pub workbook: UnitsWorkbook,
    pub active_sheet: UnitsSheetTab,

    // Top `Unit Conversion` controls (matching Fig. 1 defaults: Energy, work | Kilo J -> cal(th))
    pub selected_qty: usize,
    pub prefix1: SiPrefix,
    pub unit1: usize,
    pub prefix2: SiPrefix,
    pub unit2: usize,
    pub delta_temp_mode: bool,

    // Active spreadsheet cell tracking & formula bar
    pub active_row: usize, // 1-based Excel row number (e.g. 7 for A7)
    pub active_col: usize, // 0-based Excel column (0 = A)
    pub formula_buf: String,
    pub convert_row_count: usize, // Total display rows on Convert sheet (at least 25)
    pub last_edit_side: ConvertColSide,
    /// Raw text buffer for the cell currently being edited so `.`, `-`, `0.` etc. are never stripped mid-typing.
    pub editing_cell: Option<(usize, ConvertColSide, String)>,

    // Air sheet / 2D / 3D cached data
    pub air_grid_desc: AirHumidityGrid,
    pub air_grid_3d: AirHumidityGrid,
    pub air_table_metric: Air3DMetric,
    pub air_calc_temp_buf: String,
    pub air_calc_rh_buf: String,
    pub air_calc_p_buf: String,
    pub air_calc_res: Option<AirHumidityPoint>,

    // Mesh interpolator buffers
    pub mesh_in_buf: String,
    pub mesh_mm_buf: String,

    // Filters & modals
    pub units_filter: String,
    pub constants_filter: String,
    pub show_sampler_modal: bool,
    pub sampler_val_buf: String,
    pub show_water_air_2d_modal: bool,
    pub show_water_air_3d_modal: bool,
    pub air_3d_metric: Air3DMetric,
    pub air_3d_yaw: f32,
    pub air_3d_pitch: f32,

    pub show_add_unit_modal: bool,
    pub new_unit_name: String,
    pub new_unit_sym: String,
    pub new_unit_coef: String,

    pub show_add_qty_modal: bool,
    pub new_qty_name: String,
    pub new_qty_sym: String,
    pub new_qty_si: String,
    pub new_qty_base_name: String,
    pub new_qty_base_sym: String,

    pub show_about_modal: bool,
    pub status_msg: String,
}

impl Default for UnitsModuleState {
    fn default() -> Self {
        let workbook = SoryntelApplication::default_units_workbook();
        let air_grid_desc = SoryntelApplication::air_humidity_grid_descending();
        let air_grid_3d = SoryntelApplication::air_humidity_3d_surface();
        let air_calc_res =
            SoryntelApplication::compute_air_humidity_point(20.0, 60.0, STD_ATM_BAR).ok();

        let selected_qty = workbook.find_quantity_index("Energy, work").unwrap_or(0);
        let unit1 = workbook.find_unit_index(selected_qty, "J").unwrap_or(0);
        let unit2 = workbook
            .find_unit_index(selected_qty, "cal(th)")
            .unwrap_or(0);

        let total_q = workbook.quantities.len();
        let total_u = workbook.total_unit_count();

        Self {
            workbook,
            active_sheet: UnitsSheetTab::Convert,
            selected_qty,
            prefix1: SiPrefix::Kilo,
            unit1,
            prefix2: SiPrefix::None,
            unit2,
            delta_temp_mode: false,
            active_row: 7, // A7 = "22" matching Fig. 1
            active_col: 0,
            formula_buf: "22".to_string(),
            convert_row_count: 25,
            last_edit_side: ConvertColSide::Data1,
            editing_cell: None,
            air_grid_desc,
            air_grid_3d,
            air_table_metric: Air3DMetric::VolPct,
            air_calc_temp_buf: "20.0".to_string(),
            air_calc_rh_buf: "60.0".to_string(),
            air_calc_p_buf: "1.01325".to_string(),
            air_calc_res,
            mesh_in_buf: "200".to_string(),
            mesh_mm_buf: "0.0740".to_string(),
            units_filter: String::new(),
            constants_filter: String::new(),
            show_sampler_modal: false,
            sampler_val_buf: "22".to_string(),
            show_water_air_2d_modal: false,
            show_water_air_3d_modal: false,
            air_3d_metric: Air3DMetric::VolPct,
            air_3d_yaw: -0.68,
            air_3d_pitch: 0.44,
            show_add_unit_modal: false,
            new_unit_name: String::new(),
            new_unit_sym: String::new(),
            new_unit_coef: "1.0".to_string(),
            show_add_qty_modal: false,
            new_qty_name: String::new(),
            new_qty_sym: String::new(),
            new_qty_si: String::new(),
            new_qty_base_name: String::new(),
            new_qty_base_sym: String::new(),
            show_about_modal: false,
            status_msg: format!(
                "{total_q} Quantities | {total_u} Units | Type in Data 1 or Data 2 columns to convert."
            ),
        }
    }
}

impl UnitsModuleState {
    fn ensure_convert_row(&mut self, row_1based: usize) {
        if row_1based < 2 {
            return;
        }
        let idx = row_1based - 2;
        while self.workbook.convert_rows.len() <= idx {
            self.workbook.convert_rows.push(ConvertRow {
                data1: None,
                unit1: String::new(),
                data2: None,
                unit2: String::new(),
            });
        }
        if self.convert_row_count <= row_1based + 2 {
            self.convert_row_count = row_1based + 4;
        }
    }

    fn current_unit_labels(&self) -> (String, String) {
        let Some(q) = self.workbook.quantities.get(self.selected_qty) else {
            return (String::new(), String::new());
        };
        let u1_sym = q.units.get(self.unit1).map(|u| u.symbol.as_str()).unwrap_or("");
        let u2_sym = q.units.get(self.unit2).map(|u| u.symbol.as_str()).unwrap_or("");
        (
            self.prefix1.format_unit(u1_sym),
            self.prefix2.format_unit(u2_sym),
        )
    }

    fn sync_selectors_from_row(&mut self, row_1based: usize) {
        if row_1based < 2 {
            return;
        }
        let Some(row) = self.workbook.convert_rows.get(row_1based - 2) else {
            return;
        };
        let u1_lbl = row.unit1.trim();
        let u2_lbl = row.unit2.trim();
        if u1_lbl.is_empty() || u2_lbl.is_empty() {
            return;
        }
        // Check currently selected quantity first (handles shared units like J / cal(th) deterministically)
        if let (Some((p1, u1)), Some((p2, u2))) = (
            self.workbook.resolve_labeled_unit(self.selected_qty, u1_lbl),
            self.workbook.resolve_labeled_unit(self.selected_qty, u2_lbl),
        ) {
            self.prefix1 = p1;
            self.unit1 = u1;
            self.prefix2 = p2;
            self.unit2 = u2;
            return;
        }
        for qi in 0..self.workbook.quantities.len() {
            if let (Some((p1, u1)), Some((p2, u2))) = (
                self.workbook.resolve_labeled_unit(qi, u1_lbl),
                self.workbook.resolve_labeled_unit(qi, u2_lbl),
            ) {
                self.selected_qty = qi;
                self.prefix1 = p1;
                self.unit1 = u1;
                self.prefix2 = p2;
                self.unit2 = u2;
                return;
            }
        }
    }

    fn apply_conversion_on_row(&mut self, row_1based: usize, side: ConvertColSide, raw_text: &str) {
        if row_1based < 2 {
            return;
        }
        let trimmed = raw_text.trim();
        self.ensure_convert_row(row_1based);
        let idx = row_1based - 2;
        if trimmed.is_empty() {
            self.workbook.convert_rows[idx] = ConvertRow {
                data1: None,
                unit1: String::new(),
                data2: None,
                unit2: String::new(),
            };
            return;
        }
        let Some(val) = parse_decimal_input(trimmed) else {
            if is_incomplete_numeric_prefix(trimmed) {
                self.status_msg = format!("Typing '{trimmed}'...");
            } else {
                self.status_msg = format!("Invalid numeric input '{trimmed}'.");
            }
            return;
        };
        let (lbl1, lbl2) = self.current_unit_labels();
        let res = match side {
            ConvertColSide::Data1 => SoryntelApplication::convert_unit_value(
                &self.workbook,
                self.selected_qty,
                self.unit1,
                self.prefix1,
                self.unit2,
                self.prefix2,
                val,
                self.delta_temp_mode,
            ),
            ConvertColSide::Data2 => SoryntelApplication::convert_unit_value(
                &self.workbook,
                self.selected_qty,
                self.unit2,
                self.prefix2,
                self.unit1,
                self.prefix1,
                val,
                self.delta_temp_mode,
            ),
        };
        match res {
            Ok(out) => {
                let row = &mut self.workbook.convert_rows[idx];
                row.unit1 = lbl1.clone();
                row.unit2 = lbl2.clone();
                match side {
                    ConvertColSide::Data1 => {
                        row.data1 = Some(val);
                        row.data2 = Some(out);
                        self.status_msg =
                            format!("{} {} = {} {}", fmt_excel(val), lbl1, fmt_excel(out), lbl2);
                    }
                    ConvertColSide::Data2 => {
                        row.data1 = Some(out);
                        row.data2 = Some(val);
                        self.status_msg =
                            format!("{} {} = {} {}", fmt_excel(out), lbl1, fmt_excel(val), lbl2);
                    }
                }
            }
            Err(e) => {
                self.status_msg = e;
            }
        }
    }

    fn reconvert_active_convert_row_if_numeric(&mut self) {
        if self.active_sheet != UnitsSheetTab::Convert || self.active_row < 2 {
            return;
        }
        let idx = self.active_row - 2;
        if let Some(r) = self.workbook.convert_rows.get(idx).cloned() {
            match self.last_edit_side {
                ConvertColSide::Data1 => {
                    if let Some(v) = r.data1 {
                        self.apply_conversion_on_row(
                            self.active_row,
                            ConvertColSide::Data1,
                            &fmt_excel(v),
                        );
                    }
                }
                ConvertColSide::Data2 => {
                    if let Some(v) = r.data2 {
                        self.apply_conversion_on_row(
                            self.active_row,
                            ConvertColSide::Data2,
                            &fmt_excel(v),
                        );
                    }
                }
            }
        }
    }

    fn sync_formula_from_cell(&mut self) {
        self.formula_buf = match self.active_sheet {
            UnitsSheetTab::Convert => {
                if self.active_row == 1 {
                    match self.active_col {
                        0 => "Data 1",
                        1 => "Units 1",
                        2 => "Data 2",
                        3 => "Units 2",
                        _ => "",
                    }
                    .to_string()
                } else {
                    let idx = self.active_row - 2;
                    if let Some(r) = self.workbook.convert_rows.get(idx) {
                        match self.active_col {
                            0 => r.data1.map(fmt_excel).unwrap_or_default(),
                            1 => r.unit1.clone(),
                            2 => r.data2.map(fmt_excel).unwrap_or_default(),
                            3 => r.unit2.clone(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    }
                }
            }
            UnitsSheetTab::Units => {
                let r0 = self.active_row.saturating_sub(1);
                let q_idx = r0 / 3;
                let sub = r0 % 3;
                if let Some(q) = self.workbook.quantities.get(q_idx) {
                    if self.active_col == 0 {
                        match sub {
                            0 => q.name.clone(),
                            1 => q.symbol.clone(),
                            _ => q.si_expr.clone(),
                        }
                    } else if let Some(u) = q.units.get(self.active_col - 1) {
                        match sub {
                            0 => u.name.clone(),
                            1 => u.symbol.clone(),
                            _ => fmt_unit_coef(u.coef),
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            UnitsSheetTab::Constants => {
                let idx = self.active_row.saturating_sub(2);
                if self.active_row == 1 {
                    ["Constant Name", "Symbol", "Value", "Unit", "Alt Value", "Alt Unit"]
                        .get(self.active_col)
                        .copied()
                        .unwrap_or("")
                        .to_string()
                } else if let Some(c) = self.workbook.constants.get(idx) {
                    match self.active_col {
                        0 => c.name.clone(),
                        1 => c.symbol.clone(),
                        2 => format!("{:e}", c.value),
                        3 => c.unit.clone(),
                        4 => c.alt_value.as_ref().map(|(v, _)| format!("{v}")).unwrap_or_default(),
                        5 => c.alt_value.as_ref().map(|(_, u)| u.clone()).unwrap_or_default(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
            UnitsSheetTab::Mesh => {
                if self.active_row == 1 {
                    if self.active_col == 0 { "Grain Size Units" } else { "" }.to_string()
                } else if self.active_row == 2 {
                    match self.active_col {
                        0 => "mesh",
                        1 => "mm",
                        2 => "µm",
                        _ => "",
                    }
                    .to_string()
                } else if let Some(m) = self.workbook.mesh_table.get(self.active_row - 3) {
                    match self.active_col {
                        0 => format!("{}", m.mesh),
                        1 => format!("{}", m.mm),
                        2 => format!("{:.1}", m.mm * 1000.0),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
            UnitsSheetTab::Air => String::new(),
            UnitsSheetTab::H2O => {
                if self.active_row <= 2 {
                    "Vapour Pressure of Water".to_string()
                } else if let Some(h) = self.workbook.h2o_vapour.get(self.active_row - 4) {
                    match self.active_col {
                        0 => format!("{}", h.temp_c),
                        1 => format!("{}", h.pressure_kn_m2),
                        2 => format!("{}", h.pressure_atm),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
        };
    }

    fn export_active_csv(&mut self) {
        let mut out = String::new();
        match self.active_sheet {
            UnitsSheetTab::Convert => {
                out.push_str("Data 1,Units 1,Data 2,Units 2\n");
                for r in &self.workbook.convert_rows {
                    out.push_str(&format!(
                        "{},{},{},{}\n",
                        r.data1.map(fmt_excel).unwrap_or_default(),
                        r.unit1,
                        r.data2.map(fmt_excel).unwrap_or_default(),
                        r.unit2
                    ));
                }
            }
            UnitsSheetTab::Units => {
                for q in &self.workbook.quantities {
                    out.push_str(&format!("\"{}\"", q.name));
                    for u in &q.units {
                        out.push_str(&format!(",\"{}\"", u.name));
                    }
                    out.push('\n');
                    out.push_str(&format!("\"{}\"", q.symbol));
                    for u in &q.units {
                        out.push_str(&format!(",\"{}\"", u.symbol));
                    }
                    out.push('\n');
                    out.push_str(&format!("\"{}\"", q.si_expr));
                    for u in &q.units {
                        out.push_str(&format!(",{}", u.coef));
                    }
                    out.push('\n');
                }
            }
            UnitsSheetTab::Constants => {
                out.push_str("Constant,Symbol,Value,Unit\n");
                for c in &self.workbook.constants {
                    out.push_str(&format!(
                        "\"{}\",\"{}\",{},\"{}\"\n",
                        c.name, c.symbol, c.value, c.unit
                    ));
                }
            }
            UnitsSheetTab::Mesh => {
                out.push_str("mesh,mm,um\n");
                for m in &self.workbook.mesh_table {
                    out.push_str(&format!("{},{},{:.2}\n", m.mesh, m.mm, m.mm * 1000.0));
                }
            }
            UnitsSheetTab::Air => {
                out.push_str("Temp C");
                for rh in &self.air_grid_desc.rh_pcts {
                    out.push_str(&format!(",{rh:.0}%"));
                }
                out.push('\n');
                let cols = self.air_grid_desc.rh_pcts.len();
                for (ri, &tc) in self.air_grid_desc.temps_c.iter().enumerate() {
                    out.push_str(&format!("{tc:.0}"));
                    for ci in 0..cols {
                        let v = match self.air_table_metric {
                            Air3DMetric::VolPct => self.air_grid_desc.vol_pct_grid[ri * cols + ci],
                            Air3DMetric::GNm3 => self.air_grid_desc.g_nm3_grid[ri * cols + ci],
                        };
                        out.push_str(&format!(",{v:.4}"));
                    }
                    out.push('\n');
                }
            }
            UnitsSheetTab::H2O => {
                out.push_str("Temp C,Pressure kN/m2,Pressure atm\n");
                for h in &self.workbook.h2o_vapour {
                    out.push_str(&format!(
                        "{},{},{}\n",
                        h.temp_c, h.pressure_kn_m2, h.pressure_atm
                    ));
                }
            }
        }
        let path = format!(
            "/tmp/soryntel_units_{}.csv",
            self.active_sheet.label().to_ascii_lowercase()
        );
        match crate::storage::write_atomic(&path, out) {
            Ok(()) => self.status_msg = format!("Exported {} sheet to {path}", self.active_sheet.label()),
            Err(e) => self.status_msg = format!("CSV export failed: {e}"),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        // 1. Top Menu Bar (`File | Edit | Help`, matching Fig. 1 & Fig. 2)
        egui::TopBottomPanel::top("units_menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 246))
                    .stroke(egui::Stroke::new(0.5, egui::Color32::from_rgb(208, 212, 220)))
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("Clear Convert Sheet").clicked() {
                            self.workbook.convert_rows.clear();
                            self.active_row = 2;
                            self.active_col = 0;
                            self.formula_buf.clear();
                            self.status_msg = "Cleared Convert sheet rows.".to_string();
                            ui.close_menu();
                        }
                        if ui.button("Add Custom Unit...").clicked() {
                            self.show_add_unit_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Add New Quantity...").clicked() {
                            self.show_add_qty_modal = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export Active Sheet CSV...").clicked() {
                            self.export_active_csv();
                            ui.close_menu();
                        }
                        if ui.button("Reset Default HSC6 Database").clicked() {
                            *self = Self::default();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(crate::i18n::text("common.016")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button(crate::i18n::text("common.006"), |ui| {
                        if ui.button("Swap Units 1 <-> 2").clicked() {
                            std::mem::swap(&mut self.unit1, &mut self.unit2);
                            std::mem::swap(&mut self.prefix1, &mut self.prefix2);
                            self.reconvert_active_convert_row_if_numeric();
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(&mut self.delta_temp_mode, "Differential ΔT Mode (Temperature)")
                            .changed()
                        {
                            self.reconvert_active_convert_row_if_numeric();
                        }
                        if ui.button("Copy Active Cell Value").clicked() {
                            ctx.copy_text(self.formula_buf.clone());
                            self.status_msg = format!("Copied '{}' to clipboard.", self.formula_buf);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button("Sampler (All Equivalents)").clicked() {
                            self.show_sampler_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Water in Air (2D Curves)").clicked() {
                            self.show_water_air_2d_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Water in Air 3D Graph").clicked() {
                            self.show_water_air_3d_modal = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(crate::i18n::text("common.091")).clicked() {
                            self.show_about_modal = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // 2. Bottom Outotec Brand Footer + Excel Workbook Sheet Tabs (matching Fig. 1 & Fig. 2)
        egui::TopBottomPanel::bottom("units_bottom_footer_and_tabs")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(236, 238, 242)))
            .show(ctx, |ui| {
                // Sheet tab strip (`|< < > >|  Convert / Units / Constants / Mesh / Air / H2O`)
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(228, 230, 236))
                    .stroke(egui::Stroke::new(0.8, egui::Color32::from_rgb(176, 182, 192)))
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                            if ui.small_button("|<").clicked() {
                                self.switch_sheet(UnitsSheetTab::Convert);
                            }
                            if ui.small_button("<").clicked() {
                                let idx = UnitsSheetTab::ALL
                                    .iter()
                                    .position(|&s| s == self.active_sheet)
                                    .unwrap_or(0);
                                self.switch_sheet(UnitsSheetTab::ALL[idx.saturating_sub(1)]);
                            }
                            if ui.small_button(">").clicked() {
                                let idx = UnitsSheetTab::ALL
                                    .iter()
                                    .position(|&s| s == self.active_sheet)
                                    .unwrap_or(0);
                                self.switch_sheet(
                                    UnitsSheetTab::ALL[(idx + 1).min(UnitsSheetTab::ALL.len() - 1)],
                                );
                            }
                            if ui.small_button(">|").clicked() {
                                self.switch_sheet(UnitsSheetTab::H2O);
                            }
                            ui.add_space(6.0);

                            for tab in UnitsSheetTab::ALL {
                                let is_active = self.active_sheet == tab;
                                let fill = if is_active {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(214, 218, 226)
                                };
                                let txt_color = if is_active {
                                    egui::Color32::from_rgb(15, 23, 42)
                                } else {
                                    egui::Color32::from_rgb(60, 66, 78)
                                };
                                let label = if is_active {
                                    egui::RichText::new(tab.label()).strong().size(11.5).color(txt_color)
                                } else {
                                    egui::RichText::new(tab.label()).size(11.5).color(txt_color)
                                };
                                let btn = egui::Button::new(label)
                                    .fill(fill)
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        if is_active {
                                            egui::Color32::from_rgb(110, 118, 130)
                                        } else {
                                            egui::Color32::from_rgb(172, 178, 190)
                                        },
                                    ))
                                    .rounding(egui::Rounding {
                                        nw: 0.0,
                                        ne: 0.0,
                                        sw: 4.0,
                                        se: 4.0,
                                    })
                                    .min_size(egui::vec2(58.0, 20.0));
                                if ui.add(btn).clicked() {
                                    self.switch_sheet(tab);
                                }
                            }
                        });
                    });

                // Red `Outotec` brand bar (Fig. 1)
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(232, 234, 238))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Outotec")
                                    .strong()
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(198, 42, 34)),
                            );
                            ui.separator();
                            ui.label(
                                egui::RichText::new(&self.status_msg)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(64, 70, 82)),
                            );
                        });
                    });
            });

        // 3. Top Control Ribbon (`Unit Conversion` & `Sampler and Graphs`) + Formula Bar
        egui::TopBottomPanel::top("units_controls_and_formula")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 246))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                self.render_top_groupboxes(ui);
                ui.add_space(5.0);
                self.render_formula_bar(ui);
            });

        // 4. Main Central Workbook Grid (`Convert`, `Units`, `Constants`, `Mesh`, `Air`, `H2O`)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| match self.active_sheet {
                UnitsSheetTab::Convert => self.render_convert_sheet(ui),
                UnitsSheetTab::Units => self.render_units_db_sheet(ui),
                UnitsSheetTab::Constants => self.render_constants_sheet(ui),
                UnitsSheetTab::Mesh => self.render_mesh_sheet(ui),
                UnitsSheetTab::Air => self.render_air_sheet(ui),
                UnitsSheetTab::H2O => self.render_h2o_sheet(ui),
            });

        // 5. Floating Windows / Modals (`Sampler`, `Water in Air`, `WaterInAir 3D`, etc.)
        self.render_modals(ctx);
    }

    fn switch_sheet(&mut self, tab: UnitsSheetTab) {
        self.active_sheet = tab;
        match tab {
            UnitsSheetTab::Convert => {
                self.active_row = 7;
                self.active_col = 0;
            }
            UnitsSheetTab::Units => {
                // Default A71 ("Energy, work") matching Fig. 2
                let q_e = self
                    .workbook
                    .find_quantity_index("Energy, work")
                    .unwrap_or(0);
                self.active_row = q_e * 3 + 1;
                self.active_col = 0;
            }
            UnitsSheetTab::Constants | UnitsSheetTab::Mesh => {
                self.active_row = 2;
                self.active_col = 0;
            }
            UnitsSheetTab::Air | UnitsSheetTab::H2O => {
                self.active_row = 4;
                self.active_col = 0;
            }
        }
        self.sync_formula_from_cell();
    }

    fn render_top_groupboxes(&mut self, ui: &mut egui::Ui) {
        let total_w = ui.available_width();
        let right_w = (total_w * 0.30).clamp(150.0, 235.0);
        let left_w = (total_w - right_w - 8.0).max(240.0);
        let box_h = 96.0;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

            // LEFT GROUPBOX: `Unit Conversion` (matching Fig. 1 & Fig. 2)
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, box_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(242, 244, 248))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(196, 202, 212)))
                        .rounding(2.5)
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.set_min_width(left_w - 18.0);
                            ui.set_min_height(box_h - 12.0);
                            ui.label(
                                egui::RichText::new("Unit Conversion")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(48, 54, 64)),
                            );
                            ui.add_space(2.0);

                            // Quantity ComboBox (full width of groupbox)
                            let cur_q_name = self
                                .workbook
                                .quantities
                                .get(self.selected_qty)
                                .map(|q| q.name.clone())
                                .unwrap_or_default();
                            let combo_w = (ui.available_width() - 6.0).max(160.0);
                            let mut new_qty = self.selected_qty;
                            egui::ComboBox::from_id_salt("units_qty_combo")
                                .width(combo_w)
                                .selected_text(&cur_q_name)
                                .show_ui(ui, |ui| {
                                    for (qi, q) in self.workbook.quantities.iter().enumerate() {
                                        if ui
                                            .selectable_value(&mut new_qty, qi, &q.name)
                                            .clicked()
                                        {}
                                    }
                                });
                            if new_qty != self.selected_qty {
                                self.selected_qty = new_qty;
                                self.editing_cell = None;
                                self.unit1 = 0;
                                self.unit2 = if self
                                    .workbook
                                    .quantities
                                    .get(new_qty)
                                    .map(|q| q.units.len() > 1)
                                    .unwrap_or(false)
                                {
                                    1
                                } else {
                                    0
                                };
                                self.prefix1 = SiPrefix::None;
                                self.prefix2 = SiPrefix::None;
                                self.reconvert_active_convert_row_if_numeric();
                            }

                            ui.add_space(4.0);

                            // 4 Inline ComboBoxes: Prefix 1 | Unit 1 | Prefix 2 | Unit 2
                            let inner_w = (ui.available_width() - 14.0).max(180.0);
                            let p1_w = (inner_w * 0.25).clamp(62.0, 128.0);
                            let u1_w = (inner_w * 0.24).clamp(60.0, 130.0);
                            let p2_w = (inner_w * 0.23).clamp(58.0, 118.0);
                            let u2_w = (inner_w - p1_w - u1_w - p2_w).max(58.0);

                            let mut changed = false;
                            let units_len = self
                                .workbook
                                .quantities
                                .get(self.selected_qty)
                                .map(|q| q.units.len())
                                .unwrap_or(0);
                            if self.unit1 >= units_len {
                                self.unit1 = 0;
                            }
                            if self.unit2 >= units_len {
                                self.unit2 = 0;
                            }

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
                                egui::ComboBox::from_id_salt("units_p1_combo")
                                    .width(p1_w)
                                    .selected_text(self.prefix1.combo_label())
                                    .show_ui(ui, |ui| {
                                        for p in SiPrefix::ALL {
                                            let lbl = if p == SiPrefix::None {
                                                "(none)"
                                            } else {
                                                p.combo_label()
                                            };
                                            if ui
                                                .selectable_value(&mut self.prefix1, p, lbl)
                                                .clicked()
                                            {
                                                changed = true;
                                            }
                                        }
                                    });

                                let u1_sym = self
                                    .workbook
                                    .quantities
                                    .get(self.selected_qty)
                                    .and_then(|q| q.units.get(self.unit1))
                                    .map(|u| u.symbol.clone())
                                    .unwrap_or_default();
                                egui::ComboBox::from_id_salt("units_u1_combo")
                                    .width(u1_w)
                                    .selected_text(u1_sym)
                                    .show_ui(ui, |ui| {
                                        if let Some(q) = self.workbook.quantities.get(self.selected_qty) {
                                            for (ui_idx, u) in q.units.iter().enumerate() {
                                                let lbl = format!("{}  ({})", u.symbol, u.name);
                                                if ui
                                                    .selectable_value(&mut self.unit1, ui_idx, lbl)
                                                    .clicked()
                                                {
                                                    changed = true;
                                                }
                                            }
                                        }
                                    });

                                egui::ComboBox::from_id_salt("units_p2_combo")
                                    .width(p2_w)
                                    .selected_text(self.prefix2.combo_label())
                                    .show_ui(ui, |ui| {
                                        for p in SiPrefix::ALL {
                                            let lbl = if p == SiPrefix::None {
                                                "(none)"
                                            } else {
                                                p.combo_label()
                                            };
                                            if ui
                                                .selectable_value(&mut self.prefix2, p, lbl)
                                                .clicked()
                                            {
                                                changed = true;
                                            }
                                        }
                                    });

                                let u2_sym = self
                                    .workbook
                                    .quantities
                                    .get(self.selected_qty)
                                    .and_then(|q| q.units.get(self.unit2))
                                    .map(|u| u.symbol.clone())
                                    .unwrap_or_default();
                                egui::ComboBox::from_id_salt("units_u2_combo")
                                    .width(u2_w)
                                    .selected_text(u2_sym)
                                    .show_ui(ui, |ui| {
                                        if let Some(q) = self.workbook.quantities.get(self.selected_qty) {
                                            for (ui_idx, u) in q.units.iter().enumerate() {
                                                let lbl = format!("{}  ({})", u.symbol, u.name);
                                                if ui
                                                    .selectable_value(&mut self.unit2, ui_idx, lbl)
                                                    .clicked()
                                                {
                                                    changed = true;
                                                }
                                            }
                                        }
                                    });
                            });
                            if changed {
                                self.editing_cell = None;
                                self.reconvert_active_convert_row_if_numeric();
                            }

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if hsc_btn(ui, "Clear Sheet", 96.0).clicked() {
                                    self.workbook.convert_rows.clear();
                                    self.editing_cell = None;
                                    self.active_row = 2;
                                    self.active_col = 0;
                                    self.formula_buf.clear();
                                    self.status_msg = "Cleared Convert sheet rows.".to_string();
                                }
                                if hsc_btn(ui, "Swap <->", 74.0).clicked() {
                                    self.editing_cell = None;
                                    std::mem::swap(&mut self.unit1, &mut self.unit2);
                                    std::mem::swap(&mut self.prefix1, &mut self.prefix2);
                                    self.reconvert_active_convert_row_if_numeric();
                                }
                                if let Some(q) = self.workbook.quantities.get(self.selected_qty) {
                                    ui.label(
                                        egui::RichText::new(format!("{} [{}]", q.symbol, q.si_expr))
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(95, 104, 118)),
                                    );
                                }
                            });
                        });
                },
            );

            // RIGHT GROUPBOX: `Sampler and Graphs` (matching Fig. 1 & Fig. 2)
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, box_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(242, 244, 248))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(196, 202, 212)))
                        .rounding(2.5)
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.set_min_width(right_w - 18.0);
                            ui.set_min_height(box_h - 12.0);
                            ui.label(
                                egui::RichText::new("Sampler and Graphs")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(48, 54, 64)),
                            );
                            ui.add_space(2.0);
                            let bw = (right_w - 22.0).max(110.0);
                            if hsc_btn(ui, "Sampler", bw).clicked() {
                                self.show_sampler_modal = true;
                            }
                            ui.add_space(2.0);
                            if hsc_btn(ui, "Water in Air", bw).clicked() {
                                self.show_water_air_2d_modal = true;
                            }
                            ui.add_space(2.0);
                            if hsc_btn(ui, "Water in Air 3D", bw).clicked() {
                                self.show_water_air_3d_modal = true;
                            }
                        });
                },
            );
        });
    }

    fn render_formula_bar(&mut self, ui: &mut egui::Ui) {
        let cell_ref = format!("{}{}", col_letter(self.active_col), self.active_row);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(176, 182, 192)))
                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                .show(ui, |ui| {
                    ui.set_min_width(72.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&cell_ref).size(11.5));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("v").size(9.0));
                        });
                    });
                });

            let te_w = (ui.available_width() - 4.0).max(140.0);
            let te_resp = ui.add(
                egui::TextEdit::singleline(&mut self.formula_buf)
                    .desired_width(te_w)
                    .font(egui::FontId::proportional(11.5)),
            );
            if te_resp.changed() && self.active_sheet == UnitsSheetTab::Convert && self.active_row >= 2
            {
                let side = if self.active_col == 2 {
                    ConvertColSide::Data2
                } else {
                    ConvertColSide::Data1
                };
                self.last_edit_side = side;
                let buf = self.formula_buf.clone();
                self.editing_cell = Some((self.active_row, side, buf.clone()));
                self.apply_conversion_on_row(self.active_row, side, &buf);
            }
            if te_resp.lost_focus() {
                self.editing_cell = None;
            }
        });
    }

    fn render_convert_sheet(&mut self, ui: &mut egui::Ui) {
        let avail_w = ui.available_width();
        let row_num_w = 34.0;
        let data_w = ((avail_w - row_num_w) * 0.215).clamp(88.0, 165.0);
        let unit_w = ((avail_w - row_num_w) * 0.165).clamp(66.0, 125.0);
        let e_w = (avail_w - row_num_w - 2.0 * data_w - 2.0 * unit_w).max(60.0);
        let col_widths = [data_w, unit_w, data_w, unit_w, e_w];

        egui::ScrollArea::both()
            .id_salt("units_convert_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // Excel column header row: [ ] | A | B | C | D | E
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    for (ci, &cw) in col_widths.iter().enumerate() {
                        excel_col_header(ui, &col_letter(ci), cw, self.active_col == ci);
                    }
                });

                // Row 1: Table header (`Data 1 | Units 1 | Data 2 | Units 2 | `)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 1, row_num_w, self.active_row == 1);
                    let headers = ["Data 1", "Units 1", "Data 2", "Units 2", ""];
                    for (ci, (&cw, &txt)) in col_widths.iter().zip(headers.iter()).enumerate() {
                        let is_act = self.active_row == 1 && self.active_col == ci;
                        if draw_sheet_static_cell(
                            ui,
                            txt,
                            cw,
                            EXCEL_SECTION_BG,
                            egui::Color32::from_rgb(20, 22, 26),
                            true,
                            true,
                            false,
                            is_act,
                        )
                        .clicked()
                        {
                            self.active_row = 1;
                            self.active_col = ci;
                            self.sync_formula_from_cell();
                        }
                    }
                });

                // Rows 2..convert_row_count
                let total_rows = self.convert_row_count.max(self.workbook.convert_rows.len() + 6);
                for excel_r in 2..=total_rows {
                    let row_idx = excel_r - 2;
                    let row_data = self.workbook.convert_rows.get(row_idx).cloned();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        excel_row_header(ui, excel_r, row_num_w, self.active_row == excel_r);

                        // Col A: Data 1 (editable numeric)
                        let d1_str = row_data
                            .as_ref()
                            .and_then(|r| r.data1)
                            .map(fmt_excel)
                            .unwrap_or_default();
                        let is_act_a = self.active_row == excel_r && self.active_col == 0;
                        let mut edit_a = match &self.editing_cell {
                            Some((r, ConvertColSide::Data1, raw)) if *r == excel_r => raw.clone(),
                            _ => d1_str.clone(),
                        };
                        let resp_a = draw_editable_sheet_cell(
                            ui,
                            &format!("conv_a_{excel_r}"),
                            &mut edit_a,
                            col_widths[0],
                            egui::Color32::WHITE,
                            true,
                            is_act_a,
                        );
                        if resp_a.gained_focus() || (resp_a.clicked() && self.active_row != excel_r) {
                            self.active_row = excel_r;
                            self.active_col = 0;
                            self.last_edit_side = ConvertColSide::Data1;
                            self.sync_selectors_from_row(excel_r);
                            self.editing_cell = Some((excel_r, ConvertColSide::Data1, d1_str.clone()));
                            self.formula_buf = d1_str;
                        }
                        if resp_a.changed() {
                            self.active_row = excel_r;
                            self.active_col = 0;
                            self.last_edit_side = ConvertColSide::Data1;
                            self.editing_cell = Some((excel_r, ConvertColSide::Data1, edit_a.clone()));
                            self.formula_buf = edit_a.clone();
                            self.apply_conversion_on_row(excel_r, ConvertColSide::Data1, &edit_a);
                        }
                        if resp_a.lost_focus() {
                            if matches!(self.editing_cell, Some((r, ConvertColSide::Data1, _)) if r == excel_r) {
                                self.editing_cell = None;
                            }
                        }

                        // Col B: Units 1 (centered text)
                        let u1_str = row_data.as_ref().map(|r| r.unit1.as_str()).unwrap_or("");
                        if draw_sheet_static_cell(
                            ui,
                            u1_str,
                            col_widths[1],
                            egui::Color32::WHITE,
                            egui::Color32::from_rgb(20, 22, 26),
                            false,
                            true,
                            false,
                            self.active_row == excel_r && self.active_col == 1,
                        )
                        .clicked()
                        {
                            self.active_row = excel_r;
                            self.active_col = 1;
                            self.sync_selectors_from_row(excel_r);
                            self.sync_formula_from_cell();
                        }

                        // Col C: Data 2 (editable numeric, bidirectional!)
                        let d2_str = row_data
                            .as_ref()
                            .and_then(|r| r.data2)
                            .map(fmt_excel)
                            .unwrap_or_default();
                        let is_act_c = self.active_row == excel_r && self.active_col == 2;
                        let mut edit_c = match &self.editing_cell {
                            Some((r, ConvertColSide::Data2, raw)) if *r == excel_r => raw.clone(),
                            _ => d2_str.clone(),
                        };
                        let resp_c = draw_editable_sheet_cell(
                            ui,
                            &format!("conv_c_{excel_r}"),
                            &mut edit_c,
                            col_widths[2],
                            egui::Color32::WHITE,
                            true,
                            is_act_c,
                        );
                        if resp_c.gained_focus() || (resp_c.clicked() && self.active_row != excel_r) {
                            self.active_row = excel_r;
                            self.active_col = 2;
                            self.last_edit_side = ConvertColSide::Data2;
                            self.sync_selectors_from_row(excel_r);
                            self.editing_cell = Some((excel_r, ConvertColSide::Data2, d2_str.clone()));
                            self.formula_buf = d2_str;
                        }
                        if resp_c.changed() {
                            self.active_row = excel_r;
                            self.active_col = 2;
                            self.last_edit_side = ConvertColSide::Data2;
                            self.editing_cell = Some((excel_r, ConvertColSide::Data2, edit_c.clone()));
                            self.formula_buf = edit_c.clone();
                            self.apply_conversion_on_row(excel_r, ConvertColSide::Data2, &edit_c);
                        }
                        if resp_c.lost_focus() {
                            if matches!(self.editing_cell, Some((r, ConvertColSide::Data2, _)) if r == excel_r) {
                                self.editing_cell = None;
                            }
                        }

                        // Col D: Units 2 (centered text)
                        let u2_str = row_data.as_ref().map(|r| r.unit2.as_str()).unwrap_or("");
                        if draw_sheet_static_cell(
                            ui,
                            u2_str,
                            col_widths[3],
                            egui::Color32::WHITE,
                            egui::Color32::from_rgb(20, 22, 26),
                            false,
                            true,
                            false,
                            self.active_row == excel_r && self.active_col == 3,
                        )
                        .clicked()
                        {
                            self.active_row = excel_r;
                            self.active_col = 3;
                            self.sync_formula_from_cell();
                        }

                        // Col E: blank tail column
                        if draw_sheet_static_cell(
                            ui,
                            "",
                            col_widths[4],
                            egui::Color32::WHITE,
                            egui::Color32::from_rgb(20, 22, 26),
                            false,
                            false,
                            false,
                            self.active_row == excel_r && self.active_col == 4,
                        )
                        .clicked()
                        {
                            self.active_row = excel_r;
                            self.active_col = 4;
                            self.sync_formula_from_cell();
                        }
                    });
                }
            });
    }

    fn render_units_db_sheet(&mut self, ui: &mut egui::Ui) {
        // Quick filter + add buttons strip above Excel grid (Fig. 2)
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(246, 247, 250))
            .stroke(egui::Stroke::new(0.5, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Filter Quantity:").size(11.5));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.units_filter)
                            .desired_width(180.0)
                            .hint_text("e.g. Energy, Pressure, Mass..."),
                    );
                    if !self.units_filter.is_empty() && ui.small_button("×").clicked() {
                        self.units_filter.clear();
                    }
                    ui.separator();
                    if ui.button("+ Add Unit").clicked() {
                        self.show_add_unit_modal = true;
                    }
                    if ui.button("+ Add Quantity").clicked() {
                        self.show_add_qty_modal = true;
                    }
                });
            });

        let filter = self.units_filter.trim().to_ascii_lowercase();
        let filtered_q: Vec<usize> = self
            .workbook
            .quantities
            .iter()
            .enumerate()
            .filter(|(_, q)| filter.is_empty() || q.name.to_ascii_lowercase().contains(&filter))
            .map(|(idx, _)| idx)
            .collect();

        let max_unit_cols = filtered_q
            .iter()
            .filter_map(|&qi| self.workbook.quantities.get(qi))
            .map(|q| q.units.len())
            .max()
            .unwrap_or(4)
            .clamp(4, 32);

        let row_num_w = 36.0;
        let col_a_w = 195.0;
        let col_u_w = 148.0;

        egui::ScrollArea::both()
            .id_salt("units_db_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // Column letters header A..
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    excel_col_header(ui, "A", col_a_w, self.active_col == 0);
                    for ci in 1..=max_unit_cols {
                        excel_col_header(ui, &col_letter(ci), col_u_w, self.active_col == ci);
                    }
                });

                for q_idx in filtered_q {
                    for sub in 0..3 {
                        let excel_r = q_idx * 3 + sub + 1;
                        let bg = if q_idx % 2 == 0 {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(248, 250, 253)
                        };
                        let q = self.workbook.quantities[q_idx].clone();
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                            excel_row_header(ui, excel_r, row_num_w, self.active_row == excel_r);

                            let col_a_txt = match sub {
                                0 => q.name.as_str(),
                                1 => q.symbol.as_str(),
                                _ => q.si_expr.as_str(),
                            };
                            if draw_sheet_static_cell(
                                ui,
                                col_a_txt,
                                col_a_w,
                                bg,
                                egui::Color32::from_rgb(20, 22, 26),
                                sub == 0,
                                false,
                                false,
                                self.active_row == excel_r && self.active_col == 0,
                            )
                            .clicked()
                            {
                                self.active_row = excel_r;
                                self.active_col = 0;
                                self.selected_qty = q_idx;
                                self.sync_formula_from_cell();
                            }

                            for ci in 0..max_unit_cols {
                                if let Some(u) = q.units.get(ci) {
                                    let is_act =
                                        self.active_row == excel_r && self.active_col == ci + 1;
                                    let cell_txt = match sub {
                                        0 => u.name.clone(),
                                        1 => u.symbol.clone(),
                                        _ => fmt_unit_coef(u.coef),
                                    };
                                    if sub == 2 {
                                        // Coefficient editable!
                                        let mut edit_coef = cell_txt.clone();
                                        let resp = draw_editable_sheet_cell(
                                            ui,
                                            &format!("udb_c_{q_idx}_{ci}"),
                                            &mut edit_coef,
                                            col_u_w,
                                            bg,
                                            true,
                                            is_act,
                                        );
                                        if resp.clicked() || resp.gained_focus() {
                                            self.active_row = excel_r;
                                            self.active_col = ci + 1;
                                            self.formula_buf = cell_txt;
                                        }
                                        if resp.changed() {
                                            if let Ok(v) = edit_coef.trim().parse::<f64>() {
                                                if v > 0.0 && v.is_finite() {
                                                    self.workbook.quantities[q_idx].units[ci]
                                                        .coef = v;
                                                    self.formula_buf = edit_coef;
                                                }
                                            }
                                        }
                                    } else if draw_sheet_static_cell(
                                        ui,
                                        &cell_txt,
                                        col_u_w,
                                        bg,
                                        egui::Color32::from_rgb(20, 22, 26),
                                        false,
                                        false,
                                        false,
                                        is_act,
                                    )
                                    .clicked()
                                    {
                                        self.active_row = excel_r;
                                        self.active_col = ci + 1;
                                        self.sync_formula_from_cell();
                                    }
                                } else {
                                    draw_sheet_static_cell(
                                        ui,
                                        "",
                                        col_u_w,
                                        bg,
                                        egui::Color32::BLACK,
                                        false,
                                        false,
                                        false,
                                        self.active_row == excel_r && self.active_col == ci + 1,
                                    );
                                }
                            }
                        });
                    }
                }
            });
    }

    fn render_constants_sheet(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(246, 247, 250))
            .stroke(egui::Stroke::new(0.5, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Search Constant:").size(11.5));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.constants_filter)
                            .desired_width(200.0)
                            .hint_text("Gas constant, Faraday, Planck..."),
                    );
                });
            });

        let filter = self.constants_filter.trim().to_ascii_lowercase();
        let row_num_w = 34.0;
        let ws = [270.0, 90.0, 150.0, 120.0, 110.0, 90.0];

        egui::ScrollArea::both()
            .id_salt("units_const_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    for (ci, &cw) in ws.iter().enumerate() {
                        excel_col_header(ui, &col_letter(ci), cw, self.active_col == ci);
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 1, row_num_w, self.active_row == 1);
                    for (ci, (&cw, txt)) in ws
                        .iter()
                        .zip(["Constant", "Symbol", "Value", "Units", "Alt Value", "Alt Unit"])
                        .enumerate()
                    {
                        draw_sheet_static_cell(
                            ui,
                            txt,
                            cw,
                            EXCEL_SECTION_BG,
                            egui::Color32::BLACK,
                            true,
                            ci != 0,
                            false,
                            self.active_row == 1 && self.active_col == ci,
                        );
                    }
                });

                for (i, c) in self
                    .workbook
                    .constants
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        filter.is_empty()
                            || c.name.to_ascii_lowercase().contains(&filter)
                            || c.symbol.to_ascii_lowercase().contains(&filter)
                    })
                {
                    let excel_r = i + 2;
                    let bg = if i % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(248, 250, 253)
                    };
                    let alt_v = c
                        .alt_value
                        .as_ref()
                        .map(|(v, _)| format!("{v}"))
                        .unwrap_or_default();
                    let alt_u = c
                        .alt_value
                        .as_ref()
                        .map(|(_, u)| u.as_str())
                        .unwrap_or("");
                    let val_str = fmt_unit_coef(c.value);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        excel_row_header(ui, excel_r, row_num_w, self.active_row == excel_r);
                        let cells = [
                            (c.name.as_str(), false, false),
                            (c.symbol.as_str(), true, false),
                            (val_str.as_str(), false, true),
                            (c.unit.as_str(), true, false),
                            (alt_v.as_str(), false, true),
                            (alt_u, true, false),
                        ];
                        for (ci, ((txt, center, right), &cw)) in cells.iter().zip(ws.iter()).enumerate() {
                            if draw_sheet_static_cell(
                                ui,
                                txt,
                                cw,
                                bg,
                                egui::Color32::from_rgb(20, 22, 26),
                                false,
                                *center,
                                *right,
                                self.active_row == excel_r && self.active_col == ci,
                            )
                            .clicked()
                            {
                                self.active_row = excel_r;
                                self.active_col = ci;
                                self.formula_buf = (*txt).to_string();
                                self.status_msg = format!("{} = {} {}", c.name, val_str, c.unit);
                            }
                        }
                    });
                }
            });
    }

    fn render_mesh_sheet(&mut self, ui: &mut egui::Ui) {
        // Inline Mesh <-> Opening interpolator strip at top of Mesh sheet
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(246, 247, 250))
            .stroke(egui::Stroke::new(0.5, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Sieve Interpolator (log-log):").strong().size(11.5));
                    ui.label("Mesh:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.mesh_in_buf).desired_width(68.0))
                        .changed()
                    {
                        if let Ok(m) = self.mesh_in_buf.trim().parse::<f64>() {
                            if let Some(mm) = self.workbook.interpolate_mesh_to_mm(m) {
                                self.mesh_mm_buf = format!("{mm:.4}");
                                self.status_msg =
                                    format!("{m} mesh ≈ {mm:.4} mm ({:.1} µm)", mm * 1000.0);
                            }
                        }
                    }
                    ui.label("<->  Opening [mm]:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.mesh_mm_buf).desired_width(76.0))
                        .changed()
                    {
                        if let Ok(mm) = self.mesh_mm_buf.trim().parse::<f64>() {
                            if let Some(m) = self.workbook.interpolate_mm_to_mesh(mm) {
                                self.mesh_in_buf = format!("{m:.1}");
                                self.status_msg = format!("{mm:.4} mm ≈ {m:.1} mesh");
                            }
                        }
                    }
                });
            });

        let row_num_w = 34.0;
        let ws = [130.0, 130.0, 130.0];
        egui::ScrollArea::both()
            .id_salt("units_mesh_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    for (ci, &cw) in ws.iter().enumerate() {
                        excel_col_header(ui, &col_letter(ci), cw, self.active_col == ci);
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 1, row_num_w, self.active_row == 1);
                    draw_sheet_static_cell(
                        ui,
                        "Grain Size Units",
                        ws[0],
                        EXCEL_SECTION_BG,
                        egui::Color32::BLACK,
                        true,
                        false,
                        false,
                        false,
                    );
                    draw_sheet_static_cell(
                        ui,
                        "",
                        ws[1],
                        EXCEL_SECTION_BG,
                        egui::Color32::BLACK,
                        false,
                        false,
                        false,
                        false,
                    );
                    draw_sheet_static_cell(
                        ui,
                        "",
                        ws[2],
                        EXCEL_SECTION_BG,
                        egui::Color32::BLACK,
                        false,
                        false,
                        false,
                        false,
                    );
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 2, row_num_w, self.active_row == 2);
                    for (ci, (&cw, lbl)) in ws.iter().zip(["mesh", "mm", "µm"]).enumerate() {
                        draw_sheet_static_cell(
                            ui,
                            lbl,
                            cw,
                            EXCEL_SECTION_BG,
                            egui::Color32::BLACK,
                            true,
                            true,
                            false,
                            self.active_row == 2 && self.active_col == ci,
                        );
                    }
                });
                for (i, m) in self.workbook.mesh_table.iter().enumerate() {
                    let excel_r = i + 3;
                    let bg = if i % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(248, 250, 253)
                    };
                    let um = format!("{:.1}", m.mm * 1000.0);
                    let ms = format!("{}", m.mesh);
                    let mms = format!("{}", m.mm);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        excel_row_header(ui, excel_r, row_num_w, self.active_row == excel_r);
                        for (ci, (txt, &cw)) in [ms.as_str(), mms.as_str(), um.as_str()]
                            .iter()
                            .zip(ws.iter())
                            .enumerate()
                        {
                            if draw_sheet_static_cell(
                                ui,
                                txt,
                                cw,
                                bg,
                                egui::Color32::BLACK,
                                false,
                                false,
                                true,
                                self.active_row == excel_r && self.active_col == ci,
                            )
                            .clicked()
                            {
                                self.active_row = excel_r;
                                self.active_col = ci;
                                self.formula_buf = (*txt).to_string();
                            }
                        }
                    });
                }
            });
    }

    fn render_air_sheet(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(246, 247, 250))
            .stroke(egui::Stroke::new(0.5, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Air Humidity Calc:").strong().size(11.5));
                    ui.label("T (°C):");
                    let mut changed = ui
                        .add(egui::TextEdit::singleline(&mut self.air_calc_temp_buf).desired_width(48.0))
                        .changed();
                    ui.label("RH (%):");
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut self.air_calc_rh_buf).desired_width(44.0))
                        .changed();
                    ui.label("P (bar):");
                    changed |= ui
                        .add(egui::TextEdit::singleline(&mut self.air_calc_p_buf).desired_width(54.0))
                        .changed();
                    if changed {
                        if let (Ok(t), Ok(rh), Ok(p)) = (
                            self.air_calc_temp_buf.trim().parse::<f64>(),
                            self.air_calc_rh_buf.trim().parse::<f64>(),
                            self.air_calc_p_buf.trim().parse::<f64>(),
                        ) {
                            self.air_calc_res =
                                SoryntelApplication::compute_air_humidity_point(t, rh, p).ok();
                        }
                    }
                    if let Some(res) = self.air_calc_res {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "P_H2O = {:.5} bar | H2O = {:.3} vol-% | {:.3} g/Nm³",
                                res.p_h2o_bar, res.vol_pct, res.g_per_nm3
                            ))
                            .strong()
                            .size(11.5)
                            .color(egui::Color32::from_rgb(20, 72, 130)),
                        );
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.air_table_metric, Air3DMetric::VolPct, "H2O vol-%");
                    ui.selectable_value(&mut self.air_table_metric, Air3DMetric::GNm3, "H2O g/Nm³");
                });
            });

        egui::ScrollArea::both()
            .id_salt("units_air_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                let row_num_w = 34.0;
                let rh_cols = self.air_grid_desc.rh_pcts.len();
                let rh_w = 64.0;

                // Header A..N
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    for (ci, cw) in [72.0, 68.0, 68.0, 20.0, 58.0]
                        .into_iter()
                        .chain(std::iter::repeat(rh_w).take(rh_cols))
                        .enumerate()
                    {
                        excel_col_header(ui, &col_letter(ci), cw, false);
                    }
                });

                // Title & column headers
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 1, row_num_w, false);
                    draw_sheet_static_cell(ui, "Composition", 72.0, EXCEL_SECTION_BG, egui::Color32::BLACK, true, false, false, false);
                    draw_sheet_static_cell(ui, "Vol-%", 68.0, EXCEL_SECTION_BG, egui::Color32::BLACK, true, true, false, false);
                    draw_sheet_static_cell(ui, "Wt-%", 68.0, EXCEL_SECTION_BG, egui::Color32::BLACK, true, true, false, false);
                    draw_sheet_static_cell(ui, "", 20.0, EXCEL_HEADER_BG, egui::Color32::BLACK, false, false, false, false);
                    draw_sheet_static_cell(ui, "T (°C)", 58.0, EXCEL_SECTION_BG, egui::Color32::BLACK, true, true, false, false);
                    for &rh in &self.air_grid_desc.rh_pcts {
                        draw_sheet_static_cell(
                            ui,
                            &format!("{rh:.0} %"),
                            rh_w,
                            EXCEL_SECTION_BG,
                            egui::Color32::BLACK,
                            true,
                            true,
                            false,
                            false,
                        );
                    }
                });

                let max_r = self
                    .air_grid_desc
                    .temps_c
                    .len()
                    .max(self.workbook.dry_air.len());
                for i in 0..max_r {
                    let excel_r = i + 2;
                    let bg = if i % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(248, 250, 253)
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        excel_row_header(ui, excel_r, row_num_w, false);
                        if let Some(d) = self.workbook.dry_air.get(i) {
                            draw_sheet_static_cell(ui, &d.species, 72.0, bg, egui::Color32::BLACK, true, false, false, false);
                            draw_sheet_static_cell(ui, &format!("{}", d.vol_pct), 68.0, bg, egui::Color32::BLACK, false, false, true, false);
                            draw_sheet_static_cell(ui, &format!("{}", d.wt_pct), 68.0, bg, egui::Color32::BLACK, false, false, true, false);
                        } else {
                            draw_sheet_static_cell(ui, "", 72.0, bg, egui::Color32::BLACK, false, false, false, false);
                            draw_sheet_static_cell(ui, "", 68.0, bg, egui::Color32::BLACK, false, false, false, false);
                            draw_sheet_static_cell(ui, "", 68.0, bg, egui::Color32::BLACK, false, false, false, false);
                        }
                        draw_sheet_static_cell(ui, "", 20.0, bg, egui::Color32::BLACK, false, false, false, false);

                        if let Some(&tc) = self.air_grid_desc.temps_c.get(i) {
                            draw_sheet_static_cell(ui, &format!("{tc:.0}"), 58.0, EXCEL_HEADER_BG, egui::Color32::BLACK, true, false, true, false);
                            for ci in 0..rh_cols {
                                let v = match self.air_table_metric {
                                    Air3DMetric::VolPct => self.air_grid_desc.vol_pct_grid[i * rh_cols + ci],
                                    Air3DMetric::GNm3 => self.air_grid_desc.g_nm3_grid[i * rh_cols + ci],
                                };
                                draw_sheet_static_cell(
                                    ui,
                                    &format!("{v:.3}"),
                                    rh_w,
                                    bg,
                                    egui::Color32::BLACK,
                                    false,
                                    false,
                                    true,
                                    false,
                                );
                            }
                        }
                    });
                }
            });
    }

    fn render_h2o_sheet(&mut self, ui: &mut egui::Ui) {
        let row_num_w = 34.0;
        let ws = [140.0, 150.0, 140.0];
        egui::ScrollArea::both()
            .id_salt("units_h2o_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_col_header(ui, "", row_num_w, false);
                    for (ci, &cw) in ws.iter().enumerate() {
                        excel_col_header(ui, &col_letter(ci), cw, self.active_col == ci);
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 1, row_num_w, false);
                    draw_sheet_static_cell(ui, "Vapour Pressure of Water", ws[0], EXCEL_SECTION_BG, egui::Color32::BLACK, true, false, false, false);
                    draw_sheet_static_cell(ui, "", ws[1], EXCEL_SECTION_BG, egui::Color32::BLACK, false, false, false, false);
                    draw_sheet_static_cell(ui, "", ws[2], EXCEL_SECTION_BG, egui::Color32::BLACK, false, false, false, false);
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    excel_row_header(ui, 2, row_num_w, false);
                    for (&cw, txt) in ws.iter().zip(["Temp. (°C)", "Pressure (kN/m²)", "Pressure (atm)"]) {
                        draw_sheet_static_cell(ui, txt, cw, EXCEL_SECTION_BG, egui::Color32::BLACK, true, true, false, false);
                    }
                });
                for (i, h) in self.workbook.h2o_vapour.iter().enumerate() {
                    let excel_r = i + 3;
                    let bg = if i % 2 == 0 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(248, 250, 253)
                    };
                    let ts = format!("{}", h.temp_c);
                    let kn = fmt_excel(h.pressure_kn_m2);
                    let at = fmt_excel(h.pressure_atm);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        excel_row_header(ui, excel_r, row_num_w, self.active_row == excel_r);
                        for (ci, (txt, &cw)) in [ts.as_str(), kn.as_str(), at.as_str()]
                            .iter()
                            .zip(ws.iter())
                            .enumerate()
                        {
                            if draw_sheet_static_cell(
                                ui,
                                txt,
                                cw,
                                bg,
                                egui::Color32::BLACK,
                                false,
                                false,
                                true,
                                self.active_row == excel_r && self.active_col == ci,
                            )
                            .clicked()
                            {
                                self.active_row = excel_r;
                                self.active_col = ci;
                                self.formula_buf = (*txt).to_string();
                            }
                        }
                    });
                }
            });
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        // 1. Sampler Modal (all equivalent units in selected quantity)
        if self.show_sampler_modal {
            let mut open = true;
            egui::Window::new("Unit Sampler — All Equivalents")
                .open(&mut open)
                .default_width(540.0)
                .default_height(420.0)
                .show(ctx, |ui| {
                    let qname = self
                        .workbook
                        .quantities
                        .get(self.selected_qty)
                        .map(|q| q.name.clone())
                        .unwrap_or_default();
                    let u1_lbl = self.current_unit_labels().0;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{qname}  |  Input ({u1_lbl}):")).strong());
                        ui.add(egui::TextEdit::singleline(&mut self.sampler_val_buf).desired_width(100.0));
                    });
                    ui.separator();
                    let val = parse_decimal_input(&self.sampler_val_buf).unwrap_or(1.0);
                    let rows = SoryntelApplication::convert_unit_all_equivalents(
                        &self.workbook,
                        self.selected_qty,
                        self.unit1,
                        self.prefix1,
                        val,
                        self.delta_temp_mode,
                    );
                    egui::ScrollArea::vertical().max_height(310.0).show(ui, |ui| {
                        egui::Grid::new("sampler_eq_grid").striped(true).show(ui, |ui| {
                            ui.strong("Unit Name");
                            ui.strong("Symbol");
                            ui.strong("Equivalent Value");
                            ui.end_row();
                            for (name, sym, res) in rows {
                                ui.label(name);
                                ui.label(sym);
                                match res {
                                    Ok(v) => ui.label(fmt_excel(v)),
                                    Err(e) => ui.colored_label(egui::Color32::RED, e),
                                };
                                ui.end_row();
                            }
                        });
                    });
                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        self.show_sampler_modal = false;
                    }
                });
            if !open {
                self.show_sampler_modal = false;
            }
        }

        // 2. Water in Air 2D Graph
        if self.show_water_air_2d_modal {
            let mut open = true;
            egui::Window::new("Water in Air — Psychrometric Curves (2D)")
                .open(&mut open)
                .default_width(640.0)
                .default_height(440.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Y Axis:");
                        ui.selectable_value(&mut self.air_3d_metric, Air3DMetric::VolPct, "H2O Vol-%");
                        ui.selectable_value(&mut self.air_3d_metric, Air3DMetric::GNm3, "H2O g/Nm³");
                    });
                    let y_lbl = match self.air_3d_metric {
                        Air3DMetric::VolPct => "H2O (vol-%)",
                        Air3DMetric::GNm3 => "H2O (g/Nm³)",
                    };
                    Plot::new("water_in_air_2d_plot")
                        .x_axis_label(crate::i18n::text("common.090"))
                        .y_axis_label(y_lbl)
                        .legend(Legend::default())
                        .show(ui, |plot_ui| {
                            for (rh_i, &rh) in [100.0, 80.0, 60.0, 40.0, 20.0].iter().enumerate() {
                                let pts: PlotPoints = (-40..=100)
                                    .step_by(2)
                                    .filter_map(|tc| {
                                        SoryntelApplication::compute_air_humidity_point(
                                            tc as f64,
                                            rh,
                                            STD_ATM_BAR,
                                        )
                                        .ok()
                                        .map(|pt| {
                                            let y = match self.air_3d_metric {
                                                Air3DMetric::VolPct => pt.vol_pct,
                                                Air3DMetric::GNm3 => pt.g_per_nm3,
                                            };
                                            [tc as f64, y]
                                        })
                                    })
                                    .collect();
                                let col = [
                                    egui::Color32::from_rgb(29, 78, 216),
                                    egui::Color32::from_rgb(5, 150, 105),
                                    egui::Color32::from_rgb(217, 119, 6),
                                    egui::Color32::from_rgb(190, 24, 93),
                                    egui::Color32::from_rgb(107, 114, 128),
                                ][rh_i];
                                plot_ui.line(Line::new(pts).name(format!("RH = {rh:.0}%")).color(col).width(2.0));
                            }
                        });
                });
            if !open {
                self.show_water_air_2d_modal = false;
            }
        }

        // 3. WaterInAir 3D Surface Window matching HSC6 Fig. 3!
        if self.show_water_air_3d_modal {
            let mut open = true;
            egui::Window::new("WaterInAir 3D")
                .open(&mut open)
                .default_width(660.0)
                .default_height(520.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.air_3d_metric, Air3DMetric::VolPct, "H2O Vol-%");
                        ui.selectable_value(&mut self.air_3d_metric, Air3DMetric::GNm3, "H2O g/Nm3");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(crate::i18n::text("common.067")).clicked() {
                                self.air_3d_yaw = -0.68;
                                self.air_3d_pitch = 0.44;
                            }
                            ui.label(
                                egui::RichText::new("Drag to orbit")
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(100, 110, 125)),
                            );
                        });
                    });
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Water Content of Air")
                                .size(15.0)
                                .color(egui::Color32::from_rgb(20, 24, 30)),
                        );
                    });

                    let avail = ui.available_size();
                    let plot_size = egui::vec2(avail.x.max(260.0), (avail.y - 8.0).max(240.0));
                    let (rect, resp) = ui.allocate_exact_size(plot_size, egui::Sense::drag());
                    if resp.dragged() {
                        let d = resp.drag_delta();
                        self.air_3d_yaw += d.x * 0.008;
                        self.air_3d_pitch = (self.air_3d_pitch + d.y * 0.007).clamp(0.15, 1.35);
                    }
                    render_water_in_air_3d_surface(
                        ui.painter().with_clip_rect(rect),
                        rect,
                        &self.air_grid_3d,
                        self.air_3d_metric,
                        self.air_3d_yaw,
                        self.air_3d_pitch,
                    );
                });
            if !open {
                self.show_water_air_3d_modal = false;
            }
        }

        // 4. Add Custom Unit Modal
        if self.show_add_unit_modal {
            let mut open = true;
            egui::Window::new("Add Custom Unit to Selected Quantity")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    let qname = self
                        .workbook
                        .quantities
                        .get(self.selected_qty)
                        .map(|q| q.name.clone())
                        .unwrap_or_default();
                    ui.label(egui::RichText::new(format!("Target Quantity: {qname}")).strong());
                    ui.horizontal(|ui| {
                        ui.label("Unit Name:");
                        ui.text_edit_singleline(&mut self.new_unit_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("common.092"));
                        ui.text_edit_singleline(&mut self.new_unit_sym);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Units per 1 Base Unit:");
                        ui.text_edit_singleline(&mut self.new_unit_coef);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Add Unit").clicked() {
                            if let Ok(c) = self.new_unit_coef.trim().parse::<f64>() {
                                match self.workbook.add_unit_to_quantity(
                                    self.selected_qty,
                                    self.new_unit_name.clone(),
                                    self.new_unit_sym.clone(),
                                    c,
                                ) {
                                    Ok(()) => {
                                        self.status_msg = format!(
                                            "Added unit '{}' ({}) to '{}'.",
                                            self.new_unit_name, self.new_unit_sym, qname
                                        );
                                        self.show_add_unit_modal = false;
                                    }
                                    Err(e) => self.status_msg = e,
                                }
                            } else {
                                self.status_msg = "Coefficient must be numeric > 0.".to_string();
                            }
                        }
                        if ui.button(crate::i18n::text("common.000")).clicked() {
                            self.show_add_unit_modal = false;
                        }
                    });
                });
            if !open {
                self.show_add_unit_modal = false;
            }
        }

        // 5. Add New Quantity Modal
        if self.show_add_qty_modal {
            let mut open = true;
            egui::Window::new("Add New Physical Quantity")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Quantity Name:");
                        ui.text_edit_singleline(&mut self.new_qty_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("common.092"));
                        ui.text_edit_singleline(&mut self.new_qty_sym);
                    });
                    ui.horizontal(|ui| {
                        ui.label("SI Base Expression:");
                        ui.text_edit_singleline(&mut self.new_qty_si);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Base Unit Name:");
                        ui.text_edit_singleline(&mut self.new_qty_base_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Base Unit Symbol:");
                        ui.text_edit_singleline(&mut self.new_qty_base_sym);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Create Quantity").clicked() {
                            match self.workbook.add_quantity(
                                self.new_qty_name.clone(),
                                self.new_qty_sym.clone(),
                                self.new_qty_si.clone(),
                                self.new_qty_base_name.clone(),
                                self.new_qty_base_sym.clone(),
                            ) {
                                Ok(idx) => {
                                    self.selected_qty = idx;
                                    self.unit1 = 0;
                                    self.unit2 = 0;
                                    self.show_add_qty_modal = false;
                                    self.status_msg =
                                        format!("Created new quantity '{}'.", self.new_qty_name);
                                }
                                Err(e) => self.status_msg = e,
                            }
                        }
                        if ui.button(crate::i18n::text("common.000")).clicked() {
                            self.show_add_qty_modal = false;
                        }
                    });
                });
            if !open {
                self.show_add_qty_modal = false;
            }
        }

        // 6. About Modal
        if self.show_about_modal {
            let mut open = true;
            egui::Window::new(crate::i18n::text("common.091"))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Soryntel Chemistry — Module 14: Measure Units (`UNITS4.UNI`)");
                    ui.label(format!(
                        "• {} physical quantities, {} units + 21 SI metric prefixes (Y..y).",
                        self.workbook.quantities.len(),
                        self.workbook.total_unit_count()
                    ));
                    ui.label("• 6-sheet workbook: Convert, Units, Constants, Mesh, Air, H2O.");
                    ui.label("• Includes WaterInAir 2D & 3D psychometric surface plots.");
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

/// Draw HSC6 Fig. 3 `WaterInAir 3D` golden-orange wireframe surface over 3D grid cube.
fn render_water_in_air_3d_surface(
    painter: egui::Painter,
    rect: egui::Rect,
    grid: &AirHumidityGrid,
    metric: Air3DMetric,
    yaw: f32,
    pitch: f32,
) {
    painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
    let nt = grid.temps_c.len();
    let nr = grid.rh_pcts.len();
    if nt < 2 || nr < 2 {
        return;
    }

    // Normalized 3D point in [-1, 1] cube: x = RH (10..100), y = T (-50..102), z = log10(val) scaled
    let project = |nx: f32, ny: f32, nz: f32| -> (egui::Pos2, f32) {
        let cx = nx - 0.5;
        let cy = ny - 0.5;
        let cz = nz - 0.45;
        let (sy, cyw) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let rx = cx * cyw - cy * sy;
        let ry = cx * sy + cy * cyw;
        let rz = cz;
        let py = ry * sp - rz * cp;
        let pz = ry * cp + rz * sp;
        let scale = rect.width().min(rect.height()) * 0.58;
        let screen = egui::pos2(
            rect.center().x + rx * scale,
            rect.center().y + py * scale * 0.92,
        );
        (screen, pz)
    };

    // Background walls matching Fig. 3 subtle light grey cube planes
    let wall_bg = egui::Color32::from_rgb(228, 232, 238);
    let wall_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 62, 72));
    let floor_pts = vec![
        project(0.0, 0.0, 0.0).0,
        project(1.0, 0.0, 0.0).0,
        project(1.0, 1.0, 0.0).0,
        project(0.0, 1.0, 0.0).0,
    ];
    let back_t = vec![
        project(0.0, 1.0, 0.0).0,
        project(1.0, 1.0, 0.0).0,
        project(1.0, 1.0, 1.0).0,
        project(0.0, 1.0, 1.0).0,
    ];
    let back_r = vec![
        project(0.0, 0.0, 0.0).0,
        project(0.0, 1.0, 0.0).0,
        project(0.0, 1.0, 1.0).0,
        project(0.0, 0.0, 1.0).0,
    ];
    for poly in [floor_pts, back_t, back_r] {
        painter.add(egui::Shape::convex_polygon(poly, wall_bg, wall_stroke));
    }

    // Wall gridlines
    let wgrid = egui::Stroke::new(0.7, egui::Color32::from_rgb(138, 146, 160));
    for k in 1..=4 {
        let f = k as f32 * 0.2;
        painter.line_segment([project(0.0, 1.0, f).0, project(1.0, 1.0, f).0], wgrid);
        painter.line_segment([project(0.0, 0.0, f).0, project(0.0, 1.0, f).0], wgrid);
        painter.line_segment([project(f, 0.0, 0.0).0, project(f, 1.0, 0.0).0], wgrid);
        painter.line_segment([project(f, 1.0, 0.0).0, project(f, 1.0, 1.0).0], wgrid);
        painter.line_segment([project(0.0, f, 0.0).0, project(1.0, f, 0.0).0], wgrid);
        painter.line_segment([project(0.0, f, 0.0).0, project(0.0, f, 1.0).0], wgrid);
    }

    let z_map = |val: f64| -> f32 {
        match metric {
            Air3DMetric::VolPct => {
                // Smooth log mapping covering 0.001..100 % matching Fig. 3 curvature
                let clamped = val.clamp(0.001, 100.0);
                ((clamped.log10() + 3.0) / 5.0) as f32
            }
            Air3DMetric::GNm3 => {
                let clamped = val.clamp(0.008, 804.0);
                (((clamped / 8.03748).max(0.001).log10() + 3.0) / 5.0) as f32
            }
        }
    };

    // Collect surface quads & sort back-to-front (painter's algorithm)
    let mut quads: Vec<(f32, [egui::Pos2; 4], f32)> = Vec::with_capacity((nt - 1) * (nr - 1));
    for ti in 0..(nt - 1) {
        let ny0 = ti as f32 / (nt - 1) as f32;
        let ny1 = (ti + 1) as f32 / (nt - 1) as f32;
        for ri in 0..(nr - 1) {
            let nx0 = ri as f32 / (nr - 1) as f32;
            let nx1 = (ri + 1) as f32 / (nr - 1) as f32;
            let val_at = |t_i: usize, r_i: usize| -> f64 {
                let idx = t_i * nr + r_i;
                match metric {
                    Air3DMetric::VolPct => grid.vol_pct_grid[idx],
                    Air3DMetric::GNm3 => grid.g_nm3_grid[idx],
                }
            };
            let z00 = z_map(val_at(ti, ri));
            let z10 = z_map(val_at(ti, ri + 1));
            let z11 = z_map(val_at(ti + 1, ri + 1));
            let z01 = z_map(val_at(ti + 1, ri));
            let p00 = project(nx0, ny0, z00);
            let p10 = project(nx1, ny0, z10);
            let p11 = project(nx1, ny1, z11);
            let p01 = project(nx0, ny1, z01);
            let avg_depth = (p00.1 + p10.1 + p11.1 + p01.1) * 0.25;
            let avg_z = (z00 + z10 + z11 + z01) * 0.25;
            quads.push((avg_depth, [p00.0, p10.0, p11.0, p01.0], avg_z));
        }
    }
    quads.sort_by(|a, b| b.0.total_cmp(&a.0));

    let edge_stroke = egui::Stroke::new(0.85, egui::Color32::from_rgb(18, 18, 22));
    for (_, pts, z_avg) in quads {
        // Warm golden-amber shading modulated subtly by elevation (`Fig. 3`)
        let s = (0.78 + 0.22 * z_avg).clamp(0.65, 1.0);
        let fill = egui::Color32::from_rgb(
            (244.0 * s) as u8,
            (168.0 * s) as u8,
            (28.0 * s) as u8,
        );
        painter.add(egui::Shape::convex_polygon(pts.to_vec(), fill, edge_stroke));
    }

    // Axis captions & tick labels
    let txt_col = egui::Color32::from_rgb(20, 22, 28);
    let rh_mid = project(0.5, -0.14, 0.0).0;
    painter.text(
        rh_mid,
        egui::Align2::CENTER_CENTER,
        "Relative Humidity (%)",
        egui::FontId::proportional(12.0),
        txt_col,
    );
    for (rh_v, nx) in [(20, 0.11), (40, 0.33), (60, 0.56), (80, 0.78), (100, 1.0)] {
        painter.text(
            project(nx, -0.05, 0.0).0,
            egui::Align2::CENTER_TOP,
            format!("{rh_v}"),
            egui::FontId::proportional(10.5),
            txt_col,
        );
    }

    let t_mid = project(1.18, 0.52, 0.0).0;
    painter.text(
        t_mid,
        egui::Align2::LEFT_CENTER,
        "Temperature (°C)",
        egui::FontId::proportional(12.0),
        txt_col,
    );
    for (t_v, ny) in [(0, 0.33), (100, 0.99)] {
        painter.text(
            project(1.05, ny, 0.0).0,
            egui::Align2::LEFT_CENTER,
            format!("{t_v}"),
            egui::FontId::proportional(10.5),
            txt_col,
        );
    }

    let z_lbl = match metric {
        Air3DMetric::VolPct => "H2O (vol-%)",
        Air3DMetric::GNm3 => "H2O (g/Nm3)",
    };
    painter.text(
        project(-0.14, 0.0, 0.52).0,
        egui::Align2::RIGHT_CENTER,
        z_lbl,
        egui::FontId::proportional(12.0),
        txt_col,
    );
    for (lbl, nz) in match metric {
        Air3DMetric::VolPct => [("0.0", 0.0), ("0.1", 0.4), ("1.0", 0.6), ("10.0", 0.8), ("100.0", 1.0)],
        Air3DMetric::GNm3 => [("0.0", 0.0), ("1.0", 0.4), ("10.0", 0.6), ("100.0", 0.8), ("800", 1.0)],
    } {
        painter.text(
            project(-0.04, 0.0, nz).0,
            egui::Align2::RIGHT_CENTER,
            lbl,
            egui::FontId::proportional(10.0),
            txt_col,
        );
    }
}

fn excel_col_header(ui: &mut egui::Ui, text: &str, width: f32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let bg = if active {
            egui::Color32::from_rgb(198, 220, 248)
        } else {
            EXCEL_HEADER_BG
        };
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(11.5),
            egui::Color32::from_rgb(32, 36, 44),
        );
    }
}

fn excel_row_header(ui: &mut egui::Ui, row_num: usize, width: f32, active: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 21.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let bg = if active {
            egui::Color32::from_rgb(198, 220, 248)
        } else {
            EXCEL_HEADER_BG
        };
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_HEADER_BORDER));
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            row_num.to_string(),
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(32, 36, 44),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_sheet_static_cell(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
    right: bool,
    active: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 21.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 0.0, bg);
        p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
        let clipped = p.with_clip_rect(rect.shrink2(egui::vec2(4.0, 1.0)));
        let (pos, align) = if center {
            (rect.center(), egui::Align2::CENTER_CENTER)
        } else if right {
            (
                rect.right_center() - egui::vec2(6.0, 0.0),
                egui::Align2::RIGHT_CENTER,
            )
        } else {
            (
                rect.left_center() + egui::vec2(5.0, 0.0),
                egui::Align2::LEFT_CENTER,
            )
        };
        let font = egui::FontId::proportional(11.5);
        clipped.text(pos, align, text, font.clone(), fg);
        if bold {
            clipped.text(pos + egui::vec2(0.45, 0.0), align, text, font, fg);
        }
        if active {
            p.rect_stroke(rect.shrink(0.5), 0.0, egui::Stroke::new(1.6, EXCEL_ACTIVE_BORDER));
        }
    }
    resp
}

fn draw_editable_sheet_cell(
    ui: &mut egui::Ui,
    id_salt: &str,
    buf: &mut String,
    w: f32,
    bg: egui::Color32,
    right: bool,
    active: bool,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 21.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter()
        .rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(1.5)));
    let align = if right {
        egui::Align::RIGHT
    } else {
        egui::Align::LEFT
    };
    let resp = child.add(
        egui::TextEdit::singleline(buf)
            .id(egui::Id::new(id_salt))
            .desired_width(w - 6.0)
            .frame(false)
            .horizontal_align(align)
            .text_color(egui::Color32::from_rgb(18, 22, 28))
            .font(egui::FontId::proportional(11.5)),
    );
    if active {
        ui.painter()
            .rect_stroke(rect.shrink(0.5), 0.0, egui::Stroke::new(1.6, EXCEL_ACTIVE_BORDER));
        // Small Excel fill handle square at bottom-right matching Fig. 1 (`A7`)
        let sq = egui::Rect::from_center_size(rect.right_bottom() - egui::vec2(1.5, 1.5), egui::vec2(4.0, 4.0));
        ui.painter().rect_filled(sq, 0.0, EXCEL_ACTIVE_BORDER);
    }
    resp
}

fn hsc_btn(ui: &mut egui::Ui, label: &str, w: f32) -> egui::Response {
    ui.add_sized(
        [w, 21.0],
        egui::Button::new(
            egui::RichText::new(label)
                .size(11.5)
                .color(egui::Color32::from_rgb(20, 24, 30)),
        )
        .fill(egui::Color32::from_rgb(230, 233, 240))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 175, 188)))
        .rounding(2.0),
    )
}

fn parse_decimal_input(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.contains(',') && !trimmed.contains('.') {
        trimmed.replace(',', ".")
    } else {
        trimmed.to_string()
    };
    let val = normalized.parse::<f64>().ok()?;
    val.is_finite().then_some(val)
}

fn is_incomplete_numeric_prefix(raw: &str) -> bool {
    let t = raw.trim();
    matches!(t, "-" | "+" | "." | "," | "-." | "+." | "-," | "+,")
        || t.ends_with(['e', 'E'])
        || t.ends_with("e+")
        || t.ends_with("e-")
        || t.ends_with("E+")
        || t.ends_with("E-")
}

fn fmt_excel(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let abs = v.abs();
    if !(1e-5..1e9).contains(&abs) {
        return format!("{v:.6E}");
    }
    let s = format!("{v:.8}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn fmt_unit_coef(v: f64) -> String {
    if (v - 1.0).abs() < 1e-12 {
        return "1.0000".to_string();
    }
    let abs = v.abs();
    if abs < 1e-3 || abs >= 1e5 {
        format!("{v:.4E}")
    } else {
        format!("{v:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}
