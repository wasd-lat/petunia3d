//! Composição do shell por regiões (Wave 5b — §31, §47).
//!
//! O shell não monta mais painéis laterais à mão: ele **descreve** o layout em
//! [`PetuniaShellLayout`] e o `adapters::tile_layout` resolve a geometria
//! (divisórias, mínimos, colapsos, lado e orientação do dock).
//!
//! ```text
//! lib.rs::draw
//! ├── main_header    (Panel::top)     chrome de largura total
//! ├── status_bar     (Panel::bottom)  chrome de largura total
//! ├── asset_browser  (Panel::left)    barra retrátil (chrome)
//! └── shell::draw
//!     └── árvore do macro-layout
//!         ├── Tools      (esquerda)
//!         ├── Viewport   (centro: barra de contexto + viewport 3D)
//!         ├── PaintCanvas (centro, à direita da viewport: só no PAINT)
//!         ├── DockHeader ┐
//!         ├── Parts      │ coluna do dock de contexto
//!         ├── Context    ┘
//!         └── Bottom     (faixa inferior, só com a feature de animação)
//! ```
//!
//! ## O que muda em relação ao dock antigo
//!
//! * as divisórias agora são do motor de layout (`EditAction::TileResized`), e a
//!   fração final volta para `UiState` — não há mais soma de pixels aqui;
//! * a política AUTO/MANUAL/colapso vive em
//!   [`PetuniaShellLayout::dock_pane_sizes`], testada sem `Ui`;
//! * duplo-clique na divisória volta ao dimensionamento automático, mas o
//!   hit-test é do adapter (a faixa é derivada dos retângulos medidos);
//! * o dock destacado continua janela flutuante, e o botão de reancorar mora no
//!   cabeçalho do dock (a faixa de aviso que gastava altura útil morreu);
//! * a Timeline deixou de roubar altura da área central: com a feature
//!   `animation-workspace` ela é o paine `Bottom`.

use egui::{Context, Ui};
use petunia_config::text_id;
use petunia_core::{AppState, DockOrientation, DockSide, ModuleRegistry};
use petunia_module_model::ToolRegistry;

use crate::adapters::tile_layout::{
    PetuniaLayoutAdapter, PetuniaLayoutContext, PetuniaLayoutResponse, PetuniaPane,
    PetuniaShellLayout,
};
use crate::regions::{self, RegionSlot};
use crate::{outliner, properties_panel, tokens, toolbar, workspaces};

/// Identificador da árvore do shell (raiz dos ids internos do adapter).
const SHELL_ID: &str = "petunia-shell";

/// DTO do layout do frame corrente.
///
/// Único tradutor `AppState → PetuniaShellLayout`: nenhum outro lugar do produto
/// precisa conhecer as regras do adapter.
pub fn layout_for(state: &AppState) -> PetuniaShellLayout {
    let profile = workspaces::profile_for(state.workspace);
    PetuniaShellLayout {
        workspace: state.workspace,
        dock_side: state.ui.dock_side,
        left_width: state.ui.left_width,
        right_width: state.ui.right_width,
        bottom_height: tokens::TIMELINE_HEIGHT,
        right_dock_split: state.ui.right_dock_split,
        right_dock_orientation: state.ui.dock_orientation,
        dock_auto: state.ui.scene_split_auto,
        parts_content_height: dock_top_content_height(state),
        // O perfil do workspace decide se o centro tem duas superfícies; a largura
        // arrastada pelo usuário entra depois, em [`draw`] (memória do adapter).
        canvas_enabled: profile.paints_on_canvas(),
        canvas_width: tokens::PAINT_CANVAS_DEFAULT_WIDTH,
        bottom_enabled: profile.bottom != workspaces::BottomPaneKind::None,
        parts_collapsed: state.ui.outliner_collapsed,
        context_collapsed: state.ui.inspector_collapsed,
        context_detached: state.ui.inspector_detached,
    }
}

