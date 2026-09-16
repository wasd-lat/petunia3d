# 01 — Análise código × documentação: discrepâncias encontradas

<aside>
🔍

Auditoria feita lendo o código executável e comparando com o Livro Vivo
(`docs/bible/`) e com a *Egui Ecosystem Final Push Directive*. Cada achado traz o
**contrato**, a **evidência no código** e o **estado** neste momento.

Estados: `CORRIGIDO` · `ABERTO` · `ACEITO` (divergência consciente) · `PENDENTE`
(depende de wave posterior).

</aside>

# A. Divergências corrigidas

## A1 · Workspace fora da baseline congelada

- **Contrato:** capítulo 36 — workspaces de V1 são `MODEL / PAINT / UV`. Cap. 23:
  "`Animate` pode surgir futuramente quando esse módulo existir."
- **Evidência:** `Workspace::Animate` no enum, na barra de pílulas, no toolbar, no
  contextual shelf e na memória de layout.
- **Risco:** a UI prometia uma experiência que o escopo aprovado não cobre.
- **Correção:** `Workspace::Animate` compilado só com a feature
  `animation-workspace` (off por padrão); pílulas derivadas de `Workspace::all()`;
  `Workspace::COUNT` dimensiona a memória de UI. O módulo de animação continua
  compilável e testado. `CORRIGIDO`

## A2 · Temas oficiais versus temas embutidos

- **Contrato:** capítulo 36 — `petunia-dark` é o tema completo oficial; High
  Contrast é a variação oficial de acessibilidade; Light "pode surgir depois".
  P3D-085 — packs externos são declaração do usuário, nunca código embutido.
- **Evidência:** quatro temas embutidos em `register_builtin_themes` (dark, light,
  capuccino, tokyo-nights) e quatro pastas equivalentes em `assets/themes/`.
- **Risco:** o produto embarcava três temas não oficiais como se fossem contrato,
  e o catálogo gerado de tokens documentava isso como verdade.
- **Correção:** embutidos reduzidos a `petunia-dark` + `petunia-high-contrast`
  (com pack em disco espelhando os mesmos valores); `OFFICIAL_V1_THEME_IDS` como
  fonte única; light/capuccino/tokyo-nights movidos para `assets/theme-examples/`
  como packs declarativos; `assets/themes/dark.toml` (esquema antigo, nunca
  carregado) removido; catálogo de tokens gerado apenas com os dois oficiais.
  `CORRIGIDO`

## A3 · Seletor de tema com IDs quebrados

- **Contrato:** troca de tema é runtime e o pack precisa existir.
- **Evidência (bug real):** o menu usava literalmente `capuccino` e
  `tokyo-nights`, enquanto o registro publica `petunia-capuccino` e
  `petunia-tokyo-nights`. Escolher esses itens caía silenciosamente no fallback
  `petunia-dark`.
- **Correção:** o menu passou a ser derivado de `ThemeRegistry::available()`;
  qualquer pack novo aparece sem editar a UI. `CORRIGIDO`

## A4 · Largura da pílula PAINT dependia do idioma

- **Contrato:** capítulos 23 e 36 — as pílulas são canônicas e não traduzidas.
- **Evidência:** `pt-BR` traduzia `[ws].paint` como `"PINTURA"`, produzindo uma
  barra de largura diferente da referência visual.
- **Correção:** `MODEL / PAINT / UV` iguais nos dois locales. `CORRIGIDO`

## A5 · Vocabulário de usuário e strings hardcoded

- **Contrato:** cap. 13 e `AGENTS.md` §3 — vocabulário de usuário canônico
  (`Point`, `Round Edge`) e zero string hardcoded em UI pública.
- **Evidência:** `snap.rs` imprimia `"Vertex"`; o comando
  `select.domain_vertex` tinha rótulo `"Select Domain: Vertex"`; o menu de contexto
  (`nav_gizmo.rs`) era um bloco de ~320 linhas **em português hardcoded** com
  `"Vértice (Edit)"`, `"Aresta (Edit)"`, `"Extrude Vértice"`, `"Fechar Menu"`.
