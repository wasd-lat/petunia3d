# Petunia3D V1 — Matriz de Status: Módulos de Modelagem e Pintura

> **Data da Auditoria:** 24 de Setembro de 2026 (Sprint 4: fechamento dos itens parciais MODEL/PAINT)  
> **Autoridade Normativa:** Livro Vivo ([`docs/bible/`](../bible/index.md) — Capítulos 02, 03, 04, 06, 12, 14, 15, 21, 36 e Especificações P3D)  
> **Frontend de Produção:** `petunia_ui_slint` (`crates/ui-slint/`)  
> **Repositório:** `wasd-lat/petunia3d`

---

## 1. Resumo Executivo da Conformidade V1

| Módulo | Total de Funcionalidades | Implementado | Parcialmente | Não implementado |
| :--- | :---: | :---: | :---: | :---: |
| **Modelagem (MODEL)** | 45 | 43 (95.6%) | 0 (0.0%) | 2 (4.4%) |
| **Pintura, UV e Materiais (PAINT)** | 30 | 27 (90.0%) | 2 (6.7%) | 1 (3.3%) |
| **Total Consolidado** | **75** | **70 (93.3%)** | **2 (2.7%)** | **3 (4.0%)** |

### Critérios de Classificação:
- **Implementado**: Algoritmo completo no Core/Mesh e acessível/funcional no frontend de produção Slint.
- **Parcialmente**: Funcional no Core/Mesh ou legado egui, mas com bridge/gestos/UI parcial ou pendências abertas nas matrizes de paridade do Slint.
- **Não implementado**: Decidido no escopo ou adiado oficialmente para V1.x/Era 1 pós-GA, sem implementação no shell.

---

## 2. Módulo de Modelagem (MODEL)

O escopo de modelagem da V1 segue a diretiva **Shape-First** ([Capítulo 02](../bible/foundations/02-workflow-modelagem-shape-first.md)), com vocabulário canônico do usuário (*Point, Round Edge, Fuse, Cut, Connect, Keep Parts, Join* — [Capítulo 13](../bible/foundations/13-contrato-geracao-documentacao.md)).

