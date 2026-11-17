# VALIDATION_PLAN — Numerical Verification & Test Cases

## 1. Estrategia de Validación Numérica con `approx`

Para certificar que el motor independiente en **Rust** de **Soryntel Chemistry** produce resultados químicamente exactos e indistinguibles de las compilaciones analíticas de referencia:
- Se implementarán pruebas de regresión unitarias (`cargo test`) utilizando la macro `assert_relative_eq!` del crate `approx`.
- Criterios de tolerancia científica:
  - Funciones termodinámicas continuas ($\Delta H$, $\Delta S$, $\Delta G$): tolerancia relativa $\epsilon \le 10^{-4}$ ($0.01\%$).
  - Potencial de celda electroquímico ($E^\circ$): tolerancia absoluta $\delta \le 10^{-3}\,\text{V}$ ($1\,\text{mV}$).
  - Logaritmo decimal de equilibrio ($\log_{10} K$): tolerancia absoluta $\delta \le 10^{-3}$.

---

## 2. Casos de Prueba de Referencia Documentados

### Caso de Prueba 1: Sustancia Pura — Alúmina ($\text{Al}_2\text{O}_3$) y Rutilo ($\text{TiO}_2$)
- **Objetivo:** Verificar la integración analítica continua de $C_p(T)$ con múltiples intervalos y transiciones de fase, y la conversión exacta `1 cal = 4.184 J`.
- **Puntos de Control (`Al2O3`, Fig. 1):**
  - $T = 0.000\,^\circ\text{C}$ ($273.15\,\text{K}$): $C_p = 72.164\,\text{J}/(\text{mol}\cdot\text{K})$, $H = -1677.558\,\text{kJ}/\text{mol}$, $S = 44.407\,\text{J}/(\text{mol}\cdot\text{K})$, $G = -1689.688\,\text{kJ}/\text{mol}$.
  - Masa molar: $101.961\,\text{g}/\text{mol}$, volumen molar condensado: $25.715\,\text{ml}/\text{mol}$ ($\rho = 3.965\,\text{g}/\text{cm}^3$).
- **Puntos de Control (`TiO2`, Fig. 10 y `Maindb6.HSC`):**
  - A $298.15\,\text{K}$: $H_f^\circ = -944.747\,\text{kJ}/\text{mol}$, $S^\circ = 50.292\,\text{J}/(\text{mol}\cdot\text{K})$, $C_p = 54.949\,\text{J}/(\text{mol}\cdot\text{K})$, $\Delta G^\circ = -889.417\,\text{kJ}/\text{mol}$.

### Caso de Prueba 2: Proceso Mond para Purificación de Níquel
- **Ecuación química:**
  $$\text{Ni}(s) + 4\text{CO}(g) = \text{Ni(CO)}_4(g)$$
- **Objetivo:** Verificar equilibrio exotérmico en fase gas con inversión espontánea de signo en $\Delta G^\circ$ y cruce de $K = 1$ en el rango $70 - 160\,^\circ\text{C}$ (Figs. 3 y 4).
- **Puntos de Control Numérico:**
  - $T = 70\,^\circ\text{C}$ ($343.15\,\text{K}$): $K \approx 1.1 \times 10^3$ ($\Delta G_r^\circ < 0$).
  - $T \approx 119.3\,^\circ\text{C}$: $\Delta G_r^\circ \approx 0 \implies K = 1.0$.
  - $T = 160\,^\circ\text{C}$ ($433.15\,\text{K}$): $K \approx 1.0 \times 10^{-2}$ ($\Delta G_r^\circ > 0$).

### Caso de Prueba 3: Presión de Vapor y Fusión/Ebullición de Magnesio
- **Ecuación química:**
  $$\text{Mg}(s, l) = \text{Mg}(g)$$
