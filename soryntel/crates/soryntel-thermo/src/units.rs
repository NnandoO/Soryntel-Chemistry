//! Module 14 domain engine: Units Conversion Calculator & Reference Workbook (`UNITS4.UNI`).
//!
//! Provides:
//! - 90 physical, thermochemical, electromagnetic & engineering quantities (~444 units)
//!   with 21 SI metric prefixes (`Y` 10^24 .. `y` 10^-24).
//! - Exact linear factor conversion `x2 = x1 * (c2 / c1) * (f1 / f2)` and affine
//!   Temperature conversion (`K`, `°C`, `°F`, `°R`) with optional differential `ΔT` mode.
//! - Multi-row persistent `Convert` sheet log (matching HSC6 `Fig. 1`), editable 3-row-per-quantity
//!   `Units` master database (`Fig. 2`), `Constants` (56 CODATA constants), `Mesh` (Tyler/ASTM sieves
//!   + log-log interpolation), `Air` (dry air + moist air matrix + `WaterInAir 3D` grid), and `H2O`
//!   vapour pressure table integrated with `crate::water`.

use serde::{Deserialize, Serialize};

use crate::balance::h2o_saturation_pressure_bar;
use crate::units_data::{
    RAW_CONSTANTS, RAW_DRY_AIR, RAW_H2O_VAPOUR, RAW_MESH, RAW_QUANTITIES,
};
use crate::water::{sat_pressure_gs_bar, H2O_TT_K};

/// 1 standard atmosphere in bar (exact CODATA/NIST definition).
pub const STD_ATM_BAR: f64 = 1.01325;

/// Vapour density scale factor at 0 °C, 1 atm assuming ideal gas `H2O(g)`:
/// `MW(H2O) / Vm0 = 18.01528 / 22.414 * 1000 = 803.751 g/Nm³` (HSC6 `UNITS4.UNI` uses `803.748`).
pub const H2O_IDEAL_NM3_FACTOR: f64 = 803.748;

/// 21 SI metric prefixes supported by HSC6 Units Converter (`Y` .. `y`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SiPrefix {
    Yotta,
    Zetta,
    Exa,
    Peta,
    Tera,
    Giga,
    Mega,
    Kilo,
    Hecto,
    Deca,
    #[default]
    None,
    Deci,
    Centi,
    Milli,
    Micro,
    Nano,
    Pico,
    Femto,
    Atto,
    Zepto,
    Yocto,
}

impl SiPrefix {
    pub const ALL: [SiPrefix; 21] = [
        Self::None,
        Self::Yotta,
        Self::Zetta,
        Self::Exa,
        Self::Peta,
        Self::Tera,
        Self::Giga,
        Self::Mega,
        Self::Kilo,
        Self::Hecto,
        Self::Deca,
        Self::Deci,
        Self::Centi,
        Self::Milli,
        Self::Micro,
        Self::Nano,
        Self::Pico,
        Self::Femto,
        Self::Atto,
        Self::Zepto,
        Self::Yocto,
    ];

    /// Exponent `e_p` in `10^e_p`.
    pub fn exponent(self) -> i32 {
        match self {
            Self::Yotta => 24,
            Self::Zetta => 21,
            Self::Exa => 18,
            Self::Peta => 15,
            Self::Tera => 12,
            Self::Giga => 9,
            Self::Mega => 6,
            Self::Kilo => 3,
            Self::Hecto => 2,
            Self::Deca => 1,
            Self::None => 0,
            Self::Deci => -1,
            Self::Centi => -2,
            Self::Milli => -3,
            Self::Micro => -6,
            Self::Nano => -9,
            Self::Pico => -12,
            Self::Femto => -15,
            Self::Atto => -18,
            Self::Zepto => -21,
            Self::Yocto => -24,
        }
    }

    /// Multiplicative factor `10^e_p`.
    pub fn factor(self) -> f64 {
        10_f64.powi(self.exponent())
    }

