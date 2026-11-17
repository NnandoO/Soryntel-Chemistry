# MATHEMATICAL_SPECIFICATION — Reaction Equations Module

## 1. Fundamentos Termodinámicos y Polinomios de Capacidad Calorífica ($C_p$)

En la formulación clásica empleada en las bases de datos de Barin, Knacke, Kubaschewski y compilada en HSC Chemistry, la capacidad calorífica molar a presión constante $C_p(T)$ de una especie pura se expresa empíricamente mediante la **ecuación extendida de Kelley**:

$$C_p(T) = A + B \times 10^{-3} \, T + C \times 10^{5} \, T^{-2} + D \times 10^{-6} \, T^2 \quad [\text{J}/(\text{mol}\cdot\text{K}) \text{ o } \text{cal}/(\text{mol}\cdot\text{K})]$$

donde $A, B, C, D$ son constantes empíricas dependientes de la especie y del rango de temperatura $[T_1, T_2]$, y $T$ es la temperatura absoluta en Kelvin ($T > 0\,\text{K}$).

---

## 2. Deducción Analítica de las Funciones Termodinámicas de Estado

A partir de las relaciones fundamentales de Maxwell y definiciones termodinámicas a presión estándar constante ($P = 1\,\text{bar}$):

### 2.1. Integración Analítica por Tramos desde $T_{\text{ref}} = 298.15\,\text{K}$ (Hacia Arriba y Hacia Abajo)

En las bases de datos de HSC (ej. `Al2O3` desde $100\,\text{K}$ o `Mg` desde $20\,\text{K}$), una especie puede tener $M$ intervalos contiguos $[T_{1,0}, T_{2,0}], [T_{1,1}, T_{2,1}], \dots, [T_{1,M-1}, T_{2,M-1}]$, donde $H_f^\circ(298.15)$ y $S^\circ(298.15)$ se almacenan en el primer registro ($k=0$), mientras que los registros posteriores ($k \ge 1$) almacenan los saltos de transición $(\Delta H_{\text{tr}, k}, \Delta S_{\text{tr}, k})$ en su frontera inferior $T_{1,k}$.

Sea $k_{\text{ref}}$ el índice del intervalo que contiene a $T_{\text{ref}} = 298.15\,\text{K}$, y $k_T$ el índice del intervalo que contiene a $T$ (si $T < T_{1,0}$ se extrapola con $k_T = 0$; si $T > T_{2,M-1}$ se extrapola con $k_T = M-1$). Definimos las integrales definidas sobre un tramo $k$ entre dos temperaturas $T_a$ y $T_b$:

$$I_H^{(k)}(T_a, T_b) = \int_{T_a}^{T_b} C_p^{(k)}(\tau)\,d\tau = A_k(T_b - T_a) + \frac{B_k \times 10^{-3}}{2}(T_b^2 - T_a^2) - C_k \times 10^5 \left(\frac{1}{T_b} - \frac{1}{T_a}\right) + \frac{D_k \times 10^{-6}}{3}(T_b^3 - T_a^3)$$

$$I_S^{(k)}(T_a, T_b) = \int_{T_a}^{T_b} \frac{C_p^{(k)}(\tau)}{\tau}\,d\tau = A_k \ln\left(\frac{T_b}{T_a}\right) + B_k \times 10^{-3}(T_b - T_a) - \frac{C_k \times 10^5}{2}\left(\frac{1}{T_b^2} - \frac{1}{T_a^2}\right) + \frac{D_k \times 10^{-6}}{2}(T_b^2 - T_a^2)$$

Con esto, la evaluación exacta para cualquier $T > 0\,\text{K}$ es:
1. **Mismo intervalo ($k_T = k_{\text{ref}}$):**
   $$H(T) = H_f^\circ(298.15) + I_H^{(k_{\text{ref}})}(298.15, T), \qquad S(T) = S^\circ(298.15) + I_S^{(k_{\text{ref}})}(298.15, T)$$
