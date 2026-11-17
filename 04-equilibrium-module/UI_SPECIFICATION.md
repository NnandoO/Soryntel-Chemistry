# Especificación de Interfaz de Usuario (UI) — Módulo 4: Equilibrium Calculations

## 1. Integración en la Aplicación de Escritorio (`soryntel-desktop`)

- Se añade la pestaña **4. Equilibrium (Gibbs)** en la barra superior de módulos junto a `1. Reaction Equations`, `2. Heat & Material Balance`, `3. Heat Loss`.
- Mantiene el tema claro tipo Excel/Outotec ya unificado en `excel_grid.rs` (`col_letter`, `header_cell`, `row_num_cell`, `section_cell`, `text_cell`, `widget_cell`).

## 2. Layout Principal (Split Horizontal: Workbook Excel + Panel `Parameters`)

### 2.1 Área Izquierda: Workbook Estilo Excel (`Species` / `Results`)
- **Barra de Fórmulas Superior:**
  - `Active species:` (etiqueta de solo lectura) + selector de celda (`E5`, `C13`, etc.) + campo de edición de fórmula/valor (soporta expresiones en Columna C como `=EXP(8495/T - 2.653)` o valores numéricos).
- **Barra de Herramientas Rápida:**
  - `From Elements (Normal)`, `From Elements (Cell)`, `Add Species`, `Insert Phase`, `Delete Row/Phase`, `Sort Phases`, `Add N2(g) Stability`, `Warnings / Cp Range`, `Export Results`.
- **Hoja `Species` (Columnas `A..H` con encabezados de letra Excel y números de fila `1..N`, modo estándar):**
  - `A`: `Species Formula` (filas de fase en negrita + fondo gris claro `Phase #1`, `GAS`, `AQUA`, `SOLID`; filas de especie indentadas).
  - `B`: `Feed Temperature (°C)` (celda editable numérica, default `25.000`).
  - `C`: `Activity Coefficient (f)` — en fila de fase muestra `ComboBox` (`Mixture`, `Pure`, `Aqua`, `Ga-As Mixture`, `Al-Zn Mixture`); en fila de especie es celda numérica/fórmula (`1.000` por defecto).
  - `D`: `Amount Unit` (`kmol`, `kg`, `Nm3`, `g/l`, `mol/l`).
  - `E`: `Initial Amount` (total `SUM` de solo lectura en fila de fase; editable blanco en especies).
  - `F`: `Initial Amount (%)` (calculado automáticamente por fase).
  - `G`: `Add X-amount` (editable cuando `X-amount` es variable independiente o adición).
  - `H`: `Add Z-amount` (editable cuando hay 2 variables independientes).
- **Columnas dinámicas según modo (Lote 1, Figs. 9 y 13):**
  - En `Step Mode = Remove % (Add Amount)` (Fig. 9): la columna `H` cambia a `Remove Amount (%)`, permitiendo especificar el porcentaje removido por fase/especie en cada paso.
  - En `Calculation Mode = Fixed Activity` (Fig. 13): se inserta la columna `D: Fixed Activity` (entre `Activity Coefficient` y `Amount Unit`), desplazando `Amount Unit`, `Initial Amount`, `Initial Amount (%)` y `Add X-amount` a `E..H`.
- **Hoja `Results` / `Results1` (Fig. 32, generada automáticamente tras calcular):**
  - Encabezado de 3 filas estilo Excel: Fila 1 `Type` (`Step`, `AC`, `Gmin/RT`, `H`, ...), Fila 2 `TargetSpecies` (`CO(g)`, `CO2(g)`, ...), Fila 3 `Units` (`kJ/mol`, etc.). Filas `4..N`: valores paso a paso (`1.000, 2.000, ...`), más `pH` si hay fase `Aqua` y `Cell Voltage (V)` en `Cell Mode`.
- **Hoja / Bloque `UserFunction` (Fig. 23, `User Formulas`):**
  - En la vista `UserFunction` se muestra la tabla de estado (`Temperature`, `Pressure`, `Volume`, `Enthalpy`, `Discharge level`, `Step`), las especies con `Initial Amount`, `Equilibrium Amount`, `Equilibrium Composition (%)`, `Activity Coefficient (f)`, `Enthalpy (MJ)`, `Entropy`, y un bloque inferior `User Formulas` donde el usuario define fórmulas Excel personalizadas (p. ej. `=D10+D28` para `As + As2(g)`) con opciones `Formula`, `Difference`, `Cumulative` graficables directamente.
- **Hoja `Target Results` (Fig. 8, cuando `Target Calculations` está activo):**
  - Bloque superior resumen estilo Excel: `Sought-for function`, `Sought-for value`, `Results found`, `Achieved value`, `Variable step parameters` (`Temperature` / etc.), `Constant parameters` (`Pressure`, `Amount` de entrada), `Calculated results` (`Volume`, `Reaction Enthalpy`).
  - Bloque inferior por fase y especie con columnas: `Equilibrium Amount (kmol)`, `Equilibrium Composition (mol fraction)`, `Activity coefficient`, `Activity (mol fraction)`.
- **Selector de Hojas Inferior:** `[◄][►] | Species | UserFunction | Results1 | Target Results |`.

