# Petunia3D — PAINT + UV Workspace Redesign

> Este bloco **substitui qualquer implementação atual de PAINT e UV**. A
> implementação atual dos dois workspaces deve ser considerada **visual e
> funcionalmente reprovada**. Não tentar apenas melhorá-la: **redesenhar**.

O problema fundamental atual: `MODEL`, `PAINT` e `UV` usam praticamente o mesmo
layout, o mesmo viewport central e apenas trocam alguns botões. Isso está errado.
Os três compartilham o MESMO DOCUMENTO, mas **não** precisam compartilhar o mesmo
arranjo visual. MODEL é geometry-first. PAINT é surface/texture-first. UV é
UV-layout-first. Essa diferença precisa ser visualmente óbvia.

## 1. Princípio de arquitetura

Compartilhar: documento, scene graph, seleção, renderer, undo/redo, command
registry, assets, materiais, preferências, componentes visuais fundamentais.

Não compartilhar obrigatoriamente: composição central, quantidade de views,
barras contextuais, organização do Inspector, ferramentas, layout default.

## 2. Não preservar a implementação atual

A captura atual (~1919 × 1052 px) apresenta ~40 px de header, ~44 px de tool
rail, ~300 px de inspector direito e ~1500 px dedicados ao MESMO viewport 3D.
Isso deve ser corrigido. Não reutilizar o viewport 3D gigante para ocupar
praticamente toda a tela dos três workspaces.

## 3. Shell global

Consistente entre MODEL / PAINT / UV:

| Região | Medida | Resizable | Collapsible |
|---|---|---|---|
| Top App Bar | 40 px | — | — |
| Outliner / Scene Panel | 220–240 px | sim | sim |
| Status Bar | 24 px | — | — |
| Asset Tray | 160–180 px | sim | sim |
| Context Inspector | 300–320 px | sim | sim |
| Tool Rail | 44 px | — | — |

A **área central muda** de acordo com o workspace.

## 4. PAINT — nova filosofia

PAINT deve parecer uma ferramenta de pintura de textura, não um modelador com
quatro botões de pintura. O usuário deve imediatamente compreender: "estou
pintando a superfície deste objeto".

## 5. PAINT — modos de visualização

Três modos centrais no header interno do workspace: `[ 3D ] [ TEXTURE ] [ SPLIT ]`.

- **3D** — viewport 3D ocupa todo o centro. Ideal para pintar diretamente no objeto.
- **TEXTURE** — texture canvas 2D ocupa todo o centro. Ideal para editar a textura.
- **SPLIT** — viewport 3D + texture canvas. Default recomendado 55 % 3D / 45 %
  texture. Splitter vertical com 6 px de hit target e 1 px visual,
  redimensionável. Double-click reseta 55/45. Persistir tamanho.

## 6. PAINT — header contextual

Logo abaixo do header global. Altura 32 px. Controles de uso contínuo:

```
[Brush ▼] Size [32] Opacity [100%] Hardness [80%] Spacing [10%]
● Foreground
[3D] [Texture] [Split]
[UV Overlay] [Symmetry]
```

Não obrigar o usuário a viajar até o Inspector para mudar Size constantemente.
Esses controles são **tool controls rápidos**; o Inspector contém a configuração
completa.

## 7. PAINT — tool rail

44 px. Buttons 36 × 36 px. Ferramentas MVP: `Brush`, `Eraser`, `Fill`,
`Eyedropper`, `Line`, `Rectangle`.

Variantes de brush **não** precisam ocupar vários ícones: `Brush → preset / mode`
(`Hard`, `Soft`, `Pixel`) reduz visual clutter.

## 8. PAINT — cursor

O pincel precisa existir VISUALMENTE. Quando o mouse estiver sobre a superfície:
mostrar o círculo do brush projetado, com radius, falloff quando aplicável, cor
discretamente e hit válido/inválido. Ao alterar Size, o círculo muda
imediatamente. Ao pintar, o stroke acompanha o mouse em tempo real.

## 9. PAINT — Texture Canvas

