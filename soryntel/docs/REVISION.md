# Registro de revisión — 17 de noviembre de 2026

## Alcance y evidencia

La raíz Git es HSC-REA y el workspace está en soryntel. La documentación numerada está presente en la raíz, pero sus patrones están ignorados. La revisión anterior confundió esa situación con documentación externa: queda corregida en el plan y en la guía.

Se recuperaron 156 documentos de los 20 módulos en `referencia/`, con rutas de origen y SHA-256. Son especificaciones históricas sin certificación: por ejemplo, la especificación de equilibrio menciona nalgebra, pero el crate termoquímico no declara esa dependencia. Las afirmaciones documentales deben cotejarse con código y evidencia numérica.

## Cambios implementados

| Hallazgo | Impacto | Corrección y comprobación |
|---|---|---|
| GEM sustituía T y P no válidas mediante max | La solicitud cambiaba sin avisar; NaN podía convertirse en un valor positivo | Rechazo de temperaturas/presiones no finitas o no positivas. Regresión desde la API de aplicación |
| Reacciones no limitaba barridos ni rechazaba todas las entradas no finitas | Riesgo de consumo de memoria o resultados sin significado | Validación previa, máximo de 100001 puntos de malla; los puntos adicionales de transición no cuentan en ese límite |
| Interpolación aceptaba tablas térmicas inválidas y el solver tenía valores alternativos silenciosos | Datos dañados podían producir una respuesta aparente | Validación de finitud y orden estricto; propagación del error. Regresión desde el cálculo de pérdidas de calor |
| Navegación inglesa y versión fija 10.0.4.3 | Inconsistencia del idioma y versión mostrada | Catálogo inicial con 92 cadenas, títulos y navegación en español; versión desde Cargo |
| Documentación ignorada y ausencia de guía raíz | Revisión y mantenimiento no reproducibles | Referencias recuperadas, inventario, especificación y README |

## Validación

La línea base tiene 123 pruebas que pasan al excluir explícitamente una prueba de transporte TCP bloqueada por el sandbox. La ejecución completa original falla al intentar abrir un puerto local: `Operation not permitted`. No se modifica ni se ignora esa prueba en el código.

Resultado final: 127 pruebas aprobadas y una prueba de transporte TCP excluida explícitamente por la restricción documentada.

Se añadieron cuatro pruebas de contrato en la capa de aplicación: entradas/barridos de reacciones; dominio T/P de equilibrio; consistencia de unidades y G = H − TS para CO(g); tablas de propiedades inválidas. La prueba de identidad es una comprobación de consistencia, no una validación independiente de los datos de CO. Se usa el catálogo abierto: CO2(g) no figura en ese catálogo, por lo que no se presupone su disponibilidad.

Las pruebas de UI existentes renderizan vistas sin una sesión gráfica interactiva. Sigue pendiente inspeccionar visualmente desbordamientos, teclados y gráficos en una sesión de escritorio. Los instaladores existentes no se han reconstruido y no contienen estos cambios.

## Ejecución del plan

- [x] Instalar la skill local y acordar el límite público de validación.
- [x] Crear especificación y seguimiento local.
- [x] Recuperar documentación histórica con trazabilidad.
- [x] Inventariar los veinte módulos y obtener una línea base de pruebas.
- [x] Corregir los tres fallos de validación descritos y añadir regresiones.
- [x] Iniciar el catálogo de español y traducir navegación/títulos.
- [ ] Contrastar matemática y química de cada módulo con referencias primarias, datos independientes y tolerancias justificadas.
- [ ] Revisar conservación/KKT, actividades, extremos de fase, tablas Pitzer y extrapolación Criss–Cobble.
- [ ] Revisar saturación/propiedades de agua y coherencia entre los motores termoquímicos y add-ins.
- [ ] Traducir formularios, menús internos, errores anidados, gráficos, exportaciones y ayuda de los veinte módulos, además de CLI/web.
- [ ] Revisar visualmente pantallas, estados vacíos, accesibilidad por teclado y respuesta durante cálculos largos.
- [ ] Añadir referencias por dato/modelo, límites e incertidumbre de resultados y advertencias de convergencia.
- [ ] Revisar persistencia, compatibilidad y recuperación de proyectos.
- [ ] Completar manual de usuario y reconstruir/verificar paquetes de distribución.

