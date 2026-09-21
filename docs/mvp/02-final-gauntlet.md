# Petunia3D — Final MVP Gauntlet

> **Zero-excuse release candidate loop. 10/10 ou não está pronto.**
> Adendo obrigatório ao Master MVP Implementation Contract. Não substitui os
> requisitos funcionais anteriores; adiciona a camada
> `AUDITORIA → IMPLEMENTAÇÃO → TESTE → INSPEÇÃO → CRÍTICA → CORREÇÃO →
> REGRESSÃO → REAUDITORIA` em ciclos sucessivos.

## 0. Autoridade e revogação

A restrição anterior que impedia a execução de testes está **REVOGADA para esta
rodada final**. Está expressamente autorizado e obrigatório: compilar; executar;
testar; instrumentar; criar testes; executar testes existentes e novos; smoke
tests; integration tests; regression tests; inspeção visual; screenshots;
comparação de estados; testes de interação real; testes de persistência;
performance; linters; static analysis; fuzz/property testing quando apropriado;
auditoria de dependências, segurança, acessibilidade, arquitetura, Clean Code,
modularidade e UI/UX.

Não é mais aceitável evitar testes alegando custo, tempo ou tamanho da tarefa.

## 1. Missão

Levar o MVP a **Release Candidate real**. O objetivo não é "fazer os testes
passarem"; é entregar software que funcione. A validação considera
simultaneamente: funcionalidade; usabilidade; UI; UX; design visual;
consistência; discoverability; feedback; acessibilidade; performance;
estabilidade; segurança; arquitetura; desacoplamento; modularidade; Clean Code;
legibilidade; manutenibilidade; testabilidade; persistência; integração entre
módulos; tratamento de erros; consistência de estado; Undo/Redo; input;
rendering; i18n; temas; DPI; comportamento responsivo.

Nenhuma dessas dimensões pode ser escondida por uma média.

## 2. Regra absoluta de 10/10

O trabalho não termina com `9.3/10 overall`, nem com "todas as coisas importantes
funcionam", nem com "restam apenas pequenos problemas", nem com "UI pode ser
refinada posteriormente", nem com "é apenas um edge case", nem com "isso é
cosmético", nem com "é um botão pouco usado".

Cada categoria obrigatória precisa atingir `10/10` — não média 10:

```
FUNCIONALIDADE 10/10 · MODEL 10/10 · PAINT 10/10 · UV 10/10 · UI 10/10
UX 10/10 · VISUAL DESIGN 10/10 · INTERACTION 10/10 · ACCESSIBILITY 10/10
ARCHITECTURE 10/10 · CODE QUALITY 10/10 · SECURITY 10/10 · PERFORMANCE 10/10
PERSISTENCE 10/10 · UNDO/REDO 10/10 · I18N 10/10 · THEMING 10/10
ERROR HANDLING 10/10 · TEST COVERAGE/VALIDATION 10/10 · REGRESSION 10/10
```

Se qualquer categoria estiver em `9.9/10`, o Gauntlet continua.

## 3. 10/10 não é autoavaliação subjetiva

É proibido atribuir `10/10` por impressão. Cada `10/10` exige evidência:
requisitos avaliados; artefatos inspecionados; testes executados; resultados;
bugs encontrados; bugs corrigidos; regressões verificadas; evidências finais.

Não escrever `UI/UX: 10/10`. Escrever algo equivalente a: `UI/UX: 47/47
acceptance checks passed. 0 Critical. 0 High. 0 Medium. 0 known interaction
defects. All panel/window/input flows manually verified. Evidence: ...` e só
então `STATUS: PASS / 10/10`.

## 4. Não existe "irrelevante"

Se está visível no MVP, precisa funcionar. Isso inclui: ícone pequeno; menu;
submenu; botão; botão secundário; botão disabled; toggle; checkbox; slider;
número; unit field; dropdown; menu arrow; chevron; splitter; scrollbar; tooltip;
label interativo; context menu; modal; popover; palette; swatch; layer action;
Outliner control; eye icon; lock icon; disclosure arrow; tab; workspace
selector; toolbar; tool rail; asset card; drag target; empty-state CTA; toast
action; shortcut; status-bar control; window control; file picker trigger; color
picker; numeric scrub; gizmo handle; UV vertex; UV edge; UV island; paint
cursor. Se pode receber input, entra no inventário e no Gauntlet.

## 5. Inventário exaustivo

Antes de considerar o software validado, criar um **inventário completo de
interação**. Percorrer todos os arquivos `.slint`, controllers, commands, menus,
registries, tool definitions e components. Listar cada elemento interativo com
identificadores estáveis, ex.:

```
APP.FILE.NEW · APP.FILE.OPEN · APP.FILE.SAVE
APP.WORKSPACE.MODEL
MODEL.TOOL.EXTRUDE
MODEL.OUTLINER.VISIBILITY
PAINT.LAYER.OPACITY
UV.TOOL.UNWRAP
DIALOG.UNSAVED.SAVE
PREFERENCES.THEME.SELECT
```

Não agrupar dez controles diferentes em "toolbar works". Cada controle precisa
poder ser rastreado. Ver `03-interaction-manifest.md`.

## 6. Interaction Manifest

Cada item registra: `ID`, `Workspace`, `Location`, `Element Type`, `Label
Token`, `Tooltip Token`, `Icon ID`, `Command ID`, `Shortcut`, `Enabled
Condition`, `Disabled Reason`, `Input`, `Expected Behavior`, `State Mutation`,
`Visual Feedback`, `Undo Requirement`, `Persistence Requirement`,
`Accessibility Label`, `Automated Test`, `Manual Test`, `Status`, `Evidence`.

## 7. Coverage rule

100 % dos elementos interativos visíveis no MVP devem existir no Interaction
Manifest. Nenhum `UNTRACKED`, `UNKNOWN`, `NOT TESTED` ou `ASSUMED WORKING`.

## 8. Gauntlet loop

```
DISCOVER → AUDIT → IMPLEMENT → BUILD → STATIC ANALYSIS → UNIT TEST
→ PROPERTY/EDGE TEST → INTEGRATION TEST → UI INTERACTION TEST
→ MANUAL USABILITY PASS → VISUAL REVIEW → ACCESSIBILITY REVIEW
→ PERFORMANCE REVIEW → SECURITY REVIEW → ARCHITECTURE REVIEW
→ REGRESSION → SCORE → FIX
```

Repetir até todas as categorias = `10/10`.

