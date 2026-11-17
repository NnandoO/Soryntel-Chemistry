# UI_SPECIFICATION — Reaction Equations (Soryntel Chemistry)

## 1. Filosofía de Interfaz y Arquitectura Moderna
La interfaz de usuario original de HSC Chemistry 6.0 data de entornos Win32 legados basados en controles ActiveX y Visual Basic 6. 
Para **Soryntel Chemistry**, la interfaz se implementará utilizando **Tauri v2** acoplado a un frontend reactivo moderno (TypeScript + React / SolidJS) y componentes de alta fidelidad estilizados con Tailwind CSS, garantizando aceleración por hardware (WebGL), diseño responsivo, soporte nativo de temas claros/oscuros y renderizado vectorial SVG/Canvas interactivo.

---

## 2. Layout y Estructura Visual de la Ventana

La interfaz del módulo se divide en cuatro áreas funcionales principales:

```
+--------------------------------------------------------------------------------------------------+
| BARRA SUPERIOR DE HERRAMIENTAS Y MENÚS (Archivo, Edición, Formato, Hoja, Opciones, Ayuda)     |
+--------------------------------------------------------------------------------------------------+
| PANEL DE CONFIGURACIÓN Y ESPECIFICACIÓN (Header Form)                                            |
|  [ Ecuación / Fórmula Química: _____________________________________ ] [ Balance ] [ Factor: 1 ] |
|  [ Rango Térmico: Desde: [  0.0 ] Hasta: [ 1000.0 ] Paso: [ 100.0 ] Unid: [°C v] ] [ Buscar DB ]|
|  [ Opciones rápidas: [x] Mostrar Transiciones  [x] Criss-Cobble  Formato: (o) Normal  ( ) Delta] |
|                                                                    [ Botón CALCULAR (Hero) ]     |
+--------------------------------------------------------------------------------------------------+
| VISTA DIVIDIDA (SPLIT-VIEW FLEXIBLE / PESTAÑAS)                                                 |
| +------------------------------------------------------+---------------------------------------+ |
| | TABLA NUMÉRICA REACTIVA                              | VISUALIZADOR GRÁFICO CIENTÍFICO       | |
| | [Pestaña 1: Mond] [Pestaña 2: Mg Vap] [+]            | (Apache ECharts / Plotly.js / Canvas) | |
| | T (°C) | deltaH | deltaS | deltaG | K | log(K) | E°  |  - Curvas Cp, H, S, G, K, log(K), E°   | |
| |--------+--------+--------+--------+---+--------+-----|  - Marcadores de transiciones         | |
| | ...    | ...    | ...    | ...    |...| ...    | ... |  - Zoom interactivo, tooltips SVG     | |
| |------------------------------------------------------|  - Exportar PNG, SVG, CSV, Excel      | |
| | TABLA DE BALANCES ESTEQUIOMÉTRICOS Y PESOS FÓRMULA   |  - Opciones de trazado múltiple       | |
| | Reactivos / Productos: Formula, PM, wt-%, mol, g, V  |    ("Plot All Worksheets")            | |
| +------------------------------------------------------+---------------------------------------+ |
+--------------------------------------------------------------------------------------------------+
| BARRA DE ESTADO (Status Bar): Estado termodinámico, base activa (Own/Main), advertencias numéricas|
+--------------------------------------------------------------------------------------------------+
```

---

## 3. Especificación de Componentes y Controles

### 3.1. Barra de Herramientas Superior (*Toolbar & Menu Bar*)
- **Archivo (`File`):**
  - `Nuevo Cálculo`: Reinicia los campos a sus valores por defecto.
  - `Abrir Proyecto (.rea8 / .simjson)`: Carga el estado serializado del cálculo.
  - `Guardar / Guardar Como`: Guarda el libro de trabajo completo en formato abierto JSON o exportación compatible.
  - `Exportar a Excel (.xlsx)`: Exportación directa de todas las hojas numéricas con estilos y fórmulas.
  - `Exportar a CSV`: Exportación de texto delimitado por comas.
  - `Imprimir / Exportar Reporte PDF`: Generación de informe técnico en PDF con tablas y gráficos vectoriales embebidos.
- **Edición (`Edit`):**
  - `Copiar Fila / Selección`: Copia al portapapeles en formato TSV.
  - `Copiar Hoja Completa`: Transfiere la tabla entera con encabezados.
  - `Copiar Gráfico`: Exporta el gráfico renderizado al portapapeles como mapa de bits PNG o vector SVG.
