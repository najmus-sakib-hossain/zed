//! VS Code Extension Integration
//!
//! Provides the ExtensionApi trait and implementation for real-time security feedback
//! in the VS Code editor.
//!
//! ## Features
//! - Status bar security score display
//! - Inline secret highlighting with decoration ranges
//! - Quick-fix code actions for common vulnerabilities
//!
//! _Requirements: 10.4, 10.5, 10.6_

use crate::cli::{ScanResult, SecretFinding, SecurityCommand, SecurityScanner};
use crate::stream::Finding;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// Decoration range for inline highlighting
///
/// Represents a range in a file that should be highlighted in the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecorationRange {
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_column: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_column: u32,
    /// Decoration type/severity
    pub decoration_type: DecorationType,
    /// Hover message
    pub message: String,
}

/// Type of decoration for highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationType {
    /// Critical security issue (red)
    Critical,
    /// High severity issue (orange)
    High,
    /// Medium severity issue (yellow)
    Medium,
    /// Low severity issue (blue)
    Low,
    /// Informational (gray)
    Info,
}

impl DecorationType {
    /// Get CSS color for the decoration type
    pub fn color(&self) -> &'static str {
        match self {
            Self::Critical => "#ff0000",
            Self::High => "#ff8c00",
            Self::Medium => "#ffd700",
            Self::Low => "#1e90ff",
            Self::Info => "#808080",
        }
    }

    /// Get background color with alpha
    pub fn background_color(&self) -> &'static str {
        match self {
            Self::Critical => "rgba(255, 0, 0, 0.2)",
            Self::High => "rgba(255, 140, 0, 0.2)",
            Self::Medium => "rgba(255, 215, 0, 0.2)",
            Self::Low => "rgba(30, 144, 255, 0.2)",
            Self::Info => "rgba(128, 128, 128, 0.1)",
        }
    }
}

/// Quick-fix code action
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Action title displayed to user
    pub title: String,
    /// Kind of action (quickfix, refactor, etc.)
    pub kind: CodeActionKind,
    /// File to modify
    pub file_path: PathBuf,
    /// Text edits to apply
    pub edits: Vec<TextEdit>,
    /// Whether this is the preferred action
    pub is_preferred: bool,
}

/// Kind of code action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeActionKind {
    /// Quick fix for an issue
    QuickFix,
    /// Refactoring action
    Refactor,
    /// Source action (organize imports, etc.)
    Source,
}

/// Text edit for code actions
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_column: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_column: u32,
    /// New text to insert
    pub new_text: String,
}

/// Callback type for finding updates
pub type FindingCallback = Box<dyn Fn(Finding) + Send + Sync>;

/// Extension API trait for VS Code integration
///
/// Provides methods for the VS Code extension to interact with the security scanner.
/// _Requirements: 10.4, 10.5_
pub trait ExtensionApi: Send + Sync {
    /// Get current security score
    ///
    /// Returns the most recent security score (0-100).
    fn get_score(&self) -> u8;

    /// Get findings for a specific file
    ///
    /// Returns all security findings detected in the specified file.
    fn get_file_findings(&self, path: &Path) -> Vec<Finding>;

    /// Subscribe to finding updates
    ///
    /// The callback will be invoked whenever a new finding is detected.
    fn subscribe(&self, callback: FindingCallback);

    /// Get decoration ranges for a file
    ///
    /// Returns ranges that should be highlighted in the editor.
    fn get_decorations(&self, path: &Path) -> Vec<DecorationRange>;

    /// Get code actions for a position
    ///
    /// Returns available quick-fix actions for the given position.
    fn get_code_actions(&self, path: &Path, line: u32, column: u32) -> Vec<CodeAction>;

    /// Trigger a scan for a specific file
    ///
    /// Scans a single file and updates the findings cache.
    fn scan_file(&self, path: &Path) -> Result<(), String>;

    /// Trigger a full workspace scan
    ///
    /// Scans the entire workspace and updates all findings.
    fn scan_workspace(&self, path: &Path) -> Result<ScanResult, String>;
}

/// Extension API implementation
///
/// Thread-safe implementation of the ExtensionApi trait.
pub struct SecurityExtensionApi {
    /// Current security score
    score: RwLock<u8>,
    /// Findings cache by file path
    findings_cache: RwLock<HashMap<PathBuf, Vec<Finding>>>,
    /// Secret findings cache by file path
    secrets_cache: RwLock<HashMap<PathBuf, Vec<SecretFinding>>>,
    /// Subscribers for finding updates
    subscribers: Mutex<Vec<FindingCallback>>,
    /// Scanner instance
    scanner: Mutex<SecurityScanner>,
}

