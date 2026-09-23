//! Barra de status inferior do Petunia3D (`Status Bar`).
//! Estruturada de forma limpa, contextual e acessível (Baseline V1 / Cap. 36):
//! 1. Indicador sutil de salvamento e mensagem contextual de status/operação;
//! 2. Dica contextual truncada (domínio de seleção, atalhos do mouse, confirmação);
//! 3. Telemetria limpa de cena (Triângulos e Vértices) à direita.

use egui::{Align, Layout, RichText, Ui, vec2};
use petunia_core::{AppState, SelectionDomain};
use petunia_module_model::ToolRegistry;

use crate::tokens;

/// Renderiza a barra de status inferior simplificada e contextual.
pub fn draw(ui: &mut Ui, state: &mut AppState, tools: &ToolRegistry) {
    let is_saved = !state.is_document_dirty();
    let save_color = if is_saved {
        tokens::ACCENT_GREEN
    } else {
        tokens::ACCENT_AMBER
    };
    let save_tooltip = state.t(if is_saved {
        "status.saved_hint"
    } else {
        "status.unsaved_hint"
    });

    let status_text = if !state.ui.status.is_empty() {
        state.ui.status.clone()
    } else if state.is_interacting() {
        state
            .current_tool_feedback()
            .map(|fb| fb.delta_text)
            .unwrap_or_else(|| state.active_tool.clone())
    } else {
        state.t("status.ready")
    };

    let hint = if let Some(fb) = state.current_tool_feedback() {
        fb.status_hint.to_string()
    } else if state.is_interacting() {
        state.t("status.hint_interact")
    } else if let Some(t) = tools.get(&state.active_tool) {
        state.t(t.hint_key())
    } else {
        match state.selection_domain() {
            SelectionDomain::Object => state.t("status.hint_object"),
            SelectionDomain::Vertex => state.t("status.hint_point"),
            SelectionDomain::Edge => state.t("status.hint_edge"),
            SelectionDomain::Face => state.t("status.hint_face"),
        }
    };

    let scene_tris = state.scene_tris();
    let scene_verts = state.scene_verts();

    let bar_resp = egui::Panel::bottom("status_bar")
        .exact_size(tokens::STATUS_BAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_header(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(10, 2)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 0.0);

                // Indicador discreto de salvo / alterações não salvas
                ui.label(RichText::new("●").size(9.0).color(save_color))
                    .on_hover_text(&save_tooltip);

                // Mensagem de status (forte/primária)
                ui.label(
                    RichText::new(&status_text)
                        .size(11.0)
                        .strong()
                        .color(tokens::TEXT_PRIMARY),
                );

                ui.separator();

                // Telemetria alinhada à direita calculada antes
                let telemetry = format!("Tris: {scene_tris} · Verts: {scene_verts}");
                let tele_w = ui
                    .fonts_mut(|f| {
                        f.layout_no_wrap(
                            telemetry.clone(),
                            egui::FontId::monospace(10.5),
                            tokens::TEXT_MUTED,
                        )
                        .size()
                        .x
                    })
                    .max(80.0);

                let available = (ui.available_width() - tele_w - 24.0).max(40.0);

                // Dica contextual truncada
                ui.add_sized(
                    vec2(available, 16.0),
                    egui::Label::new(RichText::new(&hint).size(10.5).color(tokens::TEXT_MUTED))
                        .truncate(),
                )
                .on_hover_text(&hint);

                // Telemetria à direita
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(telemetry)
                            .size(10.5)
                            .color(tokens::TEXT_MUTED)
                            .monospace(),
                    );
                });
            });
        });

    crate::regions::record(
        ui.ctx(),
        crate::regions::RegionSlot::StatusBar,
        bar_resp.response.rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let tools = ToolRegistry::default();

        ctx.run_ui(egui::RawInput::default(), |ui| {
            draw(ui, &mut state, &tools);
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_status_bar_displays_dirty_state() {
        let mut state = AppState::new("en");
        state.mark_document_clean();
        assert!(!state.is_document_dirty());
        state.mark_document_dirty();
        assert!(state.is_document_dirty());
    }
}
