import os
import sys
import glob
import json
import time
from pathlib import Path

# Add bambu-studio-ai scripts to path
sys.path.insert(0, r"D:\braco-robortico\.agents\skills\bambu-studio-ai\scripts")
from bambu_studio_ai.slicing import (
    find_cli,
    find_profiles_dir,
    ProfileLibrary,
    PrintRequest,
    resolve_profiles,
    SliceJob,
    run_slice,
)

BASE_DIR = Path(r"D:\braco-robortico")
PARTS_DIR = BASE_DIR / "pecas_impressao_3d"
FATIADO_DIR = PARTS_DIR / "fatiado"
ESTIMATES_JSON = BASE_DIR / "pdf_assets" / "slice_estimates.json"
README_MD = FATIADO_DIR / "README.md"

FATIADO_DIR.mkdir(parents=True, exist_ok=True)

print("1. Localizando Bambu Studio CLI e perfis...")
cli = find_cli()
print(f"CLI: {cli[0]}")
lib = ProfileLibrary(find_profiles_dir())
req = PrintRequest(printer="A1", nozzle_mm=0.4, material="PLA", quality="standard")
choice = resolve_profiles(lib, req)

machine_dict = lib.flatten("machine", choice.machine)
process_dict = lib.flatten("process", choice.process)
filament_dict = lib.flatten("filament", choice.filament)
# Garantir cor preto para visualizador do Bambu Studio
filament_dict["default_filament_colour"] = ["#22262C"]

stl_files = sorted(glob.glob(str(PARTS_DIR / "*.stl")))
print(f"Total de STLs encontrados: {len(stl_files)}")

# Carregar estimates existentes para preservar os pratos
if ESTIMATES_JSON.exists():
    with open(ESTIMATES_JSON, "r", encoding="utf-8") as f:
        estimates = json.load(f)
else:
    estimates = {}

results = []
total_time_s = 0
total_filament_g = 0.0

print("\n2. Fatiando todas as 28 peças individualmente...")
for idx, stl_path in enumerate(stl_files, 1):
    stl_path = Path(stl_path)
    stem = stl_path.stem
    output_3mf = FATIADO_DIR / f"{stem}_sliced.3mf"
    
    job = SliceJob(
        model=stl_path,
        output=output_3mf,
        timeout_s=120,
        bed_type=choice.bed_type,
        machine=machine_dict,
        process=process_dict,
        filament=filament_dict,
    )
    
    t0 = time.time()
    res = run_slice(cli, job)
    dt = time.time() - t0
    
    sec = int(res.estimate.print_time_s)
    g = round(res.estimate.filament_g, 2)
    m = sec // 60
    s = sec % 60
    
    total_time_s += sec
    total_filament_g += g
    
    estimates[stem] = {
        "time_s": sec,
        "filament_g": g,
        "warnings": list(res.estimate.warnings)
    }
    
    results.append({
        "num": stem[:2],
        "name": stem[3:],
        "fullname": stem,
        "time_s": sec,
        "filament_g": g,
        "time_str": f"{m//60}h {m%60:02d}min" if m >= 60 else f"{m} min"
    })
    
    print(f"[{idx:02d}/28] {stem}: {m}m {s:02d}s ({sec}s) | {g:.2f}g | Fatiado em {dt:.2f}s")

# Salvar json atualizado
with open(ESTIMATES_JSON, "w", encoding="utf-8") as f:
    json.dump(estimates, f, indent=2, ensure_ascii=False)
print(f"\nSalvo estimativas atualizadas em: {ESTIMATES_JSON}")

# Gerar README.md atualizado
total_h = total_time_s // 3600
total_m = (total_time_s % 3600) // 60
total_time_str = f"{total_h}h {total_m:02d}min"

readme_lines = [
    "# 🖨️ Arquivos Fatiados — Peças Individuais (Bambu Lab A1)",
    "",
    "Contém os 28 arquivos `.3mf` prontos para enviar diretamente à impressora Bambu Lab A1.",
    "Cada arquivo foi fatiado com:",
    "- **Perfil:** `0.20mm Standard @BBL A1`",
    "- **Bico:** 0.4 mm · **Material:** Bambu PLA Basic Preto (`#22262C`) · **Prato:** Textured PEI",
    "",
    "> Os STLs originais (com numeração em relevo baixo) estão na pasta pai `pecas_impressao_3d/`.",
    "",
    "## Tempos e Filamento por Peça",
    "",
    "| Peça | Nome | Tempo | Filamento |",
    "|------|------|-------|-----------|"
]

for r in results:
    readme_lines.append(f"| {r['num']} | {r['name']} | {r['time_str']} | {r['filament_g']:.1f} g |")

readme_lines.extend([
    f"| **TOTAL** | **28 peças** | **{total_time_str}** | **{total_filament_g:.1f} g** |",
    "",
    "*Estimativas geradas por Bambu Studio CLI oficial com perfil padrão 0.20mm e Bambu PLA Basic Preto.*",
    ""
])

with open(README_MD, "w", encoding="utf-8") as f:
    f.write("\n".join(readme_lines))
print(f"Salvo README atualizado em: {README_MD}")
print(f"\nResumo Total Individual: {total_time_str} | {total_filament_g:.1f} g PLA")
