mod ai_panel;
mod chat_panel;
mod controls;
mod layout;
mod zed_ai_panel;

use crate::ai::{AiClient, AiProviderKind, AiRegistry, AiSettings, ChatMessage};
use crate::theme::{Theme, ThemeMode};
use ai_panel::{AiSettingsPanel, AiStatusBar};
use chat_panel::ChatPanel;
use gpui::{
    div, prelude::*, px, AnyElement, Context, IntoElement, KeyDownEvent, MouseButton, Pixels,
    Render, Window,
};
use gpui_component::{PixelsExt, Theme as UiTheme, ThemeMode as UiThemeMode};
// use zed_ai_panel::ZedAiPanel;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidePosition {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QuickInputPosition {
    Top,
    Center,
}

pub struct HomeView {
    theme: Theme,
    theme_mode: ThemeMode,
    show_top_bar: bool,
    show_action_bar: bool,
    show_primary_sidebar: bool,
    show_secondary_sidebar: bool,
    show_status_bar: bool,
    primary_sidebar_position: SidePosition,
    quick_input_position: QuickInputPosition,
    layout_revision: u64,
    action_bar_width_px: f32,
    primary_sidebar_width_px: f32,
    secondary_sidebar_width_px: f32,
    // AI State
    ai_registry: AiRegistry,
    ai_settings: AiSettings,
    chat_messages: Vec<ChatMessage>,
    chat_input: String,
    is_sending: bool,
}

impl HomeView {
    pub fn new(_theme: Theme, cx: &mut Context<Self>) -> Self {
        let theme_mode = match UiTheme::global(cx).mode {
            UiThemeMode::Light => ThemeMode::Light,
            UiThemeMode::Dark => ThemeMode::Dark,
        };

        Self {
            theme: Theme::new(theme_mode),
            theme_mode,
            show_top_bar: true,
            show_action_bar: true,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_status_bar: true,
            primary_sidebar_position: SidePosition::Left,
            quick_input_position: QuickInputPosition::Center,
            layout_revision: 0,
            action_bar_width_px: 44.0,
            primary_sidebar_width_px: 160.0,
            secondary_sidebar_width_px: 240.0,
            // AI State
            ai_registry: AiRegistry::new(),
            ai_settings: AiSettings::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            is_sending: false,
        }
    }

    fn touch_layout(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
    }

    fn sync_column_widths_from_sizes(&mut self, sizes: &[Pixels]) {
        let mut index = 0;

        // Action bar is no longer part of resizable columns
        match self.primary_sidebar_position {
            SidePosition::Left => {
                if self.show_primary_sidebar {
                    if let Some(size) = sizes.get(index) {
                        self.primary_sidebar_width_px = size.as_f32();
                    }
                    index += 1;
                }
            }
            SidePosition::Right => {
                if self.show_secondary_sidebar {
                    if let Some(size) = sizes.get(index) {
                        self.secondary_sidebar_width_px = size.as_f32();
                    }
                    index += 1;
                }
            }
        }

        // Skip center column
        if sizes.get(index).is_some() {
            index += 1;
        }

        match self.primary_sidebar_position {
            SidePosition::Left => {
                if self.show_secondary_sidebar {
                    if let Some(size) = sizes.get(index) {
                        self.secondary_sidebar_width_px = size.as_f32();
                    }
                }
            }
            SidePosition::Right => {
                if self.show_primary_sidebar {
                    if let Some(size) = sizes.get(index) {
                        self.primary_sidebar_width_px = size.as_f32();
                    }
                }
            }
        }
    }

