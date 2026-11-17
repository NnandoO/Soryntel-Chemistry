# Plan de Validación — Módulo 06: HSC Estimate

## 1. Casos de Prueba Cuantitativos (vs. `Maindb6.HSC`)
1. **Óxidos Mixtos / Espinelas (`MgAl2O4` desde `MgO + Al2O3`):**
   - Verificar $C_p(298\text{–}1500\text{ K})$ por Neumann-Kopp de óxidos con error típico $< 3\%$.
   - Verificar ajuste Kelley `A, B, C, D` con $R^2 > 0.999$ respecto a la curva combinada.
2. **Silicatos (`Fe2SiO4` desde `2 FeO + SiO2`):**
   - Verificar $\Delta_f H^\circ_{298}$, $S^\circ_{298}$ y $C_p(T)$ frente al registro experimental de `Maindb6`.
3. **Neumann-Kopp Elemental (`CuFeS2`, `FeS2`):**
   - Verificar $C_p(298.15\text{ K})$ sumando contribuciones atómicas/elementales de Kopp.
4. **Método de Latimer (`NaCl`, `CaCO3`, `FeSO4`):**
   - Verificar $S^\circ_{298}$ dentro de $\pm 10\text{ J/(mol}\cdot\text{K)}$ frente a `Maindb6`.
5. **Consistencia Termodinámica:**
   - Verificar $\Delta_f G^\circ_{298} = \Delta_f H^\circ_{298} - 298.15 \cdot \Delta_f S^\circ_{298}$ y continuidad de $H(T), S(T)$ al integrar los coeficientes Kelley estimados.
