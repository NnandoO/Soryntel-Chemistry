# Módulo 01: Reaction Equations (Ecuaciones de Reacción) — Soryntel Chemistry

## 1. Alcance y Propósito del Módulo
El módulo **Reaction Equations** es el núcleo analítico fundamental para el cálculo de funciones termodinámicas y propiedades de sustancias puras y reacciones químicas en función de la temperatura $T$. 

En el contexto de **Soryntel Chemistry**, este módulo tiene como objetivo proporcionar una implementación independiente de altísimo rendimiento en **Rust** (arquitectura `soryntel-core` / `soryntel-thermo` con Tauri v2), sustituyendo los binarios legados basados en 32-bits y Visual Basic 6 de HSC Chemistry v6 por un motor numéricamente robusto, concurrente, con seguridad estricta de tipos de magnitudes físicas (`f64`, unidades dimensionales seguras) y sin dependencia de código propietario.

## 2. Capacidades Funcionales Principales
1. **Sustancia Química Individual (*One Chemical Substance*):**
   - Evaluación tabular y gráfica de magnitudes termodinámicas absolutas en función de la temperatura:
     - Capacidad calorífica a presión constante $C_p(T)$
     - Entalpía $H(T)$
     - Entropía $S(T)$
     - Energía libre de Gibbs $G(T)$
   - Formatos de salida:
     - **Normal (Escala Absoluta):** Convención Barin-Knacke-Kubaschewski ($H$, $S$, $C_p$, $G$).
     - **Delta (Funciones de Formación):** $\Delta H_f(T)$, $\Delta S_f(T)$, $\Delta G_f(T)$ referenciadas a los elementos constituyentes en su estado estándar más estable a $T$.
   - Identificación de discontinuidades y transiciones de fase (*Show Transitions*): cambios alotrópicos/polimórficos, fusión, ebullición.
   - Extrapolación de propiedades a alta temperatura ($25\,^\circ\text{C}$ a $300\,^\circ\text{C}$) para especies iónicas y electrolitos acuosos mediante el modelo de **Criss-Cobble**.

2. **Ecuaciones de Reacción Química (*Reaction Equations*):**
   - Cálculo estequiométrico de funciones de reacción:
     $$\Delta H_r(T) = \sum \nu_i H_i(T)$$
     $$\Delta S_r(T) = \sum \nu_i S_i(T)$$
     $$\Delta G_r(T) = \sum \nu_i G_i(T)$$
   - Cálculo de la constante termodinámica de equilibrio químico $K(T)$ y su forma logarítmica decimal $\log_{10} K(T)$.
   - Cálculo del potencial electroquímico estándar $E^\circ(T)$ versus el Electrodo Estándar de Hidrógeno (SHE) según la convención de signos IUPAC cuando la reacción involucra intercambio electrónico ($e^-$).
   - Balance automático estequiométrico por resolución de matrices de conservación elemental.
   - Cálculo de balance de masa, pesos fórmula, concentración porcentual en masa (wt-%), cantidades molares y volumétricas molares ideales.
   - Análisis de presiones de vapor de equilibrio mediante reacciones de volatilización/sublimación/evaporación ($A(s, l) = A(g)$).

## 3. Estructura de Documentación de la Investigación
Esta carpeta contiene los 11 entregables técnicos y científicos que completan la fase de investigación:
- [MANUAL_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/MANUAL_ANALYSIS.md) — Análisis exhaustivo de los capítulos del manual, flujos y opciones.
- [IMAGE_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/IMAGE_ANALYSIS.md) — Inspección detallada y trazabilidad de las 9 imágenes del módulo.
- [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/UI_SPECIFICATION.md) — Especificación de controles, ventanas, formularios e interacciones para el frontend Tauri v2.
- [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/FUNCTIONAL_SPECIFICATION.md) — Casos de uso, validaciones, balance estequiométrico y estructura de archivos `.REA` / `.simjson`.
- [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/MATHEMATICAL_SPECIFICATION.md) — Modelado termodinámico riguroso, polinomios de Kelley, integrales analíticas, Criss-Cobble y estabilidad en `f64`.
- [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/SCIENTIFIC_REFERENCES.md) — Compilación bibliográfica formal (Barin, Knacke, Kubaschewski, Criss-Cobble, IUPAC, NIST).
- [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/REA_ANALYSIS.md) — Evidencia de ejecución con Reverse Engineer Anything (REA CLI v6.9.0) y análisis de artefactos PE.
- [GHIDRA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/GHIDRA_ANALYSIS.md) — Evaluación técnica sobre la necesidad y aplicabilidad de Ghidra en binarios VB6.
- [IMPLEMENTATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/IMPLEMENTATION_PLAN.md) — Arquitectura en Rust puro (`soryntel-core`, Tauri v2 IPC, structs, traits).
- [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/01-reaction-equations/VALIDATION_PLAN.md) — Banco de pruebas de regresión, casos reales y tolerancias numéricas con `approx`.
