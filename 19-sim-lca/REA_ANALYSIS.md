# Análisis REA — Módulo 19: Sim LCA (`HSC - Sim LCA`)

## 1. Inventario de Fuentes Verificadas

| Artefacto | Ruta | Detalle Verificado |
|---|---|---|
| Texto estructurado | [49-sim-lca.md](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.md) | `1062` líneas; secciones `49.1` a `49.5`; autores Emilia Nuppumäki, Matti Peltomäki, Markus Reuter, Susanna Horn (5 sep 2023, © Metso Finland Oy) |
| Manual fuente | [49-sim-lca.pdf](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.pdf) | Documento oficial *HSC - Sim LCA*, `48` páginas |
| Evidencia visual | [imagenes/](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/imagenes) | `66` capturas PNG (`Imagen pegada.png` a `Imagen pegada (66).png`): ejemplo clásico **Zn TSL Fuming** (`49.3`) y ejemplo beta **Iron Process** con `openLCA 2.0.0` + `ecoinvent 3.8 LCI` (`49.4`) |
| Código existente en `soryntel` | [solver.rs](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/sim/solver.rs#L1069-L1110) | Implementación básica preliminar [`export_lca_json`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/sim/solver.rs#L1070-L1110) invocada desde [`sim_export_lca_json`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-application/src/lib.rs#L594-L595) |

---

## 2. Principios ISO 14040/14044 y Frontera de Caja Negra en HSC Sim (`49.1`–`49.2`)

De acuerdo con las secciones `49.1` y `49.2` del manual, el módulo cubre dentro de cuatro fases estándar ISO (`Goal & Scope Definition`, `LCI`, `LCIA`, `Interpretation`):

1. **Alcances soportados conceptualmente:** *Cradle-to-Grave*, *Cradle-to-Gate* y *Gate-to-Gate*. El modelo exportado desde HSC Sim constituye típicamente un bloque *Gate-to-Gate* parametrizado por balances fisicoquímicos rigurosos que se ensambla posteriormente en planes/sistemas de producto para estudios *Cradle-to-Gate* o *Cradle-to-Grave*.
2. **Regla estricta de frontera (*Black-Box Boundary*):**
   - Solo los flujos externos **azules** (entradas sin unidad origen en el diagrama) y los flujos externos **rojos** (salidas sin unidad destino en el diagrama) cruzan la frontera del sistema LCI.
   - Todas las corrientes internas **negras** (que conectan dos unidades dentro del diagrama) quedan excluidas del inventario exportado o enviado a `openLCA`, ocultando la topología interna y los secretos de proceso bajo una abstracción de caja negra o por sub-bloques agregados.
3. **Separación entre LCI (inventario físico/termodinámico) y LCIA (impactos):**
   - En el modo clásico (`49.3`), HSC Sim ejecuta únicamente la compilación, mapeo y normalización LCI hacia software externo (`GaBi` / `OpenLCA`).
   - En el modo beta (`49.4`), HSC Sim permanece como editor paramétrico de LCI pero orquesta un motor LCIA remoto vía gRPC IPC para devolver categorías normalizadas (por ejemplo, `ReCiPe Midpoint (H)`: `climate change - GWP100`, `metal depletion - MDP`).

---

## 3. Descomposición Arquitectónica Dual (`49.3` vs. `49.4`)

El capítulo 49 documenta dos arquitecturas de herramienta coexistentes en HSC Sim que comparten el propósito LCA pero difieren en protocolo de datos, estructura de hojas, normalización y acoplamiento:

| Dimensión | Evaluación LCA Clásica (`49.3`) | Evaluación LCA (beta) con IPC (`49.4`) |
|---|---|---|
| **Acceso y ventana** | `Tools -> LCA Evaluation` (`LCA Evaluation`) | Herramienta beta `LCA` |
| **Acoplamiento con BD externa** | Offline por archivos: catálogo embebido **GaBi** o carga manual de `.zip` JSON-LD en **OpenLCA** (`Set Active Database...`) | Online bidireccional mediante servidor **IPC gRPC** de `openLCA 2.0` (`Tools -> Developer tools -> IPC Server`, puerto `8080`) |
| **Tipo de base soportada** | Flujos elementales/productos de GaBi u OpenLCA | Selector `openLCA`: **`Unit database`** o **`LCI database`** (validado oficialmente en bases LCI de sistema, p.ej. `ecoinvent_3_8_LCI`) |
| **Hojas estándar** | `Input`, `Output`, `Manual Input`, `Manual Output`, `Indicator` + vista modal/tabla `Normalized Flows` | `Inputs`, `Outputs`, `Results` + hojas personalizadas (`Add/Delete/Rename Custom Sheet`), instantáneas (`Saved*`) e importadas (`Ext*`) |
| **Base temporal y unidades** | Integración implícita de **1 hora**: $\text{t/h} \to \text{kg}$ ($\times 1000$) y $\text{kW} \to \text{kWh}$; opción **`Use Exergy`** cambia la base a tasa exergética ($\text{kWh}$) | Conserva unidades de simulación en hojas (`t/h`, `kW`, `MJ`); convierte potencia eléctrica $\text{kW} \to 3.6\,\text{MJ/h}$ al poblar openLCA (`Fig 39`) |
| **Tratamiento de especies e indicadores** | Hoja dedicada `Indicator` con comodines `*`, `?`, `#`, columna booleana anti doble conteo `Counted` y filtro `"To Nature"` en modo OpenLCA | Expansión in-situ sobre fila de corriente en `Inputs`/`Outputs`: **`Convert Stream to Indicators`** y **`Convert Indicators back to Stream`** (excluyendo la corriente agregada al calcular) |
| **Clasificación y proveedores** | `LCA Equivalent` + `LCA Group` (`From Nature`, `From Technosphere`, `Materials/Fuels`, `Electricity/Heat`, `Services`, `Reference Product`, `To Nature`, `Allocated by product`, `Not defined`) | `LCA Flowtype` dependiente de dirección (`Elementary Flow`, `Product Flow`, `Waste Flow`, `Reference Flow (Product)`) + **`Provider`** + **`Location`** (filtro obligatorio si proveedores `> 15`) |
| **Normalización** | Por **`1 kg`** de una única corriente marcada `Main Product` (`49.3.6`) | Por **`1 t/h`** (o unidad de flujo) de una única corriente marcada `Reference Flow (Product)` (`Fig 34`) |
| **Formatos de salida / resultados** | `To EcoSpold v1.0 (GaBi)` (`.xml`), `To JSON-LD (OpenLCA)` (`To Active Database` / `To Empty File...`), `To Excel` | Cálculo en vivo LCIA en hoja `Results` por Categoría / Unidad / Entrada-Salida / Corriente + `Export to Excel` (`LCA_exports.xlsx`) |

### 3.1. Topología de Grafo Generada en `openLCA` por el Modo Beta (`49.4`, `Figs. 38–40`)

El examen directo de las capturas `Fig 38`, `Fig 39` y `Fig 40` (`Imagen pegada (65).png` y `(66).png`) revela exactamente cómo el puente IPC de HSC Sim construye el sistema de producto dentro de `openLCA`:

```text
[Fondo / Market Providers en ecoinvent LCI]
   (p.ej. market for petroleum coke, steel low-alloyed, pig iron, lime)
                  │
                  ▼
     [Unit Process: "Blast Furnace <timestamp>"]
        - Inputs: flujos Product/Waste/Elementary asignados a esa unidad
        - Outputs: emisiones Elementary asignadas a esa unidad
        - Output sintético (1.00000 kg): "Blast Furnace Reference Flow"
                  │
                  ├───────────────────────────┐
                  ▼                           ▼
   [Unit Process: "Manual Streams <ts>"]      │
     - Input: electricity, high voltage       │
       (10000 kW -> 3.60000E4 MJ)             │
     - Output (1.00000 kg):                   │
       "Manual Streams Reference Flow"        │
                  │                           │
                  └──────────┬────────────────┘
                             ▼
            [Main Process: "LCA Process <timestamp>"]
              - Inputs: 1.00000 kg de cada "<Unit> Reference Flow"
                        y 1.00000 kg de "Manual Streams Reference Flow"
              - Output (Reference Flow global): "Pig Iron" (1.38217E5 kg)
                             │
                             ▼
            [Product System: "*LCA Process <timestamp>"]
              - Auto-enlazado hacia proveedores de fondo
              - Evaluación LCIA por categoría seleccionada
```

Esta descomposición arquitectónica explica por qué la hoja `Results` (`Fig 34`) logra atribuir el impacto total normalizado (`16883.27 kg CO2-Eq per t/h of Pig Iron`) simultáneamente por **Unidad del diagrama** (`Blast Furnace = 3419.04`, `Manual Streams = 13464.22`), por **Dirección** (`Inputs = 2685.82`, `Outputs = 733.22`) y por **Corriente o Indicador individual**. Además, mientras no cambien los mapeos ni las categorías seleccionadas, los valores recalculados tras editar caudales en el simulador pueden escalarse localmente y actualizar la hoja `Results` incluso sin mantener activo el servidor gRPC.

---

## 4. Auditoría de Brechas en `soryntel`

Tras inspeccionar el código actual en `soryntel/crates/` mediante búsqueda de símbolos y lectura directa de [`solver.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-thermo/src/sim/solver.rs#L1069-L1110), [`lib.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-application/src/lib.rs#L594-L595) y [`sim_ui/mod.rs`](file:///home/nando/Documentos/HSC-REA/soryntel/crates/soryntel-desktop/src/sim_ui/mod.rs#L175-L177), el estado observado es únicamente un exportador mínimo de inventario introducido durante el Módulo 18:

- **Lo existente hoy:**
  - `export_lca_json(&SimProject)` (`solver.rs` líneas `1069–1110`) filtra `project.streams` por `StreamConnectivity::Input` y `StreamConnectivity::Output` e imprime un JSON propietario (`"format": "Soryntel-HSC-Sim-LCA-JSON"`) con `name`, `mass_t_h`, `enthalpy_kw`, `exergy_kw`, `temp_c` y `pressure_bar` en crudo, además de los campos básicos `process_name` y `technology`.
  - Se expone mediante un único modal de exportación de solo texto en `File -> Save as LCA JSON...`.

- **Brechas críticas respecto al Módulo `49-sim-lca`:**
  1. **Ausencia total de entorno interactivo LCA (`LCA Evaluation` / `LCA`):** No existen pestañas `Input`/`Output`/`Manual`/`Indicator`/`Results`, árbol de flujos mapeables, panel de propiedades del flujo ni configuración persistente de mapeos en `SimProject`.
  2. **Falta de base de conversión y toggle Exergía:** No convierte $\text{t/h} \to 1000\,\text{kg}$ ni potencia $\text{kW} \to \text{kWh}$ o $\text{MJ}$ (`3.6` MJ/kWh), ni soporta conmutación global `Use Exergy`.
  3. **Inexistencia de indicadores con comodines y prevención de doble conteo:** No existe motor de búsqueda `*` / `?` / `#` sobre especies de corrientes de frontera, ni indicador `Counted`, ni `Convert Stream to Indicators` / `Convert Indicators back to Stream`.
  4. **Ausencia de normalización por producto principal:** No existe selección única de `Main Product` / `Reference Flow`, agregación de flujos mapeados repetidos ni escalado a `1 kg` (clásico) o `1 t/h` (beta).
  5. **Faltan formatos estándar EcoSpold v1.0 XML y JSON-LD `.zip` (OpenLCA):** El JSON actual no cumple esquema EcoSpold v1.0 ni JSON-LD (`@context`, `Process`, `Flow`, `Exchange`, `FlowProperty`, `UnitGroup`, `Location`).
  6. **Ausencia de integración IPC gRPC / motor LCIA interno:** No existe cliente gRPC compatible con `openLCA` (`Unit`/`LCI`), selector `Provider`/`Location` con umbral `> 15`, ni evaluador de categorías de impacto (`GWP100`, `MDP`, etc.) con vista jerárquica `Results` e instantáneas `Saved*` / hojas `Ext*`.
