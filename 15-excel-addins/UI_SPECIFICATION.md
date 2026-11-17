# Especificación de Interfaz (UI) — Módulo 15: Excel Add-Ins

## 1. Estética y Estructura General (Fiel a HSC6 + `AddInSample.xls`)
- **Título de ventana:** `"Excel Add-In Functions (HSC6.XLL) — Soryntel Chemistry"` (`[1020.0, 680.0]`, mínimo `[480.0, 360.0]`).
- **Barra de menú superior:** `File | Edit | Formula | Units | Help`
  - **File:** `Load AddInSample Preset`, `Clear Custom Sheet`, `Export CSV (Values)`, `Export CSV (Formulas)`, `Close`.
  - **Edit:** `Copy Selected Cell`, `Copy Formula`, `Paste Arguments`, `Reset Sample Values`.
  - **Formula:** `Insert Function...`, `Recalculate All (F9)`, `Toggle Show Formulas / Values`.
  - **Units:** cambio rápido equivalente a `=UNITS(T; E)` (`°C / K` y `kJ / MJ / kcal / Mcal / kWh`).
  - **Help:** `Add-Ins Activation Guide (27.1)`, `StreamEQ Array Help (27.4)`, `About`.
- **Toolbar compacto y Barra de Fórmulas estilo Excel:**
  - Controles rápidos: `T Unit [C ▼]`, `E Unit [MJ ▼]`, botón `Recalculate`, checkbox `Show Formulas`, badge de estado (`69 HSC6.XLL functions ready`).
  - Barra de fórmula: `[ Cell Ref (ej. C11) ]  [ fx ]  [ =H(D11;E11)   (editable) ]`

---

## 2. Pestañas del Módulo (`Add-In Workbook | Function Reference | StreamEQ & Migration`)

### 2.1 Pestaña 1: `Add-In Workbook` (Replica interactiva de `AddInSample.xls`, Fig. 3)
Grilla estilo hoja de cálculo con columnas fieles al archivo de muestra original de HSC6:
- **Columna A:** Grupo / Función (`General`, `Species`, `Reaction equation`, `Iteration (reverse)`, `Stream/Flow`, `Stream/Flow iteration (reverse)`, `Temp. independent`, `Percentage`, `CP-function related`).
- **Columna B:** Sintaxis de función (`H(Species;T)`, `BAL(Equation)`, ...).
- **Columna C:** **Return Value** (fondo crema/blanco claro, negrita para valores calculados en vivo o error Excel `#VALUE!` en rojo si aplica).
- **Columnas D–H:** **Argument 1 .. Arg. 5** (texto en **rojo** como indica Row 3 de `AddInSample.xls`: *"Red values are argument (input) values"*, editable directamente por el usuario para probar variaciones al instante).
- **Columna I:** **Description (Return value)** en texto legible.
- Incluye una sección final de **Celdas Libres (`Custom Scratch Rows`)** donde el usuario puede escribir cualquier fórmula `=FUNC(...)` referenciando celdas arriba o literales.

### 2.2 Pestaña 2: `Function Reference` (Catálogo Completo de `HSC6.XLL` + `HSC-Sim`)
- **Panel izquierdo (`~62%`):** Barra de filtro + selector de grupo + tabla con las **69 funciones de `HSC6.XLL`** (más las 5 funciones `StreamX..MineralX` marcadas como `HSC-Sim only`):
  - Columnas: `# | Function | Group | XLL Signature | Arguments | Return Unit / Type | Example`.
- **Panel derecho (`~38%`):** **Interactive Function Tester ("Insert & Test")**:
  - Muestra documentación de la función seleccionada, inputs editables para sus argumentos, resultado en tiempo real según las `UNITS` activas y botones `Copy Formula` / `Send to Scratchpad`.

### 2.3 Pestaña 3: `StreamEQ & Migration` (`Fig. 4–6` + `Update XLS Files.xls`)
Dividida limpiamente en dos subpaneles (con scroll adaptativo para pantallas pequeñas):
1. **Panel Superior/Izquierdo — `StreamEQ Array Formula Evaluator` (Fig. 4–6 / Filas 105–122 de `AddInSample.xls`):**
   - Inputs superiores: `Temperature [°C / K]` (default `1500.0`) y `Pressure [bar]` (default `1.0`).
   - Tabla bloque continua con encabezados de fase `§ PHASE 1:`, `§ PHASE 2:` resaltados en gris-azulado, columna `Input (mol)` editable y columna `Output (mol)` rodeada por un borde de **bloque matricial `{=StreamEQ(...)}`**, más el resumen paso a paso de uso en Excel (`Ctrl + Shift + Enter`) e indicador de balance elemental conservado.
   - Botones `Load Combustion Preset (C + Air)` y `+ Add Phase / Species`.
2. **Panel Inferior/Derecho — `Update XLS Formulas (HSC5.xla -> HSC6.XLL)` (Fig. 2):**
   - Área de texto de entrada para pegar fórmulas o bloques con referencias antiguas (`='C:\HSC5\AddIns_BackUp\HSC5.xla'!H(D11;E11)`), botón `Convert to HSC6 Formulas`, salida limpia lista para copiar (`=H(D11;E11)`) y reporte de auditoría de funciones detectadas.

---

## 3. Barra de Estado Inferior
- Pie estilo clásico `Outotec / Soryntel` + unidades activas (`Active Units: T = C, E = MJ`), número de celdas recalculadas y errores de evaluación si los hubiera.
- **Responsividad:** A `[480.0, 360.0]` los paneles se apilan verticalmente con `ScrollArea` independiente evitando cualquier solapamiento.
