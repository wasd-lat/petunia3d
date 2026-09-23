//! Diálogo de Recuperação de Projeto após encerramento inesperado (P3D-002 §Recovery Dialog).
//!
//! Exibe informações claras e opções explícitas:
//! - Recover (carrega o snapshot como estado modificado sem sobrescrever o original);
//! - Open Saved Version (abre o último save oficial do disco);
//! - Discard Recovery (descarta os snapshots de autosave).

use egui::{Align, Context, Layout, RichText, Stroke, Window, vec2};
use petunia_core::{AppState, ProjectService, RecoveryInfo};

use crate::tokens;

pub enum RecoveryAction {
    Recover,
    OpenSaved,
    Discard,
}

/// Renderiza o diálogo modal de recuperação caso haja uma recuperação pendente.
pub fn draw(ctx: &Context, state: &mut AppState, info: &RecoveryInfo) -> Option<RecoveryAction> {
    let mut action = None;
    // Wave 5 (§9.4): máximo nunca excede a viewport útil.
    let screen_rect = ctx.viewport_rect();
    let (default_size, min_size, max_size) = crate::regions::modal_sizes(
        screen_rect,
        vec2(screen_rect.width() * 0.5, 240.0),
        vec2(380.0, 200.0),
        vec2(560.0, 420.0),
    );

    Window::new(
        RichText::new(state.t("recovery.title"))
            .strong()
            .size(13.5)
            .color(tokens::ACCENT_AMBER),
    )
    .default_size(default_size)
    .min_size(min_size)
    .max_size(max_size)
    .resizable(false)
    .collapsible(false)
    .frame(
        egui::Frame::window(&ctx.style_of(ctx.theme()))
            .fill(tokens::BG_PANEL)
            .stroke(Stroke::new(1.5_f32, tokens::ACCENT_AMBER))
            .inner_margin(egui::Margin::same(16)),
    )
    .show(ctx, |ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(format!(
                    "A previous session for '{}' ended unexpectedly.",
                    info.project_name
                ))
                .strong()
                .size(12.5)
                .color(tokens::TEXT_PRIMARY),
            );

            ui.add_space(8.0);

            ui.label(
                RichText::new(
                    "A recent recovery snapshot was preserved. Would you like to restore it?",
                )
                .size(11.5)
                .color(tokens::TEXT_SECONDARY),
            );

            ui.add_space(10.0);

            egui::Frame::NONE
                .fill(tokens::BG_SURFACE)
                .stroke(tokens::stroke_border())
                .corner_radius(tokens::RADIUS_CONTROL)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Snapshot:")
                                .size(11.0)
                                .color(tokens::TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(
                                info.snapshot_path
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("autosave"),
                            )
                            .size(11.0)
                            .color(tokens::TEXT_PRIMARY),
                        );
                    });

                    if let Some(ref main_path) = info.main_project_path {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Official File:")
                                    .size(11.0)
                                    .color(tokens::TEXT_MUTED),
                            );
                            ui.label(
                                RichText::new(
                                    main_path
                                        .file_name()
                                        .and_then(|f| f.to_str())
                                        .unwrap_or("project.petunia"),
                                )
                                .size(11.0)
                                .color(tokens::TEXT_PRIMARY),
                            );
                        });
                    }

                    if info.is_newer_than_main {
                        ui.label(
                            RichText::new("• Snapshot contains changes newer than the saved file")
                                .size(10.5)
                                .color(tokens::ACCENT_GREEN),
                        );
                    }
                });

            ui.add_space(14.0);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Botão principal: Recover
                let recover_btn = egui::Button::new(
                    RichText::new("Recover Project")
                        .size(11.5)
                        .strong()
                        .color(tokens::TEXT_ACTIVE),
                )
                .fill(tokens::bg_surface_active_for(ui.ctx()))
                .corner_radius(tokens::RADIUS_CONTROL);

                if ui.add(recover_btn).clicked() {
                    let _ = ProjectService::recover_from_snapshot(
                        state,
                        &info.snapshot_path,
                        info.main_project_path.as_deref(),
                    );
                    action = Some(RecoveryAction::Recover);
                }

                // Botão: Open Saved (quando arquivo oficial existir)
                if let Some(ref main_path) = info.main_project_path {
                    let open_saved_btn = egui::Button::new(
                        RichText::new("Open Saved Version")
                            .size(11.5)
                            .color(tokens::TEXT_PRIMARY),
                    )
                    .fill(tokens::BG_SURFACE)
                    .stroke(tokens::stroke_border())
                    .corner_radius(tokens::RADIUS_CONTROL);

                    if ui.add(open_saved_btn).clicked() {
                        let _ = ProjectService::load_project(state, main_path);
                        action = Some(RecoveryAction::OpenSaved);
                    }
                }

                // Botão: Discard
                let discard_btn = egui::Button::new(
                    RichText::new("Discard")
                        .size(11.5)
                        .color(tokens::TEXT_MUTED),
                )
                .fill(tokens::BG_PANEL)
                .corner_radius(tokens::RADIUS_CONTROL);

                if ui.add(discard_btn).clicked() {
                    action = Some(RecoveryAction::Discard);
                }
            });
        });
    });

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_recovery_dialog_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let info = RecoveryInfo {
            snapshot_path: PathBuf::from("/tmp/autosave-001.petunia"),
            project_name: "Test Adventure".to_string(),
            snapshot_time: 12345,
            main_project_path: Some(PathBuf::from("/tmp/test.petunia")),
            is_newer_than_main: true,
        };

        ctx.run_ui(egui::RawInput::default(), |_ui| {
            let _ = draw(&ctx, &mut state, &info);
        })
        .textures_delta
        .clear();
    }
}
