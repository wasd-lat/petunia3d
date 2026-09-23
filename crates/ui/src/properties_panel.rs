//! Painel de Propriedades do Petunia3D (`Properties Panel`).
//! Contém a barra vertical de abas canônicas (Tool, Render, Output, Object, Modifiers, Data, Material)
//! e formulários sanfonados com fidelidade estética ao Blender.svg.

use egui::{Color32, RichText, ScrollArea, Ui, vec2};
use petunia_config::text_id;
use petunia_core::{
    AppState, ClearSelectionCmd, InvertSelectionCmd, ModuleRegistry, PrimitiveKind, SelectAllCmd,
    SelectLinkedCmd, Workspace,
};
use petunia_module_model::ToolRegistry;
use petunia_project::{AlphaMode, Material, ShaderProfile};
use uuid::Uuid;

use crate::adapters::taffy_layout::{
    PetuniaResponsiveLayout, clamped_width, fill_remaining, responsive,
};
use crate::foundation::motion::PetuniaMotion;
use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::inspector_context::{
    InspectorContext, InspectorTab, inspected_asset_idx, pinned_asset_idx, toggle_pin,
};
use crate::inspector_widgets::{self};
use crate::tokens;
use crate::widgets::{self};

/// Renderiza o painel de propriedades: barra de contexto + inspector
/// contextual (abas textuais por contexto, sem rail de ícones).
pub fn draw(
    ui: &mut Ui,
    state: &mut AppState,
    _tools: &ToolRegistry,
    _registry: &mut ModuleRegistry,
) {
    puffin::profile_function!();
    let context = InspectorContext::resolve(state);
    match &context {
        InspectorContext::Annotation(id) => {
            draw_tab_annotation(ui, state, *id);
            return;
        }
        InspectorContext::Measurement(id) => {
            draw_tab_measurement(ui, state, *id);
            return;
        }
        _ => {}
    }
    // PAINT usa o mesmo inspector do MODEL: no layout de pintura o pincel mora no
    // cartão flutuante da ferramenta (§34) e a coluna do dock mostra camadas
    // (topo) + inspector do objeto (base). Só o UV legado tem painel próprio.
    if !matches!(state.workspace, Workspace::Model | Workspace::Paint) {
        draw_workspace_inspector(ui, state);
        return;
    }
    // Barra do objeto ativo (contexto acima das abas, em toda aba).
    // Mesh: inspecionado (fixado ou ativo). Componentes: sempre o ativo
    // (a seleção vive na malha ativa; pin não desvia ferramenta).
    let bar_idx = match &context {
        InspectorContext::ComponentSelection { .. } => {
            (!state.project.assets.is_empty()).then(|| {
                state
                    .project
                    .active
                    .min(state.project.assets.len().saturating_sub(1))
            })
        }
        _ => inspected_asset_idx(state),
    };
    draw_object_bar(ui, state, bar_idx);
    if state.ui.inspector_collapsed {
        return;
    }
    // Wave 7 (§49): a busca do inspector abre animando a altura, como a do
    // Outliner — mesmo motivo e mesma duração.
    let search_open = state.ui.inspector_search_open;
    PetuniaMotion::section(ui, "inspector-search", search_open, |ui| {
        ui.add_space(2.0);
        let search_hint = state.t("inspector.search");
        let close_tip = state.t("ui.close");
        ui.horizontal(|ui| {
            widgets::petunia_search_box(ui, &mut state.ui.inspector_search, &search_hint);
            if widgets::PetuniaIconButton::new(PetuniaIcon::Close, &close_tip, 20.0)
                .show(ui)
                .clicked()
            {
                state.ui.inspector_search_open = false;
                state.ui.inspector_search.clear();
                state.mark_dirty();
            }
        });
    });
    ui.add_space(2.0);
    draw_model_inspector(ui, state, &context);
}

/// Lado de cada [`widgets::PetuniaIconButton`] da faixa de ações da barra do
/// objeto (pin, busca, olho, cadeado).
const OBJECT_BAR_ACTION_BTN: f32 = 20.0;
/// Quantos botões a faixa de ações tem.
const OBJECT_BAR_ACTION_COUNT: usize = 4;
/// Menor largura em que o campo de nome do objeto continua legível.
const OBJECT_BAR_NAME_MIN_W: f32 = 60.0;

/// Largura reservada à faixa de ações da barra do objeto.
///
/// Derivada do tamanho real dos botões e do vão entre eles — não de um número
/// solto. Era `(largura − 108)`, com o 108 escolhido a dedo.
const fn object_bar_actions_width() -> f32 {
    OBJECT_BAR_ACTION_BTN * OBJECT_BAR_ACTION_COUNT as f32
        + crate::foundation::spacing::RELATED * (OBJECT_BAR_ACTION_COUNT + 1) as f32
}

/// Barra do objeto ativo: colapso + ícone + nome editável + busca + pin +
/// olho + cadeado.
///
/// Substitui a linha de identidade dentro da aba Object (uma fonte única,
/// visível em qualquer aba, como o cabeçalho do Properties do Blender).
/// Opera sobre `idx` explícito (inspecionado ou ativo, conforme o contexto).
fn draw_object_bar(ui: &mut Ui, state: &mut AppState, idx: Option<usize>) {
    let collapsed = state.ui.inspector_collapsed;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
        let tip = state.t(if collapsed {
            "ui.expand"
        } else {
            "ui.collapse"
        });
        if widgets::chevron_toggle(ui, &tip, !collapsed).clicked() {
            state.ui.inspector_collapsed = !collapsed;
            state.mark_dirty();
        }
        let (icon_rect, icon_resp) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
        IconRegistry::paint(
            ui.ctx(),
            ui.painter(),
            &PetuniaIcon::ObjectMesh,
            icon_rect,
            tokens::ACCENT_BLUE,
        );
        if state.project.assets.is_empty() {
            ui.label(
                RichText::new(state.t("empty.scene_empty"))
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
            return;
        }
        // Opera sobre o inspecionado (fixado ou ativo): a barra é o contexto.
        let idx = idx.filter(|i| *i < state.project.assets.len()).unwrap_or(0);
        if let Some(asset) = state.project.assets.get(idx) {
            icon_resp.on_hover_text(format!(
                "Mesh · {} {} · {} {}",
                asset.mesh.vert_count(),
                state.t("props.verts"),
                asset.mesh.tri_count(),
                state.t("props.faces")
            ));
        }
        let asset_id = state.project.assets[idx].id;
        let visible = state.project.assets[idx].visible;
        let locked = state.project.assets[idx].locked;

        // Buffer de rename por asset (vazio + dica: sem armadilha de anexar).
        let id_key = egui::Id::new("petunia_rename_asset_id");
        let buf_key = egui::Id::new("petunia_rename_asset_buf");
        let mut buf = ui.ctx().data_mut(|d| {
            let cur = d.get_temp::<String>(id_key).unwrap_or_default();
            if cur == asset_id.to_string() {
                d.get_temp::<String>(buf_key).unwrap_or_default()
            } else {
                String::new()
            }
        });
        // O campo de nome recebe o que sobra depois da faixa de ações — quem
        // mede é o adapter, não o painel (§45, Wave 3).
        let edit_w = fill_remaining(ui, object_bar_actions_width(), OBJECT_BAR_NAME_MIN_W);
        let resp = ui.add_sized(
            vec2(edit_w, 20.0),
            egui::TextEdit::singleline(&mut buf).hint_text(state.project.assets[idx].name.clone()),
        );
        ui.ctx().data_mut(|d| {
            d.insert_temp(id_key, asset_id.to_string());
            d.insert_temp(buf_key, buf.clone());
        });
        let commit = resp.lost_focus()
            || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if commit {
            let new = buf.trim().to_string();
            if !new.is_empty() && new != state.project.assets[idx].name {
                state.checkpoint("rename object");
                if let Some(a) = state.project.assets.iter_mut().find(|a| a.id == asset_id) {
                    a.name = new;
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let lock_tip = state.t(if locked {
                "context.unlock"
            } else {
                "context.lock_tip"
            });
            let lock_icon = if locked {
                PetuniaIcon::Lock
            } else {
                PetuniaIcon::Unlock
            };
            if widgets::PetuniaIconButton::new(lock_icon, &lock_tip, 20.0)
                .show(ui)
                .clicked()
            {
                if let Some(a) = state.project.assets.get_mut(idx) {
                    a.locked = !locked;
                }
                state.mark_dirty();
            }
            let eye_tip = state.t(if visible {
                "context.hide"
            } else {
                "context.show"
            });
            let eye_icon = if visible {
                PetuniaIcon::Eye
            } else {
                PetuniaIcon::EyeHidden
            };
            if widgets::PetuniaIconButton::new(eye_icon, &eye_tip, 20.0)
                .show(ui)
                .clicked()
            {
                if let Some(a) = state.project.assets.get_mut(idx) {
                    a.visible = !visible;
                }
                state.mark_dirty();
            }
            // Pin: fixa o inspecionado (óbvio: preenchido + tooltip explicativa).
            let pinned = pinned_asset_idx(state) == Some(idx);
            if widgets::PetuniaIconButton::new(
                PetuniaIcon::Pin,
                &state.t(if pinned {
                    "inspector.unpin_tip"
                } else {
                    "inspector.pin_tip"
                }),
                20.0,
            )
            .selected(pinned)
            .show(ui)
            .clicked()
            {
                toggle_pin(state);
            }
            // Busca de propriedades (revelação progressiva).
            if widgets::PetuniaIconButton::new(
                PetuniaIcon::Search,
                &state.t("inspector.search"),
                20.0,
            )
            .selected(state.ui.inspector_search_open)
            .show(ui)
            .clicked()
            {
                state.ui.inspector_search_open = !state.ui.inspector_search_open;
                if !state.ui.inspector_search_open {
                    state.ui.inspector_search.clear();
                }
                state.mark_dirty();
            }
        });
    });
}

