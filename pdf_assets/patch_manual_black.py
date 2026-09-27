import re

HTML_PATH = r"d:\braco-robortico\manual_tecnico_braco_robotico.html"

with open(HTML_PATH, "r", encoding="utf-8") as f:
    content = f.read()

# 1. Update Hardware table on Page 3
content = re.sub(
    r'<td style="padding: 2.5px 6px;"><b>Filamento PLA Bicolor Técnico</b></td>\s*<td style="padding: 2.5px 6px;">Bambu PLA Basic Preto \(#22262C\) e Azul \(#0077D4\) 1.75mm</td>',
    r'<td style="padding: 2.5px 6px;"><b>Filamento PLA Monocromático Técnico</b></td>\n              <td style="padding: 2.5px 6px;">Bambu PLA Basic Preto (#22262C) 1.75mm (1 carretel 1kg é suficiente para todas as 28 peças)</td>',
    content
)

# 2. Update all 14 Blue Technical badges to Black Technical
content = content.replace(
    '<span style="font-weight: 700; color: #0077D4;">Azul Técnico • Bambu A1</span>',
    '<span style="font-weight: 700; color: #22262C;">Preto Técnico • Bambu A1</span>'
)

# 3. Update Step 1 to 4 Component lists
content = content.replace(
    '<div><b>Peças 3D:</b> 01 (Preto), 12 (Azul), 16 (Azul), 17 (Preto), 18 (Preto), 25 (Preto)</div>',
    '<div><b>Peças 3D:</b> 01, 12, 16, 17, 18, 25 (Todas em Bambu PLA Basic Preto)</div>'
)

content = content.replace(
    '<div><b>Peças 3D:</b> 09 (Preto), 11 (Preto), 12 (Azul), 14 (Azul), 22 (Azul), 23 (Azul)</div>',
    '<div><b>Peças 3D:</b> 09, 11, 12, 14, 22, 23 (Todas em Bambu PLA Basic Preto)</div>'
)

content = content.replace(
    '<div><b>Peças 3D:</b> 02 (Preto), 03 (Preto), 04 (Azul), 05 (Azul), 06 (Azul), 07 (Preto), 08 (Azul), 10 (Preto), 13 (Azul)</div>',
    '<div><b>Peças 3D:</b> 02, 03, 04, 05, 06, 07, 08, 10, 13 (Todas em Bambu PLA Basic Preto)</div>'
)

content = content.replace(
    '<div><b>Peças 3D:</b> 15 (Preto), 19 (Azul), 20 (Azul), 21 (Preto), 24 (Preto), 26 (Azul), 27 (Preto), 28 (Azul)</div>',
    '<div><b>Peças 3D:</b> 15, 19, 20, 21, 24, 26, 27, 28 (Todas em Bambu PLA Basic Preto)</div>'
)

# 4. Update Passo 4.1 text
content = content.replace(
    'Encaixe o quarto Servomotor SG90 no <b>24_Suporte_Encaixe_Servo_Garra</b> (Preto). Fixe as abas do motor com os micro parafusos originais. Una o suporte 24 à peça <b>19_Garra_Base_Suporte_Fixo</b> (Azul) utilizando 2 parafusos M3x10mm e porcas Nyloc.',
    'Encaixe o quarto Servomotor SG90 no <b>24_Suporte_Encaixe_Servo_Garra</b>. Fixe as abas do motor com os micro parafusos originais. Una o suporte 24 à peça <b>19_Garra_Base_Suporte_Fixo</b> utilizando 2 parafusos M3x10mm e porcas Nyloc.'
)

# 5. Update Passo 4.3 title & text
content = content.replace(
    'Sincronismo dos Dedos Móveis (Engrenamento Simétrico Preto & Azul)',
    'Sincronismo dos Dedos Móveis (Engrenamento Simétrico 20 & 21 com Identificação em Baixo-Relevo)'
)

content = re.sub(
    r'<b>PASSO CRÍTICO:</b> Posicione o <b>20_Garra_Dedo_Movel_Direito</b> \(Azul\) e o <b>21_Garra_Dedo_Movel_Esquerdo</b> \(Preto\) de modo que as engrenagens engrenem perfeitamente sincronizadas.*?confirmar visualmente o alinhamento\.',
    r'<b>PASSO CRÍTICO:</b> Posicione o <b>20_Garra_Dedo_Movel_Direito</b> e o <b>21_Garra_Dedo_Movel_Esquerdo</b> (identificados pelos números em baixo-relevo gravados em cada peça) de modo que as engrenagens engrenem perfeitamente sincronizadas com a mandíbula fechada no plano central simétrico.',
    content,
    flags=re.DOTALL
)

# 6. Update Page 11 plate summaries if needed
content = content.replace(
    '82.6 g PLA',
    '82.8 g PLA'
)
content = content.replace(
    '32.4 g PLA',
    '32.6 g PLA'
)
content = content.replace(
    '~115.0 g PLA',
    '~115.3 g PLA'
)
content = content.replace(
    '~3h 55min',
    '~4h 01min'
)
content = content.replace(
    '~1h 35min',
    '~1h 38min'
)
content = content.replace(
    '~2h 20min',
    '~2h 23min'
)

with open(HTML_PATH, "w", encoding="utf-8") as f:
    f.write(content)

print("SUCCESS: manual_tecnico_braco_robotico.html updated to 100% Black PLA!")
