use crate::ai::{ChatMessage, ChatRole};
use crate::theme::Theme;
use gpui::{div, prelude::*, px, AnyElement, IntoElement};

/// Chat panel renders the conversation in the center stage.
pub struct ChatPanel;

impl ChatPanel {
    pub fn render(
        theme: &Theme,
        messages: &[ChatMessage],
        is_sending: bool,
        chat_input: &str,
        provider_name: &str,
        model_name: &str,
        is_authenticated: bool,
    ) -> AnyElement {
        let send_button = div()
            .flex_shrink_0()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .bg(if is_sending || !is_authenticated {
                theme.muted
            } else {
                theme.primary
            })
            .text_size(px(12.0))
            .text_color(if is_sending || !is_authenticated {
                theme.muted_foreground
            } else {
                theme.primary_foreground
            })
            .child("Send")
            .into_any_element();

        Self::render_with_send(
            theme,
            messages,
            is_sending,
            chat_input,
            provider_name,
            model_name,
            is_authenticated,
            send_button,
        )
    }

    pub fn render_with_send(
        theme: &Theme,
        messages: &[ChatMessage],
        is_sending: bool,
        chat_input: &str,
        provider_name: &str,
        model_name: &str,
        is_authenticated: bool,
        send_button: AnyElement,
    ) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(Self::render_header(theme, provider_name, model_name, is_authenticated))
            .child(Self::render_messages(theme, messages, is_sending))
            .child(Self::render_input_area(
                theme,
                chat_input,
                is_sending,
                is_authenticated,
                send_button,
            ))
            .into_any_element()
    }

    fn render_header(
        theme: &Theme,
        provider_name: &str,
        model_name: &str,
        is_authenticated: bool,
    ) -> AnyElement {
        let provider_name = provider_name.to_string();
        let model_name = model_name.to_string();

        div()
            .w_full()
            .h(px(44.0))
            .flex_shrink_0()
            .px(px(16.0))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child("DX Agent"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(theme.foreground.opacity(0.5))
                            .child("•"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.foreground.opacity(0.6))
                            .child(format!("{} / {}", provider_name, model_name)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded(px(4.0)).bg(if is_authenticated {
                        gpui::rgb(0x22c55e)
                    } else {
                        gpui::rgb(0xef4444)
                    }))
                    .child(
                        div().text_size(px(11.0)).text_color(theme.foreground.opacity(0.6)).child(
                            if is_authenticated {
                                "Ready"
                            } else {
                                "No API Key"
                            },
                        ),
                    ),
            )
            .into_any_element()
    }

    fn render_messages(theme: &Theme, messages: &[ChatMessage], is_sending: bool) -> AnyElement {
        let mut container = div()
            .id("chat-messages")
            .flex_1()
            .w_full()
            .overflow_y_scroll()
            .px(px(16.0))
            .py(px(12.0))
            .flex()
            .flex_col()
            .gap(px(16.0));

        if messages.is_empty() {
            container = container.child(Self::render_empty_state(theme));
        } else {
            for msg in messages {
                container = container.child(Self::render_message(theme, msg));
            }
        }

        if is_sending {
            container = container.child(Self::render_typing_indicator(theme));
        }

        container.into_any_element()
    }

    fn render_empty_state(theme: &Theme) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .child(div().text_size(px(28.0)).text_color(theme.foreground.opacity(0.15)).child("DX"))
            .child(
                div()
                    .text_size(px(16.0))
                    .text_color(theme.foreground.opacity(0.4))
                    .child("How can I help you today?"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.foreground.opacity(0.3))
                    .child("Select a provider and enter your API key to get started."),
            )
            .into_any_element()
    }

    fn render_message(theme: &Theme, msg: &ChatMessage) -> AnyElement {
        let is_user = msg.role == ChatRole::User;
        let content = msg.content.clone();

        div()
            .w_full()
            .flex()
            .when(is_user, |this| this.justify_end())
            .when(!is_user, |this| this.justify_start())
            .child(
                div()
                    .max_w(px(600.0))
                    .px(px(14.0))
                    .py(px(10.0))
                    .rounded(px(12.0))
                    .when(is_user, |this| {
                        this.bg(theme.primary).text_color(theme.primary_foreground)
                    })
                    .when(!is_user, |this| {
                        this.bg(theme.card)
                            .border_1()
                            .border_color(theme.border)
                            .text_color(theme.foreground)
                    })
                    .child(div().text_size(px(13.0)).child(content)),
            )
            .into_any_element()
    }

    fn render_typing_indicator(theme: &Theme) -> AnyElement {
        div()
            .w_full()
            .flex()
            .justify_start()
            .child(
                div()
                    .px(px(14.0))
                    .py(px(10.0))
                    .rounded(px(12.0))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.border)
                    .child(
                        div().flex().items_center().gap(px(4.0)).child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.foreground.opacity(0.5))
                                .child("Thinking..."),
                        ),
                    ),
            )
            .into_any_element()
    }

    fn render_input_area(
        theme: &Theme,
        chat_input: &str,
        is_sending: bool,
        is_authenticated: bool,
        send_button: AnyElement,
    ) -> AnyElement {
        let input_text = if chat_input.is_empty() {
            if !is_authenticated {
                "Enter an API key first...".to_string()
            } else if is_sending {
                "Waiting for response...".to_string()
            } else {
                "Type a message... (press Enter to send)".to_string()
            }
        } else {
            chat_input.to_string()
        };

        let is_placeholder = chat_input.is_empty();

        div()
            .w_full()
            .flex_shrink_0()
            .p(px(12.0))
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .w_full()
                    .min_h(px(44.0))
                    .px(px(14.0))
                    .py(px(10.0))
                    .rounded(px(12.0))
                    .bg(theme.card)
                    .border_1()
                    .border_color(if is_placeholder {
                        theme.border
                    } else {
                        theme.primary
                    })
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.0))
                            .text_color(if is_placeholder {
                                theme.foreground.opacity(0.4)
                            } else {
                                theme.foreground
                            })
                            .child(input_text),
                    )
                    .child(send_button),
            )
            .into_any_element()
    }
}
