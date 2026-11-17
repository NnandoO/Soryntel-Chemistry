//! Chemical formula parser, reaction parser, and exact rational stoichiometric balancer.

use serde::{Deserialize, Serialize};
use soryntel_core::SoryntelError;
use std::collections::BTreeMap;

/// Parsed representation of a single chemical species term in a formula or reaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedTerm {
    /// Original species token without leading stoichiometric coefficient (e.g. "SO4(-2a)", "CO(g)", "e-").
    pub formula: String,
    /// Stoichiometric coefficient (unsigned; default 1.0 if omitted).
    pub coeff: f64,
    /// Element counts (e.g. {"S": 1.0, "O": 4.0}). Empty for "e-".
    pub elements: BTreeMap<String, f64>,
    /// Net electric charge (e.g. -2 for SO4(-2a), +1 for H(+a), -1 for e-).
    pub charge: i32,
    /// True if this term is the free electron `e-` / `e(-)`.
    pub is_electron: bool,
}

/// Parsed chemical reaction or single substance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedReaction {
    pub is_single_substance: bool,
    pub reactants: Vec<ParsedTerm>,
    pub products: Vec<ParsedTerm>,
}

/// Parse a reaction equation or single chemical formula.
pub fn parse_input(raw: &str) -> Result<ParsedReaction, SoryntelError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SoryntelError::ParseError(
            "Input formula or reaction cannot be empty".into(),
        ));
    }

    let eq_parts: Option<(&str, &str)> = if let Some(idx) = trimmed.find("<=>") {
        Some((&trimmed[..idx], &trimmed[idx + 3..]))
    } else if let Some(idx) = trimmed.find("->") {
        Some((&trimmed[..idx], &trimmed[idx + 2..]))
    } else if let Some(idx) = trimmed.find('=') {
        Some((&trimmed[..idx], &trimmed[idx + 1..]))
    } else {
        None
    };

    match eq_parts {
        None => {
            let term = parse_term(trimmed)?;
            Ok(ParsedReaction {
                is_single_substance: true,
                reactants: vec![term],
                products: vec![],
            })
        }
        Some((lhs, rhs)) => {
            if rhs.contains('=') || rhs.contains("->") {
                return Err(SoryntelError::ParseError(
                    "Multiple '=' or '->' operators in reaction".into(),
                ));
            }
            let reactants = split_side(lhs)?;
            let products = split_side(rhs)?;
            if reactants.is_empty() || products.is_empty() {
                return Err(SoryntelError::ParseError(
                    "Both reactants and products must be non-empty".into(),
                ));
            }
            Ok(ParsedReaction {
                is_single_substance: false,
                reactants,
                products,
            })
        }
    }
}

