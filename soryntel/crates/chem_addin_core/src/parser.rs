use crate::elements::atomic_weight;
use crate::error::{ChemAddinError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

static PHASE_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\((aq|ia|a|g|l|s)\)$").expect("valid phase regex"));
static LEADING_COEFF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+(?:\.\d+)?(?:/\d+(?:\.\d+)?)?)(?:\s*\*?\s*|\s+)([A-Za-z(\[].*)$")
        .expect("valid coeff regex")
});
static UNSIGNED_NUM_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+(?:\.\d+)?)(.*)$").expect("valid number prefix regex")
});
static BRACKET_CHARGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.+?)\(([+-]\d+|\d*[+-])\)$").expect("valid bracket charge regex")
});
static CARET_CHARGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.+?)\^(\d*[+-]|[+-]\d+)$").expect("valid caret charge regex")
});

/// Thermodynamic aggregation phase suffix parsed from species symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhaseTag {
    Gas,
    Liquid,
    Solid,
    Aqueous,
    Unspecified,
}

impl PhaseTag {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Gas => "g",
            Self::Liquid => "l",
            Self::Solid => "s",
            Self::Aqueous => "aq",
            Self::Unspecified => "s",
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Gas => "(g)",
            Self::Liquid => "(l)",
            Self::Solid => "(s)",
            Self::Aqueous => "(aq)",
            Self::Unspecified => "",
        }
    }

    fn from_tag(tag: &str) -> Self {
        match tag.to_ascii_lowercase().as_str() {
            "g" => Self::Gas,
            "l" => Self::Liquid,
            "s" => Self::Solid,
            "aq" | "ia" | "a" => Self::Aqueous,
            _ => Self::Unspecified,
        }
    }
}

/// Parsed single chemical species with elemental stoichiometry, phase, and net charge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSpecies {
    pub raw: String,
    /// Formula excluding phase tag (retains hydrates / charge, normalized whitespace).
    pub formula_core: String,
    /// Standardized symbol (`core + optional phase suffix`).
    pub canonical_symbol: String,
    pub phase: PhaseTag,
    pub composition: BTreeMap<String, f64>,
    pub charge: i32,
}

impl ParsedSpecies {
    /// Compute exact molar mass (g/mol) from elemental composition and ionic charge.
    pub fn molar_mass(&self) -> Result<f64> {
        let mut mw = 0.0;
        for (elem, &count) in &self.composition {
            let aw = atomic_weight(elem)
                .ok_or_else(|| ChemAddinError::UnknownElement(elem.clone()))?;
            mw += aw * count;
        }
        // Adjust for lost/gained electrons cleanly when ionic.
        if self.charge != 0 && !(self.composition.is_empty() && self.charge == -1) {
            let me = atomic_weight("e-").unwrap_or(0.00054858);
            mw -= (self.charge as f64) * me;
        }
        Ok(mw)
    }
}

/// One species term on a reaction side (`coeff * species`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionTerm {
    pub coeff: f64,
    pub explicit_coeff: bool,
    pub species: ParsedSpecies,
}

/// Parsed chemical reaction equation (`reactants = products`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedReaction {
    pub reactants: Vec<ReactionTerm>,
    pub products: Vec<ReactionTerm>,
}

/// Return true if `expr` represents a chemical reaction rather than a single species.
pub fn is_reaction(expr: &str) -> bool {
    expr.contains('=') || expr.contains("->") || expr.contains("=>")
}

