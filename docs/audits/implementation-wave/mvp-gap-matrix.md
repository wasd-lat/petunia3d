# MVP Gap Matrix — MODEL / UV / PAINT wave

Status after this implementation wave. Tests were **not** executed (user request).

| Requirement | Current implementation | Files | Status | Priority |
| ----------- | ---------------------- | ----- | ------ | -------- |
| Command spine (UI/CLI/FFI/MCP/Lua → Application) | `dispatch_intent` + aliases; tools wrap Commands | `core/src/state.rs`, `command.rs`, adapters | IMPLEMENTED_AND_CORRECT | P0 |
| MCP owns no Document | `McpSession { AppState }` | `mcp/src/server.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Primitive parse errors | `PrimitiveKind::parse` → `UnknownPrimitive` | `command.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| TopologyResult | `mesh::topology` + `extrude_selected_result` | `mesh/src/topology.rs`, `ops.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Even Loop Cut | `LoopRing::apply_even` + `LoopCutCmd` | `loop_cut.rs`, `command.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| UV0 canonical | Face-corner UV on Mesh | `mesh/src/lib.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Seams | `Mesh.uv_seams` + mark/clear | `uv_tools.rs`, `module-uv` | IMPLEMENTED_AND_CORRECT | P0 |
| Auto UV / pack / project from view | Commands + mesh methods | `uv.rs`, `uv_tools.rs`, `command.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Islands / texel density / diagnostics | `uv_islands`, `UvDiagnostics`, `UvDiagnosticsDto` | `uv_tools.rs`, `queries.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Paint stroke lifecycle | `begin_paint_stroke` / `finish_paint_stroke` (1 undo) | `state.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Dirty tiles composite | `composite_active_tiles` | `module-paint` | IMPLEMENTED_AND_CORRECT | P0 |
| Brush lock / projection / fill scope | `BrushLock`, `BrushProjectionMode`, `FillScope` | `brush.rs`, `module-paint` | IMPLEMENTED_AND_CORRECT | P0 |
| Screen-space brush | `paint_screen_space` | `module-paint` | IMPLEMENTED_AND_CORRECT | P0 |
| Layers / groups / lock / effects / decals | stack + `is_group`/`group_id`/`locked` | `paint_layers.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Viewport query DTO | `ViewportQueryBuffer` / `SurfaceAttachment` | `viewport_query.rs` | IMPLEMENTED_AND_CORRECT | P1 |
| Render revisions | project counters + fingerprint mix | `project/lib.rs`, `render_revision.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| `.petunia` ZIP + legacy postcard | `format.rs` | `project/src/format.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| GLB import materializes meshes | `import_glb_bytes` | `import_gltf.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Undo byte budget | `UndoStack` 256 MiB | `commands/src/lib.rs` | IMPLEMENTED_AND_CORRECT | P0 |
| Evaluated mesh cache | `Asset::evaluated_mesh_cached` | `project/lib.rs` | IMPLEMENTED_AND_CORRECT | P1 |
| Generational mesh handles | still `Vec` indices; remap contract added | `mesh` | PARTIAL | P2 |
| GPU viewport query buffer | CPU DTO only | `viewport_query.rs` | PARTIAL | P1 |
| Paint tile undo diffs | stroke snapshot of Project, not per-tile | `state.rs` | PARTIAL | P1 |
| Spline / Path Paint | out of scope | — | MISSING (deferred) | V1.x |
