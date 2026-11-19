# Changelog

## App 0.2.1 — trava da garra no replay — 2026-11-19

- **Correção:** a gravação de movimentos não guardava a trava da garra (G/`#L1`). Como `#L0` não move a garra, o "soltar" não deixava amostra, e o replay reproduzia só o aperto de 2,5° — sem a pressão contínua, o objeto escorregava. O JSON agora tem `locks` com o instante de cada transição, capturada pela tecla G, pelo botão do painel e pelo D4 (via status); o replay reaplica `#L1`/`#L0` nos mesmos instantes, inclusive no início e no loop.
- **Compatibilidade:** arquivos antigos, sem `locks`, carregam e o replay não mexe na trava (não há evento para reaplicar).

## App 0.2.0 — interface HUD — 2026-11-17

- **Versão:** 0.1.0 → 0.2.0. Mudou só a interface; protocolo, comandos e arquivos de movimento continuam iguais.
- **Identidade:** grade de 24 px no fundo da área central, pads sem moldura, dock em painel único e cartões com dois cantos cortados a 45°; janela inicial 1280×720.
- **Tipografia:** Rajdhani SemiBold no display e Space Grotesk Regular no corpo (SIL OFL 1.1, embutidas); o Hack do egui continua nos números, teclas e log.
- **Ícones:** 18 conceitos do pack **Ionicons Sharp Interface Icons** (SVG Repo, MIT) rasterizados num atlas 5×4 branco, tintado pelo tema. Procedência, autor e licença em `app/assets/svgrepo-icons.json` e `app/assets/SVGRepo-MIT.txt`.
- **Status:** readout `LINK`/`OFFLINE` na altura do STOP; **Atalhos** e **Firmware** viram botões HUD com ícone.

## Site — identidade HUD e screenshot novo — 2026-11-17

- **Fontes:** a landing e as docs passam a usar Rajdhani + Space Grotesk em WOFF2, as mesmas do app. Antes o CSS pedia Outfit e JetBrains Mono, que nunca eram carregadas e caíam na fonte do sistema.
- **Estilo:** grade de 24 px no fundo da landing, corte de 45° em botões, cards, figuras e avisos, tick de acento nos títulos de seção e paleta alinhada ao app.
- **Screenshot:** foto do RoboControl atualizada para a interface HUD (1280×646) na landing e na página de software.
- **Downloads:** a lista virou grade de cards estática. A lista antiga usava a classe `.where`, que o UnoCSS interpretava como utilitário e animava com um contorno pulsante rosa/azul.
- **Manual:** o HTML do manual ganhou a rail lateral das docs, fundo escuro, o mesmo azul do app e os cantos cortados só nas superfícies grandes (cards, passos, figuras) para não perder legibilidade; o PDF Rev 2.3 foi regerado (17 páginas).
- **Limpeza:** removidos `public/assets/site/site.css` e `docs.css`, CSS da versão anterior do site; o UnoCSS não varre mais `public/`.
- **Docs:** rustdoc regenerado a partir dos comentários revistos; caminho do rustdoc corrigido no README do app (`site/public/rust/`).

## App 0.1.0 — executáveis e firmware embutido — 2026-11-17

- **Executáveis:** novo workflow `app-release.yml` publica, a cada tag `app-v*`, AppImage (com `libudev` embutida; glibc ≥ 2.35), tar.gz sem FUSE, zip portable do Windows (CRT estática) e `.app` universal do macOS (arm64 + Intel, assinatura ad-hoc) — só em GitHub Releases, nunca no crates.io.
- **Firmware embutido:** o `.ino` do repositório vai compilado no binário (`include_str!`); o modal **Firmware e atualizações** mostra a versão embutida, a da placa (`#B`) e a publicada, e salva o sketch em `~/.local/share/robocontrol/firmware/` (Windows: `%APPDATA%`).
- **Checagem no boot:** uma requisição HTTPS em thread separada (rustls/ring, sem OpenSSL) para as releases e para o sketch em `main`; offline degrada para o embutido, sem popup. Nada é executado automaticamente — não há auto-update.
- **Empacotamento:** ícone PNG 1024² para AppImage/`.app`, `robocontrol.desktop` como fonte única do atalho (usado também pelo `install.sh`), `SHA256SUMS.txt` e atestado de proveniência em cada release, com o `.hex` do firmware compilado anexado.

## Firmware 1.3.7 — 2026-11-17

- **Botões travados no modo PC:** D2 (Posição 0) e D4 (garra/limite da Base) ficam inertes enquanto o app controla. Antes, segurar o joystick podia salvar a Posição 0 ou mover a Base para o limite, furando o travamento e bloqueando o loop por ~0,5 s.
- **Diagnóstico:** o firmware avisa no log quando o heartbeat do app para e o joystick assume (`PC: sem heartbeat...`).

## Firmware 1.3.6 — 2026-11-17

- **Posição exata:** a rampa agora chega no alvo (170° fica 170°, não 169°); antes ela parava a 0,005° e o status truncava para o inteiro anterior.
- **Calibração de 1°:** `OFFSET_BASE`, `OFFSET_OMBRO` e `OFFSET_COTOVELO` somam 1° só no pulso do servo (alvo 170° → pulso de 171°), compensando a folga/recuo sem mudar o status nem os limites. A garra fica em 0 por padrão.
- **Servos soltos:** Base, Ombro e Cotovelo voltam a ser desenergizados após 240/600 ms sem movimento. O status fica no alvo exato; o recuo físico é compensado pelo offset.
- **STOP:** volta a desenergizar os servos em ~0,6 s.

## Migração para Astro + Starlight — 2026-11-15

- **Stack:** site migrado para Astro 7 + Starlight + UnoCSS (ícones Lucide); landing e páginas de referência no mesmo header, busca e tema.
- **Navegação:** sidebar com “← Voltar ao site”, TOC e busca em português; a home usa `StarlightPage` sem sidebar.
- **Conteúdo:** hardware, software e guia em MDX; manual A4 e rustdoc estáticos em `site/public/`.
- **URLs:** `/hardware/`, `/software/` e `/guia_montagem/`; redirects preservam os `.html` antigos.
- **Deploy:** GitHub Actions (`withastro/action`) no lugar do Pages legacy em `/docs`.
- **Logo:** `logo.svg` da raiz corrigido (cantos brancos removidos) e usado no site e no ícone do app.

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
