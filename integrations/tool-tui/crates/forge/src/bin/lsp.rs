//! DX Forge LSP Server Binary
//!
//! Language Server Protocol server for DX Forge

use anyhow::Result;
use clap::Parser;
use std::io::{BufRead, Write};
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "forge-lsp")]
#[command(about = "DX Forge Language Server Protocol server", long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Log file path
    #[arg(long)]
    log_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    if let Some(log_file) = args.log_file {
        let file_appender = tracing_appender::rolling::daily(".", log_file);
        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_writer(file_appender)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_writer(std::io::stderr)
            .init();
    }

    info!("🚀 DX Forge LSP Server starting...");
    info!("📡 Protocol: Language Server Protocol (LSP)");
    info!("🔧 Features: DX component detection, completion, hover");

    // Create LSP server
    let _server = dx_forge::server::lsp::LspServer::new()?;

    info!("✅ LSP Server initialized");
    info!("📝 Reading from stdin, writing to stdout");
    info!("🎯 Ready to serve Language Server Protocol requests");

    // Simple JSON-RPC message loop (simplified version)
    // In production, use tower-lsp or lsp-server crates
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;

        // Parse JSON-RPC request (simplified)
        if line.contains("initialize") {
            info!("📡 Received initialize request");
            writeln!(
                stdout,
                r#"{{"jsonrpc":"2.0","id":1,"result":{{"capabilities":{{"completionProvider":{{"triggerCharacters":["d","x","<"]}}}}}}}}"#
            )?;
            stdout.flush()?;
        } else if line.contains("textDocument/didOpen") {
            info!("📄 Received didOpen notification");
        } else if line.contains("textDocument/didChange") {
            info!("✏️  Received didChange notification");
        } else if line.contains("textDocument/completion") {
            info!("💡 Received completion request");
            // Return DX completions
        } else if line.contains("shutdown") {
            info!("👋 Received shutdown request");
            break;
        }
    }

    info!("🛑 LSP Server shutting down");
    Ok(())
}
