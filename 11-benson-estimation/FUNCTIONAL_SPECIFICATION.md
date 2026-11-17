# Especificación Funcional — Módulo 11: Benson Estimation

## 1. Casos de Uso
1. **CU-01: Descomposición Automática desde SMILES**
   - El usuario ingresa una cadena SMILES (ej. `CCCC`, `CC(C)C`, `c1ccccc1`, `CCO`, `CC(=O)O`, `C1CCCCC1`) y hace clic en **Parse SMILES**.
   - El motor construye el grafo molecular, determina hibridación (`C`, `Cd`, `Ct`, `Cb`, `CO`, heteroátomos), completa hidrógenos implícitos por valencia estándar, extrae la tabla de grupos de Benson con sus conteos $n_g$, detecta anillos/correcciones y estima $\sigma_{\text{tot}}$.
2. **CU-02: Edición Manual de Grupos y Correcciones**
   - El usuario puede agregar, quitar o ajustar cantidades $n_g$ en la tabla de grupos de Benson (incluyendo moléculas no soportadas por auto-detección completa) y editar $\sigma_{\text{tot}}$, $n_{\text{opt}}$, correcciones *gauche*/*cis*/*ring* y fase objetivo (`Gas` / `Liquid`).
3. **CU-03: Cálculo Termoquímico y Ajuste Kelley**
   - Calcula $\Delta_f H^\circ_{298}$, $S^\circ_{298}$, $\Delta_f G^\circ_{298}$, desglose por contribución de cada grupo, valores de $C_p(T)$ en `298.15, 300, 400, 500, 600, 800, 1000, 1500 K` y los coeficientes Kelley `A, B, C, D` más $R^2$ / error RMS del ajuste.
4. **CU-04: Comparación con Base de Datos y Guardado**
   - Si existe una especie coincidente por fórmula empírica/nombre en el puerto `SpeciesCatalogPort`, muestra valores comparativos (`ΔH`, `S`, `Cp`) y superpone su curva $C_p(T)$ en el gráfico.
   - Permite guardar la especie estimada en el catálogo en memoria (`insert_or_update_species`) para usarla inmediatamente en Reaction, Equilibrium o Diagrams.

## 2. Reglas de Validación
- La fórmula o conjunto de grupos debe producir al menos 1 grupo válido (`total_groups > 0`).
- El número de simetría $\sigma_{\text{tot}} \ge 1$ y número de isómeros ópticos $n_{\text{opt}} \ge 1$.
- Los átomos no enlazados o con exceso de valencia en SMILES devuelven un error descriptivo sin fallar ni bloquear la UI.
- Los grupos desconocidos (sin parámetros Benson tabulados) se reportan explícitamente como advertencia (*missing group contribution*).
