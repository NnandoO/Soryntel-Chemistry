import re

new_species = """
        // ── Adiciones Estratégicas Extras ────────────────────────────────────
        // Níquel (Ni) adicionales
        {"Níquel (solido)", "Ni", 58.693, 0.0, 29.87, 0,0,0,0, 0,0,0,0,0, 2},
        {"Hidróxido de níquel(II)", "Ni(OH)2", 92.71, -528.0, 88.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de níquel(III)", "Ni2O3", 165.38, -489.5, 104.6, 0,0,0,0, 0,0,0,0,0, 2},
        
        // Cromo (Cr) adicionales
        {"Cromo (solido)", "Cr", 51.996, 0.0, 23.77, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de cromo(III)", "Cr2O3", 151.99, -1139.7, 81.2, 0,0,0,0, 0,0,0,0,0, 2},
        {"Dióxido de cromo", "CrO2", 83.99, -598.0, 50.0, 0,0,0,0, 0,0,0,0,0, 2},
        
        // Arsénico (As)
        {"Arsénico (solido)", "As", 74.92, 0.0, 35.1, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de arsénico(III)", "As2O3", 197.84, -653.0, 105.4, 0,0,0,0, 0,0,0,0,0, 2},
        {"Ácido arsenioso", "H3AsO3", 125.94, -742.2, 195.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Ácido arsénico", "H3AsO4", 141.94, -904.6, 184.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Arseniato biácido", "H2AsO4(-a)", 140.93, -900.4, 117.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Arseniato ácido", "HAsO4(-2a)", 139.92, -861.3, -4.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Arseniato", "AsO4(-3a)", 138.91, -786.1, -162.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Molibdeno (Mo)
        {"Molibdeno (solido)", "Mo", 95.95, 0.0, 28.6, 0,0,0,0, 0,0,0,0,0, 2},
        {"Dióxido de molibdeno", "MoO2", 127.95, -588.9, 46.3, 0,0,0,0, 0,0,0,0,0, 2},
        {"Molibdato", "MoO4(-2a)", 159.94, -997.9, 65.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Molibdeno(III) acuoso", "Mo(+3a)", 95.95, -60.0, -150.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Azufre (S) adicionales acuosos
        {"Azufre (solido)", "S", 32.06, 0.0, 32.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Ácido sulfhídrico acuoso", "H2S(a)", 34.08, -39.7, 121.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Cobalto (Co) adicionales
        {"Hidróxido de cobalto(II)", "Co(OH)2", 92.95, -539.7, 79.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de cobalto(III)", "Co2O3", 165.86, -605.0, 102.0, 0,0,0,0, 0,0,0,0,0, 2},
"""

with open("src/base_datos/datos_especies.hpp", "r") as f:
    content = f.read()

# Insert before the final closing brace of data vector
replacement = new_species + "\n    };\n"
content = content.replace("    };\n", replacement)

with open("src/base_datos/datos_especies.hpp", "w") as f:
    f.write(content)
print("Extra database species injected.")
