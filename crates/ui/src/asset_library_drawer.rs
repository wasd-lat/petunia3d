//! Gaveta / Modal da Biblioteca de Assets do Projeto (`Asset Library Drawer`).
//! Exibe todos os modelos/assets criados e salvos neste projeto,
//! permitindo instanciar na cena (na posição do 3D Cursor), duplicar, renomear,
//! e estabelece a clara distinção conceitual:
//! - **Salvar Asset**: Salva/registra o modelo ativo dentro da biblioteca do projeto;
//! - **Salvar Projeto**: Salva o arquivo completo `.petunia` (cena, todos os assets, materiais, câmera).

use egui::{Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui, Window, vec2};
use petunia_core::{AppState, DeleteAssetCmd, DuplicateAssetCmd};

use crate::icon_registry::PetuniaIcon;
use crate::tokens;
use crate::widgets;

/// Renderiza o modal / gaveta da Biblioteca de Assets quando `state.ui.show_asset_library` estiver ativo.
pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    if !state.ui.show_asset_library {
        return;
    }

    let mut open = state.ui.show_asset_library;
    // Wave 5 (§9.4): mínimo e máximo nunca excedem a viewport útil.
    let screen_rect = ctx.viewport_rect();
    let (default_size, min_size, max_size) = crate::regions::modal_sizes(
        screen_rect,
        vec2(screen_rect.width() * 0.65, screen_rect.height() * 0.60),
        vec2(380.0, 280.0),
        vec2(780.0, 640.0),
    );

    Window::new(RichText::new("Project Asset Library").strong().size(14.0))
        .open(&mut open)
        .default_size(default_size)
        .min_size(min_size)
        .max_size(max_size)
        .resizable(true)
        .collapsible(false)
        .frame(
            egui::Frame::window(&ctx.style_of(ctx.theme()))
                .fill(tokens::BG_PANEL)
                .stroke(tokens::stroke_border())
                .inner_margin(egui::Margin::same(12)),
        )
        .show(ctx, |ui| {
            draw_contents(ui, state);
        });

    state.ui.show_asset_library = open;
}

