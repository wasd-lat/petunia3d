//! Frontend de produção do Petunia3D em Slint 1.18.
//!
//! O shell em Slint emite intenções (`UiIntent`); o domínio continua em
//! `petunia_core`, `petunia_commands` e `petunia_project`. A integração de GPU
//! é realizada pelo adapter `WgpuViewport` ou fallback `Software3dViewport`.
//!
//! A UI egui (`crates/ui/`) é legado de transição, acessível via
//! `--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod commands;
pub mod files;
mod input;
pub mod numeric;
pub mod overlay;
pub mod theme;
pub mod viewport_gpu;
pub mod viewport_soft;

pub use viewport_soft::Software3dViewport;

use commands::CommandId;
use numeric::NumericFieldState;
use overlay::{OverlayEntry, OverlayId, OverlayKind, OverlayStack};
use petunia_config::keybinds::Mods2;
use petunia_core::{AppState, Camera, SelectionDomain, Workspace};
use petunia_project::Project;
use slint::ComponentHandle;

slint::include_modules!();

/// Tipo de transformação tridimensional manipulada no Inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    Position,
    Rotation,
    Scale,
}

/// Gesto do cursor ou toque na viewport 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportGesture {
    Orbit { dx: f32, dy: f32 },
    Pan { dx: f32, dy: f32 },
    Zoom { delta: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRenderState {
    pub shading: petunia_render::Shading,
    pub xray: bool,
    pub show_triangulation: bool,
    pub textured: bool,
    pub show_wireframe_overlay: bool,
}

impl Default for ViewportRenderState {
    fn default() -> Self {
        Self {
            shading: petunia_render::Shading::Solid,
            xray: false,
            show_triangulation: false,
            textured: false,
            show_wireframe_overlay: false,
        }
    }
}

/// Gizmo 3D projetado para o overlay da viewport.
///
/// A projeção acontece no bridge; o Slint só desenha as três hastes a partir
/// de origem, comprimento e ângulo em pixels. O overlay não conhece câmera,
/// GPU nem matriz de projeção.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoModel {
    pub visible: bool,
    pub origin_x: f32,
    pub origin_y: f32,
    pub x_end_x: f32,
    pub x_end_y: f32,
    pub y_end_x: f32,
    pub y_end_y: f32,
    pub z_end_x: f32,
    pub z_end_y: f32,
}

impl Default for GizmoModel {
    fn default() -> Self {
        Self {
            visible: false,
            origin_x: 0.0,
            origin_y: 0.0,
            x_end_x: 0.0,
            x_end_y: 0.0,
            y_end_x: 0.0,
            y_end_y: 0.0,
            z_end_x: 0.0,
            z_end_y: 0.0,
        }
    }
}

/// Ferramenta paramétrica com preview modal e Tool Properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolModalKind {
    Extrude,
    ExtrudeIndividual,
    Inset,
    Bevel,
    PushPull,
    ScaleSelection,
}

impl ToolModalKind {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "model.extrude" => Some(Self::Extrude),
            "model.extrude_individual" => Some(Self::ExtrudeIndividual),
            "model.inset" => Some(Self::Inset),
            "model.bevel" => Some(Self::Bevel),
            "model.push_pull" => Some(Self::PushPull),
            "model.scale_selection" => Some(Self::ScaleSelection),
            _ => None,
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Extrude => "Extrude",
            Self::ExtrudeIndividual => "Extrude Individual",
            Self::Inset => "Inset",
            Self::Bevel => "Bevel",
            Self::PushPull => "Push/Pull",
            Self::ScaleSelection => "Scale",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Extrude | Self::ExtrudeIndividual | Self::PushPull => "Distance",
            Self::Inset => "Amount",
            Self::Bevel => "Width",
            Self::ScaleSelection => "Factor",
        }
    }

    pub const fn modal_kind(self) -> petunia_core::ModalKind {
        match self {
            Self::Extrude => petunia_core::ModalKind::Extrude,
            Self::ExtrudeIndividual => petunia_core::ModalKind::ExtrudeIndividual,
            Self::Inset => petunia_core::ModalKind::Inset,
            Self::Bevel => petunia_core::ModalKind::Bevel,
            Self::PushPull => petunia_core::ModalKind::PushPull,
            Self::ScaleSelection => petunia_core::ModalKind::Scale,
        }
    }

    pub const fn bounds(self) -> (f32, f32) {
        match self {
            Self::Inset => (0.0, 0.95),
            Self::Bevel => (0.0, 100.0),
            Self::ScaleSelection => (0.01, 100.0),
            Self::Extrude | Self::ExtrudeIndividual | Self::PushPull => (-100.0, 100.0),
        }
    }

    pub const fn step(self) -> f32 {
        match self {
            Self::Inset => 0.01,
            _ => 0.1,
        }
    }
}

/// Sessão de arrasto transacional iniciada na viewport.
#[derive(Debug, Clone, Copy)]
pub struct ViewportDrag {
    pub kind: TransformKind,
    pub start: [f32; 2],
    pub viewport: [f32; 2],
}

/// Ação semântica emitida pelo shell Slint.
#[derive(Debug, Clone, PartialEq)]
pub enum UiIntent {
    SetWorkspace(Workspace),
    SaveProject,
    Undo,
    Redo,
    SetSelectionDomain(SelectionDomain),
    AddPrimitive(petunia_core::PrimitiveKind),
    DeleteActiveAsset,
    SetPaintColor([f32; 3]),
    SetBrushSize(f32),
    SetBrushOpacity(f32),
    SetActiveTool(String),
    OpenCommandSearch,
    OpenSettings,
    ToggleSceneDrawer,
    ExecuteCommand(CommandId),
    DismissTopOverlay,
    ScrubTransform {
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    },
    ViewportGesture(ViewportGesture),
    SaveProjectTo(PathBuf),
    OpenProjectFrom(PathBuf),
    ImportModelFrom(PathBuf),
    ExportActiveObjTo(PathBuf),
    ExportSceneGlbTo(PathBuf),
    SelectSceneAsset(String),
    ToggleSceneAssetVisibility(String),
    ToggleSceneAssetLock(String),
    SetTheme(String),
    DuplicateActiveAsset,
    SelectAll,
    ClearSelection,
    InvertSelection,
    ToggleAssetLibrary,
    ResetCamera,
    ToggleProjection,
    SaveActiveAsAsset,
}

/// Representação DTO de um item da árvore de cena do Outliner.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItemModel {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub selected: bool,
    pub verts: usize,
    pub tris: usize,
}

/// Snapshot pequeno para apresentação. Não expõe estruturas internas do core ao
/// markup Slint e não carrega estado transitório de widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellViewModel {
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pub inspector_visible: bool,
    pub saved: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub active_tool: String,
    pub paint_color: [f32; 3],
    pub brush_size: f32,
    pub brush_opacity: f32,
    pub status_message: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub scene_items: Vec<SceneItemModel>,
    pub active_object_title: String,
    pub active_object_details: String,
    pub active_material_name: String,
    pub scene_stats: String,
    pub uv_stats: String,
    pub current_theme: String,
    pub is_orthographic: bool,
    pub is_wireframe: bool,
    pub asset_library_visible: bool,
    pub gizmo: GizmoModel,
    pub add_menu_open: bool,
    pub inspector_width: f32,
    pub asset_library_height: f32,
    pub rename_active: bool,
    pub rename_value: String,
    pub context_menu_open: bool,
    pub context_menu_x: f32,
    pub context_menu_y: f32,
    pub context_menu_title: String,
    pub context_menu_visible: bool,
    pub context_menu_locked: bool,
    pub menu_open: String,
    pub menu_file_label: String,
    pub menu_edit_label: String,
    pub menu_view_label: String,
    pub menu_window_label: String,
    pub menu_file_items: Vec<MenuEntryModel>,
    pub menu_edit_items: Vec<MenuEntryModel>,
    pub menu_view_items: Vec<MenuEntryModel>,
    pub menu_window_items: Vec<MenuEntryModel>,
    pub label_parts: String,
    pub label_project_asset_library: String,
    pub label_save_active_as_asset: String,
    pub label_status_hint: String,
    pub label_unwrap_mesh: String,
    pub label_pack_islands: String,
    pub label_active_brush_color: String,
    pub label_albedo_base_color: String,
    pub label_theme: String,
    pub label_place_in_scene: String,
    pub recovery_open: bool,
    pub recovery_title: String,
    pub recovery_body: String,
    pub recovery_detail: String,
    pub recovery_recover: String,
    pub recovery_keep: String,
    pub recovery_discard: String,
    pub label_asset_library: String,
    pub label_preferences: String,
    pub shell_info: String,
    pub label_apply: String,
    pub label_cancel: String,
    pub label_delete: String,
    pub label_duplicate: String,
    pub themes: Vec<ThemeEntryModel>,
    pub uv_editor: UvEditorModel,
    pub paint_layers: Vec<PaintLayerModel>,
    pub paint_layer_count: String,
    pub loop_cut_active: bool,
    pub loop_cut_slide: f32,
    pub loop_cut_cuts: i32,
    pub tool_modal_active: bool,
    pub tool_modal_title: String,
    pub tool_modal_label: String,
    pub tool_modal_value: f32,
    pub tool_modal_step: f32,
    pub tool_modal_min: f32,
    pub tool_modal_max: f32,
}

impl ShellViewModel {
    pub fn from_state(state: &AppState) -> Self {
        let active_id = state.project.active().map(|a| a.id);
        let scene_items: Vec<SceneItemModel> = state
            .project
            .assets
            .iter()
            .map(|asset| SceneItemModel {
                id: asset.id.to_string(),
                name: asset.name.clone(),
                visible: asset.visible,
                locked: asset.locked,
                selected: active_id == Some(asset.id),
                verts: asset.mesh.verts.len(),
                tris: asset.mesh.tri_count(),
            })
            .collect();

        let (active_object_title, active_object_details, active_material_name) =
            if let Some(active_asset) = state.project.active() {
                let title = active_asset.name.clone();
                let details = format!(
                    "Vertices: {}  ·  Faces: {}",
                    active_asset.mesh.verts.len(),
                    active_asset.mesh.faces.len()
                );
                let mat = active_asset
                    .material(&state.project.project)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "Default Material".to_string());
                (title, details, mat)
            } else {
                (
                    "No Object Selected".to_string(),
                    "Vertices: 0  ·  Faces: 0".to_string(),
                    "None".to_string(),
                )
            };

        let scene_stats = format!(
            "{} tris  ·  {} verts",
            state.scene_tris(),
            state.scene_verts()
        );

        let uv_stats = match petunia_module_uv::UvModule::diagnostics(state) {
            Some(diagnostics) => format!(
                "Provider: xatlas-rs-v2 (generic fallback)\nIslands: {}\nOverlaps: {}\nZero-area faces: {}\nOut of range: {}\nStretch: mean {:.1}% / max {:.1}%",
                diagnostics.island_count,
                diagnostics.overlapping_islands,
                diagnostics.zero_area_faces,
                diagnostics.out_of_range_corners,
                diagnostics.mean_stretch * 100.0,
                diagnostics.max_stretch * 100.0,
            ),
            None => "No active mesh".to_string(),
        };

        let active_tool = if state.session.tools.active_tool.is_empty() {
            "select".to_string()
        } else {
            state.session.tools.active_tool.clone()
        };

        let status_message = if state.ui.status.is_empty() {
            "Ready".to_string()
        } else {
            state.ui.status.clone()
        };

        Self {
            workspace: state.workspace,
            selection_domain: state.selection_domain(),
            inspector_visible: true,
            saved: !state.is_document_dirty(),
            can_undo: state.project.undo.can_undo(),
            can_redo: state.project.undo.can_redo(),
            active_tool,
            paint_color: state.session.tools.paint_color,
            brush_size: state.session.tools.paint_radius,
            brush_opacity: state.session.tools.paint_strength,
            status_message,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            scene_items,
            active_object_title,
            active_object_details,
            active_material_name,
            scene_stats,
            uv_stats,
            current_theme: state.ui.active_theme_id.clone(),
            is_orthographic: state.session.camera.proj == petunia_core::Projection::Ortho,
            is_wireframe: state.session.show_wireframe_overlay,
            asset_library_visible: false,
            gizmo: GizmoModel::default(),
            add_menu_open: false,
            inspector_width: state.ui.right_width,
            asset_library_height: state.ui.shell_asset_library_height,
            rename_active: false,
            rename_value: String::new(),
            context_menu_open: false,
            context_menu_x: 0.0,
            context_menu_y: 0.0,
            context_menu_title: String::new(),
            context_menu_visible: true,
            context_menu_locked: false,
            menu_open: String::new(),
            menu_file_label: String::new(),
            menu_edit_label: String::new(),
            menu_view_label: String::new(),
            menu_window_label: String::new(),
            menu_file_items: Vec::new(),
            menu_edit_items: Vec::new(),
            menu_view_items: Vec::new(),
            menu_window_items: Vec::new(),
            label_parts: String::new(),
            label_project_asset_library: String::new(),
            label_save_active_as_asset: String::new(),
            label_status_hint: String::new(),
            label_unwrap_mesh: String::new(),
            label_pack_islands: String::new(),
            label_active_brush_color: String::new(),
            label_albedo_base_color: String::new(),
            label_theme: String::new(),
            label_place_in_scene: String::new(),
            recovery_open: false,
            recovery_title: String::new(),
            recovery_body: String::new(),
            recovery_detail: String::new(),
            recovery_recover: String::new(),
            recovery_keep: String::new(),
            recovery_discard: String::new(),
            label_asset_library: String::new(),
            label_preferences: String::new(),
            shell_info: String::new(),
            label_apply: String::new(),
            label_cancel: String::new(),
            label_delete: String::new(),
            label_duplicate: String::new(),
            themes: Vec::new(),
            uv_editor: UvEditorModel::default(),
            paint_layers: Vec::new(),
            paint_layer_count: String::new(),
            loop_cut_active: false,
            loop_cut_slide: 0.0,
            loop_cut_cuts: 1,
            tool_modal_active: false,
            tool_modal_title: String::new(),
            tool_modal_label: String::new(),
            tool_modal_value: 0.0,
            tool_modal_step: 0.1,
            tool_modal_min: 0.0,
            tool_modal_max: 0.0,
        }
    }

    pub fn workspace_label(&self) -> &'static str {
        match self.workspace {
            Workspace::Model => "MODEL",
            Workspace::Paint => "PAINT",
            Workspace::Uv => "UV",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => "ANIMATE",
        }
    }
}

/// Contrato do backend de viewport.
pub trait PetuniaViewport: Send {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, dt_seconds: f32);
    fn set_workspace(&mut self, workspace: Workspace);
    fn set_selection_domain(&mut self, domain: SelectionDomain);
    fn render_frame(
        &mut self,
        _project: &Project,
        _camera: &Camera,
        _state: ViewportRenderState,
    ) -> Option<slint::Image> {
        None
    }
}

/// Backend inicial usado como fallback seguro quando não há GPU disponível.
#[derive(Debug, Default)]
pub struct PlaceholderViewport {
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pub last_dt_seconds: f32,
}

impl PetuniaViewport for PlaceholderViewport {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, dt_seconds: f32) {
        self.last_dt_seconds = dt_seconds.max(0.0);
    }

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }
}

impl PetuniaViewport for Box<dyn PetuniaViewport> {
    fn resize(&mut self, width: u32, height: u32) {
        (**self).resize(width, height);
    }

    fn update(&mut self, dt_seconds: f32) {
        (**self).update(dt_seconds);
    }

    fn set_workspace(&mut self, workspace: Workspace) {
        (**self).set_workspace(workspace);
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        (**self).set_selection_domain(domain);
    }

    fn render_frame(
        &mut self,
        project: &Project,
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        (**self).render_frame(project, camera, state)
    }
}

/// Bridge entre callbacks Slint e a aplicação. O bridge só aplica intenção
/// semântica ao `AppState`; algoritmos geométricos permanecem no core/commands.
pub struct SlintUiBridge<V: PetuniaViewport> {
    pub state: AppState,
    pub viewport: V,
    pub overlays: OverlayStack,
    pub scene_drawer_visible: bool,
    pub asset_library_visible: bool,
    pub command_search_visible: bool,
    pub settings_visible: bool,
    pub position: [NumericFieldState; 3],
    pub rotation: [NumericFieldState; 3],
    pub scale: [NumericFieldState; 3],
    pub drag: Option<ViewportDrag>,
    pub viewport_size: [f32; 2],
    /// Última posição de tela do traço de pintura ativo.
    pub paint_last: Option<[f32; 2]>,
    /// Menu de primitivas aberto (apresentação).
    pub add_menu_open: bool,
    /// Ferramenta paramétrica modal ativa (Extrude, Inset, Bevel, Push/Pull).
    pub tool_modal: Option<ToolModalKind>,
    pub tool_modal_value: f32,
    /// Renomeação inline do ativo selecionado (Outliner): `Some(nome em edição)`.
    pub rename_draft: Option<String>,
    /// Menu de contexto do Outliner: posição em px lógicos e ativo alvo.
    pub context_menu: Option<ContextMenuState>,
    /// Menu da barra superior aberto, se houver.
    pub menu_open: Option<MenuKind>,
    /// Sessão de loop cut com slide interativo (P3D-131).
    pub loop_cut: Option<LoopCutSessionState>,
    /// Autosave rotativo do shell (P3D-002). Nunca sobrescreve o arquivo oficial.
    pub autosave: petunia_core::AutosaveService,
    /// Snapshot de recuperação detectado no arranque, aguardando decisão.
    pub pending_recovery: Option<petunia_core::RecoveryInfo>,
}

/// Menu de contexto do Outliner aberto sobre uma linha do painel Parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMenuState {
    pub x: f32,
    pub y: f32,
    pub asset: uuid::Uuid,
}

/// Menus da barra superior do shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Edit,
    View,
    Window,
}

impl MenuKind {
    pub const ALL: [MenuKind; 4] = [Self::File, Self::Edit, Self::View, Self::Window];

    pub const fn id(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Edit => "edit",
            Self::View => "view",
            Self::Window => "window",
        }
    }

    pub const fn title(self) -> petunia_config::TextId {
        use petunia_config::text_id as T;
        match self {
            Self::File => T::MENU_FILE,
            Self::Edit => T::MENU_EDIT,
            Self::View => T::MENU_VIEW,
            Self::Window => T::MENU_WINDOW,
        }
    }

    /// Itens publicados pelo menu, na ordem de exibição.
    ///
    /// Cada id é um comando canônico real ou uma ação de shell que já existe no
    /// roteador — nenhum item decorativo.
    pub const fn items(self) -> &'static [(&'static str, petunia_config::TextId, &'static str)] {
        use petunia_config::text_id as T;
        match self {
            Self::File => &[
                ("file.new", T::FILE_NEW, "Ctrl+N"),
                ("file.open", T::FILE_OPEN_PROJECT, "Ctrl+O"),
                ("file.save", T::FILE_SAVE, "Ctrl+S"),
                ("file.save_as", T::FILE_SAVE_AS, "Ctrl+Shift+S"),
                ("file.import_obj", T::FILE_IMPORT_OBJ, ""),
            ],
            Self::Edit => &[
                ("edit.undo", T::EDIT_UNDO, "Ctrl+Z"),
                ("edit.redo", T::EDIT_REDO, "Ctrl+Shift+Z"),
                ("edit.duplicate", T::UI_DUPLICATE, "Shift+D"),
            ],
            Self::View => &[
                ("view.frame_selection", T::VIEW_FRAME, "F"),
                ("view.frame_all", T::VIEW_FRAME_ALL, "Home"),
                ("view.toggle_projection", T::VIEW_TOGGLE_PROJECTION, "O"),
                ("view.toggle_wireframe", T::VIEW_TOGGLE_WIREFRAME, "Z"),
                ("view.reset_camera", T::VIEW_RESET_CAMERA, "Shift+Home"),
            ],
            Self::Window => &[
                ("window.command_palette", T::MENU_COMMAND_PALETTE, "Ctrl+P"),
                ("window.settings", T::MENU_PREFERENCES, ""),
            ],
        }
    }
}

/// Editor UV 2D: geometria do layout pronta para o `Path` do Slint.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UvEditorModel {
    /// Comandos SVG-like do `Path` do Slint (`M x y L x y ...`).
    pub layout_commands: String,
    pub island_count: usize,
    pub face_count: usize,
    pub selected_face: i32,
    /// `true` quando a malha tem mais faces do que o editor desenha.
    pub truncated: bool,
}

/// Camada de pintura publicada para o painel de camadas do shell.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintLayerModel {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub active: bool,
    pub is_group: bool,
    pub kind_label: String,
}

