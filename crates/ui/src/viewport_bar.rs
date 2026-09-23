//! Barra de contexto superior do Viewport 3D (`3D View Bar`).
//! Organizada em 7 clusters semânticos coerentes:
//! 1. Seletor de Modo (`Object` vs `Edit`) com dropdown estilizado e ícone vetorial;
//! 2. Alvos de Seleção de Malha segmentados (`Vértice`, `Aresta`, `Face`) exibidos exclusivamente em Edit Mode;
//! 3. Menus Rápidos Padronizados (`View`, `Select`, `Add`, `Mesh` / `Object`) com seta vetorial e atalhos dinâmicos;
//! 4. Orientação de Transformação, Ponto de Pivô e Travamento de Eixos [X][Y][Z];
//! 5. Auxiliares de Edição (Snapping Magnético e Edição Proporcional com ícones canônicos);
//! 6. Diagnóstico de Cena (Overlays e X-Ray com ícones vetoriais dedicados);
//! 7. 4 Esferas de Sombreamento no estilo canônico do Blender (Wireframe, Solid, Material, Rendered).
//!
//! A barra **não** decide mais o que cabe (§46, Wave 4): ela declara as faixas
//! em [`VIEWPORT_BAR`] (quem existe, quem pode cair, o que a seta esconde) e o
//! `adapters::toolbar` mede, decide e desenha. O que morava aqui — somas de
//! `28.0 + 24.0 * 3.0`, `text_w`, breakpoint de 40px de altura — era álgebra de
//! layout refazendo à mão o que o adapter resolve com medidas reais.

use egui::{Color32, CornerRadius, Rect, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
use petunia_core::{
    AppState, ClearSelectionCmd, DeleteAssetCmd, DuplicateAssetCmd, EditMode, InvertSelectionCmd,
    MergeCenterCmd, PivotPoint, PrimitiveKind, ProportionalFalloff, SelectAllCmd, SelectionDomain,
    SnapTarget, SubdivideSelectionCmd, TransformOrientation,
};
use petunia_render::Shading;

use crate::adapters::toolbar::{
    PetuniaResponsiveToolbar, PetuniaToolbarCluster, PetuniaToolbarId, PetuniaToolbarPlan,
    PetuniaToolbarSlot, PetuniaToolbarSpec,
};
use crate::foundation::spacing;
use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::tokens;
use crate::widgets::{
    PetuniaIconButton, PetuniaMenuButton, PetuniaMenuCheckboxItem, PetuniaMenuItem,
    PetuniaMenuRadioItem, petunia_menu_separator,
};

/// Identificador da barra (raiz dos ids internos do adapter).
const TOOLBAR_ID: &str = "viewport-context-bar";

/// Faixas da barra (§46). O adapter mede e decide; aqui existe só a semântica
/// de produto: quais faixas existem, qual pode cair e o que a seta esconde.
const SLOT_DOMAIN: PetuniaToolbarId = "viewport.domain";
const SLOT_MENUS: PetuniaToolbarId = "viewport.menus";
const SLOT_TRANSFORM: PetuniaToolbarId = "viewport.transform";
const SLOT_SNAP_PROP: PetuniaToolbarId = "viewport.snap-prop";
const SLOT_DISPLAY: PetuniaToolbarId = "viewport.display";
const SLOT_OVERFLOW: PetuniaToolbarId = "viewport.overflow";

/// Declaração da barra: linha principal (navegação) e linha secundária
/// (edição/visualização), com os ranks de queda.
///
/// `snap-prop` tem rank maior que `transform`: com pouco espaço é a orientação
/// de transformação que vai para a seta primeiro — comportamento preservado da
/// barra original, agora declarado em vez de implícito numa ordem de `for`.
/// Domínio, Menus e Display são núcleo: nunca saem da barra.
static VIEWPORT_BAR: PetuniaToolbarSpec = PetuniaToolbarSpec {
    primary: &[
        PetuniaToolbarCluster::pinned(SLOT_DOMAIN),
        PetuniaToolbarCluster::pinned(SLOT_MENUS),
    ],
    secondary: &[
        PetuniaToolbarCluster::overflowable(SLOT_TRANSFORM, 10),
        PetuniaToolbarCluster::overflowable(SLOT_SNAP_PROP, 20),
        PetuniaToolbarCluster::pinned(SLOT_DISPLAY),
    ],
    max_rows: 2,
    overflow: Some(SLOT_OVERFLOW),
};

/// Renderiza a barra de contexto horizontal do Viewport 3D.
///
/// Responsiva em duas dimensões, sem nenhuma conta local: a largura decide o
/// que fica visível (o excedente vai para a seta, sempre alcançável) e a altura
/// decide uma ou duas linhas. Quem mede e decide é
/// [`PetuniaResponsiveToolbar`] — larguras reais de sonda, gaps e divisores
/// medidos do tema, distribuição no `taffy`.
pub fn draw(ui: &mut Ui, state: &mut AppState) -> PetuniaToolbarPlan {
    // Interceptação defensiva de atalhos globais de modo se nenhum campo de texto estiver focado
    handle_keyboard_shortcuts(ui, state);

    let toolbar = PetuniaResponsiveToolbar::new(TOOLBAR_ID, &VIEWPORT_BAR);
    toolbar.show(ui, &mut |ui, slot| draw_slot(ui, state, slot))
}

/// Desenho de uma faixa declarada em [`VIEWPORT_BAR`].
fn draw_slot(ui: &mut Ui, state: &mut AppState, slot: PetuniaToolbarSlot<'_>) {
    match slot.id {
        SLOT_DOMAIN => draw_selection_domain_cluster(ui, state),
        SLOT_MENUS => draw_viewport_actions_cluster(ui, state),
        SLOT_TRANSFORM => draw_transform_cluster(ui, state),
        SLOT_SNAP_PROP => draw_snap_and_prop_cluster(ui, state),
        SLOT_DISPLAY => draw_display_cluster(ui, state),
        SLOT_OVERFLOW => draw_overflow_button(ui, state, slot.hidden),
        _ => {}
    }
}

/// Faixa de visualização: alternâncias de cena (Overlays, X-Ray) e as esferas
/// de sombreamento como **uma** unidade de barra.
fn draw_display_cluster(ui: &mut Ui, state: &mut AppState) {
    draw_display_toggles_cluster(ui, state);
    spacing::hspace(ui, spacing::RELATED);
    draw_shading_spheres_cluster(ui, state);
}

/// Seta de overflow: faixas ocultas por falta de largura, sempre operáveis.
fn draw_overflow_button(ui: &mut Ui, state: &mut AppState, hidden: &[PetuniaToolbarId]) {
    let tip = state.t("viewport.overflow");
    PetuniaMenuButton::chevron_only()
        .tooltip(&tip)
        .show(ui, |ui| {
            ui.set_min_width(168.0);
            if hidden.contains(&SLOT_SNAP_PROP) {
                let snap_label = state.t("viewport.snap_tip");
                if PetuniaMenuCheckboxItem::new(&snap_label, state.snap_enabled)
                    .show(ui)
                    .clicked()
                {
                    state.snap_enabled = !state.snap_enabled;
                    state.snap_settings.enabled = state.snap_enabled;
                    state.mark_dirty();
                }
                let prop_label = state.t("viewport.prop_tip");
                if PetuniaMenuCheckboxItem::new(&prop_label, state.proportional_editing)
                    .show(ui)
                    .clicked()
                {
                    state.proportional_editing = !state.proportional_editing;
                    state.proportional_settings.enabled = state.proportional_editing;
                    state.mark_dirty();
                }
                petunia_menu_separator(ui);
            }
            if hidden.contains(&SLOT_TRANSFORM) {
                ui.label(
                    egui::RichText::new(state.t("viewport.orientation"))
                        .strong()
                        .size(12.0)
                        .color(tokens::TEXT_PRIMARY),
                );
                for orient in TransformOrientation::all() {
                    if PetuniaMenuRadioItem::new(
                        orient.as_str(),
                        state.transform_orientation == orient,
                    )
                    .show(ui)
                    .clicked()
                    {
                        state.transform_orientation = orient;
                        state.mark_dirty();
                    }
                }
                petunia_menu_separator(ui);
                ui.label(
                    egui::RichText::new(state.t("viewport.pivot"))
                        .strong()
                        .size(12.0)
                        .color(tokens::TEXT_PRIMARY),
                );
                for pivot in PivotPoint::all() {
                    if PetuniaMenuRadioItem::new(pivot.as_str(), state.pivot_point == pivot)
                        .show(ui)
                        .clicked()
                    {
                        state.pivot_point = pivot;
                        state.mark_dirty();
                    }
                }
                petunia_menu_separator(ui);
                for (axis_idx, label) in [(0, "X"), (1, "Y"), (2, "Z")] {
                    let lock_label = format!("{} {label}", state.t("viewport.axis_lock"));
                    if PetuniaMenuCheckboxItem::new(&lock_label, state.is_axis_locked(axis_idx))
                        .show(ui)
                        .clicked()
                    {
                        state.toggle_axis_lock(axis_idx);
                    }
                }
            }
        });
}

/// Grupos canônicos do menu Add (§36): três famílias, dez espécies, sem
/// duplicata. Fonte única usada pelo menu e pelo teste de cobertura.
fn primitive_menu_groups() -> [(petunia_config::TextId, &'static [PrimitiveKind]); 3] {
    use petunia_config::text_id as T;
    [
        (
            T::PRIMS_GROUP_BASIC,
            [
                PrimitiveKind::Cube,
                PrimitiveKind::Plane,
                PrimitiveKind::Wedge,
            ]
            .as_slice(),
        ),
        (
            T::PRIMS_GROUP_ROUND,
            [
                PrimitiveKind::Cylinder,
                PrimitiveKind::Cone,
                PrimitiveKind::Circle,
                PrimitiveKind::Torus,
            ]
            .as_slice(),
        ),
        (
            T::PRIMS_GROUP_ORGANIC,
            [
                PrimitiveKind::Sphere,
                PrimitiveKind::Icosphere,
                PrimitiveKind::Capsule,
            ]
            .as_slice(),
        ),
    ]
}

/// Trata atalhos de teclado (Tab, 1, 2, 3, 0) diretamente no egui para máxima responsividade.
fn handle_keyboard_shortcuts(ui: &mut Ui, state: &mut AppState) {
    // Shift+Tab: Alterna Snapping Magnético (P3D-040, P3D-079)
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)) {
        state.snap_enabled = !state.snap_enabled;
        state.snap_settings.enabled = state.snap_enabled;
        state.mark_dirty();
        return;
    }

    // Tab: Alterna entre Object Domain e o último domínio de componente (P3D-015)
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
        state.cycle_selection_domain();
        return;
    }

    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }

    // 0: Modo/Domínio Objeto
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Num0)) {
        state.set_selection_domain(SelectionDomain::Object);
    }

    // 1: Vértice
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Num1)) {
        state.set_selection_domain(SelectionDomain::Vertex);
    }

    // 2: Aresta
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Num2)) {
        state.set_selection_domain(SelectionDomain::Edge);
    }

    // 3: Face
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Num3)) {
        state.set_selection_domain(SelectionDomain::Face);
    }
}

