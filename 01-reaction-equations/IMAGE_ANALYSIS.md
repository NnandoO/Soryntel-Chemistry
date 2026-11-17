# IMAGE_ANALYSIS — Reaction Equations Module (Inspección Visual Verificada)

## 1. Inventario y Mapeo de Evidencias Visuales

Las 9 imágenes del manual ubicadas en `/home/nando/Documentos/Docs HSC/10-reaction-equations/imagenes/` han sido inspeccionadas **visualmente de forma directa** y contrastadas con el texto de `10-reaction-equations.md` y el PDF original `10-reaction-equations.pdf` (12 páginas, versión 2023).

> [!NOTE]
> Las capturas del manual PDF corresponden a una versión moderna de HSC Chemistry (interfaz Outotec, extensión `.rea8`, documentación 2023), mientras que la instalación binaria local en `/home/nando/Descargas/HSC6/` corresponde a **HSC Chemistry 6.0** (2006, Outokumpu Research Oy). Las diferencias de UI y formatos entre ambas versiones se documentan explícitamente.

| Archivo de Imagen | Resolución | Figura del Manual | Pág. PDF | Contenido Visual Real Observado |
| :--- | :--- | :--- | :--- | :--- |
| `Imagen pegada.png` | 718 × 492 px | **Fig. 1** | Pág. 2 | Ventana principal de *Reaction Equations*: cálculo de sustancia pura **`Al2O3`** (`0.000` a `2000.000 °C`, paso `100.000 °C`), tabla y gráfica de `Heat capacity`. |
| `Imagen pegada (2).png` | 499 × 333 px | **Fig. 2** | Pág. 4 | Barra vertical de 6 iconos de herramientas de gráfico (*Chart tools*: Save, Copy, Print, Clone, Redraw, Toolbox). |
| `Imagen pegada (3).png` | 760 × 211 px | **Fig. 3** | Pág. 5 | Panel `Specifications` con la reacción **`Ni + 4CO(g) = Ni(CO)4(g)`** (`From: 70 °C`, `To: 160 °C`, `Step: 10 °C`, `Smallest Coefficient: 1`). |
| `Imagen pegada (4).png` | 762 × 550 px | **Fig. 4** | Pág. 6 | Gráfico `Equilibrium constant`: eje Y **`K`** en escala logarítmica (`1.0E-3` a `1.0E4`) vs `T (°C)` (`70` a `160 °C`). |
| `Imagen pegada (5).png` | 735 × 713 px | **Fig. 5** | Pág. 7 | Tablas `Reaction Data` y `Species Data` para **`Mg = Mg(g)`** (`500.000` a `1500.000 °C`, paso `50 °C`). |
| `Imagen pegada (6).png` | 747 × 544 px | **Fig. 6** | Pág. 8 | Gráfica comparativa `Equilibrium constant` (*Plot all data*): `K` lineal (`0` a `105`) vs `T (°C)` (`500` a `1500 °C`) para **`Mg = Mg(g)`**, **`Zn = Zn(g)`** y **`Cd = Cd(g)`**. |
| `Imagen pegada (7).png` | 682 × 385 px | **Fig. 7 y Fig. 8** | Pág. 8 | Panel superior (Fig. 7): ecuación sin balancear `FeS2 + H2O = Fe2O3 + SO4(-2a) + H(+a) + e-`. Panel inferior (Fig. 8): ecuación balanceada `2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-` (`0` a `175 °C`, paso `25 °C`). |
| `Imagen pegada (8).png` | 585 × 819 px | **Fig. 9** | Pág. 9 | Resultados tabulares y gráfico `Standard reduction potential` (`E vs. SHE (V)`) para la semirreacción balanceada de `FeS2`. |
| `Imagen pegada (9).png` | 637 × 483 px | **Fig. 10** | Pág. 10 | Ventana `Database Browser` filtrando **`TiO2`** (4 especies coincidentes, datos básicos y tabla de rangos de temperatura). |

---

## 2. Análisis Visual Detallado por Imagen