/// Estado da sessão de loop cut ativa no shell.
#[derive(Debug, Clone)]
pub struct LoopCutSessionState {
    pub ring: petunia_core::LoopRing,
    pub cuts: usize,
    pub slide: f32,
    /// Malha anterior ao preview: toda reconstrução parte daqui, nunca do preview.
    pub source: petunia_core::Mesh,
}

/// Tema disponível no registry, já marcado como ativo ou não.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeEntryModel {
    pub id: String,
    pub name: String,
    pub active: bool,
}

/// Item de menu já traduzido para o shell.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntryModel {
    pub id: String,
    pub label: String,
    pub shortcut: String,
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn new(state: AppState, viewport: V) -> Self {
        let mut bridge = Self {
            state,
            viewport,
            overlays: OverlayStack::default(),
            scene_drawer_visible: false,
            asset_library_visible: false,
            command_search_visible: false,
            settings_visible: false,
            drag: None,
            viewport_size: [1024.0, 768.0],
            paint_last: None,
            add_menu_open: false,
            tool_modal: None,
            tool_modal_value: 0.0,
            rename_draft: None,
            context_menu: None,
            menu_open: None,
            loop_cut: None,
            autosave: petunia_core::AutosaveService::default(),
            pending_recovery: None,
            position: [
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
            ],
            rotation: [
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
            ],
            scale: [
                NumericFieldState::new(1.0, Some(0.001), None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, Some(0.001), None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, Some(0.001), None).with_steps(0.1, 0.01),
            ],
        };
        bridge.sync_viewport_context();
        bridge
    }

    pub fn apply(&mut self, intent: UiIntent) {
        match intent {
            UiIntent::SetWorkspace(workspace) => self.state.switch_workspace(workspace),
            UiIntent::SaveProject => {
                if let Some(path) = self.state.project.project_path.clone() {
                    self.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                } else {
                    self.state.set_status("Save As required");
                }
            }
            UiIntent::OpenCommandSearch => {
                self.command_search_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::CommandPalette,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::OpenSettings => {
                self.settings_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::Settings,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::ToggleSceneDrawer => {
                self.scene_drawer_visible = !self.scene_drawer_visible;
                if self.scene_drawer_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::SceneDrawer,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::SceneDrawer);
                }
            }
            UiIntent::ExecuteCommand(id) => {
                self.execute_command(id);
            }
            UiIntent::DismissTopOverlay => {
                self.handle_escape();
            }
            UiIntent::ScrubTransform {
                kind,
                axis,
                delta,
                fine,
            } => {
                self.scrub_transform(kind, axis, delta, fine);
            }
            UiIntent::ViewportGesture(gesture) => {
                self.apply_viewport_gesture(gesture);
            }
            UiIntent::SaveProjectTo(path) => {
                if let Err(err) = petunia_project::format::save(&self.state.project.project, &path)
                {
                    self.state.set_status(format!("failed to save: {err}"));
                } else {
                    self.state.project.project_path = Some(path.display().to_string());
                    self.state.mark_document_clean();
                    self.state
                        .set_status(format!("project saved: {}", path.display()));
                }
            }
            UiIntent::OpenProjectFrom(path) => match petunia_project::format::load(&path) {
                Ok(project) => {
                    self.state.project.palette = project.palette.clone();
                    self.state.project.project = project;
                    self.state.project.undo.clear();
                    self.state.project.project_path = Some(path.display().to_string());
                    self.state.mark_document_clean();
                    self.state
                        .set_status(format!("project opened: {}", path.display()));
                }
                Err(err) => {
                    self.state.set_status(format!("failed to open: {err}"));
                }
            },
            UiIntent::ImportModelFrom(path) => {
                match petunia_core::ProjectService::import_file_pipeline(
                    &mut self.state,
                    &path,
                    &petunia_project::pipeline::ImportOptions::default(),
                ) {
                    Ok(names) => self
                        .state
                        .set_status(format!("imported {} asset(s)", names.len())),
                    Err(error) => self.state.set_status(format!("import failed: {error}")),
                }
            }
            UiIntent::ExportActiveObjTo(path) => {
                let index = self.state.project.active;
                match petunia_core::ProjectService::export_obj(&self.state, index, &path) {
                    Ok(()) => self
                        .state
                        .set_status(format!("exported {}", path.display())),
                    Err(error) => self.state.set_status(format!("export failed: {error}")),
                }
            }
            UiIntent::ExportSceneGlbTo(path) => {
                let indices = self.state.project.export_selected_indices();
                match petunia_core::ProjectService::export_glb(&self.state, &indices, &path) {
                    Ok(()) => self
                        .state
                        .set_status(format!("exported {}", path.display())),
                    Err(error) => self.state.set_status(format!("export failed: {error}")),
                }
            }
            UiIntent::Undo => {
                if self.state.undo() {
                    self.state.set_status("Desfazer executado.");
                }
            }
            UiIntent::Redo => {
                if self.state.redo() {
                    self.state.set_status("Refazer executado.");
                }
            }
            UiIntent::SetSelectionDomain(domain) => {
                self.state.set_selection_domain(domain);
                self.state
                    .set_status(format!("Modo de seleção: {:?}", domain));
            }
            UiIntent::AddPrimitive(kind) => {
                self.state.begin_primitive(kind, None);
                self.state.confirm_primitive();
                self.state
                    .set_status(format!("Primitiva adicionada: {:?}", kind));
            }
            UiIntent::DeleteActiveAsset => {
                // `edit.delete` é contextual: em Object remove o asset ativo; em
                // Point/Edge/Face remove os sub-elementos selecionados.
                if let Err(error) = self.state.dispatch_command("edit.delete") {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::SetPaintColor(color) => {
                self.state.session.tools.paint_color = color;
            }
            UiIntent::SetBrushSize(size) => {
                self.state.session.tools.paint_radius = size.clamp(0.01, 100.0);
            }
            UiIntent::SetBrushOpacity(opacity) => {
                self.state.session.tools.paint_strength = opacity.clamp(0.0, 1.0);
            }
            UiIntent::SetActiveTool(tool) => {
                self.state.session.tools.active_tool = tool.clone();
                self.state.set_status(format!("Ferramenta ativa: {tool}"));
                match tool.as_str() {
                    // Transformações são transacionais por arrasto: a sessão
                    // modal abre no pointer-down da viewport, não ao escolher a
                    // ferramenta.
                    "move" | "rotate" | "scale" => {
                        self.state.session.tools.gizmo_mode = match tool.as_str() {
                            "rotate" => petunia_core::ModalKind::Rotate,
                            "scale" => petunia_core::ModalKind::Scale,
                            _ => petunia_core::ModalKind::Move,
                        };
                    }
                    "cut" => {
                        if let Some(mesh) = self.state.project.active_mesh().cloned() {
                            self.state.session.tools.cut_session =
                                Some(petunia_core::CutSession::new(mesh));
                            self.state
                                .set_status("Cut: choose two edge points in the viewport");
                        }
                    }
                    "fill" => {
                        let count =
                            petunia_module_paint::PaintModule::fill_selection(&mut self.state);
                        self.state.set_status(format!("Filled {count} points"));
                    }
                    _ => {}
                }
            }
            UiIntent::SelectSceneAsset(id_str) => {
                if let Some(idx) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                {
                    let id = self.state.project.assets[idx].id;
                    self.state.project.active = idx;
                    self.state.session.selection.asset = Some(id);
                    self.state.sync_selection();
                    self.state.mark_dirty();
                    self.reset_transform_fields();
                }
            }
            UiIntent::ToggleSceneAssetVisibility(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) =
                        self.state
                            .dispatch(&petunia_core::ToggleVisibilityAssetCmd {
                                asset_index: Some(asset_index),
                            })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::ToggleSceneAssetLock(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) = self.state.dispatch(&petunia_core::ToggleLockAssetCmd {
                        asset_index: Some(asset_index),
                    })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::SetTheme(theme_id) => {
                self.state.ui.active_theme_id = theme_id;
            }
            UiIntent::DuplicateActiveAsset => {
                let _ = self
                    .state
                    .dispatch(&petunia_core::DuplicateAssetCmd { asset_index: None });
                self.state.set_status("Objeto duplicado.");
            }
            UiIntent::SelectAll => {
                let _ = self.state.dispatch(&petunia_core::SelectAllCmd);
                self.state.set_status("Tudo selecionado.");
            }
            UiIntent::ClearSelection => {
                let _ = self.state.dispatch(&petunia_core::ClearSelectionCmd);
                self.state.set_status("Seleção limpa.");
            }
            UiIntent::InvertSelection => {
                let _ = self.state.dispatch(&petunia_core::InvertSelectionCmd);
                self.state.set_status("Seleção invertida.");
            }
            UiIntent::ToggleAssetLibrary => {
                self.asset_library_visible = !self.asset_library_visible;
                if self.asset_library_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::AssetLibrary,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::AssetLibrary);
                }
            }
            UiIntent::ResetCamera => {
                let _ = self.state.dispatch(&petunia_core::ResetCameraCmd);
                self.state.set_status("Câmera redefinida.");
            }
            UiIntent::ToggleProjection => {
                let _ = self.state.dispatch(&petunia_core::ToggleProjectionCmd);
                self.state.set_status(
                    if self.state.session.camera.proj == petunia_core::Projection::Ortho {
                        "Projeção: Ortográfica"
                    } else {
                        "Projeção: Perspectiva"
                    },
                );
            }
            UiIntent::SaveActiveAsAsset => {
                if self.state.save_active_as_asset() {
                    self.state
                        .set_status("Ativo salvo na biblioteca de assets.");
                }
            }
        }
        self.sync_viewport_context();
    }

    pub fn apply_viewport_gesture(&mut self, gesture: ViewportGesture) {
        match gesture {
            ViewportGesture::Orbit { dx, dy } => {
                self.state.session.camera.orbit(dx, dy);
            }
            ViewportGesture::Pan { dx, dy } => {
                self.state.session.camera.pan(dx, dy);
            }
            ViewportGesture::Zoom { delta } => {
                self.state.session.camera.zoom(delta);
            }
        }
        self.state.mark_dirty();
    }

    pub fn render_viewport(&mut self) -> Option<slint::Image> {
        let render_state = ViewportRenderState {
            shading: self.state.session.shading,
            xray: self.state.session.show_xray,
            show_triangulation: self.state.session.show_triangulation,
            textured: self.state.session.textured,
            show_wireframe_overlay: self.state.session.show_wireframe_overlay,
        };
        self.viewport.render_frame(
            &self.state.project,
            &self.state.session.camera,
            render_state,
        )
    }

    pub fn resize_viewport(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.viewport.resize(width, height);
        self.viewport_size = [width as f32, height as f32];
        self.state.session.camera.aspect = width as f32 / height as f32;
        self.state.mark_dirty();
    }

    /// Inicia uma transformação modal por arrasto na viewport.
    pub fn begin_viewport_transform(&mut self, kind: TransformKind, x: f32, y: f32) -> bool {
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        match self.state.begin_modal(modal_kind) {
            Ok(()) => {
                self.reset_transform_fields();
                self.drag = Some(ViewportDrag {
                    kind,
                    start: [x, y],
                    viewport: self.viewport_size,
                });
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Atualiza a transformação a partir do deslocamento absoluto do ponteiro.
    pub fn update_viewport_transform(&mut self, x: f32, y: f32) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let total_x = x - drag.start[0];
        let total_y = y - drag.start[1];
        let world_per_pixel =
            self.state.session.camera.visible_height() / drag.viewport[1].max(1.0);
        let result = match drag.kind {
            TransformKind::Position => {
                let right = self.state.session.camera.right();
                let up = self.state.session.camera.up();
                let delta = right * (total_x * world_per_pixel) + up * (-total_y * world_per_pixel);
                self.state.update_modal(delta, 0.0)
            }
            TransformKind::Rotation => self.state.update_modal(glam::Vec3::ZERO, total_x * 0.5),
            TransformKind::Scale => {
                let factor = (1.0 + total_x * 0.005).max(0.001);
                self.state.update_modal(glam::Vec3::ZERO, factor)
            }
        };
        match result {
            Ok(()) => {
                let components = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|modal| modal.components);
                if let Some(components) = components {
                    let fields = match drag.kind {
                        TransformKind::Position => &mut self.position,
                        TransformKind::Rotation => &mut self.rotation,
                        TransformKind::Scale => &mut self.scale,
                    };
                    fields[0].set_value(components.x);
                    fields[1].set_value(components.y);
                    fields[2].set_value(components.z);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Confirma a transformação por arrasto como uma única operação de undo.
    pub fn end_viewport_transform(&mut self) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        self.commit_transform();
        self.reset_transform_fields();
        true
    }

    pub fn cancel_viewport_transform(&mut self) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        self.cancel_transform()
    }

    /// Inicia um traço de pintura contínuo com transação única de undo.
    pub fn begin_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        if self.state.project.active_mesh().is_none() {
            self.state.set_status("No active mesh to paint");
            return false;
        }
        self.state.begin_paint_stroke();
        self.paint_dab_at(x, y);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Estende o traço interpolando em espaço de tela e pintando cada dab.
    pub fn paint_stroke_to(&mut self, x: f32, y: f32) -> bool {
        let Some(last) = self.paint_last else {
            return false;
        };
        let delta_x = x - last[0];
        let delta_y = y - last[1];
        let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();
        // ~3 px lógicos entre dabs deixam o traço contínuo sem buracos.
        let steps = (distance / 3.0).ceil().max(1.0) as usize;
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            self.paint_dab_at(last[0] + delta_x * t, last[1] + delta_y * t);
        }
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Confirma o traço como uma única entrada de undo.
    pub fn end_paint_stroke_at(&mut self) -> bool {
        if self.paint_last.take().is_none() {
            return false;
        }
        self.state.finish_paint_stroke(false);
        true
    }

    pub fn cancel_paint_stroke(&mut self) -> bool {
        if self.paint_last.take().is_none() {
            return false;
        }
        self.state.finish_paint_stroke(true);
        true
    }

    /// Redimensiona o dock de contexto pelo divisor vertical.
    ///
    /// Layout é estado de apresentação: não marca o documento como alterado.
    pub fn set_inspector_width(&mut self, width: f32) -> bool {
        self.state.ui.set_right_width(width)
    }

    /// Redimensiona a Asset Library pelo divisor horizontal.
    pub fn set_asset_library_height(&mut self, height: f32) -> bool {
        self.state.ui.set_shell_asset_library_height(height)
    }

    /// Abre, troca ou fecha um menu da barra superior.
    pub fn toggle_menu(&mut self, id: &str) -> bool {
        let next = MenuKind::ALL.into_iter().find(|kind| kind.id() == id);
        self.menu_open = match (self.menu_open, next) {
            (Some(current), Some(next)) if current == next => None,
            (_, next) => next,
        };
        match self.menu_open {
            Some(_) => self.overlays.push(OverlayEntry {
                id: OverlayId::MenuBar,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            }),
            None => {
                self.overlays.remove(OverlayId::MenuBar);
            }
        }
        self.state.mark_dirty();
        self.menu_open.is_some()
    }

    pub fn close_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::MenuBar);
        self.menu_open.take().is_some()
    }

    /// Executa um item de menu pelo id canônico que ele publica.
    pub fn menu_item_invoked(&mut self, id: &str) -> bool {
        self.close_menu();
        match id {
            // Itens de arquivo que abrem diálogo são despachados pelo mesmo
            // caminho assíncrono do `command-file-action`; aqui só o que resolve
            // de imediato.
            "file.new" => self.execute_core_command("file.new").is_ok(),
            "edit.undo" => {
                self.apply(UiIntent::Undo);
                true
            }
            "edit.redo" => {
                self.apply(UiIntent::Redo);
                true
            }
            "edit.duplicate" => {
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "window.command_palette" => {
                self.apply(UiIntent::OpenCommandSearch);
                true
            }
            "window.settings" => {
                self.apply(UiIntent::OpenSettings);
                true
            }
            other => self.execute_core_command(other).is_ok(),
        }
    }

    /// Um passo de autosave, respeitando intervalo e dirty state do domínio.
    ///
    /// Retorna `true` quando um snapshot foi gravado. Autosave nunca limpa o
    /// dirty state nem toca no arquivo oficial.
    pub fn autosave_tick(&mut self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dirty = self.state.is_document_dirty();
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        matches!(
            self.autosave
                .tick(now, dirty, &self.state.project.project, path_ref),
            Some(Ok(_))
        )
    }

    /// Carrega o snapshot de recuperação detectado no arranque.
    pub fn recover_pending(&mut self) -> bool {
        let Some(info) = self.pending_recovery.take() else {
            return false;
        };
        self.apply(UiIntent::OpenProjectFrom(info.snapshot_path));
        self.state
            .set_status(format!("Recovered snapshot of '{}'", info.project_name));
        true
    }

    /// Mantém o projeto oficial e encerra o aviso de recuperação.
    pub fn keep_saved_project(&mut self) -> bool {
        self.pending_recovery.take().is_some()
    }

    /// Descarta os snapshots de recuperação e o marcador de sessão.
    pub fn discard_pending_recovery(&mut self) -> bool {
        if self.pending_recovery.take().is_none() {
            return false;
        }
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        match petunia_core::AutosaveService::discard_recovery(path_ref) {
            Ok(()) => self.state.set_status("Recovery snapshots discarded"),
            Err(error) => self
                .state
                .set_status(format!("Failed to discard snapshots: {error}")),
        }
        true
    }

    /// Constrói a geometria 2D do editor UV a partir das UVs reais da malha.
    ///
    /// O quadrado 0..1 vira uma caixa de 256 px; cada face contribui um contorno
    /// fechado. Malhas grandes são truncadas e o editor avisa, em vez de
    /// construir uma string gigante em silêncio.
    fn build_uv_editor(&self) -> UvEditorModel {
        const BOX: f32 = 256.0;
        const MAX_FACES: usize = 2_000;
        let Some(mesh) = self.state.project.active_mesh() else {
            return UvEditorModel::default();
        };
        let face_count = mesh.faces.len();
        let mut commands = String::new();
        for face in mesh.faces.iter().take(MAX_FACES) {
            if face.uv.len() < 3 {
                continue;
            }
            for (index, uv) in face.uv.iter().enumerate() {
                if !uv[0].is_finite() || !uv[1].is_finite() {
                    continue;
                }
                let x = uv[0] * BOX;
                let y = (1.0 - uv[1]) * BOX;
                if index == 0 {
                    commands.push_str(&format!("M {x:.2} {y:.2} "));
                } else {
                    commands.push_str(&format!("L {x:.2} {y:.2} "));
                }
            }
            commands.push_str("Z ");
        }
        let islands = mesh.uv_islands();
        UvEditorModel {
            layout_commands: commands,
            island_count: islands.len(),
            face_count,
            selected_face: mesh
                .faces
                .iter()
                .position(|face| face.selected)
                .map(|index| index as i32)
                .unwrap_or(-1),
            truncated: face_count > MAX_FACES,
        }
    }

    /// Marca ou desmarca como costura todas as arestas da face UV selecionada.
    pub fn toggle_selected_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        let Some(face) = mesh.faces.iter().find(|face| face.selected) else {
            self.state
                .set_status("UV: select a face in the viewport first");
            return false;
        };
        let verts = face.verts.clone();
        if verts.len() < 3 {
            return false;
        }
        self.state.checkpoint("toggle uv seams");
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        for index in 0..verts.len() {
            let a = verts[index];
            let b = verts[(index + 1) % verts.len()];
            mesh.toggle_seam(a, b);
        }
        let count = self
            .state
            .project
            .active_mesh()
            .map(|mesh| mesh.uv_seams.len())
            .unwrap_or(0);
        self.state.set_status(format!(
            "UV seams on the selected face toggled ({count} total)"
        ));
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        true
    }

    /// Limpa todas as costuras da malha ativa.
    pub fn clear_all_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        if mesh.uv_seams.is_empty() {
            self.state.set_status("UV: there are no seams to clear");
            return false;
        }
        self.state.checkpoint("clear uv seams");
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            mesh.uv_seams.clear();
        }
        self.state.set_status("UV: all seams cleared");
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        true
    }

    /// Mutações do stack de camadas do workspace PAINT.
    ///
    /// Todas passam por `ensure_stack` + checkpoint + recomposição: o raster
    /// canônico é `Asset.paint_stack` e `Asset.texture` é só o cache composto.
    fn mutate_paint_stack(
        &mut self,
        label: &str,
        mutate: impl FnOnce(&mut petunia_project::paint_layers::PaintLayerStack) -> bool,
    ) -> bool {
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let active = self.state.project.active;
        let Some(asset) = self.state.project.assets.get_mut(active) else {
            return false;
        };
        let Some(stack) = asset.paint_stack.as_mut() else {
            return false;
        };
        if !mutate(stack) {
            return false;
        }
        self.state.checkpoint(label);
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        true
    }

    pub fn add_paint_layer(&mut self) -> bool {
        let (w, h) = self.paint_canvas_size();
        self.mutate_paint_stack("add paint layer", |stack| {
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new(
                format!("Layer {}", stack.layers.len() + 1),
                w,
                h,
                [0, 0, 0, 0],
            ));
            true
        })
    }

    pub fn add_paint_group(&mut self) -> bool {
        self.mutate_paint_stack("add paint group", |stack| {
            stack.add_group(format!("Group {}", stack.layers.len() + 1));
            true
        })
    }

    pub fn set_paint_layer_active(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("activate paint layer", |stack| stack.set_active(id))
    }

    pub fn toggle_paint_layer_visibility(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer visibility", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.visible = !layer.visible;
            true
        })
    }

    pub fn toggle_paint_layer_lock(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer lock", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.locked = !layer.locked;
            true
        })
    }

    pub fn remove_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("remove paint layer", |stack| {
            // A última camada é a base do raster: removê-la deixaria o asset sem
            // superfície de pintura.
            if stack.layers.len() <= 1 {
                return false;
            }
            stack.remove_layer(id)
        })
    }

    /// Move a camada em `delta` posições na ordem de composição.
    pub fn move_paint_layer(&mut self, id: &str, delta: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("reorder paint layer", |stack| {
            let Some(from) = stack.layers.iter().position(|layer| layer.id == id) else {
                return false;
            };
            let to = from as i32 + delta;
            if to < 0 || to as usize >= stack.layers.len() {
                return false;
            }
            stack.move_layer(from, to as usize)
        })
    }

    pub fn set_paint_layer_opacity(&mut self, id: &str, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let opacity = opacity.clamp(0.0, 1.0);
        self.mutate_paint_stack("paint layer opacity", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            if (layer.opacity - opacity).abs() < f32::EPSILON {
                return false;
            }
            layer.opacity = opacity;
            true
        })
    }

    fn paint_canvas_size(&self) -> (u32, u32) {
        self.state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
            .map(|canvas| (canvas.w, canvas.h))
            .unwrap_or((256, 256))
    }

    /// Abre a sessão de loop cut a partir da aresta selecionada.
    pub fn begin_loop_cut(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh().cloned() else {
            self.state.set_status("Loop Cut: no active mesh");
            return false;
        };
        let Some(seed) = mesh.selected_edges.iter().copied().next() else {
            self.state
                .set_status("Loop Cut: select an edge on a quad ring first");
            return false;
        };
        let Ok(ring) = petunia_core::LoopRing::discover(&mesh, seed) else {
            self.state
                .set_status("Loop Cut: the selected edge is not on a quad ring");
            return false;
        };
        self.loop_cut = Some(LoopCutSessionState {
            ring,
            cuts: 1,
            slide: 0.0,
            source: mesh,
        });
        self.state.session.tools.active_tool = "loop_cut".to_string();
        if !self.apply_loop_cut_preview() {
            self.loop_cut = None;
            return false;
        }
        self.state
            .set_status("Loop Cut: drag to slide, Enter confirms, Esc cancels");
        true
    }

    /// Ajusta o slide do loop cut e reconstrói a pré-visualização.
    pub fn scrub_loop_cut(&mut self, delta_x: f32, fine: bool) -> bool {
        let step = if fine { 0.0025 } else { 0.01 };
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        session.slide = (session.slide + delta_x * step).clamp(-1.0, 1.0);
        self.apply_loop_cut_preview()
    }

    /// Ajusta a quantidade de cortes paralelos (1..=32).
    pub fn set_loop_cut_count(&mut self, cuts: usize) -> bool {
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        let clamped = cuts.clamp(1, 32);
        if session.cuts == clamped {
            return false;
        }
        session.cuts = clamped;
        self.apply_loop_cut_preview()
    }

    /// Reconstrói a malha a partir do snapshot da sessão, nunca do preview.
    fn apply_loop_cut_preview(&mut self) -> bool {
        let Some(session) = self.loop_cut.as_ref() else {
            return false;
        };
        match session
            .ring
            .apply(&session.source, session.cuts, session.slide)
        {
            Ok(mesh) => {
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = mesh;
                }
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(format!("Loop Cut: {error}"));
                false
            }
        }
    }

    /// Confirma o loop cut como uma única operação de undo.
    pub fn commit_loop_cut(&mut self) -> bool {
        let Some(session) = self.loop_cut.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".to_string();
        // O checkpoint precisa capturar a malha ANTES do corte, então o preview
        // é desfeito primeiro e o resultado final é reaplicado depois.
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source.clone();
        }
        let Ok(cut) = session
            .ring
            .apply(&session.source, session.cuts, session.slide)
        else {
            self.state
                .set_status("Loop Cut: topology refused at commit");
            self.state.emit_mesh_changed();
            return false;
        };
        self.state.checkpoint("loop cut");
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = cut;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state
            .set_status(format!("Loop cut ({})", session.cuts));
        true
    }

    /// Abandona a sessão restaurando a malha original.
    pub fn cancel_loop_cut(&mut self) -> bool {
        let Some(session) = self.loop_cut.take() else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source;
        }
        self.state.session.tools.active_tool = "select".to_string();
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Loop Cut cancelled");
        true
    }

    /// Um clique de faca na viewport: primeiro ponto ancora, segundo corta.
    ///
    /// O ponto vem de `AppState::pick_edge`, então a faca corta a aresta que o
    /// usuário realmente apontou. O corte é aplicado como uma única transação.
    pub fn knife_click(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        if self.state.session.tools.cut_session.is_none() {
            return false;
        }
        let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let Some((edge, position)) = self.state.pick_edge(origin, direction) else {
            self.state.set_status("Knife: no edge under the cursor");
            return false;
        };
        let point = petunia_core::CutEdgePoint { edge, position };

        let Some(session) = self.state.session.tools.cut_session.as_mut() else {
            return false;
        };
        let Some(start) = session.edge_start else {
            session.edge_start = Some(point);
            session.anchor = Some([normalized_x, normalized_y]);
            self.state.set_status("Knife: pick the second edge point");
            self.state.mark_dirty();
            return true;
        };

        let Some(mesh) = self.state.project.active_mesh().cloned() else {
            return false;
        };
        match session.cut_knife_segment(start, point, &mesh) {
            Ok(cut) => {
                self.state.checkpoint("knife cut");
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = cut;
                }
                self.state.session.tools.cut_session = None;
                self.state.session.tools.active_tool = "select".to_string();
                self.state.sync_selection();
                self.state.emit_mesh_changed();
                self.state.set_status("Knife: cut applied");
                true
            }
            Err(error) => {
                // Topologia recusada: a sessão continua viva para o usuário
                // escolher outro ponto em vez de perder o primeiro.
                self.state.set_status(format!("Knife: {error}"));
                false
            }
        }
    }

    /// Cancela a faca mantendo a malha intacta.
    pub fn cancel_knife(&mut self) -> bool {
        if self.state.session.tools.cut_session.take().is_none() {
            return false;
        }
        self.state.session.tools.active_tool = "select".to_string();
        self.state.set_status("Knife cancelled");
        self.state.mark_dirty();
        true
    }

    /// Instancia uma cópia do asset da biblioteca no cursor 3D.
    ///
    /// É o caminho que o comando canônico `model.instantiate_asset` não tinha:
    /// ele exige um `asset_id`, então a palette não consegue disparar sozinha.
    pub fn place_asset(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.instantiate_asset_by_id(asset, None) {
            self.state.set_status("Asset not found in project library");
            return false;
        }
        self.state.mark_dirty();
        true
    }

    /// Abre o menu de contexto do Outliner sobre a linha de `id`.
    ///
    /// O alvo é selecionado antes de abrir: as ações do menu operam sobre ele e
    /// um clique-direito em linha não selecionada precisa agir no que o usuário
    /// apontou, não no que estava ativo.
    pub fn open_context_menu(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.project.assets.iter().any(|a| a.id == asset) {
            return false;
        }
        self.select_asset_by_id(asset);
        self.context_menu = Some(ContextMenuState { x, y, asset });
        self.overlays.push(OverlayEntry {
            id: OverlayId::OutlinerContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    pub fn close_context_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::OutlinerContextMenu);
        self.context_menu.take().is_some()
    }

    /// Executa uma ação do menu de contexto sobre o alvo apontado.
    pub fn context_menu_action(&mut self, action: &str) -> bool {
        let Some(menu) = self.context_menu else {
            return false;
        };
        let id = menu.asset.to_string();
        self.close_context_menu();
        match action {
            "rename" => self.begin_rename(),
            "duplicate" => {
                self.select_asset_by_id(menu.asset);
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "visibility" => {
                self.toggle_asset_visibility(&id);
                true
            }
            "lock" => {
                self.toggle_asset_lock(&id);
                true
            }
            "frame" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.state.dispatch_command("view.frame_selection");
                true
            }
            "delete" => {
                self.select_asset_by_id(menu.asset);
                self.apply(UiIntent::DeleteActiveAsset);
                true
            }
            _ => false,
        }
    }

    fn select_asset_by_id(&mut self, asset: uuid::Uuid) {
        if let Some(index) = self
            .state
            .project
            .assets
            .iter()
            .position(|candidate| candidate.id == asset)
        {
            self.state.project.active = index;
            self.state.sync_selection();
        }
    }

    fn toggle_asset_visibility(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetVisibility(id.to_string()));
    }

    fn toggle_asset_lock(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetLock(id.to_string()));
    }

    /// Abre a edição inline do nome do ativo selecionado.
    pub fn begin_rename(&mut self) -> bool {
        if self.rename_draft.is_some() {
            return true;
        }
        let Some(asset) = self.state.project.active() else {
            self.state
                .set_status(petunia_core::AssetRenameError::NoActiveAsset.to_string());
            return false;
        };
        self.rename_draft = Some(asset.name.clone());
        self.state.mark_dirty();
        true
    }

    /// Confirma o nome em edição. O domínio decide validade e histórico.
    pub fn commit_rename(&mut self, name: &str) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        match self.state.rename_active_asset(name) {
            Ok(_) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Abandona a edição sem tocar no documento.
    pub fn cancel_rename(&mut self) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        self.state.mark_dirty();
        true
    }

    /// Abre a sessão modal de uma ferramenta paramétrica com preview próprio.
    pub fn begin_tool_modal(&mut self, kind: ToolModalKind) -> bool {
        match self.state.begin_modal(kind.modal_kind()) {
            Ok(()) => {
                self.tool_modal = Some(kind);
                let initial = match kind {
                    ToolModalKind::Inset => 0.2,
                    ToolModalKind::Bevel => 0.05,
                    ToolModalKind::ScaleSelection => 1.0,
                    _ => 0.0,
                };
                self.tool_modal_value = initial;
                if initial != 0.0 {
                    let _ = self.state.update_modal(glam::Vec3::ZERO, initial);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Ajusta o preview pelo arrasto vertical na viewport.
    pub fn scrub_tool_modal(&mut self, delta_y: f32, fine: bool) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        let step = if fine { kind.step() * 0.1 } else { kind.step() };
        let world_per_pixel =
            self.state.session.camera.visible_height() / self.viewport_size[1].max(1.0);
        let delta = match kind {
            ToolModalKind::Inset => -delta_y * step * 0.5,
            ToolModalKind::Bevel => -delta_y * world_per_pixel * 0.5,
            ToolModalKind::ScaleSelection => -delta_y * step * 0.5,
            _ => -delta_y * world_per_pixel * 0.5,
        };
        self.set_tool_modal_value(self.tool_modal_value + delta)
    }

    /// Define o valor absoluto do preview (arrasto e campo numérico).
    pub fn set_tool_modal_value(&mut self, value: f32) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        let (minimum, maximum) = kind.bounds();
        let value = value.clamp(minimum, maximum);
        if self.state.update_modal(glam::Vec3::ZERO, value).is_err() {
            // Topologia recusada (ex.: bevel inválido): mantém o último preview.
            return false;
        }
        self.tool_modal_value = value;
        self.state.mark_dirty();
        true
    }

    /// Confirma a ferramenta paramétrica como uma única operação de undo.
    pub fn commit_tool_modal(&mut self) -> bool {
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.state.commit_modal();
        self.state.mark_dirty();
        true
    }

    pub fn cancel_tool_modal(&mut self) -> bool {
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.state.cancel_modal();
        self.state.mark_dirty();
        true
    }

    fn paint_dab_at(&mut self, x: f32, y: f32) {
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        let ndc_x = (x / width).clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y / height).clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let Some((face, hit)) = pick_face_hit(&self.state, origin, direction) else {
            return;
        };
        let tool = self.state.session.tools.active_tool.clone();
        let brush = match tool.as_str() {
            "eraser" => petunia_core::BrushType::Eraser,
            "fill" => petunia_core::BrushType::Fill,
            "picker" => return,
            _ => petunia_core::BrushType::Soft,
        };
        let radius = (self.state.session.tools.paint_radius * 8.0).max(1.0) as u32;
        let strength = self.state.session.tools.paint_strength;
        let isolate = self.state.session.tools.paint_isolate_selection;
        petunia_module_paint::PaintModule::paint_mesh_3d(
            &mut self.state,
            face,
            hit,
            brush,
            radius,
            strength,
            isolate,
        );
    }

    pub fn select_viewport(&mut self, normalized_x: f32, normalized_y: f32, extend: bool) {
        // A faca consome o clique antes da seleção: com uma sessão de corte
        // aberta, clicar é escolher ponto de aresta, não selecionar.
        if self.state.session.tools.cut_session.is_some()
            && self.knife_click(normalized_x, normalized_y)
        {
            return;
        }
        let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);

        if self.state.workspace == Workspace::Paint {
            if let Some((face, hit)) = pick_face_hit(&self.state, origin, direction) {
                let tool = self.state.session.tools.active_tool.as_str();
                let brush = match tool {
                    "eraser" => petunia_core::BrushType::Eraser,
                    "fill" => petunia_core::BrushType::Fill,
                    "picker" => petunia_core::BrushType::Eyedropper,
                    _ => petunia_core::BrushType::Soft,
                };
                if brush == petunia_core::BrushType::Eyedropper {
                    if let Some(mesh) = self.state.project.active_mesh()
                        && let Some(&vi) = mesh.faces.get(face).and_then(|f| f.verts.first())
                    {
                        petunia_module_paint::PaintModule::eyedrop_vertex(
                            &mut self.state,
                            vi as usize,
                        );
                    }
                } else {
                    let radius = (self.state.session.tools.paint_radius * 8.0).max(1.0) as u32;
                    let strength = self.state.session.tools.paint_strength;
                    let isolate = self.state.session.tools.paint_isolate_selection;
                    petunia_module_paint::PaintModule::paint_mesh_3d(
                        &mut self.state,
                        face,
                        hit,
                        brush,
                        radius,
                        strength,
                        isolate,
                    );
                }
            }
            self.state.mark_dirty();
            return;
        }

        match self.state.selection_domain() {
            SelectionDomain::Object => {
                let mut best = None;
                for (index, asset) in self.state.project.assets.iter().enumerate() {
                    if !asset.visible || asset.locked || asset.mesh.verts.is_empty() {
                        continue;
                    }
                    let center = glam::Vec3::from_array(asset.mesh.selection_center());
                    let along = (center - origin).dot(direction);
                    if along < 0.0 {
                        continue;
                    }
                    let distance = (center - (origin + direction * along)).length();
                    let radius = asset
                        .mesh
                        .verts
                        .iter()
                        .map(|vertex| (vertex.vec() - center).length())
                        .fold(0.0_f32, f32::max)
                        .max(0.15);
                    if distance <= radius && best.is_none_or(|(_, depth)| along < depth) {
                        best = Some((index, along));
                    }
                }
                if let Some((index, _)) = best {
                    self.state.project.active = index;
                    self.state.session.selection.asset = Some(self.state.project.assets[index].id);
                }
            }
            SelectionDomain::Vertex => {
                if let Some((index, _)) = self.state.pick_vertex(origin, direction)
                    && let Some(mesh) = self.state.project.active_mesh_mut()
                {
                    if !extend {
                        mesh.deselect_all();
                    }
                    if let Some(vertex) = mesh.verts.get_mut(index) {
                        vertex.selected = !extend || !vertex.selected;
                    }
                }
            }
            SelectionDomain::Face => {
                if let Some((face, _)) = pick_face_hit(&self.state, origin, direction)
                    && let Some(mesh) = self.state.project.active_mesh_mut()
                {
                    if !extend {
                        for current in &mut mesh.faces {
                            current.selected = false;
                        }
                    }
                    if let Some(current) = mesh.faces.get_mut(face) {
                        current.selected = true;
                    }
                }
            }
            SelectionDomain::Edge => {
                if let Some((edge, _)) = self.state.pick_edge(origin, direction)
                    && let Some(mesh) = self.state.project.active_mesh_mut()
                {
                    if !extend {
                        mesh.selected_edges.clear();
                        mesh.deselect_all();
                    }
                    mesh.selected_edges.insert(edge);
                    for vertex in [edge.0, edge.1] {
                        if let Some(vertex) = mesh.verts.get_mut(vertex as usize) {
                            vertex.selected = true;
                        }
                    }
                }
            }
        }
        self.state.sync_selection();
        self.state.mark_dirty();
        self.reset_transform_fields();
    }

    pub fn handle_escape(&mut self) -> bool {
        if self.close_menu() {
            return true;
        }
        if self.close_context_menu() {
            return true;
        }
        if self.cancel_rename() {
            return true;
        }
        if self.cancel_paint_stroke() {
            return true;
        }
        if self.cancel_tool_modal() {
            return true;
        }
        if self.drag.take().is_some() {
            self.cancel_transform();
            return true;
        }
        if self.cancel_loop_cut() {
            return true;
        }
        if self.cancel_knife() {
            return true;
        }
        if self.cancel_transform() {
            return true;
        }
        if let Some(entry) = self.overlays.esc() {
            self.hide_overlay(entry.id);
            true
        } else {
            false
        }
    }

    pub fn handle_click_away(&mut self) -> bool {
        if let Some(entry) = self.overlays.click_away() {
            self.hide_overlay(entry.id);
            true
        } else {
            false
        }
    }

    pub fn execute_command(&mut self, id: CommandId) {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        match id {
            CommandId::SaveProject => self.apply(UiIntent::SaveProject),
            CommandId::OpenProject => self.state.set_status("open requested by Slint frontend"),
            CommandId::Undo => self.apply(UiIntent::Undo),
            CommandId::Redo => self.apply(UiIntent::Redo),
            CommandId::AddCube => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube))
            }
            CommandId::AddSphere => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere))
            }
            CommandId::AddCylinder => self.apply(UiIntent::AddPrimitive(
                petunia_core::PrimitiveKind::Cylinder,
            )),
            CommandId::AddPlane => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Plane))
            }
            CommandId::DeleteSelected => self.apply(UiIntent::DeleteActiveAsset),
            CommandId::SelectModeObject => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object))
            }
            CommandId::SelectModePoint => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex))
            }
            CommandId::SelectModeEdge => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge))
            }
            CommandId::SelectModeFace => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face))
            }
            CommandId::PaintBrush => self.apply(UiIntent::SetActiveTool("brush".to_string())),
            CommandId::PaintEraser => self.apply(UiIntent::SetActiveTool("eraser".to_string())),
            CommandId::PaintFill => self.apply(UiIntent::SetActiveTool("fill".to_string())),
            CommandId::UvUnwrap => {
                if let Err(error) = self.state.dispatch_command("uv.unwrap_auto") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::UvPackIslands => {
                if let Err(error) = self.state.dispatch_command("uv.pack_islands") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::FrameSelection => {
                if let Err(error) = self.state.dispatch_command("view.frame_selection") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::ToggleWireframe => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wireframe") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::OpenSettings => self.apply(UiIntent::OpenSettings),
            CommandId::ToggleSceneDrawer => self.apply(UiIntent::ToggleSceneDrawer),
            CommandId::DuplicateSelected => self.apply(UiIntent::DuplicateActiveAsset),
            CommandId::SelectAll => self.apply(UiIntent::SelectAll),
            CommandId::ClearSelection => self.apply(UiIntent::ClearSelection),
            CommandId::InvertSelection => self.apply(UiIntent::InvertSelection),
            CommandId::ToggleAssetLibrary => self.apply(UiIntent::ToggleAssetLibrary),
            CommandId::ResetCamera => self.apply(UiIntent::ResetCamera),
            CommandId::ToggleProjection => self.apply(UiIntent::ToggleProjection),
            CommandId::SaveActiveAsAsset => self.apply(UiIntent::SaveActiveAsAsset),
        }
    }

    pub fn search_commands(&self, query: &str) -> Vec<petunia_core::CommandPaletteItem> {
        self.state.commands.query(query, &self.state)
    }

    pub fn execute_core_command(&mut self, id: &str) -> Result<(), petunia_core::CommandError> {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        // Ferramentas paramétricas abrem uma sessão modal com Tool Properties
        // próprias em vez de rodar como one-shot de valor fixo.
        if let Some(kind) = ToolModalKind::from_id(id) {
            self.begin_tool_modal(kind);
            return Ok(());
        }
        if id == "model.loop_cut" {
            self.begin_loop_cut();
            return Ok(());
        }
        match id {
            "uv.unwrap" => self.state.dispatch_command("uv.unwrap_auto"),
            "uv.pack_islands" => self.state.dispatch_command("uv.pack_islands"),
            "model.frame_selection" => self.state.dispatch_command("view.frame_selection"),
            other => self.state.dispatch_command(other),
        }
    }

    pub fn route_shortcut(&mut self, text: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        let Some(key) = input::key_code_from_slint(text) else {
            return false;
        };
        let mods = Mods2 { ctrl, shift, alt };
        let Some(action) = self.state.ui.keybinds.find(key, mods).map(str::to_owned) else {
            return false;
        };
        match action.as_str() {
            "global.undo" => self.apply(UiIntent::Undo),
            "global.redo" => self.apply(UiIntent::Redo),
            "global.reset_camera" => self.apply(UiIntent::ResetCamera),
            "global.toggle_projection" => self.apply(UiIntent::ToggleProjection),
            "model.select_object" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object))
            }
            "model.select_vertex" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex))
            }
            "model.select_edge" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge)),
            "model.select_face" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face)),
            "model.move" => self.apply(UiIntent::SetActiveTool("move".into())),
            "model.rotate" => self.apply(UiIntent::SetActiveTool("rotate".into())),
            "model.scale" => self.apply(UiIntent::SetActiveTool("scale".into())),
            "model.frame_selection" => {
                let _ = self.execute_core_command("view.frame_selection");
            }
            "model.extrude" => {
                let _ = self.execute_core_command("model.extrude");
            }
            "model.inset" => {
                let _ = self.execute_core_command("model.inset");
            }
            "model.bevel" => {
                let _ = self.execute_core_command("model.bevel");
            }
            "model.delete" => self.apply(UiIntent::DeleteActiveAsset),
            "model.transform" => self.apply(UiIntent::SetActiveTool("move".into())),
            "model.push_pull" => {
                let _ = self.execute_core_command("model.push_pull");
            }
            "model.knife" => {
                let _ = self.execute_core_command("model.knife");
            }
            "model.extrude_individual" => {
                let _ = self.execute_core_command("model.extrude_individual");
            }
            "model.subdivide" => {
                let _ = self.execute_core_command("model.subdivide");
            }
            "model.merge" => {
                let _ = self.execute_core_command("model.merge");
            }
            "model.loop_cut" => {
                let _ = self.execute_core_command("model.loop_cut");
            }
            "model.primitives" => {
                self.add_menu_open = true;
            }
            "paint.paint" => self.apply(UiIntent::SetActiveTool("brush".into())),
            "paint.size_decrease" => self.adjust_brush_size(-1.0),
            "paint.size_increase" => self.adjust_brush_size(1.0),
            "paint.hardness_decrease" => self.adjust_brush_hardness(-0.1),
            "paint.hardness_increase" => self.adjust_brush_hardness(0.1),
            "global.cycle_mode" => {
                let _ = self.execute_core_command("select.cycle_domain");
            }
            "global.rename" => {
                self.begin_rename();
            }
            "global.save_project" => self.apply(UiIntent::SaveProject),
            "global.help" => {
                let _ = self.execute_core_command("help.documentation");
            }
            _ => return false,
        }
        true
    }

    fn adjust_brush_size(&mut self, steps: f32) {
        let next = (self.state.session.tools.paint_radius + steps).clamp(0.01, 100.0);
        self.apply(UiIntent::SetBrushSize(next));
        self.state.set_status(format!("Brush size: {next:.2}"));
    }

    fn adjust_brush_hardness(&mut self, delta: f32) {
        let next = (self.state.session.tools.brush_hardness + delta).clamp(0.0, 1.0);
        self.state.session.tools.brush_hardness = next;
        self.state.mark_dirty();
        self.state.set_status(format!("Brush hardness: {next:.2}"));
    }

    pub fn scrub_transform(
        &mut self,
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    ) -> f32 {
        if self.state.session.tools.modal.is_none()
            && let Err(error) = self.begin_transform(kind)
        {
            self.state.set_status(error.to_string());
            return self.transform_value(kind, axis.min(2));
        }
        let axis = axis.min(2);
        let field = match kind {
            TransformKind::Position => &mut self.position[axis],
            TransformKind::Rotation => &mut self.rotation[axis],
            TransformKind::Scale => &mut self.scale[axis],
        };
        let new_val = field.scrub(delta, fine);
        let components = match kind {
            TransformKind::Position => self.position.map(|field| field.value()),
            TransformKind::Rotation => self.rotation.map(|field| field.value()),
            TransformKind::Scale => self.scale.map(|field| field.value()),
        };
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(components))
        {
            self.state.set_status(error.to_string());
        }
        new_val
    }

    pub fn begin_transform(&mut self, kind: TransformKind) -> Result<(), petunia_core::ModalError> {
        self.reset_transform_fields();
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        self.state.begin_modal(modal_kind)
    }

    pub fn commit_transform(&mut self) -> bool {
        self.state.commit_modal()
    }

    pub fn commit_transform_text(
        &mut self,
        kind: TransformKind,
        axis: usize,
        text: &str,
    ) -> Result<f32, numeric::NumericInputError> {
        let axis = axis.min(2);
        if self.state.session.tools.modal.is_none()
            && let Err(error) = self.begin_transform(kind)
        {
            self.state.set_status(error.to_string());
            return Err(numeric::NumericInputError::Invalid);
        }
        let field = match kind {
            TransformKind::Position => &mut self.position[axis],
            TransformKind::Rotation => &mut self.rotation[axis],
            TransformKind::Scale => &mut self.scale[axis],
        };
        field.begin_edit();
        let value = field.commit_text(text)?;
        let components = match kind {
            TransformKind::Position => self.position.map(|field| field.value()),
            TransformKind::Rotation => self.rotation.map(|field| field.value()),
            TransformKind::Scale => self.scale.map(|field| field.value()),
        };
        if self
            .state
            .update_modal_components(glam::Vec3::from_array(components))
            .is_ok()
        {
            self.state.commit_modal();
        }
        Ok(value)
    }

    pub fn cancel_transform(&mut self) -> bool {
        let cancelled = self.state.cancel_modal();
        if cancelled {
            self.reset_transform_fields();
        }
        cancelled
    }

    pub fn view_model(&self) -> ShellViewModel {
        let mut vm = ShellViewModel::from_state(&self.state);
        vm.position = [
            self.position[0].value(),
            self.position[1].value(),
            self.position[2].value(),
        ];
        vm.rotation = [
            self.rotation[0].value(),
            self.rotation[1].value(),
            self.rotation[2].value(),
        ];
        vm.scale = [
            self.scale[0].value(),
            self.scale[1].value(),
            self.scale[2].value(),
        ];
        vm.is_orthographic = self.state.session.camera.proj == petunia_core::Projection::Ortho;
        vm.is_wireframe = self.state.session.show_wireframe_overlay;
        vm.asset_library_visible = self.asset_library_visible;
        vm.gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.add_menu_open = self.add_menu_open;
        if let Some(draft) = &self.rename_draft {
            vm.rename_active = true;
            vm.rename_value = draft.clone();
        }
        if let Some(menu) = self.context_menu {
            vm.context_menu_open = true;
            vm.context_menu_x = menu.x;
            vm.context_menu_y = menu.y;
            if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == menu.asset)
            {
                vm.context_menu_title = asset.name.clone();
                vm.context_menu_visible = asset.visible;
                vm.context_menu_locked = asset.locked;
            }
        }
        if let Some(kind) = self.menu_open {
            vm.menu_open = kind.id().to_string();
        }
        let translated = |id: petunia_config::TextId| self.state.t_id(id);
        vm.label_parts = translated(petunia_config::text_id::UI_PARTS);
        vm.label_project_asset_library =
            translated(petunia_config::text_id::UI_PROJECT_ASSET_LIBRARY);
        vm.label_save_active_as_asset =
            translated(petunia_config::text_id::UI_SAVE_ACTIVE_AS_ASSET);
        vm.label_status_hint = translated(petunia_config::text_id::UI_STATUS_HINT);
        vm.label_unwrap_mesh = translated(petunia_config::text_id::UI_UNWRAP_MESH);
        vm.label_pack_islands = translated(petunia_config::text_id::UI_PACK_ISLANDS);
        vm.label_active_brush_color = translated(petunia_config::text_id::UI_ACTIVE_BRUSH_COLOR);
        vm.label_albedo_base_color = translated(petunia_config::text_id::UI_ALBEDO_BASE_COLOR);
        vm.label_theme = translated(petunia_config::text_id::UI_THEME);
        vm.label_place_in_scene = translated(petunia_config::text_id::UI_PLACE_IN_SCENE);
        if let Some(info) = &self.pending_recovery {
            vm.recovery_open = true;
            vm.recovery_title = translated(petunia_config::text_id::UI_RECOVERY_TITLE);
            vm.recovery_body = translated(petunia_config::text_id::UI_RECOVERY_BODY);
            vm.recovery_detail = format!(
                "{}  ·  snapshot {}  ·  {}",
                info.project_name,
                info.snapshot_time,
                info.snapshot_path.display()
            );
            vm.recovery_recover = translated(petunia_config::text_id::UI_RECOVERY_RECOVER);
            vm.recovery_keep = translated(petunia_config::text_id::UI_RECOVERY_KEEP);
            vm.recovery_discard = translated(petunia_config::text_id::UI_RECOVERY_DISCARD);
        }
        vm.label_asset_library = translated(petunia_config::text_id::UI_ASSETS);
        vm.label_preferences = translated(petunia_config::text_id::MENU_PREFERENCES);
        // A linha de rodapé das preferências informa o keymap e o idioma REAIS em uso.
        vm.shell_info = format!(
            "{}: {}  ·  {}: {}",
            translated(petunia_config::text_id::UI_THEME),
            self.state.ui.active_theme_id,
            "Keymap",
            self.state.ui.active_keymap_id,
        );
        vm.label_apply = translated(petunia_config::text_id::ACTIONS_APPLY);
        vm.label_cancel = translated(petunia_config::text_id::ACTIONS_CANCEL);
        vm.label_delete = translated(petunia_config::text_id::ACTIONS_DELETE);
        vm.label_duplicate = translated(petunia_config::text_id::ACTIONS_DUPLICATE);
        vm.themes = petunia_config::theme::ThemeRegistry::global()
            .available()
            .iter()
            .map(|manifest| ThemeEntryModel {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                active: manifest.id == self.state.ui.active_theme_id,
            })
            .collect();
        for kind in MenuKind::ALL {
            let entries: Vec<MenuEntryModel> = kind
                .items()
                .iter()
                .map(|(id, label, shortcut)| MenuEntryModel {
                    id: (*id).to_string(),
                    label: translated(*label),
                    shortcut: (*shortcut).to_string(),
                })
                .collect();
            match kind {
                MenuKind::File => {
                    vm.menu_file_label = translated(kind.title());
                    vm.menu_file_items = entries;
                }
                MenuKind::Edit => {
                    vm.menu_edit_label = translated(kind.title());
                    vm.menu_edit_items = entries;
                }
                MenuKind::View => {
                    vm.menu_view_label = translated(kind.title());
                    vm.menu_view_items = entries;
                }
                MenuKind::Window => {
                    vm.menu_window_label = translated(kind.title());
                    vm.menu_window_items = entries;
                }
            }
        }
        vm.uv_editor = self.build_uv_editor();
        if let Some(stack) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
        {
            vm.paint_layers = stack
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| PaintLayerModel {
                    id: layer.id.to_string(),
                    name: layer.name.clone(),
                    visible: layer.visible,
                    locked: layer.locked,
                    opacity: layer.opacity,
                    active: index == stack.active_layer,
                    is_group: layer.is_group,
                    kind_label: match layer.kind {
                        petunia_project::paint_layers::LayerKind::Raster(_) => "Raster",
                        petunia_project::paint_layers::LayerKind::Decal(_) => "Decal",
                        petunia_project::paint_layers::LayerKind::Effect(_) => "Effect",
                    }
                    .to_string(),
                })
                .collect();
            let (width, height) = self
                .state
                .project
                .assets
                .get(self.state.project.active)
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
                .map(|canvas| (canvas.w, canvas.h))
                .unwrap_or((0, 0));
            vm.paint_layer_count =
                format!("{} layer(s)  ·  {width} × {height}", stack.layers.len());
        }
        if let Some(session) = &self.loop_cut {
            vm.loop_cut_active = true;
            vm.loop_cut_slide = session.slide;
            vm.loop_cut_cuts = session.cuts as i32;
        }
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            vm.tool_modal_active = true;
            vm.tool_modal_title = kind.title().to_string();
            vm.tool_modal_label = kind.label().to_string();
            vm.tool_modal_value = self.tool_modal_value;
            vm.tool_modal_step = kind.step();
            vm.tool_modal_min = minimum;
            vm.tool_modal_max = maximum;
        }
        vm
    }

    fn sync_viewport_context(&mut self) {
        self.viewport.set_workspace(self.state.workspace);
        self.viewport
            .set_selection_domain(self.state.selection_domain());
    }

    fn reset_transform_fields(&mut self) {
        for field in &mut self.position {
            field.set_value(0.0);
        }
        for field in &mut self.rotation {
            field.set_value(0.0);
        }
        for field in &mut self.scale {
            field.set_value(1.0);
        }
    }

    fn transform_value(&self, kind: TransformKind, axis: usize) -> f32 {
        match kind {
            TransformKind::Position => self.position[axis].value(),
            TransformKind::Rotation => self.rotation[axis].value(),
            TransformKind::Scale => self.scale[axis].value(),
        }
    }

    fn hide_overlay(&mut self, id: OverlayId) {
        match id {
            OverlayId::CommandPalette => self.command_search_visible = false,
            OverlayId::Settings => self.settings_visible = false,
            OverlayId::SceneDrawer => self.scene_drawer_visible = false,
            OverlayId::AssetLibrary => self.asset_library_visible = false,
            OverlayId::OutlinerContextMenu => self.context_menu = None,
            OverlayId::MenuBar => self.menu_open = None,
        }
    }
}

