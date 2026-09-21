# Petunia3D — Master MVP Implementation Contract

> Contrato de UI / UX / interação / ferramentas / comportamento / funcionalidade.
> Este documento é normativo. Não é inspiração, sugestão, mockup conceitual,
> lista de ideias ou guideline opcional.

## 0. Autoridade

Quando houver divergência entre a UI atualmente implementada, código legado,
componentes antigos, mockups antigos ou comportamentos improvisados **e** este
contrato, **este contrato prevalece**, exceto quando uma restrição estrutural
comprovada do core impedir tecnicamente determinado comportamento. Nesse caso:

1. não inventar comportamento alternativo silenciosamente;
2. documentar a limitação;
3. implementar a melhor integração possível;
4. marcar a funcionalidade como `PARTIAL`;
5. informar arquivo/função/limitação no relatório final.

## 1. Objetivo

Transformar o Petunia3D em um modelador 3D low-poly realmente utilizável, com
três workspaces principais — `MODEL`, `PAINT`, `UV` — compartilhando o mesmo
documento, cena, seleção, materiais, texturas, UVs, undo/redo, assets, renderer
e persistência, mas oferecendo **interfaces diferentes e apropriadas** para cada
tarefa.

O programa deve parecer: moderno, leve, profissional, contextual, acessível,
previsível, pouco intimidador, rápido, viewport-first, orientado a criação.

O programa **não** deve parecer: um mockup, um dashboard web, uma interface
administrativa, uma cópia pobre do Blender, um viewer com botões, três abas
contendo o mesmo viewport.

## 2. Princípio fundamental de implementação

**ZERO FAKE UI.** Uma funcionalidade só é `COMPLETE` quando o fluxo relevante
chega ao estado REAL do documento:

```
Button
→ Command
→ Tool Controller
→ Domain/Core
→ Document Mutation
→ Undo Transaction
→ Renderer Update
→ UI State Update
→ Save
→ Reload
```

É proibido considerar `COMPLETE` algo contendo `TODO`, `FIXME`, `todo!()`,
`unimplemented!()`, panic placeholder, mock, stub, noop, `println` como
implementação, callback vazio, estatística falsa, preview falso ou resultado
hardcoded.

## 3. Auditoria obrigatória antes da remediação

Inspecionar todo o projeto procurando `TODO`, `FIXME`, `HACK`, `TEMP`, `MOCK`,
`STUB`, `PLACEHOLDER`, `todo!()`, `unimplemented!()` e também:

- callbacks vazios;
- commands sem handler;
- buttons sem command;
- menus sem command;
- shortcuts desconectados;
- estado duplicado UI/Core;
- selection duplicada;
- renderer não invalidado;
- modais sem resultado;
- dropdowns com itens sem comportamento;
- campos numéricos que não alteram documento;
- ícones que mudam de estado sem ativar ferramenta;
- Undo parcial; Redo parcial;
- Save que ignora dados; Load que perde dados;
- workspaces que recriam documento;
- Paint que não altera textura real;
- UV que não altera coordenadas reais.

Produzir a matriz:

```
Feature | Command | UI | Input | Core | Preview | Commit | Cancel | Undo | Redo | Renderer | Persistence | Status
```

Valores permitidos: `COMPLETE`, `PARTIAL`, `UI_ONLY`, `BACKEND_ONLY`, `BROKEN`,
`STUB`, `MISSING`.

Depois **implementar as lacunas**. Não parar na auditoria.

## 4. Arquitetura

```
Slint
↓
Presentation State
↓
UI Controller
↓
Command Registry
↓
Tool Controller
↓
Application Layer
↓
Document / Domain Core
↓
Geometry / Paint / UV
↓
Renderer
```

Arquivos `.slint` **não** devem conter: algoritmos geométricos, topologia,
unwrap, painting algorithms, serialização, Undo, picking complexo, document
mutation.

Slint deve cuidar de: layout, visual state, input forwarding, bindings,
component composition, animações curtas, acessibilidade, presentation.

## 5. Command system

Toda ação reutilizável deve possuir `CommandId` único. Exemplos:

```
file.new / file.open / file.save / file.save_as
edit.undo / edit.redo / edit.duplicate / edit.delete
view.frame_selected / view.frame_all / view.toggle_grid
model.select.object / model.select.vertex / model.select.edge / model.select.face
model.move / model.rotate / model.scale
model.extrude / model.inset / model.bevel
paint.brush / paint.eraser / paint.fill
uv.unwrap / uv.pack / uv.mark_seam
```

Cada Command deve declarar: `id`, `label_token`, `description_token`, `icon_id`,
`shortcut`, `can_execute`, `execute`.

Menus, toolbar, context menu, shortcut e Search devem apontar para **o mesmo**
Command. Nunca copiar a lógica da operação em vários callbacks.

## 6. Tool State Machine

```
IDLE → ARMED → BEGIN → PREVIEW → UPDATE → COMMIT
```

ou

```
PREVIEW → CANCEL
```

Exemplo Extrude: `E` ativa a ferramenta; LMB em seleção válida inicia (`Begin`);
mouse move pré-visualiza (`Preview`); digitar valor atualiza (`Update`);
LMB/Enter confirma (`Commit`); RMB/Esc cancela (`Cancel`).

`Cancel` restaura exatamente o estado anterior. `Commit` gera **uma única**
operação lógica de Undo.

## 7. Undo / Redo

`Ctrl+Z` desfaz. `Ctrl+Shift+Z` refaz. `Ctrl+Y` é alternativa de refazer.

Operações interativas devem agrupar eventos:

- errado: 300 eventos de mouse move = 300 Undos;
- correto: um stroke = um Undo; um drag de Move = um Undo; um Extrude completo =
  um Undo; um resize de layer opacity (`mouse down → drag → mouse up`) = um
  Undo.

## 8. Sistema de medidas

Todos os valores visuais deste documento são **logical pixels**. A 100 %, 1
logical px ≈ 1 physical px. A UI precisa continuar funcional em `100 %`, `125 %`,
`150 %`, `175 %` e `200 %`. Não usar posicionamento absoluto dependente de
resolução.

## 9. Grid visual

Base: 4 px.

```
space.0  = 0
space.1  = 4
space.2  = 8
space.3  = 12
space.4  = 16
space.5  = 20
space.6  = 24
space.8  = 32
space.10 = 40
space.12 = 48
```

Não criar 5, 7, 13 ou 19 px arbitrariamente. Exceções: borders, hit areas,
alinhamento óptico.

## 10. Tipografia

Fonte de interface limpa; preferir `system-ui` ou a fonte UI já disponível no
projeto. Não introduzir dependência pesada apenas para fonte.

Pesos: Regular 400, Medium 500, Semibold 600. Evitar Bold 700 excessivo.

| Token | Size | Line height | Peso |
|---|---|---|---|
| `text.caption` | 11 | 14 | 400 |
| `text.small` | 12 | 16 | 400 |
| `text.body` | 13 | 18 | 400 |
| `text.body.medium` | 13 | 18 | 500 |
| `text.section` | 12 | 16 | 600 |
| `text.menu` | 12 | 18 | 400 |
| `text.menu.shortcut` | 11 | 16 | 400 |
| `text.workspace` | 11 | 16 | 600, letter-spacing 0.3 |
| `text.window_title` | 13 | 18 | 600 |
| `text.modal_title` | 16 | 22 | 600 |
| `text.empty_title` | 14 | 20 | 600 |
| `text.empty_body` | 12 | 18 | 400 |

Nunca utilizar texto funcional abaixo de 11 px.

## 11. Alinhamento de texto

Texto de buttons centralizado opticamente. Icon + text: gap 6 px. Label +
shortcut em menus: shortcut à direita. Numeric fields: valor à direita. Tree
labels: à esquerda.

## 12. Radii

```
radius.none = 0
radius.xs   = 2
radius.sm   = 4
radius.md   = 6
radius.lg   = 8
radius.xl   = 12
```

Buttons comuns 4 px. Popovers 6 px. Modals 8 px. Não usar pills
indiscriminadamente. Workspace selector pode usar 5–6 px.

## 13. Borders

`border.thin` = 1 px. Focus = 2 px. Divisores = 1 px. Evitar bordas grossas
desnecessárias.

## 14. Dark theme — default

Tokenizar tudo. Não espalhar HEX diretamente pelos componentes.

```
color.canvas               #121419
color.canvas.secondary     #17191F
color.app_bar              #24272F
color.panel                #292D35
color.panel.raised         #303540
color.panel.overlay        #363B46
color.control              #353A45
color.control.hover        #3D4350
color.control.pressed      #454C5A
color.control.disabled     #2E323A
color.border               #3A3F49
color.border.strong        #4B515D
color.text.primary         #F2F4F7
color.text.secondary       #BEC4CE
color.text.muted           #858C98
color.text.disabled        #656B76
color.accent               #4C8DFF
color.accent.hover         #609AFF
color.accent.pressed       #397CEB
color.accent.soft          rgba(76,141,255,0.18)
color.success              #45C98A
color.warning              #E9AA46
color.danger               #EF5968
color.info                 #59A9EF
color.axis.x               #E65A65
color.axis.y               #58C77A
color.axis.z               #5D91E8
color.selection            #4C8DFF
color.selection.hover      #75A7FF
color.viewport.background  #121419
color.grid.major           #464C58
color.grid.minor           #292E37
color.uv.tile              #20242B
color.uv.outside           #15171C
```

## 15. Light theme

Implementar arquitetura para Light Theme; não duplicar componentes, apenas
tokens.

```
canvas        #E9EBEF
app_bar       #F5F6F8
panel         #FFFFFF
raised        #F4F5F7
control       #ECEEF2
hover         #E1E5EB
pressed       #D8DDE5
border        #D2D7DF
text.primary  #1D222B
text.secondary #4C5563
text.muted    #737D8B
accent        #3278E8
```