/// Estimativa barata do conteúdo da seção **superior** do dock (O(1), sem
/// montar árvore).
///
/// Alimenta o dimensionamento AUTO: o paine cresce com o conteúdo em vez de
/// reservar meio dock para um objeto só. A seção superior depende do workspace:
/// no MODEL é a árvore da cena, no PAINT é a pilha de camadas — as duas crescem
/// com o número de linhas, então a mesma política AUTO serve para as duas.
fn dock_top_content_height(state: &AppState) -> f32 {
    let profile = workspaces::profile_for(state.workspace);
    let rows = match profile.dock_top {
        workspaces::DockSectionKind::Layers => active_layer_rows(state) + 1,
        _ => {
            state.project.assets.len()
                + state.project.collections.len()
                + state.project.annotation_groups.len()
                + state.project.annotations.len()
                + state.project.measurements.len()
                + state.project.refs.len()
                + 4
        }
    };
    regions::estimate_scene_content(rows, crate::inspector_widgets::row_h(state.ui.density))
}

/// Camadas do ativo no frame (contagem barata para o dimensionamento AUTO).
fn active_layer_rows(state: &AppState) -> usize {
    state
        .project
        .assets
        .get(state.project.active)
        .and_then(|object| object.paint_stack.as_ref())
        .map(|stack| stack.layers.len())
        .unwrap_or(1)
}

/// Desenha o macro-layout do shell e persiste o que o usuário mexeu.
pub fn draw(
    ui: &mut Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
) {
    puffin::profile_function!();
    let mut layout = layout_for(state);
    let adapter = PetuniaLayoutAdapter::new(SHELL_ID);
    if let Some(remembered) = adapter.canvas_width(ui.ctx()) {
        layout.canvas_width = remembered;
    }
    // Rótulos resolvidos **antes** dos closures: o callback de desenho empresta
    // `state` mutavelmente, então ele não pode ler i18n por conta própria.
    let labels: Vec<(PetuniaPane, String)> = PetuniaPane::ALL
        .iter()
        .map(|pane| (*pane, pane_label(state, *pane)))
        .collect();

    let response = adapter.show(
        ui,
        &layout,
        PetuniaLayoutContext {
            label: &|pane| {
                labels
                    .iter()
                    .find(|(candidate, _)| *candidate == pane)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_default()
            },
            draw: &mut |ui, pane| draw_pane(ui, state, tools, registry, pane),
        },
    );

    record_regions(ui.ctx(), &response);
    paint_dock_edge(ui, state, &response);
    persist(ui.ctx(), &adapter, state, &layout, &response);
    draw_detached_inspector(ui.ctx(), state, tools, registry);
}

/// Linha divisória entre a coluna do dock e o centro.
///
/// O tile não desenha moldura própria (o adapter é dono da geometria, não da
/// decoração): a borda que o antigo `Panel::right` fornecia é pintada aqui, na
/// fronteira **medida** — funciona igual com o dock à esquerda.
fn paint_dock_edge(ui: &Ui, state: &AppState, response: &PetuniaLayoutResponse) {
    let Some(dock) = response.dock_rect() else {
        return;
    };
    let x = match state.ui.dock_side {
        DockSide::Left => dock.max.x,
        DockSide::Right => dock.min.x,
    };
    ui.painter().line_segment(
        [egui::pos2(x, dock.min.y), egui::pos2(x, dock.max.y)],
        tokens::stroke_border_dyn(state),
    );
}

/// Registra as regiões do shell a partir dos retângulos que o adapter mediu.
///
/// O adapter é dono da geometria; o shell é dono do registro. `RightDock` é a
/// união do cabeçalho com as seções visíveis — nenhuma conta de pixel aqui.
fn record_regions(ctx: &Context, response: &PetuniaLayoutResponse) {
    let record = |pane: PetuniaPane, slot: RegionSlot| {
        if let Some(rect) = response.rect(pane) {
            regions::record(ctx, slot, rect);
        }
    };
    record(PetuniaPane::Tools, RegionSlot::LeftTools);
    record(PetuniaPane::PaintCanvas, RegionSlot::PaintCanvas);
    record(PetuniaPane::Parts, RegionSlot::RightOutliner);
    record(PetuniaPane::Context, RegionSlot::RightInspector);
    record(PetuniaPane::Bottom, RegionSlot::BottomDock);
    if let Some(dock) = response.dock_rect() {
        regions::record(ctx, RegionSlot::RightDock, dock);
    }
}

