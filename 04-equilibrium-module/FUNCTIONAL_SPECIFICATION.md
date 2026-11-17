# Especificación Funcional — Módulo 4: Equilibrium Calculations

## 1. Casos de Uso Principales

1. **Definición de Sistema Químico:**
   - **RF-01:** Cargar especies desde `Maindb6.HSC` filtrando por conjunto de elementos (`OR` / `AND`), estado físico (`Gas`, `Aqua`, `Liquid`, `Solid`), iones y especies orgánicas, o agregar especies manualmente por fórmula.
   - **RF-02:** Agrupar especies en **Fases** (`Mixture`, `Pure`, `Aqua`, `Ga-As Mixture`, `Al-Zn Mixture`), garantizando la regla de fase gaseosa primera (si existe) y presencia de `H2O` en fase `Aqua`.
   - **RF-03:** Editar cantidades iniciales en `kmol`, `kg`, `Nm3` (para gases a STP `22.414 L/mol`) o `g/l`, `mol/l` (si `Allow titrations` está activo), con conversión automática a moles internos (`mol` / `kmol`) y cálculo de `%` por fase.

2. **Modos de Equilibrio Termodinámico:**
   - **RF-04 (`N,T,P` Normal):** Minimización de Gibbs a temperatura $T$, presión $P$ y composición elemental $b$ fijas.
   - **RF-05 (`N,H,P` Adiabático):** Dada la entalpía total de entrada $H_{\text{in}} = \sum n_i^{\text{in}} H_i(T_{\text{feed},i}) + \Delta H_{\text{extra}}$, encontrar $T_{\text{eq}} \in [T_{\min}, 20000\;\text{K}]$ tal que $H_{\text{eq}}(T_{\text{eq}}, n^*(T_{\text{eq}})) = H_{\text{in}}$.
   - **RF-06 (`N,P,V` Volumen Constante / Presión Constante):** Encontrar $T_{\text{eq}}$ o $P_{\text{eq}}$ consistente con el volumen especificado $V$ (usando ecuación de gas ideal $P V_{\text{gas}} = n_{\text{gas}} R T$ más volumen molar de fases condensadas).
   - **RF-07 (`Target Calculations`):** Resolver inversamente el parámetro de barrido (p. ej. $T$ o `X-amount`) para igualar una propiedad objetivo (cantidad de especie, fracción molar, pH, etc.).
   - **RF-08 (`Cell Mode` — `N,T,P,Q`):** Simular celdas electroquímicas con fases de electrodo (`anode`/`cathode`), capacitancia eléctrica ($F$), ecuación de descarga automática y cálculo de voltaje de celda $E_{\text{cell}}$ vs nivel de descarga $Q$ (`kmol e-`).

3. **Barridos Paramétricos (0, 1 y 2 Variables Independientes):**
   - **RF-09:** `0` variables = punto único; `1` variable = serie de `Steps` puntos (escala lineal o logarítmica); `2` variables = malla `Steps_1 × Steps_2` para superficies 3D.
   - **RF-10:** Soportar `Step Mode = Add Amount` (cantidad base $+ \; X$) y `Final Amount` (interpolación entre inicial y final), así como `Remove %` entre pasos.

4. **Opciones Avanzadas y Estabilidad:**
   - **RF-11 (`Infinite Phase`):** Tratar la fase seleccionada como reservorio infinito (potenciales químicos fijados por su composición inicial, simulando atmósfera abierta o baño infinito).
   - **RF-12 (`Activity Coefficient`):** Evaluar expresiones definidas en Columna C (`T`, `X_i`) y modelos integrados (`Ideal`, `Aqua`/Pitzer simplificado, `Ga-As`, `Al-Zn`) mediante iteraciones externas `AC Steps`.
   - **RF-13 (`Warnings`):** Detectar si el rango de cálculo excede `[Tmin, Tmax]` de alguna especie en `Maindb6.HSC` y permitir su exclusión antes de resolver.
   - **RF-14 (`Results` & `Chart`):** Generar tabla completa de resultados (`Input`, `Equil Amount`, `Mole Fract`, `Activity`, `Activity Coeff`, `Gmin`, `H`, `S`, `V`, `pH`, `Cell Voltage`) y exportar a CSV/JSON.

## 2. Manejo de Errores y Casos Borde
- Sistema sin especies con cantidad inicial $> 0$: rechazar con mensaje claro antes de invocar al solver.
- Elemento presente en especie producto pero con balance total $b_e = 0$: fijar automáticamente sus especies a $0$ o advertir al usuario.
- Sistema puramente condensado sin fase gaseosa: ofrecer añadir traza de `N2(g)` (`1e-10 kmol`) para estabilidad si el usuario lo solicita.
- No convergencia en iteraciones Newton/WJD o bisección adiabática: reportar residuo máximo de balance atómico $\max |An - b|$ y número de iteraciones alcanzadas.
