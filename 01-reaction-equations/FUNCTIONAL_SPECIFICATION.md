# FUNCTIONAL_SPECIFICATION — Reaction Equations Module (Fase 2)

> [!NOTE]
> Este documento distingue estrictamente entre: **(a)** información documentada en el manual oficial, **(b)** comportamiento observado en capturas y archivos de salida reales (`.ORE`, `.REA`, `Maindb6.HSC`), **(c)** evidencia de análisis estático (REA, `HSC6.DLL` TypeLib, `HSC_Test1.frm`) y **(d)** hipótesis pendientes de verificación en tiempo de ejecución.

---

## 1. Inventario de la Instalación Original (`/home/nando/Descargas/HSC6`)

Inspección en **solo lectura** de la instalación original de HSC Chemistry 6.0 (2006, Outokumpu Research Oy):

### 1.1. Ejecutables, Bibliotecas y Controles ActiveX (PE32 x86)
- **Ejecutable principal:** `HSC.EXE` (5,595,136 bytes, PE32 i386, VB6 nativo con `MSVBVM60.DLL` / `VBA6.DLL`). Contiene el menú principal y los módulos clásicos, incluido **Reaction Equations**.
- **Motor compartido COM/ActiveX:** `HSC6.DLL` (897,024 bytes, PE32 DLL i386). Expone las interfaces `_HSC`, `_Gibbs`, `_F1Functions` y `_HSCSub` (`DllGetClassObject`, `DllRegisterServer`, etc.) utilizadas por los módulos y complementos de Excel (`AddIns/HSC6.XLL`).
- **Módulos auxiliares ejecutables:** `HscData.exe` (gestor de BD), `Gibbs5.dll` + `GIBBS.EXE` (equilibrio multifase), `EPH.EXE` (diagramas Eh-pH), `HeatLoss.exe`, `HscGeo.exe`, `Map.exe`, `PIC.EXE`, `PSD.EXE`, `Sim.exe` (simulador de diagramas de flujo), `HSCHelp.exe`, `HSCImage.exe`, y heredados de 16 bits (`CELL.EXE`, `SGM.EXE`).
- **Controles ActiveX (`.ocx`):** `HSC_Converter.ocx`, `HSCData.ocx`, `HSCPeriodicTable.ocx`.

### 1.2. Archivos de Configuración, Bases de Datos y Ejemplos del Módulo
- **Configuración:** `HSC6.INI` (rutas de servidor/terminal, `OwnDB=C:\HSC6\Databases\OwnDB6.HSC`, `MainDB=C:\HSC6\Databases\MainDB6.HSC`, fuentes de tabla/diagrama, unidades por defecto `Temperature=C`, `Energy=cal`), `HSC.INI`, `CELL.INI`, `Sim.INI`, etc.
- **Bases de datos (`Databases/`):**
  - `Maindb6.HSC` (9,353,472 bytes = **36,537 registros** fijos de 256 bytes), acompañada de índice `Maindb6.cat` y referencias `MainDB6.REF`.
  - `OWNDB6.HSC` (256 bytes) + `OWNDB6.CAT`.
  - `ELEMENT4.ELE` (pesos atómicos y propiedades elementales), `UNITS4.UNI` (conversiones de unidades), `AWDB4.HSC`.
- **Carpeta específica del módulo (`Reaction/`):**
  - Archivos binarios `.REA` (formato BIFF2): `2Cu(a).REA`, `Na(a).REA`.
  - Archivos de texto de salida `.ORE` (reportes tabulares legacy): `AL3.ORE`, `AUCN.ORE`, `CU.ORE`, `FES2.ORE`, `KCLO4.ORE`, `NI.ORE`, `PB3O4.ORE`.
- **Ejemplos oficiales de API VB6 (`HSC DLL Tools/VB6/`):**
  - `HSC_Test1/HSC_Test1.frm` y `HSC_Test2/HSC_Test2.frm` demuestran llamadas directas a `HSC.Units(unitT, unitP, unitE)`, `HSC.CP(spec, T, CP)`, `HSC.H`, `HSC.S`, `HSC.G`.
- **Documentación oficial (`Help/`):** 50 pares `.doc`/`.txt`, destacando `08 Introduction.txt`, `10 Reaction Equations.txt`, `28 HSC - Databases.txt` y `32 References.txt`.

---

## 2. Gramática y Sintaxis Química Soportada *(Documentado en Manual Cap. 10 y 28)*

### 2.1. Reglas de Notación de Especies
1. **Elementos y Compuestos Puros:**
   - Sensible a mayúsculas/minúsculas para símbolos atómicos (`Co` vs `CO`, `Fe`, `Al2O3`, `Na2SO4`).
   - Paréntesis anidados y coeficientes estequiométricos decimales o enteros en fórmulas (`Ca(OH)2`, `(NH4)2SO4`, `Ni(CO)4`).
