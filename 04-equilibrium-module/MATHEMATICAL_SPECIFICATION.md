# Especificación Matemática y Numérica — Módulo 4: Equilibrium Calculations

## 1. Formulación del Problema de Minimización de Energía de Gibbs (GEM)

Sea un sistema con $M$ elementos químicos (más balance de carga $e^-$ si hay iones acuosos) y $N$ especies distribuidas en:
- Fases mezcla ($\alpha \in \mathcal{P}_{\text{mix}}$): gas ideal, solución acuosa o mezclas líquidas/sólidas, con conjunto de especies $I_\alpha$ y moles totales $N_\alpha = \sum_{i \in I_\alpha} n_i$.
- Fases puras condensadas ($\beta \in \mathcal{P}_{\text{pure}}$): cada especie pura $k \in I_{\text{pure}}$ tiene fracción molar unitaria ($x_k = 1$, $a_k = 1$ si $n_k > 0$).

El problema primal es:
$$\min_{n \ge 0} \; \frac{G(n, T, P)}{R T} = \sum_{\alpha \in \mathcal{P}_{\text{mix}}} \sum_{i \in I_\alpha} n_i \left( \frac{\mu_i^\circ(T, P)}{R T} + \ln \gamma_i + \ln \frac{n_i}{N_\alpha} + \Delta_{\text{mix},i} \right) + \sum_{k \in I_{\text{pure}}} n_k \frac{\mu_k^\circ(T, P)}{R T}$$
sujeto a las restricciones de conservación elemental (y electroneutralidad):
$$\sum_{i=1}^{N} a_{ei} \, n_i = b_e, \qquad e = 1, \dots, M, \qquad n_i \ge 0.$$

### 1.1 Potenciales Químicos Estándar $\mu_i^\circ(T, P)$
Reutilizando `evaluate_point_si` de `soryntel-thermo`:
- Para todas las especies: $G_i^\circ(T) = H_i^\circ(T) - T S_i^\circ(T)$ (en $\text{J/mol}$).
- Corrección de presión para **especies gaseosas**:
  $$\mu_i^\circ(T, P) = G_i^\circ(T) + R T \ln\left(\frac{P}{\text{1 bar}}\right).$$
- Para fases condensadas y acuosas: $\mu_i^\circ(T, P) \approx G_i^\circ(T)$ (en HSC estándar el término $V^\circ(P - 1\;\text{bar})$ es despreciable salvo alta presión).
- **Conversión de entropía de mezcla acuosa** (`Mixing Entropy conversion = true`):
  Para solutos acuosos ($i \ne \text{H}_2\text{O}$), la escala de molalidad ($m_i = 55.5084 \, x_i / x_{\text{H}_2\text{O}}$) introduce el término:
  $$\Delta_{\text{mix},i} = \ln(55.508435) \quad (\text{es decir, } a_i = \gamma_i \, x_i \cdot \frac{55.508435}{x_{\text{H}_2\text{O}}} \text{ referido a } m^\circ = 1\;\text{mol/kg}).$$

## 2. Algoritmo de White, Johnson & Dantzig (1958) + Fases Puras (RAND / Active-Set)

Introduciendo multiplicadores de Lagrange $\pi \in \mathbb{R}^M$ (en unidades de $RT$), la condición KKT para una especie en fase mezcla es:
$$c_i + \ln x_i + \ln \gamma_i - \sum_{e=1}^M a_{ei} \pi_e = 0, \qquad c_i = \frac{\mu_i^\circ(T, P)}{RT}.$$

Dada una aproximación positiva $y_i > 0$ ($Y_\alpha = \sum_{i \in I_\alpha} y_i$) y un conjunto activo de fases puras $\mathcal{A} \subseteq I_{\text{pure}}$, la expansión cuadrática de $G/RT$ alrededor de $y$ conduce al sistema lineal reducido $(M + |\mathcal{P}_{\text{mix}}| + |\mathcal{A}|) \times (M + |\mathcal{P}_{\text{mix}}| + |\mathcal{A}|)$:
1. Para cada especie en mezcla $i \in I_\alpha$:
   $$n_i = y_i \left( u_\alpha + \sum_{e=1}^M a_{ei} \pi_e - \left(c_i + \ln \frac{y_i}{Y_\alpha} + \ln \gamma_i\right) \right), \qquad u_\alpha = \frac{N_\alpha}{Y_\alpha} - 1.$$