impl SecurityExtensionApi {
    /// Create a new extension API instance
    pub fn new() -> Self {
        Self {
            score: RwLock::new(100),
            findings_cache: RwLock::new(HashMap::new()),
            secrets_cache: RwLock::new(HashMap::new()),
            subscribers: Mutex::new(Vec::new()),
            scanner: Mutex::new(SecurityScanner::new()),
        }
    }

    /// Create a shared instance wrapped in Arc
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Update the security score
    pub fn set_score(&self, score: u8) {
        if let Ok(mut s) = self.score.write() {
            *s = score;
        }
    }

    /// Add a finding to the cache
    pub fn add_finding(&self, finding: Finding) {
        // Add to cache
        if let Ok(mut cache) = self.findings_cache.write() {
            cache
                .entry(finding.file_path.clone())
                .or_insert_with(Vec::new)
                .push(finding.clone());
        }

        // Notify subscribers
        if let Ok(subs) = self.subscribers.lock() {
            for callback in subs.iter() {
                callback(finding.clone());
            }
        }
    }

    /// Add a secret finding to the cache
    pub fn add_secret(&self, path: PathBuf, secret: SecretFinding) {
        if let Ok(mut cache) = self.secrets_cache.write() {
            cache.entry(path).or_insert_with(Vec::new).push(secret);
        }
    }

    /// Clear findings for a file
    pub fn clear_file(&self, path: &Path) {
        if let Ok(mut cache) = self.findings_cache.write() {
            cache.remove(path);
        }
        if let Ok(mut cache) = self.secrets_cache.write() {
            cache.remove(path);
        }
    }

    /// Clear all findings
    pub fn clear_all(&self) {
        if let Ok(mut cache) = self.findings_cache.write() {
            cache.clear();
        }
        if let Ok(mut cache) = self.secrets_cache.write() {
            cache.clear();
        }
    }

    /// Get status bar text
    ///
    /// Returns formatted text for the VS Code status bar.
    pub fn get_status_bar_text(&self) -> String {
        let score = self.get_score();
        let icon = if score >= 80 {
            "$(shield)"
        } else if score >= 50 {
            "$(warning)"
        } else {
            "$(error)"
        };
        format!("{} Security: {}", icon, score)
    }

    /// Get status bar tooltip
    pub fn get_status_bar_tooltip(&self) -> String {
        let score = self.get_score();
        let findings_count = self
            .findings_cache
            .read()
            .map(|c| c.values().map(|v| v.len()).sum::<usize>())
            .unwrap_or(0);

        format!("Security Score: {}/100\nFindings: {}", score, findings_count)
    }
}

impl Default for SecurityExtensionApi {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionApi for SecurityExtensionApi {
    fn get_score(&self) -> u8 {
        self.score.read().map(|s| *s).unwrap_or(0)
    }

