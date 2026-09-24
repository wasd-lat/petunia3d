//! Gestos reais do shell declarativo: o bridge isolado não cobre hit testing Slint.

use std::cell::Cell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::{PetuniaSlintShell, SceneItem};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn move_pointer(shell: &PetuniaSlintShell, x: f32, y: f32) {
    shell.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(x, y),
    });
}

fn scroll_pointer(shell: &PetuniaSlintShell, x: f32, y: f32, delta_y: f32) {
    shell.window().dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(x, y),
        delta_x: 0.0,
        delta_y,
    });
}

#[test]
fn viewport_shortcut_drag_parametric_hover_and_navigation_gesture() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_label_parts("Parts".into());
    shell.set_label_search_parts("Search parts".into());
    shell.set_label_selected_parts_only("Show selected parts only".into());
    shell.set_label_sort_parts("Sort parts by name".into());
    shell.set_label_parts_row_size("Row size".into());
    let test_part = SceneItem {
        id: "part-cube".into(),
        name: "Cube".into(),
        visible: true,
        locked: false,
        selected: true,
        active: true,
        verts: 8,
        tris: 12,
    };
    let parts_model = std::rc::Rc::new(slint::VecModel::from(vec![test_part]));
    shell.set_scene_items(parts_model.clone().into());
    shell.set_parts_items(parts_model.into());
    shell.set_active_tool("move".into());
    shell.set_gizmo_hover_axis(-1);
    shell.set_transform_instant_active(true);

    let updates = Rc::new(Cell::new(0));
    let ends = Rc::new(Cell::new(0));
    let updates_callback = Rc::clone(&updates);
    shell.on_viewport_transform_update(move |_, _, _, _| {
        updates_callback.set(updates_callback.get() + 1);
    });
    let ends_callback = Rc::clone(&ends);
    shell.on_viewport_transform_end(move || {
        ends_callback.set(ends_callback.get() + 1);
    });

    move_pointer(&shell, 600.0, 400.0);
    updates.set(0);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(600.0, 400.0),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, 630.0, 420.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(630.0, 420.0),
        button: PointerEventButton::Left,
    });
    assert!(
        updates.get() > 0,
        "o mesmo click-drag precisa atualizar Move"
    );
    assert_eq!(ends.get(), 1, "o release precisa confirmar o Move");

    shell.set_transform_instant_active(false);
    shell.set_active_tool("select".into());
    shell.set_tool_modal_active(true);
    shell.set_keyboard_tool_modal_active(true);
    shell.set_tool_activation("drag".into());
    let parametric_updates = Rc::new(Cell::new(0));
    let parametric_callback = Rc::clone(&parametric_updates);
    shell.on_tool_modal_hovered(move |_, _, _| {
        parametric_callback.set(parametric_callback.get() + 1);
    });
    let viewport_hovers = Rc::new(Cell::new(0));
    let viewport_hovers_callback = Rc::clone(&viewport_hovers);
    shell.on_viewport_hover(move |_, _| {
        viewport_hovers_callback.set(viewport_hovers_callback.get() + 1);
    });
    move_pointer(&shell, 620.0, 420.0);
    move_pointer(&shell, 625.0, 390.0);
    assert!(
        viewport_hovers.get() > 0,
        "pré-seleção precisa reagir ao hover livre"
    );
    assert!(
        parametric_updates.get() > 0,
        "atalho paramétrico precisa manipular sem clique prévio"
    );

    shell.set_tool_modal_active(false);
    shell.set_keyboard_tool_modal_active(false);
    shell.set_view_gizmo_origin_x(896.0);
    shell.set_view_gizmo_origin_y(108.0);
    let orbits = Rc::new(Cell::new(0));
    let orbit_callback = Rc::clone(&orbits);
    shell.on_viewport_orbit(move |_, _| {
        orbit_callback.set(orbit_callback.get() + 1);
    });
    move_pointer(&shell, 936.0, 150.0);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(936.0, 150.0),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, 952.0, 166.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(952.0, 166.0),
        button: PointerEventButton::Left,
    });
    assert!(orbits.get() > 0, "arrastar o tripé deve orbitar a câmera");

    let shading_button =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, "Shading options")
            .next()
            .expect("botão de opções de shading acessível");
    let popover_closes = Rc::new(Cell::new(0));
    let close_callback = Rc::clone(&popover_closes);
    let weak_shell = shell.as_weak();
    shell.on_shading_popover_toggled(move |open| {
        if !open {
            close_callback.set(close_callback.get() + 1);
        }
        weak_shell.unwrap().set_shading_popover_open(open);
    });
    let button_origin = shading_button.absolute_position();
    let button_size = shading_button.size();
    let button_center = LogicalPosition::new(
        button_origin.x + button_size.width * 0.5,
        button_origin.y + button_size.height * 0.5,
    );
    move_pointer(&shell, button_center.x, button_center.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: button_center,
        button: PointerEventButton::Left,
    });
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: button_center,
        button: PointerEventButton::Left,
    });
    assert!(
        shell.get_shading_popover_open(),
        "botão deve abrir o popover"
    );

    shell.set_label_xray_opacity("X-Ray opacity".into());
    let slider =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, "X-Ray opacity")
            .next()
            .expect("slider de X-Ray acessível");
    let origin = slider.absolute_position();
    let size = slider.size();
    assert!(size.width >= 110.0, "slider precisa de trilho utilizável");
    let start = LogicalPosition::new(origin.x + 10.0, origin.y + size.height * 0.5);
    let end = LogicalPosition::new(origin.x + size.width - 10.0, start.y);
    let opacity_updates = Rc::new(Cell::new(0));
    let opacity_callback = Rc::clone(&opacity_updates);
    shell.on_xray_opacity_set(move |_| {
        opacity_callback.set(opacity_callback.get() + 1);
    });
    move_pointer(&shell, start.x, start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, end.x, end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });
    assert!(
        opacity_updates.get() > 0,
        "arrasto precisa ajustar opacidade"
    );
    assert_eq!(popover_closes.get(), 0, "slider não deve fechar o popover");

    shell.set_shading_popover_open(false);
    let parts_search = ElementHandle::find_by_accessible_label(&shell, "Search parts")
        .find(|element| element.accessible_role() == Some(AccessibleRole::Search))
        .expect("busca acessível de Parts");
    let selected_filter =
        ElementHandle::find_by_accessible_label(&shell, "Show selected parts only")
            .next()
            .expect("filtro acessível de Parts");
    let sort_toggle = ElementHandle::find_by_accessible_label(&shell, "Sort parts by name")
        .next()
        .expect("ordenação acessível de Parts");
    assert!(
        (parts_search.absolute_position().y - selected_filter.absolute_position().y).abs() < 0.5,
        "busca e filtro devem compartilhar a mesma faixa"
    );
    assert!(
        (parts_search.absolute_position().y - sort_toggle.absolute_position().y).abs() < 0.5,
        "busca e ordenação devem compartilhar a mesma faixa"
    );

    let row_size_slider = ElementHandle::find_by_accessible_label(&shell, "Row size")
        .find(|element| {
            element.accessible_role() == Some(AccessibleRole::Slider)
                && element.accessible_id().as_deref() == Some("model-inspector-parts-row-height")
        })
        .expect("slider acessível de tamanho das linhas");
    assert_eq!(row_size_slider.accessible_value_minimum(), Some(28.0));
    assert_eq!(row_size_slider.accessible_value_maximum(), Some(44.0));
    assert_eq!(row_size_slider.accessible_value_step(), Some(1.0));
    assert_eq!(row_size_slider.accessible_value().unwrap().as_str(), "28");
    assert!(
        row_size_slider.size().width >= 100.0,
        "slider precisa de trilho utilizável"
    );

    let row = ElementHandle::find_by_accessible_label(&shell, "Cube")
        .find(|element| element.accessible_role() == Some(AccessibleRole::ListItem))
        .expect("item da lista virtualizada Parts");
    assert_eq!(row.size().height, 28.0);

    let row_size_changes = Rc::new(Cell::new(0));
    let row_size_callback = Rc::clone(&row_size_changes);
    let weak_shell = shell.as_weak();
    shell.on_parts_row_height_changed(move |height| {
        row_size_callback.set(row_size_callback.get() + 1);
        if let Some(shell) = weak_shell.upgrade() {
            shell.set_parts_row_height(height);
        }
    });
    let slider_origin = row_size_slider.absolute_position();
    let slider_size = row_size_slider.size();
    let start = LogicalPosition::new(
        slider_origin.x + slider_size.width * 0.25,
        slider_origin.y + slider_size.height * 0.5,
    );
    let end = LogicalPosition::new(slider_origin.x + slider_size.width * 0.9, start.y);
    move_pointer(&shell, start.x, start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, end.x, end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });

    assert!(
        row_size_changes.get() > 0,
        "arrastar deve alterar o tamanho das linhas"
    );
    assert!(
        shell.get_parts_row_height() >= 43.0,
        "o novo valor deve chegar ao shell"
    );
    assert_eq!(row.size().height, shell.get_parts_row_height());
    assert_eq!(
        row_size_slider.accessible_value().unwrap().as_str(),
        format!("{}", shell.get_parts_row_height() as i32).as_str()
    );

    shell.window().set_size(LogicalSize::new(800.0, 800.0));
    shell.set_compact_shell(true);
    shell.set_inspector_visible(false);
    shell.set_scene_drawer_visible(true);

    let compact_search = ElementHandle::find_by_accessible_label(&shell, "Search parts")
        .find(|element| {
            element.accessible_label().as_deref() == Some("Search parts")
                && element.accessible_role() == Some(AccessibleRole::Search)
                && element.accessible_id().as_deref() == Some("compact-parts-search")
        })
        .expect("busca do drawer compacto");
    assert_eq!(
        compact_search.accessible_label().as_deref(),
        Some("Search parts")
    );
    assert_eq!(
        compact_search.accessible_role(),
        Some(AccessibleRole::Search)
    );
    let compact_filter =
        ElementHandle::find_by_accessible_label(&shell, "Show selected parts only")
            .next()
            .expect("filtro do drawer compacto");
    let compact_sort = ElementHandle::find_by_accessible_label(&shell, "Sort parts by name")
        .next()
        .expect("ordenação do drawer compacto");
    assert!(
        compact_search.size().width > 0.0,
        "busca compacta precisa estar visível"
    );
    assert_eq!(
        compact_search.absolute_position().y,
        compact_filter.absolute_position().y
    );
    assert_eq!(
        compact_search.absolute_position().y,
        compact_sort.absolute_position().y
    );

    let compact_row_size_slider = ElementHandle::find_by_accessible_label(&shell, "Row size")
        .find(|element| {
            element.accessible_role() == Some(AccessibleRole::Slider)
                && element.accessible_id().as_deref() == Some("compact-parts-row-height")
        })
        .expect("slider do drawer compacto");
    assert!(compact_row_size_slider.size().width >= 100.0);
    assert_eq!(
        compact_row_size_slider.accessible_value().unwrap().as_str(),
        format!("{}", shell.get_parts_row_height() as i32).as_str()
    );
    let compact_row = ElementHandle::find_by_accessible_label(&shell, "Cube")
        .find(|element| element.accessible_role() == Some(AccessibleRole::ListItem))
        .expect("item da lista compacta");
    assert_eq!(compact_row.size().height, shell.get_parts_row_height());

    let compact_slider_origin = compact_row_size_slider.absolute_position();
    let compact_slider_size = compact_row_size_slider.size();
    let compact_drag_start = LogicalPosition::new(
        compact_slider_origin.x + compact_slider_size.width * 0.9,
        compact_slider_origin.y + compact_slider_size.height * 0.5,
    );
    let compact_drag_end = LogicalPosition::new(
        compact_slider_origin.x + compact_slider_size.width * 0.1,
        compact_drag_start.y,
    );
    move_pointer(&shell, compact_drag_start.x, compact_drag_start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: compact_drag_start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, compact_drag_end.x, compact_drag_end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: compact_drag_end,
        button: PointerEventButton::Left,
    });
    assert!(shell.get_parts_row_height() <= 29.0);
    assert_eq!(compact_row.size().height, shell.get_parts_row_height());
}

