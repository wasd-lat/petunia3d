//! # `PetuniaPopup` — superfícies flutuantes do produto (§34)
//!
//! Um cartão de ferramenta, uma paleta ou um menu flutuante nascem daqui: frame
//! dos tokens, posição ancorada a uma região, z-order próprio e — o contrato que
//! faltava — **perde o foco quando o usuário clica fora dela**, sem exigir
//! Enter nem Espaço.
//!
//! ```text
//! product  →  PetuniaPopup::{panel, menu}  →  egui::Window (confinado aqui)
//! ```
//!
//! Dois modos, um só estado na memória do `Context` (por `Id`):
//!
//! | modo | ao clicar fora | ao apertar Escape |
//! | --- | --- | --- |
//! | [`PetuniaPopupMode::Panel`] | **recolhe** para o cabeçalho (segue visível como faixa) | recolhe |
//! | [`PetuniaPopupMode::Menu`] | **fecha** (o dono reabre) | fecha |
//!
//! Recolher ≠ fechar de propósito: um painel de ferramenta que desaparece quando
//! o usuário toca a tela/canvas fica inalcançável no meio do trabalho. A faixa
//! permanece, e o clique no cabeçalho devolve o painel inteiro.
//!
//! Nada de estado novo em `UiState`: a superfície guarda o próprio retângulo na
//! memória do motor, então o teste de "cliquei fora" usa a geometria **medida**
//! do frame anterior, não uma estimativa.

use egui::{Color32, Context, Id, Pos2, Rect, ScrollArea, Stroke, Ui};

use crate::tokens;

/// Modo de uma superfície flutuante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetuniaPopupMode {
    /// Painel flutuante de ferramenta (recolhe ao perder o foco).
    Panel,
    /// Menu/lista (fecha ao perder o foco).
    Menu,
}

/// Cores e moldura da superfície, resolvidas por quem conhece o `AppState`.
///
/// O adapter não lê o estado da aplicação: ele recebe o estilo pronto — é o
/// mesmo contrato dos outros adapters (o produto resolve o tema, o adapter
/// resolve a geometria e o comportamento).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaPopupStyle {
    pub fill: Color32,
    pub stroke: Stroke,
    pub title: Color32,
    pub muted: Color32,
}

impl PetuniaPopupStyle {
    /// Estilo canônico do tema ativo.
    pub fn from_state(state: &petunia_core::AppState) -> Self {
        Self {
            fill: tokens::bg_panel(state),
            stroke: tokens::stroke_border_dyn(state),
            title: tokens::text_primary(state),
            muted: tokens::text_muted(state),
        }
    }
}

/// Rótulos do cabeçalho (i18n é do produto; o adapter só desenha).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetuniaPopupLabels {
    /// Dica do chevron quando a superfície está recolhida.
    pub expand: String,
    /// Dica do chevron quando a superfície está aberta.
    pub collapse: String,
}

/// Estado da superfície (memória do `Context`, por `Id`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupState {
    open: bool,
    collapsed: bool,
    /// Retângulo **medido** no frame anterior (base do teste de clique fora).
    rect: Rect,
}

impl Default for PopupState {
    fn default() -> Self {
        Self {
            open: true,
            collapsed: false,
            rect: Rect::NOTHING,
        }
    }
}

/// Resultado do frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaPopupResponse {
    /// Retângulo ocupado neste frame (a faixa, quando recolhida).
    pub rect: Rect,
    /// A superfície está recolhida (só o cabeçalho)?
    pub collapsed: bool,
}

/// Superfície flutuante (ver o módulo).
pub struct PetuniaPopup {
    id: Id,
    mode: PetuniaPopupMode,
    title: String,
    style: PetuniaPopupStyle,
    labels: Option<PetuniaPopupLabels>,
    at: Option<Pos2>,
    bounds: Option<Rect>,
    width: f32,
    max_height: f32,
}

impl PetuniaPopup {
    fn new(
        id: impl Into<Id>,
        mode: PetuniaPopupMode,
        title: impl Into<String>,
        style: PetuniaPopupStyle,
    ) -> Self {
        Self {
            id: id.into(),
            mode,
            title: title.into(),
            style,
            labels: None,
            at: None,
            bounds: None,
            width: 280.0,
            max_height: 520.0,
        }
    }

    /// Painel flutuante de ferramenta (recolhe ao perder o foco).
    pub fn panel(id: impl Into<Id>, title: impl Into<String>, style: PetuniaPopupStyle) -> Self {
        Self::new(id, PetuniaPopupMode::Panel, title, style)
    }

    /// Menu/lista flutuante (fecha ao perder o foco).
    pub fn menu(id: impl Into<Id>, title: impl Into<String>, style: PetuniaPopupStyle) -> Self {
        Self::new(id, PetuniaPopupMode::Menu, title, style)
    }

