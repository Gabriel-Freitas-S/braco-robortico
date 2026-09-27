import trimesh
import numpy as np
import json
import os

BASE_DIR = r"D:\braco-robortico"
GLB_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
STL_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")
NEW_BASE_STL = os.path.join(BASE_DIR, "cad-output", "01_Base_Chassi_Inferior.stl")
METADATA_PATH = os.path.join(BASE_DIR, "components_metadata.json")

print("1. Loading original GLB scene...")
scene = trimesh.load(GLB_PATH)

# ==============================================================================
# 2. UPDATE BASE CHASSI INFERIOR (01_Base_Chassi_Inferior)
# ==============================================================================
print("2. Replacing 01_Base_Chassi_Inferior with new ergonomic model...")
new_base = trimesh.load(NEW_BASE_STL)

# Base CAD origin: cutout_center = (3.16, 30.06).
# In GLB scene:
# espacador_base_1 is at [-21.27, 91.03, 7.84]
# CAD arm_hole_fl is at [-22.11, 5.08, 0.0]
# Translation to align arm standoffs and Z floor:
T_base = np.eye(4)
T_base[0, 3] = 0.56     # -21.27 - (-22.11) + 0.84 avg alignment
T_base[1, 3] = 85.78    # 91.03 - 5.08 = 85.95 avg
T_base[2, 3] = 10.24    # Sits directly on TPU feet (Z = 10.24 to 14.24 mm)

new_base.apply_transform(T_base)
# Color: Technical Black (#22262C)
base_color = np.array([34, 38, 44, 255], dtype=np.uint8)
new_base.visual = trimesh.visual.color.ColorVisuals(
    mesh=new_base,
    vertex_colors=np.tile(base_color, (len(new_base.vertices), 1))
)
scene.geometry['printed_01_Base_Chassi_Inferior'] = new_base

# ==============================================================================
# 3. REPOSITION THE 4 TPU FEET TO MATCH NEW TPU HOLES
# ==============================================================================
print("3. Repositioning 4 TPU feet...")
# In base CAD:
# tpu_fl = (-40.0, -75.0) -> GLB: (-40.0 + 0.56, -75.0 + 85.78) = (-39.44, 10.78)
# tpu_fr = (46.0, -75.0)  -> GLB: (46.0 + 0.56, -75.0 + 85.78)  = (46.56, 10.78)
# tpu_rl = (-34.0, 64.0)  -> GLB: (-34.0 + 0.56, 64.0 + 85.78)  = (-33.44, 149.78)
# tpu_rr = (40.0, 64.0)   -> GLB: (40.0 + 0.56, 64.0 + 85.78)   = (40.56, 149.78)
tpu_targets = {
    'pe_tpu_1': [-39.44, 10.78, 7.24],
    'pe_tpu_2': [46.56, 10.78, 7.24],
    'pe_tpu_3': [-33.44, 149.78, 7.24],
    'pe_tpu_4': [40.56, 149.78, 7.24],
}

for name, target_center in tpu_targets.items():
    if name in scene.geometry:
        foot = scene.geometry[name]
        current_center = foot.bounds.mean(axis=0)
        delta = np.array(target_center) - current_center
        foot.apply_translation(delta)

# ==============================================================================
# 4. BUILD ARDUINO UNO + SHIELD JOYSTICK DUPLO (meARM.WDT)
# ==============================================================================
print("4. Building Arduino Uno R3 + Shield Joystick Duplo geometry...")
# Extract the base Uno components from old electronics (brass standoffs, PCB, USB, DC jack, ATmega, header sockets)
old_elec = scene.geometry['eletronica_arduino_shield']
old_parts = old_elec.split()

# Identify Uno components by bounding box & color:
# Brass standoffs: Z from ~13 to 20
# Uno PCB: Z from 19.25 to 20.85
# USB metal: X from -26.5 to -14.5
# DC jack: X from -26.5 to -12.5
# ATmega: X from -2 to 33, Z ~ 20.8 to 24.3
# Header sockets: Z ~ 20.8 to 29.3

uno_submeshes = []

