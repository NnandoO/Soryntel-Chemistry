# Especificación de Interfaz de Usuario (UI) — Módulo 18: Sim Common Tools

La interfaz del entorno compartido **HSC Sim** (`HSC Sim Flowsheet Module`, ver. `10.3.1.0`) se reconstruye fielmente a partir de las `72` figuras de [40-sim-common-tools.md](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/40-sim-common-tools.md) y sus capturas oficiales en [imagenes/](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/imagenes).

---

## 1. Arquitectura Visual de la Ventana Principal (`HSC Sim - <archivo>.Sim`)

```text
+---------------------------------------------------------------------------------------------------+
| HSC Sim - <ruta>\<modelo>.Sim                                                         [ - ][ 口 ][X] |
+---------------------------------------------------------------------------------------------------+
| File | View | Select | Flowsheet | Tools | Calculate | Diagnostics | Help                         |
+---------------------------------------------------------------------------------------------------+
| [Top Toolbars (15 grupos): Save/Open/ModelBase | Connectivity | Select visualization | Dyn/Text]  |
+----+----------------------------------------------------------------------+-----------------------+
| L  |   -40  -30  -20  -10   0   10   20   30   ...   170 (Regla mm H)     | Properties            |
| e  +--+-------------------------------------------------------------------+ [Process|KPI|Page|Ar] |
| f  |0 | .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  | o Drawings (ctx.)     |
| t  |10|                                                                   | Project / Process /   |
|    |20|      [Unit 1] ----Stream 2----> [Unit 2] ----Stream 3---->        | General / Location    |
| T  |  |         |                          |                              +-----------------------+
| o  |  |      Stream 4                   Stream 5                          | Icons / Dock Inferior |
| o  |  |         v                          v                              | [Icons|Items|Names|   |
| l  |  |   (Lienzo multipágina con grilla Points 2x2 y marco de bloqueo)   |  Visibility|Controls| |
| s  +--+-------------------------------------------------------------------+  Cell Ref.|Log]       |
|12  | |<|<|>|>|+| Page1 | Page2 | Page3 |           (Barra de pestañas)    | Buscador + Zoom íconos|
+----+----------------------------------------------------------------------+-----------------------+
| [Zoom 100% --o--] | o Auto Backup [OFF/ON] | [] Auto Route | [x] Orthogonal | [x] Persist | Grid  |
+---------------------------------------------------------------------------------------------------+
```

### 1.1. Barra de Menús Principal (`8` menús en HSC 10)

> *Nota histórica (`Fig. 16`): capturas anteriores de HSC mostraban `Drawing Tools` y `Window` en la barra superior; en la interfaz consolidada de HSC 10 (`Figs. 1–41`) todos los comandos se agrupan en **8 menús**:*

1. **`File` (`Fig. 14`):**
   - *Open:* `New Process... (Ctrl+N)`, `Open Process... (Ctrl+O)`, `Open Recent Process...` (`10` recientes), `Download from ModelBase...`, `Open Process from Rex...`, `Import Another Flowsheet Into This...`.
   - *Save:* `Save Process (Ctrl+S)`, `Save Process As...`, `Save as Zip...`, `Save as LCA JSON...`, `Save Without Models As...`, `Save Without Database As...`, `Upload to ModelBase...`.
   - *Backup:* `Save Backup (Ctrl+B)`, `Auto Backup - ON / OFF` (`15`, `30`, `60` min), `Backup Manager...`.
   - *Flowsheet:* `Export Flowsheet Image...` (`.png`, `.pdf` [`Current Page`/`All Pages`], `.vdx`, `.svg`, `.dxf`), `Copy Flowsheet Image`, `Print Flowsheet (Ctrl+P)`.
   - *General:* `Sim Settings` (abre `Default Settings`), `Exit HSC Sim (Alt+Q)`.
2. **`View` (`Figs. 16, 18, 44`):** `Reset -> Reset All / Reset Docking Bar Positions`, toggles para **`Top Toolbars` (`15`)** y **`Left Toolbars` (`12`)**.
3. **`Select` (`Fig. 19`):** `20` filtros tipificados: `Units`, `Unit Name Labels`, `Streams`, `Stream Name Labels`, `Stream Value Labels`, `Stream Tables`, `Header Label`, `Name Labels`, `Value Labels`, `Text Labels`, `All Labels`, `Multi-line Text Boxes`, `General Tables`, `Value Cells`, `Title Tables`, `All Tables`, `Polylines`, `Picture Boxes`, `Other Drawing Objects`, `All Items (Ctrl+A)`.
4. **`Flowsheet` (`Fig. 20`):**
   - `Flowsheet Tools`: `Inspector (Stream Viewer)`, `Items (Process Tree)`, `Names (Rename Alias)`, `Properties`, `Visibility (Layers & Groups)`.
   - `Flowsheet Items`: `Flowsheet Chart Editor`, `Icons (Units)`, `Stream Table Editor`, `Unit Table Editor`.
   - `Flowsheet Visualization`: `Activate Visualization`, `Measurement Units`, `Visualization Settings`, `Reset Visualization Settings`.
   - `Drawing Tools`: `Align`, `Make Same Size`, `Rotate and Flip`, `Layer and Order`, `Visual Copy to`, `Copy Flowsheet Image`, `Lock Drawing`, `Lock Entire Process`.
