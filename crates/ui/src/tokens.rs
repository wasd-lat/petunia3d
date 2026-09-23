//! Design tokens e paleta semântica canônica para o Petunia3D.
//! Baseado na Golden Reference visual (`Blender.svg`) e sistema de design desktop profissional.

use egui::{Color32, CornerRadius, Stroke};

// ---------------------------------------------------------------- Cores de Fundo
pub const BG_APP: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
pub const BG_HEADER: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
pub const BG_PANEL_HEADER: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x2d, 0x2d, 0x2d);
pub const BG_SURFACE_HOVER: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
pub const BG_SURFACE_ACTIVE: Color32 = Color32::from_rgb(0x39, 0x2e, 0x49);
pub const BG_INPUT: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
pub const BG_DROPDOWN: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
pub const BG_SHELF: Color32 = Color32::from_rgba_premultiplied(0x1e, 0x1e, 0x1e, 0xf0);

// ------------------------------------------------------------- Cores de Destaque
/// Accent Petunia canônico. `ACCENT_BLUE` permanece como alias legado porque
/// o tema externo ainda serializa o token como `accent_blue`.
pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(0xb5, 0x8c, 0xff);
pub const ACCENT_PRIMARY_HOVER: Color32 = Color32::from_rgb(0xc9, 0xae, 0xff);
pub const ACCENT_FOCUS: Color32 = Color32::from_rgb(0xb5, 0x8c, 0xff);
pub const ACCENT_BLUE: Color32 = ACCENT_PRIMARY;
pub const ACCENT_BLUE_HOVER: Color32 = ACCENT_PRIMARY_HOVER;
pub const ACCENT_BORDER: Color32 = ACCENT_PRIMARY;
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(0x2e, 0xcc, 0x71);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(0xf3, 0x9c, 0x12);
pub const STATUS_INFO: Color32 = Color32::from_rgb(0x38, 0xbd, 0xf8);
/// Semântica de **erro/validação** (Wave 7 — §51): campo inválido, não
/// destrutivo. Antes era literal em cada tela de erro.
pub const ACCENT_ERROR: Color32 = Color32::from_rgb(0xef, 0x53, 0x50);
/// Fundo suave do mesmo erro (bloco de mensagens inline).
pub const ACCENT_ERROR_SOFT: Color32 = Color32::from_rgba_premultiplied(0x18, 0x08, 0x08, 0x40);

// ------------------------------------------------------------- Cores dos Modos
pub const MODE_OBJECT: Color32 = Color32::from_rgb(0x34, 0x98, 0xdb);
pub const MODE_EDIT: Color32 = Color32::from_rgb(0xe6, 0x7e, 0x22);
pub const MODE_PAINT: Color32 = Color32::from_rgb(0x2e, 0xcc, 0x71);

// -------------------------------------------------------------- Cores dos Eixos
pub const AXIS_X: Color32 = Color32::from_rgb(0xe0, 0x3c, 0x42);
pub const AXIS_Y: Color32 = Color32::from_rgb(0x62, 0xc9, 0x34);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x31, 0x82, 0xf6);

// ---------------------------------------------------------------- Cores de Texto
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xde, 0xde, 0xde);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x9c, 0x9c, 0x9c);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x62, 0x62, 0x62);
pub const TEXT_ACTIVE: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);

// ---------------------------------------------------------------- Cores de Borda
pub const BORDER_DARK: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x2f, 0x2f, 0x2f);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(0x3e, 0x3e, 0x3e);

