//! Gestos reais do shell declarativo: o bridge isolado não cobre hit testing Slint.

use std::cell::Cell;
use std::rc::Rc;

use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn move_pointer(shell: &PetuniaSlintShell, x: f32, y: f32) {
    shell.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(x, y),
    });
}

#[test]
fn viewport_shortcut_drag_parametric_hover_and_navigation_gesture() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
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
}
