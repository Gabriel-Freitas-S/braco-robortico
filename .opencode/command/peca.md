---
description: Inicia uma nova peça funcional seguindo todos os portões CAD-to-Print.
agent: build
---

Crie a peça funcional solicitada seguindo o fluxo CAD-to-Print completo e os portões, sem pular nem reordenar: `execute` → `measure` após cada `execute` → `clearance`/`fit` para montagens → `render_view` → `design_audit` → `validate` → `export step,stl` → `analyze.py --repair` → `preview.py` → `bambu.py open`.

Antes de modelar, faça todas estas perguntas juntas em uma única mensagem e aguarde as respostas:
- Dimensões exatas em mm (L×P×A), diâmetros dos furos e espessuras.
- Encaixe e tolerância: por exemplo, parafuso M3 com furo de folga de 3,2 mm; indique também press-fit ou folga.
- Peça única ou multicolorida? Peças CAD são monocromáticas; multicolor exige AMS.
- Confirme material e finalidade (padrão: PLA, funcional).

Depois das respostas, prossiga pelos portões na ordem obrigatória. Solicitação/contexto: $ARGUMENTS
