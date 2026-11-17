# Especificación Matemática — Módulo 17: Aqua

Todas las ecuaciones numeradas `(1)–(39)` y `(A1)` provienen de las secciones `36.8–36.11` y el **Apéndice A** de [36-aqua-module.md](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/36-aqua-module.md), complementadas con la formulación de densidad de **Laliberté & Cooper (2004)** verificada sobre [Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls).

---

## 1. Potencial Químico y Convenciones de Estado Estándar

En solución acuosa no ideal, el potencial químico $\mu_i$ (energía libre de Gibbs parcial molar, $\text{J/mol}$) de un soluto $i$ en escala molal ($m^\circ = 1\text{ mol/kg}$) viene dado por las Ecs. `(3)–(6)`:

$$\mu_i = \mu_i^\circ + R T \ln a_i \qquad (3)$$

$$a_i = \frac{\gamma_i m_i}{m^\circ} \qquad (4)$$

$$\gamma_i \to 1 \quad \text{y} \quad a_i \to \frac{m_i}{m^\circ} \quad \text{cuando } m_i \to 0 \qquad (5)$$

$$\mu_i = \mu_i^\circ + R T \ln\left(\frac{m_i}{m^\circ}\right) + R T \ln \gamma_i = \mu_i^\circ + R T \ln(m_i \gamma_i) \qquad (6)$$

Para una sal fuerte $M_{\nu_+} X_{\nu_-}$ que se disocia en $\nu_+$ cationes $M^+$ y $\nu_-$ aniones $X^-$ ($\nu = \nu_+ + \nu_-$), su potencial químico molal y el **coeficiente de actividad medio** $\gamma_\pm$ se definen por las Ecs. `(7)–(10)`:

$$G_m = \nu_+ \mu_+ + \nu_- \mu_- = \mu_+^\circ + \mu_-^\circ + \nu_+ R T \ln(m_+ \gamma_+) + \nu_- R T \ln(m_- \gamma_-) = G_m^\circ + R T \ln(m_\pm \gamma_\pm)^\nu \qquad (7)$$

$$\gamma_\pm = \left( \gamma_+^{\nu_+} \gamma_-^{\nu_-} \right)^{1 / (\nu_+ + \nu_-)} \qquad (8)$$

### Fuerza iónica y conversión entre escalas molal y molar / fracción molar

La **fuerza iónica molal** $I$ ($\text{mol/kg}$) se calcula en `Istrength` mediante la Ec. `(36)`:

$$I = \frac{1}{2} \sum_i m_i z_i^2 \qquad (36)$$

donde $m_i = 55.50844 \cdot \frac{n_i}{n_{\text{H}_2\text{O}}}$ (usando $W = 18.01528\text{ g/mol}$, con $1000/W \approx 55.50844\text{ mol/kg}$; en el texto de `36.8` se menciona también el valor redondeado `55.509`).

La relación entre el coeficiente de actividad **en escala molar/fracción molar** exhibido en la columna **`AC Molar Sc.`** ($\gamma_x$) y el coeficiente **molal** **`AC Molal Sc.`** ($\gamma_m$) está dada por la Ec. `(37)`:

$$\gamma_x = \frac{\gamma_m}{x(\text{H}_2\text{O})} \qquad (37)$$

> **Verificación empírica contra las capturas del manual (Fig. 3):** Con $x(\text{H}_2\text{O}) = 93.77\% = 0.9377$ a $40\,^\circ\text{C}$, para $\text{Na(+a)}$ la captura muestra `AC Molal Sc. = 0.44` y `AC Molar Sc. = 0.47`; en efecto $0.44 / 0.9377 = 0.4692 \approx 0.47$. Igualmente para $\text{OH(-a)}$: $0.71 / 0.9377 = 0.757 \approx 0.76$; para $\text{H(+a)}$: $0.29 / 0.9377 = 0.309 \approx 0.31$.
> **Nota de acoplamiento con `Gem` (`36.8`, ítem 4):** En el optimizador Gibbs `Gem` con la opción **`Mixing Entropy`** activa (referencia raoultiana $\mu^\circ_{\text{mix}} = \mu^\circ_{\text{molal}} + RT\ln(55.509)$), se obtienen resultados equivalentes usando coeficientes raoultianos $55.509 \cdot \gamma_m / x(\text{H}_2\text{O}) = 55.509\,\gamma_x$, pues $a_i = m_i \gamma_m = \left(\frac{55.509\,\gamma_m}{x(\text{H}_2\text{O})}\right) x_i$.

