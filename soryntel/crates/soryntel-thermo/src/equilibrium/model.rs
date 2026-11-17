//! Data structures for Module 4: Equilibrium Calculations (Gibbs Energy Minimization).

use crate::{ChemicalSpecies, Phase};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Input amount unit supported in the `Species` sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquilibriumAmountUnit {
    Kmol,
    Mol,
    Kg,
    Nm3,
    GramPerLiter,
    MolPerLiter,
}

impl EquilibriumAmountUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Kmol => "kmol",
            Self::Mol => "mol",
            Self::Kg => "kg",
            Self::Nm3 => "Nm3",
            Self::GramPerLiter => "g/l",
            Self::MolPerLiter => "mol/l",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "mol" => Self::Mol,
            "kg" => Self::Kg,
            "nm3" => Self::Nm3,
            "g/l" => Self::GramPerLiter,
            "mol/l" => Self::MolPerLiter,
            _ => Self::Kmol,
        }
    }
}

/// Thermodynamic state variable combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThermodynamicMode {
    /// Fixed N, T, P (Normal isothermal-isobaric Gibbs minimization).
    Ntp,
    /// Fixed N, H, P (Adiabatic equilibrium: solve for T).
    Nhp,
    /// Fixed N, P, V (Constant volume: solve for T or P).
    Npv,
    /// Fixed N, T, P, Q (Electrochemical Cell Mode).
    Ntpq,
}

impl ThermodynamicMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ntp => "N,T,P",
            Self::Nhp => "N,H,P",
            Self::Npv => "N,P,V",
            Self::Ntpq => "N,T,P,Q",
        }
    }
}

/// Calculation mode in `System Parameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalculationMode {
    Normal,
    InfinitePhase,
    FixedActivity,
    CellMode,
}

impl CalculationMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::InfinitePhase => "Fase infinita",
            Self::FixedActivity => "Actividad fija",
            Self::CellMode => "Modo celda",
        }
    }
}

/// Step mode in `System Parameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepMode {
    AddAmount,
    FinalAmount,
    RemovePercent,
    RemovePercentFinalAmount,
    ReactorStandard,
    ReactorSheetsView,
}

impl StepMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::AddAmount => "Añadir cantidad",
            Self::FinalAmount => "Cantidad final",
            Self::RemovePercent => "Retirar % (añadir cantidad)",
            Self::RemovePercentFinalAmount => "Retirar % (cantidad final)",
            Self::ReactorStandard => "Reactor (estándar)",
            Self::ReactorSheetsView => "Reactor (vista de hojas)",
        }
    }
}

/// Physical phase classification for `Phase Data` in Cell Mode (Fig. 41).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CellPhaseType {
    Gas,
    Liquid,
    #[default]
    Solid,
    Metal,
}

impl CellPhaseType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gas => "Gas",
            Self::Liquid => "Líquido",
            Self::Solid => "Sólido",
            Self::Metal => "Metal",
        }
    }
}

/// Solution / activity coefficient model for a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolutionModel {
    IdealMixture,
    PureSubstance,
    Aqua,
    GaAsMixture,
    AlZnMixture,
}

impl SolutionModel {
    pub fn label(self) -> &'static str {
        match self {
            Self::IdealMixture => "Mezcla",
            Self::PureSubstance => "Sustancia pura",
            Self::Aqua => "Aqua",
            Self::GaAsMixture => "Mezcla Ga-As",
            Self::AlZnMixture => "Mezcla Al-Zn",
        }
    }
}

/// Independent sweep axis variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SweepVariable {
    TemperatureC,
    PressureBar,
    XAmount,
    ZAmount,
    EnthalpyMj,
    VolumeM3,
    DischargeLevelKmolE,
}

impl SweepVariable {
    pub fn label(self) -> &'static str {
        match self {
            Self::TemperatureC => "Temperatura (°C)",
            Self::PressureBar => "Presión (bar)",
            Self::XAmount => "Cantidad X (kmol)",
            Self::ZAmount => "Cantidad Z (kmol)",
            Self::EnthalpyMj => "Entalpía (MJ)",
            Self::VolumeM3 => "Volumen (m³)",
            Self::DischargeLevelKmolE => "Nivel de descarga (kmol e-)",
        }
    }
}

