import trimesh
import numpy as np

berco = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
# In preview_data.json, where was 16_Berco?
import json
with open('preview_data.json', 'r') as f:
    data = json.load(f)

for p in data['parts']:
    if p['name'] == '16_Berco_Servo_Rotacao_Base':
        v = np.array(p['vertices_assembled']).reshape(-1, 3)
        print("Assembled 16_Berco:")
        print("X range:", v[:,0].min(), v[:,0].max())
        print("Y range:", v[:,1].min(), v[:,1].max())
        print("Z range:", v[:,2].min(), v[:,2].max())
        
        # Let's find the 4 corner holes in 16_Berco:
        # Corner 1: min X, min Y
        # Corner 2: max X, min Y
        # Corner 3: min X, max Y
        # Corner 4: max X, max Y
        mask = (v[:, 2] > 9.0) & (v[:, 2] < 10.0)
        v_mid = v[mask]
        print("Corners of 16_Berco:")
        print("FL:", v_mid[(v_mid[:,0] < -15) & (v_mid[:,1] < 100)].mean(axis=0))
        print("FR:", v_mid[(v_mid[:,0] > 20) & (v_mid[:,1] < 100)].mean(axis=0))
        print("RL:", v_mid[(v_mid[:,0] < -15) & (v_mid[:,1] > 135)].mean(axis=0))
        print("RR:", v_mid[(v_mid[:,0] > 20) & (v_mid[:,1] > 135)].mean(axis=0))
