# Especificación Matemática — Módulo 05: Exergy Balance

Verificada numéricamente contra `Fig. 1` y `Fig. 2` de **HSC Exergy Balance**.

## 1. Estado Muerto de Referencia
Como indica la nota oficial del módulo:
- $T_0 = 298.15\text{ K}$ ($25^\circ\text{C}$)
- $P_0 = 1.0\text{ bar}$
- $R = 8.314462618\text{ J/(mol}\cdot\text{K)}$

## 2. Propiedades por Especie (Columnas `B`–`K`)
Para cada especie $i$ con $n_i$ moles (`Amount mol`, Col `C`) a temperatura $T$ (`T (K)`, Col `E`) y presión $P$ (`P (bar)`, Col `F`):

1. **Masa (`B`):** $m_i\text{ (kg)} = n_i \cdot \frac{M_i\text{ (g/mol)}}{1000}$
2. **Fracción molar (`D`):** $x_i = \frac{n_i}{\sum_j n_j}$
3. **Entalpía total (`G`, `H Total` en kJ):**
   $$H_i(T) = n_i \cdot H_i^\circ(T)\text{ (kJ/mol)}$$
4. **Exergía química estándar molar (`H`, `Ex_elemental` en kJ/mol):**
   $$\text{ex}_{\text{chem},i}^\circ = \Delta_f G_i^\circ(T_0) + \sum_e \nu_e \, b_{\text{ch},e}^\circ$$
   basada en Szargut, Morris & Steward (1988).
5. **Exergía química (`I`, `Ex_che` en kJ):**
   $$\text{Ex}_{\text{che},i} = n_i \cdot \text{ex}_{\text{chem},i}^\circ$$
6. **Exergía física (`J`, `Ex_phy` en kJ):**
   $$\text{Ex}_{\text{phy},i} = n_i \left[ \big(H_i(T) - H_i(T_0)\big) - T_0 \big(S_i(T, P) - S_i(T_0, P_0)\big) \right]$$
   - En **Gas stream**, la variación de entropía incluye la contribución de presión:
     $$S_i(T, P) - S_i(T_0, P_0) = S_i^\circ(T) - S_i^\circ(T_0) - R \ln\left(\frac{P}{P_0}\right)$$
   - En **Normal stream** para fases condensadas, se toma $S_i^\circ(T) - S_i^\circ(T_0)$.
7. **Exergía total de especie (`K`, `Ex_tot` en kJ):**
   $$\text{Ex}_{\text{tot},i} = \text{Ex}_{\text{che},i} + \text{Ex}_{\text{phy},i}$$

## 3. Exergía Total de Corriente (`L`, `Ex_tot_stream` en kJ)

### 3.1 Corrientes Materiales (`Normal stream` y `Gas stream`)
A la suma de `Ex_tot` de las especies de la corriente se suma la exergía de mezcla ideal:
$$\text{Ex}_{\text{tot\_stream}} = \sum_i \text{Ex}_{\text{tot},i} + R T_0 \sum_{i:\, n_i > 0} n_i \ln x_i$$
*(Verificado contra Fig. 2: en `Output Stream 1`, $96153.69 + 800 \cdot 8.31446 \cdot 298.15 \cdot (0.75\ln 0.75 + 0.25\ln 0.25)/1000 = 96153.69 - 1.11 = 96152.58\text{ kJ}$; en `Input Gas Stream 1`, $147471.61 - 12.40 = 147459.21\text{ kJ}$).*

### 3.2 Corrientes de Energía (`Energy stream`)
Convierte el flujo de calor $Q$ a temperatura $T_b$ (en Kelvin) mediante eficiencia de Carnot y suma el trabajo eléctrico $W$:
$$\text{Ex}_{\text{tot\_stream}} = Q \left(1 - \frac{T_0}{T_b}\right) + W$$
*(Verificado contra Fig. 2: en `Output Energy Stream 1`, $Q = 20000\text{ kJ}$ a $T_b = 1100.00\text{ K}$: $20000 \cdot (1 - 298.15/1100.00) = 14579.09\text{ kJ}$ exactos).*

## 4. Balances Globales (Cabecera Superior)
Convención de signo `OUT - IN` (como en Fig. 2):
- **`Mass Balance (kg)`:** $\sum_{\text{OUT}} m - \sum_{\text{IN}} m$
- **`Heat Balance (kJ)`:** $\sum_{\text{OUT}} H_{\text{Total}} - \sum_{\text{IN}} H_{\text{Total}}$
- **`Exergy Balance (kJ)`:** $\sum_{\text{OUT}} \text{Ex}_{\text{tot\_stream}} - \sum_{\text{IN}} \text{Ex}_{\text{tot\_stream}}$
  *(Un valor negativo indica exergía destruida/perdida; la irreversibilidad es $I = -\text{Exergy Balance}$).*
