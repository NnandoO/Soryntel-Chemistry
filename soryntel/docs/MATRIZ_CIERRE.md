# Matriz de cierre de la revisión

| Área | Evidencia disponible | Falta para cerrar |
|---|---|---|
| Especificación | Plan y especificación local con límite de validación acordado | Mantener contratos detallados por módulo |
| Documentación | 156 documentos recuperados y hashes verificados | Cotejo científico y actualización de cada especificación |
| Entradas comunes | Regresiones para T/P, barridos e interpolaciones | Revisar todos los demás puntos de entrada |
| Equilibrio | Casos analíticos ideal/no ideal y conservación; convergencia de actividad | KKT completo, carga, múltiples fases y datos externos |
| Agua | Tres presiones NIST y comprobación de entrada | Reemplazar o validar modelos aproximados de propiedades |
| Benson | Rechazo de grupos ausentes | Referencias y tolerancias por grupo/fase |
| LCA | Aviso de factores sintéticos en UI/XLSX | Factores independientes con procedencia verificable |
| Español | Catálogo de navegación, controles y etiquetas de modelos ampliado | Ayudas largas, mensajes dinámicos, exportaciones y revisión terminológica integral |
| Interfaz | Pruebas egui existentes | Inspección interactiva, teclado, ventanas pequeñas y operaciones largas |
| Persistencia | Guardado temporal/reemplazo y regresión | Migraciones, archivos truncados y recuperación de proyectos por módulo |
| CLI/web | Ayuda, balanceo, cálculo y entradas numéricas; mejoras de presentación web | HTTP real y verificación completa de errores/red |
| Compilación | Linux y Windows de la entrega anterior | Recompilar los cambios de idioma y barridos de esta entrega |
| Paquetes | Manifiestos, componentes y hashes de la entrega anterior | Ejecución en sistemas limpios y nuevos paquetes tras cierre |
| Científica restante | Pruebas históricas de los veinte módulos | Fuentes independientes, unidades, tolerancias y límites por módulo |

La integridad documental y las claves se comprueban con `scripts/audit-repository.py`. Los literales pendientes se inventarían con `scripts/audit-ui-texts.py`. El estado no se declara completo mientras haya requisitos de la última columna sin evidencia.
