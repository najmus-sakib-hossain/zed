//! VS Code Extension Integration
//!
//! Protocol for communication between DX CLI and VS Code extension

use anyhow::Result;
use std::path::PathBuf;

pub mod protocol;
pub mod server;

/// Extension API message types
#[derive(Debug, Clone)]
pub enum ExtensionMessage {
    // Commands from extension
    RunCheck(RunCheckRequest),
    GetScore(GetScoreRequest),
    WatchProject(WatchProjectRequest),
    StopWatch(StopWatchRequest),

    // Responses to extension
    CheckComplete(CheckCompleteResponse),
    ScoreUpdate(ScoreUpdateResponse),
    DiagnosticsUpdate(DiagnosticsUpdateResponse),
    WatchStarted(WatchStartedResponse),
    WatchStopped(WatchStoppedResponse),
    Error(ErrorResponse),
}

/// Run check request from extension
#[derive(Debug, Clone)]
pub struct RunCheckRequest {
    /// Files to check
    pub files: Vec<PathBuf>,

    /// Check types to run
    pub checks: Vec<String>,

    /// Document version (for invalidation)
    pub version: u32,

    /// Request ID for correlation
    pub request_id: String,
}

/// Get score request
#[derive(Debug, Clone)]
pub struct GetScoreRequest {
    /// Project path
    pub project: PathBuf,

    /// Request ID
    pub request_id: String,
}

/// Watch project request
#[derive(Debug, Clone)]
pub struct WatchProjectRequest {
    /// Project path
    pub project: PathBuf,

    /// File patterns to watch
    pub patterns: Vec<String>,

    /// Debounce interval ms
    pub debounce_ms: u32,
}

/// Stop watch request
#[derive(Debug, Clone)]
pub struct StopWatchRequest {
    /// Project path
    pub project: PathBuf,
}

/// Check complete response
#[derive(Debug, Clone)]
pub struct CheckCompleteResponse {
    /// Request ID
    pub request_id: String,

    /// Success status
    pub success: bool,

    /// Score (0-500)
    pub score: u32,

    /// Diagnostics
    pub diagnostics: Vec<Diagnostic>,

    /// Duration ms
    pub duration_ms: u64,
}

/// Score update response
#[derive(Debug, Clone)]
pub struct ScoreUpdateResponse {
    /// Request ID
    pub request_id: String,

    /// Score (0-500)
    pub score: u32,

    /// Category breakdown
    pub categories: CategoryScores,
}

#[derive(Debug, Clone, Default)]
pub struct CategoryScores {
    pub formatting: u32,
    pub linting: u32,
    pub security: u32,
    pub patterns: u32,
    pub structure: u32,
}

/// Diagnostics update response
#[derive(Debug, Clone)]
pub struct DiagnosticsUpdateResponse {
    /// File URI
    pub uri: String,

    /// Document version
    pub version: u32,

    /// Diagnostics for this file
    pub diagnostics: Vec<Diagnostic>,
}

/// VS Code compatible diagnostic
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Start line (0-indexed)
    pub start_line: u32,

    /// Start character (0-indexed)
    pub start_char: u32,

    /// End line (0-indexed)
    pub end_line: u32,

    /// End character (0-indexed)
    pub end_char: u32,

    /// Severity (1=Error, 2=Warning, 3=Info, 4=Hint)
    pub severity: u8,

    /// Message
    pub message: String,

    /// Source (e.g., "dx-check")
    pub source: String,

    /// Error code
    pub code: Option<String>,

    /// Related information
    pub related: Vec<RelatedInfo>,

    /// Code actions
    pub actions: Vec<CodeAction>,
}

/// Related diagnostic information
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub uri: String,
    pub line: u32,
    pub char: u32,
    pub message: String,
}

/// Code action for quick fixes
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: String, // "quickfix", "refactor", etc.
    pub is_preferred: bool,
    pub edit: Option<WorkspaceEdit>,
}

/// Workspace edit
#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub changes: Vec<TextEdit>,
}

/// Text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub uri: String,
    pub start_line: u32,
    pub start_char: u32,
    pub end_line: u32,
    pub end_char: u32,
    pub new_text: String,
}

/// Watch started response
#[derive(Debug, Clone)]
pub struct WatchStartedResponse {
    pub project: PathBuf,
    pub patterns: Vec<String>,
}

/// Watch stopped response
#[derive(Debug, Clone)]
pub struct WatchStoppedResponse {
    pub project: PathBuf,
}

/// Error response
#[derive(Debug, Clone)]
pub struct ErrorResponse {
    pub request_id: Option<String>,
    pub code: u32,
    pub message: String,
}

/// Extension server for handling requests
pub struct ExtensionServer {
    socket_path: PathBuf,
}

impl ExtensionServer {
    /// Create new server
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Start server
    pub async fn start(&self) -> Result<()> {
        // TODO: Implement server
        Ok(())
    }

    /// Handle incoming message
    pub async fn handle(&self, msg: ExtensionMessage) -> Result<ExtensionMessage> {
        match msg {
            ExtensionMessage::RunCheck(req) => {
                let result = self.run_check(req).await?;
                Ok(ExtensionMessage::CheckComplete(result))
            }
            ExtensionMessage::GetScore(req) => {
                let result = self.get_score(req).await?;
                Ok(ExtensionMessage::ScoreUpdate(result))
            }
            ExtensionMessage::WatchProject(req) => {
                let result = self.watch_project(req).await?;
                Ok(ExtensionMessage::WatchStarted(result))
            }
            ExtensionMessage::StopWatch(req) => {
                let result = self.stop_watch(req).await?;
                Ok(ExtensionMessage::WatchStopped(result))
            }
            _ => Err(anyhow::anyhow!("Invalid message type")),
        }
    }

    async fn run_check(&self, req: RunCheckRequest) -> Result<CheckCompleteResponse> {
        use std::time::Instant;

        let start = Instant::now();

        // TODO: Run actual checks
        let diagnostics = vec![];
        let score = 500;

        Ok(CheckCompleteResponse {
            request_id: req.request_id,
            success: true,
            score,
            diagnostics,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn get_score(&self, req: GetScoreRequest) -> Result<ScoreUpdateResponse> {
        // TODO: Get actual score
        Ok(ScoreUpdateResponse {
            request_id: req.request_id,
            score: 500,
            categories: CategoryScores {
                formatting: 100,
                linting: 100,
                security: 100,
                patterns: 100,
                structure: 100,
            },
        })
    }

    async fn watch_project(&self, req: WatchProjectRequest) -> Result<WatchStartedResponse> {
        // TODO: Start watching
        Ok(WatchStartedResponse {
            project: req.project,
            patterns: req.patterns,
        })
    }

    async fn stop_watch(&self, req: StopWatchRequest) -> Result<WatchStoppedResponse> {
        // TODO: Stop watching
        Ok(WatchStoppedResponse {
            project: req.project,
        })
    }
}

/// Convert internal diagnostics to VS Code format
pub fn to_vscode_diagnostic(
    _file: &PathBuf,
    line: u32,
    column: u32,
    severity: u8,
    message: &str,
    code: Option<&str>,
) -> Diagnostic {
    Diagnostic {
        start_line: line.saturating_sub(1), // Convert to 0-indexed
        start_char: column.saturating_sub(1),
        end_line: line.saturating_sub(1),
        end_char: column.saturating_sub(1) + 1,
        severity,
        message: message.to_string(),
        source: "dx-check".to_string(),
        code: code.map(String::from),
        related: vec![],
        actions: vec![],
    }
}
