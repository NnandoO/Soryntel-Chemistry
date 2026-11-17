# Especificación Funcional — Módulo 07: Eh-pH Diagrams

Basada en `17 Eh - pH - Diagrams.txt` de HSC6.

## 1. Flujo de Trabajo Principal
1. **Especificación del Sistema Químico:**
   - Seleccionar **Elemento Principal** (ej. `Cu`, `Fe`, `Mn`) y **Elementos Secundarios** (ej. `S`, `Cl`, `C`) en solución acuosa (`H2O` implícito).
   - Definir hasta 4 temperaturas (`T1..T4` en `°C` o `K`, con extrapolación Criss-Cobble opcional).
   - Recuperar automáticamente desde `Maindb6` todas las especies formadas por esos elementos.
2. **Menú de Diagrama (`Species`, `Molality`, `Escalas`):**
   - Visualizar y editar la tabla de especies y valores $\Delta G^\circ(T)$, insertar especies manuales o eliminar especies por barreras cinéticas.
   - Configurar `Min/Max pH` (defecto `0..14`), `Min/Max Eh` (defecto `-2..2 V`), molalidades (`mol/kg H2O`) y presión total (`bar`).
   - Activar opcionalmente `Show Predominance Areas of Ions` (diagrama acuoso azul superpuesto al diagrama total negro).
3. **Diagrama Eh-pH Interactivo:**
   - Dibujar regiones de estabilidad etiquetadas, fronteras sólidas/acuosas, líneas punteadas de estabilidad del agua y tooltip con especie dominante en `(pH, Eh)`.
4. **Diagramas Combinados (`Combine`):**
   - Superponer hasta 4 `Data Sets` variando una variable (`Molality`, `Temperature`, `Pressure` o `Main Element`) con etiquetas de línea (`Line Text`).
   - Ver tabla numérica de segmentos y coordenadas (`Table`: `DS1..DS4`, `Label`, `Lines`).
