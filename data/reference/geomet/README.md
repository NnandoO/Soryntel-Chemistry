# GeoMet v4 — datos experimentales independientes

Autores: Júlio Hoffimann y coautores (IMPA, UFMG, Vale S.A.).
Fuente y versión fija: https://doi.org/10.5281/zenodo.7051975 (v4).
Licencia: CC BY 4.0, https://creativecommons.org/licenses/by/4.0/.
Descarga: 2026-10-12. Archivos originales, sin modificación.

- `comminution.csv`: ensayos DWT/BWI; campos F80, P80, M, A y análisis químicos.
- `flotation.csv`: recuperación de cobre en ensayos de ciclo cerrado LCT.

MD5 publicados: `1a33df8ba77f5d49c281ba3fff16b20b` (comminution),
`f2e90da6bfa81de1261177ee85a91570` (flotation).

No hay series tiempo-recuperación ni una alimentación por mineral/tamaño con parámetros
cinéticos. No se ajusta k ni se declara validado el circuito con estos archivos.
Las columnas sin una unidad/definición documentada no se reinterpretan automáticamente.
`tests/reference_data.py` verifica integridad y rangos básicos, no equivalencia industrial.
