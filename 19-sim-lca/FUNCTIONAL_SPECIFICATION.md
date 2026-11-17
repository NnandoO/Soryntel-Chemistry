# Especificación Funcional — Módulo 19: Sim LCA (`HSC - Sim LCA`)

Los casos de uso `CU-01` a `CU-06` especifican el flujo clásico (`49.3`, `Tools -> LCA Evaluation`), mientras `CU-07` a `CU-11` especifican el flujo avanzado acoplado a `openLCA` por gRPC IPC (`49.4`, `LCA (beta)`). En ambos modos es requisito previo que el diagrama de flujo de **HSC Sim** haya convergido con balances de materia y energía válidos.

---

## CU-01 — Inicialización Clásica y Selector de Base de Datos Activa (`49.3.1`)
- **Precondición:** Proyecto `.Sim` calculado con corrientes de frontera externas válidas (entradas **azules**, salidas **rojas**).
- **Flujo principal:**
  1. Desde HSC Sim, el usuario abre `Tools -> LCA Evaluation`.
  2. El módulo extrae automáticamente todas las corrientes de frontera del diagrama excluyendo las corrientes internas **negras**.
  3. En el grupo `Database` de la cinta superior, el usuario elige:
     - **`GaBi`**: utiliza el catálogo jerárquico de flujos embebido para formato EcoSpold v1.0.
     - **`OpenLCA`** -> **`Set Active Database...`**: abre selector de archivos `.zip` JSON-LD exportados previamente desde OpenLCA / openLCA Nexus para poblar el catálogo local de flujos y habilitar el modo OpenLCA.
- **Resultado:** Pestañas `Input`, `Output`, `Manual Input`, `Manual Output` e `Indicator` listas para edición y mapeo.

## CU-02 — Revisión de Corrientes, Conversión Horaria, `Use Exergy` y Flujos Manuales (`49.3.2–49.3.3`)
- **Flujo principal:**
  1. En **`Input`** y **`Output`**, cada corriente automática muestra `Stream Name`, `Unit Name` (unidad frontera conectada), `Amount` convertido a base `1 hora` (`kg` para materia, `kWh` para energía/calor), `Unit` (`kg` o `kWh`), checkbox `Use Exergy`, `LCA Equivalent`, `LCA Group` y checkbox `Main Product`.
  2. Expandiendo la fila (`+`), se despliega un desglose de solo lectura con `Mass Flow (t/h)`, `Temperature (C)`, `Pressure (bar)`, `Enthalpy Flow (kW)`, `Exergy Flow (kW)`, `Total Volume (Nm3)`, `Total Solids Flow (t/h)` y composición por especies (`t/h`).
  3. Al activar **`Use Exergy`** sobre una fila, `Amount` cambia al valor de `Exergy Flow` en `kWh` y `Unit` pasa a `kWh`.
  4. En **`Manual Input`** y **`Manual Output`**, el usuario pulsa `Add new input stream` / `Add new output stream`, ingresa nombre libre (p.ej. `Power for buildings`), `Amount`, `Unit` (`kg` o `kWh`) y asigna mapeo; o elimina filas mediante el icono `Delete` (`×`).

## CU-03 — Hoja de Indicadores, Comodines y Prevención de Doble Conteo (`49.3.4`)
- **Flujo principal:**
  1. En **`Indicator`**, el sistema pre-carga filas por defecto (`CO(g)`, `CO2(g)`, `H2O(g)`, `SO*`, `NO*`) y permite agregar nuevos patrones o eliminarlos con `Delete`.
  2. El motor de coincidencia admite comodines (`*` cero o más caracteres, `?` exactamente un carácter, `#` dígito `0..9`) sobre especies presentes en corrientes de salida filtradas.
  3. **Regla OpenLCA:** En modo OpenLCA, las filas de salida marcadas con `LCA Group = To Nature` habilitan automáticamente sus corrientes fuente en la pestaña `Indicator` para desglose químico de emisiones.
  4. Al desplegar (`+`) un indicador, cada corriente fuente encontrada muestra su aporte exacto y la casilla **`Counted`**: desmarcar `Counted` excluye esa fuente para prevenir **doble conteo** (p.ej. no contar `CO2(g)` dos veces si ya está incluido en una corriente o indicador más general).

