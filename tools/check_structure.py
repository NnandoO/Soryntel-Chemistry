"""Report source concentration; line counts are signals, not proof of modularity."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
misplaced = sorted(list(root.glob('*.py')) + list(root.glob('test*.cpp')))
sources = []
for path in (root / 'src').rglob('*'):
    if path.suffix not in {'.cpp', '.hpp', '.h'}:
        continue
    if 'nlohmann' in path.parts or path.name == 'datos_especies.hpp':
        continue  # bundled dependency and thermochemical dataset
    sources.append((len(path.read_text().splitlines()), path.relative_to(root)))
print('Largest authored source files (excluding bundled JSON and species data):')
for lines, path in sorted(sources, reverse=True)[:15]:
    print(f'{lines:5}  {path}')
large = [str(path) for lines, path in sources if lines > 1200]
if misplaced:
    print('Misplaced scripts/tests:', ', '.join(str(p.relative_to(root)) for p in misplaced))
if large:
    print('Review responsibility concentration:', ', '.join(large))
print('Also run tests/check_architecture.py: file length alone cannot prove architecture.')
sys.exit(bool(misplaced or large))
