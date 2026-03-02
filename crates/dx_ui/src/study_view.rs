//! StudyView — 3-column layout: sources / chat / studio workspace.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext, VisualContext, WindowContext,
};

/// A source reference in the study panel.
#[derive(Debug, Clone)]
pub struct StudySource {
    pub title: String,
    pub url: Option<String>,
    pub snippet: String,
    pub relevance_score: f64,
}

/// A note in the studio panel.
#[derive(Debug, Clone)]
pub struct StudyNote {
    pub id: String,
    pub content: String,
    pub created_at: std::time::SystemTime,
}

pub struct StudyView {
    focus_handle: FocusHandle,
    sources: Vec<StudySource>,
    notes: Vec<StudyNote>,
    active_column: StudyColumn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyColumn {
    Sources,
    Chat,
    Studio,
}

impl StudyView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            sources: Vec::new(),
            notes: Vec::new(),
            active_column: StudyColumn::Chat,
        }
    }

    pub fn add_source(&mut self, source: StudySource, cx: &mut ViewContext<Self>) {
        self.sources.push(source);
        cx.notify();
    }

    pub fn add_note(&mut self, note: StudyNote, cx: &mut ViewContext<Self>) {
        self.notes.push(note);
        cx.notify();
    }

    pub fn set_active_column(&mut self, column: StudyColumn, cx: &mut ViewContext<Self>) {
        self.active_column = column;
        cx.notify();
    }

    pub fn clear_sources(&mut self, cx: &mut ViewContext<Self>) {
        self.sources.clear();
        cx.notify();
    }
}

impl Focusable for StudyView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StudyView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let sources = self.sources.clone();
        let notes = self.notes.clone();

        div()
            .flex()
            .size_full()
            .gap(px(1.0))
            // Sources column (left)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(280.0))
                    .border_r_1()
                    .p(px(12.0))
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Sources"))
                    )
                    .children(sources.iter().map(|s| {
                        div()
                            .p(px(8.0))
                            .rounded(px(6.0))
                            .border_1()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(SharedString::from(s.title.clone()))
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(SharedString::from(
                                        if s.snippet.len() > 120 {
                                            format!("{}...", &s.snippet[..120])
                                        } else {
                                            s.snippet.clone()
                                        }
                                    ))
                            )
                    }))
            )
            // Chat column (center)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p(px(16.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Study Chat"))
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(SharedString::from("Ask questions about your sources..."))
                    )
            )
            // Studio column (right)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(300.0))
                    .border_l_1()
                    .p(px(12.0))
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from("Studio"))
                    )
                    .children(notes.iter().map(|n| {
                        div()
                            .p(px(8.0))
                            .rounded(px(6.0))
                            .border_1()
                            .child(SharedString::from(n.content.clone()))
                    }))
            )
    }
}
