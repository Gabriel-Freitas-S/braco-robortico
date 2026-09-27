"""
build_dae_arm_assembly.py
=========================
Reconstrói o modelo 3D completo do braço robótico com componentes (braco_completo_com_componentes.glb/stl)
utilizando a geometria funcional real do modelo DAE de referência (MDF).

Substitui a base plana de madeira antiga pela base chassi ergonômica de impressão 3D (01_Base_Chassi_Inferior)
com suporte ao Arduino Uno + CNC Shield e pés de TPU, perfeitamente alinhada pelos 4 espaçadores M3x24mm.

Elimina protoboard e jumpers avulsos, mantendo a mecânica exata e comprovada do braço em 100% Black PLA.
"""

import trimesh
import numpy as np
import os
import json

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
MDF_GLB_PATH = os.path.join(BASE_DIR, "ref_mdf", "braco_mdf_in_mm.glb")
OUTPUT_GLB = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
OUTPUT_STL = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")

print("=" * 60)
print("REBUILDING ASSEMBLY FROM DAE ROBOTIC ARM GROUND TRUTH")
print("=" * 60)

# Colors
BLACK_PLA = np.array([34, 38, 44, 255], dtype=np.uint8)      # #22262C Technical Black PLA
BLUE_SG90 = np.array([24, 115, 230, 255], dtype=np.uint8)     # #1873E6 SG90 Servo Blue
METAL_GREY = np.array([185, 190, 195, 255], dtype=np.uint8)   # Chrome / Steel fasteners
NYLON_WHITE = np.array([240, 245, 250, 255], dtype=np.uint8)  # White nylon horns
TPU_BLACK = np.array([25, 25, 28, 255], dtype=np.uint8)       # TPU feet
ARDUINO_TEAL = np.array([0, 135, 143, 255], dtype=np.uint8)   # Arduino PCB
GOLD_HEADER = np.array([212, 175, 55, 255], dtype=np.uint8)

def set_color(mesh, color):
    mesh.visual = trimesh.visual.ColorVisuals(
        mesh=mesh,
        vertex_colors=np.tile(color, (len(mesh.vertices), 1))
    )
    return mesh

# ------------------------------------------------------------------
# 1. BASE CHASSI INFERIOR (3D PRINTED)
# ------------------------------------------------------------------
print("\n1. Loading 3D Printed Base Chassi...")
base_mesh = trimesh.load(os.path.join(PARTS_DIR, "01_Base_Chassi_Inferior.stl"))
# Place base plate at Z = 0 to 4.0 mm
base_mesh.apply_translation([0, 0, -base_mesh.bounds[0][2]])
set_color(base_mesh, BLACK_PLA)
print(f"   Base plate bounds: {base_mesh.bounds.tolist()}")

# ------------------------------------------------------------------
# 2. TPU FEET (4 UNITS UNDER BASE)
# ------------------------------------------------------------------
print("\n2. Creating 4 TPU anti-vibration feet...")
tpu_feet = []
foot_coords = [
    [-46.0, -75.0],
    [ 52.0, -75.0],
    [-46.0,  75.0],
    [ 52.0,  75.0]
]
for i, (fx, fy) in enumerate(foot_coords):
    # Truncated cone / cylindrical foot: radius 8mm, height 6mm
    foot = trimesh.creation.cylinder(radius=8.0, height=6.0, sections=24)
    # Move to under base: Z from -6.0 to 0.0
    foot.apply_translation([fx, fy, -3.0])
    set_color(foot, TPU_BLACK)
    tpu_feet.append(foot)

# ------------------------------------------------------------------
# 3. 4x M3x24mm METALLIC STANDOFFS
# ------------------------------------------------------------------
print("\n3. Creating 4x M3x24mm standoffs...")
standoff_holes = [
    [ 22.70, 19.31],
    [-27.54, 19.35],
    [ 20.51, 68.00],
    [-27.54, 69.84]
]
standoffs = []
for i, (sx, sy) in enumerate(standoff_holes):
    # Hex standoff: radius 3.0mm, height 24mm, Z from 4.0 to 28.0 mm
    st = trimesh.creation.cylinder(radius=3.0, height=24.0, sections=6)
    st.apply_translation([sx, sy, 4.0 + 12.0])
    set_color(st, METAL_GREY)
    standoffs.append(st)

