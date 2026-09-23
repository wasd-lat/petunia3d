//! Componentes visuais canônicos do Petunia Design System (`PetuniaWidget`).
//!
//! Encapsula botões de toolbar, tabs de workspaces, tabs verticais de propriedades,
//! cabeçalhos de painel e caixas de busca com estados completos:
//! (Default, Hover, Pressed, Selected, Focused, Disabled).

use egui::{
    Align2, Color32, FontId, Rect, Response, Sense, StrokeKind, TextEdit, Ui, WidgetInfo,
    WidgetType, vec2,
};

use crate::icon_registry::{IconRegistry, PetuniaIcon};
use crate::tokens;

/// Direção da seta vetorial pintada (sem depender de glifo da fonte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChevronDir {
    Up,
    Down,
    Right,
}

/// Pinta uma seta triangular preenchida centrada em `rect`.
///
/// Vetorial de propósito: os glifos `▾`/`›`/`↑`/`↓` renderizam como tofu
/// (quadrado) em pacotes de fonte sem o bloco geométrico, então menus e
/// indicadores usam isto em vez de texto.
pub fn paint_chevron(painter: &egui::Painter, rect: Rect, color: Color32, dir: ChevronDir) {
    let c = rect.center();
    let (w, h) = match dir {
        ChevronDir::Down | ChevronDir::Up => (5.0, 3.5),
        ChevronDir::Right => (3.5, 5.0),
    };
    let points = match dir {
        ChevronDir::Down => vec![
            egui::pos2(c.x - w, c.y - h * 0.6),
            egui::pos2(c.x + w, c.y - h * 0.6),
            egui::pos2(c.x, c.y + h),
        ],
        ChevronDir::Up => vec![
            egui::pos2(c.x - w, c.y + h * 0.6),
            egui::pos2(c.x + w, c.y + h * 0.6),
            egui::pos2(c.x, c.y - h),
        ],
        ChevronDir::Right => vec![
            egui::pos2(c.x - w * 0.6, c.y - h),
            egui::pos2(c.x + w, c.y),
            egui::pos2(c.x - w * 0.6, c.y + h),
        ],
    };
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

/// Pinta um "check" vetorial (visto) centrado em `rect`.
pub fn paint_check(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    painter.line_segment(
        [
            egui::pos2(c.x - s, c.y + s * 0.1),
            egui::pos2(c.x - s * 0.1, c.y + s * 0.8),
        ],
        egui::Stroke::new(2.0_f32, color),
    );
    painter.line_segment(
        [
            egui::pos2(c.x - s * 0.1, c.y + s * 0.8),
            egui::pos2(c.x + s, c.y - s * 0.8),
        ],
        egui::Stroke::new(2.0_f32, color),
    );
}

/// Botão chevron de colapso (16px, seta pintada, com foco e teclado).
///
/// Substitui `small_button("+"/"–")` e textos `↑`/`↓` onde o glifo é risco.
pub fn chevron_toggle(ui: &mut Ui, tooltip: &str, open: bool) -> Response {
    chevron_toggle_dir(
        ui,
        tooltip,
        if open {
            ChevronDir::Down
        } else {
            ChevronDir::Right
        },
    )
}

/// Botão de seta pintada em direção explícita (ex. mover ↑/↓ em listas).
pub fn chevron_toggle_dir(ui: &mut Ui, tooltip: &str, dir: ChevronDir) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tooltip));
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
        paint_chevron(painter, rect.shrink(4.0), tokens::TEXT_SECONDARY, dir);
    }
    resp.on_hover_text(tooltip)
}

/// Botão de menu com seta vetorial (`PetuniaMenuButton`).
///
/// Substitui `ui.menu_button("... ▾")`: o rótulo é texto puro e a seta é
/// pintada (nunca tofu). `label = None` rende só a seta (chevron de segmento,
/// ex. Snapping/Overlays). Abre o popup via `egui::Popup::menu`.
pub struct PetuniaMenuButton<'a> {
    pub label: Option<&'a str>,
    pub tooltip: Option<&'a str>,
    pub height: f32,
}