    /// Short symbol (`"k"`, `"m"`, `""`, ...).
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Yotta => "Y",
            Self::Zetta => "Z",
            Self::Exa => "E",
            Self::Peta => "P",
            Self::Tera => "T",
            Self::Giga => "G",
            Self::Mega => "M",
            Self::Kilo => "k",
            Self::Hecto => "h",
            Self::Deca => "da",
            Self::None => "",
            Self::Deci => "d",
            Self::Centi => "c",
            Self::Milli => "m",
            Self::Micro => "µ",
            Self::Nano => "n",
            Self::Pico => "p",
            Self::Femto => "f",
            Self::Atto => "a",
            Self::Zepto => "z",
            Self::Yocto => "y",
        }
    }

    /// Descriptive label matching HSC6 Fig. 1 (`"Kilo (k) 10^3"`, `"   "` when `None`).
    pub fn combo_label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Yotta => "Yotta (Y) 10^24",
            Self::Zetta => "Zetta (Z) 10^21",
            Self::Exa => "Exa (E) 10^18",
            Self::Peta => "Peta (P) 10^15",
            Self::Tera => "Tera (T) 10^12",
            Self::Giga => "Giga (G) 10^9",
            Self::Mega => "Mega (M) 10^6",
            Self::Kilo => "Kilo (k) 10^3",
            Self::Hecto => "Hecto (h) 10^2",
            Self::Deca => "Deca (da) 10^1",
            Self::Deci => "Deci (d) 10^-1",
            Self::Centi => "Centi (c) 10^-2",
            Self::Milli => "Milli (m) 10^-3",
            Self::Micro => "Micro (µ) 10^-6",
            Self::Nano => "Nano (n) 10^-9",
            Self::Pico => "Pico (p) 10^-12",
            Self::Femto => "Femto (f) 10^-15",
            Self::Atto => "Atto (a) 10^-18",
            Self::Zepto => "Zepto (z) 10^-21",
            Self::Yocto => "Yocto (y) 10^-24",
        }
    }

    /// Compose final display unit string such as `"kJ"` or `"cal(th)"`.
    pub fn format_unit(self, base_symbol: &str) -> String {
        format!("{}{}", self.symbol(), base_symbol)
    }
}

/// Single unit entry inside a quantity row block in the `Units` sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitEntry {
    pub name: String,
    pub symbol: String,
    /// Number of this unit equal to 1 base unit of its quantity (`c_base = 1.0`).
    pub coef: f64,
}

/// A physical quantity group (3 rows in HSC6 `Units` sheet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitQuantity {
    pub name: String,
    pub symbol: String,
    pub si_expr: String,
    pub units: Vec<UnitEntry>,
}

/// A fundamental constant in the `Constants` sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantEntry {
    pub name: String,
    pub symbol: String,
    pub value: f64,
    pub unit: String,
    pub alt_value: Option<(f64, String)>,
}

/// Grain size row in the `Mesh` sheet.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MeshEntry {
    pub mesh: f64,
    pub mm: f64,
}

/// Dry air gas species composition row in the `Air` sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryAirComponent {
    pub species: String,
    pub vol_pct: f64,
    pub wt_pct: f64,
}

/// Saturated water vapour pressure row in the `H2O` sheet.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct H2OVapourRow {
    pub temp_c: f64,
    pub pressure_kn_m2: f64,
    pub pressure_atm: f64,
}

/// One row of the `Convert` sheet (`Data 1 | Units 1 | Data 2 | Units 2`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertRow {
    pub data1: Option<f64>,
    pub unit1: String,
    pub data2: Option<f64>,
    pub unit2: String,
}

/// Point-wise moist air humidity evaluation result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AirHumidityPoint {
    pub temp_c: f64,
    pub rh_pct: f64,
    pub total_p_bar: f64,
    pub p_sat_bar: f64,
    pub p_h2o_bar: f64,
    pub vol_pct: f64,
    pub g_per_nm3: f64,
}

/// 2D grid for HSC6 `WaterInAir` and `WaterInAir 3D` (`Fig. 3`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirHumidityGrid {
    /// Temperatures in descending/ascending order (`-50..102 °C`).
    pub temps_c: Vec<f64>,
    /// Relative humidities (`10..100 %` or custom step).
    pub rh_pcts: Vec<f64>,
    /// Row-major (`temp_idx * rh_pcts.len() + rh_idx`) water vol-% (`0..100`).
    pub vol_pct_grid: Vec<f64>,
    /// Row-major water g/Nm³ (`0..803.748`).
    pub g_nm3_grid: Vec<f64>,
}

/// Complete editable 6-sheet `Measure Units` workbook (`UNITS4.UNI`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitsWorkbook {
    pub convert_rows: Vec<ConvertRow>,
    pub quantities: Vec<UnitQuantity>,
    pub constants: Vec<ConstantEntry>,
    pub mesh_table: Vec<MeshEntry>,
    pub dry_air: Vec<DryAirComponent>,
    pub h2o_vapour: Vec<H2OVapourRow>,
}

