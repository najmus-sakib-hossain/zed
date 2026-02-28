//! Command registry loader for .sr configuration files
//!
//! This module handles loading command definitions from DX Serializer (.sr) files.

use super::CommandRegistry;
use super::types::*;
use std::path::Path;

/// Registry loader for .sr configuration files
pub struct RegistryLoader;

impl RegistryLoader {
    /// Load commands from a .sr file
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
        registry: &CommandRegistry,
    ) -> Result<usize, LoaderError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| LoaderError::IoError(e.to_string()))?;

        Self::load_from_str(&content, registry)
    }

    /// Load commands from a .sr file and override existing entries
    pub fn load_from_file_override<P: AsRef<Path>>(
        path: P,
        registry: &CommandRegistry,
    ) -> Result<usize, LoaderError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| LoaderError::IoError(e.to_string()))?;

        Self::load_from_str_override(&content, registry)
    }

    /// Load commands from a string
    pub fn load_from_str(content: &str, registry: &CommandRegistry) -> Result<usize, LoaderError> {
        let commands = Self::parse_sr(content)?;
        let count = commands.len();

        for cmd in commands {
            registry.register(cmd);
        }

        Ok(count)
    }

    /// Load commands from a string and override existing entries
    pub fn load_from_str_override(
        content: &str,
        registry: &CommandRegistry,
    ) -> Result<usize, LoaderError> {
        let commands = Self::parse_sr(content)?;
        let count = commands.len();

        for cmd in commands {
            registry.register_override(cmd);
        }

        Ok(count)
    }

    /// Parse .sr format into command entries
    fn parse_sr(content: &str) -> Result<Vec<CommandEntry>, LoaderError> {
        let mut commands = Vec::new();
        let mut current_command: Option<CommandBuilder> = None;
        let mut current_section = String::new();
        let mut current_args: Vec<ArgumentDef> = Vec::new();
        let mut current_arg: Option<ArgumentDef> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            // New command section
            if line.starts_with("[[command]]") {
                // Save previous command
                if let Some(builder) = current_command.take() {
                    let mut entry = builder.build();
                    entry.arguments = std::mem::take(&mut current_args);
                    commands.push(entry);
                }
                current_command = Some(CommandBuilder::new(""));
                current_section = "command".to_string();
                continue;
            }

            // Argument section
            if line.starts_with("[[argument]]") || line.starts_with("[[arg]]") {
                // Save previous argument
                if let Some(arg) = current_arg.take() {
                    current_args.push(arg);
                }
                current_arg = Some(ArgumentDef::default());
                current_section = "argument".to_string();
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Key-value pair
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim().trim_matches('"');

                match current_section.as_str() {
                    "command" => {
                        if let Some(ref mut builder) = current_command {
                            Self::apply_command_field(builder, key, value);
                        }
                    }
                    "argument" | "arg" => {
                        if let Some(ref mut arg) = current_arg {
                            Self::apply_argument_field(arg, key, value);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Save last argument
        if let Some(arg) = current_arg.take() {
            current_args.push(arg);
        }

        // Save last command
        if let Some(builder) = current_command.take() {
            let mut entry = builder.build();
            entry.arguments = current_args;
            commands.push(entry);
        }

        Ok(commands)
    }

    /// Apply a field value to a command builder
    fn apply_command_field(builder: &mut CommandBuilder, key: &str, value: &str) {
        // We need to modify the builder in place, so we use a match
        match key {
            "name" => {
                builder.entry.name = value.to_string();
            }
            "description" | "desc" => {
                builder.entry.description = value.to_string();
            }
            "long_description" => {
                builder.entry.long_description = Some(value.to_string());
            }
            "category" => {
                builder.entry.category = Some(value.to_string());
            }
            "aliases" | "alias" => {
                builder.entry.aliases = value.split(',').map(|s| s.trim().to_string()).collect();
            }
            "capabilities" | "caps" => {
                builder.entry.capabilities =
                    value.split(',').filter_map(|s| Self::parse_capability(s.trim())).collect();
            }
            "enabled" => {
                builder.entry.enabled = value.parse().unwrap_or(true);
            }
            "hidden" => {
                builder.entry.hidden = value.parse().unwrap_or(false);
            }
            "version" => {
                builder.entry.version = Some(value.to_string());
            }
            "author" => {
                builder.entry.author = Some(value.to_string());
            }
            "script" => {
                builder.entry.handler = HandlerType::Script {
                    interpreter: "sh".to_string(),
                    script: value.to_string(),
                };
            }
            "interpreter" => {
                if let HandlerType::Script { interpreter, .. } = &mut builder.entry.handler {
                    *interpreter = value.to_string();
                }
            }
            "wasm_module" => {
                builder.entry.handler = HandlerType::Wasm {
                    module_path: value.to_string(),
                    function: "main".to_string(),
                };
            }
            "wasm_function" => {
                if let HandlerType::Wasm { function, .. } = &mut builder.entry.handler {
                    *function = value.to_string();
                }
            }
            "native_library" => {
                builder.entry.handler = HandlerType::Native {
                    library_path: value.to_string(),
                    symbol: "dx_command".to_string(),
                };
            }
            "native_symbol" => {
                if let HandlerType::Native { symbol, .. } = &mut builder.entry.handler {
                    *symbol = value.to_string();
                }
            }
            "example" => {
                builder.entry.examples.push(value.to_string());
            }
            _ => {}
        }
    }

    /// Apply a field value to an argument definition
    fn apply_argument_field(arg: &mut ArgumentDef, key: &str, value: &str) {
        match key {
            "name" => arg.name = value.to_string(),
            "short" => arg.short = value.chars().next(),
            "long" => arg.long = Some(value.to_string()),
            "description" | "desc" => arg.description = value.to_string(),
            "required" => arg.required = value.parse().unwrap_or(false),
            "default" => arg.default = Some(value.to_string()),
            "values" | "possible_values" => {
                arg.possible_values = value.split(',').map(|s| s.trim().to_string()).collect();
            }
            "positional" => arg.positional = value.parse().unwrap_or(false),
            "position" => arg.position = value.parse().ok(),
            _ => {}
        }
    }

    /// Parse a capability string
    fn parse_capability(s: &str) -> Option<Capability> {
        match s.to_lowercase().as_str() {
            "fs:read" | "filesystem:read" | "file:read" => Some(Capability::FileSystemRead),
            "fs:write" | "filesystem:write" | "file:write" => Some(Capability::FileSystemWrite),
            "network" | "net" => Some(Capability::Network),
            "process" | "exec" => Some(Capability::ProcessExec),
            "env" | "environment" => Some(Capability::Environment),
            "system" | "sysinfo" => Some(Capability::SystemInfo),
            "git" => Some(Capability::Git),
            "browser" => Some(Capability::Browser),
            "ai" | "llm" => Some(Capability::AI),
            "audio" | "tts" => Some(Capability::Audio),
            _ => None,
        }
    }

    /// Load commands from the default directory
    pub fn load_default(registry: &CommandRegistry) -> Result<usize, LoaderError> {
        let config_dir = dirs::home_dir()
            .map(|h| h.join(".dx").join("config"))
            .ok_or_else(|| LoaderError::IoError("Could not find home directory".to_string()))?;

        let commands_file = config_dir.join("commands.sr");
        if commands_file.exists() {
            Self::load_from_file_override(&commands_file, registry)
        } else {
            Ok(0)
        }
    }

    /// Load all .sr files from a directory
    pub fn load_directory<P: AsRef<Path>>(
        path: P,
        registry: &CommandRegistry,
    ) -> Result<usize, LoaderError> {
        let mut total = 0;

        let entries =
            std::fs::read_dir(path.as_ref()).map_err(|e| LoaderError::IoError(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "sr") {
                match Self::load_from_file_override(&path, registry) {
                    Ok(count) => total += count,
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(total)
    }
}

/// Loader error types
#[derive(Debug, Clone)]
pub enum LoaderError {
    /// I/O error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::IoError(e) => write!(f, "IO error: {}", e),
            LoaderError::ParseError(e) => write!(f, "Parse error: {}", e),
            LoaderError::InvalidConfig(e) => write!(f, "Invalid config: {}", e),
        }
    }
}

impl std::error::Error for LoaderError {}

/// Generate a default commands.sr file
pub fn generate_default_commands_sr() -> String {
    r#"# DX CLI Command Definitions
# This file uses DX Serializer (.sr) format

# Example custom command
[[command]]
name = "hello"
description = "Say hello to someone"
category = "Examples"
aliases = "hi, hey"
capabilities = "env"
script = "echo Hello, ${1:-World}!"

[[argument]]
name = "name"
positional = true
position = 0
description = "Name to greet"
default = "World"

# Example with multiple arguments
[[command]]
name = "greet"
description = "Greet with customization"
category = "Examples"
script = "echo ${GREETING:-Hello}, ${NAME:-friend}!"

[[argument]]
name = "greeting"
short = "g"
long = "greeting"
description = "The greeting to use"
default = "Hello"

[[argument]]
name = "name"
short = "n"
long = "name"
description = "Name to greet"
required = true
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let content = r#"
[[command]]
name = "test"
description = "A test command"
category = "Testing"
aliases = "t, tst"
"#;

        let registry = CommandRegistry::new();
        let count = RegistryLoader::load_from_str(content, &registry).unwrap();

        assert_eq!(count, 1);
        let cmd = registry.get("test").unwrap();
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, "A test command");
        assert!(cmd.aliases.contains(&"t".to_string()));
    }

    #[test]
    fn test_parse_command_with_args() {
        let content = r#"
[[command]]
name = "greet"
description = "Greet someone"

[[argument]]
name = "name"
short = "n"
long = "name"
required = true
description = "Name to greet"
"#;

        let registry = CommandRegistry::new();
        RegistryLoader::load_from_str(content, &registry).unwrap();

        let cmd = registry.get("greet").unwrap();
        assert_eq!(cmd.arguments.len(), 1);
        assert_eq!(cmd.arguments[0].name, "name");
        assert!(cmd.arguments[0].required);
    }

    #[test]
    fn test_parse_script_command() {
        let content = r#"
[[command]]
name = "echo-test"
description = "Echo test"
script = "echo Hello"
interpreter = "bash"
"#;

        let registry = CommandRegistry::new();
        RegistryLoader::load_from_str(content, &registry).unwrap();

        let cmd = registry.get("echo-test").unwrap();
        if let HandlerType::Script {
            interpreter,
            script,
        } = &cmd.handler
        {
            assert_eq!(interpreter, "bash");
            assert_eq!(script, "echo Hello");
        } else {
            panic!("Expected script handler");
        }
    }

    #[test]
    fn test_parse_capabilities() {
        let content = r#"
[[command]]
name = "network-cmd"
description = "A network command"
capabilities = "network, fs:read"
"#;

        let registry = CommandRegistry::new();
        RegistryLoader::load_from_str(content, &registry).unwrap();

        let cmd = registry.get("network-cmd").unwrap();
        assert!(cmd.capabilities.contains(&Capability::Network));
        assert!(cmd.capabilities.contains(&Capability::FileSystemRead));
    }

    #[test]
    fn test_load_override_replaces_existing() {
        let registry = CommandRegistry::new();

        let built_in = r#"
[[command]]
name = "doctor"
description = "Built-in"
version = "2.0.0"
"#;

        let user = r#"
[[command]]
name = "doctor"
description = "User override"
version = "1.0.0"
"#;

        RegistryLoader::load_from_str(built_in, &registry).unwrap();
        RegistryLoader::load_from_str_override(user, &registry).unwrap();

        let cmd = registry.get("doctor").unwrap();
        assert_eq!(cmd.description, "User override");
        assert_eq!(cmd.version.as_deref(), Some("1.0.0"));
    }
}