/// Parse a chemical species string supporting phase tags (`(g)`), nested parentheses/brackets,
/// hydrate/adduct separators (`*`, `·`), fractional coefficients, and ionic charges.
pub fn parse_species(input: &str) -> Result<ParsedSpecies> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ChemAddinError::InvalidFormula("<empty>".to_string()));
    }

    // 1. Peel optional trailing phase suffix such as (g), (l), (s), (aq).
    let (without_phase, phase) = if let Some(caps) = PHASE_SUFFIX_RE.captures(trimmed) {
        let m = caps.get(0).expect("full match");
        let core = trimmed[..m.start()].trim();
        let tag = PhaseTag::from_tag(caps.get(1).map(|c| c.as_str()).unwrap_or(""));
        (core, tag)
    } else {
        (trimmed, PhaseTag::Unspecified)
    };

    if without_phase.is_empty() {
        return Err(ChemAddinError::InvalidFormula(trimmed.to_string()));
    }

    // Handle isolated electron species (`e-`, `e(-1)`, etc.)
    if matches!(without_phase, "e-" | "e(-)" | "e(-1)" | "e^-") {
        return Ok(ParsedSpecies {
            raw: trimmed.to_string(),
            formula_core: "e-".to_string(),
            canonical_symbol: "e-".to_string(),
            phase,
            composition: BTreeMap::new(),
            charge: -1,
        });
    }

    // 2. Peel optional trailing ionic charge (`(+2)`, `^-2`, `+2`, `-`, etc.)
    let (formula_body, charge) = extract_charge(without_phase)?;

    // 3. Split hydrate / adduct terms by top-level `*` or `·`.
    let adducts = split_top_level_adducts(formula_body, trimmed)?;
    let mut total_comp = BTreeMap::<String, f64>::new();

    for adduct_raw in adducts {
        let adduct = adduct_raw.trim();
        if adduct.is_empty() {
            return Err(ChemAddinError::InvalidFormula(trimmed.to_string()));
        }
        // Check for optional leading adduct stoichiometry (e.g. `5H2O`, `0.5H2O`, `4(H2O)`).
        let (mult, formula_part) = split_leading_unsigned_multiplier(adduct, trimmed)?;
        let part_comp = parse_formula_group(formula_part, trimmed)?;
        if part_comp.is_empty() {
            return Err(ChemAddinError::InvalidFormula(trimmed.to_string()));
        }
        for (el, cnt) in part_comp {
            *total_comp.entry(el).or_insert(0.0) += mult * cnt;
        }
    }

    if total_comp.is_empty() {
        return Err(ChemAddinError::InvalidFormula(trimmed.to_string()));
    }

    let core = without_phase.to_string();
    let canonical = if phase == PhaseTag::Unspecified {
        core.clone()
    } else {
        format!("{core}{}", phase.suffix())
    };

    Ok(ParsedSpecies {
        raw: trimmed.to_string(),
        formula_core: core,
        canonical_symbol: canonical,
        phase,
        composition: total_comp,
        charge,
    })
}

/// Parse a reaction string containing `=`, `<=>`, `=>`, or `->`.
pub fn parse_reaction(input: &str) -> Result<ParsedReaction> {
    let normalized = input
        .trim()
        .replace("<=>", "=")
        .replace("=>", "=")
        .replace("->", "=");

    let sides: Vec<&str> = normalized.split('=').collect();
    if sides.len() != 2 {
        return Err(ChemAddinError::InvalidReaction(format!(
            "Expected exactly one '=' separator in '{input}'"
        )));
    }

    let lhs_raw = sides[0].trim();
    let rhs_raw = sides[1].trim();
    if lhs_raw.is_empty() || rhs_raw.is_empty() {
        return Err(ChemAddinError::InvalidReaction(format!(
            "Both reactant and product sides must be non-empty in '{input}'"
        )));
    }

    let reactants = parse_reaction_side(lhs_raw)?;
    let products = parse_reaction_side(rhs_raw)?;

    Ok(ParsedReaction {
        reactants,
        products,
    })
}

fn parse_reaction_side(side: &str) -> Result<Vec<ReactionTerm>> {
    let raw_terms = split_reaction_terms(side)?;
    let mut terms = Vec::with_capacity(raw_terms.len());
    for raw in raw_terms {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(ChemAddinError::InvalidReaction(format!(
                "Empty term in reaction side '{side}'"
            )));
        }
        let (coeff, explicit_coeff, species_str) = split_reaction_term_coeff(trimmed)?;
        let species = parse_species(species_str)?;
        terms.push(ReactionTerm {
            coeff,
            explicit_coeff,
            species,
        });
    }
    Ok(terms)
}

fn split_reaction_term_coeff(term: &str) -> Result<(f64, bool, &str)> {
    if let Some(caps) = LEADING_COEFF_RE.captures(term) {
        let coeff_str = caps.get(1).expect("coeff group").as_str();
        let rest = caps.get(2).expect("formula group").as_str().trim();
        let coeff = parse_coeff_number(coeff_str, term)?;
        if coeff <= 0.0 || !coeff.is_finite() {
            return Err(ChemAddinError::InvalidReaction(format!(
                "Stoichiometric coefficient must be > 0 in '{term}'"
            )));
        }
        return Ok((coeff, true, rest));
    }
    Ok((1.0, false, term))
}

fn parse_coeff_number(raw: &str, ctx: &str) -> Result<f64> {
    if let Some((num_s, den_s)) = raw.split_once('/') {
        let num: f64 = num_s.parse().map_err(|_| {
            ChemAddinError::InvalidReaction(format!("Invalid coefficient '{raw}' in '{ctx}'"))
        })?;
        let den: f64 = den_s.parse().map_err(|_| {
            ChemAddinError::InvalidReaction(format!("Invalid coefficient '{raw}' in '{ctx}'"))
        })?;
        if den == 0.0 {
            return Err(ChemAddinError::InvalidReaction(format!(
                "Zero denominator in '{ctx}'"
            )));
        }
        return Ok(num / den);
    }
    raw.parse::<f64>().map_err(|_| {
        ChemAddinError::InvalidReaction(format!("Invalid coefficient '{raw}' in '{ctx}'"))
    })
}

