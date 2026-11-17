# Especificación Funcional — Módulo 20: Mass Balance (`HSC - Mass Balance`)

La especificación funcional cubre el ciclo completo desde la captura topológica y tabular (`51.4.1–51.4.4`), la ejecución y congelamiento del balance (`51.4.5`), la reportería y exportación (`51.4.6`), la importación heredada HSC7 (`51.4.7`), el catálogo de botones (`51.5`) y las `12` validaciones previas (`51.6`) de [51-mass-balance.md](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.md).

---

## 1. Casos de Uso Principales (`CU-01 .. CU-12`)

### CU-01 — Lanzamiento desde HSC Sim y Selección de Unidades (`51.4.1–51.4.2`)
- **Precondición:** Diagrama de flujo construido en **HSC Sim** (`Fig. 3`: unidades y corrientes etiquetadas con conectividad entrada/salida; no requiere valores simulados previos).
- **Flujo principal:**
  1. Desde HSC Sim, el usuario abre `Tools -> Mass Balancing` (o desde la baldosa **`Mas`** del lanzador).
  2. La vista **`1. Units`** carga todas las unidades del diagrama (`Conditioner`, `Rougher 1..5`, etc.).
  3. El usuario marca/desmarca **`Selected`** para restringir el subcircuito a reconciliar y activa **`Size Transforming`** en unidades con conminución/aglomeración (molinos, chancadores) para omitir balance granulométrico en ellas.
- **Resultado:** Subgrafo unitario activo definido; indicador del navegador actualizado (ej. `Units (6/6)`).

### CU-02 — Configuración de Corrientes y Tipado (`51.4.2.2`, `Fig. 5`)
- **Flujo principal:**
  1. En **`2. Streams`**, el usuario marca/desmarca **`Selected`**, define el orden visual de corrientes mediante `Move Up` / `Move Down` y configura columnas **`Type`** (`Unknown`, `Solids/Slurry`, `Liquid/Water`), **`Flows`** (check), **`Assays`** (check), **`Sampling Device`** y **`Sample Quality`** (`Good`, `Moderate`, `Bad`).
  2. Puede pulsar **`Detect Stream Types`** para clasificar automáticamente corrientes de agua pura frente a pulpas/sólidos o usar **`Unselect Streams with no Data`** en etapas posteriores.

### CU-03 — Definición de Variables, Elementos, Minerales y Streams `.HSCStream` (`51.4.2.3`, `Tabla 2`)
- **Flujo principal:**
  1. En **`3. Variables`**, el usuario agrega variables manualmente (`Add Variable ▾`), selecciona elementos químicos desde tabla periódica (`Add Elements...`), agrega minerales desde la base mineralógica (`Add Mineral from DB...`) o importa minerales desde archivos existentes (`Import Minerals from Stream` `*.HSCStream`).
  2. Asigna a cada variable: `Variable Name`, `Meas. Unit` (`t/h`, `g`, `%`, `g/t`, `ppm`), `Type Code` / `Type` (`SF Total Solids`, `SC Solids Component`, `A Assay of Solids Component`, `M Mineral Assay`, `SA Size Fraction Assay`, `SP Solids Percentage`, `Info`), y el modelo inicial de error (`Measurement Method`, `a`, `b`, `c`).
  3. Un solo rol `SF` (`Total Solids`) puede estar seleccionado simultáneamente en un balance dado.

### CU-04 — Fracciones Granulométricas y Múltiples Datasets (`51.4.2.4–51.4.2.5`)
- **Flujo principal:**
  1. En **`4. Size Fractions`**, existe por defecto la fracción `'Bulk'` (no eliminable, solo renombrable). El usuario agrega o elimina tamices (`Add Size Fractions ▾` / `Remove Size Fractions`).
  2. En **`5. Data Sets`**, gestiona escenarios experimentales independientes (`Add Data Set`, `Remove Data Set`, `Clone Data Set` copiando toda la configuración y métricas).

