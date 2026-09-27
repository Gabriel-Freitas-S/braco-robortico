"""
build_complete_dual_joystick_arm.py
===================================
Reconstrói o modelo 3D completo do braço robótico com componentes (braco_completo_com_componentes.glb/stl)
incorporando com fidelidade máxima:
1. Nova Base Chassi 01_Base_Chassi_Inferior.stl fiel ao modelo MDF de referência (95 x 145 x 4 mm,
   recorte diamond/berço da mesa giratória, 4 furações dos espaçadores e 4 furações do Arduino Uno).
2. Fixação direta do Arduino Uno R3 sobre a base com 4 parafusos M3 e espaçadores nylon de 3mm.
3. Shield com 2 Joysticks analógicos para Arduino Uno (modelo Casa da Robótica):
   - PCB de expansão sobre o Arduino Uno (65.1 x 53.8 mm)
   - 2 Joysticks analógicos ergonômicos lado a lado (gamepad layout) com chapéus emborrachados
   - Interface de 4 canais dedicados para servomotores voltada para o braço
   - Conector Bluetooth HC-06, LEDs de status e botões táteis
4. Chicote elétrico realista dos 4 servomotores:
   - Cabos de fita 3 vias (Marrom/Vermelho/Laranja) modelados com splines 3D com curvatura natural
   - Conectores Dupont fêmea de 3 pinos pretos plugados na interface de servos do Shield
5. Cinemática funcional 100% fiel ao modelo DAE (MDF de referência), em Technical Black PLA.
6. Exportação de braco_completo_com_componentes.glb e .stl.
"""

import trimesh
import numpy as np
import os
from scipy.interpolate import splprep, splev

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
MDF_GLB_PATH = os.path.join(BASE_DIR, "ref_mdf", "braco_mdf_in_mm.glb")
OUTPUT_GLB = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
OUTPUT_STL = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")

print("=" * 65)
print("BUILDING COMPLETE ROBOTIC ARM WITH DUAL JOYSTICK SHIELD & WIRING")
print("=" * 65)

# Colors
BLACK_PLA = np.array([34, 38, 44, 255], dtype=np.uint8)       # #22262C Technical Black PLA
BLUE_SG90 = np.array([24, 115, 230, 255], dtype=np.uint8)      # #1873E6 SG90 Servo Blue
METAL_GREY = np.array([185, 190, 195, 255], dtype=np.uint8)    # Chrome / Steel fasteners
NYLON_WHITE = np.array([240, 245, 250, 255], dtype=np.uint8)   # White nylon horns
TPU_BLACK = np.array([22, 22, 25, 255], dtype=np.uint8)        # Matte Black TPU
ARDUINO_BLUE = np.array([0, 135, 143, 255], dtype=np.uint8)    # Arduino Teal Blue PCB
SHIELD_BLUE = np.array([12, 95, 160, 255], dtype=np.uint8)     # Shield Dark Blue PCB
GOLD_PIN = np.array([212, 175, 55, 255], dtype=np.uint8)       # Gold header pins
RUBBER_THUMB = np.array([30, 34, 40, 255], dtype=np.uint8)     # Rubber thumbstick hat
SILVER_POT = np.array([200, 205, 210, 255], dtype=np.uint8)    # Potentiometer metal casing
DUPONT_BLACK = np.array([18, 18, 20, 255], dtype=np.uint8)     # Dupont plastic housing
WIRE_BROWN = np.array([115, 65, 30, 255], dtype=np.uint8)      # GND wire
WIRE_RED = np.array([210, 35, 35, 255], dtype=np.uint8)        # 5V wire
WIRE_ORANGE = np.array([235, 120, 20, 255], dtype=np.uint8)    # PWM Signal wire

def set_color(mesh, color):
    mesh.visual = trimesh.visual.ColorVisuals(
        mesh=mesh,
        vertex_colors=np.tile(color, (len(mesh.vertices), 1))
    )
    return mesh

# ------------------------------------------------------------------
# 1. BASE CHASSI INFERIOR (3D PRINTED) & TPU FEET
# ------------------------------------------------------------------
print("\n1. Loading Base Chassi plate and adding TPU feet...")
base_mesh = trimesh.load(os.path.join(PARTS_DIR, "01_Base_Chassi_Inferior.stl"))
base_mesh.apply_translation([0, 0, -base_mesh.bounds[0][2]])
set_color(base_mesh, BLACK_PLA)

