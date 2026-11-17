# REA Analysis — Módulo 09: Phase Stability Diagrams (`Tpp` & `Lpp`)

## 1. Fuente oficial analizada
- `/home/nando/Descargas/HSC6/Help/21 Phase Stability Diagrams.txt` (secciones `21.1`–`21.5`, módulos `Tpp` y `Lpp` / `PSD`).

## 2. Diferencias entre `Tpp` y `Lpp` en HSC6 y unificación en `soryntel`
- **`Tpp Diagram`**: soporta tanto **diagramas `p-p` isotérmicos** ($\log_{10} p_1$ vs $\log_{10} p_2$) como **diagramas `T-p`** ($T$ vs $\log_{10} p_2$ con $p_1$ fijo), pintando áreas de estabilidad por color y resolución configurable.
- **`Lpp Diagram` (`PSD`)**: calcula diagramas `p-p` isotérmicos con líneas vectoriales exactas de frontera y tabla de coordenadas (`Data` sheet).
- **En `soryntel`**: unificamos ambos en un único motor **`soryntel-thermo/src/phase_stability.rs`** que calcula la minimización de Gibbs sobre la grilla 2D + refinamiento por bisección de cruces (al igual que el módulo `Eh-pH`), entregando simultáneamente **áreas pintadas (`Tpp`)**, **segmentos vectoriales exactos (`Lpp`)** y **etiquetas centradas** en cada región.

## 3. Capacidades existentes reutilizables
- `soryntel_db::ThermoDatabase::search_by_elements`: filtrado inmediato de especies condensadas y gaseosas para los 3 elementos seleccionados.
- `soryntel_thermo::evaluate_species_si`: evaluación de $G^\circ(T)$ en SI para todas las especies condensadas y los gases de los ejes.
- `soryntel_solver::parse_term`: extracción de coeficientes estequiométricos $(m_i, a_i, b_i)$ de cada fórmula.

## 4. Brechas técnicas a implementar
1. **Validación del sistema ternario**: exactamente 3 elementos $\{M, A, B\}$, identificación automática de $M$ (elemento principal contenido en las fases condensadas seleccionadas y ausente en los gases de eje $G_1, G_2$) y verificación de independencia lineal de los gases ($\det C \ne 0$).
2. **Motor `phase_stability.rs`**: soporte para modo `PP` ($\log_{10} p_1$ vs $\log_{10} p_2$ a $T$ fijo) y modo `TP` ($T$ en `°C`/`K` vs $\log_{10} p_2$ a $p_1$ fijo), resolución configurable (`Low`, `Medium`, `High`), extracción de segmentos de frontera y tabla de especies con $\Delta G(T)$ y coordenadas.
3. **Interfaz `phase_stability_ui.rs`**: selección de 3 elementos, lista de especies, modo `T-p` / `p-p`, opciones `Grid`, `Color`, `Resolution`, `Table/Data` y lienzo interactivo con áreas coloreadas, líneas vectoriales y etiquetas.
