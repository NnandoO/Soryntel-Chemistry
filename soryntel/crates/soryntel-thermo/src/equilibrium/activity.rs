//! Activity coefficient models for Module 4:
//! - Ideal Mixture (f = 1)
//! - Redlich-Kister binary excess Gibbs (Appendices A, B: Ga-As, C: Al-Zn)
//! - Aqueous Debye-Huckel / ionic-strength model + 55.508435 mixing entropy conversion
//! - Safe Column-C formula evaluator (T, X_row, EXP, LN, arithmetic)

use crate::equilibrium::model::SolutionModel;
use std::collections::BTreeMap;

/// HSC Appendix A gas constant in J/(mol·K).
pub const R_HSC_APP_A: f64 = 8.31431;
/// Moles of H2O per kg of water (1000 / 18.01528).
pub const MOL_H2O_PER_KG: f64 = 55.508435;

/// Computes `ln(gamma_i)` for all species in a mixture phase given current mole fractions `x`.
pub fn compute_phase_ln_gamma(
    model: SolutionModel,
    formulas: &[String],
    charges: &[i32],
    exprs: &[String],
    x: &[f64],
    t_k: f64,
    row_to_mole_frac: &BTreeMap<usize, f64>,
) -> Vec<f64> {
    compute_phase_ln_gamma_with_mode(
        model,
        formulas,
        charges,
        exprs,
        x,
        t_k,
        row_to_mole_frac,
        false,
    )
}

/// Computes `ln(gamma_i)` with optional `column_c_lnf_mode` (`true` = Column C is directly `ln(f)`).
#[allow(clippy::too_many_arguments)]
pub fn compute_phase_ln_gamma_with_mode(
    model: SolutionModel,
    formulas: &[String],
    charges: &[i32],
    exprs: &[String],
    x: &[f64],
    t_k: f64,
    row_to_mole_frac: &BTreeMap<usize, f64>,
    column_c_lnf_mode: bool,
) -> Vec<f64> {
    let n = formulas.len();
    let mut ln_gamma = vec![0.0; n];

    match model {
        SolutionModel::PureSubstance => return ln_gamma,
        SolutionModel::GaAsMixture => {
            apply_ga_as_redlich_kister(formulas, x, t_k, &mut ln_gamma);
            return ln_gamma;
        }
        SolutionModel::AlZnMixture => {
            apply_al_zn_redlich_kister(formulas, x, t_k, &mut ln_gamma);
            return ln_gamma;
        }
        SolutionModel::Aqua => {
            apply_aqua_debye_huckel(formulas, charges, x, t_k, &mut ln_gamma);
        }
        SolutionModel::IdealMixture => {}
    }

    // Apply any explicit Column-C formula or constant numeric activity coefficient.
    for (i, expr) in exprs.iter().enumerate() {
        let trimmed = expr.trim();
        if trimmed.is_empty() || (!column_c_lnf_mode && (trimmed == "1" || trimmed == "1.0" || trimmed == "1.000")) {
            continue;
        }
        if let Ok(val) = trimmed.parse::<f64>() {
            if column_c_lnf_mode {
                ln_gamma[i] = val.clamp(-50.0, 50.0);
            } else if val > 0.0 {
                ln_gamma[i] = val.ln();
            }
            continue;
        }
        let upper = trimmed.to_uppercase().replace(' ', "");
        let has_explicit_prefix = upper.starts_with("LN(F)=") || upper.starts_with("=EXP(") || upper.starts_with("EXP(");
        if let Some(val) = eval_column_c_formula(trimmed, t_k, row_to_mole_frac) {
            if column_c_lnf_mode && !has_explicit_prefix {
                if val.is_finite() {
                    ln_gamma[i] = val.clamp(-50.0, 50.0);
                }
            } else if val > 0.0 && val.is_finite() {
                ln_gamma[i] = val.ln();
            }
        }
    }

    ln_gamma
}

