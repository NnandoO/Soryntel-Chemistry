# Módulo 3: Heat Loss / Heat Transfer Calculations (`HeatLoss.exe`)

## 1. Resumen Ejecutivo

Este directorio contiene la investigación documental, análisis visual, ingeniería inversa estática, especificación matemática/científica y planificación de implementación y validación del **Módulo 3 de HSC Chemistry: Heat Loss / Heat Transfer Calculations** (`12-heat-loss`, binario `HeatLoss.exe`), orientado a su implementación independiente en **Rust** dentro del workspace **Soryntel Chemistry** (`soryntel`).

El módulo resuelve transferencia de calor estacionaria unidimensional (1D) en sistemas multicapa compuestos por **conducción** en sólidos, **convección** libre y forzada en superficies externas e internas y en cavidades/encerramientos (*enclosures*), **radiación superficial** gris difusa, **radiación de mezclas de gases** ($\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$) basada en el **Exponential Wide Band Model (EWBM)** y **radiación combinada gas–partícula**, además de análisis paramétrico (*Diagram*) y búsqueda de objetivo (*Target*).

## 2. Identificación del Módulo en `/home/nando/Descargas/HSC6`

- **Ejecutable principal:** `/home/nando/Descargas/HSC6/HeatLoss.exe` (PE32 x86, 2,301,952 bytes, VB6 nativo con `MSVBVM60.DLL`, SHA-256 `d08f12967191655ed6407d25e2a8f4884ceefde15535f921babbcaf341c49876`).
- **Bases de datos asociadas (`/home/nando/Descargas/HSC6/Databases/`):**
  - `heatcond.xls` (395,776 bytes) — conductividad térmica $k(T)$ y densidad $\rho$ de sólidos/refractarios/aislantes/metales.
  - `heatconv.xls` (172,032 bytes) — propiedades termofísicas de fluidos ($T, \rho, c_p, k, \mu, \nu, Pr, \beta$) y funciones $h_c(v)$.
  - `heatradi.xls` (28,160 bytes) — emisividades normales `(n)` y hemisféricas `(h)` de materiales superficiales.
- **Archivos de ejemplo (`/home/nando/Descargas/HSC6/HeatLoss/`):** `SimpleCase.HTR`, `Radiator.HTR`, `Hotwatertank.HTR`, `OilFurnace.HTR`, `Reactor1.HTR`, `Reactor2.HTR`, `Smelting1.HTR`–`Smelting3.HTR`, `incropera_92_494.HTR`, `incropera_94_503.HTR`, `incropera_94_504.HTR`, `Processor.HTR`, `Window.HTR`, además de hojas Excel de referencia (`*.XLS`).
- **Manual oficial:** `/home/nando/Documentos/Docs HSC/12-heat-loss/12-heat-loss.md` (37 páginas, secciones 12.1–12.5, ecuaciones 1–53).
- **Capturas de pantalla archivadas:** [`imagenes/`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/imagenes/) (`fig01_smelting1.jpg` a `fig14_optimum_insulation.jpg`).

## 3. Índice de Entregables Obligatorios

1. [`README.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/README.md) — Índice y resumen ejecutivo del Módulo 3.
2. [`MANUAL_ANALYSIS.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/MANUAL_ANALYSIS.md) — Análisis exhaustivo del manual oficial (`12-heat-loss.md`).
3. [`IMAGE_ANALYSIS.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/IMAGE_ANALYSIS.md) — Análisis visual de las 14 capturas de pantalla (`fig01`–`fig14`).
4. [`UI_SPECIFICATION.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/UI_SPECIFICATION.md) — Especificación de interfaz de usuario en `soryntel-desktop` (`egui`), alineada con los Módulos 1 y 2.
5. [`FUNCTIONAL_SPECIFICATION.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/FUNCTIONAL_SPECIFICATION.md) — Modos de cálculo, columnas, restricciones, bases de datos, Target, Diagram y exportación al Módulo 2.
6. [`MATHEMATICAL_SPECIFICATION.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/MATHEMATICAL_SPECIFICATION.md) — Ecuaciones 1–53, correlaciones adimensionales, EWBM y algoritmo del resolvedor acoplado 1D.
7. [`SCIENTIFIC_REFERENCES.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/SCIENTIFIC_REFERENCES.md) — Referencias bibliográficas primarias y contraste científico.
8. [`REA_ANALYSIS.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/REA_ANALYSIS.md) — Evidencia obtenida con Reverse Engineer Anything (REA) e inspección estática PE32/strings.
9. [`GHIDRA_ANALYSIS.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/GHIDRA_ANALYSIS.md) — Evaluación de necesidad y alcance de decompilación en Ghidra sobre `HeatLoss.exe`.
10. [`IMPLEMENTATION_PLAN.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/IMPLEMENTATION_PLAN.md) — Plan de integración en el Cargo Workspace `soryntel` (`soryntel-core`, `soryntel-db`, `soryntel-thermo`, `soryntel-desktop`).
11. [`VALIDATION_PLAN.md`](file:///home/nando/Documentos/HSC-REA/03-heat-loss/VALIDATION_PLAN.md) — Casos de prueba reproducibles del manual y tolerancias numéricas (`approx`).

## 4. Criterios de Evidencia y Política de Entorno

- Toda la investigación se ejecutó en **modo de solo lectura** sobre `/home/nando/Descargas/HSC6/`.
- **Cero Wine/Proton:** No se ejecutó ningún binario Windows.
- Cada documento distingue explícitamente entre: (a) información documentada en el manual, (b) comportamiento observado en capturas y archivos de datos, (c) evidencia de análisis estático (REA/PE/strings), e (d) hipótesis técnicas por validar en implementación.
