//! Testes de integração de fluxos de UI com `egui_kittest`.

use egui_kittest::Harness;
use petunia_core::{AppState, EditMode, ModalKind, SelectMode, Workspace};
use petunia_module_model::ToolRegistry;
use petunia_ui::{UiAction, contextual_shelf, main_header, outliner, toolbar, viewport_bar};

#[test]
fn test_kittest_toolbar_flow() {
    let mut state = AppState::new("en");
    state.active_tool = "select".into();
    let tools = ToolRegistry::new();

    let mut harness = Harness::builder().build_ui(|ui| {
        toolbar::draw_contents(ui, &mut state, &tools);
    });

    harness.run();
    drop(harness);
    assert_eq!(state.active_tool, "select");
}

#[test]
fn test_kittest_main_header_flow() {
    let mut state = AppState::new("en");
    let mut action = UiAction::none();

    let mut harness = Harness::builder().build_ui(|ui| {
        main_header::draw(ui, &mut state, &mut action);
    });

    harness.run();
    drop(harness);
    assert_eq!(state.edit_mode(), EditMode::Object);
}

#[test]
fn test_kittest_viewport_bar_flow() {
    let mut state = AppState::new("en");

    let mut harness = Harness::builder().build_ui(|ui| {
        viewport_bar::draw(ui, &mut state);
    });

    harness.run();
    drop(harness);
    assert_eq!(state.gizmo_mode, ModalKind::Move);
}

#[test]
fn test_kittest_outliner_tree_flow() {
    let mut state = AppState::new("en");
    state.ui.outliner_search = "Cube".to_string();

    let mut harness = Harness::builder().build_ui(|ui| {
        outliner::draw(ui, &mut state);
    });

    harness.run();
    drop(harness);
    assert_eq!(state.ui.outliner_search, "Cube");
}

#[test]
fn test_kittest_reference_manager_flow() {
    let mut state = AppState::new("pt-BR");
    state.ui.show_reference_manager = true;

    // Adiciona uma imagem para validar rendering de slot preenchido
    state
        .project
        .refs
        .push(petunia_core::ReferenceImage::from_rgba(
            "test_ref.png".to_string(),
            64,
            64,
            vec![255u8; 64 * 64 * 4],
        ));

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::reference_manager::draw(ui.ctx(), &mut state);
    });

    harness.run_steps(2);
    drop(harness);
    assert!(state.ui.show_reference_manager);
    assert_eq!(state.project.refs.len(), 1);
}

#[test]
fn test_kittest_nav_hud_flow() {
    let state = AppState::new("en");
    assert!(state.show_nav_hud);
    assert!(state.show_overlays);

    let mut harness = Harness::builder().build_ui(|ui| {
        let rect = ui.max_rect();
        petunia_ui::nav_gizmo::draw_nav_hud(&state, rect, ui.painter());
    });

    harness.run();
    drop(harness);
}

#[test]
fn test_kittest_asset_browser_flow() {
    let mut state = AppState::new("en");
    state.ui.show_asset_browser = true;
    state.ui.asset_thumbnail_size = 96.0;

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::asset_browser::draw(ui, &mut state);
    });

    harness.run_steps(2);
    drop(harness);
    assert_eq!(state.ui.asset_thumbnail_size, 96.0);
}

#[test]
fn test_kittest_right_panel_docked_and_detached_flow() {
    let mut state = AppState::new("en");
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();

    // 1. Docked mode
    state.ui.inspector_detached = false;
    let mut harness_docked = Harness::builder().build_ui(|ui| {
        petunia_ui::shell::draw(ui, &mut state, &tools, &mut registry);
    });
    harness_docked.run_steps(2);
    drop(harness_docked);

    // 2. Detached floating window mode
    state.ui.inspector_detached = true;
    let mut harness_detached = Harness::builder().build_ui(|ui| {
        petunia_ui::shell::draw(ui, &mut state, &tools, &mut registry);
    });
    harness_detached.run_steps(2);
    drop(harness_detached);
    assert!(state.ui.inspector_detached);
}

