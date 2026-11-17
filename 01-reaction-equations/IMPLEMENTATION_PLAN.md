# IMPLEMENTATION_PLAN — Reaction Equations in Soryntel Chemistry (Rust)

## 1. Arquitectura de Crate en Rust (Cargo Workspace)

La implementación se distribuirá en crates independientes y desacoplados para garantizar la pureza matemática del motor científico y facilitar su reutilización en futuros módulos de Soryntel Chemistry:

```
soryntel-workspace/
├── crates/
│   ├── soryntel-core/          # Tipos dimensionales, unidades (uom / newtypes), constantes físicas
│   ├── soryntel-thermo/        # Motor termodinámico: polinomios Kelley, integrales H, S, G, Criss-Cobble
│   ├── soryntel-db/            # Acceso a SQLite de propiedades termodinámicas y especies
│   ├── soryntel-solver/        # Algoritmos numéricos: balance estequiométrico (SVD/Gauss)
│   └── soryntel-tauri/         # Frontend Tauri v2: IPC Commands, estado de aplicación y serialización
```

---

## 2. Definición de Tipos, Structs y Traits en Rust Puro

### 2.1. Crate `soryntel-thermo`: Coeficientes y Especie Termodinámica

```rust
use serde::{Deserialize, Serialize};

/// Rango térmico de validez para los coeficientes de Kelley
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureRange {
    pub t_min: f64, // Kelvin
    pub t_max: f64, // Kelvin
}

/// Coeficientes polinomiales empíricos de Kelley:
/// Cp(T) = A + B*1e-3*T + C*1e5*T^-2 + D*1e-6*T^2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KelleyCoefficients {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

/// Datos termodinámicos de una especie para un intervalo de temperatura dado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesThermoInterval {
    pub range: TemperatureRange,
    pub kelley: KelleyCoefficients,
    /// En el primer intervalo (k = 0): Hf°(298.15 K) en J/mol.
    /// En intervalos posteriores (k >= 1): ΔH_tr en la frontera t_min en J/mol.
    pub h_or_delta_h_tr: f64,
    /// En el primer intervalo (k = 0): S°(298.15 K) en J/(mol*K).
    /// En intervalos posteriores (k >= 1): ΔS_tr en la frontera t_min en J/(mol*K).
    pub s_or_delta_s_tr: f64,
    pub reference: String,
}

/// Estado o fase física de agregación
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    AqueousIon,
    AqueousNeutral,
}

/// Especie química termodinámica completa
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalSpecies {
    pub formula: String,
    pub name: String,
    pub phase: Phase,
    pub molecular_weight: f64, // g/mol
    pub charge: i32,            // Carga eléctrica neta (+1, -2, etc.)
    pub intervals: Vec<SpeciesThermoInterval>,
}
```

### 2.2. Trait Termodinámico de Evaluación Continua

```rust
pub trait ThermodynamicEvaluator {
    /// Evalúa la capacidad calorífica molar Cp a temperatura T (K)
    fn cp(&self, t_kelvin: f64) -> Result<f64, ThermoError>;

    /// Evalúa la entalpía molar absoluta H(T)
    fn enthalpy(&self, t_kelvin: f64) -> Result<f64, ThermoError>;

    /// Evalúa la entropía molar absoluta S(T)
    fn entropy(&self, t_kelvin: f64) -> Result<f64, ThermoError>;

    /// Evalúa la energía libre de Gibbs molar G(T) = H(T) - T * S(T)
    fn gibbs_energy(&self, t_kelvin: f64) -> Result<f64, ThermoError> {
        let h = self.enthalpy(t_kelvin)?;
        let s = self.entropy(t_kelvin)?;
        Ok(h - t_kelvin * s)
    }
}
```

### 2.3. Estructuras de Reacción y Balance

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionComponent {
    pub species: ChemicalSpecies,
    pub stoichiometric_coeff: f64, // Negativo para reactivos, positivo para productos
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalReaction {
    pub raw_equation: String,
    pub components: Vec<ReactionComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionStepResult {
    pub temperature_k: f64,
    pub delta_h: f64,       // J/mol o cal/mol
    pub delta_s: f64,       // J/(mol*K) o cal/(mol*K)
    pub delta_g: f64,       // J/mol o cal/mol
    pub k_eq: f64,          // Constante de equilibrio termodinámico
    pub log10_k: f64,       // log10(K)
    pub cell_potential_v: Option<f64>, // E° vs SHE en voltios (si n_e > 0)
}
```

---

## 3. Comandos IPC en Tauri v2 (`soryntel-tauri`)

Los cálculos científicos pesados se ejecutan de manera asíncrona en hilos de trabajo de Tokio / Rayon, devolviendo estructuras fuertemente tipadas al frontend:

```rust
#[tauri::command]
pub async fn calculate_reaction_table(
    request: ReactionCalculationRequest,
) -> Result<ReactionCalculationResponse, String> {
    tokio::task::spawn_blocking(move || {
        soryntel_thermo::compute_reaction_profile(&request)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn balance_chemical_equation(
    raw_equation: String,
    smallest_coeff: f64,
) -> Result<String, String> {
    soryntel_solver::balance_equation(&raw_equation, smallest_coeff)
        .map_err(|e| e.to_string())
}
```
