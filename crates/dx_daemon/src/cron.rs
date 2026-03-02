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
        let now = std::time::SystemTime::now();
        self.jobs
            .values()
            .filter(|j| j.enabled && self.is_due(j, now))
            .collect()
    }

    /// Check if a specific job is due now.
    fn is_due(&self, job: &CronJob, now: std::time::SystemTime) -> bool {
        if let Ok(parsed) = CronExpression::parse(&job.schedule) {
            // If never run, it's due
            let Some(last_run) = job.last_run else {
                return true;
            };

            // Check if enough time has passed based on the schedule's minimum interval
            let elapsed = now.duration_since(last_run).unwrap_or_default();
            let min_interval = parsed.minimum_interval_seconds();

            elapsed.as_secs() >= min_interval
        } else {
            log::warn!("Invalid cron expression for job '{}': {}", job.id, job.schedule);
            false
        }
    }

    /// Mark a job as having just run.
    pub fn mark_run(&mut self, id: &str) {
        if let Some(job) = self.jobs.get_mut(id) {
            job.last_run = Some(std::time::SystemTime::now());
        }
    }

    /// Get the next scheduled run time for a job (human-readable).
    pub fn next_run_description(&self, id: &str) -> Option<String> {
        let job = self.jobs.get(id)?;
        if let Ok(expr) = CronExpression::parse(&job.schedule) {
            Some(expr.describe())
        } else {
            None
        }
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Cron expression parser
// ---------------------------------------------------------------------------

/// Parsed cron expression (standard 5-field format).
///
/// Format: `minute hour day_of_month month day_of_week`
///
/// Supports:
/// - Wildcards: `*`
/// - Step values: `*/5` (every 5)
/// - Ranges: `1-5`
/// - Lists: `1,3,5`
/// - Specific values: `30`
#[derive(Debug, Clone)]
pub struct CronExpression {
    pub minute: CronField,
    pub hour: CronField,
    pub day_of_month: CronField,
    pub month: CronField,
    pub day_of_week: CronField,
}

/// A single field in a cron expression.
#[derive(Debug, Clone)]
pub enum CronField {
    /// Match any value.
    Any,
    /// Match a specific value.
    Value(u32),
    /// Match any of these values.
    List(Vec<u32>),
    /// Match a range (inclusive).
    Range(u32, u32),
    /// Match every Nth value starting from a base.
    Step { base: u32, step: u32 },
}

impl CronField {
    /// Parse a single cron field.
    pub fn parse(field: &str, min: u32, max: u32) -> anyhow::Result<Self> {
        if field == "*" {
            return Ok(CronField::Any);
        }

        // Step: */N or base/N
        if let Some((base_str, step_str)) = field.split_once('/') {
            let base = if base_str == "*" {
                min
            } else {
                base_str.parse::<u32>()?
            };
            let step = step_str.parse::<u32>()?;
            return Ok(CronField::Step { base, step });
        }

        // List: N,M,O
        if field.contains(',') {
            let values: Result<Vec<u32>, _> = field.split(',').map(|s| s.trim().parse::<u32>()).collect();
            return Ok(CronField::List(values?));
        }

        // Range: N-M
        if let Some((start_str, end_str)) = field.split_once('-') {
            let start = start_str.parse::<u32>()?;
            let end = end_str.parse::<u32>()?;
            return Ok(CronField::Range(start, end));
        }

        // Single value
        let value = field.parse::<u32>()?;
        if value < min || value > max {
            anyhow::bail!(
                "Cron field value {} out of range [{}, {}]",
                value,
                min,
                max
            );
        }
        Ok(CronField::Value(value))
    }

    /// Check if a value matches this field.
    pub fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Value(v) => *v == value,
            CronField::List(vs) => vs.contains(&value),
            CronField::Range(start, end) => value >= *start && value <= *end,
            CronField::Step { base, step } => {
                if *step == 0 {
                    return false;
                }
                value >= *base && (value - base) % step == 0
            }
        }
    }
}

impl CronExpression {
    /// Parse a standard 5-field cron expression.
    pub fn parse(expr: &str) -> anyhow::Result<Self> {
        // Handle named shortcuts
        let expr = match expr.trim() {
            "@yearly" | "@annually" => "0 0 1 1 *",
            "@monthly" => "0 0 1 * *",
            "@weekly" => "0 0 * * 0",
            "@daily" | "@midnight" => "0 0 * * *",
            "@hourly" => "0 * * * *",
            other => other,
        };

        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            anyhow::bail!(
                "Cron expression must have 5 fields (minute hour day month weekday), got {}",
                fields.len()
            );
        }

        Ok(Self {
            minute: CronField::parse(fields[0], 0, 59)?,
            hour: CronField::parse(fields[1], 0, 23)?,
            day_of_month: CronField::parse(fields[2], 1, 31)?,
            month: CronField::parse(fields[3], 1, 12)?,
            day_of_week: CronField::parse(fields[4], 0, 7)?, // 0 and 7 = Sunday
        })
    }

    /// Estimate the minimum interval between executions in seconds.
    pub fn minimum_interval_seconds(&self) -> u64 {
        match &self.minute {
            CronField::Step { step, .. } => (*step as u64) * 60,
            CronField::Value(_) => {
                // Specific minute — at most once per hour
                match &self.hour {
                    CronField::Step { step, .. } => (*step as u64) * 3600,
                    CronField::Value(_) => 86400, // Once per day
                    CronField::Any => 3600,        // Once per hour
                    CronField::List(vs) => {
                        if vs.len() > 1 { 3600 } else { 86400 }
                    }
                    CronField::Range(_, _) => 3600,
                }
            }
            CronField::Any => 60, // Every minute
            CronField::List(vs) => {
                if vs.len() > 1 { 60 } else { 3600 }
            }
            CronField::Range(_, _) => 60,
        }
    }

    /// Human-readable description of the schedule.
    pub fn describe(&self) -> String {
        let minute_desc = match &self.minute {
            CronField::Any => "every minute".to_string(),
            CronField::Value(v) => format!("at minute {}", v),
            CronField::Step { step, .. } => format!("every {} minutes", step),
            CronField::List(vs) => format!("at minutes {:?}", vs),
            CronField::Range(a, b) => format!("minutes {}-{}", a, b),
        };

        let hour_desc = match &self.hour {
            CronField::Any => String::new(),
            CronField::Value(v) => format!(", at {}:00", v),
            CronField::Step { step, .. } => format!(", every {} hours", step),
            CronField::List(vs) => format!(", at hours {:?}", vs),
            CronField::Range(a, b) => format!(", hours {}-{}", a, b),
        };

        format!("{}{}", minute_desc, hour_desc)
    }
}