# Move standoffs to match new base plate thickness (Z = 14.24 to 20.24 mm)
# And shift brass standoffs to align with exact Arduino Uno holes
uno_holes_glb = [
    [-15.47, 17.31],  # uno_h1 (front-left)
    [-14.30, 65.59],  # uno_h2 (rear-left)
    [36.80, 50.88],   # uno_h3 (rear-right)
    [36.88, 22.17]    # uno_h4 (front-right)
]

for idx, (hx, hy) in enumerate(uno_holes_glb):
    standoff = trimesh.creation.cylinder(radius=2.5, height=6.0, sections=16)
    standoff.apply_translation([hx, hy, 14.24 + 3.0])
    standoff_color = np.array([200, 160, 40, 255], dtype=np.uint8)
    standoff.visual = trimesh.visual.color.ColorVisuals(
        mesh=standoff,
        vertex_colors=np.tile(standoff_color, (len(standoff.vertices), 1))
    )
    uno_submeshes.append(standoff)

# Arduino Uno PCB (68.6 x 53.4 x 1.6 mm) sitting on top of standoffs at Z = 20.24
# Uno center:
uno_center_x = (uno_holes_glb[0][0] + uno_holes_glb[2][0]) / 2.0 + 0.3  # ≈ 10.9 mm
uno_center_y = (uno_holes_glb[0][1] + uno_holes_glb[1][1]) / 2.0        # ≈ 41.5 mm
uno_pcb_z = 20.24 + 0.8  # Z center of 1.6mm board

uno_pcb = trimesh.creation.box([68.6, 53.4, 1.6])
uno_pcb.apply_translation([uno_center_x, uno_center_y, uno_pcb_z])
uno_blue = np.array([0, 120, 180, 255], dtype=np.uint8)
uno_pcb.visual = trimesh.visual.color.ColorVisuals(
    mesh=uno_pcb, vertex_colors=np.tile(uno_blue, (len(uno_pcb.vertices), 1))
)
uno_submeshes.append(uno_pcb)

# USB-B Connector (Silver metal box at left edge)
usb_box = trimesh.creation.box([12.0, 12.0, 11.0])
usb_box.apply_translation([uno_center_x - 34.3 + 6.0, uno_center_y + 11.5, uno_pcb_z + 0.8 + 5.5])
usb_silver = np.array([215, 220, 225, 255], dtype=np.uint8)
usb_box.visual = trimesh.visual.color.ColorVisuals(
    mesh=usb_box, vertex_colors=np.tile(usb_silver, (len(usb_box.vertices), 1))
)
uno_submeshes.append(usb_box)

# DC Barrel Jack (Dark gray box at bottom-left edge)
dc_box = trimesh.creation.box([14.0, 9.0, 11.0])
dc_box.apply_translation([uno_center_x - 34.3 + 7.0, uno_center_y - 18.0, uno_pcb_z + 0.8 + 5.5])
dc_dark = np.array([25, 25, 25, 255], dtype=np.uint8)
dc_box.visual = trimesh.visual.color.ColorVisuals(
    mesh=dc_box, vertex_colors=np.tile(dc_dark, (len(dc_box.vertices), 1))
)
uno_submeshes.append(dc_box)

# ATmega328P Chip (black flat IC)
ic_box = trimesh.creation.box([35.0, 8.0, 3.5])
ic_box.apply_translation([uno_center_x + 5.0, uno_center_y - 8.0, uno_pcb_z + 0.8 + 1.75])
ic_dark = np.array([20, 20, 20, 255], dtype=np.uint8)
ic_box.visual = trimesh.visual.color.ColorVisuals(
    mesh=ic_box, vertex_colors=np.tile(ic_dark, (len(ic_box.vertices), 1))
)
uno_submeshes.append(ic_box)

# Female Header sockets (black plastic strips connecting Uno to Shield)
# Top and bottom female headers (Z from 21.04 to 29.54, height 8.5mm)
hdr_top = trimesh.creation.box([48.0, 2.6, 8.5])
hdr_top.apply_translation([uno_center_x + 4.0, uno_center_y + 24.5, 21.04 + 4.25])
hdr_top.visual = trimesh.visual.color.ColorVisuals(
    mesh=hdr_top, vertex_colors=np.tile(dc_dark, (len(hdr_top.vertices), 1))
)
uno_submeshes.append(hdr_top)

