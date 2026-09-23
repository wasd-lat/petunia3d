//! Blocos reutilizáveis do inspector contextual: densidade, campos numéricos
//! com undo correto, seções leves e abas textuais.
//!
//! Regras do sistema: tokens de densidade (sem segundo settings), undo por
//! sessão de edição (1 nível por gesto), foco visível + `widget_info` em tudo
//! custom, i18n fora daqui (chamador traduz).
//!
//! **Layout responsivo não se calcula aqui.** O componente descreve o arranjo
//! com o adapter ([`crate::adapters::taffy_layout`]) e recebe a largura já
//! resolvida de cada coluna. Foi assim que os breakpoints em pixels
//! (`avail >= 300`, `avail >= 190`) e o cálculo manual de largura de aba
//! saíram deste arquivo (Wave 3 — §45 da diretriz).

use egui::{Color32, Rect, Sense, StrokeKind, Ui, WidgetInfo, WidgetType, vec2};
use petunia_core::{AppState, UiDensity};

use crate::adapters::taffy_layout::{PetuniaColumnSpec, columns};
use crate::foundation::spacing;
use crate::foundation::typography::{self, TextRole};
use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::inspector_context::InspectorTab;
use crate::tokens;
use crate::widgets::{ChevronDir, paint_chevron};

// ---------------------------------------------------------------- densidade
//
// As métricas de densidade são **foundation** (`crate::foundation::density`),
// não do inspector: o shell inteiro depende delas. Reexportadas aqui para não
// quebrar call sites históricos — a definição vive em um só lugar.
pub use crate::foundation::density::{hit_size, input_h, row_h, section_gap, spacing_y};

// ------------------------------------------------------- sessão de edição

/// Sessão de edição com checkpoint único (undo correto por gesto).
///
/// `DragValue` muta antes de avisar: checkpoint no `drag_started`
/// (pré-mutação, sem desperdício) + fallback na primeira mudança por teclado.
/// `finish` fecha a sessão (soltar botão / perder foco). UI-state vive em
/// temp-data do egui; domínio nunca é duplicado aqui.
pub struct EditSession {
    key: &'static str,
}

impl EditSession {
    pub fn new(key: &'static str) -> Self {
        Self { key }
    }

    fn flag_id(&self) -> egui::Id {
        egui::Id::new(("petunia_edit_session", self.key))
    }

    fn checkpoint_once(&self, ui: &mut Ui, state: &mut AppState, undo_label: &str) {
        let active = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(self.flag_id()).unwrap_or(false));
        if !active {
            state.checkpoint(undo_label);
            ui.ctx().data_mut(|d| d.insert_temp(self.flag_id(), true));
        }
    }

    /// Observa a resposta do campo; retorna se houve mudança.
    pub fn poll(
        &self,
        ui: &mut Ui,
        state: &mut AppState,
        resp: &egui::Response,
        undo_label: &str,
        changed: bool,
    ) -> bool {
        if resp.drag_started() || changed {
            self.checkpoint_once(ui, state, undo_label);
        }
        changed
    }

    pub fn finish(&self, ui: &mut Ui, finished: bool) {
        if finished {
            ui.ctx().data_mut(|d| d.insert_temp(self.flag_id(), false));
        }
    }
}

// ------------------------------------------------------------ campo numérico

/// Opções de formatação de campo numérico.
#[derive(Debug, Clone, Copy)]
pub struct NumericOpts {
    pub speed: f32,
    pub decimals: usize,
    pub suffix: &'static str,
    pub min: f32,
    pub max: f32,
}

impl NumericOpts {
    pub fn plain(speed: f32, decimals: usize) -> Self {
        Self {
            speed,
            decimals,
            suffix: "",
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }
}

/// Evento de campo: mudou e/ou sessão terminou (soltou/tirou foco).
#[derive(Debug, Clone, Copy, Default)]
pub struct FieldEvent {
    pub changed: bool,
    pub finished: bool,
}

/// Campo numérico com `DragValue` (arrastar + teclado + setas), formatação
/// fixa (sem ruído decimal) e undo por sessão. Retorna o evento; quem chama
/// aplica o efeito no domínio.
pub struct NumericField<'a> {
    pub value: &'a mut f32,
    pub opts: NumericOpts,
    pub session_key: &'static str,
    pub undo_label: &'a str,
    pub width: f32,
    pub height: f32,
}

