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
use petunia_core::PivotPoint;
use petunia_core::{AppState, Camera, SelectionDomain, Workspace};
use petunia_project::{AlphaMode, Project, ShaderProfile};
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRenderState {
    pub shading: petunia_render::Shading,
    pub xray: bool,
    pub show_triangulation: bool,
    pub textured: bool,
    pub show_wireframe_overlay: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
    /// Domínio de seleção: a camada de seleção precisa saber o que desenhar.
    pub selection_domain: petunia_core::SelectionDomain,
    /// Opacidade da geometria em X-Ray.
    pub xray_opacity: f32,
    pub selection_rgb: [u8; 3],
    pub selection_thickness: f32,
    /// Overlays: grade e wireframe opcional sobre as faces.
    pub show_grid: bool,
    /// Componente sob o cursor (preselection).
    pub hover: petunia_core::HoverTarget,
}

impl Default for ViewportRenderState {
    fn default() -> Self {
        Self {
            shading: petunia_render::Shading::Solid,
            xray: false,
            show_triangulation: false,
            textured: false,
            show_wireframe_overlay: false,
            show_face_orientation: false,
            show_uv_checker: false,
            selection_domain: petunia_core::SelectionDomain::Object,
            xray_opacity: 0.42,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
            show_grid: true,
            hover: petunia_core::HoverTarget::None,
        }
    }
}

/// Gizmo 3D projetado para o overlay da viewport.
///
/// A projeção acontece no bridge; o Slint só desenha as três hastes a partir
/// de origem, comprimento e ângulo em pixels. O overlay não conhece câmera,
/// GPU nem matriz de projeção.
/// Gizmo de transformação e tripé de navegação, tudo em espaço de tela.
//
// O padrão profissional (Blender, C4D, Maya, Plasticity) usa tamanho fixo em
// pixels, nunca escalado pelo mundo: o controle tem sempre o mesmo tamanho
// aparente independente do zoom. Hastes têm setas; o tripé do canto mostra a
// orientação da câmera.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GizmoModel {
    pub visible: bool,
    pub origin_x: f32,
    pub origin_y: f32,
    /// Hastes como comandos `M x y L x y` prontos para o `Path`.
    pub x_commands: String,
    pub y_commands: String,
    pub z_commands: String,
    /// Setas como triângulos preenchidos (`M .. L .. L .. Z`).
    pub x_arrow_commands: String,
    pub y_arrow_commands: String,
    pub z_arrow_commands: String,
    /// Handles adicionais do gizmo combinado (escala e rotação por eixo).
    pub x_scale_commands: String,
    pub y_scale_commands: String,
    pub z_scale_commands: String,
    pub x_rotate_commands: String,
    pub y_rotate_commands: String,
    pub z_rotate_commands: String,
    /// Tripé de navegação no canto inferior esquerdo, em comandos prontos.
    pub view_x_commands: String,
    pub view_y_commands: String,
    pub view_z_commands: String,
    pub view_x_end: [f32; 2],
    pub view_y_end: [f32; 2],
    pub view_z_end: [f32; 2],
    pub view_origin_x: f32,
    pub view_origin_y: f32,
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

    pub const fn id(self) -> &'static str {
        match self {
            Self::Extrude => "model.extrude",
            Self::ExtrudeIndividual => "model.extrude_individual",
            Self::Inset => "model.inset",
            Self::Bevel => "model.bevel",
            Self::PushPull => "model.push_pull",
            Self::ScaleSelection => "model.scale_selection",
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
    pub last_pointer: [f32; 2],
    pub virtual_pointer: [f32; 2],
    pub rotation_angle: f32,
    pub last_angle: f32,
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
    ImportPalette(PathBuf),
    ExportPalette(PathBuf),
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
    AssignMaterialSlot(usize),
    CreateMaterial,
    DuplicateMaterial(usize),
    Paint2dStroke {
        norm_x: f32,
        norm_y: f32,
        phase: i32,
    },
    TogglePaintPixelGrid,
    SetPaintCanvasZoom(i32),
    ProjectFromReference,
    BakeReference,
    ToggleFaceOrientation,
    ToggleUvChecker,
    ToggleProportionalEditing,
    SetProportionalRadius(f32),
    SetProportionalFalloff(String),
    ToggleSnapEnabled,
    SetSnapTarget(String),
    AddProfileRectangle {
        width: f32,
        height: f32,
    },
    AddProfileCircle {
        radius: f32,
        segments: usize,
    },
    AddDecalLayer,
}

/// Representação DTO de um item da árvore de cena do Outliner.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItemModel {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub selected: bool,
    /// Objeto ativo (referência das operações). No máximo um por vez e
    /// visualmente distinto do apenas-selecionado, como no Blender.
    pub active: bool,
    pub verts: usize,
    pub tris: usize,
}

/// Ação rápida do Inspector MODEL derivada do Command Registry.
#[derive(Debug, Clone, PartialEq)]
pub struct QuickActionModel {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub active: bool,
    pub pinned: bool,
}

/// Opção de material do projeto para o seletor do Inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialOptionModel {
    pub id: String,
    pub name: String,
    pub active: bool,
}

/// Linha funcional da pilha Mirror/Symmetry no Inspector MODEL.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierRowModel {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub enabled: bool,
    pub kind: String,
    pub axis: i32,
    pub positive_to_negative: bool,
    pub can_move_up: bool,
    pub can_move_down: bool,
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
    /// Resumo persistente da seleção para a pill da viewport
    /// ("2 objects selected", "Selected: 3 points · 1 edge", "No selection").
    pub selection_summary: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub scene_items: Vec<SceneItemModel>,
    pub parts_items: Vec<SceneItemModel>,
    pub parts_query: String,
    pub parts_selected_only: bool,
    pub parts_sort_by_name: bool,
    pub parts_row_height: f32,
    pub asset_items: Vec<SceneItemModel>,
    pub asset_query: String,
    pub asset_sort_by_name: bool,
    pub asset_thumbnail_size: f32,
    pub active_object_title: String,
    pub active_object_details: String,
    pub active_material_name: String,
    pub scene_stats: String,
    pub uv_stats: String,
    pub current_theme: String,
    pub is_orthographic: bool,
    pub is_wireframe: bool,
    pub shading_mode: String,
    pub xray_opacity: f32,
    pub selection_rgb: [u8; 3],
    pub selection_thickness: f32,
    pub selection_color_hex: String,
    pub show_xray: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
    pub proportional_editing: bool,
    pub proportional_radius: f32,
    pub proportional_falloff: String,
    pub snap_enabled: bool,
    pub snap_target: String,
    pub shading_popover_open: bool,
    /// HUD da operação: título, linhas de valor e dica de controles.
    pub operation_hud_active: bool,
    pub operation_hud_title: String,
    pub operation_hud_lines: Vec<String>,
    pub operation_hud_hint: String,
    /// Feedback curto abaixo da ação ("Moving 3 vertices").
    pub operation_hud_subject: String,
    /// Barra de status contextual.
    pub context_hint: String,
    pub operation_preview_commands: String,
    pub drag_link_commands: String,
    pub hover_label: String,
    pub transform_instant_active: bool,
    pub gizmo_hover_axis: i32,
    pub gizmo_active_axis: i32,
    pub gizmo_constraint_axes: [i32; 2],
    pub asset_library_visible: bool,
    pub gizmo: GizmoModel,
    pub selection_overlay: SelectionOverlayModel,
    pub add_menu_open: bool,
    pub inspector_width: f32,
    pub asset_library_height: f32,
    pub rename_active: bool,
    pub rename_value: String,
    pub context_menu_open: bool,
    pub context_menu_x: f32,
    pub context_menu_y: f32,
    pub context_menu_title: String,
    /// "outliner" (padrão, verbetes do asset) ou "viewport" (só seleção).
    pub context_menu_mode: String,
    pub context_menu_visible: bool,
    pub context_menu_locked: bool,
    pub boolean_operand_name: String,
    pub boolean_ready: bool,
    pub boolean_keep_parts: bool,
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
    pub label_vertical_tool_drag: String,
    pub label_invert_vertical_drag: String,
    pub label_search_assets: String,
    pub label_search_parts: String,
    pub label_inspector: String,
    pub label_expand_inspector: String,
    pub label_collapse_inspector: String,
    pub label_resize_panel_width: String,
    pub label_object_name: String,
    pub label_object_visibility: String,
    pub label_object_lock: String,
    pub label_object_no_selection: String,
    pub label_stats_faces: String,
    pub label_stats_verts: String,
    pub label_stats_tris: String,
    pub label_stats_selection: String,
    pub label_tool_options: String,
    pub label_tool_options_expand: String,
    pub label_tool_options_collapse: String,
    pub label_quick_actions: String,
    pub label_quick_action_customize: String,
    pub label_quick_action_add: String,
    pub label_quick_action_remove: String,
    pub label_quick_action_reset: String,
    pub label_quick_action_done: String,
    pub label_action_subdivide: String,
    pub label_action_fuse: String,
    pub label_action_cut: String,
    pub label_action_intersect: String,
    pub label_action_join: String,
    pub label_action_merge: String,
    pub label_action_slice: String,
    pub label_action_loop_cut: String,
    pub label_material_base_color: String,
    pub label_material_profile: String,
    pub label_material_roughness: String,
    pub label_material_metallic: String,
    pub label_material_normal_scale: String,
    pub label_material_advanced: String,
    pub label_material_assign: String,
    pub label_material_new: String,
    pub label_material_duplicate: String,
    pub label_material_remove: String,
    pub label_material_no_material: String,
    pub label_material_no_selection: String,
    pub label_material_emission_strength: String,
    pub label_material_alpha_cutoff: String,
    pub label_material_texture_albedo: String,
    pub label_material_no_texture: String,
    pub label_material_create_texture: String,
    pub label_material_clear_texture: String,
    pub label_material_profile_pbr: String,
    pub label_material_profile_unlit: String,
    pub label_material_profile_toon: String,
    pub label_material_profile_glass: String,
    pub label_material_profile_emissive: String,
    pub label_material_alpha_opaque: String,
    pub label_material_alpha_mask: String,
    pub label_material_alpha_blend: String,
    pub label_modifier_mirror: String,
    pub label_modifier_symmetry: String,
    pub label_modifiers_empty: String,
    pub label_modifier_apply: String,
    pub label_modifier_axis: String,
    pub label_modifier_add_mirror: String,
    pub label_modifier_add_symmetry: String,
    pub label_modifier_remove: String,
    pub label_modifier_move_up: String,
    pub label_modifier_move_down: String,
    pub label_modifier_direction: String,
    pub label_modifier_positive_to_negative: String,
    pub label_modifier_negative_to_positive: String,
    pub label_tab_parts: String,
    pub label_tab_transform: String,
    pub label_tab_material: String,
    pub label_tab_object: String,
    pub label_tab_modifiers: String,
    pub label_numeric_field_hint: String,
    pub label_model_select: String,
    pub label_model_position: String,
    pub label_model_rotate: String,
    pub label_model_scale: String,
    pub label_model_transform: String,
    pub label_model_lasso: String,
    pub label_model_loop_cut: String,
    pub hint_model_loop_cut: String,
    pub label_model_slice: String,
    pub hint_model_slice: String,
    pub label_model_push_pull: String,
    pub hint_model_push_pull: String,
    pub label_model_profile: String,
    pub hint_model_profile: String,
    pub label_model_pivot: String,
    pub hint_model_pivot: String,
    pub label_profile_depth: String,
    pub label_profile_points: String,
    pub label_profile_close: String,
    pub label_profile_generate: String,
    pub label_profile_revolve: String,
    pub label_profile_cuts: String,
    pub label_profile_presets: String,
    pub label_profile_add_rect: String,
    pub label_profile_add_circle: String,
    pub label_profile_canvas_hint: String,
    pub hint_model_select: String,
    pub hint_model_position: String,
    pub hint_model_rotate: String,
    pub hint_model_scale: String,
    pub hint_model_transform: String,
    pub hint_model_lasso: String,
    pub label_hide_part: String,
    pub label_show_part: String,
    pub label_lock_part: String,
    pub label_unlock_part: String,
    pub label_selected_parts_only: String,
    pub label_sort_parts: String,
    pub label_parts_row_size: String,
    pub label_sort_assets: String,
    pub label_thumbnail_size: String,
    pub label_selection_color: String,
    pub label_highlight_thickness: String,
    pub label_view_wireframe: String,
    pub label_view_wireframe_hint: String,
    pub label_view_solid: String,
    pub label_view_solid_hint: String,
    pub label_view_material: String,
    pub label_view_material_hint: String,
    pub label_view_lit: String,
    pub label_view_lit_hint: String,
    pub label_more_model_tools: String,
    pub label_xray_opacity: String,
    pub label_wire_overlay: String,
    pub label_wire_overlay_hint: String,
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
    pub paint_effect_kind: String,
    pub paint_effect_params: Vec<PaintEffectParam>,
    pub paint_canvas_size: String,
    pub paint_canvas_revision: i32,
    pub paint_pixel_grid: bool,
    pub paint_canvas_zoom: i32,
    pub paint_fill_scope: String,
    pub paint_projection: String,
    pub paint_lock: String,
    pub loop_cut_active: bool,
    pub loop_cut_slide: f32,
    pub loop_cut_cuts: i32,
    pub loop_cut_preview_commands: String,
    pub loop_cut_armed: bool,
    pub profile_active: bool,
    pub pivot_id: String,
    pub pivot_label: String,
    pub pivot_menu_open: bool,
    pub pivot_median_label: String,
    pub pivot_bounds_label: String,
    pub pivot_cursor_label: String,
    pub pivot_individual_label: String,
    pub profile_point_count: i32,
    pub profile_closed: bool,
    pub profile_preview_commands: String,
    pub profile_depth: f32,
    pub tool_activation: String,
    pub keyboard_tool_modal_active: bool,
    pub invert_vertical_drag: bool,
    pub tool_modal_active: bool,
    pub tool_modal_id: String,
    pub tool_modal_title: String,
    pub tool_modal_label: String,
    pub tool_modal_value: f32,
    pub tool_modal_step: f32,
    pub tool_modal_min: f32,
    pub tool_modal_max: f32,
    pub tool_options_active: bool,
    pub tool_options_title: String,
    pub tool_options_hint: String,
    pub tool_options_accessible_label: String,
    pub object_has_selection: bool,
    pub object_id: String,
    pub object_name: String,
    pub object_visible: bool,
    pub object_locked: bool,
    pub object_verts: i32,
    pub object_faces: i32,
    pub object_tris: i32,
    pub object_selection: String,
    pub object_material: String,
    pub object_modifier_count: i32,
    pub material_has_selection: bool,
    pub material_id: String,
    pub material_name: String,
    pub material_profile: String,
    pub material_profile_label: String,
    pub material_profile_ids: Vec<String>,
    pub material_profile_labels: Vec<String>,
    pub material_base_color: [f32; 3],
    pub material_palette: Vec<[f32; 3]>,
    pub material_roughness: f32,
    pub material_metallic: f32,
    pub material_normal_scale: f32,
    pub material_emission: [f32; 3],
    pub material_emission_strength: f32,
    pub material_alpha_mode: String,
    pub material_alpha_label: String,
    pub material_alpha_ids: Vec<String>,
    pub material_alpha_labels: Vec<String>,
    pub material_alpha_cutoff: f32,
    pub material_has_albedo: bool,
    pub material_albedo_label: String,
    pub quick_actions: Vec<QuickActionModel>,
    pub quick_action_candidates: Vec<QuickActionModel>,
    pub modifier_rows: Vec<ModifierRowModel>,
    pub material_slots: Vec<String>,
    pub active_material_slot: i32,
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
                selected: state.session.selection.assets.contains(&asset.id)
                    || active_id == Some(asset.id),
                active: active_id == Some(asset.id),
                verts: asset.mesh.verts.len(),
                tris: asset.mesh.tri_count(),
            })
            .collect();

        let (active_object_title, active_object_details, active_material_name) =
            if let Some(active_asset) = state.project.active() {
                let title = active_asset.name.clone();
                let mut details = format!(
                    "Vertices: {}  ·  Faces: {}",
                    active_asset.mesh.verts.len(),
                    active_asset.mesh.faces.len()
                );
                // Segunda linha só com seleção real: o inspector mostra o
                // que as operações vão atingir, como o Object Info do Blender.
                let summary = format_selection_summary(state);
                if summary != "No selection" {
                    details.push_str(&format!("\n{summary}"));
                }
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
            paint_color: state.paint_color,
            brush_size: state.session.tools.paint_radius,
            brush_opacity: state.session.tools.paint_strength,
            status_message,
            selection_summary: format_selection_summary(state),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            scene_items,
            parts_items: Vec::new(),
            parts_query: String::new(),
            parts_selected_only: false,
            parts_sort_by_name: false,
            parts_row_height: 28.0,
            asset_items: Vec::new(),
            asset_query: String::new(),
            asset_sort_by_name: false,
            asset_thumbnail_size: state.ui.asset_thumbnail_size,
            active_object_title,
            active_object_details,
            active_material_name,
            material_slots: state
                .project
                .project
                .materials
                .iter()
                .map(|m| m.name.clone())
                .collect(),
            active_material_slot: 0,
            scene_stats,
            uv_stats,
            current_theme: state.ui.active_theme_id.clone(),
            is_orthographic: state.session.camera.proj == petunia_core::Projection::Ortho,
            is_wireframe: state.session.show_wireframe_overlay,
            shading_mode: state.shading.id().to_string(),
            xray_opacity: state.session.xray_opacity,
            selection_rgb: state.ui.selection_rgb,
            selection_thickness: state.ui.selection_thickness,
            selection_color_hex: format!(
                "#{:02X}{:02X}{:02X}",
                state.ui.selection_rgb[0], state.ui.selection_rgb[1], state.ui.selection_rgb[2]
            ),
            show_xray: state.session.show_xray,
            show_face_orientation: state.session.show_face_orientation,
            show_uv_checker: state.session.show_uv_checker,
            proportional_editing: state.session.proportional_editing,
            proportional_radius: state.session.proportional_settings.radius,
            proportional_falloff: format!("{:?}", state.session.proportional_settings.falloff)
                .to_lowercase(),
            snap_enabled: state.session.snap_enabled,
            snap_target: format!("{:?}", state.session.snap_settings.target).to_lowercase(),
            shading_popover_open: false,
            operation_hud_active: false,
            operation_hud_title: String::new(),
            operation_hud_lines: Vec::new(),
            operation_hud_hint: String::new(),
            operation_hud_subject: String::new(),
            context_hint: String::new(),
            operation_preview_commands: String::new(),
            drag_link_commands: String::new(),
            hover_label: String::new(),
            transform_instant_active: false,
            gizmo_hover_axis: -1,
            gizmo_active_axis: -1,
            gizmo_constraint_axes: [-1, -1],
            asset_library_visible: false,
            gizmo: GizmoModel::default(),
            selection_overlay: SelectionOverlayModel::default(),
            add_menu_open: false,
            inspector_width: state.ui.right_width,
            asset_library_height: state.ui.shell_asset_library_height,
            rename_active: false,
            rename_value: String::new(),
            context_menu_open: false,
            context_menu_x: 0.0,
            context_menu_y: 0.0,
            context_menu_title: String::new(),
            context_menu_mode: String::new(),
            context_menu_visible: true,
            context_menu_locked: false,
            boolean_operand_name: String::new(),
            boolean_ready: false,
            boolean_keep_parts: false,
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
            label_vertical_tool_drag: String::new(),
            label_invert_vertical_drag: String::new(),
            label_search_assets: String::new(),
            label_search_parts: String::new(),
            label_inspector: String::new(),
            label_expand_inspector: String::new(),
            label_collapse_inspector: String::new(),
            label_resize_panel_width: String::new(),
            label_object_name: String::new(),
            label_object_visibility: String::new(),
            label_object_lock: String::new(),
            label_object_no_selection: String::new(),
            label_stats_faces: String::new(),
            label_stats_verts: String::new(),
            label_stats_tris: String::new(),
            label_stats_selection: String::new(),
            label_tool_options: String::new(),
            label_tool_options_expand: String::new(),
            label_tool_options_collapse: String::new(),
            label_quick_actions: String::new(),
            label_quick_action_customize: String::new(),
            label_quick_action_add: String::new(),
            label_quick_action_remove: String::new(),
            label_quick_action_reset: String::new(),
            label_quick_action_done: String::new(),
            label_action_subdivide: String::new(),
            label_action_fuse: String::new(),
            label_action_cut: String::new(),
            label_action_intersect: String::new(),
            label_action_join: String::new(),
            label_action_merge: String::new(),
            label_action_slice: String::new(),
            label_action_loop_cut: String::new(),
            label_material_base_color: String::new(),
            label_material_profile: String::new(),
            label_material_roughness: String::new(),
            label_material_metallic: String::new(),
            label_material_normal_scale: String::new(),
            label_material_advanced: String::new(),
            label_material_assign: String::new(),
            label_material_new: String::new(),
            label_material_duplicate: String::new(),
            label_material_remove: String::new(),
            label_material_no_material: String::new(),
            label_material_no_selection: String::new(),
            label_material_emission_strength: String::new(),
            label_material_alpha_cutoff: String::new(),
            label_material_texture_albedo: String::new(),
            label_material_no_texture: String::new(),
            label_material_create_texture: String::new(),
            label_material_clear_texture: String::new(),
            label_material_profile_pbr: String::new(),
            label_material_profile_unlit: String::new(),
            label_material_profile_toon: String::new(),
            label_material_profile_glass: String::new(),
            label_material_profile_emissive: String::new(),
            label_material_alpha_opaque: String::new(),
            label_material_alpha_mask: String::new(),
            label_material_alpha_blend: String::new(),
            label_modifier_mirror: String::new(),
            label_modifier_symmetry: String::new(),
            label_modifiers_empty: String::new(),
            label_modifier_apply: String::new(),
            label_modifier_axis: String::new(),
            label_modifier_add_mirror: String::new(),
            label_modifier_add_symmetry: String::new(),
            label_modifier_remove: String::new(),
            label_modifier_move_up: String::new(),
            label_modifier_move_down: String::new(),
            label_modifier_direction: String::new(),
            label_modifier_positive_to_negative: String::new(),
            label_modifier_negative_to_positive: String::new(),
            label_tab_parts: String::new(),
            label_tab_transform: String::new(),
            label_tab_material: String::new(),
            label_tab_object: String::new(),
            label_tab_modifiers: String::new(),
            label_numeric_field_hint: String::new(),
            label_model_select: String::new(),
            label_model_position: String::new(),
            label_model_rotate: String::new(),
            label_model_scale: String::new(),
            label_model_transform: String::new(),
            label_model_lasso: String::new(),
            label_model_loop_cut: String::new(),
            hint_model_loop_cut: String::new(),
            label_model_slice: String::new(),
            hint_model_slice: String::new(),
            label_model_push_pull: String::new(),
            hint_model_push_pull: String::new(),
            label_model_profile: String::new(),
            hint_model_profile: String::new(),
            label_model_pivot: String::new(),
            hint_model_pivot: String::new(),
            label_profile_depth: String::new(),
            label_profile_points: String::new(),
            label_profile_close: String::new(),
            label_profile_generate: String::new(),
            label_profile_revolve: String::new(),
            label_profile_cuts: String::new(),
            label_profile_presets: String::new(),
            label_profile_add_rect: String::new(),
            label_profile_add_circle: String::new(),
            label_profile_canvas_hint: String::new(),
            hint_model_select: String::new(),
            hint_model_position: String::new(),
            hint_model_rotate: String::new(),
            hint_model_scale: String::new(),
            hint_model_transform: String::new(),
            hint_model_lasso: String::new(),
            label_hide_part: String::new(),
            label_show_part: String::new(),
            label_lock_part: String::new(),
            label_unlock_part: String::new(),
            label_selected_parts_only: String::new(),
            label_sort_parts: String::new(),
            label_parts_row_size: String::new(),
            label_sort_assets: String::new(),
            label_thumbnail_size: String::new(),
            label_selection_color: String::new(),
            label_highlight_thickness: String::new(),
            label_view_wireframe: String::new(),
            label_view_wireframe_hint: String::new(),
            label_view_solid: String::new(),
            label_view_solid_hint: String::new(),
            label_view_material: String::new(),
            label_view_material_hint: String::new(),
            label_view_lit: String::new(),
            label_view_lit_hint: String::new(),
            label_more_model_tools: String::new(),
            label_xray_opacity: String::new(),
            label_wire_overlay: String::new(),
            label_wire_overlay_hint: String::new(),
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
            paint_effect_kind: String::new(),
            paint_effect_params: Vec::new(),
            paint_canvas_size: String::new(),
            paint_canvas_revision: 0,
            paint_pixel_grid: true,
            paint_canvas_zoom: 1,
            paint_fill_scope: "ConnectedPixels".to_string(),
            paint_projection: "Surface".to_string(),
            paint_lock: "None".to_string(),
            loop_cut_active: false,
            loop_cut_slide: 0.0,
            loop_cut_cuts: 1,
            loop_cut_preview_commands: String::new(),
            loop_cut_armed: false,
            profile_active: false,
            pivot_id: pivot_id(state.session.pivot_point).to_string(),
            pivot_label: state.t_id(match state.session.pivot_point {
                PivotPoint::MedianPoint => petunia_config::text_id::PIVOT_MEDIAN,
                PivotPoint::BoundingBoxCenter => petunia_config::text_id::PIVOT_BOUNDS,
                PivotPoint::Cursor3D => petunia_config::text_id::PIVOT_CURSOR,
                PivotPoint::IndividualOrigins => petunia_config::text_id::PIVOT_INDIVIDUAL,
            }),
            pivot_menu_open: false,
            pivot_median_label: state.t_id(petunia_config::text_id::PIVOT_MEDIAN),
            pivot_bounds_label: state.t_id(petunia_config::text_id::PIVOT_BOUNDS),
            pivot_cursor_label: state.t_id(petunia_config::text_id::PIVOT_CURSOR),
            pivot_individual_label: state.t_id(petunia_config::text_id::PIVOT_INDIVIDUAL),
            profile_point_count: state.profile.points.len() as i32,
            profile_closed: state.profile.closed,
            profile_preview_commands: String::new(),
            profile_depth: state.profile.depth,
            tool_activation: "drag".to_string(),
            keyboard_tool_modal_active: false,
            invert_vertical_drag: state.ui.invert_vertical_drag,
            tool_modal_active: false,
            tool_modal_id: String::new(),
            tool_modal_title: String::new(),
            tool_modal_label: String::new(),
            tool_modal_value: 0.0,
            tool_modal_step: 0.1,
            tool_modal_min: 0.0,
            tool_modal_max: 0.0,
            tool_options_active: false,
            tool_options_title: String::new(),
            tool_options_hint: String::new(),
            tool_options_accessible_label: String::new(),
            object_has_selection: false,
            object_id: String::new(),
            object_name: String::new(),
            object_visible: true,
            object_locked: false,
            object_verts: 0,
            object_faces: 0,
            object_tris: 0,
            object_selection: String::new(),
            object_material: String::new(),
            object_modifier_count: 0,
            material_has_selection: false,
            material_id: String::new(),
            material_name: String::new(),
            material_profile: String::new(),
            material_profile_label: String::new(),
            material_profile_ids: Vec::new(),
            material_profile_labels: Vec::new(),
            material_base_color: [0.75, 0.75, 0.78],
            material_palette: Vec::new(),
            material_roughness: 0.5,
            material_metallic: 0.0,
            material_normal_scale: 1.0,
            material_emission: [0.0, 0.0, 0.0],
            material_emission_strength: 0.0,
            material_alpha_mode: String::new(),
            material_alpha_label: String::new(),
            material_alpha_ids: Vec::new(),
            material_alpha_labels: Vec::new(),
            material_alpha_cutoff: 0.5,
            material_has_albedo: false,
            material_albedo_label: String::new(),
            quick_actions: Vec::new(),
            quick_action_candidates: Vec::new(),
            modifier_rows: Vec::new(),
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
    /// O backend desenha alvos não selecionados com depth test próprio.
    fn draws_component_guides(&self) -> bool {
        false
    }
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

    fn draws_component_guides(&self) -> bool {
        (**self).draws_component_guides()
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
    pub asset_query: String,
    pub asset_sort_by_name: bool,
    pub parts_query: String,
    pub parts_selected_only: bool,
    pub parts_sort_by_name: bool,
    pub parts_row_height: f32,
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
    /// Sessão paramétrica aberta por atalho: o movimento do mouse já manipula.
    pub keyboard_tool_modal_active: bool,
    pub tool_modal_value: f32,
    /// Renomeação inline do ativo selecionado (Outliner): `Some(nome em edição)`.
    pub rename_draft: Option<String>,
    /// Menu de contexto do Outliner: posição em px lógicos e ativo alvo.
    pub context_menu: Option<ContextMenuState>,
    /// Menu da barra superior aberto, se houver.
    pub menu_open: Option<MenuKind>,
    pub pivot_menu_open: bool,
    /// Popover de opções de shading aberto.
    pub shading_popover_open: bool,
    pub pointer_position: [f32; 2],
    pub modal_text: String,
    pub instant_transform: bool,
    pub gizmo_hover: Option<GizmoHandle>,
    /// Handle do gizmo sendo arrastado, se houver.
    pub gizmo_drag: Option<GizmoHandle>,
    /// Forma ancorada (Line/Rectangle) em curso: canto inicial em pixels do canvas.
    pub shape_anchor: Option<(u32, u32)>,
    /// Plano de corte (Slice) ativo: âncora em pixels lógicos da viewport.
    pub slice_anchor: Option<[f32; 2]>,
    /// Sessão de loop cut com slide interativo (P3D-131).
    pub loop_cut: Option<LoopCutSessionState>,
    /// Aresta atualmente sob o cursor para a prévia não destrutiva do Loop Cut.
    pub loop_cut_hover_ring: Option<petunia_core::LoopRing>,
    /// Snapshot usado para a prévia por hover; commit só ocorre após click.
    pub loop_cut_hover_source: Option<petunia_core::Mesh>,
    pub loop_cut_hover_cuts: usize,
    pub active_material_slot: i32,
    /// Autosave rotativo do shell (P3D-002). Nunca sobrescreve o arquivo oficial.
    pub autosave: petunia_core::AutosaveService,
    /// Snapshot de recuperação detectado no arranque, aguardando decisão.
    pub pending_recovery: Option<petunia_core::RecoveryInfo>,
    pub paint_pixel_grid: bool,
    pub paint_canvas_zoom: i32,
    pub paint_2d_last: Option<(u32, u32)>,
}

/// Menu de contexto do Outliner aberto sobre uma linha do painel Parts,
/// ou menu da viewport (verbetes de seleção) aberto com o botão direito
/// sobre o espaço 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMenuState {
    pub x: f32,
    pub y: f32,
    pub asset: uuid::Uuid,
    /// Verdadeiro no modo viewport (sem alvo de asset): só verbetes de
    /// seleção, nunca rename/visibility/lock.
    pub viewport: bool,
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
    /// Faces marcadas em `uv_selected` (seleção de UV, distinta da seleção 3D).
    pub uv_selected_count: usize,
    /// `true` quando a malha tem mais faces do que o editor desenha.
    pub truncated: bool,
}

/// Feedback visual da seleção na viewport, projetado para `Path` do Slint.
///
/// O shell desenha a silhueta do objeto por cima da imagem do backend como
/// complemento da camada de seleção 3D: sem isso o usuário clica e nada
/// muda na tela. Como no Blender, ativo e selecionado são canais distintos:
/// no máximo um ativo (amarelo) contra N selecionados (laranja).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectionOverlayModel {
    pub visible: bool,
    /// Arestas do contorno (caixa do objeto, arestas ou faces selecionadas).
    pub outline_commands: String,
    /// Silhueta do objeto ATIVO. Canal separado do selecionado para o
    /// usuário saber qual objeto é referência das operações.
    pub active_outline_commands: String,
    /// Marcadores preenchidos dos vértices selecionados.
    pub point_commands: String,
    /// Elementos não selecionados do domínio atual, para o usuário ver o que
    /// pode clicar. Sem isso a viewport mostra um sólido sem alvos visíveis.
    pub unselected_outline_commands: String,
    pub unselected_point_commands: String,
    /// Cor semântica: seleção normal ou ativo em modo componente.
    pub accent: bool,
}

/// Parâmetro numérico de um efeito de camada, já com rótulo e faixa.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintEffectParam {
    pub key: String,
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
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

/// Handle do gizmo sob o cursor ou em arrasto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    X,
    Y,
    Z,
}

impl GizmoHandle {
    /// Índice do eixo para `ModalConstraint::Axis`.
    pub const fn axis(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GizmoTarget {
    handle: GizmoHandle,
    kind: TransformKind,
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

fn project_preview_segment(
    camera: &Camera,
    viewport: [f32; 2],
    a: glam::Vec3,
    b: glam::Vec3,
    commands: &mut String,
) {
    let matrix = camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = matrix * point.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
            return None;
        }
        Some([
            (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
            (0.5 - clip.y / clip.w * 0.5) * viewport[1],
        ])
    };
    if let (Some(a), Some(b)) = (project(a), project(b)) {
        use std::fmt::Write as _;
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        );
    }
}

fn pivot_id(pivot: PivotPoint) -> &'static str {
    match pivot {
        PivotPoint::MedianPoint => "median",
        PivotPoint::BoundingBoxCenter => "bounds",
        PivotPoint::Cursor3D => "cursor",
        PivotPoint::IndividualOrigins => "individual",
    }
}

fn pivot_from_id(id: &str) -> Option<PivotPoint> {
    match id {
        "median" => Some(PivotPoint::MedianPoint),
        "bounds" => Some(PivotPoint::BoundingBoxCenter),
        "cursor" => Some(PivotPoint::Cursor3D),
        "individual" => Some(PivotPoint::IndividualOrigins),
        _ => None,
    }
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
            asset_query: String::new(),
            asset_sort_by_name: false,
            parts_query: String::new(),
            parts_selected_only: false,
            parts_sort_by_name: false,
            parts_row_height: 28.0,
            command_search_visible: false,
            settings_visible: false,
            drag: None,
            viewport_size: [1024.0, 768.0],
            paint_last: None,
            add_menu_open: false,
            tool_modal: None,
            keyboard_tool_modal_active: false,
            tool_modal_value: 0.0,
            rename_draft: None,
            context_menu: None,
            menu_open: None,
            pivot_menu_open: false,
            shape_anchor: None,
            slice_anchor: None,
            loop_cut: None,
            loop_cut_hover_ring: None,
            loop_cut_hover_source: None,
            loop_cut_hover_cuts: 1,
            active_material_slot: 0,
            shading_popover_open: false,
            pointer_position: [512.0, 384.0],
            modal_text: String::new(),
            instant_transform: false,
            gizmo_hover: None,
            gizmo_drag: None,
            autosave: petunia_core::AutosaveService::default(),
            pending_recovery: None,
            paint_pixel_grid: true,
            paint_canvas_zoom: 1,
            paint_2d_last: None,
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
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
            ],
        };
        bridge.sync_viewport_context();
        bridge
    }

    pub fn apply(&mut self, intent: UiIntent) {
        if matches!(
            &intent,
            UiIntent::SetWorkspace(_)
                | UiIntent::SetSelectionDomain(_)
                | UiIntent::SetActiveTool(_)
                | UiIntent::OpenProjectFrom(_)
                | UiIntent::SelectSceneAsset(_)
        ) {
            self.cancel_active_operation();
        }
        match intent {
            UiIntent::SetWorkspace(workspace) => {
                self.state.switch_workspace(workspace);
                // Preselection morta de outro workspace não pode vazar para
                // cá: o hover pertence ao domínio e ao modo onde nasceu.
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
            }
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
            UiIntent::ImportPalette(path) => {
                match petunia_module_paint::PaintModule::import_palette_file(&mut self.state, &path)
                {
                    Ok(count) => {
                        self.state
                            .set_status(format!("imported palette ({count} colors)"));
                        self.state.mark_dirty();
                    }
                    Err(error) => {
                        self.state
                            .set_status(format!("palette import failed: {error}"));
                    }
                }
            }
            UiIntent::ExportPalette(path) => {
                match petunia_module_paint::PaintModule::export_palette_file(&self.state, &path) {
                    Ok(()) => {
                        self.state.set_status("palette exported successfully");
                    }
                    Err(error) => {
                        self.state
                            .set_status(format!("palette export failed: {error}"));
                    }
                }
            }
            UiIntent::Undo => {
                if self.cancel_active_operation() {
                    return;
                }
                if self.state.undo() {
                    self.state.set_status("Desfazer executado.");
                }
            }
            UiIntent::Redo => {
                if self.cancel_active_operation() {
                    return;
                }
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
                if color.iter().all(|component| component.is_finite()) {
                    let color = color.map(|component| component.clamp(0.0, 1.0));
                    self.state.paint_color = color;
                    self.state.session.tools.paint_color = color;
                }
            }
            UiIntent::SetBrushSize(size) => {
                self.state.session.tools.paint_radius = size.clamp(0.01, 100.0);
            }
            UiIntent::SetBrushOpacity(opacity) => {
                self.state.session.tools.paint_strength = opacity.clamp(0.0, 1.0);
            }
            UiIntent::SetActiveTool(tool) => {
                if self.state.session.tools.active_tool == "draw_profile"
                    && tool != "draw_profile"
                    && !self.state.profile.points.is_empty()
                {
                    self.state.profile.clear();
                }
                self.state.session.tools.active_tool = tool.clone();
                self.state.set_status(format!("Ferramenta ativa: {tool}"));
                match tool.as_str() {
                    // Transformações são transacionais por arrasto: a sessão
                    // modal abre no pointer-down da viewport, não ao escolher a
                    // ferramenta.
                    "move" | "rotate" | "scale" | "transform" => {
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
                    "draw_profile" => {
                        petunia_module_model::draw_profile::profile_capture_frame(&mut self.state);
                        self.state.set_status(
                            "Profile: click to add points, click the first point to close",
                        );
                    }
                    "loop_cut" => {
                        self.loop_cut_hover_ring = None;
                        self.loop_cut_hover_source = None;
                        self.loop_cut_hover_cuts = 1;
                        self.state.session.tools.hover = petunia_core::HoverTarget::None;
                        self.state.set_status("Loop Cut: hover a quad edge ring, click to place, scroll to change cuts");
                    }
                    "slice" => {
                        self.state
                            .set_status("Slice: drag in the viewport to define the cut plane");
                    }
                    "fill" => {
                        let count =
                            petunia_module_paint::PaintModule::fill_selection(&mut self.state);
                        self.state.set_status(format!("Filled {count} points"));
                    }
                    // Formas e pincéis do workspace PAINT compartilham o mesmo
                    // índice de tipo de pincel que o resto do domínio.
                    "line" | "rectangle" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(if tool == "line" {
                                petunia_core::BrushType::Line
                            } else {
                                petunia_core::BrushType::Rectangle
                            });
                        self.state
                            .set_status("Shape: press on the surface to anchor, release to commit");
                    }
                    "brush" | "eraser" | "picker" | "airbrush" | "pixel" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(match tool.as_str() {
                                "eraser" => petunia_core::BrushType::Eraser,
                                "picker" => petunia_core::BrushType::Eyedropper,
                                "airbrush" => petunia_core::BrushType::Airbrush,
                                "pixel" => petunia_core::BrushType::Pixel,
                                _ => petunia_core::BrushType::Soft,
                            });
                    }
                    _ => {}
                }
            }
            UiIntent::SelectSceneAsset(id_str) => {
                if let Some(idx) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                {
                    self.state.select_object(Some(idx), false);
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
            UiIntent::AssignMaterialSlot(slot) => {
                self.assign_material_slot(slot);
            }
            UiIntent::CreateMaterial => {
                self.create_material();
            }
            UiIntent::DuplicateMaterial(slot) => {
                self.duplicate_material(slot);
            }
            UiIntent::Paint2dStroke {
                norm_x,
                norm_y,
                phase,
            } => {
                self.paint_2d_stroke(norm_x, norm_y, phase);
            }
            UiIntent::TogglePaintPixelGrid => {
                self.paint_pixel_grid = !self.paint_pixel_grid;
            }
            UiIntent::SetPaintCanvasZoom(zoom) => {
                self.paint_canvas_zoom = zoom.clamp(1, 16);
            }
            UiIntent::ProjectFromReference => {
                self.project_from_reference();
            }
            UiIntent::BakeReference => {
                self.bake_reference();
            }
            UiIntent::ToggleFaceOrientation => {
                self.toggle_face_orientation();
            }
            UiIntent::ToggleUvChecker => {
                self.toggle_uv_checker();
            }
            UiIntent::ToggleProportionalEditing => {
                self.toggle_proportional_editing();
            }
            UiIntent::SetProportionalRadius(radius) => {
                self.set_proportional_radius(radius);
            }
            UiIntent::SetProportionalFalloff(falloff) => {
                self.set_proportional_falloff(&falloff);
            }
            UiIntent::ToggleSnapEnabled => {
                self.toggle_snap_enabled();
            }
            UiIntent::SetSnapTarget(target) => {
                self.set_snap_target(&target);
            }
            UiIntent::AddProfileRectangle { width, height } => {
                self.add_profile_rectangle(width, height);
            }
            UiIntent::AddProfileCircle { radius, segments } => {
                self.add_profile_circle(radius, segments);
            }
            UiIntent::AddDecalLayer => {
                self.add_decal_layer();
            }
        }
        self.sync_viewport_context();
    }

    pub fn apply_viewport_gesture(&mut self, gesture: ViewportGesture) {
        if self.mouse_navigation_suspended() {
            return;
        }
        match gesture {
            ViewportGesture::Orbit { dx, dy } => {
                self.state.session.camera.orbit(dx, dy);
            }
            ViewportGesture::Pan { dx, dy } => {
                self.state.session.camera.pan(dx, dy);
            }
            ViewportGesture::Zoom { delta } => {
                if self.state.session.tools.modal.is_some()
                    && self.state.session.proportional_editing
                {
                    let step = if delta > 0.0 { 0.25 } else { -0.25 };
                    self.adjust_proportional_radius(step);
                    if let Some(drag) = self.drag {
                        self.update_viewport_transform_modified(
                            drag.last_pointer[0],
                            drag.last_pointer[1],
                            false,
                            false,
                        );
                    }
                } else {
                    self.state.session.camera.zoom(delta);
                }
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
            show_face_orientation: self.state.session.show_face_orientation,
            show_uv_checker: self.state.session.show_uv_checker,
            selection_domain: self.state.selection_domain(),
            xray_opacity: self.state.session.xray_opacity,
            selection_rgb: self.state.ui.selection_rgb,
            selection_thickness: self.state.ui.selection_thickness,
            show_grid: self.state.session.show_grid,
            hover: self.state.session.tools.hover,
        };
        self.viewport.render_frame(
            &self.state.project,
            &self.state.session.camera,
            render_state,
        )
    }

    /// Cria uma camada de efeito com parâmetros padrão para o tipo pedido.
    pub fn add_paint_effect_layer(&mut self, kind: &str) -> bool {
        use petunia_project::paint_layers::{PaintEffect, PaintLayer};
        let effect = match kind {
            "Pixelate" => PaintEffect::Pixelate { cell_size: 4 },
            "Posterize" => PaintEffect::Posterize { levels: 6 },
            "Invert" => PaintEffect::Invert,
            "Grain" => PaintEffect::Grain {
                intensity: 0.15,
                seed: 1,
            },
            "BrightnessContrast" => PaintEffect::BrightnessContrast {
                brightness: 0.0,
                contrast: 0.0,
            },
            "HueSaturation" => PaintEffect::HueSaturation {
                hue_shift_deg: 0.0,
                saturation: 0.0,
            },
            _ => return false,
        };
        self.mutate_paint_stack("add effect layer", |stack| {
            stack.add_layer(PaintLayer::new_effect(kind.to_string(), effect));
            true
        })
    }

    /// Ajusta um parâmetro do efeito da camada ativa.
    pub fn set_paint_effect_param(&mut self, key: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        self.mutate_paint_stack("effect parameter", |stack| {
            let Some(layer) = stack.active_mut() else {
                return false;
            };
            let petunia_project::paint_layers::LayerKind::Effect(effect) = &mut layer.kind else {
                return false;
            };
            use petunia_project::paint_layers::PaintEffect;
            match (effect, key) {
                (PaintEffect::Pixelate { cell_size }, "cell_size") => {
                    *cell_size = (value.round() as u32).clamp(1, 64);
                }
                (PaintEffect::Posterize { levels }, "levels") => {
                    *levels = (value.round() as u8).clamp(2, 32);
                }
                (PaintEffect::Grain { intensity, .. }, "intensity") => {
                    *intensity = value.clamp(0.0, 1.0);
                }
                (PaintEffect::Grain { seed, .. }, "seed") => {
                    *seed = value.max(0.0).round() as u32;
                }
                (PaintEffect::BrightnessContrast { brightness, .. }, "brightness") => {
                    *brightness = value.clamp(-1.0, 1.0);
                }
                (PaintEffect::BrightnessContrast { contrast, .. }, "contrast") => {
                    *contrast = value.clamp(-1.0, 1.0);
                }
                (PaintEffect::HueSaturation { hue_shift_deg, .. }, "hue_shift_deg") => {
                    *hue_shift_deg = value.clamp(-180.0, 180.0);
                }
                (PaintEffect::HueSaturation { saturation, .. }, "saturation") => {
                    *saturation = value.clamp(-1.0, 1.0);
                }
                _ => return false,
            }
            true
        })
    }

    /// Dimensões do canvas composto do ativo, quando existe.
    pub fn paint_canvas_dimensions(&self) -> Option<(u32, u32)> {
        let asset = self.state.project.assets.get(self.state.project.active)?;
        if let Some(texture) = asset.texture.as_ref() {
            return Some((texture.w, texture.h));
        }
        let stack = asset.paint_stack.as_ref()?;
        let layer = stack.active()?;
        let canvas = layer.canvas()?;
        Some((canvas.w, canvas.h))
    }

    /// Publica a imagem do canvas 2D na janela, quando houver camada.
    pub fn publish_canvas_image(&mut self, window: &PetuniaSlintShell) {
        if let Some(image) = self.render_paint_canvas() {
            window.set_paint_canvas_image(image);
        }
    }

    /// Converte a camada ativa em imagem Slint para o editor 2D.
    ///
    /// A camada ativa é a superfície que o pincel realmente altera; o composto
    /// (`Asset.texture`) é o que vai para o material.
    pub fn render_paint_canvas(&mut self) -> Option<slint::Image> {
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let asset = self.state.project.assets.get(self.state.project.active)?;
        let canvas = asset
            .paint_stack
            .as_ref()
            .and_then(|stack| stack.active())
            .and_then(|layer| layer.canvas())?;
        let zoom = self.paint_canvas_zoom.clamp(1, 16) as u32;
        let out_w = canvas.w * zoom;
        let out_h = canvas.h * zoom;
        let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(out_w, out_h);
        let pixels = buffer.make_mut_bytes();
        let source = &canvas.pixels;

        if zoom == 1 {
            let length = pixels.len().min(source.len());
            pixels[..length].copy_from_slice(&source[..length]);
        } else {
            for y in 0..out_h {
                let src_y = (y / zoom).min(canvas.h - 1);
                let is_grid_y = self.paint_pixel_grid && zoom >= 4 && (y % zoom == 0);
                for x in 0..out_w {
                    let src_x = (x / zoom).min(canvas.w - 1);
                    let is_grid_x = self.paint_pixel_grid && zoom >= 4 && (x % zoom == 0);
                    let dst_idx = ((y * out_w + x) * 4) as usize;
                    let src_idx = ((src_y * canvas.w + src_x) * 4) as usize;
                    if src_idx + 3 < source.len() && dst_idx + 3 < pixels.len() {
                        if is_grid_x || is_grid_y {
                            pixels[dst_idx] = (source[src_idx] as f32 * 0.75).round() as u8;
                            pixels[dst_idx + 1] = (source[src_idx + 1] as f32 * 0.75).round() as u8;
                            pixels[dst_idx + 2] = (source[src_idx + 2] as f32 * 0.75).round() as u8;
                            pixels[dst_idx + 3] = source[src_idx + 3];
                        } else {
                            pixels[dst_idx..dst_idx + 4]
                                .copy_from_slice(&source[src_idx..src_idx + 4]);
                        }
                    }
                }
            }
        }
        Some(slint::Image::from_rgba8(buffer))
    }

    pub fn resize_viewport(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.viewport.resize(width, height);
        self.viewport_size = [width as f32, height as f32];
        self.state.session.camera.aspect = width as f32 / height as f32;
        self.state.mark_dirty();
    }

    /// Preenche o HUD da operação e a barra de status contextual.
    ///
    /// O HUD diz *o que está acontecendo e com que valor*; a barra diz *como
    /// controlar*. Os dois nunca repetem a mesma informação.
    fn fill_operation_hud(&self, vm: &mut ShellViewModel) {
        let axis_name = |index: usize| ["X", "Y", "Z"][index.min(2)];

        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Loop Cut".to_string();
            vm.operation_hud_lines = vec![format!("Cuts    {}", self.loop_cut_hover_cuts)];
            vm.operation_hud_hint =
                "Hover a quad edge · Scroll Cuts · Click place · Enter confirm · Esc cancel"
                    .to_string();
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Profile".to_string();
            vm.operation_hud_lines = vec![format!("{} point(s)", self.state.profile.points.len())];
            vm.operation_hud_hint = if self.state.profile.closed {
                "Generate Volume or Revolve · Esc cancels".to_string()
            } else {
                "Click to add points · Click first point to close · Esc cancels".to_string()
            };
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }

        // Ferramenta paramétrica modal.
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            let _ = (minimum, maximum);
            vm.operation_hud_active = true;
            vm.operation_hud_title = kind.title().to_string();
            vm.operation_hud_lines =
                vec![format!("{}   {:.3}", kind.label(), self.tool_modal_value)];
            vm.operation_hud_subject = self
                .state
                .project
                .active_mesh()
                .map(|mesh| {
                    let faces = mesh.faces.iter().filter(|face| face.selected).count();
                    let verts = mesh.verts.iter().filter(|vert| vert.selected).count();
                    if faces > 0 {
                        format!("{faces} face(s)")
                    } else {
                        format!("{verts} point(s)")
                    }
                })
                .unwrap_or_default();
            vm.operation_hud_hint = format!(
                "{} Confirm   Esc Cancel   Shift Precision",
                if self.is_instant_tool_mode() {
                    "Click"
                } else {
                    "Release"
                }
            );
            vm.context_hint = format!("{} · {}", kind.title(), vm.operation_hud_hint);
            return;
        }

        // Loop Cut.
        if let Some(session) = &self.loop_cut {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Loop Cut".to_string();
            vm.operation_hud_lines = vec![
                format!("Cuts    {}", session.cuts),
                format!("Slide   {:.3}", session.slide),
            ];
            vm.operation_hud_subject = format!("{} cut(s)", session.cuts);
            vm.operation_hud_hint = "Enter Confirm   Esc Cancel   Drag to slide".to_string();
            vm.context_hint = format!("Loop Cut · {}", vm.operation_hud_hint);
            return;
        }

        if self.state.session.tools.active_tool == "cut"
            && let Some(session) = &self.state.session.tools.cut_session
        {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Cut".into();
            vm.operation_hud_lines = vec![format!("{} segment(s)", session.segments)];
            vm.operation_hud_hint = "Click edge points · Enter Apply · Esc Cancel".into();
            vm.context_hint = vm.operation_hud_hint.clone();
            if let Some(point) = session.edge_start {
                let clip = self.state.session.camera.view_proj() * point.position.extend(1.0);
                if clip.w > 0.0 {
                    let x = (clip.x / clip.w * 0.5 + 0.5) * self.viewport_size[0];
                    let y = (0.5 - clip.y / clip.w * 0.5) * self.viewport_size[1];
                    vm.operation_preview_commands = format!(
                        "M {x:.2} {y:.2} L {:.2} {:.2}",
                        self.pointer_position[0], self.pointer_position[1]
                    );
                }
            }
            return;
        }

        // Transformação por arrasto (gizmo ou ferramenta ativa).
        if let Some(modal) = self.state.session.tools.modal.as_ref() {
            let title = match modal.kind {
                petunia_core::ModalKind::Move => "Move",
                petunia_core::ModalKind::Rotate => "Rotate",
                petunia_core::ModalKind::Scale => "Scale",
                petunia_core::ModalKind::Extrude => "Extrude",
                petunia_core::ModalKind::ExtrudeIndividual => "Extrude Individual",
                petunia_core::ModalKind::Inset => "Inset",
                petunia_core::ModalKind::Bevel => "Bevel",
                petunia_core::ModalKind::PushPull => "Push/Pull",
            };
            let mut lines = Vec::new();
            match modal.constraint {
                petunia_core::ModalConstraint::Axis(i) => {
                    lines.push(format!("{}   {:.3}", axis_name(i), modal.value));
                }
                petunia_core::ModalConstraint::Plane(i) => {
                    lines.push(format!(
                        "Plane {}{}   {:.3}",
                        axis_name((i + 1) % 3),
                        axis_name((i + 2) % 3),
                        modal.value
                    ));
                }
                petunia_core::ModalConstraint::Free => {
                    let components = modal.components;
                    if modal.kind == petunia_core::ModalKind::Move {
                        lines.push(format!("X   {:.3}", components.x));
                        lines.push(format!("Y   {:.3}", components.y));
                        lines.push(format!("Z   {:.3}", components.z));
                    } else {
                        lines.push(format!("Value   {:.3}", modal.value));
                    }
                }
            }
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.to_string();
            if !self.modal_text.is_empty() {
                lines.push(format!("Input   {}", self.modal_text));
            }
            vm.operation_hud_lines = lines;
            vm.operation_hud_subject = format!(
                "{} · {}",
                if self.state.selection_domain() == petunia_core::SelectionDomain::Object {
                    "object"
                } else {
                    "selection"
                },
                match modal.constraint {
                    petunia_core::ModalConstraint::Axis(i) => format!("{} axis", axis_name(i)),
                    petunia_core::ModalConstraint::Plane(i) => {
                        format!("{}{} plane", axis_name((i + 1) % 3), axis_name((i + 2) % 3))
                    }
                    petunia_core::ModalConstraint::Free => "free".to_string(),
                }
            );
            vm.operation_hud_hint = "Enter Confirm   Esc Cancel   Shift Precision".to_string();
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        // Em repouso: a barra informa o domínio e a navegação.
        vm.operation_hud_active = false;
        vm.context_hint = match self.state.selection_domain() {
            petunia_core::SelectionDomain::Object => {
                "Selection: Object   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Vertex => {
                "Selection: Point   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Edge => {
                "Selection: Edge   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Face => {
                "Selection: Face   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
        };
    }

    /// Orbita a câmera, usando a seleção como pivô quando existe.
    ///
    /// É o comportamento de Blender/C4D: o usuário orbita em torno do que
    /// está trabalhando, não de um ponto fixo da cena.
    pub fn orbit_viewport(&mut self, dx: f32, dy: f32) -> bool {
        if self.mouse_navigation_suspended() {
            return false;
        }
        if !dx.is_finite() || !dy.is_finite() {
            return false;
        }
        if let Some(center) = self.selection_pivot() {
            self.state.session.camera.target = center;
        }
        self.state.session.camera.orbit(dx, dy);
        self.state.mark_dirty();
        true
    }

    fn mouse_navigation_suspended(&self) -> bool {
        self.loop_cut.is_some()
            || self.state.session.tools.active_tool == "loop_cut"
            || self.tool_modal.is_some()
            || self.slice_anchor.is_some()
            || matches!(
                self.state.session.tools.active_tool.as_str(),
                "slice" | "draw_profile"
            )
    }

    pub fn set_pivot_point(&mut self, id: &str) -> bool {
        let Some(pivot) = pivot_from_id(id) else {
            return false;
        };
        if self.state.session.pivot_point == pivot {
            return false;
        }
        self.state.session.pivot_point = pivot;
        self.state.mark_dirty();
        self.state.set_status(format!("Pivot: {}", pivot.as_str()));
        true
    }

    /// Distância em mundo que um arrasto de tela representa ao longo de um eixo.
    ///
    /// Projeta a direção do eixo em espaço de tela e mede quanto do movimento
    /// do ponteiro caiu nela, convertendo por `visible_height`.
    fn screen_delta_on_axis(
        &self,
        axis: usize,
        total_x: f32,
        total_y: f32,
        viewport: [f32; 2],
    ) -> f32 {
        let world_axis = match axis {
            0 => glam::Vec3::X,
            1 => glam::Vec3::Y,
            _ => glam::Vec3::Z,
        };
        let view_proj = self.state.session.camera.view_proj();
        let project = |point: glam::Vec3| -> Option<[f32; 2]> {
            let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
            if clip.w <= 0.05 {
                return None;
            }
            let inv_w = 1.0 / clip.w;
            Some([
                (clip.x * inv_w * 0.5 + 0.5) * viewport[0],
                (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * viewport[1],
            ])
        };
        let origin = self.state.session.tools.modal.as_ref().map_or_else(
            || self.state.calculate_pivot(self.state.session.pivot_point),
            |modal| modal.pivot,
        );
        let Some(a) = project(origin) else {
            return 0.0;
        };
        let Some(b) = project(origin + world_axis) else {
            return 0.0;
        };
        let direction = [b[0] - a[0], b[1] - a[1]];
        let length_squared = direction[0] * direction[0] + direction[1] * direction[1];
        if length_squared <= 1.0e-6 {
            return 0.0;
        }
        // Fração do movimento do ponteiro na direção do eixo, em unidades de
        // mundo (a direção projetada corresponde a 1 unidade do eixo).
        (total_x * direction[0] + total_y * direction[1]) / length_squared
    }

    /// Centro da seleção do ativo, quando há algo selecionado.
    fn selection_pivot(&self) -> Option<glam::Vec3> {
        let mesh = self.state.project.active_mesh()?;
        if self.state.selection_domain() == SelectionDomain::Object {
            return Some(self.state.calculate_pivot(self.state.session.pivot_point));
        }
        if !mesh.has_selection() {
            return None;
        }
        let center = mesh.selection_center();
        if center.iter().all(|value| value.is_finite()) {
            Some(glam::Vec3::from_array(center))
        } else {
            None
        }
    }

    /// Atualiza a preselection de componente sob o cursor.
    ///
    /// Sem X-Ray um componente atrás da geometria não é selecionável, então
    /// também não pode ser destacado: o hover valida profundidade contra a
    /// face frontal antes de aceitar o alvo.
    pub fn hover_component(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        if normalized_x.is_finite() && normalized_y.is_finite() {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
        }
        if self.state.workspace == Workspace::Paint {
            // No Paint o pincel consome o ponteiro: limpar em vez de
            // congelar o último hover do modo Model.
            return self.clear_hover();
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            return true;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let previous_edge = match self.state.session.tools.hover {
                petunia_core::HoverTarget::Edge(a, b) => Some((a, b)),
                _ => None,
            };
            let next = match self.pick_target_for_domain(
                SelectionDomain::Edge,
                normalized_x,
                normalized_y,
            ) {
                petunia_core::HoverTarget::Edge(a, b) => {
                    self.state.project.active_mesh().and_then(|mesh| {
                        petunia_core::LoopRing::discover(mesh, (a, b))
                            .ok()
                            .map(|ring| (mesh.clone(), ring, (a, b)))
                    })
                }
                _ => None,
            };
            self.loop_cut_hover_source = next.as_ref().map(|(mesh, _, _)| mesh.clone());
            self.loop_cut_hover_ring = next.as_ref().map(|(_, ring, _)| ring.clone());
            let next_edge = next.map(|(_, _, edge)| edge);
            self.state.session.tools.hover = next_edge
                .map(|(a, b)| petunia_core::HoverTarget::Edge(a, b))
                .unwrap_or_default();
            let changed = self.refresh_loop_cut_hover_preview();
            if let Some(ring) = &self.loop_cut_hover_ring {
                if ring.face_count() > 0 {
                    self.state.set_status("Loop Cut: click to place, scroll to change cuts, Enter to confirm, Esc to cancel");
                }
                return changed || previous_edge != next_edge;
            }
            return changed || previous_edge.is_some();
        }
        let next = self.pick_viewport_target(normalized_x, normalized_y);

        if next == self.state.session.tools.hover {
            return self.state.session.tools.active_tool == "cut";
        }
        self.state.session.tools.hover = next;
        true
    }

    fn refresh_loop_cut_hover_preview(&self) -> bool {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return false;
        };
        let Ok(segments) = ring.preview(source, self.loop_cut_hover_cuts, 0.0) else {
            return false;
        };
        !segments.is_empty()
    }

    fn loop_cut_hover_preview_commands(&self) -> String {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return String::new();
        };
        let Ok(segments) = ring.preview(source, self.loop_cut_hover_cuts, 0.0) else {
            return String::new();
        };
        let mut commands = String::new();
        for [a, b] in segments {
            project_preview_segment(
                &self.state.session.camera,
                self.viewport_size,
                a,
                b,
                &mut commands,
            );
        }
        commands
    }

    /// Atribui o material selecionado ao objeto e às faces selecionadas quando existirem.
    pub fn assign_material_slot(&mut self, slot: usize) -> bool {
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let Some(material_id) = self.state.project.project.materials.get(slot).map(|m| m.id) else {
            return false;
        };
        let selected_faces: Vec<usize> = self
            .state
            .project
            .active_mesh()
            .map(|mesh| {
                mesh.faces
                    .iter()
                    .enumerate()
                    .filter_map(|(index, face)| face.selected.then_some(index))
                    .collect()
            })
            .unwrap_or_default();
        self.state.checkpoint("assign material");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.material_id = Some(material_id);
        }
        if !selected_faces.is_empty()
            && let Some(mesh) = self.state.project.active_mesh_mut()
        {
            for index in &selected_faces {
                mesh.faces[*index].material_slot = Some(slot);
            }
        }
        let message = if selected_faces.is_empty() {
            format!("Assigned material slot {slot} to object")
        } else {
            format!(
                "Assigned material slot {slot} to object and {} face(s)",
                selected_faces.len()
            )
        };
        self.state.set_status(message);
        self.state.emit_mesh_changed();
        true
    }

    /// Cria um novo material no projeto e o seleciona como ativo.
    pub fn create_material(&mut self) -> bool {
        let count = self.state.project.project.materials.len();
        let name = format!("Material {}", count + 1);
        self.state.checkpoint("create material");
        self.state
            .project
            .project
            .materials
            .push(petunia_project::Material::new(name));
        self.active_material_slot = count as i32;
        self.state
            .set_status(format!("Created Material {}", count + 1));
        self.state.mark_dirty();
        true
    }

    /// Duplica o material do slot indicado e o seleciona como ativo.
    pub fn duplicate_material(&mut self, slot: usize) -> bool {
        let Some(mat) = self.state.project.project.materials.get(slot).cloned() else {
            return false;
        };
        let mut dup = mat;
        dup.id = uuid::Uuid::new_v4();
        dup.name = format!("{} Copy", dup.name);
        self.state.checkpoint("duplicate material");
        self.state.project.project.materials.push(dup);
        let new_idx = self.state.project.project.materials.len() - 1;
        self.active_material_slot = new_idx as i32;
        self.state
            .set_status(format!("Duplicated material to slot {new_idx}"));
        self.state.mark_dirty();
        true
    }

    pub fn select_material_slot(&mut self, slot: i32) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if slot >= self.state.project.project.materials.len() {
            return false;
        }
        self.active_material_slot = slot as i32;
        true
    }

    pub fn remove_material(&mut self, slot: i32) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if self.state.project.project.materials.len() <= 1
            || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        let material_id = self.state.project.project.materials[slot].id;
        self.state.checkpoint("remove material");
        self.state.project.project.remove_material(material_id);
        self.state.project.project.bump_materials();
        self.active_material_slot = (slot as i32 - 1).max(0);
        self.state.mark_dirty();
        true
    }

    pub fn set_active_material_base_color(&mut self, red: f32, green: f32, blue: f32) -> bool {
        if [red, green, blue].iter().any(|value| !value.is_finite()) {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material color");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.base_color[0] = red.clamp(0.0, 1.0);
        material.base_color[1] = green.clamp(0.0, 1.0);
        material.base_color[2] = blue.clamp(0.0, 1.0);
        material.base_color[3] = 1.0;
        self.state.project.project.bump_materials();
        self.state.mark_dirty();
        true
    }

    pub fn set_active_material_scalar(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !matches!(
            field,
            "roughness" | "metallic" | "normal-scale" | "emission-strength" | "alpha-cutoff"
        ) || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        self.state.checkpoint("change material");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        match field {
            "roughness" => material.roughness = value.clamp(0.0, 1.0),
            "metallic" => material.metallic = value.clamp(0.0, 1.0),
            "normal-scale" => material.normal_scale = value.clamp(0.0, 10.0),
            "emission-strength" => material.emission_strength = value.clamp(0.0, 10.0),
            "alpha-cutoff" => material.alpha_cutoff = value.clamp(0.0, 1.0),
            _ => return false,
        }
        material.validate();
        self.state.project.project.bump_materials();
        self.state.mark_dirty();
        true
    }

    pub fn set_active_material_profile(&mut self, profile: i32) -> bool {
        let profile = match profile {
            0 => ShaderProfile::Pbr,
            1 => ShaderProfile::Unlit,
            2 => ShaderProfile::Toon,
            3 => ShaderProfile::Glass,
            4 => ShaderProfile::Emissive,
            _ => return false,
        };
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material profile");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.profile = profile;
        self.state.project.project.bump_materials();
        self.state.mark_dirty();
        true
    }

    pub fn set_active_material_alpha_mode(&mut self, mode: i32) -> bool {
        let mode = match mode {
            0 => AlphaMode::Opaque,
            1 => AlphaMode::Mask,
            2 => AlphaMode::Blend,
            _ => return false,
        };
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material alpha");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.alpha_mode = mode;
        self.state.project.project.bump_materials();
        self.state.mark_dirty();
        true
    }

    pub fn create_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        let Some(material) = self.state.project.project.materials.get(slot) else {
            return false;
        };
        let color = material.base_color;
        self.state.checkpoint("create albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = Some(petunia_project::Canvas::new(
            256,
            256,
            [
                (color[0].clamp(0.0, 1.0) * 255.0) as u8,
                (color[1].clamp(0.0, 1.0) * 255.0) as u8,
                (color[2].clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ],
        ));
        self.state.project.project.bump_materials();
        self.state.project.project.bump_textures();
        self.state.mark_dirty();
        true
    }

    pub fn clear_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        if !self
            .state
            .project
            .project
            .materials
            .get(slot)
            .is_some_and(|material| material.albedo_texture.is_some())
        {
            return false;
        }
        self.state.checkpoint("clear albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = None;
        self.state.project.project.bump_materials();
        self.state.project.project.bump_textures();
        self.state.mark_dirty();
        true
    }

    pub fn toggle_quick_action(&mut self, id: &str) -> bool {
        let pinned = self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id);
        self.state.ui.set_model_quick_action_pinned(id, !pinned)
    }

    pub fn reset_quick_actions(&mut self) -> bool {
        self.state.ui.reset_model_quick_actions()
    }

    pub fn execute_quick_action(&mut self, id: &str) -> bool {
        if !self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id)
        {
            return false;
        }
        match id {
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".into()));
                true
            }
            other => {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn add_modifier(&mut self, kind: &str) -> bool {
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        if self.state.project.assets.get(asset_index).is_none() {
            return false;
        }
        let modifier = match kind {
            "mirror" => petunia_project::ModifierInstance::mirror(0, 0.001),
            "symmetry" => petunia_project::ModifierInstance::symmetry(0, true, 0.001),
            _ => return false,
        };
        self.state.checkpoint("add modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.modifiers.push(modifier);
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if modifier.enabled == enabled {
            return false;
        }
        self.state.checkpoint("toggle modifier");
        if let Some(modifier) = self.state.project.assets[asset_index]
            .modifiers
            .iter_mut()
            .find(|item| item.id == id)
        {
            modifier.enabled = enabled;
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn remove_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        if !self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .any(|item| item.id == id)
        {
            return false;
        }
        self.state.checkpoint("remove modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .retain(|item| item.id != id);
        self.state.emit_mesh_changed();
        true
    }

    pub fn move_modifier(&mut self, id: &str, direction: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let Some(current) = self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .position(|item| item.id == id)
        else {
            return false;
        };
        let target = if direction < 0 {
            if current == 0 {
                return false;
            }
            current - 1
        } else {
            current + 1
        };
        if target >= self.state.project.assets[asset_index].modifiers.len() {
            return false;
        }
        self.state.checkpoint("reorder modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .swap(current, target);
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_axis(&mut self, id: &str, axis: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let axis_value = (axis.clamp(0, 2)) as usize;
        self.state.checkpoint("change modifier axis");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Mirror { axis, .. } => *axis = axis_value,
            petunia_project::ModifierKind::Symmetry { axis, .. } => *axis = axis_value,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_direction(&mut self, id: &str, direction_value: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        self.state.checkpoint("change modifier direction");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Symmetry {
                positive_to_negative,
                ..
            } => *positive_to_negative = direction_value,
            petunia_project::ModifierKind::Mirror { .. } => return false,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn apply_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let Some(asset) = self.state.project.assets.get(asset_index) else {
            return false;
        };
        if !asset.modifiers.iter().any(|item| item.id == id) {
            return false;
        }
        let evaluated = asset.evaluated_mesh();
        self.state.checkpoint("apply modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.mesh = evaluated;
            asset.modifiers.retain(|item| item.id != id);
        }
        self.state.emit_mesh_changed();
        true
    }

    fn profile_preview_commands(&self) -> String {
        let profile = &self.state.profile;
        if profile.points.is_empty() || self.viewport_size[0] <= 1.0 || self.viewport_size[1] <= 1.0
        {
            return String::new();
        }
        let matrix = self.state.session.camera.view_proj();
        let mut commands = String::new();
        let mut projected = Vec::with_capacity(profile.points.len());
        for index in 0..profile.points.len() {
            let point = matrix * profile.to_3d(index).extend(1.0);
            if !point.is_finite() || point.w <= 0.05 || point.z < 0.0 || point.z > point.w {
                return String::new();
            }
            projected.push([
                (point.x / point.w * 0.5 + 0.5) * self.viewport_size[0],
                (0.5 - point.y / point.w * 0.5) * self.viewport_size[1],
            ]);
        }
        use std::fmt::Write as _;
        for (index, point) in projected.iter().enumerate() {
            let action = if index == 0 { 'M' } else { 'L' };
            let _ = write!(commands, "{action} {:.2} {:.2} ", point[0], point[1]);
        }
        if profile.closed {
            commands.push_str("Z ");
        } else {
            let [x, y] = self.pointer_position;
            let _ = write!(commands, "L {:.2} {:.2}", x, y);
        }
        commands
    }

    pub fn close_profile(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" || self.state.profile.closed {
            return false;
        }
        if self.state.profile.points.len() < 3 {
            self.state
                .set_status("Profile requires at least three points");
            return false;
        }
        self.state.profile.closed = true;
        self.state.mark_dirty();
        self.state
            .set_status("Profile closed: choose Generate Volume or Revolve");
        true
    }

    pub fn set_pivot_menu_open(&mut self, open: bool) -> bool {
        self.shading_popover_open = false;
        if open {
            self.close_menu();
            self.close_context_menu();
            self.overlays.push(OverlayEntry {
                id: OverlayId::PivotMenu,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
            self.pivot_menu_open = true;
            self.state.set_status("Choose a transform pivot");
        } else if self.overlays.remove(OverlayId::PivotMenu).is_some() {
            self.pivot_menu_open = false;
        }
        self.pivot_menu_open == open
    }

    pub fn set_profile_depth(&mut self, depth: f32) -> bool {
        if !depth.is_finite()
            || depth <= 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        self.state.profile.depth = depth.clamp(0.01, 1000.0);
        self.state.mark_dirty();
        true
    }

    pub fn generate_profile_extrude(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" || !self.state.profile.closed {
            return false;
        }
        let asset_count = self.state.project.assets.len();
        petunia_module_model::draw_profile::generate_extrude(&mut self.state);
        let generated = self.state.project.assets.len() > asset_count;
        if generated {
            self.state.session.tools.active_tool = "select".to_string();
        }
        generated
    }

    pub fn generate_profile_revolve(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile"
            || self.state.profile.points.len() < 2
        {
            return false;
        }
        let asset_count = self.state.project.assets.len();
        petunia_module_model::draw_profile::generate_revolve(&mut self.state);
        let generated = self.state.project.assets.len() > asset_count;
        if generated {
            self.state.session.tools.active_tool = "select".to_string();
        }
        generated
    }

    pub fn adjust_loop_cut_hover_count(&mut self, delta: i32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut"
            || self.loop_cut.is_some()
            || self.loop_cut_hover_ring.is_none()
        {
            return false;
        }
        let next = (self.loop_cut_hover_cuts as i32 + delta).clamp(1, 32) as usize;
        if next == self.loop_cut_hover_cuts {
            return false;
        }
        self.loop_cut_hover_cuts = next;
        self.state
            .set_status(format!("Loop Cut: {next} cut(s) · click to place"));
        true
    }

    pub fn place_loop_cut_hover(&mut self) -> bool {
        if self.loop_cut.is_some() || self.state.session.tools.active_tool != "loop_cut" {
            return false;
        }
        let (Some(ring), Some(source)) = (
            self.loop_cut_hover_ring.take(),
            self.loop_cut_hover_source.take(),
        ) else {
            self.state
                .set_status("Loop Cut: move the pointer over a quad edge ring");
            return false;
        };
        self.begin_loop_cut_from_ring(ring, source, self.loop_cut_hover_cuts)
    }

    pub fn update_loop_cut_hover(&mut self, x: f32, y: f32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut" || self.loop_cut.is_some() {
            return false;
        }
        if !x.is_finite()
            || !y.is_finite()
            || self.viewport_size[0] <= 1.0
            || self.viewport_size[1] <= 1.0
        {
            return false;
        }
        self.pointer_position = [x, y];
        self.hover_component(x / self.viewport_size[0], y / self.viewport_size[1])
    }

    /// A preselection e o clique usam exatamente o mesmo hit test. Ponto e
    /// aresta usam alvos em pixels lógicos, independentes da distância da
    /// câmera; objetos e faces usam a superfície real, nunca uma esfera de
    /// bounding que seleciona no vazio.
    fn pick_viewport_target(&self, x: f32, y: f32) -> petunia_core::HoverTarget {
        self.pick_target_for_domain(self.state.selection_domain(), x, y)
    }

    fn pick_target_for_domain(
        &self,
        domain: SelectionDomain,
        x: f32,
        y: f32,
    ) -> petunia_core::HoverTarget {
        use petunia_core::HoverTarget as Target;

        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
        {
            return Target::None;
        }
        let (width, height) = (self.viewport_size[0], self.viewport_size[1]);
        if width <= 1.0 || height <= 1.0 {
            return Target::None;
        }
        let camera = &self.state.session.camera;
        let scene =
            petunia_core::viewport_query::ViewportSceneQuery::new(&self.state.project.project);
        let ndc = [x * 2.0 - 1.0, 1.0 - y * 2.0];
        if domain == SelectionDomain::Object {
            return scene
                .nearest_object(camera, ndc)
                .map_or(Target::None, Target::Object);
        }
        let Some(asset) = self
            .state
            .project
            .active()
            .filter(|asset| asset.visible && !asset.locked)
        else {
            return Target::None;
        };
        let mode = match domain {
            SelectionDomain::Vertex => petunia_core::SelectMode::Vertex,
            SelectionDomain::Edge => petunia_core::SelectMode::Edge,
            SelectionDomain::Face => petunia_core::SelectMode::Face,
            SelectionDomain::Object => return Target::None,
        };
        let through = self.state.session.show_xray
            || self.state.session.shading == petunia_core::Shading::Wireframe;
        petunia_core::picking::pick_mesh_filtered(
            &asset.mesh,
            camera,
            glam::Vec2::new(width, height),
            glam::Vec2::from_array(ndc),
            mode,
            through,
            |point| scene.point_visible(camera, point),
        )
        .map_or(Target::None, |hit| match hit.component {
            petunia_core::picking::PickComponent::Vertex(i) => Target::Vertex(i as u32),
            petunia_core::picking::PickComponent::Edge(a, b) => Target::Edge(a, b),
            petunia_core::picking::PickComponent::Face(i) => Target::Face(i),
        })
    }

    /// Limpa a preselection (ponteiro saiu da viewport).
    pub fn clear_hover(&mut self) -> bool {
        if !self.state.session.tools.hover.is_some() {
            return false;
        }
        self.state.session.tools.hover = petunia_core::HoverTarget::None;
        true
    }

    /// Oclusão do segmento olho→ponto: a preselection respeita faces, salvo
    /// em X-Ray. `origin`/`direction` documentam o raio que gerou `position`;
    /// o teste usa a mesma cena canônica (`point_visible`) do picking.
    pub fn is_occluded(
        &self,
        _origin: glam::Vec3,
        _direction: glam::Vec3,
        position: glam::Vec3,
    ) -> bool {
        if self.state.session.show_xray {
            return false;
        }
        let scene =
            petunia_core::viewport_query::ViewportSceneQuery::new(&self.state.project.project);
        !scene.point_visible(&self.state.session.camera, position)
    }

    /// Handle do gizmo sob o cursor (preselection, sem clique).
    /// Handle do gizmo sob um ponto de tela, dentro de um raio de tolerância.
    ///
    /// O teste é em espaço de tela porque o gizmo tem tamanho fixo em pixels:
    /// o alvo do mouse precisa ser generoso (12 px) mesmo com a haste fina.
    pub fn gizmo_handle_at(&self, x: f32, y: f32) -> Option<GizmoHandle> {
        self.gizmo_target_at(x, y).map(|target| target.handle)
    }

    fn gizmo_target_at(&self, x: f32, y: f32) -> Option<GizmoTarget> {
        let gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        if !gizmo.visible {
            return None;
        }
        let active = self.state.session.tools.active_tool.as_str();
        let dist_from_origin = (x - gizmo.origin_x).hypot(y - gizmo.origin_y);
        // Desambiguação do gizmo universal: zona morta do centro protege contra cliques acidentais
        if active == "transform" && dist_from_origin < 12.0 {
            return None;
        }
        let families = [
            (
                TransformKind::Scale,
                9.0,
                [
                    &gizmo.x_scale_commands,
                    &gizmo.y_scale_commands,
                    &gizmo.z_scale_commands,
                ],
            ),
            (
                TransformKind::Rotation,
                7.0,
                [
                    &gizmo.x_rotate_commands,
                    &gizmo.y_rotate_commands,
                    &gizmo.z_rotate_commands,
                ],
            ),
            (
                match active {
                    "rotate" => TransformKind::Rotation,
                    "scale" => TransformKind::Scale,
                    _ => TransformKind::Position,
                },
                12.0,
                [&gizmo.x_commands, &gizmo.y_commands, &gizmo.z_commands],
            ),
        ];
        for (kind, hit_radius, commands_by_axis) in families {
            let mut family_best: Option<(GizmoTarget, f32)> = None;
            for (axis, commands) in commands_by_axis.into_iter().enumerate() {
                let handle = [GizmoHandle::X, GizmoHandle::Y, GizmoHandle::Z][axis];
                let numbers: Vec<f32> = commands
                    .split_whitespace()
                    .filter_map(|token| token.parse::<f32>().ok())
                    .collect();
                for segment in numbers.as_chunks::<2>().0.windows(2) {
                    let a = [segment[0][0], segment[0][1]];
                    let b = [segment[1][0], segment[1][1]];
                    if (a[0] - b[0]).hypot(a[1] - b[1]) < 0.25 {
                        continue;
                    }
                    let distance = point_segment_distance([x, y], a, b);
                    if distance <= hit_radius
                        && family_best.is_none_or(|(_, current)| distance < current)
                    {
                        family_best = Some((GizmoTarget { handle, kind }, distance));
                    }
                }
            }
            if let Some((target, _)) = family_best {
                return Some(target);
            }
        }
        None
    }

    /// Atualiza o handle do gizmo sob o cursor (preselection).
    pub fn hover_gizmo(&mut self, x: f32, y: f32) -> bool {
        if self.gizmo_drag.is_some() {
            return false;
        }
        let next = self.gizmo_handle_at(x, y);
        if next == self.gizmo_hover {
            return false;
        }
        self.gizmo_hover = next;
        true
    }

    /// Inicia o arrasto no handle do gizmo, restringindo a transformação ao eixo.
    pub fn begin_gizmo_drag(&mut self, x: f32, y: f32) -> bool {
        let Some(target) = self.gizmo_target_at(x, y) else {
            return false;
        };
        let handle = target.handle;
        let kind = target.kind;
        if !self.begin_viewport_transform(kind, x, y) {
            return false;
        }
        // A restrição de eixo é do domínio: o preview já sai no eixo certo.
        let _ = self
            .state
            .set_modal_constraint(petunia_core::ModalConstraint::Axis(handle.axis()));
        self.gizmo_drag = Some(handle);
        self.state.set_status(format!(
            "{} · {} axis",
            match kind {
                TransformKind::Position => "Move",
                TransformKind::Rotation => "Rotate",
                TransformKind::Scale => "Scale",
            },
            match handle {
                GizmoHandle::X => "X",
                GizmoHandle::Y => "Y",
                GizmoHandle::Z => "Z",
            }
        ));
        true
    }

    /// Encerra o arrasto do gizmo.
    pub fn end_gizmo_drag(&mut self) -> bool {
        if self.gizmo_drag.take().is_none() {
            return false;
        }
        self.end_viewport_transform();
        self.gizmo_hover = None;
        true
    }

    /// Atualiza o plano de corte enquanto o ponteiro se move.
    pub fn update_viewport_slice(&mut self, x: f32, y: f32) -> bool {
        if self.slice_anchor.is_none() {
            return false;
        }
        self.update_slice(x, y)
    }

    /// Inicia uma transformação modal por arrasto na viewport.
    pub fn begin_viewport_transform(&mut self, kind: TransformKind, x: f32, y: f32) -> bool {
        self.modal_text.clear();
        self.instant_transform = false;
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
                    last_pointer: [x, y],
                    virtual_pointer: [x, y],
                    rotation_angle: 0.0,
                    last_angle: 0.0,
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
        self.update_viewport_transform_modified(x, y, false, false)
    }

    pub fn update_viewport_transform_modified(
        &mut self,
        x: f32,
        y: f32,
        fine: bool,
        snap: bool,
    ) -> bool {
        self.pointer_position = [x, y];
        if !self.modal_text.is_empty() {
            return false;
        }
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let Some(mut drag) = self.drag else {
            return false;
        };
        let precision = if fine { 0.1 } else { 1.0 };
        for (axis, pointer) in [x, y].into_iter().enumerate() {
            let direction = if axis == 1 && self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
            drag.virtual_pointer[axis] +=
                (pointer - drag.last_pointer[axis]) * precision * direction;
            drag.last_pointer[axis] = pointer;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return false;
        };
        let (constraint, pivot, normal) = (modal.constraint, modal.pivot, modal.normal);
        let camera = &self.state.session.camera;
        let viewport = glam::Vec2::from_array(drag.viewport);
        let start = glam::Vec2::from_array(drag.start);
        let current = glam::Vec2::from_array(drag.virtual_pointer);
        let delta = current - start;
        let axis = |index| match index {
            0 => glam::Vec3::X,
            1 => glam::Vec3::Y,
            _ => glam::Vec3::Z,
        };
        use petunia_core::{ModalConstraint, transform_projection as projection};
        let result = match drag.kind {
            TransformKind::Position => {
                let (mut translation, mut scalar) = match constraint {
                    ModalConstraint::Axis(index) => {
                        let value = projection::axis_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or_else(|| {
                            self.screen_delta_on_axis(index, delta.x, delta.y, drag.viewport)
                        });
                        (axis(index) * value, value)
                    }
                    ModalConstraint::Plane(index) => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                    ModalConstraint::Free => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            camera.forward(),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                };
                if snap || self.state.session.snap_enabled {
                    let mut settings = self.state.session.snap_settings.clone();
                    settings.enabled = true;
                    let mesh = self.state.project.active_mesh();
                    let current_pivot = pivot + translation;
                    let query = petunia_core::SnapQuery {
                        point: current_pivot,
                        start_point: Some(pivot),
                        settings: &settings,
                        mesh,
                    };
                    let snap_res = petunia_core::snap_point(query);
                    if snap_res.snapped {
                        translation = snap_res.point - pivot;
                        match constraint {
                            ModalConstraint::Axis(index) => {
                                let a = axis(index);
                                translation = a * translation.dot(a);
                                scalar = translation.dot(a);
                            }
                            ModalConstraint::Plane(index) => {
                                let a = axis(index);
                                translation = translation - a * translation.dot(a);
                            }
                            ModalConstraint::Free => {}
                        }
                    } else if settings.target == petunia_core::SnapTarget::Grid {
                        let step = settings.grid_spacing.max(0.001);
                        translation = (translation / step).round() * step;
                        scalar = (scalar / step).round() * step;
                    }
                }
                self.state.update_modal(translation, scalar)
            }
            TransformKind::Rotation => {
                let normal = match constraint {
                    ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => axis(i),
                    ModalConstraint::Free => normal,
                };
                let angle =
                    projection::rotation_angle(camera, viewport, start, current, pivot, normal)
                        .unwrap_or(delta.x * 0.5);
                let change = (angle - drag.last_angle + 180.0).rem_euclid(360.0) - 180.0;
                drag.rotation_angle += change;
                drag.last_angle = angle;
                let angle = if snap || self.state.session.snap_enabled {
                    (drag.rotation_angle / 15.0).round() * 15.0
                } else {
                    drag.rotation_angle
                };
                self.state.update_modal(glam::Vec3::ZERO, angle)
            }
            TransformKind::Scale => {
                let origin = projection::project_pixel(camera, viewport, pivot).unwrap_or(start);
                let a = start - origin;
                let b = current - origin;
                let mut factor = if a.length() >= 8.0 {
                    b.dot(a) / a.length_squared()
                } else {
                    1.0 + delta.x * 0.005
                };
                if snap || self.state.session.snap_enabled {
                    factor = (factor * 10.0).round() / 10.0;
                }
                if factor.abs() < 0.001 {
                    factor = if factor < 0.0 { -0.001 } else { 0.001 };
                }
                self.state.update_modal(glam::Vec3::ZERO, factor)
            }
        };
        self.drag = Some(drag);
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
        if self.state.session.tools.active_tool == "picker" {
            return self.pick_paint_color_at(x, y);
        }
        if self.is_shape_tool() {
            return self.begin_paint_shape_at(x, y);
        }
        self.state.begin_paint_stroke();
        self.paint_dab_at(x, y);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Ferramentas de forma ancoram no press e confirmam no release.
    pub fn is_shape_tool(&self) -> bool {
        matches!(
            petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind),
            petunia_core::BrushType::Line | petunia_core::BrushType::Rectangle
        )
    }

    /// Inicia uma forma (Line/Rectangle) no pixel do canvas sob o cursor.
    pub fn begin_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        // A forma precisa de canvas para converter UV em pixel.
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            self.state
                .set_status("Shape: point at the surface to anchor the shape");
            return false;
        };
        self.shape_anchor = Some((px, py));
        self.state.set_status("Shape anchored: release to commit");
        true
    }

    /// Confirma a forma como uma única operação de undo.
    pub fn end_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        let Some((x0, y0)) = self.shape_anchor.take() else {
            return false;
        };
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((x1, y1)) = self.canvas_pixel_at(x, y) else {
            self.state
                .set_status("Shape: release point is off the surface, discarded");
            return false;
        };
        let brush = petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind);
        let color = [
            (self.state.paint_color[0] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[1] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[2] * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ];
        let strength = self.state.session.tools.paint_strength;
        self.state.checkpoint("paint shape");
        petunia_module_paint::PaintModule::commit_shape(
            &mut self.state,
            petunia_module_paint::ShapeStroke {
                x0,
                y0,
                x1,
                y1,
                brush,
                color,
                strength,
            },
        );
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        self.state
            .set_status(format!("Shape committed ({brush:?})"));
        true
    }

    /// Cancela a forma ancorada sem tocar no documento.
    pub fn cancel_paint_shape(&mut self) -> bool {
        if self.shape_anchor.take().is_none() {
            return false;
        }
        self.state.set_status("Shape cancelled");
        true
    }

    fn canvas_pixel_at(&self, x: f32, y: f32) -> Option<(u32, u32)> {
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..width).contains(&x)
            || !(0.0..height).contains(&y)
        {
            return None;
        }
        let ndc_x = x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - y / height * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        let isolate = self.state.session.tools.paint_isolate_selection;
        let uv = petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)?;
        petunia_module_paint::PaintModule::uv_to_px(&self.state, uv)
    }

    fn pick_paint_color_at(&mut self, x: f32, y: f32) -> bool {
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            return false;
        };
        let Some(color) = self
            .state
            .project
            .active()
            .and_then(|asset| asset.texture.as_ref())
            .and_then(|texture| texture.get(px, py))
        else {
            return false;
        };
        let color = [color[0], color[1], color[2]].map(|channel| channel as f32 / 255.0);
        self.state.paint_color = color;
        self.state.session.tools.paint_color = color;
        self.state.mark_dirty();
        self.state.set_status("Color sampled from texture");
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
    pub fn end_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        if self.shape_anchor.is_some() {
            return self.end_paint_shape_at(x, y);
        }
        if self.paint_last.take().is_none() {
            return false;
        }
        self.state.finish_paint_stroke(false);
        true
    }

    pub fn cancel_paint_stroke(&mut self) -> bool {
        if self.cancel_paint_shape() {
            return true;
        }
        if self.paint_2d_last.take().is_some() {
            self.state.finish_paint_stroke(true);
            self.state.mark_dirty();
            return true;
        }
        if self.paint_last.take().is_none() {
            return false;
        }
        self.state.finish_paint_stroke(true);
        true
    }

    /// Tique periódico para acúmulo contínuo de tinta da ferramenta Airbrush (P3D-056).
    pub fn airbrush_tick(&mut self) -> bool {
        let is_airbrush = self.state.session.tools.active_tool == "airbrush"
            || petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind)
                == petunia_core::BrushType::Airbrush;
        if !is_airbrush {
            return false;
        }
        let mut changed = false;
        if let Some([x, y]) = self.paint_last {
            self.paint_dab_at(x, y);
            changed = true;
        }
        if let Some((px, py)) = self.paint_2d_last {
            let settings = self.state.brush_settings();
            petunia_module_paint::PaintModule::canvas_brush_with_settings(
                &mut self.state,
                px,
                py,
                settings,
            );
            changed = true;
        }
        if changed {
            self.state.mark_dirty();
        }
        changed
    }

    /// Processa interação interativa de desenho no canvas 2D de textura (P3D-057).
    /// phase: 0 = Down, 1 = Move, 2 = Up, outros = Cancel
    pub fn paint_2d_stroke(&mut self, norm_x: f32, norm_y: f32, phase: i32) -> bool {
        if !norm_x.is_finite() || !norm_y.is_finite() {
            return false;
        }
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let (width, height) = match self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|a| a.texture.as_ref())
        {
            Some(t) => (t.w, t.h),
            None => (256, 256),
        };
        let px = ((norm_x * width as f32).floor() as i32).clamp(0, width as i32 - 1) as u32;
        let py = ((norm_y * height as f32).floor() as i32).clamp(0, height as i32 - 1) as u32;

        match phase {
            0 => {
                let tool = self.state.session.tools.active_tool.clone();
                if tool == "picker" {
                    if let Some(color) = self
                        .state
                        .project
                        .assets
                        .get(self.state.project.active)
                        .and_then(|a| a.texture.as_ref())
                        .and_then(|t| t.get(px, py))
                    {
                        let c = [
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                        ];
                        self.state.paint_color = c;
                        self.state.session.tools.paint_color = c;
                        self.state.mark_dirty();
                        self.state.set_status("Color sampled from canvas");
                    }
                    return true;
                }
                if tool == "fill" {
                    let scope = self.state.session.tools.fill_scope;
                    petunia_module_paint::PaintModule::canvas_fill_scoped(
                        &mut self.state,
                        None,
                        Some((px, py)),
                        scope,
                    );
                    self.state.set_status(format!("Filled canvas ({scope:?})"));
                    self.state.mark_dirty();
                    return true;
                }
                self.state.begin_paint_stroke();
                let settings = self.state.brush_settings();
                petunia_module_paint::PaintModule::canvas_brush_with_settings(
                    &mut self.state,
                    px,
                    py,
                    settings,
                );
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            1 => {
                let Some((last_x, last_y)) = self.paint_2d_last else {
                    return false;
                };
                let dx = px as f32 - last_x as f32;
                let dy = py as f32 - last_y as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                let steps = (dist / 1.0).ceil().max(1.0) as usize;
                let settings = self.state.brush_settings();
                for step in 1..=steps {
                    let t = step as f32 / steps as f32;
                    let ix =
                        ((last_x as f32 + dx * t).round() as i32).clamp(0, width as i32 - 1) as u32;
                    let iy = ((last_y as f32 + dy * t).round() as i32).clamp(0, height as i32 - 1)
                        as u32;
                    petunia_module_paint::PaintModule::canvas_brush_with_settings(
                        &mut self.state,
                        ix,
                        iy,
                        settings,
                    );
                }
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            2 => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                self.state.finish_paint_stroke(false);
                self.state.mark_dirty();
                true
            }
            _ => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                self.state.finish_paint_stroke(true);
                self.state.mark_dirty();
                true
            }
        }
    }

    pub fn project_from_reference(&mut self) -> bool {
        match self.execute_core_command("uv.project_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn bake_reference(&mut self) -> bool {
        match self.execute_core_command("paint.bake_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Define o modo de sombreamento da viewport pelo id estável.
    ///
    /// Cada modo corresponde a um pipeline real: Wireframe não preenche,
    /// Solid usa o estúdio da viewport, MaterialPreview amostra a textura e
    /// Rendered usa a luz da cena.
    pub fn set_shading_mode(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::Shading::from_id(id) else {
            return false;
        };
        if self.state.shading == mode {
            return false;
        }
        self.state.shading = mode;
        // Material/Rendered dependem de amostrar o material.
        if mode.samples_material() {
            self.state.session.textured = true;
        }
        self.state.set_status(format!("Shading: {}", mode.id()));
        self.state.mark_dirty();
        true
    }

    /// Opacidade da geometria em X-Ray.
    pub fn set_xray_opacity(&mut self, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let clamped = opacity.clamp(0.1, 0.9);
        if (clamped - self.state.session.xray_opacity).abs() < f32::EPSILON {
            return false;
        }
        self.state.session.xray_opacity = clamped;
        self.state.mark_dirty();
        true
    }

    /// Define o modo de confirmação das ferramentas paramétricas.
    pub fn set_tool_activation(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::ToolActivation::from_id(id) else {
            return false;
        };
        if self.state.session.tools.tool_activation == mode {
            return false;
        }
        self.state.session.tools.tool_activation = mode;
        self.state.set_status(match mode {
            petunia_core::ToolActivation::Drag => "Tools confirm on pointer release (click + drag)",
            petunia_core::ToolActivation::Instant => {
                "Tools follow the pointer; click or Enter confirms, Esc cancels"
            }
        });
        self.state.mark_dirty();
        true
    }

    /// Preferência de input da aplicação, sem criar undo nem sujar o modelo.
    pub fn set_invert_vertical_drag(&mut self, invert: bool) -> bool {
        if self.state.ui.invert_vertical_drag == invert {
            return false;
        }
        self.state.ui.invert_vertical_drag = invert;
        let status_id = if invert {
            petunia_config::text_id::UI_VERTICAL_DRAG_INVERTED
        } else {
            petunia_config::text_id::UI_VERTICAL_DRAG_NORMAL
        };
        self.state.set_status(self.state.t_id(status_id));
        true
    }

    pub fn set_asset_query(&mut self, query: &str) -> bool {
        if self.asset_query == query {
            return false;
        }
        self.asset_query = query.to_string();
        true
    }

    pub fn set_parts_query(&mut self, query: &str) -> bool {
        if self.parts_query == query {
            return false;
        }
        self.parts_query = query.to_string();
        true
    }

    pub fn set_parts_selected_only(&mut self, enabled: bool) -> bool {
        if self.parts_selected_only == enabled {
            return false;
        }
        self.parts_selected_only = enabled;
        true
    }

    pub fn set_parts_sort_by_name(&mut self, enabled: bool) -> bool {
        if self.parts_sort_by_name == enabled {
            return false;
        }
        self.parts_sort_by_name = enabled;
        true
    }

    pub fn set_parts_row_height(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(28.0, 44.0);
        if (self.parts_row_height - size).abs() < f32::EPSILON {
            return false;
        }
        self.parts_row_height = size;
        true
    }

    pub fn set_asset_sort_by_name(&mut self, sort_by_name: bool) -> bool {
        if self.asset_sort_by_name == sort_by_name {
            return false;
        }
        self.asset_sort_by_name = sort_by_name;
        true
    }

    pub fn set_asset_thumbnail_size(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(48.0, 128.0);
        if (self.state.ui.asset_thumbnail_size - size).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.asset_thumbnail_size = size;
        true
    }

    pub fn set_selection_color_hex(&mut self, value: &str) -> bool {
        let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_INVALID);
            self.state.set_status(message);
            return false;
        }
        let mut rgb = [0u8; 3];
        for (index, channel) in rgb.iter_mut().enumerate() {
            let Some(piece) = hex.get(index * 2..index * 2 + 2) else {
                return false;
            };
            let Ok(parsed) = u8::from_str_radix(piece, 16) else {
                return false;
            };
            *channel = parsed;
        }
        if !selection_color_has_contrast(rgb) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_LOW_CONTRAST);
            self.state.set_status(message);
            return false;
        }
        if self.state.ui.selection_rgb == rgb {
            return false;
        }
        self.state.ui.selection_rgb = rgb;
        true
    }

    pub fn set_selection_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite() {
            return false;
        }
        let thickness = thickness.clamp(1.0, 6.0);
        if (self.state.ui.selection_thickness - thickness).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.selection_thickness = thickness;
        true
    }

    /// O modo Instant está ativo?
    pub fn is_instant_tool_mode(&self) -> bool {
        self.keyboard_tool_modal_active
            || self.state.session.tools.tool_activation == petunia_core::ToolActivation::Instant
    }

    /// Alinha a câmera a um eixo a partir do tripé de navegação.
    pub fn snap_view_to_axis(&mut self, axis: &str) -> bool {
        use petunia_core::ViewPreset;
        let preset = match axis {
            "x" => ViewPreset::Right,
            "y" => ViewPreset::Top,
            "z" => ViewPreset::Front,
            _ => return false,
        };
        self.state.session.camera.set_preset(preset);
        self.state.set_status(format!("View: {}", preset.title()));
        self.state.mark_dirty();
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

    /// Define o escopo de preenchimento do pincel.
    pub fn set_fill_scope(&mut self, scope: &str) -> bool {
        let parsed = match scope {
            "ConnectedPixels" => petunia_core::FillScope::ConnectedPixels,
            "Face" => petunia_core::FillScope::Face,
            "SelectedFaces" => petunia_core::FillScope::SelectedFaces,
            "UvIsland" => petunia_core::FillScope::UvIsland,
            "Object" => petunia_core::FillScope::Object,
            _ => return false,
        };
        self.state.session.tools.fill_scope = parsed;
        self.state.set_status(format!("Fill scope: {scope}"));
        true
    }

    /// Define o modo de projeção do pincel.
    pub fn set_brush_projection(&mut self, projection: &str) -> bool {
        let parsed = match projection {
            "Surface" => petunia_core::BrushProjectionMode::Surface,
            "ScreenSpace" => petunia_core::BrushProjectionMode::ScreenSpace,
            _ => return false,
        };
        self.state.session.tools.brush_projection = parsed;
        self.state.set_status(format!("Projection: {projection}"));
        true
    }

    /// Define a trava de pincel.
    pub fn set_brush_lock(&mut self, lock: &str) -> bool {
        let parsed = match lock {
            "None" => petunia_core::BrushLock::None,
            "FirstObject" => petunia_core::BrushLock::FirstObject,
            "FirstFace" => petunia_core::BrushLock::FirstFace,
            "SelectedFaces" => petunia_core::BrushLock::SelectedFaces,
            _ => return false,
        };
        self.state.session.tools.brush_lock = parsed;
        self.state.session.tools.paint_lock_face = None;
        self.state.set_status(format!("Brush lock: {lock}"));
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
            uv_selected_count: self.state.session.uv_selected.len(),
            truncated: face_count > MAX_FACES,
        }
    }

    /// Clique no editor UV 2D: seleciona a face cuja ilha contém o ponto.
    ///
    /// `u`/`v` chegam normalizados em 0..1 com origem embaixo, que é a
    /// convenção do domínio; o editor desenha com origem em cima e converte
    /// antes de chamar.
    pub fn uv_editor_click(&mut self, u: f32, v: f32, extend: bool) -> bool {
        if !u.is_finite() || !v.is_finite() {
            return false;
        }
        let Some(face) = petunia_module_uv::UvModule::uv_hit(&self.state, u, v) else {
            if !extend {
                self.state.session.uv_selected.clear();
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    for current in &mut mesh.faces {
                        current.selected = false;
                    }
                }
                self.state.sync_selection();
            }
            self.state.set_status("UV: no face under the cursor");
            return false;
        };
        if extend {
            // Com Shift a seleção acumula e alterna; sem Shift ela é substituída,
            // que é o comportamento previsível de um clique simples.
            if !self.state.session.uv_selected.insert(face) {
                self.state.session.uv_selected.remove(&face);
            }
        } else {
            self.state.session.uv_selected.clear();
            self.state.session.uv_selected.insert(face);
        }
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            if !extend {
                for current in &mut mesh.faces {
                    current.selected = false;
                }
            }
            if let Some(target) = mesh.faces.get_mut(face) {
                target.selected = self.state.session.uv_selected.contains(&face);
            }
        }
        self.state.sync_selection();
        self.state.mark_dirty();
        self.state.set_status(format!(
            "UV: face {face} selected ({} total)",
            self.state.session.uv_selected.len()
        ));
        true
    }

    /// Move as UVs selecionadas (ou todas, quando nada está marcado).
    pub fn uv_move_selected(&mut self, du: f32, dv: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !du.is_finite() || !dv.is_finite() {
            return false;
        }
        if du == 0.0 && dv == 0.0 {
            return false;
        }
        self.state.checkpoint("move uv");
        petunia_module_uv::UvModule::move_selected(&mut self.state, du, dv);
        self.state.emit_mesh_changed();
        self.state
            .set_status(format!("UV moved by ({du:.3}, {dv:.3})"));
        true
    }

    /// Escala as UVs selecionadas em torno do centroide.
    pub fn uv_scale_selected(&mut self, factor: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !factor.is_finite() || factor <= 0.0 {
            return false;
        }
        self.state.checkpoint("scale uv");
        petunia_module_uv::UvModule::scale_selected(&mut self.state, factor);
        self.state.emit_mesh_changed();
        self.state.set_status(format!("UV scaled ×{factor:.3}"));
        true
    }

    /// Rotaciona as UVs selecionadas em torno do centroide.
    pub fn uv_rotate_selected(&mut self, degrees: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !degrees.is_finite() || degrees == 0.0 {
            return false;
        }
        self.state.checkpoint("rotate uv");
        petunia_module_uv::UvModule::rotate_selected(&mut self.state, degrees.to_radians());
        self.state.emit_mesh_changed();
        self.state.set_status(format!("UV rotated {degrees:.1}°"));
        true
    }

    /// Marca ou desmarca como costura as arestas selecionadas ou da face UV selecionada.
    pub fn toggle_selected_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        let selected_edges: Vec<(u32, u32)> = mesh.selected_edges.iter().copied().collect();
        if !selected_edges.is_empty() {
            self.state.checkpoint("toggle uv seams");
            let Some(mesh) = self.state.project.active_mesh_mut() else {
                return false;
            };
            for (a, b) in selected_edges {
                mesh.toggle_seam(a, b);
            }
            let count = self
                .state
                .project
                .active_mesh()
                .map(|mesh| mesh.uv_seams.len())
                .unwrap_or(0);
            self.state.set_status(format!(
                "UV seams on selected edges toggled ({count} total)"
            ));
            self.state.emit_mesh_changed();
            self.state.mark_dirty();
            return true;
        }

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
        let (w, h) = self.paint_canvas_dimensions().unwrap_or((256, 256));
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

    pub fn add_decal_layer(&mut self) -> bool {
        self.mutate_paint_stack("add decal layer", |stack| {
            let decal_img = petunia_project::Canvas::new(64, 64, [255, 200, 50, 255]);
            let decal = petunia_project::paint_layers::DecalLayer::new(
                decal_img,
                [0.5, 0.5],
                [0.25, 0.25],
                0.0,
            );
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new_decal(
                format!("Decal {}", stack.layers.len() + 1),
                decal,
            ));
            true
        })
    }

    pub fn toggle_face_orientation(&mut self) -> bool {
        self.state.session.show_face_orientation = !self.state.session.show_face_orientation;
        let enabled = self.state.session.show_face_orientation;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Orientação de faces (azul/vermelho) ativada"
        } else {
            "Orientação de faces desativada"
        });
        enabled
    }

    pub fn toggle_uv_checker(&mut self) -> bool {
        self.state.session.show_uv_checker = !self.state.session.show_uv_checker;
        let enabled = self.state.session.show_uv_checker;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "UV Checkerboard ativado"
        } else {
            "UV Checkerboard desativado"
        });
        enabled
    }

    pub fn toggle_proportional_editing(&mut self) -> bool {
        self.state.session.proportional_editing = !self.state.session.proportional_editing;
        let enabled = self.state.session.proportional_editing;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Edição proporcional ativada"
        } else {
            "Edição proporcional desativada"
        });
        enabled
    }

    pub fn set_proportional_radius(&mut self, radius: f32) -> bool {
        if !radius.is_finite() {
            return false;
        }
        let r = radius.clamp(0.01, 100.0);
        self.state.session.proportional_settings.radius = r;
        self.state.render.mark_dirty();
        true
    }

    pub fn adjust_proportional_radius(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let current = self.state.session.proportional_settings.radius;
        let new_radius = (current + delta).clamp(0.05, 100.0);
        self.set_proportional_radius(new_radius);
        self.state
            .set_status(format!("Raio proporcional: {:.2}", new_radius));
        true
    }

    pub fn set_proportional_falloff(&mut self, falloff_str: &str) -> bool {
        use petunia_core::ProportionalFalloff;
        let falloff = match falloff_str.to_lowercase().as_str() {
            "smooth" => ProportionalFalloff::Smooth,
            "linear" => ProportionalFalloff::Linear,
            "sphere" => ProportionalFalloff::Sphere,
            "sharp" => ProportionalFalloff::Sharp,
            "constant" => ProportionalFalloff::Constant,
            _ => ProportionalFalloff::Smooth,
        };
        self.state.session.proportional_settings.falloff = falloff;
        self.state.render.mark_dirty();
        true
    }

    pub fn toggle_snap_enabled(&mut self) -> bool {
        self.state.session.snap_enabled = !self.state.session.snap_enabled;
        self.state.session.snap_settings.enabled = self.state.session.snap_enabled;
        let enabled = self.state.session.snap_enabled;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Snap magnético ativado"
        } else {
            "Snap magnético desativado"
        });
        enabled
    }

    pub fn set_snap_target(&mut self, target_str: &str) -> bool {
        use petunia_core::SnapTarget;
        let target = match target_str.to_lowercase().as_str() {
            "grid" => SnapTarget::Grid,
            "increment" => SnapTarget::Increment,
            "vertex" | "point" => SnapTarget::Vertex,
            "edge" => SnapTarget::Edge,
            "face" => SnapTarget::Face,
            _ => SnapTarget::Grid,
        };
        self.state.session.snap_settings.target = target;
        self.state.render.mark_dirty();
        self.state
            .set_status(format!("Snap target: {}", target.label()));
        true
    }

    pub fn add_profile_rectangle(&mut self, width: f32, height: f32) -> bool {
        petunia_module_model::profile_set_rectangle(&mut self.state, width, height);
        self.state.render.mark_dirty();
        self.state.set_status(format!(
            "Perfil retangular ({width:.1} x {height:.1}) criado"
        ));
        true
    }

    pub fn add_profile_circle(&mut self, radius: f32, segments: usize) -> bool {
        petunia_module_model::profile_set_circle(&mut self.state, radius, segments);
        self.state.render.mark_dirty();
        self.state.set_status(format!(
            "Perfil circular (raio {radius:.1}, {segments} seg) criado"
        ));
        true
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
        self.begin_loop_cut_from_ring(ring, mesh, 1)
    }

    fn begin_loop_cut_from_ring(
        &mut self,
        ring: petunia_core::LoopRing,
        mesh: petunia_core::Mesh,
        cuts: usize,
    ) -> bool {
        self.loop_cut = Some(LoopCutSessionState {
            ring,
            cuts: cuts.clamp(1, 32),
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

    /// Campo numérico do Slide: não altera a quantidade de cortes.
    pub fn set_loop_cut_slide(&mut self, slide: f32) -> bool {
        if !slide.is_finite() || !(-1.0..=1.0).contains(&slide) {
            self.state
                .set_status("Loop Cut: slide must be between -1 and 1");
            return false;
        }
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        if (session.slide - slide).abs() <= f32::EPSILON {
            return false;
        }
        let previous = session.slide;
        session.slide = slide;
        if self.apply_loop_cut_preview() {
            true
        } else {
            if let Some(session) = self.loop_cut.as_mut() {
                session.slide = previous;
            }
            false
        }
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

    pub fn scroll_loop_cut_count(&mut self, delta_y: f32) -> bool {
        if !delta_y.is_finite() || delta_y.abs() < f32::EPSILON {
            return false;
        }
        let delta = if delta_y > 0.0 { 1 } else { -1 };
        if self.loop_cut.is_some() {
            let current = self.loop_cut.as_ref().map_or(1, |session| session.cuts);
            return self.set_loop_cut_count((current as i32 + delta).clamp(1, 32) as usize);
        }
        self.adjust_loop_cut_hover_count(delta)
    }

    pub fn adjust_loop_cut_count_from_input(&mut self, cuts: usize) -> bool {
        if self.loop_cut.is_some() {
            return self.set_loop_cut_count(cuts);
        }
        let next = cuts.clamp(1, 32);
        let delta = next as i32 - self.loop_cut_hover_cuts as i32;
        self.adjust_loop_cut_hover_count(delta)
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

    /// Abre o plano de corte (Slice) ancorado no ponto pressionado.
    pub fn begin_slice(&mut self, x: f32, y: f32) -> bool {
        let Some(mesh) = self.state.project.active_mesh().cloned() else {
            self.state.set_status("Slice: no active mesh");
            return false;
        };
        self.state.session.tools.cut_session = Some(petunia_core::CutSession::new(mesh));
        self.state.session.tools.active_tool = "slice".to_string();
        self.slice_anchor = Some([x, y]);
        self.state
            .set_status("Slice: drag to orient the plane, release to cut");
        true
    }

    /// Atualiza a pré-visualização do plano de corte.
    pub fn update_slice(&mut self, x: f32, y: f32) -> bool {
        let Some(anchor) = self.slice_anchor else {
            return false;
        };
        let viewport = petunia_core::LogicalRect::from_min_max(
            [0.0, 0.0],
            [self.viewport_size[0], self.viewport_size[1]],
        );
        let Some(session) = self.state.session.tools.cut_session.as_ref() else {
            return false;
        };
        let Some(sliced) =
            session.compute_slice(&self.state.session.camera, anchor, [x, y], viewport)
        else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = sliced;
        }
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        true
    }

    /// Confirma o corte como uma única operação de undo.
    pub fn commit_slice(&mut self) -> bool {
        // A faca compartilha `cut_session`, então o Slice só age quando é ele que
        // está armado — senão um release cancelaria o corte da faca.
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            return false;
        };
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        // A pré-visualização já está na malha: restaurar o snapshot, capturar e
        // reaplicar o corte garante que o undo volte ao estado anterior.
        let Some(current) = self.state.project.active_mesh().cloned() else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source.clone();
        }
        self.state.checkpoint("slice");
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = current;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Slice applied");
        true
    }

    /// Abandona o plano de corte restaurando a malha original.
    pub fn cancel_slice(&mut self) -> bool {
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            return false;
        };
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Slice cancelled");
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
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return false;
        }
        let petunia_core::HoverTarget::Edge(a, b) =
            self.pick_target_for_domain(SelectionDomain::Edge, normalized_x, normalized_y)
        else {
            self.state.set_status("Cut: point at a visible edge");
            return false;
        };
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let vp = self.state.session.camera.view_proj();
        let va = mesh.verts[a as usize].vec();
        let vb = mesh.verts[b as usize].vec();
        let ca = vp * va.extend(1.0);
        let cb = vp * vb.extend(1.0);
        let screen = |p: glam::Vec4| {
            [
                (p.x / p.w * 0.5 + 0.5) * self.viewport_size[0],
                (0.5 - p.y / p.w * 0.5) * self.viewport_size[1],
            ]
        };
        let pa = screen(ca);
        let pb = screen(cb);
        let mouse = [
            normalized_x * self.viewport_size[0],
            normalized_y * self.viewport_size[1],
        ];
        let delta = [pb[0] - pa[0], pb[1] - pa[1]];
        let length = delta[0] * delta[0] + delta[1] * delta[1];
        if length <= 1.0e-6 {
            return false;
        }
        let t = (((mouse[0] - pa[0]) * delta[0] + (mouse[1] - pa[1]) * delta[1]) / length)
            .clamp(0.0, 1.0);
        let t = (t / cb.w) / ((1.0 - t) / ca.w + t / cb.w);
        let edge = (a, b);
        let position = va.lerp(vb, t);
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
                session.segments += 1;
                session.edge_start = None;
                session.anchor = None;
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = cut;
                }
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                self.state.sync_selection();
                self.state.emit_mesh_changed();
                self.state
                    .set_status("Cut preview: choose another segment, Enter applies, Esc restores");
                true
            }
            Err(error) => {
                self.state.set_status(format!("Cut: {error}"));
                false
            }
        }
    }

    /// Commit all knife segments as one undo record.
    pub fn commit_knife(&mut self) -> bool {
        if self.state.session.tools.active_tool != "cut" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".into();
        if session.segments > 0 {
            let Some(result) = self.state.project.active_mesh().cloned() else {
                return false;
            };
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                *mesh = session.source;
            }
            self.state.checkpoint("cut segments");
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                *mesh = result;
            }
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Cut applied");
        true
    }

    pub fn cancel_knife(&mut self) -> bool {
        let Some(session) = self.state.session.tools.cut_session.take() else {
            return false;
        };
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            *mesh = session.source;
        }
        self.state.session.tools.active_tool = "select".into();
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Cut cancelled");
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

    /// Define o operando B das operações booleanas.
    pub fn set_boolean_operand(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.project.assets.iter().any(|a| a.id == asset) {
            return false;
        }
        self.state.session.tools.boolean_operand = Some(asset);
        self.state
            .set_status("Boolean operand set: Fuse, Cut or Intersect now applies");
        true
    }

    /// Liga/desliga o modificador **Keep Parts**.
    pub fn set_boolean_keep_parts(&mut self, keep: bool) -> bool {
        if self.state.session.tools.boolean_keep_parts == keep {
            return false;
        }
        self.state.session.tools.boolean_keep_parts = keep;
        self.state.set_status(if keep {
            "Keep Parts on: the operand stays in the scene"
        } else {
            "Keep Parts off: the operand is consumed"
        });
        true
    }

    /// **Join** pelo id canônico.
    pub fn join_operand(&mut self) -> bool {
        match self.execute_core_command("model.join") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn clear_boolean_operand(&mut self) -> bool {
        if self.state.session.tools.boolean_operand.take().is_none() {
            return false;
        }
        self.state.set_status("Boolean operand cleared");
        true
    }

    /// Executa Fuse/Cut/Intersect pelo id canônico do comando.
    pub fn boolean_op(&mut self, id: &str) -> bool {
        match self.execute_core_command(id) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
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
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset,
            viewport: false,
        });
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

    /// Menu da viewport (botão direito no 3D): só verbetes de seleção, sem
    /// alvo de asset. Id de overlay próprio para o Escape LIFO fechar o menu
    /// certo quando Outliner e viewport competem.
    pub fn open_viewport_context_menu(&mut self, x: f32, y: f32) -> bool {
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset: uuid::Uuid::nil(),
            viewport: true,
        });
        self.overlays.push(OverlayEntry {
            id: OverlayId::ContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    /// Triagem do botão direito na viewport (Blender): com sessão ativa o
    /// clique cancela (modal, arrasto, knife...); sem sessão abre o menu.
    /// Retorna verdadeiro quando cancelou algo.
    pub fn viewport_context_triage(&mut self, x: f32, y: f32) -> bool {
        if self.cancel_rename() | self.cancel_active_operation() {
            return true;
        }
        self.open_viewport_context_menu(x, y);
        false
    }

    pub fn close_context_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::OutlinerContextMenu);
        self.overlays.remove(OverlayId::ContextMenu);
        self.context_menu.take().is_some()
    }

    /// Executa uma ação do menu de contexto sobre o alvo apontado.
    pub fn context_menu_action(&mut self, action: &str) -> bool {
        let Some(menu) = self.context_menu else {
            return false;
        };
        // Modo viewport: só verbetes de seleção, sem alvo de asset.
        if menu.viewport {
            self.close_context_menu();
            return match action {
                "select_all" => {
                    self.apply(UiIntent::SelectAll);
                    true
                }
                "clear_selection" => {
                    self.apply(UiIntent::ClearSelection);
                    true
                }
                "invert_selection" => {
                    self.apply(UiIntent::InvertSelection);
                    true
                }
                "frame" => {
                    let _ = self.state.dispatch_command("view.frame_selection");
                    true
                }
                "delete" => {
                    self.apply(UiIntent::DeleteActiveAsset);
                    true
                }
                _ => false,
            };
        }
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
            "boolean_operand" => self.set_boolean_operand(&id),
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
            self.cancel_active_operation();
            self.state.select_object(Some(index), false);
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
                self.state.session.tools.active_tool = match kind {
                    ToolModalKind::ScaleSelection => "scale",
                    ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual => "extrude",
                    ToolModalKind::Inset => "inset",
                    ToolModalKind::Bevel => "bevel",
                    ToolModalKind::PushPull => "push_pull",
                }
                .to_string();
                self.keyboard_tool_modal_active = false;
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
        let step = kind.step();
        let delta_y = delta_y
            * if fine { 0.1 } else { 1.0 }
            * if self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
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
        self.keyboard_tool_modal_active = false;
        self.state.commit_modal();
        self.state.mark_dirty();
        true
    }

    pub fn cancel_tool_modal(&mut self) -> bool {
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.keyboard_tool_modal_active = false;
        self.state.cancel_modal();
        self.state.mark_dirty();
        true
    }

    fn paint_dab_at(&mut self, x: f32, y: f32) {
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..width).contains(&x)
            || !(0.0..height).contains(&y)
        {
            return;
        }
        let ndc_x = x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - y / height * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let Some((face, hit)) = pick_face_hit(&self.state, origin, direction) else {
            return;
        };
        let tool = self.state.session.tools.active_tool.clone();
        if tool == "picker" {
            return;
        }
        if tool == "fill" {
            let scope = self.state.session.tools.fill_scope;
            let isolate = self.state.session.tools.paint_isolate_selection;
            let seed =
                petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)
                    .and_then(|uv| petunia_module_paint::PaintModule::uv_to_px(&self.state, uv));
            petunia_module_paint::PaintModule::canvas_fill_scoped(
                &mut self.state,
                Some(face),
                seed,
                scope,
            );
            self.state.set_status(format!("Filled ({scope:?})"));
            return;
        }
        let brush = match tool.as_str() {
            "eraser" => petunia_core::BrushType::Eraser,
            "airbrush" => petunia_core::BrushType::Airbrush,
            "pixel" => petunia_core::BrushType::Pixel,
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
        self.select_viewport_ext(normalized_x, normalized_y, extend, false);
    }

    pub fn select_viewport_ext(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        extend: bool,
        loop_select: bool,
    ) {
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            petunia_module_model::draw_profile::profile_add_point(
                &mut self.state,
                normalized_x,
                normalized_y,
            );
            return;
        }
        if self.instant_transform && self.state.session.tools.modal.is_some() {
            self.end_viewport_transform();
            return;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let _ = self.place_loop_cut_hover();
            return;
        }
        // No modo Instant um clique confirma a sessão paramétrica em vez de
        // trocar a seleção — é o equivalente ao Enter com o mouse.
        if self.is_instant_tool_mode() && self.tool_modal.is_some() {
            self.commit_tool_modal();
            return;
        }
        // A faca consome o clique antes da seleção: com uma sessão de corte
        // aberta, clicar é escolher ponto de aresta, não selecionar.
        if self.state.session.tools.cut_session.is_some() {
            self.knife_click(normalized_x, normalized_y);
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
                    "airbrush" => petunia_core::BrushType::Airbrush,
                    "pixel" => petunia_core::BrushType::Pixel,
                    _ => petunia_core::BrushType::Soft,
                };
                if brush == petunia_core::BrushType::Eyedropper {
                    self.pick_paint_color_at(
                        normalized_x * self.viewport_size[0],
                        normalized_y * self.viewport_size[1],
                    );
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

        use petunia_core::HoverTarget as Target;
        // O clique confirma a preselection: o alvo sob o cursor vira o hover
        // corrente para o destaque e o rótulo aparecerem de imediato, sem
        // esperar o próximo mousemove. Erro limpa em vez de congelar.
        let hit = self.pick_viewport_target(normalized_x, normalized_y);
        match hit {
            Target::Object(index) => {
                self.state.select_object(Some(index), extend);
                let name = self.state.project.assets[index].name.clone();
                self.state.set_status(format!("Selected '{name}'"));
            }
            Target::Vertex(index) => {
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if !extend {
                        mesh.deselect_all();
                    }
                    if let Some(vertex) = mesh.verts.get_mut(index as usize) {
                        vertex.selected = !extend || !vertex.selected;
                    }
                }
                self.state.set_status(format!("Point {index} selected"));
            }
            Target::Edge(a, b) => {
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if loop_select {
                        let count = mesh.select_edge_loop((a, b), extend);
                        self.state
                            .set_status(format!("Selected edge loop ({count} edges)"));
                    } else {
                        if !extend {
                            mesh.deselect_all();
                        }
                        if extend && mesh.selected_edges.contains(&(a, b)) {
                            mesh.selected_edges.remove(&(a, b));
                        } else {
                            mesh.selected_edges.insert((a, b));
                        }
                        // Operações de malha usam os vértices das arestas selecionadas.
                        // Recalcular impede que um Shift-click para desmarcar deixe
                        // vértices invisivelmente selecionados.
                        for vertex in &mut mesh.verts {
                            vertex.selected = false;
                        }
                        for &(start, end) in &mesh.selected_edges {
                            if let Some(vertex) = mesh.verts.get_mut(start as usize) {
                                vertex.selected = true;
                            }
                            if let Some(vertex) = mesh.verts.get_mut(end as usize) {
                                vertex.selected = true;
                            }
                        }
                        self.state.set_status(format!("Edge {a}-{b} selected"));
                    }
                }
            }
            Target::Face(face) => {
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if !extend {
                        mesh.deselect_all();
                    }
                    if let Some(current) = mesh.faces.get_mut(face) {
                        current.selected = !extend || !current.selected;
                    }
                    mesh.sync_vert_selection_from_faces();
                }
                self.state.set_status(format!("Face {face} selected"));
            }
            Target::None => {
                if self.state.selection_domain() == SelectionDomain::Object {
                    self.state.select_object(None, extend);
                }
                if !extend
                    && self.state.selection_domain().is_component()
                    && let Some(mesh) = self.state.project.active_mesh_mut()
                {
                    mesh.deselect_all();
                }
                self.state.set_status("Nothing under the cursor");
            }
        }
        self.state.session.tools.hover = hit;
        self.state.sync_selection();
        self.state.mark_dirty();
        self.reset_transform_fields();
    }

    fn cancel_active_operation(&mut self) -> bool {
        let mut cancelled = self.cancel_paint_stroke();
        cancelled |= self.cancel_tool_modal();
        cancelled |= self.cancel_loop_cut();
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            cancelled = true;
        }
        cancelled |= self.cancel_slice();
        cancelled |= self.cancel_knife();
        cancelled |= self.cancel_transform();
        self.drag = None;
        self.gizmo_drag = None;
        self.modal_text.clear();
        self.instant_transform = false;
        cancelled
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
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            self.state.mark_dirty();
            self.state.set_status("Loop Cut cancelled");
            return true;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.state.profile.clear();
            self.state.session.tools.active_tool = "select".to_string();
            self.state.mark_dirty();
            self.state.set_status("Profile cancelled");
            return true;
        }
        if self.cancel_slice() {
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
            CommandId::SelectLasso => self.apply(UiIntent::SetActiveTool("lasso_select".into())),
            CommandId::TransformCombined => self.apply(UiIntent::SetActiveTool("transform".into())),
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
            CommandId::ToggleWireOverlay => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wire_overlay") {
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
            CommandId::ToggleFaceOrientation => self.apply(UiIntent::ToggleFaceOrientation),
            CommandId::ToggleUvChecker => self.apply(UiIntent::ToggleUvChecker),
            CommandId::ToggleProportionalEditing => self.apply(UiIntent::ToggleProportionalEditing),
            CommandId::ToggleSnap => self.apply(UiIntent::ToggleSnapEnabled),
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
            self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
            return Ok(());
        }
        match id {
            "uv.unwrap" => self.state.dispatch_command("uv.unwrap_auto"),
            "uv.pack_islands" => self.state.dispatch_command("uv.pack_islands"),
            "model.frame_selection" => self.state.dispatch_command("view.frame_selection"),
            other => self.state.dispatch_command(other),
        }
    }

    fn execute_shortcut_tool(&mut self, id: &str) {
        let was_active = self.tool_modal.is_some();
        if self.execute_core_command(id).is_ok() && !was_active && self.tool_modal.is_some() {
            self.keyboard_tool_modal_active = true;
        }
    }

    pub fn route_shortcut(&mut self, text: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        if text == "Enter" && !ctrl && !alt {
            return self.confirm_active_operation();
        }
        if self.state.session.tools.modal.is_some() && !ctrl && !alt {
            if let Some(axis) = ["x", "y", "z"]
                .iter()
                .position(|axis| text.eq_ignore_ascii_case(axis))
            {
                let requested = if shift {
                    petunia_core::ModalConstraint::Plane(axis)
                } else {
                    petunia_core::ModalConstraint::Axis(axis)
                };
                let current = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|op| op.constraint);
                let constraint = if current == Some(requested) {
                    petunia_core::ModalConstraint::Free
                } else {
                    requested
                };
                let _ = self.state.set_modal_constraint(constraint);
                if let Some(drag) = self.drag.as_mut() {
                    drag.rotation_angle = 0.0;
                    drag.last_angle = 0.0;
                }
                if !self.modal_text.is_empty() {
                    self.preview_modal_text();
                } else {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                }
                return true;
            }
            if text == "Backspace" {
                self.modal_text.pop();
                if self.modal_text.is_empty() {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                } else {
                    self.preview_modal_text();
                }
                return true;
            }
            if text.len() == 1
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
            {
                self.modal_text.push_str(&text.replace(',', "."));
                self.preview_modal_text();
                return true;
            }
        }
        let Some(key) = input::key_code_from_slint(text) else {
            return false;
        };
        let mods = Mods2 { ctrl, shift, alt };
        let context = match self.state.workspace {
            Workspace::Model => "model",
            Workspace::Paint => "paint",
            Workspace::Uv => "uv",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => "animate",
        };
        let Some(action) = self
            .state
            .ui
            .keybinds
            .find_in_context(key, mods, context)
            .map(str::to_owned)
        else {
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
            "model.box_select" => self.apply(UiIntent::SetActiveTool("box_select".into())),
            "model.move" | "model.transform" => {
                self.begin_keyboard_transform(TransformKind::Position)
            }
            "model.rotate" => self.begin_keyboard_transform(TransformKind::Rotation),
            "model.scale" => self.begin_keyboard_transform(TransformKind::Scale),
            "model.frame_selection" => {
                let _ = self.execute_core_command("view.frame_selection");
            }
            "model.extrude" => {
                self.execute_shortcut_tool("model.extrude");
            }
            "model.inset" => {
                self.execute_shortcut_tool("model.inset");
            }
            "model.bevel" => {
                self.execute_shortcut_tool("model.bevel");
            }
            "model.delete" => self.apply(UiIntent::DeleteActiveAsset),
            "model.push_pull" => {
                self.execute_shortcut_tool("model.push_pull");
            }
            "model.knife" => {
                let _ = self.execute_core_command("model.knife");
            }
            "model.extrude_individual" => {
                self.execute_shortcut_tool("model.extrude_individual");
            }
            "model.subdivide" => {
                let _ = self.execute_core_command("model.subdivide");
            }
            "model.merge" => {
                let _ = self.execute_core_command("model.merge");
            }
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
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
            "model.slice" => {
                // A ação do keymap só arma a ferramenta; a âncora nasce no
                // pointer-down da viewport.
                self.apply(UiIntent::SetActiveTool("slice".to_string()));
            }
            "model.draw_profile" => {
                self.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
            }
            "global.save_project" => self.apply(UiIntent::SaveProject),
            "global.help" => {
                let _ = self.execute_core_command("help.documentation");
            }
            other
                if other.starts_with("view.")
                    || other.starts_with("model.")
                    || other.starts_with("uv.") =>
            {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                }
            }
            "window.command_palette" => self.apply(UiIntent::OpenCommandSearch),
            _ => return false,
        }
        true
    }

    fn begin_keyboard_transform(&mut self, kind: TransformKind) {
        let tool = match kind {
            TransformKind::Position => "move",
            TransformKind::Rotation => "rotate",
            TransformKind::Scale => "scale",
        };
        self.apply(UiIntent::SetActiveTool(tool.into()));
        let [x, y] = self.pointer_position;
        if self.begin_viewport_transform(kind, x, y) {
            self.instant_transform = true;
        }
    }

    fn preview_modal_text(&mut self) {
        let Ok(value) = numeric::parse_numeric(&self.modal_text) else {
            return;
        };
        if self.tool_modal.is_some() {
            self.set_tool_modal_value(value);
            return;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return;
        };
        let direction = match modal.constraint {
            petunia_core::ModalConstraint::Axis(index) => {
                let mut axis = glam::Vec3::ZERO;
                axis[index] = 1.0;
                axis
            }
            petunia_core::ModalConstraint::Plane(index) => {
                let mut direction = self.state.session.camera.right();
                direction[index] = 0.0;
                direction.normalize_or_zero()
            }
            petunia_core::ModalConstraint::Free => self.state.session.camera.right(),
        };
        if let Err(error) = self.state.update_modal(direction * value, value) {
            self.state.set_status(error.to_string());
        }
    }

    fn confirm_active_operation(&mut self) -> bool {
        if self.tool_modal.is_some() {
            return self.commit_tool_modal();
        }
        if self.loop_cut.is_some() {
            return self.commit_loop_cut();
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            return self.place_loop_cut_hover();
        }
        if self.slice_anchor.is_some() {
            return self.commit_slice();
        }
        if self.state.session.tools.active_tool == "cut" {
            return self.commit_knife();
        }
        if self.drag.is_some() {
            return self.end_viewport_transform();
        }
        self.commit_transform()
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
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        self.state.begin_modal(modal_kind)?;
        self.reset_transform_fields();
        Ok(())
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
        // Reject malformed text before opening a transaction.
        let value = numeric::parse_numeric(text)?;
        let kind_modal = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        if self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .is_some_and(|modal| modal.kind != kind_modal)
        {
            return Err(numeric::NumericInputError::Invalid);
        }
        let started = self.state.session.tools.modal.is_none();
        if started && let Err(error) = self.begin_transform(kind) {
            self.state.set_status(error.to_string());
            return Err(numeric::NumericInputError::Invalid);
        }
        let mut components = match kind {
            TransformKind::Position => self.position.map(|field| field.value()),
            TransformKind::Rotation => self.rotation.map(|field| field.value()),
            TransformKind::Scale => self.scale.map(|field| field.value()),
        };
        components[axis.min(2)] = value;
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(components))
        {
            self.state.set_status(error.to_string());
            if started {
                self.cancel_transform();
            }
            return Err(numeric::NumericInputError::Invalid);
        }
        let fields = match kind {
            TransformKind::Position => &mut self.position,
            TransformKind::Rotation => &mut self.rotation,
            TransformKind::Scale => &mut self.scale,
        };
        for (field, value) in fields.iter_mut().zip(components) {
            field.set_value(value);
        }
        self.state.commit_modal();
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
        vm.parts_query = self.parts_query.clone();
        vm.parts_selected_only = self.parts_selected_only;
        vm.parts_sort_by_name = self.parts_sort_by_name;
        vm.parts_row_height = self.parts_row_height;
        let parts_query = self.parts_query.trim().to_lowercase();
        vm.parts_items = vm
            .scene_items
            .iter()
            .filter(|item| {
                (parts_query.is_empty() || item.name.to_lowercase().contains(&parts_query))
                    && (!self.parts_selected_only || item.selected || item.active)
            })
            .cloned()
            .collect();
        if self.parts_sort_by_name {
            vm.parts_items.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        vm.asset_query = self.asset_query.clone();
        vm.asset_sort_by_name = self.asset_sort_by_name;
        vm.asset_thumbnail_size = self.state.ui.asset_thumbnail_size;
        let query = self.asset_query.trim().to_lowercase();
        vm.asset_items = vm
            .scene_items
            .iter()
            .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if self.asset_sort_by_name {
            vm.asset_items.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        vm.gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.selection_overlay = compute_selection_overlay(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.viewport.draws_component_guides(),
        );
        vm.add_menu_open = self.add_menu_open;
        if let Some(draft) = &self.rename_draft {
            vm.rename_active = true;
            vm.rename_value = draft.clone();
        }
        vm.shading_popover_open = self.shading_popover_open;
        vm.transform_instant_active =
            self.instant_transform && self.state.session.tools.modal.is_some();
        vm.gizmo_hover_axis = self.gizmo_hover.map_or(-1, |h| h.axis() as i32);
        vm.gizmo_active_axis = self.gizmo_drag.map_or(-1, |h| h.axis() as i32);
        vm.gizmo_constraint_axes = match self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|op| op.constraint)
        {
            Some(petunia_core::ModalConstraint::Axis(axis)) => [axis as i32, -1],
            Some(petunia_core::ModalConstraint::Plane(excluded)) => {
                [((excluded + 1) % 3) as i32, ((excluded + 2) % 3) as i32]
            }
            _ => [-1, -1],
        };
        let link_active = self.drag.is_some()
            || self.tool_modal.is_some()
            || self.state.session.tools.modal.is_some();
        vm.drag_link_commands = compute_drag_link(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.pointer_position,
            link_active,
        );
        if let Some(anchor) = self.slice_anchor {
            let dx = self.pointer_position[0] - anchor[0];
            let dy = self.pointer_position[1] - anchor[1];
            let len = dx.hypot(dy);
            if len > 2.0 {
                let dir_x = dx / len;
                let dir_y = dy / len;
                let p1 = [anchor[0] - dir_x * 2000.0, anchor[1] - dir_y * 2000.0];
                let p2 = [anchor[0] + dir_x * 2000.0, anchor[1] + dir_y * 2000.0];
                vm.drag_link_commands =
                    format!("M {:.2} {:.2} L {:.2} {:.2} ", p1[0], p1[1], p2[0], p2[1]);
            }
        }
        vm.hover_label = self.state.session.tools.hover.label();
        self.fill_operation_hud(&mut vm);
        if let Some(menu) = self.context_menu {
            vm.context_menu_open = true;
            vm.context_menu_x = menu.x;
            vm.context_menu_y = menu.y;
            if menu.viewport {
                vm.context_menu_mode = "viewport".to_string();
                vm.context_menu_title = "Viewport".to_string();
            } else if let Some(asset) = self
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
        if let Some(operand) = self.state.session.tools.boolean_operand {
            if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == operand)
            {
                vm.boolean_operand_name = asset.name.clone();
            } else {
                vm.boolean_operand_name = "missing".to_string();
            }
        }
        vm.boolean_keep_parts = self.state.session.tools.boolean_keep_parts;
        vm.boolean_ready = self.state.session.tools.boolean_operand.is_some()
            && self.state.project.active_mesh().is_some();
        if let Some(kind) = self.menu_open {
            vm.menu_open = kind.id().to_string();
        }
        vm.pivot_menu_open = self.pivot_menu_open;
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
        vm.label_vertical_tool_drag = translated(petunia_config::text_id::UI_VERTICAL_TOOL_DRAG);
        vm.label_invert_vertical_drag =
            translated(petunia_config::text_id::UI_INVERT_VERTICAL_DRAG);
        vm.label_search_assets = translated(petunia_config::text_id::UI_SEARCH_ASSETS);
        vm.label_search_parts = translated(petunia_config::text_id::UI_SEARCH_PARTS);
        vm.label_inspector = translated(petunia_config::text_id::UI_INSPECTOR);
        vm.label_expand_inspector = translated(petunia_config::text_id::UI_EXPAND_INSPECTOR);
        vm.label_collapse_inspector = translated(petunia_config::text_id::UI_COLLAPSE_INSPECTOR);
        vm.label_resize_panel_width = translated(petunia_config::text_id::UI_RESIZE_PANEL_WIDTH);
        vm.label_object_name = translated(petunia_config::text_id::UI_OBJECT_NAME);
        vm.label_object_visibility = translated(petunia_config::text_id::UI_OBJECT_VISIBILITY);
        vm.label_object_lock = translated(petunia_config::text_id::UI_OBJECT_LOCK);
        vm.label_object_no_selection = translated(petunia_config::text_id::UI_OBJECT_NO_SELECTION);
        vm.label_stats_faces = translated(petunia_config::text_id::UI_STATS_FACES);
        vm.label_stats_verts = translated(petunia_config::text_id::UI_STATS_VERTS);
        vm.label_stats_tris = translated(petunia_config::text_id::UI_STATS_TRIS);
        vm.label_stats_selection = translated(petunia_config::text_id::UI_STATS_SELECTION);
        vm.label_tool_options = translated(petunia_config::text_id::UI_TOOL_OPTIONS);
        vm.label_tool_options_expand = translated(petunia_config::text_id::UI_TOOL_OPTIONS_EXPAND);
        vm.label_tool_options_collapse =
            translated(petunia_config::text_id::UI_TOOL_OPTIONS_COLLAPSE);
        vm.label_quick_actions = translated(petunia_config::text_id::UI_QUICK_ACTIONS);
        vm.label_quick_action_customize =
            translated(petunia_config::text_id::UI_QUICK_ACTION_CUSTOMIZE);
        vm.label_quick_action_add = translated(petunia_config::text_id::UI_QUICK_ACTION_ADD);
        vm.label_quick_action_remove = translated(petunia_config::text_id::UI_QUICK_ACTION_REMOVE);
        vm.label_quick_action_reset = translated(petunia_config::text_id::UI_QUICK_ACTION_RESET);
        vm.label_quick_action_done = translated(petunia_config::text_id::UI_QUICK_ACTION_DONE);
        vm.label_action_subdivide = translated(petunia_config::text_id::UI_ACTION_SUBDIVIDE);
        vm.label_action_fuse = translated(petunia_config::text_id::UI_ACTION_FUSE);
        vm.label_action_cut = translated(petunia_config::text_id::UI_ACTION_CUT);
        vm.label_action_intersect = translated(petunia_config::text_id::UI_ACTION_INTERSECT);
        vm.label_action_join = translated(petunia_config::text_id::UI_ACTION_JOIN);
        vm.label_action_merge = translated(petunia_config::text_id::UI_ACTION_MERGE);
        vm.label_action_slice = translated(petunia_config::text_id::UI_ACTION_SLICE);
        vm.label_action_loop_cut = translated(petunia_config::text_id::UI_ACTION_LOOP_CUT);
        vm.label_material_base_color = translated(petunia_config::text_id::UI_MATERIAL_BASE_COLOR);
        vm.label_material_profile = translated(petunia_config::text_id::UI_MATERIAL_PROFILE);
        vm.label_material_roughness = translated(petunia_config::text_id::UI_MATERIAL_ROUGHNESS);
        vm.label_material_metallic = translated(petunia_config::text_id::UI_MATERIAL_METALLIC);
        vm.label_material_normal_scale =
            translated(petunia_config::text_id::UI_MATERIAL_NORMAL_SCALE);
        vm.label_material_advanced = translated(petunia_config::text_id::UI_MATERIAL_ADVANCED);
        vm.label_material_assign = translated(petunia_config::text_id::UI_MATERIAL_ASSIGN);
        vm.label_material_new = translated(petunia_config::text_id::UI_MATERIAL_NEW);
        vm.label_material_duplicate = translated(petunia_config::text_id::UI_MATERIAL_DUPLICATE);
        vm.label_material_remove = translated(petunia_config::text_id::UI_MATERIAL_REMOVE);
        vm.label_material_no_material =
            translated(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL);
        vm.label_material_no_selection =
            translated(petunia_config::text_id::UI_MATERIAL_NO_SELECTION);
        vm.label_material_emission_strength =
            translated(petunia_config::text_id::UI_MATERIAL_EMISSION_STRENGTH);
        vm.label_material_alpha_cutoff =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_CUTOFF);
        vm.label_material_texture_albedo =
            translated(petunia_config::text_id::UI_MATERIAL_TEXTURE_ALBEDO);
        vm.label_material_no_texture = translated(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE);
        vm.label_material_create_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CREATE_TEXTURE);
        vm.label_material_clear_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CLEAR_TEXTURE);
        vm.label_material_profile_pbr =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_PBR);
        vm.label_material_profile_unlit =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_UNLIT);
        vm.label_material_profile_toon =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_TOON);
        vm.label_material_profile_glass =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_GLASS);
        vm.label_material_profile_emissive =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_EMISSIVE);
        vm.label_material_alpha_opaque =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_OPAQUE);
        vm.label_material_alpha_mask = translated(petunia_config::text_id::UI_MATERIAL_ALPHA_MASK);
        vm.label_material_alpha_blend =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_BLEND);
        vm.label_modifier_mirror = translated(petunia_config::text_id::UI_MODIFIER_MIRROR);
        vm.label_modifier_symmetry = translated(petunia_config::text_id::UI_MODIFIER_SYMMETRY);
        vm.label_modifiers_empty = translated(petunia_config::text_id::UI_MODIFIERS_EMPTY);
        vm.label_modifier_apply = translated(petunia_config::text_id::UI_MODIFIER_APPLY);
        vm.label_modifier_axis = translated(petunia_config::text_id::UI_MODIFIER_AXIS);
        vm.label_modifier_add_mirror = translated(petunia_config::text_id::UI_MODIFIER_ADD_MIRROR);
        vm.label_modifier_add_symmetry =
            translated(petunia_config::text_id::UI_MODIFIER_ADD_SYMMETRY);
        vm.label_modifier_remove = translated(petunia_config::text_id::UI_MODIFIER_REMOVE);
        vm.label_modifier_move_up = translated(petunia_config::text_id::UI_MODIFIER_MOVE_UP);
        vm.label_modifier_move_down = translated(petunia_config::text_id::UI_MODIFIER_MOVE_DOWN);
        vm.label_modifier_direction = translated(petunia_config::text_id::UI_MODIFIER_DIRECTION);
        vm.label_modifier_positive_to_negative =
            translated(petunia_config::text_id::UI_MODIFIER_POSITIVE_TO_NEGATIVE);
        vm.label_modifier_negative_to_positive =
            translated(petunia_config::text_id::UI_MODIFIER_NEGATIVE_TO_POSITIVE);
        vm.label_tab_parts = translated(petunia_config::text_id::UI_TAB_PARTS);
        vm.label_tab_transform = translated(petunia_config::text_id::UI_TAB_TRANSFORM);
        vm.label_tab_material = translated(petunia_config::text_id::UI_TAB_MATERIAL);
        vm.label_tab_modifiers = translated(petunia_config::text_id::UI_TAB_MODIFIERS);
        vm.material_slots = self
            .state
            .project
            .project
            .materials
            .iter()
            .map(|m| m.name.clone())
            .collect();
        vm.active_material_slot = self
            .active_material_slot
            .clamp(0, vm.material_slots.len().saturating_sub(1) as i32);
        vm.label_tab_object = translated(petunia_config::text_id::UI_TAB_OBJECT);
        vm.label_numeric_field_hint = translated(petunia_config::text_id::UI_NUMERIC_FIELD_HINT);
        vm.label_model_select = translated(petunia_config::text_id::TOOLS_SELECT);
        vm.label_model_position = translated(petunia_config::text_id::TRANSFORM_POSITION);
        vm.label_model_rotate = translated(petunia_config::text_id::TOOLS_ROTATE);
        vm.label_model_scale = translated(petunia_config::text_id::TOOLS_SCALE);
        vm.label_model_transform = translated(petunia_config::text_id::TOOLS_TRANSFORM);
        vm.label_model_lasso = translated(petunia_config::text_id::TOOLS_SELECT_LASSO);
        vm.hint_model_select = translated(petunia_config::text_id::UI_MODEL_SELECT_HINT);
        vm.hint_model_position = translated(petunia_config::text_id::UI_MODEL_POSITION_HINT);
        vm.hint_model_rotate = translated(petunia_config::text_id::UI_MODEL_ROTATE_HINT);
        vm.hint_model_scale = translated(petunia_config::text_id::UI_MODEL_SCALE_HINT);
        vm.hint_model_transform = translated(petunia_config::text_id::UI_MODEL_TRANSFORM_HINT);
        vm.hint_model_lasso = translated(petunia_config::text_id::UI_MODEL_LASSO_HINT);
        vm.label_model_loop_cut = translated(petunia_config::text_id::TOOLS_LOOP_CUT);
        vm.hint_model_loop_cut = translated(petunia_config::text_id::UI_LOOP_CUT_HINT);
        vm.label_model_slice = translated(petunia_config::text_id::TOOLS_SLICE);
        vm.hint_model_slice = translated(petunia_config::text_id::UI_SLICE_HINT);
        vm.label_model_push_pull = translated(petunia_config::text_id::TOOLS_PUSH_PULL);
        vm.hint_model_push_pull = translated(petunia_config::text_id::UI_PUSH_PULL_HINT);
        vm.label_model_profile = translated(petunia_config::text_id::TOOLS_DRAW_PROFILE);
        vm.hint_model_profile = translated(petunia_config::text_id::UI_PROFILE_HINT);
        vm.label_model_pivot = translated(petunia_config::text_id::TOOLS_PIVOT);
        vm.hint_model_pivot = translated(petunia_config::text_id::UI_PIVOT_HINT);
        vm.label_profile_depth = translated(petunia_config::text_id::UI_PROFILE_DEPTH);
        vm.label_profile_points = translated(petunia_config::text_id::UI_PROFILE_POINTS);
        vm.label_profile_close = translated(petunia_config::text_id::UI_PROFILE_CLOSE);
        vm.label_profile_generate = translated(petunia_config::text_id::UI_PROFILE_GENERATE);
        vm.label_profile_revolve = translated(petunia_config::text_id::UI_PROFILE_REVOLVE);
        vm.label_profile_cuts = translated(petunia_config::text_id::UI_PROFILE_CUTS);
        vm.label_profile_presets = translated(petunia_config::text_id::UI_PROFILE_PRESETS);
        vm.label_profile_add_rect = translated(petunia_config::text_id::UI_PROFILE_ADD_RECT);
        vm.label_profile_add_circle = translated(petunia_config::text_id::UI_PROFILE_ADD_CIRCLE);
        vm.label_profile_canvas_hint = translated(petunia_config::text_id::UI_PROFILE_CANVAS_HINT);
        vm.label_hide_part = translated(petunia_config::text_id::UI_HIDE_PART);
        vm.label_show_part = translated(petunia_config::text_id::UI_SHOW_PART);
        vm.label_lock_part = translated(petunia_config::text_id::UI_LOCK_PART);
        vm.label_unlock_part = translated(petunia_config::text_id::UI_UNLOCK_PART);
        vm.label_selected_parts_only = translated(petunia_config::text_id::UI_SELECTED_PARTS_ONLY);
        vm.label_sort_parts = translated(petunia_config::text_id::UI_SORT_PARTS);
        vm.label_parts_row_size = translated(petunia_config::text_id::UI_PARTS_ROW_SIZE);
        vm.label_sort_assets = translated(petunia_config::text_id::UI_SORT_ASSETS);
        vm.label_thumbnail_size = translated(petunia_config::text_id::UI_THUMBNAIL_SIZE);
        vm.label_selection_color = translated(petunia_config::text_id::UI_SELECTION_COLOR);
        vm.label_highlight_thickness = translated(petunia_config::text_id::UI_HIGHLIGHT_THICKNESS);
        vm.label_view_wireframe = translated(petunia_config::text_id::UI_VIEW_WIREFRAME);
        vm.label_view_wireframe_hint = translated(petunia_config::text_id::UI_VIEW_WIREFRAME_HINT);
        vm.label_view_solid = translated(petunia_config::text_id::UI_VIEW_SOLID);
        vm.label_view_solid_hint = translated(petunia_config::text_id::UI_VIEW_SOLID_HINT);
        vm.label_view_material = translated(petunia_config::text_id::UI_VIEW_MATERIAL);
        vm.label_view_material_hint = translated(petunia_config::text_id::UI_VIEW_MATERIAL_HINT);
        vm.label_view_lit = translated(petunia_config::text_id::UI_VIEW_LIT);
        vm.label_view_lit_hint = translated(petunia_config::text_id::UI_VIEW_LIT_HINT);
        vm.label_more_model_tools = translated(petunia_config::text_id::UI_MORE_MODEL_TOOLS);
        vm.label_xray_opacity = translated(petunia_config::text_id::UI_XRAY_OPACITY);
        vm.label_wire_overlay = translated(petunia_config::text_id::UI_WIRE_OVERLAY);
        vm.label_wire_overlay_hint = translated(petunia_config::text_id::UI_WIRE_OVERLAY_HINT);
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
        if let Some(effect) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Effect(effect) => Some(*effect),
                _ => None,
            })
        {
            use petunia_project::paint_layers::PaintEffect;
            vm.paint_effect_kind = match &effect {
                PaintEffect::Pixelate { .. } => "Pixelate",
                PaintEffect::Posterize { .. } => "Posterize",
                PaintEffect::Invert => "Invert",
                PaintEffect::Grain { .. } => "Grain",
                PaintEffect::Levels { .. } => "Levels",
                PaintEffect::BrightnessContrast { .. } => "BrightnessContrast",
                PaintEffect::HueSaturation { .. } => "HueSaturation",
            }
            .to_string();
            vm.paint_effect_params = effect_params(&effect);
        }
        if let Some((width, height)) = self.paint_canvas_dimensions() {
            vm.paint_canvas_size = format!("{width} × {height}");
            vm.paint_canvas_revision = self.state.project.assets[self.state.project.active]
                .paint_stack
                .as_ref()
                .map(|stack| {
                    stack
                        .layers
                        .iter()
                        .filter_map(|layer| layer.canvas())
                        .map(|canvas| canvas.w as i32 * canvas.h as i32)
                        .sum::<i32>()
                        + stack.layers.len() as i32
                })
                .unwrap_or(0);
        }
        vm.paint_fill_scope = format!("{:?}", self.state.session.tools.fill_scope);
        vm.paint_projection = format!("{:?}", self.state.session.tools.brush_projection);
        vm.paint_lock = format!("{:?}", self.state.session.tools.brush_lock);
        vm.paint_pixel_grid = self.paint_pixel_grid;
        vm.paint_canvas_zoom = self.paint_canvas_zoom;
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
            if let Ok(segments) = session
                .ring
                .preview(&session.source, session.cuts, session.slide)
            {
                let mut commands = String::new();
                for [a, b] in segments {
                    project_preview_segment(
                        &self.state.session.camera,
                        self.viewport_size,
                        a,
                        b,
                        &mut commands,
                    );
                }
                vm.loop_cut_preview_commands = commands;
            }
        } else if self.state.session.tools.active_tool == "loop_cut" {
            vm.loop_cut_armed = true;
            vm.loop_cut_cuts = self.loop_cut_hover_cuts as i32;
            vm.loop_cut_preview_commands = self.loop_cut_hover_preview_commands();
        }
        vm.profile_active = self.state.session.tools.active_tool == "draw_profile";
        vm.profile_point_count = self.state.profile.points.len() as i32;
        vm.profile_closed = self.state.profile.closed;
        vm.profile_depth = self.state.profile.depth;
        if vm.profile_active {
            vm.profile_preview_commands = self.profile_preview_commands();
        }
        if let Some(anchor) = self.slice_anchor {
            vm.operation_preview_commands = format!(
                "M {:.2} {:.2} L {:.2} {:.2}",
                anchor[0], anchor[1], self.pointer_position[0], self.pointer_position[1]
            );
        }
        vm.tool_activation = self.state.session.tools.tool_activation.id().to_string();
        vm.keyboard_tool_modal_active =
            self.keyboard_tool_modal_active && self.tool_modal.is_some();
        vm.invert_vertical_drag = self.state.ui.invert_vertical_drag;
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            vm.tool_modal_active = true;
            vm.tool_modal_id = kind.id().to_string();
            vm.tool_modal_title = kind.title().to_string();
            vm.tool_modal_label = kind.label().to_string();
            vm.tool_modal_value = self.tool_modal_value;
            vm.tool_modal_step = kind.step();
            vm.tool_modal_min = minimum;
            vm.tool_modal_max = maximum;
        }
        vm.tool_options_active = self.tool_modal.is_some()
            || self.loop_cut.is_some()
            || self.state.session.tools.active_tool == "loop_cut"
            || self.state.session.tools.active_tool == "draw_profile"
            || (self.state.workspace == Workspace::Model
                && !matches!(
                    self.state.session.tools.active_tool.as_str(),
                    "select" | "box_select" | "lasso_select"
                ));
        vm.tool_options_title = if let Some(kind) = self.tool_modal {
            kind.title().to_string()
        } else if self.loop_cut.is_some() || self.state.session.tools.active_tool == "loop_cut" {
            self.state.t_id(petunia_config::text_id::TOOLS_LOOP_CUT)
        } else if self.state.session.tools.active_tool == "draw_profile" {
            self.state.t_id(petunia_config::text_id::TOOLS_DRAW_PROFILE)
        } else {
            match self.state.session.tools.active_tool.as_str() {
                "scale" => self.state.t_id(petunia_config::text_id::TOOLS_SCALE),
                "rotate" => self.state.t_id(petunia_config::text_id::TOOLS_ROTATE),
                "move" => self.state.t_id(petunia_config::text_id::TOOLS_TRANSFORM),
                "inset" => self
                    .state
                    .t_id(petunia_config::text_id::UI_ACTION_SUBDIVIDE),
                "bevel" => self.state.t("tools.bevel"),
                "push_pull" => self.state.t_id(petunia_config::text_id::TOOLS_PUSH_PULL),
                _ => self
                    .state
                    .t_id(petunia_config::text_id::UI_NO_TOOL_PARAMETERS),
            }
        };
        vm.tool_options_hint = if self.tool_modal.is_some() {
            self.state
                .t_id(petunia_config::text_id::UI_NUMERIC_FIELD_HINT)
        } else {
            self.state
                .t_id(petunia_config::text_id::UI_NO_TOOL_PARAMETERS)
        };
        if let Some(asset) = self.state.project.active() {
            vm.object_has_selection = true;
            vm.object_id = asset.id.to_string();
            vm.object_name = asset.name.clone();
            vm.object_visible = asset.visible;
            vm.object_locked = asset.locked;
            vm.object_verts = asset.mesh.verts.len() as i32;
            vm.object_faces = asset.mesh.faces.len() as i32;
            vm.object_tris = asset.mesh.tri_count() as i32;
            vm.object_selection = vm.selection_summary.clone();
            vm.object_material = asset
                .material(&self.state.project.project)
                .map(|material| material.name.clone())
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL)
                });
            vm.object_modifier_count = asset.modifiers.len() as i32;
        }
        let material_slot = vm.active_material_slot.max(0) as usize;
        if let Some(material) = self.state.project.project.materials.get(material_slot) {
            vm.material_has_selection = true;
            vm.material_id = material.id.to_string();
            vm.material_name = material.name.clone();
            vm.material_profile = match material.profile {
                petunia_project::ShaderProfile::Pbr => "pbr",
                petunia_project::ShaderProfile::Unlit => "unlit",
                petunia_project::ShaderProfile::Toon => "toon",
                petunia_project::ShaderProfile::Glass => "glass",
                petunia_project::ShaderProfile::Emissive => "emissive",
            }
            .to_string();
            vm.material_profile_label = material.profile.label().to_string();
            vm.material_base_color = [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ];
            vm.material_roughness = material.roughness;
            vm.material_metallic = material.metallic;
            vm.material_normal_scale = material.normal_scale;
            vm.material_emission = material.emission_color;
            vm.material_emission_strength = material.emission_strength;
            vm.material_alpha_mode = match material.alpha_mode {
                petunia_project::AlphaMode::Opaque => "opaque",
                petunia_project::AlphaMode::Mask => "mask",
                petunia_project::AlphaMode::Blend => "blend",
            }
            .to_string();
            vm.material_alpha_cutoff = material.alpha_cutoff;
            vm.material_has_albedo = material.albedo_texture.is_some();
            vm.material_albedo_label = material
                .albedo_texture
                .as_ref()
                .map(|texture| format!("{} × {}", texture.w, texture.h))
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE)
                });
        }
        vm.material_palette = self.state.project.palette.clone();
        let pinned_ids = self.state.ui.model_quick_action_ids();
        let quick_label = |id: &str| match id {
            "model.subdivide" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_SUBDIVIDE),
            "model.fuse" => self.state.t_id(petunia_config::text_id::UI_ACTION_FUSE),
            "model.cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_CUT),
            "model.intersect" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_INTERSECT),
            "model.join" => self.state.t_id(petunia_config::text_id::UI_ACTION_JOIN),
            "model.merge" => self.state.t_id(petunia_config::text_id::UI_ACTION_MERGE),
            "model.slice" => self.state.t_id(petunia_config::text_id::UI_ACTION_SLICE),
            "model.loop_cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_LOOP_CUT),
            _ => id.to_string(),
        };
        vm.quick_actions = pinned_ids
            .iter()
            .map(|id| QuickActionModel {
                id: id.clone(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || id == "model.slice",
                active: self.state.session.tools.active_tool == id.as_str(),
                pinned: true,
            })
            .collect();
        vm.quick_action_candidates = petunia_core::state::UiState::MODEL_QUICK_ACTION_CANDIDATES
            .iter()
            .map(|id| QuickActionModel {
                id: (*id).to_string(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || *id == "model.slice",
                active: self.state.session.tools.active_tool == *id,
                pinned: pinned_ids.iter().any(|pinned| pinned == id),
            })
            .collect();
        if let Some(asset) = self.state.project.active() {
            vm.modifier_rows = asset
                .modifiers
                .iter()
                .enumerate()
                .map(|(index, modifier)| {
                    let (kind, axis, positive_to_negative) = match modifier.kind {
                        petunia_project::ModifierKind::Mirror { axis, .. } => {
                            ("mirror", axis, true)
                        }
                        petunia_project::ModifierKind::Symmetry {
                            axis,
                            positive_to_negative,
                            ..
                        } => ("symmetry", axis, positive_to_negative),
                    };
                    let title = if kind == "mirror" {
                        self.state.t_id(petunia_config::text_id::UI_MODIFIER_MIRROR)
                    } else {
                        self.state
                            .t_id(petunia_config::text_id::UI_MODIFIER_SYMMETRY)
                    };
                    ModifierRowModel {
                        id: modifier.id.to_string(),
                        subtitle: format!(
                            "{} {}",
                            self.state.t_id(petunia_config::text_id::UI_MODIFIER_AXIS),
                            ["X", "Y", "Z"][axis.min(2)]
                        ),
                        title,
                        enabled: modifier.enabled,
                        kind: kind.to_string(),
                        axis: axis as i32,
                        positive_to_negative,
                        can_move_up: index > 0,
                        can_move_down: index + 1 < asset.modifiers.len(),
                    }
                })
                .collect();
        }
        vm
    }

    fn sync_viewport_context(&mut self) {
        self.viewport.set_workspace(self.state.workspace);
        self.viewport
            .set_selection_domain(self.state.selection_domain());
    }

    fn reset_transform_fields(&mut self) {
        self.modal_text.clear();
        self.instant_transform = false;
        self.gizmo_drag = None;
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
            OverlayId::ContextMenu => self.context_menu = None,
            OverlayId::MenuBar => self.menu_open = None,
            OverlayId::PivotMenu => self.pivot_menu_open = false,
        }
    }
}

/// Projeta o pivô da seleção e os três eixos do mundo para o overlay Slint.
/// Distância de um ponto a um segmento, em espaço de tela.
pub fn point_segment_distance(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [point[0] - a[0], point[1] - a[1]];
    let length_squared = ab[0] * ab[0] + ab[1] * ab[1];
    if length_squared <= 1.0e-6 {
        return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / length_squared).clamp(0.0, 1.0);
    let closest = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    ((point[0] - closest[0]).powi(2) + (point[1] - closest[1]).powi(2)).sqrt()
}

fn parse_lasso_path(path: &str) -> Option<Vec<[f32; 2]>> {
    if path.len() > 131_072 {
        return None;
    }
    let mut polygon = Vec::new();
    for part in path.split(';').filter(|part| !part.is_empty()) {
        let (x, y) = part.split_once(',')?;
        let x: f32 = x.parse().ok()?;
        let y: f32 = y.parse().ok()?;
        if !x.is_finite() || !y.is_finite() || polygon.len() >= 4096 {
            return None;
        }
        // O grab entrega valores fora da viewport. Projetar na borda mantém
        // a forma do laço, em vez de descartar pontos e abrir um corte.
        polygon.push([x.clamp(0.0, 1.0) * 2.0 - 1.0, 1.0 - y.clamp(0.0, 1.0) * 2.0]);
    }
    (polygon.len() >= 3).then_some(polygon)
}

fn compute_gizmo(state: &AppState, width: f32, height: f32) -> GizmoModel {
    /// Comprimento das hastes do gizmo de transformação, em px lógicos.
    const ROD_LENGTH: f32 = 72.0;
    /// Tamanho da seta: recuo da ponta e meia-largura da base.
    const ARROW_BACK: f32 = 13.0;
    const ARROW_HALF: f32 = 5.5;
    /// Tripé de navegação: no alto à direita, abaixo dos controles da viewport.
    const VIEW_MARGIN: f32 = 54.0;
    const VIEW_TOP: f32 = 108.0;
    const VIEW_LENGTH: f32 = 38.0;

    if width <= 1.0 || height <= 1.0 {
        return GizmoModel::default();
    }

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
    // Camera-space directions preserve foreshortening: an axis pointing at
    // the viewer should shrink, not turn into a full-length diagonal.
    let screen_direction = |axis: glam::Vec3| -> [f32; 2] {
        [
            axis.dot(state.session.camera.right()),
            -axis.dot(state.session.camera.up()),
        ]
    };

    let mut model = GizmoModel::default();

    // Tripé de navegação: sempre visível quando a viewport tem tamanho válido.
    // Ele mostra a orientação da câmera, não a cena: por isso as hastes partem
    // de uma âncora fixa no canto, não de um ponto projetado.
    {
        model.view_origin_x = width - VIEW_MARGIN;
        model.view_origin_y = VIEW_TOP.min(height - VIEW_MARGIN);
        let origin = [VIEW_MARGIN, VIEW_MARGIN];
        for (axis, slot, endpoint) in [
            (
                glam::Vec3::X,
                &mut model.view_x_commands,
                &mut model.view_x_end,
            ),
            (
                glam::Vec3::Y,
                &mut model.view_y_commands,
                &mut model.view_y_end,
            ),
            (
                glam::Vec3::Z,
                &mut model.view_z_commands,
                &mut model.view_z_end,
            ),
        ] {
            let direction = screen_direction(axis);
            let end = [
                origin[0] + direction[0] * VIEW_LENGTH,
                origin[1] + direction[1] * VIEW_LENGTH,
            ];
            *endpoint = end;
            *slot = format!(
                "M {:.2} {:.2} L {:.2} {:.2} ",
                origin[0], origin[1], end[0], end[1]
            );
        }
    }

    // Hastes de transformação: tamanho fixo em tela. O modo combinado mantém
    // três famílias de handle simultâneas e selecionáveis.
    if state.workspace != Workspace::Model
        || !matches!(
            state.session.tools.active_tool.as_str(),
            "move" | "rotate" | "scale" | "transform"
        )
    {
        return model;
    }
    let Some(asset) = state.project.active() else {
        return model;
    };
    if asset.mesh.verts.is_empty() {
        return model;
    };
    let pivot = state.session.tools.modal.as_ref().map_or_else(
        || state.calculate_pivot(state.session.pivot_point),
        |modal| modal.pivot,
    );
    let Some(origin) = project(pivot) else {
        return model;
    };
    model.visible = true;
    model.origin_x = origin[0];
    model.origin_y = origin[1];

    for (axis, rod, arrow, scale, rotate) in [
        (
            glam::Vec3::X,
            &mut model.x_commands,
            &mut model.x_arrow_commands,
            &mut model.x_scale_commands,
            &mut model.x_rotate_commands,
        ),
        (
            glam::Vec3::Y,
            &mut model.y_commands,
            &mut model.y_arrow_commands,
            &mut model.y_scale_commands,
            &mut model.y_rotate_commands,
        ),
        (
            glam::Vec3::Z,
            &mut model.z_commands,
            &mut model.z_arrow_commands,
            &mut model.z_scale_commands,
            &mut model.z_rotate_commands,
        ),
    ] {
        if state.session.tools.active_tool == "rotate" {
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let direction = screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rod.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + direction[0] * ROD_LENGTH,
                    origin[1] + direction[1] * ROD_LENGTH
                ));
            }
            continue;
        }
        let direction = screen_direction(axis);
        let end = [
            origin[0] + direction[0] * ROD_LENGTH,
            origin[1] + direction[1] * ROD_LENGTH,
        ];
        *rod = format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            origin[0], origin[1], end[0], end[1]
        );
        if state.session.tools.active_tool == "transform" {
            let scale_center = [
                origin[0] + direction[0] * 43.0,
                origin[1] + direction[1] * 43.0,
            ];
            let r = 5.0;
            *scale = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                scale_center[0] - r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] + r,
                scale_center[0] - r,
                scale_center[1] + r,
            );
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let ring_direction =
                    screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rotate.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + ring_direction[0] * 91.0,
                    origin[1] + ring_direction[1] * 91.0
                ));
            }
        }
        if state.session.tools.active_tool == "scale" {
            let r = ARROW_HALF;
            *arrow = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                end[0] - r,
                end[1] - r,
                end[0] + r,
                end[1] - r,
                end[0] + r,
                end[1] + r,
                end[0] - r,
                end[1] + r
            );
            continue;
        }
        // Seta: ponta em `end`, base recuada ao longo da haste.
        let base = [
            end[0] - direction[0] * ARROW_BACK,
            end[1] - direction[1] * ARROW_BACK,
        ];
        let perpendicular = [-direction[1], direction[0]];
        let left = [
            base[0] + perpendicular[0] * ARROW_HALF,
            base[1] + perpendicular[1] * ARROW_HALF,
        ];
        let right = [
            base[0] - perpendicular[0] * ARROW_HALF,
            base[1] - perpendicular[1] * ARROW_HALF,
        ];
        *arrow = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            end[0], end[1], left[0], left[1], right[0], right[1]
        );
    }
    model
}

/// Tracejado pontilhado do "cordão" entre a base da seleção (pivô) e o
/// mouse durante uma ferramenta de manipulação ativa (Move/Rotate/Scale por
/// arrasto ou modal de teclado, Extrude/Inset/Bevel paramétricos...).
/// O Path do Slint não tem dash nativo, então o padrão nasce aqui em Rust:
/// pontos de 2px com 4px de intervalo ao longo do segmento base→mouse.
/// A linha cresce/encolhe sozinha conforme o mouse se afasta/aproxima.
fn dotted_link_commands(from: [f32; 2], to: [f32; 2]) -> String {
    const DASH: f32 = 2.0;
    const GAP: f32 = 4.0;
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = dx.hypot(dy);
    if length < 0.5 {
        return String::new();
    }
    let (ux, uy) = (dx / length, dy / length);
    let mut commands = String::new();
    let mut cursor = 0.0;
    while cursor < length {
        let end = (cursor + DASH).min(length);
        commands.push_str(&format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            from[0] + ux * cursor,
            from[1] + uy * cursor,
            from[0] + ux * end,
            from[1] + uy * end,
        ));
        cursor = end + GAP;
    }
    commands
}

/// Cordão pivô→mouse da ferramenta ativa. Fora de sessão de manipulação
/// retorna vazio e o Path some da tela.
fn compute_drag_link(
    state: &AppState,
    width: f32,
    height: f32,
    pointer: [f32; 2],
    link_active: bool,
) -> String {
    if !link_active || width <= 1.0 || height <= 1.0 {
        return String::new();
    }
    let pivot = state.calculate_pivot(state.session.pivot_point);
    let view_proj = state.session.camera.view_proj();
    let clip = view_proj * pivot.extend(1.0);
    if clip.w <= 0.05 {
        return String::new();
    }
    let inv_w = 1.0 / clip.w;
    let base = [
        (clip.x * inv_w * 0.5 + 0.5) * width,
        (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
    ];
    dotted_link_commands(base, pointer)
}

/// Resumo legível da seleção (Object Info do Blender): o que está
/// selecionado e quanto. Linha única para a pill da viewport e o inspector.
fn format_selection_summary(state: &AppState) -> String {
    fn plural(count: usize, singular: &str, plural: &str) -> Option<String> {
        if count == 0 {
            None
        } else if count == 1 {
            Some(format!("1 {singular}"))
        } else {
            Some(format!("{count} {plural}"))
        }
    }
    if state.selection_domain() == SelectionDomain::Object {
        // Espelha a regra da UI: ativo conta como selecionado.
        let active_id = state.project.active().map(|a| a.id);
        let total = state
            .project
            .assets
            .iter()
            .filter(|a| state.session.selection.assets.contains(&a.id) || active_id == Some(a.id))
            .count();
        return plural(total, "object selected", "objects selected")
            .unwrap_or_else(|| "No selection".to_string());
    }
    let details = state.query_selection_details();
    let mut parts = Vec::new();
    if let Some(text) = plural(details.selected_verts_count, "point", "points") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_edges_count, "edge", "edges") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_faces_count, "face", "faces") {
        parts.push(text);
    }
    if parts.is_empty() {
        "No selection".to_string()
    } else {
        format!("Selected: {}", parts.join(" · "))
    }
}

fn compute_selection_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
) -> SelectionOverlayModel {
    if state.selection_domain() != SelectionDomain::Object {
        return compute_asset_overlay(
            state,
            width,
            height,
            backend_draws_guides,
            state.project.active,
        );
    }
    let mut overlay = SelectionOverlayModel::default();
    for (index, asset) in state.project.assets.iter().enumerate() {
        if !asset.visible {
            continue;
        }
        let is_active = index == state.project.active;
        let selected = state.session.selection.assets.contains(&asset.id) || is_active;
        let hovered = state.session.tools.hover == petunia_core::HoverTarget::Object(index);
        if !selected && !hovered {
            continue;
        }
        let part = compute_asset_overlay(state, width, height, backend_draws_guides, index);
        if is_active {
            overlay
                .active_outline_commands
                .push_str(&part.outline_commands);
        } else if selected {
            overlay.outline_commands.push_str(&part.outline_commands);
        } else {
            overlay
                .unselected_outline_commands
                .push_str(&part.outline_commands);
        }
    }
    overlay.visible = !overlay.outline_commands.is_empty()
        || !overlay.active_outline_commands.is_empty()
        || !overlay.unselected_outline_commands.is_empty();
    overlay
}

fn compute_asset_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
    index: usize,
) -> SelectionOverlayModel {
    /// Teto de segmentos por frame: malhas grandes não podem gerar uma string
    /// gigante a cada sync de propriedades.
    const MAX_SEGMENTS: usize = 4_000;

    if width <= 1.0 || height <= 1.0 {
        return SelectionOverlayModel::default();
    }
    if state.workspace == Workspace::Paint {
        return SelectionOverlayModel::default();
    }
    let Some(asset) = state.project.assets.get(index) else {
        return SelectionOverlayModel::default();
    };
    let evaluated = asset.evaluated_mesh();
    let mesh = &evaluated;
    if mesh.verts.is_empty() {
        return SelectionOverlayModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };

    let mut outline = String::new();
    let mut points = String::new();
    let mut unselected_outline = String::new();
    let mut unselected_points = String::new();
    let mut segments = 0usize;
    let push_segment = |commands: &mut String, a: [f32; 2], b: [f32; 2]| {
        commands.push_str(&format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        ));
    };
    let push_disc = |commands: &mut String, center: [f32; 2], radius: f32| {
        use std::fmt::Write as _;
        for step in 0..12 {
            let angle = step as f32 * std::f32::consts::TAU / 12.0;
            let action = if step == 0 { 'M' } else { 'L' };
            let _ = write!(
                commands,
                "{action} {:.2} {:.2} ",
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin()
            );
        }
        commands.push_str("Z ");
    };

    let domain = state.selection_domain();
    let mut truncated = false;

    match domain {
        SelectionDomain::Object => {
            // Silhueta: aresta entre face frontal e traseira (ou borda aberta
            // frontal). Não desenhar todas as arestas de faces frontais, o que
            // faria um objeto selecionado parecer uma caixa de arame gigante.
            let mut adjacent = std::collections::HashMap::<(u32, u32), (usize, usize)>::new();
            let eye = state.session.camera.eye();
            for (fi, face) in mesh.faces.iter().enumerate() {
                if face.verts.len() < 3 {
                    continue;
                }
                let face_center = face
                    .verts
                    .iter()
                    .filter_map(|&vi| mesh.verts.get(vi as usize))
                    .map(|vertex| vertex.vec())
                    .sum::<glam::Vec3>()
                    / face.verts.len() as f32;
                let front = mesh.face_normal(fi).dot(eye - face_center) > 0.0;
                for index in 0..face.verts.len() {
                    let a = face.verts[index];
                    let b = face.verts[(index + 1) % face.verts.len()];
                    let entry = adjacent.entry((a.min(b), a.max(b))).or_default();
                    if front {
                        entry.0 += 1;
                    } else {
                        entry.1 += 1;
                    }
                }
            }
            for ((a, b), (front, back)) in adjacent {
                if front == 0 || (back == 0 && front > 1) {
                    continue;
                }
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Vertex => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (index, vertex) in mesh.verts.iter().enumerate() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let Some(sp) = project(vertex.vec()) else {
                    continue;
                };
                let hovered =
                    state.session.tools.hover == petunia_core::HoverTarget::Vertex(index as u32);
                let radius = if hovered {
                    (state.ui.selection_thickness * 2.5).clamp(6.5, 8.5)
                } else if vertex.selected {
                    (state.ui.selection_thickness * 2.0).clamp(5.0, 7.0)
                } else {
                    (state.ui.selection_thickness * 1.5).clamp(3.5, 5.5)
                };
                let target = if hovered || vertex.selected {
                    &mut points
                } else {
                    &mut unselected_points
                };
                push_disc(target, sp, radius);
                segments += 1;
            }
        }
        SelectionDomain::Edge => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (a, b) in mesh.edges_unique() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let selected =
                    mesh.selected_edges.contains(&(a, b)) || mesh.selected_edges.contains(&(b, a));
                if selected {
                    // O GPU desenha a aresta selecionada com depth test.
                    continue;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut unselected_outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Face => {
            // A seleção e o hover de faces são preenchidos pelo renderer.
            // Marcadores centrais em todas as faces poluíam a malha.
        }
    }

    let _ = truncated;
    let visible = !outline.is_empty()
        || !points.is_empty()
        || !unselected_outline.is_empty()
        || !unselected_points.is_empty();
    SelectionOverlayModel {
        visible,
        outline_commands: outline,
        // O roteamento ativo × selecionado acontece no chamador
        // (compute_selection_overlay); aqui nasce sempre vazio.
        active_outline_commands: String::new(),
        point_commands: points,
        unselected_outline_commands: unselected_outline,
        unselected_point_commands: unselected_points,
        accent: domain != SelectionDomain::Object,
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
        for corners in mesh.face_triangle_corners(face_index) {
            let [p0, p1, p2] = corners.map(|i| mesh.verts[face.verts[i] as usize].vec());
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
    let mut state = AppState::default();
    let preferences = petunia_config::UserPreferences::load();
    state.ui.invert_vertical_drag = preferences.invert_vertical_drag;
    state.ui.selection_rgb = if selection_color_has_contrast(preferences.selection_rgb) {
        preferences.selection_rgb
    } else {
        petunia_config::UserPreferences::default().selection_rgb
    };
    state.ui.selection_thickness = preferences.selection_thickness.clamp(1.0, 6.0);
    state.ui.model_quick_actions = preferences.model_quick_actions;

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
        show_face_orientation: state.session.show_face_orientation,
        show_uv_checker: state.session.show_uv_checker,
        selection_domain: state.selection_domain(),
        xray_opacity: state.session.xray_opacity,
        selection_rgb: state.ui.selection_rgb,
        selection_thickness: state.ui.selection_thickness,
        show_grid: state.session.show_grid,
        hover: state.session.tools.hover,
    };
    if let Some(frame) = viewport.render_frame(&state.project, &state.session.camera, render_state)
    {
        window.set_viewport_image(frame);
    }
    window.set_has_gpu_viewport(true);

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

    // O primeiro layout pode ocorrer antes da instalação dos callbacks de
    // resize. Use suas dimensões reais antes do primeiro frame interativo.
    let initial_bridge = Arc::clone(&bridge);
    let initial_window = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(window) = initial_window.upgrade()
            && let Ok(mut bridge) = initial_bridge.lock()
        {
            bridge.resize_viewport(
                window.get_viewport_region_width().round().max(1.0) as u32,
                window.get_viewport_region_height().round().max(1.0) as u32,
            );
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
    });

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

    // Tique de acúmulo contínuo de tinta para Airbrush (P3D-056).
    let airbrush_bridge = Arc::clone(&bridge);
    let airbrush_window = window.as_weak();
    let airbrush_timer = slint::Timer::default();
    airbrush_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        move || {
            if let Ok(mut bridge) = airbrush_bridge.lock()
                && bridge.airbrush_tick()
                && let Some(window) = airbrush_window.upgrade()
            {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        },
    );

    println!("Petunia3D window ready");
    let result = window.run();

    drop(airbrush_timer);
    drop(autosave_timer);
    let path = bridge
        .lock()
        .map(|bridge| bridge.state.project.project_path.clone())
        .unwrap_or(None);
    petunia_core::AutosaveService::remove_session_lock(path.as_deref().map(std::path::Path::new));
    result
}

/// Parâmetros numéricos expostos de um efeito de camada.
fn effect_params(effect: &petunia_project::paint_layers::PaintEffect) -> Vec<PaintEffectParam> {
    use petunia_project::paint_layers::PaintEffect;
    let param = |key: &str, label: &str, value: f32, min: f32, max: f32| PaintEffectParam {
        key: key.to_string(),
        label: label.to_string(),
        value,
        min,
        max,
    };
    match effect {
        PaintEffect::Pixelate { cell_size } => {
            vec![param(
                "cell_size",
                "Cell size",
                *cell_size as f32,
                1.0,
                64.0,
            )]
        }
        PaintEffect::Posterize { levels } => {
            vec![param("levels", "Levels", *levels as f32, 2.0, 32.0)]
        }
        PaintEffect::Invert => Vec::new(),
        PaintEffect::Grain { intensity, seed } => vec![
            param("intensity", "Intensity", *intensity, 0.0, 1.0),
            param("seed", "Seed", *seed as f32, 0.0, 9_999.0),
        ],
        PaintEffect::Levels { .. } => Vec::new(),
        PaintEffect::BrightnessContrast {
            brightness,
            contrast,
        } => vec![
            param("brightness", "Brightness", *brightness, -1.0, 1.0),
            param("contrast", "Contrast", *contrast, -1.0, 1.0),
        ],
        PaintEffect::HueSaturation {
            hue_shift_deg,
            saturation,
        } => vec![
            param("hue_shift_deg", "Hue shift", *hue_shift_deg, -180.0, 180.0),
            param("saturation", "Saturation", *saturation, -1.0, 1.0),
        ],
    }
}

fn sync_overlay_models(
    window: &PetuniaSlintShell,
    selection: &SelectionOverlayModel,
    gizmo: &GizmoModel,
) {
    window.set_selection_overlay_visible(selection.visible);
    window.set_selection_outline_commands(selection.outline_commands.as_str().into());
    window.set_selection_active_outline_commands(selection.active_outline_commands.as_str().into());
    window.set_selection_point_commands(selection.point_commands.as_str().into());
    window.set_selection_unselected_outline_commands(
        selection.unselected_outline_commands.as_str().into(),
    );
    window.set_selection_unselected_point_commands(
        selection.unselected_point_commands.as_str().into(),
    );
    window.set_selection_overlay_accent(selection.accent);
    window.set_gizmo_visible(gizmo.visible);
    window.set_gizmo_origin_x(gizmo.origin_x);
    window.set_gizmo_origin_y(gizmo.origin_y);
    window.set_gizmo_x_commands(gizmo.x_commands.as_str().into());
    window.set_gizmo_y_commands(gizmo.y_commands.as_str().into());
    window.set_gizmo_z_commands(gizmo.z_commands.as_str().into());
    window.set_gizmo_x_arrow_commands(gizmo.x_arrow_commands.as_str().into());
    window.set_gizmo_y_arrow_commands(gizmo.y_arrow_commands.as_str().into());
    window.set_gizmo_z_arrow_commands(gizmo.z_arrow_commands.as_str().into());
    window.set_gizmo_x_scale_commands(gizmo.x_scale_commands.as_str().into());
    window.set_gizmo_y_scale_commands(gizmo.y_scale_commands.as_str().into());
    window.set_gizmo_z_scale_commands(gizmo.z_scale_commands.as_str().into());
    window.set_gizmo_x_rotate_commands(gizmo.x_rotate_commands.as_str().into());
    window.set_gizmo_y_rotate_commands(gizmo.y_rotate_commands.as_str().into());
    window.set_gizmo_z_rotate_commands(gizmo.z_rotate_commands.as_str().into());
    window.set_view_gizmo_x_commands(gizmo.view_x_commands.as_str().into());
    window.set_view_gizmo_y_commands(gizmo.view_y_commands.as_str().into());
    window.set_view_gizmo_z_commands(gizmo.view_z_commands.as_str().into());
    window.set_view_gizmo_x_end_x(gizmo.view_x_end[0]);
    window.set_view_gizmo_x_end_y(gizmo.view_x_end[1]);
    window.set_view_gizmo_y_end_x(gizmo.view_y_end[0]);
    window.set_view_gizmo_y_end_y(gizmo.view_y_end[1]);
    window.set_view_gizmo_z_end_x(gizmo.view_z_end[0]);
    window.set_view_gizmo_z_end_y(gizmo.view_z_end[1]);
    window.set_view_gizmo_origin_x(gizmo.view_origin_x);
    window.set_view_gizmo_origin_y(gizmo.view_origin_y);
}

fn sync_viewport_overlays<V: PetuniaViewport>(
    window: &PetuniaSlintShell,
    bridge: &SlintUiBridge<V>,
) {
    let [width, height] = bridge.viewport_size;
    let selection = compute_selection_overlay(
        &bridge.state,
        width,
        height,
        bridge.viewport.draws_component_guides(),
    );
    let gizmo = compute_gizmo(&bridge.state, width, height);
    sync_overlay_models(window, &selection, &gizmo);
    // Cordão da ferramenta ativa: arrasto na viewport, modal de teclado
    // (Move/Rotate/Scale/Extrude/...) ou modal paramétrico com Tool Props.
    let link_active = bridge.drag.is_some()
        || bridge.tool_modal.is_some()
        || bridge.state.session.tools.modal.is_some();
    let drag_link = compute_drag_link(
        &bridge.state,
        width,
        height,
        bridge.pointer_position,
        link_active,
    );
    window.set_drag_link_commands(drag_link.as_str().into());
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
    window.set_shading_mode(vm.shading_mode.as_str().into());
    window.set_show_xray(vm.show_xray);
    window.set_shading_popover_open(vm.shading_popover_open);
    window.set_transform_instant_active(vm.transform_instant_active);
    window.set_gizmo_hover_axis(vm.gizmo_hover_axis);
    window.set_gizmo_active_axis(vm.gizmo_active_axis);
    window.set_gizmo_constraint_axis_a(vm.gizmo_constraint_axes[0]);
    window.set_gizmo_constraint_axis_b(vm.gizmo_constraint_axes[1]);
    window.set_operation_preview_commands(vm.operation_preview_commands.as_str().into());
    window.set_drag_link_commands(vm.drag_link_commands.as_str().into());
    window.set_operation_hud_active(vm.operation_hud_active);
    window.set_operation_hud_title(vm.operation_hud_title.as_str().into());
    window.set_operation_hud_subject(vm.operation_hud_subject.as_str().into());
    window.set_operation_hud_hint(vm.operation_hud_hint.as_str().into());
    window.set_context_hint(vm.context_hint.as_str().into());
    window.set_hover_label(vm.hover_label.as_str().into());
    window.set_selection_summary(vm.selection_summary.as_str().into());
    let hud_lines: Vec<slint::SharedString> = vm
        .operation_hud_lines
        .iter()
        .map(|line| line.as_str().into())
        .collect();
    window.set_operation_hud_lines(hud_lines.as_slice().into());
    window.set_xray_opacity(vm.xray_opacity);
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
    window.set_object_has_selection(vm.object_has_selection);
    window.set_object_id(vm.object_id.as_str().into());
    window.set_object_name(vm.object_name.as_str().into());
    window.set_object_visible(vm.object_visible);
    window.set_object_locked(vm.object_locked);
    window.set_object_verts(vm.object_verts);
    window.set_object_faces(vm.object_faces);
    window.set_object_tris(vm.object_tris);
    window.set_object_selection(vm.object_selection.as_str().into());
    window.set_object_material(vm.object_material.as_str().into());
    window.set_object_modifier_count(vm.object_modifier_count);
    window.set_scene_stats(vm.scene_stats.as_str().into());
    window.set_uv_stats(vm.uv_stats.as_str().into());
    window.set_current_theme(vm.current_theme.as_str().into());

    let to_scene_item = |item: &SceneItemModel| SceneItem {
        id: item.id.as_str().into(),
        name: item.name.as_str().into(),
        visible: item.visible,
        locked: item.locked,
        selected: item.selected,
        active: item.active,
        verts: item.verts as i32,
        tris: item.tris as i32,
    };
    let scene_items: Vec<SceneItem> = vm.scene_items.iter().map(to_scene_item).collect();
    let model = std::rc::Rc::new(slint::VecModel::from(scene_items));
    window.set_scene_items(model.into());
    let parts_items: Vec<SceneItem> = vm.parts_items.iter().map(to_scene_item).collect();
    window.set_parts_items(std::rc::Rc::new(slint::VecModel::from(parts_items)).into());
    window.set_parts_query(vm.parts_query.as_str().into());
    window.set_parts_selected_only(vm.parts_selected_only);
    window.set_parts_sort_by_name(vm.parts_sort_by_name);
    window.set_parts_row_height(vm.parts_row_height);
    let asset_items: Vec<SceneItem> = vm.asset_items.iter().map(to_scene_item).collect();
    window.set_asset_items(std::rc::Rc::new(slint::VecModel::from(asset_items)).into());
    window.set_asset_query(vm.asset_query.as_str().into());
    window.set_asset_sort_by_name(vm.asset_sort_by_name);
    window.set_asset_thumbnail_size(vm.asset_thumbnail_size);
    window.set_show_face_orientation(vm.show_face_orientation);
    window.set_show_uv_checker(vm.show_uv_checker);
    window.set_proportional_editing(vm.proportional_editing);
    window.set_proportional_radius(vm.proportional_radius);
    window.set_proportional_falloff(vm.proportional_falloff.as_str().into());
    window.set_snap_enabled(vm.snap_enabled);
    window.set_snap_target(vm.snap_target.as_str().into());

    sync_overlay_models(window, &vm.selection_overlay, &vm.gizmo);
    window.set_add_menu_open(vm.add_menu_open);
    window.set_rename_active(vm.rename_active);
    window.set_rename_value(vm.rename_value.as_str().into());
    window.set_context_menu_open(vm.context_menu_open);
    window.set_context_menu_x(vm.context_menu_x);
    window.set_context_menu_y(vm.context_menu_y);
    window.set_context_menu_title(vm.context_menu_title.as_str().into());
    window.set_context_menu_mode(vm.context_menu_mode.as_str().into());
    window.set_context_menu_visible(vm.context_menu_visible);
    window.set_context_menu_locked(vm.context_menu_locked);
    window.set_boolean_operand_name(vm.boolean_operand_name.as_str().into());
    window.set_boolean_ready(vm.boolean_ready);
    window.set_boolean_keep_parts(vm.boolean_keep_parts);
    window.set_menu_open(vm.menu_open.as_str().into());
    window.set_pivot_menu_open(vm.pivot_menu_open);
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
    window.set_label_vertical_tool_drag(vm.label_vertical_tool_drag.as_str().into());
    window.set_label_invert_vertical_drag(vm.label_invert_vertical_drag.as_str().into());
    window.set_label_search_assets(vm.label_search_assets.as_str().into());
    window.set_label_search_parts(vm.label_search_parts.as_str().into());
    window.set_label_inspector(vm.label_inspector.as_str().into());
    window.set_label_resize_panel_width(vm.label_resize_panel_width.as_str().into());
    window.set_label_expand_inspector(vm.label_expand_inspector.as_str().into());
    window.set_label_collapse_inspector(vm.label_collapse_inspector.as_str().into());
    window.set_label_tab_parts(vm.label_tab_parts.as_str().into());
    window.set_label_tab_transform(vm.label_tab_transform.as_str().into());
    window.set_label_tab_material(vm.label_tab_material.as_str().into());
    window.set_label_tab_object(vm.label_tab_object.as_str().into());
    window.set_label_tab_modifiers(vm.label_tab_modifiers.as_str().into());
    window.set_label_object_name(vm.label_object_name.as_str().into());
    window.set_label_object_visibility(vm.label_object_visibility.as_str().into());
    window.set_label_object_lock(vm.label_object_lock.as_str().into());
    window.set_label_object_no_selection(vm.label_object_no_selection.as_str().into());
    window.set_label_stats_faces(vm.label_stats_faces.as_str().into());
    window.set_label_stats_verts(vm.label_stats_verts.as_str().into());
    window.set_label_stats_tris(vm.label_stats_tris.as_str().into());
    window.set_label_stats_selection(vm.label_stats_selection.as_str().into());
    window.set_label_tool_options(vm.label_tool_options.as_str().into());
    window.set_label_tool_options_expand(vm.label_tool_options_expand.as_str().into());
    window.set_label_tool_options_collapse(vm.label_tool_options_collapse.as_str().into());
    window.set_label_quick_actions(vm.label_quick_actions.as_str().into());
    window.set_label_quick_action_customize(vm.label_quick_action_customize.as_str().into());
    window.set_label_quick_action_add(vm.label_quick_action_add.as_str().into());
    window.set_label_quick_action_remove(vm.label_quick_action_remove.as_str().into());
    window.set_label_quick_action_reset(vm.label_quick_action_reset.as_str().into());
    window.set_label_quick_action_done(vm.label_quick_action_done.as_str().into());
    window.set_label_action_subdivide(vm.label_action_subdivide.as_str().into());
    window.set_label_action_fuse(vm.label_action_fuse.as_str().into());
    window.set_label_action_cut(vm.label_action_cut.as_str().into());
    window.set_label_action_intersect(vm.label_action_intersect.as_str().into());
    window.set_label_action_join(vm.label_action_join.as_str().into());
    window.set_label_action_merge(vm.label_action_merge.as_str().into());
    window.set_label_action_slice(vm.label_action_slice.as_str().into());
    window.set_label_action_loop_cut(vm.label_action_loop_cut.as_str().into());
    window.set_label_material_base_color(vm.label_material_base_color.as_str().into());
    window.set_label_material_profile(vm.label_material_profile.as_str().into());
    window.set_label_material_roughness(vm.label_material_roughness.as_str().into());
    window.set_label_material_metallic(vm.label_material_metallic.as_str().into());
    window.set_label_material_normal_scale(vm.label_material_normal_scale.as_str().into());
    window.set_label_material_advanced(vm.label_material_advanced.as_str().into());
    window.set_label_material_assign(vm.label_material_assign.as_str().into());
    window.set_label_material_new(vm.label_material_new.as_str().into());
    window.set_label_material_duplicate(vm.label_material_duplicate.as_str().into());
    window.set_label_material_remove(vm.label_material_remove.as_str().into());
    window.set_label_material_no_material(vm.label_material_no_material.as_str().into());
    window.set_label_material_no_selection(vm.label_material_no_selection.as_str().into());
    window
        .set_label_material_emission_strength(vm.label_material_emission_strength.as_str().into());
    window.set_label_material_alpha_cutoff(vm.label_material_alpha_cutoff.as_str().into());
    window.set_label_material_texture_albedo(vm.label_material_texture_albedo.as_str().into());
    window.set_label_material_no_texture(vm.label_material_no_texture.as_str().into());
    window.set_label_material_create_texture(vm.label_material_create_texture.as_str().into());
    window.set_label_material_clear_texture(vm.label_material_clear_texture.as_str().into());
    window.set_label_material_profile_pbr(vm.label_material_profile_pbr.as_str().into());
    window.set_label_material_profile_unlit(vm.label_material_profile_unlit.as_str().into());
    window.set_label_material_profile_toon(vm.label_material_profile_toon.as_str().into());
    window.set_label_material_profile_glass(vm.label_material_profile_glass.as_str().into());
    window.set_label_material_profile_emissive(vm.label_material_profile_emissive.as_str().into());
    window.set_label_material_alpha_opaque(vm.label_material_alpha_opaque.as_str().into());
    window.set_label_material_alpha_mask(vm.label_material_alpha_mask.as_str().into());
    window.set_label_material_alpha_blend(vm.label_material_alpha_blend.as_str().into());
    window.set_label_modifier_mirror(vm.label_modifier_mirror.as_str().into());
    window.set_label_modifier_symmetry(vm.label_modifier_symmetry.as_str().into());
    window.set_label_modifiers_empty(vm.label_modifiers_empty.as_str().into());
    window.set_label_modifier_apply(vm.label_modifier_apply.as_str().into());
    window.set_label_modifier_axis(vm.label_modifier_axis.as_str().into());
    window.set_label_modifier_add_mirror(vm.label_modifier_add_mirror.as_str().into());
    window.set_label_modifier_add_symmetry(vm.label_modifier_add_symmetry.as_str().into());
    window.set_label_modifier_remove(vm.label_modifier_remove.as_str().into());
    window.set_label_modifier_move_up(vm.label_modifier_move_up.as_str().into());
    window.set_label_modifier_move_down(vm.label_modifier_move_down.as_str().into());
    window.set_label_modifier_direction(vm.label_modifier_direction.as_str().into());
    window.set_label_modifier_positive_to_negative(
        vm.label_modifier_positive_to_negative.as_str().into(),
    );
    window.set_label_modifier_negative_to_positive(
        vm.label_modifier_negative_to_positive.as_str().into(),
    );
    window.set_label_numeric_field_hint(vm.label_numeric_field_hint.as_str().into());
    window.set_label_model_select(vm.label_model_select.as_str().into());
    window.set_label_model_position(vm.label_model_position.as_str().into());
    window.set_label_model_rotate(vm.label_model_rotate.as_str().into());
    window.set_label_model_scale(vm.label_model_scale.as_str().into());
    window.set_label_model_transform(vm.label_model_transform.as_str().into());
    window.set_label_model_lasso(vm.label_model_lasso.as_str().into());
    window.set_label_model_loop_cut(vm.label_model_loop_cut.as_str().into());
    window.set_label_model_slice(vm.label_model_slice.as_str().into());
    window.set_label_model_push_pull(vm.label_model_push_pull.as_str().into());
    window.set_label_model_profile(vm.label_model_profile.as_str().into());
    window.set_label_model_pivot(vm.label_model_pivot.as_str().into());
    window.set_label_profile_depth(vm.label_profile_depth.as_str().into());
    window.set_label_profile_points(vm.label_profile_points.as_str().into());
    window.set_label_profile_close(vm.label_profile_close.as_str().into());
    window.set_label_profile_generate(vm.label_profile_generate.as_str().into());
    window.set_label_profile_revolve(vm.label_profile_revolve.as_str().into());
    window.set_label_profile_cuts(vm.label_profile_cuts.as_str().into());
    window.set_label_profile_presets(vm.label_profile_presets.as_str().into());
    window.set_label_profile_add_rect(vm.label_profile_add_rect.as_str().into());
    window.set_label_profile_add_circle(vm.label_profile_add_circle.as_str().into());
    window.set_label_profile_canvas_hint(vm.label_profile_canvas_hint.as_str().into());
    window.set_hint_model_select(vm.hint_model_select.as_str().into());
    window.set_hint_model_position(vm.hint_model_position.as_str().into());
    window.set_hint_model_rotate(vm.hint_model_rotate.as_str().into());
    window.set_hint_model_scale(vm.hint_model_scale.as_str().into());
    window.set_hint_model_transform(vm.hint_model_transform.as_str().into());
    window.set_hint_model_lasso(vm.hint_model_lasso.as_str().into());
    window.set_hint_model_loop_cut(vm.hint_model_loop_cut.as_str().into());
    window.set_hint_model_slice(vm.hint_model_slice.as_str().into());
    window.set_hint_model_push_pull(vm.hint_model_push_pull.as_str().into());
    window.set_hint_model_profile(vm.hint_model_profile.as_str().into());
    window.set_hint_model_pivot(vm.hint_model_pivot.as_str().into());
    window.set_label_hide_part(vm.label_hide_part.as_str().into());
    window.set_label_show_part(vm.label_show_part.as_str().into());
    window.set_label_lock_part(vm.label_lock_part.as_str().into());
    window.set_label_unlock_part(vm.label_unlock_part.as_str().into());
    window.set_label_selected_parts_only(vm.label_selected_parts_only.as_str().into());
    window.set_label_sort_parts(vm.label_sort_parts.as_str().into());
    window.set_label_parts_row_size(vm.label_parts_row_size.as_str().into());
    window.set_label_sort_assets(vm.label_sort_assets.as_str().into());
    window.set_label_thumbnail_size(vm.label_thumbnail_size.as_str().into());
    window.set_label_selection_color(vm.label_selection_color.as_str().into());
    window.set_label_highlight_thickness(vm.label_highlight_thickness.as_str().into());
    window.set_label_view_wireframe(vm.label_view_wireframe.as_str().into());
    window.set_label_view_wireframe_hint(vm.label_view_wireframe_hint.as_str().into());
    window.set_label_view_solid(vm.label_view_solid.as_str().into());
    window.set_label_view_solid_hint(vm.label_view_solid_hint.as_str().into());
    window.set_label_view_material(vm.label_view_material.as_str().into());
    window.set_label_view_material_hint(vm.label_view_material_hint.as_str().into());
    window.set_label_view_lit(vm.label_view_lit.as_str().into());
    window.set_label_view_lit_hint(vm.label_view_lit_hint.as_str().into());
    window.set_label_more_model_tools(vm.label_more_model_tools.as_str().into());
    window.set_label_xray_opacity(vm.label_xray_opacity.as_str().into());
    window.set_label_wire_overlay(vm.label_wire_overlay.as_str().into());
    window.set_label_wire_overlay_hint(vm.label_wire_overlay_hint.as_str().into());
    window.set_selection_color(slint::Color::from_rgb_u8(
        vm.selection_rgb[0],
        vm.selection_rgb[1],
        vm.selection_rgb[2],
    ));
    window.set_selection_color_hex(vm.selection_color_hex.as_str().into());
    window.set_selection_thickness(vm.selection_thickness);
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
    window.set_uv_selected_count(vm.uv_editor.uv_selected_count as i32);
    window.set_uv_layout_truncated(vm.uv_editor.truncated);
    window.set_paint_layer_count(vm.paint_layer_count.as_str().into());
    window.set_paint_effect_kind(vm.paint_effect_kind.as_str().into());
    let effect_params: Vec<PaintEffectParamEntry> = vm
        .paint_effect_params
        .iter()
        .map(|param| PaintEffectParamEntry {
            key: param.key.as_str().into(),
            label: param.label.as_str().into(),
            value: param.value,
            minimum: param.min,
            maximum: param.max,
        })
        .collect();
    window.set_paint_effect_params(effect_params.as_slice().into());
    window.set_paint_canvas_size(vm.paint_canvas_size.as_str().into());
    window.set_paint_canvas_revision(vm.paint_canvas_revision);
    window.set_paint_fill_scope(vm.paint_fill_scope.as_str().into());
    window.set_paint_projection(vm.paint_projection.as_str().into());
    window.set_paint_lock(vm.paint_lock.as_str().into());
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
    window.set_loop_cut_preview_commands(vm.loop_cut_preview_commands.as_str().into());
    window.set_loop_cut_armed(vm.loop_cut_armed);
    window.set_pivot_id(vm.pivot_id.as_str().into());
    window.set_pivot_label(vm.pivot_label.as_str().into());
    window.set_pivot_median_label(vm.pivot_median_label.as_str().into());
    window.set_pivot_bounds_label(vm.pivot_bounds_label.as_str().into());
    window.set_pivot_cursor_label(vm.pivot_cursor_label.as_str().into());
    window.set_pivot_individual_label(vm.pivot_individual_label.as_str().into());
    window.set_profile_active(vm.profile_active);
    window.set_profile_point_count(vm.profile_point_count);
    window.set_profile_closed(vm.profile_closed);
    window.set_profile_preview_commands(vm.profile_preview_commands.as_str().into());
    window.set_profile_depth(vm.profile_depth);
    window.set_tool_activation(vm.tool_activation.as_str().into());
    window.set_keyboard_tool_modal_active(vm.keyboard_tool_modal_active);
    window.set_invert_vertical_drag(vm.invert_vertical_drag);
    window.set_tool_modal_active(vm.tool_modal_active);
    window.set_tool_options_active(vm.tool_options_active);
    window.set_tool_options_title(vm.tool_options_title.as_str().into());
    window.set_tool_options_hint(vm.tool_options_hint.as_str().into());
    window.set_tool_modal_title(vm.tool_modal_title.as_str().into());
    window.set_tool_modal_label(vm.tool_modal_label.as_str().into());
    window.set_tool_modal_value(vm.tool_modal_value);
    window.set_tool_modal_step(vm.tool_modal_step);
    window.set_tool_modal_min(vm.tool_modal_min);
    window.set_tool_modal_max(vm.tool_modal_max);

    let material_slots: Vec<slint::SharedString> = vm
        .material_slots
        .iter()
        .map(|slot| slot.as_str().into())
        .collect();
    window.set_material_slots(material_slots.as_slice().into());
    window.set_active_material_slot(vm.active_material_slot);
    window.set_material_has_selection(vm.material_has_selection);
    window.set_material_id(vm.material_id.as_str().into());
    window.set_material_name(vm.material_name.as_str().into());
    window.set_material_profile(vm.material_profile.as_str().into());
    window.set_material_profile_label(vm.material_profile_label.as_str().into());
    window.set_material_base_color(slint::Color::from_argb_f32(
        1.0,
        vm.material_base_color[0],
        vm.material_base_color[1],
        vm.material_base_color[2],
    ));
    let material_palette: Vec<slint::Color> = vm
        .material_palette
        .iter()
        .map(|color| slint::Color::from_argb_f32(1.0, color[0], color[1], color[2]))
        .collect();
    window.set_material_palette(material_palette.as_slice().into());
    window.set_material_roughness(vm.material_roughness);
    window.set_material_metallic(vm.material_metallic);
    window.set_material_normal_scale(vm.material_normal_scale);
    window.set_material_emission(slint::Color::from_argb_f32(
        1.0,
        vm.material_emission[0],
        vm.material_emission[1],
        vm.material_emission[2],
    ));
    window.set_material_emission_strength(vm.material_emission_strength);
    window.set_material_alpha_mode(vm.material_alpha_mode.as_str().into());
    window.set_material_alpha_cutoff(vm.material_alpha_cutoff);
    window.set_material_has_albedo(vm.material_has_albedo);
    window.set_material_albedo_label(vm.material_albedo_label.as_str().into());

    let quick_actions: Vec<QuickActionEntry> = vm
        .quick_actions
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    window.set_quick_actions(std::rc::Rc::new(slint::VecModel::from(quick_actions)).into());
    let quick_action_candidates: Vec<QuickActionEntry> = vm
        .quick_action_candidates
        .iter()
        .map(|action| QuickActionEntry {
            id: action.id.as_str().into(),
            label: action.label.as_str().into(),
            enabled: action.enabled,
            pinned: action.pinned,
        })
        .collect();
    window.set_quick_action_candidates(
        std::rc::Rc::new(slint::VecModel::from(quick_action_candidates)).into(),
    );
    let modifier_rows: Vec<ModifierEntry> = vm
        .modifier_rows
        .iter()
        .map(|modifier| ModifierEntry {
            id: modifier.id.as_str().into(),
            title: modifier.title.as_str().into(),
            subtitle: modifier.subtitle.as_str().into(),
            enabled: modifier.enabled,
            kind: modifier.kind.as_str().into(),
            axis: modifier.axis,
            positive_to_negative: modifier.positive_to_negative,
            can_move_up: modifier.can_move_up,
            can_move_down: modifier.can_move_down,
        })
        .collect();
    window.set_modifier_rows(std::rc::Rc::new(slint::VecModel::from(modifier_rows)).into());
    window.set_paint_pixel_grid(vm.paint_pixel_grid);
    window.set_paint_canvas_zoom(vm.paint_canvas_zoom);

    theme::apply_theme(window, &vm.current_theme);
}

fn persist_user_preferences<V: PetuniaViewport>(bridge: &mut SlintUiBridge<V>) {
    let preferences = petunia_config::UserPreferences {
        invert_vertical_drag: bridge.state.ui.invert_vertical_drag,
        selection_rgb: bridge.state.ui.selection_rgb,
        selection_thickness: bridge.state.ui.selection_thickness,
        model_quick_actions: bridge.state.ui.model_quick_actions.clone(),
    };
    if let Err(error) = preferences.save() {
        let message = bridge
            .state
            .t_id(petunia_config::text_id::UI_PREFERENCES_SAVE_FAILED);
        bridge.state.set_status(format!("{message}: {error}"));
    }
}

/// Mantém o destaque configurável legível sobre o canvas escuro oficial.
fn selection_color_has_contrast(rgb: [u8; 3]) -> bool {
    let luminance = |channels: [u8; 3]| {
        let linear = channels.map(|channel| {
            let value = channel as f32 / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
    };
    let backdrop = luminance([16, 17, 20]);
    let foreground = luminance(rgb);
    // WCAG 3:1 para indicadores não textuais.
    (foreground + 0.05) / (backdrop + 0.05) >= 3.0
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
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
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
                "palette.import" => service.import_palette().await,
                "palette.export" => service.export_palette().await,
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
                    "palette.import" => UiIntent::ImportPalette(path),
                    "palette.export" => UiIntent::ExportPalette(path),
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
        if let Ok(mut bridge) = scene_bridge.lock()
            && let Some(window) = window_weak.upgrade()
        {
            if window.get_compact_shell() {
                bridge.apply(UiIntent::ToggleSceneDrawer);
            } else {
                if bridge.scene_drawer_visible {
                    bridge.apply(UiIntent::ToggleSceneDrawer);
                }
                window.set_model_parts_open(true);
            }
            let vm = bridge.view_model();
            window.set_scene_drawer_visible(bridge.scene_drawer_visible);
            sync_window_properties(&window, &vm);
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
            if let Some(command) = CommandId::from_id_str(id_str.as_str()) {
                if command == CommandId::ToggleSceneDrawer
                    && let Some(window) = window_weak.upgrade()
                    && !window.get_compact_shell()
                {
                    if bridge.scene_drawer_visible {
                        bridge.apply(UiIntent::ToggleSceneDrawer);
                    }
                    window.set_model_parts_open(true);
                } else {
                    bridge.execute_command(command);
                }
            } else if let Err(error) = bridge.execute_core_command(id_str.as_str()) {
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
                window.set_menu_open(bridge.view_model().menu_open.into());
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
                window.set_menu_open(bridge.view_model().menu_open.into());
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

    let box_bridge = Arc::clone(&bridge);
    let box_window = window.as_weak();
    window.on_viewport_box_select(move |x0, y0, x1, y1, add, subtract| {
        if let Ok(mut bridge) = box_bridge.lock() {
            bridge.state.select_viewport_box(
                [x0 * 2.0 - 1.0, 1.0 - y0 * 2.0],
                [x1 * 2.0 - 1.0, 1.0 - y1 * 2.0],
                add,
                subtract,
            );
            // Box mudo é box confuso: dizer o que entrou na seleção fecha o
            // ciclo de feedback do gesto (Blender mostra a contagem na barra).
            let summary = match bridge.state.selection_domain() {
                SelectionDomain::Object => {
                    let total = bridge.state.session.selection.assets.len();
                    if total == 0 {
                        "Box select: nothing in the region".to_string()
                    } else {
                        format!("Box select: {total} object(s)")
                    }
                }
                SelectionDomain::Vertex | SelectionDomain::Edge | SelectionDomain::Face => {
                    match bridge.state.project.active_mesh() {
                        Some(mesh) => {
                            let points = mesh.verts.iter().filter(|v| v.selected).count();
                            let faces = mesh.faces.iter().filter(|f| f.selected).count();
                            let edges = mesh.selected_edges.len();
                            match bridge.state.selection_domain() {
                                SelectionDomain::Vertex => {
                                    if points == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {points} point(s)")
                                    }
                                }
                                SelectionDomain::Edge => {
                                    if edges == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {edges} edge(s)")
                                    }
                                }
                                _ => {
                                    if faces == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {faces} face(s)")
                                    }
                                }
                            }
                        }
                        None => "Box select: no active object".to_string(),
                    }
                }
            };
            bridge.state.set_status(summary);
            if let Some(window) = box_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let lasso_bridge = Arc::clone(&bridge);
    let lasso_window = window.as_weak();
    window.on_viewport_lasso_select(move |path, add, subtract| {
        let Some(polygon) = parse_lasso_path(path.as_str()) else {
            return;
        };
        if let Ok(mut bridge) = lasso_bridge.lock() {
            bridge.state.select_viewport_lasso(&polygon, add, subtract);
            bridge.state.set_status("Lasso selection updated");
            if let Some(window) = lasso_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
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
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade()
                && let Some(frame) = frame
            {
                window.set_viewport_image(frame);
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
            _ => return false,
        };
        let Ok(mut bridge) = transform_text_bridge.lock() else {
            return false;
        };
        let result = bridge.commit_transform_text(kind, axis as usize, text.as_str());
        if let Err(error) = result {
            bridge
                .state
                .set_status(format!("Invalid numeric value: {error:?}"));
        }
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result.is_ok()
    });

    let transform_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_transform_scrub_cancelled(move || {
        if let Ok(mut bridge) = transform_cancel_bridge.lock() {
            bridge.cancel_transform();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let orbit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_orbit(move |dx, dy| {
        if let Ok(mut bridge) = orbit_bridge.lock() {
            if bridge.mouse_navigation_suspended() {
                return;
            }
            bridge.orbit_viewport(dx, dy);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let pan_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_pan(move |dx, dy| {
        if let Ok(mut bridge) = pan_bridge.lock() {
            if bridge.mouse_navigation_suspended() {
                return;
            }
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_zoom(move |delta| {
        if let Ok(mut bridge) = zoom_bridge.lock() {
            if bridge.mouse_navigation_suspended() {
                return;
            }
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom { delta }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
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
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let viewport_select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_select(move |x, y, extend, loop_select| {
        if let Ok(mut bridge) = viewport_select_bridge.lock() {
            bridge.select_viewport_ext(x, y, extend, loop_select);
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
            // O plano de corte reusa o mesmo canal de arrasto, mas com a própria
            // sessão: nada de transformar geometria.
            "slice" => {
                if let Ok(mut bridge) = transform_begin_bridge.lock() {
                    bridge.begin_slice(x, y);
                }
                return;
            }
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock() {
            bridge.begin_viewport_transform(kind, x, y);
        }
    });

    let transform_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_transform_update(move |x, y, fine, snap| {
        if let Ok(mut bridge) = transform_drag_bridge.lock() {
            bridge.pointer_position = [x, y];
            if bridge.update_viewport_slice(x, y) {
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
                return;
            }
            bridge.update_viewport_transform_modified(x, y, fine, snap);
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
            if bridge.commit_slice() {
                let vm = bridge.view_model();
                let new_frame = bridge.render_viewport();
                if let Some(window) = window_weak.upgrade() {
                    sync_window_properties(&window, &vm);
                    if let Some(frame) = new_frame {
                        window.set_viewport_image(frame);
                    }
                }
                return;
            }
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
    let window_weak = window.as_weak();
    window.on_viewport_paint_begin(move |x, y| {
        if let Ok(mut bridge) = paint_begin_bridge.lock() {
            bridge.begin_paint_stroke_at(x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_update(move |x, y| {
        if let Ok(mut bridge) = paint_update_bridge.lock() {
            bridge.paint_stroke_to(x, y);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_end(move |x, y| {
        if let Ok(mut bridge) = paint_end_bridge.lock() {
            bridge.end_paint_stroke_at(x, y);
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

    let tool_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_hovered(move |delta, x, y| {
        if let Ok(mut bridge) = tool_hover_bridge.lock()
            && bridge.is_instant_tool_mode()
            && bridge.tool_modal.is_some()
        {
            bridge.pointer_position = [x, y];
            bridge.scrub_tool_modal(delta, false);
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

    let activation_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_activation_set(move |id| {
        if let Ok(mut bridge) = activation_bridge.lock() {
            bridge.set_tool_activation(id.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let vertical_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_invert_vertical_drag_set(move |invert| {
        if let Ok(mut bridge) = vertical_drag_bridge.lock() {
            if bridge.set_invert_vertical_drag(invert) {
                persist_user_preferences(&mut bridge);
            }
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let selection_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_color_set(move |hex| {
        if let Ok(mut bridge) = selection_color_bridge.lock() {
            if bridge.set_selection_color_hex(hex.as_str()) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let selection_thickness_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_selection_thickness_set(move |thickness| {
        if let Ok(mut bridge) = selection_thickness_bridge.lock() {
            if bridge.set_selection_thickness(thickness) {
                persist_user_preferences(&mut bridge);
            }
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let parts_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_query_changed(move |query| {
        if let Ok(mut bridge) = parts_query_bridge.lock() {
            bridge.set_parts_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_filter_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_selected_only_changed(move |enabled| {
        if let Ok(mut bridge) = parts_filter_bridge.lock() {
            bridge.set_parts_selected_only(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_sort_changed(move |enabled| {
        if let Ok(mut bridge) = parts_sort_bridge.lock() {
            bridge.set_parts_sort_by_name(enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let parts_size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_parts_row_height_changed(move |height| {
        if let Ok(mut bridge) = parts_size_bridge.lock() {
            bridge.set_parts_row_height(height);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_query_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_query_changed(move |query| {
        if let Ok(mut bridge) = asset_query_bridge.lock() {
            bridge.set_asset_query(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_sort_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_sort_changed(move |sort_by_name| {
        if let Ok(mut bridge) = asset_sort_bridge.lock() {
            bridge.set_asset_sort_by_name(sort_by_name);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let asset_size_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_asset_thumbnail_size_changed(move |size| {
        if let Ok(mut bridge) = asset_size_bridge.lock() {
            bridge.set_asset_thumbnail_size(size);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let tool_scrub_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_tool_modal_scrubbed(move |delta, fine, x, y| {
        if let Ok(mut bridge) = tool_scrub_bridge.lock() {
            bridge.pointer_position = [x, y];
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
        let Ok(mut bridge) = tool_text_bridge.lock() else {
            return false;
        };
        let accepted = match numeric::parse_numeric(text.as_str()) {
            Ok(value) => bridge.set_tool_modal_value(value),
            Err(error) => {
                bridge.state.set_status(format!("Invalid value: {error:?}"));
                false
            }
        };
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
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

    let viewport_context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_context_requested(move |x, y| {
        if let Ok(mut bridge) = viewport_context_bridge.lock() {
            bridge.viewport_context_triage(x, y);
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

    let keep_parts_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_keep_parts_set(move |keep| {
        if let Ok(mut bridge) = keep_parts_bridge.lock() {
            bridge.set_boolean_keep_parts(keep);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let join_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_join_requested(move || {
        if let Ok(mut bridge) = join_bridge.lock() {
            bridge.join_operand();
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

    let effect_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_layer_added(move |kind| {
        if let Ok(mut bridge) = effect_add_bridge.lock() {
            if !bridge.add_paint_effect_layer(kind.as_str()) {
                bridge
                    .state
                    .set_status(format!("Unknown effect layer: {kind}"));
            }
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let effect_param_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_effect_param_set(move |key, value| {
        if let Ok(mut bridge) = effect_param_bridge.lock() {
            bridge.set_paint_effect_param(key.as_str(), value);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let component_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_hover(move |x, y| {
        if let Ok(mut bridge) = component_hover_bridge.lock() {
            if bridge.state.session.tools.active_tool == "loop_cut" {
                let viewport_size = bridge.viewport_size;
                let _ = bridge.update_loop_cut_hover(x * viewport_size[0], y * viewport_size[1]);
            } else {
                let _ = bridge.hover_component(x, y);
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

    let hover_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_hover_clear(move || {
        if let Ok(mut bridge) = hover_clear_bridge.lock()
            && bridge.clear_hover()
        {
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

    let gizmo_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_hover(move |x, y| {
        if let Ok(mut bridge) = gizmo_hover_bridge.lock()
            && bridge.hover_gizmo(x, y)
        {
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let gizmo_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_begin(move |x, y| {
        if let Ok(mut bridge) = gizmo_begin_bridge.lock() {
            bridge.begin_gizmo_drag(x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let gizmo_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_end(move || {
        if let Ok(mut bridge) = gizmo_end_bridge.lock() {
            bridge.end_gizmo_drag();
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

    let shading_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_mode_set(move |id| {
        if let Ok(mut bridge) = shading_bridge.lock() {
            bridge.set_shading_mode(id.as_str());
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

    let xray_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_xray_requested(move || {
        if let Ok(mut bridge) = xray_toggle_bridge.lock() {
            if let Err(error) = bridge.execute_core_command("view.toggle_xray") {
                bridge.state.set_status(error.to_string());
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

    let xray_opacity_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_xray_opacity_set(move |opacity| {
        if let Ok(mut bridge) = xray_opacity_bridge.lock() {
            bridge.set_xray_opacity(opacity);
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

    let shading_popover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_shading_popover_toggled(move |open| {
        if let Ok(mut bridge) = shading_popover_bridge.lock() {
            bridge.shading_popover_open = open;
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let face_orient_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_face_orientation(move || {
        if let Ok(mut bridge) = face_orient_bridge.lock() {
            bridge.toggle_face_orientation();
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

    let uv_checker_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_uv_checker(move || {
        if let Ok(mut bridge) = uv_checker_bridge.lock() {
            bridge.toggle_uv_checker();
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

    let prop_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_proportional_editing(move || {
        if let Ok(mut bridge) = prop_bridge.lock() {
            bridge.toggle_proportional_editing();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_rad_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_radius(move |radius| {
        if let Ok(mut bridge) = prop_rad_bridge.lock() {
            bridge.set_proportional_radius(radius);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_fall_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_proportional_falloff(move |falloff| {
        if let Ok(mut bridge) = prop_fall_bridge.lock() {
            bridge.set_proportional_falloff(falloff.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prop_adj_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_adjust_proportional_radius(move |delta| {
        if let Ok(mut bridge) = prop_adj_bridge.lock() {
            bridge.adjust_proportional_radius(delta);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_snap_enabled(move || {
        if let Ok(mut bridge) = snap_bridge.lock() {
            bridge.toggle_snap_enabled();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let snap_target_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_snap_target(move |target| {
        if let Ok(mut bridge) = snap_target_bridge.lock() {
            bridge.set_snap_target(target.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let prof_rect_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_rectangle(move |width, height| {
        if let Ok(mut bridge) = prof_rect_bridge.lock() {
            bridge.add_profile_rectangle(width, height);
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

    let prof_circle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_add_profile_circle(move |radius, segments| {
        if let Ok(mut bridge) = prof_circle_bridge.lock() {
            bridge.add_profile_circle(radius, segments as usize);
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

    let decal_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_decal_layer_added(move || {
        if let Ok(mut bridge) = decal_bridge.lock() {
            bridge.add_decal_layer();
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

    let view_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_view_axis_clicked(move |axis| {
        if let Ok(mut bridge) = view_axis_bridge.lock() {
            bridge.snap_view_to_axis(axis.as_str());
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

    let operand_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_boolean_operand_cleared(move || {
        if let Ok(mut bridge) = operand_clear_bridge.lock() {
            bridge.clear_boolean_operand();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let fill_scope_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_fill_scope_set(move |scope| {
        if let Ok(mut bridge) = fill_scope_bridge.lock() {
            bridge.set_fill_scope(scope.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let projection_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_projection_set(move |projection| {
        if let Ok(mut bridge) = projection_bridge.lock() {
            bridge.set_brush_projection(projection.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let brush_lock_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_lock_set(move |lock| {
        if let Ok(mut bridge) = brush_lock_bridge.lock() {
            bridge.set_brush_lock(lock.as_str());
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let paint_stroke_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_paint_2d_stroke(move |norm_x, norm_y, phase| {
        if let Ok(mut bridge) = paint_stroke_bridge.lock() {
            bridge.apply(UiIntent::Paint2dStroke {
                norm_x,
                norm_y,
                phase,
            });
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pixel_grid_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_toggle_paint_pixel_grid(move || {
        if let Ok(mut bridge) = pixel_grid_bridge.lock() {
            bridge.apply(UiIntent::TogglePaintPixelGrid);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    let canvas_zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_set_paint_canvas_zoom(move |zoom| {
        if let Ok(mut bridge) = canvas_zoom_bridge.lock() {
            bridge.apply(UiIntent::SetPaintCanvasZoom(zoom));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
            }
        }
    });

    let uv_click_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_editor_clicked(move |u, v, extend| {
        if let Ok(mut bridge) = uv_click_bridge.lock() {
            bridge.uv_editor_click(u, v, extend);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_moved(move |du, dv| {
        if let Ok(mut bridge) = uv_move_bridge.lock() {
            bridge.uv_move_selected(du, dv);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_scale_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_scaled(move |factor| {
        if let Ok(mut bridge) = uv_scale_bridge.lock() {
            bridge.uv_scale_selected(factor);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let uv_rotate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_uv_rotated(move |degrees| {
        if let Ok(mut bridge) = uv_rotate_bridge.lock() {
            bridge.uv_rotate_selected(degrees);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
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

    let loop_slide_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_slide_committed(move |text| {
        let Ok(mut bridge) = loop_slide_bridge.lock() else {
            return false;
        };
        let accepted = match numeric::parse_numeric(text.as_str()) {
            Ok(value) => {
                bridge
                    .loop_cut
                    .as_ref()
                    .is_some_and(|session| session.slide == value)
                    || bridge.set_loop_cut_slide(value)
            }
            Err(_) => {
                bridge
                    .state
                    .set_status("Loop Cut: slide must be between -1 and 1");
                false
            }
        };
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        accepted
    });

    let loop_count_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_count_committed(move |text| {
        if let Ok(mut bridge) = loop_count_bridge.lock() {
            match text.trim().parse::<i32>() {
                Ok(cuts) => {
                    bridge.adjust_loop_cut_count_from_input(cuts.clamp(1, 32) as usize);
                }
                Err(_) => bridge
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

    let pivot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_set(move |id| {
        if let Ok(mut bridge) = pivot_bridge.lock() {
            bridge.set_pivot_point(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let pivot_menu_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_pivot_menu_toggled(move |open| {
        if let Ok(mut bridge) = pivot_menu_bridge.lock() {
            bridge.set_pivot_menu_open(open);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let loop_place_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_loop_cut_place(move || {
        if let Ok(mut bridge) = loop_place_bridge.lock() {
            bridge.place_loop_cut_hover();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_close_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_close(move || {
        if let Ok(mut bridge) = profile_close_bridge.lock() {
            bridge.close_profile();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let profile_depth_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_depth_set(move |text| {
        let Ok(depth) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        if let Ok(mut bridge) = profile_depth_bridge.lock() {
            let accepted = bridge.set_profile_depth(depth);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
            accepted
        } else {
            false
        }
    });

    let profile_generate_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_generate(move || {
        if let Ok(mut bridge) = profile_generate_bridge.lock() {
            bridge.generate_profile_extrude();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let profile_revolve_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_profile_revolve(move || {
        if let Ok(mut bridge) = profile_revolve_bridge.lock() {
            bridge.generate_profile_revolve();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
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
    window.on_scene_select(move |id, extend| {
        if let Ok(mut bridge) = select_bridge.lock() {
            if let Some(index) = bridge
                .state
                .project
                .assets
                .iter()
                .position(|a| a.id.to_string() == id.as_str())
            {
                bridge.cancel_active_operation();
                bridge.state.select_object(Some(index), extend);
                bridge.reset_transform_fields();
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
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let tool_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_active_tool_changed(move |tool| {
        if let Ok(mut bridge) = tool_bridge.lock() {
            bridge.apply(UiIntent::SetActiveTool(tool.as_str().to_string()));
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
            bridge.execute_command(CommandId::ToggleWireOverlay);
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

    let assign_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_assign_material_slot(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = assign_mat_bridge.lock() {
            bridge.apply(UiIntent::AssignMaterialSlot(slot as usize));
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

    let create_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_create_material(move || {
        if let Ok(mut bridge) = create_mat_bridge.lock() {
            bridge.apply(UiIntent::CreateMaterial);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let dup_mat_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_duplicate_material(move |slot| {
        if slot < 0 {
            return;
        }
        if let Ok(mut bridge) = dup_mat_bridge.lock() {
            bridge.apply(UiIntent::DuplicateMaterial(slot as usize));
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let material_slot_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_slot_selected(move |slot| {
        if let Ok(mut bridge) = material_slot_bridge.lock() {
            bridge.select_material_slot(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_color_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_base_color_changed(move |red, green, blue| {
        if let Ok(mut bridge) = material_color_bridge.lock() {
            bridge.set_active_material_base_color(red, green, blue);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_scalar_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_scalar_committed(move |field, slot, text| {
        let Ok(mut bridge) = material_scalar_bridge.lock() else {
            return false;
        };
        let Ok(value) = numeric::parse_numeric(text.as_str()) else {
            return false;
        };
        bridge.select_material_slot(slot);
        let result = bridge.set_active_material_scalar(field.as_str(), value);
        if let Some(window) = window_weak.upgrade() {
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
        result
    });

    let material_profile_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_profile_changed(move |profile| {
        if let Ok(mut bridge) = material_profile_bridge.lock() {
            bridge.set_active_material_profile(profile);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_alpha_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_alpha_mode_changed(move |mode| {
        if let Ok(mut bridge) = material_alpha_bridge.lock() {
            bridge.set_active_material_alpha_mode(mode);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let material_texture_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_create(move || {
        if let Ok(mut bridge) = material_texture_bridge.lock() {
            bridge.create_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let material_texture_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_material_albedo_clear(move || {
        if let Ok(mut bridge) = material_texture_clear_bridge.lock() {
            bridge.clear_albedo_texture();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let remove_material_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_remove_material(move |slot| {
        if let Ok(mut bridge) = remove_material_bridge.lock() {
            bridge.remove_material(slot);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let quick_execute_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_executed(move |id| {
        if let Ok(mut bridge) = quick_execute_bridge.lock() {
            bridge.execute_quick_action(id.as_str());
            let vm = bridge.view_model();
            let frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let quick_action_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_action_toggled(move |id| {
        if let Ok(mut bridge) = quick_action_bridge.lock() {
            bridge.toggle_quick_action(id.as_str());
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let quick_reset_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_quick_actions_reset(move || {
        if let Ok(mut bridge) = quick_reset_bridge.lock() {
            bridge.reset_quick_actions();
            persist_user_preferences(&mut bridge);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });

    let modifier_add_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_add(move |kind| {
        if let Ok(mut bridge) = modifier_add_bridge.lock() {
            bridge.add_modifier(kind.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_toggle_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_toggle(move |id, enabled| {
        if let Ok(mut bridge) = modifier_toggle_bridge.lock() {
            bridge.set_modifier_enabled(id.as_str(), enabled);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_remove_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_remove(move |id| {
        if let Ok(mut bridge) = modifier_remove_bridge.lock() {
            bridge.remove_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_move(move |id, direction| {
        if let Ok(mut bridge) = modifier_move_bridge.lock() {
            bridge.move_modifier(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_apply_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_apply(move |id| {
        if let Ok(mut bridge) = modifier_apply_bridge.lock() {
            bridge.apply_modifier(id.as_str());
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_axis_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_axis_changed(move |id, axis| {
        if let Ok(mut bridge) = modifier_axis_bridge.lock() {
            bridge.set_modifier_axis(id.as_str(), axis);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
    let modifier_direction_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_modifier_direction_changed(move |id, direction| {
        if let Ok(mut bridge) = modifier_direction_bridge.lock() {
            bridge.set_modifier_direction(id.as_str(), direction);
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
            }
        }
    });
}
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn visible_edge_points(
        bridge: &SlintUiBridge<PlaceholderViewport>,
    ) -> Vec<((u32, u32), [f32; 2])> {
        let mesh = bridge.state.project.active_mesh().expect("active mesh");
        mesh.edges_unique()
            .into_iter()
            .filter_map(|(a, b)| {
                let midpoint = (mesh.verts[a as usize].vec() + mesh.verts[b as usize].vec()) * 0.5;
                let ndc = bridge.state.session.camera.project_ndc(midpoint);
                let point = [(ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5];
                let picked =
                    bridge.pick_target_for_domain(SelectionDomain::Edge, point[0], point[1]);
                (picked == petunia_core::HoverTarget::Edge(a, b)).then_some(((a, b), point))
            })
            .collect()
    }

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
        assert_eq!(
            scale_z, -9.0,
            "negative scale mirrors the selected geometry"
        );

        let vm = bridge.view_model();
        assert_eq!(vm.position[0], 0.0);
        assert_eq!(vm.scale[2], -9.0);
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
        assert_eq!(
            bridge.view_model().active_object_title,
            "No Object Selected"
        );
        assert_eq!(bridge.state.project.active, usize::MAX);
        assert!(
            bridge
                .state
                .project
                .assets
                .iter()
                .any(|asset| asset.name == "Cylinder")
        );
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
        assert_eq!(bridge.state.project.assets.len(), 0);
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
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

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
    fn asset_drawer_search_sort_and_zoom_do_not_mutate_the_document() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        bridge.state.mark_document_clean();
        let total = bridge.view_model().scene_items.len();
        assert!(total >= 2);
        assert!(bridge.set_asset_query("sphere"));
        let filtered = bridge.view_model();
        assert_eq!(filtered.asset_items.len(), 1);
        assert!(
            filtered.asset_items[0]
                .name
                .to_lowercase()
                .contains("sphere")
        );
        assert_eq!(filtered.scene_items.len(), total);
        assert!(bridge.set_asset_sort_by_name(true));
        assert!(bridge.set_asset_thumbnail_size(120.0));
        assert_eq!(bridge.view_model().asset_thumbnail_size, 120.0);
        assert!(!bridge.state.is_document_dirty());
        assert!(!bridge.set_asset_thumbnail_size(f32::NAN));
    }

    #[test]
    fn parts_drawer_filters_sorts_and_scales_rows_without_mutation() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        bridge.state.mark_document_clean();
        assert_eq!(bridge.view_model().parts_items.len(), 2);

        assert!(bridge.set_parts_query("cube"));
        assert_eq!(bridge.view_model().parts_items.len(), 1);
        assert!(
            bridge.view_model().parts_items[0]
                .name
                .to_lowercase()
                .contains("cube")
        );
        assert!(bridge.set_parts_query(""));
        assert!(bridge.set_parts_selected_only(true));
        assert_eq!(bridge.view_model().parts_items.len(), 1);
        assert!(bridge.set_parts_selected_only(false));
        assert!(bridge.set_parts_sort_by_name(true));
        let names: Vec<_> = bridge
            .view_model()
            .parts_items
            .iter()
            .map(|item| item.name.clone())
            .collect();
        assert!(
            names
                .windows(2)
                .all(|pair| pair[0].to_lowercase() <= pair[1].to_lowercase())
        );
        assert!(bridge.set_parts_row_height(40.0));
        assert_eq!(bridge.view_model().parts_row_height, 40.0);
        assert!(!bridge.set_parts_row_height(f32::NAN));
        assert!(!bridge.state.is_document_dirty());
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
    fn shortcut_transform_accepts_drag_in_the_same_mouse_gesture() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.pointer_position = [400.0, 300.0];
        let before = bridge.state.project.active_mesh().unwrap().verts.clone();

        assert!(bridge.route_shortcut("G", false, false, false));
        assert!(bridge.view_model().transform_instant_active);
        // O TouchArea entrega estes updates com LMB pressionado, antes do
        // release que confirma; não existe segundo pointer-down.
        assert!(bridge.update_viewport_transform_modified(430.0, 300.0, false, false));
        assert!(bridge.update_viewport_transform_modified(470.0, 300.0, false, false));
        assert!(bridge.end_viewport_transform());
        assert!(!bridge.view_model().transform_instant_active);
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .zip(&before)
                .any(|(after, before)| after.pos != before.pos)
        );
    }

    #[test]
    fn vertical_drag_preference_reverses_transform_without_dirtying_the_document() {
        let mut regular = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let mut inverted = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        regular.resize_viewport(800, 600);
        inverted.resize_viewport(800, 600);
        let origin =
            glam::Vec3::from_array(regular.state.project.active_mesh().unwrap().verts[0].pos);

        assert!(inverted.set_invert_vertical_drag(true));
        assert!(inverted.view_model().invert_vertical_drag);
        assert!(!inverted.state.is_document_dirty());
        assert!(regular.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
        assert!(inverted.begin_viewport_transform(TransformKind::Position, 400.0, 300.0));
        assert!(regular.update_viewport_transform(400.0, 350.0));
        assert!(inverted.update_viewport_transform(400.0, 350.0));

        let normal_delta =
            glam::Vec3::from_array(regular.state.project.active_mesh().unwrap().verts[0].pos)
                - origin;
        let inverted_delta =
            glam::Vec3::from_array(inverted.state.project.active_mesh().unwrap().verts[0].pos)
                - origin;
        assert!(normal_delta.length() > 1.0e-4);
        assert!((normal_delta + inverted_delta).length() < 1.0e-4);
    }

    #[test]
    fn selection_appearance_preferences_validate_without_document_mutation() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.mark_document_clean();
        assert!(bridge.set_selection_color_hex("#20B4F0"));
        assert_eq!(bridge.view_model().selection_rgb, [32, 180, 240]);
        assert_eq!(bridge.view_model().selection_color_hex, "#20B4F0");
        assert!(!bridge.set_selection_color_hex("#xyz"));
        assert!(!bridge.set_selection_color_hex("#000000"));
        assert_eq!(bridge.view_model().selection_rgb, [32, 180, 240]);
        assert!(bridge.set_selection_thickness(4.5));
        assert_eq!(bridge.view_model().selection_thickness, 4.5);
        assert!(!bridge.set_selection_thickness(f32::NAN));
        assert!(!bridge.state.is_document_dirty());
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

        let (_, point) = *visible_edge_points(&bridge)
            .first()
            .expect("visible cube edge");
        bridge.select_viewport(point[0], point[1], false);

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
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetActiveTool("move".to_string()));
        let gizmo = bridge.view_model().gizmo;

        assert!(gizmo.visible);
        assert!((gizmo.origin_x - 512.0).abs() < 64.0);
        assert!((gizmo.origin_y - 384.0).abs() < 64.0);
        for (name, commands, arrows) in [
            ("x", &gizmo.x_commands, &gizmo.x_arrow_commands),
            ("y", &gizmo.y_commands, &gizmo.y_arrow_commands),
            ("z", &gizmo.z_commands, &gizmo.z_arrow_commands),
        ] {
            assert!(
                commands.starts_with("M ") && commands.contains(" L "),
                "haste {name} precisa de segmento: {commands}"
            );
            assert!(
                arrows.starts_with("M ") && arrows.ends_with("Z "),
                "haste {name} precisa de seta fechada: {arrows}"
            );
        }
        // As hastes usam 72 px como máximo e encolhem quando o eixo aponta
        // para a câmera; devem permanecer legíveis e contidas na viewport.
        for commands in [&gizmo.x_commands, &gizmo.y_commands, &gizmo.z_commands] {
            let numbers: Vec<f32> = commands
                .split_whitespace()
                .filter_map(|token| token.parse::<f32>().ok())
                .collect();
            assert_eq!(numbers.len(), 4);
            let length =
                ((numbers[2] - numbers[0]).powi(2) + (numbers[3] - numbers[1]).powi(2)).sqrt();
            assert!(
                (8.0..=72.1).contains(&length),
                "haste projetada fora do intervalo esperado: {length:.1}px: {commands}"
            );
        }
    }

    #[test]
    fn the_view_tripod_marks_all_three_axes_at_top_right() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().gizmo.view_x_commands.is_empty());

        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        let gizmo = bridge.view_model().gizmo;
        for (name, commands) in [
            ("x", &gizmo.view_x_commands),
            ("y", &gizmo.view_y_commands),
            ("z", &gizmo.view_z_commands),
        ] {
            assert!(
                commands.starts_with("M ") && commands.contains(" L "),
                "tripé {name} precisa de segmento: {commands}"
            );
            let numbers: Vec<f32> = commands
                .split_whitespace()
                .filter_map(|token| token.parse::<f32>().ok())
                .collect();
            assert_eq!(numbers.len(), 4);
            let length =
                ((numbers[2] - numbers[0]).powi(2) + (numbers[3] - numbers[1]).powi(2)).sqrt();
            assert!(
                (8.0..=38.1).contains(&length),
                "tripé {name} deve encurtar com a projeção, veio {length:.1}px"
            );
        }
        // O tripé existe mesmo sem ferramenta de transformação: ele mostra a
        // câmera, não a ferramenta.
        assert!(!gizmo.visible);
        assert!((gizmo.view_origin_x - (1024.0 - 54.0)).abs() < 1.0);
        assert!((gizmo.view_origin_y - 108.0).abs() < 1.0);
    }

    #[test]
    fn clicking_a_view_axis_snaps_the_camera() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.snap_view_to_axis("x"));
        assert_eq!(
            bridge.state.session.camera.view_preset(),
            Some(petunia_core::ViewPreset::Right)
        );
        assert!(bridge.snap_view_to_axis("y"));
        assert_eq!(
            bridge.state.session.camera.view_preset(),
            Some(petunia_core::ViewPreset::Top)
        );
        assert!(bridge.snap_view_to_axis("z"));
        assert_eq!(
            bridge.state.session.camera.view_preset(),
            Some(petunia_core::ViewPreset::Front)
        );
        assert!(!bridge.snap_view_to_axis("w"));
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
        assert!(bridge.end_paint_stroke_at(440.0, 300.0));

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

        bridge.toggle_menu("file");
        assert!(bridge.handle_click_away());
        assert_eq!(bridge.view_model().menu_open, "");
    }

    #[test]
    fn wire_overlay_is_independent_of_base_shading_and_xray() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let shading = bridge.state.shading;
        bridge.execute_command(CommandId::ToggleWireOverlay);
        assert!(bridge.state.session.show_wireframe_overlay);
        assert_eq!(bridge.state.shading, shading);
        bridge.execute_command(CommandId::ToggleWireframe);
        assert_eq!(bridge.state.shading, petunia_core::Shading::Wireframe);
        assert!(bridge.state.session.show_wireframe_overlay);
        bridge.execute_core_command("view.toggle_xray").unwrap();
        assert!(bridge.state.session.show_xray);
        assert!(bridge.state.session.show_wireframe_overlay);
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

        // Usa duas arestas opostas e realmente visíveis da mesma face.
        let visible = visible_edge_points(&bridge);
        let mesh = bridge.state.project.active_mesh().unwrap();
        let (first, second) = mesh
            .faces
            .iter()
            .find_map(|face| {
                let edges: Vec<_> = face
                    .verts
                    .iter()
                    .copied()
                    .zip(face.verts.iter().copied().cycle().skip(1))
                    .take(face.verts.len())
                    .map(|(a, b)| (a.min(b), a.max(b)))
                    .collect();
                visible.iter().find_map(|(edge_a, point_a)| {
                    edges
                        .contains(edge_a)
                        .then(|| {
                            visible.iter().find_map(|(edge_b, point_b)| {
                                (edges.contains(edge_b)
                                    && edge_a.0 != edge_b.0
                                    && edge_a.0 != edge_b.1
                                    && edge_a.1 != edge_b.0
                                    && edge_a.1 != edge_b.1)
                                    .then_some((*point_a, *point_b))
                            })
                        })
                        .flatten()
                })
            })
            .expect("two opposite visible edges of one face");
        assert!(
            bridge.knife_click(first[0], first[1]),
            "primeiro ponto precisa ancorar"
        );
        assert_eq!(bridge.state.ui.status, "Knife: pick the second edge point");
        assert_eq!(
            bridge.state.project.undo.depth(),
            (0, 0),
            "ancorar não corta"
        );
        assert!(
            bridge.knife_click(second[0], second[1]),
            "segundo ponto precisa cortar"
        );

        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(
            mesh.faces.len() > faces_before,
            "o corte precisa criar faces: antes {faces_before}, depois {}",
            mesh.faces.len()
        );
        assert!(
            bridge.state.session.tools.cut_session.is_some(),
            "Cut stays open for more segments"
        );
        assert!(bridge.commit_knife());
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
        assert_eq!(bridge.state.ui.status, "Cut cancelled");
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

        // O campo Slide altera a posição do corte, sem modificar Cuts.
        assert!(bridge.set_loop_cut_slide(-0.25));
        assert_eq!(bridge.loop_cut.as_ref().unwrap().slide, -0.25);
        assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 1);
        assert!(!bridge.set_loop_cut_slide(2.0));
        assert_eq!(bridge.loop_cut.as_ref().unwrap().slide, -0.25);

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
    fn loop_cut_hover_previews_without_mutating_until_placed_and_scrolls_count() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        let original = bridge.state.project.active_mesh().unwrap().clone();
        bridge.apply(UiIntent::SetActiveTool("loop_cut".into()));

        let visible = visible_edge_points(&bridge);
        let (_, cursor) = visible[0];
        assert!(bridge.hover_component(cursor[0], cursor[1]));
        assert!(bridge.loop_cut_hover_ring.is_some());
        assert!(!bridge.loop_cut_hover_preview_commands().is_empty());
        assert_eq!(
            bridge.state.project.active_mesh().unwrap().faces.len(),
            original.faces.len()
        );
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert!(bridge.scroll_loop_cut_count(1.0));
        assert_eq!(bridge.loop_cut_hover_cuts, 2);
        assert!(bridge.place_loop_cut_hover());
        assert_eq!(bridge.loop_cut.as_ref().unwrap().cuts, 2);
        assert!(bridge.commit_loop_cut());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.project.active_mesh().unwrap().verts.len() > original.verts.len());
    }

    #[test]
    fn pivot_selector_changes_transform_session_policy() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.set_pivot_point("cursor"));
        assert_eq!(bridge.state.session.pivot_point, PivotPoint::Cursor3D);
        assert_eq!(bridge.view_model().pivot_id, "cursor");
        assert!(bridge.set_pivot_menu_open(true));
        assert_eq!(
            bridge.overlays.top().map(|entry| entry.id),
            Some(OverlayId::PivotMenu)
        );
        assert!(bridge.handle_escape());
        assert!(!bridge.pivot_menu_open);
        assert!(!bridge.set_pivot_point("unknown"));
    }

    #[test]
    fn slice_keeps_both_sides_without_caps_and_can_be_cancelled() {
        let mut state = AppState::default();
        state.set_edit_mode(petunia_core::EditMode::Edit);
        let original = state.project.active_mesh().unwrap().clone();
        let session = petunia_core::CutSession::new(original.clone());
        let camera = state.session.camera.clone();
        let viewport = petunia_core::LogicalRect::from_min_max([0.0, 0.0], [800.0, 600.0]);
        let sliced = session
            .compute_slice(&camera, [300.0, 300.0], [500.0, 300.0], viewport)
            .expect("slice preview");
        assert!(sliced.verts.len() > original.verts.len());
        assert!(sliced.faces.len() > original.faces.len());
        assert!(sliced.verts.iter().all(|vertex| vertex.vec().is_finite()));
    }

    #[test]
    fn loop_cut_tool_cancel_does_not_require_a_hovered_ring() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetActiveTool("loop_cut".into()));
        assert!(bridge.handle_escape());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn profile_uses_view_frame_for_revolve_geometry() {
        let mut state = AppState::default();
        state.profile.origin = [1.0, 2.0, 3.0];
        state.profile.right = [1.0, 0.0, 0.0];
        state.profile.up = [0.0, 0.0, 1.0];
        state.profile.normal = [0.0, -1.0, 0.0];
        state.profile.points = vec![[0.0, 0.0], [0.5, 0.5]];
        state.profile.revolve_segments = 8;
        petunia_module_model::draw_profile::generate_revolve(&mut state);
        let mesh = state.project.active_mesh().expect("revolved profile");
        assert!(mesh.verts.iter().any(|vertex| vertex.pos[0] > 1.1));
        assert!(mesh.verts.iter().any(|vertex| vertex.pos[2] > 2.1));
        assert!(mesh.verts.iter().all(|vertex| vertex.vec().is_finite()));
    }

    #[test]
    fn profile_tool_draws_closes_and_generates_transactionally() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetActiveTool("draw_profile".into()));
        for point in [[0.35, 0.35], [0.65, 0.35], [0.65, 0.65], [0.35, 0.65]] {
            petunia_module_model::draw_profile::profile_add_point(
                &mut bridge.state,
                point[0],
                point[1],
            );
        }
        assert!(bridge.close_profile());
        assert!(bridge.state.profile.closed);
        assert!(bridge.generate_profile_extrude());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert!(bridge.state.project.active_mesh().is_some());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
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
    fn fill_scope_and_projection_controls_change_real_session_state() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert_eq!(bridge.view_model().paint_fill_scope, "ConnectedPixels");

        assert!(bridge.set_fill_scope("UvIsland"));
        assert_eq!(
            bridge.state.session.tools.fill_scope,
            petunia_core::FillScope::UvIsland
        );
        assert_eq!(bridge.view_model().paint_fill_scope, "UvIsland");

        assert!(bridge.set_brush_projection("ScreenSpace"));
        assert_eq!(
            bridge.state.session.tools.brush_projection,
            petunia_core::BrushProjectionMode::ScreenSpace
        );

        assert!(bridge.set_brush_lock("FirstFace"));
        assert_eq!(
            bridge.state.session.tools.brush_lock,
            petunia_core::BrushLock::FirstFace
        );

        assert!(!bridge.set_fill_scope("Nope"));
        assert!(!bridge.set_brush_projection("Nope"));
        assert!(!bridge.set_brush_lock("Nope"));
        assert_eq!(
            bridge.state.session.tools.fill_scope,
            petunia_core::FillScope::UvIsland,
            "valor inválido não pode alterar o estado"
        );
    }

    #[test]
    fn fill_scope_object_paints_the_whole_canvas_and_connected_pixels_stops_at_a_border() {
        use petunia_module_paint::PaintModule;
        let mut state = AppState::default();
        PaintModule::ensure_stack(&mut state);
        state.paint_color = [1.0, 0.0, 0.0];

        let canvas_of = |state: &AppState| {
            state
                .project
                .assets
                .get(state.project.active)
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
                .cloned()
                .expect("canvas do stack")
        };
        let painted = |state: &AppState| {
            canvas_of(state)
                .pixels
                .chunks(4)
                .filter(|px| px[3] > 0)
                .count()
        };

        // Object: canvas inteiro.
        PaintModule::canvas_fill_scoped(&mut state, None, None, petunia_core::FillScope::Object);
        let canvas = canvas_of(&state);
        let total = (canvas.w * canvas.h) as usize;
        assert_eq!(painted(&state), total, "Object pinta o canvas todo");

        // Monta uma fronteira: metade esquerda vermelha, metade direita azul.
        {
            let active = state.project.active;
            let canvas = state
                .project
                .assets
                .get_mut(active)
                .and_then(|asset| asset.paint_stack.as_mut())
                .and_then(|stack| stack.active_mut())
                .and_then(|layer| layer.canvas_mut())
                .expect("canvas mutável");
            let width = canvas.w;
            let height = canvas.h;
            for y in 0..height {
                for x in 0..width {
                    if x < width / 2 {
                        canvas.set(x, y, [255, 0, 0, 255]);
                    } else {
                        canvas.set(x, y, [0, 0, 255, 255]);
                    }
                }
            }
        }

        // ConnectedPixels a partir da esquerda: o azul da direita não é alcançado.
        state.paint_color = [0.0, 1.0, 0.0];
        PaintModule::canvas_fill_scoped(
            &mut state,
            None,
            Some((4, 4)),
            petunia_core::FillScope::ConnectedPixels,
        );
        let canvas = canvas_of(&state);
        let green = canvas
            .pixels
            .chunks(4)
            .filter(|px| px[1] > 200 && px[0] < 60)
            .count();
        let blue = canvas
            .pixels
            .chunks(4)
            .filter(|px| px[2] > 200 && px[0] < 60)
            .count();
        let half = total / 2;
        assert!(
            green.abs_diff(half) < canvas.w as usize * 2,
            "a metade esquerda vira verde: {green} vs {half}"
        );
        assert_eq!(blue, half, "a metade direita permanece azul: {blue}");
    }

    #[test]
    fn face_fill_scope_paints_only_the_hit_face_uv_region() {
        use petunia_module_paint::PaintModule;
        let mut state = AppState::default();
        PaintModule::ensure_stack(&mut state);
        state.paint_color = [0.0, 1.0, 0.0];

        // O cubo usa projeção planar, então todas as faces cobrem 0..1. Restrinjo
        // a face 0 a um quadrado interno para que o escopo por face seja visível.
        {
            let mesh = state.project.active_mesh_mut().unwrap();
            mesh.faces[0].uv = vec![[0.25, 0.25], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]];
        }

        PaintModule::canvas_fill_scoped(&mut state, Some(0), None, petunia_core::FillScope::Face);
        let canvas = state
            .project
            .assets
            .get(state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
            .cloned()
            .unwrap();
        // O canvas base nasce opaco, então o que identifica o preenchimento é a
        // cor: verde puro só existe onde o escopo pintou.
        let painted = canvas
            .pixels
            .chunks(4)
            .filter(|px| px[1] > 200 && px[0] < 60 && px[2] < 60)
            .count();
        let total = (canvas.w * canvas.h) as usize;
        let expected = total / 4;
        assert!(painted > 0, "a face 0 precisa pintar a própria região UV");
        assert!(
            painted.abs_diff(expected) < canvas.w as usize * 2,
            "o quadrado interno cobre ~1/4 do canvas: {painted} vs {expected}"
        );
        assert!(painted < total, "uma face não pode cobrir o canvas inteiro");
    }

    #[test]
    fn shape_tools_anchor_on_press_and_commit_on_release() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("rectangle".to_string()));
        assert!(
            bridge.is_shape_tool(),
            "rectangle precisa ser ferramenta de forma"
        );
        assert_eq!(
            petunia_core::brush_type_from_kind(bridge.state.session.tools.paint_brush_kind),
            petunia_core::BrushType::Rectangle
        );

        assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
        assert!(bridge.shape_anchor.is_some(), "o press ancora a forma");
        assert!(
            bridge.paint_last.is_none(),
            "forma não usa o caminho de traço livre"
        );
        assert_eq!(
            bridge.state.project.undo.depth(),
            (0, 0),
            "ancorar não pode empilhar histórico"
        );

        assert!(bridge.end_paint_stroke_at(430.0, 320.0));
        assert!(bridge.shape_anchor.is_none());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn cancelling_a_shape_leaves_the_document_untouched() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("line".to_string()));

        assert!(bridge.begin_paint_stroke_at(400.0, 300.0));
        assert!(bridge.cancel_paint_stroke());
        assert!(bridge.shape_anchor.is_none());
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert_eq!(bridge.state.ui.status, "Shape cancelled");
    }

    #[test]
    fn shape_press_off_the_surface_is_refused_with_a_reason() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("line".to_string()));

        // Canto superior esquerdo: longe do cubo padrão.
        assert!(!bridge.begin_paint_stroke_at(2.0, 2.0));
        assert!(bridge.shape_anchor.is_none());
        assert!(
            bridge.state.ui.status.starts_with("Shape:"),
            "veio: {}",
            bridge.state.ui.status
        );
    }

    #[test]
    fn boolean_operand_flows_from_the_outliner_to_a_real_fuse() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        assert_eq!(bridge.state.project.assets.len(), 2);
        let sphere_id = bridge.state.project.assets[1].id;

        // Desloca a esfera para fora do cubo: a união de um cubo com uma esfera
        // concêntrica seria o próprio cubo e o teste não provaria nada.
        {
            let mesh = &mut bridge.state.project.assets[1].mesh;
            mesh.select_all();
            mesh.translate_selected([1.6, 0.0, 0.0]);
            mesh.deselect_all();
        }

        // Escolhe a esfera como operando e volta o ativo para o cubo.
        assert!(bridge.set_boolean_operand(&sphere_id.to_string()));
        assert!(bridge.view_model().boolean_ready);
        bridge.state.project.active = 0;
        let before = bridge.state.project.assets[0].mesh.verts.len();

        assert!(bridge.boolean_op("model.fuse"));
        assert_eq!(
            bridge.state.project.assets.len(),
            1,
            "o operando é consumido"
        );
        assert!(bridge.state.project.assets[0].mesh.verts.len() > before);
        assert!(bridge.state.session.tools.boolean_operand.is_none());
        assert_eq!(bridge.view_model().boolean_operand_name, "");
        assert!(bridge.state.project.undo.can_undo());
    }

    #[test]
    fn boolean_op_without_an_operand_is_refused_and_says_why() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().boolean_ready);
        assert!(!bridge.boolean_op("model.fuse"));
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert!(
            bridge.state.ui.status.contains("operand"),
            "veio: {}",
            bridge.state.ui.status
        );
    }

    #[test]
    fn context_menu_marks_the_clicked_asset_as_the_boolean_operand() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cone));
        let cone = bridge.state.project.assets[1].id;
        bridge.open_context_menu(&cone.to_string(), 10.0, 10.0);
        assert!(bridge.context_menu_action("boolean_operand"));
        assert_eq!(bridge.state.session.tools.boolean_operand, Some(cone));
        assert_eq!(bridge.view_model().boolean_operand_name, "Cone");
        assert!(bridge.clear_boolean_operand());
        assert_eq!(bridge.view_model().boolean_operand_name, "");
    }

    #[test]
    fn paint_canvas_image_matches_the_active_layer_pixels() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

        let (width, height) = bridge.paint_canvas_dimensions().expect("canvas do stack");
        assert!(width > 0 && height > 0);

        // Pinta um pixel conhecido na camada ativa e confere que a imagem
        // publicada carrega exatamente esses bytes.
        let marker = [12u8, 200, 45, 255];
        {
            let active = bridge.state.project.active;
            let canvas = bridge
                .state
                .project
                .assets
                .get_mut(active)
                .and_then(|asset| asset.paint_stack.as_mut())
                .and_then(|stack| stack.active_mut())
                .and_then(|layer| layer.canvas_mut())
                .expect("canvas mutável");
            canvas.set(1, 1, marker);
        }

        let image = bridge.render_paint_canvas().expect("imagem do canvas");
        assert_eq!(image.size().width, width);
        assert_eq!(image.size().height, height);
        let buffer = image.to_rgba8().expect("buffer rgba8");
        let offset = ((width + 1) * 4) as usize;
        assert_eq!(&buffer.as_bytes()[offset..offset + 4], &marker);
    }

    #[test]
    fn canvas_image_is_absent_before_a_layer_exists_and_appears_after() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        assert!(
            bridge.paint_canvas_dimensions().is_none(),
            "sem stack não há canvas"
        );

        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
        assert!(bridge.paint_canvas_dimensions().is_some());
        assert!(bridge.view_model().paint_canvas_size.contains('×'));
    }

    #[test]
    fn clicking_the_uv_editor_selects_the_face_under_the_cursor() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        // O cubo usa projeção planar: o centro do espaço UV cai na face 0.
        assert!(bridge.uv_editor_click(0.5, 0.5, false));
        assert_eq!(bridge.view_model().uv_editor.selected_face, 0);
        assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 1);
        assert_eq!(bridge.state.ui.status, "UV: face 0 selected (1 total)");

        // Clicar de novo sem Shift substitui a seleção, não acumula.
        assert!(bridge.uv_editor_click(0.5, 0.5, false));
        assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 1);
        // Com Shift a seleção alterna.
        assert!(bridge.uv_editor_click(0.5, 0.5, true));
        assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 0);
        assert!(!bridge.state.project.active_mesh().unwrap().faces[0].selected);
        assert!(bridge.state.session.selection.faces.is_empty());
        assert!(!bridge.uv_move_selected(0.1, 0.0));
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn paint_color_control_and_viewport_picker_share_the_canvas_color() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::SetPaintColor([1.0, 0.0, 0.0]));
        assert_eq!(bridge.state.paint_color, [1.0, 0.0, 0.0]);
        assert_eq!(bridge.view_model().paint_color, [1.0, 0.0, 0.0]);

        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
        let (px, py) = bridge.canvas_pixel_at(512.0, 384.0).expect("cube surface");
        bridge
            .state
            .project
            .active_mut()
            .unwrap()
            .texture
            .as_mut()
            .unwrap()
            .set(px, py, [12, 100, 220, 255]);
        bridge.state.session.tools.active_tool = "picker".to_string();
        assert!(bridge.begin_paint_stroke_at(512.0, 384.0));
        assert!(bridge.paint_last.is_none());
        assert!(bridge.state.session.tools.paint_stroke.is_none());
        let picked = [12.0 / 255.0, 100.0 / 255.0, 220.0 / 255.0];
        assert_eq!(bridge.state.paint_color, picked);
        assert_eq!(bridge.view_model().paint_color, picked);
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert!(!bridge.begin_paint_stroke_at(-10.0, 384.0));
    }

    #[test]
    fn uv_editor_click_off_the_layout_clears_the_selection_and_says_so() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        // Tira a face 0 do canto para deixar uma região vazia.
        bridge.state.project.active_mesh_mut().unwrap().faces[0].uv =
            vec![[0.0, 0.0], [0.0, 0.25], [0.25, 0.25], [0.25, 0.0]];
        for face in bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .faces
            .iter_mut()
            .skip(1)
        {
            face.uv = vec![[0.0, 0.0], [0.0, 0.1], [0.1, 0.1], [0.1, 0.0]];
        }
        bridge.uv_editor_click(0.5, 0.5, false);

        assert!(!bridge.uv_editor_click(0.9, 0.9, false));
        assert_eq!(bridge.view_model().uv_editor.uv_selected_count, 0);
        assert_eq!(bridge.state.ui.status, "UV: no face under the cursor");
        assert!(!bridge.uv_editor_click(f32::NAN, 0.5, false));
    }

    #[test]
    fn uv_transforms_move_scale_and_rotate_the_selected_faces() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.uv_editor_click(0.5, 0.5, false);
        let before = bridge.state.project.active_mesh().unwrap().faces[0]
            .uv
            .clone();

        assert!(bridge.uv_move_selected(0.1, 0.0));
        let moved = bridge.state.project.active_mesh().unwrap().faces[0]
            .uv
            .clone();
        assert!((moved[0][0] - before[0][0] - 0.1).abs() < 1.0e-5);
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));

        assert!(bridge.uv_scale_selected(2.0));
        let scaled = bridge.state.project.active_mesh().unwrap().faces[0]
            .uv
            .clone();
        let span_before = moved.iter().map(|uv| uv[0]).fold(f32::MIN, f32::max)
            - moved.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
        let span_after = scaled.iter().map(|uv| uv[0]).fold(f32::MIN, f32::max)
            - scaled.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
        assert!(
            span_after > span_before,
            "escalar ×2 precisa alargar a ilha: {span_before} -> {span_after}"
        );

        assert!(bridge.uv_rotate_selected(90.0));
        assert_eq!(bridge.state.project.undo.depth(), (3, 0));

        assert!(
            !bridge.uv_move_selected(0.0, 0.0),
            "movimento nulo é recusado"
        );
        assert!(!bridge.uv_scale_selected(0.0), "escala zero é recusada");
        assert!(!bridge.uv_rotate_selected(0.0), "rotação nula é recusada");
        assert!(!bridge.uv_scale_selected(f32::NAN));
        assert_eq!(bridge.state.project.undo.depth(), (3, 0));
    }

    #[test]
    fn effect_layers_change_the_composited_raster() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

        // Pinta a camada base de um tom conhecido antes do efeito.
        {
            let active = bridge.state.project.active;
            let canvas = bridge
                .state
                .project
                .assets
                .get_mut(active)
                .and_then(|asset| asset.paint_stack.as_mut())
                .and_then(|stack| stack.active_mut())
                .and_then(|layer| layer.canvas_mut())
                .unwrap();
            canvas.fill([200, 40, 90, 255]);
        }
        petunia_module_paint::PaintModule::composite_active(&mut bridge.state);
        let before = bridge.state.project.assets[0].texture.clone().unwrap();
        let sample = before.get(8, 8).unwrap();

        assert!(bridge.add_paint_effect_layer("Invert"));
        let layers = bridge.view_model().paint_layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[1].kind_label, "Effect");
        assert_eq!(bridge.view_model().paint_effect_kind, "Invert");

        let after = bridge.state.project.assets[0].texture.clone().unwrap();
        let inverted = after.get(8, 8).unwrap();
        assert_ne!(sample, inverted, "Invert precisa alterar o pixel");
        assert_eq!(inverted[0], 255 - sample[0]);
        assert_eq!(inverted[1], 255 - sample[1]);
        assert_eq!(inverted[2], 255 - sample[2]);
        assert_eq!(inverted[3], sample[3], "o alfa é preservado");
    }

    #[test]
    fn effect_parameters_are_exposed_and_applied() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

        assert!(bridge.add_paint_effect_layer("Pixelate"));
        let params = bridge.view_model().paint_effect_params;
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].key, "cell_size");
        assert_eq!(params[0].value, 4.0);
        assert!(params[0].min < params[0].max);

        assert!(bridge.set_paint_effect_param("cell_size", 12.0));
        assert_eq!(bridge.view_model().paint_effect_params[0].value, 12.0);

        // Fora da faixa é fixado no limite, nunca aceito cru.
        assert!(bridge.set_paint_effect_param("cell_size", 9_999.0));
        assert_eq!(bridge.view_model().paint_effect_params[0].value, 64.0);
        assert!(bridge.set_paint_effect_param("cell_size", 0.0));
        assert_eq!(bridge.view_model().paint_effect_params[0].value, 1.0);

        assert!(!bridge.set_paint_effect_param("cell_size", f32::NAN));
        assert!(!bridge.set_paint_effect_param("unknown_param", 1.0));
        assert!(!bridge.add_paint_effect_layer("NotAnEffect"));
    }

    #[test]
    fn an_effect_layer_over_a_raster_layer_has_no_editable_canvas() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
        assert!(bridge.add_paint_effect_layer("Posterize"));

        // A camada ativa é a de efeito, então não há canvas para exibir.
        assert!(
            bridge.render_paint_canvas().is_none(),
            "camada de efeito não tem raster próprio"
        );
        assert!(bridge.view_model().paint_effect_kind == "Posterize");
        let params = bridge.view_model().paint_effect_params;
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].key, "levels");
    }

    #[test]
    fn join_merges_the_operand_through_the_shell_and_keeps_parts_is_opt_in() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddPrimitive(
            petunia_core::PrimitiveKind::Cylinder,
        ));
        let operand = bridge.state.project.assets[1].id;
        let operand_verts = bridge.state.project.assets[1].mesh.verts.len();
        let active_verts = bridge.state.project.assets[0].mesh.verts.len();
        // A primitiva recém-criada vira ativa; o alvo do Join é o cubo.
        bridge.state.project.active = 0;

        assert!(bridge.set_boolean_operand(&operand.to_string()));
        assert!(!bridge.view_model().boolean_keep_parts, "padrão é consumir");
        let undo_before = bridge.state.project.undo.depth().0;

        assert!(bridge.join_operand());
        assert_eq!(bridge.state.project.assets.len(), 1);
        assert_eq!(
            bridge.state.project.assets[0].mesh.verts.len(),
            active_verts + operand_verts,
            "Join preserva as duas topologias"
        );
        assert_eq!(
            bridge.state.project.undo.depth().0,
            undo_before + 1,
            "Join é exatamente uma entrada de undo"
        );
    }

    #[test]
    fn keep_parts_toggle_is_reported_and_preserves_the_operand() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.set_boolean_keep_parts(true));
        assert!(bridge.view_model().boolean_keep_parts);
        assert_eq!(
            bridge.state.ui.status,
            "Keep Parts on: the operand stays in the scene"
        );
        assert!(!bridge.set_boolean_keep_parts(true), "sem mudança real");
        assert!(bridge.set_boolean_keep_parts(false));
        assert!(!bridge.view_model().boolean_keep_parts);

        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cone));
        let operand = bridge.state.project.assets[1].id;
        {
            let mesh = &mut bridge.state.project.assets[1].mesh;
            mesh.select_all();
            mesh.translate_selected([1.6, 0.0, 0.0]);
            mesh.deselect_all();
        }
        bridge.set_boolean_operand(&operand.to_string());
        bridge.set_boolean_keep_parts(true);
        bridge.state.project.active = 0;

        assert!(bridge.boolean_op("model.fuse"));
        assert_eq!(
            bridge.state.project.assets.len(),
            2,
            "com Keep Parts o operando permanece"
        );
        assert!(bridge.state.project.assets.iter().any(|a| a.id == operand));
    }

    #[test]
    fn slice_drag_cuts_the_mesh_and_commits_one_undo_entry() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.session.tools.active_tool = "slice".to_string();

        assert!(bridge.begin_slice(400.0, 300.0));
        assert_eq!(bridge.state.session.tools.active_tool, "slice");
        assert!(bridge.slice_anchor.is_some());

        assert!(
            bridge.update_slice(400.0, 380.0),
            "arrasto vertical precisa produzir um plano de corte"
        );
        // Slice divide as faces mas mantém ambos os lados da geometria.
        let sliced = bridge.state.project.active_mesh().unwrap().clone();
        let report = sliced.validate_topology();
        assert!(report.is_manifold, "{report:?}");
        let span = |mesh: &petunia_core::Mesh| {
            let xs: Vec<f32> = mesh.verts.iter().map(|v| v.pos[0]).collect();
            let zs: Vec<f32> = mesh.verts.iter().map(|v| v.pos[2]).collect();
            let ys: Vec<f32> = mesh.verts.iter().map(|v| v.pos[1]).collect();
            (
                xs.iter().fold(f32::MIN, |a, b| a.max(*b))
                    - xs.iter().fold(f32::MAX, |a, b| a.min(*b)),
                ys.iter().fold(f32::MIN, |a, b| a.max(*b))
                    - ys.iter().fold(f32::MAX, |a, b| a.min(*b)),
                zs.iter().fold(f32::MIN, |a, b| a.max(*b))
                    - zs.iter().fold(f32::MAX, |a, b| a.min(*b)),
            )
        };
        let after = span(&sliced);
        let before = (2.0, 2.0, 2.0);
        assert_eq!(after, before, "os dois lados permanecem na malha");
        assert!(sliced.faces.len() > 6, "faces cruzadas são divididas");

        assert!(bridge.commit_slice());
        assert!(bridge.slice_anchor.is_none());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));

        assert!(bridge.state.undo());
        let restored = span(bridge.state.project.active_mesh().unwrap());
        assert_eq!(restored, before, "undo volta ao cubo inteiro");
    }

    #[test]
    fn slice_without_a_drag_is_refused_and_escape_restores_the_mesh() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.session.tools.active_tool = "slice".to_string();

        assert!(bridge.begin_slice(400.0, 300.0));
        assert!(
            !bridge.update_slice(402.0, 301.0),
            "arrasto abaixo do limiar não define plano"
        );
        assert_eq!(
            bridge.state.project.undo.depth(),
            (0, 0),
            "pré-visualização não empilha histórico"
        );

        assert!(bridge.handle_escape());
        assert!(bridge.slice_anchor.is_none());
        assert_eq!(bridge.state.session.tools.active_tool, "select");
        assert_eq!(bridge.state.ui.status, "Slice cancelled");
        assert_eq!(bridge.state.project.active_mesh().unwrap().verts.len(), 8);
    }

    #[test]
    fn the_slice_keymap_action_arms_the_tool_without_touching_geometry() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let keybinds = petunia_config::keybinds::Keybinds::defaults();
        let key = input::key_code_from_slint("K").expect("K mapeável");
        let shift = petunia_config::keybinds::Mods2 {
            ctrl: false,
            shift: true,
            alt: false,
        };
        assert_eq!(
            keybinds.find(key, shift),
            Some("model.slice"),
            "Shift+K precisa estar ligado ao Slice"
        );

        assert!(bridge.route_shortcut("K", false, true, false));
        assert_eq!(bridge.state.session.tools.active_tool, "slice");
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert!(
            bridge.slice_anchor.is_none(),
            "a âncora nasce no pointer-down"
        );
    }

    #[test]
    fn selection_overlay_outlines_the_active_object() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        let overlay = bridge.view_model().selection_overlay;
        assert!(overlay.visible, "o cubo ativo precisa de contorno visível");
        assert!(!overlay.accent, "domínio Object usa a cor de seleção");
        // Ativo tem canal próprio (amarelo), distinto do selecionado
        // (laranja): o cubo ativo padrão não polui o canal de seleção.
        assert!(overlay.outline_commands.is_empty());
        // Contorno de objeto = silhueta frontal, não a caixa nem as 12 arestas:
        // de um canto vê-se 3 faces, portanto 9 arestas de contorno.
        let edges = overlay.active_outline_commands.matches('M').count();
        assert!(
            (6..=12).contains(&edges),
            "silhueta frontal precisa ter entre 6 e 12 arestas, veio {edges}"
        );
        assert!(overlay.point_commands.is_empty());
        assert!(overlay.unselected_outline_commands.is_empty());
    }

    #[test]
    fn sphere_selection_outline_stays_finite_and_inside_the_viewport() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere));
        let overlay = bridge.view_model().selection_overlay;
        let coordinates: Vec<f32> = overlay
            .active_outline_commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        assert!(
            !coordinates.is_empty(),
            "a esfera ativa precisa de silhueta"
        );
        assert_eq!(coordinates.len() % 2, 0);
        for point in coordinates.as_chunks::<2>().0 {
            assert!(point[0].is_finite() && (0.0..=1024.0).contains(&point[0]));
            assert!(point[1].is_finite() && (0.0..=768.0).contains(&point[1]));
        }
    }

    #[test]
    fn selection_overlay_follows_the_domain() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);

        // Point: todos os vértices aparecem como alvos clicáveis, mesmo sem
        // seleção, para o usuário ver onde pode clicar.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        let overlay = bridge.view_model().selection_overlay;
        assert!(overlay.visible);
        assert!(overlay.point_commands.is_empty());
        assert_eq!(
            overlay.unselected_point_commands.matches('M').count(),
            8,
            "os 8 vértices aparecem como alvos"
        );
        bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
        bridge.state.sync_selection();
        let overlay = bridge.view_model().selection_overlay;
        assert!(overlay.visible && overlay.accent);
        assert_eq!(overlay.point_commands.matches('M').count(), 1);
        assert!(overlay.outline_commands.is_empty());
        assert_eq!(
            overlay.unselected_point_commands.matches('M').count(),
            7,
            "o vértice selecionado sai dos alvos neutros e recebe cor forte"
        );

        // Edge: uma linha por aresta selecionada.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
        bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .selected_edges
            .insert((0, 1));
        bridge.state.sync_selection();
        let overlay = bridge.view_model().selection_overlay;
        assert!(overlay.outline_commands.is_empty());
        assert_eq!(
            overlay.unselected_outline_commands.matches('M').count(),
            11,
            "as 11 arestas não selecionadas continuam como alvos"
        );

        // Face: preenchimento e preselection são do renderer, sem marcadores.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        let overlay = bridge.view_model().selection_overlay;
        assert!(
            overlay.outline_commands.is_empty(),
            "o preenchimento da face é do renderer"
        );
        assert!(overlay.point_commands.is_empty());
        assert!(overlay.unselected_point_commands.is_empty());
    }

    #[test]
    fn face_domain_does_not_cover_mesh_with_center_dots() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
        let overlay = bridge.view_model().selection_overlay;
        assert!(!overlay.visible);
        assert!(overlay.point_commands.is_empty());
        assert!(overlay.unselected_point_commands.is_empty());
    }

    #[test]
    fn clicking_the_viewport_reports_what_was_selected() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.project.active = 0;
        bridge.state.set_status("");

        bridge.select_viewport(0.5, 0.5, false);
        assert_eq!(bridge.state.ui.status, "Selected 'Cube'");

        // Fora do cubo o status diz que não acertou nada, em vez de silêncio.
        bridge.state.set_status("");
        bridge.select_viewport(0.02, 0.02, false);
        assert_eq!(bridge.state.ui.status, "Nothing under the cursor");
    }

    #[test]
    fn click_confirms_hover_and_miss_clears_it() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        // Clicar no cubo fixa a preselection: destaque e rótulo aparecem de
        // imediato, sem esperar o próximo mousemove.
        bridge.select_viewport(0.5, 0.5, false);
        assert!(
            bridge.state.session.tools.hover.is_some(),
            "o alvo clicado vira o hover corrente"
        );
        assert!(!bridge.view_model().hover_label.is_empty());
        // Erro limpa em vez de congelar o hover antigo.
        bridge.select_viewport(0.02, 0.02, false);
        assert_eq!(
            bridge.state.session.tools.hover,
            petunia_core::HoverTarget::None
        );
    }

    #[test]
    fn workspace_switch_clears_stale_hover() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.select_viewport(0.5, 0.5, false);
        assert!(bridge.state.session.tools.hover.is_some());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        assert_eq!(
            bridge.state.session.tools.hover,
            petunia_core::HoverTarget::None,
            "hover do Model não pode vazar para o Paint"
        );
    }

    #[test]
    fn dotted_link_is_empty_without_distance() {
        assert!(dotted_link_commands([10.0, 10.0], [10.0, 10.0]).is_empty());
        assert!(dotted_link_commands([10.0, 10.0], [10.2, 10.0]).is_empty());
    }

    #[test]
    fn dotted_link_grows_with_pointer_distance() {
        // Ponto de 2px + intervalo de 4px: 100px rendem 17 segmentos.
        let short = dotted_link_commands([0.0, 0.0], [20.0, 0.0]);
        let long = dotted_link_commands([0.0, 0.0], [100.0, 0.0]);
        let short_count = short.matches('M').count();
        let long_count = long.matches('M').count();
        assert_eq!(short_count, 4, "20px rendem 4 pontos, veio {short_count}");
        assert_eq!(long_count, 17, "100px rendem 17 pontos, veio {long_count}");
        assert!(long_count > short_count);
    }

    #[test]
    fn drag_link_only_exists_during_a_tool_session() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        let pointer = [700.0, 300.0];
        // Sem sessão de manipulação não há cordão, mesmo com o mouse longe.
        assert!(compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, false).is_empty());
        // Com arrasto ativo o cordão liga o pivô ao mouse...
        assert!(bridge.begin_viewport_transform(TransformKind::Position, 512.0, 384.0));
        let link_active = bridge.drag.is_some()
            || bridge.tool_modal.is_some()
            || bridge.state.session.tools.modal.is_some();
        let link = compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, link_active);
        assert!(
            !link.is_empty(),
            "arrasto ativo precisa do cordão pivô→mouse"
        );
        bridge.pointer_position = pointer;
        assert_eq!(bridge.view_model().drag_link_commands, link);
        // ...e some ao confirmar a operação.
        assert!(bridge.end_viewport_transform());
        let link_active = bridge.drag.is_some()
            || bridge.tool_modal.is_some()
            || bridge.state.session.tools.modal.is_some();
        assert!(!link_active);
        assert!(compute_drag_link(&bridge.state, 1024.0, 768.0, pointer, link_active).is_empty());
        assert!(bridge.view_model().drag_link_commands.is_empty());
    }

    #[test]
    fn selection_summary_counts_what_operations_will_hit() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        // Estado padrão: cubo ativo conta como selecionado na UI.
        assert_eq!(bridge.view_model().selection_summary, "1 object selected");
    }

    #[test]
    fn selection_summary_and_inspector_follow_component_picks() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        assert_eq!(bridge.view_model().selection_summary, "No selection");
        bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
        bridge.state.project.active_mesh_mut().unwrap().verts[1].selected = true;
        bridge.state.sync_selection();
        let vm = bridge.view_model();
        assert_eq!(vm.selection_summary, "Selected: 2 points");
        assert!(
            vm.active_object_details.contains("Selected: 2 points"),
            "o inspector mostra o que será atingido, veio: {}",
            vm.active_object_details
        );
        // Limpar volta ao vazio honesto, sem número fantasma.
        bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .deselect_all();
        bridge.state.sync_selection();
        assert_eq!(bridge.view_model().selection_summary, "No selection");
    }

    #[test]
    fn viewport_right_click_triage_cancels_session_first() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        // Com modal ativo o botão direito cancela em vez de abrir menu.
        assert!(bridge.begin_viewport_transform(TransformKind::Position, 512.0, 384.0));
        assert!(bridge.viewport_context_triage(700.0, 300.0));
        assert!(bridge.drag.is_none());
        assert!(!bridge.view_model().context_menu_open);
    }

    #[test]
    fn viewport_right_click_opens_selection_menu_without_session() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        assert!(!bridge.viewport_context_triage(700.0, 300.0));
        let vm = bridge.view_model();
        assert!(vm.context_menu_open);
        assert_eq!(vm.context_menu_mode, "viewport");
        assert_eq!(vm.context_menu_title, "Viewport");
        // Verbetes de seleção funcionam e fecham o menu.
        assert!(bridge.context_menu_action("select_all"));
        assert!(!bridge.view_model().context_menu_open);
    }

    #[test]
    fn viewport_menu_select_all_clear_and_escape() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        bridge.open_viewport_context_menu(100.0, 100.0);
        assert!(bridge.context_menu_action("select_all"));
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .all(|v| v.selected),
            "select_all do menu seleciona tudo como o atalho A"
        );
        bridge.open_viewport_context_menu(100.0, 100.0);
        assert!(bridge.context_menu_action("clear_selection"));
        assert!(
            bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .verts
                .iter()
                .all(|v| !v.selected),
            "clear do menu limpa como Alt+A"
        );
        // Escape fecha o menu da viewport pelo LIFO, como o do Outliner.
        bridge.open_viewport_context_menu(100.0, 100.0);
        assert!(bridge.view_model().context_menu_open);
        assert!(bridge.handle_escape());
        assert!(!bridge.view_model().context_menu_open);
    }

    #[test]
    fn keyboard_modal_axis_numeric_confirm_flow() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        // G abre o modal Move instantâneo, como no Blender.
        assert!(bridge.route_shortcut("G", false, false, false));
        assert!(
            bridge
                .state
                .session
                .tools
                .modal
                .as_ref()
                .is_some_and(|m| m.kind == petunia_core::ModalKind::Move),
            "G precisa abrir o modal de Move"
        );
        // HUD acompanha com título e valores ao vivo.
        let vm = bridge.view_model();
        assert!(vm.operation_hud_active);
        assert_eq!(vm.operation_hud_title, "Move");
        // X trava o eixo e aparece no HUD; repetir solta para Free.
        assert!(bridge.route_shortcut("X", false, false, false));
        assert_eq!(
            bridge
                .state
                .session
                .tools
                .modal
                .as_ref()
                .map(|m| m.constraint),
            Some(petunia_core::ModalConstraint::Axis(0))
        );
        assert_eq!(bridge.view_model().gizmo_constraint_axes, [0, -1]);
        assert!(
            bridge
                .view_model()
                .operation_hud_lines
                .iter()
                .any(|line| line.starts_with("X   ")),
            "eixo travado aparece no HUD"
        );
        assert!(bridge.route_shortcut("X", false, false, false));
        assert_eq!(bridge.view_model().gizmo_constraint_axes, [-1, -1]);
        assert_eq!(
            bridge
                .state
                .session
                .tools
                .modal
                .as_ref()
                .map(|m| m.constraint),
            Some(petunia_core::ModalConstraint::Free)
        );
        // Shift+Y exclui Y e trava o plano XZ.
        assert!(bridge.route_shortcut("Y", false, true, false));
        assert_eq!(
            bridge
                .state
                .session
                .tools
                .modal
                .as_ref()
                .map(|m| m.constraint),
            Some(petunia_core::ModalConstraint::Plane(1))
        );
        assert_eq!(bridge.view_model().gizmo_constraint_axes, [2, 0]);
        // Entrada numérica acumula, mostra Input no HUD e aceita Backspace.
        assert!(bridge.route_shortcut("2", false, false, false));
        assert!(bridge.route_shortcut(".", false, false, false));
        assert!(bridge.route_shortcut("5", false, false, false));
        assert_eq!(bridge.modal_text, "2.5");
        assert!(
            bridge
                .view_model()
                .operation_hud_lines
                .iter()
                .any(|line| line.contains("Input   2.5")),
            "texto digitado aparece no HUD"
        );
        assert!(bridge.route_shortcut("Backspace", false, false, false));
        assert_eq!(bridge.modal_text, "2.");
        // Enter confirma em UMA etapa de undo e fecha o modal.
        assert!(bridge.route_shortcut("Enter", false, false, false));
        assert!(bridge.state.session.tools.modal.is_none());
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    }

    #[test]
    fn the_gizmo_only_appears_with_a_transform_tool() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        assert!(
            !bridge.view_model().gizmo.visible,
            "com a ferramenta Select o gizmo não pode cobrir o modelo"
        );

        bridge.apply(UiIntent::SetActiveTool("move".to_string()));
        assert!(bridge.view_model().gizmo.visible);

        bridge.apply(UiIntent::SetActiveTool("select".to_string()));
        assert!(!bridge.view_model().gizmo.visible);
    }

    #[test]
    fn selection_overlay_draws_every_pickable_element_of_the_domain() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);

        // Point: os 8 vértices aparecem mesmo sem seleção, para o usuário ver
        // onde pode clicar; o selecionado vai para a camada de destaque.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
        bridge.state.sync_selection();
        let overlay = bridge.view_model().selection_overlay;
        assert_eq!(overlay.point_commands.matches('M').count(), 1);
        assert_eq!(
            overlay.unselected_point_commands.matches('M').count(),
            7,
            "os outros 7 vértices precisam aparecer como alvos"
        );

        // Edge: todas as 12 arestas do cubo aparecem; a selecionada destaca.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
        bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .selected_edges
            .insert((0, 1));
        bridge.state.sync_selection();
        let overlay = bridge.view_model().selection_overlay;
        assert!(overlay.outline_commands.is_empty());
        assert_eq!(
            overlay.unselected_outline_commands.matches('M').count(),
            11,
            "as outras 11 arestas precisam aparecer como alvos"
        );

        // Object: silhueta frontal no canal do ativo (o cubo padrão é o
        // ativo), sem camada de não selecionados.
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object));
        let overlay = bridge.view_model().selection_overlay;
        assert!(!overlay.active_outline_commands.is_empty());
        assert!(overlay.outline_commands.is_empty());
        assert!(overlay.unselected_outline_commands.is_empty());
        assert!(overlay.unselected_point_commands.is_empty());
    }

    #[test]
    fn instant_mode_confirms_a_tool_on_click_instead_of_selecting() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();

        assert!(bridge.set_tool_activation("instant"));
        assert_eq!(bridge.view_model().tool_activation, "instant");
        assert!(!bridge.set_tool_activation("instant"), "sem mudança real");
        assert!(!bridge.set_tool_activation("bogus"));

        bridge.execute_core_command("model.extrude").unwrap();
        assert!(bridge.tool_modal.is_some());
        let value_before = bridge.tool_modal_value;
        assert!(bridge.scrub_tool_modal(-30.0, false));
        assert!(bridge.tool_modal_value > value_before);

        bridge.select_viewport(0.6, 0.6, false);
        assert!(bridge.tool_modal.is_none(), "o clique confirma a sessão");
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    }

    #[test]
    fn drag_mode_keeps_selecting_on_click_with_a_tool_open() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        assert_eq!(bridge.view_model().tool_activation, "drag");

        bridge.execute_core_command("model.extrude").unwrap();
        assert!(bridge.tool_modal.is_some());
        // No modo Drag um clique na viewport seleciona normalmente; a sessão
        // continua aberta até o arrasto ou o Apply.
        bridge.select_viewport(0.5, 0.5, false);
        assert!(bridge.tool_modal.is_some());
    }

    #[test]
    fn extrude_shortcut_uses_instant_pointer_even_with_drag_preference() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        assert_eq!(bridge.view_model().tool_activation, "drag");

        assert!(bridge.route_shortcut("E", false, false, false));
        assert!(bridge.keyboard_tool_modal_active);
        assert!(bridge.view_model().keyboard_tool_modal_active);
        assert!(bridge.scrub_tool_modal(-32.0, false));
        assert!(bridge.tool_modal_value > 0.0);
        bridge.select_viewport(0.5, 0.5, false);
        assert!(bridge.tool_modal.is_none());
        assert!(!bridge.keyboard_tool_modal_active);
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    }

    #[test]
    fn escape_abandons_an_instant_tool_without_committing() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();
        bridge.set_tool_activation("instant");

        bridge.execute_core_command("model.inset").unwrap();
        bridge.scrub_tool_modal(-20.0, false);
        assert!(bridge.handle_escape());
        assert!(bridge.tool_modal.is_none());
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn geometry_never_carries_selection_colour() {
        // A regressão original: selecionar uma aresta marcava os vértices das
        // pontas como selecionados e a triangulação pintava TODAS as faces que
        // tocavam esses vértices de laranja, mesmo em modo Edge.
        let mut mesh = petunia_core::Mesh::cube(2.0);
        let before: Vec<[f32; 3]> = mesh
            .to_triangles_smooth(false)
            .into_iter()
            .map(|(_, _, color, _)| color)
            .collect();

        mesh.faces[0].selected = true;
        mesh.verts[0].selected = true;
        mesh.selected_edges.insert((0, 1));

        let after: Vec<[f32; 3]> = mesh
            .to_triangles_smooth(false)
            .into_iter()
            .map(|(_, _, color, _)| color)
            .collect();
        assert_eq!(
            before, after,
            "selecionar não pode alterar a cor da geometria"
        );

        // A cor de seleção não aparece em nenhum vértice da triangulação.
        for (_, _, color, _) in mesh.to_triangles_smooth(false) {
            assert!(
                !(color[0] > 0.95 && (color[1] - 0.55).abs() < 0.05),
                "triangulação ainda pinta seleção: {color:?}"
            );
        }
    }

    #[test]
    fn the_four_shading_modes_are_distinct_and_reachable() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert_eq!(bridge.view_model().shading_mode, "solid");

        // Cada modo tem semântica própria: preencher, amostrar material e usar
        // a luz da cena são eixos separados.
        assert!(petunia_core::Shading::Solid.fills_faces());
        assert!(!petunia_core::Shading::Solid.samples_material());
        assert!(!petunia_core::Shading::Solid.uses_scene_light());

        assert!(!petunia_core::Shading::Wireframe.fills_faces());

        assert!(petunia_core::Shading::MaterialPreview.fills_faces());
        assert!(petunia_core::Shading::MaterialPreview.samples_material());
        assert!(!petunia_core::Shading::MaterialPreview.uses_scene_light());

        assert!(petunia_core::Shading::Rendered.uses_scene_light());

        for mode in petunia_core::Shading::ALL {
            assert!(bridge.set_shading_mode(mode.id()), "{}", mode.id());
            assert_eq!(bridge.view_model().shading_mode, mode.id());
            assert_eq!(bridge.state.shading, mode);
            // Material e Rendered precisam amostrar o material de fato.
            if mode.samples_material() {
                assert!(bridge.state.session.textured, "{}", mode.id());
            }
        }

        assert!(!bridge.set_shading_mode("nope"));
        assert_eq!(
            bridge.state.shading,
            petunia_core::Shading::Rendered,
            "valor inválido não altera o modo"
        );
    }

    #[test]
    fn xray_opacity_is_clamped_and_reported() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.set_xray_opacity(0.7));
        assert!((bridge.view_model().xray_opacity - 0.7).abs() < 1.0e-6);

        assert!(bridge.set_xray_opacity(5.0));
        assert!((bridge.view_model().xray_opacity - 0.9).abs() < 1.0e-6);
        assert!(bridge.set_xray_opacity(-1.0));
        assert!((bridge.view_model().xray_opacity - 0.1).abs() < 1.0e-6);

        assert!(!bridge.set_xray_opacity(f32::NAN));
        assert!((bridge.view_model().xray_opacity - 0.1).abs() < 1.0e-6);
        assert!(!bridge.set_xray_opacity(0.1), "sem mudança real");
    }

    #[test]
    fn xray_command_toggles_the_viewport_state_without_changing_selection() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.sync_selection();
        let selection_before = bridge.state.session.selection.clone();
        assert!(!bridge.view_model().show_xray);
        bridge.execute_core_command("view.toggle_xray").unwrap();
        assert!(bridge.view_model().show_xray);
        bridge.execute_core_command("view.toggle_xray").unwrap();
        assert!(!bridge.view_model().show_xray);
        assert_eq!(
            bridge.state.session.selection.assets,
            selection_before.assets
        );
    }

    #[test]
    fn the_scene_carries_a_real_light_for_rendered_mode() {
        let project = petunia_project::Project::new();
        let light = project.active_light().expect("luz padrão da cena");
        assert!(light.enabled);
        assert_eq!(light.kind, petunia_project::LightKind::Directional);

        // A direção é normalizada e nunca degenera.
        let direction = light.normalized_direction();
        let length = (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt();
        assert!((length - 1.0).abs() < 1.0e-4);

        let degenerate = petunia_project::Light::directional("Zero", [0.0, 0.0, 0.0]);
        assert_eq!(degenerate.normalized_direction(), [0.0, 1.0, 0.0]);
        let hostile = petunia_project::Light::directional("NaN", [f32::NAN, 1.0, 0.0]);
        assert_eq!(hostile.normalized_direction(), [0.0, 1.0, 0.0]);

        // Desabilitar todas as luzes faz o Rendered cair no estúdio da viewport,
        // nunca renderizar preto.
        let mut project = petunia_project::Project::new();
        for light in &mut project.lights {
            light.enabled = false;
        }
        assert!(project.active_light().is_none());
    }

    #[test]
    fn the_gizmo_handle_is_picked_in_screen_space_with_a_generous_target() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::SetActiveTool("move".to_string()));
        let gizmo = bridge.view_model().gizmo;
        assert!(gizmo.visible);

        // O centro exato de uma haste acerta o handle.
        let x_end = {
            let numbers: Vec<f32> = gizmo
                .x_commands
                .split_whitespace()
                .filter_map(|token| token.parse::<f32>().ok())
                .collect();
            [numbers[2], numbers[3]]
        };
        assert_eq!(
            bridge.gizmo_handle_at(x_end[0], x_end[1]),
            Some(GizmoHandle::X)
        );

        // Um ponto a 6 px da haste ainda acerta: o alvo é maior que o traço.
        assert_eq!(
            bridge.gizmo_handle_at(x_end[0] + 6.0, x_end[1]),
            Some(GizmoHandle::X)
        );
        // Longe de qualquer haste não há handle.
        assert_eq!(
            bridge.gizmo_handle_at(gizmo.origin_x + 400.0, gizmo.origin_y),
            None
        );

        // Sem ferramenta de transformação não há gizmo nem handle.
        bridge.apply(UiIntent::SetActiveTool("select".to_string()));
        assert_eq!(bridge.gizmo_handle_at(x_end[0], x_end[1]), None);
    }

    #[test]
    fn combined_transform_has_independent_move_scale_and_rotate_handles() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.apply(UiIntent::SetActiveTool("transform".to_string()));
        let gizmo = bridge.view_model().gizmo;
        assert!(gizmo.visible);
        assert!(!gizmo.x_scale_commands.is_empty());
        assert!(!gizmo.x_rotate_commands.is_empty());

        let numbers = |commands: &str| -> Vec<f32> {
            commands
                .split_whitespace()
                .filter_map(|part| part.parse().ok())
                .collect()
        };
        let scale = numbers(&gizmo.x_scale_commands);
        let scale_center = [(scale[0] + scale[4]) * 0.5, (scale[1] + scale[5]) * 0.5];
        assert_eq!(
            bridge.gizmo_target_at(scale_center[0], scale_center[1]),
            Some(GizmoTarget {
                handle: GizmoHandle::X,
                kind: TransformKind::Scale,
            })
        );

        let ring = numbers(&gizmo.x_rotate_commands);
        assert_eq!(
            bridge
                .gizmo_target_at(ring[0], ring[1])
                .map(|target| target.kind),
            Some(TransformKind::Rotation)
        );

        let rod = numbers(&gizmo.x_commands);
        assert_eq!(
            bridge.gizmo_target_at(rod[2], rod[3]),
            Some(GizmoTarget {
                handle: GizmoHandle::X,
                kind: TransformKind::Position,
            })
        );
    }

    #[test]
    fn lasso_path_clamps_pointer_grab_outside_viewport() {
        let path = parse_lasso_path("-0.25,0.5;0.5,0.5;1.4,1.2;").unwrap();
        assert_eq!(path, vec![[-1.0, 0.0], [0.0, 0.0], [1.0, -1.0]]);
        assert!(parse_lasso_path("0,0;NaN,1;1,1;").is_none());
        assert!(parse_lasso_path("0,0;1,1;").is_none());
    }

    #[test]
    fn model_tool_commands_activate_the_same_tools_as_the_toolbar() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.execute_command(CommandId::SelectLasso);
        assert_eq!(bridge.state.session.tools.active_tool, "lasso_select");
        bridge.execute_command(CommandId::TransformCombined);
        assert_eq!(bridge.state.session.tools.active_tool, "transform");
        assert!(bridge.view_model().gizmo.visible);
    }

    #[test]
    fn dragging_a_gizmo_handle_constrains_the_transform_to_that_axis() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
        bridge.state.sync_selection();
        bridge.apply(UiIntent::SetActiveTool("move".to_string()));

        let gizmo = bridge.view_model().gizmo;
        let numbers: Vec<f32> = gizmo
            .x_commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        let end = [numbers[2], numbers[3]];

        // Hover antes do clique: preselection sem histórico.
        assert!(bridge.hover_gizmo(end[0], end[1]));
        assert_eq!(bridge.view_model().gizmo_hover_axis, 0);
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));

        assert!(bridge.begin_gizmo_drag(end[0], end[1]));
        assert_eq!(bridge.view_model().gizmo_active_axis, 0);
        assert!(bridge.state.session.tools.modal.is_some());

        // Arrastar só move no eixo X: Y e Z ficam intactos.
        let before = bridge.state.project.active_mesh().unwrap().verts[0].pos;
        assert!(bridge.update_viewport_transform(end[0] + 80.0, end[1]));
        let after = bridge.state.project.active_mesh().unwrap().verts[0].pos;
        assert!((after[0] - before[0]).abs() > 1.0e-3, "X precisa mudar");
        assert!((after[1] - before[1]).abs() < 1.0e-4, "Y precisa ficar");
        assert!((after[2] - before[2]).abs() < 1.0e-4, "Z precisa ficar");

        assert!(bridge.end_gizmo_drag());
        assert_eq!(bridge.view_model().gizmo_active_axis, -1);
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));
    }

    #[test]
    fn orbiting_uses_the_selection_as_pivot() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        // Move um vértice para longe da origem e seleciona só ele.
        {
            let mesh = bridge.state.project.active_mesh_mut().unwrap();
            mesh.verts[0].pos = [5.0, 0.0, 0.0];
            mesh.verts[0].selected = true;
        }
        bridge.state.sync_selection();
        assert_ne!(bridge.state.session.camera.target.x, 5.0);

        assert!(bridge.orbit_viewport(10.0, 0.0));
        assert!(
            (bridge.state.session.camera.target.x - 5.0).abs() < 1.0e-3,
            "a órbita precisa pivotar na seleção, veio {:?}",
            bridge.state.session.camera.target
        );

        // Sem seleção o alvo não é mexido.
        bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .deselect_all();
        bridge.state.sync_selection();
        let target = bridge.state.session.camera.target;
        assert!(bridge.orbit_viewport(10.0, 0.0));
        assert_eq!(bridge.state.session.camera.target, target);
    }

    #[test]
    fn the_operation_hud_reports_the_real_value_and_the_axis() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        // Extrude exige faces selecionadas, não vértices.
        bridge.state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        bridge.state.sync_selection();

        // Em repouso o HUD some e a barra informa domínio e navegação.
        let vm = bridge.view_model();
        assert!(!vm.operation_hud_active);
        assert!(
            vm.context_hint.contains("Selection:"),
            "veio: {}",
            vm.context_hint
        );
        assert!(vm.context_hint.contains("Orbit"));

        // Com ferramenta aberta o HUD mostra título, valor e como confirmar.
        bridge.execute_core_command("model.extrude").unwrap();
        let vm = bridge.view_model();
        assert!(vm.operation_hud_active);
        assert_eq!(vm.operation_hud_title, "Extrude");
        assert_eq!(vm.operation_hud_lines.len(), 1);
        assert!(
            vm.operation_hud_lines[0].starts_with("Distance"),
            "veio: {}",
            vm.operation_hud_lines[0]
        );
        assert!(vm.operation_hud_hint.contains("Confirm"));
        assert!(vm.operation_hud_hint.contains("Cancel"));
        assert!(!vm.operation_hud_subject.is_empty());

        // O valor do HUD acompanha o arrasto.
        bridge.scrub_tool_modal(-40.0, false);
        let vm = bridge.view_model();
        assert!(
            vm.operation_hud_lines[0] != "Distance   0.000",
            "o HUD precisa refletir o valor real: {}",
            vm.operation_hud_lines[0]
        );

        bridge.commit_tool_modal();
        assert!(!bridge.view_model().operation_hud_active);
    }

    #[test]
    fn the_operation_hud_shows_the_axis_constraint() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.state.project.active_mesh_mut().unwrap().verts[0].selected = true;
        bridge.state.sync_selection();
        bridge.apply(UiIntent::SetActiveTool("move".to_string()));

        let gizmo = bridge.view_model().gizmo;
        let numbers: Vec<f32> = gizmo
            .y_commands
            .split_whitespace()
            .filter_map(|token| token.parse::<f32>().ok())
            .collect();
        assert!(bridge.begin_gizmo_drag(numbers[2], numbers[3]));

        let vm = bridge.view_model();
        assert!(vm.operation_hud_active);
        assert_eq!(vm.operation_hud_title, "Move");
        assert!(
            vm.operation_hud_lines[0].starts_with('Y'),
            "a linha precisa nomear o eixo: {}",
            vm.operation_hud_lines[0]
        );
        assert!(vm.operation_hud_subject.contains("Y axis"));
        assert!(vm.context_hint.contains("Move"));
        bridge.end_gizmo_drag();
    }

    #[test]
    fn hovering_preselects_a_component_without_touching_the_document() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));

        // O centro da viewport acerta a face frontal do cubo.
        assert!(bridge.hover_component(0.5, 0.5));
        assert!(matches!(
            bridge.state.session.tools.hover,
            petunia_core::HoverTarget::Face(_)
        ));
        assert!(!bridge.view_model().hover_label.is_empty());
        // Passar o mouse não seleciona nem empilha histórico.
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
        assert!(
            !bridge
                .state
                .project
                .active_mesh()
                .unwrap()
                .faces
                .iter()
                .any(|face| face.selected)
        );

        // Sair da geometria limpa a preselection.
        assert!(bridge.hover_component(0.02, 0.02));
        assert_eq!(
            bridge.state.session.tools.hover,
            petunia_core::HoverTarget::None
        );
        assert!(!bridge.clear_hover(), "já estava limpo");
    }

    #[test]
    fn object_picking_uses_the_surface_and_ignores_the_empty_bounding_sphere() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge
            .state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Front);

        // (1.1, 1.1) fica dentro da esfera aproximada do cubo, mas fora da
        // superfície [-1, 1]². Nenhum objeto pode ser anunciado ou selecionado.
        let ndc = bridge
            .state
            .session
            .camera
            .project_ndc(glam::Vec3::new(1.1, 1.1, 0.0));
        let (x, y) = ((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);
        assert_eq!(
            bridge.pick_viewport_target(x, y),
            petunia_core::HoverTarget::None
        );
        bridge.select_viewport(x, y, false);
        assert_eq!(bridge.state.ui.status, "Nothing under the cursor");
        assert_eq!(bridge.state.project.active, usize::MAX);
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn vertex_preselection_and_click_agree_on_visibility_and_screen_target() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge
            .state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Front);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

        let front = glam::Vec3::new(-1.0, -1.0, 1.0);
        let ndc = bridge.state.session.camera.project_ndc(front);
        let (x, y) = ((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);
        assert_eq!(
            bridge.pick_viewport_target(x, y),
            petunia_core::HoverTarget::Vertex(4)
        );
        assert!(bridge.hover_component(x, y));
        bridge.select_viewport(x, y, false);
        assert_eq!(bridge.state.session.selection.verts, vec![4]);
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));

        bridge.select_viewport(x, y, true);
        assert!(bridge.state.session.selection.verts.is_empty());
        assert!(bridge.hover_component(f32::NAN, y));
        assert_eq!(
            bridge.state.session.tools.hover,
            petunia_core::HoverTarget::None
        );
        assert_eq!(
            bridge.pick_viewport_target(-0.1, y),
            petunia_core::HoverTarget::None
        );
    }

    #[test]
    fn changing_domains_converts_selection_without_ghost_faces_or_edges() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
        bridge.state.project.active_mesh_mut().unwrap().faces[1].selected = true;
        bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .sync_vert_selection_from_faces();
        bridge.state.sync_selection();
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(!mesh.faces.iter().any(|face| face.selected));
        assert_eq!(mesh.selected_edges.len(), 4);

        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(mesh.selected_edges.is_empty());
        assert_eq!(mesh.selected_vert_count(), 4);
        assert_eq!(bridge.state.project.undo.depth(), (0, 0));
    }

    #[test]
    fn hover_respects_occlusion_unless_xray_is_on() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(1024, 768);
        bridge.state.set_edit_mode(petunia_core::EditMode::Edit);
        bridge.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));

        // Raio do olho até o vértice traseiro do cubo (z = -1): atravessa a
        // face frontal, então o alvo está ocluído.
        let origin = bridge.state.session.camera.eye();
        let position = glam::Vec3::new(-1.0, -1.0, -1.0);
        let direction = (position - origin).normalize();
        assert!(
            bridge.is_occluded(origin, direction, position),
            "o vértice traseiro precisa estar atrás da face frontal"
        );

        // Um vértice frontal não está ocluído.
        let front = glam::Vec3::new(1.0, 1.0, 1.0);
        let front_dir = (front - origin).normalize();
        assert!(!bridge.is_occluded(origin, front_dir, front));

        bridge.state.session.show_xray = true;
        assert!(
            !bridge.is_occluded(origin, direction, position),
            "X-Ray existe justamente para alcançar o que está atrás"
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
        assert_eq!(bridge.state.session.selection.assets.len(), 2);

        bridge.execute_command(CommandId::ClearSelection);
        assert!(bridge.state.session.selection.assets.is_empty());
        assert_eq!(bridge.state.project.active, usize::MAX);

        bridge.execute_command(CommandId::InvertSelection);
        assert_eq!(bridge.state.session.selection.assets.len(), 2);

        bridge.execute_command(CommandId::SaveActiveAsAsset);
        assert_eq!(bridge.state.project.assets.len(), 3);
    }

    #[test]
    fn edge_loop_selection_via_select_viewport_ext() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.execute_command(CommandId::AddPlane);
        bridge.state.set_selection_domain(SelectionDomain::Edge);

        // Direct call to select_edge_loop through mesh or select_viewport_ext
        let count = bridge
            .state
            .project
            .active_mesh_mut()
            .unwrap()
            .select_edge_loop((0, 1), false);
        assert_eq!(count, 4, "boundary loop of plane has 4 edges");
        let active = bridge.state.project.active_mesh().unwrap();
        assert_eq!(active.selected_edges.len(), 4);
        assert!(active.verts.iter().all(|v| v.selected));
    }

    #[test]
    fn palette_import_and_export_intents_work() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let temp_dir = std::env::temp_dir();
        let gpl_path = temp_dir.join("test_sprint1_palette.gpl");

        let content = "GIMP Palette\nName: Test\nColumns: 4\n#\n255   0   0 Red\n  0 255   0 Green\n  0   0 255 Blue\n";
        std::fs::write(&gpl_path, content).unwrap();

        bridge.apply(UiIntent::ImportPalette(gpl_path.clone()));
        assert!(bridge.state.ui.status.contains("imported palette"));
        assert_eq!(bridge.state.project.palette.len(), 3);

        let export_path = temp_dir.join("test_sprint1_export.gpl");
        bridge.apply(UiIntent::ExportPalette(export_path.clone()));
        assert!(
            bridge
                .state
                .ui
                .status
                .contains("palette exported successfully")
        );
        assert!(export_path.exists());

        let _ = std::fs::remove_file(gpl_path);
        let _ = std::fs::remove_file(export_path);
    }

    #[test]
    fn uv_seam_toggle_with_selected_edges() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        mesh.faces[0].selected = false;
        mesh.selected_edges.insert((0, 1));
        mesh.selected_edges.insert((1, 2));

        assert!(bridge.toggle_selected_uv_seams());
        let seams = bridge.state.project.active_mesh().unwrap().uv_seams.clone();
        assert_eq!(seams.len(), 2);
        assert!(seams.contains(&(0, 1)));
        assert!(seams.contains(&(1, 2)));
        assert_eq!(bridge.state.project.undo.depth(), (1, 0));

        // Toggling again removes them
        assert!(bridge.toggle_selected_uv_seams());
        let seams_after = bridge.state.project.active_mesh().unwrap().uv_seams.clone();
        assert!(seams_after.is_empty());
    }

    #[test]
    fn universal_gizmo_disambiguation_with_deadzone() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetActiveTool("transform".to_string()));

        let gizmo = bridge.view_model().gizmo;
        assert!(gizmo.visible);
        let origin_x = gizmo.origin_x;
        let origin_y = gizmo.origin_y;

        // Deadzone check: point exactly at origin or within deadzone (<= 12px) returns None
        assert_eq!(bridge.gizmo_target_at(origin_x, origin_y), None);
        assert_eq!(bridge.gizmo_target_at(origin_x + 5.0, origin_y + 5.0), None);

        // Outside deadzone: far away point returns None
        assert_eq!(bridge.gizmo_target_at(0.0, 0.0), None);
    }

    #[test]
    fn slice_dynamic_preview_line() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.resize_viewport(800, 600);
        bridge.apply(UiIntent::SetActiveTool("slice".to_string()));

        // When slice is not dragging, drag_link_commands is empty
        assert!(bridge.view_model().drag_link_commands.is_empty());

        // When slice drag starts and mouse moves:
        bridge.slice_anchor = Some([200.0, 200.0]);
        bridge.pointer_position = [300.0, 250.0];

        let vm = bridge.view_model();
        assert!(!vm.drag_link_commands.is_empty());
        assert!(vm.drag_link_commands.contains("L "));
    }

    #[test]
    fn material_slots_assignment_and_duplication() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let mesh = bridge.state.project.active_mesh_mut().unwrap();
        // Select face 0
        for f in &mut mesh.faces {
            f.selected = false;
        }
        mesh.faces[0].selected = true;

        // Initially 1 default material in project
        let vm = bridge.view_model();
        assert!(!vm.material_slots.is_empty());

        // Create new material via UiIntent
        bridge.apply(UiIntent::CreateMaterial);
        let vm = bridge.view_model();
        assert_eq!(vm.material_slots.len(), 2);
        assert_eq!(vm.active_material_slot, 1);

        // Assign to face 0 via UiIntent
        bridge.apply(UiIntent::AssignMaterialSlot(1));
        let mesh = bridge.state.project.active_mesh().unwrap();
        assert_eq!(mesh.faces[0].material_slot, Some(1));
        assert_eq!(bridge.state.project.undo.depth(), (2, 0));

        // Duplicate material 1 via UiIntent
        bridge.apply(UiIntent::DuplicateMaterial(1));
        let vm = bridge.view_model();
        assert_eq!(vm.material_slots.len(), 3);
        assert_eq!(vm.active_material_slot, 2);
        assert!(vm.material_slots[2].contains("Copy"));
    }

    #[test]
    fn paint_2d_stroke_flow_and_undo() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

        // Set bright red paint color
        bridge.apply(UiIntent::SetPaintColor([1.0, 0.0, 0.0]));

        // Check active texture initial pixel at (128, 128)
        let initial_color = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .unwrap_or([0, 0, 0, 0]);

        // Begin 2D stroke at center (0.5, 0.5)
        bridge.apply(UiIntent::Paint2dStroke {
            norm_x: 0.5,
            norm_y: 0.5,
            phase: 0,
        });
        assert!(bridge.paint_2d_last.is_some());

        // Move stroke
        bridge.apply(UiIntent::Paint2dStroke {
            norm_x: 0.51,
            norm_y: 0.51,
            phase: 1,
        });

        // Finish stroke
        bridge.apply(UiIntent::Paint2dStroke {
            norm_x: 0.51,
            norm_y: 0.51,
            phase: 2,
        });
        assert!(bridge.paint_2d_last.is_none());

        // Texture pixel should be red
        let painted_color = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .expect("pixel");
        assert_eq!(painted_color[0], 255);
        assert_eq!(painted_color[1], 0);
        assert_eq!(painted_color[2], 0);

        // Undo stroke
        bridge.apply(UiIntent::Undo);
        let undone_color = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .unwrap_or([0, 0, 0, 0]);
        assert_eq!(undone_color, initial_color);

        // Redo stroke
        bridge.apply(UiIntent::Redo);
        let redone_color = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .expect("pixel");
        assert_eq!(redone_color[0], 255);
    }

    #[test]
    fn paint_pixel_grid_and_canvas_zoom() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.paint_pixel_grid);
        assert_eq!(bridge.paint_canvas_zoom, 1);

        // Toggle pixel grid
        bridge.apply(UiIntent::TogglePaintPixelGrid);
        assert!(!bridge.paint_pixel_grid);
        assert!(!bridge.view_model().paint_pixel_grid);

        bridge.apply(UiIntent::TogglePaintPixelGrid);
        assert!(bridge.paint_pixel_grid);
        assert!(bridge.view_model().paint_pixel_grid);

        // Set zoom
        bridge.apply(UiIntent::SetPaintCanvasZoom(4));
        assert_eq!(bridge.paint_canvas_zoom, 4);
        assert_eq!(bridge.view_model().paint_canvas_zoom, 4);

        // Clamping upper bound
        bridge.apply(UiIntent::SetPaintCanvasZoom(32));
        assert_eq!(bridge.paint_canvas_zoom, 16);

        // Clamping lower bound
        bridge.apply(UiIntent::SetPaintCanvasZoom(-2));
        assert_eq!(bridge.paint_canvas_zoom, 1);

        // render_paint_canvas succeeds with zoom and grid
        bridge.apply(UiIntent::SetPaintCanvasZoom(4));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);
        let image = bridge.render_paint_canvas();
        assert!(image.is_some());
    }

    #[test]
    fn airbrush_continuous_dab_accumulation() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        bridge.apply(UiIntent::SetActiveTool("airbrush".to_string()));
        bridge.apply(UiIntent::SetPaintColor([0.0, 1.0, 0.0]));
        petunia_module_paint::PaintModule::ensure_stack(&mut bridge.state);

        // Initially no stroke active, airbrush_tick returns false
        assert!(!bridge.airbrush_tick());

        // Start 2D stroke
        bridge.paint_2d_stroke(0.5, 0.5, 0);
        let pixel_after_dab1 = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .expect("pixel");

        // Tick airbrush while pointer held
        let ticked = bridge.airbrush_tick();
        assert!(ticked);

        let pixel_after_dab2 = bridge
            .state
            .project
            .active()
            .and_then(|a| a.texture.as_ref())
            .and_then(|t| t.get(128, 128))
            .expect("pixel");

        // Alpha or color density increases or stays solid green
        assert!(pixel_after_dab2[1] >= pixel_after_dab1[1]);
        assert!(pixel_after_dab2[3] >= pixel_after_dab1[3]);

        // Finish stroke
        bridge.paint_2d_stroke(0.5, 0.5, 2);
        assert!(!bridge.airbrush_tick());
    }

    #[test]
    fn project_from_reference_and_bake_reference_intents() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ProjectFromReference);
        assert!(bridge.view_model().status_message.contains("Projected UVs"));

        bridge.apply(UiIntent::BakeReference);
        assert!(
            bridge
                .view_model()
                .status_message
                .contains("No visible reference image")
        );

        let ref_img =
            petunia_core::ReferenceImage::from_rgba("ref1".to_string(), 1, 1, vec![255, 0, 0, 255]);
        bridge.state.project.refs.push(ref_img);

        bridge.apply(UiIntent::ProjectFromReference);
        assert!(bridge.view_model().status_message.contains("Projected UVs"));
        assert_eq!(bridge.state.project.undo.depth(), (2, 0));

        bridge.apply(UiIntent::BakeReference);
        assert!(
            bridge
                .view_model()
                .status_message
                .contains("Baked reference")
        );
        assert_eq!(bridge.state.project.undo.depth(), (3, 0));

        bridge.apply(UiIntent::Undo);
        assert_eq!(bridge.state.project.undo.depth(), (2, 1));
    }

    #[test]
    fn material_editor_updates_values_and_undo() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let before = bridge.state.project.project.materials[0].roughness;
        assert!(bridge.set_active_material_scalar("roughness", 0.2));
        assert_eq!(bridge.view_model().material_roughness, 0.2);
        assert!(bridge.state.undo());
        assert_eq!(bridge.state.project.project.materials[0].roughness, before);
    }

    #[test]
    fn quick_actions_are_controlled_and_execute_commands() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.toggle_quick_action("model.intersect"));
        assert!(
            bridge
                .view_model()
                .quick_action_candidates
                .iter()
                .any(|item| item.pinned)
        );
        bridge.state.project.active_mesh_mut().unwrap().select_all();
        assert!(bridge.execute_quick_action("model.subdivide"));
        assert!(bridge.toggle_quick_action("model.subdivide"));
        assert!(
            !bridge
                .view_model()
                .quick_actions
                .iter()
                .any(|item| item.id == "model.subdivide")
        );
    }

    #[test]
    fn modifier_stack_supports_live_rows_and_apply() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(bridge.add_modifier("mirror"));
        assert!(bridge.add_modifier("symmetry"));
        let rows = bridge.view_model().modifier_rows;
        assert_eq!(rows.len(), 2);
        assert!(bridge.set_modifier_axis(&rows[0].id, 1));
        assert!(bridge.set_modifier_direction(&rows[1].id, false));
        assert!(bridge.set_modifier_enabled(&rows[0].id, false));
        assert_eq!(bridge.view_model().modifier_rows[0].axis, 1);
        assert!(bridge.move_modifier(&rows[1].id, -1));
        assert!(bridge.apply_modifier(&rows[1].id));
        assert_eq!(bridge.view_model().modifier_rows.len(), 1);
        assert!(bridge.state.undo());
    }

    #[test]
    fn scale_tool_exposes_tool_options_immediately() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetActiveTool("scale".into()));
        let vm = bridge.view_model();
        assert!(vm.tool_options_active);
        assert!(vm.tool_options_title.contains("Scale"));
    }

    #[test]
    fn proportional_editing_radius_adjustment_and_falloff() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        // Toggle via UiIntent
        assert!(!bridge.view_model().proportional_editing);
        bridge.apply(UiIntent::ToggleProportionalEditing);
        assert!(bridge.view_model().proportional_editing);

        // Adjust radius (default is 1.5)
        bridge.adjust_proportional_radius(0.5);
        assert!((bridge.view_model().proportional_radius - 2.0).abs() < 1e-4);

        // Set falloff
        bridge.apply(UiIntent::SetProportionalFalloff("linear".into()));
        assert_eq!(bridge.view_model().proportional_falloff, "linear");

        // Interactive mouse wheel zoom during modal adjusts radius
        bridge
            .state
            .begin_modal(petunia_core::ModalKind::Move)
            .unwrap();
        bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: 1.0 });
        assert!((bridge.view_model().proportional_radius - 2.25).abs() < 1e-4);
        bridge.apply_viewport_gesture(ViewportGesture::Zoom { delta: -1.0 });
        assert!((bridge.view_model().proportional_radius - 2.0).abs() < 1e-4);
    }

    #[test]
    fn advanced_snap_to_edge_and_face() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::ToggleSnapEnabled);
        assert!(bridge.view_model().snap_enabled);

        bridge.apply(UiIntent::SetSnapTarget("edge".into()));
        assert_eq!(bridge.view_model().snap_target, "edge");
        assert_eq!(
            bridge.state.session.snap_settings.target,
            petunia_core::SnapTarget::Edge
        );

        // Edge snapping query on cube edge [-1, 1, 1] to [1, 1, 1]
        let mesh = bridge.state.project.active_mesh().unwrap().clone();
        let query_edge = petunia_core::SnapQuery {
            point: glam::Vec3::new(0.0, 1.05, 1.0),
            start_point: Some(glam::Vec3::ZERO),
            settings: &bridge.state.session.snap_settings,
            mesh: Some(&mesh),
        };
        let res_edge = petunia_core::snap_point(query_edge);
        assert!(res_edge.snapped);
        assert_eq!(res_edge.target, petunia_core::SnapTarget::Edge);

        bridge.apply(UiIntent::SetSnapTarget("face".into()));
        assert_eq!(bridge.view_model().snap_target, "face");
        assert_eq!(
            bridge.state.session.snap_settings.target,
            petunia_core::SnapTarget::Face
        );

        // Face snapping query near centroid (0, 0, 1)
        let query_face = petunia_core::SnapQuery {
            point: glam::Vec3::new(0.05, 0.02, 1.0),
            start_point: Some(glam::Vec3::ZERO),
            settings: &bridge.state.session.snap_settings,
            mesh: Some(&mesh),
        };
        let res_face = petunia_core::snap_point(query_face);
        assert!(res_face.snapped);
        assert_eq!(res_face.target, petunia_core::SnapTarget::Face);
    }

    #[test]
    fn face_orientation_and_uv_checker_overlays() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        assert!(!bridge.view_model().show_face_orientation);
        assert!(!bridge.view_model().show_uv_checker);

        bridge.apply(UiIntent::ToggleFaceOrientation);
        assert!(bridge.view_model().show_face_orientation);
        assert!(bridge.state.session.show_face_orientation);

        bridge.apply(UiIntent::ToggleUvChecker);
        assert!(bridge.view_model().show_uv_checker);
        assert!(bridge.state.session.show_uv_checker);

        let flags = petunia_core::FingerprintFlags {
            show_face_orientation: bridge.state.session.show_face_orientation,
            show_uv_checker: bridge.state.session.show_uv_checker,
            ..Default::default()
        };
        assert!(flags.show_face_orientation);
        assert!(flags.show_uv_checker);
    }

    #[test]
    fn profile_2d_rectangle_and_circle_primitives() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::AddProfileRectangle {
            width: 3.0,
            height: 2.0,
        });
        assert_eq!(bridge.state.session.profile.points.len(), 4);
        assert!(bridge.state.session.profile.closed);

        // Generate mesh from rectangle profile
        bridge.generate_profile_extrude();
        let mesh = bridge.state.project.active_mesh().unwrap();
        assert!(!mesh.verts.is_empty());

        // Add circle profile
        bridge.apply(UiIntent::AddProfileCircle {
            radius: 1.5,
            segments: 12,
        });
        assert_eq!(bridge.state.session.profile.points.len(), 12);
        assert!(bridge.state.session.profile.closed);
    }

    #[test]
    fn individual_origins_multi_object_transformation() {
        let mut state = AppState::default();
        let mut mesh1 = petunia_core::Mesh::cube(1.0);
        for v in &mut mesh1.verts {
            v.pos[0] -= 5.0;
        }
        let mut mesh2 = petunia_core::Mesh::cube(1.0);
        for v in &mut mesh2.verts {
            v.pos[0] += 5.0;
        }
        let mut combined = mesh1;
        let v_offset = combined.verts.len() as u32;
        for v in mesh2.verts {
            combined.verts.push(v);
        }
        for mut f in mesh2.faces {
            for idx in &mut f.verts {
                *idx += v_offset;
            }
            combined.faces.push(f);
        }
        combined.select_all();
        *state.project.active_mesh_mut().unwrap() = combined;

        state.set_selection_domain(petunia_core::SelectionDomain::Vertex);
        state.session.pivot_point = petunia_core::PivotPoint::IndividualOrigins;
        state.begin_modal(petunia_core::ModalKind::Scale).unwrap();
        // Scale by 2.0 around individual origins
        state.update_modal(glam::Vec3::ZERO, 2.0).unwrap();

        let transformed = state.project.active_mesh().unwrap();
        // Centroid of cube 1 should remain at -5, centroid of cube 2 at 5
        let center1_x: f32 = transformed.verts[..8].iter().map(|v| v.pos[0]).sum::<f32>() / 8.0;
        let center2_x: f32 = transformed.verts[8..].iter().map(|v| v.pos[0]).sum::<f32>() / 8.0;

        assert!((center1_x - (-5.0)).abs() < 1e-3);
        assert!((center2_x - 5.0).abs() < 1e-3);
    }

    #[test]
    fn lasso_selection_occlusion_and_xray() {
        let mut state = AppState::default();
        // Active cube
        state.set_selection_domain(petunia_core::SelectionDomain::Face);
        state.session.camera.proj = petunia_core::Projection::Perspective;

        // Lasso polygon covering center NDC screen
        let polygon = vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];

        // Without X-Ray, occluded geometry behind is filtered
        state.session.show_xray = false;
        state.select_viewport_lasso(&polygon, false, false);
        let selected_count_no_xray = state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .iter()
            .filter(|f| f.selected)
            .count();

        // With X-Ray, through-selection selects both front and back
        state.session.show_xray = true;
        state.select_viewport_lasso(&polygon, false, false);
        let selected_count_xray = state
            .project
            .active_mesh()
            .unwrap()
            .faces
            .iter()
            .filter(|f| f.selected)
            .count();

        assert!(selected_count_xray >= selected_count_no_xray);
    }

    #[test]
    fn uv_island_90_degree_rotation_in_slint() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Uv));
        bridge.state.session.uv_selected.insert(0);

        let initial_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
        let initial_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];

        // Rotate 90 degrees CW (+90)
        assert!(bridge.uv_rotate_selected(90.0));
        let rotated_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
        let rotated_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];

        assert!((rotated_u0 - initial_u0).abs() > 1e-4 || (rotated_v0 - initial_v0).abs() > 1e-4);

        // Rotate 90 degrees CCW (-90) returns to original position
        assert!(bridge.uv_rotate_selected(-90.0));
        let back_u0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][0];
        let back_v0 = bridge.state.project.active_mesh().unwrap().faces[0].uv[0][1];
        assert!((back_u0 - initial_u0).abs() < 1e-4);
        assert!((back_v0 - initial_v0).abs() < 1e-4);
    }

    #[test]
    fn decal_layer_creation_and_rendering() {
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.apply(UiIntent::SetWorkspace(Workspace::Paint));
        assert!(bridge.add_decal_layer());

        let active = bridge.state.project.active;
        let stack = bridge.state.project.assets[active]
            .paint_stack
            .as_ref()
            .unwrap();
        assert!(stack.layers.iter().any(|layer| matches!(
            layer.kind,
            petunia_project::paint_layers::LayerKind::Decal(_)
        )));
    }
}
