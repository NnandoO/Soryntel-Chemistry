# Especificación de Interfaz — Módulo 06: HSC Estimate

Basada en las capturas oficiales de **HSC Estimate** (`Fig. 1`–`Fig. 5`, `Cp Range Data`, `HSC Cp Data`) y la vista clásica **`H, S, Cp Estimate`** con gráficos de dispersión de base de datos.

## 1. Ventana Principal (`HSC Estimate`)
- **Cinta superior (Ribbon):**
  - `Main`: `File ▾` (`New`, `Load Fig. 1 Example`, `Load Fig. 2 C2H4(g)`, `Load Fig. 3 FeVO4`, `Load Classic Example`, `Exit`).
  - `Estimation Settings`: `Species Type` (`Not Specified | Inorganic | Organic`), `Organic Group`, `Functional Group`, `Show Type Column`, `Freeze Column A`, `Freeze Title Rows`.
  - `Units`: `°C Celsius` | `K Kelvin` | `n Moles` | `1kg Kilograms`.
  - `Tools`: `Graphics ▾` | `Browse HSC Database` | `Save to HSC Database ▾` (`Cp at 25°C`, `Cp Range`).
  - `Help`: `Help` | `About`.
- **Barra de Fórmulas y Estimación Rápida:**
  - Celda activa (`A4`, `A16`...), fórmula editable, y barra rápida `Species | Temperature (°C / K) | [Estimate]`.
- **Hoja `Sheet1` (`A`–`AB`):**
  - `A`: `Chemical Formula` | `B`: `Temp °C/K` | `C`: `Species Type` | `D`: `Selected set of oxidation numbers` (`Set 1..3`) | `E`: `MW (g/mol)`
  - `F–G`: `H (25 °C)` (`Estimate | Database`) | `H–I`: `S (25 °C)` (`Estimate | Database`) | `J–K`: `Cp` (`Estimate | Database`) | `L–M`: `ΔG (25 °C)` (`Estimate | Database`) (`N/A` en azul si no existe en `Maindb6`).
  - `N–AB`: bloques `Element Information` (`Elem`, `Amount`, `Charge`, `Weight-%`, `Atom-%`) por cada elemento.

## 2. Ventana `Graphics` (`Fig. 5` + Dispersión de Base de Datos)
- **Barra de herramientas superior:**
  - `Parity` | `Parity with individual points` | `Comparison` | `Differences` | `Reset Chart Positions` | `Properties` | `Close`.
  - Selector de alcance (`Scope`): `Sheet` | `Gas (Maindb6)` | `Aqueous (Maindb6)` | `Condensed (Maindb6)` | `All DB`.
- **Distribución:**
  - **Área izquierda (~75% ancho):** grilla `2×2` de 4 sub-gráficos enmarcados con título superior:
    1. `H - Enthalpy` (`H (25 °C) - kJ/mol`, eje X `Database`, eje Y `Estimate`)
    2. `S - Entropy` (`S (25 °C) - J/mol*K`, borde activo resaltado al seleccionarse)
    3. `Cp - Heat Capacity` (`Cp - J/mol*K`)
    4. `ΔG - Gibbs Energy` (`ΔG (25 °C) - kJ/mol`)
  - En modo `Parity`: diagonal verde `y = x` con **relación de aspecto 1:1** (`same min..max` en X e Y), puntos azules en `Sheet` o puntos rojos densos (`radius = 1.8`) en dispersión de base de datos, más caja resumen (`Species Count`, `Mean error`, `Median error`).
  - **Panel derecho (~240 px fijo):** `Chart Properties` → `Chart (Name)` + `Statistics` (`Species Count`, `Maximum Error`, `Maximum Error Species`, `Mean Error`, `Median Error`, `RMSE`).

## 3. Flujo `Cp Range Data` → `HSC Cp Data`
- Diálogo `Cp Range Data`: `Species`, `Tmin (K)`, `Tmax (K)`, `Steps` (`50`), `OK / Cancel`.
- Ventana `HSC Cp Data`: tabla izquierda (`Temperature K`, `Cp Estimate`, `Cp Fitting`, `Error`), panel superior derecho `Fitted` (`A, B, C, D`, `Fit Cp`, `Calculate Cp`, `Estimate Cp`, `Update Species`) y gráfico inferior derecho `Cp Heat Capacity` (`Estimated` vs `Fitted`).
