# Especificación Matemática — Módulo 11: Benson Estimation

## 1. Modelo de Aditividad de Segundo Orden (Grupos de Benson)
Un grupo de Benson se define como $A-(B_1)(B_2)\dots(B_k)$, donde $A$ es un átomo central polivalente (coordinación $\ge 2$) y $B_i$ son los átomos o pseudoatomos unidos a $A$ (`C`, `Cd` doble enlace, `Ct` triple enlace, `Cb` aromático, `CO` carbonilo, `O`, `N`, `S`, `H`, `F`, `Cl`, `Br`, `I`).

Para un compuesto orgánico en fase gaseosa ideal ($P = 1\text{ bar}$, $T_{\text{ref}} = 298.15\text{ K}$) con multiplicidades $n_g \in \mathbb{Z}_{\ge 0}$ y correcciones estructurales $m_c$:

### 1.1 Entalpía Estándar de Formación $\Delta_f H^\circ_{298.15}$
$$\Delta_f H^\circ_{298.15}(g) = \sum_{g} n_g \, \Delta_f h^\circ_g + \sum_{c} m_c \, \Delta h^{\text{corr}}_c \quad [\text{kcal/mol}]$$
donde $\Delta h^{\text{corr}}_c$ incluye:
- Interacciones *gauche* ($\approx +0.80\text{ kcal/mol}$ por par alcano-alcano en configuración *gauche*),
- Isomería *cis* en alquenos ($\approx +1.00\text{ kcal/mol}$),
- Tensión de anillo (*ring strain*): ciclopropano ($+27.6$), ciclobutano ($+26.2$), ciclopentano ($+6.3$), ciclohexano ($0.0$), epóxido ($+27.2\text{ kcal/mol}$), etc.

### 1.2 Entropía Molar Estándar $S^\circ_{298.15}$
El método de Benson exige corregir la entropía intrínseca por **simetría externa/interna** ($\sigma_{\text{tot}}$) y **mezcla de enantiómeros** ($n_{\text{opt}}$, átomos quirales o centros ópticos):
$$S^\circ_{\text{int}}(298.15) = \sum_{g} n_g \, s^\circ_g + \sum_{c} m_c \, s^{\text{corr}}_c \quad [\text{cal/(mol}\cdot\text{K)}]$$
$$S^\circ_{298.15}(g) = S^\circ_{\text{int}}(298.15) - R \ln(\sigma_{\text{tot}}) + R \ln(n_{\text{opt}})$$
con $R = 1.987204\text{ cal/(mol}\cdot\text{K)}$ ($\text{o } 8.314462\text{ J/(mol}\cdot\text{K)}$).

### 1.3 Energía Libre de Gibbs de Formación $\Delta_f G^\circ_{298.15}$
$$\Delta_f S^\circ_{298.15} = S^\circ_{298.15}(\text{compuesto}) - \sum_{e} \nu_e \, S^\circ_{298.15,\text{ref}}(e)$$
$$\Delta_f G^\circ_{298.15} = \Delta_f H^\circ_{298.15} - 298.15 \, \Delta_f S^\circ_{298.15}$$
usando las entropías estándar elementales de referencia ($C(\text{grafito})=5.74$, $H_2(g)=130.68$, $O_2(g)=205.15$, $N_2(g)=191.61$, $S(\text{rómbico})=32.05$, $Cl_2(g)=223.08$, $Br_2(l)=152.21$, $F_2(g)=202.79\text{ J/(mol}\cdot\text{K)}$).

### 1.4 Capacidad Calorífica $C_p^\circ(T)$ y Ajuste Kelley
Las contribuciones tabuladas de Benson proveen $c_{p,g}(T_k)$ en la malla estándar:
$$T_k \in \{300,\; 400,\; 500,\; 600,\; 800,\; 1000,\; 1500\}\text{ K}$$
$$C_p^\circ(T_k) = \sum_{g} n_g \, c_{p,g}(T_k) + \sum_{c} m_c \, c_{p,c}^{\text{corr}}(T_k)$$
Sobre los pares $\{(T_k, C_p^\circ(T_k))\}$ (más interpolación suave a $298.15\text{ K}$) se resuelve la regresión lineal ordinaria para el polinomio de Kelley:
$$C_p(T) = A + B \cdot 10^{-3} T + C \cdot 10^{5} T^{-2} + D \cdot 10^{-6} T^{2} \quad [\text{J/(mol}\cdot\text{K)}]$$

## 2. Estimación Opcional de Fase Condensada (Líquido / Sólido)
Cuando el usuario solicita fase líquida (`Phase::Liquid`) mediante estimación complementaria de Joback–Reid / Trouton:
- Punto de ebullición normal: $T_b \approx 198.2 + \sum_g n_g \Delta T_{b,g}\text{ [K]}$
- Entalpía de vaporización en $T_b$: $\Delta H_{\text{vap}} \approx T_b \cdot 88\text{ J/(mol}\cdot\text{K)}$ (Regla de Trouton, con incremento por puentes de hidrógeno OH/COOH)
- $\Delta_f H^\circ_{298}(l) \approx \Delta_f H^\circ_{298}(g) - \Delta H_{\text{vap}}(298\text{ K})$
- $S^\circ_{298}(l) \approx S^\circ_{298}(g) - \Delta H_{\text{vap}}/T_b$.
