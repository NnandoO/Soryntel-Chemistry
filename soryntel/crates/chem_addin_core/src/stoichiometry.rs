use crate::error::{ChemAddinError, Result};
use crate::parser::{parse_reaction, parse_species, ParsedReaction, ReactionTerm};
use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Detailed result returned by `bal_detailed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceResult {
    pub equation: String,
    pub reactants: Vec<(f64, String)>,
    pub products: Vec<(f64, String)>,
    pub coefficients: Vec<f64>,
}

/// Calculate exact molar mass (g/mol) of a chemical species formula
/// (supporting phase tags like `(g)`, nested brackets, hydrates `*`, and charges).
pub fn mw(species: &str) -> Result<f64> {
    let parsed = parse_species(species)?;
    parsed.molar_mass()
}

/// Calculate the weight percentage (wt-%) of `species1` contained in `species2`.
///
/// Example: `wtp("V", "VO")` = `100 * 50.9415 / (50.9415 + 15.999) = 76.100... %`.
/// Also supports compound moieties such as `wtp("ZnO", "ZnO*Fe2O3")`.
pub fn wtp(species1: &str, species2: &str) -> Result<f64> {
    let p1 = parse_species(species1)?;
    let p2 = parse_species(species2)?;

    let mw1 = p1.molar_mass()?;
    let mw2 = p2.molar_mass()?;
    if mw2 <= 1e-12 {
        return Err(ChemAddinError::Numeric(format!(
            "Host species '{species2}' has zero molar mass"
        )));
    }

    let mut limiting_moles = f64::INFINITY;
    for (elem, &need) in &p1.composition {
        if need <= 0.0 {
            continue;
        }
        let avail = p2.composition.get(elem).copied().unwrap_or(0.0);
        limiting_moles = limiting_moles.min(avail / need);
    }

    if !limiting_moles.is_finite() || limiting_moles <= 0.0 {
        return Ok(0.0);
    }

    Ok(100.0 * (limiting_moles * mw1) / mw2)
}

/// Balance a chemical reaction using an elemental conservation matrix `A` and SVD nullspace
/// in `nalgebra`, normalize so the smallest stoichiometric coefficient is `1.0`, and scale
/// by `multiplier`.
pub fn bal(reaction: &str, multiplier: f64) -> Result<String> {
    Ok(bal_detailed(reaction, multiplier)?.equation)
}

/// Full structured version of [`bal`] returning both formatted string and solved coefficients.
pub fn bal_detailed(reaction: &str, multiplier: f64) -> Result<BalanceResult> {
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(ChemAddinError::InvalidArgument(format!(
            "BAL multiplier must be finite and > 0 (got {multiplier})"
        )));
    }

    let parsed = parse_reaction(reaction)?;
    let n_react = parsed.reactants.len();
    let n_prod = parsed.products.len();
    let n_cols = n_react + n_prod;
    if n_react == 0 || n_prod == 0 {
        return Err(ChemAddinError::InvalidReaction(reaction.to_string()));
    }

    // Collect distinct conserved species attributes (elements + optional charge balance row).
    let mut elem_index = BTreeMap::<String, usize>::new();
    let mut has_charge = false;
    for term in parsed.reactants.iter().chain(parsed.products.iter()) {
        if term.species.charge != 0 {
            has_charge = true;
        }
        for elem in term.species.composition.keys() {
            let next = elem_index.len();
            elem_index.entry(elem.clone()).or_insert(next);
        }
    }

    let m_rows = elem_index.len() + usize::from(has_charge);
    if m_rows == 0 {
        return Err(ChemAddinError::InvalidReaction(reaction.to_string()));
    }

    // Pad with zero rows to at least `n_cols` so `nalgebra::SVD` yields a full `n_cols x n_cols` V^T.
    let padded_rows = m_rows.max(n_cols);
    let mut a = DMatrix::<f64>::zeros(padded_rows, n_cols);

    for (j, term) in parsed.reactants.iter().enumerate() {
        for (elem, &cnt) in &term.species.composition {
            a[(elem_index[elem], j)] = cnt;
        }
        if has_charge {
            a[(m_rows - 1, j)] = term.species.charge as f64;
        }
    }
    for (k, term) in parsed.products.iter().enumerate() {
        let col = n_react + k;
        for (elem, &cnt) in &term.species.composition {
            a[(elem_index[elem], col)] = -cnt;
        }
        if has_charge {
            a[(m_rows - 1, col)] = -(term.species.charge as f64);
        }
    }

    let svd = a.clone().svd(false, true);
    let v_t = svd
        .v_t
        .ok_or_else(|| ChemAddinError::Numeric("Failed to compute SVD right singular vectors".into()))?;

    let sigma_max = svd.singular_values.iter().copied().fold(0.0_f64, f64::max);
    let tol = 1e-9 * (padded_rows.max(n_cols) as f64) * sigma_max.max(1.0);

    let mut null_indices = Vec::new();
    for (idx, &sv) in svd.singular_values.iter().enumerate() {
        if sv <= tol {
            null_indices.push(idx);
        }
    }

    if null_indices.is_empty() {
        return Err(ChemAddinError::UnbalanceableReaction(format!(
            "No non-trivial stoichiometric solution exists for '{reaction}'"
        )));
    }
    if null_indices.len() > 1 {
        return Err(ChemAddinError::UnbalanceableReaction(format!(
            "Underdetermined reaction '{reaction}' (nullspace dimension = {})",
            null_indices.len()
        )));
    }

    let row_idx = null_indices[0];
    let mut raw_vec: Vec<f64> = v_t.row(row_idx).iter().copied().collect();

    // Flip orientation if the leading component is negative.
    let anchor = raw_vec
        .iter()
        .copied()
        .find(|v| v.abs() > 1e-9)
        .ok_or_else(|| ChemAddinError::UnbalanceableReaction(reaction.to_string()))?;
    if anchor < 0.0 {
        for v in &mut raw_vec {
            *v = -*v;
        }
    }

    // Ensure every reactant & product gets a strictly positive stoichiometric coefficient.
    if raw_vec.iter().any(|&v| v <= 1e-8) {
        return Err(ChemAddinError::UnbalanceableReaction(format!(
            "Species appear on incompatible sides or are spectator-zero in '{reaction}'"
        )));
    }

    // Normalize relative to smallest magnitude, snap to clean integers when rational, then apply multiplier.
    let min_raw = raw_vec.iter().copied().fold(f64::INFINITY, f64::min);
    let unit_ratios: Vec<f64> = raw_vec.iter().map(|&v| v / min_raw).collect();
    let coeffs = clean_stoichiometric_ratios(&unit_ratios, multiplier);

    // Double-check matrix residual A * coeffs ~ 0.
    let coeff_col = DMatrix::from_column_slice(n_cols, 1, &coeffs);
    let residual = &a.rows(0, m_rows) * coeff_col;
    let max_res = residual.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    if max_res > 1e-6 * multiplier.max(1.0) {
        return Err(ChemAddinError::UnbalanceableReaction(format!(
            "Conservation check residual too large ({max_res:.3e}) for '{reaction}'"
        )));
    }

    let reactants: Vec<(f64, String)> = parsed
        .reactants
        .iter()
        .enumerate()
        .map(|(j, t)| (coeffs[j], t.species.raw.clone()))
        .collect();
    let products: Vec<(f64, String)> = parsed
        .products
        .iter()
        .enumerate()
        .map(|(k, t)| (coeffs[n_react + k], t.species.raw.clone()))
        .collect();

    let lhs_fmt = reactants
        .iter()
        .map(|(c, sp)| format_term(*c, sp))
        .collect::<Vec<_>>()
        .join(" + ");
    let rhs_fmt = products
        .iter()
        .map(|(c, sp)| format_term(*c, sp))
        .collect::<Vec<_>>()
        .join(" + ");

    Ok(BalanceResult {
        equation: format!("{lhs_fmt} = {rhs_fmt}"),
        reactants,
        products,
        coefficients: coeffs,
    })
}

/// Verify whether explicit/parsed reaction coefficients conserve all elements and charge.
pub fn verify_reaction_balance(parsed: &ParsedReaction) -> Result<()> {
    let mut elem_diff = BTreeMap::<String, f64>::new();
    let mut charge_diff = 0.0_f64;

    let accumulate = |terms: &[ReactionTerm],
                      sign: f64,
                      map: &mut BTreeMap<String, f64>,
                      q: &mut f64| {
        for t in terms {
            for (el, &cnt) in &t.species.composition {
                *map.entry(el.clone()).or_insert(0.0) += sign * t.coeff * cnt;
            }
            *q += sign * t.coeff * (t.species.charge as f64);
        }
    };

    accumulate(&parsed.reactants, -1.0, &mut elem_diff, &mut charge_diff);
    accumulate(&parsed.products, 1.0, &mut elem_diff, &mut charge_diff);

    for (elem, diff) in elem_diff {
        if diff.abs() > 1e-5 {
            return Err(ChemAddinError::UnbalanceableReaction(format!(
                "Element '{elem}' unbalanced by {diff:.6}"
            )));
        }
    }
    if charge_diff.abs() > 1e-5 {
        return Err(ChemAddinError::UnbalanceableReaction(format!(
            "Net charge unbalanced by {charge_diff:.6}"
        )));
    }
    Ok(())
}

fn clean_stoichiometric_ratios(unit_ratios: &[f64], multiplier: f64) -> Vec<f64> {
    // Try snapping all ratios (relative to min = 1) to exact rationals via continued fractions.
    if let Some(rats) = unit_ratios
        .iter()
        .map(|&x| approx_rational(x, 10_000, 1e-6))
        .collect::<Option<Vec<(i64, i64)>>>()
    {
        let common_den = rats.iter().fold(1i64, |acc, &(_, d)| lcm_i64(acc, d));
        let mut ints: Vec<i64> = rats
            .iter()
            .map(|&(n, d)| n.saturating_mul(common_den / d))
            .collect();
        let common_gcd = ints.iter().fold(0i64, |acc, &v| gcd_i64(acc, v)).max(1);
        for v in &mut ints {
            *v /= common_gcd;
        }
        let min_int = *ints.iter().min().unwrap_or(&1) as f64;
        let scale = multiplier / min_int;
        return ints.into_iter().map(|v| (v as f64) * scale).collect();
    }

    unit_ratios.iter().map(|&r| r * multiplier).collect()
}

fn approx_rational(x: f64, max_den: i64, tol: f64) -> Option<(i64, i64)> {
    if !x.is_finite() || x <= 0.0 {
        return None;
    }
    let rounded = x.round();
    if (x - rounded).abs() <= tol && rounded >= 1.0 && rounded <= (max_den as f64) {
        return Some((rounded as i64, 1));
    }

    // Continued fraction expansion capped at `max_den`.
    let mut a = x.floor();
    let mut h1 = 1i64;
    let mut k1 = 0i64;
    let mut h = a as i64;
    let mut k = 1i64;
    let mut frac = x;

    for _ in 0..32 {
        let approx = (h as f64) / (k as f64);
        if (x - approx).abs() <= tol * x.max(1.0) {
            return Some((h, k));
        }
        let rem = frac - a;
        if rem.abs() < 1e-12 {
            break;
        }
        frac = 1.0 / rem;
        a = frac.floor();
        if a > 1e9 {
            break;
        }
        let ai = a as i64;
        let next_h = ai.saturating_mul(h).saturating_add(h1);
        let next_k = ai.saturating_mul(k).saturating_add(k1);
        if next_k > max_den {
            break;
        }
        h1 = h;
        k1 = k;
        h = next_h;
        k = next_k;
    }

    let final_approx = (h as f64) / (k as f64);
    if (x - final_approx).abs() <= tol * x.max(1.0) {
        Some((h, k))
    } else {
        None
    }
}

fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn lcm_i64(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    (a / gcd_i64(a, b)).saturating_mul(b)
}

fn format_term(coeff: f64, species: &str) -> String {
    if (coeff - 1.0).abs() < 1e-9 {
        return species.to_string();
    }
    let rounded = coeff.round();
    if (coeff - rounded).abs() < 1e-9 && rounded >= 1.0 {
        return format!("{} {}", rounded as i64, species);
    }
    let s = format!("{coeff:.6}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed} {species}")
}