impl UnitsWorkbook {
    /// Construct default workbook seeded with all 90 quantities (~444 units),
    /// 56 constants, 15 mesh rows, dry air, 88 H2O rows, and HSC6 `Fig. 1` Convert log.
    pub fn default_hsc6() -> Self {
        let quantities = RAW_QUANTITIES
            .iter()
            .map(|q| UnitQuantity {
                name: q.name.to_string(),
                symbol: q.symbol.to_string(),
                si_expr: q.si_expr.to_string(),
                units: q
                    .units
                    .iter()
                    .map(|u| UnitEntry {
                        name: u.name.to_string(),
                        symbol: u.symbol.to_string(),
                        coef: u.coef,
                    })
                    .collect(),
            })
            .collect();

        let constants = RAW_CONSTANTS
            .iter()
            .map(|c| ConstantEntry {
                name: c.name.to_string(),
                symbol: c.symbol.to_string(),
                value: c.value,
                unit: c.unit.to_string(),
                alt_value: c.alt_value.map(|(v, u)| (v, u.to_string())),
            })
            .collect();

        let mesh_table = RAW_MESH
            .iter()
            .map(|&(mesh, mm)| MeshEntry { mesh, mm })
            .collect();

        let dry_air = RAW_DRY_AIR
            .iter()
            .map(|&(sp, vol_pct, wt_pct)| DryAirComponent {
                species: sp.to_string(),
                vol_pct,
                wt_pct,
            })
            .collect();

        let h2o_vapour = RAW_H2O_VAPOUR
            .iter()
            .map(|&(temp_c, pressure_kn_m2, pressure_atm)| H2OVapourRow {
                temp_c,
                pressure_kn_m2,
                pressure_atm,
            })
            .collect();

        // Pre-populated example rows matching HSC6 Manual Fig. 1 (`A2..D7`)
        let convert_rows = vec![
            ConvertRow {
                data1: Some(1.0),
                unit1: "cal(th)".to_string(),
                data2: Some(4.184000633),
                unit2: "J".to_string(),
            },
            ConvertRow {
                data1: Some(0.2390057),
                unit1: "cal(th)".to_string(),
                data2: Some(1.0),
                unit2: "J".to_string(),
            },
            ConvertRow {
                data1: Some(25.0),
                unit1: "°C".to_string(),
                data2: Some(298.15),
                unit2: "K".to_string(),
            },
            ConvertRow {
                data1: Some(-17.77777778),
                unit1: "°C".to_string(),
                data2: Some(0.0),
                unit2: "°F".to_string(),
            },
            ConvertRow {
                data1: Some(3.785411818),
                unit1: "l".to_string(),
                data2: Some(1.0),
                unit2: "gal(US)".to_string(),
            },
            ConvertRow {
                data1: Some(22.0),
                unit1: "kJ".to_string(),
                data2: Some(5258.1254),
                unit2: "cal(th)".to_string(),
            },
        ];

        Self {
            convert_rows,
            quantities,
            constants,
            mesh_table,
            dry_air,
            h2o_vapour,
        }
    }

    /// Find quantity index by case-insensitive exact or partial name match.
    pub fn find_quantity_index(&self, name: &str) -> Option<usize> {
        let needle = name.trim().to_ascii_lowercase();
        self.quantities
            .iter()
            .position(|q| q.name.to_ascii_lowercase() == needle)
            .or_else(|| {
                self.quantities
                    .iter()
                    .position(|q| q.name.to_ascii_lowercase().contains(&needle))
            })
    }

    /// Find unit index inside a quantity by matching symbol or name.
    pub fn find_unit_index(&self, qty_idx: usize, sym_or_name: &str) -> Option<usize> {
        let q = self.quantities.get(qty_idx)?;
        let needle = sym_or_name.trim().to_ascii_lowercase();
        q.units
            .iter()
            .position(|u| u.symbol.to_ascii_lowercase() == needle)
            .or_else(|| {
                q.units
                    .iter()
                    .position(|u| u.name.to_ascii_lowercase() == needle)
            })
    }

    /// Total number of units summed across all quantities.
    pub fn total_unit_count(&self) -> usize {
        self.quantities.iter().map(|q| q.units.len()).sum()
    }

