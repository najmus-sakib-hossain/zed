use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── MenuBar ────────────────────────────────────────────────────────────────
// Traditional desktop application menu bar (File, Edit, View, Help, etc.)
//
// Usage:
//   MenuBar::new()
//       .item(MenuBarItem::new("File").active(true))
//       .item(MenuBarItem::new("Edit"))
//       .item(MenuBarItem::new("View"))
//       .item(MenuBarItem::new("Help"))
//       .render(&theme)

pub struct MenuBar {
    items: Vec<MenuBarItem>,
    // Whether menu bar is part of the window frame (transparent bg)
    integrated: bool,
}

#[allow(dead_code)]
impl MenuBar {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            integrated: false,
        }
    }

    pub fn item(mut self, item: MenuBarItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<MenuBarItem>) -> Self {
        self.items = items;
        self
    }

    pub fn integrated(mut self, v: bool) -> Self {
        self.integrated = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.integrated {
            gpui::transparent_black()
        } else {
            theme.background
        };
        let border_color = theme.border;

        let mut bar = div()
            .flex()
            .items_center()
            .h(px(30.0))
            .px(px(4.0))
            .bg(bg)
            .border_b_1()
            .border_color(border_color)
            .text_size(px(12.0));

        for item in self.items {
            bar = bar.child(item.render(theme));
        }

        bar
    }
}

pub struct MenuBarItem {
    label: String,
    active: bool,
    disabled: bool,
}

#[allow(dead_code)]
impl MenuBarItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            active: false,
            disabled: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_bg = theme.accent;
        let text_color = if self.disabled {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let bg = if self.active {
            theme.accent
        } else {
            gpui::transparent_black()
        };

        let mut el = div()
            .px(px(8.0))
            .py(px(4.0))
            .rounded(Radius::SM)
            .bg(bg)
            .text_color(text_color)
            .text_size(px(12.0))
            .child(self.label);

        if !self.disabled {
            el = el.cursor_pointer().hover(move |s| s.bg(hover_bg));
        }

        el
    }
}

// ─── Slider ─────────────────────────────────────────────────────────────────
// A range slider that displays a track and a filled segment.
//
// Usage:
//   Slider::new("volume")
//       .value(0.75)
//       .min(0.0)
//       .max(1.0)
//       .render(&theme)

pub struct Slider {
    _id: String,
    value: f32,
    min: f32,
    max: f32,
    disabled: bool,
    show_value: bool,
    label: Option<String>,
    suffix: Option<String>,
}