// ----------------------------------------------------------------- Dimensões (px)
pub const TOP_HEADER_HEIGHT: f32 = 28.0;
pub const TOP_HEADER_MAX_HEIGHT: f32 = 72.0;
pub const VIEWPORT_BAR_HEIGHT: f32 = 26.0;
pub const VIEWPORT_BAR_MAX_HEIGHT: f32 = 56.0;
/// Largura de um botão de ferramenta no modo compacto (ícone).
pub const TOOLBAR_WIDTH: f32 = 40.0;
/// Vão entre dois controles que dividem a linha da paleta (botão + seta).
pub const TOOLBAR_CONTROL_GAP: f32 = 2.0;
/// Largura reservada à seta do menu de um grupo (split button) da paleta.
///
/// Medido no tema padrão: o botão da seta ocupa 21.4px. O token declara 22px
/// porque um piso precisa de folga — declarar **menos** que o medido é o que
/// produz o controle cortado fora do paine.
pub const TOOLBAR_CHEVRON_WIDTH: f32 = 22.0;
/// Perda de largura entre a coluna de ferramentas e a célula da paleta.
///
/// São os dois respiros da moldura do paine (4 + 4) e o arredondamento do motor
/// de layout. Medido: coluna de 74px ⇒ célula de 64px (= piso da linha de
/// grupo). É o único jeito de o mínimo da coluna falar a língua da célula.
pub const TOOLBAR_CHROME_WIDTH: f32 = 10.0;
/// Menor largura em que a coluna de ferramentas é utilizável.
///
/// Derivada, não escolhida: é a linha de um **grupo** (botão compacto + vão +
/// seta) mais a moldura. Com a coluna mais estreita que isto a seta do menu da
/// família (Seleção/Transformação) sai do paine e o menu fica inalcançável pelo
/// mouse — era exatamente o que acontecia com o piso herdado de 48px, que só
/// cabia no ícone.
pub const TOOLBAR_MIN_WIDTH: f32 =
    TOOLBAR_WIDTH + TOOLBAR_CONTROL_GAP + TOOLBAR_CHEVRON_WIDTH + TOOLBAR_CHROME_WIDTH;
/// Respiro lateral do conteúdo da paleta (moldura do paine).
pub const TOOLBAR_PANEL_MARGIN: f32 = 4.0;
pub const TOOLBAR_MAX_WIDTH: f32 = 240.0;
pub const STATUS_BAR_HEIGHT: f32 = 24.0;
pub const TIMELINE_HEIGHT: f32 = 56.0;
pub const PROPERTIES_DEFAULT_WIDTH: f32 = 290.0;
pub const OUTLINER_DEFAULT_HEIGHT: f32 = 230.0;
/// Lado de um botão de cor da paleta (alvo de clique visível a 1x).
pub const SWATCH_SIZE: f32 = 18.0;
/// Faixa de largura do pano da tela 2D no centro do workspace PAINT.
///
/// O piso é derivado do conteúdo mínimo (a tela precisa caber com o xadrez e a
/// moldura) e o teto impede a tela de engolir a viewport — a divisória entre as
/// duas é arrastável e mora na memória do motor de painéis (por id).
pub const PAINT_CANVAS_DEFAULT_WIDTH: f32 = 420.0;
pub const PAINT_CANVAS_MIN_WIDTH: f32 = 220.0;
pub const PAINT_CANVAS_MAX_WIDTH: f32 = 1200.0;

// -------------------------------------------------------------- Raios de Cantos
pub const RADIUS_SEGMENT: CornerRadius = CornerRadius::same(5);
pub const RADIUS_PANEL: CornerRadius = CornerRadius::same(8);
pub const RADIUS_WINDOW: CornerRadius = CornerRadius::same(10);
/// egui limita o raio efetivo ao retângulo, então `u8::MAX` produz uma pílula
/// completa para qualquer altura sem escolher um raio por tamanho.
pub const RADIUS_PILL: CornerRadius = CornerRadius::same(u8::MAX);
pub const RADIUS_CONTAINER: CornerRadius = CornerRadius::same(4);
pub const RADIUS_CONTROL: CornerRadius = CornerRadius::same(4);
pub const RADIUS_SMALL: CornerRadius = CornerRadius::same(2);

// --------------------------------------------------------------- Traços (Strokes)
pub fn stroke_subtle() -> Stroke {
    Stroke::new(1.0_f32, BORDER_SUBTLE)
}

pub fn stroke_border() -> Stroke {
    Stroke::new(1.0_f32, BORDER_DARK)
}

pub fn stroke_focus(ctx: &egui::Context) -> Stroke {
    let color = ctx
        .data(|data| data.get_temp::<Color32>(egui::Id::new("petunia_focus_color")))
        .unwrap_or(ACCENT_FOCUS);
    Stroke::new(1.5_f32, color)
}

