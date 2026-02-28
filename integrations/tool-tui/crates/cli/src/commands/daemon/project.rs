//! Project Daemon - On-Demand Per-Project Service
//!
//! # Responsibilities
//! - Project-specific check/build/test operations
//! - File watching with intelligent debouncing
//! - Parallel analysis using rayon
//! - IPC with Agent daemon for R2 sync

use anyhow::Result;
use clap::Args;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::ui::theme::Theme;

/// Global shutdown signal
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Project daemon arguments
#[derive(Args, Debug, Default)]
pub struct ProjectArgs {
    /// Project directory (defaults to current)
    #[arg(long, short)]
    pub path: Option<PathBuf>,

    /// Socket path for IPC
    #[arg(long)]
    pub socket: Option<PathBuf>,

    /// Agent daemon socket to connect to
    #[arg(long, default_value = "/tmp/dx-agent.sock")]
    pub agent_socket: PathBuf,

    /// PID file location
    #[arg(long)]
    pub pidfile: Option<PathBuf>,

    /// Enable watch mode
    #[arg(long, short)]
    pub watch: bool,

    /// File patterns to watch
    #[arg(long)]
    pub include: Vec<String>,

    /// File patterns to ignore
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Debounce interval in milliseconds
    #[arg(long, default_value = "100")]
    pub debounce: u64,

    /// Number of parallel workers (defaults to CPU count)
    #[arg(long, short = 'j')]
    pub jobs: Option<usize>,

    /// Enable verbose logging
    #[arg(long, short)]
    pub verbose: bool,
}

/// Project daemon state
pub struct ProjectState {
    /// Project root directory
    pub project_root: PathBuf,

    /// Detected project type
    pub project_type: ProjectType,

    /// File change queue
    pub change_queue: Vec<FileChange>,

    /// Last check results
    pub last_results: Option<CheckResults>,

    /// Check task queue
    pub task_queue: Vec<CheckTask>,

    /// Active workers
    pub active_workers: usize,

    /// Connection to agent daemon
    pub agent_connection: Option<AgentConnection>,

    /// Project metrics
    pub metrics: ProjectMetrics,

    /// File cache for incremental analysis
    pub file_cache: HashMap<PathBuf, FileCacheEntry>,
}

/// Detected project type
#[derive(Debug, Clone)]
pub enum ProjectType {
    Rust {
        cargo_toml: PathBuf,
    },
    JavaScript {
        package_json: PathBuf,
    },
    TypeScript {
        package_json: PathBuf,
        tsconfig: PathBuf,
    },
    Python {
        pyproject: Option<PathBuf>,
        requirements: Option<PathBuf>,
    },
    Go {
        go_mod: PathBuf,
    },
    Mixed {
        types: Vec<ProjectType>,
    },
    Unknown,
}

/// File change event
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: FileChangeKind,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Check results from analysis
#[derive(Debug, Clone)]
pub struct CheckResults {
    pub score: u32,
    pub format_issues: Vec<FormatIssue>,
    pub lint_issues: Vec<LintIssue>,
    pub test_results: TestSummary,
    pub coverage: CoverageSummary,
    pub duration_ms: u64,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct FormatIssue {
    pub file: PathBuf,
    pub line: u32,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Default)]
pub struct TestSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CoverageSummary {
    pub line_percent: f32,
    pub branch_percent: f32,
    pub function_percent: f32,
}

