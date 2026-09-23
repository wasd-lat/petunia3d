//! Adapter de macro-layout do shell (`PetuniaLayoutAdapter` — §31, §47).
//!
//! Diretriz normativa: §31 (Macro-layout e egui_tiles), §14 (por que egui_tiles
//! e não egui_dock) e §47 (Wave 5 — Top Bar e shell).
//!
//! Este é o **único** arquivo autorizado a mencionar `egui_tiles::`
//! (verificado por `cargo xtask ui-guard --strict`). O produto fala de
//! [`PetuniaPane`] e [`PetuniaShellLayout`]; a árvore da crate é detalhe.
//!
//! ## O que este adapter decide (e o que ele não decide)
//!
//! ```text
//! PetuniaShellLayout        DTO Petunia (o contrato durável)
//!          ↓
//! PetuniaLayoutAdapter      árvore de runtime + regras do shell
//!          ↓
//! egui_tiles::Tree          mecânica de layout (splits, resize, divisórias)
//! ```
//!
//! O adapter é dono de:
//!
//! * topologia (esquerda · centro · dock · inferior);
//! * mínimos (viewport, laterais, dock inferior, paine);
//! * **política do dock** (AUTO pelo conteúdo, MANUAL pela fração, colapso);
//! * quais panes podem ser fechados ou arrastados (**nenhum**, §31.2);
//! * quais containers podem redimensionar;
//! * tradução de/para pixels do DTO.
//!
//! O produto continua dono de: quais paines existem por workspace, o que cada
//! paine desenha, e o estado persistido (nada de `Tree` serializado — §31.3).

use egui::{Rect, Ui, Vec2};
use egui_tiles::{Behavior, Container, Linear, LinearDir, Shares, Tile, TileId, Tree, UiResponse};
use petunia_core::{DockOrientation, DockSide, Workspace};

use crate::foundation::spacing;
use crate::regions;
use crate::tokens;

/// Menor largura em que a coluna de ferramentas ainda é utilizável.
pub const MIN_LEFT_WIDTH: f32 = tokens::TOOLBAR_MIN_WIDTH;
/// Maior largura da coluna de ferramentas (política herdada do `Panel::left`).
pub const MAX_LEFT_WIDTH: f32 = tokens::TOOLBAR_MAX_WIDTH;
/// Menor largura do dock de contexto (mesmo piso do documento de workspace).
pub const MIN_RIGHT_WIDTH: f32 = tokens::PROPERTIES_MIN_WIDTH;
/// Maior largura do dock de contexto (45% da janela, dentro da faixa do cap. 36).
pub const MAX_RIGHT_WIDTH: f32 = tokens::PROPERTIES_MAX_WIDTH;
/// Faixa de tolerância do duplo-clique na divisória do dock, em pixels.
const SPLIT_BAND: f32 = 6.0;
/// Menor altura do dock inferior.
pub const MIN_BOTTOM_HEIGHT: f32 = 72.0;
/// Piso do centro: a viewport nunca fica menor que isto (§31.2, viewport-first).
pub const MIN_VIEWPORT_WIDTH: f32 = 240.0;
/// Ver [`MIN_VIEWPORT_WIDTH`].
pub const MIN_VIEWPORT_HEIGHT: f32 = 180.0;
/// Menor lado de um paine (piso do motor de layout, não do produto).
pub const MIN_PANE_SIZE: f32 = regions::DOCK_STRIP_W;

/// Painel canônico do shell.
///
/// O conjunto espelha exatamente as regiões que o produto já tem: paleta de
/// ferramentas, viewport, cabeçalho do dock, árvore Parts/Scene, inspector e
/// dock inferior. O Asset Library é um drawer flutuante (overlay), não um paine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PetuniaPane {
    /// Paleta de ferramentas (esquerda).
    Tools,
    /// Área central: viewport 3D ou editor UV.
    Viewport,
    /// Tela 2D de pintura, ao lado da viewport no centro (workspace PAINT).
    ///
    /// É um paine de primeira classe, não um `Panel::right` do produto: a
    /// divisória, os mínimos das superfícies no modo Split e a largura persistida saem
    /// daqui, junto com o resto do macro-layout (§31).
    PaintCanvas,
    /// Cabeçalho do dock de contexto (lado + orientação).
    DockHeader,
    /// Árvore Parts/Scene (dock).
    Parts,
    /// Inspector/Properties (dock).
    Context,
    /// Dock inferior (timeline). Fora da V1 sem a feature de animação.
    Bottom,
}

impl PetuniaPane {
    /// Todos os paines, na ordem canônica do shell.
    pub const ALL: [PetuniaPane; 7] = [
        PetuniaPane::Tools,
        PetuniaPane::Viewport,
        PetuniaPane::PaintCanvas,
        PetuniaPane::DockHeader,
        PetuniaPane::Parts,
        PetuniaPane::Context,
        PetuniaPane::Bottom,
    ];

    /// Chave estável (telemetria, i18n e testes). Não é texto de UI.
    pub const fn key(self) -> &'static str {
        match self {
            PetuniaPane::Tools => "tools",
            PetuniaPane::Viewport => "viewport",
            PetuniaPane::PaintCanvas => "paint-canvas",
            PetuniaPane::DockHeader => "dock-header",
            PetuniaPane::Parts => "parts",
            PetuniaPane::Context => "context",
            PetuniaPane::Bottom => "bottom",
        }
    }

    /// Slot autorizado do paine (§31.2: "plugin panels só entram em slots
    /// permitidos"). Um paine fora do seu slot não é construído.
    pub const fn slot(self) -> PetuniaPaneSlot {
        match self {
            PetuniaPane::Tools => PetuniaPaneSlot::Left,
            PetuniaPane::Viewport | PetuniaPane::PaintCanvas => PetuniaPaneSlot::Center,
            PetuniaPane::DockHeader | PetuniaPane::Parts | PetuniaPane::Context => {
                PetuniaPaneSlot::Right
            }
            PetuniaPane::Bottom => PetuniaPaneSlot::Bottom,
        }
    }

    /// Paine que não pode ser destruído nem movido pela UI do usuário.
    ///
    /// Na V1 **todos** os paines são núcleo (a lista de paines é o produto); a
    /// função existe para que a regra seja explícita e testada, e para que o dia
    /// em que um paine de plugin entrar a exceção seja uma linha aqui.
    pub const fn is_core(self) -> bool {
        !matches!(self, PetuniaPane::Bottom)
    }

    /// O paine pode ser fechado pelo usuário? Nunca, na V1.
    pub const fn is_closable(self) -> bool {
        false
    }

    /// O paine pode ser arrastado para outro lugar? Nunca (sem docking livre).
    pub const fn is_draggable(self) -> bool {
        false
    }

    /// Presença obrigatória em qualquer workspace.
    pub const fn is_mandatory(self) -> bool {
        matches!(
            self,
            PetuniaPane::Tools | PetuniaPane::Viewport | PetuniaPane::DockHeader
        )
    }

    /// `true` quando o paine pertence à coluna do dock de contexto.
    pub const fn is_dock(self) -> bool {
        matches!(
            self,
            PetuniaPane::DockHeader | PetuniaPane::Parts | PetuniaPane::Context
        )
    }
}

