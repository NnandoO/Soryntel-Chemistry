# Especificación de Interfaz de Usuario (`UI_SPECIFICATION`) — Módulo 2: Heat and Material Balance

## 1. Principios de Diseño y Consistencia con el Módulo 1

La interfaz de usuario de **Soryntel Chemistry** para el **Módulo 2 (`Heat and Material Balance`)** en `soryntel-desktop` (Tauri v2 / escritorio nativo) debe cumplir dos criterios simultáneos:
1. **Fidelidad funcional a las capturas de HSC Balance** (Figs. 1–14: `raw-001.png` a `raw-038.png`), preservando la organización por hojas (`BALANCE`, `INx`, `OUTx`, `Inx-%`, `Outx-%`, hojas personalizadas), el panel lateral `Tools`, los paneles acoplados inferiores `Element Balance` y `Total Balance`, la barra de fórmulas y los diálogos modales (`Temperature Balance`, `Target Dialog`, `Diagram Settings`, `Diagram`).
2. **Coherencia visual y arquitectónica con el Módulo 1 (`Reaction Equations`)** en `soryntel-desktop`:
   - Selector de módulo en la barra superior de la aplicación (`Reaction Equations` | `Heat & Material Balance`).
   - Paleta tipográfica y cromática limpia (modo claro científico de alta legibilidad con tipografía monoespaciada/tabular para valores numéricos, encabezados neutros, barra de estado inferior con conexión activa a `soryntel-db`).
   - Búsqueda rápida integrada en `Maindb6.HSC` (SQLite/memoria) sin bloquear el flujo de edición.

---

## 2. Arquitectura de la Ventana Principal (Basada en Figs. 1, 3, 4, 5, 6, 11–14)

```
+---------------------------------------------------------------------------------------------------+
| Barra Superior App: Soryntel Chemistry | [Módulo 1: Reaction Equations] [Módulo 2: Heat Balance]   |
+---------------------------------------------------------------------------------------------------+
| Menú Módulo: File | Edit | View | Insert | Delete | Format | Units | Calculate | Target | Diagram  |
+---------------------------------------------------------------------------------------------------+
| Barra de Fórmulas: [ D11 v ] [ fx ] [ =0.938*(InCu-D13)/2                                       ] |
+-------------------------------------------------------------------------------+-------------------+
|                                                                               | Tools             |
|   Grilla Principal (Hoja Activa: IN1 / OUT1 / BALANCE / In1-% / FEED / ...)   |                   |
|   +----+------------+--------+--------+------+-----+-----+----------+-------+ | Balance Area      |
|   |    | A Formula  | B T(°C)| C P(b) | D kmol| E kg| F Nm3| G Heat C| H TotH| | [Area 1        v] |
|   |  2 | GAS:       |  25.00 |        | 4.386| ... | ... |     0.00|   0.00| |                   |
|   |  3 | N2(g)      |  25.00 |        | 3.465| ... | ... |     0.00|   0.00| | Get Species DB    |
|   | .. | EXTRA HEAT:|        |        |      |     |     |         |   0.00| | [Input]  [Output] |
|   +----+------------+--------+--------+------+-----+-----+----------+-------+ |                   |
|   Pestañas: [<][>] [BALANCE] [IN1] [OUT1] [In1-%] [Out1-%] [FEED] [Target]    | Insert            |
+---------------------------------------+---------------------------------------+ [Balance Area]    |
| Element Balance                       | Total Balance                         | [Stream]          |
| Element | IN(kmol) | OUT(kmol)| Bal   | AREA       | kmol | kg | Nm3 | HCont | | [Row]             |
| Cu      |    1.000 |    1.000 | 0.000 | Balance(1) | -0.81| 0.0| -6.8| 194.7 | | [Heat Flow]       |
| Fe      |    0.540 |    0.540 | 0.000 |  (+ Total H, Exergy)                  |                   |
+---------------------------------------+---------------------------------------+ Delete            |
| Barra de Estado: DB: 36,537 especies | Área: 1 | Convergencia: OK (1 iter)    | [Balance Area]    |
+-------------------------------------------------------------------------------+ [Selected Stream] |
                                                                                | [Selected Row]    |
                                                                                |                   |
                                                                                | Measure Units     |
                                                                                | Temp: (o) °C ( ) K|
                                                                                | Energy: ( ) Mcal  |
                                                                                |         (o) MJ    |
                                                                                |         ( ) kWh   |
                                                                                |                   |
                                                                                | Functions         |
                                                                                | [Temp. Balance]   |
                                                                                | [Summary]         |
                                                                                +-------------------+
```

