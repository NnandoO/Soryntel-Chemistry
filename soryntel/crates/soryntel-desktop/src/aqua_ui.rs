//! Module 17 UI: HSC Chemistry 10 `Aqua` (Aqueous Electrolyte Solutions)
//! Matching Figs. 2–10 (`Water Phase` spreadsheet, Office Ribbon, `Main Pitzer Database`,
//! `Active Pitzer Parameters`, `OwnDB` editor, `Density Calculator`, and `Ionic AC vs T` Plot).

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use soryntel_application::SoryntelApplication;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::aqua::{
    ActiveParameterSetsReport, AquaCalcResult, AquaCaseFile, AquaEnergyUnit, AquaMethod,
    AquaPressUnit, AquaSpeciesInput, AquaTempUnit, DensityCalcResult, DensityInputRow,
    MergedPitzerCatalog, OwnBinaryRow, OwnLambdaRow, OwnPitzerDb, OwnPsiRow, OwnThetaRow,
    PitzerSourceColor,
};
use soryntel_thermo::aqua_density_data::DENSITY_SALTS;
use soryntel_thermo::aqua_pitzer_data::{PitzerDatabase, PitzerTempCurve};
use std::collections::BTreeSet;

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, CELL_H, EXCEL_ACTIVE_BORDER, EXCEL_GRID_BORDER,
    EXCEL_HEADER_BG,
};

// Authentic HSC Aqua & Classic WinForms colors (Figs. 2–9 & classic Aqua window)
const HSC_BANNER_BG: egui::Color32 = egui::Color32::from_rgb(191, 191, 191);
const HSC_BEIGE_BG: egui::Color32 = egui::Color32::from_rgb(222, 218, 198);
const HSC_WIN_BG: egui::Color32 = egui::Color32::from_rgb(240, 240, 240);
const HSC_GROUP_BORDER: egui::Color32 = egui::Color32::from_rgb(208, 208, 208);
const HSC_RULE_DARK: egui::Color32 = egui::Color32::from_rgb(35, 37, 42);
const HSC_SHEET_GRID: egui::Color32 = egui::Color32::from_rgb(176, 176, 166);
const HSC_RIBBON_TOP: egui::Color32 = egui::Color32::from_rgb(245, 247, 250);
const HSC_RIBBON_BOT: egui::Color32 = egui::Color32::from_rgb(228, 233, 242);
const HSC_RIBBON_BORDER: egui::Color32 = egui::Color32::from_rgb(196, 205, 218);
const HSC_SEL_COL_BG: egui::Color32 = egui::Color32::from_rgb(207, 226, 250);
const HSC_OWNDB_ACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(213, 228, 247);
const HSC_OWNDB_ACTIVE_BORDER: egui::Color32 = egui::Color32::from_rgb(112, 154, 204);
const HSC_BRAND_RED: egui::Color32 = egui::Color32::from_rgb(179, 28, 36);
const HSC_DARK_TEXT: egui::Color32 = egui::Color32::from_rgb(22, 24, 29);
const HSC_MUTED_TEXT: egui::Color32 = egui::Color32::from_rgb(95, 105, 120);
const HSC_VIOLATION_BG: egui::Color32 = egui::Color32::from_rgb(254, 226, 226);

