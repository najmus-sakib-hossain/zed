//! Debugging support for DX-Py runtime
//!
//! Provides line number tracking, stack traces, and debugger protocol support.
//! Implements Python-compatible traceback capture and formatting.

use crate::pyframe::PyFrame;
use std::fmt;
use std::sync::Arc;

/// Line number table entry
#[derive(Debug, Clone, Copy)]
pub struct LineEntry {
    /// Bytecode offset start
    pub start_offset: u32,
    /// Bytecode offset end (exclusive)
    pub end_offset: u32,
    /// Source line number
    pub line: u32,
}

/// Line number table for mapping bytecode to source lines
#[derive(Debug, Clone, Default)]
pub struct LineTable {
    entries: Vec<LineEntry>,
}

impl LineTable {
    /// Create a new empty line table
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a line entry
    pub fn add(&mut self, start_offset: u32, end_offset: u32, line: u32) {
        self.entries.push(LineEntry {
            start_offset,
            end_offset,
            line,
        });
    }

    /// Get line number for a bytecode offset
    pub fn get_line(&self, offset: u32) -> Option<u32> {
        for entry in &self.entries {
            if offset >= entry.start_offset && offset < entry.end_offset {
                return Some(entry.line);
            }
        }
        None
    }

    /// Get all entries
    pub fn entries(&self) -> &[LineEntry] {
        &self.entries
    }

    /// Build from compressed format (offset, line pairs)
    pub fn from_pairs(pairs: &[(u32, u32)]) -> Self {
        let mut table = Self::new();
        for i in 0..pairs.len() {
            let (offset, line) = pairs[i];
            let end_offset = if i + 1 < pairs.len() {
                pairs[i + 1].0
            } else {
                u32::MAX
            };
            table.add(offset, end_offset, line);
        }
        table
    }
}

/// A single frame in a traceback
#[derive(Debug, Clone)]
pub struct TracebackFrame {
    /// Function name
    pub func_name: String,
    /// Module name
    pub module: Option<String>,
    /// File name
    pub filename: Option<String>,
    /// Line number
    pub lineno: u32,
    /// Instruction pointer
    pub ip: usize,
    /// Source code line (if available)
    pub source_line: Option<String>,
    /// Column offset (if available)
    pub col_offset: Option<u32>,
}

impl TracebackFrame {
    /// Create from a PyFrame
    pub fn from_frame(frame: &PyFrame) -> Self {
        Self {
            func_name: frame.func_name().to_string(),
            module: frame.module_name().map(|s| s.to_string()),
            filename: frame.function.filename.clone(),
            lineno: frame.lineno,
            ip: frame.ip,
            source_line: None,
            col_offset: None,
        }
    }

    /// Create a new traceback frame with all fields
    pub fn new(func_name: impl Into<String>, filename: Option<String>, lineno: u32) -> Self {
        Self {
            func_name: func_name.into(),
            module: None,
            filename,
            lineno,
            ip: 0,
            source_line: None,
            col_offset: None,
        }
    }

    /// Set the source line
    pub fn with_source_line(mut self, line: impl Into<String>) -> Self {
        self.source_line = Some(line.into());
        self
    }

    /// Set the module name
    pub fn with_module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Set the column offset
    pub fn with_col_offset(mut self, col: u32) -> Self {
        self.col_offset = Some(col);
        self
    }

    /// Try to load the source line from the file
    pub fn load_source_line(&mut self) {
        if self.source_line.is_some() {
            return;
        }
        if let Some(ref filename) = self.filename {
            if let Ok(content) = std::fs::read_to_string(filename) {
                if let Some(line) = content.lines().nth(self.lineno.saturating_sub(1) as usize) {
                    self.source_line = Some(line.to_string());
                }
            }
        }
    }
}

impl fmt::Display for TracebackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filename = self.filename.as_deref().unwrap_or("<unknown>");
        write!(f, "  File \"{}\", line {}, in {}", filename, self.lineno, self.func_name)?;

        // Include source line if available (Python-style formatting)
        if let Some(ref source) = self.source_line {
            let trimmed = source.trim();
            if !trimmed.is_empty() {
                write!(f, "\n    {}", trimmed)?;
            }
        }
        Ok(())
    }
}

/// Stack trace (traceback)
#[derive(Debug, Clone, Default)]
pub struct Traceback {
    frames: Vec<TracebackFrame>,
}

impl Traceback {
    /// Create a new empty traceback
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    /// Create traceback from a frame chain
    pub fn from_frame(frame: Arc<PyFrame>) -> Self {
        let mut tb = Self::new();
        let mut current = Some(frame);

        while let Some(f) = current {
            tb.frames.push(TracebackFrame::from_frame(&f));
            current = f.back.clone();
        }

        // Reverse so innermost frame is last (Python convention)
        tb.frames.reverse();
        tb
    }

    /// Create traceback from a frame chain, capturing source lines
    pub fn from_frame_with_source(frame: Arc<PyFrame>) -> Self {
        let mut tb = Self::from_frame(frame);
        for frame in &mut tb.frames {
            frame.load_source_line();
        }
        tb
    }

