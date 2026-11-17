//! Gas Mixture Radiation Calculator (Exponential Wide Band Model — Edwards & Balakrishnan 1973,
//! Lallemant & Weber 1996) and Combined Gas + Particle Suspension Radiation (Eqs. 44-53, Table 1).

use super::radiation::validate_emissivity;
use serde::{Deserialize, Serialize};
use soryntel_core::{SoryntelError, STEFAN_BOLTZMANN};
use std::f64::consts::PI;

/// Geometry presets for optical depth `L` (Table 1 of 12-heat-loss.md).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OpticalDepthGeometry {
    Manual(f64),
    Sphere { diameter_m: f64 },
    Cylinder { height_m: f64, diameter_m: f64 },
    Cube { a_m: f64, b_m: f64, c_m: f64 },
}

impl OpticalDepthGeometry {
    /// Compute mean beam length / optical depth `L` [m] per Table 1 & bilinear extrapolation example.
    pub fn optical_depth_m(&self) -> Result<f64, SoryntelError> {
        match *self {
            Self::Manual(l) => {
                if l <= 0.0 {
                    return Err(SoryntelError::HeatLossError(
                        "Optical depth L must be > 0".into(),
                    ));
                }
                Ok(l)
            }
            Self::Sphere { diameter_m } => Ok(0.63 * diameter_m.max(1e-6)),
            Self::Cylinder {
                height_m,
                diameter_m,
            } => {
                let d = diameter_m.max(1e-6);
                let ratio = (height_m / d).max(0.1);
                // Table 1: h=0.5D -> 0.45D, h=D -> 0.60D, h=2D -> 0.73D
                let factor = if ratio <= 1.0 {
                    0.45 + (0.60 - 0.45) * ((ratio - 0.5) / 0.5)
                } else {
                    0.60 + (0.73 - 0.60) * (ratio - 1.0)
                }
                .clamp(0.25, 0.95);
                Ok(factor * d)
            }
            Self::Cube { a_m, b_m, c_m } => {
                let mut dims = [a_m.max(1e-6), b_m.max(1e-6), c_m.max(1e-6)];
                dims.sort_by(|x, y| x.partial_cmp(y).unwrap());
                let x = dims[0];
                let r2 = dims[1] / x; // >= 1
                let r3 = dims[2] / x; // >= r2
                // Table 1 & Ch. 12.4.3 worked example (1x4x7 -> 1.35):
                // Step 1: extrapolate 1x1xr3 from (1x1x1 -> 0.60) and (1x1x4 -> 0.81)
                let l_1_1_r3 = 0.60 + (0.81 - 0.60) * ((r3 - 1.0) / (4.0 - 1.0));
                if (r2 - 1.0).abs() <= 1e-9 {
                    return Ok(l_1_1_r3.max(0.3) * x);
                }
                // Step 2: along ray (1x1x4 -> 0.81) through (1x2x6 -> 1.06),
                // intermediate aspect b_ref = 1 + (2 - 1) * (r3 - 4) / (6 - 4)
                let b_ref = 1.0 + 0.5 * (r3 - 4.0);
                let factor = if (b_ref - 1.0).abs() > 1e-6 {
                    let l_1_bref_r3 = 0.81 + (1.06 - 0.81) * ((b_ref - 1.0) / (2.0 - 1.0));
                    // Step 3: interpolate/extrapolate in r2 from b=1 to b=b_ref
                    l_1_1_r3 + (l_1_bref_r3 - l_1_1_r3) * ((r2 - 1.0) / (b_ref - 1.0))
                } else {
                    l_1_1_r3 + 0.11 * (r2 - 1.0)
                };
                Ok(factor.max(0.3) * x)
            }
        }
    }
}

/// Molar fractions of the 6 radiating gas species (sum <= 1.0; remainder is non-radiating e.g. N2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct GasComposition {
    pub x_h2o: f64,
    pub x_co2: f64,
    pub x_co: f64,
    pub x_no: f64,
    pub x_so2: f64,
    pub x_ch4: f64,
}

impl GasComposition {
    pub fn total_radiating_fraction(&self) -> f64 {
        self.x_h2o + self.x_co2 + self.x_co + self.x_no + self.x_so2 + self.x_ch4
    }

