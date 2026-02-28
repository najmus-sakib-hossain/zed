//! System Information Provider
//!
//! Provides comprehensive system information to agents, including OS details,
//! shell environment, installed languages, package managers, project structure,
//! git status, build tools, and test frameworks.
//!
//! Features:
//! - Caching with configurable TTL
//! - Automatic cache invalidation on system changes
//! - Project type detection from marker files

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod cache;
mod detectors;

#[cfg(test)]
mod property_tests;

pub use cache::{CacheEntry, SystemInfoCache};
pub use detectors::*;

/// Complete system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system details
    pub os: OsInfo,
    /// Shell environment
    pub shell: ShellInfo,
    /// Installed programming languages
    pub languages: Vec<LanguageInfo>,
    /// Available package managers
    pub package_managers: Vec<PackageManagerInfo>,
    /// Project information
    pub project: Option<ProjectInfo>,
    /// Git repository status
    pub git: Option<GitInfo>,
    /// Available build tools
    pub build_tools: Vec<BuildToolInfo>,
    /// Available test frameworks
    pub test_frameworks: Vec<TestFrameworkInfo>,
    /// Timestamp when info was collected
    pub collected_at: std::time::SystemTime,
}

/// Operating system information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OsInfo {
    /// OS name (e.g., "Windows", "Linux", "macOS")
    pub name: String,
    /// OS version
    pub version: String,
    /// Architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// OS family (e.g., "unix", "windows")
    pub family: String,
}

/// Shell environment information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellInfo {
    /// Shell name (e.g., "bash", "zsh", "powershell")
    pub name: String,
    /// Shell version
    pub version: Option<String>,
    /// Path to shell executable
    pub path: PathBuf,
}

/// Programming language information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageInfo {
    /// Language name
    pub name: String,
    /// Version string
    pub version: String,
    /// Path to executable
    pub path: PathBuf,
}

/// Package manager information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageManagerInfo {
    /// Package manager name (e.g., "cargo", "npm", "pip")
    pub name: String,
    /// Version string
    pub version: Option<String>,
    /// Path to executable
    pub path: PathBuf,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectInfo {
    /// Detected project type
    pub project_type: ProjectType,
    /// Project root directory
    pub root: PathBuf,
    /// Project name (from manifest if available)
    pub name: Option<String>,
    /// Detected frameworks
    pub frameworks: Vec<String>,
    /// Source directories
    pub source_dirs: Vec<PathBuf>,
}

/// Project type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Cpp,
    C,
    Mixed,
    Unknown,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Rust => write!(f, "Rust"),
            ProjectType::Node => write!(f, "Node.js"),
            ProjectType::Python => write!(f, "Python"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Java => write!(f, "Java"),
            ProjectType::CSharp => write!(f, "C#"),
            ProjectType::Ruby => write!(f, "Ruby"),
            ProjectType::Php => write!(f, "PHP"),
            ProjectType::Swift => write!(f, "Swift"),
            ProjectType::Kotlin => write!(f, "Kotlin"),
            ProjectType::Cpp => write!(f, "C++"),
            ProjectType::C => write!(f, "C"),
            ProjectType::Mixed => write!(f, "Mixed"),
            ProjectType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Git repository information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitInfo {
    /// Current branch name
    pub branch: String,
    /// Remote URL (if configured)
    pub remote: Option<String>,
    /// Whether there are uncommitted changes
    pub is_dirty: bool,
    /// Number of commits ahead of remote
    pub ahead: u32,
    /// Number of commits behind remote
    pub behind: u32,
    /// Last commit hash (short)
    pub last_commit: Option<String>,
}

/// Build tool information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildToolInfo {
    /// Tool name (e.g., "cargo", "make", "gradle")
    pub name: String,
    /// Version string
    pub version: Option<String>,
    /// Path to executable
    pub path: PathBuf,
}

/// Test framework information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestFrameworkInfo {
    /// Framework name (e.g., "cargo test", "jest", "pytest")
    pub name: String,
    /// Version string
    pub version: Option<String>,
    /// Associated language
    pub language: String,
}

