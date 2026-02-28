use gpui::{MouseButton, MouseDownEvent, Pixels, Window, div, prelude::*};

use crate::{theme, window_controls::WindowsWindowControls};

fn on_titlebar_mouse_down(event: &MouseDownEvent, window: &mut Window, _cx: &mut gpui::App) {
    match event.button {
        MouseButton::Left => window.start_window_move(),
        MouseButton::Right => window.show_window_menu(event.position),
        _ => {}
    }
}

pub fn platform_titlebar(height: Pixels, content: impl IntoElement) -> impl IntoElement {
    div()
        .h(height)
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .bg(theme::background())
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .items_center()
                .flex_1()
                .h_full()
                .px_3()
                .on_any_mouse_down(on_titlebar_mouse_down)
                .child(content),
        )
        .when(cfg!(target_os = "windows"), move |this| {
            this.child(WindowsWindowControls::new(height))
        })
}