    /// Convert a numeric value from `(unit1_idx, prefix1)` to `(unit2_idx, prefix2)` inside `qty_idx`.
    /// Handles both linear factor quantities (`x2 = x1 * (c2 / c1) * (f1 / f2)`) and affine
    /// `Temperature` scales (`K`, `°C`, `°F`, `°R`), optionally in differential `ΔT` mode.
    pub fn convert_value(
        &self,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        unit2_idx: usize,
        prefix2: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Result<f64, String> {
        if !value.is_finite() {
            return Err("Input value must be a finite number.".to_string());
        }
        let qty = self
            .quantities
            .get(qty_idx)
            .ok_or_else(|| "Selected quantity index is out of bounds.".to_string())?;
        let u1 = qty
            .units
            .get(unit1_idx)
            .ok_or_else(|| "Source unit index is out of bounds.".to_string())?;
        let u2 = qty
            .units
            .get(unit2_idx)
            .ok_or_else(|| "Target unit index is out of bounds.".to_string())?;

        if qty.name.eq_ignore_ascii_case("Temperature") {
            return convert_temperature(
                &u1.symbol,
                prefix1,
                &u2.symbol,
                prefix2,
                value,
                delta_temp_mode,
            );
        }

        if !u1.coef.is_finite() || u1.coef <= 0.0 || !u2.coef.is_finite() || u2.coef <= 0.0 {
            return Err(format!(
                "Invalid conversion coefficient for '{}' or '{}'.",
                u1.symbol, u2.symbol
            ));
        }

        let f1 = prefix_factor_for_unit(prefix1, &u1.symbol);
        let f2 = prefix_factor_for_unit(prefix2, &u2.symbol);
        let x_base = value * (f1 / u1.coef);
        let x_out = x_base * (u2.coef / f2);
        if !x_out.is_finite() {
            return Err("Conversion overflowed floating-point range.".to_string());
        }
        Ok(x_out)
    }

    /// Resolve a display label like `"kJ"`, `"cal(th)"`, `"°C"`, `"cm2"` into `(prefix, unit_idx)`.
    pub fn resolve_labeled_unit(&self, qty_idx: usize, label: &str) -> Option<(SiPrefix, usize)> {
        let q = self.quantities.get(qty_idx)?;
        let trimmed = label.trim();
        if let Some(uidx) = q.units.iter().position(|u| u.symbol == trimmed) {
            return Some((SiPrefix::None, uidx));
        }
        for p in SiPrefix::ALL {
            if p == SiPrefix::None {
                continue;
            }
            let psym = p.symbol();
            if let Some(rest) = trimmed.strip_prefix(psym) {
                if let Some(uidx) = q.units.iter().position(|u| u.symbol == rest) {
                    return Some((p, uidx));
                }
            }
        }
        None
    }

    /// Convert a value to every unit defined inside `qty_idx` (useful for `Sampler` / multi-unit view).
    pub fn convert_all_equivalents(
        &self,
        qty_idx: usize,
        unit1_idx: usize,
        prefix1: SiPrefix,
        value: f64,
        delta_temp_mode: bool,
    ) -> Vec<(String, String, Result<f64, String>)> {
        let Some(qty) = self.quantities.get(qty_idx) else {
            return Vec::new();
        };
        qty.units
            .iter()
            .enumerate()
            .map(|(u2_idx, u2)| {
                let res = self.convert_value(
                    qty_idx,
                    unit1_idx,
                    prefix1,
                    u2_idx,
                    SiPrefix::None,
                    value,
                    delta_temp_mode,
                );
                (u2.name.clone(), u2.symbol.clone(), res)
            })
            .collect()
    }

    /// Append a custom unit to an existing quantity.
    pub fn add_unit_to_quantity(
        &mut self,
        qty_idx: usize,
        name: String,
        symbol: String,
        coef: f64,
    ) -> Result<(), String> {
        if name.trim().is_empty() || symbol.trim().is_empty() {
            return Err("Unit name and symbol cannot be empty.".to_string());
        }
        if !coef.is_finite() || coef <= 0.0 {
            return Err("Conversion coefficient must be > 0.".to_string());
        }
        let q = self
            .quantities
            .get_mut(qty_idx)
            .ok_or_else(|| "Invalid quantity index.".to_string())?;
        q.units.push(UnitEntry {
            name: name.trim().to_string(),
            symbol: symbol.trim().to_string(),
            coef,
        });
        Ok(())
    }

    /// Add a brand-new quantity with its base reference unit (`coef = 1.0`).
    pub fn add_quantity(
        &mut self,
        qty_name: String,
        qty_symbol: String,
        si_expr: String,
        base_unit_name: String,
        base_unit_symbol: String,
    ) -> Result<usize, String> {
        if qty_name.trim().is_empty()
            || base_unit_name.trim().is_empty()
            || base_unit_symbol.trim().is_empty()
        {
            return Err("Quantity name, base unit name, and base symbol are required.".to_string());
        }
        self.quantities.push(UnitQuantity {
            name: qty_name.trim().to_string(),
            symbol: qty_symbol.trim().to_string(),
            si_expr: si_expr.trim().to_string(),
            units: vec![UnitEntry {
                name: base_unit_name.trim().to_string(),
                symbol: base_unit_symbol.trim().to_string(),
                coef: 1.0,
            }],
        });
        Ok(self.quantities.len() - 1)
    }

    /// Interpolate opening size in `mm` from a Tyler/ASTM `mesh` number using log-log piecewise fit.
    pub fn interpolate_mesh_to_mm(&self, mesh: f64) -> Option<f64> {
        if !mesh.is_finite() || mesh <= 0.0 || self.mesh_table.is_empty() {
            return None;
        }
        let mut pts: Vec<MeshEntry> = self
            .mesh_table
            .iter()
            .copied()
            .filter(|m| m.mesh > 0.0 && m.mm > 0.0)
            .collect();
        pts.sort_by(|a, b| a.mesh.total_cmp(&b.mesh));
        if pts.len() < 2 {
            return pts.first().map(|p| p.mm);
        }
        for p in &pts {
            if (p.mesh - mesh).abs() < 1e-12 {
                return Some(p.mm);
            }
        }
        let seg = if mesh < pts[0].mesh {
            (&pts[0], &pts[1])
        } else if mesh > pts[pts.len() - 1].mesh {
            (&pts[pts.len() - 2], &pts[pts.len() - 1])
        } else {
            let idx = pts
                .windows(2)
                .position(|w| mesh >= w[0].mesh && mesh <= w[1].mesh)
                .unwrap_or(0);
            (&pts[idx], &pts[idx + 1])
        };
        let t = (mesh.ln() - seg.0.mesh.ln()) / (seg.1.mesh.ln() - seg.0.mesh.ln());
        let ln_d = seg.0.mm.ln() + t * (seg.1.mm.ln() - seg.0.mm.ln());
        Some(ln_d.exp())
    }

    /// Interpolate equivalent `mesh` number from an opening size in `mm` using log-log piecewise fit.
    pub fn interpolate_mm_to_mesh(&self, mm: f64) -> Option<f64> {
        if !mm.is_finite() || mm <= 0.0 || self.mesh_table.is_empty() {
            return None;
        }
        let mut pts: Vec<MeshEntry> = self
            .mesh_table
            .iter()
            .copied()
            .filter(|m| m.mesh > 0.0 && m.mm > 0.0)
            .collect();
        // Ascending in mm
        pts.sort_by(|a, b| a.mm.total_cmp(&b.mm));
        if pts.len() < 2 {
            return pts.first().map(|p| p.mesh);
        }
        for p in &pts {
            if (p.mm - mm).abs() < 1e-12 {
                return Some(p.mesh);
            }
        }
        let seg = if mm < pts[0].mm {
            (&pts[0], &pts[1])
        } else if mm > pts[pts.len() - 1].mm {
            (&pts[pts.len() - 2], &pts[pts.len() - 1])
        } else {
            let idx = pts
                .windows(2)
                .position(|w| mm >= w[0].mm && mm <= w[1].mm)
                .unwrap_or(0);
            (&pts[idx], &pts[idx + 1])
        };
        let t = (mm.ln() - seg.0.mm.ln()) / (seg.1.mm.ln() - seg.0.mm.ln());
        let ln_m = seg.0.mesh.ln() + t * (seg.1.mesh.ln() - seg.0.mesh.ln());
        Some(ln_m.exp())
    }
}

fn prefix_factor_for_unit(prefix: SiPrefix, base_symbol: &str) -> f64 {
    if prefix == SiPrefix::None {
        return 1.0;
    }
    let first_tok = base_symbol.trim().split([' ', '/']).next().unwrap_or("");
    let power = match first_tok {
        "m2" => 2,
        "m3" => 3,
        "m-1" => -1,
        "m-2" => -2,
        "m-3" => -3,
        _ => 1,
    };
    10_f64.powi(prefix.exponent() * power)
}

fn temp_scale_kind(sym: &str) -> Option<&'static str> {
    let s = sym.trim();
    if s.eq_ignore_ascii_case("K") {
        Some("K")
    } else if s == "°C" || s.eq_ignore_ascii_case("C") || s.eq_ignore_ascii_case("degC") {
        Some("C")
    } else if s == "°F" || s.eq_ignore_ascii_case("F") || s.eq_ignore_ascii_case("degF") {
        Some("F")
    } else if s == "°R" || s.eq_ignore_ascii_case("R") || s.eq_ignore_ascii_case("degR") {
        Some("R")
    } else {
        None
    }
}

