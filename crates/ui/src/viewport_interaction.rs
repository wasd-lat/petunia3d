//! Overlays e arbitragem: modal > gizmo > navegação > seleção.
use crate::{
    gizmo::{GizmoHandle, GizmoKind, draw_gizmo},
    icon_registry::PetuniaIcon,
    modal_viewport, tokens, widgets,
};
use egui::{Color32, PointerButton, Pos2, Rect};
use glam::{Vec2, Vec3};
use petunia_core::picking::{PickComponent, pick_mesh};
use petunia_core::{
    AppState, BrushPreviewKind, BrushPreviewStyle, EditMode, ModalConstraint, ModalKind,
    SelectMode, Workspace,
};
use uuid::Uuid;

pub fn draw(
    ctx: &egui::Context,
    state: &mut AppState,
    rect: Rect,
    painter: &egui::Painter,
    response: &egui::Response,
) -> bool {
    if let Some((start, goal, elapsed)) = state.camera_frame.take() {
        let elapsed = elapsed + ctx.input(|i| i.stable_dt).clamp(0.001, 0.033);
        let t = (elapsed / 0.25).min(1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        state.camera.target = start.target.lerp(goal.target, eased);
        state.camera.distance = start.distance + (goal.distance - start.distance) * eased;
        state.camera.ortho_half_h =
            start.ortho_half_h + (goal.ortho_half_h - start.ortho_half_h) * eased;
        if t < 1.0 {
            state.camera_frame = Some((start, goal, elapsed));
        }
        state.mark_dirty();
    }
    if modal_viewport::draw(ctx, state, rect, painter) {
        return true;
    }
    if crate::cutting::draw(ctx, state, rect, painter) {
        return true;
    }
    if crate::measurement::draw(ctx, state, rect, painter, response) {
        return true;
    }
    if crate::annotation::draw(ctx, state, rect, painter, response) {
        return true;
    }
    let pointer = ctx.pointer_hover_pos().filter(|p| rect.contains(*p));
    // Navegação usa deltas por frame e a base da câmera, inclusive nas vistas Top/Bottom.
    if response.dragged_by(PointerButton::Middle) {
        state.camera_frame = None;
        let delta = ctx.input(|i| i.pointer.delta());
        if ctx.input(|i| i.modifiers.shift) {
            state.camera.pan(delta.x, delta.y);
        } else {
            state.camera.orbit(delta.x, delta.y);
        }
        if delta != egui::Vec2::ZERO {
            state.mark_dirty();
        }
        return true;
    }
    if pointer.is_some() {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            state.camera_frame = None;
            state.camera.zoom(scroll);
            state.mark_dirty();
        }
    }

    // Drag and Drop de Assets da Biblioteca para o Viewport 3D (P3D-044)
    if egui::DragAndDrop::has_payload_of_type::<Uuid>(ctx)
        && let Some(hover_pos) = pointer
    {
        painter.rect_stroke(
            rect.shrink(3.0),
            tokens::RADIUS_CONTAINER,
            egui::Stroke::new(2.0_f32, tokens::ACCENT_BLUE),
            egui::StrokeKind::Inside,
        );

        let ground_pt = modal_viewport::plane_point(state, rect, hover_pos, Vec3::ZERO, Vec3::Y);
        if let Some(pt) = ground_pt
            && let Some(sc) = modal_viewport::screen_point(&state.camera, rect, pt)
        {
            painter.circle_stroke(sc, 14.0, egui::Stroke::new(2.0_f32, tokens::ACCENT_BLUE));
            painter.circle_filled(sc, 3.0, tokens::ACCENT_BLUE);
            painter.text(
                sc + egui::vec2(0.0, 18.0),
                egui::Align2::CENTER_TOP,
                state.t("viewport.drop_to_instantiate"),
                egui::FontId::proportional(11.0),
                tokens::TEXT_PRIMARY,
            );
        }

        if ctx.input(|i| i.pointer.any_released())
            && let Some(payload) = egui::DragAndDrop::payload::<Uuid>(ctx)
        {
            let asset_id = *payload;
            let target_pos = ground_pt.map(|p| [p.x, p.y, p.z]);
            let _ = state.dispatch(&petunia_core::InstantiateAssetCmd {
                asset_id,
                position: target_pos,
            });
            egui::DragAndDrop::clear_payload(ctx);
            return true;
        }
    }

    // Overlays 3D e controles de navegação do viewport (Blender.svg Golden Reference)
    crate::nav_gizmo::draw_nav_hud(state, rect, painter);
    crate::nav_gizmo::draw_3d_cursor(state, rect, painter);
    if crate::nav_gizmo::handle_3d_cursor_placement(ctx, state, rect) {
        return true;
    }
    if crate::nav_gizmo::draw_nav_gizmo(ctx, state, rect, painter) {
        return true;
    }
    crate::nav_gizmo::draw_context_menu(ctx, state);
    if pointer.is_some()
        && ctx.input(|i| !i.modifiers.shift && i.pointer.button_clicked(PointerButton::Secondary))
        && let Some(pos) = pointer
    {
        state.ui.context_menu_pos = Some([pos.x, pos.y]);
        return true;
    }

    // Pintura é decidida pelo workspace/pela tool — nunca por um "modo" paralelo.
    let paint = state.workspace == Workspace::Paint || state.active_tool == "paint";
    if paint {
        // O ponteiro sobre o chrome da viewport (cartão flutuante de ferramenta,
        // barra contextual, cartão de primitiva) pertence ao controle: a
        // pincelada não começa ali, senão clicar num slider de cor também
        // pintaria a malha. O traço **em andamento** continua válido — só a
        // abertura de um traço novo é bloqueada.
        if !ctx.input(|i| i.pointer.button_down(PointerButton::Primary))
            && pointer.is_some_and(|p| pointer_on_viewport_chrome(ctx, p))
        {
            return false;
        }
        return paint_preview(ctx, state, rect, painter, response);
    }
    if state.workspace != Workspace::Model || state.active_tool == "draw_profile" {
        return false;
    }
    let is_object_domain = state.selection_domain() == petunia_core::SelectionDomain::Object;

    if is_object_domain {
        let scene = petunia_core::viewport_query::ViewportSceneQuery::new(&state.project.project);
        let hovered_idx = pointer.and_then(|pos| {
            let n = ndc(rect, pos);
            scene.nearest_object(&state.camera, [n.x, n.y])
        });
        state.session.tools.hover = hovered_idx.map_or(
            petunia_core::HoverTarget::None,
            petunia_core::HoverTarget::Object,
        );

        // Desenha contornos dos objetos selecionados e do objeto sob hover no domínio Object
        let project = |p| screen(state, rect, p);
        for (idx, asset) in state.project.assets.iter().enumerate() {
            if !asset.visible {
                continue;
            }
            let is_active = idx == state.project.active;
            let is_selected = state.session.selection.assets.contains(&asset.id) || is_active;
            let is_hovered = hovered_idx == Some(idx);
            if !is_selected && !is_hovered {
                continue;
            }
            let stroke_color = if is_active {
                Color32::from_rgb(255, 140, 20)
            } else if is_selected {
                Color32::from_rgb(255, 180, 50)
            } else {
                Color32::from_rgb(125, 220, 255)
            };
            let stroke_width = if is_active || is_hovered {
                2.0_f32
            } else {
                1.5_f32
            };
            let stroke = egui::Stroke::new(stroke_width, stroke_color);

            let mesh = asset.evaluated_mesh();
            for (a, b) in mesh.edges_unique() {
                if let (Some(va), Some(vb)) =
                    (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                {
                    let spa = project(va.vec());
                    let spb = project(vb.vec());
                    if rect.contains(spa) || rect.contains(spb) {
                        painter.line_segment([spa, spb], stroke);
                    }
                }
            }
        }
    } else {
        let mode = state.select_mode;
        // No domínio Point, demarca visualmente todos os pontos disponíveis para seleção.
        if state.edit_mode() == EditMode::Edit
            && state.select_mode == SelectMode::Vertex
            && let Some(mesh) = state.project.active_mesh()
        {
            for v in &mesh.verts {
                let sp = screen(state, rect, v.vec());
                if rect.contains(sp) {
                    if v.selected {
                        painter.circle_filled(sp, 3.5, Color32::from_rgb(255, 140, 20));
                        painter.circle_stroke(sp, 3.5, egui::Stroke::new(1.0_f32, Color32::WHITE));
                    } else {
                        painter.circle_filled(sp, 2.5, Color32::from_rgb(25, 25, 30));
                        painter.circle_stroke(
                            sp,
                            2.5,
                            egui::Stroke::new(1.0_f32, Color32::from_rgb(200, 200, 210)),
                        );
                    }
                }
            }
        }

        let hit = pointer.and_then(|pos| {
            state.project.active_mesh().and_then(|mesh| {
                pick_mesh(
                    mesh,
                    &state.camera,
                    Vec2::new(rect.width(), rect.height()) * ctx.pixels_per_point(),
                    ndc(rect, pos),
                    mode,
                    state.shading == petunia_render::Shading::Wireframe || state.show_xray,
                )
            })
        });
        if let (Some(hit), Some(mesh)) = (hit, state.project.active_mesh()) {
            let color = egui::Color32::from_rgb(125, 220, 255);
            let project = |p| screen(state, rect, p);
            match hit.component {
                PickComponent::Vertex(i) => {
                    if let Some(v) = mesh.verts.get(i) {
                        let sp = project(v.vec());
                        // Anel externo de foco e halo
                        painter.circle_stroke(
                            sp,
                            8.5,
                            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(100, 220, 255)),
                        );
                        // Ponto interno dourado
                        painter.circle_filled(sp, 5.0, egui::Color32::from_rgb(255, 215, 0));
                        painter.circle_stroke(
                            sp,
                            5.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
                        );
                    }
                }
                PickComponent::Edge(a, b) => {
                    if let (Some(a), Some(b)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    {
                        painter.line_segment(
                            [project(a.vec()), project(b.vec())],
                            egui::Stroke::new(3.0_f32, color),
                        );
                    }
                }
                PickComponent::Face(i) => {
                    if let Some(face) = mesh.faces.get(i) {
                        for k in 0..face.verts.len() {
                            if let (Some(a), Some(b)) = (
                                mesh.verts.get(face.verts[k] as usize),
                                mesh.verts
                                    .get(face.verts[(k + 1) % face.verts.len()] as usize),
                            ) {
                                painter.line_segment(
                                    [project(a.vec()), project(b.vec())],
                                    egui::Stroke::new(2.5_f32, color),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    if handle_annotation_gizmo(ctx, state, rect, painter, pointer) {
        return true;
    }
    let transform_tool_active = matches!(
        state.active_tool.as_str(),
        "transform" | "move" | "rotate" | "scale"
    );
    if transform_tool_active
        && !state.is_active_locked()
        && let Some(mesh) = state.project.active_mesh().filter(|m| {
            m.has_selection() || state.selection_domain() == petunia_core::SelectionDomain::Object
        })
    {
        let pivot = state.calculate_pivot(state.session.pivot_point);
        let axes = if state.transform_orientation == petunia_core::TransformOrientation::Local {
            crate::gizmo::local_axes_for_mesh(mesh)
        } else {
            [Vec3::X, Vec3::Y, Vec3::Z]
        };
        let interaction = if state.active_tool == "transform" {
            crate::gizmo::draw_universal_gizmo_oriented(
                painter,
                &state.camera,
                rect,
                pivot,
                pointer,
                axes,
            )
        } else {
            let kind = match state.active_tool.as_str() {
                "rotate" => GizmoKind::Rotate,
                "scale" => GizmoKind::Scale,
                _ => GizmoKind::Translate,
            };
            crate::gizmo::draw_gizmo_oriented(
                painter,
                &state.camera,
                rect,
                pivot,
                kind,
                pointer,
                axes,
            )
            .map(|handle| crate::gizmo::GizmoInteraction { kind, handle })
        };
        if let Some(interaction) = interaction {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
            if ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary))
                && let Some(pos) = pointer
            {
                let constraint = match interaction.handle {
                    GizmoHandle::Center => ModalConstraint::Free,
                    GizmoHandle::Axis(a) => ModalConstraint::Axis(a as usize),
                    GizmoHandle::Plane(a) => ModalConstraint::Plane(a as usize),
                };
                let kind = match interaction.kind {
                    GizmoKind::Translate => ModalKind::Move,
                    GizmoKind::Rotate => ModalKind::Rotate,
                    GizmoKind::Scale => ModalKind::Scale,
                };
                modal_viewport::start_handle(ctx, state, kind, constraint, pos);
                state.ui.box_select_start = None;
                return true;
            }
        }
    }
    let object_locked_label = state.t("context.object_locked");
    let modeling_label = state.t("context.modeling");
    let move_label = state.t("tools.move");
    let rotate_label = state.t("tools.rotate");
    let scale_label = state.t("tools.scale");
    let extrude_label = state.t("tools.extrude");
    let inset_label = state.t("tools.inset");
    let pushpull_label = state.t("tools.pushpull");
    let bevel_label = state.t("tools.bevel");
    response.context_menu(|ui| {
        if state.is_active_locked() {
            ui.label(
                egui::RichText::new(object_locked_label.as_str())
                    .italics()
                    .color(tokens::TEXT_MUTED),
            );
            return;
        }
        ui.label(
            egui::RichText::new(modeling_label.as_str())
                .strong()
                .color(tokens::TEXT_PRIMARY),
        );
        ui.separator();
        if widgets::PetuniaMenuItem::new(move_label.as_str())
            .icon(PetuniaIcon::Move)
            .shortcut(Some("G"))
            .show(ui)
            .clicked()
        {
            state.pending_modal = Some(ModalKind::Move);
            ui.close();
        }
        if widgets::PetuniaMenuItem::new(rotate_label.as_str())
            .icon(PetuniaIcon::Rotate)
            .shortcut(Some("R"))
            .show(ui)
            .clicked()
        {
            state.pending_modal = Some(ModalKind::Rotate);
            ui.close();
        }
        if widgets::PetuniaMenuItem::new(scale_label.as_str())
            .icon(PetuniaIcon::Scale)
            .shortcut(Some("S"))
            .show(ui)
            .clicked()
        {
            state.pending_modal = Some(ModalKind::Scale);
            ui.close();
        }
        let faces = state
            .project
            .active_mesh()
            .is_some_and(|m| m.selected_face_count() > 0);
        if faces {
            ui.separator();
            if widgets::PetuniaMenuItem::new(extrude_label.as_str())
                .icon(PetuniaIcon::Extrude)
                .shortcut(Some("E"))
                .show(ui)
                .clicked()
            {
                state.pending_modal = Some(ModalKind::Extrude);
                ui.close();
            }
            if widgets::PetuniaMenuItem::new(inset_label.as_str())
                .icon(PetuniaIcon::Inset)
                .shortcut(Some("I"))
                .show(ui)
                .clicked()
            {
                state.pending_modal = Some(ModalKind::Inset);
                ui.close();
            }
            if widgets::PetuniaMenuItem::new(pushpull_label.as_str())
                .icon(PetuniaIcon::PushPull)
                .shortcut(Some("P"))
                .show(ui)
                .clicked()
            {
                state.pending_modal = Some(ModalKind::PushPull);
                ui.close();
            }
        }
        if state
            .project
            .active_mesh()
            .is_some_and(|m| !m.selected_edges.is_empty())
            && widgets::PetuniaMenuItem::new(bevel_label.as_str())
                .icon(PetuniaIcon::Bevel)
                .shortcut(Some("Ctrl+B"))
                .show(ui)
                .clicked()
        {
            state.pending_modal = Some(ModalKind::Bevel);
            ui.close();
        }
    });
    false
}

fn paint_preview(
    ctx: &egui::Context,
    state: &mut AppState,
    rect: Rect,
    painter: &egui::Painter,
    response: &egui::Response,
) -> bool {
    let radius_id = egui::Id::new("paint.size_drag");
    let cancelled_id = egui::Id::new("paint.cancelled_until_release");
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.finish_paint_stroke(true);
        ctx.data_mut(|d| d.remove::<([f32; 2], [f32; 2])>(egui::Id::new("paint.shape3d")));
        if let Some((_, radius)) = ctx.data_mut(|d| d.get_temp::<(Pos2, f32)>(radius_id)) {
            state.canvas_brush = radius.round().max(1.0) as u32;
            ctx.data_mut(|d| d.remove::<(Pos2, f32)>(radius_id));
        }
        if ctx.input(|i| i.pointer.button_down(PointerButton::Primary)) {
            ctx.data_mut(|d| d.insert_temp(cancelled_id, true));
        } else {
            ctx.data_mut(|d| d.remove::<bool>(cancelled_id));
        }
        return true;
    }
    if ctx.input(|i| i.pointer.button_released(PointerButton::Primary)) {
        state.finish_paint_stroke(false);
        ctx.data_mut(|d| {
            d.remove::<bool>(cancelled_id);
            d.remove::<(f32, f32)>(egui::Id::new("paint.3d_previous_uv"));
        });
    }
    if ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)) {
        ctx.data_mut(|d| d.remove::<bool>(cancelled_id));
    }
    if ctx
        .data_mut(|d| d.get_temp::<bool>(cancelled_id))
        .unwrap_or(false)
    {
        return true;
    }
    let Some(pos) = ctx.pointer_hover_pos().filter(|p| rect.contains(*p)) else {
        return true;
    };
    if ctx.input(|i| i.key_pressed(egui::Key::F)) {
        ctx.data_mut(|d| d.insert_temp(radius_id, (pos, state.brush_settings().size_px)));
    }
    if let Some((anchor, radius)) = ctx.data_mut(|d| d.get_temp::<(Pos2, f32)>(radius_id)) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.canvas_brush = radius.round().max(1.0) as u32;
            ctx.data_mut(|d| d.remove::<(Pos2, f32)>(radius_id));
        } else {
            state.canvas_brush = (radius + (pos.x - anchor.x) * 0.5).clamp(1.0, 512.0) as u32;
            if ctx.input(|i| {
                i.pointer.button_pressed(PointerButton::Primary) || i.key_pressed(egui::Key::Enter)
            }) {
                ctx.data_mut(|d| d.remove::<(Pos2, f32)>(radius_id));
            }
        }
        painter.text(
            pos + egui::vec2(18.0, 18.0),
            egui::Align2::LEFT_TOP,
            format!("{}: {} px", state.t("paint.size"), state.canvas_brush),
            egui::FontId::monospace(14.0),
            tokens::TEXT_PRIMARY,
        );
        return true;
    }
    let point = ndc(rect, pos);
    let hit = state.project.active_mesh().and_then(|mesh| {
        pick_mesh(
            mesh,
            &state.camera,
            Vec2::new(rect.width(), rect.height()) * ctx.pixels_per_point(),
            point,
            SelectMode::Face,
            false,
        )
    });
    if let Some(hit) = hit
        && let PickComponent::Face(face) = hit.component
    {
        let settings = state.brush_settings();
        let style = BrushPreviewStyle::from_settings(settings, state.paint_color);
        let center = screen(state, rect, hit.position);
        let radius = state.brush_world_radius(hit.position);
        let radius_px = (screen(state, rect, hit.position + Vec3::X * radius).x - center.x).abs();
        match style.kind {
            BrushPreviewKind::Ring => {
                let tint = egui::Color32::from_rgba_unmultiplied(
                    (style.tint[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (style.tint[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (style.tint[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (style.fill_alpha * 80.0) as u8,
                );
                painter.circle_filled(center, radius_px, tint);
                painter.circle_stroke(center, radius_px, egui::Stroke::new(1.5, tint));
            }
            BrushPreviewKind::HollowRing => {
                painter.circle_stroke(
                    center,
                    radius_px,
                    egui::Stroke::new(1.5, tokens::TEXT_PRIMARY),
                );
            }
            BrushPreviewKind::Crosshair => {
                let arm = radius_px.max(8.0);
                painter.line_segment(
                    [center - egui::vec2(arm, 0.0), center + egui::vec2(arm, 0.0)],
                    egui::Stroke::new(1.5, tokens::TEXT_PRIMARY),
                );
                painter.line_segment(
                    [center - egui::vec2(0.0, arm), center + egui::vec2(0.0, arm)],
                    egui::Stroke::new(1.5, tokens::TEXT_PRIMARY),
                );
            }
            BrushPreviewKind::None => {}
        }
        let sample = ctx.input(|i| i.modifiers.alt || i.key_pressed(egui::Key::G));
        if sample {
            if ctx.input(|i| {
                i.pointer.button_pressed(PointerButton::Primary) || i.key_pressed(egui::Key::G)
            }) && let Some(mesh) = state.project.active_mesh()
                && let Some(vertex) = mesh
                    .faces
                    .get(face)
                    .into_iter()
                    .flat_map(|f| &f.verts)
                    .filter_map(|&v| mesh.verts.get(v as usize))
                    .min_by(|a, b| {
                        (a.vec() - hit.position)
                            .length_squared()
                            .total_cmp(&(b.vec() - hit.position).length_squared())
                    })
            {
                state.paint_color = vertex.color;
                state.mark_dirty();
            }
        } else if (response.dragged_by(PointerButton::Primary)
            && ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO))
            || ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary))
        {
            let brush_type = petunia_core::brush_type_from_kind(state.paint_brush_kind);
            // Formas 3D: press ancora o UV, release confirma o segmento.
            // Transação própria (checkpoint no press; sem sessão de stroke
            // para o finish do topo não carimbar em duplicidade).
            let shape_brush = matches!(
                brush_type,
                petunia_module_paint::BrushType::Line | petunia_module_paint::BrushType::Rectangle
            );
            if shape_brush {
                if ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary)) {
                    state.checkpoint("canvas shape");
                    if let Some(uv) = petunia_module_paint::PaintModule::face_hit_uv(
                        state,
                        face,
                        hit.position,
                        state.paint_isolate_selection,
                    ) {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("paint.shape3d"), (uv, uv)));
                    }
                }
                if ctx.input(|i| i.pointer.button_released(PointerButton::Primary))
                    && let Some((start, _)) = ctx.data(|d| {
                        d.get_temp::<([f32; 2], [f32; 2])>(egui::Id::new("paint.shape3d"))
                    })
                    && let Some(end) = petunia_module_paint::PaintModule::face_hit_uv(
                        state,
                        face,
                        hit.position,
                        state.paint_isolate_selection,
                    )
                {
                    ctx.data_mut(|d| {
                        d.remove::<([f32; 2], [f32; 2])>(egui::Id::new("paint.shape3d"))
                    });
                    let to_px =
                        |uv: [f32; 2]| petunia_module_paint::PaintModule::uv_to_px(state, uv);
                    if let (Some((x0, y0)), Some((x1, y1))) = (to_px(start), to_px(end)) {
                        petunia_module_paint::PaintModule::commit_shape(
                            state,
                            petunia_module_paint::ShapeStroke {
                                x0,
                                y0,
                                x1,
                                y1,
                                brush: brush_type,
                                color: [
                                    (state.paint_color[0] * 255.0) as u8,
                                    (state.paint_color[1] * 255.0) as u8,
                                    (state.paint_color[2] * 255.0) as u8,
                                    255,
                                ],
                                strength: state.paint_strength,
                            },
                        );
                    }
                }
                // Atualiza a âncora durante o arrasto (preview = anel do pincel).
                if ctx.input(|i| i.pointer.button_down(PointerButton::Primary))
                    && let Some(uv) = petunia_module_paint::PaintModule::face_hit_uv(
                        state,
                        face,
                        hit.position,
                        state.paint_isolate_selection,
                    )
                {
                    ctx.data_mut(|d| {
                        if let Some((start, _)) =
                            d.get_temp::<([f32; 2], [f32; 2])>(egui::Id::new("paint.shape3d"))
                        {
                            d.insert_temp(egui::Id::new("paint.shape3d"), (start, uv));
                        }
                    });
                }
            } else {
                state.begin_paint_stroke();
                state.paint_at(hit.position);
                let settings = state.brush_settings();
                let previous_id = egui::Id::new("paint.3d_previous_uv");
                if let Some(uv) = petunia_module_paint::PaintModule::face_hit_uv(
                    state,
                    face,
                    hit.position,
                    state.paint_isolate_selection,
                ) {
                    if let Some((x, y)) = petunia_module_paint::PaintModule::uv_to_px(state, uv) {
                        let previous = ctx
                            .data(|d| d.get_temp::<(f32, f32)>(previous_id))
                            .unwrap_or((x as f32, y as f32));
                        for (dab_x, dab_y) in
                            settings.stroke_dabs(previous.0, previous.1, x as f32, y as f32)
                        {
                            petunia_module_paint::PaintModule::canvas_brush_with_settings(
                                state, dab_x, dab_y, settings,
                            );
                        }
                        ctx.data_mut(|d| {
                            d.insert_temp(previous_id, (x as f32, y as f32));
                        });
                    }
                } else {
                    let _ = petunia_module_paint::PaintModule::paint_mesh_3d_with_settings(
                        state,
                        face,
                        hit.position,
                        settings,
                        state.paint_isolate_selection,
                    );
                }
            }
        }
    }
    true
}
/// O ponteiro está sobre um chrome de viewport **medido**?
///
/// A fonte é a mesma dos testes de QA: [`crate::regions`], alimentada pelo
/// retângulo real que cada superfície devolveu — não uma estimativa por largura.
/// Numa eventual defasagem de um frame (a superfície nasce depois), o pior caso
/// é uma pincelada começar sob o cartão por um frame; preferimos isso a pintar
/// por cima do controle do usuário.
fn pointer_on_viewport_chrome(ctx: &egui::Context, pos: Pos2) -> bool {
    crate::regions::load(ctx).is_some_and(|regions| {
        [
            regions.tool_properties,
            regions.shelf,
            regions.primitive_card,
        ]
        .into_iter()
        .flatten()
        .any(|chrome| chrome.contains(pos))
    })
}

fn ndc(rect: Rect, p: Pos2) -> Vec2 {
    Vec2::new(
        (p.x - rect.left()) / rect.width() * 2.0 - 1.0,
        1.0 - (p.y - rect.top()) / rect.height() * 2.0,
    )
}
fn screen(state: &AppState, rect: Rect, p: Vec3) -> Pos2 {
    let q = state.camera.project_ndc(p);
    Pos2::new(
        rect.center().x + q.x * rect.width() * 0.5,
        rect.center().y - q.y * rect.height() * 0.5,
    )
}

#[derive(Clone, Copy)]
struct AnnGizmoDrag {
    ann_id: Uuid,
    start_pointer: Pos2,
    start_trans: [f32; 3],
    start_rot: [f32; 3],
    start_scale: [f32; 3],
    handle: GizmoHandle,
    kind: GizmoKind,
}

fn handle_annotation_gizmo(
    ctx: &egui::Context,
    state: &mut AppState,
    rect: Rect,
    painter: &egui::Painter,
    pointer: Option<Pos2>,
) -> bool {
    let drag_id = egui::Id::new("ann.gizmo_drag");
    let active_drag = ctx.data_mut(|d| d.get_temp::<AnnGizmoDrag>(drag_id));

    if let Some(drag) = active_drag {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Some(ann) = state
                .project
                .annotations
                .iter_mut()
                .find(|a| a.id == drag.ann_id)
            {
                ann.translation = drag.start_trans;
                ann.rotation = drag.start_rot;
                ann.scale = drag.start_scale;
                state.mark_dirty();
            }
            ctx.data_mut(|d| d.remove::<AnnGizmoDrag>(drag_id));
            return true;
        }

        if ctx.input(|i| i.pointer.button_released(PointerButton::Primary)) {
            ctx.data_mut(|d| d.remove::<AnnGizmoDrag>(drag_id));
            state.checkpoint("transform annotation");
            state.mark_dirty();
            return true;
        }

        if ctx.input(|i| i.pointer.button_down(PointerButton::Primary)) {
            let cur_pos = ctx
                .pointer_hover_pos()
                .or_else(|| ctx.input(|i| i.pointer.interact_pos()))
                .unwrap_or(drag.start_pointer);

            let delta = cur_pos - drag.start_pointer;
            let cam_eye = state.session.camera.eye();
            let cam_forward = state.session.camera.forward();
            let cam_proj = state.session.camera.proj;
            let cam_ortho_half_h = state.session.camera.ortho_half_h;
            let cam_fov_y = state.session.camera.fov_y;
            let cam_right = state.session.camera.right();
            let cam_up = state.session.camera.up();

            if let Some(ann) = state
                .project
                .annotations
                .iter_mut()
                .find(|a| a.id == drag.ann_id)
            {
                let pivot = Vec3::from_array(ann.center());
                let depth = (pivot - cam_eye).dot(cam_forward).max(0.1);
                let world_per_pt = match cam_proj {
                    petunia_core::Projection::Ortho => 2.0 * cam_ortho_half_h / rect.height(),
                    petunia_core::Projection::Perspective => {
                        2.0 * depth * (cam_fov_y * 0.5).tan() / rect.height()
                    }
                };

                match drag.kind {
                    GizmoKind::Translate => {
                        let world_vec = (cam_right * delta.x - cam_up * delta.y) * world_per_pt;
                        let delta_vec = match drag.handle {
                            GizmoHandle::Center => world_vec,
                            GizmoHandle::Axis(a) => {
                                let ax = [Vec3::X, Vec3::Y, Vec3::Z][a as usize];
                                ax * world_vec.dot(ax)
                            }
                            GizmoHandle::Plane(a) => {
                                let ax = [Vec3::X, Vec3::Y, Vec3::Z][a as usize];
                                world_vec - ax * world_vec.dot(ax)
                            }
                        };
                        ann.translation = [
                            drag.start_trans[0] + delta_vec.x,
                            drag.start_trans[1] + delta_vec.y,
                            drag.start_trans[2] + delta_vec.z,
                        ];
                    }
                    GizmoKind::Rotate => {
                        let angle = delta.x * 0.5;
                        match drag.handle {
                            GizmoHandle::Axis(a) => {
                                let mut r = drag.start_rot;
                                r[a as usize] += angle;
                                ann.rotation = r;
                            }
                            _ => {
                                let mut r = drag.start_rot;
                                r[2] += angle;
                                ann.rotation = r;
                            }
                        }
                    }
                    GizmoKind::Scale => {
                        let factor = (1.0 + delta.x * 0.01).max(0.01);
                        match drag.handle {
                            GizmoHandle::Axis(a) => {
                                let mut s = drag.start_scale;
                                s[a as usize] *= factor;
                                ann.scale = s;
                            }
                            _ => {
                                ann.scale = [
                                    drag.start_scale[0] * factor,
                                    drag.start_scale[1] * factor,
                                    drag.start_scale[2] * factor,
                                ];
                            }
                        }
                    }
                }
                state.mark_dirty();
                let constraint = match drag.handle {
                    GizmoHandle::Center => ModalConstraint::Free,
                    GizmoHandle::Axis(a) => ModalConstraint::Axis(a as usize),
                    GizmoHandle::Plane(a) => ModalConstraint::Plane(a as usize),
                };
                crate::modal_viewport::draw_axis_guide_lines(
                    painter, state, rect, pivot, constraint,
                );
                draw_gizmo(
                    painter,
                    &state.camera,
                    rect,
                    pivot,
                    drag.kind,
                    Some(cur_pos),
                );
                return true;
            }
        }
    }

    if state.project.annotations_locked {
        return false;
    }

    if let Some(ann_id) = state.selected_annotation
        && let Some(ann) = state
            .project
            .annotations
            .iter()
            .find(|a| a.id == ann_id && !a.locked)
    {
        let pivot = Vec3::from_array(ann.center());
        let kind = match state.gizmo_mode {
            ModalKind::Rotate => GizmoKind::Rotate,
            ModalKind::Scale => GizmoKind::Scale,
            _ => GizmoKind::Translate,
        };
        if let Some(handle) = draw_gizmo(painter, &state.camera, rect, pivot, kind, pointer) {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
            if ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary))
                && let Some(pos) = pointer
            {
                ctx.data_mut(|d| {
                    d.insert_temp(
                        drag_id,
                        AnnGizmoDrag {
                            ann_id: ann.id,
                            start_pointer: pos,
                            start_trans: ann.translation,
                            start_rot: ann.rotation,
                            start_scale: ann.scale,
                            handle,
                            kind,
                        },
                    )
                });
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::{AnnotationItem, AnnotationStroke};

    #[test]
    fn test_annotation_gizmo_rendering_and_interaction() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        let stroke = AnnotationStroke {
            points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            color: [0.0, 0.74, 0.83, 1.0],
            width: 2.0,
        };
        let ann = AnnotationItem::new("Ann1", vec![stroke]);
        let ann_id = ann.id;
        state.project.add_annotation(ann);
        state.selected_annotation = Some(ann_id);

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
                let painter = ui.painter_at(rect);
                let response = ui.allocate_rect(rect, egui::Sense::drag());
                let handled = draw(&ctx, &mut state, rect, &painter, &response);
                assert!(!handled);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_annotation_gizmo_drag_session_escape_cancels() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        let stroke = AnnotationStroke {
            points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            color: [0.0, 0.74, 0.83, 1.0],
            width: 2.0,
        };
        let mut ann = AnnotationItem::new("Ann1", vec![stroke]);
        ann.translation = [5.0, 0.0, 0.0];
        let ann_id = ann.id;
        state.project.add_annotation(ann);
        state.selected_annotation = Some(ann_id);

        let drag_id = egui::Id::new("ann.gizmo_drag");
        ctx.data_mut(|d| {
            d.insert_temp(
                drag_id,
                AnnGizmoDrag {
                    ann_id,
                    start_pointer: Pos2::new(100.0, 100.0),
                    start_trans: [5.0, 0.0, 0.0],
                    start_rot: [0.0, 0.0, 0.0],
                    start_scale: [1.0, 1.0, 1.0],
                    handle: GizmoHandle::Axis(0),
                    kind: GizmoKind::Translate,
                },
            )
        });

        // Mutate translation
        state.project.annotations[0].translation = [10.0, 0.0, 0.0];

        // Frame with Escape pressed
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });

        ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
                let painter = ui.painter_at(rect);
                let response = ui.allocate_rect(rect, egui::Sense::drag());
                let handled = draw(&ctx, &mut state, rect, &painter, &response);
                assert!(handled);
            });
        })
        .textures_delta
        .clear();

        // Reverted to start_trans
        assert_eq!(state.project.annotations[0].translation, [5.0, 0.0, 0.0]);
    }
}