tpu_feet = []
foot_coords = [[-38.0, -62.0], [38.0, -62.0], [-38.0, 62.0], [38.0, 62.0]]
for i, (fx, fy) in enumerate(foot_coords):
    foot = trimesh.creation.cylinder(radius=7.0, height=5.0, sections=24)
    foot.apply_translation([fx, fy, -2.5])
    set_color(foot, TPU_BLACK)
    tpu_feet.append(foot)

# ------------------------------------------------------------------
# 2. 4x M3x24mm METALLIC STANDOFFS
# ------------------------------------------------------------------
print("\n2. Creating 4x M3x24mm standoffs...")
standoff_holes = [[28.13, 5.07], [-22.10, 5.08], [25.96, 53.73], [-22.12, 55.62]]
standoffs = []
for sx, sy in standoff_holes:
    st = trimesh.creation.cylinder(radius=2.9, height=24.0, sections=6)
    st.apply_translation([sx, sy, 4.0 + 12.0])
    set_color(st, METAL_GREY)
    standoffs.append(st)

# ------------------------------------------------------------------
# 3. ARDUINO UNO R3 (ALIGNED WITH BASE MOUNTING HOLES)
# ------------------------------------------------------------------
print("\n3. Creating Arduino Uno R3 perfectly aligned with mounting holes...")
# Center of Arduino mounting footprint on base plate:
UNO_CX = 10.42
UNO_CY = -46.81
UNO_Z = 7.0  # Sits on 3mm nylon standoffs (Z = 4.0 to 7.0 mm)

# Arduino PCB: 68.6 (length in X) x 53.4 (width in Y) x 1.6 mm
uno_pcb = trimesh.creation.box(extents=[68.6, 53.4, 1.6])
uno_pcb.apply_translation([UNO_CX, UNO_CY, UNO_Z + 0.8])
set_color(uno_pcb, ARDUINO_BLUE)

# USB-B metal receptacle & DC barrel jack on the left edge (-X)
usb = trimesh.creation.box(extents=[16.0, 12.0, 11.0])
usb.apply_translation([UNO_CX - 34.3 + 8.0, UNO_CY + 14.0, UNO_Z + 1.6 + 5.5])
set_color(usb, METAL_GREY)

pwr = trimesh.creation.box(extents=[13.5, 9.0, 11.0])
pwr.apply_translation([UNO_CX - 34.3 + 6.75, UNO_CY - 16.0, UNO_Z + 1.6 + 5.5])
set_color(pwr, DUPONT_BLACK)

# Uno stacking female pin headers (black plastic strips)
uno_hdr_top = trimesh.creation.box(extents=[48.0, 2.5, 8.5])
uno_hdr_top.apply_translation([UNO_CX + 4.0, UNO_CY + 24.5, UNO_Z + 1.6 + 4.25])
set_color(uno_hdr_top, DUPONT_BLACK)

uno_hdr_bot = trimesh.creation.box(extents=[48.0, 2.5, 8.5])
uno_hdr_bot.apply_translation([UNO_CX + 4.0, UNO_CY - 24.5, UNO_Z + 1.6 + 4.25])
set_color(uno_hdr_bot, DUPONT_BLACK)

# 4 Mounting Screws and 3mm Spacers fixing Arduino to Base 01
uno_mounting_screws = []
uno_holes = [[-16.02, -68.47], [36.29, -63.64], [36.21, -34.93], [-14.82, -20.21]]
for hx, hy in uno_holes:
    # 3mm nylon standoff spacer under Arduino (Z = 4.0 to 7.0 mm)
    spacer = trimesh.creation.cylinder(radius=3.0, height=3.0, sections=16)
    spacer.apply_translation([hx, hy, 4.0 + 1.5])
    set_color(spacer, NYLON_WHITE)
    uno_mounting_screws.append(spacer)
    # M3 screw passing through base and Arduino (Z = 0 to 9.5 mm)
    scr = trimesh.creation.cylinder(radius=1.45, height=9.5, sections=12)
    scr.apply_translation([hx, hy, 4.75])
    set_color(scr, METAL_GREY)
    uno_mounting_screws.append(scr)

arduino_parts = [uno_pcb, usb, pwr, uno_hdr_top, uno_hdr_bot] + uno_mounting_screws