/// Cluster 1 & 2: Domínio Unificado de Seleção e Interação (Object / Vertex / Edge / Face) (P3D-015).
/// Elimina a divisão artificial entre Object Mode e Edit Mode.
fn draw_selection_domain_cluster(ui: &mut Ui, state: &mut AppState) {
    let domains = [
        (
            SelectionDomain::Object,
            PetuniaIcon::ModeObject,
            "0",
            state.t("modes.object"),
        ),
        (
            SelectionDomain::Vertex,
            PetuniaIcon::SelectVertex,
            "1",
            state.t("modes.vertex"),
        ),
        (
            SelectionDomain::Edge,
            PetuniaIcon::SelectEdge,
            "2",
            state.t("modes.edge"),
        ),
        (
            SelectionDomain::Face,
            PetuniaIcon::SelectFace,
            "3",
            state.t("modes.face"),
        ),
    ];

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let active_domain = state.selection_domain();

        for (domain, icon, shortcut, name) in domains {
            let is_active = active_domain == domain;
            let (bg, fg) = if is_active {
                (tokens::ACCENT_BLUE, tokens::TEXT_ACTIVE)
            } else {
                (tokens::BG_SURFACE, tokens::TEXT_SECONDARY)
            };

            let width = if domain == SelectionDomain::Object {
                28.0
            } else {
                24.0
            };
            let (rect, resp) = ui.allocate_exact_size(vec2(width, 22.0), egui::Sense::click());
            resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, is_active, &name));
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let fill = if is_active {
                    bg
                } else if resp.hovered() {
                    tokens::BG_SURFACE_HOVER
                } else {
                    bg
                };
                painter.rect_filled(rect, tokens::RADIUS_CONTROL, fill);

                let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
                IconRegistry::paint(ui.ctx(), painter, &icon, icon_rect, fg);
                if resp.has_focus() {
                    painter.rect_stroke(
                        rect,
                        tokens::RADIUS_CONTROL,
                        tokens::stroke_focus(),
                        StrokeKind::Inside,
                    );
                }
            }

            if resp
                .on_hover_text(format!("{name} · [{shortcut}] (Tab: alternar)"))
                .clicked()
            {
                state.set_selection_domain(domain);
            }
        }
    });
}

