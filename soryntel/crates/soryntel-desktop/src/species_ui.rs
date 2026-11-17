//! Module 16 UI: Species & Thermochemical Databases (`HSC - Databases`).
//!
//! Clones HSC Chemistry 6 Ch. 28 workflow with 6 integrated views under a shared
//! `File | Edit | View | Tools | Databases | Help` menu bar:
//! 1. **Browser & Search**: Elements (`Some`/`All` + periodic grid + `Organic` / C-limit),
//!    Stoichiometry (`Fix Ratio`), Keyword (`Formula`/`Structural`/`Name`/`Common`/`CAS`),
//!    layered `OwnDB > MainDB` badges, alternative temperature datasets, `MainDB6.REF` citations.
//! 2. **Species Editor (`OwnDB`)**: Full 19-field editing, dynamic transition intervals table,
//!    54-color palette swatch selector, `Calc MW`, `X -> X(l)` 298.15 K liquid extrapolation,
//!    Criss–Cobble & aqueous entropy scale inspection, `Save OwnDB` / `Delete OwnDB`.
//! 3. **Fit Cp Data**: Kelley OLS/QR regression with `Y`/`N` coefficient locking & error plot.
//! 4. **Convert Cp Functions**: External $C_p(T)$ (`Maier-Kelley`, `Shomate`, `NASA-7`, `DIPPR-100`, `Custom`)
//!    converted over 10 subintervals (11 nodes) to Kelley `(A, B, C, D)` with live plot.
//! 5. **Convert G(T) -> H, S, Cp**: Exact analytical conversion + `Delta to Normal`.
//! 6. **DB Tools & Refs**: Active `.HSC`/`.REF` paths, `.CAT` refresh, `Sort`, `Merge HSC1 -> HSC2`
//!    with duplicate inspection, legacy `HSC 2.0-5.0 -> 6.0`, CSV/JSON import/export, and `MainDB6.REF` explorer.

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, Points};
use soryntel_application::SoryntelApplication;
use soryntel_core::{CAL_TO_JOULE, T_REF_K};
use soryntel_db::hsc_binary::ReferenceEntry;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::elements::PERIODIC_ELEMENTS;
use soryntel_thermo::species::{
    color_spec, parse_carbon_limits, CpDataPoint, CpFitMask, CpFitResult, DbMergeConflict,
    DbSource, ElementSearchMode, ExternalCpTemplate, GibbsCoeffsInput, GibbsConvertOutput,
    KeywordSearchField, MergeConflictDecision, PhaseFilterFlags, SpeciesCatalogEntry,
    HSC_COLOR_PALETTE,
};
use soryntel_thermo::{
    ChemicalSpecies, KelleyCoefficients, Phase, SpeciesCatalogPort, SpeciesMetadata,
    SpeciesRepository, SpeciesThermoInterval,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeciesViewTab {
    Browser,
    Editor,
    FitCp,
    ConvertCp,
    ConvertGibbs,
    DbTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserQueryMode {
    Keyword,
    ElementsSome,
    ElementsAll,
    Stoichiometry,
}

#[derive(Debug, Clone)]
pub struct EditableIntervalRow {
    pub t1: String,
    pub t2: String,
    pub phase: Phase,
    /// Enthalpy H_298 (row 0) or dH_tr (row >= 1) in display units (`kcal` or `kJ`).
    pub h_disp: String,
    /// Entropy S_298 (row 0) or dS_tr (row >= 1) in display units (`cal/K` or `J/K`).
    pub s_disp: String,
    /// Kelley A, B, C, D in display units (`cal/(mol*K)` or `J/(mol*K)`).
    pub a_disp: String,
    pub b_disp: String,
    pub c_disp: String,
    pub d_disp: String,
    pub reference: String,
}

impl Default for EditableIntervalRow {
    fn default() -> Self {
        Self {
            t1: "298.15".into(),
            t2: "1500.00".into(),
            phase: Phase::Solid,
            h_disp: "0.000".into(),
            s_disp: "10.000".into(),
            a_disp: "10.000".into(),
            b_disp: "0.000".into(),
            c_disp: "0.000".into(),
            d_disp: "0.000".into(),
            reference: "OwnDB".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditableSpeciesForm {
    pub formula: String,
    pub structural_formula: String,
    pub chemical_name: String,
    pub common_name: String,
    pub cas_number: String,
    pub mw_str: String,
    pub density_str: String,
    pub mp_k_str: String,
    pub bp_k_str: String,
    pub color_code: i16,
    pub solubility: String,
    pub reliability_class: i16,
    pub raw_phase_code: String,
    pub intervals: Vec<EditableIntervalRow>,
}

impl Default for EditableSpeciesForm {
    fn default() -> Self {
        Self {
            formula: "NewX".into(),
            structural_formula: "NewX".into(),
            chemical_name: "New Substance".into(),
            common_name: String::new(),
            cas_number: String::new(),
            mw_str: "0.000".into(),
            density_str: "0.000".into(),
            mp_k_str: "0.00".into(),
            bp_k_str: "0.00".into(),
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
            raw_phase_code: "s".into(),
            intervals: vec![EditableIntervalRow::default()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct FitCpPointInput {
    pub t_str: String,
    pub cp_str: String,
}

pub struct SpeciesModuleState {
    pub active_tab: SpeciesViewTab,
    pub use_calories: bool,
    pub use_kelvin: bool,

    // View 1: Browser & Search
    pub query_mode: BrowserQueryMode,
    pub source_filter: Option<DbSource>,
    pub keyword_text: String,
    pub keyword_field: KeywordSearchField,
    pub stoich_text: String,
    pub fix_ratio: bool,
    pub selected_elements: BTreeSet<String>,
    pub elements_text: String,
    pub show_periodic_picker: bool,
    pub phase_filters: PhaseFilterFlags,
    pub carbon_limit_text: String,
    pub results_limit: usize,
    pub cached_results: Vec<SpeciesCatalogEntry>,
    pub results_dirty: bool,
    pub selected_formula: Option<String>,
    pub selected_source: Option<DbSource>,
    pub show_help_dialog: bool,

    // View 2: OwnDB Species Editor
    pub editor: EditableSpeciesForm,
    pub show_color_palette: bool,

    // View 3: Fit Cp Data
    pub fit_points: Vec<FitCpPointInput>,
    pub fit_mask: CpFitMask,
    pub lock_a: String,
    pub lock_b: String,
    pub lock_c: String,
    pub lock_d: String,
    pub fit_result: Option<CpFitResult>,

    // View 4: Convert Cp Functions
    pub ext_template: ExternalCpTemplate,
    pub ext_expr: String,
    pub ext_unit: String,
    pub ext_t1: String,
    pub ext_t2: String,
    pub ext_params: [String; 6],
    pub ext_mask: CpFitMask,
    pub ext_samples: Vec<CpDataPoint>,
    pub ext_result: Option<CpFitResult>,

    // View 5: Convert G(T)
    pub gibbs_is_delta: bool,
    pub gibbs_formula: String,
    pub gibbs_h: String,
    pub gibbs_s: String,
    pub gibbs_a: String,
    pub gibbs_b: String,
    pub gibbs_c: String,
    pub gibbs_d: String,
    pub gibbs_delta_out: Option<GibbsConvertOutput>,
    pub gibbs_normal_out: Option<GibbsConvertOutput>,

    // View 6: DB Tools, Merge & Refs
    pub main_path_input: String,
    pub own_path_input: String,
    pub ref_path_input: String,
    pub merge_src_path: String,
    pub merge_dst_path: String,
    pub merge_show_duplicates: bool,
    pub merge_conflicts: Vec<DbMergeConflict>,
    pub merge_decisions: BTreeMap<String, MergeConflictDecision>,
    pub legacy_src_path: String,
    pub legacy_dst_path: String,
    pub csv_io_path: String,
    pub ref_search_query: String,

    // Status notification + optional reaction formula handoff
    pub status_msg: String,
    pub status_is_err: bool,
    pub import_to_reaction_formula: Option<String>,
}

impl SpeciesModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let summary = db.status_summary();
        let default_main = summary
            .main_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC".into());
        let default_own = summary
            .own_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| crate::ui_common::temp_file_path("soryntel_owndb6.HSC"));
        let default_ref = summary
            .ref_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "/home/nando/Descargas/HSC6/Databases/MainDB6.REF".into());

        let mut st = Self {
            active_tab: SpeciesViewTab::Browser,
            use_calories: true,
            use_kelvin: true,

            query_mode: BrowserQueryMode::Keyword,
            source_filter: None,
            keyword_text: String::new(),
            keyword_field: KeywordSearchField::Any,
            stoich_text: "SiO4FeFe".into(),
            fix_ratio: false,
            selected_elements: BTreeSet::new(),
            elements_text: "As, S, O".into(),
            show_periodic_picker: false,
            phase_filters: PhaseFilterFlags::default(),
            carbon_limit_text: String::new(),
            results_limit: 250,
            cached_results: Vec::new(),
            results_dirty: true,
            selected_formula: None,
            selected_source: None,
            show_help_dialog: false,

            editor: EditableSpeciesForm::default(),
            show_color_palette: false,

            fit_points: Self::default_naf_fit_points(),
            fit_mask: CpFitMask::default(),
            lock_a: "0.0".into(),
            lock_b: "0.0".into(),
            lock_c: "0.0".into(),
            lock_d: "0.0".into(),
            fit_result: None,

            ext_template: ExternalCpTemplate::ShomateNist,
            ext_expr: ExternalCpTemplate::ShomateNist.formula_string().into(),
            ext_unit: "J/(mol*K)".into(),
            ext_t1: "298.15".into(),
            ext_t2: "1500.0".into(),
            ext_params: [
                "25.0".into(),
                "10.0".into(),
                "0.0".into(),
                "0.0".into(),
                "0.0".into(),
                "0.0".into(),
            ],
            ext_mask: CpFitMask::default(),
            ext_samples: Vec::new(),
            ext_result: None,

            gibbs_is_delta: true,
            gibbs_formula: "Cu2S".into(),
            gibbs_h: "-14.394".into(),
            gibbs_s: "-86.625".into(),
            gibbs_a: "13.574".into(),
            gibbs_b: "-17.817".into(),
            gibbs_c: "-185.3".into(),
            gibbs_d: "0.0".into(),
            gibbs_delta_out: None,
            gibbs_normal_out: None,

            main_path_input: default_main,
            own_path_input: default_own,
            ref_path_input: default_ref,
            merge_src_path: crate::ui_common::temp_file_path("hsc_src.HSC"),
            merge_dst_path: crate::ui_common::temp_file_path("soryntel_owndb6.HSC"),
            merge_show_duplicates: true,
            merge_conflicts: Vec::new(),
            merge_decisions: BTreeMap::new(),
            legacy_src_path: "/home/nando/Descargas/HSC6/Databases/MAINDB3.HSC".into(),
            legacy_dst_path: crate::ui_common::temp_file_path("maindb3_upgraded.HSC"),
            csv_io_path: crate::ui_common::temp_file_path("soryntel_species_export.csv"),
            ref_search_query: String::new(),

            status_msg: format!(
                "Databases ready: {} species resolved ({} citations loaded).",
                summary.resolved_total_species, summary.references_count
            ),
            status_is_err: false,
            import_to_reaction_formula: None,
        };
        st.refresh_search(db);
        st
    }

    fn default_naf_fit_points() -> Vec<FitCpPointInput> {
        // S6 Validation dataset: NaF Kelley points (cal/(mol*K))
        let pts_t = [298.15, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0, 1200.0];
        pts_t
            .iter()
            .map(|&t| {
                let cp = 11.166 + 1.652e-3 * t - 0.594e5 / (t * t) + 1.546e-6 * t * t;
                FitCpPointInput {
                    t_str: format!("{t:.2}"),
                    cp_str: format!("{cp:.4}"),
                }
            })
            .collect()
    }

    #[inline]
    fn energy_scale_to_display(&self) -> f64 {
        if self.use_calories { 1.0 / CAL_TO_JOULE } else { 1.0 }
    }

    #[inline]
    fn energy_unit(&self) -> &'static str {
        if self.use_calories { "kcal/mol" } else { "kJ/mol" }
    }

    #[inline]
    fn heat_cap_unit(&self) -> &'static str {
        if self.use_calories { "cal/(mol*K)" } else { "J/(mol*K)" }
    }

    #[inline]
    fn temp_unit(&self) -> &'static str {
        if self.use_kelvin { "K" } else { "°C" }
    }

    fn fmt_temp(&self, t_k: f64) -> String {
        let v = if self.use_kelvin { t_k } else { t_k - 273.15 };
        format!("{v:.2}")
    }

    fn set_status(&mut self, msg: impl Into<String>, is_err: bool) {
        self.status_msg = msg.into();
        self.status_is_err = is_err;
    }

    /// Execute active search mode against `ThermoDatabase`.
    pub fn refresh_search(&mut self, db: &ThermoDatabase) {
        self.phase_filters.carbon_counts = parse_carbon_limits(&self.carbon_limit_text);
        let res = match self.query_mode {
            BrowserQueryMode::Keyword => db.search_advanced(
                &self.keyword_text,
                self.keyword_field,
                self.source_filter,
                &self.phase_filters,
                self.results_limit,
            ),
            BrowserQueryMode::ElementsSome | BrowserQueryMode::ElementsAll => {
                let mut elems: Vec<String> = self
                    .elements_text
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                for sel in &self.selected_elements {
                    if !elems.iter().any(|e| e == sel) {
                        elems.push(sel.clone());
                    }
                }
                let mode = if self.query_mode == BrowserQueryMode::ElementsAll {
                    ElementSearchMode::All
                } else {
                    ElementSearchMode::Some
                };
                db.search_elements_filtered(
                    &elems,
                    mode,
                    self.source_filter,
                    &self.phase_filters,
                    self.results_limit,
                )
            }
            BrowserQueryMode::Stoichiometry => {
                match db.search_stoichiometry(
                    &self.stoich_text,
                    self.fix_ratio,
                    self.source_filter,
                    &self.phase_filters,
                    self.results_limit,
                ) {
                    Ok(list) => list,
                    Err(e) => {
                        self.set_status(format!("Stoichiometry parse error: {e}"), true);
                        Vec::new()
                    }
                }
            }
        };
        self.cached_results = res;
        self.results_dirty = false;
        if let Some(first) = self.cached_results.first() {
            let still_present = self
                .selected_formula
                .as_ref()
                .map(|sf| self.cached_results.iter().any(|e| &e.formula == sf))
                .unwrap_or(false);
            if !still_present {
                self.selected_formula = Some(first.formula.clone());
                self.selected_source = Some(first.source);
            }
        }
    }

    /// Populate `OwnDB` Editor fields from any resolved or layer species entry.
    pub fn load_into_editor(&mut self, sp: &ChemicalSpecies, meta: &SpeciesMetadata, raw_phase: &str) {
        let e_scale = self.energy_scale_to_display();
        let intervals = sp
            .intervals
            .iter()
            .map(|iv| EditableIntervalRow {
                t1: self.fmt_temp(iv.t_min),
                t2: self.fmt_temp(iv.t_max),
                phase: iv.phase,
                h_disp: format!("{:.3}", (iv.h_or_delta_h_tr / 1000.0) * e_scale),
                s_disp: format!("{:.3}", iv.s_or_delta_s_tr * e_scale),
                a_disp: format!("{:.4}", iv.kelley.a * e_scale),
                b_disp: format!("{:.4}", iv.kelley.b * e_scale),
                c_disp: format!("{:.4}", iv.kelley.c * e_scale),
                d_disp: format!("{:.4}", iv.kelley.d * e_scale),
                reference: iv.reference.clone(),
            })
            .collect();

        self.editor = EditableSpeciesForm {
            formula: sp.formula.clone(),
            structural_formula: meta.structural_formula.clone(),
            chemical_name: sp.name.clone(),
            common_name: meta.common_name.clone(),
            cas_number: meta.cas_number.clone(),
            mw_str: format!("{:.4}", sp.molecular_weight),
            density_str: format!("{:.3}", sp.density_g_cm3),
            mp_k_str: format!("{:.2}", meta.melting_point_k),
            bp_k_str: format!("{:.2}", meta.boiling_point_k),
            color_code: meta.color_code,
            solubility: meta.solubility.clone(),
            reliability_class: meta.reliability_class.max(1),
            raw_phase_code: raw_phase.to_string(),
            intervals,
        };
    }

    fn save_editor_to_owndb(&mut self, db: &mut ThermoDatabase) {
        let formula = self.editor.formula.trim().to_string();
        if formula.is_empty() {
            self.set_status("Formula cannot be empty when saving to OwnDB.", true);
            return;
        }
        let inv_scale = if self.use_calories { CAL_TO_JOULE } else { 1.0 };
        let parse_f = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);

        let mut mw = parse_f(&self.editor.mw_str);
        let mut charge = 0;
        if let Ok((calc_mw, _, z)) = SoryntelApplication::species_compute_mw(&formula) {
            if mw <= 0.0 {
                mw = calc_mw;
                self.editor.mw_str = format!("{mw:.4}");
            }
            charge = z;
        }

        let mut intervals = Vec::new();
        for row in &self.editor.intervals {
            let mut t1 = parse_f(&row.t1);
            let mut t2 = parse_f(&row.t2);
            if !self.use_kelvin {
                t1 += 273.15;
                t2 += 273.15;
            }
            if t2 <= t1 {
                self.set_status(
                    format!("Invalid interval [{}..{}] K: T2 must exceed T1.", t1, t2),
                    true,
                );
                return;
            }
            intervals.push(SpeciesThermoInterval {
                t_min: t1,
                t_max: t2,
                phase: row.phase,
                kelley: KelleyCoefficients {
                    a: parse_f(&row.a_disp) * inv_scale,
                    b: parse_f(&row.b_disp) * inv_scale,
                    c: parse_f(&row.c_disp) * inv_scale,
                    d: parse_f(&row.d_disp) * inv_scale,
                },
                h_or_delta_h_tr: parse_f(&row.h_disp) * 1000.0 * inv_scale,
                s_or_delta_s_tr: parse_f(&row.s_disp) * inv_scale,
                reference: row.reference.trim().to_string(),
            });
        }
        if intervals.is_empty() {
            self.set_status("At least one temperature range is required.", true);
            return;
        }

        let sp = ChemicalSpecies {
            formula: formula.clone(),
            name: self.editor.chemical_name.trim().to_string(),
            phase: intervals[0].phase,
            molecular_weight: mw,
            density_g_cm3: parse_f(&self.editor.density_str),
            charge,
            intervals,
        };
        let meta = SpeciesMetadata {
            structural_formula: self.editor.structural_formula.trim().to_string(),
            common_name: self.editor.common_name.trim().to_string(),
            cas_number: self.editor.cas_number.trim().to_string(),
            melting_point_k: parse_f(&self.editor.mp_k_str),
            boiling_point_k: parse_f(&self.editor.bp_k_str),
            color_code: self.editor.color_code,
            solubility: if self.editor.solubility.trim().is_empty() {
                "-".into()
            } else {
                self.editor.solubility.trim().to_string()
            },
            reliability_class: self.editor.reliability_class.clamp(1, 10),
        };
        let raw_ph = self.editor.raw_phase_code.clone();
        db.upsert_own_dataset(sp, meta, raw_ph);
        self.selected_formula = Some(formula.clone());
        self.selected_source = Some(DbSource::Own);
        self.refresh_search(db);
        self.set_status(
            format!("Saved '{formula}' to active OwnDB (OwnDB > MainDB precedence active across all modules)."),
            false,
        );
    }

    /// Main UI entry point. Returns optional formula imported for Reaction Equations module.
    pub fn ui(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) -> Option<String> {
        if self.results_dirty {
            self.refresh_search(db);
        }

        self.show_menu_bar(ctx, db);
        self.show_status_footer(ctx, db);

        if self.show_help_dialog {
            let mut open = true;
            egui::Window::new("HSC Databases (Module 16) Help")
                .open(&mut open)
                .default_width(520.0)
                .show(ctx, |ui| {
                    ui.strong("Module 16: Species & Thermochemical Databases (HSC 6.0 Ch. 28)");
                    ui.separator();
                    ui.label("• Precedence: Calculation modules always resolve species from OwnDB first, then MainDB.");
                    ui.label("• Search Modes: Elements (Some / All), Stoichiometry (Fix Ratio OFF = isomer/mineral match e.g. SiO4FeFe -> *2FeO*SiO2; ON = empirical ratio e.g. AsS -> As2S2), Keyword (Formula, Structural, Name, Common, CAS).");
                    ui.label("• Organic Filter: Unchecking 'Organic' skips molecules with H > 0 and C > 2 atoms.");
                    ui.label("• Cp Tools: Kelley least-squares fitting with Y/N parameter masks & external Cp (Shomate/NASA-7/DIPPR/Custom) 10-interval conversion.");
                    ui.label("• G(T) Converter: Converts G(T) polynomial -> (H, S, A, B, C, D) and runs Delta to Normal from reference elements.");
                });
            self.show_help_dialog = open;
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            SpeciesViewTab::Browser => self.show_browser_tab(ui, db),
            SpeciesViewTab::Editor => self.show_editor_tab(ui, db),
            SpeciesViewTab::FitCp => self.show_fit_cp_tab(ui),
            SpeciesViewTab::ConvertCp => self.show_convert_cp_tab(ui),
            SpeciesViewTab::ConvertGibbs => self.show_convert_gibbs_tab(ui, db),
            SpeciesViewTab::DbTools => self.show_db_tools_tab(ui, db),
        });

        self.import_to_reaction_formula.take()
    }

    fn show_menu_bar(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        egui::TopBottomPanel::top("species_top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(crate::i18n::text("common.001"), |ui| {
                    if ui.button("Open OwnDB (.HSC)...").clicked() {
                        self.active_tab = SpeciesViewTab::DbTools;
                        ui.close_menu();
                    }
                    if ui.button("Save OwnDB (.HSC)").clicked() {
                        let path = self.own_path_input.clone();
                        match db.save_own_hsc(&path) {
                            Ok(n) => self.set_status(format!("Saved {n} OwnDB datasets -> {path}"), false),
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export Resolved View -> CSV").clicked() {
                        let csv = db.export_csv(self.source_filter);
                        match crate::storage::write_atomic(&self.csv_io_path, csv) {
                            Ok(_) => self.set_status(format!("Exported CSV -> {}", self.csv_io_path), false),
                            Err(e) => self.set_status(format!("CSV write failed: {e}"), true),
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.006"), |ui| {
                    if ui.button("New Blank Species in OwnDB Editor").clicked() {
                        self.editor = EditableSpeciesForm::default();
                        self.active_tab = SpeciesViewTab::Editor;
                        ui.close_menu();
                    }
                    if ui.button("Edit Selected Species in OwnDB").clicked() {
                        self.load_selected_into_editor(db);
                        self.active_tab = SpeciesViewTab::Editor;
                        ui.close_menu();
                    }
                    if ui.button("Delete Selected from OwnDB").clicked() {
                        if let Some(f) = self.selected_formula.clone() {
                            if db.delete_own_species(&f) {
                                self.refresh_search(db);
                                self.set_status(format!("Deleted '{f}' from OwnDB."), false);
                            } else {
                                self.set_status(format!("'{f}' is in MainDB (read-only), not OwnDB."), true);
                            }
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.028"), |ui| {
                    ui.radio_value(&mut self.use_calories, true, "Units: Calories (kcal, cal)");
                    ui.radio_value(&mut self.use_calories, false, "Units: Joules (kJ, J)");
                    ui.separator();
                    ui.radio_value(&mut self.use_kelvin, true, "Temperature: K");
                    ui.radio_value(&mut self.use_kelvin, false, "Temperature: °C");
                });
                ui.menu_button(crate::i18n::text("common.018"), |ui| {
                    for (tab, lbl) in [
                        (SpeciesViewTab::FitCp, "Fit Cp Data (Kelley OLS)"),
                        (SpeciesViewTab::ConvertCp, "Convert Cp Functions"),
                        (SpeciesViewTab::ConvertGibbs, "Convert G(T) -> H, S, Cp"),
                        (SpeciesViewTab::DbTools, "Merge / Legacy HSC 2-5 / .CAT"),
                    ] {
                        if ui.button(lbl).clicked() {
                            self.active_tab = tab;
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Databases", |ui| {
                    if ui.button("Active Layer: OwnDB > MainDB").clicked() {
                        self.source_filter = None;
                        self.refresh_search(db);
                        ui.close_menu();
                    }
                    if ui.button("Show OwnDB Only").clicked() {
                        self.source_filter = Some(DbSource::Own);
                        self.refresh_search(db);
                        ui.close_menu();
                    }
                    if ui.button("Show MainDB Only").clicked() {
                        self.source_filter = Some(DbSource::Main);
                        self.refresh_search(db);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Sort OwnDB Alphabetically").clicked() {
                        db.sort_database(DbSource::Own);
                        self.refresh_search(db);
                        self.set_status("OwnDB sorted canonically.", false);
                        ui.close_menu();
                    }
                    if ui.button("Refresh Active .CAT Indexes").clicked() {
                        let m_res = db.refresh_catalogue(DbSource::Main);
                        let o_res = db.refresh_catalogue(DbSource::Own);
                        self.set_status(
                            format!("Catalogue refresh — Main: {:?}, Own: {:?}", m_res.is_ok(), o_res.is_ok()),
                            false,
                        );
                        ui.close_menu();
                    }
                });
                ui.menu_button(crate::i18n::text("common.003"), |ui| {
                    if ui.button("Module 16 Guide & Validation Reference").clicked() {
                        self.show_help_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.separator();
                for (tab, label) in [
                    (SpeciesViewTab::Browser, "Browser & Search"),
                    (SpeciesViewTab::Editor, "Species Editor (OwnDB)"),
                    (SpeciesViewTab::FitCp, "Fit Cp Data"),
                    (SpeciesViewTab::ConvertCp, "Convert Cp"),
                    (SpeciesViewTab::ConvertGibbs, "Convert G(T)"),
                    (SpeciesViewTab::DbTools, "DB Tools & Refs"),
                ] {
                    if ui.selectable_label(self.active_tab == tab, label).clicked() {
                        self.active_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let prev_c = self.use_calories;
                    let prev_k = self.use_kelvin;
                    ui.radio_value(&mut self.use_kelvin, false, "°C");
                    ui.radio_value(&mut self.use_kelvin, true, "K");
                    ui.separator();
                    ui.radio_value(&mut self.use_calories, true, "cal");
                    ui.radio_value(&mut self.use_calories, false, "J");
                    if (prev_c != self.use_calories || prev_k != self.use_kelvin)
                        && self.active_tab == SpeciesViewTab::Editor
                    {
                        self.load_selected_into_editor(db);
                    }
                });
            });
        });
    }

    fn show_status_footer(&self, ctx: &egui::Context, db: &ThermoDatabase) {
        let sum = db.status_summary();
        egui::TopBottomPanel::bottom("species_status_footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let col = if self.status_is_err {
                    egui::Color32::from_rgb(185, 28, 28)
                } else {
                    egui::Color32::from_rgb(21, 128, 61)
                };
                ui.colored_label(col, &self.status_msg);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "MainDB: {} sp / {} rec  |  OwnDB: {} sp  |  Refs: {}",
                            sum.main_unique_species,
                            sum.main_binary_records,
                            sum.own_unique_species,
                            sum.references_count
                        ))
                        .small()
                        .color(egui::Color32::from_rgb(71, 85, 105)),
                    );
                });
            });
        });
    }

    fn load_selected_into_editor(&mut self, db: &ThermoDatabase) {
        if let Some(ref f) = self.selected_formula.clone() {
            let src = self.selected_source;
            if let Some(entry) = self
                .cached_results
                .iter()
                .find(|e| &e.formula == f && (src.is_none() || Some(e.source) == src))
                .cloned()
            {
                self.load_into_editor(&entry.species, &entry.metadata, &entry.raw_phase_code);
                return;
            }
            if let Some(sp) = db.get_species(f) {
                let meta = db.get_species_metadata(f).unwrap_or_default();
                self.load_into_editor(&sp, &meta, "");
            }
        }
    }

    fn show_browser_tab(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        let mut changed = false;

        ui.columns(3, |cols| {
            // Column 1: HSC Database Menu & Search Controls
            cols[0].set_max_width(300.0);
            cols[0].strong("Search Controls (Ch. 28.5–28.10)");
            cols[0].horizontal(|ui| {
                ui.label("Layer:");
                if ui.selectable_label(self.source_filter.is_none(), "Active").clicked() {
                    self.source_filter = None;
                    changed = true;
                }
                if ui.selectable_label(self.source_filter == Some(DbSource::Own), "OwnDB").clicked() {
                    self.source_filter = Some(DbSource::Own);
                    changed = true;
                }
                if ui.selectable_label(self.source_filter == Some(DbSource::Main), "MainDB").clicked() {
                    self.source_filter = Some(DbSource::Main);
                    changed = true;
                }
            });

            cols[0].horizontal_wrapped(|ui| {
                changed |= ui.radio_value(&mut self.query_mode, BrowserQueryMode::Keyword, "Keyword / List").changed();
                changed |= ui.radio_value(&mut self.query_mode, BrowserQueryMode::ElementsSome, "Elements (Some)").changed();
                changed |= ui.radio_value(&mut self.query_mode, BrowserQueryMode::ElementsAll, "Elements (All)").changed();
                changed |= ui.radio_value(&mut self.query_mode, BrowserQueryMode::Stoichiometry, "Stoichiometry").changed();
            });
            cols[0].separator();

            match self.query_mode {
                BrowserQueryMode::Keyword => {
                    cols[0].horizontal(|ui| {
                        ui.label("Key word:");
                        if ui.text_edit_singleline(&mut self.keyword_text).changed() {
                            changed = true;
                        }
                    });
                    cols[0].horizontal_wrapped(|ui| {
                        ui.label("Field:");
                        for (fld, lbl) in [
                            (KeywordSearchField::Any, "Any"),
                            (KeywordSearchField::Formula, "Formula"),
                            (KeywordSearchField::ChemicalName, "Name"),
                            (KeywordSearchField::CommonName, "Common"),
                            (KeywordSearchField::StructuralFormula, "Struct"),
                            (KeywordSearchField::CasNumber, "CAS"),
                        ] {
                            changed |= ui.radio_value(&mut self.keyword_field, fld, lbl).changed();
                        }
                    });
                }
                BrowserQueryMode::Stoichiometry => {
                    cols[0].horizontal(|ui| {
                        ui.label(crate::i18n::text("common.079"));
                        if ui.text_edit_singleline(&mut self.stoich_text).changed() {
                            changed = true;
                        }
                    });
                    changed |= cols[0].checkbox(&mut self.fix_ratio, "Fix Ratio of Elements (AsS -> As2S2)").changed();
                }
                BrowserQueryMode::ElementsSome | BrowserQueryMode::ElementsAll => {
                    cols[0].horizontal(|ui| {
                        ui.label("Elements:");
                        if ui.text_edit_singleline(&mut self.elements_text).changed() {
                            changed = true;
                        }
                    });
                    cols[0].horizontal(|ui| {
                        if ui.button(if self.show_periodic_picker { "Hide Periodic Picker" } else { "Periodic Picker..." }).clicked() {
                            self.show_periodic_picker = !self.show_periodic_picker;
                        }
                        if ui.button("Clear Elements").clicked() {
                            self.selected_elements.clear();
                            self.elements_text.clear();
                            changed = true;
                        }
                    });
                }
            }

            cols[0].separator();
            cols[0].strong("Search Mode Filters");
            cols[0].horizontal_wrapped(|ui| {
                changed |= ui.checkbox(&mut self.phase_filters.solid, "Condensed (s)").changed();
                changed |= ui.checkbox(&mut self.phase_filters.liquid, "Liquids (l)").changed();
                changed |= ui.checkbox(&mut self.phase_filters.gas, "Gases (g)").changed();
                changed |= ui.checkbox(&mut self.phase_filters.aqua_ions, "Aq Ions").changed();
                changed |= ui.checkbox(&mut self.phase_filters.aqua_neutrals, "Aq Neutrals").changed();
            });
            if self.phase_filters.aqua_neutrals {
                cols[0].horizontal(|ui| {
                    changed |= ui.checkbox(&mut self.phase_filters.include_ao, "ao ((a))").changed();
                    changed |= ui.checkbox(&mut self.phase_filters.include_ai, "ai ((ia))").changed();
                });
            }
            cols[0].horizontal(|ui| {
                changed |= ui.checkbox(&mut self.phase_filters.organic, "Organic (C>2 & H)").changed();
                ui.label("C count:");
                if ui
                    .add(egui::TextEdit::singleline(&mut self.carbon_limit_text).desired_width(60.0))
                    .changed()
                {
                    changed = true;
                }
            });

            if self.show_periodic_picker
                && matches!(
                    self.query_mode,
                    BrowserQueryMode::ElementsSome | BrowserQueryMode::ElementsAll
                )
            {
                cols[0].separator();
                cols[0].label("Toggle elements:");
                egui::ScrollArea::vertical()
                    .id_salt("species_periodic_picker")
                    .max_height(190.0)
                    .show(&mut cols[0], |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for el in PERIODIC_ELEMENTS.iter() {
                                let is_sel = self.selected_elements.contains(el.symbol);
                                let btn = egui::Button::new(
                                    egui::RichText::new(el.symbol).size(11.0).strong(),
                                )
                                .min_size(egui::vec2(28.0, 20.0))
                                .fill(if is_sel {
                                    egui::Color32::from_rgb(191, 219, 254)
                                } else {
                                    egui::Color32::from_rgb(241, 245, 249)
                                });
                                if ui.add(btn).on_hover_text(el.name).clicked() {
                                    if is_sel {
                                        self.selected_elements.remove(el.symbol);
                                    } else {
                                        self.selected_elements.insert(el.symbol.to_string());
                                    }
                                    changed = true;
                                }
                            }
                        });
                    });
            }

            if cols[0].button("Search / Refresh Results").clicked() {
                changed = true;
            }
            if changed {
                self.refresh_search(db);
            }

            // Column 2: Matching Species List
            cols[1].set_max_width(270.0);
            cols[1].horizontal(|ui| {
                ui.strong(format!("Species ({})", self.cached_results.len()));
                if self.cached_results.len() >= self.results_limit {
                    if ui.small_button("+500").clicked() {
                        self.results_limit += 500;
                        self.refresh_search(db);
                    }
                }
            });
            egui::ScrollArea::vertical()
                .id_salt("species_browser_list")
                .max_height(520.0)
                .show(&mut cols[1], |ui| {
                    for entry in &self.cached_results {
                        let is_sel = self.selected_formula.as_deref() == Some(&entry.formula)
                            && self.selected_source == Some(entry.source);
                        let badge = match entry.source {
                            DbSource::Own => "[Own]",
                            DbSource::Main => "[Main]",
                            DbSource::OpenDefault => "[Def]",
                        };
                        let label = format!("{}  {}", entry.formula, badge);
                        let resp = ui.selectable_label(is_sel, label);
                        if resp.clicked() {
                            self.selected_formula = Some(entry.formula.clone());
                            self.selected_source = Some(entry.source);
                        }
                        if resp.double_clicked() {
                            self.import_to_reaction_formula = Some(entry.formula.clone());
                        }
                    }
                });

            // Column 3: Detailed Species Datasheet (19 fields + intervals + citations + Criss-Cobble)
            let avail_w = cols[2].available_width();
            cols[2].set_min_width(avail_w.max(430.0));
            if let Some(sel) = self
                .selected_formula
                .as_ref()
                .and_then(|sf| {
                    self.cached_results.iter().find(|e| &e.formula == sf).cloned()
                })
            {
                cols[2].horizontal(|ui| {
                    ui.strong(format!("{} — {}", sel.formula, sel.species.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("Import to Reaction")
                                    .fill(egui::Color32::from_rgb(220, 252, 231)),
                            )
                            .clicked()
                        {
                            self.import_to_reaction_formula = Some(sel.formula.clone());
                            let _ = crate::storage::write_atomic(
                                crate::ui_common::temp_file_path("soryntel_import_formula.txt"),
                                &sel.formula,
                            );
                        }
                        if ui.button("Edit Copy in OwnDB").clicked() {
                            self.load_into_editor(&sel.species, &sel.metadata, &sel.raw_phase_code);
                            self.active_tab = SpeciesViewTab::Editor;
                        }
                    });
                });

                // Dataset selector when formula has alternative discontinuous literature sets (e.g. Mg(+2a))
                if sel.dataset_count > 1 {
                    let mut chosen = 0usize;
                    let all_sets = db.get_all_datasets_for(sel.source, &sel.formula);
                    cols[2].horizontal(|ui| {
                        ui.strong(format!("Literature Datasets ({}):", all_sets.len()));
                        for (idx, ds) in all_sets.iter().enumerate() {
                            let ref_hint = ds
                                .species
                                .intervals
                                .first()
                                .map(|iv| iv.reference.as_str())
                                .unwrap_or("-");
                            if ui
                                .selectable_label(false, format!("#{} ({})", idx + 1, ref_hint))
                                .clicked()
                            {
                                chosen = idx;
                                db.select_dataset(sel.source, &sel.formula, chosen);
                                self.refresh_search(db);
                            }
                        }
                    });
                }

                // 19 binary fields summary card
                let cspec = color_spec(sel.metadata.color_code);
                egui::Grid::new("browser_19_fields_grid")
                    .spacing([14.0, 4.0])
                    .show(&mut cols[2], |ui| {
                        ui.label(crate::i18n::text("common.080"));
                        ui.monospace(&sel.metadata.structural_formula);
                        ui.label("Molecular Weight:");
                        ui.monospace(format!("{:.4} g/mol", sel.species.molecular_weight));
                        ui.end_row();

                        ui.label(crate::i18n::text("common.081"));
                        ui.label(&sel.metadata.common_name);
                        ui.label("CAS Number:");
                        ui.monospace(&sel.metadata.cas_number);
                        ui.end_row();

                        ui.label("Density / Solubility:");
                        ui.monospace(format!(
                            "{:.3} g/cm³  |  Sol: {}",
                            sel.species.density_g_cm3, sel.metadata.solubility
                        ));
                        ui.label("Mp / Bp:");
                        ui.monospace(format!(
                            "{} / {} {}",
                            self.fmt_temp(sel.metadata.melting_point_k),
                            self.fmt_temp(sel.metadata.boiling_point_k),
                            self.temp_unit()
                        ));
                        ui.end_row();

                        ui.label("Phase / Charge:");
                        ui.monospace(format!(
                            "{:?} ({}) | z = {}",
                            sel.species.phase, sel.raw_phase_code, sel.species.charge
                        ));
                        ui.label("Color / Reliability:");
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                rect,
                                2.0,
                                egui::Color32::from_rgb(cspec.rgb[0], cspec.rgb[1], cspec.rgb[2]),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                            );
                            ui.monospace(format!(
                                "{} ({})  |  Rel: {}",
                                cspec.code, cspec.name, sel.metadata.reliability_class
                            ));
                        });
                        ui.end_row();
                    });

                if let Some(aq) = SoryntelApplication::species_analyze_aqueous(&sel.species) {
                    let s_scale = self.energy_scale_to_display();
                    let s_u = self.heat_cap_unit();
                    cols[2].separator();
                    cols[2].label(format!(
                        "Aqueous Scale (28.3–28.4): S°_conv = {:.3}, S_abs(-5·z) = {:.3}, ΔS_mix = {:.3} {s_u} | Criss–Cobble active: {}",
                        aq.s_conv_298_j * s_scale,
                        aq.s_abs_298_j * s_scale,
                        aq.delta_s_mix_j * s_scale,
                        aq.criss_cobble_active
                    ));
                }

                cols[2].separator();
                cols[2].strong(format!("Temperature Ranges ({})", sel.species.intervals.len()));
                let e_scale = self.energy_scale_to_display();
                let e_unit = self.energy_unit();
                let s_unit = self.heat_cap_unit();
                let t_u = self.temp_unit();

                egui::ScrollArea::both()
                    .id_salt("browser_intervals_table")
                    .max_height(210.0)
                    .show(&mut cols[2], |ui| {
                        egui::Grid::new("browser_iv_grid")
                            .striped(true)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong("#");
                                ui.strong(format!("T1 ({t_u})"));
                                ui.strong(format!("T2 ({t_u})"));
                                ui.strong("Ph");
                                ui.strong(format!("H ({e_unit})"));
                                ui.strong(format!("S ({s_unit})"));
                                ui.strong("A");
                                ui.strong("B");
                                ui.strong("C");
                                ui.strong("D");
                                ui.strong("Reference");
                                ui.end_row();

                                for (idx, iv) in sel.species.intervals.iter().enumerate() {
                                    ui.monospace(format!("{}", idx + 1));
                                    ui.monospace(self.fmt_temp(iv.t_min));
                                    ui.monospace(self.fmt_temp(iv.t_max));
                                    ui.monospace(format!("{:?}", iv.phase));
                                    ui.monospace(format!("{:.3}", (iv.h_or_delta_h_tr / 1000.0) * e_scale));
                                    ui.monospace(format!("{:.3}", iv.s_or_delta_s_tr * e_scale));
                                    ui.monospace(format!("{:.4}", iv.kelley.a * e_scale));
                                    ui.monospace(format!("{:.4}", iv.kelley.b * e_scale));
                                    ui.monospace(format!("{:.4}", iv.kelley.c * e_scale));
                                    ui.monospace(format!("{:.4}", iv.kelley.d * e_scale));
                                    ui.label(&iv.reference);
                                    ui.end_row();
                                }
                            });
                    });

                // Reference citations lookup from MainDB6.REF
                if let Some(first_iv) = sel.species.intervals.first() {
                    let cites = db.lookup_citations(&first_iv.reference);
                    if !cites.is_empty() {
                        cols[2].separator();
                        cols[2].strong("Resolved Citations (MainDB6.REF)");
                        egui::ScrollArea::vertical()
                            .id_salt("browser_citations_scroll")
                            .max_height(105.0)
                            .show(&mut cols[2], |ui| {
                                for c in cites {
                                    ui.label(
                                        egui::RichText::new(format!("• [{}] {}", c.key, c.citation))
                                            .small(),
                                    );
                                }
                            });
                    }
                }
            } else {
                cols[2].label("Select a species on the left to inspect its 19 HSC fields and thermochemical ranges.");
            }
        });
    }

    fn show_editor_tab(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        ui.horizontal(|ui| {
            ui.strong("OwnDB Species Editor (Ch. 28.11)");
            ui.separator();
            if ui.button("Load Selected").clicked() {
                self.load_selected_into_editor(db);
            }
            if ui.button("New Blank Species").clicked() {
                self.editor = EditableSpeciesForm::default();
            }
            if ui.button("Calc MW").clicked() {
                match SoryntelApplication::species_compute_mw(&self.editor.formula) {
                    Ok((mw, _, _)) => {
                        self.editor.mw_str = format!("{mw:.4}");
                        self.set_status(format!("Calculated MW = {mw:.4} g/mol"), false);
                    }
                    Err(e) => self.set_status(format!("MW calculation failed: {e}"), true),
                }
            }
            if ui.button("Extrapolate X -> X(l) @ 298.15 K").clicked() {
                self.run_liquid_extrapolation();
            }
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Save to OwnDB").strong())
                        .fill(egui::Color32::from_rgb(220, 252, 231)),
                )
                .clicked()
            {
                self.save_editor_to_owndb(db);
            }
            if ui.button("Persist OwnDB File (.HSC)").clicked() {
                self.save_editor_to_owndb(db);
                let p = self.own_path_input.clone();
                match db.save_own_hsc(&p) {
                    Ok(n) => self.set_status(format!("Persisted {n} OwnDB datasets -> {p}"), false),
                    Err(e) => self.set_status(format!("{e}"), true),
                }
            }
            if ui.button("Delete from OwnDB").clicked() {
                let f = self.editor.formula.clone();
                if db.delete_own_species(&f) {
                    self.refresh_search(db);
                    self.set_status(format!("Deleted '{f}' from OwnDB."), false);
                } else {
                    self.set_status(format!("'{f}' not present in OwnDB."), true);
                }
            }
        });
        ui.separator();

        egui::Grid::new("editor_basic_data_grid")
            .spacing([12.0, 5.0])
            .show(ui, |ui| {
                ui.label("Formula (24 ch):");
                ui.text_edit_singleline(&mut self.editor.formula);
                ui.label("Structural Formula (32 ch):");
                ui.text_edit_singleline(&mut self.editor.structural_formula);
                ui.label(crate::i18n::text("common.082"));
                ui.text_edit_singleline(&mut self.editor.mw_str);
                ui.end_row();

                ui.label("Chemical Name (52 ch):");
                ui.text_edit_singleline(&mut self.editor.chemical_name);
                ui.label("Common Name (32 ch):");
                ui.text_edit_singleline(&mut self.editor.common_name);
                ui.label("CAS Number (12 ch):");
                ui.text_edit_singleline(&mut self.editor.cas_number);
                ui.end_row();

                ui.label("Density (g/cm³):");
                ui.text_edit_singleline(&mut self.editor.density_str);
                ui.label("Melting Pt (K):");
                ui.text_edit_singleline(&mut self.editor.mp_k_str);
                ui.label("Boiling Pt (K):");
                ui.text_edit_singleline(&mut self.editor.bp_k_str);
                ui.end_row();

                ui.label("Solubility (14 ch):");
                ui.text_edit_singleline(&mut self.editor.solubility);
                ui.label("Reliability Class (1..10):");
                ui.add(egui::DragValue::new(&mut self.editor.reliability_class).range(1..=10));
                ui.label("Color (0..53):");
                ui.horizontal(|ui| {
                    let c = color_spec(self.editor.color_code);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, egui::Color32::from_rgb(c.rgb[0], c.rgb[1], c.rgb[2]));
                    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                    if ui.button(format!("{} ({})", c.code, c.name)).clicked() {
                        self.show_color_palette = !self.show_color_palette;
                    }
                });
                ui.end_row();
            });

        if self.show_color_palette {
            ui.separator();
            ui.label("Select HSC Color Code (0..53):");
            ui.horizontal_wrapped(|ui| {
                for spec in HSC_COLOR_PALETTE.iter() {
                    let bg = egui::Color32::from_rgb(spec.rgb[0], spec.rgb[1], spec.rgb[2]);
                    let btn = egui::Button::new(
                        egui::RichText::new(format!("{}", spec.code))
                            .size(10.5)
                            .color(if (spec.rgb[0] as u16 + spec.rgb[1] as u16 + spec.rgb[2] as u16) > 380 {
                                egui::Color32::BLACK
                            } else {
                                egui::Color32::WHITE
                            }),
                    )
                    .fill(bg)
                    .min_size(egui::vec2(26.0, 20.0));
                    if ui.add(btn).on_hover_text(spec.name).clicked() {
                        self.editor.color_code = spec.code;
                        self.show_color_palette = false;
                    }
                }
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(format!(
                "Temperature Intervals ({} / {}, Row 0 = Hf298, S298; Row >= 1 = dHtr, dStr)",
                self.energy_unit(),
                self.heat_cap_unit()
            ));
            if ui.button("+ Add Interval").clicked() && self.editor.intervals.len() < 12 {
                let mut row = EditableIntervalRow::default();
                if let Some(last) = self.editor.intervals.last() {
                    row.t1 = last.t2.clone();
                }
                self.editor.intervals.push(row);
            }
            if ui.button("- Remove Last").clicked() && self.editor.intervals.len() > 1 {
                self.editor.intervals.pop();
            }
            if ui.button("Auto dS = dH / Ttr (Rows >= 1)").clicked() {
                let is_cal = self.use_calories;
                for row in self.editor.intervals.iter_mut().skip(1) {
                    let t1 = row.t1.trim().parse::<f64>().unwrap_or(0.0);
                    let t1_k = if self.use_kelvin { t1 } else { t1 + 273.15 };
                    let dh = row.h_disp.trim().parse::<f64>().unwrap_or(0.0);
                    if t1_k > 0.0 {
                        let ds = (dh * 1000.0) / t1_k;
                        let _ = is_cal;
                        row.s_disp = format!("{ds:.3}");
                    }
                }
            }
        });

        let e_u = self.energy_unit();
        let s_u = self.heat_cap_unit();
        let t_u = self.temp_unit();
        egui::ScrollArea::both()
            .id_salt("editor_intervals_scroll")
            .max_height(290.0)
            .show(ui, |ui| {
                egui::Grid::new("editor_iv_grid")
                    .striped(true)
                    .spacing([6.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("#");
                        ui.strong(format!("T1 ({t_u})"));
                        ui.strong(format!("T2 ({t_u})"));
                        ui.strong("Phase");
                        ui.strong(format!("H / dH ({e_u})"));
                        ui.strong(format!("S / dS ({s_u})"));
                        ui.strong("A");
                        ui.strong("B");
                        ui.strong("C");
                        ui.strong("D");
                        ui.strong("Reference (36 ch)");
                        ui.end_row();

                        for (idx, row) in self.editor.intervals.iter_mut().enumerate() {
                            ui.monospace(format!("{}", idx + 1));
                            ui.add(egui::TextEdit::singleline(&mut row.t1).desired_width(64.0));
                            ui.add(egui::TextEdit::singleline(&mut row.t2).desired_width(64.0));
                            egui::ComboBox::from_id_salt(format!("ed_ph_{idx}"))
                                .selected_text(format!("{:?}", row.phase))
                                .width(85.0)
                                .show_ui(ui, |ui| {
                                    for ph in [
                                        Phase::Solid,
                                        Phase::Liquid,
                                        Phase::Gas,
                                        Phase::AqueousIon,
                                        Phase::AqueousNeutral,
                                    ] {
                                        ui.selectable_value(&mut row.phase, ph, format!("{ph:?}"));
                                    }
                                });
                            ui.add(egui::TextEdit::singleline(&mut row.h_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.s_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.a_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.b_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.c_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.d_disp).desired_width(72.0));
                            ui.add(egui::TextEdit::singleline(&mut row.reference).desired_width(170.0));
                            ui.end_row();
                        }
                    });
            });
    }

    fn run_liquid_extrapolation(&mut self) {
        let liq_idx = self.editor.intervals.iter().position(|r| r.phase == Phase::Liquid);
        let Some(li) = liq_idx else {
            self.set_status("Need a Liquid interval following solid intervals to extrapolate X -> X(l).", true);
            return;
        };
        if li == 0 {
            self.set_status("Species already starts with Liquid phase at row 0.", false);
            return;
        }
        let inv_scale = if self.use_calories { CAL_TO_JOULE } else { 1.0 };
        let parse_f = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);
        let solids: Vec<SpeciesThermoInterval> = self.editor.intervals[..li]
            .iter()
            .map(|r| {
                let mut t1 = parse_f(&r.t1);
                let mut t2 = parse_f(&r.t2);
                if !self.use_kelvin {
                    t1 += 273.15;
                    t2 += 273.15;
                }
                SpeciesThermoInterval {
                    t_min: t1,
                    t_max: t2,
                    phase: r.phase,
                    kelley: KelleyCoefficients {
                        a: parse_f(&r.a_disp) * inv_scale,
                        b: parse_f(&r.b_disp) * inv_scale,
                        c: parse_f(&r.c_disp) * inv_scale,
                        d: parse_f(&r.d_disp) * inv_scale,
                    },
                    h_or_delta_h_tr: parse_f(&r.h_disp) * 1000.0 * inv_scale,
                    s_or_delta_s_tr: parse_f(&r.s_disp) * inv_scale,
                    reference: r.reference.clone(),
                }
            })
            .collect();
        let liq_row = &self.editor.intervals[li];
        let mut t_melt_k = parse_f(&liq_row.t1);
        if !self.use_kelvin {
            t_melt_k += 273.15;
        }
        let dh_fus_j = parse_f(&liq_row.h_disp) * 1000.0 * inv_scale;
        let liq_k = KelleyCoefficients {
            a: parse_f(&liq_row.a_disp) * inv_scale,
            b: parse_f(&liq_row.b_disp) * inv_scale,
            c: parse_f(&liq_row.c_disp) * inv_scale,
            d: parse_f(&liq_row.d_disp) * inv_scale,
        };
        match SoryntelApplication::species_extrapolate_liquid_298(&solids, t_melt_k, dh_fus_j, liq_k) {
            Ok((h_l_298, s_l_298)) => {
                let e_scale = self.energy_scale_to_display();
                let base = self.editor.formula.trim_end_matches("(l)").to_string();
                let mut new_row = liq_row.clone();
                new_row.t1 = self.fmt_temp(T_REF_K);
                new_row.h_disp = format!("{:.3}", (h_l_298 / 1000.0) * e_scale);
                new_row.s_disp = format!("{:.3}", s_l_298 * e_scale);
                self.editor.formula = format!("{base}(l)");
                self.editor.raw_phase_code = "l".into();
                self.editor.intervals = vec![new_row];
                self.set_status(
                    format!("Extrapolated '{base}(l)' down to 298.15 K (Hf298={}, S298={}). Review & Save to OwnDB.", self.editor.intervals[0].h_disp, self.editor.intervals[0].s_disp),
                    false,
                );
            }
            Err(e) => self.set_status(format!("Liquid extrapolation failed: {e}"), true),
        }
    }

    fn show_fit_cp_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Fit Cp Data — Kelley Least Squares (Ch. 28.12 Fig. 11)");
            ui.separator();
            if ui.button("Load NaF Validation Preset (S6)").clicked() {
                self.use_calories = true;
                self.use_kelvin = true;
                self.fit_points = Self::default_naf_fit_points();
                self.fit_mask = CpFitMask::default();
                self.fit_result = None;
            }
            if ui.button("+ Add Point").clicked() {
                self.fit_points.push(FitCpPointInput {
                    t_str: "1300.0".into(),
                    cp_str: "15.0".into(),
                });
            }
            if ui.button("Fit Cp Curve").clicked() {
                self.run_fit_cp();
            }
            if ui.button("Copy A..D -> OwnDB Editor row 1").clicked() {
                if let Some(ref res) = self.fit_result {
                    let s = self.energy_scale_to_display();
                    if let Some(r0) = self.editor.intervals.first_mut() {
                        r0.a_disp = format!("{:.4}", res.kelley_si.a * s);
                        r0.b_disp = format!("{:.4}", res.kelley_si.b * s);
                        r0.c_disp = format!("{:.4}", res.kelley_si.c * s);
                        r0.d_disp = format!("{:.4}", res.kelley_si.d * s);
                    }
                    self.active_tab = SpeciesViewTab::Editor;
                }
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.strong("Parameter Mask (Y = Optimize, N = Lock value):");
            ui.checkbox(&mut self.fit_mask.fit_a, "A");
            if !self.fit_mask.fit_a {
                ui.add(egui::TextEdit::singleline(&mut self.lock_a).desired_width(60.0));
            }
            ui.checkbox(&mut self.fit_mask.fit_b, "B");
            if !self.fit_mask.fit_b {
                ui.add(egui::TextEdit::singleline(&mut self.lock_b).desired_width(60.0));
            }
            ui.checkbox(&mut self.fit_mask.fit_c, "C");
            if !self.fit_mask.fit_c {
                ui.add(egui::TextEdit::singleline(&mut self.lock_c).desired_width(60.0));
            }
            ui.checkbox(&mut self.fit_mask.fit_d, "D");
            if !self.fit_mask.fit_d {
                ui.add(egui::TextEdit::singleline(&mut self.lock_d).desired_width(60.0));
            }
        });
        ui.separator();

        let s_scale = self.energy_scale_to_display();
        let s_u = self.heat_cap_unit();
        let t_u = self.temp_unit();

        ui.columns(2, |cols| {
            egui::ScrollArea::vertical()
                .id_salt("fit_cp_points_scroll")
                .max_height(380.0)
                .show(&mut cols[0], |ui| {
                    egui::Grid::new("fit_cp_pts_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("#");
                            ui.strong(format!("T ({t_u})"));
                            ui.strong(format!("Cp exp ({s_u})"));
                            ui.strong(format!("Cp fit ({s_u})"));
                            ui.strong("Error %");
                            ui.end_row();
                            for (i, pt) in self.fit_points.iter_mut().enumerate() {
                                ui.monospace(format!("{}", i + 1));
                                ui.add(egui::TextEdit::singleline(&mut pt.t_str).desired_width(80.0));
                                ui.add(egui::TextEdit::singleline(&mut pt.cp_str).desired_width(90.0));
                                if let Some(ref res) = self.fit_result {
                                    let fit_v = res.fitted_cp_j.get(i).copied().unwrap_or(0.0) * s_scale;
                                    let err_p = res.rel_error_pct.get(i).copied().unwrap_or(0.0);
                                    ui.monospace(format!("{fit_v:.4}"));
                                    ui.monospace(format!("{err_p:+.4}%"));
                                } else {
                                    ui.monospace("-");
                                    ui.monospace("-");
                                }
                                ui.end_row();
                            }
                        });
                });

            if let Some(ref res) = self.fit_result {
                cols[1].strong(format!(
                    "Fitted ({s_u}):  A = {:.4},  B = {:.4},  C = {:.4},  D = {:.4}  |  Max Error = {:.4}%",
                    res.kelley_si.a * s_scale,
                    res.kelley_si.b * s_scale,
                    res.kelley_si.c * s_scale,
                    res.kelley_si.d * s_scale,
                    res.max_abs_error_pct
                ));
                let inv_scale = if self.use_calories { CAL_TO_JOULE } else { 1.0 };
                let exp_pts: Vec<[f64; 2]> = self
                    .fit_points
                    .iter()
                    .filter_map(|p| {
                        let t = p.t_str.trim().parse::<f64>().ok()?;
                        let t_k = if self.use_kelvin { t } else { t + 273.15 };
                        let cp_j = p.cp_str.trim().parse::<f64>().ok()? * inv_scale;
                        Some([t_k, cp_j * s_scale])
                    })
                    .collect();
                let mut line_pts = Vec::new();
                if let (Some(t0), Some(t1)) = (exp_pts.first().map(|p| p[0]), exp_pts.last().map(|p| p[0])) {
                    for k in 0..=40 {
                        let tk = t0 + (k as f64 / 40.0) * (t1 - t0);
                        line_pts.push([tk, res.kelley_si.cp(tk) * s_scale]);
                    }
                }
                Plot::new("species_fit_cp_plot")
                    .height(320.0)
                    .x_axis_label("T (K)")
                    .y_axis_label(s_u)
                    .show(&mut cols[1], |plot_ui| {
                        plot_ui.line(Line::new(PlotPoints::from(line_pts)).name("Kelley Fit"));
                        plot_ui.points(Points::new(PlotPoints::from(exp_pts)).radius(4.0).name("Experimental"));
                    });
            } else {
                cols[1].label("Click 'Fit Cp Curve' to compute Kelley A, B, C, D and inspect residuals.");
            }
        });
    }

    fn run_fit_cp(&mut self) {
        let inv_scale = if self.use_calories { CAL_TO_JOULE } else { 1.0 };
        let pts: Vec<CpDataPoint> = self
            .fit_points
            .iter()
            .filter_map(|p| {
                let t = p.t_str.trim().parse::<f64>().ok()?;
                let t_k = if self.use_kelvin { t } else { t + 273.15 };
                let cp_j = p.cp_str.trim().parse::<f64>().ok()? * inv_scale;
                Some(CpDataPoint { t_k, cp_j })
            })
            .collect();
        let locked = KelleyCoefficients {
            a: self.lock_a.trim().parse::<f64>().unwrap_or(0.0) * inv_scale,
            b: self.lock_b.trim().parse::<f64>().unwrap_or(0.0) * inv_scale,
            c: self.lock_c.trim().parse::<f64>().unwrap_or(0.0) * inv_scale,
            d: self.lock_d.trim().parse::<f64>().unwrap_or(0.0) * inv_scale,
        };
        match SoryntelApplication::species_fit_cp(&pts, self.fit_mask, locked) {
            Ok(res) => {
                self.set_status(
                    format!("Cp fitted successfully (RMSE = {:.4} J/(mol·K), max error = {:.4}%).", res.rmse_j, res.max_abs_error_pct),
                    false,
                );
                self.fit_result = Some(res);
            }
            Err(e) => self.set_status(format!("Fit Cp failed: {e}"), true),
        }
    }

    fn show_convert_cp_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Convert Cp Functions -> HSC Format (Ch. 28.13 Fig. 12)");
            ui.separator();
            egui::ComboBox::from_label("Template")
                .selected_text(self.ext_template.label())
                .show_ui(ui, |ui| {
                    for tpl in [
                        ExternalCpTemplate::MaierKelleyStandard,
                        ExternalCpTemplate::ShomateNist,
                        ExternalCpTemplate::Nasa7,
                        ExternalCpTemplate::Dippr100,
                        ExternalCpTemplate::Custom,
                    ] {
                        if ui.selectable_value(&mut self.ext_template, tpl.clone(), tpl.label()).clicked() {
                            self.ext_expr = tpl.formula_string().into();
                        }
                    }
                });
            if ui
                .add(
                    egui::Button::new("Convert Cp Functions to HSC Format")
                        .fill(egui::Color32::from_rgb(219, 234, 254)),
                )
                .clicked()
            {
                self.run_convert_cp();
            }
            if ui.button("Copy A..D -> OwnDB Editor").clicked() {
                if let Some(ref res) = self.ext_result {
                    let s = self.energy_scale_to_display();
                    if let Some(r0) = self.editor.intervals.first_mut() {
                        r0.t1 = self.ext_t1.clone();
                        r0.t2 = self.ext_t2.clone();
                        r0.a_disp = format!("{:.4}", res.kelley_si.a * s);
                        r0.b_disp = format!("{:.4}", res.kelley_si.b * s);
                        r0.c_disp = format!("{:.4}", res.kelley_si.c * s);
                        r0.d_disp = format!("{:.4}", res.kelley_si.d * s);
                    }
                    self.active_tab = SpeciesViewTab::Editor;
                }
            }
        });
        ui.separator();

        egui::Grid::new("convert_cp_inputs_grid")
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                ui.label("Cp Function f(T):");
                ui.add(egui::TextEdit::singleline(&mut self.ext_expr).desired_width(380.0));
                ui.label("Source Units:");
                ui.add(egui::TextEdit::singleline(&mut self.ext_unit).desired_width(120.0));
                ui.end_row();

                ui.label("T1 (K) / T2 (K):");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.ext_t1).desired_width(80.0));
                    ui.label("..");
                    ui.add(egui::TextEdit::singleline(&mut self.ext_t2).desired_width(80.0));
                });
                ui.label("Params A..F:");
                ui.horizontal(|ui| {
                    for (idx, p) in self.ext_params.iter_mut().enumerate() {
                        ui.label(format!("{}:", (b'A' + idx as u8) as char));
                        ui.add(egui::TextEdit::singleline(p).desired_width(58.0));
                    }
                });
                ui.end_row();
            });
        ui.separator();

        if let Some(ref res) = self.ext_result {
            let s_scale = self.energy_scale_to_display();
            let s_u = self.heat_cap_unit();
            ui.strong(format!(
                "Converted Kelley ({s_u}): A = {:.4},  B = {:.4},  C = {:.4},  D = {:.4}  (11 nodes, max err = {:.4}%)",
                res.kelley_si.a * s_scale,
                res.kelley_si.b * s_scale,
                res.kelley_si.c * s_scale,
                res.kelley_si.d * s_scale,
                res.max_abs_error_pct
            ));
            let pts_plot: Vec<[f64; 2]> = self
                .ext_samples
                .iter()
                .map(|p| [p.t_k, p.cp_j * s_scale])
                .collect();
            let fit_plot: Vec<[f64; 2]> = self
                .ext_samples
                .iter()
                .zip(res.fitted_cp_j.iter())
                .map(|(p, &f)| [p.t_k, f * s_scale])
                .collect();
            Plot::new("species_convert_cp_plot")
                .height(310.0)
                .x_axis_label("T (K)")
                .y_axis_label(s_u)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::from(fit_plot)).name("Kelley HSC Fit"));
                    plot_ui.points(Points::new(PlotPoints::from(pts_plot)).radius(4.0).name("External f(T)"));
                });
        }
    }

    fn run_convert_cp(&mut self) {
        let t1 = self.ext_t1.trim().parse::<f64>().unwrap_or(298.15);
        let t2 = self.ext_t2.trim().parse::<f64>().unwrap_or(1500.0);
        let mut params = [0.0_f64; 6];
        for (i, s) in self.ext_params.iter().enumerate() {
            params[i] = s.trim().parse::<f64>().unwrap_or(0.0);
        }
        match SoryntelApplication::species_convert_cp_function(
            &self.ext_expr,
            &self.ext_unit,
            t1,
            t2,
            &params,
            self.ext_mask,
            KelleyCoefficients { a: 0.0, b: 0.0, c: 0.0, d: 0.0 },
        ) {
            Ok((pts, fit)) => {
                self.ext_samples = pts;
                self.set_status(
                    format!("Converted external Cp function over 10 subintervals (max error {:.4}%).", fit.max_abs_error_pct),
                    false,
                );
                self.ext_result = Some(fit);
            }
            Err(e) => self.set_status(format!("Convert Cp failed: {e}"), true),
        }
    }

    fn show_convert_gibbs_tab(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        ui.horizontal(|ui| {
            ui.strong("Convert G(T) -> H, S, Cp & Delta to Normal (Ch. 28.14 Fig. 13)");
            ui.separator();
            if ui.button("Load Cu2S Validation Preset (S7)").clicked() {
                self.use_calories = true;
                self.gibbs_is_delta = true;
                self.gibbs_formula = "Cu2S".into();
                self.gibbs_h = "-14.394".into();
                self.gibbs_s = "-86.625".into();
                self.gibbs_a = "13.574".into();
                self.gibbs_b = "-17.817".into();
                self.gibbs_c = "-185.3".into();
                self.gibbs_d = "0.0".into();
            }
            if ui.button("Convert G(T)").clicked() {
                self.run_convert_gibbs();
            }
            if ui.button("Delta to Normal").clicked() {
                self.run_convert_gibbs();
                if let Some(ref d_out) = self.gibbs_delta_out {
                    match SoryntelApplication::species_gibbs_delta_to_normal(
                        db,
                        &self.gibbs_formula,
                        d_out,
                        T_REF_K,
                    ) {
                        Ok(norm) => {
                            self.gibbs_normal_out = Some(norm);
                            self.set_status(
                                format!("Computed normal (Hf, S°, A, B, C, D) for '{}' from reference elements.", self.gibbs_formula),
                                false,
                            );
                        }
                        Err(e) => self.set_status(format!("Delta to Normal failed: {e}"), true),
                    }
                }
            }
            if ui.button("Save Result -> OwnDB Editor").clicked() {
                let target = self.gibbs_normal_out.or(self.gibbs_delta_out);
                if let Some(out) = target {
                    let s = self.energy_scale_to_display();
                    self.editor.formula = self.gibbs_formula.clone();
                    if let Some(r0) = self.editor.intervals.first_mut() {
                        r0.t1 = "298.15".into();
                        r0.h_disp = format!("{:.3}", (out.h_ref_j / 1000.0) * s);
                        r0.s_disp = format!("{:.3}", out.s_ref_j * s);
                        r0.a_disp = format!("{:.4}", out.kelley_si.a * s);
                        r0.b_disp = format!("{:.4}", out.kelley_si.b * s);
                        r0.c_disp = format!("{:.4}", out.kelley_si.c * s);
                        r0.d_disp = format!("{:.4}", out.kelley_si.d * s);
                    }
                    self.active_tab = SpeciesViewTab::Editor;
                }
            }
        });
        ui.separator();

        ui.label("G(T) = h + s·10⁻³·T + a·10⁻³·T·ln(T) + b·10⁻⁶·T² + c·T⁻¹ + d·10⁻⁹·T³   (at T_ref = 298.15 K)");
        egui::Grid::new("gibbs_input_grid")
            .spacing([12.0, 5.0])
            .show(ui, |ui| {
                ui.checkbox(&mut self.gibbs_is_delta, "Input is Delta G(T)");
                ui.label("Target Formula:");
                ui.text_edit_singleline(&mut self.gibbs_formula);
                ui.end_row();

                ui.label("h (const):");
                ui.text_edit_singleline(&mut self.gibbs_h);
                ui.label("s (×10⁻³ T):");
                ui.text_edit_singleline(&mut self.gibbs_s);
                ui.label("a (×10⁻³ T·lnT):");
                ui.text_edit_singleline(&mut self.gibbs_a);
                ui.end_row();

                ui.label("b (×10⁻⁶ T²):");
                ui.text_edit_singleline(&mut self.gibbs_b);
                ui.label("c (T⁻¹):");
                ui.text_edit_singleline(&mut self.gibbs_c);
                ui.label("d (×10⁻⁹ T³):");
                ui.text_edit_singleline(&mut self.gibbs_d);
                ui.end_row();
            });
        ui.separator();

        let e_scale = self.energy_scale_to_display();
        let e_u = self.energy_unit();
        let s_u = self.heat_cap_unit();

        if let Some(ref d) = self.gibbs_delta_out {
            ui.group(|ui| {
                ui.strong("Converted Output (G(T) -> H, S, Cp at 298.15 K)");
                ui.monospace(format!(
                    "H = {:.3} {e_u}   |   S = {:.3} {s_u}   |   G = {:.3} {e_u}",
                    (d.h_ref_j / 1000.0) * e_scale,
                    d.s_ref_j * e_scale,
                    (d.g_ref_j / 1000.0) * e_scale
                ));
                ui.monospace(format!(
                    "A = {:.4},   B = {:.4},   C = {:.4},   D = {:.4}  {s_u}",
                    d.kelley_si.a * e_scale,
                    d.kelley_si.b * e_scale,
                    d.kelley_si.c * e_scale,
                    d.kelley_si.d * e_scale
                ));
            });
        }
        if let Some(ref n) = self.gibbs_normal_out {
            ui.group(|ui| {
                ui.strong(format!("Delta to Normal Result for '{}'", self.gibbs_formula));
                ui.monospace(format!(
                    "Hf°(298.15) = {:.3} {e_u}   |   S°(298.15) = {:.3} {s_u}   |   G°(298.15) = {:.3} {e_u}",
                    (n.h_ref_j / 1000.0) * e_scale,
                    n.s_ref_j * e_scale,
                    (n.g_ref_j / 1000.0) * e_scale
                ));
                ui.monospace(format!(
                    "A = {:.4},   B = {:.4},   C = {:.4},   D = {:.4}  {s_u}",
                    n.kelley_si.a * e_scale,
                    n.kelley_si.b * e_scale,
                    n.kelley_si.c * e_scale,
                    n.kelley_si.d * e_scale
                ));
            });
        }
    }

    fn run_convert_gibbs(&mut self) {
        let parse = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0);
        let inp = GibbsCoeffsInput {
            h: parse(&self.gibbs_h),
            s: parse(&self.gibbs_s),
            a: parse(&self.gibbs_a),
            b: parse(&self.gibbs_b),
            c: parse(&self.gibbs_c),
            d: parse(&self.gibbs_d),
            is_calories: self.use_calories,
            t_ref_k: T_REF_K,
        };
        match SoryntelApplication::species_convert_gibbs(&inp) {
            Ok(out) => {
                self.gibbs_delta_out = Some(out);
                self.set_status("Converted G(T) polynomial to (H, S, A, B, C, D).", false);
            }
            Err(e) => self.set_status(format!("G(T) conversion error: {e}"), true),
        }
    }

    fn show_db_tools_tab(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        egui::ScrollArea::vertical().id_salt("db_tools_main_scroll").show(ui, |ui| {
            ui.strong("Active Databases & Paths (Ch. 28.17)");
            egui::Grid::new("db_paths_grid").spacing([10.0, 5.0]).show(ui, |ui| {
                ui.label("MainDB (.HSC):");
                ui.add(egui::TextEdit::singleline(&mut self.main_path_input).desired_width(420.0));
                if ui.button("Open MainDB").clicked() {
                    match db.load_main_hsc(&self.main_path_input) {
                        Ok((u, r)) => {
                            self.refresh_search(db);
                            self.set_status(format!("Loaded MainDB: {u} species ({r} records)."), false);
                        }
                        Err(e) => self.set_status(format!("{e}"), true),
                    }
                }
                ui.end_row();

                ui.label("OwnDB (.HSC):");
                ui.add(egui::TextEdit::singleline(&mut self.own_path_input).desired_width(420.0));
                ui.horizontal(|ui| {
                    if ui.button("Open OwnDB").clicked() {
                        match db.load_own_hsc(&self.own_path_input) {
                            Ok((u, r)) => {
                                self.refresh_search(db);
                                self.set_status(format!("Loaded OwnDB: {u} species ({r} slots)."), false);
                            }
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                    }
                    if ui.button("Save OwnDB").clicked() {
                        match db.save_own_hsc(&self.own_path_input) {
                            Ok(n) => self.set_status(format!("Saved {n} OwnDB datasets."), false),
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                    }
                    if ui.button("Create New Empty OwnDB").clicked() {
                        match db.create_empty_own_hsc(&self.own_path_input) {
                            Ok(_) => {
                                self.refresh_search(db);
                                self.set_status(format!("Created fresh 256-byte OwnDB at {}", self.own_path_input), false);
                            }
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                    }
                });
                ui.end_row();

                ui.label("Citations (.REF):");
                ui.add(egui::TextEdit::singleline(&mut self.ref_path_input).desired_width(420.0));
                if ui.button("Load .REF").clicked() {
                    match db.load_ref_dictionary(&self.ref_path_input) {
                        Ok(n) => self.set_status(format!("Loaded {n} citations from .REF."), false),
                        Err(e) => self.set_status(format!("{e}"), true),
                    }
                }
                ui.end_row();
            });

            ui.separator();
            ui.strong("Merge Databases (HSC1 -> HSC2), Legacy HSC 2.0–5.0 Upgrade & CSV/JSON (Ch. 28.15–28.16)");
            egui::Grid::new("db_merge_legacy_grid").spacing([10.0, 5.0]).show(ui, |ui| {
                ui.label("Merge Source (HSC1):");
                ui.add(egui::TextEdit::singleline(&mut self.merge_src_path).desired_width(300.0));
                ui.label("Target (HSC2):");
                ui.add(egui::TextEdit::singleline(&mut self.merge_dst_path).desired_width(300.0));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.merge_show_duplicates, "Show Duplicates");
                    if ui.button("Inspect / Merge").clicked() {
                        match ThermoDatabase::merge_hsc_files(
                            &self.merge_src_path,
                            &self.merge_dst_path,
                            &self.merge_decisions,
                            self.merge_show_duplicates && self.merge_conflicts.is_empty(),
                        ) {
                            Ok((n, conflicts)) => {
                                if !conflicts.is_empty() && self.merge_show_duplicates && self.merge_decisions.is_empty() {
                                    self.merge_conflicts = conflicts;
                                    self.set_status(
                                        format!("Found {} duplicate formulas — resolve below then click Apply Merge.", self.merge_conflicts.len()),
                                        false,
                                    );
                                } else {
                                    self.merge_conflicts.clear();
                                    self.set_status(format!("Merged successfully ({n} datasets written to {}).", self.merge_dst_path), false);
                                }
                            }
                            Err(e) => self.set_status(format!("Merge failed: {e}"), true),
                        }
                    }
                });
                ui.end_row();

                ui.label("Legacy HSC 2–5 -> 6:");
                ui.add(egui::TextEdit::singleline(&mut self.legacy_src_path).desired_width(300.0));
                ui.label("Output HSC 6:");
                ui.add(egui::TextEdit::singleline(&mut self.legacy_dst_path).desired_width(300.0));
                if ui.button("Convert Legacy -> HSC 6").clicked() {
                    match ThermoDatabase::convert_legacy_to_hsc6(&self.legacy_src_path, &self.legacy_dst_path) {
                        Ok(n) => self.set_status(format!("Converted {n} datasets -> sorted HSC 6.0 + .CAT at {}", self.legacy_dst_path), false),
                        Err(e) => self.set_status(format!("Legacy convert failed: {e}"), true),
                    }
                }
                ui.end_row();

                ui.label("CSV Interchange:");
                ui.add(egui::TextEdit::singleline(&mut self.csv_io_path).desired_width(300.0));
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::text("common.031")).clicked() {
                        let csv = db.export_csv(self.source_filter);
                        match crate::storage::write_atomic(&self.csv_io_path, csv) {
                            Ok(_) => self.set_status(format!("CSV saved -> {}", self.csv_io_path), false),
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                    }
                    if ui.button("Import CSV -> OwnDB").clicked() {
                        match std::fs::read_to_string(&self.csv_io_path)
                            .map_err(|e| format!("{e}"))
                            .and_then(|s| db.import_csv_into_own(&s).map_err(|e| format!("{e}")))
                        {
                            Ok(n) => {
                                self.refresh_search(db);
                                self.set_status(format!("Imported {n} datasets into OwnDB from CSV."), false);
                            }
                            Err(e) => self.set_status(format!("CSV import error: {e}"), true),
                        }
                    }
                });
                ui.end_row();
            });

            if !self.merge_conflicts.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(format!("Duplicate Species Resolution ({})", self.merge_conflicts.len()));
                    if ui.button("Apply Merge Decisions").clicked() {
                        match ThermoDatabase::merge_hsc_files(
                            &self.merge_src_path,
                            &self.merge_dst_path,
                            &self.merge_decisions,
                            false,
                        ) {
                            Ok((n, _)) => {
                                self.merge_conflicts.clear();
                                self.set_status(format!("Merge applied ({n} total datasets)."), false);
                            }
                            Err(e) => self.set_status(format!("{e}"), true),
                        }
                    }
                });
                for c in &self.merge_conflicts {
                    let entry = self
                        .merge_decisions
                        .entry(c.formula.clone())
                        .or_insert(MergeConflictDecision::TakeSource);
                    ui.horizontal(|ui| {
                        ui.monospace(&c.formula);
                        ui.radio_value(entry, MergeConflictDecision::TakeSource, "Take Source");
                        ui.radio_value(entry, MergeConflictDecision::TakeTarget, "Take Target");
                    });
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.strong("Bibliographic Dictionary (MainDB6.REF)");
                ui.add(egui::TextEdit::singleline(&mut self.ref_search_query).hint_text("Filter key or author (e.g. Barin, JANAF, SGTE)"));
            });
            let refs: Vec<ReferenceEntry> = db.search_references(&self.ref_search_query, 150);
            egui::ScrollArea::vertical()
                .id_salt("ref_dict_scroll")
                .max_height(210.0)
                .show(ui, |ui| {
                    for r in refs {
                        ui.horizontal(|ui| {
                            ui.strong(&r.key);
                            ui.label(&r.citation);
                        });
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_species_ui_state_search_and_owndb_edit() {
        let mut db = ThermoDatabase::with_open_defaults();
        let mut st = SpeciesModuleState::new(&db);

        // Run Fit Cp preset (S6 NaF)
        st.run_fit_cp();
        let fit = st.fit_result.as_ref().unwrap();
        assert!(fit.max_abs_error_pct < 0.01);

        // Run Convert G(T) -> Delta to Normal (Cu2S / H2O-style open defaults)
        st.run_convert_gibbs();
        let gibbs = st.gibbs_delta_out.unwrap();
        assert_relative_approx(gibbs.kelley_si.a / CAL_TO_JOULE, -13.574);

        // Edit in OwnDB Editor and verify propagation into active DB
        st.editor.formula = "Al2O3".into();
        st.editor.chemical_name = "Alumina Custom Test".into();
        st.save_editor_to_owndb(&mut db);
        assert_eq!(db.get_species("Al2O3").unwrap().name, "Alumina Custom Test");
    }

    fn assert_relative_approx(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-3, "actual={actual}, expected={expected}");
    }
}