    /// Add a frame to the traceback
    pub fn push(&mut self, frame: TracebackFrame) {
        self.frames.push(frame);
    }

    /// Add a frame at the beginning (for building traceback during unwinding)
    pub fn push_front(&mut self, frame: TracebackFrame) {
        self.frames.insert(0, frame);
    }

    /// Get all frames
    pub fn frames(&self) -> &[TracebackFrame] {
        &self.frames
    }

    /// Get mutable access to frames
    pub fn frames_mut(&mut self) -> &mut [TracebackFrame] {
        &mut self.frames
    }

    /// Get the innermost (most recent) frame
    pub fn innermost(&self) -> Option<&TracebackFrame> {
        self.frames.last()
    }

    /// Get the outermost (oldest) frame
    pub fn outermost(&self) -> Option<&TracebackFrame> {
        self.frames.first()
    }

    /// Get depth (number of frames)
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check if traceback is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Extend this traceback with another
    pub fn extend(&mut self, other: &Traceback) {
        self.frames.extend(other.frames.iter().cloned());
    }

    /// Limit the traceback to a maximum number of frames
    pub fn limit(&mut self, max_frames: usize) {
        if self.frames.len() > max_frames {
            let removed = self.frames.len() - max_frames;
            self.frames.drain(0..removed);
        }
    }
}

impl fmt::Display for Traceback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Traceback (most recent call last):")?;
        for frame in &self.frames {
            writeln!(f, "{}", frame)?;
        }
        Ok(())
    }
}

/// Exception with traceback
#[derive(Debug, Clone)]
pub struct ExceptionInfo {
    /// Exception type name
    pub exc_type: String,
    /// Exception message
    pub message: String,
    /// Traceback
    pub traceback: Traceback,
}

impl ExceptionInfo {
    /// Create a new exception info
    pub fn new(exc_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            exc_type: exc_type.into(),
            message: message.into(),
            traceback: Traceback::new(),
        }
    }

    /// Create with traceback from frame
    pub fn with_frame(
        exc_type: impl Into<String>,
        message: impl Into<String>,
        frame: Arc<PyFrame>,
    ) -> Self {
        Self {
            exc_type: exc_type.into(),
            message: message.into(),
            traceback: Traceback::from_frame(frame),
        }
    }

    /// Format as Python-style exception
    pub fn format(&self) -> String {
        format!("{}{}: {}", self.traceback, self.exc_type, self.message)
    }
}

impl fmt::Display for ExceptionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Debugger breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    /// Unique ID
    pub id: u32,
    /// File path
    pub filename: String,
    /// Line number
    pub line: u32,
    /// Condition (optional)
    pub condition: Option<String>,
    /// Hit count
    pub hit_count: u32,
    /// Enabled flag
    pub enabled: bool,
}

impl Breakpoint {
    /// Create a new breakpoint
    pub fn new(id: u32, filename: impl Into<String>, line: u32) -> Self {
        Self {
            id,
            filename: filename.into(),
            line,
            condition: None,
            hit_count: 0,
            enabled: true,
        }
    }

    /// Check if breakpoint matches a location
    pub fn matches(&self, filename: &str, line: u32) -> bool {
        self.enabled && self.line == line && self.filename == filename
    }
}

/// Debugger state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    /// Running normally
    Running,
    /// Paused at breakpoint
    Paused,
    /// Stepping into
    StepInto,
    /// Stepping over
    StepOver,
    /// Stepping out
    StepOut,
}

/// Simple debugger for the runtime
#[derive(Debug)]
pub struct Debugger {
    /// Current state
    state: DebugState,
    /// Breakpoints
    breakpoints: Vec<Breakpoint>,
    /// Next breakpoint ID
    next_bp_id: u32,
    /// Step depth (for step over/out)
    step_depth: usize,
}

impl Debugger {
    /// Create a new debugger
    pub fn new() -> Self {
        Self {
            state: DebugState::Running,
            breakpoints: Vec::new(),
            next_bp_id: 1,
            step_depth: 0,
        }
    }

    /// Get current state
    pub fn state(&self) -> DebugState {
        self.state
    }