5. **`Tools` (`Fig. 22`):**
   - `Information`: `Notes`, `Process Information`, `Reports`.
   - `Simulation`: `Dynamic Scenario Editor`, `Static Scenario Editor`, `Model Species`, `Unit Models`.
   - `Balance and Parameters`: `Flotation Kinetics Fit`, `Global Sieve Series`, `Mass Balance`, `Process Fit (Beta)`, `Residence Time Distribution (RTD)`.
   - `Advanced`: `Life Cycle Assessment (LCA)`, `Life Cycle Assessment (LCA) (Beta)`, `Model Optimization`, `Neural Networks`, `Thermoeconomics Calculator`, `Unit Availabilities`, `Unit Protection`.
6. **`Calculate` (`Fig. 35`):**
   - `Static`: `Run Static Calculations`, `Calculate Selected Units`, `Run Until Convergence`, `Run Until Controls Converge`, `Criteria` (`Mass` / `Enthalpy` / `Both`), `Tolerance %`, `Max Rounds`.
   - `Dynamic`: `Start Dynamic Simulation`, `Simulate Selected Units`, `Stop Dynamic Simulation`, `Speed: Full Speed`, `Start Time`, `Time Step`, `Run Until Paused`, `End Time`.
   - `Functions`: `Reset Partial Simulation`, `Clear Internal Streams`, `Empty All Tanks`, `Empty Tanks Before Calculation Starts`.
7. **`Diagnostics` (`Fig. 38`):**
   - `Diagnostics & Suggestions`, `Log`, `Calculation Order` (`Set color 1/2/3`, `Remove color`, `Select All`), `Cell References`, `Controls`.
   - `Diagnostic Settings`: `Collect Data in Dynamic Scenario Editor`, `Enable Calculation Log`, `Ignore Warnings`, `Show Log After the Calculation`.
8. **`Help` (`Fig. 41`):**
   - `Help`: `Examples`, `Tutorial Videos`, `User Manuals`, `Email Feedback`, `Training Courses`.
   - `HSC Chemistry`: `About HSC Chemistry`, `About HSC Sim`, `HSC Chemistry - Metso Outotec`, `Check for Updates`.
   - `Metso Outotec`: `About Us`, `Products`, `Services`, `Sustainability`.

---

## 2. Barras de Herramientas (`Top Toolbars`, `Left Toolbars` y `Status Bar`)

### 2.1. `Top Toolbars` (`15` grupos conmutables, `Fig. 44–46`)

1. `Saving Options`
2. `ModelBase Options`
3. `Copy Flowsheet Image`
4. `Lock / Unlock`
5. `Advanced Flowsheet Tools`
6. `Visualization`: botón `Stream connectivity visualization` (`Fig. 4`), conmutador de visualización, combo `Select visualization`, unidades, acceso directo a `Stream Table Editor`, encabezados `H` y copia de imagen (`Fig. 46`).
7. `Partial Simulation Switch`
8. `Static Partial Simulation`
9. `Dynamic Partial Simulation`
10. `Static Simulation` (incluyendo acceso a `Calculate`)
11. `Dynamic Simulation` (botón Play azul, spinbox numérico p.ej. `50`, botón de pausa/reinicio, barra verde de progreso `0 min 0 s`)
12. `Advanced Dynamic Simulation`
13. `Text Formatting` (selector de fuente p.ej. `Arial`)
14. `Text Alignment`
15. `Line Formatting` (color de trazo negro/RGB y grosor de línea)

### 2.2. `Left Toolbars` (`12` grupos conmutables, `Figs. 1, 18, 45, 47, 48`)

