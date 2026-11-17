# Especificación de Interfaz de Usuario (UI) — Módulo 19: Sim LCA

La interfaz de usuario reproduce fielmente las dos ventanas documentadas en el capítulo 49: la herramienta clásica **`LCA Evaluation`** (`49.3`) y la herramienta acoplada a openLCA **`LCA`** (`49.4`), diseñadas bajo estilo de cinta de opciones (*Ribbon*) con grillas jerárquicas tipo Excel y panel lateral acoplable.

---

## 1. Ventana Clásica: `LCA Evaluation` (`49.3`, `Figs. 9–21`)

### 1.1. Cinta Superior (*Ribbon*)

Dividida en `5` grupos etiquetados en la parte inferior:

| Grupo | Botones / Desplegables | Comportamiento |
|---|---|---|
| **`Database`** | `GaBi` (botón grande), `OpenLCA ▾` (`Set Active Database...`) | Conmuta el modo activo y habilita el motor de exportación respectivo |
| **`Streams`** | `Input`, `Output`, `Manual Input`, `Manual Output`, `Indicator` | Selector exclusivo de la pestaña/hoja visible en la grilla central |
| **`Tools`** | `Mapping`, `Normalize`, `Process Info` | `Mapping` muestra/oculta el panel lateral derecho; `Normalize` genera vista `Normalized Flows`; `Process Info` abre diálogo modal |
| **`Export`** | `To EcoSpold v1.0 (GaBi)`, `To JSON-LD (OpenLCA) ▾` (`To Active Database`, `To Empty File...`), `To Excel` | Habilitado contextualmente según BD activa; dispara validación de normalización previa |
| **`Info`** | `Help` | Abre sección 49 del manual de ayuda |

### 1.2. Grillas de Corrientes (`Input Streams`, `Output Streams`, `Manual Input Streams`, `Manual Output Streams`)
- **Cabeceras (`Fig. 9–11`):** `[+]`, `Stream Name`, `Unit Name` *(oculto en manuales)*, `Amount`, `Unit` (`kg` / `kWh`), `Use Exergy` *(checkbox)*, `LCA Equivalent`, `LCA Group` *(desplegable)*, `Main Product` *(checkbox único en todo el libro)*, `Delete` *(botón `×` rojo en manuales)*.
- **Fila expandida (`+`):** Sub-tabla gris/clara con propiedades de corriente (`Mass Flow t/h`, `Temperature C`, `Pressure bar`, `Enthalpy Flow kW`, `Exergy Flow kW`, `Total Volume Nm3`, `Total Solids Flow t/h`) y desglose por especies químicas.
- **Pie de hojas manuales:** Botón ancho inferior `Add new input stream` / `Add new output stream`.

### 1.3. Grilla de Indicadores (`Indicator Streams`, `Fig. 12`)
- **Cabeceras:** `[+]`, `Stream Name` *(patrón/especie editable)*, `Measured Property` / `LCA Equivalent`, `Amount`, `Unit`, `LCA Group`, `LCA Equivalent`, `Delete`. *(Nota: en las capturas oficiales de `Fig. 12`, el rol ambiental `"To Nature"` y el nombre `"Carbon dioxide"` figuran bajo las columnas de equivalencia/grupo).*
- **Sub-grilla expandida:** Lista corrientes fuente coincidentes con columnas `Stream`, `Amount`, `Counted` *(checkbox anti doble conteo)*.

### 1.4. Panel Lateral de Mapeo (`Fig. 13`, ~35% ancho derecho)
- **Controles de filtro superiores:** Selector `Unit type`, checkbox `Search from selected category`, campo `Keyword Search` con icono de lupa/filtro.
- **Árbol `Flows`:** Jerarquía colapsable de categorías (GaBi u OpenLCA) con arrastre (*drag-and-drop*) directo sobre `LCA Equivalent`.
- **Bloque inferior de metadatos (`Flow Properties`):** Campos de solo lectura `Name`, `Category`, `Unit`, `Reference Quantity`, `Energy Quantity`, `Conversion Factor`.

