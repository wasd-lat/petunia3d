# Arquitetura de UI (egui)

UI em modo imediato com `egui`, sem retained tree própria. O shell é composto por
painéis (`Panel::top/bottom/left/right/central`) com ordem canônica — Header →
StatusBar → laterais → toolbar da viewport → centro.

## Regra de layout (normativa)

> **Raw egui para composição simples. Bibliotecas especializadas para problemas
especializados. Petunia Components e Adapters como única superfície permitida
para product UI.**

Fonte: `PETUNIA3D_EGUI_ECOSYSTEM_FINAL_PUSH_DIRECTIVE.md` §17 (diretriz
arquitetural temporariamente normativa) e capítulos 35/36 do Livro Vivo.

Matriz obrigatória:

| Problema | Solução normativa |
| --- | --- |
| micro row / micro column | `egui` built-in |
| layout complexo ou responsivo | `PetuniaTaffyLayout` (`egui_taffy`) |
| flex / wrap / grid | Taffy |
| colunas de largura igual (campos, abas) | `columns` + `PetuniaColumnSpec` |
| grade de itens com colunas derivadas (paleta) | `PetuniaToolGridSpec` (`adapters::tool_grid`) |
| campo rotulado (rótulo + controle) | `PetuniaForm` (`adapters::form`) |
| largura de controle que cede ao espaço | `clamped_width` / `fill_remaining` |
| barra de faixas com overflow (toolbars) | `PetuniaResponsiveToolbar` (`adapters::toolbar`) |
| macro shell | `PetuniaLayoutAdapter` (`egui_tiles`) |
| árvore Parts / Scene | `PetuniaTreeAdapter` (`egui_ltreeview`) |
| reorder / drag list | `PetuniaDragList` (`egui_dnd`) |
| animação (reveal, valor, posição, collapse) | `PetuniaMotion` (`egui_animation`) |
| validação por campo + resumo inline | `PetuniaFormSession` (`egui_form`) |
| async → UI | `PetuniaInboxAdapter` (`egui_inbox`) |
| ícones | `IconRegistry` / iconflow / Petunia Domain |
| transforms 3D | adapter sobre `transform-gizmo` |
| valores de tema | tokens Petunia + adapter Twill |

Proibido em product code (fora de foundation/adapter e das exceções de §36 —
viewport, canvas UV, régua da timeline, gizmo, visualização de dados e
implementação de componente de baixo nível):

```rust
if ui.available_width() < 560.0            // responsividade manual
let w = (available - n * gap) / count;     // divisão manual de largura
ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
ui.allocate_exact_size(...)                // quando existe componente padrão
ui.painter().rect_filled(...)              // idem
ui.add_space(4.0)                          // idem
```

### Contrato de colunas (Wave 3)

Quem monta um arranjo de colunas iguais — três eixos de um campo vetorial, abas
contextuais — **não calcula largura**. O adapter resolve a largura e a entrega
pronta ao componente:

```rust
// componente (inspector_widgets.rs)
columns(
    ui,
    id,
    PetuniaColumnSpec::responsive(3, AXIS_UNIT_MIN_W, AXIS_GAP),
    |index, column_w, ui| { /* desenha com `column_w`, já definitiva */ },
    |_ui| (),
);
```

- `PetuniaColumnSpec::responsive(count, min_item, gap)` — quebra em linhas a
  partir do mínimo real do item. **É aqui que um breakpoint pode existir**, e ele
  é derivado do conteúdo, não escolhido a dedo (`avail >= 300` não é contrato).
- `PetuniaColumnSpec::fixed(count, gap)` — nunca quebra (abas, faixas fixas).
- `fit_columns` / `column_width` são as primitivas puras e testadas; nenhum
  painel as reimplementa.
- A largura chega correta já na **primeira passagem de medida** do `egui_taffy`,
  então o componente nunca lê `ui.available_width()` para se dimensionar.

### Barra responsiva (Wave 4)