/// Projeta o pivô da seleção e os três eixos do mundo para o overlay Slint.
fn compute_gizmo(state: &AppState, width: f32, height: f32) -> GizmoModel {
    if state.workspace != Workspace::Model {
        return GizmoModel::default();
    }
    let Some(asset) = state.project.active() else {
        return GizmoModel::default();
    };
    if asset.mesh.verts.is_empty() {
        return GizmoModel::default();
    }
    let pivot = if asset.mesh.has_selection() {
        glam::Vec3::from_array(asset.mesh.selection_center())
    } else {
        let mut min = glam::Vec3::splat(f32::MAX);
        let mut max = glam::Vec3::splat(f32::MIN);
        for vertex in &asset.mesh.verts {
            let point = vertex.vec();
            min = min.min(point);
            max = max.max(point);
        }
        if !min.is_finite() || !max.is_finite() {
            return GizmoModel::default();
        }
        (min + max) * 0.5
    };
    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };
    let Some(origin) = project(pivot) else {
        return GizmoModel::default();
    };
    let world_length = state.session.camera.visible_height() * 0.18;
    let end_of = |direction: glam::Vec3| -> [f32; 2] {
        project(pivot + direction * world_length).unwrap_or(origin)
    };
    let x_end = end_of(glam::Vec3::X);
    let y_end = end_of(glam::Vec3::Y);
    let z_end = end_of(glam::Vec3::Z);
    GizmoModel {
        visible: true,
        origin_x: origin[0],
        origin_y: origin[1],
        x_end_x: x_end[0],
        x_end_y: x_end[1],
        y_end_x: y_end[0],
        y_end_y: y_end[1],
        z_end_x: z_end[0],
        z_end_y: z_end[1],
    }
}

