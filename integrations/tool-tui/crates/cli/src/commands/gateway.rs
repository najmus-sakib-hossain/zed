//! Gateway Command Arguments
//!
//! Clap argument definitions for gateway subcommands.

use clap::{Args, Subcommand, ValueEnum};

/// Gateway control commands for platform app communication
#[derive(Args)]
pub struct GatewayArgs {
    #[command(subcommand)]
    pub command: GatewayCommands,
}

#[derive(Subcommand)]
pub enum GatewayCommands {
    /// Start the gateway server
    Start {
        /// Host to bind to
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value = "31337")]
        port: u16,
        /// Run in foreground
        #[arg(short, long)]
        foreground: bool,
        /// Enable mDNS discovery
        #[arg(long, default_value = "true")]
        mdns: bool,
        /// Require authentication
        #[arg(long, default_value = "true")]
        auth: bool,
    },

    /// Stop the gateway server
    Stop {
        /// Force stop
        #[arg(long)]
        force: bool,
    },

    /// Show gateway status
    Status {
        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
        /// Output format
        #[arg(short, long, default_value = "human")]
        format: OutputFormat,
    },

    /// List connected clients
    Clients {
        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
    },

    /// Generate pairing code
    Pair {
        /// Duration in seconds
        #[arg(short, long, default_value = "300")]
        duration: u64,
        /// Show QR code
        #[arg(long)]
        qr: bool,
    },

    /// Disconnect a client
    Disconnect {
        /// Client ID
        client_id: String,
    },

    /// Configure gateway settings
    Config {
        /// Maximum connections
        #[arg(long)]
        max_connections: Option<usize>,
        /// Session timeout (seconds)
        #[arg(long)]
        session_timeout: Option<u64>,
        /// Allowed commands (comma-separated, empty for all)
        #[arg(long)]
        allowed_commands: Option<String>,
    },

    /// View gateway logs
    Logs {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,
        /// Follow logs
        #[arg(short, long)]
        follow: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Llm,
}