## 16. Theme options

Preferences → Appearance → Theme: `System`, `Dark`, `Light`, opcionalmente
`High Contrast`. High Contrast deve ser override de tokens, não outra UI.

## 17. Focus

Keyboard focus visível. Focus ring 2 px accent. Não remover o focus indicator.
Tab navigation deve respeitar ordem visual.

## 18. Hit targets

Mesmo quando o ícone tiver 16, 18 ou 20 px, o alvo mínimo deve ficar próximo de
28 × 28 px. Tool buttons 36 × 36 px. Top toolbar 28–32 px.

## 19. Cursores

Pointer para botões. Text para inputs. `col-resize` para splitters verticais.
`row-resize` para splitters horizontais. `grab` para itens arrastáveis;
`grabbing` durante drag. `crosshair` para Knife/Draw/Paint quando adequado.
`eyedropper` para Eyedropper. `move` para move 2D/UV quando adequado.

## 20. Janela global

Mínimo recomendado 1024 × 640. Layouts alvo obrigatórios: 1366 × 768,
1600 × 900, 1920 × 1080, 2560 × 1440. Não deixar o viewport menor que uma
região funcional.

## 21. Global App Bar

Altura 40 px. Padding esquerdo/direito 8 px. Border bottom 1 px.

```
[Petunia]
[File] [Edit] [Workspace Menu] [View] [Window] [Help]
                                  espaço flexível
[MODEL] [PAINT] [UV]
                                  espaço flexível
[Undo] [Redo] [Dirty/Saved] [Open] [Save] [Search] [Settings]
```

## 22. App logo / título

`Petunia` em 13 px semibold. Click não abre comportamento oculto. Opcionalmente
abre app menu apenas se existir um menu formal. Não tornar o logo um botão
misterioso.

## 23. Workspace buttons

Altura 30 px. Largura 86–96 px. Gap 4 px. Active: accent background. Hover:
raised. Pressed: pressed.

Trocar de workspace preserva documento; não recria cena, seleção, material,
texturas, UV nem histórico. Pode mudar layout, tools, inspector e arranjo de
viewport.

## 24. Undo icon

`icon.command.undo` — Lucide `Undo2` / `RotateCcw`. Hit area 30 × 30. Tooltip:
`Undo` / `Ctrl+Z` / segunda linha `Undo the last operation.` Disabled quando o
histórico está vazio.

## 25. Redo icon

`icon.command.redo` — `Redo2` / `RotateCw`. Tooltip: `Redo` / `Ctrl+Shift+Z`.
Disabled quando a pilha de redo está vazia.

## 26. Save state

Estados: `Saved`, `Unsaved`, `Saving…`, `Save failed`. Saved: success/muted.
Unsaved: indicador discreto + `Unsaved`. Saving: spinner pequeno. Failed:
danger. Não usar modal para cada save. Tooltips: `All changes saved.` /
`Project has unsaved changes.`

## 27. Open icon

Icon `FolderOpen`. `Ctrl+O`. Abre native file dialog via `rfd`. Se o documento
tiver mudanças, abrir o modal de Unsaved Changes primeiro.

## 28. Save icon

Icon `Save`. `Ctrl+S`. Quando o arquivo nunca foi salvo, acionar Save As.

## 29. Search icon

Icon `Search`. `Ctrl+F`. Abre Command/Search Palette. Não criar campo
permanente enorme no header.

## 30. Search / Command Palette

Overlay centralizado. Width 520 px. Max height 420 px. Top ~15 % da janela.
Input 36 px. Resultados 32 px cada.

Pesquisa: commands, tools, menu actions. Exemplo: `extr` → `Extrude` /
`Model › Mesh` / `E`.

Selecionar: Arrow Up/Down. Enter executa. Esc fecha. Click outside fecha.

## 31. Settings icon

Icon `Settings`. Click abre Preferences modal. `Ctrl+,` também abre.

## 32. Menu bar — regras

Row height 28 px. Padding 8 px horizontal. Icon column 20 px. Label. Shortcut
alinhado à direita. Separator 1 px com margem vertical 4 px. Submenu arrow
`ChevronRight` 12 px.

Menus fecham com Esc, click fora e seleção de comando. Keyboard: Alt/menu
navigation quando suportado, setas, Enter, Esc. Submenu hover delay ~180 ms.

## 33. File menu

```
New Project            Ctrl+N
Open…                  Ctrl+O
Open Recent >          lista de projetos recentes
────
Save                   Ctrl+S
Save As…               Ctrl+Shift+S
────
Import >
    Model…
    Reference Image…
    Texture…
Export >
    glTF / GLB…
    OBJ…
    Selected…
────
Close Project
Quit                   Ctrl+Q
```

## 34. File → Import

`Model…` aceita OBJ, glTF, GLB (suportados pelo MVP). `Reference Image…` e
`Texture…` aceitam os formatos suportados pelo backend. Nenhum item pode
aparecer se o backend não tiver suporte real, a menos que esteja disabled com
tooltip `Not available in this build.`

## 35. File → Export

`glTF / GLB…`, `OBJ…`, `Selected…`. `Selected` fica disabled sem seleção
exportável.

## 36. Edit menu

```
Undo                   Ctrl+Z
Redo                   Ctrl+Shift+Z
────
Duplicate              Shift+D
Delete                 Delete
────
Select All             Ctrl+A
Deselect All           Alt+A
Invert Selection       Ctrl+I
────
Preferences…           Ctrl+,
```

Itens contextuais devem respeitar `can_execute`.

## 37. Workspace menu

O terceiro menu muda de nome: com MODEL ativo chama-se `Model`; com PAINT,
`Paint`; com UV, `UV`. Isso oferece descoberta completa sem entupir a toolbar.

## 38. View menu

```
Frame Selected         Shift+F
Frame All              Home
────
Perspective / Orthographic  Numpad 5
Viewpoint >
    Front    Numpad 1        Back    Ctrl+Numpad 1
    Right    Numpad 3        Left    Ctrl+Numpad 3
    Top      Numpad 7        Bottom  Ctrl+Numpad 7
────
Shading >   Solid · Material Preview · Wireframe
────
Grid
Overlays
X-Ray
────
Panels >    Outliner · Properties · Asset Library · Status Bar
```

## 39. Window menu

```
Outliner
Properties
Asset Library
────
Reset Current Workspace Layout
Reset All Layouts
────
Fullscreen             F11
```

## 40. Help menu

```
Keyboard Shortcuts
Documentation
Report Issue          (se URL configurada)
────
About Petunia
```

Nunca inserir item de Help morto.

## 41. Tooltip system

Delay inicial 500 ms. Depois que um tooltip aparecer, os próximos podem
aparecer em ~100 ms. Padding 8 px. Max width 320 px. Radius 6 px. Título 12 px
semibold. Descrição 11–12 px. Shortcut 11 px muted.

```
Extrude                         E
Extrude the selected faces.
```

Nunca usar apenas `Extrude` se houver espaço para explicar.

## 42. Button states

Todo button: Default, Hover, Pressed, Selected, Focus, Disabled. Selected ≠
Pressed. Disabled: opacidade reduzida, mas texto legível. Tooltip disabled deve
explicar a razão quando útil, ex.: `Bridge — Select two compatible edge loops to
use Bridge.`

## 43. Icon system

Usar semantic icon IDs. Prioridade: Lucide Slint → Tabler Icons → ícone custom
Petunia. Nunca chamar diretamente vários packs em todos os componentes. Criar a
camada `SemanticIconId → concrete SVG`, ex.: `icon.file.open`, `icon.file.save`,
`icon.tool.select`, `icon.tool.move`, `icon.tool.rotate`, `icon.tool.scale`,
`icon.model.extrude`, `icon.model.inset`, `icon.paint.brush`, `icon.uv.unwrap`.
Isso permite trocar pacote sem refazer a UI.

## 44. Icon visual rules

Toolbar icon 18–20 px. Small inline 14–16 px. Major actions 20 px. Stroke
weights precisam parecer equivalentes. Alinhamento óptico obrigatório. Não
misturar ícone filled pesado com outline fino sem normalização.

## 45. Dropdown component

Altura 28 px. Chevron 12 px. Click abre lista abaixo; se não houver espaço, abre
acima. Min width = largura do control. Max popover width 320 px. Rows 28 px.
Keyboard: Enter, Space, ArrowUp, ArrowDown, Home, End, Esc. Typeahead quando
apropriado. Selected item com checkmark ou accent.

## 46. Popover

Radius 6 px. Padding 6–8 px. Shadow discreta. Border 1 px. Click outside fecha.
Esc fecha. Se pinned, não fecha por click outside.

## 47. Context menu

RMB em item válido. Ações dependem do contexto. Menus não devem incluir ações
inválidas só por completude. Podem aparecer disabled quando isso ajuda
descoberta.

## 48. Modal backdrop

Backdrop escurecido. Modal central. Não permitir interação atrás. Esc fecha
quando cancelável. Enter aciona primary action quando seguro.

## 49. Unsaved Changes modal

420 × ~190 px. Title `Unsaved Changes`. Body `You have unsaved changes in
"{project_name}".` Actions: `Save`, `Discard`, `Cancel`. Save salva e continua a
ação pendente; Discard continua sem salvar; Cancel retorna. Nunca usar apenas
Yes/No.

## 50. New Project modal

~480 × 340 px. Title `New Project`. Fields: `Project Name`, `Template`
(`Empty` / `Cube`), `Units` (`Metric` / `Generic`). Default: `Cube`, `Generic`.
Buttons: `Cancel`, `Create Project`. Criar projeto deve limpar o histórico
antigo.

## 51. Import Model options

