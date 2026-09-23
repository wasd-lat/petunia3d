//! Painel de Edição e Projeção UV (`uv_ui`).
//! Renderiza controles de projeção/escala UV e canvas interativo de edição 0..1.

use egui::{Color32, Pos2, Shape, Stroke, Ui};
use petunia_config::{TextId, text_id};
use petunia_core::AppState;
use petunia_module_uv::UvModule;

const UV_ROTATION_STEP_DEGREES: f32 = 15.0;
const UV_ROTATION_ACTIONS: [(f32, TextId, TextId); 2] = [
    (
        -UV_ROTATION_STEP_DEGREES,
        text_id::UV_ROTATE_CW,
        text_id::UV_ROTATE_CW_TIP,
    ),
    (
        UV_ROTATION_STEP_DEGREES,
        text_id::UV_ROTATE_CCW,
        text_id::UV_ROTATE_CCW_TIP,
    ),
];

fn selected_uv_faces(state: &AppState) -> Vec<usize> {
    if !state.uv_selected.is_empty() {
        let mut selected: Vec<_> = state.uv_selected.iter().copied().collect();
        selected.sort_unstable();
        selected
    } else {
        state
            .project
            .active_mesh()
            .map(|mesh| {
                mesh.faces
                    .iter()
                    .enumerate()
                    .filter_map(|(index, face)| face.selected.then_some(index))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn rotate_uv_selection(state: &mut AppState, degrees: f32) -> bool {
    let selected = selected_uv_faces(state);
    if selected.is_empty() || !degrees.is_finite() || degrees == 0.0 {
        return false;
    }
    state.uv_selected = selected.into_iter().collect();
    state.checkpoint("uv rotate");
    UvModule::rotate_selected(state, degrees.to_radians());
    state.emit_mesh_changed();
    true
}

pub fn draw_uv_panel(ui: &mut Ui, state: &mut AppState) {
    let l_uv = state.t_id(text_id::UV_TITLE);
    let l_reproj = state.t("uv.reproject");
    let l_scale = state.t("uv.scale");
    egui::CollapsingHeader::new(l_uv)
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(l_reproj)
                    .on_hover_text("Projeção planar no eixo dominante")
                    .clicked()
                {
                    UvModule::reproject(state);
                }
                if ui
                    .button("Cúbica (Box)")
                    .on_hover_text("Projeção cúbica nos 6 eixos")
                    .clicked()
                {
                    UvModule::project_cube(state);
                }
                if ui
                    .button("Auto Unwrap")
                    .on_hover_text("Desdobramento automático de malha via xatlas")
                    .clicked()
                {
                    match UvModule::unwrap_auto(state) {
                        Ok(charts) => {
                            state.set_status(format!("Auto Unwrap: {charts} ilhas geradas"))
                        }
                        Err(e) => state.set_status(format!("Erro no Unwrap: {e}")),
                    }
                }
                if ui.button(format!("{l_scale} +")).clicked() {
                    state.checkpoint("uv scale");
                    UvModule::scale_selected(state, 1.1);
                }
                if ui.button(format!("{l_scale} −")).clicked() {
                    state.checkpoint("uv scale");
                    UvModule::scale_selected(state, 1.0 / 1.1);
                }
                if ui
                    .add_enabled(
                        !selected_uv_faces(state).is_empty(),
                        egui::Button::new(state.t_id(text_id::UV_ROTATE_90)),
                    )
                    .on_hover_text(state.t_id(text_id::UV_ROTATE_90_TIP))
                    .clicked()
                {
                    rotate_uv_selection(state, 90.0);
                }
            });
            let has_selected_face = state
                .project
                .active_mesh()
                .is_some_and(|mesh| mesh.faces.iter().any(|face| face.selected));
            let has_seams = state
                .project
                .active_mesh()
                .is_some_and(|mesh| !mesh.uv_seams.is_empty());
            ui.horizontal_wrapped(|ui| {
                for (degrees, label, tip) in UV_ROTATION_ACTIONS {
                    if ui
                        .add_enabled(
                            !selected_uv_faces(state).is_empty(),
                            egui::Button::new(state.t_id(label)),
                        )
                        .on_hover_text(state.t_id(tip))
                        .clicked()
                    {
                        rotate_uv_selection(state, degrees);
                    }
                }

                if ui
                    .add_enabled(
                        has_selected_face,
                        egui::Button::new(state.t_id(text_id::UV_TOGGLE_SEAMS)),
                    )
                    .on_hover_text(state.t_id(text_id::UV_TOGGLE_SEAMS_TIP))
                    .clicked()
                {
                    if UvModule::toggle_selected_face_seams(state) {
                        state.set_status(state.t_id(text_id::UV_SEAMS_TOGGLED));
                    } else {
                        state.set_status(state.t_id(text_id::UV_SEAMS_NO_FACE));
                    }
                }
                if ui
                    .add_enabled(
                        has_seams,
                        egui::Button::new(state.t_id(text_id::UV_CLEAR_SEAMS)),
                    )
                    .on_hover_text(state.t_id(text_id::UV_CLEAR_SEAMS_TIP))
                    .clicked()
                    && UvModule::clear_all_seams(state)
                {
                    state.set_status(state.t_id(text_id::UV_SEAMS_CLEARED));
                }
            });
            ui.small(format!(
                "{}: {}",
                state.t_id(text_id::UV_SELECTED),
                state.uv_selected.len()
            ));
            // canvas UV 0..1 (preenche o pai: inspector estreito ou editor central).
            let size = ui.available_width().clamp(200.0, 720.0);
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click_and_drag());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 4.0, Color32::from_rgb(22, 22, 26));
            let to_screen =
                |u: f32, v: f32| egui::pos2(rect.min.x + u * size, rect.min.y + (1.0 - v) * size);
            if let Some(o) = state.project.assets.get(state.project.active) {
                for (fi, f) in o.mesh.faces.iter().enumerate() {
                    let pts: Vec<Pos2> = f.uv.iter().map(|q| to_screen(q[0], q[1])).collect();
                    if pts.len() < 3 {
                        continue;
                    }
                    let sel = state.uv_selected.contains(&fi) || f.selected;
                    let col = if sel {
                        Color32::from_rgb(255, 150, 50)
                    } else {
                        Color32::from_rgb(120, 170, 255)
                    };
                    p.add(Shape::convex_polygon(
                        pts,
                        if sel {
                            Color32::from_rgba_unmultiplied(255, 150, 50, 40)
                        } else {
                            Color32::TRANSPARENT
                        },
                        Stroke::new(1.0_f32, col),
                    ));
                }
            }
            // clique seleciona face (hit test); drag move selecionadas
            if resp.clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let u = (pos.x - rect.min.x) / size;
                let v = 1.0 - (pos.y - rect.min.y) / size;
                if let Some(fi) = UvModule::uv_hit(state, u, v) {
                    state.checkpoint("uv select");
                    if let Some(o) = state.project.active_mut()
                        && let Some(f) = o.mesh.faces.get_mut(fi)
                    {
                        f.selected = !f.selected;
                    }
                    state.uv_selected = state
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
                    state.sync_selection();
                }
            }
            if resp.drag_started() {
                state.checkpoint("uv move");
            }
            if resp.dragged() {
                let d = resp.drag_delta();
                UvModule::move_selected(state, d.x / size, -d.y / size);
            }
            if resp.drag_stopped() {
                state.emit_mesh_changed();
            }
            if resp.hovered() {
                let z = ui.input(|i| i.smooth_scroll_delta.y);
                if z.abs() > 0.5 {
                    state.checkpoint("uv scale");
                    UvModule::scale_selected(state, if z > 0.0 { 1.1 } else { 1.0 / 1.1 });
                    state.emit_mesh_changed();
                }
            }
            ui.small(state.t_id(text_id::UV_HINT));
        });
}

