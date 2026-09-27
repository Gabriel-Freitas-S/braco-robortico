import zipfile
import xml.etree.ElementTree as ET

with zipfile.ZipFile('Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'r') as z:
    cfg = z.read('Metadata/model_settings.config').decode('utf-8')
    root = ET.fromstring(cfg)
    for obj in root.findall('object'):
        obj_id = obj.get('id')
        name = obj.find("./metadata[@key='name']").get('value')
        part = obj.find('part')
        src_file = part.find("./metadata[@key='source_file']").get('value') if part.find("./metadata[@key='source_file']") is not None else None
        src_obj_id = part.find("./metadata[@key='source_object_id']").get('value') if part.find("./metadata[@key='source_object_id']") is not None else None
        print(f"Object {obj_id}: name={name}, src_obj_id={src_obj_id}")

    # Inspect 3D/3dmodel.model
    model_xml = z.read('3D/3dmodel.model').decode('utf-8')
    print("\n3D/3dmodel.model snippet:")
    print(model_xml[:1500])
