//! Module 07: Eh-pH Diagrams (Pourbaix / STABCAL 2D lower-envelope engine).
//!
//! Computes predominance areas for a main element `M` in aqueous systems with
//! optional secondary elements `X` (e.g., Cu-S-H2O), water stability limits
//! (`O2(g)/H2O` and `H2O/H2(g)`), Criss-Cobble aqueous extrapolation, Davies
//! activity correction, potential scale offsets (SHE, SCE, Ag/AgCl), and
//! exact boundary line segments `(X1, Y1) -> (X2, Y2)`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, FARADAY_C_MOL, R_JOULE};
use soryntel_solver::parse_term;

use crate::{
    balance::h2o_saturation_pressure_bar, evaluate_species_si, sum_element_references,
    ChemicalSpecies, Phase, SpeciesRepository,
};

/// Reference electrode potential scale for the y-axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotentialScale {
    /// Standard Hydrogen Electrode (SHE), offset = 0.0 V.
    Hydrogen,
    /// Saturated Calomel Electrode (SCE), E_SCE = E_SHE - 0.2412 V.
    Calomel,
    /// Silver / Silver Chloride (3.5 M KCl), E_AgAgCl = E_SHE - 0.197 V.
    AgAgCl,
}

impl PotentialScale {
    pub fn offset_v(self) -> f64 {
        match self {
            PotentialScale::Hydrogen => 0.0,
            PotentialScale::Calomel => 0.2412,
            PotentialScale::AgAgCl => 0.197,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PotentialScale::Hydrogen => "Eh (Volts vs. SHE)",
            PotentialScale::Calomel => "E (Volts vs. SCE)",
            PotentialScale::AgAgCl => "E (Volts vs. Ag/AgCl)",
        }
    }
}

/// Element specification (molality & partial pressure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSpec {
    pub symbol: String,
    pub molality: f64,
    pub pressure_bar: f64,
}

/// Full request for an Eh-pH diagram calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhPhRequest {
    pub elements: Vec<ElementSpec>,
    pub main_element: String,
    pub species: Vec<ChemicalSpecies>,
    pub t_k: f64,
    pub min_ph: f64,
    pub max_ph: f64,
    pub min_eh_v: f64,
    pub max_eh_v: f64,
    pub potential_scale: PotentialScale,
    pub criss_cobble: bool,
    pub use_davies: bool,
    pub ionic_strength: f64,
    pub correction_factor: f64,
    pub dielectric_constant: Option<f64>,
    pub delta_g_h2o_kcal: Option<f64>,
    pub delta_g_overrides_kcal: BTreeMap<String, f64>,
    pub show_predominance_ions: bool,
    pub grid_resolution: usize,
}

impl Default for EhPhRequest {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            main_element: String::new(),
            species: Vec::new(),
            t_k: 298.15,
            min_ph: 0.0,
            max_ph: 14.0,
            min_eh_v: -2.0,
            max_eh_v: 2.0,
            potential_scale: PotentialScale::Hydrogen,
            criss_cobble: false,
            use_davies: false,
            ionic_strength: 0.0,
            correction_factor: 1.0,
            dielectric_constant: None,
            delta_g_h2o_kcal: None,
            delta_g_overrides_kcal: BTreeMap::new(),
            show_predominance_ions: false,
            grid_resolution: 220,
        }
    }
}

/// A straight boundary line segment in `(pH, Eh_display)` coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundarySegment {
    pub species_a: String,
    pub species_b: String,
    pub x1_ph: f64,
    pub y1_v: f64,
    pub x2_ph: f64,
    pub y2_v: f64,
}

/// Predominant species region with its interior label coordinate and area index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredominanceRegion {
    pub area_no: usize,
    pub species: String,
    pub phase: Phase,
    pub delta_g_kcal_mol: f64,
    pub label_ph: f64,
    pub label_eh_v: f64,
    pub cell_count: usize,
}

/// Species table row matching HSC's `E-pH Diagram Basic Settings` (`Table`) sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesTableRow {
    pub formula: String,
    pub delta_g_kcal_mol: f64,
    pub area_no: Option<usize>,
    pub segments: Vec<(f64, f64, f64, f64)>,
}

