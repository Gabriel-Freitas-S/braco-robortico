import trimesh
import numpy as np
import os
import json

parts_dir = r"d:\braco-robortico\pecas_impressao_3d"

# Updated Plate 1 layout accommodating the new ergonomic 01_Base_Chassi_Inferior (122 x 177.1 mm)
# Bed bounds: X in [-125, 125], Y in [-125, 125]
# Let's shift base slightly to X = -58 so X is [-119, +3], leaving clear space for right column (X > 10)
plate1_new = [
    ("01_Base_Chassi_Inferior", -58, 0, 0),
    ("17_Travessa_Reforco_Base_Frontal", -58, 108, 0),
    ("18_Travessa_Reforco_Base_Traseira", -58, -108, 0),
    ("12_Mesa_Giratoria_Turntable", 40, 80, 0),
    ("25_Trava_Horn_Servo_Base", 94, 80, 0),
    ("09_Torre_Suporte_Servo_Direita", 36, 10, 0),
    ("11_Torre_Suporte_Servo_Esquerda", 92, 10, 0),
    ("16_Berco_Servo_Rotacao_Base", 36, -65, 0),
    ("14_Separador_Espacador_Central_Base", 92, -65, 90),
]

# Load meshes and compute 2D bounding boxes
boxes = []
all_clear = True
bed_limit = 125.0 # mm from center (250x250 printable area)

for name, cx, cy, rot in plate1_new:
    stl_path = os.path.join(parts_dir, f"{name}.stl")
    m = trimesh.load(stl_path)
    if rot != 0:
        R = trimesh.transformations.rotation_matrix(np.radians(rot), [0, 0, 1])
        m.apply_transform(R)
    b = m.bounds
    sx = b[1][0] - b[0][0]
    sy = b[1][1] - b[0][1]
    
    x_min = cx - sx/2.0
    x_max = cx + sx/2.0
    y_min = cy - sy/2.0
    y_max = cy + sy/2.0
    
    if x_min < -bed_limit or x_max > bed_limit or y_min < -bed_limit or y_max > bed_limit:
        print(f"OUT OF BOUNDS: {name} -> X:[{x_min:.1f}, {x_max:.1f}], Y:[{y_min:.1f}, {y_max:.1f}]")
        all_clear = False
        
    boxes.append({
        'name': name,
        'bounds': (x_min, x_max, y_min, y_max),
        'pos': (cx, cy),
        'size': (sx, sy)
    })
    print(f"{name:35s}: X=[{x_min:6.1f}, {x_max:6.1f}], Y=[{y_min:6.1f}, {y_max:6.1f}]")

# Check pairwise overlap with 2.0 mm safety clearance
for i in range(len(boxes)):
    for j in range(i+1, len(boxes)):
        b1 = boxes[i]['bounds']
        b2 = boxes[j]['bounds']
        gap = 2.0
        overlap_x = (b1[0] - gap < b2[1]) and (b1[1] + gap > b2[0])
        overlap_y = (b1[2] - gap < b2[3]) and (b1[3] + gap > b2[2])
        if overlap_x and overlap_y:
            print(f"COLLISION: {boxes[i]['name']} vs {boxes[j]['name']}")
            all_clear = False

if all_clear:
    print("\nSUCCESS: All 9 parts of Plate 1 fit cleanly with >2mm clearance on Bambu A1 bed!")
