use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

// ─── Tabs ───────────────────────────────────────────────────────────────────
// A shadcn-ui style Tabs component.
//
// Usage:
//   Tabs::new("settings")
//       .tab("account", "Account", account_content)
//       .tab("password", "Password", password_content)
//       .active("account")
//       .render(&theme)

pub struct Tabs {
    #[allow(dead_code)]
    id: String,
    tabs: Vec<TabItem>,
    active_value: Option<String>,
    orientation: TabOrientation,
}

struct TabItem {
    value: String,
    label: String,
    content: AnyElement,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabOrientation {
    Horizontal,
    #[allow(dead_code)]
    Vertical,
}

impl Tabs {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            active_value: None,
            orientation: TabOrientation::Horizontal,
        }
    }

    pub fn tab(
        mut self,
        value: impl Into<String>,
        label: impl Into<String>,
        content: impl IntoElement,
    ) -> Self {
        self.tabs.push(TabItem {
            value: value.into(),
            label: label.into(),
            content: content.into_any_element(),
        });
        self
    }

    pub fn active(mut self, value: impl Into<String>) -> Self {
        self.active_value = Some(value.into());
        self
    }

    #[allow(dead_code)]
    pub fn orientation(mut self, orientation: TabOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let active =
            self.active_value.clone().or_else(|| self.tabs.first().map(|t| t.value.clone()));

        let mut container = div().flex().flex_col().gap(px(8.0));

        // Tab list
        let mut tab_list = div()
            .flex()
            .items_center()
            .h(px(40.0))
            .p(px(4.0))
            .rounded(Radius::LG)
            .bg(theme.muted);

        for tab in &self.tabs {
            let is_active = active.as_ref().map(|a| a == &tab.value).unwrap_or(false);
            tab_list = tab_list.child(self.render_tab_trigger(&tab.label, is_active, theme));
        }

        container = container.child(tab_list);

        // Active content
        for tab in self.tabs {
            let is_active = active.as_ref().map(|a| a == &tab.value).unwrap_or(false);
            if is_active {
                container = container.child(tab.content);
                break;
            }
        }

        container
    }

    fn render_tab_trigger(&self, label: &str, active: bool, theme: &Theme) -> impl IntoElement {
        let bg = if active {
            theme.background
        } else {
            gpui::transparent_black()
        };
        let text_color = if active {
            theme.foreground
        } else {
            theme.muted_foreground
        };

        div()
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .h_full()
            .px(px(12.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .text_size(px(14.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(text_color)
            .cursor_pointer()
            .when(!active, |this| this.hover(move |style| style.text_color(theme.foreground)))
            .child(label.to_string())
    }
}

// ─── Accordion ──────────────────────────────────────────────────────────────
// A shadcn-ui style Accordion.
//
// Usage:
//   Accordion::new()
//       .item("item-1", "Is it accessible?", "Yes. It has proper ARIA attributes.")
//       .item("item-2", "Is it styled?", "Yes. It follows shadcn-ui design.")
//       .open("item-1")
//       .render(&theme)

pub struct Accordion {
    items: Vec<AccordionItem>,
    open_items: Vec<String>,
}

struct AccordionItem {
    value: String,
    trigger: String,
    content: String,
}

impl Accordion {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            open_items: Vec::new(),
        }
    }

    pub fn item(
        mut self,
        value: impl Into<String>,
        trigger: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.items.push(AccordionItem {
            value: value.into(),
            trigger: trigger.into(),
            content: content.into(),
        });
        self
    }

    pub fn open(mut self, value: impl Into<String>) -> Self {
        self.open_items.push(value.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().flex_col().w_full();
        let open_items = self.open_items.clone();

        for item in self.items {
            let is_open = open_items.contains(&item.value);
            container = container.child(Self::render_accordion_item_static(&item, is_open, theme));
        }

        container
    }

    fn render_accordion_item_static(
        item: &AccordionItem,
        is_open: bool,
        theme: &Theme,
    ) -> impl IntoElement {
        let chevron = if is_open { "▲" } else { "▼" };

        let mut el = div().flex().flex_col().border_b_1().border_color(theme.border);

        // Trigger
        el = el.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .py(px(16.0))
                .cursor_pointer()
                .hover(move |style| style.opacity(0.8))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(item.trigger.clone()),
                )
                .child(div().text_size(px(10.0)).text_color(theme.muted_foreground).child(chevron)),
        );

        // Content (only when open)
        if is_open {
            el = el.child(
                div()
                    .pb(px(16.0))
                    .text_size(px(14.0))
                    .text_color(theme.muted_foreground)
                    .child(item.content.clone()),
            );
        }

        el
    }
}