### 2.2 Panel Derecho Acoplado: `Parameters` (Ancho fijo ~330 px con scroll vertical)
1. **Operations:** botones `Calculate` (primario), `Show Chart`, `Cancel` + barra de progreso + estado de convergencia.
2. **System Parameters:**
   - `Chart header` (TextEdit)
   - `Calculation Mode`: `Normal` | `Cell Mode`
   - `Step Mode`: `Add Amount` | `Final Amount`
   - Checkboxes: `Allow titrations`, `Allow constraints`
   - `User defined parameters`: `N,T,P` | `N,H,P` | `N,P,V` | `N,T,P,Q`
   - `Number of independent variables`: radio `0` | `1` | `2`
   - `Define state change for`: checkboxes contextuales (`X-amount`, `Temperature`, `Volume`, `Enthalpy`, `Pressure`, `Z-amount`, `Discharge level`).
3. **System State:** matriz tabular compacta `Initial | Final | Unit | Steps | Log`:
   - Filas: `X-amount`, `Z-amount`, `Temperature`, `Pressure`, `Volume`, `Enthalpy`, `Discharge Level`.
   - Pie: `Use as base volume` + `Initial Pressure (bar)`.
4. **Calculation Options:**
   - `Infinite Phase` (dropdown con las fases actuales o `None`)
   - `Mixing Entropy conversion for aqueous species` (checkbox, default `true`)
   - `Criss-Cobble` (checkbox, default `false`)
5. **Phase Data:**
   - Selector de fase + `Solution Model` (`Ideal Mixture`, `Pure Substance`, `Aqua`, `Ga-As Mixture`, `Al-Zn Mixture`) + `AC Steps` (spinbox, default `3`).
6. **Target Calculations (colapsable):**
   - `Enabled`, `Equilibrium Property`, `Target` (especie/propiedad), `Target Value`.

## 3. Ventanas Secundarias / Modales (Actualizado con Lote 1: Figs. 1–17)
- **Dialog `Database Browser / From Elements` + `Sort Species` (Figs. 3–5):**
  - Filtros de texto (`Elements`, `Formula`, `Stoichiometry`, `Keywords` + modo `Possible Species` / `Free Ratio`), `Type Filters` (`Gases`, `Gas Ions`, `Condensed`, `Electrons`, `Liquids`, `Aqueous Ions`, `Aqueous Neutrals`, `Fluids`), `Organic Filter` (`Include Organics`, `Organics Only`), lista `Matching Species`, vista previa `Basic Data` + `Temperature Ranges` + minigráfica (`Enthalpy / Entropy / Heat Capacity / ΔG`), y botón `Import species`.
  - Seguido del modal `Sort Species` (`Add N2(g) for calculation stability`, `Phases`, `Gas, Aqua, Pure`, `No sorting`, `Gas, Aqua, Liquids`).
- **Dialog `Warnings` (Extrapolación de $C_p$, Figs. 35–38):** tabla Excel (`A..I`) con checkboxes para excluir especies fuera de `[Tmin, Tmax]`, valores excedidos en rojo, checkbox `Do not show this dialog` y botones `Print`, `OK`, `Cancel`.
- **Dialog `Equilibrium Results` (`Show...Table`, Fig. 33):** ventana modal tipo Excel con columnas `Data Type`, `Species`, `Phase`, `Units`, `MW g/mol`, `Min`, `Max`, y columnas `Data: 1, 2, 3...` (fila de cabecera `Temperature` arriba) + botones `Copy All`, `Print`, `Close`.
- **Dialog `Select data for axes` (Fig. 17):**
  - Tres columnas de radio buttons: `X-Axis (Horizontal Axis)`, `Y-Axis (Vertical Axis)`, `Z-Axis (Depth Axis)` con opciones (`Amount`, `Step`, `Temperature`, `Pressure`, `Equilibrium Amount`, `Equilibrium Composition`, `Activity coefficient`, `Activity`, `Volume`, `pH`, `Reaction Enthalpy`, `Element Fraction`, `Partial Pressure`, `Phase amount`, `Phase molar fraction`, etc.).
  - Selectores de `Units for Amount` (`kmol`, `mol`, `kg`, `kg, Nm3`), `Units for Temperature` (`°C`, `K`) y `Chart Type` (`Surface`, `Scatter`, `Cloud point`).
- **Dialog `Format Chart` (Figs. 20–22):** pestañas `General`, `X-axis`, `Y-axis`, `Z-axis`, `Legend box`, `Line Series`, `Regression`, `Surface`, `3D Diagram` (escala lineal/log, rejilla mayor/menor, grosor/opacidad, `Color by levels` con paleta editable).
- **Vista `Chart` (2D / 3D — Figs. 10, 12, 14–16, 19, 24–31):**
  - Área de gráfica interactiva + barra de herramientas de dibujo/anotación (línea, flecha, rectángulo, elipse, texto) con `Object Editor` tabular (`Shapes`/`Labels`), panel lateral derecho `Chart parameters` (`Legend` con checkboxes, `Contains All` por elementos y filtro por fases) con `Auto Scale` automático al filtrar, y soporte `Color by levels` en superficies 3D.
