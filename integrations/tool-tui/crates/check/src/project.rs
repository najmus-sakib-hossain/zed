//! Project Intelligence
//!
//! Zero-config detection of project type, framework, and conventions.

use crate::framework_detector::FrameworkDetector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Detected project profile
#[derive(Debug, Clone, Default)]
pub struct ProjectProfile {
    /// Detected frameworks
    pub frameworks: Vec<Framework>,
    /// Primary language
    pub language: Language,
    /// Detected style conventions
    pub style: StyleConventions,
    /// Test framework in use
    pub test_framework: Option<TestFramework>,
    /// Monorepo information
    pub monorepo: Option<MonorepoInfo>,
    /// Import aliases from tsconfig/jsconfig
    pub import_aliases: HashMap<String, String>,
    /// Package manager in use
    pub package_manager: PackageManager,
}

/// Detected frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Framework {
    React,
    Next,
    Vue,
    Nuxt,
    Svelte,
    SvelteKit,
    Angular,
    Solid,
    Qwik,
    Remix,
    Astro,
    Express,
    Fastify,
    Hono,
    Nest,
}

impl Framework {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::React => "React",
            Self::Next => "Next.js",
            Self::Vue => "Vue",
            Self::Nuxt => "Nuxt",
            Self::Svelte => "Svelte",
            Self::SvelteKit => "SvelteKit",
            Self::Angular => "Angular",
            Self::Solid => "Solid",
            Self::Qwik => "Qwik",
            Self::Remix => "Remix",
            Self::Astro => "Astro",
            Self::Express => "Express",
            Self::Fastify => "Fastify",
            Self::Hono => "Hono",
            Self::Nest => "NestJS",
        }
    }
}

/// Primary language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    JavaScript,
    TypeScript,
}

/// Detected test framework
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFramework {
    Jest,
    Vitest,
    Mocha,
    Ava,
    Playwright,
    Cypress,
}

/// Package manager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageManager {
    #[default]
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

