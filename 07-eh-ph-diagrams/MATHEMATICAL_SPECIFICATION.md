# Especificación Matemática — Módulo 07: Eh-pH Diagrams

## 1. Reacción Electroquímica General y Ecuación de Nernst
Cualquier equilibrio entre dos especies $A$ y $B$ en medio acuoso se escribe como:
$$a\,A + c\,\text{H}_2\text{O} = b\,B + m\,\text{H}^+ + n\,e^-$$
El potencial de equilibrio en la escala SHE ($E_h$ en Voltios) a temperatura $T$ (K) es:
$$E_h = E^\circ(T) - \frac{2.302585\, R T}{n F} \left( m\,\text{pH} + \log_{10} \frac{a_B^b}{a_A^a} \right), \qquad E^\circ(T) = -\frac{\Delta_r G^\circ(T)}{n F}$$
donde $R = 8.314462618\text{ J/(mol}\cdot\text{K)}$ y $F = 96485.33212\text{ C/mol}$.
- **Líneas horizontales ($m = 0, n \neq 0$):** independientes del pH.
- **Líneas verticales ($n = 0, m \neq 0$):** independientes de $E_h$:
  $$\text{pH} = -\frac{\Delta_r G^\circ(T)}{2.302585\, R T\, m} - \frac{1}{m}\log_{10}\frac{a_B^b}{a_A^a}$$
- **Líneas diagonales ($m \neq 0, n \neq 0$):** pendiente $-\frac{2.302585\, R T\, m}{n F}$.

## 2. Formulación por Envolvente Inferior 2D (STABCAL)
Para un elemento principal $M$ con molalidad total $m_M$ (`mol/kg H2O`), cada especie candidata $i$ con estequiometría $\nu_{M,i} > 0$, carga $z_i$ y potencial químico estándar $\Delta_f G_i^\circ(T)$ define una función lineal en $(\text{pH}, E_h)$:
$$g_i(\text{pH}, E_h) = \frac{1}{\nu_{M,i}} \left[ \mu_i(T, a_i) - \nu_{\text{H}_2\text{O},i}\,\Delta_f G_{\text{H}_2\text{O}}^\circ(T) - \sum_{s \neq M,\text{H},\text{O}} \nu_{s,i}\,\mu_s(\text{pH}, E_h) + \alpha_i\,\text{pH} + \beta_i\,E_h \right]$$
donde la actividad $a_i$ se toma como $a_i = m_M / \nu_{M,i}$ para especies acuosas y $a_i = 1$ para fases puras condensadas (`s`, `l`) o $P_i$ para gases.
La especie predominante en cada punto $(\text{pH}, E_h)$ es:
$$i^*(\text{pH}, E_h) = \arg\min_i g_i(\text{pH}, E_h)$$
Las fronteras del diagrama son exactamente los segmentos donde $g_i(\text{pH}, E_h) = g_j(\text{pH}, E_h) \le g_k(\text{pH}, E_h)\;\forall k$.

## 3. Límites de Estabilidad del Agua (Líneas Punteadas)
- **Límite superior ($\text{O}_2(g)$, $P_{\text{O}_2} = P$ o $1\text{ bar}$):**
  $$2\text{H}_2\text{O} = \text{O}_2(g) + 4\text{H}^+ + 4e^-$$
  $$E_{h,\text{upper}}(\text{pH}) = -\frac{\Delta_r G^\circ(T)}{4F} + \frac{2.302585\, R T}{4F}\log_{10} P_{\text{O}_2} - \frac{2.302585\, R T}{F}\,\text{pH}$$
  *(A $25^\circ\text{C}$, $1\text{ bar}$: $E_{h,\text{upper}} \approx 1.229 - 0.05916\,\text{pH}\text{ V}$).*
- **Límite inferior ($\text{H}_2(g)$, $P_{\text{H}_2} = P$ o $1\text{ bar}$):**
  $$2\text{H}^+ + 2e^- = \text{H}_2(g)$$
  $$E_{h,\text{lower}}(\text{pH}) = -\frac{\Delta_r G^\circ(T)}{2F} - \frac{2.302585\, R T}{2F}\log_{10} P_{\text{H}_2} - \frac{2.302585\, R T}{F}\,\text{pH}$$
  *(A $25^\circ\text{C}$, $1\text{ bar}$: $E_{h,\text{lower}} \approx 0.000 - 0.05916\,\text{pH}\text{ V}$).*

## 4. Restricción de Presión Total
Para mantener fase líquida acuosa a temperatura $T$, la presión total del sistema debe cumplir:
$$P_{\text{total}} \ge P_{\text{sat,H}_2\text{O}}(T)$$
calculada mediante `h2o_saturation_pressure_bar(t_k)` (IAPWS-95).
