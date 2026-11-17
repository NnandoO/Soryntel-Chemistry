# Análisis REA — Módulo 15: Excel Add-Ins

## 1. Hallazgos en HSC Chemistry 6 (`/home/nando/Descargas/HSC6/`)

### 1.1 Artefactos Inspeccionados
- **`Help/27 Excel Add-Ins.txt` (`8 099 bytes`):** Documenta la activación en 3 etapas en Excel (`Tools -> Add-Ins -> AddIns/HSC6.XLL`, alternativa vía VBA References / `Edit -> Links`), la migración desde `HSC5.XLA` (`27.2`), las familias de funciones (`27.3`), el uso matricial de `StreamEQ` con `Ctrl+Shift+Enter` (`27.4`), las funciones internas exclusivas de HSC-Sim (`27.5`: `StreamX`, `RecoveryX`, `ParticleRecX`, `FractionX`, `MineralX`) y el registro manual con `regsvr32 HSC6.DLL` (`27.6`).
- **`AddIns/HSC6.XLL` (`188 416 bytes`, PE32):** Biblioteca de interfaz nativa para Microsoft Excel (C API `XLOPER`). Exporta `77` símbolos: `4` de ciclo XLL/COM (`xlAutoOpen`, `xlAutoClose`, `xlAutoFree`, `xlAddInManagerInfo`, `DllCanUnloadNow`, `DllGetClassObject`, `DllRegisterServer`), `CheckInstance`, y **`69` funciones de hoja `My*`** registradas en una tabla estática en `.data` (registros de `36` bytes cada uno) bajo la categoría `"HSC"`.
- **`HSC6.DLL` (`897 024 bytes`, PE32):** Unifica las antiguas `HSC5.dll` y `Gibbs5.dll` (como documenta `Help/35 HSC DLL Tools.txt`). `HSC6.XLL` delega en `HSC6.DLL` vía COM/ActiveX (`OleInitialize`), mientras que el registro en Excel deja solo el nombre simple de la función sin incrustar rutas absolutas en las celdas.
- **`AddIns/AddInSample.xls` (`43 520 bytes`, BIFF8):** Libro de trabajo con `Sheet1` (`122` filas) que recopila ejemplos vivos, argumentos en rojo, fórmulas evaluadas y descripciones para las funciones de hoja y el bloque matricial `StreamEQ`.
- **`AddIns/Update XLS Files.xls` (`27 136 bytes`, BIFF8 + VBA):** Macro VBA (`Update opened XLS files`) que recorre fórmulas de libros abiertos para eliminar referencias con ruta heredadas de HSC 5.1 tipo `='C:\HSC5\AddIns_BackUp\HSC5.xla'!H(D11;E11)` y convertirlas en llamadas portables `=H(D11;E11)` de `HSC6.XLL`.

---

### 1.2 Inventario Completo Verificado en `AddIns/HSC6.XLL` (`69` Funciones Excel)

La inspección directa de la tabla de registro (`xlfRegister`) en la sección `.data` de `HSC6.XLL` confirma **exactamente 69 funciones** registradas en categoría `"HSC"`, distribuidas en los siguientes grupos y firmas de tipos XLL (`R` = `LPXLOPER` rango/escalar, `C` = cadena C, `I`/`J` = entero corto/largo, `!` = volátil):

1. **Generales (3):**
   - `Species(DBNo, Pos)` (`RIJ` → `MySPECIES`)
   - `BAL(Equation)` (`RC` → `MyBAL`)
   - `Units(TUnit, EUnit)` (`RCC!` → `MyUnits`, marcada como **volátil** `!`)
2. **Especies y Ecuaciones de Reacción (9):**
   - `H(Target, T)`, `HCM(Target, T)`, `HNM3(Target, T)`, `HKG(Target, T)`, `S(Target, T)`, `G(Target, T)`, `K(Equation, T)`, `HLAT(Target, T)`, `CP(Target, T)` (`RCR` → `My*`).
   - *Nota funcional:* Funciones como `H`, `HKG`, `S`, `CP` y `G` aceptan polimórficamente tanto el nombre de una **especie** (p. ej. `"CO2(g)"`) como una **ecuación química** (p. ej. `"2H2(g) + O2(g) = 2H2O(g)"`). `K` evalúa la constante de equilibrio de reacción y `HNM3`/`HCM`/`HLAT` operan por $\text{Nm}^3$ o excluyendo transformaciones de fase respectivamente.
3. **Iteración Inversa — Temperatura desde Propiedad (8):**
   - `TatH`, `TatHNM3`, `TatHCM`, `TatHKG`, `TatHLAT`, `TatS`, `TatCP`, `TatG` (`RCR` → `MyTat*`).
4. **Propiedades Independientes de la Temperatura y Polinomio $C_p$ (20):**
   - Físicas e identificadores (`13`, firma `RC`): `MW`, `DE`, `REF`, `BP`, `MP`, `Struct`, `ChName`, `CoName`, `CAN`, `Phase`, `RGBColor`, `Solu`, `Rel`.
   - Polinomio de capacidad calorífica (`7`, firma `RC`): `CPA`, `CPB`, `CPC`, `CPD`, `CPFUNCTION`, `TMin`, `TMax`.
5. **Porcentajes Estequiométricos (2):**
   - `MolP(Species1, Species2)` (`RCC` → `MyMolP`)
   - `WTP(Species1, Species2)` (`RCC` → `MyWTP`)
