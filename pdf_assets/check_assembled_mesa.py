import trimesh
import json
import numpy as np

with open('preview_data.json', 'r') as f:
    data = json.load(f)

for p in data['parts']:
    if p['name'] == '12_Mesa_Giratoria_Turntable':
        v = np.array(p['vertices_assembled']).reshape(-1, 3)
        faces = np.array(p['faces']).reshape(-1, 3)
        m = trimesh.Trimesh(vertices=v, faces=faces)
        slice_2d = m.section(plane_origin=[0, 0, 0.0], plane_normal=[0, 0, 1])
        print("12_Mesa in assembled space:")
        for i, entity in enumerate(slice_2d.entities):
            pts = slice_2d.vertices[entity.points]
            print(f"Entity {i}: center={pts.mean(axis=0)[:2].round(2)}, bounds min={pts.min(axis=0)[:2].round(2)}, max={pts.max(axis=0)[:2].round(2)}")
