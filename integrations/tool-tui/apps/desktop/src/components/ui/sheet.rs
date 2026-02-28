use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Sheet ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Sheet (slide-over panel).
//
// Usage:
//   Sheet::new("Settings")
//       .side(SheetSide::Right)
//       .child(div().child("Sheet content"))
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SheetSide {
    Top,
    Right,
    Bottom,
    Left,
}

pub struct Sheet {
    title: String,
    description: Option<String>,
    side: SheetSide,
    children: Vec<AnyElement>,
    footer: Option<AnyElement>,
}

impl Sheet {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            side: SheetSide::Right,
            children: Vec::new(),
            footer: None,
        }
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let is_vertical = matches!(self.side, SheetSide::Left | SheetSide::Right);

        // Overlay
        let mut overlay = div().size_full().flex().bg(theme.overlay);

        // Position
        match self.side {
            SheetSide::Right => {
                overlay = overlay.justify_end();
            }
            SheetSide::Left => {
                overlay = overlay.justify_start();
            }
            SheetSide::Top => {
                overlay = overlay.flex_col().items_start();
            }
            SheetSide::Bottom => {
                overlay = overlay.flex_col().items_end().justify_end();
            }
        }

        let mut sheet = div()
            .flex()
            .flex_col()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border);

        if is_vertical {
            sheet = sheet.w(px(380.0)).h_full();
        } else {
            sheet = sheet.w_full().max_h(gpui::relative(0.75));
        }

        // Header
        let mut header = div().flex().flex_col().gap(px(6.0)).p(px(24.0)).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(self.title),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(24.0))
                        .rounded(Radius::SM)
                        .cursor_pointer()
                        .hover(move |style| style.bg(theme.accent))
                        .child(
                            div().text_size(px(14.0)).text_color(theme.muted_foreground).child("×"),
                        ),
                ),
        );

        if let Some(desc) = self.description {
            header = header
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child(desc));
        }

        sheet = sheet.child(header);

        // Content
        if !self.children.is_empty() {
            let mut content = div().flex_1().p(px(24.0)).pt(px(0.0)).overflow_y_hidden();

            for child in self.children {
                content = content.child(child);
            }

            sheet = sheet.child(content);
        }

        // Footer
        if let Some(footer) = self.footer {
            sheet = sheet.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .p(px(24.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(footer),
            );
        }

        overlay = overlay.child(sheet);
        overlay
    }
}

// ─── ScrollArea ─────────────────────────────────────────────────────────────
// A styled scrollable container.

pub struct ScrollArea {
    id: String,
    children: Vec<AnyElement>,
    max_height: Option<gpui::Pixels>,
    horizontal: bool,
}

impl ScrollArea {
    pub fn new() -> Self {
        Self {
            id: "scroll-area".to_string(),
            children: Vec::new(),
            max_height: None,
            horizontal: false,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn max_height(mut self, height: gpui::Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    #[allow(dead_code)]
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut container = div().id(self.id).flex().flex_col().overflow_y_scroll();

        if let Some(mh) = self.max_height {
            container = container.max_h(mh);
        }

        if self.horizontal {
            container = container.overflow_x_scroll();
        }

        for child in self.children {
            container = container.child(child);
        }

        container
    }
}

// ─── Collapsible ────────────────────────────────────────────────────────────
// A collapsible section.

pub struct Collapsible {
    title: String,
    children: Vec<AnyElement>,
    open: bool,
    icon: Option<String>,
}

#[allow(dead_code)]
impl Collapsible {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
            open: false,
            icon: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let chevron = if self.open { "▲" } else { "▼" };
        let hover_bg = theme.accent;

        let mut container = div().flex().flex_col();

        // Trigger
        let mut trigger = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .py(px(8.0))
            .rounded(Radius::DEFAULT)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg));

        let mut left = div().flex().items_center().gap(px(8.0));

        if let Some(icon) = self.icon {
            left = left
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child(icon));
        }

        left = left.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(self.title),
        );

        trigger = trigger
            .child(left)
            .child(div().text_size(px(10.0)).text_color(theme.muted_foreground).child(chevron));

        container = container.child(trigger);

        // Content
        if self.open {
            let mut content = div().flex().flex_col().pl(px(16.0));
            for child in self.children {
                content = content.child(child);
            }
            container = container.child(content);
        }

        container
    }
}
