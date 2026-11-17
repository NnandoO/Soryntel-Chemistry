# Especificación Matemática — Módulo 06: HSC Estimate

## 1. Capacidad Calorífica $C_p(T)$ y Ajuste Kelley
El formato estándar en HSC y `soryntel-thermo` es el polinomio de Kelley:
$$C_p(T) = A + B \cdot 10^{-3} T + C \cdot 10^{5} T^{-2} + D \cdot 10^{-6} T^{2} \quad [\text{J/(mol}\cdot\text{K)}]$$

### 1.1 Regla de Neumann-Kopp (Sólidos y Líquidos)
Para un compuesto $\text{A}_a\text{B}_b\text{C}_c$ sin datos experimentales, se estima $C_p(T)$ como combinación lineal de:
- **Modo Elementos (Neumann-Kopp elemental):**
  $$C_p(T) \approx \sum_e \nu_e \, C_{p,e}(T)$$
  usando los elementos en su estado sólido de referencia (o contribuciones atómicas de Kopp para H, N, O, F, Cl en sólidos: $C_{p,\text{H}}=9.6$, $C_{p,\text{O}}=16.7$, $C_{p,\text{N}}=12.6$, $C_{p,\text{F}}=20.9$, $C_{p,\text{Cl}}=25.1\text{ J/(mol}\cdot\text{K)}$ a 298 K).
- **Modo Componentes / Óxidos (Neumann-Kopp compuesto, mayor precisión en silicatos/espinelas/sales):**
  $$C_p(T) \approx \sum_k \alpha_k \, C_{p,\text{comp}_k}(T)$$
  por ejemplo: $\text{MgAl}_2\text{O}_4 \approx \text{MgO} + \text{Al}_2\text{O}_3$.
- **Regresión Kelley:** evaluando $C_p(T_m)$ en una malla uniforme sobre $[298.15\text{ K}, T_{\max}]$ y resolviendo el sistema normal $(X^T X)\beta = X^T y$ para $\beta = [A, B, C, D]^T$.

## 2. Entropía Estándar $S^\circ_{298}$
1. **Aditividad de Componentes (Óxidos/Sales):**
   $$S^\circ_{298} \approx \sum_k \alpha_k \, S^\circ_{298,\text{comp}_k} + \Delta S_{\text{conf}}$$
   donde $\Delta S_{\text{conf}} = -R \sum x_i \ln x_i$ opcional si hay desorden catiónico.
2. **Método de Latimer (1951) para sólidos iónicos $\text{M}_a\text{X}_b$:**
   $$S^\circ_{298} \approx a \cdot s_{\text{cat}}(M, z_+) + b \cdot s_{\text{an}}(X, z_-)$$
   con contribuciones tabuladas de Latimer por catión (en función de $M$ y carga $z_+$) y anión ($\text{O}^{2-}$, $\text{OH}^-$, $\text{SO}_4^{2-}$, $\text{CO}_3^{2-}$, $\text{SiO}_3^{2-}$, $\text{Cl}^-$, etc.).

## 3. Entalpía Estándar de Formación $\Delta_f H^\circ_{298}$
1. **Aditividad de Componentes / Óxidos + Corrección Empírica (Kubaschewski / Hisham-Benson):**
   $$\Delta_f H^\circ_{298} \approx \sum_k \alpha_k \, \Delta_f H^\circ_{298,\text{comp}_k} + \Delta H_{\text{mix/estab}}$$
   *(Para óxidos mixtos y silicatos, la entalpía de formación desde óxidos suele aportar entre $-5$ y $-40\text{ kJ/mol}$ según diferencia de electronegatividad / acidez óptica).*
2. **Regla de Pauling / Electronegatividad (estimación rápida binaria):**
   $$\Delta_f H^\circ_{298}(\text{M}_a\text{X}_b) \approx -96.485 \cdot n_{\text{enlaces}} \cdot (\chi_X - \chi_M)^2 + \text{corrección de fase}$$

## 4. Métodos Orgánicos (Gases / Líquidos)
- **Contribución de Grupos de Benson (1976) / Joback-Reid (1987):**
  $$\Delta_f H^\circ_{298} = \Delta H_0 + \sum_g n_g \Delta h_g, \quad S^\circ_{298} = S_0 + \sum_g n_g \Delta s_g - R \ln \sigma$$
  $$C_p(T) = \sum_g n_g (a_g + b_g T + c_g T^2 + d_g T^3)$$

## 5. Transiciones de Fase (Trouton / Richards)
- Fusión: $\Delta S_{\text{fus}} \approx 8\text{–}10\text{ J/(mol}\cdot\text{K)}$ por átomo (metales $\approx 9.2\text{ J/(mol}\cdot\text{K)}$).
- Vaporización: $\Delta S_{\text{vap}} \approx 85\text{–}88\text{ J/(mol}\cdot\text{K)}$ (Regla de Trouton en $T_b$).
