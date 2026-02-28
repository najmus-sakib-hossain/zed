use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Table ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Table component for displaying tabular data.
//
// Usage:
//   Table::new()
//       .header(vec!["Name", "Status", "Email"])
//       .row(vec!["John", "Active", "john@example.com"])
//       .row(vec!["Jane", "Inactive", "jane@example.com"])
//       .render(&theme)

pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    caption: Option<String>,
    striped: bool,
}

impl Table {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            caption: None,
            striped: false,
        }
    }

    pub fn header(mut self, headers: Vec<impl Into<String>>) -> Self {
        self.headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }

    pub fn row(mut self, cells: Vec<impl Into<String>>) -> Self {
        self.rows.push(cells.into_iter().map(|c| c.into()).collect());
        self
    }

    #[allow(dead_code)]
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    #[allow(dead_code)]
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut table = div()
            .flex()
            .flex_col()
            .w_full()
            .rounded(Radius::LG)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        // Header
        if !self.headers.is_empty() {
            let mut header_row = div().flex().items_center().w_full().bg(theme.muted);

            for header in &self.headers {
                header_row = header_row.child(
                    div()
                        .flex_1()
                        .px(px(16.0))
                        .py(px(12.0))
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.muted_foreground)
                        .child(header.clone()),
                );
            }

            table = table.child(header_row);
        }

        // Rows
        for (idx, row) in self.rows.iter().enumerate() {
            let row_bg = if self.striped && idx % 2 == 1 {
                theme.muted
            } else {
                gpui::transparent_black()
            };
            let hover_bg = theme.accent;

            let mut row_el = div()
                .flex()
                .items_center()
                .w_full()
                .bg(row_bg)
                .border_t_1()
                .border_color(theme.border)
                .hover(move |style| style.bg(hover_bg));

            for cell in row {
                row_el = row_el.child(
                    div()
                        .flex_1()
                        .px(px(16.0))
                        .py(px(12.0))
                        .text_size(px(14.0))
                        .text_color(theme.foreground)
                        .child(cell.clone()),
                );
            }

            table = table.child(row_el);
        }

        // Caption
        let mut container = div().flex().flex_col().gap(px(8.0));
        container = container.child(table);

        if let Some(caption) = self.caption {
            container = container.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .child(caption),
            );
        }

        container
    }
}

// ─── DataTable (key-value pairs) ────────────────────────────────────────────
// For displaying property-value pairs compactly.

pub struct DataTable {
    rows: Vec<(String, AnyElement)>,
}

impl DataTable {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn row(mut self, label: impl Into<String>, value: impl IntoElement) -> Self {
        self.rows.push((label.into(), value.into_any_element()));
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut table = div().flex().flex_col().w_full();

        for (label, value) in self.rows {
            table = table.child(
                div()
                    .flex()
                    .items_center()
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .w(px(140.0))
                            .flex_shrink_0()
                            .text_size(px(14.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(label),
                    )
                    .child(div().flex_1().child(value)),
            );
        }

        table
    }
}

// ─── List ───────────────────────────────────────────────────────────────────
// A generic List component for rendering items.

pub struct List {
    items: Vec<AnyElement>,
    divided: bool,
}

#[allow(dead_code)]
impl List {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            divided: true,
        }
    }

    pub fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }

    pub fn divided(mut self, divided: bool) -> Self {
        self.divided = divided;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut list = div().flex().flex_col().w_full();

        for (idx, item) in self.items.into_iter().enumerate() {
            let mut row = div().w_full();
            if self.divided && idx > 0 {
                row = row.border_t_1().border_color(theme.border);
            }
            row = row.child(item);
            list = list.child(row);
        }

        list
    }
}
