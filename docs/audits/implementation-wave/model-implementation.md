# MODEL implementation

Integrated Tool → Command → geometry:

- Extrude / Extrude Individual / Inset / Bevel (segments) / Subdivide (cuts) / Scale / Duplicate / Delete
- Loop Cut + Even Loop Cut (`LoopCutCmd`)
- Connect loops (`ConnectLoopsCmd`)
- Weld / Merge / Mirror / Symmetry / Flip normals / Flip diagonal / Separate / Revolve
- Primitives: typed `PrimitiveKind::parse`, no silent Cone/Cube fallback
- UV on new extruded walls: perimeter U along, V 0→1
- Loop cut interpolates existing face UVs
- `extrude_selected_result()` returns `TopologyResult`

Application IDs: `model.extrude`, `model.loop_cut`, `model.connect`, `model.subdivide`, `model.bevel`, `model.scale_selection`.
