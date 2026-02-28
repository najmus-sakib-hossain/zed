//! Command registry types and structures
//!
//! This module defines the core types used by the command registry system.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Result of command execution
#[derive(Debug, Clone, Default)]
pub struct CommandResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

impl CommandResult {
    /// Create a successful result with stdout
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    /// Create an error result
    pub fn error(code: i32, stderr: impl Into<String>) -> Self {
        Self {
            exit_code: code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    /// Check if the command succeeded
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Command execution error
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Command not found
    NotFound {
        name: String,
        suggestions: Vec<String>,
    },
    /// Command is disabled
    Disabled { name: String },
    /// Missing required capability
    MissingCapability {
        name: String,
        capability: Capability,
    },
    /// Execution failed
    ExecutionFailed { name: String, reason: String },
    /// Invalid arguments
    InvalidArguments { name: String, reason: String },
    /// Timeout
    Timeout { name: String, timeout_ms: u64 },
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NotFound { name, suggestions } => {
                if suggestions.is_empty() {
                    write!(f, "Command not found: {}", name)
                } else {
                    write!(
                        f,
                        "Command not found: {}. Did you mean: {}?",
                        name,
                        suggestions.join(", ")
                    )
                }
            }
            CommandError::Disabled { name } => {
                write!(f, "Command is disabled: {}", name)
            }
            CommandError::MissingCapability { name, capability } => {
                write!(f, "Command '{}' requires capability: {:?}", name, capability)
            }
            CommandError::ExecutionFailed { name, reason } => {
                write!(f, "Command '{}' failed: {}", name, reason)
            }
            CommandError::InvalidArguments { name, reason } => {
                write!(f, "Invalid arguments for '{}': {}", name, reason)
            }
            CommandError::Timeout { name, timeout_ms } => {
                write!(f, "Command '{}' timed out after {}ms", name, timeout_ms)
            }
        }
    }
}

impl std::error::Error for CommandError {}

/// Synchronous command handler function type
pub type SyncHandler = Arc<dyn Fn(Vec<String>) -> Result<CommandResult, String> + Send + Sync>;

/// Async command handler function type
pub type AsyncHandler = Arc<
    dyn Fn(Vec<String>) -> Pin<Box<dyn Future<Output = Result<CommandResult, String>> + Send>>
        + Send
        + Sync,
>;

/// Command handler type
#[derive(Clone)]
pub enum HandlerType {
    /// Built-in synchronous handler
    BuiltIn(SyncHandler),
    /// Async handler
    Async(AsyncHandler),
    /// WASM plugin handler
    Wasm {
        module_path: String,
        function: String,
    },
    /// Native plugin handler (dynamic library)
    Native {
        library_path: String,
        symbol: String,
    },
    /// Script handler (shell/interpreter)
    Script { interpreter: String, script: String },
}

impl Default for HandlerType {
    fn default() -> Self {
        HandlerType::BuiltIn(Arc::new(|_| Ok(CommandResult::success(""))))
    }
}

impl std::fmt::Debug for HandlerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerType::BuiltIn(_) => write!(f, "BuiltIn(...)"),
            HandlerType::Async(_) => write!(f, "Async(...)"),
            HandlerType::Wasm {
                module_path,
                function,
            } => {
                write!(f, "Wasm {{ module: {}, function: {} }}", module_path, function)
            }
            HandlerType::Native {
                library_path,
                symbol,
            } => {
                write!(f, "Native {{ library: {}, symbol: {} }}", library_path, symbol)
            }
            HandlerType::Script {
                interpreter,
                script,
            } => {
                write!(f, "Script {{ interpreter: {}, script: {} }}", interpreter, script)
            }
        }
    }
}

/// Capability required by a command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// File system read access
    FileSystemRead,
    /// File system write access
    FileSystemWrite,
    /// Network access
    Network,
    /// Process execution
    ProcessExec,
    /// Environment variable access
    Environment,
    /// System information access
    SystemInfo,
    /// Git repository access
    Git,
    /// Browser automation
    Browser,
    /// AI/LLM access
    AI,
    /// Audio/TTS access
    Audio,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::FileSystemRead => write!(f, "fs:read"),
            Capability::FileSystemWrite => write!(f, "fs:write"),
            Capability::Network => write!(f, "network"),
            Capability::ProcessExec => write!(f, "process"),
            Capability::Environment => write!(f, "env"),
            Capability::SystemInfo => write!(f, "system"),
            Capability::Git => write!(f, "git"),
            Capability::Browser => write!(f, "browser"),
            Capability::AI => write!(f, "ai"),
            Capability::Audio => write!(f, "audio"),
        }
    }
}