# ------------------------------------------------------------------
# 4. SHIELD COM 2 JOYSTICKS PARA ARDUINO UNO (HIGH FIDELITY)
# ------------------------------------------------------------------
print("\n4. Creating Shield com 2 Joysticks (Casa da Robótica model)...")
# Shield PCB sits on top of Uno female headers:
SHIELD_Z = 17.1
shield_pcb = trimesh.creation.box(extents=[65.1, 53.8, 1.6])
shield_pcb.apply_translation([UNO_CX, UNO_CY, SHIELD_Z + 0.8])
set_color(shield_pcb, SHIELD_BLUE)

# Male pin headers under shield entering Uno headers
shield_pins_top = trimesh.creation.box(extents=[48.0, 2.0, 8.5])
shield_pins_top.apply_translation([UNO_CX + 4.0, UNO_CY + 24.5, UNO_Z + 1.6 + 4.25])
set_color(shield_pins_top, GOLD_PIN)

shield_pins_bot = trimesh.creation.box(extents=[48.0, 2.0, 8.5])
shield_pins_bot.apply_translation([UNO_CX + 4.0, UNO_CY - 24.5, UNO_Z + 1.6 + 4.25])
set_color(shield_pins_bot, GOLD_PIN)

# Helper for analog thumbstick joystick:
def create_thumbstick(center_x, center_y, base_z):
    components = []
    # 1. Square potentiometer casing box (16 x 16 x 12.0 mm)
    pot_box = trimesh.creation.box(extents=[16.0, 16.0, 12.0])
    pot_box.apply_translation([center_x, center_y, base_z + 6.0])
    set_color(pot_box, SILVER_POT)
    components.append(pot_box)
    
    # 2. Side potentiometer green/black tabs (X and Y axis pots)
    pot_x = trimesh.creation.box(extents=[3.5, 9.0, 9.0])
    pot_x.apply_translation([center_x - 9.0, center_y, base_z + 5.0])
    set_color(pot_x, DUPONT_BLACK)
    components.append(pot_x)
    
    pot_y = trimesh.creation.box(extents=[9.0, 3.5, 9.0])
    pot_y.apply_translation([center_x, center_y - 9.0, base_z + 5.0])
    set_color(pot_y, DUPONT_BLACK)
    components.append(pot_y)
    
    # 3. Articulated neck shaft (cylinder dia 4.5mm, height 8mm)
    neck = trimesh.creation.cylinder(radius=2.25, height=8.0, sections=16)
    neck.apply_translation([center_x, center_y, base_z + 12.0 + 4.0])
    set_color(neck, SILVER_POT)
    components.append(neck)
    
    # 4. Rubberized Thumbstick Hat:
    skirt = trimesh.creation.cylinder(radius=9.0, height=5.0, sections=24)
    skirt.apply_translation([center_x, center_y, base_z + 14.5])
    set_color(skirt, RUBBER_THUMB)
    components.append(skirt)
    
    cap = trimesh.creation.cylinder(radius=11.5, height=3.5, sections=32)
    cap.apply_translation([center_x, center_y, base_z + 18.0])
    set_color(cap, RUBBER_THUMB)
    components.append(cap)
    
    dish_rim = trimesh.creation.annulus(r_min=9.5, r_max=11.5, height=1.5, sections=32)
    dish_rim.apply_translation([center_x, center_y, base_z + 19.5])
    set_color(dish_rim, RUBBER_THUMB)
    components.append(dish_rim)
    
    return components

# Dual Joysticks side-by-side (gamepad arrangement)
# Left Joystick: center X = UNO_CX - 15.0 mm, Y = UNO_CY - 2.0 mm
joy_l = create_thumbstick(UNO_CX - 15.0, UNO_CY - 2.0, SHIELD_Z + 1.6)

# Right Joystick: center X = UNO_CX + 15.0 mm, Y = UNO_CY - 2.0 mm
joy_r = create_thumbstick(UNO_CX + 15.0, UNO_CY - 2.0, SHIELD_Z + 1.6)

# 4-Channel Dedicated Servo Interface (3-pin headers for 4 servos)
# Placed along top edge of Shield PCB (facing toward the arm at +Y)
servo_header_pins = []
servo_header_base = trimesh.creation.box(extents=[16.0, 8.0, 2.5])
servo_header_base.apply_translation([UNO_CX + 10.0, UNO_CY + 22.0, SHIELD_Z + 1.6 + 1.25])
set_color(servo_header_base, DUPONT_BLACK)
servo_header_pins.append(servo_header_base)