hdr_bot = trimesh.creation.box([48.0, 2.6, 8.5])
hdr_bot.apply_translation([uno_center_x + 4.0, uno_center_y - 24.5, 21.04 + 4.25])
hdr_bot.visual = trimesh.visual.color.ColorVisuals(
    mesh=hdr_bot, vertex_colors=np.tile(dc_dark, (len(hdr_bot.vertices), 1))
)
uno_submeshes.append(hdr_bot)

# ==============================================================================
# 5. SHIELD JOYSTICK DUPLO (meARM.WDT V2.10.8.22.S)
# ==============================================================================
# Sits on top of female headers at Z = 29.54 mm
shield_z_base = 29.54
shield_pcb_th = 1.6
shield_center_x = uno_center_x
shield_center_y = uno_center_y

# Shield PCB (65.1 x 53.8 x 1.6 mm) in Deep Blue
shield_pcb = trimesh.creation.box([65.1, 53.8, shield_pcb_th])
shield_pcb.apply_translation([shield_center_x, shield_center_y, shield_z_base + shield_pcb_th / 2.0])
shield_blue = np.array([12, 85, 195, 255], dtype=np.uint8)
shield_pcb.visual = trimesh.visual.color.ColorVisuals(
    mesh=shield_pcb, vertex_colors=np.tile(shield_blue, (len(shield_pcb.vertices), 1))
)
uno_submeshes.append(shield_pcb)

# Dual Joysticks: Left and Right
# Spacing: 34.0 mm center-to-center
# Y position: offset slightly toward front for comfortable ergonomics
joy_spacing = 34.0
joy_y = shield_center_y - 7.0
joy_x_left = shield_center_x - joy_spacing / 2.0   # ≈ -6.1 mm
joy_x_right = shield_center_x + joy_spacing / 2.0  # ≈ +27.9 mm

metal_gray = np.array([185, 190, 195, 255], dtype=np.uint8)
thumb_black = np.array([28, 30, 34, 255], dtype=np.uint8)

for jx in [joy_x_left, joy_x_right]:
    # Metal gimbal base box (16 x 16 x 11 mm)
    gimbal = trimesh.creation.box([16.0, 16.0, 11.0])
    gimbal.apply_translation([jx, joy_y, shield_z_base + shield_pcb_th + 5.5])
    gimbal.visual = trimesh.visual.color.ColorVisuals(
        mesh=gimbal, vertex_colors=np.tile(metal_gray, (len(gimbal.vertices), 1))
    )
    uno_submeshes.append(gimbal)

    # Articulated stem (Ø 7.0 mm x 5.0 mm)
    stem_z = shield_z_base + shield_pcb_th + 11.0
    stem = trimesh.creation.cylinder(radius=3.5, height=5.0, sections=24)
    stem.apply_translation([jx, joy_y, stem_z + 2.5])
    stem.visual = trimesh.visual.color.ColorVisuals(
        mesh=stem, vertex_colors=np.tile(thumb_black, (len(stem.vertices), 1))
    )
    uno_submeshes.append(stem)

    # Ergonomic Thumbstick cap (concave mushroom cap Ø 25.4 mm x 9.0 mm)
    cap_z = stem_z + 5.0
    # Base disc
    cap_base = trimesh.creation.cylinder(radius=12.7, height=6.0, sections=36)
    cap_base.apply_translation([jx, joy_y, cap_z + 3.0])
    # Top rim
    cap_rim = trimesh.creation.cylinder(radius=12.2, height=3.0, sections=36)
    cap_rim.apply_translation([jx, joy_y, cap_z + 6.0 + 1.5])
    
    cap = trimesh.util.concatenate([cap_base, cap_rim])
    cap.visual = trimesh.visual.color.ColorVisuals(
        mesh=cap, vertex_colors=np.tile(thumb_black, (len(cap.vertices), 1))
    )
    uno_submeshes.append(cap)