#[derive(Clone, Copy, PartialEq, Eq)]
enum RibbonIcon {
    File,
    Cut,
    Copy,
    Paste,
    Find,
    Calculate,
    Gears,
    UnitsGrid,
    UseOwnDb,
    MainDb,
    ParamSets,
    OwnDb,
    DensityFx,
    PlotCurve,
    Help,
    About,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PitzerSheetTab {
    Binary,
    Theta,
    Psi,
    Lambda,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParamSetsTab {
    Active,
    Missing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MainDbViewMode {
    StatusFlags, // Fig. 5
    DetailedX,   // Fig. 9
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CellCoord {
    row: usize, // 1-based Excel row index (e.g. 4, 7..)
    col: usize, // 0-based column index (0 = A)
}

#[derive(Clone)]
struct AquaSpeciesRowBuf {
    formula: String,
    temp_buf: String,
    amount_buf: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellAlign {
    Left,
    Center,
    Right,
}

pub struct AquaModuleState {
    case_file: AquaCaseFile,
    rows: Vec<AquaSpeciesRowBuf>,
    phase_temp_buf: String,
    main_db: PitzerDatabase,
    own_db: OwnPitzerDb,
    excluded_keys: BTreeSet<String>,
    calc_result: AquaCalcResult,
    merged_catalog: MergedPitzerCatalog,
    param_report: ActiveParameterSetsReport,

    use_ribbon_toolbar: bool,
    selected_cell: CellCoord,
    formula_bar_buf: String,
    clipboard_val: String,
    clipboard_row: Option<AquaSpeciesRowBuf>,
    dirty_since_calc: bool,
    status_msg: String,

    // Find & Replace modal
    show_find: bool,
    find_query: String,
    replace_query: String,

    // Sub-windows matching Figs. 5–10
    show_main_db: bool,
    main_db_tab: PitzerSheetTab,
    main_db_mode: MainDbViewMode,

    show_param_sets: bool,
    param_sets_tab: ParamSetsTab,

    show_own_db: bool,
    own_db_tab: PitzerSheetTab,
    own_db_selected_row: usize,
    own_db_json_path: Option<std::path::PathBuf>,

    show_density: bool,
    density_selected_salt_idx: Option<usize>,
    density_selected_entry_idx: Option<usize>,
    density_temp_buf: String,
    density_rows: Vec<(String, String)>,
    density_result: DensityCalcResult,

    show_plot: bool,
    show_help: bool,
    show_about: bool,
}

impl AquaModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let case_file = AquaCaseFile::preset_na_s_h2o_fig3();
        let main_db = SoryntelApplication::aqua_main_pitzer_db();
        let own_db = default_sample_own_db();
        let excluded_keys = BTreeSet::new();

        let rows = rows_from_case(&case_file);
        let phase_temp_buf = format_temp_display(case_file.species.first().map(|s| s.temp).unwrap_or(40.0));
        let calc_result =
            SoryntelApplication::aqua_calculate(db, &case_file, &main_db, &own_db, &excluded_keys);
        let merged_catalog = SoryntelApplication::aqua_resolve_catalog(
            &main_db,
            &own_db,
            &excluded_keys,
            case_file.use_own_db,
        );
        let param_report =
            SoryntelApplication::aqua_analyze_params(&case_file, &main_db, &own_db, &excluded_keys);

        // Fig. 7 default: ZnSO4 (0.149) + H2SO4 (0.048) @ 25 °C -> 1204.371594 kg/m3
        let density_rows = vec![
            ("ZnSO4".to_string(), "0.149".to_string()),
            ("H2SO4".to_string(), "0.048".to_string()),
        ];
        let density_result = recompute_density(&density_rows, 25.0);

        let mut st = Self {
            case_file,
            rows,
            phase_temp_buf,
            main_db,
            own_db,
            excluded_keys,
            calc_result,
            merged_catalog,
            param_report,

            use_ribbon_toolbar: false,
            selected_cell: CellCoord { row: 7, col: 0 },
            formula_bar_buf: "H2O".to_string(),
            clipboard_val: String::new(),
            clipboard_row: None,
            dirty_since_calc: false,
            status_msg: "Ready · Preset: Na–S–H2O (40 °C, Fig. 3/4)".to_string(),

            show_find: false,
            find_query: String::new(),
            replace_query: String::new(),

            show_main_db: false,
            main_db_tab: PitzerSheetTab::Binary,
            main_db_mode: MainDbViewMode::DetailedX,

            show_param_sets: false,
            param_sets_tab: ParamSetsTab::Active,

            show_own_db: false,
            own_db_tab: PitzerSheetTab::Binary,
            own_db_selected_row: 0,
            own_db_json_path: None,

            show_density: false,
            density_selected_salt_idx: Some(0),
            density_selected_entry_idx: Some(0),
            density_temp_buf: "25".to_string(),
            density_rows,
            density_result,

            show_plot: false,
            show_help: false,
            show_about: false,
        };
        st.sync_formula_bar_from_selection();
        st
    }

    fn sync_case_from_rows(&mut self) {
        self.case_file.species = self
            .rows
            .iter()
            .filter(|r| !r.formula.trim().is_empty())
            .map(|r| AquaSpeciesInput {
                formula: r.formula.trim().to_string(),
                temp: r.temp_buf.trim().parse::<f64>().unwrap_or(25.0),
                amount_kmol: r.amount_buf.trim().parse::<f64>().unwrap_or(0.0),
            })
            .collect();
    }

    fn recalculate(&mut self, db: &ThermoDatabase) {
        self.sync_case_from_rows();
        self.calc_result = SoryntelApplication::aqua_calculate(
            db,
            &self.case_file,
            &self.main_db,
            &self.own_db,
            &self.excluded_keys,
        );
        self.merged_catalog = SoryntelApplication::aqua_resolve_catalog(
            &self.main_db,
            &self.own_db,
            &self.excluded_keys,
            self.case_file.use_own_db,
        );
        self.param_report = SoryntelApplication::aqua_analyze_params(
            &self.case_file,
            &self.main_db,
            &self.own_db,
            &self.excluded_keys,
        );
        self.dirty_since_calc = false;
        self.sync_formula_bar_from_selection();
        self.status_msg = format!(
            "Calculated ({:?}) · I = {:.2} mol/kg · pH = {}",
            self.case_file.method,
            self.calc_result.summary.ionic_strength,
            self.calc_result
                .summary
                .ph
                .map(|p| format!("{p:.2}"))
                .unwrap_or_else(|| "—".into())
        );
    }

    fn load_preset(&mut self, preset: AquaCaseFile, name: &str, db: &ThermoDatabase) {
        self.case_file = preset;
        self.rows = rows_from_case(&self.case_file);
        self.phase_temp_buf =
            format_temp_display(self.case_file.species.first().map(|s| s.temp).unwrap_or(25.0));
        self.selected_cell = CellCoord { row: 7, col: 0 };
        self.recalculate(db);
        self.status_msg = format!("Loaded preset: {name}");
    }

    fn switch_temp_unit(&mut self, new_unit: AquaTempUnit, db: &ThermoDatabase) {
        let old = self.case_file.temp_unit;
        if old == new_unit {
            return;
        }
        for r in &mut self.rows {
            if let Ok(v) = r.temp_buf.trim().parse::<f64>() {
                let tc = old.to_c(v);
                r.temp_buf = format_temp_display(new_unit.from_c(tc));
            }
        }
        if let Ok(v) = self.phase_temp_buf.trim().parse::<f64>() {
            let tc = old.to_c(v);
            self.phase_temp_buf = format_temp_display(new_unit.from_c(tc));
        }
        self.case_file.temp_unit = new_unit;
        self.recalculate(db);
    }

    fn select_cell(&mut self, row: usize, col: usize) {
        self.selected_cell = CellCoord { row, col };
        self.sync_formula_bar_from_selection();
    }

    fn sync_formula_bar_from_selection(&mut self) {
        self.formula_bar_buf = self.cell_raw_string(self.selected_cell.row, self.selected_cell.col);
    }

    fn cell_raw_string(&self, row: usize, col: usize) -> String {
        if row == 1 && col == 0 {
            return "Water Phase Data".into();
        }
        if row == 4 {
            return match col {
                0 => "Water Phase".into(),
                1 => self.phase_temp_buf.clone(),
                _ => self.phase_row_col_text(col),
            };
        }
        if row >= 7 {
            let ridx = row - 7;
            if let Some(r) = self.rows.get(ridx) {
                return match col {
                    0 => r.formula.clone(),
                    1 => r.temp_buf.clone(),
                    2 => r.amount_buf.clone(),
                    _ => self.species_row_col_text(ridx, col),
                };
            }
        }
        String::new()
    }

    fn set_species_cell(&mut self, ridx: usize, col: usize, new_val: String) {
        if ridx >= self.rows.len() && new_val.is_empty() {
            return;
        }
        while self.rows.len() <= ridx {
            self.rows.push(AquaSpeciesRowBuf {
                formula: String::new(),
                temp_buf: self.phase_temp_buf.clone(),
                amount_buf: "0.0".into(),
            });
        }
        let target = &mut self.rows[ridx];
        match col {
            0 => target.formula = new_val,
            1 => target.temp_buf = new_val,
            2 => target.amount_buf = new_val,
            _ => {}
        }
        self.dirty_since_calc = true;
    }

    fn clear_sheet(&mut self) {
        self.rows.clear();
        self.case_file.species.clear();
        self.calc_result.species.clear();
        self.calc_result.element_columns.clear();
        self.calc_result.warnings.clear();
        self.selected_cell = CellCoord { row: 7, col: 0 };
        self.formula_bar_buf.clear();
        self.dirty_since_calc = false;
        self.status_msg = "Sheet cleared. Enter species in column A and click Calculate.".into();
    }

    fn add_species_row(&mut self) {
        self.rows.push(AquaSpeciesRowBuf {
            formula: String::new(),
            temp_buf: self.phase_temp_buf.clone(),
            amount_buf: "0.0".into(),
        });
        let new_row = self.rows.len() + 6;
        self.select_cell(new_row, 0);
    }

    fn delete_selected_row(&mut self, db: &ThermoDatabase) {
        if self.selected_cell.row < 7 {
            return;
        }
        let ridx = self.selected_cell.row - 7;
        if ridx < self.rows.len() && self.rows.len() > 1 {
            self.rows.remove(ridx);
            self.selected_cell.row = self.selected_cell.row.min(self.rows.len() + 6);
            self.recalculate(db);
        }
    }

    fn cut_selected_cell(&mut self, db: &ThermoDatabase) {
        self.clipboard_val = self.cell_raw_string(self.selected_cell.row, self.selected_cell.col);
        if self.selected_cell.row >= 7 {
            let ridx = self.selected_cell.row - 7;
            self.clipboard_row = self.rows.get(ridx).cloned();
        }
        self.apply_formula_bar_edit(String::new(), db);
        self.sync_formula_bar_from_selection();
    }

    fn copy_selected_cell(&mut self, ctx: &egui::Context) {
        self.clipboard_val = self.cell_raw_string(self.selected_cell.row, self.selected_cell.col);
        if self.selected_cell.row >= 7 {
            self.clipboard_row = self.rows.get(self.selected_cell.row - 7).cloned();
        }
        ctx.copy_text(self.clipboard_val.clone());
        self.status_msg = format!("Copied '{}'", self.clipboard_val);
    }

    fn paste_selected_cell(&mut self, db: &ThermoDatabase) {
        if !self.clipboard_val.is_empty() {
            self.apply_formula_bar_edit(self.clipboard_val.clone(), db);
            self.sync_formula_bar_from_selection();
        }
    }

    fn save_case_quick(&mut self) {
        self.sync_case_from_rows();
        let p = std::env::temp_dir().join("soryntel_aqua_case.aqu8.json");
        if let Ok(json) = serde_json::to_string_pretty(&self.case_file) {
            if crate::storage::write_atomic(&p, json).is_ok() {
                self.status_msg = format!("Saved case: {}", p.display());
            }
        }
    }

    fn load_case_quick(&mut self, db: &ThermoDatabase) {
        let p = std::env::temp_dir().join("soryntel_aqua_case.aqu8.json");
        if let Ok(txt) = std::fs::read_to_string(&p) {
            if let Ok(loaded) = serde_json::from_str::<AquaCaseFile>(&txt) {
                self.load_preset(loaded, &p.display().to_string(), db);
            }
        } else {
            self.status_msg = "No saved case found yet.".into();
        }
    }

    fn render_file_menu_items(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) -> bool {
        let mut close = false;
        ui.label(egui::RichText::new("HSC Presets").strong().size(11.0));
        if ui.button("Preset: Na–S–H2O (40 °C, Fig. 3/4)").clicked() {
            self.load_preset(AquaCaseFile::preset_na_s_h2o_fig3(), "Na–S–H2O (Fig. 3/4)", db);
            close = true;
        }
        if ui.button("Preset: Millero Seawater (S=35, Fig. 10)").clicked() {
            self.load_preset(AquaCaseFile::preset_millero_seawater(), "Millero Seawater (Fig. 10)", db);
            close = true;
        }
        if ui.button("Preset: NaCl–HCl–H2O (25 °C)").clicked() {
            self.load_preset(AquaCaseFile::preset_nacl_hcl(), "NaCl–HCl–H2O", db);
            close = true;
        }
        ui.separator();
        if ui.button("New Blank Water Phase").clicked() {
            self.rows = vec![AquaSpeciesRowBuf {
                formula: "H2O".into(),
                temp_buf: "25".into(),
                amount_buf: "55.50844".into(),
            }];
            self.phase_temp_buf = "25".into();
            self.recalculate(db);
            close = true;
        }
        if ui.button("Save Case (*.aqu8.json)...").clicked() {
            self.save_case_quick();
            close = true;
        }
        if ui.button("Load Last Saved Case").clicked() {
            self.load_case_quick(db);
            close = true;
        }
        ui.separator();
        if ui.button("Plot Ionic AC vs T (Fig. 10)").clicked() {
            self.show_plot = true;
            close = true;
        }
        close
    }

    fn render_units_menu_items(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) -> bool {
        let mut close = false;
        ui.label(egui::RichText::new(crate::i18n::text("common.005")).strong().size(11.0));
        for (tu, l) in [
            (AquaTempUnit::Celsius, "°C (Celsius)"),
            (AquaTempUnit::Kelvin, "K (Kelvin)"),
        ] {
            if ui.selectable_label(self.case_file.temp_unit == tu, l).clicked() {
                self.switch_temp_unit(tu, db);
                close = true;
            }
        }
        ui.separator();
        ui.label(egui::RichText::new("Energy / Entropy / Cp").strong().size(11.0));
        for (eu, l) in [
            (AquaEnergyUnit::Kcal, "kcal, cal/(mol*K)"),
            (AquaEnergyUnit::KJ, "kJ, J/(mol*K)"),
        ] {
            if ui.selectable_label(self.case_file.energy_unit == eu, l).clicked() {
                self.case_file.energy_unit = eu;
                self.recalculate(db);
                close = true;
            }
        }
        ui.separator();
        ui.label(egui::RichText::new(crate::i18n::text("common.106")).strong().size(11.0));
        for (pu, l) in [
            (AquaPressUnit::Pa, "Pa"),
            (AquaPressUnit::KPa, "kPa"),
            (AquaPressUnit::Bar, "bar"),
            (AquaPressUnit::Atm, "atm"),
        ] {
            if ui.selectable_label(self.case_file.press_unit == pu, l).clicked() {
                self.case_file.press_unit = pu;
                self.recalculate(db);
                close = true;
            }
        }
        close
    }

    fn apply_formula_bar_edit(&mut self, new_val: String, db: &ThermoDatabase) {
        let CellCoord { row, col } = self.selected_cell;
        if row == 4 && col == 1 {
            self.phase_temp_buf = new_val.clone();
            for r in &mut self.rows {
                r.temp_buf = new_val.clone();
            }
            self.recalculate(db);
            return;
        }
        if row >= 7 && col <= 2 {
            self.set_species_cell(row - 7, col, new_val);
        }
    }

    fn phase_row_col_text(&self, col: usize) -> String {
        if self.calc_result.species.is_empty() {
            return String::new();
        }
        let s = &self.calc_result.summary;
        match col {
            2 => format!("{:.2}", s.total_kmol),
            3 => format!("{:.2}", s.total_mol_pct),
            4 => format_sci_or_dec(s.total_g, 2),
            5 => format!("{:.2}", s.h_ideal_mol),
            6 => format!("{:.2}", s.h_est_mol),
            7 => format!("{:.2}", s.cp_ideal_mol),
            8 => format!("{:.2}", s.cp_est_mol),
            9 => format!("{:.2}", s.h_ideal_kg),
            10 => format!("{:.2}", s.h_est_kg),
            11 => format!("{:.2}", s.cp_ideal_kg),
            12 => format!("{:.2}", s.cp_est_kg),
            13 => format!("{:.2}", s.osmotic_coeff),
            14 => format!("{:.2}", s.h2o_vapour_press),
            15 => format!("{:.2}", s.sol_vapour_press),
            16 => format!("{:.2}", s.ionic_strength),
            17 => s.ph.map(|v| format!("{v:.2}")).unwrap_or_else(|| "—".into()),
            18 => format!("{:.2}", s.rel_humidity_pct),
            19 => format!("{:.2}", s.fp_c),
            20 => format!("{:.2}", s.bp_c),
            21 => format!("{:.2}", s.fp_aw25_c),
            22 => format!("{:.2}", s.bp_aw25_c),
            23 => format!("{:.2}", s.osmotic_pressure),
            _ => String::new(),
        }
    }

    fn species_row_col_text(&self, ridx: usize, col: usize) -> String {
        let Some(sp) = self.calc_result.species.get(ridx) else {
            return String::new();
        };
        match col {
            3 => format!("{:.2}", sp.mol_pct),
            4 => format_sci_or_dec(sp.amount_g, 2),
            5 => format!("{:.2}", sp.h_ideal_mol),
            6 => format!("{:.2}", sp.h_est_mol),
            7 => format!("{:.2}", sp.cp_ideal_mol),
            8 => format!("{:.2}", sp.cp_est_mol),
            9 => format!("{:.2}", sp.ac_molar),
            10 => format!("{:.2}", sp.ac_molal),
            11 => format_hsc_sci(sp.molality),
            12 => format!("{:.2}", sp.mw_g_mol),
            c if c >= 13 => {
                let eidx = c - 13;
                if let Some(el) = self.calc_result.element_columns.get(eidx) {
                    if let Some(&stoich) = sp.elements.get(el) {
                        if stoich.abs() > 1e-9 {
                            return format!("{stoich:.2}");
                        }
                    }
                }
                String::new()
            }
            _ => String::new(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        if self.use_ribbon_toolbar {
            self.render_ribbon(ctx, db);
        } else {
            self.render_classic_menu_and_aqua_tools(ctx, db);
        }
        self.render_formula_bar(ctx, db);
        self.render_footer_and_tabs(ctx);
        self.render_water_phase_sheet(ctx, db);

        if self.show_find {
            self.render_find_modal(ctx, db);
        }
        if self.show_main_db {
            self.render_main_db_window(ctx, db);
        }
        if self.show_param_sets {
            self.render_param_sets_window(ctx);
        }
        if self.show_own_db {
            self.render_own_db_window(ctx, db);
        }
        if self.show_density {
            self.render_density_window(ctx);
        }
        if self.show_plot {
            self.render_plot_window(ctx, db);
        }
        if self.show_help {
            self.render_help_modal(ctx);
        }
        if self.show_about {
            self.render_about_modal(ctx);
        }
    }

    fn render_classic_menu_and_aqua_tools(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        egui::TopBottomPanel::top("aqua_classic_menubar")
            .frame(
                egui::Frame::none()
                    .fill(HSC_WIN_BG)
                    .stroke(egui::Stroke::new(0.8, egui::Color32::from_rgb(225, 225, 225)))
                    .inner_margin(egui::Margin::symmetric(4.0, 1.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(egui::RichText::new(crate::i18n::text("common.001")).size(11.5).color(HSC_DARK_TEXT), |ui| {
                        if self.render_file_menu_items(ui, db) {
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(egui::RichText::new(crate::i18n::text("common.006")).size(11.5).color(HSC_DARK_TEXT), |ui| {
                        if ui.button(crate::i18n::text("common.075")).clicked() {
                            self.cut_selected_cell(db);
                            ui.close_menu();
                        }
                        if ui.button(crate::i18n::text("common.029")).clicked() {
                            self.copy_selected_cell(ctx);
                            ui.close_menu();
                        }
                        if ui.button(crate::i18n::text("common.076")).clicked() {
                            self.paste_selected_cell(db);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find / Replace...").clicked() {
                            self.show_find = !self.show_find;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("+ Add Species Row").clicked() {
                            self.add_species_row();
                            ui.close_menu();
                        }
                        if ui.button("Delete Selected Row").clicked() {
                            self.delete_selected_row(db);
                            ui.close_menu();
                        }
                        if ui.button(crate::i18n::text("common.093")).clicked() {
                            self.clear_sheet();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(egui::RichText::new("Database").size(11.5).color(HSC_DARK_TEXT), |ui| {
                        if ui
                            .checkbox(&mut self.case_file.use_own_db, "Use Own Pitzer Database (OwnDB)")
                            .changed()
                        {
                            self.recalculate(db);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Main Pitzer Database...").clicked() {
                            self.show_main_db = true;
                            ui.close_menu();
                        }
                        if ui.button("Parameter Sets (Active / Missing)...").clicked() {
                            self.show_param_sets = true;
                            ui.close_menu();
                        }
                        if ui.button("Own Pitzer Database Editor...").clicked() {
                            self.show_own_db = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(egui::RichText::new(crate::i18n::text("common.018")).size(11.5).color(HSC_DARK_TEXT), |ui| {
                        ui.menu_button("Units...", |ui| {
                            if self.render_units_menu_items(ui, db) {
                                ui.close_menu();
                            }
                        });
                        if ui.button("Density Calculator...").clicked() {
                            self.show_density = true;
                            ui.close_menu();
                        }
                        if ui.button("Plot Ionic AC vs T...").clicked() {
                            self.show_plot = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .checkbox(&mut self.use_ribbon_toolbar, "Show HSC 10 Ribbon Toolbar")
                            .changed()
                        {
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(egui::RichText::new(crate::i18n::text("common.003")).size(11.5).color(HSC_DARK_TEXT), |ui| {
                        if ui.button("Aqua Module Help...").clicked() {
                            self.show_help = true;
                            ui.close_menu();
                        }
                        if ui.button("About Soryntel Aqua...").clicked() {
                            self.show_about = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        egui::TopBottomPanel::top("aqua_classic_tools")
            .frame(
                egui::Frame::none()
                    .fill(HSC_WIN_BG)
                    .inner_margin(egui::Margin {
                        left: 8.0,
                        right: 8.0,
                        top: 4.0,
                        bottom: 5.0,
                    }),
            )
            .show(ctx, |ui| {
                draw_win_groupbox(ui, "Aqua tools", 72.0, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 11.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 11.0;
                            ui.label(
                                egui::RichText::new("Chosen method:")
                                    .size(11.5)
                                    .color(HSC_DARK_TEXT),
                            );
                            for (m, lbl) in [
                                (AquaMethod::Davies, "Davies"),
                                (AquaMethod::Pitzer, "Pitzer"),
                                (AquaMethod::Harvie, "Harvie"),
                            ] {
                                if draw_win_square_check(ui, lbl, self.case_file.method == m).clicked() {
                                    self.case_file.method = m;
                                    self.recalculate(db);
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            if draw_classic_bevel_button(ui, "Calculate", 114.0, 24.0).clicked() {
                                self.recalculate(db);
                            }
                            if draw_classic_bevel_button(ui, "Clear Sheet", 114.0, 24.0).clicked() {
                                self.clear_sheet();
                            }
                        });
                    });
                });
            });
    }

    fn render_ribbon(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        egui::TopBottomPanel::top("aqua_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(HSC_RIBBON_TOP)
                    .stroke(egui::Stroke::new(1.0, HSC_RIBBON_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                // Soft vertical gradient backdrop across ribbon
                let r_rect = ui.max_rect().expand2(egui::vec2(6.0, 4.0));
                let p = ui.painter();
                p.rect_filled(r_rect, 0.0, HSC_RIBBON_TOP);
                let bot = egui::Rect::from_min_max(
                    egui::pos2(r_rect.min.x, r_rect.center().y),
                    r_rect.max,
                );
                p.rect_filled(bot, 0.0, HSC_RIBBON_BOT);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    // Group 1: Main (File ▼)
                    ribbon_group(ui, "Main", |ui| {
                        let f_resp = large_ribbon_btn(ui, RibbonIcon::File, "File", Some("▼"), false)
                            .on_hover_text("Load HSC presets, new sheet, save/load case, or open Plot");
                        let f_pop_id = ui.make_persistent_id("aqua_file_popup");
                        if f_resp.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(f_pop_id));
                        }
                        egui::popup_below_widget(
                            ui,
                            f_pop_id,
                            &f_resp,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(245.0);
                                if self.render_file_menu_items(ui, db) {
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            },
                        );
                    });

                    // Group 2: Edit (2x2 Cut/Paste, Copy/Find)
                    ribbon_group(ui, "Edit", |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                            ui.horizontal(|ui| {
                                if small_ribbon_btn(ui, RibbonIcon::Cut, "Cut")
                                    .on_hover_text("Cut selected cell value")
                                    .clicked()
                                {
                                    self.cut_selected_cell(db);
                                }
                                if small_ribbon_btn(ui, RibbonIcon::Paste, "Paste")
                                    .on_hover_text("Paste cell or row")
                                    .clicked()
                                {
                                    self.paste_selected_cell(db);
                                }
                            });
                            ui.horizontal(|ui| {
                                if small_ribbon_btn(ui, RibbonIcon::Copy, "Copy")
                                    .on_hover_text("Copy selected cell value")
                                    .clicked()
                                {
                                    self.copy_selected_cell(ctx);
                                }
                                if small_ribbon_btn(ui, RibbonIcon::Find, "Find/Replace")
                                    .on_hover_text("Find and replace species or values")
                                    .clicked()
                                {
                                    self.show_find = !self.show_find;
                                }
                            });
                        });
                    });

                    // Group 3: Calculation Options
                    ribbon_group(ui, "Calculation Options", |ui| {
                        if large_ribbon_btn(ui, RibbonIcon::Calculate, "Calculate", None, false)
                            .on_hover_text("Calculate Pitzer / Harvie / Davies properties")
                            .clicked()
                        {
                            self.recalculate(db);
                        }

                        // Aqua method dropdown
                        let method_lbl = match self.case_file.method {
                            AquaMethod::Pitzer => "Pitzer",
                            AquaMethod::Harvie => "Harvie",
                            AquaMethod::Davies => "Davies",
                        };
                        let m_resp =
                            large_ribbon_btn(ui, RibbonIcon::Gears, "Aqua method", Some("▼"), false)
                                .on_hover_text(format!("Active method: {method_lbl}"));
                        let m_pop_id = ui.make_persistent_id("aqua_method_popup");
                        if m_resp.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(m_pop_id));
                        }
                        egui::popup_below_widget(
                            ui,
                            m_pop_id,
                            &m_resp,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(280.0);
                                for (m, lbl, desc) in [
                                    (AquaMethod::Pitzer, "Pitzer", "Pure Pitzer model (1973–1991)"),
                                    (AquaMethod::Harvie, "Harvie (Default)", "Harvie–Møller–Weare + unsymmetric mixing"),
                                    (AquaMethod::Davies, "Davies", "Empirical I ≤ 0.5 mol/kg fallback"),
                                ] {
                                    if ui
                                        .selectable_label(self.case_file.method == m, format!("{lbl} — {desc}"))
                                        .clicked()
                                    {
                                        self.case_file.method = m;
                                        self.recalculate(db);
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                }
                            },
                        );

                        // Units dropdown
                        let u_resp =
                            large_ribbon_btn(ui, RibbonIcon::UnitsGrid, "Units", Some("▼"), false)
                                .on_hover_text("Select temperature, energy & pressure units");
                        let u_pop_id = ui.make_persistent_id("aqua_units_popup");
                        if u_resp.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(u_pop_id));
                        }
                        egui::popup_below_widget(
                            ui,
                            u_pop_id,
                            &u_resp,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(180.0);
                                if self.render_units_menu_items(ui, db) {
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            },
                        );

                        // Use OwnDB toggle (Fig. 8)
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::UseOwnDb,
                            "Use OwnDB",
                            None,
                            self.case_file.use_own_db,
                        )
                        .on_hover_text("Give Own Pitzer Database precedence over Main Database (Fig. 8)")
                        .clicked()
                        {
                            self.case_file.use_own_db = !self.case_file.use_own_db;
                            self.recalculate(db);
                        }
                    });

                    // Group 4: Pitzer Database
                    ribbon_group(ui, "Pitzer Database", |ui| {
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::MainDb,
                            "Main Database",
                            None,
                            self.show_main_db,
                        )
                        .on_hover_text("Open Main Pitzer Database parameter catalog (Figs. 5 & 9)")
                        .clicked()
                        {
                            self.show_main_db = !self.show_main_db;
                        }
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::ParamSets,
                            "Parameter\nSets",
                            None,
                            self.show_param_sets,
                        )
                        .on_hover_text("Inspect Active & Missing Pitzer parameters for active solution (Fig. 6)")
                        .clicked()
                        {
                            self.show_param_sets = !self.show_param_sets;
                        }
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::OwnDb,
                            "Own Database",
                            None,
                            self.show_own_db,
                        )
                        .on_hover_text("Open editable Own Pitzer Database (Fig. 8)")
                        .clicked()
                        {
                            self.show_own_db = !self.show_own_db;
                        }
                    });

                    // Group 5: Tools
                    ribbon_group(ui, "Tools", |ui| {
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::DensityFx,
                            "Density\nCalculator",
                            None,
                            self.show_density,
                        )
                        .on_hover_text("Laliberté–Cooper (2004) 59-salt aqueous solution density calculator (Fig. 7)")
                        .clicked()
                        {
                            self.show_density = !self.show_density;
                        }
                        if large_ribbon_btn(
                            ui,
                            RibbonIcon::PlotCurve,
                            "AC Plot\nT-Dep",
                            None,
                            self.show_plot,
                        )
                        .on_hover_text("Plot Ionic Activity Coefficient vs Temperature (Fig. 10)")
                        .clicked()
                        {
                            self.show_plot = !self.show_plot;
                        }
                    });

                    // Group 6: Info & Classic Toggle
                    ribbon_group(ui, "Info", |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            if small_ribbon_btn(ui, RibbonIcon::Help, "Help").clicked() {
                                self.show_help = !self.show_help;
                            }
                            if small_ribbon_btn(ui, RibbonIcon::About, "About").clicked() {
                                self.show_about = !self.show_about;
                            }
                        });
                        if large_ribbon_btn(ui, RibbonIcon::Gears, "Classic\nView", None, false)
                            .on_hover_text("Switch back to classic Aqua tools view")
                            .clicked()
                        {
                            self.use_ribbon_toolbar = false;
                        }
                    });
                });
            });
    }

    fn render_formula_bar(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        egui::TopBottomPanel::top("aqua_formula_bar")
            .frame(
                egui::Frame::none()
                    .fill(HSC_WIN_BG)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(212, 212, 212)))
                    .inner_margin(egui::Margin::symmetric(5.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let coord_lbl = format!(
                        "{}{}",
                        col_letter(self.selected_cell.col),
                        self.selected_cell.row
                    );

                    // Classic white WinForms Name Box with right ▼ chevron
                    let (nb_rect, nb_resp) =
                        ui.allocate_exact_size(egui::vec2(138.0, 21.0), egui::Sense::click());
                    if ui.is_rect_visible(nb_rect) {
                        let p = ui.painter();
                        p.rect_filled(nb_rect, 0.0, egui::Color32::WHITE);
                        p.rect_stroke(
                            nb_rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(172, 173, 179)),
                        );
                        p.text(
                            nb_rect.left_center() + egui::vec2(6.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &coord_lbl,
                            egui::FontId::proportional(11.5),
                            HSC_DARK_TEXT,
                        );
                        let cx = nb_rect.max.x - 10.0;
                        let cy = nb_rect.center().y;
                        p.add(egui::Shape::convex_polygon(
                            vec![
                                egui::pos2(cx - 3.5, cy - 2.0),
                                egui::pos2(cx + 3.5, cy - 2.0),
                                egui::pos2(cx, cy + 2.5),
                            ],
                            HSC_DARK_TEXT,
                            egui::Stroke::NONE,
                        ));
                    }
                    let nb_pop_id = ui.make_persistent_id("aqua_name_box");
                    if nb_resp.clicked() {
                        ui.memory_mut(|mem| mem.toggle_popup(nb_pop_id));
                    }
                    egui::popup_below_widget(
                        ui,
                        nb_pop_id,
                        &nb_resp,
                        egui::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            ui.set_min_width(180.0);
                            if ui.selectable_label(false, "B4 · Phase Temp").clicked() {
                                self.select_cell(4, 1);
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                            for idx in 0..self.rows.len() {
                                let label = format!("A{} · {}", idx + 7, self.rows[idx].formula);
                                if ui.selectable_label(false, label).clicked() {
                                    self.select_cell(idx + 7, 0);
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            }
                        },
                    );

                    // Full-width crisp white Excel Formula Bar
                    let editable = (self.selected_cell.row == 4 && self.selected_cell.col == 1)
                        || (self.selected_cell.row >= 7 && self.selected_cell.col <= 2);
                    let fb_w = ui.available_width().max(160.0);
                    let (fb_rect, _) =
                        ui.allocate_exact_size(egui::vec2(fb_w, 21.0), egui::Sense::hover());
                    {
                        let p = ui.painter();
                        p.rect_filled(fb_rect, 0.0, egui::Color32::WHITE);
                        p.rect_stroke(
                            fb_rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(172, 173, 179)),
                        );
                    }
                    if editable {
                        let mut child =
                            ui.new_child(egui::UiBuilder::new().max_rect(fb_rect.shrink2(egui::vec2(4.0, 1.0))));
                        let resp = child.add(
                            egui::TextEdit::singleline(&mut self.formula_bar_buf)
                                .desired_width((fb_w - 8.0).max(120.0))
                                .frame(false)
                                .text_color(HSC_DARK_TEXT)
                                .font(egui::FontId::proportional(11.5)),
                        );
                        if resp.changed() {
                            let txt = self.formula_bar_buf.clone();
                            self.apply_formula_bar_edit(txt, db);
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.recalculate(db);
                        }
                    } else if !self.formula_bar_buf.is_empty() {
                        ui.painter()
                            .with_clip_rect(fb_rect.shrink2(egui::vec2(5.0, 1.0)))
                            .text(
                                fb_rect.left_center() + egui::vec2(5.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                &self.formula_bar_buf,
                                egui::FontId::proportional(11.5),
                                HSC_DARK_TEXT,
                            );
                    }
                });
            });
    }

    fn render_footer_and_tabs(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("aqua_footer")
            .frame(
                egui::Frame::none()
                    .fill(HSC_WIN_BG)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(205, 205, 205)))
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                // Classic Excel sheet tab & horizontal scrollbar bar
                let (tab_bar_rect, _) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
                if ui.is_rect_visible(tab_bar_rect) {
                    let p = ui.painter();
                    p.rect_filled(tab_bar_rect, 0.0, egui::Color32::from_rgb(225, 225, 225));
                    p.line_segment(
                        [tab_bar_rect.left_top(), tab_bar_rect.right_top()],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(185, 185, 185)),
                    );
                    p.line_segment(
                        [tab_bar_rect.left_bottom(), tab_bar_rect.right_bottom()],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(185, 185, 185)),
                    );

                    // Left nav arrows: |<  <  >  >|
                    p.text(
                        egui::pos2(tab_bar_rect.min.x + 8.0, tab_bar_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        "|<  <  >  >|",
                        egui::FontId::proportional(9.5),
                        HSC_MUTED_TEXT,
                    );

                    // Slanted white active worksheet tab: Water Phase
                    let tab_x0 = tab_bar_rect.min.x + 68.0;
                    let tab_rect = egui::Rect::from_min_max(
                        egui::pos2(tab_x0, tab_bar_rect.min.y),
                        egui::pos2(tab_x0 + 104.0, tab_bar_rect.max.y - 1.0),
                    );
                    let poly = vec![
                        tab_rect.left_top(),
                        tab_rect.right_top(),
                        egui::pos2(tab_rect.max.x - 8.0, tab_rect.max.y),
                        egui::pos2(tab_rect.min.x + 3.0, tab_rect.max.y),
                    ];
                    p.add(egui::Shape::convex_polygon(poly.clone(), egui::Color32::WHITE, egui::Stroke::NONE));
                    let edge_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(142, 142, 142));
                    p.line_segment([poly[0], poly[3]], edge_stroke);
                    p.line_segment([poly[3], poly[2]], edge_stroke);
                    p.line_segment([poly[2], poly[1]], edge_stroke);
                    p.text(
                        tab_rect.center() - egui::vec2(2.0, 0.0),
                        egui::Align2::CENTER_CENTER,
                        "Water Phase",
                        egui::FontId::proportional(11.0),
                        HSC_DARK_TEXT,
                    );

                    // Subtle visual horizontal scrollbar strip on the right (matching classic Excel/HSC)
                    let sb_left = (tab_rect.max.x + 280.0).min(tab_bar_rect.max.x - 160.0).max(tab_rect.max.x + 30.0);
                    let sb_rect = egui::Rect::from_min_max(
                        egui::pos2(sb_left, tab_bar_rect.min.y + 2.0),
                        egui::pos2(tab_bar_rect.max.x - 6.0, tab_bar_rect.max.y - 2.0),
                    );
                    if sb_rect.width() > 60.0 {
                        p.rect_filled(sb_rect, 1.0, egui::Color32::from_rgb(238, 238, 238));
                        let thumb = egui::Rect::from_min_size(
                            egui::pos2(sb_rect.min.x + 14.0, sb_rect.min.y + 1.0),
                            egui::vec2((sb_rect.width() * 0.28).clamp(36.0, 90.0), sb_rect.height() - 2.0),
                        );
                        p.rect_filled(thumb, 2.0, egui::Color32::from_rgb(212, 212, 212));
                        p.rect_stroke(thumb, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 168, 168)));
                    }
                }

                // Bottom Outotec/Soryntel classic brand bar
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Soryntel")
                            .strong()
                            .size(16.5)
                            .color(HSC_BRAND_RED),
                    );
                    ui.add_space(14.0);
                    if self.dirty_since_calc {
                        ui.label(
                            egui::RichText::new("● Inputs modified — click Calculate")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 83, 9)),
                        );
                    } else {
                        ui.label(egui::RichText::new(&self.status_msg).size(11.0).color(HSC_MUTED_TEXT));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} sp | OwnDB: {}",
                                self.calc_result.species.len(),
                                if self.case_file.use_own_db { "ON" } else { "OFF" }
                            ))
                            .size(10.5)
                            .color(HSC_MUTED_TEXT),
                        );
                        if let Some(w) = self.calc_result.warnings.first() {
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(format!("⚠ {w}"))
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(180, 83, 9)),
                            );
                        }
                    });
                });
                ui.add_space(3.0);
            });
    }

    fn render_water_phase_sheet(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let total_cols = 24.max(13 + self.calc_result.element_columns.len() + 2);
        let widths = aqua_column_widths(total_cols);

        let tu = self.case_file.temp_unit.label();
        let (eu, cu) = match self.case_file.energy_unit {
            AquaEnergyUnit::Kcal => ("kcal", "cal"),
            AquaEnergyUnit::KJ => ("kJ", "J"),
        };
        let pu = self.case_file.press_unit.label();

        let top_title = [
            "Water Phase\nData",
            "Temp.",
            "Amount",
            "Amount",
            "Amount",
            "H Ideal",
            "H Estimate",
            "Cp Ideal",
            "Cp Estimate",
            "H Ideal",
            "H Estimate",
            "Cp Ideal",
            "Cp Estimate",
            "Osmotic",
            "H2O Vapour\nPress.",
            "Sol. Vapour\nPress.",
            "Ionic\nStrength",
            "pH",
            "Relat.\nHumidity",
            "FP",
            "BP",
            "FP (aw\nat 25°C)",
            "BP (aw\nat 25°C)",
            "Osmotic\nPressure",
        ];
        let h_per_mol = format!("{eu}/mol");
        let cp_per_mol = format!("{cu}/(mol*K)");
        let h_per_kg = format!("{eu}/kg");
        let cp_per_kg = format!("{eu}/(kg*K)");
        let top_units: Vec<String> = vec![
            "".into(),
            tu.into(),
            "kmol".into(),
            "mol-%".into(),
            "g".into(),
            h_per_mol.clone(),
            h_per_mol.clone(),
            cp_per_mol.clone(),
            cp_per_mol.clone(),
            h_per_kg.clone(),
            h_per_kg,
            cp_per_kg.clone(),
            cp_per_kg,
            "Coeff.".into(),
            pu.into(),
            pu.into(),
            "mol/kg".into(),
            "".into(),
            "%".into(),
            "°C".into(),
            "°C".into(),
            "°C".into(),
            "°C".into(),
            pu.into(),
        ];

        let bot_title = [
            "Water Species\nData",
            "Temp.",
            "Amount",
            "Amount",
            "Amount",
            "H Ideal",
            "H Estimate",
            "Cp Ideal",
            "Cp Estimate",
            "AC",
            "AC",
            "Molality",
            "MW",
        ];
        let bot_units: Vec<String> = vec![
            "".into(),
            tu.into(),
            "kmol".into(),
            "mol-%".into(),
            "g".into(),
            h_per_mol.clone(),
            h_per_mol,
            cp_per_mol.clone(),
            cp_per_mol,
            "Molar Sc.".into(),
            "Molal Sc.".into(),
            "mol/kg".into(),
            "g/mol".into(),
        ];

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(HSC_WIN_BG))
            .show(ctx, |ui| {
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    // Excel Column Letter Strip (A, B, C...)
                    ui.horizontal(|ui| {
                        header_cell(ui, "", 32.0);
                        for (c, &w) in widths.iter().enumerate() {
                            let bg = if c == self.selected_cell.col {
                                HSC_SEL_COL_BG
                            } else {
                                EXCEL_HEADER_BG
                            };
                            draw_text_cell(ui, &col_letter(c), w, CELL_H, bg, HSC_DARK_TEXT, false, true, false);
                        }
                    });

                    // Merged Rows 1–3: Water Phase Data header band (no horizontal split across data columns)
                    let row123_resp = ui.horizontal(|ui| {
                        let any_sel = (1..=3).contains(&self.selected_cell.row);
                        draw_gutter_stack_cell(ui, &["1", "2", "3"], 32.0, 51.0, any_sel);
                        for (c, &w) in widths.iter().enumerate() {
                            let t = top_title.get(c).copied().unwrap_or("");
                            let u = top_units.get(c).map(String::as_str).unwrap_or("");
                            draw_merged_hsc_header(ui, w, 51.0, t, u);
                        }
                    });
                    paint_sheet_rule(ui, row123_resp.response.rect.left() + 32.0, row123_resp.response.rect.right(), row123_resp.response.rect.bottom());

                    // Row 4: Water Phase Summary Row (A4 beige centered, B4 white centered editable, C4.. beige centered)
                    let mut phase_temp_edited = false;
                    let mut clicked_cell: Option<CellCoord> = None;
                    let mut enter_trigger = false;
                    let row4_resp = ui.horizontal(|ui| {
                        row_num_cell_hi(ui, 4, self.selected_cell.row == 4);
                        for (c, &w) in widths.iter().enumerate() {
                            let is_sel = self.selected_cell == CellCoord { row: 4, col: c };
                            if c == 1 {
                                let (_, resp, changed, lost_enter) = draw_editable_cell_align(
                                    ui,
                                    "aqua_phase_temp_b4",
                                    &mut self.phase_temp_buf,
                                    w,
                                    CELL_H,
                                    egui::Color32::WHITE,
                                    is_sel,
                                    CellAlign::Center,
                                );
                                if resp.clicked() || resp.gained_focus() {
                                    clicked_cell = Some(CellCoord { row: 4, col: 1 });
                                }
                                if changed {
                                    phase_temp_edited = true;
                                }
                                if lost_enter {
                                    enter_trigger = true;
                                }
                            } else {
                                let txt = if c == 0 {
                                    "Water Phase".to_string()
                                } else {
                                    self.phase_row_col_text(c)
                                };
                                let resp = draw_selectable_cell_align(
                                    ui,
                                    &txt,
                                    w,
                                    CELL_H,
                                    HSC_BEIGE_BG,
                                    HSC_DARK_TEXT,
                                    false,
                                    CellAlign::Center,
                                    is_sel,
                                );
                                if resp.clicked() {
                                    clicked_cell = Some(CellCoord { row: 4, col: c });
                                }
                            }
                        }
                    });
                    paint_sheet_rule(ui, row4_resp.response.rect.left() + 32.0, row4_resp.response.rect.right(), row4_resp.response.rect.bottom());
                    if phase_temp_edited {
                        for r in &mut self.rows {
                            r.temp_buf = self.phase_temp_buf.clone();
                        }
                        self.dirty_since_calc = true;
                    }

                    // Merged Rows 5–6: Water Species Data (A..M merged vertically) + Elements (N..end)
                    let row56_resp = ui.horizontal(|ui| {
                        let any_sel = self.selected_cell.row == 5 || self.selected_cell.row == 6;
                        draw_gutter_stack_cell(ui, &["5", "6"], 32.0, 38.0, any_sel);
                        for (c, &w) in widths.iter().enumerate() {
                            if c < 13 {
                                let t = bot_title.get(c).copied().unwrap_or("");
                                let u = bot_units.get(c).map(String::as_str).unwrap_or("");
                                draw_merged_hsc_header(ui, w, 38.0, t, u);
                            } else {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    let top_txt = if c == 13 { "Elements:" } else { "" };
                                    draw_elements_top_cell(ui, top_txt, w, 19.0, c == 13);
                                    let el_txt = self
                                        .calc_result
                                        .element_columns
                                        .get(c - 13)
                                        .map(String::as_str)
                                        .unwrap_or("");
                                    draw_text_cell(ui, el_txt, w, 19.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                                });
                            }
                        }
                    });
                    paint_sheet_rule(ui, row56_resp.response.rect.left() + 32.0, row56_resp.response.rect.right(), row56_resp.response.rect.bottom());

                    // Species Rows (Row 7..N, including direct-editable blank rows 22+)
                    let num_display_rows = (self.rows.len() + 3).max(24);
                    for ridx in 0..num_display_rows {
                        let excel_row = ridx + 7;
                        ui.horizontal(|ui| {
                            row_num_cell_hi(ui, excel_row, self.selected_cell.row == excel_row);
                            let (mut f_tmp, mut t_tmp, mut a_tmp) = if let Some(row_ref) = self.rows.get(ridx) {
                                (
                                    row_ref.formula.clone(),
                                    row_ref.temp_buf.clone(),
                                    row_ref.amount_buf.clone(),
                                )
                            } else {
                                (String::new(), String::new(), String::new())
                            };

                            let (_, resp0, ch0, ent0) = draw_editable_cell_align(
                                ui,
                                &format!("aq_sp_f_{ridx}"),
                                &mut f_tmp,
                                widths[0],
                                CELL_H,
                                egui::Color32::WHITE,
                                self.selected_cell == CellCoord { row: excel_row, col: 0 },
                                CellAlign::Left,
                            );
                            if resp0.clicked() || resp0.gained_focus() {
                                clicked_cell = Some(CellCoord { row: excel_row, col: 0 });
                            }
                            if ch0 {
                                self.set_species_cell(ridx, 0, f_tmp);
                            }
                            if ent0 {
                                enter_trigger = true;
                            }

                            let (_, resp1, ch1, ent1) = draw_editable_cell_align(
                                ui,
                                &format!("aq_sp_t_{ridx}"),
                                &mut t_tmp,
                                widths[1],
                                CELL_H,
                                egui::Color32::WHITE,
                                self.selected_cell == CellCoord { row: excel_row, col: 1 },
                                CellAlign::Center,
                            );
                            if resp1.clicked() || resp1.gained_focus() {
                                clicked_cell = Some(CellCoord { row: excel_row, col: 1 });
                            }
                            if ch1 {
                                self.set_species_cell(ridx, 1, t_tmp);
                            }
                            if ent1 {
                                enter_trigger = true;
                            }

                            let (_, resp2, ch2, ent2) = draw_editable_cell_align(
                                ui,
                                &format!("aq_sp_a_{ridx}"),
                                &mut a_tmp,
                                widths[2],
                                CELL_H,
                                egui::Color32::WHITE,
                                self.selected_cell == CellCoord { row: excel_row, col: 2 },
                                CellAlign::Center,
                            );
                            if resp2.clicked() || resp2.gained_focus() {
                                clicked_cell = Some(CellCoord { row: excel_row, col: 2 });
                            }
                            if ch2 {
                                self.set_species_cell(ridx, 2, a_tmp);
                            }
                            if ent2 {
                                enter_trigger = true;
                            }

                            for (c, &w) in widths.iter().enumerate().skip(3) {
                                let is_sel = self.selected_cell == CellCoord { row: excel_row, col: c };
                                let txt = if ridx < self.rows.len() {
                                    self.species_row_col_text(ridx, c)
                                } else {
                                    String::new()
                                };
                                let resp = draw_selectable_cell_align(
                                    ui,
                                    &txt,
                                    w,
                                    CELL_H,
                                    HSC_BEIGE_BG,
                                    HSC_DARK_TEXT,
                                    false,
                                    CellAlign::Center,
                                    is_sel,
                                );
                                if resp.clicked() {
                                    clicked_cell = Some(CellCoord { row: excel_row, col: c });
                                }
                            }
                        });
                    }

                    if let Some(cc) = clicked_cell {
                        self.select_cell(cc.row, cc.col);
                    }
                    if enter_trigger {
                        self.recalculate(db);
                    }
                });
            });
    }

    fn render_main_db_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_main_db;
        let mut recalc_needed = false;
        egui::Window::new("Main Pitzer Database")
            .open(&mut open)
            .default_size([980.0, 560.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.render_pitzer_doc_header(ui, false);

                // Controls strip: View mode (Fig. 5 Status vs Fig. 9 Detailed X) + Color Legend
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("View:").strong().size(11.0));
                    ui.selectable_value(
                        &mut self.main_db_mode,
                        MainDbViewMode::DetailedX,
                        "Detailed Active Set & Eq.(22) (Fig. 9)",
                    );
                    ui.selectable_value(
                        &mut self.main_db_mode,
                        MainDbViewMode::StatusFlags,
                        "Parameter Matrix T/C/* (Fig. 5)",
                    );
                    ui.separator();
                    ui.label(egui::RichText::new("■ MainDB").small().color(egui::Color32::BLACK));
                    ui.label(
                        egui::RichText::new("■ OwnDB Only")
                            .small()
                            .color(egui::Color32::from_rgb(29, 78, 216)),
                    );
                    ui.label(
                        egui::RichText::new("■ OwnDB Overwrites MainDB")
                            .small()
                            .color(egui::Color32::from_rgb(21, 128, 61)),
                    );
                    ui.label(
                        egui::RichText::new("■ Inactive / Excluded")
                            .small()
                            .color(egui::Color32::from_rgb(185, 28, 28)),
                    );
                });
                ui.add_space(4.0);

                egui::ScrollArea::both().max_height(300.0).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let mut toggle_key: Option<String> = None;

                    match self.main_db_tab {
                        PitzerSheetTab::Binary => {
                            let show_check = self.main_db_mode == MainDbViewMode::DetailedX;
                            draw_pitzer_table_header(ui, &["Cation (+)", "Anion (-)"], show_check, show_check);
                            for row in &self.merged_catalog.binaries {
                                let fg = source_color_rgb(row.color);
                                let mut chk = row.active;
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.cation, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.anion, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    if show_check {
                                        egui::Frame::none()
                                            .fill(HSC_BEIGE_BG)
                                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                            .inner_margin(egui::Margin::symmetric(14.0, 2.0))
                                            .show(ui, |ui| {
                                                if ui.checkbox(&mut chk, "").changed() {
                                                    toggle_key = Some(row.key.clone());
                                                }
                                            });
                                    }
                                    draw_pitzer_status_or_x_cells(
                                        ui,
                                        &row.entry.beta0,
                                        &row.entry.beta1,
                                        &row.entry.beta2,
                                        &row.entry.c_phi,
                                        &row.entry.range,
                                        self.main_db_mode,
                                        fg,
                                    );
                                });
                            }
                        }
                        PitzerSheetTab::Theta => {
                            draw_pitzer_table_header(ui, &["Ion i (+/-)", "Ion j (+/-)"], false, false);
                            for row in &self.merged_catalog.thetas {
                                let fg = source_color_rgb(row.color);
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.ion_i, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.ion_j, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_single_curve_cells(ui, &row.entry.theta, &row.entry.range, fg);
                                });
                            }
                        }
                        PitzerSheetTab::Psi => {
                            draw_pitzer_table_header(ui, &["Ion 1", "Ion 2", "Ion 3"], false, false);
                            for row in &self.merged_catalog.psis {
                                let fg = source_color_rgb(row.color);
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.ion1, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.ion2, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.ion3, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_single_curve_cells(ui, &row.entry.psi, &row.entry.range, fg);
                                });
                            }
                        }
                        PitzerSheetTab::Lambda => {
                            draw_pitzer_table_header(ui, &["Neutral", "Ion"], false, false);
                            for row in &self.merged_catalog.lambdas {
                                let fg = source_color_rgb(row.color);
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.neutral, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.ion, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_single_curve_cells(ui, &row.entry.lambda, &row.entry.range, fg);
                                });
                            }
                        }
                    }

                    if let Some(k) = toggle_key {
                        if self.excluded_keys.contains(&k) {
                            self.excluded_keys.remove(&k);
                        } else {
                            self.excluded_keys.insert(k);
                        }
                        recalc_needed = true;
                    }
                });

                pitzer_4tab_bar(ui, &mut self.main_db_tab);
            });
        self.show_main_db = open;
        if recalc_needed {
            self.recalculate(db);
        }
    }

    fn render_param_sets_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_param_sets;
        egui::Window::new(crate::i18n::text("common.094"))
            .open(&mut open)
            .default_size([920.0, 520.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::text("common.094"))
                        .strong()
                        .size(15.0)
                        .color(HSC_DARK_TEXT),
                );
                ui.label(
                    egui::RichText::new("Pitzer interaction parameters used from the database based on specified solution and calculation method.")
                        .size(11.0)
                        .color(HSC_MUTED_TEXT),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("F(T) = a + b/T + c·ln(T) + d·T + e·T² + f·T⁻²")
                            .monospace()
                            .size(11.0),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new("T = temp. dependent   |   C = constant at 25 °C   |   * = unavailable")
                            .size(10.5)
                            .color(HSC_MUTED_TEXT),
                    );
                });
                ui.add_space(4.0);

                egui::ScrollArea::both().max_height(355.0).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    match self.param_sets_tab {
                        ParamSetsTab::Active => {
                            draw_text_cell(
                                ui,
                                "Pitzer Binaries: Cation - Anion Interactions",
                                860.0,
                                20.0,
                                HSC_BANNER_BG,
                                HSC_DARK_TEXT,
                                true,
                                false,
                                false,
                            );
                            draw_pitzer_table_header(ui, &["Cation (+)", "Anion (-)"], false, false);
                            for row in &self.param_report.active_binaries {
                                let fg = source_color_rgb(row.color);
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.cation, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.anion, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_pitzer_status_or_x_cells(
                                        ui,
                                        &row.entry.beta0,
                                        &row.entry.beta1,
                                        &row.entry.beta2,
                                        &row.entry.c_phi,
                                        &row.entry.range,
                                        MainDbViewMode::StatusFlags,
                                        fg,
                                    );
                                });
                            }

                            draw_text_cell(
                                ui,
                                "Theta: Cation - Cation & Anion - Anion Interactions",
                                860.0,
                                20.0,
                                HSC_BANNER_BG,
                                HSC_DARK_TEXT,
                                true,
                                false,
                                false,
                            );
                            for row in &self.param_report.active_thetas {
                                let fg = source_color_rgb(row.color);
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, &row.entry.ion_i, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_text_cell(ui, &row.entry.ion_j, 92.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                    draw_single_curve_cells(ui, &row.entry.theta, &row.entry.range, fg);
                                });
                            }

                            if !self.param_report.active_psis.is_empty() {
                                draw_text_cell(
                                    ui,
                                    "Psi: Ternary C-C-A & A-A-C Interactions",
                                    860.0,
                                    20.0,
                                    HSC_BANNER_BG,
                                    HSC_DARK_TEXT,
                                    true,
                                    false,
                                    false,
                                );
                                for row in &self.param_report.active_psis {
                                    let fg = source_color_rgb(row.color);
                                    ui.horizontal(|ui| {
                                        draw_text_cell(ui, &row.entry.ion1, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                        draw_text_cell(ui, &row.entry.ion2, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                        draw_text_cell(ui, &row.entry.ion3, 66.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
                                        draw_single_curve_cells(ui, &row.entry.psi, &row.entry.range, fg);
                                    });
                                }
                            }
                        }
                        ParamSetsTab::Missing => {
                            draw_text_cell(
                                ui,
                                "Missing Binary Pairs (Fallback = 0 or Davies)",
                                680.0,
                                20.0,
                                HSC_BANNER_BG,
                                HSC_DARK_TEXT,
                                true,
                                false,
                                false,
                            );
                            if self.param_report.missing_binaries.is_empty() {
                                draw_text_cell(ui, "None — all active pairs found", 680.0, CELL_H, HSC_BEIGE_BG, HSC_MUTED_TEXT, false, true, false);
                            } else {
                                for (c, a) in &self.param_report.missing_binaries {
                                    ui.horizontal(|ui| {
                                        draw_text_cell(ui, c, 120.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                        draw_text_cell(ui, a, 120.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                        draw_text_cell(ui, "Missing from MainDB + OwnDB", 440.0, CELL_H, HSC_BEIGE_BG, HSC_MUTED_TEXT, false, false, false);
                                    });
                                }
                            }

                            draw_text_cell(
                                ui,
                                "Missing Theta Pairs",
                                680.0,
                                20.0,
                                HSC_BANNER_BG,
                                HSC_DARK_TEXT,
                                true,
                                false,
                                false,
                            );
                            for (i, j) in &self.param_report.missing_thetas {
                                ui.horizontal(|ui| {
                                    draw_text_cell(ui, i, 120.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                    draw_text_cell(ui, j, 120.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                    draw_text_cell(ui, "Missing Theta", 440.0, CELL_H, HSC_BEIGE_BG, HSC_MUTED_TEXT, false, false, false);
                                });
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let act_n = self.param_report.active_binaries.len()
                        + self.param_report.active_thetas.len()
                        + self.param_report.active_psis.len();
                    let miss_n = self.param_report.missing_binaries.len()
                        + self.param_report.missing_thetas.len();
                    ui.selectable_value(
                        &mut self.param_sets_tab,
                        ParamSetsTab::Active,
                        format!("Active Parameters ({act_n})"),
                    );
                    ui.selectable_value(
                        &mut self.param_sets_tab,
                        ParamSetsTab::Missing,
                        format!("Missing Parameters ({miss_n})"),
                    );
                });
            });
        self.show_param_sets = open;
    }

    fn render_own_db_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_own_db;
        let mut changed = false;
        egui::Window::new("OwnDB")
            .open(&mut open)
            .default_size([920.0, 480.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Fig. 8 Top Toolbar: Table ▼ | Delete Row | + Add Row
                ui.horizontal(|ui| {
                    ui.menu_button("Table", |ui| {
                        if ui.button("Open (load temp JSON)...").clicked() {
                            let p = self
                                .own_db_json_path
                                .clone()
                                .unwrap_or_else(|| std::env::temp_dir().join("soryntel_own_pitzer.json"));
                            if let Ok(txt) = std::fs::read_to_string(&p) {
                                if let Ok(loaded) = serde_json::from_str::<OwnPitzerDb>(&txt) {
                                    self.own_db = loaded;
                                    changed = true;
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button(crate::i18n::text("common.034")).clicked() {
                            let p = self
                                .own_db_json_path
                                .get_or_insert_with(|| std::env::temp_dir().join("soryntel_own_pitzer.json"))
                                .clone();
                            if let Ok(json) = serde_json::to_string_pretty(&self.own_db) {
                                let _ = crate::storage::write_atomic(&p, json);
                                self.status_msg = format!("Saved OwnDB to {}", p.display());
                            }
                            ui.close_menu();
                        }
                        if ui.button("Save As Timestamped...").clicked() {
                            let p = std::env::temp_dir().join("soryntel_own_pitzer_backup.json");
                            self.own_db_json_path = Some(p.clone());
                            if let Ok(json) = serde_json::to_string_pretty(&self.own_db) {
                                let _ = crate::storage::write_atomic(&p, json);
                                self.status_msg = format!("Saved OwnDB As {}", p.display());
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Reset to HSC Fig. 8 Sample").clicked() {
                            self.own_db = default_sample_own_db();
                            changed = true;
                            ui.close_menu();
                        }
                    });

                    if ui.button(crate::i18n::text("common.078")).clicked() {
                        match self.own_db_tab {
                            PitzerSheetTab::Binary if self.own_db_selected_row < self.own_db.binaries.len() => {
                                self.own_db.binaries.remove(self.own_db_selected_row);
                                changed = true;
                            }
                            PitzerSheetTab::Theta if self.own_db_selected_row < self.own_db.thetas.len() => {
                                self.own_db.thetas.remove(self.own_db_selected_row);
                                changed = true;
                            }
                            PitzerSheetTab::Psi if self.own_db_selected_row < self.own_db.psis.len() => {
                                self.own_db.psis.remove(self.own_db_selected_row);
                                changed = true;
                            }
                            PitzerSheetTab::Lambda if self.own_db_selected_row < self.own_db.lambdas.len() => {
                                self.own_db.lambdas.remove(self.own_db_selected_row);
                                changed = true;
                            }
                            _ => {}
                        }
                    }

                    if ui.button(crate::i18n::text("common.095")).clicked() {
                        match self.own_db_tab {
                            PitzerSheetTab::Binary => self.own_db.binaries.push(OwnBinaryRow {
                                cation: "Na(+a)".into(),
                                anion: "Cl(-a)".into(),
                                show_t_params: false,
                                min_t_c: Some(25.0),
                                max_t_c: Some(55.0),
                                min_m: None,
                                max_m: None,
                                min_i: None,
                                max_i: None,
                                beta0: PitzerTempCurve::constant(0.0765),
                                beta1: PitzerTempCurve::constant(0.2664),
                                beta2: PitzerTempCurve::EMPTY,
                                c_phi: PitzerTempCurve::constant(0.00127),
                            }),
                            PitzerSheetTab::Theta => self.own_db.thetas.push(OwnThetaRow {
                                ion_i: "H(+a)".into(),
                                ion_j: "Na(+a)".into(),
                                show_t_params: false,
                                min_t_c: Some(25.0),
                                max_t_c: Some(55.0),
                                min_m: None,
                                max_m: None,
                                min_i: None,
                                max_i: None,
                                theta: PitzerTempCurve::constant(0.036),
                            }),
                            PitzerSheetTab::Psi => self.own_db.psis.push(OwnPsiRow {
                                ion1: "H(+a)".into(),
                                ion2: "Na(+a)".into(),
                                ion3: "Cl(-a)".into(),
                                show_t_params: false,
                                min_t_c: Some(25.0),
                                max_t_c: Some(55.0),
                                min_m: None,
                                max_m: None,
                                min_i: None,
                                max_i: None,
                                psi: PitzerTempCurve::constant(-0.004),
                            }),
                            PitzerSheetTab::Lambda => self.own_db.lambdas.push(OwnLambdaRow {
                                neutral: "CO2(a)".into(),
                                ion: "Na(+a)".into(),
                                show_t_params: false,
                                min_t_c: Some(25.0),
                                max_t_c: Some(55.0),
                                min_m: None,
                                max_m: None,
                                min_i: None,
                                max_i: None,
                                lambda: PitzerTempCurve::constant(0.10),
                            }),
                        }
                        changed = true;
                    }

                    let valid_bins = self.own_db.valid_binaries_prefix().len();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Sequential valid prefix: {valid_bins}/{} binaries (HSC Sec. 36.6)",
                                self.own_db.binaries.len()
                            ))
                            .size(10.5)
                            .color(HSC_MUTED_TEXT),
                        );
                    });
                });
                ui.separator();

                self.render_pitzer_doc_header(ui, true);

                egui::ScrollArea::both().max_height(250.0).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    match self.own_db_tab {
                        PitzerSheetTab::Binary => {
                            ui.horizontal(|ui| {
                                for (lbl, w) in [
                                    ("Cation (+)", 82.0),
                                    ("Anion (-)", 82.0),
                                    ("Show T\nparams", 58.0),
                                    ("Min T", 50.0),
                                    ("Max T", 50.0),
                                    ("Min m", 50.0),
                                    ("Max m", 50.0),
                                    ("Min I", 50.0),
                                    ("Max I", 50.0),
                                    ("b(0) a", 64.0),
                                    ("b(0) b", 64.0),
                                    ("b(0) c", 64.0),
                                    ("b(0) d", 64.0),
                                    ("b(1) a", 64.0),
                                    ("C(f) a", 64.0),
                                ] {
                                    draw_text_cell(ui, lbl, w, 28.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                                }
                            });
                            for idx in 0..self.own_db.binaries.len() {
                                let row = &mut self.own_db.binaries[idx];
                                let is_sel = idx == self.own_db_selected_row;
                                ui.horizontal(|ui| {
                                    let (_, r0, c0, _) = draw_editable_cell(ui, &format!("own_b_c_{idx}"), &mut row.cation, 82.0, CELL_H, egui::Color32::WHITE, is_sel, false);
                                    let (_, r1, c1, _) = draw_editable_cell(ui, &format!("own_b_a_{idx}"), &mut row.anion, 82.0, CELL_H, egui::Color32::WHITE, false, false);
                                    if r0.clicked() || r1.clicked() {
                                        self.own_db_selected_row = idx;
                                    }
                                    if c0 || c1 {
                                        changed = true;
                                    }
                                    egui::Frame::none()
                                        .fill(HSC_BEIGE_BG)
                                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                        .inner_margin(egui::Margin::symmetric(18.0, 2.0))
                                        .show(ui, |ui| {
                                            if ui.checkbox(&mut row.show_t_params, "").changed() {
                                                changed = true;
                                            }
                                        });
                                    for (key, field_opt) in [
                                        ("tmin", &mut row.min_t_c),
                                        ("tmax", &mut row.max_t_c),
                                        ("mmin", &mut row.min_m),
                                        ("mmax", &mut row.max_m),
                                        ("imin", &mut row.min_i),
                                        ("imax", &mut row.max_i),
                                    ] {
                                        if opt_f64_cell(ui, &format!("own_b_{key}_{idx}"), field_opt, 50.0, true) {
                                            changed = true;
                                        }
                                    }
                                    if f64_cell(ui, &format!("own_b_b0a_{idx}"), &mut row.beta0.a, 64.0, true) {
                                        row.beta0.defined = true;
                                        changed = true;
                                    }
                                    let t_en = row.show_t_params;
                                    if f64_cell(ui, &format!("own_b_b0b_{idx}"), &mut row.beta0.b, 64.0, t_en) {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_b_b0c_{idx}"), &mut row.beta0.c, 64.0, t_en) {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_b_b0d_{idx}"), &mut row.beta0.d, 64.0, t_en) {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_b_b1a_{idx}"), &mut row.beta1.a, 64.0, true) {
                                        row.beta1.defined = true;
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_b_cfa_{idx}"), &mut row.c_phi.a, 64.0, true) {
                                        row.c_phi.defined = true;
                                        changed = true;
                                    }
                                });
                            }
                        }
                        PitzerSheetTab::Theta => {
                            ui.horizontal(|ui| {
                                for (lbl, w) in [("Ion i", 86.0), ("Ion j", 86.0), ("Show T", 54.0), ("Theta a", 80.0), ("Theta b", 80.0)] {
                                    draw_text_cell(ui, lbl, w, 24.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                                }
                            });
                            for idx in 0..self.own_db.thetas.len() {
                                let row = &mut self.own_db.thetas[idx];
                                ui.horizontal(|ui| {
                                    let (_, r0, c0, _) = draw_editable_cell(ui, &format!("own_t_i_{idx}"), &mut row.ion_i, 86.0, CELL_H, egui::Color32::WHITE, idx == self.own_db_selected_row, false);
                                    let (_, r1, c1, _) = draw_editable_cell(ui, &format!("own_t_j_{idx}"), &mut row.ion_j, 86.0, CELL_H, egui::Color32::WHITE, false, false);
                                    if r0.clicked() || r1.clicked() {
                                        self.own_db_selected_row = idx;
                                    }
                                    if c0 || c1 {
                                        changed = true;
                                    }
                                    if ui.checkbox(&mut row.show_t_params, "").changed() {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_t_a_{idx}"), &mut row.theta.a, 80.0, true) {
                                        row.theta.defined = true;
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_t_b_{idx}"), &mut row.theta.b, 80.0, row.show_t_params) {
                                        changed = true;
                                    }
                                });
                            }
                        }
                        PitzerSheetTab::Psi => {
                            ui.horizontal(|ui| {
                                for (lbl, w) in [("Ion 1", 72.0), ("Ion 2", 72.0), ("Ion 3", 72.0), ("Psi a", 80.0)] {
                                    draw_text_cell(ui, lbl, w, 24.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                                }
                            });
                            for idx in 0..self.own_db.psis.len() {
                                let row = &mut self.own_db.psis[idx];
                                ui.horizontal(|ui| {
                                    if draw_editable_cell(ui, &format!("own_p_1_{idx}"), &mut row.ion1, 72.0, CELL_H, egui::Color32::WHITE, idx == self.own_db_selected_row, false).2 {
                                        changed = true;
                                    }
                                    if draw_editable_cell(ui, &format!("own_p_2_{idx}"), &mut row.ion2, 72.0, CELL_H, egui::Color32::WHITE, false, false).2 {
                                        changed = true;
                                    }
                                    if draw_editable_cell(ui, &format!("own_p_3_{idx}"), &mut row.ion3, 72.0, CELL_H, egui::Color32::WHITE, false, false).2 {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_p_a_{idx}"), &mut row.psi.a, 80.0, true) {
                                        row.psi.defined = true;
                                        changed = true;
                                    }
                                });
                            }
                        }
                        PitzerSheetTab::Lambda => {
                            ui.horizontal(|ui| {
                                for (lbl, w) in [("Neutral", 92.0), ("Ion", 92.0), ("Lambda a", 80.0)] {
                                    draw_text_cell(ui, lbl, w, 24.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                                }
                            });
                            for idx in 0..self.own_db.lambdas.len() {
                                let row = &mut self.own_db.lambdas[idx];
                                ui.horizontal(|ui| {
                                    if draw_editable_cell(ui, &format!("own_l_n_{idx}"), &mut row.neutral, 92.0, CELL_H, egui::Color32::WHITE, idx == self.own_db_selected_row, false).2 {
                                        changed = true;
                                    }
                                    if draw_editable_cell(ui, &format!("own_l_i_{idx}"), &mut row.ion, 92.0, CELL_H, egui::Color32::WHITE, false, false).2 {
                                        changed = true;
                                    }
                                    if f64_cell(ui, &format!("own_l_a_{idx}"), &mut row.lambda.a, 80.0, true) {
                                        row.lambda.defined = true;
                                        changed = true;
                                    }
                                });
                            }
                        }
                    }
                });

                pitzer_4tab_bar(ui, &mut self.own_db_tab);
            });
        self.show_own_db = open;
        if changed && self.case_file.use_own_db {
            self.recalculate(db);
        }
    }

    fn render_density_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_density;
        let mut recalc_density = false;
        egui::Window::new("Density Calculator")
            .open(&mut open)
            .default_size([900.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                // Fig. 7 mini ribbon: Add | Remove | Clear
                ui.horizontal(|ui| {
                    if ui.button("Add ➡").clicked() {
                        if let Some(idx) = self.density_selected_salt_idx {
                            if let Some(salt) = DENSITY_SALTS.get(idx) {
                                self.density_rows.push((salt.component.to_string(), "0.05".into()));
                                recalc_density = true;
                            }
                        }
                    }
                    if ui.button("⬅ Remove").clicked() {
                        if let Some(eidx) = self.density_selected_entry_idx {
                            if eidx < self.density_rows.len() {
                                self.density_rows.remove(eidx);
                                self.density_selected_entry_idx = None;
                                recalc_density = true;
                            }
                        }
                    }
                    if ui.button(crate::i18n::text("common.093")).clicked() {
                        self.density_rows.clear();
                        self.density_selected_entry_idx = None;
                        recalc_density = true;
                    }
                });
                ui.separator();

                ui.horizontal(|ui| {
                    // Left title
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Density of Aqueous Solution")
                                .strong()
                                .size(14.0)
                                .color(HSC_DARK_TEXT),
                        );
                        ui.label(
                            egui::RichText::new("Laliberté & Cooper (2004) · 59 Electrolytes (Aqueous Solution Density.xls)")
                                .size(10.5)
                                .color(HSC_MUTED_TEXT),
                        );
                    });
                    ui.add_space(40.0);

                    // Top right metrics (Fig. 7 H2..I4)
                    egui::Grid::new("density_top_summary").spacing([10.0, 2.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("H2O density").strong().size(11.0));
                        ui.label(
                            egui::RichText::new(format!("{:.2} kg/m3", self.density_result.h2o_density_kg_m3))
                                .size(11.0),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Solution density").strong().size(11.0));
                        let sol_txt = if self.density_result.solution_density_kg_m3.is_finite() {
                            format!("{:.6} kg/m3", self.density_result.solution_density_kg_m3)
                        } else {
                            self.density_result
                                .global_error
                                .clone()
                                .unwrap_or_else(|| "Invalid".into())
                        };
                        ui.label(egui::RichText::new(sol_txt).strong().size(11.5).color(HSC_DARK_TEXT));
                        ui.end_row();

                        ui.label(egui::RichText::new(crate::i18n::text("common.005")).strong().size(11.0));
                        ui.horizontal(|ui| {
                            let te = ui.add(
                                egui::TextEdit::singleline(&mut self.density_temp_buf)
                                    .desired_width(72.0)
                                    .font(egui::FontId::proportional(11.5)),
                            );
                            ui.label("°C");
                            if te.changed() {
                                recalc_density = true;
                            }
                        });
                        ui.end_row();
                    });
                });
                ui.add_space(6.0);

                ui.columns(2, |cols| {
                    // Left: 59 Salts Database table (B6..F65)
                    cols[0].spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    cols[0].horizontal(|ui| {
                        for (lbl, w) in [
                            ("Component in water", 120.0),
                            ("w Min", 62.0),
                            ("w Max", 62.0),
                            ("T Min", 52.0),
                            ("T Max", 52.0),
                        ] {
                            draw_text_cell(ui, lbl, w, 24.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("density_salt_list")
                        .max_height(280.0)
                        .show(&mut cols[0], |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                            for (i, salt) in DENSITY_SALTS.iter().enumerate() {
                                let is_sel = self.density_selected_salt_idx == Some(i);
                                ui.horizontal(|ui| {
                                    let r = draw_selectable_cell(
                                        ui,
                                        salt.component,
                                        120.0,
                                        CELL_H,
                                        egui::Color32::WHITE,
                                        HSC_DARK_TEXT,
                                        false,
                                        false,
                                        is_sel,
                                    );
                                    if r.clicked() {
                                        self.density_selected_salt_idx = Some(i);
                                    }
                                    if r.double_clicked() {
                                        self.density_rows.push((salt.component.to_string(), "0.05".into()));
                                        recalc_density = true;
                                    }
                                    let wmin = salt.w_min.map(|v| format!("{v:.4}")).unwrap_or_default();
                                    let wmax = salt.w_max.map(|v| format!("{v:.4}")).unwrap_or_default();
                                    let tmin = salt.t_min_c.map(|v| format!("{v:.0}")).unwrap_or_default();
                                    let tmax = salt.t_max_c.map(|v| format!("{v:.0}")).unwrap_or_default();
                                    draw_text_cell(ui, &wmin, 62.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, true);
                                    draw_text_cell(ui, &wmax, 62.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, true);
                                    draw_text_cell(ui, &tmin, 52.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                    draw_text_cell(ui, &tmax, 52.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, true, false);
                                });
                            }
                        });

                    // Right: Selected components & Validity violations (H6..K15)
                    cols[1].spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    cols[1].horizontal(|ui| {
                        for (lbl, w) in [
                            ("Selected components", 118.0),
                            ("Mass fraction", 92.0),
                            ("Violations (w)", 100.0),
                            ("Violations (T)", 96.0),
                        ] {
                            draw_text_cell(ui, lbl, w, 24.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
                        }
                    });
                    for ridx in 0..8 {
                        cols[1].horizontal(|ui| {
                            if ridx < self.density_rows.len() {
                                let (mut comp, mut wf) = self.density_rows[ridx].clone();
                                let is_sel = self.density_selected_entry_idx == Some(ridx);
                                let (_, r0, ch0, _) = draw_editable_cell(
                                    ui,
                                    &format!("den_c_{ridx}"),
                                    &mut comp,
                                    118.0,
                                    CELL_H,
                                    egui::Color32::WHITE,
                                    is_sel,
                                    false,
                                );
                                if r0.clicked() || r0.gained_focus() {
                                    self.density_selected_entry_idx = Some(ridx);
                                }
                                if ch0 {
                                    self.density_rows[ridx].0 = comp;
                                    recalc_density = true;
                                }

                                let (_, r1, ch1, _) = draw_editable_cell(
                                    ui,
                                    &format!("den_w_{ridx}"),
                                    &mut wf,
                                    92.0,
                                    CELL_H,
                                    egui::Color32::WHITE,
                                    false,
                                    true,
                                );
                                if r1.clicked() || r1.gained_focus() {
                                    self.density_selected_entry_idx = Some(ridx);
                                }
                                if ch1 {
                                    self.density_rows[ridx].1 = wf;
                                    recalc_density = true;
                                }

                                let viol = self.density_result.rows.get(ridx);
                                let mv = viol.and_then(|v| v.mass_violation.as_deref()).unwrap_or("");
                                let tv = viol.and_then(|v| v.temp_violation.as_deref()).unwrap_or("");
                                draw_text_cell(
                                    ui,
                                    mv,
                                    100.0,
                                    CELL_H,
                                    if mv.is_empty() { HSC_BEIGE_BG } else { HSC_VIOLATION_BG },
                                    HSC_DARK_TEXT,
                                    false,
                                    true,
                                    false,
                                );
                                draw_text_cell(
                                    ui,
                                    tv,
                                    96.0,
                                    CELL_H,
                                    if tv.is_empty() { HSC_BEIGE_BG } else { HSC_VIOLATION_BG },
                                    HSC_DARK_TEXT,
                                    false,
                                    true,
                                    false,
                                );
                            } else {
                                draw_text_cell(ui, "", 118.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, false);
                                draw_text_cell(ui, "", 92.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, false);
                                draw_text_cell(ui, "", 100.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, false);
                                draw_text_cell(ui, "", 96.0, CELL_H, HSC_BEIGE_BG, HSC_DARK_TEXT, false, false, false);
                            }
                        });
                    }
                    cols[1].add_space(6.0);
                    cols[1].label(
                        egui::RichText::new(format!(
                            "Remaining H2O mass fraction: {:.4}",
                            self.density_result.h2o_mass_fraction
                        ))
                        .size(11.0)
                        .color(HSC_MUTED_TEXT),
                    );
                });
            });
        self.show_density = open;
        if recalc_density {
            let t_c = self.density_temp_buf.trim().parse::<f64>().unwrap_or(25.0);
            self.density_result = recompute_density(&self.density_rows, t_c);
        }
    }

    fn render_plot_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_plot;
        egui::Window::new("Ionic AC vs Temperature (Fig. 10)")
            .open(&mut open)
            .default_size([720.0, 440.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Activity Coefficients (Molal Scale) vs T (0..40 °C)")
                            .strong()
                            .size(12.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Load Millero Seawater (Fig. 10)").clicked() {
                            self.load_preset(
                                AquaCaseFile::preset_millero_seawater(),
                                "Millero Seawater (Fig. 10)",
                                db,
                            );
                        }
                    });
                });
                ui.add_space(4.0);

                // Compute 9 points across 0..40 °C cleanly
                let mut ion_series: Vec<(String, Vec<[f64; 2]>)> = Vec::new();
                for (t_step, t_c) in (0..=40).step_by(5).enumerate() {
                    let mut req = self.case_file.clone();
                    req.temp_unit = AquaTempUnit::Celsius;
                    for s in &mut req.species {
                        s.temp = t_c as f64;
                    }
                    let res = SoryntelApplication::aqua_calculate(
                        db,
                        &req,
                        &self.main_db,
                        &self.own_db,
                        &self.excluded_keys,
                    );
                    for sp in res.species {
                        if sp.formula.eq_ignore_ascii_case("H2O") {
                            continue;
                        }
                        if t_step == 0 {
                            ion_series.push((sp.formula, vec![[t_c as f64, sp.ac_molal]]));
                        } else if let Some((_, pts)) =
                            ion_series.iter_mut().find(|(n, _)| n == &sp.formula)
                        {
                            pts.push([t_c as f64, sp.ac_molal]);
                        }
                    }
                }

                Plot::new("aqua_ionic_ac_plot")
                    .legend(Legend::default())
                    .x_axis_label("T / °C")
                    .y_axis_label("Ionic AC (Molal Sc.)")
                    .height(340.0)
                    .show(ui, |plot_ui| {
                        for (name, pts) in ion_series {
                            plot_ui.line(Line::new(PlotPoints::from(pts)).name(name));
                        }
                    });
            });
        self.show_plot = open;
    }

    fn render_find_modal(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_find;
        let mut replaced = false;
        egui::Window::new("Find / Replace")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("aqua_find_grid").spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label("Find what:");
                    ui.text_edit_singleline(&mut self.find_query);
                    ui.end_row();
                    ui.label("Replace with:");
                    ui.text_edit_singleline(&mut self.replace_query);
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    if ui.button("Find Next").clicked() && !self.find_query.is_empty() {
                        let q = self.find_query.to_ascii_lowercase();
                        for (idx, r) in self.rows.iter().enumerate() {
                            if r.formula.to_ascii_lowercase().contains(&q) {
                                self.select_cell(idx + 7, 0);
                                break;
                            }
                        }
                    }
                    if ui.button("Replace All").clicked() && !self.find_query.is_empty() {
                        for r in &mut self.rows {
                            if r.formula.contains(&self.find_query) {
                                r.formula = r.formula.replace(&self.find_query, &self.replace_query);
                                replaced = true;
                            }
                        }
                    }
                });
            });
        self.show_find = open;
        if replaced {
            self.recalculate(db);
        }
    }

    fn render_help_modal(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("Aqua Help")
            .open(&mut open)
            .default_size([480.0, 300.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("HSC Aqua — Aqueous Solutions").strong().size(14.0));
                ui.separator();
                ui.label("1. Enter aqueous species formulas in column A (e.g. H2O, Na(+a), SO4(-2a)), temperature in column B, and kmol amounts in column C.");
                ui.label("2. Click Calculate (blue play button) to evaluate Pitzer, Harvie–Møller–Weare, or Davies activity coefficients, osmotic coefficient, vapour pressure, pH, freezing/boiling points, and estimated H/Cp.");
                ui.label("3. Toggle Use OwnDB to give your editable OwnDB precedence over the built-in Main Pitzer Database.");
                ui.label("4. Open Density Calculator for 59-electrolyte solution density using Laliberté & Cooper (2004).");
            });
        self.show_help = open;
    }

    fn render_about_modal(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        egui::Window::new("About Soryntel Aqua")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Soryntel Chemistry® 10 — Aqua").strong().size(15.0));
                ui.label("Clean-room Rust implementation of Module 17 (Aqueous Electrolyte Solutions).");
                ui.label("Models: Pitzer–Harvie–Møller–Weare, Davies, Bradley–Pitzer Aφ, Appendix A p₀(T), Laliberté–Cooper (2004).");
            });
        self.show_about = open;
    }

    fn render_pitzer_doc_header(&self, ui: &mut egui::Ui, is_own: bool) {
        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                let sheet_name = match if is_own { self.own_db_tab } else { self.main_db_tab } {
                    PitzerSheetTab::Binary => "Pitzer Binaries: Cation - Anion Interactions",
                    PitzerSheetTab::Theta => "Theta: Cation - Cation & Anion - Anion Interactions",
                    PitzerSheetTab::Psi => "Psi: Ternary C-C-A & A-A-C Interactions",
                    PitzerSheetTab::Lambda => "Lambda: Neutral - Ion Interactions",
                };
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(sheet_name).strong().size(13.0));
                        if !is_own {
                            ui.label(
                                egui::RichText::new("Authors: Pertti Koukkari (98), Risto Pajarre (98), Justin Salminen (98, 09, 10), Hannu Sippola (98), Matias Hultgren (09, 10)")
                                    .size(10.5)
                                    .color(HSC_MUTED_TEXT),
                            );
                        }
                        ui.label(
                            egui::RichText::new("Temperature dependence:  a + b/T + c·ln(T) + d·T + e·T² + f·T⁻²")
                                .size(11.0)
                                .color(HSC_DARK_TEXT),
                        );
                    });
                    if !is_own {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Pitzer–Harvie Activity Coefficient Equation").strong().size(11.0));
                                ui.label(
                                    egui::RichText::new("ln γ_+,M = z_+² F + Σ_a m_a (2 B_Ma + Z C_Ma) + Σ_c m_c (2 Φ_Mc + Σ_a m_a ψ_Mca) + ...")
                                        .monospace()
                                        .size(10.0),
                                );
                                ui.label(
                                    egui::RichText::new("ln γ_-,X = z_-² F + Σ_c m_c (2 B_Xc + Z C_Xc) + Σ_a m_a (2 Φ_Xa + Σ_c m_c ψ_Xca) + ...")
                                        .monospace()
                                        .size(10.0),
                                );
                            });
                        });
                    }
                });
            });
        ui.add_space(4.0);
    }
}