/// Aba Object: Transform + Geometry + Modifiers + Display (+ refs).
fn draw_object_sections(ui: &mut Ui, state: &mut AppState) {
    let gap = inspector_widgets::section_gap(state.ui.density);
    let idx = inspected_asset_idx(state);
    draw_transform_section(ui, state, idx, false);
    ui.add_space(gap);
    draw_geometry_section(ui, state, idx, false);
    ui.add_space(gap);
    draw_boolean_section(ui, state);
    ui.add_space(gap);
    draw_display_section(ui, state, idx, false);
    if !state.project.refs.is_empty() {
        ui.add_space(gap);
        crate::refs_section(ui, state);
    }
}

/// Operand Boolean, modificador Keep Parts e operações do core compartilhado.
fn draw_boolean_section(ui: &mut Ui, state: &mut AppState) {
    let title = state.t_id(text_id::BOOLEAN_TITLE);
    let operand_id = state.boolean_operand;
    let operand = operand_id.and_then(|id| {
        state
            .project
            .assets
            .iter()
            .find(|asset| asset.id == id)
            .map(|asset| (id, asset.name.clone()))
    });
    let summary = operand.as_ref().map(|(_, name)| name.as_str());
    let mut command_to_execute = None;

    inspector_widgets::section(
        ui,
        state.ui.density,
        "boolean",
        inspector_widgets::SectionOpts {
            title: &title,
            summary,
            default_open: true,
            force_open: false,
        },
        |ui| {
            if let Some((_, name)) = operand.as_ref() {
                ui.label(format!("{}: {name}", state.t_id(text_id::BOOLEAN_OPERAND)));

                let mut keep_parts = state.boolean_keep_parts;
                if ui
                    .checkbox(&mut keep_parts, state.t_id(text_id::BOOLEAN_KEEP_PARTS))
                    .changed()
                {
                    state.boolean_keep_parts = keep_parts;
                    state.mark_dirty();
                }

                ui.horizontal_wrapped(|ui| {
                    for (command, label) in [
                        ("model.fuse", text_id::BOOLEAN_FUSE),
                        ("model.cut", text_id::BOOLEAN_CUT),
                        ("model.intersect", text_id::BOOLEAN_INTERSECT),
                        ("model.join", text_id::BOOLEAN_JOIN),
                    ] {
                        let enabled = state.commands.can_execute(command, state).is_ok();
                        let label_text = state.t_id(label);
                        let response = ui
                            .add_enabled_ui(enabled, |ui| {
                                widgets::petunia_action_button(ui, None, &label_text, false)
                            })
                            .inner;
                        if response.clicked() {
                            command_to_execute = Some(command);
                        }
                    }
                });
            } else {
                ui.small(state.t_id(text_id::BOOLEAN_NO_OPERAND));
            }

            if operand.is_some() || operand_id.is_some() {
                let clear_label = state.t_id(text_id::BOOLEAN_CLEAR_OPERAND);
                if widgets::petunia_action_button(ui, None, &clear_label, false).clicked() {
                    state.boolean_operand = None;
                    state.mark_dirty();
                }
            }
        },
    );

    if let Some(command) = command_to_execute
        && state.dispatch_command(command).is_err()
    {
        state.set_status(state.t_id(text_id::BOOLEAN_COMMAND_FAILED));
    }
}

/// Limites da malha (todas as verts: dimensões do objeto).
fn mesh_bounds(verts: &[petunia_mesh::Vertex]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in verts {
        for a in 0..3 {
            min[a] = min[a].min(v.pos[a]);
            max[a] = max[a].max(v.pos[a]);
        }
    }
    if !min[0].is_finite() {
        min = [0.0; 3];
        max = [0.0; 3];
    }
    (min, max)
}