Quem monta uma barra de faixas que precisa decidir o que cabe (**toolbar da
viewport**, e nas próximas waves a barra lateral e a status bar) **não soma
larguras nem mede o container**. Declara as faixas e o que pode cair:

```rust
// product code (viewport_bar.rs)
static VIEWPORT_BAR: PetuniaToolbarSpec = PetuniaToolbarSpec {
    primary: &[
        PetuniaToolbarCluster::pinned(SLOT_DOMAIN),
        PetuniaToolbarCluster::pinned(SLOT_MENUS),
    ],
    secondary: &[
        PetuniaToolbarCluster::overflowable(SLOT_TRANSFORM, 10),
        PetuniaToolbarCluster::overflowable(SLOT_SNAP_PROP, 20),
        PetuniaToolbarCluster::pinned(SLOT_DISPLAY),
    ],
    max_rows: 2,
    overflow: Some(SLOT_OVERFLOW),
};

let plan = toolbar.show(ui, &mut |ui, slot| draw_slot(ui, state, slot));
```

- `PetuniaToolbarCluster::pinned` — nunca sai da linha (núcleo do produto).
- `PetuniaToolbarCluster::overflowable(id, rank)` — cai para a faixa de acesso
  quando falta largura; `rank` maior cai depois.
- A faixa declarada em `overflow` entra no fim do plano **apenas** quando há algo
  oculto, e recebe a lista de ocultos em `PetuniaToolbarSlot::hidden`.
- O adapter mede cada faixa com uma sonda offscreen (largura e altura reais) e
  entrega a linha ao taffy; `plan_row` é pura e testável sem egui.
- Uma/duas linhas saem de `rows_that_fit(altura, altura_da_linha_medida,
  gap_vertical_do_tema, max_rows)` — **não** de um limiar de altura no produto.

Regra derivada: se um item do taffy é um grupo desenhado no `Ui` recebido, use
`PetuniaResponsiveLayout::with_item_layout(PetuniaItemLayout::Row)` — sem isso o
item herda o layout vertical do painel e empilha os próprios filhos.

### Top Bar de três zonas (Wave 5)

O cabeçalho **não** usa `ui.columns(3, …)`: colunas iguais põem o centro no meio
do terço do meio, não no meio da barra. O produto declara as três zonas e o
adapter mede, decide e distribui:

```rust
// product code (main_header.rs)
static TOP_BAR: PetuniaTopBarSpec = PetuniaTopBarSpec {
    left: &[PetuniaToolbarCluster::pinned(SLOT_MENUS)],
    center: &[PetuniaToolbarCluster::pinned(SLOT_WORKSPACES)],
    right: &[
        PetuniaToolbarCluster::overflowable(SLOT_ASSETS, 20),
        PetuniaToolbarCluster::overflowable(SLOT_SETTINGS, 30),
    ],
    overflow: Some(SLOT_OVERFLOW),
};

let bar = PetuniaTopBar::new(TOP_BAR_ID, &TOP_BAR);
bar.show(ui, &mut |ui, slot| draw_slot(ui, state, action, slot));
```

- `plan_top_bar(left_w, center_w, right, right_widths, available, gap, overflow)`
  é **pura**: decide o modo de centralização e o que cai, a partir de larguras já
  medidas.
- Com `PetuniaCentering::Centered`, as laterais recebem caixas de largura **igual** (base 0 + `flex_grow` igual): o centro cai
  no meio geométrico por construção. `three_zone` é a única peça que conhece o
  taffy.
- Quando a esquerda não cabe na metade, o modo vira `Drifting` (laterais
  empacotadas nas bordas) — escolhido **por medição**, não por palpite, e sempre
  com a seta de acesso pendurada na borda direita.
- A seta só existe quando há algo oculto; ela é sempre a última célula da direita.

### Macro-layout do shell (Wave 5)

O shell tem um DTO próprio (`PetuniaShellLayout`) e um adapter que o traduz para
uma árvore de layout:

```text
PetuniaShellLayout  →  PetuniaLayoutAdapter::tree  →  egui_tiles::Tree
```

