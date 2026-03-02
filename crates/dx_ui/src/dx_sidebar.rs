//! DxSidebar — Notion-style left sidebar with Home / Search / New buttons,
//! page tree, and dot-nav workspace switcher.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

/// An entry in the sidebar page tree.
#[derive(Debug, Clone)]
pub struct SidebarPage {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub children: Vec<SidebarPage>,
    pub is_expanded: bool,
}

/// A workspace dot in the bottom-left workspace switcher.
#[derive(Debug, Clone)]
pub struct WorkspaceDot {
    pub id: String,
    pub label: String,
    pub color: u32,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub enum SidebarEvent {
    PageSelected(String),
    WorkspaceSelected(String),
    NewPageRequested,
    SearchRequested,
    HomeRequested,
}

pub struct DxSidebar {
    focus_handle: FocusHandle,
    pages: Vec<SidebarPage>,
    workspaces: Vec<WorkspaceDot>,
    selected_page_id: Option<String>,
    is_collapsed: bool,
}

impl DxSidebar {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            pages: Vec::new(),
            workspaces: vec![
                WorkspaceDot {
                    id: "default".into(),
                    label: "Default".into(),
                    color: 0x4A9EFF,
                    is_active: true,
                },
            ],
            selected_page_id: None,
            is_collapsed: false,
        }
    }

    pub fn add_page(&mut self, page: SidebarPage, cx: &mut ViewContext<Self>) {
        self.pages.push(page);
        cx.notify();
    }

    pub fn set_pages(&mut self, pages: Vec<SidebarPage>, cx: &mut ViewContext<Self>) {
        self.pages = pages;
        cx.notify();
    }

    pub fn toggle_collapsed(&mut self, cx: &mut ViewContext<Self>) {
        self.is_collapsed = !self.is_collapsed;
        cx.notify();
    }

    pub fn is_collapsed(&self) -> bool {
        self.is_collapsed
    }

    pub fn select_page(&mut self, page_id: &str, cx: &mut ViewContext<Self>) {
        self.selected_page_id = Some(page_id.to_string());
        cx.emit(SidebarEvent::PageSelected(page_id.to_string()));
        cx.notify();
    }

    pub fn add_workspace(&mut self, workspace: WorkspaceDot, cx: &mut ViewContext<Self>) {
        self.workspaces.push(workspace);
        cx.notify();
    }

    fn render_page_tree_item(
        &self,
        page: &SidebarPage,
        depth: usize,
    ) -> gpui::Div {
        let is_selected = self
            .selected_page_id
            .as_ref()
            .map_or(false, |id| id == &page.id);

        let mut item = div()
            .pl(px((depth as f32) * 16.0 + 8.0))
            .pr(px(8.0))
            .py(px(4.0))
            .rounded(px(4.0))
            .cursor_pointer()
            .text_sm()
            .when(is_selected, |d| d.font_weight(gpui::FontWeight::SEMIBOLD))
            .child(SharedString::from(page.title.clone()));

        if page.is_expanded {
            for child in &page.children {
                item = item.child(self.render_page_tree_item(child, depth + 1));
            }
        }

        item
    }
}

impl gpui::EventEmitter<SidebarEvent> for DxSidebar {}

impl Focusable for DxSidebar {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DxSidebar {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let sidebar_width = if self.is_collapsed {
            px(48.0)
        } else {
            px(260.0)
        };

        let pages = self.pages.clone();
        let workspaces = self.workspaces.clone();

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(sidebar_width)
            .border_r_1()
            // Top action buttons: Home / Search / New
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .p(px(8.0))
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from("🏠 Home"))
                    )
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from("🔍 Search"))
                    )
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .text_xs()
                            .child(SharedString::from("➕ New"))
                    )
            )
            // Page tree
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .py(px(4.0))
                    .children(pages.iter().map(|p| self.render_page_tree_item(p, 0)))
            )
            // Workspace dot-nav switcher (bottom)
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .p(px(8.0))
                    .border_t_1()
                    .children(workspaces.iter().map(|ws| {
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded_full()
                            .cursor_pointer()
                            .when(ws.is_active, |d| {
                                d.border_1()
                            })
                    }))
            )
    }
}
