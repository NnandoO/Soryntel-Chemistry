# Especificación Funcional — Módulo 16: Species & Thermochemical Databases

## 1. Actores y Alcance
- **Usuario Final (Ingeniero / Investigador):** Consulta propiedades termodinámicas de `21 090+` especies, busca fases por composición elemental o estequiometría, edita su base personal (`OwnDB`), ajusta polinomios de $C_p$ desde datos experimentales o literatura, convierte ecuaciones de $\Delta G(T)$ y gestiona bases heredadas o de equipo.
- **Consumidores Internos de Soryntel:** Módulos `01` a `15` consumen de manera transparente el **resolución jerárquica `OwnDB -> MainDB`** mediante el puerto `SpeciesCatalogPort` / `SpeciesRepository`.

---

## 2. Casos de Uso Funcionales (`CU-01` a `CU-10`)

### `CU-01`: Búsqueda por Elementos (`Search Mode`, `28.5`)
- **Entrada:** Subconjunto de elementos seleccionados mediante botones de Tabla Periódica o texto libre (`As, S, O`); modo de coincidencia (**`Some`** = especies formadas con subconjunto de elementos / **`All`** = contienen todos los elementos seleccionados); filtros de grupo (`Solids`, `Liquids`, `Gases`, `Aqueous Ions ao`, `Aqueous Neutrals ai`, `Organic`); límite opcional de átomos de carbono (`4, 6, 7`, hasta 9 dígitos).
- **Regla Orgánica:** Si `Organic` está desmarcado, se excluyen especies que contengan simultáneamente $\text{H}$ y más de `2` átomos de $\text{C}$.
- **Salida:** Listado separado o unificado de resultados en `OwnDB` y `MainDB`, con inspección instantánea por doble clic y exportación/envío a calculadora.

### `CU-02`: Búsqueda por Estequiometría Exacta y Proporcional (`Find Stoichiometry`, `28.6`)
- **Entrada:** Fórmula desordenada o alternativa (p. ej. `SiO4FeFe`, `AsS`, `CuO*Fe2O3`) + bandera **`Fix Ratio of Elements`**.
- **Comportamiento:**
  - Calcula el vector elemental exacto de la consulta.
  - Con `Fix Ratio = OFF`: encuentra exactamente los isómeros/fases con idéntico conteo atómico (`*2FeO*SiO2`, `Fe2SiO4(B)`, `Fe2SiO4(G)`).
  - Con `Fix Ratio = ON`: reduce el vector por su MCD (en tolerancia decimal `1e-4`) y retorna todos los múltiples/polímeros (`AsS`, `As2S2`, `As3S3`, `As4S4`).

### `CU-03`: Listado de Nombres y Fórmulas (`List Names` / `List Formulae`, `28.7–28.8`)
- Permite conmutar entre `OwnDB` y `MainDB`, listar todo el universo de especies o solo las filtradas por elementos, mostrar propiedades (`Show Data`), saltar directamente al editor (`Edit Data`) o exportar listado para impresión/CSV.

### `CU-04`: Consulta Multi-Campo por Palabra Clave (`Find Species with Key Word`, `28.9`)
- **Selector de campo (`Find What`):** `Formula`, `Structural Formula`, `Chemical Name`, `Common Name` (p. ej. `Pyrite`), `Chemical Abstract Number (CAS)`.
- **Opciones:** Checkbox **`All`** (abre panel lateral con todas las coincidencias parciales/polimorfos/fuentes alternativas), selector de unidad (`Joules`/`Calories`, `K`/`°C`), toggle **`Criss-Cobble`** en vivo para iones acuosos, `Copy` al portapapeles y gráfico $C_p(T), H(T), S(T), G(T)$.
- **Visor de Fuentes:** Al hacer clic en `Reference`, decodifica las abreviaturas contra las **`948` entradas de `MainDB6.REF`** mostrando la cita bibliográfica completa.

