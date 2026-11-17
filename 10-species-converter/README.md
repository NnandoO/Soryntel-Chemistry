# Módulo 10: Species Converter (HSC Chemistry)

## Propósito
El módulo **Species Converter** (evolución de *Mineralogy Iterations* / *Composition Conversions*) permite convertir análisis químicos elementales (y/o mezclas parciales de especies y elementos) en composiciones mineralógicas/especies químicas en `wt-%`, y viceversa, mediante optimización no negativa de mínimos cuadrados (`NNLS`) sujeta a restricciones de balance elemental, valores objetivo (`Target wt-%`), coeficientes de peso (`Weight Coeff`), ponderación por estabilidad termodinámica ($\Delta G^\circ$) y ajuste automático de balance elemental (`Fix Element Balance`).

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Descomposición funcional, arquitectura hexagonal y casos de uso.
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Formulación matricial, Métodos 1/2/3, Error Residual, Balance Elemental y *Stability Weighting*.
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Reglas de negocio, entrada/salida, `Initialize`, `Solve`, `Fix Element Balance` y validación.
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Cinta `Home`/`Tools`, hoja tipo Excel (`INPUT` / `OUTPUT`), ventana `Element Balance` y panel `Charts`.
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Casos de referencia con capturas HSC (`Cu-S-Fe-Ni`, `Example Conversion`, `Fix Element Balance`, `Initialize`).
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Referencias de álgebra lineal no negativa y mineralogía computacional.
