//! Papéis tipográficos.
//!
//! Papéis tipográficos seguem a escala congelada do capítulo 36. O tamanho
//! global configurável do tema continua independente desta escala de papéis.

/// Tamanho do papel UI Default.
pub const BASE: f32 = 12.0;

/// Papel tipográfico. O valor é o tamanho em logical px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextRole {
    /// Rótulos em pílulas e chips de workspace.
    Pill,
    /// Corpo padrão da UI: rótulos, itens de menu, dicas.
    Label,
    /// Texto de campos, valores numéricos e conteúdo de tabela.
    Field,
    /// Título de seção dentro de um painel.
    SectionTitle,
    /// Cabeçalho de painel / título de modal.
    Heading,
    /// Legenda secundária (metadados, contadores).
    Caption,
    /// Micro-texto (badges, telemetria da status bar).
    Micro,
}

impl TextRole {
    /// Tamanho em logical px.
    pub const fn size(self) -> f32 {
        match self {
            TextRole::Pill => 11.0,
            TextRole::Label => 12.0,
            TextRole::Field => 12.0,
            TextRole::SectionTitle => 13.0,
            TextRole::Heading => 14.0,
            TextRole::Caption => 10.0,
            TextRole::Micro => 10.0,
        }
    }

    /// Todos os papéis, do menor ao maior.
    pub const ALL: [TextRole; 7] = [
        TextRole::Caption,
        TextRole::Micro,
        TextRole::Pill,
        TextRole::Label,
        TextRole::Field,
        TextRole::SectionTitle,
        TextRole::Heading,
    ];
}

/// Fonte proporcional de um papel.
pub fn font(role: TextRole) -> egui::FontId {
    egui::FontId::proportional(role.size())
}

/// `RichText` de um papel, ainda sem cor (o chamador escolhe o token).
pub fn rich(text: impl Into<String>, role: TextRole) -> egui::RichText {
    egui::RichText::new(text).size(role.size())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_roles_match_the_frozen_typography_scale() {
        assert_eq!(BASE, 12.0);
        assert_eq!(TextRole::Caption.size(), 10.0);
        assert_eq!(TextRole::Pill.size(), 11.0);
        assert_eq!(TextRole::Label.size(), 12.0);
        assert_eq!(TextRole::Field.size(), 12.0);
        assert_eq!(TextRole::SectionTitle.size(), 13.0);
        assert_eq!(TextRole::Heading.size(), 14.0);
    }

    #[test]
    fn roles_fit_within_the_exceptional_size() {
        assert_eq!(TextRole::ALL[0], TextRole::Caption);
        for role in TextRole::ALL {
            assert!(role.size() <= 14.0, "{role:?} excede o papel excepcional");
        }
    }

    #[test]
    fn font_helper_matches_role_size() {
        assert_eq!(font(TextRole::Field).size, TextRole::Field.size());
    }
}
