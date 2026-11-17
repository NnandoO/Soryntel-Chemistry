//! Soryntel Thermo: Kelley polynomial integration, phase transitions, Criss-Cobble,
//! chemical reaction equilibrium / electrochemical potential, and Module 2 Heat & Material Balance.

pub mod aqua;
pub mod aqua_density_data;
pub mod aqua_pitzer_data;
pub mod balance;
pub mod balance_engine;
pub mod benson;
pub mod diagrams;
pub mod ehph;
pub mod elements;
pub mod equilibrium;
pub mod estimate;
pub mod excel_addins;
pub mod exergy;
pub mod heat_loss;
mod minerals_data;
pub mod phase_stability;
pub mod sim;
pub mod species;
pub mod species_converter;
pub mod spreadsheet_export;
pub mod units;
mod units_data;
pub mod water;

use serde::{Deserialize, Serialize};
use soryntel_core::{
    EnergyUnit, ResultFormat, SoryntelError, TemperatureUnit, CAL_TO_JOULE, FARADAY_C_MOL,
    R_JOULE, STP_MOLAR_VOL_L, T_REF_K,
};
use soryntel_solver::{parse_input, verify_balance, ParsedReaction, ParsedTerm};
use std::collections::BTreeSet;

/// Physical phase of aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    AqueousIon,
    AqueousNeutral,
}

/// Kelley heat-capacity coefficients in SI units: J/(mol·K).
/// Cp(T) = A + B*1e-3*T + C*1e5*T^-2 + D*1e-6*T^2
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KelleyCoefficients {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

impl KelleyCoefficients {
    pub fn cp(&self, t_k: f64) -> f64 {
        self.a
            + self.b * 1e-3 * t_k
            + self.c * 1e5 / (t_k * t_k)
            + self.d * 1e-6 * (t_k * t_k)
    }

    pub fn integrate_h(&self, t1: f64, t2: f64) -> f64 {
        self.a * (t2 - t1)
            + 0.5 * self.b * 1e-3 * (t2 * t2 - t1 * t1)
            - self.c * 1e5 * (1.0 / t2 - 1.0 / t1)
            + (self.d * 1e-6 / 3.0) * (t2 * t2 * t2 - t1 * t1 * t1)
    }

    pub fn integrate_s(&self, t1: f64, t2: f64) -> f64 {
        self.a * (t2 / t1).ln()
            + self.b * 1e-3 * (t2 - t1)
            - 0.5 * self.c * 1e5 * (1.0 / (t2 * t2) - 1.0 / (t1 * t1))
            + 0.5 * self.d * 1e-6 * (t2 * t2 - t1 * t1)
    }

    pub fn has_temp_terms(&self) -> bool {
        self.b.abs() > 1e-12 || self.c.abs() > 1e-12 || self.d.abs() > 1e-12
    }
}

/// Continuous thermodynamic interval `[t_min, t_max]` in Kelvin, stored in SI (J/mol, J/(mol·K)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesThermoInterval {
    pub t_min: f64,
    pub t_max: f64,
    pub phase: Phase,
    pub kelley: KelleyCoefficients,
    /// For k = 0: Hf°(298.15 K) in J/mol. For k >= 1: ΔH_tr at t_min in J/mol.
    pub h_or_delta_h_tr: f64,
    /// For k = 0: S°(298.15 K) in J/(mol·K). For k >= 1: ΔS_tr at t_min in J/(mol·K).
    pub s_or_delta_s_tr: f64,
    pub reference: String,
}

/// Complete chemical species record in SI units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalSpecies {
    pub formula: String,
    pub name: String,
    pub phase: Phase,
    pub molecular_weight: f64,
    pub density_g_cm3: f64,
    pub charge: i32,
    pub intervals: Vec<SpeciesThermoInterval>,
}

/// SI thermodynamic state evaluated at a single temperature.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermoPointSI {
    pub t_k: f64,
    pub cp: f64, // J/(mol·K)
    pub h: f64,  // J/mol
    pub s: f64,  // J/(mol·K)
    pub g: f64,  // J/mol
    pub extrapolated_from_k: Option<f64>,
}

/// Trait for looking up species and reference elements.
pub trait SpeciesRepository {
    fn get_species(&self, formula: &str) -> Option<ChemicalSpecies>;
    /// Lookup reference element species for Delta (formation) mode (e.g., "Fe", "O2(g)", "H2(g)", "H(+a)").
    fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies>;
}

/// Optional extended species metadata stored in `.HSC` / `OwnDB` records (fields from HSC6 `28.11`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpeciesMetadata {
    pub structural_formula: String,
    pub common_name: String,
    pub cas_number: String,
    /// Melting point in Kelvin (`0.0` if unset).
    pub melting_point_k: f64,
    /// Boiling point in Kelvin (`0.0` if unset).
    pub boiling_point_k: f64,
    /// HSC color palette index `0..53`.
    pub color_code: i16,
    /// Aqueous solubility string or note (`"-"` when unavailable).
    pub solubility: String,
    /// Literature reliability class (`1..10`, `0` = unclassified).
    pub reliability_class: i16,
}

