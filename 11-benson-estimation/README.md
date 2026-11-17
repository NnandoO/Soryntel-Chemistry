# Módulo 11: Benson Estimation (Aditividad de Grupos de Benson)

## Propósito
El módulo **Benson Estimation** estima propiedades termoquímicas estándar ($\Delta_f H^\circ_{298.15}$, $S^\circ_{298.15}$, $\Delta_f G^\circ_{298.15}$ y $C_p(T)$ en $300\text{–}1500\text{ K}$ ajustado a polinomio de Kelley) para compuestos orgánicos (hidrocarburos, oxigenados, nitrogenados, halogenados, sulfurados y aromáticos) mediante el método de **Aditividad de Grupos de Benson (1958, 1969, 1976)** a partir de cadena SMILES / fórmula semidesarrollada o selección manual de grupos de Benson, con correcciones estructurales (*gauche*, *cis*, tensión de anillo, número de simetría $\sigma$ e isómeros ópticos).

## Documentos de la Especificación REA
- [REA_ANALYSIS.md](./REA_ANALYSIS.md) — Hallazgos en HSC6/HSC9, arquitectura hexagonal y brechas cubiertas respecto al Módulo 06.
- [MATHEMATICAL_SPECIFICATION.md](./MATHEMATICAL_SPECIFICATION.md) — Formulación exacta de grupos de Benson, correcciones no enlazantes ($\sigma$, quiralidad, anillos, *gauche*), regresión Kelley y transición de fase (Joback/Trouton).
- [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) — Descomposición automática desde SMILES/fórmula racional, editor manual de grupos, cálculo de propiedades $T$-dependientes y exportación a catálogo.
- [UI_SPECIFICATION.md](./UI_SPECIFICATION.md) — Diseño de cinta/barra superior, panel de entrada molecular (SMILES / fórmula / átomos), tabla de grupos Benson + correcciones, tabla de salida `H, S, Cp(T)` + coeficientes Kelley y gráfico `Cp(T)`.
- [VALIDATION_PLAN.md](./VALIDATION_PLAN.md) — Casos de referencia orgánicos frente a valores experimentales NIST / `Maindb6` (metano, n-butano, isobutano, benceno, tolueno, etanol, ácido acético, ciclohexano).
- [SCIENTIFIC_REFERENCES.md](./SCIENTIFIC_REFERENCES.md) — Referencias bibliográficas primarias (Benson, Domalski & Hearing, Cohen & Benson, Joback & Reid, NIST).
