# Especificación de Interfaz (Borrador Inicial) — Módulo 07: Eh-pH Diagrams

> [!NOTE]
> Este diseño preliminar sigue `17 Eh - pH - Diagrams.txt` (Figs. 1–10) y se ajustará exactamente cuando envíes las capturas oficiales del módulo **EpH**.

## 1. Ventana y Pestañas / Pasos
- **Título:** `Eh - pH Diagrams (Pourbaix) — Soryntel Chemistry`
- **Panel Izquierdo / Superior (System & Diagram Menu):**
  - Selector de `Main Element` + `Secondary Elements` (tabla periódica / texto rápido ej. `Cu, S`).
  - Lista de temperaturas (`T1..T4`), molalidades (`mol/kg H2O`), presión (`bar`), `Min/Max pH`, `Min/Max Eh`.
  - Checkboxes: `Criss-Cobble`, `Show Water Stability Lines`, `Show Predominance Areas of Ions`.
  - Botones: `[Search Species]`, `[Diagram]`, `[Combine]`, `[Table]`.
- **Área Central / Pestañas:**
  - `Diagram`: gráfico `(pH, Eh)` interactivo con regiones coloreadas/etiquetadas, fronteras negras/azules y líneas punteadas del agua.
  - `Species`: grilla editable estilo Excel (`Formula`, `Phase`, `ΔG(T1..T4)`).
  - `Combine`: configuración de hasta 4 `Data Sets` (`Select`, `Main Element`, `Temp`, `Molality`, `Pressure`, `Line Text`).
  - `Table`: coordenadas numéricas de líneas y etiquetas (`DS1..DS4`, `Label`, `Lines`).
