# Especificación Matemática — Módulo 20: Mass Balance (`HSC - Mass Balance`)

La formulación matemática procede directamente de las secciones `51.4.4`, `51.4.5` y `51.7` (`Ecuaciones 1–29`) y las `Tablas 1–4` y `Figs. 16–18` de [51-mass-balance.md](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.md). En aquellos subcomponentes donde el manual omite la derivación interna (cálculo de intervalos de confianza aposteriori `±CI`, iteración numérica *EWTLS* o efecto numérico exacto de `Low Grade Weighting`), se especifica explícitamente la formulación *clean-room* estándar equivalente.

---

## 1. Definiciones Topológicas y Nomenclatura (`51.7`)

Dado un diagrama con unidades seleccionadas e incidencias dirigidas:

- $N_{F1}$: número de corrientes activas en balance no granulométrico (*unsized*).
- $N_{F2}$: número de corrientes activas en balance granulométrico (*sized* / *sized by assay*).
- $N_U$: número total de unidades seleccionadas.
- $N_{\text{SRU}}$: número de unidades transformadoras de tamaño (*Size Transforming / Size-Reducing Units*, p.ej. molinos o chancadores).
- $N_{SF}$ / $N_S$: número de fracciones granulométricas (subflujos; por defecto `1`: `'Bulk'`).
- $N_E$: número de componentes químicos / minerales / especies analizadas.
- Matriz de incidencia unidad–corriente:

$$e_{\text{flow}}^{\text{unit}} = \begin{cases} +1, & \text{si la corriente } \text{flow} \text{ entra a la unidad } \text{unit} \\ -1, & \text{si la corriente } \text{flow} \text{ sale de la unidad } \text{unit} \\ \;\;0, & \text{en otro caso} \end{cases}$$

Variables primarias y medidas:
- $F$, $FM$: caudal másico de sólidos balanceado y medido.
- $W$, $WM$: caudal de agua/líquido balanceado y medido.
- $Pc$, $PcM$: porcentaje de sólidos balanceado y medido, relacionado con agua mediante $Pc = \frac{100\,F}{F + W}$.
- $G$, $GM$: matrices $N_{F1}\times N_E$ (o $N_{F2}\cdot N_S \times N_E$) de leyes balanceadas y medidas.
- $M$, $MM$: fracción másica granulométrica porcentual, definida por $M_{\text{flow,subflow}} = \frac{100\,F_{\text{flow,subflow}}}{F_{\text{flow}}}$.

---

## 2. Función Objetivo Estadística (`WSSQ`) y Modelos de Incertidumbre (`51.4.4–51.4.5`)

### 2.1. Suma Ponderada de Cuadrados (`Eq. 1`)

Para $k$ corrientes y $n$ componentes/variables observadas con valor medido $a_{ij}$, valor reconciliado $b_{ij}$ y desviación estándar $s_{ij} > 0$:

$$\text{WSSQ} = \sum_{j=1}^{k} \sum_{i=1}^{n} \frac{\left(a_{ij} - b_{ij}\right)^2}{s_{ij}^2} \qquad (1)$$

El parámetro **`Estimate of Null SD`** (valor por defecto en `Fig. 16`: `0.001`) actúa como piso mínimo o sustituto para desviaciones estándar nulas o celdas sin `SD` explícito al evitar divisiones por cero en $\frac{1}{s_{ij}^2}$, mientras que las celdas con estado **`Fixed`** fijan $b_{ij} = a_{ij}$ como restricción exacta y las celdas **`Excluded`** o **`Missing`** se omiten de la suma (1).

### 2.2. Modelos de Error de Medición (`Tabla 3` y `Tabla 4`)

Dado un valor medido $x$:

