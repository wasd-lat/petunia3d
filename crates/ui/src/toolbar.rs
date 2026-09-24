//! Barra lateral vertical de ferramentas do Petunia3D (`Toolbar`).
//!
//! A toolbar é deliberadamente compacta e agrupada: seleção e transformação
//! são famílias (split buttons) em vez de uma lista de botões concorrentes.
//! Operações que pertencem ao Modifier Stack (Mirror/Symmetry) não aparecem
//! aqui. A configuração do usuário continua controlando ordem/visibilidade
//! dos grupos e das ferramentas de modelagem destrutivas.

use egui::{ScrollArea, Ui, vec2};
use petunia_core::{AppState, ModalKind, Workspace};
use petunia_module_model::ToolRegistry;

use crate::adapters::drag_drop::{PetuniaDragList, PetuniaDragSpec};
use crate::adapters::tool_grid::{PetuniaToolCell, PetuniaToolGridSpec};
use crate::icon_registry::PetuniaIcon;
use crate::tokens;
use crate::widgets::PetuniaToolbarButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarSection {
    Primary,
    Inspect,
    Mesh,
}

impl ToolbarSection {
    /// Chave estável de id: cada seção é o escopo dos `Id` da grade de taffy.
    pub const fn key(self) -> &'static str {
        match self {
            ToolbarSection::Primary => "primary",
            ToolbarSection::Inspect => "inspect",
            ToolbarSection::Mesh => "mesh",
        }
    }
}

pub struct ToolbarEntry {
    pub id: &'static str,
    pub icon: PetuniaIcon,
    pub key: &'static str,
    pub label_key: &'static str,
    pub section: ToolbarSection,
    pub edit_only: bool,
}