### 1.5. Vista `Normalized Flows` y Diálogo `Process Information` (`Figs. 17–18`)
- **`Normalized Flows` (`Fig. 17`):** `Direction`, `Flow Name`, `Group Name`, `Category`, `Sub Category`, `Amount`, `Normalized Amount`, `Unit`.
- **Modal `Process Information` (`Fig. 18`):** Agrupado en `Process` (`Process Name`, `Technology`, `General Comment`, `HSC Ver. (Last save)` deshabilitado), `Location` (`Location` desplegable, `Location Comment`), `Developer` (`Author`, `Date` selector fecha, `Version Number`) y botones inferiores `OK` / `Cancel`.

---

## 2. Ventana Beta IPC: `LCA` (`49.4`, `Figs. 7–36`)

### 2.1. Cinta Superior (*Ribbon*) y Barra de Fórmulas

Dividida en `8` grupos etiquetados en la parte inferior:

| Grupo | Controles Exactos | Estado / Habilitación |
|---|---|---|
| **`Modes`** | `openLCA ▾` (`Unit database`, `LCI database`) | Inicia lectura IPC del catálogo activo desde `localhost:8080` |
| **`Streams`** | `Add Manual Stream`, `Delete Manual Stream`, `Hide Stream Content` / `Show Stream Content`, `Convert Stream to Indicators`, `Convert Indicators back to Stream` | Sensibles al tipo de fila seleccionada (padre de corriente vs. indicador vs. manual) |
| **`Custom Sheets`** | `Add Custom Sheet`, `Delete Custom Sheet`, `Rename Custom Sheet` | Eliminación/renombre habilitado solo sobre hojas no estándar; bloquea prefijos `Saved`, `Norm`, `Ext` |
| **`External Sheet`** | `Import External Sheet`, `Export to Excel` | Importa desde `LCA_exports*`; exporta hacia `LCA_exports.xlsx` |
| **`Mapping`** | `Add Mapping`, `Delete Mapping` | Aplica o remueve nodo seleccionado de `Flows` sobre la fila activa |
| **`Calculation Methods`** | `Add Impact Category`, `Delete Impact Category` | Agrega o quita categoría del método activo hacia la hoja `Results` |
| **`Results`** | `Hide Result Content` / `Show Result Content`, `Calculate Results`, `Save Results to Custom sheet` | Lanza evaluación IPC/local y guarda copias congeladas `Saved*` |
| **`Info`** | `Help` | Ayuda del submódulo beta |

Debajo del *Ribbon* aparece una **barra de referencia/fórmula tipo hoja de cálculo** (`[NombreCelda ▼] [contenido/fórmula]`) compatible con referencias de celda para flujos manuales, y pestañas de navegación en el pie izquierdo (`◄ ► │ Inputs / Outputs / Results / ...`).

### 2.2. Grillas `Inputs` y `Outputs` (`Figs. 10, 11, 22, 26, 27`)
- **Cabecera azul sólida (`Filas 1–3`):** Columnas `B` `Stream`, `C` *(subestructura/especie)*, `D` `Amount`, `E` `Units`, `F` `LCA Flowtype`, `G` `Flow Mapping`, `H` `Provider`, `I` `Location`.
- **Jerarquía de colores:**
  - **Fila de corriente** (p.ej. `Air Blast`, `Hematite Pellets`, `Furnace Gas`, `<Manual Input Stream>`): sombreado azul medio en negrita; contiene selector `LCA Flowtype` y celda blanca `Flow Mapping`.
  - **Filas de fase e ingredientes** (`Pyro Phase`, `Energy` en cursiva + especies `Fe2O3`, `CO(g)`, etc.): fondo azul claro; al usar `Convert Stream to Indicators`, cada fila de especie hereda sus propios selectores `LCA Flowtype`, `Flow Mapping`, `Provider` y `Location`.
