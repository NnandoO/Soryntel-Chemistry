#!/usr/bin/env python3
"""Comprueba integridad documental y claves de presentación usadas por el escritorio."""
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[1]
src = root / 'crates/soryntel-desktop/src'
keys = set(json.loads((src.parent / 'locales/es.json').read_text())) - {'_version'}
used = set()
for source in src.rglob('*.rs'):
    if source.name != 'i18n.rs':
        used.update(re.findall(r'(?:i18n::text|(?<![A-Za-z_])t!)\("([^"]+)"', source.read_text()))
assert not used - keys, f'Claves de traducción ausentes: {used - keys}'
reference = root / 'docs/referencia'
manifest = json.loads((reference / 'manifest.json').read_text())
for item in manifest:
    copy = reference / item['copia']
    assert hashlib.sha256(copy.read_bytes()).hexdigest() == item['sha256'], copy
for name in ['MANUAL_USUARIO.md', 'ARQUITECTURA.md', 'REVISION.md', 'INVENTARIO.md']:
    assert (root / 'docs' / name).is_file(), name
print(f'{len(used)} claves usadas presentes; {len(manifest)} documentos históricos íntegros.')
print('Esta comprobación no acredita traducción completa ni validación científica de todos los módulos.')