Após o file picker, quando opções forem necessárias: ~520 × 420 px. Fields
apenas se o backend suportar: `Scale`, `Up Axis`, `Merge Objects`,
`Import Materials`. Primary `Import`, secondary `Cancel`. Não mostrar opção que
o backend ignora.

## 52. Export modal

~540 × 480 px. Mostrar opções específicas do formato. Common: `Scope` (Scene /
Selected), `Apply transforms` quando suportado, `Materials`, `Textures`. Primary
`Export`, depois native save dialog.

## 53. Create Texture modal

~460 × 360 px. `Name`. `Resolution`: 256, 512, 1024, 2048, `Custom` se
suportado. `Background`: `Transparent`, `White`, `Black`, `Custom`. `Channel`:
`Base Color`, `Alpha`. Buttons `Cancel`, `Create`.

## 54. Preferences modal

~760 × 560 px. Sidebar: `General`, `Appearance`, `Viewport`, `Input`, `Files`,
`Language`.

- General: startup behavior, autosave se implementado.
- Appearance: Theme, UI Scale.
- Viewport: grid, orbit sensitivity, zoom sensitivity.
- Input: navigation preset no futuro; mouse sensitivity.
- Files: default project path; export defaults.
- Language: English, Português (Brasil), pseudo locale em development builds.

Apply instantaneamente quando seguro. Close.

## 55. Color picker

Popover preferido, não modal gigante. ~280 × 360 px. SV square. Hue strip.
Alpha strip quando aplicável. Fields R, G, B, A. HEX. Recent Colors. Eyedropper
action. Enter commit. Esc revert.

## 56. Outliner — propósito

Hierarquia REAL da cena, não decorativa. Deve permitir: selection, rename,
visibility, lock, hierarchy, reorder, duplicate, delete, context menu.

## 57. Outliner dimensions

Default 224 px. Min 180. Max 400. Resizable. Collapsible. Header 32 px.
Search/filter opcionalmente 28 px quando aberto. Tree row 24 px. Indent 16 px
por nível.

## 58. Outliner header

Title `Scene`. À direita: `Search`, `+`, `Collapse All`. Search icon 16 px. Plus
icon abre Add menu.

## 59. Outliner Add menu

Click `+`:

```
Mesh >   Cube · Plane · Cylinder · Cone · Capsule
Reference >   Image
```

Não colocar ferramentas de edição aqui.

## 60. Outliner icons

Scene Root: `Hierarchy` / `Boxes`. Mesh Object: `Box` / `Cube`. Reference Image:
`Image`. Material child: `Circle` / `Material` quando material for representado
como child. Expanded: `ChevronDown`. Collapsed: `ChevronRight`. Visible: `Eye`.
Hidden: `EyeOff`. Unlocked: `LockOpen` quando hover ou se necessário. Locked:
`Lock`. Active object: accent row + active indicator. Selected secondary: accent
soft.

## 61. Outliner row geometry

Height 24 px. Disclosure 16 × 16. Type icon 16 × 16. Gap 4 px. Label flex.
Visibility icon com hit target 20 × 20. Lock 20 × 20. Padding 4–6 px.

## 62. Outliner selection

LMB seleção única. Ctrl+LMB toggle selection. Shift+LMB range quando a
estrutura permitir. A seleção deve atualizar o DOCUMENTO. Não criar
`OutlinerSelection` separada.

## 63. Outliner rename

`F2` ou slow double click no label. Input inline. Enter commit. Esc cancel.
Nome vazio: rejeitar ou restaurar o anterior.

## 64. Outliner double click

Double-click em objeto: select + Frame Selected. Não iniciar rename no
double-click comum se houver conflito; usar `F2` como forma garantida.

## 65. Outliner visibility

Click no Eye alterna visibilidade. Não altera selection. Alt+click
opcionalmente Isolate somente se implementado; não inventar sem suporte.

## 66. Outliner lock

Locked: objeto continua visível. Não pode ser selecionado pelo viewport, nem
transformado, nem editado. Pode ser selecionado pelo Outliner apenas se isso for
necessário para desbloquear, mas o inspector deve indicar `Locked`.

## 67. Outliner context menu

Object: `Rename`, `Duplicate`, `Delete`, separator, `Hide`, `Lock`, separator,
`Frame Selected`.

Mesh object adicionalmente: `Separate`, `Join` quando multi-selection válida.

Reference: `Replace Image`, `Opacity`, `Lock`, `Delete`.

## 68. Splitter system

Visual 1 px. Interaction target 6 px. Hover accent soft. Active accent.
Double-click reset default size. Drag contínuo. Cursor correto. Persistir layout.

## 69. Properties / Inspector

Default 304 px. Min 260. Max 460. Resizable. Collapsible. Header 32 px. Sections
com header de 28 px. Panel padding 8 px. Gap 8 px.

## 70. Inspector section

Header com `Chevron` + `Title`, altura 28 px. Click expande/colapsa. Estado
persistido dentro do workspace quando razoável. Não persistir seção temporária
de Tool se isso causar UX estranha.

## 71. Number field

`PetuniaNumberField`. Height 28 px. Radius 4 px. Padding 6 px.

- Mouse click: text editing.
- Horizontal drag: scrub.
- Shift+drag: fine.
- Ctrl+drag: coarse.
- Double click: select all numeric text.
- Enter: commit.
- Esc: cancel/revert.
- Arrow Up: +step.
- Arrow Down: −step.
- Shift+Arrow: fine step.

Suportar: `min`, `max`, `step`, `precision`, `unit`, `default`, `sensitivity`.

## 72. Vector field

Vec2: U/V ou X/Y. Vec3: X/Y/Z. Axis colors discretas **apenas nos labels**. Não
colorir campos inteiros. Width adaptativa.

## 73. Slider field

Height 28 px. Track 4 px. Thumb 12 px. Numeric display clicável. Drag numeric:
scrub. Click numeric: vira NumberField.

## 74. Checkbox / toggle

Checkbox 16 × 16. Hit target 28 px. Toggle switch somente para flags claramente
on/off de estado contínuo. Não transformar tudo em switch.

## 75. Status bar

Height 24 px. Left: tool/status. Center: interaction hint. Right:
selection/document stats.

```
Extrude
LMB confirm · Esc cancel · Shift precise
24 faces · 48 tris
```

## 76. Asset Library

Bottom panel. Default height 176 px. Min 120. Max 360. Resizable verticalmente.
Collapsible. Tabs: `Assets`, `Materials`, `Textures`, `References`. O workspace
pode escolher a tab default.

## 77. Asset Library header

Height 32 px. Tabs à esquerda. Search. Grid/List toggle. Import.
Close/collapse.

## 78. Asset grid

Thumbnail 64 × 64. Compact 48 × 48. Label em uma linha com ellipsis. Grid gap
8 px. Card padding 4 px. Double-click: ação contextual. Drag inicia asset drag.

## 79. Drag and drop asset

Material → object: assign material. Texture → material slot: assign texture.
Reference image → viewport: create reference. Mesh → viewport: instantiate asset
quando suportado. Mostrar preview de drop target válido. Invalid drop: cursor
proibido.

## 80. Viewport header

Height 30–32 px. Dentro do viewport. Não duplicar a app bar. Conteúdo MODEL:
`Perspective / Orthographic`, `View` dropdown, `Shading`, `Grid`, `Overlays`,
`X-Ray`, camera controls relevantes.

## 81. Viewport navigation

LMB: selection / tool action. MMB drag: Orbit. Shift+MMB: Pan. Wheel: Zoom.
Shift+F: Frame Selected. Home: Frame All. Numpad mappings definidos na seção 38.
Esc: cancel active tool. RMB: context menu quando nenhum gesture conflitante.

## 82. MODEL workspace default layout

Outliner 224 px. Model Tool Rail 44 px. Viewport maior área. Properties 304 px.
Asset Library 176 px, collapsible.

## 83. MODEL Tool Rail

Width 44 px. Outer padding 4 px. Buttons 36 × 36. Gap 2 px. Categorias podem
usar flyout.

## 84. MODEL selection mode strip

No viewport header: `Object`, `Vertex`, `Edge`, `Face`. Ícones preferidos:
Object `BoxSelect` / `Box`; Vertex `Dot`; Edge `Slash` / `Line`; Face `Square`.
Shortcuts quando o viewport não está editando texto: `1` Object, `2` Vertex,
`3` Edge, `4` Face. Tooltip ex.:

```
Vertex Select                       2
Select individual mesh vertices.
```

## 85. MODEL Select tool

Command `model.select`. Icon `MousePointer2`. Shortcut `Q`. LMB seleciona.
Ctrl toggle. Shift add. Drag em espaço vazio: box select. RMB: selection context
menu. A seleção visual muda conforme o mode.

## 86. Object selection visual

Selected object: outline accent. Active object: accent ligeiramente mais forte.
Multi-selected secondary: soft accent. Não preencher todo objeto com azul
sólido.

## 87. Vertex selection

Point radius lógico ~4 px. Hover ~6 px. Selected accent. Active accent strong.
Oclusão respeitada, a menos que X-Ray esteja ativo.

## 88. Edge selection

Normal edge: renderer usual. Hover: highlight de 2 px lógicos. Selected: 2 px
accent. Active: stronger.

## 89. Face selection

Selected face: accent translucent fill + border. Opacidade moderada. Deve
continuar possível enxergar a geometria.

## 90. Move tool

Command `model.move`. Icon `Move3D`. Shortcut `G`. Acesso: Tool Rail, `Model →
Transform → Move`, Search, contextual UI.

Behavior: `G` inicia Move da selection. Mouse move pré-visualiza. `X`/`Y`/`Z`
constrain ao eixo. `Shift+X/Y/Z` constrains ao plano quando suportado. Digit
input: distância numérica. Enter commit. LMB commit. Esc/RMB cancel.

