//! SocialShareUi — Share popover connected to social_sharing::SocialShareService.
//!
//! Part 29: Presents a popover with platform buttons, preview, and share action.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharePlatform {
    Twitter,
    Reddit,
    Discord,
    LinkedIn,
    CopyLink,
    Email,
}

impl SharePlatform {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Twitter => "𝕏 Twitter",
            Self::Reddit => "Reddit",
            Self::Discord => "Discord",
            Self::LinkedIn => "LinkedIn",
            Self::CopyLink => "📋 Copy Link",
            Self::Email => "✉️ Email",
        }
    }

    pub fn all() -> &'static [SharePlatform] {
        &[
            Self::Twitter,
            Self::Reddit,
            Self::Discord,
            Self::LinkedIn,
            Self::CopyLink,
            Self::Email,
        ]
    }
}

/// Content to be shared.
#[derive(Debug, Clone)]
pub struct ShareContent {
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    /// Base64 or file path for a preview image.
    pub image_preview: Option<String>,
    pub content_type: ShareContentType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareContentType {
    TextGeneration,
    ImageGeneration,
    AudioGeneration,
    VideoGeneration,
    CodeSnippet,
    Conversation,
}

impl ShareContentType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TextGeneration => "Text",
            Self::ImageGeneration => "Image",
            Self::AudioGeneration => "Audio",
            Self::VideoGeneration => "Video",
            Self::CodeSnippet => "Code",
            Self::Conversation => "Chat",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SocialShareEvent {
    ShareRequested {
        platform: SharePlatform,
        content: ShareContent,
    },
    Dismissed,
}

pub struct SocialShareUi {
    focus_handle: FocusHandle,
    content: Option<ShareContent>,
    is_visible: bool,
    selected_platform: Option<SharePlatform>,
}

impl SocialShareUi {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: None,
            is_visible: false,
            selected_platform: None,
        }
    }

    pub fn show(&mut self, content: ShareContent, cx: &mut ViewContext<Self>) {
        self.content = Some(content);
        self.is_visible = true;
        self.selected_platform = None;
        cx.notify();
    }

    pub fn dismiss(&mut self, cx: &mut ViewContext<Self>) {
        self.is_visible = false;
        cx.emit(SocialShareEvent::Dismissed);
        cx.notify();
    }

    pub fn select_platform(&mut self, platform: SharePlatform, cx: &mut ViewContext<Self>) {
        self.selected_platform = Some(platform);
        if let Some(content) = &self.content {
            cx.emit(SocialShareEvent::ShareRequested {
                platform,
                content: content.clone(),
            });
        }
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
}

impl gpui::EventEmitter<SocialShareEvent> for SocialShareUi {}

impl Focusable for SocialShareUi {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SocialShareUi {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let is_visible = self.is_visible;
        let content = self.content.clone();
        let selected = self.selected_platform;

        div()
            .when(!is_visible, |d| d.invisible())
            .w(px(320.0))
            .rounded(px(12.0))
            .border_1()
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Share"))
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from("✕"))
                    )
            )
            // Preview
            .when_some(content, |container, content| {
                container.child(
                    div()
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .border_1()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_xs()
                                        .child(SharedString::from(content.content_type.label()))
                                )
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(SharedString::from(content.title.clone()))
                        )
                        .child(
                            div()
                                .text_xs()
                                .child(SharedString::from(
                                    if content.description.len() > 80 {
                                        format!("{}...", &content.description[..80])
                                    } else {
                                        content.description.clone()
                                    }
                                ))
                        )
                )
            })
            // Platform buttons
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .children(SharePlatform::all().iter().map(|&platform| {
                        let is_selected = selected == Some(platform);
                        div()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(8.0))
                            .border_1()
                            .cursor_pointer()
                            .text_xs()
                            .when(is_selected, |d| {
                                d.font_weight(gpui::FontWeight::BOLD)
                            })
                            .child(SharedString::from(platform.label()))
                    }))
            )
    }
}