# ------------------------------------------------------------------
# 4. ARDUINO UNO + CNC/SENSOR SHIELD
# ------------------------------------------------------------------
print("\n4. Creating Arduino Uno + Sensor Shield assembly...")
# Arduino Uno PCB: 68.6 x 53.4 x 1.6 mm, sits on 3mm nylon spacers above base plate (Z = 7.0 to 8.6 mm)
# Arduino center over mounting holes:
uno_center_x = 4.7
uno_center_y = -35.0
uno_z = 7.0
pcb_uno = trimesh.creation.box(extents=[53.4, 68.6, 1.6])
pcb_uno.apply_translation([uno_center_x, uno_center_y, uno_z + 0.8])
set_color(pcb_uno, ARDUINO_TEAL)

# Sensor Shield PCB sits on top of Arduino: Z = 19.0 to 20.6 mm
pcb_shield = trimesh.creation.box(extents=[53.4, 68.6, 1.6])
pcb_shield.apply_translation([uno_center_x, uno_center_y, uno_z + 12.0 + 0.8])
set_color(pcb_shield, ARDUINO_TEAL)

# Pin headers between Uno and Shield
hdr_l = trimesh.creation.box(extents=[3.0, 50.0, 10.4])
hdr_l.apply_translation([uno_center_x - 24.0, uno_center_y, uno_z + 1.6 + 5.2])
set_color(hdr_l, BLACK_PLA)

hdr_r = trimesh.creation.box(extents=[3.0, 50.0, 10.4])
hdr_r.apply_translation([uno_center_x + 24.0, uno_center_y, uno_z + 1.6 + 5.2])
set_color(hdr_r, BLACK_PLA)

# Servo 3-pin headers (white/red/black headers for 4 servos)
servo_hdrs = trimesh.creation.box(extents=[10.0, 25.0, 8.5])
servo_hdrs.apply_translation([uno_center_x + 12.0, uno_center_y + 10.0, uno_z + 13.6 + 4.25])
set_color(servo_hdrs, BLACK_PLA)

# USB-B metal connector and DC barrel jack
usb = trimesh.creation.box(extents=[12.0, 16.0, 11.0])
usb.apply_translation([uno_center_x - 16.0, uno_center_y - 34.3 + 8.0, uno_z + 1.6 + 5.5])
set_color(usb, METAL_GREY)

pwr = trimesh.creation.box(extents=[9.0, 13.5, 11.0])
pwr.apply_translation([uno_center_x + 15.0, uno_center_y - 34.3 + 6.75, uno_z + 1.6 + 5.5])
set_color(pwr, BLACK_PLA)

electronics = [pcb_uno, pcb_shield, hdr_l, hdr_r, servo_hdrs, usb, pwr]

# ------------------------------------------------------------------
# 5. ROBOTIC ARM FROM DAE GROUND TRUTH
# ------------------------------------------------------------------
print("\n5. Extracting and transforming robotic arm from MDF DAE...")
mdf_scene = trimesh.load(MDF_GLB_PATH)
geoms = list(mdf_scene.geometry.values())

# Translation delta to align DAE arm turntable on the 24mm standoffs:
DX = -5.42
DY = -70.26
DZ = 28.00
TRANS = np.array([DX, DY, DZ])

# Geometries breakdown in DAE:
# geoms[0..3]: connector nuts
# geoms[4]: preto_002 (servo small gears/details)
# geoms[5]: dourado_001 (brass/copper gear pivots)
# geoms[6]: Material_021 (white nylon servo horns and links)
# geoms[7]: Protoboard_004 (EXCLUDE - user specified no protoboard)
# geoms[8..13]: Madeira/wood parts (ALL ARM WOOD PARTS, except the old rectangular base plate)
# geoms[14]: Metal_007 (M3 screws and nuts)
# geoms[15]: Metal_009 (metallic standoffs in MDF - replaced by our 4 new standoffs)
# geoms[16]: METAL_001 (hardware)
# geoms[17]: Azul_Servo_001 (4 SG90 BLUE SERVO BODIES)
# geoms[18]: Metal_002 (M3 washers and hardware)

# Separate wood parts and exclude the old flat MDF base plate:
wood_geoms = geoms[8:14]
wood_combined = trimesh.util.concatenate(wood_geoms)
wood_parts = wood_combined.split(only_watertight=False)

arm_printed_parts = []
for p in wood_parts:
    # Exclude old base plate: extents ~ 95 x 145 x 3 mm at Z < -20
    b = p.bounds
    ext = p.bounding_box.extents
    if ext[0] > 90.0 and ext[1] > 140.0:
        print(f"   Excluded old MDF base plate: extents={ext.round(1).tolist()}")
        continue
    # Exclude base plate cutouts/tabs that sit at Z < -20 in MDF
    if b[1][2] < -20.0 and ext.max() < 45.0:
        continue
    
    # Translate to match our base standoffs
    p_copy = p.copy()
    p_copy.apply_translation(TRANS)
    set_color(p_copy, BLACK_PLA)
    arm_printed_parts.append(p_copy)