/// Appendix A & B: Redlich-Kister binary excess Gibbs for liquid Ga-As:
/// k0 = -25503.6 - 4.3109 * T, k1 = 5174.7 (J/mol).
fn apply_ga_as_redlich_kister(formulas: &[String], x: &[f64], t_k: f64, ln_gamma: &mut [f64]) {
    let idx_ga = formulas
        .iter()
        .position(|f| f.trim().eq_ignore_ascii_case("Ga(l)") || f.trim().eq_ignore_ascii_case("Ga"));
    let idx_as = formulas
        .iter()
        .position(|f| f.trim().eq_ignore_ascii_case("As(l)") || f.trim().eq_ignore_ascii_case("As"));
    if let (Some(i), Some(j)) = (idx_ga, idx_as) {
        let xi = x[i].clamp(0.0, 1.0);
        let xj = x[j].clamp(0.0, 1.0);
        let k0 = -25503.6 - 4.3109 * t_k;
        let k1 = 5174.7;
        let rt = R_HSC_APP_A * t_k.max(1.0);
        let (gi, gj) = redlich_kister_binary(xi, xj, &[k0, k1], rt);
        ln_gamma[i] = gi;
        ln_gamma[j] = gj;
    }
}

/// Appendix C (`13-equilibrium-module.md:1308-1499`): Redlich-Kister binary excess Gibbs for Al-Zn:
/// - Mode 1 (Liquid `Al(l)`, `Zn(l)`): k0 = 10483.5 - 4.74442*T, k1 = -210.4 + 0.85139*T
/// - Mode 2 (Solid FCC `Al`/`Al(FCC)`/`Al(s)`, `Zn`/`Zn(FCC)`/`Zn(s)`):
///     k0 = 11014.8 - 3.71812*T, k1 = 1546.9 + 1.60179*T, k2 = -308.7
/// - Mode 3 (Solid HCP `Al(HCP)`, `Zn(HCP)`): k0 = 17047.8 - 10.37525*T
pub fn apply_al_zn_redlich_kister(formulas: &[String], x: &[f64], t_k: f64, ln_gamma: &mut [f64]) {
    let rt = R_HSC_APP_A * t_k.max(1.0);

    // 1. Liquid phase: Al(l) + Zn(l)
    let idx_al_l = formulas.iter().position(|f| f.trim().eq_ignore_ascii_case("Al(l)"));
    let idx_zn_l = formulas.iter().position(|f| f.trim().eq_ignore_ascii_case("Zn(l)"));
    if let (Some(i), Some(j)) = (idx_al_l, idx_zn_l) {
        let xi = x[i].clamp(0.0, 1.0);
        let xj = x[j].clamp(0.0, 1.0);
        let k0 = 10483.5 - 4.74442 * t_k;
        let k1 = -210.4 + 0.85139 * t_k;
        let (gi, gj) = redlich_kister_binary(xi, xj, &[k0, k1], rt);
        ln_gamma[i] = gi;
        ln_gamma[j] = gj;
        return;
    }

    // 2. Solid HCP phase: Al(HCP) + Zn(HCP)
    let idx_al_hcp = formulas
        .iter()
        .position(|f| f.trim().eq_ignore_ascii_case("Al(HCP)"));
    let idx_zn_hcp = formulas
        .iter()
        .position(|f| f.trim().eq_ignore_ascii_case("Zn(HCP)"));
    if let (Some(i), Some(j)) = (idx_al_hcp, idx_zn_hcp) {
        let xi = x[i].clamp(0.0, 1.0);
        let xj = x[j].clamp(0.0, 1.0);
        let k0 = 17047.8 - 10.37525 * t_k;
        let (gi, gj) = redlich_kister_binary(xi, xj, &[k0], rt);
        ln_gamma[i] = gi;
        ln_gamma[j] = gj;
        return;
    }

    // 3. Solid FCC phase: Al / Al(FCC) / Al(s) + Zn / Zn(FCC) / Zn(s)
    let idx_al_fcc = formulas.iter().position(|f| {
        let t = f.trim();
        t.eq_ignore_ascii_case("Al")
            || t.eq_ignore_ascii_case("Al(FCC)")
            || t.eq_ignore_ascii_case("Al(s)")
    });
    let idx_zn_fcc = formulas.iter().position(|f| {
        let t = f.trim();
        t.eq_ignore_ascii_case("Zn")
            || t.eq_ignore_ascii_case("Zn(FCC)")
            || t.eq_ignore_ascii_case("Zn(s)")
    });
    if let (Some(i), Some(j)) = (idx_al_fcc, idx_zn_fcc) {
        let xi = x[i].clamp(0.0, 1.0);
        let xj = x[j].clamp(0.0, 1.0);
        let k0 = 11014.8 - 3.71812 * t_k;
        let k1 = 1546.9 + 1.60179 * t_k;
        let k2 = -308.7;
        let (gi, gj) = redlich_kister_binary(xi, xj, &[k0, k1, k2], rt);
        ln_gamma[i] = gi;
        ln_gamma[j] = gj;
    }
}

