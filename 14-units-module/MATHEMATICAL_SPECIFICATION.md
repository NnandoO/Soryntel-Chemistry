# Especificación Matemática — Módulo 14: Units Module

## 1. Modelo General de Conversión en la Hoja `Units`

### 1.1 Prefijos Métricos SI ($P_p$)
Todo prefijo SI $p$ actúa como multiplicador decimal adimensional $10^{e_p}$ sobre la unidad seleccionada:

| Prefijo | Símbolo | Factor $10^{e_p}$ | Prefijo | Símbolo | Factor $10^{e_p}$ |
|---|---|---|---|---|---|
| `yotta` | `Y` | $10^{24}$ | `deci` | `d` | $10^{-1}$ |
| `zetta` | `Z` | $10^{21}$ | `centi` | `c` | $10^{-2}$ |
| `exa` | `E` | $10^{18}$ | `milli` | `m` | $10^{-3}$ |
| `peta` | `P` | $10^{15}$ | `micro` | `µ` / `u` | $10^{-6}$ |
| `tera` | `T` | $10^{12}$ | `nano` | `n` | $10^{-9}$ |
| `giga` | `G` | $10^{9}$ | `pico` | `p` | $10^{-12}$ |
| `mega` | `M` | $10^{6}$ | `femto` | `f` | $10^{-15}$ |
| `kilo` | `k` | $10^{3}$ | `atto` | `a` | $10^{-18}$ |
| `hecto` | `h` | $10^{2}$ | `zepto` | `z` | $10^{-21}$ |
| `deca` | `da` | $10^{1}$ | `yocto` | `y` | $10^{-24}$ |
| *(none)* | — | $10^{0} = 1$ | | | |

---

### 1.2 Conversión Lineal por Factor Base (`UNITS4.UNI`)
En `UNITS4.UNI`, cada unidad $u$ dentro de una magnitud lineal $Q$ almacena un coeficiente $c_u > 0$ definido como el número de unidades $u$ equivalentes a **1 unidad de referencia base** ($c_{\text{base}} = 1.0$):
$$1\text{ [unidad base]} = c_u \text{ [unidad } u\text{]} \iff 1\text{ [unidad } u\text{]} = \frac{1}{c_u}\text{ [unidad base]}$$

Cuando se combina una unidad $u$ con un prefijo SI cuyo factor es $f_{\text{pref}}(p) = 10^{e_p}$, el tamaño físico de `1 [p·u]` respecto a la unidad base es $\frac{f_{\text{pref}}(p)}{c_u}$. Por tanto, al convertir un valor $x_1$ desde `(u_1, p_1)` hacia `(u_2, p_2)`:
1. **Hacia la unidad de referencia base del grupo $Q$:**
   $$x_{\text{base}} = x_1 \cdot \frac{f_{\text{pref}}(p_1)}{c_{u_1}}$$
2. **Desde la unidad de referencia base hacia el destino:**
   $$x_2 = x_{\text{base}} \cdot \frac{c_{u_2}}{f_{\text{pref}}(p_2)} = x_1 \cdot \left(\frac{c_{u_2}}{c_{u_1}}\right) \cdot \left(\frac{f_{\text{pref}}(p_1)}{f_{\text{pref}}(p_2)}\right)$$

**Ejemplo verificado (`Energy, work`, $1\text{ [kJ]} \to \text{[kcal(th)]}$):**
- Base `joule (J)` tiene $c_{\text{J}} = 1.0$; `calorie (th)` tiene $c_{\text{cal(th)}} = 0.2390057$ ($= 1/4.184$).
- Con $p_1 = \text{k}$ ($10^3$), $p_2 = \text{k}$ ($10^3$), $x_1 = 100$:
  $$x_2 = 100 \cdot \frac{0.2390057}{1.0} \cdot \frac{10^3}{10^3} = 23.90057\text{ kcal(th)}$$

---

### 1.3 Regla de Prefijo para Unidades Derivadas con Prefijo Implícito (`kilogram`, `km²`, etc.)
- En la magnitud **Mass**, la unidad de referencia SI es `kilogram (kg)` ($c_{\text{kg}} = 1.0$). Para evitar la ambigüedad de `milli-kilogram` vs `gram` ($\text{g}$):
  - Se incluye explícitamente `gram (g)` con $c_{\text{g}} = 1000.0$ de forma que `mg` ($\text{milli} + \text{g} \implies 10^{-3}/10^3 = 10^{-6}\text{ kg}$), `µg`, `kg`, `t` (`tonne`, $c_{\text{t}} = 10^{-3}$) operen de forma natural sin prefijos compuestos inválidos.
  - Si un usuario selecciona `kilogram` junto a un prefijo no trivial en compatibilidad estricta `UNITS4.UNI`, el prefijo actúa como escalar directo $10^{e_p}$ sobre el símbolo mostrado con advertencia o deshabilitación de prefijos redundantes sobre `kilogram`.

---

### 1.4 Conversión Afín de Temperatura (`Temperature`)
Las escalas de temperatura no comparten el mismo cero absoluto, por lo que no se describen solo con $c_u$. El motor emplea **Kelvin (`K`)** como magnitud intermedia canónica ($T_K \ge 0$, pudiéndose aplicar también prefijos SI sobre `K` como `mK`, `kK`, y opcionalmente diferencias $\Delta T$ en modo diferencial):

