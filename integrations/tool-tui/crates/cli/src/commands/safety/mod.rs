//! AI Safety and Compliance Module
//!
//! Implements safety mechanisms for AI-driven auto-updates

use anyhow::Result;
use clap::{Args, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod audit;
pub mod rollback;
pub mod validation;

/// AI Safety CLI arguments
#[derive(Debug, Args)]
pub struct SafetyArgs {
    #[command(subcommand)]
    pub command: SafetyCommands,
}

#[derive(Debug, Subcommand)]
pub enum SafetyCommands {
    /// Validate an AI-proposed change
    Validate(ValidateArgs),

    /// Review pending changes
    Review(ReviewArgs),

    /// Approve or reject a change
    Decide(DecideArgs),

    /// View audit log
    Audit(AuditArgs),

    /// Rollback a change
    Rollback(RollbackArgs),

    /// Configure safety settings
    Config(ConfigArgs),
}

/// Validate arguments
#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Change ID or file path
    #[arg(short, long)]
    pub change: String,

    /// Dry run without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Strict validation mode
    #[arg(long)]
    pub strict: bool,
}

/// Review arguments
#[derive(Debug, Args)]
pub struct ReviewArgs {
    /// Filter by status
    #[arg(short, long)]
    pub status: Option<ChangeStatus>,

    /// Filter by AI model
    #[arg(long)]
    pub model: Option<String>,

    /// Number of changes to show
    #[arg(short, long, default_value = "10")]
    pub limit: usize,
}

/// Decide arguments
#[derive(Debug, Args)]
pub struct DecideArgs {
    /// Change ID
    #[arg(short, long)]
    pub change: String,

    /// Approve the change
    #[arg(long, conflicts_with = "reject")]
    pub approve: bool,

    /// Reject the change
    #[arg(long, conflicts_with = "approve")]
    pub reject: bool,

    /// Reason for decision
    #[arg(short, long)]
    pub reason: Option<String>,
}

/// Audit arguments
#[derive(Debug, Args)]
pub struct AuditArgs {
    /// Start date (YYYY-MM-DD)
    #[arg(long)]
    pub from: Option<String>,

    /// End date (YYYY-MM-DD)
    #[arg(long)]
    pub to: Option<String>,

    /// Filter by action type
    #[arg(long)]
    pub action: Option<AuditAction>,

    /// Output format
    #[arg(long, default_value = "table")]
    pub format: AuditFormat,
}

/// Rollback arguments
#[derive(Debug, Args)]
pub struct RollbackArgs {
    /// Change ID to rollback
    #[arg(short, long)]
    pub change: String,

    /// Force rollback without confirmation
    #[arg(long)]
    pub force: bool,
}

/// Config arguments
#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// Set a config value
    #[arg(long)]
    pub set: Option<String>,

    /// Get a config value
    #[arg(long)]
    pub get: Option<String>,

    /// List all config values
    #[arg(long)]
    pub list: bool,
}

/// Change status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    Pending,
    Approved,
    Rejected,
    Applied,
    RolledBack,
}

impl std::str::FromStr for ChangeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "applied" => Ok(Self::Applied),
            "rolledback" | "rolled-back" => Ok(Self::RolledBack),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

/// Audit action type
#[derive(Debug, Clone, Copy)]
pub enum AuditAction {
    Proposed,
    Validated,
    Approved,
    Rejected,
    Applied,
    RolledBack,
}

impl std::str::FromStr for AuditAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(Self::Proposed),
            "validated" => Ok(Self::Validated),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "applied" => Ok(Self::Applied),
            "rolledback" => Ok(Self::RolledBack),
            _ => Err(format!("Unknown action: {}", s)),
        }
    }
}

/// Audit output format
#[derive(Debug, Clone, Copy, Default)]
pub enum AuditFormat {
    #[default]
    Table,
    Json,
    Csv,
}

impl std::str::FromStr for AuditFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// AI-proposed change
#[derive(Debug, Clone)]
pub struct Change {
    pub id: String,
    pub status: ChangeStatus,
    pub model: String,
    pub timestamp: u64,
    pub files: Vec<FileChange>,
    pub description: String,
    pub risk_score: f32,
    pub validation_result: Option<ValidationResult>,
}

/// File change
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub old_content: Option<String>,
    pub new_content: String,
    pub diff: String,
}

/// File change type
#[derive(Debug, Clone, Copy)]
pub enum FileChangeType {
    Create,
    Modify,
    Delete,
    Rename,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub checks: Vec<ValidationCheck>,
    pub risk_score: f32,
    pub recommendations: Vec<String>,
}

/// Validation check
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub severity: CheckSeverity,
}

/// Check severity
#[derive(Debug, Clone, Copy)]
pub enum CheckSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Safety configuration
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// Require human approval for all changes
    pub require_approval: bool,

    /// Maximum risk score allowed (0.0 - 1.0)
    pub max_risk_score: f32,

    /// Blocked file patterns
    pub blocked_patterns: Vec<String>,

    /// Allowed AI models
    pub allowed_models: Vec<String>,

    /// Retention period for audit logs (days)
    pub audit_retention_days: u32,

    /// Enable automatic rollback on failure
    pub auto_rollback: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_approval: true,
            max_risk_score: 0.7,
            blocked_patterns: vec![
                "*.key".to_string(),
                "*.pem".to_string(),
                ".env*".to_string(),
                "**/secrets/**".to_string(),
            ],
            allowed_models: vec![],
            audit_retention_days: 90,
            auto_rollback: true,
        }
    }
}