## 91. Move gizmo

X vermelho. Y verde. Z azul. Axis hover: bright. Drag axis: axis constraint.
Plane handles: pequenos quadrados translúcidos. Center: movimento livre. O gizmo
mantém tamanho visual quase constante na tela.

## 92. Rotate tool

Command `model.rotate`. Icon `Rotate3D`. Shortcut `R`. Mouse: preview de ângulo.
`X`/`Y`/`Z`: eixo. Numeric: graus. Enter/LMB commit. Esc cancel. Gizmo: rings
X/Y/Z.

## 93. Scale tool

Command `model.scale`. Icon `Scaling`. Shortcut `S`. Mouse: escala uniforme.
`X`/`Y`/`Z`: eixo. Numeric: `2` = 2×, `0.5` = metade. Enter commit. Esc cancel.

## 94. Transform orientation

Dropdown no viewport header: `Global`, `Local`, `Normal` quando aplicável.
Default `Global`. Tooltip explica a diferença.

## 95. Pivot

Dropdown: `Median`, `Active Element`, `Individual Origins`, `Bounding Box`.
Implementar apenas os modos realmente suportados.

## 96. Snap

Magnet icon. Click toggle. Dropdown arrow: config. Types: `Grid`, `Vertex`,
`Edge`, `Face`. Quando snap ativo: feedback visual no target. Shortcut pode ser
configurável; não criar conflito arbitrário.

## 97. Proportional editing

Icon semântico `circle.falloff`. Toggle. Quando ativo: mouse wheel durante
transform ajusta o radius. Overlay: círculo de influence. Inspector: radius e
falloff se aplicável.

## 98. Add primitive

Tool rail: `Plus` / `Box` category. Click abre popover com `Cube`, `Plane`,
`Cylinder`, `Cone`, `Capsule`. Também em `Model → Add`.

## 99. Create Cube

Click `Cube` cria preview. Tool Properties: `Size X`, `Size Y`, `Size Z`,
default `1 / 1 / 1`, e `Position`. `Apply` commit. `Esc` cancel/remove preview.
Após commit: Outliner atualiza, Cube selecionado, Undo remove o Cube.

## 100. Create Plane

Properties: `Width`, `Height`. `Segments` apenas se o backend suportar.
Orientação conforme work plane.

## 101. Create Cylinder

Properties: `Radius`, `Height`, `Sides`. Default de sides coerente com a
filosofia low-poly, ex. `12`; mínimo `3`. Live preview.

## 102. Create Cone

`Radius`, `Height`, `Sides`. Top radius opcional apenas se o backend já tratar
frustum.

## 103. Create Capsule

`Radius`, `Height`, `Segments`/`Rings` somente se o backend suportar. Não fingir
low-poly customization sem implementação.

## 104. Draw category

Icon `PenTool`. Flyout: `Profile Pen`, `Polygon Pen`, `Polyline`, `Rectangle`,
`Circle`, `Arc`, `Draw on Face`.

## 105. Profile Pen

Propósito: criar perfil fechado ou aberto através de pontos. Icon `PenTool`.
Acesso: Tool rail → Draw, `Model → Draw → Profile Pen`, Search.

Interaction: LMB em work plane vazio adiciona ponto. Mouse move pré-visualiza o
próximo segmento. LMB coloca ponto. Click no ponto inicial fecha o perfil. Enter
finaliza perfil aberto se válido. Backspace remove o último ponto. Esc cancela o
perfil. Double-click finaliza quando coerente. Snap: grid/vertex/edge quando
enabled.

Visual: ponto colocado, ponto em hover, segmento, preview do próximo segmento,
indicador de fechamento.

Quando o perfil é fechado, oferecer ações contextuais `Extrude` e `Revolve` se
válidas.

## 106. Polygon Pen

Semelhante ao Profile Pen, mas orientado à criação explícita de polygon fechado.
Primeiros 2 pontos: segments. A partir de 3: closing preview aparece. Click no
primeiro ponto fecha. Tool properties: `Fill` on/off quando suportado.

## 107. Polyline

Cria sequência aberta. LMB adiciona ponto. Enter/double-click finaliza.
Backspace remove ponto. Esc cancela. Não auto-fechar.

## 108. Rectangle

LMB down primeiro canto. Drag preview. LMB release segundo canto. Properties:
`Width`, `Height`. Alt: centered se implementado. Shift: square constraint se
implementado.

## 109. Circle

LMB define centro. Drag define radius. Second click/release commit. Properties:
`Radius`, `Segments`. Default de segmentos low-poly coerente.

## 110. Arc

Interaction: click 1 centro; click 2 radius/start; click 3 ângulo final. Preview
permanente. Properties: `Radius`, `Start Angle`, `End Angle`, `Segments`. Esc
cancela.

## 111. Draw on Face

Selecionar face válida. Ativar Draw on Face. A face define o work plane. Cursor
projetado na face. Profile/Polygon etc. usam esse plane. Quando o profile
fecha, pode oferecer `Inset/Profile` e `Extrude` conforme implementação
existente.

## 112. Extrude

Command `model.extrude`. Icon semântico de extrude. Shortcut `E`. Válido: face
selection. Click/shortcut inicia preview. Direção default: face normal / média
das normais. Mouse drag: distância. Numeric: distância exata. Tool Properties:
`Distance`, `Direction` (`Normal` / `Axis` quando suportado), `Individual Faces`
(toggle se suportado), `Keep Faces` (quando a semântica exigir). LMB/Enter
commit. Esc/RMB cancel. Topologia real.

## 113. Push / Pull

Icon: seta perpendicular ao plano. Sem shortcut default obrigatório. Acesso:
tool flyout, Model menu, face context menu, Search. Hover face: highlight. LMB
drag move a face ao longo da normal. Numeric: distância. A topologia deve
respeitar a implementação real.

## 114. Inset

Shortcut `I`. Selecionar faces. Ativar. Mouse: amount. Numeric: exato.
Properties: `Amount`, `Individual`/`Region` quando suportado. Preview de
topologia real.

## 115. Bevel / Chamfer

Shortcut `B`, somente quando em Mesh Edit context. Se houver conflito com box
select, o Tool state/context resolve. Properties: `Width`, `Segments`, `Clamp`
quando suportado. Mouse: width. Wheel: segments opcionalmente. Numeric: width.

## 116. Knife / Cut

Shortcut `K`. Cursor crosshair. LMB primeiro ponto de corte. Clicks subsequentes
formam o caminho. Hover edge: snap indicator. Enter commit. Esc cancel. Backspace
remove o último ponto. Não apenas desenhar overlay; o commit altera a topologia.

## 117. Slice

Se Slice for ferramenta separada: definir plane / cut line. Preview mostra a
interseção. Properties: `Keep Both` quando suportado. Commit altera mesh. Se o
backend tratar Slice e Knife como a mesma engine, a UI ainda pode apresentar
workflows adequados.

## 118. Loop Cut

Shortcut `Ctrl+R`. Hover edge: detectar loop válido. Preview: linha do loop.
Wheel: número de cuts quando suportado. LMB confirma o loop. Move: slide
position. Second LMB commit. RMB: center loop quando essa convenção for
implementada; caso contrário Cancel. Não inventar estado impossível.

## 119. Even Loop Cut

Acesso: Loop Cut flyout ou `Model → Mesh → Even Loop Cut`. Mantém spacing
uniforme. Tool properties: `Count`, `Spacing`.

## 120. Subdivide

Sem shortcut default necessário. Acesso: Model menu, context menu, Search. Tool
properties: `Cuts` / `Iterations` quando suportado. Commit altera topologia.

## 121. Mirror / Symmetry

Acesso: tool rail transform/mesh flyout, Model menu, Search. Properties: `Axis
X`, `Axis Y`, `Axis Z`. `Origin`: Object / Selection quando suportado. `Merge
threshold` quando suportado. Preview real.

## 122. Duplicate

`Shift+D`. Em Object mode duplica objeto. Em edit selection duplica componentes
se o backend suportar; caso contrário o command fica disabled nesse contexto.
Após duplicate, a nova entity/selection torna-se ativa. Undo remove o duplicado.

## 123. Delete

`Delete`. Em Object: apaga objetos. Vertex: apaga vértices. Edge: apaga arestas.
Face: apaga faces. Não mostrar modal; Undo é a recuperação. Context menu pode
oferecer `Delete` e, em Edit mode, `Delete Vertices`, `Delete Edges`,
`Delete Faces` quando necessário.

## 124. Dissolve

Não é Delete. Context dependent: `Dissolve Vertex`, `Dissolve Edge`,
`Dissolve Face`, mantendo topologia quando possível. Sem shortcut default
necessário.

## 125. Merge

Command `model.merge`. Shortcut `M` em vertex context. Menu: `At Center`, `At
Active` quando suportado. Se apenas um método existir, executar o método real e
não mostrar menu falso.

## 126. Weld

Acesso: `Model → Mesh → Weld`, context menu, Search. Properties: `Distance` /
`Threshold`. Aplica merge baseado em proximidade. Preview stats `4 vertices will
merge` quando barato de calcular.

## 127. Bridge

Válido: dois loops/edge boundaries compatíveis. Inválido: disabled. Properties:
`Segments`, `Twist` somente quando o backend suportar. Preview. Commit.

## 128. Connect

Conectar elementos selecionados, ex. vertices/edges → topology connection.
`can_execute` precisa validar a seleção.

## 129. Fill

Selecionar boundary válido. Command `model.fill`. Shortcut sugerido `Alt+F`
para evitar conflito com Frame Selected. Cria face(s). Não preencher seleção
inválida.

## 130. Separate