## CU-04 — Mapeo de Flujos Clásicos y Asignación de `LCA Group` (`49.3.5`)
- **Flujo principal:**
  1. Al activar **`Mapping`** en la cinta `Tools`, se abre a la derecha el navegador jerárquico `Flows` (`Deposited goods`, `Emissions to...`, `Resources`, `Production residues in life cycle`, `US LCI Database`, `Valuable substances`, o jerarquía JSON-LD en modo OpenLCA).
  2. El usuario puede filtrar por `Unit type`, restringir mediante `Search from selected category` y buscar mediante `Keyword Search`.
  3. Seleccionar un nodo muestra sus metadatos en el panel inferior (`Name`, `Category`, `Unit`, `Reference Quantity`, `Energy Quantity`, `Conversion Factor`).
  4. Al arrastrar un flujo hacia la celda `LCA Equivalent` de una fila en `Input`, `Output`, `Manual Input`, `Manual Output` o `Indicator`, se asigna su equivalencia.
  5. En **`LCA Group`**, el usuario clasifica cada flujo según dirección:
     - Entradas: `Not defined`, `Materials/Fuels`, `Electricity/Heat`, `Services`, `From Nature`, `From Technosphere`.
     - Salidas: `Not defined`, `Reference Product (To technosphere)`, `To Nature`, `Allocated by product`.

## CU-05 — Normalización Black-Box (`49.3.6`)
- **Precondición:** Exactamente una corriente/fila marcada como **`Main Product`**.
- **Flujo principal:**
  1. El usuario marca `Main Product` en la corriente de producto principal (p.ej. `Final ZnO Dust = 8012.99 kg`, `Zinc oxide`) y pulsa **`Normalize`** en la cinta `Tools`.
  2. El sistema valida que exista un único `Main Product` no nulo.
  3. Descarta cualquier corriente cuyo `LCA Equivalent` sea `No Mapping` (excepto manejo de subespecies ya cubiertas en `Indicator` sin doble conteo).
  4. Divide todas las cantidades entre la cantidad del `Main Product` (`1 kg` base normalizada) y consolida en una sola entrada las filas que compartan `(Direction, Flow Name, Group Name)` (p.ej. `Air` = `34590.74 + 1.29 kg` -> `4.32 kg/kg`; `Waste heat` = `6302.05 + 3327.22 + 441.09 kWh` -> `1.26 kWh/kg`).
- **Resultado:** Se presenta la tabla `Normalized Flows` (`Direction`, `Flow Name`, `Group Name`, `Category`, `Sub Category`, `Amount`, `Normalized Amount`, `Unit`).

## CU-06 — Metadatos del Proceso y Exportación EcoSpold / JSON-LD / Excel (`49.3.7–49.3.9`)
- **Flujo principal:**
  1. Pulsando **`Process Info`**, el usuario rellena `Process Name`, `Technology`, `General Comment`, `HSC Ver. (Last save)` (solo lectura), `Location`, `Location Comment`, `Author`, `Date` y `Version Number`.
  2. Pulsando **`To EcoSpold v1.0 (GaBi)`** (solo activo con DB GaBi) o **`To JSON-LD (OpenLCA)`** -> **`To Active Database`** / **`To Empty File...`** (solo activo con DB OpenLCA), si aún no se ejecutó `Normalize`, la aplicación solicita ejecutar la normalización automáticamente primero.
  3. Genera los archivos `.xml` (EcoSpold v1.0) o `.zip` (JSON-LD) y muestra el diálogo de confirmación (`EcoSpold v1.0 export succeeded!` / `JSON-LD export succeeded!`).
  4. **`To Excel`** exporta las tablas activas del libro de trabajo LCI a formato Excel.

---

## CU-07 — Conexión Beta con `openLCA` por gRPC IPC y Carga de Árboles (`49.4`)
- **Precondición:** `openLCA` (p.ej. `2.0.0.alpha3`) abierto con una base activa restaurada y `Tools -> Developer tools -> IPC Server` iniciado con **`Start as gRPC service (experimental)`** en puerto `8080`.
- **Flujo principal:**
  1. En la herramienta beta `LCA`, el usuario despliega **`Modes -> openLCA`** y elige **`Unit database`** o **`LCI database`** (validación oficial recomendada en bases *system* `LCI`).
  2. La herramienta consulta por gRPC al servidor local y descarga en el panel lateral derecho `Flows and Methods` el árbol completo de `Flows` (etiquetados `P -` para producto y `E -` para elemental) y `Impact Methods and Categories`.
  3. Pobla automáticamente las pestañas **`Inputs`** y **`Outputs`** con las corrientes externas del diagrama y sus fases/especies expandibles (`Pyro Phase`, `Energy`).

## CU-08 — Corrientes Manuales, Visibilidad y Conversión Bidireccional a Indicadores (`49.4`)
- **Flujo principal:**
  1. **`Hide Stream Content` / `Show Stream Content`**: colapsa o expande simultáneamente los desgloses de fase/especie en `Inputs` y `Outputs`, dejando solo filas de cabecera por corriente.
  2. **`Add Manual Stream`** / **`Delete Manual Stream`**: inserta o borra filas al pie de `Inputs` o `Outputs` (`<Manual Input Stream>` / `<Manual Output Stream>`), aceptando nombre, valor numérico o referencia de celda del simulador y unidad (`t/h`, `kW` o `MJ`).
  3. **`Convert Stream to Indicators`**: estando seleccionada una fila de corriente no manual sin mapeo asignado (`Flow Mapping` vacío), habilita el mapeo individual de cada especie hija (p.ej. `CO(g)`, `CO2(g)`, `H2(g)`, `H2O(g)`, `SO2(g)`, `N2(g)`, `O2(g)` de `Furnace Gas`) e impide mapear simultáneamente la corriente agregada padre.
  4. **`Convert Indicators back to Stream`**: estando seleccionada una fila indicadora hija, colapsa nuevamente el desglose a nivel de corriente única (si no existen mapeos incompatibles o limpiando asignaciones según validación).

