//! # `PetuniaDragList` — reordenação por arrasto (§49)
//!
//! Wave 7: listas ordenáveis deixam de depender de setas ↑/↓. O produto declara
//! **os itens** e o **desenho da linha**; o adapter é dono do esqueleto da linha
//! (grip + conteúdo), da detecção do arrasto e da aplicação da nova ordem.
//!
//! ```text
//! toolbar.rs (config da paleta)  →  PetuniaDragList::show(..)
//! ```
//!
//! A crate (`egui_dnd`) não aparece no código de produto — regra §22.1. O
//! contrato é do Petunia:
//!
//! | produto | adapter |
//! | --- | --- |
//! | título/checkbox da linha | desenha dentro do closure |
//! | onde fica o grip | decisão do adapter (posição inicial da linha) |
//! | ordem final | [`move_item`] aplicado no drop (via [`apply_drag_update`]) |
//!
//! ## Semântica de "soltar em cima"
//!
//! [`move_item`] é a regra do Petunia e é pura: o item **sai** de `from` e
//! **entra** em `to` — arrastar algo para cima do item 2 coloca o item na
//! posição 2 e empurra o resto para a direita. `to` fora da faixa é truncado
//! para o fim da lista (soltar depois do último item), nunca é pânico.
//!
//! ## Três armadilhas medidas do `egui_dnd` 0.17 (§ ledger)
//!
//! 1. **O limiar de clique só é avaliado com o ponteiro sobre o handle.** O
//!    motor só passa de "esperando limiar" para "pode arrastar" se
//!    `contains_pointer()` for verdadeiro no instante em que o ponteiro já
//!    andou mais de 1px (`click_tolerance`); o caminho alternativo é segurar
//!    250ms (`click_tolerance_timeout`). Medido em sonda: um primeiro passo de
//!    6px para fora de um grip de 18×20 **não** inicia o arrasto; o mesmo passo
//!    começando *dentro* do grip inicia. Consequência de projeto: o grip precisa
//!    ser grande o suficiente para o primeiro movimento — não é decorativo.
//! 2. **`to` do motor é limite exclusivo quando o arrasto desce.** `to = 2`
//!    partindo de `0` deixa o item na posição **1**, não na 2. [`apply_drag_update`]
//!    converte para a semântica de inserção do Petunia e a equivalência é
//!    verificada contra `egui_dnd::utils::shift_vec` para todos os pares de
//!    posições (`the_adapter_matches_the_engine_order_for_every_pair`).
//! 3. **O closure da linha é um desenho, não um acumulador.** Enquanto um item é
//!    arrastado, a crate chama o mesmo closure de novo para desenhar o item
//!    flutuante sob o ponteiro: medido, o closure roda `n + 1` vezes por frame
//!    durante o arrasto. Mutação de produto só em reação a
//!    `clicked()`/`changed()` continua segura.

use egui::{Color32, Id, Response, Sense, Ui, vec2};

use crate::icon_registry::PetuniaIcon;
use crate::tokens;

/// Especificação de uma lista reordenável.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaDragSpec {
    /// Tempo da animação de retorno/encaixe do item, em **segundos** — é a
    /// unidade das APIs de animação do egui/`egui_animation`.
    pub settle_s: f32,
    /// Largura da faixa do grip que dispara o arrasto.
    pub grip_width: f32,
    /// Altura da faixa do grip.
    pub grip_height: f32,
}

impl PetuniaDragSpec {
    /// Especificação com o motion do sistema ([`crate::foundation::motion`]).
    pub const fn new() -> Self {
        Self {
            settle_s: crate::foundation::motion::seconds(
                crate::foundation::motion::PetuniaMotion::BASE,
            ),
            grip_width: 18.0,
            grip_height: 20.0,
        }
    }

    /// Define o tempo de encaixe, em segundos.
    pub const fn with_settle_s(mut self, seconds: f32) -> Self {
        self.settle_s = seconds;
        self
    }

    /// Define o tamanho da faixa do grip.
    pub const fn with_grip(mut self, width: f32, height: f32) -> Self {
        self.grip_width = width;
        self.grip_height = height;
        self
    }
}

