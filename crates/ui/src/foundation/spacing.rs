//! Gaps estruturais do shell e entre controles.
//!
//! Os nomes são **semânticos de propósito**: `GUTTER` e `RELATED` já são o
//! contrato do capítulo 36 do Livro Vivo (gutter estrutural de 8px; 4px apenas
//! entre controles intimamente relacionados). Não é uma escala numérica nova
//! competindo com a escala do adapter Twill — é o vocabulário de espaçamento que
//! o product code pode citar.
//!
//! Todo valor aqui foi extraído de uso real no código (`ui.add_space(n)`) para
//! não introduzir um segundo padrão de layout.

/// Espaço entre um controle e seu rótulo imediato.
pub const TIGHT: f32 = 2.0;

/// Espaço entre controles intimamente relacionados (mesmo grupo visual).
pub const RELATED: f32 = 4.0;

/// Espaço interno padrão de um controle composto.
pub const CONTROL: f32 = 6.0;

/// **Gutter estrutural** entre regiões do shell (cap. 36: `8`).
pub const GUTTER: f32 = 8.0;

/// Espaço entre grupos distintos dentro de um painel.
pub const GROUP: f32 = 10.0;

/// Espaço amplo entre grupos relacionados.
pub const LARGE: f32 = 12.0;

/// Espaço entre seções de um painel.
pub const SECTION: f32 = 16.0;

/// Espaço excepcional entre regiões de conteúdo.
pub const EXTRA_LARGE: f32 = 20.0;

/// Coleção nomeada, usada por consumidores que querem iterar/documentar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetuniaSpacing;

impl PetuniaSpacing {
    pub const TIGHT: f32 = TIGHT;
    pub const RELATED: f32 = RELATED;
    pub const CONTROL: f32 = CONTROL;
    pub const GUTTER: f32 = GUTTER;
    pub const GROUP: f32 = GROUP;
    pub const LARGE: f32 = LARGE;
    pub const SECTION: f32 = SECTION;
    pub const EXTRA_LARGE: f32 = EXTRA_LARGE;

    /// Escala completa do capítulo 24, do mais apertado ao mais amplo.
    pub const ALL: [f32; 8] = [
        TIGHT,
        RELATED,
        CONTROL,
        GUTTER,
        GROUP,
        LARGE,
        SECTION,
        EXTRA_LARGE,
    ];
}

/// Aplica um espaço vertical nomeado.
///
/// Existe para que product code nunca escreva `ui.add_space(4.0)` solto — o
/// guard `ui-guard` mede esse padrão.
pub fn vspace(ui: &mut egui::Ui, gap: f32) {
    ui.add_space(gap);
}

/// Aplica um espaço horizontal nomeado.
pub fn hspace(ui: &mut egui::Ui, gap: f32) {
    ui.add_space(gap);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gutter_matches_frozen_baseline() {
        // Contrato do capítulo 36: gutter estrutural de 8 logical px.
        assert_eq!(PetuniaSpacing::GUTTER, 8.0);
        assert_eq!(PetuniaSpacing::RELATED, 4.0);
    }

    #[test]
    fn scale_is_monotonic() {
        let all = PetuniaSpacing::ALL;
        for pair in all.windows(2) {
            assert!(pair[0] < pair[1], "escala de spacing precisa ser crescente");
        }
    }

    #[test]
    fn scale_matches_the_frozen_design_system() {
        assert_eq!(
            PetuniaSpacing::ALL,
            [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0]
        );
    }

    #[test]
    fn tight_is_the_smallest_gap_and_related_is_the_smallest_between_controls() {
        let [tight, related, ..] = PetuniaSpacing::ALL;
        assert_eq!(tight, TIGHT);
        assert_eq!(related, RELATED);
        assert!(
            tight < related,
            "TIGHT é rótulo↔controle; RELATED é entre controles"
        );
    }
}