2. Sustituyendo en $\sum_i a_{ei} n_i = b_e$, $\sum_{i \in I_\alpha} n_i = N_\alpha$, y para fases puras activas $k \in \mathcal{A}$ la condición de potencial químico exacto:
   $$\sum_{e=1}^M a_{ek} \pi_e = c_k,$$
   se obtiene un sistema lineal simétrico resuelto por eliminación gaussiana con pivoteo parcial (equivalente histórico a `LGAUSSK` en `Gibbs5.dll`, implementado en Rust con `nalgebra` LU/QR).
3. **Búsqueda de paso $\lambda \in (0, 1]$:**
   $y^{\text{new}} = y + \lambda (n - y)$ asegurando $y^{\text{new}} > 0$ (regla de fracción del paso hasta frontera, típicamente $\lambda \le 0.99 \lambda_{\max}$) y descenso estricto de $G(y^{\text{new}})$.
4. **Actualización de conjunto activo $\mathcal{A}$ para fases puras:**
   - Si $n_k < 0$ para algún $k \in \mathcal{A}$, se expulsa de $\mathcal{A}$ la especie que más viola positividad.
   - Si una especie pura inactiva $k \notin \mathcal{A}$ tiene fuerza impulsora negativa $\Delta_k = c_k - \sum_e a_{ek} \pi_e < -\varepsilon_{\text{kkt}}$, se incorpora a $\mathcal{A}$.
5. **Criterio de convergencia:**
   $$\max_e \left| \sum_i a_{ei} n_i - b_e \right| < 10^{-11}, \qquad \frac{|G^{(k+1)} - G^{(k)}|}{1 + |G^{(k)}|} < 10^{-12}, \qquad \max_{i} \left|\frac{n_i - y_i}{1 + y_i}\right| < 10^{-9}.$$

## 3. Modelos de Coeficientes de Actividad ($\gamma_i$) e Iteraciones `AC Steps`

Se ejecutan hasta `AC Steps` iteraciones externas actualizando $\gamma_i = f_i(T, x)$ con amortiguamiento opcional:
- **Redlich–Kister binario (Apéndice A):**
  $$G^{Ex} = x_i x_j \sum_{n=0}^{m} k_n (x_i - x_j)^n,$$
  $$R T \ln f_i = x_j^2 \sum_{n=0}^m k_n \left[ (x_i - x_j)^n + 2 n x_i (x_i - x_j)^{n-1} \right],$$
  $$R T \ln f_j = x_i^2 \sum_{n=0}^m k_n \left[ (x_i - x_j)^n - 2 n x_j (x_i - x_j)^{n-1} \right],$$
  con $R = 8.31431\;\text{J/(mol}\cdot\text{K)}$ (y parámetros específicos para **Ga–As** en Apéndice B y **Al–Zn** líquido/FCC/HCP en Apéndice C).
- **Fórmulas en Columna C:** evaluador simbólico seguro de expresiones en $T$ (K) y $X_{\text{row}}$ (fracción molar de la fila).
- **Fase `Aqua`:** soporte de modelo Debye–Hückel / Pitzer simplificado por fuerza iónica $I = \frac{1}{2}\sum_i m_i z_i^2$ y cálculo de $\text{pH} = -\log_{10}(a_{\text{H}^+})$.

## 4. Modos Inversos y Especiales

- **Adiabático (`N,H,P`):** raíz de $F(T) = H_{\text{eq}}(T, n^*(T)) - H_{\text{target}} = 0$ en $T \in [T_{\text{low}}, 20000\;\text{K}]$ mediante acotamiento + Brent/bisección.
- **Volumen Constante (`N,P,V`):** dada $V_{\text{target}}$, resolver $P$ (o $T$) tal que $V_{\text{gas}} = \frac{n_{\text{gas}} R T}{P} + V_{\text{cond}} = V_{\text{target}}$.
- **Infinite Phase:** fijar potenciales químicos $\mu_e$ desde las especies de la fase infinita o asignar cantidades reservorio $M_{\text{inf}} = 10^6 \cdot \max(b)$ con compensación de salida.
- **Cell Mode (`N,T,P,Q` — Lampinen & Vuorisalo 1991/1992):**
  Se añade la restricción lineal de carga transferida $Q$ (`kmol e-`) entre nodos anódico y catódico y el término capacitivo $\frac{Q^2}{2 C_{\text{elec}}}$ en $G$; el voltaje de celda en circuito abierto bajo descarga $Q$ se obtiene de $\frac{\partial G_{\min}}{\partial Q} / F$.