    fn set_theme_mode(&mut self, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_mode = mode;
        self.theme = Theme::new(mode);

        let ui_mode = match mode {
            ThemeMode::Light => UiThemeMode::Light,
            ThemeMode::Dark => UiThemeMode::Dark,
        };
        UiTheme::change(ui_mode, Some(window), cx);

        cx.notify();
    }

    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next_mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.set_theme_mode(next_mode, window, cx);
    }

    fn panel_bg(&self, _has_left_border: bool) -> AnyElement {
        div().size_full().bg(self.theme.background).into_any_element()
    }

    fn render_primary_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(self.theme.background)
            .child(
                div()
                    .id("ai-provider-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.render_provider_list(cx)),
            )
            .child(AiStatusBar::render(&self.theme, &self.ai_registry))
            .into_any_element()
    }

    /// Renders the provider list with click handlers for each provider.
    fn render_provider_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let active_kind = self.ai_registry.active_provider_kind();

        let mut panel = div().w_full().flex().flex_col().gap(px(4.0)).p(px(12.0)).child(
            div()
                .text_size(px(11.0))
                .text_color(self.theme.foreground.opacity(0.6))
                .child("AI PROVIDERS"),
        );

        for kind in AiProviderKind::all() {
            let is_selected = active_kind == Some(*kind);
            let has_key = if kind.needs_api_key() {
                self.ai_registry.credentials.has_api_key(kind.id())
            } else {
                true
            };
            let provider_kind = *kind;

            panel = panel.child(
                div()
                    .id(kind.id())
                    .w_full()
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .when(is_selected, |this| {
                        this.bg(self.theme.primary.opacity(0.1))
                            .border_1()
                            .border_color(self.theme.primary)
                    })
                    .when(!is_selected, |this| {
                        this.bg(self.theme.background)
                            .border_1()
                            .border_color(self.theme.border)
                            .hover(|this| this.bg(self.theme.border.opacity(0.3)))
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _event, _window, cx| {
                            view.ai_registry.set_active_provider(provider_kind);
                            view.chat_input.clear();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(if is_selected {
                                self.theme.primary
                            } else {
                                self.theme.foreground
                            })
                            .child(kind.display_name().to_string()),
                    )
                    .child(div().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(if has_key {
                        gpui::rgb(0x22c55e)
                    } else {
                        gpui::rgb(0x6b7280)
                    })),
            );
        }

        panel.into_any_element()
    }

    fn render_secondary_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(self.theme.background)
            .child(
                div()
                    .id("ai-settings-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    // API Key section
                    .child(self.render_api_key_section(cx))
                    .child(
                        div()
                            .w_full()
                            .h(px(1.0))
                            .bg(self.theme.border)
                            .my(px(8.0)),
                    )
                    // Model selector with click handlers
                    .child(self.render_model_list(cx))
                    .child(
                        div()
                            .w_full()
                            .h(px(1.0))
                            .bg(self.theme.border)
                            .my(px(8.0)),
                    )
                    // Settings with toggle handlers
                    .child(self.render_settings_with_toggles(cx)),
            )
            .into_any_element()
    }

    fn render_api_key_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(kind) = self.ai_registry.active_provider_kind() else {
            return div().into_any_element();
        };

        let has_api_key = self.ai_registry.is_active_authenticated();

        if !kind.needs_api_key() {
            return div()
                .w_full()
                .p(px(12.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(self.theme.foreground.opacity(0.6))
                        .child("AUTHENTICATION"),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .px(px(10.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .bg(self.theme.success.opacity(0.1))
                        .border_1()
                        .border_color(self.theme.success.opacity(0.3))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(self.theme.foreground.opacity(0.7))
                                .child("No API key required (local provider)"),
                        ),
                )
                .into_any_element();
        }

        let stored_key = self.ai_registry.active_api_key().unwrap_or("");
        let display_key = if has_api_key {
            stored_key
        } else {
            self.chat_input.as_str()
        };

        let input_text = if display_key.is_empty() {
            kind.api_key_hint().to_string()
        } else {
            let len = display_key.len();
            if len > 8 {
                format!("{}...{}", &display_key[..4], &display_key[len - 4..])
            } else {
                "*".repeat(len)
            }
        };
        let is_placeholder = display_key.is_empty();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.theme.foreground.opacity(0.6))
                    .child("AUTHENTICATION"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(if has_api_key {
                        self.theme.success
                    } else {
                        self.theme.destructive
                    }))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(self.theme.foreground.opacity(0.7))
                            .child(if has_api_key {
                                format!("{} API key set", kind.display_name())
                            } else {
                                format!("No {} API key", kind.display_name())
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(32.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(self.theme.card)
                    .border_1()
                    .border_color(if is_placeholder {
                        self.theme.border
                    } else {
                        self.theme.primary
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if is_placeholder {
                                self.theme.foreground.opacity(0.4)
                            } else {
                                self.theme.foreground
                            })
                            .child(input_text),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .bg(self.theme.primary)
                    .text_size(px(12.0))
                    .text_color(self.theme.primary_foreground)
                    .hover(|this| this.bg(self.theme.primary.opacity(0.8)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|view, _event, _window, cx| {
                            view.save_api_key_from_input(cx);
                        }),
                    )
                    .child(if has_api_key {
                        "Update API Key"
                    } else {
                        "Save API Key"
                    }),
            )
            .into_any_element()
    }

    fn save_api_key_from_input(&mut self, cx: &mut Context<Self>) {
        let Some(kind) = self.ai_registry.active_provider_kind() else {
            return;
        };
        if !kind.needs_api_key() {
            return;
        }

        let input = self.chat_input.trim().to_string();
        if input.is_empty() {
            return;
        }

        self.ai_registry.set_api_key(kind.id(), input);
        self.chat_input.clear();
        cx.notify();
    }

    /// Renders the model list with click handlers.
    fn render_model_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let active_provider = self.ai_registry.active_provider();
        let active_model = self.ai_registry.active_model();

        let models = match active_provider {
            Some(p) => p.models.clone(),
            None => return div().into_any_element(),
        };

        let mut panel = div().w_full().flex().flex_col().gap(px(4.0)).p(px(12.0)).child(
            div()
                .text_size(px(11.0))
                .text_color(self.theme.foreground.opacity(0.6))
                .child("MODEL"),
        );

        for model in &models {
            let is_selected = active_model.as_ref().map_or(false, |m| m == &model.id);
            let model_id = model.id.clone();
            let context_str = if model.context_window >= 1_000_000 {
                format!("{}M context", model.context_window / 1_000_000)
            } else {
                format!("{}K context", model.context_window / 1000)
            };
            let model_name = model.name.clone();

            panel = panel.child(
                div()
                    .id(model_id.clone())
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .when(is_selected, |this| {
                        this.bg(self.theme.primary.opacity(0.1))
                            .border_1()
                            .border_color(self.theme.primary)
                    })
                    .when(!is_selected, |this| {
                        this.bg(self.theme.background)
                            .border_1()
                            .border_color(self.theme.border)
                            .hover(|this| this.bg(self.theme.border.opacity(0.3)))
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _event, _window, cx| {
                            view.ai_registry.set_active_model(model_id.clone());
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(if is_selected {
                                self.theme.primary
                            } else {
                                self.theme.foreground
                            })
                            .child(model_name),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(self.theme.foreground.opacity(0.5))
                            .child(context_str),
                    ),
            );
        }

        panel.into_any_element()
    }

    /// Renders settings panel with interactive toggle handlers.
    fn render_settings_with_toggles(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(12.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.theme.foreground.opacity(0.6))
                    .child("AI SETTINGS"),
            )
            .child(AiSettingsPanel::render_setting(
                &self.theme,
                "Temperature",
                &format!("{:.1}", self.ai_settings.temperature),
            ))
            .child(AiSettingsPanel::render_setting(
                &self.theme,
                "Max Tokens",
                &self.ai_settings.max_tokens.to_string(),
            ))
            .child(self.render_toggle_with_handler(
                "Stream Response",
                self.ai_settings.stream_response,
                |v, cx| {
                    v.ai_settings.toggle_stream();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_toggle_with_handler(
                "Use Tools",
                self.ai_settings.use_tools,
                |v, cx| {
                    v.ai_settings.toggle_tools();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_toggle_with_handler(
                "MCP Enabled",
                self.ai_settings.mcp_enabled,
                |v, cx| {
                    v.ai_settings.toggle_mcp();
                    cx.notify();
                },
                cx,
            ))
            .child(self.render_toggle_with_handler(
                "ACP Enabled",
                self.ai_settings.acp_enabled,
                |v, cx| {
                    v.ai_settings.toggle_acp();
                    cx.notify();
                },
                cx,
            ))
            .into_any_element()
    }

    fn render_toggle_with_handler(
        &self,
        label: &'static str,
        enabled: bool,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .w_full()
            .h(px(36.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(6.0))
            .bg(self.theme.background)
            .border_1()
            .border_color(self.theme.border)
            .cursor_pointer()
            .hover(|this| this.bg(self.theme.border.opacity(0.3)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _event, _window, cx| {
                    on_click(view, cx);
                }),
            )
            .child(div().text_size(px(13.0)).text_color(self.theme.foreground).child(label))
            .child(
                div()
                    .w(px(40.0))
                    .h(px(20.0))
                    .rounded(px(10.0))
                    .flex()
                    .items_center()
                    .when(enabled, |this| this.bg(self.theme.primary).justify_end())
                    .when(!enabled, |this| this.bg(self.theme.border).justify_start())
                    .px(px(2.0))
                    .child(div().w(px(16.0)).h(px(16.0)).rounded(px(8.0)).bg(gpui::white())),
            )
            .into_any_element()
    }

    /// Renders the center stage with the Zed AI panel.
    pub(super) fn render_center_stage(&self, _cx: &mut Context<Self>) -> AnyElement {
        // Zed AI Panel is rendered inline here (not as a separate entity)
        // This avoids the need to create a ZedAiPanel entity

        div()
            .size_full()
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(self.theme.background)
                    .child(
                        // Header
                        div()
                            .w_full()
                            .h(px(48.0))
                            .flex_shrink_0()
                            .border_b_1()
                            .border_color(self.theme.border)
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(self.theme.foreground)
                                    .child("Zed AI Assistant"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(self.theme.foreground.opacity(0.6))
                                    .child("Claude 3.5 Sonnet"),
                            ),
                    )
                    .child(
                        // Messages area
                        div().flex_1().w_full().overflow_y_hidden().p(px(16.0)).child(
                            div()
                                .text_size(px(14.0))
                                .text_color(self.theme.foreground.opacity(0.7))
                                .child("Zed AI Panel - Ready to chat!"),
                        ),
                    )
                    .child(
                        // Input area
                        div()
                            .w_full()
                            .h(px(80.0))
                            .flex_shrink_0()
                            .border_t_1()
                            .border_color(self.theme.border)
                            .bg(self.theme.background)
                            .p(px(16.0))
                            .flex()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .px(px(16.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.0))
                                    .bg(self.theme.background)
                                    .border_1()
                                    .border_color(self.theme.border)
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(self.theme.foreground.opacity(0.5))
                                            .child("Type your message..."),
                                    ),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .px(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(8.0))
                                    .bg(self.theme.primary)
                                    .hover(|this| this.bg(self.theme.primary.opacity(0.8)))
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(gpui::white())
                                            .child("Send"),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// OLD render_center_stage - commented out for reference
    /// Renders the center stage with the chat panel.
    #[allow(dead_code)]
    fn render_center_stage_old(&self, cx: &mut Context<Self>) -> AnyElement {
        let provider_name = self
            .ai_registry
            .active_provider()
            .map(|p| p.kind.display_name())
            .unwrap_or("None");
        let model_name = self.ai_registry.active_model().unwrap_or_else(|| "No model".to_string());
        let is_authenticated = self.ai_registry.is_active_authenticated();

        let send_button = div()
            .flex_shrink_0()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .bg(if self.is_sending || !is_authenticated {
                self.theme.muted
            } else {
                self.theme.primary
            })
            .text_size(px(12.0))
            .text_color(if self.is_sending || !is_authenticated {
                self.theme.muted_foreground
            } else {
                self.theme.primary_foreground
            })
            .hover(|this| this.bg(self.theme.primary.opacity(0.8)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _event, _window, cx| {
                    view.send_current_message(cx);
                }),
            )
            .child("Send")
            .into_any_element();

        let chat = ChatPanel::render_with_send(
            &self.theme,
            &self.chat_messages,
            self.is_sending,
            &self.chat_input,
            provider_name,
            &model_name,
            is_authenticated,
            send_button,
        );

        // Wrap with keyboard handler for text input
        div()
            .size_full()
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _window, cx| {
                view.handle_key_down(event, cx);
            }))
            .child(chat)
            .into_any_element()
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;

        if keystroke.key == "enter" && !keystroke.modifiers.shift {
            // Send message on Enter
            self.send_current_message(cx);
        } else if keystroke.key == "backspace" {
            self.chat_input.pop();
            cx.notify();
        } else if keystroke.key == "escape" {
            self.chat_input.clear();
            cx.notify();
        } else if let Some(ch) = keystroke.key.chars().next() {
            if !keystroke.modifiers.control && !keystroke.modifiers.alt && ch.is_ascii_graphic()
                || ch == ' '
            {
                self.chat_input.push(ch);
                cx.notify();
            }
        }
    }

    fn send_current_message(&mut self, cx: &mut Context<Self>) {
        let input = self.chat_input.trim().to_string();
        if input.is_empty() || self.is_sending {
            return;
        }

        if !self.ai_registry.is_active_authenticated() {
            // If not authenticated, treat input as an API key for the active provider.
            self.save_api_key_from_input(cx);
            return;
        }

        // Add user message
        self.chat_messages.push(ChatMessage::user(&input));
        self.chat_input.clear();
        self.is_sending = true;
        cx.notify();

        // Get provider info for the API call
        let provider_kind = match self.ai_registry.active_provider_kind() {
            Some(k) => k,
            None => {
                self.is_sending = false;
                cx.notify();
                return;
            }
        };
        let model = self.ai_registry.active_model().unwrap_or_default();
        let api_key = self.ai_registry.active_api_key().unwrap_or("").to_string();
        let messages = self.chat_messages.clone();
        let settings = self.ai_settings.clone();

        // Spawn background task for API call
        let entity = cx.entity().clone();
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    AiClient::send_message_blocking(
                        provider_kind,
                        &model,
                        &api_key,
                        &messages,
                        &settings,
                    )
                })
                .await;

            entity.update(cx, |view: &mut HomeView, cx| {
                view.is_sending = false;
                match result {
                    Ok(response) => {
                        view.chat_messages.push(ChatMessage::assistant(response));
                    }
                    Err(e) => {
                        view.chat_messages.push(ChatMessage::assistant(format!("Error: {}", e)));
                    }
                }
                cx.notify();
            })
        })
        .detach();
    }
}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .size_full()
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.background)
            .flex()
            .flex_col()
            .overflow_hidden();

        // Fixed-height top bar (not resizable)
        if self.show_top_bar {
            root = root.child(
                div()
                    .id("top-menu-bar")
                    .w_full()
                    .h(px(36.0))
                    .flex_shrink_0()
                    .bg(self.theme.background)
                    .border_b_1()
                    .border_color(self.theme.border)
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .text_size(px(13.0))
                    .text_color(self.theme.foreground)
                    .overflow_x_scroll()
                    .child("File")
                    .child("Edit")
                    .child("View")
                    .child("Go")
                    .child("Selection")
                    .child("Run")
                    .child("Terminal")
                    .child("Help"),
            );
        }

        // Main workspace area (takes remaining space)
        root = root.child(div().flex_1().min_h_0().w_full().child(self.render_main_with_panel(cx)));

        // Fixed-height status bar (not resizable)
        if self.show_status_bar {
            root = root.child(
                div()
                    .id("status-bar")
                    .w_full()
                    .h(px(24.0))
                    .flex_shrink_0()
                    .bg(self.theme.background)
                    .border_t_1()
                    .border_color(self.theme.border)
                    .overflow_x_scroll(),
            );
        }

        root
    }
}
