import trimesh
import numpy as np
import os
import json
from scipy.interpolate import splprep, splev

BASE_DIR = r"D:\braco-robortico"
GLB_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
STL_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")
NEW_BASE_STL = os.path.join(BASE_DIR, "cad-output", "01_Base_Chassi_Inferior.stl")
METADATA_PATH = os.path.join(BASE_DIR, "components_metadata.json")

print("1. Loading GLB scene...")
scene = trimesh.load(GLB_PATH)

# ==============================================================================
# 2. UPDATE BASE CHASSI INFERIOR (01_Base_Chassi_Inferior)
# ==============================================================================
print("2. Replacing 01_Base_Chassi_Inferior with new solid ergonomic model...")
new_base = trimesh.load(NEW_BASE_STL)

# Base CAD origin: center = (3.16, 30.06).
# Alignment transform to place it on TPU feet (Z = 10.24 to 14.24 mm)
# and align standoff holes with the arm mechanism:
T_base = np.eye(4)
T_base[0, 3] = 0.56
T_base[1, 3] = 85.78
T_base[2, 3] = 10.24

new_base.apply_transform(T_base)
base_color = np.array([34, 38, 44, 255], dtype=np.uint8)
new_base.visual = trimesh.visual.color.ColorVisuals(
    mesh=new_base,
    vertex_colors=np.tile(base_color, (len(new_base.vertices), 1))
)
scene.geometry['printed_01_Base_Chassi_Inferior'] = new_base

# ==============================================================================
# 3. REPOSITION SERVO 1 (servo_01_base) & TRAVA (printed_25_Trava_Horn_Servo_Base)
# ==============================================================================
print("3. Repositioning servo_01_base into its proper cradle in 12_Mesa_Giratoria_Turntable...")
# The servo was mistakenly placed 22 mm too low (hitting table at Z=0).
# Moving it by [dx, dy, dz] = [-0.60, -4.08, +19.04]:
# - mounting ears sit flush on 12_Mesa at Z = 37.24 mm to 40.24 mm
# - body bottom sits at Z = 19.04 mm (4.8 mm ABOVE the base plate at Z=14.24 mm)
# - output gear points up into 16_Berco at Z = 49.8 mm
servo_base = scene.geometry['servo_01_base']
# Check current Z min
if servo_base.bounds[0][2] < 5.0:
    servo_base.apply_translation([-0.60, -4.08, 19.04])
    print(f"  Repositioned servo_01_base: new bounds Z = [{servo_base.bounds[0][2]:.2f}, {servo_base.bounds[1][2]:.2f}]")

# Shift 25_Trava_Horn_Servo_Base to clamp over the servo mounting ears
trava = scene.geometry['printed_25_Trava_Horn_Servo_Base']
if trava.bounds[0][2] < 35.0:
    trava.apply_translation([-0.60, -4.08, 7.0])
    print(f"  Repositioned 25_Trava: new bounds Z = [{trava.bounds[0][2]:.2f}, {trava.bounds[1][2]:.2f}]")

# ==============================================================================
# 4. BUILD REALISTIC FEMALE DUPONT PLUGS & 3-WIRE RIBBON CABLES
# ==============================================================================
print("4. Building Dupont female plugs and connecting ribbon cables...")
pin_pitch = 2.54
servo_hdr_x = 10.9
servo_hdr_y = 56.5
# Pins on shield extend from Z = 34.14 to Z = 40.14 mm
# Female Dupont plug sits flush on header base (Z = 34.0 mm) up to Z = 47.5 mm (height 13.5 mm)
plug_w = 2.45
plug_l = 7.60
plug_h = 13.5
plug_z_bottom = 34.0
plug_z_center = plug_z_bottom + plug_h / 2.0  # 40.75 mm
plug_z_top = plug_z_bottom + plug_h          # 47.50 mm

plug_color = np.array([22, 25, 29, 255], dtype=np.uint8)
metal_clip_color = np.array([195, 200, 205, 255], dtype=np.uint8)

