use super::{HomeView, QuickInputPosition, SidePosition, ThemeMode};
use gpui::{div, prelude::*, px, AnyElement, Context, MouseButton};

impl HomeView {
    fn render_checkbox_row(
        &self,
        label: &'static str,
        checked: bool,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .h(px(30.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .border_b_1()
            .border_color(self.theme.border)
            .bg(self.theme.background)
            .text_color(self.theme.foreground)
            .text_size(px(13.0))
            .child(
                div()
                    .size(px(14.0))
                    .border_1()
                    .border_color(self.theme.border)
                    .bg(if checked {
                        self.theme.primary
                    } else {
                        self.theme.background
                    })
                    .text_color(self.theme.primary_foreground)
                    .text_size(px(11.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if checked { "✓" } else { "" }),
            )
            .child(label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event, _window, cx| {
                    on_click(view, cx);
                }),
            )
            .into_any_element()
    }

    fn render_option_row(
        &self,
        label: &'static str,
        selected: bool,
        right_label: Option<&'static str>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .h(px(30.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .border_b_1()
            .border_color(self.theme.border)
            .bg(self.theme.background)
            .text_color(self.theme.foreground)
            .text_size(px(13.0))
            .child(div().flex().items_center().gap(px(8.0)).child(label).child(if selected {
                "✓"
            } else {
                ""
            }))
            .when_some(right_label, |this, text| {
                this.child(
                    div().text_size(px(12.0)).text_color(self.theme.muted_foreground).child(text),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event, _window, cx| {
                    on_click(view, cx);
                }),
            )
            .into_any_element()
    }

    pub(super) fn render_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        let light_active = self.theme_mode == ThemeMode::Light;
        let dark_active = self.theme_mode == ThemeMode::Dark;

        let mut card = div()
            .w(px(640.0))
            .flex()
            .flex_col()
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.card)
            .child(
                div()
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_b_1()
                    .border_color(self.theme.border)
                    .bg(self.theme.muted)
                    .text_color(self.theme.foreground)
                    .text_size(px(14.0))
                    .child("Customize Layout"),
            )
            .child(self.render_checkbox_row(
                "Menu Bar",
                self.show_top_bar,
                |v, cx| {
                    v.show_top_bar = !v.show_top_bar;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_checkbox_row(
                "Action Bar",
                self.show_action_bar,
                |v, cx| {
                    v.show_action_bar = !v.show_action_bar;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_checkbox_row(
                "Primary Side Bar",
                self.show_primary_sidebar,
                |v, cx| {
                    v.show_primary_sidebar = !v.show_primary_sidebar;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_checkbox_row(
                "Secondary Side Bar",
                self.show_secondary_sidebar,
                |v, cx| {
                    v.show_secondary_sidebar = !v.show_secondary_sidebar;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_checkbox_row(
                "Status Bar",
                self.show_status_bar,
                |v, cx| {
                    v.show_status_bar = !v.show_status_bar;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_option_row(
                "Left",
                self.primary_sidebar_position == SidePosition::Left,
                Some("Primary Side Bar Position"),
                |v, cx| {
                    v.primary_sidebar_position = SidePosition::Left;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_option_row(
                "Right",
                self.primary_sidebar_position == SidePosition::Right,
                None,
                |v, cx| {
                    v.primary_sidebar_position = SidePosition::Right;
                    v.touch_layout();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_option_row(
                "Top",
                self.quick_input_position == QuickInputPosition::Top,
                Some("Quick Input Position"),
                |v, cx| {
                    v.quick_input_position = QuickInputPosition::Top;
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_option_row(
                "Center",
                self.quick_input_position == QuickInputPosition::Center,
                None,
                |v, cx| {
                    v.quick_input_position = QuickInputPosition::Center;
                    cx.notify();
                },
                cx,
            ));

        card = card.child(
            div()
                .h(px(44.0))
                .px(px(12.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .border_t_1()
                .border_color(self.theme.border)
                .bg(self.theme.background)
                .child(
                    div()
                        .h(px(32.0))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .border_1()
                        .border_color(self.theme.border)
                        .bg(self.theme.background)
                        .text_color(self.theme.foreground)
                        .text_size(px(13.0))
                        .child("Toggle Theme")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _event, window, cx| {
                                view.toggle_theme(window, cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .h(px(32.0))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .border_1()
                        .border_color(self.theme.border)
                        .bg(if light_active {
                            self.theme.primary
                        } else {
                            self.theme.background
                        })
                        .text_color(if light_active {
                            self.theme.primary_foreground
                        } else {
                            self.theme.foreground
                        })
                        .text_size(px(13.0))
                        .child("Light")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _event, window, cx| {
                                view.set_theme_mode(ThemeMode::Light, window, cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .h(px(32.0))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .border_1()
                        .border_color(self.theme.border)
                        .bg(if dark_active {
                            self.theme.primary
                        } else {
                            self.theme.background
                        })
                        .text_color(if dark_active {
                            self.theme.primary_foreground
                        } else {
                            self.theme.foreground
                        })
                        .text_size(px(13.0))
                        .child("Dark")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _event, window, cx| {
                                view.set_theme_mode(ThemeMode::Dark, window, cx);
                            }),
                        ),
                ),
        );

        card.into_any_element()
    }

    #[allow(dead_code)]
    pub(super) fn render_controls_stage(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut stage = div()
            .size_full()
            .bg(self.theme.background)
            .flex()
            .justify_center()
            .overflow_hidden();

        stage = match self.quick_input_position {
            QuickInputPosition::Top => stage.items_start().pt(px(24.0)),
            QuickInputPosition::Center => stage.items_center(),
        };

        stage.child(self.render_controls(cx)).into_any_element()
    }
}
