import trimesh
import numpy as np

berco = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
# Inspect the faces/triangles of berco
print("Bounding box:", berco.bounds)
# Find bounding box of internal cutouts:
# Let's inspect the vertices
v = berco.vertices
print("X range:", v[:,0].min(), v[:,0].max())
print("Y range:", v[:,1].min(), v[:,1].max())
print("Z range:", v[:,2].min(), v[:,2].max())

# Let's check how many connected components or holes
# Find faces whose normals point along X, Y, or Z
print("Face normals counts:")
for ax, name in enumerate(['X', 'Y', 'Z']):
    print(f"Normal +{name}:", np.sum(berco.face_normals[:, ax] > 0.9))
    print(f"Normal -{name}:", np.sum(berco.face_normals[:, ax] < -0.9))
