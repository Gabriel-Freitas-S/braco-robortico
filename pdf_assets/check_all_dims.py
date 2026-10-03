import trimesh
import glob
import os
import re

with open(r'd:\braco-robortico\manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    html = f.read()

# Extract dims for each card
# <div class="part-name">(\d{2}_[^\.]+)\.stl</div>.*?<div class="spec-k">Dimensões (XYZ)</div>\s*<div class="spec-v">([^<]+)</div>
cards = re.findall(
    r'<div class="part-name">(\d{2}_[^\.]+)\.stl</div>.*?'
    r'<div class="spec-k">Dimensões (XYZ)</div>\s*<div class="spec-v">([^<]+)</div>',
    html,
    re.DOTALL
)

print(f"Checking {len(cards)} parts dimensions against STLs...")
for name, dims_str in cards:
    stl_path = os.path.join(r"d:\braco-robortico\pecas_impressao_3d", f"{name}.stl")
    if os.path.exists(stl_path):
        m = trimesh.load(stl_path)
        extents = m.extents
        # Sort or compare
        print(f"{name}:")
        print(f"  Manual: {dims_str.strip()}")
        print(f"  Actual: {extents[0]:.1f} × {extents[1]:.1f} × {extents[2]:.1f} mm")