/// Cluster 3: Menus rápidos padronizados com ícones (View, Select, Add, Objeto/Malha).
fn draw_viewport_actions_cluster(ui: &mut Ui, state: &mut AppState) {
    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.hovered.weak_bg_fill = tokens::BG_SURFACE_HOVER;
    visuals.widgets.active.weak_bg_fill = tokens::ACCENT_BLUE;

    // Menu View
    let view_label = state.t("menu.view");
    PetuniaMenuButton::new(&view_label).show(ui, |ui| {
        let sc_frame = state
            .ui
            .keybinds
            .shortcut_for("view.frame_selection")
            .unwrap_or_else(|| "F".into());
        if PetuniaMenuItem::new(&state.t("view.frame"))
            .shortcut(Some(&sc_frame))
            .show(ui)
            .clicked()
        {
            state.frame_selection();
            ui.close();
        }

        let sc_frame_all = state
            .ui
            .keybinds
            .shortcut_for("view.frame_all")
            .unwrap_or_else(|| "Home".into());
        if PetuniaMenuItem::new(&state.t("view.frame_all"))
            .shortcut(Some(&sc_frame_all))
            .show(ui)
            .clicked()
        {
            state.frame_all();
            ui.close();
        }

        let sc_reset = state
            .ui
            .keybinds
            .shortcut_for("view.reset_camera")
            .unwrap_or_else(|| "Shift+Home".into());
        if PetuniaMenuItem::new(&state.t("camera.reset"))
            .shortcut(Some(&sc_reset))
            .show(ui)
            .clicked()
        {
            state.camera_frame = None;
            state.camera.reset();
            state.mark_dirty();
            ui.close();
        }

        petunia_menu_separator(ui);

        let sc_proj = state
            .ui
            .keybinds
            .shortcut_for("view.toggle_projection")
            .unwrap_or_else(|| "O".into());
        if PetuniaMenuItem::new(&state.t("camera.projection"))
            .shortcut(Some(&sc_proj))
            .show(ui)
            .clicked()
        {
            state.camera.toggle_projection();
            state.mark_dirty();
            ui.close();
        }

        petunia_menu_separator(ui);

        if PetuniaMenuItem::new(&state.t("camera.front"))
            .shortcut(Some("Numpad 1"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Front);
            state.mark_dirty();
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("camera.back"))
            .shortcut(Some("Ctrl+Numpad 1"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Back);
            state.mark_dirty();
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("camera.right"))
            .shortcut(Some("Numpad 3"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Right);
            state.mark_dirty();
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("camera.left"))
            .shortcut(Some("Ctrl+Numpad 3"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Left);
            state.mark_dirty();
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("camera.top"))
            .shortcut(Some("Numpad 7"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Top);
            state.mark_dirty();
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("camera.bottom"))
            .shortcut(Some("Ctrl+Numpad 7"))
            .show(ui)
            .clicked()
        {
            state.camera.set_preset(petunia_core::ViewPreset::Bottom);
            state.mark_dirty();
            ui.close();
        }

        petunia_menu_separator(ui);

        let iso_label = state.t("camera.isometric");
        PetuniaMenuButton::new(&iso_label).show(ui, |ui| {
            if PetuniaMenuItem::new(&state.t("camera.iso_ne"))
                .show(ui)
                .clicked()
            {
                state
                    .camera
                    .set_preset(petunia_core::ViewPreset::IsometricNE);
                state.mark_dirty();
                ui.close();
            }
            if PetuniaMenuItem::new(&state.t("camera.iso_nw"))
                .show(ui)
                .clicked()
            {
                state
                    .camera
                    .set_preset(petunia_core::ViewPreset::IsometricNW);
                state.mark_dirty();
                ui.close();
            }
            if PetuniaMenuItem::new(&state.t("camera.iso_se"))
                .show(ui)
                .clicked()
            {
                state
                    .camera
                    .set_preset(petunia_core::ViewPreset::IsometricSE);
                state.mark_dirty();
                ui.close();
            }
            if PetuniaMenuItem::new(&state.t("camera.iso_sw"))
                .show(ui)
                .clicked()
            {
                state
                    .camera
                    .set_preset(petunia_core::ViewPreset::IsometricSW);
                state.mark_dirty();
                ui.close();
            }
        });

        petunia_menu_separator(ui);

        let sc_ref = state
            .ui
            .keybinds
            .shortcut_for("window.reference_manager")
            .unwrap_or_else(|| "Shift+R".into());
        if PetuniaMenuItem::new(&state.t("refs.manage"))
            .icon(PetuniaIcon::ReferenceImage)
            .shortcut(Some(&sc_ref))
            .show(ui)
            .clicked()
        {
            state.ui.show_reference_manager = true;
            state.mark_dirty();
            ui.close();
        }
    });

    // Menu Select
    let select_label = state.t("tools.select");
    PetuniaMenuButton::new(&select_label).show(ui, |ui| {
        if PetuniaMenuItem::new(&state.t("actions.select_all"))
            .shortcut(Some("A"))
            .show(ui)
            .clicked()
        {
            let _ = state.dispatch(&SelectAllCmd);
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("actions.deselect"))
            .shortcut(Some("Alt+A"))
            .show(ui)
            .clicked()
        {
            let _ = state.dispatch(&ClearSelectionCmd);
            ui.close();
        }
        if PetuniaMenuItem::new(&state.t("actions.invert"))
            .shortcut(Some("Ctrl+I"))
            .show(ui)
            .clicked()
        {
            let _ = state.dispatch(&InvertSelectionCmd);
            ui.close();
        }
    });

    // Menu Add: três famílias de formas (§36), caminho único via sessão.
    let mut spawn_kind: Option<PrimitiveKind> = None;
    let add_label = state.t("tools.primitives");
    PetuniaMenuButton::new(&add_label).show(ui, |ui| {
        for (group, kinds) in primitive_menu_groups() {
            ui.label(
                egui::RichText::new(state.t_id(group))
                    .size(10.5)
                    .color(tokens::TEXT_MUTED)
                    .strong(),
            );
            for kind in kinds {
                let label = state.t_id(kind.name_key());
                if PetuniaMenuItem::new(&label)
                    .icon(PetuniaIcon::AddPrimitive)
                    .show(ui)
                    .on_hover_text(&label)
                    .clicked()
                {
                    spawn_kind = Some(*kind);
                    ui.close();
                }
            }
            petunia_menu_separator(ui);
        }
        if PetuniaMenuItem::new(&state.t("ui.refs"))
            .icon(PetuniaIcon::ReferenceImage)
            .shortcut(Some("Shift+R"))
            .show(ui)
            .clicked()
        {
            state.ui.show_reference_manager = true;
            state.mark_dirty();
            ui.close();
        }
    });

    // Caminho canônico único: sessão de criação (§82), nunca inserção direta.
    if let Some(kind) = spawn_kind {
        state.begin_primitive(kind, None);
    }

    // Menu contextual: domínio Object (objeto inteiro) ou domínio de componente (malha).
    if state.edit_mode() == EditMode::Object {
        let object_label = state.t("modes.object");
        PetuniaMenuButton::new(&object_label).show(ui, |ui| {
            let sc_dup = state
                .ui
                .keybinds
                .shortcut_for("model.duplicate")
                .unwrap_or_else(|| "Shift+D".into());
            if PetuniaMenuItem::new(&state.t("actions.duplicate"))
                .icon(PetuniaIcon::Duplicate)
                .shortcut(Some(&sc_dup))
                .show(ui)
                .clicked()
            {
                let _ = state.dispatch(&DuplicateAssetCmd { asset_index: None });
                ui.close();
            }

            let sc_del = state
                .ui
                .keybinds
                .shortcut_for("model.delete")
                .unwrap_or_else(|| "Delete".into());
            if PetuniaMenuItem::new(&state.t("actions.delete"))
                .icon(PetuniaIcon::Delete)
                .shortcut(Some(&sc_del))
                .show(ui)
                .clicked()
            {
                let _ = state.dispatch(&DeleteAssetCmd { asset_index: None });
                ui.close();
            }
            petunia_menu_separator(ui);
            if PetuniaMenuItem::new(&state.t("actions.cursor_to_origin"))
                .icon(PetuniaIcon::Cursor3D)
                .show(ui)
                .clicked()
            {
                state.cursor_3d = [0.0, 0.0, 0.0];
                state.mark_dirty();
                ui.close();
            }
        });
    } else {
        let mesh_label = state.t("ui.mesh");
        PetuniaMenuButton::new(&mesh_label).show(ui, |ui| {
            let sc_ext = state
                .ui
                .keybinds
                .shortcut_for("model.extrude")
                .unwrap_or_else(|| "E".into());
            if PetuniaMenuItem::new(&state.t("tools.extrude"))
                .icon(PetuniaIcon::Extrude)
                .shortcut(Some(&sc_ext))
                .show(ui)
                .clicked()
            {
                state.active_tool = "extrude".into();
                state.mark_dirty();
                ui.close();
            }

            let sc_ext_ind = state
                .ui
                .keybinds
                .shortcut_for("model.extrude_individual")
                .unwrap_or_else(|| "Alt+E".into());
            if PetuniaMenuItem::new(&state.t("tools.extrude_individual"))
                .icon(PetuniaIcon::Extrude)
                .shortcut(Some(&sc_ext_ind))
                .show(ui)
                .clicked()
            {
                let dist = if state.extrude_dist == 0.0 {
                    0.5
                } else {
                    state.extrude_dist
                };
                let _ = state.dispatch(&petunia_core::ExtrudeIndividualCmd { dist });
                ui.close();
            }

            let sc_ins = state
                .ui
                .keybinds
                .shortcut_for("model.inset")
                .unwrap_or_else(|| "I".into());
            if PetuniaMenuItem::new(&state.t("tools.inset"))
                .icon(PetuniaIcon::Inset)
                .shortcut(Some(&sc_ins))
                .show(ui)
                .clicked()
            {
                state.active_tool = "inset".into();
                state.mark_dirty();
                ui.close();
            }

            let sc_bev = state
                .ui
                .keybinds
                .shortcut_for("model.bevel")
                .unwrap_or_else(|| "Ctrl+B".into());
            if PetuniaMenuItem::new(&state.t("tools.bevel"))
                .icon(PetuniaIcon::Bevel)
                .shortcut(Some(&sc_bev))
                .show(ui)
                .clicked()
            {
                state.active_tool = "bevel".into();
                state.mark_dirty();
                ui.close();
            }

            if PetuniaMenuItem::new(&state.t("tools.loop_cut"))
                .icon(PetuniaIcon::LoopCut)
                .shortcut(Some("Ctrl+R"))
                .show(ui)
                .clicked()
            {
                state.active_tool = "loop_cut".into();
                state.mark_dirty();
                ui.close();
            }

            if PetuniaMenuItem::new(&state.t("tools.knife"))
                .icon(PetuniaIcon::Knife)
                .shortcut(Some("K"))
                .show(ui)
                .clicked()
            {
                state.active_tool = "knife".into();
                state.mark_dirty();
                ui.close();
            }

            petunia_menu_separator(ui);

            if PetuniaMenuItem::new(&state.t("actions.subdivide"))
                .icon(PetuniaIcon::Subdivide)
                .show(ui)
                .clicked()
            {
                let _ = state.dispatch(&SubdivideSelectionCmd);
                ui.close();
            }

            if PetuniaMenuItem::new(&state.t("actions.merge_center"))
                .icon(PetuniaIcon::Custom("merge"))
                .show(ui)
                .clicked()
            {
                let _ = state.dispatch(&MergeCenterCmd);
                ui.close();
            }

            if PetuniaMenuItem::new(&state.t("tools.flip_diagonal"))
                .icon(PetuniaIcon::Custom("flip_diagonal"))
                .show(ui)
                .clicked()
            {
                let _ = state.dispatch(&petunia_core::FlipDiagonalCmd);
                ui.close();
            }

            if PetuniaMenuItem::new(&state.t("tools.revolve"))
                .icon(PetuniaIcon::Custom("revolve"))
                .show(ui)
                .clicked()
            {
                state.active_tool = "revolve".into();
                state.mark_dirty();
                ui.close();
            }
        });
    }
}

