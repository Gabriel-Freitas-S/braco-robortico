# RoboControl — controle e gravação do braço 4-DOF

App desktop (Rust + egui) que conversa com o firmware pela USB em 115200 baud.
O firmware continua dono do movimento: aqui só **propomos alvos**. A rampa, os
limites mecânicos e o hold ficam na placa, então um comando perdido no USB não
vira tranco.

## Requisitos

| | |
|---|---|
| **Todos** | Arduino Uno + cabo USB |
| **Para compilar** | Rust stable + `cargo` (Linux: `libudev-dev`; Windows: Rust MSVC + *Desktop development with C++*; macOS: Xcode CLT) |
| **Linux** | usuário no grupo `dialout` (Arch: `uucp`) |
| **Windows** | driver CH340 em clones |
| **macOS** | — |

Para só usar o app, baixe o executável pronto — não precisa de Rust. O
`install.sh` é só Linux; nos outros sistemas use o pacote ou `cargo run --release`.

## Baixar pronto

Os executáveis saem do GitHub Actions a cada tag `app-v*` e ficam em
<https://github.com/Gabriel-Freitas-S/braco-robortico/releases>:

| Sistema | Arquivo | Nota |
|---|---|---|
| Linux | `RoboControl-*-x86_64.AppImage` | precisa de FUSE 2 (`libfuse2`/`libfuse2t64`); sem FUSE: `--appimage-extract-and-run` |
| Linux (alternativa) | `RoboControl-*-linux-x86_64.tar.gz` | sem FUSE; extraia e rode `./robocontrol` |
| Windows | `RoboControl-*-windows-x86_64.zip` | portable, sem instalador |
| macOS | `RoboControl-*-macos-universal.zip` | Intel + Apple Silicon, assinatura ad-hoc |

Linux exige glibc ≥ 2.35 (Ubuntu 22.04+, Debian 12+, Fedora 36+). Os binários
não têm certificado pago: no Windows use **Mais informações → Executar assim
mesmo**; no macOS, botão direito → **Abrir**. `SHA256SUMS.txt` acompanha o
release e a proveniência é atestada pelo Actions.

## Rodar

### Linux

```bash
cd app && ./install.sh   # opcional: instala no menu
# ou: cargo run --release
```

Instala sem root: binário em `~/.local/bin/robocontrol`, ícone e entrada
**RoboControl** no menu de aplicativos. Movimentos em
`~/.local/share/robocontrol/movimentos/` (ou `$XDG_DATA_HOME/...`).

### macOS

```bash
cd app && cargo run --release
```

Movimentos em `~/.local/share/robocontrol/movimentos/`.

### Windows (PowerShell)

```powershell
cd app
cargo run --release
# ou: cargo build --release; .\target\release\robocontrol.exe
```

Movimentos em `%APPDATA%\robocontrol\movimentos`.

Depois: escolher a porta (Linux `/dev/ttyACM0`; macOS `/dev/cu.usbmodem*`;
Windows `COM3`) → **Conectar** → **Assumir controle (PC)**.

## Firmware e atualizações

O sketch vai **embutido no binário** (o mesmo `.ino` do repositório), então o
app abre e funciona sem rede. No boot ele consulta o repositório uma vez, em
thread separada; o botão **Firmware** mostra:

- a versão embutida, a que a placa reporta em `#B` e a publicada em `main`;
- **Salvar sketch (.ino)** — grava em
  `~/.local/share/robocontrol/firmware/braco_4dof/` (Windows:
  `%APPDATA%\robocontrol\firmware\...`), pronto para abrir no Arduino IDE;
- **Abrir release** quando há versão nova do app. O app **não se
  auto-atualiza**: baixar e executar binário sem assinatura seria pior que o
  aviso.

