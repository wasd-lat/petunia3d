# Análise Profunda: Duas Interfaces de Modelagem 3D

## Identificação das Interfaces

| | Interface A | Interface B |
|---|---|---|
| **Aplicação** | App de CAD/modelagem moderna (estilo Plasticity) | Cinema 4D (C4D) |
| **Paradigma** | Modelagem direta/sólidos (B-Rep/NURBS) | Modelagem poligonal + materiais |
| **Era de design** | ~2023–2025, design system contemporâneo | ~2018–2022, UI legacy evoluída |
| **Modelo exibido** | Robô esférico sci-fi (modelo complexo) | Cubo simples com material azul |

---

## 1. Layout e Estrutura Espacial

### Interface A — Modernista, Viewport-First

```
┌─────────┬──────────────────────────────┬──────────┐
│ Sidebar │        VIEWPORT              │ Inspector│
│ (Layers)│   (dominante, ~65% da tela)  │(Props)   │
│         │                              │          │
│         │                              │          │
├─────────┴──────────────────────────────┴──────────┤
│              Command Bar + Tool Strip              │
└────────────────────────────────────────────────────┘
```

**Pontos fortes:**
- **Viewport dominante**: o modelo ocupa a porção máxima da tela. O viewport é inegavelmente o protagonista.
- **Hierarquia clara de 3 colunas**: sidebar esquerda (outliner), viewport central, inspector direito. Padrão F-scan natural.
- **Bottom bar funcional**: command palette + ferramentas de criação formam um dock inferior que não compete com o viewport.
- **Respiro visual**: há generoso espaço negativo; os painéis não sufocam.

**Pontos fracos:**
- **Inspector raso**: mostra apenas Location/Rotation/Scale e Construction Planes — informação mínima. Para um modelo tão complexo, falta profundidade (materiais, componentes, history).
- **Sidebar com scroll extenso**: "Curve" lista 10+ itens com nomes genéricos (Nano Space, Quantum Leap, etc.) sem agrupamento hierárquico real — é uma lista plana com ícones homogêneos.

### Interface B — Clássica, Information-Dense

```
┌────────────────────────────────────────────────────┐
│  Menu Bar (View, Cameras, Display, Options...)     │
├──┬─────────────────────────────────────┬───────────┤
│T │         VIEWPORT                    │ Objects   │
│o │                                     │ Attributes│
│o │                                     │ (Material │
│l │                                     │  Editor)  │
│b │                                     │           │
│a │                                     │           │
│r │                                     │           │
├──┴─────────────────────────────────────┼───────────┤
│  Timeline + Properties Bar             │           │
│  Material Swatches + Transform Fields  │           │
└────────────────────────────────────────┴───────────┘
```

**Pontos fortes:**
- **Densidade informacional**: tudo visível de uma vez — Objects, Attributes, Material com color picker completo (RGB, HSV, hex), model settings, texture, mix mode.
- **Material editor integrado**: preview esférico + picker espectral + sliders individuais. Workflow completo sem abrir janela separada.
- **Transform bar inferior**: Position, Size, Rotation com campos editáveis e unidades visíveis. Acesso direto sem navegar ao inspector.

**Pontos fracos:**
- **Viewport comprimido**: o viewport perde ~40% do espaço horizontal para o painel direito e ~25% vertical para toolbar + timeline. O modelo vive "apertado".
- **Sobrecarga cognitiva**: há ~15 tipos diferentes de controles visíveis simultaneamente (sliders, dropdowns, swatches, campos numéricos, botões, tabs, checkboxes, trees, thumbnails). O olho não sabe onde pousar.
- **Toolbar vertical esquerda com ícones ambíguos**: ícones monocromáticos sem label. Sem hover state visível, sem agrupamento semântico claro.

---

## 2. Sistema Visual e Linguagem de Design

### Interface A