/// Ordem canônica da paleta Model.
///
/// `select` e `transform` representam grupos. Os filhos ficam no popover do
/// grupo, portanto não consomem quatro/cinco linhas da toolbar.
pub fn canonical_toolbar_entries() -> Vec<ToolbarEntry> {
    vec![
        ToolbarEntry {
            id: "select",
            icon: PetuniaIcon::SelectBox,
            key: "Q/B",
            label_key: "tools.select",
            section: ToolbarSection::Primary,
            edit_only: false,
        },
        ToolbarEntry {
            id: "cursor_3d",
            icon: PetuniaIcon::Cursor3D,
            key: "Shift+RMB",
            label_key: "tools.cursor_3d",
            section: ToolbarSection::Primary,
            edit_only: false,
        },
        ToolbarEntry {
            id: "transform",
            icon: PetuniaIcon::Transform,
            key: "T · G/R/S",
            label_key: "tools.transform",
            section: ToolbarSection::Primary,
            edit_only: false,
        },
        ToolbarEntry {
            id: "measure",
            icon: PetuniaIcon::Measure,
            key: "M",
            label_key: "tools.measure",
            section: ToolbarSection::Inspect,
            edit_only: false,
        },
        ToolbarEntry {
            id: "annotate",
            icon: PetuniaIcon::Annotate,
            key: "D",
            label_key: "tools.annotate",
            section: ToolbarSection::Inspect,
            edit_only: false,
        },
        ToolbarEntry {
            id: "extrude",
            icon: PetuniaIcon::Extrude,
            key: "E",
            label_key: "tools.extrude",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "inset",
            icon: PetuniaIcon::Inset,
            key: "I",
            label_key: "tools.inset",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "bevel",
            icon: PetuniaIcon::Bevel,
            key: "Ctrl+B",
            label_key: "tools.bevel",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "loop_cut",
            icon: PetuniaIcon::LoopCut,
            key: "Ctrl+R",
            label_key: "tools.loop_cut",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "knife",
            icon: PetuniaIcon::Knife,
            key: "K",
            label_key: "tools.knife",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "pushpull",
            icon: PetuniaIcon::PushPull,
            key: "P",
            label_key: "tools.pushpull",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "slice",
            icon: PetuniaIcon::Slice,
            key: "",
            label_key: "tools.slice",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "subdivide",
            icon: PetuniaIcon::Subdivide,
            key: "W",
            label_key: "tools.subdivide",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "draw_profile",
            icon: PetuniaIcon::DrawProfile,
            key: "Shift+P",
            label_key: "tools.draw_profile",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
        ToolbarEntry {
            id: "merge",
            icon: PetuniaIcon::Custom("merge"),
            key: "M",
            label_key: "tools.merge",
            section: ToolbarSection::Mesh,
            edit_only: true,
        },
    ]
}

fn clone_entry(entry: &ToolbarEntry) -> ToolbarEntry {
    ToolbarEntry {
        id: entry.id,
        icon: entry.icon,
        key: entry.key,
        label_key: entry.label_key,
        section: entry.section,
        edit_only: entry.edit_only,
    }
}

fn display_entries(state: &AppState) -> Vec<ToolbarEntry> {
    let canonical = canonical_toolbar_entries();
    let mut ordered = Vec::with_capacity(canonical.len());
    if state.ui.toolbar_order.is_empty() {
        ordered = canonical;
    } else {
        for id in &state.ui.toolbar_order {
            if let Some(entry) = canonical.iter().find(|entry| entry.id == id.as_str()) {
                ordered.push(clone_entry(entry));
            }
        }
        for entry in canonical {
            if !ordered.iter().any(|candidate| candidate.id == entry.id) {
                ordered.push(entry);
            }
        }
    }
    let in_edit = state.edit_mode() == petunia_core::EditMode::Edit;
    ordered
        .into_iter()
        .filter(|entry| {
            !state
                .ui
                .toolbar_hidden
                .iter()
                .any(|hidden| hidden == entry.id)
                && (!entry.edit_only || in_edit)
        })
        .collect()
}

fn apply_toolbar_entry(state: &mut AppState, tools: &ToolRegistry, entry: &ToolbarEntry) {
    state.active_tool = entry.id.into();
    state.pending_modal = None;
    if let Some(tool) = tools.get(entry.id) {
        tool.on_activate(state);
    }
    state.mark_dirty();
}

fn entry_is_active(state: &AppState, entry: &ToolbarEntry) -> bool {
    match entry.id {
        "select" => matches!(state.active_tool.as_str(), "select" | "select_box"),
        "transform" => matches!(
            state.active_tool.as_str(),
            "transform" | "move" | "rotate" | "scale"
        ),
        _ => state.active_tool == entry.id,
    }
}

const TOOLBAR_FOOTER: f32 = 44.0;

/// Grade da paleta lateral (§48).
///
/// A linha de base da paleta fica dentro da moldura, então a largura que chega
/// aqui já é a útil. As duas larguras são **declaradas**: `min_cell` é a célula
/// só-ícone e `label_min_cell` é a partir de onde o rótulo cabe. Quantas colunas
/// existem e se cada célula mostra o rótulo sai da medição do adapter — o
/// produto não compara `available_width()` com número nenhum.
static TOOL_GRID: PetuniaToolGridSpec = PetuniaToolGridSpec::new(tokens::TOOLBAR_WIDTH, 120.0, 3.0);

/// Lista reordenável do menu de configuração da paleta (§49).
///
/// Wave 7: a ordem das ferramentas é arrasto, não setas. A especificação
/// declara só o motion e a faixa do grip; o desenho da linha fica no produto.
static TOOLBAR_ORDER_DRAG: PetuniaDragList = PetuniaDragList::new(PetuniaDragSpec::new());

/// Conteúdo da paleta de ferramentas, sem painel próprio (Wave 5b).
///
/// O paine é o tile `Tools` da árvore do shell: a largura, o divisor e a
/// posição vêm do `PetuniaShellLayout`, não de um `Panel::left` local. A
/// moldura (fundo, borda e margem) continua aqui — ela é a identidade visual da
/// Public entrypoint for the toolbar drawing, delegating to `draw_contents`.
/// Ponto de entrada público para desenho da barra de ferramentas, delegando a `draw_contents`.
pub fn draw(ui: &mut Ui, state: &mut AppState, tools: &ToolRegistry) {
    draw_contents(ui, state, tools);
}

pub fn draw_contents(ui: &mut Ui, state: &mut AppState, tools: &ToolRegistry) {
    let frame = egui::Frame::new()
        .fill(tokens::bg_panel(state))
        .stroke(tokens::stroke_border_dyn(state))
        .inner_margin(egui::Margin::symmetric(
            tokens::TOOLBAR_PANEL_MARGIN as i8,
            tokens::TOOLBAR_PANEL_MARGIN as i8,
        ));
    frame.show(ui, |ui| {
        ui.add_enabled_ui(!state.is_interacting(), |ui| {
            ScrollArea::vertical()
                .id_salt("toolbar_scroll_area")
                .max_height((ui.available_height() - TOOLBAR_FOOTER).max(80.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 3.0);
                    draw_palette(ui, state, tools);
                });
            ui.separator();
            ui.vertical_centered(|ui| draw_toolbar_config(ui, state));
        });
    });
}

