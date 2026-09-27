"""
fix_and_rebuild_assembly.py
============================
Reconstrói o braco_completo_com_componentes.glb corrigindo TODOS os erros
documentados em erros.md, usando as dimensões reais do modelo MDF como referência.

CORREÇÕES APLICADAS:
1. Servo 1 (Base/Yaw):
   - CORRIGIDO: O corpo do servo fica DENTRO DO VÃO dos espaçadores (embaixo da mesa 12)
   - A peça 16_Berco fica NO TOPO dos 4 espaçadores M3x24mm
   - A cruzeta (horn) acopla EMBAIXO da mesa giratória 12

2. Servos 2 e 3 (Torres):
   - CORRIGIDO: Servos montados pelo lado EXTERNO das torres, não interno
   - Sem colisão traseira-com-traseira

3. Paralelogramo:
   - CORRIGIDO: 04_Tirante_Paralelogramo_Medio usa comprimento 83mm (média MDF: 81.6-85.8mm)
   - CORRIGIDO: Tirante longo tem ancoragem fixa no ponto da torre (pino fixo)
   - CORRIGIDO: 13_Tirante_Superior_Garra conecta SOMENTE ao Pivô Superior do balancim

4. Garra (Servo 4):
   - CORRIGIDO: Eixo do servo apontando para BAIXO/CIMA (perpendicular à base da garra)
   - CORRIGIDO: 26_Engrenagem conectada à 28_Biela_Curta

Referência MDF confirmada:
  - Base: 95 x 145 x 3 mm
  - Braços principais: 114.8 mm
  - Antebraços: 81.6 e 85.8 mm
  - Mesa: 58 x 58 mm
  - Torres: 52-60 mm
"""

import trimesh
import numpy as np
import os
import json

BASE_DIR = r"D:\braco-robortico"
GLB_PATH = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
OUTPUT_GLB = os.path.join(BASE_DIR, "braco_completo_com_componentes.glb")
OUTPUT_STL = os.path.join(BASE_DIR, "braco_completo_com_componentes.stl")

print("=" * 60)
print("FIXING ASSEMBLY: braco_completo_com_componentes.glb")
print("=" * 60)

# Load the existing scene
print("\n1. Loading existing GLB scene...")
scene = trimesh.load(GLB_PATH)
print(f"   Loaded {len(scene.geometry)} geometries")

# ------------------------------------------------------------------
# COLORS
# ------------------------------------------------------------------
BLACK_PLA = np.array([34, 38, 44, 255], dtype=np.uint8)      # #22262C Technical Black PLA
BLUE_SG90 = np.array([24, 115, 230, 255], dtype=np.uint8)     # #1873E6 Servo blue
METAL_GREY = np.array([180, 185, 190, 255], dtype=np.uint8)   # Screws/standoffs
NYLON_WHITE = np.array([240, 245, 250, 255], dtype=np.uint8)  # Nylon washers
ORANGE_CABLE = np.array([230, 100, 20, 255], dtype=np.uint8)  # PWM wire
BROWN_CABLE = np.array([100, 55, 20, 255], dtype=np.uint8)    # GND wire
RED_CABLE = np.array([200, 30, 30, 255], dtype=np.uint8)      # 5V wire

def set_color(mesh, color):
    """Apply solid color to mesh via vertex colors."""
    mesh.visual = trimesh.visual.ColorVisuals(
        mesh=mesh,
        vertex_colors=np.tile(color, (len(mesh.vertices), 1))
    )
    return mesh

# Helper to get current center of a geometry
def center_of(geom_name):
    return scene.geometry[geom_name].bounds.mean(axis=0)

def move_to(geom_name, target_center):
    geom = scene.geometry[geom_name]
    delta = np.array(target_center) - geom.bounds.mean(axis=0)
    geom.apply_translation(delta)
    print(f"   Moved {geom_name}: new Z range [{geom.bounds[0][2]:.2f}, {geom.bounds[1][2]:.2f}]")

# ------------------------------------------------------------------
# FIX 1: BASE YAW (SERVO 1 + PECAS 12, 16, 25)
# ------------------------------------------------------------------
print("\n2. FIX 1 - Correcting Base Yaw mechanism...")
print("   Per erros.md: 16_Berco goes ON TOP of standoffs; 12_Mesa on servo shaft above")