/// Seção Transform redesenhada: Position absoluta, Rotation relativa com
/// rascunho, Scale por eixo com link, Dimensions absolutas — tudo com undo
/// por sessão de edição (1 nível por gesto) e layout pela largura real.
fn draw_transform_section(ui: &mut Ui, state: &mut AppState, idx: Option<usize>, force_open: bool) {
    let Some(idx) = idx.filter(|i| state.project.assets.len() > *i) else {
        return;
    };
    let axis_names = ["X", "Y", "Z"];
    let axis_colors = [tokens::AXIS_X, tokens::AXIS_Y, tokens::AXIS_Z];
    let section_title = state.t("transform.title");
    inspector_widgets::section(
        ui,
        state.ui.density,
        "transform",
        inspector_widgets::SectionOpts {
            title: &section_title,
            summary: None,
            default_open: true,
            force_open,
        },
        |ui| {
            // --- Position (absoluta: centróide da seleção ou malha).
            ui.label(
                RichText::new(state.t("transform.position"))
                    .strong()
                    .size(11.0),
            );
            let center = state
                .project
                .assets
                .get(idx)
                .map(|a| a.mesh.selection_center())
                .unwrap_or([0.0; 3]);
            let mut edit = center;
            let pos_opts = inspector_widgets::NumericOpts::plain(0.05, 3);
            let ev = inspector_widgets::Vector3Field {
                axis_names,
                axis_colors,
                values: &mut edit,
                opts: pos_opts,
                session_key: "tx_pos",
                undo_label: "move object",
            }
            .show(ui, state);
            if ev.changed[0] || ev.changed[1] || ev.changed[2] {
                let delta = [
                    edit[0] - center[0],
                    edit[1] - center[1],
                    edit[2] - center[2],
                ];
                if let Some(asset) = state.project.assets.get_mut(idx) {
                    asset.mesh.translate_selected(delta);
                }
                state.emit_mesh_changed();
                state.mark_dirty();
            }

            ui.add_space(4.0);

            // --- Rotation (relativa: rascunho acumulador, sem TRS no modelo).
            ui.label(
                RichText::new(state.t("transform.rotation"))
                    .strong()
                    .size(11.0),
            );
            ui.small(state.t("transform.relative_hint"));
            let mut scratch = state.transform_rotation;
            let prev_rot = scratch;
            let rot_opts = inspector_widgets::NumericOpts {
                speed: 1.0,
                decimals: 1,
                suffix: "°",
                min: -3600.0,
                max: 3600.0,
            };
            let ev = inspector_widgets::Vector3Field {
                axis_names,
                axis_colors,
                values: &mut scratch,
                opts: rot_opts,
                session_key: "tx_rot",
                undo_label: "rotate object",
            }
            .show(ui, state);
            if ev.changed[0] || ev.changed[1] || ev.changed[2] {
                let delta_deg = [
                    scratch[0] - prev_rot[0],
                    scratch[1] - prev_rot[1],
                    scratch[2] - prev_rot[2],
                ];
                let center = state
                    .project
                    .assets
                    .get(idx)
                    .map(|a| a.mesh.selection_center())
                    .unwrap_or([0.0; 3]);
                if let Some(asset) = state.project.assets.get_mut(idx) {
                    asset.mesh.rotate_selected(
                        [
                            delta_deg[0].to_radians(),
                            delta_deg[1].to_radians(),
                            delta_deg[2].to_radians(),
                        ],
                        center,
                    );
                }
                state.transform_rotation = scratch;
                state.emit_mesh_changed();
                state.mark_dirty();
            }

            ui.add_space(4.0);

            // --- Scale (por eixo + link) + Dimensions (bbox absoluta).
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(state.t("transform.scale"))
                        .strong()
                        .size(11.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let linked = state.scale_linked;
                    if widgets::PetuniaIconButton::new(
                        if linked {
                            PetuniaIcon::Lock
                        } else {
                            PetuniaIcon::Unlock
                        },
                        &state.t(if linked {
                            "transform.unlink"
                        } else {
                            "transform.link"
                        }),
                        20.0,
                    )
                    .selected(linked)
                    .show(ui)
                    .clicked()
                    {
                        state.scale_linked = !linked;
                        state.mark_dirty();
                    }
                });
            });
            let mut factors = state.scale_factors;
            let prev_factors = factors;
            let scale_opts = inspector_widgets::NumericOpts {
                speed: 0.01,
                decimals: 3,
                suffix: "",
                min: 0.01,
                max: 100.0,
            };
            let ev = inspector_widgets::Vector3Field {
                axis_names,
                axis_colors,
                values: &mut factors,
                opts: scale_opts,
                session_key: "tx_scale",
                undo_label: "scale object",
            }
            .show(ui, state);
            if ev.changed[0] || ev.changed[1] || ev.changed[2] {
                let center = state
                    .project
                    .assets
                    .get(idx)
                    .map(|a| a.mesh.selection_center())
                    .unwrap_or([0.0; 3]);
                // Eixo editado (primeiro com mudança) vira referência.
                let edited = ev.changed.iter().position(|c| *c).unwrap_or(0);
                let ratio_of = |new_v: f32, old_v: f32| {
                    if old_v.abs() < 1e-6 {
                        new_v
                    } else {
                        new_v / old_v
                    }
                };
                if state.scale_linked {
                    let ratio = ratio_of(factors[edited], prev_factors[edited]);
                    if let Some(asset) = state.project.assets.get_mut(idx) {
                        asset
                            .mesh
                            .scale_selected_factors([ratio, ratio, ratio], center);
                    }
                    factors = [factors[edited], factors[edited], factors[edited]];
                } else {
                    let ratios = [
                        ratio_of(factors[0], prev_factors[0]),
                        ratio_of(factors[1], prev_factors[1]),
                        ratio_of(factors[2], prev_factors[2]),
                    ];
                    if let Some(asset) = state.project.assets.get_mut(idx) {
                        asset.mesh.scale_selected_factors(ratios, center);
                    }
                }
                state.scale_factors = factors;
                state.emit_mesh_changed();
                state.mark_dirty();
            }

            ui.add_space(2.0);
            ui.label(
                RichText::new(state.t("transform.dimensions"))
                    .strong()
                    .size(11.0),
            );
            let (dims, center) = match state.project.assets.get(idx) {
                Some(asset) => {
                    let (min, max) = mesh_bounds(&asset.mesh.verts);
                    (
                        [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
                        asset.mesh.selection_center(),
                    )
                }
                None => ([0.0; 3], [0.0; 3]),
            };
            let mut edit_dims = dims;
            let dims_opts = inspector_widgets::NumericOpts {
                speed: 0.05,
                decimals: 3,
                suffix: "",
                min: 0.001,
                max: f32::INFINITY,
            };
            let ev = inspector_widgets::Vector3Field {
                axis_names,
                axis_colors,
                values: &mut edit_dims,
                opts: dims_opts,
                session_key: "tx_dims",
                undo_label: "resize object",
            }
            .show(ui, state);
            if ev.changed[0] || ev.changed[1] || ev.changed[2] {
                let ratios = [
                    if dims[0].abs() < 1e-6 {
                        1.0
                    } else {
                        edit_dims[0] / dims[0]
                    },
                    if dims[1].abs() < 1e-6 {
                        1.0
                    } else {
                        edit_dims[1] / dims[1]
                    },
                    if dims[2].abs() < 1e-6 {
                        1.0
                    } else {
                        edit_dims[2] / dims[2]
                    },
                ];
                if let Some(asset) = state.project.assets.get_mut(idx) {
                    asset.mesh.scale_selected_factors(ratios, center);
                }
                state.emit_mesh_changed();
                state.mark_dirty();
            }
        },
    );
}

/// Seção Geometry: resumo sempre visível (colapsada mostra contagens).
fn draw_geometry_section(ui: &mut Ui, state: &mut AppState, idx: Option<usize>, force_open: bool) {
    let summary = idx.and_then(|i| state.project.assets.get(i)).map(|a| {
        format!(
            "{} {} · {} {}",
            a.mesh.vert_count(),
            state.t("props.verts"),
            a.mesh.tri_count(),
            state.t("props.faces")
        )
    });
    let section_title = state.t("geometry.title");
    inspector_widgets::section(
        ui,
        state.ui.density,
        "geometry",
        inspector_widgets::SectionOpts {
            title: &section_title,
            summary: summary.as_deref(),
            default_open: false,
            force_open,
        },
        |ui| {
            if let Some(i) = idx
                && let Some(mesh) = state.project.assets.get(i).map(|a| &a.mesh)
            {
                ui.label(format!("{}: {}", state.t("props.verts"), mesh.vert_count()));
                ui.label(format!("{}: {}", state.t("props.faces"), mesh.faces.len()));
                ui.label(format!(
                    "{}: {}",
                    state.t("geometry.tris"),
                    mesh.tri_count()
                ));
            }
        },
    );
}

