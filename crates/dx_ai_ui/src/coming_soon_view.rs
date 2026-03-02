//! Coming Soon View — Placeholder for profiles not yet implemented (Part 2).

use gpui::{div, prelude::*, SharedString, Window};

/// A placeholder view shown for AI profiles that are still in development.
pub struct ComingSoonView {
    profile_name: SharedString,
    description: SharedString,
}

impl ComingSoonView {
    pub fn new(profile_name: impl Into<SharedString>, description: impl Into<SharedString>) -> Self {
        Self {
            profile_name: profile_name.into(),
            description: description.into(),
        }
    }
}

impl Render for ComingSoonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(self.profile_name.clone())
            .child(self.description.clone())
            .child(SharedString::from("Coming Soon"))
    }
}