fn split_reaction_terms(side: &str) -> Result<Vec<String>> {
    let bytes = side.as_bytes();
    let mut terms = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return Err(ChemAddinError::InvalidReaction(format!(
                        "Unbalanced brackets in '{side}'"
                    )));
                }
            }
            '+' if depth == 0 => {
                // Distinguish ionic charge `+` / `+2` / `+3` directly attached to a species
                // from an addition operator between two terms.
                let is_ionic_charge = if i > 0 && !bytes[i - 1].is_ascii_whitespace() {
                    let rest = &side[i + 1..];
                    let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
                    let next_after_digits = after_digits.chars().next();
                    // e.g. `H+ + OH-`, `Fe+2+O2`, `Fe+2(aq) + ...`, terminal `Fe+2`
                    let followed_by_boundary = matches!(
                        next_after_digits,
                        None | Some('+') | Some('(')
                    ) || (rest.starts_with(char::is_whitespace) && rest.trim_start().starts_with('+'));
                    let had_digits = after_digits.len() < rest.len();
                    let pure_charge_at_end_or_plus =
                        rest.trim_start().is_empty() || rest.trim_start().starts_with('+');
                    if rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
                        false
                    } else if had_digits {
                        matches!(next_after_digits, None | Some('+') | Some('('))
                            || after_digits.starts_with(char::is_whitespace)
                    } else {
                        followed_by_boundary || pure_charge_at_end_or_plus
                    }
                } else {
                    false
                };

                if !is_ionic_charge {
                    let piece = side[start..i].trim();
                    if piece.is_empty() {
                        return Err(ChemAddinError::InvalidReaction(format!(
                            "Unexpected '+' in '{side}'"
                        )));
                    }
                    terms.push(piece.to_string());
                    start = i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    if depth != 0 {
        return Err(ChemAddinError::InvalidReaction(format!(
            "Unclosed brackets in '{side}'"
        )));
    }
    let tail = side[start..].trim();
    if tail.is_empty() {
        return Err(ChemAddinError::InvalidReaction(format!(
            "Trailing '+' in '{side}'"
        )));
    }
    terms.push(tail.to_string());
    Ok(terms)
}

fn extract_charge(s: &str) -> Result<(&str, i32)> {
    if let Some(caps) = BRACKET_CHARGE_RE.captures(s) {
        let base = caps.get(1).expect("base").as_str().trim();
        let charge_raw = caps.get(2).expect("charge").as_str();
        let charge = parse_charge_token(charge_raw, s)?;
        return Ok((base, charge));
    }
    if let Some(caps) = CARET_CHARGE_RE.captures(s) {
        let base = caps.get(1).expect("base").as_str().trim();
        let charge_raw = caps.get(2).expect("charge").as_str();
        let charge = parse_charge_token(charge_raw, s)?;
        return Ok((base, charge));
    }
    // Check suffix `+`, `-`, `+2`, `-2`, `+3` attached directly to letter or closing paren/bracket
    if let Some(idx) = s.rfind(['+', '-']) {
        // Guard against misinterpreting internal separators; charge suffix only contains sign + digits.
        let suffix = &s[idx..];
        let is_valid_suffix =
            suffix.len() >= 1 && suffix[1..].chars().all(|c| c.is_ascii_digit());
        if is_valid_suffix && idx > 0 {
            let base = &s[..idx];
            if !base.ends_with('*') && !base.ends_with('(') && !base.ends_with('[') {
                let sign = if suffix.starts_with('-') { -1 } else { 1 };
                let mag: i32 = if suffix.len() == 1 {
                    1
                } else {
                    suffix[1..].parse().map_err(|_| {
                        ChemAddinError::InvalidFormula(s.to_string())
                    })?
                };
                return Ok((base, sign * mag));
            }
        }
    }
    Ok((s, 0))
}