---

## 2. Modelos de Coeficientes de Actividad (`Aqua method`)

### 2.1. Opción `Davies` (Debye–Hückel extendido) — Ec. `(15)`

El manual especifica explícitamente en el apartado `36.11` (p. 19) que esta opción utiliza una forma tipo Debye–Hückel/Pitzer (*"not actually the Davies equation but called Davies here"*):

$$\ln \gamma_i^{\text{imp.}} = -A_\phi \left[ \frac{\sqrt{I}}{1 + b\sqrt{I}} + \frac{2}{b}\ln\left(1 + b\sqrt{I}\right) \right] \qquad (15)$$

> **Nota crítica de implementación (anomalía tipográfica en Ec. `15`):** La Ecuación `(15)` impresa en la página 19 de `36-aqua-module.md` omite tipográficamente el factor de carga iónica $z_i^2$ (en el formalismo de Pitzer, la expresión entre corchetes es $F_{\text{DH}} / z_i^2$ con $b = 1.2\text{ (kg/mol)}^{1/2}$). Para satisfacer la ley límite de Debye–Hückel citada en `36.3` ($\ln\gamma_i \propto z_i^2 \sqrt{I}$ y $\gamma_n = 1$ para neutros con $z_i = 0$ en ausencia de parámetros $\lambda$), toda implementación clean-room debe multiplicar por $z_i^2$:
> $$\ln\gamma_i = -z_i^2 A_\phi \left[ \frac{\sqrt{I}}{1 + b\sqrt{I}} + \frac{2}{b}\ln\left(1 + b\sqrt{I}\right) \right]$$

---

### 2.2. Opción `Pitzer` (Solo interacciones binarias y neutro–ion) — Ecs. `(16)–(19)`

Descarta los parámetros ternarios $\theta_{ij}, \Phi_{ij}, \psi_{ijk}$. Los coeficientes de actividad iónicos de un catión $M$ y un anión $X$, el coeficiente medio $\gamma_\pm$ de un electrolito binario y el coeficiente osmótico $\phi$ son:

$$\ln \gamma_{+,M} = z_M^2 F + \sum_a m_a \left(2 B_{Ma} + Z C_{Ma}\right) + 2 \sum_n m_n \lambda_{nM} \qquad (16)$$

$$\ln \gamma_{-,X} = z_X^2 F + \sum_c m_c \left(2 B_{Xc} + Z C_{Xc}\right) + 2 \sum_n m_n \lambda_{nX} \qquad (17)$$

> *(En la Ec. `(17)` impresa del manual aparece un subíndice tipográfico `-,M` y `nM` arrastrado de `(16)`, pero corresponde simétricamente al anión $X$ y $\lambda_{nX}$, como corroboran las Figs. 5 y 9).*

Para un electrolito binario único $M_{\nu_+} X_{\nu_-}$ ($\nu = \nu_+ + \nu_-$):