impl<'a> PetuniaMenuButton<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label: Some(label),
            tooltip: None,
            height: 22.0,
        }
    }

    pub fn chevron_only() -> Self {
        Self {
            label: None,
            tooltip: None,
            height: 22.0,
        }
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<egui::InnerResponse<R>> {
        let label_w = self.label.map(|label| {
            ui.fonts_mut(|f| {
                f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), Color32::WHITE)
                    .size()
                    .x
            })
        });
        let chevron_w = 16.0;
        let width = match label_w {
            Some(w) => 8.0 + w + 4.0 + chevron_w,
            None => self.height,
        };
        let (rect, resp) = ui.allocate_exact_size(vec2(width, self.height), Sense::click());
        let acc_label = self.label.unwrap_or("menu");
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, acc_label));

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if resp.hovered() || resp.has_focus() {
                painter.rect_filled(rect, tokens::RADIUS_CONTROL, tokens::BG_SURFACE_HOVER);
            }
            if resp.has_focus() {
                painter.rect_stroke(
                    rect,
                    tokens::RADIUS_CONTROL,
                    tokens::stroke_focus(ui.ctx()),
                    StrokeKind::Inside,
                );
            }
            let fg = tokens::TEXT_PRIMARY;
            match self.label {
                Some(label) => {
                    painter.text(
                        egui::pos2(rect.min.x + 8.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        label,
                        FontId::proportional(12.0),
                        fg,
                    );
                    let chev_rect = Rect::from_center_size(
                        egui::pos2(rect.max.x - chevron_w * 0.5 - 2.0, rect.center().y),
                        vec2(chevron_w, 10.0),
                    );
                    paint_chevron(painter, chev_rect, tokens::TEXT_SECONDARY, ChevronDir::Down);
                }
                None => {
                    paint_chevron(painter, rect.shrink(5.0), fg, ChevronDir::Down);
                }
            }
        }

        let mut resp = resp;
        if let Some(tip) = self.tooltip {
            resp = resp.on_hover_text(tip);
        }
        egui::Popup::menu(&resp).show(add_contents)
    }
}

/// Botão de ferramenta da Toolbar do Petunia3D.
///
/// Suporta modos compacto (ícone centralizado) e expandido (ícone + rótulo com recorte limpo).
pub struct PetuniaToolbarButton<'a> {
    pub icon: PetuniaIcon,
    pub label: &'a str,
    pub selected: bool,
    pub compact: bool,
    pub tooltip: Option<&'a str>,
    /// Largura declarada pelo adapter (§48).
    ///
    /// `None` deixa o botão medir a faixa do `Ui` que o recebe — usado fora da
    /// grade da paleta (vitrine, linhas simples). Dentro da grade o adapter já
    /// resolveu a largura da célula, então o widget **não** mede o container.
    pub width: Option<f32>,
}

