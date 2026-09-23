//! Perfis de composição por workspace (Wave 3 — §6; Wave 8 — perfil PAINT).
//!
//! Fonte única da verdade sobre o que muda quando o usuário troca de workspace.
//! A V1 congela `MODEL / PAINT` (capítulo 36, adendo 2026-09-16); o workspace de
//! animação de P3D-066 só entra com a feature `animation-workspace`. Workspaces
//! compartilham projeto, seleção e undo; só a composição do shell muda: paleta de
//! ferramentas, centro, seções do dock e overlays da viewport.
//!
//! ## Centro e dock por workspace
//!
//! O perfil não descreve apenas a paleta esquerda: ele diz **o que cada seção do
//! dock mostra** e **se o workspace oferece editor 2D**. O modo de visualização
//! escolhido pelo usuário define se a tela usa 3D, 2D ou split: o
//! `adapters::tile_layout` continua dono da geometria, e o desenho de cada seção
//! é escolhido aqui.

use petunia_core::Workspace;

/// Paleta da toolbar esquerda por workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPaletteKind {
    /// Modelagem: seleção, transform, inspeção + malha (Edit).
    Modeling,
    /// Pintura: pincéis, formas e conta-gotas.
    Paint,
    /// Animação: seleção + transform de pose (feature `animation-workspace`).
    #[allow(dead_code)]
    Animation,
}

/// Conteúdo de uma seção do dock (topo ou base).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSectionKind {
    /// Árvore de objetos/coleções do projeto.
    Scene,
    /// Pilha de camadas de pintura (efeitos, opacidade, blend, DnD).
    Layers,
    /// Inspector do objeto (transform, geometria, modificadores, display).
    Properties,
    /// Controles de pincel, cor, canal e tela.
    Brush,
}

/// Composição da área central por workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterKind {
    /// Viewport 3D único em toda a área central.
    Viewport3D,
    /// O workspace oferece editor 2D de textura como modo opcional (PAINT).
    Canvas2D,
}

/// Painel inferior fixo por workspace (além da overlay flutuante).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPaneKind {
    None,
    /// Faixa de timeline (feature `animation-workspace`).
    #[allow(dead_code)]
    Timeline,
}

/// Perfil de layout de um workspace.
pub struct WorkspaceLayoutProfile {
    pub id: Workspace,
    pub left_tools: ToolPaletteKind,
    pub center: CenterKind,
    pub bottom: BottomPaneKind,
    /// Conteúdo da seção superior do dock.
    pub dock_top: DockSectionKind,
    /// Conteúdo da seção inferior do dock.
    pub dock_bottom: DockSectionKind,
    /// Chave i18n do rótulo da seção superior (abas do dock e a11y).
    pub dock_top_label: &'static str,
    /// Chave i18n do rótulo da seção inferior.
    pub dock_bottom_label: &'static str,
    /// Aba do inspector quando o workspace usa abas persistentes (Model).
    pub default_inspector_tab: &'static str,
    /// Shelf contextual flutuante sobre a viewport.
    pub overlay_shelf: bool,
}

impl WorkspaceLayoutProfile {
    /// Este workspace oferece a superfície de textura 2D?
    pub fn supports_canvas_2d(&self) -> bool {
        self.center == CenterKind::Canvas2D
    }
}

