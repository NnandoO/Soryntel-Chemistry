# Especificación Funcional — Módulo 18: Sim Common Tools

Este documento especifica los contratos funcionales (`CU-01` a `CU-12`) de **Sim Common Tools**, reconstruidos íntegramente a partir del capítulo `40` ([40-sim-common-tools.md](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/40-sim-common-tools.md)) y sus capturas de referencia (`Figs. 1–72`).

---

## CU-01 — Dibujo, Conexión y Reconfiguración de Unidades y Corrientes

- **Entrada UI:** Barra izquierda **`Units and Streams`** (`Select`, `Pan`, **`U`** Generic, **`R`** Reactions, **`D`** Distributions, **`τ`** Dynamic, `Streams`), o arrastrando íconos de equipos desde el panel acoplable **`Icons`**.
- **Precondición:** Página activa desbloqueada (sin `Lock Drawing` ni `Lock Entire Process`).
- **Flujo normal:**
  1. Al activar un tipo de unidad (p.ej. `R`) el puntero muestra el distintivo correspondiente (`Fig. 2`); hacer clic o arrastrar en el lienzo crea `Unit 1`, `Unit 2`, etc.
  2. Si **`Persist Tool`** está marcado en la barra de estado, la herramienta permanece activa para dibujar múltiples objetos consecutivos hasta presionar `Esc` o volver a `Select`.
  3. Con la herramienta `Streams` se trazan corrientes entre unidad origen y unidad destino, respetando las opciones globales **`Ortho` (`Orthogonal`)**, **`Auto Route`** y **`Snap to Grid`**.
  4. Las corrientes externas de entrada terminan en círculo hueco en origen; las de salida en flecha hueca en destino; las internas conectan directamente unidad–unidad (`Fig. 3`).
  5. Hacer clic en **`Stream connectivity visualization`** colorea en **azul** las corrientes de entrada externa, **negro** las internas y **rojo** las de salida (`Fig. 4`).
- **Renombrado (`Fig. 5`):** Seleccionando una unidad o corriente, editar **`NameID`** (o `Number/Alias`) en `Properties -> Process -> General`, o usar el panel **`Names`** (`Rename Alias`).
- **Reconexión e inserción intermedia (`Figs. 11–13`):** Al arrastrar el nodo cuadrado azul extremo de una corriente existente sobre una nueva unidad (p.ej. insertar `Unit 3` en `Stream 1` entre `Unit 1` y `Unit 2`), el sistema abre **`Streams Source and Destination`** mostrando origen/destino `Old` y `New` (con botón de deshacer `Old`). El usuario **debe confirmar con `Apply`** (o cancelar con `Discard`) antes de dibujar `Stream 2`.

---

## CU-02 — Creación y Edición de Tablas, Carátulas y Tablas de Corrientes

- **Entrada UI:** Barra izquierda **`Tables`** (`Table`, `Title Table`), menú `Flowsheet -> Flowsheet Items -> Stream Table Editor` / `Unit Table Editor`.
- **Subfunciones:**
  1. **`Table` y `Table Editor` (`Fig. 6`):** Inserta grillas parametrizables directamente en el lienzo, con celdas editables y enlaces por referencia a celdas del modelo.
  2. **`Title Table` (`Fig. 8`):** Inserta una carátula estructurada de `4` filas con los campos normalizados: `Title`, `Organization/Author(s)`, `HSC`, `Version`, `Date` y cuadro **`Logo`**.
  3. **`Stream Table Editor` (`Fig. 9`):**
     - Permite crear, clonar, renombrar y limpiar conjuntos de variables (**`Stream Table Sets`**, p.ej. `Table Set 1`).
     - Selección doble clic desde **`Variable List`** (ej. bajo `Minerals Processing (particles)`: `Total Solids Flow (t/h)`, `Total Gas Flow (t/h)`, `Total Liquid Flow (t/h)`, `Pulp Mass Flow (t/h)`, `Pulp Volumetric Flow (m3/h)`, `Solids SG (g/cm3)`, `Liquid SG (g/cm3)`, `Pulp SG (g/cm3)`, `% Solids (wt-%)`, `Solids Recovery (Rec-%)`, `Liquid Recovery (Rec-%)`, ensayos elementales `wt-%`/`Rec-%` y mineralógicos `wt-%`/`Rec-%`).
     - Configuración de diseño (`Table view Mode`: p.ej. `Name, value and unit`, `Columns`, `Rows`), nombres de visualización (`Edit Display Names`, `Header Visibility`, `Use Text From`, `Header Text`) y asignación selectiva por corriente en **`Configuration to Streams`**.
  4. **`Flow Table` (`Fig. 10`):** Tabla resumen consolidada por columnas de corrientes en el diagrama.

