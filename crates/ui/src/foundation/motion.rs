//! Motion — durações, curvas e o **backend real** de animação (§30, §49).
//!
//! Wave 7: o módulo deixa de ser só um catálogo de valores. As durações
//! continuam aqui (dono único), e a interpolação passa a existir atrás de três
//! funções com semântica de **feedback de estado**:
//!
//! | função | caso de uso |
//! | --- | --- |
//! | [`PetuniaMotion::reveal`] | alpha 0→1 de algo que aparece/some |
//! | [`PetuniaMotion::animate`] | valor numérico interpolado |
//! | [`PetuniaMotion::section`] | seção que abre/fecha animando a altura |
//!
//! Contrato do capítulo 36 que continua valendo: **nenhuma animação contínua
//! puramente decorativa.** Motion aqui serve para dar feedback de estado
//! (abrir/fechar, revelar campo, validar), nunca para decorar.
//!
//! ## Onde entra a crate
//!
//! `egui_animation` é detalhe de implementação: o produto chama
//! `PetuniaMotion::section(..)` e não sabe qual backend resolve a curva. Trocar
//! o backend (ou cair para `Context::animate_bool_with_time`, que é a mesma
//! máquina do egui) é edição deste arquivo.
//!
//! ## Sem animação quando o usuário não quer
//!
//! Todas as funções respeitam `ui.style().animation_time == 0.0`: com motion
//! desligado (acessibilidade / preferência), a transição é instantânea — o
//! estado final é idêntico, só não há interpolação.

use std::time::Duration;

use egui::{Context, Id, Pos2, Ui};

/// Curva de animação. Mapeada para o backend em [`Easing::curve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Acelera no fim — padrão para entradas.
    #[default]
    EaseOut,
    /// Desacelera no início.
    EaseIn,
    /// Simétrica.
    EaseInOut,
    /// Sem interpolação (troca instantânea).
    Linear,
}

impl Easing {
    /// Função de interpolação do backend.
    ///
    /// Pura e testável: recebe 0..1 e devolve 0..1.
    pub fn curve(self) -> fn(f32) -> f32 {
        match self {
            Easing::EaseOut => egui_animation::easing::cubic_out,
            Easing::EaseIn => egui_animation::easing::cubic_in,
            Easing::EaseInOut => egui_animation::easing::cubic_in_out,
            Easing::Linear => egui_animation::easing::linear,
        }
    }
}

/// Durações canônicas e funções de animação do sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetuniaMotion;

impl PetuniaMotion {
    /// Feedback imediato: hover, foco, press.
    pub const INSTANT: Duration = Duration::from_millis(80);

    /// Transição padrão: abrir/fechar painel, revelar campo.
    pub const BASE: Duration = Duration::from_millis(160);

    /// Transição deliberada: modal, troca de workspace, drill-down.
    pub const SLOW: Duration = Duration::from_millis(240);

    /// Curva padrão.
    pub const EASING: Easing = Easing::EaseOut;

    /// Todas as durações, da mais curta à mais longa.
    pub const ALL: [Duration; 3] = [Self::INSTANT, Self::BASE, Self::SLOW];

    /// Alpha de revelação em `0..=1` para um estado booleano.
    ///
    /// Use para pintar algo que aparece/some (prateleira contextual, faixa de
    /// aviso, badge): o valor **chega** a 1.0 quando visível e a 0.0 quando não,
    /// então nada fica meio transparente parado.
    pub fn reveal(ctx: &Context, id: impl egui::AsId, visible: bool) -> f32 {
        if animation_off(ctx) {
            return if visible { 1.0 } else { 0.0 };
        }
        egui_animation::animate_bool_eased(
            ctx,
            id,
            visible,
            Self::EASING.curve(),
            seconds(Self::BASE),
        )
        .clamp(0.0, 1.0)
    }

    /// Valor animado com a curva declarada.
    pub fn animate(ctx: &Context, id: impl egui::AsId, value: f32, duration: Duration) -> f32 {
        if animation_off(ctx) {
            return value;
        }
        egui_animation::animate_eased(ctx, id, value, seconds(duration), Self::EASING.curve())
    }

