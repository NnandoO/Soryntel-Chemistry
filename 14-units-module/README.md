# Módulo 14: Units Module (Conversión de Unidades y Tablas de Referencia)

## Propósito
El módulo **Units** proporciona una calculadora rápida bidireccional para la conversión de magnitudes físicas, termoquímicas y de ingeniería entre más de **90 magnitudes** y **~444 unidades** combinables con **prefijos métricos SI** (`Y` hasta `y`), junto con un libro de **6 hojas de datos editables** estilo HSC Chemistry 6 (`Databases/UNITS4.UNI`): calculadora (`Convert`), catálogo maestro de factores (`Units`), constantes físico-químicas (`Constants`), tamices/granulometría (`Mesh`), composición y humedad del aire (`Air`) y presión de vapor de agua (`H2O`).

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Análisis de ingeniería inversa de `Help/26 Units Module.txt` y del libro binario `Databases/UNITS4.UNI` (estructura de las 6 hojas, anomalías heredadas y encaje en la arquitectura hexagonal de `soryntel`).
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Ecuaciones de conversión lineal por factor base y prefijos SI (incluyendo regla especial para `kilogram`), conversión afín de temperatura (`K`, `°C`, `°F`, `°R`), interpolación/cálculo psicrométrico de `Air` y curvas de vapor de `H2O`.
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Casos de uso (`CU-01` conversión bidireccional instantánea, `CU-02` edición/ampliación de la base de datos de unidades, `CU-03` consulta y cálculo en hojas `Constants`, `Mesh`, `Air` y `H2O`, `CU-04` persistencia y exportación).
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Especificación visual fiel al módulo clásico `Units` de HSC6 (`File | Edit | Format | Help`), hoja `Convert` con selectores duales `(Unit 1 / Prefix 1 ↔ Unit 2 / Prefix 2)` y pestañas/hojas de libro estilo Excel (`Units | Constants | Mesh | Air | H2O`).
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Matriz de pruebas de paridad (`U1–U6`) con energía, presión, temperatura, masa con prefijos, constantes CODATA, tamiz Tyler/ASTM y humedad del aire.
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Referencias primarias (HSC6 `UNITS4.UNI`, NIST SP 811, CODATA, ASTM E11 / Tyler Mesh, psicrometría de aire húmedo e IAPWS).
