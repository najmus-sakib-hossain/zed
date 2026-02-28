//! Doctor command for DX CLI diagnostics
//!
//! Provides comprehensive system diagnostics and health checks.
//! Feature: cli-production-ready, Tasks 7.1-7.6
//! Validates: Requirements 11.1-11.5

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Args;
use owo_colors::OwoColorize;

use crate::ui::theme::Theme;
use crate::utils::update::CURRENT_VERSION;

/// Doctor command arguments
#[derive(Args, Debug, Default)]
pub struct DoctorArgs {
    /// Only show failures and warnings
    #[arg(long)]
    pub quiet: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Run extended diagnostics (slower)
    #[arg(long)]
    pub extended: bool,
}

/// Output format for doctor command
#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Diagnostic check result
#[derive(Debug, Clone)]
pub enum CheckResult {
    Pass(String),
    Warn(String),
    Fail(String),
}

impl CheckResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckResult::Pass(_))
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, CheckResult::Fail(_))
    }
}

/// Diagnostic check
struct DiagnosticCheck {
    name: &'static str,
    category: &'static str,
    result: CheckResult,
}

/// Run the doctor command
pub async fn run(args: DoctorArgs, _theme: &Theme) -> Result<()> {
    let checks = run_all_checks(&args).await;

    match args.format {
        OutputFormat::Text => print_text_report(&checks, &args),
        OutputFormat::Json => print_json_report(&checks),
    }

    // Exit with error code if any checks failed
    let failures = checks.iter().filter(|c| c.result.is_fail()).count();
    if failures > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_all_checks(args: &DoctorArgs) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // ═══════════════════════════════════════════════════════════════════
    // SYSTEM INFORMATION
    // ═══════════════════════════════════════════════════════════════════

    // CLI Version
    checks.push(DiagnosticCheck {
        name: "CLI Version",
        category: "System",
        result: CheckResult::Pass(format!("v{}", CURRENT_VERSION)),
    });

    // Operating System
    checks.push(DiagnosticCheck {
        name: "Operating System",
        category: "System",
        result: CheckResult::Pass(get_os_info()),
    });

    // Architecture
    checks.push(DiagnosticCheck {
        name: "Architecture",
        category: "System",
        result: CheckResult::Pass(get_arch_info()),
    });

    // ═══════════════════════════════════════════════════════════════════
    // DAEMON STATUS
    // ═══════════════════════════════════════════════════════════════════

    // Agent daemon
    checks.push(DiagnosticCheck {
        name: "Agent Daemon",
        category: "Daemon",
        result: check_agent_daemon().await,
    });

    // Project daemon
    checks.push(DiagnosticCheck {
        name: "Project Daemon",
        category: "Daemon",
        result: check_project_daemon().await,
    });

    // ═══════════════════════════════════════════════════════════════════
    // CONFIGURATION
    // ═══════════════════════════════════════════════════════════════════

    // Config file
    checks.push(DiagnosticCheck {
        name: "Config File",
        category: "Configuration",
        result: check_config_file(),
    });

    // Cache directory
    checks.push(DiagnosticCheck {
        name: "Cache Directory",
        category: "Configuration",
        result: check_cache_directory(),
    });

    // ═══════════════════════════════════════════════════════════════════
    // DIAGNOSTICS
    // ═══════════════════════════════════════════════════════════════════

    // Cache writable
    checks.push(DiagnosticCheck {
        name: "Cache Writable",
        category: "Diagnostics",
        result: check_cache_writable(),
    });

    // Network connectivity (if extended)
    if args.extended {
        checks.push(DiagnosticCheck {
            name: "Network (GitHub)",
            category: "Diagnostics",
            result: check_network_github().await,
        });

        checks.push(DiagnosticCheck {
            name: "Network (DX API)",
            category: "Diagnostics",
            result: check_network_dx_api().await,
        });
    }

    checks
}

fn print_text_report(checks: &[DiagnosticCheck], args: &DoctorArgs) {
    println!("\n{}", "DX CLI Doctor".cyan().bold());
    println!("{}\n", "─".repeat(50).dimmed());

    let mut current_category = "";

    for check in checks {
        // Skip passes in quiet mode
        if args.quiet && check.result.is_pass() {
            continue;
        }

        // Print category header
        if check.category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("{}", format!("▸ {}", check.category).yellow().bold());
            current_category = check.category;
        }

        // Print check result
        let (icon, message) = match &check.result {
            CheckResult::Pass(msg) => ("✓".green().to_string(), msg.clone()),
            CheckResult::Warn(msg) => ("⚠".yellow().to_string(), msg.clone()),
            CheckResult::Fail(msg) => ("✗".red().to_string(), msg.clone()),
        };

        println!("  {} {}: {}", icon, check.name, message.dimmed());
    }

    // Summary
    let passes = checks.iter().filter(|c| c.result.is_pass()).count();
    let warns = checks.iter().filter(|c| matches!(c.result, CheckResult::Warn(_))).count();
    let fails = checks.iter().filter(|c| c.result.is_fail()).count();

    println!("\n{}", "─".repeat(50).dimmed());
    println!(
        "{} {} passed, {} warnings, {} failed",
        "Summary:".bold(),
        passes.to_string().green(),
        warns.to_string().yellow(),
        fails.to_string().red(),
    );

    if fails == 0 && warns == 0 {
        println!("\n{}", "All checks passed! ✓".green().bold());
    } else if fails > 0 {
        println!("\n{}", "Some checks failed. Please review above.".red());
    }
}

