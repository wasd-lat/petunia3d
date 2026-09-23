//! Headless egui event regressions for the viewport transaction boundary.
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use petunia_core::{AppState, EditMode, ModalConstraint, ModalKind, ModuleRegistry};
use petunia_module_model::ToolRegistry;

use crate::modal_viewport;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0))
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn state() -> AppState {
    let mut state = AppState::new("en");
    state.set_edit_mode(EditMode::Edit);
    let mesh = state.project.active_mesh_mut().unwrap();
    mesh.deselect_all();
    mesh.faces[0].selected = true;
    mesh.sync_vert_selection_from_faces();
    state.sync_selection();
    state
}

fn frame(ctx: &egui::Context, state: &mut AppState, events: Vec<Event>) {
    let input = egui::RawInput {
        screen_rect: Some(rect()),
        events,
        ..Default::default()
    };
    ctx.run_ui(input, |_ui| {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("test.viewport"),
        ));
        modal_viewport::draw(ctx, state, rect(), &painter);
    })
    .textures_delta
    .clear();
}

fn mesh_snapshot(state: &AppState) -> String {
    format!("{:?}", state.project.active_mesh().unwrap())
}

#[test]
fn keyboard_session_mouse_preview_then_escape_restores_mesh() {
    let ctx = egui::Context::default();
    let mut state = state();
    let original = mesh_snapshot(&state);
    state.pending_modal = Some(ModalKind::Extrude);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(egui::pos2(450.0, 350.0))],
    );
    assert!(state.modal.is_some());
    assert_eq!(state.project.undo.depth(), (0, 0));
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(egui::pos2(570.0, 430.0))],
    );
    assert_ne!(mesh_snapshot(&state), original);
    frame(&ctx, &mut state, vec![key(Key::Escape)]);
    assert!(state.modal.is_none());
    assert_eq!(mesh_snapshot(&state), original);
    assert_eq!(state.project.undo.depth(), (0, 0));
}

#[test]
fn numeric_axis_session_enter_commits_one_checkpoint() {
    let ctx = egui::Context::default();
    let mut state = state();
    let original = state.project.active_mesh().unwrap().clone();
    state.pending_modal = Some(ModalKind::Move);
    frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
    frame(
        &ctx,
        &mut state,
        vec![key(Key::X), Event::Text("-1.25".into())],
    );
    frame(&ctx, &mut state, vec![key(Key::Enter)]);
    assert!(state.modal.is_none());
    assert_eq!(state.project.undo.depth(), (1, 0));
    for (before, after) in original
        .verts
        .iter()
        .zip(&state.project.active_mesh().unwrap().verts)
    {
        let expected = before.vec()
            - if before.selected {
                glam::Vec3::X * 1.25
            } else {
                glam::Vec3::ZERO
            };
        assert!((after.vec() - expected).length() < 1.0e-6);
    }
}

#[test]
fn empty_redraw_does_not_reapply_preview_or_emit_mesh_events() {
    let ctx = egui::Context::default();
    let mut state = state();
    state.pending_modal = Some(ModalKind::Move);
    frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(
            rect().center() + egui::vec2(25.0, 12.0),
        )],
    );
    let preview = mesh_snapshot(&state);
    state.events.drain();
    state.consume_dirty();
    frame(&ctx, &mut state, vec![]);
    assert_eq!(mesh_snapshot(&state), preview);
    assert_eq!(state.events.pending(), 0);
    assert!(!state.consume_dirty());
    assert_eq!(state.project.undo.depth(), (0, 0));
}

#[test]
fn invalid_number_cannot_confirm_last_valid_preview() {
    let ctx = egui::Context::default();
    let mut state = state();
    state.pending_modal = Some(ModalKind::Scale);
    frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(rect().center() + egui::vec2(20.0, 0.0))],
    );
    let valid_preview = mesh_snapshot(&state);
    // Zero is syntactically valid but rejected by the domain as a degenerate scale.
    frame(&ctx, &mut state, vec![Event::Text("0".into())]);
    frame(&ctx, &mut state, vec![]);
    frame(&ctx, &mut state, vec![key(Key::Enter)]);
    assert!(state.modal.is_some());
    assert_eq!(mesh_snapshot(&state), valid_preview);
    assert_eq!(state.project.undo.depth(), (0, 0));
    frame(&ctx, &mut state, vec![key(Key::Backspace)]);
    frame(&ctx, &mut state, vec![Event::Text("2".into())]);
    // Release before pressing Enter again to model a real second keystroke.
    let mut release = key(Key::Enter);
    if let Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(&ctx, &mut state, vec![release]);
    frame(&ctx, &mut state, vec![key(Key::Enter)]);
    assert!(state.modal.is_none());
    assert_eq!(state.project.undo.depth(), (1, 0));
}

