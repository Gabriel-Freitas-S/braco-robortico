# Simulação Wokwi — braço robótico 4-DOF

## Abrir no Wokwi

Em [wokwi.com](https://wokwi.com), crie um projeto novo de Arduino Uno e copie `sketch.ino` e `diagram.json` para os arquivos do projeto; inicie a simulação. Para trabalhar localmente, abra esta pasta com a extensão Wokwi para VS Code e configure o `wokwi.toml` de acordo com o fluxo de build Arduino local.

## Relação com o firmware

`sketch.ino` é uma cópia do firmware `firmware/braco_4dof/braco_4dof.ino`, sem adaptações. Depois de mexer no firmware, sincronize a cópia:

```bash
cp firmware/braco_4dof/braco_4dof.ino wokwi/sketch/sketch.ino
```

O `diagram.json` segue a pinagem do firmware: Base D11, Ombro D10, Cotovelo D9 e Garra D5; o joystick direito usa X em A3 (garra) e Y em A2 (ombro). A biblioteca `Servo.h` já é suportada pelo core Arduino do Wokwi; não há `libraries.txt`. Os dois pinos `SEL` dos joysticks ficam sem conexão porque o firmware só lê os eixos analógicos; o LED de status usa o LED integrado do Uno no pino D13.