## 9. O loop não possui número fixo

Pode ser 3, 5, 8, 12 ou 20 loops. A condição de saída é: todos os hard gates
passaram.

## 10. Loop N+1 precisa revalidar loop N

Correção exige: local verification + affected subsystem regression +
cross-workspace regression quando relevante. Ex.: corrigiu Outliner selection →
testar Outliner selection, viewport selection, Properties, MODEL, PAINT target,
UV synchronization, Delete, Duplicate, Save/Load, Undo/Redo.

## 11. Não otimizar para testes

Proibido: hardcode específico para fazer teste passar; alterar expectativa para
aceitar bug; remover teste legítimo; ignorar teste falhando; marcar `#[ignore]`
sem justificativa excepcional; diminuir coverage de propósito; transformar
assertion em log; capturar erro e retornar sucesso; esconder warning importante;
mockar o próprio código sendo validado em teste de integração. O teste deve
representar comportamento real.

## 12–16. Gates de build, formato, lint, análise estática e placeholders

- **Build**: dev e release `PASS`. Nenhuma feature MVP só em debug. Zero erros.
  Zero warnings novos do nosso código; warnings de dependências externas
  registrados separadamente.
- **Formatting**: executar o formatter oficial detectado no repositório
  (`cargo fmt --check` quando for o workflow real). Descobrir Cargo.toml,
  Makefile, justfile, task runner, CI. Não inventar comando.
- **Lint**: Clippy equivalente com configuração apropriada. Revisar clones
  desnecessários, `unwrap`/`expect` arriscados, complexidade, ownership confuso,
  dead code, unused code, casts, panic path, error swallowing. `#[allow(...)]`
  somente com motivo técnico real documentado.
- **Static analysis**: procurar dead code, unreachable code, duplicate
  implementation, unused command/state/callback, unsafe, panic path, unwrap,
  expect, integer conversion, indexing, unbounded allocation, recursive risk,
  race/state hazards. Cada achado corrigido ou explicitamente justificado.
- **Placeholders**: zero placeholders funcionais no MVP. Comentários `TODO`
  apenas para recursos fora do MVP, explicitamente identificados como pós-MVP.

## 17–20. Arquitetura, direção de dependência, modularidade, god object

- UI não contém domínio; domain não depende do Slint; geometry não depende do
  Inspector; paint não depende do layout; UV algorithms não conhecem widgets;
  renderer não decide UX; preferences não modificam Document sem necessidade.
- Direção esperada: `Presentation → Application → Domain`; renderer adapters →
  contracts; persistence adapters → contracts. Nunca: `Domain → Slint`; geometry
  → app window; UV → toolbar; paint → modal.
- Cada subsistema com responsabilidade clara: model, paint, uv, document,
  selection, commands, history, rendering, assets, materials, persistence, ui,
  preferences, i18n. Evitar módulos gigantes fazendo tudo.
- Procurar structs/classes/módulos responsáveis por UI + document + rendering +
  tools + persistence. Isso é falha arquitetural; separar quando houver
  acoplamento injustificado.

## 21–23. Clean Code, tamanho de função, duplicação

Nomes explícitos; coesão; acoplamento; funções pequenas quando melhora clareza;
fluxo de controle; duplicação; comentários; erros; complexidade. Ruim: `ctx`,
`mgr`, `tmp`, `proc`, `do_it`, `handle2`. Função longa não é automaticamente
errada, mas deve ser revisada para múltiplas responsabilidades, branches
aninhados, duplicação e mutação escondida; dividir apenas quando melhora
clareza, coesão e testabilidade. Duplicação em commands, menus, shortcut
handling, tool activation, fields, cores, spacing, icons, texto, modal behavior
e selection logic: corrigir. Extrude acessado por menu e toolbar deve usar o
MESMO comando.

## 24–30. Erros, panic, unsafe, segurança, arquivos maliciosos, tamanho de arquivo, save safety

- Erros não podem sumir, crashar o app, virar falso sucesso nem ser ignorados.
  Revisar `Result`, `Option`, file I/O, image decode, model import, texture
  allocation, save, load, export, renderer resource creation. Mensagens ao
  usuário claras; logs técnicos úteis.
- Zero panic esperado no caminho normal do usuário. Revisar `unwrap`, `expect` e
  index access em dados de arquivo, do usuário, do renderer e do documento.
- `unsafe`: cada bloco com justificativa, invariantes verificadas, escopo
  mínimo, testado. Nenhum unsafe novo para facilitar UI.
- Segurança desktop: file paths, path traversal, malformed files, imagens
  oversized, OBJ malformado, glTF malformado, project files inválidos, integer
  overflow, allocation bombs, texturas corrompidas, UTF-8 inesperado, symlinks
  quando relevante, arquivos temporários, comportamento de sobrescrita no save.
- Criar inputs inválidos para OBJ, glTF/GLB, project file e image metadata. O
  app deve não crashar, não corromper o documento atual e apresentar erro
  compreensível.
- Não carregar cegamente recursos gigantes sem validação. Quando a arquitetura
  permitir: validar dimensões, limites, resource count. Não impor limites
  arbitrários ridículos.
- Salvar preferencialmente de forma segura: temporary write → validation →
  replace, quando a arquitetura suportar. Testar o failure path.

## 31–35. Persistência, round-trip, Undo/Redo, branching, cancel

- Para toda operação mutável: após `Save + Close + Open` o resultado permanece?
  Se não, falha.
- Round-trip: criar projeto com multiple objects, hierarchy, references,
  materials, textures, paint layers, UV seams, UV coordinates e transforms.
  `Save A → Load → Save B`; comparar estado lógico; diferenças não intencionais
  são FALHA.
- Para cada command mutável: `State A → O → State B → Undo → State A
  equivalent → Redo → State B equivalent`. Validar dados, não apenas aparência.
- History branching: `Operation A → Operation B → Undo B → New Operation C`;
  `Redo B` deve ser invalidado corretamente.
- Toda ferramenta previewable com `Begin → Update → Cancel` retorna a estado
  equivalente ao anterior, sem ghost geometry, dirty state incorreto, history
  entry indevida ou renderer stale.

## 36–39. MODEL, invariantes de geometria, corrupção, cadeia

- Cada ferramenta MVP MODEL validada individualmente em: activation;
  availability; selection compatibility; hover; preview; numeric input; commit;
  cancel; Undo; Redo; renderer; Outliner; Inspector; save; load; invalid input;
  empty selection.
