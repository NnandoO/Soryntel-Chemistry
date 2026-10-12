from thermo import Chemical
import pandas as pd
import csv

# We will try to get elements and common compounds from thermo
elements = ["H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S", "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd", "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg", "Tl", "Pb", "Bi", "Th", "Pa", "U"]
compounds = []

# Generate common formulas: Oxides, chlorides, sulfides
for el in elements:
    compounds.append(el)
    for n in range(1, 5):
        for o in range(1, 5):
            if n==1:
                compounds.append(f"{el}O{o}")
            else:
                compounds.append(f"{el}{n}O{o}")

# We only process a subset if they resolve
results = []
for c in compounds:
    try:
        chem = Chemical(c)
        if chem.Hfm and chem.Sfm:
            hf_kj = chem.Hfm / 1000.0
            sf = chem.Sfm
            # thermo library usually defaults to gas or liquid. 
            # We will assume solid for metals if phase is solid
            fase = 0
            if chem.phase == 'l': fase = 1
            if chem.phase == 's': fase = 2
            
            # Avoid duplicates
            if not any(r[1] == c for r in results):
                results.append((chem.name, chem.formula, chem.MW, hf_kj, sf, fase))
    except:
        pass

with open("data/massive_db.csv", "w", newline='') as f:
    writer = csv.writer(f)
    writer.writerow(["Nombre", "Formula", "Masa_Molar", "Hf", "Sf", "Fase"])
    for r in results:
        writer.writerow(r)
print(f"Generated massive_db.csv with {len(results)} compounds!")
