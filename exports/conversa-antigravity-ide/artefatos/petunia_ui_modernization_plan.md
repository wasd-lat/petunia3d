# Petunia3D — Plano de Modernização da Interface

## Estado Atual (Screenshot analisado)

O shell atual do Petunia3D já tem uma boa base:
- **Viewport-first** ✅ — o cubo domina a tela
- **Dark theme** ✅ — consistente, com tokens bem definidos
- **Inspector colapsável** ✅ — painel direito com Parts/Transform/Material/Object/Modifiers/Quick Actions
- **Bottom tool shelf** ✅ — ícones de ferramentas + status bar
- **Top bar** ✅ — Petunia | menus | MODEL/PAINT/UV | undo/redo | saved

### Problemas visuais identificados no screenshot

| # | Problema | Gravidade |
|---|---|---|
| 1 | **Seções empilhadas sem gap visual** — Parts, Transform, Material, Object, Modifiers, Quick Actions formam um bloco monolítico sem respiro | 🔴 Alta |
| 2 | **Sem animações de transição** — abrir/fechar seções é instantâneo (pulo visual) | 🟡 Média |
| 3 | **NumericField badges coloridos (X🔴 Y🟢 Z🔵)** são excessivamente saturados contra o dark theme — vibram visualmente | 🟡 Média |
| 4 | **Quick Actions** parecem botões fantasma — "Subdivide / Fuse / Cut / Join" sem affordance clara de botão | 🟡 Média |
| 5 | **Status bar densa** — "Nothing under the cursor · LMB select · MMB orbit · Esc..." mistura demasiada info sem hierarquia | 🟡 Média |
| 6 | **Sem sombra/elevação** nos painéis — tudo no mesmo plano z, parece flat demais | 🟡 Média |
| 7 | **Viewport bar** (Persp + ícones) flutua sem containment visual | 🟢 Baixa |
| 8 | **Tool shelf inferior** — ícones sem separação funcional (seleção vs criação vs edição) | 🟢 Baixa |

---

## EIXO 1: Painéis Flutuantes Independentes

### Conceito

Cada módulo do inspector (Parts, Transform, Material, Object, Modifiers) se torna um **floating panel independente** — pode ser:
- **Reordenado** (drag do header)
- **Destacado** (undocked — vira um retângulo flutuante sobre o viewport)
- **Re-ancorado** (devolvido ao inspector rail ou a outro slot)

### O que Slint pode fazer nativamente

| Capacidade | Status Slint | Detalhe |
|---|---|---|
| `x` / `y` arbitrários em overlay | ✅ Nativo | `Rectangle { x: ...; y: ...; }` sobre o viewport |
| Drag de posição | ✅ Nativo | `TouchArea` com `moved =>` atualizando `x` / `y` |
| Animação de posição | ✅ Nativo | `animate x { duration: 200ms; easing: ease-out-quad; }` |
| Sombra / elevação | ⚠️ Parcial | Sem `box-shadow` nativo — usar `Rectangle` empilhado com blur ou imagem 9-patch |
| Drag & drop reordenação | ✅ Nativo (1.17+) | Drag-and-drop suportado desde junho 2026 |
| Multi-window tear-off | ❌ Não nativo | Slint não tem docking framework; precisa extensão Rust |
| Z-order dinâmico | ⚠️ Workaround | Slint renderiza na ordem de declaração; reordenação requer trick com `visible` |

### Solução proposta: FloatingPanel component

