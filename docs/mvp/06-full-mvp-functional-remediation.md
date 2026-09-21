# Petunia3D — Full MVP Functional Remediation + Modern UI Implementation

> Esta tarefa **não** é uma refatoração visual. A implementação atual da interface
> falhou em entregar um editor 3D funcional: ferramentas visíveis não funcionam;
> funcionalidades do MVP estão ausentes; não há Outliner funcional; não há
> Properties/Inspector adequado; não há Tool Properties completo; não há
> redimensionamento real de painéis; não há Asset Library satisfatória; não há
> fluxo de edição coerente; não existe integração completa entre interface,
> comandos, domínio, seleção, viewport, undo/redo e persistência; vários
> controles aparentam ser apenas decoração.
>
> O objetivo é transformar o estado atual em um MVP **realmente utilizável**.

## 0. Regra mais importante

Uma ferramenta só é considerada **IMPLEMENTADA** quando existe o caminho
completo:

```
UI → comando → application/controller → domínio → alteração real dos dados
→ atualização da seleção quando aplicável
→ atualização da geometria/UV/textura
→ atualização do renderer
→ live preview quando necessário
→ Commit/Cancel → Undo/Redo → serialização/persistência
→ reabertura correta do projeto
```

Nenhum botão de MVP pode terminar em noop, mock, `println!`, log sem efeito,
`todo!()`, `unimplemented!()`, placeholder, implementação simulada, código morto
ou callback vazio. **ZERO FAKE UI.**

## 1. Auditar o repositório inteiro

Antes de alterar a arquitetura visual, inspecionar o código procurando `todo!()`,
`unimplemented!()`, `panic!()` como placeholder, `TODO`, `FIXME`, `TEMP`,
`STUB`, `MOCK`, `PLACEHOLDER` e também: callbacks vazios; buttons sem action;
commands sem handler; handlers que não alteram estado; duplicação de estados
UI/Core; ferramentas registradas mas nunca executadas; ferramentas implementadas
no domínio mas não conectadas à UI; UI que mantém cópia própria do estado da
cena; operações que funcionam mas não geram Undo; operações que não invalidam o
renderer; controles numéricos desconectados do modelo; seleção visual diferente
da seleção do documento; valores hardcoded que deveriam estar nos tokens de UI.

Matriz:

```
Feature | UI | Command | Backend | Undo | Renderer | Persistence | Status
```

Estados: `COMPLETE`, `PARTIAL`, `UI_ONLY`, `BACKEND_ONLY`, `STUB`, `MISSING`,
`BROKEN`. **Não parar na matriz**: usá-la para implementar as lacunas.

## 2. Não reescrever o core dentro do Slint

```
Slint → UI Presenter/Controller → Application Commands
      → Petunia Domain/Core → Geometry/Paint/UV/Scene → Renderer
```

`.slint` contém: layout, componentes, bindings, estados de apresentação,
eventos/callbacks, animações de UI, tokens, componentes reutilizáveis. Rust
contém: commands, transactions, validation, selection, tool states, document
editing, undo, redo, geometry, paint, UV, persistence, renderer synchronization.

Não implementar algoritmos geométricos em `.slint`. Não colocar lógica de edição
dentro de componentes visuais.

## 3. Implementação transacional das ferramentas

```
Idle → Begin → Preview → Update → Commit
Idle → Begin → Preview → Cancel
```

Um gesto completo gera **UMA** entrada de Undo. Ex.: `mouse down → BeginExtrude`;
`mouse move → PreviewExtrude`; campo numérico alterado → `UpdateExtrude`;
`mouse up / Enter → CommitExtrude`; `Esc / clique → CancelExtrude`.

`Cancel` restaura exatamente o estado anterior. `Undo` restaura exatamente o
estado anterior. `Redo` produz exatamente o resultado anterior.

## 4. A interface atual não é referência final

Medidas da captura atual (~1800 × 1012): header ~41 px; rail esquerdo global
~56 px; toolbar ~44 px; viewport x ~100 a ~1523; inspector direito ~275 px;
status ~29 px. Essas medidas servem apenas para entender o problema; o layout
atual **não** precisa ser preservado e pode ser reorganizado completamente.