/// Monorepo information
#[derive(Debug, Clone)]
pub struct MonorepoInfo {
    /// Type of monorepo
    pub kind: MonorepoKind,
    /// Package locations
    pub packages: Vec<PackageInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonorepoKind {
    PnpmWorkspace,
    YarnWorkspace,
    NpmWorkspace,
    Turborepo,
    Nx,
    Lerna,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub path: PathBuf,
}

/// Inferred style conventions
#[derive(Debug, Clone)]
pub struct StyleConventions {
    /// Use semicolons
    pub semicolons: bool,
    /// Quote style
    pub quotes: QuoteStyle,
    /// Indentation
    pub indent: IndentStyle,
    /// Trailing commas
    pub trailing_commas: TrailingCommas,
}

impl Default for StyleConventions {
    fn default() -> Self {
        Self {
            semicolons: true,
            quotes: QuoteStyle::Double,
            indent: IndentStyle::Spaces(2),
            trailing_commas: TrailingCommas::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailingCommas {
    None,
    Es5,
    All,
}

impl ProjectProfile {
    /// Auto-detect project profile from root directory
    #[must_use]
    pub fn detect(root: &Path) -> Self {
        let mut profile = Self::default();

        // Detect package manager
        profile.package_manager = detect_package_manager(root);

        // Use FrameworkDetector for framework detection
        let mut detector = FrameworkDetector::new();
        let detection = detector.detect(root);
        profile.frameworks = detection.frameworks;

        // Read package.json for additional information
        if let Some(pkg) = read_package_json(root) {
            profile.detect_from_package_json(&pkg);
        }

        // Read tsconfig.json
        if let Some(tsconfig) = read_tsconfig(root) {
            profile.language = Language::TypeScript;
            profile.detect_from_tsconfig(&tsconfig);
        }

        // Detect monorepo
        profile.monorepo = detect_monorepo(root);

        // If in a monorepo, detect frameworks for each package
        if let Some(ref monorepo) = profile.monorepo {
            let monorepo_detections = detector.detect_monorepo(monorepo);
            // Collect all unique frameworks from monorepo packages
            for (_, detection) in monorepo_detections {
                for framework in detection.frameworks {
                    if !profile.frameworks.contains(&framework) {
                        profile.frameworks.push(framework);
                    }
                }
            }
        }

        // Infer style conventions from existing code
        profile.style = infer_style_conventions(root);

        profile
    }

    fn detect_from_package_json(&mut self, pkg: &PackageJson) {
        let deps = pkg.all_dependencies();

        // Test framework detection
        if deps.contains("vitest") {
            self.test_framework = Some(TestFramework::Vitest);
        } else if deps.contains("jest") {
            self.test_framework = Some(TestFramework::Jest);
        } else if deps.contains("mocha") {
            self.test_framework = Some(TestFramework::Mocha);
        } else if deps.contains("ava") {
            self.test_framework = Some(TestFramework::Ava);
        } else if deps.contains("@playwright/test") {
            self.test_framework = Some(TestFramework::Playwright);
        } else if deps.contains("cypress") {
            self.test_framework = Some(TestFramework::Cypress);
        }

        // Language detection
        if deps.contains("typescript") {
            self.language = Language::TypeScript;
        }
    }

    fn detect_from_tsconfig(&mut self, tsconfig: &TsConfig) {
        // Extract path aliases
        if let Some(ref paths) = tsconfig.compiler_options.paths {
            for (alias, targets) in paths {
                if let Some(target) = targets.first() {
                    let alias = alias.trim_end_matches("/*").to_string();
                    let target = target.trim_end_matches("/*").to_string();
                    self.import_aliases.insert(alias, target);
                }
            }
        }
    }

    /// Get a summary string for display
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if !self.frameworks.is_empty() {
            let frameworks: Vec<_> = self.frameworks.iter().map(Framework::as_str).collect();
            parts.push(format!("Frameworks: {}", frameworks.join(", ")));
        }

        parts.push(format!(
            "Language: {}",
            match self.language {
                Language::JavaScript => "JavaScript",
                Language::TypeScript => "TypeScript",
            }
        ));

        if let Some(ref test) = self.test_framework {
            parts.push(format!(
                "Test Runner: {}",
                match test {
                    TestFramework::Jest => "Jest",
                    TestFramework::Vitest => "Vitest",
                    TestFramework::Mocha => "Mocha",
                    TestFramework::Ava => "AVA",
                    TestFramework::Playwright => "Playwright",
                    TestFramework::Cypress => "Cypress",
                }
            ));
        }

        if let Some(ref mono) = self.monorepo {
            parts.push(format!(
                "Monorepo: {} ({} packages)",
                match mono.kind {
                    MonorepoKind::PnpmWorkspace => "pnpm",
                    MonorepoKind::YarnWorkspace => "yarn",
                    MonorepoKind::NpmWorkspace => "npm",
                    MonorepoKind::Turborepo => "Turborepo",
                    MonorepoKind::Nx => "Nx",
                    MonorepoKind::Lerna => "Lerna",
                },
                mono.packages.len()
            ));
        }

        parts.join("\n")
    }
}

// Helper types for parsing

#[derive(Debug, Deserialize)]
struct PackageJson {
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: HashMap<String, String>,
    #[serde(default)]
    workspaces: Option<Workspaces>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Workspaces {
    Array(Vec<String>),
    Object { packages: Vec<String> },
}

impl PackageJson {
    fn all_dependencies(&self) -> std::collections::HashSet<&str> {
        self.dependencies
            .keys()
            .chain(self.dev_dependencies.keys())
            .chain(self.peer_dependencies.keys())
            .map(std::string::String::as_str)
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct TsConfig {
    #[serde(default, rename = "compilerOptions")]
    compiler_options: CompilerOptions,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct CompilerOptions {
    paths: Option<HashMap<String, Vec<String>>>,
    strict: Option<bool>,
}

// Helper functions

fn read_package_json(root: &Path) -> Option<PackageJson> {
    let path = root.join("package.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn read_tsconfig(root: &Path) -> Option<TsConfig> {
    let path = root.join("tsconfig.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn detect_package_manager(root: &Path) -> PackageManager {
    if root.join("pnpm-lock.yaml").exists() {
        PackageManager::Pnpm
    } else if root.join("yarn.lock").exists() {
        PackageManager::Yarn
    } else if root.join("bun.lockb").exists() {
        PackageManager::Bun
    } else {
        PackageManager::Npm
    }
}

fn detect_monorepo(root: &Path) -> Option<MonorepoInfo> {
    // Check for pnpm workspace
    if root.join("pnpm-workspace.yaml").exists() {
        return Some(MonorepoInfo {
            kind: MonorepoKind::PnpmWorkspace,
            packages: find_workspace_packages(root),
        });
    }

    // Check for package.json workspaces
    if let Some(pkg) = read_package_json(root)
        && pkg.workspaces.is_some()
    {
        let kind = if root.join("yarn.lock").exists() {
            MonorepoKind::YarnWorkspace
        } else {
            MonorepoKind::NpmWorkspace
        };
        return Some(MonorepoInfo {
            kind,
            packages: find_workspace_packages(root),
        });
    }

    // Check for Turborepo
    if root.join("turbo.json").exists() {
        return Some(MonorepoInfo {
            kind: MonorepoKind::Turborepo,
            packages: find_workspace_packages(root),
        });
    }

    // Check for Nx
    if root.join("nx.json").exists() {
        return Some(MonorepoInfo {
            kind: MonorepoKind::Nx,
            packages: find_workspace_packages(root),
        });
    }

    // Check for Lerna
    if root.join("lerna.json").exists() {
        return Some(MonorepoInfo {
            kind: MonorepoKind::Lerna,
            packages: find_workspace_packages(root),
        });
    }

    None
}

fn find_workspace_packages(root: &Path) -> Vec<PackageInfo> {
    let mut packages = Vec::new();

    // Check common package directories
    for dir in ["packages", "apps", "libs"] {
        let packages_dir = root.join(dir);
        if packages_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(packages_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && path.join("package.json").exists()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    packages.push(PackageInfo {
                        name: name.to_string(),
                        path,
                    });
                }
            }
        }
    }

    packages
}

fn infer_style_conventions(root: &Path) -> StyleConventions {
    let mut conventions = StyleConventions::default();

    // Sample a few files to infer style
    let sample_files = find_sample_files(root, 5);
    if sample_files.is_empty() {
        return conventions;
    }

    let mut semicolons = 0;
    let mut no_semicolons = 0;
    let mut single_quotes = 0;
    let mut double_quotes = 0;
    let mut tabs = 0;
    let mut spaces = 0;

    for file in sample_files {
        if let Ok(content) = std::fs::read_to_string(&file) {
            // Semicolons
            let lines: Vec<_> = content.lines().collect();
            for line in &lines {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                    if trimmed.ends_with(';') {
                        semicolons += 1;
                    } else if trimmed.ends_with('{')
                        || trimmed.ends_with('}')
                        || trimmed.ends_with(',')
                    {
                        // Ignore structural endings
                    } else {
                        no_semicolons += 1;
                    }
                }
            }

            // Quotes (simple heuristic)
            let single = content.matches('\'').count();
            let double = content.matches('"').count();
            if single > double {
                single_quotes += 1;
            } else {
                double_quotes += 1;
            }

            // Indentation
            for line in &lines {
                if line.starts_with('\t') {
                    tabs += 1;
                } else if line.starts_with("  ") {
                    spaces += 1;
                }
            }
        }
    }

    conventions.semicolons = semicolons > no_semicolons;
    conventions.quotes = if single_quotes > double_quotes {
        QuoteStyle::Single
    } else {
        QuoteStyle::Double
    };
    conventions.indent = if tabs > spaces {
        IndentStyle::Tabs
    } else {
        IndentStyle::Spaces(2) // Default to 2 spaces
    };

    conventions
}

fn find_sample_files(root: &Path, count: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Look for JS/TS files
    for entry in walkdir::WalkDir::new(root).max_depth(4).into_iter().flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Skip node_modules and other common directories
        if path
            .components()
            .any(|c| c.as_os_str() == "node_modules" || c.as_os_str() == ".git")
        {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "js" | "jsx" | "ts" | "tsx") {
            files.push(path.to_path_buf());
            if files.len() >= count {
                break;
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let profile = ProjectProfile::default();
        assert!(profile.frameworks.is_empty());
        assert_eq!(profile.language, Language::JavaScript);
    }
}
