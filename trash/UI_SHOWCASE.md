//! # UI Showcase
//!
//! A shadcn/ui-inspired interactive component showcase window for Zed's UI library.
//! Opens as a standalone GPUI window that displays all components from `crates/ui`
//! with full interactivity and visual examples.

mod sections;

use gpui::{
    App, AppContext as _, Bounds, FocusHandle, Focusable, Window,
    WindowBounds, WindowOptions, prelude::*, px, size,
};
use theme::ActiveTheme;
use ui::{
    ButtonStyle, Color, DynamicSpacing, IconName, IconSize, Label, LabelSize, ListItem,
    ListSubHeader, h_flex, prelude::*, v_flex,
};

use sections::Section;

// ── Public surface ─────────────────────────────────────────────────────────────

/// Open the UI showcase window. Call this from `zed`'s `main.rs`.
pub fn open_showcase_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1400.), px(920.)), cx);
    let _ = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Zed UI Component Showcase".into()),
                appears_transparent: false,
                ..Default::default()
            }),
            ..Default::default()
        },
        |window, cx| {
            theme::setup_ui_font(window, cx);
            cx.new(|cx| ShowcaseWindow::new(window, cx))
        },
    );
}

// ── State ──────────────────────────────────────────────────────────────────────

/// Root interactive state for the showcase window.
pub struct ShowcaseWindow {
    focus_handle: FocusHandle,
    active_section: Section,

    // ─ Buttons ─
    click_count: usize,

    // ─ Formatting toggles ─
    toggle_bold: bool,
    toggle_italic: bool,
    toggle_underline: bool,

    // ─ Alignment selector (exclusive) ─
    alignment: usize,

    // ─ Disclosure accordion ─
    accordion_open: [bool; 3],

    // ─ Progress ─
    progress_value: f32,

    // ─ Tabs ─
    active_tab: usize,

    // ─ Tag chips (multi-select) ─
    tag_selected: [bool; 6],

    // ─ Indicator ─
    indicator_online: bool,
}

impl ShowcaseWindow {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_section: Section::Introduction,
            click_count: 0,
            toggle_bold: false,
            toggle_italic: true,
            toggle_underline: false,
            alignment: 0,
            accordion_open: [false, false, false],
            progress_value: 0.4,
            active_tab: 0,
            tag_selected: [true, false, true, false, false, true],
            indicator_online: true,
        }
    }
}

impl Focusable for ShowcaseWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ── Navigation ─────────────────────────────────────────────────────────────────

const NAV_GROUPS: &[(&str, &[(&str, Section)])] = &[
    (
        "Getting Started",
        &[("Introduction", Section::Introduction)],
    ),
    (
        "Display",
        &[
            ("Avatar", Section::Avatar),
            ("Badge / Chip", Section::Chip),
            ("Diff Stat", Section::DiffStat),
            ("Divider", Section::Divider),
            ("Facepile", Section::Facepile),
            ("Icon", Section::Icon),
            ("Indicator", Section::Indicator),
            ("Label", Section::Label),
        ],
    ),
    (
        "Inputs",
        &[
            ("Button", Section::Button),
            ("Icon Button", Section::IconButton),
            ("Toggle", Section::Toggle),
            ("Disclosure", Section::Disclosure),
        ],
    ),
    (
        "Feedback",
        &[
            ("Callout", Section::Callout),
            ("Banner", Section::Banner),
            ("Progress", Section::Progress),
        ],
    ),
    (
        "Navigation",
        &[
            ("Keybinding", Section::Keybinding),
            ("Tab Bar", Section::TabBar),
        ],
    ),
    ("Data", &[("List", Section::List)]),
];

// ── Render ─────────────────────────────────────────────────────────────────────

impl Render for ShowcaseWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_section;

        h_flex()
            .id("showcase-root")
            .key_context("ShowcaseWindow")
            .size_full()
            .track_focus(&self.focus_handle)
            .bg(cx.theme().colors().editor_background)
            // ── Sidebar ──────────────────────────────────────────────────────
            .child(self.render_sidebar(active, cx))
            // ── Content ──────────────────────────────────────────────────────
            .child(self.render_content(cx))
    }
}

impl ShowcaseWindow {
    // ── Sidebar ────────────────────────────────────────────────────────────────