# 4 Channels x 3 Pins = 12 gold pins
for ch_idx, ch_x in enumerate([UNO_CX + 5.5, UNO_CX + 8.5, UNO_CX + 11.5, UNO_CX + 14.5]):
    for row_idx, row_y in enumerate([-2.54, 0.0, 2.54]): # GND, VCC, Signal
        pin = trimesh.creation.cylinder(radius=0.35, height=6.0, sections=8)
        pin.apply_translation([ch_x, UNO_CY + 22.0 + row_y, SHIELD_Z + 1.6 + 2.5 + 3.0])
        set_color(pin, GOLD_PIN)
        servo_header_pins.append(pin)

# Additional onboard details: Bluetooth header, tact switches, LEDs
bt_hdr = trimesh.creation.box(extents=[11.0, 2.5, 8.5])
bt_hdr.apply_translation([UNO_CX - 15.0, UNO_CY + 22.0, SHIELD_Z + 1.6 + 4.25])
set_color(bt_hdr, DUPONT_BLACK)

btn1 = trimesh.creation.box(extents=[6.0, 6.0, 4.0])
btn1.apply_translation([UNO_CX - 24.0, UNO_CY - 20.0, SHIELD_Z + 1.6 + 2.0])
set_color(btn1, METAL_GREY)

btn2 = trimesh.creation.box(extents=[6.0, 6.0, 4.0])
btn2.apply_translation([UNO_CX + 24.0, UNO_CY - 20.0, SHIELD_Z + 1.6 + 2.0])
set_color(btn2, METAL_GREY)

shield_parts = [shield_pcb, shield_pins_top, shield_pins_bot, bt_hdr, btn1, btn2] + joy_l + joy_r + servo_header_pins

# ------------------------------------------------------------------
# 5. ROBOTIC ARM FROM DAE GROUND TRUTH
# ------------------------------------------------------------------
print("\n5. Extracting robotic arm from DAE reference...")
mdf_scene = trimesh.load(MDF_GLB_PATH)
geoms = list(mdf_scene.geometry.values())

DX = -0.03
DY = -84.60
DZ = 28.00
TRANS = np.array([DX, DY, DZ])

# Separate wood parts excluding old flat MDF base plate
wood_geoms = geoms[8:14]
wood_combined = trimesh.util.concatenate(wood_geoms)
wood_parts = wood_combined.split(only_watertight=False)

arm_printed_parts = []
for p in wood_parts:
    ext = p.bounding_box.extents
    b = p.bounds
    # Skip any subparts belonging to the old flat base plate at Z < 0
    if ext[0] > 70.0 and ext[1] > 100.0 and b[0][2] < -15.0:
        continue
    p_copy = p.copy()
    p_copy.apply_translation(TRANS)
    set_color(p_copy, BLACK_PLA)
    arm_printed_parts.append(p_copy)

# Servos, Horns, Fasteners
servos_mesh = geoms[17].copy()
servos_mesh.apply_translation(TRANS)
set_color(servos_mesh, BLUE_SG90)

horns_mesh = geoms[6].copy()
horns_mesh.apply_translation(TRANS)
set_color(horns_mesh, NYLON_WHITE)

gears_mesh_1 = geoms[4].copy()
gears_mesh_1.apply_translation(TRANS)
set_color(gears_mesh_1, NYLON_WHITE)

gears_mesh_2 = geoms[5].copy()
gears_mesh_2.apply_translation(TRANS)
set_color(gears_mesh_2, NYLON_WHITE)

screws_1 = geoms[14].copy()
screws_1.apply_translation(TRANS)
set_color(screws_1, METAL_GREY)

screws_2 = geoms[16].copy()
screws_2.apply_translation(TRANS)
set_color(screws_2, METAL_GREY)

screws_3 = geoms[18].copy()
screws_3.apply_translation(TRANS)
set_color(screws_3, METAL_GREY)

connectors = []
for g in geoms[0:4]:
    c = g.copy()
    c.apply_translation(TRANS)
    set_color(c, BLACK_PLA)
    connectors.append(c)

# ------------------------------------------------------------------
# 6. ROUTE 3-WIRE RIBBON CABLES WITH DUPONT PLUGS TO SHIELD
# ------------------------------------------------------------------
print("\n6. Routing 3-wire servo cables with female Dupont plugs to Shield...")

