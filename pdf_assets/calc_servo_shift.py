import trimesh
import numpy as np

s = trimesh.load('braco_completo_com_componentes.glb')
mesa = s.geometry['printed_12_Mesa_Giratoria_Turntable']
servo = s.geometry['servo_01_base']

print("Current servo bounds:")
print("  min:", servo.bounds[0])
print("  max:", servo.bounds[1])
print("  center:", servo.bounds.mean(axis=0))

# Niche in mesa is at X in [-2.87, 9.13], Y in [109.8, 132.8], Z in [37.24, 40.24]
# Niche center: X = 3.13, Y = 121.30
# Mesa bottom Z = 37.24, top Z = 40.24

# Let's inspect servo vertices to see where the mounting ears and gear are
zs = servo.vertices[:, 2]
print("Servo Z min (body bottom):", zs.min())
print("Servo Z max (gear tip):", zs.max())

# In create_sg90_servo:
# Flange is 18.2 mm above body bottom (i.e. at zs.min() + 18.2)
# If flange sits at Z = 37.24 (on bottom of mesa) or Z = 40.24 (on top of mesa):
# Let's see how much Z shift is needed:
# If currently zs.min() = 0.0, the flange is at Z = 18.2 mm.
# If flange sits on mesa at Z = 37.24 mm:
# dz = 37.24 - 18.2 = +19.04 mm!
# Then:
# New body bottom = 0.0 + 19.04 = 19.04 mm!
# New gear tip = 35.85 + 19.04 = 54.89 mm!
# And Base plate top is at Z = 14.24 mm!
# Distance from base plate to servo bottom = 19.04 - 14.24 = 4.80 mm!
print("Calculated clearance to base plate:", 19.04 - 14.24, "mm")
