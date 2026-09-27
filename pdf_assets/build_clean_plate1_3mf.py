import trimesh
import numpy as np
import os
import zipfile
import json
import xml.etree.ElementTree as ET

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
P1_3MF_PATH = os.path.join(BASE_DIR, "Prato_1_Chassi_Estrutura_Bambu_A1.3mf")

# Layout in bed coordinates (bed center is (128, 128)):
plate1_layout = [
    {"name": "01_Base_Chassi_Inferior",            "cx": 70.0,  "cy": 128.0, "rot": 0,  "color": "black"},
    {"name": "17_Travessa_Reforco_Base_Frontal",   "cx": 70.0,  "cy": 236.0, "rot": 0,  "color": "black"},
    {"name": "18_Travessa_Reforco_Base_Traseira",  "cx": 70.0,  "cy": 20.0,  "rot": 0,  "color": "black"},
    {"name": "12_Mesa_Giratoria_Turntable",        "cx": 168.0, "cy": 208.0, "rot": 0,  "color": "blue"},
    {"name": "25_Trava_Horn_Servo_Base",           "cx": 222.0, "cy": 208.0, "rot": 0,  "color": "black"},
    {"name": "09_Torre_Suporte_Servo_Direita",     "cx": 164.0, "cy": 138.0, "rot": 0,  "color": "black"},
    {"name": "11_Torre_Suporte_Servo_Esquerda",    "cx": 220.0, "cy": 138.0, "rot": 0,  "color": "black"},
    {"name": "16_Berco_Servo_Rotacao_Base",        "cx": 164.0, "cy": 63.0,  "rot": 0,  "color": "blue"},
    {"name": "14_Separador_Espacador_Central_Base","cx": 220.0, "cy": 63.0,  "rot": 90, "color": "blue"},
]

print("1. Loading Plate 1 meshes and preparing 3MF structure...")
# Generate 3D/3dmodel.model and individual object models
model_xml_lines = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">',
    ' <metadata name="Application">BambuStudio-02.08.02.61</metadata>',
    ' <metadata name="BambuStudio:3mfVersion">1</metadata>',
    ' <metadata name="BambuStudio:Version">02.08.02.61</metadata>',
    ' <resources>'
]

config_objects = []
object_files = {}
build_items = []

