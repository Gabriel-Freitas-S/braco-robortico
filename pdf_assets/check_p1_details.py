import zipfile
import xml.etree.ElementTree as ET
import numpy as np
import trimesh

with zipfile.ZipFile('Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'r') as z:
    cfg = z.read('Metadata/model_settings.config').decode('utf-8')
    root_cfg = ET.fromstring(cfg)
    obj_names = {}
    for obj in root_cfg.findall('object'):
        obj_id = obj.get('id')
        name = obj.find("./metadata[@key='name']").get('value')
        obj_names[obj_id] = name

    model_xml = z.read('3D/3dmodel.model').decode('utf-8')
    root_m = ET.fromstring(model_xml)
    ns = {'m': 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'}

    # Read components to object_*.model mapping
    comp_map = {}
    for obj in root_m.findall('.//m:object', ns):
        oid = obj.get('id')
        comp = obj.find('.//m:component', ns)
        if comp is not None:
            comp_map[oid] = comp.get('p:path') or comp.get('{http://schemas.microsoft.com/3dmanufacturing/production/2015/06}path')

    # Read items
    items = []
    for item in root_m.findall('.//m:item', ns):
        oid = item.get('objectid')
        name = obj_names.get(oid, f"obj_{oid}")
        trans_str = item.get('transform')
        mat = np.eye(4)
        if trans_str:
            vals = [float(x) for x in trans_str.split()]
            mat[:3, :3] = np.array(vals[:9]).reshape(3, 3) # Note: 3MF stores column-major or row-major
            mat[:3, 3] = vals[9:12]
        
        path = comp_map.get(oid)
        items.append({
            'oid': oid,
            'name': name,
            'path': path,
            'transform': mat
        })

    print(f"Loaded {len(items)} items from Prato 1.")
    for it in items:
        print(f"Item {it['oid']} ({it['name']}): path={it['path']}, translation={it['transform'][:3, 3]}")
