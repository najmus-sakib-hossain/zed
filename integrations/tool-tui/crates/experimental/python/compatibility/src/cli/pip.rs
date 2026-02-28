//! pip command compatibility layer
//!
//! Implements `dx-py pip` subcommand with pip-compatible behavior.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use thiserror::Error;

/// pip command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipCommand {
    /// Install packages
    Install {
        packages: Vec<String>,
        requirements: Option<PathBuf>,
        editable: bool,
        upgrade: bool,
        no_deps: bool,
        target: Option<PathBuf>,
        user: bool,
        quiet: bool,
        verbose: bool,
    },
    /// Uninstall packages
    Uninstall {
        packages: Vec<String>,
        requirements: Option<PathBuf>,
        yes: bool,
    },
    /// List installed packages
    List {
        outdated: bool,
        uptodate: bool,
        editable: bool,
        format: ListFormat,
    },
    /// Show package information
    Show {
        packages: Vec<String>,
        files: bool,
    },
    /// Freeze installed packages
    Freeze {
        all: bool,
        local: bool,
        exclude_editable: bool,
    },
    /// Download packages
    Download {
        packages: Vec<String>,
        dest: PathBuf,
        platform: Option<String>,
        python_version: Option<String>,
    },
    /// Build wheels
    Wheel {
        packages: Vec<String>,
        wheel_dir: PathBuf,
        no_deps: bool,
    },
    /// Check dependencies
    Check,
    /// Search PyPI (deprecated but supported)
    Search {
        query: String,
    },
    /// Show pip configuration
    Config {
        action: ConfigAction,
    },
    /// Cache management
    Cache {
        action: CacheAction,
    },
    /// Show pip version
    Version,
    /// Show help
    Help {
        command: Option<String>,
    },
}

/// List output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListFormat {
    #[default]
    Columns,
    Freeze,
    Json,
}

/// Config action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigAction {
    List,
    Get { key: String },
    Set { key: String, value: String },
    Unset { key: String },
    Edit,
}

/// Cache action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheAction {
    Dir,
    Info,
    List { pattern: Option<String> },
    Remove { pattern: String },
    Purge,
}

/// Result of a pip command
#[derive(Debug)]
pub struct PipCommandResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Packages affected
    pub packages_affected: Vec<String>,
}

impl PipCommandResult {
    /// Check if command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// pip compatibility layer
pub struct PipCompatLayer {
    python_path: PathBuf,
    site_packages: PathBuf,
    index_url: String,
    extra_index_urls: Vec<String>,
    trusted_hosts: Vec<String>,
    cache_dir: PathBuf,
}

#[derive(Error, Debug)]
pub enum PipError {
    #[error("Package not found: {0}")]
    PackageNotFound(String),
    #[error("Invalid requirement: {0}")]
    InvalidRequirement(String),
    #[error("Installation failed: {0}")]
    InstallationFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
}

impl PipCompatLayer {
    /// Create a new pip compatibility layer
    pub fn new(python_path: PathBuf) -> Self {
        let site_packages = python_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib").join("site-packages"))
            .unwrap_or_default();
        
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx-py")
            .join("pip");

        Self {
            python_path,
            site_packages,
            index_url: "https://pypi.org/simple/".to_string(),
            extra_index_urls: vec![],
            trusted_hosts: vec![],
            cache_dir,
        }
    }

    /// Set index URL
    pub fn with_index_url(mut self, url: String) -> Self {
        self.index_url = url;
        self
    }

    /// Add extra index URL
    pub fn with_extra_index_url(mut self, url: String) -> Self {
        self.extra_index_urls.push(url);
        self
    }

    /// Parse command line arguments into a PipCommand
    pub fn parse_args(args: &[String]) -> Result<PipCommand, PipError> {
        if args.is_empty() {
            return Ok(PipCommand::Help { command: None });
        }

        match args[0].as_str() {
            "install" => Self::parse_install(&args[1..]),
            "uninstall" | "remove" => Self::parse_uninstall(&args[1..]),
            "list" => Self::parse_list(&args[1..]),
            "show" => Self::parse_show(&args[1..]),
            "freeze" => Self::parse_freeze(&args[1..]),
            "download" => Self::parse_download(&args[1..]),
            "wheel" => Self::parse_wheel(&args[1..]),
            "check" => Ok(PipCommand::Check),
            "search" => Self::parse_search(&args[1..]),
            "config" => Self::parse_config(&args[1..]),
            "cache" => Self::parse_cache(&args[1..]),
            "--version" | "-V" => Ok(PipCommand::Version),
            "--help" | "-h" | "help" => {
                let cmd = args.get(1).cloned();
                Ok(PipCommand::Help { command: cmd })
            }
            _ => Err(PipError::ParseError(format!("Unknown command: {}", args[0]))),
        }
    }

