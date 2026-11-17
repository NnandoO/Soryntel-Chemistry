# Especificación Funcional (`FUNCTIONAL_SPECIFICATION.md`) — Módulo 3

## 1. Propósito del Módulo

Calcular pérdidas o ganancias de calor estacionarias unidimensionales (1D) y perfiles de temperatura a través de envolventes multicapa (paredes planas, cubos, cilindros y esferas) sometidas a conducción, convección libre/forzada y radiación superficial, gaseosa o particulada, con soporte para análisis paramétrico y dimensionamiento inverso (*Target*).

---

## 2. Entidades Funcionales Principales

### 2.1 Libro de Trabajo (`HeatLossWorkbook`) y Hojas (`HeatLossSheet`)
- Un libro contiene $1 \dots M$ hojas independientes (por ejemplo: `Wall`, `Roof`, `Bottom` para un tanque o reactor).
- Cada hoja posee:
  - Una geometría (`Wall`, `Cube`, `Cylinder`, `Sphere`) con sus **dimensiones internas**.
  - Una secuencia ordenada de $1 \dots N$ columnas (`HeatLossColumn`).
  - Un estado de cálculo (`HeatFlow` vs. `TemperatureProfile`) y resultado de flujo térmico de hoja $Q_{\text{sheet}}$.
- El flujo térmico total del libro es $Q_{\text{total}} = \sum_{j=1}^{M} Q_{\text{sheet}, j}$ cuando se ejecuta `Calculate all sheets`, o $Q_{\text{active}}$ en `Calculate active sheet`.

### 2.2 Tipos de Columna (`ColumnType`)
1. **`Surface`:** Representa la resistencia térmica combinada de convección + radiación en una frontera (interna o externa).
   - Entradas: $h_c$ (fijo o calculado por `HeatConv`), $h_r$ (fijo o calculado por `HeatRadi` / `GasRad` / `ParticleRad`), temperaturas de borde $T_L, T_R$.
   - Resistencia térmica equivalente:
     $$R_{\text{surf}} = \frac{1}{(h_c + h_r)\, A_{\text{surf}}}$$
2. **`SolidLayer` (`Layer`):** Representa una capa sólida homogénea de espesor $\Delta x > 0$.
   - Entradas: material sólido (tabla $k(T)$), espesor $\Delta x$, número de nodos de discretización $m$ (`Calculation Grid`, por defecto 10).
   - Calcula conductividad efectiva integrando $k(T)$ sobre $m$ subcapas y resistencia geométrica según `Wall`/`Cube`/`Cylinder`/`Sphere`.
3. **`Enclosure`:** Representa una cavidad cerrada (aire/gas confinado) entre dos paredes de espesor $\Delta x$.
   - Combina convección natural interna en cavidad ($h_{c,\text{enc}}$) y radiación entre las dos superficies enfrentadas ($h_{r,\text{enc}}$).
4. **`LayerContact`:** Modela resistencia térmica de contacto imperfecto entre dos capas sólidas adyacentes (espesor geométrico 0, resistencia térmica $R_c$ fijada directamente por el usuario).

---

## 3. Modos de Operación y Reglas de Validación

### 3.1 Modo `Heat flow`
- **Precondición:** Existen exactamente **2 temperaturas fijas** en la hoja (por defecto, borde izquierdo de la columna 1 y borde derecho de la columna $N$; también admite un par interno seleccionado).
- **Proceso:** Resuelve iterativamente el flujo térmico $Q$ y las temperaturas de interfaz intermedias hasta satisfacer continuidad térmica ($T_{R,i} = T_{L,i+1}$) y consistencia de propiedades dependientes de la temperatura ($k(T), h_c(T_s, T_\infty), h_r(T_s, T_\infty)$).
- **Salida:** $Q$, $q''_i$, $R_i$, $A_i$, $k_i$, $h_{c,i}$, $h_{r,i}$ y todas las temperaturas intermedias.