```slint
// NOVO componente: painel flutuante com drag, z-ordering, e snap
component FloatingPanel inherits Rectangle {
    in property <string> title;
    in property <Icon> header-icon;
    in property <bool> docked: true;
    in-out property <length> float-x: 100px;
    in-out property <length> float-y: 100px;
    in property <bool> open: true;
    callback undocked;
    callback redocked;
    callback toggled;
    callback drag-started;
    callback drag-ended(length, length);

    // Quando docked: width 100%, posição no flow
    // Quando undocked: posição absoluta, sombra, z-elevado
    x: root.docked ? 0px : root.float-x;
    y: root.docked ? 0px : root.float-y;
    width: root.docked ? 100% : 300px;
    height: root.open ? content-layout.preferred-height : 36px;
    
    border-radius: root.docked ? 6px : 10px;
    background: DesignTokens.surface-raised;
    border-width: 1px;
    border-color: root.docked ? DesignTokens.border-strong : DesignTokens.accent.transparentize(0.6);
    opacity: 1.0;
    
    animate x { duration: 180ms; easing: ease-out-quad; }
    animate y { duration: 180ms; easing: ease-out-quad; }
    animate width { duration: 180ms; easing: ease-out-quad; }
    animate height { duration: 200ms; easing: ease-out-cubic; }
    animate opacity { duration: 120ms; }

    // Sombra fake: retângulo atrás com blur simulado (extensão necessária)
    if !root.docked: Rectangle {
        x: 2px; y: 4px;
        width: parent.width;
        height: parent.height;
        border-radius: parent.border-radius;
        background: #00000044;
    }

    content-layout := VerticalLayout {
        padding: 4px 8px;
        spacing: 4px;

        // Header com drag handle
        Rectangle {
            height: 28px;
            HorizontalLayout {
                spacing: 6px;
                alignment: center;
                
                // Drag handle dots
                Rectangle {
                    width: 12px;
                    height: 16px;
                    // 6-dot grip pattern via ícone
                    IconDisplay {
                        icon: IconSet.GripVertical;
                        size: 12px;
                        stroke: DesignTokens.text-muted;
                    }
                }
                
                IconDisplay {
                    icon: root.header-icon;
                    size: 13px;
                    stroke: DesignTokens.text-secondary;
                }
                Text {
                    text: root.title;
                    color: DesignTokens.text-primary;
                    font-size: 12px;
                    font-weight: 600;
                    horizontal-stretch: 1;
                }
                
                // Undock button
                TopAction {
                    icon: root.docked ? IconSet.Unplug : IconSet.Plug;
                    semantic-label: root.docked ? "Undock" : "Dock";
                    clicked => {
                        if root.docked { root.undocked(); }
                        else { root.redocked(); }
                    }
                }
                
                // Collapse chevron
                IconDisplay {
                    icon: root.open ? IconSet.ChevronDown : IconSet.ChevronRight;
                    size: 12px;
                    stroke: DesignTokens.text-muted;
                }
            }
            TouchArea {
                // Double-click to toggle dock/undock
                // Single drag to reorder or move when undocked
                property <length> drag-start-x;
                property <length> drag-start-y;
                property <bool> is-dragging: false;
                
                pointer-event(event) => {
                    if event.kind == PointerEventKind.down {
                        self.drag-start-x = self.mouse-x;
                        self.drag-start-y = self.mouse-y;
                        self.is-dragging = false;
                    }
                }
                moved => {
                    if self.pressed && !root.docked {
                        root.float-x += self.mouse-x - self.drag-start-x;
                        root.float-y += self.mouse-y - self.drag-start-y;
                        if !self.is-dragging {
                            self.is-dragging = true;
                            root.drag-started();
                        }
                    }
                }
                clicked => { root.toggled(); }
            }
        }

        // Content slot
        if root.open: VerticalLayout {
            @children
        }
    }
}
```

### Extensão Rust necessária: Z-order manager

Slint não permite reordenar z-index em runtime. Precisamos de um **`FloatingPanelManager`** em Rust que:

1. Mantém um `Vec<PanelId>` com a ordem de z
2. Quando um painel é clicado/arrastado, move-o para o topo
3. O Slint renderiza N slots de overlay na ordem do Vec
4. Cada slot recebe os dados do painel correspondente via bindings

```rust
// crates/ui-slint/src/floating.rs (novo módulo)
pub struct FloatingPanelManager {
    panels: Vec<FloatingPanelState>,
    z_order: Vec<usize>, // índices em `panels`, front-to-back
}

pub struct FloatingPanelState {
    pub id: PanelId,
    pub docked: bool,
    pub x: f32,
    pub y: f32,
    pub open: bool,
    pub width: f32,
}
```

---

## EIXO 2: Micro-animações e Polish Visual

### 2.1 Transições de abertura/fechamento de seções

**Problema**: seções abrem/fecham com pulo brusco.
**Solução Slint nativa**:

```slint
// Já existe no InspectorSection, mas falta animate:
height: root.open ? section-layout.preferred-height : 36px;
// ADICIONAR:
animate height { duration: 200ms; easing: ease-out-cubic; }
```

> [!TIP]
> Slint anima `height` sem problemas. O `preferred-height` é recalculado automaticamente, e a animação interpola suavemente.

### 2.2 Hover states mais ricos

**Problema**: hover nos botões é apenas mudança de background sem transição.
**Solução**: adicionar `animate background` a todos os componentes interativos:

```slint
// Em ToolButton, ViewportBarButton, InspectorPill, etc:
animate background { duration: 100ms; easing: ease-out-quad; }
animate border-width { duration: 80ms; }
animate border-color { duration: 100ms; }
```

### 2.3 Sombra/Elevação

**Problema**: Slint não tem `box-shadow` nativo.

**Solução — extensão própria com 9-patch shadow**:

Criar uma imagem PNG de sombra (retângulo translúcido blur) e usá-la como `Image` atrás dos painéis. Ou usar camadas de `Rectangle` com opacidade decrescente:

