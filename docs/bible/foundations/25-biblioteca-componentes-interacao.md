# 25 — Biblioteca de Componentes e Contratos de Interação

<aside>
🧩

Esta página define a biblioteca conceitual de UI que o framework deve gerar/implementar antes de criar telas isoladas. O objetivo é evitar o problema já experimentado no projeto: muitas telas “parecidas” com a referência, mas sem consistência de comportamento e acabamento.

</aside>

# Regra de implementação

**Não desenhar cada tela manualmente com estilos locais.** Primeiro construir primitives e components Petunia; depois compor as telas.

# Primitives mínimas

```
Surface
Divider
Text
Icon
FocusRing
Spacer
ScrollRegion
PopupSurface
```

Primitives não devem vazar diretamente por todo o produto quando um componente semântico já existir.

# Componentes básicos

```
Button
IconButton
ToggleButton
SplitButton
SegmentedControl
Dropdown
ContextMenu
TextField
SearchField
NumberField
Slider
Checkbox
Radio/choice
Tooltip
Popover
Dialog
Toast/InlineMessage
```

A escolha do toolkit final pode fornecer muitos deles, mas Petunia deve envolvê-los em estilo/tokens próprios quando necessário.

# Componentes estruturais

```
PetuniaPanel
PanelHeader
CollapsiblePanel
WorkspaceSwitch
ViewportToolbar
ViewportBar
PartsTree
ContextPanel
AssetLibrary
StatusStrip
PropertyRow
SectionHeader
```

# PetuniaPanel

Responsabilidades:

- superfície e border consistentes;
- header padronizado;
- título acessível;
- collapse/reopen;
- sizing mínimo/máximo quando redimensionável;
- integração com keyboard focus;
- nenhuma lógica de domínio específica.

`Parts` (seção do Inspector MODEL desde a revisão de 2026-09-23 do cap. 36),
`Inspector` e `Asset Library` reutilizam o mesmo contrato estrutural onde fizer sentido.

# PanelHeader

Deve resolver de forma única:

- título;
- ações do painel;
- affordance de collapse;
- drag/resize apenas se aprovado pelo modelo de layout final;
- focus/accessible label.

Não recriar headers diferentes para cada painel.

# WorkspaceSwitch

Apresenta poucos contextos principais e mantém um selecionado.

Baseline:

```
MODEL
PAINT
UV
```

Animation entra apenas quando funcionalmente disponível. O `+` do Blender não é requisito do Petunia.

# ViewportToolbar

Toolbar vertical/flutuante. Recebe comandos disponíveis do contexto atual, não conhece diretamente o Geometry Core.

Fluxo recomendado:

```
Selection/Tool Context
→ Command Registry
→ toolbar model
→ ViewportToolbar
```

Assim keyboard shortcuts, menus, plugins e toolbar apontam para os mesmos commands.

# ViewportBar

Organizar controles em grupos semânticos:

1. seleção/contexto;
2. transform/snap/pivot apenas quando relevantes;
3. visualização/overlays.

Não preencher espaço apenas para simular Blender.

# ContextPanel

Recebe um `context model` derivado da seleção/ferramenta e mostra as propriedades aplicáveis. Deve suportar seções progressivas, com Advanced quando necessário.

Não criar tabs globais equivalentes a Render/World/ViewLayer.

# PartsTree

Árvore do asset/projeto. Requisitos conceituais:

- hierarquia simples;
- expand/collapse;
- selection sync com viewport;
- visibility;
- lock quando adotado;
- rename;
- drag/reorder/group somente se fizer sentido para o modelo do documento;
- rows legíveis mesmo em alta densidade.

# AssetLibrary

Componente especializado contendo:

- busca;
- filtros;
- grid/list de thumbnails;
- slider/controle de tamanho das thumbnails;
- drag/drop ou insert explícito, conforme interação futura;
- estado vazio;
- loading e erro;
- collapse/reopen.

# View Mode Control

Segmented control específico:

```
Wireframe
Solid
Textured
Silhouette/Reference
```

Dropdown opcional pode agrupar Lighting/Unlit e overlays adicionais. Rendered não faz parte da baseline.

# Overlay Control

Preferir toggle + dropdown. Toggle controla visibilidade geral do conjunto; dropdown abre opções detalhadas.

# Smart Snap Control

Também é candidato natural a SplitButton. A ação principal liga/desliga Smart Snap; opções secundárias expõem filtros/tipos quando o usuário realmente precisar.

# Selection Level Control

`Object / Face / Edge / Point` deve ser rápido, explícito e acessível por teclado. O sistema pode também inferir contexto por interação, mas não esconder o estado atual.

# Tool context

Uma ferramenta ativa pode fornecer:

```
name
icon
command id
short description
primary parameters
advanced parameters
available selection types
shortcut
help/manual id
```

A UI deriva toolbar, context panel e tooltip dessa metadata sempre que possível.

# Feedback de operação

Operações como Extrude/Connect/Fuse/Projection devem oferecer feedback visual próximo da ação sem abrir diálogos modais desnecessários.

O padrão preferido é:

```
preview in viewport
+
compact parameters in Context/floating HUD
+
commit/cancel
```

# Modais

Usar modal apenas quando a operação realmente exige decisão bloqueante. Evitar substituir interação direta por caixas de diálogo.

# Tooltips e ajuda

Todo controle não óbvio deve ter tooltip. Para conceitos mais difíceis, o tooltip pode conter affordance `?`/link para manual, conforme a visão histórica do projeto.

Tooltips não podem ser a única forma de descobrir informações essenciais para keyboard/screen reader users.

# Plugin UI e painéis extensíveis

Plugins devem preferir `Command + metadata` para ações simples, mas a V1 permite **novos painéis completos** quando o workflow justificar uma superfície persistente.

Painéis de plugin obedecem ao mesmo contrato visual e comportamental dos painéis nativos:

- `PetuniaPanel` / `PetuniaPanelHeader`;
- componentes públicos Petunia;
- current theme automático;
- keyboard/focus;
- AccessKit semantics;
- min/max sizing definidos pelo host;
- collapse/reopen;
- state namespaced;
- layout em extension slots controlados.

Regiões públicas V1: `left`, `right`, `bottom`. Plugins podem sugerir região/tamanho preferido e regiões permitidas, mas não criam docking irrestrito, não substituem o viewport central e não criam floating windows arbitrárias.

O builder Lua pode compor Text/Heading, Section, Row/Column, ScrollRegion, Button/IconButton/Toggle/SplitButton, SegmentedControl, Text/Search/NumberField, Slider, Checkbox/Choice/Select, PropertyRow, List/Tree por adapter limitado, InlineMessage/Progress/EmptyState, Tooltip/HelpLink e CommandButton.

Não expor `egui::Ui`, Painter, raw input, shader/GPU hooks ou markup livre. Viewport overlays são outro extension point, com capability própria.

O contrato normativo completo de Plugin Panels e Theme Extensions está no capítulo 36.

# Critério de aceite de um componente

Antes de ser usado em telas reais, verificar:

- default/hover/pressed/active/disabled/focus;
- mouse;
- keyboard;
- accessible name/role/state;
- high-DPI/scaling;
- truncation/long labels;
- i18n expansion;
- tooltip;
- dark-theme contrast;
- hit target diferente do tamanho puramente visual quando necessário.

# Regra final

**Consistência é feature.** Se duas partes do Petunia realizam a mesma classe de interação, devem usar o mesmo componente/contrato em vez de imitações locais.
