use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::process::{RestartPolicy, WorkerManager, WorkerStatus};

#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub check_interval: Duration,
    pub auto_restart: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            auto_restart: true,
        }
    }
}

pub struct HealthMonitor {
    task: JoinHandle<()>,
}

impl HealthMonitor {
    pub fn start(manager: Arc<WorkerManager>, config: HealthConfig) -> Self {
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(config.check_interval);
            loop {
                ticker.tick().await;

                let handles = manager.handles().await;
                for handle in handles {
                    let id = handle.spec.id.clone();
                    let exited_status = {
                        let mut child = handle.child.lock().await;
                        match child.try_wait() {
                            Ok(status) => status,
                            Err(e) => {
                                warn!("health check error for worker {}: {}", id, e);
                                None
                            }
                        }
                    };

                    if let Some(status) = exited_status {
                        *handle.status.lock().await = WorkerStatus::Failed;
                        warn!("worker {} exited: {:?}", id, status);
                        if config.auto_restart {
                            match &handle.spec.restart {
                                RestartPolicy::Never => {
                                    warn!("worker {} restart policy is never", id);
                                }
                                RestartPolicy::OnFailure {
                                    max_restarts,
                                    backoff_ms,
                                } => {
                                    let count = *handle.restart_count.lock().await;
                                    if count < *max_restarts {
                                        tokio::time::sleep(Duration::from_millis(*backoff_ms))
                                            .await;
                                        if let Err(e) = manager.restart(&id).await {
                                            warn!("failed to restart worker {}: {}", id, e);
                                        } else {
                                            info!("worker {} restarted by health monitor", id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { task }
    }

    pub async fn stop(self) {
        self.task.abort();
        let _ = self.task.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_config_default() {
        let cfg = HealthConfig::default();
        assert!(cfg.auto_restart);
        assert_eq!(cfg.check_interval, Duration::from_secs(5));
    }
}
