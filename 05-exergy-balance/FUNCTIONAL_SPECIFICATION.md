# Especificación Funcional — Módulo 05: Exergy Balance

Basada en `Fig. 1` y `Fig. 2` de **HSC Exergy Balance**.

## 1. Propósito del Módulo
Calculadora dedicada de balances de masa, entalpía y exergía (física, química, mezcla, calor por Carnot y electricidad) basada en la teoría de Szargut con estado de referencia estándar $25^\circ\text{C}$ ($298.15\text{ K}$) y $1\text{ bar}$.

## 2. Operaciones Principales
1. **Gestión de Corrientes (`Insert stream` / `Delete stream`):**
   - Seleccionar lado: `Input` o `Output`.
   - Seleccionar tipo: `Normal stream`, `Gas stream` o `Energy stream`.
   - `Delete stream` elimina la corriente seleccionada.
2. **Gestión de Especies (`Browse database` / `Insert/paste species here` / `Delete species`):**
   - `Browse database` abre el buscador de especies de `Maindb6` para insertar en la corriente seleccionada (en `Gas stream` filtra/acepta especies gaseosas).
   - Edición directa en la celda `Insert/paste species here`, `Amount (mol)`, `T (K)` y `P (bar)`.
   - `Delete species` elimina la fila de especie seleccionada.
3. **Unidades (`Units`):**
   - Temperatura en `K` o `°C`, Energía en `kJ` / `MJ` / `kcal` / `kWh`, Cantidad en `mol` / `kmol`.
4. **Cálculo (`Calculate`):**
   - Recalcula todas las filas de especies, totales por corriente (`Mass`, `Amount`, `H Total`, `Ex_che`, `Ex_phy`, `Ex_tot`, `Ex_tot_stream`) y los 3 balances globales superiores (`Mass Balance`, `Heat Balance`, `Exergy Balance`).
