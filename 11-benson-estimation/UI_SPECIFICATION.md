# Especificación de Interfaz (UI) — Módulo 11: Benson Estimation

## 1. Distribución General (Responsiva, `.resizable(true)` + `ScrollArea::both()`)
- **Barra superior (`TopBottomPanel::top`):**
  - Presets rápidos de prueba: `n-Butane`, `Isobutane`, `Benzene`, `Toluene`, `Ethanol`, `Acetic Acid`, `Cyclohexane`.
  - Selector de unidades de energía (`kJ/mol` / `kcal/mol`) y fase (`Gas` / `Liquid`).
  - Acciones: `Parse SMILES`, `Estimate`, `Save to Database`.
- **Panel izquierdo redimensionable (`SidePanel::left`):**
  - Entrada de **SMILES**, **Nombre** y **Fórmula empírica** autocalculada + masa molar (`g/mol`).
  - Parámetros estructurales: `Symmetry σ`, `Optical isomers (n_opt)`, `Ring / Strain correction`, `Gauche / Cis count`.
  - Catálogo buscable de grupos Benson (`+ Add Group`) para añadir/ajustar grupos manualmente.
- **Panel central con pestañas (`CentralPanel`):**
  1. **Grupos y Contribuciones (`Benson Groups`)**: tabla Excel con `Group | Count | dH (kcal/mol) | S (cal/mol·K) | Cp 300..1500` y fila de totales + correcciones ($\sigma, n_{\text{opt}}$, anillos).
  2. **Resultados y Kelley (`Properties & Kelley`)**: tarjetas resumen ($\Delta_f H^\circ_{298}$, $S^\circ_{298}$, $\Delta_f G^\circ_{298}$, $C_{p,298}$), tabla $C_p(T)$ vs ajuste Kelley, coeficientes `A, B, C, D` y comparación con `SpeciesCatalogPort` si existe coincidencia.
  3. **Gráfico $C_p(T)$ (`Cp Curve`)**: curva Kelley estimada, puntos discretos de Benson ($300\text{–}1500\text{ K}$) y curva de referencia experimental si está disponible.

## 2. Requisitos de Responsividad
- Tamaño mínimo de ventana `[480.0, 360.0]`, tema claro (`egui::Visuals::light()`), bordes visibles y `ScrollArea::both()` en tablas y paneles laterales.
