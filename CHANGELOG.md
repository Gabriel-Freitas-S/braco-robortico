# Changelog

## Revisão de consistência — 2026-11-15

- **Site mais leve:** fotos, renders e peças convertidos para WebP; `docs/` caiu de ~18 MB para ~8 MB.
- **Navegação mobile:** menu em uma linha com rolagem horizontal, header sticky com blur e âncoras alinhadas.
- **Correções:** dimensões declaradas de prato, app e peças 04/12/H1/H2; metadados sociais completos nas três páginas; `#B` documentado sem “centros”.
- **Limpeza:** removidos a engrenagem 26 do `cad-output/`, 24 renders de debug, imagens de teste e fotos antigas com fundo branco.
- **README:** badges de versão, sumário, pinagem rápida e link do changelog.

## Revisão nov/2026 — 2026-11-14

- **Firmware 1.3.4:** STOP cancela o home; `#J`/`#M` congelam alvos; trava exclusiva de eixo no joystick.
- **App 0.1.0:** teclado e atalhos, log expandível, modo PC com trava do joystick, gravação e replay com velocidade e loop.
- **Garra:** H1 (haste de olhal) e H2 (haste curta) substituem a engrenagem 26; 26 removida do projeto.
- **Geometria:** peça 04 com encaixe de 10 mm e furos a 80 mm; peça 12 na versão Turret V7 Redonda (86 mm).
- **Manual Rev 2.3:** pinagem e firmware atualizados (D11/D10/D9/D5), Posição 0 90/60/45/35, fonte 5 V / 5 A, camada 0,20 mm padrão; sketch antigo removido.
- **Site:** landing reorganizada, páginas de hardware, software e guia de montagem, downloads diretos, fotos dos componentes, metadados e correções de acessibilidade.
- **Documentação:** mapa em `docs/README.md` e BOM/pinagem/protocolo com fonte única no firmware e nas páginas de referência.

## Histórico de firmware (tags)

- **v1.2.7** — trava exclusiva de eixo no joystick (modo cruz).
- **v1.2.6** — proteção anti-batida da base e calibração de limites na EEPROM.
- **v1.2.5** — botões separados: D2 = Posição 0, D4 = trava da garra.
- **v1.2.4** — fim da descida involuntária ao mover para o lado; fechamento e pressão total na garra.
- **v1.2.3** — limite mecânico 10–90° da garra e proteção contra stall.
