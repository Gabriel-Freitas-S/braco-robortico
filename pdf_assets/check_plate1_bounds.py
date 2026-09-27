import zipfile
import xml.etree.ElementTree as ET
import numpy as np
import trimesh

def load_mesh_from_model_xml(xml_content):
    root = ET.fromstring(xml_content)
    ns = {'m': 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'}
    v_elems = root.findall('.//m:vertex', ns)
    vertices = np.array([[float(v.get('x')), float(v.get('y')), float(v.get('z'))] for v in v_elems])
    t_elems = root.findall('.//m:triangle', ns)
    faces = np.array([[int(t.get('v1')), int(t.get('v2')), int(t.get('v3'))] for t in t_elems])
    return trimesh.Trimesh(vertices=vertices, faces=faces)

with zipfile.ZipFile('Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'r') as z:
    cfg = z.read('Metadata/model_settings.config').decode('utf-8')
    root_cfg = ET.fromstring(cfg)
    obj_names = {obj.get('id'): obj.find("./metadata[@key='name']").get('value') for obj in root_cfg.findall('object')}

    model_xml = z.read('3D/3dmodel.model').decode('utf-8')
    root_m = ET.fromstring(model_xml)
    ns = {'m': 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'}

    comp_map = {}
    for obj in root_m.findall('.//m:object', ns):
        oid = obj.get('id')
        comp = obj.find('.//m:component', ns)
        if comp is not None:
            p = comp.get('p:path') or comp.get('{http://schemas.microsoft.com/3dmanufacturing/production/2015/06}path')
            comp_map[oid] = p.lstrip('/')

    meshes = {}
    for item in root_m.findall('.//m:item', ns):
        oid = item.get('objectid')
        name = obj_names.get(oid, f"obj_{oid}")
        path = comp_map[oid]
        
        # Parse transform
        trans_str = item.get('transform')
        mat = np.eye(4)
        if trans_str:
            vals = [float(x) for x in trans_str.split()]
            # 3MF affine transform matrix: 3x3 rotation + translation column
            R = np.array([[vals[0], vals[3], vals[6]],
                          [vals[1], vals[4], vals[7]],
                          [vals[2], vals[5], vals[8]]])
            T = np.array(vals[9:12])
            mat[:3, :3] = R
            mat[:3, 3] = T
            
        if name == '01_Base_Chassi_Inferior':
            # Load the NEW base STL
            m = trimesh.load(r'd:\braco-robortico\cad-output\01_Base_Chassi_Inferior.stl')
        else:
            xml_bytes = z.read(path)
            m = load_mesh_from_model_xml(xml_bytes)
            
        m.apply_transform(mat)
        meshes[name] = m
        b = m.bounds
        print(f"{name:35s}: X=[{b[0][0]:6.1f}, {b[1][0]:6.1f}], Y=[{b[0][1]:6.1f}, {b[1][1]:6.1f}], Z=[{b[0][2]:5.1f}, {b[1][2]:5.1f}]")