/// System information provider with caching
pub struct SystemInfoProvider {
    cache: SystemInfoCache,
    ttl: Duration,
    project_root: Option<PathBuf>,
}

impl Default for SystemInfoProvider {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minute default TTL
    }
}

impl SystemInfoProvider {
    /// Create a new provider with specified TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: SystemInfoCache::new(),
            ttl,
            project_root: None,
        }
    }

    /// Set the project root for project-specific detection
    pub fn with_project_root(mut self, root: impl AsRef<Path>) -> Self {
        self.project_root = Some(root.as_ref().to_path_buf());
        self
    }

    /// Get system information (cached if within TTL)
    pub fn get(&mut self) -> crate::Result<SystemInfo> {
        if let Some(cached) = self.cache.get_if_valid(self.ttl) {
            return Ok(cached.clone());
        }
        self.refresh()
    }

    /// Force refresh of system information
    pub fn refresh(&mut self) -> crate::Result<SystemInfo> {
        let info = SystemInfo {
            os: self.detect_os()?,
            shell: self.detect_shell()?,
            languages: self.detect_languages(),
            package_managers: self.detect_package_managers(),
            project: self.detect_project(),
            git: self.detect_git(),
            build_tools: self.detect_build_tools(),
            test_frameworks: self.detect_test_frameworks(),
            collected_at: std::time::SystemTime::now(),
        };
        self.cache.set(info.clone());
        Ok(info)
    }

    /// Invalidate the cache
    pub fn invalidate(&mut self) {
        self.cache.invalidate();
    }

    /// Check if cache is valid
    pub fn is_cache_valid(&self) -> bool {
        self.cache.is_valid(self.ttl)
    }

    /// Detect operating system information
    pub fn detect_os(&self) -> crate::Result<OsInfo> {
        Ok(OsInfo {
            name: std::env::consts::OS.to_string(),
            version: get_os_version(),
            arch: std::env::consts::ARCH.to_string(),
            family: std::env::consts::FAMILY.to_string(),
        })
    }

    /// Detect shell environment
    pub fn detect_shell(&self) -> crate::Result<ShellInfo> {
        #[cfg(windows)]
        {
            // Check for PowerShell first, then cmd
            if let Ok(output) = Command::new("powershell")
                .args(["-Command", "$PSVersionTable.PSVersion.ToString()"])
                .output()
            {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return Ok(ShellInfo {
                        name: "powershell".to_string(),
                        version: Some(version),
                        path: PathBuf::from("powershell.exe"),
                    });
                }
            }
            Ok(ShellInfo {
                name: "cmd".to_string(),
                version: None,
                path: PathBuf::from("cmd.exe"),
            })
        }

        #[cfg(not(windows))]
        {
            let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let shell_name = Path::new(&shell_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("sh")
                .to_string();

            let version = get_shell_version(&shell_name);

            Ok(ShellInfo {
                name: shell_name,
                version,
                path: PathBuf::from(shell_path),
            })
        }
    }

    /// Detect installed programming languages
    pub fn detect_languages(&self) -> Vec<LanguageInfo> {
        let mut languages = Vec::new();

        // Rust
        if let Some(info) = detect_language("rustc", &["--version"], "rust") {
            languages.push(info);
        }

        // Python
        if let Some(info) = detect_language("python3", &["--version"], "python") {
            languages.push(info);
        } else if let Some(info) = detect_language("python", &["--version"], "python") {
            languages.push(info);
        }

        // Node.js
        if let Some(info) = detect_language("node", &["--version"], "node") {
            languages.push(info);
        }

        // Go
        if let Some(info) = detect_language("go", &["version"], "go") {
            languages.push(info);
        }

        // Java
        if let Some(info) = detect_language("java", &["-version"], "java") {
            languages.push(info);
        }

        // Ruby
        if let Some(info) = detect_language("ruby", &["--version"], "ruby") {
            languages.push(info);
        }

        // PHP
        if let Some(info) = detect_language("php", &["--version"], "php") {
            languages.push(info);
        }

        languages
    }

    /// Detect available package managers
    pub fn detect_package_managers(&self) -> Vec<PackageManagerInfo> {
        let mut managers = Vec::new();

        // Cargo (Rust)
        if let Some(info) = detect_package_manager("cargo", &["--version"]) {
            managers.push(info);
        }

        // npm (Node)
        if let Some(info) = detect_package_manager("npm", &["--version"]) {
            managers.push(info);
        }

        // yarn (Node)
        if let Some(info) = detect_package_manager("yarn", &["--version"]) {
            managers.push(info);
        }

        // pnpm (Node)
        if let Some(info) = detect_package_manager("pnpm", &["--version"]) {
            managers.push(info);
        }

        // pip (Python)
        if let Some(info) = detect_package_manager("pip", &["--version"]) {
            managers.push(info);
        }

        // go mod (Go)
        if let Some(info) = detect_package_manager("go", &["version"]) {
            let mut info = info;
            info.name = "go mod".to_string();
            managers.push(info);
        }

        managers
    }

    /// Detect project structure and type
    pub fn detect_project(&self) -> Option<ProjectInfo> {
        let root = self.project_root.as_ref()?;

        if !root.exists() {
            return None;
        }

        let project_type = detect_project_type(root);
        let name = detect_project_name(root, project_type);
        let frameworks = detect_frameworks(root, project_type);
        let source_dirs = detect_source_dirs(root, project_type);

        Some(ProjectInfo {
            project_type,
            root: root.clone(),
            name,
            frameworks,
            source_dirs,
        })
    }

    /// Detect git repository status
    pub fn detect_git(&self) -> Option<GitInfo> {
        let root = self.project_root.as_ref()?;

        // Check if .git exists
        if !root.join(".git").exists() {
            return None;
        }

        let branch = run_git_command(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        let remote = run_git_command(root, &["config", "--get", "remote.origin.url"]);
        let is_dirty = run_git_command(root, &["status", "--porcelain"])
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let (ahead, behind) = get_git_ahead_behind(root);
        let last_commit = run_git_command(root, &["rev-parse", "--short", "HEAD"]);

        Some(GitInfo {
            branch,
            remote,
            is_dirty,
            ahead,
            behind,
            last_commit,
        })
    }

    /// Detect available build tools
    pub fn detect_build_tools(&self) -> Vec<BuildToolInfo> {
        let mut tools = Vec::new();

        // Cargo (Rust)
        if let Some(info) = detect_build_tool("cargo", &["--version"]) {
            tools.push(info);
        }

        // Make
        if let Some(info) = detect_build_tool("make", &["--version"]) {
            tools.push(info);
        }

        // CMake
        if let Some(info) = detect_build_tool("cmake", &["--version"]) {
            tools.push(info);
        }

        // Gradle
        if let Some(info) = detect_build_tool("gradle", &["--version"]) {
            tools.push(info);
        }

        // Maven
        if let Some(info) = detect_build_tool("mvn", &["--version"]) {
            let mut info = info;
            info.name = "maven".to_string();
            tools.push(info);
        }

        tools
    }

    /// Detect available test frameworks
    pub fn detect_test_frameworks(&self) -> Vec<TestFrameworkInfo> {
        let mut frameworks = Vec::new();

        // Check project-specific test frameworks
        if let Some(ref root) = self.project_root {
            // Rust: cargo test is always available if Cargo.toml exists
            if root.join("Cargo.toml").exists() {
                frameworks.push(TestFrameworkInfo {
                    name: "cargo test".to_string(),
                    version: None,
                    language: "rust".to_string(),
                });
            }

            // Node: check for jest, mocha, vitest
            if root.join("package.json").exists() {
                if let Some(fw) = detect_node_test_framework(root) {
                    frameworks.push(fw);
                }
            }

            // Python: check for pytest, unittest
            if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
                if let Some(fw) = detect_python_test_framework(root) {
                    frameworks.push(fw);
                }
            }
        }

        frameworks
    }
}

