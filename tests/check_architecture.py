#!/usr/bin/env python3
"""ArchUnit / Deptrac equivalent for C++ Hexagonal Architecture:
1. Validates the Layer Dependency Matrix (Domain <- Application/Engines <- Adapters/UI).
2. Validates the Transitive #include Graph across all headers and sources in src/.
3. Validates Physical CMake Target Isolation (CMakeLists.txt & src/*/CMakeLists.txt)
   ensuring core targets cannot link or include ImGui, ImPlot, GLFW, SQLite, or UI/Persistence adapters.
4. Forbids shell execution (popen / system) across src/.
"""
from collections import deque
import pathlib
import re
import sys

# Layer definitions (innermost = 0, outermost = 2)
DOMAIN_FOLDERS = {"domain", "nucleo", "nlohmann"}
APPLICATION_FOLDERS = {
    "application",
    "motores",
    "sim",
    "thermo",
    "diagrams",
    "aqueous",
    "estimation",
    "db",
    "tools",
    "diagrama_flujo",
}
ADAPTER_FOLDERS = {"adapters", "base_datos", "interfaz", "utils"}

CORE_FOLDERS = DOMAIN_FOLDERS | APPLICATION_FOLDERS

FORBIDDEN_IN_DOMAIN_TOKENS = (
    "application/",
    "motores/",
    "sim/",
    "thermo/",
    "diagrams/",
    "aqueous/",
    "estimation/",
    "db/",
    "tools/",
    "diagrama_flujo/",
    "base_datos/",
    "interfaz/",
    "adapters/",
    "sqlite",
    "imgui",
    "implot",
    "GLFW",
)

FORBIDDEN_IN_CORE_TOKENS = (
    "base_datos/",
    "interfaz/",
    "adapters/",
    "sqlite",
    "imgui",
    "implot",
    "GLFW",
)

# CMake targets belonging to Domain & Application layers that must never link to UI/DB/Desktop adapters
PURE_CMAKE_TARGETS = {
    "cma_domain",
    "cma_thermo",
    "cma_diagrams",
    "cma_tools",
    "cma_sim",
    "cma_estimation",
    "cma_aqueous",
    "cma_db",
}

FORBIDDEN_CMAKE_LINKS = {
    "cma_graphics",
    "cma_persistence",
    "cma_platform",
    "glfw",
    "imgui",
    "implot",
    "implot3d",
    "sqlite3",
    "sqlite3_amalgamation",
    "SQLite3::SQLite3",
    "SQLite::SQLite3",
    "${CMA_SQLITE_TARGET}",
    "OpenGL::GL",
}


def resolve_local_include(including_file: pathlib.Path, include_str: str, src_root: pathlib.Path) -> pathlib.Path | None:
    """Resolve an #include string to a canonical Path inside src/ if it belongs to project sources."""
    rel_candidate = (including_file.parent / include_str).resolve()
    if rel_candidate.exists() and src_root in rel_candidate.parents:
        return rel_candidate
    src_candidate = (src_root / include_str).resolve()
    if src_candidate.exists() and src_root in src_candidate.parents:
        return src_candidate
    return None


def layer_of(path: pathlib.Path, src_root: pathlib.Path) -> str:
    rel = path.relative_to(src_root)
    if len(rel.parts) < 2:
        return "adapter"
    top = rel.parts[0]
    if top in DOMAIN_FOLDERS:
        return "domain"
    if top in APPLICATION_FOLDERS:
        return "application"
    return "adapter"


