# Módulo 07 — Eh-pH Diagrams (Diagramas de Pourbaix) · Documentación REA

Este directorio contiene el análisis y especificación **REA** (*Requirements, Engineering & Architecture*) para el **Módulo 07: Eh-pH Diagrams** (`EpH`), basado en los capítulos `17 Eh - pH - Diagrams.txt` y `18 Eh - pH - Samples.txt` de HSC6 y referencias verificadas en `32 References.txt`.

## Índice de Documentos
1. [`REA_ANALYSIS.md`](./REA_ANALYSIS.md) — Auditoría de `17 Eh - pH - Diagrams.txt`, base STABCAL, recursos ya disponibles en `soryntel` (`criss_cobble_eval`, `h2o_saturation_pressure_bar`, `Maindb6`) y brechas técnicas.
2. [`MATHEMATICAL_SPECIFICATION.md`](./MATHEMATICAL_SPECIFICATION.md) — Ecuaciones de Nernst/Pourbaix, envolvente inferior lineal 2D en `(pH, Eh)`, límites del agua, constante dieléctrica y presión de vapor mínima.
3. [`FUNCTIONAL_SPECIFICATION.md`](./FUNCTIONAL_SPECIFICATION.md) — Selección de elementos (`Main` + secundarios), temperaturas (`T1..T4`), hoja de especies (`DG`), molalidades, `Show Predominance Areas of Ions`, diagramas combinados (`Combine` hasta 4 Data Sets) y tabla de resultados.
4. [`UI_SPECIFICATION.md`](./UI_SPECIFICATION.md) — Propuesta de interfaz alineada con `17 Eh - pH - Diagrams.txt` (sujeta a ajuste final con capturas del usuario).
5. [`VALIDATION_PLAN.md`](./VALIDATION_PLAN.md) — Casos de validación (`Mn-H2O`, `Cu-S-H2O`, `Fe-S-H2O` combinado a `1`, `10^-3`, `10^-6 mol/kg`) y verificación de líneas de agua.
6. [`SCIENTIFIC_REFERENCES.md`](./SCIENTIFIC_REFERENCES.md) — Referencias verificadas en `32 References.txt` (Pourbaix 1966, Criss & Cobble 1964, Haung/STABCAL).
