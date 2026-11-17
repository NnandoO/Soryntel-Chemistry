# Módulo 15: Excel Add-Ins (Funciones Termoquímicas de Hoja de Cálculo)

## Propósito
El módulo **Excel Add-Ins** (`27. Excel Add-In Functions` en HSC Chemistry 6) expone el motor termoquímico y la base de datos de especies como **funciones nativas de hoja de cálculo** (`=H(...)`, `=G(...)`, `=K(...)`, `=TatH(...)`, `=StreamH(...)`, `=FlowH(...)`, `=StreamEQ(...)`, etc.). En HSC6 este puente se implementa mediante la interfaz binaria `AddIns/HSC6.XLL` (69 funciones registradas bajo la categoría `"HSC"` en Excel) apoyada en el servidor COM/DLL `HSC6.DLL`, junto con el libro de ejemplo `AddIns/AddInSample.xls` y la utilidad de migración `AddIns/Update XLS Files.xls`.

En **Soryntel**, este módulo cumple un doble papel:
1. **Motor de fórmulas termoquímicas en hoja de cálculo** reutilizable dentro de la aplicación (hoja interactiva estilo `AddInSample.xls`, evaluador matricial `StreamEQ` y asistentes de fórmulas).
2. **Referencia completa, probador en vivo y migrador** de las **69 funciones XLL** de HSC6 (además de las **5 funciones exclusivas de simulación mineral** `StreamX`, `RecoveryX`, `ParticleRecX`, `FractionX`, `MineralX`), manteniendo compatibilidad sintáctica (`;`/`,`), unidades globales (`UNITS(T; E)`) y paridad numérica con HSC Chemistry 6.

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Análisis de ingeniería inversa de `Help/27 Excel Add-Ins.txt`, `Help/35 HSC DLL Tools.txt`, `AddIns/HSC6.XLL` (las 69 funciones `xlfRegister`), `AddIns/AddInSample.xls` (122 filas con resultados reales) y `AddIns/Update XLS Files.xls`.
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Formulación exacta de las 9 familias de funciones (`General`, `Species`, `Reaction`, `Reverse Iteration`, `Stream/Flow`, `Stream/Flow Reverse`, `Temp. Independent`, `Percentage`, `Cp-function`) más la función matricial de equilibrio `StreamEQ` y conversión global `UNITS`.
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Casos de uso (`CU-01` libro evaluador interactivo tipo `AddInSample`, `CU-02` catálogo/asistente de las 69 funciones, `CU-03` equilibrio matricial `StreamEQ` con bloques `§ PHASE`, `CU-04` migrador de fórmulas heredadas `HSC5.xla -> HSC6` y exportación).
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Especificación visual del módulo en `soryntel-desktop`: barra de fórmulas estilo Excel, selector global `UNITS(T, E)` y pestañas `Add-In Workbook | Function Reference | StreamEQ & Migration`.
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Casos de validación (`X1–X8`) contrastados directamente contra los valores de referencia extraídos de `AddInSample.xls` (`CO2(g)`, `2H2(g)+O2(g)=2H2O(g)`, `Tat*`, `Stream*/Flow*`, `DensityA`, `MolP`/`WTP`, `CPFUNCTION` y `StreamEQ`).
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Fuentes primarias de HSC6, Microsoft Excel XLL SDK y modelos termoquímicos/numéricos subyacentes.
