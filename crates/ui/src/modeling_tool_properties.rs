//! Canonical viewport-local controls for modeling tools.
//!
//! Tool Properties are intentionally separate from Object Properties and the
//! Modifier Stack. Modal tools delegate to `tool_fields`, which owns the same
//! transaction used by viewport interaction. Non-modal modeling operations are
//! rendered here so there is no second/legacy execution path in the Inspector.

use egui::Ui;
use petunia_core::{AppState, Command, EditMode, MergeCenterCmd, ModalKind, RevolveCmd, WeldCmd};

/// Whether the current modeling state owns a contextual Tool Properties card.
pub fn supports(state: &AppState) -> bool {
    if state.modal.is_some() || state.pending_modal.is_some() {
        return true;
    }
    matches!(
        state.active_tool.as_str(),
        "transform"
            | "move"
            | "rotate"
            | "scale"
            | "primitives"
            | "extrude"
            | "inset"
            | "bevel"
            | "pushpull"
            | "slice"
            | "subdivide"
            | "draw_profile"
            | "merge"
            | "connect"
            | "dissolve"
            | "revolve"
    )
}

/// Draw the canonical controls for the current tool.
pub fn draw(ui: &mut Ui, state: &mut AppState) {
    // A modal/pending modal always wins: one transaction, one preview, one
    // commit/cancel path, whether it came from shortcut, toolbar or property UI.
    if (state.modal.is_some() || state.pending_modal.is_some())
        && crate::tool_fields::draw(ui, state)
    {
        return;
    }

    match state.active_tool.as_str() {
        "move" | "rotate" | "scale" | "extrude" | "inset" | "bevel" | "pushpull" => {
            let _ = crate::tool_fields::draw(ui, state);
        }
        "transform" => draw_universal_transform(ui, state),
        "primitives" => draw_primitives(ui, state),
        "subdivide" => draw_subdivide(ui, state),
        "slice" => draw_slice(ui, state),
        "connect" => draw_connect(ui, state),
        "merge" => draw_merge(ui, state),
        "dissolve" => draw_dissolve(ui, state),
        "revolve" => draw_revolve(ui, state),
        "draw_profile" => draw_profile(ui, state),
        _ => {}
    }
}

// Os controles de pincel pertencem ao Context de PAINT e são desenhados por
// `paint_ui::draw_brush_contents`; este popover cuida das ferramentas MODEL.

fn edit_guard(state: &AppState) -> Option<String> {
    if state.project.active_mesh().is_none() {
        Some(state.t("actions.no_mesh"))
    } else if state.edit_mode() != EditMode::Edit {
        Some(state.t("actions.need_edit"))
    } else {
        None
    }
}

fn selection_guard(state: &AppState) -> Option<String> {
    edit_guard(state).or_else(|| {
        state
            .selection
            .is_empty()
            .then(|| state.t("actions.need_selection"))
    })
}

fn disabled_reason(ui: &mut Ui, reason: &Option<String>) {
    if let Some(reason) = reason {
        ui.small(reason);
    }
}

fn activate_transform(state: &mut AppState, id: &str, kind: ModalKind) {
    state.active_tool = id.to_owned();
    state.gizmo_mode = kind;
    state.pending_modal = Some(kind);
    state.mark_dirty();
}

fn draw_universal_transform(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("tool_properties.choose_transform"));
    ui.horizontal(|ui| {
        if ui.button(state.t("tools.move")).clicked() {
            activate_transform(state, "move", ModalKind::Move);
        }
        if ui.button(state.t("tools.rotate")).clicked() {
            activate_transform(state, "rotate", ModalKind::Rotate);
        }
        if ui.button(state.t("tools.scale")).clicked() {
            activate_transform(state, "scale", ModalKind::Scale);
        }
    });
    ui.small(state.t("hints.transform"));
}

