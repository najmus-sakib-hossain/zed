use gpui::{prelude::*, Context, IntoElement, Render, Window};
use ui::{h_flex, prelude::*, Icon, IconName, IconSize, Tooltip};
use workspace::{ItemHandle, StatusItemView};

/// A mood/content-type icon that appears in the status bar.
struct MoodIcon {
    icon: IconName,
    label: &'static str,
}

const MOOD_ICONS: &[MoodIcon] = &[
    MoodIcon {
        icon: IconName::MoodText,
        label: "Text",
    },
    MoodIcon {
        icon: IconName::MoodImage,
        label: "Image",
    },
    MoodIcon {
        icon: IconName::MoodVideo,
        label: "Video",
    },
    MoodIcon {
        icon: IconName::MoodAudio,
        label: "Audio",
    },
    MoodIcon {
        icon: IconName::Mood3d,
        label: "3D/AR/VR",
    },
    MoodIcon {
        icon: IconName::MoodDocument,
        label: "PDF, DOCS",
    },
    MoodIcon {
        icon: IconName::MoodLive,
        label: "Live",
    },
];

pub struct MoodSelector;

impl MoodSelector {
    pub fn new() -> Self {
        Self
    }
}

impl Render for MoodSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_0p5()
            .px_1()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().colors().border)
            .children(MOOD_ICONS.iter().map(|mood| {
                div()
                    .id(SharedString::from(mood.label))
                    .p_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|style| {
                        style.bg(cx.theme().colors().ghost_element_hover)
                    })
                    .tooltip(Tooltip::text(mood.label))
                    .child(
                        Icon::new(mood.icon)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    )
            }))
    }
}

impl StatusItemView for MoodSelector {
    fn set_active_pane_item(
        &mut self,
        _active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // Mood selector is always visible, no active pane item dependency.
    }
}
