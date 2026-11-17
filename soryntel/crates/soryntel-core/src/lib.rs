//! Soryntel Core: physical constants, unit conversions, and shared domain types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Reference temperature (25.0 °C in Kelvin).
pub const T_REF_K: f64 = 298.15;

/// Standard thermochemical calorie to Joule exact factor.
pub const CAL_TO_JOULE: f64 = 4.184;

/// Universal gas constant in J/(mol·K) (CODATA).
pub const R_JOULE: f64 = 8.314_462_618_153_24;

/// Universal gas constant in cal/(mol·K).
pub const R_CAL: f64 = R_JOULE / CAL_TO_JOULE;

/// Faraday constant in C/mol (CODATA).
pub const FARADAY_C_MOL: f64 = 96_485.332_12;

/// Standard ideal gas molar volume at STP (273.15 K, 1 atm) in L/mol.
pub const STP_MOLAR_VOL_L: f64 = 22.414;

/// Stefan-Boltzmann constant in W/(m^2·K^4) (CODATA).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;

/// Standard acceleration of gravity in m/s^2.
pub const GRAVITY_STANDARD: f64 = 9.806_65;

/// Supported temperature display/input units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Kelvin,
    Fahrenheit,
}

impl TemperatureUnit {
    /// Convert a value in this unit to Kelvin.
    pub fn to_kelvin(self, value: f64) -> f64 {
        match self {
            Self::Celsius => value + 273.15,
            Self::Kelvin => value,
            Self::Fahrenheit => (value - 32.0) * (5.0 / 9.0) + 273.15,
        }
    }

    /// Convert a Kelvin temperature to this unit.
    pub fn from_kelvin(self, kelvin: f64) -> f64 {
        match self {
            Self::Celsius => kelvin - 273.15,
            Self::Kelvin => kelvin,
            Self::Fahrenheit => (kelvin - 273.15) * (9.0 / 5.0) + 32.0,
        }
    }

    /// Convert a temperature step/delta in this unit to Kelvin delta.
    pub fn step_to_kelvin(self, step: f64) -> f64 {
        match self {
            Self::Celsius | Self::Kelvin => step,
            Self::Fahrenheit => step * (5.0 / 9.0),
        }
    }

    /// Short display label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Kelvin => "K",
            Self::Fahrenheit => "°F",
        }
    }
}

/// Supported energy display/input units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EnergyUnit {
    Calories,
    Kilocalories,
    Joules,
    #[default]
    Kilojoules,
}

impl EnergyUnit {
    /// Factor to convert from Joules to this energy unit.
    pub fn from_joules_factor(self) -> f64 {
        match self {
            Self::Calories => 1.0 / CAL_TO_JOULE,
            Self::Kilocalories => 1.0 / (CAL_TO_JOULE * 1000.0),
            Self::Joules => 1.0,
            Self::Kilojoules => 1.0 / 1000.0,
        }
    }

    /// Factor to convert from Joules/(mol·K) to entropy/Cp unit (cal/(mol·K) or J/(mol·K)).
    pub fn entropy_from_joules_factor(self) -> f64 {
        match self {
            Self::Calories | Self::Kilocalories => 1.0 / CAL_TO_JOULE,
            Self::Joules | Self::Kilojoules => 1.0,
        }
    }

    /// Enthalpy/Gibbs unit label.
    pub fn energy_label(self) -> &'static str {
        match self {
            Self::Calories => "cal",
            Self::Kilocalories => "kcal",
            Self::Joules => "J",
            Self::Kilojoules => "kJ",
        }
    }

    /// Entropy/Cp unit label.
    pub fn entropy_label(self) -> &'static str {
        match self {
            Self::Calories | Self::Kilocalories => "cal/(mol*K)",
            Self::Joules | Self::Kilojoules => "J/(mol*K)",
        }
    }
}

/// Output format for single-species calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResultFormat {
    #[default]
    Normal,
    Delta,
}

/// Energy units for Module 2 (Heat and Material Balance): Mcal, MJ, kWh.
/// Note: 1 MJ/kmol == 1 kJ/mol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BalanceEnergyUnit {
    Mcal,
    #[default]
    MJ,
    Kwh,
}