Command `model.separate`. Shortcut `P` em Edit context. Seleciona mesh
components. Cria novo objeto. Outliner atualiza. Novo objeto selecionado. Undo
rejunta o estado original.

## 131. Join

`Ctrl+J`. Requer 2+ mesh objects. Combina em objeto coerente. O active object
pode determinar o target. Outliner atualiza.

## 132. Keep Parts

Implementar exatamente conforme a semântica canônica existente no core. A UI
precisa explicar no tooltip. Não usar nome obscuro sem descrição.

## 133. Fuse

Usar a operação real já desenhada para o Petunia. Não criar um boolean system
enorme apenas para justificar Fuse. Tool Properties conforme backend.

## 134. Flip Normals

Acesso: `Model → Mesh → Normals → Flip`, context menu quando apropriado. Sem
toolbar permanente. Atualiza o rendering imediatamente.

## 135. Flip Diagonal

Válido: face/tri topology apropriada. Troca diagonal real. Tooltip explica.

## 136. Revolve

Input: profile selection. Tool ativa. Properties: `Axis` (X/Y/Z/custom quando
suportado), `Angle` (default 360°), `Segments` (default low-poly friendly).
Preview de volume. Commit com topologia real. Esc cancel.

## 137. Tool Properties

Quando uma ferramenta está ativa, Properties mostra `TOOL` no topo.

```
TOOL — EXTRUDE
Distance        [ 1.000 ]
Direction       [ Normal ▼ ]
Individual      [ ]
[Cancel] [Apply]
```

Apply commit. Cancel cancel. Mouse interaction e fields devem controlar a MESMA
preview transaction.

## 138. Object Properties

Quando um object está selecionado: `Transform`, `Object`, `Geometry`,
`Material`, `Visibility`.

Transform: `Position XYZ`, `Rotation XYZ`, `Scale XYZ`. Editar field:
live/document update com a transaction apropriada.

## 139. Material section

`Material` dropdown. `Add Material`. `Base Color` swatch. `Texture slot`.
`Alpha`. `Assign`. Quando textura aplicada, o viewport atualiza imediatamente.

## 140. Reference Image

Add Reference: Outliner `+`, Asset Library, `File → Import → Reference Image`.
File dialog. Depois cria Reference object. Properties: `Image`, `Opacity`,
`Position`, `Rotation`, `Scale`, `Depth`, `Visibility`, `Lock` quando suportado.
Reference pode aparecer atrás ou à frente conforme depth mode.

## 141. PAINT workspace philosophy

PAINT **não é** MODEL com ícones diferentes. É um workspace dedicado à pintura de
textura. O default layout deve mostrar que o usuário está pintando um
objeto/textura.

## 142. PAINT view modes

No Paint Workspace Header: `3D`, `TEXTURE`, `SPLIT`. Default inicial recomendado
`SPLIT`; depois persistir a última escolha.

## 143. PAINT split layout

Default `3D` 55 %, `Texture Canvas` 45 %. Splitter 1 px visual, 6 px hit.
Double-click reseta 55/45.

## 144. PAINT workspace header

Height 32 px.

```
[Brush ▼] Size [32] Opacity [100%] Hardness [80%] Foreground [■]
[3D] [Texture] [Split] [UV Overlay]
```

Os controles rápidos refletem a ferramenta ativa.

## 145. PAINT tool rail

`Brush`, `Eraser`, `Fill`, `Eyedropper`, `Line`, `Rectangle`. Hard/Soft/Pixel
são Brush modes, não necessariamente três ferramentas ocupando a rail.

## 146. Brush icon

Semantic `paint.brush`. Preferred `Paintbrush`. Shortcut `B`. Tooltip:

```
Brush                              B
Paint onto the active texture layer.
```

## 147. Brush cursor

Quando sobre mesh: círculo projetado na superfície. Mostrar radius e falloff
interno quando relevante. Superfície inválida: cursor reduz opacidade / mostra
disabled cue. Size update imediato.

## 148. Brush start / stroke

LMB down `BeginStroke`. Mouse move sample stroke. Spacing respeitado. LMB up
`CommitStroke`. Esc durante stroke cancela quando suportado; caso contrário
reverte a transaction. Um stroke = um Undo.

## 149. Brush settings

`Size`, `Opacity`, `Hardness`, `Spacing`, `Mode` (`Hard`, `Soft`, `Pixel`).
`Flow` somente se realmente suportado. Não mostrar controle sem backend.

## 150. Brush size shortcuts

`[` diminui, `]` aumenta. `Shift+[` / `Shift+]` ajustam hardness somente se
implementado. Tooltip/documentação deve indicar.

## 151. Pixel brush

Mode `Pixel`. Bordas duras. Pixel-aligned sampling quando pintando 2D. Pixel
grid opcional. Pintura 3D precisa respeitar texel mapping.

## 152. Soft brush

Falloff suave. Hardness controla núcleo/falloff. O preview do cursor deve
mostrar inner/outer radius quando possível.

## 153. Eraser

Icon `Eraser`. Shortcut `E` no Paint workspace. Opera na active layer. Não apaga
outras layers. Respeita opacity/alpha.

## 154. Fill

Icon `PaintBucket`. Shortcut `G` no Paint workspace. Aplica fill ao escopo real.
Não pode acidentalmente limpar a textura inteira se o modo previsto for
contiguous region. Properties quando o backend suportar: `Tolerance`,
`Contiguous`.

## 155. Eyedropper

Icon `Pipette`. Shortcut `I`. Cursor eyedropper. LMB sobre Texture Canvas:
sample texel exibido. LMB sobre 3D: raycast → UV → sample texture. Default
sample: composited visible color. Se o backend suportar, dropdown `Sample`:
`Composite` / `Current Layer`. Após sample, Foreground Color atualiza. A
ferramenta pode voltar à ferramenta anterior após um sample se adotado como
comportamento canônico. Recomendado: `I` pressionado momentaneamente =
Eyedropper temporário; `I` click/tool = persistente.

## 156. Line paint tool

Icon `Line`. Shortcut `L`. LMB start. Move preview. LMB end/commit. Properties:
`Width`, `Opacity`, `Brush Mode`. Esc cancel.

## 157. Rectangle paint tool

Icon `Rectangle`. Sem shortcut default necessário. LMB drag preview rectangle.
Properties: `Filled`, `Outline`, `Width`, `Opacity`. Commit no mouse up ou
Enter, conforme interaction mode.

## 158. Paint target

Inspector topo:

```
TARGET
Object:    Cube
Material:  Material.001
Texture:   base_color.png
Channel:   Base Color
```

Nunca esconder o target atual.

## 159. No paintable material

Empty state: `No paintable material` / `This object needs a material before it
can be painted.` / `[Create Material]`. Button real.

## 160. No texture

`No Base Color texture` / `[Create Texture]` abre o Create Texture modal.

## 161. No UV

`This mesh has no usable UV map.` / `[Unwrap]`. Button executa unwrap real ou
alterna para o UV workspace com a operação orientada, conforme arquitetura
definida. Não apenas mudar de aba silenciosamente.

## 162. Texture Canvas

Editor 2D real. Conteúdo: texture, transparency checker, UV overlay, paint
cursor, zoom, pan, pixel grid opcional. Wheel: zoom centrado no cursor. MMB: pan.
Home: frame full texture quando o Canvas está focado.

## 163. Texture Canvas zoom

Mostrar zoom discretamente: `100 %`. Dropdown opcional: 25, 50, 100, 200, 400,
Fit. Não ocupa espaço central excessivo.

## 164. UV overlay in Paint

Toggle. Mostra edges das UV islands. Opacity configurável via popover. Não
modifica a texture.

## 165. Color section

Foreground color swatch. Click abre Color Picker. Recent Colors. Palette.

## 166. Palette

Swatch 24 × 24. Gap 4 px. Hover: border. Selected: 2 px accent. Click:
foreground. RMB: `Replace with Current Color`, `Copy Hex`, `Delete`. Buttons:
`+`, `Import`, `Export`.

## 167. Channel section

`Base Color`, `Alpha`: somente channels editáveis no MVP. Outros canais: não
mostrar ou mostrar read-only claramente. Nunca fingir edição.

## 168. Layers panel

Layer row 28 px. Columns: visibility, thumbnail/icon, name, opacity indicator.
Selected: accent soft. Active: indicação forte de seleção. Bottom actions: `Add
Layer`, `Duplicate`, `Delete`, `Move Up/Down` quando drag não usado.

## 169. Layer actions

Create, Rename (`F2` / inline), Delete, Duplicate, Visibility, Opacity, Reorder,
Merge Down, Flatten. Undoable.

## 170. Layer drag

Drag da row. Insertion line mostra o target. Não pode dropar em posição
inválida. Auto-scroll a lista perto da borda.

## 171. Layer opacity

0–100 %. Scrubbable. Um drag = um Undo.

## 172. PAINT 3D ↔ 2D sync

Pintar no 3D atualiza o Texture Canvas imediatamente. Pintar no Texture Canvas
atualiza o material 3D imediatamente. Sem Refresh manual. Sem workspace switch.

## 173. PAINT status bar

```
Brush
LMB Paint · [ ] Size · I Eyedropper · Esc Cancel
1024×1024 · Base Color · Layer: Details
```

## 174. UV workspace philosophy

UV sem UV Editor é proibido. Default: 3D + UV Editor split. O UV Editor deve ser
protagonista.

## 175. UV default split

3D 42 %. UV 58 %. Splitter. Persist ratio. Modes: `Split`, `UV Only`,
`3D Only`.

## 176. UV header

Height 32 px.

```
[Vertex] [Edge] [Face] [Island] │ [Move] [Rotate] [Scale] │
[Mark Seam] [Clear Seam] │ [Unwrap] [Pack] │ [Checker] [Sync]
```

## 177. UV mode icons