/// Single species entry inside an equilibrium phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumSpeciesInput {
    pub formula: String,
    pub feed_temp_c: f64,
    /// Constant activity coefficient or formula string (e.g. "1.0" or "EXP(8495/T - 2.653)").
    pub activity_coeff_expr: String,
    pub fixed_activity: Option<f64>,
    /// Discharge coefficient in Cell Mode (e.g. -2.0 for Pb, -1.0 for e-(Pb)).
    pub discharge_coeff: f64,
    pub amount_unit: EquilibriumAmountUnit,
    pub initial_amount: f64,
    pub add_x_amount: f64,
    pub add_z_amount: f64,
    pub remove_percent: f64,
    /// If true, excluded via Warnings dialog.
    pub excluded: bool,
    /// Optional explicit species thermodynamic record (e.g. parsed from `.IGI` or database).
    pub species_data: Option<ChemicalSpecies>,
    /// Optional explicit elemental stoichiometry override (from `.IGI` or formula parser).
    pub stoichiometry: BTreeMap<String, f64>,
}

/// Phase grouping in the equilibrium system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumPhase {
    pub name: String,
    pub model: SolutionModel,
    pub is_gas_phase: bool,
    pub species: Vec<EquilibriumSpeciesInput>,
    /// Electrode role in Cell Mode ("anode", "cathode", or empty).
    pub electrode_role: String,
    /// Capacitance in Farads (Cell Mode).
    pub capacitance_farads: f64,
}

/// Range specification for a single state variable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AxisRange {
    pub initial: f64,
    pub final_val: f64,
    pub steps: usize,
    pub log_scale: bool,
}

impl Default for AxisRange {
    fn default() -> Self {
        Self {
            initial: 25.0,
            final_val: 25.0,
            steps: 21,
            log_scale: false,
        }
    }
}

/// Target calculation specification (inverse root solve).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    pub enabled: bool,
    pub target_species: String,
    pub target_amount_kmol: f64,
}

impl Default for TargetSpec {
    fn default() -> Self {
        Self {
            enabled: false,
            target_species: String::new(),
            target_amount_kmol: 0.0,
        }
    }
}

fn default_one() -> f64 {
    1.0
}

/// Complete configuration for an equilibrium calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumParams {
    pub chart_header: String,
    pub calc_mode: CalculationMode,
    pub step_mode: StepMode,
    pub thermo_mode: ThermodynamicMode,
    pub num_indep_vars: usize,
    pub primary_var: SweepVariable,
    pub secondary_var: Option<SweepVariable>,
    pub temp_range: AxisRange,
    pub pressure_range: AxisRange,
    pub enthalpy_range: AxisRange,
    pub volume_range: AxisRange,
    pub discharge_range: AxisRange,
    pub x_steps: usize,
    pub z_steps: usize,
    pub infinite_phase_idx: Option<usize>,
    pub mixing_entropy_aqua: bool,
    pub criss_cobble: bool,
    pub ac_steps: usize,
    pub target: TargetSpec,
    /// Base unit multiplier: 1000.0 for kmol-scale (desktop), 1.0 for mol-scale (.IGI).
    pub input_scale_to_mol: f64,
    /// Fig. 6 (`13.4.3`): Add initial ideal gas volume at `initial_pressure_bar` as base volume.
    #[serde(default)]
    pub use_as_base_volume: bool,
    /// Fig. 6 (`13.4.3`): Initial pressure (bar) used to compute base gas volume.
    #[serde(default = "default_one")]
    pub initial_pressure_bar: f64,
    /// Fig. 6 (`13.4.7`): Automatically include H2O from aqueous g/l or mol/l solutions.
    #[serde(default)]
    pub allow_titrations: bool,
    /// Solution density in kg/l for `Allow titrations` (default 1.0 kg/l).
    #[serde(default = "default_one")]
    pub titration_density_kg_l: f64,
    /// Fig. 6: Allow constrained / damped Gibbs minimization.
    #[serde(default)]
    pub allow_constraints: bool,
    /// Units menu (`13.3.7`): Interpret Column C directly as `ln(f)` instead of `f`.
    #[serde(default)]
    pub column_c_lnf_mode: bool,
    /// Units menu (`13.3.4`): Display temperatures in Kelvin instead of °C.
    #[serde(default)]
    pub temp_unit_kelvin: bool,
}

