//! Exchange Message Definitions
//!
//! All message types for daemon-to-daemon communication

use std::path::PathBuf;

/// Check request message
#[derive(Debug, Clone)]
pub struct CheckRequest {
    /// Files to check
    pub files: Vec<PathBuf>,

    /// Check types to run
    pub checks: Vec<CheckType>,

    /// Output format
    pub format: OutputFormat,

    /// Priority (higher = more urgent)
    pub priority: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum CheckType {
    Format,
    Lint,
    Test,
    Coverage,
    Security,
    All,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Human,
    Llm,
    Machine,
    Json,
}

/// Check result message
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Overall score (0-500)
    pub score: u32,

    /// Category scores
    pub categories: CategoryScores,

    /// Issues found
    pub issues: Vec<Issue>,

    /// Test results
    pub tests: Option<TestResults>,

    /// Coverage data
    pub coverage: Option<CoverageData>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CategoryScores {
    pub formatting: u32,
    pub linting: u32,
    pub security: u32,
    pub patterns: u32,
    pub structure: u32,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Default)]
pub struct TestResults {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub name: String,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct CoverageData {
    pub line_percent: f32,
    pub branch_percent: f32,
    pub function_percent: f32,
    pub files: Vec<FileCoverage>,
}

#[derive(Debug, Clone)]
pub struct FileCoverage {
    pub path: PathBuf,
    pub lines_total: u32,
    pub lines_covered: u32,
    pub branches_total: u32,
    pub branches_covered: u32,
}

/// Sync request message
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Files to sync
    pub files: Vec<SyncFile>,

    /// Sync direction
    pub direction: SyncDirection,

    /// Priority
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub struct SyncFile {
    pub local_path: PathBuf,
    pub remote_key: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncDirection {
    Upload,
    Download,
}

/// Sync progress message
#[derive(Debug, Clone)]
pub struct SyncProgress {
    /// Request ID this progress is for
    pub request_id: String,

    /// Files completed
    pub completed: u32,

    /// Total files
    pub total: u32,

    /// Bytes transferred
    pub bytes_transferred: u64,

    /// Total bytes
    pub bytes_total: u64,

    /// Current file being synced
    pub current_file: Option<String>,
}

/// Sync complete message
#[derive(Debug, Clone)]
pub struct SyncComplete {
    /// Request ID
    pub request_id: String,

    /// Success status
    pub success: bool,

    /// Files synced
    pub files_synced: u32,

    /// Bytes transferred
    pub bytes_transferred: u64,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Errors if any
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Clone)]
pub struct SyncError {
    pub file: String,
    pub message: String,
}

/// AI update request message
#[derive(Debug, Clone)]
pub struct AiUpdateRequest {
    /// Bot ID
    pub bot_id: String,

    /// New version
    pub new_version: String,

    /// Changelog
    pub changelog: String,

    /// Validation results
    pub validation: UpdateValidation,

    /// Requires approval
    pub requires_approval: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateValidation {
    pub score_before: u32,
    pub score_after: u32,
    pub issues_count: u32,
    pub dry_run_passed: bool,
}

/// AI update approval message
#[derive(Debug, Clone)]
pub struct AiUpdateApproval {
    /// Request ID to approve
    pub request_id: String,

    /// Approved or rejected
    pub approved: bool,

    /// Approver identifier
    pub approver: String,

    /// Reason if rejected
    pub reason: Option<String>,
}

/// AI update result message
#[derive(Debug, Clone)]
pub struct AiUpdateResult {
    /// Request ID
    pub request_id: String,

    /// Success status
    pub success: bool,

    /// New version applied
    pub version: String,

    /// Score after update
    pub score: u32,

    /// Rollback available
    pub rollback_available: bool,

    /// Error message if failed
    pub error: Option<String>,
}

/// Heartbeat message
#[derive(Debug, Clone)]
pub struct Heartbeat {
    /// Sequence number
    pub sequence: u64,

    /// Load information
    pub load: LoadInfo,
}

#[derive(Debug, Clone, Default)]
pub struct LoadInfo {
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub active_tasks: u32,
    pub queue_length: u32,
}

/// Status request message
#[derive(Debug, Clone)]
pub struct StatusRequest {
    /// Include metrics
    pub include_metrics: bool,

    /// Include queue info
    pub include_queue: bool,

    /// Include connection info
    pub include_connections: bool,
}

/// Status response message
#[derive(Debug, Clone)]
pub struct StatusResponse {
    /// Daemon type
    pub daemon_type: String,

    /// Running status
    pub running: bool,

    /// Uptime in seconds
    pub uptime_secs: u64,

    /// Metrics if requested
    pub metrics: Option<DaemonMetrics>,

    /// Queue info if requested
    pub queue: Option<QueueInfo>,

    /// Connection info if requested
    pub connections: Option<Vec<ConnectionInfo>>,
}

#[derive(Debug, Clone, Default)]
pub struct DaemonMetrics {
    pub checks_run: u64,
    pub files_synced: u64,
    pub bytes_transferred: u64,
    pub errors: u64,
}

#[derive(Debug, Clone)]
pub struct QueueInfo {
    pub pending: u32,
    pub processing: u32,
    pub completed: u32,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub daemon_id: String,
    pub connected_at: u64,
    pub last_heartbeat: u64,
}

/// Error message
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    /// Error code
    pub code: u32,

    /// Error message
    pub message: String,

    /// Correlation ID (what request failed)
    pub correlation_id: Option<String>,

    /// Recoverable
    pub recoverable: bool,

    /// Retry after seconds (if recoverable)
    pub retry_after: Option<u32>,
}
