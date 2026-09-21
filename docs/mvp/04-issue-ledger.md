# Issue Ledger — Petunia3D MVP Gauntlet

> Registro obrigatório de todo defeito encontrado durante o Gauntlet. Nada deve
> desaparecer da memória do agente.

| Campo | Descrição |
|---|---|
| `Issue ID` | Sequencial estável (`ISSUE-001`). |
| `Severity` | `BLOCKER`, `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `POLISH`. |
| `Subsystem` | Model, Paint, UV, Outliner, Inspector, Assets, Persistence, … |
| `Description` | Defeito em uma frase. |
| `Reproduction` | Passos mínimos. |
| `Root Cause` | Causa real, não sintoma. |
| `Fix` | Arquivo/função alterados. |
| `Test Added` | Teste que impede a volta. |
| `Regression Scope` | O que precisou ser revalidado. |
| `Status` | `OPEN`, `FIXED`, `VERIFIED`, `BLOCKED_EXTERNALLY`. |

## Regras

- Não aplicar patch superficial repetidamente; corrigir no nível arquitetural
  correto.
- Todo bug relevante corrigido recebe, quando tecnicamente apropriado, um teste
  de regressão.
- Bug em Paint pode vir de Document, Renderer, Undo, Material ou UV: não assumir
  que o arquivo onde o defeito aparece é onde a causa existe.
- Release candidate exige contagem zero em todas as severidades dentro do
  escopo.

## Issues conhecidas no início da remediação

| Issue ID | Severity | Subsystem | Description | Status |
|---|---|---|---|---|
| ISSUE-001 | BLOCKER | Model | `edit.delete` no botão/tecla apaga o objeto inteiro em modo Point/Edge/Face. | OPEN |
| ISSUE-002 | BLOCKER | Model | Edge não selecionável no viewport (`pick_edge` não usado). | FIXED |
| ISSUE-003 | BLOCKER | Paint | Pintura sem traço contínuo e sem undo (`begin_paint_stroke`/`finish_paint_stroke` não chamados). | OPEN |
| ISSUE-004 | BLOCKER | Model | Ferramentas de transformação não recebem arrasto do viewport. | FIXED |
| ISSUE-005 | BLOCKER | Model | Cut/Knife cria sessão que nada alimenta. | OPEN |
| ISSUE-006 | BLOCKER | File | `file.export_obj`, `file.export_glb` e `file.import_obj` são stubs que só escrevem status, mas aparecem na palette. | OPEN |
| ISSUE-007 | BLOCKER | Model | `model.loop_cut` inalcançável na prática por depender de seleção de aresta. | OPEN (dependente de ISSUE-002) |
| ISSUE-008 | HIGH | Model | Extrude/Inset/Bevel/Extrude Individual/Scale Selection são one-shot com valores fixos. | OPEN |
| ISSUE-009 | HIGH | Paint | Sem UI de layers, effects, fill scope, lock, projeção, formas e canvas 2D. | OPEN |
| ISSUE-010 | HIGH | UV | Sem UV Editor 2D; seams e diagnósticos inacessíveis. | OPEN |
| ISSUE-011 | HIGH | Global | Sem splitters reais; painéis não redimensionam nem persistem. | OPEN |
| ISSUE-012 | HIGH | Global | Sem Outliner funcional (rename, reorder, context menu, lock, F2). | OPEN |
| ISSUE-013 | HIGH | Global | Sem menus File/Edit/View/Window; sem i18n nas strings públicas da UI Slint. | OPEN |
| ISSUE-014 | HIGH | File | Sem autosave/recovery na UI Slint. | OPEN |
| ISSUE-015 | MEDIUM | Model | `model.instantiate_asset` permanentemente desabilitado. | OPEN |
| ISSUE-016 | MEDIUM | Global | Atalhos: apenas 20 ações roteadas; teclas anunciadas pelo keymap são consumidas sem efeito. | OPEN |
| ISSUE-017 | MEDIUM | Global | Estatísticas decorativas (ex.: `Texel Density: Auto`, algoritmo não executado). | OPEN |
