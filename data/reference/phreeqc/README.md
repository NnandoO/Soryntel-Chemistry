# Bases de referencia USGS PHREEQC

Origen: https://github.com/phreeqc-dev/iphreeqc
Revisión fija: `f107fcb2f2e1e3167ce4c1328e477b54dee3d21b`.
Archivos `database/phreeqc.dat` y `database/pitzer.dat`, copiados sin modificación.
Descarga: 2026-10-12. Derechos y limitaciones: `NOTICE.TXT`.
Los componentes CVODE/SUNDIALS del motor mantienen su licencia BSD en
`third_party/phreeqc/LICENSE-SUNDIALS.note`.

Estas bases contienen reacciones, leyes térmicas y, en Pitzer, parámetros de interacción.
Se usan únicamente en el adaptador IPhreeqc; no se convierten a especies puras Hf/Sf.
La interfaz limita el alcance a Na/Cl/Ca/Mg/K/C(4)/S(6), 0–100 °C y los casos de fase
calcita/CO2. Eso no garantiza que todas las interacciones estén parametrizadas en todo
ese intervalo: revise advertencias y las referencias internas de cada registro.

`tests/advanced_aqueous.cpp` reproduce numéricamente el caso 3A de USGS:
https://water.usgs.gov/water-resources/software/PHREEQC/documentation/phreeqc3-html/phreeqc3-65.htm

No se afirma calibración experimental de la planta del usuario ni equivalencia HSC.
