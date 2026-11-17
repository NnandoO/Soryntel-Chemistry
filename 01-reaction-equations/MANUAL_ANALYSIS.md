# MANUAL_ANALYSIS — Reaction Equations Module (HSC Chemistry 6.0)

## 1. Identificación del Documento de Origen y Versiones Contrastadas
- **Título de origen:** Chapter 10: Reaction Equations Module
- **Autores documentados:** Lauri Mäenpää, Antti Roine (Metso Finland Oy / Outokumpu Research Oy)
- **Fecha de versión del manual PDF:** 5 de septiembre de 2023 (documentación moderna Metso / Outotec, extensión `.rea8`).
- **Archivo Markdown analizado:** `/home/nando/Documentos/Docs HSC/10-reaction-equations/10-reaction-equations.md`
- **Archivo PDF original de referencia:** `/home/nando/Documentos/Docs HSC/10-reaction-equations/10-reaction-equations.pdf` (12 páginas)
- **Documentación local de la instalación HSC 6.0 (2006):** `/home/nando/Descargas/HSC6/Help/10 Reaction Equations.txt`, `08 Introduction.txt`, `28 HSC - Databases.txt` y `32 References.txt`.
- **Diferencia documental clave:** El manual 2023 muestra la interfaz moderna con gráfico integrado a la derecha de la hoja de cálculo (`Y - Axis`, `Collect Data`, `Plot All Data`, `Open in new Window`, `Copy Vector`, `Copy Raster`, formato `.rea8`), mientras que HSC 6.0 (2006) separa la ventana de entrada de la hoja de resultados/gráficos, usa el botón `Peep Database` (equivalente a `Browse...`) y guarda libros en formato binario BIFF2 `.REA`. La lógica termodinámica subyacente es equivalente en ambas versiones.

---

## 2. Estructura y Flujo General del Módulo

El módulo opera bajo dos modos de cálculo basados en la misma ventana de entrada:
1. **Modo Sustancia Individual (*One Chemical Substance*):** Se introduce una única fórmula química (ej. `Fe`, `Na2SO4`, `SO2(g)`, `Cu(+2a)`).
2. **Modo Ecuación de Reacción (*Reaction Equations*):** Se introduce una igualdad química balanceada o por balancear (ej. `Ni + 4CO(g) = Ni(CO)4(g)`, `2Cu(+a) + Cl2(g) = 2Cu(+2a) + 2Cl(-a)`).

### Parámetros de Entrada Configurados por el Usuario:
- **Fórmula / Ecuación:** Campo de texto libre para la expresión química.
- **Rango de Temperatura:**
  - `From (°C / K / °F)`: Límite inferior (ej. por defecto $0\,^\circ\text{C}$ o $25\,^\circ\text{C}$).
  - `To (°C / K / °F)`: Límite superior (ej. $1000\,^\circ\text{C}$ o $300\,^\circ\text{C}$ para iones acuosos).
  - `Step (°C / K / °F)`: Incremento o paso térmico discreto (ej. $100\,^\circ\text{C}$ o $25\,^\circ\text{C}$).
- **Multiplicador estequiométrico (*Smallest Coefficient*):** Factor para normalizar o escalar los coeficientes de la reacción (valor por defecto = 1).
- **Botón `Balance`:** Algoritmo de resolución algebraica basado en la matriz de conservación de elementos químicos.

---

## 3. Opciones y Modos de Cálculo (*Options Menu*)

### 3.1. Formato de Resultados (Afecta únicamente a sustancias individuales)
- **Normal (Escala Absoluta):**
  - Muestra las funciones termodinámicas directas de la especie: $H(T)$, $S(T)$, $C_p(T)$, $G(T)$.
  - Basado en la convención de compilación de **I. Barin, O. Knacke, y O. Kubaschewski** (1973/1977).
  - La entalpía se define con referencia a $H = 0$ para los elementos en su estado de agregación estándar y más estable a $298.15\,\text{K}$ y $1\,\text{bar}$.
- **Delta (Funciones de Formación):**
  - Muestra el cambio de formación de la especie respecto a los elementos que la constituyen a la misma temperatura $T$:
    $$\Delta H_f(T) = H_{\text{species}}(T) - \sum \nu_k H_{\text{element}, k}(T)$$
    $$\Delta S_f(T) = S_{\text{species}}(T) - \sum \nu_k S_{\text{element}, k}(T)$$
    $$\Delta G_f(T) = G_{\text{species}}(T) - \sum \nu_k G_{\text{element}, k}(T)$$
  - Para iones acuosos, se acopla a la escala de ion hidrógeno estándar:
    $$\Delta G_f(T) = G_{\text{ions}}(T) - \sum \nu_k G_{\text{elements}, k}(T) + \frac{z}{2} G_{H_2(g)}(T) - z\, G_{H^+(a)}(T)$$
    donde $z$ es la carga neta del ion.
  - Formato coincidente con las tablas NBS (*National Bureau of Standards*) y compilaciones de Pankratz (USBM).
- *Regla explícita del manual:* Para cálculos de **ecuaciones de reacción**, la opción Delta Format se desactiva/no tiene efecto, ya que las propiedades de una reacción química ($\Delta H_r, \Delta S_r, \Delta G_r$) son, por definición física, deltas.

### 3.2. Mostrar Puntos de Transición de Fase (*Show Transitions*)
- Cuando está activo, el motor termodinámico interpola e inserta filas adicionales en la tabla numérica exactamente en las temperaturas de cambio de fase ($T_{\text{tr}}$) registradas en la base de datos (puntos de transformación polimórfica, temperaturas de fusión $T_m$ y evaporación $T_b$).
- Permite observar el calor latente de transformación ($\Delta H_{\text{tr}}$) y la discontinuidad escalonada en la entropía ($\Delta S_{\text{tr}} = \Delta H_{\text{tr}} / T_{\text{tr}}$) y en la capacidad calorífica $C_p$.

