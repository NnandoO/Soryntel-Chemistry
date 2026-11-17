# Evaluación de Ingeniería Inversa en Ghidra — Módulo 4

## 1. Necesidad de Decompilación Profunda
- **Conclusión:** **No es bloqueante ni necesaria** una decompilación completa línea por línea de `Gibbs5.dll` en Ghidra para implementar el Módulo 4 en Rust con precisión científica, por tres razones verificadas:
  1. **Algoritmo estándar explícitamente citado:** El manual (§13.11) cita de forma exacta la formulación de **White, Johnson & Dantzig (1958)** y **Lampinen & Vuorisalo (1991, 1992)**, además de proporcionar las ecuaciones explícitas de **Redlich–Kister** (Apéndices A, B y C).
  2. **Arquitectura VB6 (`MSVBVM60.DLL`):** Tanto `GIBBS.EXE` como `Gibbs5.dll` están compilados con Visual Basic 6 en modo p-code/nativo con fuerte despacho COM/Variant, lo cual produce decompilaciones ruidosas en Ghidra frente a las especificaciones matemáticas exactas ya documentadas.
  3. **Oráculos numéricos completos en `/home/nando/Descargas/HSC6/Gibbs/`:** Existen decenas de pares `.IGI` (entrada completa con coeficientes $C_p$, $H$, $S$, matriz atómica $A$ y cantidades iniciales) y `.OGI` (salida completa con `Gmin#`, `Temperature`, `Pressure`, `Volume`, `Reaction enthalpy`, `Reaction entropy`, `EQUIL AMOUNT`, `MOLE FRACT`, `ACTIVITY`, `ACTIVITY COEFFICI` con 4–12 cifras significativas).

## 2. Casos Puntuales Donde Ghidra Sería Útil (Opcional en Fase de Ajuste Fino)
Si durante la validación contra `.OGI` apareciera una discrepancia menor en:
- El umbral de convergencia o perturbación inicial $y_i^{(0)}$ cuando $b_e > 0$ pero algunas especies tienen entrada cero,
- La parametrización exacta interna del modelo `Aqua` (Pitzer / Debye–Hückel) en HSC6,
se recomienda inspeccionar únicamente la rutina `CalculateIgi` dentro de `Gibbs5.dll` (`base 0x11000000`) usando Ghidra para confirmar los coeficientes empíricos de Pitzer o tolerancias de pivoteo.
