//! Module 05 — Exergy Balance UI (HSC Exergy Balance Fig. 1 & Fig. 2 parity).

use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, text_cell, widget_cell, CELL_H, EXCEL_ALT_ROW_BG,
    EXCEL_GRID_BORDER, EXCEL_HEADER_BORDER, EXCEL_SECTION_BG,
};
use eframe::egui;
use soryntel_core::TemperatureUnit;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::exergy::{ExergySide, ExergyStreamKind, ExergyWorkbook};
use soryntel_thermo::Phase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExergyEnergyUnit {
    Kj,
    Mj,
    Kcal,
    Kwh,
}

impl ExergyEnergyUnit {
    fn factor_from_kj(self) -> f64 {
        match self {
            Self::Kj => 1.0,
            Self::Mj => 1.0e-3,
            Self::Kcal => 1.0 / 4.184,
            Self::Kwh => 1.0 / 3600.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Kj => "kJ",
            Self::Mj => "MJ",
            Self::Kcal => "kcal",
            Self::Kwh => "kWh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExergyAmountUnit {
    Mol,
    Kmol,
}

impl ExergyAmountUnit {
    fn label(self) -> &'static str {
        match self {
            Self::Mol => "mol",
            Self::Kmol => "kmol",
        }
    }

    fn from_mol(self, mol: f64) -> f64 {
        match self {
            Self::Mol => mol,
            Self::Kmol => mol / 1000.0,
        }
    }

    fn to_mol(self, val: f64) -> f64 {
        match self {
            Self::Mol => val,
            Self::Kmol => val * 1000.0,
        }
    }
}

pub struct ExergyModuleState {
    pub wb: ExergyWorkbook,
    pub insert_side: ExergySide,
    pub insert_kind: ExergyStreamKind,
    pub selected_stream: Option<usize>,
    pub selected_species: Option<(usize, usize)>,
    pub new_species_inputs: Vec<String>,
    pub temp_unit: TemperatureUnit,
    pub energy_unit: ExergyEnergyUnit,
    pub amount_unit: ExergyAmountUnit,
    pub show_db_picker: bool,
    pub db_search: String,
    pub db_gas_only: bool,
    pub show_help_modal: bool,
    pub file_path_input: String,
    pub status_msg: String,
}

impl ExergyModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        let mut wb = ExergyWorkbook::new_default();
        wb.calculate(db);
        let n = wb.streams.len();
        Self {
            wb,
            insert_side: ExergySide::Input,
            insert_kind: ExergyStreamKind::Normal,
            selected_stream: Some(0),
            selected_species: None,
            new_species_inputs: vec![String::new(); n],
            temp_unit: TemperatureUnit::Kelvin,
            energy_unit: ExergyEnergyUnit::Kj,
            amount_unit: ExergyAmountUnit::Mol,
            show_db_picker: false,
            db_search: String::new(),
            db_gas_only: false,
            show_help_modal: false,
            file_path_input: "Exergy_example.exe8".to_string(),
            status_msg: "Listo · Estado estándar 25 °C (298.15 K), 1 bar (Szargut 1988)"
                .to_string(),
        }
    }

    fn sync_input_buffers(&mut self) {
        self.new_species_inputs
            .resize(self.wb.streams.len(), String::new());
    }

    fn energy_factor(&self) -> f64 {
        self.energy_unit.factor_from_kj()
    }

    fn energy_unit_label(&self) -> &'static str {
        self.energy_unit.label()
    }

    fn fmt_temp(&self, k: f64) -> f64 {
        match self.temp_unit {
            TemperatureUnit::Kelvin => k,
            TemperatureUnit::Celsius => k - 273.15,
            TemperatureUnit::Fahrenheit => (k - 273.15) * 9.0 / 5.0 + 32.0,
        }
    }

    fn parse_temp_to_k(&self, display_val: f64) -> f64 {
        match self.temp_unit {
            TemperatureUnit::Kelvin => display_val.max(1.0),
            TemperatureUnit::Celsius => (display_val + 273.15).max(1.0),
            TemperatureUnit::Fahrenheit => ((display_val - 32.0) * 5.0 / 9.0 + 273.15).max(1.0),
        }
    }

    fn temp_unit_label(&self) -> &'static str {
        match self.temp_unit {
            TemperatureUnit::Kelvin => "K",
            TemperatureUnit::Celsius => "°C",
            TemperatureUnit::Fahrenheit => "°F",
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        self.sync_input_buffers();

        // Top Menu Bar: File | Edit | Units | Help
        egui::TopBottomPanel::top("exergy_menu_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(243, 244, 246))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(crate::i18n::text("common.001"), |ui| {
                        if ui.button("New (Fig. 1 Default)").clicked() {
                            self.wb = ExergyWorkbook::new_default();
                            self.wb.calculate(db);
                            self.selected_stream = Some(0);
                            self.selected_species = None;
                            self.sync_input_buffers();
                            self.status_msg = "Nuevo balance (plantilla Fig. 1)".to_string();
                            ui.close_menu();
                        }
                        if ui.button("Load Example (Fig. 2 CuS/FeS)").clicked() {
                            self.wb = ExergyWorkbook::new_fig2_example();
                            self.wb.calculate(db);
                            self.selected_stream = Some(0);
                            self.selected_species = None;
                            self.sync_input_buffers();
                            self.status_msg =
                                "Ejemplo Fig. 2 cargado (tostación/conversión CuS/FeS)".to_string();
                            ui.close_menu();
                        }
                        if ui.button("Load Example (Sulphides Roasting 15-col)").clicked() {
                            self.wb = ExergyWorkbook::new_sulphides_example();
                            self.wb.calculate(db);
                            self.selected_stream = Some(0);
                            self.selected_species = None;
                            self.sync_input_buffers();
                            self.status_msg =
                                "Ejemplo Sulphides Roasting (15 columnas) cargado".to_string();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(crate::i18n::text("common.064"));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.file_path_input)
                                    .desired_width(140.0),
                            );
                        });
                        if ui.button("Open (.exe8)...").clicked() {
                            match std::fs::read_to_string(&self.file_path_input)
                                .map_err(|e| e.to_string())
                                .and_then(|raw| ExergyWorkbook::from_exe8_str(&raw))
                            {
                                Ok(mut wb) => {
                                    wb.calculate(db);
                                    self.wb = wb;
                                    self.selected_stream = Some(0);
                                    self.selected_species = None;
                                    self.sync_input_buffers();
                                    self.status_msg =
                                        format!("Cargado '{}'", self.file_path_input);
                                }
                                Err(e) => {
                                    self.status_msg = format!("Error al abrir: {e}");
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Save (.exe8)...").clicked() {
                            match self.wb.to_exe8_json().and_then(|j| {
                                crate::storage::write_atomic(&self.file_path_input, j).map_err(|e| e.to_string())
                            }) {
                                Ok(()) => {
                                    self.status_msg =
                                        format!("Guardado '{}'", self.file_path_input);
                                }
                                Err(e) => {
                                    self.status_msg = format!("Error al guardar: {e}");
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(crate::i18n::text("common.016")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button(crate::i18n::text("common.006"), |ui| {
                        if ui.button(crate::i18n::text("common.011")).clicked() {
                            self.wb.calculate(db);
                            self.status_msg = "Balance de exergía recalculado".to_string();
                            ui.close_menu();
                        }
                        if ui.button("Insert stream").clicked() {
                            let idx = self.wb.insert_stream(self.insert_side, self.insert_kind);
                            self.selected_stream = Some(idx);
                            self.selected_species = None;
                            self.wb.calculate(db);
                            self.sync_input_buffers();
                            ui.close_menu();
                        }
                        if ui.button("Delete stream").clicked() {
                            if let Some(s_idx) = self.selected_stream {
                                self.wb.delete_stream(s_idx);
                                self.selected_stream = None;
                                self.selected_species = None;
                                self.wb.calculate(db);
                                self.sync_input_buffers();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Delete species").clicked() {
                            if let Some((s_idx, r_idx)) = self.selected_species {
                                self.wb.delete_species(s_idx, r_idx);
                                self.selected_species = None;
                                self.wb.calculate(db);
                            }
                            ui.close_menu();
                        }
                    });

                    ui.menu_button(crate::i18n::text("common.027"), |ui| {
                        ui.label(egui::RichText::new(crate::i18n::text("common.005")).strong().small());
                        ui.radio_value(&mut self.temp_unit, TemperatureUnit::Kelvin, "K (Kelvin)");
                        ui.radio_value(
                            &mut self.temp_unit,
                            TemperatureUnit::Celsius,
                            "°C (Celsius)",
                        );
                        ui.separator();
                        ui.label(egui::RichText::new("Energy").strong().small());
                        ui.radio_value(&mut self.energy_unit, ExergyEnergyUnit::Kj, "kJ");
                        ui.radio_value(&mut self.energy_unit, ExergyEnergyUnit::Mj, "MJ");
                        ui.radio_value(&mut self.energy_unit, ExergyEnergyUnit::Kcal, "kcal");
                        ui.radio_value(&mut self.energy_unit, ExergyEnergyUnit::Kwh, "kWh");
                        ui.separator();
                        ui.label(egui::RichText::new(crate::i18n::text("common.017")).strong().small());
                        ui.radio_value(&mut self.amount_unit, ExergyAmountUnit::Mol, "mol");
                        ui.radio_value(&mut self.amount_unit, ExergyAmountUnit::Kmol, "kmol");
                    });

                    ui.menu_button(crate::i18n::text("common.003"), |ui| {
                        if ui.button("Methodology (Szargut 1988)").clicked() {
                            self.show_help_modal = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // Top Control & Global Summary Band (Fig. 1 & Fig. 2)
        egui::TopBottomPanel::top("exergy_toolbar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 237, 240))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Left block: 2 rows of controls
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([118.0, 24.0], egui::Button::new("Browse database"))
                                .clicked()
                            {
                                let is_gas = self
                                    .selected_stream
                                    .and_then(|i| self.wb.streams.get(i))
                                    .map(|s| s.kind == ExergyStreamKind::Gas)
                                    .unwrap_or(false);
                                self.db_gas_only = is_gas;
                                self.show_db_picker = true;
                            }
                            if ui
                                .add_sized([96.0, 24.0], egui::Button::new("Insert stream"))
                                .clicked()
                            {
                                let idx =
                                    self.wb.insert_stream(self.insert_side, self.insert_kind);
                                self.selected_stream = Some(idx);
                                self.selected_species = None;
                                self.wb.calculate(db);
                                self.sync_input_buffers();
                            }
                            egui::ComboBox::from_id_salt("exergy_side_combo")
                                .width(76.0)
                                .selected_text(match self.insert_side {
                                    ExergySide::Input => "Input",
                                    ExergySide::Output => "Output",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.insert_side,
                                        ExergySide::Input,
                                        "Input",
                                    );
                                    ui.selectable_value(
                                        &mut self.insert_side,
                                        ExergySide::Output,
                                        "Output",
                                    );
                                });
                            egui::ComboBox::from_id_salt("exergy_kind_combo")
                                .width(118.0)
                                .selected_text(match self.insert_kind {
                                    ExergyStreamKind::Normal => "Normal stream",
                                    ExergyStreamKind::Gas => "Gas stream",
                                    ExergyStreamKind::Energy => "Energy stream",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.insert_kind,
                                        ExergyStreamKind::Normal,
                                        "Normal stream",
                                    );
                                    ui.selectable_value(
                                        &mut self.insert_kind,
                                        ExergyStreamKind::Gas,
                                        "Gas stream",
                                    );
                                    ui.selectable_value(
                                        &mut self.insert_kind,
                                        ExergyStreamKind::Energy,
                                        "Energy stream",
                                    );
                                });
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([118.0, 24.0], egui::Button::new("Delete stream"))
                                .clicked()
                            {
                                if let Some(s_idx) = self.selected_stream {
                                    self.wb.delete_stream(s_idx);
                                    self.selected_stream = None;
                                    self.selected_species = None;
                                    self.wb.calculate(db);
                                    self.sync_input_buffers();
                                }
                            }
                            if ui
                                .add_sized([118.0, 24.0], egui::Button::new("Delete species"))
                                .clicked()
                            {
                                if let Some((s_idx, r_idx)) = self.selected_species {
                                    self.wb.delete_species(s_idx, r_idx);
                                    self.selected_species = None;
                                    self.wb.calculate(db);
                                }
                            }
                            if ui
                                .add_sized([172.0, 24.0], egui::Button::new("Calculate"))
                                .clicked()
                            {
                                self.wb.calculate(db);
                                self.status_msg = "Cálculo actualizado".to_string();
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Center block: Global balance indicators (Mass, Heat, Exergy)
                    let ef = self.energy_factor();
                    let eu = self.energy_unit_label();
                    let draw_balance_box =
                        |ui: &mut egui::Ui, title: &str, value: f64| {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(244, 245, 247))
                                .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                .show(ui, |ui| {
                                    ui.set_width(126.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(title)
                                                .strong()
                                                .size(11.5)
                                                .color(egui::Color32::from_rgb(24, 24, 27)),
                                        );
                                        ui.add_space(4.0);
                                        egui::Frame::none()
                                            .fill(egui::Color32::WHITE)
                                            .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                                            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.set_width(110.0);
                                                ui.vertical_centered(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!("{value:.2}"))
                                                            .monospace()
                                                            .size(12.5)
                                                            .color(egui::Color32::from_rgb(
                                                                17, 24, 39,
                                                            )),
                                                    );
                                                });
                                            });
                                    });
                                });
                        };

                    draw_balance_box(ui, "Mass Balance (kg)", self.wb.mass_balance_kg);
                    draw_balance_box(
                        ui,
                        &format!("Heat Balance ({eu})"),
                        self.wb.heat_balance_kj * ef,
                    );
                    draw_balance_box(
                        ui,
                        &format!("Exergy Balance ({eu})"),
                        self.wb.exergy_balance_kj * ef,
                    );

                    ui.add_space(10.0);

                    // Right block: Methodology note box
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(248, 249, 250))
                        .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.set_min_width(240.0);
                            ui.label(
                                egui::RichText::new(
                                    "All exergy calculations are based on Szargut theory with the standard state being 25°C and 1 bar.\nNormal stream accepts any species. Gas stream accepts gas species and adds pressure S to exergy.\nEnergy stream adds heat and electricity feed/loss. Carnot cycle is used to convert heat to exergy.",
                                )
                                .size(10.5)
                                .color(egui::Color32::from_rgb(31, 41, 55)),
                            );
                        });
                });
            });

        // Bottom Sheet Tab + Status strip
        egui::TopBottomPanel::bottom("exergy_bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 237, 240))
                    .stroke(egui::Stroke::new(1.0, EXCEL_HEADER_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
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
                            ui.label(
                                egui::RichText::new("Calculations")
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(17, 24, 39)),
                            );
                        });
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(&self.status_msg)
                            .small()
                            .color(egui::Color32::from_rgb(75, 85, 99)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Soryntel Exergy Engine")
                                .strong()
                                .size(11.5)
                                .color(egui::Color32::from_rgb(185, 28, 28)),
                        );
                    });
                });
            });

        // Main Spreadsheet Area (`Calculations`)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                self.render_spreadsheet(ui, db);
            });

        if self.show_db_picker {
            self.render_db_picker(ctx, db);
        }

        if self.show_help_modal {
            egui::Window::new("Methodology — Szargut (1988)")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("• Estado muerto estándar: T₀ = 298.15 K (25 °C), P₀ = 1.0 bar.");
                    ui.label("• Exergía química estándar: ex_ch° = ΔfG°(T₀) + Σ ν_e · b_ch,e° (Szargut 1988).");
                    ui.label("• Exergía física: Ex_phy = (H(T) - H(T₀)) - T₀·(S(T,P) - S(T₀,P₀)).");
                    ui.label("• Mezcla ideal en Ex_tot_stream: + R·T₀·Σ n_i·ln(x_i).");
                    ui.label("• Energy stream (Carnot): Ex = Q·(1 - T₀/T) + W.");
                    ui.add_space(8.0);
                    if ui.button("Cerrar").clicked() {
                        self.show_help_modal = false;
                    }
                });
        }
    }

    fn render_spreadsheet(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let col_widths = [
            165.0, // A: Streams and species
            66.0,  // B: Mass (kg)
            74.0,  // C: Amount (mol)
            76.0,  // D: Molfraction
            68.0,  // E: T (K)
            58.0,  // F: P (bar)
            74.0,  // G: S (J/molK)
            74.0,  // H: H (kJ/mol)
            84.0,  // I: H Total (kJ)
            84.0,  // J: Ex_elemental (kJ/mol)
            82.0,  // K: Ex_che (kJ/mol)
            82.0,  // L: Ex_phy (kJ/mol)
            82.0,  // M: Ex_tot (kJ/mol)
            94.0,  // N: Ex_tot_stream (kJ/mol)
            94.0,  // O: Ex_tot_stream (kJ)
        ];
        let row_num_w = 30.0;
        let eu = self.energy_unit_label();
        let ef = self.energy_factor();
        let tu = self.temp_unit_label();
        let au = self.amount_unit.label();

        let mut recalc_needed = false;
        let mut pending_add: Option<(usize, String)> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // Excel Column Letter Row (A .. O)
                ui.horizontal(|ui| {
                    header_cell(ui, "", row_num_w);
                    for (idx, w) in col_widths.iter().enumerate() {
                        header_cell(ui, &col_letter(idx), *w);
                    }
                });

                // Row 1: Column Titles
                ui.horizontal(|ui| {
                    row_num_cell(ui, 1, row_num_w);
                    let titles = [
                        "Streams and",
                        "Mass",
                        "Amount",
                        "Molfraction",
                        "T",
                        "P",
                        "S",
                        "H",
                        "H Total",
                        "Ex_elemental",
                        "Ex_che",
                        "Ex_phy",
                        "Ex_tot",
                        "Ex_tot_stream",
                        "Ex_tot_stream",
                    ];
                    for (idx, t) in titles.iter().enumerate() {
                        text_cell(
                            ui,
                            t,
                            col_widths[idx],
                            EXCEL_SECTION_BG,
                            egui::Color32::from_rgb(24, 24, 27),
                            false,
                            true,
                            false,
                        );
                    }
                });

                // Row 2: Column Units
                ui.horizontal(|ui| {
                    row_num_cell(ui, 2, row_num_w);
                    let units = [
                        "species".to_string(),
                        "(kg)".to_string(),
                        format!("({au})"),
                        "(fraction)".to_string(),
                        format!("({tu})"),
                        "(bar)".to_string(),
                        format!("(J/{au}K)"),
                        format!("({eu}/{au})"),
                        format!("({eu})"),
                        format!("({eu}/{au})"),
                        format!("({eu}/{au})"),
                        format!("({eu}/{au})"),
                        format!("({eu}/{au})"),
                        format!("({eu}/{au})"),
                        format!("({eu})"),
                    ];
                    for (idx, u) in units.iter().enumerate() {
                        text_cell(
                            ui,
                            u,
                            col_widths[idx],
                            EXCEL_SECTION_BG,
                            egui::Color32::from_rgb(24, 24, 27),
                            false,
                            true,
                            false,
                        );
                    }
                });

                let mut excel_row = 3usize;
                let mut prev_side: Option<ExergySide> = None;
                let stream_count = self.wb.streams.len();

                for s_idx in 0..stream_count {
                    let side = self.wb.streams[s_idx].side;
                    if prev_side == Some(ExergySide::Input) && side == ExergySide::Output {
                        // Subtle light-slate Input/Output separator row
                        ui.horizontal(|ui| {
                            row_num_cell(ui, excel_row, row_num_w);
                            for w in &col_widths {
                                text_cell(
                                    ui,
                                    "",
                                    *w,
                                    egui::Color32::from_rgb(203, 213, 225),
                                    egui::Color32::from_rgb(24, 24, 27),
                                    false,
                                    false,
                                    false,
                                );
                            }
                        });
                        excel_row += 1;
                    }
                    prev_side = Some(side);

                    let stream_kind = self.wb.streams[s_idx].kind;
                    let is_stream_selected = self.selected_stream == Some(s_idx);
                    let header_color = match stream_kind {
                        ExergyStreamKind::Normal => egui::Color32::from_rgb(17, 24, 39),
                        ExergyStreamKind::Gas => egui::Color32::from_rgb(185, 28, 28),
                        ExergyStreamKind::Energy => egui::Color32::from_rgb(21, 128, 61),
                    };
                    let header_bg = egui::Color32::from_rgb(228, 226, 218);

                    // Stream Header Row
                    ui.horizontal(|ui| {
                        row_num_cell(ui, excel_row, row_num_w);
                        if is_stream_selected && self.selected_species.is_none() {
                            widget_cell(ui, col_widths[0], header_bg, true, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.wb.streams[s_idx].name)
                                        .text_color(header_color)
                                        .desired_width(col_widths[0] - 8.0),
                                );
                            });
                        } else {
                            let name = self.wb.streams[s_idx].name.clone();
                            if text_cell(
                                ui,
                                &name,
                                col_widths[0],
                                header_bg,
                                header_color,
                                false,
                                true,
                                false,
                            )
                            .clicked()
                            {
                                self.selected_stream = Some(s_idx);
                                self.selected_species = None;
                            }
                        }

                        if stream_kind == ExergyStreamKind::Energy {
                            for idx in 1..8 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    header_bg,
                                    header_color,
                                    true,
                                    true,
                                    false,
                                );
                            }
                            let h_tot = self.wb.streams[s_idx].h_total_kj * ef;
                            text_cell(
                                ui,
                                &format!("{h_tot:.2}"),
                                col_widths[8],
                                header_bg,
                                header_color,
                                true,
                                true,
                                false,
                            );
                            for idx in 9..14 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    header_bg,
                                    header_color,
                                    true,
                                    true,
                                    false,
                                );
                            }
                            let ex_s = self.wb.streams[s_idx].ex_tot_stream_kj * ef;
                            text_cell(
                                ui,
                                &format!("{ex_s:.2}"),
                                col_widths[14],
                                header_bg,
                                header_color,
                                true,
                                true,
                                false,
                            );
                        } else {
                            let s_mass = self.wb.streams[s_idx].mass_kg;
                            let s_amt = self.amount_unit.from_mol(self.wb.streams[s_idx].amount_mol);
                            let s_mf = self.wb.streams[s_idx].mol_fraction;
                            text_cell(
                                ui,
                                &format!("{s_mass:.2}"),
                                col_widths[1],
                                header_bg,
                                header_color,
                                true,
                                true,
                                false,
                            );
                            text_cell(
                                ui,
                                &format!("{s_amt:.2}"),
                                col_widths[2],
                                header_bg,
                                header_color,
                                true,
                                true,
                                false,
                            );
                            text_cell(
                                ui,
                                &format!("{s_mf:.2}"),
                                col_widths[3],
                                header_bg,
                                header_color,
                                true,
                                true,
                                false,
                            );

                            // Editable T (Col E)
                            let mut t_disp = self.fmt_temp(self.wb.streams[s_idx].temp_k);
                            widget_cell(ui, col_widths[4], egui::Color32::WHITE, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut t_disp)
                                            .speed(1.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    self.wb.streams[s_idx].temp_k = self.parse_temp_to_k(t_disp);
                                    recalc_needed = true;
                                }
                            });

                            // Editable P (Col F)
                            let mut p_val = self.wb.streams[s_idx].pressure_bar;
                            widget_cell(ui, col_widths[5], egui::Color32::WHITE, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut p_val)
                                            .speed(0.05)
                                            .range(0.001..=10000.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    self.wb.streams[s_idx].pressure_bar = p_val;
                                    recalc_needed = true;
                                }
                            });

                            let st = &self.wb.streams[s_idx];
                            let m_scale = ef / self.amount_unit.from_mol(1.0);
                            let vals = [
                                st.s_j_molk_sum / self.amount_unit.from_mol(1.0), // G: S
                                st.h_kj_mol_sum * m_scale,                        // H: H (kJ/mol)
                                st.h_total_kj * ef,                               // I: H Total (kJ)
                                st.ex_elemental_kj_mol * m_scale,                 // J: Ex_elemental
                                st.ex_che_kj_mol_sum * m_scale,                   // K: Ex_che (kJ/mol)
                                st.ex_phy_kj_mol_sum * m_scale,                   // L: Ex_phy (kJ/mol)
                                st.ex_tot_kj_mol_sum * m_scale,                   // M: Ex_tot (kJ/mol)
                                st.ex_tot_stream_kj_mol * m_scale,                // N: Ex_tot_stream (kJ/mol)
                                st.ex_tot_stream_kj * ef,                         // O: Ex_tot_stream (kJ)
                            ];
                            for (offset, val) in vals.iter().enumerate() {
                                text_cell(
                                    ui,
                                    &format!("{val:.2}"),
                                    col_widths[6 + offset],
                                    header_bg,
                                    header_color,
                                    true,
                                    true,
                                    false,
                                );
                            }
                        }
                    });
                    excel_row += 1;

                    if stream_kind == ExergyStreamKind::Energy {
                        let (heat_lbl, elec_lbl) = if side == ExergySide::Input {
                            ("Heat feed", "Electricity feed")
                        } else {
                            ("Heat loss", "Electricity out")
                        };

                        // Heat row (Col E = T, Col I = H Total)
                        ui.horizontal(|ui| {
                            row_num_cell(ui, excel_row, row_num_w);
                            text_cell(
                                ui,
                                heat_lbl,
                                col_widths[0],
                                EXCEL_ALT_ROW_BG,
                                egui::Color32::from_rgb(24, 24, 27),
                                false,
                                false,
                                false,
                            );
                            for idx in 1..4 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            let mut ht_disp = self.fmt_temp(self.wb.streams[s_idx].heat_temp_k);
                            widget_cell(ui, col_widths[4], egui::Color32::WHITE, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut ht_disp)
                                            .speed(1.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    self.wb.streams[s_idx].heat_temp_k =
                                        self.parse_temp_to_k(ht_disp);
                                    recalc_needed = true;
                                }
                            });
                            for idx in 5..8 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            let mut h_disp = self.wb.streams[s_idx].heat_kj * ef;
                            widget_cell(ui, col_widths[8], egui::Color32::WHITE, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut h_disp)
                                            .speed(10.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    self.wb.streams[s_idx].heat_kj = h_disp / ef;
                                    recalc_needed = true;
                                }
                            });
                            for idx in 9..15 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                        });
                        excel_row += 1;

                        // Electricity row (Col I = H Total)
                        ui.horizontal(|ui| {
                            row_num_cell(ui, excel_row, row_num_w);
                            text_cell(
                                ui,
                                elec_lbl,
                                col_widths[0],
                                EXCEL_ALT_ROW_BG,
                                egui::Color32::from_rgb(24, 24, 27),
                                false,
                                false,
                                false,
                            );
                            for idx in 1..8 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                            let mut w_disp = self.wb.streams[s_idx].electricity_kj * ef;
                            widget_cell(ui, col_widths[8], egui::Color32::WHITE, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut w_disp)
                                            .speed(10.0)
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                {
                                    self.wb.streams[s_idx].electricity_kj = w_disp / ef;
                                    recalc_needed = true;
                                }
                            });
                            for idx in 9..15 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    true,
                                    false,
                                    false,
                                );
                            }
                        });
                        excel_row += 1;
                    } else {
                        // Species rows
                        let sp_len = self.wb.streams[s_idx].species.len();
                        for r_idx in 0..sp_len {
                            let is_sp_sel = self.selected_species == Some((s_idx, r_idx));
                            let missing = self.wb.streams[s_idx].species[r_idx].missing_in_db;
                            let warn = self.wb.streams[s_idx].species[r_idx].phase_warning.is_some();
                            let name_color = if missing {
                                egui::Color32::from_rgb(185, 28, 28)
                            } else if warn {
                                egui::Color32::from_rgb(180, 83, 9)
                            } else {
                                egui::Color32::from_rgb(17, 24, 39)
                            };

                            ui.horizontal(|ui| {
                                row_num_cell(ui, excel_row, row_num_w);
                                let formula =
                                    self.wb.streams[s_idx].species[r_idx].formula.clone();
                                if text_cell(
                                    ui,
                                    &formula,
                                    col_widths[0],
                                    EXCEL_ALT_ROW_BG,
                                    name_color,
                                    false,
                                    false,
                                    is_sp_sel,
                                )
                                .clicked()
                                {
                                    self.selected_stream = Some(s_idx);
                                    self.selected_species = Some((s_idx, r_idx));
                                }

                                // Editable Mass (Col B, Section 15.2)
                                let mut mass_kg = self.wb.streams[s_idx].species[r_idx].mass_kg;
                                widget_cell(ui, col_widths[1], egui::Color32::WHITE, false, |ui| {
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut mass_kg)
                                                .speed(0.01)
                                                .range(0.0..=1.0e9)
                                                .fixed_decimals(3),
                                        )
                                        .changed()
                                    {
                                        self.wb.set_species_mass_kg(s_idx, r_idx, mass_kg, db);
                                        recalc_needed = true;
                                    }
                                });

                                // Editable Amount (Col C)
                                let mut amt_disp = self
                                    .amount_unit
                                    .from_mol(self.wb.streams[s_idx].species[r_idx].amount_mol);
                                widget_cell(ui, col_widths[2], egui::Color32::WHITE, false, |ui| {
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut amt_disp)
                                                .speed(1.0)
                                                .range(0.0..=1.0e9)
                                                .fixed_decimals(2),
                                        )
                                        .changed()
                                    {
                                        self.wb.streams[s_idx].species[r_idx].amount_mol =
                                            self.amount_unit.to_mol(amt_disp);
                                        recalc_needed = true;
                                    }
                                });

                                let row = &self.wb.streams[s_idx].species[r_idx];
                                let mf = row.mol_fraction;
                                let t_d = self.fmt_temp(row.temp_k);
                                let p_d = row.pressure_bar;
                                let m_scale = ef / self.amount_unit.from_mol(1.0);
                                let cells = [
                                    format!("{mf:.2}"),
                                    format!("{t_d:.2}"),
                                    format!("{p_d:.2}"),
                                    format!(
                                        "{:.2}",
                                        row.s_j_molk / self.amount_unit.from_mol(1.0)
                                    ),
                                    format!("{:.2}", row.h_kj_mol * m_scale),
                                    format!("{:.2}", row.h_total_kj * ef),
                                    format!("{:.2}", row.ex_elemental_kj_mol * m_scale),
                                    format!("{:.2}", row.ex_che_kj_mol * m_scale),
                                    format!("{:.2}", row.ex_phy_kj_mol * m_scale),
                                    format!("{:.2}", row.ex_tot_kj_mol * m_scale),
                                ];
                                for (offset, val_txt) in cells.iter().enumerate() {
                                    text_cell(
                                        ui,
                                        val_txt,
                                        col_widths[3 + offset],
                                        EXCEL_ALT_ROW_BG,
                                        egui::Color32::from_rgb(17, 24, 39),
                                        true,
                                        false,
                                        false,
                                    );
                                }
                                for idx in 13..15 {
                                    text_cell(
                                        ui,
                                        "",
                                        col_widths[idx],
                                        EXCEL_ALT_ROW_BG,
                                        egui::Color32::BLACK,
                                        true,
                                        false,
                                        false,
                                    );
                                }
                            });
                            excel_row += 1;
                        }

                        // "Insert/paste species here" row
                        ui.horizontal(|ui| {
                            row_num_cell(ui, excel_row, row_num_w);
                            widget_cell(
                                ui,
                                col_widths[0],
                                EXCEL_ALT_ROW_BG,
                                false,
                                |ui| {
                                    let buf = &mut self.new_species_inputs[s_idx];
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(buf)
                                            .hint_text("Insert/paste species here")
                                            .desired_width(col_widths[0] - 10.0),
                                    );
                                    if resp.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        && !buf.trim().is_empty()
                                    {
                                        pending_add = Some((s_idx, buf.trim().to_string()));
                                        buf.clear();
                                    }
                                },
                            );
                            for idx in 1..15 {
                                text_cell(
                                    ui,
                                    "",
                                    col_widths[idx],
                                    EXCEL_ALT_ROW_BG,
                                    egui::Color32::BLACK,
                                    false,
                                    false,
                                    false,
                                );
                            }
                        });
                        excel_row += 1;
                    }
                }

                // Trailing empty grid rows for clean white Excel sheet look (up to row 42)
                while excel_row <= 42 {
                    ui.horizontal(|ui| {
                        row_num_cell(ui, excel_row, row_num_w);
                        for w in &col_widths {
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(*w, CELL_H), egui::Sense::hover());
                            if ui.is_rect_visible(rect) {
                                let p = ui.painter();
                                p.rect_filled(rect, 0.0, egui::Color32::WHITE);
                                p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));
                            }
                        }
                    });
                    excel_row += 1;
                }
            });

        if let Some((s_idx, formula)) = pending_add {
            match self.wb.add_species_list_to_stream(s_idx, &formula, 1.0) {
                Ok(n) => {
                    self.status_msg = if n == 1 {
                        format!("Especie '{formula}' agregada")
                    } else {
                        format!("{n} especies agregadas")
                    };
                    recalc_needed = true;
                }
                Err(e) => {
                    self.status_msg = e;
                }
            }
        }
        if recalc_needed {
            self.wb.calculate(db);
        }
    }

    fn render_db_picker(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_db_picker;
        let mut insert_formula: Option<String> = None;

        egui::Window::new("Browse Database — Insert Species into Exergy Stream")
            .open(&mut open)
            .default_size([520.0, 420.0])
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Buscar fórmula:");
                    ui.text_edit_singleline(&mut self.db_search);
                    ui.checkbox(&mut self.db_gas_only, "Solo gases");
                });
                ui.separator();

                let matches: Vec<_> = db
                    .search(self.db_search.trim(), 200)
                    .into_iter()
                    .filter(|sp| !self.db_gas_only || sp.phase == Phase::Gas)
                    .take(150)
                    .collect();

                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
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
                                .color(egui::Color32::from_rgb(100, 116, 139)),
                            );
                        });
                    }
                });
            });

        self.show_db_picker = open;
        if let Some(formula) = insert_formula {
            let target = self.selected_stream.unwrap_or(0);
            if let Err(e) = self.wb.add_species_to_stream(target, &formula, 1.0) {
                self.status_msg = e;
            } else {
                self.wb.calculate(db);
                self.status_msg = format!("Insertado '{formula}' desde Maindb6");
            }
        }
    }
}
