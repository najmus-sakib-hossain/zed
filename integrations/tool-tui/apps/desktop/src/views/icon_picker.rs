#![allow(dead_code)]

use gpui::{
    div, prelude::*, px, svg, Context, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    SharedString, Window,
};
use std::sync::Arc;

use crate::components::icon_grid::{IconGrid, IconGridItem};
use crate::components::ui::{EmptyState, Stat};
use crate::components::{Badge, Button, ButtonSize, Kbd};
use crate::icons::data::IconSource;
use crate::icons::IconDataLoader;
use crate::theme::{Radius, Spacing, Theme};

/// How many icons to display per page in the grid
const ICONS_PER_PAGE: usize = 50;

/// Maximum total icons to load (to prevent lag)
const MAX_TOTAL_ICONS: usize = 5000;

// ─── Icon Picker View ───────────────────────────────────────────────────────

/// The main icon picker view — equivalent of the www Next.js icon browser,
/// rebuilt with the DX shadcn-ui component library.
pub struct IconPickerView {
    theme: Theme,
    loader: Arc<IconDataLoader>,
    search_query: String,
    search_focus: FocusHandle,
    selected_pack: Option<String>,
    selected_icon: Option<usize>,
    filtered_icons: Vec<usize>,
    pack_names: Vec<String>,
    total_count: usize,
    page_offset: usize,
}

impl IconPickerView {
    pub fn new(theme: Theme, loader: Arc<IconDataLoader>, cx: &mut Context<Self>) -> Self {
        let pack_names = loader.pack_names();
        let total_count = loader.total_icons().min(MAX_TOTAL_ICONS);
        let filtered_icons: Vec<usize> = (0..total_count).collect();
        let search_focus = cx.focus_handle();

        Self {
            theme,
            loader,
            search_query: String::new(),
            search_focus,
            selected_pack: None,
            selected_icon: None,
            filtered_icons,
            pack_names,
            total_count,
            page_offset: 0,
        }
    }

    // ── Filtering ──

    fn update_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let icons = self.loader.icons();

        self.filtered_icons = icons
            .iter()
            .enumerate()
            .filter(|(_, icon)| {
                if let Some(ref pack) = self.selected_pack {
                    if &icon.pack != pack {
                        return false;
                    }
                }
                if !query.is_empty() {
                    let name_lower = icon.name.to_lowercase();
                    let pack_lower = icon.pack.to_lowercase();
                    if !name_lower.contains(&query) && !pack_lower.contains(&query) {
                        return fuzzy_contains(&name_lower, &query);
                    }
                }
                true
            })
            .map(|(idx, _)| idx)
            .collect();

