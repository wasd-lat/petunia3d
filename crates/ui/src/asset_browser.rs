//! Painel Lateral do Navegador de Assets do Petunia3D (`Asset Browser`).
//! Fornece visualização dedicada, busca, filtragem por categoria, instanciação
//! na coordenada do 3D Cursor e gerenciamento de modelos salvos no projeto.

use egui::{Align, Color32, Layout, Rect, RichText, ScrollArea, Ui, vec2};
use petunia_core::{AppState, DeleteAssetCmd, DuplicateAssetCmd};

use crate::tokens;
use crate::widgets;

/// Reserva vertical para separador + rodapé abaixo da lista de cards.
const FOOTER_RESERVE: f32 = 64.0;

/// Renderiza o painel lateral retrátil do Asset Browser.
pub fn draw(ui: &mut Ui, state: &mut AppState) {
    if !state.ui.show_asset_browser {
        return;
    }

    let min_w = 220.0;
    let max_w = 340.0;

    let browser_resp = egui::Panel::left("asset_browser_panel")
        .default_size(260.0)
        .size_range(min_w..=max_w)
        .resizable(true)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_panel(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(8, 6)),
        )
        .show(ui, |ui| {
            draw_header(ui, state);
            ui.separator();
            draw_categories(ui, state);
            ui.separator();
            draw_asset_cards(ui, state);
            ui.separator();
            draw_footer_actions(ui, state);
        });
    crate::regions::record(
        ui.ctx(),
        crate::regions::RegionSlot::AssetBrowser,
        browser_resp.response.rect,
    );
}

fn draw_header(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Asset Browser")
                .strong()
                .size(12.5)
                .color(tokens::TEXT_PRIMARY),
        );
        let count = state.project.assets.len();
        ui.label(
            RichText::new(format!("({count})"))
                .size(11.0)
                .color(tokens::TEXT_MUTED),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .small_button("×")
                .on_hover_text("Fechar Asset Browser")
                .clicked()
            {
                state.ui.show_asset_browser = false;
                state.mark_dirty();
            }
        });
    });

    ui.add_space(4.0);

    // Campo de busca padronizado com PetuniaSearchBox
    let filter_id = egui::Id::new("asset_browser_search_query");
    let mut query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(filter_id).unwrap_or_default());

    if widgets::petunia_search_box(ui, &mut query, "Buscar assets...").changed() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(filter_id, query.clone()));
        state.mark_dirty();
    }

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Miniaturas:")
                .size(10.0)
                .color(tokens::TEXT_MUTED),
        );
        ui.add(
            egui::Slider::new(&mut state.ui.asset_thumbnail_size, 32.0..=128.0)
                .show_value(false)
                .step_by(16.0),
        );
    });
}

fn draw_categories(ui: &mut Ui, _state: &mut AppState) {
    let cat_id = egui::Id::new("asset_browser_active_category");
    let active_cat = ui.ctx().data_mut(|d| {
        d.get_temp::<String>(cat_id)
            .unwrap_or_else(|| "all".to_string())
    });

    let categories = [
        ("all", "Todos"),
        ("props", "Props"),
        ("chars", "Personagens"),
        ("env", "Cenário"),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for (id, label) in categories {
            let is_sel = active_cat == id;
            let (bg, fg) = if is_sel {
                (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
            } else {
                (tokens::BG_SURFACE, tokens::TEXT_SECONDARY)
            };

            let btn = egui::Button::new(RichText::new(label).size(10.5).color(fg))
                .fill(bg)
                .corner_radius(tokens::RADIUS_CONTROL);

            if ui.add(btn).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(cat_id, id.to_string()));
            }
        }
    });
}