fn pick_face_hit(
    state: &AppState,
    origin: glam::Vec3,
    direction: glam::Vec3,
) -> Option<(usize, glam::Vec3)> {
    let mesh = state.project.active_mesh()?;
    let mut best = None;
    for (face_index, face) in mesh.faces.iter().enumerate() {
        if face.verts.len() < 3 {
            continue;
        }
        let p0 = mesh.verts[face.verts[0] as usize].vec();
        for triangle in 1..face.verts.len() - 1 {
            let p1 = mesh.verts[face.verts[triangle] as usize].vec();
            let p2 = mesh.verts[face.verts[triangle + 1] as usize].vec();
            if let Some(distance) = ray_triangle(origin, direction, p0, p1, p2)
                && best.is_none_or(|(_, current)| distance < current)
            {
                best = Some((face_index, distance));
            }
        }
    }
    best.map(|(face, distance)| (face, origin + direction * distance))
}

fn ray_triangle(
    origin: glam::Vec3,
    direction: glam::Vec3,
    p0: glam::Vec3,
    p1: glam::Vec3,
    p2: glam::Vec3,
) -> Option<f32> {
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let pvec = direction.cross(edge2);
    let det = edge1.dot(pvec);
    if det.abs() < 1e-7 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = origin - p0;
    let u = tvec.dot(pvec) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(edge1);
    let v = direction.dot(qvec) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(qvec) * inv;
    (distance > 1e-4).then_some(distance)
}

/// Executa o frontend de produção com composição WGPU direta ou fallback software.
pub fn run() -> Result<(), slint::PlatformError> {
    let gpu_context = viewport_gpu::WgpuViewport::create_wgpu_context().ok();
    if let Some((instance, adapter, device, queue)) = gpu_context.as_ref() {
        slint::BackendSelector::new()
            .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::Manual {
                instance: instance.clone(),
                adapter: adapter.clone(),
                device: device.clone(),
                queue: queue.clone(),
            })
            .select()?;
    }

    println!("Petunia3D - Slint production frontend");
    let window = PetuniaSlintShell::new()?;
    let state = AppState::default();
    let has_gpu_viewport = gpu_context.is_some();

    let mut viewport: Box<dyn PetuniaViewport> = if let Some((_, _, device, queue)) = gpu_context {
        println!("Viewport backend: shared WGPU fast path");
        Box::new(viewport_gpu::WgpuViewport::new(
            Arc::new(device),
            Arc::new(queue),
            1024,
            768,
        ))
    } else {
        println!("Viewport backend: software compatibility path");
        Box::new(Software3dViewport::new(1024, 768))
    };

    let render_state = ViewportRenderState {
        shading: state.session.shading,
        xray: state.session.show_xray,
        show_triangulation: state.session.show_triangulation,
        textured: state.session.textured,
        show_wireframe_overlay: state.session.show_wireframe_overlay,
    };
    if let Some(frame) = viewport.render_frame(&state.project, &state.session.camera, render_state)
    {
        window.set_viewport_image(frame);
    }
    window.set_has_gpu_viewport(has_gpu_viewport);

    let bridge = Arc::new(Mutex::new(SlintUiBridge::new(state, viewport)));

    // Ciclo de vida do autosave (P3D-002): marcador de sessão no arranque,
    // detecção de encerramento sujo e remoção no fechamento limpo.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    {
        let mut bridge = bridge
            .lock()
            .expect("Slint bridge mutex poisoned during startup");
        bridge.pending_recovery = petunia_core::AutosaveService::detect_recovery(None);
        let project_name = bridge
            .state
            .project
            .project_path
            .clone()
            .unwrap_or_else(|| "Untitled".to_string());
        let path = bridge.state.project.project_path.clone();
        if let Err(error) = petunia_core::AutosaveService::create_session_lock(
            path.as_deref().map(std::path::Path::new),
            &project_name,
            now_secs,
        ) {
            eprintln!("petunia3d: falha ao gravar marcador de sessão: {error}");
        }
    }

    connect_callbacks(&window, Arc::clone(&bridge));
    let vm = bridge
        .lock()
        .expect("Slint bridge mutex poisoned during startup")
        .view_model();
    sync_window_properties(&window, &vm);

    // Um passo de autosave a cada 30s; o intervalo real (120s) e o dirty state
    // são decididos pelo domínio, então o timer só oferece a oportunidade.
    let autosave_bridge = Arc::clone(&bridge);
    let autosave_timer = slint::Timer::default();
    autosave_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(30),
        move || {
            if let Ok(mut bridge) = autosave_bridge.lock() {
                bridge.autosave_tick();
            }
        },
    );

    println!("Petunia3D window ready");
    let result = window.run();

    drop(autosave_timer);
    let path = bridge
        .lock()
        .map(|bridge| bridge.state.project.project_path.clone())
        .unwrap_or(None);
    petunia_core::AutosaveService::remove_session_lock(path.as_deref().map(std::path::Path::new));
    result
}

