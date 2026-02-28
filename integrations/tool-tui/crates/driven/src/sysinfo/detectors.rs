//! Detection utilities for system information
//!
//! This module provides additional detection utilities that can be used
//! independently of the main SystemInfoProvider.

use super::ProjectType;
use std::path::Path;

/// Detect project type from a directory
pub fn detect_project_type_from_path(path: &Path) -> ProjectType {
    super::detect_project_type(path)
}

/// Check if a path is a Rust project
pub fn is_rust_project(path: &Path) -> bool {
    path.join("Cargo.toml").exists()
}

/// Check if a path is a Node.js project
pub fn is_node_project(path: &Path) -> bool {
    path.join("package.json").exists()
}

/// Check if a path is a Python project
pub fn is_python_project(path: &Path) -> bool {
    path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("requirements.txt").exists()
}

/// Check if a path is a Go project
pub fn is_go_project(path: &Path) -> bool {
    path.join("go.mod").exists()
}

/// Check if a path is a Java project
pub fn is_java_project(path: &Path) -> bool {
    path.join("pom.xml").exists() || path.join("build.gradle").exists()
}

/// Check if a path has a git repository
pub fn has_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Get all project marker files for a given project type
pub fn get_project_markers(project_type: ProjectType) -> Vec<&'static str> {
    match project_type {
        ProjectType::Rust => vec!["Cargo.toml", "Cargo.lock"],
        ProjectType::Node => vec![
            "package.json",
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
        ],
        ProjectType::Python => vec!["pyproject.toml", "setup.py", "requirements.txt", "Pipfile"],
        ProjectType::Go => vec!["go.mod", "go.sum"],
        ProjectType::Java => vec!["pom.xml", "build.gradle", "build.gradle.kts"],
        ProjectType::CSharp => vec!["*.csproj", "*.sln"],
        ProjectType::Ruby => vec!["Gemfile", "Gemfile.lock"],
        ProjectType::Php => vec!["composer.json", "composer.lock"],
        ProjectType::Swift => vec!["Package.swift"],
        ProjectType::Kotlin => vec!["build.gradle.kts"],
        ProjectType::Cpp => vec!["CMakeLists.txt", "*.cpp", "*.hpp"],
        ProjectType::C => vec!["Makefile", "*.c", "*.h"],
        ProjectType::Mixed | ProjectType::Unknown => vec![],
    }
}

/// Detect the primary language from file extensions in a directory
pub fn detect_primary_language(path: &Path) -> Option<ProjectType> {
    use std::collections::HashMap;

    let mut counts: HashMap<ProjectType, usize> = HashMap::new();

    let entries = walkdir::WalkDir::new(path).max_depth(3).into_iter().filter_map(|e| e.ok());

    for entry in entries {
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            let project_type = match ext {
                "rs" => Some(ProjectType::Rust),
                "js" | "ts" | "jsx" | "tsx" => Some(ProjectType::Node),
                "py" => Some(ProjectType::Python),
                "go" => Some(ProjectType::Go),
                "java" => Some(ProjectType::Java),
                "cs" => Some(ProjectType::CSharp),
                "rb" => Some(ProjectType::Ruby),
                "php" => Some(ProjectType::Php),
                "swift" => Some(ProjectType::Swift),
                "kt" | "kts" => Some(ProjectType::Kotlin),
                "cpp" | "cc" | "cxx" | "hpp" => Some(ProjectType::Cpp),
                "c" | "h" => Some(ProjectType::C),
                _ => None,
            };

            if let Some(pt) = project_type {
                *counts.entry(pt).or_insert(0) += 1;
            }
        }
    }

    counts.into_iter().max_by_key(|(_, count)| *count).map(|(pt, _)| pt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_rust_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();

        assert!(is_rust_project(temp.path()));
    }

    #[test]
    fn test_is_node_project() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();

        assert!(is_node_project(temp.path()));
    }

    #[test]
    fn test_has_git_repo() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        assert!(has_git_repo(temp.path()));
    }

    #[test]
    fn test_get_project_markers() {
        let markers = get_project_markers(ProjectType::Rust);
        assert!(markers.contains(&"Cargo.toml"));
    }
}