fn draw_asset_cards(ui: &mut Ui, state: &mut AppState) {
    let filter_id = egui::Id::new("asset_browser_search_query");
    let query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(filter_id).unwrap_or_default())
        .trim()
        .to_lowercase();

    let cat_id = egui::Id::new("asset_browser_active_category");
    let active_cat = ui.ctx().data_mut(|d| {
        d.get_temp::<String>(cat_id)
            .unwrap_or_else(|| "all".to_string())
    });

    let thumb_sz = state.ui.asset_thumbnail_size.clamp(32.0, 128.0);

    let n = state.project.assets.len();
    if n == 0 {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Nenhum asset no projeto")
                    .color(tokens::TEXT_MUTED)
                    .size(11.0),
            );
            ui.label(
                RichText::new("Salve o modelo ativo abaixo para catalogar.")
                    .color(tokens::TEXT_MUTED)
                    .size(10.0),
            );
        });
        return;
    }

    ScrollArea::vertical()
        .id_salt("asset_browser_cards_scroll")
        .auto_shrink([true, false])
        // Altura limitada com reserva do rodapé: sem isto a área de rolagem
        // ocupa todo o restante do painel e o rodapé estoura o retângulo,
        // invadindo a status bar (invariante `status_overlaps`).
        .max_height((ui.available_height() - FOOTER_RESERVE).max(120.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 6.0);

            let mut to_instantiate: Option<usize> = None;
            let mut to_activate: Option<usize> = None;
            let mut to_duplicate: Option<usize> = None;
            let mut to_delete: Option<usize> = None;

            for i in 0..n {
                let a = &state.project.assets[i];

                // Filtragem por categoria
                let cat_match = match active_cat.as_str() {
                    "all" => true,
                    "props" => {
                        a.tags.iter().any(|t| t == "props")
                            || a.collection
                                .as_deref()
                                .map(|c| c.eq_ignore_ascii_case("props"))
                                .unwrap_or(false)
                    }
                    "chars" => {
                        a.tags.iter().any(|t| t == "chars" || t == "character")
                            || a.collection
                                .as_deref()
                                .map(|c| c.eq_ignore_ascii_case("chars") || c.eq_ignore_ascii_case("characters"))
                                .unwrap_or(false)
                    }
                    "env" => {
                        a.tags.iter().any(|t| t == "env" || t == "environment")
                            || a.collection
                                .as_deref()
                                .map(|c| c.eq_ignore_ascii_case("env") || c.eq_ignore_ascii_case("environment"))
                                .unwrap_or(false)
                    }
                    other => {
                        a.tags.iter().any(|t| t == other)
                            || a.collection
                                .as_deref()
                                .map(|c| c.eq_ignore_ascii_case(other))
                                .unwrap_or(false)
                    }
                };
                if !cat_match {
                    continue;
                }

                // Filtragem por busca (nome, coleção, tags)
                let search_match = query.is_empty()
                    || a.name.to_lowercase().contains(&query)
                    || a.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    || a.collection
                        .as_deref()
                        .map(|c| c.to_lowercase().contains(&query))
                        .unwrap_or(false);
                if !search_match {
                    continue;
                }

                let is_active = state.project.active == i;
                let tris = a.mesh.tri_count();
                let verts = a.mesh.vert_count();
                let col = a.base_color;
                let name = a.name.clone();
                let asset_id = a.id;

                let card_bg = if is_active {
                    tokens::bg_surface_active_for(ui.ctx())
                } else {
                    tokens::BG_SURFACE
                };

                let card_stroke = if is_active {
                    egui::Stroke::new(1.0_f32, tokens::ACCENT_BORDER)
                } else {
                    tokens::stroke_border()
                };

                egui::Frame::new()
                    .fill(card_bg)
                    .stroke(card_stroke)
                    .corner_radius(tokens::RADIUS_CONTROL)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        if thumb_sz > 64.0 {
                            // Layout Vertical: Thumbnail grande em cima, dados embaixo
                            let (thumb_rect, thumb_resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), thumb_sz * 0.7),
                                egui::Sense::click_and_drag(),
                            );
                            if thumb_resp.drag_started() || thumb_resp.dragged() {
                                egui::DragAndDrop::set_payload(ui.ctx(), asset_id);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            }
                            if thumb_resp.double_clicked() {
                                to_instantiate = Some(i);
                            } else if thumb_resp.clicked() {
                                to_activate = Some(i);
                            }

                            // Renderiza thumbnail estilizado com base_color e silhueta
                            let p = ui.painter();
                            p.rect_filled(thumb_rect, tokens::RADIUS_SMALL, tokens::BG_INPUT);
                            let c_fill = Color32::from_rgb(
                                (col[0] * 255.0) as u8,
                                (col[1] * 255.0) as u8,
                                (col[2] * 255.0) as u8,
                            );
                            let center = thumb_rect.center();
                            let box_radius = (thumb_sz * 0.22).min(thumb_rect.height() * 0.4);
                            p.rect_filled(
                                Rect::from_center_size(center, vec2(box_radius * 2.0, box_radius * 2.0)),
                                tokens::RADIUS_SMALL,
                                c_fill.gamma_multiply(0.85),
                            );
                            p.rect_stroke(
                                Rect::from_center_size(center, vec2(box_radius * 2.0, box_radius * 2.0)),
                                tokens::RADIUS_SMALL,
                                egui::Stroke::new(1.5_f32, Color32::WHITE.gamma_multiply(0.6)),
                                egui::StrokeKind::Inside,
                            );

                            ui.add_space(3.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&name).strong().size(11.5).color(tokens::TEXT_PRIMARY));
                                if is_active {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new("• ATIVO")
                                                .size(9.0)
                                                .strong()
                                                .color(tokens::ACCENT_BLUE),
                                        );
                                    });
                                }
                            });
                        } else {
                            // Layout Horizontal: Linha 1 com miniatura integrada
                            ui.horizontal(|ui| {
                                let (thumb_rect, thumb_resp) = ui.allocate_exact_size(
                                    vec2(thumb_sz * 0.5, thumb_sz * 0.5),
                                    egui::Sense::click_and_drag(),
                                );
                                if thumb_resp.drag_started() || thumb_resp.dragged() {
                                    egui::DragAndDrop::set_payload(ui.ctx(), asset_id);
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                }
                                if thumb_resp.double_clicked() {
                                    to_instantiate = Some(i);
                                } else if thumb_resp.clicked() {
                                    to_activate = Some(i);
                                }

                                let c_fill = Color32::from_rgb(
                                    (col[0] * 255.0) as u8,
                                    (col[1] * 255.0) as u8,
                                    (col[2] * 255.0) as u8,
                                );
                                ui.painter().rect_filled(thumb_rect, tokens::RADIUS_SMALL, c_fill);
                                ui.painter().rect_stroke(
                                    thumb_rect,
                                    tokens::RADIUS_SMALL,
                                    egui::Stroke::new(1.0_f32, Color32::WHITE.gamma_multiply(0.4)),
                                    egui::StrokeKind::Inside,
                                );

                                ui.label(RichText::new(&name).strong().size(11.5).color(tokens::TEXT_PRIMARY));

                                if is_active {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new("• ATIVO")
                                                .size(9.0)
                                                .strong()
                                                .color(tokens::ACCENT_BLUE),
                                        );
                                    });
                                }
                            });
                        }

                        // Linha 2: Métricas de Geometria
                        ui.label(
                            RichText::new(format!("{tris} tris | {verts} verts"))
                                .size(10.0)
                                .color(tokens::TEXT_MUTED),
                        );

                        ui.add_space(2.0);

                        // Linha 3: Ações Rápidas do Card
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(3.0, 0.0);

                            let inst_btn = egui::Button::new(
                                RichText::new("Instantiate")
                                    .size(10.0)
                                    .color(tokens::TEXT_ACTIVE),
                            )
                            .fill(tokens::bg_surface_active_for(ui.ctx()))
                            .corner_radius(tokens::RADIUS_SMALL);

                            if ui
                                .add(inst_btn)
                                .on_hover_text("Cria uma nova instância deste asset nas coordenadas do 3D Cursor")
                                .clicked()
                            {
                                to_instantiate = Some(i);
                            }

                            if !is_active {
                                let act_btn = egui::Button::new(
                                    RichText::new("Activate")
                                        .size(10.0)
                                        .color(tokens::TEXT_PRIMARY),
                                )
                                .fill(tokens::BG_PANEL)
                                .corner_radius(tokens::RADIUS_SMALL);

                                if ui
                                    .add(act_btn)
                                    .on_hover_text("Torna este asset o modelo ativo para edição")
                                    .clicked()
                                {
                                    to_activate = Some(i);
                                }
                            }

                            let dup_btn = egui::Button::new(
                                RichText::new("Duplicate")
                                    .size(10.0)
                                    .color(tokens::TEXT_SECONDARY),
                            )
                            .fill(tokens::BG_PANEL)
                            .corner_radius(tokens::RADIUS_SMALL);

                            if ui
                                .add(dup_btn)
                                .on_hover_text("Duplicar este asset")
                                .clicked()
                            {
                                to_duplicate = Some(i);
                            }

                            if n > 1 {
                                let del_btn = egui::Button::new(
                                    RichText::new("Delete")
                                        .size(10.0)
                                        .color(Color32::from_rgb(255, 100, 100)),
                                )
                                .fill(tokens::BG_PANEL)
                                .corner_radius(tokens::RADIUS_SMALL);

                                if ui
                                    .add(del_btn)
                                    .on_hover_text("Remover asset do projeto")
                                    .clicked()
                                {
                                    to_delete = Some(i);
                                }
                            }
                        });
                    });
            }

            if let Some(idx) = to_instantiate {
                state.instantiate_asset_at_cursor(idx);
            }
            if let Some(idx) = to_activate
                && idx < state.project.assets.len() {
                    state.project.active = idx;
                    state.sync_selection();
                    state.mark_dirty();
                }
            if let Some(idx) = to_duplicate {
                let _ = state.dispatch(&DuplicateAssetCmd {
                    asset_index: Some(idx),
                });
            }
            if let Some(idx) = to_delete {
                let _ = state.dispatch(&DeleteAssetCmd {
                    asset_index: Some(idx),
                });
            }
        });
}

fn draw_footer_actions(ui: &mut Ui, state: &mut AppState) {
    ui.vertical_centered_justified(|ui| {
        let save_active_btn = egui::Button::new(
            RichText::new("Save Active Model as Asset")
                .size(11.0)
                .color(tokens::TEXT_ACTIVE),
        )
        .fill(tokens::bg_surface_active_for(ui.ctx()))
        .corner_radius(tokens::RADIUS_CONTROL);

        if ui
            .add(save_active_btn)
            .on_hover_text("Salva o modelo atualmente selecionado como asset permanente do projeto")
            .clicked()
        {
            state.save_active_as_asset();
        }
    });
}