- Após operações topológicas, validar quando aplicável: valid indices; sem
  referências a vértices removidos; face loops válidos; normais finitas;
  posições finitas; sem NaN/Infinity; referências UV coerentes; selection IDs
  válidos; bounds válidos.
- Ferramenta que deixa mesh internamente inválida é **BLOCKER**, mesmo que o
  viewport pareça correto.
- Não testar tools apenas isoladamente. Cadeia:
  `Cube → Scale → Face Select → Inset → Extrude → Bevel → Loop Cut → Move
  vertices → Flip Normals → Save → Load → Undo/Redo onde aplicável.`

## 40. Draw tool gauntlet

Testar Profile Pen, Polygon Pen, Polyline, Rectangle, Circle, Arc e Draw on
Face nos casos: 0 pontos; 1 ponto; 2 pontos; mínimo válido; close; cancel;
backspace; snap; perfil muito pequeno; perfil maior; self-intersection quando
relevante. O app não deve crashar.

## 41–46. PAINT gauntlet

- Cada ferramenta — Brush, Soft Brush mode, Pixel mode, Eraser, Fill,
  Eyedropper, Line, Rectangle — testada em 3D, Texture Canvas e Split View
  quando relevante.
- Cobertura de superfície: center face; near edge; across UV seam; near texture
  boundary; fast drag; slow drag; small brush; large brush; low opacity; 100 %
  opacity.
- Cursor: aparece; escala; some fora do target; indica hit; não lag perceptível;
  reflete hardness quando aplicável.
- Consistência 3D/2D: `3D → 2D` e `2D → 3D` sem Refresh, sem workspace switch,
  sem save/reload.
- Layers: Create, Rename, Duplicate, Delete, Hide, Show, Opacity, Reorder, Merge
  Down, Flatten; depois Undo cada ação; depois Save/Load.
- Stress: muitos strokes razoáveis; verificar input latency, memory growth, UI
  responsiveness, Undo responsiveness. Detectar regressões gritantes.

## 47–50. UV gauntlet

- Testar Vertex/Edge/Face/Island mode; Move/Rotate/Scale; Mark Seam; Clear Seam;
  Unwrap; Pack; Project From View; Checker; Selection sync.
- Reality check: após Unwrap, confirmar coordenadas UV reais. Wireframe
  desenhado arbitrariamente **não** é UV funcional.
- Pack com `fit` deve produzir resultado coerente no tile esperado; verificar
  NaN, Infinity, collapsed islands, overlap inesperado, UVs degeneradas. Quando
  overlap for inevitável pela geometria, não falsificar o teste.
- `select face 3D → corresponding UV`; `select UV island → mesh highlight`;
  `modify UV → data retained`; `switch workspaces → state retained`.

## 51–52. UI component gauntlet e state matrix

Testar cada componente genérico: Button, IconButton, Dropdown, NumberField,
Slider, Checkbox, Toggle, Popover, Tooltip, Menu, Submenu, Modal, TreeRow,
LayerRow, AssetCard, Splitter, Scrollbar, Tabs, ColorPicker, Swatch.

Para cada componente verificar: Default, Hover, Pressed, Selected, Focus,
Disabled, Error, Dragging. Não basta existir no Slint; interagir.

## 53–58. Revisão visual, screenshots, premium design, spacing, tipografia, iconografia

- Revisão visual em 1366×768, 1600×900, 1920×1080 e 2560×1440 e, quando
  possível, 100/125/150/200 % UI scale. Inspecionar spacing, alignment, icons,
  baseline, text clipping, panel balance, empty space, button geometry,
  separator consistency, visual hierarchy.
- Capturar ou inspecionar cada workspace em estados representativos. Perguntar:
  alinhamentos errados? painéis vazios? texto cortado? ícone deslocado?
  elementos apertados? espaço desperdiçado? controles gigantes? inconsistência
  de radius? contraste ruim?
- **Premium design gate**: "funciona" não é suficiente. Avaliar hierarquia,
  densidade, ritmo, alinhamento, proporção, contraste, tipografia, iconografia,
  feedback, consistência, uso de espaço, discoverability, progressive
  disclosure. Premium **não** significa mais sombras, gradientes ou animações;
  significa refinamento, consistência, previsibilidade, acabamento.
- Spacing: confirmar grid de 4 px quando aplicável; verificar icon ↔ label,
  label ↔ field, field ↔ field, section ↔ section, panel padding, toolbar
  padding, modal padding, menu padding. Valores arbitrários precisam de
  justificativa óptica.
- Tipografia: auditar font size, line height, font weight, baseline, clipping,
  ellipsis, alignment, case. Não usar texto de 9 px para economizar espaço nem
  Semibold/Bold em tudo.
- Iconografia: para cada ícone, verificar semântica compreensível, estilo
  coerente, stroke coerente, tamanho coerente, alinhamento óptico, tooltip, hit
  target, selected state, disabled state. Se não, corrigir.

## 59–67. Tooltips, menus, modais, dropdowns, number fields, sliders, color picker, splitters, scroll

- **Tooltip**: todo icon-only control precisa tooltip. Testar display, delay,
  placement, screen boundaries, shortcut display, localization, no clipping. O
  tooltip não pode cobrir permanentemente o alvo.
- **Menus**: abrir cada menu e submenu; clicar cada item executável; verificar
  disabled states; keyboard navigation; Esc; click outside; submenu overflow.
  Nenhum menu item morto.
- **Modais**: abrir cada modal; testar Tab order, Shift+Tab, Enter, Esc, Cancel,
  Primary Action, invalid input, valid input, window resizing, localization.
  Cada modal, mesmo o menos usado.
- **Dropdowns**: click, keyboard, select, Esc, outside click, overflow top,
  overflow bottom, disabled value, current value. Selection must mutate correct
  state.
- **Number field**: click edit, Ctrl+A, typing, negative, decimal, invalid
  chars, Enter, Esc, ArrowUp, ArrowDown, Shift modifier, scrub, fine scrub,
  coarse scrub, min, max, unit. Sem crash. Sem estado incoerente.
- **Slider**: mouse click, drag, min, max, keyboard quando focado, numeric
  entry, undo grouping quando alterar documento.
- **Color picker**: SV, Hue, Alpha, RGB, HEX, invalid HEX, Recent, Eyedropper,
  Enter, Esc, outside behavior.
- **Splitter**: drag menor, drag maior, min, max, double click reset, DPI,
  restart persistence. Não permitir painel negativo, viewport zero ou UI
  offscreen.
