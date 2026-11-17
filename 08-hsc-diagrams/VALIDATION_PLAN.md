# Validation Plan — Módulo 08: H, S, C and G Diagrams

## Casos de prueba unitarios y de integración (`soryntel-thermo/src/diagrams.rs`)

1. **`H Enthalpy (latent)` a $T_{\text{ref}} = 298.15\text{ K}$**:
   - Para cualquier especie (ej. `Fe`, `Al2O3`, `H2O(g)`), $H_{\text{latent}}(298.15\text{ K}) = 0.0\text{ kJ/mol}$ y crece monótonamente con $T$.
2. **`Balance Element Amount` para Ellingham (`O2` = 2 átomos de `O`)**:
   - `Fe2O3` $\to \alpha = 2/3$, `FeO` $\to \alpha = 2$, `CO(g)` $\to \alpha = 2$, `CO2(g)` $\to \alpha = 1$, `MgO` $\to \alpha = 2$.
3. **Diagrama de Ellingham (`DG`) de óxidos ($25\text{ °C} \dots 1500\text{ °C}$)**:
   - Verificar que `MgO` y `Cr2O3` quedan por debajo de `FeO` ($\Delta G$ más negativo $\Rightarrow$ óxidos más estables).
   - Verificar que la pendiente de `2 C + O2 -> 2 CO(g)` es negativa ($\Delta S > 0$) y cruza la curva de `2 Fe + O2 -> 2 FeO` alrededor de $\sim 700\text{ °C}$ (reducción carbotérmica citada en `19 H, S, C and G Diagrams.txt`).
4. **Detección de extrapolación (sólido vs. punteado)**:
   - Evaluar una especie con $T_{\max} = 1000\text{ K}$ hasta $1500\text{ K}$ y verificar que los puntos $T \le 1000\text{ K}$ tienen `extrapolated = false` y $T > 1000\text{ K}$ tienen `extrapolated = true`.
