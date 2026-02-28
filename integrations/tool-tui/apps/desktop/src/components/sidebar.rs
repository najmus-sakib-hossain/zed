use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Sidebar ────────────────────────────────────────────────────────────────
// A shadcn-ui style Sidebar with header, sections, items, and footer.
//
// Usage:
//   Sidebar::new(theme.clone())
//       .header(SidebarSection::new()
//           .item(SidebarItem::new("📝", "New thread"))
//           .item(SidebarItem::new("⚙️", "Settings")))
//       .section(SidebarSection::new().title("Recent").items(vec![...]))
//       .footer(div().child("footer"))
//       .render()

pub struct Sidebar {
    theme: Theme,
    header_items: Vec<SidebarItem>,
    sections: Vec<SidebarSection>,
    footer: Option<AnyElement>,
    width: gpui::Pixels,
    collapsed: bool,
}

impl Sidebar {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            header_items: Vec::new(),
            sections: Vec::new(),
            footer: None,
            width: px(260.0),
            collapsed: false,
        }
    }

    pub fn header_item(mut self, item: SidebarItem) -> Self {
        self.header_items.push(item);
        self
    }

    pub fn section(mut self, section: SidebarSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = width;
        self
    }

    #[allow(dead_code)]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    pub fn render(self) -> impl IntoElement {
        let w = if self.collapsed { px(64.0) } else { self.width };
        let theme = &self.theme;

        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(w)
            .h_full()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .flex_shrink_0();

        // Header items
        if !self.header_items.is_empty() {
            let mut header = div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(10.0))
                .border_b_1()
                .border_color(theme.sidebar_border);

            for item in self.header_items {
                header = header.child(item.render(theme, self.collapsed));
            }

            sidebar = sidebar.child(header);
        }

        // Sections (scrollable)
        let mut content = div().flex().flex_col().flex_1().overflow_y_hidden().py(px(2.0));

        for section in self.sections {
            content = content.child(section.render(theme, self.collapsed));
        }

        sidebar = sidebar.child(content);

        // Footer
        if let Some(footer) = self.footer {
            sidebar = sidebar.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(10.0))
                    .border_t_1()
                    .border_color(theme.sidebar_border)
                    .child(footer),
            );
        }

        sidebar
    }
}

// ─── SidebarItem ────────────────────────────────────────────────────────────

pub struct SidebarItem {
    icon: Option<AnyElement>,
    text: String,
    active: bool,
    badge: Option<String>,
}

impl SidebarItem {
    pub fn new(icon: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            icon: Some(div().child(icon.into()).into_any_element()),
            text: text.into(),
            active: false,
            badge: None,
        }
    }

    pub fn with_icon(icon: impl IntoElement, text: impl Into<String>) -> Self {
        Self {
            icon: Some(icon.into_any_element()),
            text: text.into(),
            active: false,
            badge: None,
        }
    }

    #[allow(dead_code)]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    #[allow(dead_code)]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn render(self, theme: &Theme, collapsed: bool) -> impl IntoElement {
        let bg = if self.active {
            theme.sidebar_accent
        } else {
            gpui::transparent_black()
        };
        let text_color = if self.active {
            theme.sidebar_accent_foreground
        } else {
            theme.sidebar_foreground
        };
        let hover_bg = theme.sidebar_accent;

        let mut item = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(5.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        // Icon
        if let Some(icon) = self.icon {
            item = item.child(div().flex_shrink_0().child(icon));
        }

        if !collapsed {
            item = item
                .child(div().flex_1().text_size(px(14.0)).text_color(text_color).child(self.text));

            if let Some(badge) = self.badge {
                item = item.child(
                    div()
                        .px(px(6.0))
                        .py(px(1.0))
                        .rounded(Radius::FULL)
                        .bg(theme.sidebar_primary)
                        .text_size(px(10.0))
                        .text_color(theme.sidebar_primary_foreground)
                        .child(badge),
                );
            }
        }

        item
    }
}

// ─── SidebarSection ─────────────────────────────────────────────────────────

pub struct SidebarSection {
    title: Option<String>,
    items: Vec<SidebarSectionEntry>,
}

pub enum SidebarSectionEntry {
    Item(SidebarItem),
    Thread(SidebarThread),
}

impl SidebarSection {
    pub fn new() -> Self {
        Self {
            title: None,
            items: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn item(mut self, item: SidebarItem) -> Self {
        self.items.push(SidebarSectionEntry::Item(item));
        self
    }

    pub fn thread(mut self, thread: SidebarThread) -> Self {
        self.items.push(SidebarSectionEntry::Thread(thread));
        self
    }

    /// Backwards-compatible: add multiple threads at once
    pub fn threads(mut self, threads: Vec<(&str, &str)>) -> Self {
        for (text, time) in threads {
            self.items.push(SidebarSectionEntry::Thread(SidebarThread::new(text, time)));
        }
        self
    }

    fn render(self, theme: &Theme, collapsed: bool) -> impl IntoElement {
        let mut section = div().flex().flex_col().py(px(3.0));

        // Section title
        if let Some(title) = &self.title {
            if !collapsed {
                section = section.child(
                    div().flex().items_center().justify_between().px(px(10.0)).py(px(3.0)).child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(title.clone()),
                    ),
                );
            }
        }

        // Items
        let mut items_container = div().flex().flex_col().gap(px(1.0)).px(px(6.0));

        for entry in self.items {
            match entry {
                SidebarSectionEntry::Item(item) => {
                    items_container = items_container.child(item.render(theme, collapsed));
                }
                SidebarSectionEntry::Thread(thread) => {
                    if !collapsed {
                        items_container = items_container.child(thread.render(theme));
                    }
                }
            }
        }

        section = section.child(items_container);
        section
    }
}

// ─── SidebarThread ──────────────────────────────────────────────────────────

pub struct SidebarThread {
    text: String,
    time: String,
}

impl SidebarThread {
    pub fn new(text: impl Into<String>, time: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            time: time.into(),
        }
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_bg = theme.sidebar_accent;
        let has_time = !self.time.trim().is_empty();

        let base = div()
            .flex()
            .items_center()
            .px(px(8.0))
            .py(px(5.0))
            .rounded(Radius::DEFAULT)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .flex_1()
                    .text_size(px(13.0))
                    .text_color(theme.sidebar_foreground)
                    .overflow_x_hidden()
                    .child(self.text),
            );

        if has_time {
            base.child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .flex_shrink_0()
                    .ml(px(8.0))
                    .child(self.time),
            )
            .into_any_element()
        } else {
            base.into_any_element()
        }
    }
}
