# Projeto de braço robótico — CAD e impressão 3D

- Projeto funcional para Bambu Lab A1, material PLA; todas as dimensões em mm.
- Fonte CAD em `cad-output/`; `bambu-output/` guarda artefatos de preparação para impressão.
- O build123d MCP é a única forma de modelar peças funcionais dimensionadas. Nunca use `parametric.py` nem geração por IA para essas peças.
- Dúvidas sobre o código/geometria existente do braço: encaminhe ao agente global `read` (gratuito). Siga os tiers globais `read` → `cheap` → `work` → `deep` conforme o custo e a complexidade.

## Portões obrigatórios

Para toda peça funcional, siga exatamente esta ordem, sem pular nem reordenar etapas:

`execute` → `measure` (depois de CADA `execute`) → `clearance`/`fit` (em montagens) → `render_view` → `design_audit` → `validate` → `export step,stl` → `analyze.py --repair` → `preview.py` → `bambu.py open`.

- Depois de cada operação booleana, execute `measure()`: uma booleana que falha pode não alterar a contagem de faces; confira o volume. Geometria incorreta bloqueia o avanço.
- Em montagens, use `clearance` ou `compare(kind="fit")`; aceite apenas `touching` ou `apart`, nunca `interpenetrating`.
- `design_audit()` deve estar livre de `brittle` antes da exportação; `validate()` deve retornar PASS.
- STEP/STL/3MF são artefatos derivados: nunca os edite manualmente; regenere pelo `execute` e exporte pelo fluxo CAD.
- Nunca edite STL/STEP dentro do sandbox de `execute`; use `export`, `import_cad_file` e `render_view` para I/O de arquivos.
- Nunca passe `--orient` ao `analyze.py` para saída CAD: as peças CAD já estão orientadas por projeto. `--orient` serve somente para malhas geradas por IA ou baixadas.

## Segurança física

- Nunca inicie, pause ou altere uma impressão. O usuário fatia e inicia pela Bambu Studio ou pelo Handy.
- Só afirme que uma impressão ocorreu quando `bambu.py status` confirmar.
- Monitoramento é somente leitura e apenas com opt-in explícito.
