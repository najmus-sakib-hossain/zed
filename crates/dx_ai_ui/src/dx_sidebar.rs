//! DX Sidebar — Notion-style sidebar with mode sections (Part 3).
//!
//! The sidebar organizes AI functionality by "mood" sections:
//! - Text (Chat, Code, Plan, Study, Search, Research)
//! - Image (Generate, Edit, Upscale)
//! - Audio (TTS, Music, Sound Effects)
//! - Video (Generate, Edit)
//! - Live (Conversation, Avatar)
//! - 3D (Model Generation, Scene)
//! - PDF (Generate, Convert, Sign)
//!
//! Each section expands to show relevant actions and recent sessions.

use dx_core::Mood;
use gpui::{div, prelude::*, SharedString, Window};

/// A section in the DX sidebar corresponding to a mood.
#[derive(Debug, Clone)]
pub struct SidebarSection {
    pub mood: Mood,
    pub label: SharedString,
    pub icon_name: SharedString,
    pub expanded: bool,
    pub action_count: usize,
}

/// The full DX sidebar panel.
pub struct DxSidebar {
    sections: Vec<SidebarSection>,
    active_mood: Mood,
}

impl DxSidebar {
    pub fn new() -> Self {
        let sections = vec![
            SidebarSection {
                mood: Mood::Text,
                label: "Text".into(),
                icon_name: "text".into(),
                expanded: true,
                action_count: 6,
            },
            SidebarSection {
                mood: Mood::Image,
                label: "Image".into(),
                icon_name: "image".into(),
                expanded: false,
                action_count: 4,
            },
            SidebarSection {
                mood: Mood::Audio,
                label: "Audio".into(),
                icon_name: "audio".into(),
                expanded: false,
                action_count: 3,
            },
            SidebarSection {
                mood: Mood::Video,
                label: "Video".into(),
                icon_name: "video".into(),
                expanded: false,
                action_count: 2,
            },
            SidebarSection {
                mood: Mood::Live,
                label: "Live".into(),
                icon_name: "live".into(),
                expanded: false,
                action_count: 2,
            },
            SidebarSection {
                mood: Mood::ThreeD,
                label: "3D / AR / VR".into(),
                icon_name: "cube".into(),
                expanded: false,
                action_count: 3,
            },
            SidebarSection {
                mood: Mood::Pdf,
                label: "PDF & Docs".into(),
                icon_name: "file-text".into(),
                expanded: false,
                action_count: 3,
            },
        ];

        Self {
            sections,
            active_mood: Mood::Text,
        }
    }

    pub fn toggle_section(&mut self, mood: Mood, cx: &mut Context<Self>) {
        if let Some(section) = self.sections.iter_mut().find(|s| s.mood == mood) {
            section.expanded = !section.expanded;
        }
        cx.notify();
    }

    pub fn set_active_mood(&mut self, mood: Mood, cx: &mut Context<Self>) {
        self.active_mood = mood;
        cx.notify();
    }

    pub fn active_mood(&self) -> Mood {
        self.active_mood
    }
}

impl Render for DxSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().flex().flex_col().w_56().h_full();

        for section in &self.sections {
            let is_active = section.mood == self.active_mood;
            let prefix: SharedString = if is_active { "▸ ".into() } else { "  ".into() };
            let label: SharedString =
                format!("{}{} ({})", prefix, section.label, section.action_count).into();
            root = root.child(label);
        }

        root
    }
}
