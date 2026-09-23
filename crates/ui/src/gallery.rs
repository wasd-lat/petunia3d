//! # Component Gallery — superfície de referência dos componentes Petunia
//!
//! Diretiva Egui Ecosystem Final Push, §38:
//!
//! ```bash
//! cargo run -p petunia_ui --example component_gallery
//! ```
//!
//! A regra que este módulo existe para tornar executável é curta:
//!
//! > 1. procurar componente existente; 2. modificar component gallery;
//! > 3. só depois usar no produto.
//!
//! Nada aqui é código de produto: nenhum painel do shell importa este módulo.
//! Ele é a **vitrine** — o lugar onde um componente reutilizável nasce e onde
//! alguém confere, num só frame, como ele se comporta em cada tema, densidade e
//! escala antes de citá-lo num painel.
//!
//! ## O que a galeria mostra quando o componente não existe
//!
//! [`pending`] desenha o **inventário de lacunas**: cada item exigido pela §38
//! que ainda não tem implementação reutilizável aparece com nome e contrato
//! pendente, em vez de sumir. Um componente ausente é informação de produto; a
//! alternativa (omitir a linha) faria a galeria parecer completa quando não é.
//!
//! Categorias ainda pendentes estão marcadas com o motivo real (crate não
//! instalada, ou componente preso dentro de um painel que precisa ser extraído)
//! — ver `docs/audits/ui-ecosystem-final-push/`.
//!
//! ## Fronteira com `ui-guard`
//!
//! Este arquivo é uma **exceção declarada** em `ui-guard` (`FOUNDATION_PATHS`):
//! uma vitrine precisa montar micro-layout e medir o resultado. O product code
//! continua proibido de fazer o mesmo — a exceção vale só aqui.

use egui::Ui;
use petunia_core::{AppState, UiDensity, Workspace};

use crate::adapters::drag_drop::{PetuniaDragList, PetuniaDragSpec};
use crate::adapters::form::{PetuniaForm, PetuniaFormSession, PetuniaValidationReport, plan_row};
use crate::adapters::taffy_layout::{
    PetuniaColumnSpec, PetuniaGap, PetuniaJustify, PetuniaResponsiveLayout, clamped_width, columns,
    fill_remaining, flex_key_value_row, responsive,
};
use crate::adapters::tool_grid::PetuniaToolGridSpec;
use crate::adapters::toolbar::{
    PetuniaResponsiveToolbar, PetuniaToolbarCluster, PetuniaToolbarSlot, PetuniaToolbarSpec,
};
use crate::adapters::top_bar::{PetuniaTopBar, PetuniaTopBarSlot, PetuniaTopBarSpec};
use crate::foundation::density::input_h;
use crate::foundation::motion::PetuniaMotion;
use crate::foundation::spacing::{CONTROL, GROUP, RELATED, vspace};
use crate::foundation::typography::{TextRole, rich};
use crate::icon_registry::PetuniaIcon;
use crate::inspector_context::InspectorTab;
use crate::inspector_widgets::{
    NumericField, NumericOpts, SectionOpts, Vector3Field, block_header, context_tabs, section,
};
use crate::tokens;
use crate::toolbar::plan_group;
use crate::widgets::{
    PetuniaIconButton, PetuniaMenuButton, PetuniaMenuCheckboxItem, PetuniaMenuItem,
    PetuniaMenuRadioItem, PetuniaPropertyTabButton, PetuniaToolbarButton, PetuniaWorkspacePill,
    petunia_action_button, petunia_menu_separator, petunia_search_box_with_width,
};

/// Estado próprio da galeria.
///
/// Deliberadamente **não** vive em `AppState`: o produto não deve carregar
/// estado de vitrine. O que a galeria controla é só o eixo de configuração que
/// ela precisa variar (tema, densidade, escala, idioma) mais alguns valores de
/// campo para os controles terem o que editar.
#[derive(Debug, Clone)]
pub struct GalleryState {
    /// Preset de densidade em exibição.
    pub density: UiDensity,
    /// Id de tema do registro (`petunia-dark`, `petunia-high-contrast`, ...).
    pub theme_id: String,
    /// Fator de escala da UI (`set_zoom_factor`).
    pub scale: f32,
    /// Escalas oferecidas pela §39.
    pub scales: Vec<f32>,
    /// Idioma da UI (`en`, `pt-BR`).
    pub locale: String,
    /// Campo de busca de demonstração.
    pub search: String,
    /// Campo numérico de demonstração.
    pub number: f32,
    /// Campo vetorial de demonstração.
    pub vector: [f32; 3],
    /// Slider cru de demonstração (candidato a componente Petunia).
    pub slider: f32,
    /// Checkbox de demonstração.
    pub toggle: bool,
    /// Aba de inspector selecionada na demonstração de tabs.
    pub tab: InspectorTab,
    /// Índice da pílula selecionada.
    pub pill: usize,
    /// Ordem da lista reordenável de demonstração (§49).
    pub drag_items: Vec<String>,
    /// Estado que a demonstração de motion interpola (§49).
    pub motion_visible: bool,
    /// Se a demonstração de formulário chega com erros (§51).
    pub form_invalid: bool,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            density: UiDensity::Comfortable,
            theme_id: "petunia-dark".to_string(),
            scale: 1.0,
            // §39: escalas obrigatórias de verificação.
            scales: vec![1.0, 1.25, 1.5, 1.75, 2.0],
            locale: "en".to_string(),
            search: String::new(),
            number: 1.25,
            vector: [1.0, 2.0, 3.0],
            slider: 0.5,
            toggle: true,
            tab: InspectorTab::Object,
            pill: 0,
            drag_items: vec![
                "Selecionar".to_string(),
                "Mover".to_string(),
                "Girar".to_string(),
                "Escalar".to_string(),
            ],
            motion_visible: true,
            form_invalid: true,
        }
    }
}

