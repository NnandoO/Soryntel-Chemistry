# Análisis Visual de Imágenes — Módulo 2: Heat and Material Balance

Todas las imágenes fueron extraídas directamente de `/home/nando/Documentos/Docs HSC/11-heat-and-material-balance/11-heat-and-material-balance.pdf` en `/home/nando/Documentos/HSC-REA/02-heat-and-material-balance/imagenes/` e inspeccionadas visualmente sin depender de OCR.

---

## Inventario y Mapeo Visual (Figs. 1–14)

| Archivo | Tamaño | Figura | Sección Manual | Descripción Visual Observada |
|---|---|---|---|---|
| `raw-001.png` | 1372×518 | **Fig. 1** | 11.1 (p.3) | Ventana principal `HSC Heat and Material Balance Calculator - C:\HSC8\Balance\CUCONV1.bal8`. Hoja activa `IN1` con columnas `A..K` (`Formula`, `Temper. °C`, `Pressure bar`, `Amount kmol`, `Amount kg`, `Amount Nm³`, `Heat Content MJ`, `Total H MJ`, `Heat Cont MJ/kmol`, `Tot H MJ/kmol`, `MW`). Corrientes en negrita (`GAS:`, `FLUX:`, `MATTE:`, `COOLING SCRAP:`) y `EXTRA HEAT:` en verde. Panel inferior izquierdo `Element Balance`, inferior central `Total Balance` (`Balance (1)` con `Heat Content`, `Total H`, `Exergy`), panel derecho `Tools` (`Balance Area`, `Get Species from Database`, `Insert`, `Delete`, `Measure Units`, `Functions`). |
| `raw-005.png` | 299×182 | **Fig. 2** | 11.1 (p.4) | Diálogo modal `Temperature Balance`: muestra el texto `"Temperature of products = 1576.9 °C (when Heat Balance = 0)"` y botón `OK`. |
| `raw-009.png` | 1373×536 | **Fig. 3** | 11.1 (p.4) | Hoja `OUT1` de `CUCONV1.bal8`. Corrientes: `PROCESS GAS:` (`1250 °C`: `N2(g)`, `O2(g)`, `SO2(g)`), `SLAG:` (`1300 °C`: `*2FeO*SiO2(l)`, `Fe3O4(l)`, `Cu2O(l)`), `WHITE METAL:` (`1250 °C`: `Cu2S(l)`), `METAL:` (`1250 °C`: `Cu(l)`), y `HEAT LOSESS:` en verde (`0.00`). |
| `raw-011.png` | 1374×538 | **Fig. 4** | 11.4 (p.8) | Hoja `OUT1` con la celda `D11` (`Cu2S(l)`, `0.469 kmol`) seleccionada; barra de fórmulas mostrando `=0.938*(InCu-D13)/2`, confirmando el uso de variables elementales globales (`InCu`) y sintaxis Excel. |
| `raw-013.png` | 1374×540 | **Fig. 5** | 11.5 (p.9) | Hoja `Out1-%` (`OUT1 Compositions`). Columnas: `A: OUT1 Compositions`, `B: Temper. °C`, `C: Pressure bar`, `D: Amount kmol`, `E: Amount kg`, `F: (vacía)`, `G: Content mol-%`, `H: Content wt-%`. Muestra desglose elemental por corriente (`PROCESS GAS: N, O, S`; `SLAG: Cu, Fe, O, Si`; `WHITE METAL: Cu, S`; `METAL:`). |
| `raw-015.png` | 1375×541 | **Fig. 6** | 11.6 (p.10) | Archivo `CUCONV2.bal8`, hoja personalizada `FEED` (`Copper Converter Feed`). Usa valores en **rojo** para entradas editables (`99.97 Nm3/h`, `79.00 vol-%`, `21.00 vol-%`, `12.92 kg/h`, `127.05 kg/h`, `50.02 wt-%`, `23.74 wt-%`) y negro para valores calculados (`78.98 Nm3/h`, `79.57 kg/h`, `47.47 kg/h`, `26.24 wt-%`). Celda `C11` (`SCRAP: Amount = 0.00`) seleccionada. |
| `raw-018.png` | 737×512 | **Fig. 7** | 11.7 (p.11) | Ventana `Target Dialog` (`Target Calculation Sheet (Goal seek)`). Menú propio: `File | Edit | Insert | Delete | Iterate`. Tabla naranja/amarilla con secciones `Variable` (`A: Name`, `B: Cell Ref.`, `C: Test value` en rojo, `D: Min`, `E: Max`), `Target` (`F: Name`, `G: Cell Ref.`, `H: Value` en amarillo), `Iteration` (`I: Max` = 10). Ejemplo con 3 filas acopladas (`SCRAP`, `Fe wt-%`, `Fe wt-%` contra `HEAT BAL = 0.00` y `Fe Amount = 0.540`). Botones inferiores: `Ins Row`, `Del Row`, `Exit`, `Set variable cell`, `Set target cell`, `Set goal value`, `Iterate Selected Rows`, `Iterate All`. |
| `raw-021.png` | 629×571 | **Fig. 8** | 11.8 (p.13) | Ventana `Diagram Settings`. Sección `X-Axis Range:` (`MIN = 0.000`, `MAX = 100`, `STEP = 2`), `Diagram title` (`Diagram`). Tabla de series `X`, `Y1..Y9` con columnas `Cell Reference` (`FEED!C11`, `BALANCE!H4`), `Label` (`SCRAP:`, `BALANCE`), `Units` (`Total H kWh`), `Target row 1..3`. Botones: `Ins Row`, `Ins Col`, `Del Row`, `Del Col`, `Clear target cells`, `Clear all`, `Cancel`, `Set X cell`, `Set Y1 cell`, `Target iteration`, `Set Target rows`, `Diagram`. |
| `raw-022.png` | 630×571 | **Fig. 9** | 11.8 (p.13) | Ventana `Diagram` resultante: gráfico XY azul con puntos rellenos (`Total H kWh` de `-13` a `+7` vs `SCRAP:` de `0` a `100`), leyenda con checkbox `BALANCE`, barra superior de dibujo/anotación y barra izquierda de guardar/copiar/imprimir/actualizar. |
| `raw-024.png` | 629×572 | **Fig. 10a** | 11.8 (p.14) | `Diagram Settings` con `X = FEED!C9` (`20..25`, paso `1`), `Y1 = FEED!C11` (`SCRAP:`), y `Target row 1 = 4` en celda `E3` para ejecutar el Target en cada paso de `X`. |
| `raw-025.png` | 630×570 | **Fig. 10b** | 11.8 (p.14) | `Diagram` resultante de `Target iteration`: `SCRAP:` (`0` a `85 kg/h`) en función de `Fe wt-%` (`20` a `25`). |
| `raw-027.png` | 1372×539 | **Fig. 11a** | 11.9 (p.16) | `FSF_process.bal8`, hoja `BALANCE`. Resumen multi-área (`IN1`/`OUT1`/`BALANCE` hasta `IN5`/`OUT5`/`BALANCE`) con columnas adicionales: `I: T*S kWh`, `J: G kWh`, `K: Ex Phys kWh`, `L: Ex...`. Pestañas: `BALANCE`, `IN1..OUT5`, `Process flow`, `Analysis`, `Target`. |
| `raw-029.png` | 1372×382 | **Fig. 11b (sup.)** | 11.9 (p.17) | `FSF_process.bal8`, hoja `IN1` filas 1–17 (`Balance Area 1` seleccionado en `Tools`). Celda `D15` (`Flue dust (Boiler return) -> Cu2O`) muestra fórmula `='OUT4'!D12` conectando la salida de la Caldera (área 4) con la entrada del FSF (área 1). |
| `raw-030.png` | 1374×377 | **Fig. 11b (inf.)** | 11.9 (p.17) | Continuación de `IN1` filas 18–30 (`Flue dust (ESP return)`, `Conv. 1 return`, `Conv. 2 return`). |
| `raw-032.png` | 1372×543 | **Fig. 12** | 11.9 (p.18) | `FSF_process.bal8`, hoja `OUT2` (`OUTPUT SPECIES (2)`). Celda `D6` (`CaO*Al2O3*2SiO2` en `Slag`) con fórmula `=InCa2*Analysis!L28/100`. Incluye `Heat loss (Converter I): 10.00 kWh` en verde. |
| `raw-034.png` | 1372×362 | **Fig. 13 (sup.)** | 11.9 (p.19) | `FSF_process.bal8`, hoja `Process flow` filas 1–18: diagrama esquemático construido en la cuadrícula mostrando bloques `FSF (1)`, `BOILER (4)`, `ESP (5)`, flujos en cajas de colores (`Concentrate`, `Flux`, `Air`, `Matte`, `Slag`, `Excess heat: 175.1 kW`) con entradas en rojo. |
| `raw-035.png` | 1374×356 | **Fig. 13 (inf.)** | 11.9 (p.19) | Continuación de `Process flow` filas 19–32: `CONV. II (3)`, `White metal`, `Liquid Cu`, `Scrap` y tabla resumen `HEAT LOSSES:` (`FSF 90 kW`, `Conv I 10 kW`, `Conv II 10 kW`, `Boiler 80 kW`, `ESP 10 kW`). |
| `raw-037.png` | 1373×459 | **Fig. 14 (sup.)** | 11.9 (p.20) | `FSF_process.bal8`, hoja `Analysis` filas 1–19: tablas `INPUT ANALYSIS`, `OUTPUT ANALYSIS`, `FSF DISTRIBUTION` y `CONVERTER I, DISTRIBUTION`. Celda `D15` con fórmula `=D16-D14` (`40.0 vol-%` de `N2(g)`). |
| `raw-038.png` | 1374×455 | **Fig. 14 (inf.)** | 11.9 (p.20) | Continuación de `Analysis` filas 21–35: coeficientes de distribución elemental (`wt-%` en rojo) por especie y corriente para `FSF`, `CONVERTER I` y `CONVERTER II`. |