impl<'a> NumericField<'a> {
    /// Menor largura em que o valor ainda é editável (piso de segurança).
    /// Público porque quem monta uma linha responsiva precisa do mesmo mínimo
    /// para decidir quantas cabem — e a decisão não pode divergir daqui.
    pub const MIN_WIDTH: f32 = 24.0;

    pub fn show(self, ui: &mut Ui, state: &mut AppState) -> FieldEvent {
        let session = EditSession::new(self.session_key);
        let decimals = self.opts.decimals;
        let suffix = self.opts.suffix;
        let mut staging = self.value.clamp(self.opts.min, self.opts.max);
        // Largura inteira: `add_sized` com fração arredonda PARA CIMA e a soma
        // das linhas ultrapassa o painel (o egui então alarga o dock 1px/frame).
        let width = self.width.floor().max(Self::MIN_WIDTH);
        let resp = ui.add_sized(
            vec2(width, self.height),
            egui::DragValue::new(&mut staging)
                .speed(self.opts.speed)
                .range(self.opts.min..=self.opts.max)
                .custom_formatter(move |n, _| format!("{n:.decimals$}{suffix}")),
        );
        let changed = session.poll(ui, state, &resp, self.undo_label, resp.changed());
        if changed {
            *self.value = staging;
        }
        let finished = resp.drag_stopped() || resp.lost_focus();
        session.finish(ui, finished);
        FieldEvent { changed, finished }
    }
}

// --------------------------------------------------------------- vetor xyz

/// Evento por eixo do campo vetorial.
#[derive(Debug, Clone, Copy, Default)]
pub struct AxisEvent {
    pub changed: [bool; 3],
    pub finished: bool,
}

/// Largura reservada ao rótulo de eixo (`X`/`Y`/`Z`).
///
/// Medida em Petunia: é o rótulo de um caractere no papel de campo, não uma
/// estimativa de texto arbitrária.
pub const AXIS_LABEL_W: f32 = 14.0;

/// Vão entre rótulo, campo e entre os próprios eixos ([`spacing::RELATED`]).
pub const AXIS_GAP: f32 = spacing::RELATED;

/// Menor **unidade de eixo** utilizável: rótulo + vão + campo que ainda permite
/// ler e arrastar o número.
///
/// É o único limite que decide o arranjo do [`Vector3Field`]. Substitui os
/// breakpoints `avail >= 300` / `avail >= 190`, que eram números escolhidos a
/// dedo; este é derivado do conteúdo.
pub const AXIS_UNIT_MIN_W: f32 = AXIS_LABEL_W + AXIS_GAP + 60.0;

/// Campo vetorial responsivo.
///
/// O arranjo vem do adapter de layout ([`columns`]): os três eixos ficam em uma
/// linha quando três unidades de eixo cabem; senão um eixo por linha, com campo
/// de largura cheia. Nenhum breakpoint em pixels mora aqui — o número de colunas
/// sai de [`AXIS_UNIT_MIN_W`] e a largura de cada campo é **recebida** já
/// resolvida. Mesma sessão de undo pros 3 eixos.
pub struct Vector3Field<'a> {
    pub axis_names: [&'a str; 3],
    pub axis_colors: [Color32; 3],
    pub values: &'a mut [f32; 3],
    pub opts: NumericOpts,
    pub session_key: &'static str,
    pub undo_label: &'a str,
}