/// Resumo UV para o inspector do workspace UV (Wave 3).
///
/// O editor interativo mora no centro (§6.3); aqui só o essencial de contexto,
/// sem duplicar widgets interativos.
pub fn draw_uv_summary(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        egui::RichText::new(state.t_id(text_id::UV_TITLE))
            .size(12.0)
            .strong(),
    );
    if let Some(asset) = state.project.assets.get(state.project.active) {
        let selected = asset.mesh.faces.iter().filter(|f| f.selected).count();
        ui.label(format!(
            "{}: {}",
            state.t_id(text_id::UI_ASSETS),
            asset.name
        ));
        ui.label(format!(
            "{}: {} / {}",
            state.t_id(text_id::UV_FACES),
            selected,
            asset.mesh.faces.len()
        ));
    }
    ui.label(format!(
        "{}: {}",
        state.t_id(text_id::UV_SELECTED),
        state.uv_selected.len()
    ));
    ui.small(state.t_id(text_id::UV_HINT));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_controls_render_without_a_selection() {
        let context = egui::Context::default();
        let mut state = AppState::new("en");

        context
            .run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw_uv_panel(ui, &mut state);
                });
            })
            .textures_delta
            .clear();
    }

    #[test]
    fn fine_rotation_actions_match_their_direction_labels() {
        assert_eq!(UV_ROTATION_ACTIONS[0].0, -15.0);
        assert_eq!(UV_ROTATION_ACTIONS[0].1, text_id::UV_ROTATE_CW);
        assert_eq!(UV_ROTATION_ACTIONS[1].0, 15.0);
        assert_eq!(UV_ROTATION_ACTIONS[1].1, text_id::UV_ROTATE_CCW);
    }

    #[test]
    fn fine_uv_rotation_only_changes_selected_faces() {
        let mut state = AppState::new("en");
        let (selected_before, other_before) = {
            let mesh = state.project.active_mesh_mut().expect("active mesh");
            mesh.faces[0].selected = true;
            (mesh.faces[0].uv.clone(), mesh.faces[1].uv.clone())
        };

        assert!(rotate_uv_selection(&mut state, UV_ROTATION_STEP_DEGREES));
        let mesh = state.project.active_mesh().expect("active mesh");
        assert_ne!(mesh.faces[0].uv, selected_before);
        assert_eq!(mesh.faces[1].uv, other_before);
    }

    #[test]
    fn uv_rotation_without_selected_faces_is_a_noop() {
        let mut state = AppState::new("en");
        let before = state.project.active_mesh().expect("active mesh").faces[0]
            .uv
            .clone();

        assert!(!rotate_uv_selection(&mut state, UV_ROTATION_STEP_DEGREES));
        assert_eq!(
            state.project.active_mesh().expect("active mesh").faces[0].uv,
            before
        );
    }

    #[test]
    fn uv_rotation_does_not_reuse_selection_after_switching_assets() {
        let mut state = AppState::new("en");
        let second_mesh = {
            let first_mesh = state.project.active_mesh_mut().expect("active mesh");
            first_mesh.faces[0].selected = true;
            first_mesh.clone()
        };
        state.uv_selected.insert(0);

        let mut second_asset = petunia_project::Asset::new("Second mesh", second_mesh);
        second_asset.mesh.faces[0].selected = false;
        let second_asset_id = second_asset.id;
        state.project.assets.push(second_asset);
        assert!(state.set_active_asset_by_id(second_asset_id));

        let before = state.project.active_mesh().expect("active mesh").faces[0]
            .uv
            .clone();
        assert!(!rotate_uv_selection(&mut state, UV_ROTATION_STEP_DEGREES));
        assert_eq!(
            state.project.active_mesh().expect("active mesh").faces[0].uv,
            before
        );
    }
}
