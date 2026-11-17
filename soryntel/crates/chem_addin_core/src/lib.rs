//! Clean-room Rust core engine for HSC-style Excel Add-Ins (Entrega 1).
//!
//! Provides:
//! - Session unit state (`UnitSystem`: `EnergyU`, `TempU`, `PresU`)
//! - SQLite storage (`species_thermo`, `minerals`, `mineral_elements`) with real seed data
//! - Chemical species & reaction parser (parentheses, `*` hydrates, phase tags `(g)`/`(l)`/`(s)`/`(aq)`, ions)
//! - Matrix stoichiometry (`mw`, `wtp`, `bal` via `nalgebra` SVD nullspace)
//! - Thermochemistry (`h`, `s`, `g`, `k`, `cp`) using piecewise analytical integration of Kelley `Cp(T)`

pub mod db;
pub mod elements;
pub mod error;
pub mod parser;
pub mod stoichiometry;
pub mod thermo;
pub mod units;

pub use db::{ChemDatabase, MineralRow, SpeciesInterval};
pub use elements::atomic_weight;
pub use error::{ChemAddinError, Result};
pub use parser::{
    is_reaction, parse_reaction, parse_species, ParsedReaction, ParsedSpecies, PhaseTag,
    ReactionTerm,
};
pub use stoichiometry::{bal, bal_detailed, mw, verify_reaction_balance, wtp, BalanceResult};
pub use thermo::{integrate_species_intervals, ChemEngine, ThermoPointSi};
pub use units::{EnergyUnit, PressureUnit, TempUnit, UnitSystem, CAL_TO_JOULE, R_JOULE, T_REF_K};