/// Paleta do workspace corrente.
///
/// O teto de colunas vem da preferência do usuário (`toolbar_columns`: lista ou
/// par) e o número **efetivo** de colunas, junto com a largura de cada célula,
/// vem do adapter.
fn draw_palette(ui: &mut Ui, state: &mut AppState, tools: &ToolRegistry) {
    let grid = TOOL_GRID.with_max_columns(state.ui.toolbar_columns.clamp(1, 2) as u16);
    match state.workspace {
        Workspace::Model => draw_model_tools(ui, state, tools, grid),
        Workspace::Paint => draw_paint_tools(ui, state, grid),
        Workspace::Uv => draw_paint_tools(ui, state, grid),
        #[cfg(feature = "animation-workspace")]
        Workspace::Animate => draw_animate_tools(ui, state, grid),
    }
}

fn draw_model_tools(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    grid: PetuniaToolGridSpec,
) {
    if state.edit_mode() != petunia_core::EditMode::Edit
        && canonical_toolbar_entries()
            .iter()
            .any(|entry| entry.edit_only && entry.id == state.active_tool)
    {
        state.active_tool = "select".into();
        state.mark_dirty();
    }

    let entries = display_entries(state);
    let mut seen_section = false;

    for section in [
        ToolbarSection::Primary,
        ToolbarSection::Inspect,
        ToolbarSection::Mesh,
    ] {
        let group: Vec<&ToolbarEntry> = entries
            .iter()
            .filter(|entry| entry.section == section)
            .collect();
        if group.is_empty() {
            continue;
        }
        if seen_section {
            ui.add_space(3.0);
            ui.separator();
            ui.add_space(3.0);
        }
        seen_section = true;

        // Cada seção tem a própria grade: a quebra de linha do taffy acontece
        // por seção, então o separador nunca cai no meio de um grupo.
        grid.show(
            ui,
            egui::Id::new(("petunia-tool-grid", section.key())),
            group.len(),
            |index, cell, ui| draw_model_entry(ui, state, tools, group[index], cell),
            |_ui| (),
        );
    }
}

fn draw_model_entry(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    entry: &ToolbarEntry,
    cell: PetuniaToolCell,
) {
    match entry.id {
        "select" => draw_select_group(ui, state, cell),
        "transform" => draw_transform_group(ui, state, cell),
        _ => draw_entry_button(ui, state, tools, entry, cell),
    }
}

fn draw_entry_button(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    entry: &ToolbarEntry,
    cell: PetuniaToolCell,
) {
    let label = state.t(entry.label_key);
    let hint = if entry.key.is_empty() {
        label.clone()
    } else {
        format!("{label} · [{}]", entry.key)
    };
    if PetuniaToolbarButton::new(entry.icon, &label)
        .selected(entry_is_active(state, entry))
        .compact(!cell.labeled)
        .width(cell.width)
        .tooltip(&hint)
        .show(ui)
        .clicked()
    {
        apply_toolbar_entry(state, tools, entry);
    }
}

/// Arranjo de uma linha de grupo dentro da célula da grade (§48).
///
/// O grupo é botão + seta. Quando a célula comporta os dois, o botão fica com o
/// que sobra e a seta abre o menu da família pelo clique esquerdo. Quando **não**
/// comporta, a seta simplesmente não é desenhada e o menu continua acessível
/// pelo clique secundário no próprio botão — nunca desenhamos um controle para
/// fora da célula, que era o que acontecia com a seta cortada pelo paine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupRow {
    /// Largura do botão principal.
    pub button: f32,
    /// A seta cabe ao lado do botão?
    pub chevron: bool,
}

