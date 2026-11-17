//! Module 06 — HSC Estimate UI (Figs. 1–9 + Cp Range, HSC Cp Data & Database Save workflow).

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, text_cell, widget_cell, EXCEL_GRID_BORDER,
    EXCEL_HEADER_BORDER, EXCEL_READONLY_BG,
};
use crate::ui_common::temp_file_path;
use eframe::egui;
use egui_plot::{Bar, BarChart, Legend, Line, Plot, PlotPoints, Points};
use serde::{Deserialize, Serialize};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::estimate::{
    build_estimated_species_with_intervals, classic_hsc_estimate_rows, compute_parity_stats,
    default_fig1_rows, evaluate_estimate_row, fig5_oxides_rows, fit_kelley_cp, generate_cp_curve,
    parse_estimate_formula, EstimateRow, EstimateSpeciesType, FunctionalGroup, OrganicGroup,
};
use soryntel_thermo::{
    evaluate_species_si, ChemicalSpecies, KelleyCoefficients, Phase, SpeciesCatalogPort,
    SpeciesMetadata, SpeciesRepository, SpeciesThermoInterval,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatePlotMode {
    Parity,
    ParityIndividual,
    Comparison,
    Differences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimateScatterScope {
    SheetRows,
    SelectedRows,
    GasDb,
    AqueousDb,
    CondensedDb,
    AllDb,
}

#[derive(Debug, Clone)]
pub struct CpFittedRange {
    pub tmin_k: f64,
    pub tmax_k: f64,
    pub kelley: KelleyCoefficients,
}

#[derive(Debug, Clone)]
pub struct DbReviewDraft {
    pub open: bool,
    pub formula: String,
    pub chemical_name: String,
    pub structural_formula: String,
    pub common_name: String,
    pub cas_number: String,
    pub phase: Phase,
    pub raw_phase_code: String,
    pub mw: f64,
    pub density_g_cm3: f64,
    pub charge: i32,
    pub h298_kj_mol: f64,
    pub s298_j_molk: f64,
    pub cp298_j_molk: f64,
    pub dg298_kj_mol: f64,
    pub reference: String,
    pub reliability_class: i16,
    pub intervals: Vec<SpeciesThermoInterval>,
}

impl Default for DbReviewDraft {
    fn default() -> Self {
        Self {
            open: false,
            formula: String::new(),
            chemical_name: String::new(),
            structural_formula: String::new(),
            common_name: String::new(),
            cas_number: String::new(),
            phase: Phase::Solid,
            raw_phase_code: "s".to_string(),
            mw: 0.0,
            density_g_cm3: 3.0,
            charge: 0,
            h298_kj_mol: 0.0,
            s298_j_molk: 50.0,
            cp298_j_molk: 40.0,
            dg298_kj_mol: 0.0,
            reference: "HSC Estimate".to_string(),
            reliability_class: 4,
            intervals: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EstimateSheetFile {
    rows: Vec<EstimateRow>,
}

pub struct EstimateModuleState {
    pub rows: Vec<EstimateRow>,
    pub selected_row: Option<usize>,
    pub selected_rows: BTreeSet<usize>,
    pub clipboard_rows: Vec<EstimateRow>,
    pub show_type_column: bool,
    pub freeze_col_a: bool,
    pub freeze_title_rows: bool,
    pub use_kelvin: bool,
    pub per_kg: bool,
    pub h_scroll_offset: f32,
    pub v_scroll_offset: f32,
    pub show_paste_modal: bool,
    pub paste_text_buf: String,
    pub paste_target_row: usize,
    pub show_db_picker: bool,
    pub db_search: String,
    pub show_graphics_win: bool,
    pub plot_mode: EstimatePlotMode,
    pub scatter_scope: EstimateScatterScope,
    pub cached_db_scope: Option<EstimateScatterScope>,
    pub cached_db_rows: Vec<EstimateRow>,
    pub active_stat_prop: usize, // 0=H, 1=S, 2=Cp, 3=dG
    pub show_chart_properties: bool,
    pub reset_plot_counter: u64,
    pub show_cp_range_dialog: bool,
    pub cp_tmin_k: f64,
    pub cp_tmax_k: f64,
    pub cp_steps: usize,
    pub show_cp_data_win: bool,
    pub cp_points: Vec<(f64, f64)>,
    pub cp_exp_points: Vec<Option<f64>>,
    pub cp_ranges: Vec<CpFittedRange>,
    pub cp_active_range: usize,
    pub cp_kelley: KelleyCoefficients,
    pub cp_species_type: EstimateSpeciesType,
    pub cp_org_group: OrganicGroup,
    pub cp_target_row: usize,
    pub db_review: DbReviewDraft,
    pub own_db_path: String,
    pub sheet_file_path: String,
    pub show_file_io_modal: bool,
    pub file_io_save_mode: bool,
    pub show_help_modal: bool,
    pub show_about_modal: bool,
    pub quick_species: String,
    pub quick_temp: f64,
    pub quick_is_kelvin: bool,
    pub status_msg: String,
}

impl EstimateModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let mut rows = default_fig1_rows();
        for r in &mut rows {
            evaluate_estimate_row(r, db);
        }
        let own_db_path = db
            .status_summary()
            .own_path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| temp_file_path("soryntel_owndb6.HSC"));

        let mut selected_rows = BTreeSet::new();
        selected_rows.insert(0);

        Self {
            rows,
            selected_row: Some(0),
            selected_rows,
            clipboard_rows: Vec::new(),
            show_type_column: true,
            freeze_col_a: false,
            freeze_title_rows: true,
            use_kelvin: false,
            per_kg: false,
            h_scroll_offset: 0.0,
            v_scroll_offset: 0.0,
            show_paste_modal: false,
            paste_text_buf: String::new(),
            paste_target_row: 0,
            show_db_picker: false,
            db_search: String::new(),
            show_graphics_win: false,
            plot_mode: EstimatePlotMode::Parity,
            scatter_scope: EstimateScatterScope::SheetRows,
            cached_db_scope: None,
            cached_db_rows: Vec::new(),
            active_stat_prop: 1,
            show_chart_properties: true,
            reset_plot_counter: 0,
            show_cp_range_dialog: false,
            cp_tmin_k: 298.15,
            cp_tmax_k: 600.0,
            cp_steps: 50,
            show_cp_data_win: false,
            cp_points: Vec::new(),
            cp_exp_points: Vec::new(),
            cp_ranges: vec![CpFittedRange {
                tmin_k: 298.15,
                tmax_k: 600.0,
                kelley: KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 },
            }],
            cp_active_range: 0,
            cp_kelley: KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 },
            cp_species_type: EstimateSpeciesType::NotSpecified,
            cp_org_group: OrganicGroup::None,
            cp_target_row: 0,
            db_review: DbReviewDraft::default(),
            own_db_path,
            sheet_file_path: temp_file_path("soryntel_estimate_sheet.est8"),
            show_file_io_modal: false,
            file_io_save_mode: true,
            show_help_modal: false,
            show_about_modal: false,
            quick_species: "POP".to_string(),
            quick_temp: 25.0,
            quick_is_kelvin: false,
            status_msg: "Listo · HSC Estimate (Neumann-Kopp, Latimer, Benson/Joback)".to_string(),
        }
    }

    fn recalc_all(&mut self, db: &ThermoDatabase) {
        for r in &mut self.rows {
            evaluate_estimate_row(r, db);
        }
    }

    fn select_single(&mut self, idx: usize) {
        self.selected_row = Some(idx);
        self.selected_rows.clear();
        self.selected_rows.insert(idx);
    }

    fn handle_row_click(&mut self, idx: usize, ctrl: bool, shift: bool) {
        if shift {
            let anchor = self.selected_row.unwrap_or(idx);
            let lo = anchor.min(idx);
            let hi = anchor.max(idx);
            self.selected_rows.clear();
            for i in lo..=hi {
                self.selected_rows.insert(i);
            }
        } else if ctrl {
            if self.selected_rows.contains(&idx) && self.selected_rows.len() > 1 {
                self.selected_rows.remove(&idx);
            } else {
                self.selected_rows.insert(idx);
                self.selected_row = Some(idx);
            }
        } else {
            self.select_single(idx);
        }
    }

    fn target_indices_for_edit(&self) -> Vec<usize> {
        if !self.selected_rows.is_empty() {
            self.selected_rows.iter().copied().collect()
        } else if let Some(idx) = self.selected_row {
            vec![idx]
        } else {
            Vec::new()
        }
    }

    fn copy_selected_rows(&mut self, ctx: &egui::Context) {
        let mut indices = self.target_indices_for_edit();
        indices.retain(|&i| i < self.rows.len() && !self.rows[i].formula.trim().is_empty());
        if indices.is_empty() {
            return;
        }
        self.clipboard_rows = indices.iter().map(|&i| self.rows[i].clone()).collect();
        let text = self
            .clipboard_rows
            .iter()
            .map(|r| format!("{}\t{:.2}", r.formula, r.temp_c))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.copy_text(text);
        self.status_msg = format!("Copiadas {} filas al portapapeles", self.clipboard_rows.len());
    }

    fn cut_selected_rows(&mut self, ctx: &egui::Context) {
        self.copy_selected_rows(ctx);
        let mut indices = self.target_indices_for_edit();
        indices.sort_unstable();
        indices.reverse();
        for i in indices {
            if i < self.rows.len() && self.rows.len() > 1 {
                self.rows.remove(i);
            } else if let Some(r) = self.rows.get_mut(0) {
                *r = EstimateRow::new("");
            }
        }
        self.select_single(0);
    }

    fn parse_rows_from_text(&self, txt: &str) -> Vec<EstimateRow> {
        let mut out = Vec::new();
        for line in txt.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split(['\t', ',']);
            let formula = parts.next().unwrap_or("").trim();
            if formula.is_empty() {
                continue;
            }
            let mut r = EstimateRow::new(formula);
            if let Some(t_str) = parts.next() {
                if let Ok(v) = t_str.trim().parse::<f64>() {
                    r.temp_c = v;
                }
            }
            out.push(r);
        }
        out
    }

    fn insert_evaluated_rows(&mut self, at_idx: usize, mut to_paste: Vec<EstimateRow>, db: &ThermoDatabase) {
        if to_paste.is_empty() {
            return;
        }
        let pos = at_idx.min(self.rows.len());
        let count = to_paste.len();
        for (offset, r) in to_paste.iter_mut().enumerate() {
            evaluate_estimate_row(r, db);
            self.rows.insert(pos + offset, r.clone());
        }
        self.selected_rows.clear();
        for i in pos..(pos + count) {
            self.selected_rows.insert(i);
        }
        self.selected_row = Some(pos);
        self.status_msg = format!("Pegadas {count} filas");
    }

    fn paste_at_row(&mut self, at_idx: usize, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut to_paste = self.clipboard_rows.clone();
        if to_paste.is_empty() {
            let clip_text = ctx.input(|i| {
                i.events.iter().find_map(|ev| match ev {
                    egui::Event::Paste(s) => Some(s.clone()),
                    _ => None,
                })
            });
            if let Some(txt) = clip_text {
                to_paste = self.parse_rows_from_text(&txt);
            }
        }
        if to_paste.is_empty() {
            self.paste_target_row = at_idx;
            self.paste_text_buf.clear();
            self.show_paste_modal = true;
            return;
        }
        self.insert_evaluated_rows(at_idx, to_paste, db);
    }

    pub fn open_cp_data_for_row(&mut self, row_idx: usize, db: &ThermoDatabase) {
        let Some(row) = self.rows.get(row_idx).cloned() else {
            return;
        };
        if row.formula.trim().is_empty() {
            self.status_msg = "Selecciona una fila con fórmula química primero".to_string();
            return;
        }
        self.cp_target_row = row_idx;
        self.cp_species_type = row.species_type;
        self.cp_org_group = row.organic_group;
        self.regenerate_cp_points(db);
        self.show_cp_data_win = true;
    }

    fn regenerate_cp_points(&mut self, db: &ThermoDatabase) {
        let Some(mut row) = self.rows.get(self.cp_target_row).cloned() else {
            return;
        };
        row.species_type = self.cp_species_type;
        row.organic_group = self.cp_org_group;
        let t_lo = self.cp_tmin_k.max(1.0);
        let t_hi = self.cp_tmax_k.max(t_lo + 1.0);
        self.cp_points = generate_cp_curve(&row, db, t_lo, t_hi, self.cp_steps);

        let db_sp = db.get_species(row.formula.trim());
        self.cp_exp_points = self
            .cp_points
            .iter()
            .map(|&(t, _)| {
                db_sp
                    .as_ref()
                    .and_then(|sp| evaluate_species_si(sp, t, false, false).ok().map(|p| p.0.cp))
            })
            .collect();

        let fitted = fit_kelley_cp(&self.cp_points);
        self.cp_kelley = fitted;
        self.cp_ranges = vec![CpFittedRange {
            tmin_k: t_lo,
            tmax_k: t_hi,
            kelley: fitted,
        }];
        self.cp_active_range = 0;
    }

    fn fit_cp_range_at(&mut self, idx: usize) {
        if idx >= self.cp_ranges.len() {
            return;
        }
        let r = &self.cp_ranges[idx];
        let mut sub_pts: Vec<(f64, f64)> = self
            .cp_points
            .iter()
            .copied()
            .filter(|&(t, _)| t >= r.tmin_k - 1e-6 && t <= r.tmax_k + 1e-6)
            .collect();
        if sub_pts.len() < 4 {
            sub_pts = self.cp_points.clone();
        }
        let fitted = fit_kelley_cp(&sub_pts);
        self.cp_ranges[idx].kelley = fitted;
        if idx == self.cp_active_range {
            self.cp_kelley = fitted;
        }
    }

    fn fit_active_cp_range(&mut self) {
        if self.cp_ranges.is_empty() {
            return;
        }
        let idx = self.cp_active_range.min(self.cp_ranges.len() - 1);
        self.fit_cp_range_at(idx);
    }

    fn fitted_cp_at_t(&self, t: f64) -> f64 {
        for r in &self.cp_ranges {
            if t >= r.tmin_k - 1e-6 && t <= r.tmax_k + 1e-6 {
                return r.kelley.cp(t);
            }
        }
        self.cp_kelley.cp(t)
    }

    fn prepare_db_review(&mut self, row_idx: usize, ranges: Vec<(f64, f64, KelleyCoefficients)>, db: &ThermoDatabase) {
        let Some(row) = self.rows.get(row_idx).cloned() else {
            return;
        };
        if row.formula.trim().is_empty() {
            return;
        }
        let sp = build_estimated_species_with_intervals(&row, ranges, "HSC Estimate");
        let existing_meta = db.get_species_metadata(&sp.formula);
        let (_, charge, _) = parse_estimate_formula(&sp.formula).unwrap_or_default();
        let raw_ph = match sp.phase {
            Phase::Gas => "g",
            Phase::Liquid => "l",
            Phase::AqueousIon | Phase::AqueousNeutral => "a",
            Phase::Solid => "s",
        }
        .to_string();

        let mut ref_row = row.clone();
        ref_row.temp_c = 25.0;
        evaluate_estimate_row(&mut ref_row, db);

        let pt298 = evaluate_species_si(&sp, 298.15, false, false).ok().map(|x| x.0);
        self.db_review = DbReviewDraft {
            open: true,
            formula: sp.formula.clone(),
            chemical_name: sp.name.clone(),
            structural_formula: existing_meta
                .as_ref()
                .map(|m| m.structural_formula.clone())
                .unwrap_or_default(),
            common_name: existing_meta
                .as_ref()
                .map(|m| m.common_name.clone())
                .unwrap_or_default(),
            cas_number: existing_meta
                .as_ref()
                .map(|m| m.cas_number.clone())
                .unwrap_or_default(),
            phase: sp.phase,
            raw_phase_code: raw_ph,
            mw: sp.molecular_weight,
            density_g_cm3: sp.density_g_cm3,
            charge,
            h298_kj_mol: pt298
                .map(|p| p.h / 1000.0)
                .unwrap_or_else(|| ref_row.est_h_kj_mol.unwrap_or(0.0)),
            s298_j_molk: pt298
                .map(|p| p.s)
                .unwrap_or_else(|| ref_row.est_s_j_molk.unwrap_or(50.0)),
            cp298_j_molk: pt298
                .map(|p| p.cp)
                .unwrap_or_else(|| ref_row.est_cp_j_molk.unwrap_or(40.0)),
            dg298_kj_mol: ref_row.est_dg_kj_mol.unwrap_or(0.0),
            reference: "HSC Estimate".to_string(),
            reliability_class: 4,
            intervals: sp.intervals,
        };
    }

    pub fn save_db_review_to_owndb(&mut self, db: &mut ThermoDatabase) -> Result<(), String> {
        let formula = self.db_review.formula.trim().to_string();
        if formula.is_empty() {
            return Err("La fórmula no puede estar vacía".into());
        }
        if self.db_review.intervals.is_empty() {
            return Err("Debe existir al menos un rango de temperatura".into());
        }
        let mut intervals = self.db_review.intervals.clone();
        for (i, iv) in intervals.iter_mut().enumerate() {
            iv.phase = self.db_review.phase;
            iv.reference = self.db_review.reference.trim().to_string();
            if i == 0 {
                iv.h_or_delta_h_tr = self.db_review.h298_kj_mol * 1000.0;
                iv.s_or_delta_s_tr = self.db_review.s298_j_molk;
            }
        }
        let sp = ChemicalSpecies {
            formula: formula.clone(),
            name: self.db_review.chemical_name.trim().to_string(),
            phase: self.db_review.phase,
            molecular_weight: self.db_review.mw,
            density_g_cm3: self.db_review.density_g_cm3,
            charge: self.db_review.charge,
            intervals,
        };
        let meta = SpeciesMetadata {
            structural_formula: self.db_review.structural_formula.trim().to_string(),
            common_name: self.db_review.common_name.trim().to_string(),
            cas_number: self.db_review.cas_number.trim().to_string(),
            melting_point_k: 0.0,
            boiling_point_k: 0.0,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: self.db_review.reliability_class.clamp(1, 10),
        };
        let raw_ph = self.db_review.raw_phase_code.clone();
        db.upsert_own_dataset(sp, meta, raw_ph);
        let save_path = self.own_db_path.trim().to_string();
        if !save_path.is_empty() {
            if let Some(parent) = std::path::Path::new(&save_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            db.save_own_hsc(&save_path).map_err(|e| e.to_string())?;
        }
        self.cached_db_scope = None;
        self.recalc_all(db);
        self.status_msg = format!("Especie '{formula}' guardada en OwnDB ({save_path})");
        Ok(())
    }

    pub fn save_sheet_to_path(&self, path: &str) -> Result<usize, String> {
        let non_empty: Vec<EstimateRow> = self
            .rows
            .iter()
            .filter(|r| !r.formula.trim().is_empty())
            .cloned()
            .collect();
        if path.to_ascii_lowercase().ends_with(".tsv") || path.to_ascii_lowercase().ends_with(".csv") {
            let sep = if path.to_ascii_lowercase().ends_with(".csv") { ',' } else { '\t' };
            let mut out = format!("Formula{sep}TempC{sep}Type{sep}MW{sep}H_Est{sep}S_Est{sep}Cp_Est{sep}dG_Est\n");
            for r in &non_empty {
                out.push_str(&format!(
                    "{}{sep}{:.2}{sep}{}{sep}{:.4}{sep}{:.4}{sep}{:.4}{sep}{:.4}{sep}{:.4}\n",
                    r.formula,
                    r.temp_c,
                    r.type_display().replace(sep, " "),
                    r.mw,
                    r.est_h_kj_mol.unwrap_or(0.0),
                    r.est_s_j_molk.unwrap_or(0.0),
                    r.est_cp_j_molk.unwrap_or(0.0),
                    r.est_dg_kj_mol.unwrap_or(0.0),
                ));
            }
            crate::storage::write_atomic(path, out).map_err(|e| e.to_string())?;
        } else {
            let bundle = EstimateSheetFile {
                rows: non_empty.clone(),
            };
            let json = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
            crate::storage::write_atomic(path, json).map_err(|e| e.to_string())?;
        }
        Ok(non_empty.len())
    }

    pub fn load_sheet_from_path(&mut self, path: &str, db: &ThermoDatabase) -> Result<usize, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut loaded = if let Ok(bundle) = serde_json::from_str::<EstimateSheetFile>(&raw) {
            bundle.rows
        } else {
            let mut rows = Vec::new();
            for (idx, line) in raw.lines().enumerate() {
                let t = line.trim();
                if t.is_empty() || (idx == 0 && t.to_ascii_lowercase().starts_with("formula")) {
                    continue;
                }
                let mut parts = t.split(['\t', ',']);
                let formula = parts.next().unwrap_or("").trim();
                if formula.is_empty() {
                    continue;
                }
                let mut r = EstimateRow::new(formula);
                if let Some(temp_s) = parts.next() {
                    if let Ok(v) = temp_s.trim().parse::<f64>() {
                        r.temp_c = v;
                    }
                }
                rows.push(r);
            }
            rows
        };
        for r in &mut loaded {
            evaluate_estimate_row(r, db);
        }
        let n = loaded.len();
        if loaded.is_empty() {
            loaded.push(EstimateRow::new(""));
        }
        self.rows = loaded;
        self.select_single(0);
        Ok(n)
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        if !ctx.wants_keyboard_input() {
            let (ev_copy, ev_cut, ev_paste) = ctx.input(|i| {
                let mut cp = false;
                let mut ct = false;
                let mut ps: Option<String> = None;
                for ev in &i.events {
                    match ev {
                        egui::Event::Copy => cp = true,
                        egui::Event::Cut => ct = true,
                        egui::Event::Paste(s) => ps = Some(s.clone()),
                        _ => {}
                    }
                }
                (cp, ct, ps)
            });
            if ev_copy {
                self.copy_selected_rows(ctx);
            }
            if ev_cut {
                self.cut_selected_rows(ctx);
            }
            if let Some(txt) = ev_paste {
                let parsed = self.parse_rows_from_text(&txt);
                let anchor = self.selected_row.unwrap_or(0);
                self.insert_evaluated_rows(anchor, parsed, db);
            }
        }

        if self
            .rows
            .last()
            .map(|r| !r.formula.trim().is_empty())
            .unwrap_or(true)
        {
            self.rows.push(EstimateRow::new(""));
        }

        // Top Ribbon Toolbar (Main | Estimation Settings | Units | Tools | Help)
        egui::TopBottomPanel::top("estimate_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(236, 240, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Main: File
                    ui.vertical(|ui| {
                        ui.menu_button(
                            egui::RichText::new("📄\nFile ▾").size(11.5),
                            |ui| {
                                if ui.button("New (Blank Sheet)").clicked() {
                                    self.rows = vec![EstimateRow::new("")];
                                    self.select_single(0);
                                    self.status_msg = "Hoja nueva".to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Open Sheet (.est8 / .tsv)...").clicked() {
                                    self.file_io_save_mode = false;
                                    self.show_file_io_modal = true;
                                    ui.close_menu();
                                }
                                if ui.button("Save Sheet (.est8 / .tsv / .csv)...").clicked() {
                                    self.file_io_save_mode = true;
                                    self.show_file_io_modal = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Load Fig. 1 Example").clicked() {
                                    self.rows = default_fig1_rows();
                                    self.recalc_all(db);
                                    self.select_single(0);
                                    self.status_msg = "Ejemplo Fig. 1 cargado".to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 5 Oxides & Sulfates Example").clicked() {
                                    self.rows = fig5_oxides_rows();
                                    self.recalc_all(db);
                                    self.selected_rows.clear();
                                    for i in 0..8 {
                                        self.selected_rows.insert(i);
                                    }
                                    self.selected_row = Some(0);
                                    self.status_msg = "Ejemplo Fig. 5 cargado (Li2O..CaO seleccionados)".to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Classic Example (Na2SO4, MgO*Al2O3, POP)").clicked() {
                                    self.rows = classic_hsc_estimate_rows();
                                    self.recalc_all(db);
                                    self.select_single(0);
                                    self.status_msg =
                                        "Ejemplo clásico H, S, Cp Estimate cargado (11 especies)"
                                            .to_string();
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 2 Example (C2H4(g))").clicked() {
                                    let mut r = EstimateRow::new("C2H4(g)");
                                    r.species_type = EstimateSpeciesType::Organic;
                                    r.organic_group = OrganicGroup::Alkenes;
                                    r.functional_group = FunctionalGroup::None;
                                    evaluate_estimate_row(&mut r, db);
                                    self.rows = vec![r, EstimateRow::new("")];
                                    self.select_single(0);
                                    ui.close_menu();
                                }
                                if ui.button("Load Fig. 3 Example (FeVO4)").clicked() {
                                    let mut r = EstimateRow::new("FeVO4");
                                    evaluate_estimate_row(&mut r, db);
                                    self.rows = vec![r, EstimateRow::new("")];
                                    self.select_single(0);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button(crate::i18n::text("common.016")).clicked() {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            },
                        );
                        ui.label(egui::RichText::new("Main").small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Estimation Settings
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let targets = self.target_indices_for_edit();
                            let anchor = self.selected_row.unwrap_or(0);
                            if let Some(cur) = self.rows.get(anchor).cloned() {
                                let mut next_type = cur.species_type;
                                let mut type_changed = false;
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Species Type").small());
                                    egui::ComboBox::from_id_salt("est_species_type")
                                        .width(105.0)
                                        .selected_text(cur.species_type.label())
                                        .show_ui(ui, |ui| {
                                            for st in [
                                                EstimateSpeciesType::NotSpecified,
                                                EstimateSpeciesType::Inorganic,
                                                EstimateSpeciesType::Organic,
                                            ] {
                                                type_changed |= ui
                                                    .selectable_value(&mut next_type, st, st.label())
                                                    .changed();
                                            }
                                        });
                                });
                                if type_changed {
                                    for idx in &targets {
                                        if let Some(r) = self.rows.get_mut(*idx) {
                                            r.species_type = next_type;
                                            evaluate_estimate_row(r, db);
                                        }
                                    }
                                }

                                let is_org = next_type == EstimateSpeciesType::Organic;
                                let mut next_org = cur.organic_group;
                                let mut next_fg = cur.functional_group;
                                let mut org_changed = false;
                                let mut fg_changed = false;
                                ui.add_enabled_ui(is_org, |ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Organic Group").small());
                                            egui::ComboBox::from_id_salt("est_org_group")
                                                .width(96.0)
                                                .selected_text(cur.organic_group.label())
                                                .show_ui(ui, |ui| {
                                                    for g in [
                                                        OrganicGroup::Alkanes,
                                                        OrganicGroup::Alkenes,
                                                        OrganicGroup::Alkynes,
                                                        OrganicGroup::Aromatics,
                                                        OrganicGroup::Alcohols,
                                                        OrganicGroup::Acids,
                                                    ] {
                                                        org_changed |= ui
                                                            .selectable_value(
                                                                &mut next_org,
                                                                g,
                                                                g.label(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Functional Group").small(),
                                            );
                                            egui::ComboBox::from_id_salt("est_func_group")
                                                .width(96.0)
                                                .selected_text(cur.functional_group.label())
                                                .show_ui(ui, |ui| {
                                                    for fg in [
                                                        FunctionalGroup::None,
                                                        FunctionalGroup::Ene,
                                                        FunctionalGroup::Diene,
                                                        FunctionalGroup::Triene,
                                                        FunctionalGroup::Yne,
                                                        FunctionalGroup::Hydroxyl,
                                                        FunctionalGroup::Carboxyl,
                                                    ] {
                                                        fg_changed |= ui
                                                            .selectable_value(
                                                                &mut next_fg,
                                                                fg,
                                                                fg.label(),
                                                            )
                                                            .changed();
                                                    }
                                                });
                                        });
                                    });
                                });
                                if org_changed || fg_changed {
                                    for idx in &targets {
                                        if let Some(r) = self.rows.get_mut(*idx) {
                                            if org_changed {
                                                r.organic_group = next_org;
                                            }
                                            if fg_changed {
                                                r.functional_group = next_fg;
                                            }
                                            evaluate_estimate_row(r, db);
                                        }
                                    }
                                }
                            }

                            ui.toggle_value(&mut self.show_type_column, "Show Type\nColumn");
                            ui.toggle_value(&mut self.freeze_col_a, "Freeze\nColumn A");
                            ui.toggle_value(&mut self.freeze_title_rows, "Freeze\nTitle Rows");
                        });
                        ui.label(
                            egui::RichText::new("Estimation Settings")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    });

                    ui.separator();

                    // Units
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!self.use_kelvin, "°C\nCelsius").clicked() {
                                self.use_kelvin = false;
                            }
                            if ui.selectable_label(self.use_kelvin, "K\nKelvin").clicked() {
                                self.use_kelvin = true;
                            }
                            if ui.selectable_label(!self.per_kg, "n\nMoles").clicked() {
                                self.per_kg = false;
                            }
                            if ui.selectable_label(self.per_kg, "1kg\nKilograms").clicked() {
                                self.per_kg = true;
                            }
                        });
                        ui.label(egui::RichText::new(crate::i18n::text("common.027")).small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Tools
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.menu_button("📊\nGraphics ▾", |ui| {
                                let default_sheet_scope = if self.selected_rows.len() > 1 {
                                    EstimateScatterScope::SelectedRows
                                } else {
                                    EstimateScatterScope::SheetRows
                                };
                                if ui.button(crate::i18n::text("common.068")).clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = default_sheet_scope;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.069")).clicked() {
                                    self.plot_mode = EstimatePlotMode::ParityIndividual;
                                    self.scatter_scope = default_sheet_scope;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Database Scatter (Gas Species)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::GasDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Database Scatter (Aqueous Species)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::AqueousDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button("Database Scatter (All Maindb6)").clicked() {
                                    self.plot_mode = EstimatePlotMode::Parity;
                                    self.scatter_scope = EstimateScatterScope::AllDb;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button(crate::i18n::text("common.070")).clicked() {
                                    self.plot_mode = EstimatePlotMode::Comparison;
                                    self.scatter_scope = default_sheet_scope;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.071")).clicked() {
                                    self.plot_mode = EstimatePlotMode::Differences;
                                    self.scatter_scope = default_sheet_scope;
                                    self.show_graphics_win = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button(crate::i18n::text("common.072")).clicked() {
                                    let idx = self.selected_row.unwrap_or(0);
                                    self.open_cp_data_for_row(idx, db);
                                    ui.close_menu();
                                }
                            });
                            if ui.button("🗄\nBrowse HSC\nDatabase").clicked() {
                                self.show_db_picker = true;
                            }
                            ui.menu_button("💾\nSave to HSC\nDatabase ▾", |ui| {
                                if ui.button(crate::i18n::text("common.073")).clicked() {
                                    let idx = self.selected_row.unwrap_or(0);
                                    if let Some(row) = self.rows.get(idx).cloned() {
                                        if !row.formula.trim().is_empty() {
                                            let cp = row.est_cp_j_molk.unwrap_or(40.0);
                                            let kel = KelleyCoefficients {
                                                a: cp,
                                                b: 0.0,
                                                c: 0.0,
                                                d: 0.0,
                                            };
                                            self.prepare_db_review(idx, vec![(298.15, 300.0, kel)], db);
                                        }
                                    }
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.074")).clicked() {
                                    self.cp_target_row = self.selected_row.unwrap_or(0);
                                    self.show_cp_range_dialog = true;
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.label(egui::RichText::new(crate::i18n::text("common.018")).small().color(egui::Color32::GRAY));
                    });

                    ui.separator();

                    // Help & About
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("❓\nHelp").clicked() {
                                self.show_help_modal = true;
                            }
                            if ui.button("ℹ\nAbout").clicked() {
                                self.show_about_modal = true;
                            }
                        });
                        ui.label(egui::RichText::new(crate::i18n::text("common.003")).small().color(egui::Color32::GRAY));
                    });
                });
            });

        // Formula Bar + Quick Estimate strip
        egui::TopBottomPanel::top("estimate_formula_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let row_num = self.selected_row.unwrap_or(0) + 4;
                    ui.label(egui::RichText::new(format!("A{row_num}")).monospace().strong());
                    ui.separator();
                    if let Some(idx) = self.selected_row {
                        if let Some(row) = self.rows.get_mut(idx) {
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut row.formula)
                                        .desired_width(220.0),
                                )
                                .changed()
                            {
                                evaluate_estimate_row(row, db);
                            }
                        }
                    }
                    ui.separator();
                    ui.label("Species:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.quick_species).desired_width(120.0),
                    );
                    ui.label(crate::i18n::text("common.007"));
                    ui.add(egui::DragValue::new(&mut self.quick_temp).speed(1.0).fixed_decimals(2));
                    if ui.selectable_label(!self.quick_is_kelvin, "°C").clicked() && self.quick_is_kelvin {
                        self.quick_temp -= 273.15;
                        self.quick_is_kelvin = false;
                    }
                    if ui.selectable_label(self.quick_is_kelvin, "K").clicked() && !self.quick_is_kelvin {
                        self.quick_temp += 273.15;
                        self.quick_is_kelvin = true;
                    }
                    if ui.button("Estimate").clicked() && !self.quick_species.trim().is_empty() {
                        let mut r = EstimateRow::new(self.quick_species.trim());
                        r.temp_c = if self.quick_is_kelvin {
                            self.quick_temp - 273.15
                        } else {
                            self.quick_temp
                        };
                        evaluate_estimate_row(&mut r, db);
                        let insert_pos = self
                            .rows
                            .iter()
                            .position(|x| x.formula.trim().is_empty())
                            .unwrap_or(self.rows.len());
                        self.rows.insert(insert_pos, r);
                        self.select_single(insert_pos);
                        self.status_msg = format!("Estimado '{}'", self.quick_species.trim());
                    }
                });
            });

        // Bottom Sheet Tab Bar
        egui::TopBottomPanel::bottom("estimate_bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 237, 240))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("|◀  ◀  ▶  ▶|")
                            .size(10.5)
                            .color(egui::Color32::from_rgb(107, 114, 128)),
                    );
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                        .inner_margin(egui::Margin::symmetric(10.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Sheet1").strong().size(11.5));
                        });
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .small()
                            .color(egui::Color32::from_rgb(75, 85, 99)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Soryntel Estimate Engine")
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(185, 28, 28)),
                        );
                    });
                });
            });

        // Main Spreadsheet Area
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                self.render_spreadsheet(ctx, ui, db);
            });

        // Subwindows
        if self.show_db_picker {
            self.render_db_picker(ctx, db);
        }
        if self.show_graphics_win {
            self.render_graphics_window(ctx, db);
        }
        if self.show_cp_range_dialog {
            self.render_cp_range_dialog(ctx, db);
        }
        if self.show_cp_data_win {
            self.render_cp_data_window(ctx, db);
        }
        if self.db_review.open {
            self.render_db_review_window(ctx, db);
        }
        if self.show_file_io_modal {
            self.render_file_io_modal(ctx, db);
        }
        if self.show_paste_modal {
            let mut open = true;
            let mut close_now = false;
            egui::Window::new("Paste Formulas / TSV")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("Pega fórmulas (una por línea o separadas por tab/coma con Temp °C):");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.paste_text_buf)
                            .desired_rows(6)
                            .desired_width(340.0),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Paste Rows").clicked() {
                            let parsed = self.parse_rows_from_text(&self.paste_text_buf);
                            let at = self.paste_target_row;
                            self.insert_evaluated_rows(at, parsed, db);
                            close_now = true;
                        }
                        if ui.button(crate::i18n::text("common.000")).clicked() {
                            close_now = true;
                        }
                    });
                });
            self.show_paste_modal = open && !close_now;
        }
        if self.show_help_modal {
            egui::Window::new("Help — HSC Estimate")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("• Escribe una fórmula en la columna A (ej. FeVO4, MgSO4*2H2O, NaBO3*4H2O, H2Sn(OH)6, (C2H5)2O, Fe0.998O, FeS2(M), C(D), OH(-a)).");
                    ui.label("• Limitaciones (Sec. 16.3): sin superíndices/subíndices; el último paréntesis se reserva para fase/polimorfo/carga (usa AlO2H / AlOOH / AlO*OH, no AlO*(OH)).");
                    ui.label("• Selecciona varias filas (Ctrl+Click / Shift+Click) y pulsa Graphics para Parity / Individual Series / Comparison / Differences.");
                    ui.label("• Save to HSC Database > Cp at 25°C o Cp Range -> Fit Cp -> Update Species -> Save guarda realmente en OwnDB.");
                    ui.add_space(8.0);
                    if ui.button("Cerrar").clicked() {
                        self.show_help_modal = false;
                    }
                });
        }
        if self.show_about_modal {
            egui::Window::new("About — HSC Estimate")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.strong("HSC H, S, C Estimates Module (Soryntel Chemistry)");
                    ui.label("Neumann-Kopp Cp + Einstein-Kelley Curvature, Latimer Entropy, Oxo/Halide/Hydrate Enthalpy, Benson Organics & 4-Param Kelley Fit.");
                    if ui.button(crate::i18n::text("common.002")).clicked() {
                        self.show_about_modal = false;
                    }
                });
        }
    }

    fn render_spreadsheet(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        let max_elems = self
            .rows
            .iter()
            .map(|r| r.elements.len())
            .max()
            .unwrap_or(3)
            .max(3);

        let mut col_widths = vec![
            142.0, // A: Chemical Formula
            72.0,  // B: Temp
            if self.show_type_column { 148.0 } else { 0.0 }, // C: Species Type
            102.0, // D: Selected set of oxidation numbers
            68.0,  // E: MW
            78.0,  // F: H Est
            78.0,  // G: H DB
            76.0,  // H: S Est
            76.0,  // I: S DB
            74.0,  // J: Cp Est
            74.0,  // K: Cp DB
            78.0,  // L: dG Est
            78.0,  // M: dG DB
        ];
        for _ in 0..max_elems {
            col_widths.extend_from_slice(&[54.0, 56.0, 54.0, 64.0, 62.0]);
        }

        let row_num_w = 30.0;
        let header_bg = egui::Color32::from_rgb(156, 160, 166);
        let sel_tint = egui::Color32::from_rgb(219, 234, 254);
        let temp_lbl = if self.use_kelvin { "Temp K" } else { "Temp °C" };
        let t_ref_hdr = if self.use_kelvin { "298.15 K" } else { "25 °C" };
        let mass_unit = if self.per_kg { "kg" } else { "mol" };

        let mut header_rows: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let fixed_r1 = [
            "", "", "", "Selected set", "MW",
            &format!("H ({t_ref_hdr})"), "",
            &format!("S ({t_ref_hdr})"), "",
            "Cp", "",
            &format!("ΔG ({t_ref_hdr})"), "",
        ];
        let fixed_r2 = [
            "Chemical Formula", temp_lbl, "Species Type", "of oxidation", "",
            &format!("kJ/{mass_unit}"), "",
            &format!("J/{mass_unit}*K"), "",
            &format!("J/{mass_unit}*K"), "",
            &format!("kJ/{mass_unit}"), "",
        ];
        let fixed_r3 = [
            "", "", "", "numbers", "g/mol",
            "Estimate", "Database", "Estimate", "Database",
            "Estimate", "Database", "Estimate", "Database",
        ];
        header_rows[0].extend(fixed_r1.iter().map(|s| (*s).to_string()));
        header_rows[1].extend(fixed_r2.iter().map(|s| (*s).to_string()));
        header_rows[2].extend(fixed_r3.iter().map(|s| (*s).to_string()));
        for e in 1..=max_elems {
            header_rows[0].extend(["Element Information", "", "", "", ""].iter().map(|s| (*s).to_string()));
            header_rows[1].extend(["", "", "", "", ""].iter().map(|s| (*s).to_string()));
            header_rows[2].extend([
                format!("Elem{e}"),
                "Amount".into(),
                "Charge".into(),
                "Weight-%".into(),
                "Atom-%".into(),
            ]);
        }

        let mut action = SheetAction::default();
        let display_rows = self.rows.len().max(21);

        let draw_header_slice = |ui: &mut egui::Ui, start_col: usize, include_row_num: bool| {
            ui.horizontal(|ui| {
                if include_row_num {
                    header_cell(ui, "", row_num_w);
                }
                for (idx, w) in col_widths.iter().enumerate().skip(start_col) {
                    if *w > 0.0 {
                        header_cell(ui, &col_letter(idx), *w);
                    }
                }
            });
            for (h_idx, row_labels) in header_rows.iter().enumerate() {
                ui.horizontal(|ui| {
                    if include_row_num {
                        row_num_cell(ui, h_idx + 1, row_num_w);
                    }
                    for (idx, txt) in row_labels.iter().enumerate().skip(start_col) {
                        if col_widths[idx] > 0.0 {
                            text_cell(
                                ui,
                                txt,
                                col_widths[idx],
                                header_bg,
                                egui::Color32::from_rgb(20, 20, 22),
                                false,
                                true,
                                false,
                            );
                        }
                    }
                });
            }
        };

        let draw_col_a_row = |ui: &mut egui::Ui,
                              state: &mut EstimateModuleState,
                              r_idx: usize,
                              act: &mut SheetAction| {
            let excel_r = r_idx + 4;
            let in_multi = state.selected_rows.contains(&r_idx);
            let is_anchor = state.selected_row == Some(r_idx);
            let num_resp = ui.horizontal(|ui| {
                let nr = text_cell(
                    ui,
                    &excel_r.to_string(),
                    row_num_w,
                    if in_multi { sel_tint } else { crate::excel_grid::EXCEL_HEADER_BG },
                    egui::Color32::from_rgb(40, 40, 40),
                    false,
                    false,
                    false,
                );
                if nr.clicked() {
                    let (ctrl, shift) = ui.input(|i| (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift));
                    state.handle_row_click(r_idx, ctrl, shift);
                }
                if r_idx < state.rows.len() {
                    let bg = if in_multi && state.selected_rows.len() > 1 {
                        sel_tint
                    } else {
                        egui::Color32::WHITE
                    };
                    widget_cell(ui, col_widths[0], bg, is_anchor, |ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut state.rows[r_idx].formula)
                                .desired_width(col_widths[0] - 8.0),
                        );
                        if resp.gained_focus() {
                            let (ctrl, shift) = ui.input(|i| (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift));
                            if !ctrl && !shift && !state.selected_rows.contains(&r_idx) {
                                state.select_single(r_idx);
                            } else {
                                state.selected_row = Some(r_idx);
                            }
                        }
                        if resp.changed() {
                            act.recalc_idx = Some(r_idx);
                        }
                        resp.context_menu(|ui| {
                            if ui.button(crate::i18n::text("common.075")).clicked() {
                                act.cut = true;
                                ui.close_menu();
                            }
                            if ui.button(crate::i18n::text("common.029")).clicked() {
                                act.copy = true;
                                ui.close_menu();
                            }
                            if ui.button(crate::i18n::text("common.076")).clicked() {
                                act.paste_at = Some(r_idx);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(crate::i18n::text("common.077")).clicked() {
                                act.insert_at = Some(r_idx);
                                ui.close_menu();
                            }
                            if ui.button(crate::i18n::text("common.078")).clicked() {
                                act.delete_at = Some(r_idx);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Browse HSC Database...").clicked() {
                                act.open_db_picker = true;
                                ui.close_menu();
                            }
                            ui.menu_button("Graphics...", |ui| {
                                if ui.button(crate::i18n::text("common.068")).clicked() {
                                    act.open_graph = Some(EstimatePlotMode::Parity);
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.069")).clicked() {
                                    act.open_graph = Some(EstimatePlotMode::ParityIndividual);
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.070")).clicked() {
                                    act.open_graph = Some(EstimatePlotMode::Comparison);
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.071")).clicked() {
                                    act.open_graph = Some(EstimatePlotMode::Differences);
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.072")).clicked() {
                                    act.open_cp_est_at = Some(r_idx);
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Save to HSC Database", |ui| {
                                if ui.button(crate::i18n::text("common.073")).clicked() {
                                    act.save_cp25_at = Some(r_idx);
                                    ui.close_menu();
                                }
                                if ui.button(crate::i18n::text("common.074")).clicked() {
                                    act.open_cp_range_at = Some(r_idx);
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                } else {
                    text_cell(
                        ui,
                        "",
                        col_widths[0],
                        egui::Color32::WHITE,
                        egui::Color32::BLACK,
                        false,
                        false,
                        false,
                    );
                }
            });
            let _ = num_resp;
        };

        let draw_cols_b_plus_row = |ui: &mut egui::Ui,
                                    state: &mut EstimateModuleState,
                                    r_idx: usize,
                                    act: &mut SheetAction| {
            ui.horizontal(|ui| {
                if r_idx >= state.rows.len() {
                    for (idx, w) in col_widths.iter().enumerate().skip(1) {
                        if *w > 0.0 {
                            let bg = if idx == 1 { egui::Color32::WHITE } else { EXCEL_READONLY_BG };
                            text_cell(ui, "", *w, bg, egui::Color32::BLACK, false, false, false);
                        }
                    }
                    return;
                }

                // B: Temp (editable)
                let mut t_disp = if state.use_kelvin {
                    state.rows[r_idx].temp_c + 273.15
                } else {
                    state.rows[r_idx].temp_c
                };
                widget_cell(ui, col_widths[1], egui::Color32::WHITE, false, |ui| {
                    if !state.rows[r_idx].formula.trim().is_empty()
                        && ui
                            .add(
                                egui::DragValue::new(&mut t_disp)
                                    .speed(1.0)
                                    .fixed_decimals(2),
                            )
                            .changed()
                    {
                        state.rows[r_idx].temp_c = if state.use_kelvin {
                            t_disp - 273.15
                        } else {
                            t_disp
                        };
                        act.recalc_idx = Some(r_idx);
                    }
                });

                // C: Species Type (toggleable)
                if col_widths[2] > 0.0 {
                    let type_txt = if state.rows[r_idx].formula.trim().is_empty() {
                        String::new()
                    } else {
                        state.rows[r_idx].type_display()
                    };
                    if text_cell(
                        ui,
                        &type_txt,
                        col_widths[2],
                        EXCEL_READONLY_BG,
                        egui::Color32::from_rgb(24, 24, 27),
                        false,
                        false,
                        false,
                    )
                    .clicked()
                    {
                        state.select_single(r_idx);
                    }
                }

                // D: Selected set of oxidation numbers
                let sets = state.rows[r_idx].available_sets;
                if !state.rows[r_idx].formula.trim().is_empty() && sets > 1 {
                    widget_cell(ui, col_widths[3], egui::Color32::WHITE, false, |ui| {
                        let cur_set = state.rows[r_idx].oxidation_set_idx + 1;
                        let mut chosen = state.rows[r_idx].oxidation_set_idx;
                        egui::ComboBox::from_id_salt(format!("ox_set_{r_idx}"))
                            .width(col_widths[3] - 10.0)
                            .selected_text(format!("Set {cur_set}"))
                            .show_ui(ui, |ui| {
                                for s in 0..sets {
                                    ui.selectable_value(&mut chosen, s, format!("Set {}", s + 1));
                                }
                            });
                        if chosen != state.rows[r_idx].oxidation_set_idx {
                            state.rows[r_idx].oxidation_set_idx = chosen;
                            act.recalc_idx = Some(r_idx);
                        }
                    });
                } else {
                    text_cell(
                        ui,
                        "",
                        col_widths[3],
                        EXCEL_READONLY_BG,
                        egui::Color32::BLACK,
                        false,
                        false,
                        false,
                    );
                }

                // E..M: MW, H(Est/DB), S(Est/DB), Cp(Est/DB), dG(Est/DB)
                let row = &state.rows[r_idx];
                let has_f = !row.formula.trim().is_empty();
                let scale = if state.per_kg && row.mw > 0.0 {
                    1000.0 / row.mw
                } else {
                    1.0
                };
                let fmt_opt = |v: Option<f64>| -> (String, egui::Color32) {
                    match v {
                        Some(val) if has_f => {
                            (format!("{:.2}", val * scale), egui::Color32::from_rgb(17, 24, 39))
                        }
                        _ if has_f => ("N/A".to_string(), egui::Color32::from_rgb(37, 99, 235)),
                        _ => (String::new(), egui::Color32::BLACK),
                    }
                };

                let mw_str = if has_f { format!("{:.2}", row.mw) } else { String::new() };
                text_cell(
                    ui,
                    &mw_str,
                    col_widths[4],
                    EXCEL_READONLY_BG,
                    egui::Color32::from_rgb(17, 24, 39),
                    true,
                    false,
                    false,
                );

                let pairs = [
                    fmt_opt(row.est_h_kj_mol),
                    fmt_opt(row.db_vals.as_ref().map(|d| d.h_kj_mol)),
                    fmt_opt(row.est_s_j_molk),
                    fmt_opt(row.db_vals.as_ref().map(|d| d.s_j_molk)),
                    fmt_opt(row.est_cp_j_molk),
                    fmt_opt(row.db_vals.as_ref().map(|d| d.cp_j_molk)),
                    fmt_opt(row.est_dg_kj_mol),
                    fmt_opt(row.db_vals.as_ref().map(|d| d.dg_kj_mol)),
                ];
                for (offset, (txt, col)) in pairs.iter().enumerate() {
                    text_cell(
                        ui,
                        txt,
                        col_widths[5 + offset],
                        EXCEL_READONLY_BG,
                        *col,
                        true,
                        false,
                        false,
                    );
                }

                // N..: Dynamic Element Information blocks
                for e_idx in 0..max_elems {
                    let base_col = 13 + e_idx * 5;
                    if has_f {
                        if let Some(el) = row.elements.get(e_idx) {
                            let dark = egui::Color32::from_rgb(17, 24, 39);
                            let amt_str = format_stoich_amount(el.amount);
                            text_cell(ui, &el.symbol, col_widths[base_col], EXCEL_READONLY_BG, dark, false, false, false);
                            text_cell(ui, &amt_str, col_widths[base_col + 1], EXCEL_READONLY_BG, dark, true, false, false);
                            text_cell(ui, &format!("{}", el.charge), col_widths[base_col + 2], EXCEL_READONLY_BG, dark, true, false, false);
                            text_cell(ui, &format!("{:.2}", el.weight_pct), col_widths[base_col + 3], EXCEL_READONLY_BG, dark, true, false, false);
                            text_cell(ui, &format!("{:.2}", el.atom_pct), col_widths[base_col + 4], EXCEL_READONLY_BG, dark, true, false, false);
                            continue;
                        }
                    }
                    for k in 0..5 {
                        text_cell(ui, "", col_widths[base_col + k], EXCEL_READONLY_BG, egui::Color32::BLACK, true, false, false);
                    }
                }
            });
        };

        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        if !self.freeze_col_a && !self.freeze_title_rows {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    draw_header_slice(ui, 0, true);
                    for r_idx in 0..display_rows {
                        ui.horizontal(|ui| {
                            draw_col_a_row(ui, self, r_idx, &mut action);
                            draw_cols_b_plus_row(ui, self, r_idx, &mut action);
                        });
                    }
                });
        } else if self.freeze_title_rows && !self.freeze_col_a {
            egui::ScrollArea::horizontal()
                .id_salt("est_hdr_only_hscroll")
                .horizontal_scroll_offset(self.h_scroll_offset)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    draw_header_slice(ui, 0, true);
                });
            let body_out = egui::ScrollArea::both()
                .id_salt("est_sheet_body_both")
                .horizontal_scroll_offset(self.h_scroll_offset)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for r_idx in 0..display_rows {
                        ui.horizontal(|ui| {
                            draw_col_a_row(ui, self, r_idx, &mut action);
                            draw_cols_b_plus_row(ui, self, r_idx, &mut action);
                        });
                    }
                });
            self.h_scroll_offset = body_out.state.offset.x;
        } else if self.freeze_col_a && self.freeze_title_rows {
            let left_w = row_num_w + col_widths[0];
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    ui.horizontal(|ui| {
                        header_cell(ui, "", row_num_w);
                        header_cell(ui, "A", col_widths[0]);
                    });
                    for (h_idx, row_labels) in header_rows.iter().enumerate() {
                        ui.horizontal(|ui| {
                            row_num_cell(ui, h_idx + 1, row_num_w);
                            text_cell(ui, &row_labels[0], col_widths[0], header_bg, egui::Color32::from_rgb(20, 20, 22), false, true, false);
                        });
                    }
                });
                egui::ScrollArea::horizontal()
                    .id_salt("est_hdr_hscroll")
                    .horizontal_scroll_offset(self.h_scroll_offset)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        draw_header_slice(ui, 1, false);
                    });
            });
            let avail_h = ui.available_height().max(120.0);
            let old_v = self.v_scroll_offset;
            let mut next_v = old_v;
            ui.horizontal_top(|ui| {
                ui.allocate_ui(egui::vec2(left_w, avail_h), |ui| {
                    let left_out = egui::ScrollArea::vertical()
                        .id_salt("est_frozen_cola_left_v")
                        .vertical_scroll_offset(self.v_scroll_offset)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for r_idx in 0..display_rows {
                                draw_col_a_row(ui, self, r_idx, &mut action);
                            }
                        });
                    if (left_out.state.offset.y - old_v).abs() > 0.1 {
                        next_v = left_out.state.offset.y;
                    }
                });
                let body_out = egui::ScrollArea::both()
                    .id_salt("est_frozen_cola_body")
                    .horizontal_scroll_offset(self.h_scroll_offset)
                    .vertical_scroll_offset(next_v)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for r_idx in 0..display_rows {
                            draw_cols_b_plus_row(ui, self, r_idx, &mut action);
                        }
                    });
                self.h_scroll_offset = body_out.state.offset.x;
                if (body_out.state.offset.y - old_v).abs() > 0.1 {
                    next_v = body_out.state.offset.y;
                }
            });
            self.v_scroll_offset = next_v.max(0.0);
        } else {
            // freeze_col_a && !freeze_title_rows
            let left_w = row_num_w + col_widths[0];
            let avail_h = ui.available_height().max(120.0);
            let old_v = self.v_scroll_offset;
            let mut next_v = old_v;
            ui.horizontal_top(|ui| {
                ui.allocate_ui(egui::vec2(left_w, avail_h), |ui| {
                    let left_out = egui::ScrollArea::vertical()
                        .id_salt("est_cola_nofreeze_v")
                        .vertical_scroll_offset(self.v_scroll_offset)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "", row_num_w);
                                header_cell(ui, "A", col_widths[0]);
                            });
                            for (h_idx, row_labels) in header_rows.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    row_num_cell(ui, h_idx + 1, row_num_w);
                                    text_cell(ui, &row_labels[0], col_widths[0], header_bg, egui::Color32::from_rgb(20, 20, 22), false, true, false);
                                });
                            }
                            for r_idx in 0..display_rows {
                                draw_col_a_row(ui, self, r_idx, &mut action);
                            }
                        });
                    if (left_out.state.offset.y - old_v).abs() > 0.1 {
                        next_v = left_out.state.offset.y;
                    }
                });
                let body_out = egui::ScrollArea::both()
                    .id_salt("est_cola_nofreeze_body")
                    .vertical_scroll_offset(next_v)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        draw_header_slice(ui, 1, false);
                        for r_idx in 0..display_rows {
                            draw_cols_b_plus_row(ui, self, r_idx, &mut action);
                        }
                    });
                if (body_out.state.offset.y - old_v).abs() > 0.1 {
                    next_v = body_out.state.offset.y;
                }
            });
            self.v_scroll_offset = next_v.max(0.0);
        }

        if let Some(idx) = action.recalc_idx {
            if let Some(row) = self.rows.get_mut(idx) {
                evaluate_estimate_row(row, db);
            }
        }
        if action.cut {
            self.cut_selected_rows(ctx);
        }
        if action.copy {
            self.copy_selected_rows(ctx);
        }
        if let Some(pos) = action.paste_at {
            self.paste_at_row(pos, ctx, db);
        }
        if let Some(idx) = action.insert_at {
            self.rows.insert(idx, EstimateRow::new(""));
        }
        if let Some(idx) = action.delete_at {
            if self.rows.len() > 1 {
                self.rows.remove(idx);
            }
        }
        if action.open_db_picker {
            self.show_db_picker = true;
        }
        if let Some(mode) = action.open_graph {
            self.plot_mode = mode;
            self.scatter_scope = if self.selected_rows.len() > 1 {
                EstimateScatterScope::SelectedRows
            } else {
                EstimateScatterScope::SheetRows
            };
            self.show_graphics_win = true;
        }
        if let Some(idx) = action.open_cp_est_at {
            self.open_cp_data_for_row(idx, db);
        }
        if let Some(idx) = action.save_cp25_at {
            if let Some(row) = self.rows.get(idx).cloned() {
                if !row.formula.trim().is_empty() {
                    let cp = row.est_cp_j_molk.unwrap_or(40.0);
                    let kel = KelleyCoefficients { a: cp, b: 0.0, c: 0.0, d: 0.0 };
                    self.prepare_db_review(idx, vec![(298.15, 300.0, kel)], db);
                }
            }
        }
        if let Some(idx) = action.open_cp_range_at {
            self.cp_target_row = idx;
            self.show_cp_range_dialog = true;
        }
    }

    fn render_db_picker(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_db_picker;
        let mut insert_formula: Option<String> = None;

        egui::Window::new("Browse HSC Database — Insert Species")
            .open(&mut open)
            .default_size([480.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Buscar fórmula:");
                    ui.text_edit_singleline(&mut self.db_search);
                });
                ui.separator();

                let matches = db.search(self.db_search.trim(), 150);
                egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
                    for sp in matches {
                        ui.horizontal(|ui| {
                            if ui.button(crate::i18n::text("common.010")).clicked() {
                                insert_formula = Some(sp.formula.clone());
                            }
                            ui.label(egui::RichText::new(&sp.formula).strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:?} · {:.2} g/mol",
                                    sp.phase, sp.molecular_weight
                                ))
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                        });
                    }
                });
            });

        self.show_db_picker = open;
        if let Some(f) = insert_formula {
            let mut r = EstimateRow::new(f);
            evaluate_estimate_row(&mut r, db);
            let idx = self.selected_row.unwrap_or(0).min(self.rows.len());
            self.rows.insert(idx, r);
            self.select_single(idx);
        }
    }

    fn ensure_db_scatter_cache(&mut self, db: &ThermoDatabase) {
        if matches!(
            self.scatter_scope,
            EstimateScatterScope::SheetRows | EstimateScatterScope::SelectedRows
        ) {
            return;
        }
        if self.cached_db_scope == Some(self.scatter_scope) {
            return;
        }
        let species_list = db.search("", 20_000);
        let mut evaluated = Vec::new();
        for sp in species_list {
            if sp.formula == "e-" || sp.formula == "e(-)" {
                continue;
            }
            let keep = match self.scatter_scope {
                EstimateScatterScope::SheetRows | EstimateScatterScope::SelectedRows => false,
                EstimateScatterScope::GasDb => sp.phase == Phase::Gas,
                EstimateScatterScope::AqueousDb => {
                    sp.phase == Phase::AqueousIon || sp.phase == Phase::AqueousNeutral
                }
                EstimateScatterScope::CondensedDb => {
                    sp.phase == Phase::Solid || sp.phase == Phase::Liquid
                }
                EstimateScatterScope::AllDb => true,
            };
            if !keep {
                continue;
            }
            let mut row = EstimateRow::new(sp.formula);
            evaluate_estimate_row(&mut row, db);
            if row.mw > 0.0 && row.db_vals.is_some() {
                evaluated.push(row);
            }
        }
        self.cached_db_rows = evaluated;
        self.cached_db_scope = Some(self.scatter_scope);
    }

    fn render_graphics_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        self.ensure_db_scatter_cache(db);
        let mut open = self.show_graphics_win;
        let mut close_clicked = false;
        let sel_rows_buf: Vec<EstimateRow> = if self.scatter_scope == EstimateScatterScope::SelectedRows {
            self.selected_rows
                .iter()
                .filter_map(|&i| self.rows.get(i).cloned())
                .collect()
        } else {
            Vec::new()
        };

        egui::Window::new("HSC Estimate — Graphics")
            .open(&mut open)
            .default_size([920.0, 590.0])
            .show(ctx, |ui| {
                // Top Fig. 5 toolbar
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.plot_mode, EstimatePlotMode::Parity, "📈 Parity");
                    ui.selectable_value(
                        &mut self.plot_mode,
                        EstimatePlotMode::ParityIndividual,
                        "✨ Parity with individual points",
                    );
                    ui.selectable_value(
                        &mut self.plot_mode,
                        EstimatePlotMode::Comparison,
                        "📊 Comparison",
                    );
                    ui.selectable_value(
                        &mut self.plot_mode,
                        EstimatePlotMode::Differences,
                        "📉 Differences",
                    );
                    ui.separator();
                    if ui.button("🔄 Reset Chart Positions").clicked() {
                        self.active_stat_prop = 1;
                        self.show_chart_properties = true;
                        self.reset_plot_counter = self.reset_plot_counter.wrapping_add(1);
                    }
                    ui.toggle_value(&mut self.show_chart_properties, "📋 Properties");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖ Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Scope:");
                    if self.selected_rows.len() > 1 {
                        ui.selectable_value(
                            &mut self.scatter_scope,
                            EstimateScatterScope::SelectedRows,
                            format!("Selected ({})", self.selected_rows.len()),
                        );
                    }
                    ui.selectable_value(&mut self.scatter_scope, EstimateScatterScope::SheetRows, "Sheet");
                    ui.selectable_value(&mut self.scatter_scope, EstimateScatterScope::GasDb, "Gas (DB)");
                    ui.selectable_value(&mut self.scatter_scope, EstimateScatterScope::AqueousDb, "Aqueous (DB)");
                    ui.selectable_value(&mut self.scatter_scope, EstimateScatterScope::CondensedDb, "Condensed (DB)");
                    ui.selectable_value(&mut self.scatter_scope, EstimateScatterScope::AllDb, "All DB");
                });
                ui.separator();

                let source_rows: &[EstimateRow] = match self.scatter_scope {
                    EstimateScatterScope::SelectedRows if !sel_rows_buf.is_empty() => &sel_rows_buf,
                    EstimateScatterScope::SheetRows | EstimateScatterScope::SelectedRows => &self.rows,
                    _ => &self.cached_db_rows,
                };

                let mut h_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut s_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut cp_pairs: Vec<(&str, f64, f64)> = Vec::new();
                let mut dg_pairs: Vec<(&str, f64, f64)> = Vec::new();

                for r in source_rows {
                    if r.formula.trim().is_empty() {
                        continue;
                    }
                    let scale = if self.per_kg && r.mw > 0.0 {
                        1000.0 / r.mw
                    } else {
                        1.0
                    };
                    if let Some(ref dbv) = r.db_vals {
                        if let Some(v) = r.est_h_kj_mol {
                            h_pairs.push((&r.formula, v * scale, dbv.h_kj_mol * scale));
                        }
                        if let Some(v) = r.est_s_j_molk {
                            s_pairs.push((&r.formula, v * scale, dbv.s_j_molk * scale));
                        }
                        if let Some(v) = r.est_cp_j_molk {
                            cp_pairs.push((&r.formula, v * scale, dbv.cp_j_molk * scale));
                        }
                        if let Some(v) = r.est_dg_kj_mol {
                            dg_pairs.push((&r.formula, v * scale, dbv.dg_kj_mol * scale));
                        }
                    }
                }

                let active_pairs = match self.active_stat_prop {
                    0 => &h_pairs,
                    1 => &s_pairs,
                    2 => &cp_pairs,
                    _ => &dg_pairs,
                };
                let active_name = match self.active_stat_prop {
                    0 => "H - Enthalpy",
                    1 => "S - Entropy",
                    2 => "Cp - Heat Capacity",
                    _ => "ΔG - Gibbs Energy",
                };
                let stats = compute_parity_stats(active_pairs);
                let is_large = !matches!(
                    self.scatter_scope,
                    EstimateScatterScope::SheetRows | EstimateScatterScope::SelectedRows
                );
                let pt_color = if is_large {
                    egui::Color32::from_rgb(220, 38, 38)
                } else {
                    egui::Color32::from_rgb(37, 99, 235)
                };
                let pt_radius = if is_large { 1.8 } else { 4.5 };

                let total_w = ui.available_width();
                let prop_w = if self.show_chart_properties { 232.0 } else { 0.0 };
                let charts_w = (total_w - prop_w - 14.0).max(460.0);
                let card_w = (charts_w - 10.0) * 0.5;
                let mut clicked_prop: Option<usize> = None;

                let mass_u = if self.per_kg { "kg" } else { "mol" };
                let t_u = if self.use_kelvin { "298.15 K" } else { "25°C" };

                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(charts_w);
                        let mut draw_card = |ui: &mut egui::Ui,
                                             prop_idx: usize,
                                             id: &str,
                                             header_title: &str,
                                             plot_title: &str,
                                             pairs: &[(&str, f64, f64)]| {
                            let is_active = self.active_stat_prop == prop_idx;
                            let border_col = if is_active {
                                egui::Color32::from_rgb(219, 39, 119)
                            } else {
                                EXCEL_HEADER_BORDER
                            };
                            let frame_resp = egui::Frame::none()
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(if is_active { 1.8 } else { 1.0 }, border_col))
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    ui.set_width(card_w - 14.0);
                                    if ui
                                        .add(
                                            egui::Label::new(
                                                egui::RichText::new(header_title)
                                                    .strong()
                                                    .size(11.5)
                                                    .color(if is_active {
                                                        egui::Color32::from_rgb(157, 23, 77)
                                                    } else {
                                                        egui::Color32::from_rgb(55, 65, 81)
                                                    }),
                                            )
                                            .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        clicked_prop = Some(prop_idx);
                                    }
                                    ui.label(
                                        egui::RichText::new(plot_title)
                                            .strong()
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(17, 24, 39)),
                                    );
                                    let plot_id = format!("{id}_{}", self.reset_plot_counter);
                                    let names_for_axis: Vec<String> =
                                        pairs.iter().map(|p| p.0.to_string()).collect();
                                    let is_indexed = matches!(
                                        self.plot_mode,
                                        EstimatePlotMode::Comparison | EstimatePlotMode::Differences
                                    );
                                    let mut plot = Plot::new(plot_id)
                                        .height(205.0)
                                        .x_axis_label(match self.plot_mode {
                                            EstimatePlotMode::Parity | EstimatePlotMode::ParityIndividual => {
                                                if is_large { "Experimental" } else { "Database" }
                                            }
                                            EstimatePlotMode::Comparison | EstimatePlotMode::Differences => "Species",
                                        })
                                        .y_axis_label(match self.plot_mode {
                                            EstimatePlotMode::Parity | EstimatePlotMode::ParityIndividual => {
                                                if is_large { "Estimated" } else { "Estimate" }
                                            }
                                            EstimatePlotMode::Comparison => "Value",
                                            EstimatePlotMode::Differences => "Estimate - DB",
                                        });
                                    if is_indexed && names_for_axis.len() <= 30 {
                                        plot = plot.x_axis_formatter(move |mark, _range| {
                                            let r = mark.value.round();
                                            if (mark.value - r).abs() < 0.05 && r >= 0.0 {
                                                names_for_axis
                                                    .get(r as usize)
                                                    .cloned()
                                                    .unwrap_or_default()
                                            } else {
                                                String::new()
                                            }
                                        });
                                    }
                                    if matches!(self.plot_mode, EstimatePlotMode::Parity | EstimatePlotMode::ParityIndividual) {
                                        plot = plot.data_aspect(1.0);
                                    }
                                    if self.plot_mode == EstimatePlotMode::ParityIndividual && pairs.len() <= 24 {
                                        plot = plot.legend(Legend::default());
                                    } else if self.plot_mode == EstimatePlotMode::Comparison {
                                        plot = plot.legend(Legend::default());
                                    }
                                    plot.show(ui, |plot_ui| {
                                        if pairs.is_empty() {
                                            return;
                                        }
                                        let min_raw = pairs
                                            .iter()
                                            .flat_map(|p| [p.1, p.2])
                                            .fold(f64::INFINITY, f64::min);
                                        let max_raw = pairs
                                            .iter()
                                            .flat_map(|p| [p.1, p.2])
                                            .fold(f64::NEG_INFINITY, f64::max);
                                        let pad = if (max_raw - min_raw).abs() < 1e-6 { 1.0 } else { 0.0 };
                                        let min_v = min_raw - pad;
                                        let max_v = max_raw + pad;
                                        match self.plot_mode {
                                            EstimatePlotMode::Parity => {
                                                plot_ui.line(
                                                    Line::new(PlotPoints::from(vec![
                                                        [min_v, min_v],
                                                        [max_v, max_v],
                                                    ]))
                                                    .color(egui::Color32::from_rgb(22, 163, 74)),
                                                );
                                                let pts: Vec<[f64; 2]> =
                                                    pairs.iter().map(|p| [p.2, p.1]).collect();
                                                plot_ui.points(
                                                    Points::new(PlotPoints::from(pts))
                                                        .radius(pt_radius)
                                                        .color(pt_color),
                                                );
                                            }
                                            EstimatePlotMode::ParityIndividual => {
                                                plot_ui.line(
                                                    Line::new(PlotPoints::from(vec![
                                                        [min_v, min_v],
                                                        [max_v, max_v],
                                                    ]))
                                                    .color(egui::Color32::from_rgb(22, 163, 74)),
                                                );
                                                for (i, &(name, est, dbv)) in pairs.iter().enumerate() {
                                                    let col = series_palette_color(i);
                                                    plot_ui.points(
                                                        Points::new(PlotPoints::from(vec![[dbv, est]]))
                                                            .radius(pt_radius + 1.0)
                                                            .color(col)
                                                            .name(name),
                                                    );
                                                }
                                            }
                                            EstimatePlotMode::Comparison => {
                                                if pairs.len() <= 30 {
                                                    let est_bars: Vec<Bar> = pairs
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, p)| Bar::new(i as f64 - 0.18, p.1).width(0.32))
                                                        .collect();
                                                    let db_bars: Vec<Bar> = pairs
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(i, p)| Bar::new(i as f64 + 0.18, p.2).width(0.32))
                                                        .collect();
                                                    plot_ui.bar_chart(
                                                        BarChart::new(est_bars)
                                                            .color(egui::Color32::from_rgb(37, 99, 235))
                                                            .name("Estimate"),
                                                    );
                                                    plot_ui.bar_chart(
                                                        BarChart::new(db_bars)
                                                            .color(egui::Color32::from_rgb(234, 88, 12))
                                                            .name("Database"),
                                                    );
                                                } else {
                                                    let est_pts: Vec<[f64; 2]> = pairs.iter().enumerate().map(|(i, p)| [i as f64, p.1]).collect();
                                                    let db_pts: Vec<[f64; 2]> = pairs.iter().enumerate().map(|(i, p)| [i as f64, p.2]).collect();
                                                    plot_ui.points(Points::new(PlotPoints::from(est_pts)).radius(pt_radius).name("Estimate"));
                                                    plot_ui.points(Points::new(PlotPoints::from(db_pts)).radius(pt_radius).name("Database"));
                                                }
                                            }
                                            EstimatePlotMode::Differences => {
                                                let diff_pts: Vec<[f64; 2]> = pairs
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, p)| [i as f64, p.1 - p.2])
                                                    .collect();
                                                plot_ui.line(
                                                    Line::new(PlotPoints::from(vec![
                                                        [0.0, 0.0],
                                                        [(pairs.len().saturating_sub(1)) as f64, 0.0],
                                                    ]))
                                                    .color(egui::Color32::from_rgb(156, 163, 175)),
                                                );
                                                plot_ui.points(
                                                    Points::new(PlotPoints::from(diff_pts))
                                                        .radius(pt_radius)
                                                        .color(pt_color),
                                                );
                                            }
                                        }
                                    });
                                });
                            if frame_resp.response.clicked() {
                                clicked_prop = Some(prop_idx);
                            }
                        };

                        ui.horizontal(|ui| {
                            draw_card(
                                ui,
                                0,
                                "est_plot_h",
                                "H - Enthalpy",
                                &format!("H ({t_u}) - kJ/{mass_u}"),
                                &h_pairs,
                            );
                            draw_card(
                                ui,
                                1,
                                "est_plot_s",
                                "S - Entropy",
                                &format!("S ({t_u}) - J/{mass_u}*K"),
                                &s_pairs,
                            );
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            draw_card(
                                ui,
                                2,
                                "est_plot_cp",
                                "Cp - Heat Capacity",
                                &format!("Cp - J/{mass_u}*K"),
                                &cp_pairs,
                            );
                            draw_card(
                                ui,
                                3,
                                "est_plot_dg",
                                "ΔG - Gibbs Energy",
                                &format!("ΔG ({t_u}) - kJ/{mass_u}"),
                                &dg_pairs,
                            );
                        });
                    });

                    if self.show_chart_properties {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(245, 246, 248))
                            .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.set_width(prop_w - 16.0);
                                ui.label(egui::RichText::new("Chart Properties").strong().size(12.5));
                                ui.separator();
                                ui.label(egui::RichText::new("Chart").strong().small());
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("common.030"));
                                    ui.label(egui::RichText::new(active_name).strong());
                                });
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("Statistics").strong().small());
                                egui::Grid::new("est_stats_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("Species Count");
                                        ui.label(egui::RichText::new(format!("{}", stats.species_count)).strong());
                                        ui.end_row();
                                        ui.label("Maximum Error");
                                        ui.label(egui::RichText::new(format!("{:.3}", stats.max_error)).strong());
                                        ui.end_row();
                                        ui.label("Max Error Species");
                                        ui.label(egui::RichText::new(&stats.max_error_species).strong());
                                        ui.end_row();
                                        ui.label("Mean Error");
                                        ui.label(egui::RichText::new(format!("{:.3}", stats.mean_error)).strong());
                                        ui.end_row();
                                        ui.label("Median Error");
                                        ui.label(egui::RichText::new(format!("{:.3}", stats.median_error)).strong());
                                        ui.end_row();
                                        ui.label("RMSE");
                                        ui.label(egui::RichText::new(format!("{:.3}", stats.rmse)).strong());
                                        ui.end_row();
                                    });
                            });
                    }
                });

                if let Some(p) = clicked_prop {
                    self.active_stat_prop = p;
                }
            });
        self.show_graphics_win = open && !close_clicked;
    }

    fn render_cp_range_dialog(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_cp_range_dialog;
        let mut run_fit = false;
        let mut close_now = false;

        let sp_name = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.formula.clone())
            .unwrap_or_default();

        egui::Window::new("Cp Range Data")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.label(egui::RichText::new(crate::i18n::text("common.066")).strong());
                    egui::Grid::new("cp_range_params").num_columns(2).show(ui, |ui| {
                        ui.label(crate::i18n::text("common.131"));
                        ui.label(egui::RichText::new(&sp_name).strong());
                        ui.end_row();
                        ui.label("Tmin (K)");
                        ui.add(egui::DragValue::new(&mut self.cp_tmin_k).speed(1.0).range(1.0..=10000.0));
                        ui.end_row();
                        ui.label("Tmax (K)");
                        ui.add(egui::DragValue::new(&mut self.cp_tmax_k).speed(1.0).range(2.0..=20000.0));
                        ui.end_row();
                        ui.label("Steps");
                        ui.add(egui::DragValue::new(&mut self.cp_steps).range(5..=500));
                        ui.end_row();
                    });
                });
                let valid = self.cp_tmax_k > self.cp_tmin_k && !sp_name.trim().is_empty();
                if !valid {
                    ui.colored_label(
                        egui::Color32::from_rgb(185, 28, 28),
                        "Tmax debe ser mayor que Tmin y la especie no puede estar vacía.",
                    );
                }
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(valid, |ui| {
                        if ui.button(crate::i18n::text("common.002")).clicked() {
                            run_fit = true;
                            close_now = true;
                        }
                    });
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        close_now = true;
                    }
                });
            });

        self.show_cp_range_dialog = open && !close_now;
        if run_fit {
            self.open_cp_data_for_row(self.cp_target_row, db);
        }
    }

    fn render_cp_data_window(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        let mut open = self.show_cp_data_win;
        let mut do_update_species = false;
        let mut do_fit = false;
        let mut do_estimate = false;
        let mut do_add_range = false;

        let sp_name = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.formula.clone())
            .unwrap_or_default();
        let sp_mw = self
            .rows
            .get(self.cp_target_row)
            .map(|r| r.mw)
            .unwrap_or(0.0);

        let avg_err = if self.cp_points.is_empty() {
            0.0
        } else {
            self.cp_points
                .iter()
                .enumerate()
                .map(|(i, &(t, cp_est))| {
                    let cp_ref = self.cp_exp_points.get(i).and_then(|x| *x).unwrap_or(cp_est);
                    (self.fitted_cp_at_t(t) - cp_ref).abs()
                })
                .sum::<f64>()
                / (self.cp_points.len() as f64)
        };

        egui::Window::new("HSC Cp Data")
            .open(&mut open)
            .default_size([920.0, 560.0])
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    // Left: 5-column table matching Fig. 7 (T, Cp Exp, Cp Fitting, Error, Cp Estimate)
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Species: {sp_name}   ·   MW: {sp_mw:.3} g/mol"
                            ))
                            .strong(),
                        );
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(470.0).show(ui, |ui| {
                            egui::Grid::new("hsc_cp_data_table")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("T (K)").strong());
                                    ui.label(egui::RichText::new("Cp Exp").strong());
                                    ui.label(egui::RichText::new("Cp Fitting").strong());
                                    ui.label(egui::RichText::new("Error").strong());
                                    ui.label(egui::RichText::new("Cp Estimate").strong());
                                    ui.end_row();

                                    for (i, &(t, cp_est)) in self.cp_points.iter().enumerate() {
                                        let cp_exp = self.cp_exp_points.get(i).and_then(|x| *x).unwrap_or(cp_est);
                                        let cp_fit = self.fitted_cp_at_t(t);
                                        ui.label(format!("{t:.3}"));
                                        ui.label(format!("{cp_exp:.3}"));
                                        ui.label(format!("{cp_fit:.3}"));
                                        ui.label(format!("{:.3}", cp_fit - cp_exp));
                                        ui.label(format!("{cp_est:.3}"));
                                        ui.end_row();
                                    }
                                });
                        });
                    });

                    // Right: Fitted Data (editable A..D, Range 1..N, Fit/Calculate/Estimate/Update) + Chart
                    cols[1].vertical(|ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "Cp(T) = A + B*T*10^-3 + C*T^-2*10^5 + D*T^2*10^-6",
                                    )
                                    .monospace()
                                    .small(),
                                );
                                if ui.button("⬆ Update Species").clicked() {
                                    do_update_species = true;
                                }
                            });
                            ui.separator();
                            let mut do_remove_range = false;
                            ui.horizontal(|ui| {
                                ui.label("Range:");
                                for i in 0..self.cp_ranges.len() {
                                    if ui.selectable_label(self.cp_active_range == i, format!("{}", i + 1)).clicked() {
                                        self.cp_active_range = i;
                                        self.cp_kelley = self.cp_ranges[i].kelley;
                                    }
                                }
                                if self.cp_ranges.len() < 5 && ui.button("+ Range").clicked() {
                                    do_add_range = true;
                                }
                                if self.cp_ranges.len() > 1 && ui.button("- Range").clicked() {
                                    do_remove_range = true;
                                }
                            });
                            if do_remove_range && self.cp_ranges.len() > 1 {
                                let rem = self.cp_active_range.min(self.cp_ranges.len() - 1);
                                self.cp_ranges.remove(rem);
                                self.cp_active_range = self.cp_active_range.min(self.cp_ranges.len() - 1);
                                self.cp_kelley = self.cp_ranges[self.cp_active_range].kelley;
                            }

                            let r_idx = self.cp_active_range.min(self.cp_ranges.len().saturating_sub(1));
                            if let Some(rng) = self.cp_ranges.get_mut(r_idx) {
                                egui::Grid::new("hsc_cp_fitted_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("Tmin (K)");
                                        ui.add(egui::DragValue::new(&mut rng.tmin_k).speed(1.0).fixed_decimals(2));
                                        ui.end_row();
                                        ui.label("Tmax (K)");
                                        ui.add(egui::DragValue::new(&mut rng.tmax_k).speed(1.0).fixed_decimals(2));
                                        ui.end_row();
                                        ui.label("Cp coefficient A J/(mol*K)");
                                        ui.add(egui::DragValue::new(&mut rng.kelley.a).speed(0.1).fixed_decimals(3));
                                        ui.end_row();
                                        ui.label(crate::i18n::text("common.061"));
                                        ui.add(egui::DragValue::new(&mut rng.kelley.b).speed(0.1).fixed_decimals(3));
                                        ui.end_row();
                                        ui.label(crate::i18n::text("common.062"));
                                        ui.add(egui::DragValue::new(&mut rng.kelley.c).speed(0.1).fixed_decimals(3));
                                        ui.end_row();
                                        ui.label(crate::i18n::text("common.063"));
                                        ui.add(egui::DragValue::new(&mut rng.kelley.d).speed(0.1).fixed_decimals(3));
                                        ui.end_row();
                                        ui.label("Average Error");
                                        ui.label(egui::RichText::new(format!("{avg_err:.3}")).strong());
                                        ui.end_row();
                                    });
                                self.cp_kelley = rng.kelley;
                            }

                            ui.horizontal_wrapped(|ui| {
                                if ui.button("➕ Fit Cp").clicked() {
                                    do_fit = true;
                                }
                                if ui.button("🔄 Calculate Cp").clicked() {
                                    // Fitted table & chart evaluate directly from rng.kelley every frame
                                    self.status_msg = "Cp Fitting y Error recalculados desde A, B, C, D".to_string();
                                }
                                egui::ComboBox::from_id_salt("cp_win_type")
                                    .width(96.0)
                                    .selected_text(self.cp_species_type.label())
                                    .show_ui(ui, |ui| {
                                        for st in [
                                            EstimateSpeciesType::NotSpecified,
                                            EstimateSpeciesType::Inorganic,
                                            EstimateSpeciesType::Organic,
                                        ] {
                                            ui.selectable_value(&mut self.cp_species_type, st, st.label());
                                        }
                                    });
                                ui.add_enabled_ui(self.cp_species_type == EstimateSpeciesType::Organic, |ui| {
                                    egui::ComboBox::from_id_salt("cp_win_org")
                                        .width(88.0)
                                        .selected_text(self.cp_org_group.label())
                                        .show_ui(ui, |ui| {
                                            for g in [
                                                OrganicGroup::None,
                                                OrganicGroup::Alkanes,
                                                OrganicGroup::Alkenes,
                                                OrganicGroup::Alkynes,
                                                OrganicGroup::Aromatics,
                                                OrganicGroup::Alcohols,
                                                OrganicGroup::Acids,
                                            ] {
                                                ui.selectable_value(&mut self.cp_org_group, g, g.label());
                                            }
                                        });
                                });
                                if ui.button("▶ Estimate Cp").clicked() {
                                    do_estimate = true;
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("Cp Heat Capacity").strong());
                            Plot::new("hsc_cp_fit_plot")
                                .height(205.0)
                                .x_axis_label("Temperature K")
                                .y_axis_label("J/(mol*K)")
                                .legend(Legend::default())
                                .show(ui, |plot_ui| {
                                    let est_pts: Vec<[f64; 2]> =
                                        self.cp_points.iter().map(|&(t, c)| [t, c]).collect();
                                    let fit_pts: Vec<[f64; 2]> = self
                                        .cp_points
                                        .iter()
                                        .map(|&(t, _)| [t, self.fitted_cp_at_t(t)])
                                        .collect();
                                    plot_ui.line(
                                        Line::new(PlotPoints::from(est_pts))
                                            .name("Experimental/Est")
                                            .color(egui::Color32::from_rgb(22, 163, 74)),
                                    );
                                    plot_ui.line(
                                        Line::new(PlotPoints::from(fit_pts))
                                            .name("Fitted")
                                            .color(egui::Color32::from_rgb(37, 99, 235)),
                                    );
                                });
                        });
                    });
                });
            });

        self.show_cp_data_win = open;
        if do_add_range && !self.cp_ranges.is_empty() {
            let split_idx = self.cp_active_range.min(self.cp_ranges.len() - 1);
            let old = self.cp_ranges[split_idx].clone();
            let mid = 0.5 * (old.tmin_k + old.tmax_k);
            self.cp_ranges[split_idx].tmax_k = mid;
            self.cp_ranges.insert(
                split_idx + 1,
                CpFittedRange {
                    tmin_k: mid,
                    tmax_k: old.tmax_k,
                    kelley: old.kelley,
                },
            );
            self.fit_cp_range_at(split_idx);
            self.cp_active_range = split_idx + 1;
            self.fit_cp_range_at(split_idx + 1);
        }
        if do_estimate {
            self.regenerate_cp_points(db);
        }
        if do_fit {
            self.fit_active_cp_range();
        }
        if do_update_species {
            let ranges: Vec<(f64, f64, KelleyCoefficients)> = self
                .cp_ranges
                .iter()
                .map(|r| (r.tmin_k, r.tmax_k, r.kelley))
                .collect();
            self.show_cp_data_win = false;
            self.prepare_db_review(self.cp_target_row, ranges, db);
        }
    }

    fn render_db_review_window(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        let mut open = self.db_review.open;
        let mut do_save = false;
        let mut do_cancel = false;

        egui::Window::new("Database Editor — Save Estimated Species (OwnDB)")
            .open(&mut open)
            .default_size([720.0, 480.0])
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.strong("Basic Data");
                    ui.columns(2, |cols| {
                        egui::Grid::new("est_db_left").num_columns(2).show(&mut cols[0], |ui| {
                            ui.label(crate::i18n::text("common.079"));
                            ui.text_edit_singleline(&mut self.db_review.formula);
                            ui.end_row();
                            ui.label("Chemical Name:");
                            ui.text_edit_singleline(&mut self.db_review.chemical_name);
                            ui.end_row();
                            ui.label(crate::i18n::text("common.080"));
                            ui.text_edit_singleline(&mut self.db_review.structural_formula);
                            ui.end_row();
                            ui.label(crate::i18n::text("common.081"));
                            ui.text_edit_singleline(&mut self.db_review.common_name);
                            ui.end_row();
                            ui.label("CAS:");
                            ui.text_edit_singleline(&mut self.db_review.cas_number);
                            ui.end_row();
                            ui.label("Phase:");
                            egui::ComboBox::from_id_salt("est_db_review_phase")
                                .selected_text(format!("{:?} ({})", self.db_review.phase, self.db_review.raw_phase_code))
                                .show_ui(ui, |ui| {
                                    for (ph, code, lbl) in [
                                        (Phase::Solid, "s", "Solid (s)"),
                                        (Phase::Liquid, "l", "Liquid (l)"),
                                        (Phase::Gas, "g", "Gas (g)"),
                                        (Phase::AqueousIon, "a", "Aqueous Ion (a)"),
                                        (Phase::AqueousNeutral, "a", "Aqueous Neutral (ia/a)"),
                                    ] {
                                        if ui.selectable_label(self.db_review.phase == ph, lbl).clicked() {
                                            self.db_review.phase = ph;
                                            self.db_review.raw_phase_code = code.to_string();
                                        }
                                    }
                                });
                            ui.end_row();
                        });
                        egui::Grid::new("est_db_right").num_columns(2).show(&mut cols[1], |ui| {
                            ui.label(crate::i18n::text("common.082"));
                            ui.label(format!("{:.3}", self.db_review.mw));
                            ui.end_row();
                            ui.label("H° formation at 298.15 K (kJ/mol):");
                            ui.label(format!("{:.3}", self.db_review.h298_kj_mol));
                            ui.end_row();
                            ui.label("S° at 298.15 K (J/(mol*K)):");
                            ui.label(format!("{:.3}", self.db_review.s298_j_molk));
                            ui.end_row();
                            ui.label("Cp° at 298.15 K (J/(mol*K)):");
                            ui.label(format!("{:.3}", self.db_review.cp298_j_molk));
                            ui.end_row();
                            ui.label("ΔG° at 298.15 K (kJ/mol):");
                            ui.label(format!("{:.3}", self.db_review.dg298_kj_mol));
                            ui.end_row();
                            ui.label("Reference / Rel. Class:");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.db_review.reference).desired_width(110.0));
                                let mut rc = self.db_review.reliability_class as i32;
                                if ui.add(egui::DragValue::new(&mut rc).range(1..=10)).changed() {
                                    self.db_review.reliability_class = rc as i16;
                                }
                            });
                            ui.end_row();
                        });
                    });
                });

                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.strong("Temperature Ranges");
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        egui::Grid::new("est_db_ranges_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                for h in ["Range", "Tmin (K)", "Tmax (K)", "Phase", "H kJ/mol", "S J/(mol*K)", "Cp A", "Cp B", "Cp C", "Cp D"] {
                                    ui.strong(h);
                                }
                                ui.end_row();
                                for (i, iv) in self.db_review.intervals.iter().enumerate() {
                                    ui.label(format!("{}", i + 1));
                                    ui.label(format!("{:.2}", iv.t_min));
                                    ui.label(format!("{:.2}", iv.t_max));
                                    ui.label(&self.db_review.raw_phase_code);
                                    ui.label(format!("{:.3}", iv.h_or_delta_h_tr / 1000.0));
                                    ui.label(format!("{:.3}", iv.s_or_delta_s_tr));
                                    ui.label(format!("{:.3}", iv.kelley.a));
                                    ui.label(format!("{:.3}", iv.kelley.b));
                                    ui.label(format!("{:.3}", iv.kelley.c));
                                    ui.label(format!("{:.3}", iv.kelley.d));
                                    ui.end_row();
                                }
                            });
                    });
                });

                ui.horizontal(|ui| {
                    ui.label("OwnDB file:");
                    ui.add(egui::TextEdit::singleline(&mut self.own_db_path).desired_width(360.0));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.014")).clicked() {
                        do_save = true;
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        do_cancel = true;
                    }
                });
            });

        self.db_review.open = open && !do_cancel;
        if do_save {
            match self.save_db_review_to_owndb(db) {
                Ok(()) => self.db_review.open = false,
                Err(e) => self.status_msg = format!("Error guardando OwnDB: {e}"),
            }
        }
    }

    fn render_file_io_modal(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_file_io_modal;
        let mut close_now = false;
        let title = if self.file_io_save_mode {
            "Save Estimate Sheet"
        } else {
            "Open Estimate Sheet"
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add(egui::TextEdit::singleline(&mut self.sheet_file_path).desired_width(360.0));
                });
                ui.horizontal(|ui| {
                    let action_lbl = if self.file_io_save_mode { "Save" } else { "Open" };
                    if ui.button(action_lbl).clicked() {
                        let p = self.sheet_file_path.clone();
                        if self.file_io_save_mode {
                            match self.save_sheet_to_path(&p) {
                                Ok(n) => {
                                    self.status_msg = format!("Hoja guardada ({n} especies) -> {p}");
                                    close_now = true;
                                }
                                Err(e) => self.status_msg = format!("Error guardando hoja: {e}"),
                            }
                        } else {
                            match self.load_sheet_from_path(&p, db) {
                                Ok(n) => {
                                    self.status_msg = format!("Hoja abierta ({n} especies) <- {p}");
                                    close_now = true;
                                }
                                Err(e) => self.status_msg = format!("Error abriendo hoja: {e}"),
                            }
                        }
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        close_now = true;
                    }
                });
            });
        self.show_file_io_modal = open && !close_now;
    }
}