```slint
component PanelShadow inherits Rectangle {
    // 3 camadas simulam gaussian blur
    Rectangle {
        x: 0px; y: 2px;
        width: parent.width + 4px; height: parent.height + 4px;
        border-radius: parent.border-radius + 2px;
        background: #0000000a;
    }
    Rectangle {
        x: -1px; y: 3px;
        width: parent.width + 6px; height: parent.height + 6px;
        border-radius: parent.border-radius + 3px;
        background: #00000008;
    }
    Rectangle {
        x: -2px; y: 5px;
        width: parent.width + 8px; height: parent.height + 10px;
        border-radius: parent.border-radius + 4px;
        background: #00000005;
    }
}
```

### 2.4 Active tool indicator animado

**Problema**: não é claro qual ferramenta está ativa no tool shelf.
**Solução**: sliding underline/background pill animada:

```slint
// Pill que desliza para a posição da ferramenta ativa
Rectangle {
    x: active-tool-index * 36px;  // calculado pelo Rust
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: DesignTokens.accent.transparentize(0.85);
    border-width: 1px;
    border-color: DesignTokens.accent.transparentize(0.5);
    
    animate x { duration: 180ms; easing: ease-out-quad; }
}
```

---

## EIXO 3: Componentes Mais Limpos e Modernos

### 3.1 NumericField — suavizar badges de eixo