- **Correção:** rótulos migrados para `TextId` (`assets/locales/{en,pt-BR}.toml`),
  dominio exibido como `Point`, menu de contexto reescrito derivando o conteúdo de
  `SelectionDomain` (P3D-015) e lendo rótulos do i18n. `CORRIGIDO`

## A6 · `EditMode` como segundo dono do estado de interação

- **Contrato:** P3D-015 — o estado semântico é
  `SelectionDomain::{Object, Vertex, Edge, Face}`; pode existir estado interno de
  edição, mas ele "não deve dominar a UX nem duplicar comportamento".
- **Evidência:** `EditorSession.mode: EditMode` era campo independente com
  `TexturePaint` misturando *paint* com edição de componentes; três pontos da UI
  escreviam nele para "entrar em Edit Mode".
- **Correção:** o campo foi removido. `edit_mode()` é **derivado** de domínio +
  workspace, e `set_edit_mode()` traduz a intenção para `set_selection_domain()` ou
  `switch_workspace()`. Não há mais estado paralelo a manter sincronizado.
  `CORRIGIDO`

## A7 · Duas fontes para o mesmo keymap

- **Contrato:** cap. 00 (fonte única) e P3D-090.
- **Evidência:** `assets/keybinds/petunia.toml` duplicava o keymap de
  `assets/keymaps/`, e `Keybinds::load_profile` tinha fallbacks para os dois
  diretórios.
- **Risco adicional:** os READMEs divergiam — `assets/keymaps/README.md`
  documentava uma seção `[bindings]` que o loader não lê, `redo = "Ctrl+Y"`
  (o default é `Ctrl+Shift+Z`) e ações inexistentes.
- **Correção:** diretório legado removido, fallbacks retirados do código,
  `assets/keymaps/README.md` reescrito com a estrutura real (`<namespace>.<ação>`)
  e os oito perfis. `CORRIGIDO`

## A8 · Guardrails de UI inexistentes

- **Contrato:** diretiva §36/§37.
- **Evidência:** nenhuma verificação automatizada de layout manual ou de
  confinamento de bibliotecas de UI.
- **Correção:** `cargo xtask ui-guard` (relatório por regra, 12 regras,
  whitelist de foundation/adapter), `--strict` para confinamento e `--baseline`
  para a tabela canônica; documentado em `AGENTS.md` §0.1 e §4.
  `CORRIGIDO`

## A9 · Component Gallery inexistente

- **Contrato:** diretiva §38 — `cargo run -p petunia_ui --example component_gallery`
  com a regra "procurar componente existente → modificar a gallery → só depois
  usar no produto".
- **Evidência:** não havia vitrine. Os componentes só apareciam dentro dos
  painéis de produto (`widgets.rs` tinha testes de render por widget, mas nenhuma
  superfície única onde o componente é visto em todos os temas, densidades e
  escalas).
- **Correção:** `crates/ui/src/gallery.rs` + `crates/ui/examples/component_gallery.rs`,
  com alternância de tema (`petunia-dark` / `petunia-high-contrast`), os 3 presets
  de densidade, as 5 escalas da §39 e os dois idiomas.

  A galeria **não esconde lacuna**: o que a §38 exige e ainda não tem componente
  reutilizável aparece como linha `pendente · <nome>` com o contrato que falta
  (constante `PENDING`). O inventário de lacunas também é verificado por teste —
  uma linha sem contrato explicado falha em `pending_inventory_entries_are_documented`.

  Exceção declarada no `ui-guard` (`FOUNDATION_PATHS`): a vitrine monta
  micro-layout por definição; o product code continua proibido. `CORRIGIDO`

## A10 · A grade do adapter Taffy nunca dividia a linha

- **Contrato:** diretiva §23/§45 — arranjo responsivo é resolvido pelo adapter
  (`PetuniaTaffyLayout`), não por fórmula no painel.
- **Evidência (bug real, encontrado na Wave 3):** em
  `adapters/taffy_layout.rs`, `responsive` aplicava o estilo de coluna com
  `let _ = tui.style(..)` **soltos** e com
  `flex_basis: LengthPercentage::percent(1.0 / columns)`. Duas falhas somadas:
  1. `percent()` é resolvido contra um pai de tamanho automático na passagem de
     medida e volta **zero**;
  2. `tui.style(..)` só vale quando **encadeado** com o `add` — solto, é