### 2.1. `Imagen pegada.png` — Fig. 1: Ventana Principal (*One Chemical Substance*)
- **Ruta:** [Imagen pegada.png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada.png)
- **Barra de menú superior:** `File`, `Edit`, `Format`, `Sheet`, `Options`, `Help`.
- **Panel izquierdo superior (`Specifications`):**
  - Campo de texto (fondo amarillo claro): `Reaction Equation or Chemical Formula` con valor **`Al2O3`**.
  - `Smallest Coefficient`: `1`, botón `Balance`.
  - `Temperature`: `From (°C)` = `0.000`, `To (°C)` = `2000.000`, `Step (°C)` = `100.000`.
  - Botones de acción: `Browse...` y `Calculate`.
- **Panel derecho superior (`Chart`):**
  - Desplegable `Y - Axis` (fondo amarillo): `Cp (J/(mol*K))`.
  - Casillas de verificación: `Collect Data`, `Plot All Data`.
  - Botones: `Open in new Window`, `Copy Vector`, `Copy Raster`.
- **Panel izquierdo inferior (Hoja de cálculo tipo Excel, columnas `A..H`, filas `1..32`, pestaña inferior `S1`):**
  - Bloque `Formula`: `Al2O3`.
  - Bloque `Reaction Data`: columnas `T (°C)`, `Cp (J/(mol*K))`, `H (kJ/mol)`, `S (J/(mol*K))`, `G (kJ/mol)`, **`Reference`**.
  - Valores visibles en fila 7 (`0.000 °C`): `Cp = 72.164`, `H = -1677.558`, `S = 44.407`, `G = -1689.688`, `Reference = Barin 77, JANAF 98`.
  - Valores visibles en fila 27 (`2000.000 °C`): `Cp = 145.734`, `H = -1425.351`, `S = 290.464`, `G = -2085.620`, `Reference = Belov 99`.
  - Bloque `Species Data` (filas 29–32): columnas `Formula`, `M (g/mol)`, `Conc. (wt-%)`, `Input Amounts` (`mol`, `g`, `Volume`, `Unit`), **`Extrapolated From T(K)`**.
  - Fila 32: `Al2O3 | 101.961 | 100.000 | 1.000 | 101.961 | 25.715 | ml | -`.
- **Panel derecho inferior (Gráfico):**
  - Título: `Heat capacity`, eje X `T (°C)` (`0` a `2000`), eje Y `Cp (J/(mol*K))` (`75` a `145`), etiqueta `Al2O3` sobre la curva y leyenda con checkbox `[x] Al2O3`.
- **Pie de ventana:** Marca `Outotec`.

### 2.2. `Imagen pegada (2).png` — Fig. 2: Barra de Herramientas del Gráfico (*Chart tools*)
- **Ruta:** [Imagen pegada (2).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(2).png)
- Barra vertical de 6 botones con iconos:
  1. Disquete con flecha verde (**Save**)
  2. Dos hojas superpuestas (**Copy**)
  3. Impresora (**Print**)
  4. Dos cuadros superpuestos naranja/gris (**Clone**)
  5. Flechas circulares azules (**Redraw**)
  6. Letra `A` con pincel (**Toolbox**)

### 2.3. `Imagen pegada (3).png` — Fig. 3: Entrada de Ecuación de Reacción (Proceso Mond)
- **Ruta:** [Imagen pegada (3).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(3).png)
- Ecuación introducida: **`Ni + 4CO(g) = Ni(CO)4(g)`**.
- `Smallest Coefficient`: `1`.
- Rango térmico: `From (°C)` = **`70`**, `To (°C)` = **`160`**, `Step (°C)` = **`10`**.

### 2.4. `Imagen pegada (4).png` — Fig. 4: Gráfica de Equilibrio del Proceso Mond
- **Ruta:** [Imagen pegada (4).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(4).png)
- Título: `Equilibrium constant`.
- Eje Y: **`K`** en **escala logarítmica** desde `1.0E-3` hasta `1.0E4` (marcas mayores por década).
- Eje X: `T (°C)` de **`70` a `160`**.
- Comportamiento visual: la curva decrece linealmente en escala semilogarítmica desde `~1.1E3` a `70 °C` hasta `1.0E-2` a `160 °C`, cruzando `K = 1.0E0` justo antes de `120 °C` (~`119 °C`).

