# Análisis REA — Módulo 12: Elements Module

## 1. Hallazgos en HSC Chemistry 6 (`/home/nando/Descargas/HSC6/`)
- **Documentación (`Help/24 Elements Module.txt`, `06 Whats New in HSC 5.txt`):**
  - El comportamiento termoquímico de las especies depende directamente de las propiedades periódicas de sus elementos (electronegatividad, estados de oxidación, estructura electrónica, entalpías y entropías estándar elementales).
  - El módulo **Elements** presenta una base de datos en hoja de cálculo tipo Excel (`Databases/ELEMENT4.ELE`, formato Excel 2 BIFF 2 de `83 KB`) con **~56 propiedades** por elemento.
  - Incluye **Fix Left 2 Columns** para inmovilizar las columnas de propiedad y unidades mientras se desplaza horizontalmente por los elementos (`H` a `Unh` / `Og`).
  - El botón **Diagram** genera una vista de **Tabla Periódica con esferas proporcionales** para la fila de propiedad seleccionada: la esfera del valor máximo encaja exactamente en la celda de su elemento y al hacer **doble clic** sobre cualquier casilla o etiqueta se muestra el valor numérico exacto.
- **Esquema real extraído de `Databases/ELEMENT4.ELE`:**
  - Columnas fijas: `Property | Units | Reference`, seguidas por los símbolos químicos (`H, He, Li, ..., Unq, Unp, Unh`).
  - Categorías de las ~56 propiedades cubiertas:
    1. *Identidad y masa*: `Atomic Number`, `Symbol`, `Name`, `Atomic Weight (g/mol)`.
    2. *Estados de oxidación y electrónica*: `Oxidation States, Most Stable`, `Oxidation States`, `Electron Configuration`, `Electronegativity` (Pauling), `First Ionization Potential`, `Atomic Electron Affinity`, `Polarizability (1e-24 cm³)`.
    3. *Propiedades térmicas y de cambio de fase*: `Melting Point`, `Melting Point Pressure`, `Boiling Point`, `Heat of Fusion`, `Heat of Vaporization`, `Specific Heat Capacity`, `Thermal Conductivity`, `Linear Expansion 298.15 K`.
    4. *Propiedades físicas y electromagnéticas*: `Density (g/ml)`, `Electrical Conductivity`, `Magnetic Susceptibility (1e-6 cgs)`, `Color`, `Color RGB-number`.
    5. *Tamaños y cristalografía*: `Atomic Volume`, `Atomic Radius`, `Covalent Radius`, `Crystal Structure`, `Ionic Radii in Crystals (Oxidation state)` (múltiples estados), `Lattice parameter a, b, c`.
    6. *Acidez, solubilidad, abundancia y nuclear*: `Acid-Base Properties`, `Solubility in Cold Water`, `Solubility in Hot Water`, `Content in the Sea`, `Content in the Air` (+ corteza terrestre), `Year when discovered`, `Half-Life of the most stable nuclide`, `Isotopes, Atomic number, Relative atomic weight, Half life`.
    7. *Termoquímica estándar a `298.15 K`*: `Enthalpy of Formation (Gas / Solid)`, `Gibbs Energy of Formation (Gas / Solid)`, `Standard Entropy (Gas / Solid)`, `Heat Capacity (Gas / Solid)`.

## 2. Rol en la Arquitectura Hexagonal (`soryntel`)
- **Dominio (`soryntel-thermo::elements`)**:
  - Catálogo tipado en memoria (`PERIODIC_ELEMENTS`) para los elementos químicos `Z = 1..118` con sus propiedades físicas, químicas y termoquímicas curadas.
  - Modelo de hoja de trabajo editable (`ElementWorkbook`, `ElementPropertyRow`, filas personalizadas del usuario) y motor de proyección `compute_periodic_diagram` (esferas proporcionales, estadísticas $\min/\max/\mu$ y serie `vs Z`).
- **Puerto de Salida (`SpeciesCatalogPort`)**:
  - Consulta cruzada al hacer doble clic o inspeccionar un elemento para listar especies de referencia de ese elemento presentes en `Maindb6` (ej. `Fe`, `Fe2O3`, `Fe(+2a)`).
- **Capa de Aplicación (`soryntel-application`)**:
  - Casos de uso `default_elements_workbook`, `compute_element_diagram` y `lookup_element_species`.
- **Adaptador Primario (`soryntel-desktop::elements_ui`)**:
  - Interfaz dual tipo HSC con pestañas o paneles acoplados: **Spreadsheet (`Element4`)** (reutilizando `excel_grid.rs`, `Fix Left 2 Columns`, buscador de propiedades/elementos) y **Periodic Diagram** (tabla periódica de 18 columnas + bloque *f* con esferas proporcionales, código de color y modal de ficha de elemento al doble clic), más gráfico de tendencia `Property vs Atomic Number (Z)`.

## 3. Brechas y Consideraciones de Diseño
1. Unificar pesos atómicos con `soryntel_thermo::estimate::atomic_weight` y `species_converter::atomic_weight` como fuente de verdad única en el dominio.
2. Permitir edición y adición de filas personalizadas en la hoja `Elements` con persistencia en JSON/CSV sin dependencias propietarias de BIFF 2.
3. Manejar correctamente propiedades con valores negativos ($\Delta H_f^\circ$, afinidad electrónica, potencial estándar) o no disponibles (`N/A` en gases nobles para electronegatividad) dentro del diagrama proporcional.
