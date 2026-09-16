# UI Ecosystem Lock — ledger de dependências de UI

<aside>
🔒

**Ledger obrigatório da Wave 2** (§44 da *Egui Ecosystem Final Push Directive*).
Toda dependência de UI declara aqui: fonte, versão, revisão, versão de `egui`,
razão, adapter, se entra no shipping e a **condição de saída** para voltar ao
upstream.

Sem entrada neste arquivo, uma dependência de UI não pode ser adicionada.

</aside>

# Invariante absoluta

> **Uma única família `egui` no shipping graph.**

Verificação:

```bash
cargo tree -d          # não pode listar duas versões de egui/egui_extras/eframe
cargo tree -p petunia_ui | grep -c "egui v"
```

Estado atual (2026-09-16, re-verificado depois das Waves 6 e 7): **verificado.**
`cargo tree -d` só reporta duplicações não relacionadas a UI (`base64`,
`bitflags`, `darling`, `png`, `glam`). Nenhuma segunda família `egui` no grafo
empacotado — as três entradas da Wave 7 (`egui_dnd` 0.17, `egui_animation` 0.13,
`egui_form` 0.10) declaram `egui ^0.36.0`, a mesma `0.36.2` do baseline.

> A `egui_animation` entra também como **transitiva** do `egui_dnd`: uma linha
> fina a menos para manter. Se o DnD sair do grafo um dia, o motion precisa
> declará-la explicitamente.

# Ordem normativa de compatibilidade (§18)

Uma crate publicada em versão mais antiga **não** é, sozinha, evidência de
incompatibilidade. A avaliação segue esta ordem — só avança um passo quando o
anterior falha:

1. release no `crates.io`;
2. revisão atual do upstream;
3. **revision pin** de um SHA testado;
4. **fork mínimo Petunia** (§19.2);
5. implementação local ou defer.

# Baseline instalada

Dependências de UI realmente presentes no grafo, com versão resolvida no
`Cargo.lock`.

| crate | source | version | revision | egui | por quê | adapter | shipping? | saída para o upstream |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `egui` | crates.io | 0.36.2 | — | 0.36.2 | runtime de UI | — | sim | acompanhar releases |
| `eframe` | crates.io | 0.36.2 | — | 0.36.2 | host desktop | — | sim | — |
| `egui-wgpu` | crates.io | 0.36.2 | — | 0.36.2 | host GPU canônico | — | sim | — |
| `egui-winit` | crates.io | 0.36.2 | — | 0.36.2 | integração de janela/input | — | sim | — |
| `egui_taffy` | crates.io | 0.14.0 | — | 0.36 | layout responsivo (flex/grid/wrap) | `adapters::taffy_layout` | sim | quando `egui` nativo cobrir o caso |
| `egui_tiles` | crates.io, `default-features = false` | 0.17.1 | — | 0.36 | macro-layout controlado do shell (topologia, divisórias, paines núcleo protegidos) | `adapters::tile_layout` | sim | manter só se `egui` nativo cobrir shell com paines protegidos |
| `twill` | crates.io, `default-features = false` | 0.2.0 | — | **n/a** | tokens tipados de spacing/radius | `adapters::twill_tokens` | sim | — |
| `egui_inbox` | crates.io | 0.13.0 | — | 0.36 | resultado de worker → frame da UI | `adapters::inbox` | sim | — |
| `egui_ltreeview` | crates.io | 0.9.0 | — | 0.36 | árvore Parts/Scene (seleção múltipla, culling, a11y) | `adapters::tree` | sim | — |
| `egui_extras` | crates.io, features `image`, `svg` | 0.36.2 | — | 0.36.2 | loaders de imagem e tabelas auxiliares | uso direto em componentes | sim | — |
| `iconflow` | crates.io, features `pack-lucide`, `pack-iconoir` | 1.0.0 | — | **n/a** | carga dos packs genéricos de ícones | `adapters::icons` | sim | — |
| `egui-file-dialog` | crates.io | 0.15.0 | — | 0.36 | browse/save dentro da janela | `adapters::file_dialog` | sim | — |
| `transform-gizmo` + `transform-gizmo-egui` | crates.io | 0.11.0 | — | 0.36 | gizmo 3D de transformação | `adapters::gizmo` | sim | — |
| `egui_dnd` | crates.io | 0.17.0 | — | 0.36 | reordenação por arrasto em listas ordenáveis (Wave 7) | `adapters::drag_drop` | sim | quando o egui nativo cobrir drag-to-reorder com handle |
| `egui_animation` | crates.io (transitiva de `egui_dnd`) | 0.13.0 | — | 0.36 | backend de motion atrás de `foundation::motion` | `foundation::motion` | sim | quando `Context::animate_*` cobrir curva + collapse |
| `egui_form` | crates.io | 0.10.0 | — | 0.36 | validação por campo atrás do contrato `adapters::form` | `adapters::form` | sim | — |

