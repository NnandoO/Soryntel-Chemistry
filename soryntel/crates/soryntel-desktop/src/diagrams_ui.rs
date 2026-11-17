use eframe::egui;
use serde::{Deserialize, Serialize};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::{
    diagrams::{
        balanced_coeff_for_element, calculate_diagram, ellingham_po2_isoline_val,
        ellingham_scales_at, species_db_temp_range, DiagramEnergyUnit, DiagramRequest,
        DiagramResult, DiagramSeries, DiagramSpeciesInput, DiagramType,
    },
    species::DbSource,
    ChemicalSpecies, SpeciesRepository,
};

use crate::excel_grid::{
    header_cell, row_num_cell, text_cell, widget_cell, EXCEL_ALT_ROW_BG, EXCEL_GRID_BORDER,
    EXCEL_READONLY_BG,
};
use crate::ui_common::{draw_dashed_line, draw_rotated_y_title, format_tick, nice_step};

/// 99 unique chemical elements arranged in 9 columns matching HSC6 Ch. 19 element picker.
const DIAG_ELEMENTS: &[&str] = &[
    "Ac", "Ag", "Al", "Am", "Ar", "As", "At", "Au", "B",
    "Ba", "Be", "Bi", "Bk", "Br", "C", "Ca", "Cd", "Ce",
    "Cf", "Cl", "Cm", "Co", "Cr", "Cs", "Cu", "Dy", "Er",
    "Es", "Eu", "F", "Fe", "Fm", "Fr", "Ga", "Gd", "Ge",
    "H", "He", "Hf", "Hg", "Ho", "I", "In", "Ir", "K",
    "Kr", "La", "Li", "Lu", "Mg", "Mn", "Mo", "N", "Na",
    "Nb", "Nd", "Ne", "Ni", "Np", "O", "Os", "P", "Pa",
    "Pb", "Pd", "Pm", "Po", "Pr", "Pt", "Pu", "Ra", "Rb",
    "Re", "Rh", "Rn", "Ru", "S", "Sb", "Sc", "Se", "Si",
    "Sm", "Sn", "Sr", "Ta", "Tb", "Tc", "Te", "Th", "Ti",
    "Tl", "Tm", "U", "V", "W", "Xe", "Y", "Yb", "Zn",
    "Zr", "", "", "", "", "", "", "", "",
];