#[derive(Default)]
struct SheetAction {
    recalc_idx: Option<usize>,
    cut: bool,
    copy: bool,
    paste_at: Option<usize>,
    insert_at: Option<usize>,
    delete_at: Option<usize>,
    open_db_picker: bool,
    open_graph: Option<EstimatePlotMode>,
    open_cp_est_at: Option<usize>,
    save_cp25_at: Option<usize>,
    open_cp_range_at: Option<usize>,
}

fn format_stoich_amount(val: f64) -> String {
    if (val - val.round()).abs() < 1e-6 {
        format!("{:.0}", val.round())
    } else {
        let s = format!("{val:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn series_palette_color(idx: usize) -> egui::Color32 {
    const PALETTE: [egui::Color32; 10] = [
        egui::Color32::from_rgb(37, 99, 235),
        egui::Color32::from_rgb(220, 38, 38),
        egui::Color32::from_rgb(22, 163, 74),
        egui::Color32::from_rgb(217, 119, 6),
        egui::Color32::from_rgb(147, 51, 234),
        egui::Color32::from_rgb(13, 148, 136),
        egui::Color32::from_rgb(219, 39, 119),
        egui::Color32::from_rgb(79, 70, 229),
        egui::Color32::from_rgb(101, 163, 13),
        egui::Color32::from_rgb(107, 114, 128),
    ];
    PALETTE[idx % PALETTE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp_range_fit_update_and_persist_to_owndb_roundtrip() {
        let mut db = ThermoDatabase::with_open_defaults();
        let mut state = EstimateModuleState::new(&db);
        let test_owndb = temp_file_path("soryntel_test_est_owndb6.HSC");
        let _ = std::fs::remove_file(&test_owndb);
        state.own_db_path = test_owndb.clone();

        // Add Na2WO4(g), run Cp Range 298.15..600 K, fit Kelley, update & save to OwnDB (Fig. 6-9)
        let mut row = EstimateRow::new("Na2WO4(g)");
        evaluate_estimate_row(&mut row, &db);
        state.rows.insert(0, row);
        state.select_single(0);
        state.cp_tmin_k = 298.15;
        state.cp_tmax_k = 600.0;
        state.cp_steps = 50;
        state.open_cp_data_for_row(0, &db);
        assert_eq!(state.cp_points.len(), 51);
        assert!(state.cp_ranges[0].kelley.c.abs() > 0.1);

        let ranges: Vec<(f64, f64, KelleyCoefficients)> = state
            .cp_ranges
            .iter()
            .map(|r| (r.tmin_k, r.tmax_k, r.kelley))
            .collect();
        state.prepare_db_review(0, ranges, &db);
        assert!(state.db_review.open);
        assert_eq!(state.db_review.formula, "Na2WO4(g)");
        assert_eq!(state.db_review.reference, "HSC Estimate");

        state.save_db_review_to_owndb(&mut db).unwrap();
        assert!(std::path::Path::new(&test_owndb).exists());
        // Row in Estimate sheet now shows Database values populated from OwnDB!
        assert!(state.rows[0].db_vals.is_some());

        // Reload OwnDB from disk and verify Na2WO4(g) persists cleanly
        let mut db2 = ThermoDatabase::with_open_defaults();
        db2.load_own_hsc(&test_owndb).unwrap();
        let reloaded = db2.get_species("Na2WO4(g)").expect("Na2WO4(g) must persist in OwnDB");
        assert_eq!(reloaded.phase, Phase::Gas);
        assert_eq!(reloaded.intervals[0].reference, "HSC Estimate");
        let _ = std::fs::remove_file(&test_owndb);
    }

    #[test]
    fn sheet_save_load_and_stoich_formatting() {
        assert_eq!(format_stoich_amount(1.0), "1");
        assert_eq!(format_stoich_amount(0.998), "0.998");
        assert_eq!(format_stoich_amount(4.0), "4");

        let db = ThermoDatabase::with_open_defaults();
        let mut state = EstimateModuleState::new(&db);
        let sheet_path = temp_file_path("soryntel_test_est_sheet.est8");
        let saved = state.save_sheet_to_path(&sheet_path).unwrap();
        assert_eq!(saved, 12);
        state.rows = vec![EstimateRow::new("")];
        let loaded = state.load_sheet_from_path(&sheet_path, &db).unwrap();
        assert_eq!(loaded, 12);
        assert_eq!(state.rows[0].formula, "AgCl");
        let _ = std::fs::remove_file(&sheet_path);
    }

    #[test]
    fn paste_tsv_and_multi_range_split_fit() {
        let mut db = ThermoDatabase::with_open_defaults();
        let mut state = EstimateModuleState::new(&db);
        let parsed = state.parse_rows_from_text("Li2O\t25\nMgO,100\n");
        assert_eq!(parsed.len(), 2);
        assert!((parsed[1].temp_c - 100.0).abs() < 1e-6);

        let before = state.rows.len();
        state.insert_evaluated_rows(0, parsed, &db);
        assert_eq!(state.rows.len(), before + 2);
        assert_eq!(state.rows[0].formula, "Li2O");
        assert_eq!(state.selected_rows.len(), 2);

        // Multi-range split fits both intervals accurately and writes 2 intervals to OwnDB
        state.open_cp_data_for_row(0, &db);
        let old = state.cp_ranges[0].clone();
        let mid = 0.5 * (old.tmin_k + old.tmax_k);
        state.cp_ranges[0].tmax_k = mid;
        state.cp_ranges.push(CpFittedRange {
            tmin_k: mid,
            tmax_k: old.tmax_k,
            kelley: old.kelley,
        });
        state.fit_cp_range_at(0);
        state.fit_cp_range_at(1);
        let ranges: Vec<(f64, f64, KelleyCoefficients)> = state
            .cp_ranges
            .iter()
            .map(|r| (r.tmin_k, r.tmax_k, r.kelley))
            .collect();
        state.prepare_db_review(0, ranges, &db);
        assert_eq!(state.db_review.intervals.len(), 2);
        let test_owndb = temp_file_path("soryntel_test_multi_rng.HSC");
        state.own_db_path = test_owndb.clone();
        state.save_db_review_to_owndb(&mut db).unwrap();
        let _ = std::fs::remove_file(&test_owndb);
    }
}