impl<'a> PetuniaToolbarButton<'a> {
    pub fn new(icon: PetuniaIcon, label: &'a str) -> Self {
        Self {
            icon,
            label,
            selected: false,
            compact: true,
            tooltip: None,
            width: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Largura resolvida pelo adapter (grade da paleta).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let desired_size = if self.compact {
            vec2(tokens::TOOLBAR_WIDTH, tokens::TOOLBAR_WIDTH)
        } else {
            let width = self.width.unwrap_or_else(|| ui.available_width());
            vec2(width.max(tokens::TOOLBAR_WIDTH), 32.0)
        };

        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                ui.is_enabled(),
                self.selected,
                self.label,
            )
        });

        if ui.is_rect_visible(rect) {
            let (bg_fill, fg_color) = if self.selected {
                (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
            } else if response.hovered() {
                (tokens::BG_SURFACE_HOVER, tokens::TEXT_ACTIVE)
            } else {
                (Color32::TRANSPARENT, tokens::TEXT_SECONDARY)
            };

            let painter = ui.painter().with_clip_rect(rect);

            if bg_fill != Color32::TRANSPARENT {
                painter.rect_filled(
                    rect,
                    crate::adapters::twill_tokens::toolbar_button_radius(),
                    bg_fill,
                );
            }

            // Indicador de foco acessível
            if response.has_focus() {
                painter.rect_stroke(
                    rect,
                    tokens::RADIUS_CONTAINER,
                    tokens::stroke_focus(ui.ctx()),
                    StrokeKind::Inside,
                );
            }

            if self.compact {
                // Ícone de 20x20 perfeitamente centralizado no container de 40x40
                let icon_rect = Rect::from_center_size(rect.center(), vec2(20.0, 20.0));
                IconRegistry::paint(ui.ctx(), &painter, &self.icon, icon_rect, fg_color);
            } else {
                // Ícone de 20x20 alinhado à esquerda + rótulo tipográfico à direita
                let icon_rect = Rect::from_min_size(
                    egui::pos2(rect.min.x + 8.0, rect.min.y + (rect.height() - 20.0) * 0.5),
                    vec2(20.0, 20.0),
                );
                IconRegistry::paint(ui.ctx(), &painter, &self.icon, icon_rect, fg_color);

                let text_pos =
                    egui::pos2(rect.min.x + 36.0, rect.min.y + (rect.height() - 14.0) * 0.5);
                painter.text(
                    text_pos,
                    Align2::LEFT_TOP,
                    self.label,
                    FontId::proportional(13.0),
                    fg_color,
                );
            }
        }

        if let Some(hint) = self.tooltip {
            response.on_hover_text(hint)
        } else {
            response
        }
    }
}

/// Botão de aba vertical de categoria do painel Properties (`PropertyTabButton`).
pub struct PetuniaPropertyTabButton {
    pub icon: PetuniaIcon,
    pub selected: bool,
    pub tooltip: Option<String>,
    pub accent_color: Color32,
}

impl PetuniaPropertyTabButton {
    pub fn new(icon: PetuniaIcon, selected: bool) -> Self {
        Self {
            icon,
            selected,
            tooltip: None,
            accent_color: tokens::ACCENT_BLUE,
        }
    }

    pub fn accent_color(mut self, color: Color32) -> Self {
        self.accent_color = color;
        self
    }

    pub fn tooltip(mut self, hint: impl Into<String>) -> Self {
        self.tooltip = Some(hint.into());
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let desired_size = vec2(32.0, 28.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let (bg_fill, fg_color) = if self.selected {
                (self.accent_color.gamma_multiply(0.25), tokens::TEXT_ACTIVE)
            } else if response.hovered() {
                (tokens::BG_SURFACE_HOVER, tokens::TEXT_PRIMARY)
            } else {
                (Color32::TRANSPARENT, tokens::TEXT_SECONDARY)
            };

            let painter = ui.painter().with_clip_rect(rect);

            if bg_fill != Color32::TRANSPARENT {
                painter.rect_filled(rect, tokens::RADIUS_CONTAINER, bg_fill);
            }

            if self.selected {
                // Indicador inferior elegante para aba ativa
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 4.0, rect.bottom() - 1.5),
                        egui::pos2(rect.right() - 4.0, rect.bottom() - 1.5),
                    ],
                    egui::Stroke::new(2.0_f32, self.accent_color),
                );
                // Borda sutil de destaque
                painter.rect_stroke(
                    rect,
                    tokens::RADIUS_CONTAINER,
                    egui::Stroke::new(1.0_f32, self.accent_color.gamma_multiply(0.5)),
                    StrokeKind::Inside,
                );
            }

            if response.has_focus() {
                painter.rect_stroke(
                    rect,
                    tokens::RADIUS_CONTAINER,
                    tokens::stroke_focus(ui.ctx()),
                    StrokeKind::Inside,
                );
            }

            let icon_rect = Rect::from_center_size(
                rect.center() - vec2(0.0, if self.selected { 1.0 } else { 0.0 }),
                vec2(19.0, 19.0),
            );
            IconRegistry::paint(ui.ctx(), &painter, &self.icon, icon_rect, fg_color);
        }

        if let Some(hint) = self.tooltip {
            response.on_hover_text(hint)
        } else {
            response
        }
    }
}