    /// Parse install command
    fn parse_install(args: &[String]) -> Result<PipCommand, PipError> {
        let mut packages = Vec::new();
        let mut requirements = None;
        let mut editable = false;
        let mut upgrade = false;
        let mut no_deps = false;
        let mut target = None;
        let mut user = false;
        let mut quiet = false;
        let mut verbose = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-r" | "--requirement" => {
                    i += 1;
                    if i < args.len() {
                        requirements = Some(PathBuf::from(&args[i]));
                    }
                }
                "-e" | "--editable" => {
                    editable = true;
                    i += 1;
                    if i < args.len() && !args[i].starts_with('-') {
                        packages.push(args[i].clone());
                    } else {
                        i -= 1;
                    }
                }
                "-U" | "--upgrade" => upgrade = true,
                "--no-deps" => no_deps = true,
                "-t" | "--target" => {
                    i += 1;
                    if i < args.len() {
                        target = Some(PathBuf::from(&args[i]));
                    }
                }
                "--user" => user = true,
                "-q" | "--quiet" => quiet = true,
                "-v" | "--verbose" => verbose = true,
                arg if !arg.starts_with('-') => packages.push(arg.to_string()),
                _ => {}
            }
            i += 1;
        }

        Ok(PipCommand::Install {
            packages,
            requirements,
            editable,
            upgrade,
            no_deps,
            target,
            user,
            quiet,
            verbose,
        })
    }

    /// Parse uninstall command
    fn parse_uninstall(args: &[String]) -> Result<PipCommand, PipError> {
        let mut packages = Vec::new();
        let mut requirements = None;
        let mut yes = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-r" | "--requirement" => {
                    i += 1;
                    if i < args.len() {
                        requirements = Some(PathBuf::from(&args[i]));
                    }
                }
                "-y" | "--yes" => yes = true,
                arg if !arg.starts_with('-') => packages.push(arg.to_string()),
                _ => {}
            }
            i += 1;
        }

        Ok(PipCommand::Uninstall {
            packages,
            requirements,
            yes,
        })
    }

    /// Parse list command
    fn parse_list(args: &[String]) -> Result<PipCommand, PipError> {
        let mut outdated = false;
        let mut uptodate = false;
        let mut editable = false;
        let mut format = ListFormat::Columns;

        for arg in args {
            match arg.as_str() {
                "-o" | "--outdated" => outdated = true,
                "-u" | "--uptodate" => uptodate = true,
                "-e" | "--editable" => editable = true,
                "--format=columns" => format = ListFormat::Columns,
                "--format=freeze" => format = ListFormat::Freeze,
                "--format=json" => format = ListFormat::Json,
                _ => {}
            }
        }

        Ok(PipCommand::List {
            outdated,
            uptodate,
            editable,
            format,
        })
    }

    /// Parse show command
    fn parse_show(args: &[String]) -> Result<PipCommand, PipError> {
        let mut packages = Vec::new();
        let mut files = false;

        for arg in args {
            match arg.as_str() {
                "-f" | "--files" => files = true,
                arg if !arg.starts_with('-') => packages.push(arg.to_string()),
                _ => {}
            }
        }

        Ok(PipCommand::Show { packages, files })
    }

    /// Parse freeze command
    fn parse_freeze(args: &[String]) -> Result<PipCommand, PipError> {
        let mut all = false;
        let mut local = false;
        let mut exclude_editable = false;

        for arg in args {
            match arg.as_str() {
                "--all" => all = true,
                "-l" | "--local" => local = true,
                "--exclude-editable" => exclude_editable = true,
                _ => {}
            }
        }

        Ok(PipCommand::Freeze {
            all,
            local,
            exclude_editable,
        })
    }

    /// Parse download command
    fn parse_download(args: &[String]) -> Result<PipCommand, PipError> {
        let mut packages = Vec::new();
        let mut dest = PathBuf::from(".");
        let mut platform = None;
        let mut python_version = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-d" | "--dest" => {
                    i += 1;
                    if i < args.len() {
                        dest = PathBuf::from(&args[i]);
                    }
                }
                "--platform" => {
                    i += 1;
                    if i < args.len() {
                        platform = Some(args[i].clone());
                    }
                }
                "--python-version" => {
                    i += 1;
                    if i < args.len() {
                        python_version = Some(args[i].clone());
                    }
                }
                arg if !arg.starts_with('-') => packages.push(arg.to_string()),
                _ => {}
            }
            i += 1;
        }

        Ok(PipCommand::Download {
            packages,
            dest,
            platform,
            python_version,
        })
    }

    /// Parse wheel command
    fn parse_wheel(args: &[String]) -> Result<PipCommand, PipError> {
        let mut packages = Vec::new();
        let mut wheel_dir = PathBuf::from(".");
        let mut no_deps = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-w" | "--wheel-dir" => {
                    i += 1;
                    if i < args.len() {
                        wheel_dir = PathBuf::from(&args[i]);
                    }
                }
                "--no-deps" => no_deps = true,
                arg if !arg.starts_with('-') => packages.push(arg.to_string()),
                _ => {}
            }
            i += 1;
        }

        Ok(PipCommand::Wheel {
            packages,
            wheel_dir,
            no_deps,
        })
    }

    /// Parse search command
    fn parse_search(args: &[String]) -> Result<PipCommand, PipError> {
        let query = args.iter()
            .find(|a| !a.starts_with('-'))
            .cloned()
            .unwrap_or_default();
        
        Ok(PipCommand::Search { query })
    }

    /// Parse config command
    fn parse_config(args: &[String]) -> Result<PipCommand, PipError> {
        if args.is_empty() {
            return Ok(PipCommand::Config {
                action: ConfigAction::List,
            });
        }

        let action = match args[0].as_str() {
            "list" => ConfigAction::List,
            "get" => ConfigAction::Get {
                key: args.get(1).cloned().unwrap_or_default(),
            },
            "set" => ConfigAction::Set {
                key: args.get(1).cloned().unwrap_or_default(),
                value: args.get(2).cloned().unwrap_or_default(),
            },
            "unset" => ConfigAction::Unset {
                key: args.get(1).cloned().unwrap_or_default(),
            },
            "edit" => ConfigAction::Edit,
            _ => ConfigAction::List,
        };

        Ok(PipCommand::Config { action })
    }

    /// Parse cache command
    fn parse_cache(args: &[String]) -> Result<PipCommand, PipError> {
        if args.is_empty() {
            return Ok(PipCommand::Cache {
                action: CacheAction::Info,
            });
        }

        let action = match args[0].as_str() {
            "dir" => CacheAction::Dir,
            "info" => CacheAction::Info,
            "list" => CacheAction::List {
                pattern: args.get(1).cloned(),
            },
            "remove" => CacheAction::Remove {
                pattern: args.get(1).cloned().unwrap_or_else(|| "*".to_string()),
            },
            "purge" => CacheAction::Purge,
            _ => CacheAction::Info,
        };

        Ok(PipCommand::Cache { action })
    }

    /// Execute a pip command
    pub fn execute(&self, command: &PipCommand) -> Result<PipCommandResult, PipError> {
        match command {
            PipCommand::Install { packages, requirements, editable, upgrade, no_deps, target, user, quiet, verbose } => {
                self.execute_install(packages, requirements.as_deref(), *editable, *upgrade, *no_deps, target.as_deref(), *user, *quiet, *verbose)
            }
            PipCommand::Uninstall { packages, requirements, yes } => {
                self.execute_uninstall(packages, requirements.as_deref(), *yes)
            }
            PipCommand::List { outdated, uptodate, editable, format } => {
                self.execute_list(*outdated, *uptodate, *editable, *format)
            }
            PipCommand::Show { packages, files } => {
                self.execute_show(packages, *files)
            }
            PipCommand::Freeze { all, local, exclude_editable } => {
                self.execute_freeze(*all, *local, *exclude_editable)
            }
            PipCommand::Check => self.execute_check(),
            PipCommand::Version => self.execute_version(),
            PipCommand::Help { command } => self.execute_help(command.as_deref()),
            _ => Ok(PipCommandResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: "Command not yet implemented".to_string(),
                packages_affected: vec![],
            }),
        }
    }

    /// Execute install command
    fn execute_install(
        &self,
        packages: &[String],
        requirements: Option<&Path>,
        editable: bool,
        upgrade: bool,
        no_deps: bool,
        target: Option<&Path>,
        user: bool,
        quiet: bool,
        verbose: bool,
    ) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip", "install"];
        
        if editable {
            args.push("-e");
        }
        if upgrade {
            args.push("-U");
        }
        if no_deps {
            args.push("--no-deps");
        }
        if user {
            args.push("--user");
        }
        if quiet {
            args.push("-q");
        }
        if verbose {
            args.push("-v");
        }

        let target_str;
        if let Some(t) = target {
            args.push("-t");
            target_str = t.to_string_lossy().to_string();
            args.push(&target_str);
        }

        let req_str;
        if let Some(r) = requirements {
            args.push("-r");
            req_str = r.to_string_lossy().to_string();
            args.push(&req_str);
        }

        for pkg in packages {
            args.push(pkg);
        }

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: packages.to_vec(),
        })
    }

    /// Execute uninstall command
    fn execute_uninstall(
        &self,
        packages: &[String],
        requirements: Option<&Path>,
        yes: bool,
    ) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip", "uninstall"];
        
        if yes {
            args.push("-y");
        }

        let req_str;
        if let Some(r) = requirements {
            args.push("-r");
            req_str = r.to_string_lossy().to_string();
            args.push(&req_str);
        }

        for pkg in packages {
            args.push(pkg);
        }

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: packages.to_vec(),
        })
    }

    /// Execute list command
    fn execute_list(
        &self,
        outdated: bool,
        uptodate: bool,
        editable: bool,
        format: ListFormat,
    ) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip", "list"];
        
        if outdated {
            args.push("-o");
        }
        if uptodate {
            args.push("-u");
        }
        if editable {
            args.push("-e");
        }
        
        match format {
            ListFormat::Columns => args.push("--format=columns"),
            ListFormat::Freeze => args.push("--format=freeze"),
            ListFormat::Json => args.push("--format=json"),
        }

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: vec![],
        })
    }

    /// Execute show command
    fn execute_show(&self, packages: &[String], files: bool) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip", "show"];
        
        if files {
            args.push("-f");
        }

        for pkg in packages {
            args.push(pkg);
        }

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: packages.to_vec(),
        })
    }

    /// Execute freeze command
    fn execute_freeze(&self, all: bool, local: bool, exclude_editable: bool) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip", "freeze"];
        
        if all {
            args.push("--all");
        }
        if local {
            args.push("-l");
        }
        if exclude_editable {
            args.push("--exclude-editable");
        }

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: vec![],
        })
    }

    /// Execute check command
    fn execute_check(&self) -> Result<PipCommandResult, PipError> {
        let output = Command::new(&self.python_path)
            .args(["-m", "pip", "check"])
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: vec![],
        })
    }

    /// Execute version command
    fn execute_version(&self) -> Result<PipCommandResult, PipError> {
        let output = Command::new(&self.python_path)
            .args(["-m", "pip", "--version"])
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: vec![],
        })
    }

    /// Execute help command
    fn execute_help(&self, command: Option<&str>) -> Result<PipCommandResult, PipError> {
        let mut args = vec!["-m", "pip"];
        
        if let Some(cmd) = command {
            args.push(cmd);
        }
        args.push("--help");

        let output = Command::new(&self.python_path)
            .args(&args)
            .output()?;

        Ok(PipCommandResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            packages_affected: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_install_basic() {
        let args = vec!["install".to_string(), "requests".to_string()];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        
        if let PipCommand::Install { packages, .. } = cmd {
            assert_eq!(packages, vec!["requests"]);
        } else {
            panic!("Expected Install command");
        }
    }

    #[test]
    fn test_parse_install_with_flags() {
        let args = vec![
            "install".to_string(),
            "-U".to_string(),
            "--no-deps".to_string(),
            "requests".to_string(),
        ];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        
        if let PipCommand::Install { packages, upgrade, no_deps, .. } = cmd {
            assert_eq!(packages, vec!["requests"]);
            assert!(upgrade);
            assert!(no_deps);
        } else {
            panic!("Expected Install command");
        }
    }

    #[test]
    fn test_parse_uninstall() {
        let args = vec!["uninstall".to_string(), "-y".to_string(), "requests".to_string()];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        
        if let PipCommand::Uninstall { packages, yes, .. } = cmd {
            assert_eq!(packages, vec!["requests"]);
            assert!(yes);
        } else {
            panic!("Expected Uninstall command");
        }
    }

    #[test]
    fn test_parse_list() {
        let args = vec!["list".to_string(), "--outdated".to_string(), "--format=json".to_string()];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        
        if let PipCommand::List { outdated, format, .. } = cmd {
            assert!(outdated);
            assert_eq!(format, ListFormat::Json);
        } else {
            panic!("Expected List command");
        }
    }

    #[test]
    fn test_parse_freeze() {
        let args = vec!["freeze".to_string(), "--all".to_string()];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        
        if let PipCommand::Freeze { all, .. } = cmd {
            assert!(all);
        } else {
            panic!("Expected Freeze command");
        }
    }

    #[test]
    fn test_parse_version() {
        let args = vec!["--version".to_string()];
        let cmd = PipCompatLayer::parse_args(&args).unwrap();
        assert_eq!(cmd, PipCommand::Version);
    }
}
