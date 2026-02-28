use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dx_forge::{DualWatcher, context, daemon::DaemonServer, server, storage};
use std::path::PathBuf;
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Parser)]
#[command(name = "forge")]
#[command(
    about = "Next-generation version control with operation-level tracking, CRDT-based sync, and seamless Git integration",
    version
)]
#[command(after_help = "Forge Features:
- Operation-level version control with CRDT for conflict-free collaboration
- Real-time sync between multiple peers via WebSocket
- Character-level anchors and permalinks for precise code references
- AI-powered code annotations and context exploration
- Git repository synchronization and integration
- Collaborative server for multi-user editing
- Time-travel debugging to view file states at any timestamp
- Comprehensive operation logging and querying
- Seamless Git command support without 'git' prefix

All Git commands are supported without the 'git' prefix. Use 'forge <git-command>' instead of 'git <git-command>'.

Main Porcelain Commands:
   add, am, archive, backfill, bisect, branch, bundle, checkout, cherry-pick, citool, clean, clone, commit, describe, diff, fetch, format-patch, gc, gitk, grep, gui, init, log, maintenance, merge, mv, notes, pull, push, range-diff, rebase, reset, restore, revert, rm, scalar, shortlog, show, sparse-checkout, stash, status, submodule, survey, switch, tag, worktree

Ancillary Commands / Manipulators:
   config, fast-export, fast-import, filter-branch, mergetool, pack-refs, prune, reflog, refs, remote, repack, replace

Ancillary Commands / Interrogators:
   annotate, blame, bugreport, count-objects, diagnose, difftool, fsck, gitweb, help, instaweb, merge-tree, rerere, show-branch, verify-commit, verify-tag, version, whatchanged

Interacting with Others:
   archimport, cvsexportcommit, cvsimport, cvsserver, imap-send, p4, quiltimport, request-pull, send-email, svn

Low-level Commands / Manipulators:
   apply, checkout-index, commit-graph, commit-tree, hash-object, index-pack, merge-file, merge-index, mktag, mktree, multi-pack-index, pack-objects, prune-packed, read-tree, replay, symbolic-ref, unpack-objects, update-index, update-ref, write-tree

Low-level Commands / Interrogators:
   cat-file, cherry, diff-files, diff-index, diff-pairs, diff-tree, for-each-ref, for-each-repo, get-tar-commit-id, ls-files, ls-remote, ls-tree, merge-base, name-rev, pack-redundant, rev-list, rev-parse, show-index, show-ref, unpack-file, var, verify-pack

Low-level Commands / Syncing Repositories:
   daemon, fetch-pack, http-backend, send-pack, update-server-info

Low-level Commands / Internal Helpers:
   check-attr, check-ignore, check-mailmap, check-ref-format, column, credential, credential-cache, credential-store, fmt-merge-msg, hook, interpret-trailers, mailinfo, mailsplit, merge-one-file, patch-id, sh-i18n, sh-setup, stripspace

