# Documentação do projeto

Este diretório é o site publicado no GitHub Pages **e** a documentação de bancada. O mapa abaixo diz onde cada assunto mora para evitar a duplicação que existia antes.

## Mapa

| Arquivo | Assunto |
|---|---|
| `index.html` | Landing: visão geral, montagem, hardware resumido, impressão, peças, software, downloads e créditos |
| `hardware.html` | BOM, pinagem, alimentação, botões, calibração, limites e checklist |
| `software.html` | RoboControl, firmware 1.3.4, protocolo, diagnóstico e problemas comuns |
| `guia_montagem.html` | Guia curto de bancada, mobile-friendly |
| `guia_montagem.md` | Fonte em Markdown do guia (GitHub) |
| `manual_braco_robotico.html` | Manual técnico completo, Rev 2.3 (17 páginas A4) |
| `manual_braco_robotico_rev23.pdf` | Manual para impressão/offline |
| `assets/site/site.css` | Estilo compartilhado da landing e das páginas |
| `assets/site/docs.css` | Componentes das páginas de referência |
| `assets/site/components/` | Fotos dos componentes eletrônicos (WebP com fundo transparente) |
| `rust/` | Rustdoc do app, gerado por `cargo doc` |

As imagens do site são WebP para manter o Pages leve. Se precisar editar
uma foto, trabalhe no PNG original e gere o WebP de novo com
`magick foto.png -quality 82 foto.webp` (use `-alpha on` nas fotos recortadas).

## Fonte da verdade

- **Pinagem, Posição 0, limites e protocolo:** `firmware/braco_4dof/braco_4dof.ino`.
- **BOM e montagem:** `hardware.html` + manual Rev 2.3.
- **App e comandos:** `software.html` + `app/README.md`; o rustdoc cobre a API interna.
- **Versões:** firmware 1.3.4, app 0.1.0, manual Rev 2.3. Atualize o `CHANGELOG.md` ao mudar qualquer um.

## Visualizar localmente

```bash
python3 -m http.server 8000 -d docs
# abra http://localhost:8000/
```

As páginas são estáticas e não dependem de build. O `.nojekyll` preserva o rustdoc no Pages; por isso os arquivos `.md` não são renderizados pelo GitHub Pages — use a versão `.html`.

## Regenerar

**Manual PDF** (depois de editar o HTML):

```bash
google-chrome-stable --headless --disable-gpu --no-pdf-header-footer \
  --print-to-pdf=docs/manual_braco_robotico_rev23.pdf \
  file://"$PWD"/docs/manual_braco_robotico.html
```

**Rustdoc** (na pasta `app/`):

```bash
cargo doc --no-deps --document-private-items
rsync -a --delete --exclude='.lock' --exclude='/index.html' target/doc/ ../docs/rust/
```

**Guia:** edite `guia_montagem.md` e replique as mudanças em `guia_montagem.html` (a versão web é a que o site publica).

