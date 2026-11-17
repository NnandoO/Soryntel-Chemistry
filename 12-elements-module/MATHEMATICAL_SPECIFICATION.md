# Especificación Matemática — Módulo 12: Elements Module

## 1. Normalización Proporcional en el Diagrama Periódico
Dada una fila de propiedad numérica $v(Z) \in \mathbb{R}$ definida sobre un subconjunto válido $Z \in \mathcal{V} \subseteq \{1,\dots,118\}$:

### 1.1 Escalado Lineal de Diámetro (Comportamiento Nativo HSC6)
Sea $M = \max_{Z \in \mathcal{V}} |v(Z)|$. Si $M > 0$, el diámetro normalizado $d(Z)$ de la esfera dentro de una celda periódica de lado interno útil $d_{\text{box}}$ es:
$$d(Z) = d_{\text{box}} \cdot \frac{|v(Z)|}{M} \in [0,\; d_{\text{box}}]$$
- El elemento con máximo absoluto $|v(Z^*)| = M$ dibuja una esfera que toca exactamente los bordes de su casilla ($d(Z^*) = d_{\text{box}}$).
- Para valores negativos ($v(Z) < 0$, p. ej. temperaturas de fusión en °C, afinidad electrónica o $\Delta_f H^\circ$), la magnitud $|v(Z)|$ determina el tamaño de la esfera y el signo negativo se distingue mediante color de contraste (azul/teal para $v \ge 0$, coral/rojo para $v < 0$).

### 1.2 Escalado Volumétrico Opcional (Área / Volumen)
Para propiedades con amplio rango dinámico (como conductividad eléctrica o abundancia en corteza/mar/aire, con varios órdenes de magnitud), se ofrece opcionalmente modo de escala raíz ($p \in \{1,\; 1/2,\; \log\}$):
$$d_{\text{sqrt}}(Z) = d_{\text{box}} \cdot \sqrt{\frac{|v(Z)|}{M}}, \qquad d_{\log}(Z) = d_{\text{box}} \cdot \frac{\ln(1 + |v(Z)|/v_{\min}^+)}{\ln(1 + M/v_{\min}^+)}$$
dejando el modo lineal ($p=1$) activo por defecto para paridad exacta con HSC6 (`24 Elements Module.txt`).

## 2. Estadísticas Descriptivas por Propiedad
Para la fila seleccionada sobre $\mathcal{V}$ ($N = |\mathcal{V}| \ge 1$):
$$v_{\min} = \min_{Z \in \mathcal{V}} v(Z), \quad v_{\max} = \max_{Z \in \mathcal{V}} v(Z), \quad \mu = \frac{1}{N} \sum_{Z \in \mathcal{V}} v(Z), \quad \sigma = \sqrt{\frac{1}{N}\sum_{Z \in \mathcal{V}}(v(Z)-\mu)^2}$$
que se muestran en la leyenda superior del diagrama periódico junto con los elementos extremos $(Z_{\min}, Z_{\max})$.

## 3. Consistencia de Conversión de Unidades Térmicas
- Temperatura: $T(\text{K}) = T(^\circ\text{C}) + 273.15$ (puntos de fusión y ebullición).
- Energía/Entalpía: $1\text{ kcal/mol} = 4.184\text{ kJ/mol}$ ($\Delta_f H^\circ_{298}$, $\Delta_f G^\circ_{298}$, $H_{\text{fus}}$, $H_{\text{vap}}$).
- Potencial de ionización y afinidad electrónica: $1\text{ eV} = 96.48533\text{ kJ/mol}$.
- Masa molecular / relación estequiométrica en fórmulas verificadas desde `SpeciesCatalogPort`:
  $$M_r = \sum_i n_i A_r(Z_i)$$
  utilizando los mismos pesos atómicos estándar IUPAC ($A_r$) en todo `soryntel-thermo`.