// ---------------------------------------------------------------------------
// Compact, Zero-Duplication Grid, Tab, Ribbon & Painter Icon Helpers
// ---------------------------------------------------------------------------

fn rows_from_case(case: &AquaCaseFile) -> Vec<AquaSpeciesRowBuf> {
    case.species
        .iter()
        .map(|s| AquaSpeciesRowBuf {
            formula: s.formula.clone(),
            temp_buf: format_temp_display(s.temp),
            amount_buf: format_amount_display(s.amount_kmol),
        })
        .collect()
}

fn default_sample_own_db() -> OwnPitzerDb {
    OwnPitzerDb {
        binaries: vec![
            OwnBinaryRow {
                cation: "Ag(+a)".into(),
                anion: "Cl(-a)".into(),
                show_t_params: true,
                min_t_c: Some(25.0),
                max_t_c: Some(55.0),
                min_m: Some(0.0),
                max_m: Some(2.0),
                min_i: None,
                max_i: None,
                beta0: PitzerTempCurve::full(2.0, 0.0, 5.0, 0.0, 0.0, 0.0),
                beta1: PitzerTempCurve::EMPTY,
                beta2: PitzerTempCurve::EMPTY,
                c_phi: PitzerTempCurve::EMPTY,
            },
            OwnBinaryRow {
                cation: "Al(+3a)".into(),
                anion: "Cl(-a)".into(),
                show_t_params: false,
                min_t_c: Some(25.0),
                max_t_c: Some(55.0),
                min_m: None,
                max_m: None,
                min_i: Some(0.0),
                max_i: Some(5.0),
                beta0: PitzerTempCurve::constant(3.0),
                beta1: PitzerTempCurve::EMPTY,
                beta2: PitzerTempCurve::EMPTY,
                c_phi: PitzerTempCurve::EMPTY,
            },
        ],
        ..Default::default()
    }
}

