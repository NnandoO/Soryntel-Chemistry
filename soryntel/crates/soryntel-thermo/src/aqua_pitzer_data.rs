//! Open-literature Pitzer & Harvie–Møller–Weare ion interaction parameter database
//! structured to match HSC 10 Aqua `Main Pitzer Database` (`Binary`, `Theta`, `Psi`, `Lambda`)
//! with temperature polynomial Eq. (22):
//! `F(T) = a + b/T + c*ln(T) + d*T + e*T^2 + f*T^-2` (`T` in Kelvin).

use serde::{Deserialize, Serialize};

/// 6-term temperature dependence curve Eq. (22) for a single Pitzer scalar parameter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct PitzerTempCurve {
    pub defined: bool,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl PitzerTempCurve {
    pub const EMPTY: Self = Self {
        defined: false,
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };

    pub const fn constant(a: f64) -> Self {
        Self {
            defined: true,
            a,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Creates a linear temperature curve parameterized by value at 298.15 K and slope dP/dT.
    pub const fn linear_298(val_298: f64, slope_per_k: f64) -> Self {
        Self {
            defined: true,
            a: val_298 - slope_per_k * 298.15,
            b: 0.0,
            c: 0.0,
            d: slope_per_k,
            e: 0.0,
            f: 0.0,
        }
    }

    pub const fn full(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self {
            defined: true,
            a,
            b,
            c,
            d,
            e,
            f,
        }
    }

    pub fn has_t_dep(&self) -> bool {
        self.defined
            && (self.b.abs() > 1e-18
                || self.c.abs() > 1e-18
                || self.d.abs() > 1e-18
                || self.e.abs() > 1e-18
                || self.f.abs() > 1e-18)
    }

    pub fn status_flag(&self) -> &'static str {
        if !self.defined {
            "*"
        } else if self.has_t_dep() {
            "T"
        } else {
            "C"
        }
    }

    /// Evaluates F(T) where `t_k` is in Kelvin.
    pub fn eval(&self, t_k: f64) -> f64 {
        if !self.defined {
            return 0.0;
        }
        let t = t_k.max(1.0);
        self.a
            + self.b / t
            + self.c * t.ln()
            + self.d * t
            + self.e * t * t
            + self.f / (t * t)
    }

    /// First temperature derivative dF/dT.
    pub fn d_dt(&self, t_k: f64) -> f64 {
        if !self.has_t_dep() {
            return 0.0;
        }
        let t = t_k.max(1.0);
        -self.b / (t * t) + self.c / t + self.d + 2.0 * self.e * t - 2.0 * self.f / (t * t * t)
    }

    /// Second temperature derivative d2F/dT2.
    pub fn d2_dt2(&self, t_k: f64) -> f64 {
        if !self.has_t_dep() {
            return 0.0;
        }
        let t = t_k.max(1.0);
        2.0 * self.b / (t * t * t) - self.c / (t * t) + 2.0 * self.e
            + 6.0 * self.f / (t * t * t * t)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidityRange {
    pub min_t_c: Option<f64>,
    pub max_t_c: Option<f64>,
    pub min_m: Option<f64>,
    pub max_m: Option<f64>,
    pub min_i: Option<f64>,
    pub max_i: Option<f64>,
}

impl ValidityRange {
    pub const fn any() -> Self {
        Self {
            min_t_c: None,
            max_t_c: None,
            min_m: None,
            max_m: None,
            min_i: None,
            max_i: None,
        }
    }

    pub const fn t_25() -> Self {
        Self {
            min_t_c: Some(25.0),
            max_t_c: Some(25.0),
            min_m: None,
            max_m: None,
            min_i: None,
            max_i: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryPitzerEntry {
    pub cation: String,
    pub anion: String,
    pub beta0: PitzerTempCurve,
    pub beta1: PitzerTempCurve,
    pub beta2: PitzerTempCurve,
    pub c_phi: PitzerTempCurve,
    pub range: ValidityRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThetaPitzerEntry {
    pub ion_i: String,
    pub ion_j: String,
    pub theta: PitzerTempCurve,
    pub range: ValidityRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsiPitzerEntry {
    pub ion1: String,
    pub ion2: String,
    pub ion3: String,
    pub psi: PitzerTempCurve,
    pub range: ValidityRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaPitzerEntry {
    pub neutral: String,
    pub ion: String,
    pub lambda: PitzerTempCurve,
    pub range: ValidityRange,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PitzerDatabase {
    pub binaries: Vec<BinaryPitzerEntry>,
    pub thetas: Vec<ThetaPitzerEntry>,
    pub psis: Vec<PsiPitzerEntry>,
    pub lambdas: Vec<LambdaPitzerEntry>,
}

fn bin_e(
    cat: &str,
    an: &str,
    b0: PitzerTempCurve,
    b1: PitzerTempCurve,
    b2: PitzerTempCurve,
    cp: PitzerTempCurve,
    range: ValidityRange,
) -> BinaryPitzerEntry {
    BinaryPitzerEntry {
        cation: cat.into(),
        anion: an.into(),
        beta0: b0,
        beta1: b1,
        beta2: b2,
        c_phi: cp,
        range,
    }
}

fn th_e(i: &str, j: &str, th: PitzerTempCurve, range: ValidityRange) -> ThetaPitzerEntry {
    ThetaPitzerEntry {
        ion_i: i.into(),
        ion_j: j.into(),
        theta: th,
        range,
    }
}

fn psi_e(a: &str, b: &str, c: &str, psi: PitzerTempCurve, range: ValidityRange) -> PsiPitzerEntry {
    PsiPitzerEntry {
        ion1: a.into(),
        ion2: b.into(),
        ion3: c.into(),
        psi,
        range,
    }
}

fn lam_e(n: &str, i: &str, lam: PitzerTempCurve, range: ValidityRange) -> LambdaPitzerEntry {
    LambdaPitzerEntry {
        neutral: n.into(),
        ion: i.into(),
        lambda: lam,
        range,
    }
}

/// Returns the built-in open-literature Pitzer/Harvie Main Database.
pub fn builtin_main_pitzer_db() -> PitzerDatabase {
    use PitzerTempCurve as C;
    let binaries = vec![
        bin_e("Ag(+a)", "NO3(-a)", C::constant(-0.0856), C::constant(0.0025), C::EMPTY, C::constant(0.0059), ValidityRange::any()),
        bin_e(
            "Al(+3a)",
            "Cl(-a)",
            C::linear_298(0.6993, 4.0e-4),
            C::linear_298(5.8447, 5.0e-3),
            C::EMPTY,
            C::linear_298(0.00273, -1.0e-4),
            ValidityRange { min_t_c: Some(25.0), max_t_c: Some(290.0), min_m: None, max_m: Some(5.0), min_i: None, max_i: None },
        ),
        bin_e("Al(+3a)", "NO3(-a)", C::constant(0.562), C::constant(5.10), C::EMPTY, C::constant(-0.025), ValidityRange { min_t_c: Some(25.0), max_t_c: Some(25.0), min_m: Some(1.0), max_m: Some(3.0), min_i: None, max_i: None }),
        bin_e("Al(+3a)", "SO4(-2a)", C::constant(0.870), C::constant(18.2), C::constant(-30.0), C::constant(-0.030), ValidityRange { min_t_c: Some(25.0), max_t_c: Some(25.0), min_m: None, max_m: None, min_i: None, max_i: Some(4.5) }),
        bin_e("Ba(+2a)", "Br(-a)", C::linear_298(0.3146, 3.0e-4), C::linear_298(1.5698, 2.0e-3), C::EMPTY, C::constant(-0.01596), ValidityRange { min_t_c: Some(5.0), max_t_c: Some(45.0), min_m: None, max_m: None, min_i: None, max_i: None }),
        bin_e("Ba(+2a)", "CH3COO(-a)", C::constant(0.118), C::constant(1.55), C::EMPTY, C::constant(0.0), ValidityRange::any()),
        bin_e("Ba(+2a)", "Cl(-a)", C::linear_298(0.2628, 3.5e-4), C::linear_298(1.4963, 2.8e-3), C::EMPTY, C::linear_298(-0.0194, 1.0e-4), ValidityRange { min_t_c: Some(25.0), max_t_c: Some(200.0), min_m: None, max_m: None, min_i: None, max_i: None }),
        bin_e("Ca(+2a)", "Br(-a)", C::constant(0.3816), C::constant(1.613), C::EMPTY, C::constant(-0.00257), ValidityRange::t_25()),
        bin_e("Ca(+2a)", "Cl(-a)", C::linear_298(0.3159, -1.72e-4), C::linear_298(1.614, 3.9e-3), C::EMPTY, C::linear_298(-0.00034, -8.5e-5), ValidityRange { min_t_c: Some(0.0), max_t_c: Some(250.0), min_m: None, max_m: Some(6.0), min_i: None, max_i: None }),
        bin_e("Ca(+2a)", "HCO3(-a)", C::constant(0.40), C::constant(2.977), C::EMPTY, C::EMPTY, ValidityRange::t_25()),
        bin_e("Ca(+2a)", "SO4(-2a)", C::constant(0.2000), C::constant(3.1973), C::constant(-54.24), C::constant(0.0), ValidityRange::t_25()),
        bin_e("Co(+2a)", "SO4(-2a)", C::constant(0.2000), C::constant(2.700), C::constant(-30.70), C::constant(0.0), ValidityRange::t_25()),
        bin_e("Cu(+2a)", "SO4(-2a)", C::linear_298(0.2340, -1.5e-3), C::linear_298(2.527, 1.2e-2), C::constant(-48.33), C::constant(0.0044), ValidityRange::any()),
        bin_e("Fe(+2a)", "Cl(-a)", C::constant(0.3359), C::constant(1.532), C::EMPTY, C::constant(-0.0086), ValidityRange::t_25()),
        bin_e("Fe(+2a)", "SO4(-2a)", C::constant(0.2568), C::constant(3.063), C::constant(-42.0), C::constant(0.0209), ValidityRange::t_25()),
        bin_e("H(+a)", "Br(-a)", C::linear_298(0.1960, -2.0e-4), C::linear_298(0.3564, 5.0e-4), C::EMPTY, C::constant(0.00827), ValidityRange::any()),
        bin_e("H(+a)", "Cl(-a)", C::linear_298(0.1775, -3.08e-4), C::linear_298(0.2945, 1.42e-4), C::EMPTY, C::linear_298(0.00080, -6.2e-5), ValidityRange { min_t_c: Some(0.0), max_t_c: Some(250.0), min_m: None, max_m: Some(6.0), min_i: None, max_i: None }),
        bin_e("H(+a)", "HSO4(-a)", C::linear_298(0.2065, -4.0e-4), C::linear_298(0.5556, 6.0e-4), C::EMPTY, C::EMPTY, ValidityRange::any()),
        bin_e("H(+a)", "SO4(-2a)", C::linear_298(0.0298, -5.0e-4), C::EMPTY, C::EMPTY, C::linear_298(0.0438, -2.0e-4), ValidityRange::any()),
        bin_e("K(+a)", "Br(-a)", C::linear_298(0.0569, 7.4e-4), C::linear_298(0.2212, 1.07e-3), C::EMPTY, C::linear_298(-0.00180, -7.0e-5), ValidityRange::any()),
        bin_e("K(+a)", "Cl(-a)", C::linear_298(0.04835, 5.79e-4), C::linear_298(0.2122, 1.07e-3), C::EMPTY, C::linear_298(-0.00084, -5.1e-5), ValidityRange { min_t_c: Some(0.0), max_t_c: Some(300.0), min_m: None, max_m: Some(5.0), min_i: None, max_i: None }),
        bin_e("K(+a)", "F(-a)", C::constant(0.08089), C::constant(0.2021), C::EMPTY, C::constant(0.00093), ValidityRange::t_25()),
        bin_e("K(+a)", "HCO3(-a)", C::constant(0.0296), C::constant(-0.013), C::EMPTY, C::constant(-0.008), ValidityRange::t_25()),
        bin_e("K(+a)", "HSO4(-a)", C::constant(-0.0003), C::constant(0.1735), C::EMPTY, C::EMPTY, ValidityRange::t_25()),
        bin_e("K(+a)", "OH(-a)", C::linear_298(0.1298, 2.0e-4), C::linear_298(0.320, 1.2e-3), C::EMPTY, C::constant(0.0041), ValidityRange::any()),
        bin_e("K(+a)", "SO4(-2a)", C::linear_298(0.04995, 1.4e-3), C::linear_298(0.7793, 6.7e-3), C::EMPTY, C::EMPTY, ValidityRange::any()),
        bin_e("Li(+a)", "Cl(-a)", C::linear_298(0.1494, -1.7e-4), C::linear_298(0.3074, 5.4e-4), C::EMPTY, C::linear_298(0.00359, -4.5e-5), ValidityRange::any()),
        bin_e("Mg(+2a)", "Br(-a)", C::constant(0.4327), C::constant(1.753), C::EMPTY, C::constant(0.00312), ValidityRange::t_25()),
        bin_e("Mg(+2a)", "Cl(-a)", C::linear_298(0.35235, -1.9e-4), C::linear_298(1.6815, 3.6e-3), C::EMPTY, C::linear_298(0.00519, -1.6e-4), ValidityRange { min_t_c: Some(0.0), max_t_c: Some(200.0), min_m: None, max_m: Some(5.5), min_i: None, max_i: None }),
        bin_e("Mg(+2a)", "HCO3(-a)", C::constant(0.329), C::constant(0.6072), C::EMPTY, C::EMPTY, ValidityRange::t_25()),
        bin_e("Mg(+2a)", "SO4(-2a)", C::linear_298(0.2210, -7.0e-4), C::linear_298(3.343, 1.5e-2), C::linear_298(-37.23, -0.25), C::linear_298(0.0250, -3.0e-4), ValidityRange::any()),
        bin_e("Mn(+2a)", "SO4(-2a)", C::constant(0.2065), C::constant(2.9511), C::constant(-39.00), C::constant(0.0164), ValidityRange::t_25()),
        bin_e("Na(+a)", "Br(-a)", C::linear_298(0.0973, 7.7e-4), C::linear_298(0.2791, 1.07e-3), C::EMPTY, C::linear_298(0.00116, -9.0e-5), ValidityRange::any()),
        bin_e("Na(+a)", "Cl(-a)", C::linear_298(0.0765, 7.16e-4), C::linear_298(0.2664, 7.0e-4), C::EMPTY, C::linear_298(0.00127, -1.05e-4), ValidityRange { min_t_c: Some(0.0), max_t_c: Some(300.0), min_m: None, max_m: Some(6.0), min_i: None, max_i: None }),
        bin_e("Na(+a)", "F(-a)", C::constant(0.0215), C::constant(0.2107), C::EMPTY, C::EMPTY, ValidityRange::t_25()),
        bin_e("Na(+a)", "HCO3(-a)", C::linear_298(0.0277, 1.0e-3), C::linear_298(0.0411, 1.1e-3), C::EMPTY, C::EMPTY, ValidityRange::any()),
        bin_e("Na(+a)", "HS(-a)", C::linear_298(0.060, 4.0e-4), C::EMPTY, C::EMPTY, C::constant(0.0010), ValidityRange { min_t_c: None, max_t_c: Some(50.0), min_m: None, max_m: None, min_i: None, max_i: Some(1.0) }),
        bin_e("Na(+a)", "HSO3(-a)", C::linear_298(0.036, 5.0e-4), C::linear_298(0.150, 1.0e-3), C::linear_298(0.0, 0.0), C::linear_298(0.0, 0.0), ValidityRange::any()),
        bin_e("Na(+a)", "HSO4(-a)", C::linear_298(0.0454, 4.0e-4), C::linear_298(0.398, 1.5e-3), C::EMPTY, C::linear_298(0.0, 0.0), ValidityRange::any()),
        bin_e("Na(+a)", "NaSO4(-a)", C::linear_298(0.020, 8.0e-4), C::linear_298(0.180, 1.2e-3), C::EMPTY, C::linear_298(0.002, 0.0), ValidityRange::any()),
        bin_e("Na(+a)", "OH(-a)", C::linear_298(0.0864, 7.0e-4), C::linear_298(0.253, 1.34e-4), C::EMPTY, C::linear_298(0.0044, -1.9e-4), ValidityRange::any()),
        bin_e("Na(+a)", "S(-2a)", C::constant(0.15), C::constant(1.2), C::EMPTY, C::constant(0.0), ValidityRange::t_25()),
        bin_e("Na(+a)", "SO3(-2a)", C::linear_298(0.08015, 1.8e-3), C::linear_298(1.185, 5.0e-3), C::EMPTY, C::linear_298(-0.00436, -2.0e-4), ValidityRange::any()),
        bin_e("Na(+a)", "SO4(-2a)", C::linear_298(0.01958, 2.35e-3), C::linear_298(1.113, 5.6e-3), C::EMPTY, C::linear_298(0.00497, -4.8e-4), ValidityRange::any()),
        bin_e("Na(+a)", "S2O3(-2a)", C::constant(0.06615), C::constant(1.275), C::EMPTY, C::constant(0.00372), ValidityRange::t_25()),
        bin_e("Ni(+2a)", "SO4(-2a)", C::constant(0.1702), C::constant(2.907), C::constant(-40.06), C::constant(0.0366), ValidityRange::t_25()),
        bin_e("Sr(+2a)", "Cl(-a)", C::constant(0.2858), C::constant(1.6673), C::EMPTY, C::constant(-0.0013), ValidityRange::t_25()),
        bin_e("Zn(+2a)", "Cl(-a)", C::constant(0.2602), C::constant(1.642), C::EMPTY, C::constant(-0.088), ValidityRange::t_25()),
        bin_e("Zn(+2a)", "SO4(-2a)", C::constant(0.1949), C::constant(2.883), C::constant(-32.81), C::constant(0.0290), ValidityRange::t_25()),
    ];

    let thetas = vec![
        th_e("Al(+3a)", "Ca(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "Cu(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "Fe(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "H(+a)", C::constant(0.185), ValidityRange::any()),
        th_e("Al(+3a)", "K(+a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "Mg(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "Ni(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Al(+3a)", "Na(+a)", C::constant(0.0), ValidityRange::any()),
        th_e("H(+a)", "Na(+a)", C::linear_298(0.036, 0.0), ValidityRange { min_t_c: None, max_t_c: Some(90.0), min_m: None, max_m: None, min_i: None, max_i: Some(1.0) }),
        th_e("H(+a)", "K(+a)", C::constant(0.005), ValidityRange::any()),
        th_e("H(+a)", "Ca(+2a)", C::constant(0.092), ValidityRange::any()),
        th_e("H(+a)", "Mg(+2a)", C::constant(0.100), ValidityRange::any()),
        th_e("Na(+a)", "K(+a)", C::constant(-0.012), ValidityRange::any()),
        th_e("Na(+a)", "Ca(+2a)", C::constant(0.070), ValidityRange::any()),
        th_e("Na(+a)", "Mg(+2a)", C::constant(0.070), ValidityRange::any()),
        th_e("K(+a)", "Ca(+2a)", C::constant(0.032), ValidityRange::any()),
        th_e("K(+a)", "Mg(+2a)", C::constant(0.0), ValidityRange::any()),
        th_e("Ca(+2a)", "Mg(+2a)", C::constant(0.007), ValidityRange::any()),
        th_e("Cl(-a)", "SO4(-2a)", C::constant(0.020), ValidityRange::any()),
        th_e("Cl(-a)", "OH(-a)", C::constant(-0.050), ValidityRange::any()),
        th_e("Cl(-a)", "HCO3(-a)", C::constant(0.030), ValidityRange::any()),
        th_e("Cl(-a)", "HSO4(-a)", C::constant(-0.006), ValidityRange::any()),
        th_e("Cl(-a)", "Br(-a)", C::constant(0.0), ValidityRange::any()),
        th_e("HSO4(-a)", "SO4(-2a)", C::linear_298(0.0, 1.0e-4), ValidityRange::any()),
        th_e("OH(-a)", "SO4(-2a)", C::constant(-0.013), ValidityRange::any()),
    ];

    let psis = vec![
        psi_e("Al(+3a)", "Ca(+2a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Al(+3a)", "Cu(+2a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Al(+3a)", "Fe(+2a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Al(+3a)", "H(+a)", "Cl(-a)", C::linear_298(0.014, 1.0e-4), ValidityRange::any()),
        psi_e("Al(+3a)", "K(+a)", "Cl(-a)", C::linear_298(-0.020, 1.0e-4), ValidityRange::any()),
        psi_e("Al(+3a)", "Mg(+2a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Al(+3a)", "Na(+a)", "Cl(-a)", C::linear_298(-0.014, 1.0e-4), ValidityRange::any()),
        psi_e("Al(+3a)", "Ni(+2a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Al(OH)4(-a)", "OH(-a)", "K(+a)", C::constant(0.0), ValidityRange::any()),
        psi_e("H(+a)", "Na(+a)", "Cl(-a)", C::constant(-0.004), ValidityRange::any()),
        psi_e("H(+a)", "Na(+a)", "SO4(-2a)", C::constant(0.0), ValidityRange::any()),
        psi_e("Na(+a)", "K(+a)", "Cl(-a)", C::constant(-0.0018), ValidityRange::any()),
        psi_e("Na(+a)", "Ca(+2a)", "Cl(-a)", C::constant(-0.007), ValidityRange::any()),
        psi_e("Na(+a)", "Mg(+2a)", "Cl(-a)", C::constant(-0.012), ValidityRange::any()),
        psi_e("K(+a)", "Mg(+2a)", "Cl(-a)", C::constant(-0.022), ValidityRange::any()),
        psi_e("Ca(+2a)", "Mg(+2a)", "Cl(-a)", C::constant(-0.012), ValidityRange::any()),
        psi_e("Na(+a)", "K(+a)", "SO4(-2a)", C::constant(-0.010), ValidityRange::any()),
        psi_e("Na(+a)", "Mg(+2a)", "SO4(-2a)", C::constant(-0.015), ValidityRange::any()),
        psi_e("K(+a)", "Mg(+2a)", "SO4(-2a)", C::constant(-0.048), ValidityRange::any()),
        psi_e("Na(+a)", "Cl(-a)", "SO4(-2a)", C::constant(0.0014), ValidityRange::any()),
        psi_e("Mg(+2a)", "Cl(-a)", "SO4(-2a)", C::constant(-0.004), ValidityRange::any()),
        psi_e("Ca(+2a)", "Cl(-a)", "SO4(-2a)", C::constant(-0.018), ValidityRange::any()),
        psi_e("Na(+a)", "Cl(-a)", "OH(-a)", C::constant(-0.006), ValidityRange::any()),
        psi_e("Na(+a)", "Cl(-a)", "HCO3(-a)", C::constant(-0.015), ValidityRange::any()),
        psi_e("Na(+a)", "OH(-a)", "SO4(-2a)", C::constant(-0.009), ValidityRange::any()),
    ];

    let lambdas = vec![
        lam_e("Ar(a)", "Br(-a)", C::constant(0.0), ValidityRange::any()),
        lam_e("Ar(a)", "Ca(+2a)", C::constant(0.28), ValidityRange::any()),
        lam_e("Ar(a)", "H(+a)", C::constant(0.02), ValidityRange::any()),
        lam_e("Ar(a)", "HCO3(-a)", C::constant(0.05), ValidityRange::any()),
        lam_e("Ar(a)", "K(+a)", C::constant(0.11), ValidityRange::any()),
        lam_e("Ar(a)", "Mg(+2a)", C::constant(0.27), ValidityRange::any()),
        lam_e("Ar(a)", "Na(+a)", C::constant(0.15), ValidityRange::any()),
        lam_e("Ar(a)", "OH(-a)", C::constant(0.11), ValidityRange::any()),
        lam_e("Ar(a)", "SO4(-2a)", C::constant(0.19), ValidityRange::any()),
        lam_e("B(OH)3(a)", "Cl(-a)", C::constant(0.091), ValidityRange::any()),
        lam_e("B(OH)3(a)", "H(+a)", C::constant(0.0), ValidityRange::any()),
        lam_e("B(OH)3(a)", "Na(+a)", C::constant(-0.097), ValidityRange::any()),
        lam_e("CO2(a)", "Na(+a)", C::constant(0.100), ValidityRange::any()),
        lam_e("CO2(a)", "Cl(-a)", C::constant(-0.005), ValidityRange::any()),
        lam_e("CO2(a)", "K(+a)", C::constant(0.051), ValidityRange::any()),
        lam_e("CO2(a)", "Ca(+2a)", C::constant(0.183), ValidityRange::any()),
        lam_e("CO2(a)", "Mg(+2a)", C::constant(0.183), ValidityRange::any()),
        lam_e("CO2(a)", "SO4(-2a)", C::constant(0.097), ValidityRange::any()),
    ];

    PitzerDatabase {
        binaries,
        thetas,
        psis,
        lambdas,
    }
}
