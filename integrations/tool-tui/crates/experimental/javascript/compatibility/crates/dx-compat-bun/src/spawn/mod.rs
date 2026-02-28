//! Bun.spawn() process spawning.
//!
//! High-performance subprocess spawning targeting 10,000+ spawns/second.

use crate::error::{BunError, BunResult};
use bytes::Bytes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Stdio configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StdioConfig {
    /// Pipe stdio for reading/writing
    #[default]
    Pipe,
    /// Inherit from parent process
    Inherit,
    /// Ignore (null)
    Ignore,
}

impl From<StdioConfig> for Stdio {
    fn from(config: StdioConfig) -> Self {
        match config {
            StdioConfig::Pipe => Stdio::piped(),
            StdioConfig::Inherit => Stdio::inherit(),
            StdioConfig::Ignore => Stdio::null(),
        }
    }
}

/// Spawn options.
#[derive(Debug, Clone, Default)]
pub struct SpawnOptions {
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Stdin configuration
    pub stdin: StdioConfig,
    /// Stdout configuration
    pub stdout: StdioConfig,
    /// Stderr configuration
    pub stderr: StdioConfig,
    /// Clear environment before adding env vars
    pub clear_env: bool,
}

impl SpawnOptions {
    /// Create new spawn options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set working directory.
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Set environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.get_or_insert_with(HashMap::new).insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables.
    pub fn envs(mut self, vars: impl IntoIterator<Item = (String, String)>) -> Self {
        self.env.get_or_insert_with(HashMap::new).extend(vars);
        self
    }

    /// Set stdin configuration.
    pub fn stdin(mut self, config: StdioConfig) -> Self {
        self.stdin = config;
        self
    }

    /// Set stdout configuration.
    pub fn stdout(mut self, config: StdioConfig) -> Self {
        self.stdout = config;
        self
    }

    /// Set stderr configuration.
    pub fn stderr(mut self, config: StdioConfig) -> Self {
        self.stderr = config;
        self
    }
}

/// Subprocess handle for async process management.
pub struct Subprocess {
    /// Process ID
    pub pid: u32,
    /// Child process handle
    child: tokio::process::Child,
    /// Stdin writer (if piped)
    stdin: Option<tokio::process::ChildStdin>,
    /// Stdout reader (if piped)
    stdout: Option<tokio::process::ChildStdout>,
    /// Stderr reader (if piped)
    stderr: Option<tokio::process::ChildStderr>,
}

impl Subprocess {
    /// Wait for the process to exit and return exit code.
    pub async fn exited(&mut self) -> BunResult<ExitStatus> {
        let status = self
            .child
            .wait()
            .await
            .map_err(|e| BunError::Spawn(format!("Wait failed: {}", e)))?;

        Ok(ExitStatus {
            code: status.code(),
            success: status.success(),
            #[cfg(unix)]
            signal: std::os::unix::process::ExitStatusExt::signal(&status),
            #[cfg(not(unix))]
            signal: None,
        })
    }

    /// Kill the process.
    pub async fn kill(&mut self) -> BunResult<()> {
        self.child
            .kill()
            .await
            .map_err(|e| BunError::Spawn(format!("Kill failed: {}", e)))
    }

