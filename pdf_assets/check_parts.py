import json
import numpy as np

with open(r'd:\braco-robortico\preview_data.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

for p in data['parts']:
    if any(k in p['name'] for k in ['01_Base', '12_Mesa', '16_Berco', '25_Trava', '17_Travessa', '18_Travessa']):
        v = np.array(p['vertices_assembled']).reshape(-1, 3)
        print(f"{p['name']}:")
        print(f"  Z range: [{v[:,2].min():.2f}, {v[:,2].max():.2f}]")
        print(f"  X range: [{v[:,0].min():.2f}, {v[:,0].max():.2f}]")
        print(f"  Y range: [{v[:,1].min():.2f}, {v[:,1].max():.2f}]")
