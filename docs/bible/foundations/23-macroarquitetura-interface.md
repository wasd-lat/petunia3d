# 23 — Macroarquitetura da Interface Petunia3D

<aside>
🧭

Este capítulo registra a **direção adotada para a macroestrutura da UI**. A referência visual principal é o Blender UI Redesign analisado no capítulo 22, mas a organização funcional deve ser Petunia-first e selection/task-centric.

</aside>

# Princípio principal

A interface deve ser organizada em torno de **o que o usuário está tentando fazer agora**, não em torno da taxonomia interna de um DCC generalista.

```
Blender-like architecture:
editor categories → modes → data categories → operation

Petunia architecture:
current workspace → current selection/task → contextual actions
```

# Viewport como protagonista

O viewport ocupa o máximo de área útil. Painéis devem parecer elementos auxiliares sobre/ao redor do canvas, e não paredes permanentes que fragmentam a tela.

Direção:

- canvas central contínuo;
- painéis laterais semi-flutuantes/retráteis;
- toolbar sobre o viewport;
- asset library retrátil;
- nada equivalente a Timeline no workspace Model V1;
- gutters claros entre canvas e painéis quando expandidos.

# Workspaces

Baseline visual/funcional inicial:

```
[ MODEL ] [ PAINT ] [ UV ]
```

`Animate` pode surgir futuramente quando esse módulo existir.

Não criar workspace `Layout`. O workspace **Model já é o layout de criação principal**. Não replicar Shading/Compositing/Scripting do Blender porque não correspondem ao escopo do produto.

Os workspaces devem usar o princípio de pills compactas observado no Figma: troca rápida, baixa altura, estado ativo inequívoco.

# Estrutura conceitual da tela Model

```
┌──────────────────────────────────────────────────────────────┐
│ Application chrome / menus / project                        │
├──────────────────────────────────────────────────────────────┤
│                [ MODEL ] [ PAINT ] [ UV ]                   │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│ ╭──────────╮                      ╭─────────────────────────╮ │
│ │ CREATE   │                      │ INSPECTOR               │ │
│ │ TOOLS    │       VIEWPORT       │ Parts                   │ │
│ ╰──────────╯                      │ Transform               │ │
│                                   │ Material                │ │
│                                   │ Object                  │ │
│                                   ╰─────────────────────────╯ │
│                                                              │
│                           VIEW/OVERLAY CONTROLS              │
│                                                              │
│ ╭──────────────── ASSET LIBRARY ──────────────────────────╮  │
│ ╰──────────────────────────────────────────────────────────╯  │
└──────────────────────────────────────────────────────────────┘
```

Esta composição MODEL segue a revisão de baseline de 2026-09-23 do capítulo 36.
Em largura compacta, o Inspector pode ser substituído por drawer à direita.

# PARTS em vez de Outliner

**Parts** é preferível a `Outliner` para o produto principal. No MODEL, ele é a
primeira seção recolhível do Inspector direito, não um painel sobre a barra de
criação esquerda (capítulo 36, revisão de 2026-09-23).

Objetivo: mostrar estrutura do asset, não estrutura de uma cena de render.

Exemplo:

```
PARTS
▼ Character
   Head
   Body
   Arm.L
   Arm.R
   Leg.L
   Leg.R
```

Controles permanentes devem ser mínimos. Visibility e Lock são candidatos fortes. Não replicar Object Data, Render Visibility, Scene Collection e demais conceitos de Blender sem necessidade de produto.

# CONTEXT em vez de Properties categórico

O painel direito é **selection-centric**.

Selecionou uma primitiva:

```
Cylinder
Radius
Height
Sides
```

Selecionou uma face:

```
Face
Material
Shading
Sharp
```

Selecionou uma referência:

```
Reference
Image
Opacity
Scale
Lock
```

Durante Extrude:

```
Extrude
Depth
Direction
```

O usuário não precisa primeiro decidir em qual categoria global de Properties entrar. **Um painel, um contexto atual.**

# Toolbar contextual dentro do viewport

A toolbar é overlay/floating, preservando a área estrutural do canvas. Ela não deve tentar mostrar todas as ferramentas ao mesmo tempo.

Exemplos de contexto:

## Nenhuma seleção

```
Select
Draw
Primitive
Reference
```

## Face

```
Push
Draw
Inset
Cut
Round
```

## Edge

```
Move
Round
Split
Dissolve
```

## Point

```
Move
Weld
Delete
```

O conjunto exato será validado pelo fluxo de uso; o princípio normativo é **contextualidade antes de volume de botões**.

# Seleção Object / Face / Edge / Point

Evitar o ritual `Object Mode → Edit Mode → Vertex/Edge/Face`. Petunia deve expor seleção em linguagem direta, potencialmente como segmented control:

```
[ Object | Face | Edge | Point ]
```

A apresentação final pode ser refinada, mas não exigir conhecimento prévio do conceito de Edit Mode.

# Viewport control bar

Adotar a decomposição conceitual observada na referência:

```
LEFT    → o que estou editando / ações de contexto
CENTER  → transform/snap/pivot/orientation quando relevantes
RIGHT   → como estou vendo o asset
```

O conteúdo deve ser reduzido ao Petunia. Não copiar menus redundantes apenas para preservar aparência do Blender.

# Modos de visualização

O controle compacto deve representar os quatro modos do produto:

```
Wireframe | Solid | Textured | Silhouette/Reference
```

`Rendered` não entra porque Petunia não é um renderizador. Configurações adicionais devem ir para dropdown/overlay, não criar novos modos principais.

# Overlays

Overlays são combinações sobre um modo base:

- wire overlay;
- triangulation;
- face orientation;
- selection highlights;
- reference/x-ray;
- UV/checker quando contextual.

A UI deve evitar transformar cada overlay em um novo “mode”.

# Semi-floating panels

Direção adotada:

- painéis separados visualmente do canvas por gutter pequeno;
- radius discreto;
- header consistente;
- recolhimento explícito por affordance pequena e previsível;
- estado recolhido não deve destruir contexto nem exigir reconfiguração;
- resizing deve ser possível onde trouxer valor real, sem criar docking system estilo Blender completo na V1.

# Asset Library

A Asset Library permanece uma região retrátil com:

- thumbnails;
- busca/filtro;
- controle de tamanho das thumbs;
- acesso a assets do projeto e formas salvas;
- prioridade para reaproveitamento de shapes/parts.

A região deve poder desaparecer para devolver área ao viewport.

# Home

A home do Petunia deve ser simples e orientada à continuidade:

```
New Model / New Project
Open Project
Recover Session
Recent Projects
Settings
```

Não transformar a home em portal de notícias, marketplace ou dashboard complexo no baseline.

# Densidade progressiva

Ferramentas essenciais aparecem primeiro; opções avançadas vivem em dropdown, Context ou Advanced. O usuário deve poder criar um primeiro asset sem conhecer topologia, UV internals ou dezenas de toggles.

# Regra final

**A interface deve parecer um modelador profissional simplificado, não um aplicativo infantil e não um Blender amputado.** A redução vem de menos conceitos simultâneos, não de esconder capacidade arbitrariamente.