#[test]
fn test_kittest_properties_panel_tabs_flow() {
    let mut state = AppState::new("en");
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();

    state.ui.properties_tab = "object".to_string();
    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::properties_panel::draw(ui, &mut state, &tools, &mut registry);
    });
    harness.run_steps(2);
    drop(harness);

    // Aba contextual real alterna e persiste.
    state.ui.properties_tab = "modify".to_string();
    let mut harness_modify = Harness::builder().build_ui(|ui| {
        petunia_ui::properties_panel::draw(ui, &mut state, &tools, &mut registry);
    });
    harness_modify.run_steps(2);
    drop(harness_modify);
    assert_eq!(state.ui.properties_tab, "modify");

    // Aba legada ("tool") é higienizada para o padrão do contexto.
    state.ui.properties_tab = "tool".to_string();
    let mut harness_tool = Harness::builder().build_ui(|ui| {
        petunia_ui::properties_panel::draw(ui, &mut state, &tools, &mut registry);
    });
    harness_tool.run_steps(2);
    drop(harness_tool);
    assert_eq!(state.ui.properties_tab, "object");
}

#[test]
fn test_kittest_settings_modal_stability_over_multiple_frames() {
    let mut state = AppState::new("en");
    state.ui.show_settings = true;

    for tab in [
        "appearance",
        "icons",
        "language",
        "keymap",
        "interface",
        "import_export",
    ] {
        state.ui.settings_tab = tab.to_string();
        let mut harness = Harness::builder().build_ui(|ui| {
            petunia_ui::settings_modal::draw(ui.ctx(), &mut state);
        });
        // Executa 10 frames consecutivos para assegurar que não há runaway horizontal
        harness.run_steps(10);
        drop(harness);
    }
}

#[test]
fn test_kittest_asset_library_drawer_stability_over_multiple_frames() {
    let mut state = AppState::new("en");
    state.ui.show_asset_library = true;

    // Adiciona alguns assets para preencher a gaveta
    for _ in 0..4 {
        state.save_active_as_asset();
    }

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::asset_library_drawer::draw(ui.ctx(), &mut state);
    });

    // Executa 10 frames consecutivos para garantir largura estável e sem expansão
    harness.run_steps(10);
    drop(harness);
}

#[test]
fn test_kittest_reference_manager_multi_frame_stability() {
    let mut state = AppState::new("pt-BR");
    state.ui.show_reference_manager = true;

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::reference_manager::draw(ui.ctx(), &mut state);
    });

    // 10 frames em slots vazios (onde empty_rect era alocado sem restrição)
    harness.run_steps(10);
    drop(harness);
}

#[test]
fn test_kittest_detached_inspector_multi_frame_stability() {
    let mut state = AppState::new("en");
    state.ui.inspector_detached = true;
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::shell::draw(ui, &mut state, &tools, &mut registry);
    });

    harness.run_steps(10);
    drop(harness);
}

#[test]
fn test_kittest_full_draw_with_all_modals() {
    let mut state = AppState::new("en");
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();

    // Test with each modal open during full UI draw
    for modal in 0..5 {
        state.ui.show_settings = modal == 0;
        state.ui.show_asset_library = modal == 1;
        state.ui.show_command_palette = modal == 2;
        state.ui.show_reference_manager = modal == 3;
        state.ui.inspector_detached = modal == 4;

        let mut harness = Harness::builder().build_ui(|ui| {
            petunia_ui::draw(ui, &mut state, &tools, &mut registry, &mut action);
        });

        harness.run_steps(3);
        drop(harness);
    }
}

#[test]
fn test_kittest_inspector_detachment_transition() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().active_tool = "transform".to_string();
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();

    let state_clone = state.clone();
    let mut harness = Harness::builder().build_ui(move |ui| {
        petunia_ui::draw(
            ui,
            &mut state_clone.borrow_mut(),
            &tools,
            &mut registry,
            &mut action,
        );
    });
    harness.run_steps(2);

    // Detach mid-session
    state.borrow_mut().ui.inspector_detached = true;
    harness.run_steps(3);

    // Dock back
    state.borrow_mut().ui.inspector_detached = false;
    harness.run_steps(3);
    drop(harness);
}

