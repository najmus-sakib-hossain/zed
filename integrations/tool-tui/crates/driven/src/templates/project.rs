//! Project structure templates

use super::{Template, TemplateCategory};
use crate::{Result, parser::UnifiedRule};

/// Project template definition
#[derive(Debug, Clone)]
pub struct ProjectTemplate {
    name: String,
    description: String,
    includes: Vec<String>,
    excludes: Vec<String>,
    focus: Vec<String>,
    tags: Vec<String>,
}

impl ProjectTemplate {
    /// Rust workspace project
    pub fn rust_workspace() -> Self {
        Self {
            name: "rust-workspace".to_string(),
            description: "Rust Cargo workspace with multiple crates".to_string(),
            includes: vec![
                "crates/**".to_string(),
                "src/**".to_string(),
                "Cargo.toml".to_string(),
                "Cargo.lock".to_string(),
            ],
            excludes: vec!["target/**".to_string(), "**/*.log".to_string()],
            focus: vec![
                "This is a Rust workspace with multiple crates".to_string(),
                "Use idiomatic Rust patterns".to_string(),
                "Follow Rust API guidelines".to_string(),
                "Run cargo fmt and cargo clippy before commits".to_string(),
                "Write documentation for public APIs".to_string(),
            ],
            tags: vec![
                "rust".to_string(),
                "cargo".to_string(),
                "workspace".to_string(),
            ],
        }
    }

    /// TypeScript monorepo
    pub fn typescript_monorepo() -> Self {
        Self {
            name: "typescript-monorepo".to_string(),
            description: "TypeScript monorepo with multiple packages".to_string(),
            includes: vec![
                "packages/**".to_string(),
                "apps/**".to_string(),
                "package.json".to_string(),
                "tsconfig.json".to_string(),
            ],
            excludes: vec![
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".next/**".to_string(),
            ],
            focus: vec![
                "This is a TypeScript monorepo".to_string(),
                "Use strict TypeScript configuration".to_string(),
                "Prefer functional patterns".to_string(),
                "Use ESLint and Prettier for formatting".to_string(),
                "Write tests with Vitest or Jest".to_string(),
            ],
            tags: vec![
                "typescript".to_string(),
                "monorepo".to_string(),
                "node".to_string(),
            ],
        }
    }

    /// Full-stack application
    pub fn fullstack() -> Self {
        Self {
            name: "fullstack".to_string(),
            description: "Full-stack web application".to_string(),
            includes: vec![
                "src/**".to_string(),
                "frontend/**".to_string(),
                "backend/**".to_string(),
                "api/**".to_string(),
            ],
            excludes: vec![
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "target/**".to_string(),
                ".next/**".to_string(),
            ],
            focus: vec![
                "This is a full-stack web application".to_string(),
                "Consider both frontend and backend impacts".to_string(),
                "Maintain API consistency".to_string(),
                "Consider security at every layer".to_string(),
                "Write integration tests for critical paths".to_string(),
            ],
            tags: vec![
                "fullstack".to_string(),
                "web".to_string(),
                "api".to_string(),
            ],
        }
    }

    /// CLI tool project
    pub fn cli_tool() -> Self {
        Self {
            name: "cli-tool".to_string(),
            description: "Command-line interface tool".to_string(),
            includes: vec!["src/**".to_string(), "Cargo.toml".to_string()],
            excludes: vec!["target/**".to_string()],
            focus: vec![
                "This is a CLI tool".to_string(),
                "Focus on user experience and helpful error messages".to_string(),
                "Support common CLI conventions (--help, --version, etc.)".to_string(),
                "Consider exit codes and scripting use cases".to_string(),
                "Write comprehensive help text".to_string(),
            ],
            tags: vec![
                "cli".to_string(),
                "tool".to_string(),
                "command-line".to_string(),
            ],
        }
    }

    /// Library crate
    pub fn library() -> Self {
        Self {
            name: "library".to_string(),
            description: "Reusable library crate".to_string(),
            includes: vec!["src/**".to_string(), "Cargo.toml".to_string()],
            excludes: vec!["target/**".to_string(), "examples/**".to_string()],
            focus: vec![
                "This is a library crate for reuse".to_string(),
                "Design clean and intuitive public APIs".to_string(),
                "Document all public items with examples".to_string(),
                "Minimize dependencies".to_string(),
                "Consider semver compatibility".to_string(),
                "Write comprehensive tests".to_string(),
            ],
            tags: vec![
                "library".to_string(),
                "crate".to_string(),
                "reusable".to_string(),
            ],
        }
    }
}

impl Template for ProjectTemplate {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> TemplateCategory {
        TemplateCategory::Project
    }

    fn expand(&self) -> Result<Vec<UnifiedRule>> {
        Ok(vec![UnifiedRule::Context {
            includes: self.includes.clone(),
            excludes: self.excludes.clone(),
            focus: self.focus.clone(),
        }])
    }

    fn tags(&self) -> Vec<&str> {
        self.tags.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_workspace() {
        let template = ProjectTemplate::rust_workspace();
        assert_eq!(template.name(), "rust-workspace");
        assert_eq!(template.category(), TemplateCategory::Project);

        let rules = template.expand().unwrap();
        assert_eq!(rules.len(), 1);

        if let UnifiedRule::Context { includes, .. } = &rules[0] {
            assert!(includes.contains(&"crates/**".to_string()));
        } else {
            panic!("Expected Context rule");
        }
    }
}