Editor 2D real. Deve apresentar: textura ativa, UV wireframe, checker de
transparência, zoom, pan, pixel grid opcional, brush cursor, stroke preview.

Zoom: mouse wheel. Pan: MMB. Frame texture: atalho adequado. O canvas não deve
ser uma imagem estática; é uma superfície editável.

## 10. PAINT — UV overlay

Toggle `UV Overlay`. No texture canvas: mostrar edges das UV islands.
Configuração: opacity, color, visibility. Opcionalmente: island bounds. O UV
overlay **não** deve interferir na pintura.

## 11. PAINT — Inspector

O painel atual (`Color Palette`, `Brush Settings`, `Texture Layers`) é
insuficiente. Reestruturar em seções: `TOOL`, `COLOR`, `CHANNEL`, `LAYERS`,
`MATERIAL`, `TEXTURE`. Somente seções relevantes ficam expandidas.

## 12. TOOL

Brush: `Size`, `Opacity`, `Hardness`, `Spacing`, `Flow` quando suportado,
`Falloff`, `Pixel Mode` quando aplicável.

Eraser: `Size`, `Opacity`, `Hardness`.

Fill: `Tolerance` quando aplicável, `Contiguous` quando aplicável.

Line: `Width`, `Opacity`.

Rectangle: `Filled`, `Outline`, `Width`.

## 13. COLOR

Mostrar: foreground color, background color quando necessário, palette, recent
colors, eyedropper, color field. Clicar na cor abre picker real. O picker deve
oferecer pelo menos SV area, Hue, RGB, HEX e Alpha quando aplicável.

## 14. PALETTE

A fileira atual de quadradinhos não constitui um gerenciador adequado. Criar
`Palette` component. Swatches 24 × 24 px, gap 4 px. Adicionar `+`, `Import`,
`Export`, `Recent`. Right click: `Replace`, `Delete`, `Copy Hex`.

## 15. CHANNEL

Adicionar seletor explícito `CHANNEL`: `Base Color`, `Alpha`. Outros canais podem
aparecer read-only quando já suportados. Não fingir que estão implementados. O
canal ativo deve ficar visualmente evidente.

## 16. LAYERS — obrigatório

A linha atual `Albedo (Base Color) Active` não é um sistema de layers. Criar
painel de Layers real.

```
LAYERS
──────────────────────
👁  Paint Layer 2     100%
👁  Details            75%
👁  Base              100%
[+] [Folder] [Duplicate] [Delete]
```

Cada row 28 px. Suportar: select, rename, create, duplicate, delete, visibility,
opacity, reorder, merge down, flatten quando válido. Drag and drop para reorder.
Layer ativa claramente destacada.

## 17. Layer opacity

Quando uma layer está selecionada: `Opacity`, `Blend Mode` quando suportado. Não
inventar dezenas de blend modes se o backend não possui.

## 18. PAINT — Object / Material target

O usuário precisa saber O QUE está pintando. No topo do Inspector:

```
TARGET
Object:    Cube
Material:  Material.001
Texture:   BaseColor.png
Channel:   Base Color
```

Nunca deixar ambiguamente uma cor solta no painel.

## 19. PAINT — empty states

- Objeto sem material: `No paintable material.` / `[Create Material]`.
- Material sem texture: `No Base Color texture.` / `[Create Texture]`.
- UV inexistente: `This mesh has no usable UV map.` / `[Unwrap]`.

Essas ações precisam executar comandos reais.

## 20. PAINT — Create Texture

Dialog compacto: `Name`; `Resolution` 256 / 512 / 1024 / 2048; `Background`
`Transparent` / `White` / `Custom`. Não abrir wizard gigante.

## 21. PAINT — 3D/2D sincronização

Stroke realizado em 3D deve aparecer imediatamente no Texture Canvas. Stroke
feito no Texture Canvas deve aparecer imediatamente no objeto 3D. Sem botão
Refresh. Sem reconstruir manualmente.

## 22. PAINT — status bar

Quando Brush ativo:

```
Brush
LMB Paint · Shift precision · [ / ] Size · Esc Cancel
Texture 1024×1024 · Base Color · Layer: Details
```

