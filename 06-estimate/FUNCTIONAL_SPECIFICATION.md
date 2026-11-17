# Especificación Funcional — Módulo 06: HSC Estimate

## 1. Propósito del Módulo
Estimar propiedades termodinámicas estándar ($\Delta_f H^\circ_{298}$, $S^\circ_{298}$) y función de capacidad calorífica $C_p(T)$ (coeficientes de Kelley $A, B, C, D$) para especies químicas que no figuran en `Maindb6` o cuyos datos experimentales están incompletos.

## 2. Flujos Funcionales Principales
1. **Entrada de Especie Objetivo:**
   - Ingreso de fórmula química (ej. `MgAl2O4`, `Fe2SiO4`, `CuFeS2`, `C2H5OH(g)`), nombre y fase (`Solid`, `Liquid`, `Gas`).
2. **Descomposición Automática / Manual:**
   - **Por Elementos (Neumann-Kopp):** parseo automático de la fórmula con `parse_term`.
   - **Por Componentes Base de `Maindb6`:** permitir definir la reacción de combinación (ej. `1 MgO + 1 Al2O3 -> MgAl2O4`) buscando los componentes directamente en `Maindb6`.
   - **Por Grupos Funcionales (Orgánicos):** selección de grupos Benson/Joback cuando se trata de compuestos orgánicos.
3. **Selección de Método de Estimación:**
   - Neumann-Kopp (elementos / componentes) para $C_p(T)$.
   - Latimer / Aditividad para $S^\circ_{298}$.
   - Aditividad de óxidos / Hisham-Benson / Pauling para $\Delta_f H^\circ_{298}$.
4. **Cálculo y Comparación:**
   - Calcular $\Delta_f H^\circ_{298}$, $S^\circ_{298}$, $\Delta_f G^\circ_{298}$, tabla $C_p(T), H(T), S(T), G(T)$ de `298.15 K` a `T_max` y coeficientes Kelley `A, B, C, D`.
   - Si la especie existe en `Maindb6`, mostrar lado a lado **Estimado vs. Experimental (`Maindb6`)** y el `% de error`.
5. **Exportación:**
   - Copiar tabla / coeficientes al portapapeles o guardar la especie estimada en la sesión de usuario (`ThermoDatabase`).
