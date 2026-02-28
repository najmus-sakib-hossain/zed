//! Worker process management with robust communication
//!
//! This module provides a TestWorker struct that manages communication
//! with Python worker processes using JSON-over-stdio protocol.

use crate::{DaemonConfig, WorkerState};
use dx_py_core::{AssertionStats, DaemonError, TestCase, TestResult, TestStatus};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Python worker script that runs in each daemon process
pub const WORKER_SCRIPT: &str = r#"
import sys
import json
import traceback
import time
import importlib.util
import io
from contextlib import redirect_stdout, redirect_stderr

def run_test(module_path, function_name, class_name=None):
    """Execute a single test function and return the result."""
    start_time = time.perf_counter_ns()
    stdout_capture = io.StringIO()
    stderr_capture = io.StringIO()
    
    try:
        # Load the module
        spec = importlib.util.spec_from_file_location("test_module", module_path)
        if spec is None or spec.loader is None:
            return {
                "status": "error",
                "message": f"Could not load module: {module_path}",
                "duration_ns": time.perf_counter_ns() - start_time,
                "stdout": "",
                "stderr": "",
                "traceback": None
            }
        
        module = importlib.util.module_from_spec(spec)
        sys.modules["test_module"] = module
        
        # Capture stdout/stderr during module loading and test execution
        with redirect_stdout(stdout_capture), redirect_stderr(stderr_capture):
            spec.loader.exec_module(module)
            
            # Get the test function
            if class_name:
                test_class = getattr(module, class_name)
                instance = test_class()
                test_func = getattr(instance, function_name)
            else:
                test_func = getattr(module, function_name)
            
            # Execute the test
            test_func()
        
        duration_ns = time.perf_counter_ns() - start_time
        return {
            "status": "pass",
            "duration_ns": duration_ns,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
            "traceback": None
        }
        
    except AssertionError as e:
        duration_ns = time.perf_counter_ns() - start_time
        tb = traceback.format_exc()
        return {
            "status": "fail",
            "message": str(e) if str(e) else "Assertion failed",
            "duration_ns": duration_ns,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
            "traceback": tb
        }
    except Exception as e:
        duration_ns = time.perf_counter_ns() - start_time
        tb = traceback.format_exc()
        return {
            "status": "error",
            "message": str(e),
            "duration_ns": duration_ns,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
            "traceback": tb
        }

def main():
    """Main worker loop - reads JSON commands from stdin, writes results to stdout."""
    # Disable buffering for reliable communication
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, line_buffering=True)
    
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                # EOF - parent closed stdin
                break
            
            line = line.strip()
            if not line:
                continue
                
            command = json.loads(line)
            
            if command.get("type") == "run":
                result = run_test(
                    command["module_path"],
                    command["function_name"],
                    command.get("class_name")
                )
                print(json.dumps(result), flush=True)
            elif command.get("type") == "ping":
                print(json.dumps({"status": "pong"}), flush=True)
            elif command.get("type") == "shutdown":
                print(json.dumps({"status": "shutdown_ack"}), flush=True)
                break
            else:
                print(json.dumps({"status": "error", "message": "Unknown command"}), flush=True)
                
        except json.JSONDecodeError as e:
            print(json.dumps({"status": "error", "message": f"Invalid JSON: {e}"}), flush=True)
        except Exception as e:
            print(json.dumps({"status": "error", "message": str(e), "traceback": traceback.format_exc()}), flush=True)

if __name__ == "__main__":
    main()
"#;

