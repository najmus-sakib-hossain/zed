//! Core Lint Engine
//!
//! Binary Rule Fusion Engine - executes all rules in a single AST traversal.

use crate::adaptive::{AdaptiveStrategy, WorkloadDetector};
use crate::cache::AstCache;
use crate::config::CheckerConfig;
use crate::diagnostics::{Diagnostic, Span};
use crate::framework_config::FrameworkConfigManager;
use crate::project::ProjectProfile;
use crate::rules::{RuleContext, RuleRegistry};
use ignore::WalkBuilder;
use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::any::Any;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Extract a human-readable message from a panic payload
fn extract_panic_message(panic_info: &Box<dyn Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

/// Result of a check operation
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// All diagnostics found
    pub diagnostics: Vec<Diagnostic>,
    /// Number of files checked
    pub files_checked: usize,
    /// Total time taken
    pub duration: Duration,
    /// Files per second
    pub files_per_second: f64,
}

impl CheckResult {
    /// Check if there are any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostics::DiagnosticSeverity::Error)
    }

    /// Check if there are any warnings
    #[must_use]
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::diagnostics::DiagnosticSeverity::Warning)
    }

    /// Get error count
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostics::DiagnosticSeverity::Error)
            .count()
    }

    /// Get warning count
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostics::DiagnosticSeverity::Warning)
            .count()
    }
}

/// The main checker engine
pub struct Checker {
    /// Configuration
    config: CheckerConfig,
    /// Rule registry
    registry: RuleRegistry,
    /// AST cache (optional)
    cache: Option<AstCache>,
    /// Project profile (auto-detected)
    profile: Option<ProjectProfile>,
    /// Framework configuration manager
    framework_config: FrameworkConfigManager,
    /// Adaptive optimization strategy
    adaptive: AdaptiveStrategy,
    /// Workload detector
    workload: WorkloadDetector,
}

impl Checker {
    /// Create a new checker with default configuration
    #[must_use]
    pub fn new(config: CheckerConfig) -> Self {
        let registry = RuleRegistry::from_config(&config.rules);

        Self {
            config,
            registry,
            cache: None,
            profile: None,
            framework_config: FrameworkConfigManager::new(),
            adaptive: AdaptiveStrategy::new(),
            workload: WorkloadDetector::new(),
        }
    }

    /// Create a checker with project auto-detection
    pub fn with_auto_detect(root: &Path) -> Self {
        let config = CheckerConfig::auto_detect(root);
        let profile = ProjectProfile::detect(root);
        let mut registry = RuleRegistry::from_config(&config.rules);
        let mut framework_config = FrameworkConfigManager::new();

        // Load framework-specific configurations and rules
        if let Some(prof) = Some(&profile) {
            for framework in &prof.frameworks {
                // Load framework configuration
                if let Err(e) = framework_config.load_config(root, *framework) {
                    tracing::warn!(
                        framework = framework.as_str(),
                        error = %e,
                        "Failed to load framework configuration"
                    );
                }

                // Load framework-specific rules
                if let Err(e) = registry.load_framework_rules(*framework, root) {
                    tracing::warn!(
                        framework = framework.as_str(),
                        error = %e,
                        "Failed to load framework rules"
                    );
                }

                // Apply framework configuration to registry
                if let Some(fw_config) = framework_config.get_config(*framework) {
                    registry.enable_framework_rules(*framework, &fw_config.enabled_rules);
                    registry.disable_framework_rules(*framework, &fw_config.disabled_rules);
                }
            }
        }

        Self {
            config,
            registry,
            cache: None,
            profile: Some(profile),
            framework_config,
            adaptive: AdaptiveStrategy::new(),
            workload: WorkloadDetector::new(),
        }
    }