/// Região da árvore onde um paine pode viver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetuniaPaneSlot {
    /// Coluna esquerda.
    Left,
    /// Centro (viewport-first: nunca espremido abaixo do mínimo).
    Center,
    /// Dock de contexto.
    Right,
    /// Dock inferior.
    Bottom,
}

impl PetuniaPaneSlot {
    /// `true` quando o paine pertence a este slot.
    pub const fn allows(self, pane: PetuniaPane) -> bool {
        matches!(
            (self, pane),
            (PetuniaPaneSlot::Left, PetuniaPane::Tools)
                | (PetuniaPaneSlot::Center, PetuniaPane::Viewport)
                | (PetuniaPaneSlot::Center, PetuniaPane::PaintCanvas)
                | (PetuniaPaneSlot::Right, PetuniaPane::DockHeader)
                | (PetuniaPaneSlot::Right, PetuniaPane::Parts)
                | (PetuniaPaneSlot::Right, PetuniaPane::Context)
                | (PetuniaPaneSlot::Bottom, PetuniaPane::Bottom)
        )
    }
}

/// DTO do layout do shell (§31.3).
///
/// É o contrato durável: a árvore da crate é reconstruída a partir daqui em
/// cada frame, e o que o usuário mexe (larguras, divisor, colapsos) volta para
/// o estado Petunia. Nada de `egui_tiles::Tree` serializado.
#[derive(Debug, Clone, PartialEq)]
pub struct PetuniaShellLayout {
    /// Workspace corrente (a árvore nunca cruza workspaces).
    pub workspace: Workspace,
    /// Lado do dock de contexto.
    pub dock_side: DockSide,
    /// Largura da coluna de ferramentas, em pixels.
    pub left_width: f32,
    /// Largura do dock de contexto, em pixels.
    pub right_width: f32,
    /// Altura do dock inferior, em pixels.
    pub bottom_height: f32,
    /// Fração do dock de contexto ocupada pela primeira seção (Parts).
    pub right_dock_split: f32,
    /// Disposição do dock de contexto.
    pub right_dock_orientation: DockOrientation,
    /// Dimensionamento automático do dock pelo conteúdo.
    pub dock_auto: bool,
    /// Altura estimada do conteúdo de Parts (usada quando [`Self::dock_auto`]).
    pub parts_content_height: f32,
    /// O centro divide o espaço entre a viewport 3D e a tela 2D (workspace PAINT).
    pub canvas_enabled: bool,
    /// Largura desejada do pano da tela 2D, em pixels.
    ///
    /// O adapter prende à faixa de tokens ([`tokens::PAINT_CANVAS_MIN_WIDTH`]
    /// ..=[`tokens::PAINT_CANVAS_MAX_WIDTH`]) e cede primeiro para a viewport
    /// continuar com [`MIN_VIEWPORT_WIDTH`] — o produto nunca valida este valor.
    pub canvas_width: f32,
    /// Há dock inferior neste workspace (V1: só com a feature de animação).
    pub bottom_enabled: bool,
    /// Seção Parts colapsada (fica só o cabeçalho).
    pub parts_collapsed: bool,
    /// Seção Context colapsada (fica só a barra do objeto).
    pub context_collapsed: bool,
    /// Inspector destacado (janela flutuante) — a seção sai da árvore.
    pub context_detached: bool,
}

impl Default for PetuniaShellLayout {
    fn default() -> Self {
        Self {
            workspace: Workspace::Model,
            dock_side: DockSide::Right,
            left_width: tokens::TOOLBAR_MIN_WIDTH,
            right_width: tokens::PROPERTIES_DEFAULT_WIDTH,
            bottom_height: tokens::TIMELINE_HEIGHT,
            right_dock_split: 0.42,
            right_dock_orientation: DockOrientation::Stacked,
            dock_auto: true,
            parts_content_height: 240.0,
            canvas_enabled: false,
            canvas_width: tokens::PAINT_CANVAS_DEFAULT_WIDTH,
            bottom_enabled: false,
            parts_collapsed: false,
            context_collapsed: false,
            context_detached: false,
        }
    }
}

impl PetuniaShellLayout {
    /// `true` quando o paine participa do layout neste frame.
    ///
    /// Colapsar **não** é esconder: as duas seções do dock continuam na árvore,
    /// com a altura/largura de uma tira de cabeçalho. O controle de reabrir mora
    /// no cabeçalho da própria seção (`outliner_collapsed` é o chevron do
    /// Outliner, `inspector_collapsed` é a barra do objeto); um paine de tamanho
    /// zero — ou fora do layout — perderia esse controle para sempre.
    ///
    /// O destacamento do inspector é a única exceção: o conteúdo de fato passa a
    /// viver numa janela flutuante.
    pub fn is_visible(&self, pane: PetuniaPane) -> bool {
        match pane {
            PetuniaPane::Tools
            | PetuniaPane::Viewport
            | PetuniaPane::DockHeader
            | PetuniaPane::Parts => true,
            PetuniaPane::PaintCanvas => self.canvas_enabled,
            PetuniaPane::Context => !self.context_detached,
            PetuniaPane::Bottom => self.bottom_enabled,
        }
    }