fn recompute_density(rows: &[(String, String)], t_c: f64) -> DensityCalcResult {
    let inputs: Vec<DensityInputRow> = rows
        .iter()
        .filter(|(c, _)| !c.trim().is_empty())
        .map(|(c, w)| DensityInputRow {
            component: c.trim().to_string(),
            mass_fraction: w.trim().parse::<f64>().unwrap_or(0.0),
        })
        .collect();
    SoryntelApplication::aqua_density(&inputs, t_c)
}

fn format_temp_display(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{:.0}", v.round())
    } else {
        format!("{v:.2}")
    }
}

fn format_amount_display(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if v.abs() >= 10.0 || v.abs() < 1e-2 {
        format_hsc_sci(v)
    } else {
        format!("{v:.4}")
    }
}

fn format_sci_or_dec(v: f64, dec: usize) -> String {
    if !v.is_finite() {
        return String::new();
    }
    if v.abs() >= 1000.0 || (v.abs() > 0.0 && v.abs() < 0.01) {
        format_hsc_sci(v)
    } else {
        format!("{v:.prec$}", prec = dec)
    }
}

fn format_hsc_sci(v: f64) -> String {
    if v == 0.0 {
        return "0.00".into();
    }
    let abs = v.abs();
    let exp = abs.log10().floor() as i32;
    let mant = v / 10_f64.powi(exp);
    format!("{mant:.2}E{exp:+03}")
}

