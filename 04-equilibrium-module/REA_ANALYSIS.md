# Análisis Estático (PE32 / COM / Strings) — Módulo 4 (`GIBBS.EXE` y `Gibbs5.dll`)

## 1. Nota de Honestidad sobre la Herramienta `rea`
Al intentar invocar `/home/nando/tools/rea/scripts/rea.mjs` y `rea` desde terminal, el sistema reportó que **el runtime compilado de REA no está construido** (`REA's compiled runtime is missing`, `which rea` no encontrado). Por tanto, no se ejecutó el MCP/CLI de `rea` para este módulo; en su lugar se realizó **inspección estática directa** sobre los binarios PE32 en Python (cabeceras COFF/PE, secciones, tabla de importaciones/exportaciones y extracción de cadenas ASCII/UTF-16LE).

## 2. Inventario de Binarios y Datos en `/home/nando/Descargas/HSC6/`

| Archivo | Tamaño | Arquitectura / Formato | Timestamp | Base / Entry |
|---|---:|---|---|---|
| `GIBBS.EXE` | 430,080 B | PE32 x86 (`0x014c`), GUI VB6 | `1155211800` (`2006-08-10`) | `0x00400000` / `0x0000264c` |
| `Gibbs5.dll` | 405,504 B | PE32 x86 (`0x014c`), ActiveX/COM VB6 | `1081347024` (`2004-04-07`) | `0x11000000` / `0x00002120` |
| `Gibbs/*.GEM, .IGI, .OGI, .ISG, .OSG` | Múltiples | Datasets y salidas de referencia | — | Ej. `CACO3`, `FES2`, `AMMONIA`, `GAAS`, `ALPL` |

## 3. Hallazgos Estáticos en `GIBBS.EXE`
- **Dependencias UI/Runtime:** `MSVBVM60.DLL`, `TTF16.ocx` (Formula One Workbook `F1Book` / `F1Gibbs`), `COMDLG32.OCX`, `MSCOMCTL.OCX`.
- **Formulario y rutinas internas identificadas:**
  - Formulario principal `GibForm`.
  - Procedimientos/métodos: `MCPhase`, `MDelPhase`, `MCalGibbs`, `MInsPhase`, `InsPhase`, `ReadFormula1`, `CheckDLL`.
  - Esto confirma que `GIBBS.EXE` actúa únicamente como interfaz de hoja de cálculo (`TTF16.ocx`) y orquestador de archivos `.GEM`/`.IGI`/`.OGI`, delegando el cálculo termodinámico al servidor COM `Gibbs5.dll`.

## 4. Hallazgos Estáticos en `Gibbs5.dll`
- **Exportaciones COM:** `DllCanUnloadNow`, `DllGetClassObject`, `DllRegisterServer`, `DllUnregisterServer`.
- **Identificadores internos:** `Gibbs 5.1 Functions`, `CalculateIgi`, referencias heredadas a `GIBBS432.DLL`, `LGAUSSK_DLL`, `QPRO32.DLL`, `VBA6.DLL` (lo que evidencia que el solver Fortran/VB clásico de `GIBBS 3.0 / 4.5` fue compilado directamente dentro de `Gibbs5.dll` para HSC5/6).
- **Cabeceras de salida detectadas en strings:**
  - `GIBBS 4.5 WINDOWS FOR MULTIPHASE EQUILIBRIUM CALCULATIONS`
  - `INPUT AMOUNT`, `EQUIL AMOUNT`, `MOLE FRACT`, `ACTIVITY`, `ACTIVITY COEFFICI`
  - `Reaction enthalpy`, `Reaction entropy`, `Iterations`, `Limit = 100`.