No se considera completado el plan general. Los cambios locales previos de diagramas, Eh-pH, estimación y distribución permanecen fuera de las modificaciones de esta entrega.

## Continuación: controles científicos y uso

Se amplió el catálogo de español con 119 entradas utilizadas en 327 controles de 23 archivos. Se añadieron manual de usuario y arquitectura. La interfaz de agua usa una entrada comprobada, conserva el resultado anterior si hay error y explica el alcance aproximado del modelo. La saturación se compara con tres puntos de NISTIR 5078, tabla 1: https://www.nist.gov/document/nistir5078-tab1pdf (tolerancia relativa 0,05 %).

Equilibrio ahora exige estabilidad de actividad y cierre de materia antes de marcar convergencia; se añadieron casos sintéticos analíticos ideales y no ideales. Benson rechaza grupos sin parámetros en vez de asignar contribuciones genéricas. El escritorio guarda mediante archivo temporal y reemplazo final. Se añadieron regresiones para estos comportamientos.

El catálogo LCA incorporado contiene factores sintéticos y calibrados para ejemplos; se añade una advertencia de procedencia a la vista y a los libros XLSX. Estos factores requieren sustitución por datos trazables antes de usar los resultados ambientales.

La CI fue migrada de CMake a Cargo. Se preparó un empaquetador portable con manifiesto y hashes; la compilación y ejecución de esos paquetes todavía requieren confirmación. No se considera completada la auditoría científica de todos los módulos ni la traducción total.


## Verificación de integridad y compilación

La suite actual tiene 133 pruebas aprobadas al excluir explícitamente el transporte TCP bloqueado en este entorno. Se verificaron 211 claves de traducción utilizadas sin claves faltantes y los hashes de los 156 documentos históricos. `scripts/audit-repository.py` permite reproducir esa comprobación y se ejecuta en CI.

Se añadió `packaging/verify-package.py` para comprobar los componentes obligatorios, el manifiesto y los SHA-256 de paquetes portables; CI lo ejecuta antes de publicar artefactos. Se corrigió el resumen de equilibrio que aún decía “Convergido” para el total de puntos calculados.

El intento de arranque gráfico mediante Xvfb falló en el entorno. La revisión visual interactiva queda pendiente. Las comprobaciones de claves no acreditan que todos los textos estén traducidos; tampoco sustituyen el contraste científico independiente de los veinte módulos.


Compilación Linux: `cargo build --release --offline --workspace` completada. Se recompiló escritorio tras corregir el resumen de convergencia. Se verificaron ayuda, balanceo, cálculo de CO(g) y salida 2 para un argumento numérico mal escrito en la CLI. El paquete portable Linux de `dist/revision-20261117` pasó la verificación de manifiesto, componentes y hashes. El manifiesto identifica expresamente el árbol con cambios sin commit.

Compilación Windows: `cargo build --release --offline --target x86_64-pc-windows-gnu -p soryntel-desktop` completada. El paquete ZIP de `dist/revision-20261117` pasó la verificación de componentes, manifiesto y hashes. Esto acredita compilación cruzada e integridad del paquete; falta ejecución en una sesión nativa de Windows. Los instaladores NSIS/.run/Arch anteriores no fueron regenerados en esta entrega.


## Revisión del plan y español por módulo

Se revisó el plan contra la implementación y se añadió `MATRIZ_CIERRE.md` con omisiones y criterios pendientes. Las referencias a los fallos ya corregidos del plan dejaron de describirse como si continuaran presentes. Se incorporó una auditoría reproducible de literales de presentación (`scripts/audit-ui-texts.py`), que incluye textos españoles y técnicos y necesita revisión humana; no equivale a una lista de errores de traducción.

