//! Responsive viewport-local Tool Properties.
//!
//! This surface is deliberately separate from Object Properties and the
//! Modifier Stack. Wave 3 makes the card viewport-bounded and returns its
//! actual rectangle so viewport hit-testing can never click through it.

use egui::{RichText, Ui, Vec2};
use petunia_core::{AppState, Workspace};

use crate::adapters::popup::{PetuniaPopup, PetuniaPopupLabels, PetuniaPopupStyle};
use crate::tokens;

/// Id canônico do cartão flutuante de pincel.
///
/// Quem reabre o cartão depois de ele perder o foco é a paleta de pintura
/// (`paint_ui::draw_paint_tool`), via [`PetuniaPopup::reveal`].
pub const PAINT_CARD_ID: &str = "viewport.paint_brush_card";

const VIEWPORT_MARGIN: f32 = 12.0;
const MIN_USABLE_WIDTH: f32 = 112.0;
const MIN_USABLE_HEIGHT: f32 = 96.0;
const REGULAR_CONTENT_WIDTH: f32 = 280.0;
const COMPACT_CONTENT_WIDTH: f32 = 220.0;
const REGULAR_MAX_HEIGHT: f32 = 520.0;
const COMPACT_MAX_HEIGHT: f32 = 320.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopoverLayout {
    pub position: egui::Pos2,
    /// Width available to the content; frame margins are reserved separately.
    pub content_width: f32,
    /// Maximum total height budget inside the viewport.
    pub max_height: f32,
    pub compact: bool,
}

/// Computes a viewport-safe layout without reading global screen geometry.
pub fn layout_for_viewport(viewport: egui::Rect) -> Option<PopoverLayout> {
    let usable_width = (viewport.width() - VIEWPORT_MARGIN * 2.0).max(0.0);
    let usable_height = (viewport.height() - VIEWPORT_MARGIN * 2.0).max(0.0);
    if usable_width < MIN_USABLE_WIDTH || usable_height < MIN_USABLE_HEIGHT {
        return None;
    }

    let compact = usable_width < 300.0 || usable_height < 360.0;
    // Reserve 22 points for frame margins/stroke so the measured window stays
    // inside the usable viewport even at narrow sizes.
    let content_cap = if compact {
        COMPACT_CONTENT_WIDTH
    } else {
        REGULAR_CONTENT_WIDTH
    };
    let content_width = (usable_width - 22.0).min(content_cap).max(80.0);
    let max_height = usable_height
        .min(if compact {
            COMPACT_MAX_HEIGHT
        } else {
            REGULAR_MAX_HEIGHT
        })
        .max(80.0);

    Some(PopoverLayout {
        position: viewport.left_top() + Vec2::splat(VIEWPORT_MARGIN),
        content_width,
        max_height,
        compact,
    })
}

