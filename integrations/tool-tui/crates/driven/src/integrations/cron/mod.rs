//! # Cron Scheduler Integration
//!
//! Schedule and run recurring tasks with cron syntax.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::cron::{CronScheduler, CronConfig, CronJob};
//!
//! let config = CronConfig::from_file("~/.dx/config/cron.sr")?;
//! let mut scheduler = CronScheduler::new(&config)?;
//!
//! // Add a job
//! scheduler.add_job(CronJob {
//!     name: "backup".to_string(),
//!     schedule: "0 0 * * *".to_string(), // Daily at midnight
//!     command: "dx backup".to_string(),
//!     enabled: true,
//!     ..Default::default()
//! })?;
//!
//! // Start scheduler
//! scheduler.start().await?;
//! ```

use crate::error::{DrivenError, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cron configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    /// Whether cron is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Default timezone
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Configured jobs
    #[serde(default)]
    pub jobs: Vec<CronJob>,
    /// Maximum concurrent jobs
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_true() -> bool {
    true
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_max_concurrent() -> usize {
    5
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timezone: default_timezone(),
            jobs: Vec::new(),
            max_concurrent: default_max_concurrent(),
        }
    }
}

impl CronConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Job name (unique identifier)
    pub name: String,
    /// Cron schedule expression
    pub schedule: String,
    /// Command to run
    pub command: String,
    /// Whether job is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Override timezone for this job
    pub timezone: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Retry count on failure
    #[serde(default)]
    pub retry_count: u32,
    /// Retry delay in seconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
}

fn default_retry_delay() -> u64 {
    60
}

impl Default for CronJob {
    fn default() -> Self {
        Self {
            name: String::new(),
            schedule: String::new(),
            command: String::new(),
            enabled: true,
            timezone: None,
            env: HashMap::new(),
            working_dir: None,
            timeout: None,
            retry_count: 0,
            retry_delay: default_retry_delay(),
        }
    }
}

/// Job execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is scheduled but not running
    Idle,
    /// Job is currently running
    Running,
    /// Job completed successfully
    Success,
    /// Job failed
    Failed,
    /// Job timed out
    Timeout,
    /// Job is disabled
    Disabled,
}

/// Job execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecution {
    /// Job name
    pub job_name: String,
    /// Execution ID
    pub execution_id: String,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Status
    pub status: JobStatus,
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    /// Output (stdout)
    pub output: String,
    /// Error output (stderr)
    pub error: String,
    /// Retry attempt number
    pub attempt: u32,
}

/// Parsed cron expression
#[derive(Debug, Clone)]
pub struct CronExpression {
    /// Minutes (0-59)
    pub minutes: Vec<u8>,
    /// Hours (0-23)
    pub hours: Vec<u8>,
    /// Days of month (1-31)
    pub days_of_month: Vec<u8>,
    /// Months (1-12)
    pub months: Vec<u8>,
    /// Days of week (0-6, Sunday = 0)
    pub days_of_week: Vec<u8>,
}

impl CronExpression {
    /// Parse a cron expression string
    pub fn parse(expr: &str) -> Result<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        
        if parts.len() != 5 {
            return Err(DrivenError::Parse(format!(
                "Invalid cron expression: expected 5 parts, got {}",
                parts.len()
            )));
        }

