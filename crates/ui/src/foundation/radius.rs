//! Raios canônicos de canto.
//!
//! Reexporta os valores de [`crate::tokens`] para que componentes e adapters
//! tenham uma casa única de leitura. `tokens` continua sendo a definição física
//! (é de lá que o tema e o Design System são servidos); este módulo é o contrato
//! semântico que o product code cita.

pub use crate::tokens::{
    RADIUS_CONTAINER, RADIUS_CONTROL, RADIUS_PANEL, RADIUS_PILL, RADIUS_SEGMENT, RADIUS_SMALL,
    RADIUS_WINDOW,
};

/// Raio padrão para controles interativos (botões, campos).
pub const CONTROL: egui::CornerRadius = RADIUS_CONTROL;

/// Raio para segmentos de controles agrupados.
pub const SEGMENT: egui::CornerRadius = RADIUS_SEGMENT;

/// Raio padrão para painéis.
pub const PANEL: egui::CornerRadius = RADIUS_PANEL;

/// Raio para janelas e overlays externos.
pub const WINDOW: egui::CornerRadius = RADIUS_WINDOW;

/// Alias de compatibilidade para componentes ainda não migrados.
pub const CONTAINER: egui::CornerRadius = RADIUS_CONTAINER;

/// Raio de pílula (workspace pills, chips, badges).
pub const PILL: egui::CornerRadius = RADIUS_PILL;

/// Raio mínimo (marcadores, faixas finas).
pub const SMALL: egui::CornerRadius = RADIUS_SMALL;

/// Todos os raios canônicos.
pub const ALL: [egui::CornerRadius; 5] = [CONTROL, SEGMENT, PANEL, WINDOW, PILL];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_scale_is_monotonic() {
        for pair in ALL.windows(2) {
            assert!(pair[0].nw <= pair[1].nw, "raio precisa ser crescente");
        }
    }

    #[test]
    fn pill_is_the_largest_radius() {
        assert_eq!(ALL[ALL.len() - 1], PILL);
        assert_eq!(PILL, egui::CornerRadius::same(u8::MAX));
    }

    #[test]
    fn semantic_aliases_match_tokens() {
        assert_eq!(CONTROL, crate::tokens::RADIUS_CONTROL);
        assert_eq!(CONTAINER, crate::tokens::RADIUS_CONTAINER);
        assert_eq!(PANEL, egui::CornerRadius::same(8));
        assert_eq!(WINDOW, egui::CornerRadius::same(10));
    }
}
