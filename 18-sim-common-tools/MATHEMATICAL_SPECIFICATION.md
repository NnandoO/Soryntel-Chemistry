# Especificación Matemática — Módulo 18: Sim Common Tools

Las métricas globales de convergencia (`MB` y `EB`), las distribuciones estadísticas de Monte Carlo y las reglas de seguridad numérica proceden directamente de las secciones `40.2–40.4` de [40-sim-common-tools.md](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/40-sim-common-tools.md) y de las capturas oficiales (`Figs. 29–37`, `68`, `70`). Donde el manual presenta únicamente la interfaz, el método nominal (`Tangential (fast)`, `PID`, `Order`) o parámetros estructurales sin imprimir la ecuación interna propietaria, se documenta explícitamente la **formulación matemática clean-room equivalente**.

---

## 1. Criterios de Convergencia Global del Diagrama (`Calculate -> Static -> Criteria`)

Durante el cálculo iterativo secuencial modular (`Run Until Convergence`), el simulador supervisa en cada ronda $k$ el cierre de los balances globales de **masa** y **entalpía** sobre el límite completo del proceso (*system boundary*), comparando la suma de corrientes externas de entrada frente a las de salida (pp. 23–24 del manual oficial).

### 1.1. Error de Balance de Masa (`MB`)

$$MB^{(k)} = \left| \frac{\sum_{s \in \mathcal{S}_{\text{in}}} \dot{m}_s^{(k)} - \sum_{s \in \mathcal{S}_{\text{out}}} \dot{m}_s^{(k)}}{\sum_{s \in \mathcal{S}_{\text{in}}} \dot{m}_s^{(k)}} \right| \times 100\% \qquad (1)$$

donde:
- $\mathcal{S}_{\text{in}}$ es el conjunto de corrientes de alimentación externas conectadas al diagrama (corrientes sin unidad de origen dentro del proyecto).
- $\mathcal{S}_{\text{out}}$ es el conjunto de corrientes de producto externas (corrientes sin unidad de destino).
- $\dot{m}_s^{(k)}$ es el flujo másico total de la corriente $s$ tras evaluar la ronda $k$.

### 1.2. Error de Balance de Entalpía (`EB`)

$$EB^{(k)} = \left| \frac{\sum_{s \in \mathcal{S}_{\text{in}}} \dot{H}_s^{(k)} + \sum_{u \in \mathcal{U}} \dot{Q}_{\text{in}, u}^{(k)} - \left(\sum_{s \in \mathcal{S}_{\text{out}}} \dot{H}_s^{(k)} + \sum_{u \in \mathcal{U}} \dot{Q}_{\text{out}, u}^{(k)}\right)}{\sum_{s \in \mathcal{S}_{\text{in}}} \dot{H}_s^{(k)} + \sum_{u \in \mathcal{U}} \dot{Q}_{\text{in}, u}^{(k)}} \right| \times 100\% \qquad (2)$$

> **Nota del manual (`p. 24`):** En su forma básica mostrada en la sección `Static`, $EB = \left|\frac{\text{Total enthalpy in} - \text{Total enthalpy out}}{\text{Total enthalpy in}}\right| \times 100\%$. Para que $EB \to 0$ sea físicamente alcanzable, los flujos netos de calor o pérdidas térmicas de las unidades deben contabilizarse en los términos de entrada/salida de entalpía según la convención del modelo de unidad. Si el denominador se anula ($|\text{Total in}| = 0$), se debe proteger la división mediante `safediv` o evaluar un criterio de residual absoluto.

### 1.3. Condición de Parada (`Criteria`, `Tolerance %`, `Max Rounds`)

Dada una tolerancia porcentual $\varepsilon_{\text{tol}}$ (`Tolerance %`, valor ilustrado `0.1%` en la documentación) y un límite de iteraciones $K_{\max}$ (`Max Rounds`, p.ej. `5` o `50`), el bucle estático detiene las iteraciones en la primera ronda $k \le K_{\max}$ que satisfaga el criterio activo:

- **`Mass`:** $MB^{(k)} \le \varepsilon_{\text{tol}}$
- **`Enthalpy`:** $EB^{(k)} \le \varepsilon_{\text{tol}}$
- **`Both`:** $MB^{(k)} \le \varepsilon_{\text{tol}} \;\land\; EB^{(k)} \le \varepsilon_{\text{tol}}$

Adicionalmente, el usuario puede exigir la convergencia simultánea de todos los lazos de control activos mediante **`Run Until Controls Converge`**.

---

## 2. Topología del Diagrama, Orden de Cálculo y Reciclos *(formulación clean-room)*

El diagrama de flujo se representa matemáticamente como un grafo dirigido $G = (V, E)$, donde cada vértice $u \in V$ representa una unidad y cada arista dirigida $e = (u \to v) \in E$ representa una corriente interna.

1. **Orden topológico base (`Calculation Order`):**
   - Si $G$ es un grafo dirigido acíclico (DAG), una única pasada siguiendo un orden topológico $\pi = (u_{\pi(1)}, \dots, u_{\pi(|V|)})$ evalúa el proceso de forma exacta en $k = 1$ ronda siempre que cada unidad resuelva su balance local.
