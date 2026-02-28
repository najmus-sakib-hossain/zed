//! DCP CLI tool.
//!
//! Provides commands for running DCP server and converting MCP schemas.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dcp")]
#[command(author, version, about = "Development Context Protocol CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in stdio mode (stdin/stdout for MCP compatibility)
    #[arg(long, global = true)]
    stdio: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the DCP server
    Serve {
        /// Host address to bind to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Enable MCP compatibility mode
        #[arg(long, default_value = "true")]
        mcp_compat: bool,

        /// Maximum concurrent sessions
        #[arg(long, default_value = "1000")]
        max_sessions: usize,

        /// Enable metrics collection
        #[arg(long, default_value = "true")]
        metrics: bool,

        /// Run in stdio mode instead of TCP
        #[arg(long)]
        stdio: bool,
    },

    /// Convert MCP schema to DCP format
    Convert {
        /// Input MCP schema file (JSON)
        #[arg(short, long)]
        input: PathBuf,

        /// Output DCP schema file (binary)
        #[arg(short, long)]
        output: PathBuf,

        /// Validate output schema
        #[arg(long, default_value = "true")]
        validate: bool,
    },

    /// Show server information
    Info,

    /// Validate a DCP schema file
    Validate {
        /// Schema file to validate
        #[arg(short, long)]
        schema: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    // Handle global --stdio flag (shortcut for `dcp serve --stdio`)
    if cli.stdio {
        run_stdio_mode();
        return;
    }

    match cli.command {
        Some(Commands::Serve {
            host,
            port,
            mcp_compat,
            max_sessions,
            metrics,
            stdio,
        }) => {
            if stdio {
                run_stdio_mode();
            } else {
                println!("Starting DCP server...");
                println!("  Host: {}", host);
                println!("  Port: {}", port);
                println!("  MCP Compatibility: {}", mcp_compat);
                println!("  Max Sessions: {}", max_sessions);
                println!("  Metrics: {}", metrics);
                println!();
                println!("Server would start here (async runtime required)");
                println!("For now, use the library directly in your application.");
            }
        }

        Some(Commands::Convert {
            input,
            output,
            validate,
        }) => {
            println!("Converting MCP schema to DCP format...");
            println!("  Input: {}", input.display());
            println!("  Output: {}", output.display());
            println!("  Validate: {}", validate);

            match convert_schema(&input, &output, validate) {
                Ok(()) => println!("Conversion successful!"),
                Err(e) => {
                    eprintln!("Conversion failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::Info) => {
            println!("DCP - Development Context Protocol");
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Protocol Features:");
            println!("  - Binary message envelope (8 bytes)");
            println!("  - O(1) tool dispatch via binary trie");
            println!("  - Zero-copy argument passing");
            println!("  - Ed25519 signed tool definitions");
            println!("  - XOR delta state synchronization");
            println!("  - MCP JSON-RPC compatibility layer");
        }

        Some(Commands::Validate { schema }) => {
            println!("Validating schema: {}", schema.display());

            match validate_schema(&schema) {
                Ok(()) => println!("Schema is valid!"),
                Err(e) => {
                    eprintln!("Validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        None => {
            // No command provided, show help
            println!("DCP - Development Context Protocol");
            println!("Use --help for usage information");
            println!("Use --stdio to run in stdio mode for MCP compatibility");
        }
    }
}

/// Run DCP in stdio mode for MCP compatibility
fn run_stdio_mode() {
    use dcp::compat::stdio::{StdioConfig, StdioTransport};
    use std::io::{self, BufRead, Write};

    eprintln!("[DCP] Starting in stdio mode");
    eprintln!("[DCP] Reading JSON-RPC messages from stdin");
    eprintln!("[DCP] Writing responses to stdout");

    let config = StdioConfig {
        stderr_logging: true,
        ..Default::default()
    };
    let mut transport = StdioTransport::with_config(config);

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    // Simple message loop
    loop {
        match transport.read_message(&mut stdin_lock) {
            Ok(Some(message)) => {
                transport.log_debug(&format!("Received: {}", &message[..message.len().min(100)]));

                // Parse and handle the message
                match handle_stdio_message(&message) {
                    Ok(response) => {
                        if let Some(resp) = response {
                            if let Err(e) = transport.write_message(&mut stdout_lock, &resp) {
                                transport.log_error(&format!("Write error: {}", e));
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        transport.log_error(&format!("Handler error: {}", e));
                        // Send error response
                        let error_response = format!(
                            r#"{{"jsonrpc":"2.0","error":{{"code":-32603,"message":"{}"}},"id":null}}"#,
                            e.replace('"', "\\\"")
                        );
                        let _ = transport.write_message(&mut stdout_lock, &error_response);
                    }
                }
            }
            Ok(None) => {
                // EOF or shutdown
                transport.log_stderr("Shutting down");
                break;
            }
            Err(e) => {
                transport.log_error(&format!("Read error: {}", e));
                break;
            }
        }
    }

    eprintln!("[DCP] Stdio mode terminated");
}

/// Handle a single stdio message and return optional response
fn handle_stdio_message(message: &str) -> Result<Option<String>, String> {
    use dcp::compat::json_rpc::{JsonRpcParser, RequestId};

    let request =
        JsonRpcParser::parse_request(message).map_err(|e| format!("Parse error: {}", e))?;

    // Check if this is a notification (no id = no response)
    if request.is_notification() {
        return Ok(None);
    }

    // Format the ID for JSON response
    let id = match &request.id {
        RequestId::Number(n) => n.to_string(),
        RequestId::String(s) => format!("\"{}\"", s),
        RequestId::Null => "null".to_string(),
    };

    // Handle known methods
    match request.method.as_str() {
        "initialize" => {
            let response = format!(
                r#"{{"jsonrpc":"2.0","result":{{"protocolVersion":"2024-11-05","capabilities":{{"tools":{{}},"resources":{{}},"prompts":{{}}}},"serverInfo":{{"name":"dcp","version":"{}"}}}},"id":{}}}"#,
                env!("CARGO_PKG_VERSION"),
                id
            );
            Ok(Some(response))
        }
        "initialized" => {
            // Notification, no response needed
            Ok(None)
        }
        "ping" => {
            let response = format!(r#"{{"jsonrpc":"2.0","result":{{}},"id":{}}}"#, id);
            Ok(Some(response))
        }
        "tools/list" => {
            let response = format!(r#"{{"jsonrpc":"2.0","result":{{"tools":[]}},"id":{}}}"#, id);
            Ok(Some(response))
        }
        "resources/list" => {
            let response =
                format!(r#"{{"jsonrpc":"2.0","result":{{"resources":[]}},"id":{}}}"#, id);
            Ok(Some(response))
        }
        "prompts/list" => {
            let response = format!(r#"{{"jsonrpc":"2.0","result":{{"prompts":[]}},"id":{}}}"#, id);
            Ok(Some(response))
        }
        _ => {
            // Unknown method
            let response = format!(
                r#"{{"jsonrpc":"2.0","error":{{"code":-32601,"message":"Method not found: {}"}},"id":{}}}"#,
                request.method, id
            );
            Ok(Some(response))
        }
    }
}

/// Convert MCP JSON schema to DCP binary format
fn convert_schema(input: &PathBuf, output: &PathBuf, validate: bool) -> Result<(), String> {
    use dcp::cli::convert::{convert_mcp_to_dcp, McpSchema};
    use std::fs;

    // Read input file
    let json_content =
        fs::read_to_string(input).map_err(|e| format!("Failed to read input file: {}", e))?;

    // Parse MCP schema
    let mcp_schema: McpSchema = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse MCP schema: {}", e))?;

    // Convert to DCP
    let dcp_schema =
        convert_mcp_to_dcp(&mcp_schema).map_err(|e| format!("Conversion error: {}", e))?;

    // Serialize to binary
    let binary_data = dcp_schema.to_bytes();

    // Validate if requested
    if validate {
        let _roundtrip = dcp::cli::convert::DcpSchema::from_bytes(&binary_data)
            .map_err(|e| format!("Validation failed: {}", e))?;
    }

    // Write output file
    fs::write(output, &binary_data).map_err(|e| format!("Failed to write output file: {}", e))?;

    println!("  Input size: {} bytes", json_content.len());
    println!("  Output size: {} bytes", binary_data.len());
    println!(
        "  Compression ratio: {:.1}x",
        json_content.len() as f64 / binary_data.len() as f64
    );

    Ok(())
}

/// Validate a DCP schema file
fn validate_schema(schema_path: &PathBuf) -> Result<(), String> {
    use std::fs;

    let data = fs::read(schema_path).map_err(|e| format!("Failed to read schema file: {}", e))?;

    let schema = dcp::cli::convert::DcpSchema::from_bytes(&data)
        .map_err(|e| format!("Invalid schema: {}", e))?;

    println!("  Name: {}", schema.name);
    println!("  Tool ID: {}", schema.tool_id);
    println!("  Fields: {}", schema.fields.len());
    println!("  Required fields: {}", schema.required_mask.count_ones());

    Ok(())
}
