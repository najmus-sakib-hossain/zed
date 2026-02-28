//! Crash Reporter for DX CLI
//!
//! Provides panic handling and crash report generation for the DX CLI.
//! Captures diagnostic information and saves it to a file for debugging.
//!
//! Requirements: 1.5, 10.5

use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::utils::error::DxError;
use crate::utils::resource::ResourceManager;

// ═══════════════════════════════════════════════════════════════════════════
//  CRASH REPORT STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════

/// System information captured during a crash
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    /// Total system memory in bytes
    pub memory_total: u64,
    /// Available system memory in bytes
    pub memory_available: u64,
    /// Number of CPU cores
    pub cpu_count: usize,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            memory_total: 0,
            memory_available: 0,
            cpu_count: num_cpus(),
        }
    }
}

/// Crash report containing all diagnostic information
///
/// Requirement 10.5: Include all required fields for crash reports
#[derive(Debug, Clone, Serialize)]
pub struct CrashReport {
    /// Unique identifier for this crash report
    pub id: String,
    /// Timestamp when the crash occurred
    pub timestamp: DateTime<Utc>,
    /// CLI version
    pub version: String,
    /// Operating system name
    pub os: String,
    /// CPU architecture
    pub arch: String,
    /// Panic message
    pub panic_message: String,
    /// Location where the panic occurred (file:line:column)
    pub panic_location: Option<String>,
    /// Stack backtrace
    pub backtrace: String,
    /// Recent commands executed (if available)
    pub recent_commands: Vec<String>,
    /// System information
    pub system_info: SystemInfo,
}