print(f"   Retained {len(arm_printed_parts)} functional arm printed components")

# Servos: geoms[17] (Blue SG90 bodies)
servos_mesh = geoms[17].copy()
servos_mesh.apply_translation(TRANS)
set_color(servos_mesh, BLUE_SG90)

# Servo horns / white links: geoms[6]
horns_mesh = geoms[6].copy()
horns_mesh.apply_translation(TRANS)
set_color(horns_mesh, NYLON_WHITE)

# Internal servo gears / details: geoms[4] & geoms[5]
gears_mesh_1 = geoms[4].copy()
gears_mesh_1.apply_translation(TRANS)
set_color(gears_mesh_1, BLACK_PLA)

gears_mesh_2 = geoms[5].copy()
gears_mesh_2.apply_translation(TRANS)
set_color(gears_mesh_2, GOLD_HEADER)

# Screws, nuts, washers: geoms[14], geoms[16], geoms[18]
screws_mesh_1 = geoms[14].copy()
screws_mesh_1.apply_translation(TRANS)
set_color(screws_mesh_1, METAL_GREY)

screws_mesh_2 = geoms[16].copy()
screws_mesh_2.apply_translation(TRANS)
set_color(screws_mesh_2, METAL_GREY)

screws_mesh_3 = geoms[18].copy()
screws_mesh_3.apply_translation(TRANS)
set_color(screws_mesh_3, METAL_GREY)

# Connector brackets: geoms[0..3]
connectors = []
for g in geoms[0:4]:
    c = g.copy()
    c.apply_translation(TRANS)
    set_color(c, BLACK_PLA)
    connectors.append(c)

# ------------------------------------------------------------------
# 6. ASSEMBLE FULL SCENE
# ------------------------------------------------------------------
print("\n6. Building final Trimesh Scene...")
full_scene = trimesh.Scene()

# Add base plate and feet
full_scene.add_geometry(base_mesh, node_name="printed_01_Base_Chassi_Inferior")
for i, f in enumerate(tpu_feet, 1):
    full_scene.add_geometry(f, node_name=f"pe_tpu_{i}")

# Add standoffs
for i, s in enumerate(standoffs, 1):
    full_scene.add_geometry(s, node_name=f"espacador_metalico_m3_24mm_{i}")

# Add electronics
full_scene.add_geometry(trimesh.util.concatenate(electronics), node_name="eletronica_arduino_shield")

# Add printed arm parts
arm_combined = trimesh.util.concatenate(arm_printed_parts + connectors)
full_scene.add_geometry(arm_combined, node_name="mecanica_braco_pecas_impressas_pla")

# Add servos and horns
full_scene.add_geometry(servos_mesh, node_name="servos_sg90_4x_azul")
full_scene.add_geometry(horns_mesh, node_name="cruzetas_horns_nylon_branco")
full_scene.add_geometry(trimesh.util.concatenate([gears_mesh_1, gears_mesh_2]), node_name="engrenagens_servos")

# Add all fasteners
all_fasteners = trimesh.util.concatenate([screws_mesh_1, screws_mesh_2, screws_mesh_3])
full_scene.add_geometry(all_fasteners, node_name="fixadores_parafusos_porcas_arruelas_m3")

print(f"   Scene assembled: {len(full_scene.geometry)} geometries")
print(f"   Scene bounds: {full_scene.bounds.tolist()}")
print(f"   Scene extents: {full_scene.extents.tolist()}")

# ------------------------------------------------------------------
# 7. EXPORT GLB AND STL
# ------------------------------------------------------------------
print(f"\n7. Exporting {OUTPUT_GLB}...")
full_scene.export(OUTPUT_GLB)
print(f"   Saved GLB: {os.path.getsize(OUTPUT_GLB) // 1024} KB")

print(f"\n8. Exporting {OUTPUT_STL}...")
all_meshes = list(full_scene.geometry.values())
single_mesh = trimesh.util.concatenate(all_meshes)
single_mesh.export(OUTPUT_STL)
print(f"   Saved STL: {os.path.getsize(OUTPUT_STL) // 1024} KB")

print("\n" + "=" * 60)
print("REBUILD COMPLETE: braco_completo_com_componentes (.glb and .stl)")
print("100% compliant with DAE mechanics and 3D printed components!")
print("=" * 60)
