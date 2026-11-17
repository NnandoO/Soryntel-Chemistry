# Módulo 17: Aqua — Ingeniería Inversa Clean-Room (`HSC - Aqua`)

Este directorio contiene la especificación técnica completa (clean-room) del módulo **Aqua (`Aqu` — Aqueous Solutions)** para **Soryntel**, reconstruida a partir del manual oficial de **HSC Chemistry 10** ([36-aqua-module.md](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/36-aqua-module.md)), sus capturas oficiales ([imagenes/](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/imagenes)), los artefactos heredados de **HSC 6** ([Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls)) y la auditoría de brechas sobre `soryntel-thermo`.

> **Nota de versión histórica:** A diferencia de los módulos `01–16` presentes ya desde HSC 6, **HSC 6 no incluía un módulo GUI independiente de `Aqua`** (únicamente disponía de la función `DensityA` respaldada por `Aqueous Solution Density.xls` y de las especies acuosas `ao`/`ai` con extrapolación Criss–Cobble en `Maindb6.HSC`). El módulo interactivo **Aqua** (`*.aqu8`, `AquaDLL.dll`, base Pitzer de `1 031` conjuntos y editor `OwnDB`) fue introducido en generaciones posteriores (**HSC 7/8/9/10**).

---

## Alcance del Módulo

El módulo **Aqua** modela propiedades termodinámicas y coligativas no ideales de **soluciones electrolíticas acuosas homogéneas** (binarias y multicomponente) y expone su motor de coeficientes de actividad al módulo **Equilibrium (`Gem`)** y al simulador de procesos (**`Sim`**):

1. **Coeficientes de actividad iónica y media** ($\gamma_i$, $\gamma_\pm$) en escala **molal** ($\text{mol/kg}$) y **molar / fracción molar** ($\gamma_x$), usando tres modelos seleccionables desde **`Aqua method`**: **`Harvie`** (Pitzer con parámetros binarios y ternarios $\Phi_{ij}, \Psi_{ijk}, \lambda_{ni}$), **`Pitzer`** (solo binarios $\beta^{(0)}, \beta^{(1)}, \beta^{(2)}, C^\phi$ y neutros $\lambda_{ni}$) y **`Davies`** (término electrostático Debye–Hückel extendido).
2. **Propiedades de exceso térmicas:** Entalpía molar/específica ideal vs estimada ($H_{\text{Ideal}}, H_{\text{Estimate}}$), capacidad calorífica ideal vs estimada ($C_{p,\text{Ideal}}, C_{p,\text{Estimate}}$) y entalpía de dilución $\Delta H_D(m_1 \to m_2)$ derivadas de las dependencias térmicas $F(T)$ a `6` parámetros.
3. **Propiedades del solvente y coligativas (`Osmcalc` / `Colligcalc` / `CPPitzer`):** Coeficiente osmótico ($\phi$), actividad del agua ($a_w$), presión de vapor de agua pura ($p_0$) y de la solución ($p$), humedad relativa ($\text{RH}\%$), presión osmótica ($\pi$), depresión del punto de congelación ($\theta_F, \text{FP}$) y elevación del punto de ebullición ($\theta_B, \text{BP}$) tanto a la temperatura $T$ como a la $a_w$ de $25\,^\circ\text{C}$.
4. **Propiedades electroquímicas y de composición:** Fuerza iónica ($I$ vía `Istrength`) y $\text{pH}$ en escala de actividad molal de $\text{H}^+$ (`pHcalc`).
5. **Bases de Pitzer (`Main Database`, `Parameter Sets`, `OwnDB`):** Catálogo maestro con `1 031` conjuntos de parámetros (`425` binarios, `114` Theta, `199` Psi, `293` Lambda), inspector de parámetros activos/faltantes para el sistema actual y base editable del usuario con precedencia sobre `MainDB` e integración directa en `Gem`/`Sim`.
6. **Calculadora de densidad (`Density Calculator`):** Estimación de densidad ($\text{kg/m}^3$) de soluciones electrolíticas multicomponente por fracciones másicas basada en el modelo de **Laliberté & Cooper (2004)** y el libro `Aqueous Solution Density.xls`.

---

## Índice de Documentos

| Documento | Contenido |
|---|---|
| [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/REA_ANALYSIS.md) | Inventario de fuentes (HSC 10 Ch. 36 + HSC 6 `Aqueous Solution Density.xls`), arquitectura lógica `AquaDLL.dll`, desglose de `1 031` parámetros Pitzer, persistencia `*.aqu8`/`OwnDB` y auditoría de brechas en `soryntel-thermo`. |
| [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/MATHEMATICAL_SPECIFICATION.md) | Formulación matemática completa: Ecuaciones `(1)–(39)` del manual oficial, polinomios `p0(T)` del Apéndice A (`< 160 °C` y `>= 160 °C`), ecuación de densidad de agua de McCutcheon, Laliberté–Cooper (2004), escalas molal/molar y anomalías tipográficas detectadas en el texto fuente. |
| [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/FUNCTIONAL_SPECIFICATION.md) | Casos de uso `CU-01` a `CU-09`, contratos de entrada/salida, reglas de neutralidad y disociación completa, precedencia `OwnDB > MainDB` y acoplamiento con `Equilibrium` (`Gem`). |
| [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/UI_SPECIFICATION.md) | Especificación fiel de la interfaz (Figs. 1–10): acceso desde `Aqu`, Ribbon, grilla `Water Phase` (columnas `A–X+`), visor `Main Pitzer Database` (`4` pestañas, código de `4` colores), `Active Pitzer Parameters`, editor `OwnDB` y `Density Calculator`. |
| [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/VALIDATION_PLAN.md) | Casos de validación `A1–A8` con referencias numéricas exactas: solución $\text{Na–S–H}_2\text{O}$ a $40\,^\circ\text{C}$ (Figs. 3–4), agua de mar de Millero $S=35$ a $25\,^\circ\text{C}$ y $0\text{–}35\,^\circ\text{C}$ (Tabla 1, Fig. 10), densidad $\text{ZnSO}_4+\text{H}_2\text{SO}_4$ ($1204.371594\text{ kg/m}^3$), equilibrio $K_w$ y reglas `OwnDB`. |
| [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/17-aqua-module/SCIENTIFIC_REFERENCES.md) | Referencias primarias del capítulo 36 (`1–15`), autores del motor HSC Aqua y fuente original de `Density Calculator`. |
