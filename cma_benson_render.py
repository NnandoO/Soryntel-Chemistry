import sys
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D
import numpy as np

def render_xyz(filepath, outpath):
    with open(filepath, 'r') as f:
        lines = f.readlines()
        
    num_atoms = int(lines[0].strip())
    atoms = []
    for line in lines[2:2+num_atoms]:
        parts = line.split()
        if len(parts) >= 4:
            atoms.append((parts[0], float(parts[1]), float(parts[2]), float(parts[3])))
            
    fig = plt.figure(figsize=(8,8))
    ax = fig.add_subplot(111, projection='3d')
    ax.set_axis_off()
    
    colors = {
        'C': '#333333',
        'H': '#EEEEEE',
        'O': '#EE1111',
        'N': '#1111EE',
        'Cl': '#11EE11',
        'F': '#11EE11',
        'S': '#EEEE11'
    }
    
    sizes = {
        'C': 1500,
        'H': 500,
        'O': 1200,
        'N': 1300,
        'Cl': 1800,
        'S': 1600
    }
    
    # Set equal aspect ratio FIRST so bonds don't look distorted
    coords = np.array([[a[1], a[2], a[3]] for a in atoms])
    max_range = np.array([coords[:,0].max()-coords[:,0].min(), 
                          coords[:,1].max()-coords[:,1].min(), 
                          coords[:,2].max()-coords[:,2].min()]).max() / 2.0
    mid_x = (coords[:,0].max()+coords[:,0].min()) * 0.5
    mid_y = (coords[:,1].max()+coords[:,1].min()) * 0.5
    mid_z = (coords[:,2].max()+coords[:,2].min()) * 0.5
    
    ax.set_xlim(mid_x - max_range, mid_x + max_range)
    ax.set_ylim(mid_y - max_range, mid_y + max_range)
    ax.set_zlim(mid_z - max_range, mid_z + max_range)
    
    # Draw bonds (lines between atoms closer than 2.0 Angstroms)
    for i in range(len(atoms)):
        for j in range(i+1, len(atoms)):
            dist = np.linalg.norm(coords[i] - coords[j])
            if dist < 2.0:
                ax.plot([coords[i][0], coords[j][0]], 
                        [coords[i][1], coords[j][1]], 
                        [coords[i][2], coords[j][2]], color='gray', linewidth=4, zorder=0)

    # Draw atoms
    for i, atom in enumerate(atoms):
        elem, x, y, z = atom
        c = colors.get(elem, '#888888')
        s = sizes.get(elem, 1000)
        ax.scatter(x, y, z, c=c, s=s, edgecolors='black', linewidth=1, zorder=10)
    
    plt.tight_layout()
    plt.savefig(outpath, dpi=300, transparent=True)
    print(f"Saved {outpath}")

if __name__ == '__main__':
    if len(sys.argv) > 2:
        render_xyz(sys.argv[1], sys.argv[2])