2. **Hacia arriba ($k_T > k_{\text{ref}}$, ej. fusión de $\text{Mg}$ a $923\,\text{K}$):**
   $$H(T) = H_f^\circ(298.15) + I_H^{(k_{\text{ref}})}(298.15, T_{2,k_{\text{ref}}}) + \sum_{k=k_{\text{ref}}+1}^{k_T - 1}\left[\Delta H_{\text{tr}, k} + I_H^{(k)}(T_{1,k}, T_{2,k})\right] + \Delta H_{\text{tr}, k_T} + I_H^{(k_T)}(T_{1,k_T}, T)$$
   $$S(T) = S^\circ(298.15) + I_S^{(k_{\text{ref}})}(298.15, T_{2,k_{\text{ref}}}) + \sum_{k=k_{\text{ref}}+1}^{k_T - 1}\left[\Delta S_{\text{tr}, k} + I_S^{(k)}(T_{1,k}, T_{2,k})\right] + \Delta S_{\text{tr}, k_T} + I_S^{(k_T)}(T_{1,k_T}, T)$$
3. **Hacia abajo ($k_T < k_{\text{ref}}$, ej. $\text{Al}_2\text{O}_3$ o $\text{Mg}$ a $T < 298.15\,\text{K}$):**
   Se resta la integral desde $T$ hasta $298.15\,\text{K}$ y se restan los calores/entropías de transición de las fronteras cruzadas en sentido descendente:
   $$H(T) = H_f^\circ(298.15) - I_H^{(k_{\text{ref}})}(T_{1,k_{\text{ref}}}, 298.15) - \sum_{k=k_T+1}^{k_{\text{ref}}-1}\left[\Delta H_{\text{tr}, k+1} + I_H^{(k)}(T_{1,k}, T_{2,k})\right] - \Delta H_{\text{tr}, k_T+1} - I_H^{(k_T)}(T, T_{2,k_T})$$

### 2.2. Energía Libre de Gibbs $G(T)$
La función de Gibbs molar se calcula directamente a partir de la relación fundamental:
$$G(T) = H(T) - T \cdot S(T)$$

---

## 3. Termodinámica de Reacciones Químicas

Para una reacción química general balanceada estequiométricamente:
$$\sum_{i=1}^{N} \nu_i \, M_i = 0$$
donde $\nu_i < 0$ para reactivos y $\nu_i > 0$ para productos.

### 3.1. Magnitudes Estándar de Reacción
- **Entalpía de reacción:**
  $$\Delta H_r^\circ(T) = \sum_{i=1}^{N} \nu_i \, H_i(T)$$
- **Entropía de reacción:**
  $$\Delta S_r^\circ(T) = \sum_{i=1}^{N} \nu_i \, S_i(T)$$
- **Energía libre de Gibbs de reacción:**
  $$\Delta G_r^\circ(T) = \sum_{i=1}^{N} \nu_i \, G_i(T) = \Delta H_r^\circ(T) - T \cdot \Delta S_r^\circ(T)$$

### 3.2. Constante Termodinámica de Equilibrio $K(T)$
La relación entre la energía libre de Gibbs estándar y la constante de equilibrio viene dada por el teorema de van 't Hoff:
$$\Delta G_r^\circ(T) = -R \cdot T \cdot \ln(K)$$

Despejando la constante $K$:
$$K(T) = \exp\left( -\frac{\Delta G_r^\circ(T)}{R \cdot T} \right)$$
$$\log_{10}(K(T)) = \frac{\ln(K(T))}{\ln(10)} = -\frac{\Delta G_r^\circ(T)}{2.302585092994046 \cdot R \cdot T}$$

donde la constante universal de los gases se adopta con máxima precisión:
- En unidades SI: $R = 8.31446261815324\,\text{J}/(\text{mol}\cdot\text{K})$ (CODATA 2018/2022).
- En calorías termoquímicas (compatibilidad legacy HSC): $R \approx 1.987204\,\text{cal}/(\text{mol}\cdot\text{K})$ (con $1\,\text{cal} = 4.184\,\text{J}$ exactos).

### 3.3. Potencial Electroquímico Estándar ($E^\circ$ vs SHE)
Para semirreacciones redox que transfieren $n_e$ moles de electrones (cuando se introduce el símbolo `e-` con coeficiente estequiométrico $n_e$):
Por la relación de Nernst-Faraday:
$$\Delta G_r^\circ(T) = -n_e \cdot F \cdot E^\circ(T)$$

Despejando el potencial estándar de reducción/oxidación:
$$E^\circ(T) = -\frac{\Delta G_r^\circ(T)}{n_e \cdot F}$$