## 23. PAINT — resultado esperado

Ao entrar em PAINT, o usuário deve ver imediatamente uma interface de pintura.
Não: o mesmo viewport MODEL com quatro ícones.

## 24. UV — reprojetar completamente

Um workspace de UV sem UV Editor visível **não é aceitável**. `Unwrap Mesh` +
`Pack Islands` no Inspector **não** constituem um workspace de UV.

## 25. UV — layout central default

Abrir em SPLIT VIEW. Default 42 % 3D / 58 % UV Editor. Splitter visual 1 px, hit
area 6 px, redimensionável. Persistir posição. Top local toolbar permite
`[ SPLIT ] [ UV ONLY ] [ 3D ONLY ]`. SPLIT é o default.

## 26. Viewport 3D UV

O lado 3D deve mostrar: mesh, seleção, seams, selected faces, checker texture,
UV distortion quando o modo estiver habilitado. Não precisa possuir toda a
toolbar de MODEL; é um viewport contextual UV.

## 27. UV Editor 2D

Criar editor UV real. Representar claramente: UV tile 0–1, grid principal, grid
secundário opcional, UV islands, vertices, edges, faces, selection, active
element, background texture.

## 28. UV tile

O tile 0–1 deve possuir limite visual forte. Área fora do tile: mais escura.
Grid: subdivisions discretas. Não desenhar grid infinita com o mesmo peso.

## 29. UV — toolbar superior

Header interno 32 px. Estrutura sugerida:

```
[Vertex] [Edge] [Face] [Island] │ [Move] [Rotate] [Scale] │
[Mark Seam] [Clear Seam] │ [Unwrap] [Pack] │ [Checker] [Sync Selection] [•••]
```

Não colocar todas as configurações no Inspector.

## 30. UV — left tool rail

44 px. Ferramentas principais: `Select`, `Move`, `Rotate`, `Scale`, `Seam Tool`,
`Island Select`, `Project`. Opcionalmente `Measure` / density quando
implementado. Não colocar três ícones misteriosos sem tooltip.

## 31. UV selection

Implementar `Vertex`, `Edge`, `Face`, `Island`. A seleção 2D precisa alterar a
seleção correspondente no documento. Com `SYNC SELECTION = ON`: selecionar face
no 3D seleciona o UV correspondente; selecionar island no UV destaca as faces
correspondentes no 3D.

## 32. UV visual states

UV normal: linha discreta. Selected: accent. Active: accent forte. Pinned quando
existir: estado diferente. Seam: cor específica. Overlapping: warning.

## 33. UV gizmo

No UV Editor: `Move`, `Rotate`, `Scale`. Gizmo 2D simples. Move: X/Y. Rotate:
handle circular. Scale: X/Y/uniform. Numeric input disponível.

## 34. UV numeric transform

Inspector:

```
UV TRANSFORM
Position   U [0.000]   V [0.000]
Rotation   [0.0°]
Scale      U [1.000]   V [1.000]
Pivot:     Median · Individual · Cursor · Bounding Box   (quando suportado)
```

## 35. Seams

`Mark Seam`. `Clear Seam`. A seam precisa aparecer no viewport 3D e no UV editor
quando relevante. Selecionar edges no 3D e Mark Seam deve alterar seam data
real, não apenas desenhar linha diferente.

## 36. Unwrap

Unwrap não deve ser um botão azul gigante ocupando todo o Inspector. É uma
OPERAÇÃO e pode aparecer em toolbar, menu, contextual action e command palette.
O Inspector fica responsável pelos parâmetros:

```
UNWRAP SETTINGS
Method   Angle Based / Conformal   (quando suportado)
Margin   [0.02]
[ Unwrap ]
```

Se a implementação usa xatlas, utilizar o pipeline real. Não exibir `LSCM /
ABF++ Conformal` se isso for apenas texto decorativo. Mostrar somente o
algoritmo realmente utilizado.

## 37. Pack Islands

Pack: `Padding`, `Rotate Islands`, `Scale to Fit` quando suportado. Preview
quando a arquitetura permitir. Executar operação real.