/// Decide o arranjo do grupo a partir da célula resolvida pelo adapter.
///
/// Puro e testável: o produto não mede o container, só repassa a largura que o
/// adapter já resolveu.
pub fn plan_group(cell_width: f32) -> GroupRow {
    let cell_width = cell_width.max(0.0);
    let both = tokens::TOOLBAR_WIDTH + tokens::TOOLBAR_CONTROL_GAP + tokens::TOOLBAR_CHEVRON_WIDTH;
    if cell_width >= both {
        GroupRow {
            button: (cell_width - tokens::TOOLBAR_CONTROL_GAP - tokens::TOOLBAR_CHEVRON_WIDTH)
                .max(tokens::TOOLBAR_WIDTH),
            chevron: true,
        }
    } else {
        GroupRow {
            button: cell_width.max(tokens::TOOLBAR_WIDTH),
            chevron: false,
        }
    }
}

/// Dica do botão de um grupo: atalho e, quando a seta não cabe, como abrir o
/// menu da família.
fn group_hint(state: &AppState, label: &str, key: &str, chevron: bool) -> String {
    if chevron {
        format!("{label} · [{key}]")
    } else {
        format!("{label} · [{key}] · {}", state.t("toolbar.family_hint"))
    }
}

fn draw_select_group(ui: &mut egui::Ui, state: &mut AppState, cell: PetuniaToolCell) {
    let row = plan_group(cell.width);
    let compact = !TOOL_GRID.labels_fit(row.button);
    let box_mode = state.active_tool == "select_box";
    let label_key = if box_mode {
        "tools.select_box"
    } else {
        "tools.select"
    };
    let label = state.t(label_key);
    let hint = group_hint(state, &label, "Q/B", row.chevron);
    let mut button_response = None;
    let mut chevron_response = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(tokens::TOOLBAR_CONTROL_GAP, 0.0);
        let response = PetuniaToolbarButton::new(PetuniaIcon::SelectBox, &label)
            .selected(matches!(
                state.active_tool.as_str(),
                "select" | "select_box"
            ))
            .compact(compact)
            .width(row.button)
            .tooltip(&hint)
            .show(ui);
        if response.clicked() {
            state.active_tool = if box_mode { "select_box" } else { "select" }.into();
            state.pending_modal = None;
            state.mark_dirty();
        }
        button_response = Some(response);
        if row.chevron {
            chevron_response = Some(
                ui.small_button("▾")
                    .on_hover_text(state.t("toolbar.configure")),
            );
        }
    });
    open_family_menu(chevron_response, button_response, state, |ui, state| {
        draw_select_menu(ui, state)
    });
}

/// Abre o menu da família: pela seta quando ela existe, pelo clique secundário
/// no botão quando não existe.
fn open_family_menu(
    chevron: Option<egui::Response>,
    button: Option<egui::Response>,
    state: &mut AppState,
    menu: impl FnOnce(&mut Ui, &mut AppState),
) {
    if let Some(anchor) = chevron {
        egui::Popup::menu(&anchor).show(|ui| menu(ui, state));
    } else if let Some(response) = button {
        response.context_menu(|ui| menu(ui, state));
    }
}

/// Itens do menu da família de seleção.
fn draw_select_menu(ui: &mut Ui, state: &mut AppState) {
    ui.set_max_width(190.0);
    if ui
        .selectable_label(state.active_tool == "select", state.t("tools.select"))
        .clicked()
    {
        state.active_tool = "select".into();
        state.ui.box_select_start = None;
        state.mark_dirty();
        ui.close();
    }
    if ui
        .selectable_label(
            state.active_tool == "select_box",
            state.t("tools.select_box"),
        )
        .clicked()
    {
        state.active_tool = "select_box".into();
        state.ui.box_select_start = None;
        state.mark_dirty();
        ui.close();
    }
    ui.add_enabled(false, egui::Button::new(state.t("tools.select_lasso")))
        .on_hover_text(state.t("toolbar.planned"));
}

fn transform_child(state: &AppState) -> (&'static str, PetuniaIcon, &'static str, &'static str) {
    match state.active_tool.as_str() {
        "move" => ("move", PetuniaIcon::Move, "tools.move", "G"),
        "rotate" => ("rotate", PetuniaIcon::Rotate, "tools.rotate", "R"),
        "scale" => ("scale", PetuniaIcon::Scale, "tools.scale", "S"),
        _ => ("transform", PetuniaIcon::Transform, "tools.transform", "T"),
    }
}