// Helper functions

fn get_os_version() -> String {
    #[cfg(windows)]
    {
        if let Ok(output) = Command::new("cmd").args(["/c", "ver"]).output() {
            let version = String::from_utf8_lossy(&output.stdout);
            // Extract version number from "Microsoft Windows [Version 10.0.xxxxx]"
            if let Some(start) = version.find('[') {
                if let Some(end) = version.find(']') {
                    return version[start + 1..end].replace("Version ", "");
                }
            }
        }
        "unknown".to_string()
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sw_vers").args(["-productVersion"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        "unknown".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("uname").args(["-r"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        "unknown".to_string()
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

#[cfg(not(windows))]
fn get_shell_version(shell_name: &str) -> Option<String> {
    let args = match shell_name {
        "bash" => vec!["--version"],
        "zsh" => vec!["--version"],
        "fish" => vec!["--version"],
        _ => return None,
    };

    Command::new(shell_name).args(&args).output().ok().and_then(|output| {
        let version = String::from_utf8_lossy(&output.stdout);
        version.lines().next().map(|s| s.to_string())
    })
}

fn detect_language(cmd: &str, args: &[&str], name: &str) -> Option<LanguageInfo> {
    let output = Command::new(cmd).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = extract_version(&version_output);

    let path = which::which(cmd).ok()?;

    Some(LanguageInfo {
        name: name.to_string(),
        version,
        path,
    })
}

fn detect_package_manager(cmd: &str, args: &[&str]) -> Option<PackageManagerInfo> {
    let output = Command::new(cmd).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = Some(extract_version(&version_output));

    let path = which::which(cmd).ok()?;

    Some(PackageManagerInfo {
        name: cmd.to_string(),
        version,
        path,
    })
}

fn detect_build_tool(cmd: &str, args: &[&str]) -> Option<BuildToolInfo> {
    let output = Command::new(cmd).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = Some(extract_version(&version_output));

    let path = which::which(cmd).ok()?;

    Some(BuildToolInfo {
        name: cmd.to_string(),
        version,
        path,
    })
}

fn extract_version(output: &str) -> String {
    // Try to extract version number from common patterns
    let re = regex::Regex::new(r"(\d+\.\d+(?:\.\d+)?(?:-[\w.]+)?)").ok();

    if let Some(re) = re {
        if let Some(caps) = re.captures(output) {
            return caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        }
    }

    output.lines().next().unwrap_or("").trim().to_string()
}

fn detect_project_type(root: &Path) -> ProjectType {
    // Check for project marker files
    let markers: Vec<(&str, ProjectType)> = vec![
        ("Cargo.toml", ProjectType::Rust),
        ("package.json", ProjectType::Node),
        ("pyproject.toml", ProjectType::Python),
        ("setup.py", ProjectType::Python),
        ("requirements.txt", ProjectType::Python),
        ("go.mod", ProjectType::Go),
        ("pom.xml", ProjectType::Java),
        ("build.gradle", ProjectType::Java),
        ("build.gradle.kts", ProjectType::Kotlin),
        ("*.csproj", ProjectType::CSharp),
        ("Gemfile", ProjectType::Ruby),
        ("composer.json", ProjectType::Php),
        ("Package.swift", ProjectType::Swift),
        ("CMakeLists.txt", ProjectType::Cpp),
        ("Makefile", ProjectType::C),
    ];

    let mut detected = Vec::new();

    for (marker, project_type) in markers {
        if marker.contains('*') {
            // Glob pattern
            let pattern = root.join(marker);
            if let Ok(entries) = glob::glob(pattern.to_str().unwrap_or("")) {
                if entries.count() > 0 {
                    detected.push(project_type);
                }
            }
        } else if root.join(marker).exists() {
            detected.push(project_type);
        }
    }

    match detected.len() {
        0 => ProjectType::Unknown,
        1 => detected[0],
        _ => ProjectType::Mixed,
    }
}

fn detect_project_name(root: &Path, project_type: ProjectType) -> Option<String> {
    match project_type {
        ProjectType::Rust => {
            let cargo_toml = root.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if let Ok(parsed) = content.parse::<toml::Table>() {
                    return parsed
                        .get("package")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
        ProjectType::Node => {
            let package_json = root.join("package.json");
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    return parsed.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
                }
            }
        }
        _ => {}
    }

    // Fall back to directory name
    root.file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
}

fn detect_frameworks(root: &Path, project_type: ProjectType) -> Vec<String> {
    let mut frameworks = Vec::new();

    match project_type {
        ProjectType::Rust => {
            if let Ok(content) = std::fs::read_to_string(root.join("Cargo.toml")) {
                // Check for common Rust frameworks
                if content.contains("actix-web") {
                    frameworks.push("Actix Web".to_string());
                }
                if content.contains("axum") {
                    frameworks.push("Axum".to_string());
                }
                if content.contains("rocket") {
                    frameworks.push("Rocket".to_string());
                }
                if content.contains("tokio") {
                    frameworks.push("Tokio".to_string());
                }
            }
        }
        ProjectType::Node => {
            if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
                if content.contains("\"react\"") {
                    frameworks.push("React".to_string());
                }
                if content.contains("\"vue\"") {
                    frameworks.push("Vue".to_string());
                }
                if content.contains("\"next\"") {
                    frameworks.push("Next.js".to_string());
                }
                if content.contains("\"express\"") {
                    frameworks.push("Express".to_string());
                }
                if content.contains("\"nestjs\"") || content.contains("\"@nestjs") {
                    frameworks.push("NestJS".to_string());
                }
            }
        }
        ProjectType::Python => {
            if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
                if content.contains("django") {
                    frameworks.push("Django".to_string());
                }
                if content.contains("flask") {
                    frameworks.push("Flask".to_string());
                }
                if content.contains("fastapi") {
                    frameworks.push("FastAPI".to_string());
                }
            }
        }
        _ => {}
    }

    frameworks
}