descartado.
  Resultado: `PetuniaLayoutMode::Grid { columns }` era uma linha comum; cada item
  ficava com a largura do próprio conteúdo (medido: três itens de 40px começando
  em `0/8/16` numa linha de 800px).
- **Risco:** a Wave 3 e todas as waves seguintes iriam apoiar layout responsivo
  numa primitiva que não fazia o que o nome dizia.
- **Correção:** `column_width` (pura, truncada para baixo) + `fit_columns`
  (o número de colunas vem do mínimo do item, não de um breakpoint) +
  `columns` / `PetuniaColumnSpec`, que aplica **comprimento definido** encadeado no
  `add` e entrega a largura resolvida a cada item. `responsive` em modo `Grid`
  delega para `columns`. Cobertura: 8 testes novos no adapter, incluindo um que
  falha com a implementação antiga (`grid_mode_now_really_splits_the_row`).
  Registrado também em `docs/dependencies/ui-ecosystem-lock.md`. `CORRIGIDO`

## A11 · Breakpoints de layout dentro do inspector (Wave 3 · §45)

- **Contrato:** §45 — a Wave 3 migra property rows, campos vetoriais, abas,
  "section header sizing" e controles responsivos, eliminando as fórmulas
  manuais observadas na auditoria.
- **Evidência:**
  | Onde | Fórmula |
  | --- | --- |
  | `inspector_widgets.rs` `Vector3Field` | `avail >= 300.0` / `avail >= 190.0` e `(avail - 3*label - 2*gap)/3` |
  | `inspector_widgets.rs` `context_tabs` | `tab_w = ((avail - gap*(n-1))/n).floor()` |
  | `properties_panel.rs` barra do objeto | `(ui.available_width() - 108.0).floor().max(60.0)` |
  | `properties_panel.rs` modificadores | `narrow_actions = ui.available_width() < 220.0` |
  | `properties_panel.rs` combos de grupo | `ui.available_width().clamp(96.0, 180.0)` (×4) |
- **Correção:**
  - `Vector3Field` descreve o arranjo com `PetuniaColumnSpec::responsive(3,
    AXIS_UNIT_MIN_W, AXIS_GAP)`; o mínimo da unidade de eixo é derivado do
    conteúdo (rótulo + vão + campo utilizável) e recebe a largura pronta.
  - `context_tabs` usa `PetuniaColumnSpec::fixed(n, TAB_GAP)` — sem cálculo de
    `tab_w`.
  - A faixa de ações da barra do objeto virou
    `fill_remaining(ui, object_bar_actions_width(), OBJECT_BAR_NAME_MIN_W)`, com a
    largura derivada do tamanho real dos botões (não o `108.0` mágico).
  - Os dois botões de modificador usam `PetuniaResponsiveLayout::wrap_row()`: o
    wrap do layout decide se cabem lado a lado. O breakpoint de 220px sumiu e um
    rótulo traduzido longo deixou de estourar.
  - Os quatro combos usam `clamped_width(ui, 96.0, 180.0)`.
  - "Section header sizing": `section()` e `block_header()` passaram a usar papéis
    de `foundation::typography` (`SectionTitle`, `Caption`) e
    `foundation::spacing`, em vez de `FontId::proportional(11.5)` e de
    `ui.add_space(2.0)` soltos.
- **Delta visual deliberado (para o passe de screenshot):** o título de seção cai
  de 11.5 para 11.0 px e o título de bloco de 12.5 para 11.0 px — agora existe
  **um** papel para cabeçalho em vez de dois literais. A barra do objeto deixa
  12px de folga que existiam só por causa do `108.0` estimado.
- **Preservado:** comportamento e undo. `Vector3Field` mantém a sessão de undo
  única por gesto nos três eixos; a suíte de UI (`kittest_ui_flows`,
  `panel_stability`, `sidebar_redesign_flows`) passa sem alteração de asserção.
  `CORRIGIDO`

