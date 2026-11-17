# Especificación Matemática — Módulo 16: Species & Thermochemical Databases

## 1. Modelo Termoquímico por Intervalos de Temperatura (Kelley)

Cada especie $i$ está representada por $M$ intervalos contiguos $[T_{1,k}, T_{2,k}]$ ($k = 1, \dots, M$, con $T_{1,k+1} = T_{2,k}$ en una secuencia continua de fases). Dentro del intervalo $k$, la capacidad calorífica isobárica se parametriza mediante la ecuación de **Maier–Kelley** estándar de HSC6 (`A, B, C, D` en escala calórica dentro del archivo `.HSC`, convertidos mediante $1\text{ cal}_{th} = 4.184\text{ J}$):

$$
C_{p,k}(T) = A_k + B_k \cdot 10^{-3} T + C_k \cdot 10^{5} T^{-2} + D_k \cdot 10^{-6} T^2 \quad \left[\frac{\text{cal}}{\text{mol}\cdot\text{K}}\text{ o }\frac{\text{J}}{\text{mol}\cdot\text{K}}\right]
$$

### 1.1 Integración de Entalpía $H(T)$ y Entropía $S(T)$ entre Intervalos
- **En el intervalo base ($k = 1$):** el archivo almacena $H_1 = \Delta_f H^\circ(T_{ref})$ y $S_1 = S^\circ(T_{ref})$ evaluados a $T_{ref} = T_{1,1}$ (usualmente $298.15\text{ K}$).
- **En intervalos subsiguientes ($k \ge 2$):** el archivo almacena los saltos de transición en $T_{tr,k} = T_{1,k}$: $\Delta_{tr}H_k$ y $\Delta_{tr}S_k$. Para una transición de fase en equilibrio isotérmico e isobárico debe cumplirse:
  $$
  \Delta_{tr}S_k = \frac{\Delta_{tr}H_k}{T_{tr,k}}
  $$
  Si un rango largo de una misma fase fue subdividido por complejidad de $C_p(T)$, entonces $\Delta_{tr}H_k = 0$ y $\Delta_{tr}S_k = 0$.

Para cualquier temperatura $T$ dentro del intervalo activo $m$ ($T_{1,m} \le T \le T_{2,m}$):

$$
H(T) = H_1 + \sum_{k=1}^{m-1} \left[\Delta H_{seg,k}(T_{1,k}, T_{2,k}) + \Delta_{tr}H_{k+1}\right] + \Delta H_{seg,m}(T_{1,m}, T)
$$

$$
S(T) = S_1 + \sum_{k=1}^{m-1} \left[\Delta S_{seg,k}(T_{1,k}, T_{2,k}) + \Delta_{tr}S_{k+1}\right] + \Delta S_{seg,m}(T_{1,m}, T)
$$

donde las integrales analíticas sobre el segmento $[T_a, T_b]$ son:

$$
\Delta H_{seg,k}(T_a, T_b) = \int_{T_a}^{T_b} C_{p,k}(T)\,dT = A_k (T_b - T_a) + \frac{B_k \cdot 10^{-3}}{2}(T_b^2 - T_a^2) - C_k \cdot 10^{5}\left(\frac{1}{T_b} - \frac{1}{T_a}\right) + \frac{D_k \cdot 10^{-6}}{3}(T_b^3 - T_a^3)
$$

$$
\Delta S_{seg,k}(T_a, T_b) = \int_{T_a}^{T_b} \frac{C_{p,k}(T)}{T}\,dT = A_k \ln\frac{T_b}{T_a} + B_k \cdot 10^{-3}(T_b - T_a) - \frac{C_k \cdot 10^{5}}{2}\left(\frac{1}{T_b^2} - \frac{1}{T_a^2}\right) + \frac{D_k \cdot 10^{-6}}{2}(T_b^2 - T_a^2)
$$

Y la energía libre de Gibbs molar resulta:

$$
G(T) = H(T) - T \cdot S(T)
$$

> **Extrapolación de fase líquida separada (`X(l)`, `28.2` y `28.11 B`):** Cuando se crea un registro independiente para la fase líquida (p. ej. `Cu2S(l)` o extrapolación automática de `X` con $T_{fus} = T_{1,liq}$), sus valores de referencia a $298.15\text{ K}$ deben retro-integrarse usando el $C_{p,liq}(T)$ hacia abajo desde $T_{fus}$ hasta $298.15\text{ K}$ para que $\Delta_{tr}H = 0$ y $\Delta_{tr}S = 0$ en el primer rango $[298.15, T_{max}]$:
> $$H_{liq}(298.15) = H_{sol}(T_{fus}^-) + \Delta_{fus}H - \Delta H_{seg,liq}(298.15, T_{fus})$$
> $$S_{liq}(298.15) = S_{sol}(T_{fus}^-) + \Delta_{fus}S - \Delta S_{seg,liq}(298.15, T_{fus})$$