---

## Hallazgos Clave de UI / Datos a Partir de las Imágenes

1. **Consistencia de Layout Principal:**
   - Barra de menús clásica en la parte superior (`File`, `Edit`, `View`, `Insert`, `Delete`, `Format`, `Units`, `Calculate`, `Target`, `Diagram`, `Help`).
   - Barra de fórmulas (`Name Box` + `Formula Bar`, p. ej. `D11 | =0.938*(InCu-D13)/2`).
   - Área central de hoja de cálculo con pestañas inferiores (`BALANCE`, `IN1`, `OUT1`, `In1-%`, `Out1-%`, hojas personalizadas).
   - Dos paneles acoplados inferiores (`Element Balance` a la izquierda, `Total Balance` a la derecha).
   - Panel lateral derecho fijo `Tools` con acciones rápidas agrupadas en 5 bloques (`Balance Area`, `Get Species from Database`, `Insert`, `Delete`, `Measure Units`, `Functions`).
2. **Columnas extra en hoja `BALANCE` (Fig. 11a):**
   - Además de `Amount kmol`, `Amount kg`, `Amount Nm³`, `Heat Content`, `Total H`, la hoja `BALANCE` muestra `T*S`, `G`, `Ex Phys` (exergía física) y `Ex` (exergía total/química en columna L parcialmente visible).
3. **Semántica de colores en celdas:**
   - **Azul / negrita en fila de stream:** encabezado de corriente (`Stream`).
   - **Texto verde:** `EXTRA HEAT:` (en `INx`) y `HEAT LOSSES:` (en `OUTx`).
   - **Texto rojo:** entradas manuales (`Red Font Shield`, `FEED`, `Analysis`, `Process flow`, `Target Test value`).
   - **Fondo amarillo claro:** columna `Amount kg` en `Total Balance` y `Goal Value` en `Target Dialog`.
