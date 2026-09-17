# 00 — Drift do `ui-map.json` (registro, não decidido)

Status: **aberto** · Registrado em 2026-09-16 · Branch `paint/ui-redesign`

## O que aconteceu

`cargo run -p xtask -- docs-check` falha no passo do mapa de componentes:

```text
Error: ui-map.json: símbolo 'draw' ausente em crates/ui/src/toolbar.rs (nó 'toolbar')
```

O mapa é validado contra o código por símbolo de entrada
(`crates/xtask/src/main.rs::task_ui_check`: procura `fn <entry>(`, `struct <entry>`,
`enum <entry>` ou `mod <entry>` no arquivo do nó). `docs/public/ui-map.json` está
no **site público congelado** (AGENTS.md §1), então corrigi-lo exige decisão
explícita de descongelamento — nada foi alterado.

## Por que não é deste trabalho

Os sete nós já estavam quebrados no commit base da branch (`cbc267c`, "estado
atual do repo antes das branches de paint"), ou seja: o drift precede tanto o
motor de Paint quanto o redesenho de UI.

```bash
git show HEAD:crates/ui/src/toolbar.rs | grep -c 'fn draw('   # 0
grep -c 'fn draw(' crates/ui/src/toolbar.rs                    # 0
```

## Os sete nós (verificado node a node contra o código atual)

| Nó | Arquivo | `entry` hoje | Situação real do código |
| --- | --- | --- | --- |
| `toolbar` | `crates/ui/src/toolbar.rs` | `draw` | A entrada é `draw_contents`; `draw` não existe desde `cbc267c` |
| `right_dock` | `crates/ui/src/lib.rs` | `right_panel` | Não existe: a coluna do dock é o paine `Parts`+`Context` do `adapters::tile_layout`; a entrada atual é `shell::draw` |
| `right_outliner` | `crates/ui/src/lib.rs` | `draw_split_dock` | Não existe: a seção é o paine `Parts` (`outliner::draw_body`) |
| `right_inspector` | `crates/ui/src/lib.rs` | `draw_split_dock` | Não existe: a seção é o paine `Context` (`properties_panel::draw`) |
| `detached_inspector_window` | `crates/ui/src/lib.rs` | `right_panel` | Não existe: é `shell::draw_detached_inspector` (janela flutuante) |
| `animate_center_strip` | `crates/ui/src/lib.rs` | `animate_workspace_center` | Não existe: a timeline é o paine `Bottom` (só com a feature `animation-workspace`) |
| `dock_split_helper` | `crates/ui/src/regions.rs` | `scene_panel_height` | Não existe: a política vive em `PetuniaShellLayout::dock_pane_sizes` |

Fonte: varredura de todos os 193 nós do mapa com o mesmo probe do xtask
(`fn <entry>(` / `struct` / `enum` / `mod`), 7 falhas, as demais 186 conformes.

## Impacto

- Nenhum: o mapa é documentação de componentes do site congelado; o código e os
  testes não dependem dele.
- `docs-check` fica vermelho **apenas** neste passo. Todo o resto passa:
  `bible-check` (caderno, links, vocabulário, congelamento do site intacto com 814
  arquivos), `docs-generate --check` (sem drift em `docs/generated/`), changelog
  vivo sincronizado e `ui-check` até o primeiro nó divergente.

## Caminhos possíveis (decisão do orquestrador)

1. **Manter como está** (escolhido nesta sessão): `docs-check` vermelho nesse passo
   até a decisão de descongelar; nenhuma página do caderno é afetada.
2. **Descongelar o mapa**: reescrever as sete entradas contra o código atual
   (`toolbar::draw_contents`, `shell::draw`, paines `Parts`/`Context`/`Bottom` do
   adapter) e rodar `cargo run -p xtask -- bible-lock` para regravar o hash do
   arquivo — a decisão fica registrada no próprio lock.
3. **Corrigir só o que o Paint tocou**: mistura descongelamento parcial com dívida
   remanescente; não recomendado.

O caminho 2 é o único que devolve `docs-check` verde de ponta a ponta, e é uma
edição de documentação — nenhum código depende desse arquivo.