/// Aba em formato de pílula arredondada do cabeçalho superior (`WorkspacePill`).
pub struct PetuniaWorkspacePill<'a> {
    pub label: &'a str,
    pub selected: bool,
}

impl<'a> PetuniaWorkspacePill<'a> {
    pub fn new(label: &'a str, selected: bool) -> Self {
        Self { label, selected }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (bg, fg) = if self.selected {
            (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
        } else {
            (Color32::TRANSPARENT, tokens::TEXT_SECONDARY)
        };

        let btn = egui::Button::new(
            egui::RichText::new(self.label)
                .size(11.5)
                .color(fg)
                .strong(),
        )
        .fill(bg)
        .corner_radius(tokens::RADIUS_PILL);

        ui.add(btn)
    }
}

/// Campo de busca padronizado com ícone Phosphor de lupa e botão de limpar.
pub fn petunia_search_box(ui: &mut Ui, query: &mut String, hint: &str) -> Response {
    petunia_search_box_impl(ui, query, hint, None)
}

/// Campo de busca padronizado com largura explícita.
pub fn petunia_search_box_with_width(
    ui: &mut Ui,
    query: &mut String,
    hint: &str,
    desired_width: f32,
) -> Response {
    petunia_search_box_impl(ui, query, hint, Some(desired_width))
}

fn petunia_search_box_impl(
    ui: &mut Ui,
    query: &mut String,
    hint: &str,
    fixed_width: Option<f32>,
) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

        let icon_size = vec2(16.0, 16.0);
        let (icon_rect, _) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
        IconRegistry::paint(
            ui.ctx(),
            ui.painter(),
            &PetuniaIcon::Search,
            icon_rect,
            tokens::TEXT_MUTED,
        );

        let clear_width = if query.is_empty() { 0.0 } else { 18.0 };
        let width = fixed_width.unwrap_or_else(|| {
            let avail = ui.available_width() - clear_width - 4.0;
            avail.clamp(60.0, 320.0)
        });

        let edit = TextEdit::singleline(query)
            .hint_text(hint)
            .desired_width(width);

        let resp = ui.add(edit);

        if !query.is_empty() {
            let clear_btn = egui::Button::new("×")
                .frame(false)
                .fill(Color32::TRANSPARENT);
            if ui.add(clear_btn).clicked() {
                query.clear();
            }
        }

        resp
    })
    .inner
}

/// Política de dimensionamento de menus (Wave 4 — §8.1): o popup acompanha o
/// conteúdo dentro de limites sãos, em vez de esticar cada linha à largura
/// disponível.
pub const MENU_MIN_W: f32 = 120.0;
pub const MENU_MAX_W: f32 = 320.0;

/// Largura de linha de menu a partir de galleys reais: padding + ícone/check +
/// rótulo + atalho + indicador de submenu + padding. `leading` = largura do
/// adorno à esquerda após o padding (ícone 22, check/radio 16, nenhum 0).
pub fn menu_row_width(
    ui: &mut Ui,
    leading: f32,
    label: &str,
    shortcut: Option<&str>,
    has_submenu: bool,
) -> f32 {
    let label_w = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), Color32::WHITE)
            .size()
            .x
    });
    let mut width = 8.0 + leading + label_w + 12.0;
    if let Some(sc) = shortcut {
        let sc_w = ui.fonts_mut(|f| {
            f.layout_no_wrap(sc.to_owned(), FontId::monospace(10.5), Color32::WHITE)
                .size()
                .x
        });
        width += sc_w + 8.0;
    }
    if has_submenu {
        width += 10.0;
    }
    width += 8.0;
    let measured = width.clamp(MENU_MIN_W, MENU_MAX_W);
    // Em popup estreito, encolhe até a largura disponível (texto trunca com clip).
    measured.min(ui.available_width().max(MENU_MIN_W))
}