fn draw_primitives(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.primitives"));
    let items = [
        ("Cube", "prims.cube"),
        ("Plane", "prims.plane"),
        ("Wedge", "prims.wedge"),
        ("Cylinder", "prims.cylinder"),
        ("Cone", "prims.cone"),
        ("Circle", "prims.circle"),
        ("Torus", "prims.torus"),
        ("Sphere", "prims.sphere"),
        ("Icosphere", "prims.icosphere"),
        ("Capsule", "prims.capsule"),
    ];
    egui::Grid::new("tool_properties.primitive_grid")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (index, (name, text_id)) in items.into_iter().enumerate() {
                if ui.button(state.t(text_id)).clicked() {
                    petunia_module_model::PrimitivesTool::add_primitive(state, name);
                }
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

fn draw_subdivide(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.subdivide"));
    let mut cuts = state.subdivide_cuts as i64;
    if ui
        .add(
            egui::Slider::new(&mut cuts, 1..=6)
                .text(state.t("tool_properties.cuts"))
                .step_by(1.0),
        )
        .changed()
    {
        state.subdivide_cuts = cuts.clamp(1, 6) as u32;
        state.mark_dirty();
    }

    let reason = selection_guard(state);
    ui.horizontal(|ui| {
        let subdivide = ui
            .add_enabled(
                reason.is_none(),
                egui::Button::new(state.t("actions.subdivide")),
            )
            .on_hover_text(reason.clone().unwrap_or_else(|| "W".to_owned()));
        if subdivide.clicked() {
            petunia_module_model::SubdivideTool::apply_subdivide(state);
        }
        let triangulate = ui.add_enabled(
            reason.is_none(),
            egui::Button::new(state.t("actions.triangulate")),
        );
        if triangulate.clicked() {
            petunia_module_model::SubdivideTool::apply_triangulate(state);
        }
    });
    disabled_reason(ui, &reason);
}

fn draw_slice(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.slice"));
    let reason = edit_guard(state);
    ui.horizontal(|ui| {
        for (label, axis) in [
            (state.t("actions.slice_x"), glam::Vec3::X),
            (state.t("actions.slice_y"), glam::Vec3::Y),
            (state.t("actions.slice_z"), glam::Vec3::Z),
        ] {
            let response = ui
                .add_enabled(reason.is_none(), egui::Button::new(label))
                .on_hover_text(reason.clone().unwrap_or_default());
            if response.clicked() {
                petunia_module_model::SliceTool::apply_slice(state, axis, false);
            }
        }
    });
    let cap = state.t("actions.slice_cap");
    let response = ui
        .add_enabled(reason.is_none(), egui::Button::new(cap.clone()))
        .on_hover_text(reason.clone().unwrap_or(cap));
    if response.clicked() {
        let camera_direction = state.camera.forward();
        petunia_module_model::SliceTool::apply_slice(state, camera_direction, true);
    }
    disabled_reason(ui, &reason);
}

fn draw_connect(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.connect"));
    let reason = selection_guard(state);
    let response = ui
        .add_enabled(
            reason.is_none(),
            egui::Button::new(state.t("actions.connect")),
        )
        .on_hover_text(reason.clone().unwrap_or_else(|| "B".to_owned()));
    if response.clicked() {
        petunia_module_model::ConnectTool::apply(state);
    }
    disabled_reason(ui, &reason);
}

fn draw_merge(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.merge"));
    let center_command = MergeCenterCmd;
    let center_reason = center_command
        .can_execute(state)
        .err()
        .map(|e| e.to_string());
    let response = ui
        .add_enabled(
            center_reason.is_none(),
            egui::Button::new(state.t("actions.merge_center")),
        )
        .on_hover_text(center_reason.clone().unwrap_or_else(|| "M".to_owned()));
    if response.clicked() {
        petunia_module_model::MergeTool::apply(state);
    }
    disabled_reason(ui, &center_reason);

    ui.separator();
    ui.label(state.t("actions.merge_by_distance"));
    let merge_distance_label = state.t("actions.merge_distance");
    if ui
        .add(
            egui::Slider::new(&mut state.merge_dist, 0.0005..=0.1)
                .text(merge_distance_label)
                .logarithmic(true),
        )
        .changed()
    {
        state.mark_dirty();
    }
    let weld_command = WeldCmd {
        eps: state.merge_dist,
    };
    let weld_reason = weld_command.can_execute(state).err().map(|e| e.to_string());
    let response = ui
        .add_enabled(
            weld_reason.is_none(),
            egui::Button::new(state.t("actions.merge_by_distance")),
        )
        .on_hover_text(weld_reason.clone().unwrap_or_default());
    if response.clicked() {
        petunia_module_model::MergeTool::apply_by_distance(state);
    }
    disabled_reason(ui, &weld_reason);
}

fn draw_dissolve(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.dissolve"));
    let reason = selection_guard(state);
    let response = ui
        .add_enabled(
            reason.is_none(),
            egui::Button::new(state.t("actions.dissolve")),
        )
        .on_hover_text(reason.clone().unwrap_or_else(|| "X".to_owned()));
    if response.clicked() {
        petunia_module_model::DissolveTool::apply(state);
    }
    disabled_reason(ui, &reason);
}

fn draw_revolve(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("hints.revolve"));
    let mut segments = state.revolve_segments as i64;
    if ui
        .add(
            egui::Slider::new(&mut segments, 3..=64)
                .text(state.t("prims.segments"))
                .step_by(1.0),
        )
        .changed()
    {
        state.revolve_segments = segments.clamp(3, 64) as u32;
        state.mark_dirty();
    }
    let angle_label = state.t("actions.angle");
    if ui
        .add(egui::Slider::new(&mut state.revolve_angle, 5.0..=360.0).text(angle_label))
        .changed()
    {
        state.mark_dirty();
    }
    if state.revolve_angle < 359.5 {
        ui.small(state.t("actions.revolve_open_hint"));
    }
    ui.horizontal(|ui| {
        ui.label(state.t("modifiers.axis"));
        for (axis, name) in [(0, "X"), (1, "Y"), (2, "Z")] {
            if ui
                .selectable_label(state.revolve_axis == axis, name)
                .clicked()
            {
                state.revolve_axis = axis;
                state.mark_dirty();
            }
        }
    });

    let reason = selection_guard(state);
    let response = ui
        .add_enabled(
            reason.is_none(),
            egui::Button::new(state.t("actions.revolve")),
        )
        .on_hover_text(reason.clone().unwrap_or_else(|| state.t("hints.revolve")));
    if response.clicked() {
        let center = state
            .project
            .active_mesh()
            .map(|mesh| mesh.selection_center())
            .unwrap_or([0.0, 0.0, 0.0]);
        let command = RevolveCmd {
            segments: state.revolve_segments,
            angle_deg: state.revolve_angle,
            axis: state.revolve_axis,
            center,
        };
        if let Err(error) = state.dispatch(&command) {
            state.set_status(format!("{}: {error}", state.t("tools.revolve")));
        }
    }
    if let Some(reason) = &reason {
        ui.small(reason);
    } else {
        ui.small(state.t("actions.revolve_need_edges"));
    }
}

fn draw_profile(ui: &mut Ui, state: &mut AppState) {
    ui.small(state.t("tool_properties.profile_usage"));
    ui.label(format!(
        "{}: {}",
        state.t("profile.points"),
        state.profile.points.len()
    ));

    let mut snap = state.profile.snap;
    if ui.checkbox(&mut snap, state.t("profile.snap")).changed() {
        state.profile.snap = snap;
        state.mark_dirty();
    }
    let profile_depth_label = state.t("profile.depth");
    if ui
        .add(egui::Slider::new(&mut state.profile.depth, 0.05..=8.0).text(profile_depth_label))
        .changed()
    {
        state.mark_dirty();
    }
    let profile_segments_label = state.t("profile.segments");
    if ui
        .add(
            egui::Slider::new(&mut state.profile.revolve_segments, 3..=48)
                .text(profile_segments_label),
        )
        .changed()
    {
        state.mark_dirty();
    }

    ui.horizontal(|ui| {
        if ui.button(state.t("profile.close")).clicked() {
            if state.profile.points.len() >= 3 {
                state.profile.closed = true;
            }
            state.mark_dirty();
        }
        if ui.button(state.t("profile.undo_pt")).clicked() {
            state.profile.points.pop();
            state.profile.closed = false;
            state.mark_dirty();
        }
        if ui.button(state.t("profile.clear")).clicked() {
            state.profile.clear();
            state.mark_dirty();
        }
    });
    ui.horizontal(|ui| {
        if ui.button(state.t("profile.gen_extrude")).clicked() {
            petunia_module_model::draw_profile::generate_extrude(state);
        }
        if ui.button(state.t("profile.gen_revolve")).clicked() {
            petunia_module_model::draw_profile::generate_revolve(state);
        }
    });

    if state.profile.closed {
        match petunia_mesh::triangulate::ear_clip(&state.profile.points) {
            Ok(triangles) => {
                ui.small(format!("{}: {}", state.t("profile.tris"), triangles.len()));
            }
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_surface_supports_all_contextual_modeling_tools() {
        let mut state = AppState::new("en");
        for id in [
            "transform",
            "move",
            "rotate",
            "scale",
            "primitives",
            "extrude",
            "inset",
            "bevel",
            "pushpull",
            "slice",
            "subdivide",
            "draw_profile",
            "merge",
            "connect",
            "dissolve",
            "revolve",
        ] {
            state.active_tool = id.to_owned();
            assert!(supports(&state), "missing Tool Properties for {id}");
        }
        state.active_tool = "select".to_owned();
        assert!(!supports(&state));
    }

    #[test]
    fn non_modal_property_surfaces_render_without_panicking() {
        let context = egui::Context::default();
        for id in [
            "transform",
            "primitives",
            "slice",
            "subdivide",
            "draw_profile",
            "merge",
            "connect",
            "dissolve",
            "revolve",
        ] {
            let mut state = AppState::new("en");
            state.set_edit_mode(EditMode::Edit);
            state.active_tool = id.to_owned();
            context
                .run_ui(egui::RawInput::default(), |ui| draw(ui, &mut state))
                .textures_delta
                .clear();
        }
    }
}
