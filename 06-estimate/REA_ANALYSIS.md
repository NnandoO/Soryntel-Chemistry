# Análisis REA — Módulo 06: HSC Estimate (H, S, Cp Estimation)

## 1. Hallazgos en HSC6 y Contexto del Módulo
- En la documentación de **HSC Chemistry 6** (`/home/nando/Descargas/HSC6/Help/`), el módulo independiente `H, S, C Estimates` (*Estimate*) aún no existía como ejecutable separado (fue incorporado en versiones posteriores de HSC 7/8/9), pero sus fundamentos y fuentes ya aparecen documentados en:
  - `31 Thermochemical Data Sources.txt` (línea 2): uso de métodos teóricos de estimación cuando no hay datos experimentales.
  - `28 HSC - Databases.txt` (línea 87): método de **Criss-Cobble** para extrapolar $C_p$ de iones acuosos hasta `300 °C`.
  - `33 Data References.txt` (líneas 382–384, 676–677): referencias **Hisham & Benson (1986, 1987, 1988)** para estimación empírica de entalpías de formación de sólidos inorgánicos y estimaciones `ORC / ORC 95` (*Outokumpu Research Oy*).

## 2. Qué existe actualmente en `soryntel`
- **`soryntel-thermo`:**
  - `KelleyCoefficients` (`A, B, C, D` en `J/(mol·K)`) e integración analítica de $H(T)$ y $S(T)$.
  - `criss_cobble_eval` en `soryntel-thermo/src/lib.rs` para extrapolación de especies acuosas.
- **`soryntel-solver`:**
  - `parse_term` para descomponer fórmulas químicas (ej. `Fe2SiO4`, `MgAl2O4`, `*2FeO*SiO2`) en estequiometría elemental.
- **`soryntel-db`:**
  - Acceso de solo lectura a `Maindb6.HSC` con miles de especies experimentales (elementos, óxidos simples, haluros, sulfuros, etc.) útiles como bloques base para reglas de aditividad.

## 3. Brechas Técnicas Identificadas (Gaps)
1. **Motor de estimación inorgánica (`s`, `l`, `g`):** falta implementar Neumann-Kopp (por elementos y por óxidos/componentes constituyentes), Latimer ($S^\circ_{298}$ iónico), y aditividad de óxidos/haluros para $\Delta_f H^\circ_{298}$ y $S^\circ_{298}$.
2. **Motor de estimación orgánica (`Benson` / `Joback-Reid`):** falta tabla de contribuciones de grupos funcionales para hidrocarburos y compuestos orgánicos.
3. **Ajustador de polinomio Kelley:** falta regresión por mínimos cuadrados sobre $C_p(T)$ estimado en $[298.15, T_{\max}]$ para entregar directamente coeficientes `A, B, C, D` compatibles con `Maindb6`.
4. **Comparación contra `Maindb6` y exportación a usuario:** falta interfaz dedicada para comparar la estimación vs dato experimental y generar un registro `ChemicalSpecies`.