Objetivo: `CANVAS FIRST`, `SELECTION DRIVEN`, `CONTEXTUAL`, `COMPACT`,
`PROGRESSIVE DISCLOSURE`, `MODERN`, `LOW VISUAL NOISE`.

Não copiar Blender, Maya ou Shapr3D; estudar os paradigmas, mantendo a
identidade Petunia.

## 5. Novo shell obrigatório

Todas as medidas em **logical pixels a 100 % DPI**, escalando corretamente com
DPI.

### Top Application Bar

Altura 40 px. Border inferior 1 px. Padding horizontal 8 px.

```
[Petunia]
[File] [Edit] [View] [Window]
                    [ MODEL ][ PAINT ][ UV ]
        [Undo][Redo]
[Save state] [Open] [Save] [Search] [Settings]
```

Workspace buttons: altura 32 px; min-width 96 px; border radius 6 px; gap 4 px.
Ativo: accent background. Inativo: surface raised. Hover: surface hover. Não
usar botões gigantes.

### Left Outliner

Default 232 px; mínimo 180; máximo 400; resizable SIM; collapsible SIM;
persistir tamanho.

```
Scene
────────────
▾ Cube
▾ Character
  ├ Body
  ├ Head
  └ Weapon
```

Cada linha: altura 24 px; padding horizontal 6 px; indent 16 px por nível; ícone
16 px; disclosure 12–14 px; gap 4 px.

Estados: normal, hover, selected, active, disabled, hidden, locked.

Funcionalidades: selection sincronizada; multi-selection; rename; F2 rename;
double-click; visibility; lock; hierarchy; drag and drop; reorder; parenting
quando suportado; context menu; duplicate; delete.

Nunca manter seleção independente no Outliner. **OUTLINER == DOCUMENT
SELECTION.**

### Viewport Tool Rail

Largura 44 px; botões 36 × 36 px; margem 4 px; ícones 18–20 px; gap 2–4 px.
Categorias agrupadas em trays; não mostrar 30 ferramentas simultaneamente.
Exemplo de categorias: `Select`, `Transform`, `Draw`, `Mesh`, `Topology`,
`Create`. Ao clicar numa categoria, abrir tray/popover contextual, ex.:

```
[Extrude] [Inset] [Bevel] [Loop Cut]
```

O tray deve abrir perto da ferramenta; fundo raised; border; shadow leve;
padding 6–8 px; fechar ao clicar fora; fechar com Esc; manter-se aberto quando
pinned.

### Viewport

Continua sendo a maior região. Header interno 30–32 px com: `Perspective /
Orthographic`, `View`, `Shading`, `Overlays`, `X-Ray`, `Grid`, `Reference`,
camera tools. Não usar texto para ações que funcionam melhor como ícone.
Tooltips obrigatórios. Deve suportar orbit, pan, zoom, frame selected, frame
all, front, back, left, right, top, bottom, perspective e orthographic.

### Right Context / Properties

Default 304 px; mínimo 260; máximo 460; resizable SIM; collapsible SIM. Não usar
painel vazio gigante. Painel **contextual**.

Sem objeto selecionado: `Scene`, `Document`, `Environment`, `Grid`, `Reference`.
Com objeto selecionado: `Transform`, `Object`, `Geometry`, `Material`,
`Visibility`. Com ferramenta ativa: `TOOL PROPERTIES` aparece **primeiro**:

```
Extrude
Distance      [ 1.000 ]
Direction     [ Normal ▼ ]
Individual    [ ○ ]
Keep Faces    [ ● ]
Apply   Cancel
```

Tool properties acompanha **live preview**.

### Sections

Header 28 px; padding 8 px; chevron 14 px. Conteúdo: vertical spacing 6 px;
section gap 8 px.

### Label / Control grid

Labels: largura preferencial 88–100 px. Control: restante. Altura de controles
28 px.

### Numeric field

`PetuniaNumberField`: altura 28 px; radius 4 px; padding horizontal 6 px.

Comportamentos: click edita texto; drag horizontal scrub; Shift+drag precisão
fina; Ctrl+drag incremento maior; double click seleciona valor; Enter commit;
Esc cancel; arrows increment/decrement; click outside commit.

