//! Bun runtime integration
//!
//! Uses system-installed Bun for maximum performance and npm compatibility

use super::{JsResult, Runtime};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
// Unused: use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tokio::sync::Mutex;

/// Bun JavaScript runtime
pub struct BunRuntime {
    process: Mutex<Option<BunProcess>>,
}

struct BunProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl BunRuntime {
    /// Create new Bun runtime instance
    pub fn new() -> Result<Self> {
        if !Self::is_available() {
            return Err(anyhow!("Bun is not installed or not in PATH"));
        }

        Ok(Self {
            process: Mutex::new(None),
        })
    }

    /// Start the Bun process
    async fn ensure_process(&self) -> Result<()> {
        let mut guard = self.process.lock().await;

        if guard.is_none() {
            let process = Self::spawn_process()?;
            *guard = Some(process);
        }

        Ok(())
    }

    fn spawn_process() -> Result<BunProcess> {
        let mut child = Command::new("bun")
            .arg("run")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn Bun process")?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout =
            BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?);

        Ok(BunProcess {
            child,
            stdin,
            stdout,
        })
    }

    /// Execute code in Bun process
    async fn execute_in_process(&self, code: &str) -> Result<JsResult> {
        self.ensure_process().await?;

        let mut guard = self.process.lock().await;
        let process = guard.as_mut().ok_or_else(|| anyhow!("Process not initialized"))?;

        // Wrap code in try-catch and JSON output
        let wrapped = format!(
            r#"
            (async () => {{
                try {{
                    const result = await (async () => {{ {} }})();
                    console.log(JSON.stringify({{ success: true, data: result }}));
                }} catch (error) {{
                    console.log(JSON.stringify({{ 
                        success: false, 
                        error: error.message,
                        stack: error.stack 
                    }}));
                }}
            }})();
            "#,
            code
        );

        // Send code to Bun
        writeln!(process.stdin, "{}", wrapped)?;
        process.stdin.flush()?;

        // Read response
        let mut response = String::new();
        process.stdout.read_line(&mut response)?;

        // Parse JSON response
        let result: JsResult =
            serde_json::from_str(&response).context("Failed to parse Bun response")?;

        Ok(result)
    }
}

#[async_trait::async_trait]
impl Runtime for BunRuntime {
    async fn eval(&mut self, code: &str) -> Result<JsResult> {
        self.execute_in_process(code).await
    }

    async fn call(&mut self, function: &str, args: Value) -> Result<JsResult> {
        let code = format!("return {}({})", function, serde_json::to_string(&args)?);
        self.execute_in_process(&code).await
    }

    fn is_available() -> bool {
        which::which("bun").is_ok()
    }

    fn name(&self) -> &str {
        "bun"
    }
}

impl Drop for BunRuntime {
    fn drop(&mut self) {
        // Kill process on drop
        if let Ok(mut guard) = self.process.try_lock() {
            if let Some(mut process) = guard.take() {
                let _ = process.child.kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bun_available() {
        assert!(BunRuntime::is_available(), "Bun should be installed");
    }

    #[tokio::test]
    async fn test_bun_eval() {
        if !BunRuntime::is_available() {
            return;
        }

        let mut runtime = BunRuntime::new().unwrap();
        let result = runtime.eval("return 2 + 2").await.unwrap();

        assert!(result.success);
        assert_eq!(result.data, json!(4));
    }
}
