import trimesh
import numpy as np

# Load the original STL files
berco = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
mesa = trimesh.load('pecas_impressao_3d/12_Mesa_Giratoria_Turntable.stl')
trava = trimesh.load('pecas_impressao_3d/25_Trava_Horn_Servo_Base.stl')

print("16_Berco vertices shape:", berco.vertices.shape)
print("16_Berco bounds:", berco.bounds)

# Let's inspect the original positions in preview_data.json
import json
with open('preview_data.json', 'r') as f:
    data = json.load(f)

for p in data['parts']:
    if p['name'] in ['01_Base_Chassi_Inferior', '12_Mesa_Giratoria_Turntable', '14_Separador_Espacador_Central_Base', '16_Berco_Servo_Rotacao_Base', '17_Travessa_Reforco_Base_Frontal', '18_Travessa_Reforco_Base_Traseira', '25_Trava_Horn_Servo_Base']:
        v = np.array(p['vertices_assembled']).reshape(-1, 3)
        print(f"Part {p['name']}:")
        print(f"  Center: {v.mean(axis=0)}")
        print(f"  Bounds: min={v.min(axis=0)}, max={v.max(axis=0)}")