impl CrashReport {
    /// Create a new crash report with the given panic information
    pub fn new(panic_message: String, panic_location: Option<String>) -> Self {
        Self {
            id: generate_crash_id(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            panic_message,
            panic_location,
            backtrace: capture_backtrace(),
            recent_commands: Vec::new(),
            system_info: SystemInfo::default(),
        }
    }

    /// Add recent commands to the report
    pub fn with_recent_commands(mut self, commands: Vec<String>) -> Self {
        self.recent_commands = commands;
        self
    }

    /// Add system info to the report
    pub fn with_system_info(mut self, info: SystemInfo) -> Self {
        self.system_info = info;
        self
    }

    /// Convert the report to JSON
    pub fn to_json(&self) -> Result<String, DxError> {
        serde_json::to_string_pretty(self).map_err(|e| DxError::Internal {
            message: format!("Failed to serialize crash report: {}", e),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  CRASH REPORTER
// ═══════════════════════════════════════════════════════════════════════════

/// Crash reporter that handles panics and generates crash reports
///
/// Requirement 1.5: Catch panics, log diagnostic information, display user-friendly message
pub struct CrashReporter;

impl CrashReporter {
    /// Install the crash reporter as the panic hook
    ///
    /// Integrates with ResourceManager for cleanup on panic.
    /// Requirement 1.5: Catch panics and log diagnostic information
    pub fn install(resource_manager: Option<Arc<ResourceManager>>) {
        let rm = resource_manager;

        std::panic::set_hook(Box::new(move |panic_info| {
            // Clean up resources first
            if let Some(ref rm) = rm {
                rm.cleanup();
            }

            // Generate and save crash report
            let report = Self::generate_report(panic_info);

            match Self::save_report(&report) {
                Ok(path) => Self::display_crash_message(&path, &report),
                Err(e) => {
                    eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
                    eprintln!("║                      DX CLI CRASHED                          ║");
                    eprintln!("╠══════════════════════════════════════════════════════════════╣");
                    eprintln!("║ An unexpected error occurred.                                ║");
                    eprintln!("║ Failed to save crash report: {}                              ", e);
                    eprintln!("║                                                              ║");
                    eprintln!(
                        "║ Error: {}                                                    ",
                        report.panic_message
                    );
                    eprintln!("╚══════════════════════════════════════════════════════════════╝");
                }
            }
        }));
    }

    /// Generate a crash report from panic information
    ///
    /// Requirement 1.5, 10.5: Capture panic message, location, backtrace, system info
    fn generate_report(panic_info: &PanicHookInfo) -> CrashReport {
        // Extract panic message
        let panic_message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        // Extract panic location
        let panic_location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()));

        CrashReport::new(panic_message, panic_location).with_system_info(SystemInfo::default())
    }

    /// Save the crash report to disk
    ///
    /// Saves to ~/.dx/crash-reports/ with timestamp in filename.
    /// Requirement 1.5: Log diagnostic information to a file
    fn save_report(report: &CrashReport) -> Result<PathBuf, DxError> {
        // Get crash reports directory
        let crash_dir = get_crash_reports_dir()?;

        // Create directory if it doesn't exist
        fs::create_dir_all(&crash_dir).map_err(|e| DxError::Io {
            message: format!("Failed to create crash reports directory: {}", e),
        })?;

        // Generate filename with timestamp
        let filename = format!("crash_{}.json", report.timestamp.format("%Y%m%d_%H%M%S"));
        let path = crash_dir.join(filename);

        // Write report
        let json = report.to_json()?;
        let mut file = fs::File::create(&path).map_err(|e| DxError::Io {
            message: format!("Failed to create crash report file: {}", e),
        })?;

        file.write_all(json.as_bytes()).map_err(|e| DxError::Io {
            message: format!("Failed to write crash report: {}", e),
        })?;

        Ok(path)
    }

    /// Display a user-friendly crash message
    ///
    /// Requirement 1.5: Display user-friendly crash report with instructions
    fn display_crash_message(report_path: &Path, report: &CrashReport) {
        eprintln!();
        eprintln!("╔══════════════════════════════════════════════════════════════╗");
        eprintln!("║                      DX CLI CRASHED                          ║");
        eprintln!("╠══════════════════════════════════════════════════════════════╣");
        eprintln!("║ An unexpected error occurred. We're sorry for the            ║");
        eprintln!("║ inconvenience.                                               ║");
        eprintln!("║                                                              ║");
        eprintln!("║ Error: {:<52} ║", truncate_string(&report.panic_message, 52));
        if let Some(ref loc) = report.panic_location {
            eprintln!("║ Location: {:<49} ║", truncate_string(loc, 49));
        }
        eprintln!("║                                                              ║");
        eprintln!("║ A crash report has been saved to:                            ║");
        eprintln!("║ {:<60} ║", truncate_string(&report_path.display().to_string(), 60));
        eprintln!("║                                                              ║");
        eprintln!("║ Please report this issue at:                                 ║");
        eprintln!("║ https://github.com/example/dx/issues                         ║");
        eprintln!("║                                                              ║");
        eprintln!("║ Include the crash report file when reporting.                ║");
        eprintln!("╚══════════════════════════════════════════════════════════════╝");
        eprintln!();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a unique crash ID
fn generate_crash_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);

    // Simple hash-like ID from timestamp and random component
    format!("{:016x}", timestamp)
}

/// Capture the current backtrace
fn capture_backtrace() -> String {
    let bt = Backtrace::capture();
    format!("{}", bt)
}

/// Get the number of CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1)
}

/// Get the crash reports directory
fn get_crash_reports_dir() -> Result<PathBuf, DxError> {
    let home = home::home_dir().ok_or_else(|| DxError::Io {
        message: "Could not determine home directory".to_string(),
    })?;

    Ok(home.join(".dx").join("crash-reports"))
}

/// Truncate a string to fit within a given width
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_report_creation() {
        let report =
            CrashReport::new("test panic".to_string(), Some("src/main.rs:10:5".to_string()));

        assert!(!report.id.is_empty());
        assert_eq!(report.panic_message, "test panic");
        assert_eq!(report.panic_location, Some("src/main.rs:10:5".to_string()));
        assert_eq!(report.os, std::env::consts::OS);
        assert_eq!(report.arch, std::env::consts::ARCH);
    }

    #[test]
    fn test_crash_report_with_commands() {
        let report = CrashReport::new("test".to_string(), None)
            .with_recent_commands(vec!["dx build".to_string(), "dx run".to_string()]);

        assert_eq!(report.recent_commands.len(), 2);
        assert_eq!(report.recent_commands[0], "dx build");
    }

    #[test]
    fn test_crash_report_to_json() {
        let report = CrashReport::new("test panic".to_string(), None);
        let json = report.to_json().expect("Should serialize to JSON");

        assert!(json.contains("test panic"));
        assert!(json.contains("panic_message"));
        assert!(json.contains("timestamp"));
    }

    #[test]
    fn test_system_info_default() {
        let info = SystemInfo::default();
        assert!(info.cpu_count > 0);
    }

    #[test]
    fn test_generate_crash_id() {
        let id1 = generate_crash_id();
        let id2 = generate_crash_id();

        assert!(!id1.is_empty());
        assert_eq!(id1.len(), 16); // 16 hex chars
        // IDs should be different (unless generated at exact same nanosecond)
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    #[test]
    fn test_get_crash_reports_dir() {
        let dir = get_crash_reports_dir();
        assert!(dir.is_ok());

        let path = dir.unwrap();
        assert!(path.ends_with("crash-reports"));
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus > 0);
    }
}