/// Check task for queue
#[derive(Debug, Clone)]
pub struct CheckTask {
    pub id: u64,
    pub kind: CheckTaskKind,
    pub files: Vec<PathBuf>,
    pub priority: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CheckTaskKind {
    Format,
    Lint,
    Test,
    Coverage,
    Full,
}

/// Connection to agent daemon
pub struct AgentConnection {
    pub socket_path: PathBuf,
    pub connected: bool,
    pub last_heartbeat: Instant,
}

/// Project metrics
#[derive(Debug, Default)]
pub struct ProjectMetrics {
    pub checks_run: u64,
    pub files_analyzed: u64,
    pub issues_found: u64,
    pub issues_fixed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// File cache entry for incremental analysis
#[derive(Debug, Clone)]
pub struct FileCacheEntry {
    pub hash: u64,
    pub last_modified: std::time::SystemTime,
    pub last_check: Instant,
    pub issues: Vec<LintIssue>,
}

/// Run the project daemon
pub async fn run(args: ProjectArgs, _theme: &Theme) -> Result<()> {
    use owo_colors::OwoColorize;

    let project_root = args.path.clone().unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("{}", "╔════════════════════════════════════════════╗".cyan());
    println!("{}", "║        DX Project Daemon Starting          ║".cyan());
    println!("{}", "╚════════════════════════════════════════════╝".cyan());

    // Detect project type
    let project_type = detect_project_type(&project_root)?;
    println!("{} Project: {}", "●".green(), project_root.display());
    println!("{} Type: {:?}", "●".green(), project_type);

    // Initialize state
    let state = Arc::new(RwLock::new(ProjectState {
        project_root: project_root.clone(),
        project_type,
        change_queue: vec![],
        last_results: None,
        task_queue: vec![],
        active_workers: 0,
        agent_connection: None,
        metrics: ProjectMetrics::default(),
        file_cache: HashMap::new(),
    }));

    // Set up worker pool
    let num_workers = args.jobs.unwrap_or_else(num_cpus::get);
    println!("{} Workers: {}", "●".green(), num_workers);
    rayon::ThreadPoolBuilder::new().num_threads(num_workers).build_global().ok();

    // Connect to agent daemon
    let state_clone = Arc::clone(&state);
    let agent_socket = args.agent_socket.clone();
    tokio::spawn(async move {
        agent_connection_loop(state_clone, agent_socket).await;
    });

    // Set up file watcher if enabled
    if args.watch {
        let state_clone = Arc::clone(&state);
        let debounce = args.debounce;
        tokio::spawn(async move {
            file_watch_loop(state_clone, debounce).await;
        });
        println!("{} Watch mode enabled ({}ms debounce)", "●".green(), args.debounce);
    }

    // Task processing loop
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        task_processor_loop(state_clone).await;
    });

    // Write PID file if specified
    if let Some(ref pidfile) = args.pidfile {
        write_pidfile(pidfile)?;
    }

    // Socket path
    let socket_path = args.socket.unwrap_or_else(|| {
        let hash = hash_path(&project_root);
        PathBuf::from(format!("/tmp/dx-project-{}.sock", hash))
    });
    println!("{} Socket: {}", "●".green(), socket_path.display());

    // Main IPC loop
    println!("{} Project daemon ready", "✓".green());
    ipc_server_loop(&socket_path, state).await?;

    // Cleanup
    if let Some(ref pidfile) = args.pidfile {
        cleanup_pidfile(pidfile)?;
    }
    println!("{} Project daemon stopped", "●".yellow());

    Ok(())
}

/// Detect project type from directory
fn detect_project_type(root: &PathBuf) -> Result<ProjectType> {
    let mut types = vec![];

    // Check for Rust
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.exists() {
        types.push(ProjectType::Rust { cargo_toml });
    }

    // Check for JavaScript/TypeScript
    let package_json = root.join("package.json");
    if package_json.exists() {
        let tsconfig = root.join("tsconfig.json");
        if tsconfig.exists() {
            types.push(ProjectType::TypeScript {
                package_json: package_json.clone(),
                tsconfig,
            });
        } else {
            types.push(ProjectType::JavaScript { package_json });
        }
    }

    // Check for Python
    let pyproject = root.join("pyproject.toml");
    let requirements = root.join("requirements.txt");
    if pyproject.exists() || requirements.exists() {
        types.push(ProjectType::Python {
            pyproject: if pyproject.exists() {
                Some(pyproject)
            } else {
                None
            },
            requirements: if requirements.exists() {
                Some(requirements)
            } else {
                None
            },
        });
    }

    // Check for Go
    let go_mod = root.join("go.mod");
    if go_mod.exists() {
        types.push(ProjectType::Go { go_mod });
    }

    Ok(match types.len() {
        0 => ProjectType::Unknown,
        1 => types.pop().unwrap(),
        _ => ProjectType::Mixed { types },
    })
}

/// Agent connection maintenance loop
async fn agent_connection_loop(state: Arc<RwLock<ProjectState>>, agent_socket: PathBuf) {
    let reconnect_interval = Duration::from_secs(5);
    let heartbeat_interval = Duration::from_secs(30);

    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }

        let connected = {
            let state = state.read().await;
            state.agent_connection.as_ref().map_or(false, |c| c.connected)
        };

        if !connected {
            // Try to connect
            if agent_socket.exists() {
                let mut state = state.write().await;
                state.agent_connection = Some(AgentConnection {
                    socket_path: agent_socket.clone(),
                    connected: true,
                    last_heartbeat: Instant::now(),
                });
            }
        } else {
            // Send heartbeat
            // TODO: Implement actual IPC heartbeat
        }

        tokio::time::sleep(if connected {
            heartbeat_interval
        } else {
            reconnect_interval
        })
        .await;
    }
}

