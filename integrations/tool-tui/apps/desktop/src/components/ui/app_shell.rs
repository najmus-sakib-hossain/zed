use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::Theme;

// ─── AppShell ───────────────────────────────────────────────────────────────
// Top-level layout shell for desktop applications.
// Provides the standard desktop app structure:
//   ┌──────────────────────────────────┐
//   │           Title Bar              │
//   ├──────────────────────────────────┤
//   │           Toolbar                │
//   ├────┬─────────────────────┬───────┤
//   │    │                     │       │
//   │ A  │     Main Content    │ Right │
//   │ c  │                     │ Panel │
//   │ t  │                     │       │
//   │    │                     │       │
//   ├────┴─────────────────────┴───────┤
//   │           Status Bar             │
//   └──────────────────────────────────┘
//
// Usage:
//   AppShell::new()
//       .titlebar(titlebar)
//       .toolbar(toolbar)
//       .activity_bar(activity_bar)
//       .sidebar(sidebar)
//       .main(main_content)
//       .right_panel(right)
//       .status_bar(status_bar)
//       .render(&theme)

pub struct AppShell {
    titlebar: Option<AnyElement>,
    toolbar: Option<AnyElement>,
    activity_bar: Option<AnyElement>,
    sidebar: Option<AnyElement>,
    main: Option<AnyElement>,
    right_panel: Option<AnyElement>,
    status_bar: Option<AnyElement>,
    bottom_panel: Option<AnyElement>,
}

#[allow(dead_code)]
impl AppShell {
    pub fn new() -> Self {
        Self {
            titlebar: None,
            toolbar: None,
            activity_bar: None,
            sidebar: None,
            main: None,
            right_panel: None,
            status_bar: None,
            bottom_panel: None,
        }
    }

    pub fn titlebar(mut self, element: impl IntoElement) -> Self {
        self.titlebar = Some(element.into_any_element());
        self
    }

    pub fn toolbar(mut self, element: impl IntoElement) -> Self {
        self.toolbar = Some(element.into_any_element());
        self
    }

    pub fn activity_bar(mut self, element: impl IntoElement) -> Self {
        self.activity_bar = Some(element.into_any_element());
        self
    }

    pub fn sidebar(mut self, element: impl IntoElement) -> Self {
        self.sidebar = Some(element.into_any_element());
        self
    }

    pub fn main(mut self, element: impl IntoElement) -> Self {
        self.main = Some(element.into_any_element());
        self
    }

    pub fn right_panel(mut self, element: impl IntoElement) -> Self {
        self.right_panel = Some(element.into_any_element());
        self
    }

    pub fn bottom_panel(mut self, element: impl IntoElement) -> Self {
        self.bottom_panel = Some(element.into_any_element());
        self
    }

    pub fn status_bar(mut self, element: impl IntoElement) -> Self {
        self.status_bar = Some(element.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut shell = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground);

        // Titlebar
        if let Some(titlebar) = self.titlebar {
            shell = shell.child(titlebar);
        }

        // Toolbar
        if let Some(toolbar) = self.toolbar {
            shell = shell.child(toolbar);
        }

        // Middle section: activity bar + sidebar + main + right panel
        let mut middle = div().flex().flex_row().flex_1().overflow_hidden();

        // Activity bar
        if let Some(activity_bar) = self.activity_bar {
            middle = middle.child(activity_bar);
        }

        // Sidebar
        if let Some(sidebar) = self.sidebar {
            middle = middle.child(sidebar);
        }

        // Main content area (with optional bottom panel)
        let mut main_area = div().flex().flex_col().flex_1().overflow_hidden();

        if let Some(main) = self.main {
            main_area = main_area.child(div().flex_1().overflow_hidden().child(main));
        }

        if let Some(bottom_panel) = self.bottom_panel {
            main_area = main_area.child(bottom_panel);
        }

        middle = middle.child(main_area);

        // Right panel
        if let Some(right_panel) = self.right_panel {
            middle = middle.child(right_panel);
        }

        shell = shell.child(middle);

        // Status bar
        if let Some(status_bar) = self.status_bar {
            shell = shell.child(status_bar);
        }

        shell
    }
}

// ─── ContentArea ────────────────────────────────────────────────────────────
// A content area with optional padding, max width, and centering.

pub struct ContentArea {
    children: Vec<AnyElement>,
    max_width: Option<f32>,
    padded: bool,
    centered: bool,
}

#[allow(dead_code)]
impl ContentArea {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            max_width: None,
            padded: true,
            centered: false,
        }
    }

    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.children.push(element.into_any_element());
        self
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut container = div().flex().flex_col().flex_1();

        if self.centered {
            container = container.items_center();
        }

        let mut content = div().flex().flex_col().w_full();

        if let Some(max_w) = self.max_width {
            content = content.max_w(px(max_w));
        }

        if self.padded {
            content = content.p(px(24.0));
        }

        for child in self.children {
            content = content.child(child);
        }

        container.child(content)
    }
}

// ─── PageHeader ─────────────────────────────────────────────────────────────
// Header for a page/view with title, description, and action area.

pub struct PageHeader {
    title: String,
    description: Option<String>,
    actions: Vec<AnyElement>,
    breadcrumbs: Vec<String>,
}

#[allow(dead_code)]
impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            actions: Vec::new(),
            breadcrumbs: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    pub fn breadcrumb(mut self, crumb: impl Into<String>) -> Self {
        self.breadcrumbs.push(crumb.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut header = div().flex().flex_col().gap(px(8.0)).pb(px(16.0));

        // Breadcrumbs
        if !self.breadcrumbs.is_empty() {
            let mut crumbs = div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(12.0))
                .text_color(theme.muted_foreground);

            for (i, crumb) in self.breadcrumbs.iter().enumerate() {
                if i > 0 {
                    crumbs = crumbs.child(div().child("/").mx(px(2.0)));
                }
                crumbs = crumbs.child(
                    div()
                        .cursor_pointer()
                        .hover(move |style| style.text_color(theme.foreground))
                        .child(crumb.clone()),
                );
            }

            header = header.child(crumbs);
        }

        // Title + Actions row
        let mut title_row = div().flex().items_center().justify_between();

        let mut title_area = div().flex().flex_col().gap(px(4.0));

        title_area = title_area.child(
            div()
                .text_size(px(24.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(desc) = self.description {
            title_area = title_area
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child(desc));
        }

        title_row = title_row.child(title_area);

        if !self.actions.is_empty() {
            let mut actions = div().flex().items_center().gap(px(8.0));
            for action in self.actions {
                actions = actions.child(action);
            }
            title_row = title_row.child(actions);
        }

        header.child(title_row)
    }
}
