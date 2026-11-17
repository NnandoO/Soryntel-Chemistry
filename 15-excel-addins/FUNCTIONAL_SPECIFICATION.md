# Especificación Funcional — Módulo 15: Excel Add-Ins

## 1. Casos de Uso

### CU-01: Hoja Evaluadora Termoquímica (`Add-In Workbook`)
- **Actor:** Ingeniero de procesos / investigador / estudiante.
- **Flujo principal:**
  1. Al abrir el módulo, la hoja principal muestra la plantilla de referencia estilo `AddInSample.xls` con sus 9 grupos (`General`, `Species`, `Reaction equation`, `Iteration (reverse)`, `Stream/Flow`, `Stream/Flow iteration (reverse)`, `Temp. independent`, `Percentage`, `CP-function related`).
  2. El usuario puede editar cualquier argumento de entrada (celdas en rojo, ej. especie `"CO2(g)"`, ecuación `"2H2(g) + O2(g) = 2H2O(g)"`, temperatura `100.0`, flujos, presiones) o escribir nuevas fórmulas directamente en cualquier celda (`=H(D11;E11)`, `=K("CO+0.5O2=CO2"; 1200)`, `=StreamH(D34:D36; E34:E36; F34)`).
  3. El evaluador soporta tanto separador de argumentos europeo **`;`** (como en `27 Excel Add-Ins.txt` y `AddInSample.xls`) como **`,`**, sin sensibilidad a mayúsculas/minúsculas en el nombre de la función (`H`, `h`, `StreamH`, `STREAMH`).
  4. Al cambiar las unidades globales mediante `=UNITS("C"; "MJ")` o el selector superior (`°C`/`K`, `kJ`/`MJ`/`kcal`/`Mcal`/`kWh`), todas las celdas dependientes se recalculan automáticamente.

### CU-02: Explorador y Probador de las 69 Funciones (`Function Reference`)
- Lista interactiva completa de las **69 funciones registradas en `HSC6.XLL`** más las **5 funciones exclusivas de simulación (`StreamX`, `RecoveryX`, `ParticleRecX`, `FractionX`, `MineralX`)** documentadas en `27.5`.
- Filtrado por categoría (`All`, `General`, `Species`, `Reaction`, `Reverse Tat*`, `Stream / Flow`, `Stream / Flow Reverse`, `Temp. Independent`, `Percentage`, `Cp Coefficients`, `Equilibrium Array`, `HSC-Sim Mineral`) y búsqueda por nombre o descripción.
- **Panel lateral "Try Function":** Permite probar cualquier función con sus parámetros tipados (especie, ecuación, rango rápido o $T/P$), ver la sintaxis Excel exacta lista para copiar (`=StreamTH(A1:A5; B1:B5; C1; 0; 2000)`) y el resultado calculado contra `MainDB`.

### CU-03: Bloque Matricial de Equilibrio `StreamEQ`
- **Flujo principal:**
  1. El usuario define o edita una tabla de entrada con encabezados de fase prefijados por **`§`** (p. ej. `§ PHASE 1:`, `§ PHASE 2:`), las especies debajo de cada fase y sus moles iniciales (por defecto el caso clásico de combustión/gasificación `CO/CO2/N2/O2/C` a `1500 °C`, `1 bar` de `AddInSample.xls` filas `105–122`).
  2. Especifica temperatura $T$ y presión $P$ en sus celdas o inputs dedicados.
  3. Al pulsar **Calculate StreamEQ** (equivalente a `Ctrl+Shift+Enter` en Excel sobre el rango `{=StreamEQ(B111:B122; C111:C122; D108; D109)}`), el sistema resuelve el equilibrio multifase completo y escribe la temperatura resultante en las filas `§` y los moles en equilibrio en cada especie sin romper el bloque continuo.

### CU-04: Migración de Fórmulas Heredadas (`HSC5.xla` $\to$ `HSC6.XLL`) y Exportación
- Inspirado en `AddIns/Update XLS Files.xls`:
  1. El usuario pega una o varias fórmulas (o carga un CSV/TSV con fórmulas exportadas de libros Excel antiguos) que contengan prefijos absolutos del estilo `='C:\HSC5\AddIns_BackUp\HSC5.xla'!H(D11;E11)`.
  2. El migrador detecta rutas `*.xla'!` y `*.xll'!`, normaliza las referencias, valida que cada función pertenezca al catálogo oficial de `69` funciones de HSC6 (advirtiendo si se usa una función obsoleta o exclusiva de `HSC-Sim` como `StreamX`) y genera la fórmula limpia portable (`=H(D11;E11)`).
  3. Exportación de la hoja activa a **CSV/TSV** con valores o con fórmulas.

---

## 2. Reglas de Validación y Manejo de Errores (Paridad Excel)
- **Especie inexistente o base no cargada:** retorna `#NAME?` o `#N/A` con mensaje diagnóstico en barra de estado.
- **Raíz fuera de rango `[Tmin, Tmax]` en `StreamT*` / `FlowT*`:** retorna `#VALUE!` (exactamente como especifica `AddInSample.xls` filas 60–78).
- **Rango de longitudes desiguales en `Stream*` / `Flow*` o en `StreamEQ`:** retorna `#VALUE!`.
- **Modificación parcial de un bloque matricial `StreamEQ`:** bloquea edición destructiva de una sola celda del bloque de salida (misma regla `"solid and fixed block within spreadsheet"` de `27.4`), exigiendo editar el bloque completo o recalcular.
- **Límite en `DensityA`:** valida que la suma de fracciones en peso $\sum w_k \le 1.0$; si $\sum w_k > 1$, retorna `#NUM!`.