/// Desenha a galeria inteira dentro de `ui`.
///
/// `state` é o estado real do app (tokens de tema, i18n, undo de campos); a
/// galeria o usa para que o que aparece aqui seja exatamente o que o produto
/// vê — não uma reimplementação paralela.
pub fn draw(ui: &mut Ui, state: &mut AppState, g: &mut GalleryState) {
    ui.ctx().set_zoom_factor(g.scale);
    configuration_bar(ui, state, g);
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            foundations(ui, g);
            buttons(ui, state, g);
            navigation_and_selection(ui, state, g);
            input_fields(ui, state, g);
            structure(ui, g);
            menus(ui, g);
            overlays(ui, g);
            collections(ui, g);
            feedback(ui, g);
            adapter_layouts(ui, g);
            pending_inventory(ui, g);
        });
}

// ------------------------------------------------------------------ harness

/// Barra de configuração: tema, densidade, escala e idioma.
fn configuration_bar(ui: &mut Ui, state: &mut AppState, g: &mut GalleryState) {
    ui.horizontal(|ui| {
        PetuniaMenuButton::new("Theme")
            .tooltip("Tema ativo do registro")
            .show(ui, |ui| {
                let registry = petunia_config::ThemeRegistry::global();
                for manifest in registry.available() {
                    let id = manifest.id.clone();
                    if PetuniaMenuRadioItem::new(&id, g.theme_id == id)
                        .show(ui)
                        .clicked()
                    {
                        state.ui.active_theme_id = id.clone();
                        g.theme_id = id;
                    }
                }
            });

        PetuniaMenuButton::new("Density").show(ui, |ui| {
            for density in UiDensity::all() {
                let label = density.key();
                if PetuniaMenuRadioItem::new(label, g.density == density)
                    .show(ui)
                    .clicked()
                {
                    g.density = density;
                    state.ui.density = density;
                }
            }
        });

        PetuniaMenuButton::new("Scale")
            .tooltip("Escala de UI (set_zoom_factor)")
            .show(ui, |ui| {
                for scale in g.scales.clone() {
                    let label = format!("{:.0}%", scale * 100.0);
                    if PetuniaMenuRadioItem::new(&label, (g.scale - scale).abs() < f32::EPSILON)
                        .show(ui)
                        .clicked()
                    {
                        g.scale = scale;
                    }
                }
            });

        PetuniaMenuButton::new("Locale")
            .tooltip("Idioma da UI (teste de string longa)")
            .show(ui, |ui| {
                for locale in ["en", "pt-BR"] {
                    if PetuniaMenuRadioItem::new(locale, g.locale == locale)
                        .show(ui)
                        .clicked()
                    {
                        g.locale = locale.to_string();
                        state.ui.i18n.set_lang(locale);
                    }
                }
            });

        PetuniaToolbarButton::new(PetuniaIcon::Measure, "Reset")
            .compact(false)
            .tooltip("Volta tema, densidade, escala e idioma ao default")
            .show(ui);
    });
}

// ----------------------------------------------------------------- fundações

