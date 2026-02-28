use crate::ai::{AiProviderKind, AiRegistry, AiSettings};
use crate::theme::Theme;
use gpui::{div, prelude::*, px, AnyElement, Hsla, IntoElement};

/// Convert an RGB hex to Hsla (opaque).
fn rgb_color(hex: u32) -> Hsla {
    gpui::rgb(hex).into()
}

/// AI Provider Switcher Component - renders the left sidebar with providers.
pub struct AiProviderSwitcher;

impl AiProviderSwitcher {
    pub fn render(theme: &Theme, registry: &AiRegistry) -> AnyElement {
        let active_kind = registry.active_provider_kind();

        let mut panel = div().w_full().flex().flex_col().gap(px(4.0)).p(px(12.0)).child(
            div()
                .text_size(px(11.0))
                .text_color(theme.foreground.opacity(0.6))
                .child("AI PROVIDERS"),
        );

        for kind in AiProviderKind::all() {
            let is_selected = active_kind == Some(*kind);
            let has_key = if kind.needs_api_key() {
                registry.credentials.has_api_key(kind.id())
            } else {
                true // Local providers (Ollama, LMStudio) don't need keys
            };
            panel = panel.child(Self::render_provider_button(
                theme,
                kind.display_name(),
                *kind,
                is_selected,
                has_key,
            ));
        }

        panel.into_any_element()
    }

    fn render_provider_button(
        theme: &Theme,
        label: &str,
        _kind: AiProviderKind,
        is_selected: bool,
        has_key: bool,
    ) -> AnyElement {
        let label = label.to_string();

        div()
            .w_full()
            .h(px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(6.0))
            .cursor_pointer()
            .when(is_selected, |this| {
                this.bg(theme.primary.opacity(0.1)).border_1().border_color(theme.primary)
            })
            .when(!is_selected, |this| {
                this.bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .hover(|this| this.bg(theme.border.opacity(0.3)))
            })
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(if is_selected {
                        theme.primary
                    } else {
                        theme.foreground
                    })
                    .child(label),
            )
            .child(div().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(if has_key {
                gpui::rgb(0x22c55e)
            } else {
                gpui::rgb(0x6b7280)
            }))
            .into_any_element()
    }
}

/// API Key Input section for the right sidebar.
pub struct ApiKeySection;

impl ApiKeySection {
    pub fn render(
        theme: &Theme,
        provider_kind: Option<AiProviderKind>,
        has_api_key: bool,
        api_key_input: &str,
    ) -> AnyElement {
        let kind = match provider_kind {
            Some(k) => k,
            None => return div().into_any_element(),
        };

        if !kind.needs_api_key() {
            return div()
                .w_full()
                .p(px(12.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.foreground.opacity(0.6))
                        .child("AUTHENTICATION"),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .px(px(10.0))
                        .py(px(8.0))
                        .rounded(px(6.0))
                        .bg(rgb_color(0x22c55e).opacity(0.1))
                        .border_1()
                        .border_color(rgb_color(0x22c55e).opacity(0.3))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.foreground.opacity(0.7))
                                .child("No API key required (local provider)"),
                        ),
                )
                .into_any_element();
        }

        let input_text = if api_key_input.is_empty() {
            kind.api_key_hint().to_string()
        } else {
            // Mask the API key for display
            let len = api_key_input.len();
            if len > 8 {
                format!("{}...{}", &api_key_input[..4], &api_key_input[len - 4..])
            } else {
                "*".repeat(len)
            }
        };

        let is_placeholder = api_key_input.is_empty();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.foreground.opacity(0.6))
                    .child("AUTHENTICATION"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(if has_api_key {
                        gpui::rgb(0x22c55e)
                    } else {
                        gpui::rgb(0xef4444)
                    }))
                    .child(
                        div().text_size(px(12.0)).text_color(theme.foreground.opacity(0.7)).child(
                            if has_api_key {
                                format!("{} API key set", kind.display_name())
                            } else {
                                format!("No {} API key", kind.display_name())
                            },
                        ),
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
                    .bg(theme.card)
                    .border_1()
                    .border_color(if is_placeholder {
                        theme.border
                    } else {
                        theme.primary
                    })
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(if is_placeholder {
                                theme.foreground.opacity(0.4)
                            } else {
                                theme.foreground
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
                    .bg(theme.primary)
                    .text_size(px(12.0))
                    .text_color(theme.primary_foreground)
                    .hover(|this| this.bg(theme.primary.opacity(0.8)))
                    .child(if has_api_key {
                        "Update API Key"
                    } else {
                        "Save API Key"
                    }),
            )
            .into_any_element()
    }
}

/// AI Model Selector Component
pub struct AiModelSelector;

