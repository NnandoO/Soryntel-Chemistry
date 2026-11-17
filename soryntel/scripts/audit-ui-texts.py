#!/usr/bin/env python3
"""Inventario de literales de presentación que siguen fuera del catálogo.

Es un inventario para revisión humana: incluye español y notación técnica, y no
abarca todas las expresiones dinámicas ni demuestra cobertura total del idioma.
"""
import argparse
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
source = root / 'crates/soryntel-desktop/src'
pattern = re.compile(r'(?:button|label|heading|menu_button|on_hover_text|hint_text|Window::new|RichText::new)\(\s*("(?:[^"\\]|\\.)*")')
records = []
for path in sorted(source.rglob('*.rs')):
    if path.name == 'i18n.rs':
        continue
    code = path.read_text()
    for match in pattern.finditer(code):
        try:
            text = json.loads(match[1])
        except ValueError:
            continue
        if not re.search('[A-Za-zÁÉÍÓÚáéíóúñÑ]{3}', text):
            continue
        records.append({'archivo': str(path.relative_to(root)),
                        'linea': code.count('\n', 0, match.start()) + 1,
                        'texto': text})
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps({'alcance': __doc__, 'literales': records}, ensure_ascii=False, indent=2) + '\n')
print(f'{len(records)} literales de presentación para revisión humana: {args.output}')