    fn render_sidebar(&self, active: Section, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("sidebar")
            .h_full()
            .w(px(240.))
            .flex_none()
            .border_r_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().surface_background)
            // title strip
            .child(
                h_flex()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new("UI Showcase")
                            .size(LabelSize::Default)
                            .weight(gpui::FontWeight::BOLD)
                            .color(Color::Default),
                    ),
            )
            // nav
            .child(
                div()
                    .id("sidebar-scroll")
                    .overflow_y_scroll()
                    .flex_1()
                    .p_2()
                    .children(NAV_GROUPS.iter().map(|(group_title, items)| {
                        v_flex()
                            .mb_1()
                            .child(
                                ListSubHeader::new(*group_title)
                                    .inset(true)
                                    .into_any_element(),
                            )
                            .children(items.iter().map(|(label, section)| {
                                let section = *section;
                                let selected = active == section;
                                ListItem::new(format!("nav-{label}"))
                                    .child(Label::new(*label))
                                    .selectable(true)
                                    .toggle_state(selected)
                                    .inset(true)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.active_section = section;
                                        cx.notify();
                                    }))
                            }))
                    })),
            )
    }

    // ── Main content ───────────────────────────────────────────────────────────

    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let section = self.active_section;

        v_flex()
            .id("content")
            .flex_1()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .id("content-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px(DynamicSpacing::Base08.rems(cx))
                    .py_6()
                    .child(self.render_section(section, cx)),
            )
    }

    fn render_section(&mut self, section: Section, cx: &mut Context<Self>) -> impl IntoElement {
        match section {
            Section::Introduction => sections::render_introduction(cx).into_any_element(),
            Section::Button => self.render_button_section(cx).into_any_element(),
            Section::IconButton => self.render_icon_button_section(cx).into_any_element(),
            Section::Toggle => self.render_toggle_section(cx).into_any_element(),
            Section::Disclosure => self.render_disclosure_section(cx).into_any_element(),
            Section::Label => sections::render_label_section(cx).into_any_element(),
            Section::Icon => sections::render_icon_section(cx).into_any_element(),
            Section::Chip => self.render_chip_section(cx).into_any_element(),
            Section::Avatar => sections::render_avatar_section(cx).into_any_element(),
            Section::Facepile => sections::render_facepile_section(cx).into_any_element(),
            Section::DiffStat => sections::render_diff_stat_section(cx).into_any_element(),
            Section::Divider => sections::render_divider_section(cx).into_any_element(),
            Section::Indicator => self.render_indicator_section(cx).into_any_element(),
            Section::Callout => sections::render_callout_section(cx).into_any_element(),
            Section::Banner => sections::render_banner_section(cx).into_any_element(),
            Section::Progress => self.render_progress_section(cx).into_any_element(),
            Section::Keybinding => sections::render_keybinding_section(cx).into_any_element(),
            Section::TabBar => self.render_tab_bar_section(cx).into_any_element(),
            Section::List => sections::render_list_section(cx).into_any_element(),
        }
    }

    // ── Button ─────────────────────────────────────────────────────────────────

    fn render_button_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count;

        v_flex()
            .gap_6()
            .child(section_header(
                "Button",
                "Displays a button or a component that looks like a button.",
            ))
            // ── Styles ──
            .child(showcase_card(
                "Variants",
                Some("All available button styles."),
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Button::new("btn-default", "Default")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.click_count += 1;
                                cx.notify();
                            })),
                    )
                    .child(Button::new("btn-filled", "Filled").style(ButtonStyle::Filled))
                    .child(Button::new("btn-subtle", "Subtle").style(ButtonStyle::Subtle))
                    .child(
                        Button::new("btn-tinted", "Accent")
                            .style(ButtonStyle::Tinted(ui::TintColor::Accent)),
                    )
                    .child(
                        Button::new("btn-transparent", "Transparent")
                            .style(ButtonStyle::Transparent),
                    ),
                cx,
            ))
            // ── Tints ──
            .child(showcase_card(
                "Tint Colors",
                Some("Semantic tint variants for contextual meaning."),
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Button::new("tint-accent", "Accent")
                            .style(ButtonStyle::Tinted(ui::TintColor::Accent)),
                    )
                    .child(
                        Button::new("tint-error", "Error")
                            .style(ButtonStyle::Tinted(ui::TintColor::Error)),
                    )
                    .child(
                        Button::new("tint-warning", "Warning")
                            .style(ButtonStyle::Tinted(ui::TintColor::Warning)),
                    )
                    .child(
                        Button::new("tint-success", "Success")
                            .style(ButtonStyle::Tinted(ui::TintColor::Success)),
                    ),
                cx,
            ))
            // ── States ──
            .child(showcase_card(
                "States",
                Some("Interactive states including disabled and selected."),
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(Button::new("state-normal", "Normal"))
                    .child(Button::new("state-disabled", "Disabled").disabled(true))
                    .child(
                        Button::new("state-selected", "Selected")
                            .toggle_state(true)
                            .selected_style(ButtonStyle::Tinted(ui::TintColor::Accent)),
                    ),
                cx,
            ))
            // ── With Icons ──
            .child(showcase_card(
                "With Icons",
                Some("Buttons with leading or trailing icons."),
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Button::new("icon-start", "Icon Start")
                            .icon(IconName::Check)
                            .icon_position(ui::IconPosition::Start),
                    )
                    .child(
                        Button::new("icon-end", "Icon End")
                            .icon(IconName::Check)
                            .icon_position(ui::IconPosition::End),
                    )
                    .child(
                        Button::new("icon-color", "Icon Color")
                            .icon(IconName::Check)
                            .icon_color(Color::Accent),
                    ),
                cx,
            ))
            // ── Sizes ──
            .child(showcase_card(
                "Sizes",
                Some("Available button sizes."),
                h_flex()
                    .gap_3()
                    .items_end()
                    .flex_wrap()
                    .child(
                        Button::new("size-xs", "X-Small")
                            .style(ButtonStyle::Filled)
                            .size(ui::ButtonSize::None),
                    )
                    .child(
                        Button::new("size-sm", "Small")
                            .style(ButtonStyle::Filled)
                            .size(ui::ButtonSize::Compact),
                    )
                    .child(
                        Button::new("size-default", "Default")
                            .style(ButtonStyle::Filled)
                            .size(ui::ButtonSize::Default),
                    )
                    .child(
                        Button::new("size-large", "Large")
                            .style(ButtonStyle::Filled)
                            .size(ui::ButtonSize::Large),
                    ),
                cx,
            ))
            // ── Interactive counter ──
            .child(interactive_card(
                "Interactive Demo",
                Some("Click the button to increment the counter."),
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Button::new("counter-btn", "Click me!")
                            .style(ButtonStyle::Filled)
                            .icon(IconName::Plus)
                            .icon_position(ui::IconPosition::Start)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.click_count += 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("reset-btn", "Reset")
                            .style(ButtonStyle::Subtle)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.click_count = 0;
                                cx.notify();
                            })),
                    )
                    .child(
                        Label::new(format!("Clicked {} time(s)", click_count))
                            .size(LabelSize::Default)
                            .color(Color::Muted),
                    ),
                cx,
            ))
    }

    // ── Icon Button ────────────────────────────────────────────────────────────

    fn render_icon_button_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .child(section_header(
                "Icon Button",
                "A stripped-down button showing only an icon, commonly used in toolbars.",
            ))
            .child(showcase_card(
                "Variants",
                Some("Icon buttons in different styles."),
                h_flex()
                    .gap_3()
                    .items_center()
                    .flex_wrap()
                    .child(
                        ui::IconButton::new("ib-default", IconName::Star)
                            .icon_size(IconSize::Medium),
                    )
                    .child(
                        ui::IconButton::new("ib-small", IconName::StarFilled)
                            .icon_size(IconSize::Small),
                    )
                    .child(
                        ui::IconButton::new("ib-xsmall", IconName::MagnifyingGlass)
                            .icon_size(IconSize::XSmall),
                    )
                    .child(
                        ui::IconButton::new("ib-accent", IconName::Settings)
                            .icon_color(Color::Accent),
                    )
                    .child(
                        ui::IconButton::new("ib-error", IconName::XCircle)
                            .icon_color(Color::Error),
                    )
                    .child(
                        ui::IconButton::new("ib-selected", IconName::Star)
                            .toggle_state(true),
                    )
                    .child(
                        ui::IconButton::new("ib-disabled", IconName::Eye)
                            .disabled(true),
                    ),
                cx,
            ))
            .child(showcase_card(
                "Common Use Cases",
                Some("Typical scenarios where icon buttons appear."),
                h_flex()
                    .gap_1()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .bg(cx.theme().colors().surface_background)
                    .child(ui::IconButton::new("toolbar-copy", IconName::Copy))
                    .child(ui::IconButton::new("toolbar-paste", IconName::Copy))
                    .child(ui::IconButton::new("toolbar-undo", IconName::Undo))
                    .child(ui::IconButton::new("toolbar-redo", IconName::RotateCw))
                    .child(
                        div()
                            .w_px()
                            .h_5()
                            .mx_1()
                            .bg(cx.theme().colors().border),
                    )
                    .child(ui::IconButton::new("toolbar-settings", IconName::Settings))
                    .child(ui::IconButton::new("toolbar-close", IconName::Close)),
                cx,
            ))
    }

    // ── Toggle ─────────────────────────────────────────────────────────────────

    fn render_toggle_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let bold = self.toggle_bold;
        let italic = self.toggle_italic;
        let underline = self.toggle_underline;
        let alignment = self.alignment;

        v_flex()
            .gap_6()
            .child(section_header(
                "Toggle",
                "A two-state button that presses to toggle on or off.",
            ))
            .child(interactive_card(
                "Formatting Toggles",
                Some("Click each icon button to toggle formatting on or off."),
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        ui::IconButton::new("toggle-bold", IconName::FontWeight)
                            .toggle_state(bold)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_bold = !this.toggle_bold;
                                cx.notify();
                            })),
                    )
                    .child(Label::new("Bold").color(Color::Muted))
                    .child(
                        ui::IconButton::new("toggle-italic", IconName::FontSize)
                            .toggle_state(italic)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_italic = !this.toggle_italic;
                                cx.notify();
                            })),
                    )
                    .child(Label::new("Italic").color(Color::Muted))
                    .child(
                        ui::IconButton::new("toggle-underline", IconName::Slash)
                            .toggle_state(underline)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_underline = !this.toggle_underline;
                                cx.notify();
                            })),
                    )
                    .child(Label::new("Underline").color(Color::Muted)),
                cx,
            ))
            .child(interactive_card(
                "Alignment Selector",
                Some("Click a button to select exclusive alignment."),
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("tg-left", "Left")
                            .style(if alignment == 0 { ButtonStyle::Filled } else { ButtonStyle::Subtle })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.alignment = 0;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("tg-center", "Center")
                            .style(if alignment == 1 { ButtonStyle::Filled } else { ButtonStyle::Subtle })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.alignment = 1;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("tg-right", "Right")
                            .style(if alignment == 2 { ButtonStyle::Filled } else { ButtonStyle::Subtle })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.alignment = 2;
                                cx.notify();
                            })),
                    ),
                cx,
            ))
    }

    // ── Disclosure ─────────────────────────────────────────────────────────────

    fn render_disclosure_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let open_states = self.accordion_open;
        let titles = [
            "Is it accessible?",
            "Is it styled?",
            "Is it animated?",
        ];
        let bodies = [
            "Yes. It adheres to the WAI-ARIA design pattern.",
            "Yes. It comes with default styles that match the other components.",
            "Yes. It's animated by default, but you can disable it if needed.",
        ];

        v_flex()
            .gap_6()
            .child(section_header(
                "Disclosure",
                "An expandable/collapsible container to reveal or hide content.",
            ))
            .child(interactive_card(
                "Accordion",
                Some("Click a header to expand or collapse it."),
                v_flex()
                    .gap_2()
                    .w_full()
                    .children((0..3usize).map(|i| {
                        let is_open = open_states[i];
                        v_flex()
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .rounded_md()
                            .overflow_hidden()
                            .child(
                                h_flex()
                                    .px_4()
                                    .py_3()
                                    .justify_between()
                                    .items_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(cx.theme().colors().element_hover))
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.accordion_open[i] = !this.accordion_open[i];
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        Label::new(titles[i])
                                            .weight(gpui::FontWeight::MEDIUM),
                                    )
                                    .child(
                                        ui::Icon::new(if is_open {
                                            IconName::ChevronUp
                                        } else {
                                            IconName::ChevronDown
                                        })
                                        .size(IconSize::Small)
                                        .color(Color::Muted),
                                    ),
                            )
                            .when(is_open, |this| {
                                this.child(
                                    div()
                                        .px_4()
                                        .py_3()
                                        .border_t_1()
                                        .border_color(cx.theme().colors().border)
                                        .child(Label::new(bodies[i]).color(Color::Muted)),
                                )
                            })
                    })),
                cx,
            ))
    }

    // ── Chip ───────────────────────────────────────────────────────────────────

    fn render_chip_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.tag_selected;

        let tags = ["Rust", "GPUI", "Zed", "UI", "Showcase", "Interactive"];

        v_flex()
            .gap_6()
            .child(section_header(
                "Badge / Chip",
                "Small status and metadata labels.",
            ))
            .child(showcase_card(
                "Color Variants",
                Some("Chips with semantic color meanings."),
                h_flex()
                    .gap_3()
                    .flex_wrap()
                    .child(ui::Chip::new("Default"))
                    .child(ui::Chip::new("Accent").label_color(Color::Accent))
                    .child(ui::Chip::new("Error").label_color(Color::Error))
                    .child(ui::Chip::new("Warning").label_color(Color::Warning))
                    .child(ui::Chip::new("Success").label_color(Color::Success))
                    .child(ui::Chip::new("Muted").label_color(Color::Muted)),
                cx,
            ))
            .child(interactive_card(
                "Interactive Tags",
                Some("Click to toggle selection."),
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .children(tags.iter().enumerate().map(|(i, tag)| {
                        let is_selected = selected[i];
                        div()
                            .id(format!("tag-{i}"))
                            .cursor_pointer()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.tag_selected[i] = !this.tag_selected[i];
                                    cx.notify();
                                }),
                            )
                            .child(
                                ui::Chip::new(*tag).label_color(
                                    if is_selected { Color::Accent } else { Color::Muted },
                                ),
                            )
                    })),
                cx,
            ))
    }

    // ── Indicator ──────────────────────────────────────────────────────────────

    fn render_indicator_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.indicator_online;

        v_flex()
            .gap_6()
            .child(section_header(
                "Indicator",
                "Small dot indicators for showing status or presence.",
            ))
            .child(showcase_card(
                "Color States",
                Some("Semantic color variants."),
                h_flex()
                    .gap_6()
                    .items_center()
                    .flex_wrap()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(ui::Indicator::dot().color(Color::Accent))
                            .child(Label::new("Active").color(Color::Muted)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(ui::Indicator::dot().color(Color::Error))
                            .child(Label::new("Error").color(Color::Muted)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(ui::Indicator::dot().color(Color::Warning))
                            .child(Label::new("Warning").color(Color::Muted)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(ui::Indicator::dot().color(Color::Success))
                            .child(Label::new("Success").color(Color::Muted)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(ui::Indicator::dot().color(Color::Muted))
                            .child(Label::new("Offline").color(Color::Muted)),
                    ),
                cx,
            ))
            .child(interactive_card(
                "Toggle Indicator",
                Some("Toggle the presence indicator on and off."),
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .relative()
                            .size_10()
                            .rounded_full()
                            .bg(cx.theme().colors().element_background)
                            .border_1()
                            .border_color(cx.theme().colors().border)
                            .when(active, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .bottom_0()
                                        .right_0()
                                        .child(ui::Indicator::dot().color(Color::Success)),
                                )
                            }),
                    )
                    .child(
                        Button::new("toggle-indicator", if active { "Online" } else { "Offline" })
                            .style(if active {
                                ButtonStyle::Tinted(ui::TintColor::Success)
                            } else {
                                ButtonStyle::Subtle
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.indicator_online = !this.indicator_online;
                                cx.notify();
                            })),
                    ),
                cx,
            ))
    }

    // ── Progress ───────────────────────────────────────────────────────────────

    fn render_progress_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.progress_value;

        v_flex()
            .gap_6()
            .child(section_header(
                "Progress",
                "Displays an indicator showing the completion progress of a task.",
            ))
            .child(showcase_card(
                "Progress Bar",
                Some("Linear progress bar at various completion levels."),
                v_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("25%").color(Color::Muted).size(LabelSize::Small))
                            .child(ui::ProgressBar::new("pb-25", 0.25, 1.0, cx)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("50%").color(Color::Muted).size(LabelSize::Small))
                            .child(ui::ProgressBar::new("pb-50", 0.5, 1.0, cx)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("75%").color(Color::Muted).size(LabelSize::Small))
                            .child(ui::ProgressBar::new("pb-75", 0.75, 1.0, cx)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("100%").color(Color::Muted).size(LabelSize::Small))
                            .child(ui::ProgressBar::new("pb-100", 1.0, 1.0, cx)),
                    ),
                cx,
            ))
            .child(showcase_card(
                "Circular Progress",
                Some("Circular indicators for bounded tasks."),
                h_flex()
                    .gap_6()
                    .items_center()
                    .child(ui::CircularProgress::new(0.0, 1.0, px(36.), cx))
                    .child(ui::CircularProgress::new(0.25, 1.0, px(36.), cx))
                    .child(ui::CircularProgress::new(0.5, 1.0, px(36.), cx))
                    .child(ui::CircularProgress::new(0.75, 1.0, px(36.), cx))
                    .child(ui::CircularProgress::new(1.0, 1.0, px(36.), cx)),
                cx,
            ))
            .child(interactive_card(
                "Dynamic Progress",
                Some("Adjust the progress value interactively."),
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(ui::CircularProgress::new(value, 1.0, px(48.), cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(ui::ProgressBar::new("pb-dyn", value, 1.0, cx))
                                    .child(
                                        Label::new(format!("{:.0}%", value * 100.0))
                                            .color(Color::Muted)
                                            .size(LabelSize::Small),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("prog-dec", "−10%")
                                    .style(ButtonStyle::Subtle)
                                    .disabled(value <= 0.0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.progress_value =
                                            (this.progress_value - 0.1).clamp(0.0, 1.0);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("prog-inc", "+10%")
                                    .style(ButtonStyle::Subtle)
                                    .disabled(value >= 1.0)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.progress_value =
                                            (this.progress_value + 0.1).clamp(0.0, 1.0);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("prog-reset", "Reset")
                                    .style(ButtonStyle::Transparent)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.progress_value = 0.0;
                                        cx.notify();
                                    })),
                            ),
                    ),
                cx,
            ))
    }

    // ── Tab Bar ────────────────────────────────────────────────────────────────

    fn render_tab_bar_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = self.active_tab;

        let tab_labels = ["Overview", "Settings", "Activity", "Logs"];

        v_flex()
            .gap_6()
            .child(section_header(
                "Tab Bar",
                "A set of layered sections rendered one at a time.",
            ))
            .child(interactive_card(
                "Interactive Tabs",
                Some("Click a tab to switch panels."),
                v_flex()
                    .gap_0()
                    .border_1()
                    .border_color(cx.theme().colors().border)
                    .rounded_md()
                    .overflow_hidden()
                    // Tab headers
                    .child(
                        h_flex()
                            .border_b_1()
                            .border_color(cx.theme().colors().border)
                            .bg(cx.theme().colors().surface_background)
                            .children(tab_labels.iter().enumerate().map(|(i, label)| {
                                let is_active = i == active_tab;
                                div()
                                    .px_4()
                                    .py_2()
                                    .border_b_2()
                                    .border_color(if is_active {
                                        cx.theme().colors().text_accent
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener(move |this, _, _, cx| {
                                            this.active_tab = i;
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        Label::new(*label)
                                            .color(if is_active {
                                                Color::Default
                                            } else {
                                                Color::Muted
                                            })
                                            .size(LabelSize::Small),
                                    )
                            })),
                    )
                    // Tab content
                    .child(
                        div()
                            .p_4()
                            .child(Label::new(format!(
                                "Content for the '{}' tab. Click other tabs to switch panels.",
                                tab_labels[active_tab]
                            )).color(Color::Muted)),
                    ),
                cx,
            ))
    }
}

// ── Helper components ──────────────────────────────────────────────────────────

/// A section header with title and description (shadcn-style).
pub fn section_header(title: impl Into<SharedString>, description: impl Into<SharedString>) -> impl IntoElement {
    let title: SharedString = title.into();
    let description: SharedString = description.into();
    v_flex()
        .gap_1()
        .pb_2()
        .mb_2()
        .border_b_1()
        .border_color(gpui::transparent_black())
        .child(
            div()
                .text_3xl()
                .font_weight(gpui::FontWeight::BOLD)
                .child(title),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui::Hsla::from(gpui::Rgba {
                    r: 0.6,
                    g: 0.6,
                    b: 0.6,
                    a: 1.0,
                }))
                .child(description),
        )
}