Aceitar: `min`, `max`, `step`, `decimals`, `unit`, `default`, `drag
sensitivity`.

### Vectors

Vec2/Vec3 com campos compactos. Ex.:

```
Position
X [0.00]  Y [0.00]  Z [0.00]
```

Não desperdiçar metade do inspector.

### Sliders

Slider: altura 28 px; track 4 px; thumb 12 px. Valor numérico clicável. Ao
clicar no valor, trocar por `PetuniaNumberField`; clique fora retorna ao slider.

### Bottom Asset Library

Default 176 px; mínimo 120; máximo 360; resizable verticalmente SIM;
collapsible SIM.

```
[ Assets ] [ Materials ] [ Textures ] [ References ]
Search...            [Grid/List toggle]
assets...
```

Itens: thumbnail 64 × 64; modo compacto 48 × 48; nome em uma linha com ellipsis;
tooltip com nome completo + metadata. Suportar drag & drop para contexto válido.

### Status bar

Altura 24 px. Esquerda: status atual. Centro: instruções da ferramenta. Direita:
selection stats, mesh stats, notifications/tasks.

```
Extrude
LMB confirm · RMB/Esc cancel · Shift precise
Cube
8 verts · 12 tris
```

## 6. Splitters e redimensionamento

Implementar, não simular. Splitter visual 1 px; hit target 6 px; cursor
`col-resize` ou `row-resize`; hover com realce discreto; drag com
redimensionamento contínuo; double-click restaura tamanho default. Sizes devem
ser persistidos.

Limites obrigatórios para impedir: viewport zerado; painel negativo; inspector
ocupando a tela inteira acidentalmente.

Responsive behavior: `>= 1600 px` layout completo; `1366–1599` sidebars menores;
`< 1200` Asset Library fechada por padrão e painéis secundários podem virar
drawers. Viewport **sempre** prioritário.

## 7. Drawers / floating panels

Filosofia de progressive disclosure. Use drawers/popovers para funcionalidades
secundárias. Floating panels precisam: arrastar; expandir; colapsar; pin; close;
manter dentro dos limites da janela. Não criar dezenas de janelas permanentemente
abertas. Ao clicar fora de um popover não pinned: fechar. Esc: fechar.

## 8. Tokens visuais

Não espalhar cores hardcoded pelos componentes. Criar tokens.

```
space-1 = 4   space-2 = 8   space-3 = 12   space-4 = 16   space-5 = 20   space-6 = 24
radius-sm = 3   radius-md = 5   radius-lg = 7   radius-xl = 10
control-sm = 24   control-md = 28   control-lg = 32
icon-sm = 14   icon-md = 18   icon-lg = 20   icon-xl = 24
caption = 11   body-small = 12   body = 13   label = 12
section = 12–13 semibold   workspace = 12 semibold
```

Evitar texto minúsculo ilegível.

## 9. Cores

Manter graphite/dark, melhorando a separação de planos. Níveis claros: `canvas`,
`surface`, `surface-raised`, `surface-overlay`, `control`, `control-hover`,
`control-active`, `selected`, `accent`, `border`, `border-strong`. Não usar
apenas três tons quase iguais. Texto: `primary`, `secondary`, `muted`,
`disabled`. Estados: `hover`, `pressed`, `focus`, `selected`, `disabled`,
`danger`, `warning`, `success`. Garantir contraste legível.

## 10. Ícones

Baseline: `lucide-slint` e Tabler Icons. SVG-first. Criar ícones Petunia
customizados apenas quando não existir equivalente adequado. Todos com stroke
visual consistente, grid consistente, mesmo peso e alinhamento óptico. Não
misturar ícones visualmente incompatíveis.

## 11. MODEL — MVP obrigatório

### Selection modes

`Object`, `Vertex/Point`, `Edge`, `Face`. A mudança deve alterar: picking;
highlight; Outliner quando aplicável; Tool Properties; commands disponíveis.

### Transform

Move, Rotate, Scale com gizmo visível e funcional. Suportar `X`, `Y`, `Z` e
planos quando aplicável. Orientações: `Global`, `Local`, `Normal` quando
aplicável. Snap: `Grid`, `Vertex`, `Edge`, `Face`. Entrada numérica. Transform
properties. Undo/Redo.

