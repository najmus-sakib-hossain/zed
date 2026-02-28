use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── HoverCard ──────────────────────────────────────────────────────────────
// A shadcn-ui style HoverCard for rich content on hover.

pub struct HoverCard {
    children: Vec<AnyElement>,
    width: gpui::Pixels,
}

impl HoverCard {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            width: px(280.0),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut card = div()
            .flex()
            .flex_col()
            .w(self.width)
            .p(px(16.0))
            .rounded(Radius::LG)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border);

        for child in self.children {
            card = card.child(child);
        }

        card
    }
}

// ─── EmptyState ─────────────────────────────────────────────────────────────
// A centered empty state placeholder with icon, title, and description.

pub struct EmptyState {
    icon: Option<String>,
    title: String,
    description: Option<String>,
    action: Option<AnyElement>,
}

impl EmptyState {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            icon: None,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    #[allow(dead_code)]
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .py(px(48.0));

        if let Some(icon) = self.icon {
            container = container.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(48.0))
                    .rounded(Radius::FULL)
                    .bg(theme.muted)
                    .child(
                        div().text_size(px(24.0)).text_color(theme.muted_foreground).child(icon),
                    ),
            );
        }

        container = container.child(
            div()
                .text_size(px(16.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(desc) = self.description {
            container = container.child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .max_w(px(320.0))
                    .child(desc),
            );
        }

        if let Some(action) = self.action {
            container = container.child(div().mt(px(8.0)).child(action));
        }

        container
    }
}

// ─── Stat ───────────────────────────────────────────────────────────────────
// A stat/metric display card.

pub struct Stat {
    label: String,
    value: String,
    description: Option<String>,
    trend: Option<StatTrend>,
}

#[derive(Debug, Clone)]
pub enum StatTrend {
    Up(String),
    Down(String),
    Neutral(String),
}

#[allow(dead_code)]
impl Stat {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
            trend: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn trend(mut self, trend: StatTrend) -> Self {
        self.trend = Some(trend);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(24.0))
            .rounded(Radius::LG)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border);

        // Label
        container = container.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.muted_foreground)
                .child(self.label),
        );

        // Value + trend
        let mut value_row = div().flex().items_center().gap(px(8.0));

        value_row = value_row.child(
            div()
                .text_size(px(30.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .line_height(px(36.0))
                .child(self.value),
        );

        if let Some(trend) = self.trend {
            let (icon, text, color) = match trend {
                StatTrend::Up(t) => ("↑", t, theme.success),
                StatTrend::Down(t) => ("↓", t, theme.destructive),
                StatTrend::Neutral(t) => ("→", t, theme.muted_foreground),
            };
            value_row = value_row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .text_size(px(12.0))
                    .text_color(color)
                    .child(icon)
                    .child(text),
            );
        }

        container = container.child(value_row);

        if let Some(desc) = self.description {
            container = container
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        container
    }
}

// ─── Breadcrumb ─────────────────────────────────────────────────────────────
// A breadcrumb navigation component.

pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
}

struct BreadcrumbItem {
    label: String,
    is_current: bool,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn item(mut self, label: impl Into<String>) -> Self {
        // Mark all previous items as not current, new one as current
        for item in &mut self.items {
            item.is_current = false;
        }
        self.items.push(BreadcrumbItem {
            label: label.into(),
            is_current: true,
        });
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut breadcrumb = div().flex().items_center().gap(px(4.0));
        let total = self.items.len();

        for (idx, item) in self.items.into_iter().enumerate() {
            let text_color = if item.is_current {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let is_last = idx == total - 1;

            let mut el = div().text_size(px(14.0)).text_color(text_color);

            if !is_last {
                el = el.cursor_pointer().hover(move |style| style.text_color(theme.foreground));
            } else {
                el = el.font_weight(gpui::FontWeight::MEDIUM);
            }

            breadcrumb = breadcrumb.child(el.child(item.label));

            if !is_last {
                breadcrumb = breadcrumb
                    .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child("/"));
            }
        }

        breadcrumb
    }
}

// ─── Pagination ─────────────────────────────────────────────────────────────
// A pagination component.

pub struct Pagination {
    current_page: usize,
    total_pages: usize,
    show_first_last: bool,
}

#[allow(dead_code)]
impl Pagination {
    pub fn new(current: usize, total: usize) -> Self {
        Self {
            current_page: current,
            total_pages: total,
            show_first_last: false,
        }
    }

    pub fn show_first_last(mut self, show: bool) -> Self {
        self.show_first_last = show;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().items_center().gap(px(4.0));

        // Previous
        container =
            container.child(PaginationButton::new("←", self.current_page > 1).render(theme));

        // Page numbers (show up to 5 around current)
        let start = self.current_page.saturating_sub(2).max(1);
        let end = (start + 4).min(self.total_pages);

        if start > 1 {
            container = container.child(PaginationButton::page(1, false).render(theme));
            if start > 2 {
                container = container.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.muted_foreground)
                        .px(px(4.0))
                        .child("..."),
                );
            }
        }

        for page in start..=end {
            let is_current = page == self.current_page;
            container = container.child(PaginationButton::page(page, is_current).render(theme));
        }

        if end < self.total_pages {
            if end < self.total_pages - 1 {
                container = container.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(theme.muted_foreground)
                        .px(px(4.0))
                        .child("..."),
                );
            }
            container =
                container.child(PaginationButton::page(self.total_pages, false).render(theme));
        }

        // Next
        container = container
            .child(PaginationButton::new("→", self.current_page < self.total_pages).render(theme));

        container
    }
}

struct PaginationButton {
    label: String,
    enabled: bool,
    is_current: bool,
}

impl PaginationButton {
    fn new(label: impl Into<String>, enabled: bool) -> Self {
        Self {
            label: label.into(),
            enabled,
            is_current: false,
        }
    }

    fn page(page: usize, current: bool) -> Self {
        Self {
            label: page.to_string(),
            enabled: !current,
            is_current: current,
        }
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.is_current {
            theme.primary
        } else {
            gpui::transparent_black()
        };
        let text_color = if self.is_current {
            theme.primary_foreground
        } else if self.enabled {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let hover_bg = theme.accent;

        let mut btn = div()
            .flex()
            .items_center()
            .justify_center()
            .min_w(px(32.0))
            .h(px(32.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .text_size(px(14.0))
            .text_color(text_color)
            .child(self.label);

        if self.enabled && !self.is_current {
            btn = btn
                .cursor_pointer()
                .hover(move |style| style.bg(hover_bg))
                .border_1()
                .border_color(theme.border);
        } else if !self.enabled {
            btn = btn.opacity(0.5);
        }

        btn
    }
}