/// Botão somente-ícone com ícone semântico, tooltip e apelido acessível
/// (Wave 6 — §13.2: todo controle só-ícone deriva daqui ou equivalente).
///
/// Produz `widget_info` (nome + papel), anel de foco visível e estados
/// hover/selected/disabled. Tamanho quadrado explícito — sem estimativa.
pub struct PetuniaIconButton<'a> {
    pub icon: PetuniaIcon,
    pub tooltip: &'a str,
    pub size: f32,
    pub selected: bool,
    pub enabled: bool,
}

impl<'a> PetuniaIconButton<'a> {
    pub fn new(icon: PetuniaIcon, tooltip: &'a str, size: f32) -> Self {
        Self {
            icon,
            tooltip,
            size,
            selected: false,
            enabled: true,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            vec2(self.size, self.size),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Button,
                ui.is_enabled() && self.enabled,
                self.selected,
                self.tooltip,
            )
        });

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.selected {
                painter.rect_filled(
                    rect,
                    tokens::RADIUS_CONTROL,
                    tokens::bg_surface_active_for(ui.ctx()),
                );
            } else if self.enabled && response.hovered() {
                painter.rect_filled(rect, tokens::RADIUS_CONTROL, tokens::BG_SURFACE_HOVER);
            }
            if response.has_focus() {
                painter.rect_stroke(
                    rect,
                    tokens::RADIUS_CONTROL,
                    tokens::stroke_focus(ui.ctx()),
                    StrokeKind::Inside,
                );
            }
            let fg = if !self.enabled {
                tokens::TEXT_MUTED
            } else if self.selected {
                tokens::TEXT_ACTIVE
            } else {
                tokens::TEXT_PRIMARY
            };
            let icon_rect =
                Rect::from_center_size(rect.center(), vec2(self.size * 0.7, self.size * 0.7));
            IconRegistry::paint(ui.ctx(), painter, &self.icon, icon_rect, fg);
        }

        response.on_hover_text(self.tooltip)
    }
}

/// Item padronizado de menu suspenso ou popup do Petunia3D (`[Icon] Label ... [Shortcut] + seta`).
/// A seta de submenu é vetorial (`paint_chevron`), nunca glifo de fonte.
pub struct PetuniaMenuItem<'a> {
    pub icon: Option<PetuniaIcon>,
    pub label: &'a str,
    pub shortcut: Option<&'a str>,
    pub has_submenu: bool,
    pub enabled: bool,
}

