# Análisis del Manual — Módulo 4: Equilibrium Calculations (Sección 13)

**Fuente:** [13-equilibrium-module.md](file:///home/nando/Documentos/Docs%20HSC/13-equilibrium-module/13-equilibrium-module.md) (57 páginas, 1501 líneas; autores: Petri Kobylin, Lena Furta, Danil Vilaev).

## 13.1 Introducción y Modos de Inicio
- **Objetivo:** Calcular la composición de equilibrio multicomponente y multifase mediante minimización de Energía Libre de Gibbs ($G$) dadas las cantidades iniciales de reactivos y condiciones del sistema.
- **Tres modos de inicio:**
  1. **From Elements** (`Normal Mode` o `Cell Mode`): selecciona elementos químicos, busca especies en las bases de datos (`Main` y `Own`), permite incluir iones/especies orgánicas o filtrar por fases, y ordena las fases mediante el diálogo *Sort Species* (`Phases`, `Gas, Aqua, Pure`, `No sorting`, `Gas, Aqua, Liquids`, además de la opción `Add N2(g) for calculation stability`).
  2. **Empty file:** crea una hoja en blanco para ingresar especies manualmente (p. ej. copiando desde Excel o escribiendo fórmulas con sufijo `(l)` o `(s)` para distinguir líquido/sólido).
  3. **Open:** abre archivos `.gem9` (HSC 9/10), `.GEM`, `.IGI` o `.ICE` (celdas).

## 13.2–13.3 Hoja `Species` (Definición de Fases, Especies y Coeficientes de Actividad)
- **Columnas principales de la hoja `Species`:**
  - `A`: `Species Formula` (con filas separadoras de fase como `Phase #1`, `GAS`, `AQUA`, `SOLID`).
  - `B`: `Feed Temperature (°C)` (para cálculos de entalpía/adiabáticos).
  - `C`: `Activity Coefficient (f)` en filas de especie; en filas de fase define el modelo de solución (`Mixture`, `Pure`, `Aqua`, `Ga-As Mixture`, `Al-Zn Mixture`).
  - `D`: `Amount Unit` (`kmol`, `kg`, `Nm3`, `g/l`, `mol/l`).
  - `E`: `Initial Amount` (la fila de fase muestra el `SUM(...)` en esa unidad).
  - `F`: `Initial Amount %` (porcentaje dentro de la fase).
  - `G` / `H`: `Add X-amount` / `Add Z-amount` (para barridos 1D o 2D).
- **Reglas estructurales de fases:**
  - La fase gaseosa debe colocarse como la **primera fase** si existe.
  - La fase acuosa (`Aqua`) requiere la presencia de `H2O` (solvente); la fracción molar estándar se convierte con $55.509 / x_{\text{H}_2\text{O}}$ si está activa la conversión de entropía de mezcla.
  - Una fase marcada como `Pure` representa sustancias puras invariantes (actividad unitaria cuando está presente).
- **Definición de actividad y fórmulas en Columna C:**
  - $a_i = p_i(\text{solución}) / p_i(\text{puro})$ y $f_i = a_i / x_i$ (Ecs. 1–2).
  - La Columna C admite fórmulas dinámicas dependientes de temperatura `T` (en K) y fracciones molares `X_row` (p. ej. `Ln(f) = 8495/T - 2.653`, `Ln(f) = 0.69 + 56.8*X_24 + 5.45*X_25`, `Ln(f) = -1.21*X_7^2 - 2.44*X_8^2`).

## 13.4 Panel `Parameters` y Modos de Cálculo
- **Operations:** `Calculate`, `Show Chart`, `Cancel`, barra de progreso.
- **System Parameters:**
  - `Chart header`: título de los gráficos generados.
  - `Calculation Mode`: `Normal` vs `Cell Mode`.
  - `Step Mode`: `Add Amount` (adición acumulativa) vs `Final Amount` (interpolación lineal/log entre inicial y final).
  - Checkboxes: `Allow titrations` (habilita unidades `g/l` o `mol/l`), `Allow constraints`.
  - `User defined parameters`: combinaciones de estado termodinámico (`N,T,P`, `N,H,P` adiabático, `N,P,V` volumen constante, `N,T,P,Q` en modo celda).
  - `Number of independent variables`: `0` (punto único), `1` (curva 2D), `2` (superficie 3D).
  - `Define state change for`: `X-amount`, `Temperature`, `Volume`, `Enthalpy`, `Pressure`, `Z-amount`, `Discharge level`.
- **System State:** rangos `Initial`, `Final`, `Steps` y escala `Log`; opción `Use as base volume` e `Initial Pressure`.
- **Calculation Options:**
  - `Infinite Phase` (atmósfera abierta / fase reservorio de potencial químico fijo).
  - `Mixing Entropy conversion for aqueous species` ($55.509 / x_{\text{H}_2\text{O}}$).
  - `Criss-Cobble` (extrapolación de $C_p$ acuoso a altas temperaturas).
- **Phase Data:** selección de `Solution Model` (`Ideal Mixture`, `Pure Substance`, `Aqua`, `Ga-As Mixture`, `Al-Zn Mixture`) y `AC Steps` (iteraciones externas de coeficientes de actividad).
- **Target Calculations:** búsqueda de la variable independiente para alcanzar una propiedad objetivo (p. ej. `FeSO4*4H2O = 0.25 kmol`).
- **Advanced Calculation Options (13.4.5):** `Fixed Activity mode`, `Remove %` (eliminación de fase/corriente entre pasos), `Reactor (Standard / Sheets View)`.

## 13.5–13.7 Gráficos 2D/3D, Tablas de Resultados y Advertencias de Extrapolación
- **Diagrams:** selección de ejes (`Equilibrium Amount`, `Equilibrium Compositions`, `pH`, `Cell voltage`, `Discharge level`, `User Formulas`), filtrado interactivo en leyenda con `Auto Scale`, visualización 3D con malla coloreada por niveles.
- **Equilibrium Diagram Tables:** genera la hoja `Results` con la tabla completa paso a paso (`Show...Table`).
- **Restriction of Cp Extrapolation (`Warnings`):** en modos `Adiabatic` (`N,H,P`) y `Constant Volume` (`N,P,V`), el buscador barre `[0, 20000 K]`. El diálogo `Warnings` alerta cuando `T1..T2` excede `[Tmin, Tmax]` de la base de datos y permite desactivar especies inestables a alta temperatura (ej. `H2O`, `FeSO4*7H2O`, `FeSO4*4H2O`, `FeSO4*H2O` en `FESO4_adiabatic.gem9` o `NA2SO4.gem9`).

## 13.8 Errores Numéricos y Consejos Prácticos
- No mezclar sustancias puras inertes dentro de fases mezcla si deben ser fases puras separadas.
- Añadir una pequeña cantidad de gas inerte (`Ar(g)` o `N2(g)`, p. ej. `1e-10` o `0.1 mol`) en sistemas totalmente condensados para estabilidad numérica.
- Evitar trampas estequiométricas exactas (p. ej. añadir `+1e-5 mol Cl2(g)` en `Na+Cl`).
- Caso `H2O(g)-H2O` entre `0–200 °C` y reducción de `Na2SO4` con carbón a $\ge 900\,^\circ\text{C}$.

## 13.9 Equilibrio en Celdas Electroquímicas (`Cell Equilibrium`)
- Ejemplo `LEADBATT.gem9` / formato `.ICE`.
- Cada fase define tipo (`gas`, `liquid`, `solid`, `metal`), electrodo (`anode`, `cathode`) y capacitancia en Farads ($F$; nota: en HSC7 se ingresaba como $F^{-1}$).
- `Discharge Equation` generada desde números de oxidación y variable `Discharge level` en `kmol e-`.
- Gráfica `Cell voltage (V)` vs `Discharge level (kmol e-)`.

## 13.10–13.11 Add-ins de Coeficientes de Actividad y Apéndices A–C
- **Interfaz .NET `IActivityCoefficientModel2(species, amounts, T_K, P_bar)`** para plugins externos.
- **Referencias:** White, Johnson & Dantzig (1958); Lampinen & Vuorisalo (1991, 1992).
- **Apéndice A:** Modelo binario de Redlich–Kister $G^{Ex} = x_i x_j \sum_{n=0}^{m} k_n (x_i - x_j)^n$ con $R = 8.31431\;\text{J/(mol}\cdot\text{K)}$.
- **Apéndice B:** Parámetros líquidos `Ga–As` ($k_0 = -25503.6 - 4.3109 T$, $k_1 = 5174.7$).
- **Apéndice C:** Parámetros `Al–Zn` para fases líquida, FCC y HCP.