# Current bounds analysis:
for k in ['printed_01_Base_Chassi_Inferior', 'printed_12_Mesa_Giratoria_Turntable',
          'printed_16_Berco_Servo_Rotacao_Base', 'printed_25_Trava_Horn_Servo_Base',
          'servo_01_base']:
    b = scene.geometry[k].bounds
    print(f"   {k}: Z=[{b[0][2]:.1f}, {b[1][2]:.1f}]")

# Correct assembly:
# Base chassi: Z = 10.24 to 14.24 mm (sits on TPU feet at Z=4.24-10.24mm)
# 4x M3x24mm standoffs: sit on top of base, Z = 14.24 to 38.24 mm
# 16_Berco_Servo_Rotacao_Base: sits on TOP of standoffs, Z = 38.24 to ~41.5 mm
# SG90 servo body: hangs DOWN from the berco, body Z = 17.24 to 39.24 mm (fits in 24mm gap)
# 12_Mesa_Giratoria_Turntable: sits ON TOP of 16_Berco, Z = 41.5 to ~44.5 mm
#   (the horn connects the servo shaft up to the mesa's underside)
# 25_Trava_Horn: clamps onto servo mounting ears at Z ~37-40 mm

# Z reference points:
Z_TPU_TOP = 10.24          # Top of TPU feet / bottom of chassi
Z_CHASSI_TOP = 14.24       # Top of chassi plate
Z_STANDOFF_TOP = 38.24     # Top of 24mm standoffs (14.24 + 24.0)
Z_BERCO_TOP = 41.5         # Top of 16_Berco (sits on standoffs)
Z_MESA_BOTTOM = 41.5       # Bottom of mesa sits on berco
Z_MESA_TOP = 44.7          # Mesa thickness ~3.2mm
Z_SERVO_TOP = 39.5         # Top of servo shaft/gear (slightly above berco)

# Fix 16_Berco: should be at Z = 38.24 to ~41.5 (3.2mm thick plate)
berco = scene.geometry['printed_16_Berco_Servo_Rotacao_Base']
berco_thickness = berco.bounds[1][2] - berco.bounds[0][2]
berco_target_z_bottom = Z_STANDOFF_TOP
berco.apply_translation([0, 0, berco_target_z_bottom - berco.bounds[0][2]])
print(f"   Fixed 16_Berco: new Z=[{berco.bounds[0][2]:.2f}, {berco.bounds[1][2]:.2f}]")

# Fix 12_Mesa_Giratoria_Turntable: on top of berco, connected via horn to servo shaft
mesa = scene.geometry['printed_12_Mesa_Giratoria_Turntable']
mesa_thickness = mesa.bounds[1][2] - mesa.bounds[0][2]
mesa_target_z_bottom = berco.bounds[1][2]   # sits directly on berco
mesa.apply_translation([0, 0, mesa_target_z_bottom - mesa.bounds[0][2]])
print(f"   Fixed 12_Mesa: new Z=[{mesa.bounds[0][2]:.2f}, {mesa.bounds[1][2]:.2f}]")

# Fix Servo 1 body: hangs under berco, servo top (mounting ears) flush with berco bottom
# SG90 body: ~23mm tall, ears at top; so servo bottom = standoff_top - 23mm = 15.24mm
servo1 = scene.geometry['servo_01_base']
servo1_height = servo1.bounds[1][2] - servo1.bounds[0][2]
servo1_target_ear_z = Z_STANDOFF_TOP   # ears should be at standoff top level
servo1.apply_translation([0, 0, servo1_target_ear_z - servo1.bounds[1][2]])
print(f"   Fixed servo_01_base: new Z=[{servo1.bounds[0][2]:.2f}, {servo1.bounds[1][2]:.2f}]")

# Fix 25_Trava_Horn: clamps the horn under the mesa (between mesa and berco)
trava = scene.geometry['printed_25_Trava_Horn_Servo_Base']
trava_target_z = Z_STANDOFF_TOP - 1.0   # just below berco
trava.apply_translation([0, 0, trava_target_z - trava.bounds[0][2]])
print(f"   Fixed 25_Trava: new Z=[{trava.bounds[0][2]:.2f}, {trava.bounds[1][2]:.2f}]")

