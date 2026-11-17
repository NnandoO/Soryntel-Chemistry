# Especificación Funcional — Módulo 2: Heat and Material Balance

## 1. Alcance Funcional

El Módulo 2 permite construir balances estacionarios de materia, energía y exergía para procesos químicos, metalúrgicos y energéticos compuestos por **una o múltiples áreas de balance** conectadas entre sí.

---

## 2. Entidades del Modelo de Datos

1. **BalanceWorkbook (Proyecto de Balance):**
   - Unidades activas: `TemperatureUnit` (`°C` o `K`) y `EnergyUnit` (`Mcal`, `MJ`, `kWh`).
   - Lista ordenada de `BalanceArea` ($1 \dots N$).
   - Hojas auxiliares opcionales (`FEED`, `Analysis`, `Process flow`, etc.).
   - Tabla `TargetSheet` (lista de reglas de búsqueda de objetivo).
   - Configuración de gráficos `DiagramConfig`.

2. **BalanceArea ($k = 1 \dots N$):**
   - Hoja `IN{k}` (entradas de materia y `EXTRA HEAT`).
   - Hoja `OUT{k}` (salidas de materia y `HEAT LOSSES`).
   - Composición elemental calculada `In{k}-%` y `Out{k}-%`.
   - Resumen en hoja `BALANCE` (totales `IN{k}`, `OUT{k}`, `BALANCE{k}`).

3. **Stream (Corriente):**
   - Nombre (ej. `GAS:`, `MATTE:`, `SLAG:`), temperatura de corriente opcional.
   - Lista de `SpeciesRow` pertenecientes a la corriente.
   - Fila de total automático que agrega `kmol`, `kg`, `Nm³`, `Heat Content`, `Total H`.

4. **SpeciesRow:**
   - `formula: String` (ej. `Cu2S(l)`, `*2FeO*SiO2(l)`, `H2O(g)`).
   - `temperature`: valor numérico o fórmula (ej. `=B2`).
   - `pressure_bar`: por defecto `1.0` (o vacío = `1.01325`/`1.0` atm estándar según flag `pressure_correction_h2o`).
   - `amount_kmol`: editable directamente o mediante fórmula, o derivado al editar `kg` o `Nm³`.
   - Columnas calculadas de solo lectura: `Amount kg`, `Amount Nm³`, `Heat Content`, `Total H`, `Heat Cont / kmol`, `Tot H / kmol`, `MW`.

5. **HeatFlowRow:**
   - En `IN{k}`: `EXTRA HEAT:` (suma positivamente a la energía de entrada).
   - En `OUT{k}`: `HEAT LOSSES:` (suma a la energía de salida).
   - Soporta valor directo o fórmula (ej. ecuación de convección natural).

---

## 3. Reglas de Comportamiento y Validaciones

### 3.1 Edición de Cantidades (`kmol` ↔ `kg` ↔ `Nm³`)
- Si el usuario edita **`Amount kmol`** ($n$):
  - $m = n \cdot MW$ (`kg`)
  - $V = n \cdot V_{\text{molar}}$ (`Nm³`): para gases `(g)` se usa $22.414\text{ Nm}^3/\text{kmol}$ (condiciones normales $0\,^\circ\text{C}$, $1\text{ atm}$); para fases condensadas se usa $MW / \rho$ si hay densidad disponible o volumen molar aproximado/cero según reporte HSC (en `FSF_process.bal8` `raw-029.png` las especies sólidas/líquidas muestran volumen pequeño no nulo basado en densidad molar si está disponible, y en `.OHE` solo gases muestran `Nm3` en `kg/Nm3`).
- Si el usuario edita **`Amount kg`** ($m$): recalcula $n = m / MW$.
- Si el usuario edita **`Amount Nm³`** ($V$) en un gas: recalcula $n = V / 22.414$.
- Si el usuario edita la **fila total de un `Stream`** (ej. duplicar el caudal total de `GAS:` de `100` a `150 Nm³`): escala proporcionalmente todas las `SpeciesRow` no bloqueadas por fórmula dentro de esa corriente.

