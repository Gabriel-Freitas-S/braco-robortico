---
name: cad-to-print
description: Projetar peças funcionais para impressão 3D com build123d-mcp (CAD paramétrico) e imprimir com bambu-studio-ai (Bambu Lab A1). Use para qualquer pedido com dimensões exatas, furos, tolerâncias, encaixes ou "precisa caber em...".
---

# CAD-to-Print: peça funcional → impressora Bambu

Fluxo único que une dois sistemas:

- **build123d-mcp** (servidor MCP, ferramentas `execute`, `measure`, `render_view`, `validate`, `export`...) — dono do **loop de geometria**. É aqui que a peça nasce, exata em mm.
- **bambu-studio-ai** (scripts em `.agents/skills/bambu-studio-ai/scripts/`) — dono do **handoff de fabricação**: análise/reparo da malha, preview, abertura no Bambu Studio, estimativa de fatiamento e monitoramento.

```
pedido → [ROTA] → CAD (build123d-mcp: execute → measure → render → snapshot → validate → export STEP+STL)
              → PONTE (analyze.py --repair SEM --orient no STL exportado)
              → preview → Bambu Studio (usuário fatia e imprime) → monitor (somente leitura)
```

## Pré-requisitos

1. MCP `build123d-mcp` conectado (este projeto já tem em `.agents/mcp_config.json` + global). Teste rápido:
   `uv tool run --python 3.12 build123d-mcp@latest --version`
2. Na primeira utilização, rodar o check da skill de impressão (a partir da raiz do projeto):
   `python .agents/skills/bambu-studio-ai/scripts/doctor.py`
3. Bambu Studio instalado (necessário para `bambu.py open` e `slice.py`).
4. Pastas de saída:
   - `cad-output/` — fonte da verdade CAD: `.step` + `.stl` vindos do `export` do MCP.
   - `bambu-output/` — arquivos reparados/orientados/fatiados da skill bambu (padrão dela).
5. Padrões deste projeto (braço robótico): impressora **Bambu Lab A1**, material **PLA**, propósito **functional**, unidades sempre em **mm**.

## Passo 0 — Entender o pedido (perguntar só o que falta, numa mensagem só)

- **O quê** + **dimensões exatas em mm** (largura × profundidade × altura, diâmetros de furo, espessuras).
- **Encaixe/tolerância**: parafuso M3 → furo de folga 3.2 mm; press-fit vs. folga; o que encaixa em quê.
- **Cor única ou multicolor** (multicolor exige AMS — peça CAD é monocromática por natureza).
- **Material e propósito**: padrão PLA / functional (muda paredes e infill sugeridos).
- Se o tamanho não foi dado, perguntar antes de modelar: "Qual o tamanho? ex.: 80 mm de altura".

## Passo 1 — Decidir a rota

| O pedido parece... | Rota |
|---|---|
| Dimensões exatas, furos, tolerâncias, "cabe em...", suporte, bracket, gabinete, placa, adaptador | **CAD (esta skill)** — build123d-mcp. Nunca usar `parametric.py`, nunca gerar por IA. |
| Peça do braço robótico deste projeto | **CAD (esta skill)** — manter consistência com as demais peças. |
| Objeto comum/decorativo sem medidas, personagem, forma orgânica, foto, arquivo do usuário | Delegar para **bambu-studio-ai** (search / generate / arquivo do usuário), depois voltar ao Passo 4. |

## Fase A — Modelar no CAD (build123d-mcp)

Recursos para ler no início da sessão (sem gastar tool-call): `build123d://quickref` (sintaxe testada da versão instalada), `build123d://selectors` se precisar de seletores/fillets, `build123d://bd_warehouse` se precisar de rolamento/parafuso/engrenagem prontos.

Loop obrigatório:

1. `reset` — começar limpo. `version` + `health_check` se for a primeira sessão ou houver suspeita de dependência quebrada.
2. `execute` com `from build123d import *` e a primeira geometria. Regras de autoria:
   - **Bloco de parâmetros nomeados com unidades no topo**, nunca constantes mágicas inline (ex.: `plate_t = 6.0  # mm`). Ordem: base → features secundárias → acabamento. Coordenadas derivadas dos parâmetros.
   - Atribuir a forma final a `result` ou registrar com `show(shape, "nome")` (obrigatório em montagens).
   - **Sandbox do `execute()`**: proibido `import os/pathlib/subprocess/socket/urllib/requests`, proibido `open/eval/exec`, proibido acesso a atributos dunder (`__class__`). E/S de arquivo só via ferramentas MCP (`export`, `import_cad_file`, `render_view(save_to=)`). Dúvida de API: `inspect.signature(Classe)` / `inspect.getdoc()` (permitidos).
3. `measure()` **após cada `execute`** — conferir `volume` plausível e `topology.faces` (booleano que falhou não muda as contagens). Não avançar com geometria errada.
4. Montagem (2+ peças com relação mecânica): `clearance`/`compare(kind="fit")` entre as peças nomeadas — esperado `touching` ou `apart`, nunca `interpenetrating`. Prefira `RigidJoint`/`RevoluteJoint`/etc. a `.move()` absoluto.
5. `render_view()` (começar por `iso`) **somente depois** de `measure` (e `clearance`) passarem. `quality="high"` para superfícies curvas; `clip_plane` para inspecionar furos/paredes internas. Render prova aparência, não geometria.
6. `save_snapshot("marco")` antes de qualquer operação complexa/arriscada; `restore_snapshot` se quebrar (nunca empilhar correção sobre estado quebrado).
7. Erro no `execute`: `last_error()` para a linha exata + `repair_hints(texto_do_erro)` (o `execute` já anexa hints inline).
8. Antes do export final: `design_audit()` (robustez dos parâmetros ±10%; zerar `brittle` — se pedir "sem parâmetros nomeados", içar as dimensões para o bloco de parâmetros e rodar de novo), depois `validate()` (precisa PASS).
9. `export` em **uma chamada** com `format="step,stl"` para `cad-output/` (ex.: `cad-output/suporte_braco`). O STEP é a fonte da verdade; o STL é o handoff para a Fase B. Confirmar volume/bbox da linha de sanidade retornada. Montagem: exportar por peça (`object_name="nome"`) para imprimir separado, ou `"*"` para conjunto.

