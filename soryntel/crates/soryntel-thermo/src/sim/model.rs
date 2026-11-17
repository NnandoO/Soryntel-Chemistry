//! Domain data structures for Module 18: `HSC - Sim Common Tools`.
//! Covers flowsheet topology (Units, Streams, Pages, Layers, Groups, Locks),
//! Tables / Title Tables / Stream Tables / Flow Tables, Process Info & Notes,
//! Unit Editor Controls, Scenario Editor + Monte Carlo, Convergence Settings,
//! Sankey Visualization Settings, and Hierarchical Simulation Logs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Generic,       // U
    Reactions,     // R
    Distributions, // D
    Dynamic,       // τ
}

impl UnitKind {
    pub fn badge(self) -> &'static str {
        match self {
            Self::Generic => "U",
            Self::Reactions => "R",
            Self::Distributions => "D",
            Self::Dynamic => "τ",
        }
    }

    pub fn mode_label(self) -> &'static str {
        match self {
            Self::Generic => "Generic",
            Self::Reactions => "Reactions (Hydro)",
            Self::Distributions => "Distribution (Particles)",
            Self::Dynamic => "Dynamic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelImplType {
    Excel,
    Dll,
}

impl ModelImplType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Excel => "Excel",
            Self::Dll => "DLL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelLibraryTab {
    Species,
    Particles,
    Others,
    Imported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitModelCatalogEntry {
    pub model_name: String,
    pub mode: String,
    pub technology: String,
    pub type_code: String,
    pub impl_type: ModelImplType,
    pub category: ModelLibraryTab,
    pub version: String,
    pub calculation: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowHeadStyle {
    None,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamPhaseKind {
    Gas,
    Water,
    PureSolid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSpeciesFlow {
    pub formula: String,
    pub phase: StreamPhaseKind,
    pub mass_flow_t_h: f64,
    pub enthalpy_kw: f64,
    pub exergy_kw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimStream {
    pub id: usize,
    pub name_id: String,
    pub alias: String,
    pub source_unit: Option<usize>,
    pub dest_unit: Option<usize>,
    pub source_anchor: [f32; 2],
    pub dest_anchor: [f32; 2],
    pub waypoints: Vec<[f32; 2]>,
    pub page: usize,
    pub layer: usize,
    pub visible_stream: bool,
    pub visible_name_label: bool,
    pub visible_value_label: bool,
    pub visible_stream_table: bool,
    pub has_duplicates: bool,
    // Visual properties (Fig. 64)
    pub line_color: [u8; 3],
    pub line_style: LineStyle,
    pub line_width: f32,
    pub auto_arrow_update: bool,
    pub arrow_color: [u8; 3],
    pub start_arrow_style: ArrowHeadStyle,
    pub start_arrow_size: u8,
    pub end_arrow_style: ArrowHeadStyle,
    pub end_arrow_size: u8,
    // Thermodynamic / particulate process state (Figs. 24-26, 67)
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub mass_flow_t_h: f64,
    pub gas_flow_nm3_h: f64,
    pub liquid_flow_t_h: f64,
    pub solids_flow_t_h: f64,
    pub solids_rec_pct: f64,
    pub liquid_rec_pct: f64,
    pub pulp_vol_m3_h: f64,
    pub solids_sg: f64,
    pub liquid_sg: f64,
    pub pulp_sg: f64,
    pub pct_solids: f64,
    pub cu_wt_pct: f64,
    pub fe_wt_pct: f64,
    pub s_wt_pct: f64,
    pub ccp_rec_pct: f64,
    pub cu_rec_pct: f64,
    pub p80_um: f64,
    pub p50_um: f64,
    pub enthalpy_kw: f64,
    pub exergy_kw: f64,
    pub cp_kj_kg_k: f64,
    pub thermal_energy_kw: f64,
    pub heat_content_kw: f64,
    pub pressure_drop_bar: f64,
    pub delay_s: f64,
    pub species_flows: Vec<StreamSpeciesFlow>,
}

impl SimStream {
    pub fn new(
        id: usize,
        name: impl Into<String>,
        source_unit: Option<usize>,
        dest_unit: Option<usize>,
        source_anchor: [f32; 2],
        dest_anchor: [f32; 2],
        page: usize,
    ) -> Self {
        let nm = name.into();
        Self {
            id,
            name_id: nm.clone(),
            alias: nm,
            source_unit,
            dest_unit,
            source_anchor,
            dest_anchor,
            waypoints: Vec::new(),
            page,
            layer: 1, // Layer 1 (Streams) default or Layer 0
            visible_stream: true,
            visible_name_label: true,
            visible_value_label: true,
            visible_stream_table: false,
            has_duplicates: false,
            line_color: [0, 0, 0],
            line_style: LineStyle::Solid,
            line_width: 0.25,
            auto_arrow_update: true,
            arrow_color: [0, 0, 0],
            start_arrow_style: ArrowHeadStyle::None,
            start_arrow_size: 3,
            end_arrow_style: ArrowHeadStyle::Triangle,
            end_arrow_size: 3,
            temp_c: 25.0,
            pressure_bar: 1.0,
            mass_flow_t_h: 0.0,
            gas_flow_nm3_h: 0.0,
            liquid_flow_t_h: 0.0,
            solids_flow_t_h: 0.0,
            solids_rec_pct: 100.0,
            liquid_rec_pct: 100.0,
            pulp_vol_m3_h: 0.0,
            solids_sg: 3.10,
            liquid_sg: 1.00,
            pulp_sg: 1.45,
            pct_solids: 0.0,
            cu_wt_pct: 0.0,
            fe_wt_pct: 0.0,
            s_wt_pct: 0.0,
            ccp_rec_pct: 100.0,
            cu_rec_pct: 100.0,
            p80_um: 75.0,
            p50_um: 38.0,
            enthalpy_kw: 0.0,
            exergy_kw: 0.0,
            cp_kj_kg_k: 4.18,
            thermal_energy_kw: 0.0,
            heat_content_kw: 0.0,
            pressure_drop_bar: 0.0,
            delay_s: 0.0,
            species_flows: Vec::new(),
        }
    }

    pub fn connectivity_kind(&self) -> StreamConnectivity {
        match (self.source_unit, self.dest_unit) {
            (None, Some(_)) => StreamConnectivity::Input,
            (Some(_), Some(_)) => StreamConnectivity::Internal,
            (Some(_), None) | (None, None) => StreamConnectivity::Output,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamConnectivity {
    Input,    // Blue in Connectivity View (Fig. 4)
    Internal, // Black
    Output,   // Red
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnitBehavior {
    PassThrough,
    HydroCooler {
        target_temp_c: f64,
        coolant_flow_t_h: f64,
        target_duty_kw: f64,
    },
    HydroLeaching {
        reaction_temp_c: f64,
        acid_feed_t_h: f64,
        target_acid_g_l: f64,
        conversion_pct: f64,
    },
    Hydrocyclone {
        cut_size_um: f64,
        bypass_fines: f64,
    },
    Conditioner {
        residence_s: f64,
        temp_c: f64,
    },
    FlotationCell {
        concentrate_solids_rec_pct: f64,
        base_cu_rec_pct: f64,
    },
    Splitter {
        main_fraction: f64,
    },
    DynamicTank {
        holdup_t: f64,
        max_capacity_t: f64,
        tau_s: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimUnit {
    pub id: usize,
    pub name_id: String,
    pub alias: String,
    pub kind: UnitKind,
    pub model_mode: String,
    pub model_type: ModelImplType,
    pub model_name: String,
    pub technology: String,
    pub sub_technology: String,
    pub availability_pct: f64,
    pub icon_key: Option<String>,
    pub show_unit_image: bool,
    pub transparent_bg: bool,
    pub pos: [f32; 2],  // top-left in canvas mm (Fig. 2: Unit 1 around 10, 20)
    pub size: [f32; 2], // width, height in canvas mm (approx 45 x 30 mm)
    pub page: usize,
    pub layer: usize,
    pub visible_unit: bool,
    pub visible_name_label: bool,
    pub show_title_inside: bool,
    pub font_name: String,
    pub font_size: f32,
    pub font_bold: bool,
    pub font_italic: bool,
    pub font_color: [u8; 3],
    pub fill_color_1: [u8; 3], // 255;255;255
    pub fill_color_2: [u8; 3], // 166;214;255 (Fig. 63)
    pub line_color: [u8; 3],
    pub line_width: f32,
    pub behavior: UnitBehavior,
    pub calc_color_group: Option<u8>, // 1, 2, 3 for Calculation Order (Fig. 40)
    pub sheet_notes: BTreeMap<String, String>,
}

impl SimUnit {
    pub fn new(id: usize, name: impl Into<String>, kind: UnitKind, pos: [f32; 2], page: usize) -> Self {
        let nm = name.into();
        let (model_mode, model_name, behavior) = match kind {
            UnitKind::Generic => (
                "Generic".to_string(),
                "Empty Generic unit".to_string(),
                UnitBehavior::PassThrough,
            ),
            UnitKind::Reactions => (
                "Reactions (Hydro)".to_string(),
                "Empty Reactions (Hydro) unit".to_string(),
                UnitBehavior::PassThrough,
            ),
            UnitKind::Distributions => (
                "Distribution (Pyro)".to_string(),
                "Empty Distribution (Pyro) unit".to_string(),
                UnitBehavior::Splitter { main_fraction: 0.5 },
            ),
            UnitKind::Dynamic => (
                "Dynamic".to_string(),
                "Dynamic Unit".to_string(),
                UnitBehavior::DynamicTank {
                    holdup_t: 10.0,
                    max_capacity_t: 100.0,
                    tau_s: 30.0,
                },
            ),
        };
        Self {
            id,
            name_id: nm.clone(),
            alias: nm,
            kind,
            model_mode,
            model_type: ModelImplType::Excel,
            model_name,
            technology: String::new(),
            sub_technology: String::new(),
            availability_pct: 100.0,
            icon_key: None,
            show_unit_image: true,
            transparent_bg: false,
            pos,
            size: [42.0, 28.0],
            page,
            layer: 0,
            visible_unit: true,
            visible_name_label: true,
            show_title_inside: false,
            font_name: "Arial".to_string(),
            font_size: 8.0,
            font_bold: false,
            font_italic: false,
            font_color: [0, 0, 0],
            fill_color_1: [255, 255, 255],
            fill_color_2: [166, 214, 255],
            line_color: [0, 0, 0],
            line_width: 0.25,
            behavior,
            calc_color_group: None,
            sheet_notes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMethod {
    TangentialFast,
    Pid,
}

impl ControlMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TangentialFast => "Tangential (fast)",
            Self::Pid => "PID",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimControlLoop {
    pub id: usize,
    pub control_name: String,
    pub location_unit: String,
    pub active: bool,
    pub set_point: f64,
    pub measured: f64,
    pub tolerance: f64,
    pub y_target_name: String,
    pub y_unit: String,
    pub y_cell_ref: String,
    pub x_var_name: String,
    pub control_variable: f64,
    pub x_unit: String,
    pub x_min: f64,
    pub x_max: f64,
    pub x_max_step: Option<f64>,
    pub x_cell_ref: String,
    pub control_type: String, // "Internal" | "External"
    pub method: ControlMethod,
    pub max_iter: usize,
    pub kp: f64,
    pub ti_s: f64,
    pub td_s: f64,
    pub tdf_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimCellReference {
    pub unit_name: String,
    pub ref_name: String,
    pub value: f64,
    pub sheet: String,
    pub cell: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConvergenceCriteria {
    Mass,
    Enthalpy,
    Both,
}

impl ConvergenceCriteria {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mass => "Mass",
            Self::Enthalpy => "Enthalpy",
            Self::Both => "Both",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimCalcSettings {
    pub run_until_convergence: bool,
    pub run_until_controls_converge: bool,
    pub criteria: ConvergenceCriteria,
    pub tolerance_pct: f64,
    pub max_rounds: usize,
    // Dynamic settings (Fig. 35)
    pub speed_full: bool,
    pub start_time_s: f64,
    pub time_step_s: f64,
    pub run_until_paused: bool,
    pub end_time_s: f64,
    pub empty_tanks_before_start: bool,
    // Diagnostics
    pub collect_dynamic_scenario: bool,
    pub enable_log: bool,
    pub ignore_warnings: bool,
    pub show_log_after: bool,
}

impl Default for SimCalcSettings {
    fn default() -> Self {
        Self {
            run_until_convergence: true,
            run_until_controls_converge: true,
            criteria: ConvergenceCriteria::Mass,
            tolerance_pct: 0.1,
            max_rounds: 50,
            speed_full: true,
            start_time_s: 0.0,
            time_step_s: 1.0,
            run_until_paused: true,
            end_time_s: 3600.0,
            empty_tanks_before_start: false,
            collect_dynamic_scenario: true,
            enable_log: true,
            ignore_warnings: false,
            show_log_after: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceRound {
    pub round: usize,
    pub mb_pct: f64,
    pub eb_pct: f64,
    pub total_mass_in: f64,
    pub total_mass_out: f64,
    pub total_h_in: f64,
    pub total_h_out: f64,
    pub max_control_err: f64,
    pub converged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonteCarloDist {
    Uniform,
    Normal,
    Laplace,
    Trapezoid,
}

impl MonteCarloDist {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uniform => "Uniform",
            Self::Normal => "Normal",
            Self::Laplace => "Laplace",
            Self::Trapezoid => "Trapezoid",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioColumn {
    pub unit_name: String,
    pub variable_name: String,
    pub unit_str: String,
    pub is_set: bool, // true=SET, false=GET
    pub cell_ref: String,
    pub monte_carlo_on: bool,
    pub dist_type: MonteCarloDist,
    pub mc_min: f64,
    pub mc_max: f64,
    pub mc_mean: f64,
    pub mc_sd: f64,
    pub mc_b: f64,
    pub mc_c: f64,
    pub round_to_nearest: Option<f64>,
    pub decimals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSheet {
    pub name: String,
    pub num_runs: usize,
    pub iteration_rounds: usize,
    pub show_mc_config: bool,
    pub show_dist_charts: bool,
    pub columns: Vec<ScenarioColumn>,
    /// Matrix [run_idx][col_idx]
    pub values: Vec<Vec<f64>>,
    pub run_rounds: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableViewMode {
    NameValueAndUnit,
    FlowTable,
}

impl TableViewMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NameValueAndUnit => "Name, value and unit",
            Self::FlowTable => "Flow Table",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTableSet {
    pub name: String,
    pub view_mode: TableViewMode,
    pub columns: usize,
    pub rows: usize,
    pub variables: Vec<String>,
    pub assigned_streams: Vec<String>,
    pub header_visible: bool,
    pub use_alias_name: bool,
    pub transpose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimTableObj {
    pub id: usize,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub page: usize,
    pub layer: usize,
    pub is_title_table: bool,
    pub size_locked: bool,
    pub cells: Vec<Vec<String>>,
    // Title Table fields (Fig. 8)
    pub title: String,
    pub organization_author: String,
    pub hsc_version: String,
    pub date: String,
    pub logo_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationKind {
    HeaderLabel,
    TextLabel,
    MultilineBox,
    Line,
    Ellipse,
    Triangle,
    Rectangle,
    RoundedRect,
    Arc,
    Polygon,
    PictureBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimAnnotation {
    pub id: usize,
    pub kind: AnnotationKind,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub page: usize,
    pub layer: usize,
    pub text: String,
    pub font_name: String,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub text_color: [u8; 3],
    pub fill_color: [u8; 3],
    pub line_color: [u8; 3],
    pub line_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimLayer {
    pub name: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimGroup {
    pub name: String,
    pub visible: bool,
    pub unit_ids: Vec<usize>,
    pub stream_ids: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimPage {
    pub name: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockState {
    Unlocked,
    DrawingLocked,       // Green inner frame (Fig. 58)
    EntireProcessLocked, // Red inner frame
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SankeyColorMode {
    FlowsheetStreamColors,
    PositiveAndNegativeColors,
    RangeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationSettings {
    pub active: bool,
    pub variable: String, // e.g. "Select visualization", "Mass Flow (t/h)", "Enthalpy Flow (kW)", "Temperature (°C)"
    pub connectivity_mode: bool,
    pub show_availability: bool,
    pub show_dataset_name: bool,
    pub show_measurement_units: bool,
    pub show_size_fraction_name: bool,
    pub use_visualization_layout: bool,
    pub show_color_picker: bool,
    pub scale_pages_separately: bool,
    pub visualize_stream_line_width: bool,
    pub stream_width_max: f32,
    pub color_mode: SankeyColorMode,
    pub color_1: [u8; 3],
    pub color_2: [u8; 3],
    pub range_min: f64,
    pub range_max: f64,
    pub use_absolute_values: bool,
    pub multiplier_pow10: i32,
}

impl Default for VisualizationSettings {
    fn default() -> Self {
        Self {
            active: false,
            variable: "Select visualization".to_string(),
            connectivity_mode: false,
            show_availability: false,
            show_dataset_name: false,
            show_measurement_units: true,
            show_size_fraction_name: false,
            use_visualization_layout: true,
            show_color_picker: false,
            scale_pages_separately: true,
            visualize_stream_line_width: true,
            stream_width_max: 2.0,
            color_mode: SankeyColorMode::FlowsheetStreamColors,
            color_1: [37, 99, 235],
            color_2: [220, 38, 38],
            range_min: 1.0,
            range_max: 20.0,
            use_absolute_values: true,
            multiplier_pow10: 1, // 10^1 (Fig. 68)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultCanvasSettings {
    pub page_bg_rgb: [u8; 3], // 255;255;255
    pub show_grid: bool,
    pub grid_style: String,   // "Points"
    pub grid_color_rgb: [u8; 3], // 176;196;222 (Fig. 15)
    pub grid_width: f32,      // 2
    pub grid_height: f32,     // 2
    pub orthogonal: bool,
    pub auto_route: bool,
    pub persist_tool: bool,
    pub snap_to_grid: bool,
    pub auto_backup_minutes: Option<u64>, // None=OFF, 15, 30, 60
}

impl Default for DefaultCanvasSettings {
    fn default() -> Self {
        Self {
            page_bg_rgb: [255, 255, 255],
            show_grid: true,
            grid_style: "Points".to_string(),
            grid_color_rgb: [176, 196, 222],
            grid_width: 2.0,
            grid_height: 2.0,
            orthogonal: true,
            auto_route: false,
            persist_tool: true,
            snap_to_grid: true,
            auto_backup_minutes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInformationData {
    pub project_name: String,
    pub project_path: String,
    pub process_name: String,
    pub technology: String,
    pub sub_technology: String,
    pub plant_type: String,
    pub products: String,
    pub more_information: String,
    pub comment: String,
    pub simulation_model: String,
    pub mass_balance: String,
    pub hsc_ver: String,
    pub last_save: String,
    pub customer_name: String,
    pub site_name: String,
    pub site_location: String,
    pub country: String,
    pub latitude: String,
    pub longitude: String,
    pub elevation: String,
    pub prepared_by: String,
    pub checked_by: String,
    pub approved_by: String,
    pub classification: String,
    pub date: String,
    pub version: String,
    pub report_number: String,
    pub kpi_rows: Vec<(String, String, String)>,
    pub notes_info: String,
    pub notes_links: String,
    pub notes_trainings: String,
}

impl Default for ProcessInformationData {
    fn default() -> Self {
        Self {
            project_name: "40.1_model".to_string(),
            project_path: "local://40.1_model.Sim".to_string(),
            process_name: "40.1_model".to_string(),
            technology: String::new(),
            sub_technology: String::new(),
            plant_type: String::new(),
            products: String::new(),
            more_information: String::new(),
            comment: String::new(),
            simulation_model: "YES".to_string(),
            mass_balance: "NO".to_string(),
            hsc_ver: "10.3.1.0".to_string(),
            last_save: "25.04.2023".to_string(),
            customer_name: String::new(),
            site_name: String::new(),
            site_location: String::new(),
            country: String::new(),
            latitude: String::new(),
            longitude: String::new(),
            elevation: String::new(),
            prepared_by: "Soryntel Clean-Room".to_string(),
            checked_by: String::new(),
            approved_by: String::new(),
            classification: "Confidential".to_string(),
            date: "24.04.2023".to_string(),
            version: "1.0".to_string(),
            report_number: String::new(),
            kpi_rows: vec![
                ("Global Mass Error".into(), "MB %".into(), "0.000".into()),
                ("Total Feed Mass".into(), "t/h".into(), "0.00".into()),
                ("Copper Recovery".into(), "%".into(), "0.00".into()),
            ],
            notes_info: "Main Process engineering documentation and operating notes.".into(),
            notes_links: "Chapter 40 — Sim Common Tools\nSoryntel Flowsheet Engine".into(),
            notes_trainings: "1. Flowsheet Drawing\n2. Stream Tables & Reports\n3. Scenario Editor & Controls".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimLogEntry {
    pub severity: LogSeverity,
    pub duration_s: f64,
    pub message: String,
    pub children: Vec<SimLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimProject {
    pub next_id: usize,
    pub pages: Vec<SimPage>,
    pub active_page: usize,
    pub layers: Vec<SimLayer>,
    pub groups: Vec<SimGroup>,
    pub units: Vec<SimUnit>,
    pub streams: Vec<SimStream>,
    pub tables: Vec<SimTableObj>,
    pub annotations: Vec<SimAnnotation>,
    pub stream_table_sets: Vec<StreamTableSet>,
    pub active_stream_table_set: usize,
    pub controls: Vec<SimControlLoop>,
    pub cell_references: Vec<SimCellReference>,
    pub scenarios: Vec<ScenarioSheet>,
    pub active_scenario: usize,
    pub process_info: ProcessInformationData,
    pub calc_settings: SimCalcSettings,
    pub canvas_settings: DefaultCanvasSettings,
    pub visualization: VisualizationSettings,
    pub lock_state: LockState,
    pub dynamic_time_s: f64,
    pub dynamic_progress_val: usize,
    pub convergence_history: Vec<ConvergenceRound>,
    pub imported_models: Vec<UnitModelCatalogEntry>,
    pub logs: Vec<SimLogEntry>,
}