/// Cluster 4: Orientação de transformação, Ponto de pivô e Travamento de Eixos.
fn draw_transform_cluster(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 0.0);

        let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::OrientationGlobal,
                rect,
                tokens::TEXT_SECONDARY,
            );
        }

        egui::ComboBox::from_id_salt("transform_orientation")
            .selected_text(
                egui::RichText::new(state.transform_orientation.as_str())
                    .size(11.0)
                    .color(tokens::TEXT_PRIMARY),
            )
            .width(62.0)
            .show_ui(ui, |ui| {
                for orient in TransformOrientation::all() {
                    let is_sel = state.transform_orientation == orient;
                    if ui.selectable_label(is_sel, orient.as_str()).clicked() {
                        state.transform_orientation = orient;
                        state.mark_dirty();
                    }
                }
            });

        ui.add_space(2.0);

        let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::PivotMedian,
                rect,
                tokens::TEXT_SECONDARY,
            );
        }

        egui::ComboBox::from_id_salt("pivot_point")
            .selected_text(
                egui::RichText::new(state.pivot_point.as_str())
                    .size(11.0)
                    .color(tokens::TEXT_PRIMARY),
            )
            .width(88.0)
            .show_ui(ui, |ui| {
                for pivot in PivotPoint::all() {
                    let is_sel = state.pivot_point == pivot;
                    if ui.selectable_label(is_sel, pivot.as_str()).clicked() {
                        state.pivot_point = pivot;
                        state.mark_dirty();
                    }
                }
            });
    });

    ui.add_space(2.0);

    // Controles e Indicador Visual de Travamento de Eixos na Barra do Viewport
    draw_axis_lock_controls(ui, state);
}