- **Scroll**: painel cheio, wheel, scrollbar drag, keyboard, nested scroll.
  Não deixar conteúdo inacessível.

## 68–71. Outliner, lock, Asset Library, workspace switch

- Outliner: Select; Multi-select; Range select quando suportado; Rename;
  Duplicate; Delete; Visibility; Lock; Expand; Collapse; Hierarchy; Reorder;
  Context Menu; Frame selected; cross-sync com viewport.
- Locked object: não pode ser editado; não pode ser picked por acidente no
  viewport; pode ser unlocked; Save/Load retém o lock state se persistente.
- Asset Library: tabs; search; grid; list; import; select; rename quando
  permitido; delete quando permitido; drag; valid drop; invalid drop;
  large-ish asset list; thumbnail loading.
- Trocar repetidamente `MODEL → PAINT → UV → MODEL → UV → PAINT → MODEL`,
  verificando selection, document, history, materials, textures, UV, camera
  quando esperado, layout e active tool behavior. Nenhum state leak.

## 72–80. Responsividade, acessibilidade, contraste, DPI, i18n, pseudo-locale, temas

- Redimensionar a janela continuamente. Observar panel collisions, toolbar
  clipping, modals, menus, popovers, viewport, status, asset panel. Não testar
  apenas resoluções fixas.
- Accessibility: testar somente teclado para fluxos possíveis; verificar focus
  order, focus visible, labels, tooltip, screen-reader metadata quando a stack
  permitir, contrast, hit targets, ausência de semântica só por cor.
- Contraste: verificar especialmente text muted, disabled controls, accent text,
  danger, selected rows, tooltips, menus, inputs. Não tornar muted text
  ilegível.
- High DPI 150 % e 200 %: verificar icons, text, borders, cursor, gizmo, hit
  targets, tooltips, modals. Nada microscopicamente pequeno nem grotescamente
  grande.
- i18n: testar `en-US`, `pt-BR` e pseudo locale; procurar hardcoded strings,
  truncation, broken alignment, missing token, token mostrado literalmente,
  plural issues.
- **Pseudo-locale blocker**: se o pseudo locale expõe clipping em controles
  essenciais, não é `10/10`. Corrigir layout.
- Temas: Dark, Light, System e High Contrast se implementado; verificar TODOS os
  componentes; não permitir hardcoded dark-only colors.
- Dark review: surface hierarchy, viewport distinction, selection, hover, field
  borders, menu separation, text hierarchy.
- Light review: não washed-out, não low contrast, icons readable, selection
  obvious, canvas distinguishable.

## 81–83. Conflitos de input, hierarquia do Esc, perda de foco

- Testar shortcuts em viewport, UV Editor, Texture Canvas, numeric input, text
  input, modal, menu, popover. Digitar `e` em um name field **não** ativa
  Extrude.
- Escape hierarchy: text edit → Esc cancela edit; popover → Esc fecha; active
  tool → Esc cancela tool; modal → Esc cancela quando permitido. Não ocorrer
  múltiplos efeitos com um Esc.
- Focus loss: Alt-tab e return; janela perde foco durante drag, durante paint e
  durante transform. O app não deve ficar preso em mouse-down state.

## 84–88. Performance, frame time, memória, recursos, projeto grande

- Medir ou observar com instrumentação real: startup, idle, viewport
  interaction, panel resizing, Outliner interaction, paint stroke, UV
  manipulation, save, load. Não inventar números. Registrar hardware/ambiente ao
  publicar números.
- UI hover não deve invalidar mesh geometry. Panel animation não deve
  reconstruir scene. Inspector interaction não deve causar reload de asset.
  Investigar spikes claros.
- Observar memória durante open project, workspace switching, paint strokes,
  undo/redo, texture import e repeated open/close dialogs. Procurar crescimento
  monotônico suspeito.
- Verificar liberação de textures, GPU resources, documents, temporary previews
  e recursos de tools canceladas.
- Projeto maior que o trivial: dezenas de objects, algumas textures, multiple
  materials. A UI deve continuar utilizável.

## 89–94. Renderer, seleção, disponibilidade de command, disabled UX, falha silenciosa, empty states

- Após qualquer mutation, o renderer precisa refletir o document. Nunca exigir
  click extra, workspace change, save ou manual refresh.
- Selection não pode apontar para entity removida. Após Delete, Dissolve,
  Separate, Join, Unwrap e workspace switch, validar.
- Auditar `can_execute` de TODOS os commands: não permitir Extrude com object
  selection, Bridge sem loops, Paint sem texture, Pack sem UV, Join com 1
  object.
- Quando um command está disabled: visual correto; se a razão não for óbvia,
  tooltip explica. Evitar o usuário clicar e "nada acontecer".
- Zero falha silenciosa: se o command foi acionado e falhou, informar via
  toast/status/logs.
- Testar todos os empty states: empty scene; no selection; no assets; no
  material; no texture; no UV; no layer quando possível. CTA precisa funcionar.

## 95–98. First-run, discoverability, consistency, premium UX

- Simular usuário novo sem ler código: consegue criar objeto? encontrar
  ferramentas? entender seleção? alterar objeto? encontrar Paint? criar texture?
  encontrar UV? salvar? Se a interface depende de conhecimento secreto, falha de
  UX.
- Toda feature crítica deve aparecer por toolbar/rail ou menu e ser encontrável
  por Search quando o command system suporta. Nenhuma feature essencial escondida
  só em context menu.
- Mesmo padrão para Delete, Rename, Apply, Cancel, tooltips, dropdown, numeric,
  panel header, section header. Não permitir cada painel inventar UX própria.
- Revisar como designer sênior: esta tela parece final? existe elemento com
  aparência de debug UI? excesso de texto? área vazia inexplicável? affordance
  pouco clara? ação principal perdida? ruído? inconsistência? Corrigir.

## 99–102. Revisões visuais MODEL, PAINT, UV e UI contextual

- MODEL deve parecer um modelador moderno: viewport, selection, gizmo, context,
  tools. Não um CAD corporativo antigo, nem Blender miniaturizado.
- PAINT deve imediatamente parecer uma ferramenta de pintura: brush, color,
  layer, texture, target. Não MODEL com pincel.
- UV deve imediatamente parecer um editor UV: o UV Editor é protagonista;
  islands visíveis; tile claro; selection clara. Não MODEL com dois botões UV.
