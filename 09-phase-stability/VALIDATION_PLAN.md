# Validation Plan — Módulo 09: Phase Stability Diagrams

## Casos de prueba unitarios (`soryntel-thermo/src/phase_stability.rs`)

1. **Sistema `Cu-S-O` isotérmico (`p-p`, `700 °C`, `log P(SO2)` vs `log P(O2)`)**:
   - Verificar que a muy baja $P_{\text{O}_2}$ y baja $P_{\text{SO}_2}$ predomina **`Cu`**.
   - Verificar que a alta $P_{\text{O}_2}$ y baja $P_{\text{SO}_2}$ predominan **`Cu2O`** y **`CuO`**.
   - Verificar que a alta $P_{\text{O}_2}$ y alta $P_{\text{SO}_2}$ predomina **`CuSO4`**.
   - Verificar que a baja $P_{\text{O}_2}$ y alta $P_{\text{SO}_2}$ predomina **`Cu2S`**.
2. **Sistema `Ti-Cl-O` isotérmico (`p-p`, `300 °C`, `log P(Cl2)` vs `log P(O2)`)**:
   - Verificar predominancia de **`Ti`**, **`TiO2`** (alta $P_{\text{O}_2}$, baja $P_{\text{Cl}_2}$) y **`TiCl4`** / **`TiCl3`** (alta $P_{\text{Cl}_2}$, baja $P_{\text{O}_2}$).
3. **Modo `T-p` (`Cu-S-O`, `T = 25..1000 °C`, `log P(O2) = -20..0`, `P(SO2) = 0.1 bar`)**:
   - Verificar transición térmica de `CuSO4` $\to$ `CuO*CuSO4` $\to$ `CuO` al aumentar la temperatura.
4. **Validación de gases colineales ($\det C = 0$)**:
   - Si el usuario selecciona `SO2(g)` y `SO3(g)`? No, esos son independientes; si selecciona `CO(g)` y `CO2(g)` en un sistema `Fe-C-O` está bien, pero si selecciona `O2(g)` y `O3(g)` ($\det C = 0$), debe devolver un error claro indicando que los gases de eje no son linealmente independientes.
