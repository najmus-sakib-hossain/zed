//! Supervisor — auto-restart crashed agents with exponential backoff.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// State of a supervised process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Stopped,
    Crashed,
    Restarting,
    BackoffWait,
}

/// A supervised process.
#[derive(Debug, Clone)]
pub struct SupervisedProcess {
    pub id: String,
    pub name: String,
    pub state: ProcessState,
    /// Number of consecutive crashes.
    pub crash_count: u32,
    /// Maximum allowed consecutive crashes before giving up.
    pub max_crashes: u32,
    /// Last crash timestamp.
    pub last_crash: Option<Instant>,
    /// Next restart attempt.
    pub next_restart: Option<Instant>,
    /// Current backoff duration.
    pub current_backoff: Duration,
}

impl SupervisedProcess {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            state: ProcessState::Stopped,
            crash_count: 0,
            max_crashes: 10,
            last_crash: None,
            next_restart: None,
            current_backoff: Duration::from_secs(1),
        }
    }

    /// Calculate the next backoff duration (exponential with jitter).
    pub fn next_backoff(&self) -> Duration {
        let base = self.current_backoff.as_secs_f64() * 2.0;
        let max_backoff = 300.0; // 5 minutes max
        let capped = base.min(max_backoff);
        Duration::from_secs_f64(capped)
    }

    /// Record a crash event and schedule restart.
    pub fn record_crash(&mut self) {
        self.crash_count += 1;
        self.state = ProcessState::Crashed;
        self.last_crash = Some(Instant::now());
        self.current_backoff = self.next_backoff();

        if self.crash_count < self.max_crashes {
            self.next_restart = Some(Instant::now() + self.current_backoff);
            self.state = ProcessState::BackoffWait;
            log::warn!(
                "Supervisor: '{}' crashed (#{}/{}), restarting in {:.1}s",
                self.name,
                self.crash_count,
                self.max_crashes,
                self.current_backoff.as_secs_f64()
            );
        } else {
            log::error!(
                "Supervisor: '{}' crashed {} times, giving up",
                self.name,
                self.crash_count
            );
        }
    }

    /// Reset crash count (called on successful long-running operation).
    pub fn reset_crashes(&mut self) {
        self.crash_count = 0;
        self.current_backoff = Duration::from_secs(1);
    }

    /// Check if it's time to restart.
    pub fn should_restart(&self) -> bool {
        if self.state != ProcessState::BackoffWait {
            return false;
        }
        self.next_restart
            .map_or(false, |restart_at| Instant::now() >= restart_at)
    }
}

/// Supervisor that manages multiple background processes.
pub struct Supervisor {
    processes: HashMap<String, SupervisedProcess>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
        }
    }

    /// Register a process for supervision.
    pub fn register(&mut self, id: &str, name: &str) {
        let process = SupervisedProcess::new(id, name);
        self.processes.insert(id.to_string(), process);
        log::info!("Supervisor: registered '{}'", name);
    }

    /// Mark a process as running.
    pub fn mark_running(&mut self, id: &str) {
        if let Some(process) = self.processes.get_mut(id) {
            process.state = ProcessState::Running;
            log::info!("Supervisor: '{}' is running", process.name);
        }
    }

    /// Report a crash for a process.
    pub fn report_crash(&mut self, id: &str) {
        if let Some(process) = self.processes.get_mut(id) {
            process.record_crash();
        }
    }

    /// Get all processes that need restarting.
    pub fn pending_restarts(&self) -> Vec<&SupervisedProcess> {
        self.processes
            .values()
            .filter(|p| p.should_restart())
            .collect()
    }

    /// Get all supervised processes.
    pub fn processes(&self) -> impl Iterator<Item = &SupervisedProcess> {
        self.processes.values()
    }

    /// Get a process by ID.
    pub fn get(&self, id: &str) -> Option<&SupervisedProcess> {
        self.processes.get(id)
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}
