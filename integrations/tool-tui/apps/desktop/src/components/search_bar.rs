use crate::components::label::Kbd;
use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── SearchBar ──────────────────────────────────────────────────────────────
// A shadcn-ui style SearchBar with pack chips.
//
// Usage:
//   SearchBar::new("query", None, vec!["lucide", "heroicons"])
//       .render(&theme)

pub struct SearchBar {
    query: String,
    selected_pack: Option<String>,
    pack_names: Vec<String>,
}

impl SearchBar {
    pub fn new(
        query: impl Into<String>,
        selected_pack: Option<String>,
        pack_names: Vec<String>,
    ) -> Self {
        Self {
            query: query.into(),
            selected_pack,
            pack_names,
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .child(self.render_search_input(theme))
            .child(self.render_pack_filters(theme))
    }

    fn render_search_input(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .flex_1()
            .items_center()
            .gap(px(8.0))
            .px(px(24.0))
            .py(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(Radius::MD)
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.border)
                    .hover(move |style| style.border_color(theme.ring))
                    // Search icon
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.muted_foreground)
                            .child("🔍"),
                    )
                    // Text
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(14.0))
                            .text_color(if self.query.is_empty() {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            })
                            .child(if self.query.is_empty() {
                                "Type to search icons...".to_string()
                            } else {
                                format!("Searching: {}", self.query)
                            }),
                    )
                    // Kbd shortcut
                    .child(Kbd::new("⌘K").render(theme)),
            )
    }

    fn render_pack_filters(self, theme: &Theme) -> impl IntoElement {
        let mut chips = div().flex().flex_wrap().gap(px(6.0)).px(px(24.0)).pb(px(12.0));

        let all_active = self.selected_pack.is_none();
        chips = chips.child(PackChip::new("All", all_active).render(theme));

        for pack_name in self.pack_names.iter().take(15) {
            let active = self.selected_pack.as_ref().map(|p| p == pack_name).unwrap_or(false);
            chips = chips.child(PackChip::new(pack_name, active).render(theme));
        }

        chips
    }
}

// ─── PackChip ───────────────────────────────────────────────────────────────

struct PackChip {
    label: String,
    active: bool,
}

impl PackChip {
    fn new(label: impl Into<String>, active: bool) -> Self {
        Self {
            label: label.into(),
            active,
        }
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.active {
            theme.primary
        } else {
            theme.secondary
        };
        let fg = if self.active {
            theme.primary_foreground
        } else {
            theme.secondary_foreground
        };
        let hover_bg = if self.active {
            theme.primary
        } else {
            theme.accent
        };

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(10.0))
            .py(px(4.0))
            .rounded(Radius::FULL)
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(fg)
                    .child(self.label),
            )
    }
}