2. **Sufijos de Fase de Agregación:**
   - Sin sufijo o `(s)`: Fase sólida estándar (ej. `Ni`, `FeS2`, `TiO2`).
   - `(l)`: Fase líquida (ej. `TiO2(l)`).
   - `(g)`: Fase gaseosa (ej. `CO(g)`, `Ni(CO)4(g)`, `Mg(g)`, `H2S(g)`).
   - Alótropos/polimorfos: `(A)`, `(B)`, `(G)`, `(D)` (ej. `TiO2(A)`, `Fe2SiO4(B)`).
3. **Especies Acuosas, Iónicas y Electrones:**
   - `(a)`: Especie neutra acuosa estándar (`ao` en base de datos).
   - `(ia)`: Electrolito acuoso neutro disociado (`ai` en base de datos, ej. `MgSO4(ia)`).
   - Iones con carga explícita: `(+a)`, `(-a)`, `(+2a)`, `(-2a)`, `(+3a)` (ej. `Cu(+a)`, `Cu(+2a)`, `Cl(-a)`, `SO4(-2a)`, `H(+a)`, `Al(+3a)`).
   - Electrón libre para semirreacciones electroquímicas: `e-` (masa molar aparente observada en Fig. 9: `0.001 g/mol` / $\approx 0.00054858\,\text{g/mol}$).

### 2.2. Sintaxis de Ecuaciones de Reacción
- Estructura: $\sum \nu_{r,i}\,\text{Reactivo}_i = \sum \nu_{p,j}\,\text{Producto}_j$.
- Separador de igualdad en HSC: `=` (Soryntel soportará además `->` y `<=>` como alias normalizados).
- Términos separados por `+` con espacios opcionales.

---

## 3. Algoritmo de Balance Estequiométrico Automático (`Balance`) *(Documentado + Análisis Funcional)*

1. **Matriz de Conservación Elemental y de Carga ($A \cdot \nu = 0$):**
   - Cada columna $j \in \{1..N\}$ corresponde a una especie de la ecuación (signo negativo en reactivos, positivo en productos).
   - Cada fila $i \in \{1..M\}$ corresponde a un elemento químico presente; si existen especies cargadas o `e-`, se añade una **fila adicional de conservación de carga eléctrica**.
2. **Resolución Exacta:**
   - Mediante eliminación gaussiana sobre aritmética racional (`Rational64`) o SVD, se determina el espacio nulo $\ker(A)$.
   - **Caso $\dim(\ker(A)) = 1$:** Solución única. Se escala el vector para que el menor coeficiente positivo iguale `Smallest Coefficient` (por defecto `1`). Ejemplo verificado (Figs. 7–8):
     `FeS2 + H2O = Fe2O3 + SO4(-2a) + H(+a) + e-` $\longrightarrow$ `2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-`.
   - **Caso $\dim(\ker(A)) > 1$:** Sistema subdeterminado (número de incógnitas > número de elementos independientes). El manual indica explícitamente que `Balance` no puede resolverlo y debe notificarse al usuario.
   - **Caso $\dim(\ker(A)) = 0$:** Ecuación químicamente inválida (elementos o carga desbalanceados entre reactivos y productos).
3. **Validación previa a `Calculate`:**
   - Si el usuario introduce coeficientes manuales desbalanceados, el motor verifica $A \cdot \nu = 0$ y reporta error de balance elemental antes de calcular.

---

## 4. Lógica de Búsqueda en Bases de Datos y Reglas de Cálculo

### 4.1. Jerarquía `OwnDB` $\to$ `MainDB` *(Documentado)*
- Para cada especie se consulta primero `OwnDB` (`OWNDB6.HSC`); solo si no existe coincidencia exacta se busca en `MainDB` (`Maindb6.HSC`).
- En `Maindb6.HSC`, las especies pueden tener múltiples registros contiguos por intervalos de temperatura `[t1, t2]`. Según el análisis de `hsc-inventory.md`, ~75 especies (ej. `Al(+3a)`, `Ba(+2a)`) poseen más de un conjunto de datos; el motor debe seleccionar un único conjunto contiguo consistente.

### 4.2. Comportamiento de Modos y Opciones *(Documentado + Observado en Figs. 1, 5, 9 y `AL3.ORE`)*
- **Normal vs Delta (solo sustancia individual):**
  - `Normal`: $C_p, H, S, G$ absolutos (convención Barin-Knacke-Kubaschewski).
  - `Delta`: $\Delta H_f, \Delta S_f, \Delta G_f$ respecto a los elementos en su estado de referencia a $T$ (convención NBS/Pankratz). Para iones acuosos incorpora $\frac{z}{2} G(\text{H}_2(g)) - z G(\text{H}^+(a))$.
  - En ecuaciones de reacción, `Delta Format` y `Show Transitions` no alteran la selección de formato porque $\Delta H_r, \Delta S_r, \Delta G_r$ ya son diferencias por definición.
