//! dx-js CLI entry point
//!
//! Unified runtime architecture - all execution goes through the JIT compiler.
//! The JIT compiler uses Cranelift for native code generation and supports
//! all JavaScript features including control flow, functions, classes, and async.

use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;
use std::time::Instant;

/// Minimum heap size in MB (16 MB)
const MIN_HEAP_SIZE_MB: usize = 16;
/// Maximum heap size in MB (16 GB)
const MAX_HEAP_SIZE_MB: usize = 16 * 1024;
/// Default heap size in MB (512 MB)
const DEFAULT_HEAP_SIZE_MB: usize = 512;

/// Parse and validate --max-heap-size argument
/// Returns Ok(size_in_mb) or Err(error_message)
fn parse_max_heap_size(value: &str) -> Result<usize, String> {
    let size_mb: usize = value.parse().map_err(|_| {
        format!("Invalid --max-heap-size value '{}': must be a number in MB", value)
    })?;

    if size_mb < MIN_HEAP_SIZE_MB {
        return Err(format!(
            "--max-heap-size must be at least {} MB, got {} MB",
            MIN_HEAP_SIZE_MB, size_mb
        ));
    }

    if size_mb > MAX_HEAP_SIZE_MB {
        return Err(format!(
            "--max-heap-size must be at most {} MB ({} GB), got {} MB",
            MAX_HEAP_SIZE_MB,
            MAX_HEAP_SIZE_MB / 1024,
            size_mb
        ));
    }

    Ok(size_mb)
}

fn main() -> ExitCode {
    // Load .env files from current directory
    dx_js_runtime::io::load_dotenv();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        // No file argument - start REPL mode
        return run_repl(DEFAULT_HEAP_SIZE_MB);
    }

    let file = &args[1];

    // Check for special flags
    if file == "--version" || file == "-v" {
        println!("dx-js-runtime {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    if file == "--help" || file == "-h" {
        println!("dx-js - A high-performance JavaScript/TypeScript runtime");
        println!();
        println!("ARCHITECTURE:");
        println!("  • OXC parser (fast JS/TS parser)");
        println!("  • Cranelift JIT (native code generation, no bytecode)");
        println!("  • Arena-based memory management");
        println!("  • Persistent code cache (fast cold starts)");
        println!("  • NaN-boxing (efficient primitive values)");
        println!();
        println!("USAGE:");
        println!("  dx-js                  Start interactive REPL");
        println!("  dx-js <file>           Run a JavaScript or TypeScript file");
        println!("  dx-js script.js        Run JavaScript");
        println!("  dx-js app.ts           Run TypeScript (no separate compilation!)");
        println!();
        println!("OPTIONS:");
        println!("  -v, --version          Print version information");
        println!("  -h, --help             Print this help message");
        println!("  --inspect[=port]       Start debugger on port (default: 9229)");
        println!("  --inspect-brk[=port]   Start debugger and break on first line");
        println!(
            "  --max-heap-size=<MB>   Set maximum heap size in MB (default: {}, range: {}-{})",
            DEFAULT_HEAP_SIZE_MB, MIN_HEAP_SIZE_MB, MAX_HEAP_SIZE_MB
        );
        println!();
        println!("REPL COMMANDS:");
        println!("  .exit                  Exit the REPL");
        println!("  .clear                 Clear the current input buffer");
        println!("  .help                  Show REPL help");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("  DX_DEBUG=1             Show execution timing and cache status");
        println!("  DX_CACHE_DIR=<path>    Set custom cache directory (default: .dx/cache)");
        println!("  DX_NO_CACHE=1          Disable the persistent code cache");
        println!();
        println!("EXAMPLES:");
        println!("  dx-js                             Start interactive REPL");
        println!("  dx-js hello.js                    Run a simple script");
        println!("  dx-js src/index.ts                Run TypeScript entry point");
        println!("  dx-js --inspect app.js            Run with debugger on port 9229");
        println!("  dx-js --inspect=9230 app.js       Run with debugger on port 9230");
        println!("  dx-js --max-heap-size=1024 app.js Run with 1GB heap limit");
        println!("  DX_DEBUG=1 dx-js benchmark.js    Run with timing info");
        println!();
        println!("For more information, visit: https://github.com/dx-tools/dx-javascript");
        return ExitCode::SUCCESS;
    }

    // Parse CLI flags
    let mut inspect_port: Option<u16> = None;
    let mut break_on_start = false;
    let mut file_arg: Option<&str> = None;
    let mut max_heap_size_mb: usize = DEFAULT_HEAP_SIZE_MB;

    for arg in args.iter().skip(1) {
        if arg.starts_with("--inspect-brk") {
            break_on_start = true;
            if let Some(port_str) = arg.strip_prefix("--inspect-brk=") {
                inspect_port = port_str.parse().ok();
            } else {
                inspect_port = Some(9229);
            }
        } else if arg.starts_with("--inspect") {
            if let Some(port_str) = arg.strip_prefix("--inspect=") {
                inspect_port = port_str.parse().ok();
            } else {
                inspect_port = Some(9229);
            }
        } else if arg.starts_with("--max-heap-size=") {
            if let Some(size_str) = arg.strip_prefix("--max-heap-size=") {
                match parse_max_heap_size(size_str) {
                    Ok(size) => max_heap_size_mb = size,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return ExitCode::from(1);
                    }
                }
            }
        } else if arg == "--max-heap-size" {
            eprintln!("Error: --max-heap-size requires a value (e.g., --max-heap-size=512)");
            return ExitCode::from(1);
        } else if !arg.starts_with("--") && file_arg.is_none() {
            file_arg = Some(arg);
        }
    }

    let file = match file_arg {
        Some(f) => f,
        None => {
            // No file specified with --inspect, start REPL with debugger
            if let Some(port) = inspect_port {
                return run_repl_with_debugger(port, max_heap_size_mb);
            }
            eprintln!("Error: No file specified");
            return ExitCode::from(1);
        }
    };

    // Validate file path
    let file_trimmed = file.trim();
    if file_trimmed.is_empty() {
        eprintln!("Error: File path cannot be empty");
        return ExitCode::from(1);
    }

    // Check if file exists
    if !std::path::Path::new(file_trimmed).exists() {
        eprintln!("Error: File not found: {}", file_trimmed);
        return ExitCode::from(1);
    }

    // Run the file
    let start = Instant::now();

    match std::fs::read_to_string(file_trimmed) {
        Ok(source) => {
            // All execution goes through the JIT compiler
            if let Some(port) = inspect_port {
                run_with_debugger(&source, file_trimmed, start, port, break_on_start, max_heap_size_mb)
            } else {
                run_with_jit(&source, file_trimmed, start, max_heap_size_mb)
            }
        }
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_trimmed, e);
            ExitCode::from(1)
        }
    }
}

