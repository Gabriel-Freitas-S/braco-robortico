# RoboControl — controle e gravação do braço 4-DOF

App desktop (Rust + egui) que conversa com o firmware pela USB em 115200 baud.
O firmware continua dono do movimento: aqui só **propomos alvos**. A rampa, os
limites mecânicos e o hold ficam na placa, então um comando perdido no USB não
vira tranco.

## Instalar (menu do sistema)

```bash
cd app && ./install.sh
```

Instala sem root: binário em `~/.local/bin/robocontrol`, ícone e entrada
**RoboControl** no menu de aplicativos. Movimentos gravados vão para
`~/.local/share/robocontrol/movimentos/`.

## Rodar sem instalar

```bash
cd app && cargo run --release
```

Depois: escolher `/dev/ttyACM0` → **Conectar** → **Assumir controle (PC)**.

A tela mostra **dois pads de toque**, no mesmo layout da bancada:

| Pad | Eixo X | Eixo Y |
|---|---|---|
| Esquerdo | Base | Cotovelo |
| Direito | Garra | Ombro |

Arrastar move como alavanca; **soltar o dedo centraliza = segurar a posição**.

## Modo híbrido (a regra de segurança)

| Situação | O que acontece |
|---|---|
| Joystick na mão | o firmware ignora o PC |
| **Assumir controle (PC)** | firmware adota os alvos partindo do ângulo atual |
| **Joystick com o PC no controle** | fica **travado**: mexer a alavanca não interfere (v1.3.3) |
| **Devolver ao joystick** / STOP | única forma de pegar o controle de volta na mão |
| 500 ms sem alvos (app travou, cabo caiu) | segura a posição atual e devolve ao joystick |
| **STOP** | `#H`: segura as juntas agora |
| **Travar garra** | `#L1`: mantém pressão contínua no objeto e bloqueia abertura |
| **Liberar garra** | `#L0`: a garra volta a abrir pelo pad |

A trava é a mesma do botão físico D4 — um só estado (`garraTravada`) no firmware,
então o botão da bancada e o botão do app nunca discordam. Com a trava ativa a
garra **só aperta**: nem o pad, nem o replay (`#P`) conseguem abrir.

## Gravação

- **Gravar** captura a pose a 20 Hz — inclusive quando você está movendo no
  joystick (é "ensinar fazendo").
- **Parar e salvar** grava em `~/.local/share/robocontrol/movimentos/<nome>.json`.
- **▶** carrega um movimento; **Reproduzir** manda os alvos interpolados.
  Tocar o joystick cancela o replay (o firmware sai do modo PC).

## Protocolo (firmware v1.3+)

```
#P<b>;<o>;<c>;<g>   alvos em graus (replay / ajuste fino)
#V<lx>;<ly>;<rx>;<ry>  sticks virtuais -100..100 (20 Hz; reenviar é o heartbeat)
#M                  assume o controle
#J                  devolve ao joystick
#H                  segura agora
#L0 / #L1           libera / trava a pressão da garra
#S                  status  →  S<b>;<o>;<c>;<g>;<modo>;<trava>
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
- `src/main.rs` — UI egui (pads de toque, gravação, replay)

## Sentido dos eixos

`#V` positivo em X do pad esquerdo **diminui** o ângulo da base; Y para cima do
pad esquerdo **aumenta** o cotovelo; X do pad direito **abre** a garra; Y para
cima **aumenta** o ombro. Se algum ficar invertido na mão, é só trocar o sinal
na montagem de `cmd_sticks` em `tick()`.
