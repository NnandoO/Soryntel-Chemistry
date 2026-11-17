use std::collections::{BTreeMap, BTreeSet, HashMap};

use eframe::egui;
use serde::{Deserialize, Serialize};
use soryntel_db::ThermoDatabase as SpeciesDb;
use soryntel_thermo::{
    ehph::{
        calculate_ehph, delta_g_formation_h2o_kcal, water_dielectric_constant, EhPhRequest,
        EhPhResult, ElementSpec, PotentialScale,
    },
    evaluate_species_si, ChemicalSpecies, Phase,
};

use crate::excel_grid::{
    header_cell, row_num_cell, section_cell, text_cell, widget_cell, EXCEL_ALT_ROW_BG,
    EXCEL_GRID_BORDER, EXCEL_HEADER_BG, EXCEL_READONLY_BG,
};
use crate::ui_common::{
    draw_dashed_line, draw_rotated_y_title, format_tick, nice_step, render_chart_tabs,
    ChartTabAction,
};

/// 5-column periodic-style element grid matching HSC6 EpH Fig. 1.
const ELEMENT_GRID: &[&str] = &[
    "Ac", "Cr", "K", "Po", "Th",
    "Ag", "Cs", "Kr", "Pr", "Ti",
    "Al", "Cu", "La", "Pt", "Tl",
    "Am", "Dy", "Li", "Pu", "Tm",
    "Ar", "Er", "Lu", "Ra", "U",
    "As", "Es", "Mg", "Rb", "V",
    "At", "Eu", "Mn", "Re", "W",
    "Au", "F", "Mo", "Rh", "Xe",
    "B", "Fe", "N", "Rn", "Y",
    "Ba", "Fm", "Na", "Ru", "Yb",
    "Be", "Fr", "Nb", "S", "Zn",
    "Bi", "Ga", "Nd", "Sb", "Zr",
    "Bk", "Gd", "Ne", "Sc", "T",
    "Br", "Ge", "Ni", "Se", "",
    "C", "H", "Np", "Si", "",
    "Ca", "He", "O", "Sm", "",
    "Cd", "Hf", "Os", "Sn", "",
    "Ce", "Hg", "P", "Sr", "",
    "Cf", "Ho", "Pa", "Ta", "",
    "Cl", "I", "Pb", "Tb", "",
    "Cm", "In", "Pd", "Tc", "",
    "Co", "Ir", "Pm", "Te", "",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EpHViewMode {
    Diagram,
    Combiner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Chart(usize),
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CombinerLineText {
    MainElement,
    Temperature,
    Molality,
    Pressure,
    DatasetNumber,
    None,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LabelOverride {
    custom_text: Option<String>,
    pos_ph_eh: Option<(f64, f64)>,
    font_size: Option<f32>,
    color: Option<[u8; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum H2oLineStyle {
    Dashed,
    Solid,
    Dotted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct H2oFormatConfig {
    style: H2oLineStyle,
    width: f32,
    custom_color: Option<[u8; 3]>,
}

impl Default for H2oFormatConfig {
    fn default() -> Self {
        Self {
            style: H2oLineStyle::Dashed,
            width: 1.5,
            custom_color: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EhPhSaveFile {
    version: u32,
    elements: Vec<ElementSpec>,
    main_element: String,
    selected_formulas: Vec<String>,
    mode_gases: bool,
    mode_gas_ions: bool,
    mode_liquids: bool,
    mode_condensed: bool,
    mode_aq_neutrals: bool,
    mode_aq_ions: bool,
    mode_organics: bool,
    temp_value: f64,
    temp_in_kelvin: bool,
    dielectric_str: String,
    delta_g_h2o_str: String,
    use_davies: bool,
    ionic_strength_str: String,
    correction_factor_str: String,
    criss_cobble: bool,
    min_eh_str: String,
    max_eh_str: String,
    min_ph_str: String,
    max_ph_str: String,
    potential_scale: PotentialScale,
    show_predominance_ions: bool,
    show_colors: bool,
    delta_g_overrides_kcal: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
struct ChartPage {
    name: String,
    result: EhPhResult,
    elements: Vec<ElementSpec>,
    main_element: String,
    min_ph: f64,
    max_ph: f64,
    min_eh: f64,
    max_eh: f64,
    potential_scale: PotentialScale,
    show_colors: bool,
    show_predominance_ions: bool,
    label_overrides: HashMap<String, LabelOverride>,
    label_font_size: f32,
    label_bold: bool,
}

#[derive(Debug, Clone)]
struct CombinerDataset {
    id: usize,
    main_element: String,
    temp_c: f64,
    temp_str: String,
    elements: Vec<ElementSpec>,
    mol_bufs: HashMap<String, String>,
    pres_bufs: HashMap<String, String>,
    result: Option<EhPhResult>,
    error: Option<String>,
}

impl CombinerDataset {
    fn new(id: usize, main_element: String, temp_c: f64, elements: Vec<ElementSpec>) -> Self {
        let mut mol_bufs = HashMap::new();
        let mut pres_bufs = HashMap::new();
        for el in &elements {
            mol_bufs.insert(el.symbol.clone(), format!("{:.2E}", el.molality));
            pres_bufs.insert(el.symbol.clone(), format!("{:.2E}", el.pressure_bar));
        }
        Self {
            id,
            main_element,
            temp_c,
            temp_str: format!("{temp_c:.2}"),
            elements,
            mol_bufs,
            pres_bufs,
            result: None,
            error: None,
        }
    }

    fn sync_from_buffers(&mut self) {
        if let Ok(t) = self.temp_str.trim().parse::<f64>() {
            self.temp_c = t;
        }
        for el in &mut self.elements {
            if let Some(s) = self.mol_bufs.get(&el.symbol) {
                if let Ok(v) = s.trim().parse::<f64>() {
                    if v > 0.0 {
                        el.molality = v;
                    }
                }
            }
            if let Some(s) = self.pres_bufs.get(&el.symbol) {
                if let Ok(v) = s.trim().parse::<f64>() {
                    if v > 0.0 {
                        el.pressure_bar = v;
                    }
                }
            }
        }
    }
}

pub struct EhPhModuleState {
    view_mode: EpHViewMode,
    // Left panel: Element & Species selection
    selected_elements: Vec<ElementSpec>,
    main_element: String,
    candidate_species: Vec<ChemicalSpecies>,
    selected_formulas: BTreeSet<String>,
    // Search Mode filters
    mode_gases: bool,
    mode_gas_ions: bool,
    mode_liquids: bool,
    mode_condensed: bool,
    mode_aq_neutrals: bool,
    mode_aq_ions: bool,
    mode_organics: bool,
    // Temperature & Parameters and Axes
    show_params_axes: bool,
    temp_value: f64,
    temp_str: String,
    temp_in_kelvin: bool,
    elem_mol_bufs: HashMap<String, String>,
    elem_pres_bufs: HashMap<String, String>,
    dielectric_str: String,
    delta_g_h2o_str: String,
    use_davies: bool,
    ionic_strength_str: String,
    correction_factor_str: String,
    criss_cobble: bool,
    min_eh_str: String,
    max_eh_str: String,
    min_ph_str: String,
    max_ph_str: String,
    potential_scale: PotentialScale,
    // Options
    show_predominance_ions: bool,
    create_new_page: bool,
    show_colors: bool,
    // Results & Tabs
    pages: Vec<ChartPage>,
    active_tab: RightTab,
    last_chart_idx: usize,
    status_error: Option<String>,
    status_info: Option<String>,
    show_print_dialog: bool,
    print_header: String,
    print_footer: String,
    print_landscape: bool,
    // Modals & Interactive formatting
    species_modal: Option<ChemicalSpecies>,
    dragging_label: Option<(usize, String)>,
    label_edit_modal: Option<(usize, String, String, f32, Option<[u8; 3]>)>,
    show_h2o_format: bool,
    h2o_format: H2oFormatConfig,
    default_label_font_size: f32,
    default_label_bold: bool,
    show_font_dialog: bool,
    show_file_modal: bool,
    file_modal_is_save: bool,
    file_path_buf: String,
    show_help_modal: bool,
    // Combiner state
    combiner_datasets: Vec<CombinerDataset>,
    combiner_line_text: CombinerLineText,
    combiner_show_table: bool,
    delta_g_overrides_kcal: BTreeMap<String, f64>,
    table_dg_bufs: BTreeMap<String, String>,
}

impl EhPhModuleState {
    pub fn new(db: &SpeciesDb) -> Self {
        let mut st = Self {
            view_mode: EpHViewMode::Diagram,
            selected_elements: vec![
                ElementSpec {
                    symbol: "Cu".to_string(),
                    molality: 1.0,
                    pressure_bar: 1.0,
                },
                ElementSpec {
                    symbol: "S".to_string(),
                    molality: 1.0,
                    pressure_bar: 1.0,
                },
            ],
            main_element: "Cu".to_string(),
            candidate_species: Vec::new(),
            selected_formulas: BTreeSet::new(),
            mode_gases: false,
            mode_gas_ions: false,
            mode_liquids: false,
            mode_condensed: true,
            mode_aq_neutrals: false,
            mode_aq_ions: true,
            mode_organics: false,
            show_params_axes: false,
            temp_value: 25.0,
            temp_str: "25.00".to_string(),
            temp_in_kelvin: false,
            elem_mol_bufs: HashMap::new(),
            elem_pres_bufs: HashMap::new(),
            dielectric_str: format!("{:.3}", water_dielectric_constant(298.15)),
            delta_g_h2o_str: "-56.678".to_string(),
            use_davies: false,
            ionic_strength_str: "0.000".to_string(),
            correction_factor_str: "1.000".to_string(),
            criss_cobble: false,
            min_eh_str: "-2.000".to_string(),
            max_eh_str: "2.000".to_string(),
            min_ph_str: "0.000".to_string(),
            max_ph_str: "14.000".to_string(),
            potential_scale: PotentialScale::Hydrogen,
            show_predominance_ions: false,
            create_new_page: false,
            show_colors: false,
            pages: Vec::new(),
            active_tab: RightTab::Chart(0),
            last_chart_idx: 0,
            status_error: None,
            status_info: None,
            show_print_dialog: false,
            print_header: "Cu - S - System at 25.00 °C".to_string(),
            print_footer: String::new(),
            print_landscape: false,
            species_modal: None,
            dragging_label: None,
            label_edit_modal: None,
            show_h2o_format: false,
            h2o_format: H2oFormatConfig::default(),
            default_label_font_size: 12.5,
            default_label_bold: false,
            show_font_dialog: false,
            show_file_modal: false,
            file_modal_is_save: true,
            file_path_buf: "/home/nando/.gemini/antigravity/scratch/diagram.eph8".to_string(),
            show_help_modal: false,
            combiner_datasets: vec![25.0, 50.0, 75.0]
                .into_iter()
                .enumerate()
                .map(|(i, temp_c)| {
                    CombinerDataset::new(
                        i + 1,
                        "Cu".to_string(),
                        temp_c,
                        vec![
                            ElementSpec {
                                symbol: "Cu".to_string(),
                                molality: 1.0,
                                pressure_bar: 1.0,
                            },
                            ElementSpec {
                                symbol: "S".to_string(),
                                molality: 1.0,
                                pressure_bar: 1.0,
                            },
                        ],
                    )
                })
                .collect(),
            combiner_line_text: CombinerLineText::Temperature,
            combiner_show_table: false,
            delta_g_overrides_kcal: BTreeMap::from([("FeOH(+a)".to_string(), -69.268)]),
            table_dg_bufs: BTreeMap::new(),
        };
        st.refresh_candidate_species(db, true);
        st.update_temp_derived(db);
        st.draw_current(db);
        st
    }

    fn t_k(&self) -> f64 {
        if self.temp_in_kelvin {
            self.temp_value.max(273.15)
        } else {
            (self.temp_value + 273.15).max(273.15)
        }
    }

    fn update_temp_derived(&mut self, db: &SpeciesDb) {
        let tk = self.t_k();
        self.dielectric_str = format!("{:.3}", water_dielectric_constant(tk));
        let dg = delta_g_formation_h2o_kcal(db, tk, self.criss_cobble);
        self.delta_g_h2o_str = format!("{dg:.3}");
    }

    fn query_species_for_elements(
        &self,
        db: &SpeciesDb,
        elements: &[ElementSpec],
    ) -> Vec<ChemicalSpecies> {
        let mut elems: Vec<String> = elements.iter().map(|e| e.symbol.clone()).collect();
        if !elems.iter().any(|e| e == "H") {
            elems.push("H".to_string());
        }
        if !elems.iter().any(|e| e == "O") {
            elems.push("O".to_string());
        }
        let mut list = db.search_by_elements(
            &elems,
            self.mode_gases || self.mode_gas_ions,
            self.mode_liquids,
            self.mode_condensed,
            self.mode_aq_ions,
            self.mode_aq_neutrals,
            2000,
        );
        list.retain(|sp| {
            if sp.phase == Phase::Gas {
                let keep_gas = (sp.charge == 0 && self.mode_gases)
                    || (sp.charge != 0 && self.mode_gas_ions);
                if !keep_gas {
                    return false;
                }
            }
            if !self.mode_organics && is_ehph_organic(&sp.formula) {
                return false;
            }
            true
        });
        list
    }

    fn refresh_candidate_species(&mut self, db: &SpeciesDb, select_all: bool) {
        self.candidate_species = self.query_species_for_elements(db, &self.selected_elements);
        if select_all {
            self.selected_formulas = self
                .candidate_species
                .iter()
                .map(|s| s.formula.clone())
                .collect();
        } else {
            let valid: BTreeSet<String> = self
                .candidate_species
                .iter()
                .map(|s| s.formula.clone())
                .collect();
            self.selected_formulas.retain(|f| valid.contains(f));
        }
    }

    fn build_request(&self) -> EhPhRequest {
        let selected_species: Vec<ChemicalSpecies> = self
            .candidate_species
            .iter()
            .filter(|s| self.selected_formulas.contains(&s.formula))
            .cloned()
            .collect();
        EhPhRequest {
            elements: self.selected_elements.clone(),
            main_element: self.main_element.clone(),
            species: selected_species,
            t_k: self.t_k(),
            min_ph: self.min_ph_str.trim().parse().unwrap_or(0.0),
            max_ph: self.max_ph_str.trim().parse().unwrap_or(14.0),
            min_eh_v: self.min_eh_str.trim().parse().unwrap_or(-2.0),
            max_eh_v: self.max_eh_str.trim().parse().unwrap_or(2.0),
            potential_scale: self.potential_scale,
            criss_cobble: self.criss_cobble,
            use_davies: self.use_davies,
            ionic_strength: self.ionic_strength_str.trim().parse().unwrap_or(0.0),
            correction_factor: self.correction_factor_str.trim().parse().unwrap_or(1.0),
            dielectric_constant: self.dielectric_str.trim().parse().ok(),
            delta_g_h2o_kcal: self.delta_g_h2o_str.trim().parse().ok(),
            delta_g_overrides_kcal: self.delta_g_overrides_kcal.clone(),
            show_predominance_ions: self.show_predominance_ions,
            grid_resolution: 180,
        }
    }

    fn sync_element_buffers(&mut self) {
        self.elem_mol_bufs.clear();
        self.elem_pres_bufs.clear();
        for el in &self.selected_elements {
            self.elem_mol_bufs
                .insert(el.symbol.clone(), format!("{:.2E}", el.molality));
            self.elem_pres_bufs
                .insert(el.symbol.clone(), format!("{:.2E}", el.pressure_bar));
        }
    }

    fn sync_table_dg_bufs_from_result(&mut self, res: &EhPhResult) {
        self.table_dg_bufs.clear();
        for r in &res.table_rows {
            self.table_dg_bufs
                .insert(r.formula.clone(), format!("{:.3}", r.delta_g_kcal_mol));
        }
    }

    fn active_chart_page(&self) -> Option<&ChartPage> {
        match self.active_tab {
            RightTab::Chart(idx) => self.pages.get(idx),
            RightTab::Table => self
                .pages
                .get(self.last_chart_idx)
                .or_else(|| self.pages.last()),
        }
    }

    fn draw_current(&mut self, db: &SpeciesDb) {
        for el in &mut self.selected_elements {
            if let Some(m_buf) = self.elem_mol_bufs.get(&el.symbol) {
                if let Ok(v) = m_buf.trim().parse::<f64>() {
                    if v > 0.0 {
                        el.molality = v;
                    }
                }
            }
            if let Some(p_buf) = self.elem_pres_bufs.get(&el.symbol) {
                if let Ok(v) = p_buf.trim().parse::<f64>() {
                    if v > 0.0 {
                        el.pressure_bar = v;
                    }
                }
            }
        }
        let req = self.build_request();
        match calculate_ehph(&req, Some(db)) {
            Ok(res) => {
                self.status_error = None;
                self.print_header = res.title.clone();
                self.sync_table_dg_bufs_from_result(&res);
                let page = ChartPage {
                    name: format!("Chart {}", self.pages.len() + 1),
                    result: res,
                    elements: req.elements,
                    main_element: req.main_element,
                    min_ph: req.min_ph,
                    max_ph: req.max_ph,
                    min_eh: req.min_eh_v,
                    max_eh: req.max_eh_v,
                    potential_scale: req.potential_scale,
                    show_colors: self.show_colors,
                    show_predominance_ions: req.show_predominance_ions,
                    label_overrides: HashMap::new(),
                    label_font_size: self.default_label_font_size,
                    label_bold: self.default_label_bold,
                };
                if self.create_new_page || self.pages.is_empty() {
                    let idx = self.pages.len();
                    self.pages.push(page);
                    self.active_tab = RightTab::Chart(idx);
                    self.last_chart_idx = idx;
                } else {
                    let target = match self.active_tab {
                        RightTab::Chart(i) if i < self.pages.len() => i,
                        _ if self.last_chart_idx < self.pages.len() => self.last_chart_idx,
                        _ => 0,
                    };
                    let keep_name = self.pages[target].name.clone();
                    self.pages[target] = page;
                    self.pages[target].name = keep_name;
                    self.active_tab = RightTab::Chart(target);
                    self.last_chart_idx = target;
                }
            }
            Err(err) => {
                self.status_error = Some(err);
            }
        }
    }

    fn draw_combiner(&mut self, db: &SpeciesDb) {
        let cur_syms: BTreeSet<&str> = self
            .selected_elements
            .iter()
            .map(|e| e.symbol.as_str())
            .collect();
        for ds in &mut self.combiner_datasets {
            ds.sync_from_buffers();
        }
        let mut results: Vec<Result<EhPhResult, String>> =
            Vec::with_capacity(self.combiner_datasets.len());
        let min_ph = self.min_ph_str.trim().parse().unwrap_or(0.0);
        let max_ph = self.max_ph_str.trim().parse().unwrap_or(14.0);
        let min_eh_v = self.min_eh_str.trim().parse().unwrap_or(-2.0);
        let max_eh_v = self.max_eh_str.trim().parse().unwrap_or(2.0);
        let ionic_strength = self.ionic_strength_str.trim().parse().unwrap_or(0.0);
        let correction_factor = self.correction_factor_str.trim().parse().unwrap_or(1.0);

        for ds in &self.combiner_datasets {
            let mut species = self.query_species_for_elements(db, &ds.elements);
            let ds_syms: BTreeSet<&str> = ds.elements.iter().map(|e| e.symbol.as_str()).collect();
            if ds_syms == cur_syms && !self.selected_formulas.is_empty() {
                species.retain(|sp| self.selected_formulas.contains(&sp.formula));
            }
            let req = EhPhRequest {
                elements: ds.elements.clone(),
                main_element: ds.main_element.clone(),
                species,
                t_k: (ds.temp_c + 273.15).max(273.15),
                min_ph,
                max_ph,
                min_eh_v,
                max_eh_v,
                potential_scale: self.potential_scale,
                criss_cobble: self.criss_cobble,
                use_davies: self.use_davies,
                ionic_strength,
                correction_factor,
                dielectric_constant: None,
                delta_g_h2o_kcal: None,
                delta_g_overrides_kcal: self.delta_g_overrides_kcal.clone(),
                show_predominance_ions: self.show_predominance_ions,
                grid_resolution: 160,
            };
            results.push(calculate_ehph(&req, Some(db)));
        }

        for (ds, outcome) in self.combiner_datasets.iter_mut().zip(results) {
            match outcome {
                Ok(res) => {
                    ds.result = Some(res);
                    ds.error = None;
                }
                Err(e) => {
                    ds.result = None;
                    ds.error = Some(e);
                }
            }
        }
    }

    fn load_preset(
        &mut self,
        db: &SpeciesDb,
        elements: Vec<(&str, f64, f64)>,
        main_element: &str,
        temp_c: f64,
        show_predominance_ions: bool,
    ) {
        self.selected_elements = elements
            .into_iter()
            .map(|(sym, m, p)| ElementSpec {
                symbol: sym.to_string(),
                molality: m,
                pressure_bar: p,
            })
            .collect();
        self.main_element = main_element.to_string();
        self.temp_in_kelvin = false;
        self.temp_value = temp_c;
        self.temp_str = format!("{temp_c:.2}");
        self.show_predominance_ions = show_predominance_ions;
        self.potential_scale = PotentialScale::Hydrogen;
        self.min_ph_str = "0.000".to_string();
        self.max_ph_str = "14.000".to_string();
        self.min_eh_str = "-2.000".to_string();
        self.max_eh_str = "2.000".to_string();
        self.sync_element_buffers();
        self.refresh_candidate_species(db, true);
        self.update_temp_derived(db);
        self.draw_current(db);
    }

    fn build_save_file(&self) -> EhPhSaveFile {
        EhPhSaveFile {
            version: 1,
            elements: self.selected_elements.clone(),
            main_element: self.main_element.clone(),
            selected_formulas: self.selected_formulas.iter().cloned().collect(),
            mode_gases: self.mode_gases,
            mode_gas_ions: self.mode_gas_ions,
            mode_liquids: self.mode_liquids,
            mode_condensed: self.mode_condensed,
            mode_aq_neutrals: self.mode_aq_neutrals,
            mode_aq_ions: self.mode_aq_ions,
            mode_organics: self.mode_organics,
            temp_value: self.temp_value,
            temp_in_kelvin: self.temp_in_kelvin,
            dielectric_str: self.dielectric_str.clone(),
            delta_g_h2o_str: self.delta_g_h2o_str.clone(),
            use_davies: self.use_davies,
            ionic_strength_str: self.ionic_strength_str.clone(),
            correction_factor_str: self.correction_factor_str.clone(),
            criss_cobble: self.criss_cobble,
            min_eh_str: self.min_eh_str.clone(),
            max_eh_str: self.max_eh_str.clone(),
            min_ph_str: self.min_ph_str.clone(),
            max_ph_str: self.max_ph_str.clone(),
            potential_scale: self.potential_scale,
            show_predominance_ions: self.show_predominance_ions,
            show_colors: self.show_colors,
            delta_g_overrides_kcal: self.delta_g_overrides_kcal.clone(),
        }
    }

    fn apply_save_file(&mut self, db: &SpeciesDb, sf: EhPhSaveFile) {
        self.selected_elements = sf.elements;
        self.main_element = sf.main_element;
        self.mode_gases = sf.mode_gases;
        self.mode_gas_ions = sf.mode_gas_ions;
        self.mode_liquids = sf.mode_liquids;
        self.mode_condensed = sf.mode_condensed;
        self.mode_aq_neutrals = sf.mode_aq_neutrals;
        self.mode_aq_ions = sf.mode_aq_ions;
        self.mode_organics = sf.mode_organics;
        self.temp_value = sf.temp_value;
        self.temp_in_kelvin = sf.temp_in_kelvin;
        self.temp_str = format!("{:.2}", self.temp_value);
        self.dielectric_str = sf.dielectric_str;
        self.delta_g_h2o_str = sf.delta_g_h2o_str;
        self.use_davies = sf.use_davies;
        self.ionic_strength_str = sf.ionic_strength_str;
        self.correction_factor_str = sf.correction_factor_str;
        self.criss_cobble = sf.criss_cobble;
        self.min_eh_str = sf.min_eh_str;
        self.max_eh_str = sf.max_eh_str;
        self.min_ph_str = sf.min_ph_str;
        self.max_ph_str = sf.max_ph_str;
        self.potential_scale = sf.potential_scale;
        self.show_predominance_ions = sf.show_predominance_ions;
        self.show_colors = sf.show_colors;
        self.delta_g_overrides_kcal = sf.delta_g_overrides_kcal;
        self.sync_element_buffers();
        self.refresh_candidate_species(db, false);
        let valid: BTreeSet<String> = self
            .candidate_species
            .iter()
            .map(|s| s.formula.clone())
            .collect();
        self.selected_formulas = sf
            .selected_formulas
            .into_iter()
            .filter(|f| valid.contains(f))
            .collect();
        if self.selected_formulas.is_empty() {
            self.refresh_candidate_species(db, true);
        }
        self.draw_current(db);
    }

    fn save_eph8(&mut self, path: &str) {
        let sf = self.build_save_file();
        match serde_json::to_string_pretty(&sf) {
            Ok(json) => match crate::storage::write_atomic(path, json) {
                Ok(_) => {
                    self.status_error = None;
                    self.status_info = Some(format!("Saved .eph8 to {path}"));
                }
                Err(e) => self.status_error = Some(format!("Failed saving {path}: {e}")),
            },
            Err(e) => self.status_error = Some(format!("Serialization error: {e}")),
        }
    }

    fn open_eph8(&mut self, db: &SpeciesDb, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<EhPhSaveFile>(&content) {
                Ok(sf) => {
                    self.apply_save_file(db, sf);
                    if self.status_error.is_none() {
                        self.status_info = Some(format!("Opened {path}"));
                    }
                }
                Err(e) => self.status_error = Some(format!("Invalid .eph8 file {path}: {e}")),
            },
            Err(e) => self.status_error = Some(format!("Cannot read {path}: {e}")),
        }
    }

    fn export_active_svg(&mut self, path: &str) {
        let Some(page) = self.active_chart_page() else {
            self.status_error = Some("No chart page to export.".to_string());
            return;
        };
        let (w, h, ml, mr, mt, mb) = (860.0_f64, 620.0_f64, 80.0, 30.0, 50.0, 70.0);
        let pw = w - ml - mr;
        let ph_h = h - mt - mb;
        let px = |ph: f64| ml + ((ph - page.min_ph) / (page.max_ph - page.min_ph)) * pw;
        let py = |eh: f64| mt + ph_h - ((eh - page.min_eh) / (page.max_eh - page.min_eh)) * ph_h;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" style=\"background:#fff;font-family:sans-serif\">\n\
             <text x=\"{:.1}\" y=\"28\" text-anchor=\"middle\" font-size=\"16\" font-weight=\"bold\">{}</text>\n\
             <rect x=\"{ml}\" y=\"{mt}\" width=\"{pw}\" height=\"{ph_h}\" fill=\"none\" stroke=\"#000\" stroke-width=\"1.5\"/>\n",
            w * 0.5,
            xml_escape(&page.result.title)
        );
        for line in [&page.result.water_upper_line, &page.result.water_lower_line] {
            svg.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#0000dc\" stroke-width=\"1.4\" stroke-dasharray=\"5,4\"/>\n",
                px(line[0].0), py(line[0].1), px(line[1].0), py(line[1].1)
            ));
        }
        for s in &page.result.segments {
            svg.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#000\" stroke-width=\"1.2\"/>\n",
                px(s.x1_ph), py(s.y1_v), px(s.x2_ph), py(s.y2_v)
            ));
        }
        for s in &page.result.ion_segments {
            svg.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#0046d2\" stroke-width=\"1.2\" stroke-dasharray=\"2,3\"/>\n",
                px(s.x1_ph), py(s.y1_v), px(s.x2_ph), py(s.y2_v)
            ));
        }
        for reg in &page.result.regions {
            let ov = page.label_overrides.get(&reg.species);
            let txt = ov
                .and_then(|o| o.custom_text.as_deref())
                .unwrap_or(&reg.species);
            if txt.trim().is_empty() {
                continue;
            }
            let (lph, leh) = ov
                .and_then(|o| o.pos_ph_eh)
                .unwrap_or((reg.label_ph, reg.label_eh_v));
            svg.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"12\">{}</text>\n",
                px(lph), py(leh), xml_escape(txt)
            ));
        }
        for reg in &page.result.ion_regions {
            svg.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"#0046d2\">{}</text>\n",
                px(reg.label_ph), py(reg.label_eh_v) + 12.0, xml_escape(&reg.species)
            ));
        }
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\">pH</text>\n\
             <text transform=\"rotate(-90 22 {:.1})\" x=\"22\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\">{}</text>\n</svg>\n",
            ml + pw * 0.5,
            h - 22.0,
            mt + ph_h * 0.5,
            mt + ph_h * 0.5,
            xml_escape(page.potential_scale.label())
        ));
        match crate::storage::write_atomic(path, svg) {
            Ok(_) => {
                self.status_error = None;
                self.status_info = Some(format!("Exported SVG to {path}"));
            }
            Err(e) => self.status_error = Some(format!("SVG export failed: {e}")),
        }
    }

    fn export_active_csv(&mut self, path: &str) {
        let Some(page) = self.active_chart_page() else {
            self.status_error = Some("No chart table to export.".to_string());
            return;
        };
        let mut out = String::new();
        out.push_str(&format!("Title,{}\nMain Element,{}\nTemperature C,{:.3}\nPotential Scale,{}\n",
            page.result.title, page.main_element, page.result.t_k - 273.15, page.potential_scale.label()));
        out.push_str("Element,Molality,Pressure_bar\n");
        for el in &page.elements {
            out.push_str(&format!("{},{:.4E},{:.4E}\n", el.symbol, el.molality, el.pressure_bar));
        }
        out.push_str("\nSpecies,DeltaG_kcal_mol,Area_no,X1_pH,Y1_V,X2_pH,Y2_V\n");
        for row in &page.result.table_rows {
            let area = row.area_no.map(|a| a.to_string()).unwrap_or_default();
            if row.segments.is_empty() {
                out.push_str(&format!("{},{:.3},{area},,,,\n", row.formula, row.delta_g_kcal_mol));
            } else {
                for &(x1, y1, x2, y2) in &row.segments {
                    out.push_str(&format!("{},{:.3},{area},{x1:.4},{y1:.4},{x2:.4},{y2:.4}\n", row.formula, row.delta_g_kcal_mol));
                }
            }
        }
        match crate::storage::write_atomic(path, out) {
            Ok(_) => {
                self.status_error = None;
                self.status_info = Some(format!("Exported Table CSV to {path}"));
            }
            Err(e) => self.status_error = Some(format!("CSV export failed: {e}")),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &SpeciesDb) {
        let mut light_vis = egui::Visuals::light();
        light_vis.panel_fill = egui::Color32::from_rgb(245, 246, 248);
        light_vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(light_vis);

        // Top menu / ribbon bar
        egui::TopBottomPanel::top("ehph_top_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(238, 241, 246))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("New / Default (Cu-S-H2O)").clicked() {
                            self.load_preset(db, vec![("Cu", 1.0, 1.0), ("S", 1.0, 1.0)], "Cu", 25.0, false);
                            ui.close_menu();
                        }
                        if ui.button("Open (.eph8)...").clicked() {
                            self.file_modal_is_save = false;
                            self.show_file_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Save (.eph8)...").clicked() {
                            self.file_modal_is_save = true;
                            self.show_file_modal = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Example: CUS25.eph8 (Cu-S-H2O, 25 °C)").clicked() {
                            self.load_preset(db, vec![("Cu", 1.0, 1.0), ("S", 1.0, 1.0)], "Cu", 25.0, false);
                            ui.close_menu();
                        }
                        if ui.button("Example: Cu-S-H2O + Ions Overlay (25 °C)").clicked() {
                            self.load_preset(db, vec![("Cu", 1.0, 1.0), ("S", 1.0, 1.0)], "Cu", 25.0, true);
                            ui.close_menu();
                        }
                        if ui.button("Example: Fe-H2O (m=1e-6, 25 °C)").clicked() {
                            self.load_preset(db, vec![("Fe", 1e-6, 1.0)], "Fe", 25.0, false);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export Active Chart SVG...").clicked() {
                            self.export_active_svg("/home/nando/.gemini/antigravity/scratch/ehph_chart.svg");
                            ui.close_menu();
                        }
                        if ui.button("Export Active Table CSV...").clicked() {
                            self.export_active_csv("/home/nando/.gemini/antigravity/scratch/ehph_table.csv");
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.019"), |ui| {
                        if ui.button("H2O Stability Lines...").clicked() {
                            self.show_h2o_format = true;
                            ui.close_menu();
                        }
                        if ui.button("Labels Font & Bold...").clicked() {
                            self.show_font_dialog = true;
                            ui.close_menu();
                        }
                        if ui.button("Reset Label Positions & Text").clicked() {
                            let idx = match self.active_tab {
                                RightTab::Chart(i) => i,
                                RightTab::Table => self.last_chart_idx,
                            };
                            if let Some(p) = self.pages.get_mut(idx) {
                                p.label_overrides.clear();
                            }
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button("Eh-pH Diagrams Guide (Ch. 17)").clicked() {
                            self.show_help_modal = true;
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    let switch_label = match self.view_mode {
                        EpHViewMode::Diagram => "Switch View (EpH <-> Combiner)",
                        EpHViewMode::Combiner => "Switch View (Combiner <-> EpH)",
                    };
                    if ui.button(switch_label).clicked() {
                        self.view_mode = match self.view_mode {
                            EpHViewMode::Diagram => {
                                if self.combiner_datasets.iter().all(|d| d.result.is_none()) {
                                    self.draw_combiner(db);
                                }
                                EpHViewMode::Combiner
                            }
                            EpHViewMode::Combiner => EpHViewMode::Diagram,
                        };
                    }
                    if self.view_mode == EpHViewMode::Combiner {
                        ui.separator();
                        if ui.button("Add new Dataset").clicked() {
                            let next_id = self.combiner_datasets.iter().map(|d| d.id).max().unwrap_or(0) + 1;
                            let last_t = self.combiner_datasets.last().map(|d| d.temp_c + 25.0).unwrap_or(25.0);
                            self.combiner_datasets.push(CombinerDataset::new(
                                next_id,
                                self.main_element.clone(),
                                last_t,
                                self.selected_elements.clone(),
                            ));
                        }
                        if ui.button("Draw Combiner").clicked() {
                            self.draw_combiner(db);
                        }
                        if ui
                            .selectable_label(self.combiner_show_table, "Combiner Table")
                            .clicked()
                        {
                            self.combiner_show_table = !self.combiner_show_table;
                        }
                        ui.separator();
                        ui.label("Line Text:");
                        for (val, lbl) in [
                            (CombinerLineText::MainElement, "Main Element"),
                            (CombinerLineText::Temperature, "Temperature"),
                            (CombinerLineText::Molality, "Molality"),
                            (CombinerLineText::Pressure, "Pressure"),
                            (CombinerLineText::DatasetNumber, "Dataset No"),
                            (CombinerLineText::None, "None"),
                        ] {
                            if ui.radio(self.combiner_line_text == val, lbl).clicked() {
                                self.combiner_line_text = val;
                            }
                        }
                    } else {
                        ui.separator();
                        if ui.button("Print Preview...").clicked() {
                            self.show_print_dialog = true;
                        }
                        if let Some(info) = &self.status_info {
                            ui.colored_label(egui::Color32::from_rgb(21, 128, 61), info);
                        }
                        if let Some(err) = &self.status_error {
                            ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                        }
                    }
                });
            });

        match self.view_mode {
            EpHViewMode::Diagram => self.ui_diagram_view(ctx, db),
            EpHViewMode::Combiner => self.ui_combiner_view(ctx, db),
        }

        if self.show_print_dialog {
            self.ui_print_dialog(ctx);
        }
        self.ui_extra_modals(ctx, db);
    }

    fn ui_diagram_view(&mut self, ctx: &egui::Context, db: &SpeciesDb) {
        egui::SidePanel::left("ehph_left_params")
            .resizable(true)
            .default_width(325.0)
            .min_width(180.0)
            .max_width(600.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 245))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.strong("Diagram Parameters");
                    ui.add_space(4.0);

                    // Top split: Select Elements | Select Species
                    let mut elems_changed = false;
                    ui.columns(2, |cols| {
                        cols[0].label(egui::RichText::new("Select Elements").size(11.5).strong());
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(3.0))
                            .show(&mut cols[0], |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("ehph_elem_grid")
                                    .max_height(265.0)
                                    .show(ui, |ui| {
                                        egui::Grid::new("ehph_periodic_5col")
                                            .spacing(egui::vec2(2.0, 2.0))
                                            .show(ui, |ui| {
                                                for (idx, &sym) in ELEMENT_GRID.iter().enumerate() {
                                                    if sym.is_empty() {
                                                        ui.label("");
                                                    } else {
                                                        let active = self
                                                            .selected_elements
                                                            .iter()
                                                            .any(|e| e.symbol == sym);
                                                        let btn = egui::Button::new(
                                                            egui::RichText::new(sym).size(10.5),
                                                        )
                                                        .min_size(egui::vec2(25.0, 17.0))
                                                        .fill(if active {
                                                            egui::Color32::from_rgb(203, 213, 225)
                                                        } else {
                                                            egui::Color32::WHITE
                                                        });
                                                        if ui.add(btn).clicked() {
                                                            if active {
                                                                self.selected_elements
                                                                    .retain(|e| e.symbol != sym);
                                                            } else {
                                                                self.selected_elements.push(
                                                                    ElementSpec {
                                                                        symbol: sym.to_string(),
                                                                        molality: 1.0,
                                                                        pressure_bar: 1.0,
                                                                    },
                                                                );
                                                            }
                                                            if !self
                                                                .selected_elements
                                                                .iter()
                                                                .any(|e| e.symbol == self.main_element)
                                                            {
                                                                self.main_element = self
                                                                    .selected_elements
                                                                    .first()
                                                                    .map(|e| e.symbol.clone())
                                                                    .unwrap_or_default();
                                                            }
                                                            elems_changed = true;
                                                        }
                                                    }
                                                    if (idx + 1) % 5 == 0 {
                                                        ui.end_row();
                                                    }
                                                }
                                            });
                                    });
                            });

                        cols[1].label(egui::RichText::new(crate::i18n::text("common.083")).size(11.5).strong());
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(3.0))
                            .show(&mut cols[1], |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("ehph_species_list")
                                    .max_height(265.0)
                                    .show(ui, |ui| {
                                        for sp in &self.candidate_species {
                                            let is_sel = self.selected_formulas.contains(&sp.formula);
                                            let resp = ui
                                                .selectable_label(is_sel, &sp.formula)
                                                .on_hover_text(format!(
                                                    "{} ({:?}) — click to toggle, double-click for thermodynamic details",
                                                    sp.name, sp.phase
                                                ));
                                            if resp.double_clicked() {
                                                self.species_modal = Some(sp.clone());
                                            } else if resp.clicked() {
                                                if is_sel {
                                                    self.selected_formulas.remove(&sp.formula);
                                                } else {
                                                    self.selected_formulas
                                                        .insert(sp.formula.clone());
                                                }
                                            }
                                        }
                                    });
                            });
                    });

                    if elems_changed {
                        self.refresh_candidate_species(db, true);
                    }

                    ui.add_space(4.0);
                    // Search Mode + Select All
                    let mut mode_changed = false;
                    ui.columns(2, |cols| {
                        cols[0].label(egui::RichText::new("Search Mode").size(11.5).strong());
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(4.0))
                            .show(&mut cols[0], |ui| {
                                mode_changed |= ui.checkbox(&mut self.mode_gases, "Gases").changed();
                                mode_changed |=
                                    ui.checkbox(&mut self.mode_gas_ions, "Gas Ions").changed();
                                mode_changed |=
                                    ui.checkbox(&mut self.mode_liquids, "Liquids").changed();
                                mode_changed |=
                                    ui.checkbox(&mut self.mode_condensed, "Condensed").changed();
                                mode_changed |= ui
                                    .checkbox(&mut self.mode_aq_neutrals, "Aqueous Neutrals")
                                    .changed();
                                mode_changed |=
                                    ui.checkbox(&mut self.mode_aq_ions, "Aqueous Ions").changed();
                            });
                        mode_changed |= cols[0].checkbox(&mut self.mode_organics, "Organics").changed();

                        cols[1].add_space(18.0);
                        if cols[1]
                            .add_sized([120.0, 24.0], egui::Button::new("Select All"))
                            .clicked()
                        {
                            self.selected_formulas = self
                                .candidate_species
                                .iter()
                                .map(|s| s.formula.clone())
                                .collect();
                        }
                        if cols[1]
                            .add_sized([120.0, 22.0], egui::Button::new("Clear All"))
                            .clicked()
                        {
                            self.selected_formulas.clear();
                        }
                        cols[1].label(
                            egui::RichText::new(format!(
                                "{}/{} species",
                                self.selected_formulas.len(),
                                self.candidate_species.len()
                            ))
                            .size(10.5)
                            .color(egui::Color32::from_rgb(90, 95, 105)),
                        );
                    });
                    if mode_changed {
                        self.refresh_candidate_species(db, true);
                    }

                    ui.add_space(4.0);
                    // Parameters and Axes collapsible (Fig. 5)
                    let hdr_txt = if self.show_params_axes {
                        "Parameters and Axes ▲"
                    } else {
                        "Parameters and Axes ▼"
                    };
                    if ui
                        .add_sized([ui.available_width(), 22.0], egui::Button::new(hdr_txt))
                        .clicked()
                    {
                        self.show_params_axes = !self.show_params_axes;
                    }
                    if self.show_params_axes {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(235, 237, 240))
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                egui::Grid::new("ehph_params_grid")
                                    .num_columns(3)
                                    .spacing(egui::vec2(6.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.label("Dielectric Constant");
                                        ui.add_sized(
                                            [76.0, 20.0],
                                            egui::TextEdit::singleline(&mut self.dielectric_str),
                                        );
                                        ui.label("");
                                        ui.end_row();

                                        ui.label("ΔG of H2O");
                                        ui.add_sized(
                                            [76.0, 20.0],
                                            egui::TextEdit::singleline(&mut self.delta_g_h2o_str),
                                        );
                                        ui.label("kcal/mol");
                                        ui.end_row();
                                    });

                                if ui
                                    .checkbox(
                                        &mut self.use_davies,
                                        "Use Davies Method for Activity",
                                    )
                                    .changed()
                                    && self.use_davies
                                    && self
                                        .ionic_strength_str
                                        .trim()
                                        .parse::<f64>()
                                        .unwrap_or(0.0)
                                        <= 0.0
                                {
                                    let est_i = self
                                        .selected_elements
                                        .iter()
                                        .map(|e| e.molality)
                                        .sum::<f64>()
                                        .clamp(0.001, 2.0);
                                    self.ionic_strength_str = format!("{est_i:.3}");
                                }
                                ui.add_enabled_ui(self.use_davies, |ui| {
                                    egui::Grid::new("ehph_davies_grid")
                                        .num_columns(3)
                                        .spacing(egui::vec2(6.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.label("Ion Strength");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(
                                                    &mut self.ionic_strength_str,
                                                ),
                                            );
                                            ui.horizontal(|ui| {
                                                ui.label("mol/kg H2O");
                                                if ui
                                                    .small_button("Auto")
                                                    .on_hover_text("Estimate ionic strength from element molalities (§17.3.4)")
                                                    .clicked()
                                                {
                                                    let est_i = self
                                                        .selected_elements
                                                        .iter()
                                                        .map(|e| e.molality)
                                                        .sum::<f64>()
                                                        .clamp(0.001, 2.0);
                                                    self.ionic_strength_str = format!("{est_i:.3}");
                                                }
                                            });
                                            ui.end_row();

                                            ui.label("Correction Factor");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(
                                                    &mut self.correction_factor_str,
                                                ),
                                            );
                                            ui.label("");
                                            ui.end_row();
                                        });
                                });

                                if ui.checkbox(&mut self.criss_cobble, "Criss-Cobble").changed() {
                                    self.update_temp_derived(db);
                                }

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Diagram Axes").strong().size(11.0));
                                    egui::Grid::new("ehph_axes_grid")
                                        .num_columns(3)
                                        .spacing(egui::vec2(8.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.label("Min Eh");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(&mut self.min_eh_str),
                                            );
                                            ui.label("V");
                                            ui.end_row();

                                            ui.label("Max Eh");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(&mut self.max_eh_str),
                                            );
                                            ui.label("V");
                                            ui.end_row();

                                            ui.label("Min pH");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(&mut self.min_ph_str),
                                            );
                                            ui.label("");
                                            ui.end_row();

                                            ui.label("Max pH");
                                            ui.add_sized(
                                                [70.0, 20.0],
                                                egui::TextEdit::singleline(&mut self.max_ph_str),
                                            );
                                            ui.label("");
                                            ui.end_row();
                                        });
                                });

                                ui.group(|ui| {
                                    ui.label(
                                        egui::RichText::new("Potential Scale").strong().size(11.0),
                                    );
                                    ui.horizontal(|ui| {
                                        ui.radio_value(
                                            &mut self.potential_scale,
                                            PotentialScale::Hydrogen,
                                            "Hydrogen",
                                        );
                                        ui.radio_value(
                                            &mut self.potential_scale,
                                            PotentialScale::Calomel,
                                            "Calomel",
                                        );
                                        ui.radio_value(
                                            &mut self.potential_scale,
                                            PotentialScale::AgAgCl,
                                            "Ag / AgCl",
                                        );
                                    });
                                });
                            });
                    }

                    ui.add_space(6.0);
                    // Temperature row
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("common.007"));
                        let resp = ui.add_sized(
                            [80.0, 22.0],
                            egui::TextEdit::singleline(&mut self.temp_str),
                        );
                        if resp.changed() {
                            if let Ok(v) = self.temp_str.trim().parse::<f64>() {
                                self.temp_value = v;
                                self.update_temp_derived(db);
                            }
                        }
                        let unit_lbl = if self.temp_in_kelvin { "K" } else { "°C" };
                        if ui.add_sized([50.0, 22.0], egui::Button::new(unit_lbl)).clicked() {
                            if self.temp_in_kelvin {
                                self.temp_value -= 273.15;
                            } else {
                                self.temp_value += 273.15;
                            }
                            self.temp_in_kelvin = !self.temp_in_kelvin;
                            self.temp_str = format!("{:.2}", self.temp_value);
                        }
                    });

                    ui.add_space(4.0);
                    // Element Molality & Pressure table (Fig. 1)
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                header_cell(ui, "Element", 62.0);
                                header_cell(ui, "Molality\nmol/kg H2O", 112.0);
                                header_cell(ui, "Pressure\nbar", 104.0);
                            });
                            for el in &mut self.selected_elements {
                                ui.horizontal(|ui| {
                                    text_cell(
                                        ui,
                                        &el.symbol,
                                        62.0,
                                        egui::Color32::WHITE,
                                        egui::Color32::BLACK,
                                        false,
                                        false,
                                        false,
                                    );
                                    let m_buf = self
                                        .elem_mol_bufs
                                        .entry(el.symbol.clone())
                                        .or_insert_with(|| format!("{:.2E}", el.molality));
                                    widget_cell(ui, 112.0, egui::Color32::WHITE, false, |ui| {
                                        let r = ui.text_edit_singleline(m_buf);
                                        if r.changed() || r.lost_focus() {
                                            if let Ok(v) = m_buf.trim().parse::<f64>() {
                                                if v > 0.0 {
                                                    el.molality = v;
                                                }
                                            }
                                        }
                                    });
                                    let p_buf = self
                                        .elem_pres_bufs
                                        .entry(el.symbol.clone())
                                        .or_insert_with(|| format!("{:.2E}", el.pressure_bar));
                                    widget_cell(ui, 104.0, egui::Color32::WHITE, false, |ui| {
                                        let r = ui.text_edit_singleline(p_buf);
                                        if r.changed() || r.lost_focus() {
                                            if let Ok(v) = p_buf.trim().parse::<f64>() {
                                                if v > 0.0 {
                                                    el.pressure_bar = v;
                                                }
                                            }
                                        }
                                    });
                                });
                            }
                        });

                    ui.add_space(6.0);
                    ui.checkbox(
                        &mut self.show_predominance_ions,
                        "Show predominance areas of ions",
                    );
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.create_new_page, "Create new page");
                        if ui.checkbox(&mut self.show_colors, "Show colors").changed() {
                            if let RightTab::Chart(idx) = self.active_tab {
                                if let Some(p) = self.pages.get_mut(idx) {
                                    p.show_colors = self.show_colors;
                                }
                            }
                        }
                    });

                    ui.add_space(6.0);
                    // Bottom action buttons: Table | Combiner | Draw [Main Element v]
                    ui.horizontal(|ui| {
                        if ui.add_sized([68.0, 26.0], egui::Button::new("Table")).clicked() {
                            if let Some(res) = self.pages.get(self.last_chart_idx).map(|p| p.result.clone()) {
                                self.sync_table_dg_bufs_from_result(&res);
                            }
                            self.active_tab = RightTab::Table;
                        }
                        if ui
                            .add_sized([74.0, 26.0], egui::Button::new("Combiner"))
                            .clicked()
                        {
                            self.draw_combiner(db);
                            self.view_mode = EpHViewMode::Combiner;
                        }
                        if ui.add_sized([54.0, 26.0], egui::Button::new("Draw")).clicked() {
                            self.draw_current(db);
                        }
                        let prev_main = self.main_element.clone();
                        egui::ComboBox::from_id_salt("ehph_main_elem_combo")
                            .width(58.0)
                            .selected_text(&self.main_element)
                            .show_ui(ui, |ui| {
                                for el in &self.selected_elements {
                                    ui.selectable_value(
                                        &mut self.main_element,
                                        el.symbol.clone(),
                                        &el.symbol,
                                    );
                                }
                            });
                        if self.main_element != prev_main {
                            self.draw_current(db);
                        }
                    });
                });
            });

        // Right Central Panel: Tabs (Chart 1..N, Table)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                let page_names: Vec<String> = self.pages.iter().map(|p| p.name.clone()).collect();
                let name_refs: Vec<&str> = page_names.iter().map(|s| s.as_str()).collect();
                let active_chart = match self.active_tab {
                    RightTab::Chart(i) => Some(i),
                    RightTab::Table => None,
                };
                match render_chart_tabs(
                    ui,
                    &name_refs,
                    active_chart,
                    self.active_tab == RightTab::Table,
                ) {
                    ChartTabAction::SelectChart(i) => {
                        self.active_tab = RightTab::Chart(i);
                        self.last_chart_idx = i;
                        if let Some(res) = self.pages.get(i).map(|p| p.result.clone()) {
                            self.sync_table_dg_bufs_from_result(&res);
                        }
                    }
                    ChartTabAction::CloseChart(i) if self.pages.len() > 1 => {
                        self.pages.remove(i);
                        let new_idx = i.min(self.pages.len() - 1);
                        self.last_chart_idx = new_idx;
                        if matches!(self.active_tab, RightTab::Chart(_)) {
                            self.active_tab = RightTab::Chart(new_idx);
                        }
                    }
                    ChartTabAction::SelectTable => {
                        if let RightTab::Chart(i) = self.active_tab {
                            self.last_chart_idx = i;
                        }
                        if let Some(res) = self.pages.get(self.last_chart_idx).map(|p| p.result.clone()) {
                            self.sync_table_dg_bufs_from_result(&res);
                        }
                        self.active_tab = RightTab::Table;
                    }
                    _ => {}
                }
                ui.separator();

                match self.active_tab {
                    RightTab::Chart(idx) => {
                        if idx < self.pages.len() {
                            self.last_chart_idx = idx;
                            egui::ScrollArea::both()
                                .id_salt("ehph_chart_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    self.ui_chart_page(ui, idx);
                                });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Select elements and click Draw to generate an Eh-pH diagram.");
                            });
                        }
                    }
                    RightTab::Table => {
                        if !self.pages.is_empty() {
                            let target_idx = self.last_chart_idx.min(self.pages.len() - 1);
                            self.ui_table_sheet(ui, db, target_idx);
                        } else {
                            ui.label("No computed diagram yet. Click Draw first.");
                        }
                    }
                }
            });
    }

    fn ui_chart_page(&mut self, ui: &mut egui::Ui, chart_idx: usize) {
        let Some(page) = self.pages.get(chart_idx).cloned() else {
            return;
        };
        if !page.result.warnings.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for w in &page.result.warnings {
                    ui.colored_label(egui::Color32::from_rgb(180, 83, 9), format!("⚠ {w}"));
                }
            });
        }

        // Title + subtle subtitle
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(&page.result.title)
                    .size(14.5)
                    .strong()
                    .color(egui::Color32::BLACK),
            );
            let ion_note = if page.show_predominance_ions {
                "  |  Blue dotted: Aqueous Ions predominance"
            } else {
                ""
            };
            ui.label(
                egui::RichText::new(format!(
                    "Main Element: {}  |  Scale: {}{ion_note}  (Drag labels to move • Double-click label or axes to format)",
                    page.main_element,
                    page.potential_scale.label()
                ))
                .size(10.0)
                .color(egui::Color32::from_rgb(100, 110, 125)),
            );
        });

        let avail = ui.available_size();
        let legend_h = 26.0 + (page.elements.len() as f32) * 19.0;
        let plot_h = (avail.y - legend_h - 16.0).max(180.0);
        let plot_w = (avail.x - 16.0).max(240.0);

        let (outer_rect, resp) =
            ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::click_and_drag());
        let painter = ui.painter_at(outer_rect);

        let margin_l = 64.0;
        let margin_r = 18.0;
        let margin_t = 10.0;
        let margin_b = 36.0;
        let plot_rect = egui::Rect::from_min_max(
            outer_rect.min + egui::vec2(margin_l, margin_t),
            outer_rect.max - egui::vec2(margin_r, margin_b),
        );

        let ph_to_x = |ph: f64| -> f32 {
            let t = ((ph - page.min_ph) / (page.max_ph - page.min_ph)) as f32;
            plot_rect.left() + t * plot_rect.width()
        };
        let eh_to_y = |eh: f64| -> f32 {
            let t = ((eh - page.min_eh) / (page.max_eh - page.min_eh)) as f32;
            plot_rect.bottom() - t * plot_rect.height()
        };
        let pos_to_ph_eh = |p: egui::Pos2| -> (f64, f64) {
            let fx = ((p.x - plot_rect.left()) / plot_rect.width().max(1.0)) as f64;
            let fy = ((plot_rect.bottom() - p.y) / plot_rect.height().max(1.0)) as f64;
            (
                (page.min_ph + fx * (page.max_ph - page.min_ph)).clamp(page.min_ph, page.max_ph),
                (page.min_eh + fy * (page.max_eh - page.min_eh)).clamp(page.min_eh, page.max_eh),
            )
        };

        // Background
        painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

        // Optional phase-based fills (HSC6 style: green = condensed, pink = aqueous)
        if page.show_colors && page.result.grid_n > 1 {
            let gn = page.result.grid_n;
            let cw = plot_rect.width() / gn as f32;
            let ch = plot_rect.height() / gn as f32;
            for iy in 0..gn {
                for ix in 0..gn {
                    if let Some(reg_idx) = page.result.grid_region_idx[iy * gn + ix] {
                        let col = page
                            .result
                            .regions
                            .get(reg_idx)
                            .map(|r| phase_region_color(r.phase))
                            .unwrap_or_else(|| pastel_color(reg_idx));
                        let x = plot_rect.left() + ix as f32 * cw;
                        let y = plot_rect.bottom() - (iy + 1) as f32 * ch;
                        painter.rect_filled(
                            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cw + 0.6, ch + 0.6)),
                            0.0,
                            col,
                        );
                    }
                }
            }
        }

        // Gridlines & adaptive ticks (standard 2 / 0.2 on default range, nice_step otherwise)
        draw_ehph_grid_and_ticks(&painter, plot_rect, page.min_ph, page.max_ph, page.min_eh, page.max_eh);

        // Clipped painter for lines inside plot box
        let clipped = painter.with_clip_rect(plot_rect);

        // Water stability lines (user-configurable via Format -> H2O Stability Lines)
        let water_col = match self.h2o_format.custom_color {
            Some([r, g, b]) => egui::Color32::from_rgb(r, g, b),
            None if page.show_colors => egui::Color32::from_rgb(0, 0, 220),
            None => egui::Color32::from_rgb(35, 35, 35),
        };
        let water_stroke = egui::Stroke::new(self.h2o_format.width, water_col);
        for line in [&page.result.water_upper_line, &page.result.water_lower_line] {
            let p1 = egui::pos2(ph_to_x(line[0].0), eh_to_y(line[0].1));
            let p2 = egui::pos2(ph_to_x(line[1].0), eh_to_y(line[1].1));
            match self.h2o_format.style {
                H2oLineStyle::Solid => {
                    clipped.line_segment([p1, p2], water_stroke);
                }
                H2oLineStyle::Dashed => draw_dashed_line(&clipped, p1, p2, 5.0, 3.5, water_stroke),
                H2oLineStyle::Dotted => draw_dashed_line(&clipped, p1, p2, 1.8, 2.8, water_stroke),
            }
        }

        // Solid black boundary segments
        let boundary_stroke = egui::Stroke::new(1.15, egui::Color32::BLACK);
        for seg in &page.result.segments {
            clipped.line_segment(
                [
                    egui::pos2(ph_to_x(seg.x1_ph), eh_to_y(seg.y1_v)),
                    egui::pos2(ph_to_x(seg.x2_ph), eh_to_y(seg.y2_v)),
                ],
                boundary_stroke,
            );
        }

        // Optional blue dotted aqueous-ions predominance overlay (§17.3.7)
        if !page.result.ion_segments.is_empty() {
            let ion_col = egui::Color32::from_rgb(0, 70, 210);
            let ion_stroke = egui::Stroke::new(1.25, ion_col);
            for seg in &page.result.ion_segments {
                draw_dashed_line(
                    &clipped,
                    egui::pos2(ph_to_x(seg.x1_ph), eh_to_y(seg.y1_v)),
                    egui::pos2(ph_to_x(seg.x2_ph), eh_to_y(seg.y2_v)),
                    2.0,
                    2.8,
                    ion_stroke,
                );
            }
            let full_labels: BTreeSet<&str> = page
                .result
                .regions
                .iter()
                .map(|r| r.species.as_str())
                .collect();
            for reg in &page.result.ion_regions {
                let mut pos = egui::pos2(ph_to_x(reg.label_ph), eh_to_y(reg.label_eh_v) + 12.0);
                let est_half_w = (reg.species.len() as f32 * 3.5).max(16.0);
                if plot_rect.width() > 2.0 * est_half_w + 12.0 {
                    pos.x = pos.x.clamp(
                        plot_rect.left() + est_half_w + 4.0,
                        plot_rect.right() - est_half_w - 4.0,
                    );
                }
                if plot_rect.height() > 28.0 {
                    pos.y = pos
                        .y
                        .clamp(plot_rect.top() + 10.0, plot_rect.bottom() - 10.0);
                }
                if !full_labels.contains(reg.species.as_str()) {
                    clipped.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        &reg.species,
                        egui::FontId::proportional((page.label_font_size - 1.5).max(9.0)),
                        ion_col,
                    );
                }
            }
        }

        // Collect screen positions of region labels for drag / double-click hit-testing
        let mut label_hits: Vec<(String, egui::Pos2, String, f32, Option<[u8; 3]>)> = Vec::new();
        for reg in &page.result.regions {
            let ov = page.label_overrides.get(&reg.species);
            let txt = ov
                .and_then(|o| o.custom_text.clone())
                .unwrap_or_else(|| reg.species.clone());
            let (lph, leh) = ov
                .and_then(|o| o.pos_ph_eh)
                .unwrap_or((reg.label_ph, reg.label_eh_v));
            let fsz = ov.and_then(|o| o.font_size).unwrap_or(page.label_font_size);
            let col = ov.and_then(|o| o.color);
            let mut pos = egui::pos2(ph_to_x(lph), eh_to_y(leh));
            let est_half_w = (txt.len().max(2) as f32 * (fsz * 0.31)).max(16.0);
            if plot_rect.width() > 2.0 * est_half_w + 12.0 {
                pos.x = pos.x.clamp(
                    plot_rect.left() + est_half_w + 4.0,
                    plot_rect.right() - est_half_w - 4.0,
                );
            }
            if plot_rect.height() > 28.0 {
                pos.y = pos
                    .y
                    .clamp(plot_rect.top() + 10.0, plot_rect.bottom() - 10.0);
            }
            if !txt.trim().is_empty() {
                let draw_col = col
                    .map(|[r, g, b]| egui::Color32::from_rgb(r, g, b))
                    .unwrap_or(egui::Color32::BLACK);
                let galley = clipped.layout_no_wrap(
                    txt.clone(),
                    egui::FontId::proportional(fsz),
                    draw_col,
                );
                let gpos = egui::pos2(pos.x - 0.5 * galley.size().x, pos.y - 0.5 * galley.size().y);
                clipped.galley(gpos, galley.clone(), draw_col);
                if page.label_bold {
                    clipped.galley(
                        egui::pos2(gpos.x + 0.45, gpos.y),
                        galley,
                        draw_col,
                    );
                }
            }
            label_hits.push((reg.species.clone(), pos, txt, fsz, col));
        }

        // Interactive label drag, label double-click, and axis click-to-open-Parameters (§17.4.2-3)
        if resp.drag_started() {
            if let Some(p) = resp.interact_pointer_pos() {
                if let Some((sp, _, _, _, _)) = label_hits
                    .iter()
                    .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
                    .filter(|h| h.1.distance(p) <= 28.0)
                {
                    self.dragging_label = Some((chart_idx, sp.clone()));
                }
            }
        }
        if resp.dragged() {
            if let (Some((d_idx, sp)), Some(p)) = (&self.dragging_label, resp.interact_pointer_pos()) {
                if *d_idx == chart_idx {
                    let (nph, neh) = pos_to_ph_eh(p);
                    if let Some(p_mut) = self.pages.get_mut(chart_idx) {
                        p_mut
                            .label_overrides
                            .entry(sp.clone())
                            .or_default()
                            .pos_ph_eh = Some((nph, neh));
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.dragging_label = None;
        }
        if resp.double_clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if plot_rect.contains(p) {
                    if let Some((sp, _, txt, fsz, col)) = label_hits
                        .iter()
                        .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
                        .filter(|h| h.1.distance(p) <= 32.0)
                    {
                        self.label_edit_modal = Some((
                            chart_idx,
                            sp.clone(),
                            txt.clone(),
                            *fsz,
                            *col,
                        ));
                    }
                } else if p.y > plot_rect.bottom() || p.x < plot_rect.left() {
                    self.show_params_axes = true;
                }
            }
        } else if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if !plot_rect.contains(p) && (p.y > plot_rect.bottom() || p.x < plot_rect.left()) {
                    self.show_params_axes = true;
                }
            }
        }

        draw_ehph_plot_frame(&painter, outer_rect, plot_rect, page.potential_scale.label());

        // Hover inspector tooltip
        if let Some(pos) = resp.hover_pos() {
            if plot_rect.contains(pos) && page.result.grid_n > 1 {
                let fx = (pos.x - plot_rect.left()) / plot_rect.width();
                let fy = (plot_rect.bottom() - pos.y) / plot_rect.height();
                let ph_val = page.min_ph + fx as f64 * (page.max_ph - page.min_ph);
                let eh_val = page.min_eh + fy as f64 * (page.max_eh - page.min_eh);
                let gn = page.result.grid_n;
                let ix = ((fx * (gn - 1) as f32).round() as usize).min(gn - 1);
                let iy = ((fy * (gn - 1) as f32).round() as usize).min(gn - 1);
                let sp_name = page.result.grid_region_idx[iy * gn + ix]
                    .and_then(|ri| page.result.regions.get(ri))
                    .map(|r| r.species.as_str())
                    .unwrap_or("—");
                resp.on_hover_text(format!(
                    "pH: {ph_val:.2}\nEh: {eh_val:.3} V\nSpecies: {sp_name}"
                ));
            }
        }

        // Bottom Element / Molality / Pressure legend table (Fig. 3 & Fig. 4)
        ui.vertical_centered(|ui| {
            egui::Frame::none()
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                .show(ui, |ui| {
                    egui::Grid::new("ehph_chart_legend_tbl")
                        .striped(false)
                        .spacing(egui::vec2(14.0, 2.0))
                        .show(ui, |ui| {
                            ui.strong("Element");
                            ui.strong("Molality");
                            ui.strong("Pressure");
                            ui.end_row();
                            for el in &page.elements {
                                ui.label(&el.symbol);
                                ui.label(format!("{:.2E}", el.molality));
                                ui.label(format!("{:.2E}", el.pressure_bar));
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn ui_table_sheet(&mut self, ui: &mut egui::Ui, db: &SpeciesDb, chart_idx: usize) {
        let Some(page) = self.pages.get(chart_idx).cloned() else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("E-pH Diagram Basic Settings ({})", page.name));
            ui.separator();
            if ui.button("Recalculate Diagram with Table ΔG").clicked() {
                for (formula, buf) in &self.table_dg_bufs {
                    if let Ok(v) = buf.trim().parse::<f64>() {
                        if let Some(orig) = page
                            .result
                            .table_rows
                            .iter()
                            .find(|r| &r.formula == formula)
                            .map(|r| r.delta_g_kcal_mol)
                        {
                            if (v - orig).abs() > 1e-4 {
                                self.delta_g_overrides_kcal.insert(formula.clone(), v);
                            }
                        }
                    }
                }
                self.draw_current(db);
            }
            if ui.button("Reset ΔG Overrides").clicked() {
                self.delta_g_overrides_kcal.clear();
                self.draw_current(db);
            }
            if ui.button(crate::i18n::text("common.031")).clicked() {
                self.export_active_csv("/home/nando/.gemini/antigravity/scratch/ehph_table.csv");
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::both().show(ui, |ui| {
            // Column header A..G
            ui.horizontal(|ui| {
                row_num_cell(ui, 0, 32.0);
                for (lbl, w) in [
                    ("A", 150.0),
                    ("B", 95.0),
                    ("C", 95.0),
                    ("D", 95.0),
                    ("E", 125.0),
                    ("F", 110.0),
                    ("G", 85.0),
                ] {
                    header_cell(ui, lbl, w);
                }
            });

            // Row 1: Title
            ui.horizontal(|ui| {
                row_num_cell(ui, 1, 32.0);
                section_cell(ui, &page.result.title, 755.0, false);
            });

            let settings_rows: Vec<(&str, String, &str)> = vec![
                ("Main Element", page.main_element.clone(), ""),
                ("Temperature", format!("{:.3}", page.result.t_k - 273.15), "°C"),
                ("Show Colors", format!("{}", page.show_colors).to_uppercase(), ""),
                ("Ion Overlay", format!("{}", page.show_predominance_ions).to_uppercase(), ""),
                ("Potential Scale", page.potential_scale.label().to_string(), ""),
                ("Dielectric Constant", format!("{:.3}", page.result.dielectric_constant), ""),
                ("DeltaG of H2O", format!("{:.3}", page.result.delta_g_h2o_kcal), "kcal/mol"),
                ("Davies Method", format!("{}", self.use_davies).to_uppercase(), ""),
                ("Ion Strength", self.ionic_strength_str.clone(), "mol/kg"),
                ("Correction Factor", self.correction_factor_str.clone(), ""),
                ("Criss-Cobble", format!("{}", self.criss_cobble).to_uppercase(), ""),
                ("Min Eh", format!("{:.3}", page.min_eh), "V"),
                ("Max Eh", format!("{:.3}", page.max_eh), "V"),
                ("Min pH", format!("{:.3}", page.min_ph), ""),
                ("Max pH", format!("{:.3}", page.max_ph), ""),
            ];

            for (idx, (k, v, u)) in settings_rows.iter().enumerate() {
                let row_num = idx + 2;
                ui.horizontal(|ui| {
                    row_num_cell(ui, row_num, 32.0);
                    if idx == 0 {
                        header_cell(ui, "Elements", 150.0);
                        header_cell(ui, "Molality", 95.0);
                        header_cell(ui, "Pressure", 95.0);
                    } else if let Some(el) = page.elements.get(idx - 1) {
                        text_cell(ui, &el.symbol, 150.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, &format!("{:.2E}", el.molality), 95.0, egui::Color32::WHITE, egui::Color32::BLACK, true, true, false);
                        text_cell(ui, &format!("{:.2E}", el.pressure_bar), 95.0, egui::Color32::WHITE, egui::Color32::BLACK, true, true, false);
                    } else {
                        text_cell(ui, "", 150.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 95.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 95.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                    }
                    text_cell(ui, "", 95.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                    text_cell(ui, k, 125.0, EXCEL_HEADER_BG, egui::Color32::BLACK, false, false, false);
                    text_cell(ui, v, 110.0, egui::Color32::WHITE, egui::Color32::BLACK, true, true, false);
                    text_cell(ui, u, 85.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                });
            }

            let hdr_row = settings_rows.len() + 3;
            ui.horizontal(|ui| {
                row_num_cell(ui, hdr_row, 32.0);
                header_cell(ui, "Species", 150.0);
                header_cell(ui, "DeltaG kcal/mol", 95.0);
                header_cell(ui, "Area no", 95.0);
                header_cell(ui, "X1 pH", 95.0);
                header_cell(ui, "Y1 V", 125.0);
                header_cell(ui, "X2 pH", 110.0);
                header_cell(ui, "Y2 V", 85.0);
            });

            let mut row_i = hdr_row + 1;
            for sp_row in &page.result.table_rows {
                let bg = if row_i % 2 == 0 {
                    EXCEL_ALT_ROW_BG
                } else {
                    egui::Color32::WHITE
                };
                let dg_bg = if self.delta_g_overrides_kcal.contains_key(&sp_row.formula) {
                    egui::Color32::from_rgb(254, 249, 195)
                } else {
                    bg
                };
                let dg_buf = self
                    .table_dg_bufs
                    .entry(sp_row.formula.clone())
                    .or_insert_with(|| format!("{:.3}", sp_row.delta_g_kcal_mol));
                let orig_dg = sp_row.delta_g_kcal_mol;
                let area_s = sp_row.area_no.map(|a| a.to_string()).unwrap_or_default();

                let mut edit_dg_cell = |ui: &mut egui::Ui| {
                    widget_cell(ui, 95.0, dg_bg, false, |ui| {
                        let r = ui.text_edit_singleline(dg_buf);
                        if r.changed() || r.lost_focus() {
                            if let Ok(v) = dg_buf.trim().parse::<f64>() {
                                if (v - orig_dg).abs() > 1e-3 {
                                    self.delta_g_overrides_kcal
                                        .insert(sp_row.formula.clone(), v);
                                }
                            }
                        }
                    });
                };

                if sp_row.segments.is_empty() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, row_i, 32.0);
                        text_cell(ui, &sp_row.formula, 150.0, bg, egui::Color32::BLACK, false, false, false);
                        edit_dg_cell(ui);
                        text_cell(ui, &area_s, 95.0, bg, egui::Color32::BLACK, true, false, false);
                        text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 125.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 110.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 85.0, bg, egui::Color32::BLACK, false, false, false);
                    });
                    row_i += 1;
                } else {
                    for (seg_idx, &(x1, y1, x2, y2)) in sp_row.segments.iter().take(6).enumerate() {
                        ui.horizontal(|ui| {
                            row_num_cell(ui, row_i, 32.0);
                            if seg_idx == 0 {
                                text_cell(ui, &sp_row.formula, 150.0, bg, egui::Color32::BLACK, false, false, false);
                                edit_dg_cell(ui);
                                text_cell(ui, &area_s, 95.0, bg, egui::Color32::BLACK, true, false, false);
                            } else {
                                text_cell(ui, "", 150.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                            }
                            text_cell(ui, &format!("{x1:.3}"), 95.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{y1:.3}"), 125.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{x2:.3}"), 110.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{y2:.3}"), 85.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                        });
                        row_i += 1;
                    }
                }
            }
        });
    }

    fn ui_combiner_view(&mut self, ctx: &egui::Context, db: &SpeciesDb) {
        let cur_elements = self.selected_elements.clone();
        let cur_main = self.main_element.clone();
        let mut redraw_comb = false;

        egui::SidePanel::left("ehph_combiner_left")
            .resizable(true)
            .default_width(295.0)
            .min_width(180.0)
            .max_width(520.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(240, 242, 245))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut remove_idx = None;
                    for (idx, ds) in self.combiner_datasets.iter_mut().enumerate() {
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(format!("Dataset {}", ds.id));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("×").clicked() {
                                                remove_idx = Some(idx);
                                            }
                                            if ui
                                                .small_button("Sync")
                                                .on_hover_text("Sync elements & main element from EpH view")
                                                .clicked()
                                            {
                                                ds.elements = cur_elements.clone();
                                                ds.main_element = cur_main.clone();
                                                ds.mol_bufs.clear();
                                                ds.pres_bufs.clear();
                                                for el in &ds.elements {
                                                    ds.mol_bufs.insert(
                                                        el.symbol.clone(),
                                                        format!("{:.2E}", el.molality),
                                                    );
                                                    ds.pres_bufs.insert(
                                                        el.symbol.clone(),
                                                        format!("{:.2E}", el.pressure_bar),
                                                    );
                                                }
                                                redraw_comb = true;
                                            }
                                        },
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Main Element:");
                                    egui::ComboBox::from_id_salt(format!("comb_main_{}", ds.id))
                                        .width(62.0)
                                        .selected_text(&ds.main_element)
                                        .show_ui(ui, |ui| {
                                            for el in &ds.elements {
                                                ui.selectable_value(
                                                    &mut ds.main_element,
                                                    el.symbol.clone(),
                                                    &el.symbol,
                                                );
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label(crate::i18n::text("common.007"));
                                    let r = ui.add_sized(
                                        [65.0, 20.0],
                                        egui::TextEdit::singleline(&mut ds.temp_str),
                                    );
                                    if r.changed() || r.lost_focus() {
                                        if let Ok(v) = ds.temp_str.trim().parse::<f64>() {
                                            ds.temp_c = v;
                                        }
                                    }
                                    ui.label("°C");
                                });
                                egui::Grid::new(format!("comb_tbl_{}", ds.id))
                                    .striped(true)
                                    .spacing(egui::vec2(6.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.strong("Element");
                                        ui.strong("Molality");
                                        ui.strong("Pressure");
                                        ui.end_row();
                                        for el in &mut ds.elements {
                                            ui.label(&el.symbol);
                                            let m_buf = ds
                                                .mol_bufs
                                                .entry(el.symbol.clone())
                                                .or_insert_with(|| format!("{:.2E}", el.molality));
                                            let rm = ui.add_sized(
                                                [72.0, 18.0],
                                                egui::TextEdit::singleline(m_buf),
                                            );
                                            if rm.changed() || rm.lost_focus() {
                                                if let Ok(v) = m_buf.trim().parse::<f64>() {
                                                    if v > 0.0 {
                                                        el.molality = v;
                                                    }
                                                }
                                            }
                                            let p_buf = ds
                                                .pres_bufs
                                                .entry(el.symbol.clone())
                                                .or_insert_with(|| {
                                                    format!("{:.2E}", el.pressure_bar)
                                                });
                                            let rp = ui.add_sized(
                                                [72.0, 18.0],
                                                egui::TextEdit::singleline(p_buf),
                                            );
                                            if rp.changed() || rp.lost_focus() {
                                                if let Ok(v) = p_buf.trim().parse::<f64>() {
                                                    if v > 0.0 {
                                                        el.pressure_bar = v;
                                                    }
                                                }
                                            }
                                            ui.end_row();
                                        }
                                    });
                                if let Some(err) = &ds.error {
                                    ui.colored_label(egui::Color32::RED, err);
                                }
                            });
                        ui.add_space(6.0);
                    }
                    if let Some(i) = remove_idx {
                        if self.combiner_datasets.len() > 1 {
                            self.combiner_datasets.remove(i);
                            redraw_comb = true;
                        }
                    }
                });
            });

        if redraw_comb {
            self.draw_combiner(db);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                if self.combiner_show_table {
                    ui.strong("Combiner Table");
                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| {
                        for ds in &self.combiner_datasets {
                            ui.strong(format!(
                                "Dataset {} — {} at {:.1} °C",
                                ds.id, ds.main_element, ds.temp_c
                            ));
                            if let Some(res) = &ds.result {
                                ui.label(format!(
                                    "Active regions: {} | Boundary segments: {} | ΔG°(H2O): {:.3} kcal/mol",
                                    res.regions.len(),
                                    res.segments.len(),
                                    res.delta_g_h2o_kcal
                                ));
                                egui::Grid::new(format!("comb_res_tbl_{}", ds.id))
                                    .striped(true)
                                    .spacing(egui::vec2(12.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.strong("Area #");
                                        ui.strong("Species");
                                        ui.strong("Phase");
                                        ui.strong("ΔG° (kcal/mol)");
                                        ui.strong("Label (pH, V)");
                                        ui.end_row();
                                        for reg in &res.regions {
                                            ui.label(reg.area_no.to_string());
                                            ui.label(&reg.species);
                                            ui.label(format!("{:?}", reg.phase));
                                            ui.label(format!("{:.3}", reg.delta_g_kcal_mol));
                                            ui.label(format!(
                                                "({:.2}, {:.3})",
                                                reg.label_ph, reg.label_eh_v
                                            ));
                                            ui.end_row();
                                        }
                                    });
                            } else if let Some(err) = &ds.error {
                                ui.colored_label(egui::Color32::RED, err);
                            }
                            ui.add_space(6.0);
                        }
                    });
                    return;
                }

                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Combiner Chart")
                            .size(14.5)
                            .strong()
                            .color(egui::Color32::BLACK),
                    );
                });

                let avail = ui.available_size();
                let plot_w = (avail.x - 16.0).max(320.0);
                let plot_h = (avail.y - 24.0).max(280.0);
                let (outer_rect, _) =
                    ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::hover());
                let painter = ui.painter_at(outer_rect);

                let margin_l = 64.0;
                let margin_r = 18.0;
                let margin_t = 12.0;
                let margin_b = 34.0;
                let plot_rect = egui::Rect::from_min_max(
                    outer_rect.min + egui::vec2(margin_l, margin_t),
                    outer_rect.max - egui::vec2(margin_r, margin_b),
                );

                let min_ph: f64 = self.min_ph_str.trim().parse().unwrap_or(0.0);
                let max_ph: f64 = self.max_ph_str.trim().parse().unwrap_or(14.0);
                let min_eh: f64 = self.min_eh_str.trim().parse().unwrap_or(-2.0);
                let max_eh: f64 = self.max_eh_str.trim().parse().unwrap_or(2.0);

                let ph_to_x = |ph: f64| -> f32 {
                    plot_rect.left()
                        + (((ph - min_ph) / (max_ph - min_ph)) as f32) * plot_rect.width()
                };
                let eh_to_y = |eh: f64| -> f32 {
                    plot_rect.bottom()
                        - (((eh - min_eh) / (max_eh - min_eh)) as f32) * plot_rect.height()
                };

                painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

                draw_ehph_grid_and_ticks(&painter, plot_rect, min_ph, max_ph, min_eh, max_eh);

                let clipped = painter.with_clip_rect(plot_rect);
                let palette = [
                    egui::Color32::BLACK,
                    egui::Color32::from_rgb(37, 99, 235),
                    egui::Color32::from_rgb(185, 28, 28),
                    egui::Color32::from_rgb(21, 128, 61),
                    egui::Color32::from_rgb(124, 58, 237),
                ];

                for (ds_idx, ds) in self.combiner_datasets.iter().enumerate() {
                    let Some(res) = &ds.result else {
                        continue;
                    };
                    let col = palette[ds_idx % palette.len()];
                    let stroke = egui::Stroke::new(1.15, col);
                    for line in [&res.water_upper_line, &res.water_lower_line] {
                        draw_dashed_line(
                            &clipped,
                            egui::pos2(ph_to_x(line[0].0), eh_to_y(line[0].1)),
                            egui::pos2(ph_to_x(line[1].0), eh_to_y(line[1].1)),
                            4.0,
                            3.0,
                            stroke,
                        );
                    }
                    let mut by_pair: BTreeMap<(&str, &str), Vec<&soryntel_thermo::ehph::BoundarySegment>> =
                        BTreeMap::new();
                    for seg in &res.segments {
                        let p1 = egui::pos2(ph_to_x(seg.x1_ph), eh_to_y(seg.y1_v));
                        let p2 = egui::pos2(ph_to_x(seg.x2_ph), eh_to_y(seg.y2_v));
                        clipped.line_segment([p1, p2], stroke);
                        by_pair
                            .entry((seg.species_a.as_str(), seg.species_b.as_str()))
                            .or_default()
                            .push(seg);
                    }
                    for seg in &res.ion_segments {
                        draw_dashed_line(
                            &clipped,
                            egui::pos2(ph_to_x(seg.x1_ph), eh_to_y(seg.y1_v)),
                            egui::pos2(ph_to_x(seg.x2_ph), eh_to_y(seg.y2_v)),
                            2.0,
                            2.8,
                            stroke,
                        );
                    }
                    if self.combiner_line_text != CombinerLineText::None {
                        let tag = match self.combiner_line_text {
                            CombinerLineText::MainElement => ds.main_element.clone(),
                            CombinerLineText::Temperature => format!("{:.0}", ds.temp_c),
                            CombinerLineText::Molality => ds
                                .elements
                                .iter()
                                .find(|e| e.symbol == ds.main_element)
                                .or_else(|| ds.elements.first())
                                .map(|e| format!("{:.1E}", e.molality))
                                .unwrap_or_default(),
                            CombinerLineText::Pressure => ds
                                .elements
                                .iter()
                                .find(|e| e.symbol == ds.main_element)
                                .or_else(|| ds.elements.first())
                                .map(|e| format!("{:.1E}", e.pressure_bar))
                                .unwrap_or_default(),
                            CombinerLineText::DatasetNumber => format!("DS{}", ds.id),
                            CombinerLineText::None => String::new(),
                        };
                        for segs in by_pair.values() {
                            if let Some(mid_seg) = segs.get(segs.len() / 2) {
                                let mid = egui::pos2(
                                    ph_to_x(0.5 * (mid_seg.x1_ph + mid_seg.x2_ph)),
                                    eh_to_y(0.5 * (mid_seg.y1_v + mid_seg.y2_v)),
                                );
                                clipped.text(
                                    mid,
                                    egui::Align2::CENTER_CENTER,
                                    &tag,
                                    egui::FontId::proportional(10.0),
                                    col,
                                );
                            }
                        }
                    }
                    if ds_idx == 0 {
                        for reg in &res.regions {
                            clipped.text(
                                egui::pos2(ph_to_x(reg.label_ph), eh_to_y(reg.label_eh_v)),
                                egui::Align2::CENTER_CENTER,
                                &reg.species,
                                egui::FontId::proportional(12.0),
                                egui::Color32::BLACK,
                            );
                        }
                    }
                }

                draw_ehph_plot_frame(&painter, outer_rect, plot_rect, self.potential_scale.label());
            });
    }

    fn ui_print_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_print_dialog;
        egui::Window::new("Print Settings/Preview")
            .open(&mut open)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Header:");
                    ui.text_edit_singleline(&mut self.print_header);
                });
                ui.horizontal(|ui| {
                    ui.label("Footer:");
                    ui.text_edit_singleline(&mut self.print_footer);
                });
                ui.horizontal(|ui| {
                    ui.label("Orientation:");
                    ui.radio_value(&mut self.print_landscape, false, "Portrait");
                    ui.radio_value(&mut self.print_landscape, true, "Landscape");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Export SVG...").clicked() {
                        self.export_active_svg("/home/nando/.gemini/antigravity/scratch/ehph_chart.svg");
                    }
                    if ui.button("Export CSV...").clicked() {
                        self.export_active_csv("/home/nando/.gemini/antigravity/scratch/ehph_table.csv");
                    }
                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        self.show_print_dialog = false;
                    }
                });
            });
        if !open {
            self.show_print_dialog = false;
        }
    }

    fn ui_extra_modals(&mut self, ctx: &egui::Context, db: &SpeciesDb) {
        if let Some(sp) = self.species_modal.clone() {
            let mut open = true;
            let tk = self.t_k();
            egui::Window::new(format!("Species Properties — {}", sp.formula))
                .open(&mut open)
                .resizable(true)
                .default_width(480.0)
                .show(ctx, |ui| {
                    egui::Grid::new("ehph_sp_info_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(12.0, 4.0))
                        .show(ui, |ui| {
                            ui.strong("Formula:");
                            ui.label(&sp.formula);
                            ui.end_row();
                            ui.strong("Name:");
                            ui.label(&sp.name);
                            ui.end_row();
                            ui.strong("Phase:");
                            ui.label(format!("{:?}", sp.phase));
                            ui.end_row();
                            ui.strong("Molecular Weight:");
                            ui.label(format!("{:.4} g/mol", sp.molecular_weight));
                            ui.end_row();
                            ui.strong("Density:");
                            ui.label(format!("{:.4} g/cm³", sp.density_g_cm3));
                            ui.end_row();
                            ui.strong("Charge:");
                            ui.label(format!("{:+}", sp.charge));
                            ui.end_row();
                            if sp.formula.contains("(ia)") {
                                ui.strong("Criss-Cobble (ia):");
                                ui.colored_label(
                                    egui::Color32::from_rgb(180, 83, 9),
                                    "Database 25 °C-only ion; Criss-Cobble extrapolates Cp(T) when enabled.",
                                );
                                ui.end_row();
                            }
                            if let Ok((pt, _)) = evaluate_species_si(&sp, tk, self.criss_cobble, false) {
                                ui.strong(format!("H({:.2} K):", tk));
                                ui.label(format!("{:.3} kJ/mol", pt.h / 1000.0));
                                ui.end_row();
                                ui.strong(format!("S({:.2} K):", tk));
                                ui.label(format!("{:.3} J/(mol·K)", pt.s));
                                ui.end_row();
                                ui.strong(format!("G({:.2} K):", tk));
                                ui.label(format!("{:.3} kJ/mol", pt.g / 1000.0));
                                ui.end_row();
                            }
                        });
                    ui.separator();
                    ui.strong(format!("Thermodynamic Intervals ({})", sp.intervals.len()));
                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        egui::Grid::new("ehph_sp_iv_grid")
                            .striped(true)
                            .spacing(egui::vec2(8.0, 2.0))
                            .show(ui, |ui| {
                                ui.strong("T_min..T_max (K)");
                                ui.strong("H_298/tr (kJ)");
                                ui.strong("S_298/tr (J/K)");
                                ui.strong("Reference");
                                ui.end_row();
                                for iv in &sp.intervals {
                                    ui.label(format!("{:.1}..{:.1}", iv.t_min, iv.t_max));
                                    ui.label(format!("{:.2}", iv.h_or_delta_h_tr / 1000.0));
                                    ui.label(format!("{:.2}", iv.s_or_delta_s_tr));
                                    ui.label(&iv.reference);
                                    ui.end_row();
                                }
                            });
                    });
                });
            if !open {
                self.species_modal = None;
            }
        }

        if let Some((c_idx, sp, mut txt, mut fsz, mut col)) = self.label_edit_modal.clone() {
            let mut open = true;
            let mut apply = false;
            let mut reset = false;
            egui::Window::new(format!("Format Label — {sp}"))
                .open(&mut open)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.label("Text (leave empty to hide label):");
                    ui.text_edit_singleline(&mut txt);
                    ui.add(egui::Slider::new(&mut fsz, 8.0..=20.0).text("Font Size"));
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("common.084"));
                        if ui.selectable_label(col.is_none(), "Black").clicked() {
                            col = None;
                        }
                        if ui.selectable_label(col == Some([0, 70, 210]), "Blue").clicked() {
                            col = Some([0, 70, 210]);
                        }
                        if ui.selectable_label(col == Some([185, 28, 28]), "Red").clicked() {
                            col = Some([185, 28, 28]);
                        }
                        if ui.selectable_label(col == Some([21, 128, 61]), "Green").clicked() {
                            col = Some([21, 128, 61]);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::text("common.032")).clicked() {
                            apply = true;
                        }
                        if ui.button("Reset Label").clicked() {
                            reset = true;
                        }
                    });
                });
            if apply {
                if let Some(p) = self.pages.get_mut(c_idx) {
                    let entry = p.label_overrides.entry(sp).or_default();
                    entry.custom_text = Some(txt);
                    entry.font_size = Some(fsz);
                    entry.color = col;
                }
                self.label_edit_modal = None;
            } else if reset {
                if let Some(p) = self.pages.get_mut(c_idx) {
                    p.label_overrides.remove(&sp);
                }
                self.label_edit_modal = None;
            } else if !open {
                self.label_edit_modal = None;
            } else {
                self.label_edit_modal = Some((c_idx, sp, txt, fsz, col));
            }
        }

        if self.show_h2o_format {
            let mut open = self.show_h2o_format;
            egui::Window::new("Format H2O Stability Lines")
                .open(&mut open)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Line Style:");
                        ui.radio_value(&mut self.h2o_format.style, H2oLineStyle::Dashed, "Dashed");
                        ui.radio_value(&mut self.h2o_format.style, H2oLineStyle::Dotted, "Dotted");
                        ui.radio_value(&mut self.h2o_format.style, H2oLineStyle::Solid, "Solid");
                    });
                    ui.add(egui::Slider::new(&mut self.h2o_format.width, 0.8..=3.5).text("Width"));
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::text("common.084"));
                        if ui.selectable_label(self.h2o_format.custom_color.is_none(), "Auto").clicked() {
                            self.h2o_format.custom_color = None;
                        }
                        if ui.selectable_label(self.h2o_format.custom_color == Some([0, 0, 220]), "Blue").clicked() {
                            self.h2o_format.custom_color = Some([0, 0, 220]);
                        }
                        if ui.selectable_label(self.h2o_format.custom_color == Some([40, 40, 40]), "Black").clicked() {
                            self.h2o_format.custom_color = Some([40, 40, 40]);
                        }
                        if ui.selectable_label(self.h2o_format.custom_color == Some([185, 28, 28]), "Red").clicked() {
                            self.h2o_format.custom_color = Some([185, 28, 28]);
                        }
                    });
                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        self.show_h2o_format = false;
                    }
                });
            if !open {
                self.show_h2o_format = false;
            }
        }

        if self.show_font_dialog {
            let mut open = self.show_font_dialog;
            egui::Window::new("Default Label Font")
                .open(&mut open)
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.default_label_font_size, 9.0..=18.0)
                            .text("Font Size"),
                    );
                    ui.checkbox(&mut self.default_label_bold, "Bold region labels");
                    if ui.button("Apply to All Pages").clicked() {
                        for p in &mut self.pages {
                            p.label_font_size = self.default_label_font_size;
                            p.label_bold = self.default_label_bold;
                        }
                    }
                });
            if !open {
                self.show_font_dialog = false;
            }
        }

        if self.show_file_modal {
            let mut open = self.show_file_modal;
            let title = if self.file_modal_is_save {
                "Save Eh-pH Diagram (.eph8)"
            } else {
                "Open Eh-pH Diagram (.eph8)"
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
                    self.save_eph8(&p);
                } else {
                    self.open_eph8(db, &p);
                }
                self.show_file_modal = false;
            } else if !open {
                self.show_file_modal = false;
            }
        }

        if self.show_help_modal {
            let mut open = self.show_help_modal;
            egui::Window::new("Eh-pH Diagrams Help (HSC Ch. 17)")
                .open(&mut open)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.label("• Select Elements + Main Element, adjust Molality & Pressure, and click Draw.");
                    ui.label("• Double-click a species in 'Select Species' to inspect ΔG°(T), intervals & (ia) Criss-Cobble info.");
                    ui.label("• Enable 'Show predominance areas of ions' to draw the aqueous-species-only diagram in blue dotted lines on top of the solid+aqueous diagram.");
                    ui.label("• Drag labels directly on the chart, double-click a label to rename/hide/style it, or click an axis to open Parameters and Axes.");
                    ui.label("• In Table (Basic Settings), edit ΔG° kcal/mol values and click 'Recalculate Diagram with Table ΔG' to test sensitivity.");
                    ui.label("• Use Combiner to superimpose diagrams at multiple T, P, or molalities.");
                });
            if !open {
                self.show_help_modal = false;
            }
        }
    }
}

