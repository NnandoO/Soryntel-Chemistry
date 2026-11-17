# Análisis de Ingeniería Inversa (REA) — Módulo 17: Aqua

## 1. Evidencia Directa Auditada

La especificación del módulo **Aqua** se sustenta exclusivamente en los siguientes artefactos verificados en disco:

1. **Documentación técnica oficial de HSC 10 (Metso Finland Oy, 6 de septiembre de 2023):**
   - [36-aqua-module.md](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/36-aqua-module.md) (`156 451 B`, `2 537` líneas, equivalente a `54` páginas; autores: Petri Kobylin, Justin Salminen, Alexey Rumyantsev, Matias Hultgren y Fedor Vasilyev) y [36-aqua-module.pdf](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/36-aqua-module.pdf) (`1 062 404 B`).
   - Contiene: resumen, secciones `36.1–36.13`, tabla de validación de agua de mar (`Table 1`), ecuaciones `(1)–(39)`, Apéndice A (regresión de presión de vapor de agua pura $p_0(T)$) y Apéndice B (`Tables B1–B4` con inventario de `1 031` conjuntos Pitzer).
2. **Capturas de interfaz de HSC 9 / HSC 10 (`11` archivos PNG):**
   - [Imagen pegada.png](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/imagenes/Imagen%20pegada.png) hasta [Imagen pegada (11).png](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/imagenes/Imagen%20pegada%20%2811%29.png), correspondientes a las **Fig. 1 – Fig. 10** del manual (incluyendo pantalla principal de HSC 10, hoja `Water Phase`, `Main Pitzer Database`, `Active Pitzer Parameters`, `Density Calculator`, editor `OwnDB` y gráfico de temperatura de agua de mar).