def check_include_layers_and_graph(root: pathlib.Path) -> list[str]:
    violations: list[str] = []
    src_root = (root / "src").resolve()
    include_re = re.compile(r'^\s*#include\s*[<"]([^>"]+)[>"]', re.M)

    all_files: list[pathlib.Path] = []
    direct_includes: dict[pathlib.Path, list[str]] = {}
    local_graph: dict[pathlib.Path, list[pathlib.Path]] = {}

    for path in sorted(src_root.rglob("*")):
        if path.suffix not in {".cpp", ".hpp", ".h"} or not path.is_file():
            continue
        resolved = path.resolve()
        all_files.append(resolved)
        text = resolved.read_text(errors="ignore")
        includes = include_re.findall(text)
        direct_includes[resolved] = includes
        neighbors: list[pathlib.Path] = []
        for inc in includes:
            target = resolve_local_include(resolved, inc, src_root)
            if target is not None:
                neighbors.append(target)
        local_graph[resolved] = neighbors

    # 1. Direct include rules for Domain and Core (Domain + Application)
    for path in all_files:
        layer = layer_of(path, src_root)
        rel_path = path.relative_to(root.resolve())
        if layer == "domain":
            for inc in direct_includes[path]:
                if any(token in inc for token in FORBIDDEN_IN_DOMAIN_TOKENS):
                    violations.append(f"{rel_path}: [Domain Layer Violation] forbidden dependency '{inc}'")
        elif layer == "application":
            for inc in direct_includes[path]:
                if any(token in inc for token in FORBIDDEN_IN_CORE_TOKENS):
                    violations.append(f"{rel_path}: [Application Layer Violation] forbidden dependency '{inc}'")

    # 2. Transitive include graph validation (no transitive leak from outer layer into inner layer)
    for start in all_files:
        start_layer = layer_of(start, src_root)
        if start_layer not in {"domain", "application"}:
            continue
        visited = {start}
        queue = deque([(start, [start])])
        while queue:
            curr, chain = queue.popleft()
            for nxt in local_graph.get(curr, []):
                if nxt in visited:
                    continue
                visited.add(nxt)
                nxt_layer = layer_of(nxt, src_root)
                new_chain = chain + [nxt]
                if start_layer == "domain" and nxt_layer != "domain":
                    pretty = " -> ".join(str(p.relative_to(root.resolve())) for p in new_chain)
                    violations.append(f"[Transitive Domain Leak] {pretty} (reached {nxt_layer} layer)")
                elif start_layer == "application" and nxt_layer == "adapter":
                    pretty = " -> ".join(str(p.relative_to(root.resolve())) for p in new_chain)
                    violations.append(f"[Transitive Application Leak] {pretty} (reached adapter layer)")
                else:
                    queue.append((nxt, new_chain))

    return violations


def check_cmake_target_isolation(root: pathlib.Path) -> list[str]:
    violations: list[str] = []
    cmake_files = [root / "CMakeLists.txt"] + sorted((root / "src").rglob("CMakeLists.txt"))
    link_re = re.compile(r"target_link_libraries\s*\(\s*(\S+)([^)]*)\)", re.S)
    inc_re = re.compile(r"target_include_directories\s*\(\s*(\S+)([^)]*)\)", re.S)

    for cmake_path in cmake_files:
        if not cmake_path.exists():
            continue
        content = cmake_path.read_text()
        rel = cmake_path.relative_to(root)
        for match in link_re.finditer(content):
            target, body = match.group(1), match.group(2)
            if target not in PURE_CMAKE_TARGETS:
                continue
            tokens = set(body.split())
            forbidden = tokens & FORBIDDEN_CMAKE_LINKS
            if forbidden:
                violations.append(
                    f"{rel}: [CMake Target Isolation Violation] pure target '{target}' links forbidden {sorted(forbidden)}"
                )
        for match in inc_re.finditer(content):
            target, body = match.group(1), match.group(2)
            if target not in PURE_CMAKE_TARGETS:
                continue
            for bad_inc in ("imgui", "implot", "glfw", "sqlite"):
                if bad_inc in body.lower():
                    violations.append(
                        f"{rel}: [CMake Include Isolation Violation] pure target '{target}' includes '{bad_inc}'"
                    )
    return violations


def check_shell_execution(root: pathlib.Path) -> list[str]:
    violations: list[str] = []
    for path in sorted((root / "src").rglob("*.cpp")):
        if re.search(r"\b(?:popen|system)\s*\(", path.read_text(errors="ignore")):
            violations.append(f"{path.relative_to(root)}: forbidden shell execution (popen/system)")
    return violations


def main() -> int:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    violations = (
        check_include_layers_and_graph(root)
        + check_cmake_target_isolation(root)
        + check_shell_execution(root)
    )
    if violations:
        print("\n".join(violations))
        return 1
    print(
        "Domain/application boundary rules passed "
        "(Direct + Transitive #include graph & CMake target isolation verified)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