    fn get_file_findings(&self, path: &Path) -> Vec<Finding> {
        self.findings_cache
            .read()
            .map(|cache| cache.get(path).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    fn subscribe(&self, callback: FindingCallback) {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(callback);
        }
    }

    fn get_decorations(&self, path: &Path) -> Vec<DecorationRange> {
        let mut decorations = Vec::new();

        // Get secret findings for decorations
        if let Ok(cache) = self.secrets_cache.read() {
            if let Some(secrets) = cache.get(path) {
                for secret in secrets {
                    decorations.push(DecorationRange {
                        start_line: secret.line_number.saturating_sub(1),
                        start_column: secret.column as u32,
                        end_line: secret.line_number.saturating_sub(1),
                        end_column: secret.column as u32 + 20, // Approximate secret length
                        decoration_type: DecorationType::Critical,
                        message: format!(
                            "🔐 {} detected (confidence: {:.0}%)",
                            secret.secret_type,
                            secret.confidence * 100.0
                        ),
                    });
                }
            }
        }

        // Get other findings for decorations
        if let Ok(cache) = self.findings_cache.read() {
            if let Some(findings) = cache.get(path) {
                for finding in findings {
                    let decoration_type = match finding.severity {
                        4 => DecorationType::Critical,
                        3 => DecorationType::High,
                        2 => DecorationType::Medium,
                        1 => DecorationType::Low,
                        _ => DecorationType::Info,
                    };

                    decorations.push(DecorationRange {
                        start_line: finding.line_number.saturating_sub(1),
                        start_column: finding.column as u32,
                        end_line: finding.line_number.saturating_sub(1),
                        end_column: finding.column as u32 + 10,
                        decoration_type,
                        message: finding.message.clone(),
                    });
                }
            }
        }

        decorations
    }

    fn get_code_actions(&self, path: &Path, line: u32, _column: u32) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Check for secrets at this position
        if let Ok(cache) = self.secrets_cache.read() {
            if let Some(secrets) = cache.get(path) {
                for secret in secrets {
                    // Line numbers in cache are 1-indexed, input is 0-indexed
                    if secret.line_number == line + 1 {
                        // Add quick-fix to remove the secret
                        actions.push(CodeAction {
                            title: format!("Remove {} from code", secret.secret_type),
                            kind: CodeActionKind::QuickFix,
                            file_path: path.to_path_buf(),
                            edits: vec![TextEdit {
                                start_line: line,
                                start_column: secret.column as u32,
                                end_line: line,
                                end_column: secret.column as u32 + 40, // Approximate
                                new_text: "\"<REDACTED>\"".to_string(),
                            }],
                            is_preferred: false,
                        });

                        // Add quick-fix to use environment variable
                        actions.push(CodeAction {
                            title: format!(
                                "Replace {} with environment variable",
                                secret.secret_type
                            ),
                            kind: CodeActionKind::QuickFix,
                            file_path: path.to_path_buf(),
                            edits: vec![TextEdit {
                                start_line: line,
                                start_column: secret.column as u32,
                                end_line: line,
                                end_column: secret.column as u32 + 40,
                                new_text: "std::env::var(\"SECRET_KEY\").unwrap()".to_string(),
                            }],
                            is_preferred: true,
                        });
                    }
                }
            }
        }

        actions
    }

    fn scan_file(&self, path: &Path) -> Result<(), String> {
        // Clear existing findings for this file
        self.clear_file(path);

        // Create a command for single file scan
        let cmd = SecurityCommand::new(path.to_path_buf());

        // Scan the file
        let result = self
            .scanner
            .lock()
            .map_err(|e| format!("Failed to acquire scanner lock: {}", e))?
            .scan(&cmd)
            .map_err(|e| format!("Scan failed: {}", e))?;

        // Update score
        self.set_score(result.score);

        // Cache secrets
        for secret in result.secrets {
            self.add_secret(secret.file_path.clone(), secret);
        }

        Ok(())
    }

    fn scan_workspace(&self, path: &Path) -> Result<ScanResult, String> {
        // Clear all findings
        self.clear_all();

        // Create a command for workspace scan
        let cmd = SecurityCommand::new(path.to_path_buf());

        // Scan the workspace
        let result = self
            .scanner
            .lock()
            .map_err(|e| format!("Failed to acquire scanner lock: {}", e))?
            .scan(&cmd)
            .map_err(|e| format!("Scan failed: {}", e))?;

        // Update score
        self.set_score(result.score);

        // Cache secrets
        for secret in &result.secrets {
            self.add_secret(secret.file_path.clone(), secret.clone());
        }

        Ok(result)
    }
}

/// Status bar provider for VS Code
///
/// Provides data for the security score status bar item.
#[derive(Debug, Clone)]
pub struct StatusBarProvider {
    /// Current score
    pub score: u8,
    /// Number of findings
    pub findings_count: usize,
    /// Whether a scan is in progress
    pub scanning: bool,
}

impl StatusBarProvider {
    /// Create from extension API
    pub fn from_api(api: &SecurityExtensionApi) -> Self {
        let findings_count = api
            .findings_cache
            .read()
            .map(|c| c.values().map(|v| v.len()).sum())
            .unwrap_or(0);

        Self {
            score: api.get_score(),
            findings_count,
            scanning: false,
        }
    }

    /// Get the status bar text
    pub fn text(&self) -> String {
        if self.scanning {
            "$(sync~spin) Scanning...".to_string()
        } else {
            let icon = if self.score >= 80 {
                "$(shield)"
            } else if self.score >= 50 {
                "$(warning)"
            } else {
                "$(error)"
            };
            format!("{} {}", icon, self.score)
        }
    }

    /// Get the status bar tooltip
    pub fn tooltip(&self) -> String {
        if self.scanning {
            "Security scan in progress...".to_string()
        } else {
            format!(
                "Security Score: {}/100\n{} issue(s) found\nClick to view details",
                self.score, self.findings_count
            )
        }
    }

