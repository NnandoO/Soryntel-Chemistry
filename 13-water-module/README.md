# Módulo 13: Water Module (Calculadora Presión–Temperatura / Tablas de Vapor)

## Propósito
El módulo **Water (Pressure-Temperature Calculator / `PTCalc`)** proporciona una descripción termodinámica completa del agua pura ($\text{H}_2\text{O}$) en sus fases sólida (hielo), líquida, vapor y fluido supercrítico dentro de un rango operativo de **$-200\,^\circ\text{C}$ a $2000\,^\circ\text{C}$** y **$0.001\text{ bar}$ a $1000\text{ bar}$**, reemplazando las tablas de vapor tradicionales y diagramas de Mollier mediante cálculo directo de dos puntos de proceso (**Point 1** y **Point 2**) y su diagrama interactivo $p\text{-}T$.

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Análisis de fuentes originales HSC6 (`25 Water.txt`, `PTCalc/H2O.xls`, especies `H2O(*bar*)` en `Maindb6.HSC`) y encaje en la arquitectura hexagonal de `soryntel`.
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Ecuaciones de saturación IAPWS-IF97 / Wagner–Pruss (`Sat. (g,l)`, `Sat. (g,s)`, `Sat. (l,s)`), punto triple y crítico, corrección por presión de $H, S, G, C_p, H_{\text{specific}}, \rho$, regla de mezcla bifásica (`Mixture %`) y eficiencias isentrópicas de compresor/bomba y turbina.
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Casos de uso (`CU-01` cálculo interactivo Point 1 ↔ Point 2, `CU-02` estados saturados y regla de la palanca, `CU-03` clic y arrastre sobre diagrama $p\text{-}T$, `CU-04` atajo `Point 2 = Triple Point`, unidades y proceso isentrópico).
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Especificación de la interfaz fiel a HSC6: hoja de cálculo izquierda (unidades, cantidad, entradas `T, P, Phase, Mixture %` y salidas `H, S, H specific, G, Cp, Density` + columnas `Difference` en amarillo), diagrama log-$p$ vs $T$ interactivo derecho con puntos `[1]`/`[2]` y menús `File / Edit / Show / Help`.
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Casos de verificación (`W1–W6`): punto triple, punto crítico, fusión del hielo a `500 bar` ($\approx -4.5\,^\circ\text{C}$), caldera a `285 °C`, turbina de vapor ($\eta_{s,t} \approx 0.82$) y consistencia con `Maindb6.HSC`.
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Referencias científicas y normativas (HSC6 `PTCalc`, IAPWS-95, IAPWS-IF97, Wagner & Pruss 2002, NIST WebBook).