# Central 4-Channel Servo Header (meARM.WDT 4-servos connector)
# Positioned at X = shield_center_x, Y = shield_center_y + 15.0
servo_hdr_y = shield_center_y + 15.0
servo_hdr_x = shield_center_x

# Black plastic housing (13.0 x 9.0 x 3.0 mm)
hdr_base = trimesh.creation.box([13.0, 9.0, 3.0])
hdr_base.apply_translation([servo_hdr_x, servo_hdr_y, shield_z_base + shield_pcb_th + 1.5])
hdr_base.visual = trimesh.visual.color.ColorVisuals(
    mesh=hdr_base, vertex_colors=np.tile(dc_dark, (len(hdr_base.vertices), 1))
)
uno_submeshes.append(hdr_base)

# 4 Gold Signal Pins (Row 1, closest to servos)
pin_h = 6.0
pin_z = shield_z_base + shield_pcb_th + 3.0 + pin_h / 2.0
signal_yellow = np.array([245, 185, 25, 255], dtype=np.uint8)
power_red = np.array([225, 45, 35, 255], dtype=np.uint8)
gnd_black = np.array([20, 20, 20, 255], dtype=np.uint8)

pin_pitch = 2.54
for col in range(4):
    px = servo_hdr_x - 1.5 * pin_pitch + col * pin_pitch
    # Signal pin (D11, D10, D9, D6)
    p_sig = trimesh.creation.box([0.7, 0.7, pin_h])
    p_sig.apply_translation([px, servo_hdr_y + pin_pitch, pin_z])
    p_sig.visual = trimesh.visual.color.ColorVisuals(
        mesh=p_sig, vertex_colors=np.tile(signal_yellow, (len(p_sig.vertices), 1))
    )
    uno_submeshes.append(p_sig)

    # 5V pin
    p_5v = trimesh.creation.box([0.7, 0.7, pin_h])
    p_5v.apply_translation([px, servo_hdr_y, pin_z])
    p_5v.visual = trimesh.visual.color.ColorVisuals(
        mesh=p_5v, vertex_colors=np.tile(power_red, (len(p_5v.vertices), 1))
    )
    uno_submeshes.append(p_5v)

    # GND pin
    p_gnd = trimesh.creation.box([0.7, 0.7, pin_h])
    p_gnd.apply_translation([px, servo_hdr_y - pin_pitch, pin_z])
    p_gnd.visual = trimesh.visual.color.ColorVisuals(
        mesh=p_gnd, vertex_colors=np.tile(gnd_black, (len(p_gnd.vertices), 1))
    )
    uno_submeshes.append(p_gnd)

# Reset Button (tactile switch)
rst_base = trimesh.creation.box([6.0, 6.0, 3.5])
rst_base.apply_translation([shield_center_x - 22.0, shield_center_y + 16.0, shield_z_base + shield_pcb_th + 1.75])
rst_base.visual = trimesh.visual.color.ColorVisuals(
    mesh=rst_base, vertex_colors=np.tile(metal_gray, (len(rst_base.vertices), 1))
)
uno_submeshes.append(rst_base)

rst_btn = trimesh.creation.cylinder(radius=1.6, height=2.0, sections=16)
rst_btn.apply_translation([shield_center_x - 22.0, shield_center_y + 16.0, shield_z_base + shield_pcb_th + 3.5 + 1.0])
rst_btn.visual = trimesh.visual.color.ColorVisuals(
    mesh=rst_btn, vertex_colors=np.tile(power_red, (len(rst_btn.vertices), 1))
)
uno_submeshes.append(rst_btn)

# Combine all into the single electronics geometry
combined_electronics = trimesh.util.concatenate(uno_submeshes)
scene.geometry['eletronica_arduino_shield'] = combined_electronics
print(f"Electronics combined: {len(combined_electronics.vertices)} vertices, {len(combined_electronics.faces)} faces")

