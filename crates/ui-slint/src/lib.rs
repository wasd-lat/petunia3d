//! Frontend experimental Slint do Petunia3D.
//!
//! Esta crate é deliberadamente paralela à UI egui existente. O shell em Slint
//! emite intenções; o domínio continua em `petunia_core` e a integração de GPU
//! é realizada pelo adapter `WgpuViewport` ou fallback `PlaceholderViewport`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod commands;
pub mod files;
pub mod numeric;
pub mod overlay;
pub mod theme;
pub mod viewport_gpu;
pub mod viewport_soft;

pub use viewport_soft::Software3dViewport;

use commands::{CommandContext, CommandDescriptor, CommandId, CommandRegistry};
use numeric::NumericFieldState;
use overlay::{OverlayEntry, OverlayKind, OverlayStack};
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
    fn render_frame(&mut self, _project: &Project, _camera: &Camera) -> Option<slint::Image> {
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

    fn render_frame(&mut self, project: &Project, camera: &Camera) -> Option<slint::Image> {
        (**self).render_frame(project, camera)
    }
}

/// Bridge entre callbacks Slint e a aplicação. O bridge só aplica intenção
/// semântica ao `AppState`; algoritmos geométricos permanecem no core/commands.
pub struct SlintUiBridge<V: PetuniaViewport> {
    pub state: AppState,
    pub viewport: V,
    pub overlays: OverlayStack,
    pub commands: CommandRegistry,
    pub scene_drawer_visible: bool,
    pub asset_library_visible: bool,
    pub command_search_visible: bool,
    pub settings_visible: bool,
    pub position: [NumericFieldState; 3],
    pub rotation: [NumericFieldState; 3],
    pub scale: [NumericFieldState; 3],
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn new(state: AppState, viewport: V) -> Self {
        let mut bridge = Self {
            state,
            viewport,
            overlays: OverlayStack::default(),
            commands: CommandRegistry::new(),
            scene_drawer_visible: false,
            asset_library_visible: false,
            command_search_visible: false,
            settings_visible: false,
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
                self.state.set_status("save requested by Slint frontend");
            }
            UiIntent::OpenCommandSearch => {
                self.command_search_visible = true;
                self.overlays.push(OverlayEntry {
                    kind: OverlayKind::Modal,
                    pinned: false,
                    accepts_outside_click: true,
                });
            }
            UiIntent::OpenSettings => {
                self.settings_visible = true;
                self.overlays.push(OverlayEntry {
                    kind: OverlayKind::Modal,
                    pinned: false,
                    accepts_outside_click: true,
                });
            }
            UiIntent::ToggleSceneDrawer => {
                self.scene_drawer_visible = !self.scene_drawer_visible;
                if self.scene_drawer_visible {
                    self.overlays.push(OverlayEntry {
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        accepts_outside_click: true,
                    });
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
                if let Some(active_id) = self.state.project.active().map(|a| a.id) {
                    self.state.project.assets.retain(|a| a.id != active_id);
                    self.state.project.active = self.state.project.assets.len().saturating_sub(1);
                    self.state.sync_selection();
                    self.state.mark_dirty();
                    self.state.set_status("Objeto ativo removido.");
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
                }
            }
            UiIntent::ToggleSceneAssetVisibility(id_str) => {
                if let Some(asset) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter_mut().find(|a| a.id == id))
                {
                    asset.visible = !asset.visible;
                    self.state.mark_dirty();
                }
            }
            UiIntent::ToggleSceneAssetLock(id_str) => {
                if let Some(asset) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter_mut().find(|a| a.id == id))
                {
                    asset.locked = !asset.locked;
                    self.state.mark_dirty();
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
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        accepts_outside_click: true,
                    });
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
        self.viewport
            .render_frame(&self.state.project, &self.state.session.camera)
    }

    pub fn handle_escape(&mut self) -> bool {
        if let Some(entry) = self.overlays.esc() {
            match entry.kind {
                OverlayKind::Drawer => {
                    self.scene_drawer_visible = false;
                    self.asset_library_visible = false;
                }
                OverlayKind::Modal => {
                    if self.command_search_visible {
                        self.command_search_visible = false;
                    } else if self.settings_visible {
                        self.settings_visible = false;
                    }
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }

    pub fn handle_click_away(&mut self) -> bool {
        if let Some(entry) = self.overlays.click_away() {
            match entry.kind {
                OverlayKind::Drawer => {
                    self.scene_drawer_visible = false;
                    self.asset_library_visible = false;
                }
                OverlayKind::Modal => {
                    self.command_search_visible = false;
                    self.settings_visible = false;
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }

    pub fn execute_command(&mut self, id: CommandId) {
        self.command_search_visible = false;
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
                self.state.set_status("UV Unwrap solicitado.");
            }
            CommandId::UvPackIslands => {
                self.state.set_status("UV Pack Islands solicitado.");
            }
            CommandId::FrameSelection => {
                self.state
                    .set_status("command executed: model.frame_selection");
            }
            CommandId::ToggleWireframe => {
                self.state.session.show_wireframe_overlay =
                    !self.state.session.show_wireframe_overlay;
                self.state
                    .set_status("command executed: view.toggle_wireframe");
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

    pub fn search_commands(&self, query: &str) -> Vec<CommandDescriptor> {
        self.commands.search(
            query,
            CommandContext {
                workspace: self.state.workspace,
                selection: self.state.selection_domain(),
            },
        )
    }

    pub fn scrub_transform(
        &mut self,
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    ) -> f32 {
        let axis = axis.min(2);
        let field = match kind {
            TransformKind::Position => &mut self.position[axis],
            TransformKind::Rotation => &mut self.rotation[axis],
            TransformKind::Scale => &mut self.scale[axis],
        };
        let new_val = field.scrub(delta, fine);
        self.state.mark_dirty();
        new_val
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
        vm
    }

    fn sync_viewport_context(&mut self) {
        self.viewport.set_workspace(self.state.workspace);
        self.viewport
            .set_selection_domain(self.state.selection_domain());
    }
}

/// Executa o shell experimental Slint com WGPU real (se disponível) ou fallback Placeholder.
pub fn run() -> Result<(), slint::PlatformError> {
    // Garante que o Slint utilize OpenGL (FemtoVG) por padrão no Linux, evitando incompatibilidades
    // conhecidas de apresentação de superfície WGPU em drivers gráficos legados (ex: Intel Gen 7 / Ivy Bridge).
    if std::env::var("SLINT_BACKEND").is_err() {
        // SAFETY: Chamado no ponto de entrada antes de inicializar threads adicionais.
        unsafe {
            std::env::set_var("SLINT_BACKEND", "femtovg");
        }
    }

    println!("🌸 Petunia3D — Shell Slint Experimental");
    println!(
        "🖥️  Backend de janela Slint: {}",
        std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "default".into())
    );

    let window = PetuniaSlintShell::new()?;
    let state = AppState::default();

    let mut viewport: Box<dyn PetuniaViewport> =
        match viewport_gpu::WgpuViewport::try_create_default(1024, 768) {
            Ok(gpu_viewport) => {
                println!("📐 Viewport 3D: Acelerador GPU WGPU Ativo");
                Box::new(gpu_viewport)
            }
            Err(err) => {
                println!("📐 Viewport 3D: Renderizador 3D Software Ativo ({err})");
                Box::new(Software3dViewport::new(1024, 768))
            }
        };

    if let Some(frame) = viewport.render_frame(&state.project, &state.session.camera) {
        window.set_viewport_image(frame);
    }
    window.set_has_gpu_viewport(true);

    let bridge = Arc::new(Mutex::new(SlintUiBridge::new(state, viewport)));
    connect_callbacks(&window, Arc::clone(&bridge));
    let vm = bridge
        .lock()
        .expect("Slint bridge mutex poisoned during startup")
        .view_model();
    sync_window_properties(&window, &vm);
    println!(
        "🚀 Janela aberta com sucesso. Pressione Ctrl+C no terminal ou feche a janela para sair."
    );
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

    theme::apply_theme(window, &vm.current_theme);
}

fn connect_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
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

    let search_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_search_requested(move || {
        if let Ok(mut bridge) = search_bridge.lock() {
            bridge.apply(UiIntent::OpenCommandSearch);
            let items: Vec<CommandItem> = bridge
                .search_commands("")
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.as_str().into(),
                    label: cmd.label_key.into(),
                    shortcut: cmd.shortcut.unwrap_or("").into(),
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

    let query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_command_query_changed(move |query| {
        let items: Vec<CommandItem> = if let Ok(bridge) = query_bridge.lock() {
            bridge
                .search_commands(query.as_str())
                .into_iter()
                .map(|cmd| CommandItem {
                    id: cmd.id.as_str().into(),
                    label: cmd.label_key.into(),
                    shortcut: cmd.shortcut.unwrap_or("").into(),
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
        let Some(id) = CommandId::from_id_str(id_str.as_str()) else {
            return;
        };
        if let Ok(mut bridge) = exec_bridge.lock() {
            bridge.execute_command(id);
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
        let kind = match kind_str.as_str() {
            "cube" => petunia_core::PrimitiveKind::Cube,
            "sphere" => petunia_core::PrimitiveKind::Sphere,
            "cylinder" => petunia_core::PrimitiveKind::Cylinder,
            "plane" => petunia_core::PrimitiveKind::Plane,
            _ => petunia_core::PrimitiveKind::Cube,
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
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let results = bridge.search_commands("cube");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, CommandId::AddCube);

        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        let results_paint = bridge.search_commands("cube");
        assert!(results_paint.is_empty());
    }

    #[test]
    fn transform_scrubbing_updates_values_and_clamps() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());

        let pos_x = bridge.scrub_transform(TransformKind::Position, 0, 5.0, false);
        assert_eq!(pos_x, 0.5);

        let pos_x_fine = bridge.scrub_transform(TransformKind::Position, 0, 2.0, true);
        assert_eq!(pos_x_fine, 0.52);

        let scale_z = bridge.scrub_transform(TransformKind::Scale, 2, -100.0, false);
        assert_eq!(scale_z, 0.001);

        let vm = bridge.view_model();
        assert_eq!(vm.position[0], 0.52);
        assert_eq!(vm.scale[2], 0.001);
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
    fn bridge_saves_and_loads_project_file() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let temp_path = temp_dir.path().join("test_project.petunia");

        bridge.state.project.name = "Slint Test Project".into();
        bridge.state.mark_document_dirty();
        assert!(!bridge.view_model().saved);

        bridge.apply(UiIntent::SaveProjectTo(temp_path.clone()));
        assert!(temp_path.exists());
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

        bridge.apply(UiIntent::ToggleSceneAssetLock(asset_id));
        assert!(bridge.state.project.assets[0].locked);

        let vm = bridge.view_model();
        assert!(!vm.scene_items[0].visible);
        assert!(vm.scene_items[0].locked);
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
        assert!(bridge.view_model().status_message.contains("UV Unwrap"));

        bridge.execute_command(CommandId::UvPackIslands);
        assert!(
            bridge
                .view_model()
                .status_message
                .contains("UV Pack Islands")
        );

        bridge.execute_command(CommandId::DeleteSelected);
        assert_eq!(bridge.state.project.assets.len(), 0);
        assert_eq!(
            bridge.view_model().active_object_title,
            "No Object Selected"
        );
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