/// Request sent to worker process
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerRequest {
    #[serde(rename = "run")]
    Run {
        module_path: String,
        function_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        class_name: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Response from worker process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub duration_ns: Option<u64>,
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub stderr: Option<String>,
    #[serde(default)]
    pub traceback: Option<String>,
}

/// Result of reading from worker with timeout
#[allow(dead_code)]
enum ReadResult {
    Success(String),
    Timeout,
    Closed,
    Error(String),
}

/// A Python worker process that can execute tests
pub struct TestWorker {
    pub id: usize,
    process: Option<Child>,
    pub state: WorkerState,
    /// Number of times this worker has been restarted
    pub restart_count: u32,
    /// Last crash reason (if any)
    pub last_crash_reason: Option<String>,
    /// Channel for receiving responses with timeout
    response_rx: Option<Receiver<ReadResult>>,
    /// Thread handle for the reader thread
    reader_thread: Option<thread::JoinHandle<()>>,
}

impl TestWorker {
    /// Create a new worker (not yet spawned)
    pub fn new(id: usize) -> Self {
        Self {
            id,
            process: None,
            state: WorkerState::Idle,
            restart_count: 0,
            last_crash_reason: None,
            response_rx: None,
            reader_thread: None,
        }
    }

    /// Spawn the Python worker process
    pub fn spawn(&mut self, config: &DaemonConfig) -> Result<(), DaemonError> {
        // Clean up any existing process
        self.cleanup();

        let mut child = Command::new(&config.python_path)
            .arg("-c")
            .arg(WORKER_SCRIPT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DaemonError::StartupFailure(format!("Failed to spawn worker: {}", e)))?;

        // Take stdout for the reader thread
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DaemonError::StartupFailure("Failed to get stdout".to_string()))?;

        // Create channel for responses
        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx.send(ReadResult::Success(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(ReadResult::Error(e.to_string()));
                        break;
                    }
                }
            }
            let _ = tx.send(ReadResult::Closed);
        });

        self.reader_thread = Some(reader_thread);
        self.process = Some(child);
        self.state = WorkerState::Idle;
        self.last_crash_reason = None;

        Ok(())
    }

    /// Check if the worker is available for work
    pub fn is_available(&self) -> bool {
        self.state == WorkerState::Idle && self.process.is_some()
    }

    /// Mark the worker as busy
    pub fn mark_busy(&mut self) {
        self.state = WorkerState::Busy;
    }

    /// Mark the worker as idle
    pub fn mark_idle(&mut self) {
        self.state = WorkerState::Idle;
    }

    /// Mark the worker as crashed
    pub fn mark_crashed(&mut self) {
        self.state = WorkerState::Crashed;
    }

    /// Record a crash with reason
    pub fn record_crash(&mut self, reason: String) {
        self.state = WorkerState::Crashed;
        self.last_crash_reason = Some(reason);
        self.restart_count += 1;
    }

    /// Get the restart count
    pub fn get_restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Get the last crash reason
    pub fn get_last_crash_reason(&self) -> Option<&str> {
        self.last_crash_reason.as_deref()
    }

    /// Clean up the worker process
    fn cleanup(&mut self) {
        if let Some(ref mut process) = self.process {
            // Try to kill the process
            let _ = process.kill();
            let _ = process.wait();
        }
        self.process = None;
        self.response_rx = None;
        
        // Note: We don't join the reader thread here as it may be blocked
        // The thread will exit when the pipe is closed
        self.reader_thread = None;
    }

    /// Terminate the worker process
    pub fn terminate(&mut self) -> Result<(), DaemonError> {
        // Try to send shutdown command first
        if let Some(ref mut process) = self.process {
            if let Some(ref mut stdin) = process.stdin {
                let shutdown_cmd = serde_json::to_string(&WorkerRequest::Shutdown)
                    .unwrap_or_else(|_| r#"{"type": "shutdown"}"#.to_string());
                let _ = writeln!(stdin, "{}", shutdown_cmd);
                let _ = stdin.flush();
            }
            // Give it a moment to shut down gracefully
            thread::sleep(Duration::from_millis(100));
        }

        self.cleanup();
        self.state = WorkerState::Crashed;
        Ok(())
    }

    /// Send a request to the worker
    fn send_request(&mut self, request: &WorkerRequest) -> Result<(), DaemonError> {
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| DaemonError::WorkerCrash("Worker process not running".to_string()))?;

        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| DaemonError::WorkerCrash("Worker stdin not available".to_string()))?;

        let json = serde_json::to_string(request)
            .map_err(|e| DaemonError::WorkerCrash(format!("Failed to serialize request: {}", e)))?;

        writeln!(stdin, "{}", json)
            .map_err(|e| DaemonError::WorkerCrash(format!("Failed to write to worker: {}", e)))?;

        stdin
            .flush()
            .map_err(|e| DaemonError::WorkerCrash(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }

    /// Read response with timeout
    fn read_response(&mut self, timeout: Duration) -> Result<WorkerResponse, DaemonError> {
        let rx = self
            .response_rx
            .as_ref()
            .ok_or_else(|| DaemonError::WorkerCrash("Response channel not available".to_string()))?;

        match rx.recv_timeout(timeout) {
            Ok(ReadResult::Success(line)) => {
                serde_json::from_str(&line).map_err(|e| {
                    DaemonError::WorkerCrash(format!(
                        "Invalid JSON response: {}. Raw: {}",
                        e, line
                    ))
                })
            }
            Ok(ReadResult::Timeout) => Err(DaemonError::Timeout(timeout)),
            Ok(ReadResult::Closed) => {
                Err(DaemonError::WorkerCrash("Worker closed stdout".to_string()))
            }
            Ok(ReadResult::Error(e)) => {
                Err(DaemonError::WorkerCrash(format!("Read error: {}", e)))
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(DaemonError::Timeout(timeout)),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(DaemonError::WorkerCrash("Response channel disconnected".to_string()))
            }
        }
    }

    /// Execute a test in this worker
    pub fn execute_test(
        &mut self,
        test: &TestCase,
        timeout: Duration,
    ) -> Result<TestResult, DaemonError> {
        // Check worker health before executing
        self.check_health()?;

        let start = Instant::now();

        // Send the test execution request
        let request = WorkerRequest::Run {
            module_path: test.file_path.to_string_lossy().to_string(),
            function_name: test.name.clone(),
            class_name: test.class_name.clone(),
        };

        self.send_request(&request)?;

        // Read the response with timeout
        let response = self.read_response(timeout)?;

        let duration = response
            .duration_ns
            .map(Duration::from_nanos)
            .unwrap_or_else(|| start.elapsed());

        let status = match response.status.as_str() {
            "pass" => TestStatus::Pass,
            "fail" => TestStatus::Fail,
            "skip" => TestStatus::Skip {
                reason: response.message.clone().unwrap_or_default(),
            },
            "error" => TestStatus::Error {
                message: response.message.clone().unwrap_or_else(|| "Unknown error".to_string()),
            },
            _ => TestStatus::Error {
                message: format!("Unknown status from worker: {}", response.status),
            },
        };

        Ok(TestResult {
            test_id: test.id,
            status,
            duration,
            stdout: response.stdout.unwrap_or_default(),
            stderr: response.stderr.unwrap_or_default(),
            traceback: response.traceback,
            assertions: AssertionStats::default(),
            assertion_failure: None,
        })
    }

    /// Check if the worker is alive by sending a ping
    pub fn ping(&mut self, timeout: Duration) -> bool {
        if self.send_request(&WorkerRequest::Ping).is_err() {
            return false;
        }

        match self.read_response(timeout) {
            Ok(response) => response.status == "pong",
            Err(_) => false,
        }
    }

    /// Check if the worker process is still running
    pub fn is_process_alive(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(None) => true,  // Still running
                Ok(Some(status)) => {
                    // Process exited - record the crash
                    let reason = if let Some(code) = status.code() {
                        format!("Process exited with code {}", code)
                    } else {
                        "Process terminated by signal".to_string()
                    };
                    self.record_crash(reason);
                    false
                }
                Err(e) => {
                    self.record_crash(format!("Error checking process status: {}", e));
                    false
                }
            }
        } else {
            false
        }
    }

    /// Check worker health before executing a test
    /// Returns an error if the worker is not healthy
    pub fn check_health(&mut self) -> Result<(), DaemonError> {
        if self.process.is_none() {
            return Err(DaemonError::WorkerCrash("Worker process not running".to_string()));
        }

        if !self.is_process_alive() {
            return Err(DaemonError::WorkerCrash(
                self.last_crash_reason
                    .clone()
                    .unwrap_or_else(|| "Worker process died unexpectedly".to_string()),
            ));
        }

        Ok(())
    }
}

