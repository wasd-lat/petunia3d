//! Regiões canônicas do shell UI (Wave 2 — §4.1).
//!
//! Fonte única da verdade para retângulos seguros de layout. Cada componente
//! registra seu próprio retângulo ao desenhar; overlays (shelf, gizmo, HUD) e
//! hit-testing leem daqui em vez de derivar da tela global.
//!
//! Ordem canônica de alocação (§4.2): Header → Viewport Toolbar → Status Bar →
//! Bottom Pane → Left Tools → Right Dock → Central Content/Viewport → overlays.
//! Painéis `top`/`bottom` não competem entre si; a ordem entre laterais e a
//! toolbar do viewport define que a toolbar vive só na área central (padrão
//! Blender: header do editor dentro da área do editor).

/// Slot de região do shell. Um escritor por slot (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionSlot {
    Header,
    ViewportToolbar,
    StatusBar,
    LeftTools,
    AssetBrowser,
    BottomDock,
    RightDock,
    RightOutliner,
    RightInspector,
    UvEditor,
    /// Tela 2D do workspace PAINT (centro, quando a aba 3D não está ativa).
    PaintCanvas,
    Viewport,
    Shelf,
    ToolProperties,
    PrimitiveCard,
}

/// Retângulos do shell no frame corrente. `None` = pane oculto neste frame.
#[derive(Debug, Clone, Default)]
pub struct UiRegions {
    pub header: Option<egui::Rect>,
    pub viewport_toolbar: Option<egui::Rect>,
    pub status_bar: Option<egui::Rect>,
    pub left_tools: Option<egui::Rect>,
    pub asset_browser: Option<egui::Rect>,
    pub bottom_dock: Option<egui::Rect>,
    pub right_dock: Option<egui::Rect>,
    pub right_outliner: Option<egui::Rect>,
    pub right_inspector: Option<egui::Rect>,
    pub uv_editor: Option<egui::Rect>,
    pub paint_canvas: Option<egui::Rect>,
    pub viewport: Option<egui::Rect>,
    pub shelf: Option<egui::Rect>,
    pub tool_properties: Option<egui::Rect>,
    pub primitive_card: Option<egui::Rect>,
}

impl UiRegions {
    fn slot_mut(&mut self, slot: RegionSlot) -> &mut Option<egui::Rect> {
        match slot {
            RegionSlot::Header => &mut self.header,
            RegionSlot::ViewportToolbar => &mut self.viewport_toolbar,
            RegionSlot::StatusBar => &mut self.status_bar,
            RegionSlot::LeftTools => &mut self.left_tools,
            RegionSlot::AssetBrowser => &mut self.asset_browser,
            RegionSlot::BottomDock => &mut self.bottom_dock,
            RegionSlot::RightDock => &mut self.right_dock,
            RegionSlot::RightOutliner => &mut self.right_outliner,
            RegionSlot::RightInspector => &mut self.right_inspector,
            RegionSlot::UvEditor => &mut self.uv_editor,
            RegionSlot::PaintCanvas => &mut self.paint_canvas,
            RegionSlot::Viewport => &mut self.viewport,
            RegionSlot::Shelf => &mut self.shelf,
            RegionSlot::ToolProperties => &mut self.tool_properties,
            RegionSlot::PrimitiveCard => &mut self.primitive_card,
        }
    }

    /// Painéis principais (exclui overlays contidos como shelf).
    pub fn panes(&self) -> Vec<(&'static str, egui::Rect)> {
        [
            ("header", self.header),
            ("viewport_toolbar", self.viewport_toolbar),
            ("status_bar", self.status_bar),
            ("left_tools", self.left_tools),
            ("asset_browser", self.asset_browser),
            ("bottom_dock", self.bottom_dock),
            ("right_dock", self.right_dock),
            ("uv_editor", self.uv_editor),
            ("paint_canvas", self.paint_canvas),
            ("viewport", self.viewport),
        ]
        .into_iter()
        .filter_map(|(name, r)| r.map(|rect| (name, rect)))
        .collect()
    }

