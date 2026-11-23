#!/usr/bin/env python3
"""Offline validator for the independently licensed thermodynamic catalog."""
import csv
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "data" / "open_catalog"
SPEC = {"formula","phase","mw_g_mol","h298_kj_mol","s298_j_mol_k","t_min_k","t_max_k","cp_a_j_mol_k","cp_b","cp_c","cp_d","source_id","quality"}
PROV = {"source_id","source_url","source_version","license","retrieved_sha256","transformation","notes"}
PHASES = {"gas","liquid","solid","aqueous"}

def rows(path):
    with path.open(encoding="utf-8", newline="") as f:
        r = csv.DictReader(f)
        if not r.fieldnames: raise SystemExit(f"{path}: missing header")
        return set(r.fieldnames), list(r)

def main():
    sh, species = rows(ROOT / "species.csv")
    ph, provenance = rows(ROOT / "provenance.csv")
    if SPEC - sh: raise SystemExit(f"species.csv missing columns: {sorted(SPEC-sh)}")
    if PROV - ph: raise SystemExit(f"provenance.csv missing columns: {sorted(PROV-ph)}")
    sources = {x["source_id"]: x for x in provenance}
    if len(sources) != len(provenance): raise SystemExit("duplicate source_id")
    for n, x in enumerate(species, 2):
        if not x["formula"] or x["phase"] not in PHASES: raise SystemExit(f"species.csv:{n}: invalid formula/phase")
        try:
            lo, hi = float(x["t_min_k"]), float(x["t_max_k"])
            nums = [float(x[k]) for k in SPEC if k not in {"formula","phase","source_id","quality"}]
        except ValueError as e: raise SystemExit(f"species.csv:{n}: invalid number: {e}")
        if not (0 < lo < hi) or any(v != v or abs(v) == float("inf") for v in nums): raise SystemExit(f"species.csv:{n}: invalid range/non-finite value")
        if x["quality"] not in {"reviewed", "derived"}: raise SystemExit(f"species.csv:{n}: invalid quality")
        p = sources.get(x["source_id"])
        if not p: raise SystemExit(f"species.csv:{n}: unknown source_id")
        if not p["license"] or len(p["retrieved_sha256"]) != 64: raise SystemExit(f"provenance for {x['source_id']} lacks license/hash")
    print(f"open catalog OK: {len(species)} intervals, {len(sources)} sources")

if __name__ == "__main__": main()