- **Comportamiento condicional de `Provider` / `Location`:** Deshabilitados para `Elementary Flow` y `Reference Flow (Product)`; para `Product Flow` / `Waste Flow`, si `N_providers > 15` exige seleccionar primero `Location` (`No location`, `AE`, `GLO`, `RoW`, etc.) antes de filtrar `Provider`.

### 2.3. Panel Lateral Acoplable `Flows and Methods` (`Figs. 10, 23, 30`)
- Dos pestañas: **`Flows`** y **`Impact Methods and Categories`**.
- Barra superior de búsqueda en ambas pestañas: campo desplegable `Enter text to search... ▾`, botón `Find` (resalta coincidencias en amarillo en el árbol, como se aprecia en `Fig. 25`), botón `Clear`.
- El árbol `Flows` muestra como raíz `Flows (<nombre_bd>)`, divisiones ISIC (`A..S`) y `Elementary flows`; cada hoja elegible lleva prefijo `P - <nombre>` (`Product`) o `E - <nombre>` (`Elementary`).
- El árbol `Impact Methods and Categories` lista métodos (p.ej. `ei - ReCiPe Midpoint (H)`, `ei - TRACI`, `ei - USEtox w/o LT`) y sus categorías hijas seleccionables.

### 2.4. Grilla `Results` (`Figs. 32–35`)
- **Cabeceras azules:** `Method`, `Category`, `Unit`, `Stream Name`, `Flow Mapping`, `Provider`, `Location`, `Stream Amount`, `Normalized Amounts`, `Units`, `Normalized Results`, `Units`.
- **Estructura colapsada (`Hide Result Content`, `Fig. 35`):** Una única fila resumen por categoría mostrando `Method`, `Category`, `Total` y `Normalized Results` + unidad compuesta (`kg CO2-Eq per t/h of Pig Iron`, `kg Fe-Eq per t/h of Pig Iron`).
- **Estructura expandida (`Show Result Content`, `Figs. 33–34`):** Sub-filas identadas por `Unit` (`Blast Furnace`, `Manual Streams`) -> `Inputs` / `Outputs` -> corrientes e indicadores, con totales parciales en negrita por cada bloque y el `Reference Flow` en cursiva (`Pig Iron`, `1,00 t/h`).

---

## 3. Directriz de Implementación Clean-Room en `soryntel-desktop`

Para reemplazar el modal básico actual `Save as LCA JSON...` manteniendo coherencia visual con `soryntel-desktop` (`egui`/`eframe`):

1. Añadir un botón de lanzamiento dedicado en la barra/menú **`Tools -> LCA Evaluation...`** dentro de [`sim_ui/mod.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/sim_ui/mod.rs) abriendo una ventana independiente o vista modal de ancho completo (`LCA Evaluation`), manteniendo `File -> Save as LCA JSON...` como exportación rápida compatible hacia atrás.
2. Estructurar la vista en dos bandas de modo intercambiables (`Classic LCI (EcoSpold / JSON-LD)` y `LCIA Evaluation (openLCA IPC / Built-in)`), conservando fielmente:
   - Cinta superior con grupos (`Database / Modes`, `Streams`, `Tools / Mapping`, `Calculation Methods`, `Export / Results`).
   - Área principal dividida horizontalmente (~65% grilla de pestañas `Inputs` / `Outputs` / `Manual` / `Indicators` / `Results` / `Normalized` y ~35% panel lateral buscable `Flows & Impact Methods`).
   - Modal `Process Information` y diálogo `Rename Sheet` con validación instantánea de nombres prohibidos (`Saved*`, `Norm*`, `Ext*`).
3. Desacoplar el transporte IPC gRPC mediante un puerto trait en `soryntel-application` que permita tanto conectarse a un servidor `openLCA` real como ejecutar evaluación LCIA nativa clean-room contra un catálogo interno parametrizable cuando `openLCA` no esté presente.
