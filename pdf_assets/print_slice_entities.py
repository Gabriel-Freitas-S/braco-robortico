import trimesh
import numpy as np

m = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
slice_2d = m.section(plane_origin=[0, 0, 1.5], plane_normal=[0, 0, 1])
for i, entity in enumerate(slice_2d.entities):
    pts = slice_2d.vertices[entity.points]
    print(f"Entity {i}: {len(pts)} pts, bounds min={pts.min(axis=0)[:2]}, max={pts.max(axis=0)[:2]}, center={pts.mean(axis=0)[:2]}")
