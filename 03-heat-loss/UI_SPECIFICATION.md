# Especificación de Interfaz de Usuario (`UI_SPECIFICATION.md`) — Módulo 3

Esta especificación adapta la interfaz observada en `HeatLoss.exe` (`fig01`–`fig14`) al estilo visual nativo de **`soryntel-desktop`** (`egui` + `egui_plot`, tema claro de alto contraste), manteniendo coherencia directa con el Módulo 1 (*Reaction Equations*) y el Módulo 2 (*Heat and Material Balance*).

---

## 1. Integración en el Shell de `soryntel-desktop`

- En [`crates/soryntel-desktop/src/main.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/main.rs), el selector superior de módulos (`ActiveModule`) incorpora una tercera pestaña:
  - `Reaction Equations` (Módulo 1)
  - `Heat & Material Balance` (Módulo 2)
  - **`Heat Loss`** (Módulo 3)
- Se conserva la barra superior unificada (título, selector de módulo, botón de base de datos / estado de conexión, acciones de archivo `.htr`/`.json` y acceso rápido a **Exportar pérdida térmica al Módulo 2**).

---

## 2. Distribución Principal de la Vista `Heat Loss`

Siguiendo la estructura de `fig01`, `fig02` y `fig04`:

### 2.1 Barra de Acciones y Pestañas de Hojas (Libro Multicapa)
- **Barra de herramientas superior del módulo:**
  - Archivo: `New`, `Open (.json / .htr8 import)`, `Save`.
  - Edición de columnas/hojas: `+ Add Layer`, `+ Add Surface`, `+ Add Enclosure`, `+ Add Contact`, `Delete Column`, `Move Left/Right`.
  - Hojas del libro: pestañas horizontales (`Wall`, `Roof`, `Bottom`, `+ Add Sheet`, doble clic para renombrar), reflejando el patrón `Smelting 1200°C | ...` y `Wall | Roof | Bottom` de `fig01`/`fig04`.
  - Botón de integración directa: **`Send Total Heat Flow to Balance (Module 2)`** (selecciona el `BalanceArea` destino y crea/actualiza la fila `HEAT LOSSES:` con signo negativo en `soryntel_thermo::balance_engine`).

### 2.2 Grilla Central de Capas y Superficies (`ScrollArea` horizontal + vertical)
Columnas `1..N` representando el flujo unidimensional de izquierda a derecha (`HOT => COLD` o viceversa):

| Sección | Fila | Unidades / Controles | Comportamiento visual |
|---|---|---|---|
| **Header** | `Fix / SET` | Indicador por celda editable | Celdas fijadas por el usuario se muestran con texto **rojo** y borde sutil, igual que `fig01`/`fig04`. |
| **Layer Info** | `Layer Number` | `1..N` | Solo lectura. |
| | `Layer Type` | `Surface` / `Solid Layer` / `Enclosure` / `Layer contact` | ComboBox por columna. |
| | `Layer Material` | Nombre + botón selector (`Cond...` / `Conv...`) | Abre modal de base de datos correspondiente. |
| | `Surface Material` | Nombre + botón selector (`Rad...` / `Gas Rad...`) | Solo habilitado en `Surface` / `Enclosure`. |
| **Layer Specifications** | `Temperature (left edge)` | `°C` / `K` | Editable (fix) o calculado. |
| | `Temperature (right edge)` | `°C` / `K` | Editable (fix) o calculado. |
| | `Thickness` | `m` / `mm` / `cm` / `in` | Solo `Solid Layer` y `Enclosure`. |
| | `Calculation Grid` | Entero $\ge 1$ (default `10`) | Subdivisión espacial para $k(T)$. |
| | `Average conductivity (k)` | `W/(m·K)` | Editable si fijo; calculado desde `heatcond` si libre. |
| | `Convection coefficient (hc)` | `W/(m²·K)` | Editable si fijo; calculado por `HeatConv` si libre. |
| | `Radiation coefficient (hr)` | `W/(m²·K)` | Editable si fijo; calculado por `HeatRadi`/`GasRad` si libre. |
| | `Thermal Resistance` | `K/kW` (`°C/kW`) | Calculado (editable en `Layer contact`). |
| **Results** | `Heat Flow` | `W` / `kW` / `MW` / `kcal/h` | Negrita; idéntico en todas las columnas en estado estacionario. |
| | `Heat Flux` | `W/m²` / `kW/m²` | Por área local de la capa/superficie. |
| | `Surface Area` | `m²` | Crece radial/geométricamente en `Cylinder`, `Sphere` y `Cube`. |
| | `Cumulative Thickness` | `m` / `mm` | Acumulado desde la cara interior. |
| **Database Data** | `Property Table (collapsible)` | Filas expandibles equivalentes a `96+` | Permite inspeccionar $k(T)$ o propiedades del fluido sin ensuciar la vista principal. |

### 2.3 Panel Inferior de 3 Bloques (`fig04`)

1. **Bloque Izquierdo — `Total Heat Flow` & `Shape and Dimensions`:**
   - Campo destacado `TOTAL HEAT FLOW` (fondo amarillo suave `#FFF8D6` como en `fig01`/`fig04`) + selector de unidad (`W`, `kW`, `MW`, `kcal/h`, `BTU/h`).
   - Radio buttons de geometría: `Wall`, `Cube`, `Cylinder`, `Sphere`.
   - Campos dinámicos de dimensiones internas (`Width`, `Length`, `Height`, `Diameter`) + botón `Preview Geometry` (esquema SVG/egui 2D).
2. **Bloque Central — `Calculate`:**
   - Botón `Temperature profile` (1 $T$ fija + $Q_{\text{total}}$ fijo).
   - Botón `Heat flow` (2 $T$ fijas).
   - Indicador visual bidireccional **`HOT => COLD`** (gradiente rojo $\to$ azul) o **`COLD <= HOT`**.
   - Radio buttons de alcance: `Calculate all sheets` / `Calculate active sheet`.
3. **Bloque Derecho — `Options` & Herramientas:**
   - `Toggle Fix Value` (sobre celda activa).
   - `Target Solver...` (abre modal `Target Dialog`).
   - `Diagram & Sweep...` (abre ventana/panel `HSC Diagrams` con `egui_plot`).
   - `Gas & Particle Radiation...` (abre calculadora EWBM + nube de partículas).

---

## 3. Ventanas Modales y Subcalculadoras (`fig05`–`fig14`)

- **Conduction Picker (`fig05`):** Árbol de categorías + búsqueda incremental + tabla $k(T)$ + gráfico previo `egui_plot`.
- **Convection Calculator (`fig06`–`fig07`):** Pestañas `Fluid & Correlations` (`Free` / `Forced`, orientación, velocidad $v$, tabla de números adimensionales `L, H/L, Tf, Nu, Ra, Gr, Re, Re*Pr`) y `Custom Function hc(v)`.
- **Surface Radiation Picker (`fig08`):** Filtro por tipo `(n)` / `(h)` y valor de $\varepsilon_s$.
- **Gas Mixture & Particle Radiation Calculator (`fig09`–`fig10`):** Composición molar de 6 gases, $p$, $L$, $T_g$, $T_s$, $\varepsilon_s$, tabla de bandas espectrales EWBM, parámetros de nube de partículas ($d_s, n_p, \varepsilon_p$) y botones `Apply Fixed T` / `Apply Iterative Table (0–3000 °C)`.
- **Target Dialog (`fig11`):** Selector de variable independiente, objetivo de flujo/temperatura, límites `[min, max]`, tolerancia y vista previa de convergencia.
- **Diagram Window (`fig12`–`fig14`):** Pestañas `Chart` (`egui_plot` con leyenda interactiva y exportación PNG/CSV) y `Data` (tabla numérica copiable).