/// Run safety command
pub async fn run(args: SafetyArgs, _theme: &dyn crate::theme::Theme) -> Result<()> {
    match args.command {
        SafetyCommands::Validate(validate_args) => validate_change(validate_args).await,
        SafetyCommands::Review(review_args) => review_changes(review_args).await,
        SafetyCommands::Decide(decide_args) => decide_change(decide_args).await,
        SafetyCommands::Audit(audit_args) => show_audit(audit_args).await,
        SafetyCommands::Rollback(rollback_args) => rollback_change(rollback_args).await,
        SafetyCommands::Config(config_args) => configure_safety(config_args).await,
    }
}

async fn validate_change(args: ValidateArgs) -> Result<()> {
    println!("Validating change: {}", args.change);

    let result = validation::validate(&args.change, args.strict)?;

    println!();
    println!("Validation Result: {}", if result.passed { "PASSED" } else { "FAILED" });
    println!("Risk Score: {:.2}", result.risk_score);
    println!();

    for check in &result.checks {
        let icon = if check.passed { "✓" } else { "✗" };
        println!("  {} {}: {}", icon, check.name, check.message);
    }

    if !result.recommendations.is_empty() {
        println!();
        println!("Recommendations:");
        for rec in &result.recommendations {
            println!("  • {}", rec);
        }
    }

    if !result.passed && !args.dry_run {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}

async fn review_changes(args: ReviewArgs) -> Result<()> {
    println!("Pending Changes:");
    println!();

    // TODO: Load actual changes
    let changes: Vec<Change> = vec![];

    if changes.is_empty() {
        println!("  No pending changes");
        return Ok(());
    }

    for change in changes.iter().take(args.limit) {
        let status = match change.status {
            ChangeStatus::Pending => "⏳",
            ChangeStatus::Approved => "✓",
            ChangeStatus::Rejected => "✗",
            ChangeStatus::Applied => "🚀",
            ChangeStatus::RolledBack => "↩",
        };

        println!(
            "  {} {} - {} (risk: {:.2})",
            status, change.id, change.description, change.risk_score
        );
        println!("    Model: {}, Files: {}", change.model, change.files.len());
    }

    Ok(())
}

async fn decide_change(args: DecideArgs) -> Result<()> {
    if args.approve {
        println!("Approving change: {}", args.change);

        // Log audit entry
        audit::log(audit::AuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            action: "approved".to_string(),
            change_id: args.change.clone(),
            user: whoami(),
            reason: args.reason,
            metadata: HashMap::new(),
        })?;

        println!("Change approved");
    } else if args.reject {
        println!("Rejecting change: {}", args.change);

        audit::log(audit::AuditEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            action: "rejected".to_string(),
            change_id: args.change.clone(),
            user: whoami(),
            reason: args.reason,
            metadata: HashMap::new(),
        })?;

        println!("Change rejected");
    } else {
        anyhow::bail!("Must specify --approve or --reject");
    }

    Ok(())
}

async fn show_audit(args: AuditArgs) -> Result<()> {
    let entries = audit::query(args.from.as_deref(), args.to.as_deref(), args.action)?;

    match args.format {
        AuditFormat::Table => {
            println!("┌──────────────────────┬──────────┬──────────────┬──────────────┐");
            println!("│ Timestamp            │ Action   │ Change ID    │ User         │");
            println!("├──────────────────────┼──────────┼──────────────┼──────────────┤");

            for entry in &entries {
                println!(
                    "│ {:20} │ {:8} │ {:12} │ {:12} │",
                    format_timestamp(entry.timestamp),
                    entry.action,
                    truncate(&entry.change_id, 12),
                    truncate(&entry.user, 12)
                );
            }

            println!("└──────────────────────┴──────────┴──────────────┴──────────────┘");
        }
        AuditFormat::Json => {
            println!("[");
            for (i, entry) in entries.iter().enumerate() {
                let comma = if i < entries.len() - 1 { "," } else { "" };
                println!(
                    r#"  {{"timestamp":{},"action":"{}","change_id":"{}","user":"{}"}}{}"#,
                    entry.timestamp, entry.action, entry.change_id, entry.user, comma
                );
            }
            println!("]");
        }
        AuditFormat::Csv => {
            println!("timestamp,action,change_id,user,reason");
            for entry in &entries {
                println!(
                    "{},{},{},{},{}",
                    entry.timestamp,
                    entry.action,
                    entry.change_id,
                    entry.user,
                    entry.reason.as_deref().unwrap_or("")
                );
            }
        }
    }

    Ok(())
}

async fn rollback_change(args: RollbackArgs) -> Result<()> {
    if !args.force {
        println!("Are you sure you want to rollback change {}? (y/N)", args.change);
        // TODO: Read confirmation
    }

    println!("Rolling back change: {}", args.change);

    rollback::rollback(&args.change)?;

    audit::log(audit::AuditEntry {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        action: "rolledback".to_string(),
        change_id: args.change.clone(),
        user: whoami(),
        reason: Some("Manual rollback".to_string()),
        metadata: HashMap::new(),
    })?;

    println!("Change rolled back successfully");

    Ok(())
}

async fn configure_safety(_args: ConfigArgs) -> Result<()> {
    let config = SafetyConfig::default();

    println!("Safety Configuration:");
    println!("  require_approval: {}", config.require_approval);
    println!("  max_risk_score: {}", config.max_risk_score);
    println!("  auto_rollback: {}", config.auto_rollback);
    println!("  audit_retention_days: {}", config.audit_retention_days);
    println!();
    println!("Blocked patterns:");
    for pattern in &config.blocked_patterns {
        println!("  - {}", pattern);
    }

    Ok(())
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn format_timestamp(ts: u64) -> String {
    // Simple timestamp formatting
    let secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let days = ts / 86400;

    format!("{:04}-{:02}:{:02}:{:02}", days, hours, mins, secs)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:width$}", s, width = max)
    } else {
        format!("{}...", &s[..max - 3])
    }
}
