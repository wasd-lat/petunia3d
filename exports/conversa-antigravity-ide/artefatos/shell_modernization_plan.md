# Plano: Modernização da Interface — Todas as Áreas (exceto Inspector Direito)

## Escopo

Análise e redesign de 6 áreas da interface, com base no screenshot atual e no código em [app.slint](file:///home/raillen/Documentos/petunia3d/crates/ui-slint/ui/app.slint):

1. **Header / Top Bar** (L1564–L1714)
2. **Painel Esquerdo / Tool Tray** (L1722–L1823)
3. **Viewport Floating Bar** (L2313–L2433)
4. **Bottom Tool Shelf** (L2888–L3098)
5. **Status Bar / Footer** (L5380–L5468)
6. **Tool Properties Floating Window** (L2437–L2724) — o painel mais problemático

> [!NOTE]
> O inspector direito está excluído — você já está mexendo nele.

---

## 1. Header / Top Bar

### Estado Atual (42px de altura)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Petunia  File  Edit  View  Window  [MODEL] [PAINT] [UV]  ↶ ↷  ···  Saved ≡ │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Problemas Identificados

| # | Problema | Evidência no código |
|---|---|---|
| 1.1 | **Workspace tabs (MODEL/PAINT/UV) são retângulos individuais**, não um segmented control unificado — cada um é um `Rectangle { width: 96px }` independente | L1619–L1665 |
| 1.2 | **Sem indicador visual de transição** entre workspaces — mudança instantânea, sem sliding pill | L1619–L1665 |
| 1.3 | **Ações à direita (Saved, Open, Save, Search, Settings) sem agrupamento** — são TopAction individuais fluindo linearmente | L1683–L1712 |
| 1.4 | **"Petunia" é texto estático** — sem logo, sem identidade visual | L1578–L1584 |
| 1.5 | **Undo/Redo sem estado disabled** — os ícones não refletem `can-undo` / `can-redo` | L1667–L1676 |
| 1.6 | **Dois `horizontal-stretch: 1` adjacentes** (L1678+L1681) criam distribuição de espaço inconsistente | L1678–L1681 |

### Propostas

#### 1A. Segmented Control para Workspaces

Em vez de 3 retângulos separados, criar um **segmented control** com pill animada:

```slint
component WorkspaceSegment inherits Rectangle {
    in property <string> active;
    callback changed(string);
    
    width: 3 * 80px + 8px;
    height: 30px;
    border-radius: 7px;
    background: DesignTokens.surface;
    border-width: 1px;
    border-color: DesignTokens.border;
    
    // Sliding pill
    Rectangle {
        x: active == "MODEL" ? 2px
         : active == "PAINT" ? 82px
         : 162px;
        y: 2px;
        width: 80px;
        height: 26px;
        border-radius: 5px;
        background: DesignTokens.accent;
        animate x { duration: 200ms; easing: ease-out-quad; }
    }
    
    HorizontalLayout {
        padding: 2px;
        spacing: 2px;
        for ws in [
            { id: "MODEL", label: "MODEL" },
            { id: "PAINT", label: "PAINT" },
            { id: "UV", label: "UV" },
        ]: Rectangle {
            width: 80px;
            height: 26px;
            background: transparent;
            Text {
                text: ws.label;
                color: root.active == ws.id
                    ? DesignTokens.canvas
                    : DesignTokens.text-secondary;
                font-size: 11px;
                font-weight: 700;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
            TouchArea { clicked => { root.changed(ws.id); } }
        }
    }
}
```

#### 1B. Undo/Redo com estado disabled

```slint
TopAction {
    icon: IconSet.Undo;
    semantic-label: "Undo";
    // NOVO: opacidade baseada no estado
    opacity: root.can-undo ? 1.0 : 0.35;
    animate opacity { duration: 100ms; }
    clicked => { if root.can-undo { root.undo-requested(); } }
}
```

#### 1C. Agrupamento visual da área direita

Separar com um divisor vertical sutil antes do grupo "Saved + ações":

```slint
Rectangle { width: 1px; height: 18px; background: DesignTokens.border; }
// ... Saved + Open + Save + Search + Settings
```

> [!IMPORTANT]
> **Decisão necessária**: O logo "Petunia" deve ficar como texto bold, ser substituído por um ícone/logotipo SVG, ou removido em favor de mais espaço horizontal?

---

## 2. Painel Esquerdo / Tool Tray

### Estado Atual (40px de largura)

```
┌──┐
│ + │  Add Primitive
│ □ │  Add Cube
│ ● │  Add Sphere
│   │
│   │
│   │
└──┘
```

### Problemas Identificados

| # | Problema |
|---|---|
| 2.1 | **Conteúdo mínimo no modo MODEL** — apenas 3 botões (Add, Cube, Sphere). Os outros primitivos vivem no popover. |
| 2.2 | **Largura fixa de 40px** é estreita — ícones de 32px com 4px de padding em cada lado ficam apertados |
| 2.3 | **Sem tooltip de atalho** — o `+` não mostra que tem atalho |
| 2.4 | **Sem separação visual** entre "Add menu" (toggle) e "Add directly" (ação) |
| 2.5 | **Em PAINT** mostra 6 ferramentas, em UV mostra 3 — diferença drástica de uso vertical |
| 2.6 | **Não tem label "Create" ou "Tools"** — o painel parece anônimo |

### Propostas

#### 2A. Diferenciar ferramentas de ações

A barra esquerda mistura dois conceitos:
- **Ferramentas** (Select, Move, Brush — alteram o modo de interação)
- **Ações** (Add Cube — executam uma vez)

No modo MODEL, as ferramentas estão na **bottom bar**. O painel esquerdo só tem ações de criação.

> [!IMPORTANT]
> **Decisão necessária**: O painel esquerdo deve continuar sendo exclusivamente "criação de primitivos" ou deve absorver as ferramentas de modelagem que hoje estão na bottom bar? O caderno (cap. 36) diz "barra de criação à esquerda", o que sugere que a função é criação.

#### 2B. Melhorias visuais

```slint
// Label de seção no topo do rail
Rectangle {
    width: 40px; height: 24px;
    Text {
        text: "Create"; // ou o TextId correspondente
        color: DesignTokens.text-muted;
        font-size: 8px;
        font-weight: 700;
        letter-spacing: 0.5px;
        horizontal-alignment: center;
        vertical-alignment: center;
    }
}
// Separador entre "Add menu toggle" e "primitivos diretos"
Rectangle { width: 24px; height: 1px; background: DesignTokens.border; }
```

#### 2C. Expandir o rail para 44px

Ganhar 4px extras para melhor respiro interno dos ícones de 32px.

---

## 3. Viewport Floating Bar

### Estado Atual

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Persp │ 📷 🔲 ↺ │ [Wire][Solid][Mat][Lit] │ 👁 ⚙ ⊕ 🧲 ◎ ··· OBJECT POINT EDGE FACE │
└──────────────────────────────────────────────────────────────────────────┘
```

### Problemas Identificados

| # | Problema |
|---|---|
| 3.1 | **Barra muito longa** — tenta encaixar ~15 controles numa única faixa horizontal |
| 3.2 | **Selection domain (OBJECT/POINT/EDGE/FACE)** está misturado com controles de viewport — são conceitos ortogonais |
| 3.3 | **Responsive quebra awkwardly** — `if viewport-region-width >= 600/680/750` esconde itens sem indicador de overflow |
| 3.4 | **Sem drop-shadow** — a barra não tem `drop-shadow-*` e fica "colada" na viewport |
| 3.5 | **Pivot, Snap, Proportional** são avançados mas estão no mesmo nível visual que Projection e Shading |

### Propostas

#### 3A. Separar em duas barras

Dividir a viewport bar em **duas** floating bars empilhadas ou separadas:

**Barra de Vista** (topo-centro): controles de câmera/shading
```
┌──────────────────────────────────────────────┐
│ Persp │ 📷 🔲 ↺ │ [Wire][Solid][Mat][Lit] │ 👁 ⚙ │
└──────────────────────────────────────────────┘
```

**Barra de Domínio** (topo-centro, abaixo da anterior, ou topo-direita):
```
┌────────────────────────────┐
│ OBJECT  POINT  EDGE  FACE │
└────────────────────────────┘
```

> [!IMPORTANT]
> **Decisão necessária**: A barra de domínio de seleção deve ficar:
> - (a) Abaixo da barra de vista, centralizada
> - (b) No canto superior direito da viewport (próximo ao view gizmo)
> - (c) Incorporada na top bar do shell (ao lado do workspace selector)
> - (d) Mantida unificada com a barra de vista como está hoje

#### 3B. Adicionar sombra suave

```slint
viewport-bar := Rectangle {
    // ADICIONAR:
    drop-shadow-blur: 10px;
    drop-shadow-color: #00000044;
    drop-shadow-offset-y: 2px;
}
```

#### 3C. Mover Pivot/Snap/Proportional para popover

Em vez de mostrar condicionalmente na barra, esses 3 controles avançados podem viver no popover do ⚙ (Shading Options), que se transformaria em **Viewport Options**:

```
┌─ Viewport Options ──────────┐
│ Shading: [Wire][Solid]...   │
│ X-Ray opacity: ═══●═══      │
│ ─────────────────────        │
│ Pivot: Median Point    ▾    │
│ Snap: Grid             ▾    │
│ Proportional: Smooth  ●    │
└─────────────────────────────┘
```

---

## 4. Bottom Tool Shelf (Context Bar)

### Estado Atual

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ 🖱 Pos Rot Scale Transform │ ⤨ Copy Extrude Push Inset Bevel Knife Loop ... │ │
└────────────────────────────────────────────────────────────────────────────────┘
```

### Problemas Identificados

| # | Problema |
|---|---|
| 4.1 | **Mistura tools (Select, Move) com ações one-shot (Extrude, Subdivide)** sem diferenciação visual |
| 4.2 | **Sem indicador de ferramenta ativa** claro — `active` muda background mas sem sliding pill |
| 4.3 | **Overflow esconde ferramentas** quando viewport < 760px sem indicação de que existem mais |
| 4.4 | **Ferramentas duplicadas** — Move/Rotate/Scale existem aqui E no painel esquerdo (em PAINT) |
| 4.5 | **Sem label "Tools"** — barra anônima flutuante |
| 4.6 | **Separadores insuficientes** — apenas 1 separador vertical antes do Delete |

### Propostas

#### 4A. Agrupamento semântico com separadores

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🖱 Pos Rot Scl ⊞ │ ⤨ Dup │ ⬆ Ext Push Inset Bevel │ ✂ Knife Loop │ 🗑 │
│  ← Transform →     │ Edit │    ← Deformation →     │ ← Topology →│ Del│
└──────────────────────────────────────────────────────────────────────┘
```

Grupos propostos:
1. **Transform**: Select, Position, Rotate, Scale, Transform (tool toggle)
2. **Edit**: Lasso, Duplicate
3. **Deformation**: Extrude, Push/Pull, Inset, Bevel
4. **Topology**: Knife, Loop Cut, Profile, Subdivide, Merge, Slice
5. **Danger**: Delete

#### 4B. Sliding pill para ferramenta ativa

Adicionar um retângulo animado que desliza para a posição da ferramenta ativa (similar ao conceito do WorkspaceSegment):

```slint
// Pill calculada por posição do tool ativo
Rectangle {
    x: /* calculado pelo Rust baseado no índice da tool ativa */;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: DesignTokens.accent.transparentize(0.85);
    border-width: 1px;
    border-color: DesignTokens.accent.transparentize(0.5);
    animate x { duration: 180ms; easing: ease-out-quad; }
}
```

#### 4C. Overflow indicator

Quando ferramentas são escondidas por falta de espaço, mostrar um `•••` com badge numérico:

```slint
if active-workspace == "MODEL" && root.viewport-region-width < 760: Rectangle {
    width: 32px; height: 32px;
    border-radius: 6px;
    background: DesignTokens.surface;
    
    HorizontalLayout {
        alignment: center;
        IconDisplay { icon: IconSet.MoreHorizontal; size: 16px; stroke: DesignTokens.text-secondary; }
    }
    // Badge com contagem
    Rectangle {
        x: 20px; y: 0px;
        width: 14px; height: 14px;
        border-radius: 7px;
        background: DesignTokens.accent;
        Text {
            text: "7"; // número de tools escondidas
            color: DesignTokens.canvas;
            font-size: 8px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

---

## 5. Status Bar / Footer

### Estado Atual (32px)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Nothing under the cursor · LMB select · MMB orbit · Esc...  [Parts] [Asset Library] │ 12 tris │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Problemas Identificados

| # | Problema |
|---|---|
| 5.1 | **Status message muito genérica** — "Nothing under the cursor" não ajuda |
| 5.2 | **Hint text sem hierarquia** — "LMB select · MMB orbit · Esc..." é uma lista flat |
| 5.3 | **Parts e Asset Library como botões toggle** na barra de status é incomum — confunde status com navegação |
| 5.4 | **Stats (tris) estão no canto extremo direito** sem destaque |

### Propostas

#### 5A. Simplificar o conteúdo

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🟢 Ready                                    12 tris · 8 verts · 6 faces │
└──────────────────────────────────────────────────────────────────────┘
```

- **Esquerda**: status da operação (Ready / Extruding... / Saved)
- **Centro**: vazio (respiro)
- **Direita**: stats do objeto selecionado

#### 5B. Mover Parts e Asset Library

Os toggles de **Parts** e **Asset Library** podem migrar para:
- O painel esquerdo (como tabs ou ícones adicionais)
- O header (como item do menu View)
- Manter na footer mas com visual menos proeminente

> [!IMPORTANT]
> **Decisão necessária**: Onde devem viver os toggles de Parts e Asset Library?
> - (a) Manter na footer como estão
> - (b) Mover para o header à direita
> - (c) Mover para o painel esquerdo como pills verticais
> - (d) Apenas no menu View, sem botão always-visible

---

## 6. Tool Properties Floating Window 🔴

Este é o painel mais problemático. Veja o screenshot que você enviou.

### Estado Atual

O painel em [app.slint L2437–L2724](file:///home/raillen/Documentos/petunia3d/crates/ui-slint/ui/app.slint#L2437-L2724) é um **mega-bloco monolítico** que tenta atender múltiplos casos:

```
Casos multiplexados no mesmo Rectangle:
├── tool_modal_active → Extrude/Inset/Bevel/PushPull/Scale (1 NumericField)
├── loop_cut_active → Slide + Cuts (+/- buttons)
├── loop_cut_armed → Cuts count + hint
├── profile_active → Points + Close + Depth + Generate/Revolve
├── profile presets → Rectangle + Circle
└── operation_hud_active → HUD lines monospace
```

### Problemas Identificados

| # | Problema | Gravidade |
|---|---|---|
| 6.1 | **UI monolítica** — um único `Rectangle` com 280px de largura tenta renderizar 6 estados diferentes com `if` branches | 🔴 |
| 6.2 | **Loop Cut quebrado visualmente** (screenshot) — o título diz "Loop Cut" mas abaixo repete "Loop Cut · 3 cut(s)", duplicando informação | 🔴 |
| 6.3 | **Campos Slide e Cuts do Loop Cut aparecem dobrados** — "Cuts 3 / Slide -0.130" aparece como HUD texto E como campos editáveis abaixo, criando confusão | 🔴 |
| 6.4 | **Ferramentas sem propriedades** mostram texto genérico "Select Extrude, Inset, Bevel..." — não deveria abrir o painel | 🟡 |
| 6.5 | **Largura fixa de 280px** — no screenshot, parece 355px e está grande demais para a informação que contém | 🟡 |
| 6.6 | **Sem animate height** — o painel pula de tamanho ao trocar entre estados | 🟡 |
| 6.7 | **Cancel/Apply buttons sem ícone** — apenas texto, baixa affordance | 🟡 |
| 6.8 | **Faltam opções por ferramenta** — Move/Rotate/Scale abrem o painel mas não têm campos editáveis (axis constraint, value input) | 🔴 |

### Problemas do Screenshot

Olhando o screenshot do Loop Cut que você enviou, vejo:

```
┌─────────────────────────────────┐
│ Loop Cut                     ∧  │  ← título
│                                  │
│ Select Extrude, Inset, Bevel,   │  ← hint genérica que NÃO deveria
│ Loop Cut or Profile to edit     │     aparecer com Loop Cut ativo
│ parameters here.                │
│                                  │
│         Loop Cut · 3 cut(s)     │  ← HUD duplicando o título
│                                  │
│ Cuts   3                        │  ← campo HUD (não editável)
│ Slide  -0.130                   │  ← campo HUD (não editável)
│                                  │
│ Enter Confirm  Esc Cancel       │  ← hint texto
│ Drag to slide                   │
│                                  │
│ Slide    [ -0.13         ]      │  ← campo editável (REPETIDO)
│                                  │
│ Cuts     [  3      ] [-] [+]    │  ← campo editável (REPETIDO)
│                                  │
│ [   Cancel   ] [   Apply    ]   │  ← botões
└─────────────────────────────────┘
```

O problema é claro: **3 camadas de informação redundante**.

### Solução Proposta: Redesign Completo

#### 6A. Arquitetura: um componente por modo

Em vez de um mega-bloco com `if` branches, criar componentes dedicados:

```slint
// Cada ferramenta tem seu próprio card
component ToolCard inherits Rectangle {
    in property <string> title;
    in property <bool> collapsed: false;
    callback collapse-toggled;
    
    width: 240px;  // mais estreito que o atual
    height: root.collapsed ? 36px : content.preferred-height + 44px;
    border-radius: 8px;
    background: DesignTokens.surface-raised;
    border-width: 1px;
    border-color: DesignTokens.border;
    drop-shadow-blur: 10px;
    drop-shadow-color: #00000050;
    drop-shadow-offset-y: 3px;
    
    animate height { duration: 180ms; easing: ease-out-cubic; }
    
    VerticalLayout {
        padding: 10px;
        spacing: 6px;
        
        // Header
        HorizontalLayout {
            height: 24px;
            Text {
                text: root.title;
                color: DesignTokens.text-primary;
                font-size: 12px;
                font-weight: 700;
                horizontal-stretch: 1;
                vertical-alignment: center;
            }
            Rectangle {
                width: 20px; height: 20px;
                IconDisplay {
                    icon: root.collapsed ? IconSet.ChevronDown : IconSet.ChevronUp;
                    size: 12px;
                    stroke: DesignTokens.text-muted;
                }
                TouchArea { clicked => { root.collapse-toggled(); } }
            }
        }
        
        // Content slot
        if !root.collapsed: content := VerticalLayout {
            spacing: 6px;
            @children
        }
    }
}
```

#### 6B. Loop Cut Card — sem redundância

```slint
if loop-cut-active: ToolCard {
    x: 12px;
    y: 56px;
    title: "Loop Cut";
    collapsed: root.tool-properties-collapsed;
    collapse-toggled => { root.tool-properties-collapsed = !root.tool-properties-collapsed; }
    
    // Campos diretos, sem HUD duplicado
    HorizontalLayout {
        spacing: 6px;
        height: 28px;
        Text { text: "Cuts"; color: DesignTokens.text-secondary; font-size: 11px; width: 48px; vertical-alignment: center; }
        NumericField {
            label: "";
            value: root.loop-cut-cuts;
            step: 1;
            horizontal-stretch: 1;
            text-committed(text) => { root.loop-cut-count-committed(text); }
        }
    }
    HorizontalLayout {
        spacing: 6px;
        height: 28px;
        Text { text: "Slide"; color: DesignTokens.text-secondary; font-size: 11px; width: 48px; vertical-alignment: center; }
        NumericField {
            label: "";
            value: root.loop-cut-slide;
            step: 0.01;
            horizontal-stretch: 1;
            text-committed(text) => { root.loop-cut-slide-committed(text); }
        }
    }
    
    // Hint compacta
    Text {
        text: "Enter confirm · Esc cancel · Drag slide";
        color: DesignTokens.text-muted;
        font-size: 9px;
    }
    
    // Action buttons
    HorizontalLayout {
        spacing: 6px;
        height: 28px;
        Rectangle {
            horizontal-stretch: 1;
            border-radius: 4px;
            background: DesignTokens.surface;
            border-width: 1px;
            border-color: DesignTokens.border;
            Text { text: "Cancel"; color: DesignTokens.text-primary; font-size: 11px; horizontal-alignment: center; vertical-alignment: center; }
            TouchArea { clicked => { root.loop-cut-cancel(); } }
        }
        Rectangle {
            horizontal-stretch: 1;
            border-radius: 4px;
            background: DesignTokens.accent;
            Text { text: "Apply"; color: DesignTokens.canvas; font-size: 11px; font-weight: 600; horizontal-alignment: center; vertical-alignment: center; }
            TouchArea { clicked => { root.loop-cut-apply(); } }
        }
    }
}
```

Resultado visual:
```
┌─ Loop Cut ────────── ∧ ─┐
│ Cuts    [═══3═══]       │
│ Slide   [═-0.13═]       │
│                          │
│ Enter · Esc · Drag slide │
│ [ Cancel ] [  Apply  ]  │
└──────────────────────────┘
```

#### 6C. Tool Modal Card (Extrude/Inset/Bevel)

```slint
if tool-modal-active: ToolCard {
    x: 12px; y: 56px;
    title: root.tool-modal-title;  // "Extrude", "Inset", "Bevel"
    // ...
    HorizontalLayout {
        spacing: 6px; height: 28px;
        Text { text: root.tool-modal-label; /* "Distance", "Amount", "Width" */ }
        NumericField { value: root.tool-modal-value; /* ... */ }
    }
    // hints + Cancel/Apply
}
```

#### 6D. Opções por ferramenta faltantes

Ferramentas que **abrem o painel mas não têm campos editáveis**:

| Ferramenta | O que deveria mostrar |
|---|---|
| **Move** | Axis constraint (X/Y/Z toggles), value input, coordinate space (Local/Global) |
| **Rotate** | Axis constraint, angle input, center (selection/individual/cursor) |
| **Scale** | Axis constraint, factor input, uniform toggle |
| **Select** | Selection mode info, count de selecionados |
| **Knife** | Angle snap toggle, midpoint toggle |
| **Slice** | Axis (X/Y/Z), offset value |

> [!WARNING]
> Implementar campos para Move/Rotate/Scale/Knife/Slice requer mudanças no **Rust bridge** (`lib.rs`), não apenas no Slint. Cada ferramenta precisará expor propriedades adicionais no `ViewModel`. Este é o ponto de maior impacto técnico.

#### 6E. Não abrir o painel quando não há opções

Atualmente, `tool_options_active` é `true` para qualquer tool que não seja `select/box_select/lasso_select` (ver [lib.rs L6176–L6184](file:///home/raillen/Documentos/petunia3d/crates/ui-slint/src/lib.rs#L6176-L6184)). Isso faz tools como Move e Rotate abrirem o painel mostrando "Select Extrude... to edit parameters" — confuso.

**Proposta**: só abrir quando há campos reais a editar. O painel não aparece "vazio com hint genérica".

---

## Decisões Consolidadas

Preciso das suas respostas para prosseguir:

| # | Pergunta | Opções |
|---|---|---|
| **D1** | Logo "Petunia" no header | (a) Manter texto bold (b) SVG logo (c) Remover |
| **D2** | Painel esquerdo: criação-only ou absorver tools? | (a) Criação apenas (cap. 36) (b) Absorver transform tools |
| **D3** | Barra de domínio de seleção | (a) Abaixo da viewport bar (b) Canto superior direito (c) No header (d) Unificada como está |
| **D4** | Parts/Asset Library toggles | (a) Manter na footer (b) Header (c) Painel esquerdo (d) Só no menu View |
| **D5** | Tool Properties: redesign componente-por-modo? | (a) Sim, criar ToolCard (b) Refatorar o bloco atual sem separar |
| **D6** | Implementar campos faltantes (Move/Rotate/Scale)? | (a) Sim, nesta mesma fase (b) Postergar para depois do visual |
| **D7** | Largura do tool properties panel | (a) 240px (mais compacto) (b) 280px (atual) (c) Auto-width |

---

## Verificação

### Testes Automatizados
```bash
cargo test -p petunia_ui_slint --lib
cargo clippy -p petunia_ui_slint --all-targets -- -D warnings
cargo fmt -p petunia_ui_slint -- --check
```

### Verificação Manual
- Abrir o shell em cada workspace (MODEL/PAINT/UV) e verificar layout
- Ativar cada ferramenta modal (Extrude, Inset, Bevel, Loop Cut, Profile) e verificar o card
- Redimensionar a janela para testar responsive breakpoints
- Verificar que hover/focus states animam suavemente
- Screenshot antes/depois de cada área
