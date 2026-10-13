"""Executable dependency rules for hexagonal core and application modules."""
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
violations = []
core = ["domain", "application", "nucleo", "motores", "sim", "thermo", "diagrams", "aqueous", "estimation", "db"]
for folder in core:
    for path in (root / "src" / folder).rglob("*"):
        if path.suffix not in {".cpp", ".hpp", ".h"}:
            continue
        for include in re.findall(r'^\s*#include\s*[<"]([^>"]+)', path.read_text(), re.M):
            if any(token in include for token in ("base_datos/", "interfaz/", "adapters/", "sqlite", "imgui", "implot", "GLFW")):
                violations.append(f"{path.relative_to(root)}: forbidden dependency {include}")
for path in (root / "src").rglob("*.cpp"):
    if re.search(r'\b(?:popen|system)\s*\(', path.read_text()):
        violations.append(f"{path.relative_to(root)}: shell execution")
if violations:
    print("\n".join(violations))
    sys.exit(1)
print("Domain/application boundary rules passed")