/// General Redlich-Kister binary derivative formulas (Appendix A):
/// RT ln f_i = x_j^2 * sum_n k_n [ (x_i - x_j)^n + 2 n x_i (x_i - x_j)^(n-1) ]
/// RT ln f_j = x_i^2 * sum_n k_n [ (x_i - x_j)^n - 2 n x_j (x_i - x_j)^(n-1) ]
pub fn redlich_kister_binary(xi: f64, xj: f64, k: &[f64], rt: f64) -> (f64, f64) {
    let diff = xi - xj;
    let mut sum_i = 0.0;
    let mut sum_j = 0.0;
    for (n, &kn) in k.iter().enumerate() {
        let nf = n as f64;
        let p_n = diff.powi(n as i32);
        let p_nm1 = if n == 0 { 0.0 } else { diff.powi((n - 1) as i32) };
        sum_i += kn * (p_n + 2.0 * nf * xi * p_nm1);
        sum_j += kn * (p_n - 2.0 * nf * xj * p_nm1);
    }
    ((xj * xj * sum_i) / rt, (xi * xi * sum_j) / rt)
}

/// Extended Debye-Huckel (Davies/B-dot style) for aqueous ions when `Aqua` model is selected.
fn apply_aqua_debye_huckel(
    formulas: &[String],
    charges: &[i32],
    x: &[f64],
    t_k: f64,
    ln_gamma: &mut [f64],
) {
    let h2o_idx = formulas.iter().position(|f| f.trim() == "H2O");
    let x_w = h2o_idx.map(|idx| x[idx]).unwrap_or(1.0).max(1e-12);
    // Ionic strength I = 0.5 * sum m_i z_i^2
    let mut ionic_strength = 0.0;
    for (i, &z) in charges.iter().enumerate() {
        if z != 0 {
            let mi = MOL_H2O_PER_KG * (x[i] / x_w);
            ionic_strength += 0.5 * mi * (z as f64) * (z as f64);
        }
    }
    let sqrt_i = ionic_strength.clamp(0.0, 20.0).sqrt();
    // Temperature-adjusted Debye-Huckel A_gamma (~0.509 at 298.15 K)
    let a_dh = 0.509 * (298.15 / t_k.max(250.0)).powf(1.5) * 2.302585;
    for (i, &z) in charges.iter().enumerate() {
        if z != 0 {
            let z2 = (z as f64) * (z as f64);
            // Davies-like bounded equation: ln(gamma) = -A * z^2 * (sqrt(I)/(1+sqrt(I)) - 0.2*I)
            ln_gamma[i] = (-a_dh * z2 * (sqrt_i / (1.0 + sqrt_i) - 0.2 * ionic_strength))
                .clamp(-10.0, 10.0);
        }
    }
}