Se tradujeron 203 controles adicionales y 191 entradas del catálogo, además de etiquetas de fases, geometrías y modelos. Se revisaron la ayuda de reacciones y advertencias de agua en español. El catálogo tiene ahora 402 claves utilizadas. Se preservan símbolos químicos, identificadores y formatos de proyecto. La traducción de todos los textos dinámicos y exportaciones sigue pendiente; los documentos históricos conservan su contenido y hashes.

Se corrigió otra omisión de validación: el barrido de equilibrio ahora comprueba ejes finitos, límites logarítmicos positivos, variables independientes distintas y una malla completa de hasta 100001 puntos antes de reservar memoria. Una regresión en la API pública comprueba tamaños extremos y entradas mal definidas. Este límite controla memoria; no garantiza tiempos de respuesta para todos los modelos.

`EVIDENCIA_PRUEBAS.md` recoge las pruebas declaradas por componente para facilitar el contraste entre requisitos y evidencia. Las comprobaciones de esta entrega no sustituyen los datos independientes que aún faltan para varios módulos. Los paquetes de la entrega anterior no incluyen estas modificaciones posteriores.

Validación de esta entrega: 134 pruebas aprobadas, 0 fallidas y 1 excluida por la restricción TCP documentada. Se compilaron los crates modificados mediante la suite del workspace; auditoría documental y claves sin fallos.

Se añadió el glosario español en la raíz para unificar términos de la interfaz y documentación. La inspección de dependencias del ejecutable Linux existente resolvió sus bibliotecas en este equipo; la inspección del PE de Windows mostró importaciones del sistema sin DLL de MinGW externa aparente. Estos resultados no sustituyen la prueba en sistemas limpios ni confirman la portabilidad entre versiones de glibc.

Compilación optimizada de esta entrega: `cargo build --release --offline -p soryntel-desktop -p soryntel-app` completada. Paquete Linux actualizado en `dist/revision-es-20261117`, verificado mediante manifiesto, cuatro componentes obligatorios y SHA-256. El paquete Windows anterior requiere recompilación para incorporar los cambios de esta entrega.

## Entrega centrada en el módulo 1 y arquitectura global

La evidencia y límites del módulo 1 se detallan en `modulo-01/REVISION.md`. Se corrige S° de CO(g) contra NIST, se impide calcular formación con referencias ausentes, se amplía español y se adapta la vista compacta mediante pestañas y controles desplazables. Las exportaciones usan el directorio temporal del sistema. La interfaz de reacciones consume el puerto del catálogo y CLI/web utiliza la capa de aplicación.

`AUDITORIA_HEXAGONAL.md` revisa los ocho crates, describe sus dependencias y prioriza infracciones pendientes. El chequeo `scripts/audit-architecture.py` se incorpora a CI. La prueba de renderizado cubre 360×640, 480×360 y 1340×820; no sustituye inspección visual interactiva.

## rust-i18n y acciones de archivo del módulo 1

Se preparó la migración al catálogo JSON de rust-i18n y el uso de t! en reacciones. Abrir/guardar/exportar se conectan a rfd; copiar imagen se implementa con resvg y arboard, diferenciándola de copiar datos. Se corrige el tamaño del lienzo y el reinicio continuo de ejes. Véase `modulo-01/ACCIONES_Y_TRADUCCION.md` para alcance y estado.

La compilación de esta entrega está bloqueada por resolución DNS de crates.io y dependencias nuevas ausentes de la caché. Cargo.lock está pendiente de actualización. No se han reemplazado los binarios compilados en las entregas anteriores ni se consideran válidas sus pruebas como verificación de las nuevas dependencias.


## Compilación tras disponer de dependencias nativas

Las dependencias rust-i18n/rfd/resvg ya están disponibles y Cargo.lock permite resolverlas. La compilación optimizada Linux de escritorio terminó correctamente. Se eliminaron dos constantes sin uso. La prueba de tamaño detectó que los ejes consumían parte de la altura del gráfico: se elevó el mínimo exterior a 320 px. Las cuatro pruebas de acciones y vista pasan, incluida PNG y tamaño en ventanas compactas. La recompilación final incorpora esta corrección; la operación interactiva de diálogos y portapapeles sigue requiriendo sesión gráfica.
