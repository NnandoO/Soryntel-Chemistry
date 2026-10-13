"""Historical, scientifically invalid conversion. Retained only for traceability."""
raise SystemExit("Disabled: reaction log K / enthalpy are not absolute species Hf/Sf. Use cma_catalog_audit data and the native MINTEQ importer.")
import csv
import math
def clean_name(name):
    # Remove slashes from redox couples if present, or just replace with hyphen
    name = name.replace('=', '')
    return name.lstrip('/-= ').strip()

def determine_phase(name, default_phase):
    n = name.lower()
    if '(g)' in n or ' (g)' in n:
        return 0
    if '(s)' in n or '(c)' in n or ' (s)' in n:
        return 2
    if '(aq)' in n or '(l)' in n:
        return 1
    return default_phase

def parse_blocks(filename, default_phase):
    results = []
    try:
        with open(filename, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except FileNotFoundError:
        return results

    i = 0
    while i < len(lines):
        line1 = lines[i].strip()
        if not line1 or not line1[0].isdigit():
            i += 1
            continue
        
        parts1 = line1.split(',')
        if len(parts1) < 4:
            i += 1
            continue
            
        try:
            dHr = float(parts1[2])
            logK = float(parts1[3])
        except ValueError:
            i += 1
            continue
            
        name = parts1[1]
        
        # Line 2: Stoichiometry (skip for now)
        i += 1
        
        # Line 3: Mass and Source
        mass = 0.0
        i += 1
        if i < len(lines):
            line3 = lines[i].strip()
            parts3 = line3.split(',')
            if len(parts3) >= 1:
                try:
                    mass = float(parts3[0])
                except ValueError:
                    mass = 0.0
                    
        clean_n = clean_name(name)
        phase = determine_phase(name, default_phase)
        
        # Thermo calc
        dG_kJ = - (8.31446 * 298.15 * math.log(10) / 1000.0) * logK
        if default_phase == 2:  # type6.vdb (dissolution)
            dG_kJ = -dG_kJ
            dHr = -dHr
            
        Sf = ((dHr * 1000.0) - (dG_kJ * 1000.0)) / 298.15
        
        results.append([clean_n, clean_n, mass, dG_kJ, Sf, phase])
        i += 1
        
    return results

def convert_minteq():
    components = {}
    with open('data/comp_2008.vdb', 'r', encoding='utf-8') as f:
        for line in f:
            parts = line.strip().split(',')
            if len(parts) >= 6 and parts[0].isdigit():
                comp_id = parts[0]
                name = clean_name(parts[1])
                try:
                    mass = float(parts[5])
                except ValueError:
                    mass = 0.0
                components[comp_id] = {'name': name, 'mass': mass}

    results = []
    for cid, cdata in components.items():
        results.append([cdata['name'], cdata['name'], cdata['mass'], 0.0, 0.0, 1])

    # Parse thermo (default phase 1), type6 (default 2), gases (default 0)
    results.extend(parse_blocks('data/thermo.vdb', 1))
    results.extend(parse_blocks('data/type6.vdb', 2))
    results.extend(parse_blocks('data/gases.vdb', 0))

    # Write CSV with clean UTF-8 encoding (no BOM)
    with open('data/minteq.csv', 'w', newline='', encoding='utf-8') as f:
        writer = csv.writer(f)
        writer.writerow(["Nombre", "Formula", "Masa_Molar", "Hf", "Sf", "Fase"])
        for r in results:
            writer.writerow(r)
            
    print(f"Conversión exitosa! Generados {len(results)} compuestos.")

if __name__ == '__main__':
    convert_minteq()
