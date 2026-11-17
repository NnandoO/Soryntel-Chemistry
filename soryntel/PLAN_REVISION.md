# Plan de revisión de Soryntel Chemistry

## Objetivo

Revisar esta reconstrucción de HSC Chemistry para comprobar su comportamiento científico y funcional, mejorar la interfaz y documentación en español, y dejar resultados reproducibles que permitan distinguir cálculos verificados de funciones aproximadas o aún no validadas. Este plan no afirma equivalencia total con HSC: cada módulo se contrastará con la especificación recuperada, fuentes científicas y casos de referencia disponibles.

## Alcance observado

El producto es un workspace Rust con ocho crates: núcleo, solver, termodinámica, base de datos, aplicación, escritorio, aplicación web/add-in y `chem_addin_core`. La interfaz de escritorio usa egui. La lógica cubre balanceo, datos de especies, termodinámica, equilibrio GEM, diagramas, agua, pérdidas de calor, balances, exergía, estimaciones, conversión de especies y add-ins. La raíz Git es `HSC-REA/`; el workspace Rust está en `soryntel/`; la documentación por módulos que aparece en el historial fue retirada en el commit `77e4589` (“Quitando documentacion”).

## Prioridades

### P0 — Línea base y trazabilidad científica

- Crear un inventario de funciones por crate y módulo, con entradas, salidas, unidades, supuestos, rangos de validez, estado de implementación y procedencia de datos/modelos.
- Recuperar o reescribir dentro del repositorio las especificaciones funcionales, matemáticas, de interfaz y validación. Mantenerlas vinculadas a módulos y casos concretos; no depender de carpetas ignoradas por Git.
- Registrar para cada fórmula o correlación: referencia primaria, ecuación, unidades, condiciones de aplicación, convención de signos y precisión esperada.
- Etiquetar explícitamente los resultados como calculados, estimados, interpolados o no disponibles cuando corresponda; evitar presentar aproximaciones como datos de referencia.
- Documentar el origen y los derechos de uso de datos, nombres, formatos y material relacionado con HSC, además de los límites de la ingeniería inversa.

### P1 — Revisión matemática y química

Revisar primero las partes que afectan muchos módulos y luego los cálculos específicos:

1. **Fundamentos compartidos:** análisis de fórmulas y fases, cargas, masas molares, unidades, conversiones, constantes, precisión numérica y propagación de errores.
2. **Estequiometría y balances:** balanceo de materia/carga, balances de masa y energía, reactivo limitante, base molar, convenciones de signos y consistencia dimensional.
3. **Propiedades termodinámicas:** H, S, G, Cp, exergía y correcciones de presión/fase; revisar referencias, estados estándar, discontinuidades de fase e intervalos de temperatura.
4. **Equilibrio:** formulación GEM, restricciones elementales y de carga, potenciales químicos, actividades/fugacidades, modelos de solución, fases ausentes, restricciones y criterios de convergencia.
5. **Transferencia de calor:** conducción cilíndrica/esférica/plana, convección, radiación superficial, gases/partículas, cavidades y geometría dependiente del espesor.
6. **Módulos específicos:** agua y saturación, diagramas Eh-pH y HSC, estabilidad de fases, estimación Benson, convertidor de especies, aqua, LCA y add-ins.

Casos de contraste prioritarios: conservación de elementos y carga; cierre de balances; límites de fase y de temperatura; estados de referencia; equilibrio conocido con soluciones analíticas o datos publicados; límites de radiación/convección; y ejemplos reproducibles de cada módulo. Las tolerancias se definirán antes de comparar resultados. Todo caso debe indicar fuente, datos de entrada, resultado esperado y tolerancia.

### P1 — Interfaz completamente en español

- Auditar textos visibles en `soryntel-desktop` y `soryntel-app`: títulos, menús, controles, tablas, diálogos, mensajes de error, ayuda, unidades y exportaciones.
- Crear un catálogo único de cadenas con claves estables y español como idioma inicial; evitar texto de interfaz incrustado en lógica de cálculo.
- Unificar vocabulario químico y de ingeniería, símbolos, nombres de variables y formato numérico. Mantener símbolos y fórmulas químicas convencionales.
- Revisar navegación, jerarquía visual, densidad de tablas, estados vacíos, validación junto al campo, mensajes de convergencia y accesibilidad por teclado.
- Mejorar visualización de resultados: unidades junto a valores, cifras significativas, advertencias del rango de validez, residuales y estado de convergencia.
- Permitir cambiar unidades de presentación sin alterar el estado interno SI y mostrar con claridad qué convención se está usando.

### P2 — Calidad de producto y mantenimiento