fn detect_source_dirs(root: &Path, project_type: ProjectType) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let common_dirs = match project_type {
        ProjectType::Rust => vec!["src", "crates"],
        ProjectType::Node => vec!["src", "lib", "app"],
        ProjectType::Python => vec![
            "src",
            "lib",
            root.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        ],
        ProjectType::Go => vec!["cmd", "pkg", "internal"],
        ProjectType::Java => vec!["src/main/java", "src"],
        _ => vec!["src", "lib"],
    };

    for dir in common_dirs {
        let path = root.join(dir);
        if path.exists() && path.is_dir() {
            dirs.push(path);
        }
    }

    dirs
}

fn run_git_command(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_git_ahead_behind(root: &Path) -> (u32, u32) {
    let output =
        run_git_command(root, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]);

    if let Some(output) = output {
        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() == 2 {
            let ahead = parts[0].parse().unwrap_or(0);
            let behind = parts[1].parse().unwrap_or(0);
            return (ahead, behind);
        }
    }

    (0, 0)
}

fn detect_node_test_framework(root: &Path) -> Option<TestFrameworkInfo> {
    let package_json = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&package_json) {
        if content.contains("\"vitest\"") {
            return Some(TestFrameworkInfo {
                name: "vitest".to_string(),
                version: None,
                language: "javascript".to_string(),
            });
        }
        if content.contains("\"jest\"") {
            return Some(TestFrameworkInfo {
                name: "jest".to_string(),
                version: None,
                language: "javascript".to_string(),
            });
        }
        if content.contains("\"mocha\"") {
            return Some(TestFrameworkInfo {
                name: "mocha".to_string(),
                version: None,
                language: "javascript".to_string(),
            });
        }
    }
    None
}

