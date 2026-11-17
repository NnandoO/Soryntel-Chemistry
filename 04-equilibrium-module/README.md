# Módulo 4 — Equilibrium Calculations (Gibbs Energy Minimization)

> **Estado:** Evaluación documental, visual, de ingeniería inversa estática y especificación matemática/arquitectónica completada (sin código Rust implementado aún).  
> **Manual base:** [13-equilibrium-module.md](file:///home/nando/Documentos/Docs%20HSC/13-equilibrium-module/13-equilibrium-module.md) (57 páginas, 1501 líneas).  
> **Binarios HSC6 analizados:** `/home/nando/Descargas/HSC6/GIBBS.EXE`, `/home/nando/Descargas/HSC6/Gibbs5.dll`.  
> **Imágenes archivadas:** [imagenes/](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/imagenes/) (38 capturas PNG).

## Resumen Ejecutivo

El **Módulo 4 (Equilibrium Calculations / Gibbs)** calcula la composición química de equilibrio multifase y propiedades termodinámicas derivadas minimizando la **Energía Libre de Gibbs total ($G$)** sujeta a balances atómicos lineales ($A n = b, \; n \ge 0$), basándose en el algoritmo clásico de **White, Johnson & Dantzig (1958)** y extendido con equilibrio electroquímico de celdas (**Lampinen & Vuorisalo, 1991/1992**), coeficientes de actividad no ideales (**Redlich–Kister**, **Pitzer/Aqua**, y fórmulas definidas por el usuario), modos inversos (**Adiabatic `N,H,P`**, **Constant Volume `N,P,V`**, **Target Calculations**) y restricciones abiertas (**Infinite Phase / Open Atmosphere**, **Fixed Activity**, **Titrations**).

## Índice de Entregables (11/11)

1. [README.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/README.md) — Índice general y resumen ejecutivo.
2. [MANUAL_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/MANUAL_ANALYSIS.md) — Análisis exhaustivo de las secciones 13.1–13.11 y Apéndices A–C del manual.
3. [IMAGE_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/IMAGE_ANALYSIS.md) — Inventario y análisis visual de las 38 capturas de pantalla.
4. [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/UI_SPECIFICATION.md) — Especificación de interfaz estilo Excel (`excel_grid`) + panel `Parameters` + gráficos 2D/3D + diálogos.
5. [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/FUNCTIONAL_SPECIFICATION.md) — Requisitos funcionales, flujos de trabajo, modos de cálculo y manejo de errores.
6. [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/MATHEMATICAL_SPECIFICATION.md) — Formulación completa: GEM (WJD 1958), KKT/Lagrange, fase activa pura, Redlich–Kister, Pitzer/Aqua, `N,H,P`, `N,P,V`, Target, Cell Mode.
7. [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/SCIENTIFIC_REFERENCES.md) — Referencias científicas, constantes termodinámicas y modelos de solución.
8. [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/REA_ANALYSIS.md) — Análisis estático PE32/COM/strings de `GIBBS.EXE` y `Gibbs5.dll` y estado honesto de `rea`.
9. [GHIDRA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/GHIDRA_ANALYSIS.md) — Evaluación de necesidad y alcance de decompilación en Ghidra sobre `Gibbs5.dll`.
10. [IMPLEMENTATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/IMPLEMENTATION_PLAN.md) — Integración al Cargo Workspace de `soryntel` (`soryntel-core`, `soryntel-thermo`, `soryntel-db`, `soryntel-desktop`).
11. [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/04-equilibrium-module/VALIDATION_PLAN.md) — Casos de prueba golden extraídos de `.IGI`/`.OGI` (`FES2`, `CACO3`, `AMMONIA`, `GAAS`, `LEADBATT`) con tolerancias `approx`.