    pub fn validate(&self) -> Result<(), SoryntelError> {
        let arr = [
            self.x_h2o, self.x_co2, self.x_co, self.x_no, self.x_so2, self.x_ch4,
        ];
        if arr.iter().any(|&x| !(0.0..=1.0).contains(&x)) || self.total_radiating_fraction() > 1.0001
        {
            return Err(SoryntelError::HeatLossError(
                "Gas mole fractions must be in [0, 1] and sum <= 1.0".into(),
            ));
        }
        Ok(())
    }
}

/// Input parameters for Gas Mixture Radiation Calculator (Fig. 9).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GasRadiationInput {
    pub t_gas_k: f64,
    pub t_surface_k: f64,
    pub pressure_bar: f64,
    pub optical_depth_m: f64,
    pub surface_emissivity: f64,
    pub composition: GasComposition,
}

/// One spectral band row for the Gas Radiation table (`Nyy-Low`, `Nyy-Upp`, `F-Low`, `F-Upp`, `DF`, `SumDF`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralBandRow {
    pub species: String,
    pub band_name: String,
    pub nyy_low_cm_1: f64,
    pub nyy_upp_cm_1: f64,
    pub f_low: f64,
    pub f_upp: f64,
    pub df: f64,
    pub band_emissivity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasRadiationResult {
    pub gas_emissivity: f64,
    pub gas_absorptivity: f64,
    pub heat_flux_w_m2: f64,
    pub hr_w_m2_k: f64,
    pub bands: Vec<SpectralBandRow>,
    #[serde(default)]
    pub wall_bands: Vec<SpectralBandRow>,
}

/// Complete 8-row User Input specification for the Particle Suspension Radiation Calculator (Fig. 10).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParticleSuspensionSpec {
    pub gas_amount_nm3_h: f64,
    pub particle_amount_t_h: f64,
    pub particle_density_g_ml: f64,
    pub particle_diameter_mm: f64,
    pub particle_emissivity: f64,
    pub cube_width_height_m: f64,
    pub cube_thickness_m: f64,
    pub tube_diameter_m: f64,
    pub t_gas_k: f64,
    pub pressure_bar: f64,
    pub gas_emissivity_at_gas_temp: f64,
    pub gas_absorptivity_at_surface_temp: f64,
    pub t_surface_k: f64,
    pub surface_emissivity: f64,
}

impl Default for ParticleSuspensionSpec {
    fn default() -> Self {
        // Exact default values from Fig. 10 of 12-heat-loss.md
        Self {
            gas_amount_nm3_h: 24100.0,
            particle_amount_t_h: 69.0,
            particle_density_g_ml: 4.3,
            particle_diameter_mm: 0.050,
            particle_emissivity: 1.000,
            cube_width_height_m: 0.002,
            cube_thickness_m: 0.100,
            tube_diameter_m: 4.200,
            t_gas_k: 232.0 + 273.15,
            pressure_bar: 1.0,
            gas_emissivity_at_gas_temp: 0.1480,
            gas_absorptivity_at_surface_temp: 0.1562,
            t_surface_k: 23.0 + 273.15,
            surface_emissivity: 1.000,
        }
    }
}

/// Complete 16-row `Results` table output for the Particle Suspension Radiation Calculator (Fig. 10).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParticleSuspensionDetailedResult {
    pub gas_flow_m3_h: f64,
    pub particle_flow_m3_h: f64,
    pub total_flow_m3_h: f64,
    pub particles_g_m3: f64,
    pub particles_ml_m3: f64,
    pub particles_vol_pct: f64,
    pub particle_radius_mm: f64,
    pub particle_cross_section_mm2: f64,
    pub particle_volume_mm3: f64,
    pub particle_weight_g: f64,
    pub particles_per_m3: f64,
    pub cube_volume_m3: f64,
    pub particles_in_cube: f64,
    pub particle_distance_mm: f64,
    pub gas_speed_m3_s: f64,
    pub gas_speed_in_tube_m_s: f64,
    pub cloud_emissivity: f64,
    pub total_emissivity_gc: f64,
    pub total_absorptivity_gc: f64,
    pub heat_flux_kw_m2: f64,
    pub hr_w_m2_k: f64,
}

/// Particle suspension inputs (Eqs. 48-53, Fig. 10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleCloudInput {
    pub particle_emissivity: f64,
    pub number_density_m3: f64,
    pub cloud_thickness_m: f64,
    /// Pairs `(diameter_m, count_weight)`. If single uniform size, provide `[(d, 1.0)]`.
    pub size_distribution: Vec<(f64, f64)>,
}