- Face selected → face-related actions. Edge selected → edge-related actions.
  Paint Brush → brush properties. UV Island → UV transform. Não inundar o
  usuário com opções irrelevantes.

## 103–107. Animação, tokens de design, tokens de texto, tokens de command, documentação

- Microanimations só se melhoram feedback; duração típica 100–180 ms. Não
  animar large panels lentamente, tool operation ou viewport state de forma que
  atrase o usuário. Reduced Motion preference deve ser respeitada se
  implementada.
- Audit de design tokens: procurar valores repetidos hardcoded de color,
  spacing, font, radius, height, icon size e border; transformar em tokens
  quando são parte do design system. Não criar token para cada valor único.
- Audit de text tokens: public strings hardcoded devem usar i18n. Exceções:
  technical internal logs.
- Audit de command tokens: cada command possui label, description e status hint
  quando relevante. Nenhum menu pode possuir string duplicada desconectada.
- Se o comportamento final mudou, atualizar documentação relevante —
  especialmente shortcuts, workspace behavior, tool list, MVP scope e known
  limitations reais. Não deixar docs prometendo feature inexistente.

## 108–113. Pirâmide de testes, property tests, fuzzing, sequências aleatórias, snapshots

- Não depender só de E2E. Combinar unit, property, integration, interaction,
  manual, visual, performance e security.
- Property tests quando útil: `Undo(operation(state)) ≈ state`;
  `Save/Load(state) ≈ state`; `Transform with identity ≈ state`; UV transforms
  preservam coordenadas finitas; operações topológicas nunca produzem
  referências inválidas. Não forçar property testing onde não faz sentido.
- Fuzzing direcionado quando viável: project parsing, OBJ parsing, geometry
  operation sequences, numeric input conversion. Não gastar semanas em
  infraestrutura desnecessária, mas usar quando o custo/benefício for claro.
- Sequências pseudoaleatórias controladas de selection, transform, topology e
  Undo/Redo buscando panic, invalid state, NaN e stale selection. A seed precisa
  ser reproduzível quando falha.
- Snapshot/visual regression quando o harness permitir: criar screenshots de
  estados principais. Não tornar snapshot pixel-perfect frágil ao ponto de
  bloquear toda plataforma; preferir estados importantes.
- Baselines visuais sugeridos: MODEL default; MODEL edit face; MODEL tool
  active; PAINT split; PAINT layer selection; UV split; UV island selection;
  Preferences; File menu; Color picker.

## 114–119. Fluxos reais, error flow, no feature creep, no rewrite escape, no cosmetic escape, no function-only escape

- Executar workflows: `FLOW A — MODEL: New → Cube → Scale → Inset → Extrude →
  Bevel → Save`; `FLOW B — PAINT: Material → Texture → Paint → Layer → Opacity →
  Save`; `FLOW C — UV: Mark Seam → Unwrap → Move → Pack → Checker → Save`;
  `FLOW D — ROUND TRIP: Close → Open → Validate`.
- Error flow deliberado: open missing file, open corrupt project, import invalid
  image, export invalid location, save permission failure quando simulável,
  invalid tool selection. O app continua estável.
- Não usar "vamos implementar mais 12 features" como fuga. MVP scope congelado.
- Não reescrever grandes subsistemas apenas porque o código existente é
  imperfeito. Primeiro entender, medir, corrigir. Rewrite só quando a
  arquitetura atual realmente impede o requisito; justificar.
- Não gastar rodada inteira ajustando shadow/radius/accent enquanto um botão não
  funciona. Prioridade absoluta:
  `correctness → interaction → integration → robustness → usability → polish`.
- Também não aceitar "funciona, logo está pronto". Depois da funcionalidade,
  UI/UX precisa atingir o mesmo gate.

## 120–125. Severity system

