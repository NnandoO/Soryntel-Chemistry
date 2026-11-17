# Acciones, gráfico y rust-i18n

## Implementación preparada

- Se usa `rust-i18n` 3.x con el catálogo `crates/soryntel-desktop/locales/es.json`, incorporado mediante `i18n!` y español como idioma inicial y de respaldo.
- El módulo 1 usa `t!` para las claves migradas. Las otras vistas mantienen un puente para sus etiquetas estáticas; dicho puente resuelve el mismo catálogo mediante rust-i18n. No se afirma que todos los literales del proyecto hayan sido extraídos.
- Abrir y guardar libros .rea8 utilizan `rfd::FileDialog`. Los botones de menú y barra de herramientas invocan los mismos métodos. Cancelar no modifica los datos ni escribe archivos.
- Exportar XLSX, CSV, SVG y PNG permite elegir destino en un diálogo nativo. Se añade la extensión cuando falta; se conserva la extensión explícitamente elegida por el usuario.
- Copiar imagen convierte el SVG en una imagen RGBA mediante resvg y usa el portapapeles de imágenes de arboard, conservando su instancia. Los errores se muestran en la barra de estado. La copia de datos numéricos permanece como acción independiente.
- La exportación CSV protege comas y comillas en las referencias.
- La barra de herramientas pasa encima del gráfico. El lienzo ocupa el ancho disponible y tiene alto exterior mínimo de 320 px, reservando al menos 240 px para el lienzo tras los ejes; la ventana independiente es redimensionable. El restablecimiento cambia el identificador del gráfico y ya no fuerza un reinicio continuo que impida ajustar los ejes.

## Dependencias y comprobaciones

La documentación consultada es la oficial de [rust-i18n](https://docs.rs/rust-i18n/3.1.5/rust_i18n/), [rfd](https://docs.rs/rfd/0.15.4/rfd/struct.FileDialog.html) y [resvg](https://docs.rs/resvg/0.45.1/resvg/fn.render.html).

Se intentó descargar las dependencias con Cargo; el entorno devuelve `Could not resolve host: index.crates.io`. El modo offline tampoco puede resolver resvg, que no está en la caché. Ese bloqueo correspondía al intento anterior. En la ejecución actual las dependencias ya están disponibles y `cargo build --release -p soryntel-desktop` terminó correctamente; el ejecutable Linux actualizado está en `target/release/soryntel-desktop`.

Comprobaciones realizadas: formato/sintaxis mediante rustfmt, integridad documental y 409 claves usadas presentes; reglas de arquitectura y prueba aislada del conversor CSV. Las cuatro pruebas del módulo de acciones y vista de reacciones pasan: CSV, rasterizado SVG/PNG, reapertura .rea8 y dimensiones del gráfico. Hace falta probar los diálogos y pegar la imagen en una aplicación real de escritorio.

En un entorno con acceso a crates.io:

```sh
cargo fetch
cargo test --workspace
cargo build --release -p soryntel-desktop
```

El comando opcional de extracción se instala con `cargo install rust-i18n` y se utiliza desde el crate de escritorio mediante `cargo i18n`. La instalación de esta herramienta CLI opcional no forma parte de la compilación de la aplicación. Las auditorías de claves ya leen el JSON.

La acción de informe conserva la vista previa y copia del informe existente; no se ha implementado impresión nativa en esta entrega. La copia vectorial conserva SVG como texto y está separada de la copia de imagen rasterizada.
