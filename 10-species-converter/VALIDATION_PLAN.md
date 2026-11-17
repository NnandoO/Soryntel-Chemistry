# Plan de Validación — Species Converter

| Caso | Descripción | Criterio de Éxito |
|---|---|---|
| V1 | `Initialize` con mezcla (`Cu 25, CuO 25, Cu2S 25, CuSO4 25`, Fig. 9) | `Cu ≈ 74.889%`, `O ≈ 15.053%`, `S ≈ 10.058%`, Total `100.000%` |
| V2 | `Cu-S-Fe-Ni` (`25, 40, 10, 25`) con `Method 1` (Fig. 3) | `Total Output ≈ 100%`, `FeS2 ≈ 21.48%`, `NiS` y `CuS` presentes, `Residual Error` finito |
| V3 | `Fix Element Balance` sobre `Cu-S-Fe-Ni` (Fig. 5) | Agrega `Cu`, `S`, `Fe`, `Ni` con `Weight Coeff = 0.100` y reduce `Residual Error` |
| V4 | `Element Balance` (Fig. 4) | Verifica `Input kg`, `Output kg`, `Balance kg`, `Error %` y `Limit = 100 - Error %` |
| V5 | `Method 3` validación subdeterminada Ec. (3) | Rechaza cuando $N_{\text{Output}} \le N_{\text{Targets}} + N_{\text{Input}}$ |
| V6 | `Stability Weighting` (`Sulfides`, `Weighting coef. = 80`, Fig. 11) | Clasifica `CuS`, `CuFeS2`, `NiS` como `sulfide` y favorece fases más estables |
