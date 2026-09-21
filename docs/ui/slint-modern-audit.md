# Auditoria — interface Slint de produção

## Escopo

Branch: `fix/slint-clean-ui-remediation`
Base: `9fcee47af69e1a4c620a955b0139a4693bc39cae`
Data da auditoria: 2026-09-20

O binário `petunia3d` executa o shell Slint (`crates/ui-slint/`) por padrão.
A UI egui (`crates/ui/`) é legado de transição acessível via `--legacy-egui` /
`PETUNIA_LEGACY_EGUI=1`.

## Implementation-vs-Spec Gap Matrix

A matriz abaixo reflete o estado **comprovado por testes e código** da crate
`petunia_ui_slint` (55 testes unitários verdes). Itens marcados como
`COMPLIANT` têm teste unitário ou evidência direta no código; itens
`PARTIALLY_COMPLIANT`, `MISSING` ou `STUB` são gaps conhecidos.

### Conformidade comprovada (testes verdes)

| Requisito | Estado | Evidência |
|---|---|---|
| Branch/promoção a produção | `COMPLIANT` | `src/main.rs` executa `petunia_ui_slint::run()` por padrão; egui sob flag `--legacy-egui` |
| Slint pinado na linha 1.18 | `COMPLIANT` | `Cargo.toml`: `slint ~1.18`, `compat-1-18` explícito |
| UI Slint isolada da egui | `COMPLIANT` | crate `crates/ui-slint/` não depende de `egui`; nenhum arquivo produtivo egui alterado |
| Design tokens semânticos | `COMPLIANT` | `theme.rs` sincroniza `DesignTokens` com `ThemeRegistry` (`petunia-dark`, `petunia-high-contrast`); teste `theme_change_intent_updates_active_theme` |
| Icon rail | `COMPLIANT` | rail com Scene, Tools (Model), Paint e UV com ícones Lucide em `ui/app.slint` |
| Viewport-first shell | `PARTIALLY_COMPLIANT` | viewport ocupa a região central, mas chrome permanente e sizing rígido ainda reduzem a área útil |
| Selection Domain Switcher | `COMPLIANT` | alternador Object/Point/Edge/Face; teste `selection_domain_intent_updates_state_and_view_model` |
| Undo & Redo | `COMPLIANT` | `ProjectContext::undo`/`redo`; teste `undo_redo_intents_integrate_with_project_undo_stack` |
| Scene Outliner / Drawer | `COMPLIANT` | Drawer lateral conectado a `state.project.assets`; testes `scene_item_selection_updates_active_asset_and_inspector`, `scene_item_visibility_and_lock_toggles` |
| Command Palette | `PARTIALLY_COMPLIANT` | busca e execução usam `petunia_core::CommandDispatcher`, labels apresentáveis e atalhos do keymap; navegação por setas/foco explícito ainda falta |
| Overlay/ESC policy LIFO | `COMPLIANT` | `OverlayStack` em `overlay.rs`; testes `overlay_stack_handles_escape_in_lifo_order`, `modal_identity_preserves_lifo_when_multiple_modals_are_open`, `closing_one_drawer_does_not_close_or_leave_a_ghost_for_another` |
| Numeric field | `PARTIALLY_COMPLIANT` | scrubbing transacional, fine-step e clamp funcionam; click-to-edit textual ainda não está ligado no componente Slint |
| FileDialogService | `COMPLIANT` | `files.rs` com `rfd::AsyncFileDialog`; teste `bridge_saves_and_loads_project_file` |
| Interação do Viewport (gestures) | `COMPLIANT` | órbita, pan, zoom via `ViewportGesture`; teste `bridge_applies_viewport_gestures_to_camera` |
| Viewport resize | `COMPLIANT` | teste `viewport_resize_updates_backend_and_camera_aspect` |
| PetuniaViewport (trait) | `COMPLIANT` | `WgpuViewport` e `Software3dViewport` implementam a trait |
| Integração GPU Slint/viewport | `BROKEN` | o device não é compartilhado com o renderer Slint; cada frame faz staging allocation, GPU→CPU readback e espera síncrona antes de criar `slint::Image` |
| Multi-renderer fallback / Ivy Bridge | `COMPLIANT` | `renderer-femtovg` (OpenGL) e `renderer-software` no Slint; detecção de Intel Gen 7 ativa `Software3dViewport` |
| Asset Library Drawer | `COMPLIANT` | Gaveta inferior expansível; testes `toggle_asset_library_and_overlays`, `save_active_as_asset_intent_creates_project_asset` |
| AccessKit | `COMPLIANT` | feature `accessibility` ativa; papéis e rótulos semânticos em `RailButton`, `ToolButton`, `NumericField`, etc. |
| Transform modal transacional | `COMPLIANT` | scrubbing, commit, cancel; testes `transform_scrub_*`, `escape_cancels_transform_without_closing_the_underlying_overlay`, `selecting_another_asset_resets_transform_operation_values` |
| Delete direto | `COMPLIANT` | testes `primitive_creation_and_deletion_updates_scene`, `delete_is_dirty_and_undo_restores_the_asset` |
| Visibility/Lock diretos | `COMPLIANT` | teste `scene_item_visibility_and_lock_toggles` |
| Context inspector por workspace | `PARTIALLY_COMPLIANT` | painéis de contexto adaptados por workspace (MODEL/PAINT/UV) presentes no `.slint`; refinamento de densidade e responsividade em progresso |
| Viewport HUD & Projeção | `PARTIALLY_COMPLIANT` | HUD com alternador Persp/Ortho, toggle Wireframe, Reset Camera; teste `camera_projection_and_reset` |
| Atalhos de teclado globais | `FUNCTIONAL_BUT_DIFFERENT` | FocusScope contém atalhos físicos hardcoded e ignora os perfis configuráveis de `Keybinds`; Tab também conflita com navegação de foco |

