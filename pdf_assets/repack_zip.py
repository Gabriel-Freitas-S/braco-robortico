import zipfile
import os

BASE_DIR = r"D:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pecas_impressao_3d")
ZIP_PATH = os.path.join(BASE_DIR, "braco-robotico-bambu-a1-stls.zip")

stls = sorted([f for f in os.listdir(PARTS_DIR) if f.endswith('.stl')])
print(f"Total STL files in pecas_impressao_3d: {len(stls)}")

temp_zip = ZIP_PATH + ".temp"
with zipfile.ZipFile(temp_zip, 'w', compression=zipfile.ZIP_DEFLATED) as z:
    for stl in stls:
        full_path = os.path.join(PARTS_DIR, stl)
        z.write(full_path, arcname=stl)
        print(f"  Added {stl} ({os.path.getsize(full_path)} bytes)")

os.replace(temp_zip, ZIP_PATH)
print(f"\nSUCCESS: Re-packed {ZIP_PATH} with all {len(stls)} updated STLs! ({os.path.getsize(ZIP_PATH) // 1024} KB)")