impl ParticleCloudInput {
    /// Surface mean cross-sectional area `A_p` [m^2] (Eq. 49) and cloud emissivity `eps_c` (Eq. 48).
    pub fn cloud_emissivity(&self) -> Result<(f64, f64), SoryntelError> {
        let eps_p = validate_emissivity(self.particle_emissivity)?;
        if self.size_distribution.is_empty() {
            return Err(SoryntelError::HeatLossError(
                "Particle size distribution cannot be empty".into(),
            ));
        }
        let mut sum_nd2 = 0.0;
        let mut sum_n = 0.0;
        for &(d, n) in &self.size_distribution {
            if d > 0.0 && n > 0.0 {
                sum_nd2 += n * d * d;
                sum_n += n;
            }
        }
        if sum_n <= 0.0 {
            return Ok((0.0, 0.0));
        }
        let d_s_sq = sum_nd2 / sum_n;
        let a_p = 0.25 * PI * d_s_sq;
        let tau = eps_p * self.number_density_m3.max(0.0) * self.cloud_thickness_m.max(0.0) * a_p;
        let eps_c = 1.0 - (-tau).exp();
        Ok((d_s_sq.sqrt(), eps_c.clamp(0.0, 1.0)))
    }
}

/// Exponential Wide Band parameters (Edwards 1983; Lallemant & Weber 1996).
struct BandParams {
    species: &'static str,
    band_name: &'static str,
    nu_center_cm1: f64,
    /// Integrated band intensity at 100 K [cm^-2 * (g/m^2)^-1] scaled in atm^-1·cm^-2
    alpha0_cm2_atm: f64,
    /// Exponential band width parameter omega_0 [cm^-1]
    omega0_cm1: f64,
    /// Line-width-to-spacing parameter beta_0 at 100 K
    beta0: f64,
    /// Self-broadening factor b_self
    b_self: f64,
    #[allow(dead_code)]
    t_ref_k: f64,
}

