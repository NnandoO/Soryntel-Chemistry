# Análisis del Manual Oficial — Módulo 3: Heat Loss (`12-heat-loss.md`)

**Fuente:** [`12-heat-loss.md`](file:///home/nando/Documentos/Docs%20HSC/12-heat-loss/12-heat-loss.md) (1,371 líneas, 37 páginas, Metso Finland Oy, Petri Kobylin & Peter Björklund, 6 de septiembre de 2023).

---

## 1. Estructura General del Manual

| Sección | Páginas | Contenido principal |
|---|---|---|
| **12.1 Basic Calculation Procedure** | 1–4 | Flujo de trabajo: geometría (`Wall`, `Cube`, `Cylinder`, `Sphere`), columnas (`Surface`, `Enclosure`, `Layer`, `Layer contact`), valores fijos (`Fix Value`, fuente roja), modos `Temperature profile` y `Heat flow`, cálculo por hoja o libro completo. |
| **12.2 Examples** | 5–7 | Casos incluidos: `SimpleCase`, `Radiator`, `incropera94_503`, `incropera94_504`, `Hotwatertank`, `OilFurnace`, `Reactor1`, `Reactor2`, `Smelting1`–`Smelting3`. |
| **12.3 Detailed Description** | 8–21 | Detalle de la hoja de cálculo (filas visibles e internas 96–399 y 400+), bases de datos `HeatCond.XLS`, `HeatConv.XLS`, `HeatRadi.XLS`, calculadoras de radiación de gas y partículas, diálogos `Target` y `Diagram`, limitaciones (12.3.11). |
| **12.4 Basic Theory** | 22–36 | Fundamentos (Ecs. 1–7), Conducción (12.4.1, Ecs. 8–12), Convección libre y forzada (12.4.2, Ecs. 13–40), Radiación superficial, de mezcla de gases (EWBM) y gas+partícula (12.4.3, Ecs. 41–53, Tabla 1). |
| **12.5 References** | 37 | 10 referencias bibliográficas científicas y técnicas (Incropera & DeWitt, Taine & Petit, Siegel & Howell, Themelis, Edwards & Balakrishnan, Lallemant & Weber, VDI Heat Atlas, Haar et al., Jokilaakso, Edwards 1983). |

---

## 2. Procedimiento Básico de Cálculo (Sección 12.1)

1. **Geometría y dimensiones internas:**
   - Tipos disponibles: `Wall` (`Width`, `Length`), `Cube` (`Width`, `Length`, `Height`), `Cylinder` (`Height`, `Diameter`), `Sphere` (`Diameter`).
   - Las dimensiones ingresadas corresponden siempre a la **superficie interior** del sistema (el área externa se incrementa automáticamente con el espesor acumulado en geometrías cilíndricas, esféricas y cúbicas).
2. **Tipos de columna (`Layer Type`):**
   - `Surface`: película fluida/radiativa en la frontera interna o externa (contiene $h_c$, $h_r$ y materiales de fluido/superficie).
   - `Layer` (*Solid Layer* en HSC8): capa sólida conductiva con espesor, malla de cálculo (*Calculation Grid*, por defecto 10) y conductividad $k(T)$.
   - `Enclosure`: cavidad cerrada entre dos paredes (espacio de aire o gas confinado con convección natural interna + radiación entre paredes).
   - `Layer contact`: resistencia térmica de contacto interfacial (se fija directamente la resistencia térmica en fila 15).
3. **Valores fijos (`Fix Value`) vs. iterativos:**
   - El botón `Fix Value` alterna el estado fijo de la celda seleccionada (mostrado en **fuente roja**):
     - En `Surface`: $h_c$ (fila 13) y $h_r$ (fila 14) pueden fijarse manualmente o calcularse desde `HeatConv`/`HeatRadi`/`GasRad`.
     - En `Layer`: $k$ promedio (fila 12) o conductividades específicas pueden fijarse.
     - En `Layer contact`: la resistencia térmica (fila 15) se fija manualmente.
     - Otras filas documentadas con soporte de fijado/parametrización: filas 50, 61–62, 69.
4. **Modos de cálculo:**
   - **`Temperature profile`:** Conocida **1 temperatura fija** (típicamente temperatura interior o ambiente) y el **flujo de calor total fijado** (`Total Heat Flow` conocido), calcula el perfil completo de temperaturas en todas las capas e interfaces.
   - **`Heat flow`:** Conocidas **2 temperaturas fijas** (por ejemplo, temperatura del fluido interior y temperatura del fluido exterior, o selección de rango `C8:F9`), calcula las temperaturas intermedias de interfaz, los coeficientes $h_c(T), h_r(T), k(T)$ acoplados y el flujo de calor estacionario $Q$.
5. **Dirección y alcance:**
   - Indicador `HOT => COLD` (flujo térmico desde el lado caliente al frío; también soporta flujo inverso si $T_{\text{izq}} < T_{\text{der}}$).
   - `Calculate active sheet` calcula solo la hoja actual; `Calculate all sheets` suma el `Total Heat Flow` de todas las hojas del libro (útil para modelar `Wall`, `Roof` y `Bottom` de un mismo equipo, como en `Hotwatertank.htr8`).

---

## 3. Casos de Ejemplo Documentados (Sección 12.2)

- **`SimpleCase.htr8`:** Pared plana de $A = 1.5\text{ m}^2$, tres capas/elementos en serie ($700 \to 418 \to 50.63 \to 20\,^\circ\text{C}$), flujo total exacto $q = 735\text{ W}$ (verificación analítica manual directa).
- **`Radiator.htr8`:** Radiador de agua caliente con hojas `Radiator 40°C`, `Radiator 50°C`, `Radiator 60°C` ($2.00 \times 0.50\text{ m}$, acero al carbono $3\text{ mm}$ pintado blanco acrílico, $40\,^\circ\text{C} \to 20\,^\circ\text{C}$, $q = 0.174\text{ kW}$ a $40\,^\circ\text{C}$). Usado en ejemplos de `Target` ($0.5\text{ kW} \implies \text{Width} = 5.747\text{ m}$) y `Diagram` ($30\text{–}90\,^\circ\text{C}$).
- **`incropera94_503.htr8` y `incropera94_504.htr8`:** Tubería cilíndrica a $165\,^\circ\text{C}$ validada contra ejemplos de Incropera & DeWitt (1996).
- **`Hotwatertank.htr8`:** Tanque cilíndrico vertical ($H = 4.00\text{ m}$, $D = 3.00\text{ m}$) con 3 hojas (`Wall`, `Roof`, `Bottom`), pared de acero inoxidable ($5\text{ mm}$) + espuma de uretano ($50\text{ mm}$) + revestimiento de aluminio (`Aluminium (h)`), $50\,^\circ\text{C} \to 20\,^\circ\text{C}$, pérdida total $q = 1.378\text{ kW}$. Incluye estudio de espesor óptimo de aislamiento en `Diagram` ($\approx 33\text{ cm}$).
- **`OilFurnace.htr8`:** Horno con radiación de gases de combustión ($\text{H}_2\text{O(g)} = 11.08\%$, $\text{CO}_2\text{(g)} = 11.69\%$, $\text{SO}_2\text{(g)} = 0.05\%$).
- **`Smelting1.htr8`–`Smelting3.htr8`:** Pared de horno de fundición ($1.00 \times 1.00\text{ m}$, metal fundido $1200\,^\circ\text{C}$, refractario `REXAL 2S EXTRA` $1.000\text{ m}$, refrigeración por agua/cobre $20\,^\circ\text{C}$, $h_{c,1}=144\text{ W/(m}^2\text{K)}$ fijo, $h_{c,3}=2400\text{ W/(m}^2\text{K)}$ fijo, $q = 4.251\text{ kW}$).
- **`Reactor1.htr8`, `Reactor2.htr8`:** Reactores multicapa con radiación y convección combinadas.

---

## 4. Descripción Detallada y Submódulos (Sección 12.3)

- **Estructura interna de la hoja de cálculo:**
  - Filas `1–22`: zona visible de configuración (`Layer Type`, `Layer Number`, `Layer Material`, `Surface Material`, `Layer Specifications`, `Results`).
  - Filas `92–93`: límites de temperatura de validez de la base de datos (`Tmin`, `Tmax`).
  - Filas `96–399` (ocultas/desplazables): tablas interpolables de conductividad/convección (`Conduction / Convection Data`: $T$, $\rho$, $c_p$, $k$, $\mu$, $\nu$, $Pr$, $\beta$).
  - Filas `400+` (ocultas): datos de radiación superficial/gas/partícula.
  - Fila `38`: parámetro `Speed` (velocidad del fluido para convección forzada).
- **Submódulos / Diálogos:**
  1. **Conduction (`HeatCond.XLS`):** Selección de material sólido e inserción de tabla $k(T)$ en filas 96+.
  2. **Convection (`HeatConv.XLS`):** Pestaña `Table` (propiedades termofísicas del fluido + geometría/orientación/velocidad para correlaciones $Nu$) y pestaña `Function` ($h_c(v) = A v^a + B v^b + \dots$).
  3. **Surface Radiation (`HeatRadi.XLS`):** Selección de emisividad superficial $\varepsilon_s$; distingue medición normal `(n)` y hemisférica `(h)`.
  4. **Gas Mixture Radiation Calculator:** Mezcla de hasta 6 gases radiantes ($\text{H}_2\text{O}, \text{CO}_2, \text{CO}, \text{NO}, \text{SO}_2, \text{CH}_4$) + gas no radiante (p.ej. $\text{N}_2$); modos `Select at fixed T` y `Select iterative` ($0\text{–}3000\,^\circ\text{C}$ en pasos de $50\,^\circ\text{C}$).
  5. **Particle Radiation Calculator:** Emisividad de nube de partículas $\varepsilon_c$ y combinación gas + partícula ($\varepsilon_{g+c}, \alpha_{g+c}$).
  6. **Target Dialog:** Búsqueda unidimensional de una variable independiente (p.ej. ancho `Width` o espesor) para alcanzar un flujo de calor objetivo.
  7. **Diagram Dialog:** Generación de tablas y gráficos paramétricos (perfiles $T(x)$, barridos de temperatura o espesor de aislamiento).
- **Limitaciones documentadas (Sección 12.3.11):**
  - Modelo estrictamente **estacionario** y **unidimensional (1D)** por hoja (sin conducción lateral 2D/3D ni efectos de esquina más allá de los factores de forma de `Cube`, `Cylinder` y `Sphere`).
  - En `Enclosure`, el transporte convectivo y radiativo se modela en paralelo entre dos paredes planas, cilíndricas concéntricas o esféricas concéntricas.
  - El EWBM de gases aplica en $T \in [300, 2500\text{–}3000]\text{ K}$ y $p \in [0.5, 20]\text{ bar}$.