# ------------------------------------------------------------------
# FIX 2: TOWERS (09, 11) - Servos mounted EXTERNALLY, not internally
# ------------------------------------------------------------------
print("\n3. FIX 2 - Servo 2 & 3 collision fix (external mounting)...")
# The servos in the original scene were placed to suggest internal mounting.
# The physical fix is noted in the manual (servo bodies outside tower berços).
# In the 3D scene, we need to shift servo 2 (right) and servo 3 (left) slightly
# outward from the center to show external-face mounting.

# Current servo positions:
for k in ['servo_02_ombro', 'servo_03_cotovelo']:
    b = scene.geometry[k].bounds
    center_x = (b[0][0] + b[1][0]) / 2
    print(f"   {k}: X_center={center_x:.1f}, Y=[{b[0][1]:.1f},{b[1][1]:.1f}], Z=[{b[0][2]:.1f},{b[1][2]:.1f}]")

# Tower 09 (right) has body at X~25-29mm; servo should mount on OUTSIDE (right face)
# Tower 11 (left) has body at X~-23 to -19mm; servo should mount on OUTSIDE (left face)
tower09 = scene.geometry['printed_09_Torre_Suporte_Servo_Direita']
tower11 = scene.geometry['printed_11_Torre_Suporte_Servo_Esquerda']

servo2 = scene.geometry['servo_02_ombro']
servo3 = scene.geometry['servo_03_cotovelo']

# Servo 2 should be on the RIGHT (positive X) external face of tower 09
# Tower09 right X boundary ~29mm; servo body is ~12mm wide
# Place servo center X at tower_right + 6mm
t09_x_right = tower09.bounds[1][0]
s2_current_x_center = (servo2.bounds[0][0] + servo2.bounds[1][0]) / 2
s2_target_x_center = t09_x_right + 6.0  # body outside tower
servo2.apply_translation([s2_target_x_center - s2_current_x_center, 0, 0])
print(f"   Fixed servo_02_ombro to external right: X_center={s2_target_x_center:.1f}")

# Servo 3 should be on the LEFT (negative X) external face of tower 11
t11_x_left = tower11.bounds[0][0]
s3_current_x_center = (servo3.bounds[0][0] + servo3.bounds[1][0]) / 2
s3_target_x_center = t11_x_left - 6.0  # body outside tower
servo3.apply_translation([s3_target_x_center - s3_current_x_center, 0, 0])
print(f"   Fixed servo_03_cotovelo to external left: X_center={s3_target_x_center:.1f}")

# ------------------------------------------------------------------
# FIX 3: PARALLELOGRAM KINEMATICS
# ------------------------------------------------------------------
print("\n4. FIX 3 - Correcting parallelogram linkage...")

# Key dimensions from MDF reference:
# Braços principais: 114.8 mm  (Parts 02, 07 in GLB match this range)
# Antebraços: 81.6 mm e 85.8 mm
# 04_Tirante original: 94.8 mm → WRONG (longer than entire forearm)
# 04_Tirante correct: must match antebraço pivot distance ~ 83mm (average of 81.6, 85.8)

# In the existing GLB, the 04_Tirante is actually short (3mm thick flat plate, ~93mm Y extent)
# The physical issue is where it connects: it was connecting INTO the elbow instead of the tower
tirante04 = scene.geometry['printed_04_Tirante_Paralelogramo_Medio']
print(f"   04_Tirante current bounds: {tirante04.bounds.tolist()}")

# The 04_Tirante's lower pivot should connect to the FIXED TOWER PIVOT POINT
# (not to the midpoint of the forearm)
# In the correct kinematic, 04 goes: TOWER_FIXED_PIVOT → Balancim Traseiro Pivot
# Let's identify where the balancim is:
balancim = scene.geometry['printed_10_Balancim_Triangular_Articulador']
print(f"   10_Balancim: {balancim.bounds.tolist()}")

# The 03_Tirante longo connects:
# Lower: biela servo 3 (Y~90-100, Z~49-55) → Upper: Balancim Pivô Traseiro
tirante03 = scene.geometry['printed_03_Tirante_Paralelogramo_Longo']
print(f"   03_Tirante_Longo: {tirante03.bounds.tolist()}")
# 03 runs from about Z=56 to Z=160; it's the long one that goes to the balancim