/// Surface de seleção do tema atualmente aplicado ao contexto egui.
pub fn bg_surface_active_for(ctx: &egui::Context) -> Color32 {
    ctx.data(|data| data.get_temp::<Color32>(egui::Id::new("petunia_active_surface_color")))
        .unwrap_or(BG_SURFACE_ACTIVE)
}

pub use petunia_config::ThemeToken;

/// Converte um ColorRgba do petunia_config para egui::Color32.
pub fn rgba_to_color32(c: petunia_config::ColorRgba) -> Color32 {
    let [r, g, b, a] = c.to_rgba_u8();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Obtém dinamicamente a cor correspondente a um ThemeToken para o tema ativo no AppState.
pub fn color(state: &petunia_core::AppState, token: ThemeToken) -> Color32 {
    let registry = petunia_config::ThemeRegistry::global();
    if let Some(theme) = registry.get_theme(&state.ui.active_theme_id) {
        rgba_to_color32(theme.colors.get_token_color(token))
    } else {
        match token {
            ThemeToken::BgCanvas => BG_APP,
            ThemeToken::BgHeader => BG_HEADER,
            ThemeToken::BgPanel => BG_PANEL,
            ThemeToken::BgPanelHeader => BG_PANEL_HEADER,
            ThemeToken::BgSurface => BG_SURFACE,
            ThemeToken::BgSurfaceHover => BG_SURFACE_HOVER,
            ThemeToken::BgSurfaceActive => BG_SURFACE_ACTIVE,
            ThemeToken::TextPrimary => TEXT_PRIMARY,
            ThemeToken::TextSecondary => TEXT_SECONDARY,
            ThemeToken::TextMuted => TEXT_MUTED,
            ThemeToken::TextActive => TEXT_ACTIVE,
            ThemeToken::AccentBlue => ACCENT_BLUE,
            ThemeToken::AccentOrange => MODE_EDIT,
            ThemeToken::AccentHover => ACCENT_BLUE_HOVER,
            ThemeToken::AccentBorder => ACCENT_BORDER,
            ThemeToken::BorderSubtle => BORDER_SUBTLE,
            ThemeToken::BorderStrong => BORDER_LIGHT,
            ThemeToken::BorderFocus => ACCENT_FOCUS,
            ThemeToken::StatusInfo => STATUS_INFO,
            ThemeToken::StatusWarning => ACCENT_AMBER,
            ThemeToken::StatusError => ACCENT_ERROR,
            ThemeToken::StatusSuccess => ACCENT_GREEN,
        }
    }
}

pub fn bg_canvas(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgCanvas)
}

pub fn bg_header(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgHeader)
}

pub fn bg_panel(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgPanel)
}

pub fn bg_panel_header(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgPanelHeader)
}

pub fn bg_surface(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgSurface)
}

pub fn bg_surface_hover(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgSurfaceHover)
}

pub fn bg_surface_active(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BgSurfaceActive)
}

pub fn text_primary(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::TextPrimary)
}

pub fn text_secondary(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::TextSecondary)
}

pub fn text_muted(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::TextMuted)
}

pub fn text_active(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::TextActive)
}

pub fn border_subtle(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BorderSubtle)
}

pub fn border_strong(state: &petunia_core::AppState) -> Color32 {
    color(state, ThemeToken::BorderStrong)
}

pub fn stroke_border_dyn(state: &petunia_core::AppState) -> Stroke {
    Stroke::new(1.0, border_subtle(state))
}

