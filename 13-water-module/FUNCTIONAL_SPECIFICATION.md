# Especificación Funcional — Módulo 13: Water Module (`PTCalc`)

## 1. Casos de Uso

### CU-01: Calcular Propiedades en Dos Puntos de Proceso (`Point 1` y `Point 2`)
- **Actor:** Ingeniero de procesos / investigador.
- **Flujo principal:**
  1. Selecciona las unidades globales en la barra/combos de la hoja izquierda: Temperatura (`°C` o `K`), Presión (`bar` o `MPa`), Entalpía/Gibbs (`kJ`, `kcal`, `Wh`, `kWh`, `MWh`), Entropía/Cp (`J`, `kJ`, `cal`, `kcal`, `kWh`, `MWh` por `°C` o `K`), Densidad (`kg/m3` o `g/cm3`).
  2. Ingresa la cantidad de flujo/masa (`Amount`, p. ej. `1` o `10` o `500000`) y su base (`kg` o `kmol`).
  3. Especifica para `Point 1` y `Point 2` la Temperatura, Presión y Fase (`Solid (s)`, `Liquid (l)`, `Gas (g)`, `Sat. (g,l)`, `Sat. (g,s)`, `Sat. (l,s)` o selección automática por zona).
  4. El sistema calcula instantáneamente `Enthalpy`, `Entropy`, `H specific`, `Gibbs Energy`, `Cp` y `Density` para ambos puntos, además de las diferencias en la columna amarilla `Difference`.

### CU-02: Estados Saturados y Regla de Mezcla (`Mixture %`)
- Cuando el usuario elige `Sat. (g,l)`, `Sat. (g,s)` o `Sat. (l,s)` en un punto:
  - El campo `Mixture %` (por defecto `50 %`, porcentaje de la primera fase del par) se vuelve editable.
  - En `Sat. (g,l)` o `Sat. (g,s)`, el usuario edita `Temperature` y `Pressure` se calcula automáticamente (fuente negra). Si cambia `Pressure` manualmente dentro del rango de la curva, se resuelve $T_{\text{sat}}(p)$.
  - En `Sat. (l,s)`, el usuario edita `Pressure` y `Temperature` de fusión se recalcula automáticamente (fuente negra).
  - Si se mueve fuera del límite de saturación ($T > T_c$ o $p < p_t$), la fase cambia automáticamente a la fase pura correspondiente.

### CU-03: Interacción Gráfica sobre el Diagrama $p\text{-}T$
- El usuario hace clic sobre el recuadro del punto deseado (`Point 1` o `Point 2`) para activarlo (se resalta en amarillo; el inactivo permanece en blanco).
- Al hacer clic o **arrastrar** sobre el diagrama $p\text{-}T$, el punto activo actualiza $(T, p)$ en tiempo real; si el punto está configurado en modo saturado, se proyecta (`snap`) automáticamente sobre la curva de saturación correspondiente.
- La opción `Show -> Line` dibuja una línea recta visual conectando `Point 1` y `Point 2`.

### CU-04: Presets, Isentropía y Copiado / Exportación
- `Edit -> Point 2 = Triple Point`: fija instantáneamente `Point 2` en $0.01\,^\circ\text{C}$, $0.006117\text{ bar}$, fase `Liquid (l)` para equivalencia directa con Steam Tables.
- Botón/herramienta opcional `Find Isentropic Mixture %` / calculadora $\eta_s$: permite estimar el estado ideal con $\Delta S = 0$ (útil para turbinas y compresores como en los ejemplos `25.2.2` y `25.2.3`).
- `File -> Copy Table / Export CSV`: copia o guarda la tabla completa de resultados y unidades.

## 2. Reglas de Validación y Límites
- Temperatura acotada al rango operativo `[-200 °C, 2000 °C]` (`73.15 K..2273.15 K`).
- Presión acotada al rango `[0.001 bar, 1000 bar]` (`0.0001 MPa..100 MPa`).
- `Mixture %` acotado estrictamente a `[0.0 %, 100.0 %]`.
- `Amount > 0`; valores no positivos muestran advertencia sin generar `NaN` ni divisiones por cero.
- Advertencia informativa cuando el punto entra en región extrapolada (`800–2000 °C` a `100–1000 bar` o `0.001–0.01 bar`) o en las inmediaciones del Punto Crítico (`|T - Tc| < 2 K` y `|p - pc| < 2 bar`), donde $C_p$ y $\partial\rho/\partial p$ presentan picos pronunciados.