    /// Enable AST caching
    pub fn with_cache(mut self, cache: AstCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Check a single file
    pub fn check_file(&self, path: &Path) -> Result<Vec<Diagnostic>, CheckError> {
        // Use adaptive I/O strategy
        let source = if let Ok(metadata) = std::fs::metadata(path) {
            let file_size = metadata.len() as usize;
            self.workload.record_file(file_size);

            self.adaptive.io.read_file(path).map_err(|e| CheckError::Io {
                path: path.to_path_buf(),
                error: e,
            })?
        } else {
            std::fs::read(path).map_err(|e| CheckError::Io {
                path: path.to_path_buf(),
                error: e,
            })?
        };

        let source = String::from_utf8_lossy(&source);
        self.check_source(path, &source)
    }

    /// Check source code directly
    pub fn check_source(&self, path: &Path, source: &str) -> Result<Vec<Diagnostic>, CheckError> {
        let source_type = SourceType::from_path(path).unwrap_or_default();

        // Parse the source
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, source, source_type);
        let result = parser.parse();

        // Collect parse errors
        let mut diagnostics: Vec<Diagnostic> = result
            .errors
            .iter()
            .map(|e| {
                Diagnostic::error(
                    path.to_path_buf(),
                    crate::diagnostics::Span::new(0, 0),
                    "parse-error",
                    e.to_string(),
                )
            })
            .collect();

        // If there are parse errors, return early
        if !diagnostics.is_empty() {
            return Ok(diagnostics);
        }

        // Create rule context
        let mut ctx = RuleContext::new(path, source);

        // Run file-level checks with panic catching
        for (rule, _severity) in self.registry.enabled_rules() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rule.check_file(source, &mut ctx);
            }));

            if let Err(panic_info) = result {
                let panic_msg = extract_panic_message(&panic_info);
                tracing::error!(
                    rule_id = rule.meta().name,
                    file = %path.display(),
                    "Rule panicked during file check: {}",
                    panic_msg
                );
                ctx.report(Diagnostic::error(
                    path.to_path_buf(),
                    Span::new(0, 0),
                    "rule-panic",
                    format!(
                        "Rule '{}' panicked during file check: {}",
                        rule.meta().name,
                        panic_msg
                    ),
                ));
            }
        }

        // Traverse AST and run rules
        let mut visitor = LintVisitor::new(&self.registry, &mut ctx);
        visitor.visit_program(&result.program);

        // Run end checks with panic catching
        for (rule, _severity) in self.registry.enabled_rules() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rule.check_end(&mut ctx);
            }));

            if let Err(panic_info) = result {
                let panic_msg = extract_panic_message(&panic_info);
                tracing::error!(
                    rule_id = rule.meta().name,
                    file = %path.display(),
                    "Rule panicked during end check: {}",
                    panic_msg
                );
                ctx.report(Diagnostic::error(
                    path.to_path_buf(),
                    Span::new(0, 0),
                    "rule-panic",
                    format!("Rule '{}' panicked during end check: {}", rule.meta().name, panic_msg),
                ));
            }
        }

        // Collect diagnostics
        diagnostics.extend(ctx.take_diagnostics());

        Ok(diagnostics)
    }

    /// Check a directory or file path
    pub fn check_path(&self, path: &Path) -> Result<CheckResult, CheckError> {
        let start = Instant::now();

        // Collect files to check
        let files = self.collect_files(path)?;
        let file_count = files.len();

        // Get workload stats and optimization plan
        let stats = self.workload.get_stats();
        let plan = self.adaptive.analyze(&stats);

        // Check files with adaptive parallelism
        let diagnostics: Vec<Diagnostic> =
            if !plan.use_parallel || self.config.parallel.threads == 1 {
                // Single-threaded for small workloads or debugging
                files.iter().flat_map(|f| self.check_file(f).unwrap_or_default()).collect()
            } else {
                // Parallel execution with adaptive batch size
                use rayon::prelude::*;

                if plan.use_streaming {
                    // Streaming mode for high memory pressure
                    files
                        .par_chunks(plan.batch_size)
                        .flat_map(|chunk| {
                            chunk
                                .iter()
                                .flat_map(|f| self.check_file(f).unwrap_or_default())
                                .collect::<Vec<_>>()
                        })
                        .collect()
                } else {
                    // Normal parallel mode
                    files.par_iter().flat_map(|f| self.check_file(f).unwrap_or_default()).collect()
                }
            };

        let duration = start.elapsed();
        let files_per_second = file_count as f64 / duration.as_secs_f64();

        Ok(CheckResult {
            diagnostics,
            files_checked: file_count,
            duration,
            files_per_second,
        })
    }

    /// Collect files to check based on include/exclude patterns
    fn collect_files(&self, root: &Path) -> Result<Vec<PathBuf>, CheckError> {
        let mut files = Vec::new();

        if root.is_file() {
            files.push(root.to_path_buf());
            return Ok(files);
        }

        let walker = WalkBuilder::new(root)
            .standard_filters(true) // Respect .gitignore
            .hidden(true) // Skip hidden files
            .build();

        for entry in walker.flatten() {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Check if file matches include patterns
            let matches_include = self.config.include.iter().any(|pattern| {
                glob::Pattern::new(pattern).map(|p| p.matches_path(path)).unwrap_or(false)
            });

            if !matches_include {
                // Check by extension as fallback
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let is_js_ts = matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs");
                if !is_js_ts {
                    continue;
                }
            }

            // Check if file matches exclude patterns
            let matches_exclude = self.config.exclude.iter().any(|pattern| {
                glob::Pattern::new(pattern).map(|p| p.matches_path(path)).unwrap_or(false)
            });

            if matches_exclude {
                continue;
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    /// Get the rule registry
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }

    /// Get the project profile
    pub fn profile(&self) -> Option<&ProjectProfile> {
        self.profile.as_ref()
    }

    /// Get the framework configuration manager
    pub fn framework_config(&self) -> &FrameworkConfigManager {
        &self.framework_config
    }

    /// Get mutable framework configuration manager
    pub fn framework_config_mut(&mut self) -> &mut FrameworkConfigManager {
        &mut self.framework_config
    }
}

/// Visitor for traversing AST and running rules
struct LintVisitor<'a, 'ctx> {
    registry: &'a RuleRegistry,
    ctx: &'a mut RuleContext<'ctx>,
}

impl<'a, 'ctx> LintVisitor<'a, 'ctx> {
    fn new(registry: &'a RuleRegistry, ctx: &'a mut RuleContext<'ctx>) -> Self {
        Self { registry, ctx }
    }

    fn check_node(&mut self, kind: &AstKind<'_>) {
        for (rule, _severity) in self.registry.enabled_rules() {
            // Catch panics from rule execution to prevent one bad rule from crashing everything
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rule.check(kind, self.ctx);
            }));

            if let Err(panic_info) = result {
                // Extract panic message
                let panic_msg = extract_panic_message(&panic_info);

                // Log the panic but continue with other rules
                tracing::error!(
                    rule_id = rule.meta().name,
                    file = %self.ctx.file_path.display(),
                    "Rule panicked: {}",
                    panic_msg
                );

                // Add a diagnostic about the rule failure
                self.ctx.report(crate::diagnostics::Diagnostic::error(
                    self.ctx.file_path.to_path_buf(),
                    crate::diagnostics::Span::new(0, 0),
                    "rule-panic",
                    format!("Rule '{}' panicked: {}", rule.meta().name, panic_msg),
                ));
            }
        }
    }
}