Classificar defects: `BLOCKER`, `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `POLISH`.

Release candidate exige zero em todas as severidades no escopo do contrato.
Não esconder defeitos como "polish" para terminar.

- BLOCKER: crash; data loss; save corrupt; core tool missing; paint not
  painting; UV not editing; undo corrupting; workspace losing document.
- CRITICAL: tool producing invalid geometry; layer operation losing data;
  unwrap destroying mesh state; menu exposing destructive wrong command.
- HIGH: common command unavailable; panel unusable at common resolution;
  shortcut conflict blocking typing; important dropdown broken.
- MEDIUM: tooltip wrong; context behavior inconsistent; specific interaction
  unreachable; panel persistence broken.
- LOW/POLISH: minor spacing inconsistency; one icon slightly misaligned; hover
  contrast inconsistent. Ainda assim corrigir antes de `10/10`.

## 126–130. Issue ledger, root cause, regression test, cross-module root cause, CI parity

- Manter durante o Gauntlet: `Issue ID`, `Severity`, `Subsystem`, `Description`,
  `Reproduction`, `Root Cause`, `Fix`, `Test Added`, `Regression Scope`,
  `Status`. Nada deve desaparecer da memória do agente. Ver
  `04-issue-ledger.md`.
- Não aplicar patch superficial repetidamente. Entender a causa. Ex.: selection
  stale após Delete — não apenas `if deleted { selection.clear(); }` se o
  problema real for ausência de lifecycle centralizado; corrigir no nível
  arquitetural correto.
- Todo bug relevante corrigido deve receber, quando tecnicamente apropriado, um
  teste que impediria sua volta.
- Bug em Paint pode vir de Document, Renderer, Undo, Material ou UV. Não assumir
  que o arquivo onde aparece é onde a causa existe.
- Descobrir o workflow real da CI e executar localmente os equivalentes
  possíveis. Não inventar pipeline paralelo sem motivo.

## 131–139. Release build, feature flags, default config, first project, portability, asset paths, logging, debug artifacts, release UX

- Validar em configuração equivalente à distribuição. Problemas que só aparecem
  em release precisam ser capturados.
- Auditar feature flags: o MVP não pode depender de feature que a distribuição
  padrão desativa.
- Testar instalação/configuração limpa. Não depender das preferências do
  desenvolvedor. Em config limpa: abrir app, criar projeto, sem arquivos
  secretos, sem paths hardcoded, sem cache antigo necessário.
- Não hardcode `/home/user/...` ou `C:\Users\developer\...`. Paths vêm de
  runtime/preferences.
- Validar asset paths relativos, absolutos quando suportado, missing e moved.
  Não crashar.
- Logs úteis; não despejar spam por frame; não logar milhares de mouse moves sem
  debug flag; errors com contexto.
- Release não deve exibir debug labels, test buttons, placeholder buttons, FPS
  counters não solicitados, wire debug ou TODO UI.
- Ao abrir, não parecer developer build.

## 140–149. Performance budget, latência, async, falhas, robustez numérica, valores extremos, escala zero, geometria degenerada, erro de tool

- Evitar regressões grosseiras. Não inventar meta numérica universal sem
  hardware de referência, mas registrar medições reais e comparar antes/depois
  quando possível.
- Hover instantâneo; click imediato; tool activation imediato; panel resize
  fluido; paint stroke sem atraso perceptível grave.
- Operações potencialmente demoradas não devem congelar a UI sem necessidade.
  Quando houver processamento: feedback; progress quando justificável; cancel
  quando a operação suportar.
- Mesmo operação pesada falhando: o documento permanece válido. Renderer
  resource failure não corrompe o document; informar erro.
- Rejeitar/normalizar NaN, Infinity, overflow e division by zero em campos e
  operações.
- Testar valores razoavelmente extremos: scale near zero; large scale; negative
  transform quando válido; tiny brush; large brush; UV scale. Não exigir suporte
  a valores absurdos infinitos.
- Transform zero scale: definir comportamento; evitar transform inversions
  causando panic; documentar/clamp apenas quando tecnicamente necessário.
- Ferramentas devem lidar com zero-area faces, duplicate points e invalid loop
  candidates sem crash; pode recusar a operação com mensagem.
- Erro de tool deve informar a causa quando possível. Ex.: `Bridge failed —
  Selected loops are not compatible.` Não `Operation failed.`

## 150–157. Progressive disclosure, modais, consistência de tool properties, icon-only, selected state, hover, pointer target, drag and drop

- Mostrar controles relevantes; esconder avançados quando não necessários; mas
  não esconder ferramentas MVP de forma impossível de descobrir.
- Auditar cada modal: precisa realmente bloquear? Se não, popover/panel/toast.
  Mas não mudar indiscriminadamente se o contrato exige modal.
- Todas as ferramentas paramétricas devem usar componentes comuns. Não permitir
  Extrude number field diferente de Bevel.
- Todo icon-only: tooltip e accessible name.
- Selected não pode depender só de 1 pixel azul quase invisível; precisa ser
  perceptível sem ficar gritante.
- Percorrer todas as superfícies clicáveis; hover inconsistente: corrigir.
- Verificar ícones pequenos: visual 16 px pode ter target 28–36 px; não exigir
  precisão cirúrgica.
- Todos drags: begin threshold; preview; valid target; invalid target; drop;
  cancel; Esc quando aplicável; scroll near boundaries.

## 158–169. Teclado, mouse, discoverability, colisões, contextos aninhados, status bar, preview, performance de preview, atomicidade, paint atomicity, save/workspace switch/close com tool ativa

- Fluxos básicos devem ser possíveis com forte suporte de teclado; não
  necessariamente todo modeling sem mouse, mas menus/forms/dialogs precisam
  funcionar.
- O usuário não pode precisar saber shortcut secreto para operação central.
  Tudo essencial acessível visualmente.
- Menus/tooltips mostram shortcuts.
- Gerar tabela global/contextual de shortcuts; detectar conflitos. Conflitos
  contextuais são aceitáveis quando os contexts são mutuamente exclusivos;
  documentar.
- `MODEL B = Bevel`, `PAINT B = Brush`: aceitável porque o workspace muda o
  contexto. Input field focused: nenhum dos dois.
- Hints do status bar refletem a ferramenta real. Não mostrar `Esc cancel` se a
  ferramenta não suporta cancel.
- Preview precisa representar o resultado do commit; não mostrar preview
  diferente da geometry final.
- Preview não precisa recomputar mais que o necessário, mas
  `correctness > micro-optimization`.
- Command deve comprometer estado coerente; não deixar metade da operação
  aplicada se a segunda etapa falhar.
- Stroke: ou commit completo, ou revert.
- Save com tool ativa: definir comportamento. Recomendado: commit/cancel a
  preview ativa explicitamente antes do save, ou salvar apenas o committed
  document. Nunca salvar transient inconsistent preview acidentalmente.
- Workspace switch com tool ativa: definir consistentemente. Recomendação:
  cancelar preview transiente ou commit apenas quando sem ambiguidade. Não
  deixar tool controller órfão.
- Close com tool ativa: mesma regra de estado transiente.

## 170–179. Multi-selection, active vs selected, ID stability, threading, ownership, large clone, allocation, event storm, UI state ownership, single source of truth

- Auditar ferramentas que suportam e não suportam multi-selection. Não aplicar
  silenciosamente somente ao primeiro objeto.
- Distinguir `selected set` de `active element`. Join, material assignment,
  pivot etc. podem depender do active. A UI precisa indicar quando relevante.
- Undo/Redo/Save/Load não podem depender de ponteiros transitórios de UI. Usar
  IDs estáveis apropriados.
- Se houver trabalho em background: UI state updates sincronizados; evitar race;
  não introduzir threads desnecessárias.
- Evitar clones gigantes apenas para satisfazer o borrow checker, especialmente
  mesh, texture e document. Mas não sacrificar clareza prematuramente.
- Procurar `clone()` em hot paths e classificar `necessary` / `avoidable` /
  `expensive`; corrigir quando justificável.
- Hot paths (paint, viewport, UV pan, mouse move) não devem alocar por frame
  desnecessariamente.
- Mouse move não deve disparar atualização global de todos os panels. Usar
  granular invalidation.
- UI pode possuir hover, focus, panel size, open popover e active local
  presentation. Não deve duplicar canonical mesh, materials, UV ou document
  selection.
- Para cada estado, identificar o owner. Se dois lugares acham que são
  canonical, corrigir.

## 180–187. Dirty test, save failure, recent files, native dialog failure, toast gauntlet, no toast spam, empty project

- Mutating command: dirty true. Save: dirty false. UI-only resize: não dirty.
  Workspace switch: não dirty. Hover: não dirty.
- Se Save falha: continua dirty. Nunca marcar `Saved` antes do sucesso.
- Se recent files for implementado: missing file tratado; não crasha.
- Cancelar file dialog não é erro e não altera state.
- Toast: testar Success, Info, Warning, Error; stacking; timeout; long text;
  close; action button quando existir.
- Não gerar toast por cada brush stroke, cada selection click ou cada tool
  activation.
- Petunia deve ser estável com scene vazia; todos os invalid commands disabled.

## 188–191. Projetos de teste

- One object project: base functionality.
- Multi object project: Outliner, multi-selection, material, join.
- Textureless project: Paint dá empty state útil.
- UV-less project: UV dá caminho útil.

## 192–201. Legibilidade, comentários, magic numbers, API pública, error types, qualidade de testes, nomes, flaky tests, timing, ambiente

- Abrir arquivos alterados e perguntar: outro desenvolvedor entende? Nomes
  explicam intenção? Há comentários justificando "why" e não repetindo "what"?
- Evitar `// increment i` / `i += 1`. Comentários para invariant, tradeoff,
  non-obvious math e external constraint.