### Proportional editing

Ativar/desativar. Raio visual. Falloff quando já definido pelo core. Live
preview.

### Primitives

`Cube`, `Plane`, `Cylinder`, `Cone`, `Capsule`. Ao criar: aparecer na cena;
aparecer no Outliner; ficar selecionado; mostrar Tool Properties; permitir
parâmetros relevantes; Commit; Cancel; Undo.

### Draw / Profile

Parte central da identidade do Petunia. Implementar `Profile Pen`, `Polygon
Pen`, `Polyline`, `Rectangle`, `Circle`, `Arc`, `Draw on Face`.

Fluxo esperado: selecionar work plane/surface → desenhar → editar pontos →
fechar profile → gerar shape → opcionalmente criar volume.

Mostrar: points; segments; snap; closing indication; preview. Ao fechar o
perfil, apresentar ações contextuais como `Extrude` e `Revolve` quando válidas.

### Ferramentas de mesh obrigatórias

`Extrude` (faces, live preview, distance, normal/direction, entrada numérica,
commit, cancel, undo); `Push / Pull` (interação direta sobre face, mouse drag,
distance, input numérico, live preview); `Inset` (faces, amount,
individual/region quando suportado, preview real); `Bevel / Chamfer`
(edges/faces conforme backend, width, segments quando suportado, preview);
`Cut / Slice / Knife` (traçado visual, snap, intersection real, commit, cancel —
não pode apenas desenhar linha na tela); `Loop Cut` (detectar edge loop,
preview antes do commit, position, multiple cuts quando suportado); `Even Loop
Cut` (spacing uniforme); `Subdivide` (aplicação real, iterations quando
suportado); `Mirror / Symmetry` (axes, preview, origem correta); `Duplicate`
(objeto/seleção válida, novo ID, Outliner atualizado); `Dissolve` (vertex/edge/
face conforme operação válida; não tratar como Delete); `Merge` (merge
selected); `Weld` (por threshold quando aplicável); `Bridge` (entre loops/edges
válidos); `Connect`; `Fill` (boundary válido); `Separate` (seleção em novo
objeto); `Join` (combinar objetos em mesh coerente); `Keep Parts` (preservar
componentes conforme especificação existente); `Fuse` (usar implementação
existente/correta do Petunia; não introduzir boolean system gigante); `Flip
Normals` (real, atualizar renderer imediatamente); `Flip Diagonal` (trocar
diagonal/topologia corretamente); `Revolve` (profile → axis → angle → segments →
geometry, preview obrigatório).

## 12. Não implementar agora como parte do MVP

Não aumentar o escopo. `Simple Sweep` e `Spline/Bezier Pen` são pós-MVP/V1.x e
podem manter pontos de extensão arquitetural. Não gastar tempo transformando o
Petunia em CAD completo.

## 13. PAINT — MVP

Pintura 3D sobre superfície e visualização/edição da textura. Ferramentas:
`Brush/Pencil`, `Soft Brush`, `Eraser`, `Fill`, `Eyedropper/Color Picker`,
`Line`, `Rectangle`.

- **Brush**: size, opacity, hardness quando aplicável, spacing quando aplicável;
  preview do cursor sobre a superfície; raycast correto; stroke contínuo; não
  criar buracos entre samples.
- **Pixel brush**: modo adequado a low-poly/pixel art; grid opcional; pixel
  snapping.
- **Soft brush**: falloff correto.
- **Eraser**: respeitar layer/alpha.
- **Fill**: não pintar a textura inteira incorretamente; implementar o escopo
  previsto pelo core.
- **Eyedropper**: amostragem real da textura/material.
- **Line/Rectangle**: preview antes de aplicar.
- **Color**: foreground color, recent colors, palette, import palette, export
  palette.
- **Layers**: painel real com create, delete, rename, visibility, opacity,
  reorder, merge, flatten; layer selecionada claramente.
- **Channels**: MVP editável `Base Color` e `Alpha`; Normal/Roughness/Height
  podem ser importados/visualizados quando já previsto, sem inventar edição
  avançada.
- **Undo**: um stroke = uma operação lógica de Undo. Não duplicar a textura
  inteira desnecessariamente a cada mouse event se a arquitetura já possuir
  dirty regions/tiles.