---

## CU-03 — Persistencia de Proyectos, Respaldos Automáticos, ModelBase y Exportación

- **Entrada UI:** Menú **`File`** (`Fig. 14`) y barras superiores `Saving Options` / `ModelBase Options` / `Copy Flowsheet Image`.
- **Contratos funcionales:**
  - **Apertura e importación:**
    - `New Process... (Ctrl+N)`, `Open Process... (Ctrl+O)` (`*.Sim` nativo o `*.fls` de HSC Sim 7; los modelos de HSC Sim 6 están excluidos).
    - `Open Recent Process...`: historial de los últimos `10` proyectos abiertos.
    - `Download from ModelBase...` / `Upload to ModelBase...` y `Open Process from Rex...` (autenticación requerida).
    - `Import Another Flowsheet Into This...`: fusiona unidades, corrientes y páginas de otro diagrama en el proceso actual.
  - **Guardado especializado:**
    - `Save Process (Ctrl+S)`, `Save Process As...` (`*.Sim`).
    - `Save as Zip...` (paquete portable comprimido).
    - `Save as LCA JSON...` (exportación estructurada de flujos/entradas/salidas para análisis de ciclo de vida).
    - `Save Without Models As...` y `Save Without Database As...` (compartición ligera o protección de propiedad intelectual).
  - **Respaldo no destructivo:**
    - `Save Backup (Ctrl+B)` y `Auto Backup - ON / OFF` con intervalos de **`15`**, **`30`** o **`60` minutos** (cada copia se guarda en archivo independiente sin sobrescribir copias previas ni el archivo principal); gestión histórica en **`Backup Manager...`**.
  - **Exportación e impresión:**
    - `Export Flowsheet Image...` en formatos raster y vectoriales/CAD: **`.png`**, **`.pdf`** (con alcance `Current Page` o `All Pages`), **`.vdx`** (Microsoft Visio), **`.svg`** y **`.dxf`** (AutoCAD); `Copy Flowsheet Image` al portapapeles; `Print Flowsheet (Ctrl+P)`.

---

## CU-04 — Selección Filtrada, Multipágina, Capas, Grupos y Bloqueos

- **Entrada UI:** Menú `Select` (`Fig. 19`), `Flowsheet -> Flowsheet Tools -> Visibility / Items / Names` (`Figs. 20–21`) y `Drawing Tools`.
- **Reglas funcionales:**
  1. **Selección por categoría (`20` filtros en `Select`):** Permite seleccionar exclusivamente `Units`, `Unit Name Labels`, `Streams`, `Stream Name Labels`, `Stream Value Labels`, `Stream Tables`, `Header Label`, `Name Labels`, `Value Labels`, `Text Labels`, `All Labels`, `Multi-line Text Boxes`, `General Tables`, `Value Cells`, `Title Tables`, `All Tables`, `Polylines`, `Picture Boxes`, `Other Drawing Objects` o `All Items (Ctrl+A)`.
  2. **Visibilidad (`Visibility`):** Administra tres pestañas: **`Layers`** (`Layer 0 (Default)`, `Layer 1 (Streams)`, `Layer 2 (Tables)`, `Layer 3 (Labels)`, `Layer 4`, `Layer 5`), **`Groups`** y **`Pages`**, permitiendo mostrar/ocultar objetos sin eliminarlos de la topología.
  3. **Árbol de elementos (`Items`) y Alias (`Names`):** Navegación rápida y renombrado masivo de identificadores visibles (`Number/Alias`) sin romper vínculos internos.
  4. **Bloqueos (`Lock / Unlock`):**
     - `Lock Drawing`: rodea el lienzo con marco **verde** (`Fig. 58`) y bloquea movimientos/borrados gráficos manteniendo operativos modelos y simulaciones.
     - `Lock Entire Process`: rodea el lienzo con marco **rojo** e impide cualquier modificación estructural o paramétrica.

---

## CU-05 — Información del Proceso, Notas e Informes (`Reports`)

- **Entrada UI:** Menú `Tools -> Information` (`Figs. 22–26`): `Notes`, `Process Information`, `Reports`.
- **Reglas funcionales:**
  1. **`Notes`:** Bloc estructurado del proceso principal (`Main Process`) con secciones **`Info`**, **`Links`** y **`Trainings`**.
  2. **`Process Information`:** Libro con hojas **`Info`** y **`KPI`** sincronizado con las pestañas homónimas del panel `Properties -> Process / KPI`.
  3. **`Reports`:** Genera dinámicamente un libro consolidado de cálculo y balances con `4` hojas estándar:
     - `Unit Balance - General`
     - `Stream Balance - Hydro`
     - `Streams - Properties`
     - `Flowsheet Tables`