Gravar na placa continua no Arduino IDE/arduino-cli (ver a seção **Firmware**
no [README do projeto](../README.md#firmware)). Gravar reinicia a placa e os
servos ficam sem torque durante o upload — escore o braço. A Posição 0 e os
limites ficam na EEPROM e sobrevivem à regravação.

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
| **Joystick com o PC no controle** | fica **travado**: mexer a alavanca não interfere (v1.3.4) |
| **Devolver ao joystick** | `#J`: congela no ângulo atual e devolve o controle |
| 500 ms sem alvos (app travou, cabo caiu) | segura a posição atual e devolve ao joystick |
| **STOP** | `#H`: cancela o retorno à Posição 0 e segura as juntas agora |
| **Travar garra** | `#L1`: mantém pressão contínua no objeto e bloqueia abertura |
| **Liberar garra** | `#L0`: a garra volta a abrir pelo pad |

A trava é a mesma do botão físico D4 — um só estado (`garraTravada`) no firmware,
então o botão da bancada e o botão do app nunca discordam. Com a trava ativa a
garra **só aperta**: nem o pad, nem o replay (`#P`) conseguem abrir.

## Teclado

O botão **Atalhos** (na faixa “Controle”) e `H`/`F1` abrem a lista.
Os atalhos valem com o app em foco e nenhum campo de texto ativo; o movimento
exige o PC no controle (`M`). Passar o mouse sobre os controles mostra a ação e
a tecla correspondente.

| Tecla | Ação |
|---|---|
| `A` / `D` | Base: girar à esquerda / direita |
| `W` / `S` | Cotovelo: subir / descer |
| `←` / `→` | Garra: fechar / abrir |
| `↑` / `↓` | Ombro: subir / descer |
| `Shift` + movimento | modo fino (metade da velocidade) |
| `M` | assumir o controle (PC) |
| `J` | devolver ao joystick |
| `Espaço` | STOP |
| `G` | travar / liberar a garra |
| `R` | iniciar / parar e salvar a gravação |
| `P` | reproduzir / pausar / continuar |
| `1` / `2` / `3` | velocidade 0.5× / 1× / 2× |
| `L` | repetir o movimento (loop) |
| `H` / `F1` | abrir / fechar a lista de atalhos |
| `Esc` | fechar a lista ou o modal de ângulo |

Conectar, escolher a porta, carregar/excluir movimentos e digitar o nome
continuam no mouse/toque.

## Log

A barra inferior mostra o último evento. Clique para abrir: o painel expande
com contagem de eventos, cores para erros/STOP e ações **Copiar** e **Limpar**.
As linhas são selecionáveis para copiar um trecho.

## Gravação

- **Gravar** captura a pose a 20 Hz (via `#S` quando no joystick e `S...` no modo
  PC) — inclusive quando você está movendo no joystick físico (é "ensinar fazendo").
- **Parar e salvar** remove pausas paradas no início/fim, grava em `~/.local/share/robocontrol/movimentos/<nome>.json` (Windows: `%APPDATA%\robocontrol\movimentos`) e já deixa o movimento carregado para replay.
- Se o cabo cair, a placa reiniciar ou você desconectar, a gravação em andamento é encerrada e salva (com sufixo `_2`, `_3`… se o nome já existir).
- **Reproduzir** aguarda o braço alcançar a pose inicial antes de iniciar o
  cronômetro, oferece barra de progresso, **Pausar/Continuar** e seletor de
  velocidade (**0.5×**, **1×**, **2×**). **✕** exclui (com confirmação na própria linha). Tocar num pad ou usar STOP cancela o replay.

## Protocolo (firmware v1.3+)

```
#P<b>;<o>;<c>;<g>   alvos em graus (replay / ajuste fino)
#V<lx>;<ly>;<rx>;<ry>  sticks virtuais -100..100 (20 Hz; reenviar é o heartbeat)
#M                  assume o controle e congela no ângulo atual
#J                  devolve ao joystick e congela no ângulo atual
#H                  STOP: cancela o retorno à Posição 0 e segura agora
#L0 / #L1           #L1 trava a pressão da garra; #L0 libera a abertura
#S                  status  →  S<b>;<o>;<c>;<g>;<modo>;<trava>
#B                  diagnóstico → B<versão>;boots=N;limb=min;max;limo=...;limc=...;limg=...
```

Testar sem o app:

```bash
# Linux
stty -F /dev/ttyACM0 115200 raw -echo
printf '#B\n' > /dev/ttyACM0 && cat /dev/ttyACM0
```

## Diagnóstico de alimentação

O app mostra `boots=N` (contador de resets em `.noinit`). **Se esse número
mudar enquanto o braço se move, a placa reiniciou** — é a alimentação, não o servo.
O aviso aparece em vermelho na barra superior.

## Estrutura

- `src/protocol.rs` — codificação/decodificação das linhas (testado)
- `src/link.rs` — porta serial: leitura em thread, escrita síncrona
- `src/motion.rs` — gravação, interpolação do replay, JSON (testado)
- `src/firmware.rs` — sketch embutido, versão e gravação do `.ino` (testado)
- `src/update.rs` — checagem de versões publicadas, HTTPS (testado)
- `src/main.rs` — UI egui (pads de toque, teclado, atalhos, log, gravação, replay)

## Documentação

O código usa rustdoc (`//!` nos módulos, `///` nos itens). A versão navegável
fica em [`docs/rust/`](../docs/rust/robocontrol/index.html) e é publicada em
<https://gabriel-freitas-s.github.io/braco-robortico/rust/robocontrol/index.html>.
Para regenerar depois de mudar o código:

```bash
cargo doc --no-deps --document-private-items
rsync -a --delete --exclude='.lock' --exclude='/index.html' target/doc/ ../docs/rust/
```

`--document-private-items` é obrigatório: o crate é binário, então sem ele o
HTML sai quase vazio. O `--exclude='/index.html'` preserva o redirecionamento
de `docs/rust/` para `robocontrol/`.

## Sentido dos eixos

`#V` positivo em X do pad esquerdo **diminui** o ângulo da base; Y para cima do
pad esquerdo **aumenta** o cotovelo; X do pad direito **abre** a garra; Y para
cima **aumenta** o ombro. Se algum ficar invertido na mão, é só trocar o sinal
na montagem de `cmd_sticks` em `tick()`.

## Licença

Código do app sob MIT. Projeto acadêmico da disciplina de Sistemas Embarcados
(Engenharia da Computação); modelos 3D, imagens e documentação sob
[CC BY 4.0](../LICENSE-CC-BY-4.0.txt). Ver [`LICENSE`](../LICENSE) na raiz.
