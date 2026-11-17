# Especificación de Interfaz de Usuario (`UI`) — Módulo 17: Aqua

Especificación fiel basada en las **10 figuras oficiales** (`Fig. 1 – Fig. 10`) del manual de HSC 10 ([imagenes/](file:///home/nando/Documentos/Docs%20HSC/36-aqua-module/imagenes)).

---

## 1. Punto de Entrada y Barra de Cinta (`Ribbon` — Figs. 1, 2, 8)

1. **Acceso desde pantalla principal (Fig. 1):** Mosaico **`Aqu`** (*Aqueous Solutions*), ubicado en la primera posición de la cuadrícula de módulos principales.
2. **Barra Ribbon superior de `Aqua`:** Organizada en `6` grupos funcionales:

| Grupo | Controles visibles (HSC 9 / Fig. 2 y HSC 10 / Fig. 8) | Función / Comportamiento |
|---|---|---|
| **Main** | **`File ▼`** | Crear nuevo caso, abrir / guardar archivos **`*.aqu8`**, exportar resultados. |
| **Edit** | **`Cut`**, **`Copy`**, **`Paste`**, **`Find/Replace`** | Edición directa tipo hoja de cálculo sobre celdas de entrada (`A:C`). |
| **Calculation Options** | **`Calculate`** *(play azul)*, **`Aqua method ▼`**, **`Units ▼`**, **`Use OwnDB`** *(HSC 10, Fig. 8)* | Ejecuta `AquaDLL` (`Calculate`), elige `Harvie` / `Pitzer` / `Davies` (`Aqua method`), configura unidades T/P/Energía (`Units`) y activa/visualiza `OwnDB` en `Main Database`. |
| **Pitzer Database** | **`Main Database`**, **`Parameter Sets`**, **`Own Database`** *(HSC 10, Fig. 8)* | Abre el catálogo global de `1 031` conjuntos, el inspector de parámetros activos/faltantes y el editor `OwnDB`. |
| **Tools** | **`Density Calculator`** *(ícono `f(x)`)* | Abre la ventana modal/auxiliar para densidad de soluciones acuosas multicomponente. |
| **Info** | **`Help`**, **`About`** | Ayuda del capítulo y créditos de versión. |

Debajo del Ribbon aparece una **barra de fórmulas y selector de celda** (`E13`, `A7`, `A1`, etc.) con el nombre o valor de la celda activa, y en el pie izquierdo una única pestaña inferior **`Water Phase`**.

---

## 2. Grilla Principal `Water Phase` (Figs. 2, 3, 4, 8)

### 2.1. Regiones de entrada y salida
- **Fila `4` (`Water Phase`, filas de encabezado `1–3`):** Resumen global de toda la fase acuosa. Solo `B4` (Temp.) o el agregado `C4` reflejan el estado; las columnas **`D4:X4`** son **calculadas (solo lectura, sombreado beige)**.
- **Filas `7+` (`Water Species Data`, filas de encabezado `5–6`):** Lista de especies. La fila **`7` (`H2O`)** es obligatoria. Las columnas de entrada editables por el usuario son **`A`** (fórmula con `(a)`), **`B`** (temperatura en $^\circ\text{C}$) y **`C`** (cantidad en $\text{kmol}$, celdas blancas).

### 2.2. Especificación completa de columnas `A–X+` (Figs. 3 y 4)

| Col. | Encabezado Fila 1–3 (`Water Phase`) | Encabezado Fila 5–6 (`Water Species`) | Unidad | Rol |
|---|---|---|---|---|
| **`A`** | `Water Phase Data` (`"Water Phase"`) | `Water Species Data` (`H2O`, `H(+a)`, ...) | — | Entrada |
| **`B`** | `Temp.` | `Temp.` | `°C` | Entrada |
| **`C`** | `Amount` | `Amount` | `kmol` | Entrada |
| **`D`** | `Amount` | `Amount` | `mol-%` | Salida |
| **`E`** | `Amount` | `Amount` | `g` | Salida |
| **`F`** | `H Ideal` | `H Ideal` | `kcal/mol` | Salida |
| **`G`** | `H Estimate` | `H Estimate` | `kcal/mol` | Salida |
| **`H`** | `Cp Ideal` | `Cp Ideal` | `cal/(mol*K)` | Salida |
| **`I`** | `Cp Estimate` | `Cp Estimate` | `cal/(mol*K)` | Salida |
| **`J`** | `H Ideal` (`kcal/kg`) | `AC` (`Molar Sc.`) | `kcal/kg` / adim. | Salida |
| **`K`** | `H Estimate` (`kcal/kg`) | `AC` (`Molal Sc.`) | `kcal/kg` / adim. | Salida |
| **`L`** | `Cp Ideal` (`kcal/(kg*K)`) | `Molality` | `kcal/(kg*K)` / `mol/kg` | Salida |
| **`M`** | `Cp Estimate` (`kcal/(kg*K)`) | `MW` | `kcal/(kg*K)` / `g/mol` | Salida |
| **`N`** | `Osmotic Coeff.` | `Elements:` (`H`, ...) | adim. / estequiom. | Salida |
| **`O`** | `H2O Vapour Press.` | *(Elementos sig., p. ej. `Na`)* | `Pa` / estequiom. | Salida |
| **`P`** | `Sol. Vapour Press.` | *(Elementos sig., p. ej. `O`)* | `Pa` / estequiom. | Salida |
| **`Q`** | `Ionic Strength` | *(Elementos sig., p. ej. `S`)* | `mol/kg` / estequiom. | Salida |
| **`R`** | `pH` | *(Carga elemental `e-`)* | adim. / estequiom. | Salida |
| **`S`** | `Relat. Humidity` | *(vacío)* | `%` | Salida |
| **`T`** | `FP` | *(vacío)* | `°C` | Salida |
| **`U`** | `BP` | *(vacío)* | `°C` | Salida |
| **`V`** | `FP (aw at 25°C)` | *(vacío)* | `°C` | Salida |
| **`W`** | `BP (aw at 25°C)` | *(vacío)* | `°C` | Salida |
| **`X`** | `Osmotic Pressure` | *(vacío)* | `Pa` | Salida |

> *(Nota de Soryntel UI: al adaptar la grilla a unidades dinámicas `Units`, las cabeceras de energía/temperatura en las filas `3` y `6` deben actualizarse automáticamente cuando cambie el sistema de unidades, y `e-` debe mostrarse explícitamente en la matriz elemental para verificación de electroneutralidad).*

---

## 3. Ventana `Main Pitzer Database` (Figs. 5 y 9)

Consta de `4` pestañas inferiores sincronizadas con el inventario de `1 031` conjuntos:
- **`Binary parameters`** | **`Theta parameters`** | **`Psi parameters`** | **`Lambda parameters`**

### Encabezado descriptivo superior
- Créditos de autoría visibles: *Pertti Koukkari (98), Risto Pajarre (98), Justin Salminen (98, 09, 10), Hannu Sippola (98), Matias Hultgren (09, 10)*.
- Ecuación térmica $F(T) = a + \frac{b}{T} + c\ln(T) + dT + eT^2 + fT^{-2}$ y ecuaciones completas de Harvie–Pitzer $\ln\gamma_{+,M}$ y $\ln\gamma_{-,X}$ con leyenda de símbolos (`T`, `C`, `*` / `X`).

### Tabla inferior en HSC 10 (Fig. 9) y Código de `4` Colores
- **Columnas de la tabla:** Especies (`Cation (+)` / `Anion (-)` o equivalentes según pestaña), **`Active set used in Pitzer calc.`** (checkbox), rangos reportados (`Min T [C]`, `Max T [C]`, `Min m [mol/kg]`, `Max m [mol/kg]`, `Min I [mol/kg]`, `Max I [mol/kg]`) y marcadores `X` en las columnas `a, b, c, d, e, f` para cada parámetro ($b^{(0)}, b^{(1)}, b^{(2)}, C^\phi$ según pestaña) sin revelar los coeficientes reales de `MainDB`.
- **Semántica visual estricta (`4` colores):**
  1. **Negro:** Conjunto proveniente exclusivamente de `MainDB` (activo).
  2. **Azul:** Conjunto proveniente únicamente de `OwnDB` (colocado arriba al activar `Use OwnDB`).
  3. **Verde:** Conjunto presente simultáneamente en `OwnDB` y `MainDB` (sobrescrito por `OwnDB`).
  4. **Rojo:** Conjunto **excluido** del cálculo al desmarcar su casilla `Active set`.

---

## 4. Ventana `Active Pitzer Parameters` (`Parameter Sets` — Fig. 6)

Ventana auxiliar filtrada al sistema actual de `Water Phase`:
- **Pestaña `Active Parameters`:** Lista por secciones (`Pitzer Binaries`, `Theta`, `Psi`, `Lambda`) los conjuntos encontrados en BD junto con tres columnas resumen (`Activity coefficient (AC)`, `Enthalpy (H)`, `Specific heat capacity (Cp)`) codificadas con **`T`**, **`C`** o **`*`**, los rangos de validez reportados y el desglose por subparámetro $b^{(0)}, b^{(1)}, b^{(2)}, C(f)$ en bloques `AC`, `H` y `Cp`.
- **Pestaña `Missing Parameters`:** Lista explícita de interacciones sin parámetro disponible para alertar sobre brechas de modelado.

---

## 5. Editor `OwnDB` (`Own Database` — Fig. 8)

Ventana flotante de edición para la base personal del usuario:
- **Barra superior de ventana:** Menú **`Table`** (`Open`, `Save`, `Save As`, `Reset`) y botón/acción **`Delete Row`** (también en menú contextual sobre fila activa).
- **Pestañas inferiores:** `Binary parameters` | `Theta parameters` | `Psi parameters` | `Lambda parameters`.
- **Columnas editables (celdas blancas):** Especies, checkbox **`Show T parameters`** (al desmarcarlo bloquea en gris las celdas `b..f` y borra su contenido previo, manteniendo solo `a`), rangos (`Min/Max T [C]`, `Min/Max m [mol/kg]`, `Min/Max I [mol/kg]`) y los `6` coeficientes numéricos (`a, b, c, d, e, f`) por parámetro ($b^{(0)}, b^{(1)}, b^{(2)}, C^\phi$, etc.).
- **Comportamiento de foco:** Debe mostrarse recordatorio o confirmar automáticamente el commit de celda al perder el foco / pulsar `Save`.

---

## 6. Ventana `Density Calculator` (Fig. 7) y Gráficos Analíticos (Fig. 10)

### 6.1. `Density Calculator` (Fig. 7)
- **Barra de herramientas superior:** Grupo `Species List` (**`Add`**, **`Remove`**) y grupo `Sheet` (**`Clear`**).
- **Encabezado informativo:** Título `Density of Aqueous Solution`, autor (`Tuukka Kotiranta`), ruta de referencia al archivo base (`Aqueous Solution Density.xls`).
- **Panel izquierdo (`B:F` — Catálogo de electrolitos):** `Component in water`, `Mass fraction Min`, `Mass fraction Max`, `Temperature Min °C`, `Temperature Max °C`.
- **Panel derecho (`H:K` — Cálculo y Validación):**
  - Bloque resumen superior: **`H2O density`** ($\text{kg/m}^3$), **`Solution density`** ($\text{kg/m}^3$) y celda editable blanca **`Temperature`** ($^\circ\text{C}$).
  - Tabla activa inferior: **`Selected components`**, **`Mass fraction in solution`** *(celdas blancas editables)* y **`Validity region violations`** (`Mass fraction`, `Temperature °C`).

### 6.2. Gráficos de validación térmica (Fig. 10)
- Gráficas comparativas de **`Ionic AC`** ($\gamma_i$) frente a **`T / °C`** (p. ej. `0–40 °C`) contrastando series calculadas por HSC frente a valores experimentales con leyenda diferenciada por especie (`Ca(+2)`, `Cl(-a)`, `K(+)`, `Mg(+2)`, `Na(+a)`, `SO4(-2)`).