/// Convert temperature between `K`, `°C`, `°F`, `°R` (with SI prefixes supported for Kelvin
/// and differential `ΔT` mode when `delta_mode == true`).
pub fn convert_temperature(
    u1_sym: &str,
    prefix1: SiPrefix,
    u2_sym: &str,
    prefix2: SiPrefix,
    value: f64,
    delta_mode: bool,
) -> Result<f64, String> {
    let k1 = temp_scale_kind(u1_sym).ok_or_else(|| format!("Unknown temperature unit '{u1_sym}'"))?;
    let k2 = temp_scale_kind(u2_sym).ok_or_else(|| format!("Unknown temperature unit '{u2_sym}'"))?;

    if delta_mode {
        let dk = match k1 {
            "K" => value * prefix1.factor(),
            "C" => value,
            "F" | "R" => value * (5.0 / 9.0),
            _ => unreachable!(),
        };
        let out = match k2 {
            "K" => dk / prefix2.factor(),
            "C" => dk,
            "F" | "R" => dk * (9.0 / 5.0),
            _ => unreachable!(),
        };
        return Ok(out);
    }

    let t_k = match k1 {
        "K" => value * prefix1.factor(),
        "C" => value + 273.15,
        "F" => (value - 32.0) * (5.0 / 9.0) + 273.15,
        "R" => value * (5.0 / 9.0),
        _ => unreachable!(),
    };
    if t_k < -1e-6 {
        return Err(format!(
            "Temperature {t_k:.4} K is below absolute zero (0 K / -273.15 °C)."
        ));
    }
    let t_k_clamped = t_k.max(0.0);
    let out = match k2 {
        "K" => t_k_clamped / prefix2.factor(),
        "C" => t_k_clamped - 273.15,
        "F" => (t_k_clamped - 273.15) * (9.0 / 5.0) + 32.0,
        "R" => t_k_clamped * (9.0 / 5.0),
        _ => unreachable!(),
    };
    Ok(out)
}