### 3.2 Modo `Temperature profile`
- **Precondición:** Existe **1 temperatura fija** en la hoja y un valor positivo/conocido de **`Total Heat Flow`** (o flujo de hoja).
- **Proceso:** Propaga la temperatura capa por capa desde la frontera fija usando $Q$ fijado y actualizando iterativamente $k(T), h_c(T), h_r(T)$ en cada columna.
- **Salida:** Perfil completo $T_0, T_1, \dots, T_N$ y resistencias térmicas consistentes con $Q$.

### 3.3 Validaciones y Manejo de Errores (observados en manual y `HeatLoss.exe`)
- Emisividad superficial fuera de rango: requiere $\varepsilon_s \in [0, 1]$ (`"Please enter a surface emissivity value between 0 and 1."`).
- Propiedades faltantes: si $k$ no está fijado ni seleccionado desde base de datos en un `SolidLayer`, o si $h_c$ y $h_r$ faltan en una `Surface`, se notifica el número de columna exacto.
- Sub-malla insuficiente para gradientes fuertes de $k(T)$: advierte aumentar `Calculation Grid` (fila 11/13).
- Límites térmicos de correlación/base de datos: alerta si $T$ excede $[T_{\min}, T_{\max}]$ del material/fluido.
- Convergencia: límite máximo de iteraciones con reporte claro si no converge o si el rango de búsqueda en `Target` es demasiado estrecho.

---

## 4. Submódulos Funcionales

1. **Base de Datos de Conducción (`HeatCond`):** Búsqueda de sólidos, consulta de tabla $k(T)$ (y $\rho$), interpolación lineal por tramos.
2. **Calculadora de Convección (`HeatConv`):**
   - Selección de fluido (`Air`, `Water`, gases industriales, metales líquidos, etc.).
   - Régimen `Free` o `Forced`, geometría (`Vertical wall`, `Horizontal plate heated/cooled`, `Cube`, `Vertical cylinder`, `Horizontal cylinder`, `Sphere`, `Vertical/Horizontal/Concentric enclosure`), velocidad $u_\infty$.
   - Modo `Function`: evaluación de $h_c(v) = \sum_j c_j v^{p_j}$.
3. **Calculadora de Radiación Superficial (`HeatRadi`):** Emisividades `(n)` / `(h)` + cálculo de $h_r$ (superficie hacia ambiente o entre dos superficies en `Enclosure`).
4. **Calculadora de Radiación de Gases (`GasRad`):** Composición molar de $\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$, presión total $p$, longitud óptica $L$ (manual o desde Tabla 1 geométrica), $T_g, T_s, \varepsilon_s$. Soporta cálculo puntual (`Fixed T`) o generación de tabla interpolable $0\text{–}3000\,^\circ\text{C}$ cada $50\,^\circ\text{C}$ (`Iterative`).
5. **Calculadora de Radiación de Partículas (`ParticleRad`):** Emisividad $\varepsilon_c$ de nube de partículas (monodispersa o distribución $d_i, n_i$) y acoplamiento gas + partícula ($\varepsilon_{g+c}, \alpha_{g+c}$).
6. **Target Solver:** Búsqueda de raíz 1D ($f(x) - y_{\text{target}} = 0$) sobre cualquier parámetro geométrico o de entrada (p.ej., ancho de radiador, espesor de aislante) dentro de `[min, max]`.
7. **Diagram Generator:**
   - **Perfil espacial de temperatura:** $T(x)$ acumulado a través de las capas (`Wall`, `Roof`, `Bottom`).
   - **Barrido paramétrico 1D:** Variación de temperatura de proceso, velocidad de aire o espesor de aislamiento vs. $Q$, temperaturas de superficie externa o costo total anualizado (aislamiento óptimo).

---

## 5. Integración con el Módulo 2 (`Heat & Material Balance`)

- En el Módulo 2 (`soryntel_thermo::balance_engine`), un `BalanceArea` admite filas de `Heat Losses` (`HEAT LOSSES:`) en la hoja `OUT` (o `EXTRA HEAT:`).
- El Módulo 3 exporta el `Total Heat Flow` calculado (convertido automáticamente a la unidad energética configurada en el Módulo 2: `MJ/h`, `kW`, `Mcal/h`, `kWh/h`, teniendo en cuenta la base temporal del balance), eliminando la transcripción manual.
