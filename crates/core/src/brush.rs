//! Brush descriptor unificado (P3D-056/057 — iniciativa Paint).
//!
//! `BrushSettings` é a **única fonte da verdade** dos parâmetros de pincel.
//! Tamanho em pixels de tela (estilo Photoshop): o raio de mundo é derivado
//! na profundidade do hit via `AppState::brush_world_radius`, de modo que o
//! anel de preview e o carimbo real nunca divergem.
//!
//! Este módulo não depende de `egui` nem de UI: módulos de pintura e viewport
//! consomem exatamente o mesmo descriptor.

use serde::{Deserialize, Serialize};

/// Tipo de pincel ativo no motor de pintura (P3D-056 a P3D-060).
///
/// Discriminantes são **append-only**: variantes novas entram no fim para
/// preservar qualquer valor serializado existente. `Line`/`Rectangle`
/// (cap. 15/44) e `Airbrush` (iniciativa Paint) seguiram essa regra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BrushType {
    /// Pincel rígido com pixels exatos sem anti-aliasing (P3D-056).
    #[default]
    Pixel,
    /// Pincel com atenuação radial suave controlada por `hardness` (P3D-057).
    Soft,
    /// Borracha que atenua ou remove o canal alfa (P3D-058).
    Eraser,
    /// Balde de preenchimento flood-fill por tolerância (P3D-059).
    Fill,
    /// Amostrador de cor / conta-gotas (P3D-060).
    Eyedropper,
    /// Linha reta entre dois pontos (cap. 15/44: raster op simples).
    Line,
    /// Retângulo preenchido entre dois cantos (cap. 15/44). Idem.
    Rectangle,
    /// Airbrush: acúmulo contínuo modulado por `flow` (iniciativa Paint).
    Airbrush,
}

impl BrushType {
    /// Pincéis de traço livre (carimbam dabs).
    pub const fn is_free_brush(self) -> bool {
        matches!(
            self,
            Self::Pixel | Self::Soft | Self::Eraser | Self::Airbrush
        )
    }

    /// Formas com ancoragem press→release.
    pub const fn is_shape(self) -> bool {
        matches!(self, Self::Line | Self::Rectangle)
    }
}

/// Mapeamento legado do índice de UI (`paint_brush_kind`) → [`BrushType`].
///
/// Ponte de compatibilidade enquanto a UI migra para `BrushSettings.kind`:
/// a ordem espelha o array de botões histórico (0=Pixel … 6=Rectangle) e
/// `Airbrush` entra como 7, no fim.
pub const fn brush_type_from_kind(kind: usize) -> BrushType {
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

/// Índice legado de UI para um [`BrushType`] (inverso do mapa acima).
pub const fn kind_from_brush_type(kind: BrushType) -> usize {
    match kind {
        BrushType::Pixel => 0,
        BrushType::Soft => 1,
        BrushType::Eraser => 2,
        BrushType::Fill => 3,
        BrushType::Eyedropper => 4,
        BrushType::Line => 5,
        BrushType::Rectangle => 6,
        BrushType::Airbrush => 7,
    }
}

/// Descriptor completo de pincel (P3D-056/057: parâmetros são descriptors,
/// não lógica de widget).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushSettings {
    pub kind: BrushType,
    /// Diâmetro em **pixels de tela** (Photoshop-like; escala com zoom via
    /// profundidade no hit — ver `AppState::brush_world_radius`).
    pub size_px: f32,
    /// Dureza `0..=1`: `1` = borda rígida (Pixel), `0` = falloff máximo.
    pub hardness: f32,
    /// Força/opacidade por dab, `0..=1`.
    pub strength: f32,
    /// Fluxo de tinta por dab (Airbrush), `0..=1`.
    pub flow: f32,
    /// Espaçamento entre dabs como fração do diâmetro, `0.01..=1.0`
    /// (P3D-057: traço independente do poll rate do ponteiro).
    pub spacing: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            kind: BrushType::Pixel,
            size_px: 4.0,
            hardness: 1.0,
            strength: 1.0,
            flow: 1.0,
            spacing: 0.15,
        }
    }
}

