//! CLI Module
//!
//! Command-line interface for dx-project-manager workspace management.

use crate::affected::AffectedDetector;
use crate::bag::AffectedGraphData;
use crate::cache::CacheManager;
use crate::executor::TaskExecutor;
use crate::ghost::{GhostDetector, GhostReport};
use crate::watch::WatchManager;
use crate::workspace::WorkspaceManager;
use std::env;
use std::path::PathBuf;

/// CLI command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Initialize workspace
    Init,
    /// Run a task
    Run {
        task: String,
        filter: Option<String>,
    },
    /// Show affected packages
    Affected {
        base: Option<String>,
        head: Option<String>,
    },
    /// Detect ghost dependencies
    Ghost,
    /// Watch mode
    Watch { task: String },
    /// Cache management
    Cache { subcommand: CacheSubcommand },
    /// Show help
    Help,
    /// Show version
    Version,
}

/// Cache subcommands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheSubcommand {
    /// Show cache status
    Status,
    /// Clear cache
    Clear,
}

/// CLI result
#[derive(Debug)]
pub struct CliResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Output message
    pub message: String,
}

impl CliResult {
    /// Create a success result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            message: message.into(),
        }
    }

    /// Create an error result
    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            exit_code: code,
            message: message.into(),
        }
    }
}

/// CLI parser and executor
pub struct Cli {
    /// Working directory
    cwd: PathBuf,
    /// Workspace manager
    workspace: WorkspaceManager,
    /// Task executor
    executor: TaskExecutor,
    /// Cache manager
    cache: Option<CacheManager>,
    /// Watch manager
    watch: WatchManager,
}