impl Default for EquilibriumParams {
    fn default() -> Self {
        Self {
            chart_header: "New Equilibrium System".to_string(),
            calc_mode: CalculationMode::Normal,
            step_mode: StepMode::AddAmount,
            thermo_mode: ThermodynamicMode::Ntp,
            num_indep_vars: 1,
            primary_var: SweepVariable::TemperatureC,
            secondary_var: None,
            temp_range: AxisRange {
                initial: 0.0,
                final_val: 300.0,
                steps: 31,
                log_scale: false,
            },
            pressure_range: AxisRange {
                initial: 1.0,
                final_val: 1.0,
                steps: 21,
                log_scale: false,
            },
            enthalpy_range: AxisRange {
                initial: 0.0,
                final_val: 1000.0,
                steps: 21,
                log_scale: false,
            },
            volume_range: AxisRange {
                initial: 1.0,
                final_val: 1.0,
                steps: 21,
                log_scale: false,
            },
            discharge_range: AxisRange {
                initial: 0.0,
                final_val: 6.25,
                steps: 21,
                log_scale: false,
            },
            x_steps: 21,
            z_steps: 11,
            infinite_phase_idx: None,
            mixing_entropy_aqua: true,
            criss_cobble: false,
            ac_steps: 3,
            target: TargetSpec::default(),
            input_scale_to_mol: 1000.0, // 1 kmol = 1000 mol
            use_as_base_volume: false,
            initial_pressure_bar: 1.0,
            allow_titrations: false,
            titration_density_kg_l: 1.0,
            allow_constraints: false,
            column_c_lnf_mode: false,
            temp_unit_kelvin: false,
        }
    }
}

/// Serializable `.gem9` workbook bundle combining `EquilibriumSystem` and `EquilibriumParams`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumWorkbookFile {
    pub system: EquilibriumSystem,
    pub params: EquilibriumParams,
}

impl EquilibriumWorkbookFile {
    pub fn to_gem9_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_gem9_str(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| e.to_string())
    }
}

/// Complete equilibrium system definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumSystem {
    pub title: String,
    pub phases: Vec<EquilibriumPhase>,
}

/// Equilibrium state of a single species at one calculation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesEquilibriumResult {
    pub phase_idx: usize,
    pub phase_name: String,
    pub formula: String,
    pub input_amount_mol: f64,
    pub equil_amount_mol: f64,
    pub mole_fraction: f64,
    pub activity_coeff: f64,
    pub activity: f64,
    pub enthalpy_kj: f64,
    pub entropy_jk: f64,
    pub physical_phase: Phase,
}

/// Equilibrium output for a single state point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumPointResult {
    pub step_index: usize,
    pub x_value: f64,
    pub z_value: Option<f64>,
    pub temp_k: f64,
    pub pressure_bar: f64,
    pub volume_m3: f64,
    pub g_min_rt: f64,
    pub g_min_kj: f64,
    pub reaction_enthalpy_kj: f64,
    pub reaction_entropy_jk: f64,
    pub total_enthalpy_kj: f64,
    pub iterations: usize,
    pub max_mass_balance_err: f64,
    pub converged: bool,
    pub ph: Option<f64>,
    pub cell_voltage_v: Option<f64>,
    pub discharge_kmol_e: f64,
    pub species: Vec<SpeciesEquilibriumResult>,
}

/// Target calculation summary (matching Fig. 8 `Target Results`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetResultSummary {
    pub target_species: String,
    pub sought_value_kmol: f64,
    pub achieved_value_kmol: f64,
    pub variable_name: String,
    pub solved_variable_value: f64,
    pub point: EquilibriumPointResult,
}

/// Complete result of a sweep / target equilibrium calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquilibriumSweepResult {
    pub points: Vec<EquilibriumPointResult>,
    pub target_summary: Option<TargetResultSummary>,
}

/// Extrapolation warning entry for the `Warnings` dialog (Figs. 35–38).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtrapolationWarning {
    pub phase_idx: usize,
    pub species_idx: usize,
    pub formula: String,
    pub selected_t1_c: f64,
    pub selected_t2_c: f64,
    pub available_t_min_c: f64,
    pub available_t_max_c: f64,
    pub reference: String,
    pub feed_temp_c: f64,
    pub out_of_range_low: bool,
    pub out_of_range_high: bool,
    pub suggested_remove: bool,
}