fn print_json_report(checks: &[DiagnosticCheck]) {
    use serde_json::json;

    let results: Vec<_> = checks
        .iter()
        .map(|c| {
            let (status, message) = match &c.result {
                CheckResult::Pass(msg) => ("pass", msg.clone()),
                CheckResult::Warn(msg) => ("warn", msg.clone()),
                CheckResult::Fail(msg) => ("fail", msg.clone()),
            };
            json!({
                "name": c.name,
                "category": c.category,
                "status": status,
                "message": message,
            })
        })
        .collect();

    let passes = checks.iter().filter(|c| c.result.is_pass()).count();
    let warns = checks.iter().filter(|c| matches!(c.result, CheckResult::Warn(_))).count();
    let fails = checks.iter().filter(|c| c.result.is_fail()).count();

    let report = json!({
        "success": fails == 0,
        "version": CURRENT_VERSION,
        "summary": {
            "passed": passes,
            "warnings": warns,
            "failed": fails,
        },
        "checks": results,
    });

    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

// ═══════════════════════════════════════════════════════════════════════════
// CHECK IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

fn get_os_info() -> String {
    let os = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unknown"
    };

    format!("{}", os)
}

fn get_arch_info() -> String {
    if cfg!(target_arch = "x86_64") {
        "x86_64 (64-bit)".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "ARM64 (64-bit)".to_string()
    } else if cfg!(target_arch = "x86") {
        "x86 (32-bit)".to_string()
    } else {
        "Unknown".to_string()
    }
}

async fn check_agent_daemon() -> CheckResult {
    // TODO: Implement actual daemon status check via IPC
    // For now, check if socket/pid file exists
    let socket_path = get_daemon_socket_path("agent");

    if socket_path.exists() {
        CheckResult::Pass("Running".to_string())
    } else {
        CheckResult::Warn("Not running (start with `dx daemon agent`)".to_string())
    }
}

async fn check_project_daemon() -> CheckResult {
    // TODO: Implement actual daemon status check
    let socket_path = get_daemon_socket_path("project");

    if socket_path.exists() {
        CheckResult::Pass("Running".to_string())
    } else {
        CheckResult::Pass("Not running (on-demand)".to_string())
    }
}

fn check_config_file() -> CheckResult {
    let config_locations = [
        PathBuf::from("dx.toml"),
        PathBuf::from("dx.json"),
        dirs::config_dir().map(|d| d.join("dx").join("config.toml")).unwrap_or_default(),
    ];

    for path in &config_locations {
        if path.exists() {
            return CheckResult::Pass(format!("Found at {}", path.display()));
        }
    }

    CheckResult::Pass("No config file (using defaults)".to_string())
}

fn check_cache_directory() -> CheckResult {
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("dx"))
        .unwrap_or_else(|| PathBuf::from(".dx-cache"));

    if cache_dir.exists() {
        CheckResult::Pass(format!("{}", cache_dir.display()))
    } else {
        CheckResult::Pass(format!("{} (will be created)", cache_dir.display()))
    }
}

fn check_cache_writable() -> CheckResult {
    let cache_dir = dirs::cache_dir()
        .map(|d| d.join("dx"))
        .unwrap_or_else(|| PathBuf::from(".dx-cache"));

    // Try to create directory and write a test file
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        return CheckResult::Fail(format!("Cannot create: {}", e));
    }

    let test_file = cache_dir.join(".doctor_test");
    match std::fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            CheckResult::Pass("Writable".to_string())
        }
        Err(e) => CheckResult::Fail(format!("Not writable: {}", e)),
    }
}

async fn check_network_github() -> CheckResult {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(e) => return CheckResult::Fail(format!("HTTP client error: {}", e)),
    };

    match client.head("https://api.github.com").send().await {
        Ok(resp) if resp.status().is_success() => CheckResult::Pass("Connected".to_string()),
        Ok(resp) => CheckResult::Warn(format!("HTTP {}", resp.status())),
        Err(e) => {
            if e.is_timeout() {
                CheckResult::Warn("Timeout".to_string())
            } else {
                CheckResult::Fail(format!("Error: {}", e))
            }
        }
    }
}

async fn check_network_dx_api() -> CheckResult {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(5)).build() {
        Ok(c) => c,
        Err(e) => return CheckResult::Fail(format!("HTTP client error: {}", e)),
    };

    match client.head("https://api.dx.dev").send().await {
        Ok(resp) if resp.status().is_success() => CheckResult::Pass("Connected".to_string()),
        Ok(resp) => CheckResult::Warn(format!("HTTP {}", resp.status())),
        Err(e) => {
            if e.is_timeout() {
                CheckResult::Warn("Timeout (API may be down)".to_string())
            } else {
                CheckResult::Warn(format!("Unavailable: {}", e))
            }
        }
    }
}

fn get_daemon_socket_path(daemon_type: &str) -> PathBuf {
    let runtime_dir = dirs::runtime_dir()
        .or_else(|| dirs::cache_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    runtime_dir.join("dx").join(format!("{}.sock", daemon_type))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_is_pass() {
        assert!(CheckResult::Pass("ok".to_string()).is_pass());
        assert!(!CheckResult::Warn("warn".to_string()).is_pass());
        assert!(!CheckResult::Fail("fail".to_string()).is_pass());
    }

    #[test]
    fn test_check_result_is_fail() {
        assert!(!CheckResult::Pass("ok".to_string()).is_fail());
        assert!(!CheckResult::Warn("warn".to_string()).is_fail());
        assert!(CheckResult::Fail("fail".to_string()).is_fail());
    }

    #[test]
    fn test_get_os_info() {
        let os = get_os_info();
        assert!(!os.is_empty());
    }

    #[test]
    fn test_get_arch_info() {
        let arch = get_arch_info();
        assert!(!arch.is_empty());
    }
}