fn draw_ehph_grid_and_ticks(
    painter: &egui::Painter,
    plot_rect: egui::Rect,
    min_ph: f64,
    max_ph: f64,
    min_eh: f64,
    max_eh: f64,
) {
    let span_ph = (max_ph - min_ph).max(0.2);
    let span_eh = (max_eh - min_eh).max(0.2);
    let step_ph = if (min_ph - 0.0).abs() < 1e-6 && (max_ph - 14.0).abs() < 1e-6 {
        2.0
    } else {
        nice_step(span_ph / 7.0)
    };
    let step_eh = if (min_eh - (-2.0)).abs() < 1e-6 && (max_eh - 2.0).abs() < 1e-6 {
        0.2
    } else {
        nice_step(span_eh / 10.0)
    };
    let ph_to_x = |ph: f64| -> f32 {
        plot_rect.left() + (((ph - min_ph) / span_ph) as f32) * plot_rect.width()
    };
    let eh_to_y = |eh: f64| -> f32 {
        plot_rect.bottom() - (((eh - min_eh) / span_eh) as f32) * plot_rect.height()
    };

    let grid_col = egui::Color32::from_rgb(205, 210, 218);
    let mut ph = (min_ph / step_ph).ceil() * step_ph;
    while ph <= max_ph + 1e-6 {
        let x = ph_to_x(ph);
        draw_dashed_line(
            painter,
            egui::pos2(x, plot_rect.top()),
            egui::pos2(x, plot_rect.bottom()),
            2.0,
            3.0,
            egui::Stroke::new(0.8, grid_col),
        );
        painter.text(
            egui::pos2(x, plot_rect.bottom() + 12.0),
            egui::Align2::CENTER_CENTER,
            format_tick(ph),
            egui::FontId::proportional(11.0),
            egui::Color32::BLACK,
        );
        ph += step_ph;
    }

    let mut eh = (min_eh / step_eh).ceil() * step_eh;
    while eh <= max_eh + 1e-6 {
        let y = eh_to_y(eh);
        draw_dashed_line(
            painter,
            egui::pos2(plot_rect.left(), y),
            egui::pos2(plot_rect.right(), y),
            2.0,
            3.0,
            egui::Stroke::new(0.8, grid_col),
        );
        let lbl = if step_eh < 0.99 {
            if eh.abs() < 1e-9 {
                "0.0".to_string()
            } else {
                format!("{eh:.1}")
            }
        } else {
            format_tick(eh)
        };
        painter.text(
            egui::pos2(plot_rect.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            lbl,
            egui::FontId::proportional(11.0),
            egui::Color32::BLACK,
        );
        eh += step_eh;
    }
}

fn draw_ehph_plot_frame(
    painter: &egui::Painter,
    outer_rect: egui::Rect,
    plot_rect: egui::Rect,
    y_label: &str,
) {
    painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.3, egui::Color32::BLACK));
    painter.text(
        egui::pos2(plot_rect.center().x, plot_rect.bottom() + 26.0),
        egui::Align2::CENTER_CENTER,
        "pH",
        egui::FontId::proportional(12.0),
        egui::Color32::BLACK,
    );
    draw_rotated_y_title(
        painter,
        outer_rect.left() + 12.0,
        plot_rect.center().y,
        y_label,
        egui::FontId::proportional(11.5),
        egui::Color32::BLACK,
    );
}