/// Aplica as configurações do tema do Petunia3D ao contexto egui.
pub fn apply_theme_to_egui(theme: &petunia_config::Theme, ctx: &egui::Context) {
    let is_light = theme
        .manifest
        .as_ref()
        .map(|m| m.id.contains("light"))
        .unwrap_or(false);
    let mut v = if is_light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };

    let get_c = |token: ThemeToken| rgba_to_color32(theme.colors.get_token_color(token));

    let bg = get_c(ThemeToken::BgCanvas);
    let panel = get_c(ThemeToken::BgPanel);
    let text = get_c(ThemeToken::TextPrimary);
    let text_muted = get_c(ThemeToken::TextMuted);
    let accent = get_c(ThemeToken::AccentBlue);
    let focus = get_c(ThemeToken::BorderFocus);
    let border = get_c(ThemeToken::BorderSubtle);
    let control = get_c(ThemeToken::BgSurface);
    let hover = get_c(ThemeToken::BgSurfaceHover);
    let selection = get_c(ThemeToken::BgSurfaceActive);

    v.extreme_bg_color = bg;
    v.text_edit_bg_color = Some(control);
    v.panel_fill = panel;
    v.window_fill = panel;
    v.faint_bg_color = control;
    v.weak_text_color = Some(text_muted);
    v.selection.bg_fill = selection;
    v.selection.stroke = Stroke::new(1.0_f32, text);
    v.hyperlink_color = accent;
    v.window_stroke = Stroke::new(1.0_f32, border);
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("petunia_focus_color"), focus);
        data.insert_temp(egui::Id::new("petunia_active_surface_color"), selection);
    });

    for widget in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        widget.fg_stroke = Stroke::new(1.5_f32, text);
        widget.bg_stroke = Stroke::new(1.0_f32, border);
        widget.corner_radius = RADIUS_SEGMENT;
        widget.expansion = 0.0;
    }

    v.widgets.noninteractive.bg_fill = panel;
    v.widgets.noninteractive.weak_bg_fill = panel;
    v.widgets.inactive.bg_fill = control;
    v.widgets.inactive.weak_bg_fill = control;
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent);
    v.widgets.active.bg_fill = selection;
    v.widgets.active.weak_bg_fill = selection;
    v.widgets.active.bg_stroke = Stroke::new(1.5_f32, accent);
    v.widgets.open = v.widgets.active;

    ctx.set_visuals(v);

    let size = if theme.font.size.is_finite() {
        theme.font.size.clamp(12.0, 20.0)
    } else {
        14.0
    };
    let family = if theme.font.family == "monospace" {
        egui::FontFamily::Monospace
    } else {
        egui::FontFamily::Proportional
    };

    let apply_style = |style: &mut egui::Style| {
        for (name, points) in [
            (egui::TextStyle::Body, size),
            (egui::TextStyle::Button, size),
            (egui::TextStyle::Small, (size - 2.0).max(11.0)),
            (egui::TextStyle::Heading, size + 4.0),
        ] {
            style
                .text_styles
                .insert(name, egui::FontId::new(points, family.clone()));
        }
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace((size - 1.0).max(11.0)),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 5.0);
        style.spacing.interact_size.y = 28.0;
    };
    ctx.style_mut_of(egui::Theme::Dark, apply_style);
    ctx.style_mut_of(egui::Theme::Light, apply_style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petunia_dark_accent_matches_the_floral_baseline() {
        assert_eq!(ACCENT_PRIMARY, Color32::from_rgb(0xb5, 0x8c, 0xff));
        assert_eq!(ACCENT_BLUE, ACCENT_PRIMARY);
        assert_eq!(BG_SURFACE_ACTIVE, Color32::from_rgb(0x39, 0x2e, 0x49));
        assert!(contrast_ratio(TEXT_ACTIVE, BG_SURFACE_ACTIVE) >= 4.5);
        assert!(contrast_ratio(ACCENT_PRIMARY, BG_PANEL) >= 4.5);
    }

    #[test]
    fn focus_stroke_reads_the_active_theme_focus_token() {
        let ctx = egui::Context::default();
        let mut theme = petunia_config::Theme::default();
        theme.colors.border_focus = "#FFD400".into();
        theme.colors.bg_surface_active = "#0057B8".into();

        apply_theme_to_egui(&theme, &ctx);

        assert_eq!(
            stroke_focus(&ctx).color,
            Color32::from_rgb(0xff, 0xd4, 0x00)
        );
        assert_eq!(
            bg_surface_active_for(&ctx),
            Color32::from_rgb(0x00, 0x57, 0xb8)
        );
    }

    fn contrast_ratio(foreground: Color32, background: Color32) -> f32 {
        fn luminance(color: Color32) -> f32 {
            let [r, g, b, _] = color.to_array();
            let linear = [r, g, b].map(|channel| {
                let value = channel as f32 / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            });
            0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
        }

        let (lighter, darker) = {
            let foreground = luminance(foreground);
            let background = luminance(background);
            if foreground >= background {
                (foreground, background)
            } else {
                (background, foreground)
            }
        };
        (lighter + 0.05) / (darker + 0.05)
    }
}
