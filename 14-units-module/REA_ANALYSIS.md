# Análisis REA — Módulo 14: Units Module

## 1. Hallazgos en HSC Chemistry 6 (`/home/nando/Descargas/HSC6/`)

### 1.1 Documentación (`Help/26 Units Module.txt`, `06 Whats New in HSC 5.txt`)
- En cálculos termoquímicos conviven múltiples sistemas de unidades (calorías termoquímicas vs IT, `Btu`, `kWh`, `bar`, `atm`, `mmHg`, `psi`, escalas térmicas, unidades imperiales e internacionales), lo que exige conversiones frecuentes y trazables.
- El módulo **Units** se organiza como un **libro multicapa de 6 hojas** editables por el usuario (`File Save` persiste personalizaciones), con soporte completo de formato, edición y redimensionamiento de formulario:
  1. **`Convert`**: calculadora interactiva donde el usuario selecciona la **Magnitud** (*Quantity*, p. ej. `Energy, work`), elige **Unidad + Prefijo** en las columnas 1–2 (origen) y en las columnas 3–4 (destino), e ingresa un valor numérico en cualquiera de los dos lados para obtener automáticamente su equivalente en el otro.
  2. **`Units`**: base de datos tabulada de **90 magnitudes** (`Q0..Q89`) y **~441–444 unidades** (hasta 255 unidades por magnitud en columnas), organizada estrictamente en **bloques de 3 filas por magnitud**.
  3. **`Constants`**: tabla de **56 constantes físico-químicas fundamentales** (`Absolute zero temperature`, `Avogadro constant`, `Boltzmann constant`, `Faraday constant`, `Molar gas constant`, `Planck constant`, `Speed of light`, etc.) con nombre, símbolo, valor y unidad SI.
  4. **`Mesh`**: tabla estándar de granulometría / tamices (`mesh` vs apertura en `mm`, 15 pares desde `6 mesh = 3.36 mm` hasta `400 mesh = 0.037 mm`).
  5. **`Air`**: composición del aire seco (excluyendo $\text{H}_2\text{O}$: $\text{N}_2, \text{O}_2, \text{Ar}, \text{CO}_2, \text{Ne}, \text{He}, \text{CH}_4, \text{Kr}, \text{N}_2\text{O}, \text{H}_2, \text{O}_3, \text{Xe}$ en `Vol-%` y `Wt-%`) más dos tablas psicrométricas paralelas de **contenido de humedad en aire** para temperaturas de `-50 °C` a `102 °C` (paso `2 °C`) y humedad relativa de `10 %` a `100 %`: una en `H2O vol-%` y otra en `H2O g/Nm3` bajo suposición de gas ideal.
  6. **`H2O`**: tabla de referencia rápida **Vapour Pressure of Water** con 88 pares `(Temp. °C, kN/m2, atm)` desde `-90 °C` hasta `370 °C`.

### 1.2 Inspección Directa del Archivo Binario `Databases/UNITS4.UNI`
El archivo `Databases/UNITS4.UNI` (`84 311 bytes`, formato Excel/Formula One Workbook `BIFF/VTS`) confirma el esquema exacto empleado por HSC6:
- **Esquema en bloques de 3 filas en la hoja `Units`:**
  - **Fila `3k` (Nombres):** Columna A = Nombre de la magnitud (p. ej. `Energy, work`, `Pressure, stress`, `Mass`, `Temperature`, `Volume`, `Viscosity, dynamic`), Columna B = Nombre de la unidad de referencia (`joule`, `pascal`, `kilogram`, `kelvin`), Columnas C..IV = nombres de unidades alternativas (`British thermal unit(IT)`, `calorie (th)`, etc.).
  - **Fila `3k+1` (Símbolos):** Columna A = símbolo físico de la magnitud (`E`, `p`, `m`, `T`, `V`), Columna B..IV = abreviaturas de cada unidad (`J`, `Btu(IT)`, `cal(th)`, ...).
  - **Fila `3k+2` (Expresión SI y Factores):** Columna A = unidad base en forma dimensional SI (`m2 kg s-2`, `N m-2`, `m3`), Columna B = `1.0`, Columnas C..IV = factor numérico $c_u$ expresado como **número de unidades $u$ contenidas en `1` unidad de referencia** (p. ej. `1 Pa = 1e-5 bar = 9.86923e-6 atm`; `1 J = 0.2390057 cal(th)`).