> `twill` é usado **sem** o backend `egui` (que fixa `egui 0.33`). É o que mantém
> a invariante de família única: o adapter usa só os tokens e a serialização CSS.
>
> `egui_tiles` entra **sem** `default-features` (o default liga `serde` +
> `egui/serde`). O contrato durável do shell é o DTO Petunia
> ([`PetuniaShellLayout`]), nunca a árvore da crate (§31.3) — então a
> serialização da crate não tem por que existir no grafo.
>
> Estado da migração (Wave 5): a dependência, o adapter e suas regras estão
> **prontos e testados**; a troca do shell para a árvore é o passo seguinte
> (Wave 5b). Até lá nada do shell monta `Tree` — nenhum caminho tem duas
> engines de docking.

## Três armadilhas medidas do `egui_taffy` 0.14

Comportamento verificado nas Waves 3 e 4 (sondas descartáveis + testes
permanentes no adapter). Registrado aqui porque as três falham **em silêncio** —
o layout sai plausível, só não é o pedido.

1. **`flex_basis` em percentual resolve para `0`.** `LengthPercentage::percent(1.0/3.0)`
   é avaliado contra um pai de tamanho automático na passagem de medida e volta
   zero; a célula colapsa para a largura do próprio conteúdo. Consequência real:
   `PetuniaLayoutMode::Grid { columns }` **nunca** produziu colunas iguais — era
   uma linha. Use comprimento definido: o adapter calcula `column_width` e passa
   `LengthPercentage::length(w)`.
2. **`tui.style(..)` precisa ser encadeado com o `add`.** `let _ = tui.style(st);`
   seguido de `tui.ui(..)` **descarta** o estilo antes do filho. Só
   `tui.style(st).ui(..)` aplica. Era a segunda metade do bug acima.

3. **O `Ui` de cada item herda o layout do pai.** `add_container_dyn` cria o `Ui`
   do item com `UiBuilder::new()` — sem layout explícito, ele copia o layout do
   `Ui` que chamou o `tui(..)`. Dentro de um painel `top_down`, um item que
   desenha vários filhos **direto** no `Ui` recebido empilha em vez de ficar lado
   a lado. Medido no produto (Wave 4, barra da viewport): uma linha que devia ter
   24px media **101px**, com `viewport.menus` reportando 97px e
   `viewport.display` 101px. Corrigido com
   `PetuniaResponsiveLayout::with_item_layout(PetuniaItemLayout::Row)`, que passa
   `egui_layout(Layout::left_to_right(Align::Center))` ao item. O padrão segue
   `Inherit` (o comportamento histórico), então cabe a quem monta a linha dizer
   se o item é uma linha ou uma célula.

Um quarto detalhe de projeto (não bug): dentro de uma célula, `ui.available_width()`
é `0` na passagem de medida e a largura resolvida só na final. É por isso que o
adapter entrega a largura ao componente em vez de o componente medi-la — com
largura definida desde a primeira passagem, o valor é o mesmo nas duas.

## Armadilhas medidas da Wave 6 (`egui_taffy` em product path)