def build_ribbon_cable(waypoints, wire_radius=0.45, wire_spacing=1.0):
    """Generates 3 parallel wire cylinders (Brown, Red, Orange) following waypoints spline."""
    pts = np.array(waypoints)
    tck, u = splprep([pts[:, 0], pts[:, 1], pts[:, 2]], k=3, s=0)
    u_dense = np.linspace(0, 1, 35)
    curve = np.array(splev(u_dense, tck)).T
    
    # Calculate tangent vectors
    tangents = np.gradient(curve, axis=0)
    tangents /= np.linalg.norm(tangents, axis=1)[:, None]
    
    # Normal in horizontal plane for ribbon width
    up = np.array([0, 0, 1])
    normals = np.cross(tangents, up)
    norm_len = np.linalg.norm(normals, axis=1)
    for i in range(len(norm_len)):
        if norm_len[i] < 1e-4:
            normals[i] = np.array([1, 0, 0])
        else:
            normals[i] /= norm_len[i]
            
    wires_meshes = []
    offsets = [-wire_spacing, 0.0, wire_spacing]
    colors = [WIRE_BROWN, WIRE_RED, WIRE_ORANGE]
    
    for offset_val, col in zip(offsets, colors):
        wire_pts = curve + normals * offset_val
        segments = []
        for i in range(len(wire_pts) - 1):
            p_a = wire_pts[i]
            p_b = wire_pts[i+1]
            seg = trimesh.creation.cylinder(radius=wire_radius, segment=[p_a, p_b], sections=8)
            segments.append(seg)
        wire_mesh = trimesh.util.concatenate(segments)
        set_color(wire_mesh, col)
        wires_meshes.append(wire_mesh)
        
    return wires_meshes

def create_dupont_plug(plug_x, plug_y, plug_z):
    """Creates a 3-pin female Dupont connector plugged into the shield header."""
    housing = trimesh.creation.box(extents=[2.54, 7.62, 13.5])
    housing.apply_translation([plug_x, plug_y, plug_z + 6.75])
    set_color(housing, DUPONT_BLACK)
    return housing

# Channel pin header locations on Shield:
CH1_X = UNO_CX + 5.5   # Channel 1: Servo 1 (Base Yaw)
CH2_X = UNO_CX + 8.5   # Channel 2: Servo 2 (Ombro / Shoulder)
CH3_X = UNO_CX + 11.5  # Channel 3: Servo 3 (Cotovelo / Elbow)
CH4_X = UNO_CX + 14.5  # Channel 4: Servo 4 (Garra / Gripper)
PLUG_Y = UNO_CY + 22.0
PLUG_Z = SHIELD_Z + 1.6 + 2.5 # Sits on top of header base

TURNTABLE_X = 2.87
TURNTABLE_Y = 30.50

# Cable routes (Waypoints):
# 1. Servo 1 (Base / Yaw - sits under turntable at Z ~ 12 mm)
route_s1 = [
    [TURNTABLE_X - 5.0, TURNTABLE_Y - 5.0, 14.0],
    [TURNTABLE_X - 2.0, TURNTABLE_Y - 18.0, 7.5],
    [CH1_X, PLUG_Y + 12.0, 8.5],
    [CH1_X, PLUG_Y - 2.0, 26.0],
    [CH1_X, PLUG_Y, PLUG_Z + 14.0]
]

# 2. Servo 2 (Ombro - Right Tower at X ~ 35, Y ~ 31, Z ~ 60)
route_s2 = [
    [TURNTABLE_X + 33.0, TURNTABLE_Y, 58.0],
    [TURNTABLE_X + 26.0, TURNTABLE_Y - 15.0, 36.0],
    [TURNTABLE_X + 18.0, TURNTABLE_Y - 28.0, 16.0],
    [CH2_X + 4.0, PLUG_Y + 10.0, 9.0],
    [CH2_X, PLUG_Y - 2.0, 24.0],
    [CH2_X, PLUG_Y, PLUG_Z + 14.0]
]

# 3. Servo 3 (Cotovelo - Left Tower at X ~ -29, Y ~ 31, Z ~ 60)
route_s3 = [
    [TURNTABLE_X - 33.0, TURNTABLE_Y, 58.0],
    [TURNTABLE_X - 26.0, TURNTABLE_Y - 15.0, 36.0],
    [TURNTABLE_X - 18.0, TURNTABLE_Y - 28.0, 16.0],
    [CH3_X - 4.0, PLUG_Y + 10.0, 9.0],
    [CH3_X, PLUG_Y - 2.0, 24.0],
    [CH3_X, PLUG_Y, PLUG_Z + 14.0]
]

