# Análisis del Manual Oficial — Módulo 2: Heat and Material Balance

- **Fuente:** `/home/nando/Documentos/Docs HSC/11-heat-and-material-balance/11-heat-and-material-balance.md` (24 páginas; autores: Petri Kobylin, Antti Roine, Peter Björklund; fecha de revisión del documento: 5 de septiembre de 2023).
- **Imágenes vinculadas:** 19 archivos extraídos del PDF original (`raw-001.png` a `raw-038.png`), cubriendo las Figuras 1 a 14.

---

## 1. Estructura de Secciones del Manual

| Sección | Páginas | Contenido Principal |
|---|---|---|
| **11.1 Basic Calculation Procedure** | 3–4 | Creación de hojas `IN1`, `OUT1`, `BALANCE`; ingreso de especies, `T`, `P`, `kmol`/`kg`/`Nm³`; `EXTRA HEAT` y `HEAT LOSSES` (texto verde); unidades (`°C`/`K`, `Mcal`/`MJ`/`kWh`); convección natural de pérdidas de calor; `Element Balance`; `Temperature Balance` (`Temp. Balance`); corrección de presión para `H2O`. |
| **11.2 Formatting** | 5 | Formato tipo Excel de celdas, fuentes, bordes, colores de encabezado, botones `Copy`/`Paste` y menú `Format`. |
| **11.3 Streams** | 6–7 | Uso de `Insert / Stream` (encabezado azul): la fila de corriente suma automáticamente `kmol`, `kg`, `Nm³`, `Heat Content` y `Total H` de las especies debajo; permite editar el total de la corriente para escalar proporcionalmente sus componentes o editar el porcentaje. |
| **11.4 Formulae in Cells** | 8 | Fórmulas estilo Excel entre celdas y hojas (`=0.938*IN1!C7`, `=0.938*(InCu-D13)/2`); nombres predefinidos de cantidades elementales totales (`InCu`, `OutCu`, `InFe`, `OutFe`, ..., `In1Cu`, `In2Fe`, etc.); resolución de referencias circulares por iteración. |
| **11.5 Elemental Compositions** | 9 | Generación automática de hojas `In1-%` y `Out1-%` con composición elemental (`kmol`, `kg`, `mol-%`, `wt-%`) por cada corriente (`Stream`). |
| **11.6 Additional Sheets** | 10 | Hojas personalizadas (`FEED`, `Analysis`, `Process flow`) con enlace por fórmulas hacia `INx`/`OUTx`; protección visual `Red Font Shield` (convención: valores de entrada en rojo, fórmulas en negro). |
| **11.7 Target Dialog** | 11–12 | Hoja/diálogo `Target Calculation Sheet (Goal seek)` para ajustar celdas variables (`Variable`: `Cell Ref.`, `Test value`, `Min`, `Max`) hasta alcanzar valores objetivo (`Target`: `Cell Ref.`, `Value`, `Iteration Max`). Botones `Iterate Selected Rows` / `Iterate All` (`F8`). Limitación documentada: para iterar temperaturas, solo deben usarse celdas de temperatura de encabezado de **Stream** (no celdas individuales de especie). |
| **11.8 Graphics** | 13–15 | Diálogo `Diagram Settings` y ventana `Diagram` para graficar variables `Y1..Y9` vs `X` en un rango `[MIN, MAX, STEP]`, incluyendo la opción `Target iteration` con `Target row 1..N` para re-converger objetivos en cada paso de `X`. |
| **11.9 Multiple Balance Areas** | 16–20 | Modelos multi-etapa (`IN1`/`OUT1` ... `IN5`/`OUT5`) como `FSF_process.bal8` (FSF, Convertidor I, Convertidor II, Caldera, ESP); `Insert / Balance Area`; `Copy Stream` / `Paste Stream` para conectar salidas con entradas; manejo de flujos de retorno circulares (polvos de caldera/ESP/convertidores) y tabla comparativa global en hoja `BALANCE` incluyendo `T*S`, `G` y `Ex Phys` (exergía física). |
| **11.10 General Considerations** | 21–22 | Definiciones termodinámicas clave: `Heat Content` (calor sensible + latente desde $298.15\text{ K}$) vs `Total H` ($H_f^\circ(298.15) + \text{Heat Content}$); convenciones de signos (calor liberado vs absorbido); relación con el módulo `HeatLoss`. |
| **11.11 Examples** | 23–24 | Catálogo de ejemplos incluidos: `CUSMELT`, `BUTANE` (llama adiabática), `H2O` (evaporador con corrección de presión), `COAL1/2`, `FUELOIL1/2`, `NATGAS1/2` (poderes caloríficos), `FSF_process`, `Iron_process`, `Pool1/2`, `Turbine`. |