    /// Aplica os mínimos do produto **antes** de a árvore nascer.
    ///
    /// O centro tem prioridade (viewport-first): as laterais e o dock inferior
    /// são encolhidos até os seus próprios mínimos antes de a viewport ficar
    /// abaixo de [`MIN_VIEWPORT_WIDTH`] / [`MIN_VIEWPORT_HEIGHT`].
    ///
    /// As larguras também são presas às faixas documentadas ([`MIN_LEFT_WIDTH`]
    /// ..=[`MAX_LEFT_WIDTH`] e o mínimo do conteúdo real ..= 45% da janela): o
    /// adapter é o dono único dos thresholds de layout (§24), então o valor
    /// persistido nunca precisa ser validado pelo produto.
    ///
    /// Função pura: nenhum `Ui`, nenhuma árvore — é a regra "min viewport" do
    /// §47 verificável sem montar o shell.
    pub fn clamped(&self, available: Vec2) -> Self {
        let mut out = self.clone();
        let available_w = available.x.max(0.0);
        let available_h = available.y.max(0.0);

        // Eixo horizontal: esquerda + direita + centro.
        out.left_width = out.left_width.clamp(MIN_LEFT_WIDTH, MAX_LEFT_WIDTH);
        let dock_floor = self.dock_min_width();
        let dock_ceiling = (available_w * 0.45)
            .clamp(MIN_RIGHT_WIDTH, MAX_RIGHT_WIDTH)
            .max(dock_floor);
        out.right_width = out.right_width.clamp(dock_floor, dock_ceiling);
        // Centro em Split: o piso cresce com a tela 2D —
        // viewport-first também aqui — e a largura da tela é presa à faixa dos
        // tokens antes de qualquer conta, para o produto nunca validar o valor.
        let center_floor = if out.canvas_enabled {
            MIN_VIEWPORT_WIDTH + spacing::TIGHT + tokens::PAINT_CANVAS_MIN_WIDTH
        } else {
            MIN_VIEWPORT_WIDTH
        };
        out.canvas_width = out.canvas_width.clamp(
            tokens::PAINT_CANVAS_MIN_WIDTH,
            tokens::PAINT_CANVAS_MAX_WIDTH,
        );
        let mut overflow = out.left_width + out.right_width + center_floor - available_w;
        if overflow > 0.0 {
            // Encolhe primeiro o dock de contexto, depois a coluna de ferramentas,
            // nunca abaixo dos mínimos.
            let shrink = (out.right_width - dock_floor).max(0.0).min(overflow);
            out.right_width -= shrink;
            overflow -= shrink;
            let shrink = (out.left_width - MIN_LEFT_WIDTH).max(0.0).min(overflow);
            out.left_width -= shrink;
        }
        if out.canvas_enabled {
            // Sobrou menos que o mínimo das superfícies: a tela cede até o
            // próprio piso em vez de comer a viewport.
            let center_room =
                (available_w - out.left_width - out.right_width - spacing::TIGHT).max(0.0);
            out.canvas_width = out
                .canvas_width
                .min((center_room - MIN_VIEWPORT_WIDTH).max(tokens::PAINT_CANVAS_MIN_WIDTH));
        }

        // Eixo vertical: centro + dock inferior.
        if out.bottom_enabled {
            let overflow = MIN_VIEWPORT_HEIGHT + out.bottom_height - available_h;
            if overflow > 0.0 {
                out.bottom_height =
                    (out.bottom_height - overflow).max(MIN_BOTTOM_HEIGHT.min(available_h));
            }
        }
        out.right_dock_split = out.right_dock_split.clamp(0.2, 0.8);
        out
    }

    /// Larguras de caixa do eixo horizontal: `(esquerda, centro, direita)`.
    ///
    /// O centro recebe o resto — é a definição de viewport-first. A coluna do
    /// dock sempre existe (o cabeçalho dela é o controle de lado/orientação do
    /// dock), então a largura dele nunca é zero.
    pub fn horizontal_widths(&self, available_w: f32) -> (f32, f32, f32) {
        let layout = self.clamped(egui::vec2(available_w, f32::MAX));
        let left = layout.left_width;
        let right = layout.right_width;
        let center = (available_w - left - right).max(MIN_VIEWPORT_WIDTH);
        (left, center, right)
    }

    /// Larguras do centro: `(viewport, tela 2D)` no modo Split.
    ///
    /// A tela 2D recebe a largura desejada presa entre o próprio mínimo e o que
    /// sobra depois de a viewport ficar com [`MIN_VIEWPORT_WIDTH`]; com a tela
    /// desabilitada o centro é uma superfície só (`canvas == 0`).
    ///
    /// Invariante: `viewport + canvas == center_w` — a soma nunca "sobra" para o
    /// conteúdo, que então alargaria o painel ~1px por frame (mesma regra de
    /// [`Self::dock_pane_sizes`]).
    pub fn center_widths(&self, center_w: f32) -> (f32, f32) {
        let center_w = center_w.max(0.0);
        if !self.canvas_enabled {
            return (center_w, 0.0);
        }
        let gap = spacing::TIGHT;
        // A tela cede primeiro: o teto é o que sobra depois da viewport, então
        // numa janela apertada (`ceiling < mínimo da tela`) ela sai menor que o
        // próprio mínimo em vez de esmagar a viewport — a soma continua sendo a
        // do centro, como no dock.
        let ceiling = (center_w - MIN_VIEWPORT_WIDTH - gap).max(0.0);
        let canvas = self
            .canvas_width
            .clamp(
                tokens::PAINT_CANVAS_MIN_WIDTH,
                tokens::PAINT_CANVAS_MAX_WIDTH,
            )
            .min(ceiling);
        ((center_w - canvas).max(0.0), canvas)
    }

    /// Tamanhos das duas seções do dock, em pixels, na orientação corrente.
    ///
    /// Migra a política que vivia no produto (`scene_panel_height` +
    /// `split_widths`): o adapter é o lugar único dos thresholds de layout (§24).
    ///
    /// * **AUTO** (`dock_auto`, só no empilhado): a primeira seção dimensiona
    ///   pelo conteúdo estimado, presa entre `96px` e 38% do espaço — sem
    ///   reservar meio dock para um objeto só.
    /// * **MANUAL**: a fração do divisor decide.
    /// * **Colapsada**: a seção cai para a tira do próprio cabeçalho.
    ///
    /// Empilhado divide altura (piso = [`regions::DOCK_HEADER_H`]); lado a lado
    /// divide largura respeitando os mínimos reais das duas seções
    /// ([`regions::DOCK_MIN_OUTLINER_W`], [`regions::DOCK_MIN_INSPECTOR_W`]).
    ///
    /// Invariante: `first + second == total` sempre — a soma nunca "sobra" para
    /// o conteúdo, que então alargaria o painel ~1px por frame.
    pub fn dock_pane_sizes(&self, available: Vec2) -> (f32, f32) {
        let stacked = self.right_dock_orientation == DockOrientation::Stacked;
        let total = if stacked { available.y } else { available.x }.max(0.0);
        let (min_first, min_second) = if stacked {
            (regions::DOCK_HEADER_H, regions::DOCK_HEADER_H)
        } else {
            (regions::DOCK_MIN_OUTLINER_W, regions::DOCK_MIN_INSPECTOR_W)
        };

        // Espaço insuficiente para os dois mínimos: divide igualmente em vez de
        // produzir uma seção negativa (o conteúdo de cada uma encolhe, mas as
        // duas continuam alcançáveis).
        if total <= min_first + min_second {
            let half = (total * 0.5).max(0.0);
            return (half, total - half);
        }

        let desired = if self.parts_collapsed {
            min_first
        } else if self.context_collapsed || self.context_detached {
            total - min_second
        } else if stacked && self.dock_auto {
            self.parts_content_height
                .clamp(96.0, (total * 0.38).max(96.0))
        } else {
            total * self.right_dock_split
        };
        let first = desired.clamp(min_first, total - min_second);
        (first, total - first)
    }