/// A card with a title and example content (shadcn-style).
pub fn showcase_card(
    title: impl Into<SharedString>,
    description: Option<impl Into<SharedString>>,
    content: impl IntoElement + 'static,
    cx: &App,
) -> impl IntoElement {
    let title: SharedString = title.into();
    let description: Option<SharedString> = description.map(Into::into);
    v_flex()
        .gap_3()
        .child(
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title),
                )
                .when_some(description, |this, desc| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors().text_muted)
                            .child(desc),
                    )
                }),
        )
        .child(
            div()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().colors().border)
                .p_6()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_center()
                .child(content),
        )
}

/// Identical to `showcase_card` but with a subtle blue tint to signal interactivity.
pub fn interactive_card(
    title: impl Into<SharedString>,
    description: Option<impl Into<SharedString>>,
    content: impl IntoElement + 'static,
    cx: &App,
) -> impl IntoElement {
    let title: SharedString = title.into();
    let description: Option<SharedString> = description.map(Into::into);
    v_flex()
        .gap_3()
        .child(
            v_flex()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .px_1p5()
                                .py_0p5()
                                .rounded_sm()
                                .text_xs()
                                .bg(cx.theme().colors().element_selected)
                                .text_color(cx.theme().colors().text_accent)
                                .child("Interactive"),
                        ),
                )
                .when_some(description, |this, desc| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors().text_muted)
                            .child(desc),
                    )
                }),
        )
        .child(
            div()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().colors().border_focused)
                .p_6()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_center()
                .child(content),
        )
}



