impl<'a> Vector3Field<'a> {
    pub fn show(self, ui: &mut Ui, state: &mut AppState) -> AxisEvent {
        let density = state.ui.density;
        let height = input_h(density);
        let axis_names = self.axis_names;
        let axis_colors = self.axis_colors;
        let opts = self.opts;
        let session_key = self.session_key;
        let undo_label = self.undo_label;
        let values = self.values;
        let id = ui.id().with("petunia_vector3").with(session_key);
        let mut out = AxisEvent::default();
        columns(
            ui,
            id,
            PetuniaColumnSpec::responsive(values.len(), AXIS_UNIT_MIN_W, AXIS_GAP),
            |index, column_w, ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(AXIS_GAP, 0.0);
                    ui.label(
                        egui::RichText::new(axis_names[index])
                            .strong()
                            .color(axis_colors[index]),
                    );
                    let field_w = (column_w - AXIS_LABEL_W - AXIS_GAP).max(NumericField::MIN_WIDTH);
                    let ev = NumericField {
                        value: &mut values[index],
                        opts,
                        session_key,
                        undo_label,
                        width: field_w,
                        height,
                    }
                    .show(ui, state);
                    out.changed[index] = ev.changed;
                    out.finished |= ev.finished;
                });
            },
            |_ui| (),
        );
        out
    }
}

// ------------------------------------------------------------------- seção

/// Seção leve do inspector (tipografia + separador + disclosure, sem cards).
///
/// Cabeçalho de altura `row_h` clicável por inteiro (teclado focável, anel de
/// foco, `widget_info`); `summary` aparece à direita quando colapsada
/// (ex. "8 vertices · 12 triangles"). Estado em `CollapsingState` persistente;
/// `force_open` (busca do inspector) abre sem gravar.
/// Opções da seção (título traduzido fora, resumo colapsado, abertura).
pub struct SectionOpts<'a> {
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub default_open: bool,
    pub force_open: bool,
}

/// Inset do chevron e do título dentro da linha de seção (interno do
/// componente — §36 permite micro-layout aqui, desde que nomeado).
const SECTION_CHEVRON_INSET: f32 = 11.0;
const SECTION_CHEVRON_SIZE: f32 = 12.0;
const SECTION_TITLE_INSET: f32 = 24.0;
const SECTION_SUMMARY_INSET: f32 = 6.0;
const SECTION_BODY_GAP: f32 = spacing::TIGHT;

pub fn section(
    ui: &mut Ui,
    density: UiDensity,
    id_salt: &str,
    opts: SectionOpts<'_>,
    body: impl FnOnce(&mut Ui),
) {
    let h = row_h(density);
    let row_w = ui.available_width().max(0.0);
    let id = ui.make_persistent_id(id_salt);
    let mut collapsed = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        opts.default_open,
    );
    let is_open = opts.force_open || collapsed.is_open();

    let (rect, resp) = ui.allocate_exact_size(vec2(row_w, h), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, is_open, opts.title));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(rect, tokens::RADIUS_SMALL, tokens::BG_SURFACE_HOVER);
        }
        if resp.has_focus() {
            painter.rect_stroke(
                rect,
                tokens::RADIUS_SMALL,
                tokens::stroke_focus(ui.ctx()),
                StrokeKind::Inside,
            );
        }
        paint_chevron(
            painter,
            Rect::from_center_size(
                egui::pos2(rect.min.x + SECTION_CHEVRON_INSET, rect.center().y),
                vec2(SECTION_CHEVRON_SIZE, SECTION_CHEVRON_SIZE),
            ),
            tokens::TEXT_SECONDARY,
            if is_open {
                ChevronDir::Down
            } else {
                ChevronDir::Right
            },
        );
        painter.text(
            egui::pos2(rect.min.x + SECTION_TITLE_INSET, rect.center().y),
            egui::Align2::LEFT_CENTER,
            opts.title.to_uppercase(),
            typography::font(TextRole::SectionTitle),
            tokens::TEXT_PRIMARY,
        );
        if !is_open && let Some(summary) = opts.summary {
            painter.text(
                egui::pos2(rect.max.x - SECTION_SUMMARY_INSET, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                summary,
                typography::font(TextRole::Caption),
                tokens::TEXT_MUTED,
            );
        }
    }
    if resp.on_hover_text(opts.title).clicked() && !opts.force_open {
        collapsed.toggle(ui);
    }
    if is_open {
        ui.add_space(SECTION_BODY_GAP);
        body(ui);
        ui.add_space(SECTION_BODY_GAP);
    } else {
        collapsed.store(ui.ctx());
    }
}

// -------------------------------------------------------------------- abas

/// Vão entre abas contextuais ([`spacing::RELATED`]).
const TAB_GAP: f32 = spacing::RELATED;

