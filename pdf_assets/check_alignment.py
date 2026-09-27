import trimesh
import numpy as np

s = trimesh.load('braco_completo_com_componentes.glb')
mesa = s.geometry['printed_12_Mesa_Giratoria_Turntable']
servo = s.geometry['servo_01_base'].copy()

# Apply translation
servo.apply_translation([-0.60, -4.08, 19.04])
print("New servo bounds:")
print("  min:", servo.bounds[0])
print("  max:", servo.bounds[1])

# Check intersection with mesa
# The body of the servo should pass through the niche in mesa:
# Niche is X in [-2.87, 9.13], Y in [109.8, 132.8], Z in [37.24, 40.24]
print("Mesa Z range:", mesa.bounds[0][2], mesa.bounds[1][2])
body_in_mesa_z = (servo.vertices[:, 2] > 37.24) & (servo.vertices[:, 2] < 40.24)
pts = servo.vertices[body_in_mesa_z]
print("Servo vertices in Mesa Z range:")
print("  X range:", pts[:, 0].min(), pts[:, 0].max())
print("  Y range:", pts[:, 1].min(), pts[:, 1].max())

# Mounting ears:
ears_pts = servo.vertices[(servo.vertices[:, 2] > 37.0) & (servo.vertices[:, 2] < 41.5)]
print("Mounting ears X range:", ears_pts[:, 0].min(), ears_pts[:, 0].max())
