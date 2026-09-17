//! Barra Contextual Horizontal do Viewport (`Contextual Modeling Shelf`).
//! Posicionada na base inferior do Viewport 3D, reagindo dinamicamente ao
//! workspace ativo (MODEL / PAINT / UV) e ao domínio de seleção
//! (`Object` / `Point` / `Edge` / `Face`, P3D-015).
//!
//! Arquitetura Wave 4: conteúdo como dados ([`ShelfCommand`] com prioridade),
//! larguras medidas por galley (nunca `len() * k`), cápsula de fundo com tamanho
//! exato do conteúdo e modos responsivos (Full → Compact → Overflow → Pill).

use egui::{FontId, Rect, Response, RichText, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
use petunia_core::{
    AppState, DuplicateSelectionCmd, EditMode, MergeCenterCmd, ModalKind, SubdivideSelectionCmd,
    Workspace,
};

use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::tokens;
use crate::widgets::PetuniaMenuItem;
use petunia_config::text_id;

/// Prioridade do comando para colapso responsivo (Wave 4 — §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShelfPriority {
    /// Fica visível até o modo Overflow (como ícone).
    Primary,
    /// Vira ícone no modo Compact; vai para o popover no Overflow.
    Secondary,
}

/// Ação semântica de um comando da shelf (executada sem coordenadas de UI).
#[derive(Debug, Clone, Copy)]
pub enum ShelfAction {
    SetActiveTool(&'static str),
    SetGizmo(ModalKind),
    DomainOp(DomainOp),
    AddPrimitive { kind: u8, name: &'static str },
    ReopenLast,
    OpenReferenceManager,
    AddHumanoidArmature,
    AutoRigActiveMesh,
    TimelineFirst,
    TimelinePlayPause,
    TimelineLast,
}

/// Operação de domínio despachável pela shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainOp {
    Subdivide,
    MergeCenter,
    Duplicate,
}

/// Comando da shelf: ícone semântico + rótulo localizado + prioridade + ação.
pub struct ShelfCommand {
    pub icon: Option<PetuniaIcon>,
    pub label: String,
    pub tooltip: String,
    pub priority: ShelfPriority,
    pub action: ShelfAction,
}

/// Item não-botão da shelf (controles finos, sempre visíveis quando há espaço).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShelfWidget {
    PaintRadius,
    PaintColor,
    TimelineFrame,
}

impl ShelfWidget {
    /// Largura fixa de layout (controles com tamanho forçado — exato).
    fn fixed_width(self) -> f32 {
        match self {
            // DragValue com largura forçada + folga.
            ShelfWidget::PaintRadius | ShelfWidget::TimelineFrame => 72.0,
            // Botão de cor com tamanho forçado.
            ShelfWidget::PaintColor => 28.0,
        }
    }
}

const PILL_H: f32 = 22.0;
const PILL_FONT: FontId = FontId::proportional(10.5);
const ITEM_GAP: f32 = 4.0;
const SHELF_SIDE_PAD: f32 = 8.0;
const ICON_W: f32 = 16.0;

/// Largura exata de uma pílula (ícone? + texto medido + respiro). Mesma fórmula
/// usada na renderização — cápsula nunca menor que o conteúdo.
fn pill_width(has_icon: bool, text_w: f32) -> f32 {
    let icon_w = if has_icon { ICON_W } else { 0.0 };
    let padding = if has_icon && text_w > 0.0 { 16.0 } else { 12.0 };
    (icon_w + text_w + padding).max(24.0)
}

fn measure_text(ui: &Ui, label: &str) -> f32 {
    ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), PILL_FONT, tokens::TEXT_PRIMARY))
        .size()
        .x
        + 4.0
}

/// Modo responsivo da shelf (Wave 4 — §7.2), decidido por larguras medidas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShelfMode {
    /// Ícone + rótulo em tudo.
    Full,
    /// Primários com rótulo; secundários só ícone.
    Compact,
    /// Primários só ícone; resto no popover "More…".
    Overflow,
    /// Só pílula "Tools…" com popover (viewport muito estreita).
    Pill,
    /// Sem espaço nem para a pílula.
    Hidden,
}

