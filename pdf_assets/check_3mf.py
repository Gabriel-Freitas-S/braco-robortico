import zipfile
import re

for p in ['Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'Prato_2_Mecanica_Garra_Bambu_A1.3mf']:
    print(f"=== {p} ===")
    with zipfile.ZipFile(p, 'r') as z:
        if 'Metadata/model_settings.config' in z.namelist():
            cfg = z.read('Metadata/model_settings.config').decode('utf-8')
            names = re.findall(r'key="name" value="(.*?)"', cfg)
            print(f"Total parts: {len(names)}, names:")
            for n in names:
                print("  ", n)