    /// Add a breakpoint
    pub fn add_breakpoint(&mut self, filename: impl Into<String>, line: u32) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint::new(id, filename, line));
        id
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(pos) = self.breakpoints.iter().position(|bp| bp.id == id) {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    /// Enable/disable a breakpoint
    pub fn set_breakpoint_enabled(&mut self, id: u32, enabled: bool) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Check if we should break at a location
    pub fn should_break(&mut self, filename: &str, line: u32, depth: usize) -> bool {
        // Check stepping state
        match self.state {
            DebugState::StepInto => {
                self.state = DebugState::Paused;
                return true;
            }
            DebugState::StepOver if depth <= self.step_depth => {
                self.state = DebugState::Paused;
                return true;
            }
            DebugState::StepOut if depth < self.step_depth => {
                self.state = DebugState::Paused;
                return true;
            }
            _ => {}
        }

        // Check breakpoints
        for bp in &mut self.breakpoints {
            if bp.matches(filename, line) {
                bp.hit_count += 1;
                self.state = DebugState::Paused;
                return true;
            }
        }

        false
    }

    /// Continue execution
    pub fn continue_execution(&mut self) {
        self.state = DebugState::Running;
    }

    /// Step into
    pub fn step_into(&mut self) {
        self.state = DebugState::StepInto;
    }

    /// Step over
    pub fn step_over(&mut self, current_depth: usize) {
        self.state = DebugState::StepOver;
        self.step_depth = current_depth;
    }

    /// Step out
    pub fn step_out(&mut self, current_depth: usize) {
        self.state = DebugState::StepOut;
        self.step_depth = current_depth;
    }

    /// Get all breakpoints
    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_table() {
        let mut table = LineTable::new();
        table.add(0, 10, 1);
        table.add(10, 20, 2);
        table.add(20, 30, 3);

        assert_eq!(table.get_line(0), Some(1));
        assert_eq!(table.get_line(5), Some(1));
        assert_eq!(table.get_line(10), Some(2));
        assert_eq!(table.get_line(25), Some(3));
        assert_eq!(table.get_line(100), None);
    }

    #[test]
    fn test_line_table_from_pairs() {
        let pairs = vec![(0, 1), (10, 2), (20, 3)];
        let table = LineTable::from_pairs(&pairs);

        assert_eq!(table.get_line(5), Some(1));
        assert_eq!(table.get_line(15), Some(2));
        assert_eq!(table.get_line(25), Some(3));
    }

    #[test]
    fn test_traceback_frame_display() {
        let frame = TracebackFrame::new("test_func", Some("test.py".to_string()), 42)
            .with_module("test_module");

        let display = format!("{}", frame);
        assert!(display.contains("test.py"));
        assert!(display.contains("42"));
        assert!(display.contains("test_func"));
    }

    #[test]
    fn test_traceback_frame_with_source() {
        let frame = TracebackFrame::new("test_func", Some("test.py".to_string()), 42)
            .with_source_line("    x = 1 + 2");

        let display = format!("{}", frame);
        assert!(display.contains("test.py"));
        assert!(display.contains("x = 1 + 2"));
    }

    #[test]
    fn test_traceback_display() {
        let mut tb = Traceback::new();
        tb.push(TracebackFrame::new("outer", Some("test.py".to_string()), 10));
        tb.push(TracebackFrame::new("inner", Some("test.py".to_string()), 20));

        let display = format!("{}", tb);
        assert!(display.contains("Traceback"));
        assert!(display.contains("outer"));
        assert!(display.contains("inner"));
    }

    #[test]
    fn test_traceback_push_front() {
        let mut tb = Traceback::new();
        tb.push(TracebackFrame::new("inner", Some("test.py".to_string()), 20));
        tb.push_front(TracebackFrame::new("outer", Some("test.py".to_string()), 10));

        assert_eq!(tb.frames()[0].func_name, "outer");
        assert_eq!(tb.frames()[1].func_name, "inner");
    }

    #[test]
    fn test_traceback_limit() {
        let mut tb = Traceback::new();
        for i in 0..10 {
            tb.push(TracebackFrame::new(
                format!("func_{}", i),
                Some("test.py".to_string()),
                i as u32,
            ));
        }

        assert_eq!(tb.depth(), 10);
        tb.limit(5);
        assert_eq!(tb.depth(), 5);
        // Should keep the most recent frames
        assert_eq!(tb.frames()[0].func_name, "func_5");
    }

    #[test]
    fn test_exception_info() {
        let exc = ExceptionInfo::new("ValueError", "invalid value");
        assert_eq!(exc.exc_type, "ValueError");
        assert_eq!(exc.message, "invalid value");
    }

    #[test]
    fn test_debugger_breakpoints() {
        let mut dbg = Debugger::new();

        let id1 = dbg.add_breakpoint("test.py", 10);
        let _id2 = dbg.add_breakpoint("test.py", 20);

        assert_eq!(dbg.breakpoints().len(), 2);

        assert!(dbg.should_break("test.py", 10, 0));
        assert_eq!(dbg.state(), DebugState::Paused);

        dbg.continue_execution();
        assert_eq!(dbg.state(), DebugState::Running);

        assert!(dbg.remove_breakpoint(id1));
        assert!(!dbg.should_break("test.py", 10, 0));
    }

    #[test]
    fn test_debugger_stepping() {
        let mut dbg = Debugger::new();

        dbg.step_into();
        assert_eq!(dbg.state(), DebugState::StepInto);

        // Should break on next instruction
        assert!(dbg.should_break("test.py", 1, 0));
        assert_eq!(dbg.state(), DebugState::Paused);
    }

    #[test]
    fn test_debugger_step_over() {
        let mut dbg = Debugger::new();

        dbg.step_over(1);
        assert_eq!(dbg.state(), DebugState::StepOver);

        // Should not break at deeper level
        assert!(!dbg.should_break("test.py", 1, 2));

        // Should break at same or shallower level
        assert!(dbg.should_break("test.py", 1, 1));
    }
}