/// Aplica no estado o que o usuário mudou no frame (larguras e fração do dock).
///
/// Só persiste o que **mudou de verdade**: num frame comum as medidas do layout
/// são as do DTO, e sobrescrever a preferência com o valor encolhido por falta de
/// espaço apagaria a escolha do usuário. A memória por workspace é atualizada na
/// troca de workspace ([`AppState::switch_workspace`]), então o valor persistido
/// aqui é a fonte da memória.
fn persist(
    ctx: &Context,
    adapter: &PetuniaLayoutAdapter,
    state: &mut AppState,
    layout: &PetuniaShellLayout,
    response: &PetuniaLayoutResponse,
) {
    if response.split_equalize {
        // Duplo-clique na divisória: volta ao dimensionamento automático e
        // reabre as duas seções (gesto documentado do dock).
        state.ui.scene_split_auto = true;
        state.ui.outliner_collapsed = false;
        state.ui.inspector_collapsed = false;
        state.mark_dirty();
        return;
    }
    if !response.resized {
        return;
    }
    let mut dirty = false;
    if let Some(measured) = response.left_width
        && (measured - layout.left_width).abs() > 1.0
    {
        state.ui.left_width = measured;
        dirty = true;
    }
    if let Some(measured) = response.right_width
        && (measured - layout.right_width).abs() > 1.0
    {
        state.ui.right_width = measured;
        dirty = true;
    }
    if let Some(measured) = response.canvas_width
        && measured >= tokens::PAINT_CANVAS_MIN_WIDTH - 1.0
        && (measured - layout.canvas_width).abs() > 1.0
    {
        // O piso evita gravar o valor **encolhido** por falta de espaço numa
        // janela estreita como se fosse escolha do usuário.
        // A largura da tela 2D vive na memória do adapter (o DTO não tem onde
        // persistir): arrastar a divisória do centro é o gesto que a grava.
        adapter.remember_canvas_width(ctx, measured);
        dirty = true;
    }
    if let Some(split) = response.right_dock_split
        && (split - layout.right_dock_split).abs() > f32::EPSILON
    {
        state.ui.right_dock_split = split;
        // Arrastar a divisória sai do dimensionamento automático e reabre ambas
        // as seções (o usuário acabou de escolher onde elas ficam).
        state.ui.scene_split_auto = false;
        state.ui.outliner_collapsed = false;
        state.ui.inspector_collapsed = false;
        dirty = true;
    }
    if dirty {
        state.mark_dirty();
    }
}

/// Desenho de um paine declarado em [`PetuniaPane`].
fn draw_pane(
    ui: &mut Ui,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
    pane: PetuniaPane,
) {
    match pane {
        PetuniaPane::Tools => toolbar::draw_contents(ui, state, tools),
        PetuniaPane::Viewport => viewport_region(ui, state),
        PetuniaPane::PaintCanvas => {
            egui::Frame::new()
                .fill(tokens::bg_canvas(state))
                .show(ui, |ui| {
                    crate::modules_ui::paint_ui::draw_canvas_surface(ui, state)
                });
        }
        PetuniaPane::DockHeader => pane_surface(ui, tokens::bg_panel(state), 4.0, |ui| {
            dock_header(ui, state)
        }),
        PetuniaPane::Parts => {
            pane_surface(
                ui,
                tokens::bg_panel(state),
                6.0,
                |ui| match workspaces::profile_for(state.workspace).dock_top {
                    workspaces::DockSectionKind::Layers => {
                        crate::modules_ui::paint_ui::draw_layers_section(ui, state)
                    }
                    _ => outliner::draw_body(ui, state),
                },
            )
        }
        PetuniaPane::Context => {
            pane_surface(
                ui,
                tokens::bg_panel(state),
                6.0,
                |ui| match workspaces::profile_for(state.workspace).dock_bottom {
                    // O workspace legado de UV não tem cartão flutuante: a seção
                    // inferior dele é o conteúdo de pincel (a V1 não expõe o UV).
                    workspaces::DockSectionKind::Brush => {
                        crate::modules_ui::paint_ui::draw_brush_contents(ui, state)
                    }
                    _ => properties_panel::draw(ui, state, tools, registry),
                },
            )
        }
        PetuniaPane::Bottom => bottom_region(ui, state),
    }
}

