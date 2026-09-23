//! Pintura 2D e pintura sobre o modelo (P3D-061 · P3D-134).
//!
//! O módulo de pintura **não é um painel**: é uma superfície fatiada pelas
//! regiões do shell (perfil PAINT em [`crate::workspaces`]). Cada fatia é uma
//! função pública, e nenhuma delas é duplicada em outro lugar do produto:
//!
//! ```text
//! paleta esquerda →  draw_tool_palette    pincéis, formas, conta-gotas
//! dock · topo     →  draw_layers_section  camadas, efeitos, cena (recolhível)
//! dock · base     →  draw_brush_section   cor, pincel, tela, superfície
//! centro          →  draw_canvas_surface  a tela 2D protagonista
//! ```
//!
//! As três regras que valem aqui (AGENTS.md §0.1 e §3):
//!
//! * nenhuma string visível fora do i18n — `state.t("paint…")`;
//! * nenhum tipo de `egui_dnd`/`egui_taffy`: a reordenação passa pelo
//!   [`PetuniaDragList`] e o arranjo pela [`PetuniaToolGridSpec`];
//! * o traço é interpolado com `BrushSettings::stroke_dabs`, então a densidade
//!   dos dabs não depende da taxa de eventos do ponteiro.

use egui::{
    Color32, ComboBox, Id, Rect, RichText, ScrollArea, Sense, Slider, StrokeKind, TextEdit, Ui,
};
use petunia_config::{TextId, text_id};
use petunia_core::AppState;
use petunia_module_paint::{BrushType, PaintModule};
use petunia_module_uv::UvModule;
use petunia_project::{LayerBlendMode, LayerKind, PaintEffect, PaintLayer, TextureChannel};

use crate::adapters::drag_drop::{PetuniaDragList, PetuniaDragSpec};
use crate::adapters::popup::PetuniaPopup;
use crate::adapters::tool_grid::{PetuniaToolCell, PetuniaToolGridSpec};
use crate::foundation::motion::PetuniaMotion;
use crate::icon_registry::PetuniaIcon;
use crate::tokens;
use crate::widgets::{self, PetuniaIconButton, PetuniaToolbarButton};

/// Chave da textura da tela 2D na memória do `Context`.
const CANVAS_TEXTURE_ID: &str = "paint.canvas_tex";

const FILL_SCOPES: [petunia_core::FillScope; 5] = [
    petunia_core::FillScope::ConnectedPixels,
    petunia_core::FillScope::Face,
    petunia_core::FillScope::SelectedFaces,
    petunia_core::FillScope::UvIsland,
    petunia_core::FillScope::Object,
];

fn fill_scope_label(scope: petunia_core::FillScope) -> TextId {
    match scope {
        petunia_core::FillScope::ConnectedPixels => text_id::PAINT_FILL_CONNECTED_PIXELS,
        petunia_core::FillScope::Face => text_id::PAINT_FILL_FACE,
        petunia_core::FillScope::SelectedFaces => text_id::PAINT_FILL_SELECTED_FACES,
        petunia_core::FillScope::UvIsland => text_id::PAINT_FILL_UV_ISLAND,
        petunia_core::FillScope::Object => text_id::PAINT_FILL_OBJECT,
    }
}

/// Modos de mistura oferecidos pelo seletor, na ordem de exibição.
///
/// A ordem das chaves i18n ([`blend_key`]) e dos rótulos em [`LayerLabels`]
/// segue **esta** lista: as três precisam andar juntas.
const BLEND_MODES: [LayerBlendMode; 4] = [
    LayerBlendMode::Normal,
    LayerBlendMode::Multiply,
    LayerBlendMode::Add,
    LayerBlendMode::Screen,
];

/// Altura/quadro do botão de ícone das listas do módulo (grip e olho).
const ROW_ICON: f32 = 18.0;

// ---------------------------------------------------------------------------
// Paleta de ferramentas (coluna esquerda)
// ---------------------------------------------------------------------------

/// Grupo da paleta de pintura. A ordem dos grupos é a ordem na coluna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintToolSection {
    /// Pincéis que carimbam tinta (inclui a borracha).
    Brushes,
    /// Ferramentas de forma e preenchimento.
    Shapes,
    /// Amostragem de cor.
    Sample,
}

impl PaintToolSection {
    /// Grupos na ordem em que aparecem na paleta.
    pub const ALL: [PaintToolSection; 3] = [Self::Brushes, Self::Shapes, Self::Sample];

    /// Chave estável do grupo (escopo dos `Id` da grade de taffy).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Brushes => "brushes",
            Self::Shapes => "shapes",
            Self::Sample => "sample",
        }
    }

    /// Rótulo do grupo no i18n (usado no tooltip e na a11y).
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Brushes => "paint.tool_section_brushes",
            Self::Shapes => "paint.tool_section_shapes",
            Self::Sample => "paint.tool_section_sample",
        }
    }
}

/// Ferramenta da paleta de pintura.
pub struct PaintTool {
    /// Índice do contrato (`paint_brush_kind`); o mapeamento para `BrushType`
    /// vive em [`brush_type_of`] e **não** pode mudar sem migração de projeto.
    pub kind: usize,
    pub section: PaintToolSection,
    pub icon: PetuniaIcon,
    pub label_key: &'static str,
}

/// Conteúdo canônico da paleta de pintura (ordem da coluna esquerda).
pub fn canonical_paint_tools() -> Vec<PaintTool> {
    use PetuniaIcon::{PaintBrush, PaintEraser, PaintFill, PaintLine, PaintPicker, PaintRect};
    vec![
        // Pincéis: sólido, suave, aéreo (Airbrush) e borracha.
        PaintTool {
            kind: 0,
            section: PaintToolSection::Brushes,
            icon: PaintBrush,
            label_key: "paint.brush_pixel",
        },
        PaintTool {
            kind: 1,
            section: PaintToolSection::Brushes,
            icon: PaintBrush,
            label_key: "paint.brush_soft",
        },
        PaintTool {
            kind: 7,
            section: PaintToolSection::Brushes,
            icon: PaintBrush,
            label_key: "paint.brush_airbrush",
        },
        PaintTool {
            kind: 2,
            section: PaintToolSection::Brushes,
            icon: PaintEraser,
            label_key: "paint.brush_eraser",
        },
        // Formas e preenchimento.
        PaintTool {
            kind: 5,
            section: PaintToolSection::Shapes,
            icon: PaintLine,
            label_key: "paint.brush_line",
        },
        PaintTool {
            kind: 6,
            section: PaintToolSection::Shapes,
            icon: PaintRect,
            label_key: "paint.brush_rect",
        },
        PaintTool {
            kind: 3,
            section: PaintToolSection::Shapes,
            icon: PaintFill,
            label_key: "paint.brush_fill",
        },
        // Amostragem.
        PaintTool {
            kind: 4,
            section: PaintToolSection::Sample,
            icon: PaintPicker,
            label_key: "paint.brush_picker",
        },
    ]
}

/// Tipo de pincel do contrato para o índice persistido em `paint_brush_kind`.
pub fn brush_type_of(kind: usize) -> BrushType {
    match kind {
        1 => BrushType::Soft,
        2 => BrushType::Eraser,
        3 => BrushType::Fill,
        4 => BrushType::Eyedropper,
        5 => BrushType::Line,
        6 => BrushType::Rectangle,
        7 => BrushType::Airbrush,
        _ => BrushType::Pixel,
    }
}