/// Argument definition for a command
#[derive(Debug, Clone)]
pub struct ArgumentDef {
    /// Argument name
    pub name: String,
    /// Short flag (e.g., -h)
    pub short: Option<char>,
    /// Long flag (e.g., --help)
    pub long: Option<String>,
    /// Description
    pub description: String,
    /// Whether the argument is required
    pub required: bool,
    /// Default value
    pub default: Option<String>,
    /// Possible values (for enums)
    pub possible_values: Vec<String>,
    /// Whether this is a positional argument
    pub positional: bool,
    /// Position index for positional arguments
    pub position: Option<usize>,
}

impl Default for ArgumentDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            short: None,
            long: None,
            description: String::new(),
            required: false,
            default: None,
            possible_values: Vec::new(),
            positional: false,
            position: None,
        }
    }
}

/// Command entry in the registry
#[derive(Clone)]
pub struct CommandEntry {
    /// Command name
    pub name: String,
    /// Command description
    pub description: String,
    /// Long description/help text
    pub long_description: Option<String>,
    /// Command aliases
    pub aliases: Vec<String>,
    /// Category for grouping
    pub category: Option<String>,
    /// Command handler
    pub handler: HandlerType,
    /// Required capabilities
    pub capabilities: Vec<Capability>,
    /// Argument definitions
    pub arguments: Vec<ArgumentDef>,
    /// Usage examples
    pub examples: Vec<String>,
    /// Whether the command is enabled
    pub enabled: bool,
    /// Whether the command is hidden from help
    pub hidden: bool,
    /// Command version (for plugin versioning)
    pub version: Option<String>,
    /// Author information
    pub author: Option<String>,
}

impl Default for CommandEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            long_description: None,
            aliases: Vec::new(),
            category: None,
            handler: HandlerType::default(),
            capabilities: Vec::new(),
            arguments: Vec::new(),
            examples: Vec::new(),
            enabled: true,
            hidden: false,
            version: None,
            author: None,
        }
    }
}

impl std::fmt::Debug for CommandEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandEntry")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("aliases", &self.aliases)
            .field("category", &self.category)
            .field("handler", &self.handler)
            .field("capabilities", &self.capabilities)
            .field("enabled", &self.enabled)
            .field("hidden", &self.hidden)
            .finish()
    }
}

/// Builder for creating command entries
pub struct CommandBuilder {
    pub entry: CommandEntry,
}

impl CommandBuilder {
    /// Create a new command builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            entry: CommandEntry {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Set the command description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.entry.description = desc.into();
        self
    }

    /// Set the long description
    pub fn long_description(mut self, desc: impl Into<String>) -> Self {
        self.entry.long_description = Some(desc.into());
        self
    }

    /// Add an alias
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.entry.aliases.push(alias.into());
        self
    }

    /// Set the category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.entry.category = Some(category.into());
        self
    }

    /// Set the handler
    pub fn handler(mut self, handler: HandlerType) -> Self {
        self.entry.handler = handler;
        self
    }

    /// Set a synchronous handler function
    pub fn sync_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(Vec<String>) -> Result<CommandResult, String> + Send + Sync + 'static,
    {
        self.entry.handler = HandlerType::BuiltIn(Arc::new(f));
        self
    }

    /// Add a required capability
    pub fn capability(mut self, cap: Capability) -> Self {
        self.entry.capabilities.push(cap);
        self
    }

    /// Add an argument definition
    pub fn arg(mut self, arg: ArgumentDef) -> Self {
        self.entry.arguments.push(arg);
        self
    }

    /// Add a usage example
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.entry.examples.push(example.into());
        self
    }

    /// Set whether the command is hidden
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.entry.hidden = hidden;
        self
    }

    /// Set the version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.entry.version = Some(version.into());
        self
    }

    /// Set the author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.entry.author = Some(author.into());
        self
    }

    /// Build the command entry
    pub fn build(self) -> CommandEntry {
        self.entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder() {
        let cmd = CommandBuilder::new("test")
            .description("A test command")
            .alias("t")
            .category("Testing")
            .capability(Capability::FileSystemRead)
            .example("dx test --verbose")
            .build();

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.aliases, vec!["t"]);
        assert_eq!(cmd.category, Some("Testing".to_string()));
        assert!(cmd.capabilities.contains(&Capability::FileSystemRead));
    }

    #[test]
    fn test_command_result() {
        let success = CommandResult::success("Hello");
        assert!(success.is_success());
        assert_eq!(success.stdout, "Hello");

        let error = CommandResult::error(1, "Failed");
        assert!(!error.is_success());
        assert_eq!(error.stderr, "Failed");
    }

    #[test]
    fn test_capability_display() {
        assert_eq!(Capability::FileSystemRead.to_string(), "fs:read");
        assert_eq!(Capability::Network.to_string(), "network");
    }
}