- `PetuniaPane` são os cinco painéis reais (`Tools`, `Viewport`, `Parts`,
  `Context`, `Bottom`); cada um tem um **slot autorizado**.
- Nada fecha e nada arrasta: não existe docking livre, por construção e por teste.
- `clamped(available)` garante o mínimo da viewport **antes** de a árvore nascer:
  as laterais cedem primeiro (viewport-first).
- `show` devolve as larguras finais **em pixels** para o produto persistir — nunca
  a árvore da crate.

Estado: **em product path desde a Wave 5b** — `lib.rs::draw` chama
`shell::draw`, que descreve o layout do frame (`layout_for`), deixa o adapter
resolver a geometria e persiste o que o usuário mexeu. Os antigos helper de dock
(`right_panel`, `draw_split_dock`, `split_widths`, `scene_panel_height`) não
existem mais; ver
[`04-wave5-topbar-shell.md`](../audits/ui-ecosystem-final-push/04-wave5-topbar-shell.md).

### Paleta de ferramentas e grupo split (Wave 6)

A paleta **não** decide o próprio arranjo. Ela declara duas larguras e recebe a
célula resolvida:

```rust
// product code (toolbar.rs)
static TOOL_GRID: PetuniaToolGridSpec =
    PetuniaToolGridSpec::new(tokens::TOOLBAR_WIDTH, 120.0, 3.0);

grid.show(ui, id, group.len(), |index, cell, ui| {
    // cell.width  → largura definitiva da célula
    // cell.labeled → cabe ícone + rótulo nesta largura?
    draw_model_entry(ui, state, tools, group[index], cell)
}, |_ui| ());
```

- o número de colunas sai de `fit_columns(available, max_columns, min_cell, gap)`
  — **nunca** de um limiar de largura no produto;
- o grupo (split button) usa `plan_group(cell_width)`: com célula suficiente,
  botão + seta; sem, apenas o botão (menu pelo clique secundário, com dica no
  tooltip). Nenhum controle é desenhado fora da célula;
- a coluna de ferramentas tem mínimo **derivado** do que ela hospeda:
  `tokens::TOOLBAR_MIN_WIDTH` = ícone + vão + seta + moldura (74px), e
  `core::TOOLBAR_DEFAULT_WIDTH` espelha esse mínimo. Regressão em
  `crates/ui/tests/toolbar_fit.rs`.

### Contrato de formulário (Wave 6)

Campo = rótulo com largura declarada + controle com a largura que sobra:

```rust
// product code (settings_modal.rs)
static SETTINGS_FORM: PetuniaForm = PetuniaForm::new(96.0, 160.0);

SETTINGS_FORM.section(ui, &title, Some(&description));
SETTINGS_FORM.field(ui, "settings-density", &label, |ui, control_w| {
    // `control_w` é definitiva; o controle não mede o container
});
if SETTINGS_FORM.toggle(ui, &label, &mut value) { /* `changed()` */ }
```

- `plan_row` é **pura**: `Inline` exatamente quando o controle ainda cabe com
  `control_min` depois do rótulo; senão `Stacked`, com a faixa inteira para o
  controle. É por onde painel estreito, escala de UI maior e idioma longo ficam
  cobertos sem breakpoint;
- a linha `Inline` é a primitiva `taffy_layout::fixed_label_row` (uma linha taffy,
  rótulo de largura fixa + controle crescendo), e a largura entregue ao closure é
  a largura real do `Ui` do item;
- a validação por campo (crate `egui_form`) entrou na Wave 7 como **backend** do
  mesmo contrato — o arranjo continua sem depender dela:

```rust
let report = PetuniaValidationReport::new()
    .with_error("atalho.salvar", "compartilha o atalho com 'Salvar como'");
let mut session = PetuniaFormSession::new(report);
SETTINGS_FORM.validated_control(ui, &mut session, "atalho.salvar", |ui| badge(ui));
SETTINGS_FORM.error_summary_titled(ui, &session, Some(&title));   // resumo inline
session.reveal_errors(ui);                                        // "confirmar"
```