---

## 2. Especies Acuosas (`ao`, `ai`) y Extrapolación de Criss–Cobble (`28.3–28.4`)

### 2.1 Convención de Estado de Referencia Acuoso ($298.15\text{ K}$, $1\text{ bar}$, $1\text{ mol/kg H}_2\text{O}$)
- La escala convencional fija para el protón hidratado a toda temperatura:
  $$
  \Delta_f H^\circ(\text{H}^+(a), T) = 0, \qquad S^\circ(\text{H}^+(a), T) = 0, \qquad C_p^\circ(\text{H}^+(a), T) = 0
  $$
- **Conversión a escala absoluta (Tercera Ley)** para aplicar Criss–Cobble:
  $$
  S_{abs}^\circ(298.15\text{ K}) = S_{conv}^\circ(298.15\text{ K}) - 5.0 \cdot z \quad \left[\frac{\text{cal}}{\text{mol}\cdot\text{K}}\right]
  $$
  (donde $-5.0\text{ cal/(mol}\cdot\text{K)}$ corresponde a la entropía absoluta experimental de $\text{H}^+(aq)$ a $25\,^\circ\text{C}$).
- **Conversión de entropía de mezcla a escala de fracción molar** (para equilibrio en solución acuosa diluida, $55.51\text{ mol H}_2\text{O/kg}$):
  $$
  \Delta S_{mix} = R \ln x_i = 1.9872 \cdot \ln\left(\frac{1}{55.51}\right) \approx -7.981\ \frac{\text{cal}}{\text{mol}\cdot\text{K}}
  $$

### 2.2 Extrapolación de Criss–Cobble ($25\,^\circ\text{C} \le T \le 300\,^\circ\text{C}$)
Se activa **únicamente** cuando un ion acuoso carece de coeficientes dependientes de $T$ ($B_1 = C_1 = D_1 = 0$):
1. A partir de $S_{abs}^\circ(298.15)$ se obtiene la capacidad calorífica media de correspondencia $\overline{C_p^\circ}]_{298}^{T} = a(T) + b(T) S_{abs}^\circ(298.15)$ usando las constantes empíricas de Criss–Cobble por clase de ion (cationes simples, aniones simples, oxianiones, ácidos oxigenados).
2. Integración en $T$: $H(T) \approx H(298.15) + \overline{C_p^\circ}(T)(T - 298.15)$ y $S(T) \approx S(298.15) + \overline{C_p^\circ}(T)\ln(T / 298.15)$.

### 2.3 Electrolitos Neutros `ai` (`ia)`) vs Especie Molecular `ao` (`a)`)
Los electrolitos `ai` tienen entalpía y entropía aditivas respecto a sus iones completamente disociados, p. ej.:
$$
H(\text{MgSO}_4(\text{ia})) \approx H(\text{Mg}^{+2}\text{a}) + H(\text{SO}_4^{-2}\text{a})
$$
Mientras que las especies moleculares no disociadas `ao` (`MgSO4(a)`, `HF(a)`) poseen datos experimentales independientes de la disociación y son las recomendadas por HSC6 para cálculos de equilibrio.

---

## 3. Ajuste de Datos de Capacidad Calorífica (`Cp Fitting`, `28.12`)

Dado un conjunto experimental de $N$ pares $\{(T_i, C_{p,i}^{\text{exp}})\}_{i=1}^N$ dentro de un intervalo $[T_1, T_2]$, y un vector de máscara booleana $\mathbf{m} = (m_A, m_B, m_C, m_D) \in \{0,1\}^4$ (donde `N` fija el parámetro en $0$):

Se construye el sistema de diseño lineal $\mathbf{\Phi} \boldsymbol{\theta} \approx \mathbf{y}$ restringido a las columnas activas ($m_j = 1$):

$$
\phi_A(T_i) = 1, \quad \phi_B(T_i) = 10^{-3} T_i, \quad \phi_C(T_i) = 10^{5} T_i^{-2}, \quad \phi_D(T_i) = 10^{-6} T_i^2
$$

Se resuelve por **mínimos cuadrados ordinarios** vía descomposición QR / ecuaciones normales estabilizadas:

$$
\min_{\boldsymbol{\theta}} \sum_{i=1}^{N} \left( C_{p,i}^{\text{exp}} - \mathbf{\Phi}_i \boldsymbol{\theta} \right)^2 \implies \hat{\boldsymbol{\theta}} = (\mathbf{\Phi}^\top \mathbf{\Phi})^{-1} \mathbf{\Phi}^\top \mathbf{y}
$$

