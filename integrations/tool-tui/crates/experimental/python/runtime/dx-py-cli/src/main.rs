//! DX-Py CLI - Command-line interface for the DX-Py runtime

mod bench;
mod repl;

use clap::{Parser, Subcommand};
use dx_py_compiler::SourceCompiler;
use dx_py_core::pylist::PyValue;
use dx_py_interpreter::VirtualMachine;
use std::fs;
use std::path::PathBuf;

/// DX-Py: A revolutionary Python runtime
#[derive(Parser)]
#[command(name = "dx-py")]
#[command(author = "DX-Py Team")]
#[command(version = "0.1.0")]
#[command(about = "A high-performance Python runtime", long_about = None)]
struct Cli {
    /// Python file to execute
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Execute a command string
    #[arg(short = 'c', long)]
    command: Option<String>,

    /// Run in interactive mode (REPL)
    #[arg(short, long)]
    interactive: bool,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable JIT compilation
    #[arg(long, default_value = "true")]
    jit: bool,

    /// Disable JIT compilation
    #[arg(long)]
    no_jit: bool,

    /// Enable async mode for I/O operations
    #[arg(long, short = 'a')]
    r#async: bool,

    /// Show GC statistics on exit
    #[arg(long)]
    gc_stats: bool,

    /// Set GC threshold (number of allocations before collection)
    #[arg(long, default_value = "10000")]
    gc_threshold: usize,

    /// Enable debug mode
    #[arg(long, short = 'd')]
    debug: bool,

    #[command(subcommand)]
    subcommand: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a Python file to DPB bytecode
    Compile {
        /// Input Python file
        input: PathBuf,
        /// Output DPB file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Disassemble a DPB file
    Disasm {
        /// DPB file to disassemble
        file: PathBuf,
    },
    /// Show runtime information
    Info,
    /// Run benchmarks
    Bench {
        /// Benchmark to run
        #[arg(default_value = "all")]
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Handle subcommands first
    if let Some(cmd) = cli.subcommand {
        return handle_subcommand(cmd, cli.verbose);
    }

    // Execute command string
    if let Some(command) = cli.command {
        return execute_command(&command, cli.verbose);
    }

    // Execute file
    if let Some(file) = cli.file {
        return execute_file(&file, cli.verbose);
    }

    // Default to REPL if no file or command
    if cli.interactive || (cli.file.is_none() && cli.command.is_none()) {
        return run_repl(cli.verbose);
    }

    Ok(())
}

fn handle_subcommand(cmd: Commands, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Commands::Compile { input, output } => {
            let output = output.unwrap_or_else(|| {
                let mut out = input.clone();
                out.set_extension("dpb");
                out
            });

            if verbose {
                println!("Compiling {} -> {}", input.display(), output.display());
            }

            // Read source file
            let source = fs::read_to_string(&input)
                .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

            // Compile to bytecode with improved error reporting
            let mut compiler = SourceCompiler::new(input.clone());
            match compiler.compile_to_dpb_file(&source, &output) {
                Ok(()) => {
                    println!("Compiled {} -> {}", input.display(), output.display());
                    Ok(())
                }
                Err(e) => {
                    // Use format_with_source for better error reporting with position information
                    eprintln!("{}", e.format_with_source(&source, &input.to_string_lossy()));
                    Err("Compilation failed".into())
                }
            }
        }
        Commands::Disasm { file } => {
            if verbose {
                println!("Disassembling {}", file.display());
            }

            // Load DPB file using memory-mapped loader
            let module = dx_py_bytecode::DpbLoader::load(&file)
                .map_err(|e| format!("Failed to load {}: {}", file.display(), e))?;

            let header = module.header();
            println!("DPB Module:");
            println!("  Version: {}", header.version);
            println!("  Code size: {} bytes", header.code_size);
            println!("  Constants: {}", header.constants_count);
            println!("  Names: {}", header.names_count);
            println!();

            println!("Constants:");
            for i in 0..header.constants_count {
                if let Some(c) = module.get_constant(i) {
                    println!("  {}: {:?}", i, c);
                }
            }
            println!();

            println!("Names:");
            for i in 0..header.names_count {
                if let Some(n) = module.get_name(i) {
                    println!("  {}: {}", i, n);
                }
            }
            println!();

            println!("Bytecode ({} bytes):", module.code().len());
            disassemble_bytecode(module.code());

            Ok(())
        }
        Commands::Info => {
            print_info();
            Ok(())
        }
        Commands::Bench { name } => {
            if verbose {
                println!("Running benchmark: {}", name);
            }

            match name.as_str() {
                "all" => {
                    let results = bench::run_all_benchmarks();
                    bench::validate_targets(&results);
                }
                "startup" => {
                    let result = bench::bench_startup();
                    println!("{}: {:?}", result.name, result.mean_time);
                }
                "eval" => {
                    let result = bench::bench_eval_int();
                    println!("{}: {:?}", result.name, result.mean_time);
                }
                "list" => {
                    let result = bench::bench_list_ops();
                    println!("{}: {:?}", result.name, result.mean_time);
                }
                "dict" => {
                    let result = bench::bench_dict_ops();
                    println!("{}: {:?}", result.name, result.mean_time);
                }
                "string" => {
                    let result = bench::bench_string_ops();
                    println!("{}: {:?}", result.name, result.mean_time);
                }
                _ => {
                    println!("Unknown benchmark: {}", name);
                    println!("Available: all, startup, eval, list, dict, string");
                }
            }
            Ok(())
        }
    }
}

/// Disassemble bytecode to human-readable format
fn disassemble_bytecode(code: &[u8]) {
    use dx_py_bytecode::DpbOpcode;

    let mut i = 0;
    while i < code.len() {
        let opcode_byte = code[i];
        let opcode = DpbOpcode::from_u8(opcode_byte);

        print!("  {:4}: ", i);

        if let Some(op) = opcode {
            let arg_size = op.arg_size();
            print!("{:20}", format!("{:?}", op));

            if arg_size > 0 && i + arg_size < code.len() {
                let arg = match arg_size {
                    1 => code[i + 1] as u32,
                    2 => u16::from_le_bytes([code[i + 1], code[i + 2]]) as u32,
                    4 => u32::from_le_bytes([code[i + 1], code[i + 2], code[i + 3], code[i + 4]]),
                    _ => 0,
                };
                print!(" {}", arg);
            }
            println!();
            i += 1 + arg_size;
        } else {
            println!("UNKNOWN({})", opcode_byte);
            i += 1;
        }
    }
}

fn execute_command(command: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Executing: {}", command);
    }

