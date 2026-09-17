//! Real egui pointer event regressions for continuous vertex painting.
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use petunia_core::{AppState, EditMode, ViewPreset, Workspace};

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))
}
fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
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
fn frame(ctx: &egui::Context, state: &mut AppState, events: Vec<Event>) {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(rect()),
            events,
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let response = ui.interact(
                    rect(),
                    egui::Id::new("paint.test.viewport"),
                    egui::Sense::click_and_drag(),
                );
                crate::viewport_interaction::draw(ctx, state, rect(), ui.painter(), &response);
            });
        },
    )
    .textures_delta
    .clear();
}
fn state() -> AppState {
    let mut state = AppState::new("en");
    state.workspace = Workspace::Paint;
    state.set_edit_mode(EditMode::TexturePaint);
    state.active_tool = "paint".into();
    state.camera.set_preset(ViewPreset::Front);
    state.camera.aspect = rect().aspect_ratio();
    state.paint_radius = 10.0;
    state.paint_strength = 0.25;
    state
}
fn colors(state: &AppState) -> Vec<[f32; 3]> {
    state
        .project
        .active_mesh()
        .unwrap()
        .verts
        .iter()
        .map(|v| v.color)
        .collect()
}
fn start_drag(ctx: &egui::Context, state: &mut AppState) {
    frame(ctx, state, vec![Event::PointerMoved(rect().center())]);
    frame(ctx, state, vec![button(rect().center(), true)]);
    frame(
        ctx,
        state,
        vec![Event::PointerMoved(rect().center() + egui::vec2(20.0, 0.0))],
    );
    assert!(state.paint_stroke.is_some());
}

#[test]
fn paint_mouse_drag_release_commits_one_checkpoint() {
    let ctx = egui::Context::default();
    let mut state = state();
    let original = colors(&state);
    start_drag(&ctx, &mut state);
    assert_ne!(colors(&state), original);
    assert_eq!(state.project.undo.depth(), (0, 0));
    frame(
        &ctx,
        &mut state,
        vec![button(rect().center() + egui::vec2(20.0, 0.0), false)],
    );
    assert!(state.paint_stroke.is_none());
    assert_eq!(state.project.undo.depth(), (1, 0));
    state.undo();
    assert_eq!(colors(&state), original);
}

#[test]
fn escape_cancels_paint_without_restarting_held_drag() {
    let ctx = egui::Context::default();
    let mut state = state();
    let original = colors(&state);
    start_drag(&ctx, &mut state);
    frame(&ctx, &mut state, vec![key(Key::Escape)]);
    assert!(state.paint_stroke.is_none());
    assert_eq!(colors(&state), original);
    assert_eq!(state.project.undo.depth(), (0, 0));
    frame(&ctx, &mut state, vec![]);
    assert!(state.paint_stroke.is_none());
    assert_eq!(colors(&state), original);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(rect().center() + egui::vec2(45.0, 0.0))],
    );
    assert!(state.paint_stroke.is_none());
    assert_eq!(colors(&state), original);
}

#[test]
fn idle_redraw_during_held_brush_does_not_repaint_or_request_render() {
    let ctx = egui::Context::default();
    let mut state = state();
    start_drag(&ctx, &mut state);
    let preview = colors(&state);
    state.events.drain();
    state.consume_dirty();
    frame(&ctx, &mut state, vec![]);
    assert_eq!(colors(&state), preview);
    assert_eq!(state.events.pending(), 0);
    assert!(!state.consume_dirty());
}

#[test]
fn size_drag_escape_restores_size_and_ends_adjustment() {
    // O arrasto com F ajusta o **tamanho em pixels** (`canvas_brush`, o campo que
    // `brush_settings().size_px` lê), não o raio de mundo legado: o anel de
    // preview e o carimbo usam o mesmo número, então o que o usuário vê é o que
    // o pincel pinta.
    let ctx = egui::Context::default();
    let mut state = state();
    let original = state.canvas_brush;
    frame(&ctx, &mut state, vec![Event::PointerMoved(rect().center())]);
    frame(&ctx, &mut state, vec![key(Key::F)]);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(rect().center() + egui::vec2(50.0, 0.0))],
    );
    assert!(state.canvas_brush > original);
    frame(&ctx, &mut state, vec![key(Key::Escape)]);
    assert_eq!(state.canvas_brush, original);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(
            rect().center() + egui::vec2(100.0, 0.0),
        )],
    );
    assert_eq!(state.canvas_brush, original);
    assert_eq!(state.project.undo.depth(), (0, 0));
    let mut release = key(Key::F);
    if let Event::Key { pressed, .. } = &mut release {
        *pressed = false;
    }
    frame(&ctx, &mut state, vec![release]);
    frame(&ctx, &mut state, vec![key(Key::F)]);
    frame(
        &ctx,
        &mut state,
        vec![Event::PointerMoved(
            rect().center() + egui::vec2(150.0, 0.0),
        )],
    );
    assert!(
        state.canvas_brush > original,
        "size can be adjusted again without a mouse click"
    );
}
