# Functional Specification — Módulo 09: Phase Stability Diagrams

## 1. Flujo de trabajo (`Tpp` & `Lpp`)
1. **Selección de 3 elementos**: el usuario selecciona exactamente 3 elementos (ej. `Cu`, `S`, `O` o `Ti`, `Cl`, `O`).
2. **Selección de especies condensadas**: lista filtrada de fases sólidas/líquidas que contienen al elemento principal (ej. `Cu`, `Cu2O`, `CuO`, `Cu2S`, `CuS`, `CuSO4`, `CuO*CuSO4`). Botones `Select All` / `Clear`.
3. **Configuración de ejes**:
   - **Modo de diagrama**: `p-p (Isothermal)` o `T-p (Temp vs Pressure)`.
   - **Eje X**:
     - En `p-p`: gas $G_1$ (ej. `SO2(g)` o `Cl2(g)`) y rango `log10(p)` (`Min`, `Max`, ej. `-20..0`).
     - En `T-p`: `Temperature` (`Min`, `Max`, unidad `°C` o `K`).
   - **Eje Y**: gas $G_2$ (ej. `O2(g)`) y rango `log10(p)` (`Min`, `Max`, ej. `-20..0`).
   - **Variable fija**:
     - En `p-p`: `Temperature` (`°C` o `K`, por defecto `25 °C` o `700 °C`).
     - En `T-p`: gas $G_1$ y su presión parcial fija (bar, por defecto `1.0E-20` o `1.0`).
4. **Opciones de visualización**:
   - `Grid`: cuadrícula punteada.
   - `Color`: relleno de áreas de estabilidad con paleta diferenciada por fase.
   - `Resolution`: `Low` (`100×100`), `Medium` (`180×180`), `High` (`260×260`).
5. **Salida tabular (`Data` / `Table`)**:
   - Lista de fases seleccionadas, $G^\circ(T)$, si es predominante en el rango, y segmentos de frontera `(X1, Y1) -> (X2, Y2)`.
6. **Exportación**: exportación CSV de las coordenadas de frontera.
