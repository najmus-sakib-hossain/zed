//! # Task Scheduler
//!
//! Schedule and execute tasks using cron expressions.
//! Runs tasks like checking emails, updating todos, etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::Result;

/// Cron schedule expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Cron expression (e.g., "0 * * * *" for every hour)
    pub expression: String,

    /// Human-readable description
    pub description: String,
}

impl CronSchedule {
    pub fn every_minute() -> Self {
        Self {
            expression: "* * * * *".to_string(),
            description: "Every minute".to_string(),
        }
    }

    pub fn every_hour() -> Self {
        Self {
            expression: "0 * * * *".to_string(),
            description: "Every hour".to_string(),
        }
    }

    pub fn every_day_at(hour: u32) -> Self {
        Self {
            expression: format!("0 {} * * *", hour),
            description: format!("Every day at {}:00", hour),
        }
    }

    pub fn every_week_on(day: &str, hour: u32) -> Self {
        let day_num = match day.to_lowercase().as_str() {
            "sunday" | "sun" => 0,
            "monday" | "mon" => 1,
            "tuesday" | "tue" => 2,
            "wednesday" | "wed" => 3,
            "thursday" | "thu" => 4,
            "friday" | "fri" => 5,
            "saturday" | "sat" => 6,
            _ => 0,
        };
        Self {
            expression: format!("0 {} * * {}", hour, day_num),
            description: format!("Every {} at {}:00", day, hour),
        }
    }
}

/// A scheduled task
#[derive(Clone)]
pub struct Task {
    name: String,
    description: String,
    schedule: CronSchedule,
    skill_name: String,
    skill_context: String,
    last_run: Option<DateTime<Utc>>,
    next_run: Option<DateTime<Utc>>,
    enabled: bool,
}

impl Task {
    pub fn new(
        name: &str,
        description: &str,
        schedule: CronSchedule,
        skill_name: &str,
        skill_context: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            schedule,
            skill_name: skill_name.to_string(),
            skill_context: skill_context.to_string(),
            last_run: None,
            next_run: None,
            enabled: true,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn skill_name(&self) -> &str {
        &self.skill_name
    }

    pub fn skill_context(&self) -> &str {
        &self.skill_context
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if the task is due to run
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }

        match self.next_run {
            Some(next) => Utc::now() >= next,
            None => true, // Never run before, so it's due
        }
    }

    /// Execute the task
    pub async fn execute(&self) -> Result<()> {
        info!("Executing task: {}", self.name);

        // In a real implementation, this would call the skill
        // For now, just log
        info!("  Skill: {}", self.skill_name);
        info!("  Context: {}", self.skill_context);

        Ok(())
    }

    /// Update the next run time
    pub fn update_next_run(&mut self) {
        self.last_run = Some(Utc::now());
        // In a real implementation, calculate next run from cron expression
        // For now, set to 1 hour from now
        self.next_run = Some(Utc::now() + chrono::Duration::hours(1));
    }

    /// Convert to DX format
    pub fn to_dx(&self) -> String {
        format!(
            "task:1[name={} schedule={} skill={} enabled={}]",
            self.name.replace(' ', "_"),
            self.schedule.expression,
            self.skill_name,
            self.enabled
        )
    }
}

/// Task scheduler
pub struct TaskScheduler {
    tasks: HashMap<String, Task>,
    #[allow(dead_code)]
    running: bool,
}

impl TaskScheduler {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tasks: HashMap::new(),
            running: false,
        })
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        info!("Task scheduler started");
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        info!("Task scheduler stopped");
        Ok(())
    }

    /// Add a task
    pub fn add_task(&mut self, task: Task) {
        let name = task.name.clone();
        self.tasks.insert(name.clone(), task);
        info!("Task added: {}", name);
    }

    /// Remove a task
    pub fn remove_task(&mut self, name: &str) {
        self.tasks.remove(name);
        info!("Task removed: {}", name);
    }

    /// Get a task by name
    pub fn get_task(&self, name: &str) -> Option<&Task> {
        self.tasks.get(name)
    }

    /// Get all tasks that are due to run
    pub async fn get_due_tasks(&self) -> Vec<Task> {
        self.tasks
            .values()
            .filter(|t| t.is_due())
            .cloned()
            .collect()
    }

    /// List all tasks in DX format
    pub fn list_as_dx(&self) -> String {
        let tasks: Vec<String> = self.tasks.values().map(|t| t.to_dx()).collect();

        format!("tasks:{}[{}]", tasks.len(), tasks.join(" "))
    }

    /// Load tasks from DX format
    pub fn load_from_dx(&mut self, _dx: &str) -> Result<()> {
        // Parse DX format and add tasks
        // Format: task:1[name=x schedule=y skill=z enabled=true]

        // Placeholder - would parse in production
        Ok(())
    }

    /// Create default tasks
    pub fn create_defaults(&mut self) {
        // Check email every hour
        self.add_task(Task::new(
            "check_email",
            "Check and summarize new emails",
            CronSchedule::every_hour(),
            "check_email",
            "count=10",
        ));

        // Daily summary at 9 AM
        self.add_task(Task::new(
            "daily_summary",
            "Generate daily summary of tasks and emails",
            CronSchedule::every_day_at(9),
            "browse_web",
            "url=https://calendar.google.com",
        ));

        // Weekly review on Sunday at 6 PM
        self.add_task(Task::new(
            "weekly_review",
            "Weekly review and planning",
            CronSchedule::every_week_on("sunday", 18),
            "create_todo",
            "title=Weekly_Review",
        ));
    }
}