impl BalanceEnergyUnit {
    /// Convert from MJ (or kJ/mol * kmol) to this unit.
    /// 1 Mcal = 4.184 MJ, 1 kWh = 3.6 MJ.
    pub fn from_mj_factor(self) -> f64 {
        match self {
            Self::Mcal => 1.0 / CAL_TO_JOULE,
            Self::MJ => 1.0,
            Self::Kwh => 1.0 / 3.6,
        }
    }

    /// Convert from a value in this unit back to MJ.
    pub fn to_mj_factor(self) -> f64 {
        match self {
            Self::Mcal => CAL_TO_JOULE,
            Self::MJ => 1.0,
            Self::Kwh => 3.6,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mcal => "Mcal",
            Self::MJ => "MJ",
            Self::Kwh => "kWh",
        }
    }
}

/// Common error type for Soryntel Chemistry.
#[derive(Debug, Error)]
pub enum SoryntelError {
    #[error("Temperatura no válida: {0} K (debe ser finita y mayor que cero)")]
    InvalidTemperature(f64),

    #[error("Presión no válida: {0} bar (debe ser finita y mayor que cero)")]
    InvalidPressure(f64),

    #[error("Intervalo de temperatura no válido: desde={from}, hasta={to}, paso={step}. Use valores finitos, paso positivo y un máximo de 100001 puntos")]
    InvalidRange { from: f64, to: f64, step: f64 },

    #[error("Error al interpretar la fórmula: {0}")]
    ParseError(String),

    #[error("Error de balance estequiométrico: {0}")]
    BalanceError(String),

    #[error("Especie no encontrada en la base de datos: {0}")]
    SpeciesNotFound(String),

    #[error("Error de lectura o escritura en la base de datos: {0}")]
    DatabaseError(String),

    #[error("Error de sintaxis en la fórmula de celda: {0}")]
    FormulaSyntaxError(String),

    #[error("Referencia de celda no válida: {0}")]
    InvalidCellReference(String),

    #[error("La referencia circular no convergió: {0}")]
    CircularConvergenceFailed(String),

    /// El cálculo objetivo no convergió o la raíz no está acotada: {0}
    #[error("El cálculo objetivo no convergió o la raíz no está acotada: {0}")]
    TargetNotBracketed(String),

    #[error("Error al calcular pérdidas de calor: {0}")]
    HeatLossError(String),
}

/// Heat flow rate units for Module 3 (Heat Loss). Base SI unit is Watt (W = J/s).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HeatFlowUnit {
    Watt,
    #[default]
    Kilowatt,
    Megawatt,
    KcalPerHour,
    BtuPerHour,
}

impl HeatFlowUnit {
    /// Factor to multiply a value in this unit to obtain Watts (W).
    pub fn to_watts_factor(self) -> f64 {
        match self {
            Self::Watt => 1.0,
            Self::Kilowatt => 1000.0,
            Self::Megawatt => 1_000_000.0,
            Self::KcalPerHour => (CAL_TO_JOULE * 1000.0) / 3600.0,
            Self::BtuPerHour => 1055.055_852_62 / 3600.0,
        }
    }

    /// Factor to multiply Watts (W) to obtain this unit.
    pub fn from_watts_factor(self) -> f64 {
        1.0 / self.to_watts_factor()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Watt => "W",
            Self::Kilowatt => "kW",
            Self::Megawatt => "MW",
            Self::KcalPerHour => "kcal/h",
            Self::BtuPerHour => "BTU/h",
        }
    }

    pub fn flux_label(self) -> &'static str {
        match self {
            Self::Watt => "W/m²",
            Self::Kilowatt => "kW/m²",
            Self::Megawatt => "MW/m²",
            Self::KcalPerHour => "kcal/(h·m²)",
            Self::BtuPerHour => "BTU/(h·ft²)",
        }
    }
}

/// Length / thickness units for Module 3 (Heat Loss). Base SI unit is Meter (m).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LengthUnit {
    #[default]
    Meter,
    Centimeter,
    Millimeter,
    Inch,
}

impl LengthUnit {
    pub fn to_meters_factor(self) -> f64 {
        match self {
            Self::Meter => 1.0,
            Self::Centimeter => 0.01,
            Self::Millimeter => 0.001,
            Self::Inch => 0.0254,
        }
    }

    pub fn from_meters_factor(self) -> f64 {
        1.0 / self.to_meters_factor()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Meter => "m",
            Self::Centimeter => "cm",
            Self::Millimeter => "mm",
            Self::Inch => "in",
        }
    }
}