/// Evaluates simple Column-C formulas like `=EXP(8495/T - 2.653)` or `8495/T - 2.653` (if prefixed with `LN(F)=`).
pub fn eval_column_c_formula(
    raw: &str,
    t_k: f64,
    row_to_mole_frac: &BTreeMap<usize, f64>,
) -> Option<f64> {
    let mut s: String = raw.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase();
    if let Some(rest) = s.strip_prefix('=') {
        s = rest.to_string();
    }
    let mut is_ln_f = false;
    if let Some(rest) = s.strip_prefix("LN(F)=") {
        s = rest.to_string();
        is_ln_f = true;
    }
    let mut parser = ExprParser::new(&s, t_k, row_to_mole_frac);
    let val = parser.parse_expr()?;
    if is_ln_f {
        Some(val.exp())
    } else {
        Some(val)
    }
}

struct ExprParser<'a> {
    chars: Vec<char>,
    pos: usize,
    t_k: f64,
    x_map: &'a BTreeMap<usize, f64>,
}

impl<'a> ExprParser<'a> {
    fn new(s: &str, t_k: f64, x_map: &'a BTreeMap<usize, f64>) -> Self {
        Self {
            chars: s.chars().filter(|c| !c.is_whitespace()).collect(),
            pos: 0,
            t_k,
            x_map,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn parse_expr(&mut self) -> Option<f64> {
        let mut val = self.parse_term()?;
        while let Some(c) = self.peek() {
            match c {
                '+' => {
                    self.next();
                    val += self.parse_term()?;
                }
                '-' => {
                    self.next();
                    val -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Some(val)
    }

    fn parse_term(&mut self) -> Option<f64> {
        let mut val = self.parse_power()?;
        while let Some(c) = self.peek() {
            match c {
                '*' => {
                    self.next();
                    val *= self.parse_power()?;
                }
                '/' => {
                    self.next();
                    let d = self.parse_power()?;
                    if d.abs() < 1e-30 {
                        return None;
                    }
                    val /= d;
                }
                _ => break,
            }
        }
        Some(val)
    }

    fn parse_power(&mut self) -> Option<f64> {
        let base = self.parse_unary()?;
        if self.peek() == Some('^') {
            self.next();
            let exp = self.parse_power()?;
            Some(base.powf(exp))
        } else {
            Some(base)
        }
    }

    fn parse_unary(&mut self) -> Option<f64> {
        if self.peek() == Some('-') {
            self.next();
            return Some(-self.parse_unary()?);
        }
        if self.peek() == Some('+') {
            self.next();
            return self.parse_unary();
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<f64> {
        match self.peek()? {
            '(' => {
                self.next();
                let val = self.parse_expr()?;
                if self.peek() == Some(')') {
                    self.next();
                }
                Some(val)
            }
            '0'..='9' | '.' => self.parse_number(),
            'A'..='Z' | '_' => self.parse_ident(),
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.next();
            } else if (c == 'E' || c == 'e')
                && self
                    .chars
                    .get(self.pos + 1)
                    .map(|n| n.is_ascii_digit() || *n == '+' || *n == '-')
                    .unwrap_or(false)
            {
                self.next();
                if matches!(self.peek(), Some('+') | Some('-')) {
                    self.next();
                }
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        s.parse::<f64>().ok()
    }

    fn parse_ident(&mut self) -> Option<f64> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.next();
            } else {
                break;
            }
        }
        let ident: String = self.chars[start..self.pos].iter().collect();
        if self.peek() == Some('(') {
            self.next();
            let arg = self.parse_expr()?;
            if self.peek() == Some(')') {
                self.next();
            }
            return match ident.as_str() {
                "EXP" => Some(arg.exp()),
                "LN" | "LOG" => Some(arg.max(1e-300).ln()),
                "LOG10" => Some(arg.max(1e-300).log10()),
                _ => None,
            };
        }
        if ident == "T" {
            return Some(self.t_k);
        }
        if let Some(row_str) = ident.strip_prefix("X_") {
            if let Ok(row) = row_str.parse::<usize>() {
                return Some(*self.x_map.get(&row).unwrap_or(&0.0));
            }
        } else if let Some(row_str) = ident.strip_prefix('X') {
            if let Ok(row) = row_str.parse::<usize>() {
                return Some(*self.x_map.get(&row).unwrap_or(&0.0));
            }
        }
        None
    }
}