        Ok(Self {
            minutes: Self::parse_field(parts[0], 0, 59)?,
            hours: Self::parse_field(parts[1], 0, 23)?,
            days_of_month: Self::parse_field(parts[2], 1, 31)?,
            months: Self::parse_field(parts[3], 1, 12)?,
            days_of_week: Self::parse_field(parts[4], 0, 6)?,
        })
    }

    /// Parse a single cron field
    fn parse_field(field: &str, min: u8, max: u8) -> Result<Vec<u8>> {
        let mut values = Vec::new();

        for part in field.split(',') {
            if part == "*" {
                // All values
                values.extend(min..=max);
            } else if part.contains('/') {
                // Step values (e.g., */5)
                let step_parts: Vec<&str> = part.split('/').collect();
                let step: u8 = step_parts[1].parse()
                    .map_err(|_| DrivenError::Parse(format!("Invalid step: {}", step_parts[1])))?;
                
                let start = if step_parts[0] == "*" { min } else {
                    step_parts[0].parse()
                        .map_err(|_| DrivenError::Parse(format!("Invalid start: {}", step_parts[0])))?
                };
                
                let mut val = start;
                while val <= max {
                    values.push(val);
                    val += step;
                }
            } else if part.contains('-') {
                // Range (e.g., 1-5)
                let range_parts: Vec<&str> = part.split('-').collect();
                let start: u8 = range_parts[0].parse()
                    .map_err(|_| DrivenError::Parse(format!("Invalid range start: {}", range_parts[0])))?;
                let end: u8 = range_parts[1].parse()
                    .map_err(|_| DrivenError::Parse(format!("Invalid range end: {}", range_parts[1])))?;
                
                values.extend(start..=end);
            } else {
                // Single value
                let val: u8 = part.parse()
                    .map_err(|_| DrivenError::Parse(format!("Invalid value: {}", part)))?;
                
                if val < min || val > max {
                    return Err(DrivenError::Parse(format!(
                        "Value {} out of range [{}, {}]",
                        val, min, max
                    )));
                }
                
                values.push(val);
            }
        }

        values.sort();
        values.dedup();
        Ok(values)
    }

    /// Check if the expression matches a given time
    pub fn matches(&self, time: &DateTime<Utc>) -> bool {
        let minute = time.format("%M").to_string().parse::<u8>().unwrap();
        let hour = time.format("%H").to_string().parse::<u8>().unwrap();
        let day = time.format("%d").to_string().parse::<u8>().unwrap();
        let month = time.format("%m").to_string().parse::<u8>().unwrap();
        let weekday = time.format("%w").to_string().parse::<u8>().unwrap();

        self.minutes.contains(&minute)
            && self.hours.contains(&hour)
            && self.days_of_month.contains(&day)
            && self.months.contains(&month)
            && self.days_of_week.contains(&weekday)
    }

    /// Get the next run time after a given time
    pub fn next_run(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut current = *after + chrono::Duration::minutes(1);
        
        // Search for next match within 1 year
        let limit = *after + chrono::Duration::days(366);
        
        while current < limit {
            if self.matches(&current) {
                return Some(current);
            }
            current = current + chrono::Duration::minutes(1);
        }

        None
    }
}

/// Cron scheduler
pub struct CronScheduler {
    config: CronConfig,
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
    executions: Arc<RwLock<Vec<JobExecution>>>,
    running: Arc<RwLock<bool>>,
}

impl CronScheduler {
    /// Create a new cron scheduler
    pub fn new(config: &CronConfig) -> Result<Self> {
        let mut jobs = HashMap::new();
        for job in &config.jobs {
            if job.enabled {
                // Validate cron expression
                CronExpression::parse(&job.schedule)?;
                jobs.insert(job.name.clone(), job.clone());
            }
        }

        Ok(Self {
            config: config.clone(),
            jobs: Arc::new(RwLock::new(jobs)),
            executions: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Add a job
    pub async fn add_job(&self, job: CronJob) -> Result<()> {
        // Validate cron expression
        CronExpression::parse(&job.schedule)?;
        
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.name.clone(), job);
        Ok(())
    }

    /// Remove a job
    pub async fn remove_job(&self, name: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(name);
        Ok(())
    }

    /// Get a job by name
    pub async fn get_job(&self, name: &str) -> Option<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.get(name).cloned()
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(DrivenError::Config("Cron scheduler is disabled".into()));
        }

        {
            let mut running = self.running.write().await;
            if *running {
                return Err(DrivenError::Config("Scheduler already running".into()));
            }
            *running = true;
        }

        tracing::info!("Starting cron scheduler");
        
        self.run_scheduler().await
    }

