# Referencias Científicas y Termodinámicas — Módulo 4

1. **White, W. B., Johnson, S. M., & Dantzig, G. B. (1958).**  
   *Chemical Equilibrium in Complex Mixtures.*  
   Journal of Chemical Physics, **28**(5), 751–755.  
   *(Base algorítmica del minimizador de Gibbs en `GIBBS` / `Gibbs5.dll` mediante programación cuadrática secuencial y multiplicadores de Lagrange.)*

2. **Lampinen, M. J., & Vuorisalo, J. (1991).**  
   *Thermodynamic analysis of an electrochemical cell with two electrodes and an electrolyte.*  
   Journal of Chemical Physics, **95**(11), 8401–8409.  
   *(Fundamento del modo `Cell Equilibrium` en HSC.)*

3. **Lampinen, M. J., & Vuorisalo, J. (1992).**  
   *Application of the Gibbs energy minimization method to electrochemical equilibrium calculations.*  
   Acta Polytechnica Scandinavica, Chemical Technology and Metallurgy Series No. **202**.

4. **Redlich, O., & Kister, A. T. (1948).**  
   *Algebraic Representation of Thermodynamic Properties and the Classification of Solutions.*  
   Industrial & Engineering Chemistry, **40**(2), 345–348.  
   *(Polinomios de exceso $G^{Ex}$ documentados en los Apéndices A, B y C del manual para `Ga–As` y `Al–Zn`.)*

5. **Pitzer, K. S. (1973 / 1991).**  
   *Thermodynamics of Electrolytes. I. Theoretical Basis and General Equations.*  
   Journal of Physical Chemistry, **77**(2), 268–277.  
   *(Modelo de coeficientes de actividad acuosos `Aqua`.)*

6. **Criss, C. M., & Cobble, J. W. (1964).**  
   *The Thermodynamic Properties of High Temperature Aqueous Solutions.*  
   Journal of the American Chemical Society, **86**(24), 5385–5393.  
   *(Opción `Criss-Cobble` en `Calculation Options`.)*

7. **Constantes físicas consistentes con HSC / `soryntel-core`:**
   - $R = 8.31431\;\text{J/(mol}\cdot\text{K)}$ (valor explícito del Apéndice A de HSC Equilibrium) / $R_{\text{SI}} = 8.314462618\;\text{J/(mol}\cdot\text{K)}$
   - $1\;\text{cal} = 4.184\;\text{J}$
   - $T_{\text{ref}} = 298.15\;\text{K}$
   - $F = 96485.33212\;\text{C/mol}$
   - Volumen molar gas ideal STP ($0\,^\circ\text{C}, 1\;\text{atm}$): $22.414\;\text{L/mol}$ (`Nm3` $\to$ `kmol`).
