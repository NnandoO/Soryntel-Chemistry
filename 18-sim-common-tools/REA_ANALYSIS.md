# Análisis REA — Módulo 18: Sim Common Tools

## 1. Inventario de Fuentes Verificadas

| Tipo | Recurso | Alcance y Detalles Observados |
|---|---|---|
| Manual oficial HSC 10 | [40-sim-common-tools.md](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/40-sim-common-tools.md) | Capítulo `40` completo (`1 132` líneas, `40 KB`, `37` páginas, fecha `September 6, 2023`, secciones `40.1–40.5`, Figs. `1–72`). Autores: Petri Kobylin, Lauri Mäenpää, Matti Hietala, Jussi-Pekka Kentala, Fedor Vasilyev, Ran Liu. |
| Capturas oficiales HSC 10 | [imagenes/](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/imagenes) | `57` archivos PNG (`Imagen pegada.png` y `(2)..(57)`) que documentan menús, barras de herramientas, diálogos modales, ejemplos numéricos (`40.1_model.Sim`, `Hydro_example.Sim`, flotación e hidrociclón) y `About HSC Sim` (`10.3.1.0`). |
| Manual heredado HSC 6 | [40 Sim Flowsheet.txt](file:///home/nando/Descargas/HSC6/Help/40%20Sim%20Flowsheet.txt) | Texto de ayuda heredado de HSC 6 (`100 927 B`, Antti Roine, doc. `06120-ORC-T`): arquitectura antigua `*.fls` + un archivo `*.xls` externo por cada unidad del diagrama y visualización Sankey clásica. |
| Prototipo JSON externo | [proceso_hsc.simjson](file:///home/nando/HSC6-Rust/proceso_hsc.simjson) | Prototipo preliminar de esquema JSON fuera del workspace con arreglos raíz `units`, `streams`, `tables`, `title_tables` y `stream_tables`. |
| Código base Soryntel | [soryntel/crates](file:///home/nando/Documentos/HSC-REA/soryntel/crates) | Inspección de `chem_addin_core`, `soryntel-application`, `soryntel-core`, `soryntel-db`, `soryntel-desktop` (`launcher_ui.rs`, `main.rs`, `excel_grid.rs`), `soryntel-solver` y `soryntel-thermo` (`lib.rs`, `spreadsheet_export.rs`). |

### Fronteras con otros capítulos de HSC Sim

El capítulo `40` concentra las herramientas **transversales** de `HSC Sim`. No especifica en detalle el cálculo avanzado de impacto ambiental ([49-sim-lca](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca)) ni la formulación física detallada de reactores/tanques transitorios de [55-sim-dynamic-simulations-2025](file:///home/nando/Documentos/Docs%20HSC/55-sim-dynamic-simulations-2025), aunque sí expone sus puntos de entrada en el menú principal (`Save as LCA JSON`, `Tools -> Advanced -> Life Cycle Assessment (LCA)`, `Calculate -> Dynamic`, `Dynamic Scenario Editor`).

---

## 2. Descomposición Arquitectónica del Subsistema `HSC Sim`

A partir de las secciones `40.1–40.5` y la estructura de pantallas observada en las capturas, `Sim Common Tools` se compone de **seis subsistemas acoplados** alrededor de un modelo de proyecto central (`*.Sim`):

```text
+-------------------------------------------------------------------+
|                    Shell Desktop HSC Sim (10.3.1.0)               |
| Menús (File..Help) + 15 Top Toolbars + 12 Left Toolbars + Status  |
+---------------------------+---------------------------------------+
|  Subsistema 1: Escena y   |  Subsistema 2: Docking & Propiedades  |
|  Grafo Vectorial          |  - Properties (Process/KPI/Page/      |
|  - Page1..N, Layers 0..5  |    Arrowheads / Drawings)             |
|  - Units (U/R/D/τ)        |  - Icons, Items (Process Tree), Names |
|  - Streams (Source->Dest) |  - Visibility (Layers/Groups/Pages)   |
|  - Shapes, Tables, Labels |  - Inspector (Stream Viewer)          |
+-------------+-------------+-------------------+-------------------+
              |                                 |
              v                                 v
+---------------------------+   +-----------------------------------+
| Subsistema 3: Modelos de  |   | Subsistema 4: Motor de Simulación |
| Unidad y Hojas Internas   |   | Convergencia y Control            |
| - Select Unit Models      |   | - Calculation Order (grafo + rec) |
|   (Species/Particles/     |   | - Static / Partial / Dynamic      |
|    Others/Imported)       |   | - Convergence Monitor (MB / EB %) |
| - DLL (HU-xxx) vs Excel   |   | - Control Solver (Tangential/PID) |
| - Unit Editor & Controls  |   | - Cell References & Log booleano  |
+-------------+-------------+---+---------------+-------------------+
              |                                 |
              v                                 v
+---------------------------+   +-----------------------------------+
| Subsistema 5: Escenarios, |   | Subsistema 6: Persistencia,       |
| Tablas e Informes         |   | Respaldo y Visualización Sankey   |
| - Static Scenario Editor  |   | - *.Sim / Sim 7 *.fls Importer    |
| - Monte Carlo (PDF / CDF) |   | - Auto Backup (15/30/60 min)      |
| - Stream Table Editor     |   | - ModelBase / Rex / Zip / LCA     |
| - Reports (4 hojas)       |   | - Export PNG/PDF/VDX/SVG/DXF      |
| - Title Table / Flow Tbl  |   | - Sankey Colors & Line Widths     |
+---------------------------+   +-----------------------------------+
```

### 2.1. Escena Vectorial y Modelo Topológico

1. **Entidades del grafo de proceso:**
   - **Unidades (`Units`):** clasificadas por ícono/modo funcional **`U`** (*Generic*), **`R`** (*Reactions*), **`D`** (*Distributions/Particles*) y **`τ`** (*Dynamic*). Cada unidad posee identificador único (`NameID`), alias numérico/textual (`Number/Alias`), coordenadas geométricas, asignación de página/capa y vínculo a un modelo interno (hoja de cálculo o `DLL`).
   - **Corrientes (`Streams`):** aristas dirigidas entre un origen (`Source`: unidad o frontera externa) y un destino (`Destination`: unidad o frontera externa), representadas como polilíneas ortogonales o enrutadas automáticamente (`Auto Route`, `Orthogonal`). Soportan reasignación directa mediante arrastre del nodo azul o mediante el diálogo `Streams Source and Destination`.
2. **Entidades documentales y gráficas:** tablas generales (`Table`), carátulas (`Title Table`), tablas vinculadas a corrientes (`Stream Tables` y `Flow Table`), etiquetas de nombre/valor (`Name Label`, `Value Label`, `Header Label`), cajas multilínea, polilíneas, cajas de imagen y figuras geométricas (`Drawing Shapes` y `Advanced Drawing Shapes`).
3. **Organización jerárquica:**
   - **Páginas (`Pages`):** soporte multipágina con pestañas inferiores (`Page1`, `Page2`, `Page3`, `+`), donde un proceso completo puede repartirse en subdiagramas conectados lógicamente.
   - **Capas (`Layers`):** seis capas predeterminadas (`Layer 0 (Default)`, `Layer 1 (Streams)`, `Layer 2 (Tables)`, `Layer 3 (Labels)`, `Layer 4`, `Layer 5`) con conmutación individual de visibilidad.
   - **Grupos (`Groups`) y Bloqueos:** agrupación lógica con toggle de visibilidad y dos niveles de protección (`Lock Drawing` con borde verde para evitar ediciones accidentales sin impedir cálculos, y `Lock Entire Process` con borde rojo para congelar todo el proyecto).

### 2.2. Motor de Modelos de Unidad y Hojas de Cálculo

- **`Select Unit Models`:** asigna plantillas desde la biblioteca central (`Species`, `Particles`, `Others`) o externas (`Imported` vía `Import Unit...`). Distingue modelos compilados de tipo **`DLL`** (con códigos tipificados como `HU-510-10` *Generic Cooling Tower*, `HU-810-10` *Generic Filter*, `HU-820-10` *Generic Thickener*, `HU-520-11` *Heat Exchanger*, `HU-100-1...` *MixerTest*) y plantillas tipo **`Excel`** (*Empty Reactions (Hydro) unit*, *Empty Distribution (Pyro) unit*, hojas personalizadas).
- **`Unit Editor`:** visor/editor tipo hoja de cálculo por cada unidad, con acceso a referencias de celda para `Controls` (`Y TARGET NAME`, `X VARIABLE NAME`, `CONTROL METHOD`, `PID PARAMETERS`), `Cell References` globales y `Scenario Editor`.

### 2.3. Subsistemas de Cálculo, Convergencia y Diagnóstico

- **Cálculo estático (`Static`):** soporta evaluación completa (`Run Static Calculations`), parcial (`Calculate Selected Units`), iterativa hasta balance (`Run Until Convergence`) y convergencia anidada de lazos (`Run Until Controls Converge`), gobernados por `Criteria` (`Mass`, `Enthalpy`, `Both`), `Tolerance %` y `Max Rounds`.
- **Cálculo dinámico (`Dynamic`):** control de ejecución en tiempo simulado con `Start Time`, `End Time`, `Time Step` (`1 s`), `Speed: Full Speed`, `Run Until Paused`, más utilidades de estado (`Empty All Tanks`, `Empty Tanks Before Calculation Starts`, `Clear Internal Streams`, `Reset Partial Simulation`).
- **Diagnósticos integrados:**
  - `Convergence Monitor` (curvas e histórico por ronda de errores `MB` y `EB`).
  - `Calculation Order` (colores `1/2/3` para visualizar secuencia y dependencias en pantalla).
  - `Cell References` (tabla buscable/ordenable con navegación directa `Navigate To Cell` y filtro `Search Controls`).
  - `Controls` y `Control Monitor` (activación selectiva por lazo, resaltado activo/inactivo y trazado X–Y hasta `50` pasos de barrido frente al `Set Point`).
  - `Log` (niveles `All`, `Warnings & Errors`, `Errors` con filtrado lógico booleano `AND`, `OR`, `NOT`).

---

## 3. Reglas de Importación desde Versiones Anteriores (`40.4` HSC Sim 7 Importer)

El manual documenta explícitamente que los modelos de **HSC Sim 6 no son compatibles**, mientras que los archivos `*.fls` de **HSC Sim 7** pasan por un importador con controles más estrictos y transformaciones automáticas:

| Aspecto en HSC Sim 7 (`*.fls`) | Comportamiento en HSC Sim 10 (`40.4`) |
|---|---|
| Chequeos de consistencia de hoja | Reglas más estrictas: puede reportar errores antes silenciosos en Sim 7. |
| Listas de variables *hydro* con especies ausentes de DB | Aparecen en el log como errores nuevos tras importar. |
| Asistentes Excel de **extracción por solventes** (Sim 7) | **No soportados**; el importador rechaza/falla con estas unidades. |
| Asistentes Excel de **minpro** (Sim 7) | Ejecutables en **solo lectura**; no editables con el asistente nuevo (se recomienda migrar a `DLL`). |
| `Stream Tables` creadas en Sim 7 | Conservadas visualmente en el diagrama, pero **no editables** ni visibles en `Stream Table Editor`. |
| Corrientes de entrada/salida ocultas | Se hacen visibles automáticamente; usar `Visibility` para ocultarlas nuevamente si se requiere. |
| Curvas Bézier | **No soportadas**; se omiten/degradan al importar. |
| Unidades encriptadas (Sim 7) | **No se cargan** durante la importación. |
| Referencias externas entre libros/unidades | Se reescriben anteponiendo el prefijo **`REF`** al nombre original de la referencia. |
| Operaciones de división en celdas | Se envuelven en la función protegida **`safediv`** para mitigar `#DIV/0!`. |
| Caracteres `/` ilegales en nombres de pestañas/hojas | Se reemplazan o sanean automáticamente para compatibilidad interna. |

---

## 4. Auditoría de Brechas en Soryntel

La inspección directa de [crates/](file:///home/nando/Documentos/HSC-REA/soryntel/crates) confirma que **Soryntel aún no implementa el módulo de simulación de diagrama de flujo (`Sim`)**, aunque ya dispone de varias piezas reutilizables:

| Capacidad requerida por `Sim Common Tools` | Estado verificado en `soryntel` | Evidencia / Ubicación exacta |
|---|---|---|
| Entrada desde lanzador HSC 10 | **Parcial (placeholder)** | [main.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/main.rs#L977) define el mosaico `code: "Sim"`, pero lanza temporalmente `"excel_addins"` (`Excel Add-Ins & StreamEQ`) en lugar de una ventana `WindowMode::Sim`. |
| Ventana dedicada `Sim` y lienzo vectorial | **No implementado** | [main.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/main.rs#L32-L51) enumera 18 variantes de `WindowMode` desde `Launcher` hasta `Species`, sin variante `Sim` ni `sim_ui.rs`. |
| Grafo topológico, reciclos y motor modular secuencial | **No implementado** | [soryntel-solver/src/lib.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-solver/src/lib.rs) únicamente contiene el parser de fórmulas/reacciones y el balanceador estequiométrico racional (`parse_input`, `verify_balance`, `balance_reaction`). |
| Dominio de simulación multipunto (`Sim`, `MB/EB`, `Scenario`, `Control`) | **No implementado** | [soryntel-thermo/src/lib.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/lib.rs#L4-L25) expone 22 submódulos (`aqua`, `balance`, `equilibrium`, `excel_addins`, `species`, etc.), sin módulo `sim` o `flowsheet`. |
| Grilla estilo hoja de cálculo para `Unit Editor` / `Reports` / `Scenario Editor` | **Reutilizable parcialmente** | [excel_grid.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/excel_grid.rs#L1-L40) provee helpers compartidos de celdas estilo Excel (`EXCEL_HEADER_BG`, `col_letter`, `header_cell`), y [spreadsheet_export.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/spreadsheet_export.rs#L1-L27) genera `.xlsx`/`.ods` con soporte de fórmulas y caché. |
| Evaluador de fórmulas de celda y balances multifase por unidad | **Reutilizable como base** | [soryntel-application/src/lib.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-application/src/lib.rs#L32-L37) expone `eval_formula_string`, `eval_streameq`, `eval_addin_function` y `lookup_mineral` que pueden alimentar hojas internas de unidades en modo Excel/clean-room. |
| Persistencia de proyecto de diagrama | **Prototipo externo aislado** | [proceso_hsc.simjson](file:///home/nando/HSC6-Rust/proceso_hsc.simjson) define un esqueleto inicial (`units`, `streams`, `tables`, `title_tables`, `stream_tables`), no integrado en `soryntel-core` ni `soryntel-db`. |
