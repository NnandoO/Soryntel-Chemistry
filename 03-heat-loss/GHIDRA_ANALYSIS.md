# Evaluación de Uso de Ghidra (`GHIDRA_ANALYSIS.md`) — Módulo 3

## 1. Naturaleza del Binario Analizado

- **Objetivo:** `/home/nando/Descargas/HSC6/HeatLoss.exe` (`2,301,952` bytes, SHA-256 `d08f12967191655ed6407d25e2a8f4884ceefde15535f921babbcaf341c49876`).
- **Tipo:** Ejecutable PE32 x86 compilado en **Visual Basic 6.0 nativo** con única dependencia directa `MSVBVM60.DLL` y fuerte acoplamiento a controles ActiveX (`TTF16.ocx` Formula One).

---

## 2. Evaluación Costo–Beneficio de Decompilación en Ghidra

Siguiendo el criterio metodológico del proyecto (*no forzar el uso de Ghidra cuando no sea necesario; evaluar si aporta fórmulas, constantes o flujos que no estén ya documentados o accesibles*):

1. **¿Están las ecuaciones y correlaciones completamente documentadas?**
   - **Sí.** El manual oficial (`12-heat-loss.md`, Sección 12.4) documenta de manera explícita **todas las 53 ecuaciones** del motor: conducción multicapa con malla $m$ (Ecs. 8–12), todas las correlaciones adimensionales de convección libre y forzada con sus rangos exactos de $Ra_L, Re_L, Pr$ (Ecs. 13–40), radiación superficial y en recintos (Ecs. 41–43), radiación de gases mediante EWBM (Ecs. 44–47, Tabla 1) y radiación combinada gas + partículas (Ecs. 48–53), además de citar las 10 fuentes científicas primarias exactas (Incropera & DeWitt 1996, Edwards & Balakrishnan 1973, Lallemant & Weber 1996, VDI Heat Atlas 1993).
2. **¿Las constantes de materiales y fluidos están en código o en bases externas?**
   - Están en archivos externos legibles (`/home/nando/Descargas/HSC6/Databases/heatcond.xls`, `heatconv.xls`, `heatradi.xls`) y en las hojas ejemplo `HeatLoss/*.HTR` / `*.XLS`, no ocultas en binarios opacos.
3. **¿Qué aporta el código VB6 nativo en Ghidra?**
   - En binarios VB6 nativos, la mayor parte de `.text` corresponde a despacho COM/OLE hacia `TTF16.ocx` (`Variant`, `__vbaVarAdd`, `__vbaLateMemCall`, etc.), lo cual introduce ruido alto en el decompilador para recuperar lógica que ya es explícita en la especificación científica y en los casos de prueba numéricos exactos (`SimpleCase`, `Smelting1`, `Radiator`, `Hotwatertank`, `OilFurnace`).
4. **¿Cuándo sí se justificaría abrir Ghidra puntualmente en el futuro?**
   - Únicamente si durante la fase de validación (`VALIDATION_PLAN.md`) existiera una discrepancia numérica inexplicable en:
     - El orden exacto de amortiguamiento/convergencia de `CalHeat` frente a casos límite, o
     - Algún parámetro espectral específico de las 6 especies en `GasRad` (por ejemplo, si Lallemant & Weber 1996 difiere ligeramente de la tabla usada por Tapio Ahokainen en `OilFurnace.htr8`).

---

## 3. Conclusión y Decisión

- **Decisión actual:** **Ghidra NO es necesario en esta fase de investigación**, ya que el 100% del modelo físico, las 53 ecuaciones, los rangos de validez, los algoritmos de cálculo y múltiples casos de validación con resultados exactos están documentados en `12-heat-loss.md`, las 14 capturas y las bases `.xls`/`.htr`.
- Se documenta esta decisión de forma transparente para evitar ingeniería inversa redundante sobre despacho COM de VB6, manteniendo abierta la opción de análisis dirigido en Ghidra solo ante discrepancias específicas en validación numérica.
