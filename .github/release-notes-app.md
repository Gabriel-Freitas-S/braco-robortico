App desktop (Rust + egui) que controla e grava movimentos do braço 4-DOF pela
USB. O firmware continua dono do movimento: o app só propõe alvos.

## Baixar

| Sistema | Arquivo | Observação |
|---|---|---|
| Linux x86_64 | `RoboControl-*-x86_64.AppImage` | precisa de FUSE 2 (`sudo apt install libfuse2` no 22.04, `libfuse2t64` no 24.04). Sem FUSE: `./RoboControl-*.AppImage --appimage-extract-and-run` |
| Linux x86_64 (alternativa) | `RoboControl-*-linux-x86_64.tar.gz` | sem FUSE; extraia e rode `./robocontrol` |
| Windows x86_64 | `RoboControl-*-windows-x86_64.zip` | portable, sem instalador. SmartScreen: **Mais informações → Executar assim mesmo** |
| macOS Intel e Apple Silicon | `RoboControl-*-macos-universal.zip` | `.app` com assinatura ad-hoc (não notarizado). Primeira abertura: botão direito → **Abrir**, ou Ajustes → Privacidade e Segurança |

Linux exige glibc ≥ 2.35 (Ubuntu 22.04+, Debian 12+, Fedora 36+). Usuário no
grupo `dialout` (Arch: `uucp`) para acessar `/dev/ttyACM*`; relogue depois de
`sudo usermod -aG dialout "$USER"`.

## Firmware

O sketch **vem embutido no app**: abra **Firmware e atualizações** e use
**Salvar sketch (.ino)** — o app também avisa quando há versão mais nova no
repositório. Gravar na placa continua no Arduino IDE ou no arduino-cli; o
executável não grava firmware. Este release também traz `braco_4dof.ino` e o
`braco_4dof.ino.hex` compilado.

- **Windows:** clones com CH340 precisam do driver do fabricante; o Uno R3
  original usa USB nativo e não precisa.
- **macOS:** clones com CH34x podem exigir driver; a porta aparece como
  `/dev/cu.usbmodem*` ou `/dev/cu.wchusbserial*`.
- **Gravar reinicia a placa:** os servos ficam sem torque durante o upload —
  escore o braço antes. Posição 0 e limites ficam na EEPROM e sobrevivem.

## Verificação

`SHA256SUMS.txt` acompanha os arquivos. A proveniência de cada binário pode ser
conferida com:

```bash
gh attestation verify <arquivo> --repo Gabriel-Freitas-S/braco-robortico
```
