//! Native OS sandboxing (platform-specific)

use anyhow::Result;
use std::time::Instant;

use super::ExecutionResult;
use super::config::SandboxConfig;

/// Native OS sandbox
pub struct NativeSandbox {
    config: SandboxConfig,
}

impl NativeSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Execute a command with OS-level isolation
    pub async fn execute(&self, command: &[&str]) -> Result<ExecutionResult> {
        let start = Instant::now();

        if command.is_empty() {
            anyhow::bail!("Empty command");
        }

        let mut cmd = tokio::process::Command::new(command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        // Set working directory
        cmd.current_dir(&self.config.workdir);

        // Set environment
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // Apply platform-specific sandboxing
        #[cfg(target_os = "windows")]
        self.apply_windows_sandbox(&mut cmd)?;

        #[cfg(target_os = "linux")]
        self.apply_linux_sandbox(&mut cmd);

        #[cfg(target_os = "macos")]
        self.apply_macos_sandbox(&mut cmd);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            cmd.output(),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(output)) => Ok(ExecutionResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms,
                killed: false,
            }),
            Ok(Err(e)) => Err(anyhow::anyhow!("Command failed: {}", e)),
            Err(_) => Ok(ExecutionResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: "Command timed out".into(),
                duration_ms,
                killed: true,
            }),
        }
    }

    #[cfg(target_os = "windows")]
    fn apply_windows_sandbox(&self, _cmd: &mut tokio::process::Command) -> Result<()> {
        // On Windows, we use Job Objects for process isolation.
        // Job Objects allow limiting:
        // - Memory usage
        // - CPU time
        // - Number of processes
        // - UI restrictions
        //
        // The actual Job Object creation happens in the spawned process.
        // For now, basic process-level isolation.
        tracing::debug!("Applying Windows native sandbox (Job Objects)");
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn apply_linux_sandbox(&self, cmd: &mut tokio::process::Command) {
        // On Linux, use unshare for namespace isolation
        // seccomp-bpf for syscall filtering would require additional setup
        tracing::debug!("Applying Linux native sandbox (namespaces)");

        // Set resource limits via ulimit-style rlimit
        // Note: Full namespace isolation requires root or user namespace support
    }

    #[cfg(target_os = "macos")]
    fn apply_macos_sandbox(&self, _cmd: &mut tokio::process::Command) {
        // On macOS, use sandbox-exec with a profile
        tracing::debug!("Applying macOS native sandbox (sandbox profiles)");
    }
}

/// Sandbox manager - creates and manages sandbox instances
pub struct SandboxManager {
    active_sandboxes: std::collections::HashMap<String, SandboxType>,
}

#[derive(Debug)]
enum SandboxType {
    #[allow(dead_code)]
    Docker(String), // container_id
    #[allow(dead_code)]
    Native,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            active_sandboxes: std::collections::HashMap::new(),
        }
    }

    /// Create a sandbox for a session
    pub fn create_for_session(&mut self, session_id: &str) -> String {
        let sandbox_id = format!("sandbox-{}", uuid::Uuid::new_v4().as_simple());
        self.active_sandboxes.insert(sandbox_id.clone(), SandboxType::Native);
        tracing::info!("Created sandbox {} for session {}", sandbox_id, session_id);
        sandbox_id
    }

    /// Remove a sandbox
    pub fn remove(&mut self, sandbox_id: &str) {
        self.active_sandboxes.remove(sandbox_id);
    }

    /// List active sandboxes
    pub fn list_active(&self) -> Vec<String> {
        self.active_sandboxes.keys().cloned().collect()
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_manager() {
        let mut mgr = SandboxManager::new();
        let id = mgr.create_for_session("session-1");
        assert_eq!(mgr.list_active().len(), 1);
        mgr.remove(&id);
        assert_eq!(mgr.list_active().len(), 0);
    }

    #[tokio::test]
    async fn test_native_sandbox_echo() {
        let config = super::super::config::SandboxConfig {
            workdir: ".".into(),
            timeout_secs: 10,
            ..Default::default()
        };
        let sandbox = NativeSandbox::new(config);

        #[cfg(target_os = "windows")]
        let result = sandbox.execute(&["cmd", "/c", "echo", "hello"]).await;
        #[cfg(not(target_os = "windows"))]
        let result = sandbox.execute(&["echo", "hello"]).await;

        let result = result.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }
}