/// File watch loop with debouncing
async fn file_watch_loop(state: Arc<RwLock<ProjectState>>, debounce_ms: u64) {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;

    let (tx, rx) = channel();

    let project_root = {
        let state = state.read().await;
        state.project_root.clone()
    };

    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
    watcher.watch(&project_root, RecursiveMode::Recursive).unwrap();

    let debounce = Duration::from_millis(debounce_ms);
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }

        // Process incoming events
        while let Ok(event) = rx.try_recv() {
            if let Ok(event) = event {
                for path in event.paths {
                    pending.insert(path, Instant::now());
                }
            }
        }

        // Process debounced changes
        let now = Instant::now();
        let ready: Vec<_> = pending
            .iter()
            .filter(|(_, timestamp)| now.duration_since(**timestamp) >= debounce)
            .map(|(path, _)| path.clone())
            .collect();

        if !ready.is_empty() {
            let mut state = state.write().await;
            for path in ready {
                pending.remove(&path);
                state.change_queue.push(FileChange {
                    path,
                    kind: FileChangeKind::Modified,
                    timestamp: now,
                });
            }

            // Queue a check task
            let files: Vec<_> = state.change_queue.iter().map(|c| c.path.clone()).collect();
            state.task_queue.push(CheckTask {
                id: rand::random(),
                kind: CheckTaskKind::Full,
                files,
                priority: 1,
            });
            state.change_queue.clear();
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Task processor loop using rayon for parallelism
async fn task_processor_loop(state: Arc<RwLock<ProjectState>>) {
    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }

        let task = {
            let mut state = state.write().await;
            // Sort by priority and take highest
            state.task_queue.sort_by(|a, b| b.priority.cmp(&a.priority));
            state.task_queue.pop()
        };

        if let Some(task) = task {
            // Process task using rayon
            let results = process_check_task(&task).await;

            let mut state = state.write().await;
            state.last_results = Some(results);
            state.metrics.checks_run += 1;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Process a check task with parallel analysis
async fn process_check_task(task: &CheckTask) -> CheckResults {
    use rayon::prelude::*;

    let start = Instant::now();

    let (format_issues, lint_issues) = rayon::join(
        || task.files.par_iter().flat_map(|file| check_format(file)).collect::<Vec<_>>(),
        || task.files.par_iter().flat_map(|file| check_lint(file)).collect::<Vec<_>>(),
    );

    let score = calculate_score(&format_issues, &lint_issues);

    CheckResults {
        score,
        format_issues,
        lint_issues,
        test_results: TestSummary::default(),
        coverage: CoverageSummary::default(),
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: Instant::now(),
    }
}

fn check_format(_file: &PathBuf) -> Vec<FormatIssue> {
    // TODO: Implement actual format checking
    vec![]
}

fn check_lint(_file: &PathBuf) -> Vec<LintIssue> {
    // TODO: Implement actual lint checking
    vec![]
}

fn calculate_score(format_issues: &[FormatIssue], lint_issues: &[LintIssue]) -> u32 {
    let mut score = 500;

    // Deduct for format issues
    score -= std::cmp::min(format_issues.len() as u32 * 2, 100);

    // Deduct for lint issues
    for issue in lint_issues {
        let penalty = match issue.severity {
            Severity::Error => 10,
            Severity::Warning => 5,
            Severity::Info => 2,
            Severity::Hint => 1,
        };
        score = score.saturating_sub(penalty);
    }

    score
}

/// IPC server loop
async fn ipc_server_loop(socket_path: &PathBuf, _state: Arc<RwLock<ProjectState>>) -> Result<()> {
    // Remove existing socket
    let _ = std::fs::remove_file(socket_path);

    // TODO: Implement Unix socket server for IPC
    // For now, just wait for shutdown
    while !SHUTDOWN.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

fn hash_path(path: &PathBuf) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn write_pidfile(path: &PathBuf) -> Result<()> {
    let pid = std::process::id();
    std::fs::write(path, pid.to_string())?;
    Ok(())
}

fn cleanup_pidfile(path: &PathBuf) -> Result<()> {
    let _ = std::fs::remove_file(path);
    Ok(())
}

/// Graceful shutdown
pub async fn graceful_stop() -> Result<()> {
    SHUTDOWN.store(true, Ordering::Relaxed);
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(())
}

/// Force stop
pub async fn force_stop() -> Result<()> {
    SHUTDOWN.store(true, Ordering::Relaxed);
    Ok(())
}
