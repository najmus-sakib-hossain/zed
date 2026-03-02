//! Profile switcher — six AI profile entries with distinct icons and keyboard shortcuts.

use dx_core::AiProfile;
use gpui::{
    div, prelude::*, px, rems, AnyElement, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    ParentElement, Render, SharedString, Styled, ViewContext, VisualContext, WindowContext,
};

pub struct ProfileSwitcher {
    active_profile: AiProfile,
    focus_handle: FocusHandle,
}

impl ProfileSwitcher {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            active_profile: AiProfile::Chat,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn active_profile(&self) -> AiProfile {
        self.active_profile
    }

    pub fn set_active_profile(&mut self, profile: AiProfile, cx: &mut ViewContext<Self>) {
        self.active_profile = profile;
        cx.emit(ProfileSwitchEvent { profile });
        cx.notify();
    }
}

pub struct ProfileSwitchEvent {
    pub profile: AiProfile,
}

impl EventEmitter<ProfileSwitchEvent> for ProfileSwitcher {}

impl Focusable for ProfileSwitcher {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProfileSwitcher {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let active = self.active_profile;

        div()
            .flex()
            .gap(px(4.0))
            .p(px(4.0))
            .rounded(px(8.0))
            .children(AiProfile::all().iter().map(|profile| {
                let is_active = *profile == active;
                let profile = *profile;

                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .when(is_active, |el| {
                        el.bg(gpui::rgb(0x3b82f6))
                    })
                    .child(SharedString::from(profile.display_name()))
            }))
    }
}
