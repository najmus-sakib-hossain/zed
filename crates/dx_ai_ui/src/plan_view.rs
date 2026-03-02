//! Plan View — Multi-step planning interface (Part 2).
//!
//! When the "Plan" profile is active, displays:
//! - Goal decomposition tree
//! - Step-by-step execution timeline
//! - Progress tracking with cost estimates

use gpui::{div, prelude::*, SharedString, Window};

/// Multi-step planner view.
pub struct PlanView {
    goal: String,
    steps: Vec<PlanStep>,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub title: String,
    pub description: String,
    pub status: PlanStepStatus,
    pub estimated_cost_cents: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

impl PlanView {
    pub fn new() -> Self {
        Self {
            goal: String::new(),
            steps: Vec::new(),
        }
    }

    pub fn set_goal(&mut self, goal: String, cx: &mut Context<Self>) {
        self.goal = goal;
        cx.notify();
    }

    pub fn add_step(&mut self, step: PlanStep, cx: &mut Context<Self>) {
        self.steps.push(step);
        cx.notify();
    }

    pub fn completed_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.status == PlanStepStatus::Completed)
            .count()
    }
}

impl Render for PlanView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.steps.len();
        let done = self.completed_step_count();
        let label: SharedString = if self.goal.is_empty() {
            "Enter a goal to begin planning...".into()
        } else {
            format!("Plan: {} — {}/{} steps complete", self.goal, done, total).into()
        };

        div().size_full().flex().flex_col().child(label)
    }
}
