---
description: Modela e verifica uma montagem mecânica com duas ou mais peças.
agent: build
---

Modele a montagem mecânica solicitada seguindo todos os portões aplicáveis. Dê nomes claros às peças e meça com `measure()` após cada `execute` e após cada booleana. Verifique `clearance`/`compare(kind="fit")`; aceite `touching` ou `apart`, nunca `interpenetrating`. Prefira `RigidJoint`/`RevoluteJoint` a `.move()` absoluto. Em montagens, use `show(shape, "nome")` para nomear cada peça.

Depois, siga `render_view` → `design_audit` (sem `brittle`) → `validate` (PASS) → exportação por peça com `object_name="nome"`. Mantenha a ordem obrigatória dos demais portões do projeto.

Montagem solicitada: $ARGUMENTS
