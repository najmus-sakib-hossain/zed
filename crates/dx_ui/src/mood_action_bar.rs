//! MoodActionBar — Mood toggle buttons that swap visible input actions per mood.
//!
//! Part 4: Each mood (Chat, Create, Code, Learn, Play, Explore, Focus) exposes
//! a different set of quick-actions in the input area.

use dx_core::mood::{Mood, MoodActionSet};
use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone)]
pub enum MoodActionBarEvent {
    MoodChanged(Mood),
    ActionTriggered { mood: Mood, action: String },
}

pub struct MoodActionBar {
    focus_handle: FocusHandle,
    current_mood: Mood,
    available_moods: Vec<Mood>,
}

impl MoodActionBar {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            current_mood: Mood::Chat,
            available_moods: vec![
                Mood::Chat,
                Mood::Create,
                Mood::Code,
                Mood::Learn,
                Mood::Play,
                Mood::Explore,
                Mood::Focus,
            ],
        }
    }

    pub fn current_mood(&self) -> Mood {
        self.current_mood
    }

    pub fn set_mood(&mut self, mood: Mood, cx: &mut ViewContext<Self>) {
        self.current_mood = mood;
        cx.emit(MoodActionBarEvent::MoodChanged(mood));
        cx.notify();
    }

    pub fn current_action_set(&self) -> MoodActionSet {
        MoodActionSet::for_mood(self.current_mood)
    }

    fn mood_label(mood: Mood) -> &'static str {
        match mood {
            Mood::Chat => "💬 Chat",
            Mood::Create => "🎨 Create",
            Mood::Code => "💻 Code",
            Mood::Learn => "📚 Learn",
            Mood::Play => "🎮 Play",
            Mood::Explore => "🔭 Explore",
            Mood::Focus => "🎯 Focus",
        }
    }
}

impl gpui::EventEmitter<MoodActionBarEvent> for MoodActionBar {}

impl Focusable for MoodActionBar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MoodActionBar {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let current = self.current_mood;
        let moods = self.available_moods.clone();
        let action_set = self.current_action_set();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Mood toggle row
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .px(px(8.0))
                    .children(moods.iter().map(|&mood| {
                        let is_active = mood == current;
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(8.0))
                            .cursor_pointer()
                            .text_xs()
                            .when(is_active, |d| {
                                d.font_weight(gpui::FontWeight::BOLD)
                            })
                            .child(SharedString::from(Self::mood_label(mood)))
                    }))
            )
            // Action buttons for current mood
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(6.0))
                    .px(px(8.0))
                    .children(action_set.actions.iter().map(|action| {
                        div()
                            .px(px(12.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .border_1()
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from(action.clone()))
                    }))
            )
    }
}
