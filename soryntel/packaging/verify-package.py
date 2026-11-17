#!/usr/bin/env python3
"""Comprueba hashes, manifiesto y componentes del paquete sin instalarlo."""
import argparse
import hashlib
import json
import tarfile
import zipfile
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('package', type=Path)
args = parser.parse_args()
p = args.package
expected = p.with_name(p.name + '.sha256').read_text().split()[0]
assert hashlib.sha256(p.read_bytes()).hexdigest() == expected, 'SHA-256 del paquete incorrecto'
if p.suffix == '.zip':
    with zipfile.ZipFile(p) as archive:
        assert archive.testzip() is None
        files = {name: archive.read(name) for name in archive.namelist() if not name.endswith('/')}
else:
    with tarfile.open(p, 'r:gz') as archive:
        files = {m.name: archive.extractfile(m).read() for m in archive.getmembers() if m.isfile()}
manifests = [name for name in files if name.endswith('/manifest.json')]
assert len(manifests) == 1, 'Falta un manifiesto único'
manifest_path = manifests[0]
base = manifest_path.rsplit('/', 1)[0] + '/'
m = json.loads(files[manifest_path])
required = {'README.md', 'MANUAL.md', 'soryntel-icon.png', 'soryntel-desktop' + ('.exe' if m['platform'] == 'win64' else '')}
assert required <= m['files'].keys(), 'Faltan componentes obligatorios'
for name, digest in m['files'].items():
    assert hashlib.sha256(files[base + name]).hexdigest() == digest, name
assert set(files) == {base + name for name in m['files']} | {manifest_path}, 'Archivos sin registrar'
print(f'Verificado: {p}; {len(m["files"])} componentes; commit {m["commit"]}; cambios locales: {m["dirty"]}')
