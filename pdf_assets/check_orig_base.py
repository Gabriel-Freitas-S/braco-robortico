import subprocess, trimesh, io, numpy as np
from sklearn.cluster import DBSCAN

data = subprocess.check_output(['git', 'show', 'HEAD:pecas_impressao_3d/01_Base_Chassi_Inferior.stl'])
mesh = trimesh.load(io.BytesIO(data), file_type='stl')
# Find all vertices that are interior (not on the outer rectangular border)
# Outer border is |X| ~ 47.5, |Y| ~ 72.5
mask = (np.abs(mesh.vertices[:, 0]) < 45) & (np.abs(mesh.vertices[:, 1]) < 70)
inner_v = mesh.vertices[mask]
print(f"Total inner vertices: {len(inner_v)}")

clustering = DBSCAN(eps=2.5, min_samples=3).fit(inner_v)
for label in sorted(set(clustering.labels_)):
    pts = inner_v[clustering.labels_ == label]
    c = pts.mean(axis=0)
    dx = pts[:,0].max() - pts[:,0].min()
    dy = pts[:,1].max() - pts[:,1].min()
    print(f"Feature {label:2d}: count={len(pts):3d}, center=({c[0]:6.2f}, {c[1]:6.2f}), size=({dx:5.2f}, {dy:5.2f})")
