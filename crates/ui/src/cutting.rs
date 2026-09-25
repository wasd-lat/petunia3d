//! Fluxos de faca, plano de corte e loop cut com checkpoint ao confirmar.
use egui::{Key, PointerButton, Pos2, Rect};
use glam::{Vec2, Vec3};
use petunia_core::cutting_session::CutSession;
use petunia_core::picking::{PickComponent, pick_mesh};
use petunia_core::viewport::LogicalRect;
use petunia_core::{AppState, SelectMode};
use petunia_mesh::{knife::EdgePoint, loop_cut::LoopRing};

pub fn draw(
    ctx: &egui::Context,
    state: &mut AppState,
    rect: Rect,
    painter: &egui::Painter,
) -> bool {
    let tool = state.active_tool.clone();
    if !matches!(tool.as_str(), "knife" | "slice" | "loop_cut") {
        return false;
    }
    let logical_rect =
        LogicalRect::from_min_max([rect.min.x, rect.min.y], [rect.max.x, rect.max.y]);

    if state.mesh_preview.is_none() {
        let Some(mesh) = state.project.active_mesh().cloned() else {
            return true;
        };
        if !state.begin_mesh_preview(match tool.as_str() {
            "knife" => "Knife",
            "slice" => "Slice",
            _ => "Loop cut",
        }) {
            return false;
        }
        let mut session = CutSession::new(mesh);
        if tool == "slice" {
            session.fill_cap = true;
        }
        state.cut_session = Some(session);
    }

    let Some(mut session) = state.cut_session.take() else {
        state.finish_mesh_preview(true);
        return true;
    };

    let cancel = ctx.input(|i| {
        i.key_pressed(Key::Escape) || i.pointer.button_pressed(PointerButton::Secondary)
    });
    if cancel
        && !(tool == "loop_cut" && session.sliding && !ctx.input(|i| i.key_pressed(Key::Escape)))
    {
        finish(state, true);
        return true;
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        finish(state, false);
        return true;
    }
    if cancel
        && tool == "loop_cut"
        && session.sliding
        && let Ok(mesh) = session.apply_loop_cut(0.0)
    {
        state.preview_mesh(mesh);
        finish(state, false);
        return true;
    }
    let Some(pos) = ctx.pointer_hover_pos().filter(|p| rect.contains(*p)) else {
        state.cut_session = Some(session);
        return true;
    };
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    let ndc = Vec2::new(
        (pos.x - rect.left()) / rect.width() * 2.0 - 1.0,
        1.0 - (pos.y - rect.top()) / rect.height() * 2.0,
    );
    let pressed = ctx.input(|i| i.pointer.button_pressed(PointerButton::Primary));
    let moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
    let mut message = "Clique nas arestas · Enter: aplicar · Esc/RMB: cancelar".to_owned();

    if tool == "knife" {
        let hit = state.project.active_mesh().and_then(|mesh| {
            pick_mesh(
                mesh,
                &state.camera,
                Vec2::new(rect.width(), rect.height()) * ctx.pixels_per_point(),
                ndc,
                SelectMode::Edge,
                false,
            )
        });
        if let Some(start) = session.edge_start {
            painter.line_segment(
                [screen(state, logical_rect, start.position), pos],
                egui::Stroke::new(2.0_f32, egui::Color32::YELLOW),
            );
        }
        if pressed
            && let Some(hit) = hit
            && let PickComponent::Edge(a, b) = hit.component
        {
            let point = EdgePoint {
                edge: (a, b),
                position: hit.position,
            };
            if let Some(start) = session.edge_start {
                if let Some(mesh) = state.project.active_mesh() {
                    match session.cut_knife_segment(start, point, mesh) {
                        Ok(mesh) => {
                            state.preview_mesh(mesh);
                            session.edge_start = None;
                        }
                        Err(error) => state.set_status(error),
                    }
                }
            } else {
                session.edge_start = Some(point);
            }
        }
    } else if tool == "slice" {
        message = "Arraste o plano · Enter/LMB: aplicar · Esc/RMB: cancelar".into();
        if pressed {
            if session.sliding {
                finish(state, false);
                return true;
            }
            session.anchor = Some([pos.x, pos.y]);
        }
        if let Some(anchor) = session.anchor {
            let anchor_pos = Pos2::new(anchor[0], anchor[1]);
            painter.line_segment(
                [anchor_pos, pos],
                egui::Stroke::new(2.0_f32, egui::Color32::YELLOW),
            );
            if moved
                && ctx.input(|i| i.pointer.button_down(PointerButton::Primary))
                && let Some(mesh) =
                    session.compute_slice(&state.camera, anchor, [pos.x, pos.y], logical_rect)
            {
                state.preview_mesh(mesh);
            }
            if ctx.input(|i| i.pointer.button_released(PointerButton::Primary)) {
                session.sliding = true;
            }
        }
    } else {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            session.adjust_cuts(if scroll > 0.0 { 1 } else { -1 });
        }
        if !session.sliding {
            session.ring = None;
            if let Some(hit) = pick_mesh(
                &session.source,
                &state.camera,
                Vec2::new(rect.width(), rect.height()) * ctx.pixels_per_point(),
                ndc,
                SelectMode::Edge,
                false,
            ) && let PickComponent::Edge(a, b) = hit.component
            {
                match LoopRing::discover(&session.source, (a, b)) {
                    Ok(ring) => session.ring = Some(ring),
                    Err(error) => {
                        session.ring = None;
                        state.set_status(error.to_string());
                    }
                }
            }
        }
        let slide = if cancel {
            0.0
        } else {
            session.compute_loop_slide(pos.x)
        };
        if session.ring.is_some() {
            if let Ok(lines) = session.preview_loop_lines(slide) {
                for [a, b] in lines {
                    painter.line_segment(
                        [
                            screen(state, logical_rect, a),
                            screen(state, logical_rect, b),
                        ],
                        egui::Stroke::new(2.5_f32, egui::Color32::YELLOW),
                    );
                }
            }
            if pressed || (session.sliding && (moved || scroll != 0.0 || cancel)) {
                match session.apply_loop_cut(slide) {
                    Ok(mesh) => state.preview_mesh(mesh),
                    Err(error) => state.set_status(error.to_string()),
                }
                if session.sliding && (pressed || cancel) {
                    finish(state, false);
                    return true;
                }
                if pressed {
                    session.sliding = true;
                    session.anchor = Some([pos.x, pos.y]);
                }
            }
        }
        message = format!(
            "Loop cut: {} · Scroll: cortes · LMB: {} · Esc: cancelar",
            session.cuts,
            if session.sliding {
                "aplicar · RMB: centro"
            } else {
                "deslizar"
            }
        );
    }
    painter.text(
        rect.left_top() + egui::vec2(12.0, 32.0),
        egui::Align2::LEFT_TOP,
        message,
        egui::FontId::monospace(13.0),
        egui::Color32::YELLOW,
    );
    state.cut_session = Some(session);
    true
}

fn finish(state: &mut AppState, cancel: bool) {
    state.finish_mesh_preview(cancel);
    state.active_tool = "select".into();
    state.cut_session = None;
}

fn screen(state: &AppState, rect: LogicalRect, p: Vec3) -> Pos2 {
    let ndc = state.camera.project_ndc(p);
    let s = rect.ndc_to_screen([ndc.x, ndc.y]);
    Pos2::new(s[0], s[1])
}