/// Split one side of a reaction on '+' outside parentheses.
fn split_side(side: &str) -> Result<Vec<ParsedTerm>, SoryntelError> {
    let mut terms = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes = side.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'+' if depth == 0 => {
                let slice = side[start..i].trim();
                if !slice.is_empty() {
                    terms.push(parse_term(slice)?);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = side[start..].trim();
    if !tail.is_empty() {
        terms.push(parse_term(tail)?);
    }
    Ok(terms)
}

/// Parse a single term with optional leading coefficient (e.g., "4CO(g)", "0.5O2(g)", "2FeS2", "e-").
pub fn parse_term(token: &str) -> Result<ParsedTerm, SoryntelError> {
    let s = token.trim();
    if s.is_empty() {
        return Err(SoryntelError::ParseError("Empty species term".into()));
    }

    // Extract leading numeric coefficient
    let mut num_end = 0;
    let mut seen_dot = false;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() {
            num_end = i + 1;
        } else if c == '.' && !seen_dot && i > 0 {
            seen_dot = true;
            num_end = i + 1;
        } else {
            break;
        }
    }

    let (coeff, formula_raw) = if num_end > 0 {
        let c: f64 = s[..num_end]
            .parse()
            .map_err(|_| SoryntelError::ParseError(format!("Invalid coefficient in '{s}'")))?;
        (c, s[num_end..].trim())
    } else {
        (1.0, s)
    };

    if formula_raw.is_empty() {
        return Err(SoryntelError::ParseError(format!(
            "Missing formula after coefficient in '{s}'"
        )));
    }

    if formula_raw == "e-" || formula_raw == "e(-)" {
        return Ok(ParsedTerm {
            formula: "e-".to_string(),
            coeff,
            elements: BTreeMap::new(),
            charge: -1,
            is_electron: true,
        });
    }

    let (core_formula, charge) = strip_trailing_phase_and_charge(formula_raw)?;
    let elements = parse_formula_composition(core_formula)?;
    if elements.is_empty() {
        return Err(SoryntelError::ParseError(format!(
            "No chemical elements parsed in '{formula_raw}'"
        )));
    }

    Ok(ParsedTerm {
        formula: formula_raw.to_string(),
        coeff,
        elements,
        charge,
        is_electron: false,
    })
}

/// Strip trailing HSC phase/charge suffix such as `(g)`, `(l)`, `(s)`, `(a)`, `(ia)`, `(+2a)`, `(-2a)`, `(+a)`, `(-a)`.
fn strip_trailing_phase_and_charge(formula: &str) -> Result<(&str, i32), SoryntelError> {
    if !formula.ends_with(')') {
        return Ok((formula, 0));
    }
    let open_idx = formula.rfind('(').ok_or_else(|| {
        SoryntelError::ParseError(format!("Unmatched ')' in '{formula}'"))
    })?;
    let suffix = &formula[open_idx + 1..formula.len() - 1];

    // Check if this parenthetical is a phase/charge marker rather than a chemical group like Ca(OH)2
    if let Some(charge) = parse_charge_phase_suffix(suffix) {
        return Ok((&formula[..open_idx], charge));
    }
    Ok((formula, 0))
}

fn parse_charge_phase_suffix(suffix: &str) -> Option<i32> {
    match suffix {
        "s" | "l" | "L" | "g" | "G" | "a" | "ia" | "A" | "B" | "D" | "ao" | "ai" => Some(0),
        "+" | "+a" | "+g" => Some(1),
        "-" | "-a" | "-g" => Some(-1),
        _ => {
            // Patterns like "+2a", "-2a", "+3a", "+2", "-2"
            let trimmed = suffix.trim_end_matches(|c| c == 'a' || c == 'g');
            if trimmed.starts_with('+') || trimmed.starts_with('-') {
                let sign = if trimmed.starts_with('+') { 1 } else { -1 };
                let mag_str = &trimmed[1..];
                if mag_str.is_empty() {
                    return Some(sign);
                }
                if let Ok(mag) = mag_str.parse::<i32>() {
                    return Some(sign * mag);
                }
            }
            None
        }
    }
}

/// Parse element composition of a neutral chemical formula with nested parentheses and optional `*` hydrates.
pub fn parse_formula_composition(formula: &str) -> Result<BTreeMap<String, f64>, SoryntelError> {
    let mut total = BTreeMap::new();
    // Support hydrate / adduct notation like 2FeO*SiO2 or CuSO4*5H2O
    for part in formula.split('*') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut num_end = 0;
        for (i, c) in part.char_indices() {
            if c.is_ascii_digit() || (c == '.' && i > 0) {
                num_end = i + 1;
            } else {
                break;
            }
        }
        let mult: f64 = if num_end > 0 {
            part[..num_end].parse().unwrap_or(1.0)
        } else {
            1.0
        };
        let sub = parse_group(&part[num_end..])?;
        for (el, count) in sub {
            *total.entry(el).or_insert(0.0) += count * mult;
        }
    }
    Ok(total)
}

fn parse_group(s: &str) -> Result<BTreeMap<String, f64>, SoryntelError> {
    let bytes = s.as_bytes();
    let mut stack: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new()];
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'(' {
            stack.push(BTreeMap::new());
            i += 1;
        } else if b == b')' {
            if stack.len() <= 1 {
                return Err(SoryntelError::ParseError(format!(
                    "Unexpected ')' in '{s}'"
                )));
            }
            let group = stack.pop().unwrap();
            i += 1;
            let (mult, next_i) = read_number(s, i);
            i = next_i;
            let top = stack.last_mut().unwrap();
            for (el, cnt) in group {
                *top.entry(el).or_insert(0.0) += cnt * mult;
            }
        } else if b.is_ascii_uppercase() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            let elem = &s[start..i];
            let (mult, next_i) = read_number(s, i);
            i = next_i;
            let top = stack.last_mut().unwrap();
            *top.entry(elem.to_string()).or_insert(0.0) += mult;
        } else {
            return Err(SoryntelError::ParseError(format!(
                "Unexpected character '{}' in formula '{s}'",
                b as char
            )));
        }
    }

    if stack.len() != 1 {
        return Err(SoryntelError::ParseError(format!("Unclosed '(' in '{s}'")));
    }
    Ok(stack.pop().unwrap())
}

