//! # Driven
//!
//! Professional AI-assisted development orchestrator with binary-first architecture.
//!
//! Driven brings structure, consistency, and intelligence to AI-powered coding workflows
//! by combining template-driven approaches with methodical frameworks, reimagined in Rust
//! with DX's binary-first philosophy for unparalleled performance.
//!
//! ## Features
//!
//! - **Universal Rule Format**: One source of truth, convert to any editor
//! - **Binary-First Storage**: Using DX serializer principles for 70%+ size reduction
//! - **Context Intelligence**: Deep project analysis for AI guidance
//! - **Professional Templates**: Battle-tested patterns for AI agents
//! - **Zero-Parse Loading**: Instant rule loading with memory-mapped binaries
//! - **DX Binary Dawn**: SIMD scanning, XOR patching, Ed25519 signing
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use driven::{DrivenConfig, RuleSet};
//!
//! // Load existing rules
//! let rules = RuleSet::load(".cursorrules")?;
//!
//! // Convert to different formats
//! rules.emit_copilot(".github/copilot-instructions.md")?;
//! rules.emit_windsurf(".windsurfrules")?;
//!
//! // Or use binary format for maximum efficiency
//! rules.save_binary(".driven/rules.drv")?;
//! ```
//!
//! ## Architecture
//!
//! Driven is organized into several key modules:
//!
//! - [`format`]: Binary rule format (.drv) encoding/decoding
//! - [`parser`]: Universal parser for all editor rule formats
//! - [`emitter`]: Target format generators for each editor
//! - [`templates`]: Template system with built-in library
//! - [`context`]: AI context intelligence and project analysis
//! - [`sync`]: Multi-editor synchronization
//! - [`validation`]: Rule validation and linting
//!
//! ## DX Binary Dawn Modules
//!
//! - [`binary`]: DX ∞ infinity format, zero-copy schemas, SIMD tokenizer
//! - [`fusion`]: Pre-compiled templates, hot cache, speculative loading
//! - [`streaming`]: HTIP delivery, XOR patching, ETag negotiation
//! - [`security`]: Ed25519 signing, capability manifest, sandbox
//! - [`state`]: Dirty-bit tracking, shared rules, atomic sync

// Note: Some fields are for API completeness and future use

// Core modules
pub mod agents;
pub mod config_validation;
pub mod context;
pub mod dcp;
pub mod dx_integration;
pub mod emitter;
pub mod error;
pub mod format;
pub mod generator_integration;
pub mod hooks;
pub mod integration;
pub mod modules;
pub mod parser;
pub mod scale;
pub mod steering;
pub mod sync;
pub mod sysinfo;
pub mod templates;
pub mod validation;
pub mod workflows;

// DX Binary Dawn modules
pub mod binary;
pub mod fusion;
pub mod security;
pub mod state;
pub mod streaming;

#[cfg(feature = "cli")]
pub mod cli;

// Re-export core types
pub use context::{ProjectAnalyzer, ProjectContext};
pub use emitter::Emitter;
pub use format::{DrvDecoder, DrvEncoder, DrvHeader, RuleCategory};
pub use parser::{ParsedRule, Parser, UnifiedRule};
pub use sync::SyncEngine;
pub use templates::{Template, TemplateRegistry};
pub use validation::{Linter, ValidationResult};

// Re-export DX Binary Dawn types
pub use binary::{
    Blake3Checksum, InfinityHeader, InfinityRule, MappedRule, SimdTokenizer, StringId, StringTable,
    StringTableBuilder,
};
pub use fusion::{BinaryCache, FusionModule, HotCache, SpeculativeLoader};
pub use security::{Capability, CapabilityManifest, Ed25519Signer, IntegrityGuard, Sandbox};
pub use state::{AtomicSync, DirtyBits, RuleSnapshot, SharedRules, SnapshotManager};
pub use streaming::{ChunkStreamer, ETagNegotiator, HtipDelivery, XorPatcher};

// Re-export DX integration
pub use dx_integration::DxSerializable;
pub use dx_integration::{
    DxDocumentable, DxMarkdownConfig, DxMarkdownFormat, rules_to_dx_markdown,
};
pub use dx_integration::{LegacyConverter, LegacyFormat, LegacySerializable};

// Re-export DCP integration
pub use dcp::{ConnectionState, DcpClient, DcpConfig, ToolDefinition};