impl<'a> PetuniaMenuItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            icon: None,
            label,
            shortcut: None,
            has_submenu: false,
            enabled: true,
        }
    }

    pub fn icon(mut self, icon: PetuniaIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: Option<&'a str>) -> Self {
        self.shortcut = shortcut;
        self
    }

    pub fn submenu(mut self, has_submenu: bool) -> Self {
        self.has_submenu = has_submenu;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let width = menu_row_width(
            ui,
            if self.icon.is_some() { 22.0 } else { 0.0 },
            self.label,
            self.shortcut,
            self.has_submenu,
        );
        let desired_size = vec2(width, 22.0);
        let (rect, response) = ui.allocate_exact_size(
            desired_size,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter().with_clip_rect(rect);
            let visuals = ui.visuals();

            // Fundo ao passar o mouse
            if self.enabled && response.hovered() {
                painter.rect_filled(
                    rect,
                    tokens::RADIUS_CONTROL,
                    visuals.widgets.hovered.bg_fill,
                );
            }

            // Cores dinâmicas orientadas pelo tema ativo
            let (fg_color, shortcut_color) = if !self.enabled {
                (
                    visuals.widgets.noninteractive.fg_stroke.color,
                    visuals.text_color().gamma_multiply(0.4),
                )
            } else if response.hovered() {
                (
                    visuals.widgets.hovered.fg_stroke.color,
                    visuals.text_color().gamma_multiply(0.75),
                )
            } else {
                (
                    visuals.text_color(),
                    visuals.text_color().gamma_multiply(0.55),
                )
            };

            // 1. Ícone à esquerda
            let mut text_start_x = rect.min.x + 8.0;
            if let Some(ref icon) = self.icon {
                let icon_rect = Rect::from_min_size(
                    egui::pos2(rect.min.x + 6.0, rect.min.y + (rect.height() - 16.0) * 0.5),
                    vec2(16.0, 16.0),
                );
                IconRegistry::paint(ui.ctx(), &painter, icon, icon_rect, fg_color);
                text_start_x += 22.0;
            }

            // 2. Rótulo textual
            let text_pos = egui::pos2(text_start_x, rect.min.y + (rect.height() - 13.0) * 0.5);
            painter.text(
                text_pos,
                Align2::LEFT_TOP,
                self.label,
                FontId::proportional(12.0),
                fg_color,
            );

            // 3. Seta de submenu ou atalho à direita
            let right_padding = if self.has_submenu { 18.0 } else { 8.0 };
            if let Some(sc) = self.shortcut {
                let sc_pos = egui::pos2(
                    rect.max.x - right_padding,
                    rect.min.y + (rect.height() - 12.0) * 0.5,
                );
                painter.text(
                    sc_pos,
                    Align2::RIGHT_TOP,
                    sc,
                    FontId::monospace(10.5),
                    shortcut_color,
                );
            }

            if self.has_submenu {
                let arrow_rect = Rect::from_center_size(
                    egui::pos2(rect.max.x - 9.0, rect.center().y),
                    vec2(10.0, 12.0),
                );
                paint_chevron(&painter, arrow_rect, shortcut_color, ChevronDir::Right);
            }
        }

        response
    }
}

/// Item de menu com alternância booleana (Checkbox).
pub struct PetuniaMenuCheckboxItem<'a> {
    pub label: &'a str,
    pub checked: bool,
    pub shortcut: Option<&'a str>,
    pub enabled: bool,
}

impl<'a> PetuniaMenuCheckboxItem<'a> {
    pub fn new(label: &'a str, checked: bool) -> Self {
        Self {
            label,
            checked,
            shortcut: None,
            enabled: true,
        }
    }

    pub fn shortcut(mut self, shortcut: Option<&'a str>) -> Self {
        self.shortcut = shortcut;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let width = menu_row_width(ui, 16.0, self.label, self.shortcut, false);
        let desired_size = vec2(width, 22.0);
        let (rect, response) = ui.allocate_exact_size(
            desired_size,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter().with_clip_rect(rect);
            let visuals = ui.visuals();

            if self.enabled && response.hovered() {
                painter.rect_filled(
                    rect,
                    tokens::RADIUS_CONTROL,
                    visuals.widgets.hovered.bg_fill,
                );
            }

            let (fg_color, shortcut_color) = if !self.enabled {
                (
                    visuals.widgets.noninteractive.fg_stroke.color,
                    visuals.text_color().gamma_multiply(0.4),
                )
            } else if response.hovered() {
                (
                    visuals.widgets.hovered.fg_stroke.color,
                    visuals.text_color().gamma_multiply(0.75),
                )
            } else {
                (
                    visuals.text_color(),
                    visuals.text_color().gamma_multiply(0.55),
                )
            };

            // Indicador de Checkmark à esquerda (6.0px margin)
            let check_rect = Rect::from_min_size(
                egui::pos2(rect.min.x + 6.0, rect.min.y + (rect.height() - 14.0) * 0.5),
                vec2(14.0, 14.0),
            );
            if self.checked {
                paint_check(
                    &painter,
                    check_rect,
                    if self.enabled {
                        visuals.selection.stroke.color
                    } else {
                        visuals.widgets.noninteractive.fg_stroke.color
                    },
                );
            }

            // Rótulo
            let text_pos = egui::pos2(rect.min.x + 24.0, rect.min.y + (rect.height() - 13.0) * 0.5);
            painter.text(
                text_pos,
                Align2::LEFT_TOP,
                self.label,
                FontId::proportional(12.0),
                fg_color,
            );

            // Atalho à direita
            if let Some(sc) = self.shortcut {
                let sc_pos =
                    egui::pos2(rect.max.x - 8.0, rect.min.y + (rect.height() - 12.0) * 0.5);
                painter.text(
                    sc_pos,
                    Align2::RIGHT_TOP,
                    sc,
                    FontId::monospace(10.5),
                    shortcut_color,
                );
            }
        }

        response
    }
}

