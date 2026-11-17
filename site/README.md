# Site — Astro + Starlight + UnoCSS

O site publicado em <https://gabriel-freitas-s.github.io/braco-robortico/> é
gerado aqui. A landing e as páginas de referência usam o mesmo layout do
Starlight (header, busca, tema e navegação); o manual A4 e o rustdoc continuam
arquivos estáticos em `public/`.

## Rodar

```bash
cd site
npm install
npm run dev      # http://localhost:4321/braco-robortico/
npm run build    # gera dist/
npm run preview  # serve o dist com o base do Pages
```

## Estrutura

| Caminho | O que é |
|---|---|
| `src/pages/index.astro` | Landing (StarlightPage, sem sidebar) |
| `src/content/docs/hardware.mdx` | BOM, pinagem, energia e calibração |
| `src/content/docs/software.mdx` | RoboControl, firmware 1.3.7, protocolo e problemas |
| `src/content/docs/guia_montagem.mdx` | Guia de bancada e checagem de interferência |
| `src/styles/landing.css` | Seções e cards da landing |
| `src/styles/starlight.css` | Tema e componentes das páginas de docs |
| `public/assets/` | Fotos, peças e renders (WebP) |
| `public/manual_braco_robotico.html` | Manual A4 (fonte do PDF) |
| `public/manual_braco_robotico_rev23.pdf` | Manual para impressão |
| `public/rust/` | Rustdoc gerado por `cargo doc` |
| `public/{hardware,software,guia_montagem}.html` | Redirects das URLs antigas |

## Fonte da verdade

- **Pinagem, Posição 0, limites e protocolo:** `firmware/braco_4dof/braco_4dof.ino`.
- **BOM e montagem:** `hardware.mdx` + manual Rev 2.3.
- **App e comandos:** `software.mdx` + `app/README.md`; o rustdoc cobre a API interna.
- **Versões:** firmware 1.3.7, app 0.1.0, manual Rev 2.3. Atualize o `CHANGELOG.md` ao mudar qualquer um.

## Regenerar o manual PDF

Depois de editar `public/manual_braco_robotico.html`:

```bash
google-chrome-stable --headless --disable-gpu --no-pdf-header-footer \
  --print-to-pdf=public/manual_braco_robotico_rev23.pdf \
  file://"$PWD"/public/manual_braco_robotico.html
```

## Rustdoc

Na pasta `app/`:

```bash
cargo doc --no-deps --document-private-items
rsync -a --delete --exclude='.lock' --exclude='/index.html' target/doc/ ../site/public/rust/
```

## Deploy

O workflow `.github/workflows/deploy.yml` roda `withastro/action` e publica no
GitHub Pages. O Pages precisa estar em **Settings → Pages → Source: GitHub
Actions**. O `base` do Astro é `/braco-robortico`; se o repositório mudar de
nome, ajuste `site` e `base` em `astro.config.mjs`.