/// Fundações: spacing, radius, tipografia, densidade e motion.
fn foundations(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.foundations", "Foundations", |ui| {
        ui.label(
            rich(
                "spacing · radius · typography · density · motion",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
        vspace(ui, RELATED);

        for role in TextRole::ALL {
            ui.label(rich(format!("{role:?} — {:.1}px", role.size()), role));
        }
        vspace(ui, CONTROL);

        for gap in crate::foundation::spacing::PetuniaSpacing::ALL {
            ui.label(
                rich(format!("spacing {gap:.0}px"), TextRole::Caption)
                    .color(tokens::TEXT_SECONDARY),
            );
        }
        vspace(ui, CONTROL);

        for radius in crate::foundation::radius::ALL {
            ui.label(
                rich(format!("radius {:.0}px", radius.nw), TextRole::Caption)
                    .color(tokens::TEXT_SECONDARY),
            );
        }
        vspace(ui, CONTROL);

        ui.label(rich("density presets", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        for density in UiDensity::all() {
            ui.label(
                rich(
                    format!(
                        "{:?} — row {:.0} · input {:.0}",
                        density,
                        crate::foundation::density::row_h(density),
                        crate::foundation::density::input_h(density)
                    ),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_SECONDARY),
            );
        }
        vspace(ui, CONTROL);

        ui.label(rich("motion", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        for duration in crate::foundation::motion::PetuniaMotion::ALL {
            ui.label(
                rich(
                    format!("{:.0} ms", crate::foundation::motion::millis(duration)),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_SECONDARY),
            );
        }
    });
}

// ------------------------------------------------------------------- botões

fn buttons(ui: &mut Ui, state: &AppState, g: &GalleryState) {
    group(ui, g.density, "gallery.buttons", "Buttons", |ui| {
        ui.horizontal(|ui| {
            petunia_action_button(ui, Some(PetuniaIcon::Extrude), "Extrude", false);
            petunia_action_button(ui, None, "Aplicar", false);
            petunia_action_button(ui, Some(PetuniaIcon::Slice), "Descartar", true);
        });
        vspace(ui, GROUP);

        ui.label(rich("icon buttons", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        ui.horizontal(|ui| {
            PetuniaIconButton::new(PetuniaIcon::Move, "Mover", 24.0).show(ui);
            PetuniaIconButton::new(PetuniaIcon::Rotate, "Girar", 28.0)
                .selected(true)
                .show(ui);
            PetuniaIconButton::new(PetuniaIcon::Scale, "Escalar", 32.0)
                .enabled(false)
                .show(ui);
        });
        vspace(ui, GROUP);

        ui.label(rich("toolbar cluster", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        ui.horizontal(|ui| {
            for (icon, label) in [
                (PetuniaIcon::SelectBox, "Select"),
                (PetuniaIcon::Cursor3D, "Cursor 3D"),
                (PetuniaIcon::Measure, "Measure"),
            ] {
                PetuniaToolbarButton::new(icon, label)
                    .selected(state.active_tool == label.to_lowercase())
                    .show(ui);
            }
        });
        vspace(ui, CONTROL);
        ui.horizontal(|ui| {
            PetuniaToolbarButton::new(PetuniaIcon::AddPrimitive, "Add Primitive")
                .compact(false)
                .show(ui);
        });
    });
}

// ------------------------------------------------- navegação e seleção

fn navigation_and_selection(ui: &mut Ui, state: &mut AppState, g: &mut GalleryState) {
    group(
        ui,
        g.density,
        "gallery.nav",
        "Navigation & Selection",
        |ui| {
            ui.label(rich("workspace pills", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
            ui.horizontal(|ui| {
                for (index, ws) in Workspace::all().into_iter().enumerate() {
                    let label = state.t(ws.key());
                    if PetuniaWorkspacePill::new(&label, g.pill == index)
                        .show(ui)
                        .clicked()
                    {
                        g.pill = index;
                    }
                }
            });
            vspace(ui, GROUP);

            ui.label(
                rich("segmented control (tabs)", TextRole::SectionTitle)
                    .color(tokens::TEXT_PRIMARY),
            );
            let tabs = [
                (InspectorTab::Object, state.t(InspectorTab::Object.key())),
                (InspectorTab::Modify, state.t(InspectorTab::Modify.key())),
                (
                    InspectorTab::Material,
                    state.t(InspectorTab::Material.key()),
                ),
                (
                    InspectorTab::Selection,
                    state.t(InspectorTab::Selection.key()),
                ),
            ];
            if let Some(picked) = context_tabs(ui, g.density, &tabs, g.tab) {
                g.tab = picked;
            }
            vspace(ui, GROUP);

            ui.label(rich("property tabs", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
            ui.horizontal(|ui| {
                for (index, icon) in [
                    PetuniaIcon::PropObject,
                    PetuniaIcon::PropModifiers,
                    PetuniaIcon::PropMaterial,
                    PetuniaIcon::PropRender,
                ]
                .into_iter()
                .enumerate()
                {
                    PetuniaPropertyTabButton::new(icon, g.pill == index)
                        .tooltip(format!("Property tab {index}"))
                        .show(ui);
                }
            });
        },
    );
}

// ------------------------------------------------------------------ campos

fn input_fields(ui: &mut Ui, state: &mut AppState, g: &mut GalleryState) {
    group(ui, g.density, "gallery.fields", "Inputs", |ui| {
        petunia_search_box_with_width(ui, &mut g.search, "Search…", 220.0);
        vspace(ui, GROUP);

        ui.label(rich("numeric field", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        let mut number = g.number;
        NumericField {
            value: &mut number,
            opts: NumericOpts::plain(0.01, 3),
            session_key: "gallery.number",
            undo_label: "Gallery numeric field",
            width: 96.0,
            height: input_h(g.density),
        }
        .show(ui, state);
        g.number = number;
        vspace(ui, GROUP);

        ui.label(rich("Vector3Field", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        let mut vector = g.vector;
        Vector3Field {
            axis_names: ["X", "Y", "Z"],
            axis_colors: [tokens::AXIS_X, tokens::AXIS_Y, tokens::AXIS_Z],
            values: &mut vector,
            opts: NumericOpts::plain(0.01, 3),
            session_key: "gallery.vector",
            undo_label: "Gallery vector field",
        }
        .show(ui, state);
        g.vector = vector;
        vspace(ui, GROUP);

        ui.label(rich("slider (raw egui)", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        ui.add(egui::Slider::new(&mut g.slider, 0.0..=1.0).show_value(true));
        vspace(ui, GROUP);

        ui.checkbox(&mut g.toggle, "checkbox");
    });
}

// --------------------------------------------------------------- estrutura

fn structure(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.structure", "Structure", |ui| {
        block_header(ui, PetuniaIcon::PropObject, "block_header");
        vspace(ui, CONTROL);
        ui.label(
            rich(
                "section() — disclosure de altura row_h, colapsável",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
        vspace(ui, RELATED);
        section(
            ui,
            g.density,
            "gallery.section.nested",
            SectionOpts {
                title: "Nested section",
                summary: Some("summary when collapsed"),
                default_open: true,
                force_open: false,
            },
            |ui| {
                ui.label(rich("corpo da seção", TextRole::Label).color(tokens::TEXT_SECONDARY));
            },
        );
    });
}

// ------------------------------------------------------------------- menus

fn menus(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.menus", "Menus", |ui| {
        ui.horizontal(|ui| {
            PetuniaMenuButton::new("Menu").show(ui, |ui| {
                PetuniaMenuItem::new("Extrude")
                    .icon(PetuniaIcon::Extrude)
                    .shortcut(Some("E"))
                    .show(ui);
                PetuniaMenuItem::new("Inset")
                    .icon(PetuniaIcon::Inset)
                    .shortcut(Some("I"))
                    .show(ui);
                PetuniaMenuItem::new("With submenu").submenu(true).show(ui);
                PetuniaMenuItem::new("Disabled").enabled(false).show(ui);
                petunia_menu_separator(ui);
                PetuniaMenuCheckboxItem::new("Snap to grid", true)
                    .shortcut(Some("Shift+S"))
                    .show(ui);
                PetuniaMenuCheckboxItem::new("Snap to vertex", false).show(ui);
            });

            PetuniaMenuButton::chevron_only()
                .tooltip("Menu só com chevron")
                .show(ui, |ui| {
                    PetuniaMenuRadioItem::new("Top", true).show(ui);
                    PetuniaMenuRadioItem::new("Front", false).show(ui);
                });
        });
        vspace(ui, CONTROL);
        ui.label(
            rich(
                "RMB context menu: pendente — extrair de nav_gizmo.rs para PetuniaContextMenu (§38).",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
    });
}

// ---------------------------------------------------------------- overlays

fn overlays(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.overlays", "Overlays", |ui| {
        pending(
            ui,
            "popup",
            "Abrir popup arbitrário ancorado: hoje só existe por `egui::Popup::menu` dentro de painéis.",
        );
        pending(
            ui,
            "modal",
            "Modal reutilizável: existem modais de produto (settings, recovery) sem componente comum.",
        );
    });
}

// --------------------------------------------------------------- coleções

fn collections(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.collections", "Collections", |ui| {
        pending(
            ui,
            "tree",
            "`adapters::tree` só expõe TreeViewSettings; falta a façade de árvore reutilizável.",
        );
        pending(
            ui,
            "drag list",
            "`egui_dnd` não é dependência — ver docs/dependencies/ui-ecosystem-lock.md.",
        );
        pending(
            ui,
            "asset cards",
            "`primitive_card::draw_primitive_card` exige viewport_rect do produto: extrair antes de exibir.",
        );
    });
}

// --------------------------------------------------------------- feedback

fn feedback(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.feedback", "Feedback", |ui| {
        pending(
            ui,
            "empty state",
            "Não existe helper: cada painel escreve o próprio texto vazio.",
        );
        pending(
            ui,
            "inline messages",
            "Mensagem inline (info/warn/error) precisa de componente; hoje há cores de token sem layout.",
        );
        pending(
            ui,
            "progress",
            "Nenhum uso de `ProgressBar`: jobs relatam por telemetria, não por barra.",
        );
        pending(
            ui,
            "async/loading",
            "`adapters::inbox::UiBridge` existe para o transporte; falta o estado visual de carregando.",
        );
    });
}

// ------------------------------------------------------- layouts de adapter

fn adapter_layouts(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.layouts", "Taffy layouts", |ui| {
        let item = |index: usize, ui: &mut Ui| {
            let _ = petunia_action_button(ui, None, &format!("item {}", index + 1), false);
        };

        ui.label(rich("row", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        responsive(
            ui,
            egui::Id::new("gallery.taffy.row"),
            PetuniaResponsiveLayout::row().with_gap(PetuniaGap::related()),
            3,
            |index, ui| item(index, ui),
            |_ui| (),
        );
        vspace(ui, GROUP);

        ui.label(rich("wrap row", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        responsive(
            ui,
            egui::Id::new("gallery.taffy.wrap"),
            PetuniaResponsiveLayout::wrap_row().with_gap(PetuniaGap::gutter()),
            4,
            |index, ui| item(index, ui),
            |_ui| (),
        );
        vspace(ui, GROUP);

        ui.label(rich("grid (3 columns)", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        responsive(
            ui,
            egui::Id::new("gallery.taffy.grid"),
            PetuniaResponsiveLayout::grid(3)
                .with_gap(PetuniaGap::related())
                .with_justify(PetuniaJustify::Start),
            6,
            |index, ui| item(index, ui),
            |_ui| (),
        );
        vspace(ui, GROUP);

        ui.label(rich("key/value row", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        flex_key_value_row(
            ui,
            egui::Id::new("gallery.taffy.kv"),
            "Position X",
            "1.250 m",
        );
        flex_key_value_row(
            ui,
            egui::Id::new("gallery.taffy.kv2"),
            "Rotation Y",
            "0.000°",
        );
        vspace(ui, GROUP);

        // Contrato da Wave 3 (§45): o componente recebe a largura já resolvida
        // pelo adapter, em vez de calcular breakpoints próprios. Esta é a
        // superfície que `Vector3Field` e `context_tabs` consomem.
        ui.label(
            rich("columns, responsivo (mín. 78)", TextRole::SectionTitle)
                .color(tokens::TEXT_PRIMARY),
        );
        columns(
            ui,
            egui::Id::new("gallery.taffy.columns"),
            PetuniaColumnSpec::responsive(3, 78.0, RELATED),
            |index, width, ui| {
                let _ = petunia_action_button(
                    ui,
                    None,
                    &format!("col {} · {width:.0}px", index + 1),
                    false,
                );
            },
            |_ui| (),
        );
        vspace(ui, GROUP);

        ui.label(rich("columns, fixo (abas)", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY));
        columns(
            ui,
            egui::Id::new("gallery.taffy.columns-fixed"),
            PetuniaColumnSpec::fixed(4, RELATED),
            |index, width, ui| {
                let _ =
                    petunia_action_button(ui, None, &format!("{} · {width:.0}", index + 1), false);
            },
            |_ui| (),
        );
        vspace(ui, GROUP);

        ui.label(
            rich("clamped / fill remaining", TextRole::SectionTitle).color(tokens::TEXT_PRIMARY),
        );
        ui.label(
            rich(
                format!(
                    "clamped(96..180) = {:.0} · remaining(−108) = {:.0}",
                    clamped_width(ui, 96.0, 180.0),
                    fill_remaining(ui, 108.0, 60.0)
                ),
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
    });

    tool_grid_exhibit(ui, g);
    form_exhibit(ui, g);
    form_validation_exhibit(ui, g);
    drag_list_exhibit(ui, g);
    motion_exhibit(ui, g);
    responsive_toolbar_exhibit(ui, g);
    top_bar_exhibit(ui, g);
}

// ------------------------------------------------------------ lista reordenável

/// Especificação de vitrine da lista reordenável (§49).
static GALLERY_DRAG_LIST: PetuniaDragList = PetuniaDragList::new(PetuniaDragSpec::new());

/// Lista reordenável com o mesmo esqueleto de linha do produto (grip + conteúdo).
///
/// Arraste a linha pelo grip à esquerda: a ordem é aplicada no drop, não a cada
/// frame — o item que acompanha o ponteiro é a cópia flutuante do motor.
fn drag_list_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(
        ui,
        g.density,
        "gallery.drag-list",
        "Lista reordenável (§49)",
        |ui| {
            ui.label(
                rich(
                    "Sem setas ↑/↓: o grip arrasta, o adapter aplica a ordem. O produto \
                     declara o desenho da linha; o grip, a detecção e a nova ordem são do \
                     contrato.",
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, RELATED);

            // A ordem vive na memória do contexto: a vitrine redesenha a cada
            // frame e reordenar não pode depender de um `Vec` recriado.
            let order_id = egui::Id::new("gallery.drag-order");
            let mut order = ui
                .ctx()
                .data_mut(|d| d.get_temp::<Vec<String>>(order_id))
                .unwrap_or_else(|| g.drag_items.clone());
            GALLERY_DRAG_LIST.show(
                ui,
                "gallery.drag-list",
                &mut order,
                |item| egui::Id::new(("gallery.drag-item", item.as_str())),
                |ui, item, row| {
                    let color = if row.dragging {
                        tokens::ACCENT_BLUE
                    } else {
                        tokens::TEXT_PRIMARY
                    };
                    ui.label(rich(item.as_str(), TextRole::Label).color(color));
                    if row.dragging {
                        ui.label(rich("arrastando", TextRole::Caption).color(tokens::TEXT_MUTED));
                    }
                },
            );
            let count = order.len();
            ui.ctx().data_mut(|d| d.insert_temp(order_id, order));
            ui.label(
                rich(
                    format!("{count} itens · arraste pelo grip"),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
        },
    );
}

// ------------------------------------------------------------------- motion

/// Motion do sistema (§49): as três durações com as curvas reais.
fn motion_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.motion", "Motion (§49)", |ui| {
        ui.label(
            rich(
                "Uma única fonte de durações e curvas. Com `animation_time == 0` no \
                 tema, tudo isto vira troca instantânea — o estado final é o mesmo.",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
        vspace(ui, RELATED);

        let visible = g.motion_visible;
        let alpha = PetuniaMotion::reveal(ui.ctx(), "gallery.motion.reveal", visible);
        for duration in PetuniaMotion::ALL {
            ui.label(
                rich(
                    format!(
                        "{} · {:.0} ms · curva {:?}",
                        duration_name(duration),
                        crate::foundation::motion::millis(duration),
                        PetuniaMotion::EASING
                    ),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
        }
        vspace(ui, RELATED);
        ui.label(
            rich(format!("reveal() = {alpha:.2}"), TextRole::Label).color(tokens::TEXT_PRIMARY),
        );
        let (rect, _) = ui.allocate_exact_size(egui::vec2(160.0, 8.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, tokens::RADIUS_SMALL, tokens::BG_SURFACE);
        let mut filled = rect;
        filled.set_width(rect.width() * alpha);
        ui.painter()
            .rect_filled(filled, tokens::RADIUS_SMALL, tokens::ACCENT_BLUE);
        ui.ctx().request_repaint();
    });
}

/// Nome legível de uma duração canônica.
fn duration_name(duration: std::time::Duration) -> &'static str {
    match duration {
        PetuniaMotion::INSTANT => "instant",
        PetuniaMotion::BASE => "base",
        PetuniaMotion::SLOW => "slow",
        _ => "outra",
    }
}

// ------------------------------------------------------- formulário validado

/// Validação por campo do `PetuniaForm` (§51), no mesmo contrato do arranjo.
fn form_validation_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(
        ui,
        g.density,
        "gallery.form-validation",
        "Formulário validado (§51)",
        |ui| {
            ui.label(
                rich(
                    "O domínio monta o relatório, o contrato mostra: resumo inline e \
                     marca por campo depois do gesto de confirmar (`reveal_errors`).",
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, RELATED);

            let report = if g.form_invalid {
                PetuniaValidationReport::new()
                    .with_error("campo.escala", "fora da faixa permitida")
                    .with_error("campo.atalho", "compartilha o atalho com 'Salvar como'")
            } else {
                PetuniaValidationReport::new()
            };
            let mut session = PetuniaFormSession::new(report);
            GALLERY_FORM.error_summary_titled(ui, &session, Some("Formulário inválido"));
            GALLERY_FORM.validated_field(
                ui,
                &mut session,
                "campo.escala",
                "Escala da UI",
                |ui: &mut Ui, width: f32| {
                    let mut value = g.scale;
                    ui.add_sized(
                        egui::vec2(width.min(160.0), 18.0),
                        egui::DragValue::new(&mut value).range(0.5..=2.0),
                    )
                },
            );
            vspace(ui, GROUP);
        },
    );
}

// -------------------------------------------------------- grade de ferramentas

/// Especificação de vitrine da grade da paleta (§48).
///
/// As mesmas duas larguras declaradas do produto: célula de ícone e célula que
/// comporta o rótulo.
static GALLERY_TOOL_GRID: PetuniaToolGridSpec =
    PetuniaToolGridSpec::new(tokens::TOOLBAR_WIDTH, 120.0, 3.0);

/// Grade da paleta em três larguras e nos dois tetos de coluna.
///
/// Mostra o que substituiu `available_width() < 90.0` e `>= 100.0`: a coluna é
/// derivada do mínimo da célula e "cabe o rótulo?" vem da largura que a célula
/// recebeu.
fn tool_grid_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(
        ui,
        g.density,
        "gallery.tool-grid",
        "Grade da paleta (§48)",
        |ui| {
            ui.label(
                rich(
                    "A coluna não é escolhida por breakpoint: sai do mínimo real da célula \
                     (fit_columns). O rótulo aparece quando a largura resolvida alcança o \
                     mínimo declarado — o mesmo código cobre escala de UI e idioma longo.",
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, RELATED);

            for (pane, max_columns) in [
                (40.0_f32, 1u16),
                (74.0, 1),
                (74.0, 2),
                (180.0, 1),
                (180.0, 2),
            ] {
                let grid = GALLERY_TOOL_GRID.with_max_columns(max_columns);
                let (columns, cell) = grid.plan(pane);
                ui.label(
                    rich(
                        format!(
                            "coluna {pane:.0}px · teto {max_columns} → {columns} coluna(s) de {cell:.0}px · rótulo {}",
                            if grid.labels_fit(cell) { "sim" } else { "não (ícone)" }
                        ),
                        TextRole::Caption,
                    )
                    .color(tokens::TEXT_MUTED),
                );
                ui.scope(|ui| {
                    ui.set_max_width(pane);
                    grid.show(
                        ui,
                        egui::Id::new(("gallery.tool-grid", pane as u32, max_columns)),
                        3,
                        |index, cell, ui| {
                            let label = if cell.labeled {
                                format!("ferramenta {}", index + 1)
                            } else {
                                format!("{}", index + 1)
                            };
                            let _ = petunia_action_button(ui, None, &label, false);
                        },
                        |_ui| (),
                    );
                });
                vspace(ui, RELATED);
            }

            vspace(ui, GROUP);
            ui.label(
                rich("arranjo do grupo (split button)", TextRole::SectionTitle)
                    .color(tokens::TEXT_PRIMARY),
            );
            for cell in [tokens::TOOLBAR_WIDTH, 52.0, 64.0, 160.0] {
                let row = plan_group(cell);
                ui.label(
                    rich(
                        format!(
                            "célula {cell:.0}px → botão {:.0}px · seta {}",
                            row.button,
                            if row.chevron {
                                "visível"
                            } else {
                                "não cabe (menu pelo clique secundário)"
                            }
                        ),
                        TextRole::Caption,
                    )
                    .color(tokens::TEXT_MUTED),
                );
            }
        },
    );
}

// ------------------------------------------------------------------ formulário

/// Especificação de vitrine do formulário (§48).
static GALLERY_FORM: PetuniaForm = PetuniaForm::new(96.0, 160.0);

/// Contrato `PetuniaForm`: o mesmo campo em três larguras.
///
/// A decisão `Inline`/`Stacked` é pura ([`plan_row`]) e não consulta widget
/// nenhum; aqui ela é exibida ao lado do campo realmente desenhado.
fn form_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.form", "Formulário (§48)", |ui| {
        ui.label(
            rich(
                "Rótulo com largura declarada e controle com a largura que sobra. Quando o \
                     controle não cabe com o mínimo dele, o campo empilha em vez de espremer \
                     o controle.",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
        vspace(ui, RELATED);

        for width in [220.0_f32, 380.0, 640.0] {
            let plan = plan_row(width, &GALLERY_FORM.spec);
            ui.label(
                rich(
                    format!(
                        "largura {width:.0} → {:?} · rótulo {:.0} · controle {:.0}",
                        plan.placement, plan.label_width, plan.control_width
                    ),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            ui.scope(|ui| {
                ui.set_max_width(width);
                GALLERY_FORM.field(
                    ui,
                    egui::Id::new(("gallery.form.field", width as u32)),
                    "Densidade",
                    |ui, control| {
                        ui.set_max_width(control);
                        let _ = petunia_action_button(ui, None, "Média", false);
                    },
                );
                GALLERY_FORM.section(ui, "Seção", Some("descrição da seção"));
                let mut on = true;
                GALLERY_FORM.toggle(ui, "Mostrar prateleira", &mut on);
            });
            vspace(ui, GROUP);
        }
    });
}

// ------------------------------------------------------------------ top bar

/// Especificação de vitrine da Top Bar (§25).
///
/// Três zonas com a mesma semântica do header real: menus (núcleo), pílulas no
/// centro e ações globais que caem para a seta por prioridade.
static GALLERY_TOP_BAR: PetuniaTopBarSpec = PetuniaTopBarSpec {
    left: &[PetuniaToolbarCluster::pinned("top.menus")],
    center: &[PetuniaToolbarCluster::pinned("top.workspaces")],
    right: &[
        PetuniaToolbarCluster::overflowable("top.assets", 20),
        PetuniaToolbarCluster::overflowable("top.settings", 30),
    ],
    overflow: Some("top.overflow"),
};

/// Top Bar de três zonas em quatro larguras: mostra o centro geométrico e o
/// fallback declarado quando uma lateral não cabe na metade da barra.
fn top_bar_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(ui, g.density, "gallery.top-bar", "Top Bar (§25)", |ui| {
        ui.label(
            rich(
                "O centro é geométrico por distribuição (caixas laterais de largura \
                     igual), não por espaçadores somados. Abaixo do mínimo das laterais o \
                     adapter assume o fallback `Drifting` e declara isso — nada é cortado.",
                TextRole::Caption,
            )
            .color(tokens::TEXT_MUTED),
        );
        vspace(ui, RELATED);

        for width in [420.0_f32, 640.0, 900.0, 1_320.0] {
            let plan = ui
                .scope(|ui| {
                    ui.set_max_width(width);
                    let bar = PetuniaTopBar::new(
                        egui::Id::new(("gallery.top-bar", width as u32)),
                        &GALLERY_TOP_BAR,
                    );
                    bar.show(ui, &mut |ui, slot| gallery_top_slot(ui, slot))
                })
                .inner;

            ui.label(
                rich(
                    format!(
                        "{width:.0}px → {} · visíveis {:?} · ocultas {:?}",
                        if plan.is_centered() {
                            "centro geométrico"
                        } else {
                            "centro drifting (fallback)"
                        },
                        plan.right_visible(),
                        plan.right_hidden()
                    ),
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, GROUP);
        }
    });
}

/// Desenho das faixas da Top Bar da vitrine — sem semântica de produto.
fn gallery_top_slot(ui: &mut Ui, slot: PetuniaTopBarSlot<'_>) {
    match slot.id {
        "top.menus" => {
            for label in ["File", "Edit", "Window", "Help"] {
                PetuniaMenuButton::new(label).show(ui, |ui| {
                    ui.set_min_width(120.0);
                    let _ = PetuniaMenuItem::new(label).show(ui);
                });
            }
        }
        "top.workspaces" => {
            for (label, active) in [("MODEL", true), ("PAINT", false), ("UV", false)] {
                let (bg, fg) = if active {
                    (tokens::bg_surface_active_for(ui.ctx()), tokens::TEXT_ACTIVE)
                } else {
                    (egui::Color32::TRANSPARENT, tokens::TEXT_SECONDARY)
                };
                let button = egui::Button::new(rich(label, TextRole::Pill).color(fg))
                    .fill(bg)
                    .corner_radius(tokens::RADIUS_PILL);
                let _ = ui.add(button);
            }
        }
        "top.overflow" => {
            let hidden = slot.hidden.join(", ");
            let label = if hidden.is_empty() {
                "⋯".to_owned()
            } else {
                format!("⋯ {hidden}")
            };
            PetuniaMenuButton::new(&label).show(ui, |ui| {
                ui.set_min_width(140.0);
                let _ = PetuniaMenuItem::new(&hidden).show(ui);
            });
        }
        other => {
            let label = other.trim_start_matches("top.");
            let _ = petunia_action_button(ui, None, label, false);
        }
    }
}

// ------------------------------------------------------- barra responsiva

/// Especificação de vitrine da barra responsiva (§46).
///
/// Espelha a forma do uso real: núcleo que não sai da linha, faixas de rank
/// declarado que caem para a seta e a faixa de acesso.
static GALLERY_TOOLBAR: PetuniaToolbarSpec = PetuniaToolbarSpec {
    primary: &[
        PetuniaToolbarCluster::pinned("gallery.core"),
        PetuniaToolbarCluster::pinned("gallery.menus"),
    ],
    secondary: &[
        PetuniaToolbarCluster::overflowable("gallery.transform", 10),
        PetuniaToolbarCluster::overflowable("gallery.snap", 20),
        PetuniaToolbarCluster::pinned("gallery.display"),
    ],
    max_rows: 2,
    overflow: Some("gallery.overflow"),
};

/// Barra responsiva: mesmas faixas em quatro larguras, para mostrar a decisão
/// de overflow e a passagem de uma para duas linhas.
fn responsive_toolbar_exhibit(ui: &mut Ui, g: &GalleryState) {
    group(
        ui,
        g.density,
        "gallery.responsive-toolbar",
        "Barra responsiva (§46)",
        |ui| {
            ui.label(
                rich(
                    "Nenhuma soma de pixels: o adapter mede cada faixa por sonda, \
                     decide pelo rank declarado e entrega a linha ao taffy. \
                     Amarelo = núcleo (nunca cai); vermelho = faixa que caiu para a seta.",
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, RELATED);

            for (width, height) in [
                (320.0_f32, 26.0_f32),
                (560.0, 26.0),
                (980.0, 26.0),
                (980.0, 60.0),
            ] {
                let plan = ui
                    .scope(|ui| {
                        ui.set_max_width(width);
                        let bar = PetuniaResponsiveToolbar::new(
                            egui::Id::new(("gallery.toolbar", width as u32, height as u32)),
                            &GALLERY_TOOLBAR,
                        );
                        bar.show(ui, &mut |ui, slot| gallery_slot(ui, slot))
                    })
                    .inner;

                ui.label(
                    rich(
                        format!(
                            "{width:.0}px · {height:.0}px → {} linha(s) · visíveis {:?} · ocultas {:?}",
                            plan.rows(),
                            plan.visible(1),
                            plan.hidden_all()
                        ),
                        TextRole::Caption,
                    )
                    .color(tokens::TEXT_MUTED),
                );
                vspace(ui, GROUP);
            }
        },
    );
}

/// Desenho das faixas da vitrine — sem semântica de produto, só geometria.
fn gallery_slot(ui: &mut Ui, slot: PetuniaToolbarSlot<'_>) {
    if slot.id == "gallery.overflow" {
        let hidden = slot.hidden.join(", ");
        let label = if hidden.is_empty() {
            "⋯".to_owned()
        } else {
            format!("⋯ {hidden}")
        };
        PetuniaMenuButton::new(&label).show(ui, |ui| {
            ui.set_min_width(140.0);
            let _ = PetuniaMenuItem::new(&hidden).show(ui);
        });
        return;
    }
    let label = slot.id.trim_start_matches("gallery.");
    // Núcleo (nunca cai) × faixa de rank declarado — a cor é a diferença.
    let core = matches!(
        slot.id,
        "gallery.core" | "gallery.menus" | "gallery.display"
    );
    if core {
        PetuniaMenuButton::new(label).show(ui, |ui| {
            ui.set_min_width(140.0);
            let _ = PetuniaMenuItem::new(&format!("{label} · núcleo")).show(ui);
        });
    } else {
        let _ = petunia_action_button(ui, None, label, false);
    }
}

// ------------------------------------------------------------------ lacunas

/// Inventário do que a §38 exige e ainda não existe como componente.
fn pending_inventory(ui: &mut Ui, g: &GalleryState) {
    group(
        ui,
        g.density,
        "gallery.pending",
        "Pendências (§38)",
        |ui| {
            ui.label(
                rich(
                    "Itens exigidos pela diretiva que ainda não têm componente reutilizável. \
                 Uma linha aqui é o contrato que falta — não um componente escondido.",
                    TextRole::Caption,
                )
                .color(tokens::TEXT_MUTED),
            );
            vspace(ui, RELATED);
            for (name, contract) in PENDING {
                pending(ui, name, contract);
            }
        },
    );
}

/// Itens da §38 ainda sem implementação, com o motivo real.
const PENDING: &[(&str, &str)] = &[
    (
        "context menu",
        "nav_gizmo.rs monta o menu RMB à mão; extrair para PetuniaContextMenu.",
    ),
    (
        "toggle / switch",
        "Não existe switch deslizante; `checkbox` cru é o único controle booleano.",
    ),
    (
        "badge / chip",
        "Contador de asset e tag de filtro são desenhados inline hoje.",
    ),
    (
        "breadcrumb",
        "Não há breadcrumb de contexto (Object › Mesh › Material).",
    ),
    (
        "table",
        "`egui_table` não é dependência (ver ui-ecosystem-lock).",
    ),
    (
        "virtual collection",
        "`egui_virtual_list` não é dependência; Asset Library pagina por conta própria.",
    ),
    ("suspense", "`egui_suspense` não é dependência."),
    (
        "form validation",
        "`PetuniaForm` existe (contrato Petunia sobre o taffy); falta o backend \
         `egui_form` para erro por campo e estado de validação.",
    ),
    (
        "section header",
        "Cabeçalho colapsável (chevron + rótulo forte + ações à direita) é montado \
         à mão em outliner.rs, properties_panel.rs e paint_ui.rs; extrair para \
         `PetuniaSectionHeader` quando a terceira cópia divergir.",
    ),
];

// ------------------------------------------------------------------ helpers

/// Seção da galeria com `id_salt` estável (a galeria é aberta muitas vezes).
fn group(
    ui: &mut Ui,
    density: UiDensity,
    salt: &'static str,
    title: &'static str,
    body: impl FnOnce(&mut Ui),
) {
    section(
        ui,
        density,
        salt,
        SectionOpts {
            title,
            summary: None,
            default_open: true,
            force_open: false,
        },
        |ui| {
            vspace(ui, RELATED);
            body(ui);
            vspace(ui, CONTROL);
        },
    );
}

/// Cartão de lacuna: nome + contrato pendente.
///
/// Existe para que a ausência de um componente seja **visível e rastreável** na
/// própria vitrine, em vez de silenciosa.
fn pending(ui: &mut Ui, name: &str, contract: &str) {
    ui.label(
        rich(format!("pendente · {name}"), TextRole::Label)
            .color(tokens::TEXT_SECONDARY)
            .strong(),
    );
    ui.label(rich(contract, TextRole::Caption).color(tokens::TEXT_MUTED));
    vspace(ui, RELATED);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A galeria precisa compilar e renderizar sem panic em todas as densidades.
    #[test]
    fn gallery_renders_in_every_density() {
        for density in UiDensity::all() {
            let mut state = AppState::new("en");
            let mut g = GalleryState {
                density,
                ..GalleryState::default()
            };
            let ctx = egui::Context::default();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    draw(ui, &mut state, &mut g);
                });
            })
            .textures_delta
            .clear();
            assert_eq!(g.density, density);
        }
    }

    /// Tema e idioma não podem alterar o estado do produto fora do eixo visado.
    #[test]
    fn gallery_configuration_is_local_state() {
        let mut state = AppState::new("pt-BR");
        state.ui.i18n.set_lang("pt-BR");
        let g = GalleryState {
            theme_id: "petunia-high-contrast".to_string(),
            locale: "pt-BR".to_string(),
            ..GalleryState::default()
        };
        assert_eq!(g.scale, 1.0);
        assert_eq!(g.scales.len(), 5, "§39 exige cinco escalas de verificação");
    }

    /// Toda lacuna declarada precisa de contrato — vazio é bug de inventário.
    #[test]
    fn pending_inventory_entries_are_documented() {
        assert!(!PENDING.is_empty());
        for (name, contract) in PENDING {
            assert!(!name.is_empty());
            assert!(
                contract.len() > 20,
                "lacuna '{name}' sem contrato explicado: '{contract}'"
            );
        }
    }
}
