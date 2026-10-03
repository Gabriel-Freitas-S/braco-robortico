import os
import re
import json
import trimesh

with open(r'd:\braco-robortico\pdf_assets\slice_estimates.json', 'r', encoding='utf-8') as f:
    estimates = json.load(f)

with open(r'd:\braco-robortico\manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    html = f.read()

parts_dir = r"d:\braco-robortico\pecas_impressao_3d"

# Match each part card block
pattern = re.compile(
    r'(<div class="part-name">(\d{2}_[^\.]+)\.stl</div>.*?'
    r'<div class="spec-k">Dimensões \(XYZ\)</div>\s*<div class="spec-v">([^<]+)</div>\s*</div>\s*'
    r'<div class="spec-item">\s*<div class="spec-k">Peso PLA</div>\s*<div class="spec-v">([^<]+)</div>\s*</div>\s*'
    r'<div class="spec-item">\s*<div class="spec-k">Tempo Estimado</div>\s*<div class="spec-v">([^<]+)</div>.*?'
    r'</div>\s*</div>\s*</div>)',
    re.DOTALL
)

matches = pattern.findall(html)
print(f"Total part cards matched: {len(matches)}")

for block, stem, dims, peso, tempo in matches:
    stl_path = os.path.join(parts_dir, f"{stem}.stl")
    if not os.path.exists(stl_path):
        print(f"STL NOT FOUND: {stem}")
        continue
    
    m = trimesh.load(stl_path)
    # Extents
    ext = sorted(m.extents, reverse=True)
    real_dims_sorted = f"{ext[0]:.1f} × {ext[1]:.1f} × {ext[2]:.1f} mm"
    
    est = estimates.get(stem, {})
    sec = est.get("time_s", 0)
    g = est.get("filament_g", 0.0)
    m_min = sec // 60
    real_tempo = f"~{m_min} min" if m_min < 60 else f"~{m_min//60}h {m_min%60:02d}min"
    real_peso = f"{g:.1f} g"
    
    print(f"--- {stem} ---")
    print(f"  Dims:  manual='{dims.strip()}' | actual extents (sorted)='{real_dims_sorted}' (raw: {m.extents[0]:.1f}x{m.extents[1]:.1f}x{m.extents[2]:.1f})")
    print(f"  Peso:  manual='{peso.strip()}' | actual='{real_peso}'")
    print(f"  Tempo: manual='{tempo.strip()}' | actual='{real_tempo}'")