/// Complete computed Eh-pH diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhPhResult {
    pub title: String,
    pub t_k: f64,
    pub dielectric_constant: f64,
    pub delta_g_h2o_kcal: f64,
    pub p_sat_h2o_bar: f64,
    pub warnings: Vec<String>,
    pub segments: Vec<BoundarySegment>,
    pub regions: Vec<PredominanceRegion>,
    pub water_upper_line: [(f64, f64); 2],
    pub water_lower_line: [(f64, f64); 2],
    pub table_rows: Vec<SpeciesTableRow>,
    /// Row-major `grid_n x grid_n` predominant species index (`None` if empty).
    pub grid_n: usize,
    pub grid_region_idx: Vec<Option<usize>>,
}

/// Internal parsed thermodynamic representation of a candidate species.
#[derive(Debug, Clone)]
struct PreparedSpecies {
    formula: String,
    phase: Phase,
    /// `ΔG°(T) + RT ln(a_i)` in J/mol.
    mu_std_plus_act_j: f64,
    /// `ΔG°(T)` in kcal/mol for display in the `Table` sheet.
    delta_g_kcal: f64,
    n_o: f64,
    n_h: f64,
    charge: f64,
    /// Element counts excluding `H`, `O`, `e-`.
    other_elements: BTreeMap<String, f64>,
}

impl PreparedSpecies {
    /// Gibbs energy relative to `{other_elements, H2O, H+, e-}` at `(ph, eh_she_v)` in J/mol.
    #[inline]
    fn g_rel_j(&self, ph: f64, eh_she_v: f64, t_k: f64, dg_h2o_j: f64) -> f64 {
        let nu_h_plus = self.n_h - 2.0 * self.n_o;
        let nu_e_minus = nu_h_plus - self.charge;
        let rt_ln10 = R_JOULE * t_k * std::f64::consts::LN_10;
        self.mu_std_plus_act_j - self.n_o * dg_h2o_j
            + nu_h_plus * rt_ln10 * ph
            + nu_e_minus * FARADAY_C_MOL * eh_she_v
    }
}

/// Malmberg-Maryott approximation for the static dielectric constant of water.
pub fn water_dielectric_constant(t_k: f64) -> f64 {
    let t_c = (t_k - 273.15).clamp(0.0, 350.0);
    // Calibrated so ε(25 °C) = 78.382 (matching HSC6 EpH default).
    let eps = 87.819 - 0.40008 * t_c + 9.398e-4 * t_c * t_c - 1.410e-6 * t_c * t_c * t_c;
    eps.max(10.0)
}

/// Davies activity coefficient `ln(gamma_i)` for an aqueous species of charge `z`.
fn davies_ln_gamma(z: f64, t_k: f64, eps: f64, ionic_strength: f64, corr_factor: f64) -> f64 {
    if z.abs() < 1e-9 || ionic_strength <= 0.0 {
        return 0.0;
    }
    let a_dh = 1.82483e6 * (eps * t_k).powf(-1.5);
    let sqrt_i = ionic_strength.sqrt();
    let log10_gamma = -a_dh * z * z * (sqrt_i / (1.0 + sqrt_i) - 0.3 * ionic_strength) * corr_factor;
    log10_gamma * std::f64::consts::LN_10
}

