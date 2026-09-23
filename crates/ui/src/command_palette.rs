//! Command Palette (P3D-081) — Busca e execução rápida de comandos com fuzzy search,
//! suporte a categorias visuais, atalhos dinâmicos e validação contextual de execução.

#[cfg(not(feature = "palette-autocomplete"))]
use egui::TextEdit;
use egui::{Align2, Color32, FontId, Id, Key, Rect, Sense, Ui, vec2};
use petunia_core::AppState;
use petunia_core::command::{CommandCategory, CommandPaletteItem};

use crate::tokens;

pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    if !state.ui.show_command_palette {
        return;
    }

    let mut close = false;
    let mut command_to_execute = None;

    // Esc fecha a paleta
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.ui.show_command_palette = false;
        state.mark_dirty();
        return;
    }

    let search_input_id = Id::new("command_palette_search_input");

    let screen_rect = ctx.viewport_rect();
    let palette_width = 560.0_f32.min((screen_rect.width() - 32.0).max(200.0));
    // Wave 5 (§9.4): altura máxima presa à viewport útil.
    let max_h = (screen_rect.height() - 64.0).clamp(200.0, 600.0);

    egui::Window::new("Command Palette")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_TOP, vec2(0.0, 72.0))
        .fixed_size(vec2(palette_width, 0.0))
        .max_size(vec2(palette_width, max_h))
        .frame(
            egui::Frame::new()
                .fill(tokens::BG_PANEL)
                .stroke(tokens::stroke_focus(ctx))
                .corner_radius(tokens::RADIUS_CONTAINER)
                .inner_margin(egui::Margin::symmetric(10, 10))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(180),
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(palette_width - 20.0);

            // 1. Campo de busca
            let hint = state.t("command_palette.placeholder");
            ui.horizontal(|ui| {
                let (search_rect, _) =
                    ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                crate::icon_registry::IconRegistry::paint(
                    ui.ctx(),
                    ui.painter(),
                    &crate::icon_registry::PetuniaIcon::Search,
                    search_rect,
                    crate::tokens::TEXT_MUTED,
                );
                #[cfg(feature = "palette-autocomplete")]
                let response = {
                    let ids: Vec<String> = state
                        .commands
                        .all_metadata()
                        .iter()
                        .map(|meta| meta.id.clone())
                        .collect();
                    crate::palette_complete::command_search(
                        ui,
                        search_input_id,
                        &mut state.ui.command_palette_query,
                        &ids,
                        &hint,
                    )
                };
                #[cfg(not(feature = "palette-autocomplete"))]
                let response = ui.add(
                    TextEdit::singleline(&mut state.ui.command_palette_query)
                        .id(search_input_id)
                        .hint_text(hint)
                        .desired_width(ui.available_width())
                        .font(FontId::proportional(14.0))
                        .margin(egui::Margin::symmetric(6, 4)),
                );

                response.request_focus();
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // 2. Query nos comandos registrados
            let items: Vec<CommandPaletteItem> =
                state.commands.query(&state.ui.command_palette_query, state);

            let count = items.len();
            if count == 0 {
                state.ui.command_palette_selected_index = 0;
            } else if state.ui.command_palette_selected_index >= count {
                state.ui.command_palette_selected_index = count - 1;
            }

            // Teclas de navegação: Up / Down / Enter
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && count > 0 {
                state.ui.command_palette_selected_index =
                    (state.ui.command_palette_selected_index + 1).min(count - 1);
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowUp))
                && state.ui.command_palette_selected_index > 0
            {
                state.ui.command_palette_selected_index -= 1;
            }
            if ctx.input(|i| i.key_pressed(Key::Enter))
                && count > 0
                && state.ui.command_palette_selected_index < count
            {
                let item = &items[state.ui.command_palette_selected_index];
                if item.is_available {
                    command_to_execute = Some(item.id.clone());
                    close = true;
                } else if let Some(reason) = item.disabled_reason {
                    state.set_status(format!("{}: {}", item.label, reason));
                }
            }

            // 3. Lista de resultados
            if items.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(state.t("command_palette.no_results"))
                            .color(tokens::TEXT_MUTED)
                            .size(12.5),
                    );
                    ui.add_space(16.0);
                });
            } else {
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 2.0);

                        for (idx, item) in items.iter().enumerate() {
                            let is_selected = idx == state.ui.command_palette_selected_index;
                            let clicked = render_palette_item(ui, item, is_selected);
                            if clicked {
                                if item.is_available {
                                    command_to_execute = Some(item.id.clone());
                                    close = true;
                                } else if let Some(reason) = item.disabled_reason {
                                    state.set_status(format!("{}: {}", item.label, reason));
                                }
                            }
                        }
                    });
            }

            // 4. Rodapé sutil com instruções de atalhos
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Up/Down navigate · Enter execute · Esc close")
                        .size(10.0)
                        .color(tokens::TEXT_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} commands", count))
                            .size(10.0)
                            .color(tokens::TEXT_MUTED),
                    );
                });
            });
        });

    if let Some(cmd_id) = command_to_execute
        && let Err(e) = state.dispatch_command(&cmd_id)
    {
        state.set_status(format!("Command error: {}", e));
    }

    if close {
        state.ui.show_command_palette = false;
        state.mark_dirty();
    }
}