    /// Nomes dos painéis que invadem a barra de status (deve ser vazio).
    ///
    /// Exige área positiva de interseção: `Rect::intersects` do egui usa `<=` e
    /// acusa até painéis perfeitamente adjacentes (borda compartilhada).
    pub fn status_overlaps(&self) -> Vec<&'static str> {
        let Some(status) = self.status_bar else {
            return Vec::new();
        };
        self.panes()
            .into_iter()
            .filter(|(name, _)| *name != "status_bar")
            .filter(|(_, r)| overlaps_area(*r, status))
            .map(|(name, _)| name)
            .collect()
    }

    /// A shelf deve permanecer dentro da viewport segura.
    pub fn shelf_within_viewport(&self) -> bool {
        match (self.shelf, self.viewport) {
            (Some(shelf), Some(viewport)) => viewport.contains_rect(shelf),
            // Sem shelf visível: invariante trivialmente satisfeito.
            (None, _) => true,
            // Shelf sem viewport conhecida: não verificável.
            (Some(_), None) => false,
        }
    }

    /// Every interactive overlay owned by the viewport must remain inside its
    /// canonical safe rectangle. Hidden overlays are ignored.
    pub fn viewport_overlays_within_viewport(&self) -> bool {
        let overlays = [self.shelf, self.tool_properties, self.primitive_card];
        if overlays.iter().all(Option::is_none) {
            return true;
        }
        let Some(viewport) = self.viewport else {
            return false;
        };
        overlays
            .into_iter()
            .flatten()
            .all(|overlay| viewport.contains_rect(overlay))
    }

    /// Outliner e Inspector devem ter retângulos independentes e disjuntos.
    pub fn dock_sections_disjoint(&self) -> bool {
        match (self.right_outliner, self.right_inspector) {
            (Some(a), Some(b)) => !overlaps_area(a, b),
            _ => true,
        }
    }
}

/// Interseção com área positiva (tolerância de meio pixel p/ poeira float).
fn overlaps_area(a: egui::Rect, b: egui::Rect) -> bool {
    let inter = a.intersect(b);
    inter.width() > 0.5 && inter.height() > 0.5
}

/// Espessura da divisória do dock (usada para derivar o mínimo lado a lado).
pub const DOCK_SEPARATOR_W: f32 = 10.0;
/// Mínimos de coluna do dock lado a lado (conteúdo real: barra do objeto e
/// árvore do Scene). Abaixo disso o conteúdo transborda e o painel se alarga
/// sozinho — por isso o dock lado a lado nunca nasce menor que a soma.
pub const DOCK_MIN_OUTLINER_W: f32 = 168.0;
/// Ver [`DOCK_MIN_OUTLINER_W`].
pub const DOCK_MIN_INSPECTOR_W: f32 = 232.0;

pub const DOCK_HEADER_H: f32 = 28.0;
/// Faixa de seção colapsada no modo lado a lado.
pub const DOCK_STRIP_W: f32 = 44.0;

