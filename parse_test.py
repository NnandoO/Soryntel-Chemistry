import sys

def parse_comp():
    comps = {}
    with open('data/comp_2008.vdb', 'r') as f:
        for line in f:
            parts = line.strip().split(',')
            if len(parts) >= 6 and parts[0].isdigit():
                comps[parts[0]] = parts[1]
    return comps

comps = parse_comp()
print("Comps loaded:", len(comps))
if '330' in comps: print("330 is", comps['330'])
if '471' in comps: print("471 is", comps['471'])
if '2' in comps: print("2 is", comps['2'])
if '1' in comps: print("1 is", comps['1'])