    let vm = VirtualMachine::new();

    // Split command on semicolons to support multiple statements
    // This handles: dx-py -c "x = 1; y = 2; print(x + y)"
    let statements = split_statements(command);
    
    if verbose {
        println!("Found {} statement(s)", statements.len());
    }

    for (i, stmt) in statements.iter().enumerate() {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }

        if verbose {
            println!("Executing statement {}: {}", i + 1, stmt);
        }

        // First try the simple expression evaluator for basic expressions
        // This handles simple cases like "42", "'hello'", "1 + 2", etc.
        // But only for single statements or the last statement
        if statements.len() == 1 {
            match vm.eval_expr(stmt) {
                Ok(result) => {
                    if !matches!(result, PyValue::None) {
                        println!("{}", format_value(&result));
                    }
                    continue;
                }
                Err(_) => {
                    // Fall through to try compiling as full Python code
                }
            }
        }

        // Try to compile and execute as full Python code
        let mut compiler = SourceCompiler::new(std::path::PathBuf::from("<string>"));
        match compiler.compile_module_source(stmt) {
            Ok(code) => {
                if verbose {
                    println!("Compiled {} bytes of bytecode", code.code.len());
                    println!("Locals: {} (varnames: {:?})", code.nlocals, code.varnames);
                }

                // Convert constants from bytecode format to PyValue
                let constants: Vec<PyValue> = code.constants.iter().map(constant_to_pyvalue).collect();

                // Execute the bytecode with the correct number of locals
                // The VM's globals are shared between statements, enabling namespace sharing
                match vm.execute_bytecode_with_locals(
                    code.code,
                    constants,
                    code.names,
                    code.nlocals as usize,
                ) {
                    Ok(result) => {
                        // Only print result for the last statement if it's not None
                        if i == statements.len() - 1 && !matches!(result, PyValue::None) {
                            println!("{}", format_value(&result));
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        // Stop execution on error
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                // Use format_with_source for better error reporting with position information
                eprintln!("{}", e.format_with_source(stmt, "<string>"));
                // Stop execution on error
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Split a command string on semicolons, respecting string literals and parentheses.
/// This allows compound statements like `if x: y` to work correctly.
fn split_statements(command: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        // Handle escape sequences in strings
        if in_string && c == '\\' && i + 1 < chars.len() {
            i += 2; // Skip the escaped character
            continue;
        }
        
        // Handle string delimiters
        if !in_string && (c == '"' || c == '\'') {
            // Check for triple-quoted strings
            if i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c {
                in_string = true;
                string_char = c;
                i += 3;
                continue;
            }
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }
        
        if in_string && c == string_char {
            // Check for end of triple-quoted string
            if i + 2 < chars.len() && chars[i + 1] == string_char && chars[i + 2] == string_char {
                in_string = false;
                i += 3;
                continue;
            }
            // Check if this is a single-quoted string (not triple)
            // We need to check if we started with a single quote
            if i >= 1 {
                // Simple heuristic: if we're at a quote and not in triple-quote mode
                in_string = false;
            }
            i += 1;
            continue;
        }
        
        if in_string {
            i += 1;
            continue;
        }
        
        // Track parentheses, brackets, and braces
        match c {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                // Found a statement separator
                let byte_start = chars[..start].iter().collect::<String>().len();
                let byte_end = chars[..i].iter().collect::<String>().len();
                let stmt = &command[byte_start..byte_end];
                if !stmt.trim().is_empty() {
                    statements.push(stmt);
                }
                start = i + 1;
            }
            _ => {}
        }
        
        i += 1;
    }
    
    // Add the last statement
    if start < chars.len() {
        let byte_start = chars[..start].iter().collect::<String>().len();
        let stmt = &command[byte_start..];
        if !stmt.trim().is_empty() {
            statements.push(stmt);
        }
    }
    
    // If no semicolons were found, return the whole command as a single statement
    if statements.is_empty() && !command.trim().is_empty() {
        statements.push(command);
    }
    
    statements
}

/// Convert a bytecode Constant to a PyValue
fn constant_to_pyvalue(constant: &dx_py_bytecode::Constant) -> PyValue {
    use dx_py_bytecode::Constant;
    use dx_py_core::pylist::PyCode;
    use std::sync::Arc;
    
    match constant {
        Constant::None => PyValue::None,
        Constant::Bool(b) => PyValue::Bool(*b),
        Constant::Int(i) => PyValue::Int(*i),
        Constant::Float(f) => PyValue::Float(*f),
        Constant::Complex(_, _) => PyValue::None, // Complex not supported yet
        Constant::String(s) => PyValue::Str(Arc::from(s.as_str())),
        Constant::Bytes(_) => PyValue::None, // Bytes not supported yet
        Constant::Tuple(items) => {
            let values: Vec<PyValue> = items.iter().map(constant_to_pyvalue).collect();
            PyValue::Tuple(Arc::new(dx_py_core::PyTuple::from_values(values)))
        }
        Constant::FrozenSet(_) => PyValue::None, // FrozenSet not supported yet
        Constant::Code(code_obj) => {
            // Convert CodeObject to PyCode
            let mut pycode = PyCode::new(code_obj.name.as_str(), code_obj.filename.as_str());
            pycode.qualname = Arc::from(code_obj.qualname.as_str());
            pycode.firstlineno = code_obj.firstlineno;
            pycode.argcount = code_obj.argcount;
            pycode.posonlyargcount = code_obj.posonlyargcount;
            pycode.kwonlyargcount = code_obj.kwonlyargcount;
            pycode.nlocals = code_obj.nlocals;
            pycode.stacksize = code_obj.stacksize;
            pycode.flags = code_obj.flags.bits();
            pycode.code = Arc::from(code_obj.code.as_slice());
            // Recursively convert constants
            pycode.constants = Arc::from(
                code_obj.constants.iter().map(constant_to_pyvalue).collect::<Vec<_>>()
            );
            pycode.names = Arc::from(
                code_obj.names.iter().map(|s| Arc::from(s.as_str())).collect::<Vec<_>>()
            );
            pycode.varnames = Arc::from(
                code_obj.varnames.iter().map(|s| Arc::from(s.as_str())).collect::<Vec<_>>()
            );
            pycode.freevars = Arc::from(
                code_obj.freevars.iter().map(|s| Arc::from(s.as_str())).collect::<Vec<_>>()
            );
            pycode.cellvars = Arc::from(
                code_obj.cellvars.iter().map(|s| Arc::from(s.as_str())).collect::<Vec<_>>()
            );
            PyValue::Code(Arc::new(pycode))
        }
        Constant::Ellipsis => PyValue::None, // Ellipsis not supported yet
    }
}

fn execute_file(file: &PathBuf, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("Executing file: {}", file.display());
    }

    // Check file extension
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "py" => {
            // Read and compile Python source
            let source = fs::read_to_string(file)
                .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;

            if verbose {
                println!("Compiling Python source...");
            }

            let mut compiler = SourceCompiler::new(file.clone());
            let code = match compiler.compile_module_source(&source) {
                Ok(code) => code,
                Err(e) => {
                    // Use format_with_source for better error reporting with position information
                    eprintln!("{}", e.format_with_source(&source, &file.to_string_lossy()));
                    return Ok(());
                }
            };

            if verbose {
                println!("Compiled {} bytes of bytecode", code.code.len());
                println!("Constants: {:?}", code.constants);
                println!("Names: {:?}", code.names);
                println!("Locals: {} (varnames: {:?})", code.nlocals, code.varnames);
            }

            // Execute the compiled bytecode
            let vm = VirtualMachine::new();
            let constants: Vec<PyValue> = code.constants.iter().map(constant_to_pyvalue).collect();

            match vm.execute_bytecode_with_locals(
                code.code,
                constants,
                code.names,
                code.nlocals as usize,
            ) {
                Ok(result) => {
                    if verbose && !matches!(result, PyValue::None) {
                        println!("Result: {}", format_value(&result));
                    }
                }
                Err(e) => {
                    eprintln!("Runtime error: {}", e);
                }
            }
        }
        "dpb" => {
            // Load DPB file using memory-mapped loader
            let module = dx_py_bytecode::DpbLoader::load(file)
                .map_err(|e| format!("Failed to load {}: {}", file.display(), e))?;

            let header = module.header();
            if verbose {
                println!("Loaded DPB file:");
                println!("  Code size: {} bytes", header.code_size);
                println!("  Constants: {}", header.constants_count);
                println!("  Names: {}", header.names_count);
            }

            // Convert constants from DPB format to PyValue
            let mut constants = Vec::new();
            for i in 0..header.constants_count {
                if let Some(c) = module.get_constant(i) {
                    constants.push(constant_to_pyvalue(&c));
                }
            }

            // Get names
            let mut names = Vec::new();
            for i in 0..header.names_count {
                if let Some(n) = module.get_name(i) {
                    names.push(n.to_string());
                }
            }

            // Execute the bytecode
            let vm = VirtualMachine::new();
            match vm.execute_bytecode_with_locals(
                module.code().to_vec(),
                constants,
                names,
                256, // Default locals size
            ) {
                Ok(result) => {
                    if verbose && !matches!(result, PyValue::None) {
                        println!("Result: {}", format_value(&result));
                    }
                }
                Err(e) => {
                    eprintln!("Runtime error: {}", e);
                }
            }
        }
        "dpm" => {
            // DPM module execution is not yet implemented
            println!("DPM module execution not yet implemented");
        }
        _ => {
            eprintln!("Unknown file type: {}", ext);
        }
    }

    Ok(())
}

fn run_repl(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut repl = repl::Repl::new().with_verbose(verbose);
    repl.run()
}

fn print_info() {
    println!("DX-Py Runtime Information");
    println!("========================");
    println!();
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!();
    println!("Features:");
    println!("  - Binary Python Bytecode (DPB)");
    println!("  - SIMD-Accelerated String Operations (AVX2/AVX-512/NEON)");
    println!("  - Lock-Free Parallel Garbage Collector");
    println!("  - Tiered JIT with Cranelift Backend");
    println!("  - Speculative Type Prediction");
    println!("  - Memory Teleportation FFI (NumPy zero-copy)");
    println!("  - Binary Module Format (DPM)");
    println!("  - Thread-Per-Core Parallel Executor");
    println!("  - Stack Allocation Fast Path");
    println!("  - Binary Protocol IPC (HBTP)");
    println!("  - Reactive Bytecode Cache");
    println!("  - SIMD-Accelerated Collections");
    println!("  - Compiler-Inlined Decorators");
    println!("  - Persistent Compilation Cache (PCC)");
    println!("  - Cross-Process Shared Objects");
    println!("  - Async I/O (io_uring/kqueue/IOCP)");
    println!();
    println!("CPU Features:");

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            println!("  - AVX-512: enabled (64 bytes/iteration)");
        } else if is_x86_feature_detected!("avx2") {
            println!("  - AVX2: enabled (32 bytes/iteration)");
        } else {
            println!("  - AVX2: not available");
        }
        if is_x86_feature_detected!("sse4.2") {
            println!("  - SSE4.2: enabled");
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        println!("  - NEON: enabled (16 bytes/iteration)");
    }

    println!();
    println!(
        "Threads: {}",
        std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1)
    );
    println!();
    println!("CLI Flags:");
    println!("  --jit / --no-jit  Enable/disable JIT compilation");
    println!("  --async / -a      Enable async I/O mode");
    println!("  --gc-stats        Show GC statistics on exit");
    println!("  --gc-threshold N  Set GC collection threshold");
    println!("  --debug / -d      Enable debug mode");
}

