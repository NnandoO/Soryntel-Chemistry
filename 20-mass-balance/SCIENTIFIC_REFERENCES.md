# Referencias Científicas y Bibliográficas — Módulo 20: Mass Balance

## 1. Documentación Primaria del Módulo

- Manual de usuario oficial *HSC Chemistry — Chapter 51: HSC - Mass Balance*, Antti Remes, Jaana Tommiska, Pertti Lamberg, Metso Finland Oy, 5 de septiembre de 2023 (`39` páginas):
  - Texto estructurado: [51-mass-balance.md](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.md)
  - PDF fuente: [51-mass-balance.pdf](file:///home/nando/Documentos/Docs%20HSC/51-mass-balance/51-mass-balance.pdf)
  - Capturas oficiales (`34` archivos PNG): `/home/nando/Documentos/Docs HSC/51-mass-balance/imagenes/`
- Referencia cruzada citada explícitamente en el capítulo 51 para conversión elemento → mineral:
  - *HSC Geo User Manual — Chapter 84.6 (Element to Mineral conversion)* (y su equivalente implementado en [species_converter.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/species_converter.rs#L285-L520)).

---

## 2. Bibliografía Oficial Citada en el Capítulo 51 (`Sección 51.8`)

1. **Golub, G. H., & Van Loan, C. F. (1996).** *Matrix Computations* (3rd ed.). Johns Hopkins University Press. *(Referencia base para mínimos cuadrados ponderados lineales WLS, factorización QR/SVD y rango numérico en la Etapa 2 del balance de leyes `Eqs. 8–11, 27–29`.)*
2. **Markovsky, I., Rastello, M.-L., Premoli, A., Kukush, A., & Van Huffel, S. (2006).** "The element-wise weighted total least-squares problem." *Computational Statistics & Data Analysis*, 50(1), 181–209. *(Referencia base del algoritmo iterativo **EWTLS** empleado en la Etapa 1 para reconciliación simultánea de caudales totales, agua y subflujos con incertidumbre en matriz de diseño y vector de observaciones `Eqs. 2–7, 12–18, 19–26`.)*
3. **Lawson, C. L., & Hanson, R. J. (1974).** *Solving Least Squares Problems*. Prentice-Hall. (Edición revisada SIAM Classics in Applied Mathematics, 1995). *(Referencia del algoritmo de conjunto activo **NNLS** — Non-Negative Least Squares — para solución con restricción $b \ge 0$.)*
4. **Haskell, K. H., & Hanson, R. J. (1981).** "An algorithm for linear least squares problems with equality and nonnegativity constraints." *Mathematical Programming*, 21(1), 98–118. *(Referencia del método **CLS** — Constrained Least Squares — empleado cuando existen límites inferiores/superiores `Min/Max` y restricciones de igualdad lineales simultáneas `Sum = 100`.)*
5. **Barrodale, I., & Phillips, C. (1975).** "Algorithm 495: Solution of an overdetermined system of linear equations in the Chebyshev norm [F4]." *ACM Transactions on Mathematical Software (TOMS)*, 1(3), 264–270. *(Referencia de la variante **`LS, MinMax`** mediante minimización del error máximo en norma $L_\infty$.)*

---

## 3. Referencias Complementarias para Implementación *Clean-Room* en Procesamiento Mineral y Reconciliación de Datos

6. **Wills, B. A., & Finch, J. A. (2016).** *Wills' Mineral Processing Technology* (8th ed.). Butterworth-Heinemann. *(Fundamentos del error de muestreo en tamizado — modelo de Whiten para desviación estándar de fracciones granulométricas `Table 3` — y balances metalúrgicos de circuitos de flotación y molienda).*
7. **Hodouin, D., & Everell, M. D. (1980).** "A hierarchical procedure for adjustment and material balancing of mineral processes data." *International Journal of Mineral Processing*, 7(2), 91–116. *(Formulación clásica en dos etapas flujo–leyes equivalente al desacoplamiento EWTLS/WLS e inferencia de matriz de covarianza aposteriori para intervalos de confianza `±CI`.)*
8. **Narasimhan, S., & Jordache, C. (2000).** *Data Reconciliation and Gross Error Detection: An Intelligent Use of Process Data*. Gulf Publishing Company. *(Propagación de incertidumbre linealizada para cálculo de `SD`, `RSD-%`, `WSSQ` y detección de errores gruesos mediante residuos ponderados / sombreado `RelDiff %`.)*
9. **Napier-Munn, T. J., Morrell, S., Morrison, R. D., & Kojovic, T. (1996).** *Mineral Comminution Circuits: Their Operation and Optimisation*. JKMRC, University of Queensland. *(Metodología de balances de tamaño `1.5D` / `2D` excluyendo unidades de conminución *Size Transforming Units* como molinos y chancadores en circuitos cerrados hidrociclón–molino.)*
