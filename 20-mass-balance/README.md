# Módulo 20: Balance de Masa y Reconciliación de Datos (`HSC - Mass Balance / Mas`)

Especificación *clean-room* completa para el módulo **`HSC - Mass Balance`** (**Mas**), herramienta de reconciliación estadística de datos experimentales de planta e integración bidireccional con diagramas de flujo **HSC Sim** (entrada desde flujograma/Excel HSC7 → reconciliación de masa ponderada por incertidumbre → retroalimentación hacia *Model Fit* y simulación).

---

## Alcance del Módulo

A diferencia del módulo de balance termoquímico estequiométrico ([Módulo 02 — Heat & Material Balance / Bal](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/balance.rs)), el módulo **`Mass Balance` (`Mas`)** resuelve el **problema inverso estadístico de cierre de balance de planta** cuando existen mediciones experimentales redundantes e inconsistentes por error de muestreo/análisis o flujos no medidos:

1. **Topología y Selección (`51.4.1–51.4.2`):** selección de unidades del diagrama (con indicador de transformación granulométrica *Size Transforming*), corrientes (`Solids/Slurry`, `Liquid/Water`, `Unknown`), variables (`SF`, `SC`, `A`, `M`, `SA`, `SP`, `Info`), fracciones de tamaño (`Bulk` y tamices) y múltiples conjuntos de datos (*Datasets*) o rondas de muestreo (*Sampling Rounds*).
2. **Importación, Estimación y Conversión (`51.4.2, 51.4.7`):** entrada manual o por portapapeles (orientación horizontal/vertical), importación heredada HSC7 `Analyses.xls` (`1D`, `1.5D`, `2D`), conversión Elemento ↔ Mineral vía catálogo mineralógico (*HSC Geo ch. 84.6*), `% Sólidos` a flujo de agua, estimación de faltantes, ecuaciones entre corrientes (*Stream by Equation*) y carga de recuperaciones para *Model Fit*.
3. **Modelos de Error e Incertidumbre (`51.4.3–51.4.4`):** asignación de desviaciones estándar (`SD`) y relativas (`RSD-%`) mediante `5` modelos parametrizables (`Fixed Absolute`, `Fixed Relative`, `S-shape`, `Mineral Grade Based`, `Size Fraction (Whiten)`), multiplicadores de calidad/muestreo de corriente y cálculo estadístico desde rondas de muestreo.
4. **Motor Matemático en Dos Etapas (`51.4.5, 51.7`):**
   - **Etapa 1 (Caudales):** mínimos cuadrados totales ponderados por elemento (*EWTLS*) para caudales totales de sólidos, agua/líquido y subflujos granulométricos, con escalamiento por SD de corriente de alimentación y restricción opcional de suma `100%`.
   - **Etapa 2 (Leyes y Fracciones):** mínimos cuadrados ponderados lineales (*WLS*) sobre la matriz de incidencia resuelta $B_{\text{unit,flow}} = e_{\text{flow}}^{\text{unit}} F_{\text{flow,tot}}$, bajo variantes `LS` (incluyendo norma de Chebyshev `LS, MinMax`), `NNLS`, `CLS` (límites `Min/Max`) y `LLS`.
5. **Diagnóstico, Reglas de Validación y Reportes (`51.4.6, 51.6`):** gráfica de convergencia (`Iterated Tolerance` vs ronda), gráfico de paridad log-log, barras de recuperación, indicadores de bondad de ajuste (`WSSQ`, `RMSD`, `Relative RMSD`, `±CI`), balances por unidad, gráficos comparativos y exportación hacia tablas `.Sim` / `.HSCStream` / Excel / `.HSCMas`.

---

## Índice de Especificaciones

| Documento | Descripción | Enlace |
|---|---|---|
| Análisis de Ingeniería Inversa (`REA`) | Inventario documental, descomposición arquitectónica, flujos de datos y auditoría de brechas en `soryntel` | [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/REA_ANALYSIS.md) |
| Especificación Matemática | Ecuaciones `(1)–(29)`, EWTLS, WLS, modelos de error (`SD`/`RSD%`), escalamiento MATLAB y diseño *clean-room* numérico | [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/MATHEMATICAL_SPECIFICATION.md) |
| Especificación Funcional | Casos de uso `CU-01..CU-12`, catálogo de acciones por vista (`51.5`) y las `12` reglas de verificación de errores (`51.6`) | [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/FUNCTIONAL_SPECIFICATION.md) |
| Especificación de Interfaz (`UI`) | Anatomía exacta (`Balancing Navigator`, *Ribbon*, *Working Area*, *Property Panel*), paletas semánticas y plano `soryntel-desktop` | [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/UI_SPECIFICATION.md) |
| Plan de Validación | Matriz cuantitativa `MB-01..MB-10` con cifras oficiales de verificación (`Fig. 16–23`, `Tablas 1–4`, `51.6`) | [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/VALIDATION_PLAN.md) |
| Referencias Científicas | Bibliografía oficial del capítulo 51 y referencias algorítmicas *clean-room* | [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/20-mass-balance/SCIENTIFIC_REFERENCES.md) |

---

## Fuentes Primarias Verificadas

- Manual oficial (Markdown extraído, `1036` líneas): [51-mass-balance.md](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.md)
- Manual oficial (PDF original, `39` páginas, 2023-09-05): [51-mass-balance.pdf](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.pdf)
- Capturas oficiales (`34` imágenes PNG): `/home/nando/Documentos/Docs HSC/51-mass-balance/imagenes/`