- **Objetivo:** Validar el salto de calor latente de fusión en $T_m = 923.00\,\text{K}$ ($649.85\,^\circ\text{C}$, $\Delta H_{\text{fus}} = 8.477\,\text{kJ}/\text{mol}$) y la identificación del punto de ebullición normal donde $K = p_{\text{Mg}(g)} = 1.0\,\text{bar}$ (Fig. 5).
- **Puntos de Control Numérico:**
  - $T = 500.000\,^\circ\text{C}$: $\Delta H = 143.884\,\text{kJ}$, $\Delta S = 110.035\,\text{J}/\text{K}$, $\Delta G = 58.811\,\text{kJ}$, $K = 1.063 \times 10^{-4}$, $\log_{10} K = -3.974$.
  - $T = 600.000\,^\circ\text{C}$: $\Delta H = 142.876\,\text{kJ}$, $\Delta S = 108.810\,\text{J}/\text{K}$, $\Delta G = 47.869\,\text{kJ}$, $K = 1.368 \times 10^{-3}$.
  - $T = 650.000\,^\circ\text{C}$ (post-fusión): $\Delta H = 133.839\,\text{kJ}$, $\Delta S = 99.002\,\text{J}/\text{K}$, $\Delta G = 42.445\,\text{kJ}$, $K = 3.964 \times 10^{-3}$.
  - $T = 1050.000\,^\circ\text{C}$: $K = 0.7030$, $\log_{10} K = -0.153$.
  - $T = 1100.000\,^\circ\text{C}$: $K = 1.074$, $\log_{10} K = 0.031$.

### Caso de Prueba 4: Semirreacción Redox y Potencial Electroquímico vs SHE
- **Ecuación química:**
  $$2\text{FeS}_2 + 19\text{H}_2\text{O} = \text{Fe}_2\text{O}_3 + 4\text{SO}_4^{2-} + 38\text{H}^+ + 30e^-$$
- **Objetivo:** Verificar el algoritmo de balance matricial estequiométrico con conservación de carga y el cálculo del potencial estándar $E^\circ(T)$ según la convención de la IUPAC (Figs. 7, 8 y 9).
- **Puntos de Control Numérico:**
  - Coeficientes resueltos (`Smallest Coefficient = 1`): $\nu = [-2, -19, +1, +4, +38, +30]$.
  - $T = 0.000\,^\circ\text{C}$: $\Delta H = 1497.322\,\text{kJ}$, $\Delta S = 1318.489\,\text{J}/\text{K}$, $\Delta G = 1137.177\,\text{kJ}$, $K = 3.300 \times 10^{-218}$, $E^\circ = -0.393\,\text{V}$ vs SHE.
  - $T = 25.000\,^\circ\text{C}$: $\Delta H = 1322.639\,\text{kJ}$, $\Delta S = 687.984\,\text{J}/\text{K}$, $\Delta G = 1117.516\,\text{kJ}$, $K = 1.582 \times 10^{-196}$, $E^\circ = -0.386\,\text{V}$ vs SHE.
  - $T = 175.000\,^\circ\text{C}$: $\Delta H = 1011.966\,\text{kJ}$, $\Delta S = -147.532\,\text{J}/\text{K}$, $\Delta G = 1078.082\,\text{kJ}$, $K = 2.149 \times 10^{-126}$, $E^\circ = -0.373\,\text{V}$ vs SHE.

### Caso de Prueba 4: Extrapolación de Criss-Cobble para Iones en Solución
- **Ecuación química:**
  $$\text{Al}^{3+}(aq) + 3\text{OH}^-(aq) = \text{Al(OH)}_3(s)$$
- **Objetivo:** Evaluar la continuidad matemática de $\bar{C}_p$ y entalpía extrapolada desde $25\,^\circ\text{C}$ hasta $300\,^\circ\text{C}$ sin divergencias ni singularidades numéricas.
- **Puntos de Control:**
  - $T = 25\,^\circ\text{C}$ a $300\,^\circ\text{C}$ con paso de $25\,^\circ\text{C}$.
  - Comprobación de que la corrección de entropía absoluta $S^\circ_{\text{abs}}(\text{H}^+) = -5.0\,\text{cal}/(\text{mol}\cdot\text{K})$ se aplica consistentemente en la escala de iones.