fn parse_charge_token(tok: &str, ctx: &str) -> Result<i32> {
    if tok == "+" {
        return Ok(1);
    }
    if tok == "-" {
        return Ok(-1);
    }
    if let Some(rest) = tok.strip_prefix('+') {
        let n: i32 = rest.parse().map_err(|_| ChemAddinError::InvalidFormula(ctx.to_string()))?;
        return Ok(n);
    }
    if let Some(rest) = tok.strip_prefix('-') {
        let n: i32 = rest.parse().map_err(|_| ChemAddinError::InvalidFormula(ctx.to_string()))?;
        return Ok(-n);
    }
    if let Some(rest) = tok.strip_suffix('+') {
        let n: i32 = if rest.is_empty() {
            1
        } else {
            rest.parse().map_err(|_| ChemAddinError::InvalidFormula(ctx.to_string()))?
        };
        return Ok(n);
    }
    if let Some(rest) = tok.strip_suffix('-') {
        let n: i32 = if rest.is_empty() {
            1
        } else {
            rest.parse().map_err(|_| ChemAddinError::InvalidFormula(ctx.to_string()))?
        };
        return Ok(-n);
    }
    Err(ChemAddinError::InvalidFormula(ctx.to_string()))
}

fn split_top_level_adducts<'a>(s: &'a str, full_ctx: &str) -> Result<Vec<&'a str>> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;

    for (idx, ch) in s.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
                }
            }
            '*' | '·' if depth == 0 => {
                parts.push(&s[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
    }
    parts.push(&s[start..]);
    Ok(parts)
}

fn split_leading_unsigned_multiplier<'a>(s: &'a str, full_ctx: &str) -> Result<(f64, &'a str)> {
    if let Some(caps) = UNSIGNED_NUM_PREFIX_RE.captures(s) {
        let num_str = caps.get(1).expect("num").as_str();
        let rest = caps.get(2).expect("rest").as_str().trim();
        if rest.is_empty() {
            return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
        }
        let mult: f64 = num_str
            .parse()
            .map_err(|_| ChemAddinError::InvalidFormula(full_ctx.to_string()))?;
        if mult <= 0.0 || !mult.is_finite() {
            return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
        }
        return Ok((mult, rest));
    }
    Ok((1.0, s))
}

fn parse_formula_group(s: &str, full_ctx: &str) -> Result<BTreeMap<String, f64>> {
    let bytes = s.as_bytes();
    let (map, pos) = parse_sequence(bytes, 0, None, full_ctx)?;
    if pos != bytes.len() {
        return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
    }
    Ok(map)
}

fn parse_sequence(
    bytes: &[u8],
    mut i: usize,
    closing_delim: Option<u8>,
    full_ctx: &str,
) -> Result<(BTreeMap<String, f64>, usize)> {
    let mut comp = BTreeMap::<String, f64>::new();

    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if let Some(close) = closing_delim {
            if b == close {
                return Ok((comp, i + 1));
            }
        }
        if b == b'(' || b == b'[' {
            let expected_close = if b == b'(' { b')' } else { b']' };
            let (inner, next_i) = parse_sequence(bytes, i + 1, Some(expected_close), full_ctx)?;
            if inner.is_empty() {
                return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
            }
            let (mult, after_num) = parse_optional_number(bytes, next_i, full_ctx)?;
            for (el, cnt) in inner {
                *comp.entry(el).or_insert(0.0) += cnt * mult;
            }
            i = after_num;
        } else if b.is_ascii_uppercase() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            let elem = std::str::from_utf8(&bytes[start..i])
                .map_err(|_| ChemAddinError::InvalidFormula(full_ctx.to_string()))?;
            if atomic_weight(elem).is_none() {
                return Err(ChemAddinError::UnknownElement(elem.to_string()));
            }
            let (cnt, after_num) = parse_optional_number(bytes, i, full_ctx)?;
            *comp.entry(elem.to_string()).or_insert(0.0) += cnt;
            i = after_num;
        } else {
            return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
        }
    }

    if closing_delim.is_some() {
        return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
    }

    Ok((comp, i))
}

fn parse_optional_number(bytes: &[u8], mut i: usize, full_ctx: &str) -> Result<(f64, usize)> {
    let start = i;
    let mut seen_dot = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_digit() {
            i += 1;
        } else if b == b'.' && !seen_dot {
            seen_dot = true;
            i += 1;
        } else {
            break;
        }
    }
    if i == start {
        return Ok((1.0, i));
    }
    let slice = std::str::from_utf8(&bytes[start..i])
        .map_err(|_| ChemAddinError::InvalidFormula(full_ctx.to_string()))?;
    if slice == "." || slice.ends_with('.') {
        return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
    }
    let val: f64 = slice
        .parse()
        .map_err(|_| ChemAddinError::InvalidFormula(full_ctx.to_string()))?;
    if !val.is_finite() || val < 0.0 {
        return Err(ChemAddinError::InvalidFormula(full_ctx.to_string()));
    }
    Ok((val, i))
}