/// Paleta de ferramentas do workspace PAINT (coluna esquerda).
///
/// Cada grupo é uma grade própria: a quebra de linha do adapter acontece por
/// grupo, então o separador nunca cai no meio de uma família de ferramentas.
pub fn draw_tool_palette(ui: &mut Ui, state: &mut AppState, grid: PetuniaToolGridSpec) {
    let tools = canonical_paint_tools();
    let mut seen_section = false;
    for section in PaintToolSection::ALL {
        let indices: Vec<usize> = tools
            .iter()
            .enumerate()
            .filter(|(_, tool)| tool.section == section)
            .map(|(index, _)| index)
            .collect();
        if indices.is_empty() {
            continue;
        }
        if seen_section {
            ui.add_space(3.0);
            ui.separator();
            ui.add_space(3.0);
        }
        seen_section = true;
        grid.show(
            ui,
            Id::new(("petunia-tool-grid-paint", section.key())),
            indices.len(),
            |index, cell, ui| draw_paint_tool(ui, state, &tools[indices[index]], cell),
            |_ui| (),
        );
    }
}

fn draw_paint_tool(ui: &mut Ui, state: &mut AppState, tool: &PaintTool, cell: PetuniaToolCell) {
    let label = state.t(tool.label_key);
    let active = state.active_tool == "paint" && state.paint_brush_kind == tool.kind;
    if PetuniaToolbarButton::new(tool.icon, &label)
        .selected(active)
        .compact(!cell.labeled)
        .width(cell.width)
        .tooltip(&label)
        .show(ui)
        .clicked()
    {
        state.active_tool = "paint".into();
        state.paint_brush_kind = tool.kind;
        // Escolher a ferramenta traz o cartão flutuante de volta: ele recolhe ao
        // perder o foco, e este é o gesto que o reabre (§34).
        PetuniaPopup::reveal(ui.ctx(), crate::tool_properties_popover::PAINT_CARD_ID);
        state.mark_dirty();
    }
}

/// Cor de paleta em `[f32; 3]` → `Color32` de tela.
fn swatch_color(color: [f32; 3]) -> Color32 {
    Color32::from_rgb(
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

/// Duas cores são a mesma (tolerância de float de quem veio do seletor).
fn same_color(a: [f32; 3], b: [f32; 3]) -> bool {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() < 1e-3
}

/// Hex de exibição da cor atual (`#RRGGBB`).
fn color_hex(color: [f32; 3]) -> String {
    let c = swatch_color(color);
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

/// Botão de uma cor da paleta.
///
/// É um `Button` do egui com preenchimento — não um seletor completo: clicar
/// numa cor da paleta **escolhe** a cor, não abre outro seletor em cascata (o
/// que multiplicava popups e ainda por cima escrevia na paleta).
fn palette_swatch(ui: &mut Ui, color: [f32; 3], selected: bool, tooltip: &str) -> bool {
    let stroke = if selected {
        egui::Stroke::new(1.5, tokens::TEXT_ACTIVE)
    } else {
        tokens::stroke_subtle()
    };
    ui.add(
        egui::Button::new("")
            .fill(swatch_color(color))
            .stroke(stroke)
            .min_size(egui::vec2(tokens::SWATCH_SIZE, tokens::SWATCH_SIZE)),
    )
    .on_hover_text(format!("{} {}", tooltip, color_hex(color)))
    .clicked()
}

// ---------------------------------------------------------------------------
// Camadas (seção superior do dock)
// ---------------------------------------------------------------------------

/// Ação de camada coletada durante o desenho e aplicada no fim do frame.
///
/// O adapter de arrasto chama o closure da linha mais de uma vez por frame
/// (medida + desenho, e uma cópia flutuante durante o arrasto): mutar o estado
/// dentro dele corromperia a pilha. Por isso o desenho só **coleta**.
enum LayerAction {
    Activate(uuid::Uuid),
    ToggleVisible(uuid::Uuid),
    Opacity(uuid::Uuid, f32),
    Blend(uuid::Uuid, LayerBlendMode),
    Rename(uuid::Uuid, String),
    Effect(uuid::Uuid, PaintEffect),
    Reorder(Vec<uuid::Uuid>),
    Delete(uuid::Uuid),
    Duplicate(uuid::Uuid),
    /// Cria uma camada de efeito do índice escolhido no seletor.
    AddEffect(usize),
}

/// Linha da pilha de camadas no frame (snapshot; nada de borrow vivo).
#[derive(Clone)]
struct LayerRow {
    id: uuid::Uuid,
    name: String,
    visible: bool,
    opacity: f32,
    blend: LayerBlendMode,
    effect: Option<PaintEffect>,
}

/// Rótulos da lista resolvidos uma vez por frame (i18n fora dos closures).
struct LayerLabels {
    show: String,
    hide: String,
    /// Nome de cada modo de mistura, na ordem de [`LayerBlendMode::ALL`].
    blend_names: [String; 4],
}

/// Seção **Camadas** do dock: ações, camada ativa e a lista reordenável.
///
/// A lista é o único lugar que reordena (arrasto pelo grip do
/// [`PetuniaDragList`]); não há setas ↑/↓. A cena (árvore de objetos) fica
/// recolhível no fim da seção para não competir com as camadas.
pub fn draw_layers_section(ui: &mut Ui, state: &mut AppState) {
    PaintModule::ensure_stack(state);
    let mut actions: Vec<LayerAction> = Vec::new();
    ScrollArea::vertical()
        .id_salt("paint_layers_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            draw_layer_actions(ui, state, &mut actions);
            ui.separator();
            draw_active_layer_block(ui, state, &mut actions);
            ui.separator();
            draw_layer_list(ui, state, &mut actions);
            ui.separator();
            draw_scene_block(ui, state);
        });
    apply_layer_actions(state, actions);
}

/// Barra de ações da pilha: nova camada, duplicar, excluir e adicionar efeito.
fn draw_layer_actions(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<LayerAction>) {
    let rows = layer_rows(state);
    let active = rows
        .iter()
        .position(|row| Some(row.id) == active_layer_id(state));
    ui.horizontal(|ui| {
        let add_tip = state.t("paint.layer_new");
        if PetuniaIconButton::new(PetuniaIcon::Plus, &add_tip, ROW_ICON)
            .show(ui)
            .clicked()
        {
            add_raster_layer(state);
        }
        let dup_tip = state.t("paint.layer_duplicate");
        ui.add_enabled_ui(active.is_some(), |ui| {
            if PetuniaIconButton::new(PetuniaIcon::Duplicate, &dup_tip, ROW_ICON)
                .show(ui)
                .clicked()
                && let Some(index) = active
            {
                actions.push(LayerAction::Duplicate(rows[index].id));
            }
        });
        let del_tip = state.t("paint.layer_delete");
        let can_delete = rows.len() > 1;
        ui.add_enabled_ui(can_delete, |ui| {
            if PetuniaIconButton::new(PetuniaIcon::Trash, &del_tip, ROW_ICON)
                .show(ui)
                .clicked()
                && let Some(index) = active
            {
                actions.push(LayerAction::Delete(rows[index].id));
            }
        });
    });

    // Efeitos não-destrutivos (P3D-134): escolhe o tipo, cria como camada nova.
    let effect_id = Id::new("paint.selected_effect");
    let effect_index = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(effect_id))
        .unwrap_or(0);
    let names: Vec<&'static str> = (0..EFFECT_COUNT).map(effect_name_key).collect();
    let selected = names[effect_index.min(names.len() - 1)];
    ui.horizontal(|ui| {
        ComboBox::from_id_salt("paint_effect_kind")
            .selected_text(state.t(selected))
            .show_ui(ui, |ui| {
                for (index, key) in names.iter().enumerate() {
                    if ui
                        .selectable_label(effect_index == index, state.t(key))
                        .clicked()
                    {
                        ui.ctx().data_mut(|d| d.insert_temp(effect_id, index));
                    }
                }
            });
        let add_label = state.t("paint.effect_add");
        if ui
            .button(add_label)
            .on_hover_text(state.t("paint.effect_add_tip"))
            .clicked()
        {
            actions.push(LayerAction::AddEffect(effect_index.min(names.len() - 1)));
        }
    });
}

/// Bloco da camada ativa: nome, modo de mistura e opacidade.
///
/// É o painel de propriedades da camada (padrão Photoshop): a lista fica limpa
/// e os controles valem para o que está selecionado.
fn draw_active_layer_block(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<LayerAction>) {
    let rows = layer_rows(state);
    let Some(row) = rows
        .iter()
        .find(|row| Some(row.id) == active_layer_id(state))
        .cloned()
    else {
        ui.small(state.t("paint.layer_empty"));
        return;
    };
    let mut name = row.name.clone();
    let response = ui.add(
        TextEdit::singleline(&mut name)
            .desired_width(f32::INFINITY)
            .hint_text(row.name.clone()),
    );
    if response.lost_focus() && name.trim() != row.name && !name.trim().is_empty() {
        actions.push(LayerAction::Rename(row.id, name.trim().to_string()));
    }

    let blend_label = state.t("paint.layer_blend");
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(blend_label)
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        ComboBox::from_id_salt("paint_layer_blend")
            .selected_text(state.t(blend_key(row.blend)))
            .show_ui(ui, |ui| {
                for mode in BLEND_MODES {
                    if ui
                        .selectable_label(row.blend == mode, state.t(blend_key(mode)))
                        .clicked()
                    {
                        actions.push(LayerAction::Blend(row.id, mode));
                    }
                }
            });
    });

    let mut opacity = row.opacity;
    let response = ui.add(
        Slider::new(&mut opacity, 0.0..=1.0)
            .text(state.t("paint.layer_opacity"))
            .show_value(true),
    );
    // Um checkpoint por gesto: o carimbo acontece no início do arrasto.
    if response.drag_started() {
        state.checkpoint("layer opacity");
    }
    if response.changed() {
        actions.push(LayerAction::Opacity(row.id, opacity));
    }

    if let Some(effect) = row.effect {
        draw_effect_parameters(ui, state, row.id, effect, actions);
    } else {
        ui.small(state.t("paint.layer_kind_hint"));
    }
}