    /// Send a signal to the process (Unix only).
    #[cfg(unix)]
    pub fn signal(&self, signal: i32) -> BunResult<()> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let sig = Signal::try_from(signal)
            .map_err(|e| BunError::Spawn(format!("Invalid signal: {}", e)))?;
        kill(Pid::from_raw(self.pid as i32), sig)
            .map_err(|e| BunError::Spawn(format!("Signal failed: {}", e)))
    }

    /// Write to stdin.
    pub async fn write_stdin(&mut self, data: &[u8]) -> BunResult<()> {
        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(data)
                .await
                .map_err(|e| BunError::Spawn(format!("Stdin write failed: {}", e)))?;
            stdin
                .flush()
                .await
                .map_err(|e| BunError::Spawn(format!("Stdin flush failed: {}", e)))?;
        }
        Ok(())
    }

    /// Close stdin.
    pub fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Read all stdout.
    pub async fn read_stdout(&mut self) -> BunResult<Bytes> {
        if let Some(stdout) = &mut self.stdout {
            let mut buffer = Vec::new();
            stdout
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| BunError::Spawn(format!("Stdout read failed: {}", e)))?;
            Ok(Bytes::from(buffer))
        } else {
            Ok(Bytes::new())
        }
    }

    /// Read all stderr.
    pub async fn read_stderr(&mut self) -> BunResult<Bytes> {
        if let Some(stderr) = &mut self.stderr {
            let mut buffer = Vec::new();
            stderr
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| BunError::Spawn(format!("Stderr read failed: {}", e)))?;
            Ok(Bytes::from(buffer))
        } else {
            Ok(Bytes::new())
        }
    }

    /// Read stdout line by line.
    pub async fn read_stdout_lines(&mut self) -> BunResult<Vec<String>> {
        if let Some(stdout) = self.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut result = Vec::new();
            while let Some(line) = lines
                .next_line()
                .await
                .map_err(|e| BunError::Spawn(format!("Read line failed: {}", e)))?
            {
                result.push(line);
            }
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Exit status from a subprocess.
#[derive(Debug, Clone)]
pub struct ExitStatus {
    /// Exit code (None if terminated by signal)
    pub code: Option<i32>,
    /// Whether the process exited successfully
    pub success: bool,
    /// Signal that terminated the process (Unix only)
    pub signal: Option<i32>,
}

impl ExitStatus {
    /// Get exit code, defaulting to -1 if terminated by signal.
    pub fn code_or_default(&self) -> i32 {
        self.code.unwrap_or(-1)
    }
}

/// Synchronous subprocess result.
#[derive(Debug, Clone)]
pub struct SyncSubprocess {
    /// Exit status
    pub status: ExitStatus,
    /// Stdout bytes
    pub stdout: Bytes,
    /// Stderr bytes
    pub stderr: Bytes,
}

impl SyncSubprocess {
    /// Get stdout as string.
    pub fn stdout_text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.stdout.to_vec())
    }

    /// Get stderr as string.
    pub fn stderr_text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.stderr.to_vec())
    }

    /// Parse stdout as JSON.
    pub fn stdout_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.stdout)
    }
}

/// Spawn async subprocess (Bun.spawn()).
///
/// # Arguments
/// * `cmd` - Command and arguments as a slice
/// * `options` - Optional spawn configuration
///
/// # Example
/// ```ignore
/// let mut proc = spawn(&["echo", "hello"], None).await?;
/// let status = proc.exited().await?;
/// ```
pub async fn spawn(cmd: &[&str], options: Option<SpawnOptions>) -> BunResult<Subprocess> {
    if cmd.is_empty() {
        return Err(BunError::Spawn("Empty command".to_string()));
    }

    let options = options.unwrap_or_default();
    let mut command = Command::new(cmd[0]);

    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }

    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }

    if options.clear_env {
        command.env_clear();
    }

    if let Some(env) = &options.env {
        command.envs(env);
    }

    command.stdin(options.stdin);
    command.stdout(options.stdout);
    command.stderr(options.stderr);

    let mut child = command.spawn().map_err(|e| BunError::Spawn(format!("Spawn failed: {}", e)))?;

    let pid = child.id().unwrap_or(0);
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    Ok(Subprocess {
        pid,
        child,
        stdin,
        stdout,
        stderr,
    })
}