fn sync_window_properties(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    window.set_active_workspace(vm.workspace_label().into());
    window.set_saved(vm.saved);
    window.set_can_undo(vm.can_undo);
    window.set_can_redo(vm.can_redo);
    window.set_status_message(vm.status_message.as_str().into());
    window.set_active_tool(vm.active_tool.as_str().into());
    let domain_str = match vm.selection_domain {
        SelectionDomain::Object => "OBJECT",
        SelectionDomain::Vertex => "POINT",
        SelectionDomain::Edge => "EDGE",
        SelectionDomain::Face => "FACE",
    };
    window.set_selection_domain(domain_str.into());
    window.set_is_orthographic(vm.is_orthographic);
    window.set_is_wireframe(vm.is_wireframe);
    window.set_asset_library_visible(vm.asset_library_visible);
    window.set_paint_color(slint::Color::from_argb_f32(
        1.0,
        vm.paint_color[0],
        vm.paint_color[1],
        vm.paint_color[2],
    ));
    window.set_brush_size(vm.brush_size);
    window.set_brush_opacity(vm.brush_opacity);

    window.set_pos_x(vm.position[0]);
    window.set_pos_y(vm.position[1]);
    window.set_pos_z(vm.position[2]);
    window.set_rot_x(vm.rotation[0]);
    window.set_rot_y(vm.rotation[1]);
    window.set_rot_z(vm.rotation[2]);
    window.set_scale_x(vm.scale[0]);
    window.set_scale_y(vm.scale[1]);
    window.set_scale_z(vm.scale[2]);

    window.set_active_object_title(vm.active_object_title.as_str().into());
    window.set_active_object_details(vm.active_object_details.as_str().into());
    window.set_active_material_name(vm.active_material_name.as_str().into());
    window.set_scene_stats(vm.scene_stats.as_str().into());
    window.set_uv_stats(vm.uv_stats.as_str().into());
    window.set_current_theme(vm.current_theme.as_str().into());

    let scene_items: Vec<SceneItem> = vm
        .scene_items
        .iter()
        .map(|item| SceneItem {
            id: item.id.as_str().into(),
            name: item.name.as_str().into(),
            visible: item.visible,
            locked: item.locked,
            selected: item.selected,
            verts: item.verts as i32,
            tris: item.tris as i32,
        })
        .collect();
    let model = std::rc::Rc::new(slint::VecModel::from(scene_items));
    window.set_scene_items(model.into());

    window.set_gizmo_visible(vm.gizmo.visible);
    window.set_gizmo_origin_x(vm.gizmo.origin_x);
    window.set_gizmo_origin_y(vm.gizmo.origin_y);
    window.set_gizmo_x_end_x(vm.gizmo.x_end_x);
    window.set_gizmo_x_end_y(vm.gizmo.x_end_y);
    window.set_gizmo_y_end_x(vm.gizmo.y_end_x);
    window.set_gizmo_y_end_y(vm.gizmo.y_end_y);
    window.set_gizmo_z_end_x(vm.gizmo.z_end_x);
    window.set_gizmo_z_end_y(vm.gizmo.z_end_y);
    window.set_add_menu_open(vm.add_menu_open);
    window.set_rename_active(vm.rename_active);
    window.set_rename_value(vm.rename_value.as_str().into());
    window.set_context_menu_open(vm.context_menu_open);
    window.set_context_menu_x(vm.context_menu_x);
    window.set_context_menu_y(vm.context_menu_y);
    window.set_context_menu_title(vm.context_menu_title.as_str().into());
    window.set_context_menu_visible(vm.context_menu_visible);
    window.set_context_menu_locked(vm.context_menu_locked);
    window.set_menu_open(vm.menu_open.as_str().into());
    window.set_menu_file_label(vm.menu_file_label.as_str().into());
    window.set_menu_edit_label(vm.menu_edit_label.as_str().into());
    window.set_menu_view_label(vm.menu_view_label.as_str().into());
    window.set_menu_window_label(vm.menu_window_label.as_str().into());
    let to_entries = |items: &[MenuEntryModel]| -> Vec<MenuEntry> {
        items
            .iter()
            .map(|item| MenuEntry {
                id: item.id.as_str().into(),
                label: item.label.as_str().into(),
                shortcut: item.shortcut.as_str().into(),
            })
            .collect()
    };
    window.set_menu_file_items(to_entries(&vm.menu_file_items).as_slice().into());
    window.set_menu_edit_items(to_entries(&vm.menu_edit_items).as_slice().into());
    window.set_menu_view_items(to_entries(&vm.menu_view_items).as_slice().into());
    window.set_menu_window_items(to_entries(&vm.menu_window_items).as_slice().into());
    window.set_label_parts(vm.label_parts.as_str().into());
    window.set_label_project_asset_library(vm.label_project_asset_library.as_str().into());
    window.set_label_save_active_as_asset(vm.label_save_active_as_asset.as_str().into());
    window.set_label_status_hint(vm.label_status_hint.as_str().into());
    window.set_label_unwrap_mesh(vm.label_unwrap_mesh.as_str().into());
    window.set_label_pack_islands(vm.label_pack_islands.as_str().into());
    window.set_label_active_brush_color(vm.label_active_brush_color.as_str().into());
    window.set_label_albedo_base_color(vm.label_albedo_base_color.as_str().into());
    window.set_label_theme(vm.label_theme.as_str().into());
    window.set_label_place_in_scene(vm.label_place_in_scene.as_str().into());
    window.set_recovery_open(vm.recovery_open);
    window.set_recovery_title(vm.recovery_title.as_str().into());
    window.set_recovery_body(vm.recovery_body.as_str().into());
    window.set_recovery_detail(vm.recovery_detail.as_str().into());
    window.set_recovery_recover(vm.recovery_recover.as_str().into());
    window.set_recovery_keep(vm.recovery_keep.as_str().into());
    window.set_recovery_discard(vm.recovery_discard.as_str().into());
    window.set_label_asset_library(vm.label_asset_library.as_str().into());
    window.set_label_preferences(vm.label_preferences.as_str().into());
    window.set_shell_info(vm.shell_info.as_str().into());
    window.set_label_apply(vm.label_apply.as_str().into());
    window.set_label_cancel(vm.label_cancel.as_str().into());
    window.set_label_delete(vm.label_delete.as_str().into());
    window.set_label_duplicate(vm.label_duplicate.as_str().into());
    let theme_entries: Vec<ThemeEntry> = vm
        .themes
        .iter()
        .map(|theme| ThemeEntry {
            id: theme.id.as_str().into(),
            name: theme.name.as_str().into(),
            active: theme.active,
        })
        .collect();
    window.set_themes(theme_entries.as_slice().into());
    window.set_inspector_width(vm.inspector_width);
    window.set_asset_library_height(vm.asset_library_height);
    window.set_uv_layout_commands(vm.uv_editor.layout_commands.as_str().into());
    window.set_uv_island_count(vm.uv_editor.island_count as i32);
    window.set_uv_face_count(vm.uv_editor.face_count as i32);
    window.set_uv_selected_face(vm.uv_editor.selected_face);
    window.set_uv_layout_truncated(vm.uv_editor.truncated);
    window.set_paint_layer_count(vm.paint_layer_count.as_str().into());
    let layer_entries: Vec<PaintLayerEntry> = vm
        .paint_layers
        .iter()
        .map(|layer| PaintLayerEntry {
            id: layer.id.as_str().into(),
            name: layer.name.as_str().into(),
            visible: layer.visible,
            locked: layer.locked,
            opacity: layer.opacity,
            active: layer.active,
            is_group: layer.is_group,
            kind_label: layer.kind_label.as_str().into(),
        })
        .collect();
    window.set_paint_layers(layer_entries.as_slice().into());
    window.set_loop_cut_active(vm.loop_cut_active);
    window.set_loop_cut_slide(vm.loop_cut_slide);
    window.set_loop_cut_cuts(vm.loop_cut_cuts);
    window.set_tool_modal_active(vm.tool_modal_active);
    window.set_tool_modal_title(vm.tool_modal_title.as_str().into());
    window.set_tool_modal_label(vm.tool_modal_label.as_str().into());
    window.set_tool_modal_value(vm.tool_modal_value);
    window.set_tool_modal_step(vm.tool_modal_step);
    window.set_tool_modal_min(vm.tool_modal_min);
    window.set_tool_modal_max(vm.tool_modal_max);

    theme::apply_theme(window, &vm.current_theme);
}