## A12 · A barra da viewport fazia engenharia de layout à mão (Wave 4 · §46)

- **Contrato:** §7 e §46 — `PetuniaResponsiveToolbar` com clusters, prioridade,
  intent de conteúdo mínimo, política de wrap e de overflow. Taffy cuida de gap,
  linha, alinhamento e distribuição; o produto decide **quais** clusters podem
  cair, porque isso é semântica de produto.
- **Evidência:**
  | Onde | Fórmula |
  | --- | --- |
  | `viewport_bar.rs` `measured_widths` | `28.0 + 24.0*3.0 + 3.0*2.0`; `8.0 + text_w(..) + 4.0 + 16.0`; `14.0 + 3.0 + 62.0 + 2.0 + …`; `24.0 + 22.0 + …`; margem de segurança `+ 8.0` |
  | `viewport_bar.rs` overflow | `if used + width_of(cluster) + 16.0 > avail_w { hidden.push(..) }` |
  | `viewport_bar.rs` linhas | `if ui.available_height() > 40.0 { /* duas linhas */ }` |
- **Correção:** a barra declara as cinco faixas (`pinned` / `overflowable` com
  rank) e o adapter mede cada uma com uma sonda offscreen, decide com
  `plan_row` (função pura) e desenha a linha no taffy. `measured_widths`, `text_w`
  e o enum `BarCluster` foram deletados; a altura da linha é **medida** e a
  decisão de uma/duas linhas usa `rows_that_fit`. A faixa de acesso só reserva
  espaço quando realmente há algo oculto (antes reservava 30px sempre).
- **Delta visual deliberado (para o passe de screenshot):** seta de overflow no
  fim da linha nos dois modos; espaçamento entre faixas vindo do gap do layout +
  divisor medido; barra de 41–46px fica em uma linha em vez de espremer duas.
- **Preservado:** comportamento e undo. Nenhuma asserção de teste existente
  mudou; a suíte de UI passa. `CORRIGIDO`

## A13 · Item de taffy herdava o layout vertical do painel (Wave 4 · bug real)

- **Contrato:** §17.2 — layout de linha é responsabilidade do adapter.
- **Evidência (bug real, encontrado na Wave 4):** `egui_taffy` entrega a cada item
  um `Ui` que **herda o layout do pai**. Dentro de um painel vertical, uma faixa
  que desenha vários filhos direto no `Ui` recebido empilhava: medido no produto,
  `viewport.menus` reportava 97px de altura e `viewport.display` 101px — a linha
  da barra ficava 101px alta em vez de 24px.
- **Risco:** qualquer linha de taffy usada dentro de um painel vertical mediria e
  desenharia errado — inclusive as waves seguintes (top bar, toolbar lateral).
- **Correção:** `PetuniaItemLayout::{Inherit, Row}` +
  `PetuniaResponsiveLayout::with_item_layout`; a barra declara que cada item é uma
  linha. Cobertura: `second_row_appears_only_when_the_measured_line_height_fits`
  (mede a altura desenhada) e a sonda do adapter. Registrado em
  `docs/dependencies/ui-ecosystem-lock.md` como armadilha conhecida. `CORRIGIDO`

## A14 · Top Bar centralizava no terço, não na barra (Wave 5 · bug real)

- **Contrato:** §25 — "CENTER permanece geometricamente centralizado quando
  possível" e "não usar `add_space` repetido para fingir centralização".
- **Evidência:** `ui.columns(3, …)` + `horizontal_centered` põe as pílulas no
  meio do **terço do meio**. Isso só coincide com o meio da barra quando as duas
  laterais têm largura igual; com idioma longo de um lado (ou `Assets`+`Config`
  do outro) as pílulas deslizam — e nenhum teste media o centro.
- **Risco:** o seletor de workspace (navegação primária) muda de lugar conforme o
  idioma; pílula ativa parece pular entre frames ao redimensionar janela.
