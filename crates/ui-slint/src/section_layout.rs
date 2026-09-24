// Floating and pinned state of Inspector sections.
// Fase 1 delivers the state model only: pure transitions, persistence mapping
// and bridge helpers live here; Slint markup arrives in Fase 2.
// Estado flutuante e fixado das seções do Inspector.
// A Fase 1 entrega só o modelo de estado: transições puras, mapeamento de
// persistência e helpers do bridge vivem aqui; o markup Slint chega na Fase 2.

use petunia_config::{InspectorSectionId, SectionLayout, UserPreferences};

/// Index into [`SectionLayouts`], in canonical Inspector order.
/// Índice em [`SectionLayouts`], na ordem canônica do Inspector.
pub const fn section_index(id: InspectorSectionId) -> usize {
    match id {
        InspectorSectionId::Parts => 0,
        InspectorSectionId::Transform => 1,
        InspectorSectionId::Material => 2,
        InspectorSectionId::Object => 3,
        InspectorSectionId::Modifiers => 4,
        InspectorSectionId::QuickActions => 5,
    }
}

/// Runtime layouts for all six sections; index with [`section_index`].
/// Layouts em tempo de execução das seis seções; indexar com [`section_index`].
pub type SectionLayouts = [SectionLayout; 6];

/// All sections docked: the pre-float baseline behavior.
/// Todas as seções ancoradas: o comportamento baseline pré-flutuação.
pub fn default_section_layouts() -> SectionLayouts {
    SectionLayouts::default()
}

/// Overlay persisted preferences onto defaults; missing entries stay default.
/// Sobrepõe as preferências persistidas aos padrões; entradas ausentes mantêm o padrão.
pub fn restore_section_layouts(preferences: &UserPreferences) -> SectionLayouts {
    InspectorSectionId::all().map(|id| preferences.section_layout(id))
}

/// Dock (`true`) or float (`false`) one section.
/// Ancora (`true`) ou flutua (`false`) uma seção.
pub fn set_docked(layouts: &mut SectionLayouts, id: InspectorSectionId, docked: bool) {
    layouts[section_index(id)].docked = docked;
}

/// Move a floating card; coordinates go through [`SectionLayout::sanitized`].
/// Move um card flutuante; coordenadas passam por [`SectionLayout::sanitized`].
pub fn move_floating(layouts: &mut SectionLayouts, id: InspectorSectionId, x: f32, y: f32) {
    let current = &layouts[section_index(id)];
    layouts[section_index(id)] = SectionLayout {
        x,
        y,
        ..current.clone()
    }
    .sanitized();
}

/// Pin a section open: it ignores collapse-all and panel collapse.
/// Fixa uma seção aberta: ela ignora recolher-tudo e recolhimento do painel.
pub fn set_pin_open(layouts: &mut SectionLayouts, id: InspectorSectionId, pin_open: bool) {
    layouts[section_index(id)].pin_open = pin_open;
}

/// Pin a section to an asset UUID text (`None` follows the selection).
/// Validation (length, emptiness) goes through [`SectionLayout::sanitized`].
/// Fixa uma seção ao texto de UUID de um asset (`None` segue a seleção).
/// Validação (tamanho, vazio) passa por [`SectionLayout::sanitized`].
pub fn set_pinned_asset(
    layouts: &mut SectionLayouts,
    id: InspectorSectionId,
    asset: Option<String>,
) {
    let current = &layouts[section_index(id)];
    layouts[section_index(id)] = SectionLayout {
        pinned_asset: asset,
        ..current.clone()
    }
    .sanitized();
}

