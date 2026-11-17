use crate::db::{ChemDatabase, SpeciesInterval};
use crate::error::{ChemAddinError, Result};
use crate::parser::{is_reaction, parse_reaction, parse_species};
use crate::stoichiometry;
use crate::units::{R_JOULE, T_REF_K, UnitSystem};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Raw SI thermodynamic evaluation at temperature `T` (`kJ/mol` and `J/(mol·K)`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermoPointSi {
    pub temp_k: f64,
    /// Enthalpy `H(T)` in `kJ/mol`.
    pub h_kj_mol: f64,
    /// Entropy `S(T)` in `J/(mol·K)`.
    pub s_j_mol_k: f64,
    /// Gibbs free energy `G(T) = H(T) - T * S(T) / 1000` in `kJ/mol`.
    pub g_kj_mol: f64,
    /// Heat capacity `Cp(T)` in `J/(mol·K)`.
    pub cp_j_mol_k: f64,
}

/// Stateful HSC-style calculation engine owning SQLite persistence and active [`UnitSystem`].
pub struct ChemEngine {
    pub db: ChemDatabase,
    pub units: UnitSystem,
}

impl ChemEngine {
    /// Initialize an in-memory engine preloaded with SQLite schema and real thermochemical seeds.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            db: ChemDatabase::open_in_memory()?,
            units: UnitSystem::default(),
        })
    }

    /// Initialize an engine backed by a persistent SQLite database file.
    pub fn open_sqlite(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            db: ChemDatabase::open_file(path)?,
            units: UnitSystem::default(),
        })
    }

    // --- Unit session accessors (`EnergyU`, `TempU`, `PresU`) ---

    pub fn energy_u(&self) -> &'static str {
        self.units.energy_u()
    }

    pub fn temp_u(&self) -> &'static str {
        self.units.temp_u()
    }

    pub fn pres_u(&self) -> &'static str {
        self.units.pres_u()
    }

    pub fn set_energy_u(&mut self, unit: &str) -> Result<()> {
        self.units.set_energy(unit)
    }

    pub fn set_temp_u(&mut self, unit: &str) -> Result<()> {
        self.units.set_temp(unit)
    }

    pub fn set_pres_u(&mut self, unit: &str) -> Result<()> {
        self.units.set_pressure(unit)
    }

    // --- Stoichiometry wrappers ---

    /// Molar weight `MW(species)` in `g/mol`, falling back to `species_thermo`/`minerals` names (e.g. `"Pyrite"`).
    pub fn mw(&self, species: &str) -> Result<f64> {
        match stoichiometry::mw(species) {
            Ok(val) => Ok(val),
            Err(err) => {
                if let Some(m) = self.db.get_mineral(species)? {
                    return stoichiometry::mw(&m.formula);
                }
                if let Ok(intervals) = self.db.get_species_intervals(species) {
                    if let Some(first) = intervals.first() {
                        return Ok(first.mw);
                    }
                }
                Err(err)
            }
        }
    }

    /// Weight percentage `WTP(species1, species2)`.
    pub fn wtp(&self, species1: &str, species2: &str) -> Result<f64> {
        let s1 = self.resolve_formula_alias(species1)?;
        let s2 = self.resolve_formula_alias(species2)?;
        stoichiometry::wtp(&s1, &s2)
    }

    /// Balanced chemical equation `BAL(reaction, multiplier)` using `nalgebra` SVD nullspace.
    pub fn bal(&self, reaction: &str, multiplier: f64) -> Result<String> {
        stoichiometry::bal(reaction, multiplier)
    }

    // --- Thermochemistry (`H`, `S`, `G`, `K`) ---

    /// Enthalpy `H(expr, temp)` for a single species or chemical reaction (`=`),
    /// converting input `temp` using `TempU` and returning `kJ/mol` or `kcal/mol` per `EnergyU`.
    pub fn h(&self, expr: &str, temp: f64) -> Result<f64> {
        let pt = self.eval_expr_si(expr, temp)?;
        Ok(self.units.energy_from_kj(pt.h_kj_mol))
    }

    /// Entropy `S(expr, temp)` for a single species or chemical reaction (`=`),
    /// converting input `temp` using `TempU` and returning `J/(mol·K)` or `cal/(mol·K)` per `EnergyU`.
    pub fn s(&self, expr: &str, temp: f64) -> Result<f64> {
        let pt = self.eval_expr_si(expr, temp)?;
        Ok(self.units.entropy_from_j(pt.s_j_mol_k))
    }

    /// Heat capacity `Cp(expr, temp)` in `J/(mol·K)` or `cal/(mol·K)` per `EnergyU`.
    pub fn cp(&self, expr: &str, temp: f64) -> Result<f64> {
        let pt = self.eval_expr_si(expr, temp)?;
        Ok(self.units.entropy_from_j(pt.cp_j_mol_k))
    }

    /// Gibbs free energy `G(expr, temp) = H(T) - T * S(T)` in `kJ/mol` or `kcal/mol` per `EnergyU`.
    pub fn g(&self, expr: &str, temp: f64) -> Result<f64> {
        let pt = self.eval_expr_si(expr, temp)?;
        Ok(self.units.energy_from_kj(pt.g_kj_mol))
    }

    /// Equilibrium constant `K(reaction, temp) = exp(-DeltaG°(T) / (R * T))` (dimensionless).
    /// Returns an error if `reaction` does not contain a reaction separator (`=`, `->`, `=>`).
    pub fn k(&self, reaction: &str, temp: f64) -> Result<f64> {
        if !is_reaction(reaction) {
            return Err(ChemAddinError::InvalidReaction(format!(
                "K(reaction, T) requires a reaction equation containing '=' (got '{reaction}')"
            )));
        }
        let pt = self.eval_reaction_si(reaction, temp)?;
        let dg_j = pt.g_kj_mol * 1000.0;
        let exponent = -dg_j / (R_JOULE * pt.temp_k);
        if exponent > 709.78 {
            Ok(f64::INFINITY)
        } else if exponent < -745.0 {
            Ok(0.0)
        } else {
            Ok(exponent.exp())
        }
    }

    /// Evaluate all SI thermodynamic properties `(H, S, G, Cp)` for either a single species or reaction.
    pub fn eval_expr_si(&self, expr: &str, temp_user: f64) -> Result<ThermoPointSi> {
        if is_reaction(expr) {
            self.eval_reaction_si(expr, temp_user)
        } else {
            self.eval_species_si(expr, temp_user)
        }
    }

    /// Evaluate single species SI properties at user temperature `temp_user`.
    pub fn eval_species_si(&self, species: &str, temp_user: f64) -> Result<ThermoPointSi> {
        let temp_k = self.units.temp_to_kelvin(temp_user)?;
        let intervals = self.db.get_species_intervals(species)?;
        integrate_species_intervals(&intervals, temp_k)
    }

    /// Evaluate reaction ΔH°, ΔS°, ΔG°, ΔCp° in SI at user temperature `temp_user`.
    pub fn eval_reaction_si(&self, reaction: &str, temp_user: f64) -> Result<ThermoPointSi> {
        let temp_k = self.units.temp_to_kelvin(temp_user)?;
        let parsed = parse_reaction(reaction)?;

        let mut h_kj = 0.0;
        let mut s_jk = 0.0;
        let mut g_kj = 0.0;
        let mut cp_jk = 0.0;

        for term in &parsed.reactants {
            let intervals = self.db.get_species_intervals(&term.species.raw)?;
            let pt = integrate_species_intervals(&intervals, temp_k)?;
            h_kj -= term.coeff * pt.h_kj_mol;
            s_jk -= term.coeff * pt.s_j_mol_k;
            g_kj -= term.coeff * pt.g_kj_mol;
            cp_jk -= term.coeff * pt.cp_j_mol_k;
        }
        for term in &parsed.products {
            let intervals = self.db.get_species_intervals(&term.species.raw)?;
            let pt = integrate_species_intervals(&intervals, temp_k)?;
            h_kj += term.coeff * pt.h_kj_mol;
            s_jk += term.coeff * pt.s_j_mol_k;
            g_kj += term.coeff * pt.g_kj_mol;
            cp_jk += term.coeff * pt.cp_j_mol_k;
        }

        Ok(ThermoPointSi {
            temp_k,
            h_kj_mol: h_kj,
            s_j_mol_k: s_jk,
            g_kj_mol: g_kj,
            cp_j_mol_k: cp_jk,
        })
    }

    fn resolve_formula_alias(&self, input: &str) -> Result<String> {
        if parse_species(input).is_ok() {
            return Ok(input.to_string());
        }
        if let Some(mineral) = self.db.get_mineral(input)? {
            return Ok(mineral.formula);
        }
        Ok(input.to_string())
    }
}