# 4. Servo 4 (Garra - Gripper)
route_s4 = [
    [TURNTABLE_X + 1.2, 74.0, 70.0],
    [TURNTABLE_X - 4.0, 62.0, 82.0],
    [TURNTABLE_X - 8.0, 50.0, 92.0],   # expansion bend around elbow pivot
    [TURNTABLE_X - 10.0, 40.0, 68.0],  # down main arm
    [TURNTABLE_X - 11.0, 28.0, 42.0],  # down tower
    [TURNTABLE_X - 6.0, 10.0, 16.0],
    [CH4_X - 2.0, PLUG_Y + 8.0, 9.0],
    [CH4_X, PLUG_Y - 2.0, 24.0],
    [CH4_X, PLUG_Y, PLUG_Z + 14.0]
]

cables = []
for r in [route_s1, route_s2, route_s3, route_s4]:
    cables.extend(build_ribbon_cable(r))

# 4 Dupont Plugs
dupont_plugs = [
    create_dupont_plug(CH1_X, PLUG_Y, PLUG_Z),
    create_dupont_plug(CH2_X, PLUG_Y, PLUG_Z),
    create_dupont_plug(CH3_X, PLUG_Y, PLUG_Z),
    create_dupont_plug(CH4_X, PLUG_Y, PLUG_Z)
]

# ------------------------------------------------------------------
# 7. BUILD FINAL COMBINED SCENE
# ------------------------------------------------------------------
print("\n7. Building unified master scene...")
final_scene = trimesh.Scene()

final_scene.add_geometry(base_mesh, node_name="printed_01_Base_Chassi_Inferior")
for i, f in enumerate(tpu_feet, 1):
    final_scene.add_geometry(f, node_name=f"pe_tpu_{i}")
for i, s in enumerate(standoffs, 1):
    final_scene.add_geometry(s, node_name=f"espacador_metalico_m3_24mm_{i}")

# Arduino Uno & Shield com 2 Joysticks
final_scene.add_geometry(trimesh.util.concatenate(arduino_parts), node_name="arduino_uno_r3")
final_scene.add_geometry(trimesh.util.concatenate(shield_parts), node_name="shield_2_joysticks_arduino")

# Robotic Arm Mechanics
arm_combined = trimesh.util.concatenate(arm_printed_parts + connectors)
final_scene.add_geometry(arm_combined, node_name="mecanica_braco_pecas_impressas_pla")
final_scene.add_geometry(servos_mesh, node_name="servos_sg90_4x_azul")
final_scene.add_geometry(horns_mesh, node_name="cruzetas_horns_nylon_branco")
final_scene.add_geometry(trimesh.util.concatenate([gears_mesh_1, gears_mesh_2]), node_name="engrenagens_servos")

# Fasteners
all_fasteners = trimesh.util.concatenate([screws_1, screws_2, screws_3])
final_scene.add_geometry(all_fasteners, node_name="fixadores_parafusos_porcas_arruelas_m3")

# Wiring & Plugs
final_scene.add_geometry(trimesh.util.concatenate(cables), node_name="chicote_cabos_fita_3_vias")
final_scene.add_geometry(trimesh.util.concatenate(dupont_plugs), node_name="conectores_dupont_femea_shield")

print(f"   Master scene built with {len(final_scene.geometry)} node groups")
print(f"   Scene bounds: {final_scene.bounds.tolist()}")
print(f"   Scene extents: {final_scene.extents.tolist()}")

# ------------------------------------------------------------------
# 8. EXPORT GLB AND STL
# ------------------------------------------------------------------
print(f"\n8. Exporting GLB to: {OUTPUT_GLB}...")
glb_bytes = final_scene.export(file_type="glb")
with open(OUTPUT_GLB, "wb") as f:
    f.write(glb_bytes)
print(f"   GLB saved successfully ({len(glb_bytes):,} bytes)")

print(f"\n9. Exporting full STL to: {OUTPUT_STL}...")
# Combine all meshes into a single solid mesh for STL
full_mesh = trimesh.util.concatenate(list(final_scene.geometry.values()))
full_mesh.export(OUTPUT_STL)
print(f"   STL saved successfully ({os.path.getsize(OUTPUT_STL):,} bytes)")

print("\n" + "=" * 65)
print("ASSEMBLY GENERATION COMPLETE!")
print("=" * 65)
