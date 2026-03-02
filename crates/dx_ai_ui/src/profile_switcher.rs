//! Profile Switcher — Quick profile selection (Part 2).
//!
//! A dropdown/popover that lets the user switch between AI profiles:
//! Chat, Code, Plan, Study, Deep Research, Search.
//!
//! Each profile changes the panel view and default model selection.

use dx_core::AiProfile;
use gpui::{div, prelude::*, SharedString, Window};

/// The profile switcher component.
pub struct ProfileSwitcher {
    active: AiProfile,
    profiles: Vec<ProfileOption>,
}

#[derive(Debug, Clone)]
pub struct ProfileOption {
    pub profile: AiProfile,
    pub label: SharedString,
    pub description: SharedString,
    pub icon_name: SharedString,
    pub keyboard_shortcut: Option<SharedString>,
}

impl ProfileSwitcher {
    pub fn new() -> Self {
        let profiles = vec![
            ProfileOption {
                profile: AiProfile::Chat,
                label: "Chat".into(),
                description: "General conversation & Q&A".into(),
                icon_name: "message-circle".into(),
                keyboard_shortcut: Some("Alt+1".into()),
            },
            ProfileOption {
                profile: AiProfile::Code,
                label: "Code".into(),
                description: "Agent-driven development".into(),
                icon_name: "code".into(),
                keyboard_shortcut: Some("Alt+2".into()),
            },
            ProfileOption {
                profile: AiProfile::Plan,
                label: "Plan".into(),
                description: "Multi-step task planning".into(),
                icon_name: "list-checks".into(),
                keyboard_shortcut: Some("Alt+3".into()),
            },
            ProfileOption {
                profile: AiProfile::Study,
                label: "Study".into(),
                description: "Learning & flashcards".into(),
                icon_name: "graduation-cap".into(),
                keyboard_shortcut: Some("Alt+4".into()),
            },
            ProfileOption {
                profile: AiProfile::DeepResearch,
                label: "Research".into(),
                description: "Exhaustive multi-source analysis".into(),
                icon_name: "microscope".into(),
                keyboard_shortcut: Some("Alt+5".into()),
            },
            ProfileOption {
                profile: AiProfile::Search,
                label: "Search".into(),
                description: "Web + codebase search".into(),
                icon_name: "search".into(),
                keyboard_shortcut: Some("Alt+6".into()),
            },
        ];

        Self {
            active: AiProfile::Chat,
            profiles,
        }
    }

    pub fn set_active(&mut self, profile: AiProfile, cx: &mut Context<Self>) {
        self.active = profile;
        cx.notify();
    }

    pub fn active(&self) -> AiProfile {
        self.active
    }
}

impl Render for ProfileSwitcher {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().flex().flex_col().gap_1();

        for option in &self.profiles {
            let is_active = option.profile == self.active;
            let prefix: SharedString = if is_active { "● ".into() } else { "○ ".into() };
            let label: SharedString = format!("{}{}", prefix, option.label).into();
            root = root.child(label);
        }

        root
    }
}
