---
trigger: model_decision
description: Peça 3D funcional com medidas exatas ou impressão na Bambu A1. Aplica o fluxo CAD-to-Print (build123d-mcp + bambu-studio-ai).
---

# CAD-to-Print (regras)

Vale para qualquer tarefa de modelar peça com dimensões, furos, tolerâncias, encaixes — ou imprimir na Bambu A1.

1. **Rota**: peça exata/funcional (incl. tudo do braço robótico) usa **build123d-mcp** via skill `cad-to-print` (`.agents/skills/cad-to-print/SKILL.md`). Proibido `parametric.py` ou geração por IA para esses casos.
2. **Ordem dos gates**: `execute` → `measure` (todo booleano) → `clearance`/`fit` (montagens) → `render_view` → `design_audit` → `validate` → `export step,stl` → `analyze.py --repair` → `preview.py` → `bambu.py open`. Não pular, não reordenar.
3. **Sem `--orient`** em peça saída do CAD (orientação projetada: maior face plana para baixo). `--orient` é só para modelo baixado/gerado por IA.
4. **Sandbox do `execute()`**: proibido `os`, `pathlib`, `subprocess`, rede, `open`/`eval`/`exec`, atributos dunder. Arquivo só via `export`/`import_cad_file`/`render_view`. STEP/STL exportado nunca é editado — regenera via `execute`.
5. **Segurança física**: o agente nunca inicia, pausa ou altera impressão. O usuário imprime no Bambu Studio/Handy; só afirmar impressão após `bambu.py status`. Monitor é somente-leitura e opt-in.
6. Padrões do projeto: unidades **mm**, impressora **A1**, material **PLA**, propósito **functional** salvo pedido contrário. Saídas CAD em `cad-output/`, reparadas em `bambu-output/`.
