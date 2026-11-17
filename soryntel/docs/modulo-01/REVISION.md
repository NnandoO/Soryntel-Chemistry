# Revisión del módulo 1: ecuaciones de reacción

## Cambios y evidencia

| Área | Hallazgo y acción | Comprobación |
|---|---|---|
| Termodinámica de reacción | Revisadas sumas estequiométricas de H/S/G, conversión J/kJ/cal/kcal y log K | Caso sintético H2(g) = 2H(g) con solución analítica a 500 K en kJ y kcal |
| Integración térmica | Revisada integración de Cp y saltos por intervalos | Caso sintético a 200, 298,15 y 600 K con transición a 500 K |
| Formación (Delta) | Faltaban errores al omitir referencias elementales o H2/H+ para iones | Ahora se exige disponibilidad; prueba de referencia Xe ausente |
| Datos CO(g) | S de catálogo abierto era 197,543376 J/(mol·K) | Corregida a 197,660; contraste con fuente NIST y tolerancia publicada |
| Idioma | Persistían casillas, etiquetas pintadas, títulos de sección y ayudas en inglés | Traducidos controles del gráfico, formato, tabla de especies, herramientas y magnitudes |
| Vista compacta | Apilar tablas y gráfico podía consumir el alto disponible | Controles con alto limitado y desplazamiento; pestañas Datos/Gráfico para ventanas menores de 880 px |
| Acoplamiento | La vista recibía ThermoDatabase y CLI/web llamaba directamente al dominio | Vista usa SpeciesCatalogPort; CLI/web invoca SoryntelApplication |

## Contratos de cálculo

Las propiedades de especie se expresan por mol. Las magnitudes de reacción corresponden a los coeficientes escritos; multiplicar toda la reacción modifica ΔH, ΔS, ΔG y log K. Cambiar unidades de presentación no cambia la constante de equilibrio. Las temperaturas absolutas deben ser positivas; el paso es una diferencia térmica. La respuesta conserva log K cuando K excede la representación flotante.

El modo Delta requiere todos los elementos de referencia y, para especies cargadas, las especies de la convención H2/H+. La ausencia produce un error; no se supone contribución cero. El cálculo de una reacción requiere balance elemental y de carga antes de evaluar propiedades.

Los volúmenes gaseosos tabulados se basan en el volumen molar estándar incorporado; no son volúmenes a la temperatura de cada fila. Criss–Cobble sigue siendo una aproximación cuya parametrización requiere contraste adicional. El cambio de referencia CO no convierte su ajuste Kelley en un ajuste Shomate completo.

## Fuente independiente

[NIST Chemistry WebBook, monóxido de carbono](https://webbook.nist.gov/cgi/cbook.cgi?ID=C630080&Mask=1): Hf° = −110,53 ± 0,17 kJ/mol y S°(298,15 K, 1 bar) = 197,660 ± 0,004 J/(mol·K). Se contrastan esos valores de referencia; no se usa su tolerancia como precisión global del modelo.

## Interfaz y límites de verificación

Se renderizan ambas vistas compactas y la vista amplia a 360×640, 480×360 y 1340×820 mediante egui sin pantalla. Se mantiene la prueba de reapertura .rea8 y generación SVG. Estas pruebas confirman ejecución sin errores y disponibilidad de datos en ambos modos; sigue siendo necesaria la inspección interactiva del recorte, legibilidad y uso del teclado.

La revisión no certifica todos los datos de la base ni equivalencia completa con HSC. Quedan por contrastar la convención electroquímica para reacciones invertidas, Criss–Cobble y las propiedades de todas las especies y transiciones importadas. Las exportaciones y la vista web requieren una pasada terminológica final.

Las rutas predeterminadas de exportación del módulo ahora utilizan el directorio temporal del sistema, evitando depender de `/tmp` en Windows. Si falla el guardado del SVG al copiarlo, el error se comunica en vez de anunciar un guardado que no ocurrió. Se añadieron reglas CSS para pantallas estrechas de la web local; su funcionamiento visual en un navegador sigue pendiente de comprobación interactiva.

Resultado del workspace después de los cambios: 139 pruebas aprobadas, 0 fallidas y una prueba TCP excluida por el bloqueo de puertos del entorno. Las auditorías de dependencias, claves y hashes, y `git diff --check`, pasan. No se reconstruyeron los paquetes de distribución en esta entrega.