1. **O callback de item roda mais de uma vez por item.** A grade desenha o item
   uma vez para conhecê-lo (medida) e outra para posicioná-lo. Medido no teste
   `adapters::tool_grid::every_item_receives_the_same_resolved_cell`: 6 itens
   ⇒ **12 chamadas** por frame. Consequência: o callback é um desenho, não um
   acumulador (nada de `push` em vetor nem contador de frame lá dentro). Mutações
   de produto continuam seguras porque reagem a `clicked()`, que só é verdadeiro
   na passagem real de interação.
2. **Largura real dos controles vizinhos (medida em sonda):** o botão de seta de
   um grupo (`small_button("▾")`) ocupa **21.4px** no tema padrão — o token
   declara 22 (`tokens::TOOLBAR_CHEVRON_WIDTH`) e não menos. Somado ao botão
   compacto (40) e ao vão (2), a linha de grupo pede **63.4px**.
3. **A célula da paleta é a coluna menos 10px.** Os dois respiros da moldura do
   paine (4+4) e o arredondamento do motor. Medido: coluna de 74px ⇒ célula de
   64px. É essa perda que `tokens::TOOLBAR_CHROME_WIDTH` declara, para que o
   mínimo da coluna (`TOOLBAR_MIN_WIDTH` = 74) possa falar a língua da célula.
4. **Nada é recortado por acidente — a seta era.** Antes da Wave 6, com a coluna
   em 40/48/64px o conteúdo da paleta ocupava **74px**: a seta do menu da família
   era desenhada fora do paine e ficava invisível e inalcançável. Depois do
   arranjo derivado (`plan_group`) e do mínimo derivado, a largura ocupada é
   **exatamente** a da coluna em 74–300px
   (`crates/ui/tests/toolbar_fit.rs`).

## Duas armadilhas medidas do `egui_tiles` 0.17.1

Verificadas na Wave 5 com sonda descartável + testes permanentes no adapter.

1. **As shares de um container linear são normalizadas pela soma, não pela
   largura.** `Shares::split(children, available)` divide `available` proporcionalmente
   ao valor de cada share; um filho sem share explícito fica com o default
   (`1.0`) e domina o container. Consequência medida: pedindo dock de 300px em
   1280px, o dock media **231.8px**, porque o centro manteve o share default.
   O adapter agora escreve o share das **três** colunas (pixels / span).
2. **Um `egui::Panel` pode viver dentro do `Ui` de um tile** — e é isso que
   torna a migração do shell incremental em vez de um rewrite. Medido: tile de
   199.8px de largura, `Panel::left` interno ocupando 8..51px **dentro do tile**.
   Consequência prática: a migração pode trocar painel por tile por região, sem
   reescrever o conteúdo de cada painel de uma vez (§60).

## Três armadilhas medidas do `egui_dnd` 0.17 e do `egui_animation` 0.13

1. **O arrasto só começa com o ponteiro sobre o handle.** O motor passa de
   "esperando limiar" para "pode arrastar" quando o ponteiro andou mais de 1px
   **e** `contains_pointer()` é verdadeiro — o caminho alternativo é 250ms de
   pressão (`click_tolerance_timeout`). Medido em sonda: um primeiro passo de 6px
   para fora de um grip de 18×20px não inicia o arrasto; o mesmo passo dentro do
   grip inicia. Consequência de projeto: o grip é a área de aquisição do gesto,
   não decoração (`PetuniaDragSpec::grip_width/height`).
2. **`to` é limite exclusivo quando o arrasto desce.** `to = 2` partindo de `0`
   deixa o item na posição **1**. O adapter converte para a semântica de inserção
   do Petunia e a equivalência é verificada contra `egui_dnd::utils::shift_vec`
   para todos os pares de posições.
3. **O tempo de animação é em segundos, não milissegundos.** `egui_animation` e
   o próprio egui esperam segundos: passar `160` (em vez de `0,16`) deixa a
   transição ~1000× mais lenta — medido, **0,1% por frame** em vez de 160ms.
   Nada quebra, só parece que a animação não existe. `foundation::motion::seconds`
   é a única conversão usada, e um teste de taxa falha se a unidade regredir.

## Medir uma faixa sem desenhar duas vezes (técnica da Wave 4)

O adapter da barra responsiva (`adapters::toolbar`) precisa da largura **real** de
cada faixa antes de decidir o overflow. A técnica, verificada por teste:

