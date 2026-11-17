# Especificación de Interfaz (UI) — Módulo 12: Elements Module

## 1. Distribución General (Estilo Clásico HSC `Elements` + Vista Periódica)
- **Barra superior (`TopBottomPanel::top`):**
  - `File ▾` (`Reset Default`, `Open JSON...`, `Save JSON...`, `Export CSV...`).
  - Botón/Toggle `Fix Left 2 Columns` (congela `Property` y `Units` en la hoja Excel).
  - Acciones de vista rápida: `Diagram` (activa/resalta el diagrama periódico de la fila seleccionada), `Trend vs Z` (gráfico por número atómico), `+ Add Custom Row`, `Copy Row`.
  - Selector de modo de escala para el diagrama (`Linear (HSC Default)` / `Sqrt` / `Log`).
  - Buscador rápido de propiedad y salto rápido a símbolo (`Find Element: Fe`).
- **Área Central (`CentralPanel` con pestañas o vista dual):**
  1. **Pestaña `Elements Sheet` (Hoja Excel tipo `Fig. 1`):**
     - Grilla con cabecera `Property | Units | Reference | 1 H | 2 He | ... | 118 Og`.
     - Fila seleccionada resaltada; doble clic o clic en un encabezado de elemento permite inspeccionar su ficha completa.
     - Scroll horizontal y vertical fluido (`ScrollArea::both()`) con columnas fijas cuando `Fix Left 2 Columns` está activo.
  2. **Pestaña `Periodic Diagram` (Tabla Periódica de Esferas tipo `Fig. 2`):**
     - Título automático con la propiedad y unidad seleccionadas (p. ej. `Density (g/ml)`), barra leyenda con `Min`, `Max` y colores positivo/negativo.
     - Cuadrícula de **18 columnas × 9 filas** (Periodos 1–7 arriba, espacio separador, Lantánidos `57–71` y Actínidos `89–103` abajo), coloreada suavemente por bloque/categoría química.
     - Cada casilla muestra símbolo, `Z` y la esfera sombreada centrada cuyo diámetro refleja la magnitud.
     - **Doble clic** en una casilla (o botón de inspección) abre ventana emergente con los datos exactos del elemento y sus especies disponibles en `Maindb6`.
  3. **Pestaña `Trend (Property vs Z)`:**
     - Gráfico interactivo `egui_plot` con $Z$ en el eje X y la propiedad seleccionada en Y, marcando límites de periodos (`He, Ne, Ar, Kr, Xe, Rn, Og`).
- **Barra de estado inferior:**
  - Muestra elemento y propiedad activa, conteo de elementos con dato válido, mínimo, máximo y media de la propiedad seleccionada.

## 2. Requisitos de Responsividad
- Tema claro (`egui::Visuals::light()`), tamaño mínimo `[480.0, 360.0]`, ajuste dinámico de celdas de tabla periódica al ancho disponible (`ScrollArea::both()` si la ventana es muy estrecha).