/// Lista reordenável das camadas (a única superfície de reordenação).
fn draw_layer_list(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<LayerAction>) {
    let mut rows = layer_rows(state);
    if rows.is_empty() {
        ui.small(state.t("paint.layer_empty"));
        return;
    }
    let labels = LayerLabels {
        show: state.t("paint.layer_show"),
        hide: state.t("paint.layer_hide"),
        blend_names: [
            state.t("paint.blend_normal"),
            state.t("paint.blend_multiply"),
            state.t("paint.blend_add"),
            state.t("paint.blend_screen"),
        ],
    };
    let active = active_layer_id(state);
    let order_before: Vec<uuid::Uuid> = rows.iter().map(|row| row.id).collect();
    let changed = PetuniaDragList::new(PetuniaDragSpec::new()).show(
        ui,
        "paint_layers_drag_list",
        &mut rows,
        |row| Id::new(("paint-layer", row.id)),
        |ui, row, _drag| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let (icon, tip) = if row.visible {
                        (PetuniaIcon::Eye, labels.hide.as_str())
                    } else {
                        (PetuniaIcon::EyeHidden, labels.show.as_str())
                    };
                    if PetuniaIconButton::new(icon, tip, ROW_ICON)
                        .show(ui)
                        .clicked()
                    {
                        actions.push(LayerAction::ToggleVisible(row.id));
                    }
                    let mut text = RichText::new(layer_title(row)).size(11.5);
                    if !row.visible {
                        text = text.color(tokens::TEXT_MUTED);
                    }
                    if ui.selectable_label(active == Some(row.id), text).clicked() {
                        actions.push(LayerAction::Activate(row.id));
                    }
                });
                // Resumo: o que está fora do padrão aparece na própria linha.
                let summary = layer_summary(row, &labels);
                if !summary.is_empty() {
                    ui.small(RichText::new(summary).size(10.0).color(tokens::TEXT_MUTED));
                }
            });
        },
    );
    if changed {
        let order_after: Vec<uuid::Uuid> = rows.iter().map(|row| row.id).collect();
        if order_after != order_before {
            actions.push(LayerAction::Reorder(order_after));
        }
    }
}

/// Título da linha: nome + marca de efeito.
fn layer_title(row: &LayerRow) -> String {
    if row.effect.is_some() {
        format!("fx · {}", row.name)
    } else {
        row.name.clone()
    }
}

/// Resumo da linha quando o modo/opacidade saem do padrão.
fn layer_summary(row: &LayerRow, labels: &LayerLabels) -> String {
    let mut parts: Vec<String> = Vec::new();
    if row.blend != LayerBlendMode::Normal {
        parts.push(labels.blend_names[blend_index(row.blend)].clone());
    }
    if row.opacity < 0.999 {
        parts.push(format!("{:.0}%", row.opacity * 100.0));
    }
    parts.join(" · ")
}

/// Índice do modo de mistura na ordem de [`BLEND_MODES`].
fn blend_index(mode: LayerBlendMode) -> usize {
    BLEND_MODES
        .iter()
        .position(|candidate| *candidate == mode)
        .unwrap_or(0)
}

/// Seção **Cena** recolhível dentro do dock de pintura.
///
/// O estado de abertura é efêmero do frame (`Context`), não do projeto: é
/// preferência de sessão e não justifica um campo em `UiState`.
fn draw_scene_block(ui: &mut Ui, state: &mut AppState) {
    let open_id = Id::new("paint.scene_open");
    let mut open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let tip = state.t(if open { "ui.collapse" } else { "ui.expand" });
    ui.horizontal(|ui| {
        if widgets::chevron_toggle(ui, &tip, open).clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(open_id, open));
        }
        ui.label(
            RichText::new(state.t_id(text_id::UI_OUTLINER))
                .size(11.5)
                .strong()
                .color(tokens::TEXT_PRIMARY),
        );
    });
    PetuniaMotion::section(ui, Id::new("paint.scene_section"), open, |ui| {
        crate::outliner::draw_tree_only(ui, state);
    });
}

// ---------------------------------------------------------------------------
// Pincel, cor e tela (seção inferior do dock)
// ---------------------------------------------------------------------------

/// Entrada histórica do módulo de pintura.
///
/// O mapa de componentes (`docs/public/ui-map.json`, congelado — AGENTS.md §1)
/// aponta os nós de pintura para este símbolo. O desenho real está fatiado por
/// região ([`draw_tool_palette`], [`draw_layers_section`],
/// [`draw_brush_section`]); aqui fica apenas o nome que o mapa procura, com o
/// contrato que ele declara. Nenhum caminho de produto chama esta função.
pub fn draw_paint_panel(
    ui: &mut Ui,
    state: &mut AppState,
    _canvas_texture: &mut Option<egui::TextureHandle>,
) {
    draw_brush_contents(ui, state);
}

/// Conteúdo do pincel **sem** área de rolagem própria.
///
/// É o corpo do cartão flutuante da ferramenta (`PetuniaPopup::panel`, que já
/// rola o corpo) e da seção legada do workspace UV: duas rolagens aninhadas
/// esmagam o conteúdo em vez de rolar.
pub fn draw_brush_contents(ui: &mut Ui, state: &mut AppState) {
    draw_color_block(ui, state);
    ui.separator();
    draw_brush_block(ui, state);
    ui.separator();
    draw_canvas_block(ui, state);
    ui.separator();
    draw_surface_block(ui, state);
}