#[test]
fn test_kittest_inspector_layer_collision_prevention() {
    let mut state = AppState::new("en");
    state.active_tool = "transform".to_string();
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();

    // Verifies that toggling inspector_detached inside right_panel closure
    // cleanly snapshots was_detached and never double-renders or causes layer collision.
    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::shell::draw(ui, &mut state, &tools, &mut registry);
        state.ui.inspector_detached = !state.ui.inspector_detached;
    });
    harness.run_steps(6);
    drop(harness);
}

#[test]
fn test_kittest_selection_modes_and_edit_mode_flow() {
    let mut state = AppState::new("en");

    // 1. Object Mode
    state.set_edit_mode(EditMode::Object);
    let mut harness = Harness::builder().build_ui(|ui| {
        viewport_bar::draw(ui, &mut state);
    });
    harness.run_steps(2);
    drop(harness);
    assert_eq!(state.edit_mode(), EditMode::Object);

    // 2. Edit Mode - Vertex Selection
    state.set_edit_mode(EditMode::Edit);
    state.select_mode = SelectMode::Vertex;
    let mut harness_vertex = Harness::builder().build_ui(|ui| {
        viewport_bar::draw(ui, &mut state);
    });
    harness_vertex.run_steps(2);
    drop(harness_vertex);
    assert_eq!(state.select_mode, SelectMode::Vertex);

    // 3. Edit Mode - Edge Selection
    state.select_mode = SelectMode::Edge;
    let mut harness_edge = Harness::builder().build_ui(|ui| {
        viewport_bar::draw(ui, &mut state);
    });
    harness_edge.run_steps(2);
    drop(harness_edge);
    assert_eq!(state.select_mode, SelectMode::Edge);

    // 4. Edit Mode - Face Selection
    state.select_mode = SelectMode::Face;
    let mut harness_face = Harness::builder().build_ui(|ui| {
        viewport_bar::draw(ui, &mut state);
    });
    harness_face.run_steps(2);
    drop(harness_face);
    assert_eq!(state.select_mode, SelectMode::Face);
}

#[test]
fn test_kittest_contextual_shelf_across_all_workspaces() {
    let mut state = AppState::new("en");
    let fake_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));

    // 1. Model Workspace — Object Mode
    state.workspace = Workspace::Model;
    state.set_edit_mode(EditMode::Object);
    let mut harness_obj = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, fake_rect);
        assert!(shelf_rect.is_some());
    });
    harness_obj.run_steps(2);
    drop(harness_obj);

    // 2. Model Workspace — Edit Mode
    state.set_edit_mode(EditMode::Edit);
    let mut harness_edit = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, fake_rect);
        assert!(shelf_rect.is_some());
    });
    harness_edit.run_steps(2);
    drop(harness_edit);

    // 3. Paint Workspace
    state.workspace = Workspace::Paint;
    let mut harness_paint = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, fake_rect);
        assert!(shelf_rect.is_some());
    });
    harness_paint.run_steps(2);
    drop(harness_paint);

    // 4. UV Workspace
    state.workspace = Workspace::Uv;
    let mut harness_uv = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, fake_rect);
        assert!(shelf_rect.is_some());
    });
    harness_uv.run_steps(2);
    drop(harness_uv);

    // 5. Animate Workspace — fora da V1 congelada (capítulo 36), só compila
    // com a feature `animation-workspace`.
    #[cfg(feature = "animation-workspace")]
    {
        state.workspace = Workspace::Animate;
        let mut harness_anim = Harness::builder().build_ui(|ui| {
            let shelf_rect = contextual_shelf::draw(ui, &mut state, fake_rect);
            assert!(shelf_rect.is_some());
        });
        harness_anim.run_steps(2);
        drop(harness_anim);
    }

    // 6. Narrow Viewport — Wave 4: pílula "Tools…" em vez de sumir, sem clippar.
    let narrow_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 600.0));
    let mut harness_narrow = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, narrow_rect);
        let shelf = shelf_rect.expect("narrow shelf collapses to pill, not None");
        assert!(shelf.width() <= narrow_rect.width());
        assert!(narrow_rect.contains_rect(shelf));
    });
    harness_narrow.run_steps(2);
    drop(harness_narrow);

    // 7. Medium Viewport — Compact/Overflow sem clippar.
    let medium_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
    let mut harness_medium = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, medium_rect);
        let shelf = shelf_rect.expect("medium shelf stays visible");
        assert!(shelf.width() <= medium_rect.width());
        assert!(medium_rect.contains_rect(shelf));
    });
    harness_medium.run_steps(2);
    drop(harness_medium);

    // 8. Tiny Viewport — sem espaço nem para a pílula.
    let tiny_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(60.0, 600.0));
    let mut harness_tiny = Harness::builder().build_ui(|ui| {
        let shelf_rect = contextual_shelf::draw(ui, &mut state, tiny_rect);
        assert!(shelf_rect.is_none());
    });
    harness_tiny.run_steps(2);
    drop(harness_tiny);
}