- desenhe a faixa num `Ui` filho em coordenadas muito negativas
  (`UiBuilder::new().max_rect(..)`), com `id_salt` próprio:
  - o ponteiro nunca está ali, então nada é hoverado ou clicado;
  - o `Id` diferente mantém popups fechados na sondagem (mede-se o botão, não o
    menu aberto);
- dê ao retângulo **extensão cruzada zero** (`vec2(SPAN, 0.0)` para uma linha):
  num layout horizontal o `min_rect` do egui cresce até a extensão cruzada do
  `max_rect`, então um retângulo alto devolveria a altura da sonda. Com a cruz em
  zero, `min_rect().size()` traz a largura **e** a altura do conteúdo (22–24px);
- o filho é criado com `ui.new_child(..)`, **não** com `ui.scope_builder(..)`:
  `scope` avança o cursor do pai (`advance_cursor_after_rect`), o que deslocaria a
  barra real; `new_child` só devolve o `Ui` para medir.

O `Separator` usado como divisor é medido no mesmo esquema: sua **largura** numa
linha (o `spacing` do tema) é o que entra na aritmética de overflow.

# Feature-gated (não entram no build padrão)

| crate | source | version | egui | por quê | feature | shipping? |
| --- | --- | --- | --- | --- | --- | --- |
| `egui_kittest` | crates.io | 0.36.2 | 0.36.2 | testes semânticos/visuais de UI | `dev-dependencies` | não |
| `egui_inspection` | crates.io | 0.36.2 | 0.36.2 | inspeção de árvore de a11y/input | `devtools` | não |
| `egui_mcp` | crates.io | 0.2.0 | 0.36 | ponte para agentes em desenvolvimento | `devtools` | não |
| `egui-probe` | crates.io | 0.13.0 | 0.36 | painéis de debug derivados | `devtools` | não |
| `egui_commonmark` | crates.io | 0.25.0 | 0.36 | ajuda/release notes em Markdown | `help-markdown` | não |
| `egui_autocomplete` | crates.io | 15.0.0 | 0.36 | completion da command palette | `palette-autocomplete` | não |

# P0 alvo — todas adotadas (Wave 7)

Classificação da diretiva §21. Cada linha só vira dependência quando o gate de
compatibilidade (§18 e §67) passa **e** o adapter correspondente existe.

| crate | versão alvo | adapter | estado |
| --- | --- | --- | --- |
| `egui_dnd` | 0.17 | `adapters::drag_drop` | ✅ instalada; product path: menu de configuração da paleta |
| `egui_animation` | 0.13 | `foundation::motion` | ✅ instalada (transitiva do `egui_dnd`); product paths: revelação da busca (Outliner/Inspector) |

# P1 — adotar em product paths

| crate | uso | adapter | estado |
| --- | --- | --- | --- |
| `egui_form` | validação por campo em Settings/forms | backend de `adapters::form` | ✅ instalada; product path: conflitos de keymap |
| `egui_table` | listas/tabelas do Context e Asset Library | `adapters::table` | não instalada — Wave 8/9 |
| `egui_virtual_list` | coleções grandes (Asset Library) | `adapters::virtual_collection` | não instalada — Wave 8 |
| `egui_suspense` | estados async/loading declarativos | `adapters::suspense` | não instalada — Wave 9 |
| `egui-notify` | notificações | — | exige **fork mínimo** (§19.3) — ver [`forks.md`](./forks.md) |

# Git pin e forks

Nenhuma dependência de UI usa hoje `git =` nem `[patch.crates-io]`. Quando a
primeira existir, ela precisa de entrada em [`forks.md`](./forks.md) **antes** de
entrar no manifesto (§19.5).

# Como manter este arquivo

| Evento | Ação |
| --- | --- |
| adicionar dependência de UI | nova linha aqui + adapter + teste |
| atualizar versão | atualizar `version` e o resultado de `cargo tree -d` |
| usar `git =` | preencher `revision` com SHA e registrar em `forks.md` |
| criar fork | registrar em `forks.md` com política de saída |
| remover dependência | mover a linha para "Removidas" com data e motivo |