    /// Largura mínima do dock (colunas do conteúdo real + divisória).
    ///
    /// Empilhado só precisa da largura de uma coluna; lado a lado precisa das
    /// duas — nascer menor faria a seção se alargar sozinha todo frame.
    pub fn dock_min_width(&self) -> f32 {
        if self.right_dock_orientation == DockOrientation::SideBySide {
            regions::DOCK_MIN_OUTLINER_W + regions::DOCK_MIN_INSPECTOR_W + regions::DOCK_SEPARATOR_W
        } else {
            MIN_RIGHT_WIDTH
        }
    }
}

/// Contexto de desenho entregue pelo adapter.
///
/// Os rótulos vêm do produto (i18n); o adapter não conhece strings.
pub struct PetuniaLayoutContext<'a> {
    /// Título de um paine (cabeçalho/aba).
    pub label: &'a dyn Fn(PetuniaPane) -> String,
    /// Desenho do conteúdo de um paine.
    pub draw: &'a mut dyn FnMut(&mut Ui, PetuniaPane),
}

/// O que o produto deve aplicar de volta no estado após o frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PetuniaLayoutResponse {
    /// Largura final da coluna de ferramentas.
    pub left_width: Option<f32>,
    /// Largura final do dock de contexto.
    pub right_width: Option<f32>,
    /// Altura final do dock inferior.
    pub bottom_height: Option<f32>,
    /// Fração final da primeira seção do dock (só quando o usuário mexeu).
    pub right_dock_split: Option<f32>,
    /// Largura final da tela 2D no centro (só quando o workspace a habilita).
    pub canvas_width: Option<f32>,
    /// `true` quando o divisor foi arrastado neste frame.
    pub resized: bool,
    /// Duplo-clique na divisória do dock: o produto lê como "voltar ao
    /// dimensionamento automático" (gesto documentado do dock).
    pub split_equalize: bool,
    /// Retângulo final de cada paine (fonte única das regiões do shell).
    pub pane_rects: Vec<(PetuniaPane, Rect)>,
}

impl PetuniaLayoutResponse {
    /// Retângulo final de um paine, se ele participou do layout.
    pub fn rect(&self, pane: PetuniaPane) -> Option<Rect> {
        self.pane_rects
            .iter()
            .find(|(candidate, _)| *candidate == pane)
            .map(|(_, rect)| *rect)
    }

    /// Retângulo que cobre a coluna do dock (cabeçalho + seções visíveis).
    pub fn dock_rect(&self) -> Option<Rect> {
        let mut union: Option<Rect> = None;
        for (pane, rect) in &self.pane_rects {
            if !pane.is_dock() {
                continue;
            }
            union = Some(match union {
                Some(acc) => acc.union(*rect),
                None => *rect,
            });
        }
        union
    }
}

/// Adapter de macro-layout do shell.
#[derive(Debug, Clone, Copy)]
pub struct PetuniaLayoutAdapter {
    id: egui::Id,
}

impl PetuniaLayoutAdapter {
    /// Cria o adapter. O `id` é a raiz dos ids internos da árvore.
    pub fn new(id: impl Into<egui::Id>) -> Self {
        Self { id: id.into() }
    }

    /// Id da memória da largura da tela 2D desta árvore.
    fn canvas_memory(&self) -> egui::Id {
        self.id.with("canvas_width")
    }

    /// Largura da tela 2D a usar neste frame, quando o usuário já a arrastou.
    ///
    /// `UiState` (core) não carrega esta preferência, então ela vive na memória
    /// do `Context` ao lado do estado das outras superfícies do shell — mesmo
    /// contrato de `adapters::popup`. O DTO continua sendo o contrato do frame;
    /// um campo em `UiState` é a evolução natural quando o core for tocado.
    pub fn canvas_width(&self, ctx: &egui::Context) -> Option<f32> {
        ctx.data(|d| d.get_temp::<f32>(self.canvas_memory()))
    }

    /// Persiste a largura arrastada da tela 2D (memória de [`Self::canvas_width`]).
    pub fn remember_canvas_width(&self, ctx: &egui::Context, width: f32) {
        ctx.data_mut(|d| d.insert_temp(self.canvas_memory(), width));
    }

    /// Constrói a árvore de runtime a partir do DTO.
    ///
    /// A topologia é fixa por construção — é isso que impede docking livre:
    ///
    /// ```text
    /// vertical
    /// ├── horizontal
    /// │   ├── Tools
    /// │   ├── Viewport
    /// │   │   └── horizontal     (só com `canvas_enabled`, modo Split de PAINT)
    /// │   │       └── PaintCanvas
    /// │   └── vertical            (coluna do dock)
    /// │       ├── DockHeader
    /// │       └── vertical|horizontal
    /// │           ├── Parts
    /// │           └── Context
    /// └── Bottom
    /// ```
    ///
    /// O dock pode nascer à esquerda (`dock_side`): a ordem dentro do container
    /// horizontal muda, a topologia não. Em Split, a tela 2D vive **dentro** do
    /// centro (à direita da viewport), sem atravessar a coluna do dock.
    pub fn tree(&self, layout: &PetuniaShellLayout, available: Vec2) -> Tree<PetuniaPane> {
        let layout = layout.clamped(available);
        let (left_w, center_w, right_w) = layout.horizontal_widths(available.x);

        let mut tree = Tree::empty(self.id);
        let tools = tree.tiles.insert_pane(PetuniaPane::Tools);
        let viewport = tree.tiles.insert_pane(PetuniaPane::Viewport);
        let canvas = tree.tiles.insert_pane(PetuniaPane::PaintCanvas);
        let header = tree.tiles.insert_pane(PetuniaPane::DockHeader);
        let parts = tree.tiles.insert_pane(PetuniaPane::Parts);
        let context = tree.tiles.insert_pane(PetuniaPane::Context);
        let bottom = tree.tiles.insert_pane(PetuniaPane::Bottom);

        // Seções do dock: px → share (o motor normaliza pela soma, então todas
        // as colunas recebem valor explícito). O cabeçalho consome a própria
        // tira, então as seções dividem o que sobra dele.
        let dock_room = match layout.right_dock_orientation {
            DockOrientation::Stacked => {
                egui::vec2(right_w, (available.y - regions::DOCK_HEADER_H).max(0.0))
            }
            DockOrientation::SideBySide => egui::vec2(right_w, available.y),
        };
        let (first, second) = layout.dock_pane_sizes(dock_room);
        let inner_dir = match layout.right_dock_orientation {
            DockOrientation::Stacked => LinearDir::Vertical,
            DockOrientation::SideBySide => LinearDir::Horizontal,
        };
        let mut inner = Linear::new(inner_dir, vec![parts, context]);
        set_share(&mut inner.shares, parts, first);
        set_share(&mut inner.shares, context, second);
        let inner = tree.tiles.insert_container(inner);

        let mut dock = Linear::new(LinearDir::Vertical, vec![header, inner]);
        set_share(&mut dock.shares, header, regions::DOCK_HEADER_H);
        set_share(&mut dock.shares, inner, second.max(0.0));
        let dock = tree.tiles.insert_container(dock);

        // Centro: viewport + tela 2D lado a lado quando o modo Split está ativo
        // (a tela fica **à direita** da viewport). O container
        // existe sempre — com a tela desabilitada ela fica invisível e com share
        // zero, em vez de sair da árvore: um paine que entra e sai do layout é um
        // paine que perde o próprio controle de reabrir.
        //
        // As duas larguras vêm da mesma política pura (`center_widths`), então a
        // soma é sempre a do centro.
        let (viewport_w, canvas_w) = layout.center_widths(center_w);
        let center_span = (viewport_w + canvas_w).max(f32::MIN_POSITIVE);
        let mut center_row = Linear::new(LinearDir::Horizontal, vec![viewport, canvas]);
        set_share(&mut center_row.shares, viewport, viewport_w / center_span);
        set_share(&mut center_row.shares, canvas, canvas_w / center_span);
        let center = tree.tiles.insert_container(center_row);

        let columns = match layout.dock_side {
            DockSide::Right => vec![tools, center, dock],
            DockSide::Left => vec![tools, dock, center],
        };
        let mut row = Linear::new(LinearDir::Horizontal, columns);
        let span = available
            .x
            .max(left_w + center_w + right_w)
            .max(f32::MIN_POSITIVE);
        set_share(&mut row.shares, tools, left_w / span);
        set_share(&mut row.shares, center, center_w / span);
        set_share(&mut row.shares, dock, right_w / span);
        let row = tree.tiles.insert_container(row);

        let bottom_h = if layout.bottom_enabled {
            layout.bottom_height
        } else {
            0.0
        };
        let content_h = (available.y - bottom_h).max(MIN_VIEWPORT_HEIGHT);
        let mut root = Linear::new(LinearDir::Vertical, vec![row, bottom]);
        set_share(&mut root.shares, row, content_h);
        set_share(&mut root.shares, bottom, bottom_h);
        let root = tree.tiles.insert_container(root);
        tree.root = Some(root);

        for pane in PetuniaPane::ALL {
            if let Some(tile) = tree.tiles.find_pane(&pane) {
                tree.tiles.set_visible(tile, layout.is_visible(pane));
            }
        }
        tree
    }