impl Drop for TestWorker {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_request_serialization() {
        let request = WorkerRequest::Run {
            module_path: "test.py".to_string(),
            function_name: "test_example".to_string(),
            class_name: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"run\""));
        assert!(json.contains("\"module_path\":\"test.py\""));
    }

    #[test]
    fn test_worker_request_with_class() {
        let request = WorkerRequest::Run {
            module_path: "test.py".to_string(),
            function_name: "test_method".to_string(),
            class_name: Some("TestClass".to_string()),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"class_name\":\"TestClass\""));
    }

    #[test]
    fn test_worker_response_deserialization() {
        let json = r#"{"status": "pass", "duration_ns": 1000000, "stdout": "", "stderr": ""}"#;
        let response: WorkerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "pass");
        assert_eq!(response.duration_ns, Some(1000000));
    }

    #[test]
    fn test_worker_response_with_error() {
        let json = r#"{"status": "error", "message": "Import failed", "traceback": "Traceback..."}"#;
        let response: WorkerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "error");
        assert_eq!(response.message, Some("Import failed".to_string()));
        assert!(response.traceback.is_some());
    }

    #[test]
    fn test_worker_state_transitions() {
        let mut worker = TestWorker::new(0);
        assert_eq!(worker.state, WorkerState::Idle);
        assert!(!worker.is_available()); // No process yet

        worker.mark_busy();
        assert_eq!(worker.state, WorkerState::Busy);

        worker.mark_idle();
        assert_eq!(worker.state, WorkerState::Idle);

        worker.mark_crashed();
        assert_eq!(worker.state, WorkerState::Crashed);
    }

    #[test]
    fn test_worker_crash_recording() {
        let mut worker = TestWorker::new(0);
        assert_eq!(worker.get_restart_count(), 0);
        assert!(worker.get_last_crash_reason().is_none());

        worker.record_crash("Test crash".to_string());
        assert_eq!(worker.get_restart_count(), 1);
        assert_eq!(worker.get_last_crash_reason(), Some("Test crash"));
        assert_eq!(worker.state, WorkerState::Crashed);

        worker.record_crash("Another crash".to_string());
        assert_eq!(worker.get_restart_count(), 2);
        assert_eq!(worker.get_last_crash_reason(), Some("Another crash"));
    }
}
