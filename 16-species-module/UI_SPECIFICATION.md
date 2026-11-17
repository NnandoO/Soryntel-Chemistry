# Especificación de Interfaz (UI) — Módulo 16: Species & Databases

## 1. Arquitectura de Navegación Fiel a HSC6 (`Fig. 1–18`)
- **Ventana Principal:** `"Species & Thermochemical Databases (HSC) — Soryntel Chemistry"` (`[1080.0, 720.0]`, mínimo responsive `[480.0, 360.0]`).
- **Barra de Menú Superior:** `File | Edit | View | Tools | Databases | Help`
  - **File:** `Open Main DB (.HSC)...`, `Open Own DB (.HSC)...`, `Create New OwnDB...`, `Export Selected (CSV / JSON)`, `Close`.
  - **Edit:** `Copy Selected`, `Paste Intervals`, `Clear Editor`, `Delete from OwnDB`.
  - **View:** toggle rápido `Joules / Calories`, `K / °C`, `Criss-Cobble Extrapolation`.
  - **Tools:** acceso directo a `Cp Fitting`, `Convert Cp Functions`, `Convert G(T) -> H,S,Cp`, `Merge / Sort / Legacy Convert`.
  - **Databases:** estado actual de `MainDB` (`21 090` especies / `36 537` intervalos) y `OwnDB` (`N` especies), botón `Refresh Catalogue (.CAT)`.
- **Hub / Barra de Vistas Superior (reemplaza las ventanas modales dispersas del menú `Fig. 1` manteniendo identidad visual exacta):**
  1. **`Database Menu & Browser`** (`Fig. 1, 3–9`) — Búsqueda por Elementos, Estequiometría (`Fix Ratio`), Palabras Clave (`Formula | Struct | ChemName | CommonName | CAS`), Lista de Nombres/Fórmulas e Inspector Completo de 19 campos + Citas `MainDB6.REF` + Gráfico $C_p / H / S / G$.
  2. **`Species Editor (OwnDB)`** (`Fig. 10`) — Editor tabular multi-intervalo idéntico a la Fig. 10 de HSC6.
  3. **`Fit Cp Data`** (`Fig. 11`) — Tabla izquierda $(T, C_p^{\text{exp}}, C_p^{\text{fit}}, \text{Error \%})$ + Tabla derecha $(T_1, T_2, A, B, C, D)$ con mascarado `N` + Gráfico.
  4. **`Convert Cp Functions`** (`Fig. 12`) — Selector de fórmula externa, grilla de coeficientes por rango, unidad de entrada y conversión automática a Kelley HSC.
  5. **`Convert G(T) -> H,S,Cp`** (`Fig. 13`) — Formulario `G(T)` / `Delta G(T)`, botones `Convert`, `Delta to Normal`, `Save HSC`.
  6. **`DB Tools & Active Files`** (`Fig. 14–18`) — Selección de `OwnDB`/`MainDB`, estadística `Number of Species vs Ranges`, `Merge (Show Duplicates: Take Source / Take Target)`, `Sort`, conversor `HSC 2.0 -> HSC 6.0` y tabla de referencia `Color Codes 0–53`.

---

## 2. Detalle Visual por Vista

### 2.1 Vista 1: `Database Menu & Browser` (`Fig. 1, 3–9`)
- **Panel Izquierdo (Controles de Búsqueda, ancho adaptable):**
  - Tabs internos o acordeones compactos:
    - **Keyword Search (`Fig. 8`):** input `Keyword`, combo `Find What: [Formula | Structural | Chemical Name | Common Name | CAS]`, checkbox `All matches`, filtro de DB (`Both (Own first) | OwnDB | MainDB`).
    - **Element Search (`Fig. 3`):** mini-botones de Tabla Periódica + campo texto (`As, S, O`), radio `Some Elements | All Elements`, checkboxes de fase (`Solid`, `Liquid`, `Gas`, `Aqua Ions (ao)`, `Aqua Neutrals (ai)`), checkbox `Organic` + campo `Carbon count filter`.
    - **Stoichiometry (`Fig. 4–5`):** input `Formula` (`SiO4FeFe`, `AsS`) + checkbox **`Fix Ratio of Elements`**.
  - Lista de resultados (`Matching Species`): icono/badge `[OWN]` o `[MAIN]`, fórmula en monoespaciado y color swatch si `Color != 0`. Doble clic abre directamente en el Editor o copia al módulo llamador si está en modo selector.
