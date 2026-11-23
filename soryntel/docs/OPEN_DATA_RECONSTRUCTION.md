# Base termodinámica independiente de datos abiertos

## Objetivo y límites

Soryntel no debe reconstruir ni redistribuir `MainDB6.HSC`, `OWNDB6.HSC` o
`MainDB6.REF` de HSC. Esos archivos son bases propietarias. La alternativa
legal es construir un catálogo independiente: cada registro se obtiene de una
fuente cuya licencia permite la redistribución, se transforma al modelo interno
y conserva su procedencia, licencia, versión y hash.

La presencia de una referencia bibliográfica no convierte automáticamente sus
tablas en datos redistribuibles. NIST WebBook se usará como fuente de
verificación puntual, no como descarga masiva o réplica del sitio.

## Fuentes admitidas

| Dominio | Fuente | Uso previsto | Estado |
|---|---|---|---|
| Agua/vapor | IAPWS releases (IAPWS-95/IAPWS-IF97) | ecuación de estado y propiedades de agua | revisión de términos |
| Termoquímica | NASA Glenn CEA/Thermobuild | polinomios NASA de especies | conservar aviso/licencia |
| Termoquímica experimental | NIST ThermoML Archive | solo registros redistribuibles | revisión registro por registro |
| Constantes y elementos | CODATA/IUPAC | masas atómicas y constantes | versión y atribución |
| Minerales | datasets CC-BY/CC0/O(DbL) | composición y propiedades | atribución obligatoria |

No se incorporarán datos descargados automáticamente de NIST WebBook ni datos
copiados de HSC. Los registros ambiguos quedan fuera hasta resolver la licencia.

## Formato canónico de procedencia

Cada especie tendrá un registro `SpeciesDataset` y metadatos:

```json
{
  "formula": "CO(g)",
  "source_id": "fuente-identificador",
  "source_url": "https://example.org/dataset",
  "source_version": "versión o fecha UTC",
  "license": "licencia explícita",
  "retrieved_sha256": "hash del archivo de entrada",
  "transformation": "normalización SI y ajuste documentado",
  "quality": "reviewed|derived|unverified"
}
```

Un registro no se publica si `license` no está resuelta. La aplicación debe
mostrar la procedencia en la ficha de especie y en las exportaciones CSV/JSON.

## Fases de implementación

1. Separar y etiquetar los datos integrados actuales como `OpenDefault`, sin
   llamarlos “base HSC”.
2. Añadir un importador CSV/JSON reproducible con validación de unidades,
   rangos, licencia y hash; sin scraping en tiempo de compilación.
3. Publicar un catálogo abierto inicial con licencia resuelta, procedencia y
   pruebas de `H/S/Cp/G` y balance elemental.
4. Ampliar por lotes (elementos, óxidos, gases, minerales y electrolitos) y
   comparar contra fuentes independientes, nunca contra una copia de HSC.

## Fuentes normativas

- NIST ThermoML: https://www.nist.gov/mml/acmd/trc/thermoml
- IAPWS releases: https://www.iapws.org/release.html
- NASA CEA: https://www.nasa.gov/glenn/research/chemical-equilibrium-with-applications/