/// Bloco de cor: **cor atual**, **paleta do projeto** e ações.
///
/// Hierarquia explícita (padrão de editor de pintura): primeiro o que está sendo
/// usado agora — um seletor, com o valor em hex —, depois a paleta curada.
///
/// Mudar a cor atual **não** escreve na paleta. Antes, cada quadro com o seletor
/// aberto empurrava um tom intermediário para a paleta (`changed()` dispara a
/// cada arrasto de slider), e ao clicar nas cores a paleta virava um borrão de
/// tons que o usuário nunca escolheu. A paleta agora só muda por ação explícita:
/// `+`, presets, importação e o conta-gotas.
fn draw_color_block(ui: &mut Ui, state: &mut AppState) {
    let current_label = state.t("paint.color_current");
    let current_tip = state.t("paint.color_current_tip");
    let mut color = state.paint_color;
    ui.horizontal(|ui| {
        if ui
            .color_edit_button_rgb(&mut color)
            .on_hover_text(current_tip)
            .changed()
        {
            // Só a cor em uso muda: a paleta é do usuário, não do seletor.
            state.paint_color = color;
            state.mark_dirty();
        }
        ui.vertical(|ui| {
            ui.label(
                RichText::new(current_label)
                    .size(11.0)
                    .color(tokens::TEXT_SECONDARY),
            );
            ui.label(
                RichText::new(color_hex(color))
                    .size(10.5)
                    .monospace()
                    .color(tokens::TEXT_MUTED),
            );
        });
    });

    let palette = state.project.palette.clone();
    let selected = state.paint_color;
    let use_tip = state.t("paint.palette_use");
    let mut pick = None;
    ui.horizontal_wrapped(|ui| {
        for entry in palette {
            if palette_swatch(ui, entry, same_color(entry, selected), &use_tip) {
                pick = Some(entry);
            }
        }
    });
    if let Some(entry) = pick {
        state.paint_color = entry;
        state.mark_dirty();
    }

    ui.horizontal_wrapped(|ui| {
        let add_label = state.t("paint.palette_add");
        if ui
            .small_button(&add_label)
            .on_hover_text(state.t("paint.palette_add_tip"))
            .clicked()
        {
            let color = state.paint_color;
            PaintModule::push_palette(state, color);
        }
        let remove_label = state.t("paint.palette_remove");
        let can_remove = state
            .project
            .palette
            .iter()
            .any(|entry| same_color(*entry, state.paint_color));
        if ui
            .add_enabled(can_remove, egui::Button::new(remove_label.clone()))
            .on_hover_text(state.t("paint.palette_remove_tip"))
            .clicked()
        {
            let current = state.paint_color;
            let kept: Vec<[f32; 3]> = state
                .project
                .palette
                .iter()
                .copied()
                .filter(|entry| !same_color(*entry, current))
                .collect();
            PaintModule::set_palette(state, kept);
        }
        let clear_label = state.t("paint.palette_clear");
        if ui.small_button(&clear_label).clicked() {
            PaintModule::set_palette(state, Vec::new());
        }
        // Marcas registradas: sem tradução.
        if ui.small_button("PICO-8").clicked() {
            PaintModule::set_palette(state, petunia_project::preset_pico8());
            let message = state.t("paint.palette_loaded_pico8");
            state.set_status(&message);
        }
        if ui.small_button("GameBoy").clicked() {
            PaintModule::set_palette(state, petunia_project::preset_gameboy());
            let message = state.t("paint.palette_loaded_gameboy");
            state.set_status(&message);
        }
        let import_label = state.t("paint.palette_import");
        if ui
            .small_button(&import_label)
            .on_hover_text(state.t("paint.palette_import_tip"))
            .clicked()
        {
            state
                .events
                .emit(petunia_core::AppEvent::RequestImportPalette);
        }
        let export_label = state.t("paint.palette_export");
        if ui
            .small_button(&export_label)
            .on_hover_text(state.t("paint.palette_export_tip"))
            .clicked()
        {
            state
                .events
                .emit(petunia_core::AppEvent::RequestExportPalette);
        }
    });
}

/// Descritor do pincel: um único `BrushSettings` para tela 2D e modelo 3D.
fn draw_brush_block(ui: &mut Ui, state: &mut AppState) {
    let l_size = state.t("paint.size");
    let t_size = state.t("paint.size_tip");
    let l_hardness = state.t("paint.hardness");
    let t_hardness = state.t("paint.hardness_tip");
    let l_strength = state.t("paint.strength");
    let t_strength = state.t("paint.strength_tip");
    let l_flow = state.t("paint.flow");
    let t_flow = state.t("paint.flow_tip");
    let l_spacing = state.t("paint.spacing");
    let t_spacing = state.t("paint.spacing_tip");
    let mut settings = state.brush_settings();
    let mut changed = false;
    changed |= ui
        .add(Slider::new(&mut settings.size_px, 1.0..=512.0).text(l_size))
        .on_hover_text(t_size)
        .changed();
    changed |= ui
        .add(Slider::new(&mut settings.hardness, 0.0..=1.0).text(l_hardness))
        .on_hover_text(t_hardness)
        .changed();
    changed |= ui
        .add(Slider::new(&mut settings.strength, 0.0..=1.0).text(l_strength))
        .on_hover_text(t_strength)
        .changed();
    changed |= ui
        .add(Slider::new(&mut settings.flow, 0.0..=1.0).text(l_flow))
        .on_hover_text(t_flow)
        .changed();
    changed |= ui
        .add(Slider::new(&mut settings.spacing, 0.01..=1.0).text(l_spacing))
        .on_hover_text(t_spacing)
        .changed();
    if changed {
        state.canvas_brush = settings.size_px.round().max(1.0) as u32;
        state.paint_brush_kind = petunia_core::kind_from_brush_type(settings.kind);
        state.brush_hardness = settings.hardness;
        state.paint_strength = settings.strength;
        state.brush_flow = settings.flow;
        state.brush_spacing = settings.spacing;
        state.mark_dirty();
    }

    if state.paint_brush_kind == 3 {
        ui.horizontal(|ui| {
            ui.label(state.t_id(text_id::PAINT_FILL_SCOPE));
            let mut scope_changed = false;
            ComboBox::from_id_salt("paint_fill_scope")
                .selected_text(state.t_id(fill_scope_label(state.fill_scope)))
                .show_ui(ui, |ui| {
                    for scope in FILL_SCOPES {
                        let label = state.t_id(fill_scope_label(scope));
                        scope_changed |= ui
                            .selectable_value(&mut state.fill_scope, scope, label)
                            .changed();
                    }
                });
            if scope_changed {
                state.mark_dirty();
            }
        });
    }

    ui.horizontal(|ui| {
        let fill_label = state.t("paint.fill_sel");
        if ui
            .button(fill_label)
            .on_hover_text(state.t("paint.fill_sel_tip"))
            .clicked()
        {
            let faces = PaintModule::fill_selection(state);
            let message = state
                .t("paint.fill_done")
                .replace("{n}", &faces.to_string());
            state.set_status(&message);
            state.sync_selection();
        }
        let pick_label = state.t("paint.pick");
        if ui
            .button(pick_label)
            .on_hover_text(state.t("paint.pick_hint"))
            .clicked()
        {
            state.set_status(state.t("paint.pick_hint"));
        }
    });

    // Alvo 3D: isolar faces e grade de pixels da tela 2D.
    let isolate_label = state.t("paint.isolate_faces");
    ui.checkbox(&mut state.paint_isolate_selection, isolate_label)
        .on_hover_text(state.t("paint.isolate_faces_tip"));
    let grid_label = state.t("paint.pixel_grid");
    ui.checkbox(&mut state.paint_pixel_grid, grid_label)
        .on_hover_text(state.t("paint.pixel_grid_tip"));
}