/// Renderiza a barra contextual horizontal flutuante na base do Viewport 3D.
/// Retorna o `Rect` ocupado pela shelf para permitir bloqueio de eventos na cena 3D.
pub fn draw(ui: &mut Ui, state: &mut AppState, viewport_rect: Rect) -> Option<Rect> {
    puffin::profile_function!();
    let avail = viewport_rect.width() - 24.0;
    if avail <= 0.0 {
        return None;
    }

    let (commands, widgets) = build_shelf(state);

    // Mede todos os rótulos uma vez (galley real — nunca `len() * k`).
    let measured: Vec<(ShelfCommand, f32)> = commands
        .into_iter()
        .map(|cmd| {
            let w = measure_text(ui, &cmd.label);
            (cmd, w)
        })
        .collect();

    let gap_total = |n: usize| {
        if n == 0 {
            0.0
        } else {
            ITEM_GAP * (n as f32 - 1.0)
        }
    };
    let full_w: f32 = measured
        .iter()
        .map(|(c, w)| pill_width(c.icon.is_some(), *w))
        .sum::<f32>()
        + gap_total(measured.len())
        + widget_width_sum(&widgets)
        + SHELF_SIDE_PAD * 2.0;
    let compact_w: f32 = measured
        .iter()
        .map(|(c, w)| {
            let show_label = c.priority == ShelfPriority::Primary;
            pill_width(c.icon.is_some(), if show_label { *w } else { 0.0 })
        })
        .sum::<f32>()
        + gap_total(measured.len())
        + widget_width_sum(&widgets)
        + SHELF_SIDE_PAD * 2.0;
    let more_w = pill_width(false, measure_text(ui, &state.t_id(text_id::UI_MORE)));
    let primary_icon_w: f32 = measured
        .iter()
        .filter(|(c, _)| c.priority == ShelfPriority::Primary)
        .map(|(c, _)| pill_width(c.icon.is_some(), 0.0))
        .sum::<f32>();
    let primary_n = measured
        .iter()
        .filter(|(c, _)| c.priority == ShelfPriority::Primary)
        .count();
    let overflow_w = primary_icon_w
        + gap_total(primary_n + widgets.len() + 1)
        + more_w
        + widget_width_sum(&widgets)
        + SHELF_SIDE_PAD * 2.0;
    let tools_w = pill_width(false, measure_text(ui, &state.t_id(text_id::UI_TOOLS_MENU)))
        + SHELF_SIDE_PAD * 2.0;

    let mode = if full_w <= avail {
        ShelfMode::Full
    } else if compact_w <= avail {
        ShelfMode::Compact
    } else if overflow_w <= avail {
        ShelfMode::Overflow
    } else if tools_w <= avail {
        ShelfMode::Pill
    } else {
        ShelfMode::Hidden
    };
    if mode == ShelfMode::Hidden {
        return None;
    }

    // Largura exata do conteúdo no modo escolhido → cápsula sem folga nem falta.
    let content_w = match mode {
        ShelfMode::Full => full_w,
        ShelfMode::Compact => compact_w,
        ShelfMode::Overflow => overflow_w,
        ShelfMode::Pill => tools_w,
        ShelfMode::Hidden => return None,
    } - SHELF_SIDE_PAD * 2.0;
    let shelf_h = 36.0;
    let bottom_margin = 12.0;
    let shelf_rect = Rect::from_center_size(
        pos2(
            viewport_rect.center().x,
            viewport_rect.max.y - bottom_margin - shelf_h * 0.5,
        ),
        vec2(content_w + SHELF_SIDE_PAD * 2.0, shelf_h),
    );

    let painter = ui.painter();
    painter.rect_filled(
        shelf_rect,
        tokens::RADIUS_PILL,
        tokens::bg_panel_header(state),
    );
    painter.rect_stroke(
        shelf_rect,
        tokens::RADIUS_PILL,
        tokens::stroke_border_dyn(state),
        egui::StrokeKind::Inside,
    );

    // Escudo de eventos: impede que cliques na shelf atravessem para o raycasting da cena 3D
    let _ = ui.allocate_rect(shelf_rect, egui::Sense::click_and_drag());

    let inner = shelf_rect.shrink2(vec2(SHELF_SIDE_PAD, (shelf_h - PILL_H) * 0.5));
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    row.spacing_mut().item_spacing = vec2(ITEM_GAP, 0.0);
    row.horizontal_centered(|ui| {
        match mode {
            ShelfMode::Full => {
                for (cmd, text_w) in &measured {
                    draw_shelf_pill(ui, state, cmd, Some(*text_w));
                }
            }
            ShelfMode::Compact => {
                for (cmd, text_w) in &measured {
                    let show_label = cmd.priority == ShelfPriority::Primary;
                    draw_shelf_pill(ui, state, cmd, show_label.then_some(*text_w));
                }
            }
            ShelfMode::Overflow => {
                for (cmd, _) in measured
                    .iter()
                    .filter(|(c, _)| c.priority == ShelfPriority::Primary)
                {
                    draw_shelf_pill(ui, state, cmd, None);
                }
                let overflowed: Vec<&ShelfCommand> = measured
                    .iter()
                    .filter(|(c, _)| c.priority == ShelfPriority::Secondary)
                    .map(|(c, _)| c)
                    .collect();
                if !overflowed.is_empty() {
                    draw_more_popover(ui, state, &state.t_id(text_id::UI_MORE), &overflowed);
                }
            }
            ShelfMode::Pill => {
                let all: Vec<&ShelfCommand> = measured.iter().map(|(c, _)| c).collect();
                draw_more_popover(ui, state, &state.t_id(text_id::UI_TOOLS_MENU), &all);
            }
            ShelfMode::Hidden => {}
        }
        for widget in &widgets {
            // Controles finos ficam visíveis em Full/Compact/Overflow (Pill não tem espaço).
            if mode == ShelfMode::Pill {
                continue;
            }
            draw_shelf_widget(ui, state, *widget);
        }
    });

    Some(shelf_rect)
}