### `CU-05`: Editor de Especies y Rangos de Temperatura (`OwnDB CRUD`, `28.11`)
- **Creación / Edición:** Carga datos desde `MainDB` o `OwnDB` (`Read from:`), permite añadir/eliminar columnas de rango de temperatura (`Insert / Delete Column`), cambiar unidades de edición (`Joules`/`Calories`) manteniendo almacenamiento canónico calórico en `.HSC`.
- **Cálculos asistidos:** Calcula automáticamente `MW` desde `Formula` al presionar Enter/Compute; valida continuidad $T_{2,k} = T_{1,k+1}$; sugiere $\Delta_{tr}S = \Delta_{tr}H / T_{tr}$; selector visual del código de color (`0–53`) y clase de fiabilidad (`1–10`).
- **Protección:** Advierte antes de modificar `MainDB`; guarda por defecto en `OwnDB` (`Save to: Own Database`).

### `CU-06`: Ajuste de Datos $C_p(T)$ Experimentales (`Cp Fitting`, `28.12`)
- **Flujo:** Definir límites $(T_1, T_2)$ por fase; generar malla uniforme de temperaturas mediante `Step`; ingresar valores experimentales $C_{p,i}^{\text{exp}}$; marcar con `N` los coeficientes que desean fijarse en `0` (`A, B, C, D`); ejecutar **`Fit Cp Range`**; revisar `Cp fit`, `Error %` y gráfico comparativo; transferir con **`Save to Database`** hacia el editor.

### `CU-07`: Conversión de Funciones $C_p$ Externas (`Convert Cp Functions`, `28.13`)
- **Flujo:** Seleccionar plantilla de ecuación externa (`Maier-Kelley`, `Shomate`, `NASA-7`, `DIPPR-100`, `Custom formula` con parser simbólico de variable `T` y parámetros `A..G`), indicar unidad de entrada (`J/(kmol·K)`, `cal/(mol·K)`, etc.) y límites $(T_1, T_2)$; el módulo muestrea `10+` puntos, convierte unidades y ejecuta automáticamente el ajuste a formato HSC Kelley (`A, B, C, D`).

### `CU-08`: Conversión de $G(T)$ o $\Delta G(T)$ a $H, S, C_p$ (`28.14`)
- **Flujo:** Elegir `G(T)` o `Delta G(T)` y escala (`Calories`/`Joules`); si es `Delta G(T)`, ingresar fórmula objetivo (p. ej. `Cu2S`); introducir coeficientes $(h, s, a, b, c, d)$ de $G(T) = h + s\cdot 10^{-3}T + a\cdot 10^{-3}T\ln T + b\cdot 10^{-6}T^2 + c\,T^{-1} + d\cdot 10^{-9}T^3$; pulsar `Convert` para obtener $(\Delta H, \Delta S, \Delta A, \Delta B, \Delta C, \Delta D, \Delta G)$; pulsar **`Delta to Normal`** para sumar automáticamente las contribuciones elementales de `MainDB`/`OwnDB`; transferir al editor con `Save HSC`.

### `CU-09`: Eliminación, Ordenamiento, Fusión y Detección de Duplicados (`28.10`, `28.15`)
- **Delete Species:** Eliminar registros específicos de `OwnDB` por rango o fórmula.
- **Sort Database:** Ordenar registros alfabéticamente por fórmula y regenerar índice `*.CAT`.
- **Combine / Merge (`HSC1 -> HSC2`):** Fusionar dos archivos `.HSC`; si `Show Duplicates` está activo, abre diálogo comparativo lado a lado (`Take Source Data` / `Take Target Data` / editar antes de aceptar).

### `CU-10`: Selección de Bases Activas y Conversión Heredada (`28.16–28.17`)
- Seleccionar archivos `Open Own DB` y `Open Main DB` independientes (`*.HSC`, `.json`, `.csv`), crear nueva base vacía (`Create New Database`), regenerar catálogo (`Refresh Catalogue` → `.CAT`), contar especies únicas vs registros de intervalo (`Number of Species`), e importar/convertir archivos antiguos `HSC 2.0/3.0/4.0/5.0` al formato estándar `HSC 6.0` / Soryntel JSON.
