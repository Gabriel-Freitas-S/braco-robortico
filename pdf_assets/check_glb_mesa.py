import trimesh
import numpy as np

s = trimesh.load('braco_completo_com_componentes.glb')
mesa = s.geometry['printed_12_Mesa_Giratoria_Turntable']
print("Mesa bounds:", mesa.bounds)

slice_2d = mesa.section(plane_origin=[0, 0, 38.74], plane_normal=[0, 0, 1])
for i, entity in enumerate(slice_2d.entities):
    pts = slice_2d.vertices[entity.points]
    print(f"Entity {i}: center={pts.mean(axis=0)[:2].round(2)}, bounds min={pts.min(axis=0)[:2].round(2)}, max={pts.max(axis=0)[:2].round(2)}")