fn widget_width_sum(widgets: &[ShelfWidget]) -> f32 {
    let n = widgets.len();
    widgets.iter().map(|w| w.fixed_width()).sum::<f32>()
        + if n == 0 { 0.0 } else { ITEM_GAP * n as f32 }
}

/// Executa a ação semântica de um comando da shelf.
fn exec_shelf_action(state: &mut AppState, action: &ShelfAction) {
    match action {
        ShelfAction::SetActiveTool(id) => {
            state.active_tool = id.to_string();
        }
        ShelfAction::SetGizmo(gizmo) => {
            state.active_tool = "transform".to_string();
            state.gizmo_mode = *gizmo;
        }
        ShelfAction::DomainOp(DomainOp::Subdivide) => {
            let _ = state.dispatch(&SubdivideSelectionCmd);
        }
        ShelfAction::DomainOp(DomainOp::MergeCenter) => {
            let _ = state.dispatch(&MergeCenterCmd);
        }
        ShelfAction::DomainOp(DomainOp::Duplicate) => {
            let _ = state.dispatch(&DuplicateSelectionCmd);
        }
        ShelfAction::AddPrimitive { kind, name } => {
            // Wave 8: criação com sessão (cartão Last Operation), não inserção seca.
            crate::outliner::add_primitive_to_scene(state, *kind as usize, name);
        }
        ShelfAction::ReopenLast => {
            state.reopen_last_primitive();
        }
        ShelfAction::OpenReferenceManager => {
            state.ui.show_reference_manager = true;
        }
        ShelfAction::AddHumanoidArmature => {
            let skel = petunia_project::animation::RigPreset::humanoid(1.0);
            state.project.add_skeleton(skel);
        }
        ShelfAction::AutoRigActiveMesh => {
            let active = state.project.active;
            if let Some(asset) = state.project.assets.get_mut(active) {
                let skel = petunia_project::animation::auto_fit_humanoid(&asset.mesh);
                let skel_id = skel.id;
                let skin =
                    petunia_project::animation::compute_auto_skin_weights(&asset.mesh, &skel);
                asset.skeleton_id = Some(skel_id);
                asset.skin_data = Some(skin);
                state.project.add_skeleton(skel);
            }
        }
        ShelfAction::TimelineFirst => {
            state.ui.timeline_frame = state.ui.timeline_start;
        }
        ShelfAction::TimelinePlayPause => {
            state.ui.timeline_playing = !state.ui.timeline_playing;
        }
        ShelfAction::TimelineLast => {
            state.ui.timeline_frame = state.ui.timeline_end;
        }
    }
    state.mark_dirty();
}

