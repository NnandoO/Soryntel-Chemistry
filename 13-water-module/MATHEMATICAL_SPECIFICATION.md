# Especificación Matemática — Módulo 13: Water Module (`PTCalc`)

## 1. Constantes Fundamentales y Puntos Singulares ($\text{H}_2\text{O}$)
- **Masa molar:** $M_w = 18.01528\text{ g/mol}$ ($\text{kmol} \leftrightarrow \text{kg}$).
- **Constante específica del gas:** $R_{\text{H}_2\text{O}} = \frac{R}{M_w} = \frac{8.3144626}{0.01801528} = 461.526\text{ J/(kg}\cdot\text{K)}$.
- **Punto Triple (PT):**
  $$T_t = 273.16\text{ K}\; (0.01\,^\circ\text{C}), \qquad p_t = 0.00611657\text{ bar}\; (611.657\text{ Pa})$$
  (en textos HSC redondeado a `0.0061 bar`).
- **Punto Crítico (PC):**
  $$T_c = 647.096\text{ K}\; (373.946\,^\circ\text{C}), \qquad p_c = 220.64\text{ bar}\; (22.064\text{ MPa}), \qquad \rho_c = 322.0\text{ kg/m}^3$$
- **Dominio Operativo:**
  $$T \in [-200\,^\circ\text{C},\; 2000\,^\circ\text{C}]\; (73.15\text{ K} \le T \le 2273.15\text{ K}), \qquad p \in [0.001\text{ bar},\; 1000\text{ bar}]$$

---

## 2. Ecuaciones de las Curvas de Saturación (Ecuaciones [6]–[8])

### 2.1 Saturación Gas–Líquido `Sat. (g,l)` (Wagner–Pruss, 1993 / IAPWS)
Válida para $T_t \le T \le T_c$. Con $\tau = 1 - \frac{T}{T_c}$:
$$\ln\left(\frac{p_{\text{sat,gl}}}{p_c}\right) = \frac{T_c}{T}\left(a_1\tau + a_2\tau^{1.5} + a_3\tau^3 + a_4\tau^{3.5} + a_5\tau^4 + a_6\tau^{7.5}\right)$$
donde $a_1 = -7.85951783$, $a_2 = 1.84408259$, $a_3 = -11.7866497$, $a_4 = 22.6807411$, $a_5 = -15.9618719$, $a_6 = 1.80122502$ ($p$ en MPa o bar según $p_c$).
- La función inversa $T_{\text{sat,gl}}(p)$ se resuelve mediante método de Newton–Raphson / bisección garantizando convergencia $|f(T)| < 10^{-10}\text{ bar}$ para $p \in [p_t, p_c]$.

### 2.2 Saturación Gas–Sólido `Sat. (g,s)` (Sublimación, IAPWS 2011)
Válida para $T \le T_t$. Con $\theta = \frac{T}{T_t}$:
$$\ln\left(\frac{p_{\text{sat,gs}}}{p_t}\right) = \theta^{-1}\left(a_1(1 - \theta^{1.5}) + a_2(1 - \theta^{1.25})\right)$$
donde $a_1 = -13.928169$, $a_2 = 34.707823$ (extrapolada suavemente por debajo de $-200\,^\circ\text{C}$ hasta $-220\,^\circ\text{C}$ cuando sea requerido).

### 2.3 Saturación Líquido–Sólido `Sat. (l,s)` (Fusión de Hielo Ih, IAPWS)
Válida entre $p_t$ y $p_{\max,\text{Ih}} \approx 2085.66\text{ bar}$ ($T_{\min} \approx 251.165\text{ K} \approx -21.985\,^\circ\text{C}$, cubriendo todo el rango hasta $1000\text{ bar}$):
$$\frac{p_{\text{sat,ls}}}{p_t} = 1 - 0.626000\times 10^6 \left(1 - \theta^{-3}\right) + 0.197135\times 10^6 \left(1 - \theta^{21.2}\right), \qquad \theta = \frac{T}{T_t}$$
- Dado $p \in [p_t, 1000\text{ bar}]$, $T_{\text{melt}}(p)$ se obtiene invirtiendo la ecuación monotónica anterior (p. ej. a $p = 500\text{ bar}$, resulta $T_{\text{melt}} \approx -4.1 \text{ a } -4.5\,^\circ\text{C}$, reproduciendo el Ejemplo 25.2.1 de HSC).

---

## 3. Reglas de Determinación Automática de Fase Pura
Cuando el usuario selecciona fase pura automática o hace clic en el diagrama $p\text{-}T$:
1. Si $T \ge T_c$: región **supercrítica**. Siguiendo `25 Water.txt` Sección 25.3.1, se asigna fase **Liquid (l)** si $p \ge p_c$ ($220.64\text{ bar}$) y fase **Gas (g)** si $p < p_c$, evaluando propiedades continuas sin salto de cambio de fase.
2. Si $T \le T_t$:
   - Si $p < p_{\text{sat,gs}}(T)$ → **Gas (g)**.
   - Si $p \ge p_{\text{sat,gs}}(T)$: comparar con la curva de fusión $T_{\text{melt}}(p)$ (cuando $p > p_t$). Si $T < T_{\text{melt}}(p)$ → **Solid (s)**; de lo contrario → **Liquid (l)**.
3. Si $T_t < T < T_c$:
   - Si $p < p_{\text{sat,gl}}(T)$ → **Gas (g)**.
   - Si $p \ge p_{\text{sat,gl}}(T)$ → **Liquid (l)** (salvo si $p > p_t$ y $T < T_{\text{melt}}(p)$, en cuyo caso es **Solid (s)**).