# ==============================================================================
# 5. ADJUST SERVO CABLE TERMINATIONS TO ENTER JOYSTICK SHIELD HEADER
# ==============================================================================
print("5. Routing servo cables into Joystick Shield header...")
# Connect cables directly to header pins at (servo_hdr_x, servo_hdr_y, pin_z)
# Each cable connects smoothly from arm base to (col X, servo_hdr_y)
cable_pins = {
    'cabo_servo_base': (servo_hdr_x - 1.5 * pin_pitch, servo_hdr_y + pin_pitch),      # Channel 1: D11
    'cabo_servo_ombro': (servo_hdr_x - 0.5 * pin_pitch, servo_hdr_y + pin_pitch),     # Channel 2: D10
    'cabo_servo_cotovelo': (servo_hdr_x + 0.5 * pin_pitch, servo_hdr_y + pin_pitch),  # Channel 3: D9
    'cabo_servo_garra': (servo_hdr_x + 1.5 * pin_pitch, servo_hdr_y + pin_pitch),     # Channel 4: D6
}

for c_name, (tgt_x, tgt_y) in cable_pins.items():
    if c_name in scene.geometry:
        c_mesh = scene.geometry[c_name].copy()
        # Find vertices near the bottom front (Y < 60) and adjust them to land in the shield pin
        v = c_mesh.vertices.copy()
        front_mask = v[:, 1] < 65.0
        if np.any(front_mask):
            current_front_mean = v[front_mask].mean(axis=0)
            dx = tgt_x - current_front_mean[0]
            dy = tgt_y - current_front_mean[1]
            dz = (shield_z_base + shield_pcb_th + 6.0) - current_front_mean[2]
            
            # Smooth weighting along Y
            weights = np.clip((65.0 - v[:, 1]) / 25.0, 0.0, 1.0)[:, np.newaxis]
            v += weights * np.array([dx, dy, dz])
            c_mesh.vertices = v
            scene.geometry[c_name] = c_mesh

# ==============================================================================
# 6. EXPORT UPDATED GLB AND STL
# ==============================================================================
print("6. Exporting updated GLB and STL assemblies...")
scene.export(GLB_PATH)
print(f"Saved: {GLB_PATH}")

# Full assembled STL
all_meshes = [g for g in scene.geometry.values() if isinstance(g, trimesh.Trimesh) and len(g.faces) > 0]
combined_stl = trimesh.util.concatenate(all_meshes)
combined_stl.export(STL_PATH)
print(f"Saved: {STL_PATH} ({len(combined_stl.faces)} faces)")

# ==============================================================================
# 7. UPDATE METADATA (components_metadata.json)
# ==============================================================================
print("7. Updating components_metadata.json...")
with open(METADATA_PATH, 'r', encoding='utf-8') as f:
    meta = json.load(f)

for comp in meta['components']:
    if comp['id'] == 'printed_01_Base_Chassi_Inferior':
        comp['desc'] = "Placa base estrutural ergonômica com contorno curvilíneo, abas de apoio para punhos/mãos, furações para pés de TPU e parafusos de fixação M3 para o conjunto rotativo e Arduino Uno."
    elif comp['id'] == 'eletronica_arduino_shield':
        comp['name'] = "Arduino Uno R3 + Shield Joystick Duplo (meARM.WDT V2.10)"
        comp['material'] = "PCB Dupla Face FR4 / ATmega328P / 2x Joysticks Analógicos PS2"
        comp['desc'] = "Placa controladora equipada com o Shield Joystick Duplo meArm.WDT montado no topo, com dois analógicos integrados (A0-A3) e barramento dedicado de 4 canais de servomotores (D11, D10, D9, D6)."
        comp['pin'] = "Alimentação 5V 3A / D11 (Cotovelo), D10 (Ombro), D9 (Base), D6 (Garra)"
    elif comp['id'] == 'cabos_ribbon':
        comp['desc'] = "Fiação organizada conectando cada atuador diretamente aos 4 conectores dedicados (S, 5V, GND) do Shield Joystick Duplo meARM.WDT."

# Overall statistics
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

print("SUCCESS: Assembly, metadata and 3D files completely synchronized!")
