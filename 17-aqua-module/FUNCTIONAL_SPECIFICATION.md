# Especificación Funcional — Módulo 17: Aqua

## 1. Actores y Alcance

- **Usuario Final (Ingeniero Hidrometalúrgico / Químico / Investigador):** Modela propiedades no ideales de soluciones acuosas binarias o multicomponente, selecciona el nivel de teoría (`Harvie`, `Pitzer`, `Davies`), gestiona su propia base `OwnDB` de parámetros de interacción iónica, verifica cobertura/parámetros faltantes de un sistema y calcula densidades de soluciones reales.
- **Consumidores Internos de Soryntel:** Los módulos **`04 - Equilibrium (Gem)`** y **`Sim` (Flowsheet Simulation)** consumen los coeficientes de actividad molal/molar, la actividad del agua y los parámetros `MainDB`/`OwnDB` de Pitzer, así como `DensityA(...)` desde Add-Ins y corrientes de diagrama.

---

## 2. Casos de Uso Funcionales (`CU-01` a `CU-09`)

### `CU-01`: Definición y Cálculo de Solución Acuosa (`Water Phase`, `36.3–36.4`)
- **Entradas:**
  - Lista de especies en columna `A` (`Water Species Data`): el solvente **`H2O`** es obligatorio y se inserta automáticamente como primera especie (fila `7`). Los iones y solutos neutros deben llevar el sufijo **`(a)`** (p. ej. `H(+a)`, `OH(-a)`, `CO3(-2a)`, `CO2(a)`).
  - Temperatura por especie/sistema en columna `B` ($^\circ\text{C}$ por defecto, configurable desde **`Units`**).
  - Cantidades molares en columna `C` ($\text{kmol}$ por defecto; p. ej. `55.50844 kmol` de `H2O` representan `1 000 kg` de agua para escala directa $\text{mol/kg}$).
- **Reglas de negocio:**
  - Al ejecutar **`Calculate`**, calcula automáticamente totales de fase en la fila `4` (`Water Phase`) y propiedades por especie en las filas `7+`: porcentaje molar (`mol-%`), masa (`g`), entalpías y capacidades caloríficas (`H Ideal`, `H Estimate`, `Cp Ideal`, `Cp Estimate`) tanto molares como por $\text{kg}$, `AC Molar Sc.` ($\gamma_x$), `AC Molal Sc.` ($\gamma_m$), `Molality` ($\text{mol/kg}$), `MW` ($\text{g/mol}$) y la matriz estequiométrica elemental incluyendo carga electrónica `e-`.
  - Si existen especies no consistentes con disociación completa o sin par completo catión/anión, advierte y calcula únicamente la tabla `Water Phase` asumiendo **disociación completa**.
  - En los límites muy diluidos ($m_i \to 0$), debe cumplirse $\gamma_i \to 1$ y $H_{\text{Estimate}}, C_{p,\text{Estimate}}$ deben converger al agua pura.

### `CU-02`: Conmutación de Método Termodinámico (`Aqua method`, `36.3` / `36.11`)
- **Opciones seleccionables:**
  1. **`Harvie`** *(por defecto recomendado en multicomponente)*: usa parámetros binarios ($\beta^{(0)}, \beta^{(1)}, \beta^{(2)}, C^\phi$), igual signo ($\theta_{ij}$ con términos asimétricos), tripletes ($\psi_{ijk}$) y neutros ($\lambda_{ni}$).
  2. **`Pitzer`**: desactiva $\theta_{ij}, \Phi_{ij}, \psi_{ijk}$; evalúa únicamente pares binarios ($\beta^{(0)}, \beta^{(1)}, \beta^{(2)}, C^\phi$) y neutros ($\lambda_{ni}$).
  3. **`Davies`**: ignora toda la base de interacción Pitzer; evalúa únicamente el término electrostático Debye–Hückel extendido (Ec. `15`).
- Al conmutar el selector, la interfaz informa explícitamente las simplificaciones aplicadas por el modelo seleccionado.

### `CU-03`: Inspección de Propiedades del Solvente, Osmóticas, Coligativas y pH (`36.11`)
- A partir de las molalidades, carga iónica y modelo activo, calcula y publica en la fila resumen `4` (`Water Phase`):
  - **`Osmotic Coeff.`** ($\phi$), **`H2O Vapour Press.`** ($p_0$ por Apéndice A), **`Sol. Vapour Press.`** ($p = a_w p_0$), **`Relat. Humidity`** ($\text{RH}\% = 100 a_w$) y **`Osmotic Pressure`** ($\pi$ en $\text{Pa}$ usando $\rho_w$ de McCutcheon).
  - **`Ionic Strength`** ($I$, $\text{mol/kg}$).
  - **`pH`**: se calcula exclusivamente si `H(+a)` forma parte de la lista de especies (`pH = -log10(gamma_H+ * m_H+)`); en caso contrario permanece vacío.
  - **`FP`** y **`BP`** ($^\circ\text{C}$): depresión crioscópica y ascenso ebulloscópico calculados con Ecs. `(34)–(35)` usando `CPPitzer` en $T_F$ y $T_B$, más **`FP (aw at 25°C)`** y **`BP (aw at 25°C)`**.