- **Extrapolación de Temperatura y Criss-Cobble:**
  - Si $T$ excede $T_{\max}$ registrado en la BD, se evalúa el polinomio del último tramo y se registra la temperatura límite en la columna `Extrapolated From T(K)` (observado en Fig. 1, Fig. 5 y `AL3.ORE`: `Al(+3a) Extrapolated from 398.15 K`, `H2O Extrapolated from 500 K`).
  - Si `Criss-Cobble` está activo y la especie acuosa carece de coeficientes $B, C, D$, se extrapola $C_p$ hasta $300\,^\circ\text{C}$ aplicando la conversión a entropía absoluta de ion hidrógeno ($-5\,\text{cal}/(\text{mol}\cdot\text{K})$).
- **Cálculos de Tabla Inferior `Species Data`:**
  - `wt-%`: fracción másica normalizada independientemente para reactivos (suma 100%) y productos (suma 100%). Verificado en `AL3.ORE` y Fig. 9.
  - `Volume`: para condensados, $V = m / \rho$ en `ml` (verificado en `Al2O3`: $101.961 / 3.965 = 25.715\,\text{ml}$); para gases, volumen molar ideal CNTP $22.414\,\text{l}/\text{mol}$ (verificado en `Mg(g)`: $1.000\,\text{mol} \to 22.414\,\text{l}$); para iones acuosos, `0.000 ml`.

---

## 5. Formatos de Datos y Persistencia

### 5.1. Formato de Base de Datos `.HSC` *(Análisis Estático Verificado)*
Registros de tamaño fijo de **256 bytes (`0x100`)**, little-endian (`Maindb6.HSC` = 36,537 registros):
- `0x00..0x18`: `formula` (`char[24]`)
- `0x98..0xAC`: `mw`, `mp`, `bp`, `t1`, `t2` (`5 × f32 LE`, temperaturas en K)
- `0xAC..0xAE`: `phase` (`char[2]`: `s `, `l `, `g `, `ao`, `a `, `ai`)
- `0xAE..0xC6`: `h`, `s`, `a`, `b`, `c`, `d` (`6 × f32 LE` en **kcal/mol**, **cal/(mol·K)**; conversión exacta verificada a SI: `1 cal = 4.184 J`)
- `0xC6..0xCA`: `density` (`f32 LE`)
- `0xDA..0xFE`: `reference` (`char[36]`)

### 5.2. Formatos Legados de Entrada/Salida (`.REA` y `.ORE`) *(Observado Directamente)*
- **`.REA` (`2Cu(a).REA`, `Na(a).REA`):** Contenedor binario **Microsoft Excel 2.0 BIFF2** con la tabla de resultados (`T`, `deltaH`, `deltaS`, `deltaG`, `K`, `Log(K)`) y el bloque `Species Data` (`Formula`, `FM`, `Conc. wt-%`, `Amount mol`, `Amount g`, `Volume l or ml`).
- **`.ORE` (`AL3.ORE`, `CU.ORE`, etc.):** Archivo de texto plano ASCII con la ecuación, tabla tabular (`T`, `delta H`, `delta G`, `K`), avisos de extrapolación (`Extrapolated from ... K`) y tabla de masas/volúmenes.

### 5.3. Formato Nativo Soryntel (`.simjson`)
Soryntel Chemistry implementará un importador de solo lectura para `.REA`/`.ORE` y guardará proyectos nativos en `.simjson` (JSON validado con `serde`):
```json
{
  "schema_version": "1.0.0",
  "module": "reaction_equations",
  "settings": {
    "temperature_unit": "Celsius",
    "energy_unit": "Kilojoules",
    "result_format": "Normal",
    "show_transitions": true,
    "criss_cobble": false,
    "temperature_range": { "from": 0.0, "to": 175.0, "step": 25.0 },
    "smallest_coefficient": 1.0
  },
  "reaction_input": "2FeS2 + 19H2O = Fe2O3 + 4SO4(-2a) + 38H(+a) + 30e-"
}
```

---

## 6. Hipótesis Pendientes de Verificación en Ejecución
- Comportamiento exacto de la interfaz gráfica clásica de `HSC.EXE` bajo Wine en este host (mensajes de error exactos en pantalla ante fórmulas ambiguas o coeficientes manuales erróneos), ya que toda la investigación actual se ha realizado mediante análisis estático y documentación oficial sin modificar ni ejecutar el binario legado.
