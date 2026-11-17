# Especificación de Interfaz de Usuario (UI) — Módulo 20: Mass Balance (`HSC - Mass Balance`)

Especificación estructural e interactiva derivada de las capturas oficiales `Figs. 1–28` y del catálogo de controles `51.5` en [51-mass-balance.md](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.md) (`/home/nando/Documentos/Docs HSC/51-mass-balance/imagenes/`), junto con su plano de arquitectura en `soryntel-desktop` (`egui` / `eframe`).

---

## 1. Arquitectura General de Ventana (`Fig. 2`)

La ventana **`HSC Mass Balance`** se organiza en `4` regiones acoplables fijas/colapsables:

1. **Cinta Superior (`Upper Buttons`):**
   - Bloque permanente a la izquierda **`Main Menu`**:
     `Save to Flowsheet ▾` | `Discard and Close` | `Backup ▾` | `Import HSC7 Data` *(en vista inicial / Experimental Data)* | `Import ReX Data` | `Check for Errors` | `Help ▾`.
   - Bloques contextuales a la derecha que cambian dinámicamente según el paso seleccionado en el navegador.
2. **Navegador Izquierdo (`Balancing Navigator`):** panel vertical con `5` acordeones colapsables y contadores dinámicos (`sel/tot`):
   - **`Experimental Data`**: `Units (x/y)`, `Streams (x/y)`, `Variables (x/y)`, `Size Fractions (nombre/conteo)`, `Data Sets (No. i of n)`, `Measurement Data (x/y)`
   - **`Review Data`**: `Data Status`
   - **`Standard Deviations`**: `Edit SD Table`, `Review SD`
   - **`Balancing`**: `Balance (m Equations)`, `Compare` *(deshabilitado si $N_{\text{datasets}} < 2$)*
   - **`Reporting`**: `Results`, `Charts`
   - El ítem activo se resalta con barra/fondo salmón-naranja (`#F5A882` aprox., `Figs. 2, 13, 15`).
3. **Área de Trabajo Central (`Working Area`):** grilla tipo hoja de cálculo (con encabezados de columna `A, B, C...`, barra de celda activa `A1 ▼` en vistas SD/Mediciones/Reportes) más zona inferior para gráficos acoplados en `Data Status`, `Review SD` y `Balance`.
4. **Panel Derecho (`Property Panel` / `Properties`):** inspector clave-valor contextual de solo lectura/edición según selección activa.

---

## 2. Desglose de Vistas, Botones Contextuales y Grillas (`51.4–51.5`)

### 2.1. `Experimental Data -> 1. Units` (`Figs. 2, 4`)
- **Grilla central:** columnas `[Selected ☑]`, `Unit Name`, `[Size Transforming ☐]`.
- **`Properties`:** sección `Unit` (`Size Transforming` checkbox, `Selected` checkbox).

### 2.2. `Experimental Data -> 2. Streams` (`Fig. 5`)
- **Cinta contextual (`Streams`):** `Detect Stream Types`, `Move Up`, `Move Down`, `Unselect Streams with no Data`.
- **Grilla central:** columnas `Selected`, `Stream Name`, `Type` *(desplegable: `Unknown`, `Solids/Slurry`, `Liquid/Water`)*, `Flows` *(check)*, `Assays` *(check)*, `Sampling Device` *(ej. `Default`)*, `Sample Quality` *(desplegable: `Good` verde, `Moderate`, `Bad`)*.

### 2.3. `Experimental Data -> 3. Variables` (`Fig. 6`, `51.5`)
- **Cinta contextual (`Variables`):** `Add Variable ▾`, `Remove Variable`, `Move Up`, `Move Down`, `Add Elements...` *(abre selector de tabla periódica)*, `Add Mineral from DB...` *(selector mineralógico)*, `Import Minerals from Stream` *(`*.HSCStream`)*.
- **Grilla central:** `Selected`, `Variable Name`, `Meas. Unit`, `Type Code` (`SF`, `SC`, `A`, `M`, `SA`, `SP`, `Info`), `Type`, `Measurement Method`, `a`, `b`, `c`.

### 2.4. `Experimental Data -> 4. Size Fractions` (`Fig. 7`) & `5. Data Sets` (`Fig. 8`)
- **Size Fractions:** grupo contextual `Size Fractions`: `Add Size Fractions ▾`, `Remove Size Fractions` *(deshabilitado sobre la fila base `'Bulk'`, solo renombrable)*.
- **Data Sets:** grupo contextual `Data Sets`: `Add Data Set`, `Remove Data Set`, `Clone Data Set`. Lista editable de nombres y selección de dataset activo.

