import trimesh
import numpy as np
import os
import json

parts_dir = r"d:\braco-robortico\pecas_impressao_3d"
p1_parts = [
    "01_Base_Chassi_Inferior",
    "02_Braco_Principal_Esquerdo",
    "07_Braco_Principal_Direito",
    "12_Mesa_Giratoria_Turntable",
    "09_Torre_Suporte_Servo_Direita",
    "11_Torre_Suporte_Servo_Esquerda",
    "16_Berco_Servo_Rotacao_Base",
    "17_Travessa_Reforco_Base_Frontal",
    "18_Travessa_Reforco_Base_Traseira",
]

meshes = {}
for p in p1_parts:
    m = trimesh.load(os.path.join(parts_dir, f"{p}.stl"))
    # ensure sitting flat on Z=0
    m.apply_translation([0, 0, -m.bounds[0][2]])
    meshes[p] = m
    b = m.bounds
    print(f"{p:35s}: size = [{b[1][0]-b[0][0]:5.1f}, {b[1][1]-b[0][1]:5.1f}, {b[1][2]-b[0][2]:4.1f}]")

# Let's test a clean, non-colliding layout on 256x256 bed (X in [10, 246], Y in [10, 246])
# 01_Base_Chassi_Inferior (122 x 177) centered at (74, 128) -> X: [13, 135], Y: [39.5, 216.5]
# Above base (Y > 218): space is [10, 135] x [218, 246] -> 125 x 28 mm:
# - 17_Travessa (51.5 x 18.8) at (40, 232) -> X: [14.2, 65.8], Y: [222.6, 241.4]
# - 18_Travessa (51.4 x 18.8) at (100, 232) -> X: [74.3, 125.7], Y: [222.6, 241.4]
# Below base (Y < 38): space is [10, 135] x [10, 38] -> 125 x 28 mm:
# Right side: X in [142, 246] (width 104 mm), Y in [10, 246] (height 236 mm):
# Let's stack vertically on the right:
# - 02_Braco (114.8 x 49.8) rotated 90 -> (49.8 x 114.8) at (168, 180) -> X: [143.1, 192.9], Y: [122.6, 237.4]
# - 07_Braco (84.4 x 39.1) rotated 90 -> (39.1 x 84.4) at (222, 190) -> X: [202.4, 241.6], Y: [147.8, 232.2]
# - 12_Mesa (58.0 x 58.0) at (172, 85) -> X: [143, 201], Y: [56, 114]
# - 09_Torre (52.5 x 60.2) at (220, 85) -> wait, width 52.5 + 58 = 110.5 > 104!
# Let's write an optimizer or test candidate layouts!