impl Visit<'_> for LintVisitor<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'_>) {
        self.check_node(&kind);
    }
}

/// Errors that can occur during checking
#[derive(Debug)]
pub enum CheckError {
    /// IO error reading file
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    /// Parse error
    Parse {
        path: PathBuf,
        message: String,
        line: Option<u32>,
        column: Option<u32>,
    },
    /// Configuration error
    Config { message: String },
    /// Rule execution error
    RuleExecution {
        rule_id: String,
        path: PathBuf,
        message: String,
    },
    /// Plugin error
    Plugin {
        plugin_name: String,
        message: String,
    },
    /// Cache error
    Cache { message: String },
    /// Internal error (should not happen)
    Internal { message: String },
}

impl CheckError {
    /// Create an IO error with file path context
    pub fn io(path: impl Into<PathBuf>, error: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            error,
        }
    }

    /// Create a parse error with location information
    pub fn parse(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Parse {
            path: path.into(),
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Create a parse error with line and column
    pub fn parse_at(
        path: impl Into<PathBuf>,
        message: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Self::Parse {
            path: path.into(),
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a rule execution error
    pub fn rule_execution(
        rule_id: impl Into<String>,
        path: impl Into<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self::RuleExecution {
            rule_id: rule_id.into(),
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a plugin error
    pub fn plugin(plugin_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Plugin {
            plugin_name: plugin_name.into(),
            message: message.into(),
        }
    }

    /// Create a cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Get the file path associated with this error, if any
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Io { path, .. } => Some(path),
            Self::Parse { path, .. } => Some(path),
            Self::RuleExecution { path, .. } => Some(path),
            _ => None,
        }
    }
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(f, "Failed to read {}: {}", path.display(), error)
            }
            Self::Parse {
                path,
                message,
                line,
                column,
            } => match (line, column) {
                (Some(l), Some(c)) => {
                    write!(f, "Parse error in {} at {}:{}: {}", path.display(), l, c, message)
                }
                (Some(l), None) => {
                    write!(f, "Parse error in {} at line {}: {}", path.display(), l, message)
                }
                _ => {
                    write!(f, "Parse error in {}: {}", path.display(), message)
                }
            },
            Self::Config { message } => {
                write!(f, "Configuration error: {message}")
            }
            Self::RuleExecution {
                rule_id,
                path,
                message,
            } => {
                write!(f, "Rule '{}' failed on {}: {}", rule_id, path.display(), message)
            }
            Self::Plugin {
                plugin_name,
                message,
            } => {
                write!(f, "Plugin '{plugin_name}' error: {message}")
            }
            Self::Cache { message } => {
                write!(f, "Cache error: {message}")
            }
            Self::Internal { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CheckError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::new(),
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_source_no_errors() {
        let checker = Checker::new(CheckerConfig::default());
        let source = "const x = 1;";
        let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();
        // Should have no errors for clean code
        assert!(diagnostics.iter().all(|d| d.rule_id != "parse-error"));
    }

    #[test]
    fn test_check_source_with_debugger() {
        let checker = Checker::new(CheckerConfig::default());
        let source = "debugger;";
        let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();
        // Should detect debugger statement
        assert!(diagnostics.iter().any(|d| d.rule_id == "no-debugger"));
    }
}
