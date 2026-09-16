//! # `PetuniaForm` — contrato de formulário (§48)
//!
//! Wave 6: as linhas "rótulo à esquerda · controle à direita" de Settings
//! deixam de escolher o arranjo com um breakpoint escrito à mão. O produto
//! declara duas coisas — a largura do rótulo e a menor largura em que o
//! controle ainda é utilizável — e o adapter mede, decide e entrega a largura
//! **definitiva** do controle.
//!
//! ```text
//! settings_modal.rs  →  PetuniaForm::field(..)
//!                       PetuniaForm::section(..)
//!                       PetuniaForm::toggle(..)
//! ```
//!
//! ## Por que um contrato e não uma crate nova
//!
//! O contrato é do Petunia e o **arranjo** é do `egui_taffy` (via
//! [`crate::adapters::taffy_layout::fixed_label_row`]).
//!
//! Wave 7 (§51): a **validação** entrou como segunda camada do mesmo contrato,
//! com `egui_form` como backend. Ela não reescreveu Settings: o mesmo
//! [`PetuniaForm`] que arranja o campo passou a também declarar o erro dele.
//!
//! ```text
//! let report = PetuniaValidationReport::new()
//!     .with_error("atalho.salvar", "compartilha o atalho com 'Salvar como'");
//! let mut session = PetuniaFormSession::new(report);
//! SETTINGS_FORM.validated_control(ui, &mut session, "atalho.salvar", |ui| badge(ui));
//! SETTINGS_FORM.error_summary(ui, &session);
//! ```
//!
//! Quem decide o que é válido continua sendo o domínio Petunia
//! (`Keybinds::detect_conflicts`), não a crate: o adapter só traduz o resultado
//! para erro por campo, estado de revelação e foco no primeiro inválido.
//!
//! ## Sem breakpoints
//!
//! [`plan_row`] é puro: recebe a largura disponível e devolve o arranjo
//! (`Inline` ou `Stacked`) com as larguras resolvidas. É testável sem `Ui`, e é
//! por onde o painel estreito (2 painéis lado a lado, escala de UI maior, idioma
//! com rótulo longo) fica coberto.

use std::borrow::Cow;

use egui::{Response, Ui};
use egui_form::{EguiValidationReport, Form, FormField, IntoFieldPath};

use crate::adapters::taffy_layout;
use crate::foundation::typography::{TextRole, rich};
use crate::tokens;

/// Especificação de um formulário Petunia.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaFormSpec {
    /// Largura do rótulo no arranjo lado a lado.
    pub label_width: f32,
    /// Menor largura em que o controle ainda é utilizável: abaixo disto o
    /// campo empilha (rótulo em cima) em vez de espremer o controle.
    pub control_min: f32,
    /// Vão entre rótulo e controle.
    pub gap: f32,
}

impl PetuniaFormSpec {
    /// Especificação com o vão padrão do formulário.
    pub const fn new(label_width: f32, control_min: f32) -> Self {
        Self {
            label_width,
            control_min,
            gap: 12.0,
        }
    }

    /// Define o vão entre rótulo e controle.
    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// Arranjo resolvido de um campo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetuniaFormPlacement {
    /// Rótulo à esquerda, controle à direita (cabe uma linha).
    Inline,
    /// Rótulo em cima, controle embaixo (a linha não cabe).
    Stacked,
}

/// Arranjo de um campo, já com as larguras resolvidas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaFormRow {
    pub placement: PetuniaFormPlacement,
    /// Largura reservada ao rótulo.
    pub label_width: f32,
    /// Largura **definitiva** entregue ao controle.
    pub control_width: f32,
}

/// Decide o arranjo do campo a partir da largura medida.
///
/// Puro e testável: nenhum widget, nenhum `Ui`. A regra é a documentada —
/// `Inline` exatamente quando o controle ainda cabe com `control_min`; senão
/// `Stacked`, onde o controle recebe a faixa inteira.
pub fn plan_row(available: f32, spec: &PetuniaFormSpec) -> PetuniaFormRow {
    let available = available.max(0.0);
    let inline_control = (available - spec.label_width - spec.gap).max(0.0);
    if inline_control >= spec.control_min {
        PetuniaFormRow {
            placement: PetuniaFormPlacement::Inline,
            label_width: spec.label_width,
            control_width: inline_control,
        }
    } else {
        PetuniaFormRow {
            placement: PetuniaFormPlacement::Stacked,
            label_width: available,
            control_width: available.max(0.0),
        }
    }
}

