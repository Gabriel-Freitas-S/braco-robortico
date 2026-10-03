---
description: Prepara e entrega artefatos CAD para fabricação sem iniciar a impressão.
agent: build
---

Confirme que as exportações CAD existem em `cad-output/`. Para cada arquivo, execute `python .agents/skills/bambu-studio-ai/scripts/analyze.py <file> --repair --material PLA --purpose functional` sem `--orient`. Informe a pontuação de 0–10 e os reparos aplicados. Em seguida, execute `preview.py`, mostre a prévia ao usuário e só então execute `bambu.py open`.

O USUÁRIO fatia e inicia a impressão pela Bambu Studio ou pelo Handy. O agente apenas abre o Bambu Studio e, somente com opt-in, monitora em modo somente leitura; nunca inicia, pausa ou altera a impressão.

Arquivos/solicitação: $ARGUMENTS