- **Correção:** `adapters::top_bar` (`plan_top_bar` + `three_zone`). As laterais
  recebem caixas de largura **igual** (base 0 + `flex_grow` igual), então o centro
  é geométrico por construção; quando a esquerda não cabe na metade, o plano
  assume o fallback `Drifting` **por medição** e as ações de menor rank vão para a
  seta. Cobertura: 7 testes do adapter, incluindo
  `center_is_geometric_even_when_sides_differ_wildly` e
  `narrow_bar_falls_back_to_drifting_without_losing_access`. `CORRIGIDO`

## A15 · `plan_row` contradizia a própria regra de prioridade (Wave 5 · bug real)

- **Contrato:** §46 — "rank maior = mais protegido (cai depois)"; o doc do
  adapter dizia "a queda acontece em ordem de `rank`".
- **Evidência:** o algoritmo era guloso item a item na ordem de rank: uma faixa
  de rank **alto** que não cabia era ocultada e, em seguida, uma de rank **baixo**
  que cabia era mantida. Com larguras iguais o resultado coincidia com a regra;
  com larguras diferentes (idioma longo, ícones de tamanhos distintos) não.
- **Risco:** a barra escondia o controle mais importante e mostrava o menos
  importante — exatamente o oposto da política declarada.
- **Correção:** queda iterativa ("derruba a menos protegida até caber"), seta de
  acesso só desenhada quando existe algo oculto, e um invariante em teste:
  `a_more_protected_cluster_never_falls_before_a_less_protected_one`
  (`rank` de todo visível ≥ `rank` de todo oculto, em 6 larguras). Efeito
  colateral positivo: em vários casos a barra agora esconde **menos**. `CORRIGIDO`

## A16 · `egui_tiles`: shares normalizadas pela soma (Wave 5 · armadilha medida)

- **Contrato:** §31.3 — o DTO Petunia é o contrato; o adapter traduz para a árvore.
- **Evidência (medida em teste com frame real):** `Shares::split` divide a largura
  disponível **proporcionalmente à soma das shares**. Um filho sem share explícito
  fica com o default (`1.0`) e domina o container: pedindo dock de 300px em
  1280px, o dock media **231.8px**.
- **Correção:** o adapter escreve o share das **três** colunas (`pixels / span`),
  e o teste `real_frame_reports_pixel_widths_close_to_the_dto` mede as larguras
  resultantes do frame real. Registrado em `docs/dependencies/ui-ecosystem-lock.md`
  junto com a segunda medida da wave (um `Panel` do egui 0.36 funciona **dentro**
  do `Ui` de um tile — o que torna a migração do shell incremental). `CORRIGIDO`

## A17 · A paleta escolhia o próprio arranjo com dois breakpoints (Wave 6 · §48)

- **Contrato:** §48 — "remover breakpoints manuais; usar Taffy Grid/Flex".
- **Evidência:** `draw_palette` decidia o modo compacto com
  `available_width() < 90.0` e o par de colunas com `>= 100.0`; o widget do botão
  media o container (`ui.available_width().max(TOOLBAR_WIDTH)`) para se
  dimensionar; o par de colunas era montado com `group.chunks(2)` + `ui.horizontal`.
- **Risco:** três números escolhidos a dedo (90, 100, e a largura do container)
  decidiam o que o usuário vê — sem relação com o tamanho real do botão, e
  sensíveis a escala de UI e a idioma com rótulo longo.
- **Correção:** `adapters::tool_grid` (colunas derivadas do mínimo da célula via
  `fit_columns`, célula resolvida entregue ao item, `labeled` no lugar do modo
  compacto) + `PetuniaToolbarButton::width` (o widget recebe a largura, não a
  mede). `CORRIGIDO`

## A18 · O menu da família era desenhado fora do paine (Wave 6 · bug real)

- **Contrato:** todo controle visível precisa estar dentro da coluna que o
  hospeda (§31, viewport-first) e alcançável pelo mouse.
- **Evidência (medida):** o grupo desenhava botão compacto (40px) **mais** a seta
  (21.4px medidos), sem reservar a faixa da seta: com a coluna em 40/48/64px o
  conteúdo ocupava **74px** — a seta era recortada pelo paine e o menu da família
  (Seleção / Transformação) só existia por atalho. A coluna nascia em 48px
  (política herdada do `Panel::left`), que comporta o ícone e não o par.