/// Superfície de um paine da coluna do dock: fundo e respiro laterais do antigo
/// painel da coluna.
///
/// O recorte é por paine (e não pela coluna inteira) porque cada seção agora é
/// um tile; o fundo e a margem continuam sendo os mesmos tokens, então a única
/// diferença visual combinada é a divisória interna, que passa a ser a do motor
/// de layout.
fn pane_surface<R>(
    ui: &mut Ui,
    fill: egui::Color32,
    horizontal_margin: f32,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(horizontal_margin as i8, 4))
        .show(ui, content)
        .inner
}

/// Região central: barra de contexto (topo) + viewport.
///
/// A barra vive **dentro** do paine central — é o que a mantém restrita à
/// coluna do meio em vez de atravessar o dock e a paleta. No PAINT a tela 2D é
/// outro paine do centro (à direita daqui), então a barra continua pertencendo só
/// à coluna 3D: a tela não carrega controles de câmera que não a afetam.
fn viewport_region(ui: &mut Ui, state: &mut AppState) {
    crate::viewport_bar_panel(ui, state);
    crate::viewport(ui, state);
}

/// Faixa inferior (Timeline). Só existe quando o workspace a habilita.
#[cfg(feature = "animation-workspace")]
fn bottom_region(ui: &mut Ui, state: &mut AppState) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, tokens::bg_panel(state));
    ui.painter().rect_stroke(
        rect,
        0.0,
        tokens::stroke_border_dyn(state),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink2(egui::vec2(8.0, 4.0));
    if inner.width() <= 1.0 || inner.height() <= 1.0 {
        return;
    }
    let mut strip = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    crate::timeline::draw_contents(&mut strip, state);
}

/// Sem a feature de animação o paine inferior nunca é visível (o perfil do
/// workspace não o habilita); o braço existe só para o `match` ser total.
#[cfg(not(feature = "animation-workspace"))]
fn bottom_region(_ui: &mut Ui, _state: &mut AppState) {}

/// Cabeçalho do dock: lado, disposição e reancoragem do inspector destacado.
fn dock_header(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);
        let side_label = match state.ui.dock_side {
            DockSide::Left => state.t("dock.left"),
            DockSide::Right => state.t("dock.right"),
        };
        if ui
            .small_button(side_label)
            .on_hover_text(state.t("dock.side"))
            .clicked()
        {
            state.ui.dock_side = match state.ui.dock_side {
                DockSide::Left => DockSide::Right,
                DockSide::Right => DockSide::Left,
            };
            state.mark_dirty();
        }
        let orient_label = match state.ui.dock_orientation {
            DockOrientation::Stacked => state.t("dock.stacked"),
            DockOrientation::SideBySide => state.t("dock.side_by_side"),
        };
        if ui
            .small_button(orient_label)
            .on_hover_text(state.t("dock.orientation"))
            .clicked()
        {
            state.ui.dock_orientation = match state.ui.dock_orientation {
                DockOrientation::Stacked => DockOrientation::SideBySide,
                DockOrientation::SideBySide => DockOrientation::Stacked,
            };
            state.mark_dirty();
        }
        if state.ui.inspector_detached
            && ui
                .small_button(state.t_id(text_id::UI_REDOCK))
                .on_hover_text(state.t_id(text_id::UI_FLOATING_INSPECTOR))
                .clicked()
        {
            state.ui.inspector_detached = false;
            state.mark_dirty();
        }
    });
}