**Atual**: `X` sobre badge vermelho vibrante (#ff6e6e) é visualmente agressivo.

**Proposta**: usar cor de eixo apenas no texto da label, não em background:

```slint
// ANTES (implícito no label-color):
Text { text: "X"; color: DesignTokens.axis-x; font-weight: 700; }
// PROPOSTA: suavizar com transparência
Text { text: "X"; color: DesignTokens.axis-x.transparentize(0.25); font-weight: 600; }
```

Alternativa: usar um **dot indicator** (●) em vez do texto colorido.

### 3.2 Quick Actions — dar affordance de botão

**Problema**: "Subdivide", "Fuse", "Cut", "Join" parecem labels passivos.

**Proposta**: adicionar ícone + hover + borda sutil:

```slint
component QuickActionButton inherits Rectangle {
    in property <string> label;
    in property <Icon> icon;
    in property <bool> enabled: true;
    callback clicked;

    height: 30px;
    border-radius: 5px;
    background: qa-touch.has-hover && root.enabled
        ? DesignTokens.surface-hover
        : DesignTokens.surface;
    border-width: 1px;
    border-color: qa-touch.has-hover ? DesignTokens.border-strong : DesignTokens.border;
    opacity: root.enabled ? 1.0 : 0.5;
    
    animate background { duration: 100ms; }
    animate border-color { duration: 100ms; }

    HorizontalLayout {
        padding-left: 10px;
        padding-right: 10px;
        spacing: 6px;
        alignment: center;
        
        IconDisplay {
            icon: root.icon;
            size: 13px;
            stroke: DesignTokens.text-secondary;
        }
        Text {
            text: root.label;
            color: root.enabled ? DesignTokens.text-primary : DesignTokens.text-muted;
            font-size: 11px;
            font-weight: 500;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }
    }

    qa-touch := TouchArea {
        enabled: root.enabled;
        clicked => { root.clicked(); }
    }
}
```

### 3.3 InspectorSection header — mais leve

**Proposta**: remover a `border-width: 1px` do wrapper quando docked (bordas duplas entre seções adjacentes). Usar apenas o `border-radius` e gap de `spacing`:

```slint
// ANTES:
border-width: 1px;
border-color: DesignTokens.border-strong;

// DEPOIS (quando docked):
border-width: 0px;
border-color: transparent;
// Separação visual vem do spacing: 10px do VerticalLayout pai
```

### 3.4 Viewport Bar — containment pill

**Proposta**: envolver os ícones da viewport bar num background pill translúcido:

```slint
Rectangle {
    border-radius: 8px;
    background: DesignTokens.surface.with-alpha(0.85);
    border-width: 1px;
    border-color: DesignTokens.border.transparentize(0.5);
    // backdrop-blur: não existe em Slint, simular com opacity alta
}
```

### 3.5 Tool shelf — separadores de grupo

**Proposta**: adicionar separadores verticais (1px height: 16px) entre grupos:

```
[ Select | Move | Rotate | Scale ] | [ Knife | Loop | Extrude ] | [ Cube | Sphere | ... ] | [ Delete ]
```

```slint
Rectangle { width: 1px; height: 16px; background: DesignTokens.border; }
```

---

## EIXO 4: Coerência Sistêmica

### 4.1 Tokens de elevação (novo)

Adicionar tokens de profundidade ao [tokens.slint](file:///home/raillen/Documentos/petunia3d/crates/ui-slint/ui/tokens.slint):

```slint
// Elevação / profundidade
in-out property <color> shadow-1: #00000012;  // painéis docked
in-out property <color> shadow-2: #00000020;  // floating panels  
in-out property <color> shadow-3: #00000030;  // modais/overlays
in-out property <length> radius-floating: 10px;
in-out property <length> radius-modal: 12px;

// Durações canônicas (consistency)
// (Slint não suporta tokens de duração como property, mas documentar como convenção)
// fast: 100ms — hover, border
// normal: 200ms — height, position, background
// slow: 350ms — modal appear, panel slide
```

### 4.2 Typography scale

Padronizar tamanhos de fonte (atualmente variam entre 10px–14px sem sistema):

| Role | Size | Weight | Uso |
|---|---|---|---|
| **header-1** | 14px | 700 | Título do app, header de modal |
| **header-2** | 12px | 600 | Título de seção (InspectorSection) |
| **body** | 11px | 400 | Valores, labels secundários |
| **caption** | 10px | 500 | Atalhos, tooltips, status |
| **micro** | 9px | 600 | Badges, contadores |

### 4.3 Consistent border strategy

**Regra**: nunca dois borders adjacentes (double-border artefact).

- **Seções dentro do inspector**: `border: none`, separação por `spacing`
- **Inspector container**: `border: 1px`, `border-radius: 8px`
- **Floating panels**: `border: 1px accent`, `border-radius: 10px`, sombra
- **Modais/overlays**: `border: none`, sombra forte

---

## Matriz: Slint Nativo vs Extensão Necessária

| Feature | Slint Nativo | Extensão Rust | Extensão Slint (componente) |
|---|---|---|---|
| Animate height (seções) | ✅ | | |
| Animate x/y (floating) | ✅ | | |
| Hover transitions | ✅ | | |
| Spring easing | ✅ | | |
| Drag to reposition | ✅ | | |
| Drag to reorder | ✅ (1.17) | | |
| Box-shadow | ❌ | | ✅ `PanelShadow` |
| Z-order management | ❌ | ✅ `FloatingPanelManager` | |
| Backdrop blur | ❌ | ❌ (impossível sem render hook) | ⚠️ Simular com opacity |
| Tab tearing (undock to OS window) | ❌ | ✅ Multi-window API | |
| Smooth scroll momentum | ⚠️ Parcial | ✅ Custom `ScrollView` | |
| Color picker completo | ❌ | | ✅ Componente novo |
| Gradient backgrounds | ❌ | | ⚠️ Imagem/SVG |
| Custom cursor shapes | ⚠️ Parcial | ✅ `set_cursor` via API | |

---

## Priorização sugerida

### Fase 1 — Quick wins (1-2 dias)
1. ✅ `animate height` em `InspectorSection`
2. ✅ `animate background/border` em todos os componentes interativos
3. ✅ Suavizar cores dos eixos (transparentize 25%)
4. ✅ Separadores de grupo na tool shelf
5. ✅ Tokens de elevação/shadow em `tokens.slint`
6. ✅ Remover double-borders entre seções adjacentes

### Fase 2 — Floating panels (3-5 dias)
7. `FloatingPanel` component Slint
8. `FloatingPanelManager` em Rust (z-order, docking state)
9. `PanelShadow` component
10. Migrar InspectorSections para FloatingPanel

### Fase 3 — Polish profundo (1 semana)
11. `QuickActionButton` com ícones e hover
12. Viewport bar pill container
13. Active tool sliding indicator
14. Typography scale enforcement
15. Color picker inline component

### Fase 4 — Extensões avançadas (futuro)
16. Tab tearing (multi-window)
17. Custom scroll momentum
18. Backdrop blur (requer render hook wgpu)

> [!IMPORTANT]
> Nenhuma dessas mudanças altera a arquitetura de domínio (core, commands, project). Tudo é contido em `crates/ui-slint/` — superfície visual pura. Os contratos do caderno (cap. 36) são preservados: Parts → Transform → Material → Object como ordem semântica, apenas a forma de apresentação evolui.

---

## Decisões que preciso de você

1. **Painéis flutuantes**: devem poder ser arrastados para **qualquer lugar da tela** (free-floating) ou apenas **reordenados dentro do rail direito** (reorder-only)?
2. **Sombras**: implementar como `PanelShadow` multi-camada (visual OK, sem blur real) ou investir em uma imagem 9-patch (visual melhor, mais trabalho)?
3. **Quick Actions**: manter como lista vertical de botões ou mudar para **grid de ícones** (2 colunas, mais compacto)?
4. **Prioridade**: começar pela Fase 1 (quick wins visuais) ou ir direto para Fase 2 (floating panels)?