## CU-09 — `LCA Flowtype`, Mapeo y Resolución de Proveedor y Ubicación (`49.4`)
- **Flujo principal:**
  1. Para cada corriente o indicador activo, el usuario asigna `LCA Flowtype`:
     - En `Inputs`: `Elementary Flow`, `Product Flow` o `Reference Flow(s) (Product)`.
     - En `Outputs`: `Elementary Flow`, `Waste Flow` o `Reference Flow(s) (Product)`.
  2. En el panel `Flows`, busca por texto (`Find` / `Clear`), selecciona un nodo coincidente (comprobando compatibilidad entre `E -`/`P -` y las unidades `t/h`, `kW`, `MJ`) y aplica doble clic o pulsa **`Add Mapping`** (o **`Delete Mapping`** para limpiar).
  3. Debe existir exactamente un flujo configurado como **`Reference Flow (Product)`** (p.ej. `Pig Iron`, sin mapeo a flujo externo de BD para servir de divisor normalizador).
  4. Para flujos `Product Flow` o `Waste Flow`, el sistema consulta los proveedores disponibles:
     - Si existen `≤ 15` proveedores, permite elegir directamente en el desplegable **`Provider`** (`No provider` o proceso específico).
     - Si existen `> 15` proveedores, deshabilita o vacía el selector directo de proveedor hasta que el usuario seleccione primero **`Location`** (p.ej. `GLO`, `RoW`, `AE`), filtrando luego **`Provider`** a los procesos de esa ubicación.

## CU-10 — Selección de Método LCIA, Cálculo via IPC y Actualización Reactiva (`49.4`)
- **Flujo principal:**
  1. En la pestaña lateral `Impact Methods and Categories`, el usuario expande un método único (p.ej. `ei - ReCiPe Midpoint (H)`), selecciona una o varias categorías (p.ej. `climate change - GWP100`, `metal depletion - MDP`) y pulsa **`Add Impact Category`** (o **`Delete Impact Category`**).
  2. Todas las categorías en la hoja `Results` deben pertenecer al **mismo** método de impacto; si difieren, la inserción se rechaza.
  3. Al pulsar **`Calculate Results`**:
     - Via gRPC IPC, HSC Sim crea dentro de la BD `openLCA` un proceso unitario por cada unidad del diagrama con flujos mapeados (más `Manual Streams` si aplica), cada uno emitiendo un flujo de referencia sintético de `1.00000 kg`.
     - Crea el proceso principal `LCA Process <timestamp>` que consume `1.00000 kg` de cada proceso unitario y entrega la tasa horaria del `Reference Flow` (`Pig Iron = 1.38217E5 kg`).
     - Crea el `Product System`, ejecuta la evaluación LCIA para las categorías solicitadas y puebla la hoja **`Results`** con desglose completo por categoría, unidad, entrada/salida, flujo, monto normalizado e impacto normalizado.
  4. **`Hide Result Content` / `Show Result Content`**: alterna entre vista resumida (solo filas totales por categoría) y desglose completo por unidades y corrientes.
  5. **Actualización en vivo:** Al modificar variables del diagrama en HSC Sim (sin alterar mapeos ni categorías), la hoja `Results` recalcula y actualiza los valores proporcionales automáticamente, incluso si se cerró el servidor IPC después del cálculo estructural.

## CU-11 — Hojas Personalizadas, Instantáneas (`Saved*`), Importación (`Ext*`) y Exportación Excel (`49.4`)
- **Flujo principal:**
  1. **`Add Custom Sheet` / `Delete Custom Sheet` / `Rename Custom Sheet`**: permite crear hojas auxiliares definidas por el usuario prohibiendo nombres duplicados y prefijos reservados `'Saved'`, `'Norm'` o `'Ext'`.
  2. **`Save Results to Custom sheet`**: congela una copia estática de los resultados actuales en una nueva pestaña cuyo nombre inicia por `'Saved'` para comparaciones históricas.
  3. **`Import External Sheet`**: importa tablas desde carpeta/archivos `LCA_exports*` creando pestañas prefijadas con `'Ext'`.
  4. **`Export to Excel`**: exporta el libro completo de evaluación (`Inputs`, `Outputs`, `Results`, personalizadas) a `LCA_exports.xlsx`.