4. Cuando se selecciona explícitamente un estado saturado (`Sat. (g,l)`, `Sat. (g,s)` o `Sat. (l,s)`):
   - En `Sat. (g,l)` o `Sat. (g,s)`: el usuario fija $T$ (dentro del rango de la curva) y $p$ se recalcula automáticamente como $p_{\text{sat}}(T)$ (mostrada en negro en la hoja).
   - En `Sat. (l,s)`: el usuario fija $p \ge p_t$ y $T$ se recalcula automáticamente como $T_{\text{melt}}(p)$.

---

## 4. Propiedades Termodinámicas, Mezcla Bifásica y Corrección por Presión

### 4.1 Propiedades de Referencia HSC vs Tablas de Vapor
En la escala absoluta de HSC Chemistry ($T_{\text{ref}} = 298.15\text{ K},\; p_{\text{ref}} = 1\text{ bar}$):
- **Entalpía molar estándar:** $H_m(T, p)$ con referencia $H_m^\circ(\text{H}_2\text{O}(l), 298.15\text{ K}, 1\text{ bar}) = -285.830\text{ kJ/mol}$.
- **Entropía molar estándar:** $S_m(T, p)$ con $S_m^\circ(\text{H}_2\text{O}(l), 298.15\text{ K}, 1\text{ bar}) = 69.95\text{ J/(mol}\cdot\text{K)}$.
- **Energía de Gibbs:**
  $$G(T, p) = H(T, p) - T \cdot S(T, p)$$
- **Entalpía específica (`H specific`):**
  Ignora el calor latente de transformación de fase, midiendo únicamente la integral sensible desde $25\,^\circ\text{C}$ en la fase evaluada ($H_{\text{specific}}(25\,^\circ\text{C}) = 0$).

### 4.2 Ecuación de Estado / Corrección por Presión (Ecuaciones [3]–[5] y [9])
Para cualquier fase $\phi \in \{s, l, g\}$ a $(T, p)$:
$$H_\phi(T, p) = H_f'(298.15\text{ K}, p) + \int_{298.15}^{T} C_{p,\phi}(T', p)\, dT' + \sum \Delta H_{\text{tr}}(p)$$
$$S_\phi(T, p) = S_0'(298.15\text{ K}, p) + \int_{298.15}^{T} \frac{C_{p,\phi}(T', p)}{T'}\, dT' + \sum \frac{\Delta H_{\text{tr}}(p)}{T_{\text{tr}}(p)}$$
- Para **sólido (hielo) y líquido comprimido** (Ecuación [9]): integración de Poynting/compresibilidad y expansividad térmica $\alpha$:
  $$\Delta H_p = \int_{p_f}^{p} v(T, p')\left(1 - \alpha T\right) dp', \qquad \Delta S_p = -\int_{p_f}^{p} v(T, p') \alpha\, dp'$$
- Para **vapor / fluido supercrítico**: empleo de ecuación de estado real (IAPWS-IF97 / Peng-Robinson calibrada o interpolación de las tablas `H2O(Xbarg)` + desviación residual $H^R(T,p), S^R(T,p), C_p^R(T,p), \rho(T,p)$) convergiendo al gas ideal $\text{H}_2\text{O}(g)$ cuando $p \to 0$ o $T \gg T_c$.

### 4.3 Regla de la Palanca en Estados Saturados (`Mixture %`)
Sea $x \in [0, 1]$ la fracción (`Mixture % / 100`) de la **primera fase** nombrada en el par saturado `(fase 1, fase 2)` (en `Sat. (g,l)`, $x$ es el título de vapor o *dryness fraction*; en `Sat. (g,s)`, fracción de gas; en `Sat. (l,s)`, fracción de líquido):
$$H_{\text{mix}} = x H_1 + (1 - x) H_2$$
$$S_{\text{mix}} = x S_1 + (1 - x) S_2$$
$$G_{\text{mix}} = x G_1 + (1 - x) G_2$$
$$H_{\text{spec,mix}} = x H_{\text{spec},1} + (1 - x) H_{\text{spec},2}$$
$$C_{p,\text{mix}} = x C_{p,1} + (1 - x) C_{p,2}$$
$$v_{\text{mix}} = \frac{x}{\rho_1} + \frac{1 - x}{\rho_2} \implies \rho_{\text{mix}} = \frac{1}{v_{\text{mix}}}$$

---

## 5. Diferencias entre Puntos y Eficiencia Isentrópica (Ecuaciones [1] y [2])
Para una cantidad $n$ (`kmol` o masa `kg` con $n = m / M_w$):
$$\Delta H = H(\text{Point 1}) - H(\text{Point 2}), \qquad \Delta S = S(\text{Point 1}) - S(\text{Point 2}), \qquad \Delta G = G(\text{Point 1}) - G(\text{Point 2})$$
(o respecto al orden mostrado en la hoja de HSC donde $\Delta = \text{Point 1} - \text{Point 2}$, por eso con `Point 2 = Triple Point (Liquid)` un vapor sobrecalentado en `Point 1` entrega exactamente los valores positivos $\Delta H = h_{\text{steam}} > 0$ y $\Delta S = s_{\text{steam}} > 0$ de las tablas de vapor).

- **Eficiencia isentrópica de compresor / bomba (Ecuación [1]):**
  $$\eta_{s,c} = \frac{\Delta h_s}{\Delta h}$$
- **Eficiencia isentrópica de turbina (Ecuación [2]):**
  $$\eta_{s,t} = \frac{\Delta h}{\Delta h_s}$$
  donde $\Delta h_s$ corresponde al cambio entálpico evaluado manteniendo $\Delta S = 0$ (ajustando `Mixture %` o $T_1$ isentrópicamente a la presión final).
