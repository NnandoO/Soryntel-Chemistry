# Manual de uso de Soryntel Chemistry

## Inicio y datos

Ejecute `soryntel-desktop` (Linux) o `soryntel-desktop.exe` (Windows). Seleccione un módulo en el panel principal; cada módulo se abre en su propia ventana. El indicador verde señala ventanas abiertas. La vista de tarjetas ofrece una descripción y filtros por categoría. La versión se obtiene del paquete Rust.

El catálogo abierto incorporado tiene cobertura limitada. Una fórmula ausente requiere aportar datos o una base externa mediante `--hsc-db <archivo>`. Los registros HSC externos se leen sin modificar el archivo original. La prioridad y edición de OwnDB se gestionan en el módulo de especies. Compruebe fórmula, fase, carga, intervalos térmicos y referencias antes de calcular.

Use símbolos químicos convencionales y distinga fases como `CO(g)` y `H2O(l)`. Escriba los números con punto decimal en las entradas que todavía usan el analizador numérico de Rust; las fórmulas y los formatos de intercambio conservan su sintaxis original. La traducción al español está en curso: navegación y muchos controles están traducidos, pero algunas hojas, ayudas y mensajes siguen en inglés.

## Flujo de trabajo

1. Seleccione el módulo y cargue un ejemplo disponible o introduzca sus datos.
2. Confirme unidades, base de cantidad, temperatura, presión y modelo.
3. Ejecute el cálculo. Si aparecen errores, corrija la entrada antes de usar el resultado.
4. Revise conservación, convergencia, extrapolación y disponibilidad de datos.
5. Guarde el proyecto del módulo y exporte las tablas o gráficos que necesite. Revise la ruta indicada: algunas exportaciones usan el directorio temporal del sistema.

## Los veinte módulos

| Módulo | Trabajo principal y datos que deben revisarse |
|---|---|
| Reacciones | Fórmula o reacción balanceada, intervalo de T, unidades y fases. Permite balancear y tabular propiedades |
| Balance de materia y energía | Corrientes, cantidades y temperaturas; revise cierres y objetivos adiabáticos |
| Pérdidas de calor | Geometría, capas, conductividad, condiciones de contorno, convección y radiación |
| Equilibrio | Especies, cantidades, fases, actividad, T/P y número de iteraciones; revise cada punto sin convergencia |
| Exergía | Corrientes, estado ambiental y base de exergía química; confirme el estado de referencia |
| Estimación | Método y datos disponibles para estimar H, S y Cp; distinga estimación de valor tabulado |
| Eh-pH | Sistema, especies, actividades y límites del agua; revise los supuestos de solución |
| Diagramas termoquímicos | Propiedad, especies/reacciones y rango; verifique las discontinuidades de fase |
| Estabilidad de fases | Componentes, temperatura y presiones o actividades; interprete los dominios según el modelo |
| Conversor de especies | Composición elemental y especies posibles; examine el balance y la no negatividad |
| Benson | Estructura molecular y grupos; un grupo sin parámetros ahora impide generar propiedades inventadas |
| Elementos | Propiedades elementales, tabla periódica y consultas; compruebe la fuente de cada propiedad |
| Agua y vapor | Dos estados, cantidad y unidades; la saturación y las propiedades aproximadas tienen alcances diferentes |
| Unidades | Magnitud física, unidad inicial y unidad final; distinga temperatura absoluta de diferencia térmica |
| Complementos | Funciones, fórmulas de hoja y exportaciones; conserve los identificadores de función originales |
| Especies | Catálogo, referencias, OwnDB y ajuste/conversión de coeficientes |
| Aqua | Especies iónicas, molalidad, temperatura y parámetros; compruebe la disponibilidad de interacciones |
| Simulación | Unidades de proceso y corrientes; revise reciclos, escenarios, convergencia y balances globales |
| Ciclo de vida | Inventario, método y factores de caracterización; confirme procedencia y unidades de factores |
| Balance de masa | Mediciones, incertidumbres, restricciones y reconciliación; examine residuales y factibilidad |

## Mensajes y límites concretos

- Reacciones admite hasta 100001 puntos de malla; los puntos de transición pueden añadirse aparte. Pasos demasiado pequeños y valores no finitos dan error.
- Equilibrio rechaza T ≤ 0 K y P ≤ 0 bar. Un resultado calculado puede quedar sin convergencia confirmada si los coeficientes de actividad no se estabilizan o el balance no cierra. Aumentar iteraciones no garantiza que el modelo sea adecuado.
- Agua y vapor usa una correlación IAPWS para presión de saturación, pero **no implementa IF97 completo** para todas las propiedades. Los límites numéricos de entrada (3–2273,15 K y normalmente 0,001–1000 bar) no constituyen un rango de exactitud validado. En coexistencia gas/líquido o gas/sólido, el Cp=0 mostrado es una convención de compatibilidad del programa y no una capacidad calorífica física de la transición.
- Las entradas inválidas de agua conservan el último resultado válido con un mensaje visible; corríjalas antes de exportar.
- Las tablas térmicas requieren valores finitos y abscisas estrictamente crecientes. Los errores de interpolación se comunican al usuario.
- Los archivos guardados desde el escritorio se preparan en el mismo directorio y se reemplazan al terminar. Esto reduce el riesgo de archivos truncados; no sustituye una copia de seguridad ni garantiza recuperación tras fallos físicos del almacenamiento.

## Línea de comandos y web local

```sh
soryntel-app --help
soryntel-app balance "C + O2(g) = CO(g)"
soryntel-app calc "CO(g)" 25 1000 25
soryntel-app --port 8765
```

La CLI de cálculo usa °C y kJ. Los parámetros numéricos mal escritos se rechazan. El servidor se abre en `127.0.0.1`; no sirve los veinte módulos, sino el módulo de reacciones. En la web, las constantes de equilibrio que exceden el rango del número flotante se muestran mediante su logaritmo.

## Paquetes y verificación

Los nuevos paquetes portables contienen ejecutable, icono, este manual y un manifiesto con hashes. El archivo `.sha256` permite comprobar el paquete descargado. El manifiesto identifica el commit y si la compilación incluye cambios sin commit. Descomprima en una carpeta propia; esta variante portable no modifica la instalación del sistema. Los instaladores antiguos del directorio `dist` no se actualizan automáticamente con el código.