| Método de Medición | Parámetros | Desviación Estándar Analítica $\text{SD}_{\text{meas}}$ | Desviación Relativa $\text{RSD\%}$ | Aplica `Sampling Error` |
|---|---|---|---|---|
| **`Fixed Absolute`** | $a$ (`SD`) | $\text{SD} = a$ | $\frac{100\,a}{|x|}$ (calculada en azul) | **No** |
| **`Fixed Relative`** | $a$ (`RSD%`) | $\text{SD} = \frac{a}{100}|x|$ (calculada en azul) | $\text{RSD\%} = a$ | **No** |
| **`S-shape`** | $a$ (`RSD%`), $b$ (límite detección `SD` mín), $c$ (saturación `SD` máx) | Formulación estándar *S-shape* acotada entre piso $b$ y techo $c$: $\text{SD}_{\text{meas}}(x) = \min\!\left(c,\; \sqrt{b^2 + \left(\frac{a}{100}x\right)^2}\right)$ *(Nota: equivale a `Fixed Relative + Sampling` si $b=0$ y $c \to \infty$)* | $\frac{100\,\text{SD}}{|x|}$ | **Sí** |
| **`Mineral Grade Based`** | $a$ (ej. `a = 5`) | $\text{SD}_{\text{meas}} = \frac{\text{RSD\%}}{100}\,x$ | $\text{RSD\%} = a \cdot x^{-0.5}$ | **Sí** |
| **`Size Fraction (Whiten)`** | — | $\text{SD}_{\text{meas}} = \begin{cases} 0.1 + \frac{x}{10}, & x \le 10\% \\ 1.0, & x > 10\% \end{cases}$ | $\frac{100\,\text{SD}}{|x|}$ | **Sí** |

**Multiplicador de Calidad de Muestreo (`Tabla 4`):** cuando el método está basado en fórmula (`S-shape`, `Mineral Grade Based`, `Size Fraction (Whiten)`), la desviación estándar de muestreo de corriente se escala por el factor de calidad $q$:

$$q = \begin{cases} 1.0, & \text{Good} \\ 1.5, & \text{Moderate} \\ 3.0, & \text{Bad} \end{cases}, \qquad \text{SD}_{\text{eff}} = \sqrt{\text{SD}_{\text{meas}}^2 + \left(q \cdot \text{SD}_{\text{samp}}\right)^2}$$

*(Inferido por cuadratura estadística estándar de varianzas independientes a partir de la nota de `51.4.4`: "added" para métodos con fórmula frente a columnas separadas `Sampling SD` y `Measurement SD` visibles en el panel `Properties` de `Fig. 15`.)*

### 2.3. Rondas de Muestreo (*Sampling Rounds*)

Para una celda con $R$ rondas válidas $\{x^{(1)}, \dots, x^{(R)}\}$:
- **`Calculate Master Data`**: media muestral $\bar{x} = \frac{1}{R}\sum_{r=1}^{R} x^{(r)}$.
- **`Calculate SD From Data`**: desviación estándar muestral insesgada $s = \sqrt{\frac{1}{R-1}\sum_{r=1}^{R} (x^{(r)} - \bar{x})^2}$ (si $R \ge 2$).

### 2.4. Escalamiento de Leyes (`Figs. 16–18`) y *Low Grade Weighting* (`Fig. 19`)

Antes de resolver los caudales por *EWTLS* (con opción **`Scaling`** activa por defecto), las columnas de leyes y fracciones se normalizan respecto a la desviación estándar de la corriente de alimentación designada `indexOfInput` (correspondiente al índice de fila de la corriente de entrada en los arreglos `Assays` y `AssaysSD`, replicado por `repmat` en la notación MATLAB de la `Fig. 17`):

$$S_{i,c} = \text{AssaysSD}_{\text{indexOfInput},\, c}, \qquad \text{Assays\_scaled}_{i,c} = \frac{\text{Assays}_{i,c}}{100 \cdot S_{i,c}}$$

