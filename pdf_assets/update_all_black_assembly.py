import trimesh
import numpy as np
import json
import os

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
GLB_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
STL_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")
PREVIEW_DATA = os.path.join(BASE_DIR, "preview_data.json")
METADATA_PATH = os.path.join(BASE_DIR, "components_metadata.json")

print("1. Loading original GLB scene...")
scene = trimesh.load(GLB_PATH)

print("2. Loading preview_data.json for transformation matrices...")
with open(PREVIEW_DATA, 'r', encoding='utf-8') as f:
    pv = json.load(f)

# Compute rigid transforms (R, t) for all parts using Kabsch algorithm
part_transforms = {}
for p in pv['parts']:
    name = p['name']
    v_flat = np.array(p['vertices_flat']).reshape(-1, 3)
    v_asmb = np.array(p['vertices_assembled']).reshape(-1, 3)
    
    c_flat = v_flat.mean(axis=0)
    c_asmb = v_asmb.mean(axis=0)
    
    H = (v_flat - c_flat).T @ (v_asmb - c_asmb)
    U, S, Vt = np.linalg.svd(H)
    R = Vt.T @ U.T
    if np.linalg.det(R) < 0:
        Vt[2, :] *= -1
        R = Vt.T @ U.T
    t = c_asmb - R @ c_flat
    
    T = np.eye(4)
    T[:3, :3] = R
    T[:3, 3] = t
    part_transforms[name] = T

# Color: Technical Black (#22262C)
BLACK_COLOR = np.array([34, 38, 44, 255], dtype=np.uint8)

print("3. Replacing all 28 printed parts with debossed STLs in Black PLA...")
for i in range(1, 29):
    # Find matching geom key
    matching_keys = [k for k in scene.geometry.keys() if f'_{i:02d}_' in k and k.startswith('printed_')]
    if not matching_keys:
        print(f"Warning: could not find geom for part {i}")
        continue
    geom_key = matching_keys[0]
    part_name = geom_key.replace("printed_", "")
    stl_path = os.path.join(PARTS_DIR, f"{part_name}.stl")
    
    if not os.path.exists(stl_path):
        print(f"Warning: STL not found: {stl_path}")
        continue
        
    mesh = trimesh.load(stl_path)
    
    if i == 1:
        # 01_Base_Chassi_Inferior uses the updated alignment transform
        T_base = np.eye(4)
        T_base[0, 3] = 0.56
        T_base[1, 3] = 85.78
        T_base[2, 3] = 10.24
        mesh.apply_transform(T_base)
    else:
        T = part_transforms[part_name]
        mesh.apply_transform(T)
        
    mesh.visual = trimesh.visual.color.ColorVisuals(
        mesh=mesh,
        vertex_colors=np.tile(BLACK_COLOR, (len(mesh.vertices), 1))
    )
    
    scene.geometry[geom_key] = mesh
    print(f"  [x] Updated {geom_key} ({len(mesh.faces)} faces) -> Technical Black")

# Ensure scene graph has all nodes
graph_nodes = set(scene.graph.nodes_geometry)
for geom_name in scene.geometry.keys():
    if geom_name not in graph_nodes:
        scene.graph.update(frame_to=geom_name, geometry=geom_name)

print("4. Exporting updated GLB and STL assemblies...")
scene.export(GLB_PATH)
print(f"  Saved GLB: {GLB_PATH}")

all_meshes = [g for g in scene.geometry.values() if isinstance(g, trimesh.Trimesh) and len(g.faces) > 0]
combined_stl = trimesh.util.concatenate(all_meshes)
combined_stl.export(STL_PATH)
print(f"  Saved STL: {STL_PATH} ({len(combined_stl.faces)} faces)")

# Update metadata statistics
with open(METADATA_PATH, 'r', encoding='utf-8') as f:
    meta = json.load(f)

meta['statistics']['total_triangles'] = len(combined_stl.faces)
meta['statistics']['total_vertices'] = len(combined_stl.vertices)
bounds = combined_stl.bounds
meta['statistics']['bounds'] = {
    'min': [round(float(x), 2) for x in bounds[0]],
    'max': [round(float(x), 2) for x in bounds[1]]
}
meta['statistics']['dimensions_mm'] = {
    'width_x': round(float(bounds[1][0] - bounds[0][0]), 2),
    'depth_y': round(float(bounds[1][1] - bounds[0][1]), 2),
    'height_z': round(float(bounds[1][2] - bounds[0][2]), 2),
}

with open(METADATA_PATH, 'w', encoding='utf-8') as f:
    json.dump(meta, f, indent=2, ensure_ascii=False)

print("\nSUCCESS: Arm assembly completely updated with all 28 debossed STLs in 100% Black PLA!")
