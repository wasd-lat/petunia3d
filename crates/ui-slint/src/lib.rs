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
    pub current_theme: String,
    pub is_orthographic: bool,
    pub is_wireframe: bool,
    pub asset_library_visible: bool,
    pub gizmo: GizmoModel,
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
            current_theme: state.ui.active_theme_id.clone(),
            is_orthographic: state.session.camera.proj == petunia_core::Projection::Ortho,
            is_wireframe: state.session.show_wireframe_overlay,
            asset_library_visible: false,
            gizmo: GizmoModel::default(),
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
                if let Err(error) = self
                    .state
                    .dispatch(&petunia_core::DeleteAssetCmd { asset_index: None })
                {
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

    pub fn select_viewport(&mut self, normalized_x: f32, normalized_y: f32, extend: bool) {
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
        if self.drag.take().is_some() {
            self.cancel_transform();
            return true;
        }
        if self.state.session.tools.cut_session.take().is_some() {
            self.state.set_status("Cut cancelled");
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
            "paint.paint" => self.apply(UiIntent::SetActiveTool("brush".into())),
            _ => return false,
        }
        true
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
    connect_callbacks(&window, Arc::clone(&bridge));
    let vm = bridge
        .lock()
        .expect("Slint bridge mutex poisoned during startup")
        .view_model();
    sync_window_properties(&window, &vm);
    println!("Petunia3D window ready");
    window.run()
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
                _ => None,
            };
            let Some(path) = path else {
                return;
            };
            if let Ok(mut bridge) = bridge.lock() {
                let intent = if id == "file.open" {
                    UiIntent::OpenProjectFrom(path)
                } else {
                    UiIntent::SaveProjectTo(path)
                };
                bridge.apply(intent);
                bridge.command_search_visible = false;
                bridge.overlays.remove(OverlayId::CommandPalette);
                let vm = bridge.view_model();
                let new_frame = if id == "file.open" {
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