Quem decide o que é válido é o **domínio** (ex.: `Keybinds::detect_conflicts`);
relatório, sessão e id de campo são tipos Petunia — a crate não aparece no
product code.

### Lista reordenável (Wave 7)

O adapter é dono do esqueleto da linha (grip + conteúdo) e da nova ordem; o
produto só desenha o conteúdo:

```rust
TOOLBAR_ORDER_DRAG.show(ui, "toolbar_config_order", &mut order, |id| Id::new(id), |ui, id, row| {
    // `row.dragging` colore a linha; `row.index` é a posição neste frame
});
```

A ordem é aplicada no **drop** (`apply_drag_update`), não a cada frame. O grip é
área de aquisição do gesto: o motor só inicia o arrasto com o ponteiro sobre ele
(ou após 250ms) — ver armadilhas medidas em
[`docs/dependencies/ui-ecosystem-lock.md`](../dependencies/ui-ecosystem-lock.md).

### Motion (Wave 7)

`foundation::motion` é o dono único de durações, curvas e do backend. Motion aqui
é **feedback de estado** (abrir/fechar, revelar campo), nunca decoração contínua
(capítulo 36). Com `animation_time == 0` no tema, tudo vira troca instantânea. A
unidade das APIs é **segundos** — use `motion::seconds(..)`, nunca `millis(..)`.

```rust
let alpha = PetuniaMotion::reveal(ui.ctx(), "shelf", visible);
PetuniaMotion::section(ui, "outliner-search", open, |ui| { /* conteúdo */ });
```

Estados medidos hoje (incluindo dívida herdada): `cargo run -p xtask -- ui-guard`.
Baseline congelada em
[`docs/audits/ui-ecosystem-final-push/00-baseline.md`](../audits/ui-ecosystem-final-push/00-baseline.md).

O confinamento de tipos de terceiros já vale hoje e é verificável:
`cargo run -p xtask -- ui-guard --strict` falha se `egui_taffy::`, `egui_tiles::`,
`egui_dnd::`, `egui_animation::`, `egui_form::` ou `twill::` aparecerem fora do
adapter correspondente.

## Conceitos centrais

- **`UiRegions`** (`crates/ui/src/regions.rs`): fonte única dos retângulos do
  shell; cada painel registra o seu; overlays e hit-testing leem daqui.
  Invariantes testados (ex.: laterais nunca invadem a status bar).
- **Render-on-demand**: a UI redesenha sob eventos e mutações, não por frame.
- **Estado**: `AppState` (core, sem backend); UI nunca duplica domínio;
  preferências de sessão em `UiState`, persistentes só quando o app persiste.
- **Design system** (`widgets.rs` + `tokens.rs`): botões, menus e campos
  canônicos com foco visível, `widget_info` e tooltips; zero cores hardcoded.
- **Component Gallery** (`crates/ui/src/gallery.rs`): vitrine executável onde um
  componente reutilizável nasce e é conferido em todos os temas, densidades e
  escalas antes de aparecer num painel.

  ```bash
  cargo run -p petunia_ui --example component_gallery
  ```

  Ordem obrigatória para qualquer componente reutilizável (diretiva §38):
  **procurar componente existente → modificar a gallery → só depois usar no
  produto**. Não se cria visual isolado dentro de um painel. O que ainda não
  existe como componente aparece na própria gallery como linha `pendente`, com o
  contrato que falta — uma ausência visível é informação, uma omissão não é.

  A gallery é exceção declarada em `cargo xtask ui-guard` (ela monta
  micro-layout por definição); não é caminho de produto — nenhum painel do shell
  a importa.
- **Ícones**: `IconRegistry` (iconflow Lucide/Iconoir + Phosphor de fallback +
  arte vetorial própria); setas e símbolos via pintura vetorial, nunca glifo
  de fonte frágil.
- **i18n**: chaves `TextId` + TOML por idioma, paridade en/pt-BR em CI.

Mapa completo: [Mapa de Componentes UI](./ui-component-map.md).