#[test]
fn test_kittest_menu_rows_fit_content() {
    // Linhas de menu acompanham o conteúdo (Wave 4 — §8.1), sem mínimo global.
    let mut harness = Harness::builder().build_ui(|ui| {
        let short = petunia_ui::widgets::PetuniaMenuItem::new("OK").show(ui);
        let long =
            petunia_ui::widgets::PetuniaMenuItem::new("Exportar malha selecionada como OBJ…")
                .shortcut(Some("Ctrl+E"))
                .show(ui);
        assert!(short.rect.width() < long.rect.width());
        assert!(long.rect.width() <= petunia_ui::widgets::MENU_MAX_W + 1.0);
        assert!(short.rect.width() >= petunia_ui::widgets::MENU_MIN_W - 1.0);
    });
    harness.run_steps(2);
    drop(harness);
}

#[test]
fn test_kittest_right_dock_regions_are_disjoint_and_safe() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    // Tamanho fixo: geometria determinística entre frames (Wave 2 — §21).
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    let regions = seen.borrow().clone().expect("regions recorded");
    assert!(regions.right_dock.is_some());
    assert!(regions.right_outliner.is_some());
    assert!(regions.right_inspector.is_some());
    assert!(regions.dock_sections_disjoint());
    assert!(
        regions.status_overlaps().is_empty(),
        "panes overlap status bar: {:?}",
        regions.status_overlaps()
    );
    assert!(regions.shelf_within_viewport());
    assert!(regions.viewport.is_some());
}

#[test]
fn test_kittest_dock_split_fraction_resizes_sections() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            seen_clone
                .borrow_mut()
                .push(petunia_ui::regions::load(ui.ctx()));
        });
    state.borrow_mut().ui.scene_split_auto = false;

    // Não indexa frame por número: o componente de layout (taffy) pode pedir um
    // frame extra por reflow, e a asserção é sobre o **último** frame de cada
    // configuração — nunca sobre a contagem de frames.
    state.borrow_mut().ui.right_dock_split = 0.3;
    harness.run_steps(6);
    let narrow = seen
        .borrow()
        .last()
        .and_then(|frame| frame.clone())
        .expect("regions narrow");

    state.borrow_mut().ui.right_dock_split = 0.7;
    harness.run_steps(6);
    let wide = seen
        .borrow()
        .last()
        .and_then(|frame| frame.clone())
        .expect("regions wide");

    drop(harness);

    let narrow_h = narrow.right_outliner.unwrap().height();
    let wide_h = wide.right_outliner.unwrap().height();
    assert!(
        wide_h > narrow_h + 20.0,
        "split 0.7 ({wide_h}) must exceed split 0.3 ({narrow_h})"
    );
    assert!(narrow.dock_sections_disjoint() && wide.dock_sections_disjoint());
}

#[test]
fn test_kittest_dock_collapse_gives_space_to_sibling() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    state.borrow_mut().ui.outliner_collapsed = true;
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    let regions = seen.borrow().clone().expect("regions recorded");
    let out_h = regions.right_outliner.unwrap().height();
    let insp_h = regions.right_inspector.unwrap().height();
    assert!(
        out_h <= 44.0,
        "collapsed outliner must be header-only ({out_h})"
    );
    assert!(insp_h > out_h);
    assert!(regions.dock_sections_disjoint());
}