/// Seção Modifiers: pilha persistente, não destrutiva e reordenável.
fn draw_modifiers_section(ui: &mut Ui, state: &mut AppState, force_open: bool) {
    use petunia_project::{ModifierInstance, ModifierKind};

    let section_title = state.t("modifiers.title");
    inspector_widgets::section(
        ui,
        state.ui.density,
        "modifiers",
        inspector_widgets::SectionOpts {
            title: &section_title,
            summary: None,
            default_open: false,
            force_open,
        },
        |ui| {
            let Some(asset_idx) =
                inspected_asset_idx(state).filter(|idx| *idx < state.project.assets.len())
            else {
                ui.label(RichText::new(state.t("empty.no_selection")).color(tokens::TEXT_MUTED));
                return;
            };

            // Precompute localized strings before mutably borrowing a modifier.
            let enable_tip = state.t("modifiers.enable");
            let remove_tip = state.t("modifiers.remove");
            let axis_label = state.t("modifiers.axis");
            let weld_label = state.t("actions.weld_eps");
            let mirror_title = state.t("tools.mirror");
            let symmetry_title = state.t("tools.symmetrize");
            let add_mirror = state.t("modifiers.add_mirror");
            let add_symmetry = state.t("modifiers.add_symmetry");
            let move_up_tip = state.t("toolbar.move_up");
            let move_down_tip = state.t("toolbar.move_down");
            let positive_to_negative = state.t("actions.symmetrize_dir_pos");
            let negative_to_positive = state.t("actions.symmetrize_dir_neg");

            let mut changed = false;
            let mut remove = None;
            let mut move_up = None;
            let mut move_down = None;
            let modifier_count = state.project.assets[asset_idx].modifiers.len();
            ui.push_id("modifier_stack_rows", |ui| {
                for modifier_idx in 0..modifier_count {
                    let snapshot = state.project.assets[asset_idx].modifiers[modifier_idx].clone();
                    egui::Frame::new()
                        .fill(tokens::bg_surface(state))
                        .stroke(tokens::stroke_border_dyn(state))
                        .corner_radius(tokens::RADIUS_CONTAINER)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let mut enabled = snapshot.enabled;
                                if ui
                                    .checkbox(&mut enabled, "")
                                    .on_hover_text(&enable_tip)
                                    .changed()
                                {
                                    state.project.assets[asset_idx].modifiers[modifier_idx]
                                        .enabled = enabled;
                                    changed = true;
                                }
                                let title = match snapshot.kind {
                                    ModifierKind::Mirror { .. } => &mirror_title,
                                    ModifierKind::Symmetry { .. } => &symmetry_title,
                                };
                                ui.strong(title);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("×").on_hover_text(&remove_tip).clicked()
                                        {
                                            remove = Some(modifier_idx);
                                        }
                                        if ui
                                            .add_enabled(
                                                modifier_idx + 1 < modifier_count,
                                                egui::Button::new("↓"),
                                            )
                                            .on_hover_text(&move_down_tip)
                                            .clicked()
                                        {
                                            move_down = Some(modifier_idx);
                                        }
                                        if ui
                                            .add_enabled(modifier_idx > 0, egui::Button::new("↑"))
                                            .on_hover_text(&move_up_tip)
                                            .clicked()
                                        {
                                            move_up = Some(modifier_idx);
                                        }
                                    },
                                );
                            });

                            match &mut state.project.assets[asset_idx].modifiers[modifier_idx].kind
                            {
                                ModifierKind::Mirror { axis, weld } => {
                                    ui.horizontal(|ui| {
                                        ui.label(&axis_label);
                                        for (candidate, name) in [(0, "X"), (1, "Y"), (2, "Z")] {
                                            if ui
                                                .selectable_label(*axis == candidate, name)
                                                .clicked()
                                            {
                                                *axis = candidate;
                                                changed = true;
                                            }
                                        }
                                    });
                                    changed |= ui
                                        .add(egui::Slider::new(weld, 0.0..=0.05).text(&weld_label))
                                        .changed();
                                }
                                ModifierKind::Symmetry {
                                    axis,
                                    positive_to_negative: direction,
                                    weld,
                                } => {
                                    ui.horizontal(|ui| {
                                        ui.label(&axis_label);
                                        for (candidate, name) in [(0, "X"), (1, "Y"), (2, "Z")] {
                                            if ui
                                                .selectable_label(*axis == candidate, name)
                                                .clicked()
                                            {
                                                *axis = candidate;
                                                changed = true;
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(*direction, &positive_to_negative)
                                            .clicked()
                                        {
                                            *direction = true;
                                            changed = true;
                                        }
                                        if ui
                                            .selectable_label(!*direction, &negative_to_positive)
                                            .clicked()
                                        {
                                            *direction = false;
                                            changed = true;
                                        }
                                    });
                                    changed |= ui
                                        .add(egui::Slider::new(weld, 0.0..=0.05).text(&weld_label))
                                        .changed();
                                }
                            }
                        });
                    ui.add_space(4.0);
                }
            });

            if let Some(index) = remove {
                state.project.assets[asset_idx].modifiers.remove(index);
                changed = true;
            } else if let Some(index) = move_up {
                state.project.assets[asset_idx]
                    .modifiers
                    .swap(index, index - 1);
                changed = true;
            } else if let Some(index) = move_down {
                state.project.assets[asset_idx]
                    .modifiers
                    .swap(index, index + 1);
                changed = true;
            }

            if modifier_count == 0 {
                ui.label(
                    RichText::new(state.t("modifiers.empty"))
                        .size(11.0)
                        .color(tokens::TEXT_MUTED),
                );
            }

            // O limiar de linha estreita (largura < 220px, com os botões
            // empilhados à mão) virou arranjo responsivo: os botões mantêm o
            // tamanho natural e o wrap do layout decide se cabem lado a lado.
            // Nenhum breakpoint em pixels aqui, e o rótulo traduzido longo (ou
            // uma escala grande) não estoura mais.
            let actions_id = ui.id().with("modifier_actions");
            responsive(
                ui,
                actions_id,
                PetuniaResponsiveLayout::wrap_row(),
                2,
                |index, ui| {
                    let label = if index == 0 {
                        &add_mirror
                    } else {
                        &add_symmetry
                    };
                    if ui.button(label).clicked() {
                        state.project.assets[asset_idx]
                            .modifiers
                            .push(if index == 0 {
                                ModifierInstance::mirror(0, 0.001)
                            } else {
                                ModifierInstance::symmetry(0, true, 0.001)
                            });
                        changed = true;
                    }
                },
                |_ui| (),
            );

            if changed {
                state.emit_mesh_changed();
                state.mark_dirty();
            }
        },
    );
}

/// Seção Display: visibilidade + bloqueio (mesma semântica do outliner).
fn draw_display_section(ui: &mut Ui, state: &mut AppState, idx: Option<usize>, force_open: bool) {
    let Some(idx) = idx.filter(|i| state.project.assets.len() > *i) else {
        return;
    };
    let section_title = state.t("display.title");
    inspector_widgets::section(
        ui,
        state.ui.density,
        "display",
        inspector_widgets::SectionOpts {
            title: &section_title,
            summary: None,
            default_open: false,
            force_open,
        },
        |ui| {
            let mut visible = state.project.assets[idx].visible;
            if ui.checkbox(&mut visible, state.t("refs.visible")).changed() {
                if let Some(asset) = state.project.assets.get_mut(idx) {
                    asset.visible = visible;
                }
                state.mark_dirty();
            }
            let mut locked = state.project.assets[idx].locked;
            if ui.checkbox(&mut locked, state.t("refs.lock")).changed() {
                if let Some(asset) = state.project.assets.get_mut(idx) {
                    asset.locked = locked;
                }
                state.mark_dirty();
            }
        },
    );
}

/// Aba Selection: contagens + operações de seleção + Transform do alvo.
fn draw_selection_tab(ui: &mut Ui, state: &mut AppState) {
    let (sv, se, sf) = (
        state.selection.verts.len(),
        state.selection.edges.len(),
        state.selection.faces.len(),
    );
    if sv + se + sf == 0 {
        ui.label(
            RichText::new(state.t("empty.no_selection"))
                .size(11.0)
                .color(tokens::TEXT_MUTED),
        );
        ui.small(state.t("empty.no_selection_hint"));
        return;
    }
    ui.label(
        RichText::new(format!(
            "{sv} {} · {se} {} · {sf} {} {}",
            state.t("props.verts"),
            state.t("props.edges"),
            state.t("props.faces"),
            state.t("selection.selected")
        ))
        .size(11.5)
        .strong(),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button(state.t("actions.select_all")).clicked() {
            let _ = state.dispatch(&SelectAllCmd);
        }
        if ui.button(state.t("actions.invert")).clicked() {
            let _ = state.dispatch(&InvertSelectionCmd);
        }
        if ui.button(state.t("selection.clear")).clicked() {
            let _ = state.dispatch(&ClearSelectionCmd);
        }
        if ui.button(state.t("actions.select_linked")).clicked() {
            let _ = state.dispatch(&SelectLinkedCmd);
        }
    });
    ui.add_space(4.0);
    let idx = (!state.project.assets.is_empty()).then(|| {
        state
            .project
            .active
            .min(state.project.assets.len().saturating_sub(1))
    });
    draw_transform_section(ui, state, idx, false);
}

/// Aba Modifiers: propriedades persistentes do objeto. Tool Properties vivem no viewport.
fn draw_modify_tab(ui: &mut Ui, state: &mut AppState) {
    draw_modifiers_section(ui, state, true);
}

/// Cena vazia: atalhos reais de criação (operam de imediato).
pub(crate) fn draw_quick_add(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        RichText::new(state.t("empty.scene_empty"))
            .strong()
            .size(12.0)
            .color(tokens::TEXT_PRIMARY),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for (label_key, kind) in [
            ("prims.cube", PrimitiveKind::Cube),
            ("prims.sphere", PrimitiveKind::Sphere),
            ("prims.plane", PrimitiveKind::Plane),
        ] {
            if ui.button(state.t(label_key)).clicked() {
                state.begin_primitive(kind, None);
            }
        }
        if ui.button(state.t("file.import_obj")).clicked() {
            crate::import_obj_dialog(state);
        }
    });
}

