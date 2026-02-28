use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Card ───────────────────────────────────────────────────────────────────
// A shadcn-ui style Card with header, content, and footer sections.
//
// Usage:
//   Card::new()
//       .header(CardHeader::new("Title").description("Description"))
//       .child(div().child("Content here"))
//       .footer(div().child("Footer"))
//       .render(&theme)

pub struct Card {
    header: Option<AnyElement>,
    children: Vec<AnyElement>,
    footer: Option<AnyElement>,
    hoverable: bool,
    clickable: bool,
}

impl Card {
    pub fn new() -> Self {
        Self {
            header: None,
            children: Vec::new(),
            footer: None,
            hoverable: false,
            clickable: false,
        }
    }

    /// Quick card with icon and text (backwards compatible)
    pub fn simple(icon: impl Into<String>, text: impl Into<String>) -> Self {
        let icon_str = icon.into();
        let text_str = text.into();
        Self::new().hoverable(true).clickable(true).with_child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(div().text_2xl().child(icon_str))
                .child(div().text_sm().child(text_str)),
        )
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    pub fn with_child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }

    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_bg = theme.accent;
        let mut card = div()
            .flex()
            .flex_col()
            .rounded(Radius::LG)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        if self.hoverable {
            card = card.hover(move |style| style.bg(hover_bg));
        }

        if self.clickable {
            card = card.cursor_pointer();
        }

        // Header
        let has_header = self.header.is_some();
        if let Some(header) = self.header {
            card = card.child(div().flex().flex_col().p(px(24.0)).pb(px(0.0)).child(header));
        }

        // Content
        if !self.children.is_empty() {
            let mut content = div().p(px(24.0)).pt(px(0.0));
            // Add top padding if there's a header
            if !has_header {
                content = content.pt(px(24.0));
            } else {
                content = content.pt(px(8.0));
            }
            for child in self.children {
                content = content.child(child);
            }
            card = card.child(content);
        }

        // Footer
        if let Some(footer) = self.footer {
            card = card.child(div().flex().items_center().p(px(24.0)).pt(px(0.0)).child(footer));
        }

        card
    }
}

// ─── CardHeader ─────────────────────────────────────────────────────────────

pub struct CardHeader {
    title: String,
    description: Option<String>,
}

impl CardHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut header = div().flex().flex_col().gap(px(6.0)).child(
            div()
                .text_base()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.card_foreground)
                .line_height(px(28.0))
                .child(self.title),
        );

        if let Some(desc) = self.description {
            header = header
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child(desc));
        }

        header
    }
}