---

## CU-06 — Selección e Importación de Modelos de Unidad (`Select Unit Models` & `Unit Editor`)

- **Entrada UI:** Menú `Tools -> Simulation -> Unit Models` (`Figs. 27–28`), doble clic en unidad (`Unit Editor`) o `Tools -> Simulation -> Model Species`.
- **Contratos funcionales:**
  1. **`Select Unit Models`:**
     - Muestra a la izquierda todas las unidades del proyecto (**`Process Unit Operations`**: `Unit` + `Selected Model`, con casilla de selección).
     - En el centro ofrece **`Model Library`** con `4` pestañas (**`Species`**, **`Particles`**, **`Others`**, **`Imported`**), identificando tecnología y tipo (`Excel` o `DLL`) y código de modelo cuando aplica (`HU-510-10`, `HU-810-10`, `HU-820-10`, `HU-520-11`, `HU-100-1...`).
     - A la derecha muestra **`Model Info`** (`Model Name`, `Mode`, `Type Code`, `Version`, `Calculation`, `Description`).
     - Acciones de barra superior: `Apply and Close`, `Discard and Close`, `Clear Selected Models`, `Clear All Models`, `Update All Old Models`, `Manage Unit DLL Library...` e **`Import Unit...`** (cuyos modelos aparecen bajo la pestaña **`Imported`**).
  2. **`Unit Editor`:** Abre la hoja interna (o interfaz paramétrica `DLL`) de la unidad seleccionada, sincronizada con la topología de entradas/salidas, `Cell References` y `Controls`.

---

## CU-07 — Editor de Escenarios Estáticos y Simulación Monte Carlo

- **Entrada UI:** Menú `Tools -> Simulation -> Static Scenario Editor` (`Figs. 29–34`).
- **Flujo operativo:**
  1. Añadir columnas variables desde **`Unit Editor`** copiando una celda y usando el menú contextual **`Paste cell reference (Ctrl+Shift+V)`** (o `Add Cell Reference(s)` / `Create New SET/GET Column`).
  2. Configurar por cada columna: `Unit Name`, `Variable Name`, `Measurement Unit`, modo **`SET`** (entrada inyectada) o **`GET`** (respuesta capturada) y `Cell Reference`.
  3. Definir filas **`Run 1`..`Run N`** (`Number of Runs`), con parámetro global o por corrida de `Iteration Rounds` (`Rounds` columna `B`).
  4. **Modo determinístico:** poblar valores manualmente en columnas `SET` y ejecutar `Run Selected Scenario` o `Run All Scenarios`.
  5. **Modo estocástico (`Monte Carlo`):** abrir `Configuration`, activar `Monte Carlo = ON` en la fila correspondiente, elegir `Dist. Type` (`Uniform`, `Normal`, `Laplace`, `Trapezoid`), definir sus parámetros (`Min`, `Max`, `Mean`, `SD`, `b`, `c`) y opcionalmente **`Round to Nearest`**. Verificar previsualización en **`Distribution Charts`** (PDF/CDF) y visualizar resultados cruzados en las pestañas laterales **`Charts`**.

---

## CU-08 — Cálculo Estático, Parcial y Dinámico y Monitor de Convergencia

- **Entrada UI:** Menú **`Calculate`** (`Figs. 35–37`) y barras superiores de simulación estática/dinámica.
- **Modos de ejecución:**
  1. **`Static`:**
     - `Run Static Calculations`: evalúa una pasada/secuencia según configuración.
     - `Calculate Selected Units`: recalcula únicamente las unidades seleccionadas en el lienzo.
     - `Run Until Convergence`: itera hasta satisfacer `Criteria` (`Mass`, `Enthalpy`, `Both`) dentro de `Tolerance %` o agotar `Max Rounds`, actualizando en vivo el **`Convergence Monitor`**.
     - `Run Until Controls Converge`: acopla los iteradores de equilibrio/balance con los lazos de control activos.
  2. **`Dynamic`:** `Start Dynamic Simulation`, `Simulate Selected Units`, `Stop Dynamic Simulation`, selector `Speed: Full Speed`, configuración temporal (`Start Time`, `Time Step` p.ej. `1 s`, `Run Until Paused`, `End Time`) y barra verde de progreso (`0 min 0 s`).
  3. **`Functions`:** `Reset Partial Simulation`, `Clear Internal Streams`, `Empty All Tanks`, `Empty Tanks Before Calculation Starts`.

