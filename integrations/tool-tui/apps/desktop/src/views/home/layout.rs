use super::{HomeView, SidePosition};
use gpui::{div, prelude::*, px, AnyElement, Context, Pixels};
use gpui_component::resizable::{h_resizable, resizable_panel, ResizablePanel};

impl HomeView {
    fn render_center_column(&self, cx: &mut Context<Self>, _has_left_border: bool) -> AnyElement {
        div()
            .size_full()
            .bg(self.theme.background)
            .child(self.render_center_stage(cx))
            .into_any_element()
    }

    pub(super) fn render_workspace(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut columns: Vec<ResizablePanel> = Vec::new();

        if self.primary_sidebar_position == SidePosition::Left {
            if self.show_primary_sidebar {
                columns.push(
                    resizable_panel()
                        .size(px(self.primary_sidebar_width_px.max(160.0)))
                        .size_range(px(160.0)..px(400.0))
                        .child(self.render_primary_sidebar(cx)),
                );
            }
        } else if self.show_secondary_sidebar {
            columns.push(
                resizable_panel()
                    .size(px(self.secondary_sidebar_width_px.max(180.0)))
                    .size_range(px(180.0)..px(400.0))
                    .child(self.render_secondary_sidebar(cx)),
            );
        }

        // Center column - explicit size to fill remaining space
        let center_initial_size = {
            let used = if self.show_primary_sidebar {
                self.primary_sidebar_width_px.max(160.0)
            } else {
                0.0
            } + if self.show_secondary_sidebar {
                self.secondary_sidebar_width_px.max(180.0)
            } else {
                0.0
            };
            px(800.0_f32.max(used))
        };

        columns.push(
            resizable_panel()
                .size(center_initial_size)
                .size_range(px(200.0)..px(10000.0))
                .child(self.render_center_column(cx, false)),
        );

        if self.primary_sidebar_position == SidePosition::Left {
            if self.show_secondary_sidebar {
                columns.push(
                    resizable_panel()
                        .size(px(self.secondary_sidebar_width_px.max(180.0)))
                        .size_range(px(180.0)..px(400.0))
                        .child(self.render_secondary_sidebar(cx)),
                );
            }
        } else {
            if self.show_primary_sidebar {
                columns.push(
                    resizable_panel()
                        .size(px(self.primary_sidebar_width_px.max(160.0)))
                        .size_range(px(160.0)..px(400.0))
                        .child(self.render_primary_sidebar(cx)),
                );
            }
        }

        let view = cx.entity().clone();
        let resizable_content = h_resizable(("home-workspace-columns", self.layout_revision))
            .on_resize(move |state, _window, cx| {
                let sizes: Vec<Pixels> = state.read(cx).sizes().clone();
                view.update(cx, |this, _| {
                    this.sync_column_widths_from_sizes(&sizes);
                });
            })
            .children(columns)
            .into_any_element();

        // Wrap with action bar if needed
        let mut container = div().size_full().flex();

        if self.primary_sidebar_position == SidePosition::Left {
            if self.show_action_bar {
                container = container.child(
                    div()
                        .w(px(self.action_bar_width_px.max(44.0)))
                        .h_full()
                        .flex_shrink_0()
                        .bg(self.theme.background)
                        .border_r_1()
                        .border_color(self.theme.border)
                        .child(self.panel_bg(false)),
                );
            }
            container = container.child(div().flex_1().h_full().child(resizable_content));
        } else {
            container = container.child(div().flex_1().h_full().child(resizable_content));
            if self.show_action_bar {
                container = container.child(
                    div()
                        .w(px(self.action_bar_width_px.max(44.0)))
                        .h_full()
                        .flex_shrink_0()
                        .bg(self.theme.background)
                        .border_l_1()
                        .border_color(self.theme.border)
                        .child(self.panel_bg(false)),
                );
            }
        }

        container.into_any_element()
    }

    pub(super) fn render_main_with_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        self.render_workspace(cx)
    }
}
