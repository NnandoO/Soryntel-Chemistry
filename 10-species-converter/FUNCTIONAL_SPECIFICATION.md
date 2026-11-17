# Especificación Funcional — Species Converter

## 1. Acciones Principales
- **Initialize**: Limpia resultados previos de `OUTPUT` y calcula la composición elemental de `INPUT` en `Calculated Elements` (`O`, `H`, `[Others]`).
- **Solve**: Resuelve `OUTPUT wt-%` según el método seleccionado (`Method 1`, `Method 2`, `Method 3`), actualiza `Calculated Elements`, `Total`, `Residual Error` y gráficos.
- **Fix Element Balance**: Agrega elementos puros del análisis de entrada a la tabla `OUTPUT` con `Weight Coeff = 0.100` (si no están ya presentes) y vuelve a resolver.
- **Element Balance**: Abre la ventana con la tabla comparativa por elemento (`Input kg`, `Output kg`, `Balance kg`, `Error %`, `Limit`) y umbral editable de `Limit`.
- **Charts**: Muestra/oculta el panel lateral derecho con los gráficos circulares `Input analysis` y `Output analysis` (agrupando especies `< Small Meas Limit` en `[Others]`).

## 2. Libro de Trabajo (`Workbook`)
- **Add Calculation Sheet**: Agrega una nueva hoja de cálculo (`Sheet2`, `Sheet3`, ...).
- **Add Summary Sheet**: Agrega una hoja resumen consolidando los resultados de todas las hojas de cálculo.
- **Delete Sheet**: Elimina la hoja activa (si hay más de una).
- **File (New / Open / Save)**: Crea libro nuevo, abre o guarda el libro en formato JSON.