fn aqua_column_widths(total_cols: usize) -> Vec<f32> {
    let mut w = vec![78.0; total_cols];
    w[0] = 116.0; // Species
    w[1] = 54.0; // Temp
    w[2] = 74.0; // kmol
    w[3] = 62.0; // mol-%
    w[4] = 74.0; // g
    for i in 13..total_cols.min(24) {
        w[i] = 82.0;
    }
    w
}

fn draw_win_groupbox(ui: &mut egui::Ui, title: &str, height: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let w = ui.available_width().max(240.0);
    let (outer, _) = ui.allocate_exact_size(egui::vec2(w, height + 8.0), egui::Sense::hover());
    let box_rect = egui::Rect::from_min_max(egui::pos2(outer.min.x, outer.min.y + 7.0), outer.max);

    if ui.is_rect_visible(outer) {
        let p = ui.painter();
        p.rect_stroke(box_rect, 2.5, egui::Stroke::new(1.0, HSC_GROUP_BORDER));
        let hi = egui::Color32::from_rgb(249, 249, 249);
        p.line_segment(
            [
                egui::pos2(box_rect.min.x + 2.0, box_rect.max.y - 1.0),
                egui::pos2(box_rect.max.x - 2.0, box_rect.max.y - 1.0),
            ],
            egui::Stroke::new(1.0, hi),
        );
        let title_font = egui::FontId::proportional(11.5);
        let galley = p.layout_no_wrap(title.to_string(), title_font.clone(), HSC_DARK_TEXT);
        let pill = egui::Rect::from_min_size(
            egui::pos2(box_rect.min.x + 8.0, box_rect.min.y - 7.0),
            egui::vec2(galley.size().x + 8.0, 14.0),
        );
        p.rect_filled(pill, 0.0, HSC_WIN_BG);
        p.text(
            egui::pos2(pill.min.x + 4.0, box_rect.min.y),
            egui::Align2::LEFT_CENTER,
            title,
            title_font,
            HSC_DARK_TEXT,
        );
    }

    let inner = egui::Rect::from_min_max(
        egui::pos2(box_rect.min.x + 12.0, box_rect.min.y + 11.0),
        egui::pos2(box_rect.max.x - 10.0, box_rect.max.y - 6.0),
    );
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), add_contents);
}

