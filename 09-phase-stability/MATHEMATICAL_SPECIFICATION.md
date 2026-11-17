# Mathematical Specification — Módulo 09: Phase Stability Diagrams

## 1. Sistema ternario y gases de referencia
Sea un sistema de **3 elementos** $\{M, A, B\}$ donde:
- $M$ es el elemento principal (presente en todas las fases condensadas candidatas $i$: $M_{m_i} A_{a_i} B_{b_i}$, $m_i > 0$).
- $G_1, G_2$ son dos especies gaseosas que **no contienen $M$**, con estequiometría $(\nu_{1,A}, \nu_{1,B})$ y $(\nu_{2,A}, \nu_{2,B})$.

La matriz de composición gaseosa es:
$$C = \begin{pmatrix} \nu_{1,A} & \nu_{2,A} \\ \nu_{1,B} & \nu_{2,B} \end{pmatrix}$$
Se exige $\det(C) = \nu_{1,A}\nu_{2,B} - \nu_{2,A}\nu_{1,B} \neq 0$ (por ejemplo, `SO2(g)` + `O2(g)` en `Cu-S-O`, o `Cl2(g)` + `O2(g)` en `Ti-Cl-O`).

Para formar 1 mol de la fase condensada $i$ desde $m_i$ moles de $M$ y los gases $G_1, G_2$:
$$m_i\,M + \alpha_i\,G_1 + \beta_i\,G_2 \longrightarrow M_{m_i} A_{a_i} B_{b_i}$$
donde:
$$\begin{pmatrix} \alpha_i \\ \beta_i \end{pmatrix} = C^{-1} \begin{pmatrix} a_i \\ b_i \end{pmatrix}$$

## 2. Energía libre de Gibbs normalizada (por mol de $M$)
A temperatura $T$ (K) y presiones parciales $p_1, p_2$ (bar, estado estándar $1\text{ bar}$):
$$\mu_{G_k}(T, p_k) = G_{G_k}^\circ(T) + R\,T\,\ln(10)\,\log_{10}(p_k), \quad k \in \{1, 2\}$$
La energía libre normalizada de la fase pura $i$ (actividad unitaria $a_i = 1$) respecto a $\{M, G_1, G_2\}$ es:
$$g_i(T, \log_{10} p_1, \log_{10} p_2) = \frac{1}{m_i}\Bigl[ G_i^\circ(T) - \alpha_i\,\mu_{G_1}(T, p_1) - \beta_i\,\mu_{G_2}(T, p_2) \Bigr]$$

*(Nota: como $C$ balancea todos los átomos $A$ y $B$, y $m_i$ normaliza $M$, los términos de entropía de referencia elemental de $G^\circ(T) = H(T) - T S^\circ(T)$ se cancelan exactamente entre todas las fases $i$ sin requerir correcciones adicionales).*

## 3. Criterio de predominancia (Tpp & Lpp)
En cada punto $(x, y)$ del plano:
- **Modo `p-p` isotérmico** ($T$ fijo, $x = \log_{10} p_1$, $y = \log_{10} p_2$):
  $$i^*(x, y) = \arg\min_i g_i(T, x, y)$$
- **Modo `T-p`** ($p_1$ fijo, $x = T$, $y = \log_{10} p_2$):
  $$i^*(x, y) = \arg\min_i g_i(x, \log_{10} p_1, y)$$

Las fronteras de fase corresponden a $g_i(x, y) = g_j(x, y)$ y se extraen mediante contorno dual con refinamiento por bisección.
