# Plan de Validación — Módulo 05: Exergy Balance

1. **Caso de Referencia Principal (`Fig. 2` — Tostación/Conversión de Sulfuros de Cobre):**
   - **Input:**
     - `Input Stream 1` ($1100\text{ K}, 1\text{ bar}$): `CuS` $800\text{ mol}$, `FeS` $100\text{ mol}$ $\rightarrow$ `Ex_tot` $\approx 660846.53\text{ kJ}$, `Ex_tot_stream` $\approx 660845.75\text{ kJ}$.
     - `Input Stream 2` ($200\text{ K}, 1\text{ bar}$): `SiO2` $50\text{ mol}$ $\rightarrow$ `Ex_tot_stream` $\approx 157.32\text{ kJ}$.
     - `Input Gas Stream 1` ($25\text{ K}$ / $298.15\text{ K}$ según unidad, $1\text{ bar}$): `O2(g)` $2000\text{ mol}$, `N2(g)` $8000\text{ mol}$ $\rightarrow$ `Ex_tot` $\approx 147471.61\text{ kJ}$, `Ex_tot_stream` $\approx 147459.21\text{ kJ}$.
     - `Input Energy Stream 1`: `Heat feed` $0\text{ kJ}$ ($25\text{ K}$), `Electricity feed` $10000\text{ kJ}$ $\rightarrow$ `Ex_tot_stream = 10000.00 kJ`.
   - **Output:**
     - `Output Stream 1` ($1300\text{ K}, 1\text{ bar}$): `Cu` $600\text{ mol}$, `CuO` $200\text{ mol}$ $\rightarrow$ `Ex_tot` $\approx 96153.69\text{ kJ}$, `Ex_tot_stream` $\approx 96152.58\text{ kJ}$.
     - `Output Stream 2` ($1300\text{ K}, 1\text{ bar}$): `*2FeO*SiO2` $50\text{ mol}$ $\rightarrow$ `Ex_tot_stream` $\approx 16777.88\text{ kJ}$.
     - `Output Gas Stream 1` ($1100\text{ K}, 1\text{ bar}$): `SO2(g)` $900\text{ mol}$, `SO3(g)` $0\text{ mol}$, `O2(g)` $550\text{ mol}$, `N2(g)` $8000\text{ mol}$ $\rightarrow$ `Ex_tot` $\approx 419651.34\text{ kJ}$, `Ex_tot_stream` $\approx 419638.91\text{ kJ}$.
     - `Output Energy Stream 1`: `Heat loss` $20000\text{ kJ}$ a $1100\text{ K}$, `Electricity out` $0\text{ kJ}$ $\rightarrow$ `Ex_tot_stream = 14579.09 kJ`.
   - **Balances globales:** `Mass Balance = -12.80 kg`, `Heat Balance = 57851.57 kJ`, `Exergy Balance = -271313.81 kJ`.
2. **Paridad de Carnot y Mezcla Ideal:**
   - Verificar que `Ex_tot_stream - Ex_tot` coincide con $R T_0 \sum n_i \ln x_i$ hasta $0.01\text{ kJ}$.
   - Verificar que `Energy stream` con `Heat loss = 20000 kJ` a `T = 1100 K` da `14579.09 kJ` con $T_0 = 298.15\text{ K}$.
3. **Estado Muerto ($T_0 = 298.15\text{ K}, P_0 = 1\text{ bar}$):**
   - Verificar `Ex_phy = 0.00 kJ` cuando $T = 298.15\text{ K}$ y $P = 1\text{ bar}$.