Vertex `Dot`. Edge `Line`. Face `Square`. Island: ícone semântico de múltiplos
polygons/island. Tooltips completos.

## 178. UV selection shortcuts

Quando o UV Editor está focado: `1` Vertex, `2` Edge, `3` Face, `4` Island. Não
processar quando digitando em text field.

## 179. UV Editor

Render: tile 0–1, área fora, grid, UV vertices, edges, faces, islands, selection,
active element, background texture.

## 180. UV tile

Border 0–1 mais forte. Inside: painel normal. Outside: mais escuro. Major grid
forte. Minor grid suave.

## 181. UV vertex

Normal: small point. Hover: larger. Selected: accent. Active: accent strong.

## 182. UV edge

Normal: muted. Hover: bright. Selected: accent. Seam: estilo dedicado.

## 183. UV face

Selected: accent translucent fill. Island selected: island outline + light fill.

## 184. UV sync selection

Toggle `Sync Selection`. ON: seleção de face no 3D ↔ seleção UV correspondente;
island no UV → faces correspondentes destacadas no mesh. OFF: seleção UV
independente como sub-selection, mas relações de documento preservadas. Não
perder a object selection subjacente.

## 185. UV Move

Shortcut `G`. Cursor 2D. `X` constrain U/X. `Y` constrain V/Y. Numeric: delta.
LMB/Enter commit. Esc cancel.

## 186. UV Rotate

`R`. Mouse: ângulo. Numeric: graus. Pivot respeitado.

## 187. UV Scale

`S`. Mouse: uniforme. `X`/`Y`: eixo. Numeric.

## 188. UV gizmo

Move: setas X/Y. Scale: handles. Rotate: ring. Compacto. Não cobrir a island
inteira.

## 189. UV pivot

Dropdown: `Median`, `Individual`, `Bounding Box`, `Cursor`, somente modos
implementados.

## 190. Mark Seam

Command `uv.mark_seam`. Acesso preferido: toolbar, UV menu, 3D UV context menu,
Search. Requer edges. Commit modifica seam data real. Seams visíveis
imediatamente.

## 191. Clear Seam

`uv.clear_seam`. Remove seam metadata. Undoable.

## 192. Unwrap

Command `uv.unwrap`. Shortcut `U`. Botão pequeno na toolbar, não botão gigante no
Inspector. Properties: `Method` somente métodos realmente implementados;
`Margin`. Scope `Selected Faces` / `Mesh` conforme selection. Executa geração
real de UV. Depois: UV Editor exibe islands. Undo restaura UVs anteriores.

## 193. Pack

Command `uv.pack`. Shortcut `P` no UV workspace. Properties quando suportado:
`Padding`, `Rotate Islands`, `Scale to Fit`. Depois: posições UV mudam.
Undoable.

## 194. Project From View

`UV → Project → From View`. Usa a câmera/view atual. Cria coordenadas UV reais.
Undoable.

## 195. Checker

Toggle. Aplica preview checker apenas para visualização. Não substitui
permanentemente o material. Popover: `Scale`.

## 196. UV background

Dropdown: `Active Texture`, `Checker`, `None`. Opacity slider.

## 197. UV Inspector

Sections: `Selection`, `UV Transform`, `Unwrap`, `Pack`, `Display`,
`Diagnostics`. Contextual.

## 198. UV Transform

Position: `U`, `V`. Rotation. Scale: `U`, `V`. Editar fields atualiza os UVs
selecionados.

## 199. UV selection info

```
1 Island
24 Faces
38 UV Vertices
Bounds: U 0.12 → 0.74   V 0.08 → 0.92
```

Somente mostrar valores calculados de verdade.

## 200. UV diagnostics

Nunca mostrar `Algorithm: X` ou `Texel Density: Auto` se forem apenas strings.
Diagnostics permitidos quando realmente implementados: `Overlap`, `Stretch`,
`Texel Density`. Caso contrário, omitir a seção.

## 201. Workspace menu — MODEL

```
Model
  Select Mode >   Object · Vertex · Edge · Face
  Transform >     Move · Rotate · Scale
  Add >           Cube · Plane · Cylinder · Cone · Capsule
  Draw >          Profile Pen · Polygon Pen · Polyline · Rectangle · Circle · Arc · Draw on Face
  Mesh >          Extrude · Push/Pull · Inset · Bevel · Knife · Slice · Loop Cut · Even Loop Cut
                  · Subdivide · Mirror · Dissolve · Merge · Weld · Bridge · Connect · Fill
                  · Separate · Join · Keep Parts · Fuse
  Normals >       Flip Normals
  Topology >      Flip Diagonal
  Generate >      Revolve
```

## 202. Workspace menu — PAINT

```
Paint
  Tool >       Brush · Eraser · Fill · Eyedropper · Line · Rectangle
  Brush Mode > Hard · Soft · Pixel
  View >       3D · Texture · Split
  Layer >      New Layer · Duplicate Layer · Rename Layer · Delete Layer · Merge Down · Flatten
  Palette >    Import · Export
```

## 203. Workspace menu — UV

```
UV
  Select Mode > Vertex · Edge · Face · Island
  Transform >   Move · Rotate · Scale
  Seams >       Mark Seam · Clear Seam
  Unwrap
  Pack Islands
  Project >     From View
  Display >     Checker · UV Grid · Texture · Seams · Sync Selection
```

## 204. Context menus — MODEL Object

`Frame Selected` │ `Move` `Rotate` `Scale` │ `Duplicate` `Delete` │ `Join`
quando a selection permite │ `Separate` quando o context permite │ `Properties`.

## 205. Face context menu

`Extrude` `Push/Pull` `Inset` `Bevel` │ `Fill` `Delete Face` `Dissolve Face` │
`Mark Seam` quando o UV command é aplicável.

## 206. Edge context menu

`Bevel` `Loop Cut` `Dissolve Edge` `Bridge` `Connect` │ `Mark Seam`
`Clear Seam` │ `Delete Edge`.

## 207. Vertex context menu

`Merge` `Weld` `Connect` `Dissolve Vertex` `Delete Vertex`.

## 208. PAINT context menu

No Texture Canvas: `Pick Color`, `Fill`, `Frame Texture` │ `Show UV Overlay`.
Em Layer: `Rename`, `Duplicate`, `Merge Down`, `Delete`.

## 209. UV context menu

Depende de Vertex/Edge/Face/Island. Common: `Move`, `Rotate`, `Scale`. Edge:
`Mark Seam`, `Clear Seam`. Face: `Unwrap`, `Project From View`. Island: `Pack`,
`Rotate`, `Scale`.

## 210. Text token system

Nenhuma string pública hardcoded em `.slint`. Cada command:

```
command.{id}.label
command.{id}.description
```

Panel: `panel.outliner.title`, `panel.properties.title`, `panel.assets.title`.
Section: `section.transform.title`, `section.material.title`,
`section.layers.title`. Menu: `menu.file.title`, `menu.edit.title`,
`menu.model.title`, `menu.paint.title`, `menu.uv.title`, `menu.view.title`,
`menu.window.title`, `menu.help.title`. Modal: `modal.unsaved.title`,
`modal.unsaved.body`, `modal.unsaved.save`, `modal.unsaved.discard`,
`modal.unsaved.cancel`. Empty state: `empty.paint.no_material.title`,
`empty.paint.no_material.body`. Status: `status.ready`, `status.saving`,
`status.saved`.

## 211. Token generation rule

Toda ferramenta nova deve obrigatoriamente possuir `command.{id}.label`,
`command.{id}.description`, `command.{id}.status_hint`,
`command.{id}.disabled_reason` quando relevante e `accessibility.{id}.label`
quando necessário. Assim não existe ferramenta visual sem texto correspondente.

## 212. i18n

Canonical `en-US`. Também `pt-BR`. Development: pseudo locale. O pseudo locale
deve expandir strings, adicionar marcadores e detectar clipping. Nenhum layout
pode depender da largura exata do inglês.

## 213. Keyboard input priority

Quando um text field está focado, teclas digitáveis **não** ativam ferramentas.
Esc primeiro cancela text edit, depois tool, depois popover/modal conforme
hierarchy. Priority: `Modal` → `Popover` → `Text Input` → `Active Tool` →
`Focused Editor` → `Global Command`.

## 214. Shortcut table — GLOBAL

```
Ctrl+N        New Project
Ctrl+O        Open
Ctrl+S        Save
Ctrl+Shift+S  Save As
Ctrl+Z        Undo
Ctrl+Shift+Z  Redo
Ctrl+Y        Redo alternative
Ctrl+F        Search / Command Palette
Ctrl+,        Preferences
Shift+D       Duplicate
Delete        Delete
Ctrl+A        Select All
Alt+A         Deselect All
Ctrl+I        Invert Selection
Shift+F       Frame Selected
Home          Frame All
F11           Fullscreen
```

## 215. Shortcut table — MODEL

```
Q        Select
G        Move
R        Rotate
S        Scale
1        Object Select
2        Vertex Select
3        Edge Select
4        Face Select
E        Extrude
I        Inset
B        Bevel
K        Knife
Ctrl+R   Loop Cut
M        Merge
P        Separate
Ctrl+J   Join
Alt+F    Fill
```

Tools sem shortcut: acesso via rail, menu, context, Search.

## 216. Shortcut table — PAINT

```
B   Brush
E   Eraser
G   Fill
I   Eyedropper
L   Line
[   Decrease Brush Size
]   Increase Brush Size
```

## 217. Shortcut table — UV

```
1   Vertex
2   Edge
3   Face
4   Island
G   Move
R   Rotate
S   Scale
U   Unwrap
P   Pack
```

Outros: toolbar, menu, Search.

## 218. Toast system