    /// Rótulos do chevron do cabeçalho (sem eles, o cabeçalho não tem toggle).
    pub fn labels(mut self, labels: PetuniaPopupLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Posição fixa (normalmente `região.left_top() + margem`).
    pub fn at(mut self, pos: impl Into<Pos2>) -> Self {
        self.at = Some(pos.into());
        self
    }

    /// Retângulo em que a superfície pode existir (nunca escapa dele).
    pub fn within(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Largura do conteúdo.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(80.0);
        self
    }

    /// Altura máxima do conteúdo (acima disso o corpo rola).
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(48.0);
        self
    }

    /// Id canônico da superfície (para [`Self::reveal`] e testes).
    pub fn id(&self) -> Id {
        self.id
    }

    /// Reabre e expande a superfície agora — é o que o botão que a abre chama.
    pub fn reveal(ctx: &Context, id: impl Into<Id>) {
        let id = id.into();
        ctx.data_mut(|d| {
            let mut state = d.get_temp::<PopupState>(id).unwrap_or_default();
            state.open = true;
            state.collapsed = false;
            d.insert_temp(id, state);
        });
    }

    /// A superfície está visível (não fechada)?
    pub fn is_open(ctx: &Context, id: impl Into<Id>) -> bool {
        ctx.data(|d| {
            d.get_temp::<PopupState>(id.into())
                .map(|state| state.open)
                .unwrap_or(true)
        })
    }

    /// A superfície está recolhida (só o cabeçalho)?
    pub fn is_collapsed(ctx: &Context, id: impl Into<Id>) -> bool {
        ctx.data(|d| {
            d.get_temp::<PopupState>(id.into())
                .map(|state| state.collapsed)
                .unwrap_or(false)
        })
    }

    /// Fecha (modo [`PetuniaPopupMode::Menu`]) sem depender de clique.
    pub fn close(ctx: &Context, id: impl Into<Id>) {
        let id = id.into();
        ctx.data_mut(|d| {
            let mut state = d.get_temp::<PopupState>(id).unwrap_or_default();
            state.open = false;
            d.insert_temp(id, state);
        });
    }

    /// Desenha a superfície e devolve o que aconteceu neste frame.
    ///
    /// `None` = nada desenhado (menus fechados). Painéis sempre desenham: no
    /// modo recolhido, desenham a faixa do cabeçalho.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<PetuniaPopupResponse> {
        let mut state = ctx
            .data(|d| d.get_temp::<PopupState>(self.id))
            .unwrap_or_default();
        if !state.open {
            return None;
        }

