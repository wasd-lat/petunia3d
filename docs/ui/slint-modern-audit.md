# Auditoria inicial — interface Slint experimental

## Escopo

Branch: `experiment/slint-modern-ui`  
Base: `58afca78999ac1874d8b99526e47611c802c6047`  
Data da auditoria: 2026-09-20

A branch mantém a UI egui e o app existentes intactos. O primeiro slice adiciona uma crate paralela, `petunia_ui_slint`, com shell declarativo e bridge de intents.

## Implementation-vs-Spec Gap Matrix

| Requisito da experiência | Estado | Evidência / próximo passo |
|---|---|---|
| Branch experimental separada | `COMPLIANT` | `experiment/slint-modern-ui`; `main` e `backup/main-before-slint` continuam no commit-base |
| Slint pinado na linha 1.18 | `COMPLIANT` | `slint`/`slint-build` em `~1.18`, `compat-1-18` explícito |
| UI Slint isolada da egui | `COMPLIANT` | nova crate `crates/ui-slint`; nenhum arquivo produtivo egui alterado |
| Design tokens semânticos | `COMPLIANT` | `DesignTokens` dinâmico em `ui/app.slint` sincronizado com `ThemeRegistry` do Petunia3D (`petunia-dark` e `petunia-high-contrast`) via módulo `theme.rs` e seletor no modal Preferences |
| Lucide encapsulado | `PARTIALLY_COMPLIANT` | `IconSet`/`IconDisplay` são consumidos dentro de `RailButton`/`TopAction`; catálogo semântico completo ainda falta |
| slintcn como fonte copy-source | `DOCUMENTED` | `slintcn` é CLI/installer, não runtime; componentes serão copiados e adaptados, não dependidos em produção |
| Icon rail | `COMPLIANT` | rail com Scene, Tools (Model), Paint e UV com ícones Lucide |
| Viewport-first shell | `COMPLIANT` | viewport ocupa a região central; exibe renderização real 3D via GPU ou fallback informativo |
| Context inspector | `COMPLIANT` | painéis de contexto adaptados por workspace: `MODEL` (Transform com inputs numéricos vetoriais interativos Position, Rotation, Scale; Object com contagem real de vértices/faces; Material do ativo e botão Delete Active Object); `PAINT` (paleta interativa com 8 swatches, preview de cor ativa, controles de Brush Size e Brush Opacity, e lista de camadas de textura); `UV` (operações Unwrap Mesh, Pack Islands e estatísticas de densidade UV) |
| Contextual Tool Shelf | `COMPLIANT` | barra vertical de ferramentas (44px) entre a rail de navegação e a viewport; exibe botões com estados ativos e atalhos semânticos para cada workspace (`MODEL`: Select, Move, Add Cube, Add Sphere, Cut/Knife, Frame Selection; `PAINT`: Brush, Eraser, Picker, Fill Bucket; `UV`: UV Select, Unwrap, Pack Islands) |
| Selection Domain Switcher | `COMPLIANT` | alternador no topo do shell entre Object, Point (Vertex per vocabulário canônico AGENTS.md §3), Edge e Face; sincroniza com `AppState` e `PetuniaViewport` |
| Undo & Redo | `COMPLIANT` | botões de ação e atalhos integrados ao `ProjectContext::undo` do core; reflete `can_undo`/`can_redo` reativamente no shell |
| Scene Outliner / Drawer | `COMPLIANT` | Drawer lateral conectado a `state.project.assets` com exibição de malhas, triângulos, seleção interativa e toggles de visibilidade e bloqueio |
| Command Registry | `COMPLIANT` | catálogo puro conectado à Command Palette modal no Slint com busca contextual e execução de comandos (incluindo criação de primitivas Cube/Sphere/Cylinder/Plane, seleção, ferramentas de pintura e UV) |
| Overlay/ESC policy | `COMPLIANT` | `OverlayStack` integrado no bridge; gerencia overlays (Palette, Scene Drawer, Settings) em ordem LIFO com tecla ESC e click-away |
| Numeric field | `COMPLIANT` | componentes `NumericField` e `Vector3Field` no Slint integrados com `NumericFieldState` (scrubbing, fine-step com Shift, clamping e testes unitários) |
| FileDialogService | `COMPLIANT` | `FileDialogService` assíncrono com `rfd::AsyncFileDialog` integrado aos callbacks de Salvar e Abrir projeto; persiste e carrega via `petunia_project::format` sincronizando `saved` e status |
| Interação do Viewport | `COMPLIANT` | `TouchArea` da viewport Slint captura órbita (MMB / Alt+LMB), pan (Shift+MMB) e zoom (scroll wheel) despachando `ViewportGesture` para a câmera do `AppState` com re-renderização imediata |
| `PetuniaViewport` | `COMPLIANT` | trait unificada implementada por `PlaceholderViewport` e `WgpuViewport` |
| WGPU compartilhado Slint/viewport | `COMPLIANT` | `WgpuViewport` em `viewport_gpu.rs` acopla `petunia_render_wgpu` à textura off-screen e exporta para `slint::Image` via `unstable-wgpu-30` com fallback gracioso |
| viewport-lib | `OBSOLETE` como dependência padrão | `0.22.0` pinado somente na feature opcional por ser GPL-3.0-only; uso exige revisão de distribuição |
| Multi-renderer fallback / Ivy Bridge | `COMPLIANT` | Suporte a `renderer-femtovg` (OpenGL via EGL/Glutin) e `renderer-software` adicionados ao Slint; detecção de Intel Gen 7 (Ivy Bridge / Bay Trail) evita o driver Vulkan incompleto do Mesa e ativa `PlaceholderViewport` seguro; leitura de pixels WGPU exporta para `slint::SharedPixelBuffer` universal compatível com todos os backends |
| Viewer/LSP | `MISSING` localmente | binários não estão instalados; workflow está documentado, instalação manual permanece pendente |
| Screenshot/golden | `MISSING` | criar após shell visual estabilizar |
| Asset Library Drawer | `COMPLIANT` | Gaveta inferior expansível (conforme AGENTS.md §3: *"shell Parts-esquerda / Context-direita / Asset Library-abaixo"*); cards de assets com contagem de tris/verts, ações de seleção, duplicação e botão "Save Active as Asset" |
| Viewport HUD & Projeção | `COMPLIANT` | HUD integrado sobre o viewport com alternador de Projeção (Persp / Ortho), toggle de Wireframe e botão de Reset Camera; despacha comandos para `petunia_core` |
| Atalhos de Teclado Globais | `COMPLIANT` | FocusScope raiz com atalhos de produtividade: `Ctrl+Z` (Undo), `Ctrl+Shift+Z`/`Ctrl+Y` (Redo), `Ctrl+S` (Save), `Ctrl+O` (Open), `Ctrl+K`/`Ctrl+P` (Command Palette), `Ctrl+L` (Asset Library), `Shift+D`/`Ctrl+D` (Duplicate), `1`/`2`/`3`/`4` (Seleção Object/Point/Edge/Face), `Delete`/`Backspace` (Delete), `Tab` (Scene Outliner), e ferramentas `Q`, `W`, `B`, `E`, `U` |
| AccessKit | `COMPLIANT` | feature `accessibility` ativa, papéis e rótulos semânticos completos em `RailButton`, `TopAction`, `ToolButton`, `ColorSwatch`, `NumericField` (`spinbox`), `InspectorSection` (`button`) e `search-input` (`text-input`) |

