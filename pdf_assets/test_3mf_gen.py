import trimesh
import zipfile
import xml.etree.ElementTree as ET
import os

stl_path = r"d:\braco-robortico\cad-output\01_Base_Chassi_Inferior.stl"
base_mesh = trimesh.load(stl_path)

# Let's inspect the header of object_1.model from the original 3MF
with zipfile.ZipFile('Prato_1_Chassi_Estrutura_Bambu_A1.3mf', 'r') as z:
    orig_obj1 = z.read('3D/Objects/object_1.model').decode('utf-8')

# The 3MF XML format:
# <model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">
#  <metadata name="BambuStudio:3mfVersion">1</metadata>
#  <resources>
#   <object id="1" p:UUID="..." type="model">
#    <mesh>
#     <vertices>
#      <vertex x="..." y="..." z="..." />
# ...
#     </vertices>
#     <triangles>
#      <triangle v1="..." v2="..." v3="..." />
# ...

# Let's extract the UUID from the original
import re
uuid_match = re.search(r'p:UUID="([^"]+)"', orig_obj1)
uuid_val = uuid_match.group(1) if uuid_match else "00010000-81cb-4c03-9d28-80fed5dfa1dc"

# Build new object_1.model
lines = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021" xmlns:p="http://schemas.microsoft.com/3dmanufacturing/production/2015/06" requiredextensions="p">',
    ' <metadata name="BambuStudio:3mfVersion">1</metadata>',
    ' <resources>',
    f'  <object id="1" p:UUID="{uuid_val}" type="model">',
    '   <mesh>',
    '    <vertices>'
]

# Simplify mesh slightly to keep 3MF lightweight if needed, or use full precision
# base_mesh has ~55k vertices
for v in base_mesh.vertices:
    lines.append(f'     <vertex x="{v[0]:.6f}" y="{v[1]:.6f}" z="{v[2]:.6f}"/>')

lines.append('    </vertices>')
lines.append('    <triangles>')

for f in base_mesh.faces:
    lines.append(f'     <triangle v1="{f[0]}" v2="{f[1]}" v3="{f[2]}"/>')

lines.append('    </triangles>')
lines.append('   </mesh>')
lines.append('  </object>')
lines.append(' </resources>')
lines.append('</model>')

new_obj1_content = '\n'.join(lines).encode('utf-8')
print(f"Generated new object_1.model: {len(new_obj1_content)} bytes")
