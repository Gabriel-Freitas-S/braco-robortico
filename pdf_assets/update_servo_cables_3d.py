import trimesh
import numpy as np
import os
import json
from scipy.interpolate import splprep, splev

BASE_DIR = r"D:\braco-robortico"
GLB_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
STL_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")

print("1. Loading GLB scene...")
scene = trimesh.load(GLB_PATH)

# ==============================================================================
# PIN DEFINITIONS ON SHIELD JOYSTICK DUPLO (meARM.WDT)
# ==============================================================================
# The shield center is at X ≈ 10.9, Y ≈ 41.5
# Servo header center is at X = 10.9, Y = 56.5, Z = 31.14 to 34.14
pin_pitch = 2.54
servo_hdr_x = 10.9
servo_hdr_y = 56.5
shield_z_surface = 31.14

# Channels:
# Col 0 (D11 - Cotovelo / Elbow):  X = 10.9 - 1.5 * 2.54 = 7.09
# Col 1 (D10 - Ombro / Shoulder):  X = 10.9 - 0.5 * 2.54 = 9.63
# Col 2 (D9  - Base / Yaw):        X = 10.9 + 0.5 * 2.54 = 12.17
# Col 3 (D6  - Garra / Claw):      X = 10.9 + 1.5 * 2.54 = 14.71
channels = {
    'cotovelo': {'col': 0, 'x': 7.09,  'servo_exit': [25.0, 116.0, 52.0]},
    'ombro':    {'col': 1, 'x': 9.63,  'servo_exit': [-22.0, 116.0, 48.0]},
    'base':     {'col': 2, 'x': 12.17, 'servo_exit': [4.0, 108.0, 16.0]},
    'garra':    {'col': 3, 'x': 14.71, 'servo_exit': [8.0, 152.0, 82.0]},
}

# ==============================================================================
# 2. BUILD 4 DUPONT 3-PIN CONNECTOR PLUGS (ENCAIXES FÊMEA DUPONT)
# ==============================================================================
# Dupont 3-pin connector housing dimensions:
# Width (along X): 2.50 mm
# Length (along Y): 7.60 mm
# Height (along Z): 14.0 mm
# It slides down over the pins from Z = 32.5 to Z = 46.5 mm
plug_w = 2.45
plug_l = 7.60
plug_h = 13.0
plug_z_bottom = 32.5
plug_z_center = plug_z_bottom + plug_h / 2.0  # 39.0 mm
plug_z_top = plug_z_bottom + plug_h          # 45.5 mm

plug_color = np.array([22, 24, 28, 255], dtype=np.uint8)

dupont_plugs = []
for name, ch in channels.items():
    # Main housing box
    plug_box = trimesh.creation.box([plug_w, plug_l, plug_h])
    plug_box.apply_translation([ch['x'], servo_hdr_y, plug_z_center])
    
    # Bevel top chamfer for realistic strain relief
    plug_top_rim = trimesh.creation.box([plug_w - 0.4, plug_l - 0.4, 1.5])
    plug_top_rim.apply_translation([ch['x'], servo_hdr_y, plug_z_top + 0.75])
    
    plug_mesh = trimesh.util.concatenate([plug_box, plug_top_rim])
    plug_mesh.visual = trimesh.visual.color.ColorVisuals(
        mesh=plug_mesh,
        vertex_colors=np.tile(plug_color, (len(plug_mesh.vertices), 1))
    )
    dupont_plugs.append(plug_mesh)

combined_plugs = trimesh.util.concatenate(dupont_plugs)
scene.geometry['conectores_dupont_servos'] = combined_plugs
print(f"Created 4 Dupont servo plugs: {len(combined_plugs.vertices)} vertices")

# ==============================================================================
# 3. HELPER FUNCTION TO GENERATE 3-WIRE RIBBON CABLE
# ==============================================================================
def generate_ribbon_cable(control_points, n_steps=35, wire_radius=0.55, wire_spacing=1.05):
    control_points = np.array(control_points)
    tck, u = splprep([control_points[:, 0], control_points[:, 1], control_points[:, 2]], s=0, k=min(3, len(control_points)-1))
    u_fine = np.linspace(0, 1, n_steps)
    pts = np.column_stack(splev(u_fine, tck))
    
    # Compute tangents
    tangents = np.diff(pts, axis=0)
    tangents = np.vstack([tangents, tangents[-1]])
    norms = np.linalg.norm(tangents, axis=1)[:, np.newaxis]
    tangents /= np.where(norms == 0, 1.0, norms)
    
    # Binormals for wire offsets
    up = np.array([0, 0, 1.0])
    binormals = np.cross(tangents, up)
    b_norms = np.linalg.norm(binormals, axis=1)[:, np.newaxis]
    for i in range(len(b_norms)):
        if b_norms[i] < 1e-4:
            binormals[i] = [0, 1.0, 0]
        else:
            binormals[i] /= b_norms[i]
            
    # Standard JR/Futaba wire colors:
    # Strand 0 (GND): Brown
    # Strand 1 (5V):  Red
    # Strand 2 (PWM): Bright Orange
    strand_colors = [
        [121, 85, 72, 255],   # Brown (GND)
        [229, 57, 53, 255],   # Red (5V VCC)
        [255, 130, 0, 255]    # Orange (Signal PWM)
    ]
    
    strands = []
    offsets = [-wire_spacing, 0.0, wire_spacing]
    for s_idx, off in enumerate(offsets):
        s_pts = pts + binormals * off
        segs = []
        for i in range(len(s_pts) - 1):
            p1 = s_pts[i]
            p2 = s_pts[i+1]
            seg = trimesh.creation.cylinder(radius=wire_radius, segment=np.array([p1, p2]), sections=10)
            segs.append(seg)
        strand_mesh = trimesh.util.concatenate(segs)
        c = np.array(strand_colors[s_idx], dtype=np.uint8)
        strand_mesh.visual = trimesh.visual.color.ColorVisuals(
            mesh=strand_mesh, vertex_colors=np.tile(c, (len(strand_mesh.vertices), 1))
        )
        strands.append(strand_mesh)
        
    return trimesh.util.concatenate(strands)

