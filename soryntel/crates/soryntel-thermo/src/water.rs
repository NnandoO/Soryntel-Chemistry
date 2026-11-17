//! Module 13 — Water / Steam Pressure-Temperature Calculator (`PTCalc`).
//!
//! Uses an IAPWS saturation-pressure correlation, but approximate caloric,
//! density and ice models. This is not a complete IF97 or IAPWS-95 implementation.
//! Numerical input bounds are not a statement of scientific accuracy.

use crate::balance::{h2o_saturation_pressure_bar, h2o_saturation_temp_k, H2O_PC_BAR, H2O_TC_K};
use crate::SpeciesRepository;
use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_JOULE, T_REF_K};

/// Molar mass of H2O in kg/kmol (== g/mol), matching HSC6 PTCalc display `18.01528`.
pub const H2O_MOL_WEIGHT: f64 = 18.01528;
/// Triple point temperature of water (0.01 °C = 273.16 K).
pub const H2O_TT_K: f64 = 273.16;
/// Triple point pressure of water (0.00611657 bar).
pub const H2O_PT_BAR: f64 = 0.00611657;
/// Standard reference pressure (1 bar for HSC reference H2O state).
pub const H2O_P_STD_BAR: f64 = 1.0;

/// HSC standard liquid H2O molar enthalpy at (298.15 K, 1 bar): -285.830 kJ/mol.
const H_LIQ_298_J_MOL: f64 = -285_830.0;
/// HSC standard liquid H2O molar entropy at (298.15 K, 1 bar): 69.95 J/(mol*K).
const S_LIQ_298_J_MOL_K: f64 = 69.95;
/// Standard Szargut chemical exergy of liquid H2O at (298.15 K, 1 bar): 52.118 kJ/kg = 938.920 J/mol.
const EX_CH_LIQ_298_J_MOL: f64 = 52.118 * H2O_MOL_WEIGHT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterPhaseMode {
    Solid,
    Liquid,
    Gas,
    SatGasLiquid,
    SatGasSolid,
    SatLiquidSolid,
}

