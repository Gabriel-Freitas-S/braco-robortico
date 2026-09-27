import trimesh
import numpy as np

berco = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
# Find all inner edges/faces
# The outer box is [-26.07, 26.07] in X, [-24.08, 24.08] in Y.
# Let's find vertices strictly inside: |X| < 25 and |Y| < 23
mask = (np.abs(berco.vertices[:, 0]) < 25) & (np.abs(berco.vertices[:, 1]) < 23)
inner_v = berco.vertices[mask]
print("Inner vertices range:")
print("X:", inner_v[:,0].min(), inner_v[:,0].max())
print("Y:", inner_v[:,1].min(), inner_v[:,1].max())

# Let's cluster or find holes in inner vertices
from sklearn.cluster import DBSCAN
clustering = DBSCAN(eps=2.0, min_samples=3).fit(inner_v)
for label in set(clustering.labels_):
    pts = inner_v[clustering.labels_ == label]
    print(f"Cluster {label}: count={len(pts)}, center={pts.mean(axis=0)}, bounds=[{pts.min(axis=0)}, {pts.max(axis=0)}]")
