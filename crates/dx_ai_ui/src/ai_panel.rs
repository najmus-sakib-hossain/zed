//! AI Panel — Main entry point for the DX AI experience (Part 2).
//!
//! Contains the profile-dependent views:
//! - Chat profile ↔ existing `AssistantPanel`
//! - Code profile ↔ existing `AgentPanel`
//! - Plan profile ↔ `PlanView`
//! - Study profile ↔ `StudyView`
//! - Everything else ↔ `ComingSoonView`

use dx_core::AiProfile;
use gpui::{div, prelude::*, Entity, SharedString, Window};

/// The main DX AI panel that switches views based on the active profile.
pub struct DxAiPanel {
    active_profile: AiProfile,
}

impl DxAiPanel {
    pub fn new() -> Self {
        Self {
            active_profile: AiProfile::Chat,
        }
    }

    pub fn set_profile(&mut self, profile: AiProfile, cx: &mut Context<Self>) {
        self.active_profile = profile;
        cx.notify();
    }

    pub fn active_profile(&self) -> AiProfile {
        self.active_profile
    }
}

impl Render for DxAiPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label: SharedString = match self.active_profile {
            AiProfile::Chat => "Chat — LLM conversation".into(),
            AiProfile::Code => "Code — Agent-driven development".into(),
            AiProfile::Plan => "Plan — Multi-step planner".into(),
            AiProfile::Study => "Study — Learning assistant".into(),
            AiProfile::DeepResearch => "Deep Research — Exhaustive analysis".into(),
            AiProfile::Search => "Search — Web + codebase search".into(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(label)
    }
}