// Re-export generator integration
pub use generator_integration::{
    DrivenTemplate, DrivenTemplateProvider, DrivenTemplateType, GenerateParams, GeneratedFile,
    GeneratorBridge, TemplateCategory, TemplateInfo,
};

// Re-export cross-crate integration
pub use integration::{
    BinaryFormatChecker, CrossCrateHeader, DrivenDcpBridge, DrivenTool, DrivenToolCategory,
    HookMessage, HookMessenger, SourceCrate, SpecScaffoldResult, SpecScaffolder,
    ValidationResult as IntegrationValidationResult,
};

// Re-export error handling
pub use error::{EnhancedError, EnhancedResult, ErrorContext};

// Re-export config validation
pub use config_validation::{
    ConfigValidator, ValidationError, ValidationReport, ValidationWarning, validate_config,
};

// Re-export hooks system
pub use hooks::{
    AgentHook, BuildEvent, GitOp, HookAction, HookCondition, HookContext, HookEngine,
    HookExecutionResult, HookTrigger, TestFilter, TestResult, TestStatus,
};

// Re-export steering system
pub use steering::{AgentContext, FileReference, SteeringEngine, SteeringInclusion, SteeringRule};

// Re-export sysinfo system
pub use sysinfo::{
    BuildToolInfo, GitInfo, LanguageInfo, OsInfo, PackageManagerInfo, ProjectInfo, ProjectType,
    ShellInfo, SystemInfo, SystemInfoCache, SystemInfoProvider, TestFrameworkInfo,
};

// Re-export agents system
pub use agents::{Agent, AgentPersona, AgentRegistry, DelegationRequest, DelegationResult};

// Re-export workflows system
pub use workflows::{
    StepResult, Workflow, WorkflowBranch, WorkflowEngine, WorkflowPhase, WorkflowProgress,
    WorkflowSession, WorkflowStep,
};

// Re-export scale system
pub use scale::{
    ComplexityAnalyzer, DependencyAnalyzer, FileSizeAnalyzer, HistoryAnalyzer,
    ProjectContext as ScaleProjectContext, ProjectScale, ScaleAnalyzer, ScaleDetector,
    ScaleRecommendation, TeamSizeAnalyzer,
};

// Re-export modules system
pub use modules::{Module, ModuleDependency, ModuleManager, ModuleStatus};

/// Configuration for Driven
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DrivenConfig {
    /// Version of the configuration format
    pub version: String,
    /// Default editor to target
    pub default_editor: Editor,
    /// Enabled editors for sync
    pub editors: EditorConfig,
    /// Template configuration
    pub templates: TemplateConfig,
    /// Sync settings
    pub sync: SyncConfig,
    /// Context analysis settings
    pub context: ContextConfig,
}

impl Default for DrivenConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            default_editor: Editor::Cursor,
            editors: EditorConfig::default(),
            templates: TemplateConfig::default(),
            sync: SyncConfig::default(),
            context: ContextConfig::default(),
        }
    }
}

/// Supported AI code editors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Editor {
    /// Cursor AI editor
    Cursor,
    /// GitHub Copilot in VS Code
    Copilot,
    /// Windsurf (Codeium)
    Windsurf,
    /// Claude Code (Anthropic)
    Claude,
    /// Aider CLI
    Aider,
    /// Cline extension
    Cline,
}

impl std::fmt::Display for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Editor {
    /// Get the default rule file path for this editor
    pub fn rule_path(&self) -> &'static str {
        match self {
            Editor::Cursor => ".cursorrules",
            Editor::Copilot => ".github/copilot-instructions.md",
            Editor::Windsurf => ".windsurfrules",
            Editor::Claude => ".claude/settings.json",
            Editor::Aider => ".aider.conf.yml",
            Editor::Cline => ".cline/settings.json",
        }
    }

    /// Get a human-readable name for this editor
    pub fn display_name(&self) -> &'static str {
        match self {
            Editor::Cursor => "Cursor",
            Editor::Copilot => "GitHub Copilot",
            Editor::Windsurf => "Windsurf",
            Editor::Claude => "Claude Code",
            Editor::Aider => "Aider",
            Editor::Cline => "Cline",
        }
    }
}