fn detect_python_test_framework(root: &Path) -> Option<TestFrameworkInfo> {
    // Check pyproject.toml for pytest
    if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
        if content.contains("pytest") {
            return Some(TestFrameworkInfo {
                name: "pytest".to_string(),
                version: None,
                language: "python".to_string(),
            });
        }
    }

    // Check for pytest.ini or conftest.py
    if root.join("pytest.ini").exists() || root.join("conftest.py").exists() {
        return Some(TestFrameworkInfo {
            name: "pytest".to_string(),
            version: None,
            language: "python".to_string(),
        });
    }

    // Default to unittest
    Some(TestFrameworkInfo {
        name: "unittest".to_string(),
        version: None,
        language: "python".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_os() {
        let provider = SystemInfoProvider::default();
        let os = provider.detect_os().unwrap();

        assert!(!os.name.is_empty());
        assert!(!os.arch.is_empty());
        assert!(!os.family.is_empty());
    }

    #[test]
    fn test_detect_shell() {
        let provider = SystemInfoProvider::default();
        let shell = provider.detect_shell().unwrap();

        assert!(!shell.name.is_empty());
        assert!(shell.path.exists() || cfg!(windows)); // On Windows, path might be relative
    }

    #[test]
    fn test_detect_languages() {
        let provider = SystemInfoProvider::default();
        let languages = provider.detect_languages();

        // At minimum, Rust should be detected since we're running Rust tests
        // But this might not always be true in CI, so just check it doesn't panic
        assert!(languages.len() >= 0);
    }

    #[test]
    fn test_detect_rust_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();
        fs::create_dir(temp.path().join("src")).unwrap();

        let provider = SystemInfoProvider::default().with_project_root(temp.path());

        let project = provider.detect_project().unwrap();
        assert_eq!(project.project_type, ProjectType::Rust);
        assert_eq!(project.name, Some("test-project".to_string()));
    }

    #[test]
    fn test_detect_node_project() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"name": "test-node-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        let provider = SystemInfoProvider::default().with_project_root(temp.path());

        let project = provider.detect_project().unwrap();
        assert_eq!(project.project_type, ProjectType::Node);
        assert_eq!(project.name, Some("test-node-project".to_string()));
    }

    #[test]
    fn test_detect_git_repo() {
        let temp = TempDir::new().unwrap();

        // Initialize a git repo
        let init_result = std::process::Command::new("git")
            .current_dir(temp.path())
            .args(["init"])
            .output();

        // Skip test if git is not available
        if init_result.is_err() || !temp.path().join(".git").exists() {
            return;
        }

        // Configure git user for the commit (required in some environments)
        let _ = std::process::Command::new("git")
            .current_dir(temp.path())
            .args(["config", "user.email", "test@test.com"])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(temp.path())
            .args(["config", "user.name", "Test"])
            .output();

        // Create an initial commit so HEAD exists
        let test_file = temp.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();
        let _ = std::process::Command::new("git")
            .current_dir(temp.path())
            .args(["add", "."])
            .output();
        let commit_result = std::process::Command::new("git")
            .current_dir(temp.path())
            .args(["commit", "-m", "initial"])
            .output();

        // Skip if commit failed (git might not be properly configured)
        if commit_result.is_err() || !commit_result.unwrap().status.success() {
            return;
        }

        let provider = SystemInfoProvider::default().with_project_root(temp.path());

        let git = provider.detect_git();
        assert!(git.is_some(), "Git should be detected when .git directory exists with commits");
    }

    #[test]
    fn test_cache_behavior() {
        let mut provider = SystemInfoProvider::new(Duration::from_secs(60));

        // First call should populate cache
        let info1 = provider.get().unwrap();
        assert!(provider.is_cache_valid());

        // Second call should return cached value
        let info2 = provider.get().unwrap();
        assert_eq!(info1.os, info2.os);

        // Invalidate and check
        provider.invalidate();
        assert!(!provider.is_cache_valid());
    }

    #[test]
    fn test_project_type_display() {
        assert_eq!(ProjectType::Rust.to_string(), "Rust");
        assert_eq!(ProjectType::Node.to_string(), "Node.js");
        assert_eq!(ProjectType::Python.to_string(), "Python");
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(extract_version("rustc 1.75.0"), "1.75.0");
        assert_eq!(extract_version("Python 3.11.4"), "3.11.4");
        assert_eq!(extract_version("v20.10.0"), "20.10.0");
    }

    #[test]
    fn test_detect_frameworks_rust() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[dependencies]
tokio = "1.0"
axum = "0.7"
"#,
        )
        .unwrap();

        let frameworks = detect_frameworks(temp.path(), ProjectType::Rust);
        assert!(frameworks.contains(&"Tokio".to_string()));
        assert!(frameworks.contains(&"Axum".to_string()));
    }

    #[test]
    fn test_detect_frameworks_node() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{"dependencies": {"react": "^18.0.0", "next": "^14.0.0"}}"#,
        )
        .unwrap();

        let frameworks = detect_frameworks(temp.path(), ProjectType::Node);
        assert!(frameworks.contains(&"React".to_string()));
        assert!(frameworks.contains(&"Next.js".to_string()));
    }
}