/// Conteúdo rolável do inspector no workspace Model (contextual).
fn draw_model_inspector(ui: &mut Ui, state: &mut AppState, context: &InspectorContext) {
    ScrollArea::vertical()
        .id_salt("properties_content_scroll")
        // Largura cheia: `[true, _]` deixaria o conteúdo ditar a largura do
        // painel e cada arredondamento de pixel fazia o dock crescer 1px/frame.
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_enabled_ui(!state.is_interacting(), |ui| {
                match context {
                    InspectorContext::EmptyScene => draw_empty_scene_inspector(ui, state),
                    InspectorContext::MeshObject { .. }
                    | InspectorContext::ComponentSelection { .. } => {
                        draw_context_inspector(ui, state, context)
                    }
                    InspectorContext::Annotation(_) | InspectorContext::Measurement(_) => {}
                }

                if state.ui.show_help {
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(state.t("ui.help"))
                        .default_open(false)
                        .show(ui, |ui| {
                            #[cfg(feature = "help-markdown")]
                            crate::help_markdown::render_help(ui, &state.t("help.body"));
                            #[cfg(not(feature = "help-markdown"))]
                            ui.label(state.t("help.body"));
                        });
                }
            });
        });
}

/// Cena vazia: resumo real + atalhos de criação.
fn draw_empty_scene_inspector(ui: &mut Ui, state: &mut AppState) {
    inspector_widgets::block_header(ui, PetuniaIcon::Collection, &state.t("scene.title"));
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "{}: {}",
            state.t("props.faces"),
            state.scene_tris()
        ))
        .size(11.0)
        .color(tokens::TEXT_SECONDARY),
    );
    ui.add_space(4.0);
    draw_quick_add(ui, state);
}

/// Inspector do contexto com abas textuais (ou lista plana na busca).
fn draw_context_inspector(ui: &mut Ui, state: &mut AppState, context: &InspectorContext) {
    let query = state.ui.inspector_search.trim().to_lowercase();
    if !query.is_empty() {
        draw_inspector_search_results(ui, state, context, &query);
        return;
    }

    let tabs: Vec<(InspectorTab, String)> = context
        .tabs()
        .iter()
        .map(|tab| (*tab, state.t(tab.key())))
        .collect();
    let default_tab = context.default_tab().unwrap_or(InspectorTab::Object);
    let active_tab = context
        .sanitize_tab(&state.ui.properties_tab.clone())
        .unwrap_or(default_tab);
    if InspectorTab::from_id(&state.ui.properties_tab) != Some(active_tab) {
        state.ui.properties_tab = active_tab.id().to_string();
    }
    if let Some(picked) = inspector_widgets::context_tabs(ui, state.ui.density, &tabs, active_tab) {
        state.ui.properties_tab = picked.id().to_string();
        state.mark_dirty();
        return;
    }

    ui.add_space(4.0);
    match active_tab {
        InspectorTab::Object => draw_object_sections(ui, state),
        InspectorTab::Modify => draw_modify_tab(ui, state),
        InspectorTab::Material => draw_tab_material(ui, state),
        InspectorTab::Selection => draw_selection_tab(ui, state),
    }
}

/// Busca achata as seções de todas as abas (só o que casa aparece, aberto).
fn draw_inspector_search_results(
    ui: &mut Ui,
    state: &mut AppState,
    context: &InspectorContext,
    query: &str,
) {
    // Componentes vivem na malha ativa (pin não desvia ferramenta).
    let idx = match context {
        InspectorContext::ComponentSelection { .. } => {
            (!state.project.assets.is_empty()).then(|| {
                state
                    .project
                    .active
                    .min(state.project.assets.len().saturating_sub(1))
            })
        }
        _ => inspected_asset_idx(state),
    };
    let mut any = false;
    let mut show_matched = |state: &mut AppState, title_key: &str| -> bool {
        let hit = query.is_empty() || state.t(title_key).to_lowercase().contains(query);
        if hit {
            any = true;
        }
        hit
    };
    if show_matched(state, "transform.title") {
        draw_transform_section(ui, state, idx, true);
    }
    if show_matched(state, "geometry.title") {
        draw_geometry_section(ui, state, idx, true);
    }
    if show_matched(state, "modifiers.title") {
        draw_modifiers_section(ui, state, true);
    }
    if show_matched(state, "display.title") {
        draw_display_section(ui, state, idx, true);
    }
    if state
        .t("inspector.tab_material")
        .to_lowercase()
        .contains(query)
    {
        any = true;
        draw_tab_material(ui, state);
    }
    if !any {
        ui.label(
            RichText::new(state.t("inspector.no_results"))
                .size(11.0)
                .color(tokens::TEXT_MUTED),
        );
    }
}

/// Painel do workspace ativo fora do Model (Paint/UV, mais Animate quando a
/// feature `animation-workspace` está ligada), sem rail.
fn draw_workspace_inspector(ui: &mut Ui, state: &mut AppState) {
    ScrollArea::vertical()
        .id_salt("properties_workspace_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_enabled_ui(!state.is_interacting(), |ui| {
                match state.workspace {
                    // PAINT e o UV legado mostram a seção Pincel. No dock essa
                    // seção já é desenhada pelo shell; aqui ela atende o
                    // inspector destacado — conteúdo único, sem segunda cópia.
                    Workspace::Paint | Workspace::Uv => {
                        crate::modules_ui::paint_ui::draw_brush_contents(ui, state);
                    }
                    #[cfg(feature = "animation-workspace")]
                    Workspace::Animate => {
                        crate::modules_ui::animation_ui::draw_animation_panel(ui, state);
                    }
                    Workspace::Model => {}
                }

                if state.ui.show_help {
                    egui::CollapsingHeader::new(state.t("ui.help"))
                        .default_open(false)
                        .show(ui, |ui| {
                            #[cfg(feature = "help-markdown")]
                            crate::help_markdown::render_help(ui, &state.t("help.body"));
                            #[cfg(not(feature = "help-markdown"))]
                            ui.label(state.t("help.body"));
                        });
                }
            });
        });
}

