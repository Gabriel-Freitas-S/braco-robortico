import re

HTML_PATH = r"d:\braco-robortico\manual_tecnico_braco_robotico.html"

with open(HTML_PATH, "r", encoding="utf-8") as f:
    html = f.read()

# Replace tag pills
html = html.replace(
    'style="background: #0077D4; color: #ffffff; font-size: 7.5px; font-weight: 800; padding: 1.5px 5px; border-radius: 4px; text-transform: uppercase;">PRATO 1 • AZUL</span>',
    'style="background: #22262C; color: #ffffff; font-size: 7.5px; font-weight: 800; padding: 1.5px 5px; border-radius: 4px; text-transform: uppercase;">PRATO 1 • PRETO</span>'
)

html = html.replace(
    'style="background: #0077D4; color: #ffffff; font-size: 7.5px; font-weight: 800; padding: 1.5px 5px; border-radius: 4px; text-transform: uppercase;">PRATO 2 • AZUL</span>',
    'style="background: #22262C; color: #ffffff; font-size: 7.5px; font-weight: 800; padding: 1.5px 5px; border-radius: 4px; text-transform: uppercase;">PRATO 2 • PRETO</span>'
)

# Replace border-left: 3px solid #0077D4;
html = html.replace(
    'border-left: 3px solid #0077D4;',
    'border-left: 3px solid #22262C;'
)

# Replace Prato 2 header
html = html.replace(
    '<span style="color: #0077D4;">19 PEÇAS (8 Pretas + 11 Azuis)</span>',
    '<span style="color: #22262C;">19 PEÇAS (Todas em Bambu PLA Basic Preto)</span>'
)

# Replace Passo 4.3 text
old_p43 = '<b>PASSO CRÍTICO:</b> Posicione o <b>20_Garra_Dedo_Movel_Direito</b> (Azul) e o <b>21_Garra_Dedo_Movel_Esquerdo</b> (Preto) de modo que as duas pontas da pinça fiquem em contato direto (fechadas). Engrene os setores dentados de ambos os dedos — a alternância entre o dente preto e o dente azul permite confirmar visualmente o alinhamento de cada dente.'
new_p43 = '<b>PASSO CRÍTICO:</b> Posicione o <b>20_Garra_Dedo_Movel_Direito</b> e o <b>21_Garra_Dedo_Movel_Esquerdo</b> (identificados pelos números em baixo-relevo gravados em cada peça) de modo que as duas pontas da pinça fiquem em contato direto (fechadas). Engrene os setores dentados de ambos os dedos com engrenamento perfeitamente simétrico.'

html = html.replace(old_p43, new_p43)

with open(HTML_PATH, "w", encoding="utf-8") as f:
    f.write(html)

print("Cleaned all remaining color references in manual HTML successfully!")
