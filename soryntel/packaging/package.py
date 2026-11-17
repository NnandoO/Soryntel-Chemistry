#!/usr/bin/env python3
"""Paquetes portables sin modificar los instaladores existentes ni ejecutar instalaciones."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tomllib
import zipfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--platform', choices=['linux-x86_64', 'win64'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f'No existe el ejecutable: {binary}')
    args.output.mkdir(parents=True, exist_ok=True)
    name = f'Soryntel-Chemistry-{version}-{args.platform}'
    suffix = '.exe' if args.platform == 'win64' else ''
    files = {'soryntel-desktop' + suffix: binary.read_bytes(),
             'README.md': (root.parent / 'README.md').read_bytes(),
             'MANUAL.md': (root / 'docs/MANUAL_USUARIO.md').read_bytes(),
             'soryntel-icon.png': (root / 'packaging/soryntel-icon.png').read_bytes()}
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
    manifest = {'version': version, 'platform': args.platform, 'commit': revision, 'dirty': dirty,
                'files': {k: hashlib.sha256(v).hexdigest() for k, v in files.items()}}
    files['manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    extension = '.zip' if suffix else '.tar.gz'
    destination = args.output / (name + extension)
    if destination.exists():
        parser.error(f'El paquete ya existe: {destination}')
    if suffix:
        with zipfile.ZipFile(destination, 'w', zipfile.ZIP_DEFLATED) as archive:
            for filename, content in files.items():
                archive.writestr(name + '/' + filename, content)
    else:
        with tarfile.open(destination, 'w:gz') as archive:
            for filename, content in files.items():
                info = tarfile.TarInfo(name + '/' + filename)
                info.size = len(content)
                info.mode = 0o755 if filename == 'soryntel-desktop' else 0o644
                archive.addfile(info, io.BytesIO(content))
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    destination.with_name(destination.name + '.sha256').write_text(f'{digest}  {destination.name}\n')
    print(destination)


if __name__ == '__main__':
    main()
