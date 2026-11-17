# Análisis mediante REA y Análisis Estático (`REA_ANALYSIS.md`) — Módulo 3

## 1. Herramientas Invocadas

1. **Reverse Engineer Anything (REA) CLI (`rea`):**
   - Comando ejecutado: `rea inspect-artifact /home/nando/Descargas/HSC6/HeatLoss.exe --detail summary --evidence_output /tmp/rea_heatloss_exe.json`
   - Estado: Exitoso (`exit code 0`). Archivo de evidencia generado: `/tmp/rea_heatloss_exe.json`.
2. **Inspección estática de cabecera PE32, secciones, tabla de importaciones y strings (`python3` en modo solo lectura):**
   - Ejecutada sobre `/home/nando/Descargas/HSC6/HeatLoss.exe`, `/home/nando/Descargas/HSC6/HeatLoss/SimpleCase.HTR`, `Hotwatertank.HTR` y `/home/nando/Descargas/HSC6/Databases/{heatcond,heatconv,heatradi}.xls`.

---

## 2. Resultados de REA (`inspect-artifact`)

- **Archivo analizado:** `/home/nando/Descargas/HSC6/HeatLoss.exe`
- **Formato reportado por REA:** `pe` (`artifact_kind: executable`)
- **Tamaño:** `2,301,952` bytes
- **SHA-256 verificado:** `d08f12967191655ed6407d25e2a8f4884ceefde15535f921babbcaf341c49876`
- **Proveedor REA:** `rea-artifact-graph` v1 (`status: complete`, `hash_status: verified`).

---

## 3. Estructura PE32, Importaciones y Controles ActiveX

Análisis directo de la cabecera PE32 de `HeatLoss.exe`:
- **Arquitectura:** x86 32-bit (`Machine = 0x014c`, `Magic = 0x010b` PE32).
- **Subsistema:** Windows GUI (`Subsystem = 2`), `ImageBase = 0x00400000`, `EntryPoint = 0x00008480`.
- **Timestamp de compilación PE:** `1155211875` (`2006-08-10 12:11:15 UTC`).
- **Secciones PE:**
  - `.text`: VA `0x1000`, VirtualSize `2,287,624` B, RawSize `2,289,664` B.
  - `.data`: VA `0x230000`, VirtualSize `43,992` B, RawSize `4,096` B.
  - `.rsrc`: VA `0x23b000`, VirtualSize `1,848` B, RawSize `4,096` B.
- **Tabla de importación (IAT):**
  - Única DLL importada directamente en IAT: **`MSVBVM60.DLL`** (confirma compilación nativa en **Microsoft Visual Basic 6.0**, sin metadata .NET).
- **Referencias ActiveX / COM / Librerías extraídas de strings:**
  - `TTF16.ocx` / `TTF160Ctl.F1Book` (Visual Components Formula One 6 — motor de hoja de cálculo).
  - `COMDLG32.OCX` (`MSComDlg.CommonDialog`), `MhOutBox.ocx`, `QPRO32.DLL`, `VBA6.DLL`, `kernel32.dll`.

---

## 4. Formularios, Módulos y Mensajes Internos Identificados

### 4.1 Formularios y Módulos VB6 (`HeatLoss.exe`)
- **Formularios de UI:** `HeatMain` (ventana principal con `F1BookQ`, `F1BookP`, `F1BookE`, `F1BookE2`, `F1BookD`), `HeatCond`, `HeatConv`, `HeatRadi`, `GasRad`, `HeatLossTarget`, `HeatLossDia`.
- **Módulo de cálculo:** `CalHeat`.
- **Grids embebidos:** `F1Conv`, `F1Gas`, `F1GasRes`, `F1Radi`, `F1Target`.
- **Autoría interna encontrada en strings:**
  - `Heat Loss Calculations by P Bj` (Peter Björklund).
  - `Gas Mixture Radiation Calculator - Tapio Ahokainen`.
  - `Particle Suspension Radiation Calculator`.

### 4.2 Claves Internas y Mensajes de Validación (UTF-16LE)
- Identificadores internos de propiedades: `cond_conductivity`, `conv_conductivity`, `emissivity`, `gas_emissivity`, `part_emissivity`.
- Especies gaseosas soportadas: `H2O(g)`, `CO2(g)`, `SO2(g)`, `CH4(g)` (junto con `CO(g)`, `NO(g)` documentados).
- Mensajes de error y validación observados:
  - `"Please specify k (row 6) or press 'Conduction' for column number "`
  - `"Please specify hc or hr (row 6 or 7) or press 'Convection'/'Radiation' for column number "`
  - `"4. Calculation grid too small => Increase grid size on row 13."`
  - `"Please enter a surface emissivity value between 0 and 1."`
  - `"Gas emissivity value out of limit!"`
  - `"Please calculate Gas Radiation iteratively first!"`
  - `"Zero absorptivity and emissivity !"`
  - `"Iteration did not converge, returning results from last iteration."`
  - `"Iteration did not converge, too low Variable max value ?"` / `"too high Variable min value ?"`

---

## 5. Formato de Archivos `.HTR` y Bases `.XLS`

- **`SimpleCase.HTR` (28,159 B) y `Hotwatertank.HTR` (169,217 B):**
  - Cabecera observada: `09 00 06 05 00 06 00 00 00 ee 04 11 06 00 00 ef` (formato binario propietario del control **Formula One `TTF16.ocx`**, análogo a `.BAL` en el Módulo 2).
- **Bases de datos en `/home/nando/Descargas/HSC6/Databases/`:**
  - `heatcond.xls` (395,776 B), `heatconv.xls` (172,032 B), `heatradi.xls` (28,160 B) en formato Microsoft Excel 97–2003 BIFF8 (`.xls`).
  - En Soryntel (`soryntel-db`), estas tablas se migrarán a SQLite normalizado (y/o importador `calamine` en Rust puro) para independencia total de Excel/COM.
