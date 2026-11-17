use std::f32::consts::TAU;

use eframe::egui;
use serde::{Deserialize, Serialize};
use soryntel_application::SoryntelApplication;
use soryntel_thermo::{
    species_converter::{
        ConversionMethod, ConverterInputEntry, ConverterOutputSpec, SpeciesConverterRequest,
        SpeciesConverterResult, SpeciesTypeFilter,
    },
    SpeciesCatalogPort,
};

use crate::excel_grid::{
    header_cell, row_num_cell, section_cell, text_cell, EXCEL_ALT_ROW_BG, EXCEL_GRID_BORDER,
    EXCEL_READONLY_BG,
};

const PIE_COLORS: &[egui::Color32] = &[
    egui::Color32::from_rgb(76, 132, 196),
    egui::Color32::from_rgb(192, 80, 77),
    egui::Color32::from_rgb(155, 187, 89),
    egui::Color32::from_rgb(128, 100, 162),
    egui::Color32::from_rgb(75, 172, 198),
    egui::Color32::from_rgb(247, 150, 70),
    egui::Color32::from_rgb(88, 96, 108),
    egui::Color32::from_rgb(176, 196, 222),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RibbonTab {
    Home,
    Tools,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InputRowEdit {
    formula: String,
    wt_pct_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutputRowEdit {
    formula: String,
    target_str: String,
    weight_str: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConverterSheet {
    name: String,
    is_summary: bool,
    doc_name: String,
    doc_date: String,
    doc_author: String,
    inputs: Vec<InputRowEdit>,
    outputs: Vec<OutputRowEdit>,
    result: Option<SpeciesConverterResult>,
}

impl ConverterSheet {
    fn new_cu_s_fe_ni(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_summary: false,
            doc_name: "Cu-S-Fe-Ni Conversion".into(),
            doc_date: "2026-11-14".into(),
            doc_author: "Soryntel".into(),
            inputs: vec![
                InputRowEdit { formula: "Cu".into(), wt_pct_str: "25.000".into() },
                InputRowEdit { formula: "S".into(), wt_pct_str: "40.000".into() },
                InputRowEdit { formula: "Fe".into(), wt_pct_str: "10.000".into() },
                InputRowEdit { formula: "Ni".into(), wt_pct_str: "25.000".into() },
            ],
            outputs: vec![
                OutputRowEdit { formula: "CuS".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "Cu2S".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "CuFeS2".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "FeS2".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "NiS".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "Ni3S2".into(), target_str: "".into(), weight_str: "1.000".into() },
            ],
            result: None,
        }
    }

    fn new_example_conversion(name: &str) -> Self {
        let out = |f: &str, t: &str, w: &str| OutputRowEdit {
            formula: f.into(),
            target_str: t.into(),
            weight_str: w.into(),
        };
        let inp = |f: &str, v: &str| InputRowEdit {
            formula: f.into(),
            wt_pct_str: v.into(),
        };
        Self {
            name: name.to_string(),
            is_summary: false,
            doc_name: "Example Conversion".into(),
            doc_date: "2026-11-14".into(),
            doc_author: "Soryntel".into(),
            inputs: vec![
                inp("S", "32.850"),
                inp("Cu", "25.700"),
                inp("Fe", "24.700"),
                inp("Ni", "0.550"),
                inp("Co", "0.010"),
                inp("Zn", "4.050"),
                inp("Pb", "3.900"),
                inp("Sb", "0.200"),
                inp("As", "0.130"),
                inp("Ag", "0.090"),
                inp("Au", "0.001"),
                inp("SiO2", "4.650"),
                inp("Al2O3", "2.200"),
                inp("MgO", "0.150"),
            ],
            outputs: vec![
                out("CuS", "", "1.000"),
                out("Cu2S", "", "1.000"),
                out("CuFeS2", "60.000", "10.000"),
                out("FeS", "", "1.000"),
                out("FeS2", "15.000", "10.000"),
                out("NiS", "", "1.000"),
                out("CoS", "", "1.000"),
                out("ZnS", "", "1.000"),
                out("PbS", "4.500", "10.000"),
                out("Sb2S3", "", "1.000"),
                out("As2S3", "", "1.000"),
                out("Ag2S", "", "1.000"),
                out("Au", "", "1.000"),
                out("SiO2", "", "1.000"),
                out("Al2O3", "", "1.000"),
                out("MgO", "", "1.000"),
            ],
            result: None,
        }
    }

    fn new_blank(name: &str) -> Self {
        Self {
            name: name.to_string(),
            is_summary: false,
            doc_name: name.to_string(),
            doc_date: "2026-11-14".into(),
            doc_author: "Soryntel".into(),
            inputs: vec![
                InputRowEdit { formula: "Cu".into(), wt_pct_str: "50.000".into() },
                InputRowEdit { formula: "S".into(), wt_pct_str: "50.000".into() },
            ],
            outputs: vec![
                OutputRowEdit { formula: "CuS".into(), target_str: "".into(), weight_str: "1.000".into() },
                OutputRowEdit { formula: "Cu2S".into(), target_str: "".into(), weight_str: "1.000".into() },
            ],
            result: None,
        }
    }
}

pub struct SpeciesConverterModuleState {
    ribbon_tab: RibbonTab,
    sheets: Vec<ConverterSheet>,
    active_sheet: usize,

    method: ConversionMethod,
    exact_o: bool,
    exact_h: bool,
    set_norm_limit: bool,
    norm_limit_str: String,
    small_meas_limit_str: String,
    element_limit_str: String,
    stability_type: SpeciesTypeFilter,
    stability_coef_str: String,

    show_charts: bool,
    show_element_balance: bool,
    status_msg: Option<String>,
}

impl SpeciesConverterModuleState {
    pub fn new(catalog: &dyn SpeciesCatalogPort) -> Self {
        let mut s = Self {
            ribbon_tab: RibbonTab::Home,
            sheets: vec![ConverterSheet::new_cu_s_fe_ni("Sheet1")],
            active_sheet: 0,
            method: ConversionMethod::Method1,
            exact_o: true,
            exact_h: true,
            set_norm_limit: false,
            norm_limit_str: "0.0001".into(),
            small_meas_limit_str: "0.5".into(),
            element_limit_str: "99.0".into(),
            stability_type: SpeciesTypeFilter::None,
            stability_coef_str: "80".into(),
            show_charts: true,
            show_element_balance: false,
            status_msg: None,
        };
        s.solve_active(catalog);
        s
    }

    fn build_request(&self, sheet: &ConverterSheet) -> SpeciesConverterRequest {
        let inputs = sheet
            .inputs
            .iter()
            .filter(|r| !r.formula.trim().is_empty())
            .map(|r| ConverterInputEntry {
                formula: r.formula.trim().to_string(),
                wt_pct: r.wt_pct_str.trim().parse::<f64>().unwrap_or(0.0),
            })
            .collect();

        let outputs = sheet
            .outputs
            .iter()
            .filter(|r| !r.formula.trim().is_empty())
            .map(|r| ConverterOutputSpec {
                formula: r.formula.trim().to_string(),
                target_wt_pct: if r.target_str.trim().is_empty() {
                    None
                } else {
                    r.target_str.trim().parse::<f64>().ok()
                },
                weight_coeff: r.weight_str.trim().parse::<f64>().unwrap_or(1.0),
            })
            .collect();

        SpeciesConverterRequest {
            inputs,
            outputs,
            method: self.method,
            exact_o: self.exact_o,
            exact_h: self.exact_h,
            set_norm_limit: self.set_norm_limit,
            norm_limit: self.norm_limit_str.trim().parse::<f64>().unwrap_or(0.0001),
            small_meas_limit: self.small_meas_limit_str.trim().parse::<f64>().unwrap_or(0.5),
            element_balance_limit_pct: self.element_limit_str.trim().parse::<f64>().unwrap_or(99.0),
            stability_type: self.stability_type,
            stability_weighting_coef: self.stability_coef_str.trim().parse::<f64>().unwrap_or(80.0),
        }
    }

    fn initialize_active(&mut self) {
        self.status_msg = None;
        let Some(sheet) = self.sheets.get(self.active_sheet) else {
            return;
        };
        if sheet.is_summary {
            return;
        }
        let req = self.build_request(sheet);
        match SoryntelApplication::initialize_species_converter(&req.inputs) {
            Ok((calc_elems, in_tot)) => {
                let sheet = &mut self.sheets[self.active_sheet];
                sheet.result = Some(soryntel_thermo::species_converter::SpeciesConverterResult {
                    input_total_wt_pct: in_tot,
                    calculated_input_elements: calc_elems,
                    calculated_input_total_wt_pct: in_tot,
                    output_rows: Vec::new(),
                    output_total_wt_pct: 0.0,
                    target_total_wt_pct: 0.0,
                    others_wt_pct: 0.0,
                    residual_error: 0.0,
                    element_balance: Vec::new(),
                });
            }
            Err(e) => self.status_msg = Some(e.to_string()),
        }
    }

    fn solve_active(&mut self, catalog: &dyn SpeciesCatalogPort) {
        self.status_msg = None;
        let Some(sheet) = self.sheets.get(self.active_sheet) else {
            return;
        };
        if sheet.is_summary {
            return;
        }
        let req = self.build_request(sheet);
        match SoryntelApplication::convert_species(catalog, &req) {
            Ok(res) => {
                self.sheets[self.active_sheet].result = Some(res);
            }
            Err(e) => self.status_msg = Some(e.to_string()),
        }
    }

    fn fix_balance_active(&mut self, catalog: &dyn SpeciesCatalogPort) {
        self.status_msg = None;
        let Some(sheet) = self.sheets.get(self.active_sheet) else {
            return;
        };
        if sheet.is_summary {
            return;
        }
        let req = self.build_request(sheet);
        let mut outs = req.outputs.clone();
        if let Err(e) = SoryntelApplication::fix_species_balance(&req.inputs, &mut outs) {
            self.status_msg = Some(e.to_string());
            return;
        }
        let sheet = &mut self.sheets[self.active_sheet];
        sheet.outputs = outs
            .into_iter()
            .map(|o| OutputRowEdit {
                formula: o.formula,
                target_str: o.target_wt_pct.map(|v| format!("{v:.3}")).unwrap_or_default(),
                weight_str: format!("{:.3}", o.weight_coeff),
            })
            .collect();
        self.solve_active(catalog);
    }

    fn save_workbook(&mut self) {
        let path = "/home/nando/.gemini/antigravity/scratch/species_converter_workbook.json";
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&self.sheets) {
            Ok(json) => match crate::storage::write_atomic(path, json) {
                Ok(_) => self.status_msg = Some(format!("Saved workbook to {path}")),
                Err(e) => self.status_msg = Some(format!("Save error: {e}")),
            },
            Err(e) => self.status_msg = Some(format!("Serialize error: {e}")),
        }
    }

    fn open_workbook(&mut self) {
        let path = "/home/nando/.gemini/antigravity/scratch/species_converter_workbook.json";
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<Vec<ConverterSheet>>(&json) {
                Ok(loaded) if !loaded.is_empty() => {
                    self.sheets = loaded;
                    self.active_sheet = 0;
                    self.status_msg = Some(format!("Loaded workbook from {path}"));
                }
                Ok(_) => self.status_msg = Some("Workbook file is empty.".into()),
                Err(e) => self.status_msg = Some(format!("Parse error: {e}")),
            },
            Err(e) => self.status_msg = Some(format!("Open error: {e}")),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, catalog: &dyn SpeciesCatalogPort) {
        // Top Ribbon Bar (Home / Tools)
        egui::TopBottomPanel::top("sc_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(241, 244, 249))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                self.render_ribbon(ui, catalog);
            });

        // Right Charts Panel (dockable & resizable)
        if self.show_charts {
            egui::SidePanel::right("sc_charts_panel")
                .resizable(true)
                .default_width(270.0)
                .min_width(190.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            self.render_charts_panel(ui);
                        });
                });
        }

        // Main Central Spreadsheet + Sheet Tabs
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_central_sheet(ui, catalog);
        });

        // Element Balance Modal Window (Fig. 4)
        if self.show_element_balance {
            let mut open = self.show_element_balance;
            egui::Window::new(crate::i18n::text("common.086"))
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 260.0])
                .min_size([320.0, 180.0])
                .show(ctx, |ui| {
                    self.render_element_balance_modal(ui);
                });
            self.show_element_balance = open;
        }
    }

    fn render_ribbon(&mut self, ui: &mut egui::Ui, catalog: &dyn SpeciesCatalogPort) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.ribbon_tab, RibbonTab::Home, "Home");
            ui.selectable_value(&mut self.ribbon_tab, RibbonTab::Tools, "Tools");
            ui.separator();
            if ui.small_button("Cu-S-Fe-Ni (Fig. 3)").clicked() {
                self.sheets[self.active_sheet] =
                    ConverterSheet::new_cu_s_fe_ni(&self.sheets[self.active_sheet].name);
                self.solve_active(catalog);
            }
            if ui.small_button("Example Conversion (Fig. 12)").clicked() {
                self.sheets[self.active_sheet] =
                    ConverterSheet::new_example_conversion(&self.sheets[self.active_sheet].name);
                self.solve_active(catalog);
            }
            if let Some(msg) = &self.status_msg {
                ui.separator();
                ui.colored_label(egui::Color32::from_rgb(165, 30, 30), msg);
            }
        });
        ui.separator();

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| match self.ribbon_tab {
                RibbonTab::Home => {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                if ui.button(crate::i18n::text("common.033")).clicked() {
                                    self.sheets = vec![ConverterSheet::new_blank("Sheet1")];
                                    self.active_sheet = 0;
                                    self.solve_active(catalog);
                                }
                                if ui.button(crate::i18n::text("common.087")).clicked() {
                                    self.open_workbook();
                                }
                                if ui.button(crate::i18n::text("common.034")).clicked() {
                                    self.save_workbook();
                                }
                            });
                            ui.small("File");
                        });
                    });

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Add Calculation Sheet").clicked() {
                                    let idx = self.sheets.iter().filter(|s| !s.is_summary).count() + 1;
                                    self.sheets
                                        .push(ConverterSheet::new_cu_s_fe_ni(&format!("Sheet{idx}")));
                                    self.active_sheet = self.sheets.len() - 1;
                                    self.solve_active(catalog);
                                }
                                if ui.button("Add Summary Sheet").clicked() {
                                    self.sheets.push(ConverterSheet {
                                        name: "Summary".into(),
                                        is_summary: true,
                                        doc_name: "Summary".into(),
                                        doc_date: "2026-11-14".into(),
                                        doc_author: "Soryntel".into(),
                                        inputs: Vec::new(),
                                        outputs: Vec::new(),
                                        result: None,
                                    });
                                    self.active_sheet = self.sheets.len() - 1;
                                }
                                if ui
                                    .add_enabled(
                                        self.sheets.len() > 1,
                                        egui::Button::new("Delete Sheet"),
                                    )
                                    .clicked()
                                {
                                    self.sheets.remove(self.active_sheet);
                                    self.active_sheet =
                                        self.active_sheet.min(self.sheets.len().saturating_sub(1));
                                }
                            });
                            ui.small("Workbook");
                        });
                    });

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Fix Element Balance").clicked() {
                                    self.fix_balance_active(catalog);
                                }
                                if ui.button("Initialize").clicked() {
                                    self.initialize_active();
                                }
                                if ui
                                    .button(egui::RichText::new(crate::i18n::text("common.088")).strong())
                                    .clicked()
                                {
                                    self.solve_active(catalog);
                                }
                                if ui.button(crate::i18n::text("common.086")).clicked() {
                                    self.show_element_balance = !self.show_element_balance;
                                }
                                if ui.selectable_label(self.show_charts, "Charts").clicked() {
                                    self.show_charts = !self.show_charts;
                                }
                            });
                            ui.small("Actions");
                        });
                    });
                }
                RibbonTab::Tools => {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Norm limit");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.norm_limit_str)
                                        .desired_width(60.0),
                                );
                            });
                            ui.checkbox(&mut self.set_norm_limit, "Set norm below the limit");
                            ui.small("Norm settings");
                        });
                    });

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.checkbox(&mut self.exact_o, "Exact O measurement");
                            ui.checkbox(&mut self.exact_h, "Exact H measurement");
                            ui.small("Measurements");
                        });
                    });

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Small Meas Limit");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.small_meas_limit_str)
                                        .desired_width(48.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Method");
                                egui::ComboBox::from_id_salt("sc_method")
                                    .width(96.0)
                                    .selected_text(self.method.label())
                                    .show_ui(ui, |ui| {
                                        for m in [
                                            ConversionMethod::Method1,
                                            ConversionMethod::Method2,
                                            ConversionMethod::Method3,
                                        ] {
                                            ui.selectable_value(&mut self.method, m, m.label());
                                        }
                                    });
                            });
                            ui.small("General settings");
                        });
                    });

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Species type");
                                egui::ComboBox::from_id_salt("sc_stab_type")
                                    .width(105.0)
                                    .selected_text(self.stability_type.label())
                                    .show_ui(ui, |ui| {
                                        for t in [
                                            SpeciesTypeFilter::None,
                                            SpeciesTypeFilter::Sulfides,
                                            SpeciesTypeFilter::Oxides,
                                            SpeciesTypeFilter::Sulfates,
                                            SpeciesTypeFilter::Carbonates,
                                            SpeciesTypeFilter::Chlorides,
                                            SpeciesTypeFilter::Silicates,
                                            SpeciesTypeFilter::Hydroxides,
                                        ] {
                                            ui.selectable_value(
                                                &mut self.stability_type,
                                                t,
                                                t.label(),
                                            );
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Weighting coef.");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.stability_coef_str)
                                        .desired_width(54.0),
                                );
                            });
                            ui.small("Stability Weighting");
                        });
                    });
                }
            });
        });
    }

    fn render_central_sheet(&mut self, ui: &mut egui::Ui, catalog: &dyn SpeciesCatalogPort) {
        // Sheet tabs bar
        ui.horizontal_wrapped(|ui| {
            for (idx, sh) in self.sheets.iter().enumerate() {
                if ui
                    .selectable_label(self.active_sheet == idx, &sh.name)
                    .clicked()
                {
                    self.active_sheet = idx;
                }
            }
        });
        ui.separator();

        let is_summary = self
            .sheets
            .get(self.active_sheet)
            .map(|s| s.is_summary)
            .unwrap_or(false);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if is_summary {
                    self.render_summary_sheet(ui);
                } else {
                    self.render_calculation_sheet(ui, catalog);
                }
            });
    }

    fn render_summary_sheet(&self, ui: &mut egui::Ui) {
        ui.strong("Workbook Summary");
        ui.add_space(6.0);
        egui::Grid::new("sc_summary_grid")
            .spacing([0.0, 0.0])
            .show(ui, |ui| {
                section_cell(ui, "Sheet", 110.0, false);
                section_cell(ui, "Name", 160.0, false);
                section_cell(ui, "Input Total wt-%", 120.0, true);
                section_cell(ui, "Output Total wt-%", 120.0, true);
                section_cell(ui, "Residual Error", 110.0, true);
                ui.end_row();
                for sh in self.sheets.iter().filter(|s| !s.is_summary) {
                    let (in_t, out_t, res_e) = sh
                        .result
                        .as_ref()
                        .map(|r| {
                            (
                                format!("{:.3}", r.input_total_wt_pct),
                                format!("{:.3}", r.output_total_wt_pct),
                                format!("{:.3}", r.residual_error),
                            )
                        })
                        .unwrap_or_else(|| ("-".into(), "-".into(), "-".into()));
                    text_cell(ui, &sh.name, 110.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                    text_cell(ui, &sh.doc_name, 160.0, egui::Color32::WHITE, egui::Color32::BLACK, false, false, false);
                    text_cell(ui, &in_t, 120.0, egui::Color32::WHITE, egui::Color32::BLACK, true, false, false);
                    text_cell(ui, &out_t, 120.0, egui::Color32::WHITE, egui::Color32::BLACK, true, false, false);
                    text_cell(ui, &res_e, 110.0, egui::Color32::WHITE, egui::Color32::BLACK, true, false, false);
                    ui.end_row();
                }
            });
    }

    fn render_calculation_sheet(&mut self, ui: &mut egui::Ui, catalog: &dyn SpeciesCatalogPort) {
        let Some(sheet) = self.sheets.get_mut(self.active_sheet) else {
            return;
        };

        let res_err_str = sheet
            .result
            .as_ref()
            .map(|r| format!("{:.3}", r.residual_error))
            .unwrap_or_else(|| "0.000".into());

        // Top metadata bar (Rows 1–3 matching Fig. 3)
        ui.horizontal(|ui| {
            ui.label(crate::i18n::text("common.030"));
            ui.add(egui::TextEdit::singleline(&mut sheet.doc_name).desired_width(150.0));
            ui.label(crate::i18n::text("common.089"));
            ui.add(egui::TextEdit::singleline(&mut sheet.doc_date).desired_width(90.0));
            ui.label("Author:");
            ui.add(egui::TextEdit::singleline(&mut sheet.doc_author).desired_width(90.0));
            ui.separator();
            ui.strong(format!("Residual Error: {res_err_str}"));
        });
        ui.add_space(6.0);

        let result_clone = sheet.result.clone();
        let calc_elems = result_clone
            .as_ref()
            .map(|r| r.calculated_input_elements.as_slice())
            .unwrap_or(&[]);
        let out_rows = result_clone
            .as_ref()
            .map(|r| r.output_rows.as_slice())
            .unwrap_or(&[]);

        let in_total = result_clone
            .as_ref()
            .map(|r| r.input_total_wt_pct)
            .unwrap_or_else(|| {
                sheet
                    .inputs
                    .iter()
                    .map(|r| r.wt_pct_str.trim().parse::<f64>().unwrap_or(0.0))
                    .sum()
            });
        let calc_in_total = result_clone
            .as_ref()
            .map(|r| r.calculated_input_total_wt_pct)
            .unwrap_or(0.0);
        let out_total = result_clone
            .as_ref()
            .map(|r| r.output_total_wt_pct)
            .unwrap_or(0.0);
        let tgt_total = result_clone
            .as_ref()
            .map(|r| r.target_total_wt_pct)
            .unwrap_or(0.0);
        let others_val = result_clone
            .as_ref()
            .map(|r| r.others_wt_pct)
            .unwrap_or(0.0);

        let n_rows = sheet
            .inputs
            .len()
            .max(calc_elems.len())
            .max(sheet.outputs.len() + 1)
            .max(8);

        let mut remove_in: Option<usize> = None;
        let mut remove_out: Option<usize> = None;

        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
            .show(ui, |ui| {
                egui::Grid::new("sc_main_sheet_grid")
                    .spacing([0.0, 0.0])
                    .show(ui, |ui| {
                        // Col letters
                        header_cell(ui, "", 32.0);
                        header_cell(ui, "B", 88.0);
                        header_cell(ui, "C", 74.0);
                        header_cell(ui, "D", 88.0);
                        header_cell(ui, "E", 74.0);
                        header_cell(ui, "F", 18.0);
                        header_cell(ui, "G", 94.0);
                        header_cell(ui, "H", 84.0);
                        header_cell(ui, "I", 74.0);
                        header_cell(ui, "J", 74.0);
                        header_cell(ui, "K", 74.0);
                        ui.end_row();

                        // Section row (INPUT | OUTPUT)
                        row_num_cell(ui, 5, 32.0);
                        section_cell(ui, "INPUT", 88.0, false);
                        section_cell(ui, "", 74.0, false);
                        section_cell(ui, "", 88.0, false);
                        section_cell(ui, "", 74.0, false);
                        section_cell(ui, "", 18.0, false);
                        section_cell(ui, "OUTPUT", 94.0, false);
                        section_cell(ui, "", 84.0, false);
                        section_cell(ui, "", 74.0, false);
                        section_cell(ui, "", 74.0, false);
                        section_cell(ui, "", 74.0, false);
                        ui.end_row();

                        // Column titles row
                        row_num_cell(ui, 7, 32.0);
                        section_cell(ui, "Elements", 88.0, false);
                        section_cell(ui, "wt-%", 74.0, true);
                        section_cell(ui, "Calc. Elem", 88.0, false);
                        section_cell(ui, "wt-%", 74.0, true);
                        section_cell(ui, "", 18.0, false);
                        section_cell(ui, "Species", 94.0, false);
                        section_cell(ui, "Calc wt-%", 84.0, true);
                        section_cell(ui, "Target %", 74.0, true);
                        section_cell(ui, "Weight", 74.0, true);
                        section_cell(ui, "Type", 74.0, false);
                        ui.end_row();

                        // Total row (Row 8)
                        row_num_cell(ui, 8, 32.0);
                        section_cell(ui, "Total", 88.0, false);
                        section_cell(ui, &format!("{in_total:.3}"), 74.0, true);
                        section_cell(ui, "", 88.0, false);
                        section_cell(ui, &format!("{calc_in_total:.3}"), 74.0, true);
                        section_cell(ui, "", 18.0, false);
                        section_cell(ui, "", 94.0, false);
                        section_cell(ui, &format!("{out_total:.3}"), 84.0, true);
                        section_cell(ui, &format!("{tgt_total:.3}"), 74.0, true);
                        section_cell(ui, "", 74.0, false);
                        section_cell(ui, "", 74.0, false);
                        ui.end_row();

                        // Data rows
                        for r in 0..n_rows {
                            let bg = if r % 2 == 1 {
                                EXCEL_ALT_ROW_BG
                            } else {
                                egui::Color32::WHITE
                            };
                            row_num_cell(ui, 9 + r, 32.0);

                            // Input B & C
                            if let Some(in_row) = sheet.inputs.get_mut(r) {
                                ui.horizontal(|ui| {
                                    ui.set_width(88.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut in_row.formula)
                                            .desired_width(60.0),
                                    );
                                    if ui.small_button("×").clicked() {
                                        remove_in = Some(r);
                                    }
                                });
                                ui.add_sized(
                                    [74.0, 20.0],
                                    egui::TextEdit::singleline(&mut in_row.wt_pct_str),
                                );
                            } else {
                                text_cell(ui, "", 88.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, true, false, false);
                            }

                            // Calculated Elements D & E
                            if let Some(ce) = calc_elems.get(r) {
                                let col = if ce.exceeds_limit {
                                    egui::Color32::from_rgb(200, 30, 30)
                                } else {
                                    egui::Color32::BLACK
                                };
                                text_cell(ui, &ce.element, 88.0, EXCEL_READONLY_BG, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, &format!("{:.3}", ce.wt_pct), 74.0, EXCEL_READONLY_BG, col, true, false, false);
                            } else {
                                text_cell(ui, "", 88.0, EXCEL_READONLY_BG, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 74.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, false, false);
                            }

                            // Spacer F
                            text_cell(ui, "", 18.0, bg, egui::Color32::BLACK, false, false, false);

                            // Output G, H, I, J, K
                            if let Some(out_row) = sheet.outputs.get_mut(r) {
                                ui.horizontal(|ui| {
                                    ui.set_width(94.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut out_row.formula)
                                            .desired_width(66.0),
                                    );
                                    if ui.small_button("×").clicked() {
                                        remove_out = Some(r);
                                    }
                                });
                                let calc_str = out_rows
                                    .get(r)
                                    .map(|o| format!("{:.3}", o.calculated_wt_pct))
                                    .unwrap_or_default();
                                let stype = out_rows
                                    .get(r)
                                    .map(|o| o.species_type.as_str())
                                    .unwrap_or("");
                                text_cell(ui, &calc_str, 84.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, false, false);
                                ui.add_sized(
                                    [74.0, 20.0],
                                    egui::TextEdit::singleline(&mut out_row.target_str),
                                );
                                ui.add_sized(
                                    [74.0, 20.0],
                                    egui::TextEdit::singleline(&mut out_row.weight_str),
                                );
                                text_cell(ui, stype, 74.0, bg, egui::Color32::BLACK, false, false, false);
                            } else if r == sheet.outputs.len() {
                                text_cell(ui, "[Others]", 94.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, &format!("{others_val:.3}"), 84.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "0.001", 74.0, bg, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, false, false, false);
                            } else {
                                text_cell(ui, "", 94.0, bg, egui::Color32::BLACK, false, false, false);
                                text_cell(ui, "", 84.0, EXCEL_READONLY_BG, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, true, false, false);
                                text_cell(ui, "", 74.0, bg, egui::Color32::BLACK, false, false, false);
                            }

                            ui.end_row();
                        }
                    });
            });

        if let Some(i) = remove_in {
            self.sheets[self.active_sheet].inputs.remove(i);
        }
        if let Some(i) = remove_out {
            self.sheets[self.active_sheet].outputs.remove(i);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("+ Add Input").clicked() {
                self.sheets[self.active_sheet].inputs.push(InputRowEdit {
                    formula: String::new(),
                    wt_pct_str: "0.000".into(),
                });
            }
            if ui.button("+ Add Output Species").clicked() {
                self.sheets[self.active_sheet].outputs.push(OutputRowEdit {
                    formula: String::new(),
                    target_str: String::new(),
                    weight_str: "1.000".into(),
                });
            }
            if ui.button(crate::i18n::text("common.088")).clicked() {
                self.solve_active(catalog);
            }
        });
    }

    fn render_charts_panel(&self, ui: &mut egui::Ui) {
        ui.strong("Charts");
        ui.separator();
        let Some(sheet) = self.sheets.get(self.active_sheet) else {
            return;
        };
        let in_slices: Vec<(String, f64)> = sheet
            .inputs
            .iter()
            .filter_map(|r| {
                let v = r.wt_pct_str.trim().parse::<f64>().unwrap_or(0.0);
                if !r.formula.trim().is_empty() && v > 0.0 {
                    Some((r.formula.trim().to_string(), v))
                } else {
                    None
                }
            })
            .collect();
        draw_pie_chart(ui, "Input analysis", &in_slices);
        ui.add_space(10.0);

        let small_lim = self.small_meas_limit_str.trim().parse::<f64>().unwrap_or(0.5);
        let mut out_slices: Vec<(String, f64)> = Vec::new();
        let mut others = 0.0;
        if let Some(res) = &sheet.result {
            for r in &res.output_rows {
                if r.calculated_wt_pct >= small_lim {
                    out_slices.push((r.formula.clone(), r.calculated_wt_pct));
                } else if r.calculated_wt_pct > 0.0 {
                    others += r.calculated_wt_pct;
                }
            }
            if others > 0.0 {
                out_slices.push(("[Others]".into(), others));
            }
        }
        draw_pie_chart(ui, "Output analysis", &out_slices);
    }

    fn render_element_balance_modal(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Limit threshold (%):");
            ui.add(egui::TextEdit::singleline(&mut self.element_limit_str).desired_width(60.0));
        });
        ui.separator();

        let Some(res) = self
            .sheets
            .get(self.active_sheet)
            .and_then(|s| s.result.as_ref())
        else {
            ui.label("Run Solve first to inspect element balance.");
            return;
        };

        let thresh = self.element_limit_str.trim().parse::<f64>().unwrap_or(99.0);
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("sc_elem_balance_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    section_cell(ui, "Elements", 95.0, false);
                    for eb in &res.element_balance {
                        section_cell(ui, &eb.element, 82.0, true);
                    }
                    ui.end_row();

                    let metrics: [(&str, fn(&soryntel_thermo::species_converter::ElementBalanceRow) -> f64); 5] = [
                        ("Input kg", |r| r.input_kg),
                        ("Output kg", |r| r.output_kg),
                        ("Balance kg", |r| r.balance_kg),
                        ("Error %", |r| r.error_pct),
                        ("Limit", |r| r.limit),
                    ];
                    for (idx, (lbl, getter)) in metrics.iter().enumerate() {
                        text_cell(ui, lbl, 95.0, EXCEL_READONLY_BG, egui::Color32::BLACK, false, false, false);
                        for eb in &res.element_balance {
                            let val = getter(eb);
                            let col = if idx == 4 && val < thresh {
                                egui::Color32::from_rgb(200, 30, 30)
                            } else {
                                egui::Color32::BLACK
                            };
                            text_cell(ui, &format!("{val:.3}"), 82.0, egui::Color32::WHITE, col, true, false, false);
                        }
                        ui.end_row();
                    }
                });
        });
    }
}

