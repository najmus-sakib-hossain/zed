use gpui::*;
use std::time::Duration;

pub struct GlowCard {
    rotation: f32,
}

impl GlowCard {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |card, cx| {
                    card.rotation = (card.rotation + 2.0) % 360.0;
                    cx.notify();
                });
                cx.background_executor().timer(Duration::from_millis(16)).await;
            }
        })
        .detach();

        Self { rotation: 0.0 }
    }
}

impl Render for GlowCard {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Define the specific colors from the reference
        let purple = rgb(0xa855f7);
        let cyan = rgb(0x06b6d4);
        let pink = rgb(0xec4899);
        let orange = rgb(0xf97316);
        let yellow = rgb(0xfbbf24);
        let blue = rgb(0x3b82f6);

        // Calculate intermediate angles for multi-color effect (6 layers for richer colors)
        let angle1 = self.rotation;
        let angle2 = (self.rotation + 60.0) % 360.0;
        let angle3 = (self.rotation + 120.0) % 360.0;
        let angle4 = (self.rotation + 180.0) % 360.0;
        let angle5 = (self.rotation + 240.0) % 360.0;
        let angle6 = (self.rotation + 300.0) % 360.0;

        div()
            .flex()
            .size_full()
            .bg(rgb(0x000000))
            .justify_center()
            .items_center()
            .child(
                // CONTAINER
                div()
                    .relative()
                    .w(px(500.0))
                    .h(px(320.0))
                    // LAYER: Multi-color border effect using layered gradients
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .bg(linear_gradient(
                                angle1,
                                linear_color_stop(purple, 0.0),
                                linear_color_stop(cyan, 1.0),
                            ))
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .opacity(0.8)
                            .bg(linear_gradient(
                                angle2,
                                linear_color_stop(cyan, 0.0),
                                linear_color_stop(blue, 1.0),
                            ))
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .opacity(0.8)
                            .bg(linear_gradient(
                                angle3,
                                linear_color_stop(blue, 0.0),
                                linear_color_stop(pink, 1.0),
                            ))
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .opacity(0.8)
                            .bg(linear_gradient(
                                angle4,
                                linear_color_stop(pink, 0.0),
                                linear_color_stop(orange, 1.0),
                            ))
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .opacity(0.8)
                            .bg(linear_gradient(
                                angle5,
                                linear_color_stop(orange, 0.0),
                                linear_color_stop(yellow, 1.0),
                            ))
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded_xl()
                            .opacity(0.8)
                            .bg(linear_gradient(
                                angle6,
                                linear_color_stop(yellow, 0.0),
                                linear_color_stop(purple, 1.0),
                            ))
                    )
                    // LAYER: The Content Mask (BLACK CARD - covers gradient except border)
                    .child(
                        div()
                            .absolute()
                            .inset(px(3.0))
                            .rounded_xl()
                            .bg(rgb(0x000000))
                            .relative()
                            .overflow_hidden()
                            // INNER GLOW: Layered gradients for fade effect from edges
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .h(px(80.0))
                                    .bg(linear_gradient(
                                        180.0,
                                        linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
                                        linear_color_stop(rgb(0x000000), 1.0),
                                    ))
                            )
                            .child(
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .left_0()
                                    .right_0()
                                    .h(px(80.0))
                                    .bg(linear_gradient(
                                        0.0,
                                        linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
                                        linear_color_stop(rgb(0x000000), 1.0),
                                    ))
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left_0()
                                    .w(px(80.0))
                                    .bg(linear_gradient(
                                        90.0,
                                        linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
                                        linear_color_stop(rgb(0x000000), 1.0),
                                    ))
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .right_0()
                                    .w(px(80.0))
                                    .bg(linear_gradient(
                                        270.0,
                                        linear_color_stop(hsla(0.0, 0.0, 0.0, 0.0), 0.0),
                                        linear_color_stop(rgb(0x000000), 1.0),
                                    ))
                            )
                            // Floating action buttons
                            .child(
                                div()
                                    .absolute()
                                    .bottom(px(20.0))
                                    .right(px(20.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.0))
                                    .child(icon_button("expand"))
                                    .child(icon_button("sparkle")),
                            ),
                    ),
            )
    }
}

fn icon_button(icon_type: &str) -> impl IntoElement {
    let path = match icon_type {
        "expand" => "M15 3h6v6M14 10l6.1-6.1M9 21H3v-6M10 14l-6.1 6.1",
        _ => "M9.9 14.1L5 19M20 10c0 5.5-4.5 10-10 10S0 15.5 0 10 4.5 0 10 0s10 4.5 10 10z",
    };

    div()
        .w(px(40.0))
        .h(px(40.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_xl()
        .bg(hsla(270.0 / 360.0, 0.5, 0.15, 0.6))
        .hover(|s| s.bg(hsla(270.0 / 360.0, 0.5, 0.2, 0.8)))
        .cursor_pointer()
        .child(svg().path(path.to_string()).size(px(20.0)).text_color(rgb(0xffffff)))
}