fn tool_cmd(
    state: &AppState,
    tool_id: &'static str,
    key: &str,
    icon: PetuniaIcon,
    priority: ShelfPriority,
    action: ShelfAction,
) -> ShelfCommand {
    let name = state.t(&format!("tools.{tool_id}"));
    ShelfCommand {
        icon: Some(icon),
        label: name.clone(),
        tooltip: format!("{name} · [{key}]"),
        priority,
        action,
    }
}

/// Constrói comandos + widgets da shelf para o workspace/modo atual.
fn build_shelf(state: &AppState) -> (Vec<ShelfCommand>, Vec<ShelfWidget>) {
    match state.workspace {
        Workspace::Model if state.edit_mode() == EditMode::Edit => {
            let mut cmds = vec![
                tool_cmd(
                    state,
                    "extrude",
                    "E",
                    PetuniaIcon::Extrude,
                    ShelfPriority::Primary,
                    ShelfAction::SetActiveTool("extrude"),
                ),
                tool_cmd(
                    state,
                    "inset",
                    "I",
                    PetuniaIcon::Inset,
                    ShelfPriority::Primary,
                    ShelfAction::SetActiveTool("inset"),
                ),
                tool_cmd(
                    state,
                    "bevel",
                    "Ctrl+B",
                    PetuniaIcon::Bevel,
                    ShelfPriority::Primary,
                    ShelfAction::SetActiveTool("bevel"),
                ),
                tool_cmd(
                    state,
                    "loop_cut",
                    "Ctrl+R",
                    PetuniaIcon::LoopCut,
                    ShelfPriority::Secondary,
                    ShelfAction::SetActiveTool("loop_cut"),
                ),
                tool_cmd(
                    state,
                    "knife",
                    "K",
                    PetuniaIcon::Knife,
                    ShelfPriority::Secondary,
                    ShelfAction::SetActiveTool("knife"),
                ),
            ];
            let sub_name = state.t("actions.subdivide");
            cmds.push(ShelfCommand {
                icon: Some(PetuniaIcon::Subdivide),
                label: sub_name.clone(),
                tooltip: sub_name,
                priority: ShelfPriority::Secondary,
                action: ShelfAction::DomainOp(DomainOp::Subdivide),
            });
            let merge_name = state.t("actions.merge_center");
            cmds.push(ShelfCommand {
                icon: Some(PetuniaIcon::Custom("merge")),
                label: merge_name.clone(),
                tooltip: merge_name,
                priority: ShelfPriority::Secondary,
                action: ShelfAction::DomainOp(DomainOp::MergeCenter),
            });
            let ref_name = state.t_id(text_id::UI_REFS);
            cmds.push(ShelfCommand {
                icon: Some(PetuniaIcon::ReferenceImage),
                label: ref_name.clone(),
                tooltip: ref_name,
                priority: ShelfPriority::Secondary,
                action: ShelfAction::OpenReferenceManager,
            });
            (cmds, vec![])
        }
        Workspace::Model => {
            let mut cmds = vec![
                tool_cmd(
                    state,
                    "move",
                    "G",
                    PetuniaIcon::Move,
                    ShelfPriority::Primary,
                    ShelfAction::SetGizmo(ModalKind::Move),
                ),
                tool_cmd(
                    state,
                    "rotate",
                    "R",
                    PetuniaIcon::Rotate,
                    ShelfPriority::Primary,
                    ShelfAction::SetGizmo(ModalKind::Rotate),
                ),
                tool_cmd(
                    state,
                    "scale",
                    "S",
                    PetuniaIcon::Scale,
                    ShelfPriority::Primary,
                    ShelfAction::SetGizmo(ModalKind::Scale),
                ),
            ];
            for (name, key, kind) in [
                ("Cube", "prims.cube", 0u8),
                ("Sphere", "prims.sphere", 1),
                ("Cylinder", "prims.cylinder", 2),
                ("Plane", "prims.plane", 3),
            ] {
                let label = state.t(key);
                cmds.push(ShelfCommand {
                    icon: Some(PetuniaIcon::AddPrimitive),
                    label: label.clone(),
                    tooltip: format!("{} {}", label, state.t_id(text_id::UI_AT_3D_CURSOR)),
                    priority: ShelfPriority::Secondary,
                    action: ShelfAction::AddPrimitive { kind, name },
                });
            }
            let dup_label = state.t_id(text_id::UI_DUPLICATE);
            cmds.push(ShelfCommand {
                icon: Some(PetuniaIcon::Duplicate),
                label: dup_label.clone(),
                tooltip: format!("{dup_label} · [Shift+D]"),
                priority: ShelfPriority::Primary,
                action: ShelfAction::DomainOp(DomainOp::Duplicate),
            });
            // Reabertura explícita da última criação (Wave 8 — F9 ou botão).
            if state.session.primitive_session.is_none() && state.session.last_primitive.is_some() {
                let reopen_label = state.t_id(text_id::PRIMS_REOPEN);
                cmds.push(ShelfCommand {
                    icon: Some(PetuniaIcon::Undo),
                    label: reopen_label.clone(),
                    tooltip: reopen_label,
                    priority: ShelfPriority::Secondary,
                    action: ShelfAction::ReopenLast,
                });
            }
            let ref_label = state.t_id(text_id::UI_REFS);
            cmds.push(ShelfCommand {
                icon: Some(PetuniaIcon::ReferenceImage),
                label: ref_label.clone(),
                tooltip: ref_label,
                priority: ShelfPriority::Secondary,
                action: ShelfAction::OpenReferenceManager,
            });
            (cmds, vec![])
        }
        Workspace::Paint => {
            let cmds = ["paint", "eraser", "picker"]
                .into_iter()
                .map(|id| {
                    let label = state.t(&format!("tools.{id}"));
                    ShelfCommand {
                        icon: Some(match id {
                            "paint" => PetuniaIcon::PaintBrush,
                            "eraser" => PetuniaIcon::PaintEraser,
                            _ => PetuniaIcon::PaintPicker,
                        }),
                        label: label.clone(),
                        tooltip: label,
                        priority: ShelfPriority::Primary,
                        action: ShelfAction::SetActiveTool(id),
                    }
                })
                .collect();
            (
                cmds,
                vec![ShelfWidget::PaintRadius, ShelfWidget::PaintColor],
            )
        }
        Workspace::Uv => {
            let cmds = [
                ("paint", ShelfPriority::Primary),
                ("eraser", ShelfPriority::Primary),
                ("picker", ShelfPriority::Primary),
            ]
            .into_iter()
            .map(|(id, priority)| {
                let label = state.t(&format!("tools.{id}"));
                ShelfCommand {
                    icon: None,
                    label: label.clone(),
                    tooltip: label,
                    priority,
                    action: ShelfAction::SetActiveTool(id),
                }
            })
            .collect();
            (cmds, vec![])
        }
        #[cfg(feature = "animation-workspace")]
        Workspace::Animate => {
            let hum_label = state.t_id(text_id::ANIMATE_HUMANOID);
            let rig_label = state.t_id(text_id::ANIMATE_AUTO_RIG);
            let play_label = state.t(if state.ui.timeline_playing {
                "animate.pause"
            } else {
                "animate.play"
            });
            let cmds = vec![
                ShelfCommand {
                    icon: None,
                    label: hum_label.clone(),
                    tooltip: hum_label,
                    priority: ShelfPriority::Secondary,
                    action: ShelfAction::AddHumanoidArmature,
                },
                ShelfCommand {
                    icon: None,
                    label: rig_label.clone(),
                    tooltip: rig_label,
                    priority: ShelfPriority::Secondary,
                    action: ShelfAction::AutoRigActiveMesh,
                },
                ShelfCommand {
                    icon: Some(PetuniaIcon::JumpStart),
                    label: String::new(),
                    tooltip: state.t_id(text_id::ANIMATE_FIRST_FRAME),
                    priority: ShelfPriority::Primary,
                    action: ShelfAction::TimelineFirst,
                },
                ShelfCommand {
                    icon: Some(if state.ui.timeline_playing {
                        PetuniaIcon::Pause
                    } else {
                        PetuniaIcon::Play
                    }),
                    label: play_label.clone(),
                    tooltip: play_label,
                    priority: ShelfPriority::Primary,
                    action: ShelfAction::TimelinePlayPause,
                },
                ShelfCommand {
                    icon: Some(PetuniaIcon::JumpEnd),
                    label: String::new(),
                    tooltip: state.t_id(text_id::ANIMATE_LAST_FRAME),
                    priority: ShelfPriority::Primary,
                    action: ShelfAction::TimelineLast,
                },
            ];
            (cmds, vec![ShelfWidget::TimelineFrame])
        }
    }
}