## 38. Project From View

Adicionar ao conjunto MVP previsto. Operation `Project From View`. Quando
executado, usar a câmera/view atual.

## 39. Checker

Toggle `Checker`. Quando ativo, aplicar textura checker temporária no viewport
3D. Opções: `Scale`. Permite observar distortion. Não modificar permanentemente
o material do usuário.

## 40. UV background

O UV Editor pode apresentar `Active Texture`, `Checker`, `None`. Dropdown.
Background opacity 0–100 %.

## 41. UV Inspector

O atual (`UV Operations`, `UV Statistics`) é insuficiente. Novo Inspector
contextual: `SELECTION`, `UV TRANSFORM`, `UNWRAP`, `PACK`, `DISPLAY`,
`DIAGNOSTICS`. Mostrar somente as seções necessárias.

## 42. UV selection info

```
SELECTION
1 Island
24 Faces
38 UV Vertices
Bounds: U 0.12 → 0.74   V 0.08 → 0.92
```

## 43. UV display

Toggles conforme suporte real: `Grid`, `Texture`, `Checker`, `Seams`, `Faces`,
`Stretch`, `Overlap`.

## 44. UV diagnostics

Somente quando implementado: `Overlap`, `Stretch`, `Texel Density`. Não exibir
estatística inventada. A captura atual mostra `Algorithm: LSCM / ABF++ Conformal`,
`Target: Cube`, `Texel Density: Auto` — não apresentar esse tipo de informação se
o backend não estiver realmente calculando esses dados. UI não pode fingir
sofisticação.

## 45. UV context actions

Face selection: `Unwrap`, `Project From View`, `Mark Boundary Seams`. Island
selection: `Pack`, `Rotate`, `Scale`, `Straighten` quando suportado. Edge
selection: `Mark Seam`, `Clear Seam`.

## 46. UV status bar

```
UV Face Select
LMB Select · Shift Add · G Move · R Rotate · S Scale
Island: 1/6 · 24 Faces
```

## 47. Outliner nos workspaces

O Outliner continua acessível em MODEL, PAINT e UV, mas pode assumir
dimensões/default diferentes. PAINT e UV podem iniciar recolhido ou estreito; o
usuário pode expandi-lo. Não remover a scene hierarchy completamente só porque
mudou de workspace.

## 48. Inspector é contextual

Não criar três enormes formulários estáticos. Em PAINT: Brush selecionado mostra
Brush; Layer selecionada mostra Layer; Material selecionado mostra Material. Em
UV: Island selecionada mostra Island Transform; Unwrap ativo mostra Unwrap
Settings; nada selecionado mostra informações gerais úteis.

## 49. Asset Library

PAINT: default tab pode ser `Textures`. UV: Asset Library pode iniciar recolhida.
MODEL: `Meshes` / `Materials`. Mesma Asset Library, contexto diferente.

## 50. Layouts default por workspace

Cada workspace precisa salvar seu layout independentemente. MODEL: viewport
maximizado. PAINT: 3D ou Split conforme última configuração. UV: Split como
default. Guardar panel sizes, collapsed states, split ratio e view mode.

## 51. Workspace switching

`MODEL → PAINT` não reconstrói documento. `PAINT → UV` não perde seleção.
`UV → MODEL` não perde material/textura. Apenas reorganizar apresentação.

## 52. Ferramentas não devem apenas acender

Incorreto: clicar Brush → ícone azul. Correto: clicar Brush → Command ativa
`PaintBrushTool` → cursor muda → brush toolbar aparece → Inspector muda →
raycast fica ativo → cursor circular aparece sobre a mesh → mouse down inicia
stroke → drag atualiza textura → viewport atualiza → texture canvas atualiza →
mouse up cria Undo transaction.

## 53. Unwrap não é "botão"

Fluxo correto: selecionar mesh/faces → selecionar seams se necessário → Unwrap →
backend gera UV → UV Editor atualiza → islands aparecem → seleção continua
coerente → Undo disponível.

## 54. Pack não é "botão"