**`Low Grade Weighting`** (`51.4.5`, `Fig. 19`): aplica un coeficiente $w_{\text{lg}} \in [0,\, 1.5]$ (recomendado inicial $0.7$; $0$ sin efecto; $>1.5$ desaconsejado) sobre componentes traza seleccionados (p.ej. `Au`) para amplificar su peso en la reconciliación frente a especies mayoritarias. *(Diseño clean-room: reducción efectiva de la desviación estándar escalada $s'_{ij} = s_{ij} / \alpha(w_{\text{lg}})$ o factor multiplicativo directo en el peso de residuos de las ecuaciones de balance del componente seleccionado antes de resolver Etapa 1 y Etapa 2.)*

### 2.5. Reglas de Restricción Lineal por Método (`LS`, `NNLS`, `CLS`, `LLS`, `LS MinMax`)

Dado el problema de minimización $\min_b \sum (a_m - b_m)^2 / s_m^2$ sujeto a igualdad $E b = r$:
- **`LS`**: sin restricciones de desigualdad ($b \in \mathbb{R}$). Incluye opcionalmente variante **`LS, MinMax`** en norma de Chebyshev $\min_b \|W(a - b)\|_\infty$ (Barrodale & Phillips, 1975) cuando no hay restricciones de cota.
- **`NNLS`**: no negatividad $b \ge 0$ (Lawson & Hanson, 1974).
- **`CLS`**: cotas simples de intervalo $\text{lb}_m \le b_m \le \text{ub}_m$ (`Min <= b <= Max`, Haskell & Hanson, 1981).
- **`LLS`**: *Limited Least Squares* sujeto a $F > \text{SD}_F$ y $W > \text{SD}_W$ (estrictamente **solo disponible en modo `Unsized`**; ver Regla #12 de `51.6`).

---

## 3. Solución No Granulométrica (*Unsized / Bulk*, `51.7.1`, `Eqs. 2–11`)

El algoritmo ejecuta dos etapas secuenciales principales (con la etapa de flujo subdividida en sólidos primero y agua después):

### 3.1. Etapa 1a — Caudales Totales de Sólidos ($F_{\text{flow,tot}}$, *EWTLS*)

Minimiza errores ponderados por elemento considerando incertidumbre tanto en mediciones $FM$ como en la matriz de leyes medidas $GM$:

- **Restricciones de igualdad (conservación exacta en unidades):**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot F_{\text{flow,tot}} = 0 \qquad (\text{unit} = 1, \dots, N_U) \qquad (2)$$
- **Ecuaciones ponderadas aproximadas de leyes por unidad y componente:**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot GM_{\text{flow,tot,chemical\_element}} \cdot F_{\text{flow,tot}} \approx 0 \qquad (\text{unit} = 1,\dots,N_U;\; \text{chemical\_element} = 1,\dots,N_E) \qquad (3)$$
- **Mediciones directas de flujo de sólidos:**
  $$F_{\text{flow,tot}} \approx FM_{\text{flow,tot}} \qquad (\text{flow} = 1, \dots, N_{F1}) \qquad (4)$$
- *(Más ecuaciones lineales **`Stream by Equation`**: $F_i - c \cdot F_j = 0$ cuando estén definidas en `51.4.2` / `Fig. 12`.)*

### 3.2. Etapa 1b — Caudales de Agua / Líquido ($W_{\text{flow,tot}}$, *EWTLS*)

Usando los caudales de sólidos $F_{\text{flow,tot}}$ ya reconciliados:

- **Conservación de agua en unidades:**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot W_{\text{flow,tot}} = 0 \qquad (\text{unit} = 1, \dots, N_U) \qquad (5)$$
- **Mediciones directas de agua:**
  $$W_{\text{flow,tot}} \approx WM_{\text{flow,tot}} \qquad (\text{flow} = 1, \dots, N_{F1}) \qquad (6)$$
- **Mediciones de `% de sólidos` ($PcM$):** despejando $Pc = \frac{100 F}{F + W} \implies \frac{PcM}{100}(F + W) = F$:
  $$\frac{PcM}{100} \cdot W_{\text{flow,tot}} \approx F_{\text{flow,tot}} - \frac{PcM}{100} \cdot F_{\text{flow,tot}} \qquad (\text{flow} = 1, \dots, N_{F1}) \qquad (7)$$

Las incógnitas quedan sujetas según método elegido a: sin cota (`LS`), $F, W \ge 0$ (`NNLS`), $\text{lb}_1 \le F \le \text{ub}_1,\; \text{lb}_2 \le W \le \text{ub}_2$ (`CLS`), o $F > \text{SD}_F,\; W > \text{SD}_W$ (`LLS`).

### 3.3. Etapa 2 — Reconciliación de Leyes *Bulk* ($G \in \mathbb{R}^{N_{F1} \times N_E}$, *WLS*)

Con $F_{\text{flow,tot}}$ fijos, se construye la matriz $B \in \mathbb{R}^{N_U \times N_{F1}}$ de flujos netos unitarios:

$$B_{\text{unit,flow}} = e_{\text{flow}}^{\text{unit}} \cdot F_{\text{flow,tot}} \qquad (10)$$

Se resuelve $\text{vec}(G)$ (apilamiento por columnas) mediante mínimos cuadrados ponderados lineales (*WLS*):

- **Aproximación a mediciones:**
  $$\text{vec}(G) \approx \text{vec}(GM) \qquad (8)$$
- **Restricciones de conservación exacta por elemento/mineral:**
  $$\text{vec}(B \cdot G) = 0 \qquad (9)$$
- **Restricción opcional `Assay Sum = 100` o `Mineral Sum = 100`:**
  $$\sum_{\text{chemical\_element}=1}^{N_E} G_{\text{flow,tot,chemical\_element}} = 100 \qquad (\text{flow} = 1, \dots, N_{F1}) \qquad (11)$$
  *(Al activar `(11)`, la matriz $B$ se reduce a dimensión $(N_U - 1) \times N_{F1}$ eliminando una unidad para evitar dependencia lineal entre restricciones de igualdad.)*

---

## 4. Solución Granulométrica sin Leyes por Malla (*Sized*, `51.7.2`, `Eqs. 12–18`)

### 4.1. Etapa 1 — Caudales Totales ($F_{\text{flow,tot}}$, *EWTLS*)

- **Conservación total en todas las unidades:**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot F_{\text{flow,tot}} = 0 \qquad (\text{unit} = 1, \dots, N_U) \qquad (12)$$
- **Balance por fracción $m\%$ solo sobre unidades no reductoras de tamaño (`non-SRU`):**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot MM_{\text{flow,subflow}} \cdot F_{\text{flow,tot}} \approx 0 \qquad (\text{unit} = 1,\dots, N_U - N_{\text{SRU}} - 1;\; \text{subflow} = 1,\dots,N_{SF}) \qquad (13)$$
  *(Indexación hasta $N_U - N_{\text{SRU}} - 1$ para evitar dependencia lineal con `(12)` cuando las fracciones suman 100%.)*
- **Mediciones de flujo total:**
  $$F_{\text{flow,tot}} \approx FM_{\text{flow,tot}} \qquad (\text{flow} = 1,\dots,N_{F1}) \qquad (14)$$
- **Leyes `1D` opcionales (si se proveen análisis bulk):**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot GM_{\text{flow,tot,chemical\_element}} \cdot F_{\text{flow,tot}} \approx 0 \qquad (\text{unit} = 1,\dots,N_U;\; \text{chemical\_element} = 1,\dots,N_E) \qquad (15)$$

### 4.2. Etapa 2 — Fracciones Másicas ($M_{\text{flow,subflow}}$, *EWTLS*) y Leyes *Bulk*

Con $F_{\text{flow,tot}}$ resueltos:

- **Mediciones de porcentaje retenido/pasante:**
  $$M_{\text{flow,subflow}} \approx MM_{\text{flow,subflow}} \qquad (16)$$
- **Conservación de subflujos en unidades no reductoras de tamaño:**
  $$\sum_{\text{flow}=1}^{N_{F1}} e_{\text{flow}}^{\text{unit}} \cdot M_{\text{flow,subflow}} \cdot F_{\text{flow,tot}} = 0 \qquad (\text{unit} = 1,\dots,N_U \text{ con } \text{unit}\notin\text{SRU};\; \text{subflow} = 1,\dots,N_{SF}) \qquad (17)$$
- **Cierre al 100% granulométrico por corriente:**
  $$\sum_{i=1}^{N_{SF}} M_{\text{flow,subflow}_i} = 100 \qquad (18)$$
- **Etapa 3 opcional (`51.7.2.3`):** si existen análisis *bulk*, se reconcilian igual que en la Sección `3.3` (`Eqs. 8–11`).

---

## 5. Solución Granulométrica con Leyes por Malla (*Sized by Assay*, `51.7.3`, `Eqs. 19–29`)

**Prerrequisito estricto:** antes de ejecutar `Sized by Assay`, debe haberse resuelto previamente el modelo **Unsized** o **Sized** para fijar los caudales totales $F_{\text{flow,tot}}$.

### 5.1. Etapa 1 — Subflujos por Fracción ($F_{\text{flow,subflow}}$, *EWTLS*)

Sean $n_1, \dots, n_{N_U^{\text{non-SRU}}}$ los índices de unidades no reductoras de tamaño:

- **Conservación de subflujos por malla en unidades no reductoras** (hasta $N_U - 1$ sobre el subconjunto `non-SRU` para evitar redundancia lineal con `(20)`):
  $$\sum_{\text{flow}=1}^{N_{F2}} e_{\text{flow}}^{\text{unit}} \cdot F_{\text{flow,subflow}} = 0 \qquad (\text{unit} \in \{n_1,\dots,n_{N_U-1}\},\; \text{unit}\notin\text{SRU};\; \text{subflow}=1,\dots,N_S) \qquad (19)$$
- **Consistencia de suma hacia caudal total conocido:**
  $$\sum_{\text{subflow}=1}^{N_S} F_{\text{flow,subflow}} = F_{\text{flow,tot}} \qquad (\text{flow}=1,\dots,N_{F2}) \qquad (20)$$
- **Restricciones aproximadas de leyes por malla en unidades no reductoras:**
  $$\sum_{\text{flow}=1}^{N_{F2}} e_{\text{flow}}^{\text{unit}} \cdot GM_{\text{flow,chemical\_element,subflow}} \cdot F_{\text{flow,subflow}} \approx 0 \qquad (\text{unit}\in\text{non-SRU};\; e=1,\dots,N_E;\; s=1,\dots,N_S) \qquad (21)$$
- **Observaciones de subflujo** (por fracción `%m` medida `MM` o por flujo directo medido):
  $$F_{\text{flow,subflow}} \approx \frac{MM_{\text{flow,subflow}} \cdot F_{\text{flow,tot}}}{100} \qquad (\text{o alternativamente } F_{\text{flow,subflow}} \approx F_{\text{flow,subflow}}^{\text{meas}}) \qquad (22–23)$$

### 5.2. Etapa 2 — Leyes por Fracción ($G \in \mathbb{R}^{(N_{F2}\cdot N_S) \times N_E}$, *WLS*)

- **Ajuste a mediciones:**
  $$\text{vec}(G) \approx \text{vec}(GM) \qquad (24)$$
- **Conservación por bloque diagonal de subflujos:**
  $$\text{vec}(B \cdot G) = 0 \qquad (25)$$
  donde $B = \text{diag}(B_{\text{subflow}})$ para $\text{subflow} = 1, \dots, N_S - 1$ (`Eq. 26`, omitiendo la última fracción para evitar dependencia con `(28)`), siendo cada bloque:
  $$B_{\text{subflow}} = e_{\text{flow}}^{\text{unit}} \cdot F_{\text{flow,subflow}} \qquad (27)$$
- **Consistencia de ley cabeza vs suma ponderada por subflujos:**
  $$\sum_{\text{subflow}=1}^{N_S} F_{\text{flow,subflow}} \cdot G_{\text{flow,chemical\_element,subflow}} = F_{\text{flow,tot}} \cdot G_{\text{flow,tot,chemical\_element}} \qquad (\text{flow}=1,\dots,N_{F2};\; e=1,\dots,N_E) \qquad (28)$$
- **Restricción opcional `Minerals/Assay Sum = 100`:**
  $$\sum_{\text{chemical\_element}=1}^{N_E} G_{\text{flow,chemical\_element,subflow}} = 100 \qquad (29)$$
  Al activar `(29)`, cada bloque en `B` reduce su fila de unidades no reductoras en una unidad (`N_S - 1` bloques de tamaño `N_U - N_SRU - 1`). *(En `Sized` y `Sized by Assay` solo están disponibles `LS`, `NNLS` y `CLS`; `LLS` queda bloqueado.)*

---

## 6. Cálculos Derivados y Métricas de Bondad (`51.4.5–51.4.6`)

### 6.1. Recuperaciones, Gravedad Específica (`SG`) y Caudales Volumétricos

- **Recuperación másica y de componente respecto a la corriente de alimentación (`Feed`, `Fig. 20`):**
  $$\text{Rec}_c(\text{stream}) = 100 \cdot \frac{\dot{m}_{c,\text{bal}}(\text{stream})}{\dot{m}_{c,\text{bal}}(\text{feed})}, \qquad \dot{m}_{c,\text{bal}} = \begin{cases} F_{\text{bal}}, & \text{si } c = \text{Solids/Water} \\ F_{\text{bal}} \cdot \frac{G_{\text{bal},c}}{100}, & \text{si } c \text{ es ley en \%} \end{cases}$$
- **Gravedad específica de sólidos (`Show SG`, para minerales con densidad $\rho_m$ conocida):**
  $$\frac{1}{\text{SG}_{\text{solid}}} = \sum_{m=1}^{N_M} \frac{w_m^{\text{bal}}}{\rho_m}, \qquad w_m^{\text{bal}} = \frac{M_{\text{bal},m}}{\sum_k M_{\text{bal},k}}$$
- **Caudal volumétrico de pulpa (`Calculate Volumetric Flows ▾` con gravedad específica del líquido $\rho_L$):**
  $$Q_{\text{vol}} = \frac{F_{\text{bal}}}{\text{SG}_{\text{solid}}} + \frac{W_{\text{bal}}}{\rho_L}$$

### 6.2. Indicadores Estadísticos y Reportes (`Figs. 15–16, 21–23`)

Para cada celda observada $(a_m, b_m, s_m)$:
- `Diff`: $d_m = a_m - b_m$ *(verificado en `Fig. 15`: `RC3 Ccp % Meas=36.1, Bal=33.6, Diff=+2.548`)*.
- `RelDiff %`: $100 \cdot \frac{|a_m - b_m|}{b_m}$ *(verificado en `Fig. 15`: `RC3 Mass g Meas=166.7, Bal=161.6, Diff=5.07, RelDiff%=3.043` = $5.07/166.7$ o $\approx 3.04\%$ respecto a medido — en `Fig. 15` `5.07 / 166.7 = 3.041%` y `0.378 / 3.56 = 10.618%` $\implies$ `RelDiff %` normaliza respecto al valor **medido** $100\,|a_m - b_m|/|a_m|$)*.
- `WSSQ`: suma ponderada $\sum (a_m - b_m)^2 / s_m^2$ (`Fig. 16`: `12.890`).
- `RMSD` y `Relative RMSD` (`Figs. 16, 21`): raíz cuadrática media de diferencias absolutas/relativas $\sqrt{\frac{1}{N_{\text{obs}}}\sum (a_m - b_m)^2}$ y $\sqrt{\frac{1}{N_{\text{obs}}}\sum \left(100\frac{a_m - b_m}{a_m}\right)^2}$ (o respecto a desviación estándar; diseño *clean-room* parametrizado y contrastado contra `Fig. 21`).
- **`Tolerance Error` en `Unit Balance` (`Fig. 22`):** residuo de cierre unitario relativo $|\sum e^{\text{unit}} F| / F_{\text{in}}$ (ej. `Rougher 1 Mass Balance = 1.76E-11`, `Tolerance Error = 1.17E-15`).
- **Intervalo de confianza `±CI` aposteriori (*design clean-room*):** propagado a partir de la covarianza aposteriori de parámetros reconciliados $\Sigma_{\text{post}} = \left(J^T W J\right)^{-1}$ proyectada sobre el subespacio nulo de las restricciones de igualdad exactas $C x = d$ (`95%` $\to z_{0.975} \sqrt{(\Sigma_{\text{post}})_{mm}}$, dando incertidumbre reducida tanto en valores medidos como valores no medidos calculados como `Feed Mass g ± CI = 1` en `Fig. 23`).

---

## 7. Arquitectura Numérica *Clean-Room* en Rust (`soryntel`)

1. **Dependencias lineales y solvers reutilizables:**
   - Reutilizar el solver **Lawson–Hanson NNLS** ya validado en [species_converter.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/species_converter.rs#L520) y la descomposición QR de [species.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/species.rs#L738), o integrar `nalgebra` SVD/QR en `soryntel-solver` para factorizaciones de rango completo y cálculo de pseudoinversas estables.
2. **Módulo Etapa 1 (`solve_flows_ewtls`):**
   - Formular un bucle IRLS / Gauss–Newton amortiguado para *Element-Wise Weighted Total Least Squares* (`Markovsky et al., 2006`) manteniendo registro por ronda del escalar `iterated_tolerance = ||F^(k) - F^(k-1)||_inf / max(1, ||F^(k)||_inf)` para alimentar la curva **`Balance Convergence`** (`Fig. 15`) y detener cuando `iterated_tolerance <= balance_tolerance` o `k == max_iter`.
   - Proyectar cada subproblema cuadrático linealmente restringido mediante eliminación de espacio nulo ($F = F_0 + Z y$ usando SVD de la matriz de conservación $E$, con $Z = \ker(E)$) o conjunto activo KKT (`CLS` Haskell–Hanson para cotas $\text{lb} \le b \le \text{ub}$).
3. **Módulo Etapa 2 (`solve_assays_wls`):**
   - Con $B = E \cdot \text{diag}(F_{\text{bal}})$ construido, resolver $\min_{\text{vec}(G)} \|\Sigma^{-1/2}(\text{vec}(G) - \text{vec}(GM))\|_2^2$ sujeto a $(I_{N_E} \otimes B)\,\text{vec}(G) = 0$ (y ecuaciones `Sum = 100` o `Eq. 28` en `Sized by Assay`) usando `LS` (factorización QR/SVD KKT), `NNLS` / `CLS` (conjunto activo cuadrático estricto) o `LS MinMax` (programación lineal símplex / Barrodale–Phillips L1–L∞ dual).
4. **Postprocesamiento:**
   - Calcular $\Sigma_{\text{post}}$, `±CI`, `SD_bal`, `RSD_bal`, `Diff`, `RelDiff %`, `Recovery-%`, `WSSQ`, `RMSD` y clasificación de estado posterior (`Balanced`, `Calculated`, `Non Available/backcalculated`).
