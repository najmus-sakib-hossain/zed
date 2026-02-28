//! Child Process API (exec, spawn, fork)

use crate::error::{DxError, DxResult};
use std::io::{Read, Write};
use std::process::{Child, Command, Output, Stdio};

pub struct ChildProcess;

impl ChildProcess {
    pub fn exec(command: &str) -> DxResult<Output> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", command]).output()
        } else {
            Command::new("sh").args(["-c", command]).output()
        };

        output.map_err(|e| DxError::RuntimeError(format!("exec failed: {}", e)))
    }

    pub fn spawn(command: &str, args: &[&str]) -> DxResult<SpawnedProcess> {
        let child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DxError::RuntimeError(format!("spawn failed: {}", e)))?;

        Ok(SpawnedProcess { child })
    }

    pub fn exec_sync(command: &str) -> DxResult<String> {
        let output = Self::exec(command)?;
        String::from_utf8(output.stdout)
            .map_err(|e| DxError::RuntimeError(format!("Invalid UTF-8: {}", e)))
    }
}

pub struct SpawnedProcess {
    child: Child,
}

impl SpawnedProcess {
    pub fn wait(&mut self) -> DxResult<i32> {
        let status = self
            .child
            .wait()
            .map_err(|e| DxError::RuntimeError(format!("wait failed: {}", e)))?;
        Ok(status.code().unwrap_or(-1))
    }

    pub fn kill(&mut self) -> DxResult<()> {
        self.child
            .kill()
            .map_err(|e| DxError::RuntimeError(format!("kill failed: {}", e)))
    }

    pub fn stdin(&mut self) -> Option<&mut impl Write> {
        self.child.stdin.as_mut()
    }

    pub fn stdout(&mut self) -> Option<&mut impl Read> {
        self.child.stdout.as_mut()
    }

    pub fn stderr(&mut self) -> Option<&mut impl Read> {
        self.child.stderr.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec() {
        let output = ChildProcess::exec("echo hello").unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn test_exec_sync() {
        let result = ChildProcess::exec_sync("echo test").unwrap();
        assert!(result.contains("test"));
    }
}