/// Item de menu de seleção exclusiva (Radio).
pub struct PetuniaMenuRadioItem<'a> {
    pub label: &'a str,
    pub selected: bool,
    pub shortcut: Option<&'a str>,
    pub enabled: bool,
}

impl<'a> PetuniaMenuRadioItem<'a> {
    pub fn new(label: &'a str, selected: bool) -> Self {
        Self {
            label,
            selected,
            shortcut: None,
            enabled: true,
        }
    }

    pub fn shortcut(mut self, shortcut: Option<&'a str>) -> Self {
        self.shortcut = shortcut;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let width = menu_row_width(ui, 16.0, self.label, self.shortcut, false);
        let desired_size = vec2(width, 24.0);
        let (rect, response) = ui.allocate_exact_size(
            desired_size,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter().with_clip_rect(rect);

            if self.enabled && response.hovered() {
                painter.rect_filled(rect, tokens::RADIUS_CONTROL, tokens::BG_SURFACE_HOVER);
            }

            let (fg_color, shortcut_color) = if !self.enabled {
                (tokens::TEXT_MUTED, tokens::TEXT_MUTED.gamma_multiply(0.6))
            } else if response.hovered() {
                (tokens::TEXT_ACTIVE, tokens::TEXT_SECONDARY)
            } else {
                (tokens::TEXT_PRIMARY, tokens::TEXT_MUTED)
            };

            // Indicador de Radio à esquerda
            let dot_center = egui::pos2(rect.min.x + 13.0, rect.center().y);
            if self.selected {
                painter.circle_filled(
                    dot_center,
                    3.5,
                    if self.enabled {
                        tokens::ACCENT_BLUE
                    } else {
                        tokens::TEXT_MUTED
                    },
                );
            }

            // Rótulo
            let text_pos = egui::pos2(rect.min.x + 24.0, rect.min.y + (rect.height() - 13.0) * 0.5);
            painter.text(
                text_pos,
                Align2::LEFT_TOP,
                self.label,
                FontId::proportional(12.0),
                fg_color,
            );

            // Atalho à direita
            if let Some(sc) = self.shortcut {
                let sc_pos =
                    egui::pos2(rect.max.x - 8.0, rect.min.y + (rect.height() - 12.0) * 0.5);
                painter.text(
                    sc_pos,
                    Align2::RIGHT_TOP,
                    sc,
                    FontId::monospace(10.5),
                    shortcut_color,
                );
            }
        }

        response
    }
}

/// Separador de menu fino e discreto com margens calibradas.
pub fn petunia_menu_separator(ui: &mut Ui) {
    ui.add_space(2.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            egui::Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
        );
    }
    ui.add_space(2.0);
}

