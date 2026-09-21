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
| ISSUE-001 | BLOCKER | Model | `edit.delete` no botão/tecla apaga o objeto inteiro em modo Point/Edge/Face. | FIXED |
| ISSUE-002 | BLOCKER | Model | Edge não selecionável no viewport (`pick_edge` não usado). | FIXED |
| ISSUE-003 | BLOCKER | Paint | Pintura sem traço contínuo e sem undo. | FIXED |
| ISSUE-004 | BLOCKER | Model | Ferramentas de transformação não recebem arrasto do viewport. | FIXED |
| ISSUE-005 | BLOCKER | Model | Cut/Knife cria sessão que nada alimenta. | PARTIAL — `model.knife` abre a sessão e `Esc` cancela, mas o viewport ainda não alimenta os dois pontos de aresta nem confirma com Enter. |
| ISSUE-006 | BLOCKER | File | `file.export_obj`, `file.export_glb` e `file.import_obj` eram stubs na palette. | FIXED |
| ISSUE-007 | BLOCKER | Model | `model.loop_cut` inalcançável por depender de seleção de aresta. | PARTIAL — seleção de aresta funciona e o comando é roteado; falta o preview por hover e o slide. |
| ISSUE-008 | HIGH | Model | Extrude/Inset/Bevel/Extrude Individual/Scale Selection one-shot. | PARTIAL — Extrude/Inset/Bevel/Push-Pull viraram sessões paramétricas com preview e Tool Properties; `extrude_individual` e `scale_selection` continuam one-shot. |
| ISSUE-009 | HIGH | Paint | Sem UI de layers, effects, fill scope, lock, projeção, formas e canvas 2D. | OPEN |
| ISSUE-010 | HIGH | UV | Sem UV Editor 2D; seams e diagnósticos inacessíveis. | OPEN |
| ISSUE-011 | HIGH | Global | Sem splitters reais; painéis não redimensionam nem persistem. | OPEN |
| ISSUE-012 | HIGH | Global | Sem Outliner funcional completo (rename F2, reorder, context menu). | PARTIAL — seleção, visibilidade, lock e exclusão funcionam; faltam rename, reorder e context menu. |
| ISSUE-013 | HIGH | Global | Sem menus File/Edit/View/Window; i18n ausente na UI Slint. | OPEN |
| ISSUE-014 | HIGH | File | Sem autosave/recovery na UI Slint. | OPEN |
| ISSUE-015 | MEDIUM | Model | `model.instantiate_asset` permanentemente desabilitado. | OPEN |
| ISSUE-016 | MEDIUM | Global | Atalhos: teclas anunciadas pelo keymap consumidas sem efeito. | PARTIAL — as ações dos keymaps distribuídos (extrude, push/pull, knife, inset, bevel, subdivide, merge, loop cut, primitivas, cinemática de modo, paint size/hardness) estão roteadas; duplicar/separar/juntar/fill/frame all ainda não têm bind nos keymaps. |
| ISSUE-017 | MEDIUM | Global | Estatísticas decorativas na UI. | OPEN |
| ISSUE-018 | HIGH | Global | Controles numéricos, sliders, dropdowns, modais e popovers do shell ainda não existem como componentes reutilizáveis. | OPEN |
| ISSUE-019 | HIGH | Global | Strings públicas da UI Slint são literais em inglês; sistema de `TextId` não está ligado ao shell. | OPEN |
| ISSUE-020 | MEDIUM | Global | Tokens do shell cobrem apenas parte da paleta do contrato (faltam `surface-overlay`, `control-*`, `disabled`, estados de perigo/aviso). | OPEN |
