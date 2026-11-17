# Auditoría REA — Módulo 05: Exergy Balance

## 1. Evidencia en el entorno local (`HSC6`) y capturas (`Fig. 1` y `Fig. 2`)
- **`Help/33 Data References.txt` (línea 844):** cita `Szargut 1988` (*Exergy analysis of thermal, chemical, and metallurgical processes*).
- **Capturas oficiales (`Fig. 1` y `Fig. 2`):** confirman el diseño exacto del módulo **Exergy Balance**:
  - Estado de referencia estándar: $25^\circ\text{C}$ ($298.15\text{ K}$) y $1\text{ bar}$.
  - Hoja única `Calculations` con columnas `A..L`: `Streams and species`, `Mass (kg)`, `Amount (mol)`, `Molfraction`, `T (K)`, `P (bar)`, `H Total (kJ)`, `Ex_elemental (kJ/mol)`, `Ex_che (kJ)`, `Ex_phy (kJ)`, `Ex_tot (kJ)`, `Ex_tot_stream (kJ)`.
  - Tres tipos de corriente: `Normal stream` (negro), `Gas stream` (rojo, añade presión $S$ a la exergía) y `Energy stream` (verde, `Heat feed/loss` convertido por Carnot $Q(1 - T_0/T)$ y `Electricity feed/out`).
  - Cabecera superior con `Mass Balance (kg)`, `Heat Balance (kJ)` y `Exergy Balance (kJ)` en convención `OUT - IN`.

## 2. Estado actual en `soryntel-thermo`
En `crates/soryntel-thermo/src/balance.rs` ya existe:
- `element_standard_chemical_exergy_kj_mol(symbol)` para 17 elementos de Szargut (1988).
- `species_chemical_exergy_kj_mol` y `evaluate_balance_point_si`.

## 3. Brechas detectadas para implementar el Módulo 5
1. **Estado muerto $P_0 = 1.0\text{ bar}$** en Exergy Balance (en `balance.rs` actual $P_0 = 1.01325\text{ bar}$) y temperatura en $\text{K}$ por defecto en la hoja `Calculations`.
2. **Exergía de mezcla en `Ex_tot_stream`:** sumar $R T_0 \sum n_i \ln x_i$ sobre `Ex_tot` para corrientes con múltiples especies.
3. **Corrientes de Energía (`Energy stream`):** soporte de `Heat feed` / `Heat loss` con conversión Carnot $Q(1 - T_0/T_K)$ y `Electricity feed` / `Electricity out`.
4. **Tabla periódica ampliada de Szargut** para cubrir más elementos de `Maindb6`.
5. **Motor y UI dedicados (`exergy.rs` + `exergy_ui.rs`)** replicando `Fig. 1` y `Fig. 2` en su propia ventana independiente (Módulo 05 en el Launcher).