fn read_number(s: &str, start: usize) -> (f64, usize) {
    let bytes = s.as_bytes();
    let mut i = start;
    let mut seen_dot = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            i += 1;
        } else if bytes[i] == b'.' && !seen_dot && i > start {
            seen_dot = true;
            i += 1;
        } else {
            break;
        }
    }
    if i == start {
        (1.0, start)
    } else {
        let val = s[start..i].parse::<f64>().unwrap_or(1.0);
        (val, i)
    }
}

/// Exact rational number for stoichiometric balancing without floating-point drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rat {
    num: i128,
    den: i128,
}

impl Rat {
    fn new(num: i128, den: i128) -> Self {
        assert!(den != 0);
        let (n, d) = if den < 0 { (-num, -den) } else { (num, den) };
        let g = gcd(n.abs(), d);
        Self { num: n / g, den: d / g }
    }

    fn from_f64(v: f64) -> Self {
        let scaled = (v * 1_000_000.0).round() as i128;
        Self::new(scaled, 1_000_000)
    }

    fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    fn is_zero(self) -> bool {
        self.num == 0
    }

    fn sub(self, o: Self) -> Self {
        Self::new(self.num * o.den - o.num * self.den, self.den * o.den)
    }

    fn mul(self, o: Self) -> Self {
        Self::new(self.num * o.num, self.den * o.den)
    }

    fn div(self, o: Self) -> Self {
        Self::new(self.num * o.den, self.den * o.num)
    }

    fn neg(self) -> Self {
        Self { num: -self.num, den: self.den }
    }
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    if a == 0 { 1 } else { a }
}

fn lcm(a: i128, b: i128) -> i128 {
    (a / gcd(a, b)) * b
}

/// Balance a reaction equation using exact rational Gaussian elimination.
/// Returns the formatted balanced reaction string and the updated `ParsedReaction`.
pub fn balance_reaction(
    raw: &str,
    smallest_coeff: f64,
) -> Result<(String, ParsedReaction), SoryntelError> {
    if smallest_coeff <= 0.0 || !smallest_coeff.is_finite() {
        return Err(SoryntelError::BalanceError(
            "Smallest Coefficient must be > 0".into(),
        ));
    }

    let mut parsed = parse_input(raw)?;
    if parsed.is_single_substance {
        return Err(SoryntelError::BalanceError(
            "Cannot balance a single chemical substance".into(),
        ));
    }

    let n_react = parsed.reactants.len();
    let n_prod = parsed.products.len();
    let n_cols = n_react + n_prod;

    let mut elem_set = BTreeMap::<String, usize>::new();
    let mut has_charge = false;
    for t in parsed.reactants.iter().chain(parsed.products.iter()) {
        if t.charge != 0 {
            has_charge = true;
        }
        for e in t.elements.keys() {
            let next_idx = elem_set.len();
            elem_set.entry(e.clone()).or_insert(next_idx);
        }
    }

    let n_rows = elem_set.len() + usize::from(has_charge);
    let mut mat = vec![vec![Rat::zero(); n_cols]; n_rows];

    for (j, t) in parsed.reactants.iter().enumerate() {
        for (e, &cnt) in &t.elements {
            let r = elem_set[e];
            mat[r][j] = Rat::from_f64(cnt);
        }
        if has_charge {
            mat[n_rows - 1][j] = Rat::new(t.charge as i128, 1);
        }
    }
    for (k, t) in parsed.products.iter().enumerate() {
        let j = n_react + k;
        for (e, &cnt) in &t.elements {
            let r = elem_set[e];
            mat[r][j] = Rat::from_f64(cnt).neg();
        }
        if has_charge {
            mat[n_rows - 1][j] = Rat::new(-(t.charge as i128), 1);
        }
    }

    // RREF
    let mut pivot_cols = Vec::new();
    let mut pivot_row = 0;
    for col in 0..n_cols {
        if pivot_row >= n_rows {
            break;
        }
        if let Some(r_swap) = (pivot_row..n_rows).find(|&r| !mat[r][col].is_zero()) {
            mat.swap(pivot_row, r_swap);
            let pv = mat[pivot_row][col];
            for c in col..n_cols {
                mat[pivot_row][c] = mat[pivot_row][c].div(pv);
            }
            for r in 0..n_rows {
                if r != pivot_row && !mat[r][col].is_zero() {
                    let factor = mat[r][col];
                    for c in col..n_cols {
                        let sub = factor.mul(mat[pivot_row][c]);
                        mat[r][c] = mat[r][c].sub(sub);
                    }
                }
            }
            pivot_cols.push(col);
            pivot_row += 1;
        }
    }

    let nullity = n_cols - pivot_cols.len();
    if nullity == 0 {
        return Err(SoryntelError::BalanceError(
            "No non-trivial balance exists (elements or charges do not match)".into(),
        ));
    }
    if nullity > 1 {
        return Err(SoryntelError::BalanceError(format!(
            "Underdetermined system (nullity = {nullity}): multiple independent reactions exist"
        )));
    }

    let free_col = (0..n_cols)
        .find(|c| !pivot_cols.contains(c))
        .expect("nullity == 1 guarantees one free column");

    let mut sol = vec![Rat::zero(); n_cols];
    sol[free_col] = Rat::one();
    for (r, &pcol) in pivot_cols.iter().enumerate() {
        sol[pcol] = mat[r][free_col].neg();
    }

    // Clear denominators to smallest integers
    let common_den = sol.iter().fold(1i128, |acc, r| lcm(acc, r.den));
    let mut ints: Vec<i128> = sol.iter().map(|r| r.num * (common_den / r.den)).collect();
    if ints[0] < 0 {
        for v in &mut ints {
            *v = -*v;
        }
    }
    if ints.iter().any(|&v| v <= 0) {
        return Err(SoryntelError::BalanceError(
            "Reaction requires moving a species to the opposite side to balance".into(),
        ));
    }

    let min_int = *ints.iter().min().unwrap();
    let scale = smallest_coeff / (min_int as f64);

    for (j, t) in parsed.reactants.iter_mut().enumerate() {
        t.coeff = (ints[j] as f64) * scale;
    }
    for (k, t) in parsed.products.iter_mut().enumerate() {
        t.coeff = (ints[n_react + k] as f64) * scale;
    }

    let formatted = format_reaction(&parsed);
    Ok((formatted, parsed))
}

