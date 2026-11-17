# Referencias Científicas y Fuentes Primarias — Módulo 17: Aqua

## 1. Documentación Oficial de HSC Chemistry e Instrumentación Analizada

1. **Metso Finland Oy — Kobylin, P., Salminen, J., Rumyantsev, A., Hultgren, M., Vasilyev, F.** *HSC Chemistry 10 User's Guide — Chapter 36: Aqua Module* (`54` págs., `6` de septiembre de 2023), [36-aqua-module.md](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/36-aqua-module.md) y capturas de interfaz en [imagenes/](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/imagenes). *(Créditos históricos del motor Pitzer mostrados en Fig. 5 y Fig. 9: Pertti Koukkari `98`, Risto Pajarre `98`, Justin Salminen `98, 09, 10`, Hannu Sippola `98` y Matias Hultgren `09, 10`)*.
2. **Kotiranta, T. / Roine, A. (Outotec / Outokumpu Research Oy).** *Aqueous Solution Density Database* ([Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls), `59` electrolitos de referencia en HSC 6 ampliados en HSC 9/10 para `Density Calculator` y función `DensityA`).

---

## 2. Referencias Bibliográficas Citadas en la Sección `36.13` de HSC Aqua

1. **Pitzer, K. S.** *Thermodynamics of electrolytes. 1. Theoretical basis and general equations*, *J. Phys. Chem.*, Vol. 77, pp. 268–277, 1973. *(Base teórica fundamental del modelo binario $\beta^{(0)}, \beta^{(1)}, \beta^{(2)}, C^\phi$)*.
2. **Harvie, C. E., Møller, N., Weare, J. H.** *The prediction of mineral solubilities in natural waters: the Na–K–Mg–Ca–H–Cl–SO4–OH–HCO3–CO3–CO2–H2O system to high ionic strengths at 25 °C*, *Geochimica et Cosmochimica Acta*, Vol. 48, pp. 723–751, 1984. *(Extensión multicomponente `Harvie` con parámetros $\theta_{ij}, \Phi_{ij}, \psi_{ijk}, \lambda_{ni}$ y validación en aguas naturales)*.
3. **Debye, P., Hückel, E.** *The theory of electrolytes. I. Lowering of freezing point and related phenomena*, *Physik. Z.*, Vol. 24, 185, 334, 1923 (citado vía Harned, H. S. & Owen, B. B., *The Physical Chemistry of Electrolyte Solutions*, 2nd ed., Reinhold Publishing Corp., 1950, p. 18).
4. **Pitzer, K. S.** *Thermodynamics*, 3rd ed., McGraw-Hill, 1995.
5. **Pitzer, K. S.** *Activity Coefficients in Electrolyte Solutions*, CRC Press, Boca Raton, Florida, USA, 1979.
6. **Prausnitz, J. M., Lichtenthaler, R. N., Gomes de Azevedo, E.** *Molecular Thermodynamics of Fluid-Phase Equilibria*, 3rd ed., Prentice-Hall, 1999.
7. **Silvester, L. F., Pitzer, K. S.** *Thermodynamics of electrolytes. X. Enthalpy and the effects of temperature on the activity coefficients*, *J. Solution Chem.*, Vol. 7, No. 5, pp. 327–337, 1978. *(Base para entalpía aparente relativa $\phi_L$ y entalpía de dilución $\Delta H_D$, Ec. `39`)*.
8. **Criss, C. M., Millero, F. J.** *Modeling the heat capacities of aqueous 1-1 electrolyte solutions with Pitzer's equations*, *J. Phys. Chem.*, Vol. 100, pp. 1288–1294, 1996.
9. **Zemaitis, J. F. Jr., Clark, D. M., Rafal, M., Scrivner, N. C.** *Handbook of Aqueous Electrolyte Thermodynamics. Theory and Application*, DIPPR / AIChE, New York, 1986.
10. **Wagman, D. D. et al.** *The NBS Tables of Chemical Thermodynamic Properties*, *J. Phys. Chem. Ref. Data*, Vol. 11, Suppl. 2 (citado como `5, 10`), 1982.
11. **Pabalan, R. T., Pitzer, K. S.** *Apparent molar heat capacity and other thermodynamic properties of aqueous KCl solutions to high temperatures and pressures*, *J. Chem. Eng. Data*, Vol. 33, pp. 354–362, 1988.
12. **Clegg, S. L., Whitfield, M.** En *Activity Coefficients in Electrolyte Solutions*, 2nd ed., K. S. Pitzer (ed.), CRC Press, Boca Raton, 1991, p. 413. *(Receta de agua de mar de Millero $S = 35$, Tabla 1 y Fig. 10)*.
13. **Malatesta, F., Zamboni, R., Lepori, L.** *Activity and osmotic coefficients from the emf of liquid-membrane cells. VII: Co(ClO4)2, Ni(ClO4)2, K2SO4, CdSO4, CoSO4, and NiSO4*, *J. Solution Chem.*, Vol. 28, No. 5, pp. 593–619, 1999.
14. **McCutcheon, S. C., Martin, J. L., Barnwell, T. O. Jr.** *Water Quality*. En Maidment, D. R. (ed.), *Handbook of Hydrology*, McGraw-Hill, New York, NY, 1424 pp., 1993. *(Ecuación empírica de densidad del agua pura $\rho_w(T_C)$, Ec. `27`)*.
15. **Ge, X., Wang, X.** *Estimation of Freezing Point Depression, Boiling Point Elevation, and Vaporization Enthalpies of Electrolyte Solutions*, *Ind. Eng. Chem. Res.*, Vol. 48, pp. 2229–2235, 2009. *(Derivación cuadrática de $\theta_F$ y $\theta_B$, Ecs. `28–35` empleadas en `Colligcalc`)*.

---

## 3. Referencia de Densidad Multicomponente (`Density Calculator`)

16. **Laliberté, M., Cooper, W. E.** *Model for Calculating the Density of Aqueous Electrolyte Solutions*, *Journal of Chemical & Engineering Data*, Vol. 49, No. 5, pp. 1141–1151, 2004. *(Referencia explícita embebida en la celda `K2` de [Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls) junto con sus constantes `c0..c4` y rangos de fracción másica / temperatura).*
