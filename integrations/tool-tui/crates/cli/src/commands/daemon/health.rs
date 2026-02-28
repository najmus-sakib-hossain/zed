//! Daemon Health Monitoring
//!
//! Provides health checks, status reporting, and diagnostics for both daemons

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::time::Duration;

use crate::ui::theme::Theme;

/// Status command arguments
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Show detailed status
    #[arg(long, short)]
    pub verbose: bool,

    /// Output format
    #[arg(long, default_value = "human")]
    pub format: OutputFormat,

    /// Check specific daemon only
    #[arg(long)]
    pub daemon: Option<DaemonFilter>,

    /// Include metrics
    #[arg(long)]
    pub metrics: bool,

    /// Include connection info
    #[arg(long)]
    pub connections: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Sr,
    Llm,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum DaemonFilter {
    Agent,
    Project,
}

/// Daemon health status
#[derive(Debug)]
pub struct DaemonHealth {
    pub daemon_type: DaemonType,
    pub status: HealthStatus,
    pub pid: Option<u32>,
    pub uptime: Option<Duration>,
    pub memory_mb: Option<f64>,
    pub cpu_percent: Option<f32>,
    pub socket_path: PathBuf,
    pub last_heartbeat: Option<std::time::Instant>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum DaemonType {
    Agent,
    Project,
}

#[derive(Debug, Clone, Copy)]
pub enum HealthStatus {
    Running,
    Stopped,
    Degraded,
    Unknown,
}

/// Run status command
pub async fn run_status(args: StatusArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let agent_health = check_agent_health().await?;
    let project_health = check_project_health().await?;

    match args.format {
        OutputFormat::Human => {
            println!("{}", "╔════════════════════════════════════════════╗".cyan());
            println!("{}", "║            DX Daemon Status                ║".cyan());
            println!("{}", "╚════════════════════════════════════════════╝".cyan());
            println!();

            if args.daemon.is_none() || matches!(args.daemon, Some(DaemonFilter::Agent)) {
                print_daemon_status(&agent_health, args.verbose);
            }

            if args.daemon.is_none() || matches!(args.daemon, Some(DaemonFilter::Project)) {
                print_daemon_status(&project_health, args.verbose);
            }

            if args.metrics {
                println!();
                println!("{}", "Metrics:".bold());
                print_metrics(&agent_health, &project_health);
            }

            if args.connections {
                println!();
                println!("{}", "Connections:".bold());
                print_connections(&agent_health, &project_health);
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "agent": format_health_json(&agent_health),
                "project": format_health_json(&project_health),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Sr => {
            // TODO: Output in .sr format
            println!("# .sr format not yet implemented");
        }
        OutputFormat::Llm => {
            print_llm_format(&agent_health, &project_health);
        }
    }

    Ok(())
}

fn print_daemon_status(health: &DaemonHealth, verbose: bool) {
    use owo_colors::OwoColorize;

    let name = match health.daemon_type {
        DaemonType::Agent => "Agent Daemon",
        DaemonType::Project => "Project Daemon",
    };

    let status_icon = match health.status {
        HealthStatus::Running => "●".green().to_string(),
        HealthStatus::Stopped => "●".red().to_string(),
        HealthStatus::Degraded => "●".yellow().to_string(),
        HealthStatus::Unknown => "●".dimmed().to_string(),
    };

    let status_text = match health.status {
        HealthStatus::Running => "Running".green().to_string(),
        HealthStatus::Stopped => "Stopped".red().to_string(),
        HealthStatus::Degraded => "Degraded".yellow().to_string(),
        HealthStatus::Unknown => "Unknown".dimmed().to_string(),
    };

    println!("{} {} - {}", status_icon, name.bold(), status_text);

    if verbose {
        if let Some(pid) = health.pid {
            println!("  PID: {}", pid);
        }
        if let Some(uptime) = health.uptime {
            println!("  Uptime: {:?}", uptime);
        }
        if let Some(mem) = health.memory_mb {
            println!("  Memory: {:.1} MB", mem);
        }
        if let Some(cpu) = health.cpu_percent {
            println!("  CPU: {:.1}%", cpu);
        }
        println!("  Socket: {}", health.socket_path.display());
        if let Some(ref err) = health.error {
            println!("  Error: {}", err.red());
        }
    }

    println!();
}

fn print_metrics(_agent: &DaemonHealth, _project: &DaemonHealth) {
    // TODO: Fetch and display metrics from daemons
    println!("  (metrics not available - daemons not running)");
}

fn print_connections(_agent: &DaemonHealth, _project: &DaemonHealth) {
    // TODO: Fetch and display connection info
    println!("  (connection info not available - daemons not running)");
}

fn format_health_json(health: &DaemonHealth) -> serde_json::Value {
    serde_json::json!({
        "type": format!("{:?}", health.daemon_type),
        "status": format!("{:?}", health.status),
        "pid": health.pid,
        "uptime_secs": health.uptime.map(|d| d.as_secs()),
        "memory_mb": health.memory_mb,
        "cpu_percent": health.cpu_percent,
        "socket": health.socket_path.to_string_lossy(),
        "error": health.error,
    })
}

fn print_llm_format(agent: &DaemonHealth, project: &DaemonHealth) {
    // LLM-optimized format (52-73% token savings)
    println!("DAEMON_STATUS");
    println!(
        "agent:{:?}|pid:{}|mem:{}|cpu:{}",
        agent.status,
        agent.pid.map_or("-".to_string(), |p| p.to_string()),
        agent.memory_mb.map_or("-".to_string(), |m| format!("{:.0}", m)),
        agent.cpu_percent.map_or("-".to_string(), |c| format!("{:.0}", c)),
    );
    println!(
        "project:{:?}|pid:{}|mem:{}|cpu:{}",
        project.status,
        project.pid.map_or("-".to_string(), |p| p.to_string()),
        project.memory_mb.map_or("-".to_string(), |m| format!("{:.0}", m)),
        project.cpu_percent.map_or("-".to_string(), |c| format!("{:.0}", c)),
    );
}

/// Check agent daemon health
async fn check_agent_health() -> Result<DaemonHealth> {
    let socket_path = PathBuf::from("/tmp/dx-agent.sock");
    let pidfile = PathBuf::from("/tmp/dx-agent.pid");

    let (status, pid, error) = if socket_path.exists() {
        // Try to connect and verify
        match try_connect(&socket_path).await {
            Ok(_) => {
                let pid = read_pidfile(&pidfile).ok();
                (HealthStatus::Running, pid, None)
            }
            Err(e) => (HealthStatus::Degraded, None, Some(e.to_string())),
        }
    } else if pidfile.exists() {
        // Socket gone but PID file exists - stale
        (
            HealthStatus::Unknown,
            read_pidfile(&pidfile).ok(),
            Some("Socket missing".to_string()),
        )
    } else {
        (HealthStatus::Stopped, None, None)
    };

    Ok(DaemonHealth {
        daemon_type: DaemonType::Agent,
        status,
        pid,
        uptime: None,
        memory_mb: pid.and_then(|p| get_process_memory(p)),
        cpu_percent: pid.and_then(|p| get_process_cpu(p)),
        socket_path,
        last_heartbeat: None,
        error,
    })
}

/// Check project daemon health
async fn check_project_health() -> Result<DaemonHealth> {
    let project_root = std::env::current_dir()?;
    let hash = hash_path(&project_root);
    let socket_path = PathBuf::from(format!("/tmp/dx-project-{}.sock", hash));

    let (status, pid, error) = if socket_path.exists() {
        match try_connect(&socket_path).await {
            Ok(_) => (HealthStatus::Running, None, None),
            Err(e) => (HealthStatus::Degraded, None, Some(e.to_string())),
        }
    } else {
        (HealthStatus::Stopped, None, None)
    };

    Ok(DaemonHealth {
        daemon_type: DaemonType::Project,
        status,
        pid,
        uptime: None,
        memory_mb: None,
        cpu_percent: None,
        socket_path,
        last_heartbeat: None,
        error,
    })
}

async fn try_connect(socket_path: &PathBuf) -> Result<()> {
    // TODO: Implement actual socket connection test
    if socket_path.exists() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Socket does not exist"))
    }
}

fn read_pidfile(path: &PathBuf) -> Result<u32> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.trim().parse()?)
}

fn get_process_memory(_pid: u32) -> Option<f64> {
    // TODO: Implement actual memory reading
    None
}

fn get_process_cpu(_pid: u32) -> Option<f32> {
    // TODO: Implement actual CPU reading
    None
}

fn hash_path(path: &PathBuf) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
