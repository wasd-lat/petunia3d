# Crate `petunia_ui_slint` (`crates/ui-slint/`)

Frontend gráfico e shell declarativo moderno construído com [Slint](https://slint.dev/):
- **Shell declarativo (`ui/app.slint`)**: Top bar com alternador de domínio de seleção (Object, Point, Edge, Face), Icon Rail para workspaces (`MODEL`, `PAINT`, `UV`), Tool Shelf contextual vertical, Outliner lateral (Scene Drawer), gaveta inferior para Asset Library, e painéis de Context Inspector adaptados ao workspace ativo.
- **Viewport central interativo**: Suporte a WGPU off-screen render com export para textura do Slint (`viewport_gpu.rs`) e software viewport fallback resiliente (`viewport_soft.rs`) para compatibilidade universal (ex.: Intel Gen 7 Ivy Bridge / drivers Mesa legados).
- **Bridge de intents reativo (`lib.rs`)**: Comunicação desacoplada entre a UI Slint e os domínios de `petunia_core`, `petunia_commands` e `petunia_project`.
- **Command Palette & catálogo semântico (`commands.rs`)**: Busca difusa e execução direta de comandos (criação de primitivas, transformações, pintura e UV).
- **Persistência assíncrona (`files.rs`)**: Diálogos nativos assíncronos via RFD para carregar e salvar arquivos de projeto `.petunia`.
- **Camada de design tokens e temas (`theme.rs`)**: Sincronização em tempo de execução com o `ThemeRegistry` do Petunia (`petunia-dark` e `petunia-high-contrast`).
- **Gerenciador de Overlays (`overlay.rs`)**: Pilha de overlays LIFO com suporte a dismiss por tecla Escape e click-away.