- Valores de design: tokens. Valores geométricos: named constants quando
  semanticamente importantes. Não tokenizar literal óbvio local
  desnecessariamente.
- Interfaces públicas mínimas; não expor internals por conveniência.
- Erro transporta contexto útil; não usar `String` para tudo quando typed
  errors melhoram o domínio; não overengineer dezenas de error enums inúteis.
- Teste deve falhar quando o comportamento quebra. Evitar `assert!(true)`,
  snapshot vazio e mock total sem SUT real.
- Nome de teste descreve comportamento:
  `undo_extrude_restores_original_face_topology` melhor que `test_extrude_2`.
- Executar o conjunto crítico repetidamente quando possível. Teste
  intermitente: investigar; não simplesmente rerun até passar.
- Evitar sleeps arbitrários em testes; preferir sinais/conditions. Teste não
  deve depender de developer home, internet ou absolute path específico sem
  necessidade.

## 202–210. Fixtures, compatibilidade, versionamento, determinismo, dependências, vulnerabilidades, supply chain, árvore de release limpa

- Fixtures versionadas, pequenas e representativas; não depender de arquivos
  secretos locais. Adicionar inputs deliberadamente inválidos para parser quando
  útil.
- Não quebrar projeto existente sem migration/compatibility decision. Se o
  formato de projeto mudou: migration, versioning ou erro explícito. Se já
  existir versioning, respeitar; não alterar silenciosamente.
- Quando razoável, save equivalente não deve produzir mudanças arbitrárias
  gigantes (facilita debug e version control).
- Revisar dependencies novas e existentes: necessárias? mantidas? licença
  compatível? duplicadas? pesadas sem necessidade? Não remover dependência
  funcional apenas para pontuar melhor.
- Quando o tooling estiver disponível, executar audit de dependências;
  classificar findings; não afirmar segurança absoluta; corrigir
  vulnerabilidades relevantes quando a atualização não quebra o projeto.
- Não adicionar pacote enorme para resolver algo trivial.
- Remover temporary screenshots não desejados, debug dumps, generated junk,
  unused prototype files e abandoned UI components. Não remover artefatos
  necessários.
- Componente antigo não usado: remover se seguro, evitando manutenção dupla.
- Se UI antiga e nova coexistem: verificar qual é usada; não deixar duas
  implementações funcionais divergentes sem motivo.
- Cross-check `code ↔ tests ↔ docs ↔ menus ↔ shortcuts`; corrigir diferenças.

## 214–219. Matrizes finais

- **MVP feature matrix**: matriz completa com cada recurso descrito no Master
  Contract. Não agrupar "Model tools: pass"; uma linha por ferramenta.
- **Interactive element matrix**: `Interactive elements discovered: N`,
  `Implemented: N`, `Automated tested: X`, `Manual tested: Y`, `Unverified: 0`,
  `Broken: 0`, `Stub: 0`, `Dead: 0`.
- **Tool matrix**: por ferramenta — Activation, Preview, Commit, Cancel, Undo,
  Redo, Save, Load, Visual, Accessibility, Status. Tudo `PASS`.
- **Menu matrix**: cada menu item — reachable, enabled logic, command, shortcut,
  result, test.
- **Modal matrix**: cada modal — open, input, validation, keyboard, cancel,
  submit, error, theme, i18n, DPI.
- **Component matrix**: cada componente — states, keyboard, mouse, theme, i18n,
  DPI, accessibility.

## 220–223. Testes negativos, boundary, transições, interrupções

- Não testar apenas happy path. Ex.: Extrude sem faces; Bridge com loops
  inválidos; Paint sem material; Unwrap sem mesh; Save em target inválido;
  Import de arquivo ruim; Delete com seleção vazia.
- Min/max: panel size; brush size; opacity; segments; texture size; UV values;
  numeric inputs.
- Transições: `tool active → modal opens → cancel modal → tool state still
  coherent`; `menu open → workspace switch → no stuck menu`; `popover open →
  resize → correct placement`.
- Durante operação: Esc; workspace switch; window focus loss; open modal; Undo;
  close project. Definir/validar comportamento seguro.

## 224–229. Loops de revisão

- **Design review loop**: depois de tudo funcionalmente passar, ao menos uma
  rodada exclusivamente visual, sem mudar funcionalidade sem motivo. Observar
  todas as telas; listar defeitos; corrigir; repetir até nenhum problema visual
  relevante.
- **UX review loop**: rodada exclusiva de UX. Executar tarefas sem consultar
  código. Contar clicks, context switches, ferramentas escondidas e onde o
  feedback falta. Simplificar quando puder sem remover capacidade.
- **Code review loop**: rodada exclusiva sobre código alterado; procurar dívida
  criada pela correção rápida; refatorar; rerun tests.
- **Security review loop**: depois de refactors, reauditar input/file
  boundaries.
- **Performance review loop**: com funcionalidades estabilizadas, profile hot
  areas. Não adivinhar.
- **Final regression loop**: somente quando tudo parece pronto — clean build;
  full relevant test suite; launch app; complete MODEL flow; complete PAINT
  flow; complete UV flow; Save; Close; Open; Validate.

## 230. Final fresh-state run

Limpar apenas dados temporários seguros/config test, ou usar profile novo. Abrir
como usuário novo. Reexecutar fluxos principais. Detecta dependência acidental
de state antigo.

## 231–239. No false 10/10, limitações externas, honestidade, evidência, ordem de trabalho, não parar por ansiedade de token, no "good enough", MVP ≠ protótipo, scope freeze

- É proibido declarar `10/10` se qualquer teste falha; qualquer ferramenta do
  contrato está missing; qualquer item está `PARTIAL`; qualquer interactive
  element está untested; qualquer known defect permanece; qualquer
  blocker/high/medium/low conhecido permanece no escopo; qualquer Save/Load flow
  perde dados; qualquer placeholder funcional permanece.