impl WaterPhaseMode {
    pub const ALL: [Self; 6] = [
        Self::Gas,
        Self::Liquid,
        Self::Solid,
        Self::SatGasLiquid,
        Self::SatGasSolid,
        Self::SatLiquidSolid,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Sólido (s)",
            Self::Liquid => "Líquido (l)",
            Self::Gas => "Gas (g)",
            Self::SatGasLiquid => "Sat. (g,l)",
            Self::SatGasSolid => "Sat. (g,s)",
            Self::SatLiquidSolid => "Sat. (l,s)",
        }
    }

    pub fn is_saturated(self) -> bool {
        matches!(
            self,
            Self::SatGasLiquid | Self::SatGasSolid | Self::SatLiquidSolid
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterTempUnit {
    Celsius,
    Kelvin,
}

impl WaterTempUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Kelvin => "K",
        }
    }

    pub fn to_kelvin(self, val: f64) -> f64 {
        match self {
            Self::Celsius => val + 273.15,
            Self::Kelvin => val,
        }
    }

    pub fn from_kelvin(self, t_k: f64) -> f64 {
        match self {
            Self::Celsius => t_k - 273.15,
            Self::Kelvin => t_k,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterPressureUnit {
    Bar,
    MPa,
}

impl WaterPressureUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::MPa => "MPa",
        }
    }

    pub fn to_bar(self, val: f64) -> f64 {
        match self {
            Self::Bar => val,
            Self::MPa => val * 10.0,
        }
    }

    pub fn from_bar(self, p_bar: f64) -> f64 {
        match self {
            Self::Bar => p_bar,
            Self::MPa => p_bar * 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterEnergyUnit {
    KJ,
    Kcal,
    Wh,
    KWh,
    MWh,
}

impl WaterEnergyUnit {
    pub const ALL: [Self; 5] = [Self::KJ, Self::Kcal, Self::Wh, Self::KWh, Self::MWh];

    pub fn label(self) -> &'static str {
        match self {
            Self::KJ => "kJ",
            Self::Kcal => "kcal",
            Self::Wh => "Wh",
            Self::KWh => "kWh",
            Self::MWh => "MWh",
        }
    }

    pub fn entropy_label(self, t_unit: WaterTempUnit) -> String {
        let t = match t_unit {
            WaterTempUnit::Celsius | WaterTempUnit::Kelvin => "K",
        };
        format!("{}/{}", self.label(), t)
    }

    pub fn cp_label(self, amt_unit: WaterAmountUnit) -> String {
        let a = match amt_unit {
            WaterAmountUnit::Kg => "kg",
            WaterAmountUnit::Kmol => "kmol",
        };
        format!("{}/{}K", self.label(), a)
    }

    /// Factor multiplying kJ to yield `self`.
    pub fn from_kj_factor(self) -> f64 {
        match self {
            Self::KJ => 1.0,
            Self::Kcal => 1.0 / CAL_TO_JOULE,
            Self::Wh => 1.0 / 3.6,
            Self::KWh => 1.0 / 3_600.0,
            Self::MWh => 1.0 / 3_600_000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterAmountUnit {
    Kg,
    Kmol,
}

impl WaterAmountUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Kg => "kg",
            Self::Kmol => "kmol",
        }
    }

    /// Total moles (kmol * 1000 = mol) represented by `amount`.
    pub fn to_mol(self, amount: f64) -> f64 {
        match self {
            Self::Kg => (amount * 1000.0) / H2O_MOL_WEIGHT,
            Self::Kmol => amount * 1000.0,
        }
    }

    pub fn to_kg(self, amount: f64) -> f64 {
        match self {
            Self::Kg => amount,
            Self::Kmol => amount * H2O_MOL_WEIGHT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaterDensityUnit {
    KgPerM3,
    GPerCm3,
}

impl WaterDensityUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::KgPerM3 => "kg/m³",
            Self::GPerCm3 => "g/cm³",
        }
    }

    pub fn from_kg_m3(self, rho: f64) -> f64 {
        match self {
            Self::KgPerM3 => rho,
            Self::GPerCm3 => rho * 0.001,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterPointInput {
    pub temp: f64,
    pub pressure: f64,
    pub phase: WaterPhaseMode,
    /// Percentage [0..100] of the FIRST named phase in saturated pairs:
    /// - `Sat. (g,l)` -> gas percentage (steam dryness %)
    /// - `Sat. (g,s)` -> gas percentage
    /// - `Sat. (l,s)` -> liquid percentage
    pub mixture_pct: f64,
    /// When true and phase is pure, auto-detect phase from (T, P) region.
    pub auto_phase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterCalcRequest {
    pub amount: f64,
    pub amount_unit: WaterAmountUnit,
    pub temp_unit: WaterTempUnit,
    pub pressure_unit: WaterPressureUnit,
    pub energy_unit: WaterEnergyUnit,
    pub density_unit: WaterDensityUnit,
    pub point1: WaterPointInput,
    pub point2: WaterPointInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterPointOutput {
    pub temp_display: f64,
    pub pressure_display: f64,
    pub temp_k: f64,
    pub pressure_bar: f64,
    pub resolved_phase: WaterPhaseMode,
    pub temp_is_auto: bool,
    pub pressure_is_auto: bool,
    pub h: f64,
    pub s: f64,
    pub h_specific: f64,
    pub g: f64,
    pub cp: f64,
    pub density: f64,
    pub exergy_ph: f64,
    pub exergy_ch: f64,
    pub exergy_tot: f64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterCalcResult {
    pub point1: WaterPointOutput,
    pub point2: WaterPointOutput,
    /// `Delta` for input coordinates: `point1 - point2` (matches HSC6 Fig. 1 & Fig. 4).
    pub delta_temp: f64,
    pub delta_pressure: f64,
    /// `Delta` for thermodynamic properties: `point2 - point1` (matches HSC6 Fig. 1–4).
    pub delta_h: f64,
    pub delta_s: f64,
    pub delta_h_specific: f64,
    pub delta_g: f64,
    pub delta_cp: f64,
    pub delta_density: f64,
    pub delta_exergy_ph: f64,
    pub delta_exergy_ch: f64,
    pub delta_exergy_tot: f64,
    pub isentropic_summary: Option<IsentropicSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsentropicSummary {
    pub dh_real: f64,
    pub dh_isentropic: f64,
    pub efficiency: f64,
    pub mode_label: String,
    pub target_mixture_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterDiagramCurves {
    /// Sublimation curve `(p_bar, t_c)` from `-100 °C` to `Triple Point`.
    pub sat_gs: Vec<[f64; 2]>,
    /// Vaporization curve `(p_bar, t_c)` from `Triple Point` to `Critical Point`.
    pub sat_gl: Vec<[f64; 2]>,
    /// Melting curve `(p_bar, t_c)` for Ice Ih from `Triple Point` to `~2085 bar`.
    pub sat_ls: Vec<[f64; 2]>,
}

/// Inverse vaporization temperature T_sat,gl(P) in K for P in [H2O_PT_BAR, H2O_PC_BAR].
pub fn sat_temp_gl_k(p_bar: f64) -> f64 {
    h2o_saturation_temp_k(p_bar)
}

/// IAPWS 2011 Ice Ih sublimation pressure P_sat,gs(T) in bar for T <= 273.16 K.
pub fn sat_pressure_gs_bar(t_k: f64) -> f64 {
    let t = t_k.clamp(50.0, H2O_TT_K);
    let theta = t / H2O_TT_K;
    let a1 = -13.928169;
    let a2 = 34.707823;
    let ln_pr = (a1 * (1.0 - theta.powf(-1.5)) + a2 * (1.0 - theta.powf(-1.25))).clamp(-60.0, 2.0);
    // Note: standard IAPWS-2011 sublimation uses theta^(-1.5) & theta^(-1.25):
    // ln(p/pt) = a1*(1 - theta^-1.5) + a2*(1 - theta^-1.25) gives exact monotonic drop as T decreases.
    H2O_PT_BAR * ln_pr.exp()
}

/// Inverse sublimation temperature T_sat,gs(P) in K for P <= H2O_PT_BAR.
pub fn sat_temp_gs_k(p_bar: f64) -> f64 {
    if p_bar >= H2O_PT_BAR {
        return H2O_TT_K;
    }
    let p_target = p_bar.max(1e-12);
    let mut lo = 120.0_f64;
    let mut hi = H2O_TT_K;
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        if sat_pressure_gs_bar(mid) < p_target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// IAPWS 2011 Ice Ih melting pressure P_sat,ls(T) in bar for T in [251.165 K, 273.16 K].
pub fn sat_pressure_ls_bar(t_k: f64) -> f64 {
    if t_k >= H2O_TT_K {
        return H2O_PT_BAR;
    }
    let t = t_k.clamp(251.165, H2O_TT_K);
    let theta = t / H2O_TT_K;
    let ratio =
        1.0 - 0.626_000e6 * (1.0 - theta.powi(-3)) + 0.197_135e6 * (1.0 - theta.powf(21.2));
    (H2O_PT_BAR * ratio).max(H2O_PT_BAR)
}

/// Inverse Ice Ih melting temperature T_melt(P) in K for P in [H2O_PT_BAR, 2085 bar].
pub fn sat_temp_ls_k(p_bar: f64) -> f64 {
    if p_bar <= H2O_PT_BAR {
        return H2O_TT_K;
    }
    let p_clamped = p_bar.min(2085.0);
    // Note p_sat_ls increases monotonically as T decreases from 273.16 K down to 251.165 K.
    let mut lo = 251.165_f64;
    let mut hi = H2O_TT_K;
    for _ in 0..64 {
        let mid = 0.5 * (lo + hi);
        if sat_pressure_ls_bar(mid) > p_clamped {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Pure-phase classification matching HSC6 `25 Water.txt` Section 25.3.1:
/// For supercritical temperatures (`T >= Tc`), `P >= Pc -> Liquid`, `P < Pc -> Gas`.
pub fn classify_pure_phase(t_k: f64, p_bar: f64) -> WaterPhaseMode {
    if t_k >= H2O_TC_K {
        return if p_bar >= H2O_PC_BAR {
            WaterPhaseMode::Liquid
        } else {
            WaterPhaseMode::Gas
        };
    }
    if t_k <= H2O_TT_K {
        let p_sub = sat_pressure_gs_bar(t_k);
        if p_bar < p_sub {
            return WaterPhaseMode::Gas;
        }
        if p_bar > H2O_PT_BAR && t_k >= sat_temp_ls_k(p_bar) {
            return WaterPhaseMode::Liquid;
        }
        return WaterPhaseMode::Solid;
    }
    // T_t < T < T_c
    let p_vap = h2o_saturation_pressure_bar(t_k);
    if p_bar < p_vap {
        WaterPhaseMode::Gas
    } else if p_bar > H2O_PT_BAR && t_k < sat_temp_ls_k(p_bar) {
        WaterPhaseMode::Solid
    } else {
        WaterPhaseMode::Liquid
    }
}

/// Generate full `p-T` saturation curve data for plotting.
pub fn water_diagram_curves() -> WaterDiagramCurves {
    let mut sat_gs = Vec::with_capacity(60);
    for i in 0..=59 {
        let frac = i as f64 / 59.0;
        let t_c = -40.0 + frac * (0.01 - (-40.0));
        let t_k = t_c + 273.15;
        let p = sat_pressure_gs_bar(t_k);
        if p >= 0.0005 {
            sat_gs.push([p, t_c]);
        }
    }

    let mut sat_gl = Vec::with_capacity(140);
    for i in 0..=139 {
        let frac = i as f64 / 139.0;
        let t_k = H2O_TT_K + frac * (H2O_TC_K - H2O_TT_K);
        let p = h2o_saturation_pressure_bar(t_k);
        sat_gl.push([p, t_k - 273.15]);
    }

    let mut sat_ls = Vec::with_capacity(80);
    for i in 0..=79 {
        let frac = i as f64 / 79.0;
        // Log-spaced pressure from triple point to 2085 bar
        let p = H2O_PT_BAR * (2085.0 / H2O_PT_BAR).powf(frac);
        let t_k = sat_temp_ls_k(p);
        sat_ls.push([p, t_k - 273.15]);
    }

    WaterDiagramCurves {
        sat_gs,
        sat_gl,
        sat_ls,
    }
}

/// Molar properties of pure phase at `(t_k, p_bar)` in SI units per mole:
/// `(h_j_mol, s_j_mol_k, h_spec_j_mol, cp_j_mol_k, rho_kg_m3)`.
fn eval_pure_phase_molar(t_k: f64, p_bar: f64, phase: WaterPhaseMode) -> (f64, f64, f64, f64, f64) {
    match phase {
        WaterPhaseMode::Solid => eval_ice_molar(t_k, p_bar),
        WaterPhaseMode::Liquid => eval_liquid_molar(t_k, p_bar),
        WaterPhaseMode::Gas => eval_steam_molar(t_k, p_bar),
        _ => eval_liquid_molar(t_k, p_bar),
    }
}

/// Ideal-gas heat capacity Cp_ig(T) of H2O(g) in J/(mol*K) valid 70 K..2300 K
/// (NIST / Shomate + Einstein vibrational fit).
fn cp_ideal_steam_j_mol_k(t_k: f64) -> f64 {
    let t = t_k.max(50.0);
    // Rigid rotor + translation: 4*R = 33.258 J/(mol*K), plus 3 vibrational modes (theta = 2295, 5262, 5404 K)
    let einstein = |theta: f64| -> f64 {
        let x = (theta / t).min(80.0);
        let ex = x.exp();
        if (ex - 1.0).abs() < 1e-12 {
            R_JOULE
        } else {
            R_JOULE * x * x * ex / ((ex - 1.0) * (ex - 1.0))
        }
    };
    4.0 * R_JOULE + einstein(2295.0) + einstein(5262.0) + einstein(5404.0)
}

/// Integrate ideal-gas Cp(T) from T1 to T2 using 16-point Gauss-Legendre / Simpson quadrature.
fn integrate_cp_ig(t1: f64, t2: f64) -> (f64, f64) {
    let n = 32usize;
    let dt = (t2 - t1) / n as f64;
    let mut sum_h = 0.0;
    let mut sum_s = 0.0;
    for i in 0..n {
        let tm = t1 + (i as f64 + 0.5) * dt;
        let cp = cp_ideal_steam_j_mol_k(tm);
        sum_h += cp * dt;
        sum_s += (cp / tm) * dt;
    }
    (sum_h, sum_s)
}

/// Heat capacity of liquid water in J/(mol*K), capturing slight minimum around 35 °C and rise near Tc.
fn cp_liquid_j_mol_k(t_k: f64, p_bar: f64) -> f64 {
    let tc = (t_k - 273.15).clamp(-30.0, 372.0);
    // Base 1 bar polynomial fit (J/(g*K) * 18.01528): 4.217 at 0 °C, 4.184 at 25 °C, 4.216 at 100 °C
    let cp_g_k = if t_k < H2O_TC_K - 2.0 {
        let tr = (t_k / H2O_TC_K).clamp(0.35, 0.996);
        let base = 4.217 - 0.0022 * tc + 3.4e-5 * tc * tc;
        let crit_corr = if tr > 0.72 {
            0.42 * ((tr - 0.72) / (1.0 - tr + 0.015)).min(8.5)
        } else {
            0.0
        };
        let p_supp = 1.0 - 0.00018 * (p_bar - 1.0).clamp(0.0, 900.0);
        (base + crit_corr) * p_supp.max(0.75)
    } else {
        // Supercritical liquid-like region (p >= 220.64 bar, T >= Tc)
        let dt = (t_k - H2O_TC_K).abs();
        let peak = 6.2 * (-(dt / 75.0)).exp();
        2.35 + peak
    };
    (cp_g_k * H2O_MOL_WEIGHT).clamp(32.0, 320.0)
}

fn integrate_cp_liquid(t1: f64, t2: f64, p_bar: f64) -> (f64, f64) {
    let n = 36usize;
    let dt = (t2 - t1) / n as f64;
    let mut sum_h = 0.0;
    let mut sum_s = 0.0;
    for i in 0..n {
        let tm = (t1 + (i as f64 + 0.5) * dt).max(200.0);
        let cp = cp_liquid_j_mol_k(tm, p_bar);
        sum_h += cp * dt;
        sum_s += (cp / tm) * dt;
    }
    (sum_h, sum_s)
}

/// Heat capacity of Ice Ih in J/(mol*K): ~38.0 J/(mol*K) at 273 K (~2.11 kJ/kgK), Debye decline at cryogenic T.
fn cp_ice_j_mol_k(t_k: f64) -> f64 {
    let t = t_k.clamp(20.0, 285.0);
    // Linear/Debye blend matching NIST/Feistel Ice Ih: 2.097 kJ/(kg*K) at 0 °C, 1.38 at -100 °C, ~0.7 at -200 °C
    let cp_j_kg_k = (120.0 + 7.25 * t).clamp(50.0, 2120.0);
    (cp_j_kg_k * 0.001) * H2O_MOL_WEIGHT
}

fn integrate_cp_ice(t1: f64, t2: f64) -> (f64, f64) {
    let n = 32usize;
    let dt = (t2 - t1) / n as f64;
    let mut sum_h = 0.0;
    let mut sum_s = 0.0;
    for i in 0..n {
        let tm = (t1 + (i as f64 + 0.5) * dt).max(20.0);
        let cp = cp_ice_j_mol_k(tm);
        sum_h += cp * dt;
        sum_s += (cp / tm) * dt;
    }
    (sum_h, sum_s)
}

/// Water liquid density in kg/m3.
fn liquid_density_kg_m3(t_k: f64, p_bar: f64) -> f64 {
    if t_k >= H2O_TC_K {
        // Supercritical fluid density smoothly decreasing from ~650 to ~120
        let t_over = (t_k - H2O_TC_K) / 400.0;
        let p_factor = (p_bar / H2O_PC_BAR).clamp(0.8, 4.5).powf(0.45);
        return (322.0 * p_factor / (1.0 + 1.15 * t_over)).clamp(60.0, 1020.0);
    }
    let tr = (t_k / H2O_TC_K).clamp(0.35, 0.999);
    // Kell / IAPWS saturated liquid density curve
    let theta = (1.0 - tr).max(1e-4);
    let rho_sat = 322.0
        * (1.0
            + 1.99274064 * theta.powf(1.0 / 3.0)
            + 1.09965342 * theta.powf(2.0 / 3.0)
            - 0.510839303 * theta.powf(5.0 / 3.0)
            - 1.75493479 * theta.powf(16.0 / 3.0)
            - 45.5170352 * theta.powf(43.0 / 3.0)
            - 6.74694450e5 * theta.powf(110.0 / 3.0));
    let compressibility = 4.6e-5; // 1/bar
    (rho_sat * (1.0 + compressibility * (p_bar - 1.0).max(0.0))).clamp(322.0, 1060.0)
}

fn eval_ice_molar(t_k: f64, p_bar: f64) -> (f64, f64, f64, f64, f64) {
    // At 273.15 K, H_f(Ice) = H_l(273.15) - dH_fus (6008 J/mol), S_f(Ice) = S_l(273.15) - dS_fus (22.00 J/(mol*K))
    let (dh_l_273, ds_l_273) = integrate_cp_liquid(T_REF_K, 273.15, 1.0);
    let h_ice_273 = H_LIQ_298_J_MOL + dh_l_273 - 6008.0;
    let s_ice_273 = S_LIQ_298_J_MOL_K + ds_l_273 - (6008.0 / 273.15);

    let (dh_ice, ds_ice) = integrate_cp_ice(273.15, t_k);
    // Poynting compression term for solid ice: V_m = 19.65 cm3/mol, alpha ~ 1.5e-4 / K (Eq. 9)
    let v_m_ice = 19.65e-6;
    let alpha_ice = 1.5e-4;
    let dp_pa = (p_bar - 1.0) * 1.0e5;
    let dh_p = v_m_ice * (1.0 - alpha_ice * t_k.clamp(150.0, 275.0)) * dp_pa;
    let ds_p = -v_m_ice * alpha_ice * dp_pa;

    let h = h_ice_273 + dh_ice + dh_p;
    let s = (s_ice_273 + ds_ice + ds_p).max(0.5);

    // Specific enthalpy ignoring phase change relative to extrapolated solid at 298.15 K
    let (dh_spec, _) = integrate_cp_ice(T_REF_K, t_k);
    let cp = cp_ice_j_mol_k(t_k);
    let rho = 916.8 * (1.0 + 1.2e-5 * (p_bar - 1.0).max(0.0) + 1.5e-4 * (273.15 - t_k).max(0.0));
    (h, s, dh_spec + dh_p, cp, rho.clamp(890.0, 960.0))
}

fn eval_liquid_molar(t_k: f64, p_bar: f64) -> (f64, f64, f64, f64, f64) {
    let (dh_t, ds_t) = integrate_cp_liquid(T_REF_K, t_k, p_bar);
    // Compressed liquid Poynting + thermal expansivity correction (Eq. 9)
    let rho = liquid_density_kg_m3(t_k, p_bar);
    let v_m_m3 = (H2O_MOL_WEIGHT * 0.001) / rho; // m3/mol
    let alpha_l = if t_k < 277.15 {
        -0.5e-4
    } else if t_k < H2O_TC_K {
        2.5e-4 + 1.4e-3 * ((t_k - 298.15) / 350.0).max(0.0)
    } else {
        1.5e-3
    };
    let dp_pa = (p_bar - 1.0) * 1.0e5;
    let dh_p = v_m_m3 * (1.0 - alpha_l * t_k.min(H2O_TC_K)) * dp_pa;
    let ds_p = -v_m_m3 * alpha_l * dp_pa;

    let h = H_LIQ_298_J_MOL + dh_t + dh_p;
    let s = (S_LIQ_298_J_MOL_K + ds_t + ds_p).max(1.0);
    let h_spec = dh_t + dh_p;
    let cp = cp_liquid_j_mol_k(t_k, p_bar);
    (h, s, h_spec, cp, rho)
}

/// Real-gas steam molar enthalpy, entropy, specific enthalpy, Cp, and density.
/// Calibrated to HSC `H2O(g)` standard values at `(298.15 K, 1 bar)`:
/// `Hf° = -241,818 J/mol`, `S° = 188.83 J/(mol*K)`, plus pressure non-ideality
/// near saturation and the critical region (`Tc = 647.096 K, Pc = 220.64 bar`).
fn eval_steam_molar(t_k: f64, p_bar: f64) -> (f64, f64, f64, f64, f64) {
    let t = t_k.max(70.0);
    let p = p_bar.clamp(1e-4, 1000.0);
    let (dh_ig, ds_ig) = integrate_cp_ig(T_REF_K, t);
    let h_ig = -241_818.0 + dh_ig;
    let s_ig_1bar = 188.83 + ds_ig;

    // Residual departure via Pitzer/IAPWS virial compressibility calibrated near Tc, Pc:
    // At low pressure or high T, omega_res -> 0 (reverts cleanly to ideal gas H2O(g)).
    let tr = t / H2O_TC_K;
    let pr = p / H2O_PC_BAR;
    // Polar water second virial departure B0(Tr) + acentric B1(Tr) (omega = 0.344)
    let b0 = 0.083 - 0.422 / tr.powf(1.6);
    let b0_z = 0.083 - 0.175 / tr.powf(1.6);
    let b1 = 0.139 - 0.172 / tr.powf(4.2);
    let db0_dtr = 0.422 * 1.6 / tr.powf(2.6);
    let db1_dtr = 0.172 * 4.2 / tr.powf(5.2);
    let omega = 0.344;

    let z_raw = 1.0 + pr / tr * (b0_z + omega * b1);
    let z = z_raw.clamp(0.24, 1.05);

    // Residual enthalpy H_res = R * Tc * Pr * (B0 - Tr*dB0/dTr + omega*(B1 - Tr*dB1/dTr))
    let h_res = (R_JOULE * H2O_TC_K * pr * (b0 - tr * db0_dtr + omega * (b1 - tr * db1_dtr)))
        .clamp(-18_500.0, 1_500.0);
    let s_res = (-R_JOULE * pr * (db0_dtr + omega * db1_dtr)).clamp(-26.0, 2.0);
    // Departure for Cp_res = -R * Pr * Tr * d2B/dTr2 (matches Fig. 5 peak near boiling / Tc)
    let d2b0 = -0.422 * 1.6 * 2.6 / tr.powf(3.6);
    let d2b1 = -0.172 * 4.2 * 5.2 / tr.powf(6.2);
    let cp_res = (-1.75 * R_JOULE * pr * tr * (d2b0 + omega * d2b1)).clamp(0.0, 120.0);

    let ds_pressure = -R_JOULE * (p / H2O_P_STD_BAR).ln();
    let h = h_ig + h_res;
    let s = (s_ig_1bar + ds_pressure + s_res).max(10.0);
    let h_spec = dh_ig + h_res;
    let cp = cp_ideal_steam_j_mol_k(t) + cp_res;

    // Density rho = P * M_w / (Z * R * T), in kg/m3 (`P` in Pa = p_bar * 1e5, M_w in kg/mol = 0.01801528)
    let rho = ((p * 1.0e5) * (H2O_MOL_WEIGHT * 0.001) / (z * R_JOULE * t)).max(1e-6);
    (h, s, h_spec, cp, rho)
}

/// Evaluate a single water/steam point, enforcing saturation curves when requested
/// and converting all outputs to display units.
pub fn evaluate_water_point(
    input: &WaterPointInput,
    amount: f64,
    amount_unit: WaterAmountUnit,
    temp_unit: WaterTempUnit,
    pressure_unit: WaterPressureUnit,
    energy_unit: WaterEnergyUnit,
    density_unit: WaterDensityUnit,
    repo: Option<&dyn SpeciesRepository>,
) -> WaterPointOutput {
    let _ = repo; // Pressure-corrected analytical EOS covers continuous T/P; repo hook reserved for custom overrides
    let mut warning = None;
    let raw_t_k = temp_unit.to_kelvin(input.temp).clamp(3.0, 2273.15);
    let raw_p_bar = pressure_unit.to_bar(input.pressure).clamp(0.001, 1000.0);

    let (eff_t_k, eff_p_bar, resolved_phase, temp_auto, press_auto) = match input.phase {
        WaterPhaseMode::SatGasLiquid => {
            if raw_t_k > H2O_TC_K {
                let ph = classify_pure_phase(raw_t_k, raw_p_bar);
                warning = Some("Temperatura superior al punto crítico (373,946 °C): se seleccionó una fase pura supercrítica.".into());
                (raw_t_k, raw_p_bar, ph, false, false)
            } else if raw_t_k < H2O_TT_K {
                let t = H2O_TT_K;
                (t, H2O_PT_BAR, WaterPhaseMode::SatGasLiquid, false, true)
            } else {
                (
                    raw_t_k,
                    h2o_saturation_pressure_bar(raw_t_k),
                    WaterPhaseMode::SatGasLiquid,
                    false,
                    true,
                )
            }
        }
        WaterPhaseMode::SatGasSolid => {
            if raw_t_k > H2O_TT_K {
                let ph = classify_pure_phase(raw_t_k, raw_p_bar);
                warning = Some("Temperatura superior al punto triple (0,01 °C): se seleccionó una fase pura.".into());
                (raw_t_k, raw_p_bar, ph, false, false)
            } else {
                (
                    raw_t_k,
                    sat_pressure_gs_bar(raw_t_k).clamp(0.0005, H2O_PT_BAR),
                    WaterPhaseMode::SatGasSolid,
                    false,
                    true,
                )
            }
        }
        WaterPhaseMode::SatLiquidSolid => {
            if raw_p_bar < H2O_PT_BAR {
                let ph = classify_pure_phase(raw_t_k, raw_p_bar);
                warning = Some("Presión inferior al punto triple (0,0061 bar): se seleccionó una fase pura.".into());
                (raw_t_k, raw_p_bar, ph, false, false)
            } else {
                (
                    sat_temp_ls_k(raw_p_bar),
                    raw_p_bar,
                    WaterPhaseMode::SatLiquidSolid,
                    true,
                    false,
                )
            }
        }
        explicit_pure => {
            let ph = if input.auto_phase {
                classify_pure_phase(raw_t_k, raw_p_bar)
            } else {
                explicit_pure
            };
            (raw_t_k, raw_p_bar, ph, false, false)
        }
    };

    if eff_t_k > 1073.15 && eff_p_bar > 100.0 {
        warning.get_or_insert_with(|| {
            "Región extrapolada de alta temperatura y presión (>800 °C, >100 bar).".to_string()
        });
    } else if (eff_t_k - H2O_TC_K).abs() < 2.0 && (eff_p_bar - H2O_PC_BAR).abs() < 4.0 {
        warning.get_or_insert_with(|| {
            "Cerca del punto crítico (373,946 °C, 220,64 bar): Cp y densidad son muy sensibles a las condiciones.".to_string()
        });
    }

    let x = (input.mixture_pct / 100.0).clamp(0.0, 1.0);
    let (h_m, s_m, h_spec_m, cp_m, rho_kg_m3) = match resolved_phase {
        WaterPhaseMode::SatGasLiquid => {
            let g = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Gas);
            let l = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Liquid);
            let v_mix = x / g.4.max(1e-9) + (1.0 - x) / l.4.max(1e-9);
            // Match HSC Fig. 3: isobaric gas/liquid coexistence reports Cp = 0.000
            (
                x * g.0 + (1.0 - x) * l.0,
                x * g.1 + (1.0 - x) * l.1,
                x * g.2 + (1.0 - x) * l.2,
                0.0,
                1.0 / v_mix.max(1e-12),
            )
        }
        WaterPhaseMode::SatGasSolid => {
            let g = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Gas);
            let s = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Solid);
            let v_mix = x / g.4.max(1e-9) + (1.0 - x) / s.4.max(1e-9);
            (
                x * g.0 + (1.0 - x) * s.0,
                x * g.1 + (1.0 - x) * s.1,
                x * g.2 + (1.0 - x) * s.2,
                0.0,
                1.0 / v_mix.max(1e-12),
            )
        }
        WaterPhaseMode::SatLiquidSolid => {
            let l = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Liquid);
            let s = eval_pure_phase_molar(eff_t_k, eff_p_bar, WaterPhaseMode::Solid);
            let v_mix = x / l.4.max(1e-9) + (1.0 - x) / s.4.max(1e-9);
            (
                x * l.0 + (1.0 - x) * s.0,
                x * l.1 + (1.0 - x) * s.1,
                x * l.2 + (1.0 - x) * s.2,
                x * l.3 + (1.0 - x) * s.3,
                1.0 / v_mix.max(1e-12),
            )
        }
        pure_ph => eval_pure_phase_molar(eff_t_k, eff_p_bar, pure_ph),
    };

    let n_mol = amount_unit.to_mol(amount.max(0.0));
    let k_to_e = energy_unit.from_kj_factor() / 1000.0; // J -> display energy unit

    let h = h_m * n_mol * k_to_e;
    let s = s_m * n_mol * k_to_e;
    let h_specific = h_spec_m * n_mol * k_to_e;
    let g_val = (h_m - eff_t_k * s_m) * n_mol * k_to_e;

    // Cp per unit mass (kg) or kmol as shown in HSC (`kJ/kgK` when Amount=kg, `kJ/kmolK` when kmol)
    let cp = match amount_unit {
        WaterAmountUnit::Kg => (cp_m / (H2O_MOL_WEIGHT * 0.001)) * k_to_e,
        WaterAmountUnit::Kmol => (cp_m * 1000.0) * k_to_e,
    };

    // Physical exergy relative to dead state (H2O liquid at 298.15 K, 1 bar)
    let ex_ph_m = (h_m - H_LIQ_298_J_MOL) - T_REF_K * (s_m - S_LIQ_298_J_MOL_K);
    let ex_ch_m = EX_CH_LIQ_298_J_MOL;
    let exergy_ph = ex_ph_m * n_mol * k_to_e;
    let exergy_ch = ex_ch_m * n_mol * k_to_e;
    let exergy_tot = exergy_ph + exergy_ch;

    WaterPointOutput {
        temp_display: temp_unit.from_kelvin(eff_t_k),
        pressure_display: pressure_unit.from_bar(eff_p_bar),
        temp_k: eff_t_k,
        pressure_bar: eff_p_bar,
        resolved_phase,
        temp_is_auto: temp_auto,
        pressure_is_auto: press_auto,
        h,
        s,
        h_specific,
        g: g_val,
        cp,
        density: density_unit.from_kg_m3(rho_kg_m3),
        exergy_ph,
        exergy_ch,
        exergy_tot,
        warning,
    }
}

/// Validate user input before the legacy numerical evaluator applies internal bounds.
pub fn validate_water_request(req: &WaterCalcRequest) -> Result<(), soryntel_core::SoryntelError> {
    use soryntel_core::SoryntelError;
    if !req.amount.is_finite() || req.amount <= 0.0 {
        return Err(SoryntelError::ParseError("La cantidad de agua debe ser finita y mayor que cero".into()));
    }
    for point in [&req.point1, &req.point2] {
        let t = req.temp_unit.to_kelvin(point.temp);
        let p = req.pressure_unit.to_bar(point.pressure);
        if !t.is_finite() || !(3.0..=2273.15).contains(&t) {
            return Err(SoryntelError::ParseError("El cálculo de agua admite temperaturas entre 3 y 2273,15 K".into()));
        }
        if !p.is_finite() || p <= 0.0 || p > 1000.0
            || (p < 0.001 && !matches!(point.phase, WaterPhaseMode::SatGasLiquid | WaterPhaseMode::SatGasSolid)) {
            return Err(SoryntelError::ParseError("El cálculo de agua admite presiones entre 0,001 y 1000 bar".into()));
        }
        if !point.mixture_pct.is_finite() || !(0.0..=100.0).contains(&point.mixture_pct) {
            return Err(SoryntelError::ParseError("El porcentaje de mezcla debe estar entre 0 y 100".into()));
        }
    }
    Ok(())
}

/// Evaluate complete two-point Water / Steam calculation and Delta column.
pub fn calculate_water_points(
    req: &WaterCalcRequest,
    repo: Option<&dyn SpeciesRepository>,
) -> WaterCalcResult {
    let p1 = evaluate_water_point(
        &req.point1,
        req.amount,
        req.amount_unit,
        req.temp_unit,
        req.pressure_unit,
        req.energy_unit,
        req.density_unit,
        repo,
    );
    let p2 = evaluate_water_point(
        &req.point2,
        req.amount,
        req.amount_unit,
        req.temp_unit,
        req.pressure_unit,
        req.energy_unit,
        req.density_unit,
        repo,
    );

    let dh = p2.h - p1.h;
    let ds = p2.s - p1.s;

    // Optional isentropic comparison if pressure differs noticeably
    let isentropic_summary = compute_isentropic_comparison(req, &p1, &p2, dh, repo);

    WaterCalcResult {
        delta_temp: p1.temp_display - p2.temp_display,
        delta_pressure: p1.pressure_display - p2.pressure_display,
        delta_h: dh,
        delta_s: ds,
        delta_h_specific: p2.h_specific - p1.h_specific,
        delta_g: p2.g - p1.g,
        delta_cp: p2.cp - p1.cp,
        delta_density: p2.density - p1.density,
        delta_exergy_ph: p2.exergy_ph - p1.exergy_ph,
        delta_exergy_ch: p2.exergy_ch - p1.exergy_ch,
        delta_exergy_tot: p2.exergy_tot - p1.exergy_tot,
        point1: p1,
        point2: p2,
        isentropic_summary,
    }
}

/// Solve an ideal isentropic reference (`S_2s == S_1`) at Point 2's pressure when Point 1 and Point 2
/// represent an expansion (`P1 > P2`) or compression (`P2 > P1`).
fn compute_isentropic_comparison(
    req: &WaterCalcRequest,
    p1: &WaterPointOutput,
    p2: &WaterPointOutput,
    dh_real: f64,
    repo: Option<&dyn SpeciesRepository>,
) -> Option<IsentropicSummary> {
    if (p1.pressure_bar - p2.pressure_bar).abs() < 0.05 || dh_real.abs() < 1e-9 {
        return None;
    }
    // Check if Point 2 is in saturated liquid-gas region and an isentropic mixture% exists
    if p2.resolved_phase == WaterPhaseMode::SatGasLiquid {
        let mut lo = 0.0_f64;
        let mut hi = 100.0_f64;
        for _ in 0..32 {
            let mid = 0.5 * (lo + hi);
            let mut probe = req.point2.clone();
            probe.mixture_pct = mid;
            let out = evaluate_water_point(
                &probe,
                req.amount,
                req.amount_unit,
                req.temp_unit,
                req.pressure_unit,
                req.energy_unit,
                req.density_unit,
                repo,
            );
            if out.s < p1.s {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let target_mix = 0.5 * (lo + hi);
        let mut ideal_in = req.point2.clone();
        ideal_in.mixture_pct = target_mix;
        let p2s = evaluate_water_point(
            &ideal_in,
            req.amount,
            req.amount_unit,
            req.temp_unit,
            req.pressure_unit,
            req.energy_unit,
            req.density_unit,
            repo,
        );
        if (p2s.s - p1.s).abs() < (0.02 * p1.s.abs().max(1.0)) {
            let dh_s = p2s.h - p1.h;
            if dh_s.abs() > 1e-9 && dh_real < 0.0 && dh_s < 0.0 {
                return Some(IsentropicSummary {
                    dh_real,
                    dh_isentropic: dh_s,
                    efficiency: (dh_real / dh_s).clamp(0.0, 1.5),
                    mode_label: "Turbine ηs,t = Δh / Δhs".to_string(),
                    target_mixture_pct: Some(target_mix),
                });
            }
        }
    }
    None
}

/// Build default HSC6 request matching the reference Water Calculator view (`1.0 kg`, `300 °C / 25 bar Gas` -> `25 °C / 1 bar Liquid`).
pub fn default_water_request() -> WaterCalcRequest {
    WaterCalcRequest {
        amount: 1.0,
        amount_unit: WaterAmountUnit::Kg,
        temp_unit: WaterTempUnit::Celsius,
        pressure_unit: WaterPressureUnit::Bar,
        energy_unit: WaterEnergyUnit::KJ,
        density_unit: WaterDensityUnit::KgPerM3,
        point1: WaterPointInput {
            temp: 300.0,
            pressure: 25.0,
            phase: WaterPhaseMode::Gas,
            mixture_pct: 50.0,
            auto_phase: false,
        },
        point2: WaterPointInput {
            temp: 25.0,
            pressure: 1.0,
            phase: WaterPhaseMode::Liquid,
            mixture_pct: 50.0,
            auto_phase: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_validation_w1_to_w6() {
        // W1: Triple point P_sat(0.01 °C) and Delta = 0 when P1 == P2
        assert!((h2o_saturation_pressure_bar(H2O_TT_K) - H2O_PT_BAR).abs() < 1e-6);
        assert!((sat_pressure_gs_bar(H2O_TT_K) - H2O_PT_BAR).abs() < 1e-6);
        assert!((sat_pressure_ls_bar(H2O_TT_K) - H2O_PT_BAR).abs() < 1e-6);

        let tp_pt = WaterPointInput {
            temp: 0.01,
            pressure: H2O_PT_BAR,
            phase: WaterPhaseMode::Liquid,
            mixture_pct: 50.0,
            auto_phase: false,
        };
        let mut req = default_water_request();
        req.point1 = tp_pt.clone();
        req.point2 = tp_pt;
        let res_tp = calculate_water_points(&req, None);
        assert!(res_tp.delta_h.abs() < 1e-9);
        assert!(res_tp.delta_s.abs() < 1e-9);
        assert!((res_tp.point2.density - 999.8).abs() < 2.0);
        assert!((res_tp.point2.exergy_ch - 52.118).abs() < 0.01);

        // W2: Critical point & supercritical phase classification
        assert!((h2o_saturation_pressure_bar(H2O_TC_K) - H2O_PC_BAR).abs() < 0.05);
        assert_eq!(classify_pure_phase(450.0 + 273.15, 250.0), WaterPhaseMode::Liquid);
        assert_eq!(classify_pure_phase(450.0 + 273.15, 100.0), WaterPhaseMode::Gas);

        // W3: High pressure ice melting at 500 bar (HSC Example 25.2.1 / Fig. 2)
        let ice_req = WaterCalcRequest {
            amount: 1.0,
            amount_unit: WaterAmountUnit::Kg,
            temp_unit: WaterTempUnit::Celsius,
            pressure_unit: WaterPressureUnit::Bar,
            energy_unit: WaterEnergyUnit::KJ,
            density_unit: WaterDensityUnit::KgPerM3,
            point1: WaterPointInput {
                temp: -4.0,
                pressure: 500.0,
                phase: WaterPhaseMode::SatLiquidSolid,
                mixture_pct: 0.0,
                auto_phase: false,
            },
            point2: WaterPointInput {
                temp: -4.0,
                pressure: 500.0,
                phase: WaterPhaseMode::SatLiquidSolid,
                mixture_pct: 90.0,
                auto_phase: false,
            },
        };
        let ice_res = calculate_water_points(&ice_req, None);
        assert!(
            ice_res.point1.temp_display < -3.0 && ice_res.point1.temp_display > -5.5,
            "Unexpected Tm(500bar) = {}",
            ice_res.point1.temp_display
        );
        assert!(
            (ice_res.delta_h - 300.0).abs() < 30.0,
            "Unexpected ice melting ΔH = {}",
            ice_res.delta_h
        );
        assert!(ice_res.delta_s > 0.9 && ice_res.delta_s < 1.25);

        // W4: Evaporation in boiler at 285 °C (HSC Example 25.2.2 / Fig. 3)
        let evap_req = WaterCalcRequest {
            amount: 500_000.0,
            amount_unit: WaterAmountUnit::Kg,
            temp_unit: WaterTempUnit::Celsius,
            pressure_unit: WaterPressureUnit::Bar,
            energy_unit: WaterEnergyUnit::MWh,
            density_unit: WaterDensityUnit::KgPerM3,
            point1: WaterPointInput {
                temp: 285.0,
                pressure: 1.0,
                phase: WaterPhaseMode::SatGasLiquid,
                mixture_pct: 0.0,
                auto_phase: false,
            },
            point2: WaterPointInput {
                temp: 285.0,
                pressure: 1.0,
                phase: WaterPhaseMode::SatGasLiquid,
                mixture_pct: 10.0,
                auto_phase: false,
            },
        };
        let evap_res = calculate_water_points(&evap_req, None);
        assert!((evap_res.point1.pressure_bar - 69.18).abs() < 0.4);
        assert!(evap_res.delta_h > 19.0 && evap_res.delta_h < 23.0);
        assert_eq!(evap_res.point1.cp, 0.0);

        // W5: Steam turbine 480 °C, 40 bar -> 30 °C Sat.(g,l) 91% (HSC Fig. 4)
        let turb_req = WaterCalcRequest {
            amount: 10.0,
            amount_unit: WaterAmountUnit::Kg,
            temp_unit: WaterTempUnit::Celsius,
            pressure_unit: WaterPressureUnit::Bar,
            energy_unit: WaterEnergyUnit::KJ,
            density_unit: WaterDensityUnit::KgPerM3,
            point1: WaterPointInput {
                temp: 480.0,
                pressure: 40.0,
                phase: WaterPhaseMode::Gas,
                mixture_pct: 50.0,
                auto_phase: false,
            },
            point2: WaterPointInput {
                temp: 30.0,
                pressure: 1.0,
                phase: WaterPhaseMode::SatGasLiquid,
                mixture_pct: 91.0,
                auto_phase: false,
            },
        };
        let turb_res = calculate_water_points(&turb_req, None);
        assert!(
            turb_res.delta_h < -9_500.0 && turb_res.delta_h > -11_800.0,
            "Unexpected turbine ΔH = {}",
            turb_res.delta_h
        );
        let iso = turb_res.isentropic_summary.unwrap();
        assert!(iso.efficiency > 0.72 && iso.efficiency < 0.92);

        // W6: Saturated steam at 100 °C vs Triple Point Liquid (Steam Tables ~2676 kJ/kg)
        let steam100 = WaterCalcRequest {
            amount: 1.0,
            amount_unit: WaterAmountUnit::Kg,
            temp_unit: WaterTempUnit::Celsius,
            pressure_unit: WaterPressureUnit::Bar,
            energy_unit: WaterEnergyUnit::KJ,
            density_unit: WaterDensityUnit::KgPerM3,
            point1: WaterPointInput {
                temp: 0.01,
                pressure: H2O_PT_BAR,
                phase: WaterPhaseMode::Liquid,
                mixture_pct: 50.0,
                auto_phase: false,
            },
            point2: WaterPointInput {
                temp: 100.0,
                pressure: 1.0,
                phase: WaterPhaseMode::SatGasLiquid,
                mixture_pct: 100.0,
                auto_phase: false,
            },
        };
        let s100_res = calculate_water_points(&steam100, None);
        assert!((s100_res.point2.pressure_bar - 1.0135).abs() < 0.01);
        assert!(
            (s100_res.delta_h - 2675.6).abs() < 40.0,
            "Unexpected 100 °C saturated steam h = {}",
            s100_res.delta_h
        );
    }
}
