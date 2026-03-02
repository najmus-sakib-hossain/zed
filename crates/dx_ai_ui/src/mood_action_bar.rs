//! Mood Action Bar — Contextual buttons per active mood (Part 4).
//!
//! Displays a horizontal bar of relevant actions based on the current mood:
//! - Text mood → Chat, Code, Plan, Study, Search, Research
//! - Image mood → Generate, Edit, Upscale, Variations
//! - etc.
//!
//! Uses `dx_core::mood::actions_for_mood()` to determine available actions.

use dx_core::{Mood, MoodActionSet};
use gpui::{div, prelude::*, SharedString, Window};

/// An action button in the mood bar.
#[derive(Debug, Clone)]
pub struct MoodAction {
    pub label: SharedString,
    pub icon_name: SharedString,
    pub is_primary: bool,
}

/// The mood action bar — shows contextual actions for the active mood.
pub struct MoodActionBar {
    mood: Mood,
    actions: Vec<MoodAction>,
}

impl MoodActionBar {
    pub fn new(mood: Mood) -> Self {
        let actions = Self::actions_for(mood);
        Self { mood, actions }
    }

    pub fn set_mood(&mut self, mood: Mood, cx: &mut Context<Self>) {
        self.mood = mood;
        self.actions = Self::actions_for(mood);
        cx.notify();
    }

    fn actions_for(mood: Mood) -> Vec<MoodAction> {
        let action_set = dx_core::actions_for_mood(mood);
        action_set
            .primary
            .iter()
            .map(|name| MoodAction {
                label: (*name).into(),
                icon_name: "sparkle".into(),
                is_primary: true,
            })
            .chain(action_set.secondary.iter().map(|name| MoodAction {
                label: (*name).into(),
                icon_name: "circle".into(),
                is_primary: false,
            }))
            .collect()
    }
}

impl Render for MoodActionBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut bar = div().flex().flex_row().gap_2();

        for action in &self.actions {
            bar = bar.child(action.label.clone());
        }

        bar
    }
}
