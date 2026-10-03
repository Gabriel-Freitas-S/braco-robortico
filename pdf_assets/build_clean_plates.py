import trimesh
import numpy as np
import os
import zipfile
import json
import io
import xml.etree.ElementTree as ET

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
PRATOS_DIR = os.path.join(BASE_DIR, "pratos")

with open(os.path.join(BASE_DIR, "pdf_assets", "plate_layouts.json"), "r", encoding="utf-8") as f:
    layouts = json.load(f)

def build_plate_3mf(plate_key, out_3mf_path):
    items = layouts[plate_key]
    print(f"\n=== Building {plate_key} ({len(items)} parts) -> {os.path.basename(out_3mf_path)} ===")
    
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
    rels_lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    ]
    
    for idx, (name, x, y, rot) in enumerate(items, 1):
        obj_id = idx * 2
        part_id = obj_id - 1
        uuid_str = f"0000000{idx:02d}-61cb-4c03-9d28-80fed5dfa1dc"
        comp_uuid = f"0001000{idx:02d}-b206-40ff-9872-83e8017abed1"
        
        stl_path = os.path.join(PARTS_DIR, f"{name}.stl")
        if not os.path.exists(stl_path):
            raise FileNotFoundError(f"STL not found: {stl_path}")
            
        m = trimesh.load(stl_path)
        if rot != 0:
            R = trimesh.transformations.rotation_matrix(np.radians(rot), [0, 0, 1])
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
        object_files[obj_filename] = '\n'.join(obj_xml_lines).encode('utf-8')
        rels_lines.append(f' <Relationship Target="/{obj_filename}" Id="rel-{idx}" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>')
        
        model_xml_lines.append(f'  <object id="{obj_id}" p:UUID="{uuid_str}" type="model">')
        model_xml_lines.append('   <components>')
        model_xml_lines.append(f'    <component p:path="/{obj_filename}" objectid="{part_id}" p:UUID="{comp_uuid}" transform="1 0 0 0 1 0 0 0 1 0 0 0"/>')
        model_xml_lines.append('   </components>')
        model_xml_lines.append('  </object>')
        
        # Bed center is at (128, 128)
        tx = x + 128.0
        ty = y + 128.0
        tz = 0.0
        item_uuid = f"0000000{idx:02d}-b1ec-4553-aec9-835e5b724bb4"
        build_items.append(f'<item objectid="{obj_id}" p:UUID="{item_uuid}" transform="1 0 0 0 1 0 0 0 1 {tx:.4f} {ty:.4f} {tz:.4f}" printable="1"/>')
        
        face_count = len(m.faces)
        config_objects.append(f"""  <object id="{obj_id}">
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
        print(f"  [{idx:02d}] {name} at ({tx:.1f}, {ty:.1f})")

    rels_lines.append('</Relationships>')
    rels_content = '\n'.join(rels_lines).encode('utf-8')

    model_xml_lines.append(' </resources>')
    model_xml_lines.append(' <build>')
    model_xml_lines.extend(['  ' + bi for bi in build_items])
    model_xml_lines.append(' </build>')
    model_xml_lines.append('</model>')
    model_3d_content = '\n'.join(model_xml_lines).encode('utf-8')
    
    # Read existing 3mf as template
    with open(out_3mf_path, 'rb') as f:
        existing_zip = f.read()
        
    temp_path = out_3mf_path + ".tmp"
    with zipfile.ZipFile(io.BytesIO(existing_zip), 'r') as zin, zipfile.ZipFile(temp_path, 'w', compression=zipfile.ZIP_DEFLATED) as zout:
        for zinfo in zin.infolist():
            fn = zinfo.filename
            if fn == '3D/3dmodel.model':
                zout.writestr(fn, model_3d_content)
            elif fn == '3D/_rels/3dmodel.model.rels':
                zout.writestr(fn, rels_content)
            elif fn.startswith('3D/Objects/'):
                continue # Skip old object files, we write newly generated ones
            elif fn == 'Metadata/model_settings.config':
                cfg_content = '<?xml version="1.0" encoding="UTF-8"?>\n<config>\n' + '\n'.join(config_objects) + '\n</config>\n'
                zout.writestr(fn, cfg_content.encode('utf-8'))
            elif fn == 'Metadata/project_settings.config':
                p_cfg = json.loads(zin.read(fn).decode('utf-8'))
                p_cfg['filament_colour'] = ['#22262C']
                p_cfg['filament_type'] = ['PLA']
                p_cfg['filament_settings_id'] = ['Bambu PLA Basic @BBL A1']
                zout.writestr(fn, json.dumps(p_cfg, indent=4).encode('utf-8'))
            else:
                zout.writestr(fn, zin.read(fn))
                
        # Write new object model files
        for obj_fn, obj_data in object_files.items():
            zout.writestr(obj_fn, obj_data)
            
    os.replace(temp_path, out_3mf_path)
    print(f"  [x] Successfully saved {out_3mf_path} ({os.path.getsize(out_3mf_path) // 1024} KB)")

if __name__ == "__main__":
    p1_path = os.path.join(PRATOS_DIR, "Prato_1_Chassi_Estrutura_Bambu_A1.3mf")
    p2_path = os.path.join(PRATOS_DIR, "Prato_2_Mecanica_Garra_Bambu_A1.3mf")
    
    build_plate_3mf("plate1", p1_path)
    build_plate_3mf("plate2", p2_path)
    print("\nALL 2 PLATES SUCCESSFULLY BUILT FOR 100% BLACK PLA!")
