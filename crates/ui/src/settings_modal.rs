//! Modal central de Preferências e Configurações (`Settings Modal`).
//! Fornece abas para:
//! 1. **Aparência**: Seletor de Temas (Petunia Dark, Light, Capuccino, Tokyo Nights e customizados) com visualizador de tokens de cores;
//! 2. **Ícones**: Seletor de Pacotes de Ícones (Petunia, Phosphor, Tabler, Iconoir, Lucide e customizados) com grade de pré-visualização;
//! 3. **Idioma**: Seletor i18n (pt-BR, en-US) e suporte a novos arquivos TOML de localização;
//! 4. **Atalhos de Teclado (Keymap)**: Seletor dos 8 perfis canônicos, busca de comandos e detecção automática de conflitos.

use egui::{
    Align, Color32, CornerRadius, FontId, Layout, RichText, ScrollArea, Stroke, Ui, Window, vec2,
};
use petunia_config::{Keybinds, ThemeRegistry, ThemeToken};
use petunia_core::AppState;

use crate::adapters::form::{PetuniaForm, PetuniaFormSession, PetuniaValidationReport};
use crate::adapters::taffy_layout::{self, PetuniaGap, PetuniaJustify, PetuniaResponsiveLayout};
use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::tokens;
use crate::widgets;
use petunia_config::text_id;

/// Formulário das preferências (§48).
///
/// Rótulo de 96px e controle nunca menor que 160px: abaixo disso o campo
/// empilha em vez de espremer o controle. Os dois números são a **única**
/// política de arranjo de campo da tela — não há breakpoint por aba.
static SETTINGS_FORM: PetuniaForm = PetuniaForm::new(96.0, 160.0);

/// Vão da esteira de abas (entre e dentro das linhas).
const TAB_GAP: f32 = 6.0;

/// Esteira das abas de configuração.
fn tab_strip() -> PetuniaResponsiveLayout {
    PetuniaResponsiveLayout::wrap_row().with_gap(PetuniaGap::uniform(TAB_GAP))
}

/// Renderiza a janela modal de preferências quando `state.ui.show_settings` for verdadeiro.
pub fn draw(ctx: &egui::Context, state: &mut AppState) {
    if !state.ui.show_settings {
        return;
    }

    let mut open = state.ui.show_settings;
    // Wave 5 (§9.4): mínimo e máximo nunca excedem a viewport útil.
    let screen_rect = ctx.viewport_rect();
    let (default_size, min_size, max_size) = crate::regions::modal_sizes(
        screen_rect,
        vec2(screen_rect.width() * 0.70, screen_rect.height() * 0.70),
        vec2(460.0, 360.0),
        vec2(840.0, 700.0),
    );

    Window::new(RichText::new(state.t("settings.title")).strong().size(14.0))
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
            draw_settings_content(ctx, ui, state);
        });

    state.ui.show_settings = open;
}

