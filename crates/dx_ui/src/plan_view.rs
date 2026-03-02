//! PlanView — planning mode with task breakdown, roadmaps, and todo generation.

use gpui::{
    div, prelude::*, px, AnyElement, FocusHandle, Focusable, ParentElement, Render,
    SharedString, Styled, ViewContext, VisualContext, WindowContext,
};

/// A single plan item in the generated plan.
#[derive(Debug, Clone)]
pub struct PlanItem {
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub children: Vec<PlanItem>,
}

pub struct PlanView {
    focus_handle: FocusHandle,
    plan_title: String,
    items: Vec<PlanItem>,
    is_generating: bool,
}

impl PlanView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            plan_title: String::new(),
            items: Vec::new(),
            is_generating: false,
        }
    }

    pub fn set_plan(&mut self, title: String, items: Vec<PlanItem>, cx: &mut ViewContext<Self>) {
        self.plan_title = title;
        self.items = items;
        self.is_generating = false;
        cx.notify();
    }

    pub fn set_generating(&mut self, generating: bool, cx: &mut ViewContext<Self>) {
        self.is_generating = generating;
        cx.notify();
    }

    pub fn toggle_item(&mut self, index: usize, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.items.get_mut(index) {
            item.completed = !item.completed;
            cx.notify();
        }
    }

    fn render_plan_item(item: &PlanItem, depth: usize) -> AnyElement {
        let indent = depth as f32 * 20.0;
        let title = item.title.clone();
        let completed = item.completed;

        div()
            .pl(px(indent))
            .py(px(4.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .size(px(16.0))
                    .rounded(px(3.0))
                    .border_1()
                    .when(completed, |el| el.bg(gpui::rgb(0x22c55e)))
            )
            .child(
                div()
                    .when(completed, |el| el.line_through())
                    .child(SharedString::from(title))
            )
            .into_any_element()
    }
}

impl Focusable for PlanView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PlanView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let items = self.items.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .p(px(24.0))
            .gap(px(12.0))
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(SharedString::from(
                        if self.plan_title.is_empty() {
                            "Plan Mode".to_string()
                        } else {
                            self.plan_title.clone()
                        }
                    ))
            )
            .when(self.is_generating, |el| {
                el.child(
                    div()
                        .py(px(8.0))
                        .child(SharedString::from("Generating plan..."))
                )
            })
            .children(
                items.iter().map(|item| Self::render_plan_item(item, 0))
            )
    }
}
