# Especificación de Interfaz (UI) — Módulo 14: Units Module

## 1. Distribución General (Estilo Clásico HSC6 `Units` Workbook)
La ventana emula el libro multicapa clásico de `UNITS4.UNI` con barra de menú superior, cinta/selector rápido superior de hoja y área principal tipo hoja de cálculo limpia:

### 1.1 Barra de Menú Superior (`File | Edit | Format | Help`)
- **File:** `New Custom Unit...`, `Load Workbook JSON...`, `Save Workbook JSON...`, `Export Active Sheet CSV...`, `Reset Default HSC6 Database`, `Exit`.
- **Edit:** `Copy`, `Paste`, `Swap Left <-> Right` (en `Convert`), `Add Unit to Selected Quantity`, `Add New Quantity`.
- **Format:** `Significant Figures (4 / 6 / 8 / Auto)`, `Scientific Notation`, `Show Equivalent All-Units Table`, `Font Size`.
- **Help:** `Units Module Guide`, `SI Prefixes Reference`, `About Units Module`.

### 1.2 Selector de Hojas del Libro (Bottom/Top Workbook Tabs, Estilo HSC6)
Seis pestañas ordenadas idénticamente a `UNITS4.UNI`:
`[ Convert ] [ Units ] [ Constants ] [ Mesh ] [ Air ] [ H2O ]`

### 1.3 Diseño por Hoja
1. **Hoja `Convert` (Calculadora Principal, Fig. 1 de HSC6):**
   - Cabecera superior clara con el bloque **Quantity**: ComboBox buscable/desplegable con las 90 magnitudes + etiqueta informativa con símbolo físico y dimensiones SI (`Symbol: E | SI Base: m2 kg s-2`).
   - Cuadrícula principal estilo Excel de dos lados simétricos (**Units 1 / Data 1** a la izquierda y **Units 2 / Data 2** a la derecha):
     - **Columna 1:** Selector `Unit 1` + selector `Prefix 1` (`(none)`, `Y..y`) + campo numérico **Data 1** (fondo blanco, texto azul/rojo editable con alineación derecha).
     - **Columna central estrecha:** Botón `↔ Swap` y signo `=`.
     - **Columna 2:** Selector `Unit 2` + selector `Prefix 2` + campo numérico **Data 2** (igualmente editable bidireccionalmente: escribir en `Data 1` actualiza `Data 2`, y escribir en `Data 2` actualiza `Data 1`).
   - Tarjeta resumen inferior compacta: ecuación directa (`1 <p1><u1> = <factor> <p2><u2>`) y vista opcional expandible **"All equivalents in `<Quantity>`"** que muestra simultáneamente el valor convertido a todas las unidades de esa magnitud.
2. **Hoja `Units` (Base de Datos en Bloques de 3 Filas, Fig. 2 de HSC6):**
   - Grilla tipo hoja de cálculo (`excel_grid.rs`) con congelación de la primera columna (`Quantity / Row Type`) y barra superior de herramientas `+ Add Unit`, `+ Add Quantity`, filtro por nombre de magnitud.
   - Coloreado suave alternado por bloque de 3 filas (`Name` en negrita sobre fondo gris-azulado claro, `Symbol` sobre fondo claro, `Coefficient` numérico alineado a la derecha sobre fondo blanco/editable).
3. **Hoja `Constants`:**
   - Barra de búsqueda rápida + tabla 4 columnas (`Constant Name | Symbol | Value | Unit`) con botón `Copy Value` por fila o doble clic para copiar al portapapeles.
4. **Hoja `Mesh`:**
   - Panel izquierdo: tabla clásica `Grain Size Units (mesh | mm | µm)`.
   - Panel derecho compacto: convertidor rápido `Mesh ↔ Opening (mm / µm)` con interpolación logarítmica y tabla de tamices estándar cercanos.
5. **Hoja `Air`:**
   - Bloque superior: tabla `Composition of Air (excluding H2O)` (`Component | Vol-% | Wt-%`) + calculadora psicrométrica puntual `(T °C, RH %, P bar) → P_sat, P_H2O, vol-%, g/Nm³`.
   - Bloque inferior desplazable: matriz completa `Water Content of Air` con selector de unidad (`H2O vol-%` | `H2O g/Nm³`), filas de temperatura (`102 °C .. -50 °C`) y columnas de humedad relativa (`100 % .. 10 %`).
6. **Hoja `H2O`:**
   - Tabla de 3 columnas `Vapour Pressure of Water (Temp. °C | Pressure kN/m² | Pressure atm)` desde `370 °C` hasta `-90 °C`, junto a calculadora rápida de saturación `T → P_sat` y `P → T_sat` alimentada por el módulo `water`.

### 1.4 Barra de Estado Inferior
- Muestra identidad `Outotec / Soryntel` + resumen activo (p. ej. `90 Quantities | 444 Units loaded | Sheet: Convert`) o advertencias de rango.

## 2. Requisitos de Responsividad
- Tamaño mínimo soportado `[480.0, 360.0]` sin superposición de controles ni desbordes de texto gracias a paneles adaptativos y `ScrollArea::both()` en grillas largas.
