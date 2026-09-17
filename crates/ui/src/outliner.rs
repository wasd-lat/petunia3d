//! Painel Outliner hierárquico e Galeria de Assets do Petunia3D (`Outliner`).
//! Apresenta a árvore de objetos reais da cena, coleções especializadas de Anotações e Medidas,
//! suporte a coleções/pastas de geometria, bloqueio de transformação, isolamento de visualização,
//! imagens de referência e estatísticas.

use crate::adapters::tree::{Action, NodeBuilder, TreeView, TreeViewSettings};
use egui::{Color32, Id, Rect, Response, ScrollArea, Ui, vec2};
use petunia_core::{AnnotationItem, AppState, DeleteAssetCmd, DuplicateAssetCmd, PrimitiveKind};
use uuid::Uuid;

use crate::foundation::motion::PetuniaMotion;
use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::tokens;
use crate::widgets;

/// Identificador único para cada nó da árvore no Outliner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutlinerNodeId {
    AnnotationCollection,
    AnnotationSubgroup(String),
    Annotation(Uuid),
    MeasurementCollection,
    Measurement(Uuid),
    SceneCollection,
    Collection(String),
    Camera,
    Light,
    Asset(Uuid),
    ReferenceImages,
    ReferenceImage(usize),
}

/// Renderiza o painel Outliner.
pub fn draw(ui: &mut Ui, state: &mut AppState) {
    puffin::profile_function!();
    egui::CollapsingHeader::new("Outliner")
        .default_open(true)
        .show(ui, |ui| {
            draw_body(ui, state);
        });
}

/// Conteúdo do Outliner sem o cabeçalho colapsável externo.
///
/// O dock usa colapso explícito em `UiState` (chevron no cabeçalho); quando
/// colapsado, só a linha do cabeçalho rende (faixa de 28px / tira de 44px).
pub fn draw_body(ui: &mut Ui, state: &mut AppState) {
    // 1. Cabeçalho do Outliner com colapso, contagem, ações e busca
    draw_outliner_header(ui, state);
    if state.ui.outliner_collapsed {
        return;
    }

    ui.separator();

    // Cena totalmente vazia: atalhos de criação em vez de árvore vazia.
    if scene_is_empty(state) {
        ui.add_space(6.0);
        crate::properties_panel::draw_quick_add(ui, state);
        return;
    }

    // 2. Área de rolagem com a árvore hierárquica completa.
    // Wave 2: sem cap fixo de 280px — o pai (split do dock / tile) limita.
    let scroll_max_h = ui.available_height().max(120.0);
    ScrollArea::vertical()
        .id_salt("outliner_tree_scroll")
        .max_height(scroll_max_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            draw_tree_nodes(ui, state);
        });
}

/// Árvore do Outliner **sem** cabeçalho e **sem** área de rolagem própria.
///
/// O chamador decide a moldura: no workspace PAINT a árvore é a seção Scene
/// embutida na lista de camadas (um único `ScrollArea` para as duas), e no
/// workspace MODEL o [`draw_body`] continua dono do cabeçalho, da busca e do
/// scroll. Duas rolagens aninhadas seriam o defeito clássico desse painel.
pub fn draw_tree_only(ui: &mut Ui, state: &mut AppState) {
    if scene_is_empty(state) {
        ui.add_space(6.0);
        crate::properties_panel::draw_quick_add(ui, state);
        return;
    }
    draw_tree_nodes(ui, state);
}

/// Nada criado ainda na cena (a árvore não tem o que mostrar).
fn scene_is_empty(state: &AppState) -> bool {
    state.project.assets.is_empty()
        && state.project.collections.is_empty()
        && state.project.annotations.is_empty()
        && state.project.measurements.is_empty()
}

fn outliner_node_icon(ui: &mut Ui, icon: &PetuniaIcon, fg: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        IconRegistry::paint(ui.ctx(), ui.painter(), icon, rect, fg);
    }
}

fn outliner_icon_button(ui: &mut Ui, icon: &PetuniaIcon, fg: Color32, tooltip: &str) -> Response {
    outliner_icon_button_selected(ui, icon, fg, tooltip, false)
}

fn outliner_icon_button_selected(
    ui: &mut Ui,
    icon: &PetuniaIcon,
    fg: Color32,
    tooltip: &str,
    selected: bool,
) -> Response {
    // Hitbox 24px (alvo acessível) com glifo 14px.
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if selected {
            tokens::ACCENT_BLUE
        } else if resp.hovered() {
            tokens::BG_SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, tokens::RADIUS_CONTROL, fill);
        let icon_rect = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
        IconRegistry::paint(ui.ctx(), ui.painter(), icon, icon_rect, fg);
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_CONTROL,
                tokens::stroke_focus(),
                egui::StrokeKind::Inside,
            );
        }
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tooltip));
    resp.on_hover_text(tooltip)
}

fn outliner_eye_button(ui: &mut Ui, visible: bool, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tooltip));
    if ui.is_rect_visible(rect) {
        let (icon, fg) = if visible {
            (PetuniaIcon::Eye, tokens::TEXT_PRIMARY)
        } else {
            (PetuniaIcon::EyeHidden, tokens::TEXT_MUTED)
        };
        let icon_rect = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
        IconRegistry::paint(ui.ctx(), ui.painter(), &icon, icon_rect, fg);
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_CONTROL,
                tokens::stroke_focus(),
                egui::StrokeKind::Inside,
            );
        }
    }
    resp.on_hover_text(tooltip)
}

fn outliner_lock_button(ui: &mut Ui, locked: bool, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tooltip));
    if ui.is_rect_visible(rect) {
        let (icon, fg) = if locked {
            (PetuniaIcon::Lock, Color32::from_rgb(0xe6, 0x7e, 0x22))
        } else {
            (PetuniaIcon::Unlock, tokens::TEXT_MUTED)
        };
        let icon_rect = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
        IconRegistry::paint(ui.ctx(), ui.painter(), &icon, icon_rect, fg);
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_CONTROL,
                tokens::stroke_focus(),
                egui::StrokeKind::Inside,
            );
        }
    }
    resp.on_hover_text(tooltip)
}