#[test]
fn loop_cut_armed_tool_routes_hover_scroll_and_click_without_navigating() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_active_tool("loop_cut".into());
    shell.set_loop_cut_armed(true);
    shell.set_loop_cut_cuts(1);

    let zooms = Rc::new(Cell::new(0));
    let zoom_callback = Rc::clone(&zooms);
    shell.on_viewport_zoom(move |_| zoom_callback.set(zoom_callback.get() + 1));
    let counts = Rc::new(Cell::new(0));
    let count_callback = Rc::clone(&counts);
    shell.on_loop_cut_count_committed(move |_| count_callback.set(count_callback.get() + 1));

    scroll_pointer(&shell, 600.0, 400.0, 1.0);
    assert_eq!(counts.get(), 1, "scroll ajusta Cuts durante Loop Cut");
    assert_eq!(
        zooms.get(),
        0,
        "scroll não deve navegar durante a ferramenta"
    );
}

#[test]
fn inspector_collapsed_rail_uses_spaced_icon_pills_and_single_tool_card() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_label_tab_parts("Parts".into());
    shell.set_label_tab_transform("Transform".into());
    shell.set_label_tab_material("Material".into());
    shell.set_label_tab_object("Object".into());
    shell.set_label_tab_modifiers("Modifiers".into());
    shell.set_label_tool_options_collapse("Collapse tool options".into());
    shell.set_label_tool_options_expand("Expand tool options".into());

    // Rail colapsado: 5 pílulas de ícone separadas, sem textos rotacionados.
    shell.set_inspector_collapsed(true);
    let mut tops = Vec::new();
    for label in ["Parts", "Transform", "Material", "Object", "Modifiers"] {
        let pill = ElementHandle::find_by_accessible_label(&shell, label)
            .find(|element| {
                element.accessible_role() == Some(AccessibleRole::Button)
                    && (element.size().width - 36.0).abs() < 0.5
                    && (element.size().height - 36.0).abs() < 0.5
            })
            .unwrap_or_else(|| panic!("pílula 36x36 {label} ausente no rail colapsado"));
        tops.push(pill.absolute_position().y + pill.size().height);
    }
    for (i, pair) in tops.windows(2).enumerate() {
        assert!(
            pair[1] - pair[0] >= 8.0,
            "pílulas {i} e {} precisam de respiro",
            i + 1
        );
    }

    // Card único de ferramenta: oculto em repouso.
    shell.set_inspector_collapsed(false);
    shell.set_tool_modal_active(false);
    shell.set_loop_cut_active(false);
    shell.set_loop_cut_armed(false);
    shell.set_profile_active(false);
    shell.set_operation_hud_active(false);
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Collapse tool options")
            .next()
            .is_none(),
        "sem modal/HUD não há card de ferramenta"
    );

    // Durante o modal, exatamente um card com controle de colapso.
    shell.set_tool_modal_active(true);
    shell.set_tool_options_title("Bevel".into());
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Collapse tool options")
            .next()
            .is_some(),
        "card único deve aparecer durante o modal"
    );
}