### 3.3. Extrapolación de Criss-Cobble (*Criss-Cobble Option*)
- Aplicable a especies y electrolitos acuosos en disolución diluida ($a$).
- Las bases de datos tradicionales solo reportan propiedades de iones a $25\,^\circ\text{C}$ ($298.15\,\text{K}$).
- Si la opción está habilitada, y la especie acuosa carece de coeficientes polinomiales $B, C, D$ en la base de datos, el módulo efectúa una extrapolación empírica basada en el modelo de Criss y Cobble (1964) hasta un máximo recomendado de $300\,^\circ\text{C}$ (con paso recomendado de $25\,^\circ\text{C}$).
- Realiza internamente una conversión a la "escala absoluta de entropía de iones" restando $5\,\text{cal}/(\text{mol}\cdot\text{K})$ de la escala relativa de ion hidrógeno.

### 3.4. Acumulación y Gráficos Multitabla (*Collect Data / Plot all data*)
- Permite almacenar sucesivos cálculos en diferentes hojas de trabajo dentro del mismo libro de trabajo (*Workbook*).
- La herramienta de gráficos permite comparar múltiples curvas simultáneas en un único eje cartesiano (*Plot all data*), siempre y cuando las hojas compartan las mismas unidades físicas y formato de salida.
- *Restricción explícita:* No es admisible graficar simultáneamente sustancias puras y ecuaciones de reacción en el mismo gráfico por incompatibilidad de escalas y naturalezas de variables.

---

## 4. Salidas Numéricas y Semántica Científica

### Columnas de la Tabla de Resultados (Sustancia Pura — Fig. 1):
1. **$T$:** Temperatura en las unidades seleccionadas ($^\circ\text{C}$, $\text{K}$, $^\circ\text{F}$).
2. **$C_p$:** Capacidad calorífica molar a presión constante ($\text{cal}/(\text{mol}\cdot\text{K})$ o $\text{J}/(\text{mol}\cdot\text{K})$).
3. **$H$ o $\Delta H$:** Entalpía molar ($\text{kcal}/\text{mol}$ o $\text{kJ}/\text{mol}$).
4. **$S$ o $\Delta S$:** Entropía molar ($\text{cal}/(\text{mol}\cdot\text{K})$ o $\text{J}/(\text{mol}\cdot\text{K})$).
5. **$G$ o $\Delta G$:** Energía libre de Gibbs molar ($\text{kcal}/\text{mol}$ o $\text{kJ}/\text{mol}$).
6. **Reference:** Fuente bibliográfica del rango térmico correspondiente (ej. `Barin 77, JANAF 98`, `Belov 99`).

### Columnas de la Tabla de Resultados (Ecuaciones de Reacción — Figs. 5 y 9):
1. **$T$:** Temperatura de reacción.
2. **$\Delta H$:** Entalpía neta de reacción:
   - $\Delta H < 0$: Reacción exotérmica (libera calor).
   - $\Delta H > 0$: Reacción endotérmica (absorbe calor).
3. **$\Delta S$:** Entropía neta de la reacción.
4. **$\Delta G$:** Variación de energía libre de Gibbs estándar de la reacción ($\Delta G^\circ$).
5. **$K$:** Constante termodinámica de equilibrio de masas:
   - $K > 1$ ($\Delta G^\circ < 0$): El equilibrio favorece la formación de productos (desplazamiento a la derecha).
   - $K < 1$ ($\Delta G^\circ > 0$): El equilibrio desfavorece la reacción directa (desplazamiento a la izquierda).
6. **$\log(K)$ o `E vs. SHE (V)`:**
   - En reacciones no electroquímicas (Fig. 5): muestra $\log_{10} K$.
   - En semirreacciones con electrones (`e-`, Fig. 9): muestra el potencial estándar de electrodo $E^\circ$ (V) versus SHE siguiendo estrictamente la convención de signos de la IUPAC.

### Tabla Inferior de Balances de Masa y Pesos Fórmula (`Species Data`):
- Muestra para reactivos y productos (separados por línea divisoria):
  - `Formula`: Fórmula química de la especie.
  - `M (g/mol)` (o `FM` en HSC 6): Masa molar / peso fórmula.
  - `Conc. (wt-%)`: Porcentaje en masa dentro de su lado de la reacción (reactivos suman 100%, productos suman 100%).
  - `Input Amounts`: `mol`, `g`, `Volume`, `Unit` (`ml` o `l`, usando densidad en condensados y volumen molar ideal $22.414\,\text{l}/\text{mol}$ a CNTP para gases).
  - `Extrapolated From T(K)`: Temperatura límite desde la cual se extrapola si $T$ excede el rango de la base de datos (o `-` si no hay extrapolación).

---

## 5. Integración con Bases de Datos (`OwnDB` vs `MainDB`)
1. **Jerarquía estricta de búsqueda:** El programa busca primero en la base de datos propia del usuario (`OwnDB6.HSC`). Si la especie no existe allí, recurre a la base de datos principal del sistema (`MainDB6.HSC`).
2. **Resolución de conflictos:** Si existe un registro con idéntico nombre y estado de fase en ambas bases, `OwnDB` sobrescribe con prioridad absoluta a `MainDB`.
3. **Explorador de Datos (*Database Browser*):** Permite inspeccionar las constantes termodinámicas básicas ($H_f(298.15)$, $S^\circ(298.15)$, coeficientes $A, B, C, D$, rangos de temperatura $T_1 - T_2$) pulsando el botón `Browse...`.