fn draw_settings_content(ctx: &egui::Context, ui: &mut Ui, state: &mut AppState) {
    // 1. Barra de Abas de Configuração (§48): esteira que quebra sozinha, sem
    // `add_space` somado à mão. Abas novas usam rótulo traduzido sem emoji
    // (Wave 6/7 convergem as antigas).
    {
        let interface_label = state.t_id(text_id::SETTINGS_INTERFACE);
        let import_export_label = state.t_id(text_id::SETTINGS_IMPORT_EXPORT);
        let appearance_label = state.t("settings.appearance");
        let icons_label = state.t("settings.icons");
        let language_label = state.t("settings.language");
        let keymap_label = state.t("settings.keymap");
        let tabs = [
            (
                "appearance",
                appearance_label.as_str(),
                "Temas visuais, paletas e tokens semânticos",
            ),
            (
                "icons",
                icons_label.as_str(),
                "Pacotes de ícones e símbolos da interface",
            ),
            (
                "language",
                language_label.as_str(),
                "Localização, traduções e arquivos TOML",
            ),
            (
                "keymap",
                keymap_label.as_str(),
                "Perfis de keymap e detecção de conflitos",
            ),
            ("interface", interface_label.as_str(), ""),
            ("import_export", import_export_label.as_str(), ""),
        ];

        taffy_layout::responsive(
            ui,
            "settings-tabs",
            tab_strip(),
            tabs.len(),
            |index, ui| {
                let (tab_id, label, hint) = tabs[index];
                let is_selected = state.ui.settings_tab == tab_id;
                let (bg, fg) = if is_selected {
                    (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
                } else {
                    (tokens::BG_SURFACE, tokens::TEXT_SECONDARY)
                };

                let btn = egui::Button::new(RichText::new(label).size(12.0).color(fg))
                    .fill(bg)
                    .corner_radius(tokens::RADIUS_CONTROL);

                let resp = ui.add(btn);
                let resp = if hint.is_empty() {
                    resp
                } else {
                    resp.on_hover_text(hint)
                };
                if resp.clicked() {
                    state.ui.settings_tab = tab_id.to_string();
                    state.mark_dirty();
                }
            },
            |_ui| (),
        );
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    // 2. Conteúdo da Aba Ativa
    match state.ui.settings_tab.as_str() {
        "appearance" => draw_appearance_tab(ctx, ui, state),
        "icons" => draw_icons_tab(ui, state),
        "language" => draw_language_tab(ui, state),
        "keymap" => draw_keymap_tab(ui, state),
        "interface" => draw_interface_tab(ui, state),
        "import_export" => draw_import_export_tab(ui, state),
        _ => draw_appearance_tab(ctx, ui, state),
    }
}

/// Restaura o layout do workspace atual aos padrões (Wave 5 — §15.7).
fn reset_workspace_layout(state: &mut AppState) {
    state.ui.right_dock_split = 0.42;
    state.ui.outliner_collapsed = false;
    state.ui.inspector_collapsed = false;
    state.ui.workspace_memory[petunia_core::workspace_index(state.workspace)] =
        petunia_core::WorkspaceUiMemory::default();
    state.mark_dirty();
}

/// Restaura todos os layouts de UI aos padrões (Wave 5 — §15.7).
fn reset_all_layouts(state: &mut AppState) {
    reset_workspace_layout(state);
    state.ui.workspace_memory = Default::default();
    state.ui.right_dock_split = 0.42;
    state.ui.properties_tab = "object".to_string();
    state.ui.uv_show_preview = true;
    state.ui.show_shelf = true;
    state.ui.toolbar_order.clear();
    state.ui.toolbar_hidden.clear();
    state.ui.toolbar_columns = 1;
    state.ui.dock_side = petunia_core::DockSide::Right;
    state.ui.dock_orientation = petunia_core::DockOrientation::Stacked;
    state.ui.density = petunia_core::UiDensity::Comfortable;
    state.ui.scene_split_auto = true;
    state.ui.scene_search_open = false;
    state.ui.scene_filter = petunia_core::SceneFilter::default();
    state.ui.inspector_pinned = None;
    state.ui.inspector_search_open = false;
    state.ui.inspector_search.clear();
    state.mark_dirty();
}

// ------------------------------------------------- Aba: Interface
fn draw_interface_tab(ui: &mut Ui, state: &mut AppState) {
    SETTINGS_FORM.section(ui, &state.t_id(text_id::SETTINGS_INTERFACE), None);

    let mut shelf = state.ui.show_shelf;
    if SETTINGS_FORM.toggle(ui, &state.t_id(text_id::SETTINGS_SHOW_SHELF), &mut shelf) {
        state.ui.show_shelf = shelf;
        state.mark_dirty();
    }

    ui.add_space(4.0);
    SETTINGS_FORM.field(
        ui,
        "settings-density",
        &state.t("settings.density"),
        |ui, width| {
            // O controle recebe a largura do contrato e se limita a ela: em
            // painel estreito as opções descem em vez de vazar do campo.
            ui.set_max_width(width);
            ui.horizontal_wrapped(|ui| {
                for density in petunia_core::UiDensity::all() {
                    if ui
                        .selectable_label(state.ui.density == density, state.t(density.key()))
                        .clicked()
                    {
                        state.ui.density = density;
                        state.mark_dirty();
                    }
                }
            });
        },
    );

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        if ui
            .button(state.t_id(text_id::SETTINGS_RESET_WORKSPACE))
            .clicked()
        {
            reset_workspace_layout(state);
        }
        if ui.button(state.t_id(text_id::SETTINGS_RESET_ALL)).clicked() {
            reset_all_layouts(state);
        }
    });
}

// ------------------------------------------- Aba: Importar / Exportar
fn draw_import_export_tab(ui: &mut Ui, state: &mut AppState) {
    SETTINGS_FORM.section(ui, &state.t_id(text_id::SETTINGS_IMPORT_EXPORT), None);

    let mut glb = state.project.export_gltf;
    if SETTINGS_FORM.toggle(ui, &state.t_id(text_id::SETTINGS_EXPORT_GLB), &mut glb) {
        state.project.export_gltf = glb;
        state.mark_dirty();
    }
    ui.label(
        RichText::new(state.t_id(text_id::SETTINGS_EXPORT_GLB_HINT))
            .size(11.0)
            .color(tokens::TEXT_SECONDARY),
    );
}

