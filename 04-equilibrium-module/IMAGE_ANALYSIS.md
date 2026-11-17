# Análisis Visual de Imágenes — Módulo 4: Equilibrium Calculations

**Ubicación archivada:** [imagenes/](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/imagenes/) (38 archivos PNG: `Imagen pegada.png` a `Imagen pegada (38).png`).

## Capturas Clave Inspeccionadas Directamente

1. **`Imagen pegada.png` (Fig. 1 — Starting screen):**
   - Barra de menú clásica (`File, Edit, View, Insert, Delete, Calculate, Format, Units, Window, Help`) + barra de iconos superior (`From Elements (Normal Mode)`, `Open`, `Save`).
   - Ventana MDI izquierda con hoja `Species` tipo Excel: barra de fórmulas (`Active species:`, selector `H10`, cuadro de edición), encabezados de columna `A–H` y filas `1..19`.
   - Columnas visibles: `A: Species Formula`, `C: Activity Coefficient (f)`, `D: Amount Unit`, `E: Initial Amount`, `F: Initial Amount (%)`, `G: Add X-amount`, `H: Add Z-amount`.
   - Panel acoplado derecho `Parameters` con botones `Calculate`, `Show Chart`, `Cancel` y secciones colapsables (`System Parameters`, `System State`).

2. **`Imagen pegada (5).png` (Fig. 5 — Sort Species & NICO.gem9):**
   - Diálogo modal *Sort Species* con checkbox `Add N2(g) for calculation stability` y radio buttons: `Phases`, `Gas, Aqua, Pure`, `No sorting`, `Gas, Aqua, Liquids`.
   - En la hoja `Species`, fila de fase `Phase #2` muestra un desplegable inline en la columna `C` con las opciones `Mixture`, `Pure`, `Aqua`.
   - Se observa la columna `B: Feed Temperature (°C)` con valor `25.000`, y filas de fase en gris claro con totales calculados (`4.000`, `100.000%`).

3. **`Imagen pegada (6).png` (Fig. 6 — Panel Parameters):**
   - Detalle completo de `Parameters`:
     - `Operations`: `Calculate`, `Show Chart`, `Cancel` + barra de progreso.
     - `System Parameters`: `Chart header`, `Calculation Mode` (`Normal`), `Step Mode` (`Add Amount`), `Allow titrations`, `Allow constraints`, `User defined parameters` (`N,T,P`), `Number of independent variables` (`0 / 1 / 2`), grupo `Define state change for` (`X-amount`, `Temperature`, `Volume`, `Enthalpy`, `Pressure`, `Z-amount`).
     - `System State`: tabla alineada `Initial | Final | Unit | Steps | Log` para `X-amount`, `Z-amount`, `Temperature (°C)`, `Pressure (bar)`, `Volume (m3)`, `Enthalpy (MJ)`, `Discharge Level (kmol e-)`, más `Use as base volume` e `Initial Pressure`.
     - `Calculation Options`: `Infinite Phase`, `Mixing Entropy conversion for aqueous species`, `Criss-Cobble`.
     - `Phase Data`: `Phase Name`, `Solution Model`, `AC Steps`.

4. **`Imagen pegada (7).png` (Fig. 7 — Target Calculation mode, FES04_target.gem9):**
   - Se observa el panel inferior `Target Calculations` expandido: checkbox `Enabled`, `Equilibrium Property` (`Species amount`), `Target` (`FeSO4*4H2O`), `Target Value` (`0.25 kmol`), y checkbox `Use species activity coefficient as step parameter`.

5. **`Imagen pegada (16).png` (Fig. 18 & 19 — Select Species y Diagrama 3D):**
   - Ventana `Select Species` con filtro por elementos (`OR`/`AND`), fases (`All phases`, `GAS`) y lista de especies seleccionables.
   - Ventana `Chart` con superficie 3D (`Equilibrium Amount, kmol` vs `Amount (CO2(g)), kmol` y `Temperature, °C`), malla reticulada y barra de herramientas superior.

6. **`Imagen pegada (24).png` (Fig. 29 — Chart filtering & Auto Scale):**
   - Comparación antes/después al desmarcar `CO2(a)` en el panel derecho `Chart parameters` (tabla `Legend` con checkboxes y color, más filtro `Contains All` por elementos y fases): el eje Y se reescala automáticamente (`Auto Scale`).

7. **`Imagen pegada (26).png` (Fig. 31 — 3D diagram with legend):**
   - Superficie 3D completa del sistema `H-C-O-Ca` con leyenda inferior de 16 especies (`CO2(g)`, `H2O(g)`, `O2(g)`, `N2(g)`, `H2O`, `C2O4(-2a)`, ..., `CaCO3`, `Ca(OH)2`) con checkboxes de visibilidad.

8. **`Imagen pegada (30).png` (Fig. 37 & 38 — Adiabatic calculation & Warnings):**
   - En `FESO4_adiabatic.gem9`: `User defined parameters = N,H,P`, variable independiente `Enthalpy` (`0` a `8000 MJ`, `51` pasos).
   - Diálogo `Warnings` estilo Excel (`A..I`): columna `A` con checkboxes `Remove the species outside of the interval Tmin .. Tmax` (`All Species` + fila por especie), `Species`, `Selected Range °C (T1, T2)` en rojo cuando excede (`-273.15` a `19726.85`), `Available Range °C (T min, T max)`, `Reference`, `Reliability class`, `Raw material temperature`.

9. **`Imagen pegada (32).png` (Fig. 40 — Adiabatic chart):**
   - Curva de equilibrio adiabático hasta `~20000 °C` tras remover `H2O` y los hidratos `FeSO4*7H2O`, `FeSO4*4H2O`, `FeSO4*H2O`.

10. **`Imagen pegada (34).png` (Fig. 42 — Cell mode, LEADBATT.gem9):**
    - `Calculation Mode = Cell Mode`, `User defined parameters = N,T,P,Q`, variable independiente `Discharge level` (`0` a `6.25 kmol e-`, `21` pasos).

11. **`Imagen pegada (38).png` (Fig. 46 — Activity coefficients add-ins):**
    - Tabla de add-ins disponibles: `Al - Zn Mixture` (`1.0.0.0`, `v1.1`), `Ga - As Mixture` (`1.0.0.0`, `v1.1`), `Aqua` (activo por defecto).