/// Outbound Hexagonal Port for searching, listing, and persisting species in any catalog adapter.
pub trait SpeciesCatalogPort: SpeciesRepository {
    fn search_species(&self, query: &str, limit: usize) -> Vec<ChemicalSpecies>;
    fn search_species_by_elements(
        &self,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies>;
    fn insert_or_update_species(&mut self, sp: ChemicalSpecies);
    fn species_count(&self) -> usize;

    /// Lookup extended physical/bibliographic metadata for `formula`, if stored by the backend.
    fn get_species_metadata(&self, _formula: &str) -> Option<SpeciesMetadata> {
        None
    }

    /// Lookup species formula by 0-based database insertion/catalog position (`O(1)` when indexed).
    fn get_species_at(&self, _index: usize) -> Option<ChemicalSpecies> {
        None
    }
}

/// Evaluate a species in SI units at `t_k` (K) using exact piecewise Kelley integration from 298.15 K.
/// If `side` is `Some(false)` at a transition boundary `t_min`, evaluates the lower phase; `Some(true)` evaluates the upper phase.
pub fn evaluate_species_si(
    species: &ChemicalSpecies,
    t_k: f64,
    criss_cobble: bool,
    upper_side_on_boundary: bool,
) -> Result<(ThermoPointSI, String), SoryntelError> {
    if t_k <= 0.0 || !t_k.is_finite() {
        return Err(SoryntelError::InvalidTemperature(t_k));
    }
    let recs = &species.intervals;
    if recs.is_empty() {
        return Err(SoryntelError::SpeciesNotFound(species.formula.clone()));
    }

    // Criss-Cobble extrapolation for aqueous ions without B, C, D coefficients above 298.15 K
    if criss_cobble
        && species.phase == Phase::AqueousIon
        && recs.len() == 1
        && !recs[0].kelley.has_temp_terms()
        && t_k > T_REF_K
    {
        return Ok((criss_cobble_eval(species, t_k), recs[0].reference.clone()));
    }

    let idx_ref = recs
        .iter()
        .position(|r| r.t_min <= T_REF_K + 1e-4 && T_REF_K - 1e-4 <= r.t_max)
        .unwrap_or(0);

    let mut extrapolated_from = None;
    let idx_t = if t_k < recs[0].t_min - 1e-6 {
        extrapolated_from = Some(recs[0].t_min);
        0
    } else if t_k > recs[recs.len() - 1].t_max + 1e-6 {
        extrapolated_from = Some(recs[recs.len() - 1].t_max);
        recs.len() - 1
    } else if upper_side_on_boundary {
        recs.iter()
            .rposition(|r| r.t_min - 1e-6 <= t_k && t_k <= r.t_max + 1e-6)
            .unwrap_or(recs.len() - 1)
    } else {
        recs.iter()
            .position(|r| r.t_min - 1e-6 <= t_k && t_k <= r.t_max + 1e-6)
            .unwrap_or(recs.len() - 1)
    };

    let mut h = recs[0].h_or_delta_h_tr;
    let mut s = recs[0].s_or_delta_s_tr;

    if idx_t == idx_ref {
        h += recs[idx_ref].kelley.integrate_h(T_REF_K, t_k);
        s += recs[idx_ref].kelley.integrate_s(T_REF_K, t_k);
    } else if idx_t > idx_ref {
        h += recs[idx_ref]
            .kelley
            .integrate_h(T_REF_K, recs[idx_ref].t_max);
        s += recs[idx_ref]
            .kelley
            .integrate_s(T_REF_K, recs[idx_ref].t_max);
        for k in (idx_ref + 1)..idx_t {
            h += recs[k].h_or_delta_h_tr + recs[k].kelley.integrate_h(recs[k].t_min, recs[k].t_max);
            s += recs[k].s_or_delta_s_tr + recs[k].kelley.integrate_s(recs[k].t_min, recs[k].t_max);
        }
        h += recs[idx_t].h_or_delta_h_tr + recs[idx_t].kelley.integrate_h(recs[idx_t].t_min, t_k);
        s += recs[idx_t].s_or_delta_s_tr + recs[idx_t].kelley.integrate_s(recs[idx_t].t_min, t_k);
    } else {
        // Integrate downward below 298.15 K
        h -= recs[idx_ref]
            .kelley
            .integrate_h(recs[idx_ref].t_min, T_REF_K);
        s -= recs[idx_ref]
            .kelley
            .integrate_s(recs[idx_ref].t_min, T_REF_K);
        for k in ((idx_t + 1)..idx_ref).rev() {
            h -= recs[k + 1].h_or_delta_h_tr
                + recs[k].kelley.integrate_h(recs[k].t_min, recs[k].t_max);
            s -= recs[k + 1].s_or_delta_s_tr
                + recs[k].kelley.integrate_s(recs[k].t_min, recs[k].t_max);
        }
        h -= recs[idx_t + 1].h_or_delta_h_tr
            + recs[idx_t].kelley.integrate_h(t_k, recs[idx_t].t_max);
        s -= recs[idx_t + 1].s_or_delta_s_tr
            + recs[idx_t].kelley.integrate_s(t_k, recs[idx_t].t_max);
    }

    let cp = recs[idx_t].kelley.cp(t_k);
    let g = h - t_k * s;
    Ok((
        ThermoPointSI {
            t_k,
            cp,
            h,
            s,
            g,
            extrapolated_from_k: extrapolated_from,
        },
        recs[idx_t].reference.clone(),
    ))
}

/// Criss-Cobble (1964) correspondence principle extrapolation for aqueous ions up to 300 °C (573.15 K).
fn criss_cobble_eval(species: &ChemicalSpecies, t_k: f64) -> ThermoPointSI {
    let rec = &species.intervals[0];
    let h_298 = rec.h_or_delta_h_tr;
    let s_298_conv = rec.s_or_delta_s_tr;
    let z = species.charge as f64;

    // Convert to absolute entropy scale: S_abs(H+) = -5.0 cal/(mol·K) = -20.92 J/(mol·K)
    let s_abs_298 = s_298_conv - 5.0 * CAL_TO_JOULE * z;
    let t_c = (t_k - 273.15).clamp(25.0, 300.0);
    let theta = (t_c - 25.0) / 275.0;

    // Linearly interpolated Criss-Cobble correspondence parameters (1964) for simple ions
    let (a_t, b_t) = if z >= 0.0 {
        // Simple cations: a(T) in cal/(mol·K) converted to J/(mol·K), b(T) dimensionless
        (41.8 * theta * CAL_TO_JOULE, 1.0 - 0.56 * theta)
    } else {
        // Simple anions / oxyanions
        (-49.0 * theta * CAL_TO_JOULE, 1.0 - 0.65 * theta)
    };

    let s_abs_t = a_t + b_t * s_abs_298;
    let s_conv_t = s_abs_t + 5.0 * CAL_TO_JOULE * z;
    let ln_ratio = (t_k / T_REF_K).ln();
    let cp_avg = if ln_ratio.abs() > 1e-9 {
        (s_conv_t - s_298_conv) / ln_ratio
    } else {
        rec.kelley.cp(T_REF_K)
    };

    let h_t = h_298 + cp_avg * (t_k - T_REF_K);
    let g_t = h_t - t_k * s_conv_t;

    ThermoPointSI {
        t_k,
        cp: cp_avg,
        h: h_t,
        s: s_conv_t,
        g: g_t,
        extrapolated_from_k: if t_k > 573.15 { Some(573.15) } else { None },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationRequest {
    pub input: String,
    pub t_from: f64,
    pub t_to: f64,
    pub t_step: f64,
    pub temp_unit: TemperatureUnit,
    pub energy_unit: EnergyUnit,
    pub result_format: ResultFormat,
    pub show_transitions: bool,
    pub criss_cobble: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceRow {
    pub temperature: f64,
    pub cp: f64,
    pub h: f64,
    pub s: f64,
    pub g: f64,
    pub reference: String,
    pub is_transition: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionRow {
    pub temperature: f64,
    pub delta_h: f64,
    pub delta_s: f64,
    pub delta_g: f64,
    pub k_eq: f64,
    pub log10_k: f64,
    pub e_vs_she_v: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDataRow {
    pub formula: String,
    pub side: String, // "Reactant" or "Product"
    pub molar_mass: f64,
    pub conc_wt_pct: f64,
    pub mol: f64,
    pub mass_g: f64,
    pub volume: f64,
    pub volume_unit: String,
    pub extrapolated_from_k: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationResponse {
    pub normalized_input: String,
    pub is_single_substance: bool,
    pub has_electrochemical_potential: bool,
    pub result_format: ResultFormat,
    pub temp_unit_label: String,
    pub energy_unit_label: String,
    pub entropy_unit_label: String,
    pub reaction_entropy_unit_label: String,
    pub substance_rows: Vec<SubstanceRow>,
    pub reaction_rows: Vec<ReactionRow>,
    pub species_data: Vec<SpeciesDataRow>,
}

pub fn calculate(
    req: &CalculationRequest,
    repo: &dyn SpeciesRepository,
) -> Result<CalculationResponse, SoryntelError> {
    if req.t_step <= 0.0 || req.t_to < req.t_from {
        return Err(SoryntelError::InvalidRange {
            from: req.t_from,
            to: req.t_to,
            step: req.t_step,
        });
    }

    let parsed = parse_input(&req.input)?;
    if !parsed.is_single_substance {
        verify_balance(&parsed)?;
    }

    let t_from_k = req.temp_unit.to_kelvin(req.t_from);
    let t_to_k = req.temp_unit.to_kelvin(req.t_to);
    let step_k = req.temp_unit.step_to_kelvin(req.t_step);
    if t_from_k <= 0.0 {
        return Err(SoryntelError::InvalidTemperature(t_from_k));
    }

    // Collect grid temperatures in K
    let mut temps_k = Vec::new();
    let n_steps = ((t_to_k - t_from_k) / step_k + 1e-9).floor() as usize;
    for i in 0..=n_steps {
        temps_k.push(t_from_k + (i as f64) * step_k);
    }

    let e_factor = req.energy_unit.from_joules_factor();
    let s_factor = req.energy_unit.entropy_from_joules_factor();

    if parsed.is_single_substance {
        let term = &parsed.reactants[0];
        let sp = repo
            .get_species(&term.formula)
            .ok_or_else(|| SoryntelError::SpeciesNotFound(term.formula.clone()))?;

        // Build temperature evaluation points (with optional transition splits)
        let mut eval_points: Vec<(f64, bool, bool)> =
            temps_k.iter().map(|&t| (t, false, false)).collect();
        if req.show_transitions {
            let mut tr_set = BTreeSet::new();
            for interval in sp.intervals.iter().skip(1) {
                if interval.t_min >= t_from_k && interval.t_min <= t_to_k {
                    tr_set.insert((interval.t_min * 1000.0).round() as i64);
                }
            }
            for tr_milli in tr_set {
                let t_tr = (tr_milli as f64) / 1000.0;
                eval_points.push((t_tr, false, true));
                eval_points.push((t_tr, true, true));
            }
            eval_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        }

        let mut substance_rows = Vec::with_capacity(eval_points.len());
        let mut max_extrap: Option<f64> = None;

        for (t_k, upper_side, is_tr) in eval_points {
            let (mut pt, ref_str) =
                evaluate_species_si(&sp, t_k, req.criss_cobble, upper_side)?;
            if let Some(ex) = pt.extrapolated_from_k {
                max_extrap = Some(max_extrap.map_or(ex, |m: f64| m.min(ex)));
            }

            if req.result_format == ResultFormat::Delta {
                let (h_el, s_el, g_el, cp_el) =
                    sum_element_references(term, t_k, req.criss_cobble, repo)?;
                pt.h -= h_el;
                pt.s -= s_el;
                pt.g -= g_el;
                pt.cp -= cp_el;
            }

            substance_rows.push(SubstanceRow {
                temperature: req.temp_unit.from_kelvin(t_k),
                cp: pt.cp * s_factor,
                h: pt.h * e_factor,
                s: pt.s * s_factor,
                g: pt.g * e_factor,
                reference: ref_str,
                is_transition: is_tr,
            });
        }

        let species_data = build_species_data(&parsed, &[sp], &[max_extrap]);
        let rxn_s_label = match req.energy_unit {
            EnergyUnit::Calories | EnergyUnit::Kilocalories => "cal/K",
            EnergyUnit::Joules | EnergyUnit::Kilojoules => "J/K",
        };

        return Ok(CalculationResponse {
            normalized_input: term.formula.clone(),
            is_single_substance: true,
            has_electrochemical_potential: false,
            result_format: req.result_format,
            temp_unit_label: req.temp_unit.label().into(),
            energy_unit_label: req.energy_unit.energy_label().into(),
            entropy_unit_label: req.energy_unit.entropy_label().into(),
            reaction_entropy_unit_label: rxn_s_label.into(),
            substance_rows,
            reaction_rows: vec![],
            species_data,
        });
    }

    // Reaction Equation mode
    let all_terms: Vec<(&ParsedTerm, f64)> = parsed
        .reactants
        .iter()
        .map(|t| (t, -t.coeff))
        .chain(parsed.products.iter().map(|t| (t, t.coeff)))
        .collect();

    let mut all_species = Vec::with_capacity(all_terms.len());
    let mut n_electrons = 0.0f64;
    for (term, signed_coeff) in &all_terms {
        if term.is_electron {
            n_electrons += signed_coeff.abs();
        }
        let sp = repo
            .get_species(&term.formula)
            .ok_or_else(|| SoryntelError::SpeciesNotFound(term.formula.clone()))?;
        all_species.push(sp);
    }

    let mut reaction_rows = Vec::with_capacity(temps_k.len());
    let mut extraps = vec![None; all_species.len()];

    for &t_k in &temps_k {
        let mut dh = 0.0;
        let mut ds = 0.0;
        let mut dg = 0.0;

        for (idx, ((_term, signed_coeff), sp)) in all_terms.iter().zip(&all_species).enumerate() {
            let (pt, _) = evaluate_species_si(sp, t_k, req.criss_cobble, false)?;
            if let Some(ex) = pt.extrapolated_from_k {
                extraps[idx] = Some(extraps[idx].map_or(ex, |m: f64| m.min(ex)));
            }
            dh += signed_coeff * pt.h;
            ds += signed_coeff * pt.s;
            dg += signed_coeff * pt.g;
        }

        let log10_k = -dg / (R_JOULE * t_k * std::f64::consts::LN_10);
        let k_eq = if log10_k > 308.0 {
            f64::INFINITY
        } else if log10_k < -323.0 {
            0.0
        } else {
            10.0_f64.powf(log10_k)
        };

        let e_vs_she_v = if n_electrons > 1e-9 {
            Some(-dg / (n_electrons * FARADAY_C_MOL))
        } else {
            None
        };

        reaction_rows.push(ReactionRow {
            temperature: req.temp_unit.from_kelvin(t_k),
            delta_h: dh * e_factor,
            delta_s: ds * s_factor,
            delta_g: dg * e_factor,
            k_eq,
            log10_k,
            e_vs_she_v,
        });
    }

    let species_data = build_species_data(&parsed, &all_species, &extraps);
    let rxn_s_label = match req.energy_unit {
        EnergyUnit::Calories | EnergyUnit::Kilocalories => "cal/K",
        EnergyUnit::Joules | EnergyUnit::Kilojoules => "J/K",
    };

    Ok(CalculationResponse {
        normalized_input: soryntel_solver::format_reaction(&parsed),
        is_single_substance: false,
        has_electrochemical_potential: n_electrons > 1e-9,
        result_format: req.result_format,
        temp_unit_label: req.temp_unit.label().into(),
        energy_unit_label: req.energy_unit.energy_label().into(),
        entropy_unit_label: req.energy_unit.entropy_label().into(),
        reaction_entropy_unit_label: rxn_s_label.into(),
        substance_rows: vec![],
        reaction_rows,
        species_data,
    })
}

pub(crate) fn sum_element_references(
    term: &ParsedTerm,
    t_k: f64,
    criss_cobble: bool,
    repo: &dyn SpeciesRepository,
) -> Result<(f64, f64, f64, f64), SoryntelError> {
    let mut h = 0.0;
    let mut s = 0.0;
    let mut g = 0.0;
    let mut cp = 0.0;
    for (el, &cnt) in &term.elements {
        if let Some(ref_sp) = repo.get_reference_element(el) {
            let (pt, _) = evaluate_species_si(&ref_sp, t_k, criss_cobble, false)?;
            // Determine exact stoichiometric atoms per mole of reference species (e.g. O2(g)->2, I2->2, P4->4, Fe->1)
            let atoms_per_mol = soryntel_solver::parse_term(&ref_sp.formula)
                .ok()
                .and_then(|parsed_ref| parsed_ref.elements.get(el).copied())
                .filter(|&n| n > 0.0)
                .unwrap_or_else(|| if ref_sp.formula.contains("2") { 2.0 } else { 1.0 });
            let factor = cnt / atoms_per_mol;
            h += factor * pt.h;
            s += factor * pt.s;
            g += factor * pt.g;
            cp += factor * pt.cp;
        }
    }
    if term.charge != 0 {
        if let (Some(h2), Some(hp)) = (repo.get_species("H2(g)"), repo.get_species("H(+a)")) {
            let (p_h2, _) = evaluate_species_si(&h2, t_k, criss_cobble, false)?;
            let (p_hp, _) = evaluate_species_si(&hp, t_k, criss_cobble, false)?;
            let z = term.charge as f64;
            // HSC Manual Ch. 10.1 Eq. for aqueous ions in Delta (Formation) format:
            // ΔX(ion) = X(ion) - Σ X(elements) + (z/2)·X(H2(g)) - z·X(H(+a))
            h -= (z / 2.0) * p_h2.h - z * p_hp.h;
            s -= (z / 2.0) * p_h2.s - z * p_hp.s;
            g -= (z / 2.0) * p_h2.g - z * p_hp.g;
            cp -= (z / 2.0) * p_h2.cp - z * p_hp.cp;
        }
    }
    Ok((h, s, g, cp))
}

fn build_species_data(
    parsed: &ParsedReaction,
    species_list: &[ChemicalSpecies],
    extraps: &[Option<f64>],
) -> Vec<SpeciesDataRow> {
    let n_r = parsed.reactants.len();
    let total_r_mass: f64 = parsed
        .reactants
        .iter()
        .zip(&species_list[..n_r])
        .map(|(t, sp)| t.coeff * sp.molecular_weight)
        .sum();
    let total_p_mass: f64 = parsed
        .products
        .iter()
        .zip(&species_list[n_r..])
        .map(|(t, sp)| t.coeff * sp.molecular_weight)
        .sum();

    let mut rows = Vec::with_capacity(species_list.len());
    for (idx, (term, sp)) in parsed
        .reactants
        .iter()
        .chain(parsed.products.iter())
        .zip(species_list)
        .enumerate()
    {
        let is_react = idx < n_r;
        let total_mass = if is_react { total_r_mass } else { total_p_mass };
        let mass_g = term.coeff * sp.molecular_weight;
        let conc_wt_pct = if total_mass > 1e-12 {
            (mass_g / total_mass) * 100.0
        } else {
            0.0
        };
        let (volume, volume_unit) = match sp.phase {
            Phase::Gas => (term.coeff * STP_MOLAR_VOL_L, "l".to_string()),
            Phase::Solid | Phase::Liquid if sp.density_g_cm3 > 1e-9 => {
                (mass_g / sp.density_g_cm3, "ml".to_string())
            }
            _ => (0.0, "ml".to_string()),
        };

        rows.push(SpeciesDataRow {
            formula: sp.formula.clone(),
            side: if is_react {
                "Reactant".into()
            } else {
                "Product".into()
            },
            molar_mass: sp.molecular_weight,
            conc_wt_pct,
            mol: term.coeff,
            mass_g,
            volume,
            volume_unit,
            extrapolated_from_k: extraps[idx],
        });
    }
    rows
}

/// Y-axis property selector for Module 1 (Reaction Equations, Ch. 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReactionYAxis {
    #[default]
    Cp,
    H,
    S,
    G,
    LogK,
    K,
    EVsShe,
}

impl ReactionYAxis {
    pub fn short_label(self, is_single: bool, is_delta: bool) -> &'static str {
        match self {
            Self::Cp => {
                if is_single && is_delta {
                    "ΔCp"
                } else {
                    "Cp"
                }
            }
            Self::H => {
                if !is_single || is_delta {
                    "ΔH"
                } else {
                    "H"
                }
            }
            Self::S => {
                if !is_single || is_delta {
                    "ΔS"
                } else {
                    "S"
                }
            }
            Self::G => {
                if !is_single || is_delta {
                    "ΔG"
                } else {
                    "G"
                }
            }
            Self::LogK => "Log K",
            Self::K => "K",
            Self::EVsShe => "E vs. SHE (V)",
        }
    }

    pub fn axis_label(self, res: &CalculationResponse, log_y_scale: bool) -> String {
        let is_delta = res.result_format == ResultFormat::Delta;
        match self {
            Self::Cp => {
                let p = if res.is_single_substance && is_delta {
                    "ΔCp"
                } else {
                    "Cp"
                };
                format!("{p} ({})", res.entropy_unit_label)
            }
            Self::H => {
                if res.is_single_substance {
                    let p = if is_delta { "ΔH" } else { "H" };
                    format!("{p} ({}/mol)", res.energy_unit_label)
                } else {
                    format!("ΔH ({})", res.energy_unit_label)
                }
            }
            Self::S => {
                if res.is_single_substance {
                    let p = if is_delta { "ΔS" } else { "S" };
                    format!("{p} ({})", res.entropy_unit_label)
                } else {
                    format!("ΔS ({})", res.reaction_entropy_unit_label)
                }
            }
            Self::G => {
                if res.is_single_substance {
                    let p = if is_delta { "ΔG" } else { "G" };
                    format!("{p} ({}/mol)", res.energy_unit_label)
                } else {
                    format!("ΔG ({})", res.energy_unit_label)
                }
            }
            Self::LogK => "Log K".to_string(),
            Self::K => {
                if log_y_scale {
                    "K (log scale)".to_string()
                } else {
                    "K".to_string()
                }
            }
            Self::EVsShe => "E vs. SHE (V)".to_string(),
        }
    }

    pub fn chart_title(self) -> &'static str {
        match self {
            Self::Cp => "Heat capacity",
            Self::H => "Enthalpy",
            Self::S => "Entropy",
            Self::G => "Gibbs free energy",
            Self::LogK => "Logarithm of equilibrium constant",
            Self::K => "Equilibrium constant",
            Self::EVsShe => "Standard reduction potential",
        }
    }
}

/// Single collected worksheet in Module 1 (`S1`, `S2`, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionCollectedSheet {
    pub name: String,
    pub response: CalculationResponse,
    pub visible_in_plot: bool,
    /// Multiplier for `Input Amounts` (`mol`, `g`, `Volume`) in `Species Data` (default `1.0`).
    pub amount_scale: f64,
}

/// Complete `.rea8` workbook structure (HSC Ch. 10.1 Step 10 & Ch. 10.2 Step 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionWorkbookFile {
    pub version: String,
    pub formula_input: String,
    pub smallest_coeff: f64,
    pub t_from: f64,
    pub t_to: f64,
    pub t_step: f64,
    pub temp_unit: TemperatureUnit,
    pub energy_unit: EnergyUnit,
    pub result_format: ResultFormat,
    pub show_transitions: bool,
    pub criss_cobble: bool,
    pub y_axis: ReactionYAxis,
    pub collect_data: bool,
    pub plot_all_data: bool,
    pub log_y_scale: bool,
    pub decimals: usize,
    pub sheets: Vec<ReactionCollectedSheet>,
    pub active_sheet_idx: usize,
}

pub fn serialize_rea8_workbook(wb: &ReactionWorkbookFile) -> Result<String, SoryntelError> {
    serde_json::to_string_pretty(wb)
        .map_err(|e| SoryntelError::DatabaseError(format!("Failed to serialize .rea8: {e}")))
}

pub fn deserialize_rea8_workbook(raw: &str) -> Result<ReactionWorkbookFile, SoryntelError> {
    serde_json::from_str(raw)
        .map_err(|e| SoryntelError::DatabaseError(format!("Failed to parse .rea8: {e}")))
}

/// Verify whether `candidate` sheet can be plotted together with `primary` under `Plot All Data`
/// per HSC Manual Ch. 10.1 Step 4:
/// - Must share the same mode (pure substance vs. reaction cannot be mixed)
/// - Must share the same temperature and energy units
/// - For pure substances, must share the same `ResultFormat` (`Normal` vs. `Delta`)
pub fn are_responses_plot_compatible(
    primary: &CalculationResponse,
    candidate: &CalculationResponse,
) -> bool {
    if primary.is_single_substance != candidate.is_single_substance {
        return false;
    }
    if primary.temp_unit_label != candidate.temp_unit_label
        || primary.energy_unit_label != candidate.energy_unit_label
    {
        return false;
    }
    if primary.is_single_substance && primary.result_format != candidate.result_format {
        return false;
    }
    true
}

/// Extract `(T, Y)` data points for charting a `CalculationResponse`.
pub fn extract_reaction_plot_points(
    res: &CalculationResponse,
    y_axis: ReactionYAxis,
    log_y_scale: bool,
) -> Vec<[f64; 2]> {
    if res.is_single_substance {
        res.substance_rows
            .iter()
            .map(|r| {
                let y = match y_axis {
                    ReactionYAxis::Cp => r.cp,
                    ReactionYAxis::H => r.h,
                    ReactionYAxis::S => r.s,
                    ReactionYAxis::G => r.g,
                    _ => r.cp,
                };
                [r.temperature, y]
            })
            .collect()
    } else {
        res.reaction_rows
            .iter()
            .filter_map(|r| {
                let y = match y_axis {
                    ReactionYAxis::H => r.delta_h,
                    ReactionYAxis::S => r.delta_s,
                    ReactionYAxis::G => r.delta_g,
                    ReactionYAxis::K => {
                        if log_y_scale {
                            if r.k_eq > 0.0 && r.k_eq.is_finite() {
                                r.k_eq.log10()
                            } else {
                                r.log10_k
                            }
                        } else {
                            r.k_eq
                        }
                    }
                    ReactionYAxis::LogK => r.log10_k,
                    ReactionYAxis::EVsShe => r.e_vs_she_v.unwrap_or(0.0),
                    _ => r.log10_k,
                };
                if y.is_finite() {
                    Some([r.temperature, y])
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Format a collected sheet as tab-separated values (TSV) for `Copy Sheet` (Ch. 10.1 Step 9).
pub fn format_reaction_sheet_tsv(sheet: &ReactionCollectedSheet, decimals: usize) -> String {
    let res = &sheet.response;
    let scale = if sheet.amount_scale > 0.0 && sheet.amount_scale.is_finite() {
        sheet.amount_scale
    } else {
        1.0
    };
    let dec = decimals.clamp(1, 8);
    let mut out = String::new();
    if res.is_single_substance {
        let is_delta = res.result_format == ResultFormat::Delta;
        let cp_h = if is_delta { "ΔCp" } else { "Cp" };
        let h_h = if is_delta { "ΔH" } else { "H" };
        let s_h = if is_delta { "ΔS" } else { "S" };
        let g_h = if is_delta { "ΔG" } else { "G" };
        out.push_str(&format!("Formula\t{}\n", res.normalized_input));
        out.push_str("Reaction Data\n");
        out.push_str(&format!("T\t{cp_h}\t{h_h}\t{s_h}\t{g_h}\tReference\n"));
        out.push_str(&format!(
            "{}\t{}\t{}/mol\t{}\t{}/mol\t\n",
            res.temp_unit_label,
            res.entropy_unit_label,
            res.energy_unit_label,
            res.entropy_unit_label,
            res.energy_unit_label
        ));
        for r in &res.substance_rows {
            out.push_str(&format!(
                "{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.dec$}\t{}\n",
                r.temperature,
                r.cp,
                r.h,
                r.s,
                r.g,
                r.reference,
                dec = dec
            ));
        }
    } else {
        out.push_str(&format!("Reaction Equation\t{}\n", res.normalized_input));
        out.push_str("Reaction Data\n");
        if res.has_electrochemical_potential {
            out.push_str("T\tΔH\tΔS\tΔG\tK\tLog K\tE vs. SHE\n");
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t\t\tV\n",
                res.temp_unit_label,
                res.energy_unit_label,
                res.reaction_entropy_unit_label,
                res.energy_unit_label
            ));
        } else {
            out.push_str("T\tΔH\tΔS\tΔG\tK\tLog K\n");
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t\t\n",
                res.temp_unit_label,
                res.energy_unit_label,
                res.reaction_entropy_unit_label,
                res.energy_unit_label
            ));
        }
        for r in &res.reaction_rows {
            if let Some(ev) = r.e_vs_she_v {
                out.push_str(&format!(
                    "{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.3E}\t{:.dec$}\t{:.dec$}\n",
                    r.temperature,
                    r.delta_h,
                    r.delta_s,
                    r.delta_g,
                    r.k_eq,
                    r.log10_k,
                    ev,
                    dec = dec
                ));
            } else {
                out.push_str(&format!(
                    "{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.dec$}\t{:.3E}\t{:.dec$}\n",
                    r.temperature,
                    r.delta_h,
                    r.delta_s,
                    r.delta_g,
                    r.k_eq,
                    r.log10_k,
                    dec = dec
                ));
            }
        }
    }

    out.push_str("\nSpecies Data\n");
    out.push_str("Formula\tM\tConc.\tmol\tg\tVolume\tUnit\tExtrapolated\n");
    out.push_str("\tg/mol\twt-%\tmol\tg\tVolume\tUnit\tFrom T(K)\n");
    for s in &res.species_data {
        let ext = s
            .extrapolated_from_k
            .map(|k| format!("{k:.2}"))
            .unwrap_or_else(|| "-".into());
        out.push_str(&format!(
            "{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\n",
            s.formula,
            s.molar_mass,
            s.conc_wt_pct,
            s.mol * scale,
            s.mass_g * scale,
            s.volume * scale,
            s.volume_unit,
            ext
        ));
    }
    out
}

/// Build `ExportSheet` objects for `Export To Excel (.xlsx)` (Ch. 10.2 Step 4).
pub fn build_reaction_export_sheets(
    sheets: &[ReactionCollectedSheet],
) -> Vec<spreadsheet_export::ExportSheet> {
    use spreadsheet_export::{ExportCell, ExportCellStyle, ExportCellValue, ExportSheet};

    sheets
        .iter()
        .map(|sheet| {
            let res = &sheet.response;
            let scale = if sheet.amount_scale > 0.0 && sheet.amount_scale.is_finite() {
                sheet.amount_scale
            } else {
                1.0
            };
            let mut cells = Vec::new();
            let push_txt =
                |cells: &mut Vec<ExportCell>, r: usize, c: usize, s: &str, st: ExportCellStyle| {
                    cells.push(ExportCell {
                        row: r,
                        col: c,
                        value: ExportCellValue::Text(s.to_string()),
                        style: st,
                    });
                };
            let push_num =
                |cells: &mut Vec<ExportCell>, r: usize, c: usize, v: f64, st: ExportCellStyle| {
                    cells.push(ExportCell {
                        row: r,
                        col: c,
                        value: ExportCellValue::Number(v),
                        style: st,
                    });
                };

            let title_lbl = if res.is_single_substance {
                "Formula"
            } else {
                "Reaction Equation"
            };
            push_txt(&mut cells, 0, 2, title_lbl, ExportCellStyle::Header);
            push_txt(
                &mut cells,
                1,
                2,
                &res.normalized_input,
                ExportCellStyle::BoldBox,
            );
            push_txt(&mut cells, 3, 0, "Reaction Data", ExportCellStyle::Header);

            let mut cur_row = 4;
            if res.is_single_substance {
                let is_delta = res.result_format == ResultFormat::Delta;
                let headers = [
                    "T",
                    if is_delta { "ΔCp" } else { "Cp" },
                    if is_delta { "ΔH" } else { "H" },
                    if is_delta { "ΔS" } else { "S" },
                    if is_delta { "ΔG" } else { "G" },
                    "Reference",
                ];
                let units = [
                    res.temp_unit_label.clone(),
                    res.entropy_unit_label.clone(),
                    format!("{}/mol", res.energy_unit_label),
                    res.entropy_unit_label.clone(),
                    format!("{}/mol", res.energy_unit_label),
                    String::new(),
                ];
                for (c, h) in headers.iter().enumerate() {
                    push_txt(&mut cells, cur_row, c, h, ExportCellStyle::Header);
                }
                cur_row += 1;
                for (c, u) in units.iter().enumerate() {
                    push_txt(&mut cells, cur_row, c, u, ExportCellStyle::Header);
                }
                cur_row += 1;
                for r in &res.substance_rows {
                    push_num(&mut cells, cur_row, 0, r.temperature, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 1, r.cp, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 2, r.h, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 3, r.s, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 4, r.g, ExportCellStyle::Default);
                    push_txt(&mut cells, cur_row, 5, &r.reference, ExportCellStyle::Default);
                    cur_row += 1;
                }
            } else {
                let headers = if res.has_electrochemical_potential {
                    vec!["T", "ΔH", "ΔS", "ΔG", "K", "Log K", "E vs. SHE"]
                } else {
                    vec!["T", "ΔH", "ΔS", "ΔG", "K", "Log K"]
                };
                let mut units = vec![
                    res.temp_unit_label.clone(),
                    res.energy_unit_label.clone(),
                    res.reaction_entropy_unit_label.clone(),
                    res.energy_unit_label.clone(),
                    String::new(),
                    String::new(),
                ];
                if res.has_electrochemical_potential {
                    units.push("V".to_string());
                }
                for (c, h) in headers.iter().enumerate() {
                    push_txt(&mut cells, cur_row, c, h, ExportCellStyle::Header);
                }
                cur_row += 1;
                for (c, u) in units.iter().enumerate() {
                    push_txt(&mut cells, cur_row, c, u, ExportCellStyle::Header);
                }
                cur_row += 1;
                for r in &res.reaction_rows {
                    push_num(&mut cells, cur_row, 0, r.temperature, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 1, r.delta_h, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 2, r.delta_s, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 3, r.delta_g, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 4, r.k_eq, ExportCellStyle::Default);
                    push_num(&mut cells, cur_row, 5, r.log10_k, ExportCellStyle::Default);
                    if let Some(ev) = r.e_vs_she_v {
                        push_num(&mut cells, cur_row, 6, ev, ExportCellStyle::Default);
                    }
                    cur_row += 1;
                }
            }

            cur_row += 1;
            push_txt(&mut cells, cur_row, 0, "Species Data", ExportCellStyle::Header);
            cur_row += 1;
            let sp_h1 = [
                "Formula",
                "M",
                "Conc.",
                "Input Amounts",
                "",
                "",
                "",
                "Extrapolated",
            ];
            let sp_h2 = [
                "",
                "g/mol",
                "wt-%",
                "mol",
                "g",
                "Volume",
                "Unit",
                "From T(K)",
            ];
            for (c, h) in sp_h1.iter().enumerate() {
                push_txt(&mut cells, cur_row, c, h, ExportCellStyle::Header);
            }
            cur_row += 1;
            for (c, h) in sp_h2.iter().enumerate() {
                push_txt(&mut cells, cur_row, c, h, ExportCellStyle::Header);
            }
            cur_row += 1;
            for s in &res.species_data {
                push_txt(&mut cells, cur_row, 0, &s.formula, ExportCellStyle::BoldBox);
                push_num(&mut cells, cur_row, 1, s.molar_mass, ExportCellStyle::Default);
                push_num(&mut cells, cur_row, 2, s.conc_wt_pct, ExportCellStyle::Default);
                push_num(&mut cells, cur_row, 3, s.mol * scale, ExportCellStyle::Default);
                push_num(&mut cells, cur_row, 4, s.mass_g * scale, ExportCellStyle::Default);
                push_num(&mut cells, cur_row, 5, s.volume * scale, ExportCellStyle::Default);
                push_txt(&mut cells, cur_row, 6, &s.volume_unit, ExportCellStyle::Default);
                let ext = s
                    .extrapolated_from_k
                    .map(|k| format!("{k:.2}"))
                    .unwrap_or_else(|| "-".into());
                push_txt(&mut cells, cur_row, 7, &ext, ExportCellStyle::Default);
                cur_row += 1;
            }

            ExportSheet {
                name: sheet.name.clone(),
                cells,
                col_widths: vec![14.0, 14.0, 14.0, 14.0, 15.0, 16.0, 12.0, 18.0],
            }
        })
        .collect()
}

/// Render a standalone vector SVG chart for `Save Chart` / `Copy Vector` (Ch. 10.2 Step 6 & Fig. 1/2/4/6/9).
pub fn render_reaction_chart_svg(
    title: &str,
    x_label: &str,
    y_label: &str,
    series: &[(String, Vec<[f64; 2]>, &str)],
    show_grid: bool,
    show_curve_labels: bool,
) -> String {
    let w = 860.0_f64;
    let h = 520.0_f64;
    let pad_l = 82.0_f64;
    let pad_r = 34.0_f64;
    let pad_t = 46.0_f64;
    let pad_b = 58.0_f64;
    let pw = w - pad_l - pad_r;
    let ph = h - pad_t - pad_b;

    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for (_, pts, _) in series {
        for &[x, y] in pts {
            if x.is_finite() && y.is_finite() {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
        }
    }
    if !x_min.is_finite() || (x_max - x_min).abs() < 1e-12 {
        x_min = 0.0;
        x_max = 1000.0;
    }
    if !y_min.is_finite() || (y_max - y_min).abs() < 1e-12 {
        let base = if y_min.is_finite() { y_min } else { 0.0 };
        y_min = base - 1.0;
        y_max = base + 1.0;
    } else {
        let margin = (y_max - y_min) * 0.06;
        y_min -= margin;
        y_max += margin;
    }

    let map_x = |x: f64| pad_l + ((x - x_min) / (x_max - x_min)) * pw;
    let map_y = |y: f64| pad_t + ph - ((y - y_min) / (y_max - y_min)) * ph;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n"
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    svg.push_str(&format!(
        "<text x=\"{:.1}\" y=\"28\" text-anchor=\"middle\" font-family=\"serif\" font-size=\"16\" font-weight=\"bold\" fill=\"#111827\">{}</text>\n",
        w * 0.5,
        xml_escape(title)
    ));

    if show_grid {
        for i in 0..=5 {
            let gx = pad_l + (i as f64 / 5.0) * pw;
            let gy = pad_t + (i as f64 / 5.0) * ph;
            svg.push_str(&format!(
                "<line x1=\"{gx:.1}\" y1=\"{pad_t:.1}\" x2=\"{gx:.1}\" y2=\"{:.1}\" stroke=\"#d1d5db\" stroke-dasharray=\"3,3\" stroke-width=\"1\"/>\n",
                pad_t + ph
            ));
            svg.push_str(&format!(
                "<line x1=\"{pad_l:.1}\" y1=\"{gy:.1}\" x2=\"{:.1}\" y2=\"{gy:.1}\" stroke=\"#d1d5db\" stroke-dasharray=\"3,3\" stroke-width=\"1\"/>\n",
                pad_l + pw
            ));
        }
    }

    svg.push_str(&format!(
        "<rect x=\"{pad_l:.1}\" y=\"{pad_t:.1}\" width=\"{pw:.1}\" height=\"{ph:.1}\" fill=\"none\" stroke=\"#111827\" stroke-width=\"1.4\"/>\n"
    ));

    for i in 0..=5 {
        let tx = x_min + (i as f64 / 5.0) * (x_max - x_min);
        let gx = map_x(tx);
        svg.push_str(&format!(
            "<text x=\"{gx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"11\" fill=\"#1f2937\">{tx:.1}</text>\n",
            pad_t + ph + 18.0
        ));
        let ty = y_min + (i as f64 / 5.0) * (y_max - y_min);
        let gy = map_y(ty);
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"11\" fill=\"#1f2937\">{ty:.2}</text>\n",
            pad_l - 8.0,
            gy + 4.0
        ));
    }

    svg.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"13\" font-weight=\"bold\" fill=\"#111827\">{}</text>\n",
        pad_l + pw * 0.5,
        h - 14.0,
        xml_escape(x_label)
    ));
    svg.push_str(&format!(
        "<text x=\"22\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"13\" font-weight=\"bold\" fill=\"#111827\" transform=\"rotate(-90 22 {:.1})\">{}</text>\n",
        pad_t + ph * 0.5,
        pad_t + ph * 0.5,
        xml_escape(y_label)
    ));

