//! Enhanced REPL with multi-line input, tab completion, and debugging support
//!
//! Implements Requirements 12.1-12.7:
//! - Multi-line input for compound statements
//! - Tab completion for attributes and modules
//! - breakpoint() support
//! - Debugger commands (step, next, continue, etc.)

#![allow(dead_code)]

use dx_py_core::pylist::PyValue;
use dx_py_interpreter::VirtualMachine;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// REPL state
pub struct Repl {
    /// Virtual machine instance
    vm: VirtualMachine,
    /// Command history
    history: Vec<String>,
    /// Current history index
    history_index: usize,
    /// Multi-line input buffer
    input_buffer: String,
    /// Whether we're in multi-line mode
    in_multiline: bool,
    /// Current indentation level
    indent_level: usize,
    /// Debugger state
    debugger: DebuggerState,
    /// Verbose mode
    verbose: bool,
    /// Known names for completion
    known_names: Vec<String>,
}

/// Debugger state
#[derive(Default)]
pub struct DebuggerState {
    /// Whether debugger is active
    pub active: bool,
    /// Breakpoints (file:line -> enabled)
    pub breakpoints: HashMap<String, bool>,
    /// Current execution mode
    pub mode: DebugMode,
    /// Step count (for step N)
    pub step_count: usize,
}

/// Debug execution mode
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    /// Normal execution
    #[default]
    Run,
    /// Step into (execute one instruction)
    Step,
    /// Step over (execute until next line)
    Next,
    /// Continue until breakpoint
    Continue,
}