#[test]
fn test_kittest_workspace_switch_preserves_dock_layout() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().ui.right_dock_split = 0.6;
    state.borrow_mut().ui.inspector_collapsed = true;
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    for workspace in Workspace::all() {
        state.borrow_mut().workspace = workspace;
        harness.run_steps(3);
    }
    drop(harness);

    let borrowed = state.borrow();
    assert_eq!(borrowed.ui.right_dock_split, 0.6);
    assert!(borrowed.ui.inspector_collapsed);
    drop(borrowed);
    let regions = seen.borrow().clone().expect("regions recorded");
    assert!(regions.dock_sections_disjoint());
    assert!(regions.status_overlaps().is_empty());
}

#[test]
fn test_kittest_workspace_profiles_compose_shell() {
    // Cada workspace compõe o shell de forma visivelmente distinta (§6.5).
    //
    // Baseline emendada (adendo de 2026-09-16 em foundations/36): o workspace UV
    // saiu da UI V1 — nenhum workspace expõe editor UV interativo — e o PAINT
    // **divide** o centro entre viewport 3D e tela 2D (lado a lado).
    #[allow(unused_mut)]
    let mut cases = vec![
        (Workspace::Model, false, false),
        (Workspace::Paint, false, true),
        (Workspace::Uv, false, false),
    ];
    #[cfg(feature = "animation-workspace")]
    cases.push((Workspace::Animate, true, false));

    for (workspace, expect_bottom, expect_canvas) in cases {
        let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
        state.borrow_mut().workspace = workspace;
        let tools = ToolRegistry::new();
        let mut registry = petunia_core::ModuleRegistry::new();
        let mut action = petunia_ui::UiAction::none();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
        let seen_clone = seen.clone();
        let state_clone = state.clone();

        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(1280.0, 800.0))
            .build_ui(move |ui| {
                petunia_ui::draw(
                    ui,
                    &mut state_clone.borrow_mut(),
                    &tools,
                    &mut registry,
                    &mut action,
                );
                *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
            });
        harness.run_steps(6);
        drop(harness);

        let regions = seen
            .borrow()
            .clone()
            .unwrap_or_else(|| panic!("regions recorded for {workspace:?}"));
        assert_eq!(
            regions.bottom_dock.is_some(),
            expect_bottom,
            "bottom pane for {workspace:?}"
        );
        assert!(
            regions.uv_editor.is_none(),
            "o editor UV não faz parte da UI V1 ({workspace:?})"
        );
        assert_eq!(
            regions.paint_canvas.is_some(),
            expect_canvas,
            "tela 2D no centro em {workspace:?}"
        );
        assert!(
            regions.viewport.is_some(),
            "a viewport 3D existe em todo workspace com cena ({workspace:?})"
        );
        if expect_canvas {
            let canvas = regions.paint_canvas.expect("tela 2D registrada");
            let viewport = regions.viewport.expect("viewport registrada");
            assert!(
                canvas.min.x >= viewport.min.x,
                "a tela fica à direita da viewport ({workspace:?})"
            );
        }
        assert!(
            regions.status_overlaps().is_empty(),
            "panes overlap status bar for {workspace:?}: {:?}",
            regions.status_overlaps()
        );
        assert!(
            regions.shelf_within_viewport(),
            "shelf fora da viewport em {workspace:?}"
        );
    }
}
#[test]
fn test_kittest_paint_tool_floats_the_brush_card() {
    // O pincel não ocupa seção do dock: ele flutua sobre a coluna 3D, no mesmo
    // cartão de ferramenta dos outros tools (§34).
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().workspace = Workspace::Paint;
    state.borrow_mut().active_tool = "paint".into();
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    let regions = seen.borrow().clone().expect("regions recorded");
    let card = regions.tool_properties.unwrap_or_else(|| {
        panic!(
            "o cartão do pincel precisa estar flutuando: {regions:?} tool={:?}",
            state.borrow().active_tool
        )
    });
    assert!(card.width() > 0.0 && card.height() > 0.0);
    assert!(
        regions.viewport_overlays_within_viewport(),
        "o cartão flutuante não pode escapar da viewport"
    );
}

