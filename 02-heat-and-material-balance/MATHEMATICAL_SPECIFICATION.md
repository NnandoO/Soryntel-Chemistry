# Especificación Matemática y Numérica — Módulo 2: Heat and Material Balance

## 1. Propiedades Termodinámicas de Especies y Corrientes

### 1.1 Reutilización del Módulo 1 (`soryntel-thermo`)
Para toda especie $i$ de la base de datos (`Maindb6.HSC`) evaluada a temperatura $T$ (en Kelvin, $T = T_{^\circ\text{C}} + 273.15$) y presión estándar ($P = 1\text{ bar}$ o $1\text{ atm}$), se reutiliza directamente `evaluate_species_si(species, t_k, ...)` del Módulo 1, que integra el polinomio de Kelley por segmentos:
$$C_{p,i}(T) = A_s + B_s \cdot 10^{-3} T + C_s \cdot 10^{5} T^{-2} + D_s \cdot 10^{-6} T^2 \quad [\text{J/(mol}\cdot\text{K)}]$$

A partir de $H_i(T)$ y $S_i(T)$ en unidades SI ($\text{kJ/mol} \equiv \text{MJ/kmol}$):
1. **Entalpía Total molar (`Tot H / kmol`):**
   $$h_{\text{tot}, i}(T) = H_i(T) = H_{f,i}^\circ(298.15\text{ K}) + \int_{298.15}^{T} C_{p,i}(T')\,dT' + \sum_{298.15 < T_{\text{tr}} \le T} \Delta H_{\text{tr}, i}$$
2. **Contenido de Calor molar (`Heat Content / kmol`):**
   $$h_{\text{cont}, i}(T) = H_i(T) - H_i(298.15\text{ K})$$
   Esta cantidad representa exclusivamente el calor sensible más los calores latentes de transición de fase respecto a $298.15\text{ K}$ ($25\,^\circ\text{C}$), por lo que $h_{\text{cont}, i}(298.15\text{ K}) = 0$.
3. **Propiedades extensivas de la fila ($n_i$ en $\text{kmol}$):**
   $$H_{\text{cont}, i} = n_i \cdot h_{\text{cont}, i}(T), \qquad H_{\text{tot}, i} = n_i \cdot h_{\text{tot}, i}(T)$$
   $$TS_i = n_i \cdot T \cdot S_i(T), \qquad G_i = n_i \cdot \big(H_i(T) - T \cdot S_i(T)\big)$$

### 1.2 Factores de Conversión de Unidades
Confirmados en el manual (Sección 11.1) y en `soryntel-core`:
- $1\text{ kcal}_{\text{th}} = 4.184\text{ kJ}$ (`CAL_TO_JOULE = 4.184`)
- $1\text{ Mcal}_{\text{th}} = 4.184\text{ MJ}$
- $1\text{ kWh} = 3.6\text{ MJ} = \frac{3.6}{4.184}\text{ Mcal}_{\text{th}} \approx 0.86042065\text{ Mcal}_{\text{th}}$ (el manual redondea a $0.8604$).
- Volumen normal de gas ideal (`Nm³` a $0\,^\circ\text{C}$, $1\text{ atm}$): $V_{\text{molar}} = 22.414\text{ Nm}^3/\text{kmol}$ (`STP_MOLAR_VOL_L`).

---

## 2. Corrección de Presión para $\text{H}_2\text{O}$ (`H2O` / `H2O(g)`)

En `HSC6.DLL` se identificaron `H_H2O`, `S_H2O`, `CP_H2O`, `G_H2O` y `Critical_H2O`, y en el manual (Sección 11.1 y `H2O.OHE`) la opción `Pressure correction H2O`.
- Cuando esta corrección está activa para una fila `H2O` o `H2O(g)` a $(T, P)$:
  1. Se evalúa la propiedad real del agua/vapor mediante la formulación **IAPWS-IF97** (regiones 1 líquido comprimido, 2 vapor sobrecalentado, 3 región crítica, 4 curva de saturación $P_{\text{sat}}(T)$, 5 alta temperatura), o **IAPWS-95**.
  2. Para mantener consistencia exacta con la escala termoquímica estándar de HSC ($H_f^\circ(\text{H}_2\text{O(l)}, 298.15\text{ K}, 1\text{ bar}) = -285.830\text{ kJ/mol}$), la entalpía absoluta se referencia al estado líquido estándar a $298.15\text{ K}$:
     $$h_{\text{tot}, \text{H}_2\text{O}}(T, P) = H_f^\circ(\text{H}_2\text{O(l)}, 298.15) + \big(h_{\text{IF97}}(T, P) - h_{\text{IF97}}(298.15\text{ K}, 1\text{ bar})\big)$$
  3. Verificación con `H2O.OHE`: $0.060\text{ kmol}$ de $\text{H}_2\text{O}$ a $25\,^\circ\text{C}$ tienen $H_{\text{tot}} = -17.150\text{ MJ}$ ($-17.150 / 0.060 = -285.833\text{ MJ/kmol}$), y al evaporar a $100\,^\circ\text{C}$ ($1.01325\text{ bar}$) como $\text{H}_2\text{O(g)}$ absorben $2.793\text{ MJ}$ ($H_{\text{tot}} = -14.357\text{ MJ}$).

---

## 3. Cálculo de Exergía Física y Química (`Ex Phys`, `Exergy`)

Observado en `Total Balance` (`raw-001.png`, `raw-027.png`) y hoja `BALANCE`:
1. **Exergía Física ($Ex_{\text{phys}}$):**
   Respecto al estado muerto ambiental $(T_0 = 298.15\text{ K}, P_0 = 1.01325\text{ bar})$:
   $$ex_{\text{phys}, i}(T, P) = \big(h_i(T, P) - h_i(T_0, P_0)\big) - T_0 \big(s_i(T, P) - s_i(T_0, P_0)\big)$$
   Para un gas ideal: $s_i(T, P) - s_i(T_0, P_0) = s_i^\circ(T) - s_i^\circ(T_0) - R \ln(P/P_0)$.
2. **Exergía Química ($Ex_{\text{chem}}$) y Exergía Total ($Ex$):**
   $$ex_{\text{tot}, i} = ex_{\text{phys}, i} + ex_{\text{chem}, i}^\circ$$
   donde $ex_{\text{chem}, i}^\circ$ se calcula a partir de la energía libre estándar de formación $\Delta G_f^\circ(298.15\text{ K})$ y las exergías químicas estándar de los elementos de referencia ambiental de Szargut et al. (1988):
   $$ex_{\text{chem}, i}^\circ = \Delta G_{f,i}^\circ(298.15\text{ K}) + \sum_e \nu_{i,e} \, ex_{\text{chem}, e}^\circ$$
   La columna `Exergy` del balance global es $\Delta Ex = \sum_{\text{OUT}} n_j ex_{\text{tot}, j} - \sum_{\text{IN}} n_i ex_{\text{tot}, i}$ (más trabajo/calor si aplica), siendo negativa en procesos irreversibles espontáneos (ej. `CUCONV1`: `-147.526 MJ` en `raw-001.png`).

---

## 4. Ecuación de Pérdidas de Calor por Convección Natural

Documentada en Sección 11.1 del manual:
$$H_{\text{loss}} = (6.8 + 0.046 \cdot T_2) \cdot (T_2 - T_1) \cdot A \quad [\text{kcal/h}]$$
Convertida a unidades SI ($\text{W}$):
$$Q_{\text{loss}}\,[\text{W}] = \frac{4184}{3600} \cdot (6.8 + 0.046 \cdot T_2) \cdot (T_2 - T_1) \cdot A$$

---

## 5. Algoritmos Numéricos para Resolución en Rust

### 5.1 `Temp. Balance` (Temperatura Adiabática de Productos)
Buscamos $T^* \in [T_{\min}, T_{\max}]$ (por ejemplo $[200\text{ K}, 6000\text{ K}]$) tal que:
$$F(T) = \sum_{j \in \text{OUT}_k} n_j \, h_{\text{tot}, j}(T) + Q_{\text{losses}} - \left(\sum_{i \in \text{IN}_k} n_i \, h_{\text{tot}, i}(T_i) + Q_{\text{extra}}\right) = 0$$
- **Propiedad matemática:** Dado que $C_{p,j}(T) > 0$ y $\Delta H_{\text{tr},j} \ge 0$ al aumentar $T$, $F(T)$ es **estrictamente creciente** (monótona), aunque continua a trozos por saltos de transición de fase si una especie cambia de fase (donde $H$ tiene una discontinuidad de salto $\Delta H_{\text{tr}}$).
- **Método numérico recomendado:** **Método de Brent** (bisección + secante + interpolación cuadrática inversa) con expansión automática de corchete $[T_{\text{low}}, T_{\text{high}}]$ y tolerancia $|F(T)| < 10^{-9}\text{ MJ}$ o $|T_{\text{high}} - T_{\text{low}}| < 10^{-6}\text{ K}$.
- **Manejo de transiciones de fase:** Si $F(T_{\text{tr}}^-) < 0 < F(T_{\text{tr}}^+)$, la temperatura adiabática cae exactamente en la temperatura de transición $T_{\text{tr}}$ (ej. ebullición/fusión parcial a temperatura constante, como $100\,^\circ\text{C}$ en `H2O.OHE`!). Brent converge directamente a $T_{\text{tr}}$.

### 5.2 `Target Dialog` (Goal Seek Acotado y Acoplado)
Para cada fila $r$ con variable $x_r \in [a_r, b_r]$, objetivo $g_r(x_r) = y_r^*$:
1. **Caso 1D (fila individual):** Definimos el residuo $\phi_r(x) = g_r(x) - y_r^*$.
   - Primero evaluamos los extremos $\phi_r(a_r)$ y $\phi_r(b_r)$ y el valor actual $\phi_r(x_0)$.
   - Si hay cambio de signo en $[a_r, b_r]$, aplicamos **Brent** garantizado dentro de $[a_r, b_r]$ con tolerancia $|\phi_r(x)| < 10^{-8}$.
   - Si no hay cambio de signo (p. ej. región no acotada o mínimo local), aplicamos búsqueda por **secante amortiguada / Nelder-Mead 1D** proyectada en $[a_r, b_r]$ para minimizar $|\phi_r(x)|$.
2. **Caso acoplado (`Iterate All` con múltiples filas $r=1\dots m$):**
   - **Enfoque HSC:** Gauss-Seidel no lineal (iterar secuencialmente cada fila $1 \dots m$ hasta que $\max_r |g_r(\mathbf{x}) - y_r^*| < \varepsilon$ o alcanzar `Max Iterations`).
   - **Mejora en Soryntel:** Si el sistema $m \times m$ está fuertemente acoplado y Gauss-Seidel oscila, ofrecer respaldo automático con **método de Broyden acotado** (`argmin` / `nalgebra`) sobre el vector $\mathbf{x} \in \prod_r [a_r, b_r]$.

### 5.3 Convergencia de Hojas con Referencias Circulares (Flujos de Retorno)
En diagramas multi-área con reciclaje (como `FSF_process.bal8`: polvos de `OUT4` y `OUT5` retornando a `IN1`):
- Sea $\mathbf{z} \in \mathbb{R}^p$ el vector de celdas involucradas en ciclos detectados mediante análisis de componentes fuertemente conexas (Tarjan SCC) en el grafo de dependencias de fórmulas.
- **Paso 1:** Evaluar en orden topológico todas las celdas acíclicas previas.
- **Paso 2:** Para cada SCC cíclica, resolver $\mathbf{z} = \mathcal{G}(\mathbf{z})$ mediante **sustitución sucesiva con aceleración de Anderson** (o Aitken $\Delta^2$ en ciclos 1D) hasta $\|\mathbf{z}^{(k+1)} - \mathbf{z}^{(k)}\|_\infty < 10^{-9}$, con respaldo de Broyden si el radio espectral está cercano a 1.
- **Paso 3:** Evaluar las celdas acíclicas posteriores (ej. `BALANCE` y `Total Balance`).