1. `Basic Tools` (`Select` flecha, `Pan Tool` mano)
2. `Pan Tool`
3. `Units and Streams` (`Fig. 45`): **`U`** (Generic), **`R`** (Reactions), **`D`** (Distributions), **`τ`** (Dynamic) y **`Streams`**
4. `Tables` (`Fig. 48`): **`Table`** y **`Title Table`**
5. `Drawing Items` (`Fig. 47`): `Header`, texto `Aa`, cuadro multilínea, imagen, tabla
6. `Drawing Shapes` (línea, elipse, triángulo, rectángulo, rectángulo redondeado)
7. `Advanced Drawing Shapes` (formas de diagrama de procesos adicionales)
8. `Height and Width`
9. `Rotate and Flip`
10. `Layers`
11. `Align`
12. `Locks`

### 2.3. Barra de Estado Inferior (`Figs. 2, 5, 12`)

Dispone de izquierda a derecha:
- **Control de Zoom:** `100%` con botones `-` / `+` y deslizador continuo.
- **Indicador de Respaldo:** punto de estado `Auto Backup [OFF]` / `[ON]`.
- **Conmutadores de dibujo:** checkboxes `Auto Route`, `✔ Orthogonal`, `✔ Persist Tool`, `✔ Snap to Grid`.
- **Inspector métrico dinámico (`Fig. 5, 12`):** dimensión de selección (`20 x 7mm`, `18 x 0mm`), posición origen (`-19, 17`, `73, 57`) y cursor (`87, 90`, `53, 122`).

---

## 3. Paneles Acoplables Derechos (`Docking Bars`)

### 3.1. Dock Superior: **`Properties`**

Cambia sus pestañas dinámicamente según la selección:
- **Sin selección en lienzo (`Fig. 2`):** `Process` (`Project`: `Project Name`, `Project Path`; `Process`: `Name`, `Technology`, `Sub Technology`, `Plant Type`, `Products`, `More Information`), `KPI`, `Page`, `Arrowheads`.
- **Con corriente/unidad/objeto seleccionado (`Figs. 5, 12`):**
  - `Process`: bloque `General` (`Object Type`, **`NameID`** editable, `Number/Alias`, `Source`, `Destination`, `Drawing Object Type`) y bloque `Location` (`Page`, `Layer`).
  - `Drawings`: bloque `General` (`Drawing Object Type`, `Page`, `Layer`), `Drawing - Line` (`Line Color`, `Line Style`, `Line Width` p.ej. `0,25`), `Drawing - Arrows` (`Auto Arrow End Update`).

### 3.2. Dock Inferior (`7` pestañas integradas + acoplamiento con `Inspector`)

| Pestaña | Contenido y Controles Observados |
|---|---|
| **`Icons`** | Filtro desplegable (`Units by Type`), caja de búsqueda (`Search for units`), cuadrícula paginada/desplazable de equipos vectoriales y barra deslizadora inferior para tamaño de previsualización. |
| **`Items`** | Árbol jerárquico de procesos (*Process Tree*) con páginas, unidades y corrientes. |
| **`Names`** | Tabla rápida para renombrar alias (`Number/Alias` / `NameID`) en lote. |
| **`Visibility`** | Subpestañas **`Layers`** (`Layer 0 (Default)`, `Layer 1 (Streams)`, `Layer 2 (Tables)`, `Layer 3 (Labels)`, `Layer 4`, `Layer 5`), **`Groups`** y **`Pages`** con casillas de visibilidad. |
| **`Controls`** | Barra `Control ON/OFF`, visibilidad de filas/columnas, `Navigate To Control`, y grilla `Control Name | Location | Active | Set Point | Measured | Control Variable | Type` con resaltado cromático rosa/verde según estado. |
| **`Cell Ref.`** | Botones `Sort`, `Navigate To Cell`, `Search Controls`, y tabla `Unit Name | Cell Reference Name | Value`. |
| **`Log`** | Filtros de nivel (`All | Warnings & Errors | Errors`), buscador booleano (`AND, OR, NOT`) y bitácora cronológica. |
| **`Inspector`** | Vista acoplable (*Stream Viewer*) que detalla flujos, especies y fracciones al seleccionar una corriente. |

---

## 4. Ventanas Modales y Editores Especializados