### 2.5. `Imagen pegada (5).png` — Fig. 5: Vaporización de Magnesio (`Mg = Mg(g)`)
- **Ruta:** [Imagen pegada (5).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(5).png)
- Encabezado: `Reaction Equation`: `Mg = Mg(g)`.
- Tabla `Reaction Data`: columnas `T (°C)`, `ΔH (kJ)`, `ΔS (J/K)`, `ΔG (kJ)`, `K`, `Log K`.
- Rango: `500.000 °C` a `1500.000 °C` en pasos de `50 °C`.
- Valores clave verificados visualmente:
  - `500.000 °C`: `ΔH = 143.884`, `ΔS = 110.035`, `ΔG = 58.811`, `K = 1.063E-004`, `Log K = -3.974`.
  - `600.000 °C`: `ΔH = 142.876`, `ΔS = 108.810`, `ΔG = 47.869`, `K = 1.368E-003`, `Log K = -2.864`.
  - `650.000 °C`: `ΔH = 133.839`, `ΔS = 99.002`, `ΔG = 42.445`, `K = 3.964E-003`, `Log K = -2.402` *(salto por fusión de Mg a 923 K ≈ 649.85 °C)*.
  - `1050.000 °C`: `ΔG = 3.876`, `K = 7.030E-001`, `Log K = -0.153`.
  - `1100.000 °C`: `ΔG = -0.818`, `K = 1.074E+000`, `Log K = 0.031` *(punto de ebullición normal donde $p_{\text{Mg}(g)} > 1\,\text{bar}$)*.
  - `1500.000 °C`: `ΔH = 122.337`, `ΔS = 90.170`, `ΔG = -37.548`, `K = 1.277E+001`, `Log K = 1.106`.
- Tabla `Species Data`:
  - `Mg`: `24.305 g/mol | 100.000 wt-% | 1.000 mol | 24.305 g | 13.968 ml | -`
  - `Mg(g)`: `24.305 g/mol | 100.000 wt-% | 1.000 mol | 24.305 g | 22.414 l | -` *(volumen molar ideal estándar a 273.15 K, 1 atm = 22.414 l)*.

### 2.6. `Imagen pegada (6).png` — Fig. 6: Comparación Multireacción (*Plot all data*)
- **Ruta:** [Imagen pegada (6).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(6).png)
- Título: `Equilibrium constant`.
- Eje Y: **`K`** (escala lineal de `0` a `105`), Eje X: `T (°C)` (`500` a `1500`).
- Tres curvas superpuestas con leyenda interactiva (checkboxes):
  - `[x]` Azul: `Mg = Mg(g)` (alcanza `~13` a `1500 °C`).
  - `[x]` Verde: `Zn = Zn(g)` (alcanza `~47` a `1500 °C`).
  - `[x]` Rojo: **`Cd = Cd(g)`** (alcanza `105` a `1500 °C`).

### 2.7. `Imagen pegada (7).png` — Fig. 7 y Fig. 8: Balance de Reacción Acuosa Redox
- **Ruta:** [Imagen pegada (7).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(7).png)
- **Fig. 7 (antes de Balance):** `FeS2 + H2O = Fe2O3 + SO4(-2a) + H(+a) + e-`, `From: 0`, `To: 175`, `Step: 25`.
- **Fig. 8 (después de Balance):** `2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-`, con `Smallest Coefficient: 1`.

### 2.8. `Imagen pegada (8).png` — Fig. 9: Resultados Electroquímicos (`E vs. SHE`)
- **Ruta:** [Imagen pegada (8).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(8).png)
- Tabla `Reaction Data` para `2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-`:
  - Columnas: `T (°C)`, `ΔH (kJ)`, `ΔS (J/K)`, `ΔG (kJ)`, `K`, **`E vs. SHE (V)`** *(no muestra `Log K` en esta vista; muestra el potencial de reducción estándar en voltios)*.
  - Fila `0.000 °C`: `ΔH = 1497.322`, `ΔS = 1318.489`, `ΔG = 1137.177`, `K = 3.300E-218`, `E vs. SHE = -0.393 V`.
  - Fila `25.000 °C`: `ΔH = 1322.639`, `ΔS = 687.984`, `ΔG = 1117.516`, `K = 1.582E-196`, `E vs. SHE = -0.386 V`.
  - Fila `150.000 °C`: `ΔH = 1074.585`, `ΔS = -3.849`, `ΔG = 1076.214`, `K = 1.375E-133`, `E vs. SHE = -0.372 V`.
  - Fila `175.000 °C`: `ΔH = 1011.966`, `ΔS = -147.532`, `ΔG = 1078.082`, `K = 2.149E-126`, `E vs. SHE = -0.373 V`.