/// Inspector destacado: janela flutuante com o mesmo conteúdo da seção Context.
fn draw_detached_inspector(
    ctx: &Context,
    state: &mut AppState,
    tools: &ToolRegistry,
    registry: &mut ModuleRegistry,
) {
    if !state.ui.inspector_detached {
        return;
    }
    let mut is_open = true;
    // Wave 2 (§9.4): mínimo e máximo nunca excedem a viewport útil.
    let (default_size, min_size, max_size) = regions::modal_sizes(
        ctx.viewport_rect(),
        egui::vec2(280.0, 420.0),
        egui::vec2(220.0, 200.0),
        egui::vec2(640.0, 760.0),
    );
    let win = egui::Window::new(state.t_id(text_id::UI_PROPERTIES))
        .open(&mut is_open)
        .default_size(default_size)
        .min_size(min_size)
        .max_size(max_size)
        .frame(
            egui::Frame::new()
                .fill(tokens::bg_panel(state))
                .stroke(tokens::stroke_border_dyn(state))
                .inner_margin(egui::Margin::symmetric(6, 4)),
        )
        .show(ctx, |ui| {
            ui.push_id("detached_inspector", |ui| {
                properties_panel::draw(ui, state, tools, registry);
            });
        });
    if let Some(win) = win {
        regions::record(ctx, RegionSlot::RightInspector, win.response.rect);
    }
    if !is_open {
        state.ui.inspector_detached = false;
        state.mark_dirty();
    }
}

