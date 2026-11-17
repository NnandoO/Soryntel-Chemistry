# Análisis REA — Módulo 16: Species & Thermochemical Databases

## 1. Artefactos de HSC Chemistry 6 Inspeccionados
Todos los hallazgos provienen de la inspección directa del paquete `/home/nando/Descargas/HSC6/`:

- **`Databases/Maindb6.HSC` (`9 353 472 bytes`):** Archivo binario de longitud de registro fija (**`256` bytes/registro**, exactamente **`36 537` registros**). Contiene **`21 090` fórmulas únicas** (además de `97` ranuras marcadas como `'0'`). Cada fórmula puede ocupar $1\dots N$ registros contiguos o alternativos para distintos rangos de temperatura o fuentes de literatura (máximo observado: `ZrTiO4` con `22` registros, `Ar(100bar)` con `11` registros).
  - **Distribución observada por código de fase (`0xAC..0xAD`):** gases `'g'`: `15 494`; líquidos `'l'`: `8 716`; sólidos `'s'`: `6 335`; acuosos no disociados/iones `'ao'`: `3 598`; electrolitos acuosos neutros `'ai'`: `2 102`; acuosos heredados `'a'`: `182`; marcadores/atípicos (`'0'`, `''`, etc.): `110`.
  - **Colores asignados (`0xCA..0xCB`):** `1 830` registros tienen código de color distinto de cero (`49` códigos usados de la tabla `1..53`).
  - **Clases de fiabilidad (`0xFE..0xFF`):** Clase `1` (`25 796`), Clase `2` (`9 870`), Clase `3` (`767`), Clase `0` (`102`), Clase `5` (`2`).
- **`Databases/OWNDB6.HSC` (`256 bytes`):** Archivo vacío inicial (`1` registro cabecera/centinela `'0'`), extensible hasta `60 000` especies definidas por el usuario.
- **`Databases/Maindb6.cat` (`1 599 bytes`) y `OWNDB6.CAT` (`812 bytes`):** Índices alfabéticos por rango de registro (23 índices de salto en `Maindb6.cat`: `1, 23, 222, ..., 20495`) que permiten búsqueda rápida sobre archivos ordenados por fórmula.
- **`Databases/MainDB6.REF` (`277 764 bytes`):** Diccionario binario/texto de longitud fija de **`293` bytes/registro** (`948` referencias bibliográficas exactas): `36` bytes clave abreviada (p. ej. `Abbasov 87`) + `256` bytes cita completa + `1` byte indicador (`'0'`/`'1'`).
- **Bases heredadas incluidas en `Databases/`:** `MAINDB3.HSC` (`3 558 656 B`), `MAINDB4.HSC` (`5 086 976 B`), `Maindb5.HSC` (`6 552 832 B`), `AWDB4.HSC` (`2 440 B` — tabla interna de pesos atómicos).
- **`Help/28 HSC - Databases.txt` (`46 516 bytes`, `445 líneas`):** Manual completo de las `18` ventanas del módulo (`28.1–28.18`).
- **`Help/34 Species.txt` (`272 908 bytes`, `21 197 líneas`):** Listado maestro alfabético de especies (`21 196` líneas terminando en `0`).
- **`Help/31 Thermochemical Data Sources.txt` y `Help/33 Data References.txt`:** Criterios editoriales y catálogo de citas.

---

## 2. Esquema Binario Exacto del Registro `.HSC` (`256` bytes, Little-Endian / Windows-1252)

La inspección binaria campo por campo sobre `Maindb6.HSC` revela el mapa completo de los **19 campos** descritos en la sección `28.11` del manual:

| Offset Hex | Longitud | Tipo | Campo en HSC6 Editor | Unidad / Notas de Formato |
|---|---|---|---|---|
| `0x00 .. 0x17` | `24` | `char[24]` | **Formula** | CP1252 / ASCII, rellenado con espacios o NUL. Máx 24 caracteres |
| `0x18 .. 0x37` | `32` | `char[32]` | **Structural Formula** | Fórmula estructural o sinónimo |
| `0x38 .. 0x6B` | `52` | `char[52]` | **Chemical Name** | Nombre químico sistemático |
| `0x6C .. 0x8B` | `32` | `char[32]` | **Common Name** | Nombre mineralógico o comercial |
| `0x8C .. 0x97` | `12` | `char[12]` | **Chemical Abstract Number (CAS)** | Identificador CAS (p. ej. `22205-45-4`) |
| `0x98 .. 0x9B` | `4` | `f32 LE` | **Molecular Weight (`MW`)** | `g/mol` |
| `0x9C .. 0x9F` | `4` | `f32 LE` | **Melting Point (`Mp`)** | `K` (`< 0` indica descomposición antes de fundir) |
| `0xA0 .. 0xA3` | `4` | `f32 LE` | **Boiling Point (`Bp`)** | `K` (`< 0` indica descomposición antes de ebullir) |
| `0xA4 .. 0xA7` | `4` | `f32 LE` | **Lower Temp Limit (`T1`)** | `K` |
| `0xA8 .. 0xAB` | `4` | `f32 LE` | **Upper Temp Limit (`T2`)** | `K` |
| `0xAC .. 0xAD` | `2` | `char[2]` | **Phase** | `"s "`, `"l "`, `"g "`, `"ao"`, `"ai"` (o legado `"a "`) |
| `0xAE .. 0xB1` | `4` | `f32 LE` | **Enthalpy (`H`)** | **`kcal/mol`**: en 1er intervalo $\Delta_f H^\circ(298.15\text{ K})$; en siguientes $\Delta_{tr}H(T_{tr})$ |
| `0xB2 .. 0xB5` | `4` | `f32 LE` | **Entropy (`S`)** | **`cal/(mol·K)`**: en 1er intervalo $S^\circ(298.15\text{ K})$; en siguientes $\Delta_{tr}S$ |
| `0xB6 .. 0xB9` | `4` | `f32 LE` | **Kelley `A`** | **`cal/(mol·K)`** — término constante de $C_p$ |
| `0xBA .. 0xBD` | `4` | `f32 LE` | **Kelley `B`** | **`10⁻³ cal/(mol·K²)`** — coeficiente de $T$ |
| `0xBE .. 0xC1` | `4` | `f32 LE` | **Kelley `C`** | **`10⁵ cal·K/mol`** — coeficiente de $T^{-2}$ |
| `0xC2 .. 0xC5` | `4` | `f32 LE` | **Kelley `D`** | **`10⁻⁶ cal/(mol·K³)`** — coeficiente de $T^2$ |
| `0xC6 .. 0xC9` | `4` | `f32 LE` | **Density** | **`kg/l` (`g/cm³`)** en condensados; **`g/l`** en gases |
| `0xCA .. 0xCB` | `2` | `i16 LE` | **Color Code** | Índice `0..53` según tabla `28.18` (`0` = sin dato) |
| `0xCC .. 0xD9` | `14` | bytes/f32 | **Solubility / Reservado** | Reservado (`0x00` en la mayoría; `132` registros con bytes auxiliares en `Maindb6.HSC`) |
| `0xDA .. 0xFD` | `36` | `char[36]` | **Reference** | Claves separadas por comas (p. ej. `Barin 89, SGTE 94, Landolt 00`) |
| `0xFE .. 0xFF` | `2` | `i16 LE` | **Reliability Class** | Entero `1` (máxima fiabilidad) a `10` (`0` = sin clasificar) |

> **Hallazgo clave para el escritor de `OwnDB`:** HSC6 almacena **siempre** $H$ en `kcal/mol`, $S$ y $(A,B,C,D)$ en escala `cal/(mol·K)` y $T_1, T_2, M_p, B_p$ en Kelvin dentro del archivo `.HSC`, sin importar si la interfaz muestra Joules o Calorías.

---

## 3. Reglas de Nomenclatura Química y Precedencia de Bases de Datos