- Tabla `Species Data`:
  - Reactivos: `FeS2` (`119.967 g/mol`, `41.210 wt-%`, `2.000 mol`, `239.934 g`, `47.796 ml`), `H2O` (`18.015 g/mol`, `58.790 wt-%`, `19.000 mol`, `342.289 g`, `373.270 ml`).
  - Productos: `Fe2O3` (`159.692 g/mol`, `27.427 wt-%`, `1.000 mol`, `159.692 g`, `30.476 ml`), `SO4(-2a)` (`96.058 g/mol`, `65.992 wt-%`, `4.000 mol`, `384.230 g`, `0.000 ml`), `H(+a)` (`1.008 g/mol`, `6.578 wt-%`, `38.000 mol`, `38.300 g`, `0.000 ml`), `e-` (`0.001 g/mol`, `0.003 wt-%`, `30.000 mol`, `0.016 g`, `0.000 ml`).
- Gráfico inferior: `Standard reduction potential`, eje Y `E vs. SHE (V)` (`-0.393` a `-0.372`), eje X `T (°C)` (`0` a `175`).

### 2.9. `Imagen pegada (9).png` — Fig. 10: Ventana `Database Browser`
- **Ruta:** [Imagen pegada (9).png](file:///home/nando/Documentos/Docs%20HSC/10-reaction-equations/imagenes/Imagen%20pegada%20(9).png)
- Ribbon superior: `Menu (File)`, `Unit (Joules / Calories)`, `Temperature (Kelvin / Celsius)`, `Database Selection (Main Database / Own Database)`, `Tools (Find by Elements / Diagrams)`, `Info (Help / About)`.
- Panel `Text Filters`: `Elements` (`Possible Species`), `Formula` (`TiO2`, `Start With`), `Stoichiometry` (`Free Ratio`), `Keywords` (`Structural Formula, Che...`).
- Panel `Type Filters`: `Select All`, `Gases`, `Gas Ions`, `Condensed`, `Electrons`, `Liquids`, `Aqueous Ions`, `Aqueous Neutrals`, `Fluids`.
- Panel `Organic Filter`: `Include Organics`, `Range Of Carbon Atoms`.
- Lista izquierda `Matching Species - 4`: `TiO2`, `TiO2(l)`, `TiO2(g)`, `TiO2(A)`.
- Panel `Basic Data`: `Formula: TiO2`, `Structural Formula: O2Ti`, `Chemical Name: Titanium(IV) oxide`, `Common Name: Rutile`, `CAS: 13463-67-7`, `Molecular Weight: 79.865 g/mol`, `Melting Point: 2116.000 K`, `Boiling Point: 3000.000 K`, `H° formation at 298.15: -944.747 kJ/mol`, `S° at 298.15: 50.291 J/(mol*K)`, `Cp at 298.15: 54.949 J/(mol*K)`, `ΔG° at 298.15: -889.417 kJ/mol`.
- Panel `Temperature Ranges` con fórmula explícita $C_p(T) = A + BT\cdot 10^{-3} + CT^{-2}\cdot 10^5 + DT^2\cdot 10^{-6}$ y tabla de rangos (Range 1: `298.15–2143.00 K`, fase `s`, `H=-944.747`, `S=50.292`, `A=70.195`, `B=7.573`, `C=-15.439`, `D=-1.523`, `Density=4.230 kg/l`, `Color=7`, `Ref=Barin 93; Sax`, `Reliability Class=1`; Range 2: `2143.00–3000.00 K`, fase `l`, `H=66.944`, `S=31.430`, `A=100.416`).
- Barra inferior: `Selected Species - 1` (`TiO2`), botón `Import species`.
