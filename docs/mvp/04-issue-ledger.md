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
| ISSUE-008 | HIGH | Model | Extrude/Inset/Bevel/Extrude Individual/Scale Selection one-shot. | PARTIAL — Extrude, Extrude Individual, Inset, Bevel, Push/Pull e Scale Selection viraram sessões paramétricas com preview, Tool Properties e undo único. |
| ISSUE-009 | HIGH | Paint | Sem UI de layers, effects, fill scope, lock, projeção, formas e canvas 2D. | OPEN |
| ISSUE-010 | HIGH | UV | Sem UV Editor 2D; seams e diagnósticos inacessíveis. | PARTIAL — o painel de estatísticas agora mostra ilhas, sobreposições, faces de área zero, cantos fora de faixa e stretch reais. Falta o editor 2D, seams e visualização de ilhas. |
| ISSUE-011 | HIGH | Global | Sem splitters reais; painéis não redimensionam nem persistem. | FIXED — divisor vertical no dock Context (208..560 px, dono `UiState::right_width`) e divisor horizontal na Asset Library (132..520 px, dono `UiState::shell_asset_library_height`); a memória de layout por workspace passou a carregar a largura do inspetor. |
| ISSUE-012 | HIGH | Global | Sem Outliner funcional completo (rename F2, reorder, context menu). | PARTIAL — seleção, visibilidade, lock e exclusão funcionam; faltam rename, reorder e context menu. |
| ISSUE-013 | HIGH | Global | Sem menus File/Edit/View/Window; i18n ausente na UI Slint. | OPEN |
| ISSUE-014 | HIGH | File | Sem autosave/recovery na UI Slint. | OPEN |
| ISSUE-015 | MEDIUM | Model | `model.instantiate_asset` permanentemente desabilitado. | OPEN |
| ISSUE-016 | MEDIUM | Global | Atalhos: teclas anunciadas pelo keymap consumidas sem efeito. | PARTIAL — as ações dos keymaps distribuídos estão roteadas; duplicar/separar/juntar/fill/frame all ainda não têm bind nos keymaps. |
| ISSUE-017 | MEDIUM | Global | Estatísticas decorativas na UI. | FIXED — o painel UV trocou "LSCM / ABF++ Conformal" e "Texel Density: Auto" por diagnósticos reais de malha e nome real do provider; `scene_stats` usa `scene_tris`/`scene_verts`; os defaults fabricados saíram do markup. |
| ISSUE-018 | HIGH | Global | Controles numéricos, sliders, dropdowns, modais e popovers do shell ainda não existem como componentes reutilizáveis. | OPEN |
| ISSUE-019 | HIGH | Global | Strings públicas da UI Slint são literais em inglês; sistema de `TextId` não está ligado ao shell. | OPEN |
| ISSUE-020 | MEDIUM | Global | Tokens do shell cobrem apenas parte da paleta do contrato (faltam `surface-overlay`, `control-*`, `disabled`, estados de perigo/aviso). | OPEN |
| ISSUE-021 | MEDIUM | Global | Divisores: só o dock Context e a Asset Library são redimensionáveis; a coluna de ferramentas e as duas colunas de ferramentas do MODEL continuam fixas. | OPEN |
| ISSUE-022 | MEDIUM | Global | O divisor da Asset Library não tem memória por workspace e o divisor não aparece quando a gaveta está fechada, então abrir a gaveta é pré-requisito para ajustar altura. | OPEN |
| ISSUE-023 | HIGH | Global | Divisores não têm caminho de teclado: a `accessible-action-set-value` foi removida até os divisores suportarem passo por seta, então leitor de tela só anuncia o valor. | OPEN |
| ISSUE-024 | P0 | MCP | `cargo test --workspace` está vermelho: `petunia_mcp::server::tests::execute_intent_uses_application_commands` e `add_undo_roundtrip` falham. | PRE-EXISTING — reproduzido em `fd194e0` com `crates/core/` revertido ao baseline, ou seja, não foi introduzido por esta linha de trabalho. Diagnóstico: (a) o primeiro teste passa `asset_id: Some("Plane")`, mas `intent_well_formed` exige UUID (`crates/mcp/src/server.rs:127`), então o contrato mudou e o teste não; (b) o segundo espera um único undo depois de `add_primitive`, mas `AddPrimitiveCmd` agora abre sessão de criação e `is_destructive()` é `false` (`crates/core/src/command.rs:1159`), então o número de entradas na pilha precisa ser reconferido. Não corrigir reescrevendo a expectativa sem antes decidir qual das duas é o comportamento canônico. |
