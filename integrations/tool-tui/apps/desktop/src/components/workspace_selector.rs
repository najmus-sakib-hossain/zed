use crate::components::Icon;
use crate::theme::Theme;
use gpui::{div, prelude::*, px, Context, IntoElement, SharedString, Window};

#[derive(Clone, Debug)]
pub struct Workspace {
    pub name: SharedString,
    pub path: SharedString,
}

pub struct WorkspaceSelector {
    theme: Theme,
    workspaces: Vec<Workspace>,
    selected_index: usize,
    is_open: bool,
}

impl WorkspaceSelector {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            workspaces: vec![
                Workspace {
                    name: "recipe-app".into(),
                    path: "~/projects/recipe-app".into(),
                },
                Workspace {
                    name: "My Skills".into(),
                    path: "~/projects/skills".into(),
                },
                Workspace {
                    name: "photobooth".into(),
                    path: "~/projects/photobooth".into(),
                },
                Workspace {
                    name: "developers-website".into(),
                    path: "~/projects/developers-website".into(),
                },
                Workspace {
                    name: "wanderlust".into(),
                    path: "~/projects/wanderlust".into(),
                },
                Workspace {
                    name: "openai-apps-sdk-examples".into(),
                    path: "~/projects/openai-apps-sdk-examples".into(),
                },
                Workspace {
                    name: "game-experiment".into(),
                    path: "~/projects/game-experiment".into(),
                },
            ],
            selected_index: 2, // photobooth
            is_open: false,
        }
    }

    pub fn toggle_dropdown(&mut self, cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
        cx.notify();
    }

    pub fn select_workspace(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_index = index;
        self.is_open = false;
        cx.notify();
    }

    pub fn add_workspace(&mut self, cx: &mut Context<Self>) {
        // TODO: Open file picker dialog
        self.is_open = false;
        cx.notify();
    }

    fn render_trigger(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = &self.workspaces[self.selected_index];
        let theme = &self.theme;

        div()
            .id("workspace-trigger")
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .cursor_pointer()
            .hover(|style| style.bg(theme.muted))
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.toggle_dropdown(cx);
            }))
            .child(Icon::new("folder").size(px(16.0)).color(theme.foreground).render(theme))
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme.foreground)
                    .child(selected.name.clone()),
            )
            .child(
                Icon::new("chevron-down")
                    .size(px(16.0))
                    .color(theme.muted_foreground)
                    .render(theme),
            )
    }

    fn render_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let mut dropdown = div()
            .absolute()
            .top(px(48.0))
            .left(px(0.0))
            .w(px(400.0))
            .max_h(px(500.0))
            .overflow_y_hidden()
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .shadow_lg()
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(2.0));

        // Header
        dropdown = dropdown.child(
            div()
                .px(px(8.0))
                .py(px(6.0))
                .text_size(px(12.0))
                .text_color(theme.muted_foreground)
                .child("Select your workspace"),
        );

        // Workspace list
        for (index, workspace) in self.workspaces.iter().enumerate() {
            let is_selected = index == self.selected_index;
            dropdown =
                dropdown.child(self.render_workspace_item(workspace, index, is_selected, cx));
        }

        // Add new workspace
        dropdown = dropdown.child(
            div()
                .id("add-workspace")
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .py(px(8.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .hover(|style| style.bg(theme.accent))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.add_workspace(cx);
                }))
                .child(
                    Icon::new("folder-plus").size(px(16.0)).color(theme.foreground).render(theme),
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.foreground)
                        .child("Add new workspace"),
                ),
        );

        dropdown
    }

    fn render_workspace_item(
        &self,
        workspace: &Workspace,
        index: usize,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = &self.theme;
        let workspace_name = workspace.name.clone();

        let mut item = div()
            .id(("workspace-item", index))
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .hover(|style| style.bg(theme.accent))
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.select_workspace(index, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(Icon::new("folder").size(px(16.0)).color(theme.foreground).render(theme))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.foreground)
                            .child(workspace_name),
                    ),
            );

        if is_selected {
            item = item
                .bg(theme.accent)
                .child(Icon::new("check").size(px(16.0)).color(theme.foreground).render(theme));
        }

        item
    }
}

impl gpui::Render for WorkspaceSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut container = div().relative().child(self.render_trigger(cx));

        if self.is_open {
            container = container.child(self.render_dropdown(cx));
        }

        container
    }
}
