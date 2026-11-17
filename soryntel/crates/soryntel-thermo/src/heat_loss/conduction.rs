//! 1D steady-state conduction across solid layers (Section 12.4.1, Eqs. 8-12).

use super::model::{Geometry, SolidConductivityData};
use soryntel_core::SoryntelError;

/// Result of evaluating a solid layer between `t_left_k` and `t_right_k`.
#[derive(Debug, Clone)]
pub struct ConductionResult {
    pub resistance_k_w: f64,
    pub k_avg_w_m_k: f64,
    /// Discretized `(x_m, T_K)` profile from `d_in` to `d_in + thickness_m`.
    pub profile_points: Vec<(f64, f64)>,
}

/// Evaluate thermal resistance and discretized temperature profile across a solid layer
/// using `grid_steps` sublayers (Eq. 9) and geometry shape factor `S` (Eqs. 10-12).
pub fn evaluate_solid_layer(
    geometry: &Geometry,
    d_in_m: f64,
    thickness_m: f64,
    grid_steps: usize,
    t_left_k: f64,
    t_right_k: f64,
    k_fixed: bool,
    k_fixed_val: f64,
    solid_data: Option<&SolidConductivityData>,
) -> Result<ConductionResult, SoryntelError> {
    if thickness_m <= 0.0 {
        return Err(SoryntelError::HeatLossError(
            "Solid layer thickness must be > 0".into(),
        ));
    }
    let m = grid_steps.max(1);
    let dx = thickness_m / (m as f64);

    // Precompute shape factors S_i for each sublayer
    let mut shape_factors = Vec::with_capacity(m);
    for i in 0..m {
        let d0 = d_in_m + (i as f64) * dx;
        let d1 = d_in_m + ((i + 1) as f64) * dx;
        shape_factors.push(geometry.conduction_shape_factor(d0, d1)?);
    }
    let s_total = geometry.conduction_shape_factor(d_in_m, d_in_m + thickness_m)?;

    // Fast path when conductivity is constant / fixed
    if k_fixed || solid_data.is_none() {
        if k_fixed_val <= 0.0 {
            return Err(SoryntelError::HeatLossError(
                "Thermal conductivity k must be > 0".into(),
            ));
        }
        let r_total = 1.0 / (s_total * k_fixed_val);
        let q = (t_left_k - t_right_k) / r_total;
        let mut profile = Vec::with_capacity(m + 1);
        let mut t_cur = t_left_k;
        profile.push((d_in_m, t_cur));
        for (i, &s_i) in shape_factors.iter().enumerate() {
            t_cur -= q / (s_i * k_fixed_val);
            profile.push((d_in_m + ((i + 1) as f64) * dx, t_cur));
        }
        return Ok(ConductionResult {
            resistance_k_w: r_total,
            k_avg_w_m_k: k_fixed_val,
            profile_points: profile,
        });
    }

    let data = solid_data.unwrap();
    // Sub-iteration for temperature-dependent k(T) over the m sublayers (Eq. 9)
    let mut temps = Vec::with_capacity(m + 1);
    for i in 0..=m {
        let frac = (i as f64) / (m as f64);
        temps.push(t_left_k + frac * (t_right_k - t_left_k));
    }

    let mut r_total = 0.0;
    for _iter in 0..25 {
        let mut r_sub = Vec::with_capacity(m);
        r_total = 0.0;
        for i in 0..m {
            let t_mid = 0.5 * (temps[i] + temps[i + 1]);
            let k_i = data.conductivity_at(t_mid)?.max(1e-9);
            let r_i = 1.0 / (shape_factors[i] * k_i);
            r_total += r_i;
            r_sub.push(r_i);
        }
        let q = (t_left_k - t_right_k) / r_total;
        let mut max_delta = 0.0_f64;
        let mut t_cur = t_left_k;
        for i in 0..m {
            t_cur -= q * r_sub[i];
            if i + 1 < m {
                max_delta = max_delta.max((t_cur - temps[i + 1]).abs());
                temps[i + 1] = 0.7 * t_cur + 0.3 * temps[i + 1];
            }
        }
        if max_delta < 1e-6 {
            break;
        }
    }

    let k_avg = 1.0 / (s_total * r_total);
    let profile_points = (0..=m)
        .map(|i| (d_in_m + (i as f64) * dx, temps[i]))
        .collect();

    Ok(ConductionResult {
        resistance_k_w: r_total,
        k_avg_w_m_k: k_avg,
        profile_points,
    })
}