1. **Precedencia `OwnDB` > `MainDB`:** Si una fórmula existe en ambas bases, los módulos de cálculo y búsquedas resuelven primero la definición de `OwnDB`.
2. **Sintaxis de fórmulas (`28.2.1`, 24 caracteres máx.):**
   - Sin subíndices ni superíndices; sin paréntesis anidados (`H2(Sn(OH)6)` → `H2Sn(OH)6`, `AlO(OH)` → `AlO*(OH)` o `AlO2H`).
   - El último paréntesis `(...)` está reservado para **identificador cristalino/isómero + carga + estado físico**:
     - Gas: siempre termina en `g)`, p. ej. `O2(g)`, `Ar(+g)`, `C7H6N2O4(26DNTg)`.
     - Acuoso: `ao` termina en `a)` (`H(+a)`, `SO4(-2a)`, `CH4(a)`); electrolitos neutros compuestos `ai` terminan en `ia)` (`MgSO4(ia)`, `MnCl2(ia)`).
     - Líquido: normalmente comparte fórmula con el sólido como intervalo superior (`Cu`, `Cu2S`), o usa `(l)` explícito (`H2O(l)`, `Fe(l)`) con extrapolación matemática a `298.15 K`. Si se pide `X(l)` y no existe registro propio, HSC puede extrapolar la fase líquida de `X` hasta `298.15 K` y guardarla en `OwnDB`.
     - Sólido: sin sufijo `s)` salvo polimorfos (`C`, `C(D)`, `FeS2`, `FeS2(M)`).
   - **Prefijo numérico prohibido:** fórmulas que inician con dígito estequiométrico requieren `*` inicial (`*2MgO*SiO2`, `*2FeO*SiO2`).
   - **Carga e identificador:** el último carácter antes de `g)`/`a)` no puede ser dígito si forma parte del nombre (el sufijo numérico con `+`/`-` indica carga neta: `+2`, `-2`, `+`, `-`).

---

## 4. Auditoría de Brecha en Soryntel Actual

La inspección de `crates/soryntel-db/src/lib.rs` y `crates/soryntel-desktop/src/main.rs` (`show_browser_ui`, líneas `1797–1985`) muestra lo que **ya existe** y lo que **falta implementar**:

| Capacidad | Estado actual en Soryntel | Objetivo del Módulo 16 (Paridad HSC6 + Mejoras) |
|---|---|---|
| Lectura de `Maindb6.HSC` | Parcial: lee `Formula`, `Chemical Name`, `MW`, `T1`, `T2`, `Phase`, `H`, `S`, `A..D`, `Density`, `Reference` | Leer los **19 campos completos** (`Structural Formula`, `Common Name`, `CAS`, `Mp`, `Bp`, `Color 0–53`, `Solubility`, `Reliability Class`) |
| Gestión dual `OwnDB` + `MainDB` | Inexistente (solo `ThermoDatabase` único en memoria) | Dual `MainDB` + `OwnDB` editable con precedencia `OwnDB`, creación de nueva DB y persistencia `.HSC` / JSON |
| Registros múltiples / duplicados | Descarta tras discontinuidad `(t1 - pt2).abs() > 1.0` | Exponer fuentes alternativas (como `Mg(+2a)` con 2 bloques en `Maindb6.HSC`) sin perder trazabilidad |
| Búsqueda (`28.5–28.9`) | Solo texto simple (`formula`/`name`) y `search_by_elements` en backend | Búsqueda gráfica por tabla periódica (`All`/`Some`, filtro orgánico C-limit), **estequiometría (`Fix Ratio`)** y selector de campo (`Formula`, `Structural`, `Chem Name`, `Common Name`, `CAS`) |
| Editor CRUD + Validación (`28.11`) | Inexistente en UI | Editor completo de especie e intervalos en `OwnDB`, auto-`MW`, $\Delta_{tr}S = \Delta_{tr}H / T_{tr}$ |
| Ingeniería de Datos (`28.12–28.14`) | Inexistente | **`Cp Fitting`**, **`Convert Cp Functions`** y **`Convert G(T) / ΔG(T) -> H, S, Cp`** con `Delta to Normal` |
| Diccionario `MainDB6.REF` | No cargado | Cargar las `948` referencias completas al inspeccionar el campo `Reference` |
| Criss-Cobble y propiedades calculadas | En `soryntel-thermo` solo cálculo básico | Previsualización en vivo $C_p(T), H(T), S(T), G(T)$, gráfico interactivo por rango y toggle Criss-Cobble para iones |
