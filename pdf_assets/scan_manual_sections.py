with open('manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for idx, line in enumerate(lines, 1):
    if any(k in line.lower() for k in ['subconjunto', 'lista de fixadores', 'tabela de fatiamento', 'parafuso m3', 'etapa 1', 'etapa 2', 'etapa 3', 'etapa 4', 'passo 1.', 'passo 2.', 'passo 3.', 'passo 4.']):
        print(f'{idx:4d}: {line.strip()[:100]}')