//! Static (non-interactive) section renderers and the Section navigation enum.

use gpui::{App, IntoElement, prelude::*};
use ui::{
    Avatar, Callout, Color, Divider, Facepile, Icon, IconName, IconSize, Label,
    LabelSize, Severity, h_flex, prelude::*, v_flex,
};

use crate::{section_header, showcase_card};

// ── Section enum ───────────────────────────────────────────────────────────────

/// Each variant corresponds to one page in the showcase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Introduction,
    // Display
    Avatar,
    Chip,
    DiffStat,
    Divider,
    Facepile,
    Icon,
    Indicator,
    Label,
    // Inputs
    Button,
    IconButton,
    Toggle,
    Disclosure,
    // Feedback
    Callout,
    Banner,
    Progress,
    // Navigation
    Keybinding,
    TabBar,
    // Data
    List,
}

// ── Introduction ───────────────────────────────────────────────────────────────

pub fn render_introduction(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_8()
        .max_w(gpui::px(720.))
        // ── Hero ─
        .child(
            v_flex()
                .gap_4()
                .child(
                    div()
                        .text_3xl()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child("UI Component Showcase"),
                )
                .child(
                    div()
                        .text_xl()
                        .text_color(cx.theme().colors().text_muted)
                        .child(
                            "Beautifully designed components built on GPUI — \
                             Zed's GPU-accelerated UI framework.",
                        ),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .child(
                            ui::Button::new("intro-browse", "Browse Components")
                                .style(ui::ButtonStyle::Filled)
                                .icon(IconName::ArrowRight)
                                .icon_position(ui::IconPosition::End),
                        )
                        .child(
                            ui::Button::new("intro-github", "GitHub")
                                .style(ui::ButtonStyle::Subtle)
                                .icon(IconName::Github)
                                .icon_position(ui::IconPosition::Start),
                        ),
                ),
        )
        // ── Divider ─
        .child(Divider::horizontal())
        // ── What is GPUI? ─
        .child(
            v_flex()
                .gap_4()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("What is GPUI?"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors().text_muted)
                        .child(
                            "GPUI is a hybrid immediate and retained-mode, GPU-accelerated \
                             UI framework for Rust developed by the Zed team. It powers every \
                             pixel of the Zed code editor and exposes a Tailwind-inspired \
                             styling API that compiles down to Metal / Vulkan draw calls.",
                        ),
                ),
        )
        // ── Cards ─
        .child(
            h_flex()
                .gap_4()
                .flex_wrap()
                .child(feature_card(
                    "GPU Accelerated",
                    "Every element rendered via Metal or Vulkan — smooth 60 fps at any scale.",
                    IconName::ZedXCopilot,
                    cx,
                ))
                .child(feature_card(
                    "Type-safe Styling",
                    "Tailwind-inspired builder API with Rust's full type system behind it.",
                    IconName::Code,
                    cx,
                ))
                .child(feature_card(
                    "Reactive & Fast",
                    "Immediate-mode rendering with entity-based state management.",
                    IconName::ArrowCircle,
                    cx,
                )),
        )
        // ── Component list ─
        .child(
            v_flex()
                .gap_4()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Component Library"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors().text_muted)
                        .child(
                            "This showcase covers every component in `crates/ui`. \
                             Use the sidebar to navigate. Interactive examples are highlighted \
                             with a blue border and an \"Interactive\" badge.",
                        ),
                ),
        )
}