// ---------------------------------------------------------------- Aba 1: Aparência e Temas
fn draw_appearance_tab(ctx: &egui::Context, ui: &mut Ui, state: &mut AppState) {
    let registry = ThemeRegistry::global();
    let themes = registry.available();

    SETTINGS_FORM.section(
        ui,
        "Temas do Sistema",
        Some(
            "O Petunia3D suporta temas personalizados em arquivos TOML. Crie uma pasta em 'assets/themes/<nome>/' contendo 'manifest.toml' e 'theme.toml'.",
        ),
    );

    ScrollArea::vertical()
        .auto_shrink([true, false])
        .show(ui, |ui| {
            for manifest in themes {
                let is_active = state.ui.active_theme_id == manifest.id;
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
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&manifest.name)
                                            .strong()
                                            .size(13.0)
                                            .color(tokens::TEXT_PRIMARY),
                                    );
                                    if is_active {
                                        ui.label(
                                            RichText::new("• Ativo")
                                                .size(10.5)
                                                .color(tokens::ACCENT_BLUE),
                                        );
                                    }
                                });
                                if let Some(ref desc) = manifest.description {
                                    ui.label(
                                        RichText::new(desc)
                                            .size(11.0)
                                            .color(tokens::TEXT_SECONDARY),
                                    );
                                }
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !is_active {
                                    if ui
                                        .button(RichText::new("Aplicar Tema").size(11.0))
                                        .clicked()
                                    {
                                        state.ui.active_theme_id = manifest.id.clone();
                                        if let Some(t) = registry.get_theme(&manifest.id) {
                                            tokens::apply_theme_to_egui(t, ctx);
                                        }
                                        state.mark_dirty();
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("✔ Aplicado").color(tokens::ACCENT_BLUE),
                                    );
                                }
                            });
                        });

                        // Amostras de cores do tema
                        if let Some(theme) = registry.get_theme(&manifest.id) {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Tokens:")
                                        .size(10.5)
                                        .color(tokens::TEXT_MUTED),
                                );
                                for (name, token) in [
                                    ("Canvas", ThemeToken::BgCanvas),
                                    ("Header", ThemeToken::BgHeader),
                                    ("Panel", ThemeToken::BgPanel),
                                    ("Surface", ThemeToken::BgSurface),
                                    ("Accent", ThemeToken::AccentBlue),
                                    ("Orange", ThemeToken::AccentOrange),
                                    ("Border", ThemeToken::BorderSubtle),
                                ] {
                                    let col = tokens::rgba_to_color32(
                                        theme.colors.get_token_color(token),
                                    );
                                    let (rect, resp) = ui.allocate_exact_size(
                                        vec2(18.0, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, col);
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        Stroke::new(1.0_f32, Color32::from_gray(80)),
                                        egui::StrokeKind::Inside,
                                    );
                                    resp.on_hover_text(name);
                                }
                            });
                        }
                    });

                ui.add_space(6.0);
            }
        });
}