- **Seams**: pintura 3D não pode produzir falhas gritantes em seams quando
  houver suporte existente para seam-aware painting.

## 14. UV — MVP

UV não pode ser apenas uma aba vazia. Layout: viewport 3D + UV Editor 2D.
Selecionar no 3D reflete no UV; selecionar no UV reflete no 3D. UV armazenado por
corner/face-vertex quando essa for a representação canônica.

- **Selection**: Vertex, Edge, Face, Island quando aplicável ao modelo de seleção
  existente.
- **UV transform**: Move, Rotate, Scale, numeric input.
- **Seams**: Mark Seam, Clear Seam, visualizar seams no 3D.
- **Unwrap**: executar algoritmo real; usar xatlas quando ele fizer parte da
  implementação atual; não retornar UV fictício.
- **Pack**: Pack Islands real; respeitar bounds 0–1 quando configurado assim;
  padding.
- **Project**: Project From View; outras projeções só quando já fizerem parte da
  especificação/backend atual.
- **UV view**: checker texture, grid, island bounds, selection highlight,
  background texture.
- **Diagnostics**: overlap, stretch e texel density quando já suportados pelo
  core; não bloquear o MVP caso sejam oficialmente pós-MVP.

## 15. Material

Properties deve apresentar Material real: material slots, Base Color, texture
assignment e Alpha quando aplicável. O preview deve atualizar o viewport
imediatamente. A Asset Library deve permitir atribuir material/textura por
contexto apropriado.

## 16. Asset Library

Criar um browser funcional (não apenas um botão escrito "Asset Library").
Suportar meshes, materials, textures e references. Funções: search, filter,
grid, list, select, rename quando permitido, delete quando permitido, drag and
drop, refresh, import.

## 17. Reference workflow

Petunia é reference-first/shape-first; reference é parte do fluxo. Adicionar
referência; imagem no viewport; opacity; visibility; transform; lock;
front/side/view-plane quando previsto. A reference deve aparecer no Outliner ou
sistema equivalente.

## 18. Contextual UX

Face selecionada → ferramentas de face relevantes (`Extrude`, `Inset`,
`Push/Pull`, `Bevel`, `Delete`). Edge selecionada → `Bevel`, `Loop Cut`,
`Dissolve`, `Bridge` quando válido. Vertex selecionado → `Move`, `Merge`,
`Weld`. Não mostrar todas as operações possíveis o tempo inteiro.

## 19. Tool Property Panel

Toda ferramenta paramétrica declara suas propriedades:

```
ExtrudeToolProperties { distance, direction, individual }
BevelToolProperties { width, segments }
CylinderCreateProperties { radius, height, segments }
```

A UI deve ser gerada de forma consistente. Não fazer cada ferramenta reinventar
sliders/campos.

## 20. Command Registry

Consolidar um registry de comandos. Cada ação possui: `CommandId`, nome
localizado, ícone, tooltip, shortcut, `can_execute`, `execute`. Ex.:
`model.extrude`, `model.inset`, `model.bevel`, `model.loop_cut`,
`model.subdivide`, `paint.brush`, `paint.fill`, `uv.unwrap`, `uv.pack`.

Menu, toolbar, shortcut e contextual menu apontam para o **MESMO** comando. Não
duplicar lógica.

## 21. Estados de enable/disable

Botões não podem estar habilitados quando a operação é inválida. Ex.: Extrude
somente com seleção adequada; Bridge somente com seleção válida; UV Unwrap
somente quando mesh/seleção suporta. Tooltip de item disabled pode explicar o
motivo.

## 22. Gizmo

Feedback de manipulação claro: Move, Rotate, Scale. Tamanho visual utilizável em
diferentes distâncias. Axes X/Y/Z; hover highlight; active axis highlight mais
forte; arrastar com live preview; Esc cancela; input numérico permitido.

## 23. Selection visuals

Object: outline/highlight coerente. Vertex: points claramente visíveis. Edge:
edge highlight. Face: fill overlay + border. Diferenciar active de selected.
Evitar cores saturadas ocupando faces inteiras.

## 24. Notifications

Não abrir modal para cada erro. Usar toast/status bar para `saved`, `export
complete`, `invalid operation`, `import failed`. Modal somente para decisões que
realmente bloqueiam o fluxo.