fn connect_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let shortcut_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shortcut_requested(move |text, ctrl, shift, alt| {
        if let Ok(mut bridge) = shortcut_bridge.lock() {
            bridge.route_shortcut(text.as_str(), ctrl, shift, alt);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
    let workspace_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_workspace_changed(move |workspace| {
        let workspace = match workspace.as_str() {
            "MODEL" => Workspace::Model,
            "PAINT" => Workspace::Paint,
            "UV" => Workspace::Uv,
            _ => return,
        };
        if let Ok(mut bridge) = workspace_bridge.lock() {
            bridge.apply(UiIntent::SetWorkspace(workspace));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let save_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_requested(move || {
        let existing = save_bridge
            .lock()
            .ok()
            .and_then(|bridge| bridge.state.project.project_path.clone());
        if let Some(path) = existing {
            if let Ok(mut bridge) = save_bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
            return;
        }
        let bridge = Arc::clone(&save_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.save_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::SaveProjectTo(path));
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    window.set_status_message(vm.status_message.as_str().into());
                }
            }
        });
    });

    let open_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_open_project_requested(move || {
        let bridge = Arc::clone(&open_bridge);
        let window_weak = window_weak.clone();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let Some(path) = service.open_project().await else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                bridge.apply(UiIntent::OpenProjectFrom(path));
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    window.set_saved(vm.saved);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let file_command_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_file_action(move |id| {
        let bridge = Arc::clone(&file_command_bridge);
        let window_weak = window_weak.clone();
        let id = id.to_string();
        let _ = slint::spawn_local(async move {
            let service = files::FileDialogService::new();
            let path = match id.as_str() {
                "file.open" => service.open_project().await,
                "file.save" | "file.save_as" => service.save_project().await,
                "file.import_obj" => service.import_model().await,
                "file.export_obj" => service.export_obj().await,
                "file.export_glb" => service.export_glb().await,
                _ => None,
            };
            let Some(path) = path else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                let intent = match id.as_str() {
                    "file.open" => UiIntent::OpenProjectFrom(path),
                    "file.import_obj" => UiIntent::ImportModelFrom(path),
                    "file.export_obj" => UiIntent::ExportActiveObjTo(path),
                    "file.export_glb" => UiIntent::ExportSceneGlbTo(path),
                    _ => UiIntent::SaveProjectTo(path),
                };
                let needs_render = matches!(id.as_str(), "file.open" | "file.import_obj");
                bridge.apply(intent);
                bridge.command_search_visible = false;
                bridge.overlays.remove(OverlayId::CommandPalette);
                let vm = bridge.view_model();
                let new_frame = if needs_render {
                    bridge.render_viewport()
                } else {
                    None
                };
                if let Some(window) = window_weak.upgrade() {
                    window.set_command_search_visible(false);
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
            }
        });
    });

    let search_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_search_requested(move || {
        if let Ok(mut bridge) = search_bridge.lock() {
            bridge.apply(UiIntent::OpenCommandSearch);
            let items: Vec<CommandItem> = bridge
                .search_commands("")
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(true);
                let model = std::rc::Rc::new(slint::VecModel::from(items));
                window.set_command_results(model.into());
            }
        }
    });

    let settings_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_settings_requested(move || {
        if let Ok(mut bridge) = settings_bridge.lock() {
            bridge.apply(UiIntent::OpenSettings);
            if let Some(window) = window_weak.upgrade() {
                window.set_settings_visible(true);
            }
        }
    });

    let scene_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_requested(move || {
        if let Ok(mut bridge) = scene_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneDrawer);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let scene_pin_bridge = Arc::clone(&bridge);
    window.on_scene_pin_requested(move |pinned| {
        if let Ok(mut bridge) = scene_pin_bridge.lock() {
            bridge.overlays.set_pinned(OverlayId::SceneDrawer, pinned);
        }
    });

    let query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_query_changed(move |query| {
        let items: Vec<CommandItem> = if let Ok(bridge) = query_bridge.lock() {
            bridge
                .search_commands(query.as_str())
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.into(),
                    label: cmd.label.into(),
                    shortcut: cmd.shortcut.unwrap_or_default().into(),
                    available: cmd.is_available,
                    disabled_reason: cmd.disabled_reason.unwrap_or_default().into(),
                })
                .collect()
        } else {
            Vec::new()
        };
        if let Some(window) = window_weak.upgrade() {
            let model = std::rc::Rc::new(slint::VecModel::from(items));
            window.set_command_results(model.into());
        }
    });

    let exec_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_executed(move |id_str| {
        if let Ok(mut bridge) = exec_bridge.lock() {
            let result = bridge.execute_core_command(id_str.as_str());
            if let Err(error) = result {
                bridge.state.set_status(error.to_string());
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(bridge.command_search_visible);
                window.set_settings_visible(bridge.settings_visible);
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let esc_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_escape_requested(move || {
        if let Ok(mut bridge) = esc_bridge.lock() {
            bridge.apply(UiIntent::DismissTopOverlay);
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(bridge.command_search_visible);
                window.set_settings_visible(bridge.settings_visible);
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                window.set_asset_library_visible(bridge.asset_library_visible);
            }
        }
    });

    let click_away_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_click_away_requested(move || {
        if let Ok(mut bridge) = click_away_bridge.lock() {
            bridge.handle_click_away();
            if let Some(window) = window_weak.upgrade() {
                window.set_command_search_visible(bridge.command_search_visible);
                window.set_settings_visible(bridge.settings_visible);
                window.set_scene_drawer_visible(bridge.scene_drawer_visible);
                window.set_asset_library_visible(bridge.asset_library_visible);
            }
        }
    });

    let transform_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrubbed(move |kind_str, axis, delta, fine| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_bridge.lock() {
            let val = bridge.scrub_transform(kind, axis as usize, delta, fine);
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                match (kind, axis) {
                    (TransformKind::Position, 0) => window.set_pos_x(val),
                    (TransformKind::Position, 1) => window.set_pos_y(val),
                    (TransformKind::Position, 2) => window.set_pos_z(val),
                    (TransformKind::Rotation, 0) => window.set_rot_x(val),
                    (TransformKind::Rotation, 1) => window.set_rot_y(val),
                    (TransformKind::Rotation, 2) => window.set_rot_z(val),
                    (TransformKind::Scale, 0) => window.set_scale_x(val),
                    (TransformKind::Scale, 1) => window.set_scale_y(val),
                    (TransformKind::Scale, 2) => window.set_scale_z(val),
                    _ => {}
                }
            }
        }
    });

    let transform_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_started(move |kind_str| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock()
            && let Err(error) = bridge.begin_transform(kind)
        {
            bridge.state.set_status(error.to_string());
            if let Some(window) = window_weak.upgrade() {
                window.set_status_message(error.to_string().into());
            }
        }
    });

    let transform_finish_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_finished(move || {
        if let Ok(mut bridge) = transform_finish_bridge.lock() {
            bridge.commit_transform();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let transform_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_text_committed(move |kind_str, axis, text| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_text_bridge.lock() {
            if let Err(error) = bridge.commit_transform_text(kind, axis as usize, text.as_str()) {
                bridge
                    .state
                    .set_status(format!("Invalid numeric value: {error:?}"));
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let orbit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_orbit(move |dx, dy| {
        if let Ok(mut bridge) = orbit_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Orbit { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let pan_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_pan(move |dx, dy| {
        if let Ok(mut bridge) = pan_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_zoom(move |delta| {
        if let Ok(mut bridge) = zoom_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom { delta }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let resize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_resized(move |width, height| {
        let width = width.round().max(1.0) as u32;
        let height = height.round().max(1.0) as u32;
        if let Ok(mut bridge) = resize_bridge.lock() {
            bridge.resize_viewport(width, height);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let viewport_select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_select(move |x, y, extend| {
        if let Ok(mut bridge) = viewport_select_bridge.lock() {
            bridge.select_viewport(x, y, extend);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let transform_begin_bridge = Arc::clone(&bridge);
    window.on_viewport_transform_begin(move |kind_str, x, y| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock() {
            bridge.begin_viewport_transform(kind, x, y);
        }
    });

    let transform_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_transform_update(move |x, y| {
        if let Ok(mut bridge) = transform_drag_bridge.lock() {
            bridge.update_viewport_transform(x, y);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let transform_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_transform_end(move || {
        if let Ok(mut bridge) = transform_end_bridge.lock() {
            bridge.end_viewport_transform();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_begin_bridge = Arc::clone(&bridge);
    window.on_viewport_paint_begin(move |x, y| {
        if let Ok(mut bridge) = paint_begin_bridge.lock() {
            bridge.begin_paint_stroke_at(x, y);
        }
    });

    let paint_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_update(move |x, y| {
        if let Ok(mut bridge) = paint_update_bridge.lock() {
            bridge.paint_stroke_to(x, y);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                window.set_viewport_image(frame);
            }
        }
    });

    let paint_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_end(move || {
        if let Ok(mut bridge) = paint_end_bridge.lock() {
            bridge.end_paint_stroke_at();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let add_menu_bridge = Arc::clone(&bridge);
    window.on_add_menu_changed(move |open| {
        if let Ok(mut bridge) = add_menu_bridge.lock() {
            bridge.add_menu_open = open;
        }
    });

    let tool_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_scrubbed(move |delta, fine| {
        if let Ok(mut bridge) = tool_scrub_bridge.lock() {
            bridge.scrub_tool_modal(delta, fine);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_text_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_text_committed(move |text| {
        if let Ok(mut bridge) = tool_text_bridge.lock() {
            match numeric::parse_numeric(text.as_str()) {
                Ok(value) => {
                    bridge.set_tool_modal_value(value);
                }
                Err(error) => bridge.state.set_status(format!("Invalid value: {error:?}")),
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_apply(move || {
        if let Ok(mut bridge) = tool_apply_bridge.lock() {
            bridge.commit_tool_modal();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_cancel(move || {
        if let Ok(mut bridge) = tool_cancel_bridge.lock() {
            bridge.cancel_tool_modal();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let inspector_width_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_inspector_width_changed(move |width| {
        if let Ok(mut bridge) = inspector_width_bridge.lock()
            && bridge.set_inspector_width(width)
        {
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let asset_height_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_library_height_changed(move |height| {
        if let Ok(mut bridge) = asset_height_bridge.lock()
            && bridge.set_asset_library_height(height)
        {
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let rename_edit_bridge = Arc::clone(&bridge);
    window.on_rename_edited(move |text| {
        if let Ok(mut bridge) = rename_edit_bridge.lock()
            && let Some(draft) = bridge.rename_draft.as_mut()
        {
            *draft = text.to_string();
        }
    });

    let rename_commit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_committed(move |text| {
        if let Ok(mut bridge) = rename_commit_bridge.lock() {
            bridge.commit_rename(text.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let rename_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_rename_cancelled(move || {
        if let Ok(mut bridge) = rename_cancel_bridge.lock() {
            bridge.cancel_rename();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_context_requested(move |id, x, y| {
        if let Ok(mut bridge) = context_bridge.lock() {
            bridge.open_context_menu(id.as_str(), x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let context_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_context_menu_action(move |action| {
        if let Ok(mut bridge) = context_action_bridge.lock() {
            bridge.context_menu_action(action.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_toggled(move |id| {
        if let Ok(mut bridge) = menu_bridge.lock() {
            bridge.toggle_menu(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let menu_item_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_menu_item_invoked(move |id| {
        let id = id.to_string();
        // Diálogos de arquivo continuam no caminho assíncrono já testado.
        let opens_dialog = matches!(
            id.as_str(),
            "file.open" | "file.save" | "file.save_as" | "file.import_obj"
        );
        if let Ok(mut bridge) = menu_item_bridge.lock() {
            bridge.close_menu();
            if opens_dialog {
                let vm = bridge.view_model();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    window.invoke_command_file_action(id.as_str().into());
                }
                return;
            }
            bridge.menu_item_invoked(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_seam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seam_toggled(move || {
        if let Ok(mut bridge) = uv_seam_bridge.lock() {
            bridge.toggle_selected_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let uv_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_seams_cleared(move || {
        if let Ok(mut bridge) = uv_clear_bridge.lock() {
            bridge.clear_all_uv_seams();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    // Painel de camadas do PAINT: cada ação recompoe o raster canônico.
    let paint_layer_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_added(move || {
        if let Ok(mut bridge) = paint_layer_bridge.lock() {
            bridge.add_paint_layer();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_group_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_group_added(move || {
        if let Ok(mut bridge) = paint_group_bridge.lock() {
            bridge.add_paint_group();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_removed(move |id| {
        if let Ok(mut bridge) = paint_remove_bridge.lock() {
            if !bridge.remove_paint_layer(id.as_str()) {
                bridge.state.set_status("The last layer cannot be removed");
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_active_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_activated(move |id| {
        if let Ok(mut bridge) = paint_active_bridge.lock() {
            bridge.set_paint_layer_active(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_visibility_toggled(move |id| {
        if let Ok(mut bridge) = paint_vis_bridge.lock() {
            bridge.toggle_paint_layer_visibility(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_lock_toggled(move |id| {
        if let Ok(mut bridge) = paint_lock_bridge.lock() {
            bridge.toggle_paint_layer_lock(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_moved(move |id, delta| {
        if let Ok(mut bridge) = paint_move_bridge.lock() {
            bridge.move_paint_layer(id.as_str(), delta);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let paint_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_layer_opacity_set(move |id, opacity| {
        if let Ok(mut bridge) = paint_opacity_bridge.lock() {
            bridge.set_paint_layer_opacity(id.as_str(), opacity);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_scrubbed(move |delta| {
        if let Ok(mut bridge) = loop_scrub_bridge.lock() {
            bridge.scrub_loop_cut(delta, false);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_count_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_count_committed(move |text| {
        if let Ok(mut bridge) = loop_count_bridge.lock() {
            match text.trim().parse::<i32>() {
                Ok(cuts) if cuts >= 1 => {
                    bridge.set_loop_cut_count(cuts as usize);
                }
                _ => bridge
                    .state
                    .set_status("Loop Cut: cuts must be a whole number from 1 to 32"),
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_apply(move || {
        if let Ok(mut bridge) = loop_apply_bridge.lock() {
            bridge.commit_loop_cut();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let loop_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_cancel(move || {
        if let Ok(mut bridge) = loop_cancel_bridge.lock() {
            bridge.cancel_loop_cut();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let recover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_recover_requested(move || {
        if let Ok(mut bridge) = recover_bridge.lock() {
            bridge.recover_pending();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let keep_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_keep_requested(move || {
        if let Ok(mut bridge) = keep_bridge.lock() {
            bridge.keep_saved_project();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let discard_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_recovery_discard_requested(move || {
        if let Ok(mut bridge) = discard_bridge.lock() {
            bridge.discard_pending_recovery();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_place_asset_requested(move |id| {
        if let Ok(mut bridge) = place_bridge.lock() {
            bridge.place_asset(id.as_str());
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_select(move |id| {
        if let Ok(mut bridge) = select_bridge.lock() {
            bridge.apply(UiIntent::SelectSceneAsset(id.as_str().to_string()));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let vis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_visibility(move |id| {
        if let Ok(mut bridge) = vis_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetVisibility(
                id.as_str().to_string(),
            ));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_scene_toggle_lock(move |id| {
        if let Ok(mut bridge) = lock_bridge.lock() {
            bridge.apply(UiIntent::ToggleSceneAssetLock(id.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let theme_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_theme_changed(move |theme_id| {
        if let Ok(mut bridge) = theme_bridge.lock() {
            bridge.apply(UiIntent::SetTheme(theme_id.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let undo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_undo_requested(move || {
        if let Ok(mut bridge) = undo_bridge.lock() {
            bridge.apply(UiIntent::Undo);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let redo_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_redo_requested(move || {
        if let Ok(mut bridge) = redo_bridge.lock() {
            bridge.apply(UiIntent::Redo);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let domain_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_domain_changed(move |domain_str| {
        let domain = match domain_str.as_str() {
            "OBJECT" => SelectionDomain::Object,
            "POINT" => SelectionDomain::Vertex,
            "EDGE" => SelectionDomain::Edge,
            "FACE" => SelectionDomain::Face,
            _ => return,
        };
        if let Ok(mut bridge) = domain_bridge.lock() {
            bridge.apply(UiIntent::SetSelectionDomain(domain));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let tool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_active_tool_changed(move |tool| {
        if let Ok(mut bridge) = tool_bridge.lock() {
            bridge.apply(UiIntent::SetActiveTool(tool.as_str().to_string()));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let primitive_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_primitive_requested(move |kind_str| {
        let Some(kind) = petunia_core::PrimitiveKind::parse(kind_str.as_str()).ok() else {
            if let Ok(mut bridge) = primitive_bridge.lock() {
                bridge
                    .state
                    .set_status(format!("Unknown primitive: {kind_str}"));
            }
            return;
        };
        if let Ok(mut bridge) = primitive_bridge.lock() {
            bridge.apply(UiIntent::AddPrimitive(kind));
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let delete_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_delete_requested(move || {
        if let Ok(mut bridge) = delete_bridge.lock() {
            bridge.apply(UiIntent::DeleteActiveAsset);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_color_changed(move |r, g, b| {
        if let Ok(mut bridge) = color_bridge.lock() {
            bridge.apply(UiIntent::SetPaintColor([r, g, b]));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_size_changed(move |size| {
        if let Ok(mut bridge) = size_bridge.lock() {
            bridge.apply(UiIntent::SetBrushSize(size));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_brush_opacity_changed(move |opacity| {
        if let Ok(mut bridge) = opacity_bridge.lock() {
            bridge.apply(UiIntent::SetBrushOpacity(opacity));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dup_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_requested(move || {
        if let Ok(mut bridge) = dup_bridge.lock() {
            bridge.apply(UiIntent::DuplicateActiveAsset);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let asset_lib_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_library_requested(move || {
        if let Ok(mut bridge) = asset_lib_bridge.lock() {
            bridge.apply(UiIntent::ToggleAssetLibrary);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                window.set_asset_library_visible(bridge.asset_library_visible);
                sync_window_properties(&window, &vm);
            }
        }
    });

    let proj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_projection_requested(move || {
        if let Ok(mut bridge) = proj_bridge.lock() {
            bridge.apply(UiIntent::ToggleProjection);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let wire_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_wireframe_requested(move || {
        if let Ok(mut bridge) = wire_bridge.lock() {
            bridge.execute_command(CommandId::ToggleWireframe);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let reset_cam_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_reset_camera_requested(move || {
        if let Ok(mut bridge) = reset_cam_bridge.lock() {
            bridge.apply(UiIntent::ResetCamera);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let save_asset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_save_active_as_asset_requested(move || {
        if let Ok(mut bridge) = save_asset_bridge.lock() {
            bridge.apply(UiIntent::SaveActiveAsAsset);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_model_uses_domain_context_without_ui_dependencies() {
        let state = AppState::default();
        let view_model = ShellViewModel::from_state(&state);
        assert_eq!(view_model.workspace, Workspace::Model);
        assert_eq!(view_model.selection_domain, SelectionDomain::Object);
        assert!(view_model.inspector_visible);
        assert!(view_model.saved);
        assert_eq!(view_model.position, [0.0, 0.0, 0.0]);
        assert_eq!(view_model.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn bridge_routes_workspace_intent_to_core_and_viewport() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        assert_eq!(bridge.state.workspace, Workspace::Paint);
        assert_eq!(bridge.viewport.workspace, Workspace::Paint);

        bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));
        assert_eq!(bridge.state.workspace, Workspace::Uv);
        assert_eq!(bridge.viewport.workspace, Workspace::Uv);
    }

    #[test]
    fn temporary_surfaces_are_owned_by_bridge_not_domain_state() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ToggleSceneDrawer);
        bridge.apply(UiIntent::OpenCommandSearch);
        bridge.apply(UiIntent::OpenSettings);
        assert!(bridge.scene_drawer_visible);
        assert!(bridge.command_search_visible);
        assert!(bridge.settings_visible);
        assert!(!bridge.state.ui.show_command_palette);
    }

    #[test]
    fn overlay_stack_handles_escape_in_lifo_order() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ToggleSceneDrawer);
        bridge.apply(UiIntent::OpenCommandSearch);

        assert!(bridge.scene_drawer_visible);
        assert!(bridge.command_search_visible);

        assert!(bridge.handle_escape());
        assert!(!bridge.command_search_visible);
        assert!(bridge.scene_drawer_visible);

        assert!(bridge.handle_escape());
        assert!(!bridge.scene_drawer_visible);

        assert!(!bridge.handle_escape());
    }

    #[test]
    fn click_away_dismisses_active_modal() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::OpenSettings);
        assert!(bridge.settings_visible);

        assert!(bridge.handle_click_away());
        assert!(!bridge.settings_visible);
    }

    #[test]
    fn command_search_filters_by_active_workspace() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let results = bridge.search_commands("cube");
        assert!(results.iter().any(|item| item.id == "model.add_cube"));
        assert!(
            results
                .iter()
                .all(|item| !item.label.starts_with("commands."))
        );
    }

    #[test]
    fn command_search_uses_current_keymap_shortcuts() {
        let mut state = AppState::default();
        state.ui.keybinds.remove_binding("model.add_cube");
        let bridge = SlintUiBridge::new(state, PlaceholderViewport::default());

        let item = bridge
            .search_commands("Add Cube")
            .into_iter()
            .find(|item| item.id == "model.add_cube")
            .unwrap();

        assert_eq!(item.shortcut, None);
    }

    #[test]
    fn core_palette_command_executes_through_canonical_dispatcher() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let before = bridge.state.project.assets.len();

        bridge.execute_core_command("model.add_cube").unwrap();

        assert_eq!(bridge.state.project.assets.len(), before + 1);
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn transform_scrubbing_updates_values_and_clamps() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        let pos_x = bridge.scrub_transform(TransformKind::Position, 0, 5.0, false);
        assert_eq!(pos_x, 0.5);

        let pos_x_fine = bridge.scrub_transform(TransformKind::Position, 0, 2.0, true);
        assert_eq!(pos_x_fine, 0.52);
        assert_eq!(bridge.view_model().position[0], 0.52);

        assert!(bridge.commit_transform());

        let scale_z = bridge.scrub_transform(TransformKind::Scale, 2, -100.0, false);
        assert_eq!(scale_z, 0.001);

        let vm = bridge.view_model();
        assert_eq!(vm.position[0], 0.0);
        assert_eq!(vm.scale[2], 0.001);
    }

    #[test]
    fn transform_scrub_changes_geometry_and_commits_one_undo_step() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();

        bridge.begin_transform(TransformKind::Position).unwrap();
        bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
        bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
        assert!(bridge.commit_transform());

        let after = &bridge.state.project.active_mesh().unwrap().verts;
        assert!(
            after
                .iter()
                .zip(&before)
                .all(|(after, before)| { (after.pos[0] - before.pos[0] - 2.0).abs() < 1.0e-6 })
        );
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.undo());
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .zip(&before)
                .all(|(restored, before)| restored.pos == before.pos)
        );
    }

    #[test]
    fn escape_cancels_transform_without_closing_the_underlying_overlay() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ToggleSceneDrawer);
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();
        bridge.begin_transform(TransformKind::Position).unwrap();
        bridge.scrub_transform(TransformKind::Position, 1, 10.0, false);

        assert!(bridge.handle_escape());
        assert!(bridge.scene_drawer_visible);
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .zip(&before)
                .all(|(restored, before)| restored.pos == before.pos)
        );
        assert_eq!(bridge.view_model().position, [0.0; 3]);
    }

    #[test]
    fn selecting_another_asset_resets_transform_operation_values() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        bridge.begin_transform(TransformKind::Position).unwrap();
        bridge.scrub_transform(TransformKind::Position, 0, 10.0, false);
        bridge.commit_transform();
        assert_eq!(bridge.view_model().position[0], 1.0);

        let first_id = bridge.state.project.assets[0].id.to_string();
        bridge.apply(UiIntent::SelectSceneAsset(first_id));

        assert_eq!(bridge.view_model().position, [0.0; 3]);
        assert_eq!(bridge.view_model().rotation, [0.0; 3]);
        assert_eq!(bridge.view_model().scale, [1.0; 3]);
    }

    #[test]
    fn bridge_applies_viewport_gestures_to_camera() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let initial_yaw = bridge.state.session.camera.yaw;
        let initial_pitch = bridge.state.session.camera.pitch;
        let initial_target = bridge.state.session.camera.target;
        let initial_height = bridge.state.session.camera.visible_height();

        bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Orbit {
            dx: 10.0,
            dy: -5.0,
        }));
        assert_ne!(bridge.state.session.camera.yaw, initial_yaw);
        assert_ne!(bridge.state.session.camera.pitch, initial_pitch);

        bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan {
            dx: 20.0,
            dy: 10.0,
        }));
        assert_ne!(bridge.state.session.camera.target, initial_target);

        bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom {
            delta: 50.0,
        }));
        assert_ne!(bridge.state.session.camera.visible_height(), initial_height);
    }

    #[test]
    fn viewport_resize_updates_backend_and_camera_aspect() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        bridge.resize_viewport(800, 500);

        assert_eq!(bridge.viewport.width, 800);
        assert_eq!(bridge.viewport.height, 500);
        assert!((bridge.state.session.camera.aspect - 1.6).abs() < f32::EPSILON);
        assert!(!bridge.state.is_document_dirty());
    }

    #[test]
    fn bridge_saves_and_loads_project_file() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let temp_path = temp_dir.path().join("test_project.petunia");

        bridge.state.project.name = "Slint Test Project".into();
        bridge.state.mark_document_dirty();
        assert!(!bridge.view_model().saved);

        bridge.apply(UiIntent::SaveProjectTo(temp_path.clone()));
        let status = bridge.view_model().status_message.clone();
        assert!(temp_path.exists(), "project save failed: {status}");
        assert!(bridge.view_model().saved);

        bridge.state.project.name = "Modified Project".into();
        bridge.apply(UiIntent::OpenProjectFrom(temp_path));
        assert_eq!(bridge.state.project.name, "Slint Test Project");
        assert!(bridge.view_model().saved);
    }

    #[test]
    fn scene_item_selection_updates_active_asset_and_inspector() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.execute_command(CommandId::AddCube);
        assert_eq!(bridge.state.project.assets.len(), 2);

        let second_asset_id = bridge.state.project.assets[1].id.to_string();
        bridge.apply(UiIntent::SelectSceneAsset(second_asset_id.clone()));

        assert_eq!(bridge.state.project.active, 1);
        let vm = bridge.view_model();
        assert!(
            vm.scene_items
                .iter()
                .any(|item| item.id == second_asset_id && item.selected)
        );
        assert_eq!(vm.active_object_title, "Cube");
    }

    #[test]
    fn scene_item_visibility_and_lock_toggles() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let asset_id = bridge.state.project.assets[0].id.to_string();

        assert!(bridge.state.project.assets[0].visible);
        assert!(!bridge.state.project.assets[0].locked);

        bridge.apply(UiIntent::ToggleSceneAssetVisibility(asset_id.clone()));
        assert!(!bridge.state.project.assets[0].visible);
        assert!(bridge.state.is_document_dirty());
        assert!(bridge.state.undo());
        assert!(bridge.state.project.assets[0].visible);

        bridge.apply(UiIntent::ToggleSceneAssetLock(asset_id));
        assert!(bridge.state.project.assets[0].locked);
        assert!(bridge.state.undo());
        assert!(!bridge.state.project.assets[0].locked);

        let vm = bridge.view_model();
        assert!(vm.scene_items[0].visible);
        assert!(!vm.scene_items[0].locked);
    }

    #[test]
    fn theme_change_intent_updates_active_theme() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert_eq!(bridge.state.ui.active_theme_id, "petunia-dark");

        bridge.apply(UiIntent::SetTheme("petunia-high-contrast".into()));
        assert_eq!(bridge.state.ui.active_theme_id, "petunia-high-contrast");
        assert_eq!(bridge.view_model().current_theme, "petunia-high-contrast");
    }

    #[test]
    fn add_cube_command_adds_mesh_to_scene() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let initial_count = bridge.state.project.assets.len();
        bridge.execute_command(CommandId::AddCube);
        assert_eq!(bridge.state.project.assets.len(), initial_count + 1);
        assert_eq!(bridge.view_model().scene_items.len(), initial_count + 1);
    }

    #[test]
    fn selection_domain_intent_updates_state_and_view_model() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert_eq!(
            bridge.view_model().selection_domain,
            SelectionDomain::Object
        );

        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Vertex);
        assert_eq!(
            bridge.view_model().selection_domain,
            SelectionDomain::Vertex
        );
        assert_eq!(bridge.viewport.selection_domain, SelectionDomain::Vertex);

        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Edge);
        assert_eq!(bridge.view_model().selection_domain, SelectionDomain::Edge);

        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);
        assert_eq!(bridge.view_model().selection_domain, SelectionDomain::Face);
    }

    #[test]
    fn primitive_creation_and_deletion_updates_scene() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let base_count = bridge.state.project.assets.len();

        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        assert_eq!(bridge.state.project.assets.len(), base_count + 1);
        assert_eq!(bridge.view_model().active_object_title, "Sphere");

        bridge.apply(UiIntent::AddPrimitive(
            petunia_core::PrimitiveKind::Cylinder,
        ));
        assert_eq!(bridge.state.project.assets.len(), base_count + 2);
        assert_eq!(bridge.view_model().active_object_title, "Cylinder");

        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Plane));
        assert_eq!(bridge.state.project.assets.len(), base_count + 3);
        assert_eq!(bridge.view_model().active_object_title, "Plane");

        // Deleting active asset
        bridge.apply(UiIntent::DeleteActiveAsset);
        assert_eq!(bridge.state.project.assets.len(), base_count + 2);
        assert_eq!(bridge.view_model().active_object_title, "Cylinder");
    }

    #[test]
    fn paint_tool_parameters_update_session_and_view_model() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        bridge.apply(UiIntent::SetPaintColor([0.25, 0.5, 0.75]));
        assert_eq!(bridge.state.session.tools.paint_color, [0.25, 0.5, 0.75]);
        assert_eq!(bridge.view_model().paint_color, [0.25, 0.5, 0.75]);

        bridge.apply(UiIntent::SetBrushSize(2.5));
        assert_eq!(bridge.state.session.tools.paint_radius, 2.5);
        assert_eq!(bridge.view_model().brush_size, 2.5);

        bridge.apply(UiIntent::SetBrushOpacity(0.8));
        assert_eq!(bridge.state.session.tools.paint_strength, 0.8);
        assert_eq!(bridge.view_model().brush_opacity, 0.8);

        bridge.apply(UiIntent::SetActiveTool("eraser".into()));
        assert_eq!(bridge.state.session.tools.active_tool, "eraser");
        assert_eq!(bridge.view_model().active_tool, "eraser");
    }

    #[test]
    fn command_routing_covers_selection_paint_and_uv() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        bridge.execute_command(CommandId::SelectModePoint);
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Vertex);

        bridge.execute_command(CommandId::SelectModeEdge);
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Edge);

        bridge.execute_command(CommandId::SelectModeFace);
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Face);

        bridge.execute_command(CommandId::SelectModeObject);
        assert_eq!(bridge.state.selection_domain(), SelectionDomain::Object);

        bridge.execute_command(CommandId::PaintBrush);
        assert_eq!(bridge.state.session.tools.active_tool, "brush");

        bridge.execute_command(CommandId::PaintEraser);
        assert_eq!(bridge.state.session.tools.active_tool, "eraser");

        bridge.execute_command(CommandId::PaintFill);
        assert_eq!(bridge.state.session.tools.active_tool, "fill");

        bridge.execute_command(CommandId::UvUnwrap);
        assert!(bridge.view_model().status_message.contains("Auto UV"));

        bridge.execute_command(CommandId::UvPackIslands);
        assert!(bridge.view_model().status_message.contains("Packed"));

        bridge.execute_command(CommandId::DeleteSelected);
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert!(bridge.state.is_document_dirty());
    }

    #[test]
    fn undo_redo_intents_integrate_with_project_undo_stack() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        // Initial state
        assert!(!bridge.view_model().can_undo);
        assert!(!bridge.view_model().can_redo);

        // Add primitive command automatically creates an undo checkpoint in core AppState
        bridge.execute_command(CommandId::AddCube);
        assert_eq!(bridge.state.project.assets.len(), 2);
        assert!(bridge.view_model().can_undo);

        // Apply Undo
        bridge.apply(UiIntent::Undo);
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert!(bridge.view_model().can_redo);

        // Apply Redo
        bridge.apply(UiIntent::Redo);
        assert_eq!(bridge.state.project.assets.len(), 2);
        assert!(bridge.view_model().can_undo);
    }

    #[test]
    fn duplicate_active_asset_intent_duplicates_and_creates_undo() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let base_count = bridge.state.project.assets.len();
        assert_eq!(base_count, 1);
        assert!(!bridge.view_model().can_undo);

        bridge.apply(UiIntent::DuplicateActiveAsset);
        assert_eq!(bridge.state.project.assets.len(), 2);
        assert!(bridge.view_model().can_undo);
        assert!(bridge.view_model().status_message.contains("duplicado"));

        let first_id = bridge.state.project.assets[0].id;
        let second_id = bridge.state.project.assets[1].id;
        assert_ne!(first_id, second_id);

        bridge.apply(UiIntent::Undo);
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert!(bridge.view_model().can_redo);

        bridge.apply(UiIntent::Redo);
        assert_eq!(bridge.state.project.assets.len(), 2);
        assert!(bridge.view_model().can_undo);
    }

    #[test]
    fn selection_commands_select_all_clear_and_invert() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        bridge.apply(UiIntent::SelectAll);
        let mesh = bridge.state.project.active_mesh().expect("active mesh");
        assert!(mesh.verts.iter().all(|v| v.selected));

        bridge.apply(UiIntent::ClearSelection);
        let mesh = bridge.state.project.active_mesh().expect("active mesh");
        assert!(mesh.verts.iter().all(|v| !v.selected));

        bridge.apply(UiIntent::InvertSelection);
        let mesh = bridge.state.project.active_mesh().expect("active mesh");
        assert!(mesh.verts.iter().all(|v| v.selected));
    }

    #[test]
    fn toggle_asset_library_and_overlays() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().asset_library_visible);

        bridge.apply(UiIntent::ToggleAssetLibrary);
        assert!(bridge.asset_library_visible);
        assert!(bridge.view_model().asset_library_visible);

        assert!(bridge.handle_escape());
        assert!(!bridge.asset_library_visible);
        assert!(!bridge.view_model().asset_library_visible);

        bridge.apply(UiIntent::ToggleAssetLibrary);
        assert!(bridge.asset_library_visible);
        assert!(bridge.handle_click_away());
        assert!(!bridge.asset_library_visible);
    }

    #[test]
    fn closing_one_drawer_does_not_close_or_leave_a_ghost_for_another() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ToggleSceneDrawer);
        bridge.apply(UiIntent::ToggleAssetLibrary);
        bridge.apply(UiIntent::ToggleSceneDrawer);

        assert!(!bridge.scene_drawer_visible);
        assert!(bridge.asset_library_visible);
        assert_eq!(bridge.overlays.len(), 1);
        assert!(bridge.handle_escape());
        assert!(!bridge.asset_library_visible);
        assert!(!bridge.handle_escape());
    }

    #[test]
    fn modal_identity_preserves_lifo_when_multiple_modals_are_open() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::OpenCommandSearch);
        bridge.apply(UiIntent::OpenSettings);

        assert!(bridge.handle_escape());
        assert!(!bridge.settings_visible);
        assert!(bridge.command_search_visible);
        assert!(bridge.handle_escape());
        assert!(!bridge.command_search_visible);
    }

    #[test]
    fn delete_is_dirty_and_undo_restores_the_asset() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        bridge.state.mark_document_clean();
        let before = bridge.state.project.assets.len();

        bridge.apply(UiIntent::DeleteActiveAsset);

        assert_eq!(bridge.state.project.assets.len(), before - 1);
        assert!(bridge.state.is_document_dirty());
        assert!(bridge.state.undo());
        assert_eq!(bridge.state.project.assets.len(), before);
    }

    #[test]
    fn viewport_drag_transform_moves_geometry_and_commits_one_undo_step() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();

        assert!(bridge.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
        assert!(bridge.update_viewport_transform(460.0, 300.0));
        assert!(bridge.end_viewport_transform());

        let after = &bridge.state.project.active_mesh().unwrap().verts;
        assert!(
            after
                .iter()
                .zip(&before)
                .all(|(after, before)| (after.pos[0] - before.pos[0]).abs() > 1.0e-4)
        );
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.undo());
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .zip(&before)
                .all(|(restored, before)| restored.pos == before.pos)
        );
        assert!(bridge.drag.is_none());
    }

    #[test]
    fn escape_cancels_viewport_drag_without_touching_history() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();

        assert!(bridge.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
        assert!(bridge.update_viewport_transform(460.0, 300.0));
        assert!(bridge.handle_escape());

        assert!(bridge.drag.is_none());
        assert!(!bridge.state.project.undo.can_undo());
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .zip(&before)
                .all(|(restored, before)| restored.pos == before.pos)
        );
    }

    #[test]
    fn edge_domain_selects_a_real_edge() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));

        // Aponta para o centro do cubo padrão, onde uma aresta é atingível.
        bridge.select_viewport(0.5, 0.5, false);

        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(
            mesh.selected_edges.iter().any(|&(a, b)| {
                mesh.verts.get(a as usize).is_some_and(|v| v.selected)
                    || mesh.verts.get(b as usize).is_some_and(|v| v.selected)
            }),
            "edge selection must mark the picked edge or its endpoints"
        );
    }

    #[test]
    fn locked_asset_is_not_picked_in_the_viewport() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::ToggleSceneAssetLock(
            bridge.state.project.assets[0].id.to_string(),
        ));

        let cursor_before = bridge.state.session.cursor_3d;
        bridge.select_viewport(0.5, 0.5, false);

        assert_eq!(bridge.state.session.cursor_3d, cursor_before);
        assert!(bridge.state.project.assets[0].locked);
    }

    #[test]
    fn gizmo_projects_axes_for_the_active_object() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let gizmo = bridge.view_model().gizmo;

        assert!(gizmo.visible);
        assert!((gizmo.origin_x - 512.0).abs() < 64.0);
        assert!((gizmo.origin_y - 384.0).abs() < 64.0);
        let x_length = ((gizmo.x_end_x - gizmo.origin_x).powi(2)
            + (gizmo.x_end_y - gizmo.origin_y).powi(2))
        .sqrt();
        assert!(x_length > 4.0, "gizmo X axis must have visible length");
    }

    #[test]
    fn gizmo_is_hidden_outside_the_model_workspace() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

        assert!(!bridge.view_model().gizmo.visible);
    }

    #[test]
    fn delete_in_component_mode_removes_geometry_not_the_object() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
        let asset_count = bridge.state.project.assets.len();
        let verts_before = bridge.state.project.active_mesh().unwrap().verts.len();

        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        bridge.apply(UiIntent::DeleteActiveAsset);

        assert_eq!(bridge.state.project.assets.len(), asset_count);
        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(mesh.verts.len() <= verts_before);
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn paint_stroke_is_continuous_and_commits_one_undo_step() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("brush".into()));

        assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
        assert!(bridge.paint_stroke_to(420.0, 300.0));
        assert!(bridge.paint_stroke_to(440.0, 310.0));
        assert!(bridge.end_paint_stroke_at());

        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.is_document_dirty());
        assert!(bridge.paint_last.is_none());
    }

    #[test]
    fn escape_cancels_paint_stroke_without_history() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("brush".into()));

        assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
        assert!(bridge.paint_stroke_to(430.0, 300.0));
        assert!(bridge.handle_escape());

        assert!(bridge.paint_last.is_none());
        assert!(!bridge.state.project.undo.can_undo());
    }

    #[test]
    fn keymap_routes_the_model_shortcut_table() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        assert!(bridge.route_shortcut("E", false, false, false));
        assert!(bridge.route_shortcut("I", false, false, false));
        assert!(bridge.route_shortcut("B", true, false, false));
        assert!(bridge.route_shortcut("W", false, false, false));
        assert!(bridge.route_shortcut("M", false, false, false));
        assert!(bridge.route_shortcut("K", false, false, false));
        assert!(bridge.route_shortcut("Z", true, false, false));
        assert!(bridge.route_shortcut("1", false, false, false));
        assert!(!bridge.route_shortcut("Ω", false, false, false));
    }

    #[test]
    fn push_pull_and_knife_commands_are_registered_and_contextual() {
        let mut state = AppState::default();
        assert!(state.commands.contains("model.push_pull"));
        assert!(state.commands.contains("model.knife"));

        // Push/Pull exige Edit mode e face selecionada.
        assert!(state.dispatch_command("model.push_pull").is_err());

        state.set_edit_mode(petunia_core::EditMode::Edit);
        state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        assert!(state.dispatch_command("model.push_pull").is_ok());
        assert!(state.session.tools.modal.is_some());
        state.cancel_modal();

        assert!(state.dispatch_command("model.knife").is_ok());
        assert!(state.session.tools.cut_session.is_some());
    }

    #[test]
    fn extrude_tool_modal_previews_with_drag_and_commits_one_undo() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();

        assert!(bridge.begin_tool_modal(ToolModalKind::Extrude));
        assert!(bridge.view_model().tool_modal_active);
        assert!(bridge.scrub_tool_modal(-40.0, false));
        assert!(bridge.tool_modal_value > 0.0);
        assert!(bridge.commit_tool_modal());

        assert!(bridge.state.project.undo.can_undo());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(!bridge.view_model().tool_modal_active);
    }

    #[test]
    fn tool_modal_cancel_restores_geometry_and_keeps_history_clean() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        let before = bridge.state.project.project.clone();

        assert!(bridge.begin_tool_modal(ToolModalKind::Inset));
        assert!(bridge.scrub_tool_modal(-30.0, false));
        assert!(bridge.cancel_tool_modal());

        assert!(!bridge.state.project.undo.can_undo());
        let mesh = bridge.state.project.active_mesh().unwrap();
        let original = before.active_mesh().unwrap();
        assert_eq!(mesh.verts.len(), original.verts.len());
        assert_eq!(mesh.faces.len(), original.faces.len());
    }

    #[test]
    fn executing_extrude_command_opens_the_tool_modal_not_a_fixed_preview() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();

        bridge.execute_core_command("model.extrude").unwrap();

        assert!(bridge.tool_modal.is_some());
        assert!(bridge.view_model().tool_modal_active);
        assert!(bridge.handle_escape());
        assert!(bridge.tool_modal.is_none());
    }

    #[test]
    fn extrude_individual_builds_topology_and_moves_each_face_along_its_own_normal() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        let before = bridge.state.project.active_mesh().unwrap().clone();

        bridge
            .execute_core_command("model.extrude_individual")
            .unwrap();
        assert!(bridge.scrub_tool_modal(-60.0, false));

        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(
            mesh.verts.len() > before.verts.len(),
            "extrude individual deve criar vértices"
        );
        assert!(
            mesh.verts.iter().any(|v| {
                !before.verts.iter().any(|b| {
                    (v.pos[0] - b.pos[0]).abs() < 1.0e-4
                        && (v.pos[1] - b.pos[1]).abs() < 1.0e-4
                        && (v.pos[2] - b.pos[2]).abs() < 1.0e-4
                })
            }),
            "a face extrudada precisa sair da posição original"
        );

        assert!(bridge.commit_tool_modal());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn scale_selection_modal_rejects_identity_and_commits_a_real_factor() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();

        bridge
            .execute_core_command("model.scale_selection")
            .unwrap();
        assert_eq!(bridge.tool_modal_value, 1.0);
        assert!(bridge.set_tool_modal_value(2.0));
        let scaled = bridge.state.project.active_mesh().unwrap().verts.clone();
        assert!(
            scaled
                .iter()
                .zip(&before)
                .any(|(a, b)| (a.pos[0] - b.pos[0]).abs() > 1.0e-4),
            "factor 2.0 deve deslocar vértices"
        );
        assert!(bridge.commit_tool_modal());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    }

    #[test]
    fn uv_statistics_come_from_the_real_diagnostics_not_a_hardcoded_claim() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let stats = bridge.view_model().uv_stats;
        assert!(stats.contains("xatlas-rs-v2"));
        assert!(stats.contains("Islands:"));
        assert!(!stats.contains("LSCM"));
        assert!(!stats.contains("Texel Density: Auto"));

        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        bridge.execute_core_command("uv.unwrap").unwrap();
        let stats = bridge.view_model().uv_stats;
        assert!(stats.contains("Islands: 1"), "unwrap real: {stats}");
    }

    #[test]
    fn inspector_splitter_clamps_and_never_dirties_the_document() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let saved = bridge.view_model().saved;

        assert!(bridge.set_inspector_width(384.0));
        assert_eq!(bridge.view_model().inspector_width, 384.0);
        assert_eq!(bridge.view_model().saved, saved);

        assert!(bridge.set_inspector_width(1.0));
        assert_eq!(
            bridge.view_model().inspector_width,
            petunia_core::PROPERTIES_MIN_WIDTH
        );
        assert!(bridge.set_inspector_width(9_999.0));
        assert_eq!(
            bridge.view_model().inspector_width,
            petunia_core::PROPERTIES_MAX_WIDTH
        );
        assert!(
            !bridge.set_inspector_width(f32::NAN),
            "NaN não pode alterar o layout"
        );
    }

    #[test]
    fn asset_library_splitter_clamps_its_height() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.set_asset_library_height(320.0));
        assert_eq!(bridge.view_model().asset_library_height, 320.0);

        assert!(bridge.set_asset_library_height(0.0));
        assert_eq!(
            bridge.view_model().asset_library_height,
            petunia_core::SHELL_ASSET_LIBRARY_MIN_HEIGHT
        );
        assert!(bridge.set_asset_library_height(5_000.0));
        assert_eq!(
            bridge.view_model().asset_library_height,
            petunia_core::SHELL_ASSET_LIBRARY_MAX_HEIGHT
        );
    }

    #[test]
    fn switching_workspace_remembers_the_resized_inspector_and_library() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.set_inspector_width(420.0));
        assert!(bridge.set_asset_library_height(340.0));

        bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Paint));
        assert_ne!(
            bridge.view_model().inspector_width,
            420.0,
            "Paint deve restaurar a própria largura, não herdar a de Model"
        );
        assert_ne!(
            bridge.view_model().asset_library_height,
            340.0,
            "a Asset Library também tem memória por workspace"
        );

        bridge.apply(UiIntent::SetWorkspace(petunia_core::Workspace::Model));
        assert_eq!(bridge.view_model().inspector_width, 420.0);
        assert_eq!(bridge.view_model().asset_library_height, 340.0);
    }

    #[test]
    fn rename_session_commits_one_undo_entry_and_trims_the_name() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let original = bridge.state.project.active().unwrap().name.clone();
        assert_eq!(original, "Cube");

        assert!(bridge.begin_rename());
        assert!(bridge.view_model().rename_active);
        assert_eq!(bridge.view_model().rename_value, original);

        assert!(bridge.commit_rename("  Turret Base  "));
        assert!(!bridge.view_model().rename_active);
        assert_eq!(bridge.state.project.active().unwrap().name, "Turret Base");
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));

        assert!(bridge.state.undo());
        assert_eq!(bridge.state.project.active().unwrap().name, original);
    }

    #[test]
    fn rename_rejects_empty_and_overlong_names_without_touching_history() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.begin_rename();
        assert!(!bridge.commit_rename("   "));
        assert_eq!(
            bridge.state.ui.status,
            petunia_core::AssetRenameError::EmptyName.to_string()
        );
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert_eq!(bridge.state.project.active().unwrap().name, "Cube");

        bridge.begin_rename();
        let long = "x".repeat(petunia_core::ASSET_NAME_MAX_LEN + 1);
        assert!(!bridge.commit_rename(&long));
        assert_eq!(
            bridge.state.ui.status,
            petunia_core::AssetRenameError::NameTooLong.to_string()
        );
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn confirming_an_unchanged_name_does_not_push_history() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.begin_rename();
        assert!(bridge.commit_rename("Cube"));
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn escape_abandons_a_rename_draft_before_anything_else() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.begin_rename();
        bridge.rename_draft = Some("Discarded".to_string());

        assert!(bridge.handle_escape());
        assert!(!bridge.view_model().rename_active);
        assert_eq!(bridge.state.project.active().unwrap().name, "Cube");
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn f2_resolves_to_rename_in_the_canonical_keymap() {
        let keybinds = petunia_config::keybinds::Keybinds::load_profile("petunia-default");
        let f2 = input::key_code_from_slint("F2").expect("F2 precisa ser mapeável");
        assert_eq!(
            keybinds.find(f2, Default::default()),
            Some("global.rename"),
            "o keymap canônico precisa entregar global.rename para F2"
        );

        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.route_shortcut("F2", false, false, false));
        assert!(bridge.view_model().rename_active);
    }

    /// O perfil de notebook usa F2 para seleção de aresta (não tem numpad), então
    /// o arquivo precisa deslocar `rename` para Ctrl+F2 — senão as duas ações
    /// disputariam a mesma tecla.
    ///
    /// O teste lê o TOML por caminho absoluto de propósito: `Keybinds::load_profile`
    /// resolve `assets/keymaps/` relativo ao diretório de trabalho, e o CWD dos
    /// testes é o diretório da crate.
    #[test]
    fn notebook_profile_moves_rename_off_the_edge_selection_key() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/keymaps/petunia-notebook.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("falha ao ler {}: {e}", path.display()));
        let mut section = String::new();
        let mut bindings = std::collections::HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                section = line.trim_matches(['[', ']']).to_string();
            } else if let Some((key, value)) = line.split_once('=')
                && !key.trim_start().starts_with('#')
            {
                let value = value.trim().trim_matches('"').to_string();
                bindings.insert(format!("{section}.{}", key.trim()), value);
            }
        }

        assert_eq!(
            bindings.get("model.select_edge").map(String::as_str),
            Some("F2"),
            "F2 continua sendo seleção de aresta neste perfil"
        );
        assert_eq!(
            bindings.get("global.rename").map(String::as_str),
            Some("Ctrl+F2"),
            "o perfil precisa deslocar rename para não colidir com select_edge"
        );

        let canonical = petunia_config::keybinds::Keybinds::defaults();
        let f2 = input::key_code_from_slint("F2").expect("F2 precisa ser mapeável");
        assert_eq!(
            canonical.find(f2, Default::default()),
            Some("global.rename"),
            "a lista canônica entrega F2 para rename"
        );
    }

    /// Duas ações do mesmo namespace não podem dividir o mesmo atalho.
    ///
    /// Sobreposição entre namespaces diferentes é uma categoria à parte
    /// (`ConflictKind::ContextOverlap`, onde `global` sombreia o resto) e é
    /// detectada por `Keybinds::detect_conflicts`, não por este teste.
    #[test]
    fn canonical_keymap_has_no_duplicate_binding_inside_a_namespace() {
        let canonical = petunia_config::keybinds::Keybinds::defaults();
        let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for (action, shortcut) in canonical.all_bindings() {
            let namespace = action.split('.').next().unwrap_or_default();
            let key = format!("{namespace}:{shortcut}");
            if let Some(previous) = seen.insert(key, action.clone()) {
                panic!(
                    "{shortcut} está mapeado para {previous} e para {action} no mesmo namespace"
                );
            }
        }
    }

    #[test]
    fn context_menu_targets_the_clicked_asset_not_the_previous_active_one() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        assert_eq!(bridge.state.project.assets.len(), 2);
        let first = bridge.state.project.assets[0].id;
        let second = bridge.state.project.assets[1].id;
        assert_eq!(
            bridge.state.project.active, 1,
            "a esfera acabou de ser criada"
        );

        assert!(bridge.open_context_menu(&first.to_string(), 120.0, 60.0));
        assert_eq!(
            bridge.state.project.active, 0,
            "o alvo do menu vira o ativo"
        );
        assert!(bridge.view_model().context_menu_open);
        assert_eq!(bridge.view_model().context_menu_x, 120.0);

        assert!(bridge.context_menu_action("delete"));
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert!(!bridge.state.project.assets.iter().any(|a| a.id == first));
        assert!(bridge.state.project.assets.iter().any(|a| a.id == second));
        assert!(!bridge.view_model().context_menu_open);
    }

    #[test]
    fn context_menu_visibility_and_lock_act_on_the_target() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let id = bridge.state.project.assets[0].id.to_string();

        bridge.open_context_menu(&id, 10.0, 10.0);
        assert!(bridge.view_model().context_menu_visible);
        assert!(bridge.context_menu_action("visibility"));

        bridge.open_context_menu(&id, 10.0, 10.0);
        assert!(!bridge.view_model().context_menu_visible, "Hide inverteu");
        assert!(!bridge.state.project.assets[0].visible);

        bridge.open_context_menu(&id, 10.0, 10.0);
        assert!(bridge.context_menu_action("lock"));
        assert!(bridge.state.project.assets[0].locked);
        bridge.open_context_menu(&id, 10.0, 10.0);
        assert!(bridge.view_model().context_menu_locked);
    }

    #[test]
    fn escape_closes_the_context_menu_before_anything_else() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let id = bridge.state.project.assets[0].id.to_string();
        bridge.open_context_menu(&id, 10.0, 10.0);
        bridge.begin_rename();

        assert!(bridge.handle_escape(), "fecha o menu primeiro");
        assert!(!bridge.view_model().context_menu_open);
        assert!(
            bridge.view_model().rename_active,
            "o rename continua aberto: o menu era o topo da pilha"
        );
        assert!(bridge.handle_escape());
        assert!(!bridge.view_model().rename_active);
    }

    #[test]
    fn context_menu_refuses_an_unknown_asset() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.open_context_menu(&uuid::Uuid::new_v4().to_string(), 0.0, 0.0));
        assert!(!bridge.open_context_menu("not-a-uuid", 0.0, 0.0));
        assert!(!bridge.view_model().context_menu_open);
    }

    #[test]
    fn menu_bar_labels_come_from_the_i18n_catalog() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let vm = bridge.view_model();
        assert_eq!(vm.menu_file_label, "File");
        assert_eq!(vm.menu_edit_label, "Edit");
        assert_eq!(vm.menu_view_label, "View");
        assert_eq!(vm.menu_window_label, "Window");

        bridge.state.ui.i18n = petunia_config::I18n::load("pt-BR");
        let vm = bridge.view_model();
        assert_eq!(vm.menu_file_label, "Arquivo");
        assert_eq!(vm.menu_edit_label, "Editar");
        assert_eq!(vm.menu_view_label, "Exibir");
        assert_eq!(vm.menu_window_label, "Janela");
    }

    #[test]
    fn every_menu_item_publishes_a_real_translated_label_and_command_id() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let vm = bridge.view_model();
        let menus = [
            ("file", &vm.menu_file_items),
            ("edit", &vm.menu_edit_items),
            ("view", &vm.menu_view_items),
            ("window", &vm.menu_window_items),
        ];
        for (menu, items) in menus {
            assert!(!items.is_empty(), "menu {menu} sem itens");
            for item in items {
                assert!(!item.label.is_empty(), "{} tem rótulo vazio", item.id);
                assert!(
                    !item.label.contains('.'),
                    "{} publicou a chave de i18n em vez do texto: {}",
                    item.id,
                    item.label
                );
            }
        }
    }

    #[test]
    fn menu_toggles_open_and_close_and_escape_closes_it_first() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.toggle_menu("file"));
        assert_eq!(bridge.view_model().menu_open, "file");

        assert!(bridge.toggle_menu("view"));
        assert_eq!(bridge.view_model().menu_open, "view", "troca de menu");

        assert!(!bridge.toggle_menu("view"), "clicar de novo fecha");
        assert_eq!(bridge.view_model().menu_open, "");

        bridge.toggle_menu("edit");
        bridge.begin_rename();
        assert!(bridge.handle_escape(), "o menu é o topo da pilha");
        assert_eq!(bridge.view_model().menu_open, "");
        assert!(bridge.view_model().rename_active);
    }

    #[test]
    fn menu_items_dispatch_to_the_domain() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let before = bridge.state.project.assets.len();

        assert!(bridge.menu_item_invoked("edit.duplicate"));
        assert_eq!(bridge.state.project.assets.len(), before + 1);
        assert_eq!(bridge.view_model().menu_open, "");

        assert!(bridge.menu_item_invoked("view.toggle_wireframe"));
        assert_eq!(bridge.state.shading, petunia_core::Shading::Wireframe);
        assert!(bridge.menu_item_invoked("view.toggle_wireframe"));
        assert_ne!(bridge.state.shading, petunia_core::Shading::Wireframe);

        assert!(bridge.menu_item_invoked("view.reset_camera"));
        assert!(!bridge.menu_item_invoked("view.not_a_real_command"));
    }

    #[test]
    fn shell_chrome_labels_are_translated_and_the_theme_list_comes_from_the_registry() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let vm = bridge.view_model();
        assert_eq!(vm.label_parts, "Parts");
        assert_eq!(vm.label_apply, "Apply");
        assert_eq!(vm.label_cancel, "Cancel");
        assert_eq!(vm.label_delete, "Delete");
        assert_eq!(vm.label_asset_library, "Asset Library");
        assert_eq!(vm.label_preferences, "Preferences");
        assert_eq!(vm.label_theme, "Theme");

        assert!(
            vm.themes.len() >= 2,
            "o registry precisa publicar os temas oficiais, veio {:?}",
            vm.themes
        );
        assert!(vm.themes.iter().any(|theme| theme.id == "petunia-dark"));
        assert!(
            vm.themes
                .iter()
                .any(|theme| theme.id == "petunia-high-contrast")
        );
        assert_eq!(
            vm.themes.iter().filter(|theme| theme.active).count(),
            1,
            "exatamente um tema ativo"
        );
        assert!(
            vm.themes
                .iter()
                .find(|theme| theme.active)
                .is_some_and(|theme| theme.id == "petunia-dark")
        );

        bridge.state.ui.i18n = petunia_config::I18n::load("pt-BR");
        let vm = bridge.view_model();
        assert_eq!(vm.label_parts, "Peças");
        assert_eq!(vm.label_apply, "Aplicar");
        assert_eq!(vm.label_delete, "Apagar");
        assert_eq!(vm.label_asset_library, "Assets");
        assert_eq!(vm.label_theme, "Tema");
    }

    #[test]
    fn the_preferences_footer_reports_the_real_keymap_and_theme() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let info = bridge.view_model().shell_info;
        assert!(info.contains("petunia-dark"), "veio: {info}");
        assert!(info.contains("petunia-default"), "veio: {info}");
        assert!(
            !info.contains("Keymap: Standard"),
            "a linha antiga afirmava um keymap fixo que não era o real: {info}"
        );
    }

    #[test]
    fn placing_a_library_asset_instantiates_a_copy_at_the_cursor() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let source = bridge.state.project.assets[0].id;
        let before = bridge.state.project.assets.len();
        bridge.state.session.cursor_3d = [4.0, 1.0, -2.0];

        assert!(bridge.place_asset(&source.to_string()));
        assert_eq!(bridge.state.project.assets.len(), before + 1);
        let placed = bridge.state.project.assets.last().unwrap();
        assert_ne!(placed.id, source, "a cópia precisa de identidade própria");
        let center = placed.mesh.selection_center();
        assert!((center[0] - 4.0).abs() < 1.0e-4, "veio {center:?}");
        assert!((center[2] - (-2.0)).abs() < 1.0e-4, "veio {center:?}");

        assert!(bridge.state.project.undo.can_undo());
        assert!(bridge.state.undo());
        assert_eq!(bridge.state.project.assets.len(), before);
    }

    #[test]
    fn placing_an_unknown_asset_is_refused_and_says_so() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let before = bridge.state.project.assets.len();
        assert!(!bridge.place_asset(&uuid::Uuid::new_v4().to_string()));
        assert!(!bridge.place_asset("not-a-uuid"));
        assert_eq!(bridge.state.project.assets.len(), before);
        assert_eq!(bridge.state.ui.status, "Asset not found in project library");
    }

    #[test]
    fn autosave_respects_interval_and_dirty_state() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.autosave = petunia_core::AutosaveService::new(petunia_core::AutosaveConfig {
            enabled: true,
            interval_secs: 0,
            keep_n: 2,
            only_when_dirty: true,
        });

        assert!(
            !bridge.autosave_tick(),
            "documento limpo não deve gerar snapshot"
        );

        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.checkpoint("dirty");
        assert!(
            bridge.autosave_tick(),
            "documento sujo dentro do intervalo precisa gerar snapshot"
        );
        assert!(
            bridge.state.is_document_dirty(),
            "autosave nunca limpa o dirty state"
        );
    }

    #[test]
    fn recovery_prompt_only_appears_when_a_snapshot_was_detected() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().recovery_open);
        assert!(!bridge.recover_pending());
        assert!(!bridge.keep_saved_project());
        assert!(!bridge.discard_pending_recovery());

        bridge.pending_recovery = Some(petunia_core::RecoveryInfo {
            snapshot_path: std::path::PathBuf::from("/tmp/nao-existe/autosave-1.petunia"),
            project_name: "Turret".to_string(),
            snapshot_time: 1_700_000_000,
            main_project_path: None,
            is_newer_than_main: true,
        });
        let vm = bridge.view_model();
        assert!(vm.recovery_open);
        assert!(
            vm.recovery_title.contains("Recover"),
            "veio: {}",
            vm.recovery_title
        );
        assert!(
            vm.recovery_detail.contains("Turret"),
            "veio: {}",
            vm.recovery_detail
        );
        assert!(!vm.recovery_discard.is_empty());

        assert!(bridge.keep_saved_project(), "abrir o salvo fecha o aviso");
        assert!(!bridge.view_model().recovery_open);
        assert!(!bridge.state.project.assets[0].name.is_empty());
    }

    #[test]
    fn knife_takes_two_viewport_picks_and_commits_one_cut() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.execute_core_command("model.knife").unwrap();
        assert!(bridge.state.session.tools.cut_session.is_some());
        let faces_before = bridge.state.project.active_mesh().unwrap().faces.len();

        // O cubo padrão preenche o centro da viewport: dois cliques sobre
        // arestas reais aplicam o corte.
        assert!(
            bridge.knife_click(0.5, 0.5),
            "primeiro ponto precisa ancorar"
        );
        assert_eq!(bridge.state.ui.status, "Knife: pick the second edge point");
        assert_eq!(
            bridge.state.project.undo.depth(),
            (0, 0),
            "ancorar não corta"
        );
        assert!(
            bridge.knife_click(0.35, 0.62),
            "segundo ponto precisa cortar"
        );

        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(
            mesh.faces.len() > faces_before,
            "o corte precisa criar faces: antes {faces_before}, depois {}",
            mesh.faces.len()
        );
        assert!(bridge.state.session.tools.cut_session.is_none());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn knife_click_outside_a_session_does_nothing() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.knife_click(0.5, 0.5));
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn escape_cancels_the_knife_and_restores_the_select_tool() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.execute_core_command("model.knife").unwrap();
        assert_eq!(bridge.state.session.tools.active_tool, "cut");

        assert!(bridge.handle_escape());
        assert!(bridge.state.session.tools.cut_session.is_none());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert_eq!(bridge.state.ui.status, "Knife cancelled");
    }

    #[test]
    fn loop_cut_session_slides_previews_and_commits_one_undo_entry() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        let original = bridge.state.project.active_mesh().unwrap().clone();
        // Uma aresta do cubo padrão está num anel de quads.
        let seed = {
            let mesh = bridge.state.project.active_mesh_mut().unwrap();
            let face = mesh.faces[0].verts.clone();
            let edge = (face[0], face[1]);
            mesh.selected_edges.insert(edge);
            edge
        };
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .selected_edges
                .contains(&seed)
        );

        assert!(bridge.begin_loop_cut(), "o anel precisa ser descoberto");
        assert!(bridge.loop_cut.is_some());
        let preview = bridge.state.project.active_mesh().unwrap().clone();
        assert!(
            preview.verts.len() > original.verts.len(),
            "o preview precisa inserir vértices: {} -> {}",
            original.verts.len(),
            preview.verts.len()
        );

        assert!(
            bridge.scrub_loop_cut(60.0, false),
            "slide precisa reconstruir"
        );
        assert!(bridge.loop_cut.as_ref().unwrap().slide > 0.0);

        assert!(bridge.set_loop_cut_count(3));
        assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 3);
        assert!(
            bridge.state.project.active_mesh().unwrap().verts.len()
                > bridge.state.project.undo.depth().0
        );

        assert!(bridge.commit_loop_cut());
        assert!(bridge.loop_cut.is_none());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.project.undo.can_undo());

        assert!(bridge.state.undo());
        let restored = bridge.state.project.active_mesh().unwrap();
        assert_eq!(restored.verts.len(), original.verts.len());
        assert_eq!(restored.faces.len(), original.faces.len());
    }

    #[test]
    fn loop_cut_cancel_restores_the_exact_original_mesh() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        let original = bridge.state.project.active_mesh().unwrap().clone();
        {
            let mesh = bridge.state.project.active_mesh_mut().unwrap();
            let face = mesh.faces[0].verts.clone();
            mesh.selected_edges.insert((face[0], face[1]));
        }

        assert!(bridge.begin_loop_cut());
        assert!(bridge.scrub_loop_cut(-80.0, false));
        assert!(bridge.cancel_loop_cut());

        let restored = bridge.state.project.active_mesh().unwrap();
        assert_eq!(restored.verts.len(), original.verts.len());
        assert_eq!(restored.faces.len(), original.faces.len());
        assert_eq!(
            bridge.state.project.undo.depth(),
            (0, 0),
            "cancelar não pode empilhar histórico"
        );
    }

    #[test]
    fn loop_cut_refuses_without_a_selected_edge_and_says_why() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.begin_loop_cut());
        assert!(bridge.loop_cut.is_none());
        assert!(
            bridge.state.ui.status.contains("select an edge"),
            "veio: {}",
            bridge.state.ui.status
        );
    }

    #[test]
    fn paint_layer_panel_adds_removes_reorders_and_composites() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));

        assert!(bridge.add_paint_layer());
        let layers = bridge.view_model().paint_layers;
        assert_eq!(layers.len(), 2, "base + nova camada");
        assert!(layers[1].active, "a camada nova vira ativa");
        assert_eq!(layers[1].name, "Layer 2");
        assert_eq!(layers[0].kind_label, "Raster");

        let base_id = layers[0].id.clone();
        let new_id = layers[1].id.clone();

        assert!(bridge.toggle_paint_layer_visibility(&new_id));
        assert!(!bridge.view_model().paint_layers[1].visible);
        assert!(bridge.toggle_paint_layer_visibility(&new_id));

        assert!(bridge.toggle_paint_layer_lock(&new_id));
        assert!(bridge.view_model().paint_layers[1].locked);

        assert!(bridge.set_paint_layer_opacity(&new_id, 0.25));
        assert!((bridge.view_model().paint_layers[1].opacity - 0.25).abs() < 1.0e-6);

        assert!(bridge.move_paint_layer(&new_id, -1));
        let moved = bridge.view_model().paint_layers;
        assert_eq!(moved[0].id, new_id, "desceu na ordem de composição");
        assert_eq!(moved[1].id, base_id);

        assert!(bridge.remove_paint_layer(&new_id));
        assert_eq!(bridge.view_model().paint_layers.len(), 1);
    }

    #[test]
    fn paint_layer_panel_refuses_to_remove_the_last_layer() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.add_paint_layer();
        let layers = bridge.view_model().paint_layers;
        assert!(bridge.remove_paint_layer(&layers[1].id));
        let remaining = bridge.view_model().paint_layers;
        assert_eq!(remaining.len(), 1);

        assert!(
            !bridge.remove_paint_layer(&remaining[0].id),
            "a base do raster precisa sobreviver"
        );
        assert_eq!(bridge.view_model().paint_layers.len(), 1);
    }

    #[test]
    fn paint_layer_mutations_reject_unknown_ids_and_non_finite_opacity() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        let unknown = uuid::Uuid::new_v4().to_string();
        assert!(!bridge.toggle_paint_layer_visibility(&unknown));
        assert!(!bridge.toggle_paint_layer_lock(&unknown));
        assert!(!bridge.remove_paint_layer(&unknown));
        assert!(!bridge.move_paint_layer(&unknown, 1));
        assert!(!bridge.set_paint_layer_opacity(&unknown, 0.5));
        assert!(!bridge.set_paint_layer_active(&unknown));
        assert!(!bridge.set_paint_layer_opacity("not-a-uuid", 0.5));

        let id = bridge.view_model().paint_layers[0].id.clone();
        assert!(!bridge.set_paint_layer_opacity(&id, f32::NAN));
        assert!(bridge.view_model().paint_layers[0].opacity.is_finite());
    }

    #[test]
    fn uv_editor_builds_a_real_layout_path_from_mesh_uvs() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let editor = bridge.view_model().uv_editor;
        assert_eq!(editor.face_count, 6, "o cubo padrão tem 6 faces");
        assert!(!editor.truncated);
        assert!(
            editor.layout_commands.starts_with('M'),
            "veio: {}",
            editor.layout_commands
        );
        assert_eq!(
            editor.layout_commands.matches('M').count(),
            editor.face_count,
            "uma subcaminho por face"
        );
        assert_eq!(
            editor.layout_commands.matches('Z').count(),
            editor.face_count
        );
        assert!(editor.island_count >= 1);
        assert_eq!(editor.selected_face, -1, "nada selecionado no início");
    }

    #[test]
    fn uv_editor_reports_the_selected_face_and_clears_when_deselected() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.project.active_mesh_mut().unwrap().faces[2].selected = true;
        bridge.state.sync_selection();
        assert_eq!(bridge.view_model().uv_editor.selected_face, 2);

        bridge.state.project.active_mesh_mut().unwrap().faces[2].selected = false;
        bridge.state.sync_selection();
        assert_eq!(bridge.view_model().uv_editor.selected_face, -1);
    }

    #[test]
    fn uv_seam_toggle_requires_a_selected_face_and_commits_one_entry() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.toggle_selected_uv_seams());
        assert_eq!(
            bridge.state.ui.status,
            "UV: select a face in the viewport first"
        );
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));

        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();

        assert!(bridge.toggle_selected_uv_seams());
        let seams = bridge.state.project.active_mesh().unwrap().uv_seams.len();
        assert_eq!(seams, 4, "uma costura por aresta da face quad");
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));

        assert!(bridge.toggle_selected_uv_seams());
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .uv_seams
                .is_empty(),
            "o segundo toque desmarca"
        );

        assert!(bridge.state.undo());
        assert_eq!(
            bridge.state.project.active_mesh().unwrap().uv_seams.len(),
            4
        );
    }

    #[test]
    fn clearing_uv_seams_is_idempotent_and_reports_when_empty() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.clear_all_uv_seams());
        assert_eq!(bridge.state.ui.status, "UV: there are no seams to clear");

        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        bridge.toggle_selected_uv_seams();
        assert!(bridge.clear_all_uv_seams());
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .uv_seams
                .is_empty()
        );
    }

    #[test]
    fn camera_projection_and_reset() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().is_orthographic);

        bridge.apply(UiIntent::ToggleProjection);
        assert!(bridge.view_model().is_orthographic);
        assert!(bridge.view_model().status_message.contains("Ortográfica"));

        bridge.apply(UiIntent::ToggleProjection);
        assert!(!bridge.view_model().is_orthographic);
        assert!(bridge.view_model().status_message.contains("Perspectiva"));

        let initial_target = bridge.state.session.camera.target;
        bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan {
            dx: 20.0,
            dy: 10.0,
        }));
        assert_ne!(bridge.state.session.camera.target, initial_target);

        bridge.apply(UiIntent::ResetCamera);
        assert_eq!(bridge.state.session.camera.target, initial_target);
        assert!(bridge.view_model().status_message.contains("redefinida"));
    }

    #[test]
    fn save_active_as_asset_intent_creates_project_asset() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let initial_count = bridge.state.project.assets.len();
        assert_eq!(initial_count, 1);

        bridge.apply(UiIntent::SaveActiveAsAsset);
        assert_eq!(bridge.state.project.assets.len(), initial_count + 1);
        let saved_asset = &bridge.state.project.assets[1];
        assert!(saved_asset.name.contains("(Asset)"));
        assert!(
            bridge
                .view_model()
                .status_message
                .contains("biblioteca de assets")
        );
    }

    #[test]
    fn new_commands_execute_via_command_id() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        bridge.execute_command(CommandId::DuplicateSelected);
        assert_eq!(bridge.state.project.assets.len(), 2);

        bridge.execute_command(CommandId::ToggleAssetLibrary);
        assert!(bridge.view_model().asset_library_visible);

        bridge.execute_command(CommandId::ToggleProjection);
        assert!(bridge.view_model().is_orthographic);

        bridge.execute_command(CommandId::ResetCamera);
        assert!(bridge.view_model().status_message.contains("redefinida"));

        bridge.execute_command(CommandId::SelectAll);
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .expect("active mesh")
                .verts
                .iter()
                .all(|v| v.selected)
        );

        bridge.execute_command(CommandId::ClearSelection);
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .expect("active mesh")
                .verts
                .iter()
                .all(|v| !v.selected)
        );

        bridge.execute_command(CommandId::InvertSelection);
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .expect("active mesh")
                .verts
                .iter()
                .all(|v| v.selected)
        );

        bridge.execute_command(CommandId::SaveActiveAsAsset);
        assert_eq!(bridge.state.project.assets.len(), 3);
    }
}