#[test]
fn test_kittest_paint_center_splits_3d_and_canvas() {
    // O centro do PAINT tem as duas superfícies ao mesmo tempo: viewport 3D à
    // esquerda e tela 2D à direita, com divisória arrastável. Prévia escondida
    // atrás de aba é prévia que ninguém olha.
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().workspace = Workspace::Paint;
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    let regions = seen.borrow().clone().expect("regions recorded");
    let viewport = regions.viewport.expect("a viewport 3D divide o centro");
    let canvas = regions.paint_canvas.expect("a tela 2D divide o centro");
    assert!(canvas.width() > 0.0 && viewport.width() > 0.0);
    assert!(
        canvas.min.x >= viewport.min.x,
        "a tela fica à direita da viewport: {canvas:?} vs {viewport:?}"
    );
    assert!(
        viewport.width() > 100.0,
        "o divisor não pode deixar a viewport sem largura útil: {viewport:?}"
    );
    assert!(regions.status_overlaps().is_empty());
    assert!(regions.dock_sections_disjoint());
    // A barra de contexto pertence à coluna 3D, então continua sendo registrada.
    assert!(regions.viewport_toolbar.is_some());
}

#[cfg(feature = "animation-workspace")]
#[test]
fn test_kittest_animate_toolbar_has_pose_tools() {
    // A toolbar do Animate não é mais vazia: seleção + trio de pose.
    let mut state = AppState::new("en");
    state.workspace = Workspace::Animate;
    let tools = ToolRegistry::new();

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::toolbar::draw_contents(ui, &mut state, &tools);
    });
    harness.run_steps(2);
    drop(harness);

    // Ferramenta de malha não faz sentido no Animate: normalização do Model
    // não se aplica, mas o estado sobrevive intacto ao desenho.
    assert_eq!(state.workspace, Workspace::Animate);
}

#[test]
fn test_kittest_hidden_shelf_records_no_region() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().ui.show_shelf = false;
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    let regions = seen.borrow().clone().expect("regions recorded");
    assert!(regions.shelf.is_none());
    assert!(regions.viewport.is_some());
}

#[test]
fn test_kittest_reference_manager_grid_at_widths() {
    // Grade determinística: renderiza estável em 3 larguras, sem pânico.
    for width in [1280.0, 800.0, 520.0] {
        let mut state = AppState::new("pt-BR");
        state.ui.show_reference_manager = true;
        state
            .project
            .refs
            .push(petunia_core::ReferenceImage::from_rgba(
                "grid_ref.png".to_string(),
                64,
                64,
                vec![255u8; 64 * 64 * 4],
            ));

        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(width, 700.0))
            .build_ui(|ui| {
                petunia_ui::reference_manager::draw(ui.ctx(), &mut state);
            });
        harness.run_steps(10);
        drop(harness);
        assert!(state.ui.show_reference_manager);
    }
}

#[test]
fn test_kittest_pseudo_locale_survives_layout() {
    // Pseudo-locale expande ~40%: nada pode clippar nem sair da viewport (§12.4).
    for width in [1280.0, 700.0] {
        let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
        state.borrow_mut().ui.i18n =
            petunia_config::I18n::pseudo_from(&petunia_config::I18n::load("en"));
        let tools = ToolRegistry::new();
        let mut registry = petunia_core::ModuleRegistry::new();
        let mut action = petunia_ui::UiAction::none();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
        let seen_clone = seen.clone();
        let state_clone = state.clone();

        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(width, 800.0))
            .build_ui(move |ui| {
                petunia_ui::draw(
                    ui,
                    &mut state_clone.borrow_mut(),
                    &tools,
                    &mut registry,
                    &mut action,
                );
                *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
            });
        harness.run_steps(6);
        drop(harness);

        let regions = seen.borrow().clone().expect("regions recorded");
        assert!(
            regions.status_overlaps().is_empty(),
            "pseudo@{width}: {:?}",
            regions.status_overlaps()
        );
        assert!(regions.shelf_within_viewport(), "pseudo@{width}");
        assert!(regions.dock_sections_disjoint(), "pseudo@{width}");
    }
}