---

## CU-09 — Diagnósticos, Log Booleano, Orden de Cálculo, Referencias y Controles

- **Entrada UI:** Menú **`Diagnostics`** (`Figs. 38–40`) y pestañas inferiores del dock (`Controls`, `Cell Ref.`, `Log`).
- **Reglas funcionales:**
  1. **`Diagnostics & Suggestions`:** Inspección asistida de errores topológicos o de hoja.
  2. **`Log`:** Muestra mensajes categorizados con filtro por severidad (**`All`**, **`Warnings & Errors`**, **`Errors`**) y motor de búsqueda booleana con operadores **`AND, OR, NOT`**; opciones `Enable Calculation Log`, `Ignore Warnings`, `Show Log After the Calculation` y `Collect Data in Dynamic Scenario Editor`.
  3. **`Calculation Order`:** Permite marcar grupos/fases de evaluación en el diagrama asignando colores (`Set color 1/2/3`, `Remove color`, `Select All`).
  4. **`Cell References`:** Índice global con columnas `Unit Name | Cell Reference Name | Value`, botones `Sort`, `Navigate To Cell` y **`Search Controls`**.
  5. **`Controls` y `Control Monitor` (`Fig. 70`):** Tabla con columnas `Control Name | Location | Active | Set Point | Measured | Control Variable | Type`, conmutable mediante `Control ON/OFF`, visibilidad de filas/columnas, salto `Navigate To Control`, resaltado visual de estado activo/inactivo (rosa/verde) y seguimiento gráfico de convergencia por paso en `Control Monitor`.

---

## CU-10 — Visualización Sankey e Inspector de Corrientes

- **Entrada UI:** Menú `Flowsheet -> Flowsheet Visualization` / `Inspector (Stream Viewer)` y barra superior `Visualization` (`Figs. 4, 46, 68`).
- **Capacidades:**
  - `Activate Visualization`: activa la superposición sobre corrientes según la variable seleccionada en el desplegable (`Select visualization`) y sus unidades (`Measurement Units`).
  - `Visualization Settings` y **`Visualization Color Picker`**: configura qué metadatos mostrar (`Availability`, `Dataset Name`, `Measurement Units`, `Size Fraction Name`), `Use Visualization Layout`, paleta de colores (`Flowsheet stream colors`, `Positive and negative colors`, `Range colors`), límites `Min/Max`, `Use absolute values`, `Multiplier: 10^1`, `Scale Pages Separately`, `Visualize Stream Line Width` y `Stream Width Max`.
  - **`Inspector (Stream Viewer)`:** visor en tiempo real de propiedades e inventario de especies/partículas de cualquier corriente seleccionada en el diagrama.

---

## CU-11 — Reglas de Migración e Importación desde HSC Sim 7 (`40.4`)

Al abrir un archivo `*.fls` de **HSC Sim 7**:
1. El importador rechaza explícitamente archivos de **HSC Sim 6**.
2. Aplica validación estricta de referencias y consistencia; advierte en `Log` especies ausentes en bases de datos *hydro*.
3. Aborta la carga de asistentes de extracción por solventes de Sim 7 y de unidades encriptadas; omite curvas Bézier.
4. Carga en modo solo lectura los asistentes *minpro* de Sim 7 y bloquea la edición de `Stream Tables` heredadas.
5. Fuerza la visibilidad de corrientes ocultas, antepone **`REF`** a referencias externas, envuelve divisiones en **`safediv`** y sanea caracteres `/` en nombres de hoja.

---

## CU-12 — Ajustes por Defecto y Adaptación High-DPI (`40.5`)

- **`File -> General -> Sim Settings` (`Default Settings`):** Define preferencias iniciales globales en `5` pestañas (`Page`, `Unit`, `Stream`, `ArrowHeads`, `Header Label`), incluyendo color de fondo (`255; 255; 255`), grilla visible (`Show Grid = true`, `Grid Style = Points`, `Grid Color = 176; 196; 222`, `Grid Width = 2`, `Grid Height = 2`).
- **Soporte High-DPI (`Figs. 71–72`):** En pantallas de alta densidad donde los íconos o grillas presenten escalado deficiente, la arquitectura debe respetar *per-monitor DPI awareness* (y documentar la mitigación de compatibilidad de escalado DPI del sistema en entornos Windows heredados).