/// Editor enablement configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EditorConfig {
    pub cursor: bool,
    pub copilot: bool,
    pub windsurf: bool,
    pub claude: bool,
    pub aider: bool,
    pub cline: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            cursor: true,
            copilot: true,
            windsurf: false,
            claude: true,
            aider: false,
            cline: false,
        }
    }
}

/// Template configuration
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct TemplateConfig {
    /// Active persona templates
    pub personas: Vec<String>,
    /// Project type template
    pub project: Option<String>,
    /// Active standard templates
    pub standards: Vec<String>,
    /// Active workflow template
    pub workflow: Option<String>,
}

/// Synchronization configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SyncConfig {
    /// Watch for file changes
    pub watch: bool,
    /// Automatically convert on change
    pub auto_convert: bool,
    /// Source of truth file
    pub source_of_truth: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            watch: true,
            auto_convert: true,
            source_of_truth: ".driven/rules.drv".to_string(),
        }
    }
}

/// Context analysis configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ContextConfig {
    /// Patterns to include in analysis
    pub include: Vec<String>,
    /// Patterns to exclude from analysis
    pub exclude: Vec<String>,
    /// Path to binary index
    pub index_path: String,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            include: vec!["src/**".to_string(), "crates/**".to_string()],
            exclude: vec!["target/**".to_string(), "node_modules/**".to_string()],
            index_path: ".driven/index.drv".to_string(),
        }
    }
}

/// A complete rule set that can be converted between formats
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    /// The unified rules
    pub rules: Vec<UnifiedRule>,
    /// Source file path (if loaded from file)
    pub source: Option<std::path::PathBuf>,
}

impl RuleSet {
    /// Create an empty rule set
    pub fn new() -> Self {
        Self::default()
    }

    /// Load rules from any supported format
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let parser = Parser::detect(path)?;
        let rules = parser.parse_file(path)?;
        Ok(Self {
            rules,
            source: Some(path.to_path_buf()),
        })
    }

    /// Load rules from binary format (.drv)
    pub fn load_binary(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let data = std::fs::read(path.as_ref())?;
        let decoder = DrvDecoder::new(&data)?;
        let rules = decoder.decode_all()?;
        Ok(Self {
            rules,
            source: Some(path.as_ref().to_path_buf()),
        })
    }

    /// Save rules to binary format (.drv)
    pub fn save_binary(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let encoder = DrvEncoder::new();
        let data = encoder.encode(&self.rules)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Emit rules to a specific editor format
    pub fn emit(&self, editor: Editor, path: impl AsRef<std::path::Path>) -> Result<()> {
        let emitter = Emitter::for_editor(editor);
        emitter.emit(&self.rules, path)?;
        Ok(())
    }

    /// Emit rules to Cursor format
    pub fn emit_cursor(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.emit(Editor::Cursor, path)
    }

    /// Emit rules to Copilot format
    pub fn emit_copilot(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.emit(Editor::Copilot, path)
    }

    /// Emit rules to Windsurf format
    pub fn emit_windsurf(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        self.emit(Editor::Windsurf, path)
    }

    /// Get the number of rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the rule set is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Get rules as unified format
    pub fn as_unified(&self) -> Vec<UnifiedRule> {
        self.rules.clone()
    }
}

/// Main error type for Driven
#[derive(Debug, thiserror::Error)]
pub enum DrivenError {
    #[error("IO error: {0}")]
    Io(std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Format error: {0}")]
    Format(String),

    #[error("Invalid binary format: {0}")]
    InvalidBinary(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Result type alias for Driven operations
pub type Result<T> = std::result::Result<T, DrivenError>;

impl From<std::io::Error> for DrivenError {
    fn from(err: std::io::Error) -> Self {
        DrivenError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DrivenConfig::default();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.default_editor, Editor::Cursor);
        assert!(config.editors.cursor);
        assert!(config.editors.copilot);
    }

    #[test]
    fn test_editor_paths() {
        assert_eq!(Editor::Cursor.rule_path(), ".cursorrules");
        assert_eq!(Editor::Copilot.rule_path(), ".github/copilot-instructions.md");
        assert_eq!(Editor::Windsurf.rule_path(), ".windsurfrules");
    }

    #[test]
    fn test_empty_ruleset() {
        let rules = RuleSet::new();
        assert!(rules.rules.is_empty());
        assert!(rules.source.is_none());
    }
}
