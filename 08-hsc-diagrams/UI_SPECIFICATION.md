# UI Specification — Módulo 08: H, S, C and G Diagrams

*(Especificación preliminar basada en `19 H, S, C and G Diagrams.txt`; se ajustará cuando el usuario comparta capturas oficiales del módulo).*

## 1. Estructura de la ventana (`--module diagrams`)
- **Panel izquierdo / superior de control (`Diagram Menu`)**:
  - Selector **`Diagram Type`** (8 opciones).
  - Sección **`Ellingham (DG)`**: selector de **`Element`** (`O`, `S`, `Cl`, ...) + botón **`Balance Element Amount`**.
  - Rango y unidades: `T min`, `T max`, `Step`, `°C / K`, `kJ / kcal`.
  - Botones de acción principales: **`Read Data from Database`** y **`Diagram`**.
- **Panel central con pestañas**:
  - **Pestaña `X-data`**: hoja estilo Excel con columnas `Amount | Species Formula | Database No | Tmin (K) | Tmax (K) | Status`.
  - **Pestaña `Diagram`**: gráfico interactivo `T` vs `Y` (fondo blanco, cuadrícula clara, líneas sólidas para rango DB y punteadas para extrapolación, etiquetas de especies e inspector al pasar el cursor).
  - **Pestaña `Table`**: valores numéricos tabulados por temperatura.

## 2. Requisitos de responsividad
- Paneles redimensionables (`resizable(true)`) y `ScrollArea::both()` en hojas y gráficos.
- Tamaño mínimo de ventana `[480.0, 360.0]`.