### 2.5. `Experimental Data -> 6. Measurement Data` (`Fig. 9`)
- **Cinta contextual (`3` grupos):**
  - `Table`: `Copy ▾`, `Paste Experimental Data`, `Flip Table` *(transponer)*, `Lock/Unlock Table`, `Move Up`/`Move Down` filas/columnas.
  - `Data Tools`: `Element to Mineral...`, `View Mineral Matrix`, `Sol-% to Water`, `Show SG`, `Stream by Equation`, `Estimate Missing Values`, `Clear Estimated Values`, `Enter Recoveries`.
  - `Sampling Rounds`: `Add Round`, `Remove Round`, `Calculate Master Data`.
- **Grilla central:** `Stream` en columna A (o filas tras `Flip Table`), columnas dinámicas `NombreVariable + Unidad` (`Mass g`, `Cu %`, ...). Los valores estimados se pintan en **fuente azul** (`Fig. 9`).

### 2.6. `Review Data -> Data Status` (`Figs. 10–12`)
- **Cinta contextual (`2` grupos):**
  - `Data Status`: `Balance Equations` *(modal lista expandible `RC Feed = RT1 + RC1`, `Fig. 11`)*, `Unselect Streams with no Data`, `Balancing of Minerals` *(botón alternable tipo toggle)*, `Reset Meas. Status`, `Stream by Equation`.
  - `View`: `Refresh Charts`, `Layout: Stream View ▾`.
- **Área superior:** grilla jerárquica colapsable (`Stream` ▼ / `Variable`) con subcolumnas por Dataset: `Measured`, `Meas. Status` *(desplegable: `Missing`, `Measured`, `Guesstimated`, `Fixed`, `Excluded`)*, `Bal. Status` *(`Balanced`, `Calculated`, `Non Available`)*.
- **Área inferior:** dos gráficos circulares lado a lado (`Measured Data` vs `Balanced Data`, `Fig. 10`).

### 2.7. `Standard Deviations -> Edit SD Table` (`Fig. 13`) & `Review SD` (`Fig. 14`)
- **`Edit SD Table`:** grupo `Table`: `Copy ▾`, `Paste SD or RSD Data`, `Calculate SD From Data`, `Lock/Unlock Table`. Grilla `Stream | Var1 SD | Var2 SD ...` con barra `A1 ▼`; valores automáticos en azul, sobrescrituras manuales en negro.
- **`Review SD`:** grilla jerárquica (`Dataset` → `Stream` | `Variable` | `Measurement Method ▾` | `Measured` | `RSD-%` | `SD` | `Min` | `Max`) + par de gráficos inferiores de barras (`Standard Deviations` izquierda y `Relative Standard Deviations` derecha por corriente seleccionada).

### 2.8. `Balancing -> Balance` (`Figs. 15–19`)
- **Cinta contextual (`3` grupos):**
  - `Settings`: `Mineral Sum=100` *(toggle)*, `Assay Sum=100` *(toggle)*, `Low Grade Weighting ▾` *(popover con campo `Coefficient` defecto `0.7`, checklist `Select weighted variables`, botón `Apply Coefficient`, `Fig. 19`)*, `Method: LS ▾` (`LS`, `LS, MinMax`, `NNLS`, `CLS`, `LLS`).
  - `Run Balancing`: `Sizing: Unsized ▾` (`Unsized`, `Sized`, `Sized by Assay`), `Balance Type: Solids and Assays ▾` (`Solids only`, `Assays only`, `Solids and Assays`, `Solids, Water and Assays`), `Calculate`, `Freeze Balanced`, `Clear & Unfreeze Balanced`, `Batch Run ▾`.
  - `Data Tools`: `Balanced Elements to Minerals...`, `Balance Equations`, `Show Data Bars`.
- **Grilla central (`Data Set k`):** `Stream | Variable | Measured | Balanced (azul) | ± CI | Diff | RelDiff % | Min | Max | RSD-% | SD | Recovery-%`.
- **Gráficos inferiores (`Fig. 15`):**
  - Izquierda: **`Balance Convergence`** (`Iterated Tolerance` en escala logarítmica vs `Calculation Round`, línea discontinua horizontal en `Balance Tolerance`).
  - Derecha: **`Parity Chart`** (`Bal.` vs `Meas.` en escala log-log con recta diagonal `1:1` verde y botón `Log` toggle).
- **`Property Panel` (`Figs. 15–16`):** mitad superior (`Data`, `Values`, `Balanced Values` de la celda seleccionada) + mitad inferior (`Calculation Settings`: `Method`, `Mineral Sum=100`, `Assay Sum=100`, `Elements excluded`, `Balance Type`, `Sizing`, `Scaling`; `Algorithm Parameters`: `Max Iteration`, `Balance Tolerance`, `Estimate of Null SD`; `Balancing Summary`: `Number of Meas.`, `Number of Bal.`, `Number of zeros`, `Number of negatives`, `WSSQ`, `RMSD`, `Relative RMSD`).