## Fase B — Ponte CAD → fatiador (bambu-studio-ai)

O STL saído do `export` entra no pipeline da skill de impressão **com uma diferença crucial**: peça CAD já nasce orientada (maior face plana para baixo, furos laterais em gota para cima), então **NUNCA passar `--orient`** — ele é só para modelos baixados/gerados por IA, que chegam em orientação aleatória.

A partir da raiz do projeto (`BAMBU_OUTPUT_DIR` pode redirecionar as saídas):

```
python .agents/skills/bambu-studio-ai/scripts/analyze.py cad-output/peca.stl --repair --material PLA --purpose functional
```

- Usa a impressora configurada (senão, cai para A1); para forçar: `--printer "A1"`.
- Cada etapa que altera o modelo grava um novo arquivo; continuar com o arquivo da última linha `➡️ Use this file: ...`.
- Reportar ao usuário: score /10, o que foi reparado, paredes finas, overhangs, configurações sugeridas.
- Score travado em ≤4 (malha não-estanque, peça flutuante, não cabe na mesa) = voltar à Fase A e regenerar via `execute` — **nunca editar o STL/STEP exportado**, eles são artefatos derivados.

Estimativa de tempo/filamento antes de abrir o Bambu Studio (opcional, precisa do Bambu Studio instalado):

```
python .agents/skills/bambu-studio-ai/scripts/slice.py bambu-output/peca_repaired.stl --printer A1 --material PLA
```

É estimativa de planejamento; a fatiada oficial acontece no Bambu Studio.

## Fase C — Preview → Bambu Studio → impressão (usuário) → monitor

1. Preview do arquivo que o `analyze.py` indicou:
   `python .agents/skills/bambu-studio-ai/scripts/preview.py bambu-output/peca_repaired.stl`
   Mostrar a imagem ao usuário **antes** de abrir o Bambu Studio. Se a interface exibir imagens, embutir; senão abrir com o visualizador do SO e passar o caminho. Olhar a prévia: fragmentos flutuantes, orientação errada, forma deformada.
2. Handoff:
   `python .agents/skills/bambu-studio-ai/scripts/bambu.py open bambu-output/peca_repaired.stl`
   E pedir a revisão: forma, tamanho (barra inferior), peças flutuantes; depois fatiar (Ctrl+R) e conferir tempo, filamento e suportes. Aguardar a resposta — mudança volta à fase correspondente.
3. **O usuário inicia a impressão** no Bambu Studio (ou Bambu Handy). Nunca afirmar que imprimiu sem `bambu.py status` mostrar. A skill jamais inicia/pausa/altera uma impressão: máquina física, filamento e horas em jogo.
4. Monitor somente-leitura e só se o usuário aceitar ("Quer que eu acompanhe a impressão?"). `python .agents/skills/bambu-studio-ai/scripts/monitor.py --wait-start 30 --interval 300` em background, repassando cada linha `📢 NOTIFY`; ou `--once` sob agendamento. Problema → usuário pausa/cancela na tela da impressora ou no Handy.

Multicolor/AMS: peça CAD é monocromática. Se pedirem cor, seguir `references/multicolor.md` da skill bambu (`colorize`) sobre o modelo texturizado correspondente.

## Erros comuns (não cometer)

| Erro | Em vez disso |
|---|---|
| Modelar peça exata com `parametric.py` ou IA generativa | build123d-mcp, Fase A |
| Passar `--orient` em peça saída do CAD | Sem `--orient`; orientação foi projetada no CAD |
| Ir do `generate.py`/CAD direto ao Bambu Studio | `analyze.py --repair` + `preview.py` no meio |
| Renderizar para "provar" geometria | `measure()` (números) primeiro, render depois |
| Editar STEP/STL/3MF exportado | Regenerar com `execute()` e re-exportar |
| `import os`, `open()`, `subprocess` no `execute()` | Ferramentas MCP (`export`, `import_cad_file`) |
| Dizer "pronto para imprimir" sem mostrar | Mostrar preview + reportar medidas do `measure` |
| Dizer que a impressão começou porque o usuário disse "ok" | Usuário inicia; confirmar via `bambu.py status` |

## Checklist antes de dizer "terminado"

```
[ ] Tamanho, material (PLA) e propósito (functional/decorative) conhecidos
[ ] Peça modelada no build123d-mcp com parâmetros nomeados
[ ] measure() ok após cada booleano; clearance/fit ok em montagens
[ ] design_audit() sem brittle; validate() PASS; export STEP+STL em cad-output/
[ ] analyze.py --repair (SEM --orient) executado e resultado reportado
[ ] Preview exibido ao usuário
[ ] Aberto no Bambu Studio; usuário revisou e fatiou
[ ] Usuário iniciou a impressão (oferta de monitoramento feita)
```