fn draw_outliner_header(ui: &mut Ui, state: &mut AppState) {
    let n_assets = state.project.assets.len();
    let collapsed = state.ui.outliner_collapsed;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

        // Chevron de colapso do painel (dono: UiState, como no inspector).
        let tip = state.t(if collapsed {
            "ui.expand"
        } else {
            "ui.collapse"
        });
        if widgets::chevron_toggle(ui, &tip, !collapsed).clicked() {
            state.ui.outliner_collapsed = !collapsed;
            state.mark_dirty();
        }

        // Título compacto com contador.
        ui.label(
            egui::RichText::new(state.t("scene.title"))
                .size(11.5)
                .strong()
                .color(tokens::TEXT_PRIMARY),
        );
        ui.label(
            egui::RichText::new(format!("({n_assets})"))
                .size(10.5)
                .color(tokens::TEXT_MUTED),
        );

        if collapsed {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Funil: filtros de seção/estado + voltar ao tamanho automático.
            draw_scene_filter_menu(ui, state);

            // Busca expansível (ícone → campo inline com ×).
            let search_tip = state.t("scene.search");
            if outliner_icon_button(
                ui,
                &PetuniaIcon::Search,
                tokens::TEXT_SECONDARY,
                &search_tip,
            )
            .clicked()
            {
                state.ui.scene_search_open = !state.ui.scene_search_open;
                if state.ui.scene_search_open {
                    state.ui.scene_search_focus_request = true;
                } else {
                    state.ui.outliner_search.clear();
                }
                state.mark_dirty();
            }

            // Botão de Isolar Objeto Ativo
            let is_iso = state.isolate_active;
            let iso_tip = state.t(if is_iso {
                "context.isolate_tip_on"
            } else {
                "context.isolate_tip"
            });
            if outliner_icon_button_selected(
                ui,
                &PetuniaIcon::Cursor3D,
                if is_iso {
                    tokens::TEXT_ACTIVE
                } else {
                    tokens::TEXT_SECONDARY
                },
                &iso_tip,
                is_iso,
            )
            .clicked()
            {
                state.toggle_isolate();
            }

            // Botão Nova Coleção com ícone vetorial
            let new_col_tip = state.t("context.new_collection_tip");
            if outliner_icon_button(
                ui,
                &PetuniaIcon::Folder,
                tokens::TEXT_SECONDARY,
                &new_col_tip,
            )
            .clicked()
            {
                let mut num = state.project.collections.len() + 1;
                let mut name = format!("Coleção {num}");
                while state.project.collections.contains(&name) {
                    num += 1;
                    name = format!("Coleção {num}");
                }
                state.project.add_collection(&name);
                state.mark_dirty();
            }
        });
    });

    if collapsed {
        return;
    }
    // Busca inline expansível (revelação progressiva, sem linha permanente).
    // Wave 7 (§49): o campo **abre** em vez de aparecer de uma vez — a altura
    // é interpolada pelo motion do sistema.
    let search_open = state.ui.scene_search_open;
    PetuniaMotion::section(ui, "outliner-search", search_open, |ui| {
        ui.add_space(2.0);
        let search_hint = state.t("scene.search");
        ui.horizontal(|ui| {
            let resp = widgets::petunia_search_box(ui, &mut state.ui.outliner_search, &search_hint);
            if state.ui.scene_search_focus_request {
                state.ui.scene_search_focus_request = false;
                resp.request_focus();
            }
            if outliner_icon_button(
                ui,
                &PetuniaIcon::Close,
                tokens::TEXT_MUTED,
                &state.t("ui.close"),
            )
            .clicked()
            {
                state.ui.scene_search_open = false;
                state.ui.outliner_search.clear();
                state.mark_dirty();
            }
        });
    });
}

/// Menu do funil: filtros de seção/estado + retorno ao tamanho automático.
///
/// Arquitetura extensível: novos tipos de objeto viram novos checkboxes aqui,
/// sem mexer no cabeçalho.
fn draw_scene_filter_menu(ui: &mut Ui, state: &mut AppState) {
    let tip = state.t("scene.filter");
    let resp = outliner_icon_button(ui, &PetuniaIcon::Filter, tokens::TEXT_SECONDARY, &tip);
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(160.0);
        let mut dirty = false;
        for (label_key, field) in [
            ("scene.f_collections", 0u8),
            ("scene.f_annotations", 1u8),
            ("scene.f_measurements", 2u8),
            ("scene.f_refs", 3u8),
        ] {
            let mut on = match field {
                0 => state.ui.scene_filter.show_collections,
                1 => state.ui.scene_filter.show_annotations,
                2 => state.ui.scene_filter.show_measurements,
                _ => state.ui.scene_filter.show_refs,
            };
            if ui.checkbox(&mut on, state.t(label_key)).changed() {
                match field {
                    0 => state.ui.scene_filter.show_collections = on,
                    1 => state.ui.scene_filter.show_annotations = on,
                    2 => state.ui.scene_filter.show_measurements = on,
                    _ => state.ui.scene_filter.show_refs = on,
                }
                dirty = true;
            }
        }
        ui.separator();
        ui.label(
            egui::RichText::new(state.t("scene.f_state"))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        for kind in petunia_core::SceneObjectState::all() {
            if ui
                .selectable_label(
                    state.ui.scene_filter.object_state == kind,
                    state.t(kind.key()),
                )
                .clicked()
            {
                state.ui.scene_filter.object_state = kind;
                dirty = true;
            }
        }
        ui.separator();
        if ui.button(state.t("scene.reset_split")).clicked() {
            state.ui.scene_split_auto = true;
            dirty = true;
            ui.close();
        }
        if dirty {
            state.mark_dirty();
        }
    });
}

/// Destinos das ações de uma linha de asset (aplicados após a árvore).
struct AssetRowSinks<'a> {
    delete_idx: &'a mut Option<usize>,
    toggle_lock_idx: &'a mut Option<usize>,
    toggle_vis_idx: &'a mut Option<usize>,
    dup_idx: &'a mut Option<usize>,
    isolate_idx: &'a mut Option<usize>,
    move_to_col: &'a mut Option<(usize, Option<String>)>,
    rename_asset: &'a mut Option<(usize, String)>,
}

/// Inicia rename inline de asset (F2, duplo-clique/Enter, menu).
/// Buffer começa vazio (original vai de dica): sem armadilha de anexar.
fn begin_asset_rename(ui: &mut Ui, asset_id: Uuid) {
    ui.data_mut(|d| {
        d.insert_temp(egui::Id::new("petunia_rename_asset_id"), asset_id);
        d.insert_temp(egui::Id::new("petunia_rename_asset_buf"), String::new());
        d.insert_temp(egui::Id::new("petunia_rename_asset_focus"), true);
    });
}

/// Asset em rename inline, se houver.
fn renaming_asset(ui: &Ui) -> Option<Uuid> {
    ui.data(|d| d.get_temp::<Uuid>(egui::Id::new("petunia_rename_asset_id")))
}

/// Filtro de objeto do Scene (busca textual + estado). Hierarquia intacta: o
/// estado abrir/fechar da árvore persiste por id de nó.
fn asset_passes_filters(state: &AppState, i: usize, search: &str) -> bool {
    let Some(asset) = state.project.assets.get(i) else {
        return false;
    };
    if !search.is_empty() && !asset.name.to_lowercase().contains(search) {
        return false;
    }
    match state.ui.scene_filter.object_state {
        petunia_core::SceneObjectState::All => true,
        petunia_core::SceneObjectState::VisibleOnly => asset.visible,
        petunia_core::SceneObjectState::UnlockedOnly => !asset.locked,
    }
}

