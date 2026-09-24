//! Filesystem fixtures for save/package flows (`assert_fs`, P1-13).
//!
//! Readable file-tree assertions around project save, package roundtrip and
//! hostile inputs: prelude trees, named children, and exact content checks.

use assert_fs::prelude::*;
use petunia_project::{format, open_package, save_package};

#[test]
fn save_creates_exact_tree() {
    let temp = assert_fs::TempDir::new().unwrap();
    let target = temp.child("scene.petunia");
    let mut project = petunia_project::Project::new();
    project.add("TreeCube", petunia_mesh::Mesh::cube(1.0));
    format::save(&project, target.path()).unwrap();

    assert!(target.path().is_file());
    let bytes = std::fs::read(target.path()).unwrap();
    // ZIP V1 container magic / Assinatura do container ZIP V1 (P3D-001)
    assert!(bytes.starts_with(b"PK\x03\x04"));
    let reloaded = format::load(target.path()).unwrap();
    assert_eq!(reloaded.assets.len(), project.assets.len());
    assert_eq!(reloaded.assets[1].name, "TreeCube");
}

#[test]
fn package_roundtrip_keeps_declared_tree() {
    let temp = assert_fs::TempDir::new().unwrap();
    let target = temp.child("scene.pkg");
    let mut project = petunia_project::Project::new();
    project.add("TreePlane", petunia_mesh::Mesh::plane(1.0));
    let attachments =
        vec![petunia_project::Attachment::new("note.txt", b"hello".to_vec()).unwrap()];
    let manifest = save_package(&project, "tree", &attachments, target.path()).unwrap();
    assert_eq!(manifest.files.len(), 3);

    assert!(target.path().is_file());
    let (back, _, back_attachments) = open_package(target.path()).unwrap();
    assert!(back.assets.iter().any(|a| a.name == "TreePlane"));
    assert_eq!(back_attachments, attachments);
}

#[test]
fn hostile_package_bytes_are_typed_errors() {
    let temp = assert_fs::TempDir::new().unwrap();
    let evil = temp.child("evil.pkg");
    evil.write_binary(b"\x50\x4b\x03\x04 Garbage not a manifest")
        .unwrap();
    assert!(open_package(evil.path()).is_err());
}
