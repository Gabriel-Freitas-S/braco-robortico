import trimesh
import numpy as np
import os
import zipfile
import xml.etree.ElementTree as ET

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
PRATOS_DIR = os.path.join(BASE_DIR, "pratos")

# ==============================================================================
# 1. UPDATE PLATE 1 (9 PARTS)
# ==============================================================================
print("=== 1. Updating Plate 1 (Chassi & Estrutura) ===")
p1_3mf_path = os.path.join(PRATOS_DIR, "Prato_1_Chassi_Estrutura_Bambu_A1.3mf")

plate1_layout = [
    {"name": "01_Base_Chassi_Inferior",            "cx": 70.0,  "cy": 128.0, "rot": 0},
    {"name": "17_Travessa_Reforco_Base_Frontal",   "cx": 70.0,  "cy": 236.0, "rot": 0},
    {"name": "18_Travessa_Reforco_Base_Traseira",  "cx": 70.0,  "cy": 20.0,  "rot": 0},
    {"name": "12_Mesa_Giratoria_Turntable",        "cx": 168.0, "cy": 208.0, "rot": 0},
    {"name": "25_Trava_Horn_Servo_Base",           "cx": 222.0, "cy": 208.0, "rot": 0},
    {"name": "09_Torre_Suporte_Servo_Direita",     "cx": 164.0, "cy": 138.0, "rot": 0},
    {"name": "11_Torre_Suporte_Servo_Esquerda",    "cx": 220.0, "cy": 138.0, "rot": 0},
    {"name": "16_Berco_Servo_Rotacao_Base",        "cx": 164.0, "cy": 63.0,  "rot": 0},
    {"name": "14_Separador_Espacador_Central_Base","cx": 220.0, "cy": 63.0,  "rot": 90},
]

model_xml_lines_1 = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">',
    ' <metadata name="Application">BambuStudio-02.08.02.61</metadata>',
    ' <metadata name="BambuStudio:3mfVersion">1</metadata>',
    ' <metadata name="BambuStudio:Version">02.08.02.61</metadata>',
    ' <resources>'
]

config_objects_1 = []
object_files_1 = {}
build_items_1 = []

