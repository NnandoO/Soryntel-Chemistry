# Referencias Científicas y Contraste Técnico (`SCIENTIFIC_REFERENCES.md`) — Módulo 3

## 1. Fuentes Primarias Documentadas en HSC (`12-heat-loss.md`, Sec. 12.5)

1. **Incropera, F. P., & DeWitt, D. P. (1996).** *Fundamentals of Heat and Mass Transfer* (4th ed.). John Wiley & Sons, New York. ISBN 0-471-30460-3.
   - *Aporte al módulo:* Base de las correlaciones de convección libre y forzada (Churchill–Chu, Raithby–Hollands, Churchill–Bernstein, Whitaker, encerramientos), factores de forma de conducción y ejemplos de validación (`incropera_92_494`, `incropera_94_503`, `incropera_94_504`).
2. **Taine, J., & Petit, J.-P. (1993).** *Heat Transfer*. Prentice Hall, Hempstead. ISBN 0-13-387994-1.
   - *Aporte al módulo:* Formulación de transferencia radiativa y resistencias térmicas equivalentes.
3. **Siegel, R., & Howell, J. R. (1972).** *Thermal Radiation Heat Transfer* (2nd ed.). Hemisphere Publishing Corporation. ISBN 0-07-057316-6.
   - *Aporte al módulo:* Radiación en recintos grises difusos y factores de intercambio entre superficies.
4. **Themelis, N. J. (1995).** *Transport and Chemical Rate Phenomena*. Gordon and Breach Science Publishers SA. ISBN 2-88449-127-9.
   - *Aporte al módulo:* Ecs. (44)–(45) de radiación gas–superficie y Ec. (48) de emisividad de nube de partículas.
5. **Edwards, D. K., & Balakrishnan, A. (1973).** "Thermal Radiation by Combustion Gases." *International Journal of Heat and Mass Transfer*, 16(1), 25–40.
   - *Aporte al módulo:* Modelo de banda ancha exponencial (**Exponential Wide Band Model, EWBM**) para $\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$.
6. **Lallemant, N., & Weber, R. (1996).** "A computationally efficient procedure for calculating gas radiative properties using the exponential wide band model." *International Journal of Heat and Mass Transfer*, 39(15), 3273–3286.
   - *Aporte al módulo:* Optimización numérica de cuadratura/integración espectral por bandas del EWBM utilizada por el código de Tapio Ahokainen en HSC.
7. **VDI Heat Atlas (1993).** Düsseldorf: VDI-Verlag.
   - *Aporte al módulo:* Longitudes ópticas características (Tabla 1) y acoplamiento gas + nube de partículas (Ecs. 50–53).
8. **Haar, L., Gallagher, J. S., & Kell, G. S. (1984).** *NBS/NRC Steam Tables: Thermodynamic and Transport Properties and Computer Programs for Vapor and Liquid States of Water in SI Units*. Hemisphere Publishing Corporation. ISBN 0-89116-353-0.
   - *Aporte al módulo:* Propiedades termofísicas del agua líquida y vapor empleadas en `heatconv.xls`.
9. **Jokilaakso, A. (1987).** *Virtaustekniikan, lämmönsiirron ja aineensiirron perusteet*. Technical University of Helsinki, Otakustantamo. ISBN 951-672-015-3.
   - *Aporte al módulo:* Fundamentos de transporte en ingeniería metalúrgica finlandesa.
10. **Edwards, D. K. (1983).** "Gas radiation properties." *Heat Exchanger Design Handbook*, No. 5 Physical Properties, VDI-Verlag / Hemisphere Publishing Corp.
    - *Aporte al módulo:* Parámetros espectrales de bandas ($\alpha_0, \beta_0, \omega_0, \nu_0, \eta, \delta$) para el EWBM.

---

## 2. Contraste Técnico y Modernización para Rust (`soryntel`)

| Área | Enfoque en HSC6 (`HeatLoss.exe`) | Modernización recomendada en `soryntel` |
|---|---|---|
| **Constantes físicas** | $\sigma = 5.67\times 10^{-8}$, $g = 9.81$ (típico) | CODATA 2018/2022 exacto ($\sigma = 5.670374419\times 10^{-8}\ \text{W m}^{-2}\text{K}^{-4}$, $g_n = 9.80665\ \text{m s}^{-2}$) manteniendo compatibilidad dentro de tolerancias `approx`. |
| **Resolvedor 1D no lineal** | Iteración fija sobre celdas de hoja de cálculo (`CalHeat`) | Punto fijo con sub-relajación adaptativa de Aitken/Anderson + método de Brent para `Temperature profile` y `Target`, evitando divergencias por discontinuidades en $Ra_L = 1708$ o $Re_L = 5\times 10^5$. |
| **EWBM (Gas Radiation)** | Subrutina VB6/port de Tapio Ahokainen basada en Lallemant & Weber (1996) | Implementación pura en Rust siguiendo Lallemant & Weber (1996) y Edwards (1983) en `f64`, con tabla de bandas verificada contra `OilFurnace.htr8` y `fig09_gas_radiation.jpg`. |
| **Integración de $k(T)$** | Discretización uniforme en `Calculation Grid` ($m=10$) | Cuadratura adaptativa / malla configurable ($m \ge 10$) sobre tabla $k(T)$ con interpolación lineal por tramos idéntica en `m=10` para paridad numérica exacta. |