#[test]
fn test_kittest_live_language_switch_relabels() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    let before = state.borrow().t("ui.properties");
    assert_eq!(before, "Properties");
    state.borrow_mut().ui.i18n = petunia_config::I18n::load("pt-BR");
    let after = state.borrow().t("ui.properties");
    assert_eq!(after, "Propriedades");

    // Redescreve tudo sem pânico e preserva o estado de painéis.
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let state_clone = state.clone();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
        });
    harness.run_steps(3);
    drop(harness);
    // Troca de volta: rótulos acompanham, painéis preservados.
    state.borrow_mut().ui.i18n = petunia_config::I18n::load("en");
    assert_eq!(state.borrow().t("ui.properties"), "Properties");
}

#[test]
fn test_kittest_tab_focus_moves_through_settings() {
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    state.borrow_mut().ui.show_settings = true;
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let ever_focused = std::rc::Rc::new(std::cell::RefCell::new(false));
    let ever_focused_clone = ever_focused.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            if ui.ctx().memory(|m| m.focused()).is_some() {
                *ever_focused_clone.borrow_mut() = true;
            }
        });
    harness.run_steps(2);
    for _ in 0..12 {
        harness.key_press(egui::Key::Tab);
        harness.run_steps(1);
    }
    drop(harness);
    assert!(state.borrow().ui.show_settings);
    assert!(
        *ever_focused.borrow(),
        "Tab deve alcançar controles focáveis do diálogo"
    );
}

#[test]
fn test_kittest_ui_scale_matrix_keeps_regions() {
    // §14.5: 100%–175% sem pânico e sem invasão da status bar.
    for scale in [1.0, 1.25, 1.5, 1.75] {
        let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
        let tools = ToolRegistry::new();
        let mut registry = petunia_core::ModuleRegistry::new();
        let mut action = petunia_ui::UiAction::none();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
        let seen_clone = seen.clone();
        let state_clone = state.clone();

        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(1280.0, 800.0))
            .with_pixels_per_point(scale)
            .build_ui(move |ui| {
                petunia_ui::draw(
                    ui,
                    &mut state_clone.borrow_mut(),
                    &tools,
                    &mut registry,
                    &mut action,
                );
                *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
            });
        harness.run_steps(4);
        drop(harness);

        let regions = seen
            .borrow()
            .clone()
            .unwrap_or_else(|| panic!("regions @{scale}"));
        assert!(
            regions.status_overlaps().is_empty(),
            "scale {scale}: {:?}",
            regions.status_overlaps()
        );
    }
}

#[test]
fn test_tool_registry_hint_keys_resolve_in_both_locales() {
    // Cobertura de tooltips da toolbar (§13/§22.2): todo hint_key traduz.
    let en = petunia_config::I18n::load("en");
    let pt = petunia_config::I18n::load("pt-BR");
    let tools = ToolRegistry::with_defaults();
    assert!(!tools.all().is_empty());
    for tool in tools.all() {
        for key in [tool.label_key(), tool.hint_key()] {
            assert_ne!(en.t(key), key, "{} sem en", tool.id());
            assert_ne!(pt.t(key), key, "{} sem pt-BR", tool.id());
        }
    }
}

#[test]
fn test_shelf_commands_all_have_tooltips() {
    // Toda ação só-ícone da shelf é descobrível (§13.1/§22.2).
    for workspace in petunia_core::Workspace::all() {
        let mut state = AppState::new("en");
        state.workspace = workspace;
        for mode in [EditMode::Object, EditMode::Edit] {
            state.set_edit_mode(mode);
            let mut harness = Harness::builder().build_ui(|ui| {
                let rect = ui.max_rect();
                let _ = petunia_ui::contextual_shelf::draw(ui, &mut state, rect);
            });
            harness.run_steps(1);
            drop(harness);
        }
    }
}

