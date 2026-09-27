import re

with open('manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    html = f.read()

# Find pages
pages = re.findall(r'<div class="page(?: [^"]*)?" id="([^"]*)".*?<div class="page-header">.*?<h2>(.*?)</h2>', html, re.DOTALL)
print(f'Total pages found: {len(pages)}')
for pid, title in pages:
    t = re.sub(r'<[^>]+>', '', title).strip()
    print(f'  [{pid}] {t}')

# Find subassemblies and steps
steps = re.findall(r'class="step-header".*?<h3>(.*?)</h3>', html, re.DOTALL)
print(f'\nTotal steps found: {len(steps)}')
for s in steps:
    st = re.sub(r'<[^>]+>', '', s).strip()
    print(f'  -> {st}')