3. **Artefactos heredados de HSC 6 inspeccionados:**
   - [Aqueous Solution Density.xls](file:///home/nando/Descargas/HSC6/Databases/Aqueous%20Solution%20Density.xls) (`26 624 B`, binario BIFF8 con encabezado autor `Antti Roine`, hoja `Sheet1`, `59` filas de electrolitos `R02..R60`, columnas `Component`, `c0..c4`, `Weight Fraction Min/Max`, `Temperature °C Min/Max`, referencia `Laliberté, Marc. Cooper, Edward. J. Chem. Eng. Data, 2004, Vol. 49, No. 5, 1141-1151`).
   - El análisis de `/home/nando/Descargas/HSC6/` confirma que HSC 6 carecía del módulo GUI **Aqua**; la calculadora `DensityA(Species;Amount;T)` ya existía referenciada desde `Sim` (`Help/40 Sim Flowsheet.txt`).

---

## 2. Arquitectura del Motor Original (`AquaDLL.dll`) y Flujo de Datos

De acuerdo con las secciones `36.2`, `36.6` y `36.11` del manual, la arquitectura histórica del módulo combina una capa GUI tipo hoja de cálculo (Visual Basic + Formula One + libro auxiliar MS Excel para densidad) con una librería nativa de cálculo (**`AquaDLL.dll`**):

| Subrutina reportada (`AquaDLL.dll`) | Responsabilidad funcional | Ecuaciones vinculadas | Salidas alimentadas en `Water Phase` |
|---|---|---|---|
| **`Istrength`** | Calcula la fuerza iónica molal $I = \frac{1}{2}\sum m_i z_i^2$ y la expone en sistemas binarios y multicomponente. | Ec. `(36)` | Columna `Q4` (`Ionic Strength`, $\text{mol/kg}$). |
| **`Pitzer_AC`** | Evalúa término de Debye–Hückel/Pitzer $F$, coeficientes $\gamma_m$ (escala molal) y $\gamma_x$ (escala molar/fracción molar vía Eq. 37), $\gamma_\pm$, entalpías parciales de exceso $H^{\text{ex}}$ y $C_p$ de exceso por diferenciación de $F(T)$, además del coeficiente osmótico $\phi$. | Ecs. `(3)–(22)`, `(37)`, `(39)` | Columnas `G`, `I`, `K`, `L`, `M` de la fila `4` y `G`, `I`, `J`, `K`, `L` por especie (`7+`). |
| **`CPPitzer`** | Versión reducida de `Pitzer_AC` (sin evaluación de $\gamma_i$) enfocada en calcular $\phi$, $a_w$ y derivadas térmicas de $\phi$ evaluadas en los puntos $T_F$ y $T_B$ del agua pura para soportar `Colligcalc`. | Ecs. `(19)`, `(22)`, `(23)` | Entradas internas a `Colligcalc` (`FP`, `BP`). |
| **`Osmcalc`** | Calcula actividad del agua $a_w$, presión de vapor pura $p_0(T)$ (Apéndice A), presión de vapor de solución $p$, humedad relativa $\text{RH}\%$, densidad de agua $\rho_w(T)$ (McCutcheon) y presión osmótica $\pi$. | Ecs. `(23)–(27)`, `(A1)` | Columnas `N4` (`Osmotic Coeff.`), `O4` (`H2O Vapour Press.`), `P4` (`Sol. Vapour Press.`), `S4` (`Relat. Humidity`), `X4` (`Osmotic Pressure`). |
| **`Colligcalc`** | Resuelve depresión del punto de congelación ($\theta_F$, $\text{FP}$) y elevación del punto de ebullición ($\theta_B$, $\text{BP}$) con expansión de Taylor cuadrática y variación lineal $\Delta C_p$, tanto a la $T$ de la solución como evaluando $a_w$ a $25\,^\circ\text{C}$. | Ecs. `(28)–(35)` | Columnas `T4` (`FP`), `U4` (`BP`), `V4` (`FP (aw at 25°C)`), `W4` (`BP (aw at 25°C)`). |
| **`pHcalc`** | Evalúa $\text{pH} = -\log_{10}(\gamma_{\text{H}^+} m_{\text{H}^+})$ si la especie `H(+a)` está presente en el sistema. | Ec. `(38)` | Columna `R4` (`pH`). |

### Notas críticas de diseño identificadas en `36-aqua-module.md`
- **Desacople de especiación:** Si el usuario introduce especies distintas al agua y un par catión–anión sin resolver primero su equilibrio, Aqua anuncia/advierte y **calcula únicamente la hoja `Water Phase` asumiendo disociación completa** en la fase acuosa (sin minimización de Gibbs interna; para especiación química real se delega a **Equilibrium `Gem`** o **`Sim`** con la casilla Aqua activada).
- **Dependencia térmica de `H` y `Cp`:** Los términos constante $a$ y lineal especial no contribuyen de igual manera al diferenciar $\ln\gamma_i(T)$ (por Gibbs–Helmholtz, $a$ y el término $T^{-1}$/$c\ln T$ actúan de forma diferenciada en $\partial(\ln\gamma)/\partial T$; ver Apéndice B p. 28). Por ello el visor de base de datos distingue explícitamente `T dep. AC`, `T dep. H` y `T dep. Cp`.

---

## 3. Inventario de la Base de Parámetros Pitzer (Apéndice B)

Según el Apéndice B (`36-aqua-module.md`, líneas `1108–1118` y Tablas `B1–B4`), la base estándar integrada (**`Main Database`**) almacena exactamente **`1 031` conjuntos de interacciones**, cada uno parametrizado por la función térmica de 6 coeficientes (Ec. `22`):

| Categoría (Pestaña) | Parámetros almacenados | Cantidad verificada | Ejemplos observados en Tablas B1–B4 / Capturas |
|---|---|---|---|
| **Binary parameters** (`Table B1`) | Pares catión–anión ($\beta^{(0)}_{MX}, \beta^{(1)}_{MX}, \beta^{(2)}_{MX}, C^\phi_{MX}$) | **`425`** | `Ag(+a)-NO3(-a)`, `Al(+3a)-Cl(-a)`, `Ba(+2a)-Br(-a)`, `Na(+a)-SO4(-2a)`, `H(+a)-HSO4(-a)` |
| **Theta parameters** (`Table B2`) | Pares de mismo signo catión–catión y anión–anión ($\theta_{ij}$) | **`114`** | `Al(+3a)-Ca(+2a)`, `H(+a)-Na(+a)`, `HSO4(-a)-SO4(-2a)`, `OH(-a)-SO4(-2a)` |
| **Psi parameters** (`Table B3`) | Tripletes dos cationes–un anión y dos aniones–un catión ($\psi_{ijk}$) | **`199`** | `Al(+3a)-H(+a)-Cl(-a)`, `Al(+3a)-Ca(+2a)-SO4(-2a)`, `Al(OH)4(-a)-OH(-a)-K(+a)` |
| **Lambda parameters** (`Table B4`) | Interacciones neutro acuoso–ion ($\lambda_{ni}$) | **`293`** | `Ar(a)-Br(-a)`, `Ar(a)-Na(+a)`, `B(OH)3(a)-Cl(-a)`, `B(OH)3(a)-H(+a)` |
| **Total MainDB Pitzer** | **Todas las categorías** | **`1 031`** | — |

> **Observación de ingeniería:** En el manual publicado (Apéndice B, pp. 28–54), los **valores numéricos** de `MainDB` están ocultos por propiedad intelectual de Metso (únicamente se publican los pares y banderas `Y` / `N` / vacío por propiedad `AC`, `H`, `Cp`). En una implementación clean-room en Rust (**Soryntel**), dichos coeficientes deberán poblarse a partir de las fuentes científicas abiertas citadas en la sección `36.13` (Pitzer 1973/1995, Harvie–Møller–Weare 1984, Pabalan & Pitzer 1988, Malatesta et al. 1999, Clegg & Whitfield 1991, Silvester & Pitzer 1978, etc.) manteniendo exactamente la estructura de tablas, escalas de validez (`Min/Max T`, `Min/Max m`, `Min/Max I`), banderas térmicas y precedencia `OwnDB`.

---

## 4. Formatos de Persistencia y Bases Externas

1. **Archivos de caso de estudio (`*.aqu8`):**
   - Guardan y restauran la definición del sistema en `Water Phase` (especies con sufijo `(a)` más `H2O`, temperatura en $^\circ\text{C}$ y cantidades en $\text{kmol}$ o unidad seleccionada) y opciones de cálculo (`Aqua method`, `Units`).
2. **Base de datos propia del usuario (`OwnDB`):**
   - Se edita exclusivamente desde la interfaz del módulo **Aqua** (las modificaciones externas con MS Excel provocan error de carga según la pág. 13).
   - Al guardarse localmente (`Save` / `Save As`), queda automáticamente disponible también para los módulos **`Gem`** y **`Sim`** sin necesidad de activar el botón visual `Use OwnDB` (dicho botón solo habilita su visualización y gestión en la ventana `Main Database`).
   - El programa lee cada hoja de `OwnDB` secuencialmente **hasta la primera fila inválida** (regla importante de parser).
3. **Libro de densidades (`Aqueous Solution Density.xls`):**
   - En la inspección binaria directa sobre `/home/nando/Descargas/HSC6/Databases/Aqueous Solution Density.xls` (`26 624 B`, BIFF8) se recuperaron **`59` electrolitos** (`R02..R60`: `AlCl3`, `Al2(SO4)3`, `BaCl2`, `CaCl2`, `CdCl2`, `CdSO4`, `CoCl2`, `CoSO4`, `CrCl3`, `Cr2(SO4)3`, `CuCl2`, `CuSO4`, `FeCl2`, `FeSO4`, `FeCl3`, `Fe2(SO4)3`, `HCl`, `HCN`, `HNO3`, `H3PO4`, `H2SO4`, `KCl`, `K2CO3`, `KNO3`, `KOH`, `K2SO4`, `LiCl`, `Li2SO4`, `MgCl2`, `MgSO4`, `MnCl2`, `MnSO4`, `NaBr`, `NaCl`, `NaClO3`, `Na2CO3`, `NaF`, `NaHCO3`, `NaH2PO4`, `Na2HPO4`, `NaHSO3`, `NaI`, `Na2MoO4`, `NaNO2`, `NaNO3`, `NaOH`, `Na3PO4`, `Na2SO3`, `Na2S2O3`, `Na2SO4`, `NH3`, `NH4Cl`, `NH4NO3`, `(NH4)2SO4`, `NiCl2`, `NiSO4`, `SrCl2`, `ZnCl2`, `ZnSO4`) con sus 5 constantes empíricas `c0..c4` de Laliberté & Cooper (2004).
   - En **HSC 9** (Fig. 7, autor `Tuukka Kotiranta`), esta tabla aparece ampliada (por ejemplo, incluye `Ba(NO3)2`, `Ca(CH3CO2)2`, `Ca(NO3)2`, `CaSO4`, `Cd(NO3)2` además de límites `Temperature Min/Max` poblados).

---

## 5. Auditoría de Brechas en Soryntel (`soryntel-thermo`)

Al inspeccionar el código actual en `/home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/`:

| Capacidad Aqua (HSC 10 Ch. 36) | Estado actual en Soryntel | Evidencia verificada en código | Brecha por implementar |
|---|---|---|---|
| **Módulo independiente `Aqua`** (`Aqu` / `Water Phase` / `*.aqu8`) | **Inexistente** | `crates/soryntel-thermo/src/lib.rs` (solo existen `water`, `equilibrium`, `ehph`, `excel_addins`, etc.) | Crear módulo `aqua` en `soryntel-thermo` + comandos/estado en `soryntel-app` + vista en UI. |
| **Modelos de actividad Pitzer & Harvie** (`1 031` sets, Eq. `16–22`) | **Inexistente** | [`apply_aqua_debye_huckel`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/equilibrium/activity.rs#L125-L153) y [`davies_ln_gamma`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/ehph.rs#L194-L202) solo cubren variantes simplificadas de Davies | Motor `PitzerEngine` con soporte para `Harvie`, `Pitzer` binario y `Davies` (Ec. `15`), más almacenamiento de parámetros $\beta^{(0)},\beta^{(1)},\beta^{(2)}, C^\phi, \theta_{ij}, \psi_{ijk}, \lambda_{ni}$ y curvas $F(T)$. |
| **Propiedades térmicas y coligativas (`Osmcalc`, `Colligcalc`, `CPPitzer`)** | **Inexistente** | No existen subrutinas de $\phi$, $a_w$, $p/p_0$, $\text{RH}$, $\pi$, $\theta_F$, $\theta_B$, $\Delta H_D$, ni $H^{\text{ex}}/C_p^{\text{ex}}$ | Implementar diferenciación analítica de $F(T)$, polinomios Apéndice A ($p_0$), densidad de McCutcheon (Ec. `27`) y ecuaciones cuadráticas `(34)–(35)`. |
| **Conversión molal $\leftrightarrow$ molar en `Equilibrium` (`Gem`)** | **Parcial** | [`gem_solver.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/equilibrium/gem_solver.rs#L91-L98) suma $\ln(55.508435)$ a $\mu^\circ_i/(RT)$ si `mixing_entropy_aqua` y calcula actividad `gamma_i * (ni / h2o_mol) * 55.508435` (línea `318`) | Conectar `SolutionModel::Aqua` al motor Pitzer/Harvie real (incluyendo $a_w$ vía $\phi$, neutros acuosos $\lambda_{ni}$ y parámetros `OwnDB`) en lugar de depender solo de `apply_aqua_debye_huckel`. |
| **Base editable `OwnDB` de Pitzer + exclusión por fila (`4` colores)** | **Inexistente** | Solo existe `OwnDB` para especies en `species.rs` | Almacenamiento propio `OwnDB` Pitzer con precedencia `OwnDB > MainDB`, exclusión booleana y compartición hacia `Gem`/`Sim`. |
| **Calculadora de densidad multicomponente (`Density Calculator` / `DensityA`)** | **Simplificada / Incompleta** | [`eval_aqueous_density_kg_m3`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/excel_addins.rs#L703-L753) usa aproximaciones lineales fijas para solo `5` solutos (`NaCl`, `FeSO4`, `CuSO4`, `H2SO4`, `NaOH`) y `2600.0 kg/m3` por defecto | Reemplazar por tabla completa de Laliberté & Cooper (2004) (`59+` electrolitos con `c0..c4`, cálculo con $w_{\text{tot}} = 1 - w_w$ y verificación de regiones de validez `Mass fraction` / `Temperature °C`). |