/// Evaluate water vapour pressure, `H2O vol-%` and `H2O g/Nm³` in moist air at `(temp_c, rh_pct, total_p_bar)`.
/// Shares IAPWS saturation equations with `crate::water`.
pub fn compute_air_humidity_point(
    temp_c: f64,
    rh_pct: f64,
    total_p_bar: f64,
) -> Result<AirHumidityPoint, String> {
    if !temp_c.is_finite() || !rh_pct.is_finite() || !total_p_bar.is_finite() || total_p_bar <= 0.0 {
        return Err("Invalid temperature, RH%, or total pressure.".to_string());
    }
    let t_k = (temp_c + 273.15).clamp(173.15, 647.0);
    let rh = rh_pct.clamp(0.0, 100.0);
    let p_sat_bar = if t_k >= H2O_TT_K {
        h2o_saturation_pressure_bar(t_k)
    } else {
        sat_pressure_gs_bar(t_k)
    };
    let p_h2o_bar = (p_sat_bar * (rh / 100.0)).min(total_p_bar * (rh / 100.0));
    let vol_pct = (100.0 * p_h2o_bar / total_p_bar).clamp(0.0, 100.0);
    let g_per_nm3 = (vol_pct / 100.0) * H2O_IDEAL_NM3_FACTOR;
    Ok(AirHumidityPoint {
        temp_c,
        rh_pct: rh,
        total_p_bar,
        p_sat_bar,
        p_h2o_bar,
        vol_pct,
        g_per_nm3,
    })
}

/// Build standard HSC6 `Air` sheet & `WaterInAir 3D` matrix (`102 °C` down to `-50 °C` step `2 °C`,
/// `RH = 100%..10%` step `10%`, total pressure = `1 atm = 1.01325 bar`).
pub fn build_air_humidity_grid_descending() -> AirHumidityGrid {
    let mut temps_c = Vec::with_capacity(77);
    let mut t = 102;
    while t >= -50 {
        temps_c.push(t as f64);
        t -= 2;
    }
    let rh_pcts = vec![100.0, 90.0, 80.0, 70.0, 60.0, 50.0, 40.0, 30.0, 20.0, 10.0];
    let mut vol_pct_grid = Vec::with_capacity(temps_c.len() * rh_pcts.len());
    let mut g_nm3_grid = Vec::with_capacity(temps_c.len() * rh_pcts.len());
    for &tc in &temps_c {
        for &rh in &rh_pcts {
            let pt = compute_air_humidity_point(tc, rh, STD_ATM_BAR).unwrap_or(AirHumidityPoint {
                temp_c: tc,
                rh_pct: rh,
                total_p_bar: STD_ATM_BAR,
                p_sat_bar: 0.0,
                p_h2o_bar: 0.0,
                vol_pct: 0.0,
                g_per_nm3: 0.0,
            });
            vol_pct_grid.push(pt.vol_pct);
            g_nm3_grid.push(pt.g_per_nm3);
        }
    }
    AirHumidityGrid {
        temps_c,
        rh_pcts,
        vol_pct_grid,
        g_nm3_grid,
    }
}