const CURVE_COLORS: &[(u8, u8, u8)] = &[
    (95, 135, 65),
    (22, 110, 125),
    (45, 45, 55),
    (115, 110, 80),
    (185, 115, 35),
    (30, 55, 105),
    (37, 99, 235),
    (14, 150, 180),
    (0, 128, 128),
    (155, 45, 95),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataSheetTab {
    XData,
    YData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagRowSave {
    formula: String,
    coeff_str: String,
    db_no: u8,
    visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagSaveFile {
    version: u32,
    rows: Vec<DiagRowSave>,
    diagram_type: DiagramType,
    t_min_str: String,
    t_max_str: String,
    t_step_str: String,
    temp_in_kelvin: bool,
    y_min_str: String,
    y_max_str: String,
    y_auto: bool,
    energy_unit: DiagramEnergyUnit,
    per_kg: bool,
    balance_element: String,
    balance_amount_str: String,
    show_po2_lines: bool,
    show_curve_labels: bool,
    line_width: f32,
}

#[derive(Debug, Clone)]
struct DiagramRow {
    formula: String,
    coeff_str: String,
    /// `0` = Auto (Own>Main), `1` = OwnDB only, `2` = MainDB only (§19 step 4).
    db_no: u8,
    t_min_str: String,
    t_max_str: String,
    visible: bool,
}

impl DiagramRow {
    fn new(formula: &str, coeff: f64, db_no: u8, db: &ThermoDatabase, in_kelvin: bool) -> Self {
        let mut r = Self {
            formula: formula.to_string(),
            coeff_str: format!("{coeff:.2}"),
            db_no,
            t_min_str: String::new(),
            t_max_str: String::new(),
            visible: true,
        };
        r.refresh_db_range(db, in_kelvin);
        r
    }

    fn empty() -> Self {
        Self {
            formula: String::new(),
            coeff_str: "1.00".to_string(),
            db_no: 0,
            t_min_str: String::new(),
            t_max_str: String::new(),
            visible: true,
        }
    }

    fn resolve_species(&self, db: &ThermoDatabase) -> Option<ChemicalSpecies> {
        let f = self.formula.trim();
        if f.is_empty() {
            return None;
        }
        match self.db_no {
            1 => db
                .get_all_datasets_for(DbSource::Own, f)
                .into_iter()
                .next()
                .map(|d| d.species),
            2 => db
                .get_all_datasets_for(DbSource::Main, f)
                .into_iter()
                .next()
                .map(|d| d.species),
            _ => db.get_species(f),
        }
    }

    fn refresh_db_range(&mut self, db: &ThermoDatabase, in_kelvin: bool) {
        if let Some(sp) = self.resolve_species(db) {
            let (tmin_k, tmax_k) = species_db_temp_range(&sp);
            let off = if in_kelvin { 0.0 } else { 273.15 };
            self.t_min_str = format!("{:.2}", tmin_k - off);
            self.t_max_str = format!("{:.2}", tmax_k - off);
        } else {
            self.t_min_str.clear();
            self.t_max_str.clear();
        }
    }

    fn coeff(&self) -> f64 {
        self.coeff_str.trim().parse::<f64>().unwrap_or(1.0).max(1e-6)
    }

    fn db_badge(&self) -> &'static str {
        match self.db_no {
            1 => "1:Own",
            2 => "2:Main",
            _ => "Auto",
        }
    }
}

pub struct DiagramsModuleState {
    rows: Vec<DiagramRow>,
    active_row: usize,
    sheet_tab: DataSheetTab,
    diagram_type: DiagramType,
    t_min_str: String,
    t_max_str: String,
    t_step_str: String,
    temp_in_kelvin: bool,
    y_min_str: String,
    y_max_str: String,
    y_auto: bool,
    energy_unit: DiagramEnergyUnit,
    per_kg: bool,
    balance_element: String,
    balance_amount_str: String,
    show_po2_lines: bool,
    show_curve_labels: bool,
    line_width: f32,
    result: Option<DiagramResult>,
    status_error: Option<String>,
    status_info: Option<String>,
    show_db_browser: bool,
    db_query: String,
    db_matches: Vec<ChemicalSpecies>,
    show_file_modal: bool,
    file_modal_is_save: bool,
    file_path_buf: String,
    show_help_modal: bool,
}

impl DiagramsModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let initial = vec![
            ("Cu2O", 2.0),
            ("FeO", 2.0),
            ("Fe2O3", 2.0 / 3.0),
            ("ZnO", 2.0),
            ("Cr2O3", 2.0 / 3.0),
            ("MgO", 2.0),
            ("CO(g)", 2.0),
            ("CO2(g)", 1.0),
            ("H2O(g)", 2.0),
        ];
        let mut rows: Vec<DiagramRow> = initial
            .into_iter()
            .map(|(f, c)| DiagramRow::new(f, c, 0, db, false))
            .collect();
        for _ in 0..8 {
            rows.push(DiagramRow::empty());
        }

        let mut st = Self {
            rows,
            active_row: 0,
            sheet_tab: DataSheetTab::XData,
            diagram_type: DiagramType::DeltaGEllingham,
            t_min_str: "0.00".to_string(),
            t_max_str: "2000.00".to_string(),
            t_step_str: "50.00".to_string(),
            temp_in_kelvin: false,
            y_min_str: "-1200.00".to_string(),
            y_max_str: "0.00".to_string(),
            y_auto: true,
            energy_unit: DiagramEnergyUnit::KiloJoules,
            per_kg: false,
            balance_element: "O".to_string(),
            balance_amount_str: "2.00".to_string(),
            show_po2_lines: true,
            show_curve_labels: true,
            line_width: 1.5,
            result: None,
            status_error: None,
            status_info: None,
            show_db_browser: false,
            db_query: "Fe".to_string(),
            db_matches: db.search("Fe", 40),
            show_file_modal: false,
            file_modal_is_save: true,
            file_path_buf: "/home/nando/.gemini/antigravity/scratch/diagram.dia8".to_string(),
            show_help_modal: false,
        };
        st.recalculate(db);
        st
    }

    fn recalculate(&mut self, db: &ThermoDatabase) {
        let t_min_val: f64 = self.t_min_str.trim().parse().unwrap_or(0.0);
        let t_max_val: f64 = self.t_max_str.trim().parse().unwrap_or(2000.0);
        let step_val: f64 = self.t_step_str.trim().parse().unwrap_or(50.0);
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };

        let species: Vec<DiagramSpeciesInput> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.formula.trim().is_empty())
            .map(|(row_idx, r)| DiagramSpeciesInput {
                formula: r.formula.trim().to_string(),
                coeff: r.coeff(),
                row_index: Some(row_idx),
                species_override: if r.db_no != 0 {
                    r.resolve_species(db)
                } else {
                    None
                },
            })
            .collect();

        let req = DiagramRequest {
            species,
            diagram_type: self.diagram_type,
            t_min_k: (t_min_val + off).max(1.0),
            t_max_k: (t_max_val + off).max(2.0),
            step_k: step_val.max(1.0),
            energy_unit: self.energy_unit,
            per_kg: self.per_kg,
        };

        match calculate_diagram(&req, db) {
            Ok(res) => {
                self.status_error = if res.warnings.is_empty() {
                    None
                } else {
                    Some(res.warnings.join(" | "))
                };
                self.result = Some(res);
                self.update_auto_y_range();
            }
            Err(e) => {
                self.status_error = Some(e);
            }
        }
    }

    fn is_series_visible(&self, s: &DiagramSeries, fallback_idx: usize) -> bool {
        let row_idx = s.row_index.unwrap_or(fallback_idx);
        self.rows.get(row_idx).map(|r| r.visible).unwrap_or(true)
    }

    fn update_auto_y_range(&mut self) {
        if !self.y_auto {
            return;
        }
        let Some(res) = &self.result else {
            return;
        };
        let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
        for (idx, s) in res.series.iter().enumerate() {
            if !self.is_series_visible(s, idx) {
                continue;
            }
            for p in &s.points {
                ymin = ymin.min(p.y_val);
                ymax = ymax.max(p.y_val);
            }
        }
        if ymin.is_finite() && ymax.is_finite() {
            let pad = ((ymax - ymin) * 0.06).max(1.0);
            self.y_min_str = format!("{:.2}", ymin - pad);
            self.y_max_str = format!("{:.2}", ymax + pad);
        }
    }

    fn balance_element_amounts(&mut self) {
        let target: f64 = self.balance_amount_str.trim().parse().unwrap_or(2.0);
        if target <= 0.0 || self.balance_element.is_empty() {
            return;
        }
        for r in &mut self.rows {
            if r.formula.trim().is_empty() {
                continue;
            }
            if let Some(c) =
                balanced_coeff_for_element(r.formula.trim(), &self.balance_element, target)
            {
                r.coeff_str = format!("{c:.2}");
            }
        }
    }

    fn load_preset(
        &mut self,
        db: &ThermoDatabase,
        dtype: DiagramType,
        items: &[(&str, f64)],
        bal_elem: &str,
        bal_amt: f64,
        t_max_c: f64,
    ) {
        self.rows.clear();
        for &(f, c) in items {
            self.rows.push(DiagramRow::new(f, c, 0, db, false));
        }
        for _ in 0..6 {
            self.rows.push(DiagramRow::empty());
        }
        self.active_row = 0;
        self.diagram_type = dtype;
        self.temp_in_kelvin = false;
        self.t_min_str = "0.00".to_string();
        self.t_max_str = format!("{t_max_c:.2}");
        self.t_step_str = "50.00".to_string();
        self.y_auto = true;
        self.per_kg = false;
        self.energy_unit = DiagramEnergyUnit::KiloJoules;
        self.balance_element = bal_elem.to_string();
        self.balance_amount_str = format!("{bal_amt:.2}");
        self.recalculate(db);
    }

    fn build_save_file(&self) -> DiagSaveFile {
        DiagSaveFile {
            version: 1,
            rows: self
                .rows
                .iter()
                .filter(|r| !r.formula.trim().is_empty())
                .map(|r| DiagRowSave {
                    formula: r.formula.clone(),
                    coeff_str: r.coeff_str.clone(),
                    db_no: r.db_no,
                    visible: r.visible,
                })
                .collect(),
            diagram_type: self.diagram_type,
            t_min_str: self.t_min_str.clone(),
            t_max_str: self.t_max_str.clone(),
            t_step_str: self.t_step_str.clone(),
            temp_in_kelvin: self.temp_in_kelvin,
            y_min_str: self.y_min_str.clone(),
            y_max_str: self.y_max_str.clone(),
            y_auto: self.y_auto,
            energy_unit: self.energy_unit,
            per_kg: self.per_kg,
            balance_element: self.balance_element.clone(),
            balance_amount_str: self.balance_amount_str.clone(),
            show_po2_lines: self.show_po2_lines,
            show_curve_labels: self.show_curve_labels,
            line_width: self.line_width,
        }
    }

    fn save_dia8(&mut self, path: &str) {
        let sf = self.build_save_file();
        match serde_json::to_string_pretty(&sf) {
            Ok(json) => match crate::storage::write_atomic(path, json) {
                Ok(_) => {
                    self.status_error = None;
                    self.status_info = Some(format!("Saved .dia8 to {path}"));
                }
                Err(e) => self.status_error = Some(format!("Save error: {e}")),
            },
            Err(e) => self.status_error = Some(format!("Serialize error: {e}")),
        }
    }

    fn open_dia8(&mut self, db: &ThermoDatabase, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(txt) => match serde_json::from_str::<DiagSaveFile>(&txt) {
                Ok(sf) => {
                    self.rows = sf
                        .rows
                        .into_iter()
                        .map(|r| {
                            let mut row = DiagramRow::new(
                                &r.formula,
                                r.coeff_str.trim().parse().unwrap_or(1.0),
                                r.db_no,
                                db,
                                sf.temp_in_kelvin,
                            );
                            row.coeff_str = r.coeff_str;
                            row.visible = r.visible;
                            row
                        })
                        .collect();
                    for _ in 0..6 {
                        self.rows.push(DiagramRow::empty());
                    }
                    self.diagram_type = sf.diagram_type;
                    self.t_min_str = sf.t_min_str;
                    self.t_max_str = sf.t_max_str;
                    self.t_step_str = sf.t_step_str;
                    self.temp_in_kelvin = sf.temp_in_kelvin;
                    self.y_min_str = sf.y_min_str;
                    self.y_max_str = sf.y_max_str;
                    self.y_auto = sf.y_auto;
                    self.energy_unit = sf.energy_unit;
                    self.per_kg = sf.per_kg;
                    self.balance_element = sf.balance_element;
                    self.balance_amount_str = sf.balance_amount_str;
                    self.show_po2_lines = sf.show_po2_lines;
                    self.show_curve_labels = sf.show_curve_labels;
                    self.line_width = sf.line_width.clamp(0.8, 3.5);
                    self.recalculate(db);
                    if self.status_error.is_none() {
                        self.status_info = Some(format!("Opened {path}"));
                    }
                }
                Err(e) => self.status_error = Some(format!("Invalid .dia8 file: {e}")),
            },
            Err(e) => self.status_error = Some(format!("Cannot read {path}: {e}")),
        }
    }

    fn export_ydata_csv(&mut self, path: &str) {
        let Some(res) = &self.result else {
            self.status_error = Some("No calculated diagram data to export.".to_string());
            return;
        };
        let t_unit = if self.temp_in_kelvin { "K" } else { "C" };
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };
        let vis_series: Vec<&DiagramSeries> = res
            .series
            .iter()
            .enumerate()
            .filter(|(i, s)| self.is_series_visible(s, *i))
            .map(|(_, s)| s)
            .collect();
        let mut out = format!("T ({t_unit})");
        for s in &vis_series {
            out.push_str(&format!(",{} [{}]", s.label, res.y_unit));
        }
        out.push('\n');
        let n_pts = vis_series.first().map(|s| s.points.len()).unwrap_or(0);
        for pi in 0..n_pts {
            let t_disp = vis_series[0].points[pi].t_k - off;
            out.push_str(&format!("{t_disp:.2}"));
            for s in &vis_series {
                let y = s.points.get(pi).map(|p| p.y_val).unwrap_or(f64::NAN);
                out.push_str(&format!(",{y:.4}"));
            }
            out.push('\n');
        }
        match crate::storage::write_atomic(path, out) {
            Ok(_) => {
                self.status_error = None;
                self.status_info = Some(format!("Exported Y-data CSV to {path}"));
            }
            Err(e) => self.status_error = Some(format!("CSV export failed: {e}")),
        }
    }

    fn export_chart_svg(&mut self, path: &str) {
        let Some(res) = &self.result else {
            self.status_error = Some("No diagram chart to export.".to_string());
            return;
        };
        let t_min: f64 = self.t_min_str.trim().parse().unwrap_or(0.0);
        let t_max: f64 = self.t_max_str.trim().parse::<f64>().unwrap_or(2000.0).max(t_min + 1.0);
        let y_min: f64 = self.y_min_str.trim().parse().unwrap_or(-1200.0);
        let y_max: f64 = self.y_max_str.trim().parse::<f64>().unwrap_or(0.0).max(y_min + 1.0);
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };
        let (w, h, ml, mr, mt, mb) = (900.0_f64, 620.0_f64, 82.0, 32.0, 48.0, 62.0);
        let (pw, ph) = (w - ml - mr, h - mt - mb);
        let sx = |t: f64| ml + ((t - t_min) / (t_max - t_min)) * pw;
        let sy = |y: f64| mt + ph - ((y - y_min) / (y_max - y_min)) * ph;

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" style=\"background:#fff;font-family:sans-serif\">\n\
             <text x=\"{:.1}\" y=\"28\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>\n\
             <rect x=\"{ml}\" y=\"{mt}\" width=\"{pw}\" height=\"{ph}\" fill=\"none\" stroke=\"#000\" stroke-width=\"1.4\"/>\n",
            w * 0.5,
            xml_escape(res.diagram_type.label())
        );
        for (idx, s) in res.series.iter().enumerate() {
            if !self.is_series_visible(s, idx) || s.points.len() < 2 {
                continue;
            }
            let (r, g, b) = CURVE_COLORS[idx % CURVE_COLORS.len()];
            let pts_attr: String = s
                .points
                .iter()
                .map(|p| format!("{:.1},{:.1}", sx(p.t_k - off), sy(p.y_val)))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                "<polyline fill=\"none\" stroke=\"rgb({r},{g},{b})\" stroke-width=\"{:.1}\" points=\"{pts_attr}\"/>\n",
                self.line_width
            ));
            if self.show_curve_labels {
                let li = ((s.points.len() * (50 + (idx % 4) * 11)) / 100).min(s.points.len() - 1);
                let lp = s.points[li];
                svg.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"rgb({r},{g},{b})\">{}</text>\n",
                    sx(lp.t_k - off),
                    sy(lp.y_val) - 7.0,
                    xml_escape(&s.label)
                ));
            }
        }
        let xl = if self.temp_in_kelvin { "Temperature (K)" } else { "Temperature (°C)" };
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\">{xl}</text>\n\
             <text transform=\"rotate(-90 22 {:.1})\" x=\"22\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\">{}</text>\n</svg>\n",
            ml + pw * 0.5,
            h - 20.0,
            mt + ph * 0.5,
            mt + ph * 0.5,
            xml_escape(&res.y_unit)
        ));
        match crate::storage::write_atomic(path, svg) {
            Ok(_) => {
                self.status_error = None;
                self.status_info = Some(format!("Exported SVG to {path}"));
            }
            Err(e) => self.status_error = Some(format!("SVG export failed: {e}")),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = egui::Color32::from_rgb(240, 242, 245);
        vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(vis);

        // Top menu bar (§19 Fig. 1 File | Format | Help)
        egui::TopBottomPanel::top("diag_top_menu")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 240, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("Open (.dia8)...").clicked() {
                            self.file_modal_is_save = false;
                            self.show_file_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Save (.dia8)...").clicked() {
                            self.file_modal_is_save = true;
                            self.show_file_modal = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Example: Ellingham Oxides (Fig. 2–4)").clicked() {
                            self.load_preset(
                                db,
                                DiagramType::DeltaGEllingham,
                                &[
                                    ("Cu2O", 2.0),
                                    ("FeO", 2.0),
                                    ("Fe2O3", 2.0 / 3.0),
                                    ("ZnO", 2.0),
                                    ("Cr2O3", 2.0 / 3.0),
                                    ("MgO", 2.0),
                                    ("CO(g)", 2.0),
                                    ("CO2(g)", 1.0),
                                    ("H2O(g)", 2.0),
                                ],
                                "O",
                                2.0,
                                2000.0,
                            );
                            ui.close_menu();
                        }
                        if ui.button("Example: H Heat Content (Fig. 1)").clicked() {
                            self.load_preset(
                                db,
                                DiagramType::HHeatContent,
                                &[
                                    ("Cu2O", 1.0),
                                    ("FeO", 1.0),
                                    ("Fe2O3", 1.0),
                                    ("Fe3O4", 1.0),
                                    ("CoO", 1.0),
                                    ("Cr2O3", 1.0),
                                    ("MgO", 1.0),
                                    ("CO(g)", 1.0),
                                ],
                                "O",
                                1.0,
                                2000.0,
                            );
                            ui.close_menu();
                        }
                        if ui.button("Example: Exergy (total) Curves").clicked() {
                            self.load_preset(
                                db,
                                DiagramType::ExExergy,
                                &[
                                    ("Fe", 1.0),
                                    ("FeO", 1.0),
                                    ("Fe2O3", 1.0),
                                    ("CO(g)", 1.0),
                                    ("CO2(g)", 1.0),
                                ],
                                "O",
                                1.0,
                                1500.0,
                            );
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export Chart SVG...").clicked() {
                            self.export_chart_svg("/home/nando/.gemini/antigravity/scratch/diagram_chart.svg");
                            ui.close_menu();
                        }
                        if ui.button("Export Y-data CSV...").clicked() {
                            self.export_ydata_csv("/home/nando/.gemini/antigravity/scratch/diagram_ydata.csv");
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.019"), |ui| {
                        ui.checkbox(&mut self.show_po2_lines, "Show pO2 Iso-Lines (Ellingham)");
                        ui.checkbox(&mut self.show_curve_labels, "Show Curve Labels");
                        ui.add(egui::Slider::new(&mut self.line_width, 0.8..=3.2).text("Line Width"));
                        if ui.button("Fit Y-Axis Now").clicked() {
                            self.y_auto = true;
                            self.update_auto_y_range();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button("H, S, C and G Diagrams Guide (Ch. 19)").clicked() {
                            self.show_help_modal = true;
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.strong("Thermochemical Data Diagrams");
                    ui.separator();
                    let active_f = self
                        .rows
                        .get(self.active_row)
                        .map(|r| r.formula.as_str())
                        .unwrap_or("");
                    ui.label(crate::i18n::text("common.065"));
                    ui.label(egui::RichText::new(active_f).strong());
                    if let Some(info) = &self.status_info {
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(21, 128, 61), info);
                    }
                    if let Some(err) = &self.status_error {
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                    }
                });
            });

        // 1. Left Resizable Spreadsheet Panel (X-data / Y-data)
        egui::SidePanel::left("diag_left_sheet")
            .resizable(true)
            .default_width(395.0)
            .min_width(195.0)
            .max_width(680.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.sheet_tab == DataSheetTab::XData, "X-data")
                        .clicked()
                    {
                        self.sheet_tab = DataSheetTab::XData;
                    }
                    if ui
                        .selectable_label(self.sheet_tab == DataSheetTab::YData, "Y-data")
                        .clicked()
                    {
                        self.sheet_tab = DataSheetTab::YData;
                    }
                    if ui.small_button("+ Row").clicked() {
                        self.rows.push(DiagramRow::empty());
                    }
                    if self.sheet_tab == DataSheetTab::YData && ui.small_button(crate::i18n::text("common.031")).clicked() {
                        self.export_ydata_csv("/home/nando/.gemini/antigravity/scratch/diagram_ydata.csv");
                    }
                });
                ui.separator();

                egui::ScrollArea::both()
                    .id_salt("diag_sheet_scroll")
                    .show(ui, |ui| match self.sheet_tab {
                        DataSheetTab::XData => self.ui_xdata_grid(ui, db),
                        DataSheetTab::YData => self.ui_ydata_grid(ui),
                    });
            });

        // 2. Center Resizable Controls Panel
        egui::SidePanel::left("diag_center_controls")
            .resizable(true)
            .default_width(245.0)
            .min_width(175.0)
            .max_width(420.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 245))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.label(egui::RichText::new("Diagram Type").strong().size(11.5));
                    egui::ComboBox::from_id_salt("diag_type_combo")
                        .width(ui.available_width().max(160.0) - 8.0)
                        .selected_text(self.diagram_type.label())
                        .show_ui(ui, |ui| {
                            for &dt in DiagramType::ALL {
                                if ui
                                    .selectable_value(&mut self.diagram_type, dt, dt.label())
                                    .changed()
                                {
                                    self.recalculate(db);
                                }
                            }
                        });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("X-axis Range (Temperature)")
                                .strong()
                                .size(11.0),
                        );
                        egui::Grid::new("diag_x_range_grid")
                            .spacing(egui::vec2(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label("MIN");
                                ui.label("MAX");
                                ui.label("STEP");
                                ui.end_row();
                                ui.add_sized(
                                    [58.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_min_str),
                                );
                                ui.add_sized(
                                    [58.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_max_str),
                                );
                                ui.add_sized(
                                    [52.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.t_step_str),
                                );
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            if ui.radio(!self.temp_in_kelvin, "°C").clicked() && self.temp_in_kelvin {
                                self.temp_in_kelvin = false;
                                if let Ok(v) = self.t_min_str.trim().parse::<f64>() {
                                    self.t_min_str = format!("{:.2}", v - 273.15);
                                }
                                if let Ok(v) = self.t_max_str.trim().parse::<f64>() {
                                    self.t_max_str = format!("{:.2}", v - 273.15);
                                }
                                for r in &mut self.rows {
                                    r.refresh_db_range(db, false);
                                }
                                self.recalculate(db);
                            }
                            if ui.radio(self.temp_in_kelvin, "K").clicked() && !self.temp_in_kelvin {
                                self.temp_in_kelvin = true;
                                if let Ok(v) = self.t_min_str.trim().parse::<f64>() {
                                    self.t_min_str = format!("{:.2}", v + 273.15);
                                }
                                if let Ok(v) = self.t_max_str.trim().parse::<f64>() {
                                    self.t_max_str = format!("{:.2}", v + 273.15);
                                }
                                for r in &mut self.rows {
                                    r.refresh_db_range(db, true);
                                }
                                self.recalculate(db);
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Y-axis Range").strong().size(11.0));
                        egui::Grid::new("diag_y_range_grid")
                            .spacing(egui::vec2(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label("MIN");
                                ui.label("MAX");
                                ui.label("");
                                ui.end_row();
                                ui.add_enabled(
                                    !self.y_auto,
                                    egui::TextEdit::singleline(&mut self.y_min_str)
                                        .desired_width(60.0),
                                );
                                ui.add_enabled(
                                    !self.y_auto,
                                    egui::TextEdit::singleline(&mut self.y_max_str)
                                        .desired_width(60.0),
                                );
                                if ui.checkbox(&mut self.y_auto, "Auto").changed() && self.y_auto {
                                    self.update_auto_y_range();
                                }
                                ui.end_row();
                            });
                        ui.horizontal(|ui| {
                            if ui
                                .radio(
                                    self.energy_unit == DiagramEnergyUnit::KiloCalories,
                                    "kcal/mol",
                                )
                                .clicked()
                            {
                                self.energy_unit = DiagramEnergyUnit::KiloCalories;
                                self.recalculate(db);
                            }
                            if ui
                                .radio(
                                    self.energy_unit == DiagramEnergyUnit::KiloJoules,
                                    "kJ/mol",
                                )
                                .clicked()
                            {
                                self.energy_unit = DiagramEnergyUnit::KiloJoules;
                                self.recalculate(db);
                            }
                            if ui.checkbox(&mut self.per_kg, "kg").changed() {
                                self.recalculate(db);
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("Balance Element Amount").clicked() {
                            self.balance_element_amounts();
                            self.recalculate(db);
                        }
                        ui.add_sized(
                            [52.0, 20.0],
                            egui::TextEdit::singleline(&mut self.balance_amount_str),
                        );
                    });

                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(3.0))
                        .show(ui, |ui| {
                            egui::Grid::new("diag_elem_grid_9col")
                                .spacing(egui::vec2(1.5, 1.5))
                                .show(ui, |ui| {
                                    for (idx, &sym) in DIAG_ELEMENTS.iter().enumerate() {
                                        if sym.is_empty() {
                                            ui.label("");
                                        } else {
                                            let sel = self.balance_element == sym;
                                            let btn = egui::Button::new(
                                                egui::RichText::new(sym).size(10.0),
                                            )
                                            .min_size(egui::vec2(21.0, 16.0))
                                            .fill(if sel {
                                                egui::Color32::from_rgb(191, 219, 254)
                                            } else {
                                                egui::Color32::WHITE
                                            });
                                            if ui.add(btn).clicked() {
                                                self.balance_element = sym.to_string();
                                            }
                                        }
                                        if (idx + 1) % 9 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    ui.add_space(6.0);
                    if ui
                        .add_sized(
                            [ui.available_width().max(150.0), 24.0],
                            egui::Button::new("Browse Database"),
                        )
                        .clicked()
                    {
                        self.show_db_browser = true;
                    }
                    ui.add_space(2.0);
                    if ui
                        .add_sized(
                            [ui.available_width().max(150.0), 26.0],
                            egui::Button::new("Diagram"),
                        )
                        .clicked()
                    {
                        self.recalculate(db);
                    }
                });
            });

        // 3. Right Central Chart Panel
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .id_salt("diag_chart_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.ui_chart(ui, db);
                    });
            });

        if self.show_db_browser {
            self.ui_db_browser(ctx, db);
        }
        self.ui_extra_modals(ctx, db);
    }

    fn ui_xdata_grid(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let t_unit = if self.temp_in_kelvin { "K" } else { "°C" };
        ui.horizontal(|ui| {
            row_num_cell(ui, 0, 26.0);
            header_cell(ui, "A", 104.0);
            header_cell(ui, "B", 50.0);
            header_cell(ui, "C", 68.0);
            header_cell(ui, "D", 68.0);
            header_cell(ui, "DB", 48.0);
            header_cell(ui, "", 22.0);
        });
        ui.horizontal(|ui| {
            row_num_cell(ui, 1, 26.0);
            header_cell(ui, "Species", 104.0);
            header_cell(ui, "Coef.", 50.0);
            header_cell(ui, &format!("TMin ({t_unit})"), 68.0);
            header_cell(ui, &format!("TMax ({t_unit})"), 68.0);
            header_cell(ui, "No", 48.0);
            header_cell(ui, "×", 22.0);
        });

        let mut remove_idx = None;
        for idx in 0..self.rows.len() {
            let bg = if idx % 2 == 0 {
                egui::Color32::WHITE
            } else {
                EXCEL_ALT_ROW_BG
            };
            let is_active = self.active_row == idx;
            ui.horizontal(|ui| {
                row_num_cell(ui, idx + 2, 26.0);
                widget_cell(ui, 104.0, bg, is_active, |ui| {
                    let r = ui.text_edit_singleline(&mut self.rows[idx].formula);
                    if r.gained_focus() {
                        self.active_row = idx;
                    }
                    if r.lost_focus() {
                        let tk = self.temp_in_kelvin;
                        self.rows[idx].refresh_db_range(db, tk);
                    }
                });
                widget_cell(ui, 50.0, bg, false, |ui| {
                    ui.text_edit_singleline(&mut self.rows[idx].coeff_str);
                });
                text_cell(
                    ui,
                    &self.rows[idx].t_min_str,
                    68.0,
                    EXCEL_READONLY_BG,
                    egui::Color32::BLACK,
                    true,
                    true,
                    false,
                );
                text_cell(
                    ui,
                    &self.rows[idx].t_max_str,
                    68.0,
                    EXCEL_READONLY_BG,
                    egui::Color32::BLACK,
                    true,
                    true,
                    false,
                );
                widget_cell(ui, 48.0, bg, false, |ui| {
                    let lbl = self.rows[idx].db_badge();
                    if ui
                        .small_button(lbl)
                        .on_hover_text("Click to cycle: Auto -> 1:OwnDB -> 2:MainDB (§19 step 4)")
                        .clicked()
                    {
                        self.rows[idx].db_no = (self.rows[idx].db_no + 1) % 3;
                        let tk = self.temp_in_kelvin;
                        self.rows[idx].refresh_db_range(db, tk);
                    }
                });
                widget_cell(ui, 22.0, bg, false, |ui| {
                    if self.rows.len() > 1 && ui.small_button("×").clicked() {
                        remove_idx = Some(idx);
                    }
                });
            });
        }
        if let Some(ri) = remove_idx {
            self.rows.remove(ri);
            if self.active_row >= self.rows.len() {
                self.active_row = self.rows.len().saturating_sub(1);
            }
            self.recalculate(db);
        }
    }

    fn ui_ydata_grid(&self, ui: &mut egui::Ui) {
        let Some(res) = &self.result else {
            ui.label("Click Diagram to compute Y-data.");
            return;
        };
        let t_unit = if self.temp_in_kelvin { "K" } else { "°C" };
        let vis_series: Vec<&DiagramSeries> = res
            .series
            .iter()
            .enumerate()
            .filter(|(idx, s)| self.is_series_visible(s, *idx))
            .map(|(_, s)| s)
            .collect();
        if vis_series.is_empty() {
            ui.label("No visible species series.");
            return;
        }
        ui.horizontal(|ui| {
            row_num_cell(ui, 1, 28.0);
            header_cell(ui, &format!("T ({t_unit})"), 72.0);
            for s in &vis_series {
                header_cell(ui, &format!("{}\n({})", s.label, res.y_unit), 92.0);
            }
        });
        let n_pts = vis_series.first().map(|s| s.points.len()).unwrap_or(0);
        for pi in 0..n_pts {
            let t_disp =
                vis_series[0].points[pi].t_k - if self.temp_in_kelvin { 0.0 } else { 273.15 };
            let bg = if pi % 2 == 0 {
                egui::Color32::WHITE
            } else {
                EXCEL_ALT_ROW_BG
            };
            ui.horizontal(|ui| {
                row_num_cell(ui, pi + 2, 28.0);
                text_cell(
                    ui,
                    &format!("{t_disp:.1}"),
                    72.0,
                    EXCEL_READONLY_BG,
                    egui::Color32::BLACK,
                    true,
                    true,
                    false,
                );
                for s in &vis_series {
                    let val_s = s
                        .points
                        .get(pi)
                        .map(|p| format!("{:.2}", p.y_val))
                        .unwrap_or_default();
                    text_cell(
                        ui,
                        &val_s,
                        92.0,
                        bg,
                        egui::Color32::BLACK,
                        true,
                        true,
                        false,
                    );
                }
            });
        }
    }

    fn ui_chart(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let Some(res) = self.result.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label("Click Diagram to plot.");
            });
            return;
        };

        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(res.diagram_type.label())
                    .size(14.5)
                    .strong()
                    .color(egui::Color32::BLACK),
            );
        });

        let avail = ui.available_size();
        let plot_w = (avail.x - 12.0).max(240.0);
        let plot_h = (avail.y - 34.0).max(200.0);
        let (outer_rect, resp) =
            ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::hover());
        let painter = ui.painter_at(outer_rect);

        let margin_l = 68.0;
        let margin_r = 28.0;
        let margin_t = 10.0;
        let margin_b = 36.0;
        let plot_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(margin_l, margin_t),
            outer_rect.max - egui::vec2(margin_r, margin_b),
        );

        let t_min: f64 = self.t_min_str.trim().parse().unwrap_or(0.0);
        let t_max: f64 = self
            .t_max_str
            .trim()
            .parse::<f64>()
            .unwrap_or(2000.0)
            .max(t_min + 1.0);
        let y_min: f64 = self.y_min_str.trim().parse().unwrap_or(-1200.0);
        let y_max: f64 = self
            .y_max_str
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
            .max(y_min + 1.0);
        let off = if self.temp_in_kelvin { 0.0 } else { 273.15 };

        let x_to_px = |t_disp: f64| -> f32 {
            plot_rect.left() + (((t_disp - t_min) / (t_max - t_min)) as f32) * plot_rect.width()
        };
        let y_to_px = |y_val: f64| -> f32 {
            plot_rect.bottom() - (((y_val - y_min) / (y_max - y_min)) as f32) * plot_rect.height()
        };

        painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

        // Dotted gridlines with formatted ticks (supports sub-unit steps)
        let grid_col = egui::Color32::from_rgb(205, 210, 218);
        let x_step = nice_step((t_max - t_min) / 8.0);
        let mut gx = (t_min / x_step).ceil() * x_step;
        while gx <= t_max + 1e-6 {
            let x = x_to_px(gx);
            draw_dashed_line(
                &painter,
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
                2.0,
                3.0,
                egui::Stroke::new(0.8, grid_col),
            );
            painter.text(
                egui::pos2(x, plot_rect.bottom() + 12.0),
                egui::Align2::CENTER_CENTER,
                format_tick(gx),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            gx += x_step;
        }

        let y_step = nice_step((y_max - y_min) / 8.0);
        let mut gy = (y_min / y_step).ceil() * y_step;
        while gy <= y_max + 1e-6 {
            let y = y_to_px(gy);
            draw_dashed_line(
                &painter,
                egui::pos2(plot_rect.left(), y),
                egui::pos2(plot_rect.right(), y),
                2.0,
                3.0,
                egui::Stroke::new(0.8, grid_col),
            );
            painter.text(
                egui::pos2(plot_rect.left() - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                format_tick(gy),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            gy += y_step;
        }

        let clipped = painter.with_clip_rect(plot_rect);

        // Constant pO2 iso-lines on Ellingham diagrams (§19 Appendix 1 Eq. 11)
        let bal_atoms: f64 = self.balance_amount_str.trim().parse().unwrap_or(2.0);
        if res.diagram_type == DiagramType::DeltaGEllingham && self.show_po2_lines && !self.per_kg {
            let iso_col = egui::Color32::from_rgba_unmultiplied(120, 130, 150, 125);
            let iso_stroke = egui::Stroke::new(0.85, iso_col);
            for &log_po2 in &[-40.0, -30.0, -20.0, -15.0, -10.0, -5.0, 0.0] {
                let y0 = ellingham_po2_isoline_val(t_min + off, log_po2, bal_atoms, self.energy_unit);
                let y1 = ellingham_po2_isoline_val(t_max + off, log_po2, bal_atoms, self.energy_unit);
                draw_dashed_line(
                    &clipped,
                    egui::pos2(x_to_px(t_min), y_to_px(y0)),
                    egui::pos2(x_to_px(t_max), y_to_px(y1)),
                    3.0,
                    4.0,
                    iso_stroke,
                );
                let py1 = y_to_px(y1);
                if py1 > plot_rect.top() + 8.0 && py1 < plot_rect.bottom() - 8.0 {
                    clipped.text(
                        egui::pos2(plot_rect.right() - 4.0, py1 - 6.0),
                        egui::Align2::RIGHT_CENTER,
                        format!("1e{log_po2:.0}"),
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_rgb(100, 110, 130),
                    );
                }
            }
        }

        // Plot each visible series mapped cleanly by `row_index` (solid in DB range, dashed when extrapolated)
        for (idx, s) in res.series.iter().enumerate() {
            if !self.is_series_visible(s, idx) || s.points.len() < 2 {
                continue;
            }
            let color_idx = s.row_index.unwrap_or(idx);
            let (r, g, b) = CURVE_COLORS[color_idx % CURVE_COLORS.len()];
            let col = egui::Color32::from_rgb(r, g, b);
            let stroke = egui::Stroke::new(self.line_width, col);

            for win in s.points.windows(2) {
                let p0 = egui::pos2(x_to_px(win[0].t_k - off), y_to_px(win[0].y_val));
                let p1 = egui::pos2(x_to_px(win[1].t_k - off), y_to_px(win[1].y_val));
                if win[0].extrapolated || win[1].extrapolated {
                    draw_dashed_line(&clipped, p0, p1, 4.0, 3.0, stroke);
                } else {
                    clipped.line_segment([p0, p1], stroke);
                }
            }

            if self.show_curve_labels {
                let lbl_idx =
                    ((s.points.len() * (45 + (color_idx % 5) * 10)) / 100).min(s.points.len() - 1);
                let lp = s.points[lbl_idx];
                let lpos = egui::pos2(x_to_px(lp.t_k - off), y_to_px(lp.y_val) - 10.0);
                clipped.text(
                    lpos,
                    egui::Align2::CENTER_CENTER,
                    &s.label,
                    egui::FontId::proportional(11.5),
                    col,
                );
            }
        }

        painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.3, egui::Color32::BLACK));
        let x_lbl = if self.temp_in_kelvin {
            "Temperature (K)"
        } else {
            "Temperature (°C)"
        };
        painter.text(
            egui::pos2(plot_rect.center().x, plot_rect.bottom() + 25.0),
            egui::Align2::CENTER_CENTER,
            x_lbl,
            egui::FontId::proportional(12.0),
            egui::Color32::BLACK,
        );
        draw_rotated_y_title(
            &painter,
            outer_rect.left() + 12.0,
            plot_rect.center().y,
            &res.y_unit,
            egui::FontId::proportional(11.5),
            egui::Color32::BLACK,
        );

        // Hover inspector (includes live pO2, CO/CO2 & H2/H2O scales in Ellingham mode!)
        if let Some(pos) = resp.hover_pos() {
            if plot_rect.contains(pos) {
                let fx = (pos.x - plot_rect.left()) / plot_rect.width();
                let fy = (plot_rect.bottom() - pos.y) / plot_rect.height();
                let t_hov = t_min + fx as f64 * (t_max - t_min);
                let y_hov = y_min + fy as f64 * (y_max - y_min);
                let mut tip = format!("T: {t_hov:.1}\nY: {y_hov:.2} {}", res.y_unit);
                if res.diagram_type == DiagramType::DeltaGEllingham && !self.per_kg {
                    let sc = ellingham_scales_at(
                        (t_hov + off).max(1.0),
                        y_hov,
                        bal_atoms,
                        self.energy_unit,
                        db,
                    );
                    tip.push_str(&format!("\nlog10 p(O2): {:.2}", sc.log10_po2));
                    if let Some(c) = sc.log10_co_co2 {
                        tip.push_str(&format!("\nlog10 (CO/CO2): {:.2}", c));
                    }
                    if let Some(h) = sc.log10_h2_h2o {
                        tip.push_str(&format!("\nlog10 (H2/H2O): {:.2}", h));
                    }
                }
                resp.on_hover_text(tip);
            }
        }

        // Interactive Legend strip below chart (recomputes Auto Y-range immediately)
        let mut vis_changed = false;
        ui.horizontal_wrapped(|ui| {
            for (idx, s) in res.series.iter().enumerate() {
                let row_idx = s.row_index.unwrap_or(idx);
                if let Some(row) = self.rows.get_mut(row_idx) {
                    let (r, g, b) = CURVE_COLORS[row_idx % CURVE_COLORS.len()];
                    if ui
                        .checkbox(
                            &mut row.visible,
                            egui::RichText::new(&s.label)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(r, g, b)),
                        )
                        .changed()
                    {
                        vis_changed = true;
                    }
                }
            }
        });
        if vis_changed && self.y_auto {
            self.update_auto_y_range();
        }
    }

    fn ui_db_browser(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_db_browser;
        egui::Window::new("Browse Database — Add Species")
            .open(&mut open)
            .resizable(true)
            .default_size([420.0, 380.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(crate::i18n::text("common.141"));
                    if ui.text_edit_singleline(&mut self.db_query).changed() {
                        self.db_matches = db.search(&self.db_query, 50);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for sp in self.db_matches.clone() {
                        ui.horizontal(|ui| {
                            if ui.small_button("+ Add").clicked() {
                                let tk = self.temp_in_kelvin;
                                let row = DiagramRow::new(&sp.formula, 1.0, 0, db, tk);
                                if let Some(empty) =
                                    self.rows.iter_mut().find(|r| r.formula.trim().is_empty())
                                {
                                    *empty = row;
                                } else {
                                    self.rows.push(row);
                                }
                                self.recalculate(db);
                            }
                            ui.strong(&sp.formula);
                            ui.label(&sp.name);
                        });
                    }
                });
            });
        if !open {
            self.show_db_browser = false;
        }
    }

    fn ui_extra_modals(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        if self.show_file_modal {
            let mut open = self.show_file_modal;
            let title = if self.file_modal_is_save {
                "Save Thermochemical Diagram (.dia8)"
            } else {
                "Open Thermochemical Diagram (.dia8)"
            };
            let mut action = false;
            egui::Window::new(title).open(&mut open).default_width(420.0).show(ctx, |ui| {
                ui.label(crate::i18n::text("common.085"));
                ui.text_edit_singleline(&mut self.file_path_buf);
                ui.horizontal(|ui| {
                    let lbl = if self.file_modal_is_save { "Save" } else { "Open" };
                    if ui.button(lbl).clicked() {
                        action = true;
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        self.show_file_modal = false;
                    }
                });
            });
            if action {
                let p = self.file_path_buf.trim().to_string();
                if self.file_modal_is_save {
                    self.save_dia8(&p);
                } else {
                    self.open_dia8(db, &p);
                }
                self.show_file_modal = false;
            } else if !open {
                self.show_file_modal = false;
            }
        }

        if self.show_help_modal {
            let mut open = self.show_help_modal;
            egui::Window::new("H, S, C and G Diagrams Guide (HSC Ch. 19)")
                .open(&mut open)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.label("• Supports all 9 diagram types: H Total, H Heat Content, S, Cp, G, Delta H, Delta S, Delta G (Ellingham), and Ex Exergy (total).");
                    ui.label("• For Ellingham diagrams, pick the shared element (e.g. O = 2.00 for O2(g), S = 2.00 for S2(g)) and click 'Balance Element Amount'.");
                    ui.label("• Solid curves denote species inside their database interval [TMin, TMax]; dashed segments denote extrapolation.");
                    ui.label("• Use the DB column in X-data to select Auto (Own>Main), 1:OwnDB, or 2:MainDB.");
                    ui.label("• Hover over Ellingham charts to inspect live equilibrium log10 p(O2), CO/CO2, and H2/H2O scales (Ch. 19 Appendix 1).");
                });
            if !open {
                self.show_help_modal = false;
            }
        }
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_core::CAL_TO_JOULE;
    use soryntel_thermo::{KelleyCoefficients, Phase, SpeciesThermoInterval};
    use std::collections::BTreeSet;

    #[test]
    fn diag_elements_grid_has_no_duplicates_and_includes_actinides_metals() {
        let mut seen = BTreeSet::new();
        for &el in DIAG_ELEMENTS {
            if el.is_empty() {
                continue;
            }
            assert!(seen.insert(el), "duplicate element in DIAG_ELEMENTS: {el}");
        }
        for req in ["U", "V", "W", "Ti", "Tl", "O", "S", "Cl", "Fe", "Cu", "Mg"] {
            assert!(seen.contains(req), "missing element: {req}");
        }
    }

    #[test]
    fn row_index_tracking_handles_empty_rows_and_dia8_roundtrip() {
        let mut db = ThermoDatabase::with_open_defaults();
        for (f, h_kcal, s_cal) in [
            ("Fe", 0.0, 6.5),
            ("FeO", -63.5, 14.5),
            ("MgO", -143.8, 6.4),
        ] {
            db.insert_species(ChemicalSpecies {
                formula: f.to_string(),
                name: f.to_string(),
                phase: Phase::Solid,
                molecular_weight: 50.0,
                density_g_cm3: 2.0,
                charge: 0,
                intervals: vec![SpeciesThermoInterval {
                    t_min: 298.15,
                    t_max: 2500.0,
                    phase: Phase::Solid,
                    kelley: KelleyCoefficients { a: 45.0, b: 0.0, c: 0.0, d: 0.0 },
                    h_or_delta_h_tr: h_kcal * 1000.0 * CAL_TO_JOULE,
                    s_or_delta_s_tr: s_cal * CAL_TO_JOULE,
                    reference: "test".to_string(),
                }],
            });
        }

        let mut st = DiagramsModuleState::new(&db);
        // Leave row 1 empty; place MgO on row 2 hidden
        st.rows = vec![
            DiagramRow::new("FeO", 2.0, 0, &db, false),
            DiagramRow::empty(),
            DiagramRow::new("MgO", 2.0, 0, &db, false),
        ];
        st.rows[2].visible = false;
        st.recalculate(&db);

        let res = st.result.as_ref().unwrap();
        assert_eq!(res.series.len(), 2);
        assert!(st.is_series_visible(&res.series[0], 0));
        assert!(!st.is_series_visible(&res.series[1], 1));

        // Verify .dia8 roundtrip + ExExergy diagram type
        st.diagram_type = DiagramType::ExExergy;
        st.rows[2].visible = true;
        st.recalculate(&db);
        let tmp = format!("/tmp/diag_test_{}.dia8", std::process::id());
        st.save_dia8(&tmp);
        assert!(st.status_error.is_none());

        let mut st2 = DiagramsModuleState::new(&db);
        st2.open_dia8(&db, &tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(st2.status_error.is_none(), "{:?}", st2.status_error);
        assert_eq!(st2.diagram_type, DiagramType::ExExergy);
        assert_eq!(st2.result.as_ref().unwrap().series.len(), 2);
    }

    #[test]
    fn maindb6_ellingham_and_heat_content_presets_if_present() {
        let path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let mut db = ThermoDatabase::with_open_defaults();
        db.load_main_hsc(path).expect("Maindb6 opens");
        let st = DiagramsModuleState::new(&db);
        assert!(st.status_error.is_none(), "{:?}", st.status_error);
        let res = st.result.as_ref().unwrap();
        assert_eq!(res.series.len(), 9);
        // Fig. 4 check: at 1000 °C, 2.00 MgO is much lower (more negative ΔG) than 2.00 Cu2O
        let cu2o = res.series.iter().find(|s| s.formula == "Cu2O").unwrap();
        let mgo = res.series.iter().find(|s| s.formula == "MgO").unwrap();
        let idx_1000 = 20; // 0..2000 step 50 => index 20 is 1000 °C
        assert!(mgo.points[idx_1000].y_val < cu2o.points[idx_1000].y_val - 700.0);
    }
}