    /// Desenha a árvore e devolve o que o produto deve persistir.
    pub fn show(
        &self,
        ui: &mut Ui,
        layout: &PetuniaShellLayout,
        ctx: PetuniaLayoutContext<'_>,
    ) -> PetuniaLayoutResponse {
        let mut tree = self.tree(layout, ui.available_size());
        let mut behavior = ShellBehavior {
            label: ctx.label,
            draw: ctx.draw,
            resized: false,
        };
        tree.ui(&mut behavior, ui);

        let rect_of = |pane: PetuniaPane| {
            tree.tiles
                .find_pane(&pane)
                .and_then(|id| tree.tiles.rect(id))
        };
        let pane_rects: Vec<(PetuniaPane, Rect)> = PetuniaPane::ALL
            .iter()
            .filter_map(|pane| rect_of(*pane).map(|rect| (*pane, rect)))
            .collect();
        let dock_rect = tree
            .tiles
            .find_pane(&PetuniaPane::DockHeader)
            .and_then(|header| tree.tiles.parent_of(header))
            .and_then(|dock| tree.tiles.rect(dock));

        // Duplo-clique na divisória: derivado dos retângulos reais das duas
        // seções, não de coordenadas fixas (§31 — o produto não faz hit-test).
        let split_equalize = double_click_on_split(ui, &pane_rects);

        // Divisor arrastado: a fração final volta em pixels/fração para o DTO.
        let split = if behavior.resized {
            tree.tiles
                .find_pane(&PetuniaPane::Parts)
                .and_then(|parts| tree.tiles.parent_of(parts))
                .and_then(|inner| tree.tiles.get_container(inner))
                .and_then(|container| match container {
                    Container::Linear(linear) => {
                        let parts = tree.tiles.find_pane(&PetuniaPane::Parts)?;
                        let context = tree.tiles.find_pane(&PetuniaPane::Context)?;
                        let a = share_of(&linear.shares, parts);
                        let b = share_of(&linear.shares, context);
                        (a + b > 0.0).then_some(a / (a + b))
                    }
                    _ => None,
                })
        } else {
            None
        };

        PetuniaLayoutResponse {
            left_width: rect_of(PetuniaPane::Tools).map(|rect| rect.width()),
            right_width: dock_rect.map(|rect| rect.width()),
            bottom_height: rect_of(PetuniaPane::Bottom).map(|rect| rect.height()),
            right_dock_split: split,
            canvas_width: rect_of(PetuniaPane::PaintCanvas).map(|rect| rect.width()),
            resized: behavior.resized,
            split_equalize,
            pane_rects,
        }
    }
}

/// Retângulo de um paine na lista do frame corrente.
fn pane_rect(rects: &[(PetuniaPane, Rect)], pane: PetuniaPane) -> Option<Rect> {
    rects
        .iter()
        .find(|(candidate, _)| *candidate == pane)
        .map(|(_, rect)| *rect)
}

/// Duplo-clique primário sobre a divisória entre as duas seções do dock.
///
/// A faixa de tolerância é centrada na fronteira **medida** das duas seções (a
/// orientação vem do eixo em que os centros mais se afastam), então funciona
/// igual no dock empilhado e no lado a lado, e com o dock à esquerda.
fn double_click_on_split(ui: &Ui, rects: &[(PetuniaPane, Rect)]) -> bool {
    let (Some(parts), Some(context)) = (
        pane_rect(rects, PetuniaPane::Parts),
        pane_rect(rects, PetuniaPane::Context),
    ) else {
        return false;
    };
    let (double, pointer) = ui.input(|i| {
        (
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary),
            i.pointer.interact_pos(),
        )
    });
    if !double {
        return false;
    }
    let Some(pointer) = pointer else {
        return false;
    };
    let side_by_side = (parts.center().x - context.center().x).abs()
        > (parts.center().y - context.center().y).abs();
    let band = if side_by_side {
        let x = (parts.max.x + context.min.x) * 0.5;
        let y0 = parts.min.y.min(context.min.y);
        let y1 = parts.max.y.max(context.max.y);
        Rect::from_min_max(
            egui::pos2(x - SPLIT_BAND, y0),
            egui::pos2(x + SPLIT_BAND, y1),
        )
    } else {
        let y = (parts.max.y + context.min.y) * 0.5;
        let x0 = parts.min.x.min(context.min.x);
        let x1 = parts.max.x.max(context.max.x);
        Rect::from_min_max(
            egui::pos2(x0, y - SPLIT_BAND),
            egui::pos2(x1, y + SPLIT_BAND),
        )
    };
    band.contains(pointer)
}