    for (idx, (name, pts, color)) in series.iter().enumerate() {
        if pts.is_empty() {
            continue;
        }
        let mut path = String::new();
        for (k, &[x, y]) in pts.iter().enumerate() {
            let px = map_x(x);
            let py = map_y(y);
            if k == 0 {
                path.push_str(&format!("M {px:.2} {py:.2}"));
            } else {
                path.push_str(&format!(" L {px:.2} {py:.2}"));
            }
        }
        svg.push_str(&format!(
            "<path d=\"{path}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.0\"/>\n"
        ));
        if show_curve_labels {
            let mid = ((pts.len() * (idx + 1)) / (series.len() + 1)).min(pts.len() - 1);
            let [mx, my] = pts[mid];
            let lx = map_x(mx);
            let ly = (map_y(my) - 8.0).max(pad_t + 14.0);
            svg.push_str(&format!(
                "<text x=\"{lx:.1}\" y=\"{ly:.1}\" font-family=\"sans-serif\" font-size=\"12\" font-weight=\"bold\" fill=\"{color}\">{}</text>\n",
                xml_escape(name)
            ));
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod reaction_module_tests {
    use super::*;
    use std::collections::HashMap;

    struct MockRepo {
        map: HashMap<String, ChemicalSpecies>,
    }

    impl MockRepo {
        fn new() -> Self {
            let mut map = HashMap::new();
            let add = |map: &mut HashMap<String, ChemicalSpecies>,
                       formula: &str,
                       phase: Phase,
                       mw: f64,
                       charge: i32,
                       hf: f64,
                       s0: f64,
                       a: f64| {
                map.insert(
                    formula.to_string(),
                    ChemicalSpecies {
                        formula: formula.to_string(),
                        name: formula.to_string(),
                        phase,
                        molecular_weight: mw,
                        density_g_cm3: 2.5,
                        charge,
                        intervals: vec![SpeciesThermoInterval {
                            t_min: 298.15,
                            t_max: 2000.0,
                            phase,
                            kelley: KelleyCoefficients {
                                a,
                                b: 0.0,
                                c: 0.0,
                                d: 0.0,
                            },
                            h_or_delta_h_tr: hf,
                            s_or_delta_s_tr: s0,
                            reference: "TestRef".into(),
                        }],
                    },
                );
            };
            add(&mut map, "H2(g)", Phase::Gas, 2.016, 0, 0.0, 130.68, 28.84);
            add(&mut map, "O2(g)", Phase::Gas, 31.999, 0, 0.0, 205.15, 29.38);
            add(&mut map, "S", Phase::Solid, 32.06, 0, 0.0, 32.05, 22.60);
            add(&mut map, "I2", Phase::Solid, 253.81, 0, 0.0, 116.14, 54.44);
            add(&mut map, "H(+a)", Phase::AqueousIon, 1.008, 1, 0.0, 0.0, 0.0);
            add(
                &mut map,
                "SO4(-2a)",
                Phase::AqueousIon,
                96.06,
                -2,
                -909_340.0,
                18.50,
                -293.0,
            );
            add(
                &mut map,
                "HI(g)",
                Phase::Gas,
                127.91,
                0,
                26_500.0,
                206.59,
                29.16,
            );
            Self { map }
        }
    }

    impl SpeciesRepository for MockRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.map.get(formula).cloned()
        }

        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            let f = match element_symbol {
                "H" => "H2(g)",
                "O" => "O2(g)",
                "S" => "S",
                "I" => "I2",
                _ => return None,
            };
            self.get_species(f)
        }
    }