- Añadir una guía raíz: propósito, módulos, instalación, compilación, ejecución, importación/exportación, formatos de datos, limitaciones y solución de problemas.
- Documentar arquitectura y flujo de datos; separar UI, aplicación, cálculos y persistencia donde actualmente se mezclen.
- Revisar errores y entradas inválidas. El ajuste silencioso de temperatura y presión de equilibrio ya fue sustituido por validación explícita; ampliar la revisión a entradas y rangos de los demás módulos.
- Hacer visibles fallos de interpolación y datos faltantes. Los valores alternativos silenciosos detectados en interpolación térmica ya fueron eliminados; revisar las restantes sustituciones de datos y los límites de las correlaciones.
- Revisar consistencia de esquemas y migraciones de la base, compatibilidad de archivos, guardado/recuperación de proyectos y mensajes de error.
- Aclarar qué artefactos binarios se versionan y automatizar la trazabilidad de versión, plataforma y procedimiento de empaquetado.
- Añadir matriz de módulos, limitaciones conocidas y registro de decisiones científicas/técnicas.

## Entregables y secuencia

1. **Mapa del sistema y línea base:** inventario de módulos, rutas de datos, contratos, idioma actual y estado de cada función.
2. **Especificaciones y protocolo de contraste:** documentos matemáticos/funcionales/UI, fuentes, conjuntos de referencia y tolerancias acordadas por categoría.
3. **Informe de hallazgos:** cada hallazgo tendrá severidad, módulo, evidencia, impacto, referencia y propuesta de corrección.
4. **Correcciones de alta prioridad:** validación de entradas, unidades, errores silenciosos y fallos científicos confirmados, acompañados de casos de regresión con resultados de referencia.
5. **Localización y UX:** catálogo en español, revisión terminológica y mejoras consistentes de formularios, tablas, gráficos y advertencias.
6. **Documentación y publicación:** guía de usuario/desarrollo, limitaciones, procedencia de datos, empaquetado e idiomas.

## Criterios para considerar un módulo revisado

- Contrato de entradas y salidas documentado, con unidades y supuestos.
- Ecuaciones y datos trazables a fuentes identificadas.
- Casos de referencia independientes con tolerancias explícitas.
- Entradas inválidas, datos faltantes y falta de convergencia comunicados sin convertirlos silenciosamente en resultados válidos.
- UI, errores y documentación del módulo disponibles en español.
- Limitaciones y comportamiento no equivalente al HSC original señalados claramente.

## Observaciones de alcance

- El árbol de trabajo ya tenía cambios locales en `diagrams_ui.rs`, `ehph_ui.rs`, `estimate_ui.rs`, `diagrams.rs`, `ehph.rs`, `estimate.rs` y varios paquetes de `dist/`. Deben revisarse como cambios preexistentes antes de integrarlos; este plan no los altera.
- La skill `to-spec` está instalada en `.agents/skills/to-spec` en la raíz Git. Se acordó validar desde la capa pública de aplicación y revisar visualmente la interfaz. El seguimiento se mantiene local, sin publicar issues externos.
- Esta es una planificación basada en inspección inicial del repositorio; no constituye todavía una auditoría completa de la validez de cada fórmula ni una comparación funcional exhaustiva con HSC.


## Cierre verificable y omisiones detectadas

La ejecución se controla con `docs/MATRIZ_CIERRE.md`. Cada tarea sólo se cierra con evidencia; compilar o tener claves en español no acredita una revisión científica ni la traducción completa.

Requisitos que faltaba detallar:

- Revisar ayudas, textos dinámicos, nombres de fases, cabeceras pintadas directamente, exportaciones y mensajes procedentes del dominio, además de botones.
- Probar coma decimal y punto decimal, copiar/pegar tablas, accesibilidad y textos largos en ventanas pequeñas. La compatibilidad numérica actual debe documentarse antes de cambiarla.
- Comprobar reapertura de proyectos guardados, conservación de referencias y rechazo de archivos truncados o versiones incompatibles.
- Verificar dependencias dinámicas y ejecución de paquetes en sistemas limpios; los hashes sólo acreditan integridad.
- Reproducir el transporte TCP en un entorno que permita puertos; ejecutar Windows de forma nativa.
- Distinguir alcance numérico de rango científico de validez, y separar datos de demostración de datos con fuente verificable.
- Mantener evidencia específica para los veinte módulos, incluyendo incertidumbre, electroneutralidad, estados estándar, KKT y límites de correlaciones.
- Revisar la trazabilidad entre código y paquetes después de cada nueva modificación. Los paquetes de una entrega anterior no incorporan cambios posteriores.

El inventario `docs/auditoria-textos.json`, generado por `scripts/audit-ui-texts.py`, recoge literales de presentación aún fuera del catálogo. Requiere revisión humana y no cubre todas las expresiones dinámicas. Las referencias históricas se conservan sin traducir ni alterar sus hashes; el manual vigente y los informes de esta revisión se escriben en español.
