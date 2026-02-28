//! Process spawning.

use crate::error::NodeResult;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;

/// Stdio configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stdio {
    /// Pipe stdio
    Pipe,
    /// Inherit from parent
    Inherit,
    /// Ignore
    Ignore,
}

/// Spawn options.
#[derive(Debug, Clone, Default)]
pub struct SpawnOptions {
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Stdin configuration
    pub stdin: Option<Stdio>,
    /// Stdout configuration
    pub stdout: Option<Stdio>,
    /// Stderr configuration
    pub stderr: Option<Stdio>,
}

/// Child process output.
#[derive(Debug)]
pub struct Output {
    /// Exit status code
    pub status: i32,
    /// Stdout bytes
    pub stdout: Vec<u8>,
    /// Stderr bytes
    pub stderr: Vec<u8>,
}

/// Spawn a child process asynchronously.
pub async fn spawn(cmd: &str, args: &[&str], options: Option<SpawnOptions>) -> NodeResult<Output> {
    let options = options.unwrap_or_default();
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }

    if let Some(env) = &options.env {
        command.envs(env);
    }

    let output = command.output().await?;

    Ok(Output {
        status: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Execute a command in a shell.
pub async fn exec(cmd: &str, options: Option<SpawnOptions>) -> NodeResult<Output> {
    #[cfg(unix)]
    let (shell, flag) = ("sh", "-c");
    #[cfg(windows)]
    let (shell, flag) = ("cmd", "/C");

    spawn(shell, &[flag, cmd], options).await
}

/// Spawn a child process synchronously.
pub fn spawn_sync(cmd: &str, args: &[&str], options: Option<SpawnOptions>) -> NodeResult<Output> {
    let options = options.unwrap_or_default();
    let mut command = std::process::Command::new(cmd);
    command.args(args);

    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }

    if let Some(env) = &options.env {
        command.envs(env);
    }

    let output = command.output()?;

    Ok(Output {
        status: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Execute a command in a shell synchronously.
pub fn exec_sync(cmd: &str, options: Option<SpawnOptions>) -> NodeResult<Output> {
    #[cfg(unix)]
    let (shell, flag) = ("sh", "-c");
    #[cfg(windows)]
    let (shell, flag) = ("cmd", "/C");

    spawn_sync(shell, &[flag, cmd], options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn() {
        #[cfg(unix)]
        let output = spawn("echo", &["hello"], None).await.unwrap();
        #[cfg(windows)]
        let output = spawn("cmd", &["/C", "echo", "hello"], None).await.unwrap();

        assert_eq!(output.status, 0);
        assert!(String::from_utf8_lossy(&output.stdout).contains("hello"));
    }

    #[test]
    fn test_spawn_sync() {
        #[cfg(unix)]
        let output = spawn_sync("echo", &["hello"], None).unwrap();
        #[cfg(windows)]
        let output = spawn_sync("cmd", &["/C", "echo", "hello"], None).unwrap();

        assert_eq!(output.status, 0);
    }
}