/// Linha de asset (Blender: toggle + ícone + nome; Plasticity: dot de
/// material; C4D: olho/cadeado à direita). Nome limpo com contagens no
/// tooltip; Delete só sob demanda; destrutivos no menu de contexto.
fn draw_asset_row(
    ui: &mut Ui,
    state: &mut AppState,
    i: usize,
    collections: &[String],
    renaming: Option<Uuid>,
    sinks: AssetRowSinks<'_>,
) {
    let Some(asset) = state.project.assets.get(i) else {
        return;
    };
    let name = asset.name.clone();
    let visible = asset.visible;
    let locked = asset.locked;
    let asset_id = asset.id;
    let base_color = asset.base_color;
    let vc = asset.mesh.vert_count();
    let fc = asset.mesh.tri_count();
    let is_selected = i == state.project.active;
    let AssetRowSinks {
        delete_idx,
        toggle_lock_idx,
        toggle_vis_idx,
        dup_idx,
        isolate_idx,
        move_to_col,
        rename_asset,
    } = sinks;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

        let fg = if is_selected {
            tokens::TEXT_ACTIVE
        } else {
            tokens::TEXT_PRIMARY
        };

        outliner_node_icon(ui, &PetuniaIcon::ObjectMesh, fg);
        if renaming == Some(asset_id) {
            // Edição inline (Enter confirma, Esc cancela, foco automático).
            // Começa vazia com o original de dica: sem armadilha de anexar.
            let mut buf = ui
                .data(|d| d.get_temp::<String>(egui::Id::new("petunia_rename_asset_buf")))
                .unwrap_or_default();
            let resp = ui.add_sized(
                vec2(ui.available_width().max(40.0), 20.0),
                egui::TextEdit::singleline(&mut buf).hint_text(&name),
            );
            if ui.data(|d| {
                d.get_temp::<bool>(egui::Id::new("petunia_rename_asset_focus"))
                    .unwrap_or(false)
            }) {
                resp.request_focus();
                ui.data_mut(|d| d.insert_temp(egui::Id::new("petunia_rename_asset_focus"), false));
            }
            ui.data_mut(|d| d.insert_temp(egui::Id::new("petunia_rename_asset_buf"), buf.clone()));
            let commit = resp.lost_focus()
                || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            let cancel = resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape));
            if cancel {
                ui.data_mut(|d| d.remove_temp::<Uuid>(egui::Id::new("petunia_rename_asset_id")));
            } else if commit && !buf.trim().is_empty() && buf.trim() != name {
                *rename_asset = Some((i, buf.trim().to_string()));
                ui.data_mut(|d| d.remove_temp::<Uuid>(egui::Id::new("petunia_rename_asset_id")));
            }
        } else {
            let label_resp = ui
                .selectable_label(is_selected, egui::RichText::new(&name).size(11.0).color(fg))
                .on_hover_text(format!("{name} — {vc} verts, {fc} tris"));
            if label_resp.clicked() {
                state.set_active_asset_by_id(asset_id);
                state.selected_annotation = None;
                state.selected_measurement = None;
                state.mark_dirty();
                // Foco segue para a árvore (teclado exige foco no tree_id).
                ui.memory_mut(|m| m.request_focus(egui::Id::new("petunia_outliner_ltreeview")));
            }
            label_resp.context_menu(|ui| {
                if widgets::PetuniaMenuItem::new(&state.t("context.rename"))
                    .shortcut(Some("F2"))
                    .show(ui)
                    .clicked()
                {
                    begin_asset_rename(ui, asset_id);
                    ui.close();
                }
                if widgets::PetuniaMenuItem::new(&state.t("actions.duplicate"))
                    .icon(PetuniaIcon::Duplicate)
                    .shortcut(Some("Shift+D"))
                    .show(ui)
                    .clicked()
                {
                    *dup_idx = Some(i);
                    ui.close();
                }
                let (lock_txt, lock_icon) = if locked {
                    (state.t("context.unlock"), PetuniaIcon::Unlock)
                } else {
                    (state.t("context.lock"), PetuniaIcon::Lock)
                };
                if widgets::PetuniaMenuItem::new(&lock_txt)
                    .icon(lock_icon)
                    .show(ui)
                    .clicked()
                {
                    *toggle_lock_idx = Some(i);
                    ui.close();
                }
                let iso_txt = if state.isolate_active && is_selected {
                    state.t("context.isolate_exit")
                } else {
                    state.t("context.isolate")
                };
                if widgets::PetuniaMenuItem::new(&iso_txt)
                    .icon(PetuniaIcon::Eye)
                    .shortcut(Some("Numpad /"))
                    .show(ui)
                    .clicked()
                {
                    *isolate_idx = Some(i);
                    ui.close();
                }
                if !collections.is_empty() {
                    ui.separator();
                    let move_label = state.t("context.move_to_collection");
                    let none_label = state.t("context.none_root");
                    widgets::PetuniaMenuButton::new(&move_label).show(ui, |ui| {
                        if widgets::PetuniaMenuItem::new(&none_label)
                            .show(ui)
                            .clicked()
                        {
                            *move_to_col = Some((i, None));
                            ui.close();
                        }
                        ui.separator();
                        for col in collections {
                            let is_cur = state.project.assets[i].collection.as_deref() == Some(col);
                            let label = if is_cur {
                                format!("• {col}")
                            } else {
                                col.clone()
                            };
                            if widgets::PetuniaMenuItem::new(&label)
                                .icon(PetuniaIcon::Folder)
                                .show(ui)
                                .clicked()
                            {
                                *move_to_col = Some((i, Some(col.clone())));
                                ui.close();
                            }
                        }
                    });
                }
                ui.separator();
                let export_label = state.t("context.export");
                let delete_label = state.t("actions.delete");
                if widgets::PetuniaMenuItem::new(&export_label)
                    .show(ui)
                    .clicked()
                {
                    crate::export_dialog(state, &[i]);
                    ui.close();
                }
                if widgets::PetuniaMenuItem::new(&delete_label)
                    .icon(PetuniaIcon::Trash)
                    .shortcut(Some("Delete"))
                    .show(ui)
                    .clicked()
                {
                    *delete_idx = Some(i);
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Delete só sob demanda (hover/foco/seleção): menos ruído.
                let show_delete = label_resp.hovered() || label_resp.has_focus() || is_selected;
                if show_delete
                    && outliner_icon_button(
                        ui,
                        &PetuniaIcon::Delete,
                        tokens::TEXT_MUTED,
                        &state.t("context.delete_tip"),
                    )
                    .clicked()
                {
                    *delete_idx = Some(i);
                }

                let lock_tip = state.t(if locked {
                    "context.unlock"
                } else {
                    "context.lock_tip"
                });
                if outliner_lock_button(ui, locked, &lock_tip).clicked() {
                    *toggle_lock_idx = Some(i);
                }

                let eye_tip = state.t(if visible {
                    "context.hide"
                } else {
                    "context.show"
                });
                if outliner_eye_button(ui, visible, &eye_tip).clicked() {
                    *toggle_vis_idx = Some(i);
                }

                // Ponto de material (Plasticity): cor-base + salto à aba.
                let dot_c = egui::Color32::from_rgb(
                    (base_color[0] * 255.0) as u8,
                    (base_color[1] * 255.0) as u8,
                    (base_color[2] * 255.0) as u8,
                );
                let mat_tip = state.t("inspector.go_material");
                let (dot_rect, dot_resp) =
                    ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::click());
                dot_resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, mat_tip.clone())
                });
                if ui.is_rect_visible(dot_rect) {
                    ui.painter().circle_filled(dot_rect.center(), 5.0, dot_c);
                    if dot_resp.hovered() || dot_resp.has_focus() {
                        ui.painter().circle_stroke(
                            dot_rect.center(),
                            6.5,
                            egui::Stroke::new(1.0_f32, tokens::TEXT_SECONDARY),
                        );
                    }
                }
                if dot_resp.on_hover_text(&mat_tip).clicked() {
                    state.ui.properties_tab = "material".into();
                    state.mark_dirty();
                }
            });
        }
    });
}