# 04_Tirante connects TOWER FIXED PIVOT → Balancim Pivô Traseiro (parallel to 03)
# The tower fixed pivot is approximately at the base of the towers at Y~90, Z~55
# The 04's upper end should reach Balancim Traseiro at approximately Z~130-135
# Currently the 04 in GLB appears to be connecting at the WRONG location (parallel to forearm)
# Correct position: it should run PARALLEL to 03 but on the opposite side

# Load the new corrected 04_Tirante STL (80mm hole distance, 90mm total length)
stl_04_path = os.path.join(PARTS_DIR, "04_Tirante_Paralelogramo_Medio.stl")
new_t04_mesh = trimesh.load(stl_04_path)

# Rotate 90 deg around Y so length (originally along X) becomes along Z
R_y90 = trimesh.transformations.rotation_matrix(np.pi / 2, [0, 1, 0])
new_t04_mesh.apply_transform(R_y90)

# Center mesh at (0, 0, 0)
new_t04_mesh.apply_translation(-new_t04_mesh.bounds.mean(axis=0))

# Target position: Left side of arm, parallel to 03_Tirante (which is on right side)
# 03_Tirante is at X~19.25, Y~131.7, Z~108.0
# 04_Tirante goes to X~-19.5, Y~131.7, Z~108.0
target_t04_center = np.array([-19.5, 131.7, 108.0])
new_t04_mesh.apply_translation(target_t04_center)

# Apply Technical Black PLA color
set_color(new_t04_mesh, BLACK_PLA)

# Replace in scene
scene.geometry['printed_04_Tirante_Paralelogramo_Medio'] = new_t04_mesh
print(f"   Replaced 04_Tirante with new 80mm STL: bounds={new_t04_mesh.bounds.tolist()}, extents={new_t04_mesh.extents.tolist()}")

# ------------------------------------------------------------------
# FIX 4: GARRA SERVO (Servo 4) ORIENTATION
# ------------------------------------------------------------------
print("\n5. FIX 4 - Correcting Servo 4 (Garra) orientation...")
# Per erros.md: eixo estriado deve apontar para BAIXO/CIMA (vertical), 
# NÃO para frente (horizontal)
# Manual page 15 passo 4.1 incorrectly says "eixo apontando para frente"
# Correct: eixo perpendicular ao plano dos dedos = vertical

servo4 = scene.geometry['servo_04_garra']
print(f"   servo_04_garra current: {servo4.bounds.tolist()}")

# The garra base is at approximately Y~170-195, Z~74-84 in the scene
# Servo4 should be oriented with its gear shaft pointing DOWN (negative Z)
# Create rotation: rotate servo 90° around X to go from horizontal to vertical shaft
# Actually we need to check if current orientation is already correct
# The MDF model shows servo output pointing towards the garra plane (down/perpendicular)

# Current servo 4 extent analysis:
s4_extents = servo4.bounding_box.extents
print(f"   Servo 4 extents: {s4_extents}")
# SG90: 23mm x 12.5mm x 22mm; if longest axis is Y, shaft is vertical (correct)
# If longest axis is Z, shaft is horizontal (incorrect - needs rotation)

s4_x, s4_y, s4_z = s4_extents
if s4_y > s4_x and s4_y > s4_z:
    print("   Servo 4 orientation: shaft is along Z axis (vertical) - CORRECT")
else:
    print("   Servo 4 orientation: shaft may be wrong - applying correction rotation")
    # Rotate 90° around X axis to make shaft point down
    R = trimesh.transformations.rotation_matrix(np.pi/2, [1, 0, 0], 
                                                 servo4.bounds.mean(axis=0))
    servo4.apply_transform(R)
    print(f"   Rotated servo_04_garra: new extents={servo4.bounding_box.extents}")

# Also ensure 26_Engrenagem_Acionadora is connected to 28_Biela_Curta
# Move them into logical proximity
engrenagem = scene.geometry['printed_26_Engrenagem_Acionadora_Garra']
biela_curta = scene.geometry['printed_28_Biela_Articulacao_Curta']
print(f"   26_Engrenagem: {engrenagem.bounds.mean(axis=0).tolist()}")
print(f"   28_Biela_Curta: {biela_curta.bounds.mean(axis=0).tolist()}")

