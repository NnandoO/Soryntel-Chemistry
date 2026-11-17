# Especificación Funcional — Módulo 14: Units Module

## 1. Casos de Uso

### CU-01: Conversión Bidireccional Instantánea (`Convert` Sheet)
- **Actor:** Ingeniero de procesos / termoquímico / estudiante.
- **Flujo principal:**
  1. En la hoja `Convert`, el usuario selecciona una **Magnitud** (*Quantity*) de la lista de ~90 magnitudes (o mediante cuadro de filtro rápido por texto, p. ej. `"Energy"`, `"Pressure"`, `"Viscosity"`, `"Molar enthalpy"`).
  2. Selecciona **Unidad 1** y **Prefijo 1** en el bloque izquierdo (p. ej. `joule` + `k` = `kJ`).
  3. Selecciona **Unidad 2** y **Prefijo 2** en el bloque derecho (p. ej. `calorie (th)` + `k` = `kcal(th)`).
  4. Ingresa un valor numérico en cualquiera de los dos cuadros de entrada (`Value 1` o `Value 2`).
  5. El sistema recalcula instantáneamente el cuadro opuesto sin necesidad de botón "Convertir", mostrando además el factor neto combinado (`1 [Unit 1] = K [Unit 2]`) y la expresión dimensional SI de la magnitud (`m2 kg s-2`).
- **Acciones rápidas:**
  - `Swap ↔`: intercambia unidad y prefijo izquierdo $\leftrightarrow$ derecho manteniendo o reconvirtiendo el valor activo.
  - `Copy Result` y `ΔT Mode` (solo cuando la magnitud es `Temperature`, para elegir entre escala absoluta y diferencia de temperatura).
  - Tabla inferior opcional **"All Units in Selected Quantity"**: permite ver de un vistazo el valor ingresado expresado simultáneamente en *todas* las unidades de esa magnitud.

### CU-02: Gestión y Edición de la Base de Datos de Unidades (`Units` Sheet)
- El usuario abre la pestaña **`Units`**, que muestra las **90 magnitudes** en bloques de **3 filas por magnitud** (`Unit Name / Symbol / Coefficient to Base`, estilo `Fig. 2` de HSC6).
- **Operaciones soportadas:**
  - **Añadir nueva unidad** a una magnitud existente (nombre, símbolo y coeficiente $c_u$ respecto a la unidad base de esa magnitud).
  - **Añadir nueva magnitud completa** (fila nombre + fila símbolo + fila unidad base SI `1.0`).
  - **Editar coeficientes o etiquetas** directamente en la hoja tipo Excel, reflejándose inmediatamente en el selector de la hoja `Convert`.
  - **Restaurar predeterminados (`Reset Default Database`)**.

### CU-03: Consulta y Cálculo en Hojas Científicas y de Ingeniería (`Constants`, `Mesh`, `Air`, `H2O`)
- **`Constants`:** búsqueda filtrada entre las **56 constantes físico-químicas**, copiado con un clic del valor exacto o símbolo para pegado directo en otros cálculos.
- **`Mesh`:** vista de la tabla estándar de tamices (`mesh ↔ mm`) más **calculadora interpoladora rápida** (ingresar `mesh` → obtener apertura en `mm` y `µm`, o ingresar apertura en `mm`/`µm` → obtener número de `mesh` equivalente).
- **`Air`:** vista tabular de composición del aire seco (`N2, O2, Ar, CO2, ...` en `Vol-%` y `Wt-%`), tabla de contenido de agua (`vol-%` y `g/Nm3` para `T = -50..102 °C` y `RH = 10..100 %`), más **calculadora psicrométrica puntual** para cualquier `(T °C, RH %, P bar)`.
- **`H2O`:** tabla de presión de vapor de saturación de agua `-90..370 °C` en `kN/m2`, `atm` y `bar` más buscador/evaluador puntual integrado con `soryntel_thermo::water`.

### CU-04: Persistencia y Exportación (`File | Edit | Format | Help`)
- `File -> Save Workbook` / `Load Workbook` (JSON de `soryntel` con todas las personalizaciones de `Units`, `Constants`, `Mesh`, `Air` y `H2O`).
- `File -> Export Active Sheet CSV...` y `Edit -> Copy Cell / Copy Row / Copy Table`.
- `Format -> Decimals / Scientific Notation Threshold / Reset Column Widths`.

## 2. Reglas de Validación y Límites
- Coeficientes de conversión lineal estrictamente positivos ($c_u > 0$, finitos). Coeficientes $\le 0$ o no numéricos se rechazan con indicación en barra de estado.
- Temperaturas por debajo del cero absoluto ($T_K < 0\text{ K}$, i.e., $< -273.15\,^\circ\text{C}$) generan advertencia física en modo absoluto (permitido solo si está activado modo diferencial $\Delta T$).
- Valores `NaN` o infinitos en entrada numérica no sobreescriben el estado previo ni corrompen las vistas tabulares.
- En calculadora `Air`: $0\% \le \text{RH} \le 100\%$, $T \in [-100\,^\circ\text{C}, 373\,^\circ\text{C}]$, $P > 0$.
