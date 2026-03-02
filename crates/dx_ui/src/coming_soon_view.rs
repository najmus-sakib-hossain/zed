//! ComingSoonView — Stub for Deep Research & Deep Search features.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComingSoonFeature {
    DeepResearch,
    DeepSearch,
    LiveMode,
    VideoGeneration,
    MusicGeneration,
    ThreeDGeneration,
}

impl ComingSoonFeature {
    pub fn title(&self) -> &'static str {
        match self {
            Self::DeepResearch => "Deep Research",
            Self::DeepSearch => "Deep Search",
            Self::LiveMode => "Live Mode",
            Self::VideoGeneration => "Video Generation",
            Self::MusicGeneration => "Music Generation",
            Self::ThreeDGeneration => "3D Generation",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::DeepResearch => {
                "An advanced research agent that explores topics in depth, \
                 synthesizes sources, and produces structured reports."
            }
            Self::DeepSearch => {
                "Intelligent web search that understands context, \
                 follows links, and aggregates findings in real-time."
            }
            Self::LiveMode => {
                "Live camera/screen sharing with real-time AI commentary \
                 and interactive dialogue about what the AI sees."
            }
            Self::VideoGeneration => {
                "Generate short-form video clips from text prompts \
                 using Runway, Kling, Pika, Luma, and more."
            }
            Self::MusicGeneration => {
                "Create original music tracks from text descriptions \
                 using Suno, Udio, Stability Audio, and more."
            }
            Self::ThreeDGeneration => {
                "Generate 3D models and scenes from text or images \
                 using Meshy, Tripo, and more."
            }
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::DeepResearch => "microscope",
            Self::DeepSearch => "search",
            Self::LiveMode => "video",
            Self::VideoGeneration => "film",
            Self::MusicGeneration => "music",
            Self::ThreeDGeneration => "cube",
        }
    }
}

pub struct ComingSoonView {
    focus_handle: FocusHandle,
    feature: ComingSoonFeature,
    notify_email: Option<String>,
}

impl ComingSoonView {
    pub fn new(feature: ComingSoonFeature, cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            feature,
            notify_email: None,
        }
    }

    pub fn set_feature(&mut self, feature: ComingSoonFeature, cx: &mut ViewContext<Self>) {
        self.feature = feature;
        cx.notify();
    }

    pub fn feature(&self) -> ComingSoonFeature {
        self.feature
    }
}

impl Focusable for ComingSoonView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ComingSoonView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let feature = self.feature;

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(24.0))
            // Icon placeholder
            .child(
                div()
                    .w(px(80.0))
                    .h(px(80.0))
                    .rounded(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(36.0))
                            .child(SharedString::from("🚧"))
                    )
            )
            // Title
            .child(
                div()
                    .text_size(px(28.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(SharedString::from(feature.title()))
            )
            // Description
            .child(
                div()
                    .max_w(px(480.0))
                    .text_center()
                    .text_sm()
                    .child(SharedString::from(feature.description()))
            )
            // Coming soon badge
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(px(20.0))
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from("Coming Soon"))
            )
    }
}