/// Estimativa barata de conteúdo do Scene (contagens O(1), sem montar árvore):
/// cabeçalho + busca + linhas + respiro.
///
/// A política que consome este número (AUTO / MANUAL / colapso) e a divisão das
/// duas seções vivem em `adapters::tile_layout::PetuniaShellLayout` desde a
/// Wave 5b (§24: thresholds de layout pertencem ao adapter).
pub fn estimate_scene_content(rows: usize, row_h: f32) -> f32 {
    34.0 + 30.0 + rows as f32 * row_h + 12.0
}
/// Tamanhos seguros de modal a partir da viewport (Wave 5 — §9.4).
///
/// Nenhum valor excede a viewport útil: `min = min(pedido, disponível)` com
/// piso absoluto, `max = min(disponível, teto)`, padrão preso entre os dois.
/// Retorna `(default_size, min_size, max_size)`.
pub fn modal_sizes(
    viewport: egui::Rect,
    default_desired: egui::Vec2,
    min_req: egui::Vec2,
    max_cap: egui::Vec2,
) -> (egui::Vec2, egui::Vec2, egui::Vec2) {
    let avail = egui::vec2(
        (viewport.width() - 24.0).max(0.0),
        (viewport.height() - 24.0).max(0.0),
    );
    // Floors themselves are capped by the available size; otherwise a tiny
    // application window could paradoxically produce a modal larger than its
    // viewport (the previous `max(200/160)` ordering did exactly that).
    let min = egui::vec2(
        min_req.x.min(avail.x).max(200.0_f32.min(avail.x)),
        min_req.y.min(avail.y).max(160.0_f32.min(avail.y)),
    );
    let max = egui::vec2(
        avail.x.min(max_cap.x).max(min.x),
        avail.y.min(max_cap.y).max(min.y),
    );
    let default = egui::vec2(
        default_desired.x.clamp(min.x, max.x),
        default_desired.y.clamp(min.y, max.y),
    );
    (default, min, max)
}

/// Canonical calculation for scene panel height as indexed in `ui-map.json`.
/// Cálculo canônico para a altura do painel Scene conforme indexado no `ui-map.json`.
pub fn scene_panel_height(
    total_h: f32,
    collapsed: bool,
    auto: bool,
    manual_frac: f32,
    content_est: f32,
) -> f32 {
    const DOCK_HEADER_H: f32 = 32.0;
    const DOCK_SEPARATOR_H: f32 = 6.0;
    if collapsed {
        return DOCK_HEADER_H;
    }
    if auto {
        let max_auto = (total_h * 0.38).max(DOCK_HEADER_H);
        (content_est + DOCK_HEADER_H).clamp(DOCK_HEADER_H, max_auto)
    } else {
        let available = (total_h - DOCK_SEPARATOR_H - DOCK_HEADER_H).max(DOCK_HEADER_H);
        (total_h * manual_frac.clamp(0.1, 0.9)).clamp(DOCK_HEADER_H, available)
    }
}

const REGIONS_KEY: &str = "petunia_ui_regions";

/// Reseta as regiões no início do frame (evita slots obsoletos de painéis ocultos).
pub fn reset(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(REGIONS_KEY), UiRegions::default()));
}

/// Registra o retângulo de um slot (chamado pelo dono do componente).
pub fn record(ctx: &egui::Context, slot: RegionSlot, rect: egui::Rect) {
    ctx.data_mut(|d| {
        let mut regions = d
            .get_temp::<UiRegions>(egui::Id::new(REGIONS_KEY))
            .unwrap_or_default();
        *regions.slot_mut(slot) = Some(rect);
        d.insert_temp(egui::Id::new(REGIONS_KEY), regions);
    });
}

