//! Cron scheduler — runs jobs on a schedule.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    /// Cron expression (e.g., "0 */6 * * *" = every 6 hours).
    pub schedule: String,
    /// Action to execute.
    pub action: CronAction,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// Last execution time.
    pub last_run: Option<std::time::SystemTime>,
}

/// What action a cron job performs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CronAction {
    /// Run an LLM prompt and send result to a channel.
    LlmPrompt {
        prompt: String,
        channel_id: Option<String>,
    },
    /// Generate media content.
    MediaGeneration {
        prompt: String,
        media_type: String,
    },
    /// Execute a shell command.
    ShellCommand { command: String },
    /// Send a message to a channel.
    SendMessage {
        channel_id: String,
        message: String,
    },
    /// Run a custom webhook.
    Webhook { url: String, payload: String },
}

/// Manages scheduled jobs.
pub struct CronScheduler {
    jobs: HashMap<String, CronJob>,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    /// Add a job.
    pub fn add_job(&mut self, job: CronJob) {
        self.jobs.insert(job.id.clone(), job);
    }

    /// Remove a job.
    pub fn remove_job(&mut self, id: &str) -> Option<CronJob> {
        self.jobs.remove(id)
    }

    /// Get all jobs.
    pub fn jobs(&self) -> impl Iterator<Item = &CronJob> {
        self.jobs.values()
    }

    /// Enable/disable a job.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.enabled = enabled;
        }
    }

    /// Check which jobs are due for execution.
    pub fn due_jobs(&self) -> Vec<&CronJob> {
        // Placeholder — real implementation parses cron expressions
        self.jobs.values().filter(|j| j.enabled).collect()
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}