### `CU-04`: Consulta de la Base Maestra de Pitzer (`Main Database`, `36.5–36.6`)
- Abre el catálogo de solo lectura de parámetros disponibles en 4 pestañas: `Binary parameters`, `Theta parameters`, `Psi parameters` y `Lambda parameters`.
- **Seguridad / Propiedad:** No exhibe los coeficientes numéricos de `MainDB`; muestra la matriz de presencia de datos por propiedad (`AC`, `H`, `Cp`) y componente ($b^{(0)}, b^{(1)}, b^{(2)}, C^\phi$) usando los símbolos **`T`** (dependiente de temperatura), **`C`** (constante a $25\,^\circ\text{C}$), **`*`** o vacío (no disponible), y **`X`** junto a checkboxes de activación (`Active set used in Pitzer calc.`), además de los rangos reportados de validez (`Min/Max T [C]`, `Min/Max m`, `Min/Max I`).
- Permite desmarcar checkboxes individuales para excluir conjuntos específicos del cálculo (**estado Rojo**).

### `CU-05`: Auditoría de Parámetros Activos y Faltantes (`Parameter Sets`, `36.5`)
- Toma la lista exacta de especies declaradas en la columna `A` de `Water Phase` y el método activo (`Harvie` / `Pitzer` / `Davies`).
- Presenta dos pestañas:
  1. **`Active Parameters`**: agrupa por bloques (`Pitzer Binaries`, `Theta`, `Psi`, `Lambda`) los parámetros que efectivamente intervienen en el sistema actual junto con su cobertura `T` / `C` / `*` para `AC`, `H` y `Cp`.
  2. **`Missing Parameters`**: enumera los pares/tripletes combinatorios posibles entre las especies presentes que **carecen** de parámetros en las bases activas, advirtiendo así sobre la fiabilidad del resultado (especialmente en $H^{\text{ex}}$ y $C_p^{\text{ex}}$).

### `CU-06`: Creación y Mantenimiento de Base Propia (`OwnDB`, `36.6`)
- **Edición en blanco (`Own Database`):** Permite introducir en celdas blancas especies, casilla **`Show T parameters`** (habilita/deshabilita coeficientes térmicos `b..f` de Ec. `22`; desactivarla borra los términos dependientes de $T$ dejando solo `a`), rangos de validez (`T`, `m`, `I`) y valores numéricos de `a..f`.
- **Menú y acciones:** Menú `Table` (`Open`, `Save`, `Save As`, `Reset` que desvincula sin borrar el archivo físico) y menú contextual **`Delete Row`**.
- **Reglas críticas de integridad:**
  - La lectura de cada hoja se detiene en la **primera fila inválida** (todas las especies de una fila activa deben ser válidas).
  - Las entradas numéricas inválidas en celdas de parámetros o rangos se tratan como `0` en cálculos.
  - Es obligatorio retirar el foco de la celda editada antes de cerrar o guardar para consolidar el valor.
  - Los archivos de `OwnDB` son exclusivos de Aqua (editarlos externamente en Excel invalida su lectura).

### `CU-07`: Precedencia, Visibilidad y Códigos de Color (`Use OwnDB`, `36.6`)
- **Precedencia estricta:** Si un mismo par/triplete existe en `OwnDB` y en `MainDB`, **prevalece `OwnDB`** en el cálculo (salvo que el usuario lo desmarque en `Main Database`).
- **Visibilidad (`Use OwnDB` toggle):**
  - El botón `Use OwnDB` inserta las filas de `OwnDB` al inicio de cada hoja en `Main Database` y reinicia los checkboxes y listas de recolección.
  - **Código de 4 colores:** **Negro** (`MainDB`), **Azul** (solo en `OwnDB`), **Verde** (presente en ambos, sobrescrito por `OwnDB`), **Rojo** (excluido por el usuario desmarcando la casilla).
- **Propagación automática:** Guardar un archivo `OwnDB` lo activa inmediatamente para **`Gem`** y **`Sim`** sin requerir activar el botón visual `Use OwnDB`.

### `CU-08`: Calculadora de Densidad de Soluciones (`Density Calculator`, `36.5`)
- **Flujo:** El usuario selecciona uno o más electrolitos del catálogo disponible (`Add` / `Remove`), ingresa la temperatura ($^\circ\text{C}$) y las **fracciones másicas** ($w_i \ge 0$, $\sum w_i \le 1$; el resto $1 - \sum w_i$ se asigna automáticamente a `H2O`).
- **Salidas y validación:** Reporta la densidad de agua pura (`H2O density`, $\text{kg/m}^3$), la densidad calculada de la solución (`Solution density`, $\text{kg/m}^3$) y marca en las columnas **`Validity region violations`** si algún soluto excede su intervalo experimental de fracción másica o temperatura.

### `CU-09`: Persistencia `*.aqu8`, Edición y Acoplamiento con `Equilibrium` (`Gem`, `36.3` / `36.8`)
- **`File` (`*.aqu8`):** Abrir, guardar y exportar casos de estudio acuosos; acciones estándar de edición (`Cut`, `Copy`, `Paste`, `Find/Replace`) y selector **`Units`** para unidades de temperatura, presión y energía.
- **Acoplamiento con `Gem` (reglas `36.8`):** Para resolver especiación y equilibrio multifásico con fase acuosa no ideal:
  1. Colocar `H2O` como primera especie de la fase acuosa (p. ej. `55.509 mol` por kg de agua).
  2. Asegurar electroneutralidad global e incluir pequeñas semillas (p. ej. `0.001 mol` de `H(+a)` y `OH(-a)`, o `1e-5 mol` de especies gaseosas trazadoras como `O2(g)`/`Cl2(g)` ante reactivos estrictamente estequiométricos) para estabilidad numérica del solver Gibbs.
  3. Mantener consistencia entre `Mixing Entropy` ($\mu^\circ_{\text{mix}}$) y el escalado $\gamma_x = \gamma_m / x(\text{H}_2\text{O})$.
