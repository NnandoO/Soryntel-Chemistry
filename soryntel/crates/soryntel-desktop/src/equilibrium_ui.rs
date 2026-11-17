//! Module 4 Desktop UI: Equilibrium Calculations (Gibbs Energy Minimization).
//! Matches HSC Equilibrium Screenshots (Figs. 1–46) using shared `excel_grid` helpers.

use crate::excel_grid::{
    self, EXCEL_READONLY_BG, EXCEL_SECTION_BG,
};

const COL_W: f32 = 98.0;
const ROW_W: f32 = 34.0;

fn col_letter(ui: &mut egui::Ui, label: &str) {
    excel_grid::header_cell(ui, label, COL_W);
}
fn header_cell(ui: &mut egui::Ui, label: &str) {
    excel_grid::header_cell(ui, label, COL_W);
}
fn row_num_cell(ui: &mut egui::Ui, label: &str) {
    excel_grid::header_cell(ui, label, ROW_W);
}
fn section_cell(ui: &mut egui::Ui, label: &str) {
    excel_grid::section_cell(ui, label, COL_W, false);
}
fn text_cell(ui: &mut egui::Ui, label: &str, right_align: bool, shaded: bool) {
    let bg = if shaded { EXCEL_READONLY_BG } else { egui::Color32::WHITE };
    excel_grid::text_cell(
        ui,
        label,
        COL_W,
        bg,
        egui::Color32::from_rgb(24, 24, 27),
        right_align,
        false,
        false,
    );
}
fn widget_cell(ui: &mut egui::Ui, shaded: bool, add_contents: impl FnOnce(&mut egui::Ui)) -> bool {
    let bg = if shaded { EXCEL_SECTION_BG } else { egui::Color32::WHITE };
    excel_grid::widget_cell(ui, COL_W, bg, false, add_contents);
    false
}
fn col_name(idx: usize) -> String {
    excel_grid::col_letter(idx)
}
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use soryntel_db::ThermoDatabase;
use soryntel_thermo::equilibrium::{
    collect_cp_warnings, solve_equilibrium_sweep, AxisRange, CalculationMode,
    EquilibriumAmountUnit, EquilibriumParams, EquilibriumPhase, EquilibriumSpeciesInput,
    EquilibriumSweepResult, EquilibriumSystem, EquilibriumWorkbookFile, ExtrapolationWarning,
    SolutionModel, StepMode, SweepVariable, TargetSpec, ThermodynamicMode,
};
use soryntel_thermo::{Phase, SpeciesRepository};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EquilSheetTab {
    Species,
    UserFunction,
    Results1,
    TargetResults,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartXKind {
    StepVariable,
    StepIndex,
    TemperatureC,
    TemperatureK,
    PressureBar,
    VolumeM3,
    ReactionEnthalpyMj,
    Ph,
    DischargeKmolE,
    SpeciesEquilAmount(usize),
}

impl ChartXKind {
    fn label(self, primary_label: &str, species_name: Option<&str>) -> String {
        match self {
            Self::StepVariable => primary_label.to_string(),
            Self::StepIndex => "Step".to_string(),
            Self::TemperatureC => "Temperature (°C)".to_string(),
            Self::TemperatureK => "Temperature (K)".to_string(),
            Self::PressureBar => "Pressure (bar)".to_string(),
            Self::VolumeM3 => "Volume (m3)".to_string(),
            Self::ReactionEnthalpyMj => "Reaction Enthalpy (MJ)".to_string(),
            Self::Ph => "pH".to_string(),
            Self::DischargeKmolE => "Discharge level (kmol e-)".to_string(),
            Self::SpeciesEquilAmount(_) => {
                format!("{} Equilibrium Amount (kmol)", species_name.unwrap_or("Species"))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartYKind {
    EquilibriumAmount,
    MoleFraction,
    Activity,
    ActivityCoeff,
    ReactionEnthalpy,
    CellVoltage,
    Ph,
    UserFormula,
}

impl ChartYKind {
    fn label(self) -> &'static str {
        match self {
            Self::EquilibriumAmount => "Equilibrium Amount (kmol)",
            Self::MoleFraction => "Equilibrium Composition (mol fract)",
            Self::Activity => "Activity",
            Self::ActivityCoeff => "Activity Coefficient",
            Self::ReactionEnthalpy => "Reaction Enthalpy (MJ)",
            Self::CellVoltage => "Cell Voltage (V)",
            Self::Ph => "pH",
            Self::UserFormula => "User Formula (kmol)",
        }
    }
}

#[derive(Debug, Clone)]
struct UserFormulaRow {
    label: String,
    expr: String, // e.g. "=D10+D28" or "CO(g)+Ni"
    show_formula: bool,
    show_diff: bool,
    show_cumul: bool,
}

pub struct EquilibriumUiState {
    pub system: EquilibriumSystem,
    pub params: EquilibriumParams,
    pub result: Option<EquilibriumSweepResult>,
    pub last_error: Option<String>,
    active_sheet: EquilSheetTab,
    active_cell_ref: String,
    active_cell_text: String,
    selected_phase_idx: usize,
    file_path_input: String,
    file_status_msg: Option<String>,
    // Modals
    show_from_elements: bool,
    elem_query: String,
    elem_gas: bool,
    elem_liquid: bool,
    elem_solid: bool,
    elem_aq_ion: bool,
    elem_aq_neutral: bool,
    elem_add_n2: bool,
    elem_sort_mode: usize, // 0=Phases, 1=Gas/Aqua/Pure, 2=No sorting, 3=Gas/Aqua/Liquids
    add_species_query: String,
    show_warnings_modal: bool,
    dont_show_warnings: bool,
    warnings_pending_calc: bool,
    warnings: Vec<ExtrapolationWarning>,
    show_addins_modal: bool,
    show_chart_window: bool,
    show_axes_modal: bool,
    show_diagram_table: bool,
    diagram_table_kind: ChartYKind,
    // Chart filtering, axis ranges & 3D
    chart_x_kind: ChartXKind,
    chart_y_kind: ChartYKind,
    chart_auto_axes: bool,
    chart_x_min: f64,
    chart_x_max: f64,
    chart_y_min: f64,
    chart_y_max: f64,
    chart_log_y: bool,
    chart_show_curve_labels: bool,
    chart_smooth: bool,
    chart_hidden_species: BTreeSet<String>,
    chart_phase_filter: Option<usize>,
    chart_elem_filter: String,
    chart_elem_contains_all: bool,
    chart_is_3d: bool,
    chart_3d_yaw: f32,
    chart_3d_pitch: f32,
    chart_color_by_levels: bool,
    chart_3d_show_legend: bool,
    chart_3d_cache: Option<Cached3dSurface>,
    // User formulas
    user_formulas: Vec<UserFormulaRow>,
    // Preset selector
    initialized_default: bool,
}

struct Cached3dSurface {
    key: u64,
    tex: egui::TextureHandle,
}

impl Default for EquilibriumUiState {
    fn default() -> Self {
        let mut s = Self {
            system: EquilibriumSystem {
                title: "NICO".to_string(),
                phases: Vec::new(),
            },
            params: EquilibriumParams::default(),
            result: None,
            last_error: None,
            active_sheet: EquilSheetTab::Species,
            active_cell_ref: "A1".to_string(),
            active_cell_text: String::new(),
            selected_phase_idx: 0,
            file_path_input: "NICO.gem9".to_string(),
            file_status_msg: None,
            show_from_elements: false,
            elem_query: "Ni C O".to_string(),
            elem_gas: true,
            elem_liquid: true,
            elem_solid: true,
            elem_aq_ion: false,
            elem_aq_neutral: false,
            elem_add_n2: false,
            elem_sort_mode: 0,
            add_species_query: String::new(),
            show_warnings_modal: false,
            dont_show_warnings: false,
            warnings_pending_calc: false,
            warnings: Vec::new(),
            show_addins_modal: false,
            show_chart_window: false,
            show_axes_modal: false,
            show_diagram_table: false,
            diagram_table_kind: ChartYKind::EquilibriumAmount,
            chart_x_kind: ChartXKind::StepVariable,
            chart_y_kind: ChartYKind::EquilibriumAmount,
            chart_auto_axes: true,
            chart_x_min: 0.0,
            chart_x_max: 300.0,
            chart_y_min: 0.0,
            chart_y_max: 4.0,
            chart_log_y: false,
            chart_show_curve_labels: true,
            chart_smooth: true,
            chart_hidden_species: BTreeSet::new(),
            chart_phase_filter: None,
            chart_elem_filter: "All elements".to_string(),
            chart_elem_contains_all: false,
            chart_is_3d: false,
            chart_3d_yaw: -0.52,
            chart_3d_pitch: 0.42,
            chart_color_by_levels: false,
            chart_3d_show_legend: true,
            chart_3d_cache: None,
            user_formulas: vec![UserFormulaRow {
                label: "CO(g) + Ni".to_string(),
                expr: "=D5+D10".to_string(),
                show_formula: true,
                show_diff: false,
                show_cumul: false,
            }],
            initialized_default: false,
        };
        s.load_preset_nico();
        s
    }
}

impl EquilibriumUiState {
    fn make_sp(formula: &str, init_amt: f64) -> EquilibriumSpeciesInput {
        EquilibriumSpeciesInput {
            formula: formula.to_string(),
            feed_temp_c: 25.0,
            activity_coeff_expr: "1.000".to_string(),
            fixed_activity: None,
            discharge_coeff: 0.0,
            amount_unit: EquilibriumAmountUnit::Kmol,
            initial_amount: init_amt,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        }
    }

    fn reset_chart_bounds_from_result(&mut self) {
        let Some(ref res) = self.result else {
            return;
        };
        let Some(first) = res.points.first() else {
            return;
        };
        let scale = self.params.input_scale_to_mol.max(1e-12);
        let mut xmin = f64::INFINITY;
        let mut xmax = f64::NEG_INFINITY;
        let mut ymin = 0.0f64;
        let mut ymax = 1e-6f64;
        for p in &res.points {
            let xv = self.point_x_value(p, scale);
            xmin = xmin.min(xv);
            xmax = xmax.max(xv);
            if self.chart_y_kind == ChartYKind::CellVoltage {
                let v = p.cell_voltage_v.unwrap_or(0.0);
                ymin = ymin.min(v);
                ymax = ymax.max(v);
            } else {
                for (s_idx, sp0) in first.species.iter().enumerate() {
                    if species_matches_filters(
                        sp0,
                        &self.chart_hidden_species,
                        self.chart_phase_filter,
                        &self.chart_elem_filter,
                        self.chart_elem_contains_all,
                    ) {
                        let yv = self.point_y_value(p, s_idx, scale);
                        if yv.is_finite() {
                            ymin = ymin.min(yv);
                            ymax = ymax.max(yv);
                        }
                    }
                }
            }
        }
        if xmin.is_finite() && xmax.is_finite() {
            self.chart_x_min = xmin;
            self.chart_x_max = if (xmax - xmin).abs() < 1e-9 {
                xmin + 1.0
            } else {
                xmax
            };
        }
        if ymin.is_finite() && ymax.is_finite() {
            self.chart_y_min = ymin;
            self.chart_y_max = if (ymax - ymin).abs() < 1e-9 {
                ymin + 1.0
            } else {
                ymax * 1.05
            };
        }
    }

    fn load_empty_workbook(&mut self, cell_mode: bool) {
        self.system = EquilibriumSystem {
            title: if cell_mode {
                "New_Cell".to_string()
            } else {
                "New_Equilibrium".to_string()
            },
            phases: vec![EquilibriumPhase {
                name: "Phase #1".to_string(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: !cell_mode,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: vec![Self::make_sp("N2(g)", 1.0)],
            }],
        };
        self.params = EquilibriumParams {
            calc_mode: if cell_mode {
                CalculationMode::CellMode
            } else {
                CalculationMode::Normal
            },
            thermo_mode: if cell_mode {
                ThermodynamicMode::Ntpq
            } else {
                ThermodynamicMode::Ntp
            },
            ..Default::default()
        };
        self.file_path_input = if cell_mode {
            "New_Cell.gem9".to_string()
        } else {
            "New_Equilibrium.gem9".to_string()
        };
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
        self.active_sheet = EquilSheetTab::Species;
    }

    fn ensure_axis_non_degenerate(&mut self, var: SweepVariable) {
        let fix = |r: &mut AxisRange, def_steps: usize, def_span: f64| {
            if r.steps < 2 {
                r.steps = def_steps;
            }
            if (r.final_val - r.initial).abs() < 1e-9 {
                r.final_val = r.initial + def_span;
            }
        };
        match var {
            SweepVariable::TemperatureC => fix(&mut self.params.temp_range, 11, 200.0),
            SweepVariable::PressureBar => fix(&mut self.params.pressure_range, 10, 9.0),
            SweepVariable::EnthalpyMj => fix(&mut self.params.enthalpy_range, 11, 1000.0),
            SweepVariable::VolumeM3 => fix(&mut self.params.volume_range, 11, 9.0),
            SweepVariable::DischargeLevelKmolE => fix(&mut self.params.discharge_range, 11, 5.0),
            SweepVariable::XAmount => {
                if self.params.x_steps < 2 {
                    self.params.x_steps = 11;
                }
            }
            SweepVariable::ZAmount => {
                if self.params.z_steps < 2 {
                    self.params.z_steps = 11;
                }
            }
        }
    }

    /// Ensures switching to 2 independent variables gives distinct, non-degenerate X and Z ranges.
    fn ensure_2d_sweep_defaults(&mut self) {
        let fallback_sec = if self.params.primary_var == SweepVariable::PressureBar {
            SweepVariable::TemperatureC
        } else {
            SweepVariable::PressureBar
        };
        let sec = match self.params.secondary_var {
            Some(v) if v != self.params.primary_var => v,
            _ => fallback_sec,
        };
        self.params.secondary_var = Some(sec);
        self.ensure_axis_non_degenerate(self.params.primary_var);
        self.ensure_axis_non_degenerate(sec);
    }

    fn load_preset_nico(&mut self) {
        self.system = EquilibriumSystem {
            title: "NICO".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "Phase #1".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("CO(g)", 4.0),
                        Self::make_sp("CO2(g)", 0.0),
                        Self::make_sp("Ni(g)", 0.0),
                        Self::make_sp("Ni(CO)4(g)", 0.0),
                        Self::make_sp("O2(g)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "Phase #2".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("Ni", 1.0)],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Effect of temperature on the stability of Ni(CO)4(g)".to_string(),
            num_indep_vars: 1,
            primary_var: SweepVariable::TemperatureC,
            temp_range: AxisRange {
                initial: 0.0,
                final_val: 300.0,
                steps: 31,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "NICO.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::EquilibriumAmount;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
        self.initialized_default = true;
    }

    fn load_preset_fes2(&mut self) {
        self.system = EquilibriumSystem {
            title: "FES2".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("N2(g)", 0.1),
                        Self::make_sp("S2(g)", 0.0),
                        Self::make_sp("S(g)", 0.0),
                        Self::make_sp("Fe(g)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "SOLID".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("FeS", 0.0), Self::make_sp("FeS2", 1.0)],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Thermal decomposition of FeS2".to_string(),
            num_indep_vars: 1,
            primary_var: SweepVariable::TemperatureC,
            temp_range: AxisRange {
                initial: 200.0,
                final_val: 1000.0,
                steps: 33,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "FES2.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::EquilibriumAmount;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_caco3_aqua(&mut self) {
        self.system = EquilibriumSystem {
            title: "CACO3".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("N2(g)", 1.0),
                        Self::make_sp("H2O(g)", 0.0),
                        Self::make_sp("CO2(g)", 1.5),
                    ],
                },
                EquilibriumPhase {
                    name: "AQUA".to_string(),
                    model: SolutionModel::Aqua,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("H2O", 55.5),
                        Self::make_sp("Ca(+2a)", 1.0),
                        Self::make_sp("OH(-a)", 2.0),
                        Self::make_sp("HCO3(-a)", 0.0),
                        Self::make_sp("CO3(-2a)", 0.0),
                        Self::make_sp("H(+a)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "SOLID".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("CaCO3", 0.0),
                        Self::make_sp("Ca(OH)2", 0.0),
                    ],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Effect of CO2 on CaCO3 precipitation at 25 C".to_string(),
            num_indep_vars: 0,
            temp_range: AxisRange {
                initial: 25.0,
                final_val: 25.0,
                steps: 1,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "CACO3.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::EquilibriumAmount;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_feso4_target(&mut self) {
        self.system = EquilibriumSystem {
            title: "FESO4_target".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("N2(g)", 1e-10),
                        Self::make_sp("O2(g)", 1e-12),
                        Self::make_sp("H2O(g)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "AQUA".to_string(),
                    model: SolutionModel::Aqua,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("H2O", 0.0)],
                },
                EquilibriumPhase {
                    name: "SOLID".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("FeSO4*7H2O", 0.25),
                        Self::make_sp("FeSO4*4H2O", 0.0),
                        Self::make_sp("FeSO4*H2O", 0.0),
                        Self::make_sp("FeSO4", 0.0),
                    ],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Target calculation of FeSO4*4H2O at 1 bar".to_string(),
            num_indep_vars: 1,
            primary_var: SweepVariable::TemperatureC,
            temp_range: AxisRange {
                initial: 0.0,
                final_val: 100.0,
                steps: 101,
                log_scale: false,
            },
            target: TargetSpec {
                enabled: true,
                target_species: "FeSO4*4H2O".to_string(),
                target_amount_kmol: 0.25,
            },
            ..Default::default()
        };
        self.file_path_input = "FESO4_target.gem9".to_string();
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_na2so4(&mut self) {
        self.system = EquilibriumSystem {
            title: "NA2SO4".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "Phase #1 (GAS)".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("N2(g)", 1.0),
                        Self::make_sp("CO(g)", 0.0),
                        Self::make_sp("CO2(g)", 0.0),
                        Self::make_sp("COS(g)", 0.0),
                        Self::make_sp("O2(g)", 0.0),
                        Self::make_sp("S(g)", 0.0),
                        Self::make_sp("S2(g)", 0.0),
                        Self::make_sp("S3(g)", 0.0),
                        Self::make_sp("S4(g)", 0.0),
                        Self::make_sp("S5(g)", 0.0),
                        Self::make_sp("S6(g)", 0.0),
                        Self::make_sp("S7(g)", 0.0),
                        Self::make_sp("S8(g)", 0.0),
                        Self::make_sp("SO2(g)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "Phase #2 (Condensed Mix)".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("Na2CO3", 0.0),
                        Self::make_sp("NaO2", 0.0),
                        Self::make_sp("Na2O", 0.0),
                        Self::make_sp("Na2O2", 0.0),
                        Self::make_sp("NaS", 0.0),
                        Self::make_sp("NaS2", 0.0),
                        Self::make_sp("Na2S", 0.0),
                        Self::make_sp("Na2S2", 0.0),
                        Self::make_sp("Na2S3", 0.0),
                        Self::make_sp("Na2SO3", 0.0),
                        Self::make_sp("Na2SO4", 1.0),
                    ],
                },
                EquilibriumPhase {
                    name: "Phase #3 (Carbon)".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("C", 4.0)],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Reduction of Na2SO4 with Carbon (NA2SO4.gem9)".to_string(),
            num_indep_vars: 2,
            primary_var: SweepVariable::TemperatureC,
            secondary_var: Some(SweepVariable::PressureBar),
            temp_range: AxisRange {
                initial: 0.0,
                final_val: 1000.0,
                steps: 21,
                log_scale: false,
            },
            pressure_range: AxisRange {
                initial: 1.0,
                final_val: 10.0,
                steps: 10,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "NA2SO4.gem9".to_string();
        self.chart_is_3d = true;
        self.chart_3d_yaw = -0.52;
        self.chart_3d_pitch = 0.42;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_feso4_adiabatic(&mut self) {
        self.system = EquilibriumSystem {
            title: "FESO4_adiabatic".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("N2(g)", 1e-6),
                        Self::make_sp("O2(g)", 1e-6),
                        Self::make_sp("H2O(g)", 0.0),
                        Self::make_sp("SO2(g)", 0.0),
                        Self::make_sp("SO3(g)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "AQUA".to_string(),
                    model: SolutionModel::Aqua,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("H2O", 0.0)],
                },
                EquilibriumPhase {
                    name: "SOLID".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("FeSO4*7H2O", 2.5),
                        Self::make_sp("FeSO4*4H2O", 0.0),
                        Self::make_sp("FeSO4*H2O", 0.0),
                        Self::make_sp("FeSO4", 0.0),
                        Self::make_sp("Fe2O3", 0.0),
                        Self::make_sp("Fe3O4", 0.0),
                    ],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Adiabatic Heating of FeSO4*7H2O (FESO4_adiabatic.gem9)".to_string(),
            thermo_mode: ThermodynamicMode::Nhp,
            num_indep_vars: 1,
            primary_var: SweepVariable::EnthalpyMj,
            enthalpy_range: AxisRange {
                initial: 0.0,
                final_val: 8000.0,
                steps: 51,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "FESO4_adiabatic.gem9".to_string();
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_leadbatt(&mut self) {
        let mut pb = Self::make_sp("Pb", 6.0);
        pb.discharge_coeff = -2.0;
        self.system = EquilibriumSystem {
            title: "LEADBATT".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "GAS".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("N2(g)", 0.1)],
                },
                EquilibriumPhase {
                    name: "ANODE (Lead)".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: "anode".to_string(),
                    capacitance_farads: 0.0,
                    species: vec![pb],
                },
                EquilibriumPhase {
                    name: "ELECTROLYTE".to_string(),
                    model: SolutionModel::Aqua,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("H2O", 55.5),
                        Self::make_sp("H(+a)", 14.0),
                        Self::make_sp("SO4(-2a)", 7.0),
                        Self::make_sp("HSO4(-a)", 0.0),
                        Self::make_sp("OH(-a)", 0.0),
                        Self::make_sp("Pb(+2a)", 0.0),
                    ],
                },
                EquilibriumPhase {
                    name: "CATHODE (PbO2)".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: "cathode".to_string(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("PbO2", 6.0)],
                },
                EquilibriumPhase {
                    name: "SOLID (PbSO4)".to_string(),
                    model: SolutionModel::PureSubstance,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("PbSO4", 0.2)],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Lead-Acid Battery Discharge Equilibrium (LEADBATT.gem9)".to_string(),
            calc_mode: CalculationMode::CellMode,
            thermo_mode: ThermodynamicMode::Ntpq,
            num_indep_vars: 1,
            primary_var: SweepVariable::DischargeLevelKmolE,
            discharge_range: AxisRange {
                initial: 0.0,
                final_val: 6.25,
                steps: 26,
                log_scale: false,
            },
            temp_range: AxisRange {
                initial: 25.0,
                final_val: 25.0,
                steps: 1,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "LEADBATT.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::CellVoltage;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_co2_modes(&mut self, mode: CalculationMode) {
        let mut n2 = Self::make_sp("N2(g)", 0.79);
        let mut o2 = Self::make_sp("O2(g)", 0.21);
        let mut co2_g = Self::make_sp("CO2(g)", if mode == CalculationMode::FixedActivity { 0.0 } else { 0.0003 });
        if mode == CalculationMode::Normal {
            n2.add_x_amount = 0.79;
            n2.remove_percent = 100.0;
            o2.add_x_amount = 0.21;
            o2.remove_percent = 100.0;
            co2_g.add_x_amount = 0.0003;
            co2_g.remove_percent = 100.0;
        } else if mode == CalculationMode::FixedActivity {
            co2_g.fixed_activity = Some(0.0003);
        }
        self.system = EquilibriumSystem {
            title: "CO2_Dissolution".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "Phase #1 (GAS)".to_string(),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: true,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![n2, o2, co2_g],
                },
                EquilibriumPhase {
                    name: "Phase #2 (AQUA)".to_string(),
                    model: SolutionModel::Aqua,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![
                        Self::make_sp("H2O", 100.0),
                        Self::make_sp("CO2(a)", 0.0),
                        Self::make_sp("HCO3(-a)", 0.0),
                        Self::make_sp("CO3(-2a)", 0.0),
                        Self::make_sp("H(+a)", 0.0),
                        Self::make_sp("OH(-a)", 0.0),
                    ],
                },
            ],
        };
        let is_rem = mode == CalculationMode::Normal;
        self.params = EquilibriumParams {
            chart_header: match mode {
                CalculationMode::Normal => "CO2 Dissolution — Remove % Mode (Figs. 9-10)".to_string(),
                CalculationMode::InfinitePhase => "CO2 Dissolution — Infinite Phase Mode (Figs. 11-12)".to_string(),
                CalculationMode::FixedActivity => "CO2 Dissolution — Fixed Activity Mode (Figs. 13-15)".to_string(),
                _ => "CO2 Dissolution".to_string(),
            },
            calc_mode: mode,
            step_mode: if is_rem { StepMode::RemovePercent } else { StepMode::AddAmount },
            infinite_phase_idx: if mode == CalculationMode::InfinitePhase { Some(0) } else { None },
            num_indep_vars: 1,
            primary_var: if is_rem { SweepVariable::XAmount } else { SweepVariable::TemperatureC },
            x_steps: 21,
            temp_range: AxisRange {
                initial: if is_rem { 25.0 } else { 0.0 },
                final_val: if is_rem { 25.0 } else { 100.0 },
                steps: 21,
                log_scale: false,
            },
            ..Default::default()
        };
        self.file_path_input = "CO2_Dissolution.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::EquilibriumAmount;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_al_zn_appendix_c(&mut self) {
        let mut al_l = Self::make_sp("Al(l)", 0.0);
        al_l.add_x_amount = 0.05;
        let mut zn_l = Self::make_sp("Zn(l)", 1.0);
        zn_l.add_x_amount = -0.05;
        self.system = EquilibriumSystem {
            title: "AL_ZN".to_string(),
            phases: vec![
                EquilibriumPhase {
                    name: "LIQUID_AL_ZN".to_string(),
                    model: SolutionModel::AlZnMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![al_l, zn_l],
                },
                EquilibriumPhase {
                    name: "SOLID_FCC".to_string(),
                    model: SolutionModel::AlZnMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: vec![Self::make_sp("Al", 0.01), Self::make_sp("Zn", 0.01)],
                },
            ],
        };
        self.params = EquilibriumParams {
            chart_header: "Al-Zn Binary Redlich-Kister Solution Model (Appendix C)".to_string(),
            num_indep_vars: 1,
            primary_var: SweepVariable::TemperatureC,
            temp_range: AxisRange {
                initial: 300.0,
                final_val: 700.0,
                steps: 21,
                log_scale: false,
            },
            ac_steps: 5,
            ..Default::default()
        };
        self.file_path_input = "AL_ZN.gem9".to_string();
        self.chart_x_kind = ChartXKind::StepVariable;
        self.chart_y_kind = ChartYKind::Activity;
        self.chart_is_3d = false;
        self.chart_3d_cache = None;
        self.result = None;
        self.last_error = None;
    }

    fn load_preset_h_c_o_ca_3d(&mut self) {
        self.load_preset_caco3_aqua();
        self.system.phases[0].species[2].initial_amount = 0.0;
        self.system.phases[0].species[2].add_x_amount = 0.2;
        self.params.chart_header = "H-C-O-Ca System 3D (CO2 x T)".to_string();
        self.params.num_indep_vars = 2;
        self.params.primary_var = SweepVariable::XAmount;
        self.params.secondary_var = Some(SweepVariable::TemperatureC);
        self.params.x_steps = 16;
        self.params.temp_range.initial = 25.0;
        self.params.temp_range.final_val = 200.0;
        self.params.temp_range.steps = 15;
        self.chart_is_3d = true;
        self.chart_3d_cache = None;
    }

    fn update_species_from_db(&mut self, db: &ThermoDatabase) {
        for phase in &mut self.system.phases {
            for sp in &mut phase.species {
                if let Some(rec) = db.get_species(sp.formula.trim()) {
                    sp.species_data = Some(rec);
                }
            }
        }
        self.file_status_msg = Some("Updated species records from database.".to_string());
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        // Consistent Top Menu Bar (matching Modules 1, 2, and 3)
        egui::TopBottomPanel::top("equil_menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Empty File (Normal)").clicked() {
                        self.load_empty_workbook(false);
                        ui.close_menu();
                    }
                    if ui.button("New Empty File (Cell Mode)").clicked() {
                        self.load_empty_workbook(true);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Open (.gem9)...").clicked() {
                        match std::fs::read_to_string(&self.file_path_input)
                            .map_err(|e| e.to_string())
                            .and_then(|s| EquilibriumWorkbookFile::from_gem9_str(&s))
                        {
                            Ok(wb) => {
                                self.system = wb.system;
                                self.params = wb.params;
                                self.result = None;
                                self.last_error = None;
                                self.file_status_msg =
                                    Some(format!("Loaded {}", self.file_path_input));
                            }
                            Err(e) => {
                                self.last_error = Some(format!("Open error: {}", e));
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save (.gem9)...").clicked() {
                        let wb = EquilibriumWorkbookFile {
                            system: self.system.clone(),
                            params: self.params.clone(),
                        };
                        match wb
                            .to_gem9_json()
                            .and_then(|json| std::fs::write(&self.file_path_input, json).map_err(|e| e.to_string()))
                        {
                            Ok(()) => {
                                self.file_status_msg =
                                    Some(format!("Saved {}", self.file_path_input));
                            }
                            Err(e) => {
                                self.last_error = Some(format!("Save error: {}", e));
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Chapter 13 Presets").small().strong());
                    if ui.button("Preset: Ni-CO(g) (NICO.gem9)").clicked() {
                        self.load_preset_nico();
                        ui.close_menu();
                    }
                    if ui.button("Preset: FeS2 Decomposition").clicked() {
                        self.load_preset_fes2();
                        ui.close_menu();
                    }
                    if ui.button("Preset: CaCO3 + CO2(g) + Aqua").clicked() {
                        self.load_preset_caco3_aqua();
                        ui.close_menu();
                    }
                    if ui.button("Preset: FeSO4*4H2O Target (Fig. 7-8)").clicked() {
                        self.load_preset_feso4_target();
                        ui.close_menu();
                    }
                    if ui.button("Preset: Na2SO4 + C Reduction (NA2SO4.gem9, Fig. 34)").clicked() {
                        self.load_preset_na2so4();
                        ui.close_menu();
                    }
                    if ui.button("Preset: FeSO4*7H2O Adiabatic (FESO4_adiabatic.gem9, Fig. 37)").clicked() {
                        self.load_preset_feso4_adiabatic();
                        ui.close_menu();
                    }
                    if ui.button("Preset: Lead-Acid Battery Cell Mode (LEADBATT.gem9, Fig. 41-44)").clicked() {
                        self.load_preset_leadbatt();
                        ui.close_menu();
                    }
                    if ui.button("Preset: CO2 Dissolution — Remove % (Fig. 9-10)").clicked() {
                        self.load_preset_co2_modes(CalculationMode::Normal);
                        ui.close_menu();
                    }
                    if ui.button("Preset: CO2 Dissolution — Infinite Phase (Fig. 11-12)").clicked() {
                        self.load_preset_co2_modes(CalculationMode::InfinitePhase);
                        ui.close_menu();
                    }
                    if ui.button("Preset: CO2 Dissolution — Fixed Activity (Fig. 13-15)").clicked() {
                        self.load_preset_co2_modes(CalculationMode::FixedActivity);
                        ui.close_menu();
                    }
                    if ui.button("Preset: Al-Zn Redlich-Kister Alloy (Appendix C)").clicked() {
                        self.load_preset_al_zn_appendix_c();
                        ui.close_menu();
                    }
                    if ui.button("Preset: H-C-O-Ca 3D (CO2 x T)").clicked() {
                        self.load_preset_h_c_o_ca_3d();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Update Species Data from Database").clicked() {
                        self.update_species_from_db(db);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            self.system.phases.len() > 1,
                            egui::Button::new("Delete Selected Phase"),
                        )
                        .clicked()
                    {
                        let p_idx = self
                            .selected_phase_idx
                            .min(self.system.phases.len().saturating_sub(1));
                        self.system.phases.remove(p_idx);
                        self.selected_phase_idx = 0;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Insert", |ui| {
                    if ui.button("From Elements...").clicked() {
                        self.show_from_elements = true;
                        ui.close_menu();
                    }
                    if ui.button("Insert Phase").clicked() {
                        let idx = self.system.phases.len() + 1;
                        self.system.phases.push(EquilibriumPhase {
                            name: format!("Phase #{}", idx),
                            model: SolutionModel::IdealMixture,
                            is_gas_phase: false,
                            electrode_role: String::new(),
                            capacitance_farads: 0.0,
                            species: Vec::new(),
                        });
                        ui.close_menu();
                    }
                    if ui.button("Sort Phases (Gas, Aqua, Liquid, Solid)").clicked() {
                        self.sort_phases(db);
                        ui.close_menu();
                    }
                    if ui.button("Add N2(g) Stability").clicked() {
                        if let Some(first) = self.system.phases.first_mut() {
                            if !first.species.iter().any(|s| s.formula == "N2(g)") {
                                first.species.insert(0, Self::make_sp("N2(g)", 1e-10));
                            }
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Units", |ui| {
                    ui.label(egui::RichText::new("Temperature").small().strong());
                    if ui
                        .radio(!self.params.temp_unit_kelvin, "Celsius (°C)")
                        .clicked()
                    {
                        self.params.temp_unit_kelvin = false;
                        ui.close_menu();
                    }
                    if ui
                        .radio(self.params.temp_unit_kelvin, "Kelvin (K)")
                        .clicked()
                    {
                        self.params.temp_unit_kelvin = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Column C Mode").small().strong());
                    if ui
                        .radio(!self.params.column_c_lnf_mode, "Activity Coefficient (f)")
                        .clicked()
                    {
                        self.params.column_c_lnf_mode = false;
                        ui.close_menu();
                    }
                    if ui
                        .radio(self.params.column_c_lnf_mode, "Natural Log ln(f)")
                        .clicked()
                    {
                        self.params.column_c_lnf_mode = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Set All Species Amount Units").small().strong());
                    for u in [
                        EquilibriumAmountUnit::Kmol,
                        EquilibriumAmountUnit::Mol,
                        EquilibriumAmountUnit::Kg,
                        EquilibriumAmountUnit::Nm3,
                    ] {
                        if ui.button(format!("All -> {}", u.label())).clicked() {
                            for ph in &mut self.system.phases {
                                for sp in &mut ph.species {
                                    sp.amount_unit = u;
                                }
                            }
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Calculate", |ui| {
                    if ui.button("▶ Calculate Equilibrium").clicked() {
                        self.request_calculate(db);
                        ui.close_menu();
                    }
                    if ui.button("Check Cp Extrapolation Warnings...").clicked() {
                        self.warnings = collect_cp_warnings(&self.system, &self.params, db);
                        self.warnings_pending_calc = false;
                        self.show_warnings_modal = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .add_enabled(
                            self.result.is_some(),
                            egui::Button::new("Show Chart (2D / 3D)"),
                        )
                        .clicked()
                    {
                        self.show_chart_window = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            self.result.is_some(),
                            egui::Button::new("Show Equilibrium Results Table"),
                        )
                        .clicked()
                    {
                        self.show_diagram_table = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Window", |ui| {
                    if ui.button("Add-Ins (Solution Models)...").clicked() {
                        self.show_addins_modal = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Bottom Status Bar (matching Modules 1, 2, and 3)
        egui::TopBottomPanel::bottom("equil_status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Soryntel Equilibrium | Sistema: {} | Modo: {} ({}) | Fases: {}",
                        self.system.title,
                        self.params.calc_mode.label(),
                        self.params.thermo_mode.label(),
                        self.system.phases.len()
                    ))
                    .small()
                    .color(egui::Color32::DARK_GRAY),
                );
                if let Some(ref msg) = self.file_status_msg {
                    ui.separator();
                    ui.label(egui::RichText::new(msg).small().color(egui::Color32::DARK_BLUE));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref err) = self.last_error {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                    } else if let Some(ref res) = self.result {
                        ui.colored_label(
                            egui::Color32::from_rgb(22, 101, 52),
                            format!("Convergido ({} puntos)", res.points.len()),
                        );
                    }
                });
            });
        });

        // Right Docked Parameters Panel (matches Fig. 6)
        egui::SidePanel::right("equil_parameters_panel")
            .resizable(true)
            .default_width(355.0)
            .show(ctx, |ui| {
                self.render_parameters_panel(ui, db);
            });

        // Central Excel Workbook Area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_toolbar_and_formula_bar(ui, db);
            ui.separator();

            match self.active_sheet {
                EquilSheetTab::Species => self.render_species_sheet(ui),
                EquilSheetTab::UserFunction => self.render_user_function_sheet(ui),
                EquilSheetTab::Results1 => self.render_results_sheet(ui),
                EquilSheetTab::TargetResults => self.render_target_results_sheet(ui),
            }

            // Bottom Excel-style sheet tabs
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("◄ ►").small().color(egui::Color32::GRAY));
                ui.selectable_value(&mut self.active_sheet, EquilSheetTab::Species, "Species");
                ui.selectable_value(
                    &mut self.active_sheet,
                    EquilSheetTab::UserFunction,
                    "UserFunction",
                );
                ui.selectable_value(&mut self.active_sheet, EquilSheetTab::Results1, "Results1");
                if self.params.target.enabled
                    || self
                        .result
                        .as_ref()
                        .and_then(|r| r.target_summary.as_ref())
                        .is_some()
                {
                    ui.selectable_value(
                        &mut self.active_sheet,
                        EquilSheetTab::TargetResults,
                        "Target Results",
                    );
                }
                if let Some(ref err) = self.last_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }
            });
        });

        self.render_modals(ctx, db);
    }

    fn request_calculate(&mut self, db: &ThermoDatabase) {
        if !self.dont_show_warnings {
            let ws = collect_cp_warnings(&self.system, &self.params, db);
            if !ws.is_empty() {
                self.warnings = ws;
                self.warnings_pending_calc = true;
                self.show_warnings_modal = true;
                return;
            }
        }
        self.run_calculate(db);
    }

    fn run_calculate(&mut self, db: &ThermoDatabase) {
        self.last_error = None;
        match solve_equilibrium_sweep(&self.system, &self.params, db) {
            Ok(res) => {
                let (xs, zs) = collect_distinct_axes(&res, false);
                self.chart_is_3d =
                    xs.len() >= 2 && zs.len() >= 2 && xs.len() * zs.len() == res.points.len();
                if res.target_summary.is_some() {
                    self.active_sheet = EquilSheetTab::TargetResults;
                } else {
                    self.active_sheet = EquilSheetTab::Results1;
                }
                self.result = Some(res);
                if self.chart_auto_axes {
                    self.reset_chart_bounds_from_result();
                }
            }
            Err(e) => {
                self.last_error = Some(e.to_string());
            }
        }
    }

    fn render_toolbar_and_formula_bar(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("From Elements").clicked() {
                self.show_from_elements = true;
            }
            if ui.button("Sort Phases").clicked() {
                self.sort_phases(db);
            }
            if ui.button("+ Phase").clicked() {
                let idx = self.system.phases.len() + 1;
                self.system.phases.push(EquilibriumPhase {
                    name: format!("Phase #{}", idx),
                    model: SolutionModel::IdealMixture,
                    is_gas_phase: false,
                    electrode_role: String::new(),
                    capacitance_farads: 0.0,
                    species: Vec::new(),
                });
            }
            ui.separator();
            ui.label("Add Species:");
            ui.add(
                egui::TextEdit::singleline(&mut self.add_species_query)
                    .desired_width(110.0)
                    .hint_text("e.g. CH4(g)"),
            );
            if ui.button("Add").clicked() && !self.add_species_query.trim().is_empty() {
                let f = self.add_species_query.trim().to_string();
                let target_p = self.selected_phase_idx.min(self.system.phases.len().saturating_sub(1));
                if let Some(p) = self.system.phases.get_mut(target_p) {
                    p.species.push(Self::make_sp(&f, 0.0));
                }
                self.add_species_query.clear();
            }
            if ui.button("+ N2(g) Stability").clicked() {
                if let Some(first) = self.system.phases.first_mut() {
                    if !first.species.iter().any(|s| s.formula == "N2(g)") {
                        first.species.insert(0, Self::make_sp("N2(g)", 1e-10));
                    }
                }
            }
            if ui.button("Warnings").clicked() {
                self.warnings = collect_cp_warnings(&self.system, &self.params, db);
                self.warnings_pending_calc = false;
                self.show_warnings_modal = true;
            }
            ui.separator();
            ui.label("File:");
            ui.add(
                egui::TextEdit::singleline(&mut self.file_path_input)
                    .desired_width(120.0),
            );
            ui.separator();
            egui::ComboBox::from_id_salt("equil_presets")
                .selected_text("Presets")
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "Ni-CO(g) (NICO.gem9)").clicked() {
                        self.load_preset_nico();
                    }
                    if ui.selectable_label(false, "FeS2 Decomposition").clicked() {
                        self.load_preset_fes2();
                    }
                    if ui.selectable_label(false, "CaCO3 + CO2(g) + Aqua").clicked() {
                        self.load_preset_caco3_aqua();
                    }
                    if ui.selectable_label(false, "FeSO4*4H2O Target").clicked() {
                        self.load_preset_feso4_target();
                    }
                    if ui.selectable_label(false, "Na2SO4 + C Reduction (NA2SO4.gem9)").clicked() {
                        self.load_preset_na2so4();
                    }
                    if ui.selectable_label(false, "FeSO4*7H2O Adiabatic (FESO4_adiabatic.gem9)").clicked() {
                        self.load_preset_feso4_adiabatic();
                    }
                    if ui.selectable_label(false, "Lead-Acid Battery Cell (LEADBATT.gem9)").clicked() {
                        self.load_preset_leadbatt();
                    }
                    if ui.selectable_label(false, "CO2 Dissolution — Remove %").clicked() {
                        self.load_preset_co2_modes(CalculationMode::Normal);
                    }
                    if ui.selectable_label(false, "CO2 Dissolution — Infinite Phase").clicked() {
                        self.load_preset_co2_modes(CalculationMode::InfinitePhase);
                    }
                    if ui.selectable_label(false, "CO2 Dissolution — Fixed Activity").clicked() {
                        self.load_preset_co2_modes(CalculationMode::FixedActivity);
                    }
                    if ui.selectable_label(false, "Al-Zn Alloy (Appendix C)").clicked() {
                        self.load_preset_al_zn_appendix_c();
                    }
                    if ui.selectable_label(false, "H-C-O-Ca 3D (CO2 x T)").clicked() {
                        self.load_preset_h_c_o_ca_3d();
                    }
                });
        });

        // Excel Formula Bar
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Active species:").small());
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut self.active_cell_ref).desired_width(50.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.active_cell_text)
                    .desired_width(ui.available_width().max(120.0)),
            );
        });
    }

    fn sort_phases(&mut self, db: &ThermoDatabase) {
        let mut gas = Vec::new();
        let mut aqua = Vec::new();
        let mut liq = Vec::new();
        let mut sol = Vec::new();
        for p in &self.system.phases {
            for sp in &p.species {
                let ph = db
                    .get_species(sp.formula.trim())
                    .map(|d| d.phase)
                    .unwrap_or(Phase::Solid);
                match ph {
                    Phase::Gas => gas.push(sp.clone()),
                    Phase::AqueousIon | Phase::AqueousNeutral => aqua.push(sp.clone()),
                    Phase::Liquid => liq.push(sp.clone()),
                    Phase::Solid => sol.push(sp.clone()),
                }
            }
        }
        let mut new_phases = Vec::new();
        if !gas.is_empty() {
            new_phases.push(EquilibriumPhase {
                name: "GAS".to_string(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: true,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: gas,
            });
        }
        if !aqua.is_empty() {
            new_phases.push(EquilibriumPhase {
                name: "AQUA".to_string(),
                model: SolutionModel::Aqua,
                is_gas_phase: false,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: aqua,
            });
        }
        if !liq.is_empty() {
            new_phases.push(EquilibriumPhase {
                name: "LIQUID".to_string(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: false,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: liq,
            });
        }
        if !sol.is_empty() {
            new_phases.push(EquilibriumPhase {
                name: "SOLID".to_string(),
                model: SolutionModel::PureSubstance,
                is_gas_phase: false,
                electrode_role: String::new(),
                capacitance_farads: 0.0,
                species: sol,
            });
        }
        if !new_phases.is_empty() {
            self.system.phases = new_phases;
        }
    }

    fn render_species_sheet(&mut self, ui: &mut egui::Ui) {
        let is_fixed_act = self.params.calc_mode == CalculationMode::FixedActivity;
        let is_cell = self.params.calc_mode == CalculationMode::CellMode;
        let is_remove = matches!(
            self.params.step_mode,
            StepMode::RemovePercent
                | StepMode::RemovePercentFinalAmount
                | StepMode::ReactorStandard
                | StepMode::ReactorSheetsView
        );
        let num_cols = if is_fixed_act || is_cell { 9 } else { 8 };
        let t_offset = if self.params.temp_unit_kelvin { 273.15 } else { 0.0 };

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("equil_species_excel_grid")
                .striped(false)
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    // Column letter row
                    row_num_cell(ui, "");
                    for c in 0..num_cols {
                        col_letter(ui, &((b'A' + c as u8) as char).to_string());
                    }
                    ui.end_row();

                    // Header row 1-3
                    row_num_cell(ui, "1");
                    header_cell(ui, "Species Formula");
                    header_cell(
                        ui,
                        if self.params.temp_unit_kelvin {
                            "Feed Temp (K)"
                        } else {
                            "Feed Temp (°C)"
                        },
                    );
                    header_cell(
                        ui,
                        if self.params.column_c_lnf_mode {
                            "Activity Coeff ln(f)"
                        } else {
                            "Activity Coeff (f)"
                        },
                    );
                    if is_fixed_act {
                        header_cell(ui, "Fixed Activity");
                    } else if is_cell {
                        header_cell(ui, "Discharge Coeff");
                    }
                    header_cell(ui, "Amount Unit");
                    header_cell(ui, "Initial Amount");
                    header_cell(ui, "Initial Amount %");
                    header_cell(ui, "Add X-amount");
                    header_cell(
                        ui,
                        if is_remove {
                            "Remove Amount (%)"
                        } else {
                            "Add Z-amount"
                        },
                    );
                    ui.end_row();

                    let mut excel_row = 4usize;
                    let mut remove_sp: Option<(usize, usize)> = None;

                    for p_idx in 0..self.system.phases.len() {
                        let phase_total: f64 = self.system.phases[p_idx]
                            .species
                            .iter()
                            .filter(|s| !s.excluded)
                            .map(|s| s.initial_amount)
                            .sum();
                        let phase_x_total: f64 = self.system.phases[p_idx]
                            .species
                            .iter()
                            .filter(|s| !s.excluded)
                            .map(|s| s.add_x_amount)
                            .sum();
                        let phase_z_total: f64 = self.system.phases[p_idx]
                            .species
                            .iter()
                            .filter(|s| !s.excluded)
                            .map(|s| s.add_z_amount)
                            .sum();

                        // Phase separator row
                        row_num_cell(ui, &excel_row.to_string());
                        excel_row += 1;
                        let p_name = self.system.phases[p_idx].name.clone();
                        widget_cell(ui, true, |ui| {
                            if ui
                                .selectable_label(self.selected_phase_idx == p_idx, &p_name)
                                .clicked()
                            {
                                self.selected_phase_idx = p_idx;
                            }
                        });
                        // Phase-level temperature editing (`13.3.4`, L171-174)
                        widget_cell(ui, true, |ui| {
                            let mut phase_t = self.system.phases[p_idx]
                                .species
                                .first()
                                .map(|s| s.feed_temp_c + t_offset)
                                .unwrap_or(25.0 + t_offset);
                            if ui
                                .add(
                                    egui::DragValue::new(&mut phase_t)
                                        .speed(1.0)
                                        .fixed_decimals(2),
                                )
                                .changed()
                            {
                                for sp in &mut self.system.phases[p_idx].species {
                                    sp.feed_temp_c = phase_t - t_offset;
                                }
                            }
                        });
                        widget_cell(ui, true, |ui| {
                            let model = &mut self.system.phases[p_idx].model;
                            egui::ComboBox::from_id_salt(("phase_model", p_idx))
                                .selected_text(model.label())
                                .width(95.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        model,
                                        SolutionModel::IdealMixture,
                                        "Mixture",
                                    );
                                    ui.selectable_value(model, SolutionModel::PureSubstance, "Pure");
                                    ui.selectable_value(model, SolutionModel::Aqua, "Aqua");
                                    ui.selectable_value(
                                        model,
                                        SolutionModel::GaAsMixture,
                                        "Ga-As Mixture",
                                    );
                                    ui.selectable_value(
                                        model,
                                        SolutionModel::AlZnMixture,
                                        "Al-Zn Mixture",
                                    );
                                });
                        });
                        if is_fixed_act || is_cell {
                            section_cell(ui, "");
                        }
                        section_cell(ui, "");
                        section_cell(ui, &format!("{:.3}", phase_total));
                        section_cell(ui, if phase_total > 0.0 { "100.000" } else { "0.000" });
                        section_cell(ui, &format!("{:.3}", phase_x_total));
                        if is_remove {
                            // Phase-level Remove Amount % (`Fig. 9`, Cell H4)
                            widget_cell(ui, true, |ui| {
                                let mut phase_rem = self.system.phases[p_idx]
                                    .species
                                    .first()
                                    .map(|s| s.remove_percent)
                                    .unwrap_or(0.0);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut phase_rem)
                                            .speed(1.0)
                                            .range(0.0..=100.0)
                                            .fixed_decimals(1),
                                    )
                                    .changed()
                                {
                                    for sp in &mut self.system.phases[p_idx].species {
                                        sp.remove_percent = phase_rem;
                                    }
                                }
                            });
                        } else {
                            section_cell(ui, &format!("{:.3}", phase_z_total));
                        }
                        ui.end_row();

                        // Species rows
                        let n_sp = self.system.phases[p_idx].species.len();
                        for s_idx in 0..n_sp {
                            let row_num = excel_row;
                            row_num_cell(ui, &row_num.to_string());
                            excel_row += 1;

                            let sp = &mut self.system.phases[p_idx].species[s_idx];
                            let pct = if phase_total > 1e-30 {
                                (sp.initial_amount / phase_total) * 100.0
                            } else {
                                0.0
                            };

                            widget_cell(ui, false, |ui| {
                                ui.horizontal(|ui| {
                                    if sp.excluded {
                                        ui.colored_label(
                                            egui::Color32::RED,
                                            format!("{} (out-T)", sp.formula),
                                        );
                                    } else {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut sp.formula)
                                                .desired_width(72.0),
                                        );
                                    }
                                    if ui.small_button("x").clicked() {
                                        remove_sp = Some((p_idx, s_idx));
                                    }
                                });
                            });
                            widget_cell(ui, false, |ui| {
                                let mut disp_t = sp.feed_temp_c + t_offset;
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut disp_t)
                                            .speed(1.0)
                                            .fixed_decimals(3),
                                    )
                                    .changed()
                                {
                                    sp.feed_temp_c = disp_t - t_offset;
                                }
                            });
                            widget_cell(ui, false, |ui| {
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut sp.activity_coeff_expr)
                                            .desired_width(80.0),
                                    )
                                    .changed()
                                {
                                    self.active_cell_ref = format!("C{}", row_num);
                                    self.active_cell_text = sp.activity_coeff_expr.clone();
                                }
                            });
                            if is_fixed_act {
                                widget_cell(ui, false, |ui| {
                                    let mut val = sp.fixed_activity.unwrap_or(0.0);
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut val)
                                                .speed(0.0001)
                                                .fixed_decimals(5)
                                                .range(0.0..=1e6),
                                        )
                                        .changed()
                                    {
                                        sp.fixed_activity = if val > 0.0 { Some(val) } else { None };
                                    }
                                });
                            } else if is_cell {
                                widget_cell(ui, false, |ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut sp.discharge_coeff)
                                            .speed(0.1)
                                            .fixed_decimals(3),
                                    );
                                });
                            }
                            widget_cell(ui, false, |ui| {
                                egui::ComboBox::from_id_salt(("unit", p_idx, s_idx))
                                    .selected_text(sp.amount_unit.label())
                                    .width(60.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::Kmol,
                                            "kmol",
                                        );
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::Mol,
                                            "mol",
                                        );
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::Kg,
                                            "kg",
                                        );
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::Nm3,
                                            "Nm3",
                                        );
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::GramPerLiter,
                                            "g/l",
                                        );
                                        ui.selectable_value(
                                            &mut sp.amount_unit,
                                            EquilibriumAmountUnit::MolPerLiter,
                                            "mol/l",
                                        );
                                    });
                            });
                            widget_cell(ui, false, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut sp.initial_amount)
                                            .speed(0.05)
                                            .fixed_decimals(4),
                                    )
                                    .changed()
                                {
                                    self.active_cell_ref = format!("E{}", row_num);
                                    self.active_cell_text = format!("{}", sp.initial_amount);
                                }
                            });
                            text_cell(ui, &format!("{:.3}", pct), true, true);
                            widget_cell(ui, false, |ui| {
                                ui.add(
                                    egui::DragValue::new(&mut sp.add_x_amount)
                                        .speed(0.05)
                                        .fixed_decimals(3),
                                );
                            });
                            widget_cell(ui, false, |ui| {
                                if is_remove {
                                    ui.add(
                                        egui::DragValue::new(&mut sp.remove_percent)
                                            .speed(1.0)
                                            .range(0.0..=100.0)
                                            .fixed_decimals(2),
                                    );
                                } else {
                                    ui.add(
                                        egui::DragValue::new(&mut sp.add_z_amount)
                                            .speed(0.05)
                                            .fixed_decimals(3),
                                    );
                                }
                            });
                            ui.end_row();
                        }
                    }

                    if let Some((p_i, s_i)) = remove_sp {
                        self.system.phases[p_i].species.remove(s_i);
                    }
                });
        });
    }

    fn render_user_function_sheet(&mut self, ui: &mut egui::Ui) {
        let Some(ref res) = self.result else {
            ui.label("Run Calculate first to populate UserFunction sheet.");
            return;
        };
        let Some(pt) = res.points.first() else {
            return;
        };
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("equil_userfunc_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    row_num_cell(ui, "");
                    for c in 0..8 {
                        col_letter(ui, &((b'A' + c as u8) as char).to_string());
                    }
                    ui.end_row();

                    row_num_cell(ui, "1");
                    section_cell(ui, "Temperature");
                    section_cell(ui, "°C");
                    text_cell(ui, &format!("{:.2}", pt.temp_k - 273.15), true, false);
                    section_cell(ui, "Pressure");
                    section_cell(ui, "bar");
                    text_cell(ui, &format!("{:.3}", pt.pressure_bar), true, false);
                    section_cell(ui, "Volume (m3)");
                    text_cell(ui, &format!("{:.4e}", pt.volume_m3), true, false);
                    ui.end_row();

                    row_num_cell(ui, "4");
                    header_cell(ui, "Species Formula");
                    header_cell(ui, "Unit");
                    header_cell(ui, "Initial Amount");
                    header_cell(ui, "Equil Amount");
                    header_cell(ui, "Equil Comp (%)");
                    header_cell(ui, "Activity Coeff");
                    header_cell(ui, "Enthalpy (MJ)");
                    header_cell(ui, "Entropy (J/K)");
                    ui.end_row();

                    let scale = self.params.input_scale_to_mol.max(1e-12);
                    for (idx, sp) in pt.species.iter().enumerate() {
                        row_num_cell(ui, &(5 + idx).to_string());
                        text_cell(ui, &sp.formula, false, false);
                        text_cell(ui, "kmol", false, false);
                        text_cell(ui, &format!("{:.4}", sp.input_amount_mol / scale), true, false);
                        text_cell(ui, &format!("{:.6}", sp.equil_amount_mol / scale), true, false);
                        text_cell(ui, &format!("{:.3}", sp.mole_fraction * 100.0), true, false);
                        text_cell(ui, &format!("{:.3}", sp.activity_coeff), true, false);
                        text_cell(ui, &format!("{:.4}", sp.enthalpy_kj / 1000.0), true, false);
                        text_cell(ui, &format!("{:.3}", sp.entropy_jk), true, false);
                        ui.end_row();
                    }

                    // User Formulas section
                    row_num_cell(ui, &(6 + pt.species.len()).to_string());
                    section_cell(ui, "User Formulas");
                    section_cell(ui, "Unit");
                    section_cell(ui, "Formula");
                    section_cell(ui, "Value (kmol)");
                    section_cell(ui, "Show");
                    section_cell(ui, "Difference");
                    section_cell(ui, "Cumulative");
                    widget_cell(ui, true, |ui| {
                        if ui.small_button("+ Formula").clicked() {
                            self.user_formulas.push(UserFormulaRow {
                                label: format!("Formula {}", self.user_formulas.len() + 1),
                                expr: "=D5".to_string(),
                                show_formula: true,
                                show_diff: false,
                                show_cumul: false,
                            });
                        }
                    });
                    ui.end_row();

                    let mut remove_uf: Option<usize> = None;
                    for (u_idx, uf) in self.user_formulas.iter_mut().enumerate() {
                        let rnum = 7 + pt.species.len() + u_idx;
                        row_num_cell(ui, &rnum.to_string());
                        widget_cell(ui, false, |ui| {
                            ui.add(egui::TextEdit::singleline(&mut uf.label).desired_width(92.0));
                        });
                        text_cell(ui, "kmol", false, false);
                        widget_cell(ui, false, |ui| {
                            ui.add(egui::TextEdit::singleline(&mut uf.expr).desired_width(90.0));
                        });
                        let val = eval_user_formula(&uf.expr, pt, scale);
                        text_cell(ui, &format!("{:.6}", val), true, false);
                        widget_cell(ui, false, |ui| {
                            ui.checkbox(&mut uf.show_formula, "");
                        });
                        widget_cell(ui, false, |ui| {
                            ui.checkbox(&mut uf.show_diff, "");
                        });
                        widget_cell(ui, false, |ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut uf.show_cumul, "");
                                if ui.small_button("x").clicked() {
                                    remove_uf = Some(u_idx);
                                }
                            });
                        });
                        ui.end_row();
                    }
                    if let Some(rm) = remove_uf {
                        self.user_formulas.remove(rm);
                    }
                });
        });
    }

    fn render_results_sheet(&mut self, ui: &mut egui::Ui) {
        let Some(ref res) = self.result else {
            ui.label("Click Calculate in the Parameters panel to compute equilibrium results.");
            return;
        };
        let Some(first) = res.points.first() else {
            return;
        };
        let scale = self.params.input_scale_to_mol.max(1e-12);
        let n_sp = first.species.len();

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("equil_results1_excel_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    // Col letters
                    row_num_cell(ui, "");
                    col_letter(ui, "A");
                    col_letter(ui, "B");
                    col_letter(ui, "C");
                    col_letter(ui, "D");
                    for i in 0..n_sp {
                        col_letter(ui, &col_name(4 + i));
                    }
                    col_letter(ui, &col_name(4 + n_sp));
                    col_letter(ui, &col_name(5 + n_sp));
                    col_letter(ui, &col_name(6 + n_sp));
                    ui.end_row();

                    // Row 1: Type
                    row_num_cell(ui, "1");
                    section_cell(ui, "Type");
                    header_cell(ui, "Step");
                    header_cell(ui, "T");
                    header_cell(ui, "P");
                    for _ in 0..n_sp {
                        header_cell(ui, "Equil");
                    }
                    header_cell(ui, "Gmin/RT");
                    header_cell(ui, "Reaction H");
                    header_cell(ui, "Volume");
                    ui.end_row();

                    // Row 2: TargetSpecies
                    row_num_cell(ui, "2");
                    section_cell(ui, "TargetSpecies");
                    section_cell(ui, "");
                    section_cell(ui, "");
                    section_cell(ui, "");
                    for sp in &first.species {
                        header_cell(ui, &sp.formula);
                    }
                    section_cell(ui, "");
                    section_cell(ui, "");
                    section_cell(ui, "");
                    ui.end_row();

                    // Row 3: Units
                    row_num_cell(ui, "3");
                    section_cell(ui, "Units");
                    section_cell(ui, "");
                    section_cell(ui, "°C");
                    section_cell(ui, "bar");
                    for _ in 0..n_sp {
                        section_cell(ui, "kmol");
                    }
                    section_cell(ui, "-");
                    section_cell(ui, "kJ");
                    section_cell(ui, "m3");
                    ui.end_row();

                    // Data rows
                    for (idx, pt) in res.points.iter().enumerate() {
                        row_num_cell(ui, &(4 + idx).to_string());
                        text_cell(ui, "", false, false);
                        text_cell(ui, &format!("{:.3}", pt.step_index as f64), true, false);
                        text_cell(ui, &format!("{:.2}", pt.temp_k - 273.15), true, false);
                        text_cell(ui, &format!("{:.3}", pt.pressure_bar), true, false);
                        for sp in &pt.species {
                            text_cell(
                                ui,
                                &format!("{:.4e}", sp.equil_amount_mol / scale),
                                true,
                                false,
                            );
                        }
                        text_cell(ui, &format!("{:.4}", pt.g_min_rt), true, false);
                        text_cell(ui, &format!("{:.3}", pt.reaction_enthalpy_kj), true, false);
                        text_cell(ui, &format!("{:.4e}", pt.volume_m3), true, false);
                        ui.end_row();
                    }
                });
        });
    }

    fn render_target_results_sheet(&mut self, ui: &mut egui::Ui) {
        let Some(sum) = self
            .result
            .as_ref()
            .and_then(|r| r.target_summary.as_ref())
        else {
            ui.label("Enable Target Calculations and click Calculate to view Target Results.");
            return;
        };
        let scale = self.params.input_scale_to_mol.max(1e-12);
        let pt = &sum.point;

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("equil_target_results_grid")
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    row_num_cell(ui, "");
                    for c in 0..6 {
                        col_letter(ui, &((b'A' + c as u8) as char).to_string());
                    }
                    ui.end_row();

                    let rows = [
                        ("Sought-for function", sum.target_species.as_str(), "Equilibrium Amount"),
                        ("Sought-for value", "kmol", &format!("{:.4}", sum.sought_value_kmol)),
                        ("Results found", "", "1"),
                        ("Achieved value", "kmol", &format!("{:.4}", sum.achieved_value_kmol)),
                        ("Variable step parameters", "", ""),
                        (&sum.variable_name, "", &format!("{:.4}", sum.solved_variable_value)),
                        ("Constant parameters", "", ""),
                        ("Pressure", "bar", &format!("{:.3}", pt.pressure_bar)),
                        ("Calculated results", "", ""),
                        ("Volume", "m3", &format!("{:.5e}", pt.volume_m3)),
                        ("Reaction Enthalpy", "MJ", &format!("{:.6}", pt.reaction_enthalpy_kj / 1000.0)),
                    ];
                    for (idx, (a, b, c)) in rows.iter().enumerate() {
                        row_num_cell(ui, &(1 + idx).to_string());
                        section_cell(ui, a);
                        text_cell(ui, b, false, false);
                        text_cell(ui, c, true, false);
                        text_cell(ui, "", false, false);
                        text_cell(ui, "", false, false);
                        text_cell(ui, "", false, false);
                        ui.end_row();
                    }

                    row_num_cell(ui, "13");
                    header_cell(ui, "Species");
                    header_cell(ui, "Phase");
                    header_cell(ui, "Equil Amount (kmol)");
                    header_cell(ui, "Mol Fraction");
                    header_cell(ui, "Activity Coeff");
                    header_cell(ui, "Activity");
                    ui.end_row();

                    for (idx, sp) in pt.species.iter().enumerate() {
                        row_num_cell(ui, &(14 + idx).to_string());
                        text_cell(ui, &sp.formula, false, false);
                        text_cell(ui, &sp.phase_name, false, false);
                        text_cell(ui, &format!("{:.6e}", sp.equil_amount_mol / scale), true, false);
                        text_cell(ui, &format!("{:.6}", sp.mole_fraction), true, false);
                        text_cell(ui, &format!("{:.4}", sp.activity_coeff), true, false);
                        text_cell(ui, &format!("{:.6}", sp.activity), true, false);
                        ui.end_row();
                    }
                });
        });
    }

    fn render_parameters_panel(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Parameters");
            ui.separator();

            // Operations
            egui::Frame::group(ui.style())
                .fill(egui::Color32::WHITE)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.strong("Operations");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let calc_btn = egui::Button::new(
                            egui::RichText::new("Calculate")
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(29, 78, 216))
                        .min_size(egui::vec2(96.0, 30.0));
                        if ui.add(calc_btn).clicked() {
                            self.request_calculate(db);
                        }
                        if ui
                            .add(egui::Button::new("Cancel").min_size(egui::vec2(62.0, 30.0)))
                            .clicked()
                        {
                            self.warnings_pending_calc = false;
                            self.show_warnings_modal = false;
                            self.last_error = None;
                        }
                        if ui
                            .add_enabled(
                                self.result.is_some(),
                                egui::Button::new("Show Chart (2D/3D)")
                                    .min_size(egui::vec2(132.0, 30.0)),
                            )
                            .clicked()
                        {
                            self.show_chart_window = true;
                        }
                    });
                    if let Some(ref res) = self.result {
                        if let Some(pt) = res.points.last() {
                            ui.add_space(3.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Points: {} | Last iters: {} | Max err: {:.2e}",
                                    res.points.len(),
                                    pt.iterations,
                                    pt.max_mass_balance_err
                                ))
                                .small()
                                .color(egui::Color32::from_rgb(22, 101, 52)),
                            );
                        }
                    }
                });

            // System Parameters
            egui::CollapsingHeader::new("System Parameters")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Chart header:");
                        ui.text_edit_singleline(&mut self.params.chart_header);
                        if self.params.chart_header.chars().count() > 80 {
                            self.params.chart_header =
                                self.params.chart_header.chars().take(80).collect();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Calculation Mode:");
                        egui::ComboBox::from_id_salt("calc_mode_cb")
                            .selected_text(self.params.calc_mode.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.params.calc_mode,
                                    CalculationMode::Normal,
                                    "Normal",
                                );
                                ui.selectable_value(
                                    &mut self.params.calc_mode,
                                    CalculationMode::InfinitePhase,
                                    "Infinite Phase",
                                );
                                ui.selectable_value(
                                    &mut self.params.calc_mode,
                                    CalculationMode::FixedActivity,
                                    "Fixed Activity",
                                );
                                ui.selectable_value(
                                    &mut self.params.calc_mode,
                                    CalculationMode::CellMode,
                                    "Cell Mode",
                                );
                            });
                    });
                    if self.params.calc_mode == CalculationMode::InfinitePhase
                        && !self.system.phases.is_empty()
                    {
                        let cur_inf = self
                            .params
                            .infinite_phase_idx
                            .unwrap_or(0)
                            .min(self.system.phases.len() - 1);
                        self.params.infinite_phase_idx = Some(cur_inf);
                        ui.horizontal(|ui| {
                            ui.label("Infinite Phase:");
                            egui::ComboBox::from_id_salt("infinite_phase_cb")
                                .selected_text(&self.system.phases[cur_inf].name)
                                .show_ui(ui, |ui| {
                                    for (idx, p) in self.system.phases.iter().enumerate() {
                                        if ui
                                            .selectable_label(
                                                self.params.infinite_phase_idx == Some(idx),
                                                format!("#{} {}", idx + 1, p.name),
                                            )
                                            .clicked()
                                        {
                                            self.params.infinite_phase_idx = Some(idx);
                                        }
                                    }
                                });
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label("Step Mode:");
                        egui::ComboBox::from_id_salt("step_mode_cb")
                            .selected_text(self.params.step_mode.label())
                            .show_ui(ui, |ui| {
                                for m in [
                                    StepMode::AddAmount,
                                    StepMode::FinalAmount,
                                    StepMode::RemovePercent,
                                    StepMode::RemovePercentFinalAmount,
                                    StepMode::ReactorStandard,
                                    StepMode::ReactorSheetsView,
                                ] {
                                    ui.selectable_value(&mut self.params.step_mode, m, m.label());
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Parameters:");
                        egui::ComboBox::from_id_salt("thermo_mode_cb")
                            .selected_text(self.params.thermo_mode.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.params.thermo_mode,
                                    ThermodynamicMode::Ntp,
                                    "N,T,P",
                                );
                                ui.selectable_value(
                                    &mut self.params.thermo_mode,
                                    ThermodynamicMode::Nhp,
                                    "N,H,P (Adiabatic)",
                                );
                                ui.selectable_value(
                                    &mut self.params.thermo_mode,
                                    ThermodynamicMode::Npv,
                                    "N,P,V (Const Volume)",
                                );
                                ui.selectable_value(
                                    &mut self.params.thermo_mode,
                                    ThermodynamicMode::Ntpq,
                                    "N,T,P,Q (Cell)",
                                );
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.params.allow_titrations, "Allow titrations");
                        ui.checkbox(&mut self.params.allow_constraints, "Allow constraints");
                    });
                    if self.params.allow_titrations {
                        ui.horizontal(|ui| {
                            ui.label("Soln density (kg/l):");
                            ui.add(
                                egui::DragValue::new(&mut self.params.titration_density_kg_l)
                                    .speed(0.01)
                                    .range(0.5..=3.0)
                                    .fixed_decimals(3),
                            );
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label("Indep. variables:");
                        ui.radio_value(&mut self.params.num_indep_vars, 0, "0");
                        ui.radio_value(&mut self.params.num_indep_vars, 1, "1");
                        if ui
                            .radio_value(&mut self.params.num_indep_vars, 2, "2")
                            .changed()
                        {
                            self.ensure_2d_sweep_defaults();
                        }
                    });
                    if self.params.num_indep_vars >= 1 {
                        let mut primary_changed = false;
                        ui.horizontal(|ui| {
                            ui.label("Primary var (X):");
                            egui::ComboBox::from_id_salt("primary_var_cb")
                                .selected_text(self.params.primary_var.label())
                                .show_ui(ui, |ui| {
                                    for v in [
                                        SweepVariable::TemperatureC,
                                        SweepVariable::PressureBar,
                                        SweepVariable::XAmount,
                                        SweepVariable::EnthalpyMj,
                                        SweepVariable::VolumeM3,
                                        SweepVariable::DischargeLevelKmolE,
                                    ] {
                                        if ui
                                            .selectable_value(
                                                &mut self.params.primary_var,
                                                v,
                                                v.label(),
                                            )
                                            .changed()
                                        {
                                            primary_changed = true;
                                        }
                                    }
                                });
                        });
                        if primary_changed && self.params.num_indep_vars == 2 {
                            self.ensure_2d_sweep_defaults();
                        }
                    }
                    if self.params.num_indep_vars == 2 {
                        if self.params.secondary_var.is_none()
                            || self.params.secondary_var == Some(self.params.primary_var)
                        {
                            self.ensure_2d_sweep_defaults();
                        }
                        let cur_sec = self
                            .params
                            .secondary_var
                            .unwrap_or(SweepVariable::PressureBar);
                        let mut sec_changed = false;
                        ui.horizontal(|ui| {
                            ui.label("Secondary var (Z):");
                            egui::ComboBox::from_id_salt("secondary_var_cb")
                                .selected_text(cur_sec.label())
                                .show_ui(ui, |ui| {
                                    for v in [
                                        SweepVariable::TemperatureC,
                                        SweepVariable::PressureBar,
                                        SweepVariable::ZAmount,
                                        SweepVariable::EnthalpyMj,
                                        SweepVariable::VolumeM3,
                                        SweepVariable::DischargeLevelKmolE,
                                    ] {
                                        ui.add_enabled_ui(v != self.params.primary_var, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.params.secondary_var == Some(v),
                                                    v.label(),
                                                )
                                                .clicked()
                                            {
                                                self.params.secondary_var = Some(v);
                                                sec_changed = true;
                                            }
                                        });
                                    }
                                });
                        });
                        if sec_changed {
                            self.ensure_2d_sweep_defaults();
                        }
                    } else {
                        self.params.secondary_var = None;
                    }
                });

            // System State Table (Initial | Final | Steps | Log)
            egui::CollapsingHeader::new("System State")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("sys_state_grid").show(ui, |ui| {
                        ui.label("");
                        ui.strong("Initial");
                        ui.strong("Final");
                        ui.strong("Steps");
                        ui.strong("Log");
                        ui.end_row();

                        ui.label("X-amount");
                        ui.label("-");
                        ui.label("-");
                        ui.add(egui::DragValue::new(&mut self.params.x_steps).range(2..=200));
                        ui.label("");
                        ui.end_row();

                        ui.label("Z-amount");
                        ui.label("-");
                        ui.label("-");
                        ui.add(egui::DragValue::new(&mut self.params.z_steps).range(2..=200));
                        ui.label("");
                        ui.end_row();

                        ui.label("T (°C)");
                        ui.add(egui::DragValue::new(&mut self.params.temp_range.initial).speed(1.0));
                        ui.add(egui::DragValue::new(&mut self.params.temp_range.final_val).speed(1.0));
                        ui.add(egui::DragValue::new(&mut self.params.temp_range.steps).range(1..=500));
                        ui.checkbox(&mut self.params.temp_range.log_scale, "");
                        ui.end_row();

                        ui.label("P (bar)");
                        ui.add(egui::DragValue::new(&mut self.params.pressure_range.initial).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.params.pressure_range.final_val).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.params.pressure_range.steps).range(1..=500));
                        ui.checkbox(&mut self.params.pressure_range.log_scale, "");
                        ui.end_row();

                        ui.label("H (MJ)");
                        ui.add(egui::DragValue::new(&mut self.params.enthalpy_range.initial).speed(10.0));
                        ui.add(egui::DragValue::new(&mut self.params.enthalpy_range.final_val).speed(10.0));
                        ui.add(egui::DragValue::new(&mut self.params.enthalpy_range.steps).range(1..=500));
                        ui.checkbox(&mut self.params.enthalpy_range.log_scale, "");
                        ui.end_row();

                        ui.label("V (m3)");
                        ui.add(
                            egui::DragValue::new(&mut self.params.volume_range.initial)
                                .speed(0.05)
                                .range(1e-6..=1e9),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.params.volume_range.final_val)
                                .speed(0.05)
                                .range(1e-6..=1e9),
                        );
                        ui.add(egui::DragValue::new(&mut self.params.volume_range.steps).range(1..=500));
                        ui.checkbox(&mut self.params.volume_range.log_scale, "");
                        ui.end_row();

                        ui.label("Q (kmol e-)");
                        ui.add(egui::DragValue::new(&mut self.params.discharge_range.initial).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.params.discharge_range.final_val).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.params.discharge_range.steps).range(1..=500));
                        ui.checkbox(&mut self.params.discharge_range.log_scale, "");
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.params.use_as_base_volume, "Use as base volume");
                    });
                    if self.params.use_as_base_volume {
                        ui.horizontal(|ui| {
                            ui.label("Initial Pressure (bar):");
                            ui.add(
                                egui::DragValue::new(&mut self.params.initial_pressure_bar)
                                    .speed(0.1)
                                    .range(1e-4..=1000.0),
                            );
                        });
                    }
                });

            // Phase Data (Fig. 6 & Fig. 41)
            egui::CollapsingHeader::new("Phase Data")
                .default_open(true)
                .show(ui, |ui| {
                    let has_non_ideal = self.system.phases.iter().any(|p| {
                        p.model != SolutionModel::IdealMixture && p.model != SolutionModel::PureSubstance
                    });
                    if !self.system.phases.is_empty() {
                        let p_idx = self
                            .selected_phase_idx
                            .min(self.system.phases.len().saturating_sub(1));
                        let ph = &mut self.system.phases[p_idx];
                        ui.horizontal(|ui| {
                            ui.label("Selected Phase:");
                            ui.strong(&ph.name);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Solution Model:");
                            egui::ComboBox::from_id_salt("phase_data_model_cb")
                                .selected_text(ph.model.label())
                                .show_ui(ui, |ui| {
                                    for m in [
                                        SolutionModel::IdealMixture,
                                        SolutionModel::PureSubstance,
                                        SolutionModel::Aqua,
                                        SolutionModel::GaAsMixture,
                                        SolutionModel::AlZnMixture,
                                    ] {
                                        ui.selectable_value(&mut ph.model, m, m.label());
                                    }
                                });
                        });
                        if self.params.calc_mode == CalculationMode::CellMode {
                            ui.horizontal(|ui| {
                                ui.label("Electrode:");
                                egui::ComboBox::from_id_salt("phase_electrode_cb")
                                    .selected_text(if ph.electrode_role.is_empty() {
                                        "None"
                                    } else {
                                        ph.electrode_role.as_str()
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut ph.electrode_role,
                                            String::new(),
                                            "None",
                                        );
                                        ui.selectable_value(
                                            &mut ph.electrode_role,
                                            "anode".to_string(),
                                            "Anode",
                                        );
                                        ui.selectable_value(
                                            &mut ph.electrode_role,
                                            "cathode".to_string(),
                                            "Cathode",
                                        );
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Capacitance (F):");
                                ui.add(
                                    egui::DragValue::new(&mut ph.capacitance_farads)
                                        .speed(0.1)
                                        .range(0.0..=1e6),
                                );
                            });
                        }
                    }
                    ui.horizontal(|ui| {
                        ui.label("AC Steps:");
                        ui.add_enabled(
                            has_non_ideal,
                            egui::DragValue::new(&mut self.params.ac_steps).range(1..=50),
                        );
                    });
                });

            // Calculation Options
            egui::CollapsingHeader::new("Calculation Options")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(
                        &mut self.params.mixing_entropy_aqua,
                        "Mixing Entropy conversion for aqueous species",
                    );
                    ui.checkbox(&mut self.params.criss_cobble, "Criss-Cobble");
                });

            // Target Calculations
            egui::CollapsingHeader::new("Target Calculations")
                .default_open(false)
                .show(ui, |ui| {
                    ui.checkbox(&mut self.params.target.enabled, "Enabled");
                    ui.horizontal(|ui| {
                        ui.label("Target Species:");
                        ui.text_edit_singleline(&mut self.params.target.target_species);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Target Value (kmol):");
                        ui.add(
                            egui::DragValue::new(&mut self.params.target.target_amount_kmol)
                                .speed(0.01),
                        );
                    });
                });
        });
    }

    fn point_x_value(
        &self,
        p: &soryntel_thermo::equilibrium::EquilibriumPointResult,
        scale: f64,
    ) -> f64 {
        match self.chart_x_kind {
            ChartXKind::StepVariable => p.x_value,
            ChartXKind::StepIndex => p.step_index as f64,
            ChartXKind::TemperatureC => p.temp_k - 273.15,
            ChartXKind::TemperatureK => p.temp_k,
            ChartXKind::PressureBar => p.pressure_bar,
            ChartXKind::VolumeM3 => p.volume_m3,
            ChartXKind::ReactionEnthalpyMj => p.reaction_enthalpy_kj / 1000.0,
            ChartXKind::Ph => p.ph.unwrap_or(7.0),
            ChartXKind::DischargeKmolE => p.discharge_kmol_e,
            ChartXKind::SpeciesEquilAmount(idx) => p
                .species
                .get(idx)
                .map(|s| s.equil_amount_mol / scale)
                .unwrap_or(0.0),
        }
    }

    fn point_y_value(
        &self,
        p: &soryntel_thermo::equilibrium::EquilibriumPointResult,
        s_idx: usize,
        scale: f64,
    ) -> f64 {
        let sp = &p.species[s_idx];
        match self.chart_y_kind {
            ChartYKind::EquilibriumAmount => sp.equil_amount_mol / scale,
            ChartYKind::MoleFraction => sp.mole_fraction,
            ChartYKind::Activity => sp.activity,
            ChartYKind::ActivityCoeff => sp.activity_coeff,
            ChartYKind::ReactionEnthalpy => p.reaction_enthalpy_kj / 1000.0,
            ChartYKind::CellVoltage => p.cell_voltage_v.unwrap_or(0.0),
            ChartYKind::Ph => p.ph.unwrap_or(7.0),
            ChartYKind::UserFormula => sp.equil_amount_mol / scale,
        }
    }

    fn render_modals(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        // From Elements Modal (Figs. 3-5)
        if self.show_from_elements {
            let mut open = true;
            egui::Window::new("From Elements (Database Browser & Sort Species)")
                .open(&mut open)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Elements (space/comma sep):");
                        ui.text_edit_singleline(&mut self.elem_query);
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.elem_gas, "Gases");
                        ui.checkbox(&mut self.elem_liquid, "Liquids");
                        ui.checkbox(&mut self.elem_solid, "Condensed");
                        ui.checkbox(&mut self.elem_aq_ion, "Aqua Ions");
                        ui.checkbox(&mut self.elem_aq_neutral, "Aqua Neutrals");
                    });
                    ui.separator();
                    ui.strong("Sort Species Options:");
                    ui.checkbox(&mut self.elem_add_n2, "Add N2(g) for calculation stability");
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(&mut self.elem_sort_mode, 0, "Phases");
                        ui.radio_value(&mut self.elem_sort_mode, 1, "Gas, Aqua, Pure");
                        ui.radio_value(&mut self.elem_sort_mode, 2, "No sorting");
                        ui.radio_value(&mut self.elem_sort_mode, 3, "Gas, Aqua, Liquids");
                    });

                    let elems: Vec<String> = self
                        .elem_query
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let matches = db.search_by_elements(
                        &elems,
                        self.elem_gas,
                        self.elem_liquid,
                        self.elem_solid,
                        self.elem_aq_ion,
                        self.elem_aq_neutral,
                        200,
                    );
                    ui.label(format!("Matching Species in Maindb6: {}", matches.len()));
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        for sp in &matches {
                            ui.label(format!("{} — {} ({:?})", sp.formula, sp.name, sp.phase));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Import & Build Phases").clicked() {
                            self.system.phases.clear();
                            if self.elem_sort_mode == 2 {
                                // No sorting: all species in a single Phase #1
                                let mut all_sp: Vec<EquilibriumSpeciesInput> =
                                    matches.iter().map(|sp| Self::make_sp(&sp.formula, 0.0)).collect();
                                if self.elem_add_n2
                                    && !all_sp.iter().any(|s| s.formula == "N2(g)")
                                {
                                    all_sp.insert(0, Self::make_sp("N2(g)", 1e-10));
                                }
                                self.system.phases.push(EquilibriumPhase {
                                    name: "Phase #1".to_string(),
                                    model: SolutionModel::IdealMixture,
                                    is_gas_phase: false,
                                    electrode_role: String::new(),
                                    capacitance_farads: 0.0,
                                    species: all_sp,
                                });
                            } else {
                                let mut gas = Vec::new();
                                let mut aqua = Vec::new();
                                let mut liq = Vec::new();
                                let mut sol = Vec::new();
                                for sp in &matches {
                                    let input = Self::make_sp(&sp.formula, 0.0);
                                    match sp.phase {
                                        Phase::Gas => gas.push(input),
                                        Phase::AqueousIon | Phase::AqueousNeutral => aqua.push(input),
                                        Phase::Liquid => {
                                            if self.elem_sort_mode == 0 || self.elem_sort_mode == 3
                                            {
                                                liq.push(input);
                                            } else {
                                                sol.push(input);
                                            }
                                        }
                                        Phase::Solid => sol.push(input),
                                    }
                                }
                                if self.elem_add_n2 && !gas.iter().any(|s| s.formula == "N2(g)") {
                                    gas.insert(0, Self::make_sp("N2(g)", 1e-10));
                                }
                                if !gas.is_empty() {
                                    self.system.phases.push(EquilibriumPhase {
                                        name: "GAS".to_string(),
                                        model: SolutionModel::IdealMixture,
                                        is_gas_phase: true,
                                        electrode_role: String::new(),
                                        capacitance_farads: 0.0,
                                        species: gas,
                                    });
                                }
                                if !aqua.is_empty() {
                                    self.system.phases.push(EquilibriumPhase {
                                        name: "AQUA".to_string(),
                                        model: SolutionModel::Aqua,
                                        is_gas_phase: false,
                                        electrode_role: String::new(),
                                        capacitance_farads: 0.0,
                                        species: aqua,
                                    });
                                }
                                if !liq.is_empty() {
                                    self.system.phases.push(EquilibriumPhase {
                                        name: "LIQUID".to_string(),
                                        model: SolutionModel::IdealMixture,
                                        is_gas_phase: false,
                                        electrode_role: String::new(),
                                        capacitance_farads: 0.0,
                                        species: liq,
                                    });
                                }
                                if !sol.is_empty() {
                                    self.system.phases.push(EquilibriumPhase {
                                        name: "SOLID".to_string(),
                                        model: if self.elem_sort_mode == 1 {
                                            SolutionModel::PureSubstance
                                        } else {
                                            SolutionModel::IdealMixture
                                        },
                                        is_gas_phase: false,
                                        electrode_role: String::new(),
                                        capacitance_farads: 0.0,
                                        species: sol,
                                    });
                                }
                            }
                            self.show_from_elements = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_from_elements = false;
                        }
                    });
                });
            if !open {
                self.show_from_elements = false;
            }
        }

        // Warnings Modal (Figs. 35-38)
        if self.show_warnings_modal {
            let mut open = true;
            egui::Window::new("Warnings — Cp Extrapolation Check")
                .open(&mut open)
                .default_width(740.0)
                .show(ctx, |ui| {
                    ui.strong("HSC-data will be extrapolated outside available temperature range.");
                    let mut all_checked = !self.warnings.is_empty()
                        && self.warnings.iter().all(|w| {
                            self.system.phases[w.phase_idx].species[w.species_idx].excluded
                        });
                    if ui
                        .checkbox(&mut all_checked, "All Species (Remove when outside Tmin..Tmax)")
                        .changed()
                    {
                        for w in &self.warnings {
                            self.system.phases[w.phase_idx].species[w.species_idx].excluded =
                                all_checked;
                        }
                    }
                    egui::ScrollArea::both().max_height(320.0).show(ui, |ui| {
                        egui::Grid::new("warnings_excel_grid")
                            .spacing([0.0, 0.0])
                            .show(ui, |ui| {
                                row_num_cell(ui, "");
                                for c in 0..8 {
                                    col_letter(ui, &((b'A' + c as u8) as char).to_string());
                                }
                                ui.end_row();
                                row_num_cell(ui, "4");
                                header_cell(ui, "Remove");
                                header_cell(ui, "Species");
                                header_cell(ui, "Sel T1 (°C)");
                                header_cell(ui, "Sel T2 (°C)");
                                header_cell(ui, "Avail Tmin");
                                header_cell(ui, "Avail Tmax");
                                header_cell(ui, "Reference");
                                header_cell(ui, "Feed T (°C)");
                                ui.end_row();

                                for (idx, w) in self.warnings.iter().enumerate() {
                                    row_num_cell(ui, &(5 + idx).to_string());
                                    widget_cell(ui, false, |ui| {
                                        let excl = &mut self.system.phases[w.phase_idx].species
                                            [w.species_idx]
                                            .excluded;
                                        ui.checkbox(excl, "");
                                    });
                                    text_cell(ui, &w.formula, false, false);
                                    widget_cell(ui, false, |ui| {
                                        let col = if w.out_of_range_low {
                                            egui::Color32::RED
                                        } else {
                                            egui::Color32::BLACK
                                        };
                                        ui.colored_label(col, format!("{:.2}", w.selected_t1_c));
                                    });
                                    widget_cell(ui, false, |ui| {
                                        let col = if w.out_of_range_high {
                                            egui::Color32::RED
                                        } else {
                                            egui::Color32::BLACK
                                        };
                                        ui.colored_label(col, format!("{:.2}", w.selected_t2_c));
                                    });
                                    text_cell(ui, &format!("{:.2}", w.available_t_min_c), true, false);
                                    text_cell(ui, &format!("{:.2}", w.available_t_max_c), true, false);
                                    text_cell(ui, &w.reference, false, false);
                                    text_cell(ui, &format!("{:.1}", w.feed_temp_c), true, false);
                                    ui.end_row();
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.dont_show_warnings, "Do not show this dialog");
                        if ui.button("OK").clicked() {
                            self.show_warnings_modal = false;
                            if self.warnings_pending_calc {
                                self.warnings_pending_calc = false;
                                self.run_calculate(db);
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.warnings_pending_calc = false;
                            self.show_warnings_modal = false;
                        }
                    });
                });
            if !open {
                self.warnings_pending_calc = false;
                self.show_warnings_modal = false;
            }
        }

        // Add-Ins Modal (Fig. 46)
        if self.show_addins_modal {
            let mut open = true;
            egui::Window::new("Window -> Add-Ins (Activity Coefficient Models)")
                .open(&mut open)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.strong("Installed Solution Model Add-Ins:");
                    ui.separator();
                    ui.label("1. Al - Zn Mixture (Appendix C Redlich-Kister: Liquid, Solid FCC, Solid HCP)");
                    ui.label("2. Ga - As Mixture (Appendix A & B Redlich-Kister: Liquid GaAs)");
                    ui.label("3. Aqua (Extended Debye-Huckel + 55.508435 molality conversion)");
                    ui.label("4. Column-C Formula Evaluator (T, X_row, EXP, LN, LOG10, ln(f))");
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_addins_modal = false;
                    }
                });
            if !open {
                self.show_addins_modal = false;
            }
        }

        // Chart Window (Figs. 16, 19, 25, 27-31, 43-44)
        if self.show_chart_window {
            let mut open = true;
            egui::Window::new(format!("Chart [{}]", self.params.chart_header))
                .open(&mut open)
                .resizable(true)
                .default_size([1020.0, 620.0])
                .min_width(680.0)
                .min_height(440.0)
                .show(ctx, |ui| {
                    let has_z_data = self
                        .result
                        .as_ref()
                        .map(|r| r.points.iter().any(|p| p.z_value.is_some()))
                        .unwrap_or(false);

                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Select Data for Axes...").clicked() {
                            self.show_axes_modal = true;
                        }
                        if ui.button("Show...Table (Equilibrium Results)").clicked() {
                            self.show_diagram_table = true;
                        }
                        ui.separator();
                        if ui
                            .checkbox(&mut self.chart_auto_axes, "Auto Axes")
                            .changed()
                            && self.chart_auto_axes
                        {
                            self.reset_chart_bounds_from_result();
                        }
                        if !self.chart_auto_axes {
                            ui.label("X:");
                            ui.add(egui::DragValue::new(&mut self.chart_x_min).speed(1.0));
                            ui.label("..");
                            ui.add(egui::DragValue::new(&mut self.chart_x_max).speed(1.0));
                            ui.label("Y:");
                            ui.add(egui::DragValue::new(&mut self.chart_y_min).speed(0.1));
                            ui.label("..");
                            ui.add(egui::DragValue::new(&mut self.chart_y_max).speed(0.1));
                        }
                        ui.checkbox(&mut self.chart_log_y, "Log Y");
                        ui.checkbox(&mut self.chart_show_curve_labels, "Labels");
                        ui.checkbox(&mut self.chart_smooth, "Smooth");
                        if has_z_data {
                            ui.separator();
                            ui.selectable_value(&mut self.chart_is_3d, false, "2D Slice");
                            ui.selectable_value(&mut self.chart_is_3d, true, "3D Surface");
                            if self.chart_is_3d {
                                ui.checkbox(&mut self.chart_color_by_levels, "Color by levels");
                                ui.checkbox(&mut self.chart_3d_show_legend, "Show Legend");
                                if ui.button("Reset View").clicked() {
                                    self.chart_3d_yaw = -0.52;
                                    self.chart_3d_pitch = 0.42;
                                }
                            }
                        }
                    });
                    ui.separator();

                    let Some(res) = self.result.clone() else {
                        ui.label("No calculation results.");
                        return;
                    };
                    let Some(first) = res.points.first() else {
                        return;
                    };
                    let scale = self.params.input_scale_to_mol.max(1e-12);
                    let x_sp_name = match self.chart_x_kind {
                        ChartXKind::SpeciesEquilAmount(i) => {
                            first.species.get(i).map(|s| s.formula.as_str())
                        }
                        _ => None,
                    };
                    let x_label = self
                        .chart_x_kind
                        .label(self.params.primary_var.label(), x_sp_name);
                    let y_label = if self.chart_log_y {
                        format!("log10({})", self.chart_y_kind.label())
                    } else {
                        self.chart_y_kind.label().to_string()
                    };

                    // Right legend panel (~265 px) with color swatches, species list, element chips & phase filter
                    if !self.chart_is_3d || self.chart_3d_show_legend {
                        egui::SidePanel::right("equil_chart_legend_panel")
                            .resizable(true)
                            .default_width(265.0)
                            .min_width(200.0)
                            .show_inside(ui, |ui| {
                                ui.strong("Chart parameters — Legend");
                                ui.horizontal(|ui| {
                                    if ui.small_button("All").clicked() {
                                        self.chart_hidden_species.clear();
                                    }
                                    if ui.small_button("None").clicked() {
                                        for sp0 in &first.species {
                                            self.chart_hidden_species.insert(sp0.formula.clone());
                                        }
                                    }
                                    ui.label("Filter:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.chart_elem_filter)
                                            .desired_width(78.0),
                                    );
                                    if ui.small_button("Reset").clicked() {
                                        self.chart_elem_filter = "All elements".to_string();
                                    }
                                });
                                ui.checkbox(
                                    &mut self.chart_elem_contains_all,
                                    "All elements must exist",
                                );
                                // Collect element symbols present in system for quick HSC-style chips
                                let mut elem_list: Vec<String> = Vec::new();
                                for ph in &self.system.phases {
                                    for sp_in in &ph.species {
                                        if let Ok(parsed) = soryntel_solver::parse_term(&sp_in.formula) {
                                            for el in parsed.elements.keys() {
                                                if el != "e-" && !elem_list.contains(el) {
                                                    elem_list.push(el.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                                if !elem_list.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        if ui
                                            .selectable_label(
                                                self.chart_elem_filter.eq_ignore_ascii_case("All elements"),
                                                "All elements",
                                            )
                                            .clicked()
                                        {
                                            self.chart_elem_filter = "All elements".to_string();
                                        }
                                        for el in &elem_list {
                                            let active = self.chart_elem_filter == *el;
                                            if ui.selectable_label(active, el).clicked() {
                                                self.chart_elem_filter = el.clone();
                                            }
                                        }
                                    });
                                }
                                ui.separator();
                                egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                                    for (s_idx, sp0) in first.species.iter().enumerate() {
                                        let mut visible =
                                            !self.chart_hidden_species.contains(&sp0.formula);
                                        ui.horizontal(|ui| {
                                            if ui.checkbox(&mut visible, "").changed() {
                                                if visible {
                                                    self.chart_hidden_species.remove(&sp0.formula);
                                                } else {
                                                    self.chart_hidden_species
                                                        .insert(sp0.formula.clone());
                                                }
                                            }
                                            let sw = species_palette_color(s_idx);
                                            let (r, _) = ui.allocate_exact_size(
                                                egui::vec2(11.0, 11.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(r, 1.5, sw);
                                            ui.label(format!(
                                                "{}  [P{}]",
                                                sp0.formula,
                                                sp0.phase_idx + 1
                                            ));
                                        });
                                    }
                                });
                                ui.separator();
                                ui.strong("Phases filter:");
                                if ui
                                    .selectable_label(self.chart_phase_filter.is_none(), "All phases")
                                    .clicked()
                                {
                                    self.chart_phase_filter = None;
                                }
                                for (p_idx, p) in self.system.phases.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            self.chart_phase_filter == Some(p_idx),
                                            format!("Phase #{} — {}", p_idx + 1, p.model.label()),
                                        )
                                        .clicked()
                                    {
                                        self.chart_phase_filter = Some(p_idx);
                                    }
                                }
                            });
                    }

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        if self.chart_is_3d && has_z_data {
                            render_3d_surface_chart(
                                ctx,
                                ui,
                                &res,
                                scale,
                                self.chart_y_kind,
                                &self.chart_hidden_species,
                                self.chart_phase_filter,
                                &self.chart_elem_filter,
                                self.chart_elem_contains_all,
                                &self.params.chart_header,
                                self.params.primary_var.label(),
                                self.params
                                    .secondary_var
                                    .map(|v| v.label())
                                    .unwrap_or("Z"),
                                &mut self.chart_3d_yaw,
                                &mut self.chart_3d_pitch,
                                self.chart_color_by_levels,
                                self.chart_smooth,
                                &mut self.chart_3d_cache,
                            );
                        } else {
                            let mut plot = Plot::new("equil_main_plot")
                                .legend(Legend::default())
                                .x_axis_label(x_label)
                                .y_axis_label(y_label);
                            if !self.chart_auto_axes {
                                plot = plot
                                    .include_x(self.chart_x_min)
                                    .include_x(self.chart_x_max)
                                    .include_y(self.chart_y_min)
                                    .include_y(self.chart_y_max);
                            }

                            plot.show(ui, |plot_ui| {
                                if !self.chart_auto_axes {
                                    plot_ui.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                                        [self.chart_x_min, self.chart_y_min],
                                        [self.chart_x_max, self.chart_y_max],
                                    ));
                                }
                                let transform_y = |v: f64| -> f64 {
                                    if self.chart_log_y {
                                        v.max(1e-20).log10()
                                    } else {
                                        v
                                    }
                                };
                                if self.chart_y_kind == ChartYKind::CellVoltage {
                                    let pts: PlotPoints = res
                                        .points
                                        .iter()
                                        .map(|p| {
                                            [
                                                self.point_x_value(p, scale),
                                                transform_y(p.cell_voltage_v.unwrap_or(0.0)),
                                            ]
                                        })
                                        .collect();
                                    plot_ui.line(Line::new(pts).name("Cell Voltage (V)"));
                                } else if self.chart_y_kind == ChartYKind::UserFormula {
                                    for uf in &self.user_formulas {
                                        let base_vals: Vec<(f64, f64)> = res
                                            .points
                                            .iter()
                                            .map(|p| {
                                                (
                                                    self.point_x_value(p, scale),
                                                    eval_user_formula(&uf.expr, p, scale),
                                                )
                                            })
                                            .collect();
                                        if uf.show_formula {
                                            let pts: PlotPoints = base_vals
                                                .iter()
                                                .map(|&(x, y)| [x, transform_y(y)])
                                                .collect();
                                            plot_ui.line(Line::new(pts).name(&uf.label));
                                        }
                                        if uf.show_diff {
                                            let mut prev =
                                                base_vals.first().map(|v| v.1).unwrap_or(0.0);
                                            let pts: PlotPoints = base_vals
                                                .iter()
                                                .map(|&(x, y)| {
                                                    let d = y - prev;
                                                    prev = y;
                                                    [x, transform_y(d)]
                                                })
                                                .collect();
                                            plot_ui.line(
                                                Line::new(pts)
                                                    .name(format!("{} (Diff)", uf.label)),
                                            );
                                        }
                                        if uf.show_cumul {
                                            let mut acc = 0.0;
                                            let pts: PlotPoints = base_vals
                                                .iter()
                                                .map(|&(x, y)| {
                                                    acc += y;
                                                    [x, transform_y(acc)]
                                                })
                                                .collect();
                                            plot_ui.line(
                                                Line::new(pts)
                                                    .name(format!("{} (Cumul)", uf.label)),
                                            );
                                        }
                                    }
                                } else {
                                    for (s_idx, sp0) in first.species.iter().enumerate() {
                                        if !species_matches_filters(
                                            sp0,
                                            &self.chart_hidden_species,
                                            self.chart_phase_filter,
                                            &self.chart_elem_filter,
                                            self.chart_elem_contains_all,
                                        ) {
                                            continue;
                                        }
                                        let mut max_pt: Option<[f64; 2]> = None;
                                        let pts_vec: Vec<[f64; 2]> = res
                                            .points
                                            .iter()
                                            .map(|p| {
                                                let x = self.point_x_value(p, scale);
                                                let y = transform_y(
                                                    self.point_y_value(p, s_idx, scale),
                                                );
                                                if max_pt
                                                    .map(|m| y > m[1])
                                                    .unwrap_or(true)
                                                {
                                                    max_pt = Some([x, y]);
                                                }
                                                [x, y]
                                            })
                                            .collect();
                                        let pts_smoothed = if self.chart_smooth && pts_vec.len() >= 3 {
                                            smooth_2d_curve_pchip(&pts_vec, 4)
                                        } else {
                                            pts_vec
                                        };
                                        plot_ui.line(
                                            Line::new(PlotPoints::from(pts_smoothed))
                                                .width(2.0)
                                                .color(species_palette_color(s_idx))
                                                .name(&sp0.formula),
                                        );
                                        if self.chart_show_curve_labels {
                                            if let Some([mx, my]) = max_pt {
                                                if my.abs() > 1e-6 || self.chart_log_y {
                                                    plot_ui.text(egui_plot::Text::new(
                                                        egui_plot::PlotPoint::new(mx, my),
                                                        sp0.formula.clone(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    });
                });
            if !open {
                self.show_chart_window = false;
            }
        }

        // Select Data for Axes Modal (Fig. 17 & Fig. 43)
        if self.show_axes_modal {
            let mut open = true;
            egui::Window::new("Select data for axes")
                .open(&mut open)
                .default_width(480.0)
                .show(ctx, |ui| {
                    ui.columns(2, |cols| {
                        cols[0].strong("X-Axis (Horizontal Axis):");
                        for (xk, lbl) in [
                            (ChartXKind::StepVariable, "Recommended (Step variable)"),
                            (ChartXKind::StepIndex, "Step index"),
                            (ChartXKind::TemperatureC, "Temperature (°C)"),
                            (ChartXKind::TemperatureK, "Temperature (K)"),
                            (ChartXKind::PressureBar, "Pressure (bar)"),
                            (ChartXKind::VolumeM3, "Volume (m3)"),
                            (ChartXKind::ReactionEnthalpyMj, "Reaction Enthalpy (MJ)"),
                            (ChartXKind::Ph, "pH"),
                            (ChartXKind::DischargeKmolE, "Discharge level (kmol e-)"),
                        ] {
                            if cols[0]
                                .radio_value(&mut self.chart_x_kind, xk, lbl)
                                .changed()
                                && self.chart_auto_axes
                            {
                                self.reset_chart_bounds_from_result();
                            }
                        }
                        let sp_formulas: Vec<String> = self
                            .result
                            .as_ref()
                            .and_then(|r| r.points.first())
                            .map(|f| f.species.iter().map(|s| s.formula.clone()).collect())
                            .unwrap_or_default();
                        if !sp_formulas.is_empty() {
                            cols[0].separator();
                            cols[0].label("Or Species Amount:");
                            egui::ScrollArea::vertical().max_height(120.0).show(
                                &mut cols[0],
                                |ui| {
                                    for (s_idx, formula) in sp_formulas.iter().enumerate() {
                                        if ui
                                            .radio_value(
                                                &mut self.chart_x_kind,
                                                ChartXKind::SpeciesEquilAmount(s_idx),
                                                formula,
                                            )
                                            .changed()
                                            && self.chart_auto_axes
                                        {
                                            self.reset_chart_bounds_from_result();
                                        }
                                    }
                                },
                            );
                        }

                        cols[1].strong("Y-Axis (Vertical Axis):");
                        for kind in [
                            ChartYKind::EquilibriumAmount,
                            ChartYKind::MoleFraction,
                            ChartYKind::Activity,
                            ChartYKind::ActivityCoeff,
                            ChartYKind::ReactionEnthalpy,
                            ChartYKind::CellVoltage,
                            ChartYKind::Ph,
                            ChartYKind::UserFormula,
                        ] {
                            if cols[1]
                                .radio_value(&mut self.chart_y_kind, kind, kind.label())
                                .changed()
                                && self.chart_auto_axes
                            {
                                self.reset_chart_bounds_from_result();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Finish").clicked() {
                        self.show_axes_modal = false;
                    }
                });
            if !open {
                self.show_axes_modal = false;
            }
        }

        // Equilibrium Results Transposed Table (Fig. 33)
        if self.show_diagram_table {
            let mut open = true;
            egui::Window::new("Equilibrium Results (Diagram Table)")
                .open(&mut open)
                .resizable(true)
                .default_size([860.0, 480.0])
                .show(ctx, |ui| {
                    if let Some(ref res) = self.result {
                        if let Some(first) = res.points.first() {
                            let scale = self.params.input_scale_to_mol.max(1e-12);
                            let cell_val = |kind: ChartYKind,
                                            p: &soryntel_thermo::equilibrium::EquilibriumPointResult,
                                            s_idx: usize|
                             -> f64 {
                                let sp = &p.species[s_idx];
                                match kind {
                                    ChartYKind::EquilibriumAmount => sp.equil_amount_mol / scale,
                                    ChartYKind::MoleFraction => sp.mole_fraction,
                                    ChartYKind::Activity => sp.activity,
                                    ChartYKind::ActivityCoeff => sp.activity_coeff,
                                    _ => sp.equil_amount_mol / scale,
                                }
                            };
                            ui.horizontal(|ui| {
                                ui.label("Property:");
                                egui::ComboBox::from_id_salt("diag_tbl_kind")
                                    .selected_text(self.diagram_table_kind.label())
                                    .show_ui(ui, |ui| {
                                        for k in [
                                            ChartYKind::EquilibriumAmount,
                                            ChartYKind::MoleFraction,
                                            ChartYKind::Activity,
                                            ChartYKind::ActivityCoeff,
                                        ] {
                                            ui.selectable_value(
                                                &mut self.diagram_table_kind,
                                                k,
                                                k.label(),
                                            );
                                        }
                                    });
                                let cur_kind = self.diagram_table_kind;
                                if ui.button("Copy All (TSV)").clicked() {
                                    let mut tsv = String::from("Species\tPhase\tMin\tMax");
                                    for p in &res.points {
                                        tsv.push_str(&format!("\tStep_{}", p.step_index));
                                    }
                                    tsv.push('\n');
                                    for (s_idx, sp0) in first.species.iter().enumerate() {
                                        let mut mn = f64::INFINITY;
                                        let mut mx = f64::NEG_INFINITY;
                                        for p in &res.points {
                                            let v = cell_val(cur_kind, p, s_idx);
                                            mn = mn.min(v);
                                            mx = mx.max(v);
                                        }
                                        tsv.push_str(&format!(
                                            "{}\t{}\t{:.4e}\t{:.4e}",
                                            sp0.formula,
                                            sp0.phase_idx + 1,
                                            mn,
                                            mx
                                        ));
                                        for p in &res.points {
                                            tsv.push_str(&format!(
                                                "\t{:.6e}",
                                                cell_val(cur_kind, p, s_idx)
                                            ));
                                        }
                                        tsv.push('\n');
                                    }
                                    ui.output_mut(|o| o.copied_text = tsv);
                                    self.file_status_msg =
                                        Some("Copied diagram table to clipboard.".to_string());
                                }
                            });
                            let cur_kind = self.diagram_table_kind;
                            ui.separator();

                            egui::ScrollArea::both().show(ui, |ui| {
                                egui::Grid::new("diag_table_grid")
                                    .spacing([0.0, 0.0])
                                    .show(ui, |ui| {
                                        row_num_cell(ui, "4");
                                        header_cell(ui, "Data Type");
                                        header_cell(ui, "Species");
                                        header_cell(ui, "Phase");
                                        header_cell(ui, "Min");
                                        header_cell(ui, "Max");
                                        for p in &res.points {
                                            header_cell(ui, &format!("{}", p.step_index));
                                        }
                                        ui.end_row();

                                        row_num_cell(ui, "5");
                                        section_cell(ui, "Temperature");
                                        section_cell(ui, "");
                                        section_cell(ui, "");
                                        section_cell(ui, "°C");
                                        section_cell(ui, "");
                                        for p in &res.points {
                                            section_cell(ui, &format!("{:.1}", p.temp_k - 273.15));
                                        }
                                        ui.end_row();

                                        for (s_idx, sp0) in first.species.iter().enumerate() {
                                            let mut mn = f64::INFINITY;
                                            let mut mx = f64::NEG_INFINITY;
                                            for p in &res.points {
                                                let v = cell_val(cur_kind, p, s_idx);
                                                mn = mn.min(v);
                                                mx = mx.max(v);
                                            }
                                            row_num_cell(ui, &(6 + s_idx).to_string());
                                            text_cell(
                                                ui,
                                                cur_kind.label(),
                                                false,
                                                false,
                                            );
                                            text_cell(ui, &sp0.formula, false, false);
                                            text_cell(
                                                ui,
                                                &(sp0.phase_idx + 1).to_string(),
                                                true,
                                                false,
                                            );
                                            text_cell(ui, &format!("{:.3e}", mn), true, false);
                                            text_cell(ui, &format!("{:.3e}", mx), true, false);
                                            for p in &res.points {
                                                let v = cell_val(cur_kind, p, s_idx);
                                                text_cell(ui, &format!("{:.4e}", v), true, false);
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                    }
                });
            if !open {
                self.show_diagram_table = false;
            }
        }
    }
}

fn eval_user_formula(
    expr: &str,
    pt: &soryntel_thermo::equilibrium::EquilibriumPointResult,
    scale: f64,
) -> f64 {
    let cleaned: String = expr
        .trim()
        .trim_start_matches('=')
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if cleaned.is_empty() {
        return 0.0;
    }
    // Tokenize into numbers, operators (+,-,*,/,(,)), D<row>, or species formulas
    struct UfParser<'a> {
        chars: Vec<char>,
        pos: usize,
        pt: &'a soryntel_thermo::equilibrium::EquilibriumPointResult,
        scale: f64,
    }
    impl<'a> UfParser<'a> {
        fn peek(&self) -> Option<char> {
            self.chars.get(self.pos).copied()
        }
        fn next(&mut self) -> Option<char> {
            let c = self.peek()?;
            self.pos += 1;
            Some(c)
        }
        fn parse_expr(&mut self) -> f64 {
            let mut v = self.parse_term();
            while let Some(c) = self.peek() {
                match c {
                    '+' => {
                        self.next();
                        v += self.parse_term();
                    }
                    '-' => {
                        self.next();
                        v -= self.parse_term();
                    }
                    _ => break,
                }
            }
            v
        }
        fn parse_term(&mut self) -> f64 {
            let mut v = self.parse_unary();
            while let Some(c) = self.peek() {
                match c {
                    '*' => {
                        self.next();
                        v *= self.parse_unary();
                    }
                    '/' => {
                        self.next();
                        let d = self.parse_unary();
                        if d.abs() > 1e-30 {
                            v /= d;
                        }
                    }
                    _ => break,
                }
            }
            v
        }
        fn parse_unary(&mut self) -> f64 {
            if self.peek() == Some('-') {
                self.next();
                return -self.parse_unary();
            }
            self.parse_primary()
        }
        fn parse_primary(&mut self) -> f64 {
            match self.peek() {
                Some('(') => {
                    self.next();
                    let v = self.parse_expr();
                    if self.peek() == Some(')') {
                        self.next();
                    }
                    v
                }
                Some('0'..='9') | Some('.') => {
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() || c == '.' {
                            self.next();
                        } else {
                            break;
                        }
                    }
                    self.chars[start..self.pos]
                        .iter()
                        .collect::<String>()
                        .parse::<f64>()
                        .unwrap_or(0.0)
                }
                Some(_) => {
                    // Read identifier / species formula (may include balanced parentheses like CO(g) or Ni(CO)4(g))
                    let start = self.pos;
                    let mut depth = 0i32;
                    while let Some(c) = self.peek() {
                        if c == '(' {
                            depth += 1;
                            self.next();
                        } else if c == ')' {
                            if depth > 0 {
                                depth -= 1;
                                self.next();
                            } else {
                                break;
                            }
                        } else if depth == 0 && matches!(c, '+' | '-' | '*' | '/') {
                            break;
                        } else {
                            self.next();
                        }
                    }
                    let tok: String = self.chars[start..self.pos].iter().collect();
                    let up = tok.to_uppercase();
                    if let Some(num_s) = up.strip_prefix('D') {
                        if let Ok(r) = num_s.parse::<usize>() {
                            if r >= 5 {
                                return self
                                    .pt
                                    .species
                                    .get(r - 5)
                                    .map(|s| s.equil_amount_mol / self.scale)
                                    .unwrap_or(0.0);
                            }
                        }
                    }
                    self.pt
                        .species
                        .iter()
                        .find(|s| s.formula.eq_ignore_ascii_case(&tok))
                        .map(|s| s.equil_amount_mol / self.scale)
                        .unwrap_or(0.0)
                }
                None => 0.0,
            }
        }
    }
    let mut parser = UfParser {
        chars: cleaned.chars().collect(),
        pos: 0,
        pt,
        scale,
    };
    parser.parse_expr()
}

fn species_matches_filters(
    sp: &soryntel_thermo::equilibrium::SpeciesEquilibriumResult,
    hidden: &BTreeSet<String>,
    phase_filter: Option<usize>,
    elem_filter: &str,
    contains_all: bool,
) -> bool {
    if hidden.contains(&sp.formula) {
        return false;
    }
    if let Some(pf) = phase_filter {
        if sp.phase_idx != pf {
            return false;
        }
    }
    let ef = elem_filter.trim();
    if !ef.is_empty() && !ef.eq_ignore_ascii_case("All elements") {
        let tokens: Vec<String> = ef
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if !tokens.is_empty() {
            let f_low = sp.formula.to_lowercase();
            let matched = if contains_all {
                tokens.iter().all(|t| f_low.contains(t))
            } else {
                tokens.iter().any(|t| f_low.contains(t))
            };
            if !matched {
                return false;
            }
        }
    }
    true
}

/// Fritsch-Carlson monotone cubic Hermite (PCHIP) tangents so flat plateaus and
/// zero floors never produce artificial wavy ripples.
fn pchip_slopes(ys: &[f64]) -> Vec<f64> {
    let n = ys.len();
    let mut d = vec![0.0; n];
    if n < 2 {
        return d;
    }
    let mut delta = Vec::with_capacity(n - 1);
    for i in 0..(n - 1) {
        delta.push(ys[i + 1] - ys[i]);
    }
    for i in 1..(n - 1) {
        let d0 = delta[i - 1];
        let d1 = delta[i];
        if d0 * d1 > 0.0 {
            d[i] = (2.0 * d0 * d1) / (d0 + d1);
        }
    }
    if n >= 2 {
        d[0] = delta[0];
        if n >= 3 && d[0].signum() != delta[1].signum() && delta[1].abs() > 1e-12 {
            d[0] *= 0.5;
        }
        d[n - 1] = delta[n - 2];
        if n >= 3 && d[n - 1].signum() != delta[n - 3].signum() && delta[n - 3].abs() > 1e-12 {
            d[n - 1] *= 0.5;
        }
    }
    d
}

fn eval_hermite(y0: f64, y1: f64, m0: f64, m1: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * y0 + h10 * m0 + h01 * y1 + h11 * m1
}

fn pchip_subdivide_1d(ys: &[f64], subdiv: usize) -> Vec<f64> {
    let n = ys.len();
    if n < 2 || subdiv <= 1 {
        return ys.to_vec();
    }
    let slopes = pchip_slopes(ys);
    let mut out = Vec::with_capacity((n - 1) * subdiv + 1);
    for i in 0..(n - 1) {
        for s in 0..subdiv {
            let t = s as f64 / subdiv as f64;
            out.push(eval_hermite(ys[i], ys[i + 1], slopes[i], slopes[i + 1], t));
        }
    }
    out.push(*ys.last().unwrap());
    out
}

fn smooth_2d_curve_pchip(pts: &[[f64; 2]], subdiv: usize) -> Vec<[f64; 2]> {
    if pts.len() < 3 || subdiv <= 1 {
        return pts.to_vec();
    }
    let ys: Vec<f64> = pts.iter().map(|p| p[1]).collect();
    let slopes = pchip_slopes(&ys);
    let all_non_neg = ys.iter().all(|&y| y >= -1e-12);
    let n = pts.len();
    let mut out = Vec::with_capacity((n - 1) * subdiv + 1);
    for i in 0..(n - 1) {
        for s in 0..subdiv {
            let t = s as f64 / subdiv as f64;
            let x = pts[i][0] + t * (pts[i + 1][0] - pts[i][0]);
            let mut y = eval_hermite(ys[i], ys[i + 1], slopes[i], slopes[i + 1], t);
            if all_non_neg {
                y = y.max(0.0);
            }
            out.push([x, y]);
        }
    }
    out.push(*pts.last().unwrap());
    out
}

fn species_palette_color(idx: usize) -> egui::Color32 {
    const PALETTE: &[(u8, u8, u8)] = &[
        (110, 175, 45),  // olive-green (N2(g) in HSC9)
        (14, 165, 233),  // sky blue (CO(g) in HSC9)
        (90, 95, 102),   // slate gray (CO2(g))
        (182, 168, 72),  // olive-tan
        (249, 115, 22),  // bright orange
        (29, 78, 216),   // deep blue
        (6, 182, 212),   // cyan
        (16, 185, 129),  // emerald
        (132, 204, 22),  // lime
        (234, 88, 12),   // burnt orange
        (220, 38, 38),   // red
        (190, 24, 93),   // magenta
        (124, 58, 237),  // violet
        (161, 168, 24),  // HSC yellow-olive (C / SO2)
        (59, 130, 246),  // royal blue (Na2CO3 / Na2SO4)
        (217, 119, 6),   // amber (Na2S)
    ];
    let (r, g, b) = PALETTE[idx % PALETTE.len()];
    egui::Color32::from_rgba_unmultiplied(r, g, b, 255)
}

fn level_gradient_color(t: f64) -> egui::Color32 {
    let u = t.clamp(0.0, 1.0) as f32;
    let (r, g, b) = if u < 0.25 {
        let k = u / 0.25;
        (55.0 + 20.0 * k, 65.0 + 130.0 * k, 180.0 + 50.0 * k)
    } else if u < 0.5 {
        let k = (u - 0.25) / 0.25;
        (75.0 - 40.0 * k, 195.0 + 30.0 * k, 230.0 - 140.0 * k)
    } else if u < 0.75 {
        let k = (u - 0.5) / 0.25;
        (35.0 + 210.0 * k, 225.0 - 45.0 * k, 90.0 - 60.0 * k)
    } else {
        let k = (u - 0.75) / 0.25;
        (245.0 - 25.0 * k, 180.0 - 145.0 * k, 30.0 + 110.0 * k)
    };
    egui::Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, 255)
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle_z(
    z_buf: &mut [f32],
    rgba: &mut [u8],
    w: usize,
    h: usize,
    v0: (f32, f32, f32, f32),
    v1: (f32, f32, f32, f32),
    v2: (f32, f32, f32, f32),
    base_col: egui::Color32,
) {
    let area = (v1.0 - v0.0) * (v2.1 - v0.1) - (v2.0 - v0.0) * (v1.1 - v0.1);
    if area.abs() < 1e-4 {
        return;
    }
    let inv_area = 1.0 / area;
    let min_x = v0.0.min(v1.0).min(v2.0).floor().max(0.0) as isize;
    let max_x = v0.0.max(v1.0).max(v2.0).ceil().min((w - 1) as f32) as isize;
    let min_y = v0.1.min(v1.1).min(v2.1).floor().max(0.0) as isize;
    let max_y = v0.1.max(v1.1).max(v2.1).ceil().min((h - 1) as f32) as isize;

    let br = base_col.r() as f32;
    let bg = base_col.g() as f32;
    let bb = base_col.b() as f32;

    for py in min_y..=max_y {
        let y = py as f32 + 0.5;
        for px in min_x..=max_x {
            let x = px as f32 + 0.5;
            let w0 = ((v1.0 - x) * (v2.1 - y) - (v2.0 - x) * (v1.1 - y)) * inv_area;
            let w1 = ((v2.0 - x) * (v0.1 - y) - (v0.0 - x) * (v2.1 - y)) * inv_area;
            let w2 = 1.0 - w0 - w1;
            if w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4 {
                let z = w0 * v0.2 + w1 * v1.2 + w2 * v2.2;
                let idx = (py as usize) * w + (px as usize);
                if z > z_buf[idx] {
                    z_buf[idx] = z;
                    let bright = (w0 * v0.3 + w1 * v1.3 + w2 * v2.3).clamp(0.45, 1.15);
                    let off = idx * 4;
                    rgba[off] = (br * bright).clamp(0.0, 255.0) as u8;
                    rgba[off + 1] = (bg * bright).clamp(0.0, 255.0) as u8;
                    rgba[off + 2] = (bb * bright).clamp(0.0, 255.0) as u8;
                    rgba[off + 3] = 255;
                }
            }
        }
    }
}

fn rasterize_line_z(
    z_buf: &[f32],
    rgba: &mut [u8],
    w: usize,
    h: usize,
    a: (f32, f32, f32),
    b: (f32, f32, f32),
) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let x = (a.0 + t * dx).round() as isize;
        let y = (a.1 + t * dy).round() as isize;
        if x >= 0 && x < w as isize && y >= 0 && y < h as isize {
            let z = a.2 + t * (b.2 - a.2);
            let idx = (y as usize) * w + (x as usize);
            if z + 0.004 >= z_buf[idx] {
                let off = idx * 4;
                // Blend dark gray grid line cleanly over surface pixel
                rgba[off] = ((rgba[off] as u16 * 3 + 28) / 4) as u8;
                rgba[off + 1] = ((rgba[off + 1] as u16 * 3 + 30) / 4) as u8;
                rgba[off + 2] = ((rgba[off + 2] as u16 * 3 + 35) / 4) as u8;
                rgba[off + 3] = 255;
            }
        }
    }
}

/// Collects distinct X and Z coordinate values across an equilibrium sweep result.
fn collect_distinct_axes(
    res: &soryntel_thermo::equilibrium::EquilibriumSweepResult,
    default_missing_z_zero: bool,
) -> (Vec<f64>, Vec<f64>) {
    let mut xs: Vec<f64> = Vec::new();
    let mut zs: Vec<f64> = Vec::new();
    for p in &res.points {
        if !xs.iter().any(|&v| (v - p.x_value).abs() < 1e-9) {
            xs.push(p.x_value);
        }
        let zv_opt = if default_missing_z_zero {
            Some(p.z_value.unwrap_or(0.0))
        } else {
            p.z_value
        };
        if let Some(zv) = zv_opt {
            if !zs.iter().any(|&v| (v - zv).abs() < 1e-9) {
                zs.push(zv);
            }
        }
    }
    (xs, zs)
}

#[allow(clippy::too_many_arguments)]
fn render_3d_surface_chart(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    res: &EquilibriumSweepResult,
    scale: f64,
    y_kind: ChartYKind,
    hidden: &BTreeSet<String>,
    phase_filter: Option<usize>,
    elem_filter: &str,
    contains_all: bool,
    chart_title: &str,
    x_label: &str,
    z_label: &str,
    yaw: &mut f32,
    pitch: &mut f32,
    color_by_levels: bool,
    smooth: bool,
    cache: &mut Option<Cached3dSurface>,
) {
    let Some(first) = res.points.first() else {
        return;
    };

    // Collect distinct X and Z coordinates to form the 2D surface grid.
    let (xs, zs) = collect_distinct_axes(res, true);
    let nx = xs.len();
    let nz = zs.len();
    if nx < 2 || nz < 2 || nx * nz != res.points.len() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.strong("No se puede dibujar superficie 3D (se requiere grilla >= 2x2).");
            ui.label(format!(
                "Valores distintos detectados: X = {} | Z = {} | Puntos totales = {}",
                nx,
                nz,
                res.points.len()
            ));
            ui.label("Causa comun: en 'System State', alguna de las 2 variables tiene Steps = 1 o Initial == Final (ej. P: 1.0..1.0 o T: 25..25). Cambia Initial != Final, Steps >= 2 y vuelve a pulsar Calculate.");
        });
        return;
    }

    let x_min = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let z_min = zs.iter().copied().fold(f64::INFINITY, f64::min);
    let z_max = zs.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let mut y_min = 0.0f64;
    let mut y_max = 1e-6f64;
    let mut visible_species = Vec::new();
    for (s_idx, sp0) in first.species.iter().enumerate() {
        if species_matches_filters(sp0, hidden, phase_filter, elem_filter, contains_all) {
            let mut sp_max = 0.0f64;
            for p in &res.points {
                let sp = &p.species[s_idx];
                let y = match y_kind {
                    ChartYKind::EquilibriumAmount => sp.equil_amount_mol / scale,
                    ChartYKind::MoleFraction => sp.mole_fraction,
                    ChartYKind::Activity => sp.activity,
                    ChartYKind::ActivityCoeff => sp.activity_coeff,
                    ChartYKind::ReactionEnthalpy => p.reaction_enthalpy_kj / 1000.0,
                    ChartYKind::Ph => p.ph.unwrap_or(7.0),
                    _ => sp.equil_amount_mol / scale,
                };
                if y.is_finite() {
                    y_min = y_min.min(y);
                    y_max = y_max.max(y);
                    sp_max = sp_max.max(y.abs());
                }
            }
            if sp_max > 1e-8 {
                visible_species.push(s_idx);
            }
        }
    }
    let y_span = (y_max - y_min).abs().max(1e-12);

    let avail = ui.available_size();
    let canvas_size = egui::vec2(avail.x.max(360.0), avail.y.max(420.0));
    let (rect, resp) = ui.allocate_exact_size(canvas_size, egui::Sense::drag());
    if resp.dragged() {
        let delta = resp.drag_delta();
        *yaw += delta.x * 0.01;
        *pitch = (*pitch + delta.y * 0.01).clamp(-1.35, 1.35);
    }

    let p = ui.painter_at(rect);
    p.rect_filled(rect, 2.0, egui::Color32::WHITE);
    p.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

    let center = rect.center() + egui::vec2(0.0, 12.0);
    let scale_px = rect.width().min(rect.height()) * 0.34;
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();

    // Right-handed projection with z=0 (low Z) in FRONT and z=1 (high Z) in BACK:
    // more positive `depth` is CLOSER to viewer.
    let project = |nx_val: f64, ny_val: f64, nz_val: f64| -> (egui::Pos2, f32) {
        let px = (nx_val * 2.0 - 1.0) as f32;
        let py = (ny_val * 1.25 - 0.52) as f32;
        let pz = (1.0 - 2.0 * nz_val) as f32;

        let rx = px * cy + pz * sy;
        let rz_cam = -px * sy + pz * cy;
        let ry = py * cp - rz_cam * sp;
        let depth = py * sp + rz_cam * cp;

        (egui::pos2(center.x + rx * scale_px, center.y - ry * scale_px), depth)
    };

    // Camera view direction in (px, py, pz) space for exact backface culling
    let vx = -sy * cp;
    let vy = sp;
    let vz = cy * cp;

    let box_corners = [
        project(0.0, 0.0, 0.0).0, // 0: x0, y0, z0 (front-left bottom)
        project(1.0, 0.0, 0.0).0, // 1: x1, y0, z0 (front-right bottom)
        project(1.0, 0.0, 1.0).0, // 2: x1, y0, z1 (back-right bottom)
        project(0.0, 0.0, 1.0).0, // 3: x0, y0, z1 (back-left bottom)
        project(0.0, 1.0, 0.0).0, // 4: x0, y1, z0
        project(1.0, 1.0, 0.0).0, // 5: x1, y1, z0
        project(1.0, 1.0, 1.0).0, // 6: x1, y1, z1
        project(0.0, 1.0, 1.0).0, // 7: x0, y1, z1
    ];

    // Draw back walls & bottom floor with crisp light-gray fill & gridlines (HSC9 style)
    let back_wall_fill = egui::Color32::from_rgb(230, 232, 236);
    let back_grid_stroke = egui::Stroke::new(0.7, egui::Color32::from_rgb(196, 200, 208));
    let show_x0 = -vx < 0.0;
    let show_x1 = vx < 0.0;
    let show_z0 = vz < 0.0;
    let show_z1 = -vz < 0.0;
    let show_y0 = -vy < 0.0;

    if show_y0 {
        p.add(egui::Shape::convex_polygon(
            vec![box_corners[0], box_corners[1], box_corners[2], box_corners[3]],
            egui::Color32::from_rgb(243, 244, 246),
            back_grid_stroke,
        ));
    }
    if show_x0 {
        p.add(egui::Shape::convex_polygon(
            vec![box_corners[0], box_corners[3], box_corners[7], box_corners[4]],
            back_wall_fill,
            back_grid_stroke,
        ));
        for k in 1..4 {
            let t = k as f64 / 4.0;
            p.line_segment([project(0.0, t, 0.0).0, project(0.0, t, 1.0).0], back_grid_stroke);
            p.line_segment([project(0.0, 0.0, t).0, project(0.0, 1.0, t).0], back_grid_stroke);
        }
    }
    if show_x1 {
        p.add(egui::Shape::convex_polygon(
            vec![box_corners[1], box_corners[2], box_corners[6], box_corners[5]],
            back_wall_fill,
            back_grid_stroke,
        ));
    }
    if show_z1 {
        p.add(egui::Shape::convex_polygon(
            vec![box_corners[2], box_corners[3], box_corners[7], box_corners[6]],
            back_wall_fill,
            back_grid_stroke,
        ));
        for k in 1..5 {
            let t = k as f64 / 5.0;
            p.line_segment([project(t, 0.0, 1.0).0, project(t, 1.0, 1.0).0], back_grid_stroke);
        }
        for k in 1..4 {
            let t = k as f64 / 4.0;
            p.line_segment([project(0.0, t, 1.0).0, project(1.0, t, 1.0).0], back_grid_stroke);
        }
    }
    if show_z0 {
        p.add(egui::Shape::convex_polygon(
            vec![box_corners[0], box_corners[1], box_corners[5], box_corners[4]],
            back_wall_fill,
            back_grid_stroke,
        ));
    }

    // Build cache key so we only rasterize the Z-buffered surface when needed
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let buf_w = (rect.width().round() as usize).clamp(300, 960);
    let buf_h = (rect.height().round() as usize).clamp(240, 720);
    buf_w.hash(&mut hasher);
    buf_h.hash(&mut hasher);
    yaw.to_bits().hash(&mut hasher);
    pitch.to_bits().hash(&mut hasher);
    color_by_levels.hash(&mut hasher);
    smooth.hash(&mut hasher);
    (y_kind as u8).hash(&mut hasher);
    res.points.len().hash(&mut hasher);
    if let (Some(p0), Some(p1)) = (res.points.first(), res.points.last()) {
        p0.temp_k.to_bits().hash(&mut hasher);
        p1.temp_k.to_bits().hash(&mut hasher);
        p0.pressure_bar.to_bits().hash(&mut hasher);
        p1.pressure_bar.to_bits().hash(&mut hasher);
        p0.g_min_rt.to_bits().hash(&mut hasher);
        p1.g_min_rt.to_bits().hash(&mut hasher);
    }
    visible_species.hash(&mut hasher);
    let key = hasher.finish();

    let need_redraw = cache.as_ref().map(|c| c.key != key).unwrap_or(true);
    if need_redraw {
        let mut z_buf = vec![f32::NEG_INFINITY; buf_w * buf_h];
        let mut rgba = vec![0u8; buf_w * buf_h * 4];
        let sx_ratio = buf_w as f32 / rect.width().max(1.0);
        let sy_ratio = buf_h as f32 / rect.height().max(1.0);
        let to_buf = |pt: (egui::Pos2, f32)| -> (f32, f32, f32) {
            (
                (pt.0.x - rect.left()) * sx_ratio,
                (pt.0.y - rect.top()) * sy_ratio,
                pt.1,
            )
        };

        let sub = if smooth && nx >= 3 && nz >= 3 { 3usize } else { 1usize };
        let snx = (nx - 1) * sub + 1;
        let snz = (nz - 1) * sub + 1;

        for &s_idx in &visible_species {
            let base_col = species_palette_color(s_idx);
            let mut grid_y = vec![vec![0.0f64; nx]; nz];
            for iz in 0..nz {
                for ix in 0..nx {
                    let pt = &res.points[iz * nx + ix];
                    let sp_r = &pt.species[s_idx];
                    let raw_y = match y_kind {
                        ChartYKind::EquilibriumAmount => sp_r.equil_amount_mol / scale,
                        ChartYKind::MoleFraction => sp_r.mole_fraction,
                        ChartYKind::Activity => sp_r.activity,
                        ChartYKind::ActivityCoeff => sp_r.activity_coeff,
                        ChartYKind::ReactionEnthalpy => pt.reaction_enthalpy_kj / 1000.0,
                        ChartYKind::Ph => pt.ph.unwrap_or(7.0),
                        _ => sp_r.equil_amount_mol / scale,
                    };
                    grid_y[iz][ix] = ((raw_y - y_min) / y_span).clamp(0.0, 1.0);
                }
            }

            // Build monotone PCHIP-smoothed height field (zero overshoot waves)
            let mut s_grid = vec![vec![0.0f64; snx]; snz];
            if sub == 1 {
                s_grid = grid_y;
            } else {
                let mut tmp_x = vec![vec![0.0f64; snx]; nz];
                for iz in 0..nz {
                    tmp_x[iz] = pchip_subdivide_1d(&grid_y[iz], sub);
                }
                for sx in 0..snx {
                    let col_vals: Vec<f64> = (0..nz).map(|iz| tmp_x[iz][sx]).collect();
                    let smooth_col = pchip_subdivide_1d(&col_vals, sub);
                    for sz in 0..snz {
                        s_grid[sz][sx] = smooth_col[sz].clamp(0.0, 1.0);
                    }
                }
            }

            // Precompute projected vertices + Gouraud vertex brightness from smooth central differences
            let dx_step = 1.0f32 / (snx - 1) as f32;
            let dz_step = 1.0f32 / (snz - 1) as f32;
            let mut verts = vec![vec![(0.0f32, 0.0f32, 0.0f32, 1.0f32); snx]; snz];
            for iz in 0..snz {
                let z_norm = iz as f64 / (snz - 1) as f64;
                let iz_m = iz.saturating_sub(1);
                let iz_p = (iz + 1).min(snz - 1);
                for ix in 0..snx {
                    let x_norm = ix as f64 / (snx - 1) as f64;
                    let ix_m = ix.saturating_sub(1);
                    let ix_p = (ix + 1).min(snx - 1);
                    let (px, py, pz) = to_buf(project(x_norm, s_grid[iz][ix], z_norm));
                    let dydx = (s_grid[iz][ix_p] - s_grid[iz][ix_m]) as f32
                        / ((ix_p - ix_m).max(1) as f32 * dx_step);
                    let dydz = (s_grid[iz_p][ix] - s_grid[iz_m][ix]) as f32
                        / ((iz_p - iz_m).max(1) as f32 * dz_step);
                    // Surface normal n = (-dydx, 1.0, dydz) pointing mostly upward (+Y)
                    let nx_v = -dydx * 0.7;
                    let ny_v = 1.0f32;
                    let nz_v = dydz * 0.7;
                    let n_len = (nx_v * nx_v + ny_v * ny_v + nz_v * nz_v).sqrt();
                    let l_dot = ((-0.38 * nx_v + 0.86 * ny_v + 0.34 * nz_v) / n_len).abs();
                    let bright = 0.74 + 0.32 * l_dot;
                    verts[iz][ix] = (px, py, pz, bright);
                }
            }

            // Rasterize triangles into per-pixel Z-buffer
            for iz in 0..(snz - 1) {
                for ix in 0..(snx - 1) {
                    let y00 = s_grid[iz][ix];
                    let y10 = s_grid[iz][ix + 1];
                    let y11 = s_grid[iz + 1][ix + 1];
                    let y01 = s_grid[iz + 1][ix];
                    if y00 < 1e-6 && y10 < 1e-6 && y11 < 1e-6 && y01 < 1e-6 {
                        continue;
                    }
                    let avg_y = 0.25 * (y00 + y10 + y11 + y01);
                    let tri_col = if color_by_levels {
                        level_gradient_color(avg_y)
                    } else {
                        base_col
                    };
                    let v00 = verts[iz][ix];
                    let v10 = verts[iz][ix + 1];
                    let v11 = verts[iz + 1][ix + 1];
                    let v01 = verts[iz + 1][ix];
                    rasterize_triangle_z(&mut z_buf, &mut rgba, buf_w, buf_h, v00, v10, v11, tri_col);
                    rasterize_triangle_z(&mut z_buf, &mut rgba, buf_w, buf_h, v00, v11, v01, tri_col);
                }
            }

            // Rasterize crisp wireframe mesh ONLY on the original integer grid lines with depth test
            for iz_orig in 0..nz {
                let iz = iz_orig * sub;
                for ix in 0..(snx - 1) {
                    if s_grid[iz][ix] < 1e-6 && s_grid[iz][ix + 1] < 1e-6 {
                        continue;
                    }
                    let a = verts[iz][ix];
                    let b = verts[iz][ix + 1];
                    rasterize_line_z(&z_buf, &mut rgba, buf_w, buf_h, (a.0, a.1, a.2), (b.0, b.1, b.2));
                }
            }
            for ix_orig in 0..nx {
                let ix = ix_orig * sub;
                for iz in 0..(snz - 1) {
                    if s_grid[iz][ix] < 1e-6 && s_grid[iz + 1][ix] < 1e-6 {
                        continue;
                    }
                    let a = verts[iz][ix];
                    let b = verts[iz + 1][ix];
                    rasterize_line_z(&z_buf, &mut rgba, buf_w, buf_h, (a.0, a.1, a.2), (b.0, b.1, b.2));
                }
            }
        }

        let img = egui::ColorImage::from_rgba_unmultiplied([buf_w, buf_h], &rgba);
        let tex = ctx.load_texture("equil_3d_zbuf", img, egui::TextureOptions::LINEAR);
        *cache = Some(Cached3dSurface { key, tex });
    }

    if let Some(ref c) = cache {
        p.image(
            c.tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // Crisp outer bounding-box frame + front axis tick labels (HSC9 layout)
    let axis_stroke = egui::Stroke::new(1.2, egui::Color32::from_rgb(55, 60, 68));
    for (a, b) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (0, 4),
        (3, 7),
        (2, 6),
    ] {
        p.line_segment([box_corners[a], box_corners[b]], axis_stroke);
    }

    for k in 0..=5 {
        let t = k as f64 / 5.0;
        let xv = x_min + t * (x_max - x_min);
        let p_x = project(t, 0.0, 0.0).0;
        p.text(
            p_x + egui::vec2(0.0, 5.0),
            egui::Align2::CENTER_TOP,
            format!("{:.0}", xv),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(40, 40, 40),
        );
    }
    if (z_min - 1.0).abs() < 0.05 && (z_max - 10.0).abs() < 0.05 {
        for zv in [2.0, 4.0, 6.0, 8.0, 10.0] {
            let t = (zv - z_min) / (z_max - z_min);
            let p_z = project(1.0, 0.0, t).0;
            p.text(
                p_z + egui::vec2(8.0, 2.0),
                egui::Align2::LEFT_TOP,
                format!("{:.0}", zv),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(40, 40, 40),
            );
        }
    } else {
        for k in 0..=5 {
            let t = k as f64 / 5.0;
            let zv = z_min + t * (z_max - z_min);
            let p_z = project(1.0, 0.0, t).0;
            p.text(
                p_z + egui::vec2(8.0, 2.0),
                egui::Align2::LEFT_TOP,
                format!("{:.1}", zv),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(40, 40, 40),
            );
        }
    }
    for k in 0..=4 {
        let t = k as f64 / 4.0;
        let yv = y_min + t * (y_max - y_min);
        let p_y = project(0.0, t, 0.0).0;
        p.text(
            p_y - egui::vec2(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!("{:.1}", yv),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(40, 40, 40),
        );
    }

    // Titles matching HSC9 screenshot
    p.text(
        egui::pos2(rect.center().x, rect.top() + 8.0),
        egui::Align2::CENTER_TOP,
        chart_title,
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(24, 24, 27),
    );
    p.text(
        rect.left_top() + egui::vec2(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        y_kind.label(),
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(55, 65, 81),
    );
    p.text(
        box_corners[1].lerp(box_corners[0], 0.5) + egui::vec2(0.0, 22.0),
        egui::Align2::CENTER_TOP,
        x_label,
        egui::FontId::proportional(11.5),
        egui::Color32::BLACK,
    );
    p.text(
        box_corners[1].lerp(box_corners[2], 0.5) + egui::vec2(20.0, 14.0),
        egui::Align2::LEFT_TOP,
        z_label,
        egui::FontId::proportional(11.5),
        egui::Color32::BLACK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pchip_monotone_no_overshoot_on_plateaus() {
        let vals = vec![4.0, 4.0, 3.2, 1.5, 0.0, 0.0];
        let smooth = pchip_subdivide_1d(&vals, 3);
        for &v in &smooth {
            assert!(
                (-1e-12..=4.0 + 1e-12).contains(&v),
                "PCHIP overshoot detected: {v}"
            );
        }
        assert!((smooth[0] - 4.0).abs() < 1e-12);
        assert!((smooth[3] - 4.0).abs() < 1e-12);
        assert!(smooth[smooth.len() - 1].abs() < 1e-12);
    }

    #[test]
    fn test_na2so4_preset_3d_sweep_with_maindb6() {
        let hsc_path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
        if !std::path::Path::new(hsc_path).exists() {
            return;
        }
        let mut db = ThermoDatabase::with_open_defaults();
        db.load_main_hsc(hsc_path).unwrap();
        let mut state = EquilibriumUiState::default();
        state.load_preset_na2so4();
        state.run_calculate(&db);
        assert!(state.last_error.is_none(), "Error: {:?}", state.last_error);
        let res = state.result.as_ref().expect("result must exist");
        assert_eq!(res.points.len(), 21 * 10);
        // Spot check T=0, P=1 and T=1000, P=1 and T=1000, P=10
        let pt0 = &res.points[0];
        let pt1000_p1 = &res.points[20];
        let pt1000_p10 = &res.points[21 * 9 + 20];
        let amt = |pt: &soryntel_thermo::equilibrium::EquilibriumPointResult, f: &str| -> f64 {
            pt.species
                .iter()
                .find(|s| s.formula == f)
                .map(|s| s.equil_amount_mol / 1000.0)
                .unwrap_or(-1.0)
        };
        for pt in &res.points {
            assert!(
                pt.converged,
                "Point at T={} K, P={} bar failed to converge (err={})",
                pt.temp_k,
                pt.pressure_bar,
                pt.max_mass_balance_err
            );
            assert!(pt.max_mass_balance_err < 1e-8);
        }
        // Check strictly decreasing smooth curve of Carbon C(s) at P = 1 bar (no spikes!)
        for ix in 0..20 {
            let c0 = amt(&res.points[ix], "C");
            let c1 = amt(&res.points[ix + 1], "C");
            assert!(
                c1 <= c0 + 1e-6,
                "Non-monotonic spike in C at step {}: {} -> {}",
                ix,
                c0,
                c1
            );
        }
        assert!((amt(pt0, "N2(g)") - 1.0).abs() < 1e-4);
        assert!(amt(pt1000_p1, "CO(g)") > 2.5, "Expected high CO(g) at 1000C 1 bar, got {}", amt(pt1000_p1, "CO(g)"));
        assert!(amt(pt1000_p10, "CO2(g)") >= amt(pt1000_p1, "CO2(g)"), "Higher P should stabilize CO2 vs CO");
    }

    #[test]
    fn test_switching_1d_preset_to_2d_produces_valid_3d_grid() {
        let mut state = EquilibriumUiState::default();
        // Even 0D/1D presets with degenerate ranges (e.g. CaCO3 T=25..25, NICO P=1..1)
        // must expand cleanly to a >= 2x2 grid when switching to 2 independent variables.
        state.load_preset_caco3_aqua();
        assert!(!state.chart_is_3d);
        state.params.num_indep_vars = 2;
        state.ensure_2d_sweep_defaults();
        assert_eq!(state.params.primary_var, SweepVariable::TemperatureC);
        assert_eq!(state.params.secondary_var, Some(SweepVariable::PressureBar));
        assert!(state.params.temp_range.steps >= 2);
        assert!((state.params.temp_range.final_val - state.params.temp_range.initial).abs() > 1.0);
        assert!(state.params.pressure_range.steps >= 2);
        assert!((state.params.pressure_range.final_val - state.params.pressure_range.initial).abs() > 1.0);

        let hsc_path = "/home/nando/Descargas/HSC6/Databases/Maindb6.HSC";
        if !std::path::Path::new(hsc_path).exists() {
            return;
        }
        let mut db = ThermoDatabase::with_open_defaults();
        db.load_main_hsc(hsc_path).unwrap();
        state.load_preset_nico();
        state.params.num_indep_vars = 2;
        state.ensure_2d_sweep_defaults();
        state.run_calculate(&db);
        assert!(state.last_error.is_none(), "Error: {:?}", state.last_error);
        let res = state.result.as_ref().expect("result must exist");
        let (xs, zs) = collect_distinct_axes(res, false);
        assert!(xs.len() >= 2 && zs.len() >= 2);
        assert_eq!(xs.len() * zs.len(), res.points.len());
        assert!(state.chart_is_3d);
    }
}
