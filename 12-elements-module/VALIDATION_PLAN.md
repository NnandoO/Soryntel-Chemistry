# Plan de Validación — Módulo 12: Elements Module

| Caso | Prueba | Referencia Esperada | Criterio de Aceptación |
|---|---|---|---|
| E1 | Pesos atómicos IUPAC (`Z = 1, 6, 8, 26, 29, 79, 92`) | `H=1.00794`, `C=12.011`, `O=15.9994`, `Fe=55.845`, `Cu=63.546`, `Au=196.967`, `U=238.029` | Error $< 0.01\text{ g/mol}$ |
| E2 | Densidad en tabla periódica | Máximo `Os ≈ 22.59 g/cm³` (`Ir ≈ 22.56`), `Au=19.32`, `Al=2.70`, `H2O-ref/Li=0.534` | `d(Os) == d_box`, `d(Z) <= d_box` para todo `Z` |
| E3 | Electronegatividad de Pauling | `F = 3.98` (máximo), `O = 3.44`, `Cl = 3.16`, `Cs/Fr ≈ 0.79`, gases nobles `He/Ne/Ar = N/A` | Gases nobles sin esfera falsa; `F` ocupa caja completa |
| E4 | Puntos de fusión y ebullición | `W` máximo metal (`Tm ≈ 3695 K`), `Hg ≈ 234.3 K`, `He` mínimo | Orden y conversión °C ↔ K exactos ($\pm 0.2\text{ K}$) |
| E5 | Manejo de valores negativos (`Enthalpy of Formation` / `Electron Affinity`) | Valores $< 0$ conservan magnitud $|v|/M$ y etiqueta de signo negativo | Sin radios negativos ni `NaN` |
| E6 | Consulta cruzada a `SpeciesCatalogPort` y exportación CSV | Al inspeccionar `Fe` o `Cu`, lista especies reales de `Maindb6`; CSV redondo mantiene columnas y unidades | Sin pánico ni pérdida de filas |