---

## 3. Especificación Detallada de Componentes de UI

### 3.1 Barra de Menús del Módulo
Observada en `raw-001.png` y `raw-027.png`:
- `File`: Nuevo balance, Abrir (`.sbal` JSON de Soryntel / importar `.BAL` / `.bal8`), Guardar, Exportar reporte `.OHE` / `.csv`.
- `Edit`: Copiar, Pegar, Copiar Corriente (`Copy Stream`), Pegar Corriente (`Paste Stream`).
- `View`: Mostrar/ocultar paneles `Element Balance`, `Total Balance`, `Tools`.
- `Insert`: `Balance Area` (izquierda/derecha), `Stream`, `Row`, `Heat Flow`, `Additional Sheet`, `Pressure correction H2O`.
- `Delete`: `Balance Area`, `Selected Stream`, `Selected Row`.
- `Format`: Formato numérico (decimales), colores de celda, `Red Font Shield` (bloquear celdas no rojas opcionalmente).
- `Units`: `°C` / `K`, `Mcal` / `MJ` / `kWh`.
- `Calculate`: Recalcular todo, Iterar referencias circulares, `Temperature Balance`.
- `Target`: Abrir ventana `Target Dialog`.
- `Diagram`: Abrir ventana `Diagram Settings`.

### 3.2 Grilla de Hoja de Cálculo (`INx` / `OUTx`)
Columnas exactas observadas en `raw-001.png`, `raw-009.png` y `raw-029.png`:
- `A`: `INPUT SPECIES (n) Formula` / `OUTPUT SPECIES (n) Formula`
- `B`: `Temper. °C` (o `K`)
- `C`: `Pressure bar` (fondo gris/crema suave; editable cuando aplica corrección de presión `H2O`)
- `D`: `Amount kmol`
- `E`: `Amount kg`
- `F`: `Amount Nm³`
- `G`: `Heat Content <EnergyUnit>`
- `H`: `Total H <EnergyUnit>`
- `I`: `Heat Cont <EnergyUnit> / kmol`
- `J`: `Tot H <EnergyUnit> / kmol`
- `K`: `MW` (masa molar en `g/mol` o `kg/kmol`)

**Codificación visual de filas y celdas (Figs. 1, 3, 6, 11–14):**
- **Fila de encabezado de corriente (`Stream`):** texto en **negrita** (azul/negro), nombre terminado en `:` (ej. `GAS:`, `MATTE:`, `SLAG:`), muestra temperatura común (si uniforme) y sumatorias de la corriente.
- **Fila térmica especial (`EXTRA HEAT:` en `INx`, `HEAT LOSSES:` en `OUTx`):** texto en **verde** (`#008000`), valor de flujo térmico en columna `H` (`Total H`).
- **Valores de entrada en hojas auxiliares (`FEED`, `Analysis`, `Process flow`):** texto en **rojo** (`#C00000`) para distinguir inputs editables de celdas calculadas por fórmula (negro).