fn draw_win_square_check(ui: &mut egui::Ui, label: &str, checked: bool) -> egui::Response {
    let font = egui::FontId::proportional(11.5);
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), HSC_DARK_TEXT));
    let total_w = 13.0 + 5.0 + galley.size().x + 4.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(total_w, 18.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let box_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 1.0, rect.center().y - 6.5),
            egui::vec2(13.0, 13.0),
        );
        p.rect_filled(box_rect, 0.0, egui::Color32::WHITE);
        let border_col = if resp.hovered() {
            egui::Color32::from_rgb(0, 102, 204)
        } else {
            egui::Color32::from_rgb(112, 112, 112)
        };
        p.rect_stroke(box_rect, 0.0, egui::Stroke::new(1.0, border_col));
        if checked {
            let stroke = egui::Stroke::new(1.6, egui::Color32::from_rgb(24, 26, 30));
            let c1 = egui::pos2(box_rect.min.x + 2.8, box_rect.center().y + 0.2);
            let c2 = egui::pos2(box_rect.min.x + 5.3, box_rect.max.y - 3.0);
            let c3 = egui::pos2(box_rect.max.x - 2.6, box_rect.min.y + 3.0);
            p.line_segment([c1, c2], stroke);
            p.line_segment([c2, c3], stroke);
        }
        p.text(
            egui::pos2(box_rect.max.x + 5.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            HSC_DARK_TEXT,
        );
    }
    resp
}

