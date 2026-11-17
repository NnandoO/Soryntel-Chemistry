use crate::error::{ChemAddinError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Thermodynamic energy unit (`kJ` -> `kJ/mol` and `J/(mol·K)`, `kcal` -> `kcal/mol` and `cal/(mol·K)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnergyUnit {
    Kj,
    Kcal,
}

impl EnergyUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kj => "kJ",
            Self::Kcal => "kcal",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "kj" => Ok(Self::Kj),
            "kcal" => Ok(Self::Kcal),
            other => Err(ChemAddinError::InvalidUnit(format!(
                "Unsupported energy unit '{other}' (expected 'kJ' or 'kcal')"
            ))),
        }
    }
}

impl fmt::Display for EnergyUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Temperature unit (`C` -> °C, `K` -> Kelvin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TempUnit {
    Celsius,
    Kelvin,
}

impl TempUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Celsius => "C",
            Self::Kelvin => "K",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let normalized = raw.trim().trim_start_matches('°').to_ascii_uppercase();
        match normalized.as_str() {
            "C" | "CELSIUS" => Ok(Self::Celsius),
            "K" | "KELVIN" => Ok(Self::Kelvin),
            other => Err(ChemAddinError::InvalidUnit(format!(
                "Unsupported temperature unit '{other}' (expected 'C' or 'K')"
            ))),
        }
    }
}

impl fmt::Display for TempUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Pressure unit (`bar` or `atm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PressureUnit {
    Bar,
    Atm,
}

impl PressureUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Atm => "atm",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bar" => Ok(Self::Bar),
            "atm" => Ok(Self::Atm),
            other => Err(ChemAddinError::InvalidUnit(format!(
                "Unsupported pressure unit '{other}' (expected 'bar' or 'atm')"
            ))),
        }
    }
}

impl fmt::Display for PressureUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Thermochemical calorie to Joule exact factor (4.184 J/cal).
pub const CAL_TO_JOULE: f64 = 4.184;
/// Standard temperature reference in Kelvin (25 °C).
pub const T_REF_K: f64 = 298.15;
/// CODATA universal gas constant in J/(mol·K).
pub const R_JOULE: f64 = 8.314_462_618_153_24;
/// Standard atmospheric pressure in bar.
pub const ATM_TO_BAR: f64 = 1.01325;

/// Active session measurement unit configuration (`EnergyU`, `TempU`, `PresU`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitSystem {
    pub energy: EnergyUnit,
    pub temp: TempUnit,
    pub pressure: PressureUnit,
}

impl Default for UnitSystem {
    fn default() -> Self {
        Self {
            energy: EnergyUnit::Kj,
            temp: TempUnit::Celsius,
            pressure: PressureUnit::Bar,
        }
    }
}

impl UnitSystem {
    pub fn new(energy: EnergyUnit, temp: TempUnit, pressure: PressureUnit) -> Self {
        Self {
            energy,
            temp,
            pressure,
        }
    }

    /// Excel Add-In `EnergyU()` string (`"kJ"` or `"kcal"`).
    pub fn energy_u(&self) -> &'static str {
        self.energy.as_str()
    }

    /// Excel Add-In `TempU()` string (`"C"` or `"K"`).
    pub fn temp_u(&self) -> &'static str {
        self.temp.as_str()
    }

    /// Excel Add-In `PresU()` string (`"bar"` or `"atm"`).
    pub fn pres_u(&self) -> &'static str {
        self.pressure.as_str()
    }

    pub fn set_energy(&mut self, unit: &str) -> Result<()> {
        self.energy = EnergyUnit::parse(unit)?;
        Ok(())
    }

    pub fn set_temp(&mut self, unit: &str) -> Result<()> {
        self.temp = TempUnit::parse(unit)?;
        Ok(())
    }

    pub fn set_pressure(&mut self, unit: &str) -> Result<()> {
        self.pressure = PressureUnit::parse(unit)?;
        Ok(())
    }

    /// Convert an input temperature from the active `TempU` into Kelvin, checking `T > 0 K`.
    pub fn temp_to_kelvin(&self, temp_input: f64) -> Result<f64> {
        if !temp_input.is_finite() {
            return Err(ChemAddinError::InvalidTemperature {
                temp_k: temp_input,
                detail: "temperature is not finite".to_string(),
            });
        }
        let temp_k = match self.temp {
            TempUnit::Celsius => temp_input + 273.15,
            TempUnit::Kelvin => temp_input,
        };
        if temp_k <= 0.0 {
            return Err(ChemAddinError::InvalidTemperature {
                temp_k,
                detail: "absolute temperature must be strictly above 0 K".to_string(),
            });
        }
        Ok(temp_k)
    }

    /// Convert a Kelvin temperature into the active `TempU`.
    pub fn kelvin_to_temp(&self, temp_k: f64) -> Result<f64> {
        if !temp_k.is_finite() || temp_k <= 0.0 {
            return Err(ChemAddinError::InvalidTemperature {
                temp_k,
                detail: "absolute temperature must be finite and > 0 K".to_string(),
            });
        }
        Ok(match self.temp {
            TempUnit::Celsius => temp_k - 273.15,
            TempUnit::Kelvin => temp_k,
        })
    }

    /// Convert molar enthalpy/Gibbs energy from internal `kJ/mol` into active energy unit (`kJ/mol` or `kcal/mol`).
    pub fn energy_from_kj(&self, kj_per_mol: f64) -> f64 {
        match self.energy {
            EnergyUnit::Kj => kj_per_mol,
            EnergyUnit::Kcal => kj_per_mol / CAL_TO_JOULE,
        }
    }

    /// Convert molar energy from active energy unit (`kJ/mol` or `kcal/mol`) to internal `kJ/mol`.
    pub fn energy_to_kj(&self, val: f64) -> f64 {
        match self.energy {
            EnergyUnit::Kj => val,
            EnergyUnit::Kcal => val * CAL_TO_JOULE,
        }
    }

    /// Convert molar entropy/Cp from internal `J/(mol·K)` into active entropy unit (`J/(mol·K)` or `cal/(mol·K)`).
    pub fn entropy_from_j(&self, j_per_mol_k: f64) -> f64 {
        match self.energy {
            EnergyUnit::Kj => j_per_mol_k,
            EnergyUnit::Kcal => j_per_mol_k / CAL_TO_JOULE,
        }
    }

    /// Convert pressure from active `PresU` (`bar` or `atm`) into `bar`.
    pub fn pressure_to_bar(&self, pressure: f64) -> Result<f64> {
        if !pressure.is_finite() || pressure <= 0.0 {
            return Err(ChemAddinError::InvalidArgument(format!(
                "Pressure must be finite and > 0 (got {pressure})"
            )));
        }
        Ok(match self.pressure {
            PressureUnit::Bar => pressure,
            PressureUnit::Atm => pressure * ATM_TO_BAR,
        })
    }

    /// Convert pressure from `bar` into active `PresU` (`bar` or `atm`).
    pub fn bar_to_pressure(&self, p_bar: f64) -> Result<f64> {
        if !p_bar.is_finite() || p_bar <= 0.0 {
            return Err(ChemAddinError::InvalidArgument(format!(
                "Pressure must be finite and > 0 (got {p_bar})"
            )));
        }
        Ok(match self.pressure {
            PressureUnit::Bar => p_bar,
            PressureUnit::Atm => p_bar / ATM_TO_BAR,
        })
    }
}
