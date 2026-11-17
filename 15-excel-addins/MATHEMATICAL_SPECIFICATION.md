# Especificación Matemática — Módulo 15: Excel Add-Ins

## 1. Convención de Unidades Globales: `UNITS(T_unit, E_unit)`
El estado activo de unidades escala todas las funciones energéticas y térmicas de las hojas de cálculo:
- **Temperatura ($u_T \in \{\text{"C"}, \text{"K"}\}$):**
  $$T_{\text{K}} = \begin{cases} T_{\text{in}} + 273.15 & \text{si } u_T = \text{"C"} \\ T_{\text{in}} & \text{si } u_T = \text{"K"} \end{cases}$$
  *(En funciones de capacidad calorífica y entropía diferencial respecto a $T$, la escala por $\text{K}$ o $^\circ\text{C}$ es equivalente por unidad de incremento $\Delta T = 1\text{ K} = 1\,^\circ\text{C}$.)*
- **Energía ($u_E \in \{\text{"kJ"}, \text{"MJ"}, \text{"kcal"}, \text{"Mcal"}, \text{"kWh"}\}$):**
  Si la base interna termoquímica de `soryntel-thermo` expresa magnitudes molares estándar en $\text{kJ/mol}$ (equivalente numéricamente a $\text{MJ/kmol}$) y $C_p, S$ en $\text{J/(mol·K)}$ (equiv. $\text{kJ/(kmol·K)}$):
  Sea $f_E$ el factor de conversión desde $\text{MJ}$ hacia $u_E$:
  - $\text{"MJ"} \implies f_E = 1$ (referencia directa de `AddInSample.xls` tras `=UNITS("C";"MJ")`)
  - $\text{"kJ"} \implies f_E = 1000$
  - $\text{"Mcal"} \implies f_E = 1 / 4.184 \approx 0.239005736$
  - $\text{"kcal"} \implies f_E = 1000 / 4.184 \approx 239.005736$
  - $\text{"kWh"} \implies f_E = 1 / 3.6 \approx 0.27777778$

  Para `S` y `CP`, en HSC6 el valor expresado *"per Mmol"* con `E="MJ"` equivale exactamente a $1000\cdot \text{kJ/(kmol·K)}$ si se mide en $\text{MJ/(Mmol·K)}$? En unidades reales:
  $$1\ \frac{\text{J}}{\text{mol}\cdot\text{K}} = 1\ \frac{\text{kJ}}{\text{kmol}\cdot\text{K}} = 1\ \frac{\text{MJ}}{\text{Mmol}\cdot\text{K}}$$
  Por tanto, al cambiar $u_E$, `H, G` escalan por $f_E$ (en `u_E/kmol`) y `S, CP` escalan por $f_E$ manteniendo exactamente el mismo factor respecto a `MJ/Mmol·K` (ej. `CH4(a)` a $100\,^\circ\text{C}$ con `E="MJ"` da $C_p = 205.166\ \text{J/(mol·K)} = 205.166\ \text{MJ/(Mmol·K)}$).

---

## 2. Grupo General (`BAL`, `SPECIES`, `UNITS`)
- **`BAL(Equation)`:** Resuelve los coeficientes estequiométricos enteros mínimos $\nu_i$ de la ecuación $A + B = C + D$ imponiendo el balance de cada elemento y carga ($N_{\text{elem}} \times N_{\text{esp}}$ mediante el solver `soryntel-solver` existente).
- **`SPECIES(DBNo, Position)`:** Retorna la fórmula de la especie en la base de datos `DBNo` (`1` = OwnDB / usuario, `2` = MainDB) e índice `Position` ($1$-indexado). En `AddInSample.xls`: `SPECIES(2; 200) -> "Ag2WO4"`.

---

## 3. Propiedades de Especie y de Reacción Química

### 3.1 Para una Especie $i$ a temperatura $T_{\text{K}}$
Usando los intervalos de Kelley $(T_{\min,k}, T_{\max,k})$, entalpías de transición $\Delta H_{\text{tr},k}$ y parámetros $(H_{298}^\circ, S_{298}^\circ, A_k, B_k, C_k, D_k)$:
$$C_p^\circ(T) = A_k + B_k\cdot 10^{-3} T + C_k\cdot 10^5 T^{-2} + D_k\cdot 10^{-6} T^2 \quad [\text{J/(mol·K)}]$$
$$H^\circ(T) = H_{298}^\circ + \int_{298.15}^{T} C_p^\circ(\tau)\,d\tau + \sum_{\text{tr}\le T} \Delta H_{\text{tr}} \quad [\text{kJ/mol} \equiv \text{MJ/kmol}]$$
$$S^\circ(T) = S_{298}^\circ + \int_{298.15}^{T} \frac{C_p^\circ(\tau)}{\tau}\,d\tau + \sum_{\text{tr}\le T} \frac{\Delta H_{\text{tr}}}{T_{\text{tr}}} \quad [\text{J/(mol·K)} \equiv \text{MJ/(Mmol·K)}]$$
$$G^\circ(T) = H^\circ(T) - T_{\text{K}} \cdot \frac{S^\circ(T)}{1000}$$

