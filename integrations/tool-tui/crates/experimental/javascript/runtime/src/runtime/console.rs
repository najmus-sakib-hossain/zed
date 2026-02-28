//! Complete Console API implementation with timers, counters, and groups
//!
//! Implements the full console API as specified in the WHATWG Console Standard:
//! - console.time/timeEnd/timeLog for performance timing
//! - console.count/countReset for call counting
//! - console.group/groupEnd for output grouping
//! - console.assert for assertions

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::value::Value;

/// Global console state for timers, counters, and groups
#[derive(Default)]
pub struct ConsoleState {
    /// Active timers: label -> start time
    timers: HashMap<String, Instant>,
    /// Call counters: label -> count
    counters: HashMap<String, u32>,
    /// Current indentation level for groups
    indent_level: u32,
}

impl ConsoleState {
    /// Create a new console state
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
            counters: HashMap::new(),
            indent_level: 0,
        }
    }

    /// Get the current indentation string
    fn indent(&self) -> String {
        "  ".repeat(self.indent_level as usize)
    }

    /// console.time(label) - Start a timer
    pub fn time(&mut self, label: &str) {
        let label = if label.is_empty() { "default" } else { label };
        if self.timers.contains_key(label) {
            eprintln!("{}Timer '{}' already exists", self.indent(), label);
            return;
        }
        self.timers.insert(label.to_string(), Instant::now());
    }

    /// console.timeEnd(label) - Stop timer and log elapsed time
    pub fn time_end(&mut self, label: &str) -> Option<f64> {
        let label = if label.is_empty() { "default" } else { label };
        if let Some(start) = self.timers.remove(label) {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            println!("{}{}: {:.3}ms", self.indent(), label, elapsed);
            Some(elapsed)
        } else {
            eprintln!("{}Timer '{}' does not exist", self.indent(), label);
            None
        }
    }

    /// console.timeLog(label, ...data) - Log elapsed time without stopping
    pub fn time_log(&self, label: &str, data: &[Value]) -> Option<f64> {
        let label = if label.is_empty() { "default" } else { label };
        if let Some(start) = self.timers.get(label) {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            if data.is_empty() {
                println!("{}{}: {:.3}ms", self.indent(), label, elapsed);
            } else {
                let data_str: Vec<String> = data.iter().map(|v| format!("{}", v)).collect();
                println!("{}{}: {:.3}ms {}", self.indent(), label, elapsed, data_str.join(" "));
            }
            Some(elapsed)
        } else {
            eprintln!("{}Timer '{}' does not exist", self.indent(), label);
            None
        }
    }

    /// console.count(label) - Increment and log counter
    pub fn count(&mut self, label: &str) -> u32 {
        let label = if label.is_empty() { "default" } else { label };
        let count = self.counters.entry(label.to_string()).or_insert(0);
        *count += 1;
        let current_count = *count;
        let indent = self.indent();
        println!("{}{}: {}", indent, label, current_count);
        current_count
    }

    /// console.countReset(label) - Reset counter
    pub fn count_reset(&mut self, label: &str) {
        let label = if label.is_empty() { "default" } else { label };
        if self.counters.remove(label).is_none() {
            eprintln!("{}Count for '{}' does not exist", self.indent(), label);
        }
    }

    /// console.group(label) - Increase indentation
    pub fn group(&mut self, label: Option<&str>) {
        if let Some(l) = label {
            println!("{}{}", self.indent(), l);
        }
        self.indent_level += 1;
    }

    /// console.groupCollapsed(label) - Same as group (collapsed is a browser feature)
    pub fn group_collapsed(&mut self, label: Option<&str>) {
        self.group(label);
    }

    /// console.groupEnd() - Decrease indentation
    pub fn group_end(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// console.assert(condition, ...data) - Log if condition is false
    pub fn assert(&self, condition: bool, data: &[Value]) {
        if !condition {
            let indent = self.indent();
            if data.is_empty() {
                eprintln!("{}Assertion failed", indent);
            } else {
                let data_str: Vec<String> = data.iter().map(|v| format!("{}", v)).collect();
                eprintln!("{}Assertion failed: {}", indent, data_str.join(" "));
            }
        }
    }

    /// console.log(...data) - Log with current indentation
    pub fn log(&self, args: &[Value]) {
        let indent = self.indent();
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            } else {
                print!("{}", indent);
            }
            print!("{}", arg);
        }
        println!();
    }

    /// console.warn(...data) - Log warning with current indentation
    pub fn warn(&self, args: &[Value]) {
        let indent = self.indent();
        eprint!("{}⚠️  ", indent);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                eprint!(" ");
            }
            eprint!("{}", arg);
        }
        eprintln!();
    }

    /// console.error(...data) - Log error with current indentation
    pub fn error(&self, args: &[Value]) {
        let indent = self.indent();
        eprint!("{}❌ ", indent);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                eprint!(" ");
            }
            eprint!("{}", arg);
        }
        eprintln!();
    }

    /// console.info(...data) - Log info with current indentation
    pub fn info(&self, args: &[Value]) {
        let indent = self.indent();
        print!("{}ℹ️  ", indent);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", arg);
        }
        println!();
    }

    /// console.debug(...data) - Log debug with current indentation
    pub fn debug(&self, args: &[Value]) {
        let indent = self.indent();
        print!("{}🔍 ", indent);
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", arg);
        }
        println!();
    }

    /// console.table(data) - Display data in tabular format
    pub fn table(&self, data: &Value) {
        let indent = self.indent();
        match data {
            Value::Array(arr) => {
                if arr.is_empty() {
                    println!("{}(empty array)", indent);
                    return;
                }
                // Print header
                println!("{}┌─────────┬────────────────────┐", indent);
                println!("{}│ (index) │       Value        │", indent);
                println!("{}├─────────┼────────────────────┤", indent);
                for (i, val) in arr.iter().enumerate() {
                    let val_str = format!("{}", val);
                    let truncated = if val_str.len() > 18 {
                        format!("{}...", &val_str[..15])
                    } else {
                        val_str
                    };
                    println!("{}│ {:>7} │ {:^18} │", indent, i, truncated);
                }
                println!("{}└─────────┴────────────────────┘", indent);
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    println!("{}(empty object)", indent);
                    return;
                }
                println!("{}┌────────────────────┬────────────────────┐", indent);
                println!("{}│       (key)        │       Value        │", indent);
                println!("{}├────────────────────┼────────────────────┤", indent);
                for (key, val) in obj.entries() {
                    let key_str = if key.len() > 18 {
                        format!("{}...", &key[..15])
                    } else {
                        key.clone()
                    };
                    let val_str = format!("{}", val);
                    let val_truncated = if val_str.len() > 18 {
                        format!("{}...", &val_str[..15])
                    } else {
                        val_str
                    };
                    println!("{}│ {:^18} │ {:^18} │", indent, key_str, val_truncated);
                }
                println!("{}└────────────────────┴────────────────────┘", indent);
            }
            _ => {
                println!("{}{}", indent, data);
            }
        }
    }

    /// console.clear() - Clear console (print newlines in terminal)
    pub fn clear(&self) {
        // ANSI escape code to clear screen
        print!("\x1B[2J\x1B[1;1H");
    }

    /// console.dir(obj) - Display object properties
    pub fn dir(&self, obj: &Value) {
        let indent = self.indent();
        match obj {
            Value::Object(map) => {
                println!("{}{{", indent);
                for (key, val) in map.entries() {
                    println!("{}  {}: {}", indent, key, val);
                }
                println!("{}}}", indent);
            }
            _ => println!("{}{}", indent, obj),
        }
    }

    /// Get the current timer value without logging (for testing)
    pub fn get_timer_elapsed(&self, label: &str) -> Option<f64> {
        let label = if label.is_empty() { "default" } else { label };
        self.timers.get(label).map(|start| start.elapsed().as_secs_f64() * 1000.0)
    }

    /// Get the current counter value (for testing)
    pub fn get_counter(&self, label: &str) -> Option<u32> {
        let label = if label.is_empty() { "default" } else { label };
        self.counters.get(label).copied()
    }

    /// Get the current indent level (for testing)
    pub fn get_indent_level(&self) -> u32 {
        self.indent_level
    }
}