### CU-05 — Ingreso, Pegado Inteligente y Manipulación de Datos Experimentales (`51.4.2.6`, `Fig. 9`)
- **Flujo principal:**
  1. En **`6. Measurement Data`**, el sistema genera automáticamente la plantilla `Streams × Variables` (transponible mediante **`Flip Table`**).
  2. El usuario introduce valores mediante: (a) escritura manual directa; (b) **`Paste Experimental Data`** desde Excel con detección automática de orientación horizontal/vertical por nombres de corrientes y variables; (c) **`Add Round` / `Remove Round` + `Calculate Master Data`** para promediar múltiples campañas/muestreos.
  3. Herramientas auxiliares en cinta `Data Tools`:
     - **`Element to Mineral...`**: convierte leyes elementales a composición modal mineralógica (vía algoritmo *HSC Geo ch. 84.6*).
     - **`View Mineral Matrix`**: inspecciona la matriz estequiométrica mineral × elemento y densidades específicas.
     - **`Sol-% to Water`**: convierte pares (`Total Solids`, `Solids %`) en flujo másico de agua derivado.
     - **`Show SG`**: calcula y muestra gravedad específica sólida de cada corriente.
     - **`Stream by Equation`**: fija el flujo total de sólidos de una corriente como múltiplo lineal de otra (ej. `Feed Mass = 2 * RC1`, celda sombreada en verde claro con tooltip descriptivo).
     - **`Estimate Missing Values` / `Clear Estimated Values`**: estima celdas vacías en fuente azul (se recomienda evitar antes del primer intento real de balance para no sesgar).
     - **`Enter Recoveries`**: carga recuperaciones medidas directamente desde portapapeles/tabla para exportar luego hacia **Model Fit** (`Save to Flowsheet`).

### CU-06 — Revisión de Integridad y Estados Celda a Celda (`51.4.3`, `Figs. 10–12`)
- **Flujo principal:**
  1. En **`Review Data -> Data Status`**, el usuario alterna **`Layout: Stream View ▾`**, conmuta **`Balancing of Minerals`** (excluyendo automáticamente ensayos químicos `A` para evitar sobre-restricción o conflicto mineral–elemento), inspecciona los estados previos **`Missing`**, **`Measured`**, **`Guesstimated`**, **`Fixed`**, **`Excluded`** y ajusta manualmente cualquier celda o revierte con **`Reset Meas. Status`**.
  2. Pulsa **`Balance Equations`** (`Fig. 11`) para revisar la lista expandible de ecuaciones lineales generadas por unidad (`RC Feed = RT1 + RC1`, etc.).
  3. Observa en tiempo real los dos gráficos circulares de estado (`Measured Data` y `Balanced Data`, actualizables con **`Refresh Charts`**).

### CU-07 — Asignación y Revisión de Desviaciones Estándar (`51.4.4`, `Figs. 13–14`)
- **Flujo principal:**
  1. En **`Standard Deviations -> Edit SD Table`**, el usuario revisa o sobrescribe manualmente columnas `SD`/`RSD` (valores negros = fijos por usuario; valores azules = calculados automáticamente por fórmula HSC), pega matrices desde portapapeles (`Paste SD or RSD Data`), computa varianzas empíricas (`Calculate SD From Data`) o bloquea edición (`Lock/Unlock Table`).
  2. En **`Review SD`**, ajusta `Measurement Method` (`Fixed Absolute`, `Fixed Relative`, `S-shape`, `Mineral Grade Based`, `Size Fraction (Whiten)`), `Min`, `Max` y visualiza los gráficos de barras `Standard Deviations` y `Relative Standard Deviations` por corriente.