/// Result of checking if input is complete
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputStatus {
    /// Input is complete and can be executed
    Complete,
    /// Input is incomplete (needs more lines)
    Incomplete,
    /// Input has a syntax error
    Error,
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl Repl {
    /// Create a new REPL instance
    pub fn new() -> Self {
        Self {
            vm: VirtualMachine::new(),
            history: Vec::new(),
            history_index: 0,
            input_buffer: String::new(),
            in_multiline: false,
            indent_level: 0,
            debugger: DebuggerState::default(),
            verbose: false,
            known_names: Self::builtin_names(),
        }
    }

    /// Create a REPL with verbose mode
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get builtin names for completion
    fn builtin_names() -> Vec<String> {
        vec![
            // Type constructors
            "int",
            "float",
            "str",
            "bool",
            "list",
            "dict",
            "tuple",
            "set",
            "frozenset",
            "bytes",
            "bytearray",
            "object",
            "type",
            // Built-in functions
            "print",
            "len",
            "range",
            "enumerate",
            "zip",
            "map",
            "filter",
            "sorted",
            "reversed",
            "iter",
            "next",
            "all",
            "any",
            "sum",
            "min",
            "max",
            "abs",
            "round",
            "pow",
            "divmod",
            "isinstance",
            "issubclass",
            "hasattr",
            "getattr",
            "setattr",
            "delattr",
            "callable",
            "dir",
            "vars",
            "globals",
            "locals",
            "id",
            "hash",
            "repr",
            "ascii",
            "chr",
            "ord",
            "hex",
            "bin",
            "oct",
            "open",
            "input",
            "exec",
            "eval",
            "compile",
            "format",
            "help",
            "exit",
            "quit",
            // Keywords
            "True",
            "False",
            "None",
            "and",
            "or",
            "not",
            "is",
            "in",
            "if",
            "elif",
            "else",
            "for",
            "while",
            "break",
            "continue",
            "def",
            "class",
            "return",
            "yield",
            "lambda",
            "try",
            "except",
            "finally",
            "raise",
            "assert",
            "import",
            "from",
            "as",
            "with",
            "pass",
            "del",
            "global",
            "nonlocal",
            "async",
            "await",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Run the REPL loop
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("DX-Py {} (Interactive Mode)", env!("CARGO_PKG_VERSION"));
        println!("Type 'exit()' or Ctrl+D to quit, 'help()' for help");
        println!();

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            // Print prompt
            let prompt = if self.in_multiline { "... " } else { ">>> " };
            print!("{}", prompt);
            stdout.flush()?;

            // Read line
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    // EOF
                    println!();
                    break;
                }
                Ok(_) => {
                    if let Some(should_exit) = self.process_line(&line)? {
                        if should_exit {
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process a single line of input
    /// Returns Some(true) to exit, Some(false) to continue, None for normal operation
    pub fn process_line(&mut self, line: &str) -> Result<Option<bool>, Box<dyn std::error::Error>> {
        let line_trimmed = line.trim_end_matches('\n').trim_end_matches('\r');

        // Handle empty line in multi-line mode
        if self.in_multiline && line_trimmed.is_empty() {
            // Empty line ends multi-line input
            let input = std::mem::take(&mut self.input_buffer);
            self.in_multiline = false;
            self.indent_level = 0;

            if !input.trim().is_empty() {
                self.execute_input(&input)?;
            }
            return Ok(None);
        }

        // Handle empty line in normal mode
        if line_trimmed.is_empty() && !self.in_multiline {
            return Ok(None);
        }

        // Add to input buffer
        if self.in_multiline {
            self.input_buffer.push_str(line);
        } else {
            self.input_buffer = line.to_string();
        }

        // Check if input is complete
        match self.check_input_complete(&self.input_buffer) {
            InputStatus::Complete => {
                let input = std::mem::take(&mut self.input_buffer);
                self.in_multiline = false;
                self.indent_level = 0;

                // Handle special commands
                let trimmed = input.trim();
                match trimmed {
                    "exit()" | "quit()" => return Ok(Some(true)),
                    "help()" => {
                        self.print_help();
                        return Ok(None);
                    }
                    _ => {}
                }

                // Handle debugger commands if in debug mode
                if self.debugger.active && self.handle_debugger_command(trimmed) {
                    return Ok(None);
                }

                self.execute_input(&input)?;
            }
            InputStatus::Incomplete => {
                self.in_multiline = true;
                self.indent_level = self.calculate_indent(&self.input_buffer);
            }
            InputStatus::Error => {
                // Try to execute anyway to get error message
                let input = std::mem::take(&mut self.input_buffer);
                self.in_multiline = false;
                self.indent_level = 0;
                self.execute_input(&input)?;
            }
        }

        Ok(None)
    }

    /// Check if input is complete (can be executed)
    fn check_input_complete(&self, input: &str) -> InputStatus {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return InputStatus::Complete;
        }

        // Count brackets and parentheses
        let mut paren_depth = 0i32;
        let mut bracket_depth = 0i32;
        let mut brace_depth = 0i32;
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_triple_string = false;
        let mut prev_char = ' ';
        let mut _prev_prev_char = ' ';

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Handle string literals
            if !in_string {
                if c == '"' || c == '\'' {
                    // Check for triple quotes
                    if i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c {
                        in_string = true;
                        in_triple_string = true;
                        string_char = c;
                        i += 3;
                        continue;
                    } else {
                        in_string = true;
                        in_triple_string = false;
                        string_char = c;
                        i += 1;
                        continue;
                    }
                }
            } else {
                // In string - check for end
                if in_triple_string {
                    if c == string_char
                        && i + 2 < chars.len()
                        && chars[i + 1] == string_char
                        && chars[i + 2] == string_char
                        && prev_char != '\\'
                    {
                        in_string = false;
                        in_triple_string = false;
                        i += 3;
                        continue;
                    }
                } else if c == string_char && prev_char != '\\' {
                    in_string = false;
                    i += 1;
                    continue;
                }
                _prev_prev_char = prev_char;
                prev_char = c;
                i += 1;
                continue;
            }

            // Skip comments
            if c == '#' {
                // Skip to end of line
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            // Count brackets
            match c {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }

            _prev_prev_char = prev_char;
            prev_char = c;
            i += 1;
        }

        // Unclosed string
        if in_string {
            return InputStatus::Incomplete;
        }

        // Unclosed brackets
        if paren_depth > 0 || bracket_depth > 0 || brace_depth > 0 {
            return InputStatus::Incomplete;
        }

        // Negative depth means syntax error
        if paren_depth < 0 || bracket_depth < 0 || brace_depth < 0 {
            return InputStatus::Error;
        }

        // Check for compound statements that need continuation
        let last_line = input.lines().last().unwrap_or("");
        let last_trimmed = last_line.trim();

        // Ends with colon - needs more input
        if last_trimmed.ends_with(':') {
            return InputStatus::Incomplete;
        }

        // Ends with backslash - line continuation
        if last_trimmed.ends_with('\\') {
            return InputStatus::Incomplete;
        }

        // Check if we're in an indented block
        if self.in_multiline {
            let current_indent = self.get_line_indent(last_line);
            if current_indent > 0 || last_trimmed.is_empty() {
                // Still in block, but empty line ends it
                if last_trimmed.is_empty() {
                    return InputStatus::Complete;
                }
                return InputStatus::Incomplete;
            }
        }

        InputStatus::Complete
    }

    /// Calculate expected indentation level
    fn calculate_indent(&self, input: &str) -> usize {
        let mut indent: usize = 0;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.ends_with(':') {
                indent += 1;
            }
            // Dedent on return, break, continue, pass, raise
            if (trimmed.starts_with("return")
                || trimmed.starts_with("break")
                || trimmed.starts_with("continue")
                || trimmed.starts_with("pass")
                || trimmed.starts_with("raise"))
                && indent > 0
            {
                indent = indent.saturating_sub(1);
            }
        }

        indent
    }

    /// Get indentation of a line (number of leading spaces / 4)
    fn get_line_indent(&self, line: &str) -> usize {
        let spaces = line.len() - line.trim_start().len();
        spaces / 4
    }

    /// Execute input and display result
    fn execute_input(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Ok(());
        }

        // Add to history
        if !trimmed.is_empty() && self.history.last().map(|s| s.as_str()) != Some(trimmed) {
            self.history.push(trimmed.to_string());
            self.history_index = self.history.len();
        }

        if self.verbose {
            println!("[eval] {}", trimmed);
        }

        // Check for breakpoint()
        if trimmed.contains("breakpoint()") {
            self.enter_debugger();
            return Ok(());
        }

        match self.vm.eval_expr(trimmed) {
            Ok(result) => {
                if !matches!(result, PyValue::None) {
                    println!("{}", self.format_value(&result));
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }

        Ok(())
    }

    /// Enter debugger mode
    fn enter_debugger(&mut self) {
        self.debugger.active = true;
        self.debugger.mode = DebugMode::Step;
        println!("Entering debugger...");
        println!("Type 'help' for debugger commands");
    }

    /// Handle debugger command
    /// Returns true if command was handled
    fn handle_debugger_command(&mut self, cmd: &str) -> bool {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return false;
        }

        match parts[0] {
            "s" | "step" => {
                self.debugger.mode = DebugMode::Step;
                self.debugger.step_count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                println!("Stepping {} instruction(s)", self.debugger.step_count);
                true
            }
            "n" | "next" => {
                self.debugger.mode = DebugMode::Next;
                println!("Stepping to next line");
                true
            }
            "c" | "continue" => {
                self.debugger.mode = DebugMode::Continue;
                println!("Continuing execution");
                true
            }
            "q" | "quit" => {
                self.debugger.active = false;
                self.debugger.mode = DebugMode::Run;
                println!("Exiting debugger");
                true
            }
            "p" | "print" => {
                if parts.len() > 1 {
                    let expr = parts[1..].join(" ");
                    match self.vm.eval_expr(&expr) {
                        Ok(result) => println!("{}", self.format_value(&result)),
                        Err(e) => eprintln!("{}", e),
                    }
                } else {
                    println!("Usage: print <expression>");
                }
                true
            }
            "l" | "locals" => {
                println!("Local variables:");
                // Would display locals from current frame
                println!("  (locals display not yet implemented)");
                true
            }
            "w" | "where" | "bt" => {
                println!("Stack trace:");
                // Would display call stack
                println!("  (stack trace not yet implemented)");
                true
            }
            "b" | "break" => {
                if parts.len() > 1 {
                    let location = parts[1];
                    self.debugger.breakpoints.insert(location.to_string(), true);
                    println!("Breakpoint set at {}", location);
                } else {
                    println!("Breakpoints:");
                    for (loc, enabled) in &self.debugger.breakpoints {
                        println!("  {} ({})", loc, if *enabled { "enabled" } else { "disabled" });
                    }
                }
                true
            }
            "clear" => {
                if parts.len() > 1 {
                    let location = parts[1];
                    self.debugger.breakpoints.remove(location);
                    println!("Breakpoint cleared at {}", location);
                } else {
                    self.debugger.breakpoints.clear();
                    println!("All breakpoints cleared");
                }
                true
            }
            "help" => {
                self.print_debugger_help();
                true
            }
            _ => false,
        }
    }

    /// Print debugger help
    fn print_debugger_help(&self) {
        println!("Debugger Commands:");
        println!("  s, step [N]     - Step N instructions (default 1)");
        println!("  n, next         - Step to next line (step over)");
        println!("  c, continue     - Continue until breakpoint");
        println!("  q, quit         - Exit debugger");
        println!("  p, print <expr> - Print expression value");
        println!("  l, locals       - Show local variables");
        println!("  w, where, bt    - Show stack trace");
        println!("  b, break [loc]  - Set/list breakpoints");
        println!("  clear [loc]     - Clear breakpoint(s)");
        println!("  help            - Show this help");
    }

    /// Get tab completions for input
    pub fn get_completions(&self, input: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Get the word being completed
        let word = input
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .next_back()
            .unwrap_or("");

        if word.is_empty() {
            return completions;
        }

        // Check for attribute completion (contains '.')
        if word.contains('.') {
            // Attribute completion
            let parts: Vec<&str> = word.rsplitn(2, '.').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                // Would get attributes of the object
                // For now, return common attributes
                let common_attrs = ["__class__", "__doc__", "__dict__", "__module__", "__name__"];
                for attr in common_attrs {
                    if attr.starts_with(prefix) {
                        completions.push(attr.to_string());
                    }
                }
            }
        } else {
            // Name completion
            for name in &self.known_names {
                if name.starts_with(word) {
                    completions.push(name.clone());
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    /// Print REPL help
    fn print_help(&self) {
        println!("DX-Py Interactive Help");
        println!("=====================");
        println!();
        println!("Available built-in functions:");
        println!("  print(...)  - Print values to stdout");
        println!("  len(x)      - Return length of x");
        println!("  type(x)     - Return type name of x");
        println!("  int(x)      - Convert x to integer");
        println!("  float(x)    - Convert x to float");
        println!("  str(x)      - Convert x to string");
        println!("  bool(x)     - Convert x to boolean");
        println!("  abs(x)      - Return absolute value of x");
        println!("  min(...)    - Return minimum value");
        println!("  max(...)    - Return maximum value");
        println!("  sum(x)      - Return sum of iterable x");
        println!("  range(...)  - Return range as list");
        println!();
        println!("Multi-line input:");
        println!("  - Compound statements (if, for, def, class) continue on next line");
        println!("  - Empty line ends multi-line input");
        println!("  - Use \\ for explicit line continuation");
        println!();
        println!("Debugging:");
        println!("  - Use breakpoint() to enter debugger");
        println!("  - Type 'help' in debugger for commands");
        println!();
        println!("Special commands:");
        println!("  exit()      - Exit the REPL");
        println!("  quit()      - Exit the REPL");
        println!("  help()      - Show this help");
    }

    /// Format a value for display
    #[allow(clippy::only_used_in_recursion)]
    fn format_value(&self, value: &PyValue) -> String {
        match value {
            PyValue::None => "None".to_string(),
            PyValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            PyValue::Int(i) => i.to_string(),
            PyValue::Float(f) => format!("{}", f),
            PyValue::Str(s) => format!("'{}'", s),
            PyValue::List(l) => {
                let items: Vec<String> = l.to_vec().iter().map(|v| self.format_value(v)).collect();
                format!("[{}]", items.join(", "))
            }
            PyValue::Tuple(t) => {
                let items: Vec<String> = t.to_vec().iter().map(|v| self.format_value(v)).collect();
                if items.len() == 1 {
                    format!("({},)", items[0])
                } else {
                    format!("({})", items.join(", "))
                }
            }
            PyValue::Dict(d) => {
                let items: Vec<String> = d
                    .items()
                    .iter()
                    .map(|(k, v)| format!("{:?}: {}", k, self.format_value(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            PyValue::Exception(e) => {
                format!("{}: {}", e.exc_type, e.message)
            }
            PyValue::Type(t) => format!("<class '{}'>", t.name),
            PyValue::Instance(i) => format!("<{} instance>", i.class.name),
            PyValue::BoundMethod(_) => "<bound method>".to_string(),
            PyValue::Generator(_) => "<generator object>".to_string(),
            PyValue::Coroutine(_) => "<coroutine object>".to_string(),
            PyValue::Builtin(b) => format!("<built-in function {}>", b.name),
            PyValue::Function(f) => format!("<function {}>", f.name),
            PyValue::Iterator(_) => "<iterator>".to_string(),
            PyValue::Set(s) => {
                let items: Vec<String> = s.to_vec().iter().map(|v| self.format_value(v)).collect();
                format!("{{{}}}", items.join(", "))
            }
            PyValue::Module(m) => format!("<module '{}'>", m.name),
            PyValue::Code(c) => format!("<code object {} at {:p}>", c.name, c),
            PyValue::Cell(c) => format!("<cell: {}>", self.format_value(&c.get())),
            PyValue::Super(_) => "<super>".to_string(),
            PyValue::Property(_) => "<property>".to_string(),
            PyValue::StaticMethod(_) => "<staticmethod>".to_string(),
            PyValue::ClassMethod(_) => "<classmethod>".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_complete_simple() {
        let repl = Repl::new();
        assert_eq!(repl.check_input_complete("1 + 2"), InputStatus::Complete);
        assert_eq!(repl.check_input_complete("print('hello')"), InputStatus::Complete);
    }

    #[test]
    fn test_input_incomplete_brackets() {
        let repl = Repl::new();
        assert_eq!(repl.check_input_complete("[1, 2,"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("(1 + 2"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("{1: 2,"), InputStatus::Incomplete);
    }

    #[test]
    fn test_input_incomplete_colon() {
        let repl = Repl::new();
        assert_eq!(repl.check_input_complete("if True:"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("def foo():"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("for i in range(10):"), InputStatus::Incomplete);
    }

    #[test]
    fn test_input_incomplete_string() {
        let repl = Repl::new();
        assert_eq!(repl.check_input_complete("'hello"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("\"hello"), InputStatus::Incomplete);
        assert_eq!(repl.check_input_complete("'''hello"), InputStatus::Incomplete);
    }

    #[test]
    fn test_input_complete_multiline_string() {
        let repl = Repl::new();
        assert_eq!(repl.check_input_complete("'''hello'''"), InputStatus::Complete);
        assert_eq!(repl.check_input_complete("\"\"\"hello\"\"\""), InputStatus::Complete);
    }

    #[test]
    fn test_completions() {
        let repl = Repl::new();
        let completions = repl.get_completions("pri");
        assert!(completions.contains(&"print".to_string()));
    }

    #[test]
    fn test_debugger_commands() {
        let mut repl = Repl::new();
        repl.debugger.active = true;

        assert!(repl.handle_debugger_command("step"));
        assert!(repl.handle_debugger_command("next"));
        assert!(repl.handle_debugger_command("continue"));
        assert!(repl.handle_debugger_command("help"));
        assert!(!repl.handle_debugger_command("unknown_command"));
    }
}