fn draw_tab_annotation(ui: &mut Ui, state: &mut AppState, ann_id: Uuid) {
    let Some(ann_idx) = state
        .project
        .annotations
        .iter()
        .position(|a| a.id == ann_id)
    else {
        ui.label(egui::RichText::new("Anotação selecionada não encontrada").italics());
        return;
    };

    let is_locked = state.project.annotations[ann_idx].locked || state.project.annotations_locked;

    // 1. Identidade da Anotação (Header ciano)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
        let (icon_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
        IconRegistry::paint(
            ui.ctx(),
            ui.painter(),
            &PetuniaIcon::Annotate,
            icon_rect,
            Color32::from_rgb(0, 210, 211),
        );
        ui.label(
            egui::RichText::new("Annotation")
                .color(Color32::from_rgb(0, 210, 211))
                .strong(),
        );
    });

    let mut name = state.project.annotations[ann_idx].name.clone();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Name")
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        let name_resp = ui.add(egui::TextEdit::singleline(&mut name).hint_text("Annotation Name"));
        if name_resp.changed() {
            state.project.annotations[ann_idx].name = name;
            state.mark_dirty();
        }
        if name_resp.lost_focus() {
            state.checkpoint("rename annotation");
        }
    });

    ui.add_space(4.0);

    // 2. Visibilidade e Bloqueio
    ui.horizontal(|ui| {
        let mut vis = state.project.annotations[ann_idx].visible;
        if ui.checkbox(&mut vis, "Visible").changed() {
            state.checkpoint("toggle annotation visibility");
            state.project.annotations[ann_idx].visible = vis;
            state.mark_dirty();
        }

        let mut locked = state.project.annotations[ann_idx].locked;
        if ui.checkbox(&mut locked, "Locked").changed() {
            state.checkpoint("toggle annotation lock");
            state.project.annotations[ann_idx].locked = locked;
            state.mark_dirty();
        }
    });

    // Subgrupo dentro da collection de Anotações
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Subgroup:")
                .size(11.0)
                .color(tokens::TEXT_MUTED),
        );
        let current_group = state.project.annotations[ann_idx].group.clone();
        let current_label = current_group.as_deref().unwrap_or("(Root / No subgroup)");

        let groups = state.project.annotation_groups.clone();
        egui::ComboBox::from_id_salt("annotation_group_selector")
            .width(clamped_width(ui, 96.0, 180.0))
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current_group.is_none(), "(Root / No subgroup)")
                    .clicked()
                {
                    state.checkpoint("move annotation to root");
                    state.project.annotations[ann_idx].group = None;
                    state.mark_dirty();
                }
                for grp in groups {
                    let is_sel = current_group.as_deref() == Some(&grp);
                    if ui.selectable_label(is_sel, &grp).clicked() {
                        state.checkpoint("change annotation group");
                        state.project.annotations[ann_idx].group = Some(grp);
                        state.mark_dirty();
                    }
                }
            });
    });

    ui.add_space(4.0);

    // 3. Aparência do Traço
    inspector_widgets::section(
        ui,
        state.ui.density,
        "ecs_stroke",
        inspector_widgets::SectionOpts {
            title: "Stroke Style",
            summary: None,
            default_open: true,
            force_open: false,
        },
        |ui| {
            let ann = &state.project.annotations[ann_idx];
            let mut color = ann
                .strokes
                .first()
                .map(|s| s.color)
                .unwrap_or([0.0, 0.74, 0.83, 1.0]);
            let mut width = ann.strokes.first().map(|s| s.width).unwrap_or(2.0);

            let mut color_changed = false;
            let mut width_changed = false;

            ui.horizontal(|ui| {
                ui.label("Color:");
                let resp = ui.color_edit_button_rgba_unmultiplied(&mut color);
                if resp.changed() {
                    color_changed = true;
                }
                if resp.drag_stopped() {
                    state.checkpoint("change annotation color");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Width:");
                let resp = ui.add(egui::Slider::new(&mut width, 0.5..=10.0).suffix(" px"));
                if resp.changed() {
                    width_changed = true;
                }
                if resp.drag_stopped() {
                    state.checkpoint("change annotation width");
                }
            });

            if color_changed {
                for s in &mut state.project.annotations[ann_idx].strokes {
                    s.color = color;
                }
                state.mark_dirty();
            }
            if width_changed {
                for s in &mut state.project.annotations[ann_idx].strokes {
                    s.width = width;
                }
                state.mark_dirty();
            }
        },
    );

    ui.add_space(4.0);

    // 4. Seção de Transformação (Location, Rotation, Scale)
    inspector_widgets::section(
        ui,
        state.ui.density,
        "ecs_annot_transform",
        inspector_widgets::SectionOpts {
            title: "Transform",
            summary: None,
            default_open: true,
            force_open: false,
        },
        |ui| {
            if is_locked {
                ui.label(
                    egui::RichText::new("Annotation locked against transformations")
                        .italics()
                        .color(tokens::TEXT_MUTED),
                );
            }

            ui.add_enabled_ui(!is_locked, |ui| {
                let mut trans = state.project.annotations[ann_idx].translation;
                let mut rot = state.project.annotations[ann_idx].rotation;
                let mut scale = state.project.annotations[ann_idx].scale;

                let mut changed = false;
                let mut stopped = false;

                // Location X, Y, Z
                ui.label(egui::RichText::new("Location").strong().size(11.0));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").color(tokens::AXIS_X).strong());
                    let r0 = ui.add(egui::DragValue::new(&mut trans[0]).speed(0.05));
                    ui.label(egui::RichText::new("Y").color(tokens::AXIS_Y).strong());
                    let r1 = ui.add(egui::DragValue::new(&mut trans[1]).speed(0.05));
                    ui.label(egui::RichText::new("Z").color(tokens::AXIS_Z).strong());
                    let r2 = ui.add(egui::DragValue::new(&mut trans[2]).speed(0.05));

                    if r0.changed() || r1.changed() || r2.changed() {
                        changed = true;
                    }
                    if r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped() {
                        stopped = true;
                    }
                });

                ui.add_space(2.0);

                // Rotation X, Y, Z (graus)
                ui.label(egui::RichText::new("Rotation").strong().size(11.0));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").color(tokens::AXIS_X).strong());
                    let r0 = ui.add(egui::DragValue::new(&mut rot[0]).speed(1.0).suffix("°"));
                    ui.label(egui::RichText::new("Y").color(tokens::AXIS_Y).strong());
                    let r1 = ui.add(egui::DragValue::new(&mut rot[1]).speed(1.0).suffix("°"));
                    ui.label(egui::RichText::new("Z").color(tokens::AXIS_Z).strong());
                    let r2 = ui.add(egui::DragValue::new(&mut rot[2]).speed(1.0).suffix("°"));

                    if r0.changed() || r1.changed() || r2.changed() {
                        changed = true;
                    }
                    if r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped() {
                        stopped = true;
                    }
                });

                ui.add_space(2.0);

                // Scale X, Y, Z
                ui.label(egui::RichText::new("Scale").strong().size(11.0));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").color(tokens::AXIS_X).strong());
                    let r0 = ui.add(
                        egui::DragValue::new(&mut scale[0])
                            .speed(0.02)
                            .range(0.01..=100.0),
                    );
                    ui.label(egui::RichText::new("Y").color(tokens::AXIS_Y).strong());
                    let r1 = ui.add(
                        egui::DragValue::new(&mut scale[1])
                            .speed(0.02)
                            .range(0.01..=100.0),
                    );
                    ui.label(egui::RichText::new("Z").color(tokens::AXIS_Z).strong());
                    let r2 = ui.add(
                        egui::DragValue::new(&mut scale[2])
                            .speed(0.02)
                            .range(0.01..=100.0),
                    );

                    if r0.changed() || r1.changed() || r2.changed() {
                        changed = true;
                    }
                    if r0.drag_stopped() || r1.drag_stopped() || r2.drag_stopped() {
                        stopped = true;
                    }
                });

                ui.add_space(4.0);

                if widgets::petunia_action_button(ui, None, "Reset Transform", false).clicked() {
                    state.checkpoint("reset annotation transform");
                    state.project.annotations[ann_idx].translation = [0.0, 0.0, 0.0];
                    state.project.annotations[ann_idx].rotation = [0.0, 0.0, 0.0];
                    state.project.annotations[ann_idx].scale = [1.0, 1.0, 1.0];
                    state.mark_dirty();
                }

                if changed {
                    state.project.annotations[ann_idx].translation = trans;
                    state.project.annotations[ann_idx].rotation = rot;
                    state.project.annotations[ann_idx].scale = scale;
                    state.mark_dirty();
                }
                if stopped {
                    state.checkpoint("transform annotation");
                }
            });
        },
    );

    ui.add_space(8.0);
    ui.separator();

    // 5. Ações (Deletar Anotação)
    ui.horizontal(|ui| {
        if widgets::petunia_action_button(ui, Some(PetuniaIcon::Delete), "Delete Annotation", true)
            .clicked()
        {
            state.checkpoint("delete annotation");
            state.project.remove_annotation(ann_id);
            state.selected_annotation = None;
            state.mark_dirty();
        }
    });
}