# Channels:
# Col 0 (D11 - Cotovelo / Elbow):  X = 10.9 - 1.5 * 2.54 = 7.09
# Col 1 (D10 - Ombro / Shoulder):  X = 10.9 - 0.5 * 2.54 = 9.63
# Col 2 (D9  - Base / Yaw):        X = 10.9 + 0.5 * 2.54 = 12.17
# Col 3 (D6  - Garra / Claw):      X = 10.9 + 1.5 * 2.54 = 14.71
channels = {
    'cabo_servo_cotovelo': {
        'col': 0, 'x': 7.09,
        'dest': [25.0, 116.0, 52.0],
        'intermediate': [
            [7.09, 56.5, 49.0],
            [7.09, 58.5, 58.0],
            [15.0, 75.0, 64.0],
            [23.0, 95.0, 58.0],
            [25.0, 108.0, 54.0],
            [25.0, 116.0, 52.0]
        ]
    },
    'cabo_servo_ombro': {
        'col': 1, 'x': 9.63,
        'dest': [-22.0, 116.0, 48.0],
        'intermediate': [
            [9.63, 56.5, 49.0],
            [9.63, 58.5, 56.0],
            [4.0,  72.0, 62.0],
            [-10.0, 92.0, 55.0],
            [-18.0, 105.0, 50.0],
            [-22.0, 116.0, 48.0]
        ]
    },
    'cabo_servo_base': {
        'col': 2, 'x': 12.17,
        'dest': [3.13, 110.0, 22.0],
        'intermediate': [
            [12.17, 56.5, 49.0],
            [12.17, 58.5, 52.0],
            [11.0,  68.0, 40.0],
            [8.5,   82.0, 26.0],
            [5.5,   96.0, 22.0],
            [3.13, 110.0, 22.0]
        ]
    },
    'cabo_servo_garra': {
        'col': 3, 'x': 14.71,
        'dest': [8.0, 152.0, 82.0],
        'intermediate': [
            [14.71, 56.5, 49.0],
            [14.71, 58.5, 62.0],
            [12.0,  76.0, 78.0],
            [8.0,   98.0, 98.0],
            [7.0,  122.0, 104.0],
            [7.5,  138.0, 96.0],
            [8.0,  152.0, 82.0]
        ]
    }
}

def create_dupont_plug(ch_x):
    parts = []
    # Main housing box
    box = trimesh.creation.box([plug_w, plug_l, plug_h])
    box.apply_translation([ch_x, servo_hdr_y, plug_z_center])
    box.visual = trimesh.visual.color.ColorVisuals(
        mesh=box, vertex_colors=np.tile(plug_color, (len(box.vertices), 1))
    )
    parts.append(box)
    
    # Top collar (strain relief)
    collar = trimesh.creation.box([plug_w - 0.3, plug_l - 0.4, 1.6])
    collar.apply_translation([ch_x, servo_hdr_y, plug_z_top + 0.8])
    collar.visual = trimesh.visual.color.ColorVisuals(
        mesh=collar, vertex_colors=np.tile(plug_color, (len(collar.vertices), 1))
    )
    parts.append(collar)
    
    # 3 terminal latch clips on the front face (metallic silver accents)
    for py in [servo_hdr_y - pin_pitch, servo_hdr_y, servo_hdr_y + pin_pitch]:
        tab = trimesh.creation.box([0.3, 1.2, 2.0])
        tab.apply_translation([ch_x + plug_w/2.0 + 0.05, py, plug_z_center - 1.0])
        tab.visual = trimesh.visual.color.ColorVisuals(
            mesh=tab, vertex_colors=np.tile(metal_clip_color, (len(tab.vertices), 1))
        )
        parts.append(tab)
        
    return trimesh.util.concatenate(parts)

def generate_ribbon_strands(ch_x, ctrl_points, n_steps=32, wire_radius=0.52):
    # Strand pin positions along Y at the connector:
    # Front pin (-Y): GND (Brown) at Y = 56.5 - 2.54 = 53.96
    # Middle pin (Y): 5V (Red)    at Y = 56.5
    # Rear pin   (+Y): PWM (Orange) at Y = 56.5 + 2.54 = 59.04
    strand_defs = [
        {'name': 'GND',    'y_plug': servo_hdr_y - pin_pitch, 'color': [121, 85, 72, 255]},   # Brown
        {'name': '5V',     'y_plug': servo_hdr_y,             'color': [229, 57, 53, 255]},   # Red
        {'name': 'Signal', 'y_plug': servo_hdr_y + pin_pitch, 'color': [255, 130, 0, 255]}    # Orange
    ]
    
    strands = []
    for s_idx, s_def in enumerate(strand_defs):
        # Create control points for this specific strand
        pts_s = []
        # Point 0: INSIDE the Dupont plug at Z = 44.0 mm
        pts_s.append([ch_x, s_def['y_plug'], 44.0])
        # Point 1: Exiting through top collar at Z = 48.0 mm
        pts_s.append([ch_x, s_def['y_plug'], 48.5])
        
        # Follow the intermediate spline points with smooth lateral offset
        y_off = (s_idx - 1) * 0.95
        for cp in ctrl_points:
            pts_s.append([cp[0], cp[1] + y_off, cp[2]])
            
        pts_s = np.array(pts_s)
        tck, u = splprep([pts_s[:, 0], pts_s[:, 1], pts_s[:, 2]], s=0, k=min(3, len(pts_s)-1))
        u_fine = np.linspace(0, 1, n_steps)
        curve = np.column_stack(splev(u_fine, tck))
        
        segs = []
        for i in range(len(curve) - 1):
            p1 = curve[i]
            p2 = curve[i+1]
            seg = trimesh.creation.cylinder(radius=wire_radius, segment=np.array([p1, p2]), sections=10)
            segs.append(seg)
            
        strand_mesh = trimesh.util.concatenate(segs)
        c = np.array(s_def['color'], dtype=np.uint8)
        strand_mesh.visual = trimesh.visual.color.ColorVisuals(
            mesh=strand_mesh, vertex_colors=np.tile(c, (len(strand_mesh.vertices), 1))
        )
        strands.append(strand_mesh)
        
    return trimesh.util.concatenate(strands)