fn feature_card(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    icon: IconName,
    cx: &App,
) -> impl IntoElement {
    let title: SharedString = title.into();
    let body: SharedString = body.into();
    v_flex()
        .gap_3()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().colors().border)
        .bg(cx.theme().colors().surface_background)
        .w(gpui::px(200.))
        .child(
            Icon::new(icon)
                .size(IconSize::XLarge)
                .color(Color::Accent),
        )
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().colors().text_muted)
                .child(body),
        )
}

// ── Avatar ─────────────────────────────────────────────────────────────────────

pub fn render_avatar_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Avatar",
            "An image element with a fallback for representing users.",
        ))
        .child(showcase_card(
            "Shapes",
            Some("Avatars are always circular in GPUI."),
            h_flex()
                .gap_6()
                .items_center()
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(Avatar::new("https://picsum.photos/seed/a/40"))
                        .child(Label::new("User A").size(LabelSize::Small).color(Color::Muted)),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(Avatar::new("https://picsum.photos/seed/b/40"))
                        .child(Label::new("User B").size(LabelSize::Small).color(Color::Muted)),
                )
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(Avatar::new("https://picsum.photos/seed/c/40"))
                        .child(Label::new("User C").size(LabelSize::Small).color(Color::Muted)),
                ),
            cx,
        ))
        .child(showcase_card(
            "Sizes",
            Some("Different avatar sizes using pixel values."),
            h_flex()
                .gap_6()
                .items_end()
                .child(sized_avatar("XS", gpui::px(20.)))
                .child(sized_avatar("S", gpui::px(28.)))
                .child(sized_avatar("M", gpui::px(36.)))
                .child(sized_avatar("L", gpui::px(48.))),
            cx,
        ))
}

