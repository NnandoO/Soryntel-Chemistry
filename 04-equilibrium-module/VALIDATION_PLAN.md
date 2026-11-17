# Plan de Validación — Módulo 4: Equilibrium Calculations

Todos los casos de prueba se ejecutarán con `cargo test -p soryntel-thermo` utilizando aserciones numéricas con `approx::assert_relative_eq!` y `approx::assert_abs_diff_eq!` contra las salidas reales de HSC6 (`.OGI`) y ejemplos del manual.

## Casos de Prueba Golden

### Caso 1: Descomposición térmica de Pirita (`FES2.IGI` / `FES2.OGI`)
- **Sistema:** `N2(g) = 0.1 mol`, `FeS2 = 1.0 mol`, 11 especies gaseosas (`Fe(g), FeS(g), S..S8(g)`), 2 fases sólidas (`FeS/Fe0.877S` mezcla y `FeS2` pura).
- **Condición:** $T = 273.15\;\text{K}$, $P = 1\;\text{bar}$.
- **Valores esperados (`.OGI`):**
  - `Gmin# = -83.68854385952` (`rel_tol = 1e-6`)
  - `Reaction enthalpy = -1.595 kJ`, `Reaction entropy = -5.585 J/K` (`abs_tol = 1e-2`)
  - `N2(g) = 1.0000e-1 mol`, `FeS2 = 1.0000 mol`, `S2(g) = 3.3694e-39 mol` (`rel_tol = 1e-3`).

### Caso 2: Precipitación acuosa de `CaCO3` con `CO2(g)` (`CACO3.IGI` / `CACO3.OGI`)
- **Sistema:** Gas (`N2, H2O, O2, CO2`), Fase acuosa (`H2O = 55.5 mol`, `Ca(+2a) = 1.0 mol`, `OH(-a) = 2.0 mol` + 7 iones/especies), Sólidos puros `CaCO3`, `Ca(OH)2`.
- **Condición (último paso `.OGI`, $P = 1\;\text{bar}$, $298.15\;\text{K}$, `CO2(g)_in = 3.001 mol`):**
  - `Volume = 7.565e-2 m3`, `Reaction enthalpy = -91.73 kJ`, `Reaction entropy = 135.4 J/K` (`rel_tol = 1e-3`)
  - `CO2(g) = 1.9945 mol`, `H2O(g) = 9.7855e-2 mol`
  - `H2O(aq) = 56.396 mol`, `Ca(+2a) = 6.4955e-3 mol`, `HCO3(-a) = 1.2990e-2 mol`, `H(+a) = 7.7515e-7 mol`
  - `CaCO3(s) = 0.99350 mol`, `Ca(OH)2(s) = 0.0 mol` (`rel_tol = 5e-3`).

### Caso 3: Síntesis de Amoniaco (`AMMONIA.IGI`, $480\,^\circ\text{C} = 753.15\;\text{K}$)
- **Validación:** Conservación atómica $\max |An - b| < 10^{-11}$ y verificación de presión de equilibrio de `NH3(g)` frente a constante $K_p$ del Módulo 1 (`Reaction Equations`).

### Caso 4: Mezclas No Ideales (`GAAS.IGI` y `ALPL.IGI` — Apéndices B y C)
- **Validación:** Coeficientes de actividad $\gamma_{\text{Ga}}, \gamma_{\text{As}}$ mediante Redlich–Kister y convergencia del lazo `AC Steps` (`abs_tol = 1e-6` en $\ln \gamma_i$).

### Caso 5: Modo Adiabático (`FESO4_adiabatic.gem9` / `N,H,P`)
- **Validación:** Dado $H_{\text{target}} \in [0, 8000\;\text{MJ}]$, verificar $|H_{\text{eq}}(T^*) - H_{\text{target}}| < 10^{-4}\;\text{MJ}$ y detección de especies fuera de `[Tmin, Tmax]` en `Warnings`.

### Caso 6: Modo Celda Electroquímica (`LEADBATT.gem9` / `N,T,P,Q`)
- **Validación:** Barrido `Discharge level` de `0` a `6.25 kmol e-`, conservación de carga y cálculo monotónico de voltaje de celda $E_{\text{cell}}(Q)$.
