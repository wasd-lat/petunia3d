use petunia_mesh::Mesh;
use petunia_project::{Asset, ModifierInstance, Project, export_obj, format};

#[test]
fn modifier_evaluation_does_not_mutate_base_mesh() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let base_vertices = asset.mesh.verts.len();
    let base_faces = asset.mesh.faces.len();
    asset.modifiers.push(ModifierInstance::mirror(0, 0.0));

    let evaluated = asset.evaluated_mesh();

    assert_eq!(asset.mesh.verts.len(), base_vertices);
    assert_eq!(asset.mesh.faces.len(), base_faces);
    assert_eq!(evaluated.verts.len(), base_vertices * 2);
    assert_eq!(evaluated.faces.len(), base_faces * 2);
}

#[test]
fn disabled_modifier_is_not_evaluated() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let mut mirror = ModifierInstance::mirror(0, 0.0);
    mirror.enabled = false;
    asset.modifiers.push(mirror);

    let evaluated = asset.evaluated_mesh();

    assert_eq!(evaluated.verts.len(), asset.mesh.verts.len());
    assert_eq!(evaluated.faces.len(), asset.mesh.faces.len());
}

#[test]
fn export_obj_uses_evaluated_modifier_stack() {
    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    asset.modifiers.push(ModifierInstance::mirror(0, 0.0));

    let obj = export_obj(&asset);
    let exported_vertices = obj.lines().filter(|line| line.starts_with("v ")).count();

    assert_eq!(exported_vertices, asset.mesh.verts.len() * 2);
}

#[test]
fn modifier_stack_roundtrips_through_project_format() {
    let mut project = Project::new();
    project.assets.clear();
    let mut asset = Asset::new("Modified Cube", Mesh::cube(2.0));
    let mirror = ModifierInstance::mirror(0, 0.0);
    let mirror_id = mirror.id;
    asset.modifiers.push(mirror);
    asset
        .modifiers
        .push(ModifierInstance::symmetry(1, true, 0.001));
    project.assets.push(asset);
    project.active = 0;

    let path = std::env::temp_dir().join(format!(
        "petunia_modifier_roundtrip_{}.petunia",
        uuid::Uuid::new_v4()
    ));
    format::save(&project, &path).unwrap();
    let loaded = format::load(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.assets.len(), 1);
    assert_eq!(loaded.assets[0].modifiers.len(), 2);
    assert_eq!(loaded.assets[0].modifiers[0].id, mirror_id);
    assert_eq!(loaded.assets[0].modifiers, project.assets[0].modifiers);
}

#[test]
fn modifier_cache_invalidates_when_properties_change() {
    use petunia_project::ModifierKind;

    let mut asset = Asset::new("Cube", Mesh::cube(2.0));
    let mirror = ModifierInstance::mirror(0, 0.0);
    asset.modifiers.push(mirror);

    let first_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(first_eval.verts.len(), 16);

    // Toggle enabled off (modifiers.len() does NOT change!)
    asset.modifiers[0].enabled = false;
    let second_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(second_eval.verts.len(), 8);

    // Toggle enabled back on
    asset.modifiers[0].enabled = true;
    let third_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(third_eval.verts.len(), 16);

    // Change axis from 0 to 1
    asset.modifiers[0].kind = ModifierKind::Mirror { axis: 1, weld: 0.0 };
    let fourth_eval = asset.evaluated_mesh_cached().clone();
    assert_eq!(fourth_eval.verts.len(), 16);
    assert_ne!(first_eval.verts[8].pos, fourth_eval.verts[8].pos);
}
