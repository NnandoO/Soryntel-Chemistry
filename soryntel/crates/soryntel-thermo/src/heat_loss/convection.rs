//! Free and forced convection correlations (Section 12.4.2, Eqs. 13-40)
//! and custom polynomial/power function `hc(v) = sum(c_j * v^p_j)`.

use super::model::{
    ConvectionFunctionTerm, ConvectionGeometry, ConvectionRegime, FluidPropertyTable,
};
use soryntel_core::{SoryntelError, GRAVITY_STANDARD};

/// Diagnostic dimensionless numbers matching the Convection Dialog `RESULTS` panel (Fig. 6).
#[derive(Debug, Clone, Copy)]
pub struct ConvectionDiagnostics {
    pub char_length_m: f64,
    pub aspect_ratio_h_l: f64,
    pub t_film_k: f64,
    pub nusselt: f64,
    pub rayleigh: f64,
    pub grashof: f64,
    pub reynolds: f64,
    pub peclet_re_pr: f64,
    pub hc_w_m2_k: f64,
}

/// Evaluate custom velocity function `hc(v) = sum(coeff * v^exponent)`.
pub fn evaluate_hc_function(terms: &[ConvectionFunctionTerm], speed_m_s: f64) -> f64 {
    let v = speed_m_s.max(0.0);
    terms
        .iter()
        .map(|t| {
            if v == 0.0 && t.exponent == 0.0 {
                t.coeff
            } else if v == 0.0 && t.exponent > 0.0 {
                0.0
            } else {
                t.coeff * v.powf(t.exponent)
            }
        })
        .sum::<f64>()
        .max(0.0)
}

/// Evaluate convection coefficient `hc` and dimensionless diagnostics using Eqs. (13)-(40).
pub fn evaluate_convection(
    fluid: &FluidPropertyTable,
    regime: ConvectionRegime,
    geom: ConvectionGeometry,
    t_surface_k: f64,
    t_ambient_k: f64,
    char_length_m: f64,
    enclosure_height_m: Option<f64>,
    speed_m_s: f64,
    outer_diameter_m: Option<f64>,
) -> Result<ConvectionDiagnostics, SoryntelError> {
    let l = char_length_m.max(1e-6);
    let t_f = 0.5 * (t_surface_k + t_ambient_k);
    let props = fluid.interpolate(t_f)?;

    let nu = props.nu_m2_s();
    let alpha = props.alpha_m2_s();
    let pr = props.prandtl().max(1e-6);
    let beta = props.beta_or_ideal(t_f);
    let dt = (t_surface_k - t_ambient_k).abs();

    // Eq. (15)-(16): Rayleigh & Grashof
    let ra_l = (GRAVITY_STANDARD * beta * dt * l.powi(3)) / (nu * alpha);
    let gr_l = ra_l / pr;
    // Eq. (36): Reynolds
    let re_l = (speed_m_s.max(0.0) * l) / nu;
    let pe_l = re_l * pr;

    let h_l = enclosure_height_m.map(|h| h / l).unwrap_or(1.0);

    let nusselt = match regime {
        ConvectionRegime::Free => free_convection_nusselt(geom, ra_l, pr, h_l, l, outer_diameter_m),
        ConvectionRegime::Forced => {
            let mu_s = fluid.interpolate(t_surface_k)?.mu_pa_s;
            forced_convection_nusselt(geom, re_l, pr, props.mu_pa_s, mu_s)
        }
    };

    // Eq. (13), (28), (30), (33), (35): hc calculation
    let hc = match (regime, geom) {
        (ConvectionRegime::Free, ConvectionGeometry::ConcentricCylinders) => {
            let d_o = outer_diameter_m.unwrap_or(l * 2.0 + 0.1).max(2.0 * l + 1e-4);
            let d_i = (d_o - 2.0 * l).max(1e-4);
            // Eqs. (28)-(30): hc = k_eff / (ln(Do/Di) * Di/2) where nusselt holds k_eff / k
            (nusselt * props.k_w_m_k / ((d_o / d_i).ln() * 0.5 * d_i)).max(0.0)
        }
        (ConvectionRegime::Free, ConvectionGeometry::ConcentricSpheres) => {
            let d_o = outer_diameter_m.unwrap_or(l * 2.0 + 0.1).max(2.0 * l + 1e-4);
            let d_i = (d_o - 2.0 * l).max(1e-4);
            // Eqs. (33)-(35): hc = k_eff * (Do / Di) / L (or k_eff * pi * Di * Do / L per unit inner area)
            (nusselt * props.k_w_m_k * (d_o / d_i) / l).max(0.0)
        }
        _ => (nusselt * props.k_w_m_k / l).max(0.0),
    };

    Ok(ConvectionDiagnostics {
        char_length_m: l,
        aspect_ratio_h_l: h_l,
        t_film_k: t_f,
        nusselt,
        rayleigh: ra_l,
        grashof: gr_l,
        reynolds: re_l,
        peclet_re_pr: pe_l,
        hc_w_m2_k: hc,
    })
}