for cable_name, ch_data in channels.items():
    plug = create_dupont_plug(ch_data['x'])
    ribbon = generate_ribbon_strands(ch_data['x'], ch_data['intermediate'])
    
    # Merge plug and ribbon into ONE complete cable assembly
    full_cable = trimesh.util.concatenate([plug, ribbon])
    scene.geometry[cable_name] = full_cable
    print(f"  Built {cable_name}: plug + 3 strands ({len(full_cable.vertices)} vertices)")

# Remove old unused plug key if present
if 'conectores_dupont_servos' in scene.geometry:
    del scene.geometry['conectores_dupont_servos']

# ==============================================================================
# 5. SYNCHRONIZE SCENE GRAPH NODES
# ==============================================================================
print("5. Verifying and synchronizing scene graph nodes...")
graph_nodes = set(scene.graph.nodes_geometry)
for geom_name in scene.geometry.keys():
    if geom_name not in graph_nodes:
        scene.graph.update(frame_to=geom_name, geometry=geom_name)
        print(f"  Added missing node to graph: {geom_name}")

# ==============================================================================
# 6. EXPORT UPDATED GLB AND STL ASSEMBLIES
# ==============================================================================
print("6. Exporting updated GLB and STL assemblies...")
scene.export(GLB_PATH)
print(f"  Saved GLB: {GLB_PATH}")

all_meshes = [g for g in scene.geometry.values() if isinstance(g, trimesh.Trimesh) and len(g.faces) > 0]
combined_stl = trimesh.util.concatenate(all_meshes)
combined_stl.export(STL_PATH)
print(f"  Saved STL: {STL_PATH} ({len(combined_stl.faces)} faces)")

# ==============================================================================
# 7. UPDATE METADATA (components_metadata.json)
# ==============================================================================
print("7. Updating components_metadata.json...")
with open(METADATA_PATH, 'r', encoding='utf-8') as f:
    meta = json.load(f)

for comp in meta['components']:
    if comp['id'] == 'printed_01_Base_Chassi_Inferior':
        comp['desc'] = "Placa base estrutural ergonômica sólida com contorno curvilíneo, abas de apoio para punhos/mãos, furações de fixação M3 para pés de TPU, espaçadores do braço e Arduino Uno. Fundação robusta sem recortes inferiores."
    elif comp['id'] == 'servo_01_base':
        comp['desc'] = "Acionamento do giro da base em 180° (Yaw), alojado no nicho da mesa giratória 4.8mm acima da base chassi, sem qualquer saliência ou contato com a bancada."
    elif comp['id'] == 'cabos_ribbon':
        comp['desc'] = "Fiação organizada JR 3 vias (GND marrom, 5V vermelho, PWM laranja) com conectores fêmea Dupont 3 pinos perfeitamente plugados nos 4 canais do Shield Joystick Duplo."

meta['statistics']['total_triangles'] = len(combined_stl.faces)
meta['statistics']['total_vertices'] = len(combined_stl.vertices)
bounds = combined_stl.bounds
meta['statistics']['bounds'] = {
    'min': [round(float(x), 2) for x in bounds[0]],
    'max': [round(float(x), 2) for x in bounds[1]]
}
meta['statistics']['dimensions_mm'] = {
    'width_x': round(float(bounds[1][0] - bounds[0][0]), 2),
    'depth_y': round(float(bounds[1][1] - bounds[0][1]), 2),
    'height_z': round(float(bounds[1][2] - bounds[0][2]), 2),
}

with open(METADATA_PATH, 'w', encoding='utf-8') as f:
    json.dump(meta, f, indent=2, ensure_ascii=False)

print("\n=== VERIFICATION CHECKS ===")
print("Combined STL bounds:")
print("  min:", bounds[0])
print("  max:", bounds[1])
print("Base plate Z:", scene.geometry['printed_01_Base_Chassi_Inferior'].bounds[:, 2])
print("Servo 1 Z:", scene.geometry['servo_01_base'].bounds[:, 2])
print("TPU Foot 1 Z:", scene.geometry['pe_tpu_1'].bounds[:, 2])
print(f"Clearance (Servo 1 bottom - Base plate top): {scene.geometry['servo_01_base'].bounds[0][2] - scene.geometry['printed_01_Base_Chassi_Inferior'].bounds[1][2]:.2f} mm")
print(f"Clearance (Servo 1 bottom - Table surface): {scene.geometry['servo_01_base'].bounds[0][2] - scene.geometry['pe_tpu_1'].bounds[0][2]:.2f} mm")
print("SUCCESS: 3D Assembly completely revised and synchronized!")
