# Plan de Implementación en Rust (`IMPLEMENTATION_PLAN`) — Módulo 2

## 1. Integración con el Cargo Workspace `soryntel`

El Módulo 2 se integrará directamente en el workspace existente `/home/nando/Documentos/HSC-REA/soryntel/` respetando la separación estricta entre **motor científico puro** y **capa de presentación (`soryntel-desktop`)**.

---

## 2. Reutilización vs. Extensiones por Crate

### 2.1 `soryntel_core`
- **Reutilizado del Módulo 1:**
  - `T_REF_K = 298.15`, `CAL_TO_JOULE = 4.184`, `R_JOULE`, `STP_MOLAR_VOL_L = 22.414`.
  - `TemperatureUnit::{Celsius, Kelvin, Fahrenheit}`, `SoryntelError`.
- **Nuevo para el Módulo 2:**
  - Ampliar `EnergyUnit` (o crear `BalanceEnergyUnit`) con `Mcal`, `MJ`, `kWh` y factores de conversión exactos (`1 kWh = 3.6 MJ`, `1 Mcal = 4.184 MJ`).
  - Nuevas variantes en `SoryntelError`: `FormulaSyntaxError`, `CircularConvergenceFailed`, `TargetNotBracketed`, `InvalidCellReference`.

### 2.2 `soryntel_thermo`
- **Reutilizado del Módulo 1:**
  - `SpeciesRepository` (`get_species`, `get_reference_element`).
  - `evaluate_species_si(species, t_k, criss_cobble, upper_side_on_boundary) -> Result<(ThermoPointSI, String), SoryntelError>`.
- **Nuevo para el Módulo 2:**
  - Módulo `thermo::balance_props`:
    - `evaluate_heat_content_si(repo, species, t_k) -> Result<HeatContentPoint, SoryntelError>`: calcula $h_{\text{cont}}(T) = H(T) - H(298.15\text{ K})$, $h_{\text{tot}}(T) = H(T)$, $T \cdot S(T)$, $G(T)$.
  - Módulo `thermo::water_iapws`:
    - Implementación pura en Rust de **IAPWS-IF97** (`h2o_properties_si(t_k, p_bar) -> Result<ThermoPointSI, SoryntelError>`) para soportar `Pressure correction H2O` en líquido, vapor y saturación.
  - Módulo `thermo::exergy`:
    - Tabla de exergías químicas estándar de Szargut et al. (1988) y cálculo de `Ex Phys` y `Ex Chem`.

### 2.3 `soryntel_solver`
- **Reutilizado del Módulo 1:**
  - `parse_formula_composition(formula: &str) -> Result<BTreeMap<String, f64>, SoryntelError>` y `parse_term` para descomposición elemental de cada especie en `Element Balance` y hojas `Inx-%` / `Outx-%`.
- **Nuevo para el Módulo 2 (módulo `solver::balance`):**
  1. `BalanceWorkbook`, `BalanceArea`, `Stream`, `SpeciesRow`, `HeatFlowRow`, `CustomSheet`: estructuras serializables (`serde`) del modelo de datos.
  2. **Motor de fórmulas (`formula_eval.rs`):**
     - Parser AST de expresiones tipo Excel (`=0.938*(InCu-D13)/2`, `='OUT4'!D12`, `=InCa2*Analysis!L28/100`, `SUM`, `IF`, `ABS`).
     - Detección de ciclos con **Tarjan SCC** y convergencia por sustitución sucesiva + aceleración de Anderson.
  3. **Evaluador de Balance (`engine.rs`):**
     - Cálculo de `Amount kg`, `Amount Nm³`, `Heat Content`, `Total H`, `T*S`, `G`, `Ex Phys`, `Exergy`.
     - Cálculo de `Element Balance` y `Total Balance` por área y global.
     - Generación de resúmenes `Inx-%` / `Outx-%`.
  4. **Solvers numéricos (`numeric.rs`):**
     - `solve_adiabatic_temperature(...)` usando **Brent** con soporte de discontinuidades de cambio de fase.
     - `solve_target_rows(...)` (`Iterate Selected Rows` / `Iterate All`) con Brent 1D acotado + Gauss-Seidel / Broyden para filas acopladas.
     - `evaluate_diagram_sweep(...)` con `Target iteration` anidado por punto.

### 2.4 `soryntel_db`
- **Reutilizado del Módulo 1:**
  - `ThermoDatabase` (implementación de `SpeciesRepository`, `search(query, limit)` para los botones `Get Species from Database: Input / Output`).
- **Nuevo para el Módulo 2:**
  - Cache en memoria opcional de especies frecuentes en un `BalanceWorkbook` para acelerar evaluaciones iterativas (`Temp. Balance`, `Target`, `Diagram`) a cientos de evaluaciones por milisegundo.

### 2.5 `soryntel-desktop` (Tauri v2 + UI)
- **Reutilizado del Módulo 1:**
  - Ventana principal y barra superior de aplicación; componente buscador de especies conectado por comandos Tauri a `ThermoDatabase::search`.
- **Nuevo para el Módulo 2:**
  - Selector de módulo (`Reaction Equations` | `Heat & Material Balance`).
  - Grilla virtualizada estilo hoja de cálculo con barra de fórmulas, pestañas (`BALANCE`, `INx`, `OUTx`, `Inx-%`, `Outx-%`, hojas personalizadas), codificación de colores (azul stream, verde flujos térmicos, rojo entradas `Red Font Shield`, amarillo objetivos).
  - Paneles acoplados inferiores (`Element Balance`, `Total Balance`) y panel lateral derecho `Tools`.
  - Modales `Temperature Balance`, `Target Dialog` y `Diagram Settings` + visor gráfico `Diagram` interactivo.
  - Comandos Tauri: `bal_evaluate_workbook`, `bal_solve_temp_balance`, `bal_run_target`, `bal_run_diagram`, `bal_import_ohe_or_bal`, `bal_export_report`.

---

## 3. Fases de Desarrollo Recomendadas

| Fase | Entregable Técnico | Criterio de Terminación |
|---|---|---|
| **Fase 2.1** | `thermo::balance_props` + `water_iapws` + `exergy` | Tests unitarios contra `BUTANE.OHE` y `H2O.OHE` pasan con `approx`. |
| **Fase 2.2** | `solver::balance` (`Workbook`, `formula_eval`, Tarjan SCC, `Element/Total Balance`) | `CUCONV1` y `CUSMELT` reproducen exactamente masas y entalpías. |
| **Fase 2.3** | `Temp. Balance` (Brent), `Target Dialog`, `Diagram` sweep, multi-área (`FSF_process`) | Convergencia de temperatura adiabática y ciclos multi-área validada. |
| **Fase 2.4** | Integración Tauri + UI en `soryntel-desktop` | Flujo completo interactivo (editar celdas, fórmulas, Target, Diagram) funcionando nativamente. |