fn draw_tab_measurement(ui: &mut Ui, state: &mut AppState, meas_id: Uuid) {
    let Some(meas_idx) = state
        .project
        .measurements
        .iter()
        .position(|m| m.id == meas_id)
    else {
        ui.label(egui::RichText::new("Measurement not found").italics());
        return;
    };

    let meas = &state.project.measurements[meas_idx];
    let distance = meas.distance;
    let start = meas.start;
    let end = meas.end;
    let deltas = meas.deltas();
    let mut name = meas.name.clone();

    // 1. Identidade da Medição (Header amarelo)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
        let (icon_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
        IconRegistry::paint(
            ui.ctx(),
            ui.painter(),
            &PetuniaIcon::Measure,
            icon_rect,
            Color32::from_rgb(0xfe, 0xca, 0x57),
        );
        ui.label(
            egui::RichText::new("Measurement")
                .color(Color32::from_rgb(0xfe, 0xca, 0x57))
                .strong(),
        );
    });

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Name")
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        let name_resp = ui.add(egui::TextEdit::singleline(&mut name).hint_text("Measurement Name"));
        if name_resp.changed() {
            state.project.measurements[meas_idx].name = name;
            state.mark_dirty();
        }
        if name_resp.lost_focus() {
            state.checkpoint("rename measurement");
        }
    });

    ui.add_space(4.0);

    // 2. Opções: estritamente apenas ocultar ou deletar
    ui.horizontal(|ui| {
        let mut vis = state.project.measurements[meas_idx].visible;
        if ui.checkbox(&mut vis, "Visible").changed() {
            state.checkpoint("toggle measurement visibility");
            state.project.measurements[meas_idx].visible = vis;
            state.mark_dirty();
        }
    });

    ui.add_space(4.0);

    // 3. Leituras de Medição (Readouts)
    inspector_widgets::section(
        ui,
        state.ui.density,
        "ecs_measure",
        inspector_widgets::SectionOpts {
            title: "Measurement Values",
            summary: None,
            default_open: true,
            force_open: false,
        },
        |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Total Distance:").strong());
                ui.label(
                    egui::RichText::new(format!("{distance:.3} m"))
                        .color(Color32::from_rgb(0xfe, 0xca, 0x57))
                        .strong(),
                );
            });

            ui.add_space(2.0);

            ui.label(
                egui::RichText::new("Cartesian Deltas (|Δ|)")
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ΔX").color(tokens::AXIS_X).strong());
                ui.label(format!("{:.3} m", deltas[0]));
                ui.label(egui::RichText::new("ΔY").color(tokens::AXIS_Y).strong());
                ui.label(format!("{:.3} m", deltas[1]));
                ui.label(egui::RichText::new("ΔZ").color(tokens::AXIS_Z).strong());
                ui.label(format!("{:.3} m", deltas[2]));
            });

            ui.add_space(2.0);

            ui.label(
                egui::RichText::new("Coordinates")
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
            ui.label(format!(
                "Start: ({:.2}, {:.2}, {:.2})",
                start[0], start[1], start[2]
            ));
            ui.label(format!(
                "End:   ({:.2}, {:.2}, {:.2})",
                end[0], end[1], end[2]
            ));
        },
    );

    ui.add_space(8.0);
    ui.separator();

    // 4. Ações: estritamente deletar
    ui.horizontal(|ui| {
        if widgets::petunia_action_button(ui, Some(PetuniaIcon::Delete), "Delete Measurement", true)
            .clicked()
        {
            state.checkpoint("delete measurement");
            state.project.remove_measurement(meas_id);
            state.selected_measurement = None;
            state.mark_dirty();
        }
    });
}