Fluxo: selecionar islands → Pack → calcular layout → atualizar coordenadas →
mostrar resultado → Undo.

## 55. Componentes Slint reutilizáveis

Criar componentes equivalentes a: `WorkspaceHeader`, `ToolRail`, `ToolButton`,
`ToolPopover`, `ResizablePanel`, `Splitter`, `ContextInspector`,
`InspectorSection`, `NumberField`, `VectorField`, `SliderField`, `ToggleField`,
`DropdownField`, `ColorSwatch`, `ColorPalette`, `ColorPicker`, `LayerList`,
`LayerRow`, `TextureCanvas`, `UVEditor`, `UVToolbar`, `UVGrid`,
`UVIslandRenderer`, `ViewportHeader`, `EmptyState`, `StatusBar`, `AssetBrowser`.

Não duplicar implementações de controles entre workspaces.

## 56. Pixel spec

| Elemento | Valor |
|---|---|
| App Header | 40 px |
| Workspace contextual header | 32 px |
| Tool Rail | 44 px |
| Tool button | 36 × 36 px |
| Tool icon | 18–20 px |
| Inspector default / min / max | 304 / 260 / 440 px |
| Outliner default | 224 px |
| Section Header | 28 px |
| Inspector field | 28 px |
| Layer row | 28 px |
| Status bar | 24 px |
| Asset tray default | 176 px |
| Splitter | 1 px visual, 6 px interaction target |
| Panel padding | 8 px |
| Section gap | 8 px |
| Control gap | 6 px |
| Small gap | 4 px |

## 57. Resolução 1920×1080 / captura atual

PAINT SPLIT: Outliner ~220 px; Tool Rail 44 px; Paint 3D ~650–720 px; Texture
Canvas ~550–650 px; Inspector 304 px. Não são valores absolutos; splitters
precisam permitir adaptação.

UV: Outliner 180–220 px ou collapsed; Tool Rail 44 px; 3D ~550–650 px; UV Editor
~700–800 px; Inspector 304 px. Priorizar o UV Editor no workspace UV.

## 58. Não fazer mais isto

Não: usar o mesmo viewport gigante nos três workspaces; chamar um painel com um
Layer de "layer system"; chamar dois botões de "UV workspace"; usar texto
estático para fingir estatísticas; mostrar algoritmo que não está sendo
executado; deixar 70 % do Inspector vazio; criar botões enormes de Unwrap;
mostrar ferramentas sem estados; usar tool rail sem tooltips; esconder
propriedades essenciais; criar controles desconectados do backend; implementar
brush que apenas muda cursor; implementar UV editor que apenas desenha
wireframe; recriar o documento ao trocar workspace.

## 59. Acceptance test visual — PAINT

Identificar imediatamente: objeto alvo; material alvo; texture alvo; channel
ativo; ferramenta ativa; brush cursor; cor ativa; layers; layer selecionada;
parâmetros da ferramenta; Texture Canvas; UV overlay; modo 3D / Texture / Split.

Funcionalmente: stroke aparece; Texture Canvas sincroniza; 3D sincroniza; layers
funcionam; undo funciona; redo funciona; save/load preserva pintura.

## 60. Acceptance test visual — UV

Identificar imediatamente: viewport 3D; UV Editor; tile 0–1; UV islands; seleção
UV; selection mode; transform tools; seams; Unwrap; Pack; checker; parâmetros
contextuais.

Funcionalmente: seleção 3D ↔ UV sincroniza; Mark Seam altera mesh; Clear Seam
funciona; Unwrap produz UV real; Pack modifica islands; Move funciona; Rotate
funciona; Scale funciona; Undo funciona; Redo funciona; Save/Load preserva UVs.

## 61. Regra final

Perguntar: "Se eu esconder o botão MODEL/PAINT/UV do topo, é imediatamente óbvio
pelo restante da interface em qual workspace estou?" Se a resposta for NÃO, o
redesign falhou. PAINT precisa parecer PAINT. UV precisa parecer UV. MODEL
precisa parecer MODEL. Pertencem ao mesmo Petunia, mas **não** são três cópias do
mesmo viewport com botões diferentes.