## Inventário atual relevante

- Domínio: `crates/core`, `crates/project`, `crates/commands`, `crates/config`.
- Host atual: `crates/app/src/lib.rs`, com winit + wgpu + egui.
- Render atual: `crates/render-wgpu` e `crates/render-gl`.
- Contrato geométrico de viewport: `crates/core/src/viewport.rs`.
- Estado de editor: `AppState` separa `ProjectState`, `EditorSession`, `UiState` e `RenderResources`.
- Comandos atuais: `AppState::dispatch` e `CommandDispatcher`; a crate Slint não duplica algoritmos.

## Dependências pesquisadas

| Dependência | Decisão | Observação |
|---|---|---|
| `slint` | usar | `~1.18`, `unstable-wgpu-30`, `renderer-femtovg`, `renderer-software`, AccessKit e backend winit |
| `slintcn` | não runtime | CLI MIT para copiar componentes; proveniência deve ficar registrada quando um componente for incorporado |
| `lucide-slint` | usar | `=1.47.0`, paths Slint pré-convertidos; API atual usa `IconSet` + `IconDisplay` |
| Tabler | adiar | nenhum subset necessário no bootstrap; não baixar assets ainda |
| `rfd` | usar na boundary | `=0.17.2`, `AsyncFileDialog`, filtros centralizados |
| `viewport-lib` | opcional | `=0.22.0`, feature `wgpu30`, GPL-3.0-only; não compila no default |
| `wgpu` | existente | workspace já usa `30.0.1`; Slint 1.18 expõe suporte WGPU 30 instável |
| `glam` | existente | domínio atual usa `0.27`; `viewport-lib` exige `0.30.10`, outra razão para manter o adapter isolado |

## Validação executada

```text
cargo fmt -p petunia_ui_slint -- --check             PASS
cargo check -p petunia_ui_slint --all-targets        PASS
cargo check -p petunia_ui_slint --example shell      PASS
cargo test -p petunia_ui_slint --lib                 PASS (40 testes unitários: 25 shell/bridge/undo/primitives/paint/selection/persistence/camera/scene/theme/asset-library/duplicate/new-commands, 4 commands, 1 files, 5 numeric, 3 overlay, 2 theme)
cargo clippy -p petunia_ui_slint --all-targets -- -D warnings PASS
cargo run -p xtask -- ui-guard --strict              PASS
```

Todos os módulos (`commands`, `files`, `numeric`, `overlay`, `theme`), a integração WGPU (`viewport_gpu`), o shell declarativo em Slint e a bridge com Scene Outliner hierárquico, Asset Library drawer inferior, persistência de arquivo, atalhos globais de teclado, HUD de navegação com projeção e wireframe, controles de câmera, pilha de overlays, inputs numéricos, Tool Shelf contextual, seletor de domínio de seleção, duplicação e salvamento de ativos, Undo/Redo e painéis de Inspector estão testados e com zero avisos de linter.

### Próximos passos identificados

1. **Catálogo de Ícones Semânticos**: Expandir mapeamento de Lucide para cobrir ferramentas adicionais de modelagem.
2. **Screenshots & Golden Tests**: Criar rotinas automatizadas de render snapshot para testes de regressão visual do shell Slint.