$$\ln \gamma_\pm = |z_+ z_-| f^\gamma + \frac{2\nu_+ \nu_-}{\nu} m \left( B_{+-}^\phi + B_{+-} \right) + \frac{3\nu_+ \nu_-}{\nu} m Z C_{+-} \qquad (18')$$

*(El manual escribe en `(18)` la forma equivalente: $\ln \gamma_\pm = z_+ |z_-| f^\gamma + \frac{\nu_+ \nu_-}{\nu} m \left( 2 B_{+-}^\phi + 2 B_{+-} + 3 Z C_{+-} \right)$).*

$$\phi - 1 = -A_\phi z_+ |z_-| \frac{\sqrt{I}}{1 + b\sqrt{I}} + 2 \frac{\nu_+ \nu_-}{\nu} m \left( B_{+-}^\phi + Z C_{+-} \right) \qquad (19)$$

donde los parámetros de entrada en esta opción son $\beta^{(0)}_{MX}, \beta^{(1)}_{MX}, \beta^{(2)}_{MX}, C^\phi_{MX}$ y $\lambda_{ni}$ (con $Z = \sum_i m_i |z_i|$, $b = 1.2$, $\alpha_1 = 2.0$ para 1:1 / 1:2 / 2:1 o $\alpha_1 = 1.4, \alpha_2 = 12.0$ para electrolitos 2:2 según Pitzer 1973/1991, y conversión $C_{MX} = \frac{C^\phi_{MX}}{2\sqrt{|z_M z_X|}}$).

---

### 2.3. Opción `Harvie` (Binarios + Ternarios completos $\Phi_{ij}, \Psi_{ijk}, \lambda_{ni}$) — Ecs. `(20)–(21)`

Extiende el modelo de Pitzer siguiendo a Harvie, Møller y Weare (1984) incorporando parámetros binarios entre iones de igual signo ($\theta_{cc'}, \theta_{aa'}$ y términos asimétricos electrostáticos $\Phi_{cc'}, \Phi_{aa'}$), tripletes $\psi_{cca}, \psi_{caa'}$ y neutros $\lambda_{nM}, \lambda_{nX}$ (mostradas en `36.11` y en las Figs. 5 y 9):

$$\ln \gamma_{+,M} = z_+^2 F + \sum_a m_a \left(2 B_{Ma} + Z C_{Ma}\right) + \sum_c m_c \left(2 \Phi_{Mc} + \sum_a m_a \Psi_{Mca}\right) + \sum_{a < a'} \sum m_a m_{a'} \Psi_{Maa'} + |z_M| \sum_c \sum_a m_c m_a C_{ca} + 2 \sum_n m_n \lambda_{nM} + \cdots \qquad (20)$$

$$\ln \gamma_{-,X} = z_-^2 F + \sum_c m_c \left(2 B_{Xc} + Z C_{Xc}\right) + \sum_a m_a \left(2 \Phi_{Xa} + \sum_c m_c \Psi_{Xca}\right) + \sum_{c < c'} \sum m_c m_{c'} \Psi_{cc'X} + |z_X| \sum_c \sum_a m_c m_a C_{ca} + 2 \sum_n m_n \lambda_{nX} + \cdots \qquad (21)$$

donde los parámetros configurables en el banco de datos son $\beta^{(0)}_{MX}, \beta^{(1)}_{MX}, \beta^{(2)}_{MX}, C^\phi_{MX}, \theta_{ij}, \psi_{ijk}$ y $\lambda_{ni}$ (con $F$ incluyendo además las derivadas de $B'_{ca}$ y $\Phi'_{ij}$ respecto a $I$ en el formalismo estándar de Pitzer–Harvie).

---

## 3. Dependencia con la Temperatura y Propiedades de Exceso

### 3.1. Función paramétrica unificada en HSC Aqua — Ec. `(22)`

Todos los parámetros Pitzer–Harvie ($P \in \{\beta^{(0)}, \beta^{(1)}, \beta^{(2)}, C^\phi, \theta, \psi, \lambda\}$) se ajustan en `MainDB` y `OwnDB` con una única ecuación térmica en Kelvin ($T$):

$$F(T) = a + \frac{b}{T} + c \ln(T) + d T + e T^2 + f T^{-2} \qquad (22)$$

- Si solo existen datos a $25\,^\circ\text{C}$, únicamente $a \neq 0$ ($b=c=d=e=f=0$, marcado como **`C`** en el visor).
- Las derivadas analíticas usadas para entalpía y $C_p$ son:
  $$F'(T) = -\frac{b}{T^2} + \frac{c}{T} + d + 2 e T - \frac{2 f}{T^3}$$
  $$F''(T) = \frac{2 b}{T^3} - \frac{c}{T^2} + 2 e + \frac{6 f}{T^4}$$

### 3.2. Entalpía de exceso, capacidad calorífica y entalpía de dilución — Ecs. `(11)–(14)`, `(39)`

Por relación de Gibbs–Helmholtz sobre la energía libre de Gibbs de exceso ($L \equiv H^{\text{ex}}$):

$$\left(\frac{\partial \ln \gamma_i}{\partial T}\right)_P = -\frac{H^{\text{ex}}_{m,i}}{\nu R T^2} \qquad (11)$$

La entalpía molar aparente relativa $\phi_L$ ($\text{J/mol}$) y la capacidad calorífica molar $C_{P,m}$ ($\text{J/(K}\cdot\text{mol)}$) y aparente $\phi_{C_P}$ cumplen:

$$\phi_L = \frac{L - n_1 L_1^\circ}{n_2} = \frac{L}{n_2} \qquad (12)$$

$$C_{P,m} = \left(\frac{\partial H_m}{\partial T}\right)_P = C^\circ_{P,m} + \left(\frac{\partial L_m}{\partial T}\right)_P \qquad (13)$$

$$\phi_{C_P} = \frac{C_P - n_1 C_{P1}^\circ}{n_2} \qquad (14)$$

La entalpía de dilución $\Delta H_D(m_1 \to m_2)$ de una solución binaria entre molalidad inicial $m_1$ y final $m_2$ es (Silvester & Pitzer, 1978):

$$\Delta H_D(m_1 \to m_2) = \phi_{L,2} - \phi_{L,1} \qquad (39)$$

---

## 4. Actividad del Agua, Presión de Vapor, Presión Osmótica y pH (`Osmcalc` & `pHcalc`)

### 4.1. Actividad del agua, presión de vapor y humedad relativa — Ecs. `(23)–(25)` y Apéndice A `(A1)`

$$\ln(a_w) = -\frac{W}{1000} \left(\sum_i m_i\right) \phi \qquad (23)$$

$$a_w = \frac{p}{p_0} \implies p = a_w \cdot p_0 \qquad (24)$$

$$\text{RH}(\%) = \frac{p}{p_0} \cdot 100\% = 100 \cdot a_w \qquad (25)$$

donde $W \approx 18.01528\text{ g/mol}$ es la masa molar del agua, $\sum_i m_i$ suma todos los solutos (iones + neutros) en $\text{mol/kg}$, y $p_0$ ($\text{Pa}$, función polinomial de grado 10 en temperatura $T_C$ en $^\circ\text{C}$ calibrada contra `Measure Units` de HSC) sigue el **Apéndice A** `(A1)`:

$$p_0(T_C) = \left( a\,T_C^{10} + b\,T_C^9 + c\,T_C^8 + d\,T_C^7 + e\,T_C^6 + f\,T_C^5 + g\,T_C^4 + h\,T_C^3 + i\,T_C^2 + j\,T_C + k \right) \cdot 1000 \qquad (A1)$$

> *(Nota: en las líneas `695` y `1068` del manual convertido desde PDF aparece el error tipográfico `*d` al final de la expansión, mientras la Ec. `(A1)` de la línea `1062` muestra correctamente el multiplicador `1000` para convertir de $\text{kPa}$ a $\text{Pa}$; comprobado numéricamente a $40\,^\circ\text{C}$ donde la fórmula arroja exactamente $7377.08\text{ Pa}$ como en Fig. 4).*

| Coeficiente | Rango $T_C < 160\,^\circ\text{C}$ | Rango $T_C \ge 160\,^\circ\text{C}$ |
|---|---|---|
| **$a$** | `-3.10432954399391e-21` | `1.11982860630743e-18` |
| **$b$** | `1.00959411100662e-18` | `-4.04920519596997e-15` |
| **$c$** | `-5.88099024207425e-17` | `6.08290457531376e-12` |
| **$d$** | `-3.18476754457404e-14` | `-5.12897992812743e-9` |
| **$e$** | `6.55612456912702e-12` | `2.7240831169274e-6` |
| **$f$** | `2.25575938350116e-9` | `-9.59696050004219e-4` |
| **$g$** | `2.99998801482107e-7` | `0.228254919514224` |
| **$h$** | `2.57849258952755e-5` | `-36.311462374836` |
| **$i$** | `1.43776358189044e-3` | `3706.72828793979` |
| **$j$** | `4.51730065352071e-2` | `-219658.499462442` |
| **$k$** | `0.599030194123347` | `5746593.5787401` |

*(Observación: en el punto de empalme $T_C = 160\,^\circ\text{C}$ existe un pequeño salto de continuidad entre tramos de $617.88\text{ kPa}$ vs $618.67\text{ kPa}$, equivalente a `0.13%`).*

### 4.2. Presión osmótica y densidad del agua pura — Ecs. `(26)–(27)`

$$\pi = -\frac{R T}{v_w} \ln(a_w) = -\frac{R T \rho_w}{M_w} \ln(a_w) \qquad (26)$$

con la densidad del agua pura $\rho_w$ ($\text{kg/m}^3$) de McCutcheon et al. (1993), donde $T_C$ está en $^\circ\text{C}$ y $M_w = 0.01801528\text{ kg/mol}$:

$$\rho_w(T_C) = 1000 \cdot \left(1 - \frac{(T_C + 288.9414)\,(T_C - 3.9863)^2}{508929.2\,(T_C + 68.12963)}\right) \qquad (27)$$

### 4.3. pH de la solución — Ec. `(38)`

Si el ion hidrógeno $\text{H(+a)}$ está incluido en la lista de especies (`pHcalc`):

$$\text{pH} = -\log_{10}(a_{\text{H}^+}) = -\log_{10}\left(\gamma_{\text{H}^+} \cdot m_{\text{H}^+}\right) \qquad (38)$$

---

## 5. Propiedades Coligativas: Congelación y Ebullición (`Colligcalc`) — Ecs. `(28)–(35)`

Partiendo de la igualdad de potenciales químicos del solvente líquido y sólido/vapor ($\mu_{\text{liq}}(T,P) = \mu_{\text{sol}}(T,P)$), la energía de fusión $\Delta G^{\text{fus}}_o = -R T \ln(a_{\text{liq}})$ (Ecs. `28–30`), la ecuación de Gibbs–Helmholtz:

$$\frac{d \ln(a_{\text{liq}})}{dT} = \frac{\Delta H_T^{\text{fus}}}{R T^2} \qquad (31)$$

y dependencia térmica lineal de la entalpía de cambio de fase respecto a $\theta_F = T_F - T$ o $\theta_B = T - T_B$:

$$\Delta H_T^{\text{fus}} = \Delta H_{0,T_F}^{\text{fus}} + \Delta C_P^{\text{fus}} (T - T_F) = \Delta H_{0,T_F}^{\text{fus}} - \left(C_P^{\text{liq}} - C_P^{\text{sol}}\right) \theta_F \qquad (32)$$

la integración exacta da la Ec. `(33)`:

$$R \ln(a_{\text{liq}}) = \Delta H_{0,T_F}^{\text{fus}} \left(\frac{1}{T_F} - \frac{1}{T_F - \theta_F}\right) + \Delta C_P^{\text{fus}} \left[ \ln\left(\frac{T_F - \theta_F}{T_F}\right) + \frac{\theta_F}{T_F - \theta_F} \right] \qquad (33)$$

Mediante expansión de Taylor de segundo orden del logaritmo, `Colligcalc` en `AquaDLL.dll` evalúa explícitamente la **depresión del punto de congelación** $\theta_F$ ($\text{FP} = T_{F,\text{agua}} - \theta_F$) y la **elevación del punto de ebullición** $\theta_B$ ($\text{BP} = T_{B,\text{agua}} + \theta_B$) mediante las expresiones cuadráticas `(34)–(35)` de Ge & Wang (2009), usando `CPPitzer` para obtener $\Delta C_P$ y $a_{\text{liq}}$ en $T_F$ y $T_B$:

$$\theta_F = \frac{\Delta H_{0,T_F}^{\text{fus}} - 2 R T_F \ln(a_{\text{liq}}) - \sqrt{ 2 \Delta C_P^{\text{fus}} T_F^2 R \ln(a_{\text{liq}}) + \left(\Delta H_{0,T_F}^{\text{fus}}\right)^2 }}{ 2 \cdot \left[ \frac{\Delta H_{0,T_F}^{\text{fus}}}{T_F} + 0.5\,\Delta C_P^{\text{fus}} - R \ln(a_{\text{liq}}) \right] } \qquad (34)$$

$$\theta_B = \frac{\Delta H_{0,T_B}^{\text{vap}} - 2 R T_B \ln(a_{\text{liq}}) - \sqrt{ 2 \Delta C_P^{\text{vap}} T_B^2 R \ln(a_{\text{liq}}) + \left(\Delta H_{0,T_B}^{\text{vap}}\right)^2 }}{ 2 \cdot \left[ \frac{\Delta H_{0,T_B}^{\text{vap}}}{T_B} + 0.5\,\Delta C_P^{\text{vap}} - R \ln(a_{\text{liq}}) \right] } \qquad (35)$$

Además de evaluar $\theta_F(T), \theta_B(T)$, HSC Aqua reporta `FP (aw at 25°C)` y `BP (aw at 25°C)` utilizando el valor de $a_w$ calculado a $25\,^\circ\text{C}$.

---

## 6. Modelo de Densidad de Soluciones Acuosas (`Density Calculator`)

A partir de las cabeceras, coeficientes y cita bibliográfica verificadas directamente en el archivo binario [Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls), la densidad de mezcla de electrolitos en solución acuosa emplea el modelo de **Laliberté & Cooper (2004)**:

1. **Fracción másica del agua y total de solutos:**
   $$w_{\text{tot}} = \sum_{i=1}^{N} w_i, \qquad w_{\text{H}_2\text{O}} = 1 - w_{\text{tot}}$$
2. **Densidad aparente del electrolito $i$ en función de la concentración total de soluto $(1 - w_{\text{H}_2\text{O}})$ y $T_C$ ($^\circ\text{C}$):**
   $$\rho_{\text{app},i}(w_{\text{tot}}, T_C) = \frac{\left(c_{0,i} w_{\text{tot}} + c_{1,i}\right)\,\exp\!\left(10^{-6}\,(T_C + c_{4,i})^2\right)}{w_{\text{tot}} + c_{2,i} + c_{3,i}\,T_C}$$
3. **Densidad total de la solución ($\text{kg/m}^3$) por aditividad de volúmenes específicos aparentes:**
   $$\frac{1}{\rho_{\text{sol}}} = \frac{w_{\text{H}_2\text{O}}}{\rho_w(T_C)} + \sum_{i=1}^{N} \frac{w_i}{\rho_{\text{app},i}(w_{\text{tot}}, T_C)}$$

*(Comprobación realizada contra la Fig. 7 del manual para $w_{\text{ZnSO}_4}=0.149, w_{\text{H}_2\text{SO}_4}=0.048$ a $25\,^\circ\text{C}$: usando $c_0\dots c_4$ de `Aqueous Solution Density.xls` se obtiene $\rho_{\text{sol}} = 1204.33\text{–}1204.48\text{ kg/m}^3$ frente a $1204.371594\text{ kg/m}^3$ reportado en HSC 9).*