---

## 2. Reglas, Constantes y Ecuaciones Documentadas en el Manual

1. **Conversión de energía documentada (Sección 11.1):**
   $$1\text{ kWh} = 3.6\text{ MJ} = 0.8604\text{ Mcal}_{\text{th}}$$
2. **Ecuación empírica de pérdidas de calor por convección natural (Sección 11.1):**
   $$H_{\text{loss}} = (6.8 + 0.046 \cdot T_2) \cdot (T_2 - T_1) \cdot A \quad [\text{kcal/h}]$$
   donde $T_2$ es la temperatura de superficie ($^\circ\text{C}$), $T_1$ la temperatura ambiente ($^\circ\text{C}$) y $A$ el área ($\text{m}^2$).
3. **Diferencia entre `Heat Content` y `Total H` (Sección 11.10):**
   - `Heat Content` ($H_{\text{cont}}$): $\Delta H_{\text{sensible + latente}}(T) = H(T) - H(298.15\text{ K})$. Es $0$ a $25\,^\circ\text{C}$ ($298.15\text{ K}$).
   - `Total H` ($H_{\text{tot}}$): $H_f^\circ(298.15\text{ K}) + \big(H(T) - H(298.15\text{ K})\big)$.
4. **Criterio de Balance Térmico:**
   $$\Delta H_{\text{balance}} = \sum_{\text{OUT}} H_{\text{tot}} + Q_{\text{losses}} - \left(\sum_{\text{IN}} H_{\text{tot}} + Q_{\text{extra}}\right)$$
   - Si $\Delta H_{\text{balance}} < 0$: el proceso libera calor (exotérmico en neto / requiere enfriamiento o produce exceso de calor).
   - Si $\Delta H_{\text{balance}} > 0$: el proceso requiere aporte externo de calor.
   - `Temp. Balance`: encuentra $T_{\text{out}}$ común tal que $\Delta H_{\text{balance}}(T_{\text{out}}) = 0$.
5. **Corrección de presión para $\text{H}_2\text{O}$ (`Insert / Pressure correction H2O`):**
   - Documentada en Sección 11.1 y ejemplo `H2O`: permite que especies `H2O` y `H2O(g)` respondan a la columna `Pressure (bar)` usando ecuaciones de estado del agua/vapor (a $1\text{ bar}$, ebullición a $\approx 99.6\,^\circ\text{C}$; a $1.01325\text{ bar}$, $100\,^\circ\text{C}$).
6. **Nombres definidos para balance elemental:**
   - Para una única área o área 1: `InAc`, `OutAc`, ..., `InCu`, `OutCu`, `InFe`, `OutFe`, ...
   - Para múltiples áreas: `In1Cu`, `Out1Cu`, `In2Cu`, `Out2Cu`, etc.
7. **Convención `Target Dialog`:**
   - Columna `C` (`Test value`) se muestra en **rojo** como valor iterado inicial/actual.
   - Columna `H` (`Value` objetivo) aparece sombreada en **amarillo claro**.
   - Límite de iteraciones por defecto: `10` (editable en columna `I`).
