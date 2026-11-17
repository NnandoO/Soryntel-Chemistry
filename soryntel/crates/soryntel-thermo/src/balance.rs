//! Module 2 Thermodynamic Extensions: Heat Content, Total Enthalpy, Exergy,
//! and H2O Pressure Correction (IAPWS-IF97 saturation + phase selection).

use crate::{evaluate_species_si, ChemicalSpecies, Phase, SpeciesRepository};
use soryntel_core::{BalanceEnergyUnit, SoryntelError, CAL_TO_JOULE, R_JOULE, T_REF_K};
use soryntel_solver::parse_term;
use std::collections::BTreeMap;

/// HSC Manual Ch. 11.1 Eq. (1): First estimate of heat losses for an air-cooled reactor
/// (natural convection) in kcal/h, converted to the workbook's `BalanceEnergyUnit` (per hour basis):
/// `H_loss [kcal/h] = (6.8 + 0.046 * T2) * (T2 - T1) * A`
/// where `A` = outer surface area (m²), `T2` = surface temperature (°C), `T1` = room temperature (°C).
pub fn estimate_air_cooled_reactor_heat_loss(
    area_m2: f64,
    t_surface_c: f64,
    t_room_c: f64,
    unit: BalanceEnergyUnit,
) -> f64 {
    if area_m2 <= 0.0 || t_surface_c <= t_room_c {
        return 0.0;
    }
    let h_loss_kcal = (6.8 + 0.046 * t_surface_c) * (t_surface_c - t_room_c) * area_m2;
    let h_loss_mj = (h_loss_kcal / 1000.0) * CAL_TO_JOULE;
    h_loss_mj * unit.from_mj_factor()
}

