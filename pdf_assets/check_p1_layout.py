import zipfile
import xml.etree.ElementTree as ET
import numpy as np

with zipfile.ZipFile('Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'r') as z:
    cfg = z.read('Metadata/model_settings.config').decode('utf-8')
    root = ET.fromstring(cfg)
    obj_names = {}
    for obj in root.findall('object'):
        obj_id = obj.get('id')
        name = obj.find("./metadata[@key='name']").get('value')
        obj_names[obj_id] = name

    model_xml = z.read('3D/3dmodel.model').decode('utf-8')
    root_m = ET.fromstring(model_xml)
    ns = {'m': 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'}
    
    # Read build items
    for item in root_m.findall('.//m:item', ns):
        obj_id = item.get('objectid')
        name = obj_names.get(obj_id, f"obj_{obj_id}")
        transform = item.get('transform')
        mat = np.array([float(x) for x in transform.split()]).reshape(4, 3).T if transform else np.eye(4)
        print(f"Item {obj_id:2s} ({name:35s}): pos = ({mat[0, 3]:6.1f}, {mat[1, 3]:6.1f}, {mat[2, 3]:6.1f})")