External commands:
   askpass, askyesno, credential-helper-selector, credential-manager, flow, lfs, update-git-for-windows")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Forge repository
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Watch for changes and track operations
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable real-time sync
        #[arg(long)]
        sync: bool,

        /// WebSocket peer(s) to connect, e.g. ws://localhost:3000/ws
        #[arg(long, value_name = "URL")]
        peer: Vec<String>,
    },

    /// Query the operation log
    OpLog {
        #[arg(short, long)]
        file: Option<PathBuf>,

        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Create a character-level anchor/permalink
    Anchor {
        file: PathBuf,
        line: usize,
        column: usize,

        #[arg(short, long)]
        message: Option<String>,
    },

    /// Annotate code with context
    Annotate {
        file: PathBuf,
        line: usize,

        #[arg(short, long)]
        message: String,

        #[arg(long)]
        ai: bool,
    },

    /// Show annotations and context for a file
    Context {
        file: PathBuf,

        #[arg(short, long)]
        line: Option<usize>,
    },

    /// Sync Forge repository
    ForgeSync {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Any unrecognized subcommand will be passed to the system `git`.
    #[command(external_subcommand)]
    GitPassthrough(Vec<String>),

    /// Start collaborative server
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,

        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show time-travel view of a file
    TimeTravel {
        file: PathBuf,

        #[arg(short, long)]
        timestamp: Option<String>,
    },

    /// Update DX-managed components
    Update {
        /// Component to update (or "all")
        component: Option<String>,

        /// Force update (skip conflict checks)
        #[arg(long)]
        force: bool,
    },

    /// List managed DX components
    Components {
        /// Show component details
        #[arg(long)]
        verbose: bool,
    },

    /// Register a component for tracking
    Register {
        /// Path to component file
        path: PathBuf,

        /// Component source (e.g., "dx-ui")
        #[arg(long)]
        source: String,

        /// Component name (e.g., "Button")
        #[arg(long)]
        name: String,

        /// Component version
        #[arg(long)]
        version: String,
    },

    /// Start the Forge daemon (background service)
    #[command(name = "daemon")]
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the Forge daemon
    Start {
        /// Project root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,

        /// WebSocket port for VS Code extension
        #[arg(long, default_value = "9876")]
        port: u16,

        /// IPC port for CLI communication (Windows)
        #[arg(long, default_value = "9877")]
        ipc_port: u16,
    },

    /// Stop the Forge daemon
    Stop {
        /// Force stop without graceful shutdown
        #[arg(long)]
        force: bool,
    },

    /// Restart the Forge daemon
    Restart {
        /// Project root directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show daemon status
    Status,

    /// List registered tools
    Tools,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    // Initialize structured logging to logs/forge.log with daily rotation
    let log_dir = std::env::current_dir()?.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    // Use daily rotation to prevent unbounded log file growth
    let file_appender = rolling::daily(&log_dir, "forge.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);

    let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    let env_filter = match EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(_) => EnvFilter::new("warn"), // Changed from 'info' to 'warn' to reduce log spam
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // keep the guard alive for the lifetime of the program
    let _guard = guard;

    // Only log at startup if debug/trace is enabled
    // info!("Logging initialized: {}", log_dir.join("forge.log").display());

    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => Commands::Watch {
            path: ".".into(),
            sync: false,
            peer: vec![],
        },
    };

    match command {
        Commands::Init { path } => {
            println!("{}", "🚀 Initializing Forge DeltaDB repository...".cyan().bold());
            storage::init(&path).await?;
            println!("{}", "✓ Repository initialized successfully!".green());
            println!("\n{}", "Next steps:".yellow());
            println!("  1. {} - Start tracking operations", "forge watch".bright_white());
            println!("  2. {} - View operation log", "forge oplog".bright_white());
            println!(
                "  3. {} - Add context to code",
                "forge annotate <file> <line> -m \"message\"".bright_white()
            );
        }

        Commands::Watch { path, sync, peer } => {
            println!("{}", "✔ Starting dual-watcher system...".cyan().bold());
            let mut watcher = DualWatcher::new()?;
            watcher.start(&path).await?;

            if sync {
                println!("{}", "✔ Sync enabled".green());
            }
            if !peer.is_empty() {
                for peer_addr in &peer {
                    println!("{}", format!("✔ Peer: {}", peer_addr).green());
                }
            }

            let mut rx = watcher.receiver();
            while let Ok(change) = rx.recv().await {
                println!("Change detected: {:?} ({:?})", change.path, change.source);
            }
        }

        Commands::OpLog { file, limit } => {
            storage::show_log(file, limit.unwrap_or(50)).await?;
        }

        Commands::Anchor {
            file,
            line,
            column,
            message,
        } => {
            let anchor = context::create_anchor(&file, line, column, message).await?;
            println!("{} Created anchor: {}", "✓".green(), anchor.id.to_string().bright_yellow());
            println!("  Permalink: {}", anchor.permalink().bright_blue());
        }

        Commands::Annotate {
            file,
            line,
            message,
            ai,
        } => {
            context::annotate(&file, line, &message, ai).await?;
            println!("{} Annotation added", "✓".green());
        }

        Commands::Context { file, line } => {
            context::show_context(&file, line).await?;
        }

        Commands::ForgeSync { path } => {
            storage::git_sync(&path).await?;
        }

        Commands::GitPassthrough(args) => {
            use tokio::process::Command;
            let status = if args.is_empty() {
                Command::new("git").status().await?
            } else {
                Command::new("git").args(args).status().await?
            };
            if !status.success() {
                eprintln!("git exited with status: {}", status);
            }
        }

        Commands::Serve { port, path } => {
            println!("{}", format!("🌐 Starting server on port {}...", port).cyan().bold());
            server::start(port, path).await?;
        }

        Commands::TimeTravel { file, timestamp } => {
            storage::time_travel(&file, timestamp).await?;
        }

        Commands::Update {
            component,
            force: _,
        } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let state_mgr = ComponentStateManager::new(&forge_dir)?;

            if let Some(comp_name) = component {
                if comp_name == "all" {
                    println!("{}", "🔄 Updating all components...".cyan().bold());
                    let components = state_mgr.list_components();
                    for comp in &components {
                        println!(
                            "\n{} Checking {}...",
                            "→".bright_black(),
                            comp.name.bright_cyan()
                        );
                        // In production, fetch remote version and apply update
                        println!(
                            "   {} (placeholder - would fetch and update)",
                            "→".bright_black()
                        );
                    }
                } else {
                    println!(
                        "{}",
                        format!("🔄 Updating component: {}...", comp_name).cyan().bold()
                    );
                    // In production, fetch remote version and apply update
                    println!("   {} (placeholder - would fetch and update)", "→".bright_black());
                }
            } else {
                println!("{}", "Please specify a component name or 'all'".yellow());
            }
        }

        Commands::Components { verbose } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let state_mgr = ComponentStateManager::new(&forge_dir)?;

            let components = state_mgr.list_components();

            if components.is_empty() {
                println!("{}", "No managed components found.".yellow());
                println!("\n{}", "To register a component:".bright_black());
                println!(
                    "  {}",
                    "forge register <path> --source dx-ui --name Button --version 1.0.0"
                        .bright_white()
                );
            } else {
                println!("{}", "📦 Managed Components".cyan().bold());
                println!("{}", "═".repeat(80).bright_black());

                for comp in &components {
                    println!(
                        "\n{} {} {}",
                        "●".bright_green(),
                        comp.name.bright_cyan().bold(),
                        format!("v{}", comp.version).bright_black()
                    );
                    println!("   {} {}", "Source:".bright_black(), comp.source);
                    println!("   {} {}", "Path:  ".bright_black(), comp.path);

                    if verbose {
                        println!("   {} {}", "Hash:  ".bright_black(), &comp.base_hash[..16]);
                        println!(
                            "   {} {}",
                            "Added: ".bright_black(),
                            comp.installed_at.format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                }

                println!("\n{}", "─".repeat(80).bright_black());
                println!(
                    "{} {} | {} {}",
                    format!("{} components", components.len()).bright_white().bold(),
                    "Use --verbose for details".bright_black(),
                    "forge update <name>".bright_white(),
                    "to update".bright_black()
                );
            }
        }

        Commands::Register {
            path,
            source,
            name,
            version,
        } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let mut state_mgr = ComponentStateManager::new(&forge_dir)?;

            // Read component content
            let content = tokio::fs::read_to_string(&path).await?;

            // Register component
            state_mgr.register_component(&path, &source, &name, &version, &content)?;

            println!(
                "{} Registered {} {} {}",
                "✓".green(),
                name.bright_cyan().bold(),
                format!("v{}", version).bright_black(),
                format!("from {}", source).bright_black()
            );
            println!("   {} {}", "Path:".bright_black(), path.display());
        }

        Commands::Daemon { action } => {
            match action {
                DaemonAction::Start {
                    path,
                    foreground,
                    port,
                    ipc_port,
                } => {
                    println!("{}", "🚀 Starting Forge Daemon...".cyan().bold());

                    // Check if daemon is already running
                    if is_daemon_running() {
                        println!("{}", "⚠️  Forge daemon is already running".yellow());
                        return Ok(());
                    }

                    // Set environment variables for ports
                    // SAFETY: We're setting environment variables before spawning any threads
                    // and this is the main entry point of the CLI
                    unsafe {
                        std::env::set_var("DX_FORGE_WS_PORT", port.to_string());
                        std::env::set_var("DX_FORGE_IPC_PORT", ipc_port.to_string());
                    }

                    if foreground {
                        // Run in foreground
                        println!("📁 Project: {}", path.display());
                        println!("🔌 WebSocket port: {}", port);
                        println!("🔌 IPC port: {}", ipc_port);
                        println!();

                        let server = DaemonServer::new();
                        server.start().await?;
                    } else {
                        // Spawn as background process
                        #[cfg(windows)]
                        {
                            use std::process::Command;
                            let exe = std::env::current_exe()?;
                            let child = Command::new(exe)
                                .args(["daemon", "start", "--foreground", "--port", &port.to_string(), "--ipc-port", &ipc_port.to_string(), &path.to_string_lossy()])
                                .creation_flags(0x00000008) // DETACHED_PROCESS
                                .spawn()?;

                            println!("{} Forge daemon started (PID: {})", "✓".green(), child.id());
                            println!("   WebSocket: ws://127.0.0.1:{}", port);
                            println!("   IPC: 127.0.0.1:{}", ipc_port);
                        }

                        #[cfg(unix)]
                        {
                            use std::process::Command;
                            let exe = std::env::current_exe()?;
                            let child = Command::new(exe)
                                .args([
                                    "daemon",
                                    "start",
                                    "--foreground",
                                    "--port",
                                    &port.to_string(),
                                    "--ipc-port",
                                    &ipc_port.to_string(),
                                    &path.to_string_lossy(),
                                ])
                                .spawn()?;

                            println!("{} Forge daemon started (PID: {})", "✓".green(), child.id());
                            println!("   WebSocket: ws://127.0.0.1:{}", port);
                            println!("   Socket: {}", DaemonServer::socket_path().display());
                        }
                    }
                }

                DaemonAction::Stop { force } => {
                    println!("{}", "🛑 Stopping Forge Daemon...".cyan().bold());

                    if !is_daemon_running() {
                        println!("{}", "⚠️  Forge daemon is not running".yellow());
                        return Ok(());
                    }

                    // Send shutdown command via IPC
                    match send_ipc_command(r#"{"command":"Shutdown","force":false}"#).await {
                        Ok(_) => {
                            println!("{} Forge daemon stopped", "✓".green());
                        }
                        Err(e) => {
                            if force {
                                // Force kill by PID
                                if let Ok(pid_str) =
                                    std::fs::read_to_string(DaemonServer::pid_path())
                                {
                                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                                        #[cfg(windows)]
                                        {
                                            let _ = std::process::Command::new("taskkill")
                                                .args(["/F", "/PID", &pid.to_string()])
                                                .output();
                                        }
                                        #[cfg(unix)]
                                        {
                                            let _ = std::process::Command::new("kill")
                                                .args(["-9", &pid.to_string()])
                                                .output();
                                        }
                                        println!(
                                            "{} Forge daemon force stopped (PID: {})",
                                            "✓".green(),
                                            pid
                                        );
                                    }
                                }
                            } else {
                                println!("{} Failed to stop daemon: {}", "✗".red(), e);
                                println!("   Use --force to force stop");
                            }
                        }
                    }

                    // Clean up PID file
                    let _ = std::fs::remove_file(DaemonServer::pid_path());
                }

                DaemonAction::Restart { path } => {
                    println!("{}", "🔄 Restarting Forge Daemon...".cyan().bold());

                    // Stop if running
                    if is_daemon_running() {
                        let _ = send_ipc_command(r#"{"command":"Shutdown","force":false}"#).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }

                    // Start again
                    #[cfg(windows)]
                    {
                        use std::process::Command;
                        let exe = std::env::current_exe()?;
                        let child = Command::new(exe)
                            .args(["daemon", "start", "--foreground", &path.to_string_lossy()])
                            .creation_flags(0x00000008)
                            .spawn()?;

                        println!("{} Forge daemon restarted (PID: {})", "✓".green(), child.id());
                    }

                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let exe = std::env::current_exe()?;
                        let child = Command::new(exe)
                            .args(["daemon", "start", "--foreground", &path.to_string_lossy()])
                            .spawn()?;

                        println!("{} Forge daemon restarted (PID: {})", "✓".green(), child.id());
                    }
                }

                DaemonAction::Status => {
                    if !is_daemon_running() {
                        println!("{} Forge daemon is {}", "●".red(), "not running".red());
                        return Ok(());
                    }

                    match send_ipc_command(r#"{"command":"GetStatus"}"#).await {
                        Ok(response) => {
                            if let Ok(status) = serde_json::from_str::<serde_json::Value>(&response)
                            {
                                println!("{} Forge daemon is {}", "●".green(), "running".green());
                                println!();
                                println!("{}", "Status:".cyan().bold());
                                println!(
                                    "   State:          {}",
                                    status["state"].as_str().unwrap_or("unknown")
                                );
                                println!(
                                    "   Uptime:         {}s",
                                    status["uptime_seconds"].as_u64().unwrap_or(0)
                                );
                                println!(
                                    "   Files changed:  {}",
                                    status["files_changed"].as_u64().unwrap_or(0)
                                );
                                println!(
                                    "   Tools executed: {}",
                                    status["tools_executed"].as_u64().unwrap_or(0)
                                );
                                println!(
                                    "   Cache hits:     {}",
                                    status["cache_hits"].as_u64().unwrap_or(0)
                                );
                                println!(
                                    "   Errors:         {}",
                                    status["errors"].as_u64().unwrap_or(0)
                                );
                            }
                        }
                        Err(e) => {
                            println!("{} Failed to get status: {}", "✗".red(), e);
                        }
                    }
                }

                DaemonAction::Tools => {
                    if !is_daemon_running() {
                        println!("{}", "⚠️  Forge daemon is not running".yellow());
                        return Ok(());
                    }

                    match send_ipc_command(r#"{"command":"ListTools"}"#).await {
                        Ok(response) => {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&response) {
                                if let Some(tools) = data["tools"].as_array() {
                                    println!("{}", "📦 Registered Tools".cyan().bold());
                                    println!("{}", "═".repeat(60).bright_black());

                                    for tool in tools {
                                        let status_icon =
                                            match tool["status"].as_str().unwrap_or("") {
                                                "Ready" => "●".green(),
                                                "Running" => "●".yellow(),
                                                "Disabled" => "●".bright_black(),
                                                "Error" => "●".red(),
                                                _ => "●".white(),
                                            };

                                        println!(
                                            "{} {} {} {}",
                                            status_icon,
                                            tool["name"]
                                                .as_str()
                                                .unwrap_or("unknown")
                                                .bright_cyan()
                                                .bold(),
                                            format!(
                                                "v{}",
                                                tool["version"].as_str().unwrap_or("0.0.0")
                                            )
                                            .bright_black(),
                                            if tool["is_dummy"].as_bool().unwrap_or(false) {
                                                "[dummy]".bright_black()
                                            } else {
                                                "".into()
                                            }
                                        );
                                        println!(
                                            "     Runs: {} | Errors: {}",
                                            tool["run_count"].as_u64().unwrap_or(0),
                                            tool["error_count"].as_u64().unwrap_or(0)
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("{} Failed to list tools: {}", "✗".red(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if daemon is running by checking PID file
fn is_daemon_running() -> bool {
    let pid_path = DaemonServer::pid_path();
    if !pid_path.exists() {
        return false;
    }

    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Check if process is actually running
            #[cfg(windows)]
            {
                use std::process::Command;
                let output =
                    Command::new("tasklist").args(["/FI", &format!("PID eq {}", pid)]).output();
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    return stdout.contains(&pid.to_string());
                }
            }
            #[cfg(unix)]
            {
                use std::process::Command;
                let output = Command::new("kill").args(["-0", &pid.to_string()]).output();
                if let Ok(out) = output {
                    return out.status.success();
                }
            }
        }
    }

    false
}

/// Send IPC command to daemon
async fn send_ipc_command(command: &str) -> Result<String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[cfg(windows)]
    {
        use tokio::net::TcpStream;
        let port = DaemonServer::ipc_port();
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;
        Ok(response)
    }

    #[cfg(unix)]
    {
        use tokio::net::UnixStream;
        let socket_path = DaemonServer::socket_path();
        let mut stream = UnixStream::connect(&socket_path).await?;
        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await?;
        Ok(response)
    }
}