// ---------------------------------------------------------------- Aba 2: Pacotes de Ícones
fn draw_icons_tab(ui: &mut Ui, state: &mut AppState) {
    let packs = IconRegistry::available_packs();

    SETTINGS_FORM.section(
        ui,
        &state.t("settings.icons_title"),
        Some(&state.t("settings.icons_desc")),
    );

    ScrollArea::vertical()
        .auto_shrink([true, false])
        .show(ui, |ui| {
            for pack in packs {
                // Wave 6 (§11.3–11.4): só pacotes compilados são selecionáveis.
                // Pacotes anunciados mas fora do build mostram o motivo real;
                // pacotes de disco usam a via de plugins (sem botão fictício).
                let compiled = pack.id == "petunia"
                    || crate::icon_provider::GenericPack::from_id(&pack.id)
                        .is_some_and(|p| p.is_enabled());
                let is_active = state.ui.active_icon_pack_id == pack.id;
                let (card_bg, border_color) = if is_active {
                    (tokens::bg_surface_active_for(ui.ctx()), tokens::ACCENT_BLUE)
                } else {
                    (tokens::BG_SURFACE, tokens::BORDER_SUBTLE)
                };
                // Nome/descrição do manifest são dados; o locale pode traduzir
                // por id e o valor do manifest fica de reserva (pacotes de disco).
                let pack_name = pack_text(state, "pack_name", &pack.id, &pack.name);
                let pack_desc = pack
                    .description
                    .as_ref()
                    .map(|d| pack_text(state, "pack_desc", &pack.id, d));

                egui::Frame::new()
                    .fill(card_bg)
                    .stroke(Stroke::new(1.0_f32, border_color))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&pack_name)
                                            .strong()
                                            .size(13.0)
                                            .color(tokens::TEXT_PRIMARY),
                                    );
                                    if is_active {
                                        ui.label(
                                            RichText::new(state.t("settings.pack_active"))
                                                .size(10.5)
                                                .color(tokens::ACCENT_BLUE),
                                        );
                                    }
                                    if !compiled {
                                        ui.label(
                                            RichText::new(state.t("settings.pack_not_compiled"))
                                                .size(10.5)
                                                .color(tokens::TEXT_MUTED),
                                        );
                                    }
                                });
                                if let Some(desc) = &pack_desc {
                                    ui.label(
                                        RichText::new(desc)
                                            .size(11.0)
                                            .color(tokens::TEXT_SECONDARY),
                                    );
                                }
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !is_active && compiled {
                                    if ui
                                        .button(
                                            RichText::new(state.t("settings.pack_use")).size(11.0),
                                        )
                                        .clicked()
                                    {
                                        state.ui.active_icon_pack_id = pack.id.clone();
                                        state.mark_dirty();
                                    }
                                } else if is_active {
                                    ui.label(
                                        RichText::new(state.t("settings.pack_selected"))
                                            .color(tokens::ACCENT_BLUE),
                                    );
                                }
                            });
                        });

                        // Pré-visualização com ícones UTILITÁRIOS (os que o
                        // pacote realmente fornece — Wave 6): cada pacote muda
                        // visivelmente estes glifos; ferramentas ficam Petunia.
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(state.t("settings.examples"))
                                    .size(10.5)
                                    .color(tokens::TEXT_MUTED),
                            );
                            for icon in [
                                PetuniaIcon::Search,
                                PetuniaIcon::Folder,
                                PetuniaIcon::Eye,
                                PetuniaIcon::Lock,
                                PetuniaIcon::Play,
                                PetuniaIcon::Trash,
                                PetuniaIcon::Plus,
                                PetuniaIcon::Undo,
                                PetuniaIcon::Settings,
                                PetuniaIcon::Move,
                            ] {
                                let (rect, resp) =
                                    ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
                                IconRegistry::paint_pack(
                                    ui.ctx(),
                                    ui.painter(),
                                    &icon,
                                    rect,
                                    tokens::TEXT_PRIMARY,
                                    &pack.id,
                                );
                                // Rótulo do preview: id estável (Wave 7 localiza).
                                resp.on_hover_text(icon.id());
                            }
                        });
                    });

                ui.add_space(6.0);
            }
        });
}

/// Texto localizado de pacote por id (`settings.<kind>_<id>`), com o valor do
/// manifest como reserva. `t()` devolve a própria chave quando falta — é esse
/// sentinela que distingue "traduzido" de "sem tradução".
fn pack_text(state: &AppState, kind: &str, id: &str, fallback: &str) -> String {
    let key = format!("settings.{kind}_{id}");
    let value = state.t(&key);
    if value == key {
        fallback.to_string()
    } else {
        value
    }
}

// ---------------------------------------------------------------- Aba 3: Idioma e Tradução
fn draw_language_tab(ui: &mut Ui, state: &mut AppState) {
    SETTINGS_FORM.section(
        ui,
        "Idioma da Interface (i18n)",
        Some(
            "Todo o texto da aplicação é mapeado através de arquivos TOML em 'assets/locales/<idioma>.toml'. Qualquer usuário pode adicionar novos idiomas ou customizar textos com facilidade.",
        ),
    );
    ui.add_space(2.0);

    let languages = [
        ("pt-BR", "Português do Brasil", "assets/locales/pt-BR.toml"),
        ("en", "English (United States)", "assets/locales/en.toml"),
    ];

    for (code, name, file_path) in languages {
        let is_active =
            state.ui.i18n.lang == code || (code == "en" && state.ui.i18n.lang == "en-US");
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
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(name)
                                    .strong()
                                    .size(13.0)
                                    .color(tokens::TEXT_PRIMARY),
                            );
                            if is_active {
                                ui.label(
                                    RichText::new("• Idioma Ativo")
                                        .size(10.5)
                                        .color(tokens::ACCENT_BLUE),
                                );
                            }
                        });
                        ui.label(
                            RichText::new(format!("Código: {code} · Arquivo: {file_path}"))
                                .size(11.0)
                                .color(tokens::TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !is_active {
                            if ui.button(RichText::new("Selecionar").size(11.0)).clicked() {
                                state.ui.i18n.set_lang(code);
                                state.mark_dirty();
                            }
                        } else {
                            ui.label(RichText::new("✔ Selecionado").color(tokens::ACCENT_BLUE));
                        }
                    });
                });
            });

        ui.add_space(6.0);
    }
}

