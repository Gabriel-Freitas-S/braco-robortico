import trimesh
import numpy as np
import os
import json
import xml.etree.ElementTree as ET

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
RENDERS_DIR = os.path.join(BASE_DIR, "pdf_assets", "renders")
os.makedirs(RENDERS_DIR, exist_ok=True)

CYAN_COLOR = [0, 210, 255, 255]     # #00D2FF
CHARCOAL_COLOR = [34, 38, 44, 255]  # #22262C - Technical Black PLA

# ==============================================================================
# 1. BUILD EXPLODED ASSEMBLY GLB
# ==============================================================================
print("1. Creating Exploded Model GLB...")
complete_glb = trimesh.load(os.path.join(BASE_DIR, "braco_completo_com_componentes.glb"))

exploded_scene = trimesh.Scene()

# Offsets mapping (dx, dy, dz)
offsets = {
    # Base Chassis at bottom
    "printed_01_Base_Chassi_Inferior": (0, 0, -55),
    
    # Cross braces and horn lock
    "printed_17_Travessa_Reforco_Base_Frontal": (0, -25, -20),
    "printed_18_Travessa_Reforco_Base_Traseira": (0, 25, -20),
    "printed_25_Trava_Horn_Servo_Base": (0, 0, -10),
    
    # Turntable & servo cradle
    "printed_12_Mesa_Giratoria_Turntable": (0, 0, 15),
    "printed_16_Berco_Servo_Rotacao_Base": (0, 0, 30),
    
    # Towers & center spacer & servo connection arms
    "printed_09_Torre_Suporte_Servo_Direita": (25, 0, 65),
    "printed_11_Torre_Suporte_Servo_Esquerda": (-25, 0, 65),
    "printed_14_Separador_Espacador_Central_Base": (0, 0, 65),
    "printed_22_Biela_Conexao_Horn_Direito": (35, 15, 75),
    "printed_23_Biela_Conexao_Horn_Esquerdo": (-35, 15, 75),
    
    # Lower arm links
    "printed_02_Braco_Principal_Esquerdo": (-20, 0, 115),
    "printed_07_Braco_Principal_Direito": (20, 0, 115),
    "printed_05_Haste_Acionamento_Servo_Braco": (35, -15, 120),
    "printed_10_Balancim_Triangular_Articulador": (0, 0, 125),
    
    # Upper arm links
    "printed_03_Tirante_Paralelogramo_Longo": (20, -15, 170),
    "printed_04_Tirante_Paralelogramo_Medio": (-15, -15, 170),
    "printed_06_Antebraco_Esquerdo": (-15, 10, 185),
    "printed_08_Antebraco_Direito": (15, 10, 185),
    "printed_13_Tirante_Superior_Garra": (-25, 15, 155),
    
    # Claw head mount
    "printed_15_Suporte_Cabeca_Garra": (15, 25, 230),
    
    # Claw mechanism
    "printed_19_Garra_Base_Suporte_Fixo": (10, 65, 250),
    "printed_24_Suporte_Encaixe_Servo_Garra": (10, 50, 260),
    "printed_26_Engrenagem_Acionadora_Garra": (-5, 75, 250),
    "printed_27_Guia_Articulacao_Garra": (25, 60, 265),
    "printed_28_Biela_Articulacao_Curta": (0, 65, 255),
    "printed_20_Garra_Dedo_Movel_Direito": (25, 110, 250),
    "printed_21_Garra_Dedo_Movel_Esquerdo": (-10, 110, 250),
}

for name, geom in complete_glb.geometry.items():
    if name.startswith("printed_"):
        mesh = geom.copy()
        offset = offsets.get(name, (0, 0, 0))
        mesh.apply_translation(offset)
        
        # All printed parts in Black PLA
        mesh.visual = trimesh.visual.ColorVisuals(mesh=mesh, vertex_colors=[CHARCOAL_COLOR]*len(mesh.vertices))
        exploded_scene.add_geometry(mesh, node_name=name)

exploded_glb_path = os.path.join(BASE_DIR, "pdf_assets", "braco_exploded.glb")
exploded_scene.export(exploded_glb_path)
print(f"Exported exploded GLB to {exploded_glb_path} with {len(exploded_scene.geometry)} parts.")

# ==============================================================================
# 2. BUILD KNOLLED PLATES GLB
# ==============================================================================
print("\n2. Creating Knolled Plates GLB (Prato 1 + Prato 2)...")
knolled_scene = trimesh.Scene()

# Load Plate 1 and Plate 2 meshes from pratos directory
p1_path = os.path.join(BASE_DIR, "pratos", "Prato_1_Chassi_Estrutura_Bambu_A1.3mf")
p2_path = os.path.join(BASE_DIR, "pratos", "Prato_2_Mecanica_Garra_Bambu_A1.3mf")
if not os.path.exists(p1_path):
    p1_path = os.path.join(BASE_DIR, "Prato_1_Chassi_Estrutura_Bambu_A1.3mf")
if not os.path.exists(p2_path):
    p2_path = os.path.join(BASE_DIR, "Prato_2_Mecanica_Garra_Bambu_A1.3mf")

p1_3mf = trimesh.load(p1_path)
p2_3mf = trimesh.load(p2_path)

# Offsets for bed centers:
# Bed 1 at X = -160 mm
# Bed 2 at X = +160 mm
bed1 = trimesh.creation.box(extents=[256, 256, 2])
bed1.apply_translation([-160, 0, -1])
bed_gold_color = [228, 180, 48, 255] # Bambu Lab Gold Textured PEI
bed1.visual = trimesh.visual.ColorVisuals(mesh=bed1, vertex_colors=[bed_gold_color]*len(bed1.vertices))
knolled_scene.add_geometry(bed1, node_name="bed_plate_1")

bed2 = trimesh.creation.box(extents=[256, 256, 2])
bed2.apply_translation([160, 0, -1])
bed2.visual = trimesh.visual.ColorVisuals(mesh=bed2, vertex_colors=[bed_gold_color]*len(bed2.vertices))
knolled_scene.add_geometry(bed2, node_name="bed_plate_2")

m1_list = p1_3mf.dump()
m2_list = p2_3mf.dump()

for i, m in enumerate(m1_list):
    m_copy = m.copy()
    m_copy.apply_translation([-160 - 128.0, -128.0, 0])
    m_copy.visual = trimesh.visual.ColorVisuals(mesh=m_copy, vertex_colors=[CHARCOAL_COLOR]*len(m_copy.vertices))
    knolled_scene.add_geometry(m_copy, node_name=f"p1_part_{i}")

for i, m in enumerate(m2_list):
    m_copy = m.copy()
    m_copy.apply_translation([160 - 128.0, -128.0, 0])
    m_copy.visual = trimesh.visual.ColorVisuals(mesh=m_copy, vertex_colors=[CHARCOAL_COLOR]*len(m_copy.vertices))
    knolled_scene.add_geometry(m_copy, node_name=f"p2_part_{i}")

knolled_glb_path = os.path.join(BASE_DIR, "pdf_assets", "plates_knolled.glb")
knolled_scene.export(knolled_glb_path)
print(f"Exported knolled GLB to {knolled_glb_path} with {len(knolled_scene.geometry)} meshes.")