Bottom-right. Width 280–360 px. Duration: success ~3 segundos; error persiste um
pouco mais. Types: `Success`, `Info`, `Warning`, `Error`. Exemplos: `Project
saved.`, `Texture imported.`, `Unable to import model.` com `[Details]`. Não usar
modal para tudo.

## 219. Error behavior

Invalid command: preferir disabled command antes da execução. Runtime failure:
toast. Se detalhes técnicos úteis: `Details`. Nunca falhar em silêncio.

## 220. Empty states

Outliner: `No objects in scene.` / `[Add Object]`. Asset Library: `No assets.` /
`[Import]`. Properties: `Nothing selected.` Paint: `No paintable material.` UV:
`No mesh selected.` Sempre ação clara quando aplicável.

## 221. Responsive — ≥1600

Full default layout. Outliner visível. Inspector visível. Asset Library
disponível.

## 222. Responsive — 1366–1599

Outliner ~190–210. Inspector ~280. Asset Library ~150. Viewport permanece
prioritário.

## 223. Responsive — <1200

Asset Library recolhida por default. Outliner pode colapsar. Inspector pode usar
drawer mode. Tool Rail permanece.

## 224. Panel collapse

Collapse arrow. Collapsed strip não deve ocupar 40 px desnecessários. Ideal
0–24 px de affordance dependendo do componente. Window menu também restaura o
painel.

## 225. Layout persistence

Persistir independentemente por workspace: left panel width, right panel width,
bottom panel height, collapsed states, central split ratios, last view mode.
Estado separado para MODEL, PAINT e UV.

## 226. Reset layout

`Window → Reset Current Workspace Layout`. Reset imediato. Toast: `Workspace
layout reset.` Não precisa modal.

## 227. Material workflow

Selecionar objeto. `Properties → Material`. Sem material: `Add Material`. Com
material: `Base Color`, `Texture slot`. Click no texture slot: asset
picker/popover. Drag de Texture da Asset Library: assign.

## 228. Reference workflow

`File → Import → Reference Image` ou `Outliner + → Reference → Image`. Escolher
arquivo. Reference aparece, selecionada, properties visíveis. Lock recomendado
após posicionar.

## 229. Save data

O save precisa preservar pelo menos: scene graph; object IDs; names; transforms;
mesh geometry; selection-relevant stable data quando apropriado; materials;
textures/references; UVs; seams; paint layers; paint data; channels; references;
workspace metadata quando armazenada no project ou preferences.

## 230. Save/Load acceptance flow

`New. Create Cube. Extrude. Inset. Create Material. Create Texture. Paint
stroke. Create layer. Mark seam. Unwrap. Pack. Save. Close. Open.` Resultado
deve permanecer equivalente.

## 231. Import / export

MVP: `OBJ`, `glTF/GLB`, import de imagem necessário a textures/reference. Não
exibir formato não conectado.

## 232. Performance

UI não deve rebuildar árvores inteiras por hover. Outliner: event-driven.
Inspector: context-driven. Assets: lazy thumbnails. Texture: dirty regions
quando a arquitetura permitir. Renderer: não invalidar tudo por hover de UI
irrelevante.

## 233. Paint performance

Não salvar a texture inteira por mouse move se dirty tiles/regions existirem. O
stroke preview deve ser incremental. O Undo pode usar dirty rectangle, tiles ou
command diff quando a arquitetura permitir.

## 234. UV performance

O UV Editor não deve recalcular unwrap durante pan/zoom. Geometry só é
recalculada quando um command exige. Pan/zoom: presentation transform.

## 235. Accessibility

Keyboard usable. Focus visible. Hit areas adequadas. Tooltips. DPI scaling. Text
expansion. Contrast. Nenhuma funcionalidade apenas por cor. Selected tool:
background + icon/text state. Error: icon + color.

## 236. Mouse outside click

Popover non-pinned: close. Numeric edit: commit. Dropdown: close. Tool
operation: não cancelar apenas porque o mouse saiu da janela, a menos que a
interação tenha essa regra específica.

## 237. Drag threshold

Não iniciar drag ao primeiro pixel. Usar pequeno threshold lógico ~3–4 px. Evita
drag acidental.

## 238. Double click timing

Usar o timing fornecido pela plataforma quando possível. Não hardcode
arbitrariamente.

## 239. Hover timing

Buttons: imediato. Tooltip: 500 ms. Submenu: ~180 ms.

## 240. Scrollbars

Width 8–10 px. Auto-hide visual se o framework permitir, mantendo
discoverability. Thumb com tamanho mínimo adequado. Wheel: scroll expected
container.

## 241. No dead space

O Inspector não deve conter centenas de pixels vazios porque uma section acabou.
Pode usar espaço vazio naturalmente, mas ações principais devem permanecer
próximas ao conteúdo. Não colocar Delete no rodapé isolado após 400 px vazios.

## 242. No giant buttons

Unwrap: não ocupa largura toda como CTA gigante exceto em empty state. Pack:
idem. Tools: toolbar/menu commands compactos.

## 243. No static fake statistics

É proibido escrever `Algorithm: LSCM / ABF++`, `Texel Density: Auto` ou
`Faces: 6` se não vier de estado real. Todos os números derivam do documento.

## 244. Menu discoverability

Toda ferramenta MVP deve ser acessível por pelo menos duas vias. Preferível:
toolbar/rail + workspace menu, e opcionalmente context, shortcut, Search.
Nenhuma função crítica pode existir apenas em menu obscuro.

## 245. Tool activation feedback

Ao ativar uma ferramenta: (1) icon becomes selected; (2) cursor changes if
relevant; (3) Tool Properties updates; (4) status bar updates; (5) viewport
overlay appears when relevant; (6) `can_execute` state is enforced. Só mudar o
botão para azul **não** basta.

## 246. Tool deactivation

Após operação one-shot, retornar para Select ou permanecer na ferramenta, de
acordo com a categoria. Recomendado: transform tools permanecem ativas; brush
permanece ativa; knife permanece ativa até saída explícita após commit, se esse
for o workflow escolhido. One-shot command como Flip Normals executa sem mudar a
active tool persistente.

## 247. One-shot commands

Exemplos: `Flip Normals`, `Flip Diagonal`, `Pack`, `Unwrap`, `Duplicate`,
`Delete`. Não precisam virar persistent tool.

## 248. Continuous tools

`Select`, `Move`, `Rotate`, `Scale`, `Brush`, `Eraser`, `Knife`, `Profile Pen`,
`UV Transform`. Possuem active state.

## 249. Persistent selection

Trocar de workspace não deve limpar a object selection. UV pode derivar
sub-selection. Paint mantém o target object.

## 250. Dirty state

Document mutation seta dirty. UI-only changes — panel width, hover, tool
selection — não devem marcar o documento dirty, a menos que sejam salvos dentro
do projeto por design.

## 251. Project vs preferences state

Project: geometry, materials, paint, UV, reference assets. Preferences: theme,
language, UI scaling, recent files, layout quando escolhido globalmente. Não
misturar sem motivo.

## 252. Icons — top bar

Undo `Undo2`; Redo `Redo2`; Open `FolderOpen`; Save `Save`; Search `Search`;
Settings `Settings`; Plus/Add `Plus`; Visibility `Eye` / `EyeOff`; Lock `Lock` /
`LockOpen`; Close `X`; Collapse `ChevronLeft/Right/Up/Down`; Overflow
`MoreHorizontal`.

## 253. Icons — MODEL

Select `MousePointer2`; Move `Move3D` / `Move`; Rotate `Rotate3D`; Scale
`Scaling`; Add `Plus` / `Box`; Draw `PenTool`; Extrude semantic
custom/extrude; Inset semantic inset; Bevel semantic bevel; Knife `Slice` /
`Scissors` style semantic; Loop Cut semantic loop-cut; Subdivide `Grid3X3` /
semantic; Mirror `FlipHorizontal2`; Merge `Combine`; Weld `Merge`; Bridge
`BetweenHorizontalStart` / semantic; Fill `Square`; Revolve `RotateCw` around
axis semantic. Quando o pack não possuir ícone perfeito: custom SVG alinhado à
linguagem visual.

## 254. Icons — PAINT

Brush `Paintbrush`; Eraser `Eraser`; Fill `PaintBucket`; Eyedropper `Pipette`;
Line `Minus` / `Slash`; Rectangle `RectangleHorizontal`; Palette `Palette`;
Layers `Layers`; Texture `Image`.

## 255. Icons — UV

Select `MousePointer2`; Move `Move`; Rotate `RotateCw`; Scale `Scaling`; Mark
Seam `Scissors` / semantic seam; Unwrap semantic unwrap; Pack `LayoutGrid` /
semantic pack; Checker `Grid2X2`; Sync `RefreshCw` / `Link`; Island `Shapes` /
semantic island.

## 256. Icons — Outliner

Scene `Boxes`; Mesh `Box`; Reference `Image`; Material `CircleDot` / `Palette`
semantic; Chevron `ChevronRight`/`Down`; Visible `Eye`; Hidden `EyeOff`; Locked
`Lock`; Unlocked `LockOpen`.

## 257. Icon color

Default `text secondary`. Hover `text primary`. Selected accent-compatible.
Danger apenas em Delete quando relevante. Não pintar todos os ícones com cores
diferentes; axes são exceção.

## 258. Delete visual language

O ícone de Delete pode ficar danger em hover. Não usar botão vermelho enorme
permanentemente. Context menu: `Delete` em red/danger semantic.

## 259. Tool property Apply/Cancel

Footer pequeno da section. Cancel secondary button. Apply primary. Height 28 px.
Gap 6 px. Keyboard: Enter Apply, Esc Cancel.

## 260. Primary button

Height 28–32 px. Accent fill. Não usar primary button para comandos rotineiros de
toolbar. Primary fica reservado para `Create`, `Import`, `Export`, `Apply`,
`Save modal`.

