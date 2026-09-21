# Compatibility

- Existing `.petunia` postcard files still load.
- New saves are ZIP; tools that sniffed `PETUNIA\0` must accept `PK`.
- `PrimitiveKind::parse` is stricter: unknown names error.
- `BevelCmd` gained `segments`.
- `Mesh` gained `uv_seams` (serde default empty).
- `PaintLayer` gained `locked`, `group_id`, `is_group` (serde default).
- `Asset.eval_cache` is skipped in serde.
- MCP `add_primitive` uses Application commands; names like `MCP cube` preserved.
- Command aliases: `global.undo` → `edit.undo`, `model.select_all` → `select.all`.
