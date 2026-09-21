# UV implementation

- Canonical UV0 on `Face.uv`
- Seams: `Mesh.uv_seams`; `UvModule::mark_selected_seams` / `clear_selected_seams`
- Islands: adjacency minus seams
- Auto unwrap: `UnwrapAutoCmd` (xatlas provider + fallback)
- Pack: `UvPackIslandsCmd` (grid pack, padding, no overlap by construction)
- Project From View: `UvProjectFromViewCmd`
- Planar / cube projection unchanged
- Texel density measure + normalize
- Diagnostics DTO: islands, overlap, zero-area, OOR, stretch
- Selection: `AppEvent::SelectionChanged` still mirrors faces into `uv_selected`
