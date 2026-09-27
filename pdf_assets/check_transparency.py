from PIL import Image
import re
import os

with open('manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    html = f.read()

images = sorted(list(set(re.findall(r'src=["\']([^"\']+?\.(?:png|jpg|webp))["\']', html))))
print(f'Total unique images referenced in manual: {len(images)}')

non_transparent = []
transparent = []
for p in images:
    if not os.path.exists(p):
        print(f'File does not exist: {p}')
        continue
    try:
        im = Image.open(p)
        corners = [
            im.getpixel((0, 0)),
            im.getpixel((im.width - 1, 0)),
            im.getpixel((0, im.height - 1)),
            im.getpixel((im.width - 1, im.height - 1))
        ]
        is_transparent = False
        if im.mode == 'RGBA':
            alphas = [c[3] if isinstance(c, tuple) and len(c) == 4 else 255 for c in corners]
            if min(alphas) == 0:
                is_transparent = True
        
        if is_transparent:
            transparent.append(p)
        else:
            non_transparent.append((p, im.mode, corners[0]))
    except Exception as e:
        print(f'Error reading {p}: {e}')

print(f'Transparent count: {len(transparent)}')
print(f'Non-transparent count: {len(non_transparent)}')
for item in non_transparent:
    print('NON-TRANSPARENT:', item[0], item[1], item[2])