impl Default for PetuniaDragSpec {
    fn default() -> Self {
        Self::new()
    }
}

/// Estado da linha neste frame. O produto lê, nunca constrói.
pub struct PetuniaDragRow {
    /// Posição da linha na lista **neste** frame.
    pub index: usize,
    /// `true` quando esta linha é a que está sendo arrastada (ou sua cópia
    /// flutuante).
    pub dragging: bool,
    /// Resposta do grip, para tooltip/estado visual do produto.
    pub handle: Response,
}

/// Lista reordenável por arrasto.
///
/// O produto guarda **uma** instância estática
/// (`static TOOLBAR_ORDER: PetuniaDragList = PetuniaDragList::new(..)`) e chama
/// [`PetuniaDragList::show`] uma vez por frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetuniaDragList {
    pub spec: PetuniaDragSpec,
}

impl PetuniaDragList {
    /// Lista com a especificação de motion declarada.
    pub const fn new(spec: PetuniaDragSpec) -> Self {
        Self { spec }
    }

    /// Desenha a lista e devolve `true` quando a ordem mudou neste frame.
    ///
    /// `id_of` dá a identidade estável de cada item (a crate guarda o estado do
    /// arrasto por `Id`) — ids repetidos fazem duas linhas arrastarem juntas. A
    /// ordem é aplicada no **drop**, não a cada frame: durante o arrasto a
    /// lista fica parada e a crate desenha o item flutuante.
    pub fn show<T>(
        &self,
        ui: &mut Ui,
        salt: impl egui::AsId,
        items: &mut Vec<T>,
        id_of: impl Fn(&T) -> Id,
        mut row: impl FnMut(&mut Ui, &mut T, &PetuniaDragRow),
    ) -> bool {
        let ids: Vec<DragId> = items.iter().map(|item| DragId(id_of(item))).collect();
        let spec = self.spec;

        let response = if ui.is_enabled() {
            egui_dnd::dnd(ui, salt)
                .with_animation_time(spec.settle_s)
                .show(ids.into_iter(), |ui, _drag_id, handle, state| {
                    let index = state.index;
                    let dragging = state.dragged;
                    // `index` vem do iterador na ordem atual da lista: é
                    // exatamente a posição do item na fatia.
                    let Some(item) = items.get_mut(index) else {
                        return;
                    };
                    ui.horizontal(|ui| {
                        let handle_response = handle.ui(ui, |ui| draw_grip(ui, spec, dragging));
                        row(
                            ui,
                            item,
                            &PetuniaDragRow {
                                index,
                                dragging,
                                handle: handle_response,
                            },
                        );
                    });
                })
        } else {
            // Lista desabilitada: nada de arrasto, mas a linha continua
            // legível (o produto vê `dragging: false` e nenhum movimento).
            for (index, item) in items.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let response = ui
                        .allocate_response(vec2(spec.grip_width, spec.grip_height), Sense::hover());
                    draw_grip_at(ui, response.rect, false, false);
                    row(
                        ui,
                        item,
                        &PetuniaDragRow {
                            index,
                            dragging: false,
                            handle: response,
                        },
                    );
                });
            }
            return false;
        };

        let Some(update) = response.final_update() else {
            return false;
        };
        apply_drag_update(items, update.from, update.to)
    }
}

/// Move o item de `from` para `to` (remover e inserir).
///
/// Regra do Petunia, pura e testável: o item sai de `from` e **entra** em `to`,
/// empurrando o que estava lá para a direita. `to` além do fim é truncado.
pub fn move_item<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from >= items.len() {
        return false;
    }
    let to = to.min(items.len() - 1);
    if from == to {
        return false;
    }
    let item = items.remove(from);
    items.insert(to, item);
    true
}

/// Traduz a instrução do motor de arrasto para [`move_item`].
///
/// Medido no motor (§ ledger): quando o arrasto **desce**, `to` é o limite
/// exclusivo do trecho deslocado (o item termina em `to - 1`); quando **sobe**,
/// `to` já é o índice de inserção. A conversão existe para o resultado coincidir
/// com o que a própria crate faria — e é verificada contra ela em teste.
pub fn apply_drag_update<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    let target = if to > from { to.saturating_sub(1) } else { to };
    move_item(items, from, target)
}

