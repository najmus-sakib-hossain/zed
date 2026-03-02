//! SessionHistoryRail — Right-side rail showing session history grouped by date.
//!
//! Part 5: Sessions are grouped as Today / Yesterday / This Week / Older.
//! Each entry shows a short title, timestamp, and mood icon.

use dx_core::session::{group_sessions_by_date, SessionEntry, SessionGroup};
use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone)]
pub enum SessionHistoryEvent {
    SessionSelected(String),
    SessionDeleted(String),
    NewSessionRequested,
}

pub struct SessionHistoryRail {
    focus_handle: FocusHandle,
    sessions: Vec<SessionEntry>,
    selected_session_id: Option<String>,
    is_expanded: bool,
}

impl SessionHistoryRail {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            sessions: Vec::new(),
            selected_session_id: None,
            is_expanded: true,
        }
    }

    pub fn set_sessions(&mut self, sessions: Vec<SessionEntry>, cx: &mut ViewContext<Self>) {
        self.sessions = sessions;
        cx.notify();
    }

    pub fn select_session(&mut self, id: &str, cx: &mut ViewContext<Self>) {
        self.selected_session_id = Some(id.to_string());
        cx.emit(SessionHistoryEvent::SessionSelected(id.to_string()));
        cx.notify();
    }

    pub fn toggle_expanded(&mut self, cx: &mut ViewContext<Self>) {
        self.is_expanded = !self.is_expanded;
        cx.notify();
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    fn grouped_sessions(&self) -> Vec<SessionGroup> {
        group_sessions_by_date(&self.sessions)
    }

    fn group_label(group: &SessionGroup) -> &'static str {
        match group {
            SessionGroup::Today(_) => "Today",
            SessionGroup::Yesterday(_) => "Yesterday",
            SessionGroup::ThisWeek(_) => "This Week",
            SessionGroup::Older(_) => "Older",
        }
    }

    fn group_sessions(group: &SessionGroup) -> &[SessionEntry] {
        match group {
            SessionGroup::Today(s)
            | SessionGroup::Yesterday(s)
            | SessionGroup::ThisWeek(s)
            | SessionGroup::Older(s) => s,
        }
    }
}

impl gpui::EventEmitter<SessionHistoryEvent> for SessionHistoryRail {}

impl Focusable for SessionHistoryRail {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SessionHistoryRail {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let groups = self.grouped_sessions();
        let selected_id = self.selected_session_id.clone();
        let is_expanded = self.is_expanded;

        let rail_width = if is_expanded { px(260.0) } else { px(40.0) };

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(rail_width)
            .border_l_1()
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p(px(8.0))
                    .border_b_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .when(!is_expanded, |d| d.invisible())
                            .child(SharedString::from("History"))
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from(
                                if is_expanded { "◀" } else { "▶" }
                            ))
                    )
            )
            // Session groups
            .when(is_expanded, |container| {
                container.child(
                    div()
                        .flex_1()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .p(px(8.0))
                        .children(groups.iter().map(|group| {
                            let sessions = Self::group_sessions(group);
                            let label = Self::group_label(group);
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                // Group header
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .pb(px(2.0))
                                        .child(SharedString::from(label))
                                )
                                // Session entries
                                .children(sessions.iter().map(|session| {
                                    let is_selected = selected_id
                                        .as_ref()
                                        .map_or(false, |id| id == &session.id);
                                    div()
                                        .px(px(8.0))
                                        .py(px(6.0))
                                        .rounded(px(6.0))
                                        .cursor_pointer()
                                        .when(is_selected, |d| {
                                            d.font_weight(gpui::FontWeight::MEDIUM)
                                        })
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_sm()
                                                .child(SharedString::from(
                                                    session.title.clone(),
                                                ))
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .child(SharedString::from(
                                                    session.mood_label(),
                                                ))
                                        )
                                }))
                        }))
                )
            })
            // New session button at bottom
            .when(is_expanded, |container| {
                container.child(
                    div()
                        .p(px(8.0))
                        .border_t_1()
                        .child(
                            div()
                                .w_full()
                                .py(px(6.0))
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .text_xs()
                                .text_center()
                                .child(SharedString::from("+ New Session"))
                        )
                )
            })
    }
}
