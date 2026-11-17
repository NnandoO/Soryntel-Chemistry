# HSC-REA / Soryntel Chemistry

Aplicación Rust de química, termodinámica y simulación de procesos reconstruida a partir de análisis de HSC Chemistry. El workspace está en `soryntel/`. La revisión científica está en curso; la existencia de un módulo o de pruebas automáticas no implica equivalencia certificada con HSC.

## Ejecución

Con Rust/Cargo y las dependencias gráficas de eframe disponibles:

```sh
cd soryntel
cargo run -p soryntel-desktop
cargo run -p soryntel-app -- balance "C + O2(g) = CO(g)"
cargo run -p soryntel-app -- calc "CO(g)" 25 1000 25
```

La aplicación `soryntel-app` sin subcomando abre un servidor local en el puerto 8765 para el módulo de reacciones. El escritorio contiene los veinte módulos. Ambos aceptan `--hsc-db <archivo>` para leer una base externa aportada por el usuario. El catálogo abierto incorporado tiene cobertura limitada: una especie ausente debe dar error, no sustituirse por datos inventados.

## Organización

- `soryntel-core`: constantes, unidades y errores compartidos.
- `soryntel-solver`: fórmulas químicas y balanceo.
- `soryntel-thermo`: modelos y cálculos de los módulos.
- `soryntel-db`: catálogo y lectura de bases HSC/OwnDB.
- `soryntel-application`: casos de uso públicos que conectan UI y dominio.
- `soryntel-desktop`: interfaz egui.
- `soryntel-app`: CLI y servidor local de reacciones.
- `chem_addin_core`: motor adicional para complementos de hojas de cálculo.

## Documentación y validación

El [plan](soryntel/PLAN_REVISION.md), la [especificación](.scratch/issues/001-revision-quimica-espanol.md) y el [registro de revisión](soryntel/docs/REVISION.md) describen alcance y estado. Las [referencias recuperadas](soryntel/docs/referencia/README.md) conservan las especificaciones históricas con hashes de procedencia.

```sh
cd soryntel
cargo test --workspace --offline
```

`--offline` requiere dependencias ya descargadas. En este entorno está bloqueada la apertura de puertos de la prueba `live_tcp_listener_roundtrip_eval_and_streameq`. Puede verificarse el resto explícitamente con `cargo test --workspace --offline -- --skip live_tcp_listener_roundtrip_eval_and_streameq`; esto no valida el transporte TCP. No se han regenerado instaladores: los binarios existentes no incorporan los cambios de esta revisión.

La revisión específica de [ecuaciones de reacción](soryntel/docs/modulo-01/REVISION.md) y la [auditoría hexagonal](soryntel/docs/AUDITORIA_HEXAGONAL.md) recogen cálculos, adaptación de la interfaz y límites arquitectónicos identificados.