#[test]
fn right_click_cancels_and_left_click_confirms_keyboard_sessions() {
    for (button_kind, commits) in [
        (PointerButton::Secondary, false),
        (PointerButton::Primary, true),
    ] {
        let ctx = egui::Context::default();
        let mut state = state();
        let original = mesh_snapshot(&state);
        state.pending_modal = Some(ModalKind::Inset);
        frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
        let pos = rect().center() + egui::vec2(40.0, 0.0);
        frame(&ctx, &mut state, vec![Event::PointerMoved(pos)]);
        frame(&ctx, &mut state, vec![button(pos, button_kind, true)]);
        assert!(state.modal.is_none());
        assert_eq!(state.project.undo.depth(), (usize::from(commits), 0));
        if !commits {
            assert_eq!(mesh_snapshot(&state), original);
        }
    }
}

#[test]
fn gizmo_drag_commits_on_release_without_extra_click() {
    let ctx = egui::Context::default();
    let mut state = state();
    let anchor = rect().center();
    modal_viewport::start_handle(
        &ctx,
        &mut state,
        ModalKind::Move,
        ModalConstraint::Axis(0),
        anchor,
    );
    frame(
        &ctx,
        &mut state,
        vec![
            Event::PointerMoved(anchor),
            button(anchor, PointerButton::Primary, true),
        ],
    );
    assert!(state.modal.is_some());
    let pos = anchor + egui::vec2(60.0, 0.0);
    frame(&ctx, &mut state, vec![Event::PointerMoved(pos)]);
    frame(
        &ctx,
        &mut state,
        vec![button(pos, PointerButton::Primary, false)],
    );
    assert!(state.modal.is_none());
    assert_eq!(state.project.undo.depth(), (1, 0));
}

#[test]
fn pending_tool_waits_for_viewport_and_escape_cancels_without_history() {
    let ctx = egui::Context::default();
    let mut state = state();
    let original = mesh_snapshot(&state);
    state.pending_modal = Some(ModalKind::Extrude);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(egui::pos2(-10.0, -10.0))],
    );
    assert!(state.modal.is_none());
    assert!(state.pending_modal.is_some());
    frame(&ctx, &mut state, vec![key(Key::Escape)]);
    assert!(state.pending_modal.is_none());
    assert_eq!(mesh_snapshot(&state), original);
    assert_eq!(state.project.undo.depth(), (0, 0));
}

fn text_position(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<Pos2> {
    fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.pos + shape.galley.size() * 0.5)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, text)),
            _ => None,
        }
    }
    shapes.iter().find_map(|shape| find(&shape.shape, text))
}

#[test]
fn transform_inspector_does_not_host_object_actions() {
    let ctx = egui::Context::default();
    let mut state = state();
    state.active_tool = "transform".into();
    let tools = ToolRegistry::with_defaults();
    let mut registry = ModuleRegistry::new();
    let mut panel_frame = |state: &mut AppState, events: Vec<Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(rect()),
                events,
                ..Default::default()
            },
            |ui| {
                crate::shell::draw(ui, state, &tools, &mut registry);
            },
        );
        out.textures_delta.clear();
        out
    };
    panel_frame(&mut state, vec![]);
    let output = panel_frame(&mut state, vec![]);
    assert!(text_position(&output.shapes, &state.t("actions.duplicate")).is_none());
    assert!(text_position(&output.shapes, &state.t("transform.reset_origin")).is_none());
}

#[test]
fn pointer_leaving_window_keeps_last_preview_until_cancel() {
    let ctx = egui::Context::default();
    let mut state = state();
    state.pending_modal = Some(ModalKind::Move);
    frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(
            rect().center() + egui::vec2(40.0, 20.0),
        )],
    );
    let preview = mesh_snapshot(&state);
    frame(&ctx, &mut state, vec![Event::PointerGone]);
    assert_eq!(mesh_snapshot(&state), preview);
    assert!(state.modal.is_some());
    assert_eq!(state.project.undo.depth(), (0, 0));
    frame(&ctx, &mut state, vec![key(Key::Escape)]);
    assert!(state.modal.is_none());
}
