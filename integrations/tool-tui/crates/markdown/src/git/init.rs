//! Repository initialization for DXM Git filters.
//!
//! Configures Git filters, .gitattributes, and optional pre-commit hooks
//! for seamless DXM workflows.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// Errors that can occur during repository initialization.
#[derive(Debug, Error)]
pub enum InitError {
    /// Not in a git repository.
    #[error("Not in a git repository")]
    NotGitRepo,

    /// Failed to configure git.
    #[error("Failed to configure git: {0}")]
    GitConfigError(String),

    /// Failed to write .gitattributes.
    #[error("Failed to update .gitattributes: {0}")]
    GitattributesError(String),

    /// Failed to create .dx directory.
    #[error("Failed to create .dx directory: {0}")]
    DxDirError(String),

    /// Failed to install hooks.
    #[error("Failed to install hooks: {0}")]
    HookError(String),

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result of repository initialization.
#[derive(Debug, Clone, Default)]
pub struct InitResult {
    /// Whether git filters were configured.
    pub filters_configured: bool,
    /// Whether .gitattributes was updated.
    pub gitattributes_updated: bool,
    /// Whether .dx/config was created.
    pub dx_config_created: bool,
    /// Whether pre-commit hooks were installed.
    pub hooks_installed: bool,
}

/// Initialize DXM support in a git repository.
pub struct RepoInitializer {
    /// Repository root path.
    pub repo_root: PathBuf,
    /// Whether to configure globally.
    pub global: bool,
    /// Whether to install hooks.
    pub install_hooks: bool,
}

impl RepoInitializer {
    /// Create a new repository initializer.
    ///
    /// # Arguments
    ///
    /// * `repo_root` - Path to the repository root
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
            global: false,
            install_hooks: false,
        }
    }

    /// Configure at user level (global).
    pub fn with_global(mut self, global: bool) -> Self {
        self.global = global;
        self
    }

    /// Install pre-commit hooks.
    pub fn with_hooks(mut self, install_hooks: bool) -> Self {
        self.install_hooks = install_hooks;
        self
    }

    /// Initialize DXM support.
    ///
    /// # Returns
    ///
    /// Result indicating what was configured.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn init(&self) -> Result<InitResult, InitError> {
        let mut result = InitResult::default();

        // Verify we're in a git repository (unless global)
        if !self.global && !self.is_git_repo() {
            return Err(InitError::NotGitRepo);
        }

        // Configure git filters
        self.configure_filters()?;
        result.filters_configured = true;

        // Update .gitattributes (only for local config)
        if !self.global {
            self.update_gitattributes()?;
            result.gitattributes_updated = true;

            // Create .dx/config
            self.create_dx_config()?;
            result.dx_config_created = true;

            // Install hooks if requested
            if self.install_hooks {
                self.install_pre_commit_hook()?;
                result.hooks_installed = true;
            }
        }

        Ok(result)
    }

    /// Check if we're in a git repository.
    fn is_git_repo(&self) -> bool {
        self.repo_root.join(".git").exists()
    }

    /// Configure git filters.
    fn configure_filters(&self) -> Result<(), InitError> {
        let scope = if self.global { "--global" } else { "--local" };

        // Configure clean filter
        self.run_git_config(scope, "filter.dxm.clean", "dx dxm clean")?;

        // Configure smudge filter
        self.run_git_config(scope, "filter.dxm.smudge", "dx dxm smudge")?;

        // Mark filter as required (fail if dx not installed)
        self.run_git_config(scope, "filter.dxm.required", "true")?;

        // Configure diff textconv for semantic diffs
        self.run_git_config(scope, "diff.dxm.textconv", "dx dxm text-view")?;

        // Enable textconv caching
        self.run_git_config(scope, "diff.dxm.cachetextconv", "true")?;

        Ok(())
    }

    /// Run a git config command.
    fn run_git_config(&self, scope: &str, key: &str, value: &str) -> Result<(), InitError> {
        let output = Command::new("git")
            .args(["config", scope, key, value])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| InitError::GitConfigError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InitError::GitConfigError(format!("Failed to set {}: {}", key, stderr)));
        }

        Ok(())
    }

    /// Update .gitattributes with DXM rules.
    fn update_gitattributes(&self) -> Result<(), InitError> {
        let gitattributes_path = self.repo_root.join(".gitattributes");
        let dxm_rule = "*.dxm filter=dxm diff=dxm text";

        // Read existing content
        let existing = if gitattributes_path.exists() {
            fs::read_to_string(&gitattributes_path)
                .map_err(|e| InitError::GitattributesError(e.to_string()))?
        } else {
            String::new()
        };

        // Check if rule already exists
        if existing.lines().any(|line| line.trim() == dxm_rule) {
            return Ok(());
        }

        // Append rule
        let new_content = if existing.is_empty() {
            format!("# DXM files use clean/smudge filters\n{}\n", dxm_rule)
        } else if existing.ends_with('\n') {
            format!("{}\n# DXM files use clean/smudge filters\n{}\n", existing, dxm_rule)
        } else {
            format!("{}\n\n# DXM files use clean/smudge filters\n{}\n", existing, dxm_rule)
        };

        fs::write(&gitattributes_path, new_content)
            .map_err(|e| InitError::GitattributesError(e.to_string()))?;

        Ok(())
    }

    /// Create .dx/config file.
    fn create_dx_config(&self) -> Result<(), InitError> {
        let dx_dir = self.repo_root.join(".dx");
        let config_path = dx_dir.join("config");

        // Create .dx directory if it doesn't exist
        if !dx_dir.exists() {
            fs::create_dir_all(&dx_dir).map_err(|e| InitError::DxDirError(e.to_string()))?;
        }

        // Don't overwrite existing config
        if config_path.exists() {
            return Ok(());
        }

        let config_content = r#"[dxm]
enabled = true
auto_refs = true
phantom_mode = true

[cache]
enabled = true
directory = ".dx/cache"
"#;

        fs::write(&config_path, config_content)
            .map_err(|e| InitError::DxDirError(e.to_string()))?;

        Ok(())
    }

    /// Install pre-commit hook.
    fn install_pre_commit_hook(&self) -> Result<(), InitError> {
        let hooks_dir = self.repo_root.join(".git").join("hooks");
        let hook_path = hooks_dir.join("pre-commit");

        // Create hooks directory if it doesn't exist
        if !hooks_dir.exists() {
            fs::create_dir_all(&hooks_dir).map_err(|e| InitError::HookError(e.to_string()))?;
        }

        let hook_content = r#"#!/bin/bash
# DXM Pre-Commit Hook: Sync shadow files

# Check if dx is installed
if ! command -v dx &> /dev/null; then
    # DX not installed, skip sync
    exit 0
fi

# Sync all staged .dxm files
for dxm in $(git diff --cached --name-only | grep '\.dxm$'); do
    md="${dxm%.dxm}.md"
    dx dxm clean --input "$dxm" > "$md"
    git add "$md"
done

exit 0
"#;

        // Check if hook already exists
        if hook_path.exists() {
            let existing =
                fs::read_to_string(&hook_path).map_err(|e| InitError::HookError(e.to_string()))?;

            // If it's our hook or contains our marker, update it
            if existing.contains("DXM Pre-Commit Hook") {
                fs::write(&hook_path, hook_content)
                    .map_err(|e| InitError::HookError(e.to_string()))?;
            } else {
                // Append to existing hook
                let new_content = format!("{}\n\n{}", existing, hook_content);
                fs::write(&hook_path, new_content)
                    .map_err(|e| InitError::HookError(e.to_string()))?;
            }
        } else {
            fs::write(&hook_path, hook_content).map_err(|e| InitError::HookError(e.to_string()))?;
        }

        // Make hook executable (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path)
                .map_err(|e| InitError::HookError(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)
                .map_err(|e| InitError::HookError(e.to_string()))?;
        }

        Ok(())
    }
}

/// Get the repository root from the current directory.
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_result_default() {
        let result = InitResult::default();
        assert!(!result.filters_configured);
        assert!(!result.gitattributes_updated);
        assert!(!result.dx_config_created);
        assert!(!result.hooks_installed);
    }

    #[test]
    fn test_repo_initializer_new() {
        let init = RepoInitializer::new("/tmp/test");
        assert_eq!(init.repo_root, PathBuf::from("/tmp/test"));
        assert!(!init.global);
        assert!(!init.install_hooks);
    }

    #[test]
    fn test_repo_initializer_with_global() {
        let init = RepoInitializer::new("/tmp/test").with_global(true);
        assert!(init.global);
    }

    #[test]
    fn test_repo_initializer_with_hooks() {
        let init = RepoInitializer::new("/tmp/test").with_hooks(true);
        assert!(init.install_hooks);
    }

    #[test]
    fn test_find_repo_root_not_found() {
        // Use a path that's unlikely to be in a git repo
        let result = find_repo_root(Path::new("/"));
        // This might find a repo or not depending on the system
        // Just verify it doesn't panic
        let _ = result;
    }
}