/// Operações da tela 2D e canal alvo (V1: Albedo; resto desabilitado).
fn draw_canvas_block(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(state.t("paint.channel"))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        ComboBox::from_id_salt("paint_channel_combo")
            .selected_text(state.t("paint.channel_albedo"))
            .show_ui(ui, |ui| {
                let albedo_label = state.t("paint.channel_albedo");
                ui.selectable_value(
                    &mut state.paint_channel,
                    TextureChannel::Albedo,
                    albedo_label,
                );
                for key in [
                    "paint.channel_normal",
                    "paint.channel_roughness",
                    "paint.channel_metallic",
                    "paint.channel_emission",
                    "paint.channel_height",
                ] {
                    let label = state.t(key);
                    ui.add_enabled_ui(false, |ui| {
                        let _ = ui.selectable_label(false, label);
                    });
                }
                ui.small(state.t("paint.channel_locked_tip"));
            });
    });

    ui.horizontal(|ui| {
        let fill_label = state.t("paint.fill");
        if ui.button(fill_label).clicked() {
            state.checkpoint("canvas fill");
            PaintModule::canvas_fill(state);
        }
        let clear_label = state.t("paint.clear");
        if ui.button(clear_label).clicked() {
            state.checkpoint("canvas clear");
            PaintModule::canvas_clear(state);
            state.mark_dirty();
        }
        let new_label = state.t("paint.new_canvas");
        if ui
            .button(new_label)
            .on_hover_text(state.t("paint.new_canvas_tip"))
            .clicked()
        {
            state.checkpoint("canvas new");
            if let Some(object) = state.project.active_mut() {
                let base = object.base_color;
                object.texture = Some(petunia_project::Canvas::new(
                    256,
                    256,
                    [
                        (base[0] * 255.0) as u8,
                        (base[1] * 255.0) as u8,
                        (base[2] * 255.0) as u8,
                        255,
                    ],
                ));
                // A pilha recomeça junto (representação única, sem divergir).
                object.paint_stack = None;
            }
            PaintModule::ensure_stack(state);
            PaintModule::composite_active(state);
        }
    });

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(state.t("paint.canvas_size"))
                .size(11.0)
                .color(tokens::TEXT_SECONDARY),
        );
        let current = canvas_size(state);
        for side in [64u32, 128, 256, 512, 1024] {
            let selected = current == (side, side);
            if ui
                .selectable_label(selected, format!("{side}"))
                .on_hover_text(state.t("paint.canvas_resize"))
                .clicked()
                && !selected
            {
                state.checkpoint("canvas resize");
                PaintModule::resize_canvas(state, side, side);
                state.render.canvas_dirty = true;
            }
        }
    });
}

/// Preparo de superfície: as projeções que substituíram o editor UV na V1.
fn draw_surface_block(ui: &mut Ui, state: &mut AppState) {
    ui.label(
        RichText::new(state.t("paint.prepare_surface"))
            .size(11.5)
            .strong()
            .color(tokens::TEXT_PRIMARY),
    );
    ui.small(state.t("paint.prepare_surface_hint"));
    ui.horizontal_wrapped(|ui| {
        let planar_label = state.t("paint.surface_planar");
        if ui
            .button(planar_label)
            .on_hover_text(state.t("paint.surface_planar_tip"))
            .clicked()
        {
            UvModule::reproject(state);
        }
        let box_label = state.t("paint.surface_box");
        if ui
            .button(box_label)
            .on_hover_text(state.t("paint.surface_box_tip"))
            .clicked()
        {
            UvModule::project_cube(state);
        }
        let unwrap_label = state.t("paint.surface_auto_unwrap");
        if ui
            .button(unwrap_label)
            .on_hover_text(state.t("paint.surface_auto_unwrap_tip"))
            .clicked()
        {
            match UvModule::unwrap_auto(state) {
                Ok(charts) => state.set_status(
                    state
                        .t("paint.surface_unwrapped")
                        .replace("{n}", &charts.to_string()),
                ),
                Err(error) => state.set_status(error),
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Tela 2D (centro)
// ---------------------------------------------------------------------------

/// Desenha a tela 2D no centro e resolve a pintura com dabs interpolados.
///
/// A tela é `albedo` composto da camada ativa (`PaintModule::composite_active`
/// mantém uma única representação). O traço usa `brush_settings()` — o mesmo
/// descritor que a viewport 3D usa no anel de preview.
pub fn draw_canvas_surface(ui: &mut Ui, state: &mut AppState) {
    let pane = ui.available_rect_before_wrap();
    ui.painter()
        .rect_filled(pane, 0.0, tokens::bg_canvas(state));
    if !PaintModule::has_canvas(state) {
        state.checkpoint("canvas new");
        PaintModule::ensure_canvas(state);
    }
    let (canvas_w, canvas_h) = canvas_size(state);
    if canvas_w == 0 || canvas_h == 0 {
        return;
    }
    let texture_id = Id::new(CANVAS_TEXTURE_ID);
    let mut texture: Option<egui::TextureHandle> = ui.ctx().data_mut(|d| d.get_temp(texture_id));
    if state.render.canvas_dirty || texture.is_none() {
        if let Some(canvas) = active_canvas(state) {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [canvas.w as usize, canvas.h as usize],
                &canvas.pixels,
            );
            texture = Some(ui.ctx().load_texture(
                "paint_canvas",
                image,
                egui::TextureOptions::NEAREST,
            ));
        }
        state.render.canvas_dirty = false;
    }
    if texture.is_none() {
        return;
    }
    if let Some(handle) = texture.as_ref() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(texture_id, handle.clone()));
    }

    // Tela centralizada, com no mínimo 1px por texel.
    let scale = ((pane.width() - 32.0) / canvas_w as f32)
        .min((pane.height() - 32.0) / canvas_h as f32)
        .max(1.0);
    let rect = Rect::from_center_size(
        pane.center(),
        egui::vec2(canvas_w as f32 * scale, canvas_h as f32 * scale),
    );
    let painter = ui.painter_at(rect);
    draw_checkerboard(&painter, rect, state);
    if let Some(handle) = texture.as_ref() {
        painter.image(
            handle.id(),
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    painter.rect_stroke(
        rect,
        0.0,
        tokens::stroke_border_dyn(state),
        StrokeKind::Outside,
    );
    if state.paint_pixel_grid && scale >= 3.0 {
        let mut x = rect.min.x;
        while x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
            );
            x += scale;
        }
        let mut y = rect.min.y;
        while y <= rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
            );
            y += scale;
        }
    }

    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    if state.is_interacting() {
        return;
    }
    // O retângulo da tela é registrado pelo paine que a hospeda (`paint_center`):
    // um escritor por slot, e o slot é o do paine, não o do texel.
    let brush = if ui.input(|i| i.modifiers.ctrl) {
        BrushType::Eraser
    } else {
        brush_type_of(state.paint_brush_kind)
    };
    let view = CanvasView {
        rect,
        scale,
        w: canvas_w,
        h: canvas_h,
    };
    match brush {
        BrushType::Line | BrushType::Rectangle => canvas_shape(ui, state, view, brush, &response),
        BrushType::Fill | BrushType::Eyedropper => canvas_single_dab(state, view, brush, &response),
        _ => canvas_stroke(ui, state, view, brush, &response),
    }
}

/// Geometria da tela resolvida no frame: retângulo na tela, escala (pixels de
/// tela por texel) e tamanho em texels.
///
/// Agrupa o que **toda** interação de tela precisa, para as assinaturas não
/// crescerem em parâmetros soltos (e para a conversão tela↔texel ter um dono só).
#[derive(Clone, Copy)]
struct CanvasView {
    rect: Rect,
    scale: f32,
    w: u32,
    h: u32,
}

impl CanvasView {
    /// Texel sob uma posição de tela, preso à tela.
    fn texel(&self, pos: egui::Pos2) -> (u32, u32) {
        let x = (((pos.x - self.rect.min.x) / self.scale) as u32).min(self.w.saturating_sub(1));
        let y = (((pos.y - self.rect.min.y) / self.scale) as u32).min(self.h.saturating_sub(1));
        (x, y)
    }

    /// Posição de tela do canto superior esquerdo de um texel.
    fn screen(&self, (x, y): (u32, u32)) -> egui::Pos2 {
        egui::pos2(
            self.rect.min.x + x as f32 * self.scale,
            self.rect.min.y + y as f32 * self.scale,
        )
    }
}

