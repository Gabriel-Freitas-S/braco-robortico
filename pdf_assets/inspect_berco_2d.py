import trimesh
import numpy as np

m = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
print("Faces:", len(m.faces))
print("Vertices:", len(m.vertices))
print("Bounds:", m.bounds)

# Find all edges and loops
lines = []
for face in m.faces:
    for i in range(3):
        p1 = m.vertices[face[i]]
        p2 = m.vertices[face[(i+1)%3]]
        # check if horizontal
        if abs(p1[2] - p2[2]) < 1e-3 and p1[2] < 1.0: # bottom face
            lines.append((p1[:2], p2[:2]))

print("Bottom face lines count:", len(lines))

# Let's inspect the 2D contour using shapely or simple projection
from shapely.geometry import Polygon, MultiPolygon
from shapely.ops import polygonize, unary_union

# Let's project bottom vertices to 2D
slice_2d = m.section(plane_origin=[0, 0, 1.5], plane_normal=[0, 0, 1])
if slice_2d:
    for entity in slice_2d.entities:
        print("Entity type:", type(entity).__name__, "points:", len(entity.points))
    # print vertices of the section
    pts = slice_2d.vertices
    print(f"Section vertices count: {len(pts)}, bounds: min={pts.min(axis=0)}, max={pts.max(axis=0)}")