// Thread-local console state for zero-lock access in single-threaded contexts
thread_local! {
    static CONSOLE_STATE: std::cell::RefCell<ConsoleState> =
        std::cell::RefCell::new(ConsoleState::new());
}

/// Global console state for multi-threaded access - reserved for worker thread console
#[allow(dead_code)]
static GLOBAL_CONSOLE: std::sync::OnceLock<Arc<RwLock<ConsoleState>>> = std::sync::OnceLock::new();

/// Reserved for multi-threaded console access
#[allow(dead_code)]
fn get_global_console() -> &'static Arc<RwLock<ConsoleState>> {
    GLOBAL_CONSOLE.get_or_init(|| Arc::new(RwLock::new(ConsoleState::new())))
}

// Public API functions that use thread-local state

/// console.time(label)
pub fn console_time(label: &str) {
    CONSOLE_STATE.with(|c| c.borrow_mut().time(label));
}

/// console.timeEnd(label)
pub fn console_time_end(label: &str) -> Option<f64> {
    CONSOLE_STATE.with(|c| c.borrow_mut().time_end(label))
}

/// console.timeLog(label, ...data)
pub fn console_time_log(label: &str, data: &[Value]) -> Option<f64> {
    CONSOLE_STATE.with(|c| c.borrow().time_log(label, data))
}

