# Plan de Implementación en Rust — Módulo 4: Equilibrium Calculations

## 1. Integración en el Cargo Workspace (`soryntel`)

### 1.1 `crates/soryntel-core` (Reutilización + Nuevos Tipos)
- **Reutilizado:** `TemperatureUnit`, `EnergyUnit`, `SoryntelError`, `T_REF_K`, `CAL_TO_JOULE`, `R_JOULE`, `FARADAY_C_MOL`, `STP_MOLAR_VOL_L`.
- **Nuevo:** `EquilibriumAmountUnit` (`Kmol`, `Kg`, `Nm3`, `GramPerLiter`, `MolPerLiter`), `ThermodynamicMode` (`Ntp`, `Nhp`, `Npv`, `Ntpq`), `PhaseModel` (`Mixture`, `Pure`, `Aqua`, `GaAsMixture`, `AlZnMixture`).

### 1.2 `crates/soryntel-db` (Reutilización Total)
- **Reutilizado:** `ThermoDatabase` y el trait `SpeciesRepository` (`Maindb6.HSC`), añadiendo únicamente un método helper de búsqueda multi-elemento (`search_by_elements(elements, mode_and, include_ions, include_organics)`) usando los campos ya parseados.

### 1.3 `crates/soryntel-thermo` (Nuevo submódulo `equilibrium/`)
Reutiliza `ChemicalSpecies`, `Phase`, `evaluate_point_si`, `evaluate_species` y `balance_engine` (matriz estequiométrica/atómica):
- `equilibrium/mod.rs`: API pública `solve_equilibrium(system, params) -> Result<EquilibriumSweepResult, SoryntelError>`.
- `equilibrium/model.rs`: estructuras `EquilibriumSystem`, `EquilibriumPhase`, `EquilibriumSpeciesInput`, `SweepAxis`, `TargetSpec`, `CellPhaseConfig`, `EquilibriumPointResult`.
- `equilibrium/gem_solver.rs`: solver **White–Johnson–Dantzig (1958)** + conjunto activo de fases puras (`nalgebra::DMatrix` con pivoteo LU/QR para el sistema reducido $(M + |\mathcal{P}_{\text{mix}}| + |\mathcal{A}|)$).
- `equilibrium/activity.rs`: evaluador `ActivityModel` (`Ideal`, `RedlichKister` con presets `Ga-As` y `Al-Zn`, `Aqua` Debye–Hückel/Pitzer, y parser seguro de fórmulas en Columna C).
- `equilibrium/outer_solvers.rs`: lazo de `AC Steps`, bisección/Brent para `N,H,P` (Adiabático) y `N,P,V` (Volumen constante), `Target Calculations`, `Infinite Phase`, y `Cell Mode` (`N,T,P,Q`).
- `equilibrium/igi_parser.rs`: lector de archivos `.IGI`/`.OGI` de HSC6 para pruebas automáticas de regresión.

### 1.4 `crates/soryntel-desktop` (Nueva pestaña `Módulo 4: Equilibrium`)
- **Reutilizado:** `excel_grid.rs` (`col_letter`, `header_cell`, `row_num_cell`, `section_cell`, `text_cell`, `widget_cell`) para las hojas `Species`, `Results` y el diálogo `Warnings`.
- **Nuevo:** `equilibrium_ui.rs` (`EquilibriumState`, tabla Excel `A..H`, panel acoplado `Parameters`, selector `From Elements` + `Sort Species`, gráfico 2D con `egui_plot` / tabla 3D).

## 2. Dependencias Nuevas Estimadas
- `nalgebra` (álgebra lineal densa pura en Rust para el sistema lineal WJD, sin dependencias C/Fortran).
- `approx` (dev-dependency en `soryntel-thermo` para `VALIDATION_PLAN.md`).
