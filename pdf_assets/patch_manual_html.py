with open('manual_tecnico_braco_robotico.html', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update line 528 render caption
content = content.replace(
    'Arduino Uno + Shield, 4 espaçadores',
    'Arduino Uno + Shield Joystick Duplo, 4 espaçadores'
)

# 2. Update BOM table items
old_bom_elec = """          <tr>
            <td style="padding: 2.5px 6px;"><b>Sensor Shield v5.0</b></td>
            <td style="padding: 2.5px 6px;">Expansor I/O para servos (Jumper SEL removido)</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">1 un</td>
            <td style="padding: 2.5px 6px;">Barramento de potência isolado dos atuadores</td>
          </tr>
          <tr>
            <td style="padding: 2.5px 6px;"><b>Módulo Joystick Analógico 2-Eixos</b></td>
            <td style="padding: 2.5px 6px;">Potenciômetros 10k duplos + Botão Push (KY-023)</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">2 un</td>
            <td style="padding: 2.5px 6px;">Controle manual dos 4 graus de liberdade</td>
          </tr>
          <tr>
            <td style="padding: 2.5px 6px;"><b>Fonte Chaveada 5V 5A + Filtro</b></td>
            <td style="padding: 2.5px 6px;">5.0V / 5.0A contínuos + Capacitor 1000 µF / 16V</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">1 un</td>
            <td style="padding: 2.5px 6px;">Potência dedicada para picos simultâneos de stall</td>
          </tr>
          <tr>
            <td style="padding: 2.5px 6px;"><b>Jumpers Fêmea-Fêmea 20cm</b></td>
            <td style="padding: 2.5px 6px;">Fios flexíveis coloridos com terminais Dupont</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">10 un</td>
            <td style="padding: 2.5px 6px;">Interligação dos joysticks ao Shield</td>
          </tr>"""

new_bom_elec = """          <tr>
            <td style="padding: 2.5px 6px;"><b>Shield Joystick Duplo (meARM.WDT V2.10)</b></td>
            <td style="padding: 2.5px 6px;">Shield montado no Uno com 2 analógicos integrados + barramento de 4 servos direto</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">1 un</td>
            <td style="padding: 2.5px 6px;">Controle dos 4 eixos; os 4 cabos dos servos conectam todos direto no Shield</td>
          </tr>
          <tr>
            <td style="padding: 2.5px 6px;"><b>Fonte Chaveada 5V 5A + Filtro</b></td>
            <td style="padding: 2.5px 6px;">5.0V / 5.0A contínuos + Capacitor 1000 µF / 16V</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">1 un</td>
            <td style="padding: 2.5px 6px;">Alimentação de alta corrente para os 4 servos no conector do Shield</td>
          </tr>
          <tr>
            <td style="padding: 2.5px 6px;"><b>Cabo USB Tipo-A para Tipo-B</b></td>
            <td style="padding: 2.5px 6px;">Cabo blindado de comunicação e upload de código</td>
            <td style="text-align: center; font-weight: 700; padding: 2.5px 6px;">1 un</td>
            <td style="padding: 2.5px 6px;">Gravação do firmware C++ e alimentação lógica ATmega328P</td>
          </tr>"""

content = content.replace(old_bom_elec, new_bom_elec)

# 3. Update Page 16 Pinout section
content = content.replace(
    'Pinagem oficial no Arduino Uno com Sensor Shield v5, código C++ de controle e checklist final de validação.',
    'Todos os 4 servomotores conectam-se diretamente no Shield Joystick Duplo (meARM.WDT). Código C++ e checklist.'
)

content = content.replace(
    'Pinagem Oficial das Conexões (Arduino Uno + Sensor Shield v5)',
    'Pinagem Oficial das Conexões — Todos os Servos Conectam Direto no Shield Joystick Duplo'
)

# In the table:
content = content.replace(
    '<td>Barramento Externo 5V Shield</td>\n            <td>90° (Apontando reto para frente)</td>',
    '<td>Canal 3 do Shield (S=D9, 5V, GND)</td>\n            <td>90° (Apontando reto para frente)</td>'
)
content = content.replace(
    '<td>Barramento Externo 5V Shield</td>\n            <td>90° (Braço mestre a 45° elevação)</td>',
    '<td>Canal 2 do Shield (S=D10, 5V, GND)</td>\n            <td>90° (Braço mestre a 45° elevação)</td>'
)
content = content.replace(
    '<td>Barramento Externo 5V Shield</td>\n            <td>90° (Antebraço nivelado no horizonte)</td>',
    '<td>Canal 1 do Shield (S=D11, 5V, GND)</td>\n            <td>90° (Antebraço nivelado no horizonte)</td>'
)
content = content.replace(
    '<td>Barramento Externo 5V Shield</td>\n            <td>0° (Pinça completamente fechada)</td>',
    '<td>Canal 4 do Shield (S=D6, 5V, GND)</td>\n            <td>0° (Pinça completamente fechada)</td>'
)

content = content.replace(
    '<td><b>Joystick 1 (Base e Ombro)</b></td>\n            <td><b>Pinos A0 (X) e A1 (Y)</b></td>\n            <td>5V e GND do Arduino Uno</td>',
    '<td><b>Joystick Esquerdo Integrado</b></td>\n            <td><b>A0 (Eixo X) e A1 (Eixo Y)</b></td>\n            <td>Trilhas internas do Shield (sem fiação externa)</td>'
)
content = content.replace(
    '<td><b>Joystick 2 (Cotovelo e Garra)</b></td>\n            <td><b>Pinos A2 (X) e A3 (Y)</b></td>\n            <td>5V e GND do Arduino Uno</td>',
    '<td><b>Joystick Direito Integrado</b></td>\n            <td><b>A2 (Eixo X) e A3 (Eixo Y)</b></td>\n            <td>Trilhas internas do Shield (sem fiação externa)</td>'
)

# Checklist item
content = content.replace(
    '<div>☑ <b>Jumper Shield:</b> Jumper PWR/VCC removido para isolar a fonte 5V.</div>',
    '<div>☑ <b>Conexão dos Servos:</b> Todos os 4 cabos (S, 5V, GND) plugados direto no barramento do Shield.</div>'
)

with open('manual_tecnico_braco_robotico.html', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated manual_tecnico_braco_robotico.html successfully!')
