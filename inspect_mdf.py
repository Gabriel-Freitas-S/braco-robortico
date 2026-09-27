import trimesh
import numpy as np

scene = trimesh.load('ref_mdf/extracted/Aula 21 - Braço Robótico.dae')
print("Scene loaded!")

# Let's inspect the parts in SG90ServoMotor_-_GearSet-1_GearTop-1_018-mesh
# We want to find the connected components of the wooden parts (Madeira)
braco_meshes = [v for k, v in scene.geometry.items() if 'SG90ServoMotor' in k]

# Combine all submeshes of Braco into one scene and separate by connected components:
full_braco = trimesh.util.concatenate(braco_meshes)
parts = full_braco.split(only_watertight=False)
print(f"Total separate solid parts in MDF arm: {len(parts)}")

# Sort by volume or bounding box size
parts = sorted(parts, key=lambda p: p.bounding_box.volume, reverse=True)
for i, p in enumerate(parts[:25]):
    ext = p.bounding_box.extents * 1000.0  # in mm
    cen = p.bounding_box.centroid * 1000.0  # in mm
    print(f"Part {i:02d}: Extents={ext[0]:.1f} x {ext[1]:.1f} x {ext[2]:.1f} mm | Centroid=({cen[0]:.1f}, {cen[1]:.1f}, {cen[2]:.1f})")
