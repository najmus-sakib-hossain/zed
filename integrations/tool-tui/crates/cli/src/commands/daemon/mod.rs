//! Dual Daemon Architecture - Agent 24/7 + Project On-Demand
//!
//! # Architecture
//! - **Agent Daemon**: Always-running, manages R2 sync, AI updates, global state
//! - **Project Daemon**: On-demand per project, handles check/build/test operations
//!
//! # Communication
//! All inter-daemon communication uses .sr (serializer) files for efficiency

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::ui::theme::Theme;

pub mod agent;
pub mod health;
pub mod ipc;
pub mod project;

/// Daemon management commands
#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommands,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommands {
    /// Start the agent daemon (24/7 background service)
    Agent(agent::AgentArgs),

    /// Start project daemon for current directory
    Project(project::ProjectArgs),

    /// Check daemon health status
    Status(health::StatusArgs),

    /// Stop all running daemons
    Stop(StopArgs),

    /// Restart daemons
    Restart(RestartArgs),
}

#[derive(Args, Debug)]
pub struct StopArgs {
    /// Stop agent daemon
    #[arg(long)]
    pub agent: bool,

    /// Stop project daemon
    #[arg(long)]
    pub project: bool,

    /// Stop all daemons
    #[arg(long, short)]
    pub all: bool,

    /// Force stop without graceful shutdown
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct RestartArgs {
    /// Restart agent daemon
    #[arg(long)]
    pub agent: bool,

    /// Restart project daemon
    #[arg(long)]
    pub project: bool,
}

/// Run daemon commands
pub async fn run(args: DaemonArgs, theme: &Theme) -> Result<()> {
    match args.command {
        DaemonCommands::Agent(args) => agent::run(args, theme).await,
        DaemonCommands::Project(args) => project::run(args, theme).await,
        DaemonCommands::Status(args) => health::run_status(args, theme).await,
        DaemonCommands::Stop(args) => run_stop(args, theme).await,
        DaemonCommands::Restart(args) => run_restart(args, theme).await,
    }
}

async fn run_stop(args: StopArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    if args.all || args.agent {
        println!("{} Stopping agent daemon...", "●".yellow());
        if args.force {
            agent::force_stop().await?;
        } else {
            agent::graceful_stop().await?;
        }
        println!("{} Agent daemon stopped", "✓".green());
    }

    if args.all || args.project {
        println!("{} Stopping project daemon...", "●".yellow());
        if args.force {
            project::force_stop().await?;
        } else {
            project::graceful_stop().await?;
        }
        println!("{} Project daemon stopped", "✓".green());
    }

    Ok(())
}

async fn run_restart(args: RestartArgs, theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    if args.agent {
        println!("{} Restarting agent daemon...", "●".yellow());
        agent::graceful_stop().await?;
        agent::run(agent::AgentArgs::default(), theme).await?;
    }

    if args.project {
        println!("{} Restarting project daemon...", "●".yellow());
        project::graceful_stop().await?;
        project::run(project::ProjectArgs::default(), theme).await?;
    }

    Ok(())
}
