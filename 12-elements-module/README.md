# Módulo 12: Elements Module (Propiedades Periódicas de los Elementos)

## Propósito
El módulo **Elements** ofrece una base de datos interactiva en formato hoja de cálculo con las **56 propiedades físicas, químicas, cristalográficas y termoquímicas** fundamentales de los elementos químicos (`Z = 1..118`, estilo `ELEMENT4.ELE` de HSC Chemistry), junto con un **Diagrama de Tabla Periódica** proporcional por esferas (donde el diámetro de cada esfera representa la magnitud de la propiedad seleccionada respecto al máximo del sistema periódico) y gráfico de tendencia frente al número atómico $Z$.

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Análisis de fuentes en HSC6 (`24 Elements Module.txt`, `ELEMENT4.ELE`), esquema de 56 propiedades y rol en la arquitectura hexagonal de `soryntel`.
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Normalización proporcional de esferas (`d(Z) / d_box`), manejo de signos negativos/faltantes, estadísticas descriptivas y conversión de unidades.
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Casos de uso (`CU-01` hoja Excel con `Fix Left 2 Columns`, `CU-02` diagrama periódico interactivo y doble clic, `CU-03` gráfico de tendencia periódica, `CU-04` edición/exportación y consulta al catálogo).
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Especificación de interfaz responsiva: barra superior, tabla periódica de 18 grupos + bloque *f*, grilla tipo Excel editable y modal de ficha de elemento.
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Casos de verificación (`E1–E6`) de pesos atómicos IUPAC, densidades, electronegatividad de Pauling, puntos de fusión, invariantes del diagrama de esferas y exportación.
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Referencias primarias (HSC6 `ELEMENT4.ELE`, IUPAC CIAAW, CRC Handbook, NIST, Pauling, Shannon).
