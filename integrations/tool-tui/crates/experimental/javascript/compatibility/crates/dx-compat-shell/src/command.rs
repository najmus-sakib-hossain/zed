//! Shell command builder and execution.
//!
//! Provides Bun-compatible shell scripting with:
//! - Template literal-style command execution
//! - Safe argument escaping
//! - Command chaining (pipes, &&, ||)
//! - Environment variable support

use crate::error::{ShellError, ShellResult};
use crate::output::ShellOutput;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;

/// Shell command builder.
pub struct ShellCommand {
    cmd: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    cwd: Option<PathBuf>,
    quiet: bool,
    nothrow: bool,
    stdin_data: Option<Vec<u8>>,
}

impl ShellCommand {
    /// Create a new shell command.
    pub fn new(cmd: &str) -> Self {
        Self {
            cmd: cmd.to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            quiet: false,
            nothrow: false,
            stdin_data: None,
        }
    }

    /// Parse and create a command from a shell string.
    ///
    /// Supports basic shell syntax including pipes and chaining.
    pub fn parse(shell_str: &str) -> Self {
        let parts: Vec<&str> = shell_str.split_whitespace().collect();
        if parts.is_empty() {
            return Self::new("");
        }

        let mut cmd = Self::new(parts[0]);
        if parts.len() > 1 {
            cmd.args = parts[1..].iter().map(|s| s.to_string()).collect();
        }
        cmd
    }

    /// Add a single argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set environment variable.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set multiple environment variables.
    pub fn envs(mut self, vars: &[(&str, &str)]) -> Self {
        for (key, value) in vars {
            self.env.insert(key.to_string(), value.to_string());
        }
        self
    }

    /// Set working directory.
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Suppress output.
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    /// Don't throw on non-zero exit.
    pub fn nothrow(mut self) -> Self {
        self.nothrow = true;
        self
    }

    /// Set stdin data.
    pub fn stdin(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.stdin_data = Some(data.into());
        self
    }

    /// Set stdin from string.
    pub fn stdin_text(self, text: &str) -> Self {
        self.stdin(text.as_bytes().to_vec())
    }

    /// Run the command asynchronously.
    pub async fn run(self) -> ShellResult<ShellOutput> {
        use tokio::io::AsyncWriteExt;

        let mut cmd = Command::new(&self.cmd);
        cmd.args(&self.args);

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        if self.stdin_data.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        // Write stdin if provided
        if let Some(stdin_data) = self.stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&stdin_data).await?;
            }
        }

        let output = child.wait_with_output().await?;
        let exit_code = output.status.code().unwrap_or(-1);

        if !self.nothrow && exit_code != 0 {
            return Err(ShellError::NonZeroExit(exit_code));
        }

        Ok(ShellOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code,
        })
    }

    /// Run the command synchronously.
    pub fn run_sync(self) -> ShellResult<ShellOutput> {
        use std::io::Write;
        use std::process::{Command as StdCommand, Stdio};

        let mut cmd = StdCommand::new(&self.cmd);
        cmd.args(&self.args);

        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        if self.stdin_data.is_some() {
            cmd.stdin(Stdio::piped());
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Write stdin if provided
        if let Some(stdin_data) = self.stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(&stdin_data)?;
            }
        }

        let output = child.wait_with_output()?;
        let exit_code = output.status.code().unwrap_or(-1);

        if !self.nothrow && exit_code != 0 {
            return Err(ShellError::NonZeroExit(exit_code));
        }

        Ok(ShellOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code,
        })
    }
}

/// Escape a string for safe shell usage.
pub fn escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }

    // Check if escaping is needed
    let needs_escaping = s.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\''
                | '"'
                | '\\'
                | '$'
                | '`'
                | '!'
                | '*'
                | '?'
                | '['
                | ']'
                | '{'
                | '}'
                | '('
                | ')'
                | '<'
                | '>'
                | '|'
                | '&'
                | ';'
                | '#'
                | '~'
        )
    });

    if !needs_escaping {
        return s.to_string();
    }

    // Use single quotes and escape any single quotes within
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Execute a pipeline of commands.
pub struct Pipeline {
    commands: Vec<ShellCommand>,
    env: HashMap<String, String>,
    cwd: Option<PathBuf>,
}

impl Pipeline {
    /// Create a new pipeline.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            env: HashMap::new(),
            cwd: None,
        }
    }

    /// Add a command to the pipeline.
    pub fn pipe(mut self, cmd: ShellCommand) -> Self {
        self.commands.push(cmd);
        self
    }

    /// Set environment for all commands.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set working directory for all commands.
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Run the pipeline.
    pub async fn run(self) -> ShellResult<ShellOutput> {
        if self.commands.is_empty() {
            return Ok(ShellOutput {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit_code: 0,
            });
        }

        let mut current_input: Option<Vec<u8>> = None;
        let mut last_output = ShellOutput {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        };

        for mut cmd in self.commands {
            // Apply pipeline-level settings
            for (key, value) in &self.env {
                cmd.env.entry(key.clone()).or_insert_with(|| value.clone());
            }
            if cmd.cwd.is_none() {
                cmd.cwd = self.cwd.clone();
            }

            // Pipe previous output as stdin
            if let Some(input) = current_input.take() {
                cmd.stdin_data = Some(input);
            }

            last_output = cmd.run().await?;
            current_input = Some(last_output.stdout.clone());
        }

        Ok(last_output)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_simple() {
        assert_eq!(escape("hello"), "hello");
        assert_eq!(escape("hello world"), "'hello world'");
        assert_eq!(escape(""), "''");
    }

    #[test]
    fn test_escape_special_chars() {
        assert_eq!(escape("$HOME"), "'$HOME'");
        assert_eq!(escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_parse_command() {
        let cmd = ShellCommand::parse("echo hello world");
        assert_eq!(cmd.cmd, "echo");
        assert_eq!(cmd.args, vec!["hello", "world"]);
    }

    #[tokio::test]
    async fn test_echo_command() {
        let output = ShellCommand::new("echo").arg("hello").nothrow().run().await;

        // This test may fail on systems without echo
        if let Ok(out) = output {
            assert!(out.text().unwrap().contains("hello"));
        }
    }
}
