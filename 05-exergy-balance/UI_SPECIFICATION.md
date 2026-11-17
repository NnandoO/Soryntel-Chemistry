# Especificación de Interfaz — Módulo 05: Exergy Balance

Basado en las capturas oficiales de **HSC Exergy Balance** (`Fig. 1` y `Fig. 2`).

## 1. Ventana y Menú Superior
- **Título de ventana:** `Exergy Balance`
- **Barra de menú:** `File | Edit | Units | Help`

## 2. Panel Superior de Control y Resumen
Debajo del menú se ubica una banda de herramientas dividida en 3 bloques horizontales:

1. **Bloque Izquierdo (Controles de edición y cálculo):**
   - Fila 1: `[Browse database]` | `[Insert stream]` | `[Input / Output ▾]` | `[Normal stream / Gas stream / Energy stream ▾]`
   - Fila 2: `[Delete stream]` | `[Delete species]` | `[Calculate]`
2. **Bloque Central (Indicadores globales):**
   - `Mass Balance (kg)` (ej. `-12.80`)
   - `Heat Balance (kJ)` (ej. `57851.57`)
   - `Exergy Balance (kJ)` (ej. `-271313.81`)
3. **Bloque Derecho (Nota metodológica fija):**
   > *"All exergy calculations are based on Szargut theory with the standard state being 25°C and 1 bar. Normal stream accepts any species. Gas stream accepts gas species and adds pressure S to exergy. Energy stream adds heat and electricity feed/loss. Carnot cycle is used to convert heat to exergy."*

## 3. Hoja de Cálculo Principal (`Calculations`)
Pestaña inferior única estilo Excel: `Calculations`.

### Columnas (`A`–`L`)
| Col | Encabezado | Unidad | Descripción |
|---|---|---|---|
| `A` | `Streams and species` | — | Nombre de corriente o fórmula química / flujo energético |
| `B` | `Mass` | `kg` | Masa de especie o suma de corriente |
| `C` | `Amount` | `mol` | Moles de especie (editable) o total de corriente |
| `D` | `Molfraction` | `fraction` | Fracción molar $x_i$ en la corriente |
| `E` | `T` | `K` (o `°C`) | Temperatura (editable en cabecera de corriente o fila `Heat feed`/`Heat loss`) |
| `F` | `P` | `bar` | Presión (editable en corrientes materiales) |
| `G` | `H Total` | `kJ` | Entalpía total $H(T)$ o flujo de calor/electricidad |
| `H` | `Ex_elemental` | `kJ/mol` | Exergía química estándar molar ($\text{ex}_{\text{chem}}^\circ$) |
| `I` | `Ex_che` | `kJ` | Exergía química de la especie / corriente |
| `J` | `Ex_phy` | `kJ` | Exergía física de la especie / corriente |
| `K` | `Ex_tot` | `kJ` | Suma $\text{Ex}_{\text{che}} + \text{Ex}_{\text{phy}}$ |
| `L` | `Ex_tot_stream` | `kJ` | Exergía total de corriente incluyendo mezcla $R T_0 \sum n_i \ln x_i$ o Carnot + electricidad |

### Tipos de Corrientes y Colores
- **Normal stream** (`Input Stream 1`, `Output Stream 1`): encabezado en **negro negrita**. Acepta cualquier especie. Incluye fila final `Insert/paste species here`.
- **Gas stream** (`Input Gas Stream 1`, `Output Gas Stream 1`): encabezado en **rojo negrita**. Acepta especies gaseosas y añade el término de presión a la exergía. Incluye fila final `Insert/paste species here`.
- **Energy stream** (`Input Energy Stream 1`, `Output Energy Stream 1`): encabezado en **verde negrita**.
  - En Input: filas `Heat feed` (`T` en Col `E`, `Q` en Col `G`) y `Electricity feed` (`W` en Col `G`).
  - En Output: filas `Heat loss` (`T` en Col `E`, `Q` en Col `G`) y `Electricity out` (`W` en Col `G`).
- **Separador Input / Output:** fila divisoria gris oscuro entre las corrientes de entrada (arriba) y las de salida (abajo).