fn category_color(cat: CommandCategory) -> (Color32, Color32) {
    match cat {
        CommandCategory::Model => (
            Color32::from_rgb(0x35, 0x22, 0x14),
            Color32::from_rgb(0xf3, 0x9c, 0x12),
        ),
        CommandCategory::Edit => (
            Color32::from_rgb(0x14, 0x27, 0x38),
            Color32::from_rgb(0x34, 0x98, 0xdb),
        ),
        CommandCategory::File => (
            Color32::from_rgb(0x18, 0x30, 0x20),
            Color32::from_rgb(0x2e, 0xcc, 0x71),
        ),
        CommandCategory::View => (
            Color32::from_rgb(0x28, 0x18, 0x38),
            Color32::from_rgb(0x9b, 0x59, 0xb6),
        ),
        CommandCategory::Tools => (
            Color32::from_rgb(0x33, 0x28, 0x14),
            Color32::from_rgb(0xe6, 0x7e, 0x22),
        ),
        CommandCategory::Window => (
            Color32::from_rgb(0x22, 0x28, 0x32),
            Color32::from_rgb(0x7f, 0x8c, 0x8d),
        ),
        CommandCategory::Help => (
            Color32::from_rgb(0x18, 0x2e, 0x35),
            Color32::from_rgb(0x1a, 0xbc, 0x9c),
        ),
        CommandCategory::Select => (
            Color32::from_rgb(0x28, 0x25, 0x14),
            Color32::from_rgb(0xf1, 0xc4, 0x0f),
        ),
    }
}

fn render_palette_item(ui: &mut Ui, item: &CommandPaletteItem, is_selected: bool) -> bool {
    let desired_size = vec2(ui.available_width(), 34.0);
    let (rect, response) = ui.allocate_exact_size(
        desired_size,
        if item.is_available {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    let is_hovered = response.hovered();

    if ui.is_rect_visible(rect) {
        let painter = ui.painter().with_clip_rect(rect);

        // Fundo
        let bg = if is_selected {
            tokens::bg_surface_active_for(ui.ctx())
        } else if is_hovered {
            tokens::BG_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };

        if bg != Color32::TRANSPARENT {
            painter.rect_filled(rect, tokens::RADIUS_CONTROL, bg);
        }

        let (text_color, desc_color) = if !item.is_available {
            (tokens::TEXT_MUTED, tokens::TEXT_MUTED.gamma_multiply(0.7))
        } else if is_selected {
            (
                tokens::TEXT_ACTIVE,
                tokens::TEXT_ACTIVE.gamma_multiply(0.85),
            )
        } else {
            (tokens::TEXT_PRIMARY, tokens::TEXT_SECONDARY)
        };

        // 1. Badge da Categoria
        let (cat_bg, cat_fg) = category_color(item.category);
        let cat_str = item.category.as_str();
        let cat_font = FontId::proportional(9.5);
        let cat_width = ui.fonts_mut(|f| {
            f.layout_no_wrap(cat_str.to_string(), cat_font.clone(), cat_fg)
                .size()
                .x
        }) + 10.0;
        let badge_rect = Rect::from_min_size(
            egui::pos2(rect.min.x + 8.0, rect.min.y + (rect.height() - 16.0) * 0.5),
            vec2(cat_width, 16.0),
        );
        painter.rect_filled(badge_rect, tokens::RADIUS_SMALL, cat_bg);
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            cat_str,
            cat_font,
            cat_fg,
        );

        // 2. Rótulo principal e descrição
        let text_x = badge_rect.max.x + 10.0;
        let label_pos = egui::pos2(text_x, rect.min.y + 4.0);
        painter.text(
            label_pos,
            Align2::LEFT_TOP,
            &item.label,
            FontId::proportional(12.0),
            text_color,
        );

        let desc_pos = egui::pos2(text_x, rect.min.y + 18.0);
        painter.text(
            desc_pos,
            Align2::LEFT_TOP,
            &item.description,
            FontId::proportional(10.0),
            desc_color,
        );

        // 3. Atalho ou Razão de Desabilitação à direita
        let right_x = rect.max.x - 8.0;

        if let Some(sc) = &item.shortcut {
            let sc_font = FontId::monospace(10.5);
            let sc_color = if is_selected {
                tokens::TEXT_ACTIVE
            } else {
                tokens::TEXT_MUTED
            };
            painter.text(
                egui::pos2(right_x, rect.center().y),
                Align2::RIGHT_CENTER,
                sc,
                sc_font,
                sc_color,
            );
        } else if let Some(reason) = item.disabled_reason {
            let reason_font = FontId::proportional(10.0);
            painter.text(
                egui::pos2(right_x, rect.center().y),
                Align2::RIGHT_CENTER,
                format!("({})", reason),
                reason_font,
                tokens::TEXT_MUTED,
            );
        }
    }

    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.ui.show_command_palette = true;

        ctx.run_ui(egui::RawInput::default(), |_ui| {
            draw(&ctx, &mut state);
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_command_palette_query_filtering() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.ui.show_command_palette = true;
        state.ui.command_palette_query = "extrude".to_string();

        ctx.run_ui(egui::RawInput::default(), |_ui| {
            draw(&ctx, &mut state);
        })
        .textures_delta
        .clear();
    }
}