fn free_convection_nusselt(
    geom: ConvectionGeometry,
    ra: f64,
    pr: f64,
    h_l: f64,
    l: f64,
    outer_diameter_m: Option<f64>,
) -> f64 {
    if ra <= 1e-12 {
        return match geom {
            ConvectionGeometry::Sphere => 2.0,
            ConvectionGeometry::HorizontalEnclosure | ConvectionGeometry::VerticalEnclosure => 1.0,
            _ => 0.68,
        };
    }

    match geom {
        ConvectionGeometry::VerticalWall
        | ConvectionGeometry::VerticalCylinder
        | ConvectionGeometry::Cube => {
            if ra < 1e9 {
                // Eq. (17) laminar Churchill-Chu
                0.68 + (0.670 * ra.powf(0.25))
                    / (1.0 + (0.492 / pr).powf(9.0 / 16.0)).powf(4.0 / 9.0)
            } else {
                // Eq. (18) turbulent Churchill-Chu
                let term = 0.825
                    + (0.387 * ra.powf(1.0 / 6.0))
                        / (1.0 + (0.492 / pr).powf(9.0 / 16.0)).powf(8.0 / 27.0);
                term * term
            }
        }
        ConvectionGeometry::VerticalEnclosure => {
            if h_l <= 2.0 {
                // Eq. (19)
                0.18 * ((pr / (0.2 + pr)) * ra).powf(0.29)
            } else if h_l < 10.0 {
                // Eq. (20)
                0.22 * ((pr / (0.2 + pr)) * ra).powf(0.28) * h_l.powf(-0.25)
            } else {
                // Eq. (21)
                0.42 * ra.powf(0.25) * pr.powf(0.012) * h_l.powf(-0.3)
            }
            .max(1.0)
        }
        ConvectionGeometry::HorizontalPlateHotDown => {
            // Eq. (22)
            0.27 * ra.powf(0.25)
        }
        ConvectionGeometry::HorizontalPlateHotUp => {
            if ra <= 1e7 {
                // Eq. (23)
                0.54 * ra.powf(0.25)
            } else {
                // Eq. (24)
                0.15 * ra.powf(1.0 / 3.0)
            }
        }
        ConvectionGeometry::HorizontalEnclosure => {
            if ra <= 1708.0 {
                // Eq. (25a) pure conduction
                1.0
            } else {
                // Eq. (25b)
                0.069 * ra.powf(1.0 / 3.0) * pr.powf(0.074)
            }
        }
        ConvectionGeometry::HorizontalCylinder => {
            // Eq. (26) Churchill-Chu
            let term = 0.60
                + (0.387 * ra.powf(1.0 / 6.0))
                    / (1.0 + (0.559 / pr).powf(9.0 / 16.0)).powf(8.0 / 27.0);
            term * term
        }
        ConvectionGeometry::ConcentricCylinders => {
            // Eqs. (27)-(30) Raithby-Hollands k_eff/k
            let d_o = outer_diameter_m.unwrap_or(l * 2.0 + 0.1).max(2.0 * l + 1e-4);
            let d_i = (d_o - 2.0 * l).max(1e-4);
            let ra_c_star = (d_o / d_i).ln().powi(4) * ra
                / (l.powi(3) * (d_i.powf(-0.6) + d_o.powf(-0.6)).powi(5));
            if ra_c_star < 100.0 {
                // Eq. (28): pure conduction (k_eff = k)
                1.0
            } else {
                // Eq. (29)-(30)
                let k_eff_k =
                    0.386 * (pr / (0.861 + pr)).powf(0.25) * ra_c_star.powf(0.25);
                k_eff_k.max(1.0)
            }
        }
        ConvectionGeometry::Sphere => {
            // Eq. (31) Churchill
            2.0 + (0.589 * ra.powf(0.25))
                / (1.0 + (0.469 / pr).powf(9.0 / 16.0)).powf(4.0 / 9.0)
        }
        ConvectionGeometry::ConcentricSpheres => {
            // Eqs. (32)-(35) Raithby-Hollands k_eff/k
            let d_o = outer_diameter_m.unwrap_or(l * 2.0 + 0.1).max(2.0 * l + 1e-4);
            let d_i = (d_o - 2.0 * l).max(1e-4);
            let ra_s_star = (l * ra)
                / ((d_i * d_o).powi(4) * (d_i.powf(-1.4) + d_o.powf(-1.4)).powi(5));
            if ra_s_star < 100.0 {
                // Eq. (33): pure conduction (k_eff = k)
                1.0
            } else {
                // Eq. (34)-(35)
                let k_eff_k =
                    0.74 * (pr / (0.861 + pr)).powf(0.25) * ra_s_star.powf(0.25);
                k_eff_k.max(1.0)
            }
        }
    }
}

fn forced_convection_nusselt(
    geom: ConvectionGeometry,
    re: f64,
    pr: f64,
    mu: f64,
    mu_s: f64,
) -> f64 {
    if re <= 1e-12 {
        return match geom {
            ConvectionGeometry::Sphere => 2.0,
            _ => 0.3,
        };
    }

    match geom {
        ConvectionGeometry::HorizontalCylinder | ConvectionGeometry::VerticalCylinder => {
            // Eq. (39) Churchill-Bernstein
            0.3 + (0.62 * re.sqrt() * pr.powf(1.0 / 3.0))
                / (1.0 + (0.4 / pr).powf(2.0 / 3.0)).powf(0.25)
                * (1.0 + (re / 282_000.0).powf(5.0 / 8.0)).powf(0.8)
        }
        ConvectionGeometry::Sphere => {
            // Eq. (40) Whitaker
            let vis_ratio = (mu / mu_s.max(1e-12)).powf(0.25);
            2.0 + (0.4 * re.sqrt() + 0.06 * re.powf(2.0 / 3.0)) * pr.powf(0.4) * vis_ratio
        }
        _ => {
            // Eqs. (37)-(38) Flat plate / Cube / Wall
            if re <= 5e5 {
                0.664 * re.sqrt() * pr.powf(1.0 / 3.0)
            } else {
                (0.037 * re.powf(0.8) - 871.0).max(0.0) * pr.powf(1.0 / 3.0)
            }
        }
    }
}