### 2.9. `Reporting -> Results` (`Figs. 20–23`) & `Charts` (`Fig. 24` imagen / `Figs. 22–23` manual)
- **Cinta contextual (`Reporting` / `Charts`):**
  - En `Results`: `Copy ▾` (`Copy selection`, `Copy all`), `Calculate Volumetric Flows ▾`, `Calculate All Mineral Elements`, `Stream Tables ▾` *(actualizar tablas en HSC Sim)*, `Export to Excel`, `Save Stream As...`.
  - Pestañas inferiores de hoja en `Results`: `Stream Summary` (`Fig. 20`), `Goodness` (`Fig. 21`), `Unit Balance` (`Fig. 22`), `Balance details` (`Fig. 23`).
  - En `Charts`: `Add New Panel`, `Add Bar Chart`, `Add Default Charts ▾` (`Total Solids`, `Stream Composition`), `Chart Properties`, `Reset`. Leyenda superior interactiva con checkboxes por serie (`Ccp % Bal.`, `Py % Bal.`, etc.).

### 2.10. Diálogo Modal `Import HSC7 Mass Balance` (`Fig. 28`)
- Ventana con pestañas `Import Options` | `HSC7 Data`:
  - `General`: radios `Dimension` (`1D - Unsized Data`, `1.5D - Size Fraction Data`, `2D - Size Fraction With Assays`) y `Direction` (`Vertical Data`, `Horizontal Data`).
  - `Analyses`: lista con checks `Select Analyses Sheets` (`Flowsheet`, `Streams`, `Units`, `MineralMatrix`, etc.).
  - `Flowsheet`: radios `Select Flowsheet` (`[Nothing]`, `Flowsheet`, `Streams`, `Units`, `MineralMatrix`, `Balanced`) y `Flowsheet Direction` (`Units Horizontal`, `Streams Horizontal`).
  - Botones inferiores: `Cancel` / `OK`.

---

## 3. Convenciones Semánticas de Color y Tipografía

| Señal Visual | Significado Exacto | Referencia |
|---|---|---|
| Texto **azul** en tabla de datos / SD | Valor estimado (`Estimate Missing Values`) o calculado automáticamente (`RSD%`/`SD` por fórmula, `Balanced`, `±CI`, residuo `Balance`/`Tolerance Error`) | `Figs. 9, 13–15, 22–23` |
| Texto **gris tenue** | Variable excluida del balance activo (ej. ensayos `Cu..Zn` al activar `Balancing of Minerals`) | `Figs. 12, 14` |
| Celda verde claro + tooltip fórmula | Flujo ligado por **`Stream by Equation`** (`"Balanced value will be by equation value. Equation=..."`) | `Fig. 12` |
| Fondo amarillo / salmón-rojo en `Diff` | Mapa de calor proporcional a discrepancia entre medido y balanceado (`RelDiff %` medio → alto) | `Fig. 15` |
| Barra azul horizontal embebida | Indicador visual **`Show Data Bars`** de `Recovery-%` respecto al `100%` de alimentación | `Fig. 15` |
| Columna `Mass g Bal.` en **negrita cursiva** / `Rec %` sombreado azul tenue | Distinción rápida en `Stream Summary` entre caudal balanceado y recuperación calculada | `Fig. 20` |

---

## 4. Integración Limpia en `soryntel-desktop` (Plano Objetivo)

1. **Desacoplar el Lanzador ([main.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/main.rs#L1032)):**
   - Cambiar `TileSpec { code: "Mas", label: "Mass\nBalances", action: Act::Launch("mass_balance"), ... }` agregando `WindowMode::MassBalance`, separado de `"balance"` (`Heat & Material Balance`).
2. **Menú `Tools` en `HSC Sim` ([sim_ui/mod.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/sim_ui/mod.rs#L270-L320)):**
   - Añadir opción **`Mass Balancing...`** que inicializa el nuevo estado UI `mass_balance_ui::MassBalanceState` a partir de la topología `Project` (`units`, `streams`) y escribe de vuelta al pulsar `Save to Flowsheet` o `Stream Tables` (actualizando `Stream.mass_balance` en [sim/model.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/sim/model.rs#L766)).
3. **Gráficos nativos (`egui_plot`):**
   - Implementar `Balance Convergence` (escala semilog Y con línea de tolerancia), `Parity Chart` (scatter log-log + diagonal `y = x`), tortas de `Data Status` y gráficos de barras / barras apiladas 100% de `Review SD` y `Reporting -> Charts` dentro del crate `egui_plot = "0.29"` ya presente en `soryntel-desktop/Cargo.toml`.