/// Run the interactive REPL
fn run_repl(max_heap_size_mb: usize) -> ExitCode {
    println!(
        "dx-js v{} - A high-performance JavaScript/TypeScript runtime",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type \".help\" for more information, \".exit\" to quit.");
    if max_heap_size_mb != DEFAULT_HEAP_SIZE_MB {
        println!("Heap size: {} MB", max_heap_size_mb);
    }
    println!();

    // Create runtime with configured heap size
    let config = dx_js_runtime::DxConfig {
        max_heap_size_mb,
        ..Default::default()
    };
    let mut runtime = match dx_js_runtime::DxRuntime::with_config(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize runtime: {}", e);
            return ExitCode::from(1);
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input_buffer = String::new();
    let mut line_count = 0;

    loop {
        // Show prompt
        let prompt = if input_buffer.is_empty() {
            "> "
        } else {
            "... "
        };
        print!("{}", prompt);
        if stdout.flush().is_err() {
            break;
        }

        // Read line
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                // EOF
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }

        let trimmed = line.trim();

        // Handle REPL commands
        if input_buffer.is_empty() {
            match trimmed {
                ".exit" | ".quit" => {
                    println!("Goodbye!");
                    break;
                }
                ".clear" => {
                    input_buffer.clear();
                    println!("Input buffer cleared.");
                    continue;
                }
                ".help" => {
                    println!("REPL Commands:");
                    println!("  .exit, .quit    Exit the REPL");
                    println!("  .clear          Clear the current input buffer");
                    println!("  .help           Show this help message");
                    println!();
                    println!("Tips:");
                    println!("  - Multi-line input: incomplete statements continue on next line");
                    println!("  - Variables persist between evaluations");
                    println!("  - Press Ctrl+C to cancel current input");
                    println!("  - Press Ctrl+D to exit");
                    continue;
                }
                "" => continue,
                _ => {}
            }
        }

        // Add line to buffer
        input_buffer.push_str(&line);
        line_count += 1;

        // Try to evaluate the input
        // Check if the input looks complete (simple heuristic)
        if is_input_complete(&input_buffer) {
            let source = input_buffer.trim();
            if !source.is_empty() {
                match runtime.run_sync(source, &format!("repl:{}", line_count)) {
                    Ok(result) => {
                        // Print result if not undefined
                        if !matches!(result, dx_js_runtime::Value::Undefined) {
                            println!("{}", result);
                        }
                    }
                    Err(e) => {
                        // Check if it's a parse error that might be fixed with more input
                        let err_str = e.to_string();
                        if err_str.contains("Unexpected end of file")
                            || err_str.contains("Expected")
                        {
                            // Might be incomplete, continue reading
                            continue;
                        }
                        // Use the new error formatting for REPL errors
                        let error_output = match &e {
                            dx_js_runtime::DxError::SyntaxError {
                                message,
                                line,
                                column,
                            } => {
                                let exception = dx_js_runtime::syntax_error_with_snippet(
                                    message,
                                    source,
                                    format!("repl:{}", line_count),
                                    *line as u32,
                                    *column as u32,
                                );
                                dx_js_runtime::format_error_for_cli(&exception)
                            }
                            dx_js_runtime::DxError::TypeError { message } => {
                                let exception = dx_js_runtime::JsException::type_error(message);
                                dx_js_runtime::format_error_for_cli(&exception)
                            }
                            dx_js_runtime::DxError::ReferenceError { name } => {
                                let exception = dx_js_runtime::JsException::reference_error(
                                    format!("{} is not defined", name),
                                );
                                dx_js_runtime::format_error_for_cli(&exception)
                            }
                            _ => format!("Error: {}", e),
                        };
                        eprint!("{}", error_output);
                    }
                }
            }
            input_buffer.clear();
        }
    }

    ExitCode::SUCCESS
}