fn draw_pie_chart(ui: &mut egui::Ui, title: &str, slices: &[(String, f64)]) {
    ui.vertical_centered(|ui| {
        ui.strong(title);
    });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 175.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::WHITE);

    let total: f64 = slices.iter().map(|(_, v)| *v).sum();
    if total <= 0.0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No data",
            egui::FontId::proportional(11.5),
            egui::Color32::GRAY,
        );
        return;
    }

    let center = egui::pos2(rect.center().x, rect.center().y + 4.0);
    let radius = 56.0_f32;
    let mut start_angle = -TAU * 0.25;

    for (idx, (label, val)) in slices.iter().enumerate() {
        let frac = (*val / total) as f32;
        if frac <= 0.001 {
            continue;
        }
        let sweep = frac * TAU;
        let end_angle = start_angle + sweep;
        let steps = ((sweep / 0.12).ceil() as usize).max(3);
        let mut pts = Vec::with_capacity(steps + 2);
        pts.push(center);
        for s in 0..=steps {
            let a = start_angle + sweep * (s as f32 / steps as f32);
            pts.push(egui::pos2(
                center.x + radius * a.cos(),
                center.y + radius * a.sin(),
            ));
        }
        let color = PIE_COLORS[idx % PIE_COLORS.len()];
        painter.add(egui::Shape::convex_polygon(
            pts,
            color,
            egui::Stroke::new(1.0, egui::Color32::BLACK),
        ));

        let mid = 0.5 * (start_angle + end_angle);
        let lbl_pos = egui::pos2(
            center.x + (radius + 18.0) * mid.cos(),
            center.y + (radius + 14.0) * mid.sin(),
        );
        painter.text(
            lbl_pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.0),
            egui::Color32::BLACK,
        );
        start_angle = end_angle;
    }
}