donde:
- $F = 96485.33212\,\text{C}/\text{mol}$ (Constante de Faraday).
- Signo: Sigue estrictamente la convención de la **IUPAC (Convención de Estocolmo, 1953)**, donde el potencial de reducción estándar para el electrodo de hidrógeno $2\text{H}^+ + 2e^- = \text{H}_2(g)$ es $E^\circ \equiv 0.000\,\text{V}$ a cualquier temperatura de referencia.

---

## 4. Modelo de Extrapolación de Criss-Cobble para Iones Acuosos

Para especies acuosas diluidas a temperaturas elevadas ($T > 298.15\,\text{K}$ hasta $573.15\,\text{K}$ / $300\,^\circ\text{C}$):
El principio fundamental del modelo de **Criss y Cobble (1964)** postula que la entropía absoluta media molar de un ion acuoso a cualquier temperatura $T$ está linealmente correlacionada con su entropía absoluta a $298.15\,\text{K}$ a través de dos parámetros de ajuste térmico:

$$\bar{S}^\circ_{\text{abs}}(T) = a(T) + b(T) \cdot \bar{S}^\circ_{\text{abs}}(298.15)$$

### 4.1. Conversión a la Escala de Entropía Absoluta del Tercer Principio
La escala convencional de termodinámica en solución fija $S^\circ(\text{H}^+, 298.15) = 0$.
Criss y Cobble demostraron que para normalizar los coeficientes lineales, la entropía "absoluta" del ion $\text{H}^+(aq)$ debe fijarse en:
$$S^\circ_{\text{abs}}(\text{H}^+, 298.15) = -5.0\,\text{cal}/(\text{mol}\cdot\text{K}) = -20.92\,\text{J}/(\text{mol}\cdot\text{K})$$

Por lo tanto, para cualquier ion con carga $z$:
$$\bar{S}^\circ_{\text{abs}}(298.15) = \bar{S}^\circ_{\text{conv}}(298.15) - 5.0 \cdot z \quad [\text{cal}/(\text{mol}\cdot\text{K})]$$

### 4.2. Capacidad Calorífica Media Extrapolada
A partir de la definición de entropía media en el intervalo $[298.15, T]$:
$$\left. \bar{C}_p \right|_{298.15}^{T} = \frac{\bar{S}^\circ(T) - \bar{S}^\circ(298.15)}{\ln(T / 298.15)}$$

A partir de $\left. \bar{C}_p \right|_{298.15}^{T}$, las correcciones de entalpía y energía de Gibbs se calculan como:
$$\Delta H(T) = \Delta H(298.15) + \left. \bar{C}_p \right|_{298.15}^{T} \cdot (T - 298.15)$$
$$\Delta G(T) = \Delta H(T) - T \cdot \bar{S}^\circ(T)$$

---

## 5. Consideraciones de Precisión Numérica en Punto Flotante (`f64`)

1. **Catástrofe de Cancelación Numérica en $\Delta G = \Delta H - T \Delta S$:**
   - Cuando $|\Delta H| \approx |T \Delta S|$, la resta directa en punto flotante puede perder dígitos de precisión significativa.
   - En el motor `soryntel-thermo`, los cálculos se preservan rigurosamente en formato IEEE 754 de 64 bits (`f64`, 53 bits de mantisa $\approx 15-17$ cifras decimales exactas).
2. **Overflow y Underflow en Constantes de Equilibrio $K = \exp(-\Delta G / RT)$:**
   - Para reacciones termodinámicamente imposibles ($\Delta G \gg 0$), $K \to 0$ (underflow). Se satura de forma segura a subnormales o $0.0$.
   - Para reacciones sumamente espontáneas ($\Delta G \ll 0$, ej. $K = 10^{47}$), $\exp(x)$ excede el límite del flotante estándar si $x > 709.78$.
   - **Solución Algorítmica en Rust:** Se evalúa y almacena primordialmente el logaritmo común $\log_{10}(K) = -\Delta G / (RT \ln 10)$, el cual nunca produce overflow para magnitudes termodinámicas reales, y $K$ se presenta formateado en notación científica a partir de la mantisa y exponente enteros:
     $$\log_{10} K = e + m, \quad m \in [0, 1) \implies K = 10^m \times 10^e$$
