use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use soryntel_core::CAL_TO_JOULE;
use soryntel_solver::parse_term;

use crate::{evaluate_species_si, sum_element_references, SpeciesRepository};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionMethod {
    Method1,
    Method2,
    Method3,
}

impl ConversionMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Method1 => "Method 1",
            Self::Method2 => "Method 2",
            Self::Method3 => "Method 3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesTypeFilter {
    None,
    Sulfides,
    Oxides,
    Sulfates,
    Carbonates,
    Chlorides,
    Silicates,
    Hydroxides,
}

impl SpeciesTypeFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Sulfides => "Sulfides",
            Self::Oxides => "Oxides",
            Self::Sulfates => "Sulfates",
            Self::Carbonates => "Carbonates",
            Self::Chlorides => "Chlorides",
            Self::Silicates => "Silicates",
            Self::Hydroxides => "Hydroxides",
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Sulfides => "sulfide",
            Self::Oxides => "oxide",
            Self::Sulfates => "sulfate",
            Self::Carbonates => "carbonate",
            Self::Chlorides => "chloride",
            Self::Silicates => "silicate",
            Self::Hydroxides => "hydroxide",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverterInputEntry {
    pub formula: String,
    pub wt_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverterOutputSpec {
    pub formula: String,
    pub target_wt_pct: Option<f64>,
    pub weight_coeff: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesConverterRequest {
    pub inputs: Vec<ConverterInputEntry>,
    pub outputs: Vec<ConverterOutputSpec>,
    pub method: ConversionMethod,
    pub exact_o: bool,
    pub exact_h: bool,
    pub set_norm_limit: bool,
    pub norm_limit: f64,
    pub small_meas_limit: f64,
    pub element_balance_limit_pct: f64,
    pub stability_type: SpeciesTypeFilter,
    pub stability_weighting_coef: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedElementRow {
    pub element: String,
    pub wt_pct: f64,
    pub exceeds_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertedSpeciesRow {
    pub formula: String,
    pub calculated_wt_pct: f64,
    pub target_wt_pct: Option<f64>,
    pub weight_coeff: f64,
    pub species_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementBalanceRow {
    pub element: String,
    pub input_kg: f64,
    pub output_kg: f64,
    pub balance_kg: f64,
    pub error_pct: f64,
    pub limit: f64,
    pub below_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesConverterResult {
    pub input_total_wt_pct: f64,
    pub calculated_input_elements: Vec<CalculatedElementRow>,
    pub calculated_input_total_wt_pct: f64,
    pub output_rows: Vec<ConvertedSpeciesRow>,
    pub output_total_wt_pct: f64,
    pub target_total_wt_pct: f64,
    pub others_wt_pct: f64,
    pub residual_error: f64,
    pub element_balance: Vec<ElementBalanceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeciesConverterError {
    EmptyInput,
    EmptyOutput,
    InvalidFormula(String),
    Method3RequiresMoreSpecies {
        output_species: usize,
        targets_plus_inputs: usize,
    },
}

impl std::fmt::Display for SpeciesConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "Input analysis is empty."),
            Self::EmptyOutput => write!(f, "Output species list is empty."),
            Self::InvalidFormula(s) => write!(f, "Invalid chemical formula or unknown element: '{s}'."),
            Self::Method3RequiresMoreSpecies {
                output_species,
                targets_plus_inputs,
            } => write!(
                f,
                "Method 3 requires N_output ({output_species}) > N_targets + N_input ({targets_plus_inputs})."
            ),
        }
    }
}

impl std::error::Error for SpeciesConverterError {}

/// Classify a formula into one of HSC's 7 stability weighting categories.
pub fn classify_species_type(formula: &str) -> &'static str {
    let Ok(parsed) = parse_term(formula) else {
        return "";
    };
    let elems = &parsed.elements;
    let has_o = elems.contains_key("O");
    let has_s = elems.contains_key("S");
    let has_c = elems.contains_key("C");
    let has_h = elems.contains_key("H");
    let has_cl = elems.contains_key("Cl");
    let has_si = elems.contains_key("Si");

    if has_s && !has_o && elems.len() >= 2 {
        "sulfide"
    } else if has_s && has_o && elems.get("O").copied().unwrap_or(0.0) >= 3.0 * elems.get("S").copied().unwrap_or(1.0) {
        "sulfate"
    } else if has_c && has_o && elems.get("O").copied().unwrap_or(0.0) >= 2.5 * elems.get("C").copied().unwrap_or(1.0) {
        "carbonate"
    } else if has_si && has_o && elems.len() >= 2 {
        "silicate"
    } else if has_cl && !has_o && elems.len() >= 2 {
        "chloride"
    } else if has_o && has_h && !has_s && !has_c && elems.len() >= 3 {
        "hydroxide"
    } else if has_o && !has_s && !has_c && !has_si && !has_h && elems.len() >= 2 {
        "oxide"
    } else {
        ""
    }
}

/// Compute elemental mass fractions `element -> fraction (0..1)` for any HSC formula.
pub fn element_mass_fractions(formula: &str) -> Result<BTreeMap<String, f64>, SpeciesConverterError> {
    let parsed = parse_term(formula.trim())
        .map_err(|_| SpeciesConverterError::InvalidFormula(formula.to_string()))?;
    if parsed.elements.is_empty() {
        return Err(SpeciesConverterError::InvalidFormula(formula.to_string()));
    }
    let mut masses = BTreeMap::new();
    let mut total_mass = 0.0_f64;
    for (el, count) in &parsed.elements {
        if el == "e-" {
            continue;
        }
        let aw = atomic_weight(el)
            .ok_or_else(|| SpeciesConverterError::InvalidFormula(formula.to_string()))?;
        let m = aw * (*count);
        total_mass += m;
        masses.insert(el.clone(), m);
    }
    if total_mass <= 0.0 {
        return Err(SpeciesConverterError::InvalidFormula(formula.to_string()));
    }
    for val in masses.values_mut() {
        *val /= total_mass;
    }
    Ok(masses)
}

/// Initialize elemental composition from mixed element/species input entries (Fig. 9).
pub fn initialize_input(
    entries: &[ConverterInputEntry],
) -> Result<(Vec<CalculatedElementRow>, f64), SpeciesConverterError> {
    let valid: Vec<&ConverterInputEntry> = entries
        .iter()
        .filter(|e| !e.formula.trim().is_empty())
        .collect();
    if valid.is_empty() {
        return Err(SpeciesConverterError::EmptyInput);
    }

    let mut elem_order = Vec::new();
    let mut elem_totals: BTreeMap<String, f64> = BTreeMap::new();
    let mut input_total = 0.0_f64;

    for entry in valid {
        input_total += entry.wt_pct;
        let fracs = element_mass_fractions(&entry.formula)?;
        for (el, frac) in fracs {
            if !elem_totals.contains_key(&el) {
                elem_order.push(el.clone());
            }
            *elem_totals.entry(el).or_insert(0.0) += entry.wt_pct * frac;
        }
    }

    let rows = elem_order
        .into_iter()
        .map(|el| CalculatedElementRow {
            wt_pct: elem_totals.get(&el).copied().unwrap_or(0.0),
            element: el,
            exceeds_limit: false,
        })
        .collect();

    Ok((rows, input_total))
}

/// Append pure elements from the input analysis to the output list with `Weight Coeff = 0.100` (Fig. 5).
pub fn fix_element_balance(
    inputs: &[ConverterInputEntry],
    outputs: &mut Vec<ConverterOutputSpec>,
) -> Result<(), SpeciesConverterError> {
    let (elems, _) = initialize_input(inputs)?;
    let existing: BTreeSet<String> = outputs
        .iter()
        .map(|o| o.formula.trim().to_string())
        .collect();
    for row in elems {
        if !existing.contains(&row.element) {
            outputs.push(ConverterOutputSpec {
                formula: row.element,
                target_wt_pct: None,
                weight_coeff: 0.100,
            });
        }
    }
    Ok(())
}

/// Solve species conversion using NNLS (Methods 1, 2, and 3).
pub fn solve_species_conversion(
    repo: &dyn SpeciesRepository,
    req: &SpeciesConverterRequest,
) -> Result<SpeciesConverterResult, SpeciesConverterError> {
    let (init_elems, input_total) = initialize_input(&req.inputs)?;
    let valid_outputs: Vec<&ConverterOutputSpec> = req
        .outputs
        .iter()
        .filter(|o| !o.formula.trim().is_empty() && o.formula.trim() != "[Others]")
        .collect();
    if valid_outputs.is_empty() {
        return Err(SpeciesConverterError::EmptyOutput);
    }

    let n_targets = valid_outputs
        .iter()
        .filter(|o| o.target_wt_pct.is_some())
        .count();
    let n_inputs = req
        .inputs
        .iter()
        .filter(|i| !i.formula.trim().is_empty())
        .count();
    if req.method == ConversionMethod::Method3 && valid_outputs.len() <= n_targets + n_inputs {
        return Err(SpeciesConverterError::Method3RequiresMoreSpecies {
            output_species: valid_outputs.len(),
            targets_plus_inputs: n_targets + n_inputs,
        });
    }

    let mut input_map: BTreeMap<String, f64> = BTreeMap::new();
    let mut elem_list: Vec<String> = Vec::new();
    for r in &init_elems {
        elem_list.push(r.element.clone());
        input_map.insert(r.element.clone(), r.wt_pct);
    }
    for oh in ["O", "H"] {
        if !input_map.contains_key(oh) {
            elem_list.push(oh.to_string());
            input_map.insert(oh.to_string(), 0.0);
        }
    }

    // Disregard species containing elements absent from input analysis (except O and H).
    let mut active_indices = Vec::new();
    let mut out_fracs = Vec::with_capacity(valid_outputs.len());
    for (idx, out) in valid_outputs.iter().enumerate() {
        let fracs = element_mass_fractions(&out.formula)?;
        let allowed = fracs
            .keys()
            .all(|el| el == "O" || el == "H" || input_map.get(el).copied().unwrap_or(0.0) > 0.0);
        if allowed {
            active_indices.push(idx);
        }
        out_fracs.push(fracs);
    }

    // Build NNLS linear system M * x ≈ y (x >= 0) over active output species.
    let s_act = active_indices.len();
    let mut rows_m: Vec<Vec<f64>> = Vec::new();
    let mut rhs_y: Vec<f64> = Vec::new();

    if s_act > 0 {
        for el in &elem_list {
            let meas = input_map.get(el).copied().unwrap_or(0.0);
            let is_o_or_h = el == "O" || el == "H";
            let include_eq = if is_o_or_h {
                (el == "O" && req.exact_o) || (el == "H" && req.exact_h) || meas > 1e-12
            } else {
                meas > 1e-12
            };
            if !include_eq {
                continue;
            }
            let w_elem = if is_o_or_h && meas <= 1e-12 { 0.25 } else { 1.0 };
            let mut row = Vec::with_capacity(s_act);
            for &orig_idx in &active_indices {
                row.push(w_elem * out_fracs[orig_idx].get(el).copied().unwrap_or(0.0));
            }
            rows_m.push(row);
            rhs_y.push(w_elem * meas);
        }

        // Total mass conservation row (soft constraint)
        let tot_w = if req.method == ConversionMethod::Method1 && req.set_norm_limit {
            0.25 / req.norm_limit.max(1e-4).sqrt().min(1.0)
        } else {
            0.35
        };
        rows_m.push(vec![tot_w; s_act]);
        rhs_y.push(tot_w * input_total);

        // Target wt-% equations & Stability Weighting / Weight Coeff regularization
        for (col, &orig_idx) in active_indices.iter().enumerate() {
            let spec = valid_outputs[orig_idx];
            if let Some(target) = spec.target_wt_pct {
                let w = spec.weight_coeff.max(0.01);
                let mut row = vec![0.0; s_act];
                row[col] = w;
                rows_m.push(row);
                rhs_y.push(w * target);
            } else if req.method == ConversionMethod::Method1 {
                let w = (spec.weight_coeff - 1.0).abs().max(0.005) * 0.05;
                let mut row = vec![0.0; s_act];
                row[col] = w;
                rows_m.push(row);
                rhs_y.push(0.0);
            } else if req.method == ConversionMethod::Method3 {
                let w = 0.01 / spec.weight_coeff.max(0.01);
                let mut row = vec![0.0; s_act];
                row[col] = w;
                rows_m.push(row);
                rhs_y.push(0.0);
            }

            // Stability Weighting (delta G at 298.15 K)
            if req.stability_type != SpeciesTypeFilter::None {
                let stype = classify_species_type(&spec.formula);
                if stype == req.stability_type.tag() {
                    if let Some(sp) = repo.get_species(&spec.formula) {
                        if let Ok((pt, _)) = evaluate_species_si(&sp, 298.15, false, false) {
                            let dg_elem = parse_term(&spec.formula)
                                .ok()
                                .and_then(|t| sum_element_references(&t, 298.15, false, repo).ok())
                                .map(|(_, _, g, _)| g)
                                .unwrap_or(0.0);
                            let dg_kcal = (pt.g - dg_elem) / (1000.0 * CAL_TO_JOULE);
                            let bonus = (-dg_kcal / req.stability_weighting_coef.max(1.0)).clamp(0.0, 10.0);
                            let mut row = vec![0.0; s_act];
                            row[col] = 0.08;
                            rows_m.push(row);
                            rhs_y.push(0.08 * bonus);
                        }
                    }
                }
            }
        }
    }

    let x_act = if s_act > 0 {
        solve_nnls(&rows_m, &rhs_y)
    } else {
        Vec::new()
    };

    let mut x_full = vec![0.0_f64; valid_outputs.len()];
    for (col, &orig_idx) in active_indices.iter().enumerate() {
        x_full[orig_idx] = x_act[col];
    }

    // Output rows
    let mut output_rows = Vec::with_capacity(valid_outputs.len());
    let mut output_total = 0.0_f64;
    let mut target_total = 0.0_f64;
    let mut others_wt_pct = 0.0_f64;
    for (idx, spec) in valid_outputs.iter().enumerate() {
        let val = x_full[idx];
        output_total += val;
        if let Some(t) = spec.target_wt_pct {
            target_total += t;
        }
        if val > 0.0 && val < req.small_meas_limit {
            others_wt_pct += val;
        }
        output_rows.push(ConvertedSpeciesRow {
            formula: spec.formula.clone(),
            calculated_wt_pct: val,
            target_wt_pct: spec.target_wt_pct,
            weight_coeff: spec.weight_coeff,
            species_type: classify_species_type(&spec.formula).to_string(),
        });
    }

    // Calculated elemental composition from output
    let mut calc_elem_map: BTreeMap<String, f64> = BTreeMap::new();
    for (idx, val) in x_full.iter().enumerate() {
        for (el, frac) in &out_fracs[idx] {
            *calc_elem_map.entry(el.clone()).or_insert(0.0) += val * frac;
        }
    }

    let mut calc_elems = Vec::new();
    let mut calc_total = 0.0_f64;
    let mut residual_error = 0.0_f64;
    let mut element_balance = Vec::new();

    for el in &elem_list {
        let out_wt = calc_elem_map.get(el).copied().unwrap_or(0.0);
        let in_wt = input_map.get(el).copied().unwrap_or(0.0);
        calc_total += out_wt;
        if in_wt > 1e-12 {
            residual_error += (out_wt - in_wt).abs();
            let input_kg = 10.0 * in_wt;
            let output_kg = 10.0 * out_wt;
            let balance_kg = output_kg - input_kg;
            let error_pct = (balance_kg.abs() / input_kg.max(1e-12)) * 100.0;
            let limit = 100.0 - error_pct;
            let below_limit = limit < req.element_balance_limit_pct;
            element_balance.push(ElementBalanceRow {
                element: el.clone(),
                input_kg,
                output_kg,
                balance_kg,
                error_pct,
                limit,
                below_limit,
            });
            calc_elems.push(CalculatedElementRow {
                element: el.clone(),
                wt_pct: out_wt,
                exceeds_limit: below_limit,
            });
        } else {
            calc_elems.push(CalculatedElementRow {
                element: el.clone(),
                wt_pct: out_wt,
                exceeds_limit: false,
            });
        }
    }

    Ok(SpeciesConverterResult {
        input_total_wt_pct: input_total,
        calculated_input_elements: calc_elems,
        calculated_input_total_wt_pct: calc_total,
        output_rows,
        output_total_wt_pct: output_total,
        target_total_wt_pct: target_total,
        others_wt_pct,
        residual_error,
        element_balance,
    })
}

/// Lawson–Hanson active-set NNLS solver for `min_{x >= 0} ||M x - y||_2^2`.
fn solve_nnls(m: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
    let n_rows = m.len();
    let n_cols = m.first().map(|r| r.len()).unwrap_or(0);
    if n_rows == 0 || n_cols == 0 {
        return vec![0.0; n_cols];
    }

    let mut x = vec![0.0_f64; n_cols];
    let mut passive = vec![false; n_cols];
    let max_iter = 5 * n_cols + 40;

    for _ in 0..max_iter {
        // Compute gradient w = M^T (y - M x)
        let mut res = vec![0.0_f64; n_rows];
        for i in 0..n_rows {
            let mut mx = 0.0;
            for j in 0..n_cols {
                mx += m[i][j] * x[j];
            }
            res[i] = y[i] - mx;
        }
        let mut best_j = None;
        let mut best_w = 1e-10_f64;
        for j in 0..n_cols {
            if passive[j] {
                continue;
            }
            let mut wj = 0.0;
            for i in 0..n_rows {
                wj += m[i][j] * res[i];
            }
            if wj > best_w {
                best_w = wj;
                best_j = Some(j);
            }
        }
        let Some(enter) = best_j else {
            break;
        };
        passive[enter] = true;

        // Inner active-set loop
        for _ in 0..max_iter {
            let p_cols: Vec<usize> = (0..n_cols).filter(|&j| passive[j]).collect();
            if p_cols.is_empty() {
                break;
            }
            let z_p = solve_normal_equations(m, y, &p_cols);
            let mut all_pos = true;
            for &val in &z_p {
                if val <= 1e-12 {
                    all_pos = false;
                    break;
                }
            }
            if all_pos {
                for (k, &j) in p_cols.iter().enumerate() {
                    x[j] = z_p[k];
                }
                break;
            }
            let mut alpha = f64::INFINITY;
            for (k, &j) in p_cols.iter().enumerate() {
                if z_p[k] <= 1e-12 {
                    let denom = x[j] - z_p[k];
                    if denom > 1e-15 {
                        alpha = alpha.min(x[j] / denom);
                    }
                }
            }
            if !alpha.is_finite() {
                break;
            }
            for (k, &j) in p_cols.iter().enumerate() {
                x[j] += alpha * (z_p[k] - x[j]);
                if x[j].abs() <= 1e-12 {
                    x[j] = 0.0;
                    passive[j] = false;
                }
            }
        }
    }
    x
}

fn solve_normal_equations(m: &[Vec<f64>], y: &[f64], cols: &[usize]) -> Vec<f64> {
    let k = cols.len();
    let mut a = vec![vec![0.0_f64; k]; k];
    let mut b = vec![0.0_f64; k];
    for i in 0..m.len() {
        for r in 0..k {
            let mr = m[i][cols[r]];
            b[r] += mr * y[i];
            for c in 0..k {
                a[r][c] += mr * m[i][cols[c]];
            }
        }
    }
    for r in 0..k {
        a[r][r] += 1e-12;
    }
    // Gaussian elimination with partial pivoting
    for piv in 0..k {
        let mut best_row = piv;
        let mut best_val = a[piv][piv].abs();
        for r in (piv + 1)..k {
            if a[r][piv].abs() > best_val {
                best_val = a[r][piv].abs();
                best_row = r;
            }
        }
        a.swap(piv, best_row);
        b.swap(piv, best_row);
        let diag = a[piv][piv];
        if diag.abs() < 1e-15 {
            continue;
        }
        for r in (piv + 1)..k {
            let f = a[r][piv] / diag;
            for c in piv..k {
                a[r][c] -= f * a[piv][c];
            }
            b[r] -= f * b[piv];
        }
    }
    let mut sol = vec![0.0_f64; k];
    for r in (0..k).rev() {
        let mut s = b[r];
        for c in (r + 1)..k {
            s -= a[r][c] * sol[c];
        }
        sol[r] = if a[r][r].abs() > 1e-15 {
            s / a[r][r]
        } else {
            0.0
        };
    }
    sol
}

/// Standard IUPAC atomic weights (g/mol).
pub fn atomic_weight(symbol: &str) -> Option<f64> {
    let w = match symbol {
        "H" => 1.00794,
        "He" => 4.002602,
        "Li" => 6.941,
        "Be" => 9.012182,
        "B" => 10.811,
        "C" => 12.0107,
        "N" => 14.0067,
        "O" => 15.9994,
        "F" => 18.9984032,
        "Ne" => 20.1797,
        "Na" => 22.989769,
        "Mg" => 24.3050,
        "Al" => 26.9815386,
        "Si" => 28.0855,
        "P" => 30.973762,
        "S" => 32.065,
        "Cl" => 35.453,
        "Ar" => 39.948,
        "K" => 39.0983,
        "Ca" => 40.078,
        "Sc" => 44.955912,
        "Ti" => 47.867,
        "V" => 50.9415,
        "Cr" => 51.9961,
        "Mn" => 54.938045,
        "Fe" => 55.845,
        "Co" => 58.933195,
        "Ni" => 58.6934,
        "Cu" => 63.546,
        "Zn" => 65.38,
        "Ga" => 69.723,
        "Ge" => 72.64,
        "As" => 74.9216,
        "Se" => 78.96,
        "Br" => 79.904,
        "Kr" => 83.798,
        "Rb" => 85.4678,
        "Sr" => 87.62,
        "Y" => 88.90585,
        "Zr" => 91.224,
        "Nb" => 92.90638,
        "Mo" => 95.96,
        "Tc" => 98.0,
        "Ru" => 101.07,
        "Rh" => 102.9055,
        "Pd" => 106.42,
        "Ag" => 107.8682,
        "Cd" => 112.411,
        "In" => 114.818,
        "Sn" => 118.710,
        "Sb" => 121.760,
        "Te" => 127.60,
        "I" => 126.90447,
        "Xe" => 131.293,
        "Cs" => 132.90545,
        "Ba" => 137.327,
        "La" => 138.90547,
        "Ce" => 140.116,
        "Pr" => 140.90765,
        "Nd" => 144.242,
        "Sm" => 150.36,
        "Eu" => 151.964,
        "Gd" => 157.25,
        "Tb" => 158.92535,
        "Dy" => 162.500,
        "Ho" => 164.93032,
        "Er" => 167.259,
        "Tm" => 168.93421,
        "Yb" => 173.054,
        "Lu" => 174.9668,
        "Hf" => 178.49,
        "Ta" => 180.94788,
        "W" => 183.84,
        "Re" => 186.207,
        "Os" => 190.23,
        "Ir" => 192.217,
        "Pt" => 195.084,
        "Au" => 196.966569,
        "Hg" => 200.59,
        "Tl" => 204.3833,
        "Pb" => 207.2,
        "Bi" => 208.98040,
        "Th" => 232.03806,
        "U" => 238.02891,
        _ => return None,
    };
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChemicalSpecies;

    struct EmptyRepo;
    impl SpeciesRepository for EmptyRepo {
        fn get_species(&self, _formula: &str) -> Option<ChemicalSpecies> {
            None
        }
        fn get_reference_element(&self, _element_symbol: &str) -> Option<ChemicalSpecies> {
            None
        }
    }

    #[test]
    fn initialize_mixed_cu_species_matches_fig9() {
        let entries = vec![
            ConverterInputEntry { formula: "Cu".into(), wt_pct: 25.0 },
            ConverterInputEntry { formula: "CuO".into(), wt_pct: 25.0 },
            ConverterInputEntry { formula: "Cu2S".into(), wt_pct: 25.0 },
            ConverterInputEntry { formula: "CuSO4".into(), wt_pct: 25.0 },
        ];
        let (rows, total) = initialize_input(&entries).expect("initialize should succeed");
        assert!((total - 100.0).abs() < 1e-6);
        let get = |e: &str| rows.iter().find(|r| r.element == e).map(|r| r.wt_pct).unwrap_or(0.0);
        assert!((get("Cu") - 74.889).abs() < 0.05);
        assert!((get("O") - 15.053).abs() < 0.05);
        assert!((get("S") - 10.058).abs() < 0.05);
    }

    #[test]
    fn solve_cu_s_fe_ni_and_fix_element_balance() {
        let mut req = SpeciesConverterRequest {
            inputs: vec![
                ConverterInputEntry { formula: "Cu".into(), wt_pct: 25.0 },
                ConverterInputEntry { formula: "S".into(), wt_pct: 40.0 },
                ConverterInputEntry { formula: "Fe".into(), wt_pct: 10.0 },
                ConverterInputEntry { formula: "Ni".into(), wt_pct: 25.0 },
            ],
            outputs: vec![
                ConverterOutputSpec { formula: "CuS".into(), target_wt_pct: None, weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "Cu2S".into(), target_wt_pct: None, weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "CuFeS2".into(), target_wt_pct: None, weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "FeS2".into(), target_wt_pct: None, weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "NiS".into(), target_wt_pct: None, weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "Ni3S2".into(), target_wt_pct: None, weight_coeff: 1.0 },
            ],
            method: ConversionMethod::Method1,
            exact_o: true,
            exact_h: true,
            set_norm_limit: false,
            norm_limit: 0.0001,
            small_meas_limit: 0.5,
            element_balance_limit_pct: 99.0,
            stability_type: SpeciesTypeFilter::None,
            stability_weighting_coef: 80.0,
        };

        let res1 = solve_species_conversion(&EmptyRepo, &req).expect("Method 1 should solve");
        assert!((res1.output_total_wt_pct - 100.0).abs() < 1.0);
        assert!(!res1.element_balance.is_empty());

        fix_element_balance(&req.inputs, &mut req.outputs).expect("fix balance should succeed");
        let res2 = solve_species_conversion(&EmptyRepo, &req).expect("Method 1 after fix should solve");
        assert!(res2.residual_error <= res1.residual_error + 1e-6);
    }

    #[test]
    fn method3_rejects_underdetermined_constraint_violation() {
        let req = SpeciesConverterRequest {
            inputs: vec![
                ConverterInputEntry { formula: "Cu".into(), wt_pct: 50.0 },
                ConverterInputEntry { formula: "S".into(), wt_pct: 50.0 },
            ],
            outputs: vec![
                ConverterOutputSpec { formula: "CuS".into(), target_wt_pct: Some(50.0), weight_coeff: 1.0 },
                ConverterOutputSpec { formula: "Cu2S".into(), target_wt_pct: None, weight_coeff: 1.0 },
            ],
            method: ConversionMethod::Method3,
            exact_o: true,
            exact_h: true,
            set_norm_limit: false,
            norm_limit: 0.0001,
            small_meas_limit: 0.5,
            element_balance_limit_pct: 99.0,
            stability_type: SpeciesTypeFilter::None,
            stability_weighting_coef: 80.0,
        };

        let err = solve_species_conversion(&EmptyRepo, &req).unwrap_err();
        assert!(matches!(err, SpeciesConverterError::Method3RequiresMoreSpecies { .. }));
    }
}
