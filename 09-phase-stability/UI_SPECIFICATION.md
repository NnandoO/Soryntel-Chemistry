# UI Specification — Módulo 09: Phase Stability Diagrams

*(Especificación preliminar basada en `21 Phase Stability Diagrams.txt`; se ajustará cuando el usuario comparta capturas oficiales).*

## 1. Estructura de la ventana (`--module phase_stability`)
- **Panel izquierdo redimensionable (`Diagram Parameters`, `ScrollArea::both()`)**:
  - Grilla compacta de elementos (límite máximo 3 elementos activos; indicador visual de los 3 seleccionados).
  - Lista multi-selección de especies condensadas + `Select All` / `Clear`.
  - Selector de modo: `p-p Diagram` / `T-p Diagram`.
  - Controles de `X-axis`, `Y-axis` y `Fixed Variable` (`Temperature` o `Partial Pressure`).
  - Opciones `Grid`, `Color`, `Resolution` (`Low / Medium / High`) y botón **`Diagram`**.
- **Panel central con pestañas (`ScrollArea::both()`)**:
  - **`Diagram`**: lienzo blanco con áreas coloreadas opcionales, fronteras negras nítidas, etiquetas centradas por fase e inspector al pasar el cursor (`X, Y, Phase`).
  - **`Data (Table)`**: tabla estilo Excel con especies, $\Delta G(T)$ y coordenadas de fronteras.

## 2. Requisitos de responsividad
- Paneles laterales `.resizable(true)` y tamaño mínimo de ventana `[480.0, 360.0]`.
