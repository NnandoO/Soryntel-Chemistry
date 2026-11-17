# Especificación de Interfaz (UI) — Módulo 13: Water Module (`PTCalc`)

## 1. Distribución General (Estilo Clásico HSC `Pressure-Temperature Calculator`)
La interfaz se compone de una barra de menú superior, un panel izquierdo tipo hoja de cálculo y un panel derecho con el diagrama de fases $p\text{-}T$ interactivo:

### 1.1 Barra de Menú Superior (`File | Edit | Show | Help`)
- **File:** `Reset Default (Steam Table)`, `Copy Worksheet`, `Export CSV...`, `Save Diagram SVG...`, `Exit`.
- **Edit:** `Point 2 = Triple Point`, `Swap Point 1 <-> Point 2`, `Match Isentropic Point 1 (ΔS = 0)`, `Copy Difference Values`.
- **Show:** Toggle `Show Line (Point 1 — Point 2)`, `Show Ice Polymorphs Labels`, `Show Gridlines`, `Show Critical/Triple Callouts`, `Show Isentropic Efficiency Calculator`.
- **Help:** Acerca del módulo `Water (PTCalc)` y rangos de validez.

### 1.2 Panel Izquierdo — Hoja de Cálculo (`Calculation Worksheet`)
1. **Cabecera de Configuración Global:**
   - Campo numérico `Amount` + ComboBox unidad de cantidad (`kg` / `kmol`).
   - Selectores de unidades en línea compacta:
     - `T`: `°C` | `K`
     - `P`: `bar` | `MPa`
     - `H, G`: `kJ` | `kcal` | `Wh` | `kWh` | `MWh`
     - `S, Cp`: `J/K` | `kJ/K` | `kcal/K` | `kWh/°C` | `MWh/°C` (respetando escala de temperatura seleccionada)
     - `Density`: `kg/m³` | `g/cm³`
2. **Grilla de 4 Columnas (`Property / Units | Point 1 | Point 2 | Difference`):**
   - **Filas de Entrada (fondo claro/editable):**
     - `Temperature` (`°C` o `K`) — texto negro no editable cuando `Sat. (l,s)` la calcula automáticamente; editable en el resto.
     - `Pressure` (`bar` o `MPa`) — texto negro automático cuando `Sat. (g,l)` o `Sat. (g,s)` la calculan; editable en el resto.
     - `Phase` — ComboBox por columna (`Auto`, `Solid (s)`, `Liquid (l)`, `Gas (g)`, `Sat. (g,l)`, `Sat. (g,s)`, `Sat. (l,s)`).
     - `Mixture %` — visible/habilitada únicamente cuando la fase seleccionada es saturada (`Sat. (*,*)`).
   - **Separador y Filas de Resultados:**
     - `Enthalpy` (`H`) — celda `Difference` resaltada en **amarillo suave** (`#FEF9C3`, estilo HSC).
     - `Entropy` (`S`) — celda `Difference` resaltada en **amarillo suave**.
     - `H specific` — entalpía sensible respecto a $25\,^\circ\text{C}$.
     - `Gibbs Energy` (`G`)
     - `Heat Capacity` (`Cp`)
     - `Density` ($\rho$)
   - **Bloque Inferior Opcional (`Isentropic Analysis`):** cuando está visible o hay flujo de turbina/compresor, muestra $\Delta h$, $\Delta h_s$ ($\Delta S=0$) y $\eta_s$.

### 1.3 Panel Derecho — Diagrama Interactivo $p\text{-}T$
- Gráfico cartesiano con eje X lineal en Temperatura (`-200` a `2000 °C` o zoom rápido `[-50, 600 °C]`) y eje Y **logarítmico** en Presión (`0.001` a `1000 bar`).
- **Elementos dibujados:**
  - Regiones sutilmente sombreadas: `Solid Ice` (con referencias de fases `Ih`, `III`, `V`, `VI` de alta presión), `Liquid`, `Gas (Steam)` y cuadrante `Supercritical`.
  - Curvas continuas nítidas: sublimación `Sat. (g,s)`, vaporización `Sat. (g,l)` terminando en `Critical Point (373.946 °C, 220.64 bar)` y fusión `Sat. (l,s)` partiendo desde `Triple Point (0.01 °C, 0.0061 bar)`.
  - Marcadores de proceso `[1]` y `[2]`: caja **amarilla** para el punto activo y **blanca** para el punto inactivo; clic en el marcador selecciona el punto activo; clic o arrastre sobre el lienzo mueve el punto activo y recalcula al instante.
  - Línea discontinua/sólida entre `[1]` y `[2]` cuando `Show -> Line` está activo.

## 2. Requisitos de Responsividad y Calidad Visual
- Tamaño mínimo soportado `[480.0, 360.0]` con barra horizontal/vertical (`ScrollArea`) para evitar solapamientos.
- Tema claro limpio estilo escritorio técnico HSC6 con contraste alto y clip estricto en los trazos de `egui_plot` o lienzo vectorial.
