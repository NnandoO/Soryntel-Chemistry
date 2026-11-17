# Plan de Validación (`VALIDATION_PLAN`) — Módulo 2: Heat and Material Balance

Todos los casos de prueba se ejecutarán mediante `cargo test --workspace` usando la macro `assert_relative_eq!` / `assert_abs_diff_eq!` de **`approx`**, contrastando contra los valores exactos observados en los archivos `.OHE` de `/home/nando/Descargas/HSC6/Balance/` y las capturas del manual.

---

## 1. Casos de Prueba de Referencia

### Caso 1: Combustión Adiabática de Butano (`BUTANE.OHE`)
- **Fuente:** `/home/nando/Descargas/HSC6/Balance/BUTANE.OHE`.
- **Entradas (`25.0 °C`):**
  - `C4H10(NBAg)`: `1.000 kmol` (`Total H = -126.148 MJ`, `Heat Content = 0.000 MJ`)
  - `O2(g)`: `6.500 kmol` (`Total H = 0.000 MJ`)
  - `N2(g)`: `24.500 kmol` (`Total H = 0.000 MJ`)
- **Salidas (`2000.0 °C`):**
  - `CO2(g)`: `4.000 kmol` (`Heat Content = 432.430 MJ`, `Total H = -1141.591 MJ`)
  - `H2O(g)`: `5.000 kmol` (`Heat Content = 435.321 MJ`, `Total H = -773.809 MJ`)
  - `N2(g)`: `24.500 kmol` (`Heat Content = 1617.973 MJ`, `Total H = 1617.973 MJ`)
- **Aserciones (`approx`):**
  - Balance elemental (`C = 4.0`, `H = 10.0`, `N = 49.0`, `O = 13.0`): `abs_diff_eq!(diff, 0.0, epsilon = 1e-6)`.
  - `Total H` entrada: `abs_diff_eq!(h_in, -126.148, epsilon = 0.05)`.
  - `Total H` salida a `2000 °C`: `abs_diff_eq!(h_out, -297.427, epsilon = 0.1)`.
  - **`Temp. Balance`:** `abs_diff_eq!(t_adiab_c, 2121.948, epsilon = 0.5)`.

---

### Caso 2: Evaporación de Agua con Corrección de Presión (`H2O.OHE`)
- **Fuente:** `/home/nando/Descargas/HSC6/Balance/H2O.OHE`.
- **Entradas:** `H2O` `0.060 kmol` a `25.0 °C` + `EXTRA HEAT = 2.793 MJ` (`Total H = -14.357 MJ`).
- **Salidas:** `H2O(g)` `0.060 kmol` a `100.0 °C`, `P = 1.01325 bar` (`Heat Content = 0.152 MJ` sobre `H2O(g)` a `25 °C` o `Total H = -14.357 MJ`).
- **Aserciones (`approx`):**
  - Balance elemental (`H = 0.120 kmol`, `O = 0.060 kmol`): `epsilon = 1e-6`.
  - Balance térmico neto: `abs_diff_eq!(delta_h, 0.0, epsilon = 0.01)`.
  - `Temp. Balance`: `abs_diff_eq!(t_adiab_c, 100.0, epsilon = 0.5)`.

---

### Caso 3: Fusión de Concentrado de Cobre (`CUSMELT.OHE`)
- **Fuente:** `/home/nando/Descargas/HSC6/Balance/CUSMELT.OHE`.
- **Entradas (`25.0 °C`):** `Cu2S` (`1.000 kmol`), `FeS` (`2.000 kmol`), `SiO2` (`1.000 kmol`), `O2(g)` (`4.000 kmol`). `Total H = -1195.369 MJ`.
- **Salidas:** `Cu` (`2.000 kmol` a `1100 °C`), `*2FeO*SiO2` (`1.000 kmol` a `1400 °C`), `SO2(g)` (`3.000 kmol` a `1100 °C`), `HEAT LOSSES = 500.000 MJ`. `Total H salida + pérdidas = -1265.612 MJ`.
- **Aserciones (`approx`):**
  - `Heat Content` entrada: `0.000 MJ`, salida: `604.729 MJ` (`epsilon = 0.2`).
  - Calor liberado: `abs_diff_eq!(delta_h, -70.243, epsilon = 0.2)`.
  - **`Temp. Balance`:** `abs_diff_eq!(t_adiab_c, 1398.243, epsilon = 1.0)`.

---

### Caso 4: Convertidor de Cobre — Fórmulas, Target y Exergía (`CUCONV1` / `CUCONV2`, Figs. 1–7)
- **Fuente:** Capturas `raw-001.png` (`CUCONV1.bal8`), `raw-005.png`, `raw-011.png`, `raw-015.png`, `raw-018.png`.
- **Validaciones:**
  1. Fórmula dependiente de balance elemental: `=0.938*(InCu-D13)/2` evalúa a `0.469 kmol` de `Cu2S(l)`.
  2. `Element Balance`: `Cu = 1.000`, `Fe = 0.540`, `N = 6.930`, diferencia `0.000`.
  3. `Total Balance` (`CUCONV1`): `Amount kmol = -0.807`, `Amount kg = 0.000`, `Amount Nm3 = -6.813`, `Heat Content = 194.699 MJ`, `Total H = -81.326 MJ`, `Exergy = -147.526 MJ` (tolerancia `< 1.0 MJ` por posibles diferencias menores de base entre HSC6 y HSC8).
  4. **`Target Dialog` acoplado (`CUCONV2`, Fig. 7):** resolver simultáneamente `SCRAP` (`kg/h`) y `Fe wt-%` para lograr `HEAT BAL = 0.00` y `Fe Amount = 0.540 kmol` (`Fe wt-% ≈ 20.91`).

---

### Caso 5: Diagrama Multi-Área con Retornos Circulares (`FSF_process.bal8`, Figs. 11–14)
- **Fuente:** Capturas `raw-027.png` a `raw-038.png`.
- **Validaciones:**
  1. 5 áreas de balance conectadas (`FSF`, `Converter I`, `Converter II`, `Boiler`, `ESP`) con referencias cruzadas entre hojas (`='OUT4'!D12`, `=InCa2*Analysis!L28/100`).
  2. Convergencia estable de las corrientes de retorno circulares (`Flue dust Boiler/ESP/Conv.1/Conv.2 return`) en menos de `50` iteraciones con tolerancia `< 1e-8`.
  3. Conservación de masa global por área: `Amount kg` en `BALANCE` igual a `0.000` (`epsilon = 1e-4`).

---

## 2. Criterios de Aceptación del Módulo 2

1. `cargo test --workspace` sin fallos ni advertencias bloqueantes.
2. Tolerancias `approx` cumplidas en todos los casos `.OHE` (`< 0.1%` en entalpías y `< 1.0 °C` en `Temp. Balance`).
3. Convergencia determinista en referencias circulares y en `Target Dialog` acotado dentro de `[Min, Max]`.