Derivadas directas de unidad:
- **`H(Species, T)`**: $H^\circ(T)\cdot f_E$ $[u_E/\text{kmol}]$.
- **`HKG(Species, T)`**: $\dfrac{H^\circ(T)\cdot f_E}{\text{MW}_i}$ $[u_E/\text{kg}]$ donde $\text{MW}_i$ está en $\text{kg/kmol}$.
- **`HNM3(Species, T)` / `HCM(Species, T)`**: $\dfrac{H^\circ(T)\cdot f_E}{V_m^\circ}$, con volumen molar normal ideal $V_m^\circ = 22.413996\ \text{Nm}^3/\text{kmol}$ (`101.325 kPa`, `273.15 K`).
- **`HLAT(Species, T)`**: $H_{\text{lat}}(T) = H^\circ(T) - H^\circ(298.15\text{ K}) - \sum_{\text{tr}\le T}\Delta H_{\text{tr}}$ cuando la descripción indica entalpía sensible excluyendo transiciones de fase (o $H(T)-H(298.15)$ sin saltos de fase, verificable en `AddInSample.xls` fila 14: `HLAT("H2O(l)", 25) = 0.0`, y fila 43: `STREAMHLAT("Cu2S",1,100) = 14.060` vs `STREAMH` sensible puro sin $\Delta H_{298}^\circ$ ni transformaciones!).

### 3.2 Polimorfismo sobre Ecuación Química $\sum_j \nu_j X_j = 0$
Cuando el primer argumento de `H`, `HKG`, `S`, `CP`, `G` o `K` contiene `'='` (o `'->'`), la ecuación se balancea y se evalúa $\Delta \Phi(T) = \sum_j \nu_j \Phi_j(T)$:
- $\Delta H(T) = \sum \nu_j H_j(T)$ (en `AddInSample.xls` fila 19: `2H2(g)+O2(g)=2H2O(g)` a `100 °C` da `-485.1318 MJ/kmol`).
- $\Delta H_{\text{kg}}(T) = \dfrac{\Delta H(T)}{M_{\text{reactivos}}}$ donde $M_{\text{reactivos}} = \sum_{\nu_j < 0} |\nu_j|\,\text{MW}_j$ (`2*2.016 + 32.00 = 36.032 kg` $\implies -485.1318 / 36.0315 = -13.4645\text{ MJ/kg}$, coincide exacto con fila 20 de `AddInSample.xls`!).
- $\ln K_{\text{eq}}(T) = -\dfrac{\Delta G^\circ(T)}{R\,T_{\text{K}}}$ (con $R = 8.31451\text{ J/(mol·K)}$), $K = \exp(\ln K_{\text{eq}})$.

---

## 4. Funciones Inversas (`Tat*`, `StreamT*`, `FlowT*`)
Para una propiedad objetivo $Y^* \in \{H, H_{\text{kg}}, H_{\text{Nm}^3}, H_{\text{lat}}, S, C_p, G\}$:
- **`TatY(Species, Y_target)`:** Busca $T$ tal que $Y(\text{Species}, T) = Y^*$ mediante barrido por intervalos de temperatura de la base + bisección/Brent híbrida hasta $|Y(T) - Y^*| < 10^{-9}$.
- **`StreamTY(SpeciesRange, AmountRange, Y_target, Tmin, Tmax)`** y **`FlowTY(...)`:** Evalúan la mezcla $\Phi_{\text{mix}}(T) = \sum_{k=1}^{m} n_k \phi_k(T)$ dentro del intervalo $[T_{\min}, T_{\max}]$.
  - Si $Y^*$ no queda acotado en $[T_{\min}, T_{\max}]$ (o la función no alcanza $Y^*$ en el rango), devuelve error de hoja **`#VALUE!`** exactamente como documenta `AddInSample.xls` filas 60–78 (`"Outside range returns #VALUE!"`).

---