fn draw_axis_lock_controls(ui: &mut Ui, state: &mut AppState) {
    let any_locked = state.is_axis_locked(0) || state.is_axis_locked(1) || state.is_axis_locked(2);
    let lock_icon = if any_locked {
        PetuniaIcon::Lock
    } else {
        PetuniaIcon::Unlock
    };
    let lock_col = if any_locked {
        tokens::TEXT_ACTIVE
    } else {
        tokens::TEXT_MUTED
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 0.0);

        let (rect, _) = ui.allocate_exact_size(vec2(14.0, 18.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            let icon_rect = Rect::from_center_size(rect.center(), vec2(13.0, 13.0));
            IconRegistry::paint(ui.ctx(), ui.painter(), &lock_icon, icon_rect, lock_col);
        }

        for (axis_idx, label, color) in [
            (0, "X", tokens::AXIS_X),
            (1, "Y", tokens::AXIS_Y),
            (2, "Z", tokens::AXIS_Z),
        ] {
            let is_locked = state.is_axis_locked(axis_idx);
            let btn = if is_locked {
                egui::Button::new(
                    egui::RichText::new(label)
                        .strong()
                        .size(10.5)
                        .color(Color32::WHITE),
                )
                .fill(color)
                .corner_radius(tokens::RADIUS_CONTROL)
            } else {
                egui::Button::new(egui::RichText::new(label).size(10.5).color(color))
                    .fill(tokens::BG_SURFACE)
                    .corner_radius(tokens::RADIUS_CONTROL)
            };

            let tooltip = if is_locked {
                format!("{} {label}", state.t("viewport.axis_unlock"))
            } else {
                format!("{} {label}", state.t("viewport.axis_lock"))
            };

            if ui.add(btn).on_hover_text(tooltip).clicked() {
                state.toggle_axis_lock(axis_idx);
            }
        }

        // Se houver restrição ativa, exibir badge estilizado
        if let Some((label, rgb)) = state.active_axis_constraint_label() {
            ui.add_space(2.0);
            let badge_bg = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            let (rect, _) = ui.allocate_exact_size(
                vec2(20.0 + (label.len() as f32) * 6.5, 18.0),
                egui::Sense::hover(),
            );
            ui.painter()
                .rect_filled(rect, tokens::RADIUS_CONTROL, badge_bg);

            let icon_rect =
                Rect::from_min_size(pos2(rect.min.x + 3.0, rect.min.y + 2.5), vec2(13.0, 13.0));
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::Lock,
                icon_rect,
                Color32::WHITE,
            );

            ui.painter().text(
                pos2(rect.min.x + 18.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::monospace(10.0),
                Color32::WHITE,
            );
        }
    });
}

/// Cluster 5: Snapping magnético e Edição proporcional com ícones canônicos e popovers (P3D-079, P3D-080).
fn draw_snap_and_prop_cluster(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(1.0, 0.0);

        // --- Snapping Magnético (Segmented: Botão Mestre + Chevron Popover) ---
        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 22.0), egui::Sense::click());
        let snap_tip = state.t("viewport.snap_tip");
        resp.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                true,
                state.snap_enabled,
                snap_tip.clone(),
            )
        });
        if ui.is_rect_visible(rect) {
            let is_active = state.snap_enabled;
            let bg = if is_active {
                tokens::ACCENT_BLUE
            } else if resp.hovered() {
                tokens::BG_SURFACE_HOVER
            } else {
                tokens::BG_SURFACE
            };
            let fg = if is_active {
                tokens::TEXT_ACTIVE
            } else {
                tokens::TEXT_SECONDARY
            };
            ui.painter().rect_filled(
                rect,
                CornerRadius {
                    nw: 4,
                    sw: 4,
                    ne: 0,
                    se: 0,
                },
                bg,
            );
            let icon_rect = Rect::from_center_size(rect.center(), vec2(15.0, 15.0));
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::SnapMagnet,
                icon_rect,
                fg,
            );
            if resp.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius {
                        nw: 4,
                        sw: 4,
                        ne: 0,
                        se: 0,
                    },
                    tokens::stroke_focus(),
                    StrokeKind::Inside,
                );
            }
        }
        if resp.on_hover_text(snap_tip).clicked() {
            state.snap_enabled = !state.snap_enabled;
            state.snap_settings.enabled = state.snap_enabled;
            state.mark_dirty();
        }

        PetuniaMenuButton::chevron_only().show(ui, |ui| {
            ui.set_min_width(144.0);
            ui.label(
                egui::RichText::new("Opções de Snapping")
                    .strong()
                    .size(12.0)
                    .color(tokens::TEXT_PRIMARY),
            );
            ui.separator();

            let mut dirty = false;
            ui.label(
                egui::RichText::new("Alvo de Snap")
                    .size(11.0)
                    .color(tokens::TEXT_SECONDARY),
            );
            for target in SnapTarget::all() {
                let is_sel = state.snap_settings.target == target;
                if ui.selectable_label(is_sel, target.label()).clicked() {
                    state.snap_settings.target = target;
                    dirty = true;
                }
            }

            ui.separator();
            if ui
                .add(
                    egui::Slider::new(&mut state.snap_settings.grid_spacing, 0.1..=10.0)
                        .text("Espaçamento Grade")
                        .step_by(0.1),
                )
                .changed()
            {
                dirty = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.snap_settings.snap_distance, 0.05..=2.0)
                        .text("Distância de Snap")
                        .step_by(0.05),
                )
                .changed()
            {
                dirty = true;
            }

            if dirty {
                state.mark_dirty();
            }
        });

        ui.add_space(3.0);

        // --- Edição Proporcional (Segmented: Botão Mestre + Chevron Popover) ---
        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 22.0), egui::Sense::click());
        let prop_tip = state.t("viewport.prop_tip");
        resp.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                true,
                state.proportional_editing,
                prop_tip.clone(),
            )
        });
        if ui.is_rect_visible(rect) {
            let is_active = state.proportional_editing;
            let bg = if is_active {
                tokens::ACCENT_BLUE
            } else if resp.hovered() {
                tokens::BG_SURFACE_HOVER
            } else {
                tokens::BG_SURFACE
            };
            let fg = if is_active {
                tokens::TEXT_ACTIVE
            } else {
                tokens::TEXT_SECONDARY
            };
            ui.painter().rect_filled(
                rect,
                CornerRadius {
                    nw: 4,
                    sw: 4,
                    ne: 0,
                    se: 0,
                },
                bg,
            );
            let icon_rect = Rect::from_center_size(rect.center(), vec2(15.0, 15.0));
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::ProportionalEditing,
                icon_rect,
                fg,
            );
            if resp.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius {
                        nw: 4,
                        sw: 4,
                        ne: 0,
                        se: 0,
                    },
                    tokens::stroke_focus(),
                    StrokeKind::Inside,
                );
            }
        }
        if resp.on_hover_text(prop_tip).clicked() {
            state.proportional_editing = !state.proportional_editing;
            state.proportional_settings.enabled = state.proportional_editing;
            state.mark_dirty();
        }

        PetuniaMenuButton::chevron_only().show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.label(
                egui::RichText::new("Edição Proporcional")
                    .strong()
                    .size(12.0)
                    .color(tokens::TEXT_PRIMARY),
            );
            ui.separator();

            let mut dirty = false;
            ui.label(
                egui::RichText::new("Curva de Decaimento")
                    .size(11.0)
                    .color(tokens::TEXT_SECONDARY),
            );
            for falloff in ProportionalFalloff::all() {
                let is_sel = state.proportional_settings.falloff == falloff;
                if ui.selectable_label(is_sel, falloff.label()).clicked() {
                    state.proportional_settings.falloff = falloff;
                    dirty = true;
                }
            }

            ui.separator();
            if ui
                .add(
                    egui::Slider::new(&mut state.proportional_settings.radius, 0.1..=20.0)
                        .text("Raio de Influência")
                        .step_by(0.1),
                )
                .changed()
            {
                dirty = true;
            }

            if dirty {
                state.mark_dirty();
            }
        });
    });
}