fn draw_classic_bevel_button(ui: &mut egui::Ui, label: &str, w: f32, h: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let (top_fill, bot_fill, border_col) = if resp.is_pointer_button_down_on() {
            (
                egui::Color32::from_rgb(216, 228, 240),
                egui::Color32::from_rgb(206, 220, 235),
                egui::Color32::from_rgb(0, 84, 153),
            )
        } else if resp.hovered() {
            (
                egui::Color32::from_rgb(236, 244, 252),
                egui::Color32::from_rgb(222, 236, 249),
                egui::Color32::from_rgb(60, 127, 177),
            )
        } else {
            (
                egui::Color32::from_rgb(245, 245, 245),
                egui::Color32::from_rgb(225, 225, 225),
                egui::Color32::from_rgb(172, 172, 172),
            )
        };
        p.rect_filled(rect, 2.0, bot_fill);
        let top_half = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.center().y));
        p.rect_filled(top_half, 2.0, top_fill);
        p.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, border_col));
        p.rect_stroke(
            rect.shrink(1.0),
            1.5,
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 190)),
        );
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.5),
            HSC_DARK_TEXT,
        );
    }
    resp
}

fn draw_gutter_stack_cell(
    ui: &mut egui::Ui,
    nums: &[&str],
    w: f32,
    total_h: f32,
    selected: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, total_h), egui::Sense::hover());
    if !ui.is_rect_visible(rect) || nums.is_empty() {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, row_num_bg(selected));
    p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
    let sub_h = total_h / nums.len() as f32;
    for (i, n) in nums.iter().enumerate() {
        let y0 = rect.min.y + sub_h * i as f32;
        if i > 0 {
            p.line_segment(
                [egui::pos2(rect.min.x, y0), egui::pos2(rect.max.x, y0)],
                egui::Stroke::new(0.7, egui::Color32::from_rgb(202, 206, 214)),
            );
        }
        p.text(
            egui::pos2(rect.center().x, y0 + sub_h * 0.5),
            egui::Align2::CENTER_CENTER,
            *n,
            egui::FontId::proportional(10.5),
            HSC_DARK_TEXT,
        );
    }
}

fn draw_merged_hsc_header(ui: &mut egui::Ui, w: f32, total_h: f32, title: &str, unit: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, total_h), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, HSC_BANNER_BG);
    p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(138, 138, 138)));

    let lines: Vec<&str> = title.lines().collect();
    if lines.is_empty() && unit.is_empty() {
        return;
    }
    let line_h = 12.5;
    let title_band_h = if unit.is_empty() {
        rect.height()
    } else {
        (rect.height() - 16.0).max(16.0)
    };
    let start_y = rect.min.y + (title_band_h - (lines.len() as f32 - 1.0) * line_h) * 0.5;
    let clipped = p.with_clip_rect(rect.shrink2(egui::vec2(2.0, 1.0)));
    for (i, line) in lines.iter().enumerate() {
        clipped.text(
            egui::pos2(rect.center().x, start_y + i as f32 * line_h),
            egui::Align2::CENTER_CENTER,
            *line,
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(15, 17, 21),
        );
    }
    if !unit.is_empty() {
        clipped.text(
            egui::pos2(rect.center().x, rect.max.y - 8.5),
            egui::Align2::CENTER_CENTER,
            unit,
            egui::FontId::proportional(10.5),
            HSC_DARK_TEXT,
        );
    }
}

fn draw_elements_top_cell(ui: &mut egui::Ui, text: &str, w: f32, h: f32, first_col: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, HSC_BANNER_BG);
    let border = egui::Stroke::new(1.0, egui::Color32::from_rgb(138, 138, 138));
    p.line_segment([rect.left_top(), rect.right_top()], border);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], border);
    if first_col {
        p.line_segment([rect.left_top(), rect.left_bottom()], border);
    }
    if !text.is_empty() {
        p.text(
            rect.left_center() + egui::vec2(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(11.0),
            egui::Color32::from_rgb(15, 17, 21),
        );
    }
}

fn paint_sheet_rule(ui: &mut egui::Ui, x0: f32, x1: f32, y: f32) {
    if x1 > x0 {
        ui.painter().line_segment(
            [egui::pos2(x0, y), egui::pos2(x1, y)],
            egui::Stroke::new(1.5, HSC_RULE_DARK),
        );
    }
}

fn row_num_bg(selected: bool) -> egui::Color32 {
    if selected {
        HSC_SEL_COL_BG
    } else {
        EXCEL_HEADER_BG
    }
}