/// Rótulo do paine (abas do motor, telemetria e a11y). Nunca é texto novo: vem
/// do i18n.
fn pane_label(state: &AppState, pane: PetuniaPane) -> String {
    let profile = workspaces::profile_for(state.workspace);
    match pane {
        PetuniaPane::Tools => state.t("ui.tools"),
        PetuniaPane::Viewport => state.t("ui.viewport"),
        PetuniaPane::PaintCanvas => state.t("paint.canvas"),
        PetuniaPane::DockHeader => state.t("dock.orientation"),
        // As seções do dock têm o nome do **conteúdo** do workspace: Scene e
        // Properties no MODEL, Layers e Brush no PAINT.
        PetuniaPane::Parts => state.t(profile.dock_top_label),
        PetuniaPane::Context => state.t(profile.dock_bottom_label),
        PetuniaPane::Bottom => state.t("ui.timeline"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    fn state() -> AppState {
        let mut state = AppState::new("en");
        state.project.assets.clear();
        state.project.add("Cube", Mesh::cube(2.0));
        state.project.add("Vase", Mesh::cylinder(12, 0.4, 2.0));
        state.project.active = 0;
        state
    }

    /// Roda o shell num contexto real e devolve as regiões do último frame.
    fn run_shell(state: &mut AppState, size: egui::Vec2, frames: usize) -> regions::UiRegions {
        let ctx = Context::default();
        let tools = ToolRegistry::new();
        let mut registry = ModuleRegistry::new();
        let mut regions = None;
        for _ in 0..frames {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| {
                    regions::reset(ui.ctx());
                    draw(ui, state, &tools, &mut registry);
                },
            );
            out.textures_delta.clear();
            regions = regions::load(&ctx);
        }
        regions.expect("o shell registra as regiões todo frame")
    }

    #[test]
    fn layout_mirrors_the_ui_state() {
        let mut state = state();
        state.ui.left_width = 64.0;
        state.ui.right_width = 320.0;
        state.ui.dock_side = DockSide::Left;
        state.ui.outliner_collapsed = true;
        let layout = layout_for(&state);
        assert_eq!(layout.workspace, state.workspace);
        assert_eq!(layout.left_width, 64.0);
        assert_eq!(layout.right_width, 320.0);
        assert_eq!(layout.dock_side, DockSide::Left);
        assert!(layout.parts_collapsed);
        assert!(layout.dock_auto, "AUTO é o padrão do Scene");
        assert!(!layout.context_detached);
        assert!(!layout.bottom_enabled, "sem a feature não há dock inferior");
    }

    #[test]
    fn every_pane_region_comes_from_the_adapter_geometry() {
        let mut state = state();
        let regions = run_shell(&mut state, egui::vec2(1_280.0, 800.0), 3);
        for (name, rect) in regions.panes() {
            assert!(
                rect.width() > 0.0 && rect.height() > 0.0,
                "região {name} vazia: {rect:?}"
            );
        }
        assert!(regions.status_overlaps().is_empty());
        assert!(
            regions.left_tools.is_some(),
            "paleta de ferramentas ausente"
        );
        assert!(regions.right_outliner.is_some(), "Outliner ausente");
        assert!(regions.right_inspector.is_some(), "Inspector ausente");
        assert!(regions.right_dock.is_some(), "coluna do dock ausente");
        assert!(
            regions.viewport_toolbar.is_some(),
            "barra da viewport ausente"
        );
        assert!(
            regions.dock_sections_disjoint(),
            "Outliner e Inspector se sobrepõem"
        );
        assert!(
            regions.viewport_overlays_within_viewport(),
            "overlay fora da viewport"
        );
    }

    #[test]
    fn dock_sections_never_overlap_in_any_orientation() {
        for orientation in [DockOrientation::Stacked, DockOrientation::SideBySide] {
            for (outliner_collapsed, inspector_collapsed) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let mut state = state();
                state.ui.dock_orientation = orientation;
                state.ui.outliner_collapsed = outliner_collapsed;
                state.ui.inspector_collapsed = inspector_collapsed;
                let regions = run_shell(&mut state, egui::vec2(1_280.0, 800.0), 3);
                assert!(
                    regions.dock_sections_disjoint(),
                    "{orientation:?} outliner={outliner_collapsed} inspector={inspector_collapsed}"
                );
                assert!(
                    regions.status_overlaps().is_empty(),
                    "{orientation:?} invadiu a barra de status"
                );
            }
        }
    }

    #[test]
    fn dock_side_moves_the_column_without_touching_the_toolbar() {
        let mut right = state();
        let right_regions = run_shell(&mut right, egui::vec2(1_280.0, 800.0), 3);
        let mut left = state();
        left.ui.dock_side = DockSide::Left;
        let left_regions = run_shell(&mut left, egui::vec2(1_280.0, 800.0), 3);

        let tools_r = right_regions.left_tools.unwrap();
        let dock_r = right_regions.right_outliner.unwrap();
        let viewport_r = right_regions.viewport.unwrap();
        let tools_l = left_regions.left_tools.unwrap();
        let dock_l = left_regions.right_outliner.unwrap();
        let viewport_l = left_regions.viewport.unwrap();
        // A paleta de ferramentas não se move (ordem preservada do shell
        // original); o dock apenas troca de lado em volta da viewport.
        assert!(
            dock_r.min.x >= tools_r.max.x,
            "dock à direita deveria vir depois da paleta"
        );
        assert!(
            dock_r.min.x >= viewport_r.max.x - 1.0,
            "dock à direita deveria vir depois da viewport: {dock_r:?} vs {viewport_r:?}"
        );
        assert!(
            dock_l.min.x >= tools_l.max.x,
            "dock à esquerda continua depois da paleta: {dock_l:?} vs {tools_l:?}"
        );
        assert!(
            dock_l.max.x <= viewport_l.min.x + 1.0,
            "dock à esquerda deveria vir antes da viewport: {dock_l:?} vs {viewport_l:?}"
        );
    }

    #[test]
    fn detached_inspector_leaves_the_dock_column() {
        let mut docked = state();
        let docked_regions = run_shell(&mut docked, egui::vec2(1_280.0, 800.0), 3);
        let mut detached = state();
        detached.ui.inspector_detached = true;
        let detached_regions = run_shell(&mut detached, egui::vec2(1_280.0, 800.0), 3);

        let docked_dock = docked_regions.right_dock.unwrap();
        let detached_dock = detached_regions.right_dock.unwrap();
        assert!(
            detached_dock.height() > 0.0,
            "a coluna do dock continua existindo (Outliner + cabeçalho)"
        );
        assert!(
            docked_regions.right_inspector.unwrap().height() > detached_dock.height() * 0.25,
            "o inspector encaixado ocupa uma seção real do dock"
        );
        assert!(
            detached_dock.height() < docked_dock.height() + 1.0,
            "destacar o inspector não pode aumentar a coluna"
        );
    }

    /// Contexto + adapter do shell para exercitar `persist` sem montar a árvore.
    fn persist_env() -> (Context, PetuniaLayoutAdapter) {
        (Context::default(), PetuniaLayoutAdapter::new(SHELL_ID))
    }

    #[test]
    fn drag_persists_the_split_and_leaves_auto() {
        let (ctx, adapter) = persist_env();
        let mut state = state();
        state.ui.scene_split_auto = true;
        let layout = layout_for(&state);
        let response = PetuniaLayoutResponse {
            resized: true,
            right_dock_split: Some(0.62),
            ..Default::default()
        };
        persist(&ctx, &adapter, &mut state, &layout, &response);
        assert!((state.ui.right_dock_split - 0.62).abs() < f32::EPSILON);
        assert!(!state.ui.scene_split_auto, "arrastar sai do AUTO");
        assert!(!state.ui.outliner_collapsed);
        assert!(!state.ui.inspector_collapsed);
    }

    #[test]
    fn double_click_returns_to_auto_without_touching_the_split() {
        let (ctx, adapter) = persist_env();
        let mut state = state();
        state.ui.scene_split_auto = false;
        state.ui.right_dock_split = 0.7;
        state.ui.outliner_collapsed = true;
        state.ui.inspector_collapsed = true;
        let layout = layout_for(&state);
        persist(
            &ctx,
            &adapter,
            &mut state,
            &layout,
            &PetuniaLayoutResponse {
                split_equalize: true,
                ..Default::default()
            },
        );
        assert!(state.ui.scene_split_auto, "duplo-clique volta ao AUTO");
        assert!(
            (state.ui.right_dock_split - 0.7).abs() < f32::EPSILON,
            "equalizar é voltar ao AUTO, não reescrever a fração"
        );
        assert!(!state.ui.outliner_collapsed);
        assert!(!state.ui.inspector_collapsed);
    }

    #[test]
    fn a_frame_without_resize_preserves_the_user_preference() {
        let (ctx, adapter) = persist_env();
        let mut state = state();
        state.ui.left_width = 64.0;
        state.ui.right_width = 320.0;
        let layout = layout_for(&state);
        persist(
            &ctx,
            &adapter,
            &mut state,
            &layout,
            &PetuniaLayoutResponse {
                left_width: Some(12.0),
                right_width: Some(12.0),
                resized: false,
                ..Default::default()
            },
        );
        assert_eq!(state.ui.left_width, 64.0);
        assert_eq!(state.ui.right_width, 320.0);
    }

    /// Arrastar a divisória do centro grava a largura da tela 2D na memória do
    /// adapter; um frame sem arrasto preserva o que o usuário escolheu.
    #[test]
    fn dragging_the_center_divider_remembers_the_canvas_width() {
        let (ctx, adapter) = persist_env();
        let mut state = state();
        state.switch_workspace(petunia_core::Workspace::Paint);
        let layout = layout_for(&state);
        assert!(
            layout.canvas_enabled,
            "o PAINT tem duas superfícies no centro"
        );
        persist(
            &ctx,
            &adapter,
            &mut state,
            &layout,
            &PetuniaLayoutResponse {
                resized: true,
                canvas_width: Some(360.0),
                ..Default::default()
            },
        );
        assert_eq!(adapter.canvas_width(&ctx), Some(360.0));
        // Frame seguinte sem arrasto: a preferência continua onde estava.
        persist(
            &ctx,
            &adapter,
            &mut state,
            &layout,
            &PetuniaLayoutResponse {
                resized: false,
                canvas_width: Some(120.0),
                ..Default::default()
            },
        );
        assert_eq!(adapter.canvas_width(&ctx), Some(360.0));
    }

    /// O paine da tela 2D só entra no layout quando o perfil do workspace pinta
    /// no canvas: no MODEL o centro é uma superfície só.
    #[test]
    fn the_canvas_pane_belongs_to_the_paint_profile() {
        let model = layout_for(&state());
        assert!(!model.canvas_enabled, "MODEL não divide o centro");
        assert!(!model.is_visible(PetuniaPane::PaintCanvas));
        let mut painting = state();
        painting.switch_workspace(petunia_core::Workspace::Paint);
        let paint = layout_for(&painting);
        assert!(paint.is_visible(PetuniaPane::PaintCanvas));
        assert_eq!(paint.canvas_width, tokens::PAINT_CANVAS_DEFAULT_WIDTH);
    }

    /// Wave 5b: a Timeline deixou de ser uma faixa dentro da área central e
    /// passou a ser o paine `Bottom` da árvore — a viewport fica com a altura
    /// toda que sobra, em vez de dividir o mesmo espaço.
    #[cfg(feature = "animation-workspace")]
    #[test]
    fn animate_workspace_gives_the_timeline_its_own_pane() {
        let mut state = state();
        state.switch_workspace(Workspace::Animate);
        let regions = run_shell(&mut state, egui::vec2(1_280.0, 800.0), 3);
        let bottom = regions.bottom_dock.expect("faixa inferior registrada");
        let viewport = regions.viewport.expect("viewport registrada");
        assert!(bottom.height() > 0.0, "faixa inferior vazia");
        assert!(
            viewport.max.y <= bottom.min.y + 1.0,
            "a timeline não pode roubar altura da viewport: {viewport:?} vs {bottom:?}"
        );
        assert!(regions.status_overlaps().is_empty());
    }
    /// O workspace PAINT divide o **centro** entre viewport 3D e tela 2D (lado a
    /// lado, divisória arrastável) e troca o conteúdo da seção superior do dock
    /// (Layers). A geometria continua sendo do adapter: aqui só se verifica que
    /// cada região existe no lugar certo e que ninguém invade a barra de status.
    #[test]
    fn paint_workspace_splits_the_center_between_3d_and_canvas() {
        let mut state = state();
        state.switch_workspace(petunia_core::Workspace::Paint);
        let regions = run_shell(&mut state, egui::vec2(1_280.0, 800.0), 3);
        let viewport = regions.viewport.expect("a viewport 3D divide o centro");
        let canvas = regions.paint_canvas.expect("a tela 2D divide o centro");
        assert!(
            canvas.min.x >= viewport.min.x,
            "a tela fica à direita da viewport: {canvas:?} vs {viewport:?}"
        );
        assert!(
            regions.viewport_toolbar.is_some(),
            "a barra de contexto é chrome da coluna 3D"
        );
        assert!(regions.left_tools.is_some(), "paleta de pintura ausente");
        assert!(regions.right_dock.is_some(), "dock de camadas ausente");
        assert!(regions.status_overlaps().is_empty());
        assert!(regions.dock_sections_disjoint());
        assert!(regions.viewport_overlays_within_viewport());
    }

    #[test]
    fn the_shell_never_loses_a_region_on_a_narrow_window() {
        // O contrato da janela estreita: as regiões essenciais continuam
        // registradas e ninguém invade a barra de status.
        for width in [480.0, 640.0, 1_024.0, 1_920.0] {
            let mut state = state();
            let regions = run_shell(&mut state, egui::vec2(width, 720.0), 3);
            assert!(regions.left_tools.is_some(), "{width}px sem paleta");
            assert!(regions.right_dock.is_some(), "{width}px sem dock");
            assert!(regions.viewport.is_some(), "{width}px sem viewport");
            assert!(
                regions.status_overlaps().is_empty(),
                "{width}px invadiu a barra de status"
            );
            assert!(
                regions.dock_sections_disjoint(),
                "{width}px seções em conflito"
            );
        }
    }
}
