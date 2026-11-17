# Referencias Científicas y Fuentes Primarias — Módulo 19: Sim LCA

## 1. Documentación Primaria del Producto (Clean-Room Source Baseline)

| Fuente | Ubicación | Descripción |
|---|---|---|
| Manual oficial HSC 10 — Cap. 49 (Markdown) | [49-sim-lca.md](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.md) | *HSC - Sim LCA*, `1062` líneas, 5 de septiembre de 2023, Emilia Nuppumäki, Matti Peltomäki, Markus Reuter y Susanna Horn, © 2023 Metso Finland Oy |
| Manual oficial HSC 10 — Cap. 49 (PDF) | [49-sim-lca.pdf](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.pdf) | Documento fuente paginado (`48` páginas), secciones `49.1` a `49.5` |
| Repositorio de capturas oficiales | [imagenes/](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/imagenes) | `66` capturas PNG de `LCA Evaluation` clásico (`Zn TSL Fuming`), `EcoSpold v1.0`, `JSON-LD`, `GaBi`, `openLCA 2.0.0.alpha3` y `LCA` beta (`Iron Process`) |

---

## 2. Bibliografía Oficial Citada en el Manual (`49.5`)

Las siguientes referencias normativas y científicas constan textualmente en la sección `49.5` de [49-sim-lca.md](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.md):

1. **Worrell, E. & Reuter, M. A. (2014).** *Handbook of Recycling: State-of-the-art for Practitioners, Analysts, and Scientists*. Elsevier, Amsterdam, ISBN 978-0-12-396459-5 (fundamentos de reciclaje metalúrgico, exergía y análisis sistémico de procesos vinculados a LCA).
2. **ISO 14044 (2006).** *Environmental management — Life cycle assessment — Requirements and guidelines*. International Organization for Standardization, Geneva (requisitos y directrices para LCI, LCIA, asignación, normalización y revisión crítica).
3. **ISO 14040 (2006).** *Environmental management — Life cycle assessment — Principles and framework*. International Organization for Standardization, Geneva (marco metodológico en cuatro fases: definición de objetivo/alcance, análisis de inventario, evaluación de impacto e interpretación).
4. **Gediga, J. (2014).** *GaBi Paper Clip Tutorial*. PE International / thinkstep (tutorial clásico de modelado de planes, procesos e importación/evaluación en GaBi).
5. **thinkstep / GaBi Software Tutorials.** *How to model in GaBi* (`http://tutorials.gabi-software.com/` — referencia de flujo de trabajo para importación EcoSpold v1 y construcción de planes).
6. **Reuter, M. A., van Schaik, A., & Gediga, J. (2015).** *Simulation-based design for resource efficiency of metal production and recycling systems: Cases - copper production and recycling, e-waste (LED lamps) and nickel pig iron*. *International Journal of Life Cycle Assessment*, 20(5), 671–693, DOI: `10.1007/s11367-015-0860-4` (artículo seminal sobre acoplamiento directo entre simulación termodinámica de procesos metalúrgicos en HSC Sim y software LCA).
7. **Reuter, M. A., Hudson, C., van Schaik, A., Heiskanen, K., Meskers, C., & Hagelüken, C. (2013).** *Metal Recycling: Opportunities, Limits, Infrastructure* (Eds. Graedel, T. E. & Reuter, M. A.). A Report of the Working Group on the Global Metal Flows to the International Resource Panel, UNEP (`http://www.unep.org/resourcepanel/Publications/MetalRecycling/`).

---

## 3. Estándares Abiertos y Formulaciones Computacionales Clean-Room

Para implementar de forma independiente los formatos de intercambio y el motor LCIA sin código propietario externo:

1. **EcoSpold v1.0 XML Format (`ecoinvent` / `GaBi` import):** Especificación abierta basada en esquema XSD (`dataset` / `metaInformation` / `processInformation` / `flowData` / `exchange` con atributos `inputGroup`, `outputGroup`, `meanValue`, `unit`, `location`).
2. **openLCA Schema & JSON-LD Archive Format:** Modelo de datos abierto de GreenDelta (MPL 2.0) estructurado como archivo `.zip` conteniendo carpetas `processes/`, `flows/`, `flow_properties/`, `unit_groups/`, `locations/`, `categories/` y `@context` JSON-LD.
3. **openLCA IPC / gRPC Protocol (`olca-ipc` / `olca-proto`):** Interfaz cliente-servidor abierta (puerto por defecto `8080`, servicio gRPC habilitado bajo `Tools -> Developer tools -> IPC Server`) para consulta de descriptores (`Flow`, `ImpactMethod`, `ImpactCategory`, `Process`, `Location`), creación de `Process` / `ProductSystem` y ejecución de `CalculationSetup`.
4. **Heijungs, R. & Suh, S. (2002).** *The Computational Structure of Life Cycle Assessment*. Kluwer Academic Publishers, Dordrecht (formulación matricial estándar $\mathbf{s} = \mathbf{A}^{-1}\mathbf{f}$, $\mathbf{g} = \mathbf{B}\mathbf{s}$, $\mathbf{h} = \mathbf{Q}\mathbf{g}$ utilizada para verificación algebraica limpia).
5. **Métodos de Caracterización LCIA referenciados en el manual:**
   - **CML** (Leiden University, GWP100 con $CH_4 = 25\,\text{kg CO}_2\text{-eq/kg}$ citado en `49.1`).
   - **TRACI** (US EPA, citado en `49.1`).
   - **ReCiPe Midpoint (H)** (`climate change - GWP100`, `metal depletion - MDP`, `water depletion - WDP`, `particulate matter formation - PMFP`, etc., verificado en `49.4`, `Figs. 30–35`).
