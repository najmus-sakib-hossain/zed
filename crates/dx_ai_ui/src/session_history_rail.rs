//! Session History Rail — Chronological session groups (Part 5).
//!
//! Displays past AI sessions grouped by date (Today, Yesterday, This Week,
//! This Month, Older) in a vertical rail on the sidebar edge.
//!
//! Uses `dx_core::session::group_sessions_by_date()`.

use dx_core::session::{group_sessions_by_date, SessionEntry, SessionGroup};
use gpui::{div, prelude::*, SharedString, Window};

/// The session history rail — a narrow vertical list of past sessions.
pub struct SessionHistoryRail {
    groups: Vec<SessionGroup>,
    selected_session_id: Option<String>,
}

impl SessionHistoryRail {
    pub fn new(sessions: Vec<SessionEntry>) -> Self {
        let groups = group_sessions_by_date(sessions);
        Self {
            groups,
            selected_session_id: None,
        }
    }

    pub fn refresh(&mut self, sessions: Vec<SessionEntry>, cx: &mut Context<Self>) {
        self.groups = group_sessions_by_date(sessions);
        cx.notify();
    }

    pub fn select_session(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected_session_id = Some(id);
        cx.notify();
    }

    pub fn total_sessions(&self) -> usize {
        self.groups.iter().map(|g| g.sessions.len()).sum()
    }
}

impl Render for SessionHistoryRail {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut rail = div().flex().flex_col().w_48();

        for group in &self.groups {
            let header: SharedString =
                format!("{} ({})", group.label, group.sessions.len()).into();
            rail = rail.child(header);

            for session in &group.sessions {
                let is_selected = self
                    .selected_session_id
                    .as_ref()
                    .map_or(false, |id| id == &session.id);
                let prefix = if is_selected { "▸ " } else { "  " };
                let entry: SharedString = format!("{}{}", prefix, session.title).into();
                rail = rail.child(entry);
            }
        }

        rail
    }
}