/// Lê o share atual de um filho (0.0 quando o container não o conhece).
fn share_of(shares: &Shares, tile: TileId) -> f32 {
    shares
        .iter()
        .find(|(id, _)| **id == tile)
        .map(|(_, value)| *value)
        .unwrap_or(0.0)
}

/// Define o share de um filho em pixels.
///
/// `Shares::split` divide a largura disponível **proporcionalmente à soma** das
/// shares; um filho sem valor explícito fica com o default (`1.0`) e domina o
/// container. Por isso todo filho construído aqui recebe o seu.
fn set_share(shares: &mut Shares, tile: TileId, value: f32) {
    shares.set_share(tile, value.max(0.0));
}

/// Paine que não pode ser fechado nem arrastado; containers só redimensionam
/// quando fazem parte da topologia autorizada.
struct ShellBehavior<'a> {
    label: &'a dyn Fn(PetuniaPane) -> String,
    draw: &'a mut dyn FnMut(&mut Ui, PetuniaPane),
    resized: bool,
}

impl Behavior<PetuniaPane> for ShellBehavior<'_> {
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: TileId, pane: &mut PetuniaPane) -> UiResponse {
        (self.draw)(ui, *pane);
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &PetuniaPane) -> egui::WidgetText {
        (self.label)(*pane).into()
    }

    /// Núcleo do shell não fecha (§31.2).
    fn is_tab_closable(&self, _tiles: &egui_tiles::Tiles<PetuniaPane>, _tile_id: TileId) -> bool {
        false
    }

    /// Sem docking livre: nenhum paine é arrastável (§31.2).
    fn is_tile_draggable(&self, _tiles: &egui_tiles::Tiles<PetuniaPane>, _tile_id: TileId) -> bool {
        false
    }

    /// Só containers lineares (as colunas do shell) redimensionam; abas não.
    fn is_container_resizable(
        &self,
        tiles: &egui_tiles::Tiles<PetuniaPane>,
        tile_id: TileId,
    ) -> bool {
        matches!(
            tiles.get(tile_id),
            Some(Tile::Container(Container::Linear(_)))
        )
    }

    /// Nenhum paine some por GC silencioso.
    fn retain_pane(&mut self, _pane: &PetuniaPane) -> bool {
        true
    }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        regions::DOCK_HEADER_H
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        spacing::TIGHT
    }

    fn min_size(&self) -> f32 {
        MIN_PANE_SIZE
    }

    fn on_edit(&mut self, edit_action: egui_tiles::EditAction) {
        if edit_action == egui_tiles::EditAction::TileResized {
            self.resized = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> PetuniaShellLayout {
        PetuniaShellLayout {
            workspace: Workspace::Model,
            left_width: 56.0,
            right_width: 300.0,
            bottom_height: 160.0,
            right_dock_split: 0.42,
            right_dock_orientation: DockOrientation::Stacked,
            dock_auto: false,
            parts_content_height: 240.0,
            bottom_enabled: false,
            parts_collapsed: false,
            context_collapsed: false,
            context_detached: false,
            ..PetuniaShellLayout::default()
        }
    }

    fn run(adapter: &PetuniaLayoutAdapter, layout: &PetuniaShellLayout) -> PetuniaLayoutResponse {
        let ctx = egui::Context::default();
        let mut response = None;
        for _ in 0..3 {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1_280.0, 800.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                response = Some(adapter.show(
                    ui,
                    layout,
                    PetuniaLayoutContext {
                        label: &|pane| pane.key().to_string(),
                        draw: &mut |ui: &mut Ui, pane| {
                            ui.label(pane.key());
                        },
                    },
                ));
            });
            out.textures_delta.clear();
        }
        response.expect("o shell sempre produz resposta")
    }

    #[test]
    fn tree_contains_exactly_the_declared_panes() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let tree = adapter.tree(&layout(), egui::vec2(1_280.0, 800.0));
        for pane in PetuniaPane::ALL {
            assert!(
                tree.tiles.find_pane(&pane).is_some(),
                "pane {pane:?} ausente da árvore"
            );
        }
        assert_eq!(
            tree.tiles.len(),
            PetuniaPane::ALL.len() + 5,
            "5 containers (raiz, linha, centro, dock, seções)"
        );
    }

    #[test]
    fn core_panes_cannot_be_closed_and_nothing_can_be_dragged() {
        let mut behavior = ShellBehavior {
            label: &|pane| pane.key().to_string(),
            draw: &mut |_ui, _pane| {},
            resized: false,
        };
        let tiles: egui_tiles::Tiles<PetuniaPane> = egui_tiles::Tiles::default();
        for pane in PetuniaPane::ALL {
            assert!(!pane.is_closable(), "{pane:?} não pode ser fechável");
            assert!(!pane.is_draggable(), "{pane:?} não pode ser arrastável");
            assert!(
                behavior.retain_pane(&pane),
                "{pane:?} não pode ser coletado"
            );
        }
        for tile_id in [TileId::from_u64(1), TileId::from_u64(99)] {
            assert!(!behavior.is_tile_draggable(&tiles, tile_id));
        }
    }

    #[test]
    fn every_pane_lives_only_in_its_authorized_slot() {
        for pane in PetuniaPane::ALL {
            assert!(
                pane.slot().allows(pane),
                "{pane:?} fora do próprio slot {:?}",
                pane.slot()
            );
            for slot in [
                PetuniaPaneSlot::Left,
                PetuniaPaneSlot::Center,
                PetuniaPaneSlot::Right,
                PetuniaPaneSlot::Bottom,
            ] {
                if slot != pane.slot() {
                    assert!(!slot.allows(pane), "{pane:?} não pode entrar em {slot:?}");
                }
            }
        }
    }

    #[test]
    fn collapsed_sections_stay_in_the_tree_as_a_header() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let mut l = layout();
        l.parts_collapsed = true;
        let tree = adapter.tree(&l, egui::vec2(1_280.0, 800.0));
        let parts = tree.tiles.find_pane(&PetuniaPane::Parts).unwrap();
        assert!(
            tree.tiles.is_visible(parts),
            "colapsado não é invisível: a seção vira cabeçalho"
        );
        let (first, _second) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!(
            (first - regions::DOCK_HEADER_H).abs() < 0.01,
            "colapsado ocupa só o cabeçalho ({first})"
        );
    }

    #[test]
    fn detached_inspector_leaves_the_tree() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let mut l = layout();
        l.context_detached = true;
        let tree = adapter.tree(&l, egui::vec2(1_280.0, 800.0));
        let context = tree.tiles.find_pane(&PetuniaPane::Context).unwrap();
        assert!(!tree.tiles.is_visible_in_layout(context));
    }

    #[test]
    fn collapsing_a_section_keeps_it_reachable_as_a_strip() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let mut l = layout();
        l.context_collapsed = true;
        let tree = adapter.tree(&l, egui::vec2(1_280.0, 800.0));
        let context = tree.tiles.find_pane(&PetuniaPane::Context).unwrap();
        // Colapsar não esconde: o controle de reabrir (barra do objeto) vive no
        // próprio paine. Esconder mataria o gesto.
        assert!(tree.tiles.is_visible(context));
        assert!(tree.tiles.is_visible_in_layout(context));
        let (_parts, second) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!(
            (second - regions::DOCK_HEADER_H).abs() < 0.01,
            "colapsado ocupa só a tira do cabeçalho ({second})"
        );
    }

    #[test]
    fn min_viewport_wins_over_the_side_columns() {
        let l = PetuniaShellLayout {
            left_width: 400.0,
            right_width: 400.0,
            ..layout()
        };
        let clamped = l.clamped(egui::vec2(700.0, 600.0));
        let (left, center, right) = clamped.horizontal_widths(700.0);
        assert!(center >= MIN_VIEWPORT_WIDTH, "centro esmagado: {center}");
        assert!(left >= MIN_LEFT_WIDTH);
        assert!(right >= MIN_RIGHT_WIDTH);
        assert!(
            left + center + right <= 700.0 + 0.01,
            "layout estourou a largura: {left} + {center} + {right}"
        );
    }

    #[test]
    fn context_dock_width_matches_the_frozen_shell_baseline() {
        let layout = PetuniaShellLayout::default();
        assert_eq!(layout.right_width, 288.0);
        assert_eq!(MIN_RIGHT_WIDTH, 240.0);
        assert_eq!(MAX_RIGHT_WIDTH, 440.0);
        assert_eq!(petunia_core::PROPERTIES_DEFAULT_WIDTH, layout.right_width);
        assert_eq!(petunia_core::PROPERTIES_MIN_WIDTH, MIN_RIGHT_WIDTH);
        assert_eq!(petunia_core::PROPERTIES_MAX_WIDTH, MAX_RIGHT_WIDTH);
    }

    #[test]
    fn min_viewport_never_grows_beyond_a_tiny_window() {
        let l = PetuniaShellLayout {
            left_width: 300.0,
            right_width: 300.0,
            bottom_height: 400.0,
            bottom_enabled: true,
            ..layout()
        };
        let clamped = l.clamped(egui::vec2(320.0, 240.0));
        let (left, center, right) = clamped.horizontal_widths(320.0);
        assert!(left + center + right <= 320.0 + MIN_VIEWPORT_WIDTH);
        assert!(clamped.bottom_height <= 240.0);
        assert!(clamped.bottom_height >= MIN_BOTTOM_HEIGHT.min(240.0));
    }

    #[test]
    fn bottom_dock_only_exists_when_the_workspace_enables_it() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let tree = adapter.tree(&layout(), egui::vec2(1_280.0, 800.0));
        let bottom = tree.tiles.find_pane(&PetuniaPane::Bottom).unwrap();
        assert!(!tree.tiles.is_visible_in_layout(bottom));
        assert!(!layout().is_visible(PetuniaPane::Bottom));
    }

    #[test]
    fn dock_policy_auto_manual_and_side_by_side() {
        let mut l = layout();
        // Manual empilhado: a fração decide, deixando um cabeçalho para o irmão.
        l.dock_auto = false;
        l.right_dock_split = 0.3;
        let (a, b) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!((a - 210.0).abs() < 1.0, "esperado 30% de 700, veio {a}");
        assert!((a + b - 700.0).abs() < 0.01);
        l.right_dock_split = 0.7;
        let (c, _) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!(c > a + 20.0, "0.7 ({c}) precisa exceder 0.3 ({a})");

        // AUTO: dimensiona pelo conteúdo, preso em 38%.
        l.dock_auto = true;
        l.parts_content_height = 20.0;
        let (small, _) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!(small >= 96.0, "piso de conteúdo: {small}");
        l.parts_content_height = 10_000.0;
        let (big, _) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!((big - 700.0 * 0.38).abs() < 1.0, "teto de 38%: {big}");

        // Lado a lado: nunca abaixo dos mínimos reais das duas colunas.
        let side = PetuniaShellLayout {
            right_dock_orientation: DockOrientation::SideBySide,
            right_dock_split: 0.05,
            dock_auto: false,
            ..layout()
        };
        let (out_w, insp_w) = side.dock_pane_sizes(egui::vec2(600.0, 700.0));
        assert!(out_w >= regions::DOCK_MIN_OUTLINER_W);
        assert!(insp_w >= regions::DOCK_MIN_INSPECTOR_W);
    }

    #[test]
    fn real_frame_reports_pixel_widths_close_to_the_dto() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let l = layout();
        let response = run(&adapter, &l);
        let left = response.left_width.expect("largura da esquerda");
        let right = response.right_width.expect("largura do dock");
        assert!(
            (left - l.left_width).abs() < 24.0,
            "esquerda pedida {} medida {left}",
            l.left_width
        );
        assert!(
            (right - l.right_width).abs() < 40.0,
            "dock pedido {} medido {right}",
            l.right_width
        );
        // Toda seção **visível** devolve retângulo: é a fonte das regiões do
        // shell. Um paine fora do layout não tem retângulo nenhum (e por isso
        // não registra região nem é clicável).
        for pane in PetuniaPane::ALL {
            assert_eq!(
                response.rect(pane).is_some(),
                l.is_visible(pane),
                "{pane:?} divergiu da visibilidade declarada"
            );
        }
        assert!(
            response.dock_rect().is_some(),
            "coluna do dock sem retângulo"
        );
    }

    /// A tela 2D é do centro, à direita da viewport — nunca uma coluna do shell
    /// (ela não atravessa o dock) e nunca abaixo do próprio mínimo.
    #[test]
    fn paint_canvas_sits_right_of_the_viewport_inside_the_center() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let paint = PetuniaShellLayout {
            workspace: Workspace::Paint,
            canvas_enabled: true,
            ..layout()
        };
        let response = run(&adapter, &paint);
        let viewport = response
            .rect(PetuniaPane::Viewport)
            .expect("viewport no centro");
        let canvas = response
            .rect(PetuniaPane::PaintCanvas)
            .expect("tela 2D no centro");
        assert!(
            canvas.min.x >= viewport.max.x - 1.0,
            "a tela precisa nascer depois da viewport: {canvas:?} vs {viewport:?}"
        );
        assert!(
            viewport.width() >= MIN_VIEWPORT_WIDTH - 1.0,
            "viewport abaixo do mínimo: {viewport:?}"
        );
        assert!(
            canvas.width() >= tokens::PAINT_CANVAS_MIN_WIDTH - 1.0,
            "tela abaixo do mínimo: {canvas:?}"
        );
        assert!(
            (paint.canvas_width - response.canvas_width.unwrap_or_default()).abs() < 40.0,
            "largura medida longe da pedida"
        );
    }

    /// Sem o perfil de pintura a tela continua na árvore (para não perder o
    /// controle de reabrir), mas fora do layout e sem ocupar um pixel.
    #[test]
    fn the_canvas_pane_leaves_the_layout_when_the_workspace_does_not_paint() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let response = run(&adapter, &layout());
        assert!(
            response.rect(PetuniaPane::PaintCanvas).is_none(),
            "tela fora do layout não devolve retângulo"
        );
        let tree = adapter.tree(&layout(), egui::vec2(1_280.0, 800.0));
        let canvas = tree
            .tiles
            .find_pane(&PetuniaPane::PaintCanvas)
            .expect("o paine existe na árvore");
        assert!(!tree.tiles.is_visible_in_layout(canvas));
    }

    #[test]
    fn center_widths_always_add_up_and_protect_the_viewport() {
        let comfortable = MIN_VIEWPORT_WIDTH + spacing::TIGHT + tokens::PAINT_CANVAS_MIN_WIDTH;
        for total in [0.0, 200.0, 460.0, 900.0, 2_000.0] {
            for canvas_enabled in [false, true] {
                for canvas_width in [0.0, 120.0, 420.0, 5_000.0] {
                    let l = PetuniaShellLayout {
                        canvas_enabled,
                        canvas_width,
                        ..layout()
                    };
                    let (viewport, canvas) = l.center_widths(total);
                    assert!(
                        (viewport + canvas - total).abs() < 0.01,
                        "total {total}: {viewport} + {canvas}"
                    );
                    assert!(viewport >= 0.0 && canvas >= 0.0);
                    if canvas_enabled && total >= comfortable {
                        assert!(
                            canvas >= tokens::PAINT_CANVAS_MIN_WIDTH - 0.01,
                            "tela {canvas} abaixo do mínimo (total {total})"
                        );
                        assert!(
                            viewport >= MIN_VIEWPORT_WIDTH - 0.01,
                            "viewport {viewport} esmagada (total {total})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_canvas_never_starves_the_viewport_on_a_narrow_window() {
        let l = PetuniaShellLayout {
            canvas_enabled: true,
            canvas_width: tokens::PAINT_CANVAS_MAX_WIDTH,
            left_width: 400.0,
            right_width: 400.0,
            ..layout()
        };
        let clamped = l.clamped(egui::vec2(900.0, 700.0));
        let (left, center, right) = clamped.horizontal_widths(900.0);
        let (viewport, canvas) = clamped.center_widths(center);
        assert!(
            viewport >= MIN_VIEWPORT_WIDTH - 0.01,
            "a tela não pode comer a viewport: {viewport}"
        );
        assert!(canvas >= tokens::PAINT_CANVAS_MIN_WIDTH - 0.01, "{canvas}");
        assert!(canvas <= tokens::PAINT_CANVAS_MAX_WIDTH + 0.01);
        assert!(
            left + center + right <= 900.0 + 0.01,
            "layout estourou: {left} + {center} + {right}"
        );
    }

    #[test]
    fn dock_pane_sizes_always_add_up_to_the_available_space() {
        let orientations = [DockOrientation::Stacked, DockOrientation::SideBySide];
        for orientation in orientations {
            for total in [0.0, 40.0, 120.0, 300.0, 700.0, 1_600.0] {
                for (parts_collapsed, context_collapsed) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    for split in [0.05, 0.42, 0.9] {
                        let l = PetuniaShellLayout {
                            right_dock_orientation: orientation,
                            parts_collapsed,
                            context_collapsed,
                            right_dock_split: split,
                            ..layout()
                        };
                        let (first, second) = l.dock_pane_sizes(egui::vec2(total, total));
                        assert!(first >= 0.0 && second >= 0.0, "{first} / {second}");
                        assert!(
                            (first + second - total).abs() < 0.01,
                            "{orientation:?} total {total}: {first} + {second}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tight_docks_split_evenly_instead_of_going_negative() {
        // 300px não comporta as duas colunas mínimas do lado a lado (168+232).
        let l = PetuniaShellLayout {
            right_dock_orientation: DockOrientation::SideBySide,
            ..layout()
        };
        let (first, second) = l.dock_pane_sizes(egui::vec2(300.0, 700.0));
        assert!((first - 150.0).abs() < 0.01, "first = {first}");
        assert!((second - 150.0).abs() < 0.01, "second = {second}");
    }

    #[test]
    fn dock_column_follows_the_requested_side() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let right = run(&adapter, &layout());
        let left = run(
            &adapter,
            &PetuniaShellLayout {
                dock_side: DockSide::Left,
                ..layout()
            },
        );
        let tools_r = right.rect(PetuniaPane::Tools).unwrap();
        let dock_r = right.rect(PetuniaPane::Parts).unwrap();
        assert!(
            dock_r.min.x > tools_r.max.x,
            "dock à direita deveria nascer depois da toolbar: {dock_r:?} vs {tools_r:?}"
        );
        let tools_l = left.rect(PetuniaPane::Tools).unwrap();
        let dock_l = left.rect(PetuniaPane::Parts).unwrap();
        let viewport_l = left.rect(PetuniaPane::Viewport).unwrap();
        // A paleta de ferramentas continua na borda esquerda (ordem preservada
        // do shell original); o dock apenas troca de lado com a viewport.
        assert!(
            dock_l.min.x > tools_l.max.x,
            "a toolbar não pode ser deslocada pelo dock: {dock_l:?} vs {tools_l:?}"
        );
        assert!(
            dock_l.max.x <= viewport_l.min.x,
            "dock à esquerda deveria nascer antes da viewport: {dock_l:?} vs {viewport_l:?}"
        );
    }

    #[test]
    fn resize_is_authorized_only_for_the_shell_columns() {
        let adapter = PetuniaLayoutAdapter::new("shell");
        let tree = adapter.tree(&layout(), egui::vec2(1_280.0, 800.0));
        let behavior = ShellBehavior {
            label: &|pane| pane.key().to_string(),
            draw: &mut |_ui, _pane| {},
            resized: false,
        };
        let mut resizable_containers = 0;
        for (tile_id, tile) in tree.tiles.iter() {
            if matches!(tile, Tile::Container(Container::Linear(_))) {
                assert!(
                    behavior.is_container_resizable(&tree.tiles, *tile_id),
                    "coluna do shell precisa redimensionar"
                );
                resizable_containers += 1;
            }
            if matches!(tile, Tile::Container(Container::Tabs(_))) {
                assert!(!behavior.is_container_resizable(&tree.tiles, *tile_id));
            }
        }
        assert_eq!(
            resizable_containers, 5,
            "cinco colunas lineares no shell (raiz, linha, centro, dock, seções)"
        );
    }
}