- **Cobertura completa de las 90 magnitudes detectadas (`Q0..Q89`):**
  - *Mecánica y geometría*: `Acceleration`, `Angular acceleration`, `Angular velocity`, `Area`, `Density`, `Force`, `Length` (`Lenght` en el original), `Mass`, `Moment of force`, `Plane angle`, `Solid angle`, `Specific volume`, `Time`, `Velocity, speed`, `Volume`, `Wave number`.
  - *Termodinámica, química y transferencia*: `Amount of substance`, `Concentration`, `Energy, work`, `Energy density`, `Entropy`, `Heat capacity`, `Heat flux density, irradiance`, `Heat quantity`, `Heat transfer coefficient`, `Heat transfer coefficient, volumetric`, `Molality`, `Molar energy`, `Molar enthalpy`, `Molar entropy`, `Molar Gibbs function`, `Molar heat capacity`, `Molar mass`, `Molar thermodynamic energy`, `Molar volume`, `Rate of reaction`, `Specific enthalpy`, `Specific entropy`, `Specific heat capacity`, `Temperature`, `Thermal conductivity`, `Thermal flux`, `Thermal resistance`.
  - *Transporte de fluidos*: `Diffusivity`, `Mass transfer coefficient`, `Surface tension`, `Viscosity, dynamic`, `Viscosity, kinematic`.
  - *Electricidad y magnetismo*: `Capacitance`, `Charge, electric flux`, `Charge density`, `Conductance`, `Conductivity`, `Current (electric)`, `Current density`, `Dipole moment`, `Electric field strength`, `Electric flux density`, `Electric polarization`, `Electromotive force`, `Inductance`, `Magnetic field strength`, `Magnetic flux`, `Magnetic flux density, magnetic induction`, `Magnetic moment`, `Molar conductivity`, `Permeability`, `Permittivity`, `Potential`, `Resistance`, `Resistivity`, `Surface charge density`, `Surface current density`, `Volume resistivity`.
  - *Radiación, óptica y miscelánea*: `Absorbed dose`, `Absorbed dose (radiation)`, `Activity (radiation)`, `Dose equivalent`, `Frequency`, `Illuminance`, `Kerma`, `Light exposure`, `Luminance`, `Luminous flux`, `Luminous intensity`, `Power, radiant flux`, `Radiance`, `Radiant intensity`, `Currency 26.1.1999` (31 divisas históricas).

### 1.3 Anomalías Históricas Detectadas en `UNITS4.UNI` a Corregir en `soryntel`
1. **Errores tipográficos en etiquetas originales:** `"Lenght"`, `"Avogdaro constant"`, `"Spesific electron charge"`, `"lb cal-1(US)"` (para `lb gal-1(US)` en Densidad), `"furlong"`/`"ch(Gynter)"`, `"poundal"` dentro de `Mass` (aunque `poundal` es fuerza; debe depurarse o señalarse).
2. **Factor almacenado como cadena mal formada:** En `Density`, `lb in-3` figuraba en `UNITS4.UNI` como cadena `"3.6127292-5"` (sin la letra `e` científica); `soryntel` debe usar el flotante exacto `3.6127292e-5` ($\text{lb/in}^3 \text{ por kg/m}^3$).
3. **Caso especial de `Temperature`:** En `UNITS4.UNI`, `Celsius`, `Fahrenheit` y `Rankine` tienen coeficiente nulo/vacío en la fila `3k+2` porque no son conversiones puramente multiplicativas; deben evaluarse mediante transformación afín exacta.
4. **Caso especial del prefijo SI sobre `kilogram`:** La unidad base de masa en `UNITS4.UNI` es `kilogram` (`kg`, que ya contiene $10^3\text{ g}$). Aplicar prefijos SI (`m`, `µ`, `M`, etc.) debe respetar que un prefijo sobre el sistema gramo/kilogramo no produzca ambigüedades (`mg = 1e-6 kg`, `g = 1e-3 kg`, `kg = 1 kg`, `Mg/t = 1e3 kg`) o bien incluir `gram (g)` explícitamente sin doble prefijado erróneo.
5. **Constantes CODATA actualizadas / paridad:** `UNITS4.UNI` usa CODATA-1986 (`R = 8.3145107`, `NA = 6.022136736e23`, `po = 101300 Pa`). Se documentan y preservan los valores de referencia con soporte de precisión CODATA moderna en `soryntel-core`.

## 2. Rol en la Arquitectura Hexagonal (`soryntel`)
- **Dominio (`soryntel-thermo::units`):**
  - Catálogo tipado en memoria de las **90 magnitudes y ~444 unidades**, prefijos SI (`10^{24}..10^{-24}`), motor de conversión lineal y afín (`Temperature`), cálculo de **Constants**, tabla **Mesh** con buscador/interpolador `mesh ↔ mm / µm`, tablas **Air** (composición y humedad psicrométrica `T, RH → vol-%, g/Nm³`) y tabla **H2O** (integrada con `water::h2o_saturation_pressure_bar` y `sat_pressure_gs_bar` como fuente única).
  - Libro editable en memoria (`UnitsWorkbook`: filas personalizadas por el usuario en `Units`, `Constants`, `Mesh`, `Air`, `H2O`).
- **Capa de Aplicación (`soryntel-application`):**
  - Casos de uso `convert_unit_value`, `default_units_workbook`, `add_custom_unit`, `interpolate_mesh_size`, `compute_moist_air_humidity`.
- **Adaptador Primario (`soryntel-desktop::units_ui`):**
  - Interfaz de escritorio estilo HSC6 con menú `File | Edit | Format | Help`, barra de pestañas de las **6 hojas del libro** (`Convert | Units | Constants | Mesh | Air | H2O`), panel calculadora bidireccional en `Convert` con búsqueda rápida de magnitudes, selectores `Unit 1 / Prefix 1` y `Unit 2 / Prefix 2`, botón `Swap ↔`, y grillas Excel editables (`excel_grid.rs`) con exportación CSV/JSON.
