import json
import trimesh
import numpy as np
import os

parts_dir = r"d:\braco-robortico\pecas_impressao_3d"
new_base = trimesh.load(os.path.join(parts_dir, "01_Base_Chassi_Inferior.stl"))
print("New Base bounds:", new_base.bounds)
print("New Base size:", new_base.bounds[1] - new_base.bounds[0])

# Bed is 256 x 256 mm: X in [0, 256], Y in [0, 256] (or centered [-128, 128])
# New base size: 122 mm in X, 177.1 mm in Y, 4 mm in Z