fn activate_transform_child(state: &mut AppState, id: &str) {
    state.active_tool = id.into();
    state.pending_modal = None;
    match id {
        "move" => state.gizmo_mode = ModalKind::Move,
        "rotate" => state.gizmo_mode = ModalKind::Rotate,
        "scale" => state.gizmo_mode = ModalKind::Scale,
        _ => {}
    }
    state.mark_dirty();
}

fn draw_transform_group(ui: &mut egui::Ui, state: &mut AppState, cell: PetuniaToolCell) {
    let row = plan_group(cell.width);
    let compact = !TOOL_GRID.labels_fit(row.button);
    let (current_id, icon, label_key, key) = transform_child(state);
    let label = state.t(label_key);
    let active = matches!(
        state.active_tool.as_str(),
        "transform" | "move" | "rotate" | "scale"
    );
    let hint = group_hint(state, &label, key, row.chevron);
    let mut button_response = None;
    let mut chevron_response = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(tokens::TOOLBAR_CONTROL_GAP, 0.0);
        let response = PetuniaToolbarButton::new(icon, &label)
            .selected(active)
            .compact(compact)
            .width(row.button)
            .tooltip(&hint)
            .show(ui);
        if response.clicked() {
            activate_transform_child(state, current_id);
        }
        button_response = Some(response);
        if row.chevron {
            chevron_response = Some(
                ui.small_button("▾")
                    .on_hover_text(state.t("tools.transform")),
            );
        }
    });
    open_family_menu(chevron_response, button_response, state, |ui, state| {
        draw_transform_menu(ui, state)
    });
}

/// Itens do menu da família de transformação.
fn draw_transform_menu(ui: &mut Ui, state: &mut AppState) {
    ui.set_max_width(190.0);
    for (id, label_key, shortcut) in [
        ("transform", "tools.transform", "T"),
        ("move", "tools.move", "G"),
        ("rotate", "tools.rotate", "R"),
        ("scale", "tools.scale", "S"),
    ] {
        let text = format!("{}    {}", state.t(label_key), shortcut);
        if ui.selectable_label(state.active_tool == id, text).clicked() {
            activate_transform_child(state, id);
            ui.close();
        }
    }
}

/// Aplica ao estado a ordem arrastada e a visibilidade marcada (§49).
///
/// Escritor único do menu de configuração: comparar antes de escrever evita
/// `mark_dirty` a cada frame (o arrasto só produz ordem quando termina).
fn apply_toolbar_config(state: &mut AppState, order: &[String], hidden: &[String]) -> bool {
    let mut changed = false;
    if state.ui.toolbar_order != order {
        state.ui.toolbar_order = order.to_vec();
        changed = true;
    }
    if state.ui.toolbar_hidden != hidden {
        state.ui.toolbar_hidden = hidden.to_vec();
        changed = true;
    }
    changed
}