for idx, item in enumerate(plate1_layout, 1):
    obj_id = idx * 2
    part_id = obj_id - 1
    uuid_str = f"0000000{idx}-61cb-4c03-9d28-80fed5dfa1dc"
    comp_uuid = f"0001000{idx}-b206-40ff-9872-83e8017abed1"
    
    name = item["name"]
    stl_path = os.path.join(PARTS_DIR, f"{name}.stl")
    m = trimesh.load(stl_path)
    
    if item["rot"] != 0:
        R = trimesh.transformations.rotation_matrix(np.radians(item["rot"]), [0, 0, 1])
        m.apply_transform(R)
        
    b = m.bounds
    center_xy = (b[0][:2] + b[1][:2]) / 2.0
    m.apply_translation([-center_xy[0], -center_xy[1], -b[0][2]])
    
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
    object_files_1[obj_filename] = '\n'.join(obj_xml_lines).encode('utf-8')
    
    model_xml_lines_1.append(f'  <object id="{obj_id}" p:UUID="{uuid_str}" type="model">')
    model_xml_lines_1.append('   <components>')
    model_xml_lines_1.append(f'    <component p:path="/{obj_filename}" objectid="{part_id}" p:UUID="{comp_uuid}" transform="1 0 0 0 1 0 0 0 1 0 0 0"/>')
    model_xml_lines_1.append('   </components>')
    model_xml_lines_1.append('  </object>')
    
    tx = item["cx"]
    ty = item["cy"]
    tz = 0.0
    item_uuid = f"0000000{idx}-b1ec-4553-aec9-835e5b724bb4"
    build_items_1.append(f'<item objectid="{obj_id}" p:UUID="{item_uuid}" transform="1 0 0 0 1 0 0 0 1 {tx:.4f} {ty:.4f} {tz:.4f}" printable="1"/>')
    
    face_count = len(m.faces)
    config_objects_1.append(f"""  <object id="{obj_id}">
    <metadata key="name" value="{name}"/>
    <metadata key="extruder" value="1"/>
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

model_xml_lines_1.append(' </resources>')
model_xml_lines_1.append(' <build>')
model_xml_lines_1.extend(['  ' + bi for bi in build_items_1])
model_xml_lines_1.append(' </build>')
model_xml_lines_1.append('</model>')

model_3d_content_1 = '\n'.join(model_xml_lines_1).encode('utf-8')

with open(p1_3mf_path, 'rb') as f_in:
    zin_bytes = f_in.read()

import io
with zipfile.ZipFile(io.BytesIO(zin_bytes), 'r') as zin, open(p1_3mf_path, 'wb') as f_out:
    with zipfile.ZipFile(f_out, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
        for zinfo in zin.infolist():
            fn = zinfo.filename
            if fn == '3D/3dmodel.model':
                zout.writestr(fn, model_3d_content_1)
            elif fn.startswith('3D/Objects/object_'):
                if fn in object_files_1:
                    zout.writestr(fn, object_files_1[fn])
            elif fn == 'Metadata/model_settings.config':
                cfg_content = '<?xml version="1.0" encoding="UTF-8"?>\n<config>\n' + '\n'.join(config_objects_1) + '\n</config>\n'
                zout.writestr(fn, cfg_content.encode('utf-8'))
            else:
                zout.writestr(fn, zin.read(fn))

print(f"  [x] Prato 1 updated with debossed STLs (Extruder 1 / Black PLA): {os.path.getsize(p1_3mf_path) // 1024} KB")

# ==============================================================================
# 2. UPDATE PLATE 2 (19 PARTS)
# ==============================================================================
print("\n=== 2. Updating Plate 2 (Mecanica & Garra) ===")
p2_3mf_path = os.path.join(PRATOS_DIR, "Prato_2_Mecanica_Garra_Bambu_A1.3mf")

# Read existing Plate 2 config and models to map object files to part names
with zipfile.ZipFile(p2_3mf_path, 'r') as zin:
    cfg_raw = zin.read('Metadata/model_settings.config').decode('utf-8')
    model_3d_raw = zin.read('3D/3dmodel.model').decode('utf-8')

root_cfg = ET.fromstring(cfg_raw)
obj_to_name = {}
for obj in root_cfg.findall('object'):
    oid = obj.attrib['id']
    for md in obj.findall('metadata'):
        if md.attrib.get('key') == 'name':
            obj_to_name[oid] = md.attrib.get('value')

# Parse components in 3dmodel.model to find which object_X.model belongs to which object
root_m = ET.fromstring(model_3d_raw)
ns = {'m': 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02', 'p': 'http://schemas.microsoft.com/3dmanufacturing/production/2015/06'}

object_file_to_name = {}
for obj in root_m.findall('.//m:object', ns):
    oid = obj.attrib.get('id')
    part_name = obj_to_name.get(oid)
    if not part_name:
        continue
    comp = obj.find('.//m:component', ns)
    if comp is not None:
        path = comp.attrib.get('{http://schemas.microsoft.com/3dmanufacturing/production/2015/06}path', '')
        if path.startswith('/'):
            path = path[1:]
        object_file_to_name[path] = (part_name, oid)

print(f"  Mapped {len(object_file_to_name)} object files in Plate 2:")

object_files_2 = {}
updated_cfg_objects = []

for fn, (name, oid) in object_file_to_name.items():
    stl_path = os.path.join(PARTS_DIR, f"{name}.stl")
    if not os.path.exists(stl_path):
        print(f"  Warning: STL not found for {name}: {stl_path}")
        continue
        
    m = trimesh.load(stl_path)
    # Center mesh at (0, 0, 0)
    b = m.bounds
    center = (b[0] + b[1]) / 2.0
    m.apply_translation(-center)
    
    # Read part_id and uuid from existing object model in 3mf
    with zipfile.ZipFile(p2_3mf_path, 'r') as zin:
        old_obj_xml = zin.read(fn).decode('utf-8')
    root_old = ET.fromstring(old_obj_xml)
    old_obj = root_old.find('.//m:object', ns)
    part_id = old_obj.attrib.get('id', '1')
    uuid_str = old_obj.attrib.get('{http://schemas.microsoft.com/3dmanufacturing/production/2015/06}UUID', '')
    
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
    
    object_files_2[fn] = '\n'.join(obj_xml_lines).encode('utf-8')
    face_count = len(m.faces)
    print(f"    [x] {fn} -> {name} ({face_count} faces)")

# Update model_settings.config for Plate 2: extruder=1 for ALL objects
for obj in root_cfg.findall('object'):
    for md in obj.findall('metadata'):
        if md.attrib.get('key') == 'extruder':
            md.attrib['value'] = '1'
        if md.attrib.get('key') == 'name':
            pname = md.attrib.get('value')
            stl_p = os.path.join(PARTS_DIR, f"{pname}.stl")
            if os.path.exists(stl_p):
                fc = len(trimesh.load(stl_p).faces)
                obj.find('part').attrib['uuid'] = obj.find('part').attrib.get('uuid', '')
                ms = obj.find('.//mesh_stat')
                if ms is not None:
                    ms.attrib['face_count'] = str(fc)

cfg_content_2 = ET.tostring(root_cfg, encoding='utf-8', xml_declaration=True)

with open(p2_3mf_path, 'rb') as f_in2:
    zin_bytes_2 = f_in2.read()

with zipfile.ZipFile(io.BytesIO(zin_bytes_2), 'r') as zin, open(p2_3mf_path, 'wb') as f_out2:
    with zipfile.ZipFile(f_out2, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
        for zinfo in zin.infolist():
            fn = zinfo.filename
            if fn in object_files_2:
                zout.writestr(fn, object_files_2[fn])
            elif fn == 'Metadata/model_settings.config':
                zout.writestr(fn, cfg_content_2)
            else:
                zout.writestr(fn, zin.read(fn))

print(f"  [x] Prato 2 updated with debossed STLs (Extruder 1 / Black PLA): {os.path.getsize(p2_3mf_path) // 1024} KB")

# Copy updated plates to root if they exist there
for p_name in ["Prato_1_Chassi_Estrutura_Bambu_A1.3mf", "Prato_2_Mecanica_Garra_Bambu_A1.3mf"]:
    src = os.path.join(PRATOS_DIR, p_name)
    dst = os.path.join(BASE_DIR, p_name)
    if os.path.exists(dst):
        import shutil
        shutil.copy2(src, dst)
        print(f"  [x] Copied {src} -> {dst}")

print("\nSUCCESS: Both plates 1 & 2 fully refreshed with debossed STLs and configured for 100% Black PLA!")