fn draw_contents(ui: &mut Ui, state: &mut AppState) {
    // 1. Barra de Ações Superior (Distinção clara: Salvar Asset vs Salvar Projeto)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Project internal library")
                .color(tokens::TEXT_SECONDARY)
                .size(11.5),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Botão: Salvar Projeto Completo
            let save_proj_btn = egui::Button::new(
                RichText::new("Save Project (.petunia)")
                    .size(11.0)
                    .color(tokens::TEXT_PRIMARY),
            )
            .fill(tokens::BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, tokens::BORDER_SUBTLE))
            .corner_radius(tokens::RADIUS_CONTROL);

            if ui
                .add(save_proj_btn)
                .on_hover_text(
                    "Save project with all assets, camera, palette and state to disk (.petunia)",
                )
                .clicked()
            {
                crate::save_project_dialog(state, false);
            }

            // Botão: Salvar Ativo como Asset
            let save_asset_btn = egui::Button::new(
                RichText::new("Save Active Model as Asset")
                    .size(11.0)
                    .color(tokens::TEXT_ACTIVE),
            )
            .fill(tokens::bg_surface_active_for(ui.ctx()))
            .corner_radius(tokens::RADIUS_CONTROL);

            if ui
                .add(save_asset_btn)
                .on_hover_text("Save a copy of the active 3D model into this project's library")
                .clicked()
            {
                state.save_active_as_asset();
            }
        });
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // 2. Campo de busca / filtro rápido
    let filter_id = egui::Id::new("asset_library_filter_text");
    let mut search_query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(filter_id).unwrap_or_default());

    ui.horizontal(|ui| {
        let resp = widgets::petunia_search_box(ui, &mut search_query, "Filter assets...");
        if resp.changed() {
            ui.ctx()
                .data_mut(|d| d.insert_temp(filter_id, search_query.clone()));
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("Total: {} asset(s)", state.project.assets.len()))
                    .size(11.0)
                    .color(tokens::TEXT_SECONDARY),
            );
        });
    });

    ui.add_space(8.0);

    // 3. Grade / Lista de Cards de Assets
    let total_assets = state.project.assets.len();
    let mut to_instantiate: Option<usize> = None;
    let mut to_activate: Option<usize> = None;
    let mut to_duplicate: Option<usize> = None;
    let mut to_remove: Option<usize> = None;

    ScrollArea::vertical()
        .auto_shrink([true, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);

                for i in 0..total_assets {
                    let asset = &state.project.assets[i];

                    // Aplica filtro de busca
                    if !search_query.is_empty()
                        && !asset
                            .name
                            .to_lowercase()
                            .contains(&search_query.to_lowercase())
                    {
                        continue;
                    }

                    let is_active = state.project.active == i;
                    let (card_bg, border_color) = if is_active {
                        (tokens::bg_surface_active_for(ui.ctx()), tokens::ACCENT_BLUE)
                    } else {
                        (tokens::BG_SURFACE, tokens::BORDER_SUBTLE)
                    };

                    egui::Frame::new()
                        .fill(card_bg)
                        .stroke(Stroke::new(1.0_f32, border_color))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_width(210.0);
                            ui.set_min_height(140.0);

                            // Cabeçalho do Card: Nome e Tag Ativo
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&asset.name)
                                        .strong()
                                        .size(12.5)
                                        .color(tokens::TEXT_PRIMARY),
                                );
                                if is_active {
                                    ui.label(
                                        RichText::new("• Active")
                                            .size(10.0)
                                            .color(tokens::ACCENT_BLUE),
                                    );
                                }
                            });

                            // Indicador de Cor Base / Miniatura de Material
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let c = Color32::from_rgb(
                                    (asset.base_color[0] * 255.0) as u8,
                                    (asset.base_color[1] * 255.0) as u8,
                                    (asset.base_color[2] * 255.0) as u8,
                                );
                                let (chip_rect, _) =
                                    ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                                ui.painter().rect_filled(chip_rect, 3.0, c);
                                ui.painter().rect_stroke(
                                    chip_rect,
                                    3.0,
                                    Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
                                    egui::StrokeKind::Inside,
                                );

                                ui.label(
                                    RichText::new(format!(
                                        "{} verts · {} faces",
                                        asset.mesh.vert_count(),
                                        asset.mesh.face_count()
                                    ))
                                    .size(10.5)
                                    .color(tokens::TEXT_SECONDARY),
                                );
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Botões de Ação do Card
                            ui.horizontal(|ui| {
                                if widgets::petunia_action_button(
                                    ui,
                                    Some(PetuniaIcon::Cursor3D),
                                    "Instantiate",
                                    false,
                                )
                                .on_hover_text("Spawn an instance at 3D Cursor position")
                                .clicked()
                                {
                                    to_instantiate = Some(i);
                                }

                                if !is_active
                                    && widgets::petunia_action_button(
                                        ui,
                                        Some(PetuniaIcon::ModeEdit),
                                        "Edit",
                                        false,
                                    )
                                    .on_hover_text(
                                        "Set this asset as active in viewport for editing",
                                    )
                                    .clicked()
                                {
                                    to_activate = Some(i);
                                }
                            });

                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                if widgets::petunia_action_button(
                                    ui,
                                    Some(PetuniaIcon::Duplicate),
                                    "Duplicate",
                                    false,
                                )
                                .on_hover_text("Duplicate this asset in the library")
                                .clicked()
                                {
                                    to_duplicate = Some(i);
                                }

                                if total_assets > 1
                                    && widgets::petunia_action_button(
                                        ui,
                                        Some(PetuniaIcon::Trash),
                                        "Delete",
                                        true,
                                    )
                                    .on_hover_text("Remove this asset from project library")
                                    .clicked()
                                {
                                    to_remove = Some(i);
                                }
                            });
                        });
                }
            });
        });

    // 4. Aplicação das ações pendentes fora dos borrows
    if let Some(idx) = to_instantiate {
        state.instantiate_asset_at_cursor(idx);
    }
    if let Some(idx) = to_activate {
        state.project.active = idx;
        state.sync_selection();
        state.emit_mesh_changed();
        state.mark_dirty();
    }
    if let Some(idx) = to_duplicate {
        let _ = state.dispatch(&DuplicateAssetCmd {
            asset_index: Some(idx),
        });
    }
    if let Some(idx) = to_remove {
        let _ = state.dispatch(&DeleteAssetCmd {
            asset_index: Some(idx),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_library_drawer_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.ui.show_asset_library = true;

        ctx.run_ui(egui::RawInput::default(), |_ui| {
            draw(&ctx, &mut state);
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_save_active_as_asset_and_instantiate() {
        let mut state = AppState::new("en");
        let initial_count = state.project.assets.len();
        assert_eq!(initial_count, 1);

        // Salvar ativo como novo asset
        assert!(state.save_active_as_asset());
        assert_eq!(state.project.assets.len(), 2);

        // Instanciar asset na cena
        assert!(state.instantiate_asset_at_cursor(0));
        assert_eq!(state.project.assets.len(), 3);
    }
}