/// Draws canonical Tool Properties and returns the occupied viewport rectangle.
pub fn draw(ui: &mut Ui, state: &mut AppState, viewport: egui::Rect) -> Option<egui::Rect> {
    if state.is_active_locked() || state.session.primitive_session.is_some() {
        return None;
    }
    // O pincel tem cartão próprio: flutuante e **recolhível** (perde o foco no
    // clique fora, §34), com o corpo vindo do módulo de pintura.
    if state.active_tool == "paint" {
        return draw_paint_card(ui, state, viewport);
    }
    if state.workspace != Workspace::Model && state.workspace != Workspace::Paint {
        return None;
    }
    if !crate::modeling_tool_properties::supports(state) {
        return None;
    }
    let layout = layout_for_viewport(viewport)?;

    let active = state.active_tool.clone();
    let title = state
        .modal
        .as_ref()
        .map(|modal| crate::tool_fields::kind_label(state, modal.kind))
        .or_else(|| {
            state
                .pending_modal
                .map(|kind| crate::tool_fields::kind_label(state, kind))
        })
        .unwrap_or_else(|| {
            if active == "transform" {
                state.t("tool_properties.universal_transform")
            } else {
                let key = format!("tools.{active}");
                let translated = state.t(&key);
                if translated == key {
                    active.clone()
                } else {
                    translated
                }
            }
        });

    let frame = egui::Frame::new()
        .fill(tokens::bg_panel(state))
        .stroke(tokens::stroke_border_dyn(state))
        .corner_radius(tokens::RADIUS_CONTAINER)
        .inner_margin(egui::Margin::symmetric(10, 8));

    let window = egui::Window::new("viewport.tool_properties.window")
        .id(egui::Id::new("viewport.tool_properties"))
        .title_bar(false)
        .fixed_pos(layout.position)
        .default_width(layout.content_width)
        .max_width(layout.content_width + 20.0)
        .max_height(layout.max_height)
        .constrain_to(viewport)
        .collapsible(false)
        .resizable(false)
        .frame(frame)
        .show(ui.ctx(), |ui| {
            ui.set_width(layout.content_width);
            ui.spacing_mut().item_spacing = if layout.compact {
                egui::vec2(5.0, 4.0)
            } else {
                egui::vec2(6.0, 5.0)
            };

            ui.label(
                RichText::new(&title)
                    .strong()
                    .size(if layout.compact { 11.5 } else { 12.5 })
                    .color(tokens::text_primary(state)),
            );
            ui.label(
                RichText::new(state.t("tool_properties.title"))
                    .size(10.0)
                    .color(tokens::text_muted(state)),
            );
            ui.separator();

            // Header + frame consume roughly 48 pt. The inner ScrollArea keeps
            // long tool forms usable instead of escaping a short viewport.
            let body_height = (layout.max_height - 48.0).max(48.0);
            egui::ScrollArea::vertical()
                .id_salt("viewport.tool_properties.scroll")
                .max_height(body_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    crate::modeling_tool_properties::draw(ui, state);
                });
        });

    window.map(|window| window.response.rect.intersect(viewport))
}

/// Cartão flutuante do pincel (mesma arquitetura do cartão de ferramenta).
///
/// O corpo é o do módulo de pintura ([`crate::modules_ui::paint_ui`]) — não há
/// segunda cópia dos controles de pincel no produto.
fn draw_paint_card(ui: &mut Ui, state: &mut AppState, viewport: egui::Rect) -> Option<egui::Rect> {
    let layout = layout_for_viewport(viewport)?;
    let title = state.t("paint.brush");
    let labels = PetuniaPopupLabels {
        expand: state.t("ui.expand"),
        collapse: state.t("ui.collapse"),
    };
    let popup = PetuniaPopup::panel(PAINT_CARD_ID, title, PetuniaPopupStyle::from_state(state))
        .at(layout.position)
        .within(viewport)
        .width(layout.content_width.max(240.0))
        .max_height(layout.max_height)
        .labels(labels);
    let ctx = ui.ctx().clone();
    popup
        .show(&ctx, |ui| {
            crate::modules_ui::paint_ui::draw_brush_contents(ui, state)
        })
        .map(|response| response.rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_bounded_and_regular_on_large_viewports() {
        let viewport = egui::Rect::from_min_size(egui::pos2(40.0, 60.0), egui::vec2(1280.0, 800.0));
        let layout = layout_for_viewport(viewport).expect("large viewport");
        assert!(!layout.compact);
        assert_eq!(
            layout.position,
            viewport.left_top() + Vec2::splat(VIEWPORT_MARGIN)
        );
        assert!(layout.content_width <= REGULAR_CONTENT_WIDTH);
        assert!(layout.max_height <= REGULAR_MAX_HEIGHT);
    }

    #[test]
    fn layout_compacts_on_narrow_viewports() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(260.0, 260.0));
        let layout = layout_for_viewport(viewport).expect("compact viewport");
        assert!(layout.compact);
        assert!(layout.content_width + 22.0 <= viewport.width() - VIEWPORT_MARGIN * 2.0 + 0.01);
        assert!(layout.max_height <= viewport.height() - VIEWPORT_MARGIN * 2.0 + 0.01);
    }

    #[test]
    fn unusably_small_viewport_suppresses_overlay() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 80.0));
        assert!(layout_for_viewport(viewport).is_none());
    }

    #[test]
    fn primitive_session_owns_contextual_surface() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.workspace = Workspace::Model;
        state.active_tool = "primitives".to_owned();
        assert!(state.begin_primitive(petunia_core::PrimitiveKind::Cube, None));
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(draw(ui, &mut state, viewport).is_none());
        })
        .textures_delta
        .clear();
    }
}
