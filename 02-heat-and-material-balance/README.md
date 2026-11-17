# Módulo 2: Heat and Material Balance (Balance de Materia y Calor)

## 1. Resumen Ejecutivo

Este directorio contiene la investigación técnica, documental, visual, matemática y arquitectónica del **Módulo 2: Heat and Material Balance (`Balance`)** de HSC Chemistry, orientada a su implementación nativa en **Rust** dentro del workspace **`soryntel`** de **Soryntel Chemistry**.

A diferencia del Módulo 1 (`Reaction Equations`), que evalúa reacciones individuales o tablas tabulares en función de la temperatura, el **Módulo 2 (`Heat and Material Balance`)** modela procesos completos (hornos, convertidores, calderas, precipitadores, evaporadores, combustión adiabática) mediante:
1. Hojas de cálculo estructuradas por **Áreas de Balance** (`IN1`/`OUT1`, ..., `INn`/`OUTn`, `BALANCE`), **Corrientes (`Streams`)**, especies de la base de datos termodinámica, y flujos térmicos adicionales (`EXTRA HEAT:` y `HEAT LOSSES:`).
2. Cálculo automático en tiempo real de **Balance de Elementos** (`kmol` y `kg`), **Balance Total** (`kmol`, `kg`, `Nm³`, `Heat Content`, `Total H`, `Exergy`), y composiciones elementales (`Inx-%` / `Outx-%`).
3. Evaluación de propiedades termodinámicas de especies puras y corrientes (incluyendo corrección de presión para `H2O` / vapor mediante funciones tipo IAPWS), así como resolución numérica inversa:
   - **Temperature Balance (`Temp. Balance`)**: temperatura adiabática de salida cuando $\Delta H_{\text{total}} = 0$.
   - **Target Dialog (Goal Seek)**: optimización/búsqueda de raíz acotada de variables de proceso con soporte de múltiples restricciones acopladas.
   - **Diagram Settings / Diagram**: barridos paramétricos 1D con iteración anidada de `Target` por punto.
   - **Flujos circulares / múltiples áreas de balance**: evaluación por punto fijo / sustitución sucesiva o métodos cuasi-Newton (Broyden).

## 2. Índice de Entregables (11 Documentos Obligatorios)

| # | Documento | Propósito |
|---|---|---|
| 1 | [README.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/README.md) | Índice maestro, alcance e integración con el Módulo 1 y `soryntel`. |
| 2 | [MANUAL_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/MANUAL_ANALYSIS.md) | Análisis exhaustivo de las 24 páginas y 11 secciones del manual oficial (`11-heat-and-material-balance.md`). |
| 3 | [IMAGE_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/IMAGE_ANALYSIS.md) | Inspección visual directa de las 19 capturas extraídas del manual (`raw-001.png` a `raw-038.png`, Figs. 1–14). |
| 4 | [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/UI_SPECIFICATION.md) | Especificación de interfaz de escritorio (`soryntel-desktop`) basada en las capturas del Módulo 2 y consistente con el Módulo 1. |
| 5 | [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/FUNCTIONAL_SPECIFICATION.md) | Especificación funcional completa: hojas, corrientes, fórmulas, `Target`, `Diagram`, `Temp. Balance`, áreas múltiples y exportación. |
| 6 | [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/MATHEMATICAL_SPECIFICATION.md) | Formulación matemática rigurosa: `Heat Content`, `Total H`, `Exergy`, corrección `H2O(P,T)`, algoritmos numéricos (`Brent`, `Broyden`, punto fijo). |
| 7 | [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/SCIENTIFIC_REFERENCES.md) | Referencias bibliográficas termodinámicas, exergéticas, de vapor (`IAPWS-95`/`IF97`) y métodos numéricos. |
| 8 | [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/REA_ANALYSIS.md) | Evidencia estática extraída de `HSC.EXE`, `HSC6.DLL` y archivos de ejemplo `/Balance/*.BAL` y `*.OHE` mediante REA y `strings`. |
| 9 | [GHIDRA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/GHIDRA_ANALYSIS.md) | Evaluación de necesidad de decompilación con Ghidra para el Módulo 2 y justificación técnica. |
| 10 | [IMPLEMENTATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/IMPLEMENTATION_PLAN.md) | Plan de integración en el Cargo Workspace `soryntel` (`soryntel-core`, `soryntel-thermo`, `soryntel-solver`, `soryntel-db`, `soryntel-desktop`). |
| 11 | [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/02-heat-and-material-balance/VALIDATION_PLAN.md) | Casos de prueba numéricos reproducibles (`BUTANE.OHE`, `H2O.OHE`, `CUSMELT.OHE`, `CUCONV.OHE`, `CUCONV1`, `FSF_process`) con tolerancias `approx`. |

## 3. Continuidad con el Módulo 1 y Evidencia Base

- **Instalación analizada (solo lectura):** `/home/nando/Descargas/HSC6/` (`HSC.EXE`, `HSC6.DLL`, `/Balance/*.BAL`, `/Balance/*.OHE`).
- **Base de datos reutilizada:** `Maindb6.HSC` (36,537 registros de 256 bytes, ya decodificada e indexada por `soryntel-db` en SQLite/memoria).
- **Motor reutilizado del Módulo 1:**
  - `soryntel_core`: constantes físicas (`T_REF_K = 298.15`, `CAL_TO_JOULE = 4.184`, `STP_MOLAR_VOL_L = 22.414`), unidades y errores.
  - `soryntel_solver`: parser de fórmulas químicas (`parse_formula_composition`, `parse_term`) para descomposición estequiométrica automática en el **Balance de Elementos**.
  - `soryntel_thermo`: `SpeciesRepository` y `evaluate_species_si` ($C_p(T)$, $H(T)$, $S(T)$, $G(T)$ por segmentos polinómicos HSC y transiciones de fase).
  - `soryntel_db`: `ThermoDatabase` para búsqueda y obtención de especies en botones `Input` / `Output` (`Get Species from Database`).
