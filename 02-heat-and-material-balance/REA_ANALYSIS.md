# Análisis con REA (Reverse Engineer Anything) — Módulo 2

## 1. Herramientas y Artefactos Analizados

Siguiendo la metodología del proyecto, se ejecutaron herramientas de análisis estático en modo de solo lectura sobre los binarios originales de `/home/nando/Descargas/HSC6/` y sus archivos de datos en `/home/nando/Descargas/HSC6/Balance/`:

1. **REA CLI (`rea inspect-artifact`)** sobre:
   - `/home/nando/Descargas/HSC6/HSC.EXE` (SHA-256: `12ea9147b70c4c523a43b0ae4634e04b0737c47bed585ec91c8b5e5c9bd37e02`, formato `pe`)
   - `/home/nando/Descargas/HSC6/HSC6.DLL` (SHA-256: `4153eb6675f17c1372c7b9b31cdfe330658fab6efdee73e9191a3a383e34dcb9`, formato `pe`)
2. **Extracción de strings ASCII y UTF-16LE (`strings -n 5` / `strings -el`)** sobre `HSC.EXE` y `HSC6.DLL`.
3. **Inspección de formato (`file`) y lectura directa** de `/home/nando/Descargas/HSC6/Balance/*.BAL` y `*.OHE`.

---

## 2. Arquitectura Binaria del Módulo 2 en HSC 6.0

A diferencia de módulos independientes como `GIBBS.EXE`, `EPH.EXE` o `CELL.EXE`, el módulo **Heat and Material Balance** en HSC 6.0 **reside dentro del ejecutable principal `HSC.EXE`** (compilado en Visual Basic 6, runtime `MSVBVM60.DLL`) y delega el cálculo termodinámico en la biblioteca compartida **`HSC6.DLL`**.

### 2.1 Símbolos e Identificadores en `HSC.EXE`
La extracción de strings de `HSC.EXE` reveló la autoría interna y los formularios/controles del módulo:
- **Identificador interno:** `Heat and Material Balance - Peter Bj`
- **Controles de hoja de cálculo (`Formula One`, prefijo `F1*`):**
  - `F1BalTot`: grilla de balance total.
  - `F1Stream1`, `F1Stream2`: grillas de corrientes `IN` y `OUT`.
  - `F1Target`: grilla del diálogo `Target Calculation Sheet (Goal seek)`.
- **Módulos / Formularios / Procedimientos identificados:**
  - `BalAreas`, `BalDia`, `BalLayout`, `BalList`, `BalMat`, `BalTarget`, `BalTem`, `BalTot`, `CalTBal`
  - `GetBalAreas`, `GetEleBal`, `GetTarget`
  - `MCStream`, `MDelStream`, `MInsStream`, `MInsTarget`, `MIterate`, `MIterateA`, `MIterateS`
  - `MTarget`, `MTargetCalS`, `MTargetCalS_Click`, `MTargetD`, `MTargetSheet`
- **Mensajes de validación observados (UTF-16LE):**
  - `"Can not Paste a Stream to an OUT sheet unless inside a Balance Area!"`
  - `"Can only Copy a Stream from an OUT sheet unless inside a Balance Area!"`
  - `"Not a Balance Area Sheet!"`
  - `"Syntax error in Target Cell Reference !"`
  - `"No formula in target cell: "`
  - `"No target cell reference on row "`
  - `"Please give Target Value !"`
  - `"Element Balance Error:  "`

### 2.2 Métodos Exportados / COM en `HSC6.DLL`
En `HSC6.DLL` se identificaron las funciones utilizadas para evaluar corrientes, especies y agua a presión:
- **Propiedades de corrientes (`Stream*`):**
  - `StreamH`, `StreamHadd`, `StreamHCMadd`, `StreamHKGadd`
  - `StreamCP`, `StreamCPadd`
  - `StreamG`, `StreamGadd`
  - `StreamEQ`, `StreamEQAdd`, `oldStreamEQAdd`
  - `StreamGetX`, `gStreamH`, `gStreamS`, `gStreamG`, `gStreamX` (exergía)
  - `BALadd`, `MolPadd`, `WTPadd`, `CreateStreamFormulas`, `CopyStreamData`
- **Propiedades de $\text{H}_2\text{O}$ dependientes de presión (`*_H2O`):**
  - `H_H2O`, `HKG_H2O`, `S_H2O`, `SKG_H2O`, `CP_H2O`, `CPKG_H2O`, `G_H2O`, `GKG_H2O`, `Critical_H2O`
  - Confirma que la corrección de presión para `H2O` / `H2O(g)` se resuelve mediante un módulo específico separado de la regresión polinómica estándar de `Maindb6.HSC`.

---

## 3. Formato de Archivos de Datos en `/Balance/`

1. **Archivos `.BAL` (`BUTANE.BAL`, `CUCONV1.BAL`, `FSF_process.BAL`, etc.):**
   - `file` reporta: **`Excel 2 BIFF 2`** (formato binario heredado de Microsoft Excel 2.x soportado nativamente por el control ActiveX *Formula One* `VCF132.OCX` en HSC6; en HSC8+ evolucionó a `.bal8`).
2. **Archivos `.OHE` (`BUTANE.OHE`, `H2O.OHE`, `CUSMELT.OHE`, `CUCONV.OHE`):**
   - Reportes en texto plano generados por HSC con:
     - `MASS BALANCE I` (`kmol`) y `MASS BALANCE II` (`kg`) por elemento (`INPUT`, `OUTPUT`, `DIFF.`).
     - `HEAT BALANCE`: tablas `INPUT` y `OUTPUT` con `COMPONENT`, `TEMPERATURE (C)`, `AMOUNT (kmol, kg/Nm3)`, `LATENT H (MJ)` (= `Heat Content`), `TOTAL H (MJ)`.
     - `HEAT IS RELEASED` / absorbed y `FINAL TEMPERATURE OF THE PRODUCTS = ... C`.
   - **Nota sobre nomenclatura en `.OHE`:** En HSC6, la columna de `Heat Content` aparecía etiquetada como `LATENT H` en el reporte `.OHE`; en HSC8+ (capturas del manual) se renombró con mayor precisión a `Heat Content`.

---

## 4. Conclusiones para Soryntel

1. **No depender de ActiveX/BIFF2 en Rust:** El motor de hojas y fórmulas en `HSC.EXE` es esencialmente una instancia de *Formula One* (`F1*`) acoplada a llamadas COM de `HSC6.DLL`. En **Soryntel**, reemplazaremos esto por un evaluador de grafo de dependencias (DAG + iteración de ciclos) en Rust puro y almacenamiento moderno (JSON/SQLite), manteniendo opcionalmente un importador de lectura si se requiere en el futuro.
2. **Exergía (`gStreamX` / `Ex Phys`):** `HSC6.DLL` calcula exergía de corrientes de forma nativa además de $H$, $S$ y $G$, lo que se alinea con la hoja `BALANCE` mostrada en la Fig. 11a.
