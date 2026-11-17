# Análisis REA — Módulo 10: Species Converter

## 1. Contexto en la Arquitectura Hexagonal
- **Dominio (`soryntel-thermo::species_converter`)**: Cálculo de pesos molares a partir de fórmulas HSC (`soryntel_solver::parse_term`), matriz de fracciones másicas elemento-especie $A_{e,s}$, inicialización elemental (`Initialize`), optimización NNLS (`Method 1`, `Method 2`, `Method 3`), ponderación por estabilidad $\Delta G^\circ$ (`Stability Weighting`) y balance elemental (`Element Balance`, `Fix Element Balance`).
- **Puerto de Salida (`SpeciesCatalogPort` / `SpeciesRepository`)**: Consulta opcional de $\Delta G^\circ(298.15\text{ K})$ en base de datos para `Stability Weighting`; las fórmulas del usuario no requieren existir en la base de datos para conversión estequiométrica pura.
- **Capa de Aplicación (`soryntel-application`)**: Casos de uso `initialize_species_converter`, `convert_species`, `fix_species_balance`.
- **Adaptador Primario (`soryntel-desktop::species_converter_ui`)**: Cinta `Home` / `Tools`, libro multi-hoja (`Sheet1..N` + `Summary`), tabla Excel `INPUT`/`OUTPUT`, modal `Element Balance` y gráficos circulares `Charts`.

## 2. Actores y Flujos Principales
1. **Conversión Elemento → Especies (`Solve`)**: El usuario ingresa análisis elemental (o mixto) en `INPUT`, define especies candidatas en `OUTPUT` (opcionalmente con `Target wt-%` y `Weight Coeff`) y ejecuta `Solve`.
2. **Conversión Especies/Mezcla → Elementos (`Initialize`)**: El usuario ingresa especies en `INPUT` (ej. `Cu`, `CuO`, `Cu2S`, `CuSO4`) y ejecuta `Initialize` para obtener la composición elemental equivalente en `Calculated Elements`.
3. **Diagnóstico y Corrección (`Element Balance` / `Fix Element Balance`)**: Inspecciona `Input kg`, `Output kg`, `Balance kg`, `Error %` y `Limit` por elemento; `Fix Element Balance` agrega elementos puros al `OUTPUT` con peso `0.100` para cerrar el balance.