### CU-08 — Ejecución, Diagnóstico y Congelamiento en Etapas (`51.4.5`, `Figs. 15–19`)
- **Precondición:** Validación **`Check for Errors`** sin errores bloqueantes (`51.6`).
- **Flujo principal:**
  1. En **`Balancing -> Balance`**, configura en la cinta / panel `Properties`: **`Method`** (`LS`, `LS MinMax`, `NNLS`, `CLS`, `LLS`), **`Sizing`** (`Unsized`, `Sized`, `Sized by Assay`), **`Balance Type`** (`Solids only`, `Assays only`, `Solids and assays`, `Solids, water and assays`), **`Assay Sum=100`**, **`Mineral Sum=100`**, **`Low Grade Weighting`** (`Coefficient` + lista de variables), **`Scaling`** (`true` por defecto), `Balance Tolerance` (`0.001`), `Max Iteration` (`1000`), `Estimate of Null SD` (`0.001`).
  2. Pulsa **`Calculate`** (o **`Batch Run ▾`** para múltiples datasets).
  3. Analiza tabla de resultados (`Measured`, `Balanced`, `±CI`, `Diff`, `RelDiff %`, `Recovery-%` con `Show Data Bars`), la curva **`Balance Convergence`** y el **`Parity Chart`** (`Log` on/off).
  4. Para balances jerárquicos por zonas o escalas, utiliza **`Freeze Balanced`** tras converger una subetapa (fijando resultados como entradas invariantes para el paso siguiente, ej. `Bulk/Sized` antes de `Sized by Assay`), o reinicia mediante **`Clear & Unfreeze Balanced`**.
  5. Si las leyes elementales ya convergieron, puede ejecutar **`Balanced Elements to Minerals...`** post-balance.

### CU-09 — Comparación Multi-Dataset, Reportes Tabulares y Gráficos (`51.4.6`, `Figs. 20–23`)
- **Flujo principal:**
  1. Con `>= 2` datasets calculados, **`Balancing -> Compare`** contrasta balances lado a lado.
  2. En **`Reporting -> Results`**, inspecciona y copia/exporta (`Copy ▾`, `Export to Excel`, `Save Stream As...`, `Stream Tables ▾`) las vistas **`Stream Summary`** (caudales, leyes y recuperaciones %), **`Goodness`** (resúmenes por variable y por corriente: `WSSQ`, `Diff.Tot`, `Rel.Dif`, `AVG SD`, `AVG RSD`, `RMSD`, `Relative RMSD`), **`Unit Balance`** (`Inputs`, `Outputs`, `Balance`, `Tolerance Error`), **`Balance details`** y ejecuta **`Calculate Volumetric Flows ▾`** o **`Calculate All Mineral Elements`**.
  3. En **`Reporting -> Charts`**, gestiona paneles (`Add New Panel`), agrega gráficos de barras (`Add Bar Chart`, `Add Default Charts ▾`: `Total Solids`, `Stream Composition`), personaliza propiedades (`Chart Properties`) o restablece (`Reset`).

### CU-10 — Sincronización con HSC Sim y Respaldo `.HSCMas`
- **Flujo principal:** **`Save to Flowsheet ▾`** escribe los resultados reconciliados en el diagrama `.Sim` y habilita el salto hacia *Model Fit*; **`Discard and Close`** descarta ediciones en sesión; **`Backup ▾`** abre o guarda copias externas completas **`.HSCMas`**.

### CU-11 — Importación Heredada HSC7 Excel (`Analyses.xls`) (`51.4.7`, `Figs. 24–28`)
- **Flujo principal:**
  1. Pulsa **`Import HSC7 Data`** y selecciona el archivo `.xls/.xlsx`.
  2. En el diálogo modal (`Import Options` | `HSC7 Data`), configura: **`Dimension`** (`1D - Unsized Data`, `1.5D - Size Fraction Data`, `2D - Size Fraction With Assays`), **`Direction`** (`Vertical Data` — solo válido para `1D` o `1.5D` — o `Horizontal Data`), hojas en **`Select Analyses Sheets`**, hoja de topología **`Select Flowsheet`** (`[Nothing]`, `Flowsheet`, `Streams`, `Units`, `MineralMatrix`, `Balanced`) y **`Flowsheet Direction`** (`Units Horizontal` o `Streams Horizontal`).
  3. Confirma con `OK`; el parser lee la matriz de incidencia `+1/-1/0` (o columnas `Source`/`Destination` con `?` como frontera abierta) y puebla topología, corrientes, fracciones y mediciones.

### CU-12 — Importación desde Repositorio HSC ReX
- **Flujo principal:** **`Import ReX Data`** importa conjuntos experimentales y análisis desde un repositorio HSC ReX directamente hacia las grillas de variables/mediciones del dataset activo.

---