| Aspecto | Avaliação |
|---|---|
| **Tema** | Dark puro (#1a1a1a–#222), alto contraste com grid sutil |
| **Tipografia** | Sans-serif geométrica, peso consistente, tamanho legível |
| **Ícones** | Outline style, monocromáticos, stroke consistente |
| **Cores de acento** | Verde vibrante (avatar), laranja/teal (tags de command), amarelo (seleção gizmo) |
| **Bordas/separadores** | Invisíveis ou 1px com opacidade baixa — bordas por espaçamento |
| **Consistência** | Alta — poucos padrões visuais, todos repetidos |

**Destaques de design:**
- O **context menu flutuante** (Slice / Union / Difference / New Body / Radius) é elegante: fundo escuro semitransparente, atalhos visíveis, posicionado próximo ao ponto de ação.
- A **command bar** inferior com tags coloridas (Bridge, Split, Boolean) funciona como action chips — discoverable e clicável.
- O **gizmo de manipulação** é limpo: pontos brancos com borda amarela, linhas finas, sem poluição visual.

**Críticas de design:**
- Os **radio buttons** na sidebar (●) são muito pequenos para touch e marginal para mouse — violam Fitts's Law.
- A **barra de status inferior** (X/Y/Z com badges vermelhos/verdes + Grid Snap / Object Snap) mistura muita informação em pouco espaço com contraste insuficiente.
- Falta **feedback de estado**: qual ferramenta está ativa? Qual modo de seleção? Não há indicador claro.

### Interface B

| Aspecto | Avaliação |
|---|---|
| **Tema** | Dark cinza médio (#3a3a3a–#4a4a4a), menor contraste |
| **Tipografia** | System font, tamanhos variados, alinhamento irregular |
| **Ícones** | Mistura de estilos: outline, filled, multicoloridos, tamanhos variados |
| **Cores de acento** | Laranja (seleção, wireframe, botões), azul (material), verde (play) |
| **Bordas/separadores** | Linhas finas 1px explícitas entre todas as seções |
| **Consistência** | Média — acúmulo histórico de padrões |

**Destaques de design:**
- O **color picker** é excelente em funcionalidade: SV square + hue bar + RGB/HSV sliders + hex input — workflow completo.
- **Material swatches** na bottom bar oferecem acesso rápido visual — o olho humano identifica materiais por aparência mais rápido que por nome.
- O **object list** com ícones de visibilidade/editabilidade por objeto é um padrão consolidado e eficiente.

**Críticas de design:**
- **Poluição de toolbar**: a top bar tem ~20 ícones com estilos, tamanhos e cores inconsistentes. O ícone de "ProRender" é colorido, os de transformação são outline, os de snap são preenchidos — caos visual.
- A **tab system** (Basic / Color / Reflectense / Illumination / Assign) usa texto pequeno sem ícone — difícil de scanear.
- O **campo hex** (0076FF) está desalinhado do color picker e usa formatação diferente dos campos RGB abaixo.

---

## 3. Arquitetura de Informação

### Profundidade vs. Amplitude

| Dimensão | Interface A | Interface B |
|---|---|---|
| **Amplitude** (features visíveis) | Baixa (~15 controles) | Alta (~50 controles) |
| **Profundidade** (níveis de navegação) | Média (context menus) | Baixa (tudo exposto) |
| **Progressive disclosure** | ✅ Forte — mostra o mínimo, revela por contexto | ❌ Fraco — expõe tudo sempre |
| **Findability** | Depende de memorizar atalhos/commands | Visual scanning, mas overwhelm |

**Interface A** aposta em **disclosure progressivo**: o menu contextual aparece no ponto de uso; a command bar sugere ações; o inspector mostra só o relevante ao tipo selecionado. O risco é que o usuário não descubra features que existem.

**Interface B** aposta em **exposição total**: tudo está visível. O risco é que o usuário se perca na densidade e não encontre o que precisa entre tanto ruído.

### Nomenclatura e Vocabulário

| | Interface A | Interface B |
|---|---|---|
| **Operações** | Slice, Union, Difference, Bridge, Split, Boolean | Create, Edit, View, Select, Material, Texture |
| **Propriedades** | Location, Rotation, Scale, Radius, Construction Planes | Position, Size, Rotation, Color, Brightness, Roughness |
| **Organização** | Solids, Curve | Objects, Attributes, Mode |

A Interface A usa **vocabulário de CAD** (Union, Difference, Slice — termos de operações booleanas). A Interface B usa **vocabulário mais genérico** (Create, Edit, Size).

> [!NOTE]
> Para o Petunia3D, o caderno canônico (cap. 13) define vocabulário próprio: **Fuse** (não Union), **Cut** (não Difference), **Round Edge** (não Bevel), **Point** (não Vertex). A Interface A está mais próxima do padrão CAD, mas nenhuma usa o vocabulário Petunia.

---

## 4. Padrões de Interação

### Manipulação Direta

| Padrão | Interface A | Interface B |
|---|---|---|
| **Gizmo 3D** | ✅ Visível no modelo, pontos de controle | ✅ Arrows triaxiais clássicos (RGB=XYZ) |
| **Seleção visual** | Outline amarelo nos pontos | Outline laranja no wireframe |
| **Feedback de hover** | Não evidente | Não evidente |
| **Drag direto** | Aparente nos pontos de controle | Aparente nos handles do gizmo |

A Interface A tem um **gizmo minimalista** (pontos brancos conectados por linhas), adequado para edição de topologia. A Interface B tem o **gizmo clássico de transformação** (setas coloridas XYZ), adequado para translate/rotate/scale.

### Command Discovery

**Interface A** usa um modelo **hybrid command palette**: a barra inferior com "🔍 Command" é uma searchable command palette (padrão VS Code / Spotlight). Isso favorece usuários avançados que sabem o que querem.

**Interface B** usa **menu bar tradicional** (Create / Edit / View / Select / Material / Texture). Isso favorece exploração sequencial e é mais acessível para iniciantes.

### Atalhos de Teclado

A Interface A **expõe atalhos no context menu** (Shift+Q, Q, Shift+E, N, O) — excelente para aprendizado gradual. A Interface B **não mostra atalhos na interface visível** — requer memorização ou documentação externa.

---

## 5. Críticas de UX Específicas

### Interface A — 7 problemas identificados

1. **Hit targets minúsculos**: os indicadores ● na sidebar têm ~8px — abaixo do mínimo recomendado de 24px (WCAG 2.5.8).
2. **Contraste insuficiente na status bar**: texto cinza sobre fundo cinza-escuro na barra inferior.
3. **Sem breadcrumb de contexto**: não é claro em que modo o usuário está (modelo, face, edge, point?).
4. **Grid snap toggles sem affordance**: "Grid Snap" e "Object Snap" parecem labels, não botões toggle. O estado ativo/inativo não é diferenciável o suficiente.
5. **Nenhum indicador de ferramenta ativa**: das ~15 ferramentas na bottom strip, qual está selecionada?
6. **Scroll na sidebar sem paginação**: a lista de Curves ultrapassa a viewport sem indicador de "mais itens abaixo".
7. **Inspector vazio demais**: com um modelo complexo selecionado, mostrar apenas transform + construction planes é subutilizar o painel.

### Interface B — 7 problemas identificados

1. **Viewport claustrofóbico**: o modelo ocupa menos de 30% da área total da janela.
2. **Inconsistência de ícones**: toolbar superior mistura pelo menos 3 estilos iconográficos diferentes.
3. **Text tabs sem scan visual**: "Basic / Color / Reflectense / Illumination / Assign" é difícil de scanear — ícones ajudariam.
4. **"Reflectense" é typo**: provavelmente deveria ser "Reflectance" — erro de UI shipping.
5. **Material swatches sem label hover**: os thumbnails na bottom bar têm labels cortados ("Black f", "Curtai") — truncamento sem tooltip.
6. **Painel direito não colapsável**: parece fixo, consumindo espaço mesmo quando não necessário.
7. **Dupla exposição de transform**: Position/Size/Rotation aparecem tanto no painel inferior quanto potencialmente no inspector — redundância.

---

## 6. Lições para o Petunia3D

Com base nesta análise e nos princípios do caderno canônico (caps. 23, 36):

### ✅ Adotar da Interface A

| Padrão | Razão | Alinhamento com caderno |
|---|---|---|
| **Viewport-first layout** | Modelagem 3D exige máximo espaço visual | Cap. 36: viewport ≥ 480×360 lógico |
| **Progressive disclosure** | Reduz carga cognitiva, mostra o relevante | Cap. 23: simplicidade progressiva |
| **Command palette** | Discovery + velocity para power users | Complementa o keymap system |
| **Context menus no ponto de uso** | Reduz distância do mouse (Fitts's Law) | — |
| **Atalhos visíveis nos menus** | Aprendizado gradual, zero wall | — |
| **Dark theme com alto contraste** | Padrão da indústria 3D, reduz fadiga | Cap. 36: dark oficial |

### ✅ Adotar da Interface B

| Padrão | Razão | Alinhamento com caderno |
|---|---|---|
| **Color picker completo inline** | Workflow de material sem janela separada | Workspace PAINT precisa disso |
| **Material swatches visuais** | Identificação visual é mais rápida que textual | Asset Library inferior (cap. 36) |
| **Object visibility toggles** | Controle direto por objeto no outliner | — |
| **Transform fields always-visible** | Feedback numérico constante durante edição | Inspector com Transform (cap. 36) |

### ❌ Evitar de ambas

| Anti-padrão | Por quê |
|---|---|
| Hit targets < 24px (A) | Violação de acessibilidade |
| Exposição total sem hierarquia (B) | Sobrecarga cognitiva |
| Ícones sem label e sem tooltip (ambas) | Barreira para novos usuários |
| Toolbar fixa não customizável (B) | Rigidez de workflow |
| Ausência de estado de modo visível (A) | Desorientação do usuário |
| Vocabulário de CAD puro (A) | Petunia usa vocabulário próprio (cap. 13) |

---

## 7. Síntese Comparativa

```
                    MINIMALISMO ◄──────────────────► DENSIDADE
                         
    Interface A ●────────────────────────────── · ·
    Interface B · · ──────────────────────────────● 
    
    Petunia3D (ideal) · · · · · · · · ●─── · · · · ·
                                    ▲
                          Progressive disclosure:
                          denso quando precisa,
                          limpo quando não precisa
```

A Interface A erra por **excesso de minimalismo** — esconde informação útil. A Interface B erra por **excesso de exposição** — mostra tudo indiscriminadamente. O Petunia3D deve buscar o ponto de equilíbrio: **layout limpo com disclosure progressivo**, onde a densidade aumenta conforme a profundidade de interação do usuário.

> [!IMPORTANT]
> O caderno canônico (cap. 36) já define a resposta arquitetural para isso: **barra de criação à esquerda, Inspector à direita com Parts → Transform → Material → Object, Asset Library inferior**. A Interface A valida o layout estrutural; a Interface B valida a necessidade de profundidade nos painéis de propriedades. Ambas informam, nenhuma é modelo a copiar.