    /// Stop the scheduler
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Stopping cron scheduler");
        Ok(())
    }

    /// Run the scheduler loop
    async fn run_scheduler(&self) -> Result<()> {
        loop {
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            let now = Utc::now();
            let jobs = self.jobs.read().await.clone();

            for (name, job) in jobs {
                if !job.enabled {
                    continue;
                }

                let expr = match CronExpression::parse(&job.schedule) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!("Invalid cron expression for {}: {}", name, e);
                        continue;
                    }
                };

                if expr.matches(&now) {
                    // Run job in background
                    let job_clone = job.clone();
                    let executions = self.executions.clone();
                    
                    tokio::spawn(async move {
                        let result = Self::execute_job(&job_clone).await;
                        
                        let mut execs = executions.write().await;
                        execs.push(result);
                        
                        // Keep only last 1000 executions
                        if execs.len() > 1000 {
                            execs.drain(0..execs.len() - 1000);
                        }
                    });
                }
            }

            // Sleep until next minute
            let sleep_duration = 60 - now.timestamp() % 60;
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration as u64)).await;
        }

        Ok(())
    }

    /// Execute a job
    async fn execute_job(job: &CronJob) -> JobExecution {
        let execution_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();

        tracing::info!("Executing job: {}", job.name);

        let mut execution = JobExecution {
            job_name: job.name.clone(),
            execution_id,
            started_at,
            ended_at: None,
            status: JobStatus::Running,
            exit_code: None,
            output: String::new(),
            error: String::new(),
            attempt: 1,
        };

        // Run command
        let result = Self::run_command(job).await;

        execution.ended_at = Some(Utc::now());

        match result {
            Ok((exit_code, stdout, stderr)) => {
                execution.exit_code = Some(exit_code);
                execution.output = stdout;
                execution.error = stderr;
                execution.status = if exit_code == 0 {
                    JobStatus::Success
                } else {
                    JobStatus::Failed
                };
            }
            Err(e) => {
                execution.error = e.to_string();
                execution.status = JobStatus::Failed;
            }
        }

        execution
    }

    /// Run a shell command
    async fn run_command(job: &CronJob) -> Result<(i32, String, String)> {
        use tokio::process::Command;

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &job.command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &job.command]);
            c
        };

        // Set working directory
        if let Some(ref dir) = job.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &job.env {
            cmd.env(key, value);
        }

        let output = if let Some(timeout_secs) = job.timeout {
            tokio::time::timeout(
                tokio::time::Duration::from_secs(timeout_secs),
                cmd.output(),
            )
            .await
            .map_err(|_| DrivenError::Timeout("Job timed out".into()))?
            .map_err(|e| DrivenError::Process(e.to_string()))?
        } else {
            cmd.output()
                .await
                .map_err(|e| DrivenError::Process(e.to_string()))?
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((exit_code, stdout, stderr))
    }

    /// Get recent executions
    pub async fn get_executions(&self, limit: usize) -> Vec<JobExecution> {
        let executions = self.executions.read().await;
        executions.iter().rev().take(limit).cloned().collect()
    }

    /// Get executions for a specific job
    pub async fn get_job_executions(&self, job_name: &str, limit: usize) -> Vec<JobExecution> {
        let executions = self.executions.read().await;
        executions
            .iter()
            .filter(|e| e.job_name == job_name)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cron_expression() {
        let expr = CronExpression::parse("0 0 * * *").unwrap();
        assert_eq!(expr.minutes, vec![0]);
        assert_eq!(expr.hours, vec![0]);
        assert_eq!(expr.days_of_month.len(), 31);
    }

    #[test]
    fn test_parse_step() {
        let expr = CronExpression::parse("*/15 * * * *").unwrap();
        assert_eq!(expr.minutes, vec![0, 15, 30, 45]);
    }

    #[test]
    fn test_parse_range() {
        let expr = CronExpression::parse("0 9-17 * * *").unwrap();
        assert_eq!(expr.hours, vec![9, 10, 11, 12, 13, 14, 15, 16, 17]);
    }

    #[test]
    fn test_default_config() {
        let config = CronConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timezone, "UTC");
    }
}