- **Correção:** `plan_group(cell_width)` decide o arranjo (botão + seta quando
  cabe; só o botão, com o menu pelo clique secundário e dica no tooltip, quando
  não cabe) e o mínimo da coluna passou a ser **derivado** do que ela hospeda
  (`TOOLBAR_MIN_WIDTH` = ícone + vão + seta + moldura = 74px), com o padrão
  espelhando o mínimo. Regressão coberta por `crates/ui/tests/toolbar_fit.rs`.
  Delta deliberado: coluna 48 → 74px. `CORRIGIDO`

## A19 · Reference Manager fazia a matemática da grade no produto (Wave 6 · §48)

- **Contrato:** §37/§48 — quem produz layout é o adapter; o componente recebe a
  largura resolvida.
- **Evidência:** `grid_columns` e `grid_card_width` calculavam colunas e largura
  do cartão no arquivo de produto, com um laço de `chunks` montando as linhas à
  mão; o cabeçalho escolhia entre empilhado e lado a lado com o breakpoint
  `available_width() < 560.0`.
- **Correção:** grade por `taffy_layout::columns` sobre
  `PetuniaColumnSpec::responsive` (o cartão recebe `card_w` do adapter) e
  cabeçalho por esteira flex `wrap_row` + `SpaceBetween` com uma ação por item
  (`draw_global_actions`, que preserva o símbolo documentado no mapa de UI).
  As duas fórmulas e o breakpoint morreram; os testes passaram a verificar o
  contrato declarado contra `fit_columns`. `CORRIGIDO`

# B. Divergências documentais corrigidas

| # | Documento | Contradição | Correção |
| --- | --- | --- | --- |
| B1 | `docs/developers/ui-architecture.md` | "UI sem `egui_tiles`" | regra de layout normativa + matriz §17.2 + proibições §36 |
| B2 | `docs/developers/ui-component-map.md` | "built-in primeiro", "`egui_tiles` removido", "`egui_taffy` quando um grid exigir" | bloco de avaliação substituído, com tabela de classificação vigente |
| B3 | Livro Vivo cap. 35 | `egui-lucide` como baseline de ícones; DnD/Motion/Table como opcionais | `iconflow` + pack Petunia; DnD/Motion baseline; tabelas/forms/async como P1 |
| B4 | Livro Vivo cap. 36 | "`egui_tiles` é BASELINE" sem ressalva, quando a crate **não é dependência** | status de implementação explícito + ponteiro para a Wave 2 |
| B5 | `docs/modernization/06` e `08` | "Taffy somente piloto", "não substitui tiles sem ADR" | contrato revisado: Tiles = macro-layout, Taffy = layout responsivo; não são alternativas |
| B6 | `docs/audits/stack-modernization/40` | "Phase E 100% COMPLETE" lido como adoção de produto | nota histórica: integração de **dependência** completa; adoção em product paths era piloto |
| B7 | `PETUNIA3D_STACK_MODERNIZATION_P0_P3_GAUNTLET.md` | estado auditado de 2026-09-14 lido como descrição atual | seção "Reconciliação de estado" com números reais e inventário de gates |
| B8 | `docs/manual/interface.md` | não explicava como o layout é calculado | seção curta sobre motor de macro-layout e determinismo do arranjo |

# C. Achados metodológicos que valem registro

1. **`egui_tiles` não é dependência.** Nenhum `Cargo.toml` a declara; existe apenas
   como comentário em `crates/ui/src/flex_layout.rs` e como decisão nos capítulos
   35/36. Documentação descrevia como baseline algo que não estava instalado.
2. **Taffy e Twill são pilotos de um arquivo cada** (`flex_layout.rs`,
   `twill_bridge.rs`), enquanto 349 ocorrências de layout manual viviam no product
   code. A dependência estava "integrada"; a arquitetura de produto não.
3. **O confinamento, por outro lado, estava correto.** `ui-guard --strict` passa:
   nenhum tipo de biblioteca auxiliar escapou para product code. A fronteira P0/P1
   do `arch-check` funcionou — o que faltava era uso, não disciplina.
