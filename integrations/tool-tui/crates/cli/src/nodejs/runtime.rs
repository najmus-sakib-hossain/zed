//! Bun runtime bridge for executing Node.js code from Rust
//! Provides high-performance JavaScript execution via Bun subprocess

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::mpsc;

/// Bun runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BunConfig {
    pub bun_path: PathBuf,
    pub working_dir: PathBuf,
    pub env_vars: Vec<(String, String)>,
    pub timeout_secs: u64,
}

impl Default for BunConfig {
    fn default() -> Self {
        Self {
            bun_path: PathBuf::from("bun"),
            working_dir: std::env::current_dir().unwrap_or_default(),
            env_vars: vec![],
            timeout_secs: 30,
        }
    }
}

/// Bun runtime executor
pub struct BunRuntime {
    config: BunConfig,
    process: Option<Child>,
}

impl BunRuntime {
    pub fn new(config: BunConfig) -> Self {
        Self {
            config,
            process: None,
        }
    }

    /// Check if Bun is installed
    pub fn is_installed() -> bool {
        Command::new("bun")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Execute a JavaScript file with Bun
    pub async fn execute_file(&mut self, script_path: &Path) -> Result<String> {
        let mut cmd = TokioCommand::new(&self.config.bun_path);
        cmd.arg("run")
            .arg(script_path)
            .current_dir(&self.config.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, val) in &self.config.env_vars {
            cmd.env(key, val);
        }

        let mut child = cmd.spawn().context("Failed to spawn Bun process")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stderr = child.stderr.take().context("Failed to capture stderr")?;

        let (tx, mut rx) = mpsc::channel(100);
        let tx_err = tx.clone();

        // Read stdout
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(line).await;
            }
        });

        // Read stderr
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("Bun stderr: {}", line);
                let _ = tx_err.send(line).await;
            }
        });

        let mut output = String::new();
        while let Some(line) = rx.recv().await {
            output.push_str(&line);
            output.push('\n');
        }

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("Bun process exited with status: {}", status);
        }

        Ok(output)
    }

    /// Execute JavaScript code directly
    pub async fn execute_code(&mut self, code: &str) -> Result<String> {
        let temp_file = self.config.working_dir.join(".dx_temp_script.js");
        tokio::fs::write(&temp_file, code).await?;

        let result = self.execute_file(&temp_file).await;
        let _ = tokio::fs::remove_file(&temp_file).await;

        result
    }

    /// Spawn a long-running Bun process with IPC
    pub async fn spawn_worker(&mut self, script_path: &Path) -> Result<()> {
        let mut cmd = TokioCommand::new(&self.config.bun_path);
        cmd.arg("run")
            .arg(script_path)
            .current_dir(&self.config.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, val) in &self.config.env_vars {
            cmd.env(key, val);
        }

        let child = cmd.spawn().context("Failed to spawn Bun worker")?;
        self.process = Some(child);

        Ok(())
    }

    /// Send message to worker process
    pub async fn send_message(&mut self, message: &str) -> Result<()> {
        if let Some(ref mut child) = self.process {
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(message.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    /// Stop the worker process
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().await?;
        }
        Ok(())
    }
}

impl Drop for BunRuntime {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let _ = child.kill().await;
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bun_installed() {
        let installed = BunRuntime::is_installed();
        println!("Bun installed: {}", installed);
    }

    #[tokio::test]
    async fn test_execute_code() {
        if !BunRuntime::is_installed() {
            return;
        }

        let mut runtime = BunRuntime::new(BunConfig::default());
        let result = runtime.execute_code("console.log('Hello from Bun')").await;
        assert!(result.is_ok());
    }
}