/// Botão de ação padrão para painéis (Properties, Outliner, Modais), suportando ícone opcional e tint de perigo.
pub fn petunia_action_button(
    ui: &mut Ui,
    icon: Option<PetuniaIcon>,
    label: &str,
    danger: bool,
) -> Response {
    let height = 24.0;
    let padding_x = 8.0;
    let icon_size = 14.0;

    // Nunca mais largo que o espaço real: o botão elide o rótulo (`…`) em vez
    // de transbordar — um botão maior que o painel alargava o painel sozinho.
    let avail = ui.available_width().max(48.0);
    let icon_extra = if icon.is_some() { icon_size + 6.0 } else { 0.0 };
    let text_budget = (avail - padding_x * 2.0 - icon_extra).max(16.0);
    let galley = ui.fonts_mut(|f| {
        let mut job = egui::text::LayoutJob {
            wrap: egui::text::TextWrapping {
                max_width: text_budget,
                max_rows: 1,
                break_anywhere: false,
                overflow_character: Some('…'),
            },
            ..Default::default()
        };
        job.append(
            label,
            0.0,
            egui::TextFormat {
                font_id: FontId::proportional(11.5),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        f.layout_job(job)
    });
    let text_w = galley.size().x;
    let width = (padding_x * 2.0 + icon_extra + text_w).min(avail);

    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());

    if ui.is_rect_visible(rect) {
        let (bg, border, fg) = if resp.is_pointer_button_down_on() {
            if danger {
                (
                    Color32::from_rgb(0x80, 0x18, 0x18),
                    Color32::from_rgb(0xc0, 0x39, 0x2b),
                    Color32::WHITE,
                )
            } else {
                (
                    tokens::bg_surface_active_for(ui.ctx()),
                    tokens::ACCENT_BORDER,
                    Color32::WHITE,
                )
            }
        } else if resp.hovered() {
            if danger {
                (
                    Color32::from_rgb(0x5a, 0x18, 0x18),
                    Color32::from_rgb(0xa0, 0x20, 0x20),
                    Color32::from_rgb(0xff, 0x90, 0x90),
                )
            } else {
                (
                    tokens::BG_SURFACE_HOVER,
                    tokens::BORDER_LIGHT,
                    tokens::TEXT_PRIMARY,
                )
            }
        } else if danger {
            (
                tokens::BG_SURFACE,
                tokens::BORDER_SUBTLE,
                Color32::from_rgb(0xe0, 0x60, 0x60),
            )
        } else {
            (
                tokens::BG_SURFACE,
                tokens::BORDER_SUBTLE,
                tokens::TEXT_PRIMARY,
            )
        };

        ui.painter().rect(
            rect,
            tokens::RADIUS_CONTROL,
            bg,
            egui::Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );

        let mut cur_x = rect.left() + padding_x;
        if let Some(ic) = icon {
            let icon_rect = Rect::from_center_size(
                egui::pos2(cur_x + icon_size * 0.5, rect.center().y),
                vec2(icon_size, icon_size),
            );
            IconRegistry::paint(ui.ctx(), ui.painter(), &ic, icon_rect, fg);
            cur_x += icon_size + 6.0;
        }

        ui.painter().galley(
            egui::pos2(cur_x, rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );
    }

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolbar_button_widget_rendering() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let btn = PetuniaToolbarButton::new(PetuniaIcon::Move, "Move")
                    .selected(true)
                    .compact(true)
                    .tooltip("Transladar · G");
                let _resp = btn.show(ui);

                let btn_wide = PetuniaToolbarButton::new(PetuniaIcon::Rotate, "Rotate")
                    .selected(false)
                    .compact(false);
                let _resp_wide = btn_wide.show(ui);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_property_tab_button_widget_rendering() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let tab = PetuniaPropertyTabButton::new(PetuniaIcon::PropRender, true)
                    .tooltip("Propriedades de Render");
                let _resp = tab.show(ui);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_workspace_pill_widget_rendering() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let pill = PetuniaWorkspacePill::new("MODEL", true);
                let _resp = pill.show(ui);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_search_box_widget_rendering() {
        let ctx = egui::Context::default();
        let mut query = String::from("Cube");
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let _resp = petunia_search_box(ui, &mut query, "Search Scene...");
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_menu_item_widget_rendering() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let item = PetuniaMenuItem::new("Extrude")
                    .icon(PetuniaIcon::Extrude)
                    .shortcut(Some("E"));
                let _resp = item.show(ui);
                petunia_menu_separator(ui);
            });
        })
        .textures_delta
        .clear();
    }
}