y se reporta el diagnóstico punto a punto:

$$
\text{Error}_i\,(\%) = \frac{C_{p}^{\text{fit}}(T_i) - C_{p,i}^{\text{exp}}}{C_{p,i}^{\text{exp}}} \times 100
$$

---

## 4. Conversión de Funciones $C_p$ Externas a Formato HSC (`28.13`)

Cuando la literatura provee un polinomio externo $f_{\text{ext}}(T)$ con unidades arbitrarias (`J/(kmol·K)`, `BTU/(lb·°F)`, etc.) en $[T_1, T_2]$:
1. Se particiona $[T_1, T_2]$ uniformemente en $K \ge 10$ subintervalos ($K+1 = 11$ nodos: $T_i = T_1 + \frac{i}{K}(T_2 - T_1)$).
2. Se evalúa $f_{\text{ext}}(T_i)$ en cada nodo aplicando el factor de escala $u$ a `cal/(mol·K)` o `J/(mol·K)`.
3. Se aplica el algoritmo de **Mínimos Cuadrados Kelley (`§3`)** sobre los puntos generados para obtener $(A, B, C, D)$ con su error porcentual máximo.

---

## 5. Conversión de Funciones $G(T)$ o $\Delta_r G(T)$ a $(H, S, A, B, C, D)$ (`28.14`)

El módulo convierte expresiones polinómicas/logarítmicas estándar de energía de Gibbs (formato HSC / Barin / Kubaschewski, en $\text{kcal/mol}$ cuando está seleccionada la escala calórica, análogo en $\text{kJ/mol}$):

$$
G(T) = h + s_0\,T + a_0\,T \ln T + b_0\,T^2 + c_0\,T^{-1} + d_0\,T^3
$$

donde los campos de entrada en pantalla están escalados como $s_0 = s\cdot 10^{-3}$, $a_0 = a\cdot 10^{-3}$, $b_0 = b\cdot 10^{-6}$, $c_0 = c$ (en $\text{kcal}\cdot\text{K/mol}$ o $\text{kJ}\cdot\text{K/mol}$) y $d_0 = d\cdot 10^{-9}$.

Aplicando las identidades termodinámicas fundamentales:

$$
S(T) = -\left(\frac{\partial G}{\partial T}\right)_P = -s_0 - a_0(1 + \ln T) - 2 b_0\,T + c_0\,T^{-2} - 3 d_0\,T^2
$$

$$
H(T) = G(T) + T\,S(T) = h - a_0\,T - b_0\,T^2 + 2 c_0\,T^{-1} - 2 d_0\,T^3
$$

$$
C_p(T) = \left(\frac{\partial H}{\partial T}\right)_P = -a_0 - 2 b_0\,T - 2 c_0\,T^{-2} - 6 d_0\,T^2
$$

Comparando directamente con la forma de Maier–Kelley $C_p(T) = A + B\cdot 10^{-3}T + C\cdot 10^5 T^{-2} + D\cdot 10^{-6}T^2$ (multiplicando por $10^3$ para pasar de $\text{kcal}$ a $\text{cal}$ o de $\text{kJ}$ a $\text{J}$):

$$
A = -a, \qquad B = -2\,b, \qquad C = -2 \cdot 10^{-2}\,c, \qquad D = -6 \cdot 10^{-3}\,d
$$

Y evaluados a $T_0 = 298.15\text{ K}$: $\Delta H = H(T_0)\ [\text{kcal/mol}]$, $\Delta S = 10^3\,S(T_0)\ [\text{cal/(mol}\cdot\text{K)}]$, $\Delta G = G(T_0)\ [\text{kcal/mol}]$.

### 5.1 Operación `Delta to Normal` (para $\Delta_f G(T)$ de Formación)
Si la expresión ingresada corresponde a $\Delta_f G(T)$ para una fórmula objetivo $X$, el sistema construye automáticamente la reacción balanceada de formación desde los **elementos de referencia en estado estándar** ($\sum_e \nu_e E_e \to X$, p. ej. $2\text{Cu} + \text{S} \to \text{Cu}_2\text{S}$) consultando `MainDB`/`OwnDB`:

$$
H(X) = \Delta_f H + \sum_{e} \nu_e H(E_e), \qquad S(X) = \Delta_f S + \sum_{e} \nu_e S(E_e), \qquad \mathbf{C}_p(X) = \Delta_f \mathbf{C}_p + \sum_{e} \nu_e \mathbf{C}_p(E_e)
$$

donde la suma sobre $(A, B, C, D)$ se efectúa componente a componente multiplicando por la estequiometría $\nu_e$ de cada elemento.