fn row_num_cell_hi(ui: &mut egui::Ui, row_num: usize, selected: bool) {
    if selected {
        draw_text_cell(
            ui,
            &row_num.to_string(),
            32.0,
            CELL_H,
            HSC_SEL_COL_BG,
            HSC_DARK_TEXT,
            true,
            true,
            false,
        );
    } else {
        row_num_cell(ui, row_num, 32.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text_cell(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    center: bool,
    right: bool,
) {
    let align = if center {
        CellAlign::Center
    } else if right {
        CellAlign::Right
    } else {
        CellAlign::Left
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    paint_cell_interior(ui, rect, text, bg, fg, bold, align, false);
}

#[allow(clippy::too_many_arguments)]
fn draw_selectable_cell_align(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    align: CellAlign,
    selected: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    paint_cell_interior(ui, rect, text, bg, fg, bold, align, selected);
    resp
}

#[allow(clippy::too_many_arguments)]
fn draw_selectable_cell(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    right: bool,
    selected: bool,
) -> egui::Response {
    let align = if right { CellAlign::Right } else { CellAlign::Left };
    draw_selectable_cell_align(ui, text, w, h, bg, fg, bold, align, selected)
}

#[allow(clippy::too_many_arguments)]
fn paint_cell_interior(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: &str,
    bg: egui::Color32,
    fg: egui::Color32,
    bold: bool,
    align: CellAlign,
    selected: bool,
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, bg);
    let border = if selected {
        egui::Stroke::new(1.8, HSC_DARK_TEXT)
    } else {
        egui::Stroke::new(1.0, HSC_SHEET_GRID)
    };
    p.rect_stroke(rect, 0.0, border);
    if selected {
        let h_rect = egui::Rect::from_min_size(rect.right_bottom() - egui::vec2(4.0, 4.0), egui::vec2(4.0, 4.0));
        p.rect_filled(h_rect, 0.0, HSC_DARK_TEXT);
    }
    if !text.is_empty() {
        let clipped = p.with_clip_rect(rect.shrink2(egui::vec2(4.0, 1.0)));
        let _ = bold;
        let (pos, align2) = match align {
            CellAlign::Center => (rect.center(), egui::Align2::CENTER_CENTER),
            CellAlign::Right => (rect.right_center() - egui::vec2(5.0, 0.0), egui::Align2::RIGHT_CENTER),
            CellAlign::Left => (rect.left_center() + egui::vec2(5.0, 0.0), egui::Align2::LEFT_CENTER),
        };
        clipped.text(pos, align2, text, egui::FontId::proportional(11.0), fg);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_editable_cell_align(
    ui: &mut egui::Ui,
    id: &str,
    buf: &mut String,
    w: f32,
    h: f32,
    bg: egui::Color32,
    selected: bool,
    align_mode: CellAlign,
) -> (egui::Rect, egui::Response, bool, bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, bg);
    p.rect_stroke(
        rect,
        0.0,
        if selected {
            egui::Stroke::new(1.8, HSC_DARK_TEXT)
        } else {
            egui::Stroke::new(1.0, HSC_SHEET_GRID)
        },
    );
    if selected {
        let h_rect = egui::Rect::from_min_size(rect.right_bottom() - egui::vec2(4.0, 4.0), egui::vec2(4.0, 4.0));
        p.rect_filled(h_rect, 0.0, HSC_DARK_TEXT);
    }
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(1.0)));
    let align = match align_mode {
        CellAlign::Left => egui::Align::LEFT,
        CellAlign::Center => egui::Align::Center,
        CellAlign::Right => egui::Align::RIGHT,
    };
    let resp = child.add(
        egui::TextEdit::singleline(buf)
            .id(egui::Id::new(id))
            .desired_width(w - 4.0)
            .frame(false)
            .horizontal_align(align)
            .text_color(HSC_DARK_TEXT)
            .font(egui::FontId::proportional(11.0)),
    );
    let ch = resp.changed();
    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    (rect, resp, ch, enter)
}

#[allow(clippy::too_many_arguments)]
fn draw_editable_cell(
    ui: &mut egui::Ui,
    id: &str,
    buf: &mut String,
    w: f32,
    h: f32,
    bg: egui::Color32,
    selected: bool,
    right: bool,
) -> (egui::Rect, egui::Response, bool, bool) {
    let align_mode = if right { CellAlign::Right } else { CellAlign::Left };
    draw_editable_cell_align(ui, id, buf, w, h, bg, selected, align_mode)
}

fn source_color_rgb(c: PitzerSourceColor) -> egui::Color32 {
    match c {
        PitzerSourceColor::MainBlack => egui::Color32::BLACK,
        PitzerSourceColor::OwnBlue => egui::Color32::from_rgb(29, 78, 216),
        PitzerSourceColor::BothGreen => egui::Color32::from_rgb(21, 128, 61),
        PitzerSourceColor::ExcludedRed => egui::Color32::from_rgb(185, 28, 28),
    }
}

fn draw_pitzer_table_header(ui: &mut egui::Ui, key_cols: &[&str], show_check: bool, detailed_x: bool) {
    ui.horizontal(|ui| {
        let key_w = if key_cols.len() == 3 { 66.0 } else { 92.0 };
        for &kc in key_cols {
            draw_text_cell(ui, kc, key_w, 28.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
        }
        if show_check {
            draw_text_cell(ui, "Active", 48.0, 28.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
        }
        if detailed_x {
            for (lbl, w) in [
                ("Min T", 50.0),
                ("Max T", 50.0),
                ("Min m", 50.0),
                ("Max m", 50.0),
                ("Min I", 48.0),
                ("Max I", 48.0),
                ("b0:a", 34.0),
                ("b0:b", 34.0),
                ("b0:c", 34.0),
                ("b0:d", 34.0),
                ("b0:e", 34.0),
                ("b0:f", 34.0),
                ("b(1)", 42.0),
                ("b(2)", 42.0),
                ("C(f)", 42.0),
            ] {
                draw_text_cell(ui, lbl, w, 28.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
            }
        } else {
            for (lbl, w) in [
                ("AC", 34.0),
                ("H", 34.0),
                ("Cp", 34.0),
                ("Min T", 52.0),
                ("Max T", 52.0),
                ("Range", 92.0),
                ("b0", 30.0),
                ("b1", 30.0),
                ("b2", 30.0),
                ("Cf", 30.0),
                ("H:b0", 34.0),
                ("Cp:b0", 36.0),
            ] {
                draw_text_cell(ui, lbl, w, 28.0, HSC_BANNER_BG, HSC_DARK_TEXT, true, true, false);
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_pitzer_status_or_x_cells(
    ui: &mut egui::Ui,
    b0: &PitzerTempCurve,
    b1: &PitzerTempCurve,
    b2: &PitzerTempCurve,
    cf: &PitzerTempCurve,
    range: &soryntel_thermo::aqua_pitzer_data::ValidityRange,
    mode: MainDbViewMode,
    fg: egui::Color32,
) {
    let tmin = range.min_t_c.map(|v| format!("{v:.0}")).unwrap_or_else(|| "*".into());
    let tmax = range.max_t_c.map(|v| format!("{v:.0}")).unwrap_or_else(|| "*".into());
    let mmin = range.min_m.map(|v| format!("{v:.1}")).unwrap_or_default();
    let mmax = range.max_m.map(|v| format!("{v:.1}")).unwrap_or_default();
    let imin = range.min_i.map(|v| format!("{v:.1}")).unwrap_or_default();
    let imax = range.max_i.map(|v| format!("{v:.1}")).unwrap_or_default();

    if mode == MainDbViewMode::DetailedX {
        for (txt, w) in [
            (tmin, 50.0),
            (tmax, 50.0),
            (mmin, 50.0),
            (mmax, 50.0),
            (imin, 48.0),
            (imax, 48.0),
        ] {
            draw_text_cell(ui, &txt, w, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
        }
        let coeff_present = [
            b0.a != 0.0 || b0.defined,
            b0.b.abs() > 1e-18,
            b0.c.abs() > 1e-18,
            b0.d.abs() > 1e-18,
            b0.e.abs() > 1e-18,
            b0.f.abs() > 1e-18,
        ];
        for present in coeff_present {
            draw_text_cell(
                ui,
                if present { "X" } else { "" },
                34.0,
                CELL_H,
                HSC_BEIGE_BG,
                fg,
                false,
                true,
                false,
            );
        }
        for c in [b1, b2, cf] {
            draw_text_cell(ui, c.status_flag(), 42.0, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
        }
    } else {
        let ac_f = b0.status_flag();
        let h_f = if b0.has_t_dep() { "T" } else { "*" };
        let range_lbl = match (range.max_m, range.max_i) {
            (Some(mm), _) => format!("m < {}", format_compact(mm)),
            (_, Some(ii)) => format!("I < {}", format_compact(ii)),
            _ => "*".into(),
        };
        for (txt, w) in [
            (ac_f, 34.0),
            (h_f, 34.0),
            (h_f, 34.0),
            (tmin.as_str(), 52.0),
            (tmax.as_str(), 52.0),
            (range_lbl.as_str(), 92.0),
            (b0.status_flag(), 30.0),
            (b1.status_flag(), 30.0),
            (b2.status_flag(), 30.0),
            (cf.status_flag(), 30.0),
            (h_f, 34.0),
            (h_f, 36.0),
        ] {
            draw_text_cell(ui, txt, w, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
        }
    }
}

fn draw_single_curve_cells(
    ui: &mut egui::Ui,
    curve: &PitzerTempCurve,
    range: &soryntel_thermo::aqua_pitzer_data::ValidityRange,
    fg: egui::Color32,
) {
    let f = curve.status_flag();
    let h_f = if curve.has_t_dep() { "T" } else { "*" };
    let tmin = range.min_t_c.map(|v| format!("{v:.0}")).unwrap_or_else(|| "*".into());
    let tmax = range.max_t_c.map(|v| format!("{v:.0}")).unwrap_or_else(|| "*".into());
    let r = range
        .max_i
        .map(|ii| format!("I < {}", format_compact(ii)))
        .unwrap_or_else(|| "*".into());
    for (txt, w) in [
        (f, 34.0),
        (h_f, 34.0),
        (h_f, 34.0),
        (tmin.as_str(), 52.0),
        (tmax.as_str(), 52.0),
        (r.as_str(), 92.0),
    ] {
        draw_text_cell(ui, txt, w, CELL_H, HSC_BEIGE_BG, fg, false, true, false);
    }
}

fn pitzer_4tab_bar(ui: &mut egui::Ui, active: &mut PitzerSheetTab) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (tab, label) in [
            (PitzerSheetTab::Binary, "Binary parameters"),
            (PitzerSheetTab::Theta, "Theta parameters"),
            (PitzerSheetTab::Psi, "Psi parameters"),
            (PitzerSheetTab::Lambda, "Lambda parameters"),
        ] {
            ui.selectable_value(active, tab, label);
        }
    });
}

fn format_compact(v: f64) -> String {
    if !v.is_finite() {
        return String::new();
    }
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e9 {
        format!("{:.0}", v.round())
    } else {
        format!("{v}")
    }
}

fn numeric_buf_cell(
    ui: &mut egui::Ui,
    id: &str,
    initial: String,
    w: f32,
    enabled: bool,
) -> Option<(String, bool)> {
    if !enabled {
        draw_text_cell(ui, "", w, CELL_H, HSC_BEIGE_BG, HSC_MUTED_TEXT, false, true, false);
        return None;
    }
    let buf_id = egui::Id::new(format!("{id}_editbuf"));
    let mut s: String = ui
        .ctx()
        .data(|d| d.get_temp::<String>(buf_id))
        .unwrap_or(initial);
    let (_, resp, ch, _) =
        draw_editable_cell(ui, id, &mut s, w, CELL_H, egui::Color32::WHITE, false, true);
    if resp.has_focus() {
        ui.ctx().data_mut(|d| d.insert_temp(buf_id, s.clone()));
    } else if resp.lost_focus() {
        ui.ctx().data_mut(|d| d.remove::<String>(buf_id));
    }
    Some((s, ch))
}

fn opt_f64_cell(ui: &mut egui::Ui, id: &str, opt: &mut Option<f64>, w: f32, enabled: bool) -> bool {
    let initial = opt.map(format_compact).unwrap_or_default();
    let Some((s, ch)) = numeric_buf_cell(ui, id, initial, w, enabled) else {
        return false;
    };
    if ch {
        *opt = if s.trim().is_empty() {
            None
        } else {
            s.trim().parse::<f64>().ok()
        };
    }
    ch
}

fn f64_cell(ui: &mut egui::Ui, id: &str, val: &mut f64, w: f32, enabled: bool) -> bool {
    let initial = if val.abs() < 1e-18 {
        String::new()
    } else {
        format_compact(*val)
    };
    let Some((s, ch)) = numeric_buf_cell(ui, id, initial, w, enabled) else {
        return false;
    };
    if ch {
        if let Ok(v) = s.trim().parse::<f64>() {
            *val = v;
        } else if s.trim().is_empty() {
            *val = 0.0;
        }
    }
    ch
}

fn ribbon_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.set_min_height(64.0);
        ui.horizontal(add_contents);
        ui.add_space(2.0);
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new(title).size(9.5).color(HSC_MUTED_TEXT));
        });
    });
    let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(2.0, 62.0), egui::Sense::hover());
    ui.painter()
        .line_segment([sep_rect.center_top(), sep_rect.center_bottom()], egui::Stroke::new(1.0, HSC_RIBBON_BORDER));
}

fn large_ribbon_btn(
    ui: &mut egui::Ui,
    icon: RibbonIcon,
    label: &str,
    sub_caret: Option<&str>,
    active: bool,
) -> egui::Response {
    let w = if label.len() > 11 && !label.contains('\n') {
        78.0
    } else {
        60.0
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 50.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if active {
            p.rect_filled(rect, 3.0, HSC_OWNDB_ACTIVE_BG);
            p.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, HSC_OWNDB_ACTIVE_BORDER));
        } else if resp.hovered() {
            p.rect_filled(rect, 3.0, egui::Color32::from_rgb(232, 240, 252));
            p.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, EXCEL_ACTIVE_BORDER));
        }
        let icon_center = egui::pos2(rect.center().x, rect.min.y + 15.0);
        paint_ribbon_icon(p, icon_center, icon, 11.0);
        let text_y = rect.min.y + 34.0;
        p.text(
            egui::pos2(rect.center().x, text_y),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            HSC_DARK_TEXT,
        );
        if let Some(c) = sub_caret {
            p.text(
                egui::pos2(rect.center().x, rect.max.y - 5.0),
                egui::Align2::CENTER_CENTER,
                c,
                egui::FontId::proportional(8.0),
                HSC_MUTED_TEXT,
            );
        }
    }
    resp
}

fn small_ribbon_btn(ui: &mut egui::Ui, icon: RibbonIcon, label: &str) -> egui::Response {
    let w = (label.len() as f32 * 5.8 + 26.0).max(58.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 22.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 2.5, egui::Color32::from_rgb(232, 240, 252));
            p.rect_stroke(rect, 2.5, egui::Stroke::new(1.0, EXCEL_ACTIVE_BORDER));
        }
        paint_ribbon_icon(p, egui::pos2(rect.min.x + 11.0, rect.center().y), icon, 7.0);
        p.text(
            egui::pos2(rect.min.x + 21.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(10.5),
            HSC_DARK_TEXT,
        );
    }
    resp
}

fn paint_ribbon_icon(p: &egui::Painter, c: egui::Pos2, icon: RibbonIcon, r: f32) {
    let blue = egui::Color32::from_rgb(56, 130, 214);
    let gold = egui::Color32::from_rgb(217, 165, 32);
    let dark = egui::Color32::from_rgb(70, 82, 98);
    match icon {
        RibbonIcon::File => {
            p.rect_filled(egui::Rect::from_center_size(c, egui::vec2(1.4 * r, 1.8 * r)), 1.5, egui::Color32::WHITE);
            p.rect_stroke(egui::Rect::from_center_size(c, egui::vec2(1.4 * r, 1.8 * r)), 1.5, egui::Stroke::new(1.2, dark));
        }
        RibbonIcon::Calculate => {
            let pts = vec![
                c + egui::vec2(-0.55 * r, -0.75 * r),
                c + egui::vec2(0.8 * r, 0.0),
                c + egui::vec2(-0.55 * r, 0.75 * r),
            ];
            p.add(egui::Shape::convex_polygon(pts, blue, egui::Stroke::new(1.0, dark)));
        }
        RibbonIcon::Gears => {
            p.circle_filled(c + egui::vec2(-0.3 * r, -0.2 * r), 0.48 * r, egui::Color32::from_rgb(160, 175, 195));
            p.circle_stroke(c + egui::vec2(-0.3 * r, -0.2 * r), 0.48 * r, egui::Stroke::new(1.2, dark));
            p.circle_filled(c + egui::vec2(0.45 * r, 0.35 * r), 0.38 * r, egui::Color32::from_rgb(190, 204, 222));
            p.circle_stroke(c + egui::vec2(0.45 * r, 0.35 * r), 0.38 * r, egui::Stroke::new(1.2, dark));
        }
        RibbonIcon::UnitsGrid => {
            let rect = egui::Rect::from_center_size(c, egui::vec2(1.6 * r, 1.4 * r));
            p.rect_filled(rect, 1.0, egui::Color32::WHITE);
            p.rect_stroke(rect, 1.0, egui::Stroke::new(1.1, dark));
            p.line_segment([rect.center_top(), rect.center_bottom()], egui::Stroke::new(0.9, blue));
            p.line_segment([rect.left_center(), rect.right_center()], egui::Stroke::new(0.9, blue));
        }
        RibbonIcon::UseOwnDb | RibbonIcon::MainDb | RibbonIcon::ParamSets | RibbonIcon::OwnDb => {
            for dy in [-0.45 * r, 0.0, 0.45 * r] {
                let d_rect = egui::Rect::from_center_size(c + egui::vec2(0.0, dy), egui::vec2(1.4 * r, 0.55 * r));
                let fill = if icon == RibbonIcon::UseOwnDb { blue } else { gold };
                p.rect_filled(d_rect, 2.5, fill);
                p.rect_stroke(d_rect, 2.5, egui::Stroke::new(0.9, egui::Color32::from_rgb(130, 92, 16)));
            }
        }
        RibbonIcon::DensityFx => {
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                "f(x)",
                egui::FontId::monospace(14.0),
                HSC_DARK_TEXT,
            );
        }
        RibbonIcon::PlotCurve => {
            let rect = egui::Rect::from_center_size(c, egui::vec2(1.6 * r, 1.4 * r));
            p.rect_stroke(rect, 1.0, egui::Stroke::new(1.1, dark));
            let pts = vec![
                rect.left_bottom() + egui::vec2(2.0, -3.0),
                c + egui::vec2(-2.0, 1.0),
                c + egui::vec2(3.0, -3.0),
                rect.right_top() + egui::vec2(-2.0, 2.0),
            ];
            p.add(egui::Shape::line(pts, egui::Stroke::new(1.6, HSC_BRAND_RED)));
        }
        RibbonIcon::Help | RibbonIcon::About => {
            p.circle_filled(c, 0.9 * r, blue);
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                if icon == RibbonIcon::Help { "?" } else { "i" },
                egui::FontId::proportional(10.0),
                egui::Color32::WHITE,
            );
        }
        RibbonIcon::Cut => {
            p.line_segment([c - egui::vec2(r, r * 0.7), c + egui::vec2(r, r * 0.7)], egui::Stroke::new(1.3, dark));
            p.line_segment([c - egui::vec2(r, -r * 0.7), c + egui::vec2(r, -r * 0.7)], egui::Stroke::new(1.3, blue));
        }
        RibbonIcon::Copy | RibbonIcon::Paste => {
            let rect = egui::Rect::from_center_size(c, egui::vec2(1.4 * r, 1.6 * r));
            p.rect_filled(rect, 1.0, if icon == RibbonIcon::Paste { gold } else { egui::Color32::WHITE });
            p.rect_stroke(rect, 1.0, egui::Stroke::new(1.1, dark));
        }
        RibbonIcon::Find => {
            p.circle_stroke(c - egui::vec2(0.2 * r, 0.1 * r), 0.65 * r, egui::Stroke::new(1.4, blue));
            p.line_segment([c + egui::vec2(0.2 * r, 0.3 * r), c + egui::vec2(0.9 * r, 0.9 * r)], egui::Stroke::new(1.6, dark));
        }
    }
}
