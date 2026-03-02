//! GPUI Workflow Panel
//!
//! This module provides the GPUI-native UI for the n8n workflow engine integration.
//! It renders a workflow panel in the Zed dock that allows users to:
//! - View available workflow templates
//! - Execute workflows triggered by the AI agent
//! - Monitor workflow execution status
//! - View execution history and results

use crate::hybrid_executor::HybridWorkflowExecutor;
use crate::protocol::{WorkflowBuilder, WorkflowDefinition, WorkflowResult};
use crate::sidecar::{N8nSidecar, SidecarConfig};
use crate::workflow_router::AiWorkflowRouter;
use anyhow::Result;
use gpui::{
    actions, div, prelude::*, px, App, AppContext, Context, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Task, WeakEntity, Window,
};
use parking_lot::Mutex;
use std::sync::Arc;

// Define actions for the workflow panel
actions!(
    workflow_panel,
    [
        ToggleFocus,
        ExecuteWorkflow,
        RefreshStatus,
        ClearHistory,
    ]
);

/// Initialize the workflow panel subsystem
pub fn init(cx: &mut AppContext) {
    cx.observe_new(WorkflowPanel::register).detach();
}

/// A workflow execution record for the history
#[derive(Clone, Debug)]
pub struct ExecutionRecord {
    pub id: String,
    pub workflow_name: String,
    pub status: String,
    pub timestamp: std::time::SystemTime,
    pub execution_time_ms: u64,
    pub result: Option<WorkflowResult>,
}

/// The main workflow panel view
pub struct WorkflowPanel {
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,

    /// The n8n sidecar (if started)
    sidecar: Option<Arc<N8nSidecar>>,

    /// The AI workflow router
    router: Option<Arc<Mutex<AiWorkflowRouter>>>,

    /// The hybrid executor
    executor: Option<Arc<HybridWorkflowExecutor>>,

    /// Panel width
    width: f32,

    /// Whether the panel is visible
    visible: bool,

    /// Execution history
    history: Vec<ExecutionRecord>,

    /// Currently selected workflow template
    selected_template: Option<String>,

    /// Available workflow templates
    templates: Vec<String>,

    /// Status message
    status_message: Option<String>,

    /// Whether the engine is running
    engine_running: bool,
}

impl WorkflowPanel {
    /// Create a new workflow panel
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Initialize router with default templates
        let router = AiWorkflowRouter::new().ok();
        let templates = router
            .as_ref()
            .map(|r| r.list_intents().iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Self {
            focus_handle,
            sidecar: None,
            router: router.map(|r| Arc::new(Mutex::new(r))),
            executor: None,
            width: 300.0,
            visible: true,
            history: Vec::new(),
            selected_template: None,
            templates,
            status_message: Some("Engine not started".to_string()),
            engine_running: false,
        }
    }

    /// Register the panel with a workspace
    fn register(
        _workspace: &mut gpui::Window,
        _cx: &mut App,
    ) {
        // Registration logic would go here
        // This is called when a new workspace is created
    }

