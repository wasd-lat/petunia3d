//! Registro semântico inicial para palette, menus e atalhos futuros.

use petunia_core::{SelectionDomain, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    SaveProject,
    OpenProject,
    Undo,
    Redo,
    AddCube,
    AddSphere,
    AddCylinder,
    AddPlane,
    DeleteSelected,
    SelectModeObject,
    SelectModePoint,
    SelectModeEdge,
    SelectModeFace,
    PaintBrush,
    PaintEraser,
    PaintFill,
    UvUnwrap,
    UvPackIslands,
    FrameSelection,
    ToggleWireframe,
    OpenSettings,
    ToggleSceneDrawer,
    DuplicateSelected,
    SelectAll,
    ClearSelection,
    InvertSelection,
    ToggleAssetLibrary,
    ResetCamera,
    ToggleProjection,
    SaveActiveAsAsset,
}

impl CommandId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SaveProject => "global.save_project",
            Self::OpenProject => "global.open_project",
            Self::Undo => "global.undo",
            Self::Redo => "global.redo",
            Self::AddCube => "model.add_cube",
            Self::AddSphere => "model.add_sphere",
            Self::AddCylinder => "model.add_cylinder",
            Self::AddPlane => "model.add_plane",
            Self::DeleteSelected => "edit.delete",
            Self::SelectModeObject => "select.mode_object",
            Self::SelectModePoint => "select.mode_point",
            Self::SelectModeEdge => "select.mode_edge",
            Self::SelectModeFace => "select.mode_face",
            Self::PaintBrush => "paint.brush",
            Self::PaintEraser => "paint.eraser",
            Self::PaintFill => "paint.fill",
            Self::UvUnwrap => "uv.unwrap",
            Self::UvPackIslands => "uv.pack_islands",
            Self::FrameSelection => "model.frame_selection",
            Self::ToggleWireframe => "view.toggle_wireframe",
            Self::OpenSettings => "global.settings",
            Self::ToggleSceneDrawer => "window.scene_drawer",
            Self::DuplicateSelected => "model.duplicate",
            Self::SelectAll => "select.all",
            Self::ClearSelection => "select.clear",
            Self::InvertSelection => "select.invert",
            Self::ToggleAssetLibrary => "window.asset_library",
            Self::ResetCamera => "view.reset_camera",
            Self::ToggleProjection => "view.toggle_projection",
            Self::SaveActiveAsAsset => "model.save_as_asset",
        }
    }

    pub fn from_id_str(s: &str) -> Option<Self> {
        match s {
            "global.save_project" => Some(Self::SaveProject),
            "global.open_project" => Some(Self::OpenProject),
            "global.undo" => Some(Self::Undo),
            "global.redo" => Some(Self::Redo),
            "model.add_cube" => Some(Self::AddCube),
            "model.add_sphere" => Some(Self::AddSphere),
            "model.add_cylinder" => Some(Self::AddCylinder),
            "model.add_plane" => Some(Self::AddPlane),
            "edit.delete" => Some(Self::DeleteSelected),
            "select.mode_object" => Some(Self::SelectModeObject),
            "select.mode_point" => Some(Self::SelectModePoint),
            "select.mode_edge" => Some(Self::SelectModeEdge),
            "select.mode_face" => Some(Self::SelectModeFace),
            "paint.brush" => Some(Self::PaintBrush),
            "paint.eraser" => Some(Self::PaintEraser),
            "paint.fill" => Some(Self::PaintFill),
            "uv.unwrap" => Some(Self::UvUnwrap),
            "uv.pack_islands" => Some(Self::UvPackIslands),
            "model.frame_selection" => Some(Self::FrameSelection),
            "view.toggle_wireframe" => Some(Self::ToggleWireframe),
            "global.settings" => Some(Self::OpenSettings),
            "window.scene_drawer" => Some(Self::ToggleSceneDrawer),
            "model.duplicate" => Some(Self::DuplicateSelected),
            "select.all" => Some(Self::SelectAll),
            "select.clear" => Some(Self::ClearSelection),
            "select.invert" => Some(Self::InvertSelection),
            "window.asset_library" => Some(Self::ToggleAssetLibrary),
            "view.reset_camera" => Some(Self::ResetCamera),
            "view.toggle_projection" => Some(Self::ToggleProjection),
            "model.save_as_asset" => Some(Self::SaveActiveAsAsset),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub label_key: &'static str,
    pub shortcut: Option<&'static str>,
    pub workspace: Option<Workspace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandContext {
    pub workspace: Workspace,
    pub selection: SelectionDomain,
}

#[derive(Debug, Default)]
pub struct CommandRegistry;

impl CommandRegistry {
    pub const fn new() -> Self {
        Self
    }

    pub fn all(&self) -> &'static [CommandDescriptor] {
        &[
            CommandDescriptor {
                id: CommandId::SaveProject,
                label_key: "commands.save_project",
                shortcut: Some("Ctrl+S"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::OpenProject,
                label_key: "commands.open_project",
                shortcut: Some("Ctrl+O"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::Undo,
                label_key: "commands.undo",
                shortcut: Some("Ctrl+Z"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::Redo,
                label_key: "commands.redo",
                shortcut: Some("Ctrl+Shift+Z"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::AddCube,
                label_key: "commands.add_cube",
                shortcut: None,
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::AddSphere,
                label_key: "commands.add_sphere",
                shortcut: None,
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::AddCylinder,
                label_key: "commands.add_cylinder",
                shortcut: None,
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::AddPlane,
                label_key: "commands.add_plane",
                shortcut: None,
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::DeleteSelected,
                label_key: "commands.delete",
                shortcut: Some("Del"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::SelectModeObject,
                label_key: "commands.select_mode_object",
                shortcut: Some("1"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::SelectModePoint,
                label_key: "commands.select_mode_point",
                shortcut: Some("2"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::SelectModeEdge,
                label_key: "commands.select_mode_edge",
                shortcut: Some("3"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::SelectModeFace,
                label_key: "commands.select_mode_face",
                shortcut: Some("4"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::PaintBrush,
                label_key: "commands.paint_brush",
                shortcut: Some("B"),
                workspace: Some(Workspace::Paint),
            },
            CommandDescriptor {
                id: CommandId::PaintEraser,
                label_key: "commands.paint_eraser",
                shortcut: Some("E"),
                workspace: Some(Workspace::Paint),
            },
            CommandDescriptor {
                id: CommandId::PaintFill,
                label_key: "commands.paint_fill",
                shortcut: Some("G"),
                workspace: Some(Workspace::Paint),
            },
            CommandDescriptor {
                id: CommandId::UvUnwrap,
                label_key: "commands.uv_unwrap",
                shortcut: Some("U"),
                workspace: Some(Workspace::Uv),
            },
            CommandDescriptor {
                id: CommandId::UvPackIslands,
                label_key: "commands.uv_pack_islands",
                shortcut: None,
                workspace: Some(Workspace::Uv),
            },
            CommandDescriptor {
                id: CommandId::FrameSelection,
                label_key: "commands.frame_selection",
                shortcut: Some("Numpad ."),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::ToggleWireframe,
                label_key: "commands.toggle_wireframe",
                shortcut: None,
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::OpenSettings,
                label_key: "commands.settings",
                shortcut: None,
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::ToggleSceneDrawer,
                label_key: "commands.scene_drawer",
                shortcut: Some("Tab"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::DuplicateSelected,
                label_key: "commands.duplicate",
                shortcut: Some("Shift+D"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::SelectAll,
                label_key: "commands.select_all",
                shortcut: Some("Ctrl+A"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::ClearSelection,
                label_key: "commands.clear_selection",
                shortcut: Some("Alt+A"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::InvertSelection,
                label_key: "commands.invert_selection",
                shortcut: Some("Ctrl+I"),
                workspace: Some(Workspace::Model),
            },
            CommandDescriptor {
                id: CommandId::ToggleAssetLibrary,
                label_key: "commands.asset_library",
                shortcut: Some("Ctrl+L"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::ResetCamera,
                label_key: "commands.reset_camera",
                shortcut: Some("Home"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::ToggleProjection,
                label_key: "commands.toggle_projection",
                shortcut: Some("Numpad 5"),
                workspace: None,
            },
            CommandDescriptor {
                id: CommandId::SaveActiveAsAsset,
                label_key: "commands.save_as_asset",
                shortcut: None,
                workspace: Some(Workspace::Model),
            },
        ]
    }

    pub fn search(&self, query: &str, context: CommandContext) -> Vec<CommandDescriptor> {
        let query = query.trim().to_lowercase();
        self.all()
            .iter()
            .copied()
            .filter(|descriptor| {
                descriptor
                    .workspace
                    .is_none_or(|workspace| workspace == context.workspace)
            })
            .filter(|descriptor| query.is_empty() || fuzzy_match(&query, descriptor.label_key))
            .collect()
    }
}

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    let mut candidate_chars = candidate.chars();
    query
        .chars()
        .all(|query_char| candidate_chars.any(|candidate_char| candidate_char == query_char))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_keeps_global_commands_in_every_workspace() {
        let registry = CommandRegistry::new();
        let results = registry.search(
            "save",
            CommandContext {
                workspace: Workspace::Paint,
                selection: SelectionDomain::Object,
            },
        );
        assert_eq!(results, vec![registry.all()[0]]);
    }

    #[test]
    fn model_only_commands_are_hidden_from_paint() {
        let registry = CommandRegistry::new();
        let results = registry.search(
            "cube",
            CommandContext {
                workspace: Workspace::Paint,
                selection: SelectionDomain::Object,
            },
        );
        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_search_matches_semantic_key() {
        let registry = CommandRegistry::new();
        let results = registry.search(
            "frmsel",
            CommandContext {
                workspace: Workspace::Model,
                selection: SelectionDomain::Object,
            },
        );
        assert_eq!(results[0].id, CommandId::FrameSelection);
    }

    #[test]
    fn command_id_roundtrips_through_string_representations() {
        let registry = CommandRegistry::new();
        for descriptor in registry.all() {
            let str_repr = descriptor.id.as_str();
            assert_eq!(
                CommandId::from_id_str(str_repr),
                Some(descriptor.id),
                "Failed to parse id for command {:?}",
                descriptor.id
            );
        }
    }
}
