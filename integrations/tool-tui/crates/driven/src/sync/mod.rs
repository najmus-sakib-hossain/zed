//! Multi-Editor Synchronization
//!
//! Keeps AI rules synchronized across multiple editors.

mod differ;
mod propagator;
mod watcher;

pub use differ::RuleDiffer;
pub use propagator::ChangePropagator;
pub use watcher::FileWatcher;

use crate::{Editor, EditorConfig, Result, RuleSet};
use std::path::Path;

/// Engine for synchronizing rules across editors
#[derive(Debug)]
pub struct SyncEngine {
    /// Source of truth path
    source_of_truth: std::path::PathBuf,
    /// Enabled editors
    editors: EditorConfig,
    /// File watcher
    watcher: Option<FileWatcher>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(source_of_truth: impl AsRef<Path>, editors: EditorConfig) -> Self {
        Self {
            source_of_truth: source_of_truth.as_ref().to_path_buf(),
            editors,
            watcher: None,
        }
    }

    /// Get enabled editors
    pub fn enabled_editors(&self) -> Vec<Editor> {
        let mut editors = Vec::new();

        if self.editors.cursor {
            editors.push(Editor::Cursor);
        }
        if self.editors.copilot {
            editors.push(Editor::Copilot);
        }
        if self.editors.windsurf {
            editors.push(Editor::Windsurf);
        }
        if self.editors.claude {
            editors.push(Editor::Claude);
        }
        if self.editors.aider {
            editors.push(Editor::Aider);
        }
        if self.editors.cline {
            editors.push(Editor::Cline);
        }

        editors
    }

    /// Sync rules to all enabled editors
    pub fn sync(&self, project_root: &Path) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        // Load source of truth
        let source_path = project_root.join(&self.source_of_truth);
        let rules = if source_path.exists() {
            if source_path.extension().is_some_and(|ext| ext == "drv") {
                RuleSet::load_binary(&source_path)?
            } else {
                RuleSet::load(&source_path)?
            }
        } else {
            return Err(crate::DrivenError::Sync(format!(
                "Source of truth not found: {}",
                source_path.display()
            )));
        };

        // Emit to each enabled editor
        for editor in self.enabled_editors() {
            let target_path = project_root.join(editor.rule_path());

            match rules.emit(editor, &target_path) {
                Ok(()) => {
                    report.synced.push(SyncedEditor {
                        editor,
                        path: target_path,
                    });
                }
                Err(e) => {
                    report.errors.push(SyncError {
                        editor,
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(report)
    }

    /// Start watching for changes
    pub fn start_watching(&mut self, project_root: &Path) -> Result<()> {
        let watcher = FileWatcher::new(project_root)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Stop watching for changes
    pub fn stop_watching(&mut self) {
        self.watcher = None;
    }

    /// Check if watching is active
    pub fn is_watching(&self) -> bool {
        self.watcher.is_some()
    }
}

/// Report of sync operation
#[derive(Debug, Default)]
pub struct SyncReport {
    /// Successfully synced editors
    pub synced: Vec<SyncedEditor>,
    /// Errors encountered
    pub errors: Vec<SyncError>,
}

impl SyncReport {
    /// Check if sync was fully successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get count of synced editors
    pub fn synced_count(&self) -> usize {
        self.synced.len()
    }
}

/// Successfully synced editor
#[derive(Debug)]
pub struct SyncedEditor {
    /// The editor
    pub editor: Editor,
    /// Path that was written
    pub path: std::path::PathBuf,
}

/// Sync error for an editor
#[derive(Debug)]
pub struct SyncError {
    /// The editor
    pub editor: Editor,
    /// Error message
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_engine_new() {
        let engine = SyncEngine::new(".driven/rules.drv", EditorConfig::default());
        assert!(!engine.is_watching());
    }

    #[test]
    fn test_enabled_editors() {
        let engine = SyncEngine::new(
            ".driven/rules.drv",
            EditorConfig {
                cursor: true,
                copilot: true,
                windsurf: false,
                claude: false,
                aider: false,
                cline: false,
            },
        );

        let editors = engine.enabled_editors();
        assert_eq!(editors.len(), 2);
        assert!(editors.contains(&Editor::Cursor));
        assert!(editors.contains(&Editor::Copilot));
    }

    #[test]
    fn test_sync_report() {
        let report = SyncReport::default();
        assert!(report.is_success());
        assert_eq!(report.synced_count(), 0);
    }
}
