# CMA Chemistry

## Compilar y abrir

Desde la raíz del repositorio:

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --target cma_chemistry -j 3
./build/cma_chemistry
```

Requiere CMake >= 3.20, compilador C++17, Eigen >= 3.4, SQLite3 y los paquetes de
desarrollo OpenGL, X11/Wayland. CMake descarga GLFW, ImGui, ImPlot e IPhreeqc al
configurar un build nuevo. IPhreeqc está fijado a una revisión y SHA256; se puede
deshabilitar con `-DCMA_WITH_IPHREEQC=OFF` (sin equilibrio avanzado/Pitzer).
La exportación molecular requiere Python, NumPy y Matplotlib.

## Módulos nuevos

- **Módulos → 20. Soluciones Acuosas**: pestañas USGS/Pitzer, MINTEQ nativo y hoja heredada.
- **Módulos → 25. Procesamiento mineral**: clasificación, flotación de primer orden,
  clasificación seguida de flotación de finos, energía de Bond y ajuste cinético desde CSV.
- **Módulos → 26. Auditoría del catálogo**: señales de revisión y exportación JSON.

Los paneles nuevos invalidan resultados al editar entradas. Agua/vapor distingue
T/P de estados saturados P/x y muestra como no disponibles las propiedades inválidas.

## Estructura

- `src/domain`, `src/nucleo`, `src/motores`: contratos y cálculos científicos.
- `src/application`: casos de uso y puertos de entrada/salida.
- `src/adapters`, `src/base_datos`: implementaciones de plataforma y persistencia.
- `src/interfaz/application`: ciclo de vida, menús, layout, proyectos y simulación.
- `src/interfaz/canvas`: dibujo de equipos, conexiones, navegación y anotaciones.
- `src/interfaz/panels`: vistas por herramienta científica.
- `tests`: pruebas ejecutables y reglas de dependencias; `tests/legacy` conserva experimentos.
- `tools/data`: generación/importación; `tools/rendering`: recursos Python de ejecución.
- `tools/legacy`: parches históricos, fuera del flujo de mantenimiento actual.

## Comprobar concentración y dependencias

```bash
python3 tools/check_structure.py
python3 tests/check_architecture.py .
cmake --build build -j 3
ctest --test-dir build --output-on-failure
```

El informe de tamaño señala archivos de más de 1200 líneas; no certifica la ausencia
de monolitos. Las reglas de dependencias comprueban que dominio/casos de uso no
incluyan UI, SQLite ni adaptadores concretos. La revisión debe comprobar además
cohesión, responsabilidades y posibilidad de probar cada módulo independientemente.
Dividir un archivo no elimina por sí mismo el acoplamiento de una clase.

La UI mantiene una clase coordinadora con numerosos paneles y algunos accesos
heredados a singletons. Los headers agregados de paneles y helpers de reacción
siguen siendo candidatos a separación de contratos. Los datos termodinámicos
incorporados y los modelos aproximados no equivalen a validación experimental.

MINTEQ se importa en su formato nativo mediante `cma_catalog_audit data`, separado
de las especies puras; el CSV convertido antiguo no se carga automáticamente.
Los motores nuevos están integrados en paneles. Pitzer usa el motor IPhreeqc y su
base parametrizada; HMW no está implementado como modelo independiente. Los parámetros
de procesamiento mineral deben calibrarse con datos apropiados: las muestras GeoMet
incluidas no son series cinéticas ni validan una planta concreta.
Consulte [el plan de ampliación](docs/HSC_PARITY_ROADMAP.md) y
[la cobertura científica](docs/SCIENTIFIC_VALIDATION.md).
Una compilación exitosa no significa equivalencia científica con HSC.
