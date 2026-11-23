#!/usr/bin/env python3
"""Convert the open canonical CSV to Soryntel's import-compatible CSV.

This creates a neutral interchange file; it does not create or modify HSC files.
"""
import csv, sys
from pathlib import Path

root = Path(__file__).resolve().parents[1] / "data" / "open_catalog"
src = root / "species.csv"
out = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "soryntel_import.csv"
phase = {"gas":"g", "liquid":"l", "solid":"s", "aqueous":"ao"}
with src.open(encoding="utf-8", newline="") as f, out.open("w", encoding="utf-8", newline="") as g:
    rows = csv.DictReader(f)
    w = csv.writer(g)
    w.writerow(["Formula","StructuralFormula","ChemicalName","CommonName","CAS","MW","Mp_K","Bp_K","T1_K","T2_K","Phase","H_kcal","S_cal","CpA_cal","CpB_cal","CpC_cal","CpD_cal","Density_g_cm3","Color","Solubility","Reference","Reliability"])
    for r in rows:
        # Canonical values are SI; import format follows the historical CSV units.
        w.writerow([r["formula"],"","","","",r["mw_g_mol"],"0","0",r["t_min_k"],r["t_max_k"],phase[r["phase"]],float(r["h298_kj_mol"])/4.184,float(r["s298_j_mol_k"])/4.184,float(r["cp_a_j_mol_k"])/4.184,float(r["cp_b"])/4.184,float(r["cp_c"])/4.184,float(r["cp_d"])/4.184,"0","","",r["source_id"],"1"])
print(f"wrote {out}")

