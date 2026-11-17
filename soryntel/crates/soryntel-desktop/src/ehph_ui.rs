use std::collections::{BTreeMap, BTreeSet};

use eframe::egui;
use soryntel_core::CAL_TO_JOULE;
use soryntel_db::ThermoDatabase as SpeciesDb;
use soryntel_thermo::{
    ehph::{
        calculate_ehph, delta_g_formation_h2o_kcal, water_dielectric_constant, EhPhRequest,
        EhPhResult, ElementSpec, PotentialScale,
    },
    ChemicalSpecies,
};

use crate::excel_grid::{
    header_cell, row_num_cell, section_cell, text_cell, widget_cell, EXCEL_ALT_ROW_BG,
    EXCEL_GRID_BORDER, EXCEL_HEADER_BG, EXCEL_READONLY_BG,
};
use crate::ui_common::{draw_dashed_line, draw_rotated_y_title};

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
}

#[derive(Debug, Clone)]
struct CombinerDataset {
    id: usize,
    main_element: String,
    temp_c: f64,
    elements: Vec<ElementSpec>,
    result: Option<EhPhResult>,
    error: Option<String>,
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
    elem_mol_bufs: std::collections::HashMap<String, String>,
    elem_pres_bufs: std::collections::HashMap<String, String>,
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
    status_error: Option<String>,
    show_print_dialog: bool,
    print_header: String,
    print_footer: String,
    print_landscape: bool,
    // Combiner state
    combiner_datasets: Vec<CombinerDataset>,
    combiner_line_text: CombinerLineText,
    combiner_show_table: bool,
    delta_g_overrides_kcal: BTreeMap<String, f64>,
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
            elem_mol_bufs: std::collections::HashMap::new(),
            elem_pres_bufs: std::collections::HashMap::new(),
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
            status_error: None,
            show_print_dialog: false,
            print_header: "Cu - S - System at 25.00 °C".to_string(),
            print_footer: String::new(),
            print_landscape: false,
            combiner_datasets: vec![
                CombinerDataset {
                    id: 1,
                    main_element: "Cu".to_string(),
                    temp_c: 25.0,
                    elements: vec![
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
                    result: None,
                    error: None,
                },
                CombinerDataset {
                    id: 2,
                    main_element: "Cu".to_string(),
                    temp_c: 50.0,
                    elements: vec![
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
                    result: None,
                    error: None,
                },
                CombinerDataset {
                    id: 3,
                    main_element: "Cu".to_string(),
                    temp_c: 75.0,
                    elements: vec![
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
                    result: None,
                    error: None,
                },
            ],
            combiner_line_text: CombinerLineText::Temperature,
            combiner_show_table: false,
            delta_g_overrides_kcal: BTreeMap::from([("FeOH(+a)".to_string(), -69.268)]),
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

    fn refresh_candidate_species(&mut self, db: &SpeciesDb, select_all: bool) {
        let mut elems: Vec<String> = self
            .selected_elements
            .iter()
            .map(|e| e.symbol.clone())
            .collect();
        if !elems.iter().any(|e| e == "H") {
            elems.push("H".to_string());
        }
        if !elems.iter().any(|e| e == "O") {
            elems.push("O".to_string());
        }
        let mut list = db.search_by_elements(
            &elems,
            self.mode_gases || self.mode_gas_ions,
            self.mode_liquids || self.mode_condensed,
            self.mode_condensed,
            self.mode_aq_ions,
            self.mode_aq_neutrals,
            500,
        );
        if !self.mode_organics {
            // Keep inorganic or species without C-H organic chains when Organics is unchecked,
            // unless C is explicitly selected by the user.
            let c_selected = self.selected_elements.iter().any(|e| e.symbol == "C");
            if !c_selected {
                list.retain(|sp| !sp.formula.contains("CH"));
            }
        }
        self.candidate_species = list;
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
                };
                if self.create_new_page || self.pages.is_empty() {
                    let idx = self.pages.len();
                    self.pages.push(page);
                    self.active_tab = RightTab::Chart(idx);
                } else {
                    let target = match self.active_tab {
                        RightTab::Chart(i) if i < self.pages.len() => i,
                        _ => 0,
                    };
                    let keep_name = self.pages[target].name.clone();
                    self.pages[target] = page;
                    self.pages[target].name = keep_name;
                    self.active_tab = RightTab::Chart(target);
                }
            }
            Err(err) => {
                self.status_error = Some(err);
            }
        }
    }