### Gaps conhecidos (sem evidência de implementação no Slint)

| Requisito | Estado | Observação |
|---|---|---|
| Registry centralizado de ícones (`IconRegistry`) | `MISSING` | O legado egui tem `crates/ui/src/icon_registry.rs`; o Slint usa Lucide diretamente via `lucide-slint` sem registry unificado |
| Registry centralizado de comandos (`CommandDispatcher` do core) | `COMPLIANT` | a palette consulta `state.commands.query(...)` e executa `state.dispatch_command(...)`; teste `core_palette_command_executes_through_canonical_dispatcher` |
| Keymap profiles (8 perfis canônicos) | `MISSING` | `petunia_config::keybinds` tem os perfis (Blender, Maya, etc.); sem bridge no shell Slint |
| i18n TOML (`pt-BR.toml`, `en.toml`) | `MISSING` | Arquivos existem em `assets/locales/` para o legado egui; não portados para o Slint |
| Fast path GPU sem readback | `BROKEN` | `viewport_gpu.rs` bloqueia em `PollType::wait_indefinitely()` e copia pixels para CPU por frame |
| Layout flex/grid complexo (Properties, Outliner) | `PARTIALLY_COMPLIANT` | Shell usa layout declarativo nativo Slint; painéis densos precisam de refinamento de responsividade |
| Screenshot/golden visual regression | `MISSING` | Criar após shell visual estabilizar |
| Viewer/LSP Slint | `MISSING` localmente | Binários não instalados; workflow documentado |
| `viewport-lib` (GPL) | `OBSOLETE` como dependência padrão | `0.22.0` pinado somente na feature opcional `viewport-lib-backend` por ser GPL-3.0-only |
| Theme Extension API (`.petunia-theme`) | `STUB` | Contrato definido no capítulo 36; implementação ausente no Slint |
| Plugin Panels Lua | `STUB` | Contrato definido no capítulo 36; implementação ausente |

## Inventário atual relevante

