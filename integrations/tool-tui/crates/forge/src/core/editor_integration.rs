//! Editor integration for detecting active editor and output directory
//!
//! Provides functionality to detect VSCode extension presence and determine
//! where to output generated files.

use std::path::PathBuf;

/// Type of code editor
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorType {
    /// Visual Studio Code
    VSCode,

    /// Other or unknown editor
    Other(String),
}

/// Information about the active editor
#[derive(Debug, Clone)]
pub struct EditorInfo {
    /// Type of editor
    pub editor_type: EditorType,

    /// Workspace/project path
    pub workspace_path: PathBuf,

    /// Installed extensions
    pub extensions: Vec<String>,
}

/// Strategy for where to output generated files
#[derive(Debug, Clone)]
pub enum OutputStrategy {
    /// Output to current editor's active directory
    CurrentEditorDir { path: PathBuf },

    /// Output to project root
    ProjectRoot { path: PathBuf },

    /// File watching only, no specific output directory
    FileWatchOnly,
}

/// Manages editor integration
pub struct EditorIntegration {
    active_editor: Option<EditorInfo>,
    output_strategy: OutputStrategy,
    vscode_extension_present: bool,
}

impl EditorIntegration {
    /// Create a new editor integration manager
    pub fn new() -> Self {
        Self {
            active_editor: None,
            output_strategy: OutputStrategy::FileWatchOnly,
            vscode_extension_present: false,
        }
    }

    /// Detect the active editor
    pub fn detect_editor(&mut self) -> Option<EditorType> {
        // Check for VSCode via environment variables
        if std::env::var("VSCODE_PID").is_ok()
            || std::env::var("TERM_PROGRAM").is_ok_and(|t| t == "vscode")
        {
            self.vscode_extension_present = self.check_vscode_extension();
            return Some(EditorType::VSCode);
        }

        None
    }

    /// Check if VSCode Forge extension is installed
    fn check_vscode_extension(&self) -> bool {
        // Check common VSCode extension directories
        if let Some(home) = dirs::home_dir() {
            let extension_dirs = vec![
                home.join(".vscode").join("extensions"),
                home.join(".vscode-server").join("extensions"),
            ];

            for ext_dir in extension_dirs {
                if ext_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(ext_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name();
                            if name.to_string_lossy().contains("forge-lsp") {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Get current editor directory (if available from extension)
    pub fn get_current_editor_dir(&self) -> Option<PathBuf> {
        // This would be populated by WebSocket communication from VSCode extension
        // For now, return None - will be implemented when WebSocket integration is added
        None
    }

    /// Set the active editor
    pub fn set_active_editor(&mut self, editor_info: EditorInfo) {
        self.active_editor = Some(editor_info);
    }

    /// Check if VSCode extension is present
    pub fn has_vscode_extension(&self) -> bool {
        self.vscode_extension_present
    }

    /// Set output strategy
    pub fn set_output_strategy(&mut self, strategy: OutputStrategy) {
        tracing::info!("Output strategy changed to: {:?}", strategy);
        self.output_strategy = strategy;
    }

    /// Get current output strategy
    pub fn output_strategy(&self) -> &OutputStrategy {
        &self.output_strategy
    }

    /// Get output directory based on current strategy
    pub fn get_output_directory(&self) -> Option<PathBuf> {
        match &self.output_strategy {
            OutputStrategy::CurrentEditorDir { path } => Some(path.clone()),
            OutputStrategy::ProjectRoot { path } => Some(path.clone()),
            OutputStrategy::FileWatchOnly => None,
        }
    }

    /// Update current editor directory (called by extension via WebSocket)
    pub fn update_editor_directory(&mut self, path: PathBuf) {
        self.output_strategy = OutputStrategy::CurrentEditorDir { path: path.clone() };
        tracing::debug!("Updated editor directory to: {:?}", path);
    }
}

impl Default for EditorIntegration {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to get home directory (cross-platform)
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var_os("USERPROFILE").map(PathBuf::from)
        }

        #[cfg(not(windows))]
        {
            std::env::var_os("HOME").map(PathBuf::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_integration_creation() {
        let integration = EditorIntegration::new();
        assert!(matches!(integration.output_strategy, OutputStrategy::FileWatchOnly));
    }

    #[test]
    fn test_set_output_strategy() {
        let mut integration = EditorIntegration::new();

        let path = PathBuf::from("/test/path");
        integration.set_output_strategy(OutputStrategy::CurrentEditorDir { path: path.clone() });

        assert_eq!(integration.get_output_directory(), Some(path));
    }

    #[test]
    fn test_update_editor_directory() {
        let mut integration = EditorIntegration::new();

        let path = PathBuf::from("/new/dir");
        integration.update_editor_directory(path.clone());

        match integration.output_strategy() {
            OutputStrategy::CurrentEditorDir { path: p } => {
                assert_eq!(p, &path);
            }
            _ => panic!("Expected CurrentEditorDir strategy"),
        }
    }
}