/// Formulário Petunia: rótulos, campos e seções de uma tela de configuração.
///
/// O produto guarda **uma** instância estática por tela
/// (`static SETTINGS_FORM: PetuniaForm = PetuniaForm::new(..)`) e o campo é
/// declarado com rótulo + closure de controle. O controle recebe a largura
/// pronta e não mede o container.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaForm {
    pub spec: PetuniaFormSpec,
}

impl PetuniaForm {
    /// Formulário com a largura de rótulo e o mínimo de controle declarados.
    pub const fn new(label_width: f32, control_min: f32) -> Self {
        Self {
            spec: PetuniaFormSpec::new(label_width, control_min),
        }
    }

    /// Define o vão entre rótulo e controle.
    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.spec = self.spec.with_gap(gap);
        self
    }

    /// Arranjo do próximo campo, para a largura informada.
    pub fn plan(&self, available: f32) -> PetuniaFormRow {
        plan_row(available, &self.spec)
    }

    /// Cabeçalho de seção do formulário (título e, se houver, descrição).
    ///
    /// Existe para que a hierarquia tipográfica e o respiro da seção sejam do
    /// contrato: antes cada aba repetia `strong().size(13.0)` + `add_space(8.0)`.
    pub fn section(&self, ui: &mut Ui, title: &str, description: Option<&str>) {
        ui.label(
            rich(title, TextRole::Heading)
                .strong()
                .color(tokens::TEXT_PRIMARY),
        );
        if let Some(text) = description {
            ui.label(rich(text, TextRole::Label).color(tokens::TEXT_SECONDARY));
        }
        ui.add_space(8.0);
    }

    /// Campo rotulado: o controle recebe a largura definitiva.
    ///
    /// `Inline` usa a linha assimétrica do adapter (rótulo com largura
    /// declarada, controle com o resto); `Stacked` desenha o rótulo e entrega a
    /// faixa inteira ao controle. Nos dois casos a largura passada ao closure é
    /// a que o `Ui` do item realmente tem — o campo pode repassá-la ao widget
    /// sem medir o container.
    pub fn field<R>(
        &self,
        ui: &mut Ui,
        id: impl Into<egui::Id>,
        label: &str,
        control: impl FnOnce(&mut Ui, f32) -> R,
    ) -> R {
        let plan = self.plan(ui.available_width());
        match plan.placement {
            PetuniaFormPlacement::Inline => taffy_layout::fixed_label_row(
                ui,
                id,
                plan.label_width,
                self.spec.gap,
                |ui| field_label(ui, label),
                |width, ui| control(ui, width),
            ),
            PetuniaFormPlacement::Stacked => {
                field_label(ui, label);
                control(ui, plan.control_width)
            }
        }
    }

    /// Campo rotulado **validado**: mesma geometria de [`PetuniaForm::field`],
    /// mas o controle fica dentro do campo do formulário, então erro, borda e
    /// mensagem por campo vêm do contrato.
    pub fn validated_field(
        &self,
        ui: &mut Ui,
        session: &mut PetuniaFormSession,
        field: &str,
        label: &str,
        control: impl FnOnce(&mut Ui, f32) -> Response,
    ) -> Response {
        let plan = self.plan(ui.available_width());
        // O id da linha sai do **nome do campo**: um nome, um id, sem chance de
        // o erro apontar para um campo e a geometria para outro.
        let id = egui::Id::new(("petunia-form-row", field));
        match plan.placement {
            PetuniaFormPlacement::Inline => taffy_layout::fixed_label_row(
                ui,
                id,
                plan.label_width,
                self.spec.gap,
                |ui| field_label(ui, label),
                |width, ui| {
                    Self::validated(ui, session, field, |ui| {
                        // A largura do contrato vai ao controle pelo mesmo canal
                        // do campo comum: o `Ui` já tem a largura resolvida.
                        let _ = width;
                        control(ui, width)
                    })
                },
            ),
            PetuniaFormPlacement::Stacked => {
                field_label(ui, label);
                Self::validated(ui, session, field, |ui| control(ui, plan.control_width))
            }
        }
    }

    /// Controle validado **sem** rótulo de arranjo: para campos que vivem em
    /// outras estruturas (célula de grade, linha de tabela) e só precisam do
    /// erro e da mensagem do contrato.
    pub fn validated_control(
        &self,
        ui: &mut Ui,
        session: &mut PetuniaFormSession,
        field: &str,
        control: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        Self::validated(ui, session, field, control)
    }

    /// Bloco de mensagens inline do formulário (uma linha por campo inválido).
    ///
    /// Devolve `true` quando há erro. Substitui os banners escritos à mão: a cor
    /// e a tipografia vêm do token do design system, e o texto é do domínio.
    pub fn error_summary(&self, ui: &mut Ui, session: &PetuniaFormSession) -> bool {
        self.error_summary_titled(ui, session, None)
    }

    /// Igual a [`PetuniaForm::error_summary`], com um título localizado.
    pub fn error_summary_titled(
        &self,
        ui: &mut Ui,
        session: &PetuniaFormSession,
        title: Option<&str>,
    ) -> bool {
        if session.is_valid() {
            return false;
        }
        egui::Frame::new()
            .fill(tokens::ACCENT_ERROR_SOFT)
            .stroke(egui::Stroke::new(1.0_f32, tokens::ACCENT_ERROR))
            .corner_radius(tokens::RADIUS_CONTAINER)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                if let Some(title) = title {
                    ui.label(
                        rich(title, TextRole::Label)
                            .strong()
                            .color(tokens::ACCENT_ERROR),
                    );
                }
                for (field, message) in session.errors() {
                    ui.horizontal(|ui| {
                        ui.label(
                            rich("!", TextRole::Label)
                                .strong()
                                .color(tokens::ACCENT_ERROR),
                        );
                        ui.label(
                            rich(format!("{field}: {message}"), TextRole::Label)
                                .color(tokens::TEXT_PRIMARY),
                        );
                    });
                }
            });
        true
    }

    /// Marcas de erro do formulário (usado pelo resumo e pelos testes).
    fn validated(
        ui: &mut Ui,
        session: &mut PetuniaFormSession,
        field: &str,
        control: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        FormField::new(&mut session.form, field).ui(ui, control)
    }

    /// Alternador rotulado (caixa + texto).
    ///
    /// O rótulo **é** o texto do controle, então não há geometria a decidir: o
    /// contrato existe para a tipografia e a semântica de acessibilidade serem
    /// as mesmas em todas as abas. Devolve `true` quando o valor mudou.
    pub fn toggle(&self, ui: &mut Ui, label: &str, value: &mut bool) -> bool {
        ui.checkbox(value, rich(label, TextRole::Label)).changed()
    }
}