/// Effective open state: pinned-open sections ignore collapse-all.
/// Estado aberto efetivo: seções com pin aberto ignoram recolher-tudo.
pub fn is_effectively_open(
    layouts: &SectionLayouts,
    open: &[bool; 6],
    id: InspectorSectionId,
) -> bool {
    let index = section_index(id);
    open[index] || layouts[index].pin_open
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlaceholderViewport, SlintUiBridge, UiIntent};
    use petunia_core::AppState;

    fn indexed() -> SectionLayouts {
        default_section_layouts()
    }

    fn hermetic_bridge(name: &str) -> (SlintUiBridge<PlaceholderViewport>, std::path::PathBuf) {
        let path =
            std::env::temp_dir().join(format!("petunia-sections-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        bridge.preferences_path_override = Some(path.clone());
        (bridge, path)
    }

    #[test]
    fn section_index_covers_all_six_sections_distinctly() {
        let mut seen = [false; 6];
        for id in InspectorSectionId::all() {
            let index = section_index(id);
            assert!(index < 6);
            assert!(!seen[index], "duplicate index for {id:?}");
            seen[index] = true;
        }
        assert!(seen.iter().all(|used| *used));
    }

    #[test]
    fn defaults_keep_every_section_docked() {
        let layouts = indexed();
        for id in InspectorSectionId::all() {
            assert!(
                layouts[section_index(id)].docked,
                "{id:?} must start docked"
            );
            assert_eq!(layouts[section_index(id)], SectionLayout::default());
        }
    }

    #[test]
    fn restore_overlays_persisted_entries_onto_defaults() {
        let mut preferences = UserPreferences::default();
        preferences.set_section_layout(
            InspectorSectionId::Material,
            SectionLayout {
                docked: false,
                x: 200.0,
                y: 100.0,
                pin_open: false,
                pinned_asset: None,
            },
        );
        let layouts = restore_section_layouts(&preferences);
        assert!(!layouts[section_index(InspectorSectionId::Material)].docked);
        assert!(layouts[section_index(InspectorSectionId::Parts)].docked);
    }

    #[test]
    fn move_clamps_coordinates_through_sanitized() {
        let mut layouts = indexed();
        move_floating(&mut layouts, InspectorSectionId::Object, -20.0, 99999.0);
        let moved = &layouts[section_index(InspectorSectionId::Object)];
        assert_eq!((moved.x, moved.y), (0.0, 8192.0));
        move_floating(&mut layouts, InspectorSectionId::Object, f32::NAN, 50.0);
        let moved = &layouts[section_index(InspectorSectionId::Object)];
        assert_eq!((moved.x, moved.y), (12.0, 50.0));
    }

    #[test]
    fn pin_open_survives_collapse_all() {
        let mut layouts = indexed();
        let collapsed = [false; 6];
        assert!(!is_effectively_open(
            &layouts,
            &collapsed,
            InspectorSectionId::Parts
        ));
        set_pin_open(&mut layouts, InspectorSectionId::Parts, true);
        assert!(is_effectively_open(
            &layouts,
            &collapsed,
            InspectorSectionId::Parts
        ));
        assert!(!is_effectively_open(
            &layouts,
            &collapsed,
            InspectorSectionId::Transform
        ));
    }

    #[test]
    fn section_view_model_exposes_canonical_order() {
        let bridge = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        let vm = bridge.view_model();
        let ids: Vec<&str> = vm
            .section_states
            .iter()
            .map(|state| state.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "parts",
                "transform",
                "material",
                "object",
                "modifiers",
                "quick_actions"
            ]
        );
        assert!(vm.section_states.iter().all(|state| state.docked));
    }

    #[test]
    fn section_intents_mutate_state_and_persist_hermetically() {
        let (mut bridge, path) = hermetic_bridge("intents");
        bridge.apply(UiIntent::SetSectionDocked {
            section: InspectorSectionId::Material,
            docked: false,
        });
        bridge.apply(UiIntent::MoveSectionFloat {
            section: InspectorSectionId::Material,
            x: 200.0,
            y: -30.0,
        });
        bridge.apply(UiIntent::SetSectionPinOpen {
            section: InspectorSectionId::Material,
            pin_open: true,
        });
        bridge.apply(UiIntent::SetSectionPinnedAsset {
            section: InspectorSectionId::Material,
            asset: Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
        });
        let material = &bridge.section_layouts[section_index(InspectorSectionId::Material)];
        assert!(!material.docked);
        assert_eq!((material.x, material.y), (200.0, 0.0));
        assert!(material.pin_open);
        assert!(material.pinned_asset.is_some());
        let reloaded = UserPreferences::load_from_path(&path).unwrap();
        assert_eq!(
            reloaded.section_layout(InspectorSectionId::Material),
            *material
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn restore_section_layouts_loads_persisted_entries() {
        let (mut bridge, path) = hermetic_bridge("restore");
        let mut preferences = UserPreferences::default();
        preferences.set_section_layout(
            InspectorSectionId::Object,
            SectionLayout {
                docked: false,
                ..SectionLayout::default()
            },
        );
        bridge.restore_section_layouts(&preferences);
        assert!(!bridge.section_layouts[section_index(InspectorSectionId::Object)].docked);
        assert!(bridge.section_layouts[section_index(InspectorSectionId::Parts)].docked);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pinned_asset_rejects_oversized_ids() {
        let mut layouts = indexed();
        set_pinned_asset(
            &mut layouts,
            InspectorSectionId::Transform,
            Some("x".repeat(65)),
        );
        assert_eq!(
            layouts[section_index(InspectorSectionId::Transform)].pinned_asset,
            None
        );
        set_pinned_asset(
            &mut layouts,
            InspectorSectionId::Transform,
            Some("01234567-89ab-cdef-0123-456789abcdef".to_string()),
        );
        assert!(
            layouts[section_index(InspectorSectionId::Transform)]
                .pinned_asset
                .is_some()
        );
    }
}
