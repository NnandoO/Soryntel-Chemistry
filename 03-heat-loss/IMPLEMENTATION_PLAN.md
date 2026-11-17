# Plan de Implementación en Rust (`IMPLEMENTATION_PLAN.md`) — Módulo 3

Este plan integra el **Módulo 3 (Heat Loss / Heat Transfer Calculations)** dentro del Cargo Workspace existente en [`/home/nando/Documentos/HSC-REA/soryntel/`](file:///home/nando/Documentos/HSC-REA/soryntel/), reutilizando los tipos, errores y arquitectura de los Módulos 1 y 2.

---

## 1. Reutilización vs. Nuevos Componentes en el Workspace `soryntel`

| Crate | Reutilizado de Módulos 1 y 2 | Nuevo para Módulo 3 |
|---|---|---|
| **`soryntel-core`** | `TemperatureUnit` (`Celsius`, `Kelvin`), `BalanceEnergyUnit`, `SoryntelError`, constantes `T_REF_K` (`298.15`), `CAL_TO_JOULE` (`4.184`). | `HeatFlowUnit` (`W`, `kW`, `MW`, `kcal/h`, `BTU/h`), `LengthUnit` (`m`, `cm`, `mm`, `in`), `STEFAN_BOLTZMANN` (`5.670374419e-8`), `GRAVITY_STANDARD` (`9.80665`). |
| **`soryntel-db`** | Conexión SQLite y patrón de repositorio/consulta. | Importador `calamine` (Rust puro) + esquema SQLite para `heatcond.xls` (`SolidConductivityTable`), `heatconv.xls` (`FluidPropertyTable`, `ConvectionFunction`), `heatradi.xls` (`SurfaceEmissivity`). |
| **`soryntel-thermo`** | `balance_engine` (consumidor del flujo exportado hacia `BalanceArea` en Módulo 2). | Submódulo `heat_loss` (o crate dedicado `soryntel-heat` si se prefiere desacoplamiento total): conducción 1D, convección, radiación superficial, EWBM gas, partículas, resolvedor de red térmica, `Target` y `Diagram`. |
| **`soryntel-desktop`** | Shell `egui` + `egui_plot`, tema claro, `ActiveModule::{ReactionEquations, HeatBalance}`. | Nueva variante `ActiveModule::HeatLoss`, vista `heat_loss_ui.rs` (grilla multicapa, panel inferior de 3 bloques, modales `Cond`/`Conv`/`Rad`/`GasRad`/`ParticleRad`/`Target`/`Diagram`), acción de exportación al Módulo 2. |

---

## 2. Arquitectura Modular Propuesta (`soryntel_thermo::heat_loss`)

```text
crates/soryntel-thermo/src/heat_loss/
├── mod.rs              // API pública: HeatLossWorkbook, HeatLossSheet, solve_sheet, solve_workbook
├── model.rs            // Geometry (Wall, Cube, Cylinder, Sphere), ColumnType, LayerSpec, FixState
├── conduction.rs       // Ecs. (8)-(12): malla m, factores de forma S, k(T) por interpolación lineal
├── convection.rs       // Ecs. (13)-(40): Free/Forced, Churchill-Chu, Raithby-Hollands, Whitaker, Function hc(v)
├── radiation.rs        // Ecs. (41)-(43): radiación superficial gris y recintos
├── gas_radiation.rs    // Ecs. (44)-(47) + Tabla 1: EWBM (Edwards & Balakrishnan / Lallemant & Weber) + partículas (48)-(53)
├── solver.rs           // Resolvedor acoplado HeatFlow (sub-relajación) y TemperatureProfile (Brent/secante)
├── target.rs           // Búsqueda de raíz 1D (Brent) para Target Dialog
└── diagram.rs          // Generación de perfiles T(x), barridos paramétricos y curva de aislamiento óptimo
```

---

## 3. Fases de Implementación Recomendadas

1. **Fase 3.1 — Tipos y Constantes (`soryntel-core` + `model.rs`):**
   - Definir `Geometry`, `ColumnType`, `HeatLossColumn`, `HeatLossSheet`, `HeatLossWorkbook`, unidades de flujo térmico y longitud.
2. **Fase 3.2 — Ingesta de Bases de Datos (`soryntel-db`):**
   - Leer `heatcond.xls`, `heatconv.xls`, `heatradi.xls` con `calamine` (sin Excel/COM) y persistir/cachear en SQLite de `soryntel-db` con interpolación lineal por tramos $f(T)$ y validación de rango $[T_{\min}, T_{\max}]$.
3. **Fase 3.3 — Motores Físicos Independientes:**
   - `conduction.rs`: discretización de $m$ subcapas y geometrías `Wall`/`Cube`/`Cylinder`/`Sphere`.
   - `convection.rs`: todas las correlaciones de las Ecs. (13)–(40) + pestaña `Function` $h_c(v) = \sum c_j v^{p_j}$.
   - `radiation.rs` y `gas_radiation.rs`: radiación superficial, EWBM para $\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$ y nube de partículas.
4. **Fase 3.4 — Resolvedor Acoplado, Target y Diagram:**
   - Implementar `solve_sheet` (`HeatFlow` y `TemperatureProfile`), `solve_workbook`, `solve_target` (Brent) y `generate_diagram`.
   - Ejecutar la batería completa de `VALIDATION_PLAN.md` con `approx`.
5. **Fase 3.5 — Interfaz de Escritorio (`soryntel-desktop`) e Integración con Módulo 2:**
   - Añadir `ActiveModule::HeatLoss` en `main.rs` y `heat_loss_ui.rs`.
   - Conectar el botón **`Send Total Heat Flow to Balance (Module 2)`** con `balance_engine` para inyectar/actualizar automáticamente la pérdida térmica del `BalanceArea`.

---

## 4. Dependencias Rust

- **Ya presentes en el workspace:** `serde`, `serde_json`, `thiserror`, `approx`, `egui`, `egui_plot`, `rusqlite`.
- **Nuevas sugeridas (100% Rust puro, compilación nativa Linux y Windows):**
  - `calamine` (solo lectura de `heatcond.xls`, `heatconv.xls`, `heatradi.xls` para importar a SQLite).
  - Ninguna dependencia C/Fortran externa requerida: el resolvedor 1D, Brent y EWBM se implementan directamente en `f64` puro.