fn draw_toolbar_config(ui: &mut egui::Ui, state: &mut AppState) {
    let gear_tip = state.t("toolbar.configure");
    let response =
        crate::widgets::PetuniaIconButton::new(PetuniaIcon::Settings, &gear_tip, 24.0).show(ui);
    egui::Popup::menu(&response).show(|ui| {
        // Não imponha 250px a um menu que normalmente precisa de ~180px.
        ui.set_min_width(176.0);
        ui.set_max_width(230.0);
        ui.label(
            egui::RichText::new(state.t("toolbar.configure"))
                .strong()
                .size(12.0)
                .color(tokens::TEXT_PRIMARY),
        );
        ui.separator();

        let mut order: Vec<String> = if state.ui.toolbar_order.is_empty() {
            canonical_toolbar_entries()
                .iter()
                .map(|entry| entry.id.to_string())
                .collect()
        } else {
            state.ui.toolbar_order.clone()
        };
        // Purga ids legados que agora pertencem a grupos/modifiers.
        order.retain(|id| {
            canonical_toolbar_entries()
                .iter()
                .any(|entry| entry.id == id)
        });
        for entry in canonical_toolbar_entries() {
            if !order.iter().any(|id| id == entry.id) {
                order.push(entry.id.to_string());
            }
        }
        state.ui.toolbar_hidden.retain(|id| {
            canonical_toolbar_entries()
                .iter()
                .any(|entry| entry.id == id)
        });

        let mut dirty = false;
        // Wave 7 (§49): a ordem é arrasto, não setas ↑/↓. O adapter é dono do
        // esqueleto da linha (grip + conteúdo) e da aplicação da nova ordem; a
        // linha aqui é só caixa de visibilidade + rótulo.
        let hidden = state.ui.toolbar_hidden.clone();
        egui::ScrollArea::vertical()
            .id_salt("toolbar_config_scroll")
            .max_height(320.0)
            .show(ui, |ui| {
                let mut visible_ids = hidden.clone();
                TOOLBAR_ORDER_DRAG.show(
                    ui,
                    "toolbar_config_order",
                    &mut order,
                    |id| egui::Id::new(("toolbar-order", id.as_str())),
                    |ui, id, row| {
                        let label = canonical_toolbar_entries()
                            .iter()
                            .find(|entry| entry.id == id.as_str())
                            .map(|entry| state.t(entry.label_key))
                            .unwrap_or_else(|| id.clone());
                        let mut visible = !visible_ids.iter().any(|hidden| hidden == id);
                        if ui.checkbox(&mut visible, "").changed() {
                            if visible {
                                visible_ids.retain(|hidden| hidden != id);
                            } else if !visible_ids.iter().any(|hidden| hidden == id) {
                                visible_ids.push(id.clone());
                            }
                        }
                        let color = if row.dragging {
                            tokens::ACCENT_BLUE
                        } else {
                            tokens::TEXT_PRIMARY
                        };
                        ui.label(egui::RichText::new(&label).size(11.5).color(color));
                    },
                );
                // Escritor único: o adapter só devolve a ordem, a linha só
                // marca a visibilidade — quem grava no estado é esta função.
                if apply_toolbar_config(state, &order, &visible_ids) {
                    dirty = true;
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(state.t("toolbar.columns"))
                    .size(11.5)
                    .color(tokens::TEXT_SECONDARY),
            );
            for (cols, key) in [(1u8, "toolbar.one_column"), (2u8, "toolbar.two_columns")] {
                if ui
                    .selectable_label(state.ui.toolbar_columns.clamp(1, 2) == cols, state.t(key))
                    .clicked()
                {
                    state.ui.toolbar_columns = cols;
                    dirty = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(state.t("toolbar.reset")).clicked() {
                    state.ui.toolbar_order.clear();
                    state.ui.toolbar_hidden.clear();
                    state.ui.toolbar_columns = 1;
                    dirty = true;
                }
            });
        });
        if dirty {
            state.mark_dirty();
        }
    });
}

#[cfg(feature = "animation-workspace")]
fn draw_animate_tools(ui: &mut egui::Ui, state: &mut AppState, grid: PetuniaToolGridSpec) {
    grid.show(
        ui,
        "petunia-tool-grid-animate",
        2,
        |index, cell, ui| {
            if index == 0 {
                draw_select_group(ui, state, cell);
            } else {
                draw_transform_group(ui, state, cell);
            }
        },
        |_ui| (),
    );
}

/// Paleta do workspace PAINT: os pincéis, formas e o conta-gotas moram aqui
/// (não no painel de propriedades). O conteúdo canônico é do módulo de pintura —
/// [`crate::modules_ui::paint_ui::draw_tool_palette`] — e esta função só entrega
/// a grade resolvida pelo adapter.
fn draw_paint_tools(ui: &mut egui::Ui, state: &mut AppState, grid: PetuniaToolGridSpec) {
    crate::modules_ui::paint_ui::draw_tool_palette(ui, state, grid);
}