fn draw_tab_material(ui: &mut Ui, state: &mut AppState) {
    if state.project.assets.is_empty() {
        draw_quick_add(ui, state);
        return;
    }

    let active_idx = state.project.active.min(state.project.assets.len() - 1);
    let active_mat_id = state.project.assets[active_idx].material_id;

    inspector_widgets::section(
        ui,
        state.ui.density,
        "ecs_material",
        inspector_widgets::SectionOpts {
            title: "Material PBR (P3D-050)",
            summary: None,
            default_open: true,
            force_open: false,
        },
        |ui| {
            // 1. Slot de Material e Seletor
            ui.horizontal(|ui| {
                ui.label("Material:");
                let current_name = state
                    .project
                    .project
                    .materials
                    .iter()
                    .find(|m| Some(m.id) == active_mat_id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "Nenhum".to_string());

                egui::ComboBox::from_id_salt("material_picker_dropdown")
                    .width(clamped_width(ui, 96.0, 180.0))
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        let mats: Vec<(Uuid, String)> = state
                            .project
                            .project
                            .materials
                            .iter()
                            .map(|m| (m.id, m.name.clone()))
                            .collect();
                        for (mid, mname) in mats {
                            let is_sel = active_mat_id == Some(mid);
                            if ui.selectable_label(is_sel, mname).clicked() {
                                state.checkpoint("change asset material");
                                if let Some(a) = state.project.assets.get_mut(active_idx) {
                                    a.material_id = Some(mid);
                                }
                                state.mark_dirty();
                            }
                        }
                    });

                if ui
                    .button("+ Novo")
                    .on_hover_text("Criar novo material no projeto")
                    .clicked()
                {
                    state.checkpoint("create new material");
                    let count = state.project.project.materials.len() + 1;
                    let new_mat = Material::new(format!("Material {count}"));
                    let new_id = state.project.project.add_material(new_mat);
                    if let Some(a) = state.project.assets.get_mut(active_idx) {
                        a.material_id = Some(new_id);
                    }
                    state.mark_dirty();
                }

                if ui
                    .button("⧉")
                    .on_hover_text("Duplicar material ativo")
                    .clicked()
                    && let Some(cur_mat) = state.project.project.active_material().cloned()
                {
                    state.checkpoint("duplicate material");
                    let dup = cur_mat.duplicate();
                    let dup_id = state.project.project.add_material(dup);
                    if let Some(a) = state.project.assets.get_mut(active_idx) {
                        a.material_id = Some(dup_id);
                    }
                    state.mark_dirty();
                }

                if let Some(material_id) = active_mat_id
                    && ui
                        .button("×")
                        .on_hover_text("Excluir material ativo")
                        .clicked()
                {
                    state.checkpoint("delete material");
                    if state.project.project.remove_material(material_id) {
                        state.render.canvas_dirty = true;
                        state.emit_mesh_changed();
                        state.mark_dirty();
                    }
                }
            });

            ui.separator();

            // 2. Edição de Propriedades do Material Ativo
            let mat_id = match active_mat_id {
                Some(id) => id,
                None => {
                    ui.label(egui::RichText::new("Nenhum material atribuído ao asset").italics());
                    return;
                }
            };

            let mut should_emit_change = false;

            if let Some(mat) = state.project.project.get_material_mut(mat_id) {
                // Nome
                ui.horizontal(|ui| {
                    ui.label("Nome:");
                    ui.text_edit_singleline(&mut mat.name);
                });

                // Perfil de Shader (P3D-140)
                ui.horizontal(|ui| {
                    ui.label("Perfil:");
                    egui::ComboBox::from_id_salt("material_profile_combo")
                        .width(clamped_width(ui, 96.0, 180.0))
                        .selected_text(mat.profile.label())
                        .show_ui(ui, |ui| {
                            for prof in ShaderProfile::ALL {
                                if ui
                                    .selectable_value(&mut mat.profile, prof, prof.label())
                                    .clicked()
                                {
                                    should_emit_change = true;
                                }
                            }
                        });
                });

                // Cor Base (P3D-051)
                ui.horizontal(|ui| {
                    ui.label("Cor Base:");
                    let mut rgb = [mat.base_color[0], mat.base_color[1], mat.base_color[2]];
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        mat.base_color[0] = rgb[0];
                        mat.base_color[1] = rgb[1];
                        mat.base_color[2] = rgb[2];
                        should_emit_change = true;
                    }
                });

                // Rugosidade / Roughness & Glossiness (P3D-053)
                ui.horizontal(|ui| {
                    ui.label("Rugosidade:");
                    if ui
                        .add(egui::Slider::new(&mut mat.roughness, 0.0..=1.0))
                        .changed()
                    {
                        should_emit_change = true;
                    }
                    ui.label(format!("(Brilho: {:.0}%)", mat.glossiness() * 100.0));
                });

                // Metacidade
                ui.horizontal(|ui| {
                    ui.label("Metálico:");
                    if ui
                        .add(egui::Slider::new(&mut mat.metallic, 0.0..=1.0))
                        .changed()
                    {
                        should_emit_change = true;
                    }
                });

                // Normal Scale (P3D-052)
                ui.horizontal(|ui| {
                    ui.label("Escala Normal:");
                    if ui
                        .add(egui::Slider::new(&mut mat.normal_scale, 0.0..=5.0))
                        .changed()
                    {
                        should_emit_change = true;
                    }
                });

                // Emissão
                if mat.profile == ShaderProfile::Emissive || mat.profile == ShaderProfile::Pbr {
                    ui.horizontal(|ui| {
                        ui.label("Emissão:");
                        if ui.color_edit_button_rgb(&mut mat.emission_color).changed() {
                            should_emit_change = true;
                        }
                        if ui
                            .add(egui::Slider::new(&mut mat.emission_strength, 0.0..=10.0))
                            .changed()
                        {
                            should_emit_change = true;
                        }
                    });
                }

                // Modo Alfa
                ui.horizontal(|ui| {
                    ui.label("Modo Alfa:");
                    let mode_lbl = match mat.alpha_mode {
                        AlphaMode::Opaque => "Opaco",
                        AlphaMode::Mask => "Máscara (Cutoff)",
                        AlphaMode::Blend => "Translucidez (Blend)",
                    };
                    egui::ComboBox::from_id_salt("material_alpha_mode_combo")
                        .width(clamped_width(ui, 96.0, 180.0))
                        .selected_text(mode_lbl)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut mat.alpha_mode, AlphaMode::Opaque, "Opaco");
                            ui.selectable_value(
                                &mut mat.alpha_mode,
                                AlphaMode::Mask,
                                "Máscara (Cutoff)",
                            );
                            ui.selectable_value(
                                &mut mat.alpha_mode,
                                AlphaMode::Blend,
                                "Translucidez (Blend)",
                            );
                        });
                });
                if mat.alpha_mode == AlphaMode::Mask {
                    ui.horizontal(|ui| {
                        ui.label("Corte Alfa:");
                        ui.add(egui::Slider::new(&mut mat.alpha_cutoff, 0.0..=1.0));
                    });
                }

                // Texturas anexadas
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Textura Albedo:");
                    if let Some(cv) = &mat.albedo_texture {
                        ui.label(format!("{}x{} px", cv.w, cv.h));
                        if ui.small_button("Limpar").clicked() {
                            mat.albedo_texture = None;
                            should_emit_change = true;
                        }
                    } else {
                        ui.label("Nenhuma");
                        if ui.small_button("+ Criar").clicked() {
                            let c = mat.base_color;
                            mat.albedo_texture = Some(petunia_project::Canvas::new(
                                256,
                                256,
                                [
                                    (c[0] * 255.0) as u8,
                                    (c[1] * 255.0) as u8,
                                    (c[2] * 255.0) as u8,
                                    255,
                                ],
                            ));
                            should_emit_change = true;
                        }
                    }
                });
            }

            if should_emit_change {
                state.render.canvas_dirty = true;
                state.emit_mesh_changed();
                state.mark_dirty();
            }

            // 3. Paleta do Projeto
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Paleta do Projeto:")
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                let palette_colors = state.project.palette.clone();
                for (pal_idx, pal_col) in palette_colors.iter().enumerate() {
                    let color32 = egui::Color32::from_rgb(
                        (pal_col[0] * 255.0) as u8,
                        (pal_col[1] * 255.0) as u8,
                        (pal_col[2] * 255.0) as u8,
                    );
                    let (rect, resp) =
                        ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
                    ui.painter()
                        .rect_filled(rect, tokens::RADIUS_CONTROL, color32);
                    ui.painter().rect_stroke(
                        rect,
                        tokens::RADIUS_CONTROL,
                        tokens::stroke_border(),
                        egui::StrokeKind::Outside,
                    );
                    if resp
                        .on_hover_text(format!("Aplicar cor {pal_idx} ao material"))
                        .clicked()
                    {
                        state.checkpoint("apply palette color");
                        if let Some(mat) = state.project.project.get_material_mut(mat_id) {
                            mat.base_color = [pal_col[0], pal_col[1], pal_col[2], 1.0];
                        }
                        if let Some(o) = state.project.assets.get_mut(active_idx) {
                            o.base_color = *pal_col;
                        }
                        state.render.canvas_dirty = true;
                        state.emit_mesh_changed();
                        state.mark_dirty();
                    }
                }
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_inspector_renders_with_an_assigned_operand() {
        let context = egui::Context::default();
        let mut state = AppState::new("en");
        state.project.add("Operand", petunia_mesh::Mesh::cube(1.0));
        let operand_id = state.project.assets.last().expect("operand").id;
        state.project.active = 0;
        state.boolean_operand = Some(operand_id);

        context
            .run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw_boolean_section(ui, &mut state);
                });
            })
            .textures_delta
            .clear();
    }

    #[test]
    fn test_inspector_tabs_render_without_panic() {
        use petunia_core::EditMode;
        let tools = ToolRegistry::default();
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        crate::outliner::add_primitive_to_scene(&mut state, 0, "Cube");
        let mut registry = ModuleRegistry::new();

        // Modify + Material com objeto.
        for tab in ["modify", "material"] {
            state.ui.properties_tab = tab.to_string();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw(ui, &mut state, &tools, &mut registry);
                });
            })
            .textures_delta
            .clear();
        }

        // Selection com componentes selecionados no modo de edição.
        state.set_edit_mode(EditMode::Edit);
        let n_verts = state.project.assets[0].mesh.verts.len();
        state.project.assets[0].mesh.select_all();
        state.selection.verts = (0..n_verts as u32).collect();
        state.ui.properties_tab = "selection".to_string();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();
        assert!(!state.selection.is_empty());

        // Bloco da operação modal ativa + todas as densidades.
        state.set_edit_mode(petunia_core::EditMode::Object);
        state.selection.clear();
        state
            .begin_modal(petunia_core::ModalKind::Move)
            .expect("modal abre");
        for density in petunia_core::UiDensity::all() {
            state.ui.density = density;
            state.ui.properties_tab = "modify".to_string();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw(ui, &mut state, &tools, &mut registry);
                });
            })
            .textures_delta
            .clear();
        }
        assert!(state.modal.is_some());
        assert!(state.cancel_modal());
        assert!(state.modal.is_none());
    }

    #[test]
    fn test_inspector_empty_collapsed_and_object_states_render() {
        let tools = ToolRegistry::default();
        // 1. Cena vazia: quick-add, sem pânico.
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        state.project.assets.clear();
        state.project.active = 0;
        assert!(state.project.assets.is_empty());
        let mut registry = ModuleRegistry::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();

        // 2. Com objeto + colapsado: só a barra, sem pânico.
        crate::outliner::add_primitive_to_scene(&mut state, 0, "Cube");
        state.ui.inspector_collapsed = true;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();

        // 3. Com objeto expandido: rail + pilha, sem pânico.
        state.ui.inspector_collapsed = false;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();
        assert_eq!(state.project.assets.len(), 1);
    }

    #[test]
    fn test_properties_panel_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let tools = ToolRegistry::default();
        let mut registry = ModuleRegistry::new();

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_properties_panel_renders_annotation_inspector() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let tools = ToolRegistry::default();
        let mut registry = ModuleRegistry::new();

        let stroke = petunia_core::AnnotationStroke {
            points: vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            color: [0.0, 0.74, 0.83, 1.0],
            width: 3.0,
        };
        let mut ann = petunia_core::AnnotationItem::new("TestNote", vec![stroke]);
        ann.translation = [1.0, 2.0, 3.0];
        ann.rotation = [45.0, 0.0, 0.0];
        ann.scale = [2.0, 2.0, 2.0];
        let ann_id = ann.id;
        state.project.add_annotation(ann);
        state.selected_annotation = Some(ann_id);
        state.ui.properties_tab = "object".to_string();

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();

        assert_eq!(state.project.annotations.len(), 1);
        assert_eq!(state.project.annotations[0].translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_properties_panel_renders_measurement_inspector() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let tools = ToolRegistry::default();
        let mut registry = ModuleRegistry::new();

        let meas =
            petunia_core::MeasurementItem::new("TestMeas", [0.0, 0.0, 0.0], [3.0, 4.0, 0.0], 5.0);
        let meas_id = meas.id;
        state.project.add_measurement(meas);
        state.selected_measurement = Some(meas_id);
        state.ui.properties_tab = "object".to_string();

        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                draw(ui, &mut state, &tools, &mut registry);
            });
        })
        .textures_delta
        .clear();

        assert_eq!(state.project.measurements.len(), 1);
        assert_eq!(state.project.measurements[0].distance, 5.0);
    }
}