const BANDS: &[BandParams] = &[
    // H2O(g) rotational + 6.3, 2.7, 1.87, 1.38 um bands
    BandParams { species: "H2O", band_name: "Rotational", nu_center_cm1: 250.0, alpha0_cm2_atm: 1850.0, omega0_cm1: 180.0, beta0: 0.143, b_self: 5.0, t_ref_k: 100.0 },
    BandParams { species: "H2O", band_name: "6.3 um", nu_center_cm1: 1600.0, alpha0_cm2_atm: 41.2, omega0_cm1: 56.4, beta0: 0.094, b_self: 5.0, t_ref_k: 100.0 },
    BandParams { species: "H2O", band_name: "2.7 um", nu_center_cm1: 3760.0, alpha0_cm2_atm: 23.3, omega0_cm1: 60.0, beta0: 0.132, b_self: 5.0, t_ref_k: 100.0 },
    BandParams { species: "H2O", band_name: "1.87 um", nu_center_cm1: 5350.0, alpha0_cm2_atm: 3.0, omega0_cm1: 43.1, beta0: 0.082, b_self: 5.0, t_ref_k: 100.0 },
    BandParams { species: "H2O", band_name: "1.38 um", nu_center_cm1: 7250.0, alpha0_cm2_atm: 2.5, omega0_cm1: 32.0, beta0: 0.116, b_self: 5.0, t_ref_k: 100.0 },
    // CO2(g) 15, 10.4, 9.4, 4.3, 2.7, 2.0 um bands
    BandParams { species: "CO2", band_name: "15 um", nu_center_cm1: 667.0, alpha0_cm2_atm: 19.0, omega0_cm1: 12.7, beta0: 0.062, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CO2", band_name: "10.4 um", nu_center_cm1: 960.0, alpha0_cm2_atm: 0.052, omega0_cm1: 13.4, beta0: 0.040, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CO2", band_name: "4.3 um", nu_center_cm1: 2410.0, alpha0_cm2_atm: 110.0, omega0_cm1: 11.2, beta0: 0.247, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CO2", band_name: "2.7 um", nu_center_cm1: 3660.0, alpha0_cm2_atm: 4.0, omega0_cm1: 23.5, beta0: 0.133, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CO2", band_name: "2.0 um", nu_center_cm1: 5000.0, alpha0_cm2_atm: 0.08, omega0_cm1: 28.0, beta0: 0.120, b_self: 1.3, t_ref_k: 100.0 },
    // CO(g) 4.7, 2.35 um
    BandParams { species: "CO", band_name: "4.7 um", nu_center_cm1: 2143.0, alpha0_cm2_atm: 20.9, omega0_cm1: 25.5, beta0: 0.075, b_self: 1.1, t_ref_k: 100.0 },
    BandParams { species: "CO", band_name: "2.35 um", nu_center_cm1: 4260.0, alpha0_cm2_atm: 0.14, omega0_cm1: 20.0, beta0: 0.168, b_self: 1.1, t_ref_k: 100.0 },
    // NO(g) 5.3 um
    BandParams { species: "NO", band_name: "5.3 um", nu_center_cm1: 1876.0, alpha0_cm2_atm: 11.8, omega0_cm1: 20.0, beta0: 0.180, b_self: 1.0, t_ref_k: 100.0 },
    // SO2(g) 19.3, 8.7, 7.3, 4.0 um
    BandParams { species: "SO2", band_name: "19.3 um", nu_center_cm1: 518.0, alpha0_cm2_atm: 4.5, omega0_cm1: 16.0, beta0: 0.250, b_self: 1.5, t_ref_k: 100.0 },
    BandParams { species: "SO2", band_name: "8.7 um", nu_center_cm1: 1151.0, alpha0_cm2_atm: 3.7, omega0_cm1: 14.0, beta0: 0.220, b_self: 1.5, t_ref_k: 100.0 },
    BandParams { species: "SO2", band_name: "7.3 um", nu_center_cm1: 1362.0, alpha0_cm2_atm: 25.0, omega0_cm1: 12.0, beta0: 0.300, b_self: 1.5, t_ref_k: 100.0 },
    // CH4(g) 7.7, 3.3, 2.3 um
    BandParams { species: "CH4", band_name: "7.7 um", nu_center_cm1: 1306.0, alpha0_cm2_atm: 28.0, omega0_cm1: 21.0, beta0: 0.087, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CH4", band_name: "3.3 um", nu_center_cm1: 3020.0, alpha0_cm2_atm: 46.0, omega0_cm1: 23.0, beta0: 0.056, b_self: 1.3, t_ref_k: 100.0 },
    BandParams { species: "CH4", band_name: "2.3 um", nu_center_cm1: 4220.0, alpha0_cm2_atm: 2.9, omega0_cm1: 35.0, beta0: 0.120, b_self: 1.3, t_ref_k: 100.0 },
];

/// Blackbody fractional function F(0 -> nu_cm1, T_K) via rapid series expansion
/// (C2 = 1.438777 cm·K).
fn planck_blackbody_fraction(nu_cm1: f64, t_k: f64) -> f64 {
    if nu_cm1 <= 0.0 || t_k <= 0.0 {
        return 0.0;
    }
    const C2_CM_K: f64 = 1.438_777;
    // lambda_um * T_K = 10000 * T_K / nu_cm1; x = C2 * nu / T
    let x = C2_CM_K * nu_cm1 / t_k;
    if x > 60.0 {
        return 1.0;
    }
    // F(0 -> lambda) = 1 - F(0 -> nu) where lambda*T = 10000 / (nu/T)
    // Using standard series for int_{x}^{inf} u^3/(e^u - 1) du * 15 / pi^4 = 1 - F(nu, T)
    let mut tail = 0.0;
    for m in 1..=12 {
        let mf = m as f64;
        let mx = mf * x;
        if mx > 80.0 {
            break;
        }
        tail += ((-mx).exp() / mf.powi(4)) * (((mx + 3.0) * mx + 6.0) * mx + 6.0);
    }
    let f_lambda_to_inf = (15.0 / PI.powi(4)) * tail;
    (1.0 - f_lambda_to_inf).clamp(0.0, 1.0)
}

/// Core EWBM gas emissivity `eps_g(T_g, p_bar, L_m, composition)` (Eq. 46).
fn compute_ewbm_emissivity(
    t_k: f64,
    p_bar: f64,
    l_m: f64,
    comp: &GasComposition,
) -> (f64, Vec<SpectralBandRow>) {
    let t_eff = t_k.max(250.0);
    let p_atm = (p_bar * 0.986923).max(0.01);
    let sqrt_t = (t_eff / 100.0).sqrt();

    let species_x_molar = |sp: &str| -> (f64, f64) {
        match sp {
            "H2O" => (comp.x_h2o, 18.015),
            "CO2" => (comp.x_co2, 44.010),
            "CO" => (comp.x_co, 28.010),
            "NO" => (comp.x_no, 30.006),
            "SO2" => (comp.x_so2, 64.064),
            "CH4" => (comp.x_ch4, 16.043),
            _ => (0.0, 28.0),
        }
    };

    let mut rows = Vec::new();
    // Track per-band absorptances and combine overlapping bands multiplicatively: 1 - prod(1 - eps_i)
    let mut prod_trans = 1.0_f64;

    for b in BANDS {
        let (x_sp, m_g_mol) = species_x_molar(b.species);
        if x_sp <= 1e-9 {
            continue;
        }

        // Mass path length u [g/m^2] = (p_i * M_i / (R * T)) * L
        let p_pa = p_bar.max(0.01) * 1e5;
        let u = (x_sp * p_pa * m_g_mol / (soryntel_core::R_JOULE * t_eff)) * l_m.max(1e-4);
        let omega = b.omega0_cm1 * sqrt_t;
        let p_e = p_atm * (1.0 + (b.b_self - 1.0) * x_sp);
        let beta = b.beta0 / sqrt_t;
        let eta = beta * p_e;
        let tau_0 = (b.alpha0_cm2_atm * u) / omega.max(1e-6);

        // Lallemant & Weber (1996) four-regime closed-form effective bandwidth A / omega
        let a_over_omega = if tau_0 <= 1.0 {
            if tau_0 <= eta {
                tau_0
            } else {
                (2.0 * (tau_0 * eta).sqrt() - eta).min(tau_0)
            }
        } else if eta >= 1.0 {
            tau_0.ln() + 1.0
        } else {
            let t_cross = 1.0 / eta.max(1e-6);
            if tau_0 <= t_cross {
                2.0 * (tau_0 * eta).sqrt() - eta
            } else {
                (tau_0 * eta).ln() + 2.0 - eta
            }
        }
        .max(0.0);

        let delta_nu = (omega * a_over_omega).min(2.0 * b.nu_center_cm1);
        if delta_nu <= 1e-6 {
            continue;
        }
        let nu_low = (b.nu_center_cm1 - 0.5 * delta_nu).max(1.0);
        let nu_upp = b.nu_center_cm1 + 0.5 * delta_nu;
        let f_low = planck_blackbody_fraction(nu_low, t_eff);
        let f_upp = planck_blackbody_fraction(nu_upp, t_eff);
        let df = (f_upp - f_low).max(0.0);

        // In EWBM, delta_nu = omega * (A/omega) is the equivalent black bandwidth
        let band_eps = df.clamp(0.0, 1.0);
        prod_trans *= 1.0 - band_eps;

        rows.push(SpectralBandRow {
            species: b.species.to_string(),
            band_name: b.band_name.to_string(),
            nyy_low_cm_1: nu_low,
            nyy_upp_cm_1: nu_upp,
            f_low,
            f_upp,
            df,
            band_emissivity: band_eps,
        });
    }

    let eps_g = (1.0 - prod_trans).clamp(0.0, 0.999);
    (eps_g, rows)
}

/// Calculate gas mixture radiation (Eqs. 44-47).
pub fn calculate_gas_radiation(input: &GasRadiationInput) -> Result<GasRadiationResult, SoryntelError> {
    input.composition.validate()?;
    let eps_s = validate_emissivity(input.surface_emissivity)?;
    if input.t_gas_k <= 0.0 || input.t_surface_k <= 0.0 {
        return Err(SoryntelError::InvalidTemperature(input.t_gas_k));
    }

    let (eps_g, bands) = compute_ewbm_emissivity(
        input.t_gas_k,
        input.pressure_bar,
        input.optical_depth_m,
        &input.composition,
    );

    // Eq. (47): Gas absorptivity with T_s / T_g correction factors
    let l_abs = input.optical_depth_m * (input.t_surface_k / input.t_gas_k).powf(1.5);
    let (eps_g_corr, mut wall_bands) = compute_ewbm_emissivity(
        input.t_gas_k,
        input.pressure_bar,
        l_abs,
        &input.composition,
    );
    let corr_sqrt = (input.t_gas_k / input.t_surface_k).sqrt();
    for wb in &mut wall_bands {
        wb.df = (wb.df * corr_sqrt).clamp(0.0, 1.0);
        wb.band_emissivity = wb.df;
    }
    let alpha_g = (eps_g_corr * corr_sqrt).clamp(0.0, 0.999);

    // Eq. (44)-(45)
    let q_flux =
        STEFAN_BOLTZMANN * 0.5 * (eps_s + 1.0) * (eps_g * input.t_gas_k.powi(4) - alpha_g * input.t_surface_k.powi(4));
    let dt = input.t_gas_k - input.t_surface_k;
    let hr = if dt.abs() < 1e-6 {
        0.0
    } else {
        (q_flux / dt).max(0.0)
    };

    Ok(GasRadiationResult {
        gas_emissivity: eps_g,
        gas_absorptivity: alpha_g,
        heat_flux_w_m2: q_flux,
        hr_w_m2_k: hr,
        bands,
        wall_bands,
    })
}

/// Combined Gas + Particle Suspension Radiation (Eqs. 48-53).
pub fn calculate_gas_particle_radiation(
    gas_input: &GasRadiationInput,
    particle_input: &ParticleCloudInput,
) -> Result<(GasRadiationResult, f64, f64, f64), SoryntelError> {
    let gas_res = calculate_gas_radiation(gas_input)?;
    let eps_s = validate_emissivity(gas_input.surface_emissivity)?;
    let (d_s_m, eps_c) = particle_input.cloud_emissivity()?;

    // Eqs. (50)-(51)
    let eps_gc = gas_res.gas_emissivity + eps_c - gas_res.gas_emissivity * eps_c;
    let alpha_gc = gas_res.gas_absorptivity + eps_c - gas_res.gas_absorptivity * eps_c;
    let denom = alpha_gc + eps_s - alpha_gc * eps_s;
    if denom <= 1e-12 {
        return Err(SoryntelError::HeatLossError(
            "Zero absorptivity and emissivity !".into(),
        ));
    }

    // Eqs. (52)-(53)
    let q_flux = (eps_s / denom)
        * STEFAN_BOLTZMANN
        * (eps_gc * gas_input.t_gas_k.powi(4) - alpha_gc * gas_input.t_surface_k.powi(4));
    let dt = gas_input.t_gas_k - gas_input.t_surface_k;
    let hr = if dt.abs() < 1e-6 {
        0.0
    } else {
        (q_flux / dt).max(0.0)
    };

    Ok((
        GasRadiationResult {
            gas_emissivity: eps_gc,
            gas_absorptivity: alpha_gc,
            heat_flux_w_m2: q_flux,
            hr_w_m2_k: hr,
            bands: gas_res.bands,
            wall_bands: gas_res.wall_bands,
        },
        d_s_m,
        eps_c,
        hr,
    ))
}

/// Evaluate all 16 rows of the Particle Suspension Radiation Calculator `Results` table (Fig. 10, Eqs. 48-53).
pub fn evaluate_particle_suspension_detailed(
    spec: &ParticleSuspensionSpec,
) -> Result<ParticleSuspensionDetailedResult, SoryntelError> {
    let eps_p = validate_emissivity(spec.particle_emissivity)?;
    let eps_s = validate_emissivity(spec.surface_emissivity)?;
    let t_g = spec.t_gas_k.max(1.0);
    let t_s = spec.t_surface_k.max(1.0);
    let p_bar = spec.pressure_bar.max(0.01);

    // Gas flow rate at operating T and P from normal flow at 273.15 K, 1.01325 bar
    let gas_flow_m3_h =
        spec.gas_amount_nm3_h.max(0.0) * (t_g / 273.15) * (1.01325 / p_bar);
    // Particle volumetric flow rate: (t/h) / (t/m^3 = g/ml)
    let rho_t_m3 = spec.particle_density_g_ml.max(1e-6);
    let particle_flow_m3_h = spec.particle_amount_t_h.max(0.0) / rho_t_m3;
    let total_flow_m3_h = (gas_flow_m3_h + particle_flow_m3_h).max(1e-9);

    // Particle concentration in suspension
    let particles_g_m3 = (spec.particle_amount_t_h.max(0.0) * 1e6) / total_flow_m3_h;
    let particles_ml_m3 = (particle_flow_m3_h * 1e6) / total_flow_m3_h;
    let particles_vol_pct = 100.0 * (particle_flow_m3_h / total_flow_m3_h);

    // Single particle geometry
    let d_mm = spec.particle_diameter_mm.max(1e-6);
    let r_mm = 0.5 * d_mm;
    let a_p_mm2 = PI * r_mm * r_mm;
    let v_p_mm3 = (4.0 / 3.0) * PI * r_mm.powi(3);
    // Weight in grams: V [cm^3 = mm^3 * 1e-3] * rho [g/cm^3]
    let w_p_g = (v_p_mm3 * 1e-3) * rho_t_m3;

    let n_p_m3 = if w_p_g > 0.0 {
        particles_g_m3 / w_p_g
    } else {
        0.0
    };

    let cube_vol_m3 =
        spec.cube_width_height_m.max(0.0).powi(2) * spec.cube_thickness_m.max(0.0);
    let particles_in_cube = n_p_m3 * cube_vol_m3;
    let particle_distance_mm = if n_p_m3 > 0.0 {
        1000.0 / n_p_m3.cbrt()
    } else {
        0.0
    };

    let gas_speed_m3_s = total_flow_m3_h / 3600.0;
    let tube_area_m2 = 0.25 * PI * spec.tube_diameter_m.max(1e-4).powi(2);
    let gas_speed_in_tube_m_s = gas_speed_m3_s / tube_area_m2;

    // Eq. (48): eps_c = 1 - exp(-eps_p * n_p * L * A_p)
    let a_p_m2 = a_p_mm2 * 1e-6;
    let l_cloud_m = spec.cube_thickness_m.max(0.0);
    let tau = eps_p * n_p_m3 * l_cloud_m * a_p_m2;
    let cloud_emissivity = (1.0 - (-tau).exp()).clamp(0.0, 1.0);

    // Eqs. (50)-(53)
    let eps_g = spec.gas_emissivity_at_gas_temp.clamp(0.0, 1.0);
    let alpha_g = spec.gas_absorptivity_at_surface_temp.clamp(0.0, 1.0);
    let eps_gc = eps_g + cloud_emissivity - eps_g * cloud_emissivity;
    let alpha_gc = alpha_g + cloud_emissivity - alpha_g * cloud_emissivity;
    let denom = (alpha_gc + eps_s - alpha_gc * eps_s).max(1e-12);
    let q_flux_w_m2 =
        (eps_s / denom) * STEFAN_BOLTZMANN * (eps_gc * t_g.powi(4) - alpha_gc * t_s.powi(4));
    let dt = t_g - t_s;
    let hr_w_m2_k = if dt.abs() < 1e-6 {
        0.0
    } else {
        (q_flux_w_m2 / dt).max(0.0)
    };

    Ok(ParticleSuspensionDetailedResult {
        gas_flow_m3_h,
        particle_flow_m3_h,
        total_flow_m3_h,
        particles_g_m3,
        particles_ml_m3,
        particles_vol_pct,
        particle_radius_mm: r_mm,
        particle_cross_section_mm2: a_p_mm2,
        particle_volume_mm3: v_p_mm3,
        particle_weight_g: w_p_g,
        particles_per_m3: n_p_m3,
        cube_volume_m3: cube_vol_m3,
        particles_in_cube,
        particle_distance_mm,
        gas_speed_m3_s,
        gas_speed_in_tube_m_s,
        cloud_emissivity,
        total_emissivity_gc: eps_gc,
        total_absorptivity_gc: alpha_gc,
        heat_flux_kw_m2: q_flux_w_m2 / 1000.0,
        hr_w_m2_k,
    })
}

/// Generate the 0 °C to 3000 °C (step 50 °C) iterative emissivity table (`Select iterative`, Ch. 12.3.7).
pub fn generate_iterative_gas_radiation_table(
    pressure_bar: f64,
    optical_depth_m: f64,
    comp: &GasComposition,
) -> Vec<(f64, f64)> {
    let mut table = Vec::with_capacity(61);
    for step in 0..=60 {
        let t_c = (step as f64) * 50.0;
        let t_k = t_c + 273.15;
        let (eps_g, _) = compute_ewbm_emissivity(t_k, pressure_bar, optical_depth_m, comp);
        table.push((t_k, eps_g));
    }
    table
}