- **Domínio**: `crates/core`, `crates/project`, `crates/commands`, `crates/config`.
- **Frontend produção**: `crates/ui-slint/src/lib.rs` (bridge `UiIntent`), `ui/app.slint` (shell declarativo).
- **Frontend legado**: `crates/ui/` (egui), `crates/app/` (host egui).
- **Render**: `crates/render-wgpu` e `crates/render-gl` (toolkit-neutros via `PetuniaRenderer`).
- **Contrato geométrico de viewport**: `crates/core/src/viewport.rs`.
- **Estado de editor**: `AppState` separa `ProjectState`, `EditorSession`, `UiState` e `RenderResources`.
- **Comandos**: `AppState::dispatch` e `CommandDispatcher`; a crate Slint não duplica algoritmos.

## Dependências

| Dependência | Decisão | Observação |
|---|---|---|
| `slint` | usar (produção) | `~1.18`, `unstable-wgpu-30`, `renderer-femtovg`, `renderer-software`, AccessKit, backend winit |
| `lucide-slint` | usar | `=1.47.0`, paths Slint pré-convertidos; API `IconSet` + `IconDisplay` |
| `rfd` | usar na boundary | `=0.17.2`, `AsyncFileDialog`, filtros centralizados |
| `viewport-lib` | opcional | `=0.22.0`, feature `wgpu30`, GPL-3.0-only; não compila no default |
| `wgpu` | existente | workspace usa `30.0.1`; Slint 1.18 expõe suporte WGPU 30 instável |
| `glam` | existente | domínio usa `0.27`; `viewport-lib` exige `0.30.10`, razão para manter adapter isolado |
| `egui` | legado | workspace ainda pinado em `0.36.2`; crate `crates/ui/` não recebe novas features |

## Validação executada

```text
cargo fmt -p petunia_ui_slint -- --check             PASS
cargo check -p petunia_ui_slint --all-targets        PASS
cargo check -p petunia_ui_slint --example shell      PASS
cargo test -p petunia_ui_slint --lib                 PASS (55 testes unitários)
cargo clippy -p petunia_ui_slint --all-targets -- -D warnings PASS
```

### Cobertura dos 55 testes unitários

- Shell/bridge: workspace routing, temporary surfaces, viewport gestures, viewport resize, camera projection/reset
- Undo/Redo: integração com `ProjectContext::undo`/`redo`
- Primitivas: criação, deleção, duplicação, save-as-asset
- Paint: parâmetros de ferramenta (cor, tamanho, opacidade)
- Seleção: domain switch, select all/clear/invert, scene item selection
- Transform: scrubbing, clamping, commit, cancel, asset switch reset
- Scene: visibility, lock toggles
- Overlays: ESC LIFO, click-away, modal identity, drawer isolation
- Commands: search filter, routing, new commands via CommandId
- Files: save/load project
- Theme: registry, apply, change intent
- Viewport: WGPU init/skip, software resize/render/init

## Completion Track

The completion branch adds the direct WGPU 30 configuration path, keymap-driven
shortcut routing, viewport picking for Object/Point selection, numeric text
commit, command-palette focus and keyboard selection, disabled command reasons,
paint fill/UV dispatch, responsive inspector collapse, a pinned Parts drawer,
semantic viewport tokens, and the modular `tokens.slint`/`input.rs` boundaries.

Cut/Knife preview and Paint stroke projection still require the viewport event
adapter to feed domain coordinates into their existing sessions. They remain
explicitly partial rather than being presented as complete tools.

## Próximos passos

1. **Registry centralizado de ícones**: portar uma abstração semântica de `IconId` para o Slint.
2. **Keymap bridge**: conectar os 8 perfis canônicos de `petunia_config::keybinds` ao shell Slint.
3. **i18n**: portar `pt-BR.toml`/`en.toml` e o sistema de `TextId` para o Slint.
4. **GPU readback**: implementar rotina de snapshot para golden tests.
5. **Theme Extension API**: implementar `.petunia-theme` conforme capítulo 36.
6. **Plugin Panels**: implementar registry e isolation conforme capítulo 36.
7. **Layout refinement**: densidade e responsividade de Properties/Outliner.
8. **Screenshot/golden**: criar rotinas automatizadas de regressão visual.
