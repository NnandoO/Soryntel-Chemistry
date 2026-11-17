# Plan de Validación Numérica (`VALIDATION_PLAN.md`) — Módulo 3

Todas las pruebas unitarias y de integración se implementarán en `soryntel-thermo` usando el crate `approx` (`assert_relative_eq!` / `assert_abs_diff_eq!`) contra resultados documentados en [`12-heat-loss.md`](file:///home/nando/Documentos/Docs%20HSC/12-heat-loss/12-heat-loss.md) y capturas archivadas en [`imagenes/`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/imagenes/).

---

## 1. Casos de Prueba de Referencia (`Golden Tests`)

### Caso 1: `SimpleCase` (Verificación analítica exacta de red en serie)
- **Fuente:** `12-heat-loss.md`, Sección 12.2 (`SimpleCase.htr8`).
- **Configuración:** Pared plana $A = 1.5\text{ m}^2$, temperaturas extremas $700\,^\circ\text{C} \to 20\,^\circ\text{C}$ a través de 3 elementos en serie con temperaturas de interfaz esperadas $418\,^\circ\text{C}$ y $50.63\,^\circ\text{C}$.
- **Resultado esperado:**
  - $q = 735.0\text{ W}$ (tolerancia relativa `1e-4`).
  - Temperaturas intermedias: $T_1 \approx 418.0\,^\circ\text{C}$, $T_2 \approx 50.63\,^\circ\text{C}$ (tolerancia absoluta `0.1 °C`).
  - Modo `Temperature profile` inverso (fijando $T_0 = 700\,^\circ\text{C}$ y $Q = 735\text{ W}$) debe recuperar $T_N = 20.0\,^\circ\text{C}$ dentro de `1e-3 °C`.

### Caso 2: `Smelting1` (Pared de horno refractario con $h_c$ fijos y $k(T)$ variable)
- **Fuente:** `fig01_smelting1.jpg` (`Smelting1.htr8`, hoja `Smelting 1200°C`).
- **Configuración:**
  - Geometría: `Wall`, `Width = 1.00 m`, `Length = 1.00 m` ($A = 1.000\text{ m}^2$).
  - Col 1 (`Surface`, `Molten metal`): $T_L = 1200\,^\circ\text{C}$, $h_c = 144.000\text{ W/(m}^2\text{K)}$ fijo, $h_r = 0$.
  - Col 2 (`Solid Layer`, `REXAL 2S EXTRA`): espesor $1.000\text{ m}$, `Calculation Grid = 10`, $k_{\text{avg}} \approx 3.942\text{ W/(m}\cdot\text{K)}$.
  - Col 3 (`Surface`, `Water, Copper elements cooling`): $T_R = 20\,^\circ\text{C}$, $h_c = 2400.000\text{ W/(m}^2\text{K)}$ fijo, $h_r = 0$.
- **Resultados esperados (`fig01`):**
  - `Heat Flow`: $4.251\text{ kW}$ (`abs_diff <= 0.01 kW`).
  - Temperaturas de interfaz: $1171\,^\circ\text{C}$ y $22\,^\circ\text{C}$ (`abs_diff <= 0.5 °C`).
  - Resistencias: $R_1 = 6.944\,^\circ\text{C/kW}$, $R_2 = 270.234\,^\circ\text{C/kW}$, $R_3 = 0.417\,^\circ\text{C/kW}$.

### Caso 3: `Radiator` + `Target Solver`
- **Fuente:** `fig02_radiator.jpg` y `fig11_target_dialog.jpg` (`Radiator.htr8`, hoja `Radiator 40°C`).
- **Configuración:**
  - `Wall` $2.00\text{ m} \times 0.50\text{ m}$ ($A = 1.000\text{ m}^2$), `Water H2O (liquid)` ($40\,^\circ\text{C}$) $\to$ `Carbon steel: (0.2% C)` ($0.003\text{ m}$, $k = 50.000$) $\to$ `Air (gas)` ($20\,^\circ\text{C}$) + `Paint, white (acrylic) (h)`.
- **Resultados esperados:**
  - `Heat Flow` base: $0.174\text{ kW}$ (`abs_diff <= 0.005 kW`), $h_{c,3} \approx 3.322$, $h_{r,3} \approx 5.674\text{ W/(m}^2\text{K)}$.
  - **`Target Solver`:** Objetivo $Q = 0.50\text{ kW}$ variando `Width` $\implies \text{Width} \approx 5.747\text{ m}$ (`abs_diff <= 0.02 m`).

### Caso 4: `Hotwatertank` (Geometría cilíndrica multi-hoja + espesor óptimo)
- **Fuente:** `fig03_hotwatertank_profile.jpg`, `fig04_main_layout_annotated.jpg`, `fig14_optimum_insulation.jpg` (`Hotwatertank.htr8`).
- **Configuración:**
  - `Cylinder` ($H = 4.00\text{ m}$, $D = 3.00\text{ m}$), 3 hojas (`Wall`, `Roof`, `Bottom`), acero inoxidable ($0.005\text{ m}$) + espuma de uretano ($0.050\text{ m}$, $k = 0.060$) + `Aluminium (h)`, $50\,^\circ\text{C} \to 20\,^\circ\text{C}$.
- **Resultados esperados:**
  - Áreas crecientes en hoja `Wall`: $37.699 \to 37.699 \to 37.919 \to 40.156\text{ m}^2$ (`abs_diff <= 1e-3 m²`).
  - `Total Heat Flow` (3 hojas): $1.378\text{ kW}$ (`abs_diff <= 0.02 kW`).
  - Barrido económico (`Diagram`): mínimo de costo total anualizado en espesor de aislamiento $\approx 0.33\text{ m}$ ($33\text{ cm}$).

### Caso 5: Radiación de Mezcla de Gases (`OilFurnace` / `GasRad`)
- **Fuente:** `12-heat-loss.md` Sec. 12.2 (`OilFurnace.htr8`) y `fig09_gas_radiation.jpg`.
- **Configuración:** Mezcla con $\text{H}_2\text{O(g)} = 11.08\%$, $\text{CO}_2\text{(g)} = 11.69\%$, $\text{SO}_2\text{(g)} = 0.05\%$ (resto $\text{N}_2$).
- **Resultados esperados:**
  - Verificación de $\varepsilon_g \in (0, 1)$, $\alpha_g \in (0, 1)$, monotonía con $L$ y $p$, y concordancia de `Gas Emissivity` / `Heat Flux` con la tabla espectral EWBM de `fig09` (tolerancia relativa `2%` frente a la implementación de referencia de Lallemant & Weber 1996).

---

## 2. Pruebas de Propiedades y Casos Borde (`Unit Tests`)

- **Continuidad y límites de correlaciones:**
  - `Enclosure` horizontal alrededor de $Ra_L = 1708$ ($Nu = 1$ para $Ra \le 1708$).
  - `Flat plate` forzada alrededor de $Re_L = 5\times 10^5$.
  - `Sphere` con $Ra_D \to 0$ debe tender exactamente al límite conductivo $Nu_D \to 2$.
- **Validaciones de entrada:**
  - Rechazar $\varepsilon_s < 0$ o $\varepsilon_s > 1$.
  - Rechazar espesor $\Delta x \le 0$ en `SolidLayer`.
  - Rechazar modos mal especificados ($\ne 2$ temperaturas fijas en `HeatFlow`; $\ne 1$ temperatura fija en `TemperatureProfile`).
- **Invarianza estacionaria:**
  - En cualquier hoja convergida, $\max_{i,j} |Q_i - Q_j| / Q_{\text{sheet}} < 10^{-7}$ en todas las columnas.