fn sized_avatar(label: impl Into<SharedString>, size: gpui::Pixels) -> impl IntoElement {
    let label: SharedString = label.into();
    v_flex()
        .gap_2()
        .items_center()
        .child(Avatar::new("https://picsum.photos/seed/c/40").size(size))
        .child(Label::new(label).size(LabelSize::Small).color(Color::Muted))
}

// ── Label ──────────────────────────────────────────────────────────────────────

pub fn render_label_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Label",
            "Renders text in the UI font with size and color control.",
        ))
        .child(showcase_card(
            "Sizes",
            Some("All label size variants."),
            h_flex()
                .gap_6()
                .items_end()
                .child(lr("Large", LabelSize::Large))
                .child(lr("Default", LabelSize::Default))
                .child(lr("Small", LabelSize::Small))
                .child(lr("XSmall", LabelSize::XSmall)),
            cx,
        ))
        .child(showcase_card(
            "Colors",
            Some("Semantic text colors."),
            h_flex()
                .gap_6()
                .flex_wrap()
                .child(Label::new("Default").color(Color::Default))
                .child(Label::new("Muted").color(Color::Muted))
                .child(Label::new("Accent").color(Color::Accent))
                .child(Label::new("Error").color(Color::Error))
                .child(Label::new("Warning").color(Color::Warning))
                .child(Label::new("Success").color(Color::Success))
                .child(Label::new("Hidden").color(Color::Hidden))
                .child(Label::new("Disabled").color(Color::Disabled)),
            cx,
        ))
        .child(showcase_card(
            "Overflow",
            Some("Labels can truncate or wrap long text."),
            v_flex()
                .gap_3()
                .w(gpui::px(300.))
                .child(
                    v_flex()
                        .gap_1()
                        .child(Label::new("Truncated").size(LabelSize::Small).color(Color::Muted))
                        .child(
                            Label::new(
                                "This is a very long label that will be truncated with an ellipsis.",
                            )
                            .truncate(),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(Label::new("Line-clamped").size(LabelSize::Small).color(Color::Muted))
                        .child(Label::new(
                            "This is a very long label that will be clamped to two lines maximum.",
                        )),
                ),
            cx,
        ))
}

