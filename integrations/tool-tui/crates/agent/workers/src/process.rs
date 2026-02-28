use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use dx_agent_sandbox::config::SandboxConfig;
use dx_agent_sandbox::docker::DockerSandbox;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::codec::{WorkerEnvelope, WorkerMessageKind};
use crate::ipc::{IpcEndpoint, connect};

#[derive(Debug, Clone)]
pub enum RestartPolicy {
    Never,
    OnFailure { max_restarts: u32, backoff_ms: u64 },
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::OnFailure {
            max_restarts: 5,
            backoff_ms: 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerSpec {
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub endpoint: IpcEndpoint,
    pub restart: RestartPolicy,
    pub sandboxed: bool,
    pub sandbox_config: Option<SandboxConfig>,
}

impl WorkerSpec {
    pub fn new(id: impl Into<String>, program: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            endpoint: IpcEndpoint::local(&id),
            id,
            program: program.into(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            restart: RestartPolicy::default(),
            sandboxed: false,
            sandbox_config: None,
        }
    }

    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandboxed = true;
        self.sandbox_config = Some(config);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Starting,
    Running,
    Stopped,
    Failed,
}

pub struct WorkerHandle {
    pub spec: WorkerSpec,
    pub child: Arc<Mutex<Child>>,
    pub status: Arc<Mutex<WorkerStatus>>,
    pub restart_count: Arc<Mutex<u32>>,
}

pub struct WorkerManager {
    workers: DashMap<String, Arc<WorkerHandle>>,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            workers: DashMap::new(),
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.workers.iter().map(|w| w.key().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.workers.len()
    }

    pub async fn spawn(&self, spec: WorkerSpec) -> anyhow::Result<()> {
        let mut cmd = Self::build_worker_command(&spec)?;

        let child = cmd.spawn()?;
        let handle = Arc::new(WorkerHandle {
            spec: spec.clone(),
            child: Arc::new(Mutex::new(child)),
            status: Arc::new(Mutex::new(WorkerStatus::Running)),
            restart_count: Arc::new(Mutex::new(0)),
        });

        self.workers.insert(spec.id.clone(), handle);
        info!("spawned worker {}", spec.id);
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> anyhow::Result<()> {
        let Some(handle) = self.workers.get(id).map(|h| h.clone()) else {
            anyhow::bail!("worker not found: {}", id);
        };

        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
        *handle.status.lock().await = WorkerStatus::Stopped;
        self.workers.remove(id);
        Ok(())
    }

    pub async fn send_command(&self, id: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        let Some(handle) = self.workers.get(id).map(|h| h.clone()) else {
            anyhow::bail!("worker not found: {}", id);
        };
        let mut conn = connect(&handle.spec.endpoint).await?;
        let env = WorkerEnvelope::new(id, WorkerMessageKind::Command, payload);
        conn.send(&env).await
    }

    pub async fn ping(&self, id: &str) -> anyhow::Result<()> {
        let Some(handle) = self.workers.get(id).map(|h| h.clone()) else {
            anyhow::bail!("worker not found: {}", id);
        };
        let mut conn = connect(&handle.spec.endpoint).await?;
        let env = WorkerEnvelope::new(id, WorkerMessageKind::Ping, serde_json::json!({}));
        conn.send(&env).await
    }

    pub async fn restart(&self, id: &str) -> anyhow::Result<()> {
        let Some(handle) = self.workers.get(id).map(|h| h.clone()) else {
            anyhow::bail!("worker not found: {}", id);
        };

        {
            let mut child = handle.child.lock().await;
            let _ = child.kill().await;
        }

        let mut cmd = Self::build_worker_command(&handle.spec)?;

        let new_child = cmd.spawn()?;
        *handle.child.lock().await = new_child;
        *handle.status.lock().await = WorkerStatus::Running;
        *handle.restart_count.lock().await += 1;
        warn!("restarted worker {}", id);
        Ok(())
    }

    pub async fn status(&self, id: &str) -> anyhow::Result<WorkerStatus> {
        let Some(handle) = self.workers.get(id).map(|h| h.clone()) else {
            anyhow::bail!("worker not found: {}", id);
        };
        Ok(*handle.status.lock().await)
    }

    pub async fn handles(&self) -> Vec<Arc<WorkerHandle>> {
        self.workers.iter().map(|h| h.value().clone()).collect()
    }

    fn build_worker_command(spec: &WorkerSpec) -> anyhow::Result<Command> {
        let mut cmd = if spec.sandboxed {
            Self::build_docker_command(spec)?
        } else {
            Command::new(&spec.program)
        };

        if !spec.sandboxed {
            cmd.args(&spec.args);
            if let Some(cwd) = &spec.cwd {
                cmd.current_dir(cwd);
            }
            for (k, v) in &spec.env {
                cmd.env(k, v);
            }
            Self::apply_endpoint_env(&mut cmd, &spec.endpoint);
        }

        cmd.kill_on_drop(true)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        Ok(cmd)
    }

    fn build_docker_command(spec: &WorkerSpec) -> anyhow::Result<Command> {
        if !DockerSandbox::is_available() {
            anyhow::bail!(
                "sandboxed worker '{}' requires Docker, but Docker is unavailable",
                spec.id
            );
        }

        let config = spec.sandbox_config.clone().unwrap_or_default();
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("--init")
            .arg("--cpus")
            .arg(config.limits.cpu_cores.to_string())
            .arg("--memory")
            .arg(format!("{}m", config.limits.memory_mb))
            .arg("--pids-limit")
            .arg(config.limits.max_pids.to_string());

        if !config.network.enabled {
            cmd.arg("--network").arg("none");
        }

        for (host, container) in &config.volumes {
            cmd.arg("-v").arg(format!("{}:{}:rw", host, container));
        }

        if let Some(cwd) = &spec.cwd {
            cmd.arg("-v")
                .arg(format!("{}:/workspace:rw", cwd.display()))
                .arg("-w")
                .arg("/workspace");
        } else {
            cmd.arg("-w").arg(&config.workdir);
        }

        for (k, v) in &config.env {
            cmd.arg("-e").arg(format!("{}={}", k, v));
        }
        for (k, v) in &spec.env {
            cmd.arg("-e").arg(format!("{}={}", k, v));
        }

        #[cfg(windows)]
        match &spec.endpoint {
            IpcEndpoint::NamedPipe(name) => {
                cmd.arg("-e").arg(format!("DX_WORKER_PIPE={}", name));
            }
        }
        #[cfg(not(windows))]
        if let IpcEndpoint::UnixSocket(path) = &spec.endpoint {
            cmd.arg("-e").arg(format!("DX_WORKER_SOCKET={}", path));
        }

        cmd.arg(&config.image).arg(&spec.program).args(&spec.args);
        Ok(cmd)
    }

    fn apply_endpoint_env(cmd: &mut Command, endpoint: &IpcEndpoint) {
        #[cfg(windows)]
        match endpoint {
            IpcEndpoint::NamedPipe(name) => {
                cmd.env("DX_WORKER_PIPE", name);
            }
        }

        #[cfg(not(windows))]
        if let IpcEndpoint::UnixSocket(path) = endpoint {
            cmd.env("DX_WORKER_SOCKET", path);
        }
    }
}

impl Default for WorkerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manager_spawn_and_stop() {
        let manager = WorkerManager::new();

        let spec = if cfg!(windows) {
            let mut s = WorkerSpec::new("test-worker", "cmd");
            s.args = vec!["/C".into(), "ping -n 2 127.0.0.1 >nul".into()];
            s
        } else {
            let mut s = WorkerSpec::new("test-worker", "sh");
            s.args = vec!["-c".into(), "sleep 1".into()];
            s
        };

        manager.spawn(spec).await.expect("spawn");
        assert_eq!(manager.count(), 1);
        manager.stop("test-worker").await.expect("stop");
        assert_eq!(manager.count(), 0);
    }
}
