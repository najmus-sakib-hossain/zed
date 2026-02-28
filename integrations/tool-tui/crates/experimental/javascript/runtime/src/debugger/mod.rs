//! Debugger support with source maps and Chrome DevTools Protocol

pub mod cdp;
pub mod sourcemap;

pub use cdp::CdpServer;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub enabled: bool,
}

pub struct Debugger {
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    paused: bool,
    /// Current stack frame - reserved for step debugging
    #[allow(dead_code)]
    current_frame: Option<StackFrame>,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            paused: false,
            current_frame: None,
        }
    }

    pub fn set_breakpoint(&mut self, file: String, line: usize, column: usize) {
        self.breakpoints.entry(file.clone()).or_default().push(Breakpoint {
            file,
            line,
            column,
            enabled: true,
        });
    }

    pub fn remove_breakpoint(&mut self, file: &str, line: usize) {
        if let Some(bps) = self.breakpoints.get_mut(file) {
            bps.retain(|bp| bp.line != line);
        }
    }

    pub fn should_break(&self, file: &str, line: usize) -> bool {
        self.breakpoints
            .get(file)
            .is_some_and(|bps| bps.iter().any(|bp| bp.enabled && bp.line == line))
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }
    pub fn resume(&mut self) {
        self.paused = false;
    }
    pub fn step_over(&mut self) {}
    pub fn step_into(&mut self) {}
    pub fn step_out(&mut self) {}

    pub fn get_variables(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}
