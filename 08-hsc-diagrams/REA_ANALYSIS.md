# REA Analysis — Módulo 08: H, S, C and G Diagrams

## 1. Fuente oficial analizada
- `/home/nando/Descargas/HSC6/Help/19 H, S, C and G Diagrams.txt` (HSC Chemistry 6).

## 2. Capacidades existentes reutilizables en `soryntel`
- **`soryntel_thermo::evaluate_species_si(sp, t_k, criss_cobble, upper_side)`**:
  - Evalúa `cp`, `h`, `s`, `g` en SI para cualquier especie de `Maindb6.HSC` / `OwnDB`, integrando intervalos de Kelley y transiciones de fase ($\Delta H_{tr}, \Delta S_{tr}$).
  - Devuelve `extrapolated_from_k: Option<f64>`, que indica exactamente si `t_k` excede el rango tabulado en la base de datos.
- **`soryntel_thermo::sum_element_references(term, t_k, criss_cobble, repo)`**:
  - Calcula $(H_{\text{ref}}, S_{\text{ref}}, G_{\text{ref}}, C_{p,\text{ref}})$ respecto a los elementos de referencia a temperatura $T$, permitiendo calcular directamente `DH` ($\Delta_f H$), `DS` ($\Delta_f S$) y `DG` ($\Delta_f G$).
- **`soryntel_solver::parse_term`**:
  - Parsea fórmulas químicas y coeficientes estequiométricos (ej. `2/3 Fe2O3` o `0.667 Fe2O3`) y extrae el conteo de cada elemento (`O`, `S`, `Cl`, etc.).
- **`soryntel_db::ThermoDatabase`**:
  - Búsqueda de especies por fórmula (`get_species`, `search`) y soporte de base de datos principal + especies agregadas por el usuario.

## 3. Brechas técnicas a implementar
1. **Motor `soryntel-thermo/src/diagrams.rs`**:
   - Evaluación vectorizada sobre un rango `[T_min, T_max]` con paso `ΔT` para una lista de especies con multiplicadores opcionales `(coeff, formula)`.
   - Implementación de los 8 modos (`H_total`, `H_latent`, `S`, `Cp`, `G`, `DH`, `DS`, `DG`).
   - Segmentación de cada curva en tramos **sólidos** (`extrapolated_from_k.is_none()`) y **punteados** (`extrapolated_from_k.is_some()`).
   - Helper `balance_element_amount(formulas, element, target_atoms)` para diagramas de Ellingham (`DG`).
2. **Conversión de unidades**:
   - Temperatura en eje X: `°C` o `K`.
   - Energía / Entropía en eje Y: `kJ`, `kcal`, `J`, `cal` (por mol o por cantidad escalada).
3. **Interfaz gráfica (`diagrams_ui.rs`)**:
   - Tabla `X-data` editable, selector de `Diagram Type`, controles `DG` (`Element` + `Balance Element Amount`), botones `Read Data from Database` y `Diagram`, y lienzo gráfico con líneas sólidas/punteadas y etiquetas en curva.
