# Módulo 19: Sim LCA (Life Cycle Assessment) — Ingeniería Inversa Clean-Room (`HSC - Sim LCA`)

Especificación técnica **clean-room** del submódulo **HSC - Sim LCA (Life Cycle Assessment)** basada estrictamente en el capítulo oficial 49 del manual de HSC Chemistry (`48` páginas, fechado el 5 de septiembre de 2023, autores Emilia Nuppumäki, Matti Peltomäki, Markus Reuter y Susanna Horn) y sus `66` capturas de referencia.

## Alcance del Módulo

El módulo **Sim LCA** implementa la fase de **Inventario de Ciclo de Vida (LCI)** y la **Evaluación de Impacto de Ciclo de Vida (LCIA)** sobre balances de materia, energía y exergía calculados en **HSC Sim** conforme al marco **ISO 14040 / ISO 14044**. Opera sobre dos generaciones complementarias documentadas en el manual oficial:

1. **Evaluación LCA Clásica (`49.3`, `Tools -> LCA Evaluation`)**:
   - Extracción automática de flujos de frontera externos (entradas **azules** y salidas **rojas**; exclusión de corrientes internas **negras** como caja negra industrial).
   - Conversión horaria base ($\text{t/h} \to 1000\,\text{kg}$, $\text{kW} \to 1\,\text{kWh}$) o conmutación exergética (`Use Exergy`), gestión de corrientes manuales (`Manual Input`, `Manual Output`), detección de indicadores con comodines (`*`, `?`, `#`) sin doble conteo, mapeo contra árboles de flujo **GaBi** u **OpenLCA**, normalización a **`1 kg`** de `Main Product` y exportación a **EcoSpold v1.0 XML**, **JSON-LD (`.zip`)** y **Excel**.
2. **Evaluación LCA (beta) acoplada a `openLCA` (`49.4`, gRPC IPC)**:
   - Conexión directa mediante servidor **IPC gRPC** de `openLCA 2.0` (`localhost:8080`) con descarga en vivo de árboles `Flows` (`P -` producto, `E -` elemental) e `Impact Methods and Categories` desde bases `Unit` o `LCI`.
   - Clasificación en `4` tipos (`Elementary Flow`, `Product Flow`, `Waste Flow`, `Reference Flow (Product)`), resolución de proveedores y ubicaciones (con filtro obligatorio de ubicación cuando existen `> 15` proveedores), conversión bidireccional de corrientes a indicadores (`Convert Stream to Indicators` / vuelta a corriente), creación automática de grafo de procesos (`Unit Processes`, `Main Process` y `Product System`), normalización por unidad de flujo de referencia (`t/h`) y desglose jerárquico de resultados LCIA en tiempo real en la hoja `Results`.

---

## Índice de Documentos

| Documento | Descripción | Enlace |
|---|---|---|
| **Análisis REA y Arquitectura** | Inventario de fuentes, arquitectura dual (clásica vs. beta gRPC), topología de procesos openLCA y auditoría de brechas en `soryntel` | [REA_ANALYSIS.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/REA_ANALYSIS.md) |
| **Especificación Matemática** | Frontera LCI, conversiones masa/energía/exergía/MJ, normalización de caja negra, comodines anti-doble-conteo y formulación matricial LCIA | [MATHEMATICAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/MATHEMATICAL_SPECIFICATION.md) |
| **Especificación Funcional** | Casos de uso `CU-01` a `CU-11` cubriendo flujos clásico y beta, IPC, mapeos, proveedores, hojas personalizadas y exportaciones | [FUNCTIONAL_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/FUNCTIONAL_SPECIFICATION.md) |
| **Especificación de UI** | Anatomía detallada de ventanas `LCA Evaluation` (`49.3`) y `LCA` (`49.4`), cintas de herramientas, grillas, árboles laterales y diálogos | [UI_SPECIFICATION.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/UI_SPECIFICATION.md) |
| **Plan de Validación** | Matriz `LCA-01` a `LCA-10` con cifras exactas de verificación (`Zn TSL Fuming` para LCI clásico y `Iron Process` para LCIA ReCiPe Midpoint H) | [VALIDATION_PLAN.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/VALIDATION_PLAN.md) |
| **Referencias Científicas** | Fuentes primarias del manual, normas ISO 14040/14044, bibliografía oficial (`49.5`) y estándares abiertos EcoSpold / JSON-LD / gRPC IPC | [SCIENTIFIC_REFERENCES.md](file:///home/nando/Documentos/HSC-REA/19-sim-lca/SCIENTIFIC_REFERENCES.md) |

---

## Fuentes Primarias Consultadas

- Manual oficial convertido: [49-sim-lca.md](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.md) (`1062` líneas)
- Documento PDF original: [49-sim-lca.pdf](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.pdf) (`48` páginas)
- Capturas oficiales de referencia: [imagenes/](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/imagenes) (`66` archivos PNG)
