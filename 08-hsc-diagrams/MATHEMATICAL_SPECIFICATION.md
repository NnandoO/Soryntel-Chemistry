# Mathematical Specification — Módulo 08: H, S, C and G Diagrams

## 1. Propiedades termodinámicas por especie a temperatura $T$
Sea una especie $i$ con coeficiente multiplicador $\alpha_i > 0$ (por defecto $\alpha_i = 1$). A partir de `evaluate_species_si(sp_i, T, false, false)` se obtienen en SI:
- $C_{p,i}(T)$ [J/(mol·K)]
- $H_i(T)$ [J/mol]
- $S_i(T)$ [J/(mol·K)]
- $G_i(T) = H_i(T) - T\,S_i(T)$ [J/mol]

Y mediante `sum_element_references(term_i, T, false, repo)` se obtienen los totales de referencia elemental $(H_{\text{ref},i}(T), S_{\text{ref},i}(T), G_{\text{ref},i}(T))$.

## 2. Los 8 tipos de diagrama
1. **H Enthalpy (total)**:
   $$Y_i(T) = \alpha_i \, H_i(T)$$
2. **H Enthalpy (latent)**:
   $$Y_i(T) = \alpha_i \, \bigl(H_i(T) - H_i(T_{\text{ref}})\bigr), \quad T_{\text{ref}} = 298.15\text{ K}$$
3. **S Entropy**:
   $$Y_i(T) = \alpha_i \, S_i(T)$$
4. **Cp Heat Capacity**:
   $$Y_i(T) = \alpha_i \, C_{p,i}(T)$$
5. **G Gibbs Energy**:
   $$Y_i(T) = \alpha_i \, G_i(T)$$
6. **DH ($\Delta_f H$)**:
   $$Y_i(T) = \alpha_i \, \bigl(H_i(T) - H_{\text{ref},i}(T)\bigr)$$
7. **DS ($\Delta_f S$)**:
   $$Y_i(T) = \alpha_i \, \bigl(S_i(T) - S_{\text{ref},i}(T)\bigr)$$
8. **DG ($\Delta_f G$, Diagramas de Ellingham)**:
   $$Y_i(T) = \alpha_i \, \bigl(G_i(T) - G_{\text{ref},i}(T)\bigr)$$

## 3. Balanceo de elemento para diagramas de Ellingham (`DG`)
Dado un elemento objetivo $E$ (ej. `O` para óxidos, `S` para sulfuros, `Cl` para cloruros) y una base de átomos $N_{\text{base}}$ (por defecto $N_{\text{base}} = 2$ para $O_2, S_2, Cl_2$, o $N_{\text{base}} = 1$ por átomo):
Si la fórmula $i$ contiene $\nu_{i,E} > 0$ átomos del elemento $E$, su coeficiente balanceado es:
$$\alpha_i = \frac{N_{\text{base}}}{\nu_{i,E}}$$
Ejemplos para $E = \text{O}$, $N_{\text{base}} = 2$ (base $1\text{ mol O}_2$):
- `Fe2O3` ($\nu_{\text{O}} = 3$) $\Rightarrow \alpha = 2/3 \approx 0.6667$
- `FeO` ($\nu_{\text{O}} = 1$) $\Rightarrow \alpha = 2$
- `CO2(g)` ($\nu_{\text{O}} = 2$) $\Rightarrow \alpha = 1$
- `MgO` ($\nu_{\text{O}} = 1$) $\Rightarrow \alpha = 2$

## 4. Segmentación sólido vs. punteado (extrapolación)
En cada punto $T_k$ de la grilla $[T_{\min}, T_{\max}]$:
- Si `extrapolated_from_k.is_none()`, el punto pertenece a un tramo **sólido** (dentro del rango tabulado de la base de datos).
- Si `extrapolated_from_k.is_some()`, el punto pertenece a un tramo **punteado** (extrapolación de la función $C_p(T)$ fuera del rango de la base de datos).