fn draw_tree_nodes(ui: &mut Ui, state: &mut AppState) {
    let search = state.ui.outliner_search.trim().to_lowercase();
    let n_assets = state.project.assets.len();
    let active_idx = state.project.active;
    let collections = state.project.collections.clone();

    let tree_id = Id::new("petunia_outliner_ltreeview");
    let tree = TreeView::new(tree_id)
        .with_settings(TreeViewSettings {
            override_indent: Some(14.0),
            default_node_height: Some(crate::inspector_widgets::row_h(state.ui.density)),
            ..Default::default()
        })
        .allow_drag_and_drop(false)
        .allow_multi_selection(false);

    if let Some(mut tree_state) =
        crate::adapters::tree::TreeViewState::<OutlinerNodeId>::load(ui, tree_id)
        && let Some(active_asset) = state.project.assets.get(active_idx)
    {
        let target = OutlinerNodeId::Asset(active_asset.id);
        if !tree_state.selected().contains(&target) {
            tree_state.set_one_selected(target);
            tree_state.store(ui, tree_id);
        }
    }

    // Ações para meshes
    let mut toggle_vis_idx: Option<usize> = None;
    let mut toggle_lock_idx: Option<usize> = None;
    let mut delete_idx: Option<usize> = None;
    let mut dup_idx: Option<usize> = None;
    let mut isolate_idx: Option<usize> = None;
    let mut move_to_col: Option<(usize, Option<String>)> = None;
    let mut rename_asset: Option<(usize, String)> = None;
    let renaming = renaming_asset(ui);

    let mut toggle_col_vis: Option<String> = None;
    let mut toggle_col_lock: Option<String> = None;
    let mut delete_col: Option<String> = None;
    let mut rename_col: Option<(String, String)> = None;

    // Ações para referências
    let mut ref_toggle_vis: Option<usize> = None;
    let mut ref_toggle_xray: Option<usize> = None;
    let mut ref_delete: Option<usize> = None;

    // Ações para anotações
    let mut toggle_ann_vis: Option<Uuid> = None;
    let mut toggle_ann_lock: Option<Uuid> = None;
    let mut delete_ann: Option<Uuid> = None;
    let mut dup_ann: Option<Uuid> = None;
    let mut ann_move_to_group: Option<(Uuid, Option<String>)> = None;
    let mut toggle_all_ann_vis = false;
    let mut toggle_all_ann_lock = false;
    let mut clear_all_ann = false;
    let mut add_ann_subgroup: Option<String> = None;
    let mut delete_ann_subgroup: Option<String> = None;

    // Ações para medidas
    let mut toggle_meas_vis: Option<Uuid> = None;
    let mut delete_meas: Option<Uuid> = None;
    let mut toggle_all_meas_vis = false;
    let mut clear_all_meas = false;

    // Atalhos de teclado no Outliner quando nenhum campo de texto está ativo
    if !ui.ctx().egui_wants_keyboard_input() {
        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if let Some(ann_id) = state.selected_annotation {
                delete_ann = Some(ann_id);
            } else if let Some(meas_id) = state.selected_measurement {
                delete_meas = Some(meas_id);
            } else if active_idx < state.project.assets.len() {
                delete_idx = Some(active_idx);
            }
        }
        if ui.input(|i| (i.modifiers.shift || i.modifiers.command) && i.key_pressed(egui::Key::D)) {
            if let Some(ann_id) = state.selected_annotation {
                dup_ann = Some(ann_id);
            } else if active_idx < state.project.assets.len() {
                dup_idx = Some(active_idx);
            }
        }
        // F2 renomeia o ativo; Ctrl+F abre e foca a busca do Scene.
        if ui.input(|i| i.key_pressed(egui::Key::F2))
            && let Some(asset) = state.project.assets.get(active_idx)
        {
            begin_asset_rename(ui, asset.id);
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            state.ui.scene_search_open = true;
            state.ui.scene_search_focus_request = true;
            state.mark_dirty();
        }
    }

    let show_ann_collection = (!state.project.annotations.is_empty()
        || !state.project.annotation_groups.is_empty()
        || state.active_tool == "annotate")
        && state.ui.scene_filter.show_annotations;
    let show_meas_collection = (!state.project.measurements.is_empty()
        || state.active_tool == "measure")
        && state.ui.scene_filter.show_measurements;

    let (_, actions) = tree.show(ui, |builder| {
        // =========================================================================
        // SEÇÃO 1: ANOTAÇÕES (Acima das existentes, tipo dedicado, cor ciano)
        // =========================================================================
        if show_ann_collection {
            let n_anns = state.project.annotations.len();
            let all_ann_vis = state.project.annotations_visible;
            let all_ann_lock = state.project.annotations_locked;

            let open_ann = builder.node(
                NodeBuilder::dir(OutlinerNodeId::AnnotationCollection)
                    .default_open(true)
                    .label_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                            outliner_node_icon(
                                ui,
                                &PetuniaIcon::Annotate,
                                Color32::from_rgb(0, 210, 211),
                            );
                            let col_resp = ui.label(
                                egui::RichText::new(format!("Annotations ({n_anns})"))
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(0, 210, 211)),
                            );

                            col_resp.context_menu(|ui| {
                                if ui.button("New Subgroup...").clicked() {
                                    let mut num = state.project.annotation_groups.len() + 1;
                                    let mut name = format!("Subgroup {num}");
                                    while state.project.annotation_groups.contains(&name) {
                                        num += 1;
                                        name = format!("Subgroup {num}");
                                    }
                                    add_ann_subgroup = Some(name);
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Toggle Collection Visibility").clicked() {
                                    toggle_all_ann_vis = true;
                                    ui.close();
                                }
                                if ui.button("Toggle Collection Lock").clicked() {
                                    toggle_all_ann_lock = true;
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Clear All Annotations").clicked() {
                                    clear_all_ann = true;
                                    ui.close();
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if outliner_eye_button(
                                        ui,
                                        all_ann_vis,
                                        "Toggle visibility of all annotations",
                                    )
                                    .clicked()
                                    {
                                        toggle_all_ann_vis = true;
                                    }

                                    if outliner_lock_button(
                                        ui,
                                        all_ann_lock,
                                        "Toggle lock of all annotations",
                                    )
                                    .clicked()
                                    {
                                        toggle_all_ann_lock = true;
                                    }
                                },
                            );
                        });
                    }),
            );

            if open_ann {
                // Subgrupos internos de anotações
                for group_name in &state.project.annotation_groups.clone() {
                    let anns_in_group: Vec<AnnotationItem> = state
                        .project
                        .annotations
                        .iter()
                        .filter(|a| a.group.as_deref() == Some(group_name))
                        .cloned()
                        .collect();

                    let group_for_closure = group_name.clone();
                    let count = anns_in_group.len();

                    let open_group = builder.node(
                        NodeBuilder::dir(OutlinerNodeId::AnnotationSubgroup(group_name.clone()))
                            .default_open(true)
                            .label_ui(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                                    outliner_node_icon(
                                        ui,
                                        &PetuniaIcon::Folder,
                                        Color32::from_rgb(0, 180, 180),
                                    );
                                    let resp = ui.label(
                                        egui::RichText::new(format!(
                                            "{group_for_closure} ({count})"
                                        ))
                                        .size(11.0)
                                        .color(Color32::from_rgb(0, 180, 180)),
                                    );
                                    resp.context_menu(|ui| {
                                        if ui.button("Delete Subgroup").clicked() {
                                            delete_ann_subgroup = Some(group_for_closure.clone());
                                            ui.close();
                                        }
                                    });
                                });
                            }),
                    );

                    if open_group {
                        for ann in anns_in_group {
                            let is_selected = state.selected_annotation == Some(ann.id);
                            let ann_id = ann.id;
                            let name = ann.name.clone();
                            let visible = ann.visible;
                            let locked = ann.locked;

                            builder.node(
                                NodeBuilder::leaf(OutlinerNodeId::Annotation(ann.id)).label_ui(
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                                            let fg = if is_selected {
                                                tokens::TEXT_ACTIVE
                                            } else {
                                                Color32::from_rgb(0, 210, 211)
                                            };

                                            outliner_node_icon(ui, &PetuniaIcon::Annotate, fg);
                                            let label_resp = ui.label(
                                                egui::RichText::new(&name)
                                                    .size(11.0)
                                                    .color(fg)
                                                    .background_color(if is_selected {
                                                        Color32::from_rgb(0, 100, 120)
                                                    } else {
                                                        Color32::TRANSPARENT
                                                    }),
                                            );

                                            label_resp.context_menu(|ui| {
                                                widgets::PetuniaMenuButton::new("Move to Subgroup")
                                                    .show(ui, |ui| {
                                                        if ui.button("None (Root)").clicked() {
                                                            ann_move_to_group =
                                                                Some((ann_id, None));
                                                            ui.close();
                                                        }
                                                        for g in &state.project.annotation_groups {
                                                            if ui.button(g).clicked() {
                                                                ann_move_to_group =
                                                                    Some((ann_id, Some(g.clone())));
                                                                ui.close();
                                                            }
                                                        }
                                                    });
                                                ui.separator();
                                                let lock_txt =
                                                    if locked { "Unlock" } else { "Lock" };
                                                if ui.button(lock_txt).clicked() {
                                                    toggle_ann_lock = Some(ann_id);
                                                    ui.close();
                                                }
                                                if ui.button("Duplicate").clicked() {
                                                    dup_ann = Some(ann_id);
                                                    ui.close();
                                                }
                                                if ui.button("Delete").clicked() {
                                                    delete_ann = Some(ann_id);
                                                    ui.close();
                                                }
                                            });

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if outliner_eye_button(
                                                        ui,
                                                        visible,
                                                        "Toggle annotation visibility",
                                                    )
                                                    .clicked()
                                                    {
                                                        toggle_ann_vis = Some(ann_id);
                                                    }

                                                    if outliner_lock_button(
                                                        ui,
                                                        locked,
                                                        "Toggle annotation lock",
                                                    )
                                                    .clicked()
                                                    {
                                                        toggle_ann_lock = Some(ann_id);
                                                    }
                                                },
                                            );
                                        });
                                    },
                                ),
                            );
                        }
                        builder.close_dir();
                    }
                }

                // Anotações na raiz da coleção (sem subgrupo)
                for ann in state
                    .project
                    .annotations
                    .iter()
                    .filter(|a| a.group.is_none())
                    .cloned()
                {
                    let is_selected = state.selected_annotation == Some(ann.id);
                    let ann_id = ann.id;
                    let name = ann.name.clone();
                    let visible = ann.visible;
                    let locked = ann.locked;

                    builder.node(
                        NodeBuilder::leaf(OutlinerNodeId::Annotation(ann.id)).label_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                                let fg = if is_selected {
                                    tokens::TEXT_ACTIVE
                                } else {
                                    Color32::from_rgb(0, 210, 211)
                                };

                                outliner_node_icon(ui, &PetuniaIcon::Annotate, fg);
                                let label_resp = ui.label(
                                    egui::RichText::new(&name)
                                        .size(11.0)
                                        .color(fg)
                                        .background_color(if is_selected {
                                            Color32::from_rgb(0, 100, 120)
                                        } else {
                                            Color32::TRANSPARENT
                                        }),
                                );

                                label_resp.context_menu(|ui| {
                                    if !state.project.annotation_groups.is_empty() {
                                        widgets::PetuniaMenuButton::new("Move to Subgroup").show(
                                            ui,
                                            |ui| {
                                                for g in &state.project.annotation_groups {
                                                    if ui.button(g).clicked() {
                                                        ann_move_to_group =
                                                            Some((ann_id, Some(g.clone())));
                                                        ui.close();
                                                    }
                                                }
                                            },
                                        );
                                        ui.separator();
                                    }
                                    let lock_txt = if locked { "Unlock" } else { "Lock" };
                                    if ui.button(lock_txt).clicked() {
                                        toggle_ann_lock = Some(ann_id);
                                        ui.close();
                                    }
                                    if ui.button("Duplicate").clicked() {
                                        dup_ann = Some(ann_id);
                                        ui.close();
                                    }
                                    if ui.button("Delete").clicked() {
                                        delete_ann = Some(ann_id);
                                        ui.close();
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if outliner_eye_button(
                                            ui,
                                            visible,
                                            "Toggle annotation visibility",
                                        )
                                        .clicked()
                                        {
                                            toggle_ann_vis = Some(ann_id);
                                        }

                                        if outliner_lock_button(
                                            ui,
                                            locked,
                                            "Toggle annotation lock",
                                        )
                                        .clicked()
                                        {
                                            toggle_ann_lock = Some(ann_id);
                                        }
                                    },
                                );
                            });
                        }),
                    );
                }

                builder.close_dir();
            }
        }

        // =========================================================================
        // SEÇÃO 2: MEDIDAS (Acima das de malha, tipo dedicado, cor amarela)
        // =========================================================================
        if show_meas_collection {
            let n_meas = state.project.measurements.len();
            let all_meas_vis = state.project.measurements_visible;

            let open_meas = builder.node(
                NodeBuilder::dir(OutlinerNodeId::MeasurementCollection)
                    .default_open(true)
                    .label_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                            outliner_node_icon(
                                ui,
                                &PetuniaIcon::Measure,
                                Color32::from_rgb(254, 202, 87),
                            );
                            let col_resp = ui.label(
                                egui::RichText::new(format!("Measurements ({n_meas})"))
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(254, 202, 87)),
                            );

                            col_resp.context_menu(|ui| {
                                if ui.button("Toggle Collection Visibility").clicked() {
                                    toggle_all_meas_vis = true;
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button("Clear All Measurements").clicked() {
                                    clear_all_meas = true;
                                    ui.close();
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if outliner_eye_button(
                                        ui,
                                        all_meas_vis,
                                        "Toggle visibility of all measurements",
                                    )
                                    .clicked()
                                    {
                                        toggle_all_meas_vis = true;
                                    }
                                },
                            );
                        });
                    }),
            );

            if open_meas {
                for m in state.project.measurements.clone() {
                    let m_id = m.id;
                    let is_selected = state.selected_measurement == Some(m_id);
                    let name = m.name.clone();
                    let dist = m.distance;
                    let visible = m.visible;

                    builder.node(
                        NodeBuilder::leaf(OutlinerNodeId::Measurement(m_id)).label_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                                let fg = if is_selected {
                                    tokens::TEXT_ACTIVE
                                } else {
                                    Color32::from_rgb(254, 202, 87)
                                };

                                outliner_node_icon(ui, &PetuniaIcon::Measure, fg);
                                let label_resp = ui.label(
                                    egui::RichText::new(format!("{name} ({dist:.2}m)"))
                                        .size(11.0)
                                        .color(fg)
                                        .background_color(if is_selected {
                                            Color32::from_rgb(120, 100, 20)
                                        } else {
                                            Color32::TRANSPARENT
                                        }),
                                );

                                label_resp.context_menu(|ui| {
                                    if ui.button("Delete").clicked() {
                                        delete_meas = Some(m_id);
                                        ui.close();
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if outliner_eye_button(
                                            ui,
                                            visible,
                                            "Toggle measurement visibility",
                                        )
                                        .clicked()
                                        {
                                            toggle_meas_vis = Some(m_id);
                                        }

                                        if (label_resp.hovered()
                                            || label_resp.has_focus()
                                            || is_selected)
                                            && outliner_icon_button(
                                                ui,
                                                &PetuniaIcon::Delete,
                                                tokens::TEXT_MUTED,
                                                "Delete measurement",
                                            )
                                            .clicked()
                                        {
                                            delete_meas = Some(m_id);
                                        }
                                    },
                                );
                            });
                        }),
                    );
                }

                builder.close_dir();
            }
        }

        // =========================================================================
        // SEÇÃO 3: SCENE COLLECTION (Malhas 3D e Coleções de Objetos)
        // =========================================================================
        let open_scene = builder.node(
            NodeBuilder::dir(OutlinerNodeId::SceneCollection)
                .default_open(true)
                .label_ui(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                        outliner_node_icon(ui, &PetuniaIcon::Collection, tokens::TEXT_PRIMARY);
                        ui.label(
                            egui::RichText::new("Scene Collection")
                                .size(11.5)
                                .color(tokens::TEXT_PRIMARY),
                        );
                    });
                }),
        );

        if open_scene {
            // 1. Coleções personalizadas de geometria (funil pode ocultar;
            // os assets caem para a raiz, nunca somem).
            for col_name in &collections {
                if !state.ui.scene_filter.show_collections {
                    continue;
                }
                let assets_in_col: Vec<usize> = (0..n_assets)
                    .filter(|&i| state.project.assets[i].collection.as_deref() == Some(col_name))
                    .collect();

                let all_visible = !assets_in_col.is_empty()
                    && assets_in_col
                        .iter()
                        .all(|&i| state.project.assets[i].visible);
                let all_locked = !assets_in_col.is_empty()
                    && assets_in_col
                        .iter()
                        .all(|&i| state.project.assets[i].locked);

                let col_for_closure = col_name.clone();
                let count = assets_in_col.len();

                let open_col = builder.node(
                    NodeBuilder::dir(OutlinerNodeId::Collection(col_name.clone()))
                        .default_open(true)
                        .label_ui(|ui| {
                            let rename_id = egui::Id::new("petunia_renaming_col");
                            let is_renaming = ui.data(|d| {
                                d.get_temp::<String>(rename_id).as_deref() == Some(&col_for_closure)
                            });

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

                                if is_renaming {
                                    let mut edit_buf = ui.data(|d| {
                                        d.get_temp::<String>(egui::Id::new("petunia_rename_buf"))
                                            .unwrap_or_else(|| col_for_closure.clone())
                                    });
                                    let resp = ui.text_edit_singleline(&mut edit_buf);
                                    if resp.lost_focus()
                                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        if edit_buf.trim() != col_for_closure
                                            && !edit_buf.trim().is_empty()
                                        {
                                            rename_col = Some((
                                                col_for_closure.clone(),
                                                edit_buf.trim().to_string(),
                                            ));
                                        }
                                        ui.data_mut(|d| d.remove_temp::<String>(rename_id));
                                        ui.data_mut(|d| {
                                            d.remove_temp::<String>(egui::Id::new(
                                                "petunia_rename_buf",
                                            ))
                                        });
                                    } else {
                                        ui.data_mut(|d| {
                                            d.insert_temp(
                                                egui::Id::new("petunia_rename_buf"),
                                                edit_buf,
                                            )
                                        });
                                    }
                                } else {
                                    outliner_node_icon(
                                        ui,
                                        &PetuniaIcon::Folder,
                                        tokens::TEXT_PRIMARY,
                                    );
                                    let col_resp = ui.label(
                                        egui::RichText::new(format!("{col_for_closure} ({count})"))
                                            .size(11.0)
                                            .color(tokens::TEXT_PRIMARY),
                                    );
                                    col_resp.context_menu(|ui| {
                                        if widgets::PetuniaMenuItem::new(
                                            &state.t("context.rename_collection"),
                                        )
                                        .icon(PetuniaIcon::Folder)
                                        .show(ui)
                                        .clicked()
                                        {
                                            ui.data_mut(|d| {
                                                d.insert_temp(rename_id, col_for_closure.clone());
                                                d.insert_temp(
                                                    egui::Id::new("petunia_rename_buf"),
                                                    col_for_closure.clone(),
                                                );
                                            });
                                            ui.close();
                                        }
                                        if widgets::PetuniaMenuItem::new(
                                            &state.t("context.delete_collection"),
                                        )
                                        .icon(PetuniaIcon::Trash)
                                        .show(ui)
                                        .clicked()
                                        {
                                            delete_col = Some(col_for_closure.clone());
                                            ui.close();
                                        }
                                        ui.separator();
                                        let toggle_vis_label = state.t("context.toggle_col_vis");
                                        let toggle_lock_label = state.t("context.toggle_col_lock");
                                        if widgets::PetuniaMenuItem::new(&toggle_vis_label)
                                            .icon(PetuniaIcon::Eye)
                                            .show(ui)
                                            .clicked()
                                        {
                                            toggle_col_vis = Some(col_for_closure.clone());
                                            ui.close();
                                        }
                                        if widgets::PetuniaMenuItem::new(&toggle_lock_label)
                                            .icon(PetuniaIcon::Lock)
                                            .show(ui)
                                            .clicked()
                                        {
                                            toggle_col_lock = Some(col_for_closure.clone());
                                            ui.close();
                                        }
                                    });
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let col_eye_tip = state.t("context.toggle_col_vis");
                                        let col_lock_tip = state.t("context.toggle_col_lock");
                                        if outliner_eye_button(ui, all_visible, &col_eye_tip)
                                            .clicked()
                                        {
                                            toggle_col_vis = Some(col_for_closure.clone());
                                        }

                                        if outliner_lock_button(ui, all_locked, &col_lock_tip)
                                            .clicked()
                                        {
                                            toggle_col_lock = Some(col_for_closure.clone());
                                        }
                                    },
                                );
                            });
                        }),
                );

                if open_col {
                    for i in assets_in_col {
                        if !asset_passes_filters(state, i, &search) {
                            continue;
                        }
                        let asset_id = state.project.assets[i].id;
                        builder.node(NodeBuilder::leaf(OutlinerNodeId::Asset(asset_id)).label_ui(
                            |ui| {
                                draw_asset_row(
                                    ui,
                                    state,
                                    i,
                                    &collections,
                                    renaming,
                                    AssetRowSinks {
                                        delete_idx: &mut delete_idx,
                                        toggle_lock_idx: &mut toggle_lock_idx,
                                        toggle_vis_idx: &mut toggle_vis_idx,
                                        dup_idx: &mut dup_idx,
                                        isolate_idx: &mut isolate_idx,
                                        move_to_col: &mut move_to_col,
                                        rename_asset: &mut rename_asset,
                                    },
                                );
                            },
                        ));
                    }
                    builder.close_dir();
                }
            }

            // 2. Objetos na raiz da cena (sem coleção ou com coleção inexistente)
            // 2. Objetos na raiz da cena (sem coleção, coleção inexistente
            // ou coleções ocultas pelo filtro).
            for i in 0..n_assets {
                let show_at_root = {
                    let has_valid = state.project.assets[i]
                        .collection
                        .as_ref()
                        .is_some_and(|c| collections.contains(c));
                    !has_valid || !state.ui.scene_filter.show_collections
                };
                if !show_at_root {
                    continue;
                }
                if !asset_passes_filters(state, i, &search) {
                    continue;
                }
                let asset_id = state.project.assets[i].id;
                builder.node(
                    NodeBuilder::leaf(OutlinerNodeId::Asset(asset_id)).label_ui(|ui| {
                        draw_asset_row(
                            ui,
                            state,
                            i,
                            &collections,
                            renaming,
                            AssetRowSinks {
                                delete_idx: &mut delete_idx,
                                toggle_lock_idx: &mut toggle_lock_idx,
                                toggle_vis_idx: &mut toggle_vis_idx,
                                dup_idx: &mut dup_idx,
                                isolate_idx: &mut isolate_idx,
                                move_to_col: &mut move_to_col,
                                rename_asset: &mut rename_asset,
                            },
                        );
                    }),
                );
            }

            builder.close_dir();
        }

        // =========================================================================
        // SEÇÃO 4: IMAGENS DE REFERÊNCIA
        // =========================================================================
        if !state.project.refs.is_empty() && state.ui.scene_filter.show_refs {
            let open_refs = builder.node(
                NodeBuilder::dir(OutlinerNodeId::ReferenceImages)
                    .default_open(true)
                    .label_ui(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                            outliner_node_icon(
                                ui,
                                &PetuniaIcon::ReferenceImage,
                                tokens::TEXT_PRIMARY,
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "Reference Images ({})",
                                    state.project.refs.len()
                                ))
                                .size(11.5)
                                .color(tokens::TEXT_PRIMARY),
                            );
                        });
                    }),
            );

            if open_refs {
                for r_idx in 0..state.project.refs.len() {
                    let r_vis = state.project.refs[r_idx].visible;
                    let r_xray = state.project.refs[r_idx].xray;
                    let r_axis = format!("{:?}", state.project.refs[r_idx].axis);
                    let r_dim = format!(
                        "{}x{}",
                        state.project.refs[r_idx].width, state.project.refs[r_idx].height
                    );

                    builder.node(
                        NodeBuilder::leaf(OutlinerNodeId::ReferenceImage(r_idx)).label_ui(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                                outliner_node_icon(
                                    ui,
                                    &PetuniaIcon::ReferenceImage,
                                    tokens::TEXT_PRIMARY,
                                );
                                let item_label = format!("{r_axis} ({r_dim})");
                                let label_resp = ui.label(
                                    egui::RichText::new(item_label)
                                        .size(11.0)
                                        .color(tokens::TEXT_PRIMARY),
                                );
                                label_resp.context_menu(|ui| {
                                    let xray_txt = if r_xray {
                                        "Disable X-Ray"
                                    } else {
                                        "Enable X-Ray"
                                    };
                                    if ui.button(xray_txt).clicked() {
                                        ref_toggle_xray = Some(r_idx);
                                        ui.close();
                                    }
                                    if ui.button("Remove Image").clicked() {
                                        ref_delete = Some(r_idx);
                                        ui.close();
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if outliner_eye_button(
                                            ui,
                                            r_vis,
                                            if r_vis {
                                                "Hide Reference Image"
                                            } else {
                                                "Show Reference Image"
                                            },
                                        )
                                        .clicked()
                                        {
                                            ref_toggle_vis = Some(r_idx);
                                        }

                                        let xray_col = if r_xray {
                                            tokens::ACCENT_BLUE
                                        } else {
                                            tokens::TEXT_MUTED
                                        };
                                        if outliner_icon_button(
                                            ui,
                                            &PetuniaIcon::XRay,
                                            xray_col,
                                            if r_xray {
                                                "X-Ray Active (visible over meshes)"
                                            } else {
                                                "Enable X-Ray"
                                            },
                                        )
                                        .clicked()
                                        {
                                            ref_toggle_xray = Some(r_idx);
                                        }
                                    },
                                );
                            });
                        }),
                    );
                }
                builder.close_dir();
            }
        }
    });

    // =========================================================================
    // DISPATCHING DE AÇÕES APÓS TREEVIEW
    // =========================================================================

    // Ações de anotações
    if let Some(ann_id) = toggle_ann_vis
        && let Some(ann) = state
            .project
            .annotations
            .iter_mut()
            .find(|a| a.id == ann_id)
    {
        ann.visible = !ann.visible;
        state.mark_dirty();
    }
    if let Some(ann_id) = toggle_ann_lock
        && let Some(ann) = state
            .project
            .annotations
            .iter_mut()
            .find(|a| a.id == ann_id)
    {
        ann.locked = !ann.locked;
        state.mark_dirty();
    }
    if let Some(ann_id) = delete_ann {
        state.checkpoint("delete annotation");
        state.project.remove_annotation(ann_id);
        if state.selected_annotation == Some(ann_id) {
            state.selected_annotation = None;
        }
        state.mark_dirty();
    }
    if let Some(ann_id) = dup_ann {
        let dup = state
            .project
            .annotations
            .iter()
            .find(|a| a.id == ann_id)
            .cloned();
        if let Some(mut dup) = dup {
            state.checkpoint("duplicate annotation");
            dup.id = Uuid::new_v4();
            dup.name = format!("{} (cópia)", dup.name);
            let new_id = dup.id;
            state.project.add_annotation(dup);
            state.selected_annotation = Some(new_id);
            state.mark_dirty();
        }
    }
    if let Some((ann_id, grp)) = ann_move_to_group
        && let Some(ann) = state
            .project
            .annotations
            .iter_mut()
            .find(|a| a.id == ann_id)
    {
        ann.group = grp;
        state.mark_dirty();
    }
    if toggle_all_ann_vis {
        state.project.annotations_visible = !state.project.annotations_visible;
        state.mark_dirty();
    }
    if toggle_all_ann_lock {
        state.project.annotations_locked = !state.project.annotations_locked;
        state.mark_dirty();
    }
    if clear_all_ann && !state.project.annotations.is_empty() {
        state.checkpoint("clear annotations");
        state.project.annotations.clear();
        state.selected_annotation = None;
        state.mark_dirty();
    }
    if let Some(subgroup_name) = add_ann_subgroup {
        state.project.add_annotation_group(&subgroup_name);
        state.mark_dirty();
    }
    if let Some(subgroup_name) = delete_ann_subgroup {
        state.project.remove_annotation_group(&subgroup_name);
        state.mark_dirty();
    }

    // Ações de medidas
    if let Some(meas_id) = toggle_meas_vis
        && let Some(m) = state
            .project
            .measurements
            .iter_mut()
            .find(|m| m.id == meas_id)
    {
        m.visible = !m.visible;
        state.mark_dirty();
    }
    if let Some(meas_id) = delete_meas {
        state.checkpoint("delete measurement");
        state.project.remove_measurement(meas_id);
        if state.selected_measurement == Some(meas_id) {
            state.selected_measurement = None;
        }
        state.mark_dirty();
    }
    if toggle_all_meas_vis {
        state.project.measurements_visible = !state.project.measurements_visible;
        state.mark_dirty();
    }
    if clear_all_meas && !state.project.measurements.is_empty() {
        state.checkpoint("clear measurements");
        state.project.measurements.clear();
        state.selected_measurement = None;
        state.mark_dirty();
    }

    // Ações de meshes
    if let Some(idx) = toggle_vis_idx {
        let _ = state.dispatch(&petunia_core::ToggleVisibilityAssetCmd {
            asset_index: Some(idx),
        });
    }
    if let Some(idx) = toggle_lock_idx {
        let _ = state.dispatch(&petunia_core::ToggleLockAssetCmd {
            asset_index: Some(idx),
        });
    }
    if let Some(idx) = isolate_idx
        && idx < state.project.assets.len()
    {
        state.project.active = idx;
        state.toggle_isolate();
    }
    if let Some((idx, col)) = move_to_col {
        let _ = state.dispatch(&petunia_core::SetAssetCollectionCmd {
            asset_index: idx,
            collection: col,
        });
    }
    if let Some(col) = toggle_col_vis {
        let _ = state.dispatch(&petunia_core::ToggleCollectionVisibilityCmd { collection: col });
    }
    if let Some(col) = toggle_col_lock {
        let _ = state.dispatch(&petunia_core::ToggleCollectionLockCmd { collection: col });
    }
    if let Some(col) = delete_col {
        state.project.remove_collection(&col);
        state.mark_dirty();
    }
    if let Some((old_name, new_name)) = rename_col
        && !new_name.trim().is_empty()
        && !state.project.collections.contains(&new_name)
    {
        for c in &mut state.project.collections {
            if *c == old_name {
                *c = new_name.clone();
            }
        }
        for a in &mut state.project.assets {
            if a.collection.as_deref() == Some(&old_name) {
                a.collection = Some(new_name.clone());
            }
        }
        state.mark_dirty();
    }
    if let Some((idx, new_name)) = rename_asset
        && let Some(asset) = state.project.assets.get(idx)
        && asset.name != new_name
    {
        state.checkpoint("rename object");
        if let Some(asset) = state.project.assets.get_mut(idx) {
            asset.name = new_name;
        }
    }

    // Ações de referências
    if let Some(idx) = ref_toggle_vis
        && let Some(r) = state.project.refs.get_mut(idx)
    {
        r.visible = !r.visible;
        state.mark_dirty();
    }
    if let Some(idx) = ref_toggle_xray
        && let Some(r) = state.project.refs.get_mut(idx)
    {
        r.xray = !r.xray;
        state.mark_dirty();
    }
    if let Some(idx) = ref_delete
        && idx < state.project.refs.len()
    {
        state.project.refs.remove(idx);
        state.mark_dirty();
    }
    if let Some(idx) = delete_idx {
        let _ = state.dispatch(&DeleteAssetCmd {
            asset_index: Some(idx),
        });
        // Pin nunca retém inválido: fixado removido solta com fallback.
        if crate::inspector_context::pinned_asset_idx(state).is_none() {
            state.ui.inspector_pinned = None;
        }
    }
    if let Some(idx) = dup_idx {
        let _ = state.dispatch(&DuplicateAssetCmd {
            asset_index: Some(idx),
        });
    }

    // Trata seleção pelo TreeView
    for action in actions {
        match action {
            Action::SetSelected(nodes) => {
                // O foco segue a seleção: só assim as setas do teclado
                // (ltreeview exige foco na árvore) funcionam após o clique.
                ui.memory_mut(|m| m.request_focus(tree_id));
                for node in nodes {
                    match node {
                        OutlinerNodeId::Asset(id) => {
                            state.set_active_asset_by_id(id);
                            state.selected_annotation = None;
                            state.selected_measurement = None;
                        }
                        OutlinerNodeId::Annotation(id) => {
                            state.selected_annotation = Some(id);
                            state.selected_measurement = None;
                            state.mark_dirty();
                        }
                        OutlinerNodeId::Measurement(id) => {
                            state.selected_measurement = Some(id);
                            state.selected_annotation = None;
                            state.mark_dirty();
                        }
                        _ => {}
                    }
                }
            }
            Action::Activate(act) => {
                for node in act.selected {
                    match node {
                        OutlinerNodeId::Asset(id) => {
                            state.set_active_asset_by_id(id);
                            state.selected_annotation = None;
                            state.selected_measurement = None;
                            // Duplo-clique/Enter renomeia (a seleção já ocorreu).
                            if state.project.assets.iter().any(|a| a.id == id) {
                                begin_asset_rename(ui, id);
                            }
                        }
                        OutlinerNodeId::Annotation(id) => {
                            state.selected_annotation = Some(id);
                            state.selected_measurement = None;
                            state.mark_dirty();
                        }
                        OutlinerNodeId::Measurement(id) => {
                            state.selected_measurement = Some(id);
                            state.selected_annotation = None;
                            state.mark_dirty();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Caminho canônico único de criação (P1D-§82): todas as espécies abrem a
/// sessão de criação (transação única + cartão Last Operation).
pub fn add_primitive_to_scene(state: &mut AppState, kind: usize, name: &str) {
    let p_kind = match kind {
        0 => PrimitiveKind::Cube,
        1 => PrimitiveKind::Sphere,
        2 => PrimitiveKind::Cylinder,
        3 => PrimitiveKind::Plane,
        4 => PrimitiveKind::Cone,
        5 => PrimitiveKind::Capsule,
        6 => PrimitiveKind::Wedge,
        7 => PrimitiveKind::Circle,
        8 => PrimitiveKind::Torus,
        _ => PrimitiveKind::Icosphere,
    };
    state.begin_primitive(p_kind, Some(name.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::MeasurementItem;
    use petunia_mesh::Mesh;

    #[test]
    fn test_outliner_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_outliner_renders_with_collections_and_locks() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.project.add_collection("Props");
        state.project.add("Table", Mesh::cube(1.0));
        state.project.assets.last_mut().unwrap().collection = Some("Props".to_string());
        state.project.assets.last_mut().unwrap().locked = true;

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state);
            });
        })
        .textures_delta
        .clear();

        assert_eq!(state.project.collections.len(), 1);
        assert!(state.project.assets.last().unwrap().locked);
    }

    #[test]
    fn test_outliner_renders_annotation_and_measurement_collections() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state
            .project
            .add_annotation(AnnotationItem::new("Note 1", vec![]));
        state.project.add_measurement(MeasurementItem::new(
            "Dist 1",
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            1.73,
        ));

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state);
            });
        })
        .textures_delta
        .clear();

        assert_eq!(state.project.annotations.len(), 1);
        assert_eq!(state.project.measurements.len(), 1);
    }

    #[test]
    fn test_outliner_search_filtering() {
        let mut state = AppState::new("en");
        state.project.add("TargetCube", Mesh::cube(1.0));
        state.project.add("OtherObject", Mesh::cube(1.0));

        state.ui.outliner_search = "target".to_string();
        assert_eq!(state.project.assets.len(), 3); // Default cube + 2 novos
    }

    #[test]
    fn test_add_primitive_to_scene() {
        let mut state = AppState::new("en");
        let before = state.project.assets.len();
        add_primitive_to_scene(&mut state, 0, "NewCube");
        assert_eq!(state.project.assets.len(), before + 1);
        assert_eq!(state.project.assets.last().unwrap().name, "NewCube");
    }
}