### 3.2 Evaluador de Fórmulas y Nombres Elementales
El motor debe soportar sintaxis compatible con hojas de cálculo para fórmulas en celdas:
- Operaciones aritméticas: `+`, `-`, `*`, `/`, `^`, paréntesis.
- Funciones básicas: `SUM(...)`, `AVERAGE(...)`, `MIN(...)`, `MAX(...)`, `IF(...)`, `ABS(...)`.
- Referencias entre celdas y hojas: `IN1!D7`, `'OUT4'!D12`, `Analysis!L28`, `FEED!C11`.
- **Variables elementales automáticas:**
  - `In<El>` / `Out<El>` para el área 1 (ej. `InCu`, `OutFe`, `InS`, `InO`).
  - `In<k><El>` / `Out<k><El>` o `In<El><k>` (nota: en Fig. 12 `raw-032.png` se observa `=InCa2*Analysis!L28/100` donde `InCa2` significa entrada de `Ca` en el Área 2!). El parser de nombres debe reconocer tanto `In<El><k>` como `In<k><El>`.
- **Ciclos y referencias circulares:**
  - Al conectar corrientes de retorno (`OUT4 -> IN1`, `OUT5 -> IN1`, o `=0.938*(InCu-D13)/2`), el grafo puede contener ciclos. El evaluador debe iterar automáticamente hasta alcanzar convergencia ($|\Delta x|_\infty < 10^{-8}$) o reportar no-convergencia.

### 3.3 Funciones Especiales del Módulo
1. **`Element Balance`:**
   - Para cada elemento químico presente en `IN{k}` u `OUT{k}`, descompone cada fórmula mediante `parse_formula_composition` y suma:
     $$N_{e,\text{IN}}^{(k)} = \sum_{i \in \text{IN}_k} n_i \cdot \nu_{i,e}, \qquad N_{e,\text{OUT}}^{(k)} = \sum_{j \in \text{OUT}_k} n_j \cdot \nu_{j,e}$$
     $$\Delta N_e^{(k)} = N_{e,\text{IN}}^{(k)} - N_{e,\text{OUT}}^{(k)} \quad (\text{o } \text{IN} - \text{OUT} \text{ según vista})$$
   - Nota sobre convención en Fig. 1 (`raw-001.png`) y `Total Balance`: en `Total Balance` (`OUT - IN`), `Amount kmol = -0.807` (porque salen menos kmol gaseosos de los que entran), `Heat Content = +194.699 MJ`, `Total H = -81.326 MJ` (calor liberado). En `.OHE`, `DIFF. = INPUT - OUTPUT`. La especificación documenta explícitamente ambas vistas y mantiene `OUT - IN` en la grilla `BALANCE` / `Total Balance` tal como muestran `raw-001.png` y `raw-027.png`.

2. **`Temp. Balance` (Temperatura Adiabática de Salida):**
   - Toma la hoja `OUT{k}` del área activa y varía uniformemente la temperatura $T_{\text{out}}$ de todas las especies de salida hasta que:
     $$\Delta H_{\text{total}}(T_{\text{out}}) = \sum_{\text{OUT}_k} H_{\text{tot}}(T_{\text{out}}) + Q_{\text{losses}} - \left(\sum_{\text{IN}_k} H_{\text{tot}} + Q_{\text{extra}}\right) = 0$$
   - Muestra el resultado en el diálogo modal sin sobrescribir automáticamente las temperaturas originales (a menos que el usuario elija aplicarlo).

3. **`Target Dialog` (Goal Seek Multi-fila):**
   - Cada fila $r$ define: celda variable $x_r \in [\text{Min}_r, \text{Max}_r]$, celda objetivo $f_r(x_r)$, valor meta $y_r^*$, y máximo de iteraciones $M_r$.
   - `Iterate Selected Rows`: resuelve las filas seleccionadas en orden secuencial.
   - `Iterate All` (`F8`): ejecuta todas las filas activas en bucle externo hasta que todos los objetivos converjan simultáneamente dentro de tolerancia (permite resolver sistemas acoplados como el ejemplo de `CUCONV2.bal8` en `raw-018.png`).
   - Restricción documentada: si la variable es temperatura, debe apuntar a la celda de temperatura del encabezado de un `Stream` para propagar la temperatura a todas sus especies.

4. **`Diagram Settings` & `Diagram`:**
   - Barre $x \in [x_{\min}, x_{\max}]$ con paso $\Delta x$.
   - Si `Target iteration` está habilitado con `Target row 1..N`, ejecuta `Target` en cada punto $x_m$ antes de leer `Y1..Y9`.
   - Renderiza gráfica interactiva XY y permite exportar tabla de puntos (`.csv`) e imagen (`.svg`/`.png`).

5. **`Summary` (`Inx-%` / `Outx-%`):**
   - Calcula por cada corriente la suma de moles y masas de cada elemento químico, junto con `mol-%` y `wt-%` dentro de la corriente.
