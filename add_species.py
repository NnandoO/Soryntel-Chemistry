import re

# Standard thermodynamic data for ions (Hf in kJ/mol, Sf in J/mol*K)
# Note: Format must match: 
# {"Nombre", "Formula", Masa_molar, h_formacion, s_formacion, cp_a, cp_b, cp_c, cp_d, antoine_a, antoine_b, antoine_c, temp_min, temp_max, fase_std(0=gas,1=liq,2=sol)}
# For aqueous ions we'll use fase_std = 1 (líquido/acuoso).

new_species = """
        // ── Iones Acuosos y Variantes Pourbaix ──────────────────────────────
        // Hierro (Fe)
        {"Hierro(II) acuoso", "Fe(+2a)", 55.845, -89.1, -137.7, 0,0,0,0, 0,0,0,0,0, 1},
        {"Hierro(III) acuoso", "Fe(+3a)", 55.845, -48.5, -315.9, 0,0,0,0, 0,0,0,0,0, 1},
        {"Dihidroxoferrato(II)", "HFeO2(-a)", 88.85, -379.0, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Ferrato(VI)", "FeO4(-2a)", 119.84, -454.0, 50.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Cobre (Cu)
        {"Cobre(I) acuoso", "Cu(+a)", 63.546, 51.9, 40.6, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cobre(II) acuoso", "Cu(+2a)", 63.546, 64.77, -99.6, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cuprato(II) biácido", "HCuO2(-a)", 96.55, -258.5, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cuprato(II)", "CuO2(-2a)", 95.54, -183.6, 20.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Zinc (Zn)
        {"Zinc(II) acuoso", "Zn(+2a)", 65.38, -153.89, -112.1, 0,0,0,0, 0,0,0,0,0, 1},
        {"Zincato", "ZnO2(-2a)", 97.38, -390.4, 20.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Hidroxozincato", "HZnO2(-a)", 98.39, -464.4, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Aluminio (Al)
        {"Aluminio(III) acuoso", "Al(+3a)", 26.98, -531.0, -321.7, 0,0,0,0, 0,0,0,0,0, 1},
        {"Aluminato", "AlO2(-a)", 58.98, -940.1, 41.8, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Níquel (Ni)
        {"Níquel(II) acuoso", "Ni(+2a)", 58.69, -54.0, -128.9, 0,0,0,0, 0,0,0,0,0, 1},
        {"Niquelato biácido", "HNiO2(-a)", 91.70, -347.0, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Cromo (Cr)
        {"Cromo(II) acuoso", "Cr(+2a)", 51.996, -143.5, -10.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cromo(III) acuoso", "Cr(+3a)", 51.996, -254.0, -282.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cromato", "CrO4(-2a)", 115.99, -881.1, 50.2, 0,0,0,0, 0,0,0,0,0, 1},
        {"Dicromato", "Cr2O7(-2a)", 215.99, -1490.3, 261.9, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Especies del Azufre y Carbono y otros iones comunes
        {"Protón", "H(+a)", 1.008, 0.0, 0.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Hidroxilo", "OH(-a)", 17.008, -230.0, -10.75, 0,0,0,0, 0,0,0,0,0, 1},
        {"Sulfato", "SO4(-2a)", 96.06, -909.27, 20.1, 0,0,0,0, 0,0,0,0,0, 1},
        {"Bisulfato", "HSO4(-a)", 97.07, -887.3, 131.8, 0,0,0,0, 0,0,0,0,0, 1},
        {"Sulfuro", "S(-2a)", 32.06, 33.1, -14.6, 0,0,0,0, 0,0,0,0,0, 1},
        {"Bisulfuro", "HS(-a)", 33.07, -17.6, 62.8, 0,0,0,0, 0,0,0,0,0, 1},
        {"Carbonato", "CO3(-2a)", 60.01, -677.1, -56.9, 0,0,0,0, 0,0,0,0,0, 1},
        {"Bicarbonato", "HCO3(-a)", 61.02, -691.99, 91.2, 0,0,0,0, 0,0,0,0,0, 1},
        {"Nitrato", "NO3(-a)", 62.00, -205.0, 146.4, 0,0,0,0, 0,0,0,0,0, 1},
"""

with open("src/base_datos/datos_especies.hpp", "r") as f:
    content = f.read()

# Insert before the final closing brace of data vector
# The vector ends with "    };"
replacement = new_species + "\n    };\n"
content = content.replace("    };\n", replacement)

with open("src/base_datos/datos_especies.hpp", "w") as f:
    f.write(content)
print("Database successfully expanded!")
