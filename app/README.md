# RoboControl — controle e gravação do braço 4-DOF

App desktop (Rust + egui) que conversa com o firmware pela USB em 115200 baud.
O firmware continua dono do movimento: aqui só **propomos alvos**. A rampa, os
limites mecânicos e o hold ficam na placa, então um comando perdido no USB não
vira tranco.

## Rodar

```bash
cd app
cargo run --release
```

Depois: escolher `/dev/ttyACM0` → **Conectar** → **Assumir controle (PC)**.
Os 4 sliders passam a mandar alvos a 20 Hz.

## Modo híbrido (a regra de segurança)

| Situação | O que acontece |
|---|---|
| Joystick na mão | o firmware ignora o PC |
| **Assumir controle (PC)** | firmware adota os alvos partindo do ângulo atual |
| Qualquer deflexão do joystick | devolve o controle na hora — pegar no braço sempre vence |
| 500 ms sem alvos (app travou, cabo caiu) | segura a posição atual |
| **STOP** | `#H`: segura as juntas agora |

## Gravação

- **Gravar** captura a pose a 20 Hz — inclusive quando você está movendo no
  joystick (é "ensinar fazendo").
- **Parar gravação** salva em `app/movimentos/<nome>.json`.
- **▶** carrega um movimento; **Reproduzir** manda os alvos interpolados.
  Tocar o joystick cancela o replay (o firmware sai do modo PC).

## Protocolo (firmware v1.3+)

```
#P<b>;<o>;<c>;<g>   alvos em graus (20 Hz; reenviar é o heartbeat)
#M                  assume o controle
#J                  devolve ao joystick
#H                  segura agora
#S                  status  →  S<b>;<o>;<c>;<g>;<modo>
#B                  diagnóstico → B<versão>;boots=N;limb=min;max;limo=...;limc=...;limg=...
```

Testar sem o app:

```bash
stty -F /dev/ttyACM0 115200 raw -echo
printf '#B\n' > /dev/ttyACM0 && cat /dev/ttyACM0
```

## Diagnóstico de alimentação

O app mostra `boots=N` (contador de resets em `.noinit`). **Se esse número subir
enquanto o braço se move, a placa reiniciou** — é a alimentação, não o servo.
O aviso aparece em vermelho na barra superior.

## Estrutura

- `src/protocol.rs` — codificação/decodificação das linhas (testado)
- `src/link.rs` — porta serial: leitura em thread, escrita síncrona
- `src/motion.rs` — gravação, interpolação do replay, JSON (testado)
- `src/main.rs` — UI egui