## 25. Accessibility

Suportar keyboard navigation; focus visible; tooltips; DPI scaling; 150 %;
200 %; high contrast; localização; text expansion; focus order coerente. Área
clicável não pode ser ridiculamente pequena: ícone visual pode ter 18 px, mas o
hit target fica próximo de 32–36 px.

## 26. i18n

Não hardcodar strings públicas na UI. Usar o sistema de TextId/i18n existente.
Garantir `en` canônico, `pt-BR` e pseudo locale. A UI precisa sobreviver a
strings maiores.

## 27. Tema

Tematização completamente tokenizada. Nada do tipo `#33353b`, `#555`, `12px`,
`7px` espalhado por dezenas de arquivos. Todos os valores compartilhados vêm de
tokens.

## 28. Performance de UI

Evitar recomputar Outliner inteiro, Asset Library inteira ou Inspector inteiro a
cada frame sem necessidade. Atualização por eventos/state. Viewport não deve ser
invalidado por hover em botão sem relação com render. Evitar allocations
gigantes em loop de frame. Não regenerar thumbnails continuamente.

## 29. Save / Load

O fluxo `New Project → Create Cube → Edit mesh → Paint → Edit UV → Save → Close
→ Open` deve permanecer equivalente, incluindo objects, transforms, geometry,
materials, textures, UVs, references, layers relevantes, selection state quando
apropriado e workspace/layout preferences quando apropriado.

## 30. Import / Export

Manter o pipeline previsto de glTF/GLB, OBJ e assets de imagem necessários. Não
mostrar opção de exportação se o backend não estiver conectado.

## 31. Menus

```
FILE     New · Open · Save · Save As · Import · Export · Quit
EDIT     Undo · Redo · Duplicate · Delete · Preferences quando pertinente
VIEW     Frame Selected · Frame All · Perspective/Orthographic · Grid
         · Overlays · X-Ray · Shading
WINDOW   Outliner · Properties · Asset Library · Reset Layout
```

Menus precisam executar os mesmos Commands usados pela toolbar.

## 32. Shortcuts

Não criar shortcuts em múltiplos lugares. Usar o `KeymapResolver`/keymap system
existente. Mostrar o shortcut no tooltip:

```
Extrude
Extrude selected faces
E
```

## 33. Hover / focus / pressed

Todo componente interativo distingue visualmente: default, hover, pressed,
selected, focus, disabled. A implementação atual parece "chapada"; corrigir.

## 34. Tooltip

Delay aproximado 400–600 ms.

```
Extrude                  E
Extrude selected faces.
```

Tooltips mais detalhados podem apresentar uma linha adicional.

## 35. Empty states

Não deixar painel com oceano vazio. Sem seleção:

```
Nothing selected
Select an object to inspect its properties.
```

Com CTA discreto quando apropriado. Asset Library vazia:

```
No assets yet
Import Asset
```

## 36. Layout persistence

Persistir: left panel width, right panel width, bottom panel height, collapsed
state, active workspace e tool panels pinned quando previsto. Adicionar `Window
→ Reset Layout`.

## 37. Resoluções obrigatórias

Projetar para 1366×768, 1600×900, 1920×1080 e 2560×1440. Em resolução menor,
não comprimir o viewport até ficar inutilizável; painéis secundários recolhem.

## 38. Definição de "Done" para cada ferramenta

- [ ] aparece no lugar correto;
- [ ] possui ícone coerente;
- [ ] possui tooltip;
- [ ] respeita `can_execute`;
- [ ] ativa modo correto;
- [ ] fornece feedback visual;
- [ ] altera os dados reais;
- [ ] possui live preview quando apropriado;
- [ ] permite entrada numérica quando apropriado;
- [ ] Commit funciona;
- [ ] Cancel funciona;
- [ ] Esc funciona;
- [ ] Undo funciona;
- [ ] Redo funciona;
- [ ] renderer atualiza;
- [ ] Outliner atualiza quando necessário;
- [ ] Properties atualiza;
- [ ] Save mantém resultado;
- [ ] Reload mantém resultado;
- [ ] não contém placeholder.