fn draw_shelf_pill(
    ui: &mut Ui,
    state: &mut AppState,
    cmd: &ShelfCommand,
    text_w: Option<f32>,
) -> Response {
    let is_active = matches!(cmd.action, ShelfAction::SetActiveTool(id) if state.active_tool == id)
        || matches!(cmd.action, ShelfAction::SetGizmo(g) if state.gizmo_mode == g && state.active_tool == "transform")
        || matches!(cmd.action, ShelfAction::TimelinePlayPause if state.ui.timeline_playing);
    let (bg, fg) = if is_active {
        (tokens::ACCENT_BLUE, tokens::TEXT_ACTIVE)
    } else {
        (tokens::BG_SURFACE, tokens::TEXT_PRIMARY)
    };

    let label = text_w.map(|_| cmd.label.as_str()).unwrap_or("");
    let tw = text_w.unwrap_or(0.0);
    let desired_size = vec2(pill_width(cmd.icon.is_some(), tw), PILL_H);

    let (rect, resp) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let accessible_label = if cmd.label.is_empty() {
        cmd.tooltip.clone()
    } else {
        cmd.label.clone()
    };
    resp.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Button,
            true,
            is_active,
            accessible_label.clone(),
        )
    });
    if ui.is_rect_visible(rect) {
        let fill = if is_active {
            bg
        } else if resp.hovered() {
            tokens::BG_SURFACE_HOVER
        } else {
            bg
        };
        ui.painter().rect_filled(rect, tokens::RADIUS_PILL, fill);

        let mut start_x = rect.min.x
            + if cmd.icon.is_some() && label.is_empty() {
                (rect.width() - 14.0) * 0.5
            } else {
                6.0
            };
        if let Some(ic) = &cmd.icon {
            let icon_rect = Rect::from_min_size(
                pos2(start_x, rect.min.y + (rect.height() - 14.0) * 0.5),
                vec2(14.0, 14.0),
            );
            IconRegistry::paint(ui.ctx(), ui.painter(), ic, icon_rect, fg);
            start_x += 17.0;
        }

        if !label.is_empty() {
            ui.painter().text(
                pos2(start_x, rect.min.y + (rect.height() - 13.0) * 0.5),
                egui::Align2::LEFT_TOP,
                label,
                PILL_FONT,
                fg,
            );
        }
        if resp.has_focus() {
            ui.painter().rect_stroke(
                rect,
                tokens::RADIUS_PILL,
                tokens::stroke_focus(),
                StrokeKind::Inside,
            );
        }
    }

    let resp = resp.on_hover_text(&cmd.tooltip);
    if resp.clicked() {
        exec_shelf_action(state, &cmd.action);
    }
    resp
}