fn is_ehph_organic(formula: &str) -> bool {
    let Ok(parsed) = soryntel_solver::parse_term(formula) else {
        return false;
    };
    if !(parsed.elements.contains_key("C") && parsed.elements.contains_key("H")) {
        return false;
    }
    let is_inorg = matches!(
        formula,
        "HCO3(-a)" | "H2CO3(a)" | "HCN(a)" | "HCN(g)" | "HOCN(a)" | "HSCN(a)" | "HCO3-"
    );
    !is_inorg
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn phase_region_color(phase: soryntel_thermo::Phase) -> egui::Color32 {
    match phase {
        soryntel_thermo::Phase::Solid | soryntel_thermo::Phase::Liquid => {
            egui::Color32::from_rgb(167, 243, 167)
        }
        soryntel_thermo::Phase::AqueousIon | soryntel_thermo::Phase::AqueousNeutral => {
            egui::Color32::from_rgb(249, 178, 178)
        }
        soryntel_thermo::Phase::Gas => egui::Color32::from_rgb(191, 219, 254),
    }
}

fn pastel_color(idx: usize) -> egui::Color32 {
    const PALETTE: &[(u8, u8, u8)] = &[
        (219, 234, 254),
        (220, 252, 231),
        (254, 249, 195),
        (254, 226, 226),
        (237, 233, 254),
        (207, 250, 254),
        (255, 237, 213),
        (243, 232, 255),
    ];
    let (r, g, b) = PALETTE[idx % PALETTE.len()];
    egui::Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_core::CAL_TO_JOULE;
    use soryntel_thermo::{KelleyCoefficients, SpeciesThermoInterval};

    fn synth_sp(formula: &str, phase: Phase, charge: i32, dg_kcal: f64) -> ChemicalSpecies {
        ChemicalSpecies {
            formula: formula.to_string(),
            name: formula.to_string(),
            phase,
            molecular_weight: 60.0,
            density_g_cm3: 2.0,
            charge,
            intervals: vec![SpeciesThermoInterval {
                t_min: 273.15,
                t_max: 2000.0,
                phase,
                kelley: KelleyCoefficients {
                    a: 0.0,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: dg_kcal * 1000.0 * CAL_TO_JOULE,
                s_or_delta_s_tr: 0.0,
                reference: "test".to_string(),
            }],
        }
    }

    fn make_cu_s_test_db() -> SpeciesDb {
        let mut db = SpeciesDb::with_open_defaults();
        for sp in [
            synth_sp("Cu", Phase::Solid, 0, 0.0),
            synth_sp("Cu2O", Phase::Solid, 0, -35.335),
            synth_sp("Cu(OH)2", Phase::Solid, 0, -89.079),
            synth_sp("Cu(+2a)", Phase::AqueousIon, 2, 15.66),
            synth_sp("CuO2(-2a)", Phase::AqueousIon, -2, -69.83),
            synth_sp("Cu2S", Phase::Solid, 0, -20.6),
            synth_sp("S", Phase::Solid, 0, 0.0),
            synth_sp("HS(-a)", Phase::AqueousIon, -1, 2.88),
            synth_sp("SO4(-2a)", Phase::AqueousIon, -2, -177.95),
        ] {
            db.insert_species(sp);
        }
        db
    }

    #[test]
    fn organic_filter_preserves_inorganic_carbonates_when_organics_off() {
        assert!(is_ehph_organic("CH4(g)"));
        assert!(is_ehph_organic("C2H5OH(a)"));
        assert!(!is_ehph_organic("HCO3(-a)"));
        assert!(!is_ehph_organic("H2CO3(a)"));
        assert!(!is_ehph_organic("CO3(-2a)"));
    }

    #[test]
    fn eph8_save_and_open_roundtrip() {
        let db = make_cu_s_test_db();
        let mut st = EhPhModuleState::new(&db);
        st.load_preset(&db, vec![("Cu", 0.5, 1.0), ("S", 0.1, 1.0)], "Cu", 50.0, true);
        let tmp = format!("/tmp/test_roundtrip_{}.eph8", std::process::id());
        st.save_eph8(&tmp);
        assert!(st.status_error.is_none(), "{:?}", st.status_error);

        let mut st2 = EhPhModuleState::new(&db);
        st2.open_eph8(&db, &tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(st2.status_error.is_none(), "{:?}", st2.status_error);
        assert!(st2.show_predominance_ions);
        assert!((st2.temp_value - 50.0).abs() < 1e-6);
        assert!(
            !st2.pages[0].result.ion_segments.is_empty(),
            "before_ion_regions={:?} after_ion_regions={:?} table={:?}",
            st.pages[0].result.ion_regions,
            st2.pages[0].result.ion_regions,
            st2.pages[0].result.table_rows.iter().map(|r| (&r.formula, r.delta_g_kcal_mol)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn combiner_dataset_syncs_buffers_and_computes() {
        let db = make_cu_s_test_db();
        let mut st = EhPhModuleState::new(&db);
        st.combiner_datasets[0].temp_str = "80.0".to_string();
        st.combiner_datasets[0]
            .mol_bufs
            .insert("S".to_string(), "1.0E-3".to_string());
        st.draw_combiner(&db);
        assert!((st.combiner_datasets[0].temp_c - 80.0).abs() < 1e-6);
        assert!(
            (st.combiner_datasets[0]
                .elements
                .iter()
                .find(|e| e.symbol == "S")
                .unwrap()
                .molality
                - 1e-3)
                .abs()
                < 1e-9
        );
        assert!(
            st.combiner_datasets[0].result.is_some(),
            "{:?}",
            st.combiner_datasets[0].error
        );
    }

    #[test]
    fn maindb6_cus25_and_s_cu_h2o_if_present() {
        let path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let mut db = SpeciesDb::with_open_defaults();
        db.load_main_hsc(path).expect("Maindb6 opens");
        let mut st = EhPhModuleState::new(&db);
        // Check Search Mode Organics filter when C is selected
        st.selected_elements = vec![ElementSpec {
            symbol: "C".to_string(),
            molality: 1.0,
            pressure_bar: 1.0,
        }];
        st.mode_organics = false;
        st.mode_gases = true;
        st.refresh_candidate_species(&db, true);
        assert!(!st.candidate_species.iter().any(|s| s.formula == "CH4(g)"));
        st.mode_organics = true;
        st.refresh_candidate_species(&db, true);
        assert!(st.candidate_species.iter().any(|s| s.formula == "CH4(g)"));

        // Fig. 1-4: Cu - S - H2O with ion overlay at 25 °C
        st.mode_gases = false;
        st.mode_organics = false;
        st.load_preset(&db, vec![("Cu", 1.0, 1.0), ("S", 1.0, 1.0)], "Cu", 25.0, true);
        assert!(st.status_error.is_none());
        let res = &st.pages[0].result;
        assert!(!res.regions.is_empty());
        assert!(!res.ion_regions.is_empty());

        // Main element switched to S (Fig. 6 analogy)
        st.load_preset(&db, vec![("Cu", 1.0, 1.0), ("S", 1.0, 1.0)], "S", 25.0, false);
        assert!(st.status_error.is_none());
        let s_names: Vec<&str> = st.pages[0].result.regions.iter().map(|r| r.species.as_str()).collect();
        assert!(s_names.iter().any(|n| n.contains("SO4")));
    }
}