const PROFILES: [WorkspaceLayoutProfile; Workspace::COUNT] = [
    WorkspaceLayoutProfile {
        id: Workspace::Model,
        left_tools: ToolPaletteKind::Modeling,
        center: CenterKind::Viewport3D,
        bottom: BottomPaneKind::None,
        dock_top: DockSectionKind::Scene,
        dock_bottom: DockSectionKind::Properties,
        dock_top_label: "ui.outliner",
        dock_bottom_label: "ui.properties",
        default_inspector_tab: "object",
        overlay_shelf: true,
    },
    WorkspaceLayoutProfile {
        id: Workspace::Paint,
        left_tools: ToolPaletteKind::Paint,
        center: CenterKind::Canvas2D,
        bottom: BottomPaneKind::None,
        dock_top: DockSectionKind::Layers,
        // Context é dedicado à pintura: cor, pincel, canal e tela. A pilha de
        // camadas fica na seção superior do mesmo dock.
        dock_bottom: DockSectionKind::Brush,
        dock_top_label: "paint.layers",
        dock_bottom_label: "paint.brush",
        default_inspector_tab: "object",
        overlay_shelf: false,
    },
    // Workspace UV mantido só para projetos legados: a edição UV saiu da UI V1
    // (adendo de foundations/36) e o antigo `CenterKind::UvSplit` morreu junto.
    // O perfil continua existindo porque todo workspace compilado tem um.
    WorkspaceLayoutProfile {
        id: Workspace::Uv,
        left_tools: ToolPaletteKind::Paint,
        center: CenterKind::Viewport3D,
        bottom: BottomPaneKind::None,
        dock_top: DockSectionKind::Layers,
        dock_bottom: DockSectionKind::Brush,
        dock_top_label: "paint.layers",
        dock_bottom_label: "paint.brush",
        default_inspector_tab: "object",
        overlay_shelf: false,
    },
    // Nota: o UV legado mantém o conteúdo de pintura nas duas seções porque ele
    // não tem cartão flutuante (não é um workspace de trabalho da V1).
    #[cfg(feature = "animation-workspace")]
    WorkspaceLayoutProfile {
        id: Workspace::Animate,
        left_tools: ToolPaletteKind::Animation,
        center: CenterKind::Viewport3D,
        bottom: BottomPaneKind::Timeline,
        dock_top: DockSectionKind::Scene,
        dock_bottom: DockSectionKind::Properties,
        dock_top_label: "ui.outliner",
        dock_bottom_label: "ui.properties",
        default_inspector_tab: "object",
        overlay_shelf: true,
    },
];

/// Workspaces expostos no chrome atual. UV permanece disponível no domínio para
/// compatibilidade de projetos, mas sua edição não faz parte da UI V1.
pub fn visible_workspaces() -> &'static [Workspace] {
    &[Workspace::Model, Workspace::Paint]
}

/// Perfil canônico do workspace (existe um perfil para cada workspace compilado).
pub fn profile_for(workspace: Workspace) -> &'static WorkspaceLayoutProfile {
    PROFILES
        .iter()
        .find(|p| p.id == workspace)
        .expect("workspace sem perfil de layout")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_workspace_has_a_profile() {
        for ws in Workspace::all() {
            assert_eq!(profile_for(ws).id, ws);
        }
    }

    #[test]
    fn legacy_uv_uses_the_normal_viewport() {
        assert_eq!(profile_for(Workspace::Uv).center, CenterKind::Viewport3D);
        assert!(!profile_for(Workspace::Uv).supports_canvas_2d());
    }

    #[test]
    fn paint_is_the_canvas_workspace() {
        let paint = profile_for(Workspace::Paint);
        assert!(paint.supports_canvas_2d(), "PAINT oferece editor 2D");
        assert_eq!(paint.dock_top, DockSectionKind::Layers);
        assert_eq!(
            paint.dock_bottom,
            DockSectionKind::Brush,
            "Context apresenta os controles de pintura"
        );
        assert_eq!(paint.left_tools, ToolPaletteKind::Paint);
    }

    #[test]
    fn model_keeps_the_scene_and_the_inspector() {
        let model = profile_for(Workspace::Model);
        assert_eq!(model.dock_top, DockSectionKind::Scene);
        assert_eq!(model.dock_bottom, DockSectionKind::Properties);
        assert!(!model.supports_canvas_2d());
    }

    #[cfg(feature = "animation-workspace")]
    #[test]
    fn animate_is_the_only_timeline_bottom() {
        for ws in Workspace::all() {
            assert_eq!(
                profile_for(ws).bottom == BottomPaneKind::Timeline,
                ws == Workspace::Animate
            );
        }
    }

    #[test]
    fn v1_visible_workspaces_are_model_and_paint() {
        assert_eq!(visible_workspaces(), &[Workspace::Model, Workspace::Paint]);
    }

    #[test]
    fn tool_palettes_are_distinct_per_workspace() {
        let kinds: Vec<ToolPaletteKind> = visible_workspaces()
            .iter()
            .map(|ws| profile_for(*ws).left_tools)
            .collect();
        let mut sorted = kinds.clone();
        sorted.sort_by_key(|k| *k as u8);
        sorted.dedup_by_key(|k| *k as u8);
        assert_eq!(sorted.len(), kinds.len());
    }
}