    #[test]
    fn delta_format_handles_i2_reference_and_aqueous_ion_thermodynamic_consistency() {
        let repo = MockRepo::new();
        // 1. Check HI(g) at 298.15 K in Delta mode: ΔHf(298.15) must equal +26.500 kJ/mol
        // because H2(g) and I2 both have Hf(298.15) = 0 and I2 has 2 I atoms per mole.
        let req_hi = CalculationRequest {
            input: "HI(g)".into(),
            t_from: 25.0,
            t_to: 125.0,
            t_step: 100.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Delta,
            show_transitions: false,
            criss_cobble: false,
        };
        let res_hi = calculate(&req_hi, &repo).unwrap();
        let r0 = &res_hi.substance_rows[0];
        assert!((r0.h - 26.5).abs() < 1e-4);
        // Verify ΔG = ΔH - T·ΔS at T = 398.15 K (125 °C)
        let r1 = &res_hi.substance_rows[1];
        let dg_expected = r1.h - 398.15 * (r1.s / 1000.0);
        assert!((r1.g - dg_expected).abs() < 1e-4);

        // 2. Check SO4(-2a) in Delta mode at 25 °C and 125 °C:
        // ΔG(T) must satisfy ΔG(T) = ΔH(T) - T·ΔS(T) exactly with the H2(g)/H(+a) charge term.
        let req_ion = CalculationRequest {
            input: "SO4(-2a)".into(),
            t_from: 25.0,
            t_to: 125.0,
            t_step: 100.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Delta,
            show_transitions: false,
            criss_cobble: false,
        };
        let res_ion = calculate(&req_ion, &repo).unwrap();
        for (idx, row) in res_ion.substance_rows.iter().enumerate() {
            let t_k = 298.15 + (idx as f64) * 100.0;
            let dg_from_dh_ds = row.h - t_k * (row.s / 1000.0);
            assert!(
                (row.g - dg_from_dh_ds).abs() < 1e-4,
                "Thermodynamic identity ΔG = ΔH - T·ΔS violated for ion at T={t_k}: g={}, dh-Tds={}",
                row.g,
                dg_from_dh_ds
            );
        }
    }

    #[test]
    fn plot_all_compatibility_enforces_manual_section_10_1_rules() {
        let repo = MockRepo::new();
        let req_sub_norm = CalculationRequest {
            input: "HI(g)".into(),
            t_from: 25.0,
            t_to: 125.0,
            t_step: 50.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Normal,
            show_transitions: false,
            criss_cobble: false,
        };
        let mut req_sub_delta = req_sub_norm.clone();
        req_sub_delta.result_format = ResultFormat::Delta;

        let mut req_rxn = req_sub_norm.clone();
        req_rxn.input = "0.5H2(g) + 0.5I2 = HI(g)".into();

        let res_norm = calculate(&req_sub_norm, &repo).unwrap();
        let res_delta = calculate(&req_sub_delta, &repo).unwrap();
        let res_rxn = calculate(&req_rxn, &repo).unwrap();

        assert!(are_responses_plot_compatible(&res_norm, &res_norm));
        assert!(!are_responses_plot_compatible(&res_norm, &res_delta));
        assert!(!are_responses_plot_compatible(&res_norm, &res_rxn));
    }
}