4. **Falhas de teste pré-existentes encontradas e corrigidas:** o teste de largura
   do dock (`regions.rs`) esperava 247.8 px quando o código trunca para inteiro de
   propósito; o teste de catálogo de temas esperava "Petunia Light".

# D. Pendências que NÃO foram resolvidas aqui

| Pendência | Por que | Onde |
| --- | --- | --- |
| Captura real de screenshots | decisão explícita: capturar depois de todas as correções | diretiva §40, §42 |
| `PetuniaTable`, `PetuniaVirtualCollection`, `PetuniaAsyncView` | dependem das waves 8–9 | diretiva §50–§51 |
| Reordenar o Modifier Stack por arrasto | o contrato existe (`PetuniaDragList`); falta trocar as setas da lista de modificadores — primeira expansão da §49 | diretiva §49, §52 |
| Componentes listados em `PENDING` (context menu, toggle/switch, badge, breadcrumb, table, virtual list, suspense, drag handle do Modifier Stack) | dependem de extração de painel ou de crate não instalada | `crates/ui/src/gallery.rs` · diretiva §38 |
| Adoção de Tables/Virtual List/Suspense em product paths | Waves 8–9 | diretiva §50–§51 |
| Captura real de screenshots do inspector migrado | depende do passe de screenshot | diretiva §40, §42 |
| Breakpoints remanescentes fora do inspector (`primitive_card`, `status_bar`, `animation_ui`, timeline) | pertencem às waves 8–10; `toolbar`, `reference_manager` (wave 6) e as buscas do Outliner/Inspector (wave 7) já saíram | `cargo run -p xtask -- ui-guard` · diretiva §47–§50 |
| `docs/public/ui-map.json` desatualizado (7 nós: `toolbar::draw`, `right_panel`, `draw_split_dock`, `animate_workspace_center`, `scene_panel_height`) | o arquivo está no **site público congelado** (AGENTS.md §1); corrigir exige `bible-lock` e decisão explícita de descongelar. O `docs-check` fica vermelho só neste passo | `cargo run -p xtask -- docs-check` · Wave 5b/6 |
| Largura mínima da barra da viewport | o núcleo (Domain/Menus/Display) é largo por natureza: abaixo de ~660px a linha passa da largura em vez de ocultar núcleo | diretiva §47/§59 |

> `ui-guard` na CI: **feito** — `.github/workflows/ui-guard.yml` roda
> `ui-guard --strict` e compila a vitrine.
>
> Adoção de `egui_dnd`/`egui_animation`/`egui_form` em product path: **wave 7** —
> reordenar a paleta por arrasto (`adapters::drag_drop`), revelação animada do
> campo de busca (Outliner/Inspector, via `foundation::motion`) e validação de
> conflitos de keymap (`PetuniaFormSession`). Três armadilhas medidas ficaram no
> `docs/dependencies/ui-ecosystem-lock.md` (limiar do handle, `to` exclusivo,
> tempo em segundos).
>
> Adoção de Taffy em product path: **wave 3** (inspector), **wave 4** (barra da
> viewport), **wave 5** (Top Bar) e **wave 6** (paleta lateral, Settings,
> Reference Manager). Verificável — `cargo run -p xtask -- ui-guard` lista
> `available_width` de produto em **14** (era 18 ao fim da wave 5, 45 na
> baseline), sem `toolbar.rs` nem `reference_manager.rs` entre os arquivos com
> ocorrência, e `spacing_mut` de produto em **37** (era 48 na wave 5, 54 na
> baseline); as ocorrências restantes são desenho de controle de baixo nível,
> não layout.
>
> Adoção de `egui_tiles` em product path: **wave 5b** — o shell descreve o
> macro-layout em `PetuniaShellLayout` e a árvore do adapter resolve geometria,
> divisórias, colapsos e lado/orientação do dock (`shell.rs` substituiu
> `right_panel`, `draw_split_dock`, `split_widths` e `scene_panel_height`).
> Nenhuma segunda engine de docking existe no grafo: `egui_dock` não é
> dependência.
