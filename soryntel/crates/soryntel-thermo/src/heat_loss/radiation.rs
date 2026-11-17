//! Surface and enclosure gray diffuse radiation (Section 12.4.3, Eqs. 41-43).

use super::model::Geometry;
use soryntel_core::{SoryntelError, STEFAN_BOLTZMANN};

/// Validate surface emissivity is within `[0.0, 1.0]`.
pub fn validate_emissivity(eps: f64) -> Result<f64, SoryntelError> {
    if !(0.0..=1.0).contains(&eps) || !eps.is_finite() {
        return Err(SoryntelError::HeatLossError(
            "Please enter a surface emissivity value between 0 and 1.".into(),
        ));
    }
    Ok(eps)
}

/// Linearized gray diffuse surface radiation coefficient `hr` [W/(m^2·K)] (Eq. 42)
/// between a surface at `t_s_k` (emissivity `eps_s`) and large surroundings at `t_inf_k`.
pub fn surface_radiation_hr(
    eps_s: f64,
    t_s_k: f64,
    t_inf_k: f64,
) -> Result<f64, SoryntelError> {
    let eps = validate_emissivity(eps_s)?;
    Ok(eps
        * STEFAN_BOLTZMANN
        * (t_s_k + t_inf_k)
        * (t_s_k * t_s_k + t_inf_k * t_inf_k))
}

/// Radiative heat flux `q''` [W/m^2] from surface `t_s_k` to surroundings `t_inf_k` (Eq. 41).
pub fn surface_radiation_flux(
    eps_s: f64,
    t_s_k: f64,
    t_inf_k: f64,
) -> Result<f64, SoryntelError> {
    let eps = validate_emissivity(eps_s)?;
    Ok(eps * STEFAN_BOLTZMANN * (t_s_k.powi(4) - t_inf_k.powi(4)))
}

/// Effective emissivity and linearized `hr` [W/(m^2·K)] for an enclosure between two
/// diffuse gray walls with emissivities `eps_1`, `eps_2` (Eq. 43).
pub fn enclosure_radiation_hr(
    geometry: &Geometry,
    d_in_m: f64,
    d_out_m: f64,
    eps_1: f64,
    eps_2: f64,
    t_1_k: f64,
    t_2_k: f64,
) -> Result<f64, SoryntelError> {
    let e1 = validate_emissivity(eps_1)?.max(1e-6);
    let e2 = validate_emissivity(eps_2)?.max(1e-6);

    let eps_eff = match *geometry {
        Geometry::Wall { .. } | Geometry::Cube { .. } => 1.0 / (1.0 / e1 + 1.0 / e2 - 1.0),
        Geometry::Cylinder { diameter_m, .. } => {
            let r1 = 0.5 * diameter_m + d_in_m;
            let r2 = (0.5 * diameter_m + d_out_m).max(r1);
            1.0 / (1.0 / e1 + ((1.0 - e2) / e2) * (r1 / r2))
        }
        Geometry::Sphere { diameter_m } => {
            let r1 = 0.5 * diameter_m + d_in_m;
            let r2 = (0.5 * diameter_m + d_out_m).max(r1);
            let ratio = r1 / r2;
            1.0 / (1.0 / e1 + ((1.0 - e2) / e2) * ratio * ratio)
        }
    };

    Ok(eps_eff * STEFAN_BOLTZMANN * (t_1_k + t_2_k) * (t_1_k * t_1_k + t_2_k * t_2_k))
}