fn prepare_species(
    sp: &ChemicalSpecies,
    t_k: f64,
    criss_cobble: bool,
    repo: Option<&dyn SpeciesRepository>,
    dg_overrides_kcal: &BTreeMap<String, f64>,
    elem_map: &HashMap<String, &ElementSpec>,
    main_elem: &str,
    use_davies: bool,
    eps: f64,
    ionic_strength: f64,
    corr_factor: f64,
) -> Option<PreparedSpecies> {
    if sp.formula.eq_ignore_ascii_case("H2O")
        || sp.formula.eq_ignore_ascii_case("H2O(L)")
        || sp.formula.eq_ignore_ascii_case("H(+a)")
        || sp.formula.eq_ignore_ascii_case("OH(-a)")
        || sp.formula.eq_ignore_ascii_case("e-")
    {
        // Water & H+/OH- are implicit background components in Eh-pH coordinates.
        return None;
    }
    let term = parse_term(&sp.formula).ok()?;
    let (dg_j, dg_kcal) = if let Some(&ov_kcal) = dg_overrides_kcal.get(&sp.formula) {
        (ov_kcal * 1000.0 * CAL_TO_JOULE, ov_kcal)
    } else {
        let (pt, _) = evaluate_species_si(sp, t_k, criss_cobble, false).ok()?;
        let g_ref = if let Some(r) = repo {
            sum_element_references(&term, t_k, criss_cobble, r)
                .map(|(_, _, g, _)| g)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let j = pt.g - g_ref;
        (j, j / (1000.0 * CAL_TO_JOULE))
    };

    let n_o = *term.elements.get("O").unwrap_or(&0.0);
    let n_h = *term.elements.get("H").unwrap_or(&0.0);
    let charge = term.charge as f64;
    let mut other_elements = BTreeMap::new();
    for (el, &cnt) in &term.elements {
        if el == "H" || el == "O" || el == "e-" {
            continue;
        }
        other_elements.insert(el.clone(), cnt);
    }
    if other_elements.is_empty() {
        return None;
    }

    // Activity term RT ln(a_i)
    let ln_a = match sp.phase {
        Phase::Solid | Phase::Liquid => 0.0,
        Phase::Gas => {
            let p_bar = elem_map
                .get(main_elem)
                .map(|e| e.pressure_bar)
                .unwrap_or(1.0)
                .max(1e-12);
            p_bar.ln()
        }
        Phase::AqueousIon | Phase::AqueousNeutral => {
            let m = elem_map
                .get(main_elem)
                .map(|e| e.molality)
                .unwrap_or(1.0)
                .max(1e-12);
            let nu_m = *other_elements.get(main_elem).unwrap_or(&1.0);
            let m_i = (m / nu_m.max(1.0)).max(1e-12);
            let ln_gamma = if use_davies {
                davies_ln_gamma(charge, t_k, eps, ionic_strength, corr_factor)
            } else {
                0.0
            };
            m_i.ln() + ln_gamma
        }
    };

    Some(PreparedSpecies {
        formula: sp.formula.clone(),
        phase: sp.phase,
        mu_std_plus_act_j: dg_j + R_JOULE * t_k * ln_a,
        delta_g_kcal: dg_kcal,
        n_o,
        n_h,
        charge,
        other_elements,
    })
}

/// Computes the predominant secondary-element chemical potential `mu_X(pH, Eh)` (J/mol of X)
/// at `(ph, eh_she_v)` over all species containing `x_elem` and not containing `main_elem`.
fn secondary_mu(
    x_elem: &str,
    sec_species: &[PreparedSpecies],
    ph: f64,
    eh_she_v: f64,
    t_k: f64,
    dg_h2o_j: f64,
) -> f64 {
    let mut best = f64::INFINITY;
    for sp in sec_species {
        if let Some(&nu_x) = sp.other_elements.get(x_elem) {
            if nu_x > 0.0 && sp.other_elements.len() == 1 {
                let g = sp.g_rel_j(ph, eh_she_v, t_k, dg_h2o_j) / nu_x;
                if g < best {
                    best = g;
                }
            }
        }
    }
    if best.is_finite() {
        best
    } else {
        0.0
    }
}

/// Evaluates normalized Gibbs energy per mole of `main_elem` for all candidate `main_species`.
#[inline]
fn eval_norm_energies(
    main_species: &[PreparedSpecies],
    sec_species: &[PreparedSpecies],
    main_elem: &str,
    ph: f64,
    eh_she_v: f64,
    t_k: f64,
    dg_h2o_j: f64,
    sec_elems: &[String],
    out_g: &mut Vec<f64>,
) -> usize {
    let mut sec_mus: Vec<(&str, f64)> = Vec::with_capacity(sec_elems.len());
    for sx in sec_elems {
        let mu = secondary_mu(sx, sec_species, ph, eh_she_v, t_k, dg_h2o_j);
        sec_mus.push((sx.as_str(), mu));
    }

    out_g.clear();
    let mut best_idx = 0usize;
    let mut best_val = f64::INFINITY;
    for (idx, sp) in main_species.iter().enumerate() {
        let nu_m = *sp.other_elements.get(main_elem).unwrap_or(&1.0);
        let mut g = sp.g_rel_j(ph, eh_she_v, t_k, dg_h2o_j);
        for (el, &nu_x) in &sp.other_elements {
            if el == main_elem {
                continue;
            }
            if let Some((_, mu_x)) = sec_mus.iter().find(|(sx, _)| *sx == el.as_str()) {
                g -= nu_x * mu_x;
            }
        }
        let g_norm = g / nu_m;
        out_g.push(g_norm);
        if g_norm < best_val {
            best_val = g_norm;
            best_idx = idx;
        }
    }
    best_idx
}

/// Bisection refinement along a segment `(ph0, eh0) -> (ph1, eh1)` where `idx_a` and `idx_b` cross.
fn refine_crossing(
    main_species: &[PreparedSpecies],
    sec_species: &[PreparedSpecies],
    main_elem: &str,
    sec_elems: &[String],
    t_k: f64,
    dg_h2o_j: f64,
    idx_a: usize,
    idx_b: usize,
    p0: (f64, f64),
    p1: (f64, f64),
) -> (f64, f64) {
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    let mut buf = Vec::with_capacity(main_species.len());
    eval_norm_energies(
        main_species,
        sec_species,
        main_elem,
        p0.0,
        p0.1,
        t_k,
        dg_h2o_j,
        sec_elems,
        &mut buf,
    );
    let diff0 = buf[idx_a] - buf[idx_b];
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        let ph = p0.0 + mid * (p1.0 - p0.0);
        let eh = p0.1 + mid * (p1.1 - p0.1);
        eval_norm_energies(
            main_species,
            sec_species,
            main_elem,
            ph,
            eh,
            t_k,
            dg_h2o_j,
            sec_elems,
            &mut buf,
        );
        let diff_m = buf[idx_a] - buf[idx_b];
        if (diff0 <= 0.0 && diff_m <= 0.0) || (diff0 >= 0.0 && diff_m >= 0.0) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t = 0.5 * (lo + hi);
    (p0.0 + t * (p1.0 - p0.0), p0.1 + t * (p1.1 - p0.1))
}

/// Computes ΔfG°(H2O, T) in kcal/mol using the repository's reference elements.
pub fn delta_g_formation_h2o_kcal(
    repo: &dyn SpeciesRepository,
    t_k: f64,
    criss_cobble: bool,
) -> f64 {
    let Some(h2o) = repo.get_species("H2O").or_else(|| repo.get_species("H2O(L)")) else {
        return -56.678;
    };
    let Ok((pt, _)) = evaluate_species_si(&h2o, t_k, criss_cobble, false) else {
        return -56.678;
    };
    let g_ref = parse_term(&h2o.formula)
        .ok()
        .and_then(|t| sum_element_references(&t, t_k, criss_cobble, repo).ok())
        .map(|(_, _, g, _)| g)
        .unwrap_or(0.0);
    (pt.g - g_ref) / (1000.0 * CAL_TO_JOULE)
}

/// Computes the Eh-pH diagram for `req`.
pub fn calculate_ehph(
    req: &EhPhRequest,
    repo: Option<&dyn SpeciesRepository>,
) -> Result<EhPhResult, String> {
    if req.main_element.trim().is_empty() {
        return Err("Main element is required.".to_string());
    }
    if req.max_ph <= req.min_ph {
        return Err("Max pH must be greater than Min pH.".to_string());
    }
    if req.max_eh_v <= req.min_eh_v {
        return Err("Max Eh must be greater than Min Eh.".to_string());
    }
    let t_k = req.t_k.max(273.15);
    let eps = req
        .dielectric_constant
        .unwrap_or_else(|| water_dielectric_constant(t_k));

    let dg_h2o_kcal = if let Some(v) = req.delta_g_h2o_kcal {
        v
    } else if let Some(r) = repo {
        let h2o_sp = r.get_species("H2O").or_else(|| r.get_species("H2O(L)"));
        if let Some(h2o) = h2o_sp {
            let (pt, _) = evaluate_species_si(&h2o, t_k, req.criss_cobble, false)
                .map_err(|e| format!("Failed evaluating H2O: {e}"))?;
            let g_ref = parse_term(&h2o.formula)
                .ok()
                .and_then(|t| sum_element_references(&t, t_k, req.criss_cobble, r).ok())
                .map(|(_, _, g, _)| g)
                .unwrap_or(0.0);
            (pt.g - g_ref) / (1000.0 * CAL_TO_JOULE)
        } else {
            -56.678
        }
    } else {
        -56.678
    };
    let dg_h2o_j = dg_h2o_kcal * 1000.0 * CAL_TO_JOULE;

    let p_sat_h2o_bar = h2o_saturation_pressure_bar(t_k);
    let mut warnings = Vec::new();
    for el in &req.elements {
        if el.pressure_bar < p_sat_h2o_bar {
            warnings.push(format!(
                "Pressure for {} ({:.3} bar) is below H2O saturation pressure ({:.3} bar) at {:.2} °C.",
                el.symbol,
                el.pressure_bar,
                p_sat_h2o_bar,
                t_k - 273.15
            ));
        }
    }

    let elem_map: HashMap<String, &ElementSpec> = req
        .elements
        .iter()
        .map(|e| (e.symbol.clone(), e))
        .collect();
    let sec_elems: Vec<String> = req
        .elements
        .iter()
        .map(|e| e.symbol.clone())
        .filter(|s| s != &req.main_element && s != "H" && s != "O")
        .collect();

    let mut main_species = Vec::new();
    let mut sec_species = Vec::new();

    for sp in &req.species {
        if req.show_predominance_ions
            && !matches!(sp.phase, Phase::AqueousIon | Phase::AqueousNeutral)
        {
            continue;
        }
        if let Some(prep) = prepare_species(
            sp,
            t_k,
            req.criss_cobble,
            repo,
            &req.delta_g_overrides_kcal,
            &elem_map,
            &req.main_element,
            req.use_davies,
            eps,
            req.ionic_strength,
            req.correction_factor,
        ) {
            if prep.other_elements.contains_key(&req.main_element) {
                main_species.push(prep);
            } else {
                sec_species.push(prep);
            }
        }
    }

    if main_species.is_empty() {
        return Err(format!(
            "No valid species containing main element '{}'.",
            req.main_element
        ));
    }

    let n = req.grid_resolution.clamp(60, 360);
    let scale_off = req.potential_scale.offset_v();
    // Internally compute on SHE scale, then shift by -scale_off for display.
    let min_she = req.min_eh_v + scale_off;
    let max_she = req.max_eh_v + scale_off;
    let dph = (req.max_ph - req.min_ph) / (n as f64 - 1.0);
    let deh = (max_she - min_she) / (n as f64 - 1.0);

    let mut winner = vec![0usize; n * n];
    let mut g_buf = Vec::with_capacity(main_species.len());
    for iy in 0..n {
        let eh_she = min_she + (iy as f64) * deh;
        for ix in 0..n {
            let ph = req.min_ph + (ix as f64) * dph;
            let idx = eval_norm_energies(
                &main_species,
                &sec_species,
                &req.main_element,
                ph,
                eh_she,
                t_k,
                dg_h2o_j,
                &sec_elems,
                &mut g_buf,
            );
            winner[iy * n + ix] = idx;
        }
    }

    // Dual-cell contour extraction for crisp boundary segments.
    let mut segments = Vec::new();
    for iy in 0..(n - 1) {
        let eh0 = min_she + (iy as f64) * deh;
        let eh1 = eh0 + deh;
        for ix in 0..(n - 1) {
            let ph0 = req.min_ph + (ix as f64) * dph;
            let ph1 = ph0 + dph;
            let w00 = winner[iy * n + ix];
            let w10 = winner[iy * n + ix + 1];
            let w01 = winner[(iy + 1) * n + ix];
            let w11 = winner[(iy + 1) * n + ix + 1];
            if w00 == w10 && w00 == w01 && w00 == w11 {
                continue;
            }
            // Collect exact crossings on the 4 cell edges.
            let mut crossings: Vec<(usize, usize, (f64, f64))> = Vec::new();
            if w00 != w10 {
                let p = refine_crossing(
                    &main_species,
                    &sec_species,
                    &req.main_element,
                    &sec_elems,
                    t_k,
                    dg_h2o_j,
                    w00,
                    w10,
                    (ph0, eh0),
                    (ph1, eh0),
                );
                crossings.push((w00.min(w10), w00.max(w10), p));
            }
            if w01 != w11 {
                let p = refine_crossing(
                    &main_species,
                    &sec_species,
                    &req.main_element,
                    &sec_elems,
                    t_k,
                    dg_h2o_j,
                    w01,
                    w11,
                    (ph0, eh1),
                    (ph1, eh1),
                );
                crossings.push((w01.min(w11), w01.max(w11), p));
            }
            if w00 != w01 {
                let p = refine_crossing(
                    &main_species,
                    &sec_species,
                    &req.main_element,
                    &sec_elems,
                    t_k,
                    dg_h2o_j,
                    w00,
                    w01,
                    (ph0, eh0),
                    (ph0, eh1),
                );
                crossings.push((w00.min(w01), w00.max(w01), p));
            }
            if w10 != w11 {
                let p = refine_crossing(
                    &main_species,
                    &sec_species,
                    &req.main_element,
                    &sec_elems,
                    t_k,
                    dg_h2o_j,
                    w10,
                    w11,
                    (ph1, eh0),
                    (ph1, eh1),
                );
                crossings.push((w10.min(w11), w10.max(w11), p));
            }

            if crossings.len() == 2 && crossings[0].0 == crossings[1].0 && crossings[0].1 == crossings[1].1 {
                let (a, b, p_a) = crossings[0];
                let (_, _, p_b) = crossings[1];
                segments.push(BoundarySegment {
                    species_a: main_species[a].formula.clone(),
                    species_b: main_species[b].formula.clone(),
                    x1_ph: p_a.0,
                    y1_v: p_a.1 - scale_off,
                    x2_ph: p_b.0,
                    y2_v: p_b.1 - scale_off,
                });
            } else if crossings.len() >= 3 {
                // Triple point inside cell: connect each crossing to cell center.
                let cx = 0.5 * (ph0 + ph1);
                let cy = 0.5 * (eh0 + eh1) - scale_off;
                for (a, b, p_a) in crossings {
                    segments.push(BoundarySegment {
                        species_a: main_species[a].formula.clone(),
                        species_b: main_species[b].formula.clone(),
                        x1_ph: p_a.0,
                        y1_v: p_a.1 - scale_off,
                        x2_ph: cx,
                        y2_v: cy,
                    });
                }
            }
        }
    }

    // Compute region counts & interior label positions (pole-of-inaccessibility approximation).
    let mut counts = vec![0usize; main_species.len()];
    for &w in &winner {
        counts[w] += 1;
    }

    let mut active_map: HashMap<usize, usize> = HashMap::new();
    let mut regions = Vec::new();
    for (sp_idx, &cnt) in counts.iter().enumerate() {
        if cnt == 0 {
            continue;
        }
        // Compute mean position and refine to deepest interior cell.
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        for iy in 0..n {
            for ix in 0..n {
                if winner[iy * n + ix] == sp_idx {
                    sum_x += req.min_ph + (ix as f64) * dph;
                    sum_y += req.min_eh_v + (iy as f64) * deh;
                }
            }
        }
        let mut best_x = sum_x / cnt as f64;
        let mut best_y = sum_y / cnt as f64;
        // Find grid cell of `sp_idx` closest to centroid that is also surrounded by `sp_idx`.
        let mut best_score = f64::INFINITY;
        for iy in 1..(n - 1) {
            for ix in 1..(n - 1) {
                if winner[iy * n + ix] != sp_idx {
                    continue;
                }
                let is_interior = winner[iy * n + ix - 1] == sp_idx
                    && winner[iy * n + ix + 1] == sp_idx
                    && winner[(iy - 1) * n + ix] == sp_idx
                    && winner[(iy + 1) * n + ix] == sp_idx;
                let px = req.min_ph + (ix as f64) * dph;
                let py = req.min_eh_v + (iy as f64) * deh;
                let dist2 = (px - best_x).powi(2) + (py - best_y).powi(2)
                    + if is_interior { 0.0 } else { 100.0 };
                if dist2 < best_score {
                    best_score = dist2;
                    best_x = px;
                    best_y = py;
                }
            }
        }

        let area_no = regions.len() + 1;
        active_map.insert(sp_idx, area_no - 1);
        regions.push(PredominanceRegion {
            area_no,
            species: main_species[sp_idx].formula.clone(),
            phase: main_species[sp_idx].phase,
            delta_g_kcal_mol: main_species[sp_idx].delta_g_kcal,
            label_ph: best_x,
            label_eh_v: best_y,
            cell_count: cnt,
        });
    }

    let grid_region_idx = winner
        .iter()
        .map(|w| active_map.get(w).copied())
        .collect();

    // Water stability limits:
    // Upper: O2(g) + 4 H+ + 4 e- = 2 H2O  => E_SHE = (-2 * dG_H2O_J) / (4*F) - slope * pH
    // Lower: 2 H+ + 2 e- = H2(g)          => E_SHE = 0 - slope * pH
    let slope_v_per_ph = (R_JOULE * t_k * std::f64::consts::LN_10) / FARADAY_C_MOL;
    let e0_o2_she = (-2.0 * dg_h2o_j) / (4.0 * FARADAY_C_MOL);
    let water_upper_line = [
        (req.min_ph, e0_o2_she - slope_v_per_ph * req.min_ph - scale_off),
        (req.max_ph, e0_o2_she - slope_v_per_ph * req.max_ph - scale_off),
    ];
    let water_lower_line = [
        (req.min_ph, -slope_v_per_ph * req.min_ph - scale_off),
        (req.max_ph, -slope_v_per_ph * req.max_ph - scale_off),
    ];

    // Table rows
    let mut table_rows = Vec::with_capacity(main_species.len());
    for (sp_idx, sp) in main_species.iter().enumerate() {
        let area_no = active_map.get(&sp_idx).map(|i| i + 1);
        let segs = segments
            .iter()
            .filter(|s| s.species_a == sp.formula || s.species_b == sp.formula)
            .map(|s| (s.x1_ph, s.y1_v, s.x2_ph, s.y2_v))
            .collect();
        table_rows.push(SpeciesTableRow {
            formula: sp.formula.clone(),
            delta_g_kcal_mol: sp.delta_g_kcal,
            area_no,
            segments: segs,
        });
    }

    let mut sys_parts = vec![req.main_element.clone()];
    for s in &sec_elems {
        sys_parts.push(s.clone());
    }
    let title = format!("{} - System at {:.2} °C", sys_parts.join(" - "), t_k - 273.15);

    Ok(EhPhResult {
        title,
        t_k,
        dielectric_constant: eps,
        delta_g_h2o_kcal: dg_h2o_kcal,
        p_sat_h2o_bar,
        warnings,
        segments,
        regions,
        water_upper_line,
        water_lower_line,
        table_rows,
        grid_n: n,
        grid_region_idx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KelleyCoefficients, SpeciesThermoInterval};

    fn make_sp(formula: &str, phase: Phase, dg_kcal_298: f64) -> ChemicalSpecies {
        // Construct a species with H298 = dg_kcal_298, S298 = 0 so G(298.15) = dg_kcal_298.
        ChemicalSpecies {
            formula: formula.to_string(),
            name: formula.to_string(),
            phase,
            molecular_weight: 50.0,
            density_g_cm3: 1.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 298.15,
                t_max: 2000.0,
                phase,
                kelley: KelleyCoefficients {
                    a: 0.0,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: dg_kcal_298 * 1000.0 * CAL_TO_JOULE,
                s_or_delta_s_tr: 0.0,
                reference: "test".to_string(),
            }],
        }
    }

    #[test]
    fn water_stability_lines_at_25c() {
        let cu = make_sp("Cu", Phase::Solid, 0.0);
        let req = EhPhRequest {
            elements: vec![ElementSpec {
                symbol: "Cu".to_string(),
                molality: 1.0,
                pressure_bar: 1.0,
            }],
            main_element: "Cu".to_string(),
            species: vec![cu],
            t_k: 298.15,
            delta_g_h2o_kcal: Some(-56.678),
            grid_resolution: 60,
            ..Default::default()
        };
        let res = calculate_ehph(&req, None).unwrap();
        assert!((res.water_upper_line[0].1 - 1.2289).abs() < 0.002);
        assert!((res.water_lower_line[0].1 - 0.0).abs() < 1e-6);
        // Slope at 25 C is -0.05916 V/pH -> at pH 14: -0.828 V
        assert!((res.water_lower_line[1].1 + 0.8282).abs() < 0.002);
        assert!((res.dielectric_constant - 78.382).abs() < 0.01);
        assert!(res.warnings.is_empty());
    }

    #[test]
    fn cu_h2o_predominance_regions() {
        let species = vec![
            make_sp("Cu", Phase::Solid, 0.0),
            make_sp("Cu(+2a)", Phase::AqueousIon, 15.66),
            make_sp("Cu2O", Phase::Solid, -35.335),
            make_sp("Cu(OH)2", Phase::Solid, -89.079),
        ];
        let req = EhPhRequest {
            elements: vec![ElementSpec {
                symbol: "Cu".to_string(),
                molality: 1.0,
                pressure_bar: 1.0,
            }],
            main_element: "Cu".to_string(),
            species,
            t_k: 298.15,
            delta_g_h2o_kcal: Some(-56.678),
            grid_resolution: 100,
            ..Default::default()
        };
        let res = calculate_ehph(&req, None).unwrap();
        let names: Vec<&str> = res.regions.iter().map(|r| r.species.as_str()).collect();
        assert!(names.contains(&"Cu"));
        assert!(names.contains(&"Cu(+2a)"));
        assert!(names.contains(&"Cu(OH)2"));
        assert!(!res.segments.is_empty());
    }

    #[test]
    fn cu_s_h2o_includes_cu2s_and_pressure_warning() {
        let species = vec![
            make_sp("Cu", Phase::Solid, 0.0),
            make_sp("Cu(+2a)", Phase::AqueousIon, 15.66),
            make_sp("Cu2S", Phase::Solid, -20.6),
            make_sp("Cu(OH)2", Phase::Solid, -89.079),
            make_sp("HS(-a)", Phase::AqueousIon, 2.88),
            make_sp("SO4(-2a)", Phase::AqueousIon, -177.95),
        ];
        let req = EhPhRequest {
            elements: vec![
                ElementSpec {
                    symbol: "Cu".to_string(),
                    molality: 1.0,
                    pressure_bar: 1.0,
                },
                ElementSpec {
                    symbol: "S".to_string(),
                    molality: 1.0,
                    pressure_bar: 1.0,
                },
            ],
            main_element: "Cu".to_string(),
            species,
            t_k: 393.15, // 120 °C -> P_sat > 1 bar
            delta_g_h2o_kcal: Some(-54.0),
            grid_resolution: 80,
            ..Default::default()
        };
        let res = calculate_ehph(&req, None).unwrap();
        let names: Vec<&str> = res.regions.iter().map(|r| r.species.as_str()).collect();
        assert!(names.contains(&"Cu2S"));
        assert!(!res.warnings.is_empty());
    }
}