/// Lê as regiões do frame corrente (para overlays e testes).
pub fn load(ctx: &egui::Context) -> Option<UiRegions> {
    ctx.data(|d| d.get_temp::<UiRegions>(egui::Id::new(REGIONS_KEY)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
    }

    #[test]
    fn no_overlap_by_default() {
        let regions = UiRegions::default();
        assert!(regions.status_overlaps().is_empty());
        assert!(regions.shelf_within_viewport());
        assert!(regions.viewport_overlays_within_viewport());
        assert!(regions.dock_sections_disjoint());
    }

    #[test]
    fn detects_status_overlap() {
        let regions = UiRegions {
            status_bar: Some(rect(0.0, 1000.0, 1920.0, 1080.0)),
            viewport: Some(rect(0.0, 100.0, 1500.0, 1050.0)),
            ..Default::default()
        };
        assert_eq!(regions.status_overlaps(), vec!["viewport"]);
    }

    #[test]
    fn edge_touching_panels_do_not_overlap() {
        let regions = UiRegions {
            status_bar: Some(rect(0.0, 1000.0, 1920.0, 1080.0)),
            viewport: Some(rect(0.0, 100.0, 1500.0, 1000.0)),
            ..Default::default()
        };
        assert!(regions.status_overlaps().is_empty());
    }

    #[test]
    fn shelf_outside_viewport_fails() {
        let regions = UiRegions {
            viewport: Some(rect(0.0, 0.0, 800.0, 600.0)),
            shelf: Some(rect(0.0, 700.0, 800.0, 740.0)),
            ..Default::default()
        };
        assert!(!regions.shelf_within_viewport());
    }

    #[test]
    fn disjoint_sections_pass() {
        let regions = UiRegions {
            right_outliner: Some(rect(1500.0, 0.0, 1920.0, 400.0)),
            right_inspector: Some(rect(1500.0, 408.0, 1920.0, 1000.0)),
            ..Default::default()
        };
        assert!(regions.dock_sections_disjoint());
    }

    #[test]
    fn intersecting_sections_fail() {
        let regions = UiRegions {
            right_outliner: Some(rect(1500.0, 0.0, 1920.0, 500.0)),
            right_inspector: Some(rect(1500.0, 400.0, 1920.0, 1000.0)),
            ..Default::default()
        };
        assert!(!regions.dock_sections_disjoint());
    }

    #[test]
    fn scene_content_estimate_grows_with_the_row_count() {
        // A política de dimensionamento (AUTO/MANUAL/colapso) é testada no
        // adapter; aqui fica só o contrato da estimativa que ela consome.
        let one = estimate_scene_content(2, 28.0);
        let many = estimate_scene_content(300, 28.0);
        assert!(one < many);
        assert!((many - one - 298.0 * 28.0).abs() < 0.01);
        assert!(one >= 34.0 + 30.0 + 2.0 * 28.0);
    }

    #[test]
    fn modal_sizes_never_exceed_viewport() {
        for (w, h) in [
            (1920.0, 1080.0),
            (1280.0, 800.0),
            (700.0, 500.0),
            (400.0, 300.0),
        ] {
            let vp = rect(0.0, 0.0, w, h);
            let (def, min, max) = modal_sizes(
                vp,
                egui::vec2(720.0, 560.0),
                egui::vec2(480.0, 360.0),
                egui::vec2(860.0, 720.0),
            );
            assert!(max.x <= w - 24.0 + 0.01 && max.y <= h - 24.0 + 0.01);
            assert!(min.x <= max.x && min.y <= max.y);
            assert!(def.x >= min.x && def.x <= max.x && def.y >= min.y && def.y <= max.y);
        }
    }

    #[test]
    fn viewport_overlay_invariant_covers_tool_and_primitive_cards() {
        let viewport = rect(0.0, 0.0, 800.0, 600.0);
        let valid = UiRegions {
            viewport: Some(viewport),
            tool_properties: Some(rect(12.0, 12.0, 280.0, 300.0)),
            primitive_card: Some(rect(420.0, 180.0, 690.0, 420.0)),
            ..Default::default()
        };
        assert!(valid.viewport_overlays_within_viewport());

        let invalid = UiRegions {
            primitive_card: Some(rect(700.0, 500.0, 900.0, 650.0)),
            ..valid
        };
        assert!(!invalid.viewport_overlays_within_viewport());
    }

    #[test]
    fn modal_sizes_stay_inside_tiny_viewports() {
        let viewport = rect(0.0, 0.0, 150.0, 120.0);
        let (default, min, max) = modal_sizes(
            viewport,
            egui::vec2(720.0, 560.0),
            egui::vec2(480.0, 360.0),
            egui::vec2(860.0, 720.0),
        );
        let avail = egui::vec2(126.0, 96.0);
        for size in [default, min, max] {
            assert!(size.x <= avail.x + f32::EPSILON);
            assert!(size.y <= avail.y + f32::EPSILON);
            assert!(size.x >= 0.0 && size.y >= 0.0);
        }
    }
}
