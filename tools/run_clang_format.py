#!/usr/bin/env python3
"""Prettier-equivalent C++ formatter wrapper using clang-format (.clang-format)."""
import argparse
from pathlib import Path
import shutil
import subprocess
import sys

EXCLUDED_NAMES = {"datos_especies.hpp", "json.hpp"}
EXCLUDED_PARTS = {"nlohmann", "third_party", "build"}


def discover_sources(root: Path, paths: list[str]) -> list[Path]:
    targets = [root / p for p in paths] if paths else [root / "src", root / "tests"]
    files: list[Path] = []
    for target in targets:
        if target.is_file():
            files.append(target)
            continue
        if not target.exists():
            continue
        for path in sorted(target.rglob("*")):
            if path.suffix not in {".cpp", ".hpp", ".h"}:
                continue
            if path.name in EXCLUDED_NAMES or (set(path.parts) & EXCLUDED_PARTS):
                continue
            files.append(path)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description="Run clang-format across hexagonal C++ sources.")
    parser.add_argument("--check", action="store_true", help="Check formatting without modifying files.")
    parser.add_argument("--apply", action="store_true", help="Format files in-place.")
    parser.add_argument("paths", nargs="*", help="Optional subset of directories or files relative to repo root.")
    args = parser.parse_args()

    clang_format = shutil.which("clang-format")
    if not clang_format:
        print("clang-format binary not found in PATH.", file=sys.stderr)
        return 1

    root = Path(__file__).resolve().parents[1]
    files = discover_sources(root, args.paths)
    if not files:
        print("No C++ source files matched.")
        return 0

    if args.apply:
        skipped_line_limit = []
        formatted = 0
        for f in files:
            proc = subprocess.run(
                [clang_format, "--style=file", str(f)],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            )
            new_lines = len(proc.stdout.splitlines())
            if "src" in f.parts and new_lines > 1200:
                skipped_line_limit.append((f.relative_to(root), new_lines))
                continue
            f.write_text(proc.stdout)
            formatted += 1
        print(f"clang-format applied to {formatted} file(s).")
        if skipped_line_limit:
            for rel, lines in skipped_line_limit:
                print(f"  [skipped >1200 lines] {rel} ({lines} lines)")
        return 0

    # Default to --check
    cmd = [clang_format, "--style=file", "--dry-run", "--Werror"] + [str(f) for f in files]
    proc = subprocess.run(cmd, cwd=root, capture_output=True, text=True)
    if proc.returncode != 0:
        print(proc.stderr.strip())
        return proc.returncode
    print(f"clang-format check passed ({len(files)} files verified).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
