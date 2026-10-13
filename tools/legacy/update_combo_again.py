import re

with open("src/interfaz/panel_pourbaix.cpp", "r") as f:
    content = f.read()

old_combo = """const char* elementos[] = {"Fe", "Cu", "Zn", "Al", "Ni", "Cr", "S", "C", "N", "Ag", "Au", "Pt", "Mg", "Ca", "U", "Mn", "Ti", "Co", "Pb", "Sn", "Hg", "V", "W"};"""
new_combo = """const char* elementos[] = {"Fe", "Cu", "Zn", "Al", "Ni", "Cr", "S", "C", "N", "Ag", "Au", "Pt", "Mg", "Ca", "U", "Mn", "Ti", "Co", "Pb", "Sn", "Hg", "V", "W", "As", "Mo"};"""

content = content.replace(old_combo, new_combo)

with open("src/interfaz/panel_pourbaix.cpp", "w") as f:
    f.write(content)