// ---------------------------------------------------------------- Aba 4: Keymaps e Atalhos
fn draw_keymap_tab(ui: &mut Ui, state: &mut AppState) {
    let profiles = Keybinds::available_profiles();
    let mut profile_switched = false;

    SETTINGS_FORM.section(
        ui,
        "Perfis de Teclado (Keymaps)",
        Some(
            "Alterne entre o mapa nativo do Petunia3D e padrões consagrados da indústria como Blender, Maya, 3ds Max e Cinema 4D.",
        ),
    );

    // Seletor de Perfil Ativo: esteira que quebra (§48). O rótulo é o primeiro
    // item e cada perfil é um item — em painel estreito os perfis descem
    // sozinhos, sem perder a ordem.
    let layout = PetuniaResponsiveLayout::wrap_row()
        .with_justify(PetuniaJustify::Start)
        .with_gap(PetuniaGap {
            horizontal: TAB_GAP,
            vertical: TAB_GAP,
        });
    taffy_layout::responsive(
        ui,
        "settings-keymap-profiles",
        layout,
        profiles.len() + 1,
        |index, ui| {
            if index == 0 {
                ui.label(RichText::new("Perfil Ativo:").strong().size(12.0));
                return;
            }
            let profile = &profiles[index - 1];
            let is_active = state.ui.active_keymap_id == profile.id;
            let (bg, fg) = if is_active {
                (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
            } else {
                (tokens::BG_SURFACE, tokens::TEXT_SECONDARY)
            };

            let btn = egui::Button::new(RichText::new(&profile.name).size(11.0).color(fg))
                .fill(bg)
                .corner_radius(tokens::RADIUS_CONTROL);

            if ui.add(btn).on_hover_text(&profile.description).clicked() {
                state.ui.active_keymap_id = profile.id.clone();
                state.ui.keybinds = Keybinds::load_profile(&profile.id);
                state.mark_dirty();
                // Escolher um perfil é o "confirmar" deste formulário: o perfil
                // pode chegar com conflitos, e é agora que eles devem aparecer
                // campo a campo.
                profile_switched = true;
            }
        },
        |_ui| (),
    );

    ui.add_space(8.0);

    // Validação de Conflitos (Wave 7 — §51). O domínio continua dono da regra
    // (`Keybinds::detect_conflicts`); o contrato `PetuniaForm` mostra erro por
    // campo e resumo inline — sem banner com cor literal na tela.
    let conflicts = state.ui.keybinds.detect_conflicts();
    let mut session = PetuniaFormSession::new(keymap_conflict_report(state, &conflicts));
    let conflict_title = state.t("keymap.conflict_title");
    if SETTINGS_FORM.error_summary_titled(ui, &session, Some(&conflict_title)) {
        ui.add_space(8.0);
    }

    // Busca e Listagem de Atalhos
    let search_id = egui::Id::new("keymap_search_query");
    let mut search_query = ui
        .ctx()
        .data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());

    ui.horizontal(|ui| {
        let resp = widgets::petunia_search_box(
            ui,
            &mut search_query,
            "Filter shortcuts by action or key...",
        );
        if resp.changed() {
            ui.ctx()
                .data_mut(|d| d.insert_temp(search_id, search_query.clone()));
        }
    });

    ui.add_space(6.0);

    let all_bindings = state.ui.keybinds.all_bindings();

    ScrollArea::vertical()
        .auto_shrink([true, false])
        .show(ui, |ui| {
            egui::Grid::new("keymaps_table_grid")
                .striped(true)
                .min_col_width(120.0)
                .spacing(vec2(16.0, 6.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Ação / Comando")
                            .strong()
                            .color(tokens::TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new("Atalho de Teclado")
                            .strong()
                            .color(tokens::TEXT_SECONDARY),
                    );
                    #[cfg(feature = "keymap-capture")]
                    ui.label(
                        RichText::new("Capturar")
                            .strong()
                            .color(tokens::TEXT_SECONDARY),
                    );
                    ui.end_row();

                    for (action, shortcut) in all_bindings {
                        if !search_query.is_empty()
                            && !action.to_lowercase().contains(&search_query.to_lowercase())
                            && !shortcut
                                .to_lowercase()
                                .contains(&search_query.to_lowercase())
                        {
                            continue;
                        }

                        ui.label(
                            RichText::new(&action)
                                .size(11.5)
                                .color(tokens::TEXT_PRIMARY),
                        );

                        // Badge de tecla — dentro do campo validado: o atalho
                        // em conflito recebe a marca do contrato.
                        ui.horizontal(|ui| {
                            SETTINGS_FORM.validated_control(ui, &mut session, &action, |ui| {
                                let (badge_rect, response) = ui.allocate_exact_size(
                                    vec2(shortcut.len() as f32 * 7.5 + 14.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter()
                                    .rect_filled(badge_rect, 3.0, tokens::BG_SURFACE_HOVER);
                                ui.painter().rect_stroke(
                                    badge_rect,
                                    3.0,
                                    Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &shortcut,
                                    FontId::monospace(11.0),
                                    tokens::TEXT_ACTIVE,
                                );
                                response
                            });
                        });

                        #[cfg(feature = "keymap-capture")]
                        {
                            let capture_id = egui::Id::new("keymap-capture").with(&action);
                            let prompt = state.t("keymap.capture");
                            if let Some(combo) =
                                crate::key_capture::key_capture(ui, capture_id, &prompt)
                                && !crate::key_capture::apply_captured(state, &action, combo)
                            {
                                state.set_status(state.t("keymap.unsupported_key"));
                            }
                        }

                        ui.end_row();
                    }
                });
        });

    // O gesto de "confirmar" revela os erros campo a campo — os campos já
    // existem neste ponto do frame, então as marcas aparecem na próxima pintura.
    if profile_switched {
        session.reveal_errors(ui);
    }
}

/// Relatório de validação do keymap (Wave 7 — §51).
///
/// A regra é do domínio (`Keybinds::detect_conflicts`) e já existia; o que muda
/// é a **forma**: cada conflito vira erro do campo `action_a`, com o atalho e o
/// outro dono na mensagem. Um campo com mais de um conflito recebe só o primeiro
/// — o resumo mostra todos.
fn keymap_conflict_report(
    state: &AppState,
    conflicts: &[petunia_config::KeyConflict],
) -> PetuniaValidationReport {
    let mut report = PetuniaValidationReport::new();
    let mut seen: Vec<&str> = Vec::new();
    for conflict in conflicts {
        if seen.contains(&conflict.action_a.as_str()) {
            continue;
        }
        seen.push(conflict.action_a.as_str());
        report = report.with_error(
            conflict.action_a.clone(),
            format!(
                "{} — {} [{}]",
                state.t("keymap.conflict_shared"),
                conflict.action_b,
                conflict.shortcut
            ),
        );
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_modal_renders_all_tabs_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.ui.show_settings = true;

        for tab in [
            "appearance",
            "icons",
            "language",
            "keymap",
            "interface",
            "import_export",
        ] {
            state.ui.settings_tab = tab.into();
            ctx.run_ui(egui::RawInput::default(), |_ui| {
                draw(&ctx, &mut state);
            })
            .textures_delta
            .clear();
        }
    }

    #[test]
    fn test_reset_layout_actions_restore_defaults() {
        let mut state = AppState::new("en");
        state.ui.right_dock_split = 0.7;
        state.ui.outliner_collapsed = true;
        state.ui.properties_tab = "material".to_string();
        state.ui.show_shelf = false;

        reset_workspace_layout(&mut state);
        assert_eq!(state.ui.right_dock_split, 0.42);
        assert!(!state.ui.outliner_collapsed);
        // Aba do inspector não é layout do dock: preservada no reset parcial.
        assert_eq!(state.ui.properties_tab, "material");

        state.ui.right_dock_split = 0.7;
        reset_all_layouts(&mut state);
        assert_eq!(state.ui.right_dock_split, 0.42);
        assert_eq!(state.ui.properties_tab, "object");
        assert!(state.ui.show_shelf);
        assert!(state.ui.uv_show_preview);
    }

    #[test]
    fn test_shelf_visible_by_default() {
        assert!(AppState::new("en").ui.show_shelf);
    }
}