2. **Descomposición en reciclos (SCC):**
   - Cuando existen corrientes de recirculación, $G$ contiene una o más componentes fuertemente conexas (algoritmo de Tarjan).
   - Dentro de cada componente cíclica, se selecciona un subconjunto de aristas de corte (*tear streams*) $\mathcal{T} \subset E$ cuyo vector de estado (flujos por especie/fase, distribución granulométrica, temperatura y presión) constituye la variable de iteración $\mathbf{z}^{(k)}$.
3. **Iteración de punto fijo y aceleración:**
   - Una ronda $k$ ejecuta $\mathbf{z}_{\text{calc}}^{(k)} = \mathcal{F}(\mathbf{z}^{(k-1)})$ siguiendo el orden de evaluación; con actualización directa (sustitución sucesiva de Wegstein/punto fijo amortiguado) o aceleración de lazos, el cálculo prosigue hasta cumplir el criterio de la Sección 1 o alcanzar `Max Rounds`.
   - Al ejecutar **`Clear Internal Streams`**, todas las corrientes internas $\mathbf{z}^{(0)}$ se reinicializan a cero antes de recalcular desde las corrientes externas $\mathcal{S}_{\text{in}}$.

---

## 3. Algoritmos del Resolvedor de Controles (`Control Solver` y `Control Monitor`)

Cada bloque de control definido dentro de una unidad (`Controls` dock / barra de unidad) vincula:
- Variable objetivo medida: **`Y TARGET NAME`** ($Y$, comparada contra **`Set Point`** $Y_{\text{sp}}$).
- Variable manipulada: **`X VARIABLE NAME`** ($X$, acotada a su dominio admisible $[X_{\min}, X_{\max}]$ cuando se configura).
- Método seleccionado en **`CONTROL METHOD`**.

### 3.1. Régimen Estático — `Tangential (fast)` *(formulación clean-room inferida)*

En modo estacionario, cada lazo activo $j$ busca la raíz de la función residual evaluada sobre el modelo de la unidad o el flowsheet:

$$g_j(X_j) = Y_j(X_j) - Y_{\text{sp}, j} = 0 \qquad (3)$$

El método **`Tangential (fast)`** observado en el editor de controles corresponde a una aproximación cuasi-Newton / secante de pendiente tangente por diferencias finitas:

$$m_k = \frac{Y(X_k) - Y(X_{k-1})}{X_k - X_{k-1}}, \qquad X_{k+1} = \operatorname{clip}\!\left(X_k - \omega \frac{Y(X_k) - Y_{\text{sp}}}{m_k},\; X_{\min},\; X_{\max}\right) \qquad (4)$$

con paso de perturbación inicial pequeño $\delta X$ cuando aún no existen dos puntos previos $(X_{k-1}, X_k)$, factor de amortiguamiento $\omega \in (0, 1]$ y guarda numérica contra $|m_k| \approx 0$. El **`Control Monitor`** (`Fig. 70`) grafica la trayectoria discreta $(X_k, Y_k)$ frente al objetivo $Y_{\text{sp}}$ en un barrido de evaluación (observado de `0` a `50` pasos).

### 3.2. Régimen Dinámico — Controlador `PID` *(formulación clean-room inferida)*

Para simulaciones con avance temporal discreto $t_n = t_0 + n\,\Delta t$ (`Start Time`, `Time Step` p.ej. `1 s`, `End Time`), el bloque expone **`PID PARAMETERS`** con error instantáneo $e_n = Y_{\text{sp}, n} - Y_n$:

$$u_n = K_p \left[ e_n + \frac{\Delta t}{T_i} \sum_{m=0}^{n} e_m + T_d \frac{e_n - e_{n-1}}{\Delta t} \right] \qquad (5)$$

sujeto a saturación de actuador $[X_{\min}, X_{\max}]$ y mecanismo *anti-windup* cuando el lazo está activo (`Active = true`).

---

## 4. Distribuciones Estadísticas del Editor de Escenarios (`Monte Carlo`)

Cuando la columna de variable **`SET`** tiene habilitado **`Monte Carlo = ON`**, cada corrida $r \in \{1, \dots, N_{\text{runs}}\}$ genera un valor pseudoaleatorio independiente según la distribución elegida en **`Dist. Type`** (`Fig. 32–34`):

### 4.1. Distribución Uniforme (`Uniform`)

Parámetros configurables en grilla: **`Min`** ($a$) y **`Max`** ($b$), con $a < b$:

$$f(x) = \begin{cases} \dfrac{1}{b - a}, & a \le x \le b \\ 0, & \text{en otro caso} \end{cases}, \qquad X = a + (b - a)\,U, \quad U \sim \mathcal{U}(0, 1) \qquad (6)$$

### 4.2. Distribución Normal (`Normal`, con acotamiento opcional `[Min, Max]`)

Parámetros configurables: **`Mean`** ($\mu$), **`SD`** ($\sigma > 0$), **`Min`** ($a$) y **`Max`** ($b$):