    /// Posição animada (listas que reordenam, item que encaixa).
    pub fn position(ui: &mut Ui, id: impl egui::AsId, value: Pos2, duration: Duration) -> Pos2 {
        if animation_off(ui.ctx()) {
            return value;
        }
        egui_animation::animate_position(
            ui,
            id,
            value,
            seconds(duration),
            Self::EASING.curve(),
            /* scroll_correction */ true,
        )
    }

    /// Seção que abre/fecha **animando a altura** em vez de saltar.
    ///
    /// O conteúdo continua sendo desenhado enquanto a seção fecha (recortado e
    /// encolhendo), então o closure precisa ser um desenho idempotente — mesma
    /// disciplina das linhas de taffy e das listas de arrasto.
    pub fn section(ui: &mut Ui, id: impl Into<Id>, open: bool, add_contents: impl FnOnce(&mut Ui)) {
        if animation_off(ui.ctx()) {
            if open {
                add_contents(ui);
            }
            return;
        }
        egui_animation::Collapse::vertical(id, open)
            .with_animation_time(seconds(Self::BASE))
            .ui(ui, add_contents);
    }
}

/// Duração em milissegundos — para exibir (vitrine, docs).
pub fn millis(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

/// Duração em **segundos** — a unidade das APIs de animação do egui e do
/// `egui_animation`. Passar milissegundos aqui deixa a transição ~1000× mais
/// lenta e é o erro mais fácil de cometer (medido em sonda: 160 em vez de 0,16
/// avança 0,1% por frame).
pub const fn seconds(duration: Duration) -> f32 {
    duration.as_secs_f32()
}

/// Motion desligado: o tema pediu `animation_time == 0`.
fn animation_off(ctx: &Context) -> bool {
    ctx.global_style().animation_time <= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_ordered_and_under_a_quarter_second() {
        for pair in PetuniaMotion::ALL.windows(2) {
            assert!(pair[0] < pair[1], "durações precisam ser crescentes");
        }
        assert!(
            PetuniaMotion::SLOW <= Duration::from_millis(300),
            "motion de UI não pode parecer lento"
        );
    }

    #[test]
    fn millis_conversion_is_exact() {
        assert_eq!(millis(PetuniaMotion::BASE), 160.0);
        assert_eq!(millis(PetuniaMotion::INSTANT), 80.0);
    }

    #[test]
    fn seconds_conversion_is_exact() {
        assert!((seconds(PetuniaMotion::BASE) - 0.16).abs() < 1e-6);
        assert!((seconds(PetuniaMotion::SLOW) - 0.24).abs() < 1e-6);
    }

    #[test]
    fn a_reveal_advances_at_the_declared_rate() {
        // Invariante de unidade: em 8 frames de 20ms (0,16s) a revelação sai de
        // ~0 para 1. Se `seconds` fosse alimentado com milissegundos, aqui o
        // valor ficaria abaixo de 0,01.
        let ctx = Context::default();
        assert_eq!(reveal_frame(&ctx, "rate", false, 0.0), 0.0);
        let start = reveal_frame(&ctx, "rate", true, 0.001);
        assert!(start < 0.2, "começou alto demais: {start}");
        let mut value = start;
        for step in 1..=8 {
            value = reveal_frame(&ctx, "rate", true, 0.001 + step as f64 * 0.02);
        }
        assert!(
            value > 0.9,
            "revelação lenta demais: {value} (unidade errada?)"
        );
    }

    #[test]
    fn default_easing_is_ease_out() {
        assert_eq!(Easing::default(), Easing::EaseOut);
        assert_eq!(PetuniaMotion::EASING, Easing::EaseOut);
    }

    #[test]
    fn every_curve_maps_zero_to_zero_and_one_to_one() {
        for easing in [
            Easing::EaseOut,
            Easing::EaseIn,
            Easing::EaseInOut,
            Easing::Linear,
        ] {
            let curve = easing.curve();
            assert!(curve(0.0).abs() < 1e-4, "{easing:?} não começa em 0");
            assert!(
                (curve(1.0) - 1.0).abs() < 1e-4,
                "{easing:?} não termina em 1"
            );
        }
    }

    #[test]
    fn curves_are_monotonic_in_the_unit_interval() {
        for easing in [
            Easing::EaseOut,
            Easing::EaseIn,
            Easing::EaseInOut,
            Easing::Linear,
        ] {
            let curve = easing.curve();
            let mut previous = curve(0.0);
            for step in 1..=40 {
                let x = step as f32 / 40.0;
                let y = curve(x);
                assert!(
                    y >= previous - 1e-4,
                    "{easing:?} não é monótona em x={x}: {previous} → {y}"
                );
                assert!(
                    (-1e-3..=1.0 + 1e-3).contains(&y),
                    "{easing:?} saiu de 0..1: {y}"
                );
                previous = y;
            }
        }
    }

    /// Roda um frame do contexto com o tempo dado e devolve o alpha medido.
    fn reveal_frame(ctx: &Context, id: &str, visible: bool, time: f64) -> f32 {
        let mut measured = f32::NAN;
        let raw = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| {
            measured = PetuniaMotion::reveal(ui.ctx(), id, visible);
        });
        output.textures_delta.clear();
        measured
    }

    #[test]
    fn reveal_ramps_up_then_settles_and_ramps_down() {
        let ctx = Context::default();
        // Frame de referência escondido: é ele que dá estado ao egui. Medido —
        // sem um frame anterior em `false`, a primeira revelação já nasce em
        // 1.0 (o egui não tem valor de onde interpolar).
        assert_eq!(reveal_frame(&ctx, "reveal", false, 0.0), 0.0);

        let first = reveal_frame(&ctx, "reveal", true, 0.01);
        assert!(first < 1.0, "a revelação saltou direto para {first}");

        // ~30 frames de 16ms: tempo de sobra para 160ms de motion.
        let mut previous = first;
        let mut settled = first;
        for step in 1..=30 {
            let value = reveal_frame(&ctx, "reveal", true, 0.01 + step as f64 / 60.0);
            assert!(
                value >= previous - 1e-3,
                "revelação andou para trás: {value}"
            );
            previous = value;
            settled = value;
        }
        assert!((settled - 1.0).abs() < 1e-3, "revelação parou em {settled}");

        // Some: volta para 0 e para lá fica.
        let leaving = reveal_frame(&ctx, "reveal", false, 1.05);
        assert!(
            leaving <= 1.0 && leaving > 0.0,
            "alpha de saída inválido: {leaving}"
        );
        let gone = reveal_frame(&ctx, "reveal", false, 2.0);
        assert!((gone - 0.0).abs() < 1e-3, "revelação não zerou: {gone}");
    }

    #[test]
    fn reveal_falls_back_to_a_step_when_motion_is_off() {
        let ctx = Context::default();
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        assert_eq!(
            reveal_frame(&ctx, "reveal-off", true, 0.0),
            1.0,
            "sem motion o estado final é imediato"
        );
        assert_eq!(reveal_frame(&ctx, "reveal-off", false, 0.1), 0.0);
    }

    #[test]
    fn a_closed_section_with_motion_off_draws_nothing() {
        let ctx = Context::default();
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        let mut drawn = 0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            PetuniaMotion::section(ui, "section-off", false, |ui| {
                drawn += 1;
                ui.label("conteúdo");
            });
        });
        output.textures_delta.clear();
        assert_eq!(drawn, 0, "seção fechada não desenha conteúdo sem motion");
    }

    #[test]
    fn a_section_animates_open_and_draws_its_content() {
        let ctx = Context::default();
        let mut drawn = 0;
        let mut height = 0.0;
        for step in 0..4 {
            let raw = egui::RawInput {
                time: Some(step as f64 * 0.05),
                ..Default::default()
            };
            let mut output = ctx.run_ui(raw, |ui| {
                let before = ui.cursor().min.y;
                PetuniaMotion::section(ui, "section", true, |ui| {
                    drawn += 1;
                    ui.label("conteúdo");
                });
                height = ui.cursor().min.y - before;
            });
            output.textures_delta.clear();
        }
        assert_eq!(drawn, 4, "o conteúdo é desenhado a cada frame da abertura");
        assert!(height >= 0.0, "altura inválida: {height}");
    }
}
