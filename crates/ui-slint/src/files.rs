//! Boundary de arquivos da UI experimental.
//!
//! Callbacks Slint devem chamar este serviço, nunca `rfd` diretamente. O
//! domínio recebe caminhos; não conhece detalhes do diálogo nativo.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogKind {
    OpenProject,
    SaveProject,
    ImportMesh,
    ExportMesh,
    SelectAssetFolder,
}

impl FileDialogKind {
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::OpenProject => "files.open_project",
            Self::SaveProject => "files.save_project",
            Self::ImportMesh => "files.import_mesh",
            Self::ExportMesh => "files.export_mesh",
            Self::SelectAssetFolder => "files.select_asset_folder",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileDialogService;

impl FileDialogService {
    pub const fn new() -> Self {
        Self
    }

    pub async fn open_project(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Petunia project", &["petunia", "pkg"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn save_project(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Petunia project", &["petunia"])
            .set_file_name("untitled.petunia")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn import_mesh(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("3D mesh", &["obj", "gltf", "glb"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn export_mesh(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .add_filter("glTF", &["gltf", "glb"])
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn select_asset_folder(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Importa um modelo 3D do disco (OBJ, glTF ou GLB).
    pub async fn import_model(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("3D model", &["obj", "gltf", "glb"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Escolhe o destino de exportação OBJ.
    pub async fn export_obj(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .set_file_name("model.obj")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Escolhe o destino de exportação GLB.
    pub async fn export_glb(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("glTF Binary", &["glb"])
            .set_file_name("scene.glb")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_kinds_have_stable_translation_keys() {
        assert_eq!(
            FileDialogKind::OpenProject.title_key(),
            "files.open_project"
        );
        assert_eq!(FileDialogKind::ExportMesh.title_key(), "files.export_mesh");
    }
}