        self.page_offset = 0;
        self.selected_icon = None;
    }

    fn set_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.update_filter();
        cx.notify();
    }

    fn set_selected_pack(&mut self, pack: Option<String>, cx: &mut Context<Self>) {
        self.selected_pack = pack;
        self.update_filter();
        cx.notify();
    }

    fn next_page(&mut self, cx: &mut Context<Self>) {
        if self.page_offset + ICONS_PER_PAGE < self.filtered_icons.len() {
            self.page_offset += ICONS_PER_PAGE;
            cx.notify();
        }
    }

    fn prev_page(&mut self, cx: &mut Context<Self>) {
        if self.page_offset >= ICONS_PER_PAGE {
            self.page_offset -= ICONS_PER_PAGE;
            cx.notify();
        }
    }

    // ── Header ──

    fn render_header(&self, _window: &mut Window) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(Spacing::SIX)
            .py(Spacing::FOUR)
            .border_b_1()
            .border_color(self.theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(self.theme.foreground)
                            .child("DX"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(self.theme.border)
                            .bg(self.theme.background)
                            .cursor_pointer()
                            .hover(|style| style.bg(self.theme.muted))
                            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                                println!("Workspace selector clicked");
                            })
                            .child(self.render_icon("folder"))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(self.theme.foreground)
                                    .child("photobooth"),
                            )
                            .child(self.render_icon("chevron-down")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(self.theme.border)
                            .bg(self.theme.background)
                            .cursor_pointer()
                            .hover(|style| style.bg(self.theme.muted))
                            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                                println!("Open clicked");
                            })
                            .child(self.render_icon("folder"))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(self.theme.foreground)
                                    .child("Open"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(self.theme.border)
                            .bg(self.theme.background)
                            .cursor_pointer()
                            .hover(|style| style.bg(self.theme.muted))
                            .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                                println!("Commit clicked");
                            })
                            .child(self.render_icon("git-commit"))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(self.theme.foreground)
                                    .child("Commit"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(self.theme.muted_foreground)
                            .child("+771 -315"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(self.theme.foreground)
                            .child("New thread"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(self.theme.muted))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        move |_event, _window, _cx| {
                                            println!("Minimize clicked");
                                        },
                                    )
                                    .child(
                                        div()
                                            .text_size(px(16.0))
                                            .text_color(self.theme.foreground)
                                            .child("−"),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(self.theme.muted))
                                    .on_mouse_down(MouseButton::Left, move |_event, window, _cx| {
                                        window.toggle_fullscreen();
                                    })
                                    .child(
                                        div()
                                            .text_size(px(16.0))
                                            .text_color(self.theme.foreground)
                                            .child("□"),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(self.theme.destructive))
                                    .on_mouse_down(MouseButton::Left, move |_event, window, _cx| {
                                        window.remove_window();
                                    })
                                    .child(
                                        div()
                                            .text_size(px(16.0))
                                            .text_color(self.theme.foreground)
                                            .child("✕"),
                                    ),
                            ),
                    ),
            )
    }

    fn render_icon(&self, name: &str) -> impl IntoElement {
        let asset_path = SharedString::from(format!("icons/{}.svg", name));
        svg()
            .path(asset_path)
            .text_color(self.theme.foreground)
            .size(px(18.0))
            .flex_shrink_0()
            .into_any_element()
    }

    // ── Search Bar ──

    fn render_search_bar(&self, cx: &mut Context<Self>, window: &mut Window) -> impl IntoElement {
        let is_focused = self.search_focus.is_focused(window);

        div()
            .flex()
            .items_center()
            .gap(Spacing::TWO)
            .px(Spacing::SIX)
            .py(Spacing::THREE)
            .child(
                div()
                    .id("icon-search-input")
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap(Spacing::TWO)
                    .px(Spacing::FOUR)
                    .py(Spacing::TWO)
                    .rounded(Radius::MD)
                    .bg(self.theme.card)
                    .border_1()
                    .border_color(if is_focused {
                        self.theme.ring
                    } else {
                        self.theme.border
                    })
                    .cursor_text()
                    .track_focus(&self.search_focus)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|view, _event, window, cx| {
                            view.search_focus.focus(window, cx);
                        }),
                    )
                    .on_key_down(cx.listener(|view, event: &KeyDownEvent, _window, cx| {
                        let keystroke = &event.keystroke;
                        if keystroke.key == "backspace" {
                            view.search_query.pop();
                            view.update_filter();
                            cx.notify();
                        } else if keystroke.key.len() == 1
                            && !keystroke.modifiers.control
                            && !keystroke.modifiers.alt
                        {
                            view.search_query.push_str(&keystroke.key);
                            view.update_filter();
                            cx.notify();
                        } else if keystroke.key == "escape" {
                            view.search_query.clear();
                            view.update_filter();
                            cx.notify();
                        }
                    }))
                    // Search icon placeholder
                    .child(
                        div()
                            .text_sm()
                            .text_color(self.theme.muted_foreground)
                            .child("🔍"),
                    )
                    // Input value / placeholder
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .items_center()
                            .gap(Spacing::ONE)
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(if self.search_query.is_empty() {
                                        self.theme.muted_foreground
                                    } else {
                                        self.theme.foreground
                                    })
                                    .child(if self.search_query.is_empty() {
                                        "Type to search icons...".to_string()
                                    } else {
                                        self.search_query.clone()
                                    }),
                            )
                            .when(is_focused, |el| {
                                el.child(div().w(px(2.0)).h(px(16.0)).bg(self.theme.foreground))
                            }),
                    )
                    // Clear button
                    .when(!self.search_query.is_empty(), |el| {
                        el.child(
                            Button::ghost("✕")
                                .size(ButtonSize::Sm)
                                .on_click(cx.listener(|view, _event, _window, cx| {
                                    view.search_query.clear();
                                    view.update_filter();
                                    cx.notify();
                                }))
                                .render(&self.theme),
                        )
                    })
                    // Kbd shortcut hint
                    .child(Kbd::new("Esc").render(&self.theme)),
            )
    }

    // ── Pack Filter Chips ──

    fn render_pack_chips(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row =
            div().flex().flex_wrap().gap(Spacing::TWO).px(Spacing::SIX).py(Spacing::THREE);

        // "All" chip
        let all_active = self.selected_pack.is_none();
        row = row.child(self.render_chip("All", all_active, None, cx));

        // Show top 15 packs
        for pack_name in self.pack_names.iter().take(15) {
            let active = self.selected_pack.as_ref().map(|p| p == pack_name).unwrap_or(false);
            row = row.child(self.render_chip(pack_name, active, Some(pack_name.clone()), cx));
        }

        row
    }

    fn render_chip(
        &self,
        label: &str,
        active: bool,
        pack: Option<String>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (bg, fg) = if active {
            (self.theme.primary, self.theme.primary_foreground)
        } else {
            (self.theme.secondary, self.theme.secondary_foreground)
        };
        let hover_bg = if active {
            self.theme.primary
        } else {
            self.theme.accent
        };

        div()
            .id(SharedString::from(format!("pack-chip-{}", label)))
            .flex()
            .items_center()
            .px(Spacing::THREE)
            .py(Spacing::ONE)
            .rounded(Radius::FULL)
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event, _window, cx| {
                    view.set_selected_pack(pack.clone(), cx);
                }),
            )
            .child(div().text_xs().text_color(fg).child(label.to_string()))
    }

    // ── Icon Grid ──

    fn render_icon_grid(&self) -> impl IntoElement {
        let icons = self.loader.icons();
        let start = self.page_offset;
        let end = (start + ICONS_PER_PAGE).min(self.filtered_icons.len());

        if self.filtered_icons.is_empty() {
            return div().flex_1().child(
                EmptyState::new("No icons found")
                    .icon("🔍")
                    .description("Try a different search or change the pack filter.")
                    .render(&self.theme),
            );
        }

        let visible_icons: Vec<IconGridItem> = self.filtered_icons[start..end]
            .iter()
            .filter_map(|&idx| icons.get(idx))
            .enumerate()
            .map(|(display_idx, icon)| IconGridItem {
                index: display_idx,
                name: icon.name.clone(),
                pack: icon.pack.clone(),
                svg_body: icon.svg_body.clone(),
                width: icon.width,
                height: icon.height,
                selected: self.selected_icon == Some(display_idx),
            })
            .collect();

        div().flex_1().child(IconGrid::new(visible_icons).render(&self.theme))
    }

    // ── Pagination ──

    fn render_pagination(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.filtered_icons.len();
        let page = self.page_offset / ICONS_PER_PAGE + 1;
        let total_pages = total.div_ceil(ICONS_PER_PAGE).max(1);
        let can_prev = self.page_offset > 0;
        let can_next = self.page_offset + ICONS_PER_PAGE < total;

        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(Spacing::FOUR)
            .px(Spacing::SIX)
            .py(Spacing::THREE)
            .border_t_1()
            .border_color(self.theme.border)
            .child({
                let mut btn = Button::outline("← Previous").size(ButtonSize::Sm);
                if !can_prev {
                    btn = btn.disabled(true);
                } else {
                    btn = btn.on_click(cx.listener(|view, _event, _window, cx| {
                        view.prev_page(cx);
                    }));
                }
                btn.render(&self.theme)
            })
            .child(Badge::outline(format!("{} / {}", page, total_pages)).render(&self.theme))
            .child({
                let mut btn = Button::outline("Next →").size(ButtonSize::Sm);
                if !can_next {
                    btn = btn.disabled(true);
                } else {
                    btn = btn.on_click(cx.listener(|view, _event, _window, cx| {
                        view.next_page(cx);
                    }));
                }
                btn.render(&self.theme)
            })
    }

    // ── Source Sidebar (kept for future use) ──

    fn render_sidebar(&self) -> impl IntoElement {
        let theme = &self.theme;

        div()
            .flex()
            .flex_col()
            .w(px(220.0))
            .h_full()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(
                div()
                    .px(Spacing::FOUR)
                    .py(Spacing::FOUR)
                    .border_b_1()
                    .border_color(theme.sidebar_border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.sidebar_foreground)
                            .child("📦 Icon Packs"),
                    ),
            )
            .child(self.render_source_section("www/icons", IconSource::WwwIcons))
            .child(self.render_source_section("www/svgl", IconSource::WwwSvgl))
            .child(self.render_source_section("crate/data", IconSource::CrateData))
            .child(
                div()
                    .mt_auto()
                    .px(Spacing::FOUR)
                    .py(Spacing::THREE)
                    .border_t_1()
                    .border_color(theme.sidebar_border)
                    .child(Stat::new("Total", format!("{}", self.total_count)).render(theme))
                    .child(Stat::new("Packs", format!("{}", self.pack_names.len())).render(theme)),
            )
    }

    fn render_source_section(&self, label: &str, source: IconSource) -> impl IntoElement {
        let packs_for_source: Vec<&str> = self
            .loader
            .packs()
            .iter()
            .filter(|p| p.source == source)
            .map(|p| p.prefix.as_str())
            .collect();
        let count = packs_for_source.len();

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(Spacing::FOUR)
            .py(Spacing::TWO)
            .border_b_1()
            .border_color(self.theme.sidebar_border)
            .child(div().text_xs().text_color(self.theme.muted_foreground).child(label.to_string()))
            .child(Badge::secondary(format!("{}", count)).render(&self.theme))
    }

    // ── Main Content ──

    fn render_main_content(&self, cx: &mut Context<Self>, window: &mut Window) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .bg(self.theme.background)
            .child(self.render_header(window))
            .child(self.render_search_bar(cx, window))
            .child(
                div()
                    .border_b_1()
                    .border_color(self.theme.border)
                    .child(self.render_pack_chips(cx)),
            )
            .child(self.render_icon_grid())
            .child(self.render_pagination(cx))
    }
}

impl Render for IconPickerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(self.theme.background)
            .on_mouse_move(cx.listener(|_view, _event, _window, cx| {
                cx.notify();
            }))
            .child(self.render_main_content(cx, window))
    }
}

// ── Helpers ──

/// Simple fuzzy match: check if all chars of query appear in order in target
fn fuzzy_contains(target: &str, query: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}