/// Traço contínuo: interpola dabs entre dois eventos de ponteiro.
fn canvas_stroke(
    ui: &mut Ui,
    state: &mut AppState,
    view: CanvasView,
    brush: BrushType,
    response: &egui::Response,
) {
    if !(response.dragged() || response.clicked()) {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    // Um clique sem arrasto também é uma pincelada: carimba o checkpoint no
    // mesmo frame em que muta (o motor de undo exige carimbo antes da mutação).
    if response.drag_started() || response.clicked() {
        state.checkpoint("canvas paint");
    }
    let (px, py) = view.texel(pos);
    let mut settings = state.brush_settings();
    settings.kind = brush;
    let previous_id = Id::new("paint.canvas_previous_point");
    let previous = ui.ctx().data(|d| d.get_temp::<(f32, f32)>(previous_id));
    let start = previous.unwrap_or((px as f32, py as f32));
    for (dab_x, dab_y) in settings.stroke_dabs(start.0, start.1, px as f32, py as f32) {
        PaintModule::canvas_brush_with_settings(state, dab_x, dab_y, settings);
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(previous_id, (px as f32, py as f32)));
    // Sem arrasto em curso, o traço terminou: solta o ponto de referência para
    // o próximo traço começar onde o ponteiro está (nada de ligar dois traços).
    if !response.dragged() && previous.is_some() {
        ui.ctx().data_mut(|d| d.remove::<(f32, f32)>(previous_id));
        state.emit_mesh_changed();
    }
}

/// Um único dab: balde (`Fill`) e conta-gotas (`Eyedropper`).
fn canvas_single_dab(
    state: &mut AppState,
    view: CanvasView,
    brush: BrushType,
    response: &egui::Response,
) {
    let activated = if brush == BrushType::Fill {
        response.clicked()
    } else {
        response.dragged() || response.clicked()
    };
    if !activated {
        return;
    }
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let (px, py) = view.texel(pos);
    if brush == BrushType::Fill {
        fill_canvas_at(state, view, px, py);
    } else {
        if response.drag_started() || response.clicked() {
            state.checkpoint("canvas brush");
        }
        let mut settings = state.brush_settings();
        settings.kind = brush;
        PaintModule::canvas_brush_with_settings(state, px, py, settings);
    }
    if brush == BrushType::Eyedropper {
        state.set_status(state.t("paint.picked"));
    }
    if response.drag_stopped() {
        state.emit_mesh_changed();
    }
}

/// Applies the current Fill scope at the clicked texel on the UV canvas.
fn fill_canvas_at(state: &mut AppState, view: CanvasView, x: u32, y: u32) -> bool {
    if state.project.active().is_none() {
        return false;
    }
    let u = (x as f32 + 0.5) / view.w as f32;
    let v = 1.0 - (y as f32 + 0.5) / view.h as f32;
    let face_hint = UvModule::uv_hit(state, u, v);
    let scope = state.fill_scope;
    let has_target = match scope {
        petunia_core::FillScope::ConnectedPixels | petunia_core::FillScope::Object => true,
        petunia_core::FillScope::Face => face_hint.is_some_and(|face| {
            state
                .project
                .active_mesh()
                .is_some_and(|mesh| mesh.faces.get(face).is_some())
        }),
        petunia_core::FillScope::SelectedFaces => state
            .project
            .active_mesh()
            .is_some_and(|mesh| mesh.faces.iter().any(|face| face.selected)),
        petunia_core::FillScope::UvIsland => face_hint.is_some_and(|face| {
            state.project.active_mesh().is_some_and(|mesh| {
                mesh.uv_islands()
                    .iter()
                    .any(|island| island.faces.contains(&face))
            })
        }),
    };
    if !has_target {
        return false;
    }

    state.checkpoint("scoped canvas fill");
    PaintModule::canvas_fill_scoped(state, face_hint, Some((x, y)), scope);
    true
}

/// Formas: arrastar define o retângulo/segmento, soltar confirma.
fn canvas_shape(
    ui: &mut Ui,
    state: &mut AppState,
    view: CanvasView,
    brush: BrushType,
    response: &egui::Response,
) {
    let shape_id = Id::new("paint.shape_drag");
    if response.drag_started()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let (px, py) = view.texel(pos);
        state.checkpoint("canvas shape");
        ui.ctx()
            .data_mut(|d| d.insert_temp(shape_id, (px, py, px, py)));
    }
    if response.dragged()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let (px, py) = view.texel(pos);
        ui.ctx().data_mut(|d| {
            if let Some(start) = d.get_temp::<(u32, u32, u32, u32)>(shape_id) {
                d.insert_temp(shape_id, (start.0, start.1, px, py));
            }
        });
    }
    if let Some((x0, y0, x1, y1)) = ui
        .ctx()
        .data(|d| d.get_temp::<(u32, u32, u32, u32)>(shape_id))
    {
        let to_screen = |texel| view.screen(texel);
        let painter = ui.painter_at(view.rect);
        let stroke = egui::Stroke::new(1.5_f32, Color32::WHITE.gamma_multiply(0.9));
        if brush == BrushType::Rectangle {
            painter.rect_stroke(
                Rect::from_two_pos(to_screen((x0, y0)), to_screen((x1, y1))),
                0.0,
                stroke,
                StrokeKind::Outside,
            );
        } else {
            painter.line_segment([to_screen((x0, y0)), to_screen((x1, y1))], stroke);
        }
    }
    if response.drag_stopped()
        && let Some((x0, y0, x1, y1)) = ui
            .ctx()
            .data(|d| d.get_temp::<(u32, u32, u32, u32)>(shape_id))
    {
        ui.ctx()
            .data_mut(|d| d.remove::<(u32, u32, u32, u32)>(shape_id));
        let color = state.paint_color;
        PaintModule::commit_shape(
            state,
            petunia_module_paint::ShapeStroke {
                x0,
                y0,
                x1,
                y1,
                brush,
                color: [
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    255,
                ],
                strength: state.paint_strength,
            },
        );
        state.emit_mesh_changed();
    }
}

/// Xadrez de transparência atrás da tela (padrão de pintura).
fn draw_checkerboard(painter: &egui::Painter, rect: Rect, state: &AppState) {
    let cell = 8.0;
    painter.rect_filled(rect, 0.0, tokens::bg_panel(state));
    let light = tokens::BG_SURFACE;
    let cols = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 != 0 {
                continue;
            }
            let min = egui::pos2(
                rect.min.x + col as f32 * cell,
                rect.min.y + row as f32 * cell,
            );
            let max = egui::pos2(
                (min.x + cell).min(rect.max.x),
                (min.y + cell).min(rect.max.y),
            );
            painter.rect_filled(Rect::from_min_max(min, max), 0.0, light);
        }
    }
}

// ---------------------------------------------------------------------------
// Contrato de efeitos
// ---------------------------------------------------------------------------

/// Quantidade de efeitos não-destrutivos do contrato (cap. 42).
pub const EFFECT_COUNT: usize = 7;

/// Chave i18n do efeito na posição `index`.
pub fn effect_name_key(index: usize) -> &'static str {
    match index {
        0 => "paint.effect_grain",
        1 => "paint.effect_levels",
        2 => "paint.effect_brightness_contrast",
        3 => "paint.effect_hue_saturation",
        4 => "paint.effect_pixelate",
        5 => "paint.effect_posterize",
        _ => "paint.effect_invert",
    }
}

/// Efeito padrão (parâmetros neutros) para o índice do seletor.
fn default_effect(index: usize) -> PaintEffect {
    match index {
        0 => PaintEffect::Grain {
            intensity: 0.15,
            seed: 1,
        },
        1 => PaintEffect::Levels {
            in_min: 0.0,
            in_max: 1.0,
            gamma: 1.0,
            out_min: 0.0,
            out_max: 1.0,
        },
        2 => PaintEffect::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
        },
        3 => PaintEffect::HueSaturation {
            hue_shift_deg: 0.0,
            saturation: 0.0,
        },
        4 => PaintEffect::Pixelate { cell_size: 4 },
        5 => PaintEffect::Posterize { levels: 4 },
        _ => PaintEffect::Invert,
    }
}