$$f(x) = \frac{1}{\sigma \sqrt{2\pi}} \exp\!\left(-\frac{(x - \mu)^2}{2\sigma^2}\right) \qquad (7)$$

En la captura oficial de referencia (`Fig. 33–34`), se configura $\mu = 185.0$, $\sigma = 20.21$, $\text{Min} = 150.0$, $\text{Max} = 250.0$ con `Number of Runs = 20`. Cuando se especifican cotas finitas $[a, b]$, el muestreador aplica distribución normal truncada:

$$F_{[a, b]}(x) = \frac{\Phi\!\left(\frac{x-\mu}{\sigma}\right) - \Phi\!\left(\frac{a-\mu}{\sigma}\right)}{\Phi\!\left(\frac{b-\mu}{\sigma}\right) - \Phi\!\left(\frac{a-\mu}{\sigma}\right)}, \qquad x \in [a, b] \qquad (8)$$

implementable exactamente mediante transformada inversa sobre $U \sim \mathcal{U}(0, 1)$ o muestreo por aceptación-rechazo dentro de $[a, b]$.

### 4.3. Distribución de Laplace (`Laplace`)

Parámetros configurables: localización **`Mean`** ($\mu$), parámetro de escala **`b`** ($b_L > 0$), y cotas opcionales `Min`/`Max`:

$$f(x) = \frac{1}{2 b_L} \exp\!\left(-\frac{|x - \mu|}{b_L}\right), \qquad F(x) = \begin{cases} \dfrac{1}{2}\exp\!\left(\dfrac{x - \mu}{b_L}\right), & x < \mu \\ 1 - \dfrac{1}{2}\exp\!\left(-\dfrac{x - \mu}{b_L}\right), & x \ge \mu \end{cases} \qquad (9)$$

### 4.4. Distribución Trapezoidal (`Trapezoid`)

Parámetros configurables: **`Min`** ($a$), **`b`**, **`c`** y **`Max`** ($d$), con ordenamiento $a \le b \le c \le d$ y $a < d$:

$$h = \frac{2}{(d - a) + (c - b)}, \qquad f(x) = \begin{cases}
h \dfrac{x - a}{b - a}, & a \le x < b \\
h, & b \le x \le c \\
h \dfrac{d - x}{d - c}, & c < x \le d \\
0, & \text{en otro caso}
\end{cases} \qquad (10)$$

(Se reduce a distribución triangular cuando $b = c$, y a uniforme cuando $a = b$ y $c = d$.)

### 4.5. Cuantización **`Round to Nearest`**

Si el campo **`Round to Nearest`** especifica un paso $\Delta q > 0$, la muestra generada $X$ se discretiza antes de inyectarse en la celda `SET`:

$$X_{\text{final}} = \Delta q \cdot \left\lfloor \frac{X}{\Delta q} + 0.5 \right\rfloor \qquad (11)$$

Las vistas **`Distribution Charts`** muestran simultáneamente la función de densidad de probabilidad (**PDF**) y la función de distribución acumulada empírica/teórica (**CDF**).

---

## 5. Seguridad Numérica y Mapeo Visual Sankey

### 5.1. Función Protegida `safediv` (`40.4`)

Durante la importación de hojas desde **HSC Sim 7**, todas las divisiones se reescriben con la primitiva segura **`safediv(num, den)`** para evitar errores `#DIV/0!` durante estados intermedios con corrientes vacías:

$$\operatorname{safediv}(\text{num}, \text{den}) = \begin{cases} \dfrac{\text{num}}{\text{den}}, & |\text{den}| > \epsilon_{\text{zero}} \\ 0, & |\text{den}| \le \epsilon_{\text{zero}} \end{cases} \qquad (12)$$

### 5.2. Escalado de Ancho y Rango de Colores Sankey (`Visualization Settings`, `Fig. 68`)

Dada la variable de corriente seleccionada $v_s$, el multiplicador de potencia $M$ (ej. **`Multiplier: 10^1`**), el modo **`Use absolute values`**, y los límites globales o por página (`Scale Pages Separately`) $[v_{\min}, v_{\max}]$ configurados con **`Min/Max`**:

$$\tilde{v}_s = \frac{\text{val}(v_s)}{M}, \qquad \text{donde } \text{val}(v_s) = \begin{cases} |v_s|, & \text{si } \texttt{Use absolute values} = \text{ON} \\ v_s, & \text{en otro caso} \end{cases} \qquad (13)$$

$$t_s = \operatorname{clip}\!\left(\frac{\tilde{v}_s - v_{\min}}{v_{\max} - v_{\min}},\; 0,\; 1\right), \qquad w_s = w_{\min} + t_s \cdot \left(W_{\max} - w_{\min}\right) \qquad (14)$$

donde $W_{\max}$ corresponde a **`Stream Width Max`** cuando está activa la opción **`Visualize Stream Line Width`**, mientras que $t_s$ interpola la paleta cromática seleccionada (`Flowsheet stream colors`, `Positive and negative colors` o `Range colors`).