/// Spawn sync subprocess (Bun.spawnSync()).
///
/// # Arguments
/// * `cmd` - Command and arguments as a slice
/// * `options` - Optional spawn configuration
///
/// # Example
/// ```ignore
/// let result = spawn_sync(&["echo", "hello"], None)?;
/// println!("Output: {}", result.stdout_text().unwrap());
/// ```
pub fn spawn_sync(cmd: &[&str], options: Option<SpawnOptions>) -> BunResult<SyncSubprocess> {
    if cmd.is_empty() {
        return Err(BunError::Spawn("Empty command".to_string()));
    }

    let options = options.unwrap_or_default();
    let mut command = std::process::Command::new(cmd[0]);

    if cmd.len() > 1 {
        command.args(&cmd[1..]);
    }

    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }

    if options.clear_env {
        command.env_clear();
    }

    if let Some(env) = &options.env {
        command.envs(env);
    }

    // For sync, we always capture output
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.output().map_err(|e| BunError::Spawn(format!("Spawn failed: {}", e)))?;

    Ok(SyncSubprocess {
        status: ExitStatus {
            code: output.status.code(),
            success: output.status.success(),
            #[cfg(unix)]
            signal: std::os::unix::process::ExitStatusExt::signal(&output.status),
            #[cfg(not(unix))]
            signal: None,
        },
        stdout: Bytes::from(output.stdout),
        stderr: Bytes::from(output.stderr),
    })
}

/// Execute a command in a shell.
///
/// # Arguments
/// * `cmd` - Shell command string
/// * `options` - Optional spawn configuration
pub fn exec_sync(cmd: &str, options: Option<SpawnOptions>) -> BunResult<SyncSubprocess> {
    #[cfg(windows)]
    let shell_cmd = ["cmd", "/C", cmd];
    #[cfg(not(windows))]
    let shell_cmd = ["sh", "-c", cmd];

    spawn_sync(&shell_cmd, options)
}

/// Execute a command in a shell asynchronously.
pub async fn exec(cmd: &str, options: Option<SpawnOptions>) -> BunResult<Subprocess> {
    #[cfg(windows)]
    let shell_cmd = ["cmd", "/C", cmd];
    #[cfg(not(windows))]
    let shell_cmd = ["sh", "-c", cmd];

    spawn(&shell_cmd, options).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_options_builder() {
        let options = SpawnOptions::new()
            .cwd("/tmp")
            .env("FOO", "bar")
            .stdin(StdioConfig::Pipe)
            .stdout(StdioConfig::Pipe);

        assert_eq!(options.cwd, Some(PathBuf::from("/tmp")));
        assert_eq!(options.env.as_ref().unwrap().get("FOO"), Some(&"bar".to_string()));
        assert_eq!(options.stdin, StdioConfig::Pipe);
        assert_eq!(options.stdout, StdioConfig::Pipe);
    }

    #[test]
    fn test_spawn_sync_echo() {
        #[cfg(windows)]
        let result = spawn_sync(&["cmd", "/C", "echo hello"], None);
        #[cfg(not(windows))]
        let result = spawn_sync(&["echo", "hello"], None);

        let result = result.unwrap();
        assert!(result.status.success);
        let stdout = result.stdout_text().unwrap();
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn test_spawn_sync_with_env() {
        #[cfg(windows)]
        let result = spawn_sync(
            &["cmd", "/C", "echo %TEST_VAR%"],
            Some(SpawnOptions::new().env("TEST_VAR", "test_value")),
        );
        #[cfg(not(windows))]
        let result = spawn_sync(
            &["sh", "-c", "echo $TEST_VAR"],
            Some(SpawnOptions::new().env("TEST_VAR", "test_value")),
        );

        let result = result.unwrap();
        assert!(result.status.success);
        let stdout = result.stdout_text().unwrap();
        assert!(stdout.contains("test_value"));
    }

    #[test]
    fn test_exec_sync() {
        let result = exec_sync("echo hello", None).unwrap();
        assert!(result.status.success);
        let stdout = result.stdout_text().unwrap();
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_spawn_async() {
        #[cfg(windows)]
        let mut proc = spawn(&["cmd", "/C", "echo hello"], None).await.unwrap();
        #[cfg(not(windows))]
        let mut proc = spawn(&["echo", "hello"], None).await.unwrap();

        let status = proc.exited().await.unwrap();
        assert!(status.success);
    }

    #[test]
    fn test_exit_status() {
        let status = ExitStatus {
            code: Some(0),
            success: true,
            signal: None,
        };
        assert_eq!(status.code_or_default(), 0);

        let status = ExitStatus {
            code: None,
            success: false,
            signal: Some(9),
        };
        assert_eq!(status.code_or_default(), -1);
    }
}
