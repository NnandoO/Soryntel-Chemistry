#!/usr/bin/env python3
"""ESLint-equivalent C++ static AST linter runner using clang-tidy (.clang-tidy + compile_commands.json)."""
import argparse
from pathlib import Path
import shutil
import subprocess
import sys

ROOT_DIR = Path(__file__).resolve().parents[1]

DEFAULT_TARGET_DIRS = [
    "src/thermo",
    "src/diagrams",
    "src/aqueous",
    "src/estimation",
    "src/sim",
    "src/tools",
    "src/db",
    "src/motores",
]


def discover_default_targets() -> list[str]:
    targets: list[str] = []
    for rel_dir in DEFAULT_TARGET_DIRS:
        dir_path = ROOT_DIR / rel_dir
        if dir_path.is_dir():
            for cpp_file in sorted(dir_path.rglob("*.cpp")):
                targets.append(str(cpp_file.relative_to(ROOT_DIR)))
    return targets


DEFAULT_TARGETS = discover_default_targets()


def main() -> int:
    parser = argparse.ArgumentParser(description="Run clang-tidy with compile_commands.json.")
    parser.add_argument("-p", "--build-dir", default="build", help="Path to build directory with compile_commands.json")
    parser.add_argument("files", nargs="*", help="Source files to analyze (defaults to core hexagonal engines).")
    args = parser.parse_args()

    clang_tidy = shutil.which("clang-tidy")
    if not clang_tidy:
        print("clang-tidy binary not found in PATH.", file=sys.stderr)
        return 1

    root = Path(__file__).resolve().parents[1]
    build_dir = root / args.build_dir
    compile_db = build_dir / "compile_commands.json"
    if not compile_db.exists():
        print(f"Missing {compile_db}. Configure CMake with -DCMAKE_EXPORT_COMPILE_COMMANDS=ON first.", file=sys.stderr)
        return 1

    targets = args.files if args.files else DEFAULT_TARGETS
    resolved = [str(root / t) for t in targets]
    cmd = [clang_tidy, f"-p={build_dir}", "--quiet"] + resolved
    proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    if proc.stdout.strip():
        print(proc.stdout.strip())
    if proc.returncode != 0:
        if proc.stderr.strip():
            print(proc.stderr.strip(), file=sys.stderr)
        return proc.returncode
    print(f"clang-tidy AST analysis passed ({len(resolved)} translation units verified).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
