# Cobertura científica y límites

## Evidencia numérica automatizada

| Componente | Referencia o comprobación | Lo que NO demuestra |
| --- | --- | --- |
| Agua/vapor IF97 | IAPWS tablas 5, 15, 33, 42 y saturación; tolerancia 3e-6 escalada | Hielo, transporte bifásico o precisión experimental universal |
| Gibbs | Casos del conjunto de referencia y barrido térmico; reinicio de warm start fallido | Soluciones reales no ideales ni equivalencia de bases comerciales |
| MINTEQ nativo | Formato/estequiometría, importación atómica, equilibrio analítico de dímero y balance de carbonatos | Electronegatividad/electroneutralidad resuelta, precipitación o extrapolación térmica |
| USGS IPhreeqc | Calcita (3A: pH ±0.003, Ca ±5e-6, SI ±1e-7); yeso analítico (0.015 mol/kg, SI ±1e-6); halita Pitzer (6.16 mol/kg, SI ±1e-6); coeficientes de actividad iónica gamma_i; multimetal Cu/Zn/Fe | Que cada interacción/temperatura del panel esté validada experimentalmente |
| Clasificación/flotación/molienda | Partición, conservación de masa/agua, recuperación de primer orden, molienda Bond y circuitos cerrados con reciclo (Wegstein) | Calibración de un banco continuo CSTR o de una planta completa |
| Diagramas estabilidad Kellogg | Predominancia por minimización G_norm para Cu-O-S, Fe-O-S, Ni-O-S, Pb-O-S, Zn-O-S; límites 300..2500 K | Soluciones sólidas no ideales ni cinética de tostación/reducción |
| Estimación Benson vs Joback | Benson 2do orden (Cohen & Benson 1993) en gas ideal 298.15 K (±4 kJ/mol Hf); Joback-Reid como alternativa declarada | Propiedades en fase líquida o extrapolación térmica certificada |
| Ajuste cinético | Recuperación de k de una serie analítica sintética; rechazo de R >= R infinito y tiempos sin información | Ajuste independiente de R infinito, incertidumbre estadística o validación industrial |
| Catálogo heredado | Auditoría de masa/formula/Cp con soporte de aductos/iones/polímeros, fuentes y rangos; metadatos exportados en SQLite | Corrección automática de los 16 566 registros o certificación de sus valores |

La tolerancia IF97 usa `max(1, abs(referencia))`; es absoluta cuando la magnitud es menor que uno.
Los tests de integridad de archivos comprueban hashes y rangos, no el ajuste de un modelo a observaciones.

## Datos públicos descargados

- [USGS IPhreeqc](https://github.com/phreeqc-dev/iphreeqc), revisión fija y derechos USGS/SUNDIALS conservados. Bases originales bajo `data/reference/phreeqc`.
- [GeoMet v4](https://doi.org/10.5281/zenodo.7051975), Hoffimann y coautores, CC BY 4.0. CSV originales bajo `data/reference/geomet`: 60 ensayos de molienda, 53 registros de flotación con 52 LCT y un faltante.

GeoMet no incluye una serie tiempo-recuperación para ajustar k. Las variables cuya definición/unidad no está documentada no se reinterpretan ni rellenan.

## Uso de la calibración

En procesamiento mineral, importe CSV numérico sin encabezados con columnas
`tiempo_min,recuperacion_fraccional`, >=3 muestras. Introduzca R infinito medido
por separado; el ajuste usa `-ln(1-R/Rinf)=k*t`, recta por el origen.
El RMSE se informa en el espacio de recuperaciones. Reserve muestras independientes
para comprobar predicción; no copie parámetros entre minerales sin evidencia.

## Reproducir

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DCMA_WARNINGS_AS_ERRORS=ON
cmake --build build -j 3
ctest --test-dir build --output-on-failure
cmake -S . -B build/asan -DCMA_BUILD_GUI=OFF -DCMA_WITH_IPHREEQC=OFF -DCMA_SANITIZERS=ON -DCMAKE_BUILD_TYPE=Debug -DCMA_WARNINGS_AS_ERRORS=ON
cmake --build build/asan -j 2
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 UBSAN_OPTIONS=halt_on_error=1 ctest --test-dir build/asan --output-on-failure
```

La segunda configuración cubre el núcleo propio, no IPhreeqc ni el escritorio.
La CI de `.github/workflows/verify.yml` no es una ejecución remota hasta publicarla.
