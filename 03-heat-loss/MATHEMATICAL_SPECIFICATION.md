# Especificación Matemática y Numérica (`MATHEMATICAL_SPECIFICATION.md`) — Módulo 3

Todas las ecuaciones numeradas `(1)`–`(53)` corresponden directamente a la Sección 12.4 de [`12-heat-loss.md`](file:///home/nando/Documentos/Docs%20HSC/12-heat-loss/12-heat-loss.md).

---

## 1. Fundamentos y Red de Resistencias Térmicas (Ecs. 1–7)

- Flujo de calor total y densidad de flujo térmico:
  $$q_x = q''_x \cdot A \quad [\text{W}] \qquad (1)$$
- Resistencia térmica $R$ y conductancia térmica $G$:
  $$R = \frac{T_1 - T_2}{q_x} \quad [\text{K/W}] \qquad (2), \qquad G = \frac{1}{R} \quad [\text{W/K}] \qquad (3)$$
- Propiedades termofísicas del fluido:
  - Difusividad térmica: $\alpha = \frac{k}{c_p \rho} \quad [\text{m}^2/\text{s}] \qquad (4)$
  - Viscosidad cinemática: $\nu = \frac{\mu}{\rho} \quad [\text{m}^2/\text{s}] \qquad (5)$
  - Número de Prandtl: $Pr = \frac{\nu}{\alpha} = \frac{c_p \mu}{k} \qquad (6)$
  - Coeficiente de expansión volumétrica: $\beta = -\frac{1}{\rho}\left(\frac{\partial \rho}{\partial T}\right)_p \quad [\text{K}^{-1}] \qquad (7)$
    *(Para gas ideal: $\beta = 1 / T_f$, donde $T_f$ está en Kelvin).*
- Constante de Stefan–Boltzmann (CODATA): $\sigma = 5.670374419 \times 10^{-8}\ \text{W}/(\text{m}^2\cdot\text{K}^4)$ (en literatura clásica del manual: $5.67\times 10^{-8}$).

---

## 2. Conducción Unidimensional en Sólidos (Sección 12.4.1, Ecs. 8–12)

- Ley de Fourier: $q''_x = -k \frac{dT}{dx} \qquad (8)$.
- Discretización en malla de $m$ subcapas (`Calculation Grid`, default $m=10$) cuando $k = k(T)$:
  $$q_x = \frac{T_0 - T_m}{\sum_{i=1}^{m} R_i}, \qquad R_i = \frac{1}{S_i \, k(\bar{T}_i)}, \qquad \bar{T}_i = \frac{T_{i-1} + T_i}{2} \qquad (9)$$
- **Factores de forma geométricos $S_i$** (para subcapa entre radio/posición $x_{i-1}$ y $x_i$, con dimensiones **internas** del equipo):
  - **Pared plana (`Wall`) y Cubo (`Cube`):**
    $$S = \frac{A}{\Delta x} \qquad (10)$$
    *(Para `Cube` con dimensiones internas $W \times L \times H$ y espesor acumulado $d$, el área media de capa o suma de caras sigue el factor geométrico cúbico de área creciente con $d$).*
  - **Cilindro (`Cylinder`, altura $h$, radio interior $r_i$, exterior $r_o$):**
    $$S = \frac{2\pi h}{\ln(r_o / r_i)} \qquad (11), \qquad R_{\text{cyl}} = \frac{\ln(r_o / r_i)}{2\pi h k}$$
  - **Esfera (`Sphere`, radio interior $r_i$, exterior $r_o$):**
    $$S = \frac{4\pi r_i r_o}{r_o - r_i} \qquad (12), \qquad R_{\text{sph}} = \frac{r_o - r_i}{4\pi r_i r_o k}$$
- Conductividad efectiva promedio reportada en fila 12:
  $$k_{\text{avg}} = \frac{q_x}{S_{\text{total}}\,(T_0 - T_m)}$$

---

## 3. Convección Libre y Forzada (Sección 12.4.2, Ecs. 13–40)

- Coeficiente convectivo: $h_c = \frac{Nu_L \, k}{L} \qquad (13)$
- Temperatura de película: $T_f = \frac{T_s + T_\infty}{2} \qquad (14)$ (en Kelvin para evaluar $\rho, c_p, k, \mu, \nu, Pr, \beta$).
- Número de Rayleigh y Grashof:
  $$Ra_L = \frac{g \beta |T_s - T_\infty| L^3}{\nu \alpha} = Gr_L \, Pr \qquad (15), \qquad Gr_L = \frac{Ra_L}{Pr} \qquad (16)$$
  con $g = 9.80665\ \text{m/s}^2$.

### 3.1 Correlaciones de Convección Libre (`Free Convection`)

| Geometría / Orientación | Rango / Condición | Correlación $Nu_L$ | Ec. |
|---|---|---|---|
| **Pared vertical** | $Ra_L < 10^9$ (laminar) | $Nu_L = 0.68 + \dfrac{0.670\, Ra_L^{1/4}}{\left[1 + (0.492/Pr)^{9/16}\right]^{4/9}}$ | (17) |
| **Pared vertical** (Churchill–Chu) | $Ra_L \ge 10^9$ (turbulento) | $Nu_L = \left\{0.825 + \dfrac{0.387\, Ra_L^{1/6}}{\left[1 + (0.492/Pr)^{9/16}\right]^{8/27}}\right\}^2$ | (18) |
| **Encerramiento vertical** ($H/L$) | $H/L \le 2$ | $Nu_L = 0.18 \left(\dfrac{Pr}{0.2+Pr} Ra_L\right)^{0.29}$ | (19) |
| | $2 < H/L < 10$ | $Nu_L = 0.22 \left(\dfrac{Pr}{0.2+Pr} Ra_L\right)^{0.28} \left(\dfrac{H}{L}\right)^{-1/4}$ | (20) |
| | $H/L \ge 10$ | $Nu_L = 0.42 \, Ra_L^{1/4} Pr^{0.012} \left(\dfrac{H}{L}\right)^{-0.3}$ | (21) |
| **Placa horizontal** ($L = A_s / P$) | Cara caliente abajo / fría arriba | $Nu_L = 0.27\, Ra_L^{1/4}$ | (22) |
| | Cara caliente arriba / fría abajo, $10^4 \le Ra_L \le 10^7$ | $Nu_L = 0.54\, Ra_L^{1/4}$ | (23) |
| | Cara caliente arriba / fría abajo, $10^7 < Ra_L \le 10^{11}$ | $Nu_L = 0.15\, Ra_L^{1/3}$ | (24) |
| **Encerramiento horizontal** | $Ra_L \le 1708$ (conducción pura) | $Nu_L = 1$ | (25a) |
| | $Ra_L > 1708$ | $Nu_L = 0.069\, Ra_L^{1/3} Pr^{0.074}$ | (25b) |
| **Cilindro horizontal** (Churchill–Chu, $L = D$) | $Ra_D \le 10^{12}$ | $Nu_D = \left\{0.60 + \dfrac{0.387\, Ra_D^{1/6}}{\left[1 + (0.559/Pr)^{9/16}\right]^{8/27}}\right\}^2$ | (26) |
| **Cilindros concéntricos** (Raithby–Hollands) | $k_{\text{eff}}/k$ | $\dfrac{k_{\text{eff}}}{k} = 0.386 \left(\dfrac{Pr}{0.861+Pr}\right)^{1/4} Ra_c^{*1/4}$, $Ra_c^* = \dfrac{[\ln(D_o/D_i)]^4}{L^3 (D_i^{-3/5} + D_o^{-3/5})^5} Ra_L$ | (27)–(30) |
| **Esfera** (Churchill, $L = D$) | $Ra_D \le 10^{11}$, $Pr \ge 0.7$ | $Nu_D = 2 + \dfrac{0.589\, Ra_D^{1/4}}{\left[1 + (0.469/Pr)^{9/16}\right]^{4/9}}$ | (31) |
| **Esferas concéntricas** | $k_{\text{eff}}/k$ | $\dfrac{k_{\text{eff}}}{k} = 0.74 \left(\dfrac{Pr}{0.861+Pr}\right)^{1/4} Ra_s^{*1/4}$, $Ra_s^* = \dfrac{L}{(D_i D_o)^4} \dfrac{Ra_L}{(D_i^{-7/5}+D_o^{-7/5})^5}$ | (32)–(35) |

*(Para `Cube` y `Vertical Cylinder`, el manual usa longitud característica vertical/equivalente y criterios de curvatura estándar de Incropera & DeWitt).*

### 3.2 Correlaciones de Convección Forzada (`Forced Convection`)

- Número de Reynolds: $Re_L = \frac{u_\infty L}{\nu} \qquad (36)$
- **Placa plana / Cubo:**
  - Laminar ($Re_L \le 5\times 10^5$): $Nu_L = 0.664\, Re_L^{1/2} Pr^{1/3} \qquad (37)$
  - Mixto/Turbulento ($5\times 10^5 < Re_L \le 10^7$): $Nu_L = (0.037\, Re_L^{4/5} - 871)\, Pr^{1/3} \qquad (38)$
- **Cilindro en flujo cruzado (Churchill–Bernstein, $Re_D Pr \ge 0.2$):**
  $$Nu_D = 0.3 + \frac{0.62\, Re_D^{1/2} Pr^{1/3}}{\left[1 + (0.4/Pr)^{2/3}\right]^{1/4}} \left[1 + \left(\frac{Re_D}{282000}\right)^{5/8}\right]^{4/5} \qquad (39)$$
- **Esfera (Whitaker):**
  $$Nu_D = 2 + \left(0.4\, Re_D^{1/2} + 0.06\, Re_D^{2/3}\right) Pr^{0.4} \left(\frac{\mu}{\mu_s}\right)^{1/4} \qquad (40)$$
- **Función personalizada (`Function` tab):** $h_c(v) = A v^a + B v^b + \dots$

---

## 4. Radiación Superficial, de Gases y de Partículas (Sección 12.4.3, Ecs. 41–53)

### 4.1 Radiación Superficial (Cuerpo Gris Difuso)
$$q'' = \varepsilon_s \sigma (T_s^4 - T_\infty^4) \qquad (41), \qquad h_r = \varepsilon_s \sigma (T_s + T_\infty)(T_s^2 + T_\infty^2) \qquad (42)$$
- Para **encerramiento entre dos superficies** $(\varepsilon_1, T_1)$ y $(\varepsilon_2, T_2)$:
  - Placas paralelas infinitas: $\varepsilon_{\text{eff}} = \left(\frac{1}{\varepsilon_1} + \frac{1}{\varepsilon_2} - 1\right)^{-1}$
  - Cilindros/esferas concéntricos: $\varepsilon_{\text{eff}} = \left(\frac{1}{\varepsilon_1} + \frac{1-\varepsilon_2}{\varepsilon_2}\left(\frac{r_1}{r_2}\right)^n\right)^{-1}$ ($n=1$ cilindro, $n=2$ esfera).
  $$h_r = \varepsilon_{\text{eff}} \sigma (T_1 + T_2)(T_1^2 + T_2^2) \qquad (43)$$

### 4.2 Radiación de Mezcla de Gases (EWBM, Ecs. 44–47)
$$q'' = \sigma \frac{\varepsilon_s + 1}{2}\left(\varepsilon_g T_g^4 - \alpha_g T_s^4\right) \qquad (44), \qquad h_r = \frac{(\varepsilon_s + 1)\sigma\left(\varepsilon_g T_g^4 - \alpha_g T_s^4\right)}{2(T_g - T_s)} \qquad (45)$$
- Emisividad de mezcla de 6 especies ($\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$):
  $$\varepsilon_g = \varepsilon_g(T_g, p, L, x_{\text{H}_2\text{O}}, x_{\text{CO}_2}, x_{\text{CO}}, x_{\text{NO}}, x_{\text{SO}_2}, x_{\text{CH}_4}) \qquad (46)$$
  calculada mediante el **Exponential Wide Band Model (EWBM)** de Edwards & Balakrishnan (1973) con integración espectral optimizada por Lallemant & Weber (1996) sobre bandas vibracionales–rotacionales (`Nyy-Low`, `Nyy-Upp`, `F-Low`, `F-Upp`, `DF`, `SumDF`).
- Absortividad de gas:
  $$\alpha_g = \varepsilon_g\!\left(T_g,\ p,\ L\left(\frac{T_s}{T_g}\right)^{1.5},\ x_{\text{H}_2\text{O}},\dots,x_{\text{CH}_4}\right) \cdot \left(\frac{T_g}{T_s}\right)^{0.5} \qquad (47)$$
- Rango de validez: $T \in [300, 3000]\text{ K}$, $p \in [0.5, 20]\text{ bar}$. Longitud óptica $L$ desde Tabla 1 del manual (p.ej. $0.63D$ esfera, $0.6D$ cilindro $h=D$, $0.6x$ cubo $1\times 1\times 1$, con interpolación/extrapolación bilineal).

### 4.3 Radiación Combinada Gas + Nube de Partículas (Ecs. 48–53)
- Emisividad de nube de partículas:
  $$\varepsilon_c = 1 - \exp\!\left(-\varepsilon_p n_p L A_p\right) \qquad (48), \qquad A_p = \frac{\pi d_s^2}{4} = \frac{\pi \sum n_i d_i^2}{4 \sum n_i} \qquad (49)$$
- Acoplamiento gas + partícula:
  $$\varepsilon_{g+c} = \varepsilon_g + \varepsilon_c - \varepsilon_g \varepsilon_c \qquad (50), \qquad \alpha_{g+c} = \alpha_g + \varepsilon_c - \alpha_g \varepsilon_c \qquad (51)$$
- Flujo y coeficiente radiativo combinado:
  $$q'' = \frac{\varepsilon_s}{\alpha_{g+c} + \varepsilon_s - \alpha_{g+c}\varepsilon_s}\,\sigma\!\left(\varepsilon_{g+c} T_g^4 - \alpha_{g+c} T_s^4\right) \qquad (52)$$
  $$h_r = \frac{\varepsilon_s\,\sigma\!\left(\varepsilon_{g+c} T_g^4 - \alpha_{g+c} T_s^4\right)}{\left(\alpha_{g+c} + \varepsilon_s - \alpha_{g+c}\varepsilon_s\right)(T_g - T_s)} \qquad (53)$$

---

## 5. Algoritmos Numéricos para el Resolvedor en Rust (`soryntel`)

1. **Modo `Heat flow` (red no lineal 1D de $N$ elementos y $N+1$ nodos $T_0, \dots, T_N$):**
   - Dadas $T_a$ y $T_b$ fijas (típicamente $T_0$ y $T_N$), inicializar perfil lineal $T_k^{(0)}$.
   - En cada iteración externa $m$:
     1. Para cada columna $i = 1 \dots N$, evaluar $R_i(T_{i-1}^{(m)}, T_i^{(m)})$ (subdividiendo los `SolidLayer` en $m_{\text{grid}}$ pasos con sub-iteración de Picard o Newton local para $k(\bar{T})$).
     2. Calcular el flujo de calor estacionario:
        $$Q^{(m)} = \frac{T_a - T_b}{\sum_{i=a}^{b-1} R_i}$$
     3. Actualizar temperaturas nodales: $T_i^{\text{new}} = T_{i-1}^{\text{new}} - Q^{(m)} R_i$.
     4. Aplicar **sub-relajación amortiguada** $T_i^{(m+1)} = \omega T_i^{\text{new}} + (1-\omega) T_i^{(m)}$ (con $\omega \in [0.4, 0.8]$ adaptativo) para garantizar estabilidad ante saltos de régimen laminar/turbulento en $Nu(Ra_L)$ o radiación $T^4$.
   - Criterio de convergencia: $\max_i |T_i^{(m+1)} - T_i^{(m)}| < 10^{-6}\text{ K}$ y $|\Delta Q / Q| < 10^{-8}$.
2. **Modo `Temperature profile`:**
   - Dado $Q$ fijado y 1 temperatura fija $T_{k_0}$, propagar secuencialmente hacia ambos lados resolviendo en cada columna $i$ la ecuación escalar $T_{\text{unknown}} - T_{\text{known}} \pm Q \cdot R_i(T_{\text{known}}, T_{\text{unknown}}) = 0$ mediante **Brent / secante amortiguada**.
3. **Target Solver:**
   - Método de **Brent (`brentq`)** con acotamiento automático dentro de `[min, max]` para resolver $g(x) - y_{\text{target}} = 0$ con tolerancia configurable.
