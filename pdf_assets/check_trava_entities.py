import trimesh
import numpy as np

trava = trimesh.load('pecas_impressao_3d/25_Trava_Horn_Servo_Base.stl')
print("Trava bounds:", trava.bounds)
slice_2d = trava.section(plane_origin=[0, 0, 1.5], plane_normal=[0, 0, 1])
for i, entity in enumerate(slice_2d.entities):
    pts = slice_2d.vertices[entity.points]
    print(f"Trava Entity {i}: {len(pts)} pts, center={pts.mean(axis=0)[:2].round(2)}, bounds min={pts.min(axis=0)[:2].round(2)}, max={pts.max(axis=0)[:2].round(2)}")
