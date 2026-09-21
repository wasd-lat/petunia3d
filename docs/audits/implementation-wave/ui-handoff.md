# UI handoff (Slint / egui — do not implement here)

## New Commands / IDs

| ID | Behavior |
| -- | -------- |
| `model.loop_cut` | args `[cuts, even(0/1)]` |
| `model.connect` | two selected faces |
| `uv.unwrap_auto` | Auto UV |
| `uv.pack_islands` | args `[padding]` |
| `uv.project_view` | camera projection |
| `model.scale_selection` | uniform scale |

## DTOs / state

- `UvDiagnosticsDto` via `AppState::query_uv_diagnostics()`
- `BrushLock`, `BrushProjectionMode`, `FillScope` on `ToolState`
- `ViewportQuerySample` for hover/paint/picking overlay (CPU)
- `SurfaceAttachment` + `AttachmentValidity` for decal gizmos
- Layer `locked`, `is_group`, `group_id`

## Expected UI

- Loop Cut: cuts + Even checkbox
- UV: pack padding, texel density field, diagnostics badges (overlap/stretch)
- Paint: projection Surface/Screen, lock dropdown, fill scope, layer groups, add decal
- Primitive picker: show error toast on unknown name (no Cone fallback)

## Files likely affected

- `crates/ui-slint/ui/app.slint` (do not edit in this wave)
- `crates/ui-slint/src/lib.rs` intents
- Inspector Context for MODEL/UV/PAINT