| # | Funcionalidade | Referência Normativa | Status | Evidência / Estado no Código e UI Slint |
| :---: | :--- | :--- | :---: | :--- |
| **1** | **Seleção de Vértices (Point Select)** | [P3D-017](../bible/specs/p3d-017-vertex-select.md) | **Implementado** | Discos visuais com hover e highlight nos renderizadores WGPU e software; clique e Shift+clique em `crates/core/src/selection.rs`. |
| **2** | **Seleção de Arestas (Edge Select)** | [P3D-018](../bible/specs/p3d-018-edge-select.md) | **Implementado** | Picking de arestas por raycast, realce visual em traço espesso e toggle múltiplo com Shift. |
| **3** | **Seleção de Faces (Face Select)** | [P3D-019](../bible/specs/p3d-019-face-select.md) | **Implementado** | Picking baricêntrico de faces poligonais sem dots centrais poluidores ([rodada 2 do Inspector](model-inspector-viewport-gap-matrix.md)). |
| **4** | **Seleção de Objeto (Object Select)** | [P3D-020](../bible/specs/p3d-020-object-selection.md) | **Implementado** | Seleção de assets inteiros na viewport e sincronização bidirecional imediata com o Outliner/Parts. |
| **5** | **Seleção Retangular (Box Select)** | [P3D-020](../bible/specs/p3d-020-object-selection.md) | **Implementado** | Arraste de retângulo na viewport (`pending-box-select`), selecionando elementos contidos ou cruzados. |
| **6** | **Seleção por Laço (Lasso Select)** | [P3D-020](../bible/specs/p3d-020-object-selection.md) | **Implementado** | Polígono 2D com detecção de arestas cruzadas, filtro de oclusão por raycast (X-Ray/Wireframe atravessam) e ferramenta na shelf; verificado por `lasso_selection_occlusion_and_xray` em `crates/ui-slint/src/lib.rs`. QA com dispositivo físico segue pendente. |
| **7** | **Seleção de Loop de Arestas (Loop Select)** | [Capítulo 03](../bible/foundations/03-geometry-core-faces-topologia.md) | **Implementado** | `LoopRing::discover` e `select_edge_loop` integrados com Alt+clique e Shift+Alt+clique no Slint via `select_viewport_ext(..., loop_select: true)` com feedback de status e testes automatizados. |
| **8** | **Move / Translação** | [P3D-021](../bible/specs/p3d-021-move.md) | **Implementado** | Atalho `G`, arrasto no mesmo gesto, campos numéricos transacionais e suporte a Undo/Redo no core. |
| **9** | **Rotate / Rotação** | [P3D-022](../bible/specs/p3d-022-rotate.md) | **Implementado** | Atalho `R`, anel circular no gizmo, rotação em graus em torno do eixo da câmera ou eixos travados. |
| **10** | **Scale / Escala** | [P3D-023](../bible/specs/p3d-023-scale.md) | **Implementado** | Atalho `S`, escala uniforme ou por eixos individuais com campos numéricos proporcionais. |
| **11** | **Universal Transform (Gizmo Combinado)** | [P3D-024](../bible/specs/p3d-024-universal-transform.md) | **Implementado** | Gizmo combinado triaxial no Slint com priorização espacial (Scale e Rotate com deadzone de 12px sobre o shaft de Move) para eliminação de ambiguidades de clique. |
| **12** | **Restrições de Eixo e Plano (Axis/Plane Locking)** | [P3D-025](../bible/specs/p3d-025-axis-constraints.md) | **Implementado** | Teclas `X`, `Y`, `Z` para eixos e `Shift+X/Y/Z` para planos coordenados, com feedback visual nos gizmos. |
| **13** | **Orientação Global vs Local** | [P3D-026](../bible/specs/p3d-026-transform-orientation.md) | **Implementado** | Alternância no header da viewport e reorientação imediata dos eixos locais da malha ativa. |
| **14** | **Sistema de Pivô (Pivot System)** | [P3D-027](../bible/specs/p3d-027-pivot-system.md) | **Implementado** | Quatro modos no Core com selector no header da viewport; `Individual Origins` verificado em modo objeto e componente (escala preserva centroides por peça). Active Element segue fora da enum/spec aprovada. |
| **15** | **Feedback Modal (HUD, Guias e Eixos)** | [P3D-131](../bible/specs/p3d-131-modal-tool-feedback-system.md) | **Implementado** | Linhas infinitas coloridas nos eixos X/Y/Z, linha-guia pontilhada pivô-ponteiro e HUD flutuante no viewport Slint. |
| **16** | **Primitivas Básicas (Cubo, Cilindro, Esfera, Icoesfera, Cone, Plano)** | [P3D-028](../bible/specs/p3d-028-add-primitives.md) | **Implementado** | Geradores low-poly em `crates/mesh/src/primitives.rs` com botões dedicados na barra lateral esquerda da UI Slint. |
| **17** | **Primitiva Cápsula (Capsule)** | [P3D-028](../bible/specs/p3d-028-add-primitives.md), [Capítulo 06](../bible/foundations/06-escopo-essencial.md) | **Implementado** | Adicionada ao Core V1 (`Mesh::capsule`) e disponível na barra de criação do Slint. |
| **18** | **Reedição Paramétrica Pós-Criação de Primitivas** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md) | **Não implementado** | `PrimitiveDescriptor` só existe no momento da criação e não é persistido no Asset; requer contrato pós-GA de proveniência/re-geração. |
| **19** | **Desenho de Perfil 2D (Draw Profile: Polyline, Retângulo, Círculo)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Polylines, fechamento e presets Retângulo/Círculo (`add-profile-rectangle`, `add-profile-circle`) com geração por Extrude/Revolve respeitando o frame do perfil. |
| **20** | **Extrude (Extrusão de Faces e Arestas)** | [P3D-029](../bible/specs/p3d-029-extrude.md) | **Implementado** | Atalho `E`, preview interativo contínuo ao longo da normal da face e gravação em transação única. |
| **21** | **Multi-Extrude / Extrude Individual Faces** | [P3D-030](../bible/specs/p3d-030-multi-extrude.md) | **Implementado** | Atalho `Alt+E`, extrusão individual por face gerando topos disjuntos sem soldar paredes vizinhas. |
| **22** | **Push / Pull (Deslocamento Topológico Local)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Operação topológica local em faces planas coplanares sem recorrer a booleano; exposta na shelf do Slint. |
| **23** | **Inset (Inserção de Faces Internas)** | [P3D-031](../bible/specs/p3d-031-inset.md) | **Implementado** | Atalho `I`, preview com mouse e amortecimento métrico step-down para evitar inversão de normais. |
| **24** | **Bevel / Round Edge (Chanfro 1 segmento Core V1)** | [P3D-032](../bible/specs/p3d-032-bevel.md), [Capítulo 14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Atalho `Ctrl+B`, chanfro determinístico de 1 segmento em arestas manifold (multi-segment preservado como capacidade interna). |
| **25** | **Knife Cut (Corte por Faca Segmentada)** | [P3D-033](../bible/specs/p3d-033-knife-cut.md) | **Implementado** | Atalho `K`, traçado de pontos de corte em arestas e faces, cancelamento por `Esc` e commit por `Enter`. |
| **26** | **Slice (Corte por Plano Infinito)** | [Capítulo 14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Atalho `Shift+K`; preview laser animado dinâmico em `drag_link_commands` projetado no viewport com corte atômico no release preservando ambos os lados e Undo transacional. |
| **27** | **Loop Cut & Slide (Corte em Anel de Quads)** | [P3D-034](../bible/specs/p3d-034-loop-cut.md) | **Implementado** | Descoberta do anel por hover, preview visual, scroll do mouse para contagem de loops, slide modal e commit. |
| **28** | **Subdivide (Subdivisão Simples)** | [P3D-035](../bible/specs/p3d-035-subdivide.md) | **Implementado** | `SubdivideSelectionCmd` divide arestas e quads/triângulos mantendo topologia manifold e Undo limpo. |
| **29** | **Revolve 360° (Revolução Procedural de Perfil)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Revolução completa de perfis conectados ao redor de um eixo com contagem de passos radiais; integrado à ferramenta Profile. |
| **30** | **Merge / Weld (Fusão de Vértices)** | [P3D-036](../bible/specs/p3d-036-merge.md) | **Implementado** | Fusão nos modos At Center, At First/Last, Collapse e Weld com tolerância de distância pequena. |
| **31** | **Split / Separate (Desconexão e Separação de Peças)** | [P3D-037](../bible/specs/p3d-037-split-separate.md) | **Implementado** | Atalho `P` para separar faces selecionadas em um novo objeto ou separar partes desconexas (*Loose Parts*). |
| **32** | **Mirror Simples (Espelhamento Planar X/Y/Z)** | [P3D-038](../bible/specs/p3d-038-mirror-simples.md) | **Implementado** | Espelhamento simétrico destrutivo V1 (`SymmetrizeCmd`) com costura na linha central (mirror live não-destrutivo é Era 1.5). |
| **33** | **Proportional Editing (Edição Proporcional)** | [P3D-039](../bible/specs/p3d-039-proportional-editing.md) | **Implementado** | Falloff aplicado no commit modal (`crates/core/src/modal.rs`); toggle `O`, ajuste de raio pela roda, seletor de falloff e teste `proportional_editing_radius_adjustment_and_falloff` no Slint. |
| **34** | **Sistema de Snapping (Grade e Elementos)** | [P3D-040](../bible/specs/p3d-040-sistema-de-snap.md) | **Implementado** | `snap_point` (Grid, Increment, Vertex, Edge, Face) aplicado ao arrasto modal de Move, incremento de 15° no Rotate e 0.1 no Scale; toggle `Shift+Tab` e seletor de alvo na viewport. |
| **35** | **Dissolve (Remoção Segura de Vértices e Arestas)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Remoção de edges/points coplanares sem destruir as faces adjacentes (`crates/mesh/src/ops.rs`). |
| **36** | **Fill (Preenchimento de Boundaries e Furos)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Atalho `F`; fecha loops abertos de arestas usando triangulação determinística sem criar non-manifold. |
| **37** | **Combine: Keep Parts e Join** | [Capítulo 04](../bible/foundations/04-combine-fuse-weld-personagens.md), [12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | `JoinObjectsCmd` agrupa malhas selecionadas em um único ativo preservando suas geometrias originais. |
| **38** | **Combine: Fuse (União Booleana 3D)** | [Capítulo 04](../bible/foundations/04-combine-fuse-weld-personagens.md), [12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | União booleana volumétrica via provider robusto `manifold-rust` (`crates/mesh/src/boolean.rs`). |
| **39** | **Cut / Boolean Difference (Corte Booleano 3D)** | [Capítulo 14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Subtração volumétrica usando objeto cortador via provider booleano para cortes que atravessam superfícies. |
| **40** | **Connect (Bridge Topológico entre Faces/Loops)** | [Capítulo 04](../bible/foundations/04-combine-fuse-weld-personagens.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Implementado** | Conexão de continuidade topológica entre boundaries com quads e triângulos de transição (`crates/mesh/src/connect.rs`). |
| **41** | **Inspeção de Triangulação (Show Triangulation)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | Wireframe determinístico de diagonais internas renderizado com profundidade e toggle na viewport. |
| **42** | **Inversão de Diagonais (Flip Diagonal)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | `FlipDiagonalCmd` inverte a triangulação de quads para controle explícito de silhueta low-poly. |
| **43** | **Orientação de Normais e Recálculo** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | `FlipNormalsCmd`/`recalculate_normals` no Core e overlay Face Orientation (frente azul / verso vermelho) no WGPU e no fallback software, com toggle no popover de shading. Backend GL legado não renderiza o overlay (só invalida cache). |
| **44** | **Contadores de Geometria (Verts, Faces, Tris, Objs)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [36](../bible/foundations/36-ui-baseline-temas-plugin-panels.md) | **Implementado** | Exibição discreta em tempo real no footer da aplicação e na seção de objeto do Inspector direito. |
| **45** | **Simple Sweep (Profile + Path)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [14](../bible/foundations/14-modelagem-v1-cut-slice-bevel-revolve-sweep.md) | **Não implementado** | Fechado canonicamente no Livro Vivo como **Official Extension / V1.x** (fora do Core V1 obrigatório). |

---

## 3. Módulo de Pintura, UV e Materiais (PAINT)

O módulo Paint cobre a pintura 3D direta, editor 2D, pilha de camadas, desdobramento UV integrado e materiais PBR glTF ([Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md)).

| # | Funcionalidade | Referência Normativa | Status | Evidência / Estado no Código e UI Slint |
| :---: | :--- | :--- | :---: | :--- |
| **1** | **Pixel Brush (Pincel Rígido)** | [P3D-056](../bible/specs/p3d-056-pixel-brush.md) | **Implementado** | Pincel rígido para estética pixel art/low-poly sem interpolação; tamanho em px e atalho funcional. |
| **2** | **Soft Brush (Pincel Suave)** | [P3D-057](../bible/specs/p3d-057-soft-brush.md) | **Implementado** | Atenuação circular quadrática com hardness e size configuráveis em `crates/module-paint/src/lib.rs`. |
| **3** | **Airbrush (Aerógrafo Contínuo)** | Iniciativa Paint 2026-09-16, [Cap. 44](../bible/foundations/44-pos-v1-surface-paint-toolbox.md) | **Implementado** | Botão Airbrush na shelf PAINT, `BrushType::Airbrush`, acúmulo contínuo de dabs via timer de 50ms (`airbrush_tick`) com modulação de fluxo e opacidade progressiva no 3D e 2D. |
| **4** | **Eraser (Borracha de Pintura)** | [P3D-058](../bible/specs/p3d-058-eraser.md) | **Implementado** | Reduz ou apaga o canal alfa da camada ativa; funcional na viewport 3D e na camada base. |
| **5** | **Flood Fill (Balde de Preenchimento)** | [P3D-059](../bible/specs/p3d-059-fill.md) | **Implementado** | Preenchimento com tolerância de cor, suportando escopo de pixels contíguos (`ConnectedPixels`) e camada global. |
| **6** | **Color Picker / Eyedropper (Conta-gotas)** | [P3D-060](../bible/specs/p3d-060-color-picker.md) | **Implementado** | Captura cor de texels da textura ou cor de vértices da malha e atualiza instantaneamente o pincel ativo. |
| **7** | **Formas Raster (Linha e Retângulo)** | [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Ferramentas Line e Rectangle na shelf PAINT do Slint, ancoragem no press, preview do traço e confirmação no release via transação única de Undo. |
| **8** | **Pintura 3D sobre a Superfície (Paint on Model)** | [P3D-062](../bible/specs/p3d-062-paint-de-mapas-via-uv.md) | **Implementado** | Raycast do mouse na malha + coordenadas baricêntricas projetadas para UV0; atualiza ao vivo o bitmap WGPU. |
| **9** | **Isolamento por Seleção / Paint Masks** | [P3D-132](../bible/specs/p3d-132-paint-masks-face-selection-isolation.md) | **Implementado** | `isolate_selection` restringe a aplicação do traço do pincel exclusivamente às faces selecionadas da malha. |
| **10** | **Transação Única por Pincelada (Stroke-as-Transaction)** | [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md), [P3D-041](../bible/specs/p3d-041-undo-redo.md) | **Implementado** | O traço completo do mouse entre o pressionar e soltar grava exatamente um snapshot no histórico de Undo. |
| **11** | **Editor 2D de Textura (Paint 2D Canvas)** | [Capítulo 12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Canvas 2D interativo com `TouchArea`, `paint_2d_stroke` com interpolação contínua de traço para todos os pincéis (Pixel, Soft, Airbrush, Eraser, Fill, Picker) e Undo/Redo no stack. |
| **12** | **Pixel Grid Contextual** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Zoom de 1x a 16x com controles interativos (- / + / Nx), toggle de grid e renderização de texels com delimitações sutis quando zoom >= 4x (`render_paint_canvas`). |
| **13** | **Pilha de Camadas (PaintLayerStack)** | [P3D-061](../bible/specs/p3d-061-simple-paint-layers.md) | **Implementado** | Adição de camada, exclusão, reordenação (up/down), visibilidade (`👁`), lock (`🔒`) e slider de opacidade. |
| **14** | **Modos de Mesclagem (Blend Modes)** | [P3D-061](../bible/specs/p3d-061-simple-paint-layers.md) | **Implementado** | Modos canônicos suportados: `Normal`, `Multiply`, `Add` e `Screen` com composição atômica por tile/canvas. |
| **15** | **Decal & Projection Layers** | [P3D-133](../bible/specs/p3d-133-decal-projection-layers.md) | **Parcialmente** | `DecalLayer` com UV, escala, rotação e opacidade, criação pelo botão Add Decal e composição na pilha; manipulador 3D de posicionamento na superfície segue previsto para a Era 1 pós-GA. |
| **16** | **Paint Effect Stack (Pilha de Efeitos)** | [P3D-134](../bible/specs/p3d-134-paint-effect-stack.md) | **Implementado** | Efeitos não-destrutivos integrados à composição de camadas: `Pixelate`, `Posterize` e `Invert`, com controles no painel. |
| **17** | **Paleta de Cores e Cores Recentes** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Cores salvas no projeto e lista das últimas 16 cores usadas com swatches rápidos na UI Slint. |
| **18** | **Importação e Exportação de Paletas (.gpl / .hex)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Diálogos de importação e exportação de paletas GPL no Slint (`import-palette`, `export-palette`), atualizando a paleta ativa e sincronizando swatches com Undo/Redo. |
| **19** | **Auto UV - Projeção Planar** | [P3D-064](../bible/specs/p3d-064-uv-editing-basico.md), [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | `Mesh::project_planar` projeta as coordenadas ao longo do plano da face ou eixo coordenado. |
| **20** | **Auto UV - Projeção Cúbica (Box Mapping)** | [P3D-064](../bible/specs/p3d-064-uv-editing-basico.md), [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | `Mesh::project_cube` mapeia as 6 faces cardeais em ilhas independentes sem distorções angulares. |
| **21** | **Auto UV - Desdobramento via xatlas** | [P3D-064](../bible/specs/p3d-064-uv-editing-basico.md), [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Integração completa com a crate `xatlas-rs-v2` (`UnwrapAutoCmd`), gerando charts e packing automático para malhas arbitrárias. |
| **22** | **Project From Reference / View** | [Capítulo 08](../bible/foundations/08-photo-projection-trace-project.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Comandos `uv.project_reference` e `paint.bake_reference` registrados, botões na shelf e projeção/bake da imagem de referência na textura ativa com UndoStack. |
| **23** | **Transformações de Ilhas UV (Move, Scale, Rotate 90°)** | [P3D-064](../bible/specs/p3d-064-uv-editing-basico.md), [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Parcialmente** | Move/Scale/Rotate incremental e botões dedicados de 90° CW/CCW no Slint; workspace UV dedicado segue condensado no shell seguindo o fluxo Paint-first (`decisão 2026-09-16`). |
| **24** | **Costuras UV Manuais (UV Seams - Mark / Clear)** | [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Comando `uv.toggle_seam`, atalho `U` / Mark Seam na shelf UV, renderização de arestas de costura com cor laranja vibrante no shader e Undo/Redo atômico. |
| **25** | **Empacotamento de Ilhas UV (UV Packing)** | [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | `UvPackIslandsCmd` empacota as ilhas sem sobreposição com padding configurável em texels. |
| **26** | **Diagnóstico de Distorção UV (Checker & Stretch)** | [Capítulo 06](../bible/foundations/06-escopo-essencial.md), [15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | `UvDiagnostics` (área zero, sobreposição, stretch) e overlay Checkerboard procedural ativado direto na viewport (WGPU + software) via `view.toggle_uv_checker` no popover de shading. |
| **27** | **Sistema de Materiais PBR glTF Metallic-Roughness** | [P3D-050](../bible/specs/p3d-050-material-system.md), [Capítulo 12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | Modelo `Material` com Base Color, Metallic, Roughness, Normal, Emissive e Alpha Mode integrado ao Inspector direito e ao WGPU. |
| **28** | **Shader Profiles V1 (Pbr, Unlit, Toon, Glass, Emissive)** | [P3D-140](../bible/specs/p3d-140-material-shader-profiles-effects.md), [Capítulo 12](../bible/foundations/12-baseline-funcional-roadmap-escopo.md) | **Implementado** | Perfis de sombreamento selecionáveis no Inspector de Material e vinculados ao pipeline de renderização. |
| **29** | **Atribuição de Múltiplos Materiais por Face (Material Slots)** | [P3D-050](../bible/specs/p3d-050-material-system.md), [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Implementado** | Seção Material Slots no Inspector Slint com lista interativa de slots, criação (New), duplicação (Duplicate), atribuição por face (Assign) e Undo/Redo. |
| **30** | **Symmetry Painting (Pintura Espelhada Simétrica)** | [Capítulo 15](../bible/foundations/15-uv-textura-projecao-materiais.md) | **Não implementado** | Classificado explicitamente no Livro Vivo como **V1.x** (fora do Core V1 obrigatório de pintura). |

---

## 4. Diagnóstico Técnico e Próximos Passos Prioritários

1. **Modelagem:**
   - O núcleo geométrico em `crates/mesh/` e transacional em `crates/core/` está praticamente completo e verificado por testes de regressão automatizados.
   - O foco atual está registrado no [plano de trabalho do MODEL / viewport](workspace-inspector-implementation-plan.md): polimento da ergonomia dos gestos de modal sem clique adicional, acabamento do preview do Slice e consolidação do Inspector direito com a ordem canônica aprovada em 2026-09-23 (`Parts → Transform → Material → Object`).

2. **Pintura e UV:**
   - O motor raster em `crates/module-paint/` e projeção UV em `crates/module-uv/` já entregam camadas com blend modes, pintura 3D baricêntrica contínua, isolamento de seleção, xatlas e efeitos base.
   - Conforme a decisão de produto de 2026-09-16 registrada em ``PROJECT_STATE.md``, o fluxo prioritário é **Paint-first**: consolidar o editor 2D integrado e o descriptor unificado de pincéis antes de reabrir o redesign visual do workspace UV independente.
