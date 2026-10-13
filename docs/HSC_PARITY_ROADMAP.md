# Ruta hacia una alternativa funcional a HSC Chemistry

Objetivo: ampliar capacidades científicas verificables, no copiar datos propietarios ni afirmar equivalencia sin evidencia.

## Entrega actual

- Núcleo separado de persistencia y escritorio, con comprobación automática de dependencias.
- Paneles, aplicación y lienzo divididos por responsabilidades; tamaño no equivale por sí solo a arquitectura hexagonal.
- Correcciones de integración temporal, ajuste estadístico, balances, CSV y equilibrio Gibbs.
- Agua/vapor mediante CoolProp IF97 MIT, revisión fijada en `third_party/if97/README.md`.
- Importador MINTEQ nativo con fuentes y estequiometría: log K y entalpía de reacción NO son Hf/Sf absolutos.
- Motor nativo ideal/Davies integrado, a 298.15 K, con balances de componentes y pH fijado. Reporta carga residual; no impone electroneutralidad ni precipitación.
- Adaptador USGS IPhreeqc separado: asociación iónica/Pitzer parametrizado, ajuste de carga con Na y equilibrio con calcita/CO2. Referencia numérica USGS 3A reproducida. No es HMW independiente.
- Clasificación granulométrica, flotación batch/flujo pistón y molienda Bond/CSS conectadas al flowsheet general con soporte de corrientes particuladas (minerales, tamaño y agua), y convergencia de circuitos cerrados con reciclo mediante aceleración Wegstein.
- Analizador de fórmulas ampliado con soporte de aductos (*, ·, •), polímeros y cargas iónicas completas.
- Ajuste log-lineal de k desde CSV tiempo-recuperación, con R infinito medido y RMSE. No certifica extrapolación industrial.
- Auditoría de procedencia del catálogo heredado; registros no verificados no se convierten automáticamente en datos fiables.
- Pruebas de referencia IF97, equilibrio analítico y conservación mineral. Validación numérica no significa calibración industrial.
- Datos GeoMet v4 CC BY 4.0 descargados y conservados sin modificaciones; 60 ensayos de molienda y 52 recuperaciones LCT disponibles (un faltante no imputado).
- Exportación SQLite de procedencia/rangos desconocidos explícitos; CI local preparada para escritorio/referencias y núcleo con ASan/UBSan. Aún no ejecutada en GitHub.

## Fases y criterios de aceptación

| Fase | Trabajo pendiente | Criterio de salida |
| --- | --- | --- |
| 1. Datos fiables | Revisar cada Hf/Sf/Cp, fase, unidad, fuente, rango y transiciones del catálogo heredado; completar lectura/edición de metadatos | Fuentes recuperables y comparación independiente; ninguna extrapolación silenciosa |
| 2. Equilibrio avanzado | Ampliar casos y componentes de IPhreeqc; validar interacciones, rangos y fases de cada sistema; mejorar exportación | Casos publicados con balances y tolerancias documentadas; no extrapolar la cobertura limitada |
| 3. Procesamiento mineral | Conectar corrientes por mineral/tamaño, molienda, clasificación, flotación y reciclajes | Conservación por componente y agua; convergencia de circuitos; calibración con datos experimentales |
| 4. Diagramas y estimación | Fases/soluciones no ideales, diagramas multicomponente y grupos Benson documentados | Comparación con casos de referencia; errores y límites visibles |
| 5. Producto | Flujos reproducibles, importación/exportación, documentación, CI y empaquetado | Pruebas automáticas y casos de usuario completos en una instalación limpia |

## Bloqueadores de equivalencia

Las fuentes públicas obtenidas permiten referencias acuosas y verificación de entradas geometalúrgicas, pero no sustituyen series cinéticas y balances completos de una planta. Faltan referencias/licencias de buena parte del catálogo heredado y parámetros para otros sistemas/modelos. No se deben inventar. La base actual no está certificada como equivalente a la base comercial de HSC.

## Reproducir verificaciones

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j 3
ctest --test-dir build --output-on-failure
./build/cma_catalog_audit data > build/catalog_audit.json
./build/cma_chemistry
```

Referencias: [IAPWS IF97](https://iapws.org/technical-guidance/release/IF97-Rev), [CoolProp IF97](https://github.com/CoolProp/IF97), [MINTEQ EPA](https://www.epa.gov/sites/default/files/documents/SUPPLE1.PDF), [HSC Chemistry](https://www.metso.com/portfolio/hsc-chemistry/).