1. **`Streams Source and Destination` (`Fig. 12`):** Botones `Apply` y `Discard`; combo selector de corriente (`Stream 1`); paneles `Source Unit` y `Destination Unit` con etiquetas `Old` (y botón revertir `Old`) y selectores `New`.
2. **`Default Settings` (`Fig. 15`, vía `Sim Settings`):** Pestañas `Page`, `Unit`, `Stream`, `ArrowHeads`, `Header Label` con visor de propiedades por categoría (`Page Background Color: 255; 255; 255`, `Show Grid: True`, `Grid Style: Points`, `Grid Color: 176; 196; 222`, `Grid Width: 2`, `Grid Height: 2`).
3. **`Stream Table Editor` (`Fig. 9`):**
   - Ribbon `Main` (`Apply`: `Apply and Close`, `Discard without Saving`; `Stream Table Sets`: `Add/Delete/Clone/Rename/Clear Table Set`; `View Mode and Dimension`: `Table view Mode` [`Name, value and unit`], `Columns`, `Rows`; `Display Names`: `Edit Display Names`) y pestaña `Formatting`.
   - 3 áreas principales: `Variable List` izquierda (árbol codificado por color: morado flujos/densidades, negro recuperaciones, azul elementos `wt-%`/`Rec-%`, rojo minerales), grilla central `Selected Variables` (`Table Set 1`), panel derecho superior `Variable Properties` (`Stream Table Variable` + `Header`) e inferior `Configuration to Streams`.
4. **`Title Table` (`Fig. 8`):** Plantilla de `4` filas con celdas `Title`, `Organization/Author(s)`, `HSC | Version` (izquierda) y bloque rojo `Logo` combinando las filas 3–4 (derecha), más `Date`.
5. **`Select Unit Models` (`Figs. 27–28`):**
   - Ribbon: `Apply` (`Apply and Close`, `Discard and Close`), `Clear` (`Clear Selected Models`, `Clear All Models`), `Models` (`Update All Old Models`, `Manage Unit DLL Library...`, `Import Unit...`), `Zoom Font` (deslizador `-` / `+`), `Help`.
   - 3 paneles: `Process Unit Operations` (`Unit | Selected Model`), `Model Library` (`Species | Particles | Others | Imported`, columnas `Model | Technology | Type`), `Model Info` (`General` + `Description`).
6. **`Unit Editor`:** Libro estilo Excel por unidad con barra superior dedicada para bloques de control (`Y TARGET NAME`, `X VARIABLE NAME`, `CONTROL METHOD`, `PID PARAMETERS`).
7. **`Scenario Editor` (`Figs. 29–34`):**
   - Ribbon superior: `Run` (`Run Selected Scenario`, `Run All Scenarios`, `Stop Calculations`, progreso `%`, `Number of Runs`, `Iteration Rounds`), `Monte Carlo` (`Configuration`, `Distribution Charts`), `Scenarios` (`Add/Clone/Delete/Rename Scenario`), `Data Columns` (`Add Cell Reference(s)`, `Create New SET/GET Column`, `Delete/Move Left/Right`, `Freeze First Column`), `Chart Settings` (`Create a New Chart Tab`, `Create a New Chart`, `Edit Chart Data`).
   - Grilla principal con pestañas `Scenario 1...`: filas cabecera `Unit Name`, `Variable Name`, `Measurement Unit`, `SET / GET`, `Cell Reference` (menú contextual con **`Paste cell reference Ctrl+Shift+V`**, `Formula Navigator`, `Export to Excel`, `Column decimals`) y filas de ejecución `Run 1..N` con columna `Rounds`.
   - Dock derecho `Charts` con gráfica integrada `HSC Chart`.
   - Subdiálogos `Configuration` (parámetros `Monte Carlo`, `Dist. Type`, `Min`, `Max`, `Mean`, `SD`, `b`, `c`, `Round to Nearest`) y `Distribution Charts` (curvas **PDF** y **CDF**).
8. **`Reports` (`Figs. 24–26`):** Visor de libro multihoja con `4` pestañas fijas: `Unit Balance - General`, `Stream Balance - Hydro`, `Streams - Properties`, `Flowsheet Tables`.
9. **`Convergence Monitor` & `Control Monitor` (`Figs. 37, 70`):**
   - `Convergence Monitor`: gráfica y tabla por ronda de los residuales `MB %` y `EB %` frente a la banda `Tolerance %`.
   - `Control Monitor`: gráfica X–Y de seguimiento por paso (`0..50` muestras) comparando respuesta medida `Y` vs `Set Point`.
10. **`Visualization Color Picker` (`Fig. 68`):** Selector de modo de color (`Flowsheet stream colors`, `Positive and negative colors`, `Range colors`), histograma/escala, controles `Min/Max`, `Use absolute values`, **`Multiplier: 10^1`**, `Scale Pages Separately`, `Visualize Stream Line Width` y `Stream Width Max`.
11. **`Process Information`, `Notes`, `About HSC Sim` (`Fig. 43`) y `Check for Updates` (`Fig. 42`):** Formularios auxiliares de documentación del proceso (`Info`/`KPI`, `Info`/`Links`/`Trainings`), créditos de versión (`10.3.1.0`) y verificación de licencia/actualizaciones.
