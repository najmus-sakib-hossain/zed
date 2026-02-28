//! Cron job scheduler
//!
//! Pure Rust implementation using tokio-cron-scheduler

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};

/// Cron job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub command: String,
    pub enabled: bool,
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run: Option<chrono::DateTime<chrono::Utc>>,
}

/// Cron job manager
pub struct CronManager {
    scheduler: JobScheduler,
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
}

impl CronManager {
    /// Create new cron manager
    pub async fn new() -> Result<Self> {
        Ok(Self {
            scheduler: JobScheduler::new().await?,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Add a cron job
    pub async fn add_job(&mut self, job_def: CronJob) -> Result<()> {
        let job_id = job_def.id.clone();
        let command = job_def.command.clone();
        let jobs = self.jobs.clone();

        let job = Job::new_async(job_def.schedule.as_str(), move |_uuid, _lock| {
            let cmd = command.clone();
            let jobs_clone = jobs.clone();
            let id = job_id.clone();

            Box::pin(async move {
                tracing::info!("Executing cron job: {}", id);

                // Execute command
                if let Err(e) = execute_command(&cmd).await {
                    tracing::error!("Cron job {} failed: {}", id, e);
                }

                // Update last run time
                let mut jobs = jobs_clone.write().await;
                if let Some(job) = jobs.get_mut(&id) {
                    job.last_run = Some(chrono::Utc::now());
                }
            })
        })?;

        self.scheduler.add(job).await?;

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_def.id.clone(), job_def);

        Ok(())
    }

    /// Remove a cron job
    pub async fn remove_job(&mut self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(job_id);
        Ok(())
    }

    /// List all jobs
    pub async fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Start the scheduler
    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        Ok(())
    }

    /// Shutdown the scheduler
    pub async fn shutdown(&mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        Ok(())
    }
}

/// Execute a shell command
async fn execute_command(command: &str) -> Result<()> {
    use tokio::process::Command;

    #[cfg(unix)]
    let output = Command::new("sh").arg("-c").arg(command).output().await?;

    #[cfg(windows)]
    let output = Command::new("cmd").arg("/C").arg(command).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cron_manager() {
        let mut manager = CronManager::new().await.unwrap();

        let job = CronJob {
            id: "test-job".to_string(),
            name: "Test Job".to_string(),
            schedule: "0 * * * * *".to_string(), // Every minute
            command: "echo 'Hello from cron'".to_string(),
            enabled: true,
            last_run: None,
            next_run: None,
        };

        manager.add_job(job).await.unwrap();
        manager.start().await.unwrap();

        let jobs = manager.list_jobs().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "test-job");

        manager.shutdown().await.unwrap();
    }
}
