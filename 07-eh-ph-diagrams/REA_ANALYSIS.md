# Análisis REA — Módulo 07: Eh-pH Diagrams (EpH)

## 1. Hallazgos en HSC6 (`/home/nando/Descargas/HSC6/Help/17 Eh - pH - Diagrams.txt`)
- El módulo `EpH` de HSC está basado en **STABCAL** (*Stability Calculations for Aqueous Systems*, H.H. Haung, Montana Tech.) y en el atlas clásico de **Pourbaix (1966)**.
- Flujo original de HSC6 en 3 pasos + modo combinado:
  1. **System Specifications:** selección de elemento principal (`Main Element`), elementos secundarios (sistema + $\text{H}_2\text{O}$), hasta 4 temperaturas y búsqueda automática de especies en `Maindb6`.
  2. **Eh-pH Diagram Menu:** hoja `Species` con $\Delta G^\circ(T_1..T_4)$, edición/inserción/eliminación de especies, selección de temperatura, `Min/Max pH`, `Min/Max Eh`, `Molality (mol/kg H2O)`, `Pressure (bar ≥ P_sat(T))`, y opción `Show Predominance Areas of Ions`.
  3. **Diagram Form:** gráfico interactivo `(pH, Eh)` con líneas de equilibrio sólido/acuoso (horizontales, verticales, diagonales), líneas punteadas de estabilidad del agua y etiquetas centradas por región.
  4. **Combine Menu:** superposición de hasta **4 Data Sets** variando molalidad, temperatura, presión o elemento principal, con tabla de resultados (`DS1..DS4`, `Label`, `Lines`).

## 2. Qué existe actualmente en `soryntel`
- **`soryntel-thermo`:**
  - `evaluate_species_si` con `criss_cobble_eval` ya implementado para extrapolar iones acuosos (`Phase::AqueousIon`, `Phase::AqueousNeutral`) hasta `300 °C`.
  - `h2o_saturation_pressure_bar(t_k)` en `balance.rs` para validar $P_{\text{total}} \ge P_{\text{sat}}(T)$ y calcular propiedades del agua.
  - `parse_term` en `soryntel-solver` que ya extrae carga eléctrica `charge` (ej. `Cu(+2a)` → `+2`, `SO4(-2a)` → `-2`, `OH(-a)` → `-1`).
- **`soryntel-db`:**
  - `Maindb6.HSC` contiene miles de especies acuosas, sólidas y gaseosas listas para filtrarse por conjunto de elementos `{Main, Secundarios, H, O}`.

## 3. Brechas Técnicas Identificadas (Gaps)
1. **Motor de envolvente lineal 2D (`EpH Solver`):** falta el cálculo de potenciales químicos normalizados por átomo del elemento principal en el plano `(pH, Eh)` y la extracción exacta de polígonos/segmentos de frontera entre especies predominantes.
2. **Acoplamiento con especies secundarias:** falta resolver la especiación de fondo de elementos secundarios (ej. $\text{S}$ entre $\text{SO}_4^{2-}$, $\text{HSO}_4^-$, $\text{H}_2\text{S}(a)$, $\text{HS}^-$, $\text{S}^{2-}$) para el diagrama del elemento principal (`Cu-S-H2O`, `Fe-S-H2O`).
3. **Límites de agua y escalas de potencial:** falta implementar $E_{h,\text{O}_2}(T, P, \text{pH})$, $E_{h,\text{H}_2}(T, P, \text{pH})$ y escala `SHE` (u otros electrodos de referencia).
4. **Modo `Combine` (hasta 4 Data Sets)** y **hoja `Species` editable** en `soryntel-desktop`.