/// Integrate piecewise Kelley polynomial `Cp(T)` from reference `298.15 K` up (or down) to `temp_k`.
pub fn integrate_species_intervals(
    intervals: &[SpeciesInterval],
    temp_k: f64,
) -> Result<ThermoPointSi> {
    if intervals.is_empty() {
        return Err(ChemAddinError::SpeciesNotFound("empty interval slice".into()));
    }
    if !temp_k.is_finite() || temp_k <= 0.0 {
        return Err(ChemAddinError::InvalidTemperature {
            temp_k,
            detail: "temperature must be finite and > 0 K".into(),
        });
    }

    // Identify reference interval covering 298.15 K (fallback to first interval if T_min > 298.15).
    let ref_idx = intervals
        .iter()
        .position(|iv| iv.t_min <= T_REF_K + 1e-6 && T_REF_K <= iv.t_max + 1e-6)
        .unwrap_or(0);

    let mut h_j = intervals[ref_idx].h298 * 1000.0;
    let mut s_j = intervals[ref_idx].s298;
    let cp_j: f64;

    if temp_k >= T_REF_K {
        let mut t_curr = T_REF_K;
        let mut active_idx = ref_idx;

        for idx in ref_idx..intervals.len() {
            let iv = &intervals[idx];
            if idx > ref_idx {
                h_j += iv.dh_tr * 1000.0;
                s_j += iv.ds_tr;
                t_curr = iv.t_min;
            }
            active_idx = idx;
            let is_last = idx + 1 == intervals.len();
            let t_end = if is_last || temp_k <= iv.t_max {
                temp_k
            } else {
                iv.t_max
            };

            h_j += iv.integrate_enthalpy_j(t_curr, t_end);
            s_j += iv.integrate_entropy_j(t_curr, t_end);

            if is_last || temp_k <= iv.t_max {
                break;
            }
        }
        cp_j = intervals[active_idx].cp_j(temp_k);
    } else {
        // Integrate downward from 298.15 K across any lower intervals down to `temp_k`.
        let mut t_curr = T_REF_K;
        let mut active_idx = ref_idx;

        for idx in (0..=ref_idx).rev() {
            let iv = &intervals[idx];
            active_idx = idx;
            let is_first = idx == 0;
            let t_end = if is_first || temp_k >= iv.t_min {
                temp_k
            } else {
                iv.t_min
            };

            h_j += iv.integrate_enthalpy_j(t_curr, t_end);
            s_j += iv.integrate_entropy_j(t_curr, t_end);

            if is_first || temp_k >= iv.t_min {
                break;
            }
            h_j -= iv.dh_tr * 1000.0;
            s_j -= iv.ds_tr;
            t_curr = iv.t_min;
        }
        cp_j = intervals[active_idx].cp_j(temp_k);
    }

    let h_kj_mol = h_j / 1000.0;
    let g_kj_mol = h_kj_mol - temp_k * (s_j / 1000.0);

    Ok(ThermoPointSi {
        temp_k,
        h_kj_mol,
        s_j_mol_k: s_j,
        g_kj_mol,
        cp_j_mol_k: cp_j,
    })
}