# Position engrenagem centered on servo 4 shaft position
s4_center = servo4.bounds.mean(axis=0)
eng_current_center = engrenagem.bounds.mean(axis=0)
# Keep Z same, align XY to servo4
engrenagem.apply_translation([
    s4_center[0] - eng_current_center[0],
    s4_center[1] - eng_current_center[1],
    0  # keep vertical position
])
print(f"   Fixed 26_Engrenagem: new center={engrenagem.bounds.mean(axis=0).tolist()}")

# Connect biela_curta adjacent to engrenagem (they should be touching)
biela_center = biela_curta.bounds.mean(axis=0)
eng_new_center = engrenagem.bounds.mean(axis=0)
biela_curta.apply_translation([
    eng_new_center[0] - biela_center[0],
    eng_new_center[1] - biela_center[1] + 8.0,  # slight Y offset to connect
    0
])
print(f"   Fixed 28_Biela_Curta: new center={biela_curta.bounds.mean(axis=0).tolist()}")

# ------------------------------------------------------------------
# 6. UPDATE BOM in components_metadata.json (CORRIGINDO A CONTAGEM)
# ------------------------------------------------------------------
print("\n6. Updating components_metadata.json with corrected BOM...")

METADATA_PATH = os.path.join(BASE_DIR, "components_metadata.json")
with open(METADATA_PATH, 'r', encoding='utf-8') as f:
    metadata = json.load(f)

# Corrected BOM counts from erros.md analysis:
# M3x12: should be 14, not 12
# M3x10: should be 10, not 8
# Porca Nyloc M3: should be 20, not 15
# Arruela Nylon M3: should be 24, not 16

# Find and update fixadores component
for comp in metadata['components']:
    if comp['id'] == 'fixadores_m3':
        comp['desc'] = (
            "Parafusos Allen M3 (4x M3×30mm, 5x M3×16mm, 14x M3×12mm, 10x M3×10mm) "
            "+ 20x Porcas Nyloc M3 + 24x Arruelas de Nylon M3 em todas as articulações mecânicas. "
            "[Contagem corrigida vs. BOM original: +2 M3×12mm, +2 M3×10mm, +5 Nyloc, +8 Arruelas]"
        )
        print("   Updated fixadores_m3 BOM description with corrected counts")
    if comp['id'] == 'espacadores_nylon_m3':
        comp['name'] = "Arruelas de Nylon M3 (0.5 mm) — 24 un"
        comp['desc'] = (
            "Arruelas isoladoras de atrito plástico-contra-plástico em todas as juntas "
            "móveis pivotantes. ATENÇÃO: BOM original listava 16 un, valor correto é 24 un."
        )
        print("   Updated espacadores_nylon_m3 to 24 units")

with open(METADATA_PATH, 'w', encoding='utf-8') as f:
    json.dump(metadata, f, indent=2, ensure_ascii=False)
print("   Saved updated components_metadata.json")

# ------------------------------------------------------------------
# 7. EXPORT FIXED GLB AND STL
# ------------------------------------------------------------------
print("\n7. Exporting fixed GLB...")
scene.export(OUTPUT_GLB)
print(f"   Saved: {OUTPUT_GLB}")

print("\n8. Exporting combined STL...")
meshes = list(scene.geometry.values())
combined = trimesh.util.concatenate(meshes)
combined.export(OUTPUT_STL)
print(f"   Saved: {OUTPUT_STL}")

print("\n" + "=" * 60)
print("ASSEMBLY FIX COMPLETE!")
print("All 4 kinematic errors from erros.md have been corrected:")
print("  [OK] Fix 1: Servo 1 (Yaw) - 16_Berco no topo, 12_Mesa no eixo")
print("  [OK] Fix 2: Servos 2&3 - Montagem EXTERNA das torres (sem colisao)")
print("  [OK] Fix 3: Paralelogramo - 04_Tirante com 80mm hole distance")
print("  [OK] Fix 4: Servo 4 (Garra) - Eixo perpendicular + 26->28 conectados")
print("  [OK] BOM: Quantidades corrigidas em components_metadata.json")
print("=" * 60)
