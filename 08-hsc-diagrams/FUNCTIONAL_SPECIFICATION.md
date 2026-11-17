# Functional Specification — Módulo 08: H, S, C and G Diagrams

## 1. Flujo de trabajo estándar (HSC6 Help §19)
1. El usuario ingresa fórmulas químicas (con o sin coeficiente inicial, ej. `Fe2O3` o `2 FeO`) en la primera columna de la hoja **`X-data`**.
2. Selecciona uno de los 8 tipos en **`Diagram Type`**:
   - `H Enthalpy (total)`
   - `H Enthalpy (latent)`
   - `S Entropy`
   - `Cp Heat Capacity`
   - `G Gibbs Energy`
   - `DH`
   - `DS`
   - `DG (Ellingham)`
3. **(Solo para `DG` / Ellingham)**:
   - Selecciona un elemento de la lista (`O`, `S`, `Cl`, `F`, `N`, `C`, etc.) y presiona **`Balance Element Amount`** para normalizar automáticamente los coeficientes de todas las especies respecto a la misma cantidad del elemento (ej. `1 mol O2` = `2 átomos O`).
4. Presiona **`Read Data from Database`**:
   - Lee y calcula las curvas para todas las especies válidas en el rango `[T_min, T_max]` con el paso definido.
   - Respeta la columna **`Database No`** (`Auto`, `1 = Own/User DB`, `2 = Main DB`).
   - Si el usuario modifica rangos o unidades de los ejes, debe volver a presionar **`Read Data from Database`** (o recalcular) antes de graficar.
5. Presiona **`Diagram`**:
   - Dibuja las curvas vs. temperatura: **línea sólida** dentro del rango válido en DB y **línea punteada** en zonas extrapoladas.
   - Muestra etiquetas de fórmula directamente sobre/junto a cada curva y permite inspeccionar valores $(T, Y)$ al pasar el cursor.

## 2. Configuraciones y Unidades
- **Eje X (Temperatura)**: `T min`, `T max`, `Step`, unidad `°C` o `K`.
- **Eje Y (Propiedad)**: `Y min`, `Y max` (auto o manual), unidad energética `kJ` / `kcal` / `J` / `cal`.
- **Exportación**: exportación CSV de la tabla de datos calculados $(T, Y_1, Y_2, \dots)$.
