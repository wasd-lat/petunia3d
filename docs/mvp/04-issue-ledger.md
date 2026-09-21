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
| ISSUE-005 | BLOCKER | Model | Cut/Knife cria sessão que nada alimenta. | FIXED — cada clique resolve o raio e usa `pick_edge`; o primeiro ancora, o segundo corta com uma entrada de undo, topologia recusada mantém a âncora e `Esc` restaura a ferramenta de seleção. |
| ISSUE-006 | BLOCKER | File | `file.export_obj`, `file.export_glb` e `file.import_obj` eram stubs na palette. | FIXED |
| ISSUE-007 | BLOCKER | Model | `model.loop_cut` inalcançável por depender de seleção de aresta. | FIXED — sessão interativa: o anel é descoberto na abertura, arrastar na viewport desliza, o painel mostra slide e contagem de cortes (1..=32) com Apply/Cancel, o commit restaura o snapshot antes do checkpoint e o undo volta à malha exata. |
| ISSUE-008 | HIGH | Model | Extrude/Inset/Bevel/Extrude Individual/Scale Selection one-shot. | PARTIAL — Extrude, Extrude Individual, Inset, Bevel, Push/Pull e Scale Selection viraram sessões paramétricas com preview, Tool Properties e undo único. **Fuse/Cut/Intersect** entraram como comandos canônicos com operando explícito. Faltam Draw/Profile, Bridge, Connect, Keep Parts e Join como fluxos de UI. |
| ISSUE-009 | HIGH | Paint | Sem UI de layers, effects, fill scope, lock, projeção, formas e canvas 2D. | PARTIAL — camadas completas; fill scope (5 escopos) com flood fill e rasterização de polígono UV; projeção Surface/ScreenSpace; trava de pincel; Line e Rectangle ancorados; **canvas 2D visível** mostrando os pixels reais da camada ativa, atualizado a cada evento de pintura. Falta a camada de efeito (`PaintLayer::new_effect` existe no domínio sem UI) e a grade de pixels. |
| ISSUE-010 | HIGH | UV | Sem UV Editor 2D; seams e diagnósticos inacessíveis. | PARTIAL — editor 2D desenha o contorno real, **seleciona a face sob o clique** (substitui sem Shift, acumula com Shift) e **move/escala/rotaciona as UVs selecionadas** com undo. Faltam visualização de ilhas por cor, seams visíveis no editor e seleção por retângulo. |
| ISSUE-011 | HIGH | Global | Sem splitters reais; painéis não redimensionam nem persistem. | FIXED — divisor vertical no dock Context (208..560 px, dono `UiState::right_width`) e divisor horizontal na Asset Library (132..520 px, dono `UiState::shell_asset_library_height`); a memória de layout por workspace passou a carregar a largura do inspetor. |
| ISSUE-012 | HIGH | Global | Sem Outliner funcional completo (rename F2, reorder, context menu). | PARTIAL — seleção, visibilidade, lock, exclusão e rename inline com F2 funcionam; faltam reorder por arrasto, context menu e busca. |
| ISSUE-012a | MEDIUM | Global | `key_code_from_slint` não mapeava nenhuma tecla de função (F1..F12), então qualquer atalho de tecla de função anunciado pelo keymap era descartado em silêncio. | FIXED — F1..F12 e Escape passaram a ser resolvidos; `global.rename = F2` é o primeiro consumidor. |
| ISSUE-013 | HIGH | Global | Sem menus File/Edit/View/Window; i18n ausente na UI Slint. | FIXED — barra de menus com File/Edit/View/Window ligada a comandos canônicos e 31 `TextId` cobrindo o chrome do shell; a lista de temas vem do `ThemeRegistry`. |
| ISSUE-014 | HIGH | File | Sem autosave/recovery na UI Slint. | FIXED — ciclo de vida completo: session lock no arranque, timer de 30s oferecendo o passo (intervalo e dirty ficam no domínio), diálogo de recuperação com Recuperar / Abrir salvo / Descartar e remoção do lock no fechamento limpo. |
| ISSUE-015 | MEDIUM | Model | `model.instantiate_asset` permanentemente desabilitado. | FIXED — cada cartão da Asset Library tem ação de colocar no cursor 3D, com uma entrada de undo e identidade própria. |
| ISSUE-016 | MEDIUM | Global | Atalhos: teclas anunciadas pelo keymap consumidas sem efeito. | PARTIAL — as ações dos keymaps distribuídos estão roteadas; duplicar/separar/juntar/fill/frame all ainda não têm bind nos keymaps. |
| ISSUE-017 | MEDIUM | Global | Estatísticas decorativas na UI. | FIXED — o painel UV trocou "LSCM / ABF++ Conformal" e "Texel Density: Auto" por diagnósticos reais de malha e nome real do provider; `scene_stats` usa `scene_tris`/`scene_verts`; os defaults fabricados saíram do markup. |
| ISSUE-018 | HIGH | Global | Controles numéricos, sliders, dropdowns, modais e popovers do shell ainda não existem como componentes reutilizáveis. | OPEN |
| ISSUE-019 | HIGH | Global | Strings públicas da UI Slint são literais em inglês; sistema de `TextId` não está ligado ao shell. | PARTIAL — 31 `TextId` ligados ao shell (menus, chrome de painéis, diálogo de recuperação) e as duas linhas que mentiam foram substituídas por valores reais. Restam literais em áreas de conteúdo (propriedades de material, seções de paint/UV). |
| ISSUE-020 | MEDIUM | Global | Tokens do shell cobrem apenas parte da paleta do contrato (faltam `surface-overlay`, `control-*`, `disabled`, estados de perigo/aviso). | OPEN |
| ISSUE-021 | MEDIUM | Global | Divisores: só o dock Context e a Asset Library são redimensionáveis; a coluna de ferramentas e as duas colunas de ferramentas do MODEL continuam fixas. | OPEN |
| ISSUE-022 | MEDIUM | Global | O divisor da Asset Library não tem memória por workspace e o divisor não aparece quando a gaveta está fechada, então abrir a gaveta é pré-requisito para ajustar altura. | OPEN |
| ISSUE-023 | HIGH | Global | Divisores não têm caminho de teclado: a `accessible-action-set-value` foi removida até os divisores suportarem passo por seta, então leitor de tela só anuncia o valor. | OPEN |
| ISSUE-024 | P0 | MCP | `cargo test --workspace` está vermelho: `petunia_mcp::server::tests::execute_intent_uses_application_commands` e `add_undo_roundtrip` falham. | PRE-EXISTING — reproduzido em `fd194e0` com `crates/core/` revertido ao baseline, ou seja, não foi introduzido por esta linha de trabalho. Diagnóstico: (a) o primeiro teste passa `asset_id: Some("Plane")`, mas `intent_well_formed` exige UUID (`crates/mcp/src/server.rs:127`), então o contrato mudou e o teste não; (b) o segundo espera um único undo depois de `add_primitive`, mas `AddPrimitiveCmd` agora abre sessão de criação e `is_destructive()` é `false` (`crates/core/src/command.rs:1159`), então o número de entradas na pilha precisa ser reconferido. Não corrigir reescrevendo a expectativa sem antes decidir qual das duas é o comportamento canônico. |
| ISSUE-025 | P0 | Docs | `cargo run -p xtask -- docs-check` e `bible-check` não podem passar: o lock do site congelado referencia `docs/.vitepress/.env.local`, arquivo que nunca foi versionado (`.gitignore` linha 17: `.env*`) e portanto não existe em checkout limpo. | PRE-EXISTING — provado rodando `bible-check` com as mudanças desta linha de trabalho em `git stash`: o erro `✖ removido: docs/.vitepress/.env.local` persiste idêntico. **Bloqueado por decisão**: corrigir exige editar `docs/audits/bible-conformance/frozen-site.lock.json` ou rodar `bible-lock`, e o AGENTS.md §0 proíbe editar o lock por conta própria (descongelamento é decisão explícita do mantenedor). |
| ISSUE-026 | HIGH | Docs | `docs/generated/COMMANDS.md` estava defasado em 14 comandos e o gerador emite linhas duplicadas para `edit.delete` e `edit.duplicate`. | FIXED — a causa era `CommandDispatcher::alias` copiar a metadata do dono sem reescrever o `id`; o clone agora carrega o id do alias e um teste percorre `all_metadata()` garantindo que nenhum id se repete. A defasagem de 14 comandos também foi corrigida por regeneração. |
| ISSUE-027 | MEDIUM | Config | `Keybinds::load_profile` resolve `assets/keymaps/*.toml` relativo ao diretório de trabalho e falha em silêncio, caindo na lista canônica interna. Rodando o binário instalado fora da raiz do workspace, nenhum perfil é lido. | OPEN — descoberto ao escrever o teste do perfil de notebook, que só conseguiu validar o arquivo lendo por caminho absoluto. |

## Rodada de fechamento — booleanas e formas

| Issue | Entrega |
|---|---|
| NOVO | `model.fuse`, `model.cut`, `model.intersect`: operações booleanas pelo provedor `manifold-rust`, com operando explícito escolhido no Outliner, uma entrada de undo e recusas com motivo próprio. O provedor existia desde P0-04 e nunca era chamado pelo produto. |
| NOVO | `FillScope` honrado de verdade no canvas: `ConnectedPixels` por flood fill com tolerância, `Object` no canvas todo, `Face`/`SelectedFaces`/`UvIsland` por rasterização par-ímpar do polígono UV. |
| NOVO | `BrushLock` e `BrushProjectionMode` ligados à sessão, com reset da trava por traço. |
| NOVO | Ferramentas **Line** e **Rectangle** ancoradas no press e confirmadas no release, com cancelamento por Escape. |