/// Verify that a parsed reaction obeys elemental and charge conservation.
pub fn verify_balance(parsed: &ParsedReaction) -> Result<(), SoryntelError> {
    if parsed.is_single_substance {
        return Ok(());
    }
    let mut balance = BTreeMap::<String, f64>::new();
    let mut charge_bal = 0.0f64;

    for t in &parsed.reactants {
        for (el, &cnt) in &t.elements {
            *balance.entry(el.clone()).or_insert(0.0) -= t.coeff * cnt;
        }
        charge_bal -= t.coeff * (t.charge as f64);
    }
    for t in &parsed.products {
        for (el, &cnt) in &t.elements {
            *balance.entry(el.clone()).or_insert(0.0) += t.coeff * cnt;
        }
        charge_bal += t.coeff * (t.charge as f64);
    }

    for (el, diff) in balance {
        if diff.abs() > 1e-4 {
            return Err(SoryntelError::BalanceError(format!(
                "Element '{el}' is not balanced (net difference = {diff:.4})"
            )));
        }
    }
    if charge_bal.abs() > 1e-4 {
        return Err(SoryntelError::BalanceError(format!(
            "Electric charge is not balanced (net difference = {charge_bal:.4})"
        )));
    }
    Ok(())
}

/// Format a balanced reaction back to canonical HSC string syntax.
pub fn format_reaction(parsed: &ParsedReaction) -> String {
    let fmt_term = |t: &ParsedTerm| {
        if (t.coeff - 1.0).abs() < 1e-9 {
            t.formula.clone()
        } else if (t.coeff - t.coeff.round()).abs() < 1e-9 {
            format!("{}{}", t.coeff.round() as i64, t.formula)
        } else {
            format!("{:.4}{}", t.coeff, t.formula)
        }
    };
    let lhs: Vec<String> = parsed.reactants.iter().map(fmt_term).collect();
    if parsed.is_single_substance {
        return lhs.join(" + ");
    }
    let rhs: Vec<String> = parsed.products.iter().map(fmt_term).collect();
    format!("{} = {}", lhs.join(" + "), rhs.join(" + "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_fes2_redox() {
        let (eq, parsed) =
            balance_reaction("FeS2 + H2O = Fe2O3 + SO4(-2a) + H(+a) + e-", 1.0).unwrap();
        assert_eq!(eq, "2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-");
        verify_balance(&parsed).unwrap();
    }

    #[test]
    fn test_balance_mond_process() {
        let (eq, parsed) = balance_reaction("Ni + CO(g) = Ni(CO)4(g)", 1.0).unwrap();
        assert_eq!(eq, "Ni + 4CO(g) = Ni(CO)4(g)");
        verify_balance(&parsed).unwrap();
    }
}