/// Abas textuais contextuais (texto > memorização de ícones).
///
/// Divisão igual da largura disponível pelo adapter ([`columns`] com mínimo
/// zero: abas nunca quebram linha). Botões custom de altura `row_h`, foco
/// visível e semântica de seleção. Retorna a aba clicada (teclado Enter/Espaço
/// incluso).
pub fn context_tabs(
    ui: &mut Ui,
    density: UiDensity,
    tabs: &[(InspectorTab, String)],
    active: InspectorTab,
) -> Option<InspectorTab> {
    if tabs.is_empty() {
        return None;
    }
    let h = row_h(density);
    let id = ui.id().with("petunia_context_tabs");
    let mut picked = None;
    // `min_item = 0.0`: abas nunca quebram linha — só a divisão igual interessa.
    columns(
        ui,
        id,
        PetuniaColumnSpec::fixed(tabs.len(), TAB_GAP),
        |index, tab_w, ui| {
            let (tab, label) = &tabs[index];
            let selected = *tab == active;
            let (rect, resp) = ui.allocate_exact_size(vec2(tab_w, h), Sense::click());
            resp.widget_info(|| {
                WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label.clone())
            });
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let (fill, fg) = if selected {
                    (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
                } else if resp.hovered() {
                    (tokens::BG_SURFACE_HOVER, tokens::TEXT_PRIMARY)
                } else {
                    (tokens::BG_SURFACE, tokens::TEXT_SECONDARY)
                };
                painter.rect_filled(rect, tokens::RADIUS_CONTROL, fill);
                if resp.has_focus() {
                    painter.rect_stroke(
                        rect,
                        tokens::RADIUS_CONTROL,
                        tokens::stroke_focus(ui.ctx()),
                        StrokeKind::Inside,
                    );
                }
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    typography::font(TextRole::Field),
                    fg,
                );
            }
            if resp.on_hover_text(label).clicked() {
                picked = Some(*tab);
            }
        },
        |_ui| (),
    );
    picked
}

/// Linha de cabeçalho de bloco com ícone + título (cabeçalhos de contexto).
pub fn block_header(ui: &mut Ui, icon: PetuniaIcon, title: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(spacing::CONTROL, 0.0);
        let (icon_rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
        IconRegistry::paint(
            ui.ctx(),
            ui.painter(),
            &icon,
            icon_rect,
            tokens::ACCENT_BLUE,
        );
        ui.label(typography::rich(title, TextRole::SectionTitle).strong());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_tokens_grow_monotonically() {
        use UiDensity::{Comfortable, Compact, Spacious};
        for f in [row_h, hit_size, spacing_y, input_h, section_gap] {
            assert!(f(Compact) < f(Comfortable));
            assert!(f(Comfortable) < f(Spacious));
        }
        assert!(row_h(UiDensity::Compact) >= 28.0);
    }

    #[test]
    fn edit_session_checkpoints_once_per_gesture() {
        let ctx = egui::Context::default();
        let mut state = AppState::new("en");
        let base_depth = state.project.undo.depth().0;
        let session = EditSession::new("test_gesture");
        ctx.run_ui(egui::RawInput::default(), |ui| {
            // Foco/teclado sem arrasto: primeira mudança carimba, resto não.
            let resp = ui.allocate_response(vec2(10.0, 10.0), Sense::click());
            assert!(session.poll(ui, &mut state, &resp, "test edit", true));
            assert!(session.poll(ui, &mut state, &resp, "test edit", true));
            session.finish(ui, true);
            // Nova sessão carimba de novo.
            assert!(session.poll(ui, &mut state, &resp, "test edit", true));
        })
        .textures_delta
        .clear();
        assert_eq!(state.project.undo.depth().0, base_depth + 2);
    }

    #[test]
    fn context_tabs_render_and_pick() {
        let ctx = egui::Context::default();
        let mut picked = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                picked = context_tabs(
                    ui,
                    UiDensity::Comfortable,
                    &[
                        (InspectorTab::Object, "Object".to_string()),
                        (InspectorTab::Modify, "Modify".to_string()),
                    ],
                    InspectorTab::Object,
                );
            });
        })
        .textures_delta
        .clear();
        // Sem clique, nada escolhido.
        assert_eq!(picked, None);
    }
}