/// Rótulo de campo com o papel tipográfico do contrato.
fn field_label(ui: &mut Ui, label: &str) {
    ui.label(rich(label, TextRole::Label).color(tokens::TEXT_SECONDARY));
}

/// Id de campo do formulário Petunia (nome estável e único na tela).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PetuniaFieldId<'a>(pub &'a str);

impl<'a> IntoFieldPath<PetuniaFieldId<'a>> for PetuniaFieldId<'a> {
    fn into_field_path(self) -> PetuniaFieldId<'a> {
        self
    }
}

impl<'a> IntoFieldPath<PetuniaFieldId<'a>> for &'a str {
    fn into_field_path(self) -> PetuniaFieldId<'a> {
        PetuniaFieldId(self)
    }
}

/// Resultado de validação de um formulário Petunia.
///
/// É um **dado do domínio**: quem monta o relatório é a regra de negócio (ex.:
/// conflito de atalhos), não a UI. Ordem de inserção é preservada, porque é ela
/// que o resumo inline apresenta.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PetuniaValidationReport {
    errors: Vec<(String, String)>,
}

impl PetuniaValidationReport {
    /// Relatório sem erros.
    pub fn new() -> Self {
        Self::default()
    }

    /// Acrescenta um erro de campo.
    pub fn with_error(mut self, field: impl Into<String>, message: impl Into<String>) -> Self {
        self.errors.push((field.into(), message.into()));
        self
    }