    /// Get the status bar color
    pub fn color(&self) -> Option<&'static str> {
        if self.scanning {
            None
        } else if self.score >= 80 {
            Some("#00ff00") // Green
        } else if self.score >= 50 {
            Some("#ffff00") // Yellow
        } else {
            Some("#ff0000") // Red
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::FindingType;

    #[test]
    fn test_extension_api_new() {
        let api = SecurityExtensionApi::new();
        assert_eq!(api.get_score(), 100);
    }

    #[test]
    fn test_set_score() {
        let api = SecurityExtensionApi::new();
        api.set_score(75);
        assert_eq!(api.get_score(), 75);
    }

    #[test]
    fn test_add_finding() {
        let api = SecurityExtensionApi::new();
        let finding = Finding::new(
            FindingType::Secret,
            4,
            PathBuf::from("test.rs"),
            10,
            "Test finding".to_string(),
        );

        api.add_finding(finding);

        let findings = api.get_file_findings(Path::new("test.rs"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].message, "Test finding");
    }

    #[test]
    fn test_clear_file() {
        let api = SecurityExtensionApi::new();
        let finding = Finding::new(
            FindingType::Secret,
            4,
            PathBuf::from("test.rs"),
            10,
            "Test finding".to_string(),
        );

        api.add_finding(finding);
        assert_eq!(api.get_file_findings(Path::new("test.rs")).len(), 1);

        api.clear_file(Path::new("test.rs"));
        assert_eq!(api.get_file_findings(Path::new("test.rs")).len(), 0);
    }

    #[test]
    fn test_get_decorations() {
        let api = SecurityExtensionApi::new();
        let secret = SecretFinding {
            file_path: PathBuf::from("test.rs"),
            line_number: 5,
            column: 10,
            secret_type: "AwsAccessKey".to_string(),
            confidence: 0.95,
        };

        api.add_secret(PathBuf::from("test.rs"), secret);

        let decorations = api.get_decorations(Path::new("test.rs"));
        assert_eq!(decorations.len(), 1);
        assert_eq!(decorations[0].start_line, 4); // 0-indexed
        assert_eq!(decorations[0].decoration_type, DecorationType::Critical);
    }

    #[test]
    fn test_get_code_actions() {
        let api = SecurityExtensionApi::new();
        let secret = SecretFinding {
            file_path: PathBuf::from("test.rs"),
            line_number: 5,
            column: 10,
            secret_type: "AwsAccessKey".to_string(),
            confidence: 0.95,
        };

        api.add_secret(PathBuf::from("test.rs"), secret);

        // Line 4 (0-indexed) = line 5 (1-indexed)
        let actions = api.get_code_actions(Path::new("test.rs"), 4, 10);
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|a| a.title.contains("Remove")));
        assert!(actions.iter().any(|a| a.title.contains("environment variable")));
    }

    #[test]
    fn test_status_bar_text() {
        let api = SecurityExtensionApi::new();

        api.set_score(90);
        assert!(api.get_status_bar_text().contains("$(shield)"));

        api.set_score(60);
        assert!(api.get_status_bar_text().contains("$(warning)"));

        api.set_score(30);
        assert!(api.get_status_bar_text().contains("$(error)"));
    }

    #[test]
    fn test_decoration_type_colors() {
        assert_eq!(DecorationType::Critical.color(), "#ff0000");
        assert_eq!(DecorationType::High.color(), "#ff8c00");
        assert_eq!(DecorationType::Medium.color(), "#ffd700");
        assert_eq!(DecorationType::Low.color(), "#1e90ff");
        assert_eq!(DecorationType::Info.color(), "#808080");
    }

    #[test]
    fn test_status_bar_provider() {
        let api = SecurityExtensionApi::new();
        api.set_score(85);

        let provider = StatusBarProvider::from_api(&api);
        assert_eq!(provider.score, 85);
        assert!(provider.text().contains("$(shield)"));
        assert_eq!(provider.color(), Some("#00ff00"));
    }

    #[test]
    fn test_subscribe() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let api = Arc::new(SecurityExtensionApi::new());
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        api.subscribe(Box::new(move |_finding| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        let finding =
            Finding::new(FindingType::Secret, 4, PathBuf::from("test.rs"), 10, "Test".to_string());

        api.add_finding(finding);

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