### 3.3 Hoja `BALANCE` y Hojas de Composición (`Inx-%`, `Outx-%`)
- **Hoja `BALANCE` (Fig. 11a, `raw-027.png`):**
  - Bloques por área (`IN1`, `OUT1`, `BALANCE`, ..., `INn`, `OUTn`, `BALANCE`).
  - Columnas: `Amount kmol`, `Amount kg`, `Amount Nm³`, `Heat Content`, `Total H`, `T*S`, `G`, `Ex Phys` (exergía física), `Ex` (exergía total).
- **Hojas `Inx-%` / `Outx-%` (Fig. 5, `raw-013.png`):**
  - Columnas: `Formula/Element`, `Temper. °C`, `Pressure bar`, `Amount kmol`, `Amount kg`, `(espacio)`, `Content mol-%`, `Content wt-%`.
  - Lista automática de corrientes y sus elementos constituyentes generada bajo demanda (`Summary` o actualización automática).

### 3.4 Paneles Inferiores Acoplados
1. **`Element Balance` (inferior izquierdo):**
   - Columnas: `Element | IN (kmol) | OUT (kmol) | Balance (kmol)`.
   - Resalta automáticamente en rojo/ámbar cualquier elemento con `|Balance| > tolerancia` (desbalance de masa).
2. **`Total Balance` (inferior derecho):**
   - Columnas: `BALANCE AREAS | Amount kmol | Amount kg | Amount Nm3 | Heat Content | Total H | Exergy`.
   - Columna `Amount kg` sombreada en amarillo suave como indicador rápido de conservación de masa global (`0.000` cuando balanceado).

### 3.5 Panel Lateral Derecho `Tools`
Organizado verticalmente idéntico a `raw-001.png`:
1. `Balance Area`: Selector desplegable `View Balance Area` (`Balance Area 1..N`).
2. `Get Species from Database`: Botones `[Input]` y `[Output]` (abren buscador/selector de `soryntel-db` con autocompletado y vista previa de fases/rango $T$, reutilizando el componente de búsqueda del Módulo 1).
3. `Insert`: `[Balance Area]`, `[Stream]`, `[Row]`, `[Heat Flow]`.
4. `Delete`: `[Balance Area]`, `[Selected Stream]`, `[Selected Row]`.
5. `Measure Units`: Radio buttons `Temperature: (°C / K)` y `Energy: (Mcal / MJ / kWh)`.
6. `Functions`: `[Temp. Balance]` y `[Summary]`.

### 3.6 Ventanas y Diálogos Secundarios
1. **Diálogo `Temperature Balance` (Fig. 2, `raw-005.png`):**
   - Muestra: `Temperature of products = {T_adiab} {°C|K} (when Heat Balance = 0)` + botón opcional `"Aplicar a salidas"` y `"OK"`.
2. **Ventana `Target Dialog` (Fig. 7, `raw-018.png`):**
   - Encabezado `Target Calculation Sheet (Goal seek)`.
   - Tabla con bloques `Variable` (`Name`, `Cell Ref.`, `Test value` en rojo, `Min`, `Max`), `Target` (`Name`, `Cell Ref.`, `Value` con fondo amarillo), `Iteration` (`Max`, por defecto `10`).
   - Botones inferiores: `Ins Row`, `Del Row`, `Set variable cell`, `Set target cell`, `Set goal value`, `Iterate Selected Rows`, `Iterate All` (`F8`), `Exit`.
3. **Ventana `Diagram Settings` y `Diagram` (Figs. 8–10, `raw-021.png`..`raw-025.png`):**
   - `Diagram Settings`: Campos `MIN`, `MAX`, `STEP` para `X-Axis Range`, `Diagram title`, grilla `X`, `Y1..Y9` con `Cell Reference`, `Label`, `Units`, `Target row 1..N`. Botones `Set X cell`, `Set Y1 cell`, `Target iteration`, `Set Target rows`, `Diagram`.
   - `Diagram`: Gráfico interactivo XY (en `soryntel-desktop` mediante SVG/Canvas reactivo con zoom, tooltips exactos, toggle de series en leyenda y exportación PNG/SVG/CSV).