## 2. Catálogo Completo de Reglas de Verificación de Errores (`51.6`)

El botón **`Check for Errors`** (y la validación pre-cálculo antes de `Calculate`) evalúa exactamente las `12` condiciones documentadas en `51.6`:

| # | Severidad | Mensaje Exacto Documentado (`51.6`) | Condición de Disparo | Acción Correctiva Indicada |
|---|---|---|---|---|
| **1** | Error | `Units: No selected units found` | Ninguna unidad tiene `Selected = true` | `At least one unit should be selected` |
| **2** | Error | `Streams: No selected streams found` | Ninguna corriente tiene `Selected = true` | `Some of the streams should be selected` |
| **3** | Error | `Units: No input stream` | Al menos una unidad activa carece de corrientes entrantes | `Found a unit with no input streams` |
| **4** | Error | `Units: No output stream` | Al menos una unidad activa carece de corrientes salientes | `Found a unit with no output streams` |
| **5** | Error | `Streams: Source and destination missing` | Una corriente activa no tiene unidad origen ni destino conectada | `Found a stream with no source and destination` |
| **6** | Error | `Data: No data found` | Matriz de mediciones vacía en el dataset activo | `Add data in the measurement data view` |
| **7** | Error | `Variables: No selected assays found` | Ninguna variable seleccionada es `A` (*Solids Component Assay*) ni `M` (*Mineral Assay*) cuando el balance requiere leyes | `There should be at least one selected variable of the Mineral Assay or Solids Component Assay type` |
| **8** | Error | `Streams: All stream types unknown` | Todas las corrientes activas tienen `Type = Unknown` | `Go to Streams and press Detect Stream Types button or set the stream types manually` |
| **9** | Error | `.9 Stream Types: Unknown found` | Existe al menos una corriente con `Type = Unknown` (conserva prefijo `.9` literal en manual) | `Go to Streams and press Detect Stream Types button or set the stream types manually` |
| **10** | Error | `Units: No combined units found` | El grafo unitario no puede cerrarse/combinarse en subcircuito balanceable por topología inconsistente o déficit crítico de mediciones | `There can be several reasons why combined units cannot be formed. There may be too many missing measurements or there may be errors in the flowsheet.` |
| **11** | Aviso | `Notification: Min or Max values detected: It is recommended to use the CLS method` | Existen cotas `Min` o `Max` no nulas definidas pero `Method != CLS` | Cambiar método a `CLS` para respetar cotas simples |
| **12** | Aviso / Bloqueo | `Notification: Method LLS not available for sized solution: It is recommended to use LS, NNLS, or CLS method` | Usuario selecciona `Method = LLS` junto con `Sizing = Sized` o `Sized by Assay` | Cambiar método a `LS`, `NNLS` o `CLS` |

---

## 3. Reglas Funcionales de Compatibilidad (`Tabla 1`, `51.2`)

| Combinación (`Data to Balance` × `PSD Balance`) | Requisitos Obligatorios y Reglas de Cálculo |
|---|---|
| `Solids only` × `Unsized` | Al menos `1` medición `SF` (*Total Solids*). No requiere leyes `A`/`M`. |
| `Assays only` × `Unsized` | **Todas** las corrientes activas deben tener caudal `SF` definido (`Fixed`/fijado). Solo reconcilia leyes y recuperaciones relativas. |
| `Solids and assays` × `Unsized` | Al menos `1` medición `SF` + al menos `1` variable de ley (`A` o `M`). |
| `Solids, water and assays` × `Unsized` | Al menos `1` medición `SF`, mediciones de agua y/o `% sólidos` (`SP`), más leyes. Resuelve sólidos primero y luego agua (`Eqs. 5–7`). |
| `*` × `Sized` | Requiere fracciones adicionales a `'Bulk'`; unidades con `Size Transforming = true` excluidas de ecuaciones granulométricas (`Eqs. 13, 17`). Bloquea `LLS`. |
| `*` × `Sized by Assay` | Exige haber resuelto previamente `Unsized` o `Sized` para contar con $F_{\text{flow,tot}}$ congelado/disponible (`51.7.3`). Bloquea `LLS`. |