6. **Corrientes y Flujos (`Flow*`, `Stream*`, `DensityA`) (15):**
   - `FlowH`, `FlowHLAT`, `FlowHKG`, `FlowHNM3`, `FlowS`, `FlowCP`, `FlowG`, `DensityA` (`8` funciones `RRRR` → `MyFlow*` / `MyDensityA`)
   - `StreamH`, `StreamHLAT`, `StreamHKG`, `StreamHNM3`, `StreamS`, `StreamCP`, `StreamG` (`7` funciones `RRRR` → `MyStream*`)
7. **Iteración Inversa de Corrientes y Flujos (`StreamT*` y `FlowT*`) (11):**
   - `StreamTH`, `StreamTHKG`, `StreamTHLAT`, `StreamTHNM3`, `StreamTS`, `StreamTCP`, `StreamTG` (`7` funciones `RRRRRR`)
   - `FlowTH`, `FlowTS`, `FlowTCP`, `FlowTG` (`4` funciones `RRRRRR`)
8. **Equilibrio Matricial de Corrientes (1):**
   - `StreamEQ(SpeciesRange, InputAmountRange, T, P)` (`RRRRR` → `MyStreamEQ`, devuelve vector vertical con temperaturas de fase y moles en equilibrio para bloques delimitados por prefijo `§`).

> **Resumen exacto del conteo en `HSC6.XLL`:** `3 + 9 + 8 + 20 + 2 + 15 + 11 + 1 = 69` funciones exportadas y registradas.

---

### 1.3 Particularidades, Anomalías y Restricciones Heredadas Detectadas
1. **Diferencias entre `AddInSample.xls`, `27 Excel Add-Ins.txt` y `HSC6.XLL`:**
   - En `AddInSample.xls` (fila 2 titulada aún `"HSC Chemistry 5.1 add-in functions in MS Excel 2000"`), la celda `B75` estaba etiquetada por errata como `'FLOWTH(Species;Amount;H;Tmin;TMax)'` aunque correspondía a `FLOWTCP` (entre `STREAMTCP` en fila 74 y `STREAMTG` en fila 77).
   - En `AddInSample.xls` fila 8, el valor cacheado de `BAL("H2(g)+O2(g)=H2O(g)")` contenía un texto residual de depuración (`"H2(g)+O2(g)=H2O(g)qweasd"`).
   - La función `Solu(Species)` figura registrada en `HSC6.XLL` (`RC`, índice 30), aunque no aparece en `AddInSample.xls`.
   - `HSC6.XLL` implementa `FlowTH`, `FlowTS`, `FlowTCP`, `FlowTG`, pero omite `FlowTHKG`, `FlowTHLAT` y `FlowTHNM3` (que sí existen con prefijo `StreamTH*`).
2. **Restricción de instancia única en `HSC6.XLL`:** `CheckInstance` muestra el aviso *"Unable to complete calculation with other instances of Excel Opened"*, limitación COM clásica que desaparece completamente al implementar el evaluador en Rust dentro de `soryntel`.
3. **Semántica `Units(T, E)` en `AddInSample.xls`:** En `AddInSample.xls` se usó `UNITS("C", "MJ")`, por lo que sus entalpías y energías libres en `Sheet1` están expresadas en **MJ/kmol** o **MJ/kg** (`CO2(g)` a `100 °C` → `-390.5929 MJ/kmol`), mientras que las **entropías `S` y capacidades caloríficas `CP`** en ese ejemplo muestran valores numéricos como `205.166` (`CH4(a)`, `J/(mol·K)` ≡ `kJ/(kmol·K)` ≡ `MJ/(Mmol·K)`), razón por la que la descripción en `AddInSample.xls` indica explícitamente `"per Mmol"`.
4. **Funciones exclusivas de HSC-Sim / Mineral Based Models (`27.5` y `57`):** `StreamX`, `RecoveryX`, `ParticleRecX`, `FractionX` y `MineralX` no están en `HSC6.XLL` (solo operan dentro del entorno HSC-Sim cuando existen modelos de partículas y minerales).

---

### 1.4 Encaje en la Arquitectura Hexagonal (`soryntel`)
- **Dominio (`soryntel-thermo::excel_addins`):**
  - Estado de unidades de sesión (`ExcelAddonUnits`: `C`/`K` y `kJ`/`MJ`/`kcal`/`Mcal`/`kWh`), catálogo descriptivo de las `69` funciones de `HSC6.XLL` (+ `5` funciones `HSC-Sim`), evaluador polimórfico especie/reacción, resolutor inverso `Tat*` / `StreamT*` / `FlowT*`, `DensityA`, `MolP`/`WTP`, formateador `CPFUNCTION`, evaluador matricial `StreamEQ` (delegando el solver de Gibbs en `equilibrium::solve_equilibrium`) y migrador de fórmulas `HSC5.xla -> HSC6`.
  - Evaluador ligero de hojas (`ExcelFormulaEvaluator`) capaz de resolver expresiones `=FUNC(...)`, referencias de celda (`A1`, `D34:D36`), aritmética básica y funciones matriciales `StreamEQ`.
- **Aplicación (`soryntel-application`):**
  - Casos de uso `evaluate_addin_call`, `evaluate_streameq_block`, `list_addin_functions`, `default_addin_sample_sheet` y `migrate_legacy_hsc5_formulas`.
- **Adaptador Primario (`soryntel-desktop::excel_addins_ui`):**
  - Ventana dedicada **"Excel Add-In Functions (`HSC6.XLL`)"** accesible desde el Launcher (`15`) con hoja de cálculo interactiva precargada con los casos de `AddInSample.xls`, explorador/probador de las `69` funciones, laboratorio matricial `StreamEQ` con bloques `§ PHASE` y convertidor/migrador de fórmulas heredadas.