fn draw_more_popover(ui: &mut Ui, state: &mut AppState, label: &str, cmds: &[&ShelfCommand]) {
    ui.menu_button(label, |ui| {
        ui.set_min_width(148.0);
        for cmd in cmds {
            let mut item = PetuniaMenuItem::new(&cmd.label);
            if let Some(icon) = &cmd.icon {
                item = item.icon(*icon);
            }
            if item.show(ui).on_hover_text(&cmd.tooltip).clicked() {
                exec_shelf_action(state, &cmd.action);
                ui.close();
            }
        }
    });
}

fn draw_shelf_widget(ui: &mut Ui, state: &mut AppState, widget: ShelfWidget) {
    match widget {
        ShelfWidget::PaintRadius => {
            ui.label(
                RichText::new(state.t("paint.size"))
                    .size(10.5)
                    .color(tokens::TEXT_SECONDARY),
            );
            ui.add_sized(
                vec2(64.0, 18.0),
                egui::DragValue::new(&mut state.canvas_brush)
                    .range(1..=512)
                    .speed(1.0),
            );
        }
        ShelfWidget::PaintColor => {
            ui.label(
                RichText::new(state.t_id(text_id::PAINT_COLOR))
                    .size(11.0)
                    .color(tokens::TEXT_SECONDARY),
            );
            let mut color = state.paint_color;
            ui.add_sized(vec2(24.0, 18.0), |ui: &mut Ui| {
                ui.color_edit_button_rgb(&mut color)
            });
            state.paint_color = color;
        }
        ShelfWidget::TimelineFrame => {
            ui.label(
                RichText::new(state.t_id(text_id::ANIMATE_FRAME))
                    .size(10.5)
                    .color(tokens::TEXT_SECONDARY),
            );
            ui.add_sized(
                vec2(64.0, 18.0),
                egui::DragValue::new(&mut state.ui.timeline_frame)
                    .range(state.ui.timeline_start..=state.ui.timeline_end),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contextual_shelf_renders_without_panic() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(800.0, 600.0));
                let _ = draw(ui, &mut state, rect);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn pill_width_covers_icon_text_and_padding() {
        // Sem estimativa: texto de 100px + ícone 16 + respiro 16.
        assert_eq!(pill_width(true, 100.0), 132.0);
        assert_eq!(pill_width(false, 100.0), 112.0);
        assert_eq!(pill_width(true, 0.0), 28.0);
        assert_eq!(pill_width(false, 0.0), 24.0);
    }

    #[test]
    fn every_workspace_builds_commands() {
        for workspace in Workspace::all() {
            let mut state = AppState::new("en");
            state.workspace = workspace;
            for mode in [EditMode::Object, EditMode::Edit] {
                state.set_edit_mode(mode);
                let (cmds, _) = build_shelf(&state);
                assert!(!cmds.is_empty(), "{workspace:?} sem comandos");
            }
        }
    }

    #[test]
    fn every_shelf_command_has_label_and_tooltip() {
        // Descoberta de ações só-ícone (Wave 7 — §13.1/§22.2).
        for workspace in Workspace::all() {
            for lang in ["en", "pt-BR"] {
                let mut state = AppState::new(lang);
                state.workspace = workspace;
                for mode in [EditMode::Object, EditMode::Edit] {
                    state.set_edit_mode(mode);
                    let (cmds, _) = build_shelf(&state);
                    for cmd in &cmds {
                        assert!(!cmd.tooltip.is_empty(), "{workspace:?} tooltip vazio");
                        // Rótulo vazio só com ícone (representação icon-only válida).
                        assert!(
                            !cmd.label.is_empty() || cmd.icon.is_some(),
                            "{workspace:?} sem rótulo nem ícone"
                        );
                    }
                }
            }
        }
    }
}