# ==============================================================================
# 4. BUILD 4 REALISTIC SERVO RIBBON CABLES CONNECTED TO DUPONT PLUGS
# ==============================================================================
z_plug_exit = plug_z_top + 1.5  # ≈ 47.0 mm (top of strain relief)

# Cable 1: Servo Base (Yaw) -> Plug at X=12.17, Y=56.5
ctrl_base = [
    [12.17, 56.5, z_plug_exit],
    [12.17, 59.0, 56.0],        # Rises vertically from connector
    [10.0,  68.0, 42.0],        # Arches smoothly down
    [7.5,   82.0, 22.0],        # Enters the base clearance window
    [5.0,   96.0, 16.0],        # Routes under base turntable
    [4.0,  108.0, 16.0]         # Enters base servo SG90
]
cable_base = generate_ribbon_cable(ctrl_base, n_steps=32)
scene.geometry['cabo_servo_base'] = cable_base
print("Generated cabo_servo_base connected to Shield")

# Cable 2: Servo Ombro (Shoulder) -> Plug at X=9.63, Y=56.5
ctrl_ombro = [
    [9.63,  56.5, z_plug_exit],
    [9.63,  59.0, 58.0],        # Rises vertically from connector
    [4.0,   72.0, 62.0],        # Curves leftward and upward
    [-10.0, 92.0, 55.0],        # Climbs towards left tower
    [-18.0, 105.0, 50.0],       # Approaches left tower servo
    [-22.0, 116.0, 48.0]        # Enters shoulder servo SG90
]
cable_ombro = generate_ribbon_cable(ctrl_ombro, n_steps=34)
scene.geometry['cabo_servo_ombro'] = cable_ombro
print("Generated cabo_servo_ombro connected to Shield")

# Cable 3: Servo Cotovelo (Elbow) -> Plug at X=7.09, Y=56.5
ctrl_cotovelo = [
    [7.09,  56.5, z_plug_exit],
    [7.09,  59.0, 60.0],        # Rises vertically from connector
    [14.0,  74.0, 64.0],        # Curves rightward and upward
    [22.0,  94.0, 58.0],        # Climbs towards right tower
    [25.0, 105.0, 54.0],        # Approaches right tower servo
    [25.0, 116.0, 52.0]         # Enters elbow servo SG90
]
cable_cotovelo = generate_ribbon_cable(ctrl_cotovelo, n_steps=34)
scene.geometry['cabo_servo_cotovelo'] = cable_cotovelo
print("Generated cabo_servo_cotovelo connected to Shield")

# Cable 4: Servo Garra (Claw) -> Plug at X=14.71, Y=56.5
# Follows the upper spar/parallelogram high up to the claw end effector!
ctrl_garra = [
    [14.71, 56.5, z_plug_exit],
    [14.71, 59.0, 64.0],        # Rises vertically from connector
    [12.0,  76.0, 78.0],        # Arches high towards the arm boom
    [8.0,   98.0, 98.0],        # Runs along the main lifting spar
    [7.0,  122.0, 104.0],       # Reaches forearm pivot
    [7.5,  138.0, 96.0],        # Follows forearm linkage
    [8.0,  152.0, 82.0]         # Enters claw micro-servo SG90
]
cable_garra = generate_ribbon_cable(ctrl_garra, n_steps=40)
scene.geometry['cabo_servo_garra'] = cable_garra
print("Generated cabo_servo_garra connected to Shield")

# ==============================================================================
# 5. EXPORT SCENE AND COMBINED STL
# ==============================================================================
scene.export(GLB_PATH)
print(f"Updated GLB saved: {GLB_PATH}")

all_meshes = [g for g in scene.geometry.values() if isinstance(g, trimesh.Trimesh) and len(g.faces) > 0]
combined_stl = trimesh.util.concatenate(all_meshes)
combined_stl.export(STL_PATH)
print(f"Updated STL saved: {STL_PATH} ({len(combined_stl.faces)} faces)")

print("SUCCESS: 3D Cables and Dupont plugs fully connected to Shield Joystick Duplo!")
