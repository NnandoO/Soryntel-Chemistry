# Especificación Funcional — Módulo 12: Elements Module

## 1. Casos de Uso
1. **CU-01: Explorar y Filtrar Propiedades en Hoja de Cálculo (`Element4`)**
   - El usuario visualiza la matriz completa de ~56 propiedades (filas) × 118 elementos (columnas `H` a `Og`).
   - Puede activar **Fix Left 2 Columns** para mantener fijas las columnas `Property` y `Units` al desplazarse horizontalmente por los elementos.
   - Puede filtrar por texto de propiedad (p. ej. `Density`, `Melting`, `Electronegativity`, `Ionization`) o saltar directamente a un elemento (`Symbol` o `Z`).
2. **CU-02: Editar Celdas y Agregar Filas Personalizadas**
   - El usuario puede modificar valores en la tabla y agregar nuevas filas de propiedades personalizadas (p. ej. precio de mercado o límite normativo), guardando el libro (`File -> Save JSON/CSV`) y abriendo archivos guardados (`File -> Open`).
3. **CU-03: Dibujar Diagrama Periódico de Esferas Proporcionales (`Diagram`)**
   - Al seleccionar cualquier fila numérica y pulsar **Diagram**, se renderiza la Tabla Periódica completa (Grupos 1–18, Periodos 1–7 + Lantánidos y Actínidos) con esferas cuyo diámetro es proporcional al valor de cada elemento respecto al máximo de la fila.
   - Al pasar el ratón se muestra tooltip con `Z, Symbol, Name, Value, Unit`; al hacer **doble clic** en cualquier elemento se abre su ficha modal completa con todas sus propiedades y sus especies asociadas en `SpeciesCatalogPort`.
4. **CU-04: Gráfico de Tendencia Periódica (`Property vs Z`) y Comparación**
   - Muestra una curva/barras de la propiedad seleccionada frente al número atómico $Z$ (`1..118`) resaltando periodos (gases nobles/alcalinos) para analizar periodicidad.
   - Permite copiar al portapapeles la fila o ficha del elemento y exportar a CSV.

## 2. Reglas de Validación
- Al menos 1 valor numérico válido y finito para dibujar el diagrama de esferas; las filas puramente textuales (como `Crystal Structure` o `Electron Configuration`) muestran sus etiquetas resumidas en las casillas o advierten que no son cuantitativas para esferas.
- Cualquier celda editada por el usuario que no sea numérica en una fila numérica mantiene advertencia visual sin corromper estadísticas ni causar división por cero.
- Ninguna tabla o diagrama debe truncar datos sin barra de desplazamiento (`ScrollArea::both()`).
