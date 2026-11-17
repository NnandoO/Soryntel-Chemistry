# SCIENTIFIC_REFERENCES — Reaction Equations Module

Este documento compila las fuentes bibliográficas, normas y publicaciones científicas revisadas por pares que fundamentan teórica y numéricamente la implementación independiente del módulo de ecuaciones de reacción en **Soryntel Chemistry**.

---

## 1. Fuentes Termoquímicas Fundamentales (Compilaciones de Datos de Sustancias Puras)

1. **Barin, I., Knacke, O., & Kubaschewski, O. (1973).**
   *Thermodynamic Properties of Inorganic Substances.*
   Springer-Verlag, Berlin / Heidelberg / New York. Supplement publicado en 1977.
   - *Aporte científico:* Base fundamental de las tablas termoquímicas para más de 1000 elementos y compuestos inorgánicos. Define la convención de escalas absolutas para entalpía $H$ y entropía $S$.

2. **Kubaschewski, O., Alcock, C. B., & Spencer, P. J. (1993).**
   *Materials Thermochemistry* (6th ed.).
   Pergamon Press, Oxford.
   - *Aporte científico:* Metodología de interpolación polinomial de capacidades caloríficas $C_p$, formulación de calores latentes de transformación de fases y constantes de equilibrio en metalurgia extractiva.

3. **Pankratz, L. B. (1982).**
   *Thermodynamic Properties of Elements and Oxides.*
   U.S. Department of the Interior, Bureau of Mines, Bulletin 672.
   - *Aporte científico:* Formulación de ecuaciones polinomiales para la dependencia de la energía libre de Gibbs y entalpías estándar de óxidos refractarios.

4. **Pankratz, L. B. (1984).**
   *Thermodynamic Properties of Halides.*
   U.S. Department of the Interior, Bureau of Mines, Bulletin 674.
   - *Aporte científico:* Compilación de funciones de entalpía y energía de formación para halogenuros metálicos y sales anhidras.

5. **Chase, M. W., Jr. (1998).**
   *NIST-JANAF Thermochemical Tables* (4th ed.).
   Journal of Physical and Chemical Reference Data, Monograph 9, 1-1951.
   - *Aporte científico:* Estándar de referencia internacional para gases ideales, constantes atómicas y funciones termodinámicas con rigor espectroscópico.

---

## 2. Termodinámica de Soluciones Acuosas e Iones a Alta Temperatura

6. **Criss, C. M., & Cobble, J. W. (1964).**
   *The Thermodynamic Properties of High Temperature Aqueous Solutions. IV. Entropies of the Ions up to 200° and the Correspondence Principle.*
   *Journal of the American Chemical Society*, 86(24), 5385–5390. https://doi.org/10.1021/ja01078a001
   - *Aporte científico:* Postulación del principio de correspondencia para las entropías molares de iones acuosos y definición de la escala absoluta mediante $S^\circ_{\text{abs}}(\text{H}^+, 298.15\,\text{K}) = -5.0\,\text{cal}/(\text{mol}\cdot\text{K})$.

7. **Criss, C. M., & Cobble, J. W. (1964).**
   *The Thermodynamic Properties of High Temperature Aqueous Solutions. V. The Calculation of Ionic Heat Capacities up to 200°. Entropies and Heat Capacities above 200°.*
   *Journal of the American Chemical Society*, 86(24), 5390–5393. https://doi.org/10.1021/ja01078a002
   - *Aporte científico:* Fórmulas empíricas de extrapolación de $\bar{C}_p$ a partir de la correlación lineal de entropías para cationes simples, aniones simples, oxianiones y aniones ácidos.

8. **Taylor, D. F. (1978).**
   *Thermodynamic Properties of Metal-Water Systems at Elevated Temperatures.*
   *Journal of the Electrochemical Society*, 125(5), 808–811.
   - *Aporte científico:* Extensión de los modelos de Criss-Cobble al cálculo de equilibrios redox en corrosión y reactores hidrometalúrgicos hasta $300\,^\circ\text{C}$.

9. **Chen, C. M., Aral, K., & Theus, G. J. (1983).**
   *Computer-Calculated Potential pH Diagrams to 300°C.*
   Electric Power Research Institute (EPRI), Research Project 1167-2, NP-3137.
   - *Aporte científico:* Métodos computacionales para el cálculo de potenciales redox $E^\circ(T)$ en medios acuosos a alta temperatura.

---

## 3. Normas Internacionales y Convenciones Electroquímicas

10. **IUPAC (International Union of Pure and Applied Chemistry). (2007).**
    *Quantities, Units and Symbols in Physical Chemistry* (The IUPAC "Green Book", 3rd ed.).
    RSC Publishing, Cambridge.
    - *Aporte científico:* Definición oficial de la convención de signos de Estocolmo (1953) para potenciales de electrodo estándar ($E^\circ$ positivo para procesos espontáneos frente al SHE), factores de conversión de unidades termodinámicas y constantes físicas universales (CODATA).

11. **Pourbaix, M. (1974).**
    *Atlas of Electrochemical Equilibria in Aqueous Solutions* (2nd ed.).
    National Association of Corrosion Engineers (NACE), Houston.
    - *Aporte científico:* Relación matemática fundamental entre la energía de Gibbs de semirreacciones y el potencial de electrodo: $E^\circ = -\Delta G^\circ / (n_e F)$.
