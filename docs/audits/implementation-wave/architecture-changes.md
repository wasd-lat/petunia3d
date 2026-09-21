# Architecture changes

- Core no longer depends on `petunia_render`. `Shading` lives in `petunia_core`.
- Mutation path: `CommandIntent` → `AppState::dispatch_intent` → `CommandDispatcher` → domain.
- Topology edits expose `TopologyResult` / `DirtyDomains` / `ElementRemap`.
- UV0 is the only UV set; seams are undirected edge keys on `Mesh`.
- Paint canonical raster: `Asset.paint_stack`; `Asset.texture` is composed cache; material albedo is derived.
- Renderer invalidation prefers revision counters over hashing all texture bytes.
- Persistence: new saves are ZIP (`manifest.json` + `document.json`); postcard `PETUNIA\0` still loads.