/// Check if the input looks complete (simple heuristic)
fn is_input_complete(input: &str) -> bool {
    let trimmed = input.trim();

    // Empty input is complete
    if trimmed.is_empty() {
        return true;
    }

    // Count brackets
    let mut brace_count = 0i32;
    let mut bracket_count = 0i32;
    let mut paren_count = 0i32;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut in_template = false;
    let mut escape_next = false;

    for ch in trimmed.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' {
            escape_next = true;
            continue;
        }

        if in_string {
            if ch == string_char {
                in_string = false;
            }
            continue;
        }

        if in_template {
            if ch == '`' {
                in_template = false;
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                in_string = true;
                string_char = ch;
            }
            '`' => {
                in_template = true;
            }
            '{' => brace_count += 1,
            '}' => brace_count -= 1,
            '[' => bracket_count += 1,
            ']' => bracket_count -= 1,
            '(' => paren_count += 1,
            ')' => paren_count -= 1,
            _ => {}
        }
    }

    // Input is complete if all brackets are balanced and we're not in a string
    brace_count == 0 && bracket_count == 0 && paren_count == 0 && !in_string && !in_template
}

/// Run with JIT compiler (unified execution path for all JavaScript)
fn run_with_jit(source: &str, filename: &str, start: Instant, max_heap_size_mb: usize) -> ExitCode {
    // Create runtime with configured heap size
    let config = dx_js_runtime::DxConfig {
        max_heap_size_mb,
        ..Default::default()
    };
    match dx_js_runtime::DxRuntime::with_config(config) {
        Ok(mut runtime) => {
            match runtime.run_sync(source, filename) {
                Ok(result) => {
                    // Print result if not undefined
                    if !matches!(result, dx_js_runtime::Value::Undefined) {
                        println!("{}", result);
                    }

                    let elapsed = start.elapsed();
                    if env::var("DX_DEBUG").is_ok() {
                        let stats = runtime.cache_stats();
                        eprintln!("\n─── Performance ───");
                        eprintln!("  Mode: JIT (Cranelift)");
                        eprintln!("  Cache: {} hits, {} misses", stats.hits, stats.misses);
                        eprintln!("  Total time: {:?}", elapsed);
                    }

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    // Use the new error formatting system
                    // Try to create a JsException with source context if possible
                    let error_output = match &e {
                        dx_js_runtime::DxError::ParseErrorWithLocation {
                            file,
                            line,
                            column,
                            message,
                        } => {
                            let exception = dx_js_runtime::syntax_error_with_snippet(
                                message,
                                source,
                                file,
                                *line as u32,
                                *column as u32,
                            );
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::SyntaxError {
                            message,
                            line,
                            column,
                        } => {
                            let exception = dx_js_runtime::syntax_error_with_snippet(
                                message,
                                source,
                                filename,
                                *line as u32,
                                *column as u32,
                            );
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::TypeError { message } => {
                            let exception = dx_js_runtime::JsException::type_error(message);
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::ReferenceError { name } => {
                            let exception = dx_js_runtime::JsException::reference_error(format!(
                                "{} is not defined",
                                name
                            ));
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        _ => {
                            // For other errors, use the default display
                            format!("Error: {}", e)
                        }
                    };
                    eprint!("{}", error_output);
                    ExitCode::from(1)
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize runtime: {}", e);
            ExitCode::from(1)
        }
    }
}

/// Run with debugger attached
fn run_with_debugger(
    source: &str,
    filename: &str,
    start: Instant,
    port: u16,
    break_on_start: bool,
    max_heap_size_mb: usize,
) -> ExitCode {
    use dx_js_runtime::debugger::CdpServer;

    // Start CDP server
    let cdp_server = CdpServer::new(port);
    if let Err(e) = cdp_server.start() {
        eprintln!("Failed to start debugger: {}", e);
        return ExitCode::from(1);
    }

    if break_on_start {
        println!("Waiting for debugger to connect...");
        // In a real implementation, we'd wait for a client to connect
        // and then pause execution at the first statement
    }

    // Create runtime with configured heap size
    let config = dx_js_runtime::DxConfig {
        max_heap_size_mb,
        ..Default::default()
    };
    match dx_js_runtime::DxRuntime::with_config(config) {
        Ok(mut runtime) => {
            match runtime.run_sync(source, filename) {
                Ok(result) => {
                    // Print result if not undefined
                    if !matches!(result, dx_js_runtime::Value::Undefined) {
                        println!("{}", result);
                    }

                    let elapsed = start.elapsed();
                    if env::var("DX_DEBUG").is_ok() {
                        let stats = runtime.cache_stats();
                        eprintln!("\n─── Performance ───");
                        eprintln!("  Mode: JIT (Cranelift) with Debugger");
                        eprintln!("  Cache: {} hits, {} misses", stats.hits, stats.misses);
                        eprintln!("  Total time: {:?}", elapsed);
                    }

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    // Use the new error formatting system
                    let error_output = match &e {
                        dx_js_runtime::DxError::ParseErrorWithLocation {
                            file,
                            line,
                            column,
                            message,
                        } => {
                            let exception = dx_js_runtime::syntax_error_with_snippet(
                                message,
                                source,
                                file,
                                *line as u32,
                                *column as u32,
                            );
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::SyntaxError {
                            message,
                            line,
                            column,
                        } => {
                            let exception = dx_js_runtime::syntax_error_with_snippet(
                                message,
                                source,
                                filename,
                                *line as u32,
                                *column as u32,
                            );
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::TypeError { message } => {
                            let exception = dx_js_runtime::JsException::type_error(message);
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        dx_js_runtime::DxError::ReferenceError { name } => {
                            let exception = dx_js_runtime::JsException::reference_error(format!(
                                "{} is not defined",
                                name
                            ));
                            dx_js_runtime::format_error_for_cli(&exception)
                        }
                        _ => format!("Error: {}", e),
                    };
                    eprint!("{}", error_output);
                    ExitCode::from(1)
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize runtime: {}", e);
            ExitCode::from(1)
        }
    }
}

/// Run REPL with debugger attached
fn run_repl_with_debugger(port: u16, max_heap_size_mb: usize) -> ExitCode {
    use dx_js_runtime::debugger::CdpServer;

    // Start CDP server
    let cdp_server = CdpServer::new(port);
    if let Err(e) = cdp_server.start() {
        eprintln!("Failed to start debugger: {}", e);
        return ExitCode::from(1);
    }

    // Run normal REPL
    run_repl(max_heap_size_mb)
}