impl Cli {
    /// Create a new CLI instance
    pub fn new() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            workspace: WorkspaceManager::new(),
            executor: TaskExecutor::new(),
            cache: None,
            watch: WatchManager::new(),
        }
    }

    /// Create CLI with custom working directory
    pub fn with_cwd(cwd: PathBuf) -> Self {
        Self {
            cwd,
            workspace: WorkspaceManager::new(),
            executor: TaskExecutor::new(),
            cache: None,
            watch: WatchManager::new(),
        }
    }

    /// Parse command line arguments
    pub fn parse_args(args: &[String]) -> Result<Command, String> {
        if args.is_empty() {
            return Ok(Command::Help);
        }

        match args[0].as_str() {
            "init" => Ok(Command::Init),
            "run" => {
                if args.len() < 2 {
                    return Err("run command requires a task name".to_string());
                }
                let task = args[1].clone();
                let filter = Self::parse_flag(&args[2..], "--filter");
                Ok(Command::Run { task, filter })
            }
            "affected" => {
                let base = Self::parse_flag(&args[1..], "--base");
                let head = Self::parse_flag(&args[1..], "--head");
                Ok(Command::Affected { base, head })
            }
            "ghost" => Ok(Command::Ghost),
            "watch" => {
                if args.len() < 2 {
                    return Err("watch command requires a task name".to_string());
                }
                Ok(Command::Watch {
                    task: args[1].clone(),
                })
            }
            "cache" => {
                if args.len() < 2 {
                    return Err("cache command requires a subcommand (status|clear)".to_string());
                }
                let subcommand = match args[1].as_str() {
                    "status" => CacheSubcommand::Status,
                    "clear" => CacheSubcommand::Clear,
                    other => return Err(format!("unknown cache subcommand: {}", other)),
                };
                Ok(Command::Cache { subcommand })
            }
            "help" | "--help" | "-h" => Ok(Command::Help),
            "version" | "--version" | "-V" => Ok(Command::Version),
            other => Err(format!("unknown command: {}", other)),
        }
    }

    /// Parse a flag value from arguments
    fn parse_flag(args: &[String], flag: &str) -> Option<String> {
        for (i, arg) in args.iter().enumerate() {
            if arg == flag && i + 1 < args.len() {
                return Some(args[i + 1].clone());
            }
            if let Some(value) = arg.strip_prefix(&format!("{}=", flag)) {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Execute a command
    pub fn execute(&mut self, command: Command) -> CliResult {
        match command {
            Command::Init => self.cmd_init(),
            Command::Run { task, filter } => self.cmd_run(&task, filter.as_deref()),
            Command::Affected { base, head } => self.cmd_affected(base.as_deref(), head.as_deref()),
            Command::Ghost => self.cmd_ghost(),
            Command::Watch { task } => self.cmd_watch(&task),
            Command::Cache { subcommand } => self.cmd_cache(subcommand),
            Command::Help => self.cmd_help(),
            Command::Version => self.cmd_version(),
        }
    }

    /// Initialize workspace (Task 25.1)
    fn cmd_init(&mut self) -> CliResult {
        // Check if already initialized
        let manifest_path = self.cwd.join(".dx-project-manager").join("workspace.bwm");
        if manifest_path.exists() {
            return CliResult::error(1, "Workspace already initialized");
        }

        // Create workspace directory
        let workspace_dir = self.cwd.join(".dx-project-manager");
        if let Err(e) = std::fs::create_dir_all(&workspace_dir) {
            return CliResult::error(1, format!("Failed to create workspace directory: {}", e));
        }

        // Regenerate manifest from package.json files
        match self.workspace.regenerate() {
            Ok(()) => CliResult::success("Workspace initialized successfully"),
            Err(e) => CliResult::error(1, format!("Failed to initialize workspace: {}", e)),
        }
    }

    /// Run a task (Task 25.2)
    fn cmd_run(&mut self, task: &str, filter: Option<&str>) -> CliResult {
        // Load workspace if not loaded
        if !self.workspace.is_loaded() {
            let manifest_path = self.cwd.join(".dx-project-manager").join("workspace.bwm");
            if let Err(e) = self.workspace.load(&manifest_path) {
                return CliResult::error(1, format!("Failed to load workspace: {}", e));
            }
        }

        // Load task graph
        let graph_path = self.cwd.join(".dx-project-manager").join("tasks.btg");
        if let Err(e) = self.executor.load(&graph_path) {
            return CliResult::error(1, format!("Failed to load task graph: {}", e));
        }

        // Get packages to run (filtered or all)
        let packages: Vec<u32> = if let Some(filter_pattern) = filter {
            self.filter_packages(filter_pattern)
        } else {
            (0..self.workspace.package_count() as u32).collect()
        };

        // Execute tasks in topological order
        let mut outputs = Vec::new();
        for pkg_idx in &packages {
            if let Some(_task_data) = self.executor.get_task(*pkg_idx, task) {
                // Find task index
                let task_idx = self.find_task_index(*pkg_idx, task);
                if let Some(idx) = task_idx {
                    match self.executor.execute(idx) {
                        Ok(output) => outputs.push(output),
                        Err(e) => {
                            return CliResult::error(1, format!("Task failed: {}", e));
                        }
                    }
                }
            }
        }

        CliResult::success(format!("Executed {} tasks successfully", outputs.len()))
    }

    /// Show affected packages (Task 25.3)
    fn cmd_affected(&mut self, base: Option<&str>, head: Option<&str>) -> CliResult {
        // Load workspace if not loaded
        if !self.workspace.is_loaded() {
            let manifest_path = self.cwd.join(".dx-project-manager").join("workspace.bwm");
            if let Err(e) = self.workspace.load(&manifest_path) {
                return CliResult::error(1, format!("Failed to load workspace: {}", e));
            }
        }

        // Get changed files from git diff
        let changed_files = self.get_git_changed_files(base, head);

        // Create affected detector
        let graph = self.build_affected_graph();
        let detector = AffectedDetector::new(graph);

        // Get affected packages
        let affected = detector.affected(&changed_files);

        if affected.is_empty() {
            return CliResult::success("No packages affected");
        }

        // Build output
        let mut output = String::from("Affected packages:\n");
        for pkg_idx in &affected {
            if let Some(pkg) = self.workspace.get_package_by_index(*pkg_idx) {
                output.push_str(&format!("  - {}\n", pkg.name));
            }
        }

        CliResult::success(output)
    }

    /// Detect ghost dependencies (Task 25.4)
    fn cmd_ghost(&mut self) -> CliResult {
        // Load workspace if not loaded
        if !self.workspace.is_loaded() {
            let manifest_path = self.cwd.join(".dx-project-manager").join("workspace.bwm");
            if let Err(e) = self.workspace.load(&manifest_path) {
                return CliResult::error(1, format!("Failed to load workspace: {}", e));
            }
        }

        // Create ghost detector
        let mut detector = GhostDetector::new();

        // Configure detector with workspace packages
        for idx in 0..self.workspace.package_count() as u32 {
            if let Some(pkg) = self.workspace.get_package_by_index(idx) {
                detector.add_workspace_package(pkg.name.clone());
                detector.set_package_path(idx, self.cwd.join(&pkg.path));

                // Set declared dependencies
                let deps: std::collections::HashSet<String> =
                    pkg.dependencies.iter().cloned().collect();
                detector.set_declared_deps(idx, deps);
            }
        }

        // Scan for ghosts
        match detector.scan() {
            Ok(report) => self.format_ghost_report(&report),
            Err(e) => CliResult::error(1, format!("Ghost detection failed: {}", e)),
        }
    }

    /// Watch mode (Task 25.5)
    fn cmd_watch(&mut self, task: &str) -> CliResult {
        // Load workspace if not loaded
        if !self.workspace.is_loaded() {
            let manifest_path = self.cwd.join(".dx-project-manager").join("workspace.bwm");
            if let Err(e) = self.workspace.load(&manifest_path) {
                return CliResult::error(1, format!("Failed to load workspace: {}", e));
            }
        }

        // Configure watch manager
        let _task_name = task.to_string();
        self.watch.on_predicted_change(move |_path| {
            // Return task indices that should run
            // In a real implementation, this would use the affected detector
            vec![]
        });

        // Add workspace paths to watch
        for idx in 0..self.workspace.package_count() as u32 {
            if let Some(pkg) = self.workspace.get_package_by_index(idx) {
                self.watch.watch_path(self.cwd.join(&pkg.path));
            }
        }

        // Start watching
        match self.watch.start() {
            Ok(()) => CliResult::success(format!("Watching for changes... (task: {})", task)),
            Err(e) => CliResult::error(1, format!("Failed to start watch: {}", e)),
        }
    }

    /// Cache management (Task 25.6)
    fn cmd_cache(&mut self, subcommand: CacheSubcommand) -> CliResult {
        // Initialize cache if needed
        if self.cache.is_none() {
            let cache_dir = self.cwd.join(".dx-project-manager").join("cache");
            self.cache = Some(CacheManager::new(cache_dir, 1024 * 1024 * 1024));
            // 1GB
        }

        let cache = self.cache.as_mut().unwrap();

        match subcommand {
            CacheSubcommand::Status => {
                let size = cache.size();
                let size_mb = size as f64 / (1024.0 * 1024.0);
                CliResult::success(format!("Cache size: {:.2} MB", size_mb))
            }
            CacheSubcommand::Clear => match cache.clear() {
                Ok(()) => CliResult::success("Cache cleared"),
                Err(e) => CliResult::error(1, format!("Failed to clear cache: {}", e)),
            },
        }
    }

    /// Show help
    fn cmd_help(&self) -> CliResult {
        let help = r#"dx-project-manager - Binary-first project management

USAGE:
    dx-project-manager <COMMAND> [OPTIONS]

COMMANDS:
    init                Initialize workspace
    run <task>          Execute task pipeline
        --filter <pattern>  Filter packages by pattern
    affected            Show affected packages
        --base <ref>        Git base reference (default: HEAD~1)
        --head <ref>        Git head reference (default: HEAD)
    ghost               Detect ghost dependencies
    watch <task>        Watch mode for continuous builds
    cache <subcommand>  Cache management
        status              Show cache statistics
        clear               Clear local cache
    help                Show this help message
    version             Show version information

EXAMPLES:
    dx-project-manager init
    dx-project-manager run build
    dx-project-manager run test --filter=@myorg/*
    dx-project-manager affected --base=main
    dx-project-manager ghost
    dx-project-manager watch build
    dx-project-manager cache status
    dx-project-manager cache clear
"#;
        CliResult::success(help)
    }

    /// Show version
    fn cmd_version(&self) -> CliResult {
        CliResult::success(format!("dx-project-manager {}", env!("CARGO_PKG_VERSION")))
    }

    // Helper methods

    /// Filter packages by pattern
    fn filter_packages(&self, pattern: &str) -> Vec<u32> {
        let mut result = Vec::new();

        for idx in 0..self.workspace.package_count() as u32 {
            if let Some(pkg) = self.workspace.get_package_by_index(idx) {
                if Self::matches_pattern(&pkg.name, pattern) {
                    result.push(idx);
                }
            }
        }

        result
    }

    /// Check if package name matches filter pattern
    fn matches_pattern(name: &str, pattern: &str) -> bool {
        // Simple glob matching
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix("*") {
            return name.starts_with(prefix);
        }

        if let Some(suffix) = pattern.strip_prefix("*") {
            return name.ends_with(suffix);
        }

        name == pattern
    }

    /// Find task index by package and name
    fn find_task_index(&self, package_idx: u32, task_name: &str) -> Option<u32> {
        // In a real implementation, this would use the task index
        // For now, iterate through tasks
        for idx in 0..self.executor.task_count() as u32 {
            if let Some(task) = self.executor.get_task_by_index(idx) {
                if task.package_idx == package_idx && task.name == task_name {
                    return Some(idx);
                }
            }
        }
        None
    }

    /// Get changed files from git diff
    fn get_git_changed_files(&self, base: Option<&str>, head: Option<&str>) -> Vec<PathBuf> {
        let base_ref = base.unwrap_or("HEAD~1");
        let head_ref = head.unwrap_or("HEAD");

        // Run git diff to get changed files
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", base_ref, head_ref])
            .current_dir(&self.cwd)
            .output();

        match output {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| self.cwd.join(line))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Build affected graph from workspace
    fn build_affected_graph(&self) -> AffectedGraphData {
        let pkg_count = self.workspace.package_count();

        // Collect dependency edges
        let mut edges = Vec::new();
        for idx in 0..pkg_count as u32 {
            for dep_idx in self.workspace.dependencies(idx) {
                edges.push((idx, dep_idx));
            }
        }

        let mut graph = AffectedGraphData::from_edges(pkg_count as u32, &edges);

        // Add file mappings
        for idx in 0..pkg_count as u32 {
            if let Some(pkg) = self.workspace.get_package_by_index(idx) {
                // Map package path to package index
                graph.add_file_mapping(&pkg.path, idx);
            }
        }

        graph
    }

    /// Format ghost report for output
    fn format_ghost_report(&self, report: &GhostReport) -> CliResult {
        if report.ghosts.is_empty() && report.hoisting_accidents.is_empty() {
            return CliResult::success("No ghost dependencies found");
        }

        let mut output = String::new();

        if !report.ghosts.is_empty() {
            output.push_str(&format!("Found {} ghost dependencies:\n", report.ghosts.len()));
            for ghost in &report.ghosts {
                output.push_str(&format!(
                    "  {} in {}:{}:{}\n",
                    ghost.package_name,
                    ghost.importing_file.display(),
                    ghost.line,
                    ghost.column
                ));
            }
        }

        if !report.hoisting_accidents.is_empty() {
            output.push_str(&format!(
                "\nFound {} hoisting accidents:\n",
                report.hoisting_accidents.len()
            ));
            for accident in &report.hoisting_accidents {
                output.push_str(&format!(
                    "  {} (declared by {})\n",
                    accident.dependency, accident.declared_by
                ));
            }
        }

        if !report.vulnerabilities.is_empty() {
            output.push_str(&format!(
                "\nFound {} vulnerabilities in ghost deps:\n",
                report.vulnerabilities.len()
            ));
            for vuln in &report.vulnerabilities {
                output.push_str(&format!(
                    "  {} [{}]: {}\n",
                    vuln.package, vuln.severity, vuln.description
                ));
            }
        }

        CliResult::error(1, output)
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

/// Run CLI with command line arguments
pub fn run(args: &[String]) -> CliResult {
    let mut cli = Cli::new();

    match Cli::parse_args(args) {
        Ok(command) => cli.execute(command),
        Err(e) => CliResult::error(1, e),
    }
}

/// Main entry point for CLI binary
pub fn main() -> i32 {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = run(&args);

    if !result.message.is_empty() {
        if result.exit_code == 0 {
            println!("{}", result.message);
        } else {
            eprintln!("{}", result.message);
        }
    }

    result.exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init() {
        let cmd = Cli::parse_args(&["init".to_string()]).unwrap();
        assert_eq!(cmd, Command::Init);
    }

    #[test]
    fn test_parse_run() {
        let cmd = Cli::parse_args(&["run".to_string(), "build".to_string()]).unwrap();
        assert_eq!(
            cmd,
            Command::Run {
                task: "build".to_string(),
                filter: None
            }
        );
    }

    #[test]
    fn test_parse_run_with_filter() {
        let cmd = Cli::parse_args(&[
            "run".to_string(),
            "build".to_string(),
            "--filter".to_string(),
            "@myorg/*".to_string(),
        ])
        .unwrap();
        assert_eq!(
            cmd,
            Command::Run {
                task: "build".to_string(),
                filter: Some("@myorg/*".to_string())
            }
        );
    }

    #[test]
    fn test_parse_affected() {
        let cmd = Cli::parse_args(&["affected".to_string()]).unwrap();
        assert_eq!(
            cmd,
            Command::Affected {
                base: None,
                head: None
            }
        );
    }

    #[test]
    fn test_parse_affected_with_refs() {
        let cmd = Cli::parse_args(&[
            "affected".to_string(),
            "--base".to_string(),
            "main".to_string(),
            "--head".to_string(),
            "feature".to_string(),
        ])
        .unwrap();
        assert_eq!(
            cmd,
            Command::Affected {
                base: Some("main".to_string()),
                head: Some("feature".to_string())
            }
        );
    }

    #[test]
    fn test_parse_ghost() {
        let cmd = Cli::parse_args(&["ghost".to_string()]).unwrap();
        assert_eq!(cmd, Command::Ghost);
    }

    #[test]
    fn test_parse_watch() {
        let cmd = Cli::parse_args(&["watch".to_string(), "test".to_string()]).unwrap();
        assert_eq!(
            cmd,
            Command::Watch {
                task: "test".to_string()
            }
        );
    }

    #[test]
    fn test_parse_cache_status() {
        let cmd = Cli::parse_args(&["cache".to_string(), "status".to_string()]).unwrap();
        assert_eq!(
            cmd,
            Command::Cache {
                subcommand: CacheSubcommand::Status
            }
        );
    }

    #[test]
    fn test_parse_cache_clear() {
        let cmd = Cli::parse_args(&["cache".to_string(), "clear".to_string()]).unwrap();
        assert_eq!(
            cmd,
            Command::Cache {
                subcommand: CacheSubcommand::Clear
            }
        );
    }

    #[test]
    fn test_parse_help() {
        let cmd = Cli::parse_args(&["help".to_string()]).unwrap();
        assert_eq!(cmd, Command::Help);

        let cmd = Cli::parse_args(&["--help".to_string()]).unwrap();
        assert_eq!(cmd, Command::Help);
    }

    #[test]
    fn test_parse_version() {
        let cmd = Cli::parse_args(&["version".to_string()]).unwrap();
        assert_eq!(cmd, Command::Version);
    }

    #[test]
    fn test_parse_empty_args() {
        let cmd = Cli::parse_args(&[]).unwrap();
        assert_eq!(cmd, Command::Help);
    }

    #[test]
    fn test_parse_unknown_command() {
        let result = Cli::parse_args(&["unknown".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_run_missing_task() {
        let result = Cli::parse_args(&["run".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_matches_pattern() {
        assert!(Cli::matches_pattern("@myorg/pkg-a", "@myorg/*"));
        assert!(Cli::matches_pattern("@myorg/pkg-b", "@myorg/*"));
        assert!(!Cli::matches_pattern("@other/pkg", "@myorg/*"));

        assert!(Cli::matches_pattern("anything", "*"));

        assert!(Cli::matches_pattern("pkg-utils", "*-utils"));
        assert!(!Cli::matches_pattern("pkg-core", "*-utils"));

        assert!(Cli::matches_pattern("exact-match", "exact-match"));
        assert!(!Cli::matches_pattern("not-match", "exact-match"));
    }

    #[test]
    fn test_cli_help() {
        let cli = Cli::new();
        let result = cli.cmd_help();
        assert_eq!(result.exit_code, 0);
        assert!(result.message.contains("dx-project-manager"));
        assert!(result.message.contains("COMMANDS"));
    }

    #[test]
    fn test_cli_version() {
        let cli = Cli::new();
        let result = cli.cmd_version();
        assert_eq!(result.exit_code, 0);
        assert!(result.message.contains("dx-project-manager"));
    }

    #[test]
    fn test_cli_cache_status() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut cli = Cli::with_cwd(temp.path().to_path_buf());

        let result = cli.cmd_cache(CacheSubcommand::Status);
        assert_eq!(result.exit_code, 0);
        assert!(result.message.contains("Cache size"));
    }

    #[test]
    fn test_cli_cache_clear() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut cli = Cli::with_cwd(temp.path().to_path_buf());

        let result = cli.cmd_cache(CacheSubcommand::Clear);
        assert_eq!(result.exit_code, 0);
        assert!(result.message.contains("cleared"));
    }

    #[test]
    fn test_cli_result() {
        let success = CliResult::success("ok");
        assert_eq!(success.exit_code, 0);
        assert_eq!(success.message, "ok");

        let error = CliResult::error(1, "failed");
        assert_eq!(error.exit_code, 1);
        assert_eq!(error.message, "failed");
    }
}