/// Chave i18n do modo de mistura da camada.
fn blend_key(mode: LayerBlendMode) -> &'static str {
    match mode {
        LayerBlendMode::Normal => "paint.blend_normal",
        LayerBlendMode::Multiply => "paint.blend_multiply",
        LayerBlendMode::Add => "paint.blend_add",
        LayerBlendMode::Screen => "paint.blend_screen",
    }
}

/// Parâmetros do efeito da camada ativa (só ela: menos ruído no painel).
fn draw_effect_parameters(
    ui: &mut Ui,
    state: &mut AppState,
    id: uuid::Uuid,
    effect: PaintEffect,
    actions: &mut Vec<LayerAction>,
) {
    let mut next = effect;
    let mut changed = false;
    match &mut next {
        PaintEffect::Grain { intensity, .. } => {
            changed |= ui
                .add(Slider::new(intensity, 0.0..=1.0).text(state.t("paint.effect_intensity")))
                .changed();
        }
        PaintEffect::Levels {
            in_min,
            in_max,
            gamma,
            out_min,
            out_max,
        } => {
            changed |= ui
                .add(Slider::new(in_min, 0.0..=1.0).text(state.t("paint.effect_in_min")))
                .changed();
            changed |= ui
                .add(Slider::new(in_max, 0.0..=1.0).text(state.t("paint.effect_in_max")))
                .changed();
            changed |= ui
                .add(Slider::new(gamma, 0.1..=4.0).text(state.t("paint.effect_gamma")))
                .changed();
            changed |= ui
                .add(Slider::new(out_min, 0.0..=1.0).text(state.t("paint.effect_out_min")))
                .changed();
            changed |= ui
                .add(Slider::new(out_max, 0.0..=1.0).text(state.t("paint.effect_out_max")))
                .changed();
        }
        PaintEffect::BrightnessContrast {
            brightness,
            contrast,
        } => {
            changed |= ui
                .add(Slider::new(brightness, -1.0..=1.0).text(state.t("paint.effect_brightness")))
                .changed();
            changed |= ui
                .add(Slider::new(contrast, -1.0..=1.0).text(state.t("paint.effect_contrast")))
                .changed();
        }
        PaintEffect::HueSaturation {
            hue_shift_deg,
            saturation,
        } => {
            changed |= ui
                .add(Slider::new(hue_shift_deg, -180.0..=180.0).text(state.t("paint.effect_hue")))
                .changed();
            changed |= ui
                .add(Slider::new(saturation, -1.0..=1.0).text(state.t("paint.effect_saturation")))
                .changed();
        }
        PaintEffect::Pixelate { cell_size } => {
            let mut value = *cell_size as i64;
            if ui
                .add(Slider::new(&mut value, 1..=64).text(state.t("paint.effect_cell_size")))
                .changed()
            {
                *cell_size = value as u32;
                changed = true;
            }
        }
        PaintEffect::Posterize { levels } => {
            let mut value = *levels as i64;
            if ui
                .add(Slider::new(&mut value, 2..=32).text(state.t("paint.effect_levels_count")))
                .changed()
            {
                *levels = value as u8;
                changed = true;
            }
        }
        PaintEffect::Invert => {
            ui.small(state.t("paint.effect_no_parameters"));
        }
    }
    if changed {
        actions.push(LayerAction::Effect(id, next));
    }
}

// ---------------------------------------------------------------------------
// Estado da pilha
// ---------------------------------------------------------------------------

/// Tamanho da tela 2D do ativo (0×0 quando não há tela).
fn canvas_size(state: &AppState) -> (u32, u32) {
    active_canvas(state)
        .map(|canvas| (canvas.w, canvas.h))
        .unwrap_or((0, 0))
}

/// Tela composta do ativo (`None` sem projeto ativo ou sem tela).
fn active_canvas(state: &AppState) -> Option<&petunia_project::Canvas> {
    state
        .project
        .assets
        .get(state.project.active)
        .and_then(|object| object.texture.as_ref())
}

/// Id da camada ativa.
fn active_layer_id(state: &AppState) -> Option<uuid::Uuid> {
    state
        .project
        .assets
        .get(state.project.active)
        .and_then(|object| object.paint_stack.as_ref())
        .and_then(|stack| stack.active())
        .map(|layer| layer.id)
}

