use thiserror::Error;

/// Unified error type returned by `chem_addin_core` functions and database queries.
#[derive(Debug, Error)]
pub enum ChemAddinError {
    #[error("Invalid chemical formula '{0}'")]
    InvalidFormula(String),

    #[error("Unknown chemical element '{0}'")]
    UnknownElement(String),

    #[error("Species '{0}' not found in species_thermo database")]
    SpeciesNotFound(String),

    #[error("Mineral '{0}' not found in minerals database")]
    MineralNotFound(String),

    #[error("Invalid chemical reaction '{0}'")]
    InvalidReaction(String),

    #[error("Reaction cannot be uniquely balanced: {0}")]
    UnbalanceableReaction(String),

    #[error("Invalid temperature ({temp_k:.4} K): {detail}")]
    InvalidTemperature { temp_k: f64, detail: String },

    #[error("Invalid thermodynamic unit '{0}'")]
    InvalidUnit(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Numerical error: {0}")]
    Numeric(String),

    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ChemAddinError>;