impl AiModelSelector {
    pub fn render(theme: &Theme, registry: &AiRegistry) -> AnyElement {
        let active_provider = registry.active_provider();
        let active_model = registry.active_model();

        let models = match active_provider {
            Some(p) => &p.models,
            None => return div().into_any_element(),
        };

        let mut panel = div().w_full().flex().flex_col().gap(px(4.0)).p(px(12.0)).child(
            div()
                .text_size(px(11.0))
                .text_color(theme.foreground.opacity(0.6))
                .child("MODEL"),
        );

        for model in models {
            let is_selected = active_model.as_ref().map_or(false, |m| m == &model.id);
            let context_str = if model.context_window >= 1_000_000 {
                format!("{}M context", model.context_window / 1_000_000)
            } else {
                format!("{}K context", model.context_window / 1000)
            };
            panel = panel.child(Self::render_model_option(
                theme,
                &model.name,
                &context_str,
                is_selected,
            ));
        }

        panel.into_any_element()
    }

    fn render_model_option(
        theme: &Theme,
        label: &str,
        context: &str,
        is_selected: bool,
    ) -> AnyElement {
        let label = label.to_string();
        let context = context.to_string();

        div()
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .when(is_selected, |this| {
                this.bg(theme.primary.opacity(0.1)).border_1().border_color(theme.primary)
            })
            .when(!is_selected, |this| {
                this.bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .hover(|this| this.bg(theme.border.opacity(0.3)))
            })
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(if is_selected {
                        theme.primary
                    } else {
                        theme.foreground
                    })
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.foreground.opacity(0.5))
                    .child(context),
            )
            .into_any_element()
    }
}

/// AI Settings Panel Component
pub struct AiSettingsPanel;

impl AiSettingsPanel {
    pub fn render(theme: &Theme, settings: &AiSettings) -> AnyElement {
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(12.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.foreground.opacity(0.6))
                    .child("AI SETTINGS"),
            )
            .child(Self::render_setting(
                theme,
                "Temperature",
                &format!("{:.1}", settings.temperature),
            ))
            .child(Self::render_setting(theme, "Max Tokens", &settings.max_tokens.to_string()))
            .child(Self::render_setting(theme, "Top P", &format!("{:.1}", settings.top_p)))
            .child(Self::render_toggle(theme, "Stream Response", settings.stream_response))
            .child(Self::render_toggle(theme, "Use Tools", settings.use_tools))
            .child(Self::render_toggle(theme, "MCP Enabled", settings.mcp_enabled))
            .child(Self::render_toggle(theme, "ACP Enabled", settings.acp_enabled))
            .into_any_element()
    }

    pub fn render_setting(theme: &Theme, label: &str, value: &str) -> AnyElement {
        let label = label.to_string();
        let value = value.to_string();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(div().text_size(px(12.0)).text_color(theme.foreground).child(label))
            .child(
                div()
                    .w_full()
                    .h(px(32.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .child(div().text_size(px(13.0)).text_color(theme.foreground).child(value)),
            )
            .into_any_element()
    }

    fn render_toggle(theme: &Theme, label: &str, enabled: bool) -> AnyElement {
        let label = label.to_string();

        div()
            .w_full()
            .h(px(36.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(6.0))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|this| this.bg(theme.border.opacity(0.3)))
            .child(div().text_size(px(13.0)).text_color(theme.foreground).child(label))
            .child(
                div()
                    .w(px(40.0))
                    .h(px(20.0))
                    .rounded(px(10.0))
                    .flex()
                    .items_center()
                    .when(enabled, |this| this.bg(theme.primary).justify_end())
                    .when(!enabled, |this| this.bg(theme.border).justify_start())
                    .px(px(2.0))
                    .child(div().w(px(16.0)).h(px(16.0)).rounded(px(8.0)).bg(gpui::white())),
            )
            .into_any_element()
    }
}

/// AI Status Bar Component
pub struct AiStatusBar;

impl AiStatusBar {
    pub fn render(theme: &Theme, registry: &AiRegistry) -> AnyElement {
        let provider_name =
            registry.active_provider().map(|p| p.kind.display_name()).unwrap_or("None");
        let model_name = registry.active_model().unwrap_or_default();
        let is_auth = registry.is_active_authenticated();

        div()
            .w_full()
            .h(px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(if is_auth {
                        gpui::rgb(0x22c55e)
                    } else {
                        gpui::rgb(0xef4444)
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.foreground.opacity(0.7))
                            .child(if is_auth { "Connected" } else { "No API Key" }),
                    ),
            )
            .child(div().text_size(px(11.0)).text_color(theme.foreground.opacity(0.5)).child("•"))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.foreground.opacity(0.7))
                    .child(provider_name.to_string()),
            )
            .child(div().text_size(px(11.0)).text_color(theme.foreground.opacity(0.5)).child("•"))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.foreground.opacity(0.7))
                    .child(Self::short_model_name(&model_name)),
            )
            .into_any_element()
    }

    fn short_model_name(model_id: &str) -> String {
        // Shorten model ID for display
        if let Some(name) = model_id.split('/').last() {
            name.to_string()
        } else {
            model_id.to_string()
        }
    }
}