/// Snapshot das camadas do ativo, na ordem da pilha.
fn layer_rows(state: &AppState) -> Vec<LayerRow> {
    state
        .project
        .assets
        .get(state.project.active)
        .and_then(|object| object.paint_stack.as_ref())
        .map(|stack| {
            stack
                .layers
                .iter()
                .map(|layer| LayerRow {
                    id: layer.id,
                    name: layer.name.clone(),
                    visible: layer.visible,
                    opacity: layer.opacity,
                    blend: layer.blend,
                    effect: match &layer.kind {
                        LayerKind::Effect(effect) => Some(*effect),
                        _ => None,
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Cria uma camada raster nova do tamanho da tela.
fn add_raster_layer(state: &mut AppState) {
    state.checkpoint("layer add");
    PaintModule::ensure_stack(state);
    let (w, h) = canvas_size(state);
    let (w, h) = if w == 0 { (256, 256) } else { (w, h) };
    let ordinal = layer_rows(state).len() + 1;
    // i18n resolvido antes do borrow mutável.
    let name = state
        .t("paint.layer_default_name")
        .replace("{n}", &ordinal.to_string());
    let active = state.project.active;
    if let Some(stack) = state
        .project
        .assets
        .get_mut(active)
        .and_then(|object| object.paint_stack.as_mut())
    {
        stack.add_layer(PaintLayer::new(name, w, h, [0, 0, 0, 0]));
    }
    PaintModule::composite_active(state);
}

/// Aplica as ações coletadas com **um** checkpoint por gesto.
fn apply_layer_actions(state: &mut AppState, actions: Vec<LayerAction>) {
    if actions.is_empty() {
        return;
    }
    let needs_checkpoint = actions
        .iter()
        .any(|action| !matches!(action, LayerAction::Opacity(_, _)));
    if needs_checkpoint {
        state.checkpoint("layer edit");
    }
    // i18n resolvido fora do empréstimo do projeto.
    let copy_name = state.t("paint.layer_copy_name");
    let effect_names: Vec<String> = (0..EFFECT_COUNT)
        .map(|index| state.t(effect_name_key(index)))
        .collect();
    let active_index = state.project.active;
    for action in actions {
        let Some(object) = state.project.assets.get_mut(active_index) else {
            return;
        };
        let Some(stack) = object.paint_stack.as_mut() else {
            return;
        };
        match action {
            LayerAction::Activate(id) => {
                stack.set_active(id);
            }
            LayerAction::ToggleVisible(id) => {
                if let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) {
                    layer.visible = !layer.visible;
                }
            }
            LayerAction::Opacity(id, opacity) => {
                if let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) {
                    layer.opacity = opacity.clamp(0.0, 1.0);
                }
            }
            LayerAction::Blend(id, mode) => {
                if let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) {
                    layer.blend = mode;
                }
            }
            LayerAction::Rename(id, name) => {
                if let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) {
                    layer.name = name;
                }
            }
            LayerAction::Effect(id, effect) => {
                if let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id)
                    && let LayerKind::Effect(current) = &mut layer.kind
                {
                    *current = effect;
                }
            }
            LayerAction::Reorder(order) => {
                let mut reordered = Vec::with_capacity(stack.layers.len());
                for id in order {
                    if let Some(index) = stack.layers.iter().position(|layer| layer.id == id) {
                        reordered.push(stack.layers[index].clone());
                    }
                }
                if reordered.len() == stack.layers.len() {
                    stack.layers = reordered;
                }
            }
            LayerAction::Delete(id) => {
                stack.remove_layer(id);
            }
            LayerAction::Duplicate(id) => {
                if let Some(index) = stack.layers.iter().position(|layer| layer.id == id) {
                    let mut copy = stack.layers[index].clone();
                    // Identidade nova: id repetido faria duas linhas arrastarem juntas.
                    copy.id = uuid::Uuid::new_v4();
                    copy.name = copy_name.replace("{name}", &copy.name);
                    stack.add_layer(copy);
                }
            }
            LayerAction::AddEffect(index) => {
                let name = effect_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| effect_names[0].clone());
                stack.add_layer(PaintLayer::new_effect(name, default_effect(index)));
            }
        }
    }
    PaintModule::composite_active(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_project::Canvas;

    #[test]
    fn fill_scope_selector_covers_all_five_domain_scopes() {
        assert_eq!(
            FILL_SCOPES,
            [
                petunia_core::FillScope::ConnectedPixels,
                petunia_core::FillScope::Face,
                petunia_core::FillScope::SelectedFaces,
                petunia_core::FillScope::UvIsland,
                petunia_core::FillScope::Object,
            ]
        );
        let labels: Vec<_> = FILL_SCOPES.into_iter().map(fill_scope_label).collect();
        assert_eq!(
            labels,
            [
                text_id::PAINT_FILL_CONNECTED_PIXELS,
                text_id::PAINT_FILL_FACE,
                text_id::PAINT_FILL_SELECTED_FACES,
                text_id::PAINT_FILL_UV_ISLAND,
                text_id::PAINT_FILL_OBJECT,
            ]
        );
    }

    #[test]
    fn fill_scope_controls_render_for_the_fill_brush() {
        let context = egui::Context::default();
        let mut state = AppState::new("en");
        state.paint_brush_kind = 3;

        context
            .run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw_brush_block(ui, &mut state);
                });
            })
            .textures_delta
            .clear();

        assert_eq!(state.fill_scope, petunia_core::FillScope::ConnectedPixels);
    }

    #[test]
    fn fill_brush_uses_connected_pixel_seed() {
        let mut state = AppState::new("en");
        PaintModule::ensure_stack(&mut state);
        PaintModule::resize_canvas(&mut state, 4, 4);
        state.paint_color = [1.0, 0.0, 0.0];
        state.fill_scope = petunia_core::FillScope::ConnectedPixels;

        let mut canvas = Canvas::new(4, 4, [0, 0, 0, 255]);
        for y in 0..4 {
            canvas.set(2, y, [255, 255, 255, 255]);
        }
        if let Some(layer) = state
            .project
            .active_mut()
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.active_mut())
            && let Some(layer_canvas) = layer.canvas_mut()
        {
            *layer_canvas = canvas;
        }
        state.project.active_mut().unwrap().texture = Some(Canvas::new(4, 4, [0, 0, 0, 255]));

        fill_canvas_at(
            &mut state,
            CanvasView {
                rect: Rect::NOTHING,
                scale: 1.0,
                w: 4,
                h: 4,
            },
            0,
            0,
        );

        let result = state.project.active().unwrap().texture.as_ref().unwrap();
        assert_eq!(result.get(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(result.get(3, 0), Some([0, 0, 0, 255]));
    }

    #[test]
    fn scoped_fill_without_a_target_does_not_create_an_undo_step() {
        let mut state = AppState::new("en");
        state.fill_scope = petunia_core::FillScope::SelectedFaces;
        let undo_before = state.project.undo.undo_label().map(str::to_owned);

        assert!(!fill_canvas_at(
            &mut state,
            CanvasView {
                rect: Rect::NOTHING,
                scale: 1.0,
                w: 256,
                h: 256,
            },
            0,
            0,
        ));
        assert_eq!(
            state.project.undo.undo_label().map(str::to_owned),
            undo_before
        );
    }

    /// A paleta cobre os oito índices do contrato, uma vez cada.
    ///
    /// `paint_brush_kind` é estado persistido: dois botões com o mesmo índice
    /// fariam uma ferramenta nunca ser alcançável, e um índice órfão deixaria
    /// uma ferramenta sem botão.
    #[test]
    fn the_tool_palette_covers_every_brush_kind_once() {
        let tools = canonical_paint_tools();
        let mut kinds: Vec<usize> = tools.iter().map(|tool| tool.kind).collect();
        kinds.sort_unstable();
        assert_eq!(kinds, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        // Todo grupo tem pelo menos uma ferramenta: um grupo vazio desenharia um
        // separador solto na coluna.
        for section in PaintToolSection::ALL {
            assert!(
                tools.iter().any(|tool| tool.section == section),
                "grupo {} vazio",
                section.key()
            );
        }
    }

    /// O índice persistido e o tipo do motor não podem divergir.
    #[test]
    fn every_brush_kind_maps_to_its_engine_type() {
        for tool in canonical_paint_tools() {
            let expected = match tool.kind {
                0 => BrushType::Pixel,
                1 => BrushType::Soft,
                2 => BrushType::Eraser,
                3 => BrushType::Fill,
                4 => BrushType::Eyedropper,
                5 => BrushType::Line,
                6 => BrushType::Rectangle,
                _ => BrushType::Airbrush,
            };
            assert_eq!(brush_type_of(tool.kind), expected, "kind {}", tool.kind);
        }
        // Índice fora da faixa nunca vira pânico (projeto legado/corrompido).
        assert_eq!(brush_type_of(999), BrushType::Pixel);
    }

    /// Chave i18n e índice andam juntos: a linha da lista mostra o nome do modo
    /// que o seletor mostra.
    #[test]
    fn blend_names_and_keys_share_one_order() {
        assert_eq!(blend_key(BLEND_MODES[0]), "paint.blend_normal");
        assert_eq!(blend_key(BLEND_MODES[1]), "paint.blend_multiply");
        assert_eq!(blend_key(BLEND_MODES[2]), "paint.blend_add");
        assert_eq!(blend_key(BLEND_MODES[3]), "paint.blend_screen");
        for (index, mode) in BLEND_MODES.iter().enumerate() {
            assert_eq!(blend_index(*mode), index);
        }
    }

    #[test]
    fn every_effect_of_the_contract_has_a_key_and_neutral_defaults() {
        for index in 0..EFFECT_COUNT {
            let key = effect_name_key(index);
            assert!(key.starts_with("paint.effect_"), "{key}");
        }
        // Fora da faixa o seletor não estoura: cai no último efeito do contrato.
        assert!(matches!(default_effect(99), PaintEffect::Invert));
        assert_eq!(effect_name_key(99), "paint.effect_invert");
        // Os presets nascem neutros: adicionar um efeito não muda a imagem.
        assert!(matches!(
            default_effect(2),
            PaintEffect::BrightnessContrast {
                brightness,
                contrast
            } if brightness == 0.0 && contrast == 0.0
        ));
        assert!(matches!(
            default_effect(3),
            PaintEffect::HueSaturation {
                hue_shift_deg,
                saturation
            } if hue_shift_deg == 0.0 && saturation == 0.0
        ));
    }

    #[test]
    fn the_row_summary_only_shows_what_left_the_default() {
        let labels = LayerLabels {
            show: "show".into(),
            hide: "hide".into(),
            blend_names: [
                "Normal".into(),
                "Multiply".into(),
                "Add".into(),
                "Screen".into(),
            ],
        };
        let plain = LayerRow {
            id: uuid::Uuid::nil(),
            name: "Layer 1".into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            effect: None,
        };
        assert!(layer_summary(&plain, &labels).is_empty());
        let adjusted = LayerRow {
            opacity: 0.5,
            blend: LayerBlendMode::Multiply,
            ..plain.clone()
        };
        assert_eq!(layer_summary(&adjusted, &labels), "Multiply · 50%");
        assert_eq!(layer_title(&plain), "Layer 1");
        assert_eq!(
            layer_title(&LayerRow {
                effect: Some(PaintEffect::Invert),
                ..plain
            }),
            "fx · Layer 1"
        );
    }
}