        // Perda de foco: Escape ou um clique fora do retângulo medido no frame
        // anterior. A medida vem do próprio motor — nada de estimar por largura.
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let pressed = ctx.input(|i| i.pointer.any_pressed());
        let pressed_inside = ctx
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|pos| state.rect.contains(pos));
        let measured = state.rect != Rect::NOTHING;
        let lost_focus = escape || (pressed && measured && !pressed_inside);
        if lost_focus {
            match self.mode {
                PetuniaPopupMode::Panel => state.collapsed = true,
                PetuniaPopupMode::Menu => {
                    state.open = false;
                    ctx.data_mut(|d| d.insert_temp(self.id, state));
                    return None;
                }
            }
        }

        // Sem região declarada, o motor limita à tela do frame.
        let bounds = self.bounds.unwrap_or(egui::Rect::EVERYTHING);
        let frame = egui::Frame::new()
            .fill(self.style.fill)
            .stroke(self.style.stroke)
            .corner_radius(tokens::RADIUS_CONTAINER)
            .inner_margin(egui::Margin::symmetric(10, 8));
        let mut window = egui::Window::new("")
            .id(self.id.with("window"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .default_width(self.width)
            .max_width(self.width + 20.0)
            .max_height(self.max_height)
            .constrain_to(bounds)
            .frame(frame);
        if let Some(pos) = self.at {
            window = window.fixed_pos(pos);
        }

        let labels = self.labels;
        let title = self.title;
        let style = self.style;
        let collapsed = state.collapsed;
        let body_height = (self.max_height - 44.0).max(48.0);
        let shown = window.show(ctx, |ui| {
            ui.set_width(self.width);
            let mut toggled = None;
            if !title.is_empty() {
                ui.horizontal(|ui| {
                    if let Some(labels) = labels.as_ref() {
                        let tip = if collapsed {
                            labels.expand.clone()
                        } else {
                            labels.collapse.clone()
                        };
                        if crate::widgets::chevron_toggle(ui, &tip, !collapsed).clicked() {
                            toggled = Some(!collapsed);
                        }
                    }
                    ui.label(
                        egui::RichText::new(&title)
                            .strong()
                            .size(12.0)
                            .color(style.title),
                    );
                });
            }
            let mut open = !collapsed;
            if let Some(next) = toggled {
                open = next;
            }
            if open {
                ui.separator();
                // O corpo rola por conta própria: o cabeçalho continua visível
                // quando o conteúdo não cabe na região.
                ScrollArea::vertical()
                    .id_salt(self.id.with("body"))
                    .max_height(body_height)
                    .auto_shrink([false, true])
                    .show(ui, add_contents);
            }
            open
        });

        let shown = shown?;
        let rect = shown.response.rect.intersect(bounds);
        let collapsed = !shown.inner.unwrap_or(true);
        let next = PopupState {
            open: true,
            collapsed,
            rect,
        };
        ctx.data_mut(|d| d.insert_temp(self.id, next));
        Some(PetuniaPopupResponse { rect, collapsed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = "test.paint.card";

    fn style() -> PetuniaPopupStyle {
        PetuniaPopupStyle {
            fill: Color32::from_gray(32),
            stroke: Stroke::new(1.0_f32, Color32::from_gray(64)),
            title: Color32::from_gray(220),
            muted: Color32::from_gray(120),
        }
    }

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))
    }

    /// Desenha o cartão num frame com os eventos de ponteiro dados.
    fn frame(ctx: &Context, events: Vec<egui::Event>) -> Option<PetuniaPopupResponse> {
        let raw = egui::RawInput {
            screen_rect: Some(screen()),
            events,
            ..Default::default()
        };
        let mut out = None;
        ctx.run_ui(raw, |ui| {
            out = PetuniaPopup::panel(CARD, "Pincel", style())
                .at(Pos2::new(20.0, 20.0))
                .within(screen())
                .labels(PetuniaPopupLabels {
                    expand: "expandir".into(),
                    collapse: "recolher".into(),
                })
                .show(ui.ctx(), |ui| {
                    ui.label("conteúdo");
                });
        })
        .textures_delta
        .clear();
        out
    }

    fn press(pos: Pos2) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn a_panel_collapses_when_the_user_clicks_outside_it() {
        let ctx = Context::default();
        let first = frame(&ctx, Vec::new()).expect("painel desenha no primeiro frame");
        assert!(!first.collapsed);
        // Clique longe do cartão (o cartão nasce em 20,20): perde o foco.
        frame(&ctx, vec![press(Pos2::new(700.0, 500.0))]);
        assert!(PetuniaPopup::is_collapsed(&ctx, CARD));
        let response = frame(&ctx, Vec::new()).expect("a faixa continua desenhada");
        assert!(response.collapsed);
        assert!(
            response.rect.height() < first.rect.height(),
            "recolhido precisa ocupar menos altura: {:?} vs {:?}",
            response.rect,
            first.rect
        );
    }

    #[test]
    fn a_panel_keeps_the_focus_when_the_click_lands_inside_it() {
        let ctx = Context::default();
        let first = frame(&ctx, Vec::new()).expect("painel desenha");
        frame(&ctx, vec![press(first.rect.center())]);
        assert!(!PetuniaPopup::is_collapsed(&ctx, CARD));
    }

    #[test]
    fn reveal_expands_a_collapsed_panel_again() {
        let ctx = Context::default();
        frame(&ctx, Vec::new());
        frame(&ctx, vec![press(Pos2::new(780.0, 580.0))]);
        assert!(PetuniaPopup::is_collapsed(&ctx, CARD));
        PetuniaPopup::reveal(&ctx, CARD);
        assert!(PetuniaPopup::is_open(&ctx, CARD));
        assert!(!PetuniaPopup::is_collapsed(&ctx, CARD));
    }

    /// Desenha um menu (fecha ao perder o foco) com os eventos dados.
    fn menu_frame(ctx: &Context, events: Vec<egui::Event>) -> Option<PetuniaPopupResponse> {
        let raw = egui::RawInput {
            screen_rect: Some(screen()),
            events,
            ..Default::default()
        };
        let mut out = None;
        ctx.run_ui(raw, |ui| {
            out = PetuniaPopup::menu("test.menu", "Menu", style())
                .at(Pos2::new(10.0, 10.0))
                .within(screen())
                .show(ui.ctx(), |ui| {
                    ui.label("item");
                });
        })
        .textures_delta
        .clear();
        out
    }

    #[test]
    fn a_menu_closes_on_escape_and_stays_closed() {
        let ctx = Context::default();
        assert!(menu_frame(&ctx, Vec::new()).is_some());
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(
            menu_frame(&ctx, vec![escape]).is_none(),
            "Escape fecha o menu"
        );
        assert!(!PetuniaPopup::is_open(&ctx, "test.menu"));
        PetuniaPopup::reveal(&ctx, "test.menu");
        assert!(menu_frame(&ctx, Vec::new()).is_some(), "o dono reabre");
    }
}
