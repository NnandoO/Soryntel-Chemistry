import re

new_species = """
        // ── Inyección Masiva (Plan Híbrido) ──────────────────────────────────
        // Plata (Ag)
        {"Plata (solido)", "Ag", 107.868, 0.0, 42.55, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de plata(I)", "Ag2O", 231.735, -31.05, 121.3, 0,0,0,0, 0,0,0,0,0, 2},
        {"Plata(I) acuoso", "Ag(+a)", 107.868, 105.58, 72.68, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Oro (Au)
        {"Oro (solido)", "Au", 196.967, 0.0, 47.4, 0,0,0,0, 0,0,0,0,0, 2},
        {"Oro(I) acuoso", "Au(+a)", 196.967, 280.0, 80.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Oro(III) acuoso", "Au(+3a)", 196.967, 260.0, 100.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Óxido de oro(III)", "Au2O3", 441.93, -3.4, 130.0, 0,0,0,0, 0,0,0,0,0, 2},
        
        // Platino (Pt)
        {"Platino (solido)", "Pt", 195.084, 0.0, 41.6, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de platino(II)", "PtO", 211.08, -80.0, 40.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Óxido de platino(IV)", "PtO2", 227.08, -170.0, 50.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Platino(II) acuoso", "Pt(+2a)", 195.084, 120.0, -100.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Magnesio (Mg)
        {"Magnesio (solido)", "Mg", 24.305, 0.0, 32.7, 0,0,0,0, 0,0,0,0,0, 2},
        {"Magnesio(II) acuoso", "Mg(+2a)", 24.305, -466.85, -138.1, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Calcio (Ca)
        {"Calcio (solido)", "Ca", 40.078, 0.0, 41.4, 0,0,0,0, 0,0,0,0,0, 2},
        {"Calcio(II) acuoso", "Ca(+2a)", 40.078, -542.83, -53.1, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Uranio (U)
        {"Uranio (solido)", "U", 238.029, 0.0, 50.2, 0,0,0,0, 0,0,0,0,0, 2},
        {"Dióxido de uranio", "UO2", 270.03, -1085.0, 77.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Trióxido de uranio", "UO3", 286.03, -1223.8, 96.1, 0,0,0,0, 0,0,0,0,0, 2},
        {"Uranio(IV) acuoso", "U(+4a)", 238.029, -591.2, -410.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Uranilo(VI)", "UO2(+2a)", 270.03, -1019.0, -98.2, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Manganeso (Mn)
        {"Manganeso(II) acuoso", "Mn(+2a)", 54.938, -220.8, -73.6, 0,0,0,0, 0,0,0,0,0, 1},
        {"Permanganato", "MnO4(-a)", 118.94, -541.4, 191.2, 0,0,0,0, 0,0,0,0,0, 1},
        {"Manganato", "MnO4(-2a)", 118.94, -653.0, 59.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Titanio (Ti)
        {"Titanio (solido)", "Ti", 47.867, 0.0, 30.6, 0,0,0,0, 0,0,0,0,0, 2},
        {"Titanio(II) acuoso", "Ti(+2a)", 47.867, -252.0, -80.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Titanio(III) acuoso", "Ti(+3a)", 47.867, -324.0, -150.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Óxido de titanio(III)", "Ti2O3", 143.73, -1520.9, 77.2, 0,0,0,0, 0,0,0,0,0, 2},
        
        // Cobalto (Co)
        {"Cobalto (solido)", "Co", 58.933, 0.0, 30.0, 0,0,0,0, 0,0,0,0,0, 2},
        {"Cobalto(II) acuoso", "Co(+2a)", 58.933, -58.2, -113.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Cobalto(III) acuoso", "Co(+3a)", 58.933, 114.0, -300.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Óxido de cobalto(II,III)", "Co3O4", 240.8, -891.0, 102.5, 0,0,0,0, 0,0,0,0,0, 2},
        
        // Plomo (Pb)
        {"Plomo (solido)", "Pb", 207.2, 0.0, 64.8, 0,0,0,0, 0,0,0,0,0, 2},
        {"Plomo(II) acuoso", "Pb(+2a)", 207.2, -1.7, 10.5, 0,0,0,0, 0,0,0,0,0, 1},
        {"Dióxido de plomo", "PbO2", 239.2, -277.4, 68.6, 0,0,0,0, 0,0,0,0,0, 2},
        {"Plumbato(II)", "HPbO2(-a)", 240.2, -438.0, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Estaño (Sn)
        {"Estaño (solido)", "Sn", 118.71, 0.0, 51.2, 0,0,0,0, 0,0,0,0,0, 2},
        {"Estaño(II) acuoso", "Sn(+2a)", 118.71, -8.8, -17.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Dióxido de estaño", "SnO2", 150.71, -577.6, 52.3, 0,0,0,0, 0,0,0,0,0, 2},
        {"Estannito", "HSnO2(-a)", 151.71, -446.0, 40.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Estannato", "SnO3(-2a)", 166.71, -550.0, 20.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Mercurio (Hg)
        {"Mercurio (liquido)", "Hg", 200.59, 0.0, 75.9, 0,0,0,0, 0,0,0,0,0, 1},
        {"Óxido de mercurio(II)", "HgO", 216.59, -90.8, 70.3, 0,0,0,0, 0,0,0,0,0, 2},
        {"Mercurio(II) acuoso", "Hg(+2a)", 200.59, 171.1, -32.2, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Vanadio (V)
        {"Vanadio (solido)", "V", 50.94, 0.0, 28.9, 0,0,0,0, 0,0,0,0,0, 2},
        {"Vanadio(II) acuoso", "V(+2a)", 50.94, -225.0, -100.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Vanadio(III) acuoso", "V(+3a)", 50.94, -259.0, -150.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Vanadilo(IV)", "VO(+2a)", 66.94, -483.0, -42.0, 0,0,0,0, 0,0,0,0,0, 1},
        {"Dioxovanadio(V)", "VO2(+a)", 82.94, -649.0, -20.0, 0,0,0,0, 0,0,0,0,0, 1},
        
        // Wolframio (W)
        {"Wolframio (solido)", "W", 183.84, 0.0, 32.6, 0,0,0,0, 0,0,0,0,0, 2},
        {"Dióxido de wolframio", "WO2", 215.84, -589.7, 50.5, 0,0,0,0, 0,0,0,0,0, 2},
        {"Wolframato", "WO4(-2a)", 247.84, -1074.0, 100.0, 0,0,0,0, 0,0,0,0,0, 1},
"""

with open("src/base_datos/datos_especies.hpp", "r") as f:
    content = f.read()

# Insert before the final closing brace of data vector
replacement = new_species + "\n    };\n"
content = content.replace("    };\n", replacement)

with open("src/base_datos/datos_especies.hpp", "w") as f:
    f.write(content)
print("Massive DB Expansion inserted successfully!")
