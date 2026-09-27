import trimesh
import numpy as np

berco = trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl')
print("Berco bounds:", berco.bounds)
print("Berco dimensions:", berco.bounds[1] - berco.bounds[0])

# Inspect 14_Separador_Espacador_Central_Base.stl
sep = trimesh.load('pecas_impressao_3d/14_Separador_Espacador_Central_Base.stl')
print("Separador bounds:", sep.bounds)
print("Separador dimensions:", sep.bounds[1] - sep.bounds[0])

# Inspect 25_Trava_Horn_Servo_Base.stl
trava = trimesh.load('pecas_impressao_3d/25_Trava_Horn_Servo_Base.stl')
print("Trava bounds:", trava.bounds)
print("Trava dimensions:", trava.bounds[1] - trava.bounds[0])

# Inspect 12_Mesa_Giratoria_Turntable.stl
mesa = trimesh.load('pecas_impressao_3d/12_Mesa_Giratoria_Turntable.stl')
print("Mesa bounds:", mesa.bounds)
print("Mesa dimensions:", mesa.bounds[1] - mesa.bounds[0])