/// Id interno da linha — a crate exige um tipo com identidade própria.
#[derive(Debug, Clone, Copy)]
struct DragId(Id);

impl egui_dnd::DragDropItem for DragId {
    fn id(&self) -> Id {
        self.0
    }
}

/// Desenha o grip dentro da faixa de arrasto do `Handle`.
///
/// A resposta de interação é a do próprio `Handle` (é ele quem detecta o
/// arrasto); aqui só alocamos a faixa e pintamos os pontos.
fn draw_grip(ui: &mut Ui, spec: PetuniaDragSpec, dragging: bool) {
    let response = ui.allocate_response(vec2(spec.grip_width, spec.grip_height), Sense::click());
    let rect = response.rect;
    let hovered = ui.rect_contains_pointer(rect);
    draw_grip_at(ui, rect, hovered, dragging);
    if hovered {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
    }
}

/// Pinta os dois pontinhos por linha do grip.
fn draw_grip_at(ui: &Ui, rect: egui::Rect, hovered: bool, dragging: bool) {
    let color = if dragging {
        tokens::ACCENT_BLUE
    } else if hovered {
        tokens::TEXT_SECONDARY
    } else {
        tokens::TEXT_MUTED
    };
    let painter = ui.painter();
    let dot = 1.5;
    let gap = 4.0;
    let column_gap = 5.0;
    let center = rect.center();
    for (row, dy) in [-gap, 0.0, gap].into_iter().enumerate() {
        let _ = row;
        for dx in [-column_gap * 0.5, column_gap * 0.5] {
            let dot_rect = egui::Rect::from_center_size(
                egui::pos2(center.x + dx, center.y + dy),
                vec2(dot, dot),
            );
            painter.rect_filled(dot_rect, 0.5, color);
        }
    }
}

/// Ícone semântico do grip, para o produto usar em tooltip/label.
pub const GRIP_ICON: PetuniaIcon = PetuniaIcon::Move;