    fn draw_combiner(&mut self, db: &SpeciesDb) {
        for ds in &mut self.combiner_datasets {
            let mut elems: Vec<String> = ds.elements.iter().map(|e| e.symbol.clone()).collect();
            if !elems.iter().any(|e| e == "H") {
                elems.push("H".to_string());
            }
            if !elems.iter().any(|e| e == "O") {
                elems.push("O".to_string());
            }
            let species = db.search_by_elements(
                &elems,
                self.mode_gases || self.mode_gas_ions,
                self.mode_liquids || self.mode_condensed,
                self.mode_condensed,
                self.mode_aq_ions,
                self.mode_aq_neutrals,
                500,
            );
            let req = EhPhRequest {
                elements: ds.elements.clone(),
                main_element: ds.main_element.clone(),
                species,
                t_k: (ds.temp_c + 273.15).max(273.15),
                min_ph: self.min_ph_str.trim().parse().unwrap_or(0.0),
                max_ph: self.max_ph_str.trim().parse().unwrap_or(14.0),
                min_eh_v: self.min_eh_str.trim().parse().unwrap_or(-2.0),
                max_eh_v: self.max_eh_str.trim().parse().unwrap_or(2.0),
                potential_scale: self.potential_scale,
                criss_cobble: self.criss_cobble,
                use_davies: self.use_davies,
                ionic_strength: self.ionic_strength_str.trim().parse().unwrap_or(0.0),
                correction_factor: self.correction_factor_str.trim().parse().unwrap_or(1.0),
                dielectric_constant: None,
                delta_g_h2o_kcal: None,
                delta_g_overrides_kcal: self.delta_g_overrides_kcal.clone(),
                show_predominance_ions: self.show_predominance_ions,
                grid_resolution: 160,
            };
            match calculate_ehph(&req, Some(db)) {
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
                    ui.strong("Eh-pH Diagram");
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
                            self.combiner_datasets.push(CombinerDataset {
                                id: next_id,
                                main_element: self.main_element.clone(),
                                temp_c: last_t,
                                elements: self.selected_elements.clone(),
                                result: None,
                                error: None,
                            });
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

                        cols[1].label(egui::RichText::new("Select Species").size(11.5).strong());
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
                                            if ui.selectable_label(is_sel, &sp.formula).clicked() {
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

                                ui.checkbox(
                                    &mut self.use_davies,
                                    "Use Davies Method for Activity",
                                );
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
                                            ui.label("mol/kg H2O");
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
                        ui.label("Temperature:");
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
                ui.horizontal(|ui| {
                    for (idx, page) in self.pages.iter().enumerate() {
                        let sel = self.active_tab == RightTab::Chart(idx);
                        if ui.selectable_label(sel, &page.name).clicked() {
                            self.active_tab = RightTab::Chart(idx);
                        }
                    }
                    let tbl_sel = self.active_tab == RightTab::Table;
                    if ui.selectable_label(tbl_sel, "Table (Basic Settings)").clicked() {
                        self.active_tab = RightTab::Table;
                    }
                });
                ui.separator();

                match self.active_tab {
                    RightTab::Chart(idx) => {
                        if let Some(page) = self.pages.get(idx).cloned() {
                            egui::ScrollArea::both()
                                .id_salt("ehph_chart_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    self.ui_chart_page(ui, &page);
                                });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Select elements and click Draw to generate an Eh-pH diagram.");
                            });
                        }
                    }
                    RightTab::Table => {
                        if let Some(page) = self.pages.last().cloned() {
                            self.ui_table_sheet(ui, &page);
                        } else {
                            ui.label("No computed diagram yet. Click Draw first.");
                        }
                    }
                }
            });
    }

    fn ui_chart_page(&self, ui: &mut egui::Ui, page: &ChartPage) {
        if !page.result.warnings.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for w in &page.result.warnings {
                    ui.colored_label(egui::Color32::from_rgb(180, 83, 9), format!("⚠ {w}"));
                }
            });
        }

        // Title
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(&page.result.title)
                    .size(14.5)
                    .strong()
                    .color(egui::Color32::BLACK),
            );
        });

        let avail = ui.available_size();
        let legend_h = 26.0 + (page.elements.len() as f32) * 19.0;
        let plot_h = (avail.y - legend_h - 14.0).max(180.0);
        let plot_w = (avail.x - 16.0).max(240.0);

        let (outer_rect, resp) =
            ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::hover());
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

        // Dotted gridlines (pH step 2, Eh step 0.2)
        let grid_col = egui::Color32::from_rgb(205, 210, 218);
        let mut ph = (page.min_ph / 2.0).ceil() * 2.0;
        while ph <= page.max_ph + 1e-6 {
            let x = ph_to_x(ph);
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
                format!("{ph:.0}"),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            ph += 2.0;
        }

        let mut eh = (page.min_eh / 0.2).ceil() * 0.2;
        while eh <= page.max_eh + 1e-6 {
            let y = eh_to_y(eh);
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
                format!("{eh:.1}"),
                egui::FontId::proportional(11.0),
                egui::Color32::BLACK,
            );
            eh += 0.2;
        }

        // Clipped painter for lines inside plot box
        let clipped = painter.with_clip_rect(plot_rect);

        // Water stability dashed lines (blue when Show colors is on, dark gray otherwise)
        let water_col = if page.show_colors {
            egui::Color32::from_rgb(0, 0, 220)
        } else {
            egui::Color32::from_rgb(35, 35, 35)
        };
        let water_stroke = egui::Stroke::new(1.5, water_col);
        for line in [&page.result.water_upper_line, &page.result.water_lower_line] {
            draw_dashed_line(
                &clipped,
                egui::pos2(ph_to_x(line[0].0), eh_to_y(line[0].1)),
                egui::pos2(ph_to_x(line[1].0), eh_to_y(line[1].1)),
                4.0,
                3.0,
                water_stroke,
            );
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

        // Species labels inside predominance regions (clamped so edge labels never clip)
        for reg in &page.result.regions {
            let mut pos = egui::pos2(ph_to_x(reg.label_ph), eh_to_y(reg.label_eh_v));
            let est_half_w = (reg.species.len() as f32 * 3.8).max(16.0);
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
            clipped.text(
                pos,
                egui::Align2::CENTER_CENTER,
                &reg.species,
                egui::FontId::proportional(12.5),
                egui::Color32::BLACK,
            );
        }

        // Plot border & axis titles
        painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.3, egui::Color32::BLACK));
        painter.text(
            egui::pos2(plot_rect.center().x, plot_rect.bottom() + 26.0),
            egui::Align2::CENTER_CENTER,
            "pH",
            egui::FontId::proportional(12.0),
            egui::Color32::BLACK,
        );
        draw_rotated_y_title(
            &painter,
            outer_rect.left() + 12.0,
            plot_rect.center().y,
            page.potential_scale.label(),
            egui::FontId::proportional(11.5),
            egui::Color32::BLACK,
        );

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

    fn ui_table_sheet(&self, ui: &mut egui::Ui, page: &ChartPage) {
        ui.strong("E-pH Diagram Basic Settings");
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
                    ("F", 95.0),
                    ("G", 85.0),
                ] {
                    header_cell(ui, lbl, w);
                }
            });

            // Row 1: Title
            ui.horizontal(|ui| {
                row_num_cell(ui, 1, 32.0);
                section_cell(ui, &page.result.title, 740.0, false);
            });

            let settings_rows: Vec<(&str, String, &str)> = vec![
                ("Temperature", format!("{:.3}", page.result.t_k - 273.15), "°C"),
                ("Show Colors", format!("{}", page.show_colors).to_uppercase(), ""),
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
                    text_cell(ui, v, 95.0, egui::Color32::WHITE, egui::Color32::BLACK, true, true, false);
                    text_cell(ui, u, 85.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                });
            }

            // Species table header (Row 16)
            ui.horizontal(|ui| {
                row_num_cell(ui, 16, 32.0);
                header_cell(ui, "Species", 150.0);
                header_cell(ui, "DeltaG kcal/mol", 95.0);
                header_cell(ui, "Area no", 95.0);
                header_cell(ui, "X1 pH", 95.0);
                header_cell(ui, "Y1 V", 125.0);
                header_cell(ui, "X2 pH", 95.0);
                header_cell(ui, "Y2 V", 85.0);
            });

            let mut row_i = 17;
            for sp_row in &page.result.table_rows {
                let bg = if row_i % 2 == 0 {
                    EXCEL_ALT_ROW_BG
                } else {
                    egui::Color32::WHITE
                };
                if sp_row.segments.is_empty() {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, row_i, 32.0);
                        text_cell(ui, &sp_row.formula, 150.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, &format!("{:.3}", sp_row.delta_g_kcal_mol), 95.0, bg, egui::Color32::BLACK, true, true, false);
                        let area_s = sp_row.area_no.map(|a| a.to_string()).unwrap_or_default();
                        text_cell(ui, &area_s, 95.0, bg, egui::Color32::BLACK, true, false, false);
                        text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 125.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                        text_cell(ui, "", 85.0, bg, egui::Color32::BLACK, false, false, false);
                    });
                    row_i += 1;
                } else {
                    for (seg_idx, &(x1, y1, x2, y2)) in sp_row.segments.iter().take(6).enumerate() {
                        ui.horizontal(|ui| {
                            row_num_cell(ui, row_i, 32.0);
                            if seg_idx == 0 {
                                text_cell(ui, &sp_row.formula, 150.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, &format!("{:.3}", sp_row.delta_g_kcal_mol), 95.0, bg, egui::Color32::BLACK, true, true, false);
                                let area_s = sp_row.area_no.map(|a| a.to_string()).unwrap_or_default();
                                text_cell(ui, &area_s, 95.0, bg, egui::Color32::BLACK, true, false, false);
                            } else {
                                text_cell(ui, "", 150.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 95.0, bg, egui::Color32::BLACK, false, false, false);
                            }
                            text_cell(ui, &format!("{x1:.3}"), 95.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{y1:.3}"), 125.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{x2:.3}"), 95.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                            text_cell(ui, &format!("{y2:.3}"), 85.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, true, false);
                        });
                        row_i += 1;
                    }
                }
            }
        });
    }

    fn ui_combiner_view(&mut self, ctx: &egui::Context, _db: &SpeciesDb) {
        egui::SidePanel::left("ehph_combiner_left")
            .resizable(true)
            .default_width(285.0)
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
                                    ui.label("Temperature:");
                                    let mut ts = format!("{:.1}", ds.temp_c);
                                    if ui
                                        .add_sized([65.0, 20.0], egui::TextEdit::singleline(&mut ts))
                                        .changed()
                                    {
                                        if let Ok(v) = ts.trim().parse::<f64>() {
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
                                            let mut ms = format!("{:.2E}", el.molality);
                                            if ui
                                                .add_sized(
                                                    [72.0, 18.0],
                                                    egui::TextEdit::singleline(&mut ms),
                                                )
                                                .lost_focus()
                                            {
                                                if let Ok(v) = ms.trim().parse::<f64>() {
                                                    if v > 0.0 {
                                                        el.molality = v;
                                                    }
                                                }
                                            }
                                            let mut ps = format!("{:.2E}", el.pressure_bar);
                                            if ui
                                                .add_sized(
                                                    [72.0, 18.0],
                                                    egui::TextEdit::singleline(&mut ps),
                                                )
                                                .lost_focus()
                                            {
                                                if let Ok(v) = ps.trim().parse::<f64>() {
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
                        }
                    }
                });
            });

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
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for ds in &self.combiner_datasets {
                            ui.strong(format!(
                                "Dataset {} — {} at {:.1} °C",
                                ds.id, ds.main_element, ds.temp_c
                            ));
                            if let Some(res) = &ds.result {
                                ui.label(format!(
                                    "Active regions: {} | Boundary segments: {}",
                                    res.regions.len(),
                                    res.segments.len()
                                ));
                            }
                            ui.add_space(4.0);
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

                let margin_l = 58.0;
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
                    plot_rect.left() + (((ph - min_ph) / (max_ph - min_ph)) as f32) * plot_rect.width()
                };
                let eh_to_y = |eh: f64| -> f32 {
                    plot_rect.bottom() - (((eh - min_eh) / (max_eh - min_eh)) as f32) * plot_rect.height()
                };

                painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

                let grid_col = egui::Color32::from_rgb(205, 210, 218);
                let mut ph = (min_ph / 2.0).ceil() * 2.0;
                while ph <= max_ph + 1e-6 {
                    let x = ph_to_x(ph);
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
                        format!("{ph:.0}"),
                        egui::FontId::proportional(11.0),
                        egui::Color32::BLACK,
                    );
                    ph += 2.0;
                }
                let mut eh = (min_eh / 0.2).ceil() * 0.2;
                while eh <= max_eh + 1e-6 {
                    let y = eh_to_y(eh);
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
                        format!("{eh:.1}"),
                        egui::FontId::proportional(11.0),
                        egui::Color32::BLACK,
                    );
                    eh += 0.2;
                }

                let clipped = painter.with_clip_rect(plot_rect);
                let palette = [
                    egui::Color32::BLACK,
                    egui::Color32::from_rgb(37, 99, 235),
                    egui::Color32::from_rgb(185, 28, 28),
                    egui::Color32::from_rgb(21, 128, 61),
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
                    for (seg_i, seg) in res.segments.iter().enumerate() {
                        let p1 = egui::pos2(ph_to_x(seg.x1_ph), eh_to_y(seg.y1_v));
                        let p2 = egui::pos2(ph_to_x(seg.x2_ph), eh_to_y(seg.y2_v));
                        clipped.line_segment([p1, p2], stroke);
                        if seg_i % 45 == 12 && self.combiner_line_text != CombinerLineText::None {
                            let tag = match self.combiner_line_text {
                                CombinerLineText::MainElement => ds.main_element.clone(),
                                CombinerLineText::Temperature => format!("{:.0}", ds.temp_c),
                                CombinerLineText::Molality => ds
                                    .elements
                                    .first()
                                    .map(|e| format!("{:.1E}", e.molality))
                                    .unwrap_or_default(),
                                CombinerLineText::Pressure => ds
                                    .elements
                                    .first()
                                    .map(|e| format!("{:.1E}", e.pressure_bar))
                                    .unwrap_or_default(),
                                CombinerLineText::DatasetNumber => format!("DS{}", ds.id),
                                CombinerLineText::None => String::new(),
                            };
                            let mid = egui::pos2(0.5 * (p1.x + p2.x), 0.5 * (p1.y + p2.y));
                            clipped.text(
                                mid,
                                egui::Align2::CENTER_CENTER,
                                tag,
                                egui::FontId::proportional(10.0),
                                col,
                            );
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

                painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.3, egui::Color32::BLACK));
                painter.text(
                    egui::pos2(plot_rect.center().x, plot_rect.bottom() + 24.0),
                    egui::Align2::CENTER_CENTER,
                    "pH",
                    egui::FontId::proportional(12.0),
                    egui::Color32::BLACK,
                );
                painter.text(
                    egui::pos2(outer_rect.left() + 18.0, plot_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    self.potential_scale.label(),
                    egui::FontId::proportional(11.5),
                    egui::Color32::BLACK,
                );
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
                ui.label(
                    egui::RichText::new(
                        "Preview ready. Use Export CSV from Table or OS Print/Screenshot.",
                    )
                    .size(11.0),
                );
                if ui.button("Close").clicked() {
                    self.show_print_dialog = false;
                }
            });
        if !open {
            self.show_print_dialog = false;
        }
    }
}

fn find_h2o_species(db: &SpeciesDb) -> Option<ChemicalSpecies> {
    let matches = db.search("H2O", 12);
    matches
        .into_iter()
        .find(|s| s.formula == "H2O" || s.formula == "H2O(L)")
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