#[allow(dead_code)]
impl Slider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            _id: id.into(),
            value: 0.5,
            min: 0.0,
            max: 1.0,
            disabled: false,
            show_value: false,
            label: None,
            suffix: None,
        }
    }

    pub fn value(mut self, v: f32) -> Self {
        self.value = v;
        self
    }

    pub fn min(mut self, v: f32) -> Self {
        self.min = v;
        self
    }

    pub fn max(mut self, v: f32) -> Self {
        self.max = v;
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn show_value(mut self, v: bool) -> Self {
        self.show_value = v;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = Some(s.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let range = self.max - self.min;
        let percent = if range > 0.0 {
            ((self.value - self.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let track_h = 6.0;
        let thumb_size = 16.0;
        let track_bg = theme.muted;
        let fill_bg = theme.primary;
        let thumb_bg = theme.background;
        let thumb_border = theme.primary;

        let mut container = div().flex().flex_col().gap(px(6.0)).w_full();

        // Label row
        if self.label.is_some() || self.show_value {
            let mut label_row =
                div().flex().items_center().justify_between().w_full().text_size(px(12.0));

            if let Some(label) = self.label {
                label_row = label_row.child(
                    div()
                        .text_color(theme.foreground)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(label),
                );
            }

            if self.show_value {
                let display = if let Some(suffix) = &self.suffix {
                    format!("{:.0}{}", self.value, suffix)
                } else {
                    format!("{:.1}", self.value)
                };
                label_row =
                    label_row.child(div().text_color(theme.muted_foreground).child(display));
            }

            container = container.child(label_row);
        }

        // Track
        let mut track = div().relative().w_full().h(px(track_h)).bg(track_bg).rounded(Radius::FULL);

        if self.disabled {
            track = track.opacity(0.5);
        }

        // Fill
        let fill_width = percent * 100.0;
        let fill = div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .w(gpui::relative(fill_width / 100.0))
            .bg(fill_bg)
            .rounded(Radius::FULL);

        track = track.child(fill);

        // Thumb
        let thumb = div()
            .absolute()
            .top(px(-(thumb_size - track_h) / 2.0))
            .left(gpui::relative(percent))
            .w(px(thumb_size))
            .h(px(thumb_size))
            .rounded(Radius::FULL)
            .bg(thumb_bg)
            .border_2()
            .border_color(thumb_border)
            .shadow_sm();

        track = track.child(thumb);

        container = container.child(track);

        container
    }
}

// ─── DataGrid ───────────────────────────────────────────────────────────────
// A richer data grid with column definitions, typed columns, and row styling.
//
// Usage:
//   DataGrid::new()
//       .column(GridColumn::new("Name").width(200.0))
//       .column(GridColumn::new("Size").width(80.0).align_right())
//       .row(vec!["main.rs".into(), "2.4 KB".into()])
//       .row(vec!["lib.rs".into(), "1.1 KB".into()])
//       .striped(true)
//       .render(&theme)

pub struct GridColumn {
    label: String,
    width: Option<f32>,
    flex: f32,
    align_right: bool,
    sortable: bool,
    sort_direction: Option<SortDirection>,
}

#[derive(Clone, Copy)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[allow(dead_code)]
impl GridColumn {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            width: None,
            flex: 1.0,
            align_right: false,
            sortable: false,
            sort_direction: None,
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }

    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }

    pub fn align_right(mut self) -> Self {
        self.align_right = true;
        self
    }

    pub fn sortable(mut self, v: bool) -> Self {
        self.sortable = v;
        self
    }

    pub fn sort_direction(mut self, d: SortDirection) -> Self {
        self.sort_direction = Some(d);
        self
    }
}

pub struct DataGrid {
    columns: Vec<GridColumn>,
    rows: Vec<Vec<String>>,
    striped: bool,
    bordered: bool,
    compact: bool,
    selected_row: Option<usize>,
}

#[allow(dead_code)]
impl DataGrid {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            striped: false,
            bordered: false,
            compact: false,
            selected_row: None,
        }
    }

    pub fn column(mut self, col: GridColumn) -> Self {
        self.columns.push(col);
        self
    }

    pub fn columns(mut self, cols: Vec<GridColumn>) -> Self {
        self.columns = cols;
        self
    }

    pub fn row(mut self, cells: Vec<String>) -> Self {
        self.rows.push(cells);
        self
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn striped(mut self, v: bool) -> Self {
        self.striped = v;
        self
    }

    pub fn bordered(mut self, v: bool) -> Self {
        self.bordered = v;
        self
    }

    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }

    pub fn selected_row(mut self, row: usize) -> Self {
        self.selected_row = Some(row);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let row_height = if self.compact { 28.0 } else { 36.0 };
        let px_x = if self.compact { 8.0 } else { 12.0 };

        let mut grid = div().flex().flex_col().w_full().overflow_hidden();

        if self.bordered {
            grid = grid.border_1().border_color(theme.border).rounded(Radius::MD);
        }

        // Header
        let mut header = div()
            .flex()
            .items_center()
            .w_full()
            .h(px(row_height))
            .px(px(px_x))
            .bg(theme.muted)
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground);

        for col in &self.columns {
            let mut cell = div().px(px(4.0));
            if let Some(w) = col.width {
                cell = cell.w(px(w)).flex_shrink_0();
            } else {
                cell = cell.flex_1();
            }
            if col.align_right {
                cell = cell.text_right();
            }

            let mut label_content = col.label.clone();
            if let Some(dir) = &col.sort_direction {
                match dir {
                    SortDirection::Ascending => label_content.push_str(" ↑"),
                    SortDirection::Descending => label_content.push_str(" ↓"),
                }
            }

            cell = cell.child(label_content);

            if col.sortable {
                cell = cell.cursor_pointer();
            }

            header = header.child(cell);
        }

        grid = grid.child(header);

        // Rows
        let hover_bg = theme.accent;
        for (i, row_data) in self.rows.into_iter().enumerate() {
            let is_selected = self.selected_row == Some(i);
            let bg = if is_selected {
                theme.accent
            } else if self.striped && i % 2 == 1 {
                theme.muted
            } else {
                gpui::transparent_black()
            };

            let mut row = div()
                .flex()
                .items_center()
                .w_full()
                .h(px(row_height))
                .px(px(px_x))
                .bg(bg)
                .border_b_1()
                .border_color(theme.border)
                .text_size(px(12.0))
                .text_color(theme.foreground)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg));

            for (j, cell_text) in row_data.into_iter().enumerate() {
                let mut cell =
                    div().px(px(4.0)).overflow_hidden().text_ellipsis().whitespace_nowrap();

                if let Some(col) = self.columns.get(j) {
                    if let Some(w) = col.width {
                        cell = cell.w(px(w)).flex_shrink_0();
                    } else {
                        cell = cell.flex_1();
                    }
                    if col.align_right {
                        cell = cell.text_right();
                    }
                } else {
                    cell = cell.flex_1();
                }

                cell = cell.child(cell_text);
                row = row.child(cell);
            }

            grid = grid.child(row);
        }

        grid
    }
}