/// Cluster 6: Alternâncias de visualização de cena (Overlays e X-Ray com ícones vetoriais).
fn draw_display_toggles_cluster(ui: &mut Ui, state: &mut AppState) {
    // Overlays (Segmented Toggle + Popover Dropdown, P3D-010)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(1.0, 0.0);

        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 22.0), egui::Sense::click());
        let overlays_tip = state.t("viewport.overlays_tip");
        resp.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                true,
                state.show_overlays,
                overlays_tip.clone(),
            )
        });
        if ui.is_rect_visible(rect) {
            let is_active = state.show_overlays;
            let bg = if is_active {
                tokens::ACCENT_BLUE
            } else if resp.hovered() {
                tokens::BG_SURFACE_HOVER
            } else {
                tokens::BG_SURFACE
            };
            let fg = if is_active {
                tokens::TEXT_ACTIVE
            } else {
                tokens::TEXT_SECONDARY
            };
            ui.painter().rect_filled(
                rect,
                CornerRadius {
                    nw: 4,
                    sw: 4,
                    ne: 0,
                    se: 0,
                },
                bg,
            );
            let icon_rect = Rect::from_center_size(rect.center(), vec2(15.0, 15.0));
            IconRegistry::paint(
                ui.ctx(),
                ui.painter(),
                &PetuniaIcon::Overlays,
                icon_rect,
                fg,
            );
            if resp.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius {
                        nw: 4,
                        sw: 4,
                        ne: 0,
                        se: 0,
                    },
                    tokens::stroke_focus(),
                    StrokeKind::Inside,
                );
            }
        }
        if resp.on_hover_text(overlays_tip).clicked() {
            state.show_overlays = !state.show_overlays;
            state.mark_dirty();
        }

        PetuniaMenuButton::chevron_only().show(ui, |ui| {
            ui.set_min_width(180.0);
            ui.label(
                egui::RichText::new("Opções de Overlay")
                    .strong()
                    .size(12.0)
                    .color(tokens::text_primary(state)),
            );
            ui.separator();

            let mut dirty = false;
            let grid_cfg_id = ui.make_persistent_id("show_grid_settings_popover");
            let mut show_cfg = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(grid_cfg_id).unwrap_or(false));

            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut state.show_grid, "Grade 3D (Grid)")
                    .changed()
                {
                    dirty = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if PetuniaIconButton::new(
                        PetuniaIcon::Settings,
                        "Configurações da Grade 3D e Guias Isométricas",
                        20.0,
                    )
                    .selected(show_cfg)
                    .show(ui)
                    .clicked()
                    {
                        show_cfg = !show_cfg;
                        ui.ctx().data_mut(|d| d.insert_temp(grid_cfg_id, show_cfg));
                    }
                });
            });

            if show_cfg {
                egui::Frame::group(ui.style())
                    .fill(tokens::bg_surface(state))
                    .stroke(tokens::stroke_border_dyn(state))
                    .corner_radius(tokens::RADIUS_CONTROL)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Configuração da Grade:")
                                .size(10.5)
                                .color(tokens::text_secondary(state)),
                        );
                        if ui
                            .add(
                                egui::Slider::new(&mut state.grid_settings.size, 2.0..=50.0)
                                    .text("Tamanho")
                                    .step_by(1.0),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut state.grid_settings.subdivisions,
                                    1.0..=50.0,
                                )
                                .text("Subdivisões"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if ui
                            .add(
                                egui::Slider::new(&mut state.grid_settings.opacity, 0.05..=1.0)
                                    .text("Opacidade")
                                    .step_by(0.05),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if ui
                            .checkbox(
                                &mut state.grid_settings.show_isometric_guide,
                                "Guias Isométricas",
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if state.grid_settings.show_isometric_guide
                            && ui
                                .add(
                                    egui::Slider::new(
                                        &mut state.grid_settings.isometric_angle_deg,
                                        15.0..=60.0,
                                    )
                                    .suffix("°")
                                    .text("Ângulo"),
                                )
                                .changed()
                        {
                            dirty = true;
                        }
                    });
            }
            if ui
                .checkbox(&mut state.show_axes, "Eixos Mundiais (Axes)")
                .changed()
            {
                dirty = true;
            }
            if ui.checkbox(&mut state.show_cursor, "Cursor 3D").changed() {
                dirty = true;
            }
            if ui
                .checkbox(
                    &mut state.show_wireframe_overlay,
                    "Aramado (Wireframe Overlay)",
                )
                .changed()
            {
                dirty = true;
            }
            if ui
                .checkbox(&mut state.show_triangulation, "Triangulação (Diagonais)")
                .changed()
            {
                dirty = true;
            }
            if ui
                .checkbox(&mut state.show_nav_hud, "Navigation HUD (Orientação)")
                .changed()
            {
                dirty = true;
            }

            ui.separator();

            if PetuniaMenuItem::new("Gerenciador de Referências...")
                .icon(PetuniaIcon::ReferenceImage)
                .shortcut(Some("Shift+R"))
                .show(ui)
                .clicked()
            {
                state.ui.show_reference_manager = true;
                dirty = true;
                ui.close();
            }

            if dirty {
                state.mark_dirty();
            }
        });
    });

    // X-Ray
    let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 22.0), egui::Sense::click());
    let xray_tip = state.t("viewport.xray_tip");
    resp.widget_info(|| {
        WidgetInfo::selected(WidgetType::Button, true, state.show_xray, xray_tip.clone())
    });
    if ui.is_rect_visible(rect) {
        let is_active = state.show_xray;
        let bg = if is_active {
            tokens::ACCENT_BLUE
        } else if resp.hovered() {
            tokens::BG_SURFACE_HOVER
        } else {
            tokens::BG_SURFACE
        };
        let fg = if is_active {
            tokens::TEXT_ACTIVE
        } else {
            tokens::TEXT_SECONDARY
        };
        ui.painter().rect_filled(rect, tokens::RADIUS_CONTROL, bg);
        let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
        IconRegistry::paint(ui.ctx(), ui.painter(), &PetuniaIcon::XRay, icon_rect, fg);
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_CONTROL,
                tokens::stroke_focus(),
                StrokeKind::Inside,
            );
        }
    }
    if resp.on_hover_text(xray_tip).clicked() {
        state.show_xray = !state.show_xray;
        state.mark_dirty();
    }

    // Triangulação
    let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 22.0), egui::Sense::click());
    let tri_tip = state.t("viewport.tri_tip");
    resp.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Button,
            true,
            state.show_triangulation,
            tri_tip.clone(),
        )
    });
    if ui.is_rect_visible(rect) {
        let is_active = state.show_triangulation;
        let bg = if is_active {
            tokens::ACCENT_BLUE
        } else if resp.hovered() {
            tokens::BG_SURFACE_HOVER
        } else {
            tokens::BG_SURFACE
        };
        let fg = if is_active {
            tokens::TEXT_ACTIVE
        } else {
            tokens::TEXT_SECONDARY
        };
        ui.painter().rect_filled(rect, tokens::RADIUS_CONTROL, bg);
        let r = rect.shrink(5.0);
        ui.painter().line_segment(
            [pos2(r.left(), r.bottom()), pos2(r.right(), r.top())],
            egui::Stroke::new(1.5_f32, fg),
        );
        ui.painter().rect_stroke(
            r,
            1.0,
            egui::Stroke::new(1.0_f32, fg.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_CONTROL,
                tokens::stroke_focus(),
                StrokeKind::Inside,
            );
        }
    }
    if resp.on_hover_text(tri_tip).clicked() {
        state.show_triangulation = !state.show_triangulation;
        state.mark_dirty();
    }
}