    /// `true` quando nenhum campo falhou.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Quantos campos falharam.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Mensagem de um campo, se ele falhou.
    pub fn error_for(&self, field: &str) -> Option<&str> {
        self.errors
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, message)| message.as_str())
    }

    /// Erros na ordem de declaração.
    pub fn errors(&self) -> impl Iterator<Item = (&str, &str)> {
        self.errors
            .iter()
            .map(|(field, message)| (field.as_str(), message.as_str()))
    }
}

impl EguiValidationReport for PetuniaValidationReport {
    type FieldPath<'a> = PetuniaFieldId<'a>;
    type Errors = Vec<(String, String)>;

    fn get_field_error(&self, field: Self::FieldPath<'_>) -> Option<Cow<'static, str>> {
        self.error_for(field.0)
            .map(|message| Cow::Owned(message.to_string()))
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn error_count(&self) -> usize {
        self.errors.len()
    }

    fn get_errors(&self) -> Option<&Self::Errors> {
        (!self.errors.is_empty()).then_some(&self.errors)
    }
}

/// Sessão de validação de um formulário, válida por **um frame**.
///
/// O produto monta o relatório a partir do estado, cria a sessão, desenha os
/// campos e desenha o resumo. O `egui_form` fica inteiramente encapsulado: nem
/// o [`PetuniaValidationReport`] nem a sessão expõem tipos da crate.
pub struct PetuniaFormSession {
    form: Form<PetuniaValidationReport>,
    errors: Vec<(String, String)>,
}

impl PetuniaFormSession {
    /// Cria a sessão a partir do relatório do domínio.
    pub fn new(report: PetuniaValidationReport) -> Self {
        let errors = report
            .errors()
            .map(|(f, m)| (f.to_string(), m.to_string()))
            .collect();
        Self {
            form: Form::new().add_report(report),
            errors,
        }
    }

    /// `true` quando não há erro nenhum.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Erros na ordem de declaração.
    pub fn errors(&self) -> impl Iterator<Item = (&str, &str)> {
        self.errors
            .iter()
            .map(|(field, message)| (field.as_str(), message.as_str()))
    }

    /// Revela **todos** os erros de uma vez e foca o primeiro campo inválido.
    ///
    /// É o gesto de "tentar confirmar": enquanto o usuário não pede, o erro só
    /// aparece no resumo; a partir daqui, cada campo inválido se marca sozinho
    /// (inclusive sem foco de teclado, como as coleções reordenáveis).
    pub fn reveal_errors(&mut self, ui: &mut Ui) {
        if let Err(errors) = self.form.try_submit(ui) {
            let _ = errors;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: PetuniaFormSpec = PetuniaFormSpec::new(96.0, 160.0);

    #[test]
    fn wide_panel_stays_inline_and_hands_the_rest_to_the_control() {
        let row = plan_row(420.0, &SPEC);
        assert_eq!(row.placement, PetuniaFormPlacement::Inline);
        assert_eq!(row.label_width, 96.0);
        assert!((row.control_width - (420.0 - 96.0 - 12.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn narrow_panel_stacks_instead_of_squeezing_the_control() {
        let row = plan_row(200.0, &SPEC);
        assert_eq!(row.placement, PetuniaFormPlacement::Stacked);
        assert_eq!(row.control_width, 200.0);
    }

    #[test]
    fn the_exact_minimum_is_the_inline_boundary() {
        let exactly = 96.0 + 12.0 + 160.0;
        assert_eq!(
            plan_row(exactly, &SPEC).placement,
            PetuniaFormPlacement::Inline
        );
        assert_eq!(
            plan_row(exactly - 1.0, &SPEC).placement,
            PetuniaFormPlacement::Stacked
        );
    }

    #[test]
    fn control_width_is_never_negative() {
        for available in [0.0, 1.0, 40.0, 120.0, 4000.0] {
            let row = plan_row(available, &SPEC);
            assert!(row.control_width >= 0.0, "largura {available}");
            assert!(row.control_width <= available.max(0.0));
        }
    }

    fn frame(ctx: &egui::Context, mut body: impl FnMut(&mut Ui)) {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(520.0, 400.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| body(ui));
        output.textures_delta.clear();
    }

    #[test]
    fn report_keeps_declaration_order_and_lookup() {
        let report = PetuniaValidationReport::new()
            .with_error("atalho.salvar", "compartilha o atalho com 'Salvar como'")
            .with_error("atalho.abrir", "duplicado");

        assert!(!report.is_valid());
        assert_eq!(report.error_count(), 2);
        assert_eq!(
            report.error_for("atalho.abrir"),
            Some("duplicado"),
            "o resumo segue a ordem de declaração do domínio"
        );
        assert_eq!(
            report.errors().next(),
            Some(("atalho.salvar", "compartilha o atalho com 'Salvar como'"))
        );
        assert_eq!(report.error_for("inexistente"), None);
        assert!(PetuniaValidationReport::new().is_valid());
    }

    #[test]
    fn the_summary_only_draws_while_there_are_errors() {
        let ctx = egui::Context::default();
        let form = PetuniaForm::new(96.0, 160.0);
        let invalid =
            PetuniaFormSession::new(PetuniaValidationReport::new().with_error("campo", "mensagem"));
        let valid = PetuniaFormSession::new(PetuniaValidationReport::new());

        let mut drawn = (false, false);
        frame(&ctx, |ui| {
            drawn.0 = form.error_summary(ui, &invalid);
            drawn.1 = form.error_summary(ui, &valid);
        });
        assert!(drawn.0, "com erro o resumo precisa aparecer");
        assert!(!drawn.1, "sem erro não pode sobrar banner");
    }

    #[test]
    fn a_validated_field_renders_in_both_placements() {
        let ctx = egui::Context::default();
        let form = PetuniaForm::new(96.0, 160.0);
        for width in [260.0, 900.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(width, 400.0),
                )),
                ..Default::default()
            };
            let mut session = PetuniaFormSession::new(
                PetuniaValidationReport::new().with_error("escala", "fora da faixa"),
            );
            let mut output = ctx.run_ui(raw, |ui| {
                form.validated_field(
                    ui,
                    &mut session,
                    "escala",
                    "Escala",
                    |ui: &mut Ui, w: f32| {
                        ui.add(egui::Slider::new(&mut 1.0_f32, 0.0..=2.0))
                            .on_hover_text(format!("{w:.0}"))
                    },
                );
            });
            output.textures_delta.clear();
        }
    }

    #[test]
    fn revealing_errors_focuses_the_invalid_field() {
        let ctx = egui::Context::default();
        let form = PetuniaForm::new(96.0, 160.0);
        let mut session =
            PetuniaFormSession::new(PetuniaValidationReport::new().with_error("campo", "mensagem"));
        // Dois frames: o primeiro desenha os campos, o segundo revela os erros.
        for step in 0..2 {
            frame(&ctx, |ui| {
                let mut text = String::new();
                form.validated_control(ui, &mut session, "campo", |ui| {
                    ui.text_edit_singleline(&mut text)
                });
                if step == 1 {
                    session.reveal_errors(ui);
                }
            });
        }
        assert!(
            ctx.memory(|m| m.focused().is_some()),
            "depois de revelar, o primeiro campo inválido precisa receber foco"
        );
        assert!(!session.is_valid());
    }

    #[test]
    fn the_form_renders_in_both_placements_without_panic() {
        let ctx = egui::Context::default();
        let form = PetuniaForm::new(96.0, 160.0);
        for width in [200.0, 800.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(width, 400.0),
                )),
                ..Default::default()
            };
            ctx.run_ui(raw, |ui| {
                form.section(ui, "Interface", Some("descrição"));
                let mut on = false;
                form.toggle(ui, "Mostrar prateleira", &mut on);
                let width = form.field(ui, "density", "Densidade", |ui, width| {
                    ui.label(format!("{width:.0}px"));
                    width
                });
                assert!(width >= 0.0);
            })
            .textures_delta
            .clear();
        }
    }
}