## 5. Corrientes, Flujos y Densidad Acuosa (`Stream*`, `Flow*`, `DensityA`)
Dada una lista de $m$ especies $\{s_k\}$ y cantidades $\{n_k\}$ a temperatura $T$:
$$\Phi_{\text{stream}} = \sum_{k=1}^{m} n_k \cdot \phi(s_k, T)$$
- Tanto `Flow*` como `Stream*` suman sobre el rango de filas paralelas `SpeciesRange` y `AmountRange` (`H` en `kmol`, `HKG` en `kg`, `HNM3` en `Nm³`, `HLAT` en `kmol`, `S` y `CP` en `Mmol` equivalentes, `G` en `kmol`).
- **`DensityA(SpeciesRange, WeightFracRange, T_C)`:** Densidad ($\text{kg/m}^3$) de una disolución acuosa multicomponente definida por fracciones másicas $w_k$ de solutos ($\sum w_k \le 1$, fracción de agua $w_{\text{H}_2\text{O}} = 1 - \sum_k w_k$) a temperatura $T\ [^\circ\text{C}]$:
  $$\frac{1}{\rho_{\text{sol}}(T)} = \frac{w_{\text{H}_2\text{O}}}{\rho_{\text{H}_2\text{O}}(T)} + \sum_k w_k\,\bar{v}_k(T, w_k)$$
  donde las densidades puras de agua provienen de `water::water_density` y las contribuciones de electrolitos acuosos incorporan las correlaciones de HSC `Aqueous Solution Density.xls` / Pitzer-Millero calibradas para sales comunes (`NaCl`, `FeSO4`, `CuSO4`, `H2SO4`, etc.).

---

## 6. Propiedades Independientes de $T$, Porcentajes y Polinomio $C_p$
- **`MolP(S1, S2)`** y **`WTP(S1, S2)`:** Estequiometría de especies compuestas o minerales (p. ej. `ZnO` en `ZnO*Fe2O3`): descomponiendo las fórmulas elementales $\mathbf{a}_1, \mathbf{a}_2$ y hallando el máximo coeficiente estequiométrico compatible o razón molar de componentes óxido/especie, $\text{mol-}\% = 100\cdot n_1 / n_{\text{total}}$ y $\text{wt-}\% = 100\cdot (n_1 \text{MW}_1) / \text{MW}_2$. En `AddInSample.xls`: `ZnO` frente a `ZnO*Fe2O3` ($\equiv \text{ZnO} + \text{Fe}_2\text{O}_3$ o relación de átomos $\text{Zn}:\text{Fe}:\text{O}$? Verificando fila 94: `28.57142857% = 2/7 = 100 * (2 átomos en ZnO? o moles en especies?)`: `ZnO` tiene 2 átomos, `ZnFe2O4` tiene $1+2+4=7$ átomos → $2/7 = 28.57142857\%$, mientras `WTP = 100 * 81.38 / 241.07 = 33.75736\%`!). ¡Excelente verificación matemática directa de HSC6!
- **`CPA..CPD`, `TMin`, `TMax`, `CPFUNCTION`:** Leen el primer (o activo) rango de Kelley del registro en `SpeciesDb` y formatean `A + B*10^(-3)*T + C*10^5*T^(-2) + D*10^(-6)*T^2`.

---

## 7. Función Matricial `StreamEQ(SpeciesRange, AmountRange, T, P)`
Dado un rango vertical de $N$ filas dividido en fases encabezadas por filas con prefijo **`§`** (por ejemplo `§ PHASE 1:`, `§ PHASE 2:`):
1. Las filas con prefijo `§` definen el **nombre de fase**; en la celda de salida correspondiente a la fila `§`, `StreamEQ` escribe la temperatura de esa fase (`D111 = 1500.0`).
2. Las filas de especies bajo cada `§ PHASE` aportan los moles de entrada $b_i^{\text{in}}$ para construir el vector de balance elemental $\mathbf{b} = \mathbf{A}\mathbf{n}^{\text{in}}$.
3. El solver multifase Gibbs (`soryntel_thermo::equilibrium::solve_equilibrium`) minimiza:
   $$\min_{\mathbf{n} \ge 0} \sum_{p=1}^{P}\sum_{i \in p} n_i \left(\frac{\mu_i^\circ(T_p)}{RT_p} + \ln a_i(n_i, P)\right) \quad \text{s.a. } \mathbf{A}\mathbf{n} = \mathbf{b}$$
4. Devuelve un arreglo columna de longitud exactamente $N$ alineado fila a fila con `SpeciesRange`.
