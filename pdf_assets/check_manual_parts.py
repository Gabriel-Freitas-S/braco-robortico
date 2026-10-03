import json
import re

with open(r'd:\braco-robortico\pdf_assets\slice_estimates.json', 'r', encoding='utf-8') as f:
    estimates = json.load(f)

with open(r'd:\braco-robortico\manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    html = f.read()

cards = re.findall(
    r'<div class="part-name">(\d{2}_[^\.]+)\.stl</div>.*?'
    r'<div class="spec-k">Peso PLA</div>\s*<div class="spec-v">([^<]+)</div>\s*</div>\s*'
    r'<div class="spec-item">\s*<div class="spec-k">Tempo Estimado</div>\s*<div class="spec-v">([^<]+)</div>',
    html,
    re.DOTALL
)

print(f"Found {len(cards)} part cards in manual HTML:")
mismatches = []
for name, peso, tempo in cards:
    if name in estimates:
        est = estimates[name]
        sec = est["time_s"]
        g = est["filament_g"]
        m = sec // 60
        calc_tempo = f"~{m} min" if m < 60 else f"~{m//60}h {m%60:02d}min"
        calc_peso = f"{g:.1f} g"
        
        p_clean = peso.strip()
        t_clean = tempo.strip()
        
        diff = []
        if p_clean != calc_peso:
            diff.append(f'Peso: manual="{p_clean}" vs calc="{calc_peso}"')
        if t_clean != calc_tempo:
            diff.append(f'Tempo: manual="{t_clean}" vs calc="{calc_tempo}"')
        if diff:
            mismatches.append((name, diff, calc_peso, calc_tempo))
            print(f"  [MISMATCH] {name}: " + " | ".join(diff))
        else:
            print(f"  [OK] {name}: {p_clean}, {t_clean}")
    else:
        print(f"  [NOT IN ESTIMATES] {name}")

print(f"\nTotal mismatches: {len(mismatches)}")