- Impedimento REAL externo (hardware inexistente, OS API indisponível,
  dependency upstream quebrada): não declarar `10/10` naquela dimensão; marcar
  `BLOCKED_EXTERNALLY` com evidence, impact e workaround attempted. Continuar
  todo o restante. Não inventar resultado.
- Se algo não foi testado: dizer `NOT TESTED`. Não "should work", "likely works",
  "appears correct". Isso não vale `10/10`.
- Toda afirmação importante no relatório final deriva de code inspection, test
  result, manual execution, instrumentation ou artifact. Não de inferência
  otimista.
- Não desperdiçar metade do trabalho em relatórios intermediários longos.
  Durante os loops, manter ledger técnico, continuar corrigindo; só gerar
  relatório consolidado após atingir a stop condition ou encontrar blocker
  externo genuíno.
- Não reduzir qualidade para economizar resposta. Usar checkpoints concisos e
  prosseguir. Priorizar código/testes sobre prosa repetitiva.
- Frases proibidas como critério de encerramento: `good enough`, `acceptable for
  MVP`, `mostly complete`, `minor issues remain`, `functionally sufficient`,
  `polish later`.
- MVP significa minimum viable PRODUCT, não minimum visible prototype.
- `10/10` não significa adicionar features pós-MVP: não adicionar Simple Sweep,
  Spline/Bézier Pen, advanced animation, nodes, advanced sculpt, CAD
  constraints ou procedural ecosystem nesta rodada. Perfeição é relativa ao
  escopo definido.

## 240–249. Scorecard final, contagem final, evidências, relatório de tools/UI/código/performance/segurança, condição de RC

- Scorecard por área: `Area | Requirements | Passed | Failed | Untested | Known
  Issues | Evidence | Score`. Uma área recebe `10/10` somente quando
  `Failed = 0`, `Untested = 0` e `Known Issues = 0` dentro do escopo.
- Áreas obrigatórias: Architecture; Code Quality; Build; Static Analysis; Model
  Core; Model UI; Model UX; Paint Core; Paint UI; Paint UX; UV Core; UV UI; UV
  UX; Outliner; Inspector; Asset Library; Menus; Modals; Popovers; Dropdowns;
  Tooltips; Shortcuts; Input; Undo/Redo; Save/Load; Import/Export; Renderer
  Sync; Performance; Memory; Error Handling; Security; Accessibility; i18n;
  Themes; DPI; Responsive Layout; Visual Design; Consistency; Regression;
  Documentation.
- Contagem final: `Blocker: 0`, `Critical: 0`, `High: 0`, `Medium: 0`, `Low: 0`,
  `Known MVP regressions: 0`, `Untested MVP interactions: 0`,
  `Stubs exposed to MVP: 0`.
- Evidências: reportar exatamente build commands, test commands, lint commands,
  audit commands, manual flows completed, resolutions checked, themes checked e
  languages checked. Não inventar nomes de comandos; usar os reais descobertos
  no repo.
- Relatório de tools: para toda ferramenta — name, command id, access paths,
  automated test status, manual test status, Undo/Redo status, persistence
  status, final status.
- Relatório de UI: quantidade inventariada e testada de buttons, icon buttons,
  menu items, submenus, modals, dropdowns, tooltips, context menus, splitters e
  interactive fields. Os números vêm do inventário.
- Relatório de código: format status; lint status; warnings; unsafe blocks;
  remaining TODOs; remaining unwrap/expect hot-path risks; architecture
  findings; duplication findings. Tudo sob controle para `10/10`.
- Relatório de performance: somente números medidos, com ambiente. Quando a
  métrica não foi medida: não inventar. Mas não usar a ausência de benchmark
  sofisticado para ignorar revisão de performance.
- Relatório de segurança: dependency audit; parser robustness; malformed file
  tests; save safety; panic review; unsafe review. Não escrever "100 % secure";
  escrever o que foi verificado.
- O projeto só pode receber `PETUNIA3D MVP — RELEASE CANDIDATE` quando todos os
  gates obrigatórios passaram; todos os requisitos do Master Contract passaram;
  todos os workspaces funcionam; todos os interactive elements foram
  inventariados; todos os interactive elements relevantes foram exercitados;
  nenhum known MVP defect permanece; o scorecard possui `10/10` em TODAS as
  áreas.

## 250. Checagem final absoluta

Antes de encerrar, perguntar e **responder com evidência**:

Existe qualquer botão visível que não faz o que promete? Existe qualquer ícone
sem semântica clara? Existe qualquer tooltip ausente? Existe qualquer menu item
morto? Existe qualquer submenu errado? Existe qualquer dropdown falso? Existe
qualquer modal quebrada? Existe qualquer field desconectado? Existe qualquer
splitter falso? Existe qualquer estado disabled incorreto? Existe qualquer
shortcut conflitante? Existe qualquer ferramenta parcial? Existe qualquer
preview mentiroso? Existe qualquer Undo incompleto? Existe qualquer Save
incompleto? Existe qualquer Paint falso? Existe qualquer UV falso? Existe
qualquer painel que pareça placeholder? Existe qualquer texto hardcoded público?
Existe qualquer componente ilegível em Light/Dark? Existe clipping em pt-BR ou
pseudo locale? Existe qualquer crash conhecido? Existe qualquer data-loss path
conhecido? Existe qualquer warning de código nosso sem análise? Existe qualquer
TODO funcional do MVP? Existe qualquer interação ainda `NOT TESTED`?

Se qualquer resposta for `YES`, o Gauntlet continua.

## 251. Última regra

Não se apaixonar pela implementação. Atacá-la. Tentar quebrá-la: tentar
confundir a seleção; cancelar operações; desfazer operações; repetir operações;
alternar workspaces; carregar dados inválidos; redimensionar a UI; trocar tema;
trocar idioma; operar sem seleção; operar com seleção inválida; salvar; fechar;
reabrir; encontrar inconsistências visuais; encontrar inconsistências
arquiteturais; encontrar acoplamento; encontrar estado duplicado; encontrar
código difícil de manter; encontrar qualquer detalhe que faria um usuário dizer
"isso ainda parece inacabado".

Então corrigir. Repetir.

O Gauntlet não existe para provar que o Petunia está bom. O Gauntlet existe para
encontrar todas as razões pelas quais ele ainda **não** está bom. Somente quando
ele parar de encontrar defeitos dentro do escopo do MVP, e todas as evidências
confirmarem isso, o trabalho termina.
