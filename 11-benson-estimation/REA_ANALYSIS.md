# Análisis REA — Módulo 11: Benson Estimation

## 1. Hallazgos en HSC6 / HSC9 y Relación con el Módulo 06
- En `HSC Chemistry 6` (`/home/nando/Descargas/HSC6/Help/`):
  - `29 Hydrocarbons.txt` documenta la nomenclatura, sufijos y fórmulas estructurales abreviadas (ej. `CH3CH2CHClCH3`, `C6H5CH3`, `-C(CH3)3`, `-CH(CH3)2`) para distinguir isómeros orgánicos en `Maindb6.HSC`.
  - `33 Data References.txt` cita a **Benson** en estimaciones empíricas (`Hisham & Benson 1986–1988` para sólidos inorgánicos), mientras que el **Módulo 06 (`HSC Estimate`)** implementado en `06-estimate/` cubre principalmente aditividad inorgánica (Neumann-Kopp elemental/óxidos, Latimer, Pauling).
- En **HSC Chemistry 9 / 10**, **Benson Estimation** aparece como módulo dedicado para **compuestos orgánicos** que permite:
  1. Ingresar una estructura molecular (**SMILES** o fórmula semidesarrollada / grafo molecular) o editar directamente la lista de **grupos de Benson** (`C-(H)3(C)`, `C-(H)2(C)2`, `Cb-(H)`, `O-(H)(C)`, `CO-(C)(O)`, etc.).
  2. Aplicar correcciones conformacionales, de simetría rotacional externa ($\sigma$), quiralidad ($n_{\text{opt}}$) y tensión cíclica (*ring strain*).
  3. Obtener $\Delta_f H^\circ_{298}$, $S^\circ_{298}$, $C_p(300..1500\text{ K})$, coeficientes **Kelley (`A, B, C, D`)**, y opcionalmente propiedades de fase condensada (`Joback` $T_b, T_m$ + Trouton).
  4. Guardar la especie estimada en la base de datos a través del puerto hexagonal.

## 2. Rol en la Arquitectura Hexagonal (`soryntel`)
- **Dominio (`soryntel-thermo::benson`)**:
  - Tabla de contribuciones de grupos de Benson ($\Delta_f H^\circ_{298}$, $S^\circ_{298}$ y $C_p$ en `[300, 400, 500, 600, 800, 1000, 1500]` K).
  - Parser de grafo molecular desde **SMILES** (enlaces simples/dobles/triples, ramificaciones `( )`, anillos `1..9`, aromáticos `c`, heteroátomos `O, N, S, F, Cl, Br, I`) y extracción automática de multiplicidades de grupos Benson $(X-(Y_1)(Y_2)\dots)$ + correcciones básicas.
  - Ajuste de polinomio Kelley mediante `fit_kelley_cp` (reutilizando `soryntel_thermo::estimate::fit_kelley_cp`).
- **Puerto de Salida (`SpeciesCatalogPort`)**:
  - Consulta de referencia experimental en catálogo si existe el compuesto (`get_species` / `search_species`) y persistencia de la especie estimada (`insert_or_update_species`).
- **Capa de Aplicación (`soryntel-application`)**:
  - Casos de uso `parse_benson_smiles`, `estimate_benson` y `save_estimated_species`.
- **Adaptador Primario (`soryntel-desktop::benson_ui`)**:
  - Interfaz responsiva para edición de SMILES / grupos / correcciones, inspección de contribuciones por grupo, tabla `H, S, G, Cp(T)`, ajuste Kelley y gráfico `Cp(T)` vs comparación con catálogo.

## 3. Brechas Técnicas a Resolver
1. Parser SMILES robusto sin dependencias pesadas externas (grafo átomo–enlace con valencias implícitas de H y aromaticidad básica).
2. Base curada de ~60+ grupos de Benson (`C`, `Cd`, `Ct`, `Cb`, `O`, `CO`, `N`, `S`, halógenos `F/Cl/Br`) y correcciones de anillo/*gauche*/*cis*.
3. Cálculo consistente en kJ/mol y kcal/mol con conversión exacta a `ChemicalSpecies` SI (`KelleyCoefficients`).