## 261. Secondary button

Neutral surface. Hover/pressed states.

## 262. Danger button

Usar apenas quando a ação é realmente destrutiva. Delete object não precisa
modal, mas a context action pode estar em danger.

## 263. Modal tab order

Title não recebe focus. Primeiro field. Depois campos. Depois secondary. Depois
primary. Esc cancel.

## 264. Input validation

Invalid numeric: red border / error text. Não crashar. Não silenciar clamp
quando o input é claramente inválido; pode clampar limites mecânicos.

## 265. Tool disabled reasons

Exemplos: Extrude disabled — `Select one or more faces to use Extrude.` Bridge
disabled — `Select two compatible edge loops.` Paint Brush disabled — `Create or
assign a paintable texture first.` Pack disabled — `No UV islands available.`

## 266. MVP does not include

Não ampliar escopo nesta remediação. Pós-MVP: Simple Sweep; Spline / Bézier Pen;
advanced sculpt; full procedural system; node editor; advanced PBR authoring;
complex CAD constraints; video reference; advanced animation.

## 267. Visual QA — MODEL

MODEL precisa mostrar claramente: Outliner, Selection Mode, Tool Rail, Viewport,
Gizmo, Properties, Tool Properties, Asset Library, Status Bar, Transform
controls.

## 268. Visual QA — PAINT

PAINT precisa mostrar claramente: Target, Brush Tool, Brush Cursor, Foreground
Color, Layers, Active Layer, Channel, Texture Canvas, 3D View, `3D / Texture /
Split`, UV Overlay.

## 269. Visual QA — UV

UV precisa mostrar claramente: 3D view, UV Editor, tile 0–1, UV islands,
selection mode, Transform tools, Seams, Unwrap, Pack, Checker, Sync selection.

## 270. Functional QA — MODEL

Criar Cube. Move. Rotate. Scale. Select Vertex. Move vertex. Select Edge. Bevel.
Select Face. Extrude. Inset. Undo. Redo. Save. Reload. Resultado preservado.

## 271. Functional QA — DRAW

Profile Pen. Place points. Close profile. Extrude profile. Undo. Redo. Save.
Reload. Profile-derived geometry permanece.

## 272. Functional QA — PAINT

Select object. Create Material. Create Texture. Paint Brush stroke in 3D.
Confirm stroke appears on Texture Canvas. Paint on Texture Canvas. Confirm 3D
updates. Create Layer. Paint second layer. Toggle visibility. Change opacity.
Undo. Redo. Save. Reload. Painting remains.

## 273. Functional QA — UV

Select mesh. Mark seams. Unwrap. UV islands appear. Select island. Move. Rotate.
Scale. Pack. Checker. Undo. Redo. Save. Reload. UVs remain.

## 274. Cross-workspace QA

MODEL: extrude cube. PAINT: paint cube. UV: edit UV. MODEL: return — geometry
must remain. PAINT: return — texture remains. UV: return — UV remains. Selection
remains coherent.

## 275. Interaction QA

Testar mouse, keyboard, mixed mouse+keyboard, panel resizing, collapse, theme
change, language change, DPI 150 %, resoluções 1366×768, 1920×1080 e 2560×1440.

## 276. No clipping QA

Pseudo locale. 150 % scale. Long labels. Sem text overlap com icons. Sem button
truncando label crítica em silêncio. Ellipsis apenas onde intencional. Tooltip
revela valor/texto completo.

## 277. Definition of Done — UI component

Default state works. Hover works. Pressed works. Focus works. Disabled works.
Keyboard works where applicable. DPI works. Theme works. Localization works. No
hardcoded public string. No duplicated token.

## 278. Definition of Done — Tool

- [ ] command exists
- [ ] menu access exists
- [ ] primary UI access exists
- [ ] icon exists
- [ ] tooltip exists
- [ ] shortcut documented when assigned
- [ ] `can_execute` exists
- [ ] active state works
- [ ] cursor/overlay works
- [ ] preview works when applicable
- [ ] numeric input works when applicable
- [ ] commit works
- [ ] cancel works
- [ ] Esc works
- [ ] Undo works
- [ ] Redo works
- [ ] document changes
- [ ] renderer updates
- [ ] Inspector updates
- [ ] Save persists
- [ ] Load restores
- [ ] no placeholder remains

## 279. Definition of Done — Menu item

Label token; CommandId; shortcut when assigned; enabled state; correct context;
tooltip/description when necessary. Click must cause real action.

## 280. Definition of Done — Icon

Semantic ID; tooltip if icon-only; correct hit target; hover; pressed; selected
if applicable; disabled state; accessible label.

## 281. Definition of Done — Modal

Title; clear purpose; cancel path; keyboard behavior; validation; correct primary
action; does not lose document state; handles errors; returns actual result.

## 282. Definition of Done — Dropdown

Opens; closes; selects; supports keyboard; shows current value; updates
underlying state; persists when relevant; handles disabled entries; does not
overflow window.

## 283. Definition of Done — Panel

Resizes; collapses; restores; persists size; has min/max; does not crush
viewport; scrolls when needed; does not clip controls.

## 284. Implementation order

```
PHASE 1   Audit.
PHASE 2   Command registry. Undo transactions. Selection synchronization.
PHASE 3   Design tokens. Reusable controls.
PHASE 4   Global shell. Menus. Outliner. Properties. Asset Library. Status. Splitters.
PHASE 5   Viewport interaction. Selection. Gizmos. Transforms.
PHASE 6   MODEL tools.
PHASE 7   PAINT architecture. Texture Canvas. Layers. Brush pipeline.
PHASE 8   UV Editor. Selection sync. Seams. Unwrap. Pack.
PHASE 9   Materials. Assets. References.
PHASE 10  Save/Load. Import/Export.
PHASE 11  Theme. i18n. DPI. Accessibility.
PHASE 12  Functional validation. Visual cleanup.
```

## 285. Cost control / avoid wasted work

Não gastar grandes passes de implementação em shadows, micro-animation ou
decorative polish enquanto commands seguem desconectados. Prioridade: `FUNCTION
→ INTERACTION → LAYOUT → VISUAL POLISH`. Não reescrever lógica de core já
correta apenas para satisfazer refactor de UI. Reusar o backend que funciona.

## 286. Test strategy

Após a implementação de cada subsistema major, executar testes direcionados em
vez de repetir a suíte inteira cara. Sugerido: Command/Undo tests após o
trabalho de command; Geometry tests após MODEL; Paint tests após PAINT; UV tests
após UV; Persistence tests após Save/Load. Depois, suíte final de
integração/smoke. Não declarar sucesso porque a aplicação compila.

## 287. Manual smoke test required

Lançar a aplicação. Interagir de verdade com menus, dropdowns, panels,
splitters, viewport, Outliner, Properties, Model tools, Paint, UV. Um screenshot
sozinho não é validação.

## 288. Failure conditions

A implementação FALHA esta tarefa se: um botão visível do MVP não faz nada; um
menu item não faz nada; um shortcut não faz nada; um field não afeta estado
real; Paint não modifica a textura; UV não mostra UVs reais; Unwrap não gera UV;
Outliner não seleciona objetos reais; panels não redimensionam; workspace
switching perde estado; Undo não restaura dados; Save/Load perde estado editado;
uma estatística falsa permanece; um placeholder permanece exposto.

## 289. Report final

Ao concluir, fornecer: implementation summary; files changed; new files;
architecture changes; commands added; commands repaired; MODEL tools (uma linha
por ferramenta com status); PAINT tools (uma linha por ferramenta); UV tools
(uma linha por ferramenta); menus status; Outliner status; Inspector status;
Asset Library status; Splitters status; Modals status; Undo/Redo status;
Save/Load status; Import/Export status; i18n status; themes status;
accessibility status; tests executed list; remaining limitations com
arquivo/função/razão exatos.

## 290. Absolute rule

Não reportar `Implemented` sem evidência. Não classificar `PARTIAL` como
`COMPLETE`. Não esconder comportamento ausente. Não fazer screenshots parecerem
acabados enquanto a funcionalidade está ausente. Não produzir um shell visual
fingindo ser um editor.

## 291. Final user flow

Um usuário novo deve conseguir: lançar o Petunia; entender a tela; criar uma
primitiva; encontrar o objeto no Outliner; selecioná-lo; mover; rotacionar;
escalar; mudar para Face mode; extrudar; inset; bevel; usar Draw/Profile tools;
criar material; criar texture; trocar para Paint; ver claramente que Paint é um
workspace de pintura; pintar diretamente no modelo; ver a textura atualizar;
criar e gerenciar layers; trocar para UV; ver um UV Editor real; marcar seams;
unwrap; mover islands; pack islands; usar checker; voltar para Model; salvar o
projeto; fechar; reabrir; encontrar o mesmo modelo, materiais, pintura e UV;
exportar.

Se esse fluxo não puder ser completado sem abrir o código-fonte, o MVP não está
pronto.

## 292. Final visual test

Esconder as palavras `MODEL`, `PAINT`, `UV` do workspace selector. Inspecionar
cada workspace. Deve continuar imediatamente óbvio qual é modeling, painting e
UV editing. Se as três telas ainda parecerem "o mesmo viewport com botões
diferentes", o redesign FALHOU.

## 293. Final product principle

Petunia3D deve reduzir fricção, não funcionalidade. Simplicidade significa
mostrar a ferramenta certa, no momento certo, com os controles certos, e
esconder complexidade irrelevante. Simplicidade **não** significa: painéis
ausentes; ferramentas ausentes; botões sem ação; um Inspector vazio; um viewport
gigante reutilizado em todo lugar; minimalismo falso.

Implementar o MVP completo como uma aplicação 3D realmente utilizável.