/// Cluster 7: Os 4 modos canônicos de sombreamento no estilo esférico do Blender com ícones vetoriais.
fn draw_shading_spheres_cluster(ui: &mut Ui, state: &mut AppState) {
    let wire_tip = state.t("shading.tip_wireframe");
    let solid_tip = state.t("shading.tip_solid");
    let mat_tip = state.t("shading.tip_material");
    let rend_tip = state.t("shading.tip_rendered");
    let modes = [
        (Shading::Wireframe, PetuniaIcon::ShadingWireframe, wire_tip),
        (Shading::Solid, PetuniaIcon::ShadingSolid, solid_tip),
        (Shading::MaterialPreview, PetuniaIcon::ShadingMaterial, mat_tip),
        (Shading::Rendered, PetuniaIcon::ShadingRendered, rend_tip),
    ];

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 0.0);
        for (shading, icon, hint) in modes {
            let is_active = state.shading == shading;
            let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::click());
            resp.widget_info(|| {
                WidgetInfo::selected(WidgetType::Button, true, is_active, hint.clone())
            });
            if ui.is_rect_visible(rect) {
                let bg = if is_active {
                    tokens::ACCENT_BLUE
                } else if resp.hovered() {
                    tokens::BG_SURFACE_HOVER
                } else {
                    tokens::BG_SURFACE
                };
                let fg = if is_active {
                    tokens::TEXT_ACTIVE
                } else {
                    tokens::TEXT_SECONDARY
                };
                ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
                let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
                IconRegistry::paint(ui.ctx(), ui.painter(), &icon, icon_rect, fg);
                if resp.has_focus() {
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(11),
                        tokens::stroke_focus(),
                        StrokeKind::Inside,
                    );
                }
            }
            if resp.on_hover_text(hint).clicked() {
                state.shading = shading;
                state.mark_dirty();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::SelectMode;

    /// Desenha a barra num retângulo de tamanho controlado e devolve o plano.
    fn draw_in_rect(state: &mut AppState, size: egui::Vec2) -> (PetuniaToolbarPlan, egui::Rect) {
        let ctx = egui::Context::default();
        let mut plan = None;
        let mut used = egui::Rect::NOTHING;
        // Duas passagens: a primeira é a passagem de medida do `egui_taffy`.
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let mut bar = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(egui::Pos2::ZERO, size))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                plan = Some(draw(&mut bar, state));
                used = bar.min_rect();
            })
            .textures_delta
            .clear();
        }
        (plan.expect("barra sempre produz plano"), used)
    }

    #[test]
    fn toolbar_keeps_the_core_visible_no_matter_how_narrow() {
        let mut state = AppState::new("en");
        for width in [180.0, 320.0, 520.0, 900.0, 1_600.0] {
            let (plan, _) = draw_in_rect(&mut state, egui::vec2(width, 26.0));
            for core in [SLOT_DOMAIN, SLOT_MENUS, SLOT_DISPLAY] {
                assert!(
                    plan.is_visible(core),
                    "{core} é núcleo e sumiu numa barra de {width}px"
                );
            }
            // Toda faixa do modelo aparece visível ou na seta — nunca no limbo.
            let visible_clusters = plan
                .visible(1)
                .iter()
                .filter(|id| **id != SLOT_OVERFLOW)
                .count();
            assert_eq!(
                visible_clusters + plan.hidden(1).len(),
                5,
                "faixa perdida numa barra de {width}px"
            );
        }
    }

    #[test]
    fn only_pinned_clusters_may_exceed_the_available_width() {
        let mut state = AppState::new("en");
        let mut previous = 0.0_f32;
        for width in [420.0, 500.0, 640.0, 900.0, 1_600.0] {
            let (plan, used) = draw_in_rect(&mut state, egui::vec2(width, 26.0));
            assert!(
                used.width() >= previous - 0.5,
                "dar mais largura não pode encolher a barra ({}px)",
                width
            );
            previous = used.width();

            // O núcleo não pode ser ocultado: a única forma de a linha passar da
            // largura é o próprio núcleo ser largo. Nenhuma faixa **ocultável**
            // pode estar desenhada quando a linha estoura.
            if used.width() > width + 0.5 {
                for id in [SLOT_TRANSFORM, SLOT_SNAP_PROP] {
                    assert!(
                        !plan.visible(1).contains(&id),
                        "{id} desenhado enquanto a barra estoura em {width}px"
                    );
                }
            }
        }
    }

    #[test]
    fn narrow_bar_moves_transform_and_snap_to_overflow() {
        let mut state = AppState::new("en");
        let (wide, _) = draw_in_rect(&mut state, egui::vec2(1_800.0, 26.0));
        assert!(wide.visible(1).contains(&SLOT_TRANSFORM));
        assert!(wide.visible(1).contains(&SLOT_SNAP_PROP));
        assert!(!wide.has_hidden(), "com 1800px não cabe tudo?");

        let (narrow, _) = draw_in_rect(&mut state, egui::vec2(420.0, 26.0));
        assert!(narrow.has_hidden(), "420px não comporta a barra inteira");
        for hidden in narrow.hidden_all() {
            assert!(
                hidden != SLOT_DOMAIN && hidden != SLOT_MENUS && hidden != SLOT_DISPLAY,
                "só faixas de rank declarado podem cair"
            );
        }
    }

    #[test]
    fn long_locale_pushes_more_clusters_to_overflow_than_en() {
        // pt-BR tem rótulos mais longos que en: a barra precisa ceder para a seta
        // em vez de cortar.
        let mut en = AppState::new("en");
        let mut pt = AppState::new("pt-BR");
        let (en_plan, en_rect) = draw_in_rect(&mut en, egui::vec2(700.0, 26.0));
        let (pt_plan, pt_rect) = draw_in_rect(&mut pt, egui::vec2(700.0, 26.0));

        assert!(
            pt_rect.width() >= en_rect.width() - 0.5,
            "rótulos maiores encolheram a barra: en={:?} pt-BR={:?}",
            en_rect.width(),
            pt_rect.width()
        );
        assert!(
            pt_plan.hidden_all().len() >= en_plan.hidden_all().len(),
            "idioma longo precisa ceder para a seta, não caber por mágica"
        );
        for (locale, plan) in [("en", &en_plan), ("pt-BR", &pt_plan)] {
            for core in [SLOT_DOMAIN, SLOT_MENUS, SLOT_DISPLAY] {
                assert!(plan.is_visible(core), "[{locale}] {core} é núcleo");
            }
            if plan.has_hidden() {
                assert!(
                    plan.is_visible(SLOT_OVERFLOW),
                    "[{locale}] ocultos sem seta de acesso: {:?}",
                    plan.hidden_all()
                );
            }
        }
    }

    #[test]
    fn second_row_appears_only_when_the_measured_line_height_fits() {
        let mut state = AppState::new("en");

        let (one_row, used) = draw_in_rect(&mut state, egui::vec2(1_600.0, 26.0));
        assert_eq!(one_row.rows(), 1);
        assert!(
            used.height() <= 30.0,
            "uma linha de 22px não pode ocupar {}px",
            used.height()
        );

        let (two_rows, used) = draw_in_rect(&mut state, egui::vec2(1_600.0, 60.0));
        assert_eq!(two_rows.rows(), 2, "altura de sobra precisa virar 2 linhas");
        assert!(
            used.height() > 40.0,
            "duas linhas desenhadas não podem medir {}px",
            used.height()
        );
        assert!(
            used.height() <= 60.0,
            "duas linhas passaram da altura disponível: {}px",
            used.height()
        );

        // Campo apertado entre 1 e 2 linhas: fica em uma, sem espremer a segunda.
        let (tight, used) = draw_in_rect(&mut state, egui::vec2(1_600.0, 30.0));
        assert_eq!(tight.rows(), 1, "30px não são duas linhas de 22px");
        assert!(used.height() <= 34.0);
    }

    #[test]
    fn primitive_menu_covers_all_ten_species_once() {
        let groups = primitive_menu_groups();
        assert_eq!(groups.len(), 3);
        let mut seen: Vec<PrimitiveKind> = Vec::new();
        for (_, kinds) in &groups {
            seen.extend(kinds.iter().copied());
        }
        assert_eq!(seen.len(), 10);
        seen.sort_by_key(|k| *k as u8);
        seen.dedup();
        assert_eq!(seen.len(), 10);
    }

    #[test]
    fn test_viewport_bar_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn snap_and_prop_stay_reachable_through_the_overflow_menu() {
        // A seta precisa existir sempre que algo é ocultado — do contrário a
        // ferramenta fica inalcançável numa tela pequena.
        let mut state = AppState::new("en");
        let (plan, _) = draw_in_rect(&mut state, egui::vec2(400.0, 26.0));
        assert!(plan.has_hidden(), "400px não comporta a barra inteira");
        assert!(
            plan.overflow_visible(SLOT_OVERFLOW),
            "algo oculto sem seta de acesso: {:?}",
            plan.hidden_all()
        );
    }

    #[test]
    fn test_selection_modes_toggle_via_bar() {
        let mut state = AppState::new("en");
        assert_eq!(state.edit_mode(), EditMode::Object);

        state.set_edit_mode(EditMode::Edit);
        state.select_mode = SelectMode::Vertex;
        assert_eq!(state.select_mode, SelectMode::Vertex);

        state.select_mode = SelectMode::Face;
        assert_eq!(state.select_mode, SelectMode::Face);
    }

    #[test]
    fn test_mode_and_target_separation() {
        let mut state = AppState::new("en");
        // Em Object Mode, alvos de seleção de malha não devem ser alterados
        assert_eq!(state.edit_mode(), EditMode::Object);

        state.set_edit_mode(EditMode::Edit);
        assert_eq!(state.edit_mode(), EditMode::Edit);
        state.select_mode = SelectMode::Edge;
        assert_eq!(state.select_mode, SelectMode::Edge);
    }

    #[test]
    fn test_selection_domain_controls() {
        let mut state = AppState::new("en");
        assert_eq!(state.selection_domain(), SelectionDomain::Object);

        state.set_selection_domain(SelectionDomain::Vertex);
        assert_eq!(state.selection_domain(), SelectionDomain::Vertex);
        assert_eq!(state.edit_mode(), EditMode::Edit);
        assert_eq!(state.select_mode, SelectMode::Vertex);

        state.cycle_selection_domain();
        assert_eq!(state.selection_domain(), SelectionDomain::Object);

        state.cycle_selection_domain();
        assert_eq!(state.selection_domain(), SelectionDomain::Vertex);
    }

    #[test]
    fn test_snap_and_proportional_controls_render() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw_snap_and_prop_cluster(ui, &mut state);
            });
        })
        .textures_delta
        .clear();

        state.snap_enabled = true;
        state.proportional_editing = true;

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw_snap_and_prop_cluster(ui, &mut state);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_axis_lock_controls_render_and_badge() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        // 1. Renderizar com todos eixos livres
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw_axis_lock_controls(ui, &mut state);
            });
        })
        .textures_delta
        .clear();

        // 2. Travar eixo X e verificar renderização do badge
        state.toggle_axis_lock(0);
        assert!(state.is_axis_locked(0));
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Eixo X", [235, 75, 75]))
        );

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw_axis_lock_controls(ui, &mut state);
            });
        })
        .textures_delta
        .clear();

        // 3. Travar eixo Z formando plano XZ e verificar badge
        state.toggle_axis_lock(2);
        assert_eq!(
            state.active_axis_constraint_label(),
            Some(("Plano XZ", [142, 68, 173]))
        );

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw_axis_lock_controls(ui, &mut state);
            });
        })
        .textures_delta
        .clear();
    }
}