#[allow(dead_code)]
fn print_help() {
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
    println!("Special commands:");
    println!("  exit()      - Exit the REPL");
    println!("  quit()      - Exit the REPL");
    println!("  help()      - Show this help");
}

fn format_value(value: &PyValue) -> String {
    match value {
        PyValue::None => "None".to_string(),
        PyValue::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        PyValue::Int(i) => i.to_string(),
        PyValue::Float(f) => format!("{}", f),
        PyValue::Str(s) => format!("'{}'", s),
        PyValue::List(l) => {
            let items: Vec<String> = l.to_vec().iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        PyValue::Tuple(t) => {
            let items: Vec<String> = t.to_vec().iter().map(format_value).collect();
            if items.len() == 1 {
                format!("({},)", items[0])
            } else {
                format!("({})", items.join(", "))
            }
        }
        PyValue::Dict(d) => {
            let items: Vec<String> =
                d.items().iter().map(|(k, v)| format!("{:?}: {}", k, format_value(v))).collect();
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
            let items: Vec<String> = s.to_vec().iter().map(format_value).collect();
            format!("{{{}}}", items.join(", "))
        }
        PyValue::Module(m) => format!("<module '{}'>", m.name),
        PyValue::Code(c) => format!("<code object {} at {:p}>", c.name, c),
        PyValue::Cell(c) => format!("<cell: {}>", format_value(&c.get())),
        PyValue::Super(_) => "<super>".to_string(),
        PyValue::Property(_) => "<property>".to_string(),
        PyValue::StaticMethod(_) => "<staticmethod>".to_string(),
        PyValue::ClassMethod(_) => "<classmethod>".to_string(),
    }
}

#[cfg(test)]
mod split_statements_tests {
    use super::split_statements;

    #[test]
    fn test_single_statement() {
        let stmts = split_statements("x = 1");
        assert_eq!(stmts, vec!["x = 1"]);
    }

    #[test]
    fn test_multiple_statements() {
        let stmts = split_statements("x = 1; y = 2; z = 3");
        assert_eq!(stmts, vec!["x = 1", " y = 2", " z = 3"]);
    }

    #[test]
    fn test_semicolon_in_string() {
        let stmts = split_statements("x = 'hello; world'");
        assert_eq!(stmts, vec!["x = 'hello; world'"]);
    }

    #[test]
    fn test_semicolon_in_double_quoted_string() {
        let stmts = split_statements("x = \"hello; world\"");
        assert_eq!(stmts, vec!["x = \"hello; world\""]);
    }

    #[test]
    fn test_semicolon_in_parentheses() {
        let stmts = split_statements("x = (1; 2)");
        assert_eq!(stmts, vec!["x = (1; 2)"]);
    }

    #[test]
    fn test_semicolon_in_brackets() {
        let stmts = split_statements("x = [1; 2]");
        assert_eq!(stmts, vec!["x = [1; 2]"]);
    }

    #[test]
    fn test_semicolon_in_braces() {
        let stmts = split_statements("x = {1; 2}");
        assert_eq!(stmts, vec!["x = {1; 2}"]);
    }

    #[test]
    fn test_empty_statements_filtered() {
        let stmts = split_statements("x = 1;; y = 2");
        assert_eq!(stmts, vec!["x = 1", " y = 2"]);
    }

    #[test]
    fn test_trailing_semicolon() {
        let stmts = split_statements("x = 1;");
        assert_eq!(stmts, vec!["x = 1"]);
    }

    #[test]
    fn test_leading_semicolon() {
        let stmts = split_statements("; x = 1");
        assert_eq!(stmts, vec![" x = 1"]);
    }

    #[test]
    fn test_complex_expression() {
        let stmts = split_statements("x = 1; print(x + 2); y = x * 3");
        assert_eq!(stmts, vec!["x = 1", " print(x + 2)", " y = x * 3"]);
    }

    #[test]
    fn test_nested_parentheses() {
        let stmts = split_statements("x = f(g(1; 2)); y = 3");
        assert_eq!(stmts, vec!["x = f(g(1; 2))", " y = 3"]);
    }

    #[test]
    fn test_mixed_quotes() {
        let stmts = split_statements("x = 'a;b'; y = \"c;d\"; z = 1");
        assert_eq!(stmts, vec!["x = 'a;b'", " y = \"c;d\"", " z = 1"]);
    }

    #[test]
    fn test_empty_input() {
        let stmts = split_statements("");
        assert!(stmts.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let stmts = split_statements("   ");
        assert!(stmts.is_empty());
    }
}