fn lr(text: impl Into<SharedString>, size: LabelSize) -> impl IntoElement {
    let text: SharedString = text.into();
    v_flex()
        .gap_1()
        .items_center()
        .child(Label::new(text).size(size))
        .child(
            Label::new(format!("{:?}", size))
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
}

// ── Icon ───────────────────────────────────────────────────────────────────────

pub fn render_icon_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Icon",
            "SVG icons from the bundled icon library.",
        ))
        .child(showcase_card(
            "Sizes",
            Some("Indicator (10 px) → XLarge (48 px)."),
            h_flex()
                .gap_5()
                .items_end()
                .child(icon_size_demo(IconSize::Indicator, "Ind"))
                .child(icon_size_demo(IconSize::XSmall, "XS"))
                .child(icon_size_demo(IconSize::Small, "S"))
                .child(icon_size_demo(IconSize::Medium, "Med"))
                .child(icon_size_demo(IconSize::XLarge, "XL")),
            cx,
        ))
        .child(showcase_card(
            "Colors",
            Some("Icons in semantic colors."),
            h_flex()
                .gap_5()
                .items_center()
                .child(Icon::new(IconName::Star).color(Color::Default))
                .child(Icon::new(IconName::Star).color(Color::Accent))
                .child(Icon::new(IconName::Star).color(Color::Error))
                .child(Icon::new(IconName::Star).color(Color::Warning))
                .child(Icon::new(IconName::Star).color(Color::Success))
                .child(Icon::new(IconName::Star).color(Color::Muted))
                .child(Icon::new(IconName::Star).color(Color::Disabled)),
            cx,
        ))
        .child(showcase_card(
            "Icon Gallery",
            Some("A selection of commonly-used icons."),
            div()
                .flex()
                .flex_wrap()
                .gap_3()
                .children(GALLERY_ICONS.iter().map(|(name, icon)| {
                    v_flex()
                        .gap_1()
                        .items_center()
                        .w(gpui::px(64.))
                        .child(Icon::new(*icon).size(IconSize::Medium).color(Color::Default))
                        .child(
                            Label::new(*name)
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .truncate(),
                        )
                })),
            cx,
        ))
}

fn icon_size_demo(size: IconSize, label: impl Into<SharedString>) -> impl IntoElement {
    let label: SharedString = label.into();
    v_flex()
        .gap_1()
        .items_center()
        .child(Icon::new(IconName::Star).size(size))
        .child(Label::new(label).size(LabelSize::XSmall).color(Color::Muted))
}

const GALLERY_ICONS: &[(&str, IconName)] = &[
    ("Settings", IconName::Settings),
    ("Search", IconName::MagnifyingGlass),
    ("File", IconName::File),
    ("Folder", IconName::Folder),
    ("Close", IconName::Close),
    ("Check", IconName::Check),
    ("Warning", IconName::Warning),
    ("Error", IconName::XCircle),
    ("Info", IconName::Info),
    ("Plus", IconName::Plus),
    ("Minus", IconName::SquareMinus),
    ("Trash", IconName::Trash),
    ("Copy", IconName::Copy),
    ("Undo", IconName::Undo),
    ("Redo", IconName::RotateCw),
    ("Download", IconName::Download),
    ("GitHub", IconName::Github),
    ("Branch", IconName::GitBranch),
    ("Star", IconName::Star),
    ("Starred", IconName::StarFilled),
    ("Eye", IconName::Eye),
    ("Code", IconName::Code),
    ("Terminal", IconName::Terminal),
    ("Arrow →", IconName::ArrowRight),
    ("Chevron ↓", IconName::ChevronDown),
    ("Person", IconName::Person),
    ("Pencil", IconName::Pencil),
    ("Sparkle", IconName::Sparkle),
    ("Filter", IconName::Filter),
];

// ── Facepile ───────────────────────────────────────────────────────────────────

pub fn render_facepile_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Facepile",
            "A group of overlapping avatars representing multiple users.",
        ))
        .child(showcase_card(
            "Basic Facepile",
            Some("Avatars stack and overlap in a horizontal row."),
            v_flex()
                .gap_5()
                .child(
                    Facepile::empty()
                        .child(Avatar::new("https://picsum.photos/seed/u1/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u2/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u3/32")),
                )
                .child(
                    Facepile::empty()
                        .child(Avatar::new("https://picsum.photos/seed/u4/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u5/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u6/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u7/32"))
                        .child(Avatar::new("https://picsum.photos/seed/u8/32")),
                ),
            cx,
        ))
}

// ── DiffStat ───────────────────────────────────────────────────────────────────

pub fn render_diff_stat_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Diff Stat",
            "Visual representation of lines added and removed in a diff.",
        ))
        .child(showcase_card(
            "Variants",
            Some("Different addition/removal ratios."),
            v_flex()
                .gap_4()
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(ui::DiffStat::new("diff-0", 100usize, 0usize))
                        .child(Label::new("100 additions, 0 removals").color(Color::Muted)),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(ui::DiffStat::new("diff-1", 50usize, 50usize))
                        .child(Label::new("50 additions, 50 removals").color(Color::Muted)),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(ui::DiffStat::new("diff-2", 0usize, 100usize))
                        .child(Label::new("0 additions, 100 removals").color(Color::Muted)),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(ui::DiffStat::new("diff-3", 80usize, 20usize))
                        .child(Label::new("80 additions, 20 removals").color(Color::Muted)),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(ui::DiffStat::new("diff-4", 1usize, 500usize))
                        .child(Label::new("1 addition, 500 removals").color(Color::Muted)),
                ),
            cx,
        ))
}

// ── Divider ────────────────────────────────────────────────────────────────────

pub fn render_divider_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Divider",
            "A thin line that groups, separates, or marks content.",
        ))
        .child(showcase_card(
            "Horizontal",
            Some("Standard horizontal rule."),
            v_flex()
                .gap_4()
                .w_full()
                .child(Label::new("Content above divider").color(Color::Muted))
                .child(Divider::horizontal())
                .child(Label::new("Content below divider").color(Color::Muted)),
            cx,
        ))
        .child(showcase_card(
            "Vertical",
            Some("Vertical separator, e.g. in toolbars."),
            h_flex()
                .gap_3()
                .items_center()
                .h(gpui::px(40.))
                .child(Label::new("Left").color(Color::Muted))
                .child(Divider::vertical())
                .child(Label::new("Right").color(Color::Muted)),
            cx,
        ))
        .child(showcase_card(
            "With Label",
            Some("A labelled rule for sectioning — custom implementation."),
            v_flex()
                .gap_4()
                .w_full()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().h_px().bg(cx.theme().colors().border))
                        .child(
                            Label::new("OR")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                        .child(div().flex_1().h_px().bg(cx.theme().colors().border)),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().h_px().bg(cx.theme().colors().border))
                        .child(
                            Label::new("CONTINUE")
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        )
                        .child(div().flex_1().h_px().bg(cx.theme().colors().border)),
                ),
            cx,
        ))
}

