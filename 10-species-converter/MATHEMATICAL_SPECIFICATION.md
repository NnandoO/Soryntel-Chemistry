# Especificación Matemática — Species Converter

## 1. Fracciones Másicas Elemento–Especie
Para cada especie $s$ con fórmula química válida, sea $n_{e,s}$ el número de átomos del elemento $e$ y $M_e$ su masa atómica ($\text{g/mol}$). La masa molar es:
$$M_s = \sum_e n_{e,s} M_e, \qquad A_{e,s} = \frac{n_{e,s} M_e}{M_s}$$
de modo que $\sum_e A_{e,s} = 1$.

## 2. Inicialización (`Initialize`)
Dada una lista de entradas `INPUT` $(f_k, w_k^{\text{in}})$ (que pueden ser elementos puros o especies), la composición elemental calculada es:
$$b_e = \sum_k w_k^{\text{in}} A_{e, f_k}$$

## 3. Problema de Optimización No Negativa (`Solve`, $x \ge 0$)
Se busca el vector $x \in \mathbb{R}_{\ge 0}^S$ (en `wt-%` de cada especie de `OUTPUT`) que minimiza:
$$\min_{x \ge 0} \sum_{e \in \mathcal{E}_{\text{act}}} w_e^{\text{elem}} \left(\sum_s A_{e,s} x_s - b_e\right)^2 + \sum_{s \in \mathcal{T}} \left(w_s^{\text{target}}\right)^2 (x_s - t_s)^2 + \lambda_{\text{reg}} \sum_s r_s x_s^2$$
- **Exact O / H measurement:** `O` y `H` solo se incluyen como restricciones activas en $\mathcal{E}_{\text{act}}$ si están presentes en el `INPUT` o si `Exact O measurement` / `Exact H measurement` está activado.
- **Method 1:** Aplica `Weight Coeff` tanto a especies con `Target wt-%` como a especies sin `Target` (regularización/prioridad), y habilita `Norm limit` y `Exact O/H`.
- **Method 2:** Aplica `Weight Coeff` únicamente a especies que tienen `Target wt-%`; ignora los demás pesos (`Weight Coeff` sin `Target` no penaliza).
- **Method 3 (*Column scaling*):** Requiere la condición subdeterminada:
  $$N_{\text{Output species}} > N_{\text{Targets}} + N_{\text{Input species}}$$
  (excluyendo `[Others]`).

## 4. Error Residual y Balance Elemental
$$\text{ResidualError} = \sum_{e \in \mathcal{E}_{\text{in}}} \left| N_e - N_{e,\text{measured}} \right|$$
Para cada elemento $e$ sobre una base de $1000\text{ kg}$ (o $10 \times \text{wt-\%}$):
- $\text{Input kg}_e = 10 \cdot b_e$
- $\text{Output kg}_e = 10 \cdot \sum_s A_{e,s} x_s$
- $\text{Balance kg}_e = \text{Output kg}_e - \text{Input kg}_e$
- $\text{Error \%}_e = \frac{|\text{Balance kg}_e|}{\max(\text{Input kg}_e, 10^{-12})} \cdot 100$
- $\text{Limit}_e = 100 - \text{Error \%}_e$ (en rojo si $\text{Limit}_e < \text{umbral}$).

## 5. Stability Weighting
Al seleccionar un `Species type` (`Sulfides`, `Oxides`, `Sulfates`, `Carbonates`, `Chlorides`, `Silicates`, `Hydroxides`) con `Weighting coef.` $k_w$, las especies de `OUTPUT` clasificadas en ese tipo obtienen $\Delta_f G^\circ(298.15\text{ K})$ desde la base de datos y reciben prioridad proporcional a $-\Delta_f G^\circ / k_w$ en la función objetivo.