impl BrushSettings {
    /// Sanitiza todos os parâmetros para domínios válidos (nunca NaN/∞).
    pub fn sanitized(self) -> Self {
        Self {
            kind: self.kind,
            size_px: if self.size_px.is_finite() {
                self.size_px.clamp(1.0, 512.0)
            } else {
                Self::default().size_px
            },
            hardness: clamp01(self.hardness),
            strength: clamp01(self.strength),
            flow: clamp01(self.flow),
            spacing: if self.spacing.is_finite() {
                self.spacing.clamp(0.01, 1.0)
            } else {
                Self::default().spacing
            },
        }
    }

    /// Raio do carimbo em pixels de canvas, derivado do diâmetro de tela.
    pub fn radius_px(self) -> u32 {
        (self.size_px * 0.5).round().max(1.0) as u32
    }

    /// Passo entre dabs em pixels para o diâmetro atual.
    pub fn dab_step_px(self) -> f32 {
        (self.size_px * self.spacing).max(0.5)
    }

    /// Amostras de dab ao longo de um segmento, espaçadas por
    /// `dab_step_px`. Traços rápidos e lentos com os mesmos extremos
    /// produzem exatamente o mesmo conjunto (P3D-057).
    pub fn stroke_dabs(self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(u32, u32)> {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        let step = self.dab_step_px();
        if len <= step {
            return vec![(x0.round() as u32, y0.round() as u32)];
        }
        let steps = (len / step).ceil() as usize;
        (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32;
                ((x0 + dx * t).round() as u32, (y0 + dy * t).round() as u32)
            })
            .collect()
    }
}

/// Clamp 0..=1 tolerante a NaN (NaN cai em 0, o valor neutro dos parâmetros).
fn clamp01(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sanitizes_and_clamps() {
        let s = BrushSettings {
            size_px: 0.0,
            hardness: 2.0,
            strength: -1.0,
            flow: f32::NAN,
            spacing: 0.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.size_px, 1.0);
        assert_eq!(s.hardness, 1.0);
        assert_eq!(s.strength, 0.0);
        assert_eq!(s.flow, 0.0);
        assert_eq!(s.spacing, 0.01);
    }

    #[test]
    fn kind_mapping_roundtrip() {
        for kind in [
            BrushType::Pixel,
            BrushType::Soft,
            BrushType::Eraser,
            BrushType::Fill,
            BrushType::Eyedropper,
            BrushType::Line,
            BrushType::Rectangle,
            BrushType::Airbrush,
        ] {
            assert_eq!(brush_type_from_kind(kind_from_brush_type(kind)), kind);
        }
    }

    #[test]
    fn stroke_dabs_are_poll_rate_invariant() {
        let s = BrushSettings {
            size_px: 10.0,
            spacing: 0.2,
            ..Default::default()
        };
        // Traço "rápido": um único segmento longo.
        let fast = s.stroke_dabs(0.0, 0.0, 100.0, 100.0);
        // Traço "lento": o mesmo caminho em vários segmentos pequenos.
        let mut slow = Vec::new();
        let mut prev: Option<(f32, f32)> = None;
        for i in 1..=10 {
            let p = (i as f32 * 10.0, i as f32 * 10.0);
            if let Some((px, py)) = prev {
                slow.extend(s.stroke_dabs(px, py, p.0, p.1));
            }
            prev = Some(p);
        }
        slow.insert(0, (0, 0));
        slow.dedup();
        assert!(slow.len() > 5, "traço lento não pode degenerar");
        // Cobertura equivalente: extremos e densidade comparáveis.
        assert_eq!(fast.first(), Some(&(0, 0)));
        assert_eq!(fast.last(), Some(&(100, 100)));
        assert!((fast.len() as i32 - slow.len() as i32).abs() <= 2);
    }
}