// ── Callout ────────────────────────────────────────────────────────────────────

pub fn render_callout_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Callout",
            "Highlighted boxes to draw attention to important information.",
        ))
        .child(showcase_card(
            "Severity Variants",
            Some("Info, Tip, Warning, Error."),
            v_flex()
                .gap_4()
                .w_full()
                .child(
                    Callout::new()
                        .description("A general informational note about the feature.")
                        .severity(Severity::Info),
                )
                .child(
                    Callout::new()
                        .description("Consider using this pattern for better performance.")
                        .severity(Severity::Success),
                )
                .child(
                    Callout::new()
                        .description("This setting will be deprecated in the next release.")
                        .severity(Severity::Warning),
                )
                .child(
                    Callout::new()
                        .description("A critical error occurred. Verify your configuration.")
                        .severity(Severity::Error),
                ),
            cx,
        ))
}

// ── Banner ─────────────────────────────────────────────────────────────────────

pub fn render_banner_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Banner",
            "A full-width status bar with icon and action.",
        ))
        .child(showcase_card(
            "Banner Variants",
            Some("Banners display inline notifications with an optional action."),
            v_flex()
                .gap_4()
                .w_full()
                .child(
                    ui::Banner::new()
                        .children([Label::new("Update available: Zed 1.2.0 is ready to install.").into_any_element()]),
                )
                .child(
                    ui::Banner::new()
                        .severity(Severity::Warning)
                        .children([Label::new("Your session will expire in 5 minutes. Extend it now.").into_any_element()]),
                )
                .child(
                    ui::Banner::new()
                        .severity(Severity::Error)
                        .children([Label::new("Connection error: could not reach server.").into_any_element()]),
                ),
            cx,
        ))
}

// ── Keybinding ─────────────────────────────────────────────────────────────────

pub fn render_keybinding_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header(
            "Keybinding",
            "Visual representation of keyboard shortcuts.",
        ))
        .child(showcase_card(
            "Common Shortcuts",
            Some("Keybindings as they typically appear in Zed's UI."),
            v_flex()
                .gap_4()
                .children(
                    [
                        ("Open file", vec!["ctrl", "p"]),
                        ("Save", vec!["ctrl", "s"]),
                        ("Command palette", vec!["ctrl", "shift", "p"]),
                        ("New window", vec!["ctrl", "shift", "n"]),
                    ]
                    .iter()
                    .map(|(label, keys)| {
                        h_flex()
                            .gap_3()
                            .items_center()
                            .justify_between()
                            .child(Label::new(*label).color(Color::Default))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .children(keys.iter().map(|k| {
                                        div()
                                            .px_1p5()
                                            .py_0p5()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(gpui::transparent_black())
                                            .bg(gpui::transparent_black())
                                            .text_xs()
                                            .font_family("Zed Mono")
                                            .child(*k)
                                    })),
                            )
                    }),
                ),
            cx,
        ))
}

// ── List ───────────────────────────────────────────────────────────────────────

pub fn render_list_section(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_6()
        .child(section_header("List", "Vertically stacked interactive items."))
        .child(showcase_card(
            "Basic List",
            Some("Standard list items with optional icons and meta info."),
            v_flex()
                .gap_0()
                .w_full()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().colors().border)
                .overflow_hidden()
                .children(
                    [
                        ("projects.toml", "Modified 2m ago", IconName::File),
                        ("main.rs", "Modified 4m ago", IconName::File),
                        ("Cargo.toml", "Modified 1h ago", IconName::File),
                        ("README.md", "Modified 2 days ago", IconName::File),
                    ]
                    .iter()
                    .enumerate()
                    .map(|(i, (name, meta, icon))| {
                        h_flex()
                            .px_3()
                            .py_2()
                            .gap_3()
                            .items_center()
                            .when(i > 0, |this| {
                                this.border_t_1()
                                    .border_color(cx.theme().colors().border)
                            })
                            .hover(|s| s.bg(cx.theme().colors().element_hover))
                            .child(Icon::new(*icon).size(IconSize::Small).color(Color::Muted))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .child(Label::new(*name))
                                    .child(
                                        Label::new(*meta)
                                            .size(LabelSize::XSmall)
                                            .color(Color::Muted),
                                    ),
                            )
                    }),
                ),
            cx,
        ))
        .child(showcase_card(
            "List with Badges",
            Some("List items with status chips."),
            v_flex()
                .gap_0()
                .w_full()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().colors().border)
                .overflow_hidden()
                .children(
                    [
                        ("Production", Color::Success, "Healthy"),
                        ("Staging", Color::Warning, "Degraded"),
                        ("Development", Color::Accent, "Running"),
                        ("Testing", Color::Error, "Error"),
                    ]
                    .iter()
                    .enumerate()
                    .map(|(i, (name, color, status))| {
                        h_flex()
                            .px_3()
                            .py_2()
                            .gap_3()
                            .items_center()
                            .justify_between()
                            .when(i > 0, |this| {
                                this.border_t_1()
                                    .border_color(cx.theme().colors().border)
                            })
                            .hover(|s| s.bg(cx.theme().colors().element_hover))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(ui::Indicator::dot().color(*color))
                                    .child(Label::new(*name)),
                            )
                            .child(
                                ui::Chip::new(*status).label_color(*color),
                            )
                    }),
                ),
            cx,
        ))
}














[package]
name = "ui_showcase"
version = "0.1.0"
edition.workspace = true
publish = false

[lints]
workspace = true

[lib]
path = "src/lib.rs"

[dependencies]
component.workspace = true
gpui.workspace = true
settings.workspace = true
theme.workspace = true
ui.workspace = true
release_channel.workspace = true