- **A Kelvin ($T_K$):**
  - $\text{K}$: $T_K = T \cdot f_{\text{pref}}$
  - $^\circ\text{C}$: $T_K = T + 273.15$
  - $^\circ\text{F}$: $T_K = (T - 32) \cdot \frac{5}{9} + 273.15 = \frac{T + 459.67}{1.8}$
  - $^\circ\text{R}$ (Rankine): $T_K = \frac{T}{1.8}$

- **Desde Kelvin ($T_K$) hacia escala destino:**
  - $\text{K}$: $T = T_K / f_{\text{pref}}$
  - $^\circ\text{C}$: $T = T_K - 273.15$
  - $^\circ\text{F}$: $T = (T_K - 273.15) \cdot \frac{9}{5} + 32 = 1.8\,T_K - 459.67$
  - $^\circ\text{R}$: $T = 1.8\,T_K$

- **Modo diferencia térmica ($\Delta T$):** si el usuario activa la opción de incremento térmico, los desfases constantes ($+273.15$, $+32$) se omiten: $\Delta T_{^\circ\text{C}} = \Delta T_K$, $\Delta T_{^\circ\text{F}} = \Delta T_{^\circ\text{R}} = 1.8\,\Delta T_K$.

---

## 2. Ecuaciones de las Hojas de Datos Complementarias

### 2.1 Hoja `Mesh` (Tamaño de Partícula Tyler / ASTM E11)
Para valores discretos tabulados $(M_i, d_i)$ en `UNITS4.UNI` (`6..400 mesh` $\leftrightarrow$ `3.36..0.037 mm`) y valores intermedios fuera de tabla:
- La serie estándar Tyler sigue una progresión geométrica de razón $\sqrt{2}$ (o $\sqrt[4]{2}$ en tamices finos), por lo que la interpolación en espacio logarítmico conserva precisión física:
  $$\ln d(M) = \ln d_i + \frac{\ln M - \ln M_i}{\ln M_{i+1} - \ln M_i}\left(\ln d_{i+1} - \ln d_i\right)$$
  y su inversa $M(d)$ resulta simétrica y monotónica decreciente.

### 2.2 Hoja `Air` (Composición del Aire Seco y Contenido de Agua)
En la hoja `Air` de `UNITS4.UNI`, la humedad del aire se tabula para presión atmosférica estándar $p_{\text{atm}} = 101.325\text{ kPa}$ ($1\text{ atm}$), temperatura $T \in [-50\,^\circ\text{C},\, 102\,^\circ\text{C}]$ y humedad relativa $\text{RH} \in [0\%, 100\%]$:
1. **Presión parcial de vapor de agua:**
   $$p_{\text{H}_2\text{O}} = \frac{\text{RH}}{100} \cdot p_{\text{sat}}(T)$$
   donde $p_{\text{sat}}(T)$ se evalúa sobre agua líquida si $T \ge 0.01\,^\circ\text{C}$ (`Sat. (g,l)`) y sobre hielo si $T < 0.01\,^\circ\text{C}$ (`Sat. (g,s)`), compartiendo las ecuaciones IAPWS de `soryntel_thermo::water`.
2. **Fracción molar / volumétrica en porcentaje (`H2O vol-%`):**
   $$y_{\text{H}_2\text{O}}\,(\%) = 100 \cdot \frac{p_{\text{H}_2\text{O}}}{p_{\text{total}}}$$
   (Nótese que en `UNITS4.UNI`, cuando $T \ge 100\,^\circ\text{C}$ a $1\text{ atm}$, $p_{\text{sat}} \ge 1\text{ atm}$ y `UNITS4.UNI` acota $100\%\text{ RH} \to 100.0\text{ vol-}\%$).
3. **Humedad absoluta por metro cúbico normal de gas húmedo ideal (`H2O g/Nm³`):**
   Con volumen molar normal de gas ideal $V_{m,0} = 22.414\text{ Nm}^3/\text{kmol}$ ($0\,^\circ\text{C}, 1\text{ atm}$) y masa molar del agua $M_{\text{H}_2\text{O}} = 18.01528\text{ g/mol}$:
   $$\rho_{\text{H}_2\text{O}}^{\text{Nm}^3} = \left(\frac{y_{\text{H}_2\text{O}}}{100}\right) \cdot \frac{M_{\text{H}_2\text{O}}}{V_{m,0}} \cdot 1000 = \left(\frac{y_{\text{H}_2\text{O}}}{100}\right) \cdot 803.751\text{ g/Nm}^3$$
   Verificación exacta contra `UNITS4.UNI`: a `100 °C, 100 % RH` ($y_{\text{H}_2\text{O}} = 100\%$), `UNITS4.UNI` reporta `803.748 g/Nm3`; a `20 °C, 100 % RH` ($y = 2.34\%$), $0.0234 \times 803.748 = 18.808\text{ g/Nm}^3$ (valor exacto en `Air` fila 45 col 17 de `UNITS4.UNI`: `18.808`!).

### 2.3 Hoja `H2O` (Presión de Vapor de Agua `Vapour Pressure of Water`)
- Tabla de referencia rápida de `-90 °C` a `370 °C` con conversión exacta $\text{kN/m}^2 \leftrightarrow \text{atm}$ usando $1\text{ atm} = 101.325\text{ kN/m}^2$ (en `UNITS4.UNI` usa `101.33 kN/m² ≈ 1.0 atm` a `100 °C`), consistente con el Módulo 13 (`Water`).