for idx, item in enumerate(plate1_layout, 1):
    obj_id = idx * 2
    part_id = obj_id - 1
    uuid_str = f"0000000{idx}-61cb-4c03-9d28-80fed5dfa1dc"
    comp_uuid = f"0001000{idx}-b206-40ff-9872-83e8017abed1"
    
    name = item["name"]
    stl_path = os.path.join(PARTS_DIR, f"{name}.stl")
    m = trimesh.load(stl_path)
    
    # Apply rotation if needed
    if item["rot"] != 0:
        R = trimesh.transformations.rotation_matrix(np.radians(item["rot"]), [0, 0, 1])
        m.apply_transform(R)
        
    # Center mesh at (0, 0) and place bottom at Z = 0
    b = m.bounds
    center_xy = (b[0][:2] + b[1][:2]) / 2.0
    m.apply_translation([-center_xy[0], -center_xy[1], -b[0][2]])
    
    # Save object_X.model XML
    obj_xml_lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">',
        ' <metadata name="BambuStudio:3mfVersion">1</metadata>',
        ' <resources>',
        f'  <object id="{part_id}" p:UUID="{uuid_str}" type="model">',
        '   <mesh>',
        '    <vertices>'
    ]
    for v in m.vertices:
        obj_xml_lines.append(f'     <vertex x="{v[0]:.6f}" y="{v[1]:.6f}" z="{v[2]:.6f}"/>')
    obj_xml_lines.append('    </vertices>')
    obj_xml_lines.append('    <triangles>')
    for f in m.faces:
        obj_xml_lines.append(f'     <triangle v1="{f[0]}" v2="{f[1]}" v3="{f[2]}"/>')
    obj_xml_lines.append('    </triangles>')
    obj_xml_lines.append('   </mesh>')
    obj_xml_lines.append('  </object>')
    obj_xml_lines.append(' </resources>')
    obj_xml_lines.append('</model>')
    
    obj_filename = f"3D/Objects/object_{idx}.model"
    object_files[obj_filename] = '\n'.join(obj_xml_lines).encode('utf-8')
    
    # Resource object definition in 3dmodel.model
    model_xml_lines.append(f'  <object id="{obj_id}" p:UUID="{uuid_str}" type="model">')
    model_xml_lines.append('   <components>')
    model_xml_lines.append(f'    <component p:path="/{obj_filename}" objectid="{part_id}" p:UUID="{comp_uuid}" transform="1 0 0 0 1 0 0 0 1 0 0 0"/>')
    model_xml_lines.append('   </components>')
    model_xml_lines.append('  </object>')
    
    # Build item placement transform
    tx = item["cx"]
    ty = item["cy"]
    tz = 0.0
    item_uuid = f"0000000{idx}-b1ec-4553-aec9-835e5b724bb4"
    build_items.append(f'<item objectid="{obj_id}" p:UUID="{item_uuid}" transform="1 0 0 0 1 0 0 0 1 {tx:.4f} {ty:.4f} {tz:.4f}" printable="1"/>')
    
    # Config object entry
    # Filament: 1 for Black, 2 for Blue
    filament_id = 1 if item["color"] == "black" else 2
    face_count = len(m.faces)
    config_objects.append(f"""  <object id="{obj_id}">
    <metadata key="name" value="{name}"/>
    <metadata key="extruder" value="{filament_id}"/>
    <metadata face_count="{face_count}"/>
    <part id="{part_id}" subtype="normal_part" uuid="{uuid_str}">
      <metadata key="name" value="{name}"/>
      <metadata key="matrix" value="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1"/>
      <metadata key="source_file" value="{name}.stl"/>
      <metadata key="source_object_id" value="{idx-1}"/>
      <metadata key="source_volume_id" value="0"/>
      <metadata key="source_offset_x" value="0"/>
      <metadata key="source_offset_y" value="0"/>
      <metadata key="source_offset_z" value="0"/>
      <mesh_stat face_count="{face_count}" edges_fixed="0" degenerate_facets="0" facets_removed="0" facets_reversed="0" backwards_edges="0"/>
    </part>
  </object>""")

model_xml_lines.append(' </resources>')
model_xml_lines.append(' <build>')
model_xml_lines.extend(['  ' + bi for bi in build_items])
model_xml_lines.append(' </build>')
model_xml_lines.append('</model>')

model_3d_content = '\n'.join(model_xml_lines).encode('utf-8')

# Read existing 3MF to preserve Bambu Studio metadata, profiles, thumbnails
print("2. Packing updated Prato_1_Chassi_Estrutura_Bambu_A1.3mf...")
temp_3mf = P1_3MF_PATH + ".temp"

with zipfile.ZipFile(P1_3MF_PATH, 'r') as zin, zipfile.ZipFile(temp_3mf, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
    for zinfo in zin.infolist():
        fn = zinfo.filename
        if fn == '3D/3dmodel.model':
            zout.writestr(fn, model_3d_content)
        elif fn.startswith('3D/Objects/object_'):
            if fn in object_files:
                zout.writestr(fn, object_files[fn])
        elif fn == 'Metadata/model_settings.config':
            # Update config with new object metadata
            cfg_content = '<?xml version="1.0" encoding="UTF-8"?>\n<config>\n' + '\n'.join(config_objects) + '\n</config>\n'
            zout.writestr(fn, cfg_content.encode('utf-8'))
        else:
            # Copy all other settings, profiles, filament config intact
            zout.writestr(fn, zin.read(fn))

os.replace(temp_3mf, P1_3MF_PATH)
print(f"SUCCESS: Prato_1_Chassi_Estrutura_Bambu_A1.3mf updated successfully! ({os.path.getsize(P1_3MF_PATH) // 1024} KB)")