- **Opciones de Cálculo (`Options`):**
  - `Unidades de Temperatura`: Selector mutuamente excluyente:
    - Celsius ($^\circ\text{C}$, por defecto)
    - Kelvin ($\text{K}$)
    - Fahrenheit ($^\circ\text{F}$)
  - `Unidades de Energía`:
    - Calorías ($\text{cal}$, $\text{kcal}$)
    - Joules ($\text{J}$, $\text{kJ}$, por defecto en Soryntel)
  - `Formato de Resultados (Pure Species)`:
    - Normal (Escala absoluta Barin-Knacke-Kubaschewski)
    - Delta (Entalpías y energías de formación relativas a elementos)
  - `Show Phase Transitions`: Checkbox booleano para intercalar puntos de discontinuidad de fase.
  - `Criss-Cobble Extrapolation`: Checkbox booleano para extrapolar $C_p$ de especies acuosas diluidas hasta $300\,^\circ\text{C}$.

### 3.2. Panel de Entrada de Datos (*Specification Input Panel*)
- **Campo de Fórmula / Ecuación Química:**
  - `Input Text` reactivo con resaltado de sintaxis química (soporte de subíndices, superíndices, fases entre paréntesis `(s)`, `(l)`, `(g)`, `(a)`, y cargas `(+2a)`, `(-a)`, `e-`).
  - Validación sintáctica en tiempo real con indicador visual de validez (verde/rojo) y tooltip explicativo.
- **Botón `Balance`:**
  - Activa la resolución estequiométrica elemental. Si la ecuación está incompleta o carece de coeficientes, calcula y actualiza los coeficientes directamente en el campo de texto.
- **Selector `Smallest Coefficient`:**
  - Entrada numérica entera/flotante positiva (default `1.0`). Permite normalizar la reacción respecto a la especie limitante o al menor coeficiente entero.
- **Campos de Rango Térmico:**
  - `From`: Límite térmico inferior (validación: $T \ge 0\,\text{K}$).
  - `To`: Límite térmico superior ($T_{\text{max}} \ge T_{\text{min}}$).
  - `Step`: Incremento térmico ($\Delta T > 0$).
- **Botón `Browse Database`:**
  - Abre un modal/cajón lateral (*Drawer*) con el visor de especies de la base de datos de Soryntel (`soryntel-db`), permitiendo filtrar por elementos constituyentes, fórmula o nombre químico, e importar directamente a la ecuación activa.

### 3.3. Tabla Numérica Reactiva (*Data Grid*)
- **Virtualización:** Implementada mediante virtual scrolling (`tanstack-table` o `ag-grid` / renderizado nativo Tauri) para soportar miles de pasos térmicos sin degradación de rendimiento.
- **Intercalación de transiciones de fase:**
  - Filas de transición visualmente diferenciadas con un fondo sutil o etiqueta destacada (ej. `[Transición $\alpha \to \beta$ a $1042\,\text{K}$]`).
  - Muestra dos evaluaciones continuas al límite: $T_{\text{tr}}^-$ (fase previa) y $T_{\text{tr}}^+$ (fase posterior con absorción de calor latente $\Delta H_{\text{tr}}$).
- **Formateo numérico científico:**
  - Notación exponencial científica para números de magnitud extrema (ej. constantes de equilibrio $K = 2.222 \times 10^{45}$).
  - Precisión configurable por el usuario (3 a 6 cifras decimales).

### 3.4. Visor Gráfico Científico (*Interactive Charting Canvas*)
- **Variables seleccionables en el eje Y:**
  - Selector desplegable o pestañas: $\Delta H$, $\Delta S$, $\Delta G$, $C_p$, $K$, $\log_{10}(K)$, $E^\circ$.
  - Eje X: Temperatura $T$ en la unidad seleccionada.
- **Interacciones integradas:**
  - Zoom dinámico (caja de arrastre y rueda de ratón), pan (desplazamiento lateral) y reset de vista.
  - Tooltip inspectivo con interpolación exacta de coordenadas al posar el cursor.
  - Soporte de trazado conjunto (*Plot all sheets*): Superpone las curvas de cálculo acumuladas en diferentes pestañas para análisis comparativo de procesos (ej. presiones de vapor de múltiples metales o equilibrios de reducción concurrentes).
