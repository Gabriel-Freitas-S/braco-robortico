import json
import numpy as np

with open('preview_data.json', 'r') as f:
    data = json.load(f)

for p in data['parts']:
    v = np.array(p['vertices_assembled']).reshape(-1, 3)
    print(f"{p['name']:40s} Z: [{v[:,2].min():6.2f}, {v[:,2].max():6.2f}]")
