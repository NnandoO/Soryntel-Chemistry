# Especificación de Interfaz (UI) — Species Converter

## 1. Cinta Superior (`Home` / `Tools`)
- **Pestaña `Home`**:
  - Grupo `File`: `New`, `Open`, `Save`.
  - Grupo `Workbook`: `Add Calculation Sheet`, `Add Summary Sheet`, `Delete Sheet`.
  - Grupo `Actions`: `Fix Element Balance`, `Initialize`, `Solve`, `Element Balance`, `Charts`.
- **Pestaña `Tools`**:
  - `Norm settings`: `Norm limit` (`0.0001`), checkbox `Set norm below the limit`.
  - `Measurements`: checkboxes `Exact O measurement`, `Exact H measurement`.
  - `General settings`: `Small Meas Limit` (`0.5`), selector `Method` (`Method 1`, `Method 2`, `Method 3`).
  - `Stability Weighting`: selector `Species type` (`None`, `Sulfides`, `Oxides`, `Sulfates`, `Carbonates`, `Chlorides`, `Silicates`, `Hydroxides`), campo `Weighting coef.` (`80`).

## 2. Hoja Principal (`Sheet1`, etc.)
- Cabecera (filas 1–3): `Name`, `Date`, `Author` (izquierda) y `Residual Error` (derecha).
- Bloque izquierdo `INPUT`: columnas `Analysis Elements`, `wt-%`, `Calculated Elements`, `wt-%` (resaltado en rojo cuando no cumple `Limit`).
- Bloque derecho `OUTPUT`: columnas `Species`, `Calculated wt-%`, `Target wt-%`, `Weight Coeff`, `Type`.
- Fila `Total` (fila 8) con sumatorias automáticas.

## 3. Panel Derecho (`Charts`) y Ventana `Element Balance`
- Panel derecho redimensionable (`egui::SidePanel::right`) con gráfico circular `Input analysis` arriba y `Output analysis` abajo.
- Ventana `Element Balance` redimensionable con grilla por elemento (`Input kg`, `Output kg`, `Balance kg`, `Error %`, `Limit` en rojo si `< umbral`).
- Responsividad: `ScrollArea::both()` en la hoja principal y ventana mínima `[480, 360]`.
