# PAINT implementation

- Stroke: `begin_paint_stroke` / samples / `finish_paint_stroke` = one undo
- Dirty tiles: `composite_active_tiles` after dabs
- Ownership: paint_stack canonical; texture composed; albedo derived
- Brushes: Pixel, Soft, Airbrush, Eraser, Fill, Eyedropper, Line, Rectangle
- `BrushProjectionMode::{Surface, ScreenSpace}`
- `BrushLock::{None, FirstObject, FirstFace, SelectedFaces}`
- `FillScope::{ConnectedPixels, Face, SelectedFaces, UvIsland, Object}`
- `paint_screen_space` stamps visible faces through the same brush
- Layers: visibility, opacity, blend, lock, rename (existing), groups (`is_group`/`group_id`)
- Effects: Pixelate, Posterize, Invert, Grain, Levels, BrightnessContrast, HueSaturation
- Decals: persistent `LayerKind::Decal` + `PaintModule::add_decal`
- `ViewportQueryBuffer` / `SurfaceAttachment` for future GPU query + invalidation