/// console.count(label)
pub fn console_count(label: &str) -> u32 {
    CONSOLE_STATE.with(|c| c.borrow_mut().count(label))
}

/// console.countReset(label)
pub fn console_count_reset(label: &str) {
    CONSOLE_STATE.with(|c| c.borrow_mut().count_reset(label));
}

/// console.group(label)
pub fn console_group(label: Option<&str>) {
    CONSOLE_STATE.with(|c| c.borrow_mut().group(label));
}

/// console.groupCollapsed(label)
pub fn console_group_collapsed(label: Option<&str>) {
    CONSOLE_STATE.with(|c| c.borrow_mut().group_collapsed(label));
}

/// console.groupEnd()
pub fn console_group_end() {
    CONSOLE_STATE.with(|c| c.borrow_mut().group_end());
}

/// console.assert(condition, ...data)
pub fn console_assert(condition: bool, data: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().assert(condition, data));
}

/// console.log(...data)
pub fn console_log(args: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().log(args));
}

/// console.warn(...data)
pub fn console_warn(args: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().warn(args));
}

/// console.error(...data)
pub fn console_error(args: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().error(args));
}

/// console.info(...data)
pub fn console_info(args: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().info(args));
}

/// console.debug(...data)
pub fn console_debug(args: &[Value]) {
    CONSOLE_STATE.with(|c| c.borrow().debug(args));
}

/// console.table(data)
pub fn console_table(data: &Value) {
    CONSOLE_STATE.with(|c| c.borrow().table(data));
}

/// console.clear()
pub fn console_clear() {
    CONSOLE_STATE.with(|c| c.borrow().clear());
}

/// console.dir(obj)
pub fn console_dir(obj: &Value) {
    CONSOLE_STATE.with(|c| c.borrow().dir(obj));
}

/// Get timer elapsed (for testing)
pub fn get_timer_elapsed(label: &str) -> Option<f64> {
    CONSOLE_STATE.with(|c| c.borrow().get_timer_elapsed(label))
}

/// Get counter value (for testing)
pub fn get_counter(label: &str) -> Option<u32> {
    CONSOLE_STATE.with(|c| c.borrow().get_counter(label))
}

/// Get indent level (for testing)
pub fn get_indent_level() -> u32 {
    CONSOLE_STATE.with(|c| c.borrow().get_indent_level())
}

/// Reset console state (for testing)
pub fn reset_console_state() {
    CONSOLE_STATE.with(|c| *c.borrow_mut() = ConsoleState::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_basic() {
        reset_console_state();
        console_time("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = console_time_end("test");
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= 10.0);
    }

    #[test]
    fn test_timer_default_label() {
        reset_console_state();
        console_time("");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let elapsed = get_timer_elapsed("");
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= 5.0);
        console_time_end("");
    }

    #[test]
    fn test_counter_basic() {
        reset_console_state();
        assert_eq!(console_count("test"), 1);
        assert_eq!(console_count("test"), 2);
        assert_eq!(console_count("test"), 3);
        assert_eq!(get_counter("test"), Some(3));
    }

    #[test]
    fn test_counter_reset() {
        reset_console_state();
        console_count("test");
        console_count("test");
        console_count_reset("test");
        assert_eq!(get_counter("test"), None);
        assert_eq!(console_count("test"), 1);
    }

    #[test]
    fn test_group_indent() {
        reset_console_state();
        assert_eq!(get_indent_level(), 0);
        console_group(Some("Group 1"));
        assert_eq!(get_indent_level(), 1);
        console_group(Some("Group 2"));
        assert_eq!(get_indent_level(), 2);
        console_group_end();
        assert_eq!(get_indent_level(), 1);
        console_group_end();
        assert_eq!(get_indent_level(), 0);
    }

    #[test]
    fn test_group_end_at_zero() {
        reset_console_state();
        console_group_end(); // Should not go negative
        assert_eq!(get_indent_level(), 0);
    }
}