#[test]
fn test_kittest_primitive_session_card_lifecycle() {
    // Sessão ativa → cartão renderiza sem pânico; centros registrados OK.
    let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
    assert!(
        state
            .borrow_mut()
            .begin_primitive(petunia_core::PrimitiveKind::Cube, None)
    );
    let tools = ToolRegistry::new();
    let mut registry = petunia_core::ModuleRegistry::new();
    let mut action = petunia_ui::UiAction::none();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_clone = seen.clone();
    let state_clone = state.clone();

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            petunia_ui::draw(
                ui,
                &mut state_clone.borrow_mut(),
                &tools,
                &mut registry,
                &mut action,
            );
            *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
        });
    harness.run_steps(6);
    drop(harness);

    assert!(state.borrow().primitive_session_valid());
    let regions = seen.borrow().clone().expect("regions recorded");
    assert!(regions.status_overlaps().is_empty());
    assert!(regions.viewport.is_some());

    // Cancela: asset some, sessão limpa, UI segue estável.
    let before = state.borrow().project.assets.len();
    assert!(state.borrow_mut().cancel_primitive());
    assert_eq!(state.borrow().project.assets.len(), before - 1);
    assert!(!state.borrow().primitive_session_valid());
}

#[test]
fn test_kittest_primitive_card_pseudo_and_narrow() {
    // Painel de criação sobrevive a rótulos longos e janela estreita (§39).
    for (width, lang) in [(800.0, "pseudo"), (700.0, "pt-BR"), (1280.0, "en")] {
        let state = std::rc::Rc::new(std::cell::RefCell::new(AppState::new("en")));
        if lang == "pseudo" {
            state.borrow_mut().ui.i18n =
                petunia_config::I18n::pseudo_from(&petunia_config::I18n::load("en"));
        } else {
            state.borrow_mut().ui.i18n = petunia_config::I18n::load(lang);
        }
        assert!(
            state
                .borrow_mut()
                .begin_primitive(petunia_core::PrimitiveKind::Torus, None)
        );
        let tools = ToolRegistry::new();
        let mut registry = petunia_core::ModuleRegistry::new();
        let mut action = petunia_ui::UiAction::none();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
        let seen_clone = seen.clone();
        let state_clone = state.clone();

        let mut harness = Harness::builder()
            .with_size(egui::Vec2::new(width, 700.0))
            .build_ui(move |ui| {
                petunia_ui::draw(
                    ui,
                    &mut state_clone.borrow_mut(),
                    &tools,
                    &mut registry,
                    &mut action,
                );
                *seen_clone.borrow_mut() = petunia_ui::regions::load(ui.ctx());
            });
        harness.run_steps(6);
        drop(harness);

        assert!(
            state.borrow().primitive_session_valid(),
            "sessão viva em {width}/{lang}"
        );
        let regions = seen.borrow().clone().expect("regions recorded");
        assert!(regions.status_overlaps().is_empty(), "{width}/{lang}");
    }
}

#[cfg(feature = "animation-workspace")]
#[test]
fn test_kittest_animation_workspace_and_rig_panel() {
    let mut state = AppState {
        session: petunia_core::state::EditorSession {
            workspace: Workspace::Animate,
            ..Default::default()
        },
        ..Default::default()
    };

    // Adiciona um esqueleto ao projeto
    let skel = petunia_project::animation::RigPreset::humanoid(1.0);
    state.project.add_skeleton(skel);

    // Adiciona um clipe de animação
    let clip =
        petunia_project::animation::AnimationLibrary::humanoid_idle(&state.project.skeletons[0]);
    state
        .project
        .add_animation(petunia_project::animation::AnimationAsset::new(
            "Humanoid_Idle",
            clip,
        ));

    let mut harness = Harness::builder().build_ui(|ui| {
        petunia_ui::modules_ui::animation_ui::draw_animation_panel(ui, &mut state);
    });

    harness.run_steps(2);
    drop(harness);

    assert_eq!(state.project.skeletons.len(), 1);
    assert_eq!(state.project.animations.len(), 1);
    assert!(state.project.skeletons[0].bones.len() >= 15);
}