Se **uma** dessas responsabilidades essenciais estiver faltando, a feature
permanece classificada `PARTIAL`.

## 39. Ordem de implementação

```
PHASE A  Audit e bindings quebrados.
PHASE B  Command architecture e state synchronization.
PHASE C  Shell: Header, Outliner, Viewport, Properties, Asset Library, Status Bar, Splitters.
PHASE D  Selection + Gizmo + Transform.
PHASE E  MODEL tools.
PHASE F  PAINT.
PHASE G  UV.
PHASE H  Materials / Assets / References.
PHASE I  Save/Load/Import/Export integration.
PHASE J  UI polish e responsive behavior.
```

Não começar adicionando animações bonitas enquanto os comandos ainda são noop.

## 40. Não fazer

Não reconstruir o Petunia como Blender. Não aumentar o MVP. Não implementar
Simple Sweep nem Spline/Bezier Pen agora. Não introduzir node editor nem sistema
procedural complexo. Não encher o programa de modais. Não deixar sidebar
ocupando metade do viewport. Não esconder funcionalidades essenciais em menus
obscuros. Não implementar geometria dentro do Slint. Não criar estado duplicado
de documento na UI. Não criar dezenas de callbacks específicos quando o
`CommandId` resolve. Não criar botão sem implementação. Não deixar placeholder
visível. Não considerar código que compila equivalente a feature funcionando.

## 41. Stack de UI

Continuar com a interface Slint moderna. Ferramentas já escolhidas: Slint,
slintcn, lucide-slint, Tabler Icons, Slint Viewer/LSP, rfd. Não voltar para egui
nesta implementação. Petunia Bloom precisa ser a interface visualmente
refinada. Core e funcionalidades continuam compartilháveis com as demais
interfaces.

## 42. Resultado visual esperado

Editor profissional; compacto; limpo; rápido; moderno; coeso; pouco intimidador;
viewport-first. **Não** deve parecer dashboard web, formulário empresarial,
mockup, painel administrativo ou cópia simplificada do Blender. A experiência
lembra a simplicidade de ferramentas modernas e contextuais, preservando o
paradigma próprio do Petunia.

## 43. Critério final

Deve ser possível abrir o Petunia3D e realizar, sem código externo: criar um
objeto; selecionar Object/Vertex/Edge/Face; mover/rotacionar/escalar; modelar
usando as ferramentas MVP; criar formas pelo Draw/Profile; editar propriedades;
gerenciar objetos pelo Outliner; utilizar a Asset Library; trabalhar com
materiais; abrir PAINT; pintar realmente; utilizar layers; abrir UV; editar UV;
unwrap; pack; marcar seams; retornar a MODEL sem perder estado; Undo/Redo entre
operações; salvar; fechar; reabrir; encontrar o projeto equivalente; exportar.

Se o usuário clicar em uma ferramenta do MVP e nada acontecer, o MVP **não** está
pronto. Se o botão apenas mudar de cor, o MVP **não** está pronto. Se o backend
existir mas não estiver acessível pela interface, o MVP **não** está pronto. Se a
interface existir mas chamar um stub, o MVP **não** está pronto.

## 44. Restrição de execução

Implementar primeiro. Não executar test suites, benchmarks, clippy, smoke tests
ou conformance suites até autorização explícita. Ainda assim: inspecionar o
código; raciocinar sobre invariantes; evitar quebrar APIs; preparar a matriz de
verificação; deixar a implementação pronta para a validação posterior. Não usar
a ausência de execução de testes como justificativa para entregar código
parcial.

## 45. Relatório final

Entregar: A. arquivos modificados; B. componentes novos; C. comandos conectados;
D. ferramentas MODEL implementadas; E. ferramentas PAINT implementadas;
F. ferramentas UV implementadas; G. Outliner; H. Properties; I. Tool Properties;
J. Asset Library; K. splitters/redimensionamento; L. Undo/Redo; M. Save/Load;
N. pontos que eram stubs e agora são implementações reais; O. qualquer lacuna
restante, com arquivo/função exatos.

Não esconder lacunas. Não marcar `PARTIAL` como `COMPLETE`. O objetivo é
transformar o Petunia3D de uma demonstração visual em um editor 3D MVP realmente
utilizável.