/// Parse elemental composition of an HSC species formula (stripping phase/isomer suffixes like `(g)`, `(l)`, `(NBAg)`).
pub fn parse_hsc_species_elements(formula: &str) -> Result<BTreeMap<String, f64>, SoryntelError> {
    let trimmed = formula.trim();
    // Strip HSC isomer/phase suffixes like (NBAg), (IBAg) if not handled by parse_term
    let cleaned = if let Some(idx) = trimmed.rfind('(') {
        let suffix = &trimmed[idx..];
        if suffix.ends_with(')') && (suffix.contains("BA") || suffix == "(g)" || suffix == "(l)" || suffix == "(s)" || suffix == "(a)") {
            &trimmed[..idx]
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    parse_term(cleaned).map(|t| t.elements)
}

/// Standard ambient dead state pressure (1 atm = 1.01325 bar).
pub const P_REF_BAR: f64 = 1.01325;

/// Critical point of H2O (IAPWS-95 / IF97).
pub const H2O_TC_K: f64 = 647.096;
pub const H2O_PC_BAR: f64 = 220.64;

/// Molar thermodynamic properties for Module 2 in MJ/kmol (== kJ/mol).
#[derive(Debug, Clone, Copy, Default)]
pub struct BalancePointSI {
    pub t_k: f64,
    /// Sensible + latent heat above 298.15 K (MJ/kmol == kJ/mol).
    pub h_cont_mj_kmol: f64,
    /// Total enthalpy: Hf°(298.15 K) + Heat Content (MJ/kmol == kJ/mol).
    pub h_tot_mj_kmol: f64,
    /// T * S(T) (MJ/kmol).
    pub ts_mj_kmol: f64,
    /// Gibbs free energy H(T) - T * S(T) (MJ/kmol).
    pub g_mj_kmol: f64,
    /// Physical exergy relative to (298.15 K, 1.01325 bar) (MJ/kmol).
    pub ex_phys_mj_kmol: f64,
    /// Standard chemical exergy at (298.15 K, 1.01325 bar) (MJ/kmol).
    pub ex_chem_mj_kmol: f64,
    /// Total exergy = Ex_phys + Ex_chem (MJ/kmol).
    pub ex_tot_mj_kmol: f64,
}

/// IAPWS-95 Wagner & Pruss saturation vapor pressure of water P_sat(T) in bar.
pub fn h2o_saturation_pressure_bar(t_k: f64) -> f64 {
    if t_k <= 273.16 {
        return 0.00611657;
    }
    if t_k >= H2O_TC_K {
        return H2O_PC_BAR;
    }
    let tau = 1.0 - t_k / H2O_TC_K;
    let a1 = -7.85951783;
    let a2 = 1.84408259;
    let a3 = -11.7866497;
    let a4 = 22.6807411;
    let a5 = -15.9618719;
    let a6 = 1.80122502;
    let ln_pr = (H2O_TC_K / t_k)
        * (a1 * tau
            + a2 * tau.powf(1.5)
            + a3 * tau.powi(3)
            + a4 * tau.powf(3.5)
            + a5 * tau.powi(4)
            + a6 * tau.powf(7.5));
    H2O_PC_BAR * ln_pr.exp()
}

/// Saturation temperature T_sat(P) in K via bisection on `h2o_saturation_pressure_bar`.
pub fn h2o_saturation_temp_k(p_bar: f64) -> f64 {
    if p_bar <= 0.00611657 {
        return 273.16;
    }
    if p_bar >= H2O_PC_BAR {
        return H2O_TC_K;
    }
    let mut lo = 273.16;
    let mut hi = H2O_TC_K;
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if h2o_saturation_pressure_bar(mid) < p_bar {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Standard chemical exergy of elements (kJ/mol == MJ/kmol) at T0=298.15 K, P0=1.01325 bar
/// based on Szargut, Morris & Steward (1988).
pub fn element_standard_chemical_exergy_kj_mol(symbol: &str) -> Option<f64> {
    let val = match symbol {
        "C" => 410.26,
        "H" => 118.05, // 0.5 * 236.10 (H2(g))
        "O" => 1.985,  // 0.5 * 3.97 (O2(g))
        "N" => 0.36,   // 0.5 * 0.72 (N2(g))
        "S" => 609.60,
        "Fe" => 376.40,
        "Cu" => 134.20,
        "Si" => 854.60,
        "Al" => 888.40,
        "Ca" => 712.40,
        "Mg" => 633.80,
        "Zn" => 339.20,
        "Pb" => 232.80,
        "Ni" => 232.70,
        "Na" => 336.60,
        "K" => 366.60,
        "Cl" => 61.80, // 0.5 * 123.60 (Cl2(g))
        "F" => 233.15, // 0.5 * 466.30 (F2(g))
        "Br" => 50.60, // 0.5 * 101.20 (Br2(l))
        "I" => 87.35,  // 0.5 * 174.70 (I2(s))
        "P" => 875.80,
        "Mn" => 482.30,
        "Cr" => 544.30,
        "Ti" => 906.90,
        "Sn" => 544.80,
        "Co" => 265.00,
        "Ag" => 70.20,
        "Au" => 15.40,
        "As" => 494.60,
        "Sb" => 435.80,
        "Bi" => 274.50,
        "Mo" => 730.30,
        "W" => 827.50,
        "V" => 721.10,
        "B" => 628.50,
        "Ba" => 747.70,
        "Sr" => 730.20,
        "Li" => 393.00,
        "Be" => 604.30,
        "Cd" => 293.20,
        "Hg" => 115.90,
        "Zr" => 1082.90,
        "Pt" => 141.00,
        "Pd" => 138.70,
        _ => return None,
    };
    Some(val)
}

/// Compute standard chemical exergy of a species (MJ/kmol == kJ/mol):
/// ex_chem = dGf°(298.15) + sum_e nu_e * ex_chem,e°.
pub fn species_chemical_exergy_kj_mol(
    repo: &dyn SpeciesRepository,
    species: &ChemicalSpecies,
) -> f64 {
    let Ok((pt_ref, _)) = evaluate_species_si(species, T_REF_K, false, false) else {
        return 0.0;
    };
    let Ok(elems) = parse_hsc_species_elements(&species.formula) else {
        return 0.0;
    };
    if elems.is_empty() {
        return 0.0;
    }

    // Compute dGf°(298.15) = G_species(298.15) - sum_e nu_e * G_ref_elem(298.15)
    let mut sum_g_ref = 0.0;
    let mut sum_ex_elem = 0.0;
    for (el, nu) in elems {
        let Some(ex_e) = element_standard_chemical_exergy_kj_mol(&el) else {
            return 0.0;
        };
        sum_ex_elem += nu * ex_e;
        if let Some(ref_sp) = repo.get_reference_element(&el) {
            // Count atoms of `el` in reference species (e.g., O2(g) has 2 O atoms)
            let ref_atoms = parse_hsc_species_elements(&ref_sp.formula)
                .ok()
                .and_then(|m| m.get(&el).copied())
                .unwrap_or(1.0);
            if let Ok((ref_pt, _)) = evaluate_species_si(&ref_sp, T_REF_K, false, false) {
                sum_g_ref += (nu / ref_atoms) * (ref_pt.g / 1000.0);
            }
        }
    }
    let dg_f_kj = (pt_ref.g / 1000.0) - sum_g_ref;
    dg_f_kj + sum_ex_elem
}

/// Evaluate Heat Content, Total H, T*S, G, Physical Exergy and Total Exergy for a species at (t_k, p_bar).
pub fn evaluate_balance_point_si(
    repo: &dyn SpeciesRepository,
    species: &ChemicalSpecies,
    t_k: f64,
    p_bar: f64,
    h2o_pressure_corr: bool,
    upper_side_on_boundary: bool,
) -> Result<BalancePointSI, SoryntelError> {
    // Handle H2O pressure correction if requested
    let mut effective_species = species.clone();
    let mut pressure_dh_j = 0.0;
    let mut pressure_ds_j = 0.0;

    if (h2o_pressure_corr || (p_bar - 1.0).abs() > 1e-4)
        && (species.formula == "H2O"
            || species.formula == "H2O(l)"
            || species.formula == "H2O(g)")
        && p_bar > 0.0
    {
        let t_sat = h2o_saturation_temp_k(p_bar);
        let is_vapor = t_k >= t_sat - 1e-4;
        if is_vapor {
            if let Some(steam) = repo.get_species("H2O(g)") {
                effective_species = steam;
            }
            // Real-gas / pressure entropy shift -R*ln(P/P_ref)
            pressure_ds_j = -R_JOULE * (p_bar / P_REF_BAR).ln();
        } else {
            if let Some(liq) = repo.get_species("H2O").or_else(|| repo.get_species("H2O(l)")) {
                effective_species = liq;
            }
            // Poynting incompressible liquid correction: V_m * (P - P_ref)
            let v_molar_m3 = 18.015e-6; // m3/mol
            pressure_dh_j = v_molar_m3 * (p_bar - P_REF_BAR) * 1.0e5;
        }
    } else if effective_species.phase == Phase::Gas && p_bar > 0.0 && (p_bar - 1.0).abs() > 1e-4 {
        pressure_ds_j = -R_JOULE * (p_bar / 1.0).ln();
    }

    let (pt_t, _) = evaluate_species_si(&effective_species, t_k, false, upper_side_on_boundary)?;
    let (pt_ref, _) = evaluate_species_si(&effective_species, T_REF_K, false, false)?;

    let h_t = pt_t.h + pressure_dh_j;
    let s_t = pt_t.s + pressure_ds_j;

    // For H2O(g) when pressure correction is off, Heat Content is relative to H2O(g) at 298.15 K
    // (consistent with HSC Heat Balance: H(T) - H(298.15) of the same species).
    let h_cont = h_t - pt_ref.h;
    let ts = t_k * s_t;
    let g = h_t - ts;

    // Physical exergy: (H(T,P) - H(T0,P0)) - T0 * (S(T,P) - S(T0,P0))
    let ds_pressure_gas = if effective_species.phase == Phase::Gas && p_bar > 0.0 && !h2o_pressure_corr
    {
        -R_JOULE * (p_bar / P_REF_BAR).ln()
    } else {
        0.0
    };
    let ex_phys = (h_t - pt_ref.h) - T_REF_K * ((s_t + ds_pressure_gas) - pt_ref.s);
    let ex_chem_kj = species_chemical_exergy_kj_mol(repo, &effective_species);

    Ok(BalancePointSI {
        t_k,
        h_cont_mj_kmol: h_cont / 1000.0,
        h_tot_mj_kmol: h_t / 1000.0,
        ts_mj_kmol: ts / 1000.0,
        g_mj_kmol: g / 1000.0,
        ex_phys_mj_kmol: ex_phys / 1000.0,
        ex_chem_mj_kmol: ex_chem_kj,
        ex_tot_mj_kmol: ex_phys / 1000.0 + ex_chem_kj,
    })
}