/// Cor do item que está sendo arrastado (feedback de estado).
pub fn dragging_tint() -> Color32 {
    tokens::BG_SURFACE_ACTIVE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn moving_down_pushes_the_target_right() {
        let mut items = list(&["a", "b", "c", "d"]);
        assert!(move_item(&mut items, 0, 2));
        assert_eq!(items, ["b", "c", "a", "d"]);
    }

    #[test]
    fn moving_up_lands_exactly_on_the_hovered_index() {
        let mut items = list(&["a", "b", "c", "d"]);
        assert!(move_item(&mut items, 3, 1));
        assert_eq!(items, ["a", "d", "b", "c"]);
    }

    #[test]
    fn dropping_past_the_end_goes_to_the_end_without_panicking() {
        let mut items = list(&["a", "b", "c"]);
        assert!(move_item(&mut items, 0, 99));
        assert_eq!(items, ["b", "c", "a"]);
    }

    #[test]
    fn no_op_moves_report_no_change() {
        let mut items = list(&["a", "b", "c"]);
        assert!(!move_item(&mut items, 1, 1));
        assert!(!move_item(&mut items, 7, 0));
        assert_eq!(items, ["a", "b", "c"]);
    }

    #[test]
    fn any_move_is_a_permutation_of_the_original() {
        let original = list(&["a", "b", "c", "d", "e"]);
        for from in 0..original.len() {
            for to in 0..original.len() {
                let mut items = original.clone();
                move_item(&mut items, from, to);
                let mut sorted = items.clone();
                sorted.sort();
                assert_eq!(sorted, original, "from={from} to={to}");
            }
        }
    }

    /// Roda um frame com os eventos dados e devolve o `Context` que os recebeu.
    fn frame(
        ctx: &egui::Context,
        adapter: &PetuniaDragList,
        items: &mut Vec<String>,
        events: Vec<egui::Event>,
        changed: &mut bool,
    ) {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(200.0, 240.0),
            )),
            events,
            ..Default::default()
        };
        let output = ctx.run_ui(raw, |ui| {
            *changed |= adapter.show(
                ui,
                "probe-drag",
                items,
                |id| Id::new(id.as_str()),
                |ui, id, _row| {
                    ui.label(id.as_str());
                },
            );
        });
        let mut output = output;
        output.textures_delta.clear();
    }

    #[test]
    fn a_real_drag_reorders_the_list_on_drop() {
        // Arrasta a primeira linha para cima da segunda. A lista é desenhada em
        // coordenadas conhecidas: linha 0 em y≈8, linha 1 em y≈28, o grip é a
        // faixa mais à esquerda (x≈4..22).
        let ctx = egui::Context::default();
        let adapter = PetuniaDragList::new(PetuniaDragSpec::new());
        let mut items = list(&["a", "b", "c"]);
        let mut changed = false;
        let expected = {
            let mut reference = list(&["a", "b", "c"]);
            let mut raw = list(&["a", "b", "c"]);
            // A referência é a própria crate: mesmo resultado, sempre.
            egui_dnd::utils::shift_vec(0, 2, &mut raw);
            reference.clear();
            reference.extend(raw);
            reference
        };
        let from = egui::pos2(12.0, 16.0);
        let to = egui::pos2(12.0, 40.0);

        // Dois frames de aquecimento: a lista precisa existir antes do clique.
        frame(
            &ctx,
            &adapter,
            &mut items,
            vec![egui::Event::PointerMoved(from)],
            &mut changed,
        );
        frame(&ctx, &adapter, &mut items, vec![], &mut changed);

        let down = vec![egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        }];
        frame(&ctx, &adapter, &mut items, down, &mut changed);

        // Movimento além do limiar de clique. O primeiro passo tem de caber
        // dentro do grip: a crate só aceita passar o limiar enquanto o ponteiro
        // ainda está sobre o handle (o fallback é o timeout de clique).
        for pos in [
            egui::pos2(12.0, 18.0),
            egui::pos2(12.0, 22.0),
            egui::pos2(12.0, 30.0),
            to,
            to,
        ] {
            let moved = vec![egui::Event::PointerMoved(pos)];
            frame(&ctx, &adapter, &mut items, moved, &mut changed);
        }

        let up = vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }];
        frame(&ctx, &adapter, &mut items, up, &mut changed);
        // O drop chega como `final_update` no frame seguinte.
        frame(&ctx, &adapter, &mut items, vec![], &mut changed);

        assert!(changed, "a mudança de ordem precisa ser reportada");
        assert_eq!(
            items, expected,
            "o resultado do produto precisa coincidir com o do motor de arrasto"
        );
    }

    #[test]
    fn the_adapter_matches_the_engine_order_for_every_pair() {
        // A tradução `to → índice de inserção` é verificada contra a crate, para
        // todos os pares de posições, em uma lista de 5 itens.
        for from in 0..5usize {
            for to in 0..=5usize {
                let mut mine = list(&["a", "b", "c", "d", "e"]);
                apply_drag_update(&mut mine, from, to);
                let mut engine = list(&["a", "b", "c", "d", "e"]);
                egui_dnd::utils::shift_vec(from, to, &mut engine);
                assert_eq!(mine, engine, "from={from} to={to}");
            }
        }
    }

    #[test]
    fn a_disabled_list_still_draws_its_rows() {
        let ctx = egui::Context::default();
        let adapter = PetuniaDragList::new(PetuniaDragSpec::new());
        let mut items = list(&["a", "b", "c"]);
        let mut drawn = 0;
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(320.0, 240.0),
            )),
            ..Default::default()
        };
        ctx.run_ui(raw, |ui| {
            ui.disable();
            let changed = adapter.show(
                ui,
                "drag-probe",
                &mut items,
                |item| Id::new(item),
                |ui, item, row| {
                    drawn += 1;
                    ui.label(item.as_str());
                    assert_eq!(row.index, drawn - 1);
                },
            );
            assert!(!changed);
        })
        .textures_delta
        .clear();
        assert_eq!(drawn, 3);
    }
}