- **Panel Derecho (Ficha Completa + Tabla de Intervalos + Gráfico):**
  - **Basic Data (`19` campos):** `Formula`, `Structural Formula`, `Chemical Name`, `Common Name`, `CAS Number`, `Molecular Weight`, `Mp`, `Bp`, `Density`, `Color` (cuadro visual RGB + etiqueta `0..53`), `Solubility`, `Reliability Class` (`1..10`) y bloque **`Reference`** expandible con resolución en vivo contra `MainDB6.REF`.
  - **Tabla de Intervalos (`T1–T2`, `Phase`, `H`, `S`, `A`, `B`, `C`, `D`, `Ref`, `Class`):** alternando escala `J/cal` y `K/°C` con resaltado Criss-Cobble cuando aplique a iones acuosos.
  - **Sub-panel Evaluador & Gráfico:** entrada rápida de $T$ puntual para ver $C_p, H, S, G$ y gráfico `egui_plot` continuo sobre $[T_{min}, T_{max}]$.

### 2.2 Vista 2: `Species Editor (OwnDB)` (`Fig. 10`)
- Barra superior: selector de origen `Read from: [OwnDB | MainDB]`, campo `Formula` (`24` chars max, indicador de longitud restante), botón `Calc MW`, toggles `Calories / Joules` y `Clear All`.
- Cabecera editable de metadatos (`Structural`, `Chemical Name`, `Common Name`, `CAS`, `MW`, `Mp`, `Bp`, `Density`, selector desplegable `Color 0–53` con muestra visual, `Reliability Class`).
- Grilla de columnas editables por intervalo `Range 1 .. Range N` (con botones `+ Add Column`, `- Remove Column`, cálculo asistido `ΔtrS = ΔtrH / Ttr`, alerta roja si $T_{2,k} \ne T_{1,k+1}$ o si falta sufijo `(g)`/`(a)`).
- Barra inferior: botón destacado **`Save to: Own Database`** (y botón secundario avanzado `Save to Main Database` protegido con confirmación).

### 2.3 Vistas 3, 4 y 5: Ingeniería Termoquímica (`Fig. 11–13`)
- **`Fit Cp Data`:** Split `50% / 50%` con tabla experimental (`Generate Grid by Step`), tabla de coeficientes (`N` para bloquear parámetro), botones `Fit Cp Range`, `Diagram` y `Save to Editor`.
- **`Convert Cp Functions`:** Selector rápido (`Maier-Kelley`, `Shomate`, `NASA-7`, `DIPPR`, `Custom`), grilla de rangos, botón `Convert Cp Functions to HSC Format`.
- **`Convert G(T) -> H,S,Cp`:** Toggle `Delta G(T)` / `G(T)`, fila superior de entradas `Formula`, `h, s, a, b, c, d`, fila inferior de resultados `H, S, A, B, C, D, G`, botones `Convert`, `Delta to Normal`, `Save to Editor`.

### 2.4 Vista 6: `DB Tools & Active Files` (`Fig. 14–18`)
- Tarjetas limpias para rutas activas (`OwnDB6.HSC`, `Maindb6.HSC`), conteos exactos (`Unique Species` vs `Temperature Ranges`), botones `Refresh .CAT`, `Sort Database`, `Merge HSC1 -> HSC2` (con modal de comparación de duplicados `Take Source` / `Take Target`), exportador/importador `CSV / JSON / HSC6` y conversor heredado `HSC 2.0 -> HSC 6.0`.