    /// Start the n8n engine
    pub fn start_engine(&mut self, cx: &mut Context<Self>) {
        if self.engine_running {
            self.status_message = Some("Engine already running".to_string());
            cx.notify();
            return;
        }

        self.status_message = Some("Starting engine...".to_string());
        cx.notify();

        let config = SidecarConfig::default();

        cx.spawn(|this, mut cx| async move {
            match N8nSidecar::spawn(config).await {
                Ok(sidecar) => {
                    let sidecar = Arc::new(sidecar);
                    let executor = Arc::new(HybridWorkflowExecutor::new(sidecar.clone()));

                    cx.update(|cx| {
                        this.update(cx, |panel, cx| {
                            panel.sidecar = Some(sidecar);
                            panel.executor = Some(executor);
                            panel.engine_running = true;
                            panel.status_message = Some("Engine running".to_string());
                            cx.notify();
                        })
                        .ok();
                    })
                    .ok();
                }
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |panel, cx| {
                            panel.status_message =
                                Some(format!("Failed to start engine: {}", e));
                            cx.notify();
                        })
                        .ok();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// Execute a workflow by intent
    pub fn execute_workflow(
        &mut self,
        intent: &str,
        parameters: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        let Some(router) = &self.router else {
            self.status_message = Some("Router not initialized".to_string());
            cx.notify();
            return;
        };

        let Some(executor) = &self.executor else {
            self.status_message = Some("Executor not initialized".to_string());
            cx.notify();
            return;
        };

        // Route the decision
        let decision = serde_json::json!({
            "intent": intent,
            "parameters": parameters,
            "context": {}
        });

        let route = {
            let router_guard = router.lock();
            router_guard.route_ai_decision(decision)
        };

        match route {
            Ok(route) => {
                let action = route
                    .get("action")
                    .and_then(|a| a.as_str())
                    .unwrap_or("reject");

                if action == "execute" {
                    // Build workflow from route
                    if let Ok(workflow) = serde_json::from_value::<WorkflowDefinition>(
                        route.get("workflow").cloned().unwrap_or_default(),
                    ) {
                        let workflow_name = workflow.name.clone();
                        let execution_id = uuid::Uuid::new_v4().to_string();

                        let input = route
                            .get("input_data")
                            .cloned()
                            .unwrap_or(serde_json::json!({}));

                        // Add to history as pending
                        self.history.push(ExecutionRecord {
                            id: execution_id.clone(),
                            workflow_name: workflow_name.clone(),
                            status: "running".to_string(),
                            timestamp: std::time::SystemTime::now(),
                            execution_time_ms: 0,
                            result: None,
                        });

                        self.status_message =
                            Some(format!("Executing: {}", workflow_name));
                        cx.notify();

                        // Execute asynchronously
                        let executor = executor.clone();
                        let exec_id = execution_id.clone();

                        cx.spawn(|this, mut cx| async move {
                            let result = executor.execute(&workflow, input).await;

                            cx.update(|cx| {
                                this.update(cx, |panel, cx| {
                                    // Update history entry
                                    if let Some(entry) = panel
                                        .history
                                        .iter_mut()
                                        .find(|e| e.id == exec_id)
                                    {
                                        match result {
                                            Ok(res) => {
                                                entry.status = res.status.clone();
                                                entry.execution_time_ms =
                                                    res.execution_time_ms;
                                                entry.result = Some(res);
                                                panel.status_message = Some(format!(
                                                    "Completed: {}",
                                                    entry.workflow_name
                                                ));
                                            }
                                            Err(e) => {
                                                entry.status = "error".to_string();
                                                panel.status_message =
                                                    Some(format!("Error: {}", e));
                                            }
                                        }
                                    }
                                    cx.notify();
                                })
                                .ok();
                            })
                            .ok();
                        })
                        .detach();
                    }
                } else {
                    let reason = route
                        .get("reason")
                        .and_then(|r| r.as_str())
                        .unwrap_or("Unknown");
                    self.status_message =
                        Some(format!("Rejected: {}", reason));
                    cx.notify();
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Routing error: {}", e));
                cx.notify();
            }
        }
    }

    /// Clear execution history
    pub fn clear_history(&mut self, cx: &mut Context<Self>) {
        self.history.clear();
        self.status_message = Some("History cleared".to_string());
        cx.notify();
    }

    /// Get the list of available templates
    pub fn list_templates(&self) -> &[String] {
        &self.templates
    }

    /// Select a template
    pub fn select_template(&mut self, template: &str, cx: &mut Context<Self>) {
        self.selected_template = Some(template.to_string());
        cx.notify();
    }

    /// Render a history entry
    fn render_history_entry(&self, entry: &ExecutionRecord) -> impl IntoElement {
        let status_color = match entry.status.as_str() {
            "success" => gpui::rgb(0x4ade80), // Green
            "error" => gpui::rgb(0xf87171),   // Red
            "running" => gpui::rgb(0xfbbf24), // Yellow
            _ => gpui::rgb(0x9ca3af),         // Gray
        };

        div()
            .px(px(8.0))
            .py(px(4.0))
            .mb(px(4.0))
            .rounded(px(4.0))
            .bg(gpui::rgb(0x1f2937))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(gpui::rgb(0xf9fafb))
                            .child(entry.workflow_name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(entry.status.clone()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(0x6b7280))
                    .child(format!("{}ms", entry.execution_time_ms)),
            )
    }

    /// Render a template button
    fn render_template_button(&self, template: &str) -> impl IntoElement {
        let is_selected = self.selected_template.as_deref() == Some(template);
        let bg_color = if is_selected {
            gpui::rgb(0x3b82f6) // Blue when selected
        } else {
            gpui::rgb(0x374151)
        };

        div()
            .id(SharedString::from(format!("template-{}", template)))
            .px(px(12.0))
            .py(px(8.0))
            .mb(px(4.0))
            .rounded(px(4.0))
            .bg(bg_color)
            .cursor_pointer()
            .child(
                div()
                    .text_sm()
                    .text_color(gpui::rgb(0xf9fafb))
                    .child(template.to_string()),
            )
    }
}

impl EventEmitter<()> for WorkflowPanel {}

impl Focusable for WorkflowPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WorkflowPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self
            .status_message
            .clone()
            .unwrap_or_else(|| "Ready".to_string());

        let engine_status = if self.engine_running {
            "🟢 Running"
        } else {
            "🔴 Stopped"
        };

        div()
            .id("workflow-panel")
            .size_full()
            .flex()
            .flex_col()
            .bg(gpui::rgb(0x111827))
            .text_color(gpui::rgb(0xf9fafb))
            // Header
            .child(
                div()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(gpui::rgb(0x374151))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("🔄 Workflow Engine"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(gpui::rgb(0x9ca3af))
                                    .child(engine_status),
                            ),
                    ),
            )
            // Status bar
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(gpui::rgb(0x1f2937))
                    .text_xs()
                    .text_color(gpui::rgb(0x9ca3af))
                    .child(status),
            )
            // Templates section
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(gpui::rgb(0x9ca3af))
                            .mb(px(8.0))
                            .child("Available Templates"),
                    )
                    .children(
                        self.templates
                            .iter()
                            .map(|t| self.render_template_button(t)),
                    ),
            )
            // History section
            .child(
                div()
                    .flex_1()
                    .px(px(16.0))
                    .py(px(8.0))
                    .overflow_y_scroll()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(gpui::rgb(0x9ca3af))
                            .mb(px(8.0))
                            .child("Execution History"),
                    )
                    .children(
                        self.history
                            .iter()
                            .rev()
                            .take(20)
                            .map(|e| self.render_history_entry(e)),
                    ),
            )
            // Stats footer
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .border_t_1()
                    .border_color(gpui::rgb(0x374151))
                    .text_xs()
                    .text_color(gpui::rgb(0x6b7280))
                    .child(format!(
                        "Templates: {} | Executions: {}",
                        self.templates.len(),
                        self.history.len()
                    )),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here using GPUI's test infrastructure
}
