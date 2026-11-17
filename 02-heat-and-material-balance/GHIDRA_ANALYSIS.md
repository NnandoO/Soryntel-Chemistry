# Evaluación de Uso de Ghidra — Módulo 2: Heat and Material Balance

## 1. Decisión Técnica

**No se requiere decompilación profunda con Ghidra para el Módulo 2**, por las siguientes razones técnicas documentadas:

1. **Naturaleza del binario (`HSC.EXE`):**
   - `HSC.EXE` es un ejecutable **Visual Basic 6 (P-code / native VB6 COM)** cuyo núcleo para este módulo consiste en gestionar eventos de UI sobre el control ActiveX *Formula One* (`F1Stream1`, `F1Stream2`, `F1BalTot`, `F1Target`) y despachar cálculos termodinámicos a `HSC6.DLL`.
   - Decompilar código VB6 orientado a eventos ActiveX no aporta algoritmos científicos ocultos que no estén ya descritos en el manual y en las firmas de `HSC6.DLL`.
2. **Transparencia de las ecuaciones termodinámicas:**
   - Como se verificó en el Módulo 1 y en las salidas `.OHE` (`BUTANE.OHE`, `H2O.OHE`, `CUSMELT.OHE`, `CUCONV.OHE`), `Heat Content`, `Total H`, $T\cdot S$ y $G$ provienen directamente de la integración de $C_p(T)$ sobre los coeficientes polinómicos de `Maindb6.HSC` (ya implementada y validada en `soryntel-thermo`).
   - Las propiedades de $\text{H}_2\text{O}(T, P)$ (`H_H2O`, `S_H2O`, `CP_H2O`, `G_H2O`, `Critical_H2O`) corresponden al estándar internacional **IAPWS-95 / IAPWS-IF97** para agua y vapor, cuya formulación publicada es exacta y estándar en ingeniería química.
   - Los algoritmos numéricos requeridos (`Temp. Balance` unidimensional, `Target Dialog` acotado, convergencia de ciclos) se implementan de manera superior y verificable en Rust usando métodos estándar (`Brent`, secante/Newton amortiguado, punto fijo acelerado / Broyden) sin necesidad de imitar heurísticas internas de VB6.
3. **Respeto de restricciones del proyecto:**
   - Siguiendo la instrucción maestra (*"No fuerces el uso de Ghidra cuando no sea necesario"* y *"No copies ni reutilices código propietario"*), la combinación de **análisis documental + inspección visual + REA/strings + archivos `.OHE` de referencia** proporciona cobertura completa y verificable.

## 2. Cuándo se reconsideraría Ghidra

Únicamente si durante la fase de validación (`VALIDATION_PLAN.md`) existiera una discrepancia inexplicable mayor a `1%` en el cálculo de **Exergía química estándar** de alguna especie de referencia ambiental en la hoja `BALANCE` que no coincidiera con la tabla estándar de Szargut et al. (1988). En ese caso puntual, se inspeccionaría únicamente la tabla de exergías estándar de referencia en `HSC6.DLL`.