#[allow(dead_code)]
fn draw_uv_tools(ui: &mut egui::Ui, state: &mut AppState, grid: PetuniaToolGridSpec) {
    grid.show(
        ui,
        "petunia-tool-grid-uv",
        1,
        |_index, cell, ui| draw_select_group(ui, state, cell),
        |_ui| (),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::EditMode;

    #[test]
    fn the_drag_order_and_the_visibility_marks_land_in_the_state() {
        let mut state = AppState::new("en");
        let order = vec![
            "rotate".to_string(),
            "select".to_string(),
            "move".to_string(),
        ];
        let hidden = vec!["move".to_string()];
        assert!(
            apply_toolbar_config(&mut state, &order, &hidden),
            "mudança real precisa reportar dirty"
        );
        assert_eq!(state.ui.toolbar_order, order);
        assert_eq!(state.ui.toolbar_hidden, hidden);

        // Mesma ordem de novo: nada a gravar, nada a marcar.
        assert!(!apply_toolbar_config(&mut state, &order, &hidden));
    }

    #[test]
    fn test_toolbar_renders_without_panic_in_all_modes() {
        let ctx = egui::Context::default();
        let tools = ToolRegistry::default();
        for mode in [EditMode::Object, EditMode::Edit, EditMode::TexturePaint] {
            let mut state = AppState::new("en");
            state.set_edit_mode(mode);
            ctx.run_ui(egui::RawInput::default(), |ui| {
                draw_contents(ui, &mut state, &tools);
            })
            .textures_delta
            .clear();
        }
    }

    #[test]
    fn mesh_tools_are_hidden_in_object_mode() {
        let mut state = AppState::new("en");
        state.set_edit_mode(EditMode::Object);
        state.active_tool = "extrude".into();
        let ctx = egui::Context::default();
        let tools = ToolRegistry::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_contents(ui, &mut state, &tools);
        })
        .textures_delta
        .clear();
        assert_eq!(state.active_tool, "select");
    }

    #[test]
    fn modifier_operations_are_not_toolbar_tools() {
        let ids: Vec<&str> = canonical_toolbar_entries()
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert!(!ids.contains(&"mirror"));
        assert!(!ids.contains(&"symmetrize"));
        assert!(ids.contains(&"transform"));
        assert!(!ids.contains(&"move"));
        assert!(!ids.contains(&"rotate"));
        assert!(!ids.contains(&"scale"));
    }

    /// A célula mínima que a coluna de ferramentas promete entrega ao adapter.
    ///
    /// É o contrato cruzado dos dois módulos: `tokens::TOOLBAR_MIN_WIDTH` é a
    /// coluna, e a célula é a coluna menos a moldura — medido, não estimado.
    fn cell_at_minimum_pane() -> f32 {
        tokens::TOOLBAR_MIN_WIDTH - tokens::TOOLBAR_CHROME_WIDTH
    }

    #[test]
    fn the_declared_minimum_fits_a_whole_group_row() {
        let row = plan_group(cell_at_minimum_pane());
        assert!(
            row.chevron,
            "a coluna mínima precisa comportar botão + seta; célula {:.1}",
            cell_at_minimum_pane()
        );
        assert!(
            row.button + tokens::TOOLBAR_CONTROL_GAP + tokens::TOOLBAR_CHEVRON_WIDTH
                <= cell_at_minimum_pane() + f32::EPSILON,
            "o grupo não pode passar da célula: botão {:.1}",
            row.button
        );
        assert!(
            !TOOL_GRID.labels_fit(row.button),
            "a paleta mínima continua sendo trilho de ícones, não de rótulos"
        );
    }

    #[test]
    fn a_cell_too_narrow_for_the_arrow_keeps_the_menu_reachable() {
        let row = plan_group(tokens::TOOLBAR_WIDTH);
        assert!(!row.chevron, "não há largura para desenhar a seta");
        assert_eq!(
            row.button,
            tokens::TOOLBAR_WIDTH,
            "o botão compacto continua inteiro — nenhum controle fora da célula"
        );
    }

    #[test]
    fn a_wide_palette_gives_the_group_button_the_label() {
        let row = plan_group(240.0);
        assert!(row.chevron);
        assert!(
            TOOL_GRID.labels_fit(row.button),
            "coluna larga: o botão do grupo cabe com rótulo ({:.1})",
            row.button
        );
    }

    #[test]
    fn toolbar_config_filters_legacy_entries() {
        let mut state = AppState::new("en");
        state.set_edit_mode(EditMode::Edit);
        state.ui.toolbar_hidden = vec!["knife".to_string(), "mirror".to_string()];
        state.ui.toolbar_order = vec![
            "symmetrize".to_string(),
            "select".to_string(),
            "merge".to_string(),
        ];
        let shown: Vec<&str> = display_entries(&state)
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(&shown[..2], &["select", "merge"]);
        assert!(!shown.contains(&"knife"));
    }
}