/// Build smooth ascending mesh for `WaterInAir 3D` (`Fig. 3`: `T = -50..102 °C`, `RH = 10..100 %`).
pub fn build_air_humidity_3d_surface() -> AirHumidityGrid {
    let temps_c: Vec<f64> = (-50..=102).step_by(4).map(|t| t as f64).collect();
    let rh_pcts: Vec<f64> = (10..=100).step_by(5).map(|r| r as f64).collect();
    let mut vol_pct_grid = Vec::with_capacity(temps_c.len() * rh_pcts.len());
    let mut g_nm3_grid = Vec::with_capacity(temps_c.len() * rh_pcts.len());
    for &tc in &temps_c {
        for &rh in &rh_pcts {
            let pt = compute_air_humidity_point(tc, rh, STD_ATM_BAR).unwrap_or(AirHumidityPoint {
                temp_c: tc,
                rh_pct: rh,
                total_p_bar: STD_ATM_BAR,
                p_sat_bar: 0.0,
                p_h2o_bar: 0.0,
                vol_pct: 0.0,
                g_per_nm3: 0.0,
            });
            vol_pct_grid.push(pt.vol_pct);
            g_nm3_grid.push(pt.g_per_nm3);
        }
    }
    AirHumidityGrid {
        temps_c,
        rh_pcts,
        vol_pct_grid,
        g_nm3_grid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_validation_u1_to_u6() {
        let wb = UnitsWorkbook::default_hsc6();

        // U4: Integrity counts (90 quantities, >= 440 units, 56 constants, 15 mesh, 88 H2O)
        assert_eq!(wb.quantities.len(), 90);
        assert!(wb.total_unit_count() >= 440);
        assert_eq!(wb.constants.len(), 56);
        assert_eq!(wb.mesh_table.len(), 15);
        assert_eq!(wb.h2o_vapour.len(), 88);
        assert!(!wb.dry_air.is_empty());

        // U1: Energy conversion & SI prefix parity (22 kJ -> 5258.1254 cal(th), Fig. 1)
        let q_e = wb.find_quantity_index("Energy, work").unwrap();
        let u_j = wb.find_unit_index(q_e, "J").unwrap();
        let u_cal = wb.find_unit_index(q_e, "cal(th)").unwrap();
        let res_cal = wb
            .convert_value(q_e, u_j, SiPrefix::Kilo, u_cal, SiPrefix::None, 22.0, false)
            .unwrap();
        assert!((res_cal - 5258.1254).abs() < 0.01, "22 kJ -> cal(th) = {res_cal}");
        let back_kj = wb
            .convert_value(q_e, u_cal, SiPrefix::None, u_j, SiPrefix::Kilo, res_cal, false)
            .unwrap();
        assert!((back_kj - 22.0).abs() < 1e-9);

        // U2: Pressure (1.0 atm -> bar & Pa)
        let q_p = wb.find_quantity_index("Pressure, stress").unwrap();
        let u_atm = wb.find_unit_index(q_p, "atm").unwrap();
        let u_bar = wb.find_unit_index(q_p, "bar").unwrap();
        let u_pa = wb.find_unit_index(q_p, "Pa").unwrap();
        let bar = wb
            .convert_value(q_p, u_atm, SiPrefix::None, u_bar, SiPrefix::None, 1.0, false)
            .unwrap();
        let pa = wb
            .convert_value(q_p, u_atm, SiPrefix::None, u_pa, SiPrefix::None, 1.0, false)
            .unwrap();
        assert!((bar - 1.01325).abs() < 1e-3);
        assert!((pa - 101325.0).abs() < 10.0);

        // U3: Affine Temperature (25 °C = 298.15 K, 0 °F = -17.7778 °C, Fig. 1) + Mass mg -> kg
        let q_t = wb.find_quantity_index("Temperature").unwrap();
        let u_c = wb.find_unit_index(q_t, "°C").unwrap();
        let u_k = wb.find_unit_index(q_t, "K").unwrap();
        let u_f = wb.find_unit_index(q_t, "°F").unwrap();
        let tk = wb
            .convert_value(q_t, u_c, SiPrefix::None, u_k, SiPrefix::None, 25.0, false)
            .unwrap();
        assert!((tk - 298.15).abs() < 1e-9);
        let tc = wb
            .convert_value(q_t, u_f, SiPrefix::None, u_c, SiPrefix::None, 0.0, false)
            .unwrap();
        assert!((tc + 17.77777778).abs() < 1e-6);
        assert!(wb
            .convert_value(q_t, u_c, SiPrefix::None, u_k, SiPrefix::None, -300.0, false)
            .is_err());

        let q_m = wb.find_quantity_index("Mass").unwrap();
        let u_g = wb.find_unit_index(q_m, "g").unwrap();
        let u_kg = wb.find_unit_index(q_m, "kg").unwrap();
        let kg = wb
            .convert_value(q_m, u_g, SiPrefix::Milli, u_kg, SiPrefix::None, 2500.0, false)
            .unwrap();
        assert!((kg - 0.0025).abs() < 1e-12);

        // U5: Mesh sieve lookup & interpolation
        assert!((wb.interpolate_mesh_to_mm(200.0).unwrap() - 0.074).abs() < 1e-6);
        let d_120 = wb.interpolate_mesh_to_mm(120.0).unwrap();
        assert!(d_120 > 0.105 && d_120 < 0.149);
        assert!((wb.interpolate_mm_to_mesh(d_120).unwrap() - 120.0).abs() < 1e-6);

        // U6: Moist air humidity at 20 °C, 100% RH (~2.31-2.34 vol-%, ~18.6-18.8 g/Nm3)
        let air20 = compute_air_humidity_point(20.0, 100.0, STD_ATM_BAR).unwrap();
        assert!((air20.vol_pct - 2.31).abs() < 0.08, "vol% = {}", air20.vol_pct);
        assert!((air20.g_per_nm3 - 18.6).abs() < 0.6, "g/Nm3 = {}", air20.g_per_nm3);

        // U7: Exponent-aware SI prefixes (cm2/cm3) & audited HSC coefficients
        let q_a = wb.find_quantity_index("Area").unwrap();
        let u_m2 = wb.find_unit_index(q_a, "m2").unwrap();
        let u_cm2 = wb.find_unit_index(q_a, "cm2").unwrap();
        let area_via_prefix = wb.convert_value(q_a, u_m2, SiPrefix::Centi, u_m2, SiPrefix::None, 1.0, false).unwrap();
        let area_via_unit = wb.convert_value(q_a, u_cm2, SiPrefix::None, u_m2, SiPrefix::None, 1.0, false).unwrap();
        assert!((area_via_prefix - 1e-4).abs() < 1e-12);
        assert!((area_via_unit - 1e-4).abs() < 1e-12);

        let q_v = wb.find_quantity_index("Volume").unwrap();
        let u_m3 = wb.find_unit_index(q_v, "m3").unwrap();
        let u_cm3 = wb.find_unit_index(q_v, "cm3").unwrap();
        let vol_via_prefix = wb.convert_value(q_v, u_m3, SiPrefix::Centi, u_m3, SiPrefix::None, 1.0, false).unwrap();
        let vol_via_unit = wb.convert_value(q_v, u_cm3, SiPrefix::None, u_m3, SiPrefix::None, 1.0, false).unwrap();
        assert!((vol_via_prefix - 1e-6).abs() < 1e-12);
        assert!((vol_via_unit - 1e-6).abs() < 1e-12);

        let u_mmhg0 = wb.find_unit_index(q_p, "mm Hg (0 C)").unwrap();
        let atm_mmhg = wb.convert_value(q_p, u_atm, SiPrefix::None, u_mmhg0, SiPrefix::None, 1.0, false).unwrap();
        assert!((atm_mmhg - 760.0).abs() < 0.05, "1 atm = {atm_mmhg} mmHg(0C)");

        let q_vk = wb.find_quantity_index("Viscosity, kinematic").unwrap();
        let u_m2s = wb.find_unit_index(q_vk, "m2 s-1").unwrap();
        let u_st = wb.find_unit_index(q_vk, "St").unwrap();
        assert!((wb.convert_value(q_vk, u_m2s, SiPrefix::None, u_st, SiPrefix::None, 1.0, false).unwrap() - 10000.0).abs() < 1e-6);

        let q_cp = wb.find_quantity_index("Specific heat capacity").unwrap();
        let u_jkgk = wb.find_unit_index(q_cp, "J kg-1 K-1").unwrap();
        let u_calgk = wb.find_unit_index(q_cp, "cal g-1 K-1").unwrap();
        let jkgk = wb.convert_value(q_cp, u_calgk, SiPrefix::None, u_jkgk, SiPrefix::None, 1.0, false).unwrap();
        assert!((jkgk - 4184.0).abs() < 0.1, "1 cal/(g K) = {jkgk} J/(kg K)");

        let q_av = wb.find_quantity_index("Angular velocity").unwrap();
        let u_rad = wb.find_unit_index(q_av, "rad s-1").unwrap();
        let u_deg = wb.find_unit_index(q_av, "degree s-1").unwrap();
        let rad = wb.convert_value(q_av, u_deg, SiPrefix::None, u_rad, SiPrefix::None, 180.0, false).unwrap();
        assert!((rad - std::f64::consts::PI).abs() < 1e-6);
    }
}
